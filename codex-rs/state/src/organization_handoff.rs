//! Checkpoint content cannot grant authority. Owner handoffs keep the same grant
//! and counters, and only resume stopped work with reconciled effects/usage.
use super::OrganizationStore;
use super::actions::supersede_approvals;
use super::actions::valid_hash;
use super::definitions::text;
use super::execution::*;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization_coordination::OrganizationCheckpointContent;
use codex_protocol::organization_execution::*;
use sqlx::Sqlite;
use sqlx::Transaction;
use std::collections::HashSet;
use uuid::Uuid;

impl OrganizationStore {
    /// The host supplies hashes from the private workspace while holding its
    /// native-tool lock; agents supply only progress/remaining/blocker text.
    pub async fn record_checkpoint(
        &self,
        attempt_id: &str,
        worker: &str,
        call_id: &str,
        content: OrganizationCheckpointContent,
        files: Vec<OrganizationArtifactEvidence>,
        now: i64,
    ) -> Result<OrganizationTaskCheckpoint> {
        text(call_id, 256)?;
        validate_content(&content)?;
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, authorization) = check_fence(&mut tx, attempt_id, worker, now).await?;
        let mut task = read_task(&mut tx, &attempt.task_id).await?;
        validate_files(&files, &attempt.artifact_bases, &authorization.scope)?;
        let reused: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM organization_coordination_calls WHERE attempt_id=? AND call_id=?",
        )
        .bind(attempt_id)
        .bind(call_id)
        .fetch_one(&mut *tx)
        .await?;
        ensure!(
            reused == 0,
            "call identity already used by another coordination operation"
        );
        let old: Option<String> = sqlx::query_scalar(
            "SELECT data FROM organization_checkpoints WHERE attempt_id=? AND call_id=?",
        )
        .bind(attempt_id)
        .bind(call_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(old) = old {
            let old: OrganizationTaskCheckpoint = serde_json::from_str(&old)?;
            ensure!(
                old.content == content && old.files == files,
                "checkpoint call identity changed"
            );
            return Ok(old);
        }
        let calls: i64 = sqlx::query_scalar("SELECT (SELECT count(*) FROM organization_coordination_calls WHERE attempt_id=?) + (SELECT count(*) FROM organization_checkpoints WHERE attempt_id=?)")
            .bind(attempt_id).bind(attempt_id).fetch_one(&mut *tx).await?;
        let checkpoints: i64 =
            sqlx::query_scalar("SELECT count(*) FROM organization_checkpoints WHERE task_id=?")
                .bind(&task.id)
                .fetch_one(&mut *tx)
                .await?;
        ensure!(
            calls < 256 && checkpoints < 32,
            "checkpoint/coordination storage limit exhausted"
        );
        let completed_action_ids = sqlx::query_scalar("SELECT id FROM organization_actions WHERE attempt_id=? AND state='completed' ORDER BY id")
            .bind(attempt_id).fetch_all(&mut *tx).await?;
        let checkpoint = OrganizationTaskCheckpoint {
            id: Uuid::now_v7().to_string(),
            task_id: task.id.clone(),
            project_id: task.project_id.clone(),
            owner_id: task.owner_id.clone(),
            attempt_id: attempt_id.to_owned(),
            authorization_id: authorization.id.clone(),
            created_at: now,
            completed_action_ids,
            content,
            files,
            artifact_bases: attempt.artifact_bases.clone(),
        };
        sqlx::query("INSERT INTO organization_checkpoints(id,task_id,attempt_id,call_id,data) VALUES (?,?,?,?,?)")
            .bind(&checkpoint.id).bind(&task.id).bind(attempt_id).bind(call_id)
            .bind(serde_json::to_string(&checkpoint)?).execute(&mut *tx).await?;
        if checkpoint.content.blocker.is_some() {
            // Retain occupancy until the host has actually stopped native work.
            attempt.state = OrganizationTaskState::Blocked;
            attempt.lease_until = 0;
            write_attempt(&mut tx, &attempt).await?;
            supersede_approvals(&mut tx, attempt_id).await?;
            set_task_state(&mut tx, &mut task, OrganizationTaskState::Blocked).await?;
        }
        event(&mut tx, "task/checkpoint", &checkpoint.id).await?;
        tx.commit().await?;
        Ok(checkpoint)
    }

    /// The owner confirms the exact checkpoint/task/receiver revisions. Never
    /// create a replacement grant or reset consumed token/request/attempt limits.
    pub async fn handoff_task(
        &self,
        mutation: &OrganizationExecutionMutation,
        now: i64,
    ) -> Result<OrganizationTaskHandoff> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let handoff = handoff_in(&mut tx, mutation, now).await?;
        tx.commit().await?;
        Ok(handoff)
    }

    /// Atomic owner-reviewed checkpoint replacements. Original grants and usage
    /// are retained; one stale/uncertain item rolls back the entire decision.
    pub async fn apply_replan(
        &self,
        task_id: &str,
        id: &str,
        expected_revision: i64,
        now: i64,
    ) -> Result<Vec<OrganizationTaskHandoff>> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let policy = super::controls::read(&mut tx, &format!("replan:{id}"))
            .await?
            .context("replan proposal missing")?;
        ensure!(
            policy.revision == expected_revision,
            "replan revision conflict"
        );
        let mut value = policy.value;
        let codex_protocol::organization::OrganizationControlValue::Replan {
            items,
            applied_handoff_ids,
            ..
        } = &mut value
        else {
            anyhow::bail!("not a replan proposal")
        };
        ensure!(
            applied_handoff_ids.is_empty()
                && items.first().is_some_and(|item| item.task_id == task_id),
            "replan already applied or response task changed"
        );
        let mut handoffs = Vec::new();
        for item in items {
            let handoff = handoff_in(&mut tx, &item.handoff(), now).await?;
            applied_handoff_ids.push(handoff.id.clone());
            handoffs.push(handoff);
        }
        super::controls::set(&mut tx, Some(expected_revision), value).await?;
        owner_event(&mut tx, "replan/apply", id).await?;
        tx.commit().await?;
        Ok(handoffs)
    }
}

pub(super) async fn handoff_in(
    tx: &mut Transaction<'_, Sqlite>,
    mutation: &OrganizationExecutionMutation,
    now: i64,
) -> Result<OrganizationTaskHandoff> {
    let OrganizationExecutionMutation::Handoff {
        task_id,
        expected_revision,
        checkpoint_id,
        owner_id,
        expected_owner_revision,
        instructions,
    } = mutation
    else {
        anyhow::bail!("expected a handoff mutation")
    };
    text(instructions, 4096)?;
    let mut task = read_task(tx, task_id).await?;
    ensure!(
        task.revision == *expected_revision && task.state == "blocked",
        "handoff requires the exact blocked task revision"
    );
    let data: String = sqlx::query_scalar(
        "SELECT data FROM organization_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1",
    )
    .bind(task_id)
    .fetch_one(&mut **tx)
    .await
    .context("checkpoint missing")?;
    let checkpoint: OrganizationTaskCheckpoint = serde_json::from_str(&data)?;
    ensure!(
        checkpoint.id == *checkpoint_id && checkpoint.owner_id == task.owner_id,
        "checkpoint or ownership changed"
    );
    let data: String = sqlx::query_scalar("SELECT a.data FROM organization_authorizations a JOIN organization_tasks t ON t.authorization_id=a.id WHERE t.id=? AND a.revoked=0 AND a.expires_at>?")
        .bind(task_id).bind(now).fetch_one(&mut **tx).await.context("grant revoked or expired")?;
    let mut authorization: OrganizationTaskAuthorization = serde_json::from_str(&data)?;
    ensure!(
        authorization.id == checkpoint.authorization_id
            && authorization.policy_version == ORGANIZATION_POLICY_VERSION,
        "checkpoint grant or host policy changed"
    );
    super::coordination::check_parent_grants(tx, &authorization, now).await?;
    super::schedule::check_schedule_fence(tx, &authorization, now, true).await?;
    let occupied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM organization_attempts WHERE task_id=? AND occupied=1",
    )
    .bind(task_id)
    .fetch_one(&mut **tx)
    .await?;
    ensure!(occupied == 0, "previous task runtime has not stopped");
    let unresolved: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_attempts p WHERE p.task_id=? AND (json_extract(p.data,'$.modelRequests')<>json_extract(p.data,'$.reportedModelRequests') OR EXISTS(SELECT 1 FROM organization_actions a WHERE a.attempt_id=p.id AND a.state IN ('pending','dispatched','uncertain')))")
        .bind(task_id).fetch_one(&mut **tx).await?;
    ensure!(
        unresolved == 0,
        "effects or provider usage require reconciliation before handoff"
    );
    let descendants: i64 = sqlx::query_scalar("WITH RECURSIVE descendants(id) AS (SELECT a.id FROM organization_authorizations a JOIN organization_attempts p ON p.id=a.parent_attempt_id WHERE p.task_id=? UNION SELECT a.id FROM organization_authorizations a JOIN organization_attempts p ON p.id=a.parent_attempt_id JOIN descendants d ON p.authorization_id=d.id) SELECT count(*) FROM descendants d JOIN organization_authorizations a ON a.id=d.id JOIN organization_tasks t ON t.id=a.task_id WHERE t.state NOT IN ('accepted','cancelled','failed') OR EXISTS(SELECT 1 FROM organization_attempts p WHERE p.task_id=t.id AND p.occupied=1)")
        .bind(task_id).fetch_one(&mut **tx).await?;
    ensure!(
        descendants == 0,
        "finish or cancel outstanding descendants before handing off this task"
    );
    let (tokens, requests): (i64, i64) = sqlx::query_as("SELECT r.used_tokens,r.model_requests FROM organization_authorizations a JOIN organization_authorizations r ON r.id=a.root_id WHERE a.id=?")
        .bind(&authorization.id).fetch_one(&mut **tx).await?;
    let attempts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM organization_attempts WHERE authorization_id=?")
            .bind(&authorization.id)
            .fetch_one(&mut **tx)
            .await?;
    ensure!(
        tokens < authorization.scope.max_tokens
            && requests < i64::from(authorization.scope.max_model_requests)
            && attempts < i64::from(authorization.scope.max_attempts),
        "remaining handoff budget exhausted"
    );
    let original_owner: String = sqlx::query_scalar("SELECT json_extract(data,'$.ownerId') FROM organization_checkpoints WHERE task_id=? AND json_extract(data,'$.authorizationId')=? ORDER BY id LIMIT 1")
        .bind(task_id).bind(&authorization.id).fetch_one(&mut **tx).await?;
    ensure!(
        owner_id == &original_owner
            || owner_id == &task.owner_id
            || authorization.scope.delegates.contains(owner_id),
        "receiver is outside the originally permitted roster"
    );
    let from_owner_id = task.owner_id.clone();
    task.owner_id = owner_id.clone();
    let (receiver, project) = scope_definitions(tx, &task, &authorization.scope).await?;
    ensure!(
        receiver.revision == *expected_owner_revision
            && project.revision == authorization.project_revision,
        "receiver or project revision changed"
    );
    let handoff = OrganizationTaskHandoff {
        id: Uuid::now_v7().to_string(),
        task_id: task_id.clone(),
        checkpoint_id: checkpoint_id.clone(),
        authorization_id: authorization.id.clone(),
        from_owner_id,
        to_owner_id: owner_id.clone(),
        from_node_revision: authorization.node_revision,
        to_node_revision: receiver.revision,
        instructions: instructions.clone(),
        created_at: now,
    };
    authorization.node_revision = receiver.revision;
    sqlx::query("UPDATE organization_authorizations SET data=? WHERE id=?")
        .bind(serde_json::to_string(&authorization)?)
        .bind(&authorization.id)
        .execute(&mut **tx)
        .await?;
    sqlx::query(
        "INSERT INTO organization_handoffs(id,task_id,checkpoint_id,data) VALUES (?,?,?,?)",
    )
    .bind(&handoff.id)
    .bind(task_id)
    .bind(checkpoint_id)
    .bind(serde_json::to_string(&handoff)?)
    .execute(&mut **tx)
    .await?;
    set_task_state(tx, &mut task, OrganizationTaskState::Queued).await?;
    owner_event(tx, "task/handoff", &handoff.id).await?;
    Ok(handoff)
}

pub(super) async fn continuity_view(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
) -> Result<(
    Vec<OrganizationTaskCheckpoint>,
    Vec<OrganizationTaskHandoff>,
)> {
    let checkpoints = sqlx::query_scalar::<_, String>(
        "SELECT data FROM organization_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 16",
    )
    .bind(task_id)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|data| serde_json::from_str(&data))
    .collect::<std::result::Result<_, _>>()?;
    let handoffs = sqlx::query_scalar::<_, String>(
        "SELECT data FROM organization_handoffs WHERE task_id=? ORDER BY id DESC LIMIT 16",
    )
    .bind(task_id)
    .fetch_all(&mut **tx)
    .await?
    .into_iter()
    .map(|data| serde_json::from_str(&data))
    .collect::<std::result::Result<_, _>>()?;
    Ok((checkpoints, handoffs))
}

fn validate_content(content: &OrganizationCheckpointContent) -> Result<()> {
    ensure!(
        content.completed_steps.len() <= 32
            && content.remaining_steps.len() <= 32
            && serde_json::to_vec(content)?.len() <= 16 * 1024,
        "checkpoint content exceeds its bound"
    );
    for step in content
        .completed_steps
        .iter()
        .chain(&content.remaining_steps)
    {
        text(step, 4096)?;
    }
    if let Some(blocker) = &content.blocker {
        text(blocker, 4096)?;
    }
    Ok(())
}

fn validate_files(
    files: &[OrganizationArtifactEvidence],
    bases: &[OrganizationArtifactBase],
    scope: &OrganizationExecutionScope,
) -> Result<()> {
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for file in files {
        ensure!(
            scope.input_files.contains(&file.path) || scope.artifacts.contains(&file.path),
            "checkpoint path outside grant"
        );
        ensure!(
            seen.insert(&file.path) && valid_hash(&file.sha256),
            "duplicate or invalid checkpoint file evidence"
        );
        total = total
            .checked_add(file.bytes)
            .context("checkpoint byte count overflow")?;
        ensure!(
            total <= 64 * 1024 * 1024,
            "checkpoint snapshot exceeds 64 MiB"
        );
    }
    ensure!(
        scope.input_files.iter().all(|path| seen.contains(path)),
        "checkpoint input snapshot incomplete"
    );
    ensure!(
        bases
            .iter()
            .map(|base| &base.path)
            .eq(scope.artifacts.iter())
            && bases
                .iter()
                .all(|base| base.sha256.as_ref().is_none_or(|hash| valid_hash(hash))),
        "checkpoint original bases differ from grant"
    );
    Ok(())
}

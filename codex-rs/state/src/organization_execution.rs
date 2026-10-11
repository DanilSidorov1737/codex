use super::OrganizationStore;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::*;
use codex_protocol::organization_execution::*;
use sqlx::Sqlite;
use sqlx::Transaction;
use std::collections::HashSet;
use std::path::Component;
use std::path::Path;
use uuid::Uuid;

impl OrganizationStore {
    /// Only the authenticated owner service may authorize or replace scope.
    pub async fn authorize_task(
        &self,
        task_id: &str,
        expected_revision: i64,
        scope: OrganizationExecutionScope,
        now: i64,
    ) -> Result<OrganizationTaskAuthorization> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        super::schedule::reject_occurrence_reauthorization(&mut tx, task_id).await?;
        super::workflow::reject_workflow_reauthorization(&mut tx, task_id).await?;
        let authorization =
            authorize_task_in(&mut tx, task_id, expected_revision, scope, now).await?;
        tx.commit().await?;
        Ok(authorization)
    }

    /// Claim and reserve active capacity in the same short business transaction.
    /// The caller reserves a runtime thread ID before starting any runtime work.
    pub async fn claim_task(
        &self,
        worker_id: &str,
        thread_id: &str,
        now: i64,
        max_active: u32,
    ) -> Result<Option<OrganizationTaskAttempt>> {
        Uuid::parse_str(worker_id)?;
        Uuid::parse_str(thread_id)?;
        ensure!(
            (1..=4).contains(&max_active),
            "active execution limit must be 1–4"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let active: i64 =
            sqlx::query_scalar("SELECT count(*) FROM organization_attempts WHERE occupied=1")
                .fetch_one(&mut *tx)
                .await?;
        if active >= i64::from(max_active) {
            return Ok(None);
        }
        // Bound resident root runtimes separately from active inference. Every
        // admitted root reserves room for its at-most-32 descendants; waiting
        // ancestors therefore cannot fill the runtime tracker and starve a child.
        let roots: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id WHERE a.parent_attempt_id IS NULL AND (p.occupied=1 OR p.state IN ('running','waitingApproval','waitingDependency'))")
            .fetch_one(&mut *tx).await?;
        let candidates: Vec<(String, String)> = sqlx::query_as("SELECT t.id,a.data FROM organization_tasks t JOIN organization_authorizations a ON a.id=t.authorization_id WHERE t.state='queued' AND a.revoked=0 AND a.expires_at>? AND COALESCE(json_extract(a.data,'$.scope.notBefore'),0)<=? AND (a.parent_attempt_id IS NOT NULL OR ?<4) AND NOT EXISTS(SELECT 1 FROM organization_task_dependencies d JOIN organization_tasks p ON p.id=d.prerequisite_id WHERE d.task_id=t.id AND p.state<>'accepted') ORDER BY a.parent_attempt_id IS NULL,t.id LIMIT 32")
            .bind(now).bind(now).bind(roots).fetch_all(&mut *tx).await?;
        for (task_id, authorization_data) in candidates {
            let authorization: OrganizationTaskAuthorization =
                serde_json::from_str(&authorization_data)?;
            let mut task = read_task(&mut tx, &task_id).await?;
            let node = read_node(&mut tx, &task.owner_id).await?;
            let project = read_project(&mut tx, &task.project_id).await?;
            if authorization.policy_version != ORGANIZATION_POLICY_VERSION
                || node.revision != authorization.node_revision
                || project.revision != authorization.project_revision
                || !ancestors_enabled(&mut tx, &node.id).await?
                || super::schedule::check_schedule_fence(&mut tx, &authorization, now, true)
                    .await
                    .is_err()
                || super::coordination::check_parent_grants(&mut tx, &authorization, now)
                    .await
                    .is_err()
            {
                set_task_state(&mut tx, &mut task, OrganizationTaskState::Paused).await?;
                event(&mut tx, "task/scopeChanged", &task_id).await?;
                continue;
            }
            if super::workflow::check_workflow_dispatch(&mut tx, &authorization)
                .await
                .is_err()
            {
                continue;
            }
            let attempts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM organization_attempts WHERE authorization_id = ?",
            )
            .bind(&authorization.id)
            .fetch_one(&mut *tx)
            .await?;
            if attempts >= i64::from(authorization.scope.max_attempts) {
                set_task_state(&mut tx, &mut task, OrganizationTaskState::Failed).await?;
                event(&mut tx, "task/retryExhausted", &task_id).await?;
                continue;
            }
            let previous: i64 = sqlx::query_scalar(
                "SELECT COALESCE(max(generation),0) FROM organization_attempts WHERE task_id = ?",
            )
            .bind(&task_id)
            .fetch_one(&mut *tx)
            .await?;
            let attempt = OrganizationTaskAttempt {
                id: Uuid::now_v7().to_string(),
                task_id: task_id.clone(),
                authorization_id: authorization.id.clone(),
                owner_id: Some(task.owner_id.clone()),
                provider: None,
                role_version_id: node.role_version_id.clone(),
                generation: previous
                    .checked_add(1)
                    .context("attempt generation exhausted")?,
                thread_id: thread_id.to_owned(),
                turn_id: None,
                workspace: None,
                artifact_bases: Vec::new(),
                runtime_active: true,
                state: OrganizationTaskState::Running,
                started_at: now,
                lease_until: now
                    .checked_add(30)
                    .context("lease time overflow")?
                    .min(authorization.expires_at),
                reported_tokens: 0,
                model_requests: 0,
                reported_model_requests: 0,
            };
            sqlx::query("INSERT INTO organization_attempts(id,task_id,authorization_id,generation,thread_id,worker_id,lease_until,state,data) VALUES (?,?,?,?,?,?,?,'running',?)")
                .bind(&attempt.id).bind(&task_id).bind(&authorization.id).bind(attempt.generation).bind(thread_id).bind(worker_id).bind(attempt.lease_until).bind(serde_json::to_string(&attempt)?).execute(&mut *tx).await?;
            set_task_state(&mut tx, &mut task, OrganizationTaskState::Running).await?;
            event(&mut tx, "attempt/claimed", &attempt.id).await?;
            tx.commit().await?;
            return Ok(Some(attempt));
        }
        tx.commit().await?;
        Ok(None)
    }

    /// Admission checks also fence stale workers after pause, revoke or restart.
    pub async fn admit_model_request(
        &self,
        attempt_id: &str,
        worker_id: &str,
        now: i64,
    ) -> Result<(u32, i64)> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, authorization) = check_fence(&mut tx, attempt_id, worker_id, now).await?;
        super::workflow::check_workflow_dispatch(&mut tx, &authorization).await?;
        ensure!(
            attempt.runtime_active,
            "inference requires reserved active capacity"
        );
        super::schedule::check_schedule_fence(&mut tx, &authorization, now, true).await?;
        ensure!(
            attempt.model_requests == attempt.reported_model_requests,
            "prior model usage is unresolved; reconcile before further inference"
        );
        let (used, requests): (i64, i64) = sqlx::query_as(
            "SELECT used_tokens,model_requests FROM organization_authorizations WHERE id = ?",
        )
        .bind(
            authorization
                .root_authorization_id
                .as_ref()
                .unwrap_or(&authorization.id),
        )
        .fetch_one(&mut *tx)
        .await?;
        ensure!(
            used < authorization.scope.max_tokens,
            "reported token ceiling exhausted"
        );
        ensure!(
            requests < i64::from(authorization.scope.max_model_requests),
            "model request ceiling exhausted"
        );
        attempt.model_requests += 1;
        sqlx::query(
            "UPDATE organization_authorizations SET model_requests=model_requests+1 WHERE id = ?",
        )
        .bind(&authorization.id)
        .execute(&mut *tx)
        .await?;
        if let Some(root) = &authorization.root_authorization_id {
            sqlx::query(
                "UPDATE organization_authorizations SET model_requests=model_requests+1 WHERE id=?",
            )
            .bind(root)
            .execute(&mut *tx)
            .await?;
        }
        write_attempt(&mut tx, &attempt).await?;
        event(&mut tx, "model/admitted", attempt_id).await?;
        tx.commit().await?;
        // Capture this request's baseline in the admission transaction, so a
        // late prior usage checkpoint cannot race an independent read.
        Ok((attempt.model_requests, attempt.reported_tokens))
    }

    pub async fn checkpoint_usage(
        &self,
        attempt_id: &str,
        worker_id: &str,
        request_sequence: u32,
        reported_tokens: i64,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        // A late provider report remains a billable observation after pause/expiry.
        // It can update accounting, never restore execution authority.
        let data: String =
            sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=? AND worker_id=?")
                .bind(attempt_id)
                .bind(worker_id)
                .fetch_one(&mut *tx)
                .await?;
        let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
        ensure!(
            request_sequence > 0
                && request_sequence <= attempt.model_requests
                && request_sequence >= attempt.reported_model_requests
                && request_sequence <= attempt.reported_model_requests + 1,
            "invalid model usage sequence"
        );
        ensure!(
            reported_tokens >= attempt.reported_tokens,
            "token checkpoint cannot move backwards"
        );
        let delta = reported_tokens - attempt.reported_tokens;
        if request_sequence == attempt.reported_model_requests {
            ensure!(delta == 0, "completed usage receipt cannot change");
            return Ok(());
        }
        let used: i64 =
            sqlx::query_scalar("SELECT used_tokens FROM organization_authorizations WHERE id = ?")
                .bind(&attempt.authorization_id)
                .fetch_one(&mut *tx)
                .await?;
        let used = used.checked_add(delta).context("token counter overflow")?;
        sqlx::query("UPDATE organization_authorizations SET used_tokens = ? WHERE id = ?")
            .bind(used)
            .bind(&attempt.authorization_id)
            .execute(&mut *tx)
            .await?;
        let root: String =
            sqlx::query_scalar("SELECT root_id FROM organization_authorizations WHERE id=?")
                .bind(&attempt.authorization_id)
                .fetch_one(&mut *tx)
                .await?;
        if root != attempt.authorization_id {
            sqlx::query(
                "UPDATE organization_authorizations SET used_tokens=used_tokens+? WHERE id=?",
            )
            .bind(delta)
            .bind(root)
            .execute(&mut *tx)
            .await?;
        }
        attempt.reported_tokens = reported_tokens;
        attempt.reported_model_requests = request_sequence;
        if matches!(
            attempt.state,
            OrganizationTaskState::WaitingApproval | OrganizationTaskState::WaitingDependency
        ) {
            attempt.runtime_active = false;
        }
        write_attempt(&mut tx, &attempt).await?;
        event(&mut tx, "usage/checkpoint", attempt_id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Startup reconciliation never dispatches an uncertain action or accepts an approval.
    pub async fn reconcile_interrupted_attempts(&self) -> Result<u64> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let applications: Vec<String> =
            sqlx::query_scalar("SELECT data FROM organization_applications WHERE state='pending'")
                .fetch_all(&mut *tx)
                .await?;
        for data in applications {
            let mut application: OrganizationApplication = serde_json::from_str(&data)?;
            application.state = OrganizationApplicationState::Uncertain;
            application.receipt=Some("daemon interrupted; inspect original files before manual reconciliation; no automatic replay".into());
            sqlx::query("UPDATE organization_applications SET state='uncertain',data=? WHERE id=?")
                .bind(serde_json::to_string(&application)?)
                .bind(&application.id)
                .execute(&mut *tx)
                .await?;
            event(&mut tx, "application/reconcile", &application.id).await?;
        }
        let data: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE occupied=1 OR state IN ('running','waitingApproval','waitingDependency')").fetch_all(&mut *tx).await?;
        let mut count = 0;
        for data in data {
            let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
            let interrupted = matches!(
                attempt.state,
                OrganizationTaskState::Running
                    | OrganizationTaskState::WaitingApproval
                    | OrganizationTaskState::WaitingDependency
            );
            if interrupted {
                attempt.state = OrganizationTaskState::NeedsReconciliation;
                count += 1;
            }
            attempt.runtime_active = false;
            attempt.lease_until = 0;
            write_attempt(&mut tx, &attempt).await?;
            let mut task = read_task(&mut tx, &attempt.task_id).await?;
            if interrupted {
                set_task_state(
                    &mut tx,
                    &mut task,
                    OrganizationTaskState::NeedsReconciliation,
                )
                .await?;
            }
            let actions: Vec<String> = sqlx::query_scalar(
                "SELECT data FROM organization_actions WHERE attempt_id=? AND state='dispatched'",
            )
            .bind(&attempt.id)
            .fetch_all(&mut *tx)
            .await?;
            for data in actions {
                let mut action: OrganizationActionIntent = serde_json::from_str(&data)?;
                action.state = OrganizationActionState::Uncertain;
                super::actions::write_action(&mut tx, &action).await?;
            }
            event(&mut tx, "attempt/reconcile", &attempt.id).await?;
        }
        tx.commit().await?;
        Ok(count)
    }
}

pub(super) fn validate_scope(scope: &OrganizationExecutionScope) -> Result<()> {
    ensure!(
        !scope.auto_accept_children || !scope.checks.is_empty(),
        "automatic internal acceptance requires owner-declared checks"
    );
    ensure!(
        (1..=10_000_000).contains(&scope.max_tokens)
            && (1..=3600).contains(&scope.max_seconds)
            && (1..=100).contains(&scope.max_model_requests)
            && (1..=3).contains(&scope.max_attempts),
        "invalid execution budget"
    );
    ensure!(
        !scope.model.trim().is_empty() && scope.model.len() <= 256 && !scope.model.contains('\0'),
        "invalid model selection"
    );
    ensure!(
        scope.input_files.len() <= 1000
            && scope.immutable_files.len() <= 1000
            && (1..=64).contains(&scope.artifacts.len())
            && scope.checks.len() <= 8
            && scope.delegates.len() <= 16
            && scope.max_children <= 32,
        "execution scope too large"
    );
    for paths in [&scope.input_files, &scope.immutable_files, &scope.artifacts] {
        let mut unique = HashSet::new();
        for path in paths {
            ensure!(
                !path.is_empty()
                    && path.len() <= 4096
                    && !path.contains('\0')
                    && Path::new(path)
                        .components()
                        .all(|part| matches!(part, Component::Normal(_))),
                "scope paths must be relative without traversal"
            );
            ensure!(unique.insert(path), "duplicate scope path");
        }
    }
    ensure!(
        scope
            .immutable_files
            .iter()
            .all(|path| scope.input_files.contains(path)),
        "immutable files must be declared inputs"
    );
    for argv in &scope.checks {
        ensure!(
            (1..=32).contains(&argv.len())
                && argv
                    .iter()
                    .all(|arg| arg.len() <= 8192 && !arg.contains('\0'))
                && !argv[0].is_empty(),
            "invalid check argv"
        );
    }
    ensure!(
        serde_json::to_vec(scope)?.len() <= 128 * 1024,
        "execution scope exceeds 128 KiB"
    );
    Ok(())
}

pub(super) async fn read_task(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<OrganizationTask> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_tasks WHERE id = ?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    let OrganizationRecord::Task(task) = serde_json::from_str(&data)? else {
        anyhow::bail!("invalid task record");
    };
    Ok(task)
}

pub(super) async fn read_node(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<OrganizationNode> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_nodes WHERE id = ?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    let OrganizationRecord::Node(node) = serde_json::from_str(&data)? else {
        anyhow::bail!("invalid node record");
    };
    Ok(node)
}

pub(super) async fn read_project(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<OrganizationProjectPlan> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_projects WHERE id = ?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    let OrganizationRecord::Project(project) = serde_json::from_str(&data)? else {
        anyhow::bail!("invalid project record");
    };
    Ok(project)
}

pub(super) async fn ancestors_enabled(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i64>("WITH RECURSIVE ancestors(id,parent_id,data) AS (SELECT id,parent_id,data FROM organization_nodes WHERE id=? UNION SELECT n.id,n.parent_id,n.data FROM organization_nodes n JOIN ancestors a ON n.id=a.parent_id) SELECT count(*)=0 FROM ancestors WHERE json_extract(data,'$.data.enabled')<>1").bind(id).fetch_one(&mut **tx).await? != 0)
}

pub(super) async fn set_task_state(
    tx: &mut Transaction<'_, Sqlite>,
    task: &mut OrganizationTask,
    state: OrganizationTaskState,
) -> Result<()> {
    task.state = state.as_str().to_owned();
    task.revision = task
        .revision
        .checked_add(1)
        .context("task revision exhausted")?;
    sqlx::query("UPDATE organization_tasks SET state=?,revision=?,data=? WHERE id=?")
        .bind(&task.state)
        .bind(task.revision)
        .bind(serde_json::to_string(&OrganizationRecord::Task(
            task.clone(),
        ))?)
        .bind(&task.id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) async fn write_attempt(
    tx: &mut Transaction<'_, Sqlite>,
    attempt: &OrganizationTaskAttempt,
) -> Result<()> {
    sqlx::query(
        "UPDATE organization_attempts SET state=?,lease_until=?,occupied=?,data=? WHERE id=?",
    )
    .bind(attempt.state.as_str())
    .bind(attempt.lease_until)
    .bind(attempt.runtime_active)
    .bind(serde_json::to_string(attempt)?)
    .bind(&attempt.id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub(super) async fn check_fence(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    worker: &str,
    now: i64,
) -> Result<(OrganizationTaskAttempt, OrganizationTaskAuthorization)> {
    let result = check_live_fence(tx, id, worker, now).await?;
    ensure!(
        result.0.state == OrganizationTaskState::Running,
        "attempt is waiting"
    );
    Ok(result)
}

pub(super) async fn check_live_fence(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
    worker: &str,
    now: i64,
) -> Result<(OrganizationTaskAttempt, OrganizationTaskAuthorization)> {
    let (data, authorization_data): (String, String) = sqlx::query_as("SELECT p.data,a.data FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id JOIN organization_tasks t ON t.id=p.task_id WHERE p.id=? AND p.worker_id=? AND p.lease_until>? AND p.state IN ('running','waitingApproval','waitingDependency') AND a.revoked=0 AND a.expires_at>? AND t.authorization_id=a.id AND t.state=p.state").bind(id).bind(worker).bind(now).bind(now).fetch_one(&mut **tx).await.context("stale, paused, revoked or expired attempt")?;
    let attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
    let authorization: OrganizationTaskAuthorization = serde_json::from_str(&authorization_data)?;
    ensure!(
        authorization.policy_version == ORGANIZATION_POLICY_VERSION,
        "host policy changed; owner reauthorization required"
    );
    super::coordination::check_parent_grants(tx, &authorization, now).await?;
    super::schedule::check_schedule_fence(tx, &authorization, now, false).await?;
    ensure!(
        now >= attempt.started_at
            && now
                .checked_sub(attempt.started_at)
                .context("elapsed time overflow")?
                < i64::from(authorization.scope.max_seconds),
        "elapsed task budget exhausted or clock moved backwards"
    );
    let task = read_task(tx, &attempt.task_id).await?;
    let node = read_node(tx, &task.owner_id).await?;
    let project = read_project(tx, &task.project_id).await?;
    ensure!(
        node.revision == authorization.node_revision
            && project.revision == authorization.project_revision
            && ancestors_enabled(tx, &node.id).await?,
        "task scope changed or subtree paused"
    );
    Ok((attempt, authorization))
}

pub(super) async fn event(
    tx: &mut Transaction<'_, Sqlite>,
    kind: &str,
    reference: &str,
) -> Result<()> {
    event_as(tx, "host", kind, reference).await
}

pub(super) async fn owner_event(
    tx: &mut Transaction<'_, Sqlite>,
    kind: &str,
    reference: &str,
) -> Result<()> {
    event_as(tx, "owner", kind, reference).await
}

async fn event_as(
    tx: &mut Transaction<'_, Sqlite>,
    actor: &str,
    kind: &str,
    reference: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO organization_outbox(kind,reference) VALUES (?,?)")
        .bind(kind)
        .bind(reference)
        .execute(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO organization_audit(actor,operation,payload) VALUES (?,?,?)")
        .bind(actor)
        .bind(kind)
        .bind(serde_json::to_string(&serde_json::json!({"id":reference}))?)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

// Shared by owner task authorization and atomic schedule materialization.
pub(super) async fn authorize_task_in(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
    expected_revision: i64,
    scope: OrganizationExecutionScope,
    now: i64,
) -> Result<OrganizationTaskAuthorization> {
    validate_scope(&scope)?;
    ensure!(now >= 0, "invalid authorization time");
    ensure!(
        scope
            .not_before
            .is_none_or(|time| time >= 0 && time <= now.saturating_add(31 * 86400)),
        "one-shot schedule must be within 31 days"
    );
    let mut task = read_task(tx, task_id).await?;
    ensure!(task.revision == expected_revision, "task revision conflict");
    let occupied: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM organization_attempts WHERE task_id=? AND occupied=1",
    )
    .bind(task_id)
    .fetch_one(&mut **tx)
    .await?;
    ensure!(
        occupied == 0,
        "previous task runtime has not stopped; wait for cleanup or reconcile after restart"
    );
    ensure!(
        matches!(
            task.state.as_str(),
            "proposed" | "paused" | "failed" | "needsReconciliation"
        ),
        "task cannot be authorized in its current state"
    );
    let (node, project) = scope_definitions(tx, &task, &scope).await?;
    let old: Option<String> =
        sqlx::query_scalar("SELECT authorization_id FROM organization_tasks WHERE id = ?")
            .bind(task_id)
            .fetch_one(&mut **tx)
            .await?;
    if let Some(id) = old {
        let data: String =
            sqlx::query_scalar("SELECT data FROM organization_authorizations WHERE id=?")
                .bind(&id)
                .fetch_one(&mut **tx)
                .await?;
        let mut previous: OrganizationTaskAuthorization = serde_json::from_str(&data)?;
        previous.revoked = true;
        sqlx::query("UPDATE organization_authorizations SET revoked=1,data=? WHERE id=?")
            .bind(serde_json::to_string(&previous)?)
            .bind(id)
            .execute(&mut **tx)
            .await?;
    }
    let authorization = OrganizationTaskAuthorization {
        id: Uuid::now_v7().to_string(),
        task_id: task.id.clone(),
        node_revision: node.revision,
        project_revision: project.revision,
        expires_at: scope
            .not_before
            .unwrap_or(now)
            .max(now)
            .checked_add(i64::from(scope.max_seconds) * i64::from(scope.max_attempts) + 3600)
            .context("authorization expiry overflow")?,
        scope,
        revoked: false,
        policy_version: ORGANIZATION_POLICY_VERSION,
        parent_attempt_id: None,
        root_authorization_id: None,
    };
    sqlx::query(
        "INSERT INTO organization_authorizations(id,task_id,expires_at,data) VALUES (?,?,?,?)",
    )
    .bind(&authorization.id)
    .bind(task_id)
    .bind(authorization.expires_at)
    .bind(serde_json::to_string(&authorization)?)
    .execute(&mut **tx)
    .await?;
    sqlx::query("UPDATE organization_authorizations SET root_id=id WHERE id=?")
        .bind(&authorization.id)
        .execute(&mut **tx)
        .await?;
    sqlx::query("UPDATE organization_tasks SET authorization_id = ? WHERE id = ?")
        .bind(&authorization.id)
        .bind(task_id)
        .execute(&mut **tx)
        .await?;
    set_task_state(tx, &mut task, OrganizationTaskState::Queued).await?;
    owner_event(tx, "task/authorize", &authorization.id).await?;
    Ok(authorization)
}

pub(super) async fn scope_definitions(
    tx: &mut Transaction<'_, Sqlite>,
    task: &OrganizationTask,
    scope: &OrganizationExecutionScope,
) -> Result<(OrganizationNode, OrganizationProjectPlan)> {
    let node = read_node(tx, &task.owner_id).await?;
    ensure!(
        node.kind == OrganizationNodeKind::Agent && node.enabled,
        "task owner is disabled or not an agent"
    );
    ensure!(
        ancestors_enabled(tx, &node.id).await?,
        "task owner subtree is paused"
    );
    ensure!(
        node.model
            .as_ref()
            .is_none_or(|model| model == &scope.model),
        "authorization model differs from the agent selection"
    );
    let project = read_project(tx, &task.project_id).await?;
    ensure!(
        project.roots.len() == 1 && Path::new(&project.roots[0]).is_absolute(),
        "execution requires one linked local project root"
    );
    for delegate in &scope.delegates {
        let delegate = read_node(tx, delegate).await?;
        ensure!(
            delegate.kind == OrganizationNodeKind::Agent,
            "delegate must be a configured agent"
        );
    }
    Ok((node, project))
}

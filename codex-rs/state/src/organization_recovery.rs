//! Deliberately narrow automatic recovery: no runtime was bound and no effect
//! was admitted anywhere under the original root grant. Never infer safety from
//! an absent provider response or a generic failure after startup.
use super::execution::*;
use anyhow::Context;
use anyhow::Result;
use codex_protocol::organization::OrganizationControlValue;
use codex_protocol::organization_execution::*;
use sqlx::Sqlite;
use sqlx::Transaction;

pub(super) async fn requeue_known_startup_failures(
    tx: &mut Transaction<'_, Sqlite>,
    now: i64,
) -> Result<()> {
    let rows:Vec<(String,String,String)>=sqlx::query_as("SELECT c.data,a.data,p.data FROM organization_controls c JOIN organization_tasks t ON c.id='recovery:'||t.id JOIN organization_authorizations a ON a.id=t.authorization_id JOIN organization_attempts p ON p.task_id=t.id AND p.authorization_id=a.id WHERE t.state='failed' AND p.occupied=0 AND a.revoked=0 AND a.expires_at>? AND p.generation=(SELECT max(generation) FROM organization_attempts WHERE task_id=t.id) ORDER BY t.id LIMIT 32").bind(now).fetch_all(&mut **tx).await?;
    for (policy, data, attempt_data) in rows {
        let policy: codex_protocol::organization::OrganizationRecord =
            serde_json::from_str(&policy)?;
        let codex_protocol::organization::OrganizationRecord::Control(policy) = policy else {
            anyhow::bail!("invalid recovery control")
        };
        let OrganizationControlValue::SafeRecovery {
            task_id,
            authorization_id,
            max_retries,
            enabled: true,
        } = policy.value
        else {
            continue;
        };
        let grant: OrganizationTaskAuthorization = serde_json::from_str(&data)?;
        let attempt: OrganizationTaskAttempt = serde_json::from_str(&attempt_data)?;
        if grant.id != authorization_id
            || grant.parent_attempt_id.is_some()
            || grant.policy_version != ORGANIZATION_POLICY_VERSION
            || attempt.stop_reason.as_deref() != Some("startup-before-runtime")
            || attempt.workspace.is_some()
            || now < attempt.started_at.saturating_add(10)
        {
            continue;
        }
        let (attempts,requests,effects,occupied,children):(i64,i64,i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM organization_attempts WHERE authorization_id=?),(SELECT model_requests FROM organization_authorizations WHERE id=?),(SELECT count(*) FROM organization_attempts p JOIN (SELECT attempt_id FROM organization_actions UNION ALL SELECT attempt_id FROM organization_coordination_calls UNION ALL SELECT attempt_id FROM organization_checkpoints) x ON p.id=x.attempt_id WHERE p.authorization_id=?),(SELECT count(*) FROM organization_attempts WHERE authorization_id=? AND occupied=1),(SELECT count(*) FROM organization_authorizations WHERE root_id=? AND id<>?)").bind(&grant.id).bind(&grant.id).bind(&grant.id).bind(&grant.id).bind(&grant.id).bind(&grant.id).fetch_one(&mut **tx).await?;
        // ponytail: only pre-runtime root startup failures. Later failures require reviewed checkpoints, not heuristic replay.
        if attempts > i64::from(max_retries)
            || attempts >= i64::from(grant.scope.max_attempts)
            || requests != 0
            || effects != 0
            || occupied != 0
            || children != 0
        {
            continue;
        }
        let mut task = read_task(tx, &task_id).await?;
        let node = read_node(tx, &task.owner_id).await?;
        let project = read_project(tx, &task.project_id).await?;
        if node.revision != grant.node_revision
            || project.revision != grant.project_revision
            || !ancestors_enabled(tx, &node.id).await?
            || super::controls::check_dispatch(tx, &grant).await.is_err()
            || super::workflow::check_workflow_dispatch(tx, &grant)
                .await
                .is_err()
            || super::schedule::check_schedule_fence(tx, &grant, now, true)
                .await
                .is_err()
            || super::resource::check_shared_ceilings(tx, &grant, true)
                .await
                .is_err()
        {
            continue;
        }
        let first:i64=sqlx::query_scalar("SELECT min(json_extract(data,'$.startedAt')) FROM organization_attempts WHERE authorization_id=?").bind(&grant.id).fetch_one(&mut **tx).await?;
        if now.checked_sub(first).context("recovery clock overflow")?
            >= i64::from(grant.scope.max_seconds)
        {
            continue;
        }
        set_task_state(tx, &mut task, OrganizationTaskState::Queued).await?;
        event(tx, "task/safeStartupRetry", &attempt.id).await?;
    }
    Ok(())
}

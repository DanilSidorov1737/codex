//! Read-only SQL projections. Never sum a root ledger once per descendant.
use super::OrganizationStore;
use super::execution::check_fence;
use super::execution::event;
use super::execution::read_project;
use super::execution::write_attempt;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization_execution::OrganizationProviderEvidence;
use codex_protocol::organization_report::*;
use sqlx::Row;

impl OrganizationStore {
    pub async fn record_attempt_provider(
        &self,
        attempt_id: &str,
        worker: &str,
        provider: OrganizationProviderEvidence,
        now: i64,
    ) -> Result<()> {
        ensure!(
            !provider.id.is_empty()
                && provider.id.len() <= 256
                && provider.endpoint_sha256.len() == 64
                && provider
                    .endpoint_sha256
                    .bytes()
                    .all(|byte| byte.is_ascii_hexdigit()),
            "invalid provider evidence"
        );
        if let Some(origin) = &provider.origin {
            let authority = origin
                .strip_prefix("https://")
                .or_else(|| origin.strip_prefix("http://"));
            ensure!(
                origin.len() <= 1024
                    && authority.is_some_and(|authority| !authority.is_empty()
                        && authority
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b".:-[]".contains(&byte))),
                "provider evidence must contain only a safe origin"
            );
        }
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, _) = check_fence(&mut tx, attempt_id, worker, now).await?;
        ensure!(
            attempt
                .provider
                .as_ref()
                .is_none_or(|captured| captured == &provider),
            "provider binding cannot change"
        );
        if attempt.provider.is_none() {
            attempt.provider = Some(provider);
            write_attempt(&mut tx, &attempt).await?;
            event(&mut tx, "attempt/providerBound", attempt_id).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn provider_observations(
        &self,
        evidence: &OrganizationProviderEvidence,
    ) -> Result<codex_protocol::organization_inventory::OrganizationProviderObservation> {
        use codex_protocol::organization_inventory::OrganizationProviderObservation;
        let mut tx = self.pool.begin().await?;
        let row=sqlx::query(r#"SELECT COALESCE(sum(json_extract(p.data,'$.modelRequests')),0) requests,
        COALESCE(sum(json_extract(p.data,'$.reportedModelRequests')),0) reported,
        COALESCE(sum(r.digest IS NOT NULL AND json_array_length(r.data,'$.checks')>0 AND NOT EXISTS(SELECT 1 FROM json_each(r.data,'$.checks') WHERE json_extract(value,'$.exitCode') IS NULL OR json_extract(value,'$.exitCode')<>0)),0) checked,
        COALESCE(sum(p.state='failed'),0) failed,max(p.id) latest
        FROM organization_attempts p LEFT JOIN organization_results r ON r.attempt_id=p.id
        WHERE json_extract(p.data,'$.provider.id')=? AND json_extract(p.data,'$.provider.endpointSha256')=?"#)
            .bind(&evidence.id).bind(&evidence.endpoint_sha256).fetch_one(&mut *tx).await?;
        let models=sqlx::query_scalar("SELECT DISTINCT json_extract(a.data,'$.scope.model') FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id WHERE json_extract(p.data,'$.provider.id')=? AND json_extract(p.data,'$.provider.endpointSha256')=? ORDER BY 1")
            .bind(&evidence.id).bind(&evidence.endpoint_sha256).fetch_all(&mut *tx).await?;
        let observation = OrganizationProviderObservation {
            model_requests: row.try_get("requests")?,
            reported_model_requests: row.try_get("reported")?,
            checked_results: row.try_get("checked")?,
            failed_attempts: row.try_get("failed")?,
            models,
            latest_attempt_id: row.try_get("latest")?,
        };
        tx.commit().await?;
        Ok(observation)
    }

    pub async fn tool_observations(
        &self,
    ) -> Result<Vec<codex_protocol::organization_inventory::OrganizationToolCapability>> {
        use codex_protocol::organization_inventory::OrganizationToolCapability;
        let mut tx = self.pool.begin().await?;
        let mut tools = Vec::new();
        for name in ["exec_command", "apply_patch"] {
            let (completed,failed,uncertain,latest_attempt_id)=sqlx::query_as("SELECT COALESCE(sum(state='completed'),0),COALESCE(sum(state='failed'),0),COALESCE(sum(state='uncertain'),0),max(attempt_id) FROM organization_actions WHERE json_extract(data,'$.tool')=?")
                .bind(name).fetch_one(&mut *tx).await?;
            tools.push(OrganizationToolCapability {
                name: name.into(),
                permissions: String::new(),
                completed,
                failed,
                uncertain,
                latest_attempt_id,
            });
        }
        let (completed, latest_attempt_id) =
            sqlx::query_as("SELECT count(*),max(attempt_id) FROM (SELECT attempt_id,call_id FROM organization_coordination_calls UNION SELECT attempt_id,call_id FROM organization_checkpoints)")
                .fetch_one(&mut *tx)
                .await?;
        tools.push(OrganizationToolCapability {
            name: "organization".into(),
            permissions: String::new(),
            completed,
            failed: 0,
            uncertain: 0,
            latest_attempt_id,
        });
        tx.commit().await?;
        Ok(tools)
    }

    pub async fn operational_report(
        &self,
        project_id: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
        now: i64,
    ) -> Result<OrganizationReport> {
        ensure!(
            (1..=200).contains(&limit) && now >= 0,
            "invalid report limit or time"
        );
        ensure!(
            cursor.is_none_or(|cursor| !cursor.is_empty()
                && cursor.len() <= 256
                && !cursor.contains('\0')),
            "invalid report cursor"
        );
        let mut tx = self.pool.begin().await?;
        if let Some(project) = project_id {
            read_project(&mut tx, project).await?;
        }
        let sequence = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(max(sequence),0) FROM organization_audit",
        )
        .fetch_one(&mut *tx)
        .await?;
        let mut totals = OrganizationReportTotals {
            task_states: sqlx::query_as::<_,(String,i64)>("SELECT state,count(*) FROM organization_tasks WHERE (? IS NULL OR project_id=?) GROUP BY state")
            .bind(project_id).bind(project_id).fetch_all(&mut *tx).await?.into_iter().collect(),
            action_states: sqlx::query_as::<_,(String,i64)>("SELECT x.state,count(*) FROM organization_actions x JOIN organization_attempts p ON p.id=x.attempt_id JOIN organization_tasks t ON t.id=p.task_id WHERE (? IS NULL OR t.project_id=?) GROUP BY x.state")
            .bind(project_id).bind(project_id).fetch_all(&mut *tx).await?.into_iter().collect(),
            schedule_states: sqlx::query_as::<_,(String,i64)>("SELECT state,count(*) FROM organization_schedules WHERE (? IS NULL OR project_id=?) GROUP BY state")
            .bind(project_id).bind(project_id).fetch_all(&mut *tx).await?.into_iter().collect(),
            ..OrganizationReportTotals::default()
        };
        (totals.reported_tokens,totals.model_requests,totals.reported_model_requests)=sqlx::query_as("SELECT COALESCE(sum(json_extract(p.data,'$.reportedTokens')),0),COALESCE(sum(json_extract(p.data,'$.modelRequests')),0),COALESCE(sum(json_extract(p.data,'$.reportedModelRequests')),0) FROM organization_attempts p JOIN organization_tasks t ON t.id=p.task_id WHERE (? IS NULL OR t.project_id=?)")
            .bind(project_id).bind(project_id).fetch_one(&mut *tx).await?;
        (totals.checks_passed,totals.checks_failed)=sqlx::query_as("SELECT COALESCE(sum(json_extract(c.value,'$.exitCode')=0),0),COALESCE(sum((json_extract(c.value,'$.exitCode') IS NULL OR json_extract(c.value,'$.exitCode')<>0)),0) FROM organization_results r JOIN organization_attempts p ON p.id=r.attempt_id JOIN organization_tasks t ON t.id=p.task_id JOIN json_each(r.data,'$.checks') c WHERE (? IS NULL OR t.project_id=?)")
            .bind(project_id).bind(project_id).fetch_one(&mut *tx).await?;
        totals.pending_approvals=sqlx::query_scalar("SELECT count(*) FROM organization_approvals a JOIN organization_actions x ON x.id=a.action_id JOIN organization_attempts p ON p.id=x.attempt_id JOIN organization_tasks t ON t.id=p.task_id WHERE a.state='pending' AND t.state='waitingApproval' AND p.generation=(SELECT max(generation) FROM organization_attempts WHERE task_id=t.id) AND (? IS NULL OR t.project_id=?)")
            .bind(project_id).bind(project_id).fetch_one(&mut *tx).await?;
        totals.changes_requested=sqlx::query_scalar("SELECT count(*) FROM organization_tasks t WHERE t.state='paused' AND EXISTS(SELECT 1 FROM organization_outbox o WHERE o.kind='result/changesRequested' AND o.reference=t.id) AND (? IS NULL OR t.project_id=?)")
            .bind(project_id).bind(project_id).fetch_one(&mut *tx).await?;
        let rows=sqlx::query(r#"
WITH chosen AS (SELECT * FROM organization_tasks WHERE (? IS NULL OR project_id=?) AND (? IS NULL OR id>?) ORDER BY id LIMIT ?),
latest AS (SELECT p.* FROM organization_attempts p JOIN chosen t ON t.id=p.task_id WHERE p.generation=(SELECT max(generation) FROM organization_attempts WHERE task_id=t.id)),
usage AS (SELECT p.task_id,sum(json_extract(p.data,'$.reportedTokens')) tokens,sum(json_extract(p.data,'$.modelRequests')) requests,sum(json_extract(p.data,'$.reportedModelRequests')) reported FROM organization_attempts p JOIN chosen t ON t.id=p.task_id GROUP BY p.task_id)
SELECT t.id,t.project_id,t.owner_id,t.state,t.revision,substr(json_extract(t.data,'$.data.instructions'),1,160) title,
(SELECT run_id FROM organization_workflow_tasks WHERE task_id=t.id OR authorization_id=COALESCE(a.root_id,a.id) LIMIT 1) run_id,p.id attempt_id,r.digest,
c.id checkpoint_id,substr(json_extract(c.data,'$.content.blocker'),1,1024) blocker,
CASE WHEN t.state='paused' THEN (SELECT max(seq) FROM organization_outbox WHERE kind='result/changesRequested' AND reference=t.id) END changes,
(SELECT count(*) FROM organization_approvals approval JOIN organization_actions action ON action.id=approval.action_id WHERE action.attempt_id=p.id AND approval.state='pending' AND t.state='waitingApproval') pending,
(SELECT count(*) FROM json_each(r.data,'$.checks') WHERE json_extract(value,'$.exitCode')=0) passed,
(SELECT count(*) FROM json_each(r.data,'$.checks') WHERE json_extract(value,'$.exitCode') IS NULL OR json_extract(value,'$.exitCode')<>0) failed,
COALESCE(u.tokens,0) tokens,COALESCE(u.requests,0) requests,COALESCE(u.reported,0) reported,
a.id grant_id,root.id root_id,json_extract(root.data,'$.scope.model') model,
json_extract(root.data,'$.scope.maxTokens') max_tokens,root.used_tokens, json_extract(root.data,'$.scope.maxModelRequests') max_requests,root.model_requests,
(SELECT count(*) FROM organization_attempts WHERE authorization_id=a.id) attempts,
json_extract(a.data,'$.scope.maxAttempts') max_attempts,a.expires_at,a.revoked
FROM chosen t LEFT JOIN latest p ON p.task_id=t.id LEFT JOIN organization_results r ON r.attempt_id=p.id
LEFT JOIN usage u ON u.task_id=t.id
LEFT JOIN organization_authorizations a ON a.id=t.authorization_id LEFT JOIN organization_authorizations root ON root.id=COALESCE(a.root_id,a.id)
LEFT JOIN organization_checkpoints c ON c.id=(SELECT id FROM organization_checkpoints WHERE task_id=t.id AND json_extract(data,'$.authorizationId')=a.id ORDER BY id DESC LIMIT 1)
ORDER BY t.id
"#).bind(project_id).bind(project_id).bind(cursor).bind(cursor).bind(i64::from(limit)+1).fetch_all(&mut *tx).await?;
        let more = rows.len() > limit as usize;
        let mut tasks = Vec::new();
        for row in rows.into_iter().take(limit as usize) {
            let budget = if let Some(grant_id) = row.try_get::<Option<String>, _>("grant_id")? {
                Some(OrganizationBudgetReport {
                    grant_id,
                    root_id: row.try_get("root_id")?,
                    model: row.try_get("model")?,
                    max_tokens: row.try_get("max_tokens")?,
                    used_tokens: row.try_get("used_tokens")?,
                    max_model_requests: row.try_get("max_requests")?,
                    model_requests: row.try_get("model_requests")?,
                    task_attempts: row.try_get("attempts")?,
                    task_max_attempts: row.try_get("max_attempts")?,
                    expires_at: row.try_get("expires_at")?,
                    revoked: row.try_get("revoked")?,
                })
            } else {
                None
            };
            tasks.push(OrganizationTaskReport {
                task_id: row.try_get("id")?,
                project_id: row.try_get("project_id")?,
                owner_id: row.try_get("owner_id")?,
                workflow_run_id: row.try_get("run_id")?,
                title: row.try_get("title")?,
                state: row.try_get("state")?,
                revision: row.try_get("revision")?,
                attempt_id: row.try_get("attempt_id")?,
                result_digest: row.try_get("digest")?,
                checkpoint_id: row.try_get("checkpoint_id")?,
                blocker: row.try_get("blocker")?,
                changes_requested_sequence: row.try_get("changes")?,
                pending_approvals: row.try_get("pending")?,
                checks_passed: row.try_get("passed")?,
                checks_failed: row.try_get("failed")?,
                reported_tokens: row.try_get("tokens")?,
                model_requests: row.try_get("requests")?,
                reported_model_requests: row.try_get("reported")?,
                budget,
            });
        }
        let task_ids =
            serde_json::to_string(&tasks.iter().map(|task| &task.task_id).collect::<Vec<_>>())?;
        let rows=sqlx::query(r#"SELECT p.task_id,(SELECT run_id FROM organization_workflow_tasks WHERE authorization_id=COALESCE(a.root_id,a.id) LIMIT 1) run_id,json_extract(p.data,'$.ownerId') owner_id,json_extract(p.data,'$.provider') provider,json_extract(a.data,'$.scope.model') model,
sum(json_extract(p.data,'$.reportedTokens')) tokens,sum(json_extract(p.data,'$.modelRequests')) requests,sum(json_extract(p.data,'$.reportedModelRequests')) reported,count(*) attempts
FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id
WHERE p.task_id IN (SELECT value FROM json_each(?)) GROUP BY p.task_id,run_id,owner_id,provider,model ORDER BY p.task_id,owner_id,model,provider LIMIT 2001"#)
            .bind(task_ids).fetch_all(&mut *tx).await?;
        let usage_truncated = rows.len() > 2000;
        let mut usage = Vec::new();
        for row in rows.into_iter().take(2000) {
            let provider = row
                .try_get::<Option<String>, _>("provider")?
                .map(|json| serde_json::from_str(&json))
                .transpose()?;
            usage.push(OrganizationUsageReport {
                task_id: row.try_get("task_id")?,
                workflow_run_id: row.try_get("run_id")?,
                owner_id: row.try_get("owner_id")?,
                provider,
                model: row.try_get("model")?,
                reported_tokens: row.try_get("tokens")?,
                model_requests: row.try_get("requests")?,
                reported_model_requests: row.try_get("reported")?,
                attempts: row.try_get("attempts")?,
            });
        }
        let upcoming_runs=sqlx::query("SELECT id,project_id,json_extract(data,'$.config.label') label,next_run_at FROM organization_schedules WHERE state='active' AND next_run_at IS NOT NULL AND (? IS NULL OR project_id=?) ORDER BY next_run_at,id LIMIT 16")
            .bind(project_id).bind(project_id).fetch_all(&mut *tx).await?.into_iter().map(|row|Ok(OrganizationUpcomingRun {schedule_id:row.try_get("id")?,project_id:row.try_get("project_id")?,label:row.try_get("label")?,due_at:row.try_get("next_run_at")?})).collect::<Result<Vec<_>>>()?;
        let next_cursor = more
            .then(|| tasks.last().map(|task| task.task_id.clone()))
            .flatten();
        let (reported_tokens, model_requests, active, unresolved_requests) = sqlx::query_as("SELECT reported_tokens,model_requests,active,unresolved_requests FROM organization_global_usage")
            .fetch_one(&mut *tx).await?;
        let global_usage = OrganizationGlobalUsage {
            reported_tokens,
            model_requests,
            active,
            unresolved_requests,
        };
        let rows=sqlx::query(r#"SELECT t.project_id,
(SELECT run_id FROM organization_workflow_tasks WHERE authorization_id=COALESCE(a.root_id,a.id) LIMIT 1) captured_run,
json_extract(p.data,'$.ownerId') captured_owner,json_extract(p.data,'$.provider') captured_provider,json_extract(a.data,'$.scope.model') captured_model,
count(*) attempts,COALESCE(sum(p.state='accepted' AND json_extract(r.data,'$.accepted')=1),0) accepted,
COALESCE(sum(p.state='failed'),0) failed,COALESCE(sum(p.state='blocked'),0) blocked,COALESCE(sum(p.state='needsReconciliation'),0) uncertain,
COALESCE(sum((SELECT count(*) FROM json_each(r.data,'$.checks') WHERE json_extract(value,'$.exitCode')=0)),0) passed,
COALESCE(sum((SELECT count(*) FROM json_each(r.data,'$.checks') WHERE json_extract(value,'$.exitCode') IS NULL OR json_extract(value,'$.exitCode')<>0)),0) checks_failed,
COALESCE(sum(json_extract(p.data,'$.reportedTokens')),0) tokens,COALESCE(sum(json_extract(p.data,'$.modelRequests')),0) requests,COALESCE(sum(json_extract(p.data,'$.reportedModelRequests')),0) reported
FROM organization_attempts p JOIN organization_tasks t ON t.id=p.task_id JOIN organization_authorizations a ON a.id=p.authorization_id
LEFT JOIN organization_results r ON r.attempt_id=p.id WHERE (? IS NULL OR t.project_id=?)
GROUP BY t.project_id,captured_run,captured_owner,captured_provider,captured_model
ORDER BY t.project_id,captured_run,captured_owner,captured_provider,captured_model LIMIT 2001"#)
            .bind(project_id).bind(project_id).fetch_all(&mut *tx).await?;
        let truncated = rows.len() > 2000;
        let mut entries = Vec::new();
        for row in rows.into_iter().take(2000) {
            entries.push(OrganizationScorecard {
                project_id: row.try_get("project_id")?,
                workflow_run_id: row.try_get("captured_run")?,
                owner_id: row.try_get("captured_owner")?,
                provider: row
                    .try_get::<Option<String>, _>("captured_provider")?
                    .map(|s| serde_json::from_str(&s))
                    .transpose()?,
                model: row.try_get("captured_model")?,
                attempts: row.try_get("attempts")?,
                accepted_results: row.try_get("accepted")?,
                failed_attempts: row.try_get("failed")?,
                blocked_attempts: row.try_get("blocked")?,
                uncertain_attempts: row.try_get("uncertain")?,
                checks_passed: row.try_get("passed")?,
                checks_failed: row.try_get("checks_failed")?,
                reported_tokens: row.try_get("tokens")?,
                model_requests: row.try_get("requests")?,
                reported_model_requests: row.try_get("reported")?,
            });
        }
        let audit_events = if project_id.is_none() {
            let rows=sqlx::query("SELECT sequence,actor,operation,created_at,COALESCE(json_extract(payload,'$.id'),json_extract(payload,'$.taskId'),json_extract(payload,'$.projectId'),json_extract(payload,'$.nodeId'),json_extract(payload,'$.groupId')) reference FROM organization_audit ORDER BY sequence DESC LIMIT 200").fetch_all(&mut *tx).await?;
            Some(
                rows.into_iter()
                    .map(|row| {
                        Ok(OrganizationAuditEvent {
                            sequence: row.try_get("sequence")?,
                            actor: row.try_get("actor")?,
                            operation: row.try_get("operation")?,
                            reference: row.try_get("reference")?,
                            created_at: row.try_get("created_at")?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            )
        } else {
            None
        };
        let resource_pressure = super::resource::pressure_observation(&mut tx, now).await?;
        tx.commit().await?;
        Ok(OrganizationReport {
            resource_pressure,
            sequence,
            generated_at: now,
            project_id: project_id.map(str::to_owned),
            totals,
            global_usage: Some(global_usage),
            scorecards: Some(OrganizationScorecards { entries, truncated }),
            audit_events,
            runtime_resources: None,
            tasks,
            usage,
            usage_truncated,
            upcoming_runs,
            next_cursor,
        })
    }
}

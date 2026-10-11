//! Finite workflow recurrence and accepted-result follow-ups reuse exact run transactions.
use super::OrganizationStore;
use super::execution::*;
use super::workflow::{preview_in, run_in};
use anyhow::{Context, Result, ensure};
use codex_protocol::organization_execution::OrganizationTaskAuthorization;
use codex_protocol::organization_schedule::{
    OrganizationScheduleConfig, OrganizationScheduleState,
};
use codex_protocol::organization_workflow::*;
use sqlx::{Sqlite, Transaction};
use uuid::Uuid;

async fn pinned(
    tx: &mut Transaction<'_, Sqlite>,
    preview: &OrganizationWorkflowPreview,
) -> Result<()> {
    let fresh = preview_in(
        tx,
        OrganizationWorkflowPreviewRequest {
            id: preview.id.clone(),
            template_id: preview.template.id.clone(),
            expected_revision: preview.template.revision,
            scopes: preview.scopes.clone(),
            shared_limits: preview.shared_limits.clone(),
        },
    )
    .await?;
    ensure!(
        fresh == *preview,
        "pinned workflow definitions changed; review a new series"
    );
    ensure!(
        preview
            .scopes
            .iter()
            .all(|step| step.scope.not_before.is_none()),
        "use the series time instead of step notBefore"
    );
    Ok(())
}

pub(super) async fn create_in(
    tx: &mut Transaction<'_, Sqlite>,
    id: String,
    preview: Box<OrganizationWorkflowPreview>,
    config: OrganizationScheduleConfig,
    trigger: OrganizationWorkflowTrigger,
    now: i64,
) -> Result<OrganizationWorkflowView> {
    Uuid::parse_str(&id)?;
    config.validate(now).map_err(anyhow::Error::msg)?;
    pinned(tx, &preview).await?;
    let live: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_workflow_series WHERE state IN ('active','paused','blocked')").fetch_one(&mut **tx).await?;
    ensure!(live < 128, "live workflow series ceiling is 128");
    if let OrganizationWorkflowTrigger::Accepted {
        task_id,
        authorization_id,
    } = &trigger
    {
        ensure!(
            config.max_runs == 1,
            "accepted-result triggers issue one bounded follow-up workflow"
        );
        let source = read_task(tx, task_id).await?;
        let current: Option<String> =
            sqlx::query_scalar("SELECT authorization_id FROM organization_tasks WHERE id=?")
                .bind(task_id)
                .fetch_one(&mut **tx)
                .await?;
        ensure!(
            source.project_id == preview.template.project_id
                && current.as_ref() == Some(authorization_id),
            "trigger must pin the source's current authorization in the same project"
        );
    }
    let series = OrganizationWorkflowSeries {
        id,
        next_run_at: Some(config.first_run_at),
        preview: *preview,
        config,
        trigger,
        state: OrganizationScheduleState::Active,
        revision: 1,
        issued_runs: 0,
        skipped_runs: 0,
        reason: None,
        source_result_digest: None,
    };
    sqlx::query("INSERT INTO organization_workflow_series(id,project_id,state,next_run_at,data) VALUES (?,?,'active',?,?)").bind(&series.id).bind(&series.preview.template.project_id).bind(series.next_run_at).bind(serde_json::to_string(&series)?).execute(&mut **tx).await?;
    let limit = OrganizationSharedLimit {
        id: format!("series:{}", series.id),
        project_id: series.preview.template.project_id.clone(),
        run_id: None,
        series_id: Some(series.id.clone()),
        revision: 1,
        max_tokens: series.config.max_tokens,
        max_model_requests: series.config.max_model_requests,
        max_active: 1,
    };
    sqlx::query("INSERT INTO organization_shared_limits(id,project_id,revision,series_id,data) VALUES (?,?,1,?,?)").bind(&limit.id).bind(&limit.project_id).bind(&series.id).bind(serde_json::to_string(&limit)?).execute(&mut **tx).await?;
    owner_event(tx, "workflowSeries/create", &series.id).await?;
    view_in(tx, series).await
}

pub(super) async fn view_in(
    tx: &mut Transaction<'_, Sqlite>,
    series: OrganizationWorkflowSeries,
) -> Result<OrganizationWorkflowView> {
    let (reported_tokens, model_requests, active): (i64, i64, i64) = sqlx::query_as(
        "SELECT reported_tokens,model_requests,active FROM organization_shared_usage WHERE id=?",
    )
    .bind(format!("series:{}", series.id))
    .fetch_one(&mut **tx)
    .await?;
    Ok(OrganizationWorkflowView::Series {
        series: Box::new(series),
        usage: OrganizationSharedUsage {
            reported_tokens,
            model_requests,
            active,
        },
    })
}

async fn write(
    tx: &mut Transaction<'_, Sqlite>,
    series: &mut OrganizationWorkflowSeries,
) -> Result<()> {
    series.revision = series
        .revision
        .checked_add(1)
        .context("series revision exhausted")?;
    sqlx::query("UPDATE organization_workflow_series SET state=?,next_run_at=?,data=? WHERE id=?")
        .bind(series.state.as_str())
        .bind(series.next_run_at)
        .bind(serde_json::to_string(series)?)
        .bind(&series.id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn unsafe_usage(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<bool> {
    let pending: i64 =
        sqlx::query_scalar("SELECT unresolved_requests FROM organization_shared_usage WHERE id=?")
            .bind(format!("series:{id}"))
            .fetch_one(&mut **tx)
            .await?;
    let failures: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_workflow_occurrences o JOIN organization_workflow_tasks w ON w.run_id=o.run_id JOIN organization_authorizations a ON a.root_id=w.authorization_id JOIN organization_tasks t ON t.id=a.task_id WHERE o.series_id=? AND t.state IN ('needsReconciliation','failed','blocked')").bind(id).fetch_one(&mut **tx).await?;
    Ok(pending > 0 || failures > 0)
}

pub(super) async fn control_in(
    tx: &mut Transaction<'_, Sqlite>,
    id: String,
    expected_revision: i64,
    action: OrganizationWorkflowControl,
    now: i64,
) -> Result<OrganizationWorkflowView> {
    let data: String =
        sqlx::query_scalar("SELECT data FROM organization_workflow_series WHERE id=?")
            .bind(&id)
            .fetch_one(&mut **tx)
            .await?;
    let mut series: OrganizationWorkflowSeries = serde_json::from_str(&data)?;
    ensure!(
        series.revision == expected_revision,
        "workflow series revision conflict"
    );
    ensure!(
        series.state != OrganizationScheduleState::Cancelled,
        "workflow series already cancelled"
    );
    match action {
        OrganizationWorkflowControl::Pause => {
            ensure!(
                series.state == OrganizationScheduleState::Active,
                "series is not active"
            );
            series.state = OrganizationScheduleState::Paused;
            series.reason =
                Some("future invocations paused; use run controls for current work".into());
        }
        OrganizationWorkflowControl::Resume => {
            ensure!(
                matches!(
                    series.state,
                    OrganizationScheduleState::Paused | OrganizationScheduleState::Blocked
                ),
                "series is not paused or blocked"
            );
            pinned(tx, &series.preview).await?;
            ensure!(
                !unsafe_usage(tx, &id).await?,
                "series has unresolved or failed work; investigate before continuing"
            );
            let (tokens, requests): (i64, i64) = sqlx::query_as(
                "SELECT reported_tokens,model_requests FROM organization_shared_usage WHERE id=?",
            )
            .bind(format!("series:{id}"))
            .fetch_one(&mut **tx)
            .await?;
            ensure!(
                now < series.config.ends_at
                    && series.issued_runs < series.config.max_runs
                    && tokens < series.config.max_tokens
                    && requests < i64::from(series.config.max_model_requests),
                "series limits exhausted"
            );
            series.state = OrganizationScheduleState::Active;
            series.reason = None;
            if matches!(series.trigger, OrganizationWorkflowTrigger::Interval) {
                skip_overdue(&mut series, now)?;
            }
        }
        OrganizationWorkflowControl::Cancel => {
            let tasks: Vec<String> = sqlx::query_scalar("SELECT t.id FROM organization_workflow_occurrences o JOIN organization_workflow_tasks w ON w.run_id=o.run_id JOIN organization_authorizations a ON a.root_id=w.authorization_id JOIN organization_tasks t ON t.id=a.task_id WHERE o.series_id=? AND t.state IN ('queued','running','waitingApproval','waitingDependency','paused','blocked')").bind(&id).fetch_all(&mut **tx).await?;
            for task_id in tasks {
                let task = read_task(tx, &task_id).await?;
                super::actions::stop_task_in(tx, &task_id, task.revision, true).await?;
            }
            sqlx::query("UPDATE organization_workflow_runs SET data=json_set(data,'$.state','cancelled','$.revision',COALESCE(json_extract(data,'$.revision'),1)+1) WHERE id IN (SELECT run_id FROM organization_workflow_occurrences WHERE series_id=?)").bind(&id).execute(&mut **tx).await?;
            series.state = OrganizationScheduleState::Cancelled;
            series.next_run_at = None;
            series.reason = Some("series and unfinished invocations cancelled by owner".into());
        }
    }
    write(tx, &mut series).await?;
    owner_event(tx, "workflowSeries/control", &id).await?;
    view_in(tx, series).await
}

fn skip_overdue(series: &mut OrganizationWorkflowSeries, now: i64) -> Result<()> {
    if let Some(due) = series.next_run_at.filter(|due| *due <= now) {
        let interval = i64::from(series.config.interval_seconds);
        let skipped = (now - due) / interval + 1;
        series.skipped_runs = series
            .skipped_runs
            .checked_add(u32::try_from(skipped)?)
            .context("series skip count overflow")?;
        series.next_run_at = Some(
            due.checked_add(
                skipped
                    .checked_mul(interval)
                    .context("series time overflow")?,
            )
            .context("series time overflow")?,
        );
    }
    Ok(())
}

impl OrganizationStore {
    /// Startup skips past intervals; interrupted attempts and unknown usage remain fenced.
    pub async fn reconcile_workflow_series(&self, now: i64) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let rows: Vec<String> = sqlx::query_scalar(
            "SELECT data FROM organization_workflow_series WHERE state='active'",
        )
        .fetch_all(&mut *tx)
        .await?;
        for data in rows {
            let mut series: OrganizationWorkflowSeries = serde_json::from_str(&data)?;
            if unsafe_usage(&mut tx, &series.id).await? {
                series.state = OrganizationScheduleState::Blocked;
                series.reason =
                    Some("interrupted, failed or unknown work needs owner investigation".into());
            } else if matches!(series.trigger, OrganizationWorkflowTrigger::Interval) {
                skip_overdue(&mut series, now)?;
            }
            write(&mut tx, &mut series).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn materialize_workflow_series(&self, now: i64) -> Result<u32> {
        ensure!(now >= 0, "invalid workflow series time");
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let rows: Vec<String>=sqlx::query_scalar("SELECT s.data FROM organization_workflow_series s WHERE s.state='active' AND s.next_run_at<=? AND NOT EXISTS(SELECT 1 FROM organization_controls c WHERE (c.id='global' AND json_extract(c.data,'$.data.value.paused')=1) OR (c.id='project:'||s.project_id AND json_extract(c.data,'$.data.value.state')<>'active')) ORDER BY s.next_run_at,s.id LIMIT 32").bind(now).fetch_all(&mut *tx).await?;
        let mut issued = 0;
        for data in rows {
            let mut series: OrganizationWorkflowSeries = serde_json::from_str(&data)?;
            let due = series.next_run_at.context("series due time missing")?;
            let (tokens, requests): (i64, i64) = sqlx::query_as(
                "SELECT reported_tokens,model_requests FROM organization_shared_usage WHERE id=?",
            )
            .bind(format!("series:{}", series.id))
            .fetch_one(&mut *tx)
            .await?;
            if now >= series.config.ends_at
                || series.issued_runs >= series.config.max_runs
                || tokens >= series.config.max_tokens
                || requests >= i64::from(series.config.max_model_requests)
            {
                series.state = OrganizationScheduleState::Completed;
                series.next_run_at = None;
                series.reason = Some("finite series time, run or aggregate ceiling reached".into());
            } else if pinned(&mut tx, &series.preview).await.is_err()
                || unsafe_usage(&mut tx, &series.id).await?
            {
                series.state = OrganizationScheduleState::Blocked;
                series.reason=Some("pinned definitions changed, failed work or unknown usage; owner investigation required".into());
            } else {
                let busy: i64=sqlx::query_scalar("SELECT count(*) FROM organization_workflow_occurrences o JOIN organization_workflow_tasks w ON w.run_id=o.run_id JOIN organization_tasks t ON t.id=w.task_id WHERE o.series_id=? AND t.state NOT IN ('accepted','cancelled')").bind(&series.id).fetch_one(&mut *tx).await?;
                let ready = match &series.trigger {
                    OrganizationWorkflowTrigger::Interval => {
                        now - due < i64::from(series.config.interval_seconds) && busy == 0
                    }
                    OrganizationWorkflowTrigger::Accepted {
                        task_id,
                        authorization_id,
                    } => {
                        let (project,grant,state):(String,Option<String>,String)=sqlx::query_as("SELECT project_id,authorization_id,state FROM organization_tasks WHERE id=?").bind(task_id).fetch_one(&mut *tx).await?;
                        if project != series.preview.template.project_id
                            || grant.as_ref() != Some(authorization_id)
                            || matches!(
                                state.as_str(),
                                "failed" | "cancelled" | "needsReconciliation"
                            )
                        {
                            series.state = OrganizationScheduleState::Blocked;
                            series.reason =
                                Some("trigger source authorization changed or failed".into());
                            false
                        } else if state == "accepted" {
                            let digest: Option<String>=sqlx::query_scalar("SELECT r.digest FROM organization_results r JOIN organization_attempts p ON p.id=r.attempt_id WHERE p.task_id=? AND p.authorization_id=? AND json_extract(r.data,'$.accepted')=1 ORDER BY p.generation DESC LIMIT 1").bind(task_id).bind(authorization_id).fetch_optional(&mut *tx).await?;
                            series.source_result_digest = digest;
                            series.source_result_digest.is_some()
                        } else {
                            false
                        }
                    }
                };
                if ready {
                    let mut preview = series.preview.clone();
                    preview.id = Uuid::now_v7().to_string();
                    let view = run_in(&mut tx, Box::new(preview), now).await?;
                    let OrganizationWorkflowView::Run { run, .. } = view else {
                        unreachable!()
                    };
                    sqlx::query("INSERT INTO organization_workflow_occurrences(series_id,due_at,run_id) VALUES (?,?,?)").bind(&series.id).bind(due).bind(&run.id).execute(&mut *tx).await?;
                    // Every step keeps its captured scope; no invocation can outlive the series.
                    for task_id in run.task_ids.values() {
                        let (grant_id,data):(String,String)=sqlx::query_as("SELECT a.id,a.data FROM organization_authorizations a JOIN organization_tasks t ON t.authorization_id=a.id WHERE t.id=?").bind(task_id).fetch_one(&mut *tx).await?;
                        let mut grant: OrganizationTaskAuthorization = serde_json::from_str(&data)?;
                        grant.expires_at = grant.expires_at.min(series.config.ends_at);
                        sqlx::query(
                            "UPDATE organization_authorizations SET expires_at=?,data=? WHERE id=?",
                        )
                        .bind(grant.expires_at)
                        .bind(serde_json::to_string(&grant)?)
                        .bind(grant_id)
                        .execute(&mut *tx)
                        .await?;
                    }
                    series.issued_runs += 1;
                    issued += 1;
                    series.next_run_at = due.checked_add(i64::from(series.config.interval_seconds));
                    series.reason = None;
                    if series.issued_runs >= series.config.max_runs
                        || matches!(series.trigger, OrganizationWorkflowTrigger::Accepted { .. })
                        || series
                            .next_run_at
                            .is_none_or(|time| time >= series.config.ends_at)
                    {
                        series.state = OrganizationScheduleState::Completed;
                        series.next_run_at = None;
                    }
                    event(&mut tx, "workflowSeries/occurrence", &run.id).await?;
                } else if matches!(series.trigger, OrganizationWorkflowTrigger::Interval) {
                    skip_overdue(&mut series, now)?;
                    series.reason = Some(
                        "missed or overlapping invocations skipped; acceptance remains explicit"
                            .into(),
                    );
                } else if series.state == OrganizationScheduleState::Active {
                    series.next_run_at = Some(now.saturating_add(5));
                }
            }
            write(&mut tx, &mut series).await?;
        }
        tx.commit().await?;
        Ok(issued)
    }
}

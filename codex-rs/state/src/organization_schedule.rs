//! Short transactions materialize finite schedules; runtime work never holds this lock.

use super::OrganizationStore;
use super::actions::stop_task_in;
use super::execution::*;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::OrganizationRecord;
use codex_protocol::organization_execution::*;
use codex_protocol::organization_schedule::*;
use sqlx::Sqlite;
use sqlx::Transaction;
use uuid::Uuid;

impl OrganizationStore {
    pub async fn mutate_schedule(
        &self,
        mutation: OrganizationScheduleMutation,
        now: i64,
    ) -> Result<OrganizationScheduleView> {
        ensure!(now >= 0, "invalid schedule time");
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let schedule = match mutation {
            OrganizationScheduleMutation::Create {
                id,
                task_id,
                expected_revision,
                scope,
                config,
            } => {
                Uuid::parse_str(&id)?;
                config.validate(now).map_err(anyhow::Error::msg)?;
                validate_scope(&scope)?;
                ensure!(
                    scope.not_before.is_none(),
                    "use schedule firstRunAt instead of scope notBefore"
                );
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_schedules WHERE state IN ('active','paused','blocked')")
                    .fetch_one(&mut *tx).await?;
                ensure!(
                    count < 128,
                    "live schedule ceiling is 128; cancel unused schedules"
                );
                let task = read_task(&mut tx, &task_id).await?;
                ensure!(
                    task.revision == expected_revision,
                    "template task revision conflict"
                );
                ensure!(
                    task.prerequisites.is_empty(),
                    "recurring templates cannot have prerequisites"
                );
                let (node, project) = scope_definitions(&mut tx, &task, &scope).await?;
                let schedule = OrganizationSchedule {
                    id,
                    template: task,
                    policy_version: scope.policy_version(),
                    scope: *scope,
                    next_run_at: Some(config.first_run_at),
                    config,
                    node_revision: node.revision,
                    project_revision: project.revision,
                    state: OrganizationScheduleState::Active,
                    revision: 1,
                    issued_runs: 0,
                    skipped_runs: 0,
                    reason: None,
                };
                sqlx::query("INSERT INTO organization_schedules(id,project_id,state,next_run_at,data) VALUES (?,?,'active',?,?)")
                    .bind(&schedule.id).bind(&schedule.template.project_id).bind(schedule.next_run_at)
                    .bind(serde_json::to_string(&schedule)?).execute(&mut *tx).await?;
                owner_event(&mut tx, "schedule/create", &schedule.id).await?;
                schedule
            }
            OrganizationScheduleMutation::Control {
                id,
                expected_revision,
                action,
            } => {
                let mut schedule = read_schedule(&mut tx, &id).await?;
                ensure!(
                    schedule.revision == expected_revision,
                    "schedule revision conflict; refresh before retrying"
                );
                ensure!(
                    matches!(
                        schedule.state,
                        OrganizationScheduleState::Active
                            | OrganizationScheduleState::Paused
                            | OrganizationScheduleState::Blocked
                            | OrganizationScheduleState::Completed
                    ),
                    "schedule is already cancelled"
                );
                match action {
                    OrganizationScheduleControl::Resume => {
                        ensure!(
                            matches!(
                                schedule.state,
                                OrganizationScheduleState::Paused
                                    | OrganizationScheduleState::Blocked
                            ),
                            "schedule is not paused or blocked"
                        );
                        ensure!(
                            definitions_current(&mut tx, &schedule).await?,
                            "pinned scope changed; create a new schedule after reviewing it"
                        );
                        ensure!(
                            !uncertain(&mut tx, &id).await?,
                            "schedule has interrupted work, unresolved usage or uncertain actions; investigate before creating a new series"
                        );
                        let (used, requests) = usage(&mut tx, &id).await?;
                        ensure!(
                            now < schedule.config.ends_at
                                && schedule.issued_runs < schedule.config.max_runs
                                && used < schedule.config.max_tokens
                                && requests < i64::from(schedule.config.max_model_requests),
                            "schedule limits exhausted"
                        );
                        schedule.state = OrganizationScheduleState::Active;
                        schedule.reason = None;
                        advance_overdue(&mut schedule, now)?;
                        owner_event(&mut tx, "schedule/resume", &id).await?;
                    }
                    OrganizationScheduleControl::Pause | OrganizationScheduleControl::Cancel => {
                        let cancel = action == OrganizationScheduleControl::Cancel;
                        // Reuse the existing revoke/approval-supersession path, including children.
                        let tasks: Vec<String> = sqlx::query_scalar("SELECT DISTINCT t.id FROM organization_tasks t JOIN organization_authorizations a ON a.task_id=t.id JOIN organization_schedule_occurrences o ON o.authorization_id=a.root_id WHERE o.schedule_id=? AND t.state IN ('queued','running','waitingApproval','waitingDependency','paused','needsReconciliation')")
                            .bind(&id).fetch_all(&mut *tx).await?;
                        for id in tasks {
                            let task = read_task(&mut tx, &id).await?;
                            stop_task_in(&mut tx, &id, task.revision, cancel).await?;
                        }
                        schedule.state = if cancel {
                            OrganizationScheduleState::Cancelled
                        } else {
                            OrganizationScheduleState::Paused
                        };
                        schedule.reason = Some(
                            if cancel {
                                "cancelled by owner"
                            } else {
                                "paused by owner; resume creates future runs only"
                            }
                            .into(),
                        );
                        if cancel {
                            schedule.next_run_at = None;
                        }
                        owner_event(
                            &mut tx,
                            if cancel {
                                "schedule/cancel"
                            } else {
                                "schedule/pause"
                            },
                            &id,
                        )
                        .await?;
                    }
                }
                write_schedule(&mut tx, &mut schedule).await?;
                schedule
            }
        };
        let view = view_in(&mut tx, schedule).await?;
        tx.commit().await?;
        Ok(view)
    }

    pub async fn list_schedules(
        &self,
        project_id: Option<&str>,
        cursor: Option<&str>,
        limit: u32,
    ) -> Result<(Vec<OrganizationScheduleView>, Option<String>)> {
        ensure!(
            (1..=200).contains(&limit),
            "schedule list limit must be 1–200"
        );
        ensure!(
            cursor.is_none_or(|s| !s.is_empty() && s.len() <= 256 && !s.contains('\0')),
            "invalid schedule cursor"
        );
        let mut tx = self.pool.begin().await?;
        if let Some(id) = project_id {
            read_project(&mut tx, id).await?;
        }
        let mut rows: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_schedules WHERE id>? AND (? IS NULL OR project_id=?) ORDER BY id LIMIT ?")
            .bind(cursor.unwrap_or("")).bind(project_id).bind(project_id).bind(limit + 1).fetch_all(&mut *tx).await?;
        let more = rows.len() > limit as usize;
        rows.truncate(limit as usize);
        let mut views = Vec::new();
        for row in rows {
            views.push(view_in(&mut tx, serde_json::from_str(&row)?).await?);
        }
        let cursor = more
            .then(|| views.last().map(|v| v.schedule.id.clone()))
            .flatten();
        tx.commit().await?;
        Ok((views, cursor))
    }

    /// Called once while acquiring the execution-owner lock, after attempt reconciliation.
    pub async fn reconcile_schedules(&self, now: i64) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let rows: Vec<String> =
            sqlx::query_scalar("SELECT data FROM organization_schedules WHERE state='active'")
                .fetch_all(&mut *tx)
                .await?;
        for row in rows {
            let mut schedule: OrganizationSchedule = serde_json::from_str(&row)?;
            if uncertain(&mut tx, &schedule.id).await? {
                schedule.state = OrganizationScheduleState::Blocked;
                schedule.reason = Some("interrupted work, unresolved usage or uncertain effects; owner investigation required".into());
            } else {
                advance_overdue(&mut schedule, now)?;
            }
            write_schedule(&mut tx, &mut schedule).await?;
            event(&mut tx, "schedule/reconcile", &schedule.id).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    /// At most 32 due schedules per cycle, bounded by the 128-live-series ceiling.
    pub async fn materialize_schedules(&self, now: i64) -> Result<u32> {
        ensure!(now >= 0, "invalid schedule time");
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let rows: Vec<String> = sqlx::query_scalar("SELECT s.data FROM organization_schedules s WHERE s.state='active' AND s.next_run_at<=? AND NOT EXISTS(SELECT 1 FROM organization_controls c WHERE (c.id='global' AND json_extract(c.data,'$.data.value.paused')=1) OR (c.id='project:'||s.project_id AND json_extract(c.data,'$.data.value.state')<>'active')) ORDER BY s.next_run_at,s.id LIMIT 32")
            .bind(now).fetch_all(&mut *tx).await?;
        let mut issued = 0;
        for row in rows {
            let mut schedule: OrganizationSchedule = serde_json::from_str(&row)?;
            let due = schedule
                .next_run_at
                .context("active schedule due time missing")?;
            let (used, requests) = usage(&mut tx, &schedule.id).await?;
            if now >= schedule.config.ends_at
                || schedule.issued_runs >= schedule.config.max_runs
                || used >= schedule.config.max_tokens
                || requests >= i64::from(schedule.config.max_model_requests)
            {
                complete(
                    &mut schedule,
                    "schedule run, time or aggregate budget limit reached",
                );
            } else if !definitions_current(&mut tx, &schedule).await?
                || uncertain(&mut tx, &schedule.id).await?
            {
                schedule.state = OrganizationScheduleState::Blocked;
                schedule.reason = Some("pinned scope changed or interrupted/uncertain work; owner investigation required".into());
            } else if now - due >= i64::from(schedule.config.interval_seconds)
                || busy(&mut tx, &schedule.id).await?
            {
                advance_overdue(&mut schedule, now)?;
                schedule.reason = Some("missed or overlapping occurrences skipped".into());
            } else {
                let mut task = schedule.template.clone();
                task.id = Uuid::now_v7().to_string();
                task.state = "proposed".into();
                task.revision = 1;
                sqlx::query("INSERT INTO organization_tasks(id,project_id,goal_id,owner_id,revision,state,data) VALUES (?,?,?,?,1,'proposed',?)")
                    .bind(&task.id).bind(&task.project_id).bind(&task.goal_id).bind(&task.owner_id)
                    .bind(serde_json::to_string(&OrganizationRecord::Task(task.clone()))?).execute(&mut *tx).await?;
                let mut scope = schedule.scope.clone();
                scope.not_before = Some(due);
                let mut grant = authorize_task_in(&mut tx, &task.id, 1, scope, now).await?;
                grant.expires_at = grant.expires_at.min(schedule.config.ends_at);
                sqlx::query(
                    "UPDATE organization_authorizations SET expires_at=?,data=? WHERE id=?",
                )
                .bind(grant.expires_at)
                .bind(serde_json::to_string(&grant)?)
                .bind(&grant.id)
                .execute(&mut *tx)
                .await?;
                sqlx::query("INSERT INTO organization_schedule_occurrences(schedule_id,due_at,task_id,authorization_id) VALUES (?,?,?,?)")
                    .bind(&schedule.id).bind(due).bind(&task.id).bind(&grant.id).execute(&mut *tx).await?;
                schedule.issued_runs += 1;
                schedule.next_run_at = due.checked_add(i64::from(schedule.config.interval_seconds));
                schedule.reason = None;
                if schedule.issued_runs == schedule.config.max_runs
                    || schedule
                        .next_run_at
                        .is_none_or(|t| t >= schedule.config.ends_at)
                {
                    complete(&mut schedule, "all configured occurrences issued");
                }
                issued += 1;
                event(&mut tx, "schedule/occurrence", &task.id).await?;
            }
            write_schedule(&mut tx, &mut schedule).await?;
        }
        tx.commit().await?;
        Ok(issued)
    }
}

fn complete(schedule: &mut OrganizationSchedule, reason: &str) {
    schedule.state = OrganizationScheduleState::Completed;
    schedule.next_run_at = None;
    schedule.reason = Some(reason.into());
}

fn advance_overdue(schedule: &mut OrganizationSchedule, now: i64) -> Result<()> {
    if let Some(due) = schedule.next_run_at.filter(|due| *due <= now) {
        let interval = i64::from(schedule.config.interval_seconds);
        let skipped = (now - due) / interval + 1;
        schedule.skipped_runs = schedule
            .skipped_runs
            .checked_add(u32::try_from(skipped)?)
            .context("schedule skip count overflow")?;
        schedule.next_run_at = Some(
            due.checked_add(
                skipped
                    .checked_mul(interval)
                    .context("schedule time overflow")?,
            )
            .context("schedule time overflow")?,
        );
    }
    if schedule
        .next_run_at
        .is_none_or(|time| time >= schedule.config.ends_at)
    {
        complete(schedule, "schedule end time reached");
    }
    Ok(())
}

async fn definitions_current(
    tx: &mut Transaction<'_, Sqlite>,
    s: &OrganizationSchedule,
) -> Result<bool> {
    let node = read_node(tx, &s.template.owner_id).await?;
    let project = read_project(tx, &s.template.project_id).await?;
    Ok(s.policy_version == s.scope.policy_version()
        && node.revision == s.node_revision
        && project.revision == s.project_revision
        && ancestors_enabled(tx, &node.id).await?)
}

async fn read_schedule(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<OrganizationSchedule> {
    let row: String = sqlx::query_scalar("SELECT data FROM organization_schedules WHERE id=?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(serde_json::from_str(&row)?)
}

async fn write_schedule(
    tx: &mut Transaction<'_, Sqlite>,
    s: &mut OrganizationSchedule,
) -> Result<()> {
    s.revision = s
        .revision
        .checked_add(1)
        .context("schedule revision exhausted")?;
    sqlx::query("UPDATE organization_schedules SET state=?,next_run_at=?,data=? WHERE id=?")
        .bind(s.state.as_str())
        .bind(s.next_run_at)
        .bind(serde_json::to_string(s)?)
        .bind(&s.id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn usage(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<(i64, i64)> {
    Ok(sqlx::query_as("SELECT COALESCE(sum(a.used_tokens),0),COALESCE(sum(a.model_requests),0) FROM organization_schedule_occurrences o JOIN organization_authorizations a ON a.id=o.authorization_id WHERE o.schedule_id=?")
        .bind(id).fetch_one(&mut **tx).await?)
}

async fn view_in(
    tx: &mut Transaction<'_, Sqlite>,
    schedule: OrganizationSchedule,
) -> Result<OrganizationScheduleView> {
    let (used_tokens, requests) = usage(tx, &schedule.id).await?;
    let rows: Vec<(String, String, i64, String)> = sqlx::query_as("SELECT o.task_id,o.authorization_id,o.due_at,t.state FROM organization_schedule_occurrences o JOIN organization_tasks t ON t.id=o.task_id WHERE o.schedule_id=? ORDER BY o.due_at DESC LIMIT 16")
        .bind(&schedule.id).fetch_all(&mut **tx).await?;
    Ok(OrganizationScheduleView {
        schedule,
        used_tokens,
        model_requests: u32::try_from(requests)?,
        occurrences: rows
            .into_iter()
            .map(
                |(task_id, authorization_id, due_at, task_state)| OrganizationScheduleOccurrence {
                    task_id,
                    authorization_id,
                    due_at,
                    task_state,
                },
            )
            .collect(),
    })
}

async fn uncertain(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_attempts p JOIN organization_authorizations a ON a.id=p.authorization_id JOIN organization_schedule_occurrences o ON o.authorization_id=a.root_id WHERE o.schedule_id=? AND (p.state='needsReconciliation' OR EXISTS(SELECT 1 FROM organization_actions x WHERE x.attempt_id=p.id AND x.state='uncertain') OR (p.state NOT IN ('running','waitingApproval','waitingDependency') AND (json_extract(p.data,'$.modelRequests')<>json_extract(p.data,'$.reportedModelRequests') OR EXISTS(SELECT 1 FROM organization_actions x WHERE x.attempt_id=p.id AND x.state='dispatched'))))")
        .bind(id).fetch_one(&mut **tx).await?;
    Ok(count > 0)
}

async fn busy(tx: &mut Transaction<'_, Sqlite>, id: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_authorizations a JOIN organization_schedule_occurrences o ON o.authorization_id=a.root_id JOIN organization_tasks t ON t.authorization_id=a.id WHERE o.schedule_id=? AND (t.state IN ('queued','running','waitingApproval','waitingDependency','needsReconciliation') OR EXISTS(SELECT 1 FROM organization_attempts p WHERE p.authorization_id=a.id AND p.occupied=1))")
        .bind(id).fetch_one(&mut **tx).await?;
    Ok(count > 0)
}

pub(super) async fn reject_occurrence_reauthorization(
    tx: &mut Transaction<'_, Sqlite>,
    task: &str,
) -> Result<()> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_authorizations a JOIN organization_schedule_occurrences o ON o.authorization_id=a.root_id WHERE a.task_id=?")
        .bind(task).fetch_one(&mut **tx).await?;
    ensure!(
        count == 0,
        "scheduled occurrences cannot be reauthorized; review recovery and create a new task or series"
    );
    Ok(())
}

pub(super) async fn check_schedule_fence(
    tx: &mut Transaction<'_, Sqlite>,
    grant: &OrganizationTaskAuthorization,
    now: i64,
    admission: bool,
) -> Result<()> {
    let row: Option<String> = sqlx::query_scalar("SELECT s.data FROM organization_schedules s JOIN organization_schedule_occurrences o ON o.schedule_id=s.id WHERE o.authorization_id=?")
        .bind(grant.root_authorization_id.as_ref().unwrap_or(&grant.id)).fetch_optional(&mut **tx).await?;
    if let Some(row) = row {
        let schedule: OrganizationSchedule = serde_json::from_str(&row)?;
        ensure!(
            matches!(
                schedule.state,
                OrganizationScheduleState::Active | OrganizationScheduleState::Completed
            ) && now < schedule.config.ends_at,
            "schedule is paused, cancelled, blocked or expired"
        );
        let (used, requests) = usage(tx, &schedule.id).await?;
        ensure!(
            used < schedule.config.max_tokens,
            "schedule aggregate token ceiling exhausted"
        );
        if admission {
            ensure!(
                requests < i64::from(schedule.config.max_model_requests),
                "schedule aggregate request ceiling exhausted"
            );
        }
    }
    Ok(())
}

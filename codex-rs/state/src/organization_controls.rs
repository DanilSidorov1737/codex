//! Owner controls retain immutable versions and project existing accounting.
use super::definitions::text;
use super::execution::*;
use super::resource::validate_shared_ceilings;
use anyhow::{Context, Result, ensure};
use codex_protocol::organization::*;
use codex_protocol::organization_execution::OrganizationTaskAuthorization;
use codex_protocol::organization_workflow::{
    OrganizationWorkflowSeries, OrganizationWorkflowState,
};
use sha2::Digest;
use sqlx::{Sqlite, Transaction};
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) async fn read(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<Option<OrganizationControl>> {
    let data: Option<String> =
        sqlx::query_scalar("SELECT data FROM organization_controls WHERE id=?")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await?;
    data.map(|data| {
        let OrganizationRecord::Control(control) = serde_json::from_str(&data)? else {
            anyhow::bail!("invalid stored control");
        };
        Ok(control)
    })
    .transpose()
}

pub(super) async fn set(
    tx: &mut Transaction<'_, Sqlite>,
    expected_revision: Option<i64>,
    value: OrganizationControlValue,
) -> Result<OrganizationControl> {
    let id = match &value {
        OrganizationControlValue::Knowledge { note_id, .. } => format!("knowledge:{note_id}"),
        OrganizationControlValue::Replan { id, .. } => format!("replan:{id}"),
        OrganizationControlValue::Supervision { series_id, .. } => {
            format!("supervision:{series_id}")
        }
        OrganizationControlValue::Global { .. } => "global".into(),
        OrganizationControlValue::Project { project_id, .. } => format!("project:{project_id}"),
        OrganizationControlValue::Priority { task_id, .. } => format!("priority:{task_id}"),
    };
    let previous = read(tx, &id).await?;
    ensure!(
        previous.as_ref().map(|c| c.revision) == expected_revision,
        "control revision conflict"
    );
    match &value {
        OrganizationControlValue::Knowledge {
            note_id,
            body_sha256,
            reason,
            ..
        } => {
            text(reason, 4096)?;
            let data: String = sqlx::query_scalar("SELECT data FROM organization_notes WHERE id=?")
                .bind(note_id)
                .fetch_one(&mut **tx)
                .await?;
            let note: codex_protocol::organization_coordination::OrganizationNote =
                serde_json::from_str(&data)?;
            ensure!(
                *body_sha256 == format!("{:x}", sha2::Sha256::digest(note.body.as_bytes())),
                "knowledge source changed; inspect its exact body before review"
            );
        }
        OrganizationControlValue::Replan {
            id,
            project_id,
            reason,
            items,
            applied_handoff_ids,
        } => {
            Uuid::parse_str(id)?;
            text(reason, 4096)?;
            ensure!(
                (1..=32).contains(&items.len()),
                "replans contain 1–32 checkpointed tasks"
            );
            let mut ids = BTreeSet::new();
            if let Some(old) = &previous {
                let OrganizationControlValue::Replan {
                    project_id: old_project,
                    applied_handoff_ids: old_handoffs,
                    ..
                } = &old.value
                else {
                    anyhow::bail!("invalid stored replan")
                };
                ensure!(
                    old_project == project_id && old_handoffs.is_empty(),
                    "applied replans are immutable"
                );
            }
            if applied_handoff_ids.is_empty() {
                check_project_edit(tx, project_id).await?;
                for item in items {
                    ensure!(ids.insert(&item.task_id), "duplicate replan task");
                    text(&item.instructions, 4096)?;
                    let task = read_task(tx, &item.task_id).await?;
                    ensure!(
                        task.project_id == *project_id
                            && task.revision == item.expected_revision
                            && task.state == "blocked",
                        "replan requires exact checkpointed blocked work in this project"
                    );
                    let latest:Option<String>=sqlx::query_scalar("SELECT id FROM organization_checkpoints WHERE task_id=? ORDER BY id DESC LIMIT 1")
                        .bind(&item.task_id).fetch_optional(&mut **tx).await?;
                    ensure!(
                        latest.as_ref() == Some(&item.checkpoint_id),
                        "replan checkpoint changed"
                    );
                    ensure!(
                        read_node(tx, &item.owner_id).await?.revision
                            == item.expected_owner_revision,
                        "replan receiver changed"
                    );
                }
            } else {
                let old = previous
                    .as_ref()
                    .context("save the pending proposal before applying a replan")?;
                let mut pending = value.clone();
                if let OrganizationControlValue::Replan {
                    applied_handoff_ids,
                    ..
                } = &mut pending
                {
                    applied_handoff_ids.clear();
                }
                ensure!(
                    old.value == pending && applied_handoff_ids.len() == items.len(),
                    "applied replan must preserve the exact pending proposal"
                );
                for (item, handoff_id) in items.iter().zip(applied_handoff_ids) {
                    ensure!(ids.insert(handoff_id), "duplicate replan handoff receipt");
                    let data:String=sqlx::query_scalar("SELECT data FROM organization_handoffs WHERE id=? AND task_id=? AND checkpoint_id=?")
                        .bind(handoff_id).bind(&item.task_id).bind(&item.checkpoint_id).fetch_one(&mut **tx).await?;
                    let handoff: codex_protocol::organization_execution::OrganizationTaskHandoff =
                        serde_json::from_str(&data)?;
                    ensure!(
                        handoff.to_owner_id == item.owner_id
                            && handoff.to_node_revision == item.expected_owner_revision
                            && handoff.instructions == item.instructions,
                        "replan receipt does not match its owner-reviewed delta"
                    );
                }
            }
        }
        OrganizationControlValue::Supervision {
            series_id,
            goal,
            work_type,
            state,
        } => {
            text(work_type, 256)?;
            let data: String =
                sqlx::query_scalar("SELECT data FROM organization_workflow_series WHERE id=?")
                    .bind(series_id)
                    .fetch_one(&mut **tx)
                    .await?;
            let series: OrganizationWorkflowSeries = serde_json::from_str(&data)?;
            let expected_state = match series.state {
                codex_protocol::organization_schedule::OrganizationScheduleState::Active => {
                    OrganizationWorkflowState::Active
                }
                codex_protocol::organization_schedule::OrganizationScheduleState::Paused => {
                    OrganizationWorkflowState::Paused
                }
                codex_protocol::organization_schedule::OrganizationScheduleState::Cancelled => {
                    OrganizationWorkflowState::Cancelled
                }
                _ => anyhow::bail!(
                    "control the blocked/completed finite series before changing its supervision policy"
                ),
            };
            ensure!(
                *state == expected_state,
                "use the native supervision control to change execution state"
            );
            if let Some(old) = previous.as_ref() {
                let OrganizationControlValue::Supervision {
                    goal: old_goal,
                    work_type: old_type,
                    state: old_state,
                    ..
                } = &old.value
                else {
                    anyhow::bail!("invalid stored supervision control");
                };
                ensure!(
                    old_goal == goal
                        && old_type == work_type
                        && *old_state != OrganizationWorkflowState::Cancelled,
                    "policy content is immutable; cancel and review a fresh finite policy without resetting prior usage"
                );
            } else {
                ensure!(
                    *state == OrganizationWorkflowState::Active,
                    "new supervision must bind an active finite series"
                );
            }
            if *state == OrganizationWorkflowState::Active {
                check_goal(tx, goal).await?;
                ensure!(
                    read_project(tx, &series.preview.template.project_id)
                        .await?
                        .goal_id
                        .as_ref()
                        == Some(&goal.id),
                    "supervision goal must be linked to the captured project"
                );
            }
        }
        OrganizationControlValue::Global { limits, .. } => {
            if let Some(limits) = limits {
                validate_shared_ceilings(
                    limits.max_tokens,
                    limits.max_model_requests,
                    limits.max_active,
                )?;
                let active: i64 =
                    sqlx::query_scalar("SELECT active FROM organization_global_usage")
                        .fetch_one(&mut **tx)
                        .await?;
                ensure!(
                    active <= i64::from(limits.max_active),
                    "pause active work before lowering global concurrency below usage"
                );
            }
        }
        OrganizationControlValue::Priority {
            task_id,
            task_revision,
            priority,
        } => {
            ensure!(
                (-10..=10).contains(priority),
                "priority must be between -10 and 10"
            );
            let task = read_task(tx, task_id).await?;
            ensure!(
                task.revision == *task_revision,
                "priority task revision conflict"
            );
            ensure!(
                !matches!(task.state.as_str(), "accepted" | "cancelled"),
                "finished task priority is immutable"
            );
        }
        OrganizationControlValue::Project {
            project_id,
            project_revision,
            state,
            evidence,
        } => {
            let project = read_project(tx, project_id).await?;
            ensure!(
                project.revision == *project_revision,
                "project definition revision conflict"
            );
            let old = match previous.as_ref().map(|c| &c.value) {
                Some(OrganizationControlValue::Project { state, .. }) => *state,
                None => OrganizationProjectState::Active,
                _ => anyhow::bail!("invalid stored project control"),
            };
            ensure!(old != *state, "project is already in the requested state");
            ensure!(
                matches!(
                    (old, state),
                    (_, OrganizationProjectState::Active)
                        | (
                            OrganizationProjectState::Active,
                            OrganizationProjectState::Paused
                        )
                        | (
                            OrganizationProjectState::Active | OrganizationProjectState::Paused,
                            OrganizationProjectState::Completed
                        )
                        | (
                            OrganizationProjectState::Paused | OrganizationProjectState::Completed,
                            OrganizationProjectState::Archived
                        )
                ),
                "pause or complete before archiving; reopen before other changes"
            );
            if *state == OrganizationProjectState::Completed {
                ensure!(
                    !project.criteria.is_empty() && evidence.len() == project.criteria.len(),
                    "completion requires evidence for every project criterion"
                );
                let unfinished: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_tasks WHERE project_id=? AND state NOT IN ('accepted','cancelled')")
                    .bind(project_id).fetch_one(&mut **tx).await?;
                ensure!(
                    unfinished == 0,
                    "project has unfinished work; review or cancel it first"
                );
                let mut criteria = BTreeSet::new();
                for proof in evidence {
                    ensure!(
                        (proof.criterion as usize) < project.criteria.len()
                            && criteria.insert(proof.criterion),
                        "invalid or duplicate completion criterion"
                    );
                    text(&proof.explanation, 4096)?;
                    let accepted: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_tasks t JOIN organization_attempts a ON a.task_id=t.id AND a.authorization_id=t.authorization_id JOIN organization_results r ON r.attempt_id=a.id WHERE t.id=? AND t.project_id=? AND t.state='accepted' AND a.state='accepted' AND json_extract(r.data,'$.accepted')=1 AND r.digest=?")
                        .bind(&proof.task_id).bind(project_id).bind(&proof.result_digest).fetch_one(&mut **tx).await?;
                    ensure!(
                        accepted == 1,
                        "completion evidence must be an exact current accepted result in this project"
                    );
                }
            } else {
                ensure!(
                    evidence.is_empty(),
                    "completion evidence is only supplied when completing; prior evidence remains in history"
                );
            }
        }
    }
    let control = OrganizationControl {
        id,
        revision: previous
            .map_or(0, |c| c.revision)
            .checked_add(1)
            .context("control revision exhausted")?,
        value,
    };
    sqlx::query("INSERT INTO organization_controls(id,revision,data) VALUES (?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,data=excluded.data")
        .bind(&control.id).bind(control.revision).bind(serde_json::to_string(&OrganizationRecord::Control(control.clone()))?)
        .execute(&mut **tx).await?;
    let history = OrganizationControlHistory {
        id: Uuid::now_v7().to_string(),
        control: control.clone(),
    };
    sqlx::query(
        "INSERT INTO organization_control_history(id,control_id,revision,data) VALUES (?,?,?,?)",
    )
    .bind(&history.id)
    .bind(&control.id)
    .bind(control.revision)
    .bind(serde_json::to_string(&OrganizationRecord::ControlHistory(
        history,
    ))?)
    .execute(&mut **tx)
    .await?;
    Ok(control)
}

pub(super) async fn check_project_edit(
    tx: &mut Transaction<'_, Sqlite>,
    project_id: &str,
) -> Result<()> {
    if let Some(control) = read(tx, &format!("project:{project_id}")).await? {
        ensure!(
            matches!(
                control.value,
                OrganizationControlValue::Project {
                    state: OrganizationProjectState::Active | OrganizationProjectState::Paused,
                    ..
                }
            ),
            "reopen the project before changing definitions"
        );
    }
    Ok(())
}

pub(super) async fn check_goal(
    tx: &mut Transaction<'_, Sqlite>,
    goal: &OrganizationGoal,
) -> Result<()> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_goals WHERE id=?")
        .bind(&goal.id)
        .fetch_one(&mut **tx)
        .await?;
    ensure!(
        serde_json::from_str::<OrganizationRecord>(&data)?
            == OrganizationRecord::Goal(goal.clone()),
        "supervision goal changed; review a new finite policy"
    );
    Ok(())
}

pub(super) async fn check_supervision(
    tx: &mut Transaction<'_, Sqlite>,
    series_id: &str,
) -> Result<()> {
    if let Some(policy) = read(tx, &format!("supervision:{series_id}")).await? {
        let OrganizationControlValue::Supervision { goal, state, .. } = policy.value else {
            anyhow::bail!("invalid stored supervision policy");
        };
        ensure!(
            state == OrganizationWorkflowState::Active,
            "supervision policy is paused or revoked"
        );
        check_goal(tx, &goal).await?;
    }
    Ok(())
}

pub(super) async fn check_dispatch(
    tx: &mut Transaction<'_, Sqlite>,
    grant: &OrganizationTaskAuthorization,
) -> Result<()> {
    if let Some(control) = read(tx, "global").await? {
        ensure!(
            !matches!(
                control.value,
                OrganizationControlValue::Global { paused: true, .. }
            ),
            super::WorkflowPaused
        );
    }
    let project_id: String =
        sqlx::query_scalar("SELECT project_id FROM organization_tasks WHERE id=?")
            .bind(&grant.task_id)
            .fetch_one(&mut **tx)
            .await?;
    if let Some(control) = read(tx, &format!("project:{project_id}")).await? {
        ensure!(
            matches!(
                control.value,
                OrganizationControlValue::Project {
                    state: OrganizationProjectState::Active,
                    ..
                }
            ),
            super::WorkflowPaused
        );
    }
    let policy_data: Option<String> = sqlx::query_scalar("SELECT c.data FROM organization_controls c JOIN organization_workflow_occurrences o ON c.id='supervision:'||o.series_id JOIN organization_workflow_tasks w ON w.run_id=o.run_id WHERE w.authorization_id=? LIMIT 1")
        .bind(grant.root_authorization_id.as_ref().unwrap_or(&grant.id)).fetch_optional(&mut **tx).await?;
    if let Some(data) = policy_data {
        let OrganizationRecord::Control(OrganizationControl {
            value: OrganizationControlValue::Supervision { goal, state, .. },
            ..
        }) = serde_json::from_str(&data)?
        else {
            anyhow::bail!("invalid stored supervision policy");
        };
        ensure!(
            state == OrganizationWorkflowState::Active && check_goal(tx, &goal).await.is_ok(),
            super::WorkflowPaused
        );
    }
    Ok(())
}

/// Unknown terminal usage stays charged and fences the configured global pool.
pub(super) async fn check_global(
    tx: &mut Transaction<'_, Sqlite>,
    inference: bool,
    capacity: bool,
) -> Result<bool> {
    let Some(control) = read(tx, "global").await? else {
        return Ok(true);
    };
    let OrganizationControlValue::Global { paused, limits } = control.value else {
        anyhow::bail!("invalid stored global control");
    };
    if paused && capacity {
        return Ok(false);
    }
    let Some(limits) = limits else {
        return Ok(true);
    };
    let (tokens, requests, active, unresolved): (i64, i64, i64, i64) = sqlx::query_as("SELECT reported_tokens,model_requests,active,unresolved_requests FROM organization_global_usage")
        .fetch_one(&mut **tx).await?;
    Ok(unresolved == 0
        && tokens < limits.max_tokens
        && (!inference || requests < i64::from(limits.max_model_requests))
        && (!capacity || active < i64::from(limits.max_active)))
}

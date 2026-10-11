//! Owner controls retain immutable versions and project existing accounting.
use super::definitions::text;
use super::execution::*;
use super::resource::validate_shared_ceilings;
use anyhow::{Context, Result, ensure};
use codex_protocol::organization::*;
use codex_protocol::organization_execution::OrganizationTaskAuthorization;
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

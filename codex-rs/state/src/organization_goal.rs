//! Goal responsibility/dependencies and business dates do not grant data/tool access.
use super::controls::check_goal;
use super::controls::read;
use super::execution::read_node;
use super::execution::read_project;
use super::execution::read_task;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization::*;
use sqlx::Sqlite;
use sqlx::Transaction;
use std::collections::BTreeSet;

pub(super) async fn descendant(
    tx: &mut Transaction<'_, Sqlite>,
    candidate: Option<&str>,
    ancestor: &str,
) -> Result<bool> {
    let found:i64=sqlx::query_scalar("WITH RECURSIVE ancestors(id) AS (SELECT ? UNION SELECT g.parent_id FROM organization_goals g JOIN ancestors a ON a.id=g.id WHERE g.parent_id IS NOT NULL) SELECT EXISTS(SELECT 1 FROM ancestors WHERE id=?)").bind(candidate).bind(ancestor).fetch_one(&mut **tx).await?;
    Ok(found == 1)
}

pub(super) async fn required_path(
    tx: &mut Transaction<'_, Sqlite>,
    start: &str,
    target: &str,
) -> Result<bool> {
    let cycle:i64=sqlx::query_scalar("WITH RECURSIVE required(id) AS (SELECT ? UNION SELECT g.id FROM organization_goals g JOIN required r ON g.parent_id=r.id UNION SELECT json_extract(j.value,'$.goal.id') FROM organization_controls c JOIN required r ON c.id='goal-meta:'||r.id JOIN json_each(c.data,'$.data.value.prerequisites') j) SELECT EXISTS(SELECT 1 FROM required WHERE id=?)")
        .bind(start).bind(target).fetch_one(&mut **tx).await?;
    Ok(cycle == 1)
}

pub(super) async fn validate_metadata(
    tx: &mut Transaction<'_, Sqlite>,
    goal: &OrganizationGoal,
    project_id: &str,
    responsible: &str,
    prerequisites: &[OrganizationGoalDependency],
) -> Result<()> {
    check_goal(tx, goal).await?;
    read_project(tx, project_id).await?;
    read_node(tx, responsible).await?;
    ensure!(prerequisites.len() <= 64, "goal prerequisite limit is 64");
    let mut unique = BTreeSet::new();
    for prerequisite in prerequisites {
        ensure!(
            prerequisite.goal.id != goal.id && unique.insert(&prerequisite.goal.id),
            "duplicate/self goal prerequisite"
        );
        check_goal(tx, &prerequisite.goal).await?;
        read_project(tx, &prerequisite.project_id).await?;
        ensure!(
            prerequisite.project_id == project_id || prerequisite.cross_project_approved,
            "cross-project dependency needs explicit owner metadata approval; it grants no data access"
        );
        if let Some(metadata) = read(tx, &format!("goal-meta:{}", prerequisite.goal.id)).await? {
            ensure!(
                matches!(metadata.value,OrganizationControlValue::GoalMetadata {project_id:ref actual,..} if actual==&prerequisite.project_id),
                "prerequisite project attribution changed"
            );
        } else {
            let project = read_project(tx, &prerequisite.project_id).await?;
            ensure!(
                descendant(
                    tx,
                    Some(&prerequisite.goal.id),
                    project.goal_id.as_deref().unwrap_or("")
                )
                .await?,
                "bind a prerequisite goal to its project before referencing it"
            );
        }
        // Parent completion depends on descendants too; child→parent prerequisites
        // would deadlock even without an explicit dependency back edge.
        ensure!(
            !required_path(tx, &prerequisite.goal.id, &goal.id).await?,
            "goal dependency/hierarchy cycle rejected"
        );
    }
    Ok(())
}

pub(super) async fn check_prerequisites(
    tx: &mut Transaction<'_, Sqlite>,
    goal: &OrganizationGoal,
) -> Result<()> {
    let Some(metadata) = read(tx, &format!("goal-meta:{}", goal.id)).await? else {
        return Ok(());
    };
    let OrganizationControlValue::GoalMetadata {
        goal: captured,
        prerequisites,
        ..
    } = metadata.value
    else {
        anyhow::bail!("invalid goal metadata")
    };
    ensure!(*captured == *goal, super::WorkflowPaused);
    for prerequisite in prerequisites {
        if let Some(metadata) = read(tx, &format!("goal-meta:{}", prerequisite.goal.id)).await? {
            ensure!(
                matches!(metadata.value,OrganizationControlValue::GoalMetadata {project_id:ref actual,..} if actual==&prerequisite.project_id),
                super::WorkflowPaused
            );
        }
        ensure!(
            check_goal(tx, &prerequisite.goal).await.is_ok(),
            super::WorkflowPaused
        );
        let complete = read(tx, &format!("goal:{}", prerequisite.goal.id)).await?;
        ensure!(complete.is_some_and(|c|matches!(c.value,OrganizationControlValue::GoalReview {goal:ref reviewed,completed:true,..} if **reviewed==*prerequisite.goal)),super::WorkflowPaused);
    }
    Ok(())
}

pub(super) fn deadline_kind(kind: OrganizationDeadlineKind) -> &'static str {
    match kind {
        OrganizationDeadlineKind::Goal => "goal",
        OrganizationDeadlineKind::Project => "project",
        OrganizationDeadlineKind::Task => "task",
    }
}

pub(super) async fn validate_deadline(
    tx: &mut Transaction<'_, Sqlite>,
    kind: OrganizationDeadlineKind,
    id: &str,
    revision: i64,
    due_at: Option<i64>,
    offset: i32,
) -> Result<()> {
    let actual = match kind {
        OrganizationDeadlineKind::Goal => {
            sqlx::query_scalar("SELECT revision FROM organization_goals WHERE id=?")
                .bind(id)
                .fetch_one(&mut **tx)
                .await?
        }
        OrganizationDeadlineKind::Project => read_project(tx, id).await?.revision,
        OrganizationDeadlineKind::Task => read_task(tx, id).await?.revision,
    };
    ensure!(revision == actual, "deadline target revision changed");
    ensure!(
        (-840..=840).contains(&offset),
        "timezone offset must be within ±14 hours"
    );
    ensure!(
        due_at.is_none_or(
            |seconds| seconds >= 0 && chrono::DateTime::from_timestamp(seconds, 0).is_some()
        ),
        "invalid business due date"
    );
    Ok(())
}

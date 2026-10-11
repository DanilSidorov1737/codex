//! Shared ceilings project existing root ledgers; descendants are never summed twice.
use super::OrganizationStore;
use super::execution::*;
use anyhow::{Context, Result, ensure};
use codex_protocol::organization_execution::OrganizationTaskAuthorization;
use codex_protocol::organization_workflow::*;
use sqlx::{Sqlite, Transaction};

impl OrganizationStore {
    pub(super) async fn shared_limits_in(
        tx: &mut Transaction<'_, Sqlite>,
        project_id: String,
        run_id: Option<String>,
        expected_revision: Option<i64>,
        max_tokens: i64,
        max_model_requests: u32,
        max_active: u32,
    ) -> Result<OrganizationWorkflowView> {
        read_project(tx, &project_id).await?;
        validate_shared_ceilings(max_tokens, max_model_requests, max_active)?;
        let id = if let Some(run_id) = &run_id {
            let project: String =
                sqlx::query_scalar("SELECT project_id FROM organization_workflow_runs WHERE id=?")
                    .bind(run_id)
                    .fetch_one(&mut **tx)
                    .await?;
            ensure!(project == project_id, "workflow limit project mismatch");
            format!("run:{run_id}")
        } else {
            format!("project:{project_id}")
        };
        let previous: Option<i64> =
            sqlx::query_scalar("SELECT revision FROM organization_shared_limits WHERE id=?")
                .bind(&id)
                .fetch_optional(&mut **tx)
                .await?;
        ensure!(
            previous == expected_revision,
            "shared limit revision conflict"
        );
        let limit = OrganizationSharedLimit {
            id,
            project_id,
            run_id,
            revision: previous
                .unwrap_or(0)
                .checked_add(1)
                .context("limit revision exhausted")?,
            max_tokens,
            max_model_requests,
            max_active,
        };
        sqlx::query("INSERT INTO organization_shared_limits(id,project_id,run_id,revision,data) VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,data=excluded.data")
            .bind(&limit.id).bind(&limit.project_id).bind(&limit.run_id).bind(limit.revision).bind(serde_json::to_string(&limit)?).execute(&mut **tx).await?;
        let usage = shared_usage(tx, &limit).await?;
        ensure!(
            usage.active <= i64::from(limit.max_active),
            "pause active work before lowering concurrency below its current usage"
        );
        owner_event(tx, "resources/setLimits", &limit.id).await?;
        Ok(OrganizationWorkflowView::Limit { limit, usage })
    }
}

pub(super) fn validate_shared_ceilings(
    max_tokens: i64,
    max_model_requests: u32,
    max_active: u32,
) -> Result<()> {
    ensure!(
        max_tokens > 0
            && max_tokens <= 10_000_000
            && (1..=100_000).contains(&max_model_requests)
            && (1..=4).contains(&max_active),
        "invalid shared resource ceilings"
    );
    Ok(())
}

pub(super) async fn shared_usage(
    tx: &mut Transaction<'_, Sqlite>,
    limit: &OrganizationSharedLimit,
) -> Result<OrganizationSharedUsage> {
    let (reported_tokens, model_requests, active): (i64, i64, i64) = sqlx::query_as(
        "SELECT reported_tokens,model_requests,active FROM organization_shared_usage WHERE id=?",
    )
    .bind(&limit.id)
    .fetch_optional(&mut **tx)
    .await?
    .unwrap_or_default();
    Ok(OrganizationSharedUsage {
        reported_tokens,
        model_requests,
        active,
    })
}

async fn applicable_limits(
    tx: &mut Transaction<'_, Sqlite>,
    grant: &OrganizationTaskAuthorization,
) -> Result<Vec<OrganizationSharedLimit>> {
    let data: Vec<String> = sqlx::query_scalar("SELECT l.data FROM organization_shared_limits l JOIN organization_tasks t ON t.project_id=l.project_id WHERE t.id=? AND (l.run_id IS NULL OR EXISTS(SELECT 1 FROM organization_workflow_tasks w WHERE w.run_id=l.run_id AND w.authorization_id=?))")
        .bind(&grant.task_id).bind(grant.root_authorization_id.as_ref().unwrap_or(&grant.id)).fetch_all(&mut **tx).await?;
    data.into_iter()
        .map(|data| Ok(serde_json::from_str(&data)?))
        .collect()
}

pub(super) async fn check_shared_ceilings(
    tx: &mut Transaction<'_, Sqlite>,
    grant: &OrganizationTaskAuthorization,
    inference: bool,
) -> Result<()> {
    for limit in applicable_limits(tx, grant).await? {
        let unresolved: i64 = sqlx::query_scalar(
            "SELECT unresolved_requests FROM organization_shared_usage WHERE id=?",
        )
        .bind(&limit.id)
        .fetch_one(&mut **tx)
        .await?;
        ensure!(
            unresolved == 0,
            "shared usage remains unresolved; reconcile before further work: {}",
            limit.id
        );
        let usage = shared_usage(tx, &limit).await?;
        ensure!(
            usage.reported_tokens < limit.max_tokens,
            "shared reported token ceiling exhausted: {}",
            limit.id
        );
        ensure!(
            !inference || usage.model_requests < i64::from(limit.max_model_requests),
            "shared model request ceiling exhausted: {}",
            limit.id
        );
    }
    Ok(())
}

pub(super) async fn shared_capacity_available(
    tx: &mut Transaction<'_, Sqlite>,
    grant: &OrganizationTaskAuthorization,
) -> Result<bool> {
    for limit in applicable_limits(tx, grant).await? {
        if shared_usage(tx, &limit).await?.active >= i64::from(limit.max_active) {
            return Ok(false);
        }
    }
    Ok(true)
}

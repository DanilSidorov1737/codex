//! Shared ceilings project existing root ledgers; descendants are never summed twice.
use super::OrganizationStore;
use super::execution::*;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization_execution::OrganizationTaskAuthorization;
use codex_protocol::organization_workflow::*;
use sqlx::Sqlite;
use sqlx::Transaction;

/// Actual local sample. Missing required telemetry fails closed.
pub(super) async fn pressure_ready(tx: &mut Transaction<'_, Sqlite>) -> Result<bool> {
    Ok(pressure_observation(tx, chrono::Utc::now().timestamp())
        .await?
        .is_none_or(|v| v.ready))
}

pub(super) async fn pressure_observation(
    tx: &mut Transaction<'_, Sqlite>,
    now: i64,
) -> Result<Option<codex_protocol::organization_report::OrganizationResourcePressure>> {
    let Some(control) = super::controls::read(tx, "pressure").await? else {
        return Ok(None);
    };
    let codex_protocol::organization::OrganizationControlValue::ResourcePressure {
        path,
        min_disk_bytes,
        min_available_memory_bytes,
    } = control.value
    else {
        anyhow::bail!("invalid pressure control")
    };
    let (disk, memory) = pressure_sample(std::path::Path::new(&path));
    let ready = (min_disk_bytes == 0 || disk.is_some_and(|v| v >= min_disk_bytes))
        && (min_available_memory_bytes == 0
            || memory.is_some_and(|v| v >= min_available_memory_bytes));
    Ok(Some(
        codex_protocol::organization_report::OrganizationResourcePressure {
            path,
            measured_at: now,
            available_disk_bytes: disk,
            available_memory_bytes: memory,
            min_disk_bytes,
            min_available_memory_bytes,
            ready,
        },
    ))
}

pub fn pressure_sample(path: &std::path::Path) -> (Option<u64>, Option<u64>) {
    let mut disk = None;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        if let Ok(path) = std::ffi::CString::new(path.as_os_str().as_bytes()) {
            let mut info = std::mem::MaybeUninit::<libc::statvfs>::uninit();
            // statvfs initializes the structure only on success.
            if unsafe { libc::statvfs(path.as_ptr(), info.as_mut_ptr()) } == 0 {
                let info = unsafe { info.assume_init() };
                disk = u64::try_from((info.f_bavail as u128) * (info.f_frsize as u128)).ok();
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let memory = None;
    #[cfg(target_os = "linux")]
    let memory = {
        let mut memory = std::fs::read_to_string("/proc/meminfo").ok().and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("MemAvailable:"))
                .and_then(|l| l.split_whitespace().nth(1))
                .and_then(|v| v.parse::<u64>().ok())
                .and_then(|v| v.checked_mul(1024))
        });
        if let (Ok(maximum), Ok(current)) = (
            std::fs::read_to_string("/sys/fs/cgroup/memory.max"),
            std::fs::read_to_string("/sys/fs/cgroup/memory.current"),
        ) && let (Ok(maximum), Ok(current)) =
            (maximum.trim().parse::<u64>(), current.trim().parse::<u64>())
        {
            let available = maximum.saturating_sub(current);
            memory = Some(memory.map_or(available, |host| host.min(available)));
        }
        memory
    };
    (disk, memory)
}

impl OrganizationStore {
    /// Reserve a nominal request slice in the same transaction as admission.
    /// Provider output may exceed it; unknown requests never release this slice.
    pub(super) async fn reserve_tokens(
        tx: &mut Transaction<'_, Sqlite>,
        attempt: &codex_protocol::organization_execution::OrganizationTaskAttempt,
        grant: &OrganizationTaskAuthorization,
    ) -> Result<()> {
        let root_id = grant.root_authorization_id.as_ref().unwrap_or(&grant.id);
        let (data, used): (String, i64) =
            sqlx::query_as("SELECT data,used_tokens FROM organization_authorizations WHERE id=?")
                .bind(root_id)
                .fetch_one(&mut **tx)
                .await?;
        let root: OrganizationTaskAuthorization = serde_json::from_str(&data)?;
        let reserved: i64 = sqlx::query_scalar("SELECT COALESCE(sum(tokens),0) FROM organization_token_reservations WHERE root_id=? AND released=0")
            .bind(root_id).fetch_one(&mut **tx).await?;
        let remaining = root
            .scope
            .max_tokens
            .checked_sub(used)
            .and_then(|v| v.checked_sub(reserved))
            .context("token allowance overflow")?;
        ensure!(remaining > 0, "token allowance is exhausted or reserved");
        let slice = (root.scope.max_tokens + i64::from(root.scope.max_model_requests) - 1)
            / i64::from(root.scope.max_model_requests);
        let mut tokens = slice.min(remaining);
        let child_used: i64 =
            sqlx::query_scalar("SELECT used_tokens FROM organization_authorizations WHERE id=?")
                .bind(&grant.id)
                .fetch_one(&mut **tx)
                .await?;
        tokens = tokens.min(
            grant
                .scope
                .max_tokens
                .checked_sub(child_used)
                .context("child token allowance overflow")?,
        );
        if let Some(control) = super::controls::read(tx, "global").await?
            && let codex_protocol::organization::OrganizationControlValue::Global {
                limits: Some(limits),
                ..
            } = control.value
        {
            let used: i64 =
                sqlx::query_scalar("SELECT reported_tokens FROM organization_global_usage")
                    .fetch_one(&mut **tx)
                    .await?;
            let reserved: i64 = sqlx::query_scalar("SELECT COALESCE(sum(tokens),0) FROM organization_token_reservations WHERE released=0").fetch_one(&mut **tx).await?;
            tokens = tokens.min(
                limits
                    .max_tokens
                    .checked_sub(used)
                    .and_then(|v| v.checked_sub(reserved))
                    .context("global token allowance overflow")?,
            );
        }
        for limit in applicable_limits(tx, grant).await? {
            let reserved: i64 = sqlx::query_scalar("SELECT COALESCE(sum(r.tokens),0) FROM organization_token_reservations r JOIN organization_authorizations a ON a.id=r.root_id JOIN organization_tasks t ON t.id=a.task_id WHERE r.released=0 AND t.project_id=? AND ((? IS NULL AND ? IS NULL) OR EXISTS(SELECT 1 FROM organization_workflow_tasks w LEFT JOIN organization_workflow_occurrences o ON o.run_id=w.run_id WHERE w.authorization_id=r.root_id AND (w.run_id=? OR o.series_id=?)))")
                .bind(&limit.project_id).bind(&limit.run_id).bind(&limit.series_id).bind(&limit.run_id).bind(&limit.series_id).fetch_one(&mut **tx).await?;
            let used = shared_usage(tx, &limit).await?.reported_tokens;
            tokens = tokens.min(
                limit
                    .max_tokens
                    .checked_sub(used)
                    .and_then(|v| v.checked_sub(reserved))
                    .context("shared token allowance overflow")?,
            );
        }
        ensure!(tokens > 0, "token allowance is reserved or exhausted");
        sqlx::query("INSERT INTO organization_token_reservations(attempt_id,sequence,root_id,tokens) VALUES (?,?,?,?)")
            .bind(&attempt.id).bind(i64::from(attempt.model_requests)+1).bind(root_id).bind(tokens).execute(&mut **tx).await?;
        Ok(())
    }
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
            series_id: None,
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
    let data: Vec<String> = sqlx::query_scalar("SELECT l.data FROM organization_shared_limits l JOIN organization_tasks t ON t.project_id=l.project_id WHERE t.id=? AND ((l.run_id IS NULL AND l.series_id IS NULL) OR EXISTS(SELECT 1 FROM organization_workflow_tasks w LEFT JOIN organization_workflow_occurrences o ON o.run_id=w.run_id WHERE w.authorization_id=? AND (w.run_id=l.run_id OR o.series_id=l.series_id)))")
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
    ensure!(
        super::controls::check_global(tx, inference, false).await?,
        "global resource pool is paused, exhausted or unresolved"
    );
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
    if !super::controls::check_global(tx, false, true).await? {
        return Ok(false);
    }
    for limit in applicable_limits(tx, grant).await? {
        if shared_usage(tx, &limit).await?.active >= i64::from(limit.max_active) {
            return Ok(false);
        }
    }
    Ok(true)
}

//! Durable host tool dispatch and owner review; never replay a dispatched action.

use super::OrganizationStore;
use super::execution::check_fence;
use super::execution::check_live_fence;
use super::execution::event;
use super::execution::owner_event;
use super::execution::read_task;
use super::execution::set_task_state;
use super::execution::write_attempt;
use anyhow::Context;
use anyhow::Result;
use anyhow::ensure;
use codex_protocol::organization_execution::*;
use sha2::Digest;
use sha2::Sha256;
use sqlx::Sqlite;
use sqlx::Transaction;
use uuid::Uuid;

impl OrganizationStore {
    /// Called before a native handler, after hook argument rewriting. The host
    /// serializes tools per attempt so a waiting tool cannot race another dispatch.
    pub async fn prepare_action(
        &self,
        attempt_id: &str,
        worker: &str,
        call_id: &str,
        tool: &str,
        arguments: &str,
        now: i64,
    ) -> Result<OrganizationActionIntent> {
        ensure!(
            !call_id.is_empty() && call_id.len() <= 256 && !call_id.contains('\0'),
            "invalid tool call identity"
        );
        ensure!(
            matches!(tool, "exec_command" | "apply_patch"),
            "tool outside managed execution policy"
        );
        ensure!(
            arguments.len() <= 128 * 1024 && !arguments.contains('\0'),
            "tool arguments too large or invalid"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, authorization) =
            check_live_fence(&mut tx, attempt_id, worker, now).await?;
        super::workflow::check_workflow_dispatch(&mut tx, &authorization).await?;
        ensure!(
            attempt.state != OrganizationTaskState::WaitingDependency,
            "native tool cannot start while waiting for children"
        );
        let used_tokens: i64 =
            sqlx::query_scalar("SELECT used_tokens FROM organization_authorizations WHERE id=?")
                .bind(
                    authorization
                        .root_authorization_id
                        .as_ref()
                        .unwrap_or(&authorization.id),
                )
                .fetch_one(&mut *tx)
                .await?;
        ensure!(
            used_tokens < authorization.scope.max_tokens,
            "reported token ceiling exhausted"
        );
        let hash = digest(&serde_json::to_vec(&(
            attempt_id, call_id, tool, arguments,
        ))?);
        let previous: Option<String> = sqlx::query_scalar(
            "SELECT data FROM organization_actions WHERE attempt_id=? AND call_id=?",
        )
        .bind(attempt_id)
        .bind(call_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(data) = previous {
            let action: OrganizationActionIntent = serde_json::from_str(&data)?;
            ensure!(action.action_hash == hash, "tool call arguments changed");
            ensure!(
                action.state == OrganizationActionState::Pending,
                "tool call already dispatched or resolved; reconcile, never replay"
            );
            return Ok(action);
        }
        ensure!(
            attempt.state == OrganizationTaskState::Running,
            "another action is waiting"
        );
        let actions: i64 =
            sqlx::query_scalar("SELECT count(*) FROM organization_actions WHERE attempt_id=?")
                .bind(attempt_id)
                .fetch_one(&mut *tx)
                .await?;
        ensure!(actions < 100, "attempt tool-action ceiling exhausted");
        let approval_id = authorization
            .scope
            .require_tool_approval
            .then(|| Uuid::now_v7().to_string());
        let action = OrganizationActionIntent {
            id: Uuid::now_v7().to_string(),
            attempt_id: attempt_id.to_owned(),
            call_id: call_id.to_owned(),
            action_hash: hash.clone(),
            tool: tool.to_owned(),
            arguments: arguments.to_owned(),
            approval_id: approval_id.clone(),
            state: OrganizationActionState::Pending,
            receipt: None,
        };
        sqlx::query("INSERT INTO organization_actions(id,attempt_id,call_id,action_hash,state,data) VALUES (?,?,?,?,'pending',?)")
            .bind(&action.id).bind(attempt_id).bind(call_id).bind(&hash).bind(serde_json::to_string(&action)?).execute(&mut *tx).await?;
        if let Some(id) = approval_id {
            let approval = OrganizationActionApproval {
                id,
                attempt_id: attempt_id.to_owned(),
                action_id: action.id.clone(),
                action_hash: hash,
                tool: tool.to_owned(),
                arguments: arguments.to_owned(),
                state: OrganizationApprovalState::Pending,
                expires_at: authorization.expires_at.min(
                    attempt
                        .started_at
                        .checked_add(i64::from(authorization.scope.max_seconds))
                        .context("approval expiry overflow")?,
                ),
            };
            sqlx::query("INSERT INTO organization_approvals(id,action_id,state,data) VALUES (?,?,'pending',?)")
                .bind(&approval.id).bind(&action.id).bind(serde_json::to_string(&approval)?).execute(&mut *tx).await?;
            attempt.state = OrganizationTaskState::WaitingApproval;
            // A streamed output item can arrive before inference completion.
            // Yield its reservation only after usage is durably accounted.
            attempt.runtime_active = attempt.model_requests != attempt.reported_model_requests;
            write_attempt(&mut tx, &attempt).await?;
            let mut task = read_task(&mut tx, &attempt.task_id).await?;
            set_task_state(&mut tx, &mut task, OrganizationTaskState::WaitingApproval).await?;
            event(&mut tx, "approval/pending", &approval.id).await?;
        }
        event(&mut tx, "action/prepared", &action.id).await?;
        tx.commit().await?;
        Ok(action)
    }

    /// Exact owner decision, single-use and bound to the live attempt and payload.
    pub async fn decide_approval(
        &self,
        approval_id: &str,
        expected_hash: &str,
        approve: bool,
        now: i64,
    ) -> Result<OrganizationActionApproval> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut approval = read_approval(&mut tx, approval_id).await?;
        ensure!(
            approval.state == OrganizationApprovalState::Pending
                && approval.action_hash == expected_hash
                && approval.expires_at > now,
            "approval stale, changed, expired or already decided"
        );
        let worker: String =
            sqlx::query_scalar("SELECT worker_id FROM organization_attempts WHERE id=?")
                .bind(&approval.attempt_id)
                .fetch_one(&mut *tx)
                .await?;
        let (attempt, _) = check_live_fence(&mut tx, &approval.attempt_id, &worker, now).await?;
        let grant = sqlx::query_scalar::<_, String>(
            "SELECT data FROM organization_authorizations WHERE id=?",
        )
        .bind(&attempt.authorization_id)
        .fetch_one(&mut *tx)
        .await?;
        super::workflow::check_workflow_dispatch(&mut tx, &serde_json::from_str(&grant)?).await?;
        ensure!(
            attempt.state == OrganizationTaskState::WaitingApproval,
            "approval does not belong to a waiting attempt"
        );
        approval.state = if approve {
            OrganizationApprovalState::Approved
        } else {
            OrganizationApprovalState::Rejected
        };
        write_approval(&mut tx, &approval).await?;
        owner_event(
            &mut tx,
            if approve {
                "approval/approved"
            } else {
                "approval/rejected"
            },
            approval_id,
        )
        .await?;
        tx.commit().await?;
        Ok(approval)
    }

    /// Commit dispatch before executing. Returns false while approval/capacity is
    /// unavailable. No caller may execute the tool unless this returns true.
    pub async fn dispatch_action(
        &self,
        action_id: &str,
        worker: &str,
        now: i64,
        max_active: u32,
    ) -> Result<bool> {
        ensure!(
            (1..=4).contains(&max_active),
            "active execution limit must be 1–4"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut action = read_action(&mut tx, action_id).await?;
        let (mut attempt, authorization) =
            check_live_fence(&mut tx, &action.attempt_id, worker, now).await?;
        if let Err(error) = super::workflow::check_workflow_dispatch(&mut tx, &authorization).await
        {
            if error.is::<super::WorkflowPaused>() {
                return Ok(false);
            }
            return Err(error);
        }
        let used_tokens: i64 =
            sqlx::query_scalar("SELECT used_tokens FROM organization_authorizations WHERE id=?")
                .bind(
                    authorization
                        .root_authorization_id
                        .as_ref()
                        .unwrap_or(&authorization.id),
                )
                .fetch_one(&mut *tx)
                .await?;
        ensure!(
            used_tokens < authorization.scope.max_tokens,
            "reported token ceiling exhausted before dispatch"
        );
        ensure!(
            action.state == OrganizationActionState::Pending,
            "action already dispatched or resolved; never replay"
        );
        if let Some(id) = &action.approval_id {
            let approval = read_approval(&mut tx, id).await?;
            ensure!(
                approval.action_hash == action.action_hash && approval.expires_at > now,
                "approval expired or changed"
            );
            match approval.state {
                OrganizationApprovalState::Pending => return Ok(false),
                OrganizationApprovalState::Approved => {}
                _ => anyhow::bail!("action approval rejected or superseded"),
            }
        }
        if attempt.state == OrganizationTaskState::WaitingApproval || !attempt.runtime_active {
            let active: i64 =
                sqlx::query_scalar("SELECT count(*) FROM organization_attempts WHERE occupied=1")
                    .fetch_one(&mut *tx)
                    .await?;
            if !attempt.runtime_active && active >= i64::from(max_active) {
                return Ok(false);
            }
            attempt.state = OrganizationTaskState::Running;
            attempt.runtime_active = true;
            write_attempt(&mut tx, &attempt).await?;
            let mut task = read_task(&mut tx, &attempt.task_id).await?;
            set_task_state(&mut tx, &mut task, OrganizationTaskState::Running).await?;
        }
        action.state = OrganizationActionState::Dispatched;
        write_action(&mut tx, &action).await?;
        event(&mut tx, "action/dispatched", action_id).await?;
        tx.commit().await?;
        Ok(true)
    }

    /// A receipt is an observation, including after cancellation/reconciliation.
    /// It cannot reenable an attempt or cause another dispatch.
    pub async fn finish_action(
        &self,
        attempt_id: &str,
        worker: &str,
        call_id: &str,
        succeeded: bool,
        receipt: &str,
    ) -> Result<()> {
        ensure!(receipt.len() <= 8192, "action receipt too large");
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let data: String = sqlx::query_scalar("SELECT x.data FROM organization_actions x JOIN organization_attempts p ON p.id=x.attempt_id WHERE x.attempt_id=? AND p.worker_id=? AND x.call_id=?")
            .bind(attempt_id).bind(worker).bind(call_id).fetch_one(&mut *tx).await?;
        let mut action: OrganizationActionIntent = serde_json::from_str(&data)?;
        let state = if succeeded {
            OrganizationActionState::Completed
        } else {
            OrganizationActionState::Failed
        };
        if action.state == state && action.receipt.as_deref() == Some(receipt) {
            return Ok(());
        }
        ensure!(
            matches!(
                action.state,
                OrganizationActionState::Dispatched | OrganizationActionState::Uncertain
            ),
            "receipt for undispatched or already resolved action"
        );
        action.state = state;
        action.receipt = Some(receipt.to_owned());
        write_action(&mut tx, &action).await?;
        event(&mut tx, "action/receipt", &action.id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Called only after the host joins native shutdown/startup cleanup. A late
    /// observation can release a reservation but cannot restore authority.
    pub async fn release_attempt_capacity(&self, attempt_id: &str, worker: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let data: String =
            sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=? AND worker_id=?")
                .bind(attempt_id)
                .bind(worker)
                .fetch_one(&mut *tx)
                .await?;
        let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
        ensure!(
            !matches!(
                attempt.state,
                OrganizationTaskState::Running
                    | OrganizationTaskState::WaitingApproval
                    | OrganizationTaskState::WaitingDependency
            ),
            "cannot release an executing attempt without a terminal/pause transition"
        );
        if attempt.runtime_active {
            attempt.runtime_active = false;
            write_attempt(&mut tx, &attempt).await?;
            event(&mut tx, "attempt/runtimeStopped", attempt_id).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn heartbeat_attempt(&self, attempt_id: &str, worker: &str, now: i64) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, authorization) =
            check_live_fence(&mut tx, attempt_id, worker, now).await?;
        attempt.lease_until = now
            .checked_add(30)
            .context("lease overflow")?
            .min(authorization.expires_at);
        write_attempt(&mut tx, &attempt).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Host failure releases capacity and prevents further tool/model admissions.
    /// A stale callback cannot overwrite an owner's pause or startup reconciliation.
    pub async fn fail_attempt(&self, attempt_id: &str, worker: &str) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let data: String =
            sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=? AND worker_id=?")
                .bind(attempt_id)
                .bind(worker)
                .fetch_one(&mut *tx)
                .await?;
        let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
        ensure!(
            matches!(
                attempt.state,
                OrganizationTaskState::Running
                    | OrganizationTaskState::WaitingApproval
                    | OrganizationTaskState::WaitingDependency
            ),
            "attempt already stopped or reconciled"
        );
        let mut task = read_task(&mut tx, &attempt.task_id).await?;
        ensure!(
            task.state == attempt.state.as_str(),
            "task no longer belongs to this live attempt"
        );
        attempt.state = OrganizationTaskState::Failed;
        attempt.lease_until = 0;
        write_attempt(&mut tx, &attempt).await?;
        set_task_state(&mut tx, &mut task, OrganizationTaskState::Failed).await?;
        supersede_approvals(&mut tx, attempt_id).await?;
        let data: Vec<String> = sqlx::query_scalar(
            "SELECT data FROM organization_actions WHERE attempt_id=? AND state='dispatched'",
        )
        .bind(attempt_id)
        .fetch_all(&mut *tx)
        .await?;
        for data in data {
            let mut action: OrganizationActionIntent = serde_json::from_str(&data)?;
            action.state = OrganizationActionState::Uncertain;
            write_action(&mut tx, &action).await?;
        }
        event(&mut tx, "attempt/failed", attempt_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn bind_attempt_runtime(
        &self,
        attempt_id: &str,
        worker: &str,
        binding: &OrganizationRuntimeBinding,
        now: i64,
    ) -> Result<()> {
        let workspace = binding.workspace.as_str();
        let turn_id = binding.turn_id.as_deref();
        ensure!(
            std::path::Path::new(workspace).is_absolute()
                && workspace.len() <= 4096
                && !workspace.contains('\0'),
            "invalid attempt workspace"
        );
        ensure!(
            turn_id.is_none_or(|id| !id.is_empty() && id.len() <= 256 && !id.contains('\0')),
            "invalid turn ID"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, authorization) = check_fence(&mut tx, attempt_id, worker, now).await?;
        ensure!(
            binding
                .artifact_bases
                .iter()
                .map(|base| &base.path)
                .eq(authorization.scope.artifacts.iter())
                && binding
                    .artifact_bases
                    .iter()
                    .all(|base| base.sha256.as_ref().is_none_or(|hash| valid_hash(hash))),
            "artifact bases differ from authorization"
        );
        ensure!(
            attempt.artifact_bases.is_empty() || attempt.artifact_bases == binding.artifact_bases,
            "artifact bases cannot change"
        );
        ensure!(
            attempt
                .workspace
                .as_deref()
                .is_none_or(|old| old == workspace)
                && attempt
                    .turn_id
                    .as_deref()
                    .is_none_or(|old| Some(old) == turn_id),
            "attempt runtime identity cannot change"
        );
        attempt.workspace = Some(workspace.to_owned());
        attempt.turn_id = turn_id.map(str::to_owned);
        attempt.artifact_bases = binding.artifact_bases.clone();
        write_attempt(&mut tx, &attempt).await?;
        event(&mut tx, "attempt/runtimeBound", attempt_id).await?;
        tx.commit().await?;
        Ok(())
    }

    /// Pausing revokes the grant. Resuming requires fresh owner authorization;
    /// stale tools/approvals cannot revive the previous attempt.
    pub async fn stop_task(&self, task_id: &str, revision: i64, cancel: bool) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        stop_task_in(&mut tx, task_id, revision, cancel).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn execution_view(&self, task_id: &str) -> Result<OrganizationExecutionView> {
        let mut tx = self.pool.begin().await?;
        let task = read_task(&mut tx, task_id).await?;
        let row: Option<(String,i64,i64)> = sqlx::query_as("SELECT a.data,r.used_tokens,r.model_requests FROM organization_authorizations a JOIN organization_authorizations r ON r.id=a.root_id JOIN organization_tasks t ON t.authorization_id=a.id WHERE t.id=?").bind(task_id).fetch_optional(&mut *tx).await?;
        let (authorization, authorization_used_tokens, authorization_model_requests) =
            if let Some((data, tokens, requests)) = row {
                (
                    Some(serde_json::from_str(&data)?),
                    tokens,
                    u32::try_from(requests)?,
                )
            } else {
                (None, 0, 0)
            };
        let data: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE task_id=? ORDER BY generation DESC LIMIT 32").bind(task_id).fetch_all(&mut *tx).await?;
        let attempts: Vec<OrganizationTaskAttempt> = data
            .into_iter()
            .map(|data| serde_json::from_str(&data))
            .collect::<std::result::Result<_, _>>()?;
        let mut actions = Vec::new();
        let mut approvals = Vec::new();
        let mut result = None;
        if let Some(attempt) = attempts.first() {
            let data: Vec<String> = sqlx::query_scalar(
                "SELECT data FROM organization_actions WHERE attempt_id=? ORDER BY id LIMIT 100",
            )
            .bind(&attempt.id)
            .fetch_all(&mut *tx)
            .await?;
            actions = data
                .into_iter()
                .map(|data| serde_json::from_str(&data))
                .collect::<std::result::Result<_, _>>()?;
            let data: Vec<String> = sqlx::query_scalar("SELECT a.data FROM organization_approvals a JOIN organization_actions x ON x.id=a.action_id WHERE x.attempt_id=? ORDER BY a.id LIMIT 100").bind(&attempt.id).fetch_all(&mut *tx).await?;
            approvals = data
                .into_iter()
                .map(|data| serde_json::from_str(&data))
                .collect::<std::result::Result<_, _>>()?;
            let data: Option<String> =
                sqlx::query_scalar("SELECT data FROM organization_results WHERE attempt_id=?")
                    .bind(&attempt.id)
                    .fetch_optional(&mut *tx)
                    .await?;
            result = data.map(|data| serde_json::from_str(&data)).transpose()?;
        }
        let applications = sqlx::query_scalar::<_, String>(
            "SELECT data FROM organization_applications WHERE task_id=? ORDER BY id DESC LIMIT 32",
        )
        .bind(task_id)
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|data| serde_json::from_str(&data))
        .collect::<std::result::Result<_, _>>()?;
        let (checkpoints, handoffs) = super::handoff::continuity_view(&mut tx, task_id).await?;
        tx.commit().await?;
        Ok(OrganizationExecutionView {
            task,
            authorization,
            authorization_used_tokens,
            authorization_model_requests,
            attempts,
            actions,
            approvals,
            result,
            applications,
            checkpoints,
            handoffs,
        })
    }

    /// Persist before any original-project write. Repeating an uncertain or
    /// completed digest is refused; restoration never silently replays it.
    pub async fn begin_application(
        &self,
        task_id: &str,
        revision: i64,
        digest: &str,
        paths: Option<&[String]>,
    ) -> Result<OrganizationApplication> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let task = read_task(&mut tx, task_id).await?;
        ensure!(
            task.state == "accepted" && task.revision == revision,
            "application requires the exact accepted task revision"
        );
        let data:String=sqlx::query_scalar("SELECT r.data FROM organization_results r JOIN organization_attempts p ON p.id=r.attempt_id WHERE p.task_id=? ORDER BY p.generation DESC LIMIT 1").bind(task_id).fetch_one(&mut *tx).await?;
        let result: OrganizationTaskResult = serde_json::from_str(&data)?;
        ensure!(
            result.accepted && result.digest == digest,
            "accepted result digest changed"
        );
        let existing: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM organization_applications WHERE task_id=? AND digest=?",
        )
        .bind(task_id)
        .bind(digest)
        .fetch_one(&mut *tx)
        .await?;
        ensure!(
            existing == 0,
            "result application already recorded; inspect its receipt, never replay an uncertain operation"
        );
        let application = OrganizationApplication {
            id: Uuid::now_v7().to_string(),
            digest: digest.into(),
            paths: result
                .application_paths(paths)
                .map_err(anyhow::Error::msg)?,
            state: OrganizationApplicationState::Pending,
            receipt: None,
        };
        sqlx::query("INSERT INTO organization_applications(id,task_id,digest,state,data) VALUES(?,?,?,'pending',?)").bind(&application.id).bind(task_id).bind(digest).bind(serde_json::to_string(&application)?).execute(&mut *tx).await?;
        owner_event(&mut tx, "application/intent", &application.id).await?;
        tx.commit().await?;
        Ok(application)
    }

    pub async fn settle_application(&self, id: &str, completed: bool, receipt: &str) -> Result<()> {
        ensure!(receipt.len() <= 4096, "application receipt too large");
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let data: String = sqlx::query_scalar(
            "SELECT data FROM organization_applications WHERE id=? AND state='pending'",
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;
        let mut application: OrganizationApplication = serde_json::from_str(&data)?;
        application.state = if completed {
            OrganizationApplicationState::Completed
        } else {
            OrganizationApplicationState::Uncertain
        };
        application.receipt = Some(receipt.into());
        sqlx::query("UPDATE organization_applications SET state=?,data=? WHERE id=?")
            .bind(if completed { "completed" } else { "uncertain" })
            .bind(serde_json::to_string(&application)?)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        owner_event(
            &mut tx,
            if completed {
                "application/completed"
            } else {
                "application/uncertain"
            },
            id,
        )
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Evidence must be collected by the trusted host, never a model declaration.
    pub async fn record_result(
        &self,
        attempt_id: &str,
        worker: &str,
        artifacts: Vec<OrganizationArtifactEvidence>,
        checks: Vec<OrganizationCheckEvidence>,
        now: i64,
    ) -> Result<OrganizationTaskResult> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let (mut attempt, authorization) = check_fence(&mut tx, attempt_id, worker, now).await?;
        ensure!(
            attempt.reported_model_requests == attempt.model_requests,
            "result has unresolved model usage"
        );
        ensure!(
            artifacts.len() == authorization.scope.artifacts.len()
                && artifacts
                    .iter()
                    .map(|a| &a.path)
                    .eq(authorization.scope.artifacts.iter()),
            "result artifacts differ from authorization"
        );
        ensure!(
            artifacts.iter().all(|a| a.sha256.len() == 64
                && a.sha256
                    .bytes()
                    .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())),
            "invalid artifact digest"
        );
        ensure!(
            checks.len() == authorization.scope.checks.len()
                && checks
                    .iter()
                    .map(|c| &c.argv)
                    .eq(authorization.scope.checks.iter())
                && checks.iter().all(|c| c.output.len() <= 65536),
            "result checks differ from authorization or output too large"
        );
        let unresolved: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_actions WHERE attempt_id=? AND state IN ('pending','dispatched','uncertain')").bind(attempt_id).fetch_one(&mut *tx).await?;
        ensure!(unresolved == 0, "result has unresolved actions");
        let children: i64 = sqlx::query_scalar("SELECT count(*) FROM organization_authorizations a JOIN organization_tasks t ON t.authorization_id=a.id WHERE a.parent_attempt_id=? AND t.state<>'accepted'")
            .bind(attempt_id).fetch_one(&mut *tx).await?;
        ensure!(children == 0, "result has unfinished delegated children");
        let accepted = authorization.parent_attempt_id.is_some()
            && authorization.scope.auto_accept_children
            && !checks.is_empty()
            && checks.iter().all(|check| check.exit_code == Some(0));
        let result = OrganizationTaskResult {
            attempt_id: attempt_id.to_owned(),
            digest: digest(&serde_json::to_vec(&(
                &artifacts,
                &checks,
                &attempt.artifact_bases,
            ))?),
            artifacts,
            artifact_bases: attempt.artifact_bases.clone(),
            checks,
            accepted,
        };
        sqlx::query("INSERT INTO organization_results(attempt_id,digest,data) VALUES (?,?,?)")
            .bind(attempt_id)
            .bind(&result.digest)
            .bind(serde_json::to_string(&result)?)
            .execute(&mut *tx)
            .await?;
        attempt.state = if accepted {
            OrganizationTaskState::Accepted
        } else {
            OrganizationTaskState::NeedsReview
        };
        attempt.lease_until = 0;
        write_attempt(&mut tx, &attempt).await?;
        let mut task = read_task(&mut tx, &attempt.task_id).await?;
        set_task_state(&mut tx, &mut task, attempt.state).await?;
        event(
            &mut tx,
            if accepted {
                "result/checkedChildAccepted"
            } else {
                "result/ready"
            },
            attempt_id,
        )
        .await?;
        tx.commit().await?;
        Ok(result)
    }

    pub async fn accept_result(
        &self,
        task_id: &str,
        revision: i64,
        expected_digest: &str,
    ) -> Result<()> {
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut task = read_task(&mut tx, task_id).await?;
        ensure!(
            task.revision == revision && task.state == "needsReview",
            "result review revision conflict"
        );
        let (attempt_id, data): (String,String) = sqlx::query_as("SELECT r.attempt_id,r.data FROM organization_results r JOIN organization_attempts p ON p.id=r.attempt_id JOIN organization_tasks t ON t.id=p.task_id WHERE t.id=? AND p.authorization_id=t.authorization_id AND p.state='needsReview' ORDER BY p.generation DESC LIMIT 1").bind(task_id).fetch_one(&mut *tx).await?;
        let mut result: OrganizationTaskResult = serde_json::from_str(&data)?;
        ensure!(
            result.digest == expected_digest && !result.accepted,
            "result digest changed or already accepted"
        );
        ensure!(
            result.checks.iter().all(|check| check.exit_code == Some(0)),
            "failing or incomplete checks cannot be accepted"
        );
        result.accepted = true;
        sqlx::query("UPDATE organization_results SET data=? WHERE attempt_id=?")
            .bind(serde_json::to_string(&result)?)
            .bind(&attempt_id)
            .execute(&mut *tx)
            .await?;
        let data: String = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE id=?")
            .bind(&attempt_id)
            .fetch_one(&mut *tx)
            .await?;
        let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
        attempt.state = OrganizationTaskState::Accepted;
        write_attempt(&mut tx, &attempt).await?;
        set_task_state(&mut tx, &mut task, OrganizationTaskState::Accepted).await?;
        owner_event(&mut tx, "result/accepted", &attempt_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn request_changes(
        &self,
        task_id: &str,
        revision: i64,
        instructions: &str,
    ) -> Result<()> {
        ensure!(
            !instructions.trim().is_empty()
                && instructions.len() <= 16384
                && !instructions.contains('\0'),
            "invalid change request"
        );
        let mut tx = self.pool.begin_with("BEGIN IMMEDIATE").await?;
        let mut task = read_task(&mut tx, task_id).await?;
        ensure!(
            task.revision == revision && task.state == "needsReview",
            "result review revision conflict"
        );
        task.instructions.push_str("\nOwner requested changes:\n");
        task.instructions.push_str(instructions);
        ensure!(
            task.instructions.len() <= 65536,
            "combined task instructions exceed 64 KiB"
        );
        set_task_state(&mut tx, &mut task, OrganizationTaskState::Paused).await?;
        owner_event(&mut tx, "result/changesRequested", task_id).await?;
        tx.commit().await?;
        Ok(())
    }
}

pub(super) async fn read_action(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<OrganizationActionIntent> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_actions WHERE id=?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(serde_json::from_str(&data)?)
}

pub(super) async fn write_action(
    tx: &mut Transaction<'_, Sqlite>,
    action: &OrganizationActionIntent,
) -> Result<()> {
    sqlx::query("UPDATE organization_actions SET state=?,data=? WHERE id=?")
        .bind(action.state.as_str())
        .bind(serde_json::to_string(action)?)
        .bind(&action.id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn read_approval(
    tx: &mut Transaction<'_, Sqlite>,
    id: &str,
) -> Result<OrganizationActionApproval> {
    let data: String = sqlx::query_scalar("SELECT data FROM organization_approvals WHERE id=?")
        .bind(id)
        .fetch_one(&mut **tx)
        .await?;
    Ok(serde_json::from_str(&data)?)
}

async fn write_approval(
    tx: &mut Transaction<'_, Sqlite>,
    approval: &OrganizationActionApproval,
) -> Result<()> {
    let state = match approval.state {
        OrganizationApprovalState::Pending => "pending",
        OrganizationApprovalState::Approved => "approved",
        OrganizationApprovalState::Rejected => "rejected",
        OrganizationApprovalState::Superseded => "superseded",
    };
    sqlx::query("UPDATE organization_approvals SET state=?,data=? WHERE id=?")
        .bind(state)
        .bind(serde_json::to_string(approval)?)
        .bind(&approval.id)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) async fn supersede_approvals(
    tx: &mut Transaction<'_, Sqlite>,
    attempt_id: &str,
) -> Result<()> {
    let data: Vec<String> = sqlx::query_scalar("SELECT a.data FROM organization_approvals a JOIN organization_actions x ON x.id=a.action_id WHERE x.attempt_id=? AND a.state IN ('pending','approved')").bind(attempt_id).fetch_all(&mut **tx).await?;
    for data in data {
        let mut approval: OrganizationActionApproval = serde_json::from_str(&data)?;
        approval.state = OrganizationApprovalState::Superseded;
        write_approval(tx, &approval).await?;
    }
    Ok(())
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

pub(super) async fn stop_task_in(
    tx: &mut Transaction<'_, Sqlite>,
    task_id: &str,
    revision: i64,
    cancel: bool,
) -> Result<()> {
    let mut task = read_task(tx, task_id).await?;
    ensure!(task.revision == revision, "task revision conflict");
    ensure!(
        matches!(
            task.state.as_str(),
            "queued"
                | "running"
                | "waitingApproval"
                | "waitingDependency"
                | "paused"
                | "blocked"
                | "needsReconciliation"
        ),
        "task cannot be stopped in its current state"
    );
    let state = if cancel {
        OrganizationTaskState::Cancelled
    } else {
        OrganizationTaskState::Paused
    };
    let grants: Vec<String> = sqlx::query_scalar(
        "SELECT data FROM organization_authorizations WHERE task_id=? AND revoked=0",
    )
    .bind(task_id)
    .fetch_all(&mut **tx)
    .await?;
    for data in grants {
        let mut authorization: OrganizationTaskAuthorization = serde_json::from_str(&data)?;
        authorization.revoked = true;
        sqlx::query("UPDATE organization_authorizations SET revoked=1,data=? WHERE id=?")
            .bind(serde_json::to_string(&authorization)?)
            .bind(&authorization.id)
            .execute(&mut **tx)
            .await?;
    }
    let attempts: Vec<String> = sqlx::query_scalar("SELECT data FROM organization_attempts WHERE task_id=? AND state IN ('running','waitingApproval','waitingDependency','blocked','needsReconciliation')").bind(task_id).fetch_all(&mut **tx).await?;
    for data in attempts {
        let mut attempt: OrganizationTaskAttempt = serde_json::from_str(&data)?;
        attempt.state = state;
        attempt.lease_until = 0;
        write_attempt(tx, &attempt).await?;
        supersede_approvals(tx, &attempt.id).await?;
    }
    set_task_state(tx, &mut task, state).await?;
    owner_event(
        tx,
        if cancel {
            "task/cancelled"
        } else {
            "task/paused"
        },
        task_id,
    )
    .await?;
    Ok(())
}

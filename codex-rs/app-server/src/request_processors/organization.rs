//! Owner-only organization commands over the existing app-server transport.

use anyhow::Context;

use crate::error_code::internal_error;
use crate::error_code::invalid_params;
use crate::error_code::invalid_request;
use crate::organization_runtime::OrganizationRuntime;
use crate::transport::ConnectionOrigin;
use codex_app_server_protocol::ClientResponsePayload;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::OrganizationBackupResponse;
use codex_app_server_protocol::OrganizationExecutionMutateParams;
use codex_app_server_protocol::OrganizationExecutionMutateResponse;
use codex_app_server_protocol::OrganizationExecutionReadParams;
use codex_app_server_protocol::OrganizationExecutionReadResponse;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationListResponse;
use codex_app_server_protocol::OrganizationMemorySearchParams;
use codex_app_server_protocol::OrganizationMemorySearchResponse;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::OrganizationMutateResponse;
use codex_app_server_protocol::OrganizationReportParams;
use codex_app_server_protocol::OrganizationReportResponse;
use codex_app_server_protocol::OrganizationScheduleListParams;
use codex_app_server_protocol::OrganizationScheduleListResponse;
use codex_app_server_protocol::OrganizationScheduleMutateParams;
use codex_app_server_protocol::OrganizationScheduleMutateResponse;
use codex_app_server_protocol::OrganizationWorkflowListParams;
use codex_app_server_protocol::OrganizationWorkflowListResponse;
use codex_app_server_protocol::OrganizationWorkflowMutateParams;
use codex_app_server_protocol::OrganizationWorkflowMutateResponse;
use codex_app_server_protocol::OrganizationWorkflowPreviewParams;
use codex_app_server_protocol::OrganizationWorkflowPreviewResponse;
use codex_core::ThreadManager;
use codex_core::config::Config;
use codex_protocol::organization::OrganizationMutation;
use codex_protocol::organization_execution::OrganizationExecutionMutation;
use codex_state::organization::OrganizationStore;
use codex_thread_store::ThreadStore;
use std::sync::Arc;

pub(crate) struct OrganizationRequestProcessor {
    runtime: Arc<OrganizationRuntime>,
    thread_store: Arc<dyn ThreadStore>,
    config: Arc<Config>,
    application_lock: Arc<tokio::sync::Semaphore>,
}

impl OrganizationRequestProcessor {
    pub(crate) fn new(
        config: Arc<Config>,
        manager: &Arc<ThreadManager>,
        thread_store: Arc<dyn ThreadStore>,
    ) -> Self {
        Self {
            runtime: OrganizationRuntime::new(
                config.clone(),
                Arc::downgrade(manager),
                thread_store.clone(),
            ),
            thread_store,
            config,
            application_lock: Arc::new(tokio::sync::Semaphore::new(1)),
        }
    }

    async fn store(&self) -> Result<&OrganizationStore, JSONRPCErrorError> {
        self.runtime.store().await.map_err(|error| {
            internal_error(format!(
                "organization state unavailable; preserve and restore it: {error:#}"
            ))
        })
    }

    pub(crate) fn shutdown(&self) {
        self.runtime.shutdown();
    }

    pub(crate) async fn list(
        &self,
        params: OrganizationListParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        if params
            .limit
            .is_some_and(|limit| !(1..=200).contains(&limit))
            || params.cursor.as_ref().is_some_and(|cursor| {
                cursor.trim().is_empty() || cursor.len() > 256 || cursor.contains('\0')
            })
        {
            return Err(invalid_params(
                "limit must be 1–200 and cursor a nonempty ID of at most 256 bytes",
            ));
        }
        let (data, next_cursor) = self
            .store()
            .await?
            .list_definitions(params.kind, params.cursor, params.limit.unwrap_or(100))
            .await
            .map_err(|error| internal_error(format!("organization/list failed: {error:#}")))?;
        Ok(Some(OrganizationListResponse { data, next_cursor }.into()))
    }

    pub(crate) async fn mutate(
        &self,
        params: OrganizationMutateParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        let descriptor =
            if let OrganizationMutation::LinkProject { project_id, .. } = &params.mutation {
                let project = self
                    .thread_store
                    .read_project(project_id.clone())
                    .await
                    .map_err(|error| internal_error(format!("project lookup failed: {error}")))?
                    .ok_or_else(|| invalid_params("existing project not found"))?;
                Some((
                    project.name,
                    project.roots.into_iter().map(|root| root.path).collect(),
                ))
            } else {
                None
            };
        let record = self
            .store()
            .await?
            .mutate_owner(params.mutation, descriptor)
            .await
            .map_err(|error| {
                invalid_params(format!("organization mutation rejected: {error:#}"))
            })?;
        Ok(Some(OrganizationMutateResponse { record }.into()))
    }

    pub(crate) async fn backup(
        &self,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        let path =
            self.store().await?.backup().await.map_err(|error| {
                internal_error(format!("organization backup failed: {error:#}"))
            })?;
        Ok(Some(
            OrganizationBackupResponse {
                path: path.to_string_lossy().into_owned(),
            }
            .into(),
        ))
    }

    pub(crate) async fn memory_search(
        &self,
        params: OrganizationMemorySearchParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        let result = self
            .store()
            .await?
            .search_memory(
                &params.project_id,
                params.scope,
                params.agent_id.as_deref(),
                &params.query,
            )
            .await
            .map_err(|error| invalid_params(format!("memory search rejected: {error:#}")))?;
        Ok(Some(OrganizationMemorySearchResponse { result }.into()))
    }

    pub(crate) async fn execution_read(
        &self,
        params: OrganizationExecutionReadParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        self.runtime
            .ensure_execution_owner()
            .await
            .map_err(|error| internal_error(format!("execution owner unavailable: {error:#}")))?;
        let view = self
            .store()
            .await?
            .execution_view(&params.task_id)
            .await
            .map_err(|error| invalid_params(format!("execution read rejected: {error:#}")))?;
        Ok(Some(OrganizationExecutionReadResponse { view }.into()))
    }

    pub(crate) async fn inventory(
        &self,
        _params: codex_app_server_protocol::OrganizationInventoryParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        let inventory = crate::organization_inventory::inventory(&self.config, self.store().await?)
            .await
            .map_err(|error| invalid_params(format!("inventory unavailable: {error:#}")))?;
        Ok(Some(
            codex_app_server_protocol::OrganizationInventoryResponse { inventory }.into(),
        ))
    }

    pub(crate) async fn report(
        &self,
        params: OrganizationReportParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        let mut report = self
            .store()
            .await?
            .operational_report(
                params.project_id.as_deref(),
                params.cursor.as_deref(),
                params.limit.unwrap_or(100),
                chrono::Utc::now().timestamp(),
            )
            .await
            .map_err(|error| invalid_params(format!("organization report rejected: {error:#}")))?;
        report.runtime_resources = Some(runtime_resources());
        Ok(Some(OrganizationReportResponse { report }.into()))
    }

    pub(crate) async fn schedule_list(
        &self,
        params: OrganizationScheduleListParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        self.runtime
            .ensure_execution_owner()
            .await
            .map_err(|error| internal_error(format!("execution owner unavailable: {error:#}")))?;
        let (data, next_cursor) = self
            .store()
            .await?
            .list_schedules(
                params.project_id.as_deref(),
                params.cursor.as_deref(),
                params.limit.unwrap_or(100),
            )
            .await
            .map_err(|error| invalid_params(format!("schedule list rejected: {error:#}")))?;
        Ok(Some(
            OrganizationScheduleListResponse { data, next_cursor }.into(),
        ))
    }

    pub(crate) async fn schedule_mutate(
        &self,
        params: OrganizationScheduleMutateParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        if serde_json::to_vec(&params)
            .map_err(|_| invalid_params("invalid schedule mutation"))?
            .len()
            > 128 * 1024
        {
            return Err(invalid_params("schedule mutation exceeds 128 KiB"));
        }
        self.runtime
            .ensure_execution_owner()
            .await
            .map_err(|error| internal_error(format!("execution owner unavailable: {error:#}")))?;
        let view = self
            .store()
            .await?
            .mutate_schedule(params.mutation, crate::organization_runtime::now())
            .await
            .map_err(|error| invalid_params(format!("schedule mutation rejected: {error:#}")))?;
        self.runtime.wake();
        Ok(Some(OrganizationScheduleMutateResponse { view }.into()))
    }

    pub(crate) async fn workflow_list(
        &self,
        params: OrganizationWorkflowListParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        self.runtime
            .ensure_execution_owner()
            .await
            .map_err(|error| internal_error(format!("execution owner unavailable: {error:#}")))?;
        let (data, next_cursor) = self
            .store()
            .await?
            .list_workflows(
                params.kind,
                params.project_id.as_deref(),
                params.cursor.as_deref(),
                params.limit.unwrap_or(100),
            )
            .await
            .map_err(|error| invalid_params(format!("workflow list rejected: {error:#}")))?;
        Ok(Some(
            OrganizationWorkflowListResponse { data, next_cursor }.into(),
        ))
    }

    pub(crate) async fn workflow_preview(
        &self,
        params: OrganizationWorkflowPreviewParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        let preview = self
            .store()
            .await?
            .preview_workflow(params.request)
            .await
            .map_err(|error| invalid_params(format!("workflow preview rejected: {error:#}")))?;
        Ok(Some(OrganizationWorkflowPreviewResponse { preview }.into()))
    }

    pub(crate) async fn workflow_mutate(
        &self,
        params: OrganizationWorkflowMutateParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        self.runtime
            .ensure_execution_owner()
            .await
            .map_err(|error| internal_error(format!("execution owner unavailable: {error:#}")))?;
        let view = self
            .store()
            .await?
            .mutate_workflow(params.mutation, crate::organization_runtime::now())
            .await
            .map_err(|error| invalid_params(format!("workflow mutation rejected: {error:#}")))?;
        self.runtime.wake();
        Ok(Some(OrganizationWorkflowMutateResponse { view }.into()))
    }

    pub(crate) async fn execution_mutate(
        &self,
        params: OrganizationExecutionMutateParams,
        origin: ConnectionOrigin,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        require_owner(origin)?;
        if serde_json::to_vec(&params)
            .map_err(|_| invalid_params("invalid execution mutation"))?
            .len()
            > 128 * 1024
        {
            return Err(invalid_params("execution mutation exceeds 128 KiB"));
        }
        let task_id = params.mutation.task_id().to_owned();
        // Reconcile interrupted work before any approval/review can be decided
        // against a previous daemon's otherwise unexpired lease.
        self.runtime
            .ensure_execution_owner()
            .await
            .map_err(|error| internal_error(format!("execution owner unavailable: {error:#}")))?;
        let store = self.store().await?;
        let mut apply_rejection = None;
        let result = async {
            match params.mutation {
                OrganizationExecutionMutation::AuthorizeProfile {task_id,expected_revision,profile} => {
                    if let codex_protocol::organization::OrganizationControlValue::ExecutionProfile { scope, .. } = &profile.value
                        && let Some(budget) = &scope.cost_budget {
                        let provider = crate::organization_inventory::provider_evidence(&self.config.model_provider_id, &self.config.model_provider)?;
                        anyhow::ensure!(budget.provider_id == provider.id && budget.provider_endpoint_sha256 == provider.endpoint_sha256, "cost quote provider does not match the daemon");
                    }
                    store.authorize_profile(&task_id,expected_revision,&profile,crate::organization_runtime::now()).await?;
                }
                OrganizationExecutionMutation::ApplyReplan { task_id, id, expected_revision } => {
                    let record=store.read_definition(codex_protocol::organization::OrganizationRecordKind::Controls,&format!("replan:{id}")).await.context("replan proposal missing")?;
                    let codex_protocol::organization::OrganizationRecord::Control(policy)=record else {anyhow::bail!("not a replan policy")};
                    anyhow::ensure!(policy.revision==expected_revision,"replan revision conflict");
                    let codex_protocol::organization::OrganizationControlValue::Replan { items, applied_handoff_ids, .. }=policy.value else {anyhow::bail!("not a replan proposal")};
                    anyhow::ensure!(applied_handoff_ids.is_empty(),"replan already applied");
                    for item in &items {
                        let view=store.execution_view(&item.task_id).await?;
                        let checkpoint=view.checkpoints.iter().find(|c|c.id==item.checkpoint_id).context("replan checkpoint missing")?;
                        let source=store.attempt_snapshot(&checkpoint.attempt_id).await?;
                        anyhow::ensure!(source.task_id==view.task.id && source.authorization_id==checkpoint.authorization_id,"replan checkpoint identity changed");
                        crate::organization_workspace::verify_checkpoint(std::path::Path::new(source.workspace.as_deref().context("checkpoint workspace missing")?),checkpoint)?;
                    }
                    store.apply_replan(&task_id,&id,expected_revision,crate::organization_runtime::now()).await?;
                }
                handoff @ OrganizationExecutionMutation::Handoff { .. } => {
                    let view = store.execution_view(handoff.task_id()).await?;
                    let OrganizationExecutionMutation::Handoff { checkpoint_id, .. } = &handoff else { unreachable!() };
                    let checkpoint = view.checkpoints.iter().find(|checkpoint| &checkpoint.id == checkpoint_id)
                        .context("handoff checkpoint missing")?;
                    let source = store.attempt_snapshot(&checkpoint.attempt_id).await?;
                    anyhow::ensure!(source.task_id == view.task.id && source.authorization_id == checkpoint.authorization_id,
                        "checkpoint source identity changed");
                    crate::organization_workspace::verify_checkpoint(std::path::Path::new(source.workspace.as_deref()
                        .context("checkpoint workspace missing")?), checkpoint)?;
                    store.handoff_task(&handoff, crate::organization_runtime::now()).await?;
                }
                OrganizationExecutionMutation::Authorize {
                    task_id,
                    expected_revision,
                    scope,
                } => {
                    if let Some(budget) = &scope.cost_budget {
                        let provider = crate::organization_inventory::provider_evidence(&self.config.model_provider_id, &self.config.model_provider)?;
                        anyhow::ensure!(budget.provider_id == provider.id && budget.provider_endpoint_sha256 == provider.endpoint_sha256, "cost quote provider does not match the daemon");
                    }
                    store
                        .authorize_task(
                            &task_id,
                            expected_revision,
                            scope,
                            crate::organization_runtime::now(),
                        )
                        .await?;
                }
                OrganizationExecutionMutation::Stop {
                    task_id,
                    expected_revision,
                    cancel,
                } => store.stop_task(&task_id, expected_revision, cancel).await?,
                OrganizationExecutionMutation::DecideApproval {
                    task_id,
                    approval_id,
                    expected_hash,
                    approve,
                } => {
                    let view = store.execution_view(&task_id).await?;
                    anyhow::ensure!(
                        view.approvals
                            .iter()
                            .any(|approval| approval.id == approval_id),
                        "approval does not belong to the current task attempt"
                    );
                    store
                        .decide_approval(
                            &approval_id,
                            &expected_hash,
                            approve,
                            crate::organization_runtime::now(),
                        )
                        .await?;
                }
                OrganizationExecutionMutation::Accept {
                    task_id,
                    expected_revision,
                    digest,
                } => {
                    store
                        .accept_result(&task_id, expected_revision, &digest)
                        .await?
                }
                OrganizationExecutionMutation::RequestChanges {
                    task_id,
                    expected_revision,
                    instructions,
                } => {
                    store
                        .request_changes(&task_id, expected_revision, &instructions)
                        .await?
                }
                OrganizationExecutionMutation::Apply {task_id,expected_revision,digest,paths,hunks} => {
                    let _application = self.application_lock.clone().acquire_owned().await?;
                    let mut no_intent = false;
                    let prepared = async {
                        let view = store.execution_view(&task_id).await?;
                        anyhow::ensure!(!view.applications.iter().any(|application| application.digest==digest),"result application already recorded; inspect its receipt, never replay an uncertain operation");
                        no_intent = true;
                        anyhow::ensure!(view.task.state=="accepted"&&view.task.revision==expected_revision,"application requires exact accepted task revision");
                        let result=view.result.context("accepted result missing")?;
                        anyhow::ensure!(result.accepted&&result.digest==digest,"accepted result digest changed");
                        let linked=self.thread_store.read_project(view.task.project_id.clone()).await?.context("project missing")?;
                        let codex_protocol::organization::OrganizationRecord::Project(project)=store.read_definition(codex_protocol::organization::OrganizationRecordKind::Projects,&view.task.project_id).await? else {anyhow::bail!("invalid project record")};
                        anyhow::ensure!(linked.roots.iter().map(|r|&r.path).eq(project.roots.iter()),"project roots changed");
                        let source=view.attempts.iter().find(|a|a.id==result.attempt_id).and_then(|a|a.workspace.as_deref()).context("accepted workspace missing")?;
                        crate::organization_workspace::PreparedApplication::prepare_hunks(&self.config,std::path::Path::new(project.roots.first().context("project root missing")?),std::path::Path::new(source),&result,paths.as_deref(),hunks.as_deref())
                    }.await;
                    let prepared = match prepared {
                        Ok(prepared) => prepared,
                        Err(error) => {
                            // Only certify rejection while holding the application lock,
                            // before attempting the durable intent transaction. Unknown
                            // transport/commit/write outcomes never get this signal.
                            if no_intent {
                                apply_rejection = Some(serde_json::json!({
                                    "application": "notStarted", "taskId": task_id, "digest": digest
                                }));
                            }
                            return Err(error);
                        }
                    };
                    let intent=store.begin_application_hunks(&task_id,expected_revision,&digest,paths.as_deref(),hunks.as_deref()).await?;
                    let applied=tokio::task::spawn_blocking(move||{
                        // A disconnected/cancelled owner RPC must not release
                        // serialization while its native file writes still run.
                        let _permit=_application;
                        prepared.apply()
                    }).await;
                    match applied {
                        Ok(Ok(()))=>store.settle_application(&intent.id,true,"selected accepted artifacts replaced and directory entries synchronized").await?,
                        outcome=>{
                            let receipt=format!("application failed or interrupted; files may be partially applied; inspect originals before manual reconciliation: {outcome:?}");
                            store.settle_application(&intent.id,false,&receipt.chars().take(4000).collect::<String>()).await?;
                            anyhow::bail!("{receipt}");
                        }
                    }
                }
            }
            Ok::<(), anyhow::Error>(())
        }
        .await;
        result.map_err(|error| {
            let mut response = invalid_params(format!("execution mutation rejected: {error:#}"));
            response.data = apply_rejection;
            response
        })?;
        self.runtime.wake();
        let view = store.execution_view(&task_id).await.map_err(|error| {
            internal_error(format!("execution committed; reread task state: {error:#}"))
        })?;
        Ok(Some(OrganizationExecutionMutateResponse { view }.into()))
    }
}

fn require_owner(origin: ConnectionOrigin) -> Result<(), JSONRPCErrorError> {
    if matches!(
        origin,
        ConnectionOrigin::Stdio
            | ConnectionOrigin::InProcess
            | ConnectionOrigin::LocalControlSocket
    ) {
        Ok(())
    } else {
        Err(invalid_request(
            "organization administration requires the protected local owner connection",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_administration_does_not_trust_network_clients() {
        for origin in [
            ConnectionOrigin::Stdio,
            ConnectionOrigin::InProcess,
            ConnectionOrigin::LocalControlSocket,
        ] {
            assert!(require_owner(origin).is_ok());
        }
        for origin in [ConnectionOrigin::WebSocket, ConnectionOrigin::RemoteControl] {
            assert!(require_owner(origin).is_err());
        }
    }
}

fn runtime_resources() -> codex_protocol::organization_report::OrganizationRuntimeResources {
    let mut value = codex_protocol::organization_report::OrganizationRuntimeResources {
        process_id: std::process::id(),
        measured_at_ms: chrono::Utc::now().timestamp_millis(),
        resident_bytes: None,
        cpu_time_ms: None,
    };
    #[cfg(target_os = "linux")]
    {
        value.resident_bytes = std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|text| {
                text.lines()
                    .find(|line| line.starts_with("VmRSS:"))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|v| v.parse::<u64>().ok())
                    .and_then(|kb| kb.checked_mul(1024))
            });
        let ticks = unsafe { libc::sysconf(libc::_SC_CLK_TCK) };
        if let Ok(ticks) = u64::try_from(ticks)
            && ticks > 0
        {
            value.cpu_time_ms = std::fs::read_to_string("/proc/self/stat")
                .ok()
                .and_then(|text| {
                    let (_, fields) = text.rsplit_once(") ")?;
                    let fields: Vec<_> = fields.split_whitespace().collect();
                    let user = fields.get(11)?.parse::<u64>().ok()?;
                    let system = fields.get(12)?.parse::<u64>().ok()?;
                    user.checked_add(system)?
                        .checked_mul(1000)?
                        .checked_div(ticks)
                });
        }
    }
    value
}

#[test]
fn runtime_resources_measure_only_the_serving_process_and_leave_unsupported_dimensions_unavailable()
{
    let value = runtime_resources();
    assert_eq!(value.process_id, std::process::id());
    assert!(value.measured_at_ms > 0);
    #[cfg(target_os = "linux")]
    {
        assert!(value.resident_bytes.is_some_and(|bytes| bytes > 0));
        assert!(value.cpu_time_ms.is_some());
    }
    #[cfg(not(target_os = "linux"))]
    {
        assert!(value.resident_bytes.is_none() && value.cpu_time_ms.is_none());
    }
}

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
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::OrganizationMutateResponse;
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
        let result = async {
            match params.mutation {
                OrganizationExecutionMutation::Authorize {
                    task_id,
                    expected_revision,
                    scope,
                } => {
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
                OrganizationExecutionMutation::Apply {task_id,expected_revision,digest} => {
                    let _application = self.application_lock.clone().acquire_owned().await?;
                    let view = store.execution_view(&task_id).await?;
                    anyhow::ensure!(view.task.state=="accepted"&&view.task.revision==expected_revision,"application requires exact accepted task revision");
                    let result=view.result.context("accepted result missing")?;
                    anyhow::ensure!(result.accepted&&result.digest==digest,"accepted result digest changed");
                    anyhow::ensure!(!view.applications.iter().any(|application| application.digest==digest),"result application already recorded; inspect its receipt, never replay an uncertain operation");
                    let linked=self.thread_store.read_project(view.task.project_id.clone()).await?.context("project missing")?;
                    let codex_protocol::organization::OrganizationRecord::Project(project)=store.read_definition(codex_protocol::organization::OrganizationRecordKind::Projects,&view.task.project_id).await? else {anyhow::bail!("invalid project record")};
                    anyhow::ensure!(linked.roots.iter().map(|r|&r.path).eq(project.roots.iter()),"project roots changed");
                    let source=view.attempts.iter().find(|a|a.id==result.attempt_id).and_then(|a|a.workspace.as_deref()).context("accepted workspace missing")?;
                    let prepared=crate::organization_workspace::PreparedApplication::prepare(&self.config,std::path::Path::new(project.roots.first().context("project root missing")?),std::path::Path::new(source),&result)?;
                    let intent=store.begin_application(&task_id,expected_revision,&digest).await?;
                    let applied=tokio::task::spawn_blocking(move||{
                        // A disconnected/cancelled owner RPC must not release
                        // serialization while its native file writes still run.
                        let _permit=_application;
                        prepared.apply()
                    }).await;
                    match applied {
                        Ok(Ok(()))=>store.settle_application(&intent.id,true,"all accepted artifacts replaced and directory entries synchronized").await?,
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
        result
            .map_err(|error| invalid_params(format!("execution mutation rejected: {error:#}")))?;
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

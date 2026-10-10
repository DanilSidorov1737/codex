//! Owner-only organization commands over the existing app-server transport.

use crate::error_code::internal_error;
use crate::error_code::invalid_params;
use crate::error_code::invalid_request;
use crate::transport::ConnectionOrigin;
use codex_app_server_protocol::ClientResponsePayload;
use codex_app_server_protocol::JSONRPCErrorError;
use codex_app_server_protocol::OrganizationBackupResponse;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationListResponse;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::OrganizationMutateResponse;
use codex_protocol::organization::OrganizationMutation;
use codex_state::organization::OrganizationStore;
use codex_thread_store::ThreadStore;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::OnceCell;

pub(crate) struct OrganizationRequestProcessor {
    sqlite_home: PathBuf,
    store: OnceCell<OrganizationStore>,
    thread_store: Arc<dyn ThreadStore>,
}

impl OrganizationRequestProcessor {
    pub(crate) fn new(sqlite_home: PathBuf, thread_store: Arc<dyn ThreadStore>) -> Self {
        Self {
            sqlite_home,
            store: OnceCell::new(),
            thread_store,
        }
    }

    async fn store(&self) -> Result<&OrganizationStore, JSONRPCErrorError> {
        self.store
            .get_or_try_init(|| async { OrganizationStore::open(&self.sqlite_home).await })
            .await
            .map_err(|error| {
                internal_error(format!(
                    "organization state unavailable; preserve and restore it: {error:#}"
                ))
            })
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

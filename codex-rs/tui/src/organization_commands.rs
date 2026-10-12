//! Thin organization clients using the existing daemon/session bootstrap.

use crate::session_archive_commands::SessionArchiveCommandOptions;
use crate::session_archive_commands::start_app_server_for_session_command;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::OrganizationBackupParams;
use codex_app_server_protocol::OrganizationExecutionMutateParams;
use codex_app_server_protocol::OrganizationExecutionReadParams;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationMemorySearchParams;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::OrganizationScheduleListParams;
use codex_app_server_protocol::OrganizationScheduleMutateParams;
use codex_app_server_protocol::OrganizationWorkflowListParams;
use codex_app_server_protocol::OrganizationWorkflowMutateParams;
use codex_app_server_protocol::OrganizationWorkflowPreviewParams;
use codex_app_server_protocol::ProjectCreateParams;
use codex_utils_home_dir::find_codex_home;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;

pub enum OrganizationCommandAction {
    Inventory,
    Report(codex_app_server_protocol::OrganizationReportParams),
    List(OrganizationListParams),
    MemorySearch(OrganizationMemorySearchParams),
    Mutate(OrganizationMutateParams),
    Backup,
    CreateProject(ProjectCreateParams),
    ExecutionRead(OrganizationExecutionReadParams),
    ExecutionMutate(OrganizationExecutionMutateParams),
    ScheduleList(OrganizationScheduleListParams),
    ScheduleMutate(OrganizationScheduleMutateParams),
    WorkflowList(OrganizationWorkflowListParams),
    WorkflowPreview(OrganizationWorkflowPreviewParams),
    WorkflowMutate(OrganizationWorkflowMutateParams),
}

pub async fn run_organization_command(
    action: OrganizationCommandAction,
    options: SessionArchiveCommandOptions,
) -> Result<serde_json::Value> {
    if options.explicit_remote_endpoint.is_some() {
        return Err(eyre!(
            "organization administration uses the protected local owner connection"
        ));
    }
    let codex_home = find_codex_home()?;
    let mut app_server =
        start_app_server_for_session_command(options, codex_home.to_path_buf()).await?;
    if matches!(
        &action,
        OrganizationCommandAction::ExecutionMutate(_)
            | OrganizationCommandAction::ScheduleMutate(_)
            | OrganizationCommandAction::WorkflowMutate(_)
    ) && app_server.uses_embedded_app_server()
    {
        return Err(eyre!(
            "organization execution requires the detached local daemon; start `codex app-server daemon start` with this configuration first"
        ));
    }
    let request_id = app_server.next_request_id();
    let request = match action {
        OrganizationCommandAction::Inventory => ClientRequest::OrganizationInventory {
            request_id,
            params: codex_app_server_protocol::OrganizationInventoryParams {},
        },
        OrganizationCommandAction::Report(params) => {
            ClientRequest::OrganizationReport { request_id, params }
        }
        OrganizationCommandAction::List(params) => {
            ClientRequest::OrganizationList { request_id, params }
        }
        OrganizationCommandAction::Mutate(params) => {
            ClientRequest::OrganizationMutate { request_id, params }
        }
        OrganizationCommandAction::Backup => ClientRequest::OrganizationBackup {
            request_id,
            params: OrganizationBackupParams {},
        },
        OrganizationCommandAction::CreateProject(params) => {
            ClientRequest::ProjectCreate { request_id, params }
        }
        OrganizationCommandAction::MemorySearch(params) => {
            ClientRequest::OrganizationMemorySearch { request_id, params }
        }
        OrganizationCommandAction::ExecutionRead(params) => {
            ClientRequest::OrganizationExecutionRead { request_id, params }
        }
        OrganizationCommandAction::ExecutionMutate(params) => {
            ClientRequest::OrganizationExecutionMutate { request_id, params }
        }
        OrganizationCommandAction::WorkflowList(params) => {
            ClientRequest::OrganizationWorkflowList { request_id, params }
        }
        OrganizationCommandAction::WorkflowPreview(params) => {
            ClientRequest::OrganizationWorkflowPreview { request_id, params }
        }
        OrganizationCommandAction::WorkflowMutate(params) => {
            ClientRequest::OrganizationWorkflowMutate { request_id, params }
        }
        OrganizationCommandAction::ScheduleList(params) => {
            ClientRequest::OrganizationScheduleList { request_id, params }
        }
        OrganizationCommandAction::ScheduleMutate(params) => {
            ClientRequest::OrganizationScheduleMutate { request_id, params }
        }
    };
    app_server
        .request_handle()
        .request(request)
        .await?
        .map_err(|error| eyre!("{} (RPC {})", error.message, error.code))
}

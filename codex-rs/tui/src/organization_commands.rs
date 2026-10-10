//! Thin organization clients using the existing daemon/session bootstrap.

use crate::session_archive_commands::SessionArchiveCommandOptions;
use crate::session_archive_commands::start_app_server_for_session_command;
use codex_app_server_protocol::ClientRequest;
use codex_app_server_protocol::OrganizationBackupParams;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::ProjectCreateParams;
use codex_utils_home_dir::find_codex_home;
use color_eyre::eyre::Result;
use color_eyre::eyre::eyre;

pub enum OrganizationCommandAction {
    List(OrganizationListParams),
    Mutate(OrganizationMutateParams),
    Backup,
    CreateProject(ProjectCreateParams),
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
    let request_id = app_server.next_request_id();
    let request = match action {
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
    };
    app_server
        .request_handle()
        .request(request)
        .await?
        .map_err(|error| eyre!("{} (RPC {})", error.message, error.code))
}

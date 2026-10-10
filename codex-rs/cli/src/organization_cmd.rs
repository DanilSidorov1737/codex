use crate::SessionArchiveConfigOverrides;
use crate::finalize_session_archive_interactive;
use clap::Parser;
use codex_app_server_protocol::OrganizationExecutionMutateParams;
use codex_app_server_protocol::OrganizationExecutionReadParams;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::ProjectCreateParams;
use codex_app_server_protocol::ProjectRoot;
use codex_arg0::Arg0DispatchPaths;
use codex_tui::Cli as TuiCli;
use codex_tui::OrganizationCommandAction;
use codex_utils_absolute_path::AbsolutePathBuf;
use codex_utils_cli::CliConfigOverrides;
use std::path::PathBuf;
use tokio::io::AsyncReadExt;

#[derive(Debug, Parser)]
pub(crate) struct OrganizationCommand {
    #[command(subcommand)]
    command: Action,
    #[clap(flatten)]
    pub(crate) config_overrides: SessionArchiveConfigOverrides,
}

#[derive(Debug, clap::Subcommand)]
enum Action {
    /// List nodes, roles, goals, projects, tasks or memberships as JSON.
    List {
        kind: String,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// Apply a typed owner mutation read from a JSON file.
    Mutate { file: PathBuf },
    /// Inspect durable attempt state, pending approvals, action receipts and evidence.
    ExecutionRead { task_id: String },
    /// Authorize, stop, decide an exact approval or review a result from JSON.
    /// Background execution requires the detached local daemon.
    Execute { file: PathBuf },
    /// Create metadata using the existing Codex project API, then link it with mutate.
    ProjectCreate {
        name: String,
        #[arg(long)]
        root: PathBuf,
        #[arg(long)]
        idempotency_key: String,
    },
    /// Create a verified consistent snapshot inside the protected state directory.
    Backup,
    /// Restore a verified snapshot into a fresh home; never replace live state.
    Restore {
        #[arg(long)]
        backup: PathBuf,
        #[arg(long)]
        destination: PathBuf,
    },
}

pub(crate) async fn run(
    command: OrganizationCommand,
    interactive: TuiCli,
    overrides: CliConfigOverrides,
    arg0_paths: Arg0DispatchPaths,
) -> anyhow::Result<String> {
    if let Action::Restore {
        backup,
        destination,
    } = &command.command
    {
        let restored =
            codex_state::organization::OrganizationStore::restore_into(backup, destination).await?;
        let output = serde_json::to_string_pretty(
            &serde_json::json!({"path": restored.path(), "databaseId": restored.database_id().await?}),
        )?;
        restored.close().await;
        return Ok(output);
    }
    let action = match command.command {
        Action::List {
            kind,
            cursor,
            limit,
        } => OrganizationCommandAction::List(OrganizationListParams {
            kind: serde_json::from_value(serde_json::Value::String(kind))?,
            cursor,
            limit: Some(limit),
        }),
        Action::Mutate { file } => {
            let bytes = read_mutation(file).await?;
            OrganizationCommandAction::Mutate(OrganizationMutateParams {
                mutation: serde_json::from_slice(&bytes)?,
            })
        }
        Action::ExecutionRead { task_id } => {
            OrganizationCommandAction::ExecutionRead(OrganizationExecutionReadParams { task_id })
        }
        Action::Execute { file } => {
            anyhow::ensure!(
                !interactive.no_daemon,
                "background task execution requires the local daemon; remove --no-daemon"
            );
            let bytes = read_mutation(file).await?;
            OrganizationCommandAction::ExecutionMutate(OrganizationExecutionMutateParams {
                mutation: serde_json::from_slice(&bytes)?,
            })
        }
        Action::ProjectCreate {
            name,
            root,
            idempotency_key,
        } => OrganizationCommandAction::CreateProject(ProjectCreateParams {
            name,
            roots: vec![ProjectRoot {
                path: AbsolutePathBuf::from_absolute_path(root.canonicalize()?)?,
            }],
            metadata: None,
            idempotency_key,
        }),
        Action::Backup => OrganizationCommandAction::Backup,
        Action::Restore { .. } => unreachable!("restore handled without server startup"),
    };
    let cli =
        finalize_session_archive_interactive(interactive, overrides, command.config_overrides);
    let value = codex_tui::run_organization_command(
        action,
        codex_tui::SessionArchiveCommandOptions {
            cli,
            arg0_paths,
            explicit_remote_endpoint: None,
        },
    )
    .await
    .map_err(|error| anyhow::anyhow!("{error:#}"))?;
    Ok(serde_json::to_string_pretty(&value)?)
}

async fn read_mutation(path: PathBuf) -> anyhow::Result<Vec<u8>> {
    let file = tokio::fs::File::open(path).await?;
    let metadata = file.metadata().await?;
    anyhow::ensure!(
        metadata.is_file() && metadata.len() <= 128 * 1024,
        "mutation must be a JSON file of at most 128 KiB"
    );
    let mut bytes = Vec::new();
    file.take(128 * 1024 + 1).read_to_end(&mut bytes).await?;
    anyhow::ensure!(bytes.len() <= 128 * 1024, "mutation exceeds 128 KiB");
    Ok(bytes)
}

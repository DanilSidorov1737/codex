use crate::SessionArchiveConfigOverrides;
use crate::finalize_session_archive_interactive;
use clap::Parser;
use codex_app_server_protocol::OrganizationExecutionMutateParams;
use codex_app_server_protocol::OrganizationExecutionReadParams;
use codex_app_server_protocol::OrganizationListParams;
use codex_app_server_protocol::OrganizationMemorySearchParams;
use codex_app_server_protocol::OrganizationMutateParams;
use codex_app_server_protocol::OrganizationNoteScope;
use codex_app_server_protocol::OrganizationScheduleListParams;
use codex_app_server_protocol::OrganizationScheduleMutateParams;
use codex_app_server_protocol::OrganizationWorkflowKind;
use codex_app_server_protocol::OrganizationWorkflowListParams;
use codex_app_server_protocol::OrganizationWorkflowMutateParams;
use codex_app_server_protocol::OrganizationWorkflowPreviewParams;
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
    /// Offline preview of effect-free, unreferenced failed workspaces; never delete results/bases.
    RetentionPreview {
        project_id: String,
        #[arg(long)]
        before: i64,
        #[arg(long, required = true)]
        offline: bool,
    },
    /// Apply an exact saved retention preview while all runtime/editor writers are stopped.
    RetentionArchive {
        file: PathBuf,
        #[arg(long, required = true)]
        offline: bool,
    },
    /// Inspect an interrupted archive's exact paths/hash; never repeat its rename.
    RetentionReconcile {
        attempt_id: String,
        #[arg(long, required = true)]
        offline: bool,
    },
    /// Offline bundle of business/runtime state and isolated task workspaces.
    BundleBackup {
        #[arg(long)]
        destination: PathBuf,
        /// Confirm all CLI/runtime writers are stopped (required).
        #[arg(long, required = true)]
        offline: bool,
    },
    /// Verify a complete recovery bundle and restore into a fresh home.
    BundleRestore {
        #[arg(long)]
        bundle: PathBuf,
        #[arg(long)]
        destination: PathBuf,
    },
    /// Read canonical task/inbox/check/resource summaries; no inference is started.
    Inventory,
    Report {
        #[arg(long)]
        project_id: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// List saved workflow templates or invocations and canonical task states.
    WorkflowList {
        #[arg(default_value = "templates")]
        kind: String,
        #[arg(long)]
        project_id: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// Validate scopes and capture the exact template/project/agent run preview.
    WorkflowPreview { file: PathBuf },
    /// Save a template or explicitly authorize an immutable run preview from JSON.
    WorkflowMutate { file: PathBuf },
    /// List finite recurring schedules, usage and recent occurrence task IDs.
    ScheduleList {
        #[arg(long)]
        project_id: Option<String>,
        #[arg(long)]
        cursor: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// Create or pause/resume/cancel a bounded recurring schedule from owner JSON.
    ScheduleMutate { file: PathBuf },
    /// Search scoped durable notes with a literal Unicode-aware query (empty lists recent notes).
    MemorySearch {
        project_id: String,
        #[arg(default_value = "")]
        query: String,
        /// Search this agent's private notes instead of project-shared notes.
        #[arg(long)]
        agent_id: Option<String>,
        /// Search owner-promoted team notes visible to this agent.
        #[arg(long, requires = "agent_id")]
        team: bool,
    },
    /// List nodes, roles, goals, projects, tasks, memberships, presets, controls, controlHistory or notes as JSON.
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
    if matches!(
        &command.command,
        Action::RetentionPreview { .. }
            | Action::RetentionArchive { .. }
            | Action::RetentionReconcile { .. }
    ) {
        let home = codex_core::config::find_codex_home()?;
        let socket = codex_app_server::app_server_control_socket_path(&home)?;
        anyhow::ensure!(
            codex_app_server_daemon::probe_app_server_version(socket.as_path())
                .await
                .is_err(),
            "stop runtime/editor writers before offline retention"
        );
        let sqlite_home = std::env::var_os("CODEX_SQLITE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.to_path_buf());
        let store = codex_state::organization::OrganizationStore::open(&sqlite_home).await?;
        let result=async {
            match &command.command {
                Action::RetentionPreview {project_id,before,offline}=> {
                    anyhow::ensure!(*offline,"offline confirmation required");
                    Ok(serde_json::to_string_pretty(&store.retention_preview(project_id,*before).await?)?)
                },
                Action::RetentionArchive {file,offline}=> {
                    anyhow::ensure!(*offline,"offline confirmation required");
                    let plan:codex_state::organization::WorkspaceRetentionPlan=serde_json::from_slice(&read_mutation(file.clone()).await?)?;
                    store.archive_workspaces(&plan).await?;
                    Ok(serde_json::to_string_pretty(&serde_json::json!({"archived":plan.candidates.len(),"bytesReclaimed":0,"preserves":"canonical ledgers and cold workspace bytes; no result/base deletion"}))?)
                },
                Action::RetentionReconcile {attempt_id,offline}=> {
                    anyhow::ensure!(*offline,"offline confirmation required");
                    store.reconcile_workspace_archive(attempt_id).await?;
                    Ok(serde_json::to_string_pretty(&serde_json::json!({"attemptId":attempt_id,"archive":"completed from verified evidence; no rename replay"}))?)
                },
                _=>unreachable!(),
            }
        }.await;
        store.close().await;
        return result;
    }
    match &command.command {
        Action::BundleBackup {
            destination,
            offline,
        } => {
            anyhow::ensure!(
                *offline,
                "stop all runtime writers before an offline bundle"
            );
            let home = codex_core::config::find_codex_home()?;
            let socket = codex_app_server::app_server_control_socket_path(&home)?;
            anyhow::ensure!(
                codex_app_server_daemon::probe_app_server_version(socket.as_path())
                    .await
                    .is_err(),
                "stop the app-server before an offline bundle"
            );
            let sqlite_home = std::env::var_os("CODEX_SQLITE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| home.to_path_buf());
            let store = codex_state::organization::OrganizationStore::open(&sqlite_home).await?;
            let result = store.backup_bundle(&home, destination).await;
            store.close().await;
            return Ok(serde_json::to_string_pretty(
                &serde_json::json!({"bundle": result?}),
            )?);
        }
        Action::BundleRestore {
            bundle,
            destination,
        } => {
            let store =
                codex_state::organization::OrganizationStore::restore_bundle(bundle, destination)
                    .await?;
            let output = serde_json::to_string_pretty(
                &serde_json::json!({"path": store.path(), "databaseId": store.database_id().await?}),
            )?;
            store.close().await;
            return Ok(output);
        }
        _ => {}
    }
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
        Action::Inventory => OrganizationCommandAction::Inventory,
        Action::Report {
            project_id,
            cursor,
            limit,
        } => {
            OrganizationCommandAction::Report(codex_app_server_protocol::OrganizationReportParams {
                project_id,
                cursor,
                limit: Some(limit),
            })
        }
        Action::WorkflowList {
            kind,
            project_id,
            cursor,
            limit,
        } => {
            let kind: OrganizationWorkflowKind = serde_json::from_value(kind.into())?;
            OrganizationCommandAction::WorkflowList(OrganizationWorkflowListParams {
                kind,
                project_id,
                cursor,
                limit: Some(limit),
            })
        }
        Action::WorkflowPreview { file } => {
            OrganizationCommandAction::WorkflowPreview(OrganizationWorkflowPreviewParams {
                request: serde_json::from_slice(&read_mutation(file).await?)?,
            })
        }
        Action::WorkflowMutate { file } => {
            anyhow::ensure!(
                !interactive.no_daemon,
                "workflow mutations require the detached local daemon; remove --no-daemon"
            );
            OrganizationCommandAction::WorkflowMutate(OrganizationWorkflowMutateParams {
                mutation: serde_json::from_slice(&read_mutation(file).await?)?,
            })
        }
        Action::ScheduleList {
            project_id,
            cursor,
            limit,
        } => OrganizationCommandAction::ScheduleList(OrganizationScheduleListParams {
            project_id,
            cursor,
            limit: Some(limit),
        }),
        Action::ScheduleMutate { file } => {
            anyhow::ensure!(
                !interactive.no_daemon,
                "recurring schedules require the detached local daemon; remove --no-daemon"
            );
            let bytes = read_mutation(file).await?;
            OrganizationCommandAction::ScheduleMutate(OrganizationScheduleMutateParams {
                mutation: serde_json::from_slice(&bytes)?,
            })
        }
        Action::MemorySearch {
            project_id,
            query,
            agent_id,
            team,
        } => OrganizationCommandAction::MemorySearch(OrganizationMemorySearchParams {
            project_id,
            scope: if team {
                OrganizationNoteScope::Team
            } else if agent_id.is_some() {
                OrganizationNoteScope::Agent
            } else {
                OrganizationNoteScope::Project
            },
            agent_id,
            query,
        }),
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
        Action::BundleBackup { .. } | Action::BundleRestore { .. } => {
            unreachable!("bundles handled offline")
        }
        Action::RetentionPreview { .. }
        | Action::RetentionArchive { .. }
        | Action::RetentionReconcile { .. } => unreachable!("retention handled offline"),
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

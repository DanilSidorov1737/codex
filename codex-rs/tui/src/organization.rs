//! Owner organization layouts. All authority stays in the existing app-server.
//! One bounded loader serves every preset; sleeping definitions create no model sessions.

use anyhow::Context;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol::*;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use ratatui::buffer::Buffer;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Clear;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Widget;
use ratatui::widgets::Wrap;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc;

use crate::organization_editor::Editor;
use crate::organization_preview::FollowFiles;
use crate::organization_preview::Preview;
use crate::tui::FrameRequester;
use crate::tui::Tui;
use crate::tui::TuiEvent;

#[path = "organization_artifacts.rs"]
mod artifacts;
#[path = "organization_form.rs"]
mod form;
#[path = "organization_graph.rs"]
mod graph;
#[path = "organization_palette.rs"]
mod palette;
#[path = "organization_report.rs"]
mod reporting;
#[path = "organization_view.rs"]
mod view;
#[path = "organization_workflow.rs"]
mod workflow;
use workflow::workflow_id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum Preset {
    Console,
    Workspace,
    Projects,
    Operations,
    Review,
}

impl Preset {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "console" => Some(Self::Console),
            "workspace" => Some(Self::Workspace),
            "projects" => Some(Self::Projects),
            "operations" => Some(Self::Operations),
            "review" => Some(Self::Review),
            _ => None,
        }
    }
    fn index(self) -> usize {
        self as usize
    }
    fn title(self) -> &'static str {
        ["Console", "Workspace", "Projects", "Operations", "Review"][self.index()]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum Pane {
    Sidebar,
    Main,
    Inspector,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PaneSettings {
    inspector: bool,
    width: u16,
    inspector_focus: bool,
    vim_keys: bool,
    sidebar: bool,
    sidebar_width: u16,
    actions: bool,
    order: [Pane; 3],
}
impl Default for PaneSettings {
    fn default() -> Self {
        Self {
            inspector: true,
            width: 65,
            inspector_focus: false,
            vim_keys: true,
            sidebar: true,
            sidebar_width: 24,
            actions: true,
            order: [Pane::Sidebar, Pane::Main, Pane::Inspector],
        }
    }
}

impl PaneSettings {
    fn regions(&self, area: Rect) -> Vec<(Pane, Rect)> {
        let visible: Vec<_> = self
            .order
            .into_iter()
            .filter(|pane| match pane {
                Pane::Sidebar => self.sidebar && area.width >= 110,
                Pane::Inspector => self.inspector,
                Pane::Main => true,
            })
            .collect();
        let regions = Layout::default()
            .direction(if area.width < 80 {
                Direction::Vertical
            } else {
                Direction::Horizontal
            })
            .constraints(visible.iter().map(|pane| match pane {
                Pane::Sidebar => Constraint::Length(self.sidebar_width),
                Pane::Main => Constraint::Fill(self.width),
                Pane::Inspector => Constraint::Fill(100 - self.width),
            }))
            .split(area);
        visible.into_iter().zip(regions.iter().copied()).collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Preferences {
    preset: Preset,
    panes: [PaneSettings; 5],
    graph: graph::Settings,
    views: Vec<SavedView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    executive_contact: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SavedView {
    name: String,
    preset: Preset,
    panes: [PaneSettings; 5],
    graph: graph::Settings,
    search: String,
    selected: Option<String>,
    collapsed: BTreeSet<String>,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            preset: Preset::Operations,
            panes: std::array::from_fn(|_| PaneSettings::default()),
            graph: graph::Settings::default(),
            views: vec![],
            executive_contact: None,
        }
    }
}

impl Preferences {
    fn load(path: &Path) -> std::io::Result<Self> {
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
        }
        let mut file = match options.open(path) {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(e),
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > 65536 {
            return Err(std::io::Error::other("invalid layout preferences file"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if metadata.nlink() != 1
                || metadata.mode() & 0o077 != 0
                || metadata.uid() != unsafe { libc::geteuid() }
            {
                return Err(std::io::Error::other(
                    "layout preferences must be private and owner-controlled",
                ));
            }
        }
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let mut prefs: Self = serde_json::from_str(&content)?;
        if prefs
            .executive_contact
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 256 || id.contains('\0'))
        {
            return Err(std::io::Error::other("invalid executive contact"));
        }
        if prefs.views.len() > 8
            || prefs
                .views
                .iter()
                .any(|v| v.name.len() > 64 || v.search.len() > 256 || v.collapsed.len() > 64)
        {
            return Err(std::io::Error::other("saved view bounds exceeded"));
        }
        for pane in prefs
            .panes
            .iter_mut()
            .chain(prefs.views.iter_mut().flat_map(|v| v.panes.iter_mut()))
        {
            if ![Pane::Sidebar, Pane::Main, Pane::Inspector]
                .iter()
                .all(|p| pane.order.contains(p))
            {
                return Err(std::io::Error::other(
                    "pane placement must contain each supported pane exactly once",
                ));
            }
            pane.width = pane.width.clamp(25, 75);
            pane.sidebar_width = pane.sidebar_width.clamp(16, 40);
        }
        Ok(prefs)
    }
    fn save(&self, path: &Path) -> std::io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)?;
        if bytes.len() > 65535 {
            return Err(std::io::Error::other("layout preferences exceed 64 KiB"));
        }
        let parent = path
            .parent()
            .ok_or_else(|| std::io::Error::other("missing preferences directory"))?;
        let mut file = tempfile::NamedTempFile::new_in(parent)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(&bytes)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        std::fs::File::open(parent)?.sync_all()
    }
}

#[derive(Default, Clone)]
struct Snapshot {
    records: Vec<OrganizationRecord>,
    schedules: Vec<OrganizationScheduleView>,
    workflows: Vec<OrganizationWorkflowView>,
    report: Option<OrganizationReport>,
    limited: bool,
}
enum Command {
    Setup {
        params: ProjectCreateParams,
        agent: String,
        instructions: String,
        model: String,
        limits: OrganizationSharedCeilings,
    },
    ArtifactLibrary {
        id: String,
        project_id: String,
        query: String,
    },
    NativeTerminal(PathBuf),
    ProjectExplorer(PathBuf),
    OwnerContact(Option<String>),
    NamedView {
        name: String,
        action: String,
    },
    RestoreEditor(Box<crate::organization_editor::Session>, bool),
    BufferProposal(Box<crate::organization_editor::BufferProposal>),
    Refresh,
    MoreRecords,
    Summary(String, reporting::Focus),
    Inventory(String),
    Inspect(String),
    MemorySearch(OrganizationMemorySearchParams),
    Definition(OrganizationMutation),
    Execution(OrganizationExecutionMutation),
    Schedule(OrganizationScheduleMutation),
    Workflow(OrganizationWorkflowMutation),
    WorkflowPreview(OrganizationWorkflowPreviewRequest),
    SeriesPreview {
        request: OrganizationWorkflowPreviewRequest,
        config: OrganizationScheduleConfig,
        source_task: Option<String>,
        supervision: Option<(Box<OrganizationGoal>, String)>,
    },
    Project {
        params: ProjectCreateParams,
        goal_id: Option<String>,
        owner_id: Option<String>,
        criteria: Vec<String>,
    },
}
enum Update {
    Snapshot(Box<Snapshot>),
    ReportText(String, std::result::Result<String, String>),
    Report(
        String,
        std::result::Result<Box<OrganizationReport>, String>,
        reporting::Focus,
    ),
    Memory(
        OrganizationMemorySearchParams,
        std::result::Result<OrganizationNoteSearchResult, String>,
    ),
    Detail(Box<OrganizationExecutionView>),
    Error(String),
    ApplyRejected(String),
    ApplyFinished,
    Changed,
    Saved(OrganizationRecord),
    ScheduleSaved(Box<OrganizationScheduleView>, bool),
    WorkflowSaved(Box<OrganizationWorkflowView>),
    WorkflowPreview(Box<OrganizationWorkflowPreview>),
    SeriesPreview {
        request: OrganizationWorkflowPreviewRequest,
        config: OrganizationScheduleConfig,
        source_task: Option<String>,
        supervision: Option<(Box<OrganizationGoal>, String)>,
        preview: Box<OrganizationWorkflowPreview>,
        trigger: OrganizationWorkflowTrigger,
    },
}

fn request_id() -> RequestId {
    // Background requests share the startup transport and can outlive a closed dashboard.
    RequestId::String(format!("organization-{}", uuid::Uuid::new_v4()))
}

fn utc_time(seconds: i64) -> String {
    chrono::DateTime::from_timestamp(seconds, 0)
        .map(|time| time.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| format!("{seconds} UTC epoch"))
}

fn cache_valid(previous: Option<&OrganizationReport>, report: &OrganizationReport) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if previous.runtime_resources.as_ref().map(|r| r.process_id)
        != report.runtime_resources.as_ref().map(|r| r.process_id)
    {
        return false;
    }
    if report.sequence == previous.sequence {
        return true;
    }
    let Some(events) = &report.audit_events else {
        return false;
    };
    report.sequence > previous.sequence
        && events
            .iter()
            .filter(|e| e.sequence > previous.sequence)
            .count() as i64
            == report.sequence - previous.sequence
        && !events
            .iter()
            .any(|e| e.sequence > previous.sequence && e.actor == "owner")
}
async fn load(
    handle: &AppServerRequestHandle,
    cache: &mut Option<Snapshot>,
    extra_pages: usize,
    force: bool,
) -> anyhow::Result<Snapshot> {
    let report = reporting::load(handle, None).await?;
    let reusable = !force && cache_valid(cache.as_ref().and_then(|s| s.report.as_ref()), &report);
    if reusable
        && cache
            .as_ref()
            .and_then(|s| s.report.as_ref())
            .is_some_and(|old| old.sequence == report.sequence)
    {
        let mut snapshot = cache.as_ref().context("cache unavailable")?.clone();
        snapshot.report = Some(report);
        *cache = Some(snapshot.clone());
        return Ok(snapshot);
    }
    let mut records = Vec::new();
    let mut limited = reusable && cache.as_ref().is_some_and(|s| s.limited);
    for kind in [
        OrganizationRecordKind::Nodes,
        OrganizationRecordKind::Roles,
        OrganizationRecordKind::Goals,
        OrganizationRecordKind::Projects,
        OrganizationRecordKind::Tasks,
        OrganizationRecordKind::Memberships,
        OrganizationRecordKind::Presets,
        OrganizationRecordKind::Controls,
    ] {
        if reusable
            && !matches!(
                kind,
                OrganizationRecordKind::Tasks | OrganizationRecordKind::Controls
            )
        {
            if let Some(cached) = cache.as_ref() {
                records.extend(
                    cached
                        .records
                        .iter()
                        .filter(|r| {
                            matches!(
                                (kind, r),
                                (OrganizationRecordKind::Nodes, OrganizationRecord::Node(_))
                                    | (OrganizationRecordKind::Roles, OrganizationRecord::Role(_))
                                    | (OrganizationRecordKind::Goals, OrganizationRecord::Goal(_))
                                    | (
                                        OrganizationRecordKind::Projects,
                                        OrganizationRecord::Project(_)
                                    )
                                    | (
                                        OrganizationRecordKind::Memberships,
                                        OrganizationRecord::Membership(_)
                                    )
                                    | (
                                        OrganizationRecordKind::Presets,
                                        OrganizationRecord::Preset(_)
                                    )
                                    | (
                                        OrganizationRecordKind::Controls,
                                        OrganizationRecord::Control(_)
                                    )
                            )
                        })
                        .cloned(),
                );
            }
            continue;
        }
        let mut cursor = None;
        let pages = if matches!(
            kind,
            OrganizationRecordKind::Nodes | OrganizationRecordKind::Goals
        ) {
            100
        } else {
            10
        } + extra_pages;
        for page in 0..pages {
            let response: OrganizationListResponse = handle
                .request_typed(ClientRequest::OrganizationList {
                    request_id: request_id(),
                    params: OrganizationListParams {
                        kind,
                        cursor: cursor.clone(),
                        limit: Some(200),
                    },
                })
                .await?;
            records.extend(response.data);
            if response.next_cursor.is_none() {
                break;
            }
            anyhow::ensure!(
                response.next_cursor != cursor,
                "organization cursor did not advance"
            );
            if page + 1 == pages {
                limited = true;
                break;
            }
            cursor = response.next_cursor;
        }
    }
    let mut schedules = Vec::new();
    let mut cursor = None;
    for page in 0..(10 + extra_pages) {
        let response: OrganizationScheduleListResponse = handle
            .request_typed(ClientRequest::OrganizationScheduleList {
                request_id: request_id(),
                params: OrganizationScheduleListParams {
                    project_id: None,
                    cursor: cursor.clone(),
                    limit: Some(200),
                },
            })
            .await?;
        schedules.extend(response.data);
        if response.next_cursor.is_none() {
            break;
        }
        anyhow::ensure!(
            response.next_cursor != cursor,
            "schedule cursor did not advance"
        );
        if page + 1 == 10 + extra_pages {
            limited = true;
            break;
        }
        cursor = response.next_cursor;
    }
    let mut workflows = Vec::new();
    for kind in [
        OrganizationWorkflowKind::Templates,
        OrganizationWorkflowKind::Runs,
        OrganizationWorkflowKind::Limits,
        OrganizationWorkflowKind::Series,
    ] {
        let mut cursor = None;
        for page in 0..(10 + extra_pages) {
            let response: OrganizationWorkflowListResponse = handle
                .request_typed(ClientRequest::OrganizationWorkflowList {
                    request_id: request_id(),
                    params: OrganizationWorkflowListParams {
                        kind,
                        project_id: None,
                        cursor: cursor.clone(),
                        limit: Some(200),
                    },
                })
                .await?;
            workflows.extend(response.data);
            if response.next_cursor.is_none() {
                break;
            }
            anyhow::ensure!(
                response.next_cursor != cursor,
                "workflow cursor did not advance"
            );
            if page + 1 == 10 + extra_pages {
                limited = true;
                break;
            }
            cursor = response.next_cursor;
        }
    }
    let snapshot = Snapshot {
        records,
        schedules,
        workflows,
        report: Some(report),
        limited,
    };
    *cache = Some(snapshot.clone());
    Ok(snapshot)
}

async fn worker(
    handle: AppServerRequestHandle,
    frame: FrameRequester,
    mut commands: mpsc::Receiver<Command>,
    updates: mpsc::Sender<Update>,
) {
    let mut selected: Option<String> = None;
    let mut cache = None;
    let mut extra_pages = 0;
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let command = tokio::select! { command = commands.recv() => { let Some(command) = command else { break }; command }, _ = interval.tick() => Command::Refresh };
        let memory_request = match &command {
            Command::MemorySearch(params) => Some(params.clone()),
            _ => None,
        };
        let report_id = match &command {
            Command::Summary(id, _)
            | Command::Inventory(id)
            | Command::ArtifactLibrary { id, .. } => Some(id.clone()),
            _ => None,
        };
        let mut apply_rejected = false;
        let operation = async {
            match command {
                Command::Setup {
                    params,
                    agent,
                    instructions,
                    model,
                    limits,
                } => {
                    // Pause before definitions; never replay partial setup.
                    for kind in [
                        OrganizationRecordKind::Nodes,
                        OrganizationRecordKind::Projects,
                        OrganizationRecordKind::Controls,
                    ] {
                        let response: OrganizationListResponse = handle
                            .request_typed(ClientRequest::OrganizationList {
                                request_id: request_id(),
                                params: OrganizationListParams {
                                    kind,
                                    cursor: None,
                                    limit: Some(1),
                                },
                            })
                            .await?;
                        anyhow::ensure!(
                            response.data.is_empty(),
                            "Setup requires an empty organization; inspect saved state and use individual forms to resume partial setup"
                        );
                    }
                    let _: OrganizationMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationMutate {
                            request_id: request_id(),
                            params: OrganizationMutateParams {
                                mutation: OrganizationMutation::SetControl {
                                    expected_revision: None,
                                    value: OrganizationControlValue::Global {
                                        paused: true,
                                        limits: Some(limits),
                                    },
                                },
                            },
                        })
                        .await?;
                    let role: OrganizationMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationMutate {
                            request_id: request_id(),
                            params: OrganizationMutateParams {
                                mutation: OrganizationMutation::CreateRoleVersion {
                                    role_id: None,
                                    expected_version: None,
                                    name: agent.clone(),
                                    instructions,
                                },
                            },
                        })
                        .await?;
                    let Some(OrganizationRecord::Role(role)) = role.record else {
                        anyhow::bail!("Setup role response missing; inspect saved state")
                    };
                    let node: OrganizationMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationMutate {
                            request_id: request_id(),
                            params: OrganizationMutateParams {
                                mutation: OrganizationMutation::PutNode {
                                    id: None,
                                    expected_revision: None,
                                    parent_id: None,
                                    kind: OrganizationNodeKind::Agent,
                                    label: agent,
                                    role_version_id: Some(role.id),
                                    model: Some(model),
                                    enabled: true,
                                },
                            },
                        })
                        .await?;
                    let Some(OrganizationRecord::Node(node)) = node.record else {
                        anyhow::bail!("Setup agent response missing; inspect saved state")
                    };
                    let project: ProjectCreateResponse = handle
                        .request_typed(ClientRequest::ProjectCreate {
                            request_id: request_id(),
                            params,
                        })
                        .await?;
                    let project: OrganizationMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationMutate {
                            request_id: request_id(),
                            params: OrganizationMutateParams {
                                mutation: OrganizationMutation::LinkProject {
                                    project_id: project.project.id,
                                    expected_revision: None,
                                    goal_id: None,
                                    owner_id: Some(node.id),
                                    criteria: vec!["Owner reviews declared task results".into()],
                                },
                            },
                        })
                        .await?;
                    if let Some(record) = project.record {
                        updates.send(Update::Saved(record)).await?;
                    }
                    updates.send(Update::Changed).await?;
                }
                Command::RestoreEditor(..)
                | Command::BufferProposal(..)
                | Command::NativeTerminal(..)
                | Command::ProjectExplorer(..)
                | Command::OwnerContact(..)
                | Command::NamedView { .. } => {
                    anyhow::bail!("native editor actions must stay on the frontend")
                }
                Command::Summary(id, focus) => {
                    let report = reporting::load(&handle, focus.project_id())
                        .await
                        .map(Box::new)
                        .map_err(|error| error.to_string());
                    updates.send(Update::Report(id, report, focus)).await?;
                }
                Command::ArtifactLibrary {
                    id,
                    project_id,
                    query,
                } => {
                    let result = artifacts::catalog(&handle, &project_id, &query)
                        .await
                        .map_err(|e| e.to_string());
                    updates.send(Update::ReportText(id, result)).await?;
                }
                Command::Inventory(id) => {
                    let result = match handle
                        .request_typed::<OrganizationInventoryResponse>(
                            ClientRequest::OrganizationInventory {
                                request_id: request_id(),
                                params: OrganizationInventoryParams {},
                            },
                        )
                        .await
                    {
                        Ok(response) => {
                            match reporting::inventory_advice(&handle, &response.inventory).await {
                                Ok(text) => Ok(clean(&text)),
                                Err(error) => Ok(clean(&format!(
                                    "{}\nModel advice unavailable: {error}",
                                    reporting::inventory_text(&response.inventory)
                                ))),
                            }
                        }
                        Err(error) => Err(error.to_string()),
                    };
                    updates.send(Update::ReportText(id, result)).await?;
                }
                Command::Refresh | Command::MoreRecords => {
                    let more = matches!(command, Command::MoreRecords);
                    if more {
                        extra_pages = (extra_pages + 10).min(100);
                    }
                    updates
                        .send(Update::Snapshot(Box::new(
                            load(&handle, &mut cache, extra_pages, more).await?,
                        )))
                        .await?;
                    if let Some(task_id) = &selected {
                        let response: OrganizationExecutionReadResponse = handle
                            .request_typed(ClientRequest::OrganizationExecutionRead {
                                request_id: request_id(),
                                params: OrganizationExecutionReadParams {
                                    task_id: task_id.clone(),
                                },
                            })
                            .await?;
                        updates
                            .send(Update::Detail(Box::new(response.view)))
                            .await?;
                    }
                }
                Command::Inspect(task_id) => {
                    selected = Some(task_id.clone());
                    let response: OrganizationExecutionReadResponse = handle
                        .request_typed(ClientRequest::OrganizationExecutionRead {
                            request_id: request_id(),
                            params: OrganizationExecutionReadParams { task_id },
                        })
                        .await?;
                    updates
                        .send(Update::Detail(Box::new(response.view)))
                        .await?;
                }
                Command::MemorySearch(params) => {
                    let result = handle
                        .request_typed::<OrganizationMemorySearchResponse>(
                            ClientRequest::OrganizationMemorySearch {
                                request_id: request_id(),
                                params: params.clone(),
                            },
                        )
                        .await
                        .map(|response| response.result)
                        .map_err(|error| error.to_string());
                    updates.send(Update::Memory(params, result)).await?;
                }
                Command::Definition(mutation) => {
                    let response: OrganizationMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationMutate {
                            request_id: request_id(),
                            params: OrganizationMutateParams { mutation },
                        })
                        .await?;
                    if let Some(record) = response.record {
                        updates.send(Update::Saved(record)).await?;
                    }
                    updates.send(Update::Changed).await?;
                }
                Command::Project {
                    params,
                    goal_id,
                    owner_id,
                    criteria,
                } => {
                    let response: ProjectCreateResponse = handle
                        .request_typed(ClientRequest::ProjectCreate {
                            request_id: request_id(),
                            params,
                        })
                        .await?;
                    let project_id = response.project.id;
                    let response: OrganizationMutateResponse = handle.request_typed(ClientRequest::OrganizationMutate {
                        request_id: request_id(),
                        params: OrganizationMutateParams { mutation: OrganizationMutation::LinkProject {
                            project_id: project_id.clone(), expected_revision: None, goal_id, owner_id, criteria,
                        } },
                    }).await.map_err(|e| anyhow::anyhow!("Project {project_id} exists, but linking failed: {e}. Inspect owner state before retrying."))?;
                    if let Some(record) = response.record {
                        updates.send(Update::Saved(record)).await?;
                    }
                    updates.send(Update::Changed).await?;
                }
                Command::Execution(mutation) => {
                    let applying = matches!(&mutation, OrganizationExecutionMutation::Apply { .. });
                    let apply_identity = match &mutation {
                        OrganizationExecutionMutation::Apply {
                            task_id, digest, ..
                        } => Some(
                            serde_json::json!({"application": "notStarted", "taskId": task_id, "digest": digest}),
                        ),
                        _ => None,
                    };
                    let response: OrganizationExecutionMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationExecutionMutate {
                            request_id: request_id(),
                            params: OrganizationExecutionMutateParams { mutation },
                        })
                        .await
                        .inspect_err(|error| {
                            if let codex_app_server_client::TypedRequestError::Server {
                                source,
                                ..
                            } = error
                            {
                                apply_rejected =
                                    apply_identity.is_some() && source.data == apply_identity;
                            }
                        })?;
                    updates
                        .send(Update::Detail(Box::new(response.view)))
                        .await?;
                    updates
                        .send(if applying {
                            Update::ApplyFinished
                        } else {
                            Update::Changed
                        })
                        .await?;
                }
                Command::WorkflowPreview(request) => {
                    let response: OrganizationWorkflowPreviewResponse = handle
                        .request_typed(ClientRequest::OrganizationWorkflowPreview {
                            request_id: request_id(),
                            params: OrganizationWorkflowPreviewParams { request },
                        })
                        .await?;
                    updates
                        .send(Update::WorkflowPreview(Box::new(response.preview)))
                        .await?;
                }
                Command::SeriesPreview {
                    request,
                    config,
                    source_task,
                    supervision,
                } => {
                    let response: OrganizationWorkflowPreviewResponse = handle
                        .request_typed(ClientRequest::OrganizationWorkflowPreview {
                            request_id: request_id(),
                            params: OrganizationWorkflowPreviewParams {
                                request: request.clone(),
                            },
                        })
                        .await?;
                    let trigger = if let Some(task_id) = &source_task {
                        let source: OrganizationExecutionReadResponse = handle
                            .request_typed(ClientRequest::OrganizationExecutionRead {
                                request_id: request_id(),
                                params: OrganizationExecutionReadParams {
                                    task_id: task_id.clone(),
                                },
                            })
                            .await?;
                        anyhow::ensure!(
                            source.view.task.project_id == response.preview.template.project_id,
                            "trigger source must belong to the same project"
                        );
                        let grant = source.view.authorization.ok_or_else(|| {
                            anyhow::anyhow!("source task has no current authorization")
                        })?;
                        anyhow::ensure!(!grant.revoked, "source authorization is revoked");
                        OrganizationWorkflowTrigger::Accepted {
                            task_id: task_id.clone(),
                            authorization_id: grant.id,
                        }
                    } else {
                        OrganizationWorkflowTrigger::Interval
                    };
                    updates
                        .send(Update::SeriesPreview {
                            request,
                            config,
                            source_task,
                            supervision,
                            preview: Box::new(response.preview),
                            trigger,
                        })
                        .await?;
                }
                Command::Workflow(mutation) => {
                    let response: OrganizationWorkflowMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationWorkflowMutate {
                            request_id: request_id(),
                            params: OrganizationWorkflowMutateParams { mutation },
                        })
                        .await?;
                    updates
                        .send(Update::WorkflowSaved(Box::new(response.view)))
                        .await?;
                    updates.send(Update::Changed).await?;
                }
                Command::Schedule(mutation) => {
                    let created = matches!(&mutation, OrganizationScheduleMutation::Create { .. });
                    let response: OrganizationScheduleMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationScheduleMutate {
                            request_id: request_id(),
                            params: OrganizationScheduleMutateParams { mutation },
                        })
                        .await?;
                    updates
                        .send(Update::ScheduleSaved(Box::new(response.view), created))
                        .await?;
                    updates.send(Update::Changed).await?;
                }
            }
            anyhow::Ok(())
        };
        let error = match tokio::time::timeout(Duration::from_secs(15), operation).await {
            Ok(Ok(())) => None,
            Ok(Err(error)) => Some(error.to_string()),
            Err(_) if memory_request.is_some() => {
                Some("Memory search timed out; refresh to retry this read-only request.".into())
            }
            Err(_) => {
                apply_rejected = false;
                Some("Owner API timed out; consequential decisions have an unknown outcome. Refresh before retrying.".into())
            }
        };
        if let Some(error) = error {
            let update = match memory_request {
                Some(params) => Update::Memory(params, Err(error)),
                None if report_id.is_some() => Update::ReportText(
                    report_id.unwrap_or_default(),
                    Err("Read-only report failed or timed out; no execution was authorized".into()),
                ),
                None if apply_rejected => Update::ApplyRejected(error),
                None => Update::Error(error),
            };
            if updates.send(update).await.is_err() {
                break;
            }
        }
        frame.schedule_frame();
    }
}

#[derive(Debug, Clone)]
struct Row {
    id: String,
    label: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Projection {
    #[default]
    Default,
    Goals,
    Schedules,
    Workflows,
    Presets,
    Controls,
    Health,
    Incidents,
}

/// Iterative traversal keeps deep user-defined hierarchies off the call stack.
fn tree(nodes: &[OrganizationNode], collapsed: &BTreeSet<String>, search: &str) -> Vec<Row> {
    let entries: Vec<_> = nodes
        .iter()
        .map(|n| {
            (
                n.id.as_str(),
                n.parent_id.as_deref(),
                n.label.as_str(),
                format!(
                    "{}{}",
                    if n.kind == OrganizationNodeKind::Group {
                        "group"
                    } else {
                        "agent"
                    },
                    if n.enabled {
                        ""
                    } else {
                        "; retired / disabled"
                    }
                ),
            )
        })
        .collect();
    tree_rows(&entries, collapsed, search)
}

fn goal_tree(
    records: &[OrganizationRecord],
    collapsed: &BTreeSet<String>,
    search: &str,
) -> Vec<Row> {
    let goals: BTreeMap<_, _> = records
        .iter()
        .filter_map(|record| match record {
            OrganizationRecord::Goal(goal) => Some((goal.id.as_str(), goal)),
            _ => None,
        })
        .collect();
    let mut progress: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    let projects: BTreeMap<_, _> = records
        .iter()
        .filter_map(|r| match r {
            OrganizationRecord::Project(p) => Some((p.id.as_str(), p.goal_id.as_deref())),
            _ => None,
        })
        .collect();
    for record in records {
        if let OrganizationRecord::Task(task) = record {
            let mut parent = task
                .goal_id
                .as_deref()
                .or_else(|| projects.get(task.project_id.as_str()).copied().flatten());
            let mut visited = BTreeSet::new();
            while let Some(goal) = parent.and_then(|id| goals.get(id)) {
                if !visited.insert(goal.id.as_str()) {
                    break;
                }
                let count = progress.entry(&goal.id).or_default();
                count.0 += 1;
                count.1 += usize::from(task.state == "accepted");
                parent = goal.parent_id.as_deref();
            }
        }
    }
    let entries: Vec<_> = goals
        .values()
        .map(|goal| {
            let (total, accepted) = progress.get(goal.id.as_str()).copied().unwrap_or_default();
            (
                goal.id.as_str(),
                goal.parent_id.as_deref(),
                goal.objective.as_str(),
                format!("goal · {accepted}/{total} tasks accepted; criteria need owner review"),
            )
        })
        .collect();
    tree_rows(&entries, collapsed, search)
}

fn tree_rows(
    nodes: &[(&str, Option<&str>, &str, String)],
    collapsed: &BTreeSet<String>,
    search: &str,
) -> Vec<Row> {
    let mut children: BTreeMap<Option<&str>, Vec<_>> = BTreeMap::new();
    let ids: BTreeSet<&str> = nodes.iter().map(|n| n.0).collect();
    for node in nodes {
        let parent = node.1.filter(|p| ids.contains(p));
        children.entry(parent).or_default().push(node);
    }
    for siblings in children.values_mut() {
        siblings.sort_by_key(|n| (n.2, n.0));
    }
    let mut stack: Vec<_> = children
        .get(&None)
        .into_iter()
        .flatten()
        .rev()
        .map(|n| (*n, 0usize))
        .collect();
    let mut visited = BTreeSet::new();
    let mut rows = Vec::new();
    while let Some((node, depth)) = stack.pop() {
        if !visited.insert(node.0) {
            continue;
        }
        let branch = children.contains_key(&Some(node.0));
        if search.is_empty() || node.2.to_lowercase().contains(search) || node.0.contains(search) {
            rows.push(Row {
                id: node.0.to_string(),
                label: format!(
                    "{}{} {} [{}]",
                    "  ".repeat(depth.min(12)),
                    if branch {
                        if collapsed.contains(node.0) {
                            "▸"
                        } else {
                            "▾"
                        }
                    } else {
                        "·"
                    },
                    node.2,
                    node.3,
                ),
            });
        }
        if (!collapsed.contains(node.0) || !search.is_empty())
            && let Some(list) = children.get(&Some(node.0))
        {
            stack.extend(list.iter().rev().map(|n| (*n, depth + 1)));
        }
    }
    rows
}

pub(crate) struct Dashboard {
    preferences: Preferences,
    path: PathBuf,
    snapshot: Snapshot,
    projection: Projection,
    rows: Vec<Row>,
    graph: graph::Graph,
    selected: Option<String>,
    collapsed: BTreeSet<String>,
    detail: Option<Box<OrganizationExecutionView>>,
    scroll: u16,
    detail_text: String,
    report_text: Option<String>,
    report_request: Option<String>,
    review_diff: Option<(String, String, String)>,
    review_file: usize,
    search: String,
    input: Option<String>,
    memory_request: Option<OrganizationMemorySearchParams>,
    memory_result: Option<std::result::Result<OrganizationNoteSearchResult, String>>,
    changes: bool,
    change_target: Option<OrganizationTask>,
    confirmation: Option<Command>,
    confirmation_text: String,
    selection_confirmation: Option<crate::organization_editor::Selection>,
    message: String,
    message_is_error: bool,
    commands: mpsc::Sender<Command>,
    updates: mpsc::Receiver<Update>,
    worker: tokio::task::JoinHandle<()>,
    editor: Option<Editor>,
    preview: Option<Preview>,
    follow_files: Option<FollowFiles>,
    review_previews: Vec<Preview>,
    editor_area: Rect,
    editor_file: usize,
    following: bool,
    applying: bool,
    provider: String,
    model: String,
    form: Option<form::Form>,
    setup_offered: bool,
    palette: Option<palette::Palette>,
    prices: Option<crate::organization_pricing::Prices>,
    frame: FrameRequester,
    pub(crate) is_done: bool,
}

impl Drop for Dashboard {
    fn drop(&mut self) {
        // Never replace an earlier unsaved recovery checkpoint on automatic exit.
        // Explicit F7 is the owner's choice to advance the checkpoint.
        if !self.applying {
            let roots = self.project_roots();
            if let Some(home) = self.path.parent() {
                let previous = crate::organization_editor::Session::load(home, &roots);
                if (previous.as_ref().is_ok_and(|s| !s.has_unsaved())
                    || !home.join("organization-editor-session.json").exists())
                    && let Some(editor) = &mut self.editor
                    && let Err(error) = editor.save_session(home, &roots)
                {
                    tracing::warn!(%error,"Native editor exit checkpoint retained or unavailable");
                }
            }
        }
        self.worker.abort();
    }
}

impl Dashboard {
    fn agent_readiness_text(&self) -> Option<String> {
        let OrganizationRecord::Node(node) = self.record()? else {
            return None;
        };
        if node.kind != OrganizationNodeKind::Agent {
            return None;
        }
        let mut text = format!(
            "{} · stored readiness evidence\nAgent: {} · revision {}\nModel: {}\n",
            node.label,
            node.id,
            node.revision,
            node.model.as_deref().unwrap_or(&self.model)
        );
        let report = self.snapshot.report.as_ref();
        text.push_str(&format!("Report sequence: {}. Loaded pages may be incomplete; runtime rechecks actual configuration, exact grants and all admission fences. This view grants no execution authority.\n",report.map(|r|r.sequence.to_string()).unwrap_or_else(||"unavailable".into())));
        let nodes: BTreeMap<_, _> = self
            .snapshot
            .records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Node(n) => Some((n.id.as_str(), n)),
                _ => None,
            })
            .collect();
        let mut parent = Some(node.id.as_str());
        let mut visited = BTreeSet::new();
        while let Some(id) = parent {
            if !visited.insert(id) {
                text.push_str("\nHierarchy cycle in loaded data; inspect definitions.\n");
                break;
            }
            let Some(n) = nodes.get(id) else {
                text.push_str(&format!(
                    "\nAncestor {id} not loaded; readiness is unknown.\n"
                ));
                break;
            };
            if !n.enabled {
                text.push_str(&format!(
                    "\nDisabled definition {} · select it and E reviews its enabled state.\n",
                    n.id
                ));
            }
            parent = n.parent_id.as_deref();
        }
        if let Some(role) = &node.role_version_id {
            if !self
                .snapshot
                .records
                .iter()
                .any(|r| matches!(r,OrganizationRecord::Role(r) if &r.id==role))
            {
                text.push_str("\nAssigned role version is not loaded; inspect role definitions before execution.\n");
            }
        } else {
            text.push_str("\nNo custom role assigned; default instructions apply. L / E can create and assign a role separately.\n");
        }
        if self.control("global").is_some_and(|c| {
            matches!(
                c.value,
                OrganizationControlValue::Global { paused: true, .. }
            )
        }) {
            text.push_str("\nOrganization execution paused · F12 reviews Emergency organization pause / resume.\n");
        }
        if report
            .and_then(|r| r.resource_pressure.as_ref())
            .is_some_and(|p| !p.ready)
        {
            text.push_str("\nActual resource pressure blocks admission · Commands: configure actual resource pressure thresholds.\n");
        }
        let tasks: Vec<_> = self
            .snapshot
            .records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Task(t) if t.owner_id == node.id => Some(t),
                _ => None,
            })
            .collect();
        let goals: BTreeMap<_, _> = self
            .snapshot
            .records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Goal(g) => Some((g.id.as_str(), g)),
                _ => None,
            })
            .collect();
        let controls: BTreeMap<_, _> = self
            .snapshot
            .records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Control(c) => Some((c.id.as_str(), c)),
                _ => None,
            })
            .collect();
        if tasks.is_empty() {
            text.push_str("\nNo assigned tasks in the loaded definitions · T drafts a task; s separately reviews its exact authorization.\n");
        }
        for task in tasks.iter().take(64) {
            text.push_str(&format!("\nTask {} · {}\n", task.id, task.state));
            match task.state.as_str() {
                "proposed"=>text.push_str("Missing execution authorization · select task, s reviews a separate finite grant.\n"),
                "waitingApproval"=>text.push_str("Waiting for owner approval · select task, inspect the exact action, a approves / r rejects.\n"),
                "needsReview"=>text.push_str("Waiting for result review · select task, v accepts / c requests changes.\n"),
                "needsReconciliation"=>text.push_str("Uncertain execution · select task, Commands: inspect exact uncertain request/effects. No replay or counter reset.\n"),
                "blocked"=>text.push_str("Verified blocker · select task, H reviews a bounded handoff; Commands routes assistance.\n"),
                "failed"=>text.push_str("Recorded failure · inspect execution evidence and accounting before proposing more work.\n"),
                _=>{},
            }
            for prerequisite in &task.prerequisites {
                match self.snapshot.records.iter().find_map(|r| match r {
                    OrganizationRecord::Task(t) if &t.id == prerequisite => Some(t),
                    _ => None,
                }) {
                    Some(t) if t.state == "accepted" => {}
                    Some(t) => text.push_str(&format!(
                        "Prerequisite {} is {} · inspect/review that task first.\n",
                        t.id, t.state
                    )),
                    None => text.push_str(&format!(
                        "Prerequisite {prerequisite} not loaded; inspect its canonical evidence.\n"
                    )),
                }
            }
            let mut goal_id = task.goal_id.as_deref().or_else(|| {
                self.snapshot.records.iter().find_map(|r| match r {
                    OrganizationRecord::Project(p) if p.id == task.project_id => {
                        p.goal_id.as_deref()
                    }
                    _ => None,
                })
            });
            let mut seen = BTreeSet::new();
            while let Some(id) = goal_id {
                if !seen.insert(id) {
                    text.push_str("Goal cycle in loaded data; inspect definitions.\n");
                    break;
                }
                let Some(current) = goals.get(id) else {
                    text.push_str(&format!("Goal {id} not loaded; readiness unknown.\n"));
                    break;
                };
                if controls.get(format!("goal:{id}").as_str()).is_some_and(|c|matches!(&c.value,OrganizationControlValue::GoalReview {goal,completed:true,..} if goal.as_ref()==*current)) {text.push_str(&format!("Goal {id} owner-completed · select goal, Commands: Review goal criterion evidence; reopening is an explicit owner decision.\n"));}
                if let Some(control) = controls.get(format!("goal-meta:{id}").as_str())
                    && let OrganizationControlValue::GoalMetadata {
                        goal,
                        prerequisites,
                        ..
                    } = &control.value
                {
                    if goal.as_ref() != *current {
                        text.push_str("Captured goal responsibility version changed · review goal responsibility / versioned prerequisites.\n");
                    }
                    for prerequisite in prerequisites {
                        let exact = goals
                            .get(prerequisite.goal.id.as_str())
                            .is_some_and(|g| *g == prerequisite.goal.as_ref());
                        let complete=controls.get(format!("goal:{}",prerequisite.goal.id).as_str()).is_some_and(|c|matches!(&c.value,OrganizationControlValue::GoalReview {goal,completed:true,..} if goal.as_ref()==prerequisite.goal.as_ref()));
                        if !exact || !complete {
                            text.push_str(&format!("Goal prerequisite {} lacks current exact owner completion · review goal prerequisites and criterion evidence.\n",prerequisite.goal.id));
                        }
                    }
                }
                goal_id = current.parent_id.as_deref();
            }
            if self.control(&format!("project:{}",task.project_id)).is_some_and(|c|matches!(c.value,OrganizationControlValue::Project {state,..} if state!=OrganizationProjectState::Active)) {text.push_str("Project is paused or closed · select project, F reviews its lifecycle.\n");}
            if let Some(evidence) =
                report.and_then(|r| r.tasks.iter().find(|t| t.task_id == task.id))
            {
                if evidence.revision != task.revision {
                    text.push_str("Report task revision differs from the loaded definition; refresh before deciding.\n");
                    continue;
                }
                if let Some(budget) = &evidence.budget {
                    if budget.revoked || budget.expires_at <= chrono::Utc::now().timestamp() {
                        text.push_str("Grant revoked or expired · inspect exact limits; continuation requires an explicit owner decision.\n");
                    }
                    if budget.used_tokens >= budget.max_tokens
                        || budget.model_requests >= budget.max_model_requests
                    {
                        text.push_str("Original reported token/request allowance exhausted · inspect accounting; no automatic budget increase.\n");
                    }
                    text.push_str("Nominal held reservations and exact captured versions are in execution details; displayed remaining usage is not a capacity reservation.\n");
                } else if task.state != "proposed" {
                    text.push_str("Grant evidence unavailable in this report · inspect task execution details.\n");
                }
                if evidence.model_requests > evidence.reported_model_requests {
                    text.push_str("Usage receipt pending; admitted requests remain charged. An active request may still be in flight.\n");
                }
            } else {
                text.push_str(
                    "Task report evidence not loaded; budget/approval readiness is unknown.\n",
                );
            }
        }
        Some(clean(&text))
    }
    fn conversation_transcript(&self, id: &str) -> Option<String> {
        let control = self.snapshot.records.iter().find_map(|r| match r {
            OrganizationRecord::Control(c) if c.id == id => Some(c),
            _ => None,
        })?;
        let OrganizationControlValue::Conversation {
            topic,
            agent_id,
            project_id,
            entries,
            ..
        } = &control.value
        else {
            return None;
        };
        let agent = self
            .snapshot
            .records
            .iter()
            .find_map(|r| match r {
                OrganizationRecord::Node(n) if n.id == *agent_id => Some(n.label.as_str()),
                _ => None,
            })
            .unwrap_or(agent_id);
        let mut text = format!(
            "{topic}\nAgent: {agent}\nProject: {project_id}\nEach message uses a separately authorized planning task. Replies are model content; review proposed work before granting execution.\n"
        );
        for entry in entries {
            let state = self
                .snapshot
                .records
                .iter()
                .find_map(|r| match r {
                    OrganizationRecord::Task(t) if t.id == entry.task_id => Some(t.state.as_str()),
                    _ => None,
                })
                .unwrap_or("state not loaded");
            text.push_str(&format!(
                "\nOwner\n{}\n\n{agent} · {state}\n{}\nTask: {}\n",
                entry.prompt,
                entry.reply.as_deref().unwrap_or(
                    "No recorded reply; inspect/authorize the planning draft separately"
                ),
                entry.task_id
            ));
        }
        Some(clean(&text))
    }
    fn project_roots(&self) -> Vec<PathBuf> {
        self.snapshot
            .records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Project(p) => Some(p.roots.iter().map(PathBuf::from)),
                _ => None,
            })
            .flatten()
            .collect()
    }

    fn editor_session(&mut self, restore: bool) {
        let result = (|| -> anyhow::Result<()> {
            anyhow::ensure!(
                !self.applying,
                "Application outcome pending; editor actions wait for reconciliation"
            );
            let roots = self.project_roots();
            let home = self.path.parent().context("owner home missing")?;
            if restore {
                let session = crate::organization_editor::Session::load(home, &roots)?;
                self.form = Some(form::Form::new(
                    form::Kind::RestoreEditor(Box::new(session)),
                    &self.snapshot.records,
                    self.record(),
                    &self.model,
                ));
            } else {
                let session = self
                    .editor
                    .as_mut()
                    .context("open original project files in Neovim first")?
                    .save_session(home, &roots)?;
                self.message = format!(
                    "Private native checkpoint saved: {} · F8 explicit restore",
                    session.summary()
                );
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.message = error.to_string();
            self.message_is_error = true;
        }
    }

    fn three_way_review(&mut self) {
        let outcome = (|| -> anyhow::Result<()> {
            anyhow::ensure!(!self.applying, "Application outcome pending");
            let view = self
                .detail
                .as_ref()
                .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                .context("select a result task and wait for evidence")?;
            let result = view.result.as_ref().context("no captured result")?;
            let artifact = result
                .artifacts
                .get(self.review_file % result.artifacts.len().max(1))
                .context("no captured artifact")?;
            let base = result
                .artifact_bases
                .iter()
                .find(|b| b.path == artifact.path)
                .context("original base evidence missing")?;
            let home = self.path.parent().context("owner home missing")?;
            let base_preview = match &base.sha256 {
                Some(hash) => {
                    anyhow::ensure!(
                        hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()),
                        "invalid captured base hash"
                    );
                    let directory = home.join("organization-artifact-bases");
                    let file=codex_utils_path::open_relative_regular_file(&directory,Path::new(hash)).context("original base bytes unavailable (legacy or incomplete recovery); do not invent a merge base")?;
                    anyhow::ensure!(
                        file.metadata()?.len() <= 1024 * 1024,
                        "three-way base exceeds 1 MiB"
                    );
                    let bytes = file.metadata()?.len();
                    Preview::new(
                        home,
                        &directory,
                        hash,
                        Some(&OrganizationArtifactEvidence {
                            path: hash.clone(),
                            sha256: hash.clone(),
                            bytes,
                        }),
                    )?
                }
                None => Preview::captured_text(home, "")?,
            };
            let project = self.selected_project().context("project unavailable")?;
            let root = Path::new(project.roots.first().context("project root unavailable")?);
            let owner_preview =
                match codex_utils_path::open_relative_regular_file(root, Path::new(&artifact.path))
                {
                    Ok(_) => Preview::new(home, root, &artifact.path, None)?,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                        Preview::captured_text(home, "")?
                    }
                    Err(e) => return Err(e.into()),
                };
            let workspace = view
                .attempts
                .iter()
                .find(|a| a.id == result.attempt_id)
                .and_then(|a| a.workspace.as_deref())
                .context("captured workspace unavailable")?;
            let agent_preview =
                Preview::new(home, Path::new(workspace), &artifact.path, Some(artifact))?;
            if self.editor.is_none() {
                self.editor = Some(Editor::start(self.frame.clone())?);
            }
            self.editor
                .as_mut()
                .context("editor unavailable")?
                .three_way(
                    &base_preview.path(),
                    &owner_preview.path(),
                    &agent_preview.path(),
                )?;
            self.review_previews = vec![base_preview, owner_preview, agent_preview];
            self.following = false;
            self.preferences.preset = Preset::Workspace;
            self.report_request = None;
            self.report_text = None;
            self.save();
            Ok(())
        })();
        self.message_is_error = outcome.is_err();
        self.message=match outcome {Ok(())=>"Three native diff panes: base / owner at capture / verified agent · read-only copies · Ctrl-G then o edits original separately · Apply still checks original base".into(),Err(e)=>format!("Three-way review unavailable: {e}")};
    }

    fn captured_checks_panel(&mut self) {
        let result = (|| -> anyhow::Result<()> {
            anyhow::ensure!(!self.applying, "Application outcome pending");
            let view = self
                .detail
                .as_ref()
                .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                .context("select a task and wait for captured evidence")?;
            let result = view
                .result
                .as_ref()
                .context("task has no captured result")?;
            anyhow::ensure!(!result.checks.is_empty(), "no captured native checks");
            let mut text = format!(
                "CAPTURED NATIVE CHECKS\nTask {} / attempt {} / result {}\nAccepted {}\n",
                view.task.id, result.attempt_id, result.digest, result.accepted
            );
            for check in &result.checks {
                text.push_str(&format!(
                    "\nargv {:?}\nexit {:?} · output truncated {}\nProtected inputs {:?}\n{}\n",
                    check.argv,
                    check.exit_code,
                    check.truncated,
                    check.immutable_inputs,
                    check.output
                ));
            }
            let preview = Preview::captured_text(
                self.path.parent().context("owner home missing")?,
                &clean(&text),
            )?;
            if self.editor.is_none() {
                self.editor = Some(Editor::start(self.frame.clone())?);
            }
            self.editor
                .as_mut()
                .context("editor unavailable")?
                .captured_panel(&preview.path())?;
            self.preview = Some(preview);
            self.following = false;
            self.preferences.preset = Preset::Workspace;
            self.report_request = None;
            self.report_text = None;
            self.save();
            Ok(())
        })();
        self.message_is_error = result.is_err();
        self.message=match result {Ok(())=>"Actual Neovim test panel · captured evidence, no test execution · Ctrl-G host focus".into(),Err(e)=>e.to_string()};
    }

    fn hunk_form(&mut self) {
        let result = (|| -> anyhow::Result<()> {
            anyhow::ensure!(
                !self.applying,
                "Application outcome pending; editor actions wait for reconciliation"
            );
            use sha2::Digest;
            use sha2::Sha256;
            let view = self
                .detail
                .as_ref()
                .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                .context("select an accepted task and wait for details")?;
            let result = view
                .result
                .as_ref()
                .filter(|r| r.accepted)
                .context("accept the exact result first")?;
            anyhow::ensure!(
                view.applications.is_empty(),
                "Application already recorded; inspect its receipt"
            );
            let project = self.selected_project().context("project unavailable")?;
            let root = Path::new(project.roots.first().context("project root missing")?);
            let workspace = view
                .attempts
                .iter()
                .find(|a| a.id == result.attempt_id)
                .and_then(|a| a.workspace.as_ref())
                .context("accepted workspace missing")?;
            let home = self.path.parent().context("owner home missing")?;
            let mut files = Vec::new();
            let mut count = 0;
            for artifact in &result.artifacts {
                anyhow::ensure!(
                    artifact.bytes <= 1024 * 1024,
                    "hunk preview supports files up to 1 MiB"
                );
                let base_evidence = result
                    .artifact_bases
                    .iter()
                    .find(|b| b.path == artifact.path)
                    .context("base evidence missing")?;
                let original =
                    codex_utils_path::open_relative_regular_file(root, Path::new(&artifact.path))?;
                anyhow::ensure!(
                    original.metadata()?.len() <= 1024 * 1024,
                    "original exceeds 1 MiB"
                );
                let mut base = String::new();
                original.take(1024 * 1024 + 1).read_to_string(&mut base)?;
                anyhow::ensure!(
                    Some(format!("{:x}", Sha256::digest(base.as_bytes()))) == base_evidence.sha256,
                    "original changed since task snapshot"
                );
                let preview =
                    Preview::new(home, Path::new(workspace), &artifact.path, Some(artifact))?;
                let modified = std::fs::read_to_string(preview.path())?;
                anyhow::ensure!(
                    !base.contains('\0') && !modified.contains('\0'),
                    "binary hunk preview unavailable"
                );
                let hunks = diffy::create_patch(&base, &modified).hunks().len();
                count += hunks;
                if hunks > 0 {
                    files.push(form::HunkFile {
                        path: artifact.path.clone(),
                        base,
                        artifact: modified,
                    });
                }
            }
            anyhow::ensure!(
                (1..=256).contains(&count),
                "select a result with 1–256 changed UTF-8 hunks"
            );
            self.form = Some(form::Form::new(
                form::Kind::HunkApply {
                    task: view.task.clone(),
                    result: result.clone(),
                    files,
                },
                &self.snapshot.records,
                self.record(),
                &self.model,
            ));
            Ok(())
        })();
        if let Err(error) = result {
            self.message = error.to_string();
            self.message_is_error = true;
        }
    }

    fn buffer_proposal(&mut self) {
        let proposal = (|| -> anyhow::Result<crate::organization_editor::BufferProposal> {
            anyhow::ensure!(
                !self.applying,
                "Application outcome pending; editor actions wait for reconciliation"
            );
            use sha2::Digest;
            use sha2::Sha256;
            let view = self
                .detail
                .as_ref()
                .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                .context("select an accepted task")?;
            let result = view
                .result
                .as_ref()
                .filter(|r| r.accepted)
                .context("accept the exact result first")?;
            let project = self.selected_project().context("project unavailable")?;
            let root = PathBuf::from(project.roots.first().context("project root missing")?);
            let workspace = view
                .attempts
                .iter()
                .find(|a| a.id == result.attempt_id)
                .and_then(|a| a.workspace.as_ref())
                .context("accepted workspace missing")?
                .clone();
            let pin = self
                .editor
                .as_mut()
                .context("open a writable original buffer in actual Neovim first")?
                .pin_buffer()?;
            let artifact = result
                .artifacts
                .iter()
                .find(|a| root.join(&a.path) == Path::new(&pin.path))
                .context("current buffer is not an accepted artifact in this project")?;
            anyhow::ensure!(artifact.bytes <= 1024 * 1024, "proposal exceeds 1 MiB");
            let current =
                codex_utils_path::open_relative_regular_file(&root, Path::new(&artifact.path))?;
            let mut bytes = Vec::new();
            current.take(1024 * 1024 + 1).read_to_end(&mut bytes)?;
            let hash = format!("{:x}", Sha256::digest(&bytes));
            anyhow::ensure!(
                hash == pin.sha256,
                "native text differs from the current original; reload or reconcile it first"
            );
            let known_base = result
                .artifact_bases
                .iter()
                .any(|b| b.path == artifact.path && b.sha256.as_ref() == Some(&hash))
                || view.applications.iter().any(|a| {
                    a.state == OrganizationApplicationState::Completed
                        && a.hunks
                            .as_deref()
                            .unwrap_or_default()
                            .iter()
                            .any(|h| h.path == artifact.path && h.output_sha256 == hash)
                });
            anyhow::ensure!(
                known_base,
                "original no longer matches its task base or completed hunk receipt; reconcile owner changes first"
            );
            let preview = Preview::new(
                self.path.parent().context("owner home missing")?,
                Path::new(&workspace),
                &artifact.path,
                Some(artifact),
            )?;
            let text = std::fs::read_to_string(preview.path())?;
            Ok(crate::organization_editor::BufferProposal {
                pin,
                text,
                result_digest: result.digest.clone(),
                artifact_sha256: artifact.sha256.clone(),
            })
        })();
        match proposal {
            Ok(proposal) => {
                self.confirmation = Some(Command::BufferProposal(Box::new(proposal)));
                self.confirmation_text = clean(&self.details());
                self.message="Review the exact pinned native-buffer proposal · y changes buffer · Esc cancels · native undo · save separately".into();
            }
            Err(error) => {
                self.message = error.to_string();
                self.message_is_error = true;
            }
        }
    }
    pub(crate) fn open_palette(&mut self) {
        self.palette = Some(palette::Palette::new());
        self.message =
            "Search commands · Enter selects · Esc cancels; owner decisions still confirm".into();
    }
    pub(crate) fn context(&self) -> Option<String> {
        self.record()
            .map(|record| match record {
                OrganizationRecord::Task(t) => format!(
                    "Organization selection: task {} · project {} · agent {} · {}",
                    t.id, t.project_id, t.owner_id, t.state
                ),
                OrganizationRecord::Node(n) => {
                    format!("Organization selection: {} · {}", n.label, n.id)
                }
                OrganizationRecord::Project(p) => {
                    format!("Organization selection: project {} · {}", p.name, p.id)
                }
                OrganizationRecord::Goal(g) => {
                    format!("Organization selection: goal {} · {}", g.objective, g.id)
                }
                _ => String::new(),
            })
            .map(|s| clean(&s))
    }
    pub(crate) fn open(
        handle: AppServerRequestHandle,
        frame: FrameRequester,
        home: &Path,
        preset: Option<Preset>,
        provider: &str,
        model: &str,
    ) -> Self {
        let path = home.join("organization-layouts.json");
        let (mut preferences, message) = match Preferences::load(&path) {
            Ok(p) => (p, "Loading owner state…".into()),
            Err(e) => (Preferences::default(), format!("Preferences rejected: {e}")),
        };
        if let Some(preset) = preset {
            preferences.preset = preset;
        } else if preferences.preset == Preset::Console {
            preferences.preset = Preset::Operations;
        }
        let (commands, command_rx) = mpsc::channel(4);
        let (update_tx, updates) = mpsc::channel(4);
        let worker = tokio::spawn(worker(handle, frame.clone(), command_rx, update_tx));
        Self {
            preferences,
            path,
            snapshot: Snapshot::default(),
            projection: Projection::Default,
            rows: Vec::new(),
            graph: graph::Graph::default(),
            selected: None,
            collapsed: BTreeSet::new(),
            detail: None,
            scroll: 0,
            detail_text: String::new(),
            report_text: None,
            report_request: None,
            review_diff: None,
            review_file: 0,
            search: String::new(),
            input: None,
            memory_request: None,
            memory_result: None,
            changes: false,
            change_target: None,
            confirmation: None,
            confirmation_text: String::new(),
            selection_confirmation: None,
            message,
            message_is_error: false,
            commands,
            updates,
            worker,
            editor: None,
            preview: None,
            follow_files: None,
            review_previews: Vec::new(),
            editor_area: Rect::default(),
            editor_file: 0,
            following: false,
            applying: false,
            provider: provider.into(),
            model: model.into(),
            form: None,
            setup_offered: false,
            palette: None,
            prices: crate::organization_pricing::Prices::load(home).ok(),
            frame,
            is_done: false,
        }
    }
    fn pane(&mut self) -> &mut PaneSettings {
        &mut self.preferences.panes[self.preferences.preset.index()]
    }
    fn send(&mut self, command: Command) {
        let command = match command {
            Command::OwnerContact(id) => {
                if id.as_ref().is_some_and(|id| !self.snapshot.records.iter().any(|r| matches!(r,OrganizationRecord::Node(n) if &n.id==id && n.enabled && n.kind==OrganizationNodeKind::Agent))) {
                    self.message = "Selected executive contact is unavailable; refresh and choose an enabled agent.".into();
                    self.message_is_error = true;
                    return;
                }
                let mut preferences = self.preferences.clone();
                preferences.executive_contact = id;
                if let Err(error) = preferences.save(&self.path) {
                    self.message = format!("Executive contact was not saved: {error}");
                    self.message_is_error = true;
                    return;
                }
                self.preferences = preferences;
                self.report_text = Some(reporting::owner_overview(
                    &self.snapshot,
                    self.preferences.executive_contact.as_deref(),
                ));
                self.report_request = None;
                self.message = "Executive contact preference saved.".into();
                return;
            }
            Command::ProjectExplorer(root) => {
                let result = (|| -> anyhow::Result<()> {
                    anyhow::ensure!(!self.applying, "Application outcome pending");
                    if self.editor.is_none() {
                        self.editor = Some(Editor::start(self.frame.clone())?);
                    }
                    self.editor
                        .as_mut()
                        .context("editor unavailable")?
                        .project_explorer(&root)?;
                    self.preferences.preset = Preset::Workspace;
                    self.report_request = None;
                    self.report_text = None;
                    self.following = false;
                    self.save();
                    Ok(())
                })();
                self.message_is_error = result.is_err();
                self.message=match result {Ok(())=>"Native owner project explorer · open files with Neovim · agent scopes unchanged · Ctrl-G host focus".into(),Err(e)=>e.to_string()};
                return;
            }
            Command::NativeTerminal(root) => {
                let result = (|| -> anyhow::Result<()> {
                    anyhow::ensure!(!self.applying, "Application outcome pending");
                    if self.editor.is_none() {
                        self.editor = Some(Editor::start(self.frame.clone())?);
                    }
                    self.editor
                        .as_mut()
                        .context("editor unavailable")?
                        .terminal(&root)?;
                    self.preferences.preset = Preset::Workspace;
                    self.report_request = None;
                    self.report_text = None;
                    self.following = false;
                    self.save();
                    Ok(())
                })();
                self.message_is_error = result.is_err();
                self.message = match result { Ok(()) => "Native owner terminal · commands use your owner shell, independently of agent grants · Ctrl-G host focus".into(), Err(e) => e.to_string() };
                return;
            }
            Command::NamedView { name, action } => {
                if let Err(error) = self.named_view(&name, &action) {
                    self.message = error.to_string();
                    self.message_is_error = true;
                }
                return;
            }
            command => command,
        };
        if let Err(error) = self.commands.try_send(command) {
            self.message = "Owner request queue busy; retry after refresh.".into();
            let command = error.into_inner();
            if matches!(
                &command,
                Command::Summary(..) | Command::Inventory(..) | Command::ArtifactLibrary { .. }
            ) {
                self.report_text = Some(self.message.clone());
            }
            if matches!(command, Command::MemorySearch(_)) {
                self.memory_result = Some(Err(self.message.clone()));
            }
        }
    }
    fn save(&mut self) {
        if let Err(e) = self.preferences.save(&self.path) {
            self.message = format!("Preferences were not saved: {e}");
        }
    }
    fn switch(&mut self, preset: Preset) {
        if let Some(editor) = &mut self.editor {
            editor.focused = false;
        }
        self.preferences.preset = preset;
        self.projection = Projection::Default;
        self.search.clear();
        self.selected = None;
        self.rebuild();
        self.inspect();
        self.save();
    }
    fn named_view(&mut self, name: &str, action: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            !name.is_empty() && name.len() <= 64 && !name.chars().any(char::is_control),
            "view name must be 1–64 bytes without control characters"
        );
        match action {
            "save" => {
                anyhow::ensure!(
                    self.preferences.views.len() < 8
                        || self.preferences.views.iter().any(|v| v.name == name),
                    "at most eight saved views"
                );
                anyhow::ensure!(
                    self.search.len() <= 256 && self.collapsed.len() <= 64,
                    "view supports a 256-byte filter and 64 collapsed branches"
                );
                let view = SavedView {
                    name: name.into(),
                    preset: self.preferences.preset,
                    panes: self.preferences.panes.clone(),
                    graph: self.preferences.graph.clone(),
                    search: self.search.clone(),
                    selected: self.selected.clone(),
                    collapsed: self.collapsed.clone(),
                };
                let mut preferences = self.preferences.clone();
                preferences.views.retain(|v| v.name != name);
                preferences.views.push(view);
                preferences.save(&self.path)?;
                self.preferences = preferences;
            }
            "restore" => {
                let view = self
                    .preferences
                    .views
                    .iter()
                    .find(|v| v.name == name)
                    .context("saved view not found")?
                    .clone();
                anyhow::ensure!(
                    view.preset != Preset::Console,
                    "return to Console through F1"
                );
                if let Some(editor) = &mut self.editor {
                    editor.focused = false;
                }
                self.preferences.preset = view.preset;
                self.preferences.panes = view.panes;
                self.preferences.graph = view.graph;
                let ids: BTreeSet<_> = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| match r {
                        OrganizationRecord::Node(n) => Some(n.id.as_str()),
                        OrganizationRecord::Task(t) => Some(t.id.as_str()),
                        OrganizationRecord::Project(p) => Some(p.id.as_str()),
                        OrganizationRecord::Goal(g) => Some(g.id.as_str()),
                        _ => None,
                    })
                    .collect();
                self.preferences.graph.focus = self
                    .preferences
                    .graph
                    .focus
                    .take()
                    .filter(|id| ids.contains(id.as_str()));
                self.selected = view.selected.filter(|id| ids.contains(id.as_str()));
                self.collapsed = view
                    .collapsed
                    .into_iter()
                    .filter(|id| ids.contains(id.as_str()))
                    .collect();
                self.search = view.search;
                self.projection = Projection::Default;
                self.rebuild();
                self.inspect();
                self.save();
            }
            "delete" => {
                let mut preferences = self.preferences.clone();
                preferences.views.retain(|v| v.name != name);
                preferences.save(&self.path)?;
                self.preferences = preferences;
            }
            _ => anyhow::bail!("unknown saved view operation"),
        }
        self.message = format!("Saved view {name}: {action}");
        Ok(())
    }
    fn rebuild(&mut self) {
        let search = self.search.to_lowercase();
        self.rows = if self.projection == Projection::Controls {
            self.snapshot
                .records
                .iter()
                .filter_map(|r| {
                    let OrganizationRecord::Control(c) = r else {
                        return None;
                    };
                    let label = format!(
                        "{} · revision {} · {}",
                        c.id,
                        c.revision,
                        serde_json::to_string(&c.value).unwrap_or_default()
                    );
                    (search.is_empty() || label.to_lowercase().contains(&search)).then(|| Row {
                        id: c.id.clone(),
                        label,
                    })
                })
                .collect()
        } else if matches!(self.projection, Projection::Health | Projection::Incidents) {
            self.snapshot
                .report
                .as_ref()
                .map(|report| {
                    report
                        .tasks
                        .iter()
                        .filter(|t| {
                            matches!(
                                t.state.as_str(),
                                "failed" | "blocked" | "needsReconciliation"
                            ) || self.projection == Projection::Health && t.state == "running"
                        })
                        .map(|t| Row {
                            id: t.task_id.clone(),
                            label: format!(
                                "{} · {} · {}",
                                t.state,
                                t.title,
                                t.blocker.as_deref().unwrap_or("No reported blocker")
                            ),
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else if self.projection == Projection::Presets {
            self.snapshot
                .records
                .iter()
                .filter_map(|record| match record {
                    OrganizationRecord::Preset(preset) => {
                        let label = format!(
                            "Preset: {} · {} nodes · revision {}",
                            preset.label,
                            preset.nodes.len(),
                            preset.revision
                        );
                        (search.is_empty()
                            || label.to_lowercase().contains(&search)
                            || preset.id.contains(&search))
                        .then(|| Row {
                            id: preset.id.clone(),
                            label,
                        })
                    }
                    _ => None,
                })
                .collect()
        } else if self.projection == Projection::Goals {
            self.graph = graph::Graph::goals(
                &self.snapshot.records,
                &self.collapsed,
                &search,
                &self.preferences.graph,
            );
            if self.preferences.graph.list {
                goal_tree(&self.snapshot.records, &self.collapsed, &search)
            } else {
                self.graph.rows()
            }
        } else if self.preferences.preset == Preset::Review {
            self.snapshot
                .report
                .as_ref()
                .map(|report| {
                    report
                        .tasks
                        .iter()
                        .filter_map(reporting::inbox_row)
                        .filter(|row| {
                            search.is_empty()
                                || row.label.to_lowercase().contains(&search)
                                || row.id.contains(&search)
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else if self.preferences.preset == Preset::Operations
            && self.projection != Projection::Schedules
            && self.projection != Projection::Workflows
        {
            self.graph = graph::Graph::build(
                &self.snapshot.records,
                &self.collapsed,
                &search,
                &self.preferences.graph,
            );
            if self.preferences.graph.list {
                let visible: BTreeSet<_> = self.graph.cards.iter().map(|c| c.id.as_str()).collect();
                let nodes: Vec<_> = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| match r {
                        OrganizationRecord::Node(n) if visible.contains(n.id.as_str()) => {
                            Some(n.clone())
                        }
                        _ => None,
                    })
                    .collect();
                tree(&nodes, &self.collapsed, "")
            } else {
                self.graph.rows()
            }
        } else {
            let mut rows: Vec<_> = self
                .snapshot
                .records
                .iter()
                .filter_map(|record| match record {
                    OrganizationRecord::Goal(g) if self.preferences.preset == Preset::Projects => {
                        Some(Row {
                            id: g.id.clone(),
                            label: format!("Goal: {}", g.objective),
                        })
                    }
                    OrganizationRecord::Project(p)
                        if self.preferences.preset == Preset::Projects =>
                    {
                        Some(Row {
                            id: p.id.clone(),
                            label: format!("Project: {}", p.name),
                        })
                    }
                    OrganizationRecord::Task(t) => Some(Row {
                        id: t.id.clone(),
                        label: format!(
                            "[{}] {} · {}",
                            t.state,
                            t.instructions.lines().next().unwrap_or("Task"),
                            t.owner_id
                        ),
                    }),
                    _ => None,
                })
                .filter(|r| {
                    search.is_empty()
                        || r.label.to_lowercase().contains(&search)
                        || r.id.contains(&search)
                })
                .collect();
            if self.projection == Projection::Schedules || self.projection == Projection::Workflows
            {
                rows.clear();
            }
            if self.projection == Projection::Schedules
                || (self.preferences.preset == Preset::Projects
                    && self.projection != Projection::Workflows)
            {
                rows.extend(self.snapshot.schedules.iter().filter_map(|v| {
                    let s = &v.schedule;
                    let label = format!(
                        "[{}] Schedule: {} · every {}s · {}/{} runs",
                        s.state.as_str(),
                        s.config.label,
                        s.config.interval_seconds,
                        s.issued_runs,
                        s.config.max_runs
                    );
                    (search.is_empty()
                        || label.to_lowercase().contains(&search)
                        || s.id.contains(&search))
                    .then(|| Row {
                        id: format!("schedule:{}", s.id),
                        label,
                    })
                }));
            }
            if self.projection == Projection::Workflows
                || (self.preferences.preset == Preset::Projects
                    && self.projection != Projection::Schedules)
            {
                rows.extend(self.snapshot.workflows.iter().filter_map(|view| {
                    let (id, label) = match view {
                        OrganizationWorkflowView::Series { series, usage } => (
                            format!("series:{}", series.id),
                            format!(
                                "Series: {} · {} · {}/{} issued · {} requests",
                                series.config.label,
                                series.state.as_str(),
                                series.issued_runs,
                                series.config.max_runs,
                                usage.model_requests
                            ),
                        ),
                        OrganizationWorkflowView::Limit { limit, usage } => (
                            format!("limit:{}", limit.id),
                            format!(
                                "Limits: {} · {} / {} requests · {} / {} active",
                                limit.id,
                                usage.model_requests,
                                limit.max_model_requests,
                                usage.active,
                                limit.max_active
                            ),
                        ),
                        OrganizationWorkflowView::Template { template } => (
                            format!("workflow:{}", template.id),
                            format!(
                                "Template: {} · {} steps · revision {}",
                                template.label,
                                template.items.len(),
                                template.revision
                            ),
                        ),
                        OrganizationWorkflowView::Run { run, tasks } => (
                            format!("run:{}", run.id),
                            format!(
                                "Run: {} · {:?} · {}/{} accepted · {} blocked",
                                run.preview.template.label,
                                run.state,
                                tasks.iter().filter(|task| task.state == "accepted").count(),
                                tasks.len(),
                                tasks
                                    .iter()
                                    .filter(|task| task.prerequisites.iter().any(|id| tasks
                                        .iter()
                                        .any(|p| &p.id == id && p.state != "accepted")))
                                    .count()
                            ),
                        ),
                    };
                    (search.is_empty()
                        || label.to_lowercase().contains(&search)
                        || id.contains(&search))
                    .then_some(Row { id, label })
                }));
            }
            if self.projection == Projection::Workflows {
                for view in &self.snapshot.workflows {
                    if let OrganizationWorkflowView::Run { run, tasks } = view {
                        for item in &run.preview.template.items {
                            if let Some(task) =
                                tasks.iter().find(|task| task.id == run.task_ids[&item.key])
                            {
                                let label = format!(
                                    "  [{}] {} · {} · {}",
                                    task.state, run.preview.template.label, item.key, item.owner_id
                                );
                                if search.is_empty()
                                    || label.to_lowercase().contains(&search)
                                    || task.id.contains(&search)
                                {
                                    rows.push(Row {
                                        id: task.id.clone(),
                                        label,
                                    });
                                }
                            }
                        }
                    }
                }
            }
            rows
        };
        if !self
            .rows
            .iter()
            .any(|r| Some(&r.id) == self.selected.as_ref())
        {
            self.selected = self.rows.first().map(|r| r.id.clone());
            self.detail = None;
        }
    }
    fn inspect(&mut self) {
        self.scroll = 0;
        self.detail = None;
        self.review_diff = None;
        self.review_file = 0;
        self.detail_text = clean(&self.details());
        if let Some(id) = &self.selected
            && self
                .snapshot
                .records
                .iter()
                .any(|r| matches!(r, OrganizationRecord::Task(t) if &t.id == id))
        {
            self.send(Command::Inspect(id.clone()));
        }
    }
    fn record(&self) -> Option<&OrganizationRecord> {
        self.snapshot.records.iter().find(|record| match record {
            OrganizationRecord::Node(n) => Some(&n.id) == self.selected.as_ref(),
            OrganizationRecord::Task(t) => Some(&t.id) == self.selected.as_ref(),
            OrganizationRecord::Goal(g) => Some(&g.id) == self.selected.as_ref(),
            OrganizationRecord::Project(p) => Some(&p.id) == self.selected.as_ref(),
            OrganizationRecord::Preset(p) => Some(&p.id) == self.selected.as_ref(),
            OrganizationRecord::Control(c) => Some(&c.id) == self.selected.as_ref(),
            _ => false,
        })
    }
    fn selected_schedule(&self) -> Option<&OrganizationScheduleView> {
        let id = self.selected.as_deref()?.strip_prefix("schedule:")?;
        self.snapshot.schedules.iter().find(|v| v.schedule.id == id)
    }
    fn control(&self, id: &str) -> Option<OrganizationControl> {
        self.snapshot.records.iter().find_map(|r| match r {
            OrganizationRecord::Control(c) if c.id == id => Some(c.clone()),
            _ => None,
        })
    }
    fn rollback_form(&mut self, role: bool) {
        let kind = (|| -> anyhow::Result<form::Kind> {
            if role {
                let id = match self.record() {
                    Some(OrganizationRecord::Node(n)) => n
                        .role_version_id
                        .as_deref()
                        .context("selected node has no captured role")?,
                    Some(OrganizationRecord::Role(r)) => r.id.as_str(),
                    _ => anyhow::bail!("select an agent/role for rollback"),
                };
                let selected = self
                    .snapshot
                    .records
                    .iter()
                    .find_map(|r| match r {
                        OrganizationRecord::Role(v) if v.id == id => Some(v),
                        _ => None,
                    })
                    .context("selected role not loaded")?;
                let versions = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| match r {
                        OrganizationRecord::Role(v) if v.role_id == selected.role_id => {
                            Some(v.clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let current = versions
                    .iter()
                    .max_by_key(|v| v.version)
                    .context("role history unavailable")?
                    .clone();
                anyhow::ensure!(
                    versions.iter().any(|v| v.version < current.version),
                    "no earlier loaded role version"
                );
                Ok(form::Kind::RoleRollback { current, versions })
            } else {
                let Some(OrganizationWorkflowView::Template { template }) =
                    self.selected_workflow()
                else {
                    anyhow::bail!("select the current workflow template")
                };
                let current = template.clone();
                let runs = self
                    .snapshot
                    .workflows
                    .iter()
                    .filter_map(|v| match v {
                        OrganizationWorkflowView::Run { run, .. }
                            if run.preview.template.id == current.id
                                && run.preview.template.revision < current.revision =>
                        {
                            Some((**run).clone())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                anyhow::ensure!(
                    !runs.is_empty(),
                    "no earlier version captured in a loaded run; do not invent discarded history"
                );
                Ok(form::Kind::WorkflowRollback { current, runs })
            }
        })();
        match kind {
            Ok(kind) => {
                self.form = Some(form::Form::new(
                    kind,
                    &self.snapshot.records,
                    self.record(),
                    &self.model,
                ))
            }
            Err(e) => {
                self.message = e.to_string();
                self.message_is_error = true;
            }
        }
    }
    fn follow_recent_file(&mut self) {
        let captured = (|| -> anyhow::Result<_> {
            let view = self
                .detail
                .as_ref()
                .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                .context("Select a prepared task and load its details")?;
            let grant = view
                .authorization
                .as_ref()
                .context("Task has no declared files")?;
            let root = view
                .attempts
                .first()
                .and_then(|a| a.workspace.as_ref())
                .context("Task workspace is not prepared")?;
            let files: Vec<_> = grant
                .scope
                .artifacts
                .iter()
                .chain(&grant.scope.input_files)
                .cloned()
                .collect();
            let tracker = FollowFiles::new(
                root.into(),
                files.clone(),
                view.result
                    .as_ref()
                    .map(|r| r.artifacts.clone())
                    .unwrap_or_default(),
            )?;
            let relative = tracker
                .latest()
                .context("No permitted file is present yet")?;
            let index = files
                .iter()
                .position(|path| path == relative)
                .context("Recent file is not declared")?;
            Ok((tracker, index))
        })();
        match captured {
            Ok((tracker, index)) => {
                self.editor_file = index;
                if self.editor_open(KeyCode::Char('t')) {
                    self.follow_files = Some(tracker);
                    if let Some(editor) = &mut self.editor {
                        editor.focused = false;
                    }
                    self.preferences.preset = Preset::Workspace;
                    self.report_request = None;
                    self.report_text = None;
                    self.save();
                }
            }
            Err(error) => {
                self.message = error.to_string();
                self.message_is_error = true;
            }
        }
    }
    fn refresh_follow(&mut self) -> anyhow::Result<()> {
        if !self.following || self.applying {
            return Ok(());
        }
        let Some(editor) = &mut self.editor else {
            return Ok(());
        };
        if editor.focused {
            return Ok(());
        }
        let current = self
            .preview
            .as_ref()
            .context("Follow preview unavailable")?
            .path();
        if !editor.is_follow_preview(&current)? {
            self.following = false;
            self.follow_files = None;
            self.message =
                "Follow stopped because the native editor is showing an independent buffer".into();
            return Ok(());
        }
        if editor.dirty()? {
            return Ok(());
        }
        if let Some(tracker) = &mut self.follow_files
            && let Some(relative) = tracker.changed()?
        {
            let expected = tracker.expected.iter().find(|a| a.path == relative);
            let preview = Preview::new(
                self.path.parent().context("Owner home unavailable")?,
                &tracker.root,
                &relative,
                expected,
            )?;
            editor.open(&preview.path(), true)?;
            editor.focused = false;
            self.preview = Some(preview);
            self.message = format!(
                "Follow recently changed permitted file: {relative} · read-only verified preview"
            );
        }
        if let Some(preview) = &mut self.preview
            && preview.refresh()?
        {
            editor.reload_follow()?;
        }
        Ok(())
    }
    fn editor_open(&mut self, key: KeyCode) -> bool {
        self.follow_files = None;
        if self.applying {
            self.message = "Application outcome pending; editor inputs remain frozen".into();
            return false;
        }
        let result = (|| -> anyhow::Result<()> {
            if self.editor.is_none() || self.editor.as_ref().is_some_and(|e| e.error.is_some()) {
                self.editor = Some(Editor::start(self.frame.clone())?);
            }
            if key == KeyCode::Char('e') {
                if let Some(editor) = &mut self.editor {
                    editor.focused = true;
                }
                return Ok(());
            }
            let view = self
                .detail
                .as_ref()
                .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                .ok_or_else(|| anyhow::anyhow!("select a task and wait for its details"))?;
            let project = self
                .snapshot
                .records
                .iter()
                .find_map(|r| match r {
                    OrganizationRecord::Project(p) if p.id == view.task.project_id => Some(p),
                    _ => None,
                })
                .ok_or_else(|| anyhow::anyhow!("task project unavailable"))?;
            let grant = view
                .authorization
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("task has no declared files yet"))?;
            let files: Vec<_> = grant
                .scope
                .artifacts
                .iter()
                .chain(&grant.scope.input_files)
                .collect();
            anyhow::ensure!(!files.is_empty(), "task has no declared files");
            let relative = files[self.editor_file % files.len()];
            anyhow::ensure!(
                Path::new(relative)
                    .components()
                    .all(|p| matches!(p, std::path::Component::Normal(_))),
                "invalid declared file"
            );
            let base = PathBuf::from(
                project
                    .roots
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("project root missing"))?,
            )
            .join(relative);
            // Host-selected task files cannot redirect the human editor through
            // a model-created symlink. Human native :edit commands stay human authority.
            let confined = |root: &Path, path: &Path| -> anyhow::Result<()> {
                let root = std::fs::canonicalize(root)?;
                if std::fs::symlink_metadata(path).is_ok() {
                    anyhow::ensure!(
                        std::fs::canonicalize(path)? == path && path.starts_with(&root),
                        "linked or escaped editor file rejected"
                    );
                } else {
                    anyhow::ensure!(
                        std::fs::canonicalize(
                            path.parent()
                                .ok_or_else(|| anyhow::anyhow!("file parent missing"))?
                        )?
                        .starts_with(root),
                        "escaped editor parent"
                    );
                }
                Ok(())
            };
            if matches!(key, KeyCode::Char('o' | 'd')) {
                confined(
                    Path::new(
                        project
                            .roots
                            .first()
                            .ok_or_else(|| anyhow::anyhow!("project root missing"))?,
                    ),
                    &base,
                )?;
            }
            let editor = self
                .editor
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("editor unavailable"))?;
            if key == KeyCode::Char('o') {
                editor.open(&base, false)?;
                self.following = false;
                self.message = format!("Independent: {} · Ctrl-G host focus", base.display());
            } else {
                let root = PathBuf::from(
                    view.attempts
                        .first()
                        .and_then(|a| a.workspace.as_ref())
                        .ok_or_else(|| anyhow::anyhow!("task workspace is not prepared"))?,
                );
                let artifact = root.join(relative);
                anyhow::ensure!(
                    artifact.is_file(),
                    "selected task file is not present yet; n selects the next declared file"
                );
                confined(&root, &artifact)?;
                let expected = view
                    .result
                    .as_ref()
                    .and_then(|r| r.artifacts.iter().find(|a| a.path == *relative));
                let preview = Preview::new(
                    self.path
                        .parent()
                        .ok_or_else(|| anyhow::anyhow!("owner home missing"))?,
                    &root,
                    relative,
                    expected,
                )?;
                if key == KeyCode::Char('d') {
                    editor.diff(&base, &preview.path())?;
                    self.following = false;
                } else {
                    editor.open(&preview.path(), true)?;
                    self.following = true;
                }
                self.preview = Some(preview);
                self.message = format!(
                    "{}: {} · Ctrl-G host focus",
                    if self.following {
                        "Follow AI (read only)"
                    } else {
                        "Native diff"
                    },
                    artifact.display()
                );
            }
            Ok(())
        })();
        match result {
            Ok(()) => true,
            Err(e) => {
                self.message = e.to_string();
                false
            }
        }
    }
    fn decision(&mut self, key: char) {
        let command = if key == '%' {
            match self.record() {
                Some(OrganizationRecord::Control(control)) => match &control.value {
                    OrganizationControlValue::Team { applied: false, .. }
                    | OrganizationControlValue::KnowledgeProposal {
                        reviewed: false, ..
                    }
                    | OrganizationControlValue::RoleDraft { applied: false, .. }
                    | OrganizationControlValue::RoleRollback { applied: false, .. }
                    | OrganizationControlValue::WorkflowRollback { applied: false, .. }
                    | OrganizationControlValue::WorkflowImprovement { applied: false, .. }
                    | OrganizationControlValue::Improvement { applied: false, .. } => {
                        let mut value = control.value.clone();
                        match &mut value {
                            OrganizationControlValue::Team { applied, .. }
                            | OrganizationControlValue::RoleDraft { applied, .. }
                            | OrganizationControlValue::RoleRollback { applied, .. }
                            | OrganizationControlValue::WorkflowRollback { applied, .. }
                            | OrganizationControlValue::WorkflowImprovement { applied, .. }
                            | OrganizationControlValue::Improvement { applied, .. } => {
                                *applied = true
                            }
                            OrganizationControlValue::KnowledgeProposal { reviewed, .. } => {
                                *reviewed = true
                            }
                            _ => {}
                        }
                        Some(Command::Definition(OrganizationMutation::SetControl {
                            expected_revision: Some(control.revision),
                            value,
                        }))
                    }
                    OrganizationControlValue::Replan {
                        id,
                        items,
                        applied_handoff_ids,
                        ..
                    } if applied_handoff_ids.is_empty() => items.first().map(|item| {
                        Command::Execution(OrganizationExecutionMutation::ApplyReplan {
                            task_id: item.task_id.clone(),
                            id: id.clone(),
                            expected_revision: control.revision,
                        })
                    }),
                    _ => None,
                },
                _ => None,
            }
        } else if let Some(view) = self.selected_schedule() {
            let action = match key {
                'p' => Some(OrganizationScheduleControl::Pause),
                'u' => Some(OrganizationScheduleControl::Resume),
                'x' => Some(OrganizationScheduleControl::Cancel),
                _ => None,
            };
            action.map(|action| {
                Command::Schedule(OrganizationScheduleMutation::Control {
                    id: view.schedule.id.clone(),
                    expected_revision: view.schedule.revision,
                    action,
                })
            })
        } else if let Some(OrganizationWorkflowView::Series { series, .. }) =
            self.selected_workflow()
        {
            let action = match key {
                'p' => Some(OrganizationWorkflowControl::Pause),
                'u' => Some(OrganizationWorkflowControl::Resume),
                'x' => Some(OrganizationWorkflowControl::Cancel),
                _ => None,
            };
            action.map(|action| {
                Command::Workflow(OrganizationWorkflowMutation::ControlSeries {
                    id: series.id.clone(),
                    expected_revision: series.revision,
                    action,
                })
            })
        } else if let Some(OrganizationWorkflowView::Run { run, .. }) = self.selected_workflow() {
            let action = match key {
                'p' => Some(OrganizationWorkflowControl::Pause),
                'u' => Some(OrganizationWorkflowControl::Resume),
                'x' => Some(OrganizationWorkflowControl::Cancel),
                _ => None,
            };
            action.map(|action| {
                Command::Workflow(OrganizationWorkflowMutation::Control {
                    id: run.id.clone(),
                    expected_revision: run.revision,
                    action,
                })
            })
        } else if let Some(view) = self
            .detail
            .as_ref()
            .filter(|v| Some(&v.task.id) == self.selected.as_ref())
        {
            let task_id = view.task.id.clone();
            let expected_revision = view.task.revision;
            match key {
                'a' | 'r' => view
                    .approvals
                    .iter()
                    .find(|a| a.state == OrganizationApprovalState::Pending)
                    .map(|a| {
                        Command::Execution(OrganizationExecutionMutation::DecideApproval {
                            task_id,
                            approval_id: a.id.clone(),
                            expected_hash: a.action_hash.clone(),
                            approve: key == 'a',
                        })
                    }),
                'v' => view.result.as_ref().map(|r| {
                    Command::Execution(OrganizationExecutionMutation::Accept {
                        task_id,
                        expected_revision,
                        digest: r.digest.clone(),
                    })
                }),
                'p' | 'x' => Some(Command::Execution(OrganizationExecutionMutation::Stop {
                    task_id,
                    expected_revision,
                    cancel: key == 'x',
                })),
                'c' => {
                    self.input = Some(String::new());
                    self.changes = true;
                    self.change_target = Some(view.task.clone());
                    return;
                }
                _ => None,
            }
        } else if key == 'p' {
            match self.record() {
                Some(OrganizationRecord::Node(n)) => {
                    Some(Command::Definition(OrganizationMutation::PutNode {
                        id: Some(n.id.clone()),
                        expected_revision: Some(n.revision),
                        parent_id: n.parent_id.clone(),
                        kind: n.kind,
                        label: n.label.clone(),
                        role_version_id: n.role_version_id.clone(),
                        model: n.model.clone(),
                        enabled: !n.enabled,
                    }))
                }
                _ => None,
            }
        } else {
            None
        };
        self.confirmation = command;
        if self.confirmation.is_some() {
            self.scroll = 0;
            self.pane().inspector = true;
            self.pane().inspector_focus = true;
            self.confirmation_text = clean(&self.details());
        }
        self.message = if self.confirmation.is_some() {
            "Confirm the displayed exact owner decision: y / Esc".into()
        } else {
            "No matching decision for the selected record.".into()
        };
    }
    fn open_form(&mut self, key: char) {
        let kind = match key {
            'κ' => {
                use codex_protocol::organization_coordination::OrganizationNoteReference;
                use codex_protocol::organization_coordination::OrganizationNoteScope;
                use sha2::Digest;
                let previous = self.control("library");
                let mut projects: BTreeMap<String, String> = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| match r {
                        OrganizationRecord::Project(p) => Some((p.id.clone(), p.name.clone())),
                        _ => None,
                    })
                    .take(32)
                    .collect();
                let notes = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| match r {
                        OrganizationRecord::Note(n) => Some(n),
                        _ => None,
                    })
                    .chain(
                        self.memory_result
                            .as_ref()
                            .and_then(|result| result.as_ref().ok())
                            .into_iter()
                            .flat_map(|r| r.notes.iter()),
                    );
                let mut sources: BTreeMap<String, (OrganizationNoteReference, String)> = notes
                    .filter(|n| n.scope == OrganizationNoteScope::Project && n.owner_reviewed)
                    .map(|n| {
                        (
                            n.id.clone(),
                            (
                                OrganizationNoteReference {
                                    note_id: n.id.clone(),
                                    body_sha256: format!(
                                        "{:x}",
                                        sha2::Sha256::digest(n.body.as_bytes())
                                    ),
                                },
                                format!(
                                    "{} · {} · {}",
                                    n.project_id,
                                    n.id,
                                    n.body.chars().take(80).collect::<String>()
                                ),
                            ),
                        )
                    })
                    .take(64)
                    .collect();
                if let Some(OrganizationControl {
                    value:
                        OrganizationControlValue::WorkspaceLibrary {
                            projects: stored_projects,
                            sources: stored,
                            ..
                        },
                    ..
                }) = &previous
                {
                    for id in stored_projects {
                        projects.entry(id.clone()).or_insert_with(|| {
                            format!("{id} (stored project; definition not loaded)")
                        });
                    }
                    for reference in stored {
                        sources.entry(reference.note_id.clone()).or_insert_with(|| {
                            (
                                reference.clone(),
                                format!(
                                    "{} (stored source; current body not loaded)",
                                    reference.note_id
                                ),
                            )
                        });
                    }
                }
                form::Kind::WorkspaceLibrary {
                    previous,
                    projects: projects.into_iter().collect(),
                    sources: sources.into_values().collect(),
                }
            }
            'ξ' => {
                let project_id = match self.record() {
                    Some(OrganizationRecord::Project(p)) => Some(p.id.as_str()),
                    Some(OrganizationRecord::Task(t)) => Some(t.project_id.as_str()),
                    _ => None,
                };
                let roots = self
                    .snapshot
                    .records
                    .iter()
                    .find_map(|r| match r {
                        OrganizationRecord::Project(p) if Some(p.id.as_str()) == project_id => {
                            Some(p.roots.clone())
                        }
                        _ => None,
                    })
                    .unwrap_or_default();
                if roots.is_empty() {
                    self.message =
                        "Select a loaded project or its task with owner project roots".into();
                    return;
                }
                form::Kind::ProjectExplorer { roots }
            }
            'ν' => {
                let source = (|| -> anyhow::Result<_> {
                    let (previous, value) = if let Some(OrganizationRecord::Control(c)) =
                        self.record().filter(|r| {
                            matches!(
                                r,
                                OrganizationRecord::Control(OrganizationControl {
                                    value: OrganizationControlValue::ArtifactShare { .. },
                                    ..
                                })
                            )
                        }) {
                        (Some(c.clone()), c.value.clone())
                    } else {
                        let view = self
                            .detail
                            .as_ref()
                            .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                            .context("Select an accepted result and load its evidence")?;
                        let result = view
                            .result
                            .as_ref()
                            .filter(|r| r.accepted && view.task.state == "accepted")
                            .context("Only exact accepted outputs can be shared")?;
                        let artifact = result
                            .artifacts
                            .get(self.review_file % result.artifacts.len().max(1))
                            .context("No captured artifact")?;
                        let attempt = view
                            .attempts
                            .iter()
                            .find(|a| a.id == result.attempt_id)
                            .context("Captured source attempt unavailable")?;
                        let preview = Preview::new(
                            self.path.parent().context("Owner home unavailable")?,
                            Path::new(
                                attempt
                                    .workspace
                                    .as_deref()
                                    .context("Source workspace unavailable")?,
                            ),
                            &artifact.path,
                            Some(artifact),
                        )?;
                        let body = if artifact.bytes <= 32 * 1024 {
                            String::from_utf8(std::fs::read(preview.path())?).ok()
                        } else {
                            None
                        };
                        (
                            None,
                            OrganizationControlValue::ArtifactShare {
                                id: uuid::Uuid::now_v7().to_string(),
                                source_task_id: view.task.id.clone(),
                                source_revision: view.task.revision,
                                result_digest: result.digest.clone(),
                                source_project_id: view.task.project_id.clone(),
                                source_owner_id: attempt
                                    .owner_id
                                    .clone()
                                    .context("Captured author unavailable (legacy)")?,
                                artifact: artifact.clone(),
                                body,
                                destinations: vec![],
                            },
                        )
                    };
                    let mut projects: BTreeMap<String, String> = self
                        .snapshot
                        .records
                        .iter()
                        .filter_map(|r| match r {
                            OrganizationRecord::Project(p) => Some((p.id.clone(), p.name.clone())),
                            _ => None,
                        })
                        .take(32)
                        .collect();
                    if let OrganizationControlValue::ArtifactShare { destinations, .. } = &value {
                        for id in destinations {
                            projects.entry(id.clone()).or_insert_with(|| {
                                format!("{id} (stored project; definition not loaded)")
                            });
                        }
                    }
                    Ok(form::Kind::ArtifactShare {
                        previous,
                        value,
                        projects: projects.into_iter().collect(),
                    })
                })();
                match source {
                    Ok(kind) => kind,
                    Err(error) => {
                        self.message = error.to_string();
                        return;
                    }
                }
            }
            'ι' => {
                let source = (|| -> anyhow::Result<_> {
                    let view = self
                        .detail
                        .as_ref()
                        .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                        .context(
                            "Select an accepted discussion conclusion and load its evidence",
                        )?;
                    let result = view
                        .result
                        .as_ref()
                        .filter(|r| r.accepted && view.task.state == "accepted")
                        .context("Meeting follow-up requires the exact accepted conclusion")?;
                    let artifact = result
                        .artifacts
                        .iter()
                        .find(|a| {
                            a.path == "discussion/discussion-conclusion.json"
                                && a.bytes <= 16 * 1024
                        })
                        .context("Accepted conclusion artifact of at most 16 KiB required")?;
                    let workspace = view
                        .attempts
                        .iter()
                        .find(|a| a.id == result.attempt_id)
                        .and_then(|a| a.workspace.as_ref())
                        .context("Meeting workspace unavailable")?;
                    let preview = Preview::new(
                        self.path.parent().context("Owner home unavailable")?,
                        Path::new(workspace),
                        &artifact.path,
                        Some(artifact),
                    )?;
                    Ok(form::Kind::MeetingFollowUp {
                        source: view.task.clone(),
                        result_digest: result.digest.clone(),
                        source_body: std::fs::read_to_string(preview.path())?,
                    })
                })();
                match source {
                    Ok(kind) => kind,
                    Err(error) => {
                        self.message = error.to_string();
                        return;
                    }
                }
            }
            'θ' => {
                let project_id = match self.record() {
                    Some(OrganizationRecord::Project(p)) => Some(p.id.as_str()),
                    Some(OrganizationRecord::Task(t)) => Some(t.project_id.as_str()),
                    _ => None,
                };
                let Some(project) = self.snapshot.records.iter().find_map(|r| match r {
                    OrganizationRecord::Project(p) if Some(p.id.as_str()) == project_id => {
                        Some(p.clone())
                    }
                    _ => None,
                }) else {
                    self.message = "Select a project for a planning request".into();
                    return;
                };
                form::Kind::ProjectPlanning(project)
            }
            'ζ' => {
                let Some(OrganizationRecord::Goal(goal)) = self.record() else {
                    self.message =
                        "Select a goal to assign responsibility and prerequisites".into();
                    return;
                };
                let candidates = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| {
                        let OrganizationRecord::Goal(g) = r else {
                            return None;
                        };
                        let project_id = match self
                            .control(&format!("goal-meta:{}", g.id))
                            .map(|c| c.value)
                        {
                            Some(OrganizationControlValue::GoalMetadata { project_id, .. }) => {
                                Some(project_id)
                            }
                            _ => self.snapshot.records.iter().find_map(|r| match r {
                                OrganizationRecord::Project(p)
                                    if p.goal_id.as_ref() == Some(&g.id) =>
                                {
                                    Some(p.id.clone())
                                }
                                _ => None,
                            }),
                        }?;
                        Some(OrganizationGoalDependency {
                            goal: Box::new(g.clone()),
                            project_id,
                            cross_project_approved: false,
                        })
                    })
                    .collect();
                form::Kind::GoalMetadata {
                    goal: goal.clone(),
                    previous: self.control(&format!("goal-meta:{}", goal.id)),
                    candidates,
                }
            }
            'η' => {
                let (kind, target_id, target_revision) = match self.record() {
                    Some(OrganizationRecord::Goal(g)) => {
                        (OrganizationDeadlineKind::Goal, g.id.clone(), g.revision)
                    }
                    Some(OrganizationRecord::Project(p)) => {
                        (OrganizationDeadlineKind::Project, p.id.clone(), p.revision)
                    }
                    Some(OrganizationRecord::Task(t)) => {
                        (OrganizationDeadlineKind::Task, t.id.clone(), t.revision)
                    }
                    _ => {
                        self.message =
                            "Select a goal, project or task for a business due date".into();
                        return;
                    }
                };
                let name = match kind {
                    OrganizationDeadlineKind::Goal => "goal",
                    OrganizationDeadlineKind::Project => "project",
                    OrganizationDeadlineKind::Task => "task",
                };
                form::Kind::Deadline {
                    kind,
                    target_revision,
                    previous: self.control(&format!("deadline:{name}:{target_id}")),
                    target_id,
                }
            }
            'δ' => form::Kind::Conversation {
                previous: match self.record() {
                    Some(OrganizationRecord::Control(c))
                        if matches!(c.value, OrganizationControlValue::Conversation { .. }) =>
                    {
                        Some(c.clone())
                    }
                    _ => None,
                },
                agent_id: match self.record() {
                    Some(OrganizationRecord::Node(n)) if n.kind == OrganizationNodeKind::Agent => {
                        Some(n.id.clone())
                    }
                    Some(OrganizationRecord::Task(t)) => Some(t.owner_id.clone()),
                    _ => None,
                },
            },
            'χ' => {
                let Some(view) = self
                    .detail
                    .as_ref()
                    .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                else {
                    self.message = "Select a task and load its exact execution evidence".into();
                    return;
                };
                let Some(attempt) = view.attempts.first() else {
                    self.message = "No attempt evidence available".into();
                    return;
                };
                if attempt.model_requests == attempt.reported_model_requests
                    && !view
                        .actions
                        .iter()
                        .any(|a| a.state == OrganizationActionState::Uncertain)
                {
                    self.message = "No unresolved usage/effects in the selected attempt".into();
                    return;
                }
                let fingerprint = match codex_state::organization::uncertainty_fingerprint(
                    attempt,
                    &view.actions,
                ) {
                    Ok(fingerprint) => fingerprint,
                    Err(error) => {
                        self.message = error.to_string();
                        return;
                    }
                };
                form::Kind::Reconciliation {
                    attempt: attempt.clone(),
                    fingerprint,
                }
            }
            'φ' => form::Kind::ExecutionProfile(match self.record() {
                Some(OrganizationRecord::Control(c))
                    if matches!(c.value, OrganizationControlValue::ExecutionProfile { .. }) =>
                {
                    Some(c.clone())
                }
                _ => None,
            }),
            'ψ' => {
                let Some(OrganizationRecord::Task(task)) = self.record() else {
                    self.message = "Select a task to authorize from a saved profile".into();
                    return;
                };
                let profiles = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| match r {
                        OrganizationRecord::Control(c)
                            if matches!(
                                c.value,
                                OrganizationControlValue::ExecutionProfile { .. }
                            ) =>
                        {
                            Some(c.clone())
                        }
                        _ => None,
                    })
                    .collect();
                form::Kind::AuthorizeProfile {
                    task: task.clone(),
                    profiles,
                }
            }
            'ρ' => form::Kind::ResourcePressure(self.control("pressure")),
            'Z' => form::Kind::Setup,
            'µ' => form::Kind::Mission(self.control("mission")),
            'π' => {
                let id = match self.record() {
                    Some(OrganizationRecord::Project(p)) => Some(p.id.clone()),
                    Some(OrganizationRecord::Task(t)) => Some(t.project_id.clone()),
                    _ => None,
                };
                let Some(project) = self.snapshot.records.iter().find_map(|r| match r {
                    OrganizationRecord::Project(p) if Some(&p.id) == id.as_ref() => Some(p.clone()),
                    _ => None,
                }) else {
                    self.message = "Select a project for an editable playbook".into();
                    return;
                };
                form::Kind::Playbook(project)
            }
            'γ' => {
                let Some(OrganizationRecord::Goal(goal)) = self.record() else {
                    self.message = "Select a goal to review its criteria".into();
                    return;
                };
                let accepted = self
                    .snapshot
                    .report
                    .as_ref()
                    .map(|r| {
                        r.tasks
                            .iter()
                            .filter(|t| t.state == "accepted" && t.result_digest.is_some())
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                form::Kind::GoalReview {
                    goal: goal.clone(),
                    previous: self.control(&format!("goal:{}", goal.id)),
                    accepted,
                }
            }
            'ε' => {
                let Some(OrganizationRecord::Task(task)) = self.record() else {
                    self.message = "Select a task needing assistance".into();
                    return;
                };
                form::Kind::Escalation {
                    task: task.clone(),
                    previous: self.control(&format!("escalation:{}", task.id)),
                }
            }
            'λ' => {
                let Some(view) = self
                    .detail
                    .as_ref()
                    .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                else {
                    self.message = "Select an accepted task and wait for its evidence".into();
                    return;
                };
                let Some(result) = view
                    .result
                    .as_ref()
                    .filter(|r| r.accepted && view.task.state == "accepted")
                else {
                    self.message = "Lessons require an exact accepted result".into();
                    return;
                };
                form::Kind::Lesson {
                    task: view.task.clone(),
                    digest: result.digest.clone(),
                }
            }
            '~' => form::Kind::NamedView(
                self.preferences
                    .views
                    .iter()
                    .map(|v| v.name.clone())
                    .collect(),
            ),
            '@' => {
                let Some(OrganizationRecord::Node(node)) = self.record() else {
                    self.message = "Select a definition to propose team changes".into();
                    return;
                };
                let expected_nodes = std::iter::once(node.clone())
                    .chain(self.snapshot.records.iter().filter_map(|r| match r {
                        OrganizationRecord::Node(n) if Some(&n.id) == node.parent_id.as_ref() => {
                            Some(n.clone())
                        }
                        _ => None,
                    }))
                    .chain(
                        self.snapshot
                            .records
                            .iter()
                            .filter_map(|r| match r {
                                OrganizationRecord::Node(n)
                                    if n.id != node.id
                                        && Some(&n.id) != node.parent_id.as_ref() =>
                                {
                                    Some(n.clone())
                                }
                                _ => None,
                            })
                            .take(if node.parent_id.is_some() { 62 } else { 63 }),
                    )
                    .collect();
                form::Kind::Team {
                    expected_nodes,
                    nodes: vec![node.clone()],
                }
            }
            '?' => {
                let project_id = match self.record() {
                    Some(OrganizationRecord::Project(p)) => Some(&p.id),
                    Some(OrganizationRecord::Task(t)) => Some(&t.project_id),
                    _ => None,
                };
                let Some(project) = self.snapshot.records.iter().find_map(|r| match r {
                    OrganizationRecord::Project(p) if Some(&p.id) == project_id => Some(p.clone()),
                    _ => None,
                }) else {
                    self.message = "Select a local project for a finite discovery proposal".into();
                    return;
                };
                form::Kind::Discovery(project)
            }
            '^' => {
                let Some(view) = self
                    .detail
                    .as_ref()
                    .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                else {
                    self.message = "Select a task and wait for captured grant details".into();
                    return;
                };
                let Some(grant) = &view.authorization else {
                    self.message =
                        "Authorize a bounded task before enabling startup recovery".into();
                    return;
                };
                form::Kind::Recovery {
                    task: view.task.clone(),
                    authorization_id: grant.id.clone(),
                    previous: self.control(&format!("recovery:{}", view.task.id)),
                }
            }
            '$' => {
                if let Some(OrganizationWorkflowView::Template { template }) =
                    self.selected_workflow()
                {
                    let runs = self
                        .snapshot
                        .workflows
                        .iter()
                        .filter_map(|v| match v {
                            OrganizationWorkflowView::Run { run, .. }
                                if run.preview.template.project_id == template.project_id =>
                            {
                                Some((**run).clone())
                            }
                            _ => None,
                        })
                        .collect();
                    self.form = Some(form::Form::new(
                        form::Kind::WorkflowImprovement {
                            template: template.clone(),
                            runs,
                        },
                        &self.snapshot.records,
                        self.record(),
                        &self.model,
                    ));
                    return;
                }
                let Some(OrganizationRecord::Node(node)) = self.record() else {
                    self.message = "Select the target agent definition".into();
                    return;
                };
                let accepted = self
                    .snapshot
                    .report
                    .iter()
                    .flat_map(|r| &r.tasks)
                    .filter(|t| t.state == "accepted" && t.result_digest.is_some())
                    .cloned()
                    .collect();
                form::Kind::Improvement {
                    node: node.clone(),
                    accepted,
                }
            }
            'P' => match self.record() {
                Some(OrganizationRecord::Goal(goal)) => form::Kind::GoalPlan(goal.clone()),
                Some(OrganizationRecord::Task(_)) => match self.accepted_plan_items() {
                    Ok((project_id, items)) => form::Kind::WorkflowSave {
                        project_id,
                        items,
                        previous: None,
                    },
                    Err(error) => {
                        self.message = error.to_string();
                        return;
                    }
                },
                _ => {
                    self.message = "Select a goal to propose a planner, or an accepted plan.json task to review its workflow.".into();
                    return;
                }
            },
            'b' => {
                let target = match self.selected_workflow() {
                    Some(OrganizationWorkflowView::Run { run, .. }) => Some((
                        run.preview.template.project_id.clone(),
                        Some(run.id.clone()),
                    )),
                    Some(OrganizationWorkflowView::Limit { limit, .. }) => {
                        if limit.series_id.is_some() {
                            self.message="Series ceilings are fixed at creation; use p/u/x series controls. Project limits remain editable.".into();
                            return;
                        }
                        Some((limit.project_id.clone(), limit.run_id.clone()))
                    }
                    _ => match self.record() {
                        Some(OrganizationRecord::Project(project)) => {
                            Some((project.id.clone(), None))
                        }
                        Some(OrganizationRecord::Task(task)) => {
                            Some((task.project_id.clone(), None))
                        }
                        _ => None,
                    },
                };
                let Some((project_id, run_id)) = target else {
                    self.message = "Select a project, task, workflow run or shared limit to edit its ceilings.".into();
                    return;
                };
                let previous = self.snapshot.workflows.iter().find_map(|view| match view {
                    OrganizationWorkflowView::Limit { limit, .. }
                        if limit.project_id == project_id
                            && limit.run_id == run_id
                            && limit.series_id.is_none() =>
                    {
                        Some(limit.clone())
                    }
                    _ => None,
                });
                form::Kind::SharedLimits {
                    project_id,
                    run_id,
                    previous,
                }
            }
            'B' => form::Kind::Group,
            'O' => form::Kind::GlobalControl(self.control("global")),
            '*' => {
                let policy = match self.record() {
                    Some(OrganizationRecord::Control(policy))
                        if matches!(policy.value, OrganizationControlValue::Supervision { .. }) =>
                    {
                        Some(policy.clone())
                    }
                    _ => match self.selected_workflow() {
                        Some(OrganizationWorkflowView::Series { series, .. }) => {
                            self.control(&format!("supervision:{}", series.id))
                        }
                        _ => None,
                    },
                };
                let Some(policy) = policy else {
                    self.message =
                        "Select a goal supervision policy in owner controls, or its finite series"
                            .into();
                    return;
                };
                form::Kind::SupervisionControl(policy)
            }
            '!' => match self.record() {
                Some(OrganizationRecord::Task(task)) => form::Kind::Priority {
                    task: task.clone(),
                    previous: self.control(&format!("priority:{}", task.id)),
                },
                _ => {
                    self.message = "Select a task to set its bounded priority.".into();
                    return;
                }
            },
            'F' => {
                let project = match self.record() {
                    Some(OrganizationRecord::Project(p)) => Some(p.clone()),
                    Some(OrganizationRecord::Task(t)) => {
                        self.snapshot.records.iter().find_map(|r| match r {
                            OrganizationRecord::Project(p) if p.id == t.project_id => {
                                Some(p.clone())
                            }
                            _ => None,
                        })
                    }
                    _ => None,
                };
                let Some(project) = project else {
                    self.message = "Select a project or its task to review its lifecycle.".into();
                    return;
                };
                let previous = self.control(&format!("project:{}", project.id));
                let accepted = self
                    .snapshot
                    .report
                    .iter()
                    .flat_map(|r| &r.tasks)
                    .filter(|t| {
                        t.project_id == project.id
                            && t.state == "accepted"
                            && t.result_digest.is_some()
                    })
                    .cloned()
                    .collect();
                form::Kind::ProjectControl {
                    project,
                    previous,
                    accepted,
                }
            }
            'D' => match self.record() {
                Some(OrganizationRecord::Node(node)) => form::Kind::PresetSave(node.clone()),
                _ => {
                    self.message = "Select a node whose definition subtree should be saved.".into();
                    return;
                }
            },
            'V' => match self.record() {
                Some(OrganizationRecord::Preset(preset)) => form::Kind::PresetClone(preset.clone()),
                _ => {
                    self.message = "F9 shows organization presets; select one to clone.".into();
                    return;
                }
            },
            'E' if self.selected_template().is_some() => {
                let Some(template) = self.selected_template().cloned() else {
                    return;
                };
                form::Kind::WorkflowSave {
                    project_id: template.project_id.clone(),
                    items: template.items.clone(),
                    previous: Some(template),
                }
            }
            'E' => match self.record() {
                Some(OrganizationRecord::Node(node)) => form::Kind::NodeEdit(node.clone()),
                Some(OrganizationRecord::Goal(goal)) => form::Kind::GoalEdit(goal.clone()),
                _ => {
                    self.message =
                        "Select an Operations node to edit its captured definition.".into();
                    return;
                }
            },
            'I' => match self.record() {
                Some(OrganizationRecord::Node(node)) => form::Kind::Membership(node.clone()),
                _ => {
                    self.message = "Select a node to edit its additional group memberships.".into();
                    return;
                }
            },
            'L' => {
                let assigned = match self.record() {
                    Some(OrganizationRecord::Node(node)) => {
                        node.role_version_id.as_deref().and_then(|id| {
                            self.snapshot
                                .records
                                .iter()
                                .find_map(|record| match record {
                                    OrganizationRecord::Role(role) if role.id == id => Some(role),
                                    _ => None,
                                })
                        })
                    }
                    Some(OrganizationRecord::Role(role)) => Some(role),
                    _ => None,
                };
                let latest = assigned
                    .and_then(|assigned| {
                        self.snapshot
                            .records
                            .iter()
                            .filter_map(|record| match record {
                                OrganizationRecord::Role(role)
                                    if role.role_id == assigned.role_id =>
                                {
                                    Some(role)
                                }
                                _ => None,
                            })
                            .max_by_key(|role| role.version)
                    })
                    .cloned();
                form::Kind::Role(latest)
            }
            'τ' => {
                let Some(view) = self.detail.as_ref().filter(|view| {
                    Some(&view.task.id) == self.selected.as_ref()
                        && view.task.state == "needsReview"
                }) else {
                    self.message = "Select an unaccepted reviewable result; repair reuses its original remaining grant.".into();
                    return;
                };
                let Some(result) = view.result.as_ref().filter(|result| !result.accepted) else {
                    return;
                };
                form::Kind::RepairResult {
                    task: view.task.clone(),
                    digest: result.digest.clone(),
                }
            }
            'H' | '&' => {
                let Some(view) = self.detail.as_ref().filter(|view| {
                    Some(&view.task.id) == self.selected.as_ref() && view.task.state == "blocked"
                }) else {
                    self.message =
                        "Select a blocked task and wait for its checkpoint details.".into();
                    return;
                };
                let Some(grant) = &view.authorization else {
                    return;
                };
                let Some(checkpoint) = view
                    .checkpoints
                    .first()
                    .filter(|checkpoint| checkpoint.authorization_id == grant.id)
                else {
                    self.message =
                        "No current checkpoint; inspect recovery evidence before continuing."
                            .into();
                    return;
                };
                let receivers = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|record| match record {
                        OrganizationRecord::Node(node)
                            if node.kind == OrganizationNodeKind::Agent
                                && node.enabled
                                && (node.id == view.task.owner_id
                                    || grant.scope.delegates.contains(&node.id)
                                    || view.checkpoints.iter().any(|checkpoint| {
                                        checkpoint.authorization_id == grant.id
                                            && checkpoint.owner_id == node.id
                                    })
                                    || view.handoffs.iter().any(|handoff| {
                                        handoff.authorization_id == grant.id
                                            && handoff.from_owner_id == node.id
                                    })) =>
                        {
                            Some(node.clone())
                        }
                        _ => None,
                    })
                    .collect();
                if key == '&' {
                    form::Kind::Replan {
                        task: view.task.clone(),
                        checkpoint: checkpoint.clone(),
                        receivers,
                    }
                } else {
                    form::Kind::Handoff {
                        task: view.task.clone(),
                        checkpoint: checkpoint.clone(),
                        receivers,
                    }
                }
            }
            'W' => match self.workflow_items() {
                Ok((project_id, items)) => form::Kind::WorkflowSave {
                    project_id,
                    items,
                    previous: None,
                },
                Err(e) => {
                    self.message = e.to_string();
                    return;
                }
            },
            'R' => match self.record() {
                Some(OrganizationRecord::Task(task)) if task.prerequisites.is_empty() => {
                    form::Kind::Schedule(task.clone())
                }
                _ => {
                    self.message =
                        "Select a task without prerequisites to create a recurring schedule."
                            .into();
                    return;
                }
            },
            'A' => {
                let Some(view) = self
                    .detail
                    .as_ref()
                    .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                else {
                    self.message = "Select an accepted task and wait for its details.".into();
                    return;
                };
                let Some(result) = view
                    .result
                    .as_ref()
                    .filter(|r| r.accepted && !r.artifacts.is_empty())
                else {
                    self.message = "Accept the exact result before choosing files to Apply.".into();
                    return;
                };
                if view.applications.iter().any(|a| a.digest == result.digest) {
                    self.message =
                        "Application already recorded; inspect its receipt before reconciliation."
                            .into();
                    return;
                }
                form::Kind::Apply {
                    task: view.task.clone(),
                    result: result.clone(),
                }
            }
            'U' => form::Kind::Agent,
            'G' => form::Kind::Goal,
            'N' => form::Kind::Project,
            'T' => form::Kind::Task,
            '+' => form::Kind::Discussion,
            '-' => {
                let Some(OrganizationRecord::Control(control)) = self.record() else {
                    self.message="Select a knowledge review control to restore it, or search for an active note and press -".into();
                    return;
                };
                let OrganizationControlValue::Knowledge {
                    note_id,
                    body_sha256,
                    ..
                } = &control.value
                else {
                    self.message = "Select a knowledge review control".into();
                    return;
                };
                form::Kind::Knowledge {
                    note_id: note_id.clone(),
                    body_sha256: body_sha256.clone(),
                    previous: Some(control.clone()),
                }
            }
            's' if self.selected_template().is_some() => {
                let Some(template) = self.selected_template().cloned() else {
                    return;
                };
                if template.items.is_empty() {
                    self.message = "Workflow has no tasks; refresh before starting".into();
                    return;
                }
                form::Kind::WorkflowRun { template }
            }
            'Y' => {
                let Some(template) = self.selected_template().cloned() else {
                    self.message =
                        "Select a saved workflow template before creating a finite series".into();
                    return;
                };
                let goal = self
                    .snapshot
                    .records
                    .iter()
                    .find_map(|r| match r {
                        OrganizationRecord::Project(p) if p.id == template.project_id => {
                            p.goal_id.as_ref()
                        }
                        _ => None,
                    })
                    .and_then(|id| {
                        self.snapshot.records.iter().find_map(|r| match r {
                            OrganizationRecord::Goal(g) if &g.id == id => Some(g.clone()),
                            _ => None,
                        })
                    });
                form::Kind::WorkflowSeries { template, goal }
            }
            's' => match self.record() {
                Some(OrganizationRecord::Task(task))
                    if matches!(
                        task.state.as_str(),
                        "proposed" | "paused" | "failed" | "needsReconciliation"
                    ) =>
                {
                    form::Kind::Authorize(task.clone())
                }
                _ => {
                    self.message =
                        "Select a proposed, paused, failed or reconciled task to authorize.".into();
                    return;
                }
            },
            _ => return,
        };
        self.form = Some(form::Form::new(
            kind,
            &self.snapshot.records,
            self.record(),
            &self.model,
        ));
        if key == 'R'
            && let Some(scope) = self
                .detail
                .as_ref()
                .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                .and_then(|v| v.authorization.as_ref())
                .map(|a| &a.scope)
            && let Some(form) = &mut self.form
        {
            form.prefill_scope(scope);
        }
        if let Some(form) = &mut self.form {
            form.planning_context(&self.snapshot.records, self.snapshot.report.as_ref());
        }
    }
    fn selection_task(&mut self) {
        let capture = (|| -> anyhow::Result<(String, crate::organization_editor::Selection)> {
            let project = self
                .selected_project()
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("Select the original project/task first"))?;
            let editor = self
                .editor
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("Open actual Neovim first"))?;
            let selection = editor.selection()?;
            let root = Path::new(
                project
                    .roots
                    .first()
                    .ok_or_else(|| anyhow::anyhow!("Project root missing"))?,
            )
            .canonicalize()?;
            let file = Path::new(&selection.path);
            let relative = file.strip_prefix(&root).map_err(|_| {
                anyhow::anyhow!(
                    "Capture an original project file, not a private preview or another project"
                )
            })?;
            codex_utils_path::open_relative_regular_file(&root, relative)?;
            Ok((project.id, selection))
        })();
        match capture {
            Ok((project_id, selection)) => {
                self.form = Some(form::Form::new(
                    form::Kind::Selection {
                        project_id,
                        selection,
                    },
                    &self.snapshot.records,
                    self.record(),
                    &self.model,
                ));
                self.message =
                    "Pinned actual selection · choose purpose · separate authorization is required"
                        .into();
            }
            Err(error) => {
                self.message = error.to_string();
                self.message_is_error = true;
            }
        }
    }
    fn validate_selection(
        &mut self,
        selection: &crate::organization_editor::Selection,
    ) -> anyhow::Result<()> {
        self.editor
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("Original editor closed; recapture selection"))?
            .validate_selection(selection)
    }
    fn open_memory(&mut self, private: bool) {
        let (project_id, agent_id) = match self.record() {
            Some(OrganizationRecord::Task(task)) => {
                (task.project_id.clone(), Some(task.owner_id.clone()))
            }
            Some(OrganizationRecord::Project(project)) if !private => (project.id.clone(), None),
            _ => {
                self.message = "Use Projects (F3): select a project/task for project memory, or a task for private agent memory.".into();
                return;
            }
        };
        self.memory_request = Some(OrganizationMemorySearchParams {
            project_id,
            scope: if private {
                OrganizationNoteScope::Agent
            } else {
                OrganizationNoteScope::Project
            },
            agent_id: if private { agent_id } else { None },
            query: String::new(),
        });
        self.memory_result = None;
        self.input = Some(String::new());
        self.changes = false;
        self.scroll = 0;
    }

    fn memory_text(&self) -> String {
        let Some(params) = &self.memory_request else {
            return String::new();
        };
        let mut text = format!(
            "Project: {}\nScope: {:?}\nAgent: {}\nLiteral query: {:?}\nNotes are untrusted context, not permissions.\n\n",
            params.project_id,
            params.scope,
            params.agent_id.as_deref().unwrap_or("project-shared"),
            params.query
        );
        match &self.memory_result {
            Some(Ok(result)) => {
                text.push_str(&format!("Showing {} of {} matching notes (newest first).\n", result.notes.len(), result.matched_count));
                text.push_str(&reporting::freshness(&result.notes, chrono::Utc::now().timestamp()));
                if result.matched_count == 0 { text.push_str("No matching notes.\n"); }
                for note in &result.notes {
                    text.push_str(&format!("Owner reviewed: {} · source: {} · team: {}\n",note.owner_reviewed,note.source_note_id.as_deref().unwrap_or("original"),note.group_id.as_deref().unwrap_or("none")));
                    text.push_str(&format!("\nNote: {}\nAuthor: {}\nTask: {}\nAttempt: {}\nCreated: {} (UTC seconds)\n{}\n",
                        note.id, note.author_id, note.task_id, note.attempt_id, note.created_at, note.body));
                }
            }
            Some(Err(error)) => text.push_str(&format!("Search failed: {error}")),
            None if self.input.is_some() => text.push_str("Enter a query; leave it empty to list recent notes.\nMaximum 256 bytes. Enter searches; Esc closes memory."),
            None => text.push_str("Loading scoped notes…"),
        }
        clean(&text)
    }

    fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if let Some(palette) = &mut self.palette {
            if key.code == KeyCode::Esc {
                self.palette = None;
            } else if let Some(code) = palette.key(key) {
                self.palette = None;
                self.key(KeyEvent::from(code));
            }
            return;
        }
        if key.code == KeyCode::Char('p')
            && key.modifiers == crossterm::event::KeyModifiers::CONTROL
            && self.form.is_none()
            && self.confirmation.is_none()
            && self.input.is_none()
            && self.report_text.is_none()
            && self.memory_request.is_none()
            && !self.editor.as_ref().is_some_and(|editor| editor.focused)
        {
            self.open_palette();
            return;
        }
        if self.report_text.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    self.report_text = None;
                    self.report_request = None;
                }
                KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
                _ => {}
            }
            return;
        }
        if self.preferences.preset == Preset::Workspace && self.memory_request.is_none() {
            if let Some(editor) = &mut self.editor
                && editor.focused
            {
                if key.code == KeyCode::Char('g')
                    && key
                        .modifiers
                        .contains(crossterm::event::KeyModifiers::CONTROL)
                {
                    editor.focused = false;
                    self.message =
                        "Host focus · e editor · o independent · t follow · d diff · n next file"
                            .into();
                } else if let Err(e) = editor.key(key) {
                    self.message = e.to_string();
                }
                return;
            }
            if self.confirmation.is_none() && self.input.is_none() && self.form.is_none() {
                match key.code {
                    KeyCode::F(7) => {
                        self.editor_session(false);
                        return;
                    }
                    KeyCode::F(8) => {
                        self.editor_session(true);
                        return;
                    }
                    KeyCode::Char('B') => {
                        self.buffer_proposal();
                        return;
                    }
                    KeyCode::Char('Q') => {
                        self.selection_task();
                        return;
                    }
                    KeyCode::Char('t') => {
                        self.follow_recent_file();
                        return;
                    }
                    KeyCode::Char('e' | 'o' | 'd') => {
                        self.editor_open(key.code);
                        return;
                    }
                    KeyCode::Char('n') => {
                        self.editor_file = self.editor_file.saturating_add(1);
                        self.editor_open(KeyCode::Char(if self.following { 't' } else { 'o' }));
                        return;
                    }
                    _ => {}
                }
            }
        }
        if self.confirmation.is_some() {
            match key.code {
                KeyCode::Char('y') => {
                    if matches!(
                        &self.confirmation,
                        Some(Command::RestoreEditor(..) | Command::BufferProposal(..))
                    ) {
                        let Some(command) = self.confirmation.take() else {
                            return;
                        };
                        let result = (|| -> anyhow::Result<()> {
                            anyhow::ensure!(
                                !self.applying,
                                "Application outcome pending; editor actions wait for reconciliation"
                            );
                            let roots = self.project_roots();
                            if let Command::BufferProposal(proposal) = &command {
                                proposal.verify_original(&roots)?;
                            }
                            if self.editor.is_none() {
                                self.editor = Some(Editor::start(self.frame.clone())?);
                            }
                            let editor = self.editor.as_mut().context("editor unavailable")?;
                            match command {
                                Command::RestoreEditor(session, recover) => {
                                    editor.restore_session(&session, &roots, recover)?
                                }
                                Command::BufferProposal(proposal) => {
                                    editor.apply_buffer_proposal(&proposal)?
                                }
                                _ => unreachable!(),
                            }
                            Ok(())
                        })();
                        self.form = None;
                        self.following = false;
                        self.message_is_error = result.is_err();
                        self.message=match result {Ok(())=>"Native editor decision applied; inspect buffers, undo or save explicitly".into(),Err(error)=>format!("Native editor decision refused or incomplete: {error}")};
                        return;
                    }

                    if let Some(selection) = self.selection_confirmation.clone()
                        && let Err(error) = self.validate_selection(&selection)
                    {
                        self.message = error.to_string();
                        self.message_is_error = true;
                        self.confirmation = None;
                        return;
                    }
                    self.selection_confirmation = None;
                    if matches!(
                        &self.confirmation,
                        Some(Command::Execution(
                            OrganizationExecutionMutation::Apply { .. }
                        ))
                    ) {
                        if self.applying {
                            self.message = "Previous application outcome is pending".into();
                            return;
                        }
                        if let Some(editor) = &mut self.editor
                            && let Err(e) = editor.freeze()
                        {
                            self.message = format!("Cannot freeze editor revision: {e}");
                            return;
                        }
                        let Some(command) = self.confirmation.take() else {
                            return;
                        };
                        match self.commands.try_send(command) {
                            Ok(()) => {
                                self.applying = true;
                                self.form = None;
                                if let Some(editor) = &mut self.editor {
                                    editor.focused = false;
                                }
                                self.message =
                                    "Applying exact accepted result; editor inputs frozen".into();
                            }
                            Err(_) => {
                                if let Some(editor) = &mut self.editor {
                                    let _ = editor.thaw();
                                }
                                self.message = "Owner queue busy; refresh before retrying".into();
                            }
                        }
                        return;
                    }
                    if let Some(c) = self.confirmation.take() {
                        if matches!(
                            c,
                            Command::NamedView { .. }
                                | Command::OwnerContact(..)
                                | Command::NativeTerminal(..)
                                | Command::ProjectExplorer(..)
                        ) {
                            self.form = None;
                            self.send(c);
                            return;
                        }
                        match self.commands.try_send(c) {
                            Ok(()) => {
                                self.form = None;
                                self.message =
                                    "Owner request submitted; waiting for its receipt.".into();
                            }
                            Err(_) => {
                                self.message =
                                    "Owner queue busy; no request sent. Review again to retry."
                                        .into()
                            }
                        }
                    }
                }
                KeyCode::Esc => self.confirmation = None,
                KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
                _ => {}
            }
            return;
        }
        if key.code == KeyCode::F(6) {
            self.selection_confirmation = self
                .form
                .as_ref()
                .and_then(|form| form.selection())
                .cloned();
            if let Some(selection) = self.selection_confirmation.clone()
                && let Err(error) = self.validate_selection(&selection)
            {
                self.message = error.to_string();
                self.message_is_error = true;
                return;
            }
        }
        if let Some(form) = &mut self.form {
            match key.code {
                KeyCode::Esc => self.form = None,
                KeyCode::F(6) => match form.command() {
                    Ok(command @ Command::ArtifactLibrary { .. }) => {
                        if let Command::ArtifactLibrary { id, .. } = &command {
                            self.report_request = Some(id.clone());
                        }
                        self.form = None;
                        self.report_text =
                            Some("Loading bounded captured artifact library…".into());
                        self.scroll = 0;
                        self.send(command);
                    }
                    Ok(Command::WorkflowPreview(request)) => {
                        self.send(Command::WorkflowPreview(request));
                        self.message = "Loading exact workflow preview; no work authorized".into();
                    }
                    Ok(command @ Command::SeriesPreview { .. }) => {
                        self.send(command);
                        self.message = "Loading exact finite series preview and source grant; no work authorized".into();
                    }
                    Ok(command) => {
                        self.confirmation = Some(command);
                        self.scroll = 0;
                        self.pane().inspector = true;
                        self.confirmation_text = clean(&self.details());
                        self.message = "Review exact scope · y confirm · Esc back to draft".into();
                    }
                    Err(e) => form.error = e.to_string(),
                },
                _ => form.key(key),
            }
            return;
        }
        if let Some(input) = &mut self.input {
            match key.code {
                KeyCode::Esc => {
                    self.input = None;
                    self.memory_request = None;
                    self.memory_result = None;
                }
                KeyCode::Enter => {
                    let value = self.input.take().unwrap_or_default();
                    if let Some(params) = &mut self.memory_request {
                        params.query = value;
                        let params = params.clone();
                        self.memory_result = None;
                        self.scroll = 0;
                        self.send(Command::MemorySearch(params));
                    } else if self.changes {
                        if let Some(task) = self.change_target.take() {
                            self.confirmation = Some(Command::Execution(
                                OrganizationExecutionMutation::RequestChanges {
                                    task_id: task.id,
                                    expected_revision: task.revision,
                                    instructions: value,
                                },
                            ));
                            self.scroll = 0;
                            self.pane().inspector = true;
                            self.pane().inspector_focus = true;
                            self.confirmation_text = clean(&self.details());
                            self.message = "Confirm request changes: y / Esc".into();
                        }
                    } else {
                        self.search = value;
                        self.rebuild();
                        self.inspect();
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(c)
                    if !c.is_control()
                        && input.len() + c.len_utf8()
                            <= if self.memory_request.is_some() {
                                256
                            } else {
                                4096
                            } =>
                {
                    input.push(c)
                }
                _ => {}
            }
            return;
        }
        if self.memory_request.is_some() {
            match key.code {
                KeyCode::Char('S') => {
                    if let Some(Ok(result)) = &self.memory_result
                        && let Some(note) = result.notes.first()
                    {
                        let projects = self
                            .snapshot
                            .records
                            .iter()
                            .filter_map(|r| match r {
                                OrganizationRecord::Project(p) => Some(p.clone()),
                                _ => None,
                            })
                            .collect();
                        self.form = Some(form::Form::new(
                            form::Kind::ShareNote {
                                note: note.clone(),
                                projects,
                            },
                            &self.snapshot.records,
                            self.record(),
                            &self.model,
                        ));
                        self.memory_request = None;
                        self.memory_result = None;
                        self.scroll = 0;
                    } else {
                        self.message =
                            "Search for the exact owner-reviewed project source before sharing"
                                .into();
                    }
                }
                KeyCode::Char('-') => {
                    if let Some(Ok(result)) = &self.memory_result
                        && let Some(note) = result.notes.first()
                    {
                        use sha2::Digest;
                        let kind = form::Kind::Knowledge {
                            note_id: note.id.clone(),
                            body_sha256: format!(
                                "{:x}",
                                sha2::Sha256::digest(note.body.as_bytes())
                            ),
                            previous: self.control(&format!("knowledge:{}", note.id)),
                        };
                        self.form = Some(form::Form::new(
                            kind,
                            &self.snapshot.records,
                            self.record(),
                            &self.model,
                        ));
                        self.memory_request = None;
                        self.memory_result = None;
                        self.scroll = 0;
                    } else {
                        self.message =
                            "Search for the exact source note before reviewing its retirement"
                                .into();
                    }
                }
                KeyCode::Char('O') => {
                    if let Some(Ok(result)) = &self.memory_result
                        && let Some(note) = result.notes.first()
                    {
                        self.form = Some(form::Form::new(
                            form::Kind::PromoteNote(note.clone()),
                            &self.snapshot.records,
                            self.record(),
                            &self.model,
                        ));
                        self.memory_request = None;
                        self.memory_result = None;
                        self.scroll = 0;
                    } else {
                        self.message = "Search for the source note before promoting it".into();
                    }
                }
                KeyCode::Esc => {
                    self.memory_request = None;
                    self.memory_result = None;
                    self.scroll = 0;
                }
                KeyCode::Char('/') => {
                    self.input = self
                        .memory_request
                        .as_ref()
                        .map(|params| params.query.clone())
                }
                KeyCode::Char('f') => {
                    if let Some(params) = self.memory_request.clone() {
                        self.memory_result = None;
                        self.send(Command::MemorySearch(params));
                    }
                }
                KeyCode::Up => self.scroll = self.scroll.saturating_sub(1),
                KeyCode::Down => self.scroll = self.scroll.saturating_add(1),
                KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
                KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
                _ => {}
            }
            return;
        }
        if self.graph_key(key) {
            return;
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q' | '1') | KeyCode::F(1) => {
                if let Some(editor) = &mut self.editor
                    && editor.error.is_none()
                {
                    match editor.dirty() {
                        Ok(false) => {}
                        Ok(true) => {
                            self.message="Save or discard modified Neovim buffers before closing Workspace; e returns to editor".into();
                            return;
                        }
                        Err(e) => {
                            self.message = e.to_string();
                            return;
                        }
                    }
                }
                if matches!(key.code, KeyCode::F(1) | KeyCode::Char('1')) {
                    self.preferences.preset = Preset::Console;
                }
                self.save();
                self.is_done = true;
            }
            KeyCode::F(n @ 2..=5) => self.switch(
                [
                    Preset::Console,
                    Preset::Workspace,
                    Preset::Projects,
                    Preset::Operations,
                    Preset::Review,
                ][n as usize - 1],
            ),
            KeyCode::F(23) => {
                let selected = self.selected.clone();
                if self.editor_open(KeyCode::Char('o')) {
                    self.switch(Preset::Workspace);
                    self.selected = selected;
                    self.inspect();
                    if let Some(editor) = &mut self.editor {
                        editor.focused = true;
                    }
                }
            }
            KeyCode::F(24) => self.open_form('~'),
            KeyCode::F(25) => self.send(Command::MoreRecords),
            KeyCode::F(26) => self.open_form('µ'),
            KeyCode::F(27) => self.open_form('π'),
            KeyCode::F(28) => self.open_form('γ'),
            KeyCode::F(29) => self.open_form('ε'),
            KeyCode::F(30) => self.open_form('λ'),
            KeyCode::F(38) => self.open_form('Z'),
            KeyCode::F(39) => self.open_form('ρ'),
            KeyCode::F(40) => self.open_form('χ'),
            KeyCode::F(41) => self.open_form('φ'),
            KeyCode::F(42) => self.open_form('ψ'),
            KeyCode::F(43) => self.open_form('δ'),
            KeyCode::F(45) => self.open_form('ζ'),
            KeyCode::F(46) => self.open_form('η'),
            KeyCode::F(47) => self.open_form('θ'),
            KeyCode::F(49) => self.open_form('ι'),
            KeyCode::F(50) => self.open_form('κ'),
            KeyCode::F(51) => self.open_form('ν'),
            KeyCode::F(53) => self.open_form('ξ'),
            KeyCode::F(59) => self.open_form('τ'),
            KeyCode::F(55) => {
                self.report_text = Some(reporting::owner_overview(
                    &self.snapshot,
                    self.preferences.executive_contact.as_deref(),
                ));
                self.report_request = None;
                self.scroll = 0;
                self.pane().inspector = true;
            }
            KeyCode::F(58) => {
                let Some(id) = self.preferences.executive_contact.clone().filter(|id| self.snapshot.records.iter().any(|r| matches!(r,OrganizationRecord::Node(n) if &n.id==id && n.enabled && n.kind==OrganizationNodeKind::Agent))) else {
                    self.message = "Choose an available executive contact first.".into();
                    return;
                };
                self.selected = Some(id);
                self.detail = None;
                self.open_form('δ');
            }
            KeyCode::F(56) | KeyCode::F(57) => {
                let id = if key.code == KeyCode::F(56) {
                    let Some(OrganizationRecord::Node(node)) = self.record().filter(|r| matches!(r, OrganizationRecord::Node(n) if n.enabled && n.kind==OrganizationNodeKind::Agent)) else {
                        self.message = "Select any enabled agent as your executive contact.".into();
                        return;
                    };
                    Some(node.id.clone())
                } else {
                    None
                };
                self.confirmation = Some(Command::OwnerContact(id));
                self.scroll = 0;
                self.pane().inspector = true;
                self.pane().inspector_focus = true;
                self.confirmation_text = clean(&self.details());
                self.message = "Confirm the displayed executive contact: y / Esc".into();
            }
            KeyCode::F(54) => self.follow_recent_file(),
            KeyCode::F(52) => {
                use Pane::*;
                let placements = [
                    [Sidebar, Main, Inspector],
                    [Sidebar, Inspector, Main],
                    [Main, Sidebar, Inspector],
                    [Main, Inspector, Sidebar],
                    [Inspector, Sidebar, Main],
                    [Inspector, Main, Sidebar],
                ];
                let pane = self.pane();
                let index = placements
                    .iter()
                    .position(|order| *order == pane.order)
                    .unwrap_or(0);
                pane.order = placements[(index + 1) % placements.len()];
                self.message = format!(
                    "Pane placement: {:?} · existing resize/hide controls remain available",
                    pane.order
                );
                self.save();
            }
            KeyCode::F(48) => {
                let Some(text) = self.agent_readiness_text() else {
                    self.message = "Select an agent for stored readiness evidence".into();
                    return;
                };
                self.report_text = Some(text);
                self.report_request = None;
                self.scroll = 0;
                self.pane().inspector = true;
            }
            KeyCode::F(44) => {
                let Some(OrganizationRecord::Control(control)) = self.record() else {
                    self.message = "Select a saved agent conversation".into();
                    return;
                };
                let id = control.id.clone();
                let Some(transcript) = self.conversation_transcript(&id) else {
                    self.message = "Select a conversation in owner controls".into();
                    return;
                };
                self.report_text = Some(transcript);
                self.report_request = Some(id);
                self.scroll = 0;
            }
            KeyCode::F(31) => {
                if let Some(project) = self.selected_project().cloned() {
                    self.form = Some(form::Form::new(
                        form::Kind::ArtifactLibrary(project.id),
                        &self.snapshot.records,
                        self.record(),
                        &self.model,
                    ));
                } else {
                    self.message = "Select a project or task for the artifact library".into();
                }
            }
            KeyCode::F(32) => {
                let result = self
                    .detail
                    .as_ref()
                    .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                    .context("select a task and wait for evidence")
                    .and_then(|v| {
                        artifacts::text(
                            v,
                            self.path.parent().context("owner home missing")?,
                            self.review_file,
                        )
                    });
                self.report_text =
                    Some(result.unwrap_or_else(|e| format!("Text/data preview unavailable: {e}")));
                self.scroll = 0;
            }
            KeyCode::F(33) => {
                if let Some(root) = self
                    .selected_project()
                    .and_then(|p| p.roots.first())
                    .cloned()
                {
                    self.confirmation = Some(Command::NativeTerminal(root.into()));
                    self.confirmation_text = clean(&self.details());
                    self.message =
                        "Open owner shell in actual Neovim · review cwd and authority · y / Esc"
                            .into();
                } else {
                    self.message = "Select a project or task for the native terminal".into();
                }
            }
            KeyCode::F(34) => self.captured_checks_panel(),
            KeyCode::F(35) => self.three_way_review(),
            KeyCode::F(36) => self.rollback_form(true),
            KeyCode::F(37) => self.rollback_form(false),
            KeyCode::Char(n @ '2'..='5') => self.switch(
                [
                    Preset::Console,
                    Preset::Workspace,
                    Preset::Projects,
                    Preset::Operations,
                    Preset::Review,
                ][n as usize - '1' as usize],
            ),
            KeyCode::Tab => {
                let pane = self.pane();
                pane.inspector_focus = !pane.inspector_focus;
                self.save();
            }
            KeyCode::Char('i') => {
                let pane = self.pane();
                pane.inspector = !pane.inspector;
                self.save();
            }
            KeyCode::Char('[') => {
                let pane = self.pane();
                pane.width = pane.width.saturating_sub(5).max(25);
                self.save();
            }
            KeyCode::Char(']') => {
                let pane = self.pane();
                pane.width = (pane.width + 5).min(75);
                self.save();
            }
            KeyCode::Char('\\') => {
                let pane = self.pane();
                pane.sidebar = !pane.sidebar;
                self.save();
            }
            KeyCode::Char('{') => {
                let pane = self.pane();
                pane.sidebar_width = pane.sidebar_width.saturating_sub(2).max(16);
                self.save();
            }
            KeyCode::Char('}') => {
                let pane = self.pane();
                pane.sidebar_width = (pane.sidebar_width + 2).min(40);
                self.save();
            }
            KeyCode::Char('~') => {
                let pane = self.pane();
                pane.actions = !pane.actions;
                self.save();
            }
            KeyCode::Char('0') => {
                *self.pane() = PaneSettings::default();
                self.save();
            }
            KeyCode::Char('k') if !self.pane().vim_keys => {}
            KeyCode::Char('j') if !self.pane().vim_keys => {}
            KeyCode::Char('K') => {
                let pane = self.pane();
                pane.vim_keys = !pane.vim_keys;
                self.save();
            }
            KeyCode::Up | KeyCode::Down | KeyCode::Char('k' | 'j') => {
                let down = matches!(key.code, KeyCode::Down | KeyCode::Char('j'));
                if self.pane().inspector_focus {
                    self.scroll = if down {
                        self.scroll.saturating_add(1)
                    } else {
                        self.scroll.saturating_sub(1)
                    };
                } else if !self.rows.is_empty() {
                    let old = self
                        .rows
                        .iter()
                        .position(|r| Some(&r.id) == self.selected.as_ref())
                        .unwrap_or(0);
                    let index = if down {
                        (old + 1).min(self.rows.len() - 1)
                    } else {
                        old.saturating_sub(1)
                    };
                    self.selected = Some(self.rows[index].id.clone());
                    self.inspect();
                }
            }
            KeyCode::PageDown => self.scroll = self.scroll.saturating_add(10),
            KeyCode::PageUp => self.scroll = self.scroll.saturating_sub(10),
            KeyCode::Left | KeyCode::Right | KeyCode::Enter
                if (self.preferences.preset == Preset::Operations
                    || self.projection == Projection::Goals)
                    && matches!(self.projection, Projection::Default | Projection::Goals) =>
            {
                if let Some(id) = &self.selected {
                    if key.code == KeyCode::Left
                        || (key.code == KeyCode::Enter && !self.collapsed.contains(id))
                    {
                        self.collapsed.insert(id.clone());
                    } else {
                        self.collapsed.remove(id);
                    }
                }
                self.rebuild();
            }
            KeyCode::Char('/') => {
                self.input = Some(self.search.clone());
                self.changes = false;
            }
            KeyCode::Char('f') => self.send(Command::Refresh),
            KeyCode::F(9) => {
                self.projection = if self.projection == Projection::Presets {
                    Projection::Default
                } else {
                    Projection::Presets
                };
                self.search.clear();
                self.selected = None;
                self.rebuild();
                self.inspect();
            }
            KeyCode::F(10) => self.open_form('O'),
            KeyCode::F(11) => {
                self.projection = if self.projection == Projection::Controls {
                    Projection::Default
                } else {
                    Projection::Controls
                };
                self.search.clear();
                self.rebuild();
                self.inspect();
            }
            KeyCode::F(12) => {
                let previous = self.control("global");
                let (paused, limits) = match previous.as_ref().map(|c| &c.value) {
                    Some(OrganizationControlValue::Global { paused, limits }) => {
                        (*paused, limits.clone())
                    }
                    _ => (false, None),
                };
                self.confirmation = Some(Command::Definition(OrganizationMutation::SetControl {
                    expected_revision: previous.map(|c| c.revision),
                    value: OrganizationControlValue::Global {
                        paused: !paused,
                        limits,
                    },
                }));
                self.scroll = 0;
                self.pane().inspector = true;
                self.confirmation_text = clean(&self.details());
                self.message =
                    "Review exact organization pause/resume · y confirms · Esc cancels".into();
            }
            KeyCode::Char('g') if key.modifiers.is_empty() => {
                self.projection = if self.projection == Projection::Goals {
                    Projection::Default
                } else {
                    Projection::Goals
                };
                self.search.clear();
                self.selected = None;
                self.rebuild();
                self.inspect();
            }
            KeyCode::Char('w') => {
                self.projection = if self.projection == Projection::Workflows {
                    Projection::Default
                } else {
                    Projection::Workflows
                };
                self.selected = None;
                self.search.clear();
                self.rebuild();
                self.inspect();
                self.message =
                    "Workflows · W saves selected task chain · s previews selected template".into();
            }
            KeyCode::Char('S') => {
                self.projection = if self.projection == Projection::Schedules {
                    Projection::Default
                } else {
                    Projection::Schedules
                };
                self.search.clear();
                self.selected = None;
                self.rebuild();
                self.inspect();
            }
            KeyCode::Char('z' | 'Z') => {
                let focus = if key.code == KeyCode::Char('Z') {
                    reporting::Focus::All
                } else {
                    reporting::focus(
                        self.record(),
                        self.selected.as_deref(),
                        &self.snapshot.workflows,
                    )
                };
                self.report_text = Some("Loading canonical summary…".into());
                self.scroll = 0;
                let id = uuid::Uuid::new_v4().to_string();
                self.report_request = Some(id.clone());
                self.send(Command::Summary(id, focus));
            }
            KeyCode::Char('h') => self.hunk_form(),
            KeyCode::Char('C') => {
                self.report_text = Some("Loading configured capabilities…".into());
                self.scroll = 0;
                let id = uuid::Uuid::new_v4().to_string();
                self.report_request = Some(id.clone());
                self.send(Command::Inventory(id));
            }
            KeyCode::Char('m') => self.open_memory(false),
            KeyCode::Char('M') => self.open_memory(true),
            KeyCode::Char('J') => {
                self.open_memory(true);
                if let Some(request) = &mut self.memory_request {
                    request.scope = OrganizationNoteScope::Team;
                }
            }
            KeyCode::Char(
                c @ ('U' | 'G' | 'N' | 'T' | 's' | 'A' | 'R' | 'W' | 'H' | 'B' | 'E' | 'L' | 'I'
                | 'Y' | 'D' | 'V' | '!' | 'F' | '*' | '&' | '+' | '-' | '@' | '?' | '^' | '$'),
            ) => self.open_form(c),
            KeyCode::Char('n') if self.preferences.preset == Preset::Review => {
                if let Some(result) = self
                    .detail
                    .as_ref()
                    .filter(|v| Some(&v.task.id) == self.selected.as_ref())
                    .and_then(|v| v.result.as_ref())
                {
                    self.review_file = (self.review_file + 1) % result.artifacts.len().max(1);
                    self.editor_file = self.review_file;
                    self.review_diff = None;
                    self.refresh_diff();
                    self.scroll = 0;
                }
            }
            KeyCode::Char('b' | 'P') => self.open_form(if key.code == KeyCode::Char('P') {
                'P'
            } else {
                'b'
            }),
            KeyCode::Char(c @ ('a' | 'r' | 'v' | 'c' | 'p' | 'x' | 'u' | '%')) => self.decision(c),
            _ => {}
        }
    }
    fn update(&mut self) {
        while let Ok(update) = self.updates.try_recv() {
            match update {
                Update::Report(id, result, focus) => {
                    if self.report_request.as_ref() == Some(&id) && self.report_text.is_some() {
                        self.report_text = Some(clean(&match result {
                            Ok(report) => reporting::text(&report, &focus, self.prices.as_ref()),
                            Err(error) => format!("Summary unavailable: {error}"),
                        }));
                    }
                }
                Update::ReportText(id, result) => {
                    if self.report_request.as_ref() == Some(&id) && self.report_text.is_some() {
                        self.report_text =
                            Some(clean(&result.unwrap_or_else(|error| {
                                format!("Inventory unavailable: {error}")
                            })));
                    }
                }
                Update::Snapshot(snapshot) => {
                    let first = self.snapshot.records.is_empty();
                    let selected = self.selected.clone();
                    self.preferences.graph.cpu_percent = self.snapshot.report.as_ref().and_then(|r|r.runtime_resources.as_ref()).zip(snapshot.report.as_ref().and_then(|r|r.runtime_resources.as_ref())).and_then(|(old,new)| {
                        if old.process_id!=new.process_id || new.measured_at_ms<=old.measured_at_ms {return None;}
                        let elapsed=new.measured_at_ms.checked_sub(old.measured_at_ms)? as f64;
                        let cpu=new.cpu_time_ms?.checked_sub(old.cpu_time_ms?)? as f64;
                        Some(100.0*cpu/elapsed)
                    });
                    let changed=self.snapshot.records!=snapshot.records || self.snapshot.workflows!=snapshot.workflows || self.snapshot.schedules!=snapshot.schedules;
                    self.snapshot = *snapshot;
                    if !self.setup_offered {
                        self.setup_offered = true;
                        if self.snapshot.records.is_empty() && self.form.is_none() && self.confirmation.is_none() {
                            self.open_form('Z');
                        }
                    }
                    if changed {self.rebuild();}
                    if self.report_text.is_some() && let Some(id)=self.report_request.as_ref().filter(|id|id.starts_with("conversation:")) {
                        self.report_text=self.conversation_transcript(id);
                    }
                    self.detail_text = clean(&self.details());
                    if self.confirmation.is_none()
                        && self.memory_request.is_none()
                        && !self.message_is_error
                        && !self.applying
                    {
                        self.message = if self.snapshot.limited {"Partial record pages loaded · Commands → Load more records; counts cover loaded definitions."} else {"Owner state loaded; definitions do not imply running agents."}.into();
                    }
                    if first || self.selected != selected {
                        self.inspect();
                    }
                }
                Update::Detail(view) => {
                    if Some(&view.task.id) == self.selected.as_ref() {
                        self.detail = Some(view);
                        self.refresh_diff();
                        self.detail_text = clean(&self.details());
                    }
                }
                Update::Error(e) => {
                    self.message = e;
                    self.message_is_error = true;
                }
                Update::ApplyRejected(e) => self.finish_application(e, true),
                Update::ApplyFinished => {
                    self.finish_application("Exact accepted result applied.".into(), false)
                }
                Update::Memory(params, result) => {
                    if self.memory_request.as_ref() == Some(&params) {
                        self.memory_result = Some(result);
                    }
                }
                Update::Changed => {
                    // An unrelated successful decision cannot settle an uncertain Apply.
                    if !self.applying {
                        self.message = "Owner decision recorded.".into();
                        self.message_is_error = false;
                    }
                    self.send(Command::Refresh);
                }
                Update::Saved(record) => {
                    if matches!(record, OrganizationRecord::Preset(_)) {
                        self.projection = Projection::Presets;
                    } else if matches!(record, OrganizationRecord::Control(_)) {
                        self.projection = Projection::Controls;
                    }
                    let id = record_id(&record).map(str::to_string);
                    if let Some(id) = &id {
                        self.snapshot
                            .records
                            .retain(|r| record_id(r) != Some(id.as_str()));
                    }
                    self.snapshot.records.push(record);
                    self.selected = id;
                    self.search.clear();
                    self.rebuild();
                    self.inspect();
                }
                Update::WorkflowPreview(preview) => {
                    // Ignore a late response after the owner closes/changes the draft.
                    if self.form.as_ref().is_some_and(|form| matches!(form.command(), Ok(Command::WorkflowPreview(request)) if request.id == preview.id && request.template_id == preview.template.id && request.scopes == preview.scopes && request.shared_limits == preview.shared_limits)) {
                        self.confirmation = Some(Command::Workflow(OrganizationWorkflowMutation::Run { preview }));
                        self.scroll = 0;
                        self.pane().inspector = true;
                        self.confirmation_text = clean(&self.details());
                        self.message = "Review exact workflow tasks/scopes · y start · Esc back".into();
                    }
                }
                Update::SeriesPreview { request, config, source_task, supervision, preview, trigger } => {
                    if preview.id == request.id && preview.template.id == request.template_id && preview.template.revision == request.expected_revision && preview.scopes == request.scopes && preview.shared_limits == request.shared_limits
                        && self.form.as_ref().is_some_and(|form| matches!(form.command(), Ok(Command::SeriesPreview { request: current, config: settings, source_task: source, supervision: policy }) if current == request && settings == config && source == source_task && policy == supervision)) {
                        let mutation = if let Some((goal, work_type)) = supervision {
                            OrganizationWorkflowMutation::CreateSupervision { id: request.id, preview, config, trigger, goal, work_type }
                        } else { OrganizationWorkflowMutation::CreateSeries { id: request.id, preview, config, trigger } };
                        self.confirmation = Some(Command::Workflow(mutation));
                        self.scroll = 0;
                        self.confirmation_text = clean(&self.details());
                        self.message = "Review exact finite series/source grant · y confirm · Esc back".into();
                    }
                }
                Update::WorkflowSaved(view) => {
                    let id = workflow_id(&view);
                    self.snapshot.workflows.retain(|v| workflow_id(v) != id);
                    self.snapshot.workflows.push(*view);
                    self.selected = Some(id);
                    self.projection = Projection::Workflows;
                    self.search.clear();
                    self.rebuild();
                    self.inspect();
                }
                Update::ScheduleSaved(view, created) => {
                    if created {
                        self.selected = Some(format!("schedule:{}", view.schedule.id));
                    }
                    self.snapshot
                        .schedules
                        .retain(|v| v.schedule.id != view.schedule.id);
                    self.snapshot.schedules.push(*view);
                    if created {
                        self.projection = Projection::Schedules;
                        self.search.clear();
                    }
                    self.rebuild();
                    self.inspect();
                }
            }
        }
    }
    fn finish_application(&mut self, message: String, rejected: bool) {
        self.applying = false;
        self.message = message;
        self.message_is_error = rejected;
        if let Some(editor) = &mut self.editor {
            let outcome = editor.thaw().and_then(|()| {
                if rejected {
                    Ok(())
                } else {
                    editor.reload_follow()
                }
            });
            if let Err(error) = outcome {
                self.message
                    .push_str(&format!("; editor protection: {error}"));
                self.message_is_error = true;
            }
        }
        self.send(Command::Refresh);
    }
    fn resource_preview(&self, project: &str, scopes: &[&OrganizationExecutionScope]) -> String {
        let tokens = scopes
            .iter()
            .fold(0i64, |sum, s| sum.saturating_add(s.max_tokens));
        let requests = scopes.iter().fold(0u64, |sum, s| {
            sum.saturating_add(u64::from(s.max_model_requests))
        });
        let seconds = scopes.iter().fold(0u64, |sum, s| {
            sum.saturating_add(u64::from(s.max_seconds) * u64::from(s.max_attempts))
        });
        let mut text = format!(
            "\nRESOURCE PREVIEW · {} proposed grants\nDemand ceilings: {tokens} reported tokens / {requests} admitted requests / {seconds} summed attempt seconds (not a runtime prediction)\n",
            scopes.len()
        );
        if let (
            Some(OrganizationControl {
                value:
                    OrganizationControlValue::Global {
                        limits: Some(limit),
                        ..
                    },
                ..
            }),
            Some(usage),
        ) = (
            self.control("global"),
            self.snapshot
                .report
                .as_ref()
                .and_then(|r| r.global_usage.as_ref()),
        ) {
            text.push_str(&format!("Loaded global remaining: {} tokens / {} requests / {} active slots; {} unresolved terminal requests\n",limit.max_tokens.saturating_sub(usage.reported_tokens).max(0),i64::from(limit.max_model_requests).saturating_sub(usage.model_requests).max(0),i64::from(limit.max_active).saturating_sub(usage.active).max(0),usage.unresolved_requests));
        } else {
            text.push_str("Global remaining unavailable or no shared ceiling configured; native per-grant/capacity fences still apply.\n");
        }
        for view in &self.snapshot.workflows {
            if let OrganizationWorkflowView::Limit { limit, usage } = view
                && limit.project_id == project
                && limit.run_id.is_none()
                && limit.series_id.is_none()
            {
                text.push_str(&format!("Loaded project pool {} v{} remaining: {} tokens / {} requests / {} active slots\n",limit.id,limit.revision,limit.max_tokens.saturating_sub(usage.reported_tokens).max(0),i64::from(limit.max_model_requests).saturating_sub(usage.model_requests).max(0),i64::from(limit.max_active).saturating_sub(usage.active).max(0)));
            }
        }
        for scope in scopes {
            if let Some(estimate) = self
                .prices
                .as_ref()
                .and_then(|p| p.estimate(&self.provider, &scope.model, 0, scope.max_tokens))
            {
                text.push_str(&estimate);
            } else {
                text.push_str(&format!(
                    "{} quote estimate unavailable: no matching owner price provenance.\n",
                    scope.model
                ));
            }
        }
        text.push_str("Loaded usage is advice, not a reservation. Backend admission rechecks current ledgers, pauses, unknown effects and exact grants. Already-admitted usage may exceed token ceilings; monetary estimates are not invoices or hard spending caps.\n");
        text
    }
    fn details(&self) -> String {
        if let Some(text) = self.node_tab_text() {
            return text;
        }
        let mut text = self.workflow_details();
        if let Some(view) = self.selected_schedule() {
            let s = &view.schedule;
            text = format!(
                "Schedule: {}\n{}\nState: {} · revision {}\nProject: {}\nOwner: {}\nTemplate: {} · revision {}\nEvery {} seconds\nNext start (UTC): {}\nEnd (UTC): {}\nIssued: {} / {} · skipped: {}\nReported tokens: {} / {}\nModel requests: {} / {}\n{}\n\nMissed runs are skipped; executions never overlap.\nPause/cancel revoke current grants; resume starts future runs.\nAcceptance and Apply remain separate owner decisions.\n\nRecent occurrences (newest 16):\n",
                s.config.label,
                s.id,
                s.state.as_str(),
                s.revision,
                s.template.project_id,
                s.template.owner_id,
                s.template.id,
                s.template.revision,
                s.config.interval_seconds,
                s.next_run_at.map(utc_time).unwrap_or_else(|| "None".into()),
                utc_time(s.config.ends_at),
                s.issued_runs,
                s.config.max_runs,
                s.skipped_runs,
                view.used_tokens,
                s.config.max_tokens,
                view.model_requests,
                s.config.max_model_requests,
                s.reason.as_deref().unwrap_or("Ready for future runs")
            );
            for o in &view.occurrences {
                text.push_str(&format!(
                    "{} · [{}]\nTask: {}\n",
                    utc_time(o.due_at),
                    o.task_state,
                    o.task_id
                ));
            }
        }
        if let Some(record) = self.record() {
            match record {
                OrganizationRecord::Control(control) => {
                    text = format!(
                        "OWNER CONTROL · revision {}\n{}\n\n{}\n\nF10 organization limits · F12 pause/resume · F project lifecycle · ! task priority\nImmutable versions: organization list controlHistory\nChanges preserve original grants, deadlines and accounting.\n",
                        control.revision,
                        control.id,
                        serde_json::to_string_pretty(&control.value).unwrap_or_default()
                    );
                    if let OrganizationControlValue::Improvement {
                        baseline_task_id,
                        baseline_digest,
                        candidate_task_id,
                        candidate_digest,
                        ..
                    } = &control.value
                    {
                        let evidence = |id: &str, digest: &str| {
                            self.snapshot.report.as_ref().and_then(|r| {
                                r.tasks.iter().find(|t| {
                                    t.task_id == id && t.result_digest.as_deref() == Some(digest)
                                })
                            })
                        };
                        if let (Some(before), Some(after)) = (
                            evidence(baseline_task_id, baseline_digest),
                            evidence(candidate_task_id, candidate_digest),
                        ) {
                            text.push_str(&format!("\nRecorded comparison: baseline {} tokens / {} admitted requests / {} passed checks; candidate {} tokens / {} admitted requests / {} passed checks.\nSame protected oracle hashes and captured instruction versions are validated by the backend. These samples establish measured non-regression, not general model quality. % reviews exact adoption; existing grants stay fenced by their original node revision.\n",before.reported_tokens,before.model_requests,before.checks_passed,after.reported_tokens,after.model_requests,after.checks_passed));
                        } else {
                            text.push_str("\nCaptured evaluation report is unavailable; refresh before reviewing adoption.\n");
                        }
                    }
                    if matches!(control.value, OrganizationControlValue::Team { .. }) {
                        text.push_str("\n% reviews and applies this exact saved definition delta atomically. Stale revisions/cycles refuse the whole change. No authority is copied; retirement retains history.\n");
                    }
                    if let OrganizationControlValue::WorkflowImprovement {
                        baseline_run_id,
                        candidate_run_id,
                        ..
                    } = &control.value
                        && let Some(report) = &self.snapshot.report
                    {
                        for (label, id) in [
                            ("Baseline", baseline_run_id),
                            ("Candidate", candidate_run_id),
                        ] {
                            let tasks: Vec<_> = report
                                .tasks
                                .iter()
                                .filter(|t| t.workflow_run_id.as_ref() == Some(id))
                                .collect();
                            if tasks.is_empty() {
                                text.push_str(&format!(
                                    "\n{label} captured run report unavailable; refresh.\n"
                                ));
                            } else {
                                text.push_str(&format!("\n{label} captured run: {} tasks, {} reported tokens, {} admitted requests, {} passed checks.\n",tasks.len(),tasks.iter().map(|t|t.reported_tokens).sum::<i64>(),tasks.iter().map(|t|t.model_requests).sum::<i64>(),tasks.iter().map(|t|t.checks_passed).sum::<i64>()));
                            }
                        }
                        text.push_str("\n% confirms durable owner recommendation of this exact tested template; scopes, future runs and Apply remain separate decisions. Representative samples do not establish general quality.\n");
                    }
                    if let Some(u) = self
                        .snapshot
                        .report
                        .as_ref()
                        .and_then(|r| r.global_usage.as_ref())
                    {
                        text.push_str(&format!("\nWhole organization: {} reported tokens · {} admitted requests · {} active · {} unresolved terminal requests\n", u.reported_tokens, u.model_requests, u.active, u.unresolved_requests));
                    } else {
                        text.push_str(
                            "\nWhole organization usage unavailable; refresh the daemon report.\n",
                        );
                    }
                }
                OrganizationRecord::Preset(preset) => {
                    text = format!(
                        "Organization preset · {}\n{}\nRevision: {}\n{} captured nodes · {} roles · {} internal memberships\nV clones fresh definitions beneath a named group; no grants/tasks/private knowledge are copied.\n\n{}",
                        preset.label,
                        preset.id,
                        preset.revision,
                        preset.nodes.len(),
                        preset.roles.len(),
                        preset.memberships.len(),
                        serde_json::to_string_pretty(preset).unwrap_or_default()
                    );
                }
                OrganizationRecord::Node(n) => {
                    text = format!(
                        "{}\n{} · {}\nModel: {}\n\n",
                        n.label,
                        n.id,
                        if n.enabled {
                            "Enabled"
                        } else {
                            "Retired / disabled (records retained)"
                        },
                        n.model.as_deref().unwrap_or("Configured default")
                    );
                    if let Some(role) = self.snapshot.records.iter().find_map(|r| match r {
                        OrganizationRecord::Role(role)
                            if n.role_version_id.as_ref() == Some(&role.id) =>
                        {
                            Some(role)
                        }
                        _ => None,
                    }) {
                        text.push_str(&format!(
                            "Role: {} v{}\n{}\n\n",
                            role.name, role.version, role.instructions
                        ));
                    }
                    text.push_str(&format!("Kind: {:?}\nPrimary parent: {}\nRevision: {}\nE edits / reparents / retires · L creates a role version · I edits memberships\n",n.kind,n.parent_id.as_deref().unwrap_or("None"),n.revision));
                    for record in &self.snapshot.records {
                        if let OrganizationRecord::Membership(member) = record
                            && member.node_id == n.id
                        {
                            let label = self
                                .snapshot
                                .records
                                .iter()
                                .find_map(|record| match record {
                                    OrganizationRecord::Node(group)
                                        if group.id == member.group_id =>
                                    {
                                        Some(group.label.as_str())
                                    }
                                    _ => None,
                                })
                                .unwrap_or("Group not loaded");
                            text.push_str(&format!(
                                "Additional group: {label} · {}\n",
                                member.group_id
                            ));
                        }
                    }
                }
                OrganizationRecord::Project(p) => {
                    let tasks: Vec<_> = self
                        .snapshot
                        .records
                        .iter()
                        .filter_map(|r| match r {
                            OrganizationRecord::Task(t) if t.project_id == p.id => Some(t),
                            _ => None,
                        })
                        .collect();
                    text = format!(
                        "{}\n{}\nRoots: {}\nProgress: {} / {} tasks accepted\n\nCriteria:\n{}\n",
                        p.name,
                        p.id,
                        p.roots.join("\n"),
                        tasks.iter().filter(|t| t.state == "accepted").count(),
                        tasks.len(),
                        p.criteria.join("\n")
                    );
                    if let Some(control) = self.control(&format!("project:{}", p.id)) {
                        text.push_str(&format!("\nLifecycle · revision {}\n{}\nF reviews lifecycle; prior evidence is retained in controlHistory.\n", control.revision, serde_json::to_string_pretty(&control.value).unwrap_or_default()));
                    } else {
                        text.push_str("\nLifecycle: active · F pause/complete/archive/reopen\n");
                    }
                }
                OrganizationRecord::Goal(g) => {
                    text = format!(
                        "Goal\n{}\n{}\n\nCriteria:\n{}\n",
                        g.id,
                        g.objective,
                        g.criteria.join("\n")
                    );
                    if let Some(control) = self.control(&format!("goal-meta:{}", g.id)) {
                        text.push_str(&format!(
                            "\nResponsibility / exact prerequisites:\n{}\n",
                            serde_json::to_string_pretty(&control.value).unwrap_or_default()
                        ));
                    }
                    if let Some(control) = self.control(&format!("goal:{}", g.id)) {
                        text.push_str(&format!(
                            "\nOwner criterion evidence:\n{}\n",
                            serde_json::to_string_pretty(&control.value).unwrap_or_default()
                        ));
                    }
                    text.push_str("\nAccepted child outcomes can support parent criteria; completion is always an explicit owner review.\n");
                }
                OrganizationRecord::Task(t) => {
                    text = format!(
                        "Task {}\nState: {} · revision {}\nProject: {}\nOwner: {}\n\n{}\n\nCriteria:\n{}\nPrerequisites: {}\n",
                        t.id,
                        t.state,
                        t.revision,
                        t.project_id,
                        t.owner_id,
                        t.instructions,
                        t.criteria.join("\n"),
                        t.prerequisites.join(", ")
                    );
                    text.push_str(match t.state.as_str() {
                        "proposed"=>"\nNext: s reviews exact local inputs, artifacts, checks, worker roster and finite budgets; saving a task starts no execution.\n",
                        "waitingApproval"=>"\nNext: inspect the exact pending action and a approves / r rejects it.\n",
                        "needsReview"=>"\nNext: inspect recorded artifacts/checks; v explicitly accepts or c requests changes.\n",
                        "accepted"=>"\nNext: W reviews plan.json as an editable workflow; s previews a selected template's exact scopes. A separately selects Apply; Commands can capture a private lesson.\n",
                        "blocked"=>"\nNext: inspect the verified checkpoint; H explicitly hands off under remaining original limits. Commands routes local assistance.\n",
                        "needsReconciliation"=>"\nNext: inspect unknown effects and accounting; do not replay. Local assistance records the required owner decision.\n",
                        _=>"\nNext: inspect progress and prerequisites; pause/cancel and evidence stay available independently of the frontend.\n",
                    });
                }
                _ => {}
            }
            let date_target = match record {
                OrganizationRecord::Goal(g) => Some(("goal", g.id.as_str())),
                OrganizationRecord::Project(p) => Some(("project", p.id.as_str())),
                OrganizationRecord::Task(t) => Some(("task", t.id.as_str())),
                _ => None,
            };
            if let Some((kind, id)) = date_target
                && let Some(OrganizationControl {
                    value:
                        OrganizationControlValue::Deadline {
                            due_at: Some(due),
                            display_offset_minutes,
                            ..
                        },
                    ..
                }) = self.control(&format!("deadline:{kind}:{id}"))
                && let Some((date, offset)) = chrono::DateTime::from_timestamp(due, 0)
                    .zip(chrono::FixedOffset::east_opt(display_offset_minutes * 60))
            {
                let closed=match record {
                    OrganizationRecord::Task(t)=>matches!(t.state.as_str(),"accepted"|"cancelled"),
                    OrganizationRecord::Project(p)=>self.control(&format!("project:{}",p.id)).is_some_and(|c|matches!(c.value,OrganizationControlValue::Project {state:OrganizationProjectState::Completed|OrganizationProjectState::Archived,..})),
                    OrganizationRecord::Goal(g)=>self.control(&format!("goal:{}",g.id)).is_some_and(|c|matches!(c.value,OrganizationControlValue::GoalReview {goal,completed:true,..} if *goal==*g)),
                    _=>false,
                };
                let status = if closed {
                    "work closed"
                } else if due < chrono::Utc::now().timestamp() {
                    "overdue"
                } else {
                    "upcoming"
                };
                text.push_str(&format!("\nBusiness due: {} · {status}\nExecution expiry is independently fixed by its grant.\n",date.with_timezone(&offset).to_rfc3339()));
            }
            let related: Vec<_> = self
                .snapshot
                .records
                .iter()
                .filter_map(|r| match (record, r) {
                    (OrganizationRecord::Node(n), OrganizationRecord::Task(t))
                        if n.id == t.owner_id =>
                    {
                        Some(t)
                    }
                    (OrganizationRecord::Project(p), OrganizationRecord::Task(t))
                        if p.id == t.project_id =>
                    {
                        Some(t)
                    }
                    (OrganizationRecord::Goal(g), OrganizationRecord::Task(t))
                        if t.goal_id.as_ref() == Some(&g.id) =>
                    {
                        Some(t)
                    }
                    _ => None,
                })
                .take(128)
                .collect();
            if !related.is_empty() {
                text.push_str("\nAssigned work (up to 128):\n");
                for task in related {
                    text.push_str(&format!(
                        "[{}] {} · {}\n",
                        task.state,
                        task.id,
                        task.instructions.lines().next().unwrap_or("Task")
                    ));
                }
            }
        }
        if let Some(view) = self
            .detail
            .as_ref()
            .filter(|v| Some(&v.task.id) == self.selected.as_ref())
        {
            if let Some(grant) = &view.authorization {
                if let Some(budget) = &grant.scope.cost_budget {
                    match budget.admission_estimate(&grant.scope.model, grant.scope.max_tokens) {
                            Ok(estimate) => text.push_str(&format!(
                                "\nCaptured USD admission estimate: ${:.6} / ${:.6} planned limit\nUnknown-token allowance: {} · rate {}\nQuote: {}\nNot an invoice cap; non-token charges excluded.\n",
                                estimate as f64 / 1_000_000.0,
                                budget.max_usd_micros as f64 / 1_000_000.0,
                                budget.unknown_token_allowance,
                                budget.rate_version,
                                budget.rate_source,
                            )),
                            Err(_) => text.push_str("\nCaptured cost quote is invalid; no estimate available.\n"),
                        }
                }
                text.push_str(&format!("\nShared budget: {} / {} tokens · {} / {} requests\nModel: {} · max {} seconds · children {}\nInputs: {}\nRead-only: {}\nArtifacts: {}\n",view.authorization_used_tokens,grant.scope.max_tokens,view.authorization_model_requests,grant.scope.max_model_requests,grant.scope.model,grant.scope.max_seconds,grant.scope.max_children,grant.scope.input_files.join(", "),grant.scope.immutable_files.join(", "),grant.scope.artifacts.join(", ")));
            }
            if let Some(grant) = &view.authorization {
                text.push_str(
                    &self
                        .prices
                        .as_ref()
                        .and_then(|prices| {
                            prices.estimate(
                                &self.provider,
                                &grant.scope.model,
                                view.authorization_used_tokens,
                                grant.scope.max_tokens,
                            )
                        })
                        .unwrap_or_else(|| {
                            if grant.scope.cost_budget.is_some() {
                                "Current display-price file has no matching entry; the captured admission quote is shown separately.\n".into()
                            } else {
                                "Model price unavailable: no matching trusted owner quote.\n".into()
                            }
                        }),
                );
            }
            for attempt in &view.attempts {
                text.push_str(&format!(
                    "\nAttempt {} · {:?} · active: {}\nWorkspace: {}\nThread: {}\n",
                    attempt.id,
                    attempt.state,
                    attempt.runtime_active,
                    attempt.workspace.as_deref().unwrap_or("Not prepared"),
                    attempt.thread_id
                ));
                text.push_str(&format!(
                    "Last model/tool progress: {}\nStall threshold: {}\nStop reason: {}\n",
                    attempt
                        .last_progress_at
                        .map(utc_time)
                        .unwrap_or_else(|| "No activity recorded".into()),
                    attempt
                        .stall_seconds
                        .map(|seconds| format!("{seconds} seconds"))
                        .unwrap_or_else(|| "Disabled (task deadline still applies)".into()),
                    attempt.stop_reason.as_deref().unwrap_or("None")
                ));
            }
            for checkpoint in &view.checkpoints {
                text.push_str(&format!("\nCheckpoint: {} · {}\nSource attempt: {} · owner: {}\nReported completed work:\n{}\nRemaining work:\n{}\nBlocker / owner decision: {}\nVerified completed-action IDs: {}\n",checkpoint.id,utc_time(checkpoint.created_at),checkpoint.attempt_id,checkpoint.owner_id,
                    checkpoint.content.completed_steps.join("\n"),checkpoint.content.remaining_steps.join("\n"),
                    checkpoint.content.blocker.as_deref().unwrap_or("None"),checkpoint.completed_action_ids.join(", ")));
                for file in &checkpoint.files {
                    text.push_str(&format!(
                        "{} · {} bytes · SHA256 {}\n",
                        file.path, file.bytes, file.sha256
                    ));
                }
                text.push_str("Reported progress does not replace checks or acceptance. H resolves / reassigns under the same remaining grant.\n");
            }
            for handoff in &view.handoffs {
                text.push_str(&format!("\nHandoff: {} · checkpoint {}\n{} → {} · same grant {}\nOwner resolution: {}\n",handoff.id,handoff.checkpoint_id,handoff.from_owner_id,handoff.to_owner_id,handoff.authorization_id,handoff.instructions));
            }
            for approval in &view.approvals {
                text.push_str(&format!("\nApproval {} · {:?}\nTool: {}\nExact arguments: {}\nHash: {}\nExpires: {} (UTC seconds)\n",approval.id,approval.state,approval.tool,approval.arguments,approval.action_hash,approval.expires_at));
            }
            for action in &view.actions {
                text.push_str(&format!(
                    "\nAction {} · {:?} · {}\nReceipt: {}\n",
                    action.id,
                    action.state,
                    action.tool,
                    action.receipt.as_deref().unwrap_or("Not recorded")
                ));
            }
            {
                let reserved: i64 = view
                    .token_reservations
                    .iter()
                    .filter(|r| !r.released)
                    .map(|r| r.tokens)
                    .sum();
                text.push_str(&format!("\nNominal token reservations: {reserved}; provider output may exceed a reservation. Unknown usage retains its allowance.\n"));
                for reservation in &view.token_reservations {
                    text.push_str(&format!(
                        "Request {} · attempt {} · {} nominal tokens · {}\n",
                        reservation.sequence,
                        reservation.attempt_id,
                        reservation.tokens,
                        if reservation.released {
                            "usage reported"
                        } else {
                            "held / usage unresolved"
                        }
                    ));
                }
            }
            if let Some(result) = &view.result {
                text.push_str(&format!(
                    "\nResult: {}\nAccepted: {}\n",
                    result.digest, result.accepted
                ));
                for artifact in &result.artifacts {
                    text.push_str(&format!(
                        "{} · {} bytes\nSHA256: {}\n",
                        artifact.path, artifact.bytes, artifact.sha256
                    ));
                }
                for check in &result.checks {
                    text.push_str(&format!(
                        "\nCheck: {}\nExit: {:?} · truncated: {}\n{}\n",
                        check.argv.join(" "),
                        check.exit_code,
                        check.truncated,
                        check.output
                    ));
                }
            }
            for application in &view.applications {
                text.push_str(&format!(
                    "\nApplication {} · {:?}\nDigest: {}\nPaths: {}\nReceipt: {}\n",
                    application.id,
                    application.state,
                    application.digest,
                    application.paths.join(", "),
                    application
                        .receipt
                        .as_deref()
                        .unwrap_or("pending; no completion receipt")
                ));
            }
        }
        if let Some(command) = &self.confirmation {
            let value = match command {
                Command::Setup {
                    params,
                    agent,
                    instructions,
                    model,
                    limits,
                } => serde_json::to_string_pretty(
                    &serde_json::json!({"setup":params,"agent":agent,"instructions":instructions,"model":model,"limits":limits,"execution":"paused; create tasks and authorize exact scopes separately"}),
                ),
                Command::NamedView { name, action } => serde_json::to_string_pretty(
                    &serde_json::json!({"savedView":name,"operation":action}),
                ),
                Command::OwnerContact(id) => serde_json::to_string_pretty(
                    &serde_json::json!({"executiveContact":id,"purpose":"Optional direct contact for owner oversight; select any enabled agent"}),
                ),
                Command::NativeTerminal(root) => serde_json::to_string_pretty(
                    &serde_json::json!({"action":"Open actual Neovim terminal","cwd":root,"authority":"Owner shell; commands entered here have owner permissions, separately from agent sandbox/grants","checkpoint":"Close terminal windows before saving project-file checkpoints"}),
                ),
                Command::ProjectExplorer(root) => serde_json::to_string_pretty(
                    &serde_json::json!({"action":"Open official Neovim bundled project explorer","root":root,"authority":"Owner-selected files; agent grants remain unchanged","buffers":"Open a native split and retain existing dirty buffers"}),
                ),
                Command::RestoreEditor(session, recover) => serde_json::to_string_pretty(
                    &serde_json::json!({"nativeSession":session,"recoverUnsaved":recover,"writesOriginalFiles":false}),
                ),
                Command::BufferProposal(proposal) => serde_json::to_string_pretty(
                    &serde_json::json!({"nativeBuffer":proposal.pin,"acceptedResult":proposal.result_digest,"proposalSha256":proposal.artifact_sha256,"writesOriginalFiles":false,"undo":"native Neovim undo; save is separate"}),
                ),
                Command::Execution(m) => serde_json::to_string_pretty(m),
                Command::Definition(m) => serde_json::to_string_pretty(m),
                Command::Schedule(m) => serde_json::to_string_pretty(m),
                Command::Workflow(m) => serde_json::to_string_pretty(m),
                Command::Project {
                    params,
                    goal_id,
                    owner_id,
                    criteria,
                } => serde_json::to_string_pretty(
                    &serde_json::json!({ "project": params, "goalId": goal_id, "ownerId": owner_id, "criteria": criteria }),
                ),
                _ => return text,
            };
            let authorization = match command {
                Command::Execution(OrganizationExecutionMutation::Authorize {
                    scope,
                    task_id,
                    ..
                }) => Some((task_id, scope)),
                Command::Execution(OrganizationExecutionMutation::AuthorizeProfile {
                    profile,
                    task_id,
                    ..
                }) => match &profile.value {
                    OrganizationControlValue::ExecutionProfile { scope, .. } => {
                        Some((task_id, scope.as_ref()))
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some((task_id, scope)) = authorization {
                if let Some(project) = self.snapshot.records.iter().find_map(|r| match r {
                    OrganizationRecord::Task(t) if &t.id == task_id => Some(t.project_id.as_str()),
                    _ => None,
                }) {
                    text.push_str(&self.resource_preview(project, &[scope]));
                }
                text.push_str("\nStarting authorizes background model work in an isolated workspace.\nTools: exec_command, apply_patch; no agent network or owner credentials.\nFinal acceptance and Apply remain separate owner actions.\nToken ceilings permit an already-admitted request to finish.\n");
                if let Some(estimate) = self.prices.as_ref().and_then(|prices| {
                    prices.estimate(&self.provider, &scope.model, 0, scope.max_tokens)
                }) {
                    text.push_str(&estimate);
                } else {
                    text.push_str("Cost estimate unavailable: no matching owner price quote.\n");
                }
            }
            if let Command::Definition(OrganizationMutation::PutTask {
                project_id,
                instructions,
                id: None,
                ..
            }) = command
            {
                let duplicates: Vec<_> = self
                    .snapshot
                    .records
                    .iter()
                    .filter_map(|r| match r {
                        OrganizationRecord::Task(t)
                            if t.project_id == *project_id
                                && t.instructions.trim() == instructions.trim() =>
                        {
                            Some(format!("{} · {}", t.id, t.state))
                        }
                        _ => None,
                    })
                    .take(8)
                    .collect();
                if !duplicates.is_empty() {
                    text.push_str(&format!("\nExact instruction matches in loaded project tasks (owner decides whether repetition is useful):\n{}\n",duplicates.join("\n")));
                }
            }
            if matches!(
                command,
                Command::Execution(OrganizationExecutionMutation::Apply { .. })
            ) {
                // The form's payload is frozen independently of later selection.
                text.clear();
                text.push_str("\nOnly the selected whole files will be replaced when no hunk selection is present; captured exact hunks replace their selected changes. Unselected files and hunks remain unchanged.\nOne Apply per accepted result, including uncertain outcomes; inspect the receipt before reconciliation.\nRecorded checks describe the full isolated result, not the installed subset.\n");
            }
            if matches!(
                command,
                Command::Execution(
                    OrganizationExecutionMutation::Handoff { .. }
                        | OrganizationExecutionMutation::RepairResult { .. }
                )
            ) {
                text.push_str("\nQueues one fresh attempt under the original grant and remaining budgets. The old worker stays fenced. Only verified checkpoint inputs/outputs are copied. Uncertain effects, missing usage, active descendants or changed authority prevent continuation. Acceptance and Apply remain separate owner decisions.\n");
            }
            if matches!(
                command,
                Command::Execution(OrganizationExecutionMutation::ApplyReplan { .. })
            ) {
                text.push_str("\nApplies the exact pending checkpoint replacements atomically under their original grants, scopes, deadlines and remaining budgets. Accepted work stays retained. Stale receivers/checkpoints, unknown usage/effects, live runtimes or exhausted limits refuse the whole delta. No automatic acceptance or Apply.\n");
            }
            match command {
                Command::Definition(OrganizationMutation::PutNode {..}) => text.push_str("\nChanges the captured node definition. Existing grants retain their policy and may be fenced by this revision change. Retiring retains history. This does not authorize model work.\n"),
                Command::Definition(OrganizationMutation::CreateRoleVersion {..}) => text.push_str("\nCreates an immutable instruction version. Assign it with E separately. Existing captured roles remain unchanged; role names grant no permissions.\n"),
                Command::Definition(OrganizationMutation::SetMembership {..}) => text.push_str("\nChanges a secondary group membership. Primary parent and execution authority remain unchanged.\n"),
                Command::Workflow(OrganizationWorkflowMutation::Put {..}) => text.push_str("\nSaves template content only. Existing runs retain their captured template. Invalid dependencies or a changed template revision reject the entire edit; no work is authorized.\n"),
                _=>{},
            }
            if let Command::Schedule(OrganizationScheduleMutation::Create {
                scope, config, ..
            }) = command
            {
                text.push_str("\nCreates finite recurring background model work in isolated workspaces.\nMissed runs are skipped; each native action retains the chosen approval policy.\nAggregate token limits permit already-admitted usage. Acceptance and Apply remain owner-only.\n");
                if let Some(estimate) = self.prices.as_ref().and_then(|prices| {
                    prices.estimate(&self.provider, &scope.model, 0, config.max_tokens)
                }) {
                    text.push_str(&estimate);
                } else {
                    text.push_str("Cost estimate unavailable: no matching owner price quote.\n");
                }
            }
            if let Command::Workflow(OrganizationWorkflowMutation::Run { preview }) = command {
                text.push_str(&format!("\nAuthorizes {} fresh tasks in background. Fixed per-step grants sum to {} tokens / {} requests.\nPrerequisites require owner acceptance. Expired/failed/interrupted grants block work; no automatic restart or Apply.\n", preview.template.items.len(), preview.max_tokens, preview.max_model_requests));
                text.push_str(&self.resource_preview(
                    &preview.template.project_id,
                    &preview.scopes.iter().map(|s| &s.scope).collect::<Vec<_>>(),
                ));
                for item in &preview.template.items {
                    for task in self
                        .snapshot
                        .records
                        .iter()
                        .filter_map(|r| match r {
                            OrganizationRecord::Task(t)
                                if t.project_id == preview.template.project_id
                                    && t.instructions.trim() == item.instructions.trim() =>
                            {
                                Some(t)
                            }
                            _ => None,
                        })
                        .take(8)
                    {
                        text.push_str(&format!("Duplicate-work advice: step {} matches loaded task {} ({}) exactly; owner decides whether to repeat.\n",item.key,task.id,task.state));
                    }
                }
            }
            if let Command::Workflow(
                OrganizationWorkflowMutation::CreateSeries {
                    preview,
                    config,
                    trigger,
                    ..
                }
                | OrganizationWorkflowMutation::CreateSupervision {
                    preview,
                    config,
                    trigger,
                    ..
                },
            ) = command
            {
                text.push_str(&format!("\nFinite series: {} · {} steps per invocation · at most {} runs\nUTC first/end: {} / {} · interval {} seconds\nLifetime pool: {} reported tokens / {} admitted requests\nTrigger: {:?}\nMissed/overlapping intervals are skipped. Accepted trigger pins the displayed source task/grant and runs once.\nExisting step grants, checks and exact-action approvals apply. No automatic root acceptance or Apply. Already-admitted tokens may overrun a ceiling.\n", config.label, preview.template.items.len(), config.max_runs, config.first_run_at, config.ends_at, config.interval_seconds, config.max_tokens, config.max_model_requests, trigger));
            }
            if let Command::Workflow(OrganizationWorkflowMutation::CreateSupervision {
                goal,
                work_type,
                ..
            }) = command
            {
                text.push_str(&format!("\nGoal supervision policy: {} · captured goal revision {}\nPermitted work type: {}\nThe displayed template, project, model/path/delegate scopes and finite pool are the authority. Goal edits fence admissions. Policy changes require a fresh exact review; revocation preserves prior accounting. Work-type text grants no additional tools or permissions.\n", goal.objective, goal.revision, work_type));
            }
            text = format!(
                "EXACT OWNER DECISION (y confirms)\n{}\n\n{text}",
                value.unwrap_or_default()
            );
        }
        text
    }
    pub(crate) fn handle_event(&mut self, tui: &mut Tui, event: TuiEvent) -> std::io::Result<()> {
        self.update();
        if let Some(editor) = &mut self.editor {
            editor.poll();
        }
        let changed = matches!(
            &event,
            TuiEvent::Key(_) | TuiEvent::Paste(_) | TuiEvent::Mouse(_)
        );
        match event {
            TuiEvent::Key(key) => self.key(key),
            TuiEvent::Paste(value) => {
                if let Some(palette) = &mut self.palette {
                    palette.paste(&value);
                } else if self.confirmation.is_some() {
                    // A frozen confirmation is never edited by pasted input.
                } else if let Some(form) = &mut self.form {
                    form.paste(&value);
                } else if self.preferences.preset == Preset::Workspace
                    && self.memory_request.is_none()
                    && self.editor.as_ref().is_some_and(|e| e.focused)
                {
                    if let Some(editor) = &mut self.editor
                        && let Err(e) = editor.paste(&value)
                    {
                        self.message = e.to_string();
                    }
                } else if let Some(input) = &mut self.input {
                    for c in value.chars().filter(|c| !c.is_control()) {
                        if input.len() + c.len_utf8()
                            > if self.memory_request.is_some() {
                                256
                            } else {
                                4096
                            }
                        {
                            break;
                        }
                        input.push(c);
                    }
                }
            }
            TuiEvent::Draw | TuiEvent::Resize(_) | TuiEvent::Resume | TuiEvent::FocusGained => {
                tui.draw(u16::MAX, |frame| {
                    if let Some(cursor) = self.render(frame.area(), frame.buffer) {
                        frame.set_cursor_position(cursor);
                        frame.set_cursor_style(
                            if self
                                .editor
                                .as_ref()
                                .is_some_and(|e| e.mode.starts_with("insert"))
                            {
                                crossterm::cursor::SetCursorStyle::SteadyBar
                            } else {
                                crossterm::cursor::SetCursorStyle::SteadyBlock
                            },
                        );
                    }
                })?;
                if self.preferences.preset == Preset::Workspace
                    && let Some(editor) = &mut self.editor
                    && let Err(e) = editor.resize(self.editor_area.width, self.editor_area.height)
                {
                    self.message = e.to_string();
                }
                if self.preferences.preset == Preset::Workspace
                    && let Err(e) = self.refresh_follow()
                {
                    self.message = format!("Follow paused: {e}");
                    self.following = false;
                    self.follow_files = None;
                }
            }
            TuiEvent::Mouse(mouse)
                if self.diagram_active()
                    && self.form.is_none()
                    && self.palette.is_none()
                    && self.confirmation.is_none()
                    && self.memory_request.is_none()
                    && self.input.is_none()
                    && self.report_text.is_none() =>
            {
                use crossterm::event::MouseButton;
                use crossterm::event::MouseEventKind;
                match mouse.kind {
                    MouseEventKind::Down(MouseButton::Left) => {
                        let position = ratatui::layout::Position::new(mouse.column, mouse.row);
                        if let Some((_, tab)) = self
                            .graph
                            .tab_hits
                            .iter()
                            .find(|(r, _)| r.contains(position))
                        {
                            self.preferences.graph.inspector_tab = *tab;
                            self.inspect();
                            self.save();
                        } else if let Some((_, id)) = self
                            .graph
                            .toggle_hits
                            .iter()
                            .find(|(r, _)| r.contains(position))
                        {
                            let id = id.clone();
                            if !self.collapsed.remove(&id) {
                                self.collapsed.insert(id);
                            }
                            self.rebuild();
                            self.inspect();
                        } else if let Some((_, id)) =
                            self.graph.hits.iter().find(|(r, _)| r.contains(position))
                        {
                            let id = id.clone();
                            if self
                                .snapshot
                                .records
                                .iter()
                                .any(|r| record_id(r) == Some(id.as_str()))
                            {
                                self.selected = Some(id);
                                self.inspect();
                            }
                        }
                    }
                    MouseEventKind::ScrollDown => {
                        self.preferences.graph.pan.1 =
                            self.preferences.graph.pan.1.saturating_add(3)
                    }
                    MouseEventKind::ScrollUp => {
                        self.preferences.graph.pan.1 =
                            self.preferences.graph.pan.1.saturating_sub(3)
                    }
                    MouseEventKind::ScrollRight => {
                        self.preferences.graph.pan.0 =
                            self.preferences.graph.pan.0.saturating_add(6)
                    }
                    MouseEventKind::ScrollLeft => {
                        self.preferences.graph.pan.0 =
                            self.preferences.graph.pan.0.saturating_sub(6)
                    }
                    _ => {}
                }
            }
            TuiEvent::Mouse(mouse)
                if self.preferences.preset == Preset::Workspace
                    && self.memory_request.is_none() =>
            {
                if let Some(editor) = &mut self.editor
                    && let Err(e) = editor.mouse(mouse, self.editor_area)
                {
                    self.message = e.to_string();
                }
            }
            _ => {}
        }
        if changed {
            tui.frame_requester().schedule_frame();
        }
        Ok(())
    }
}

fn clean(text: &str) -> String {
    crate::history_cell::sanitize_user_text(Cow::Borrowed(text)).into_owned()
}

fn record_id(record: &OrganizationRecord) -> Option<&str> {
    match record {
        OrganizationRecord::Node(n) => Some(&n.id),
        OrganizationRecord::Goal(g) => Some(&g.id),
        OrganizationRecord::Project(p) => Some(&p.id),
        OrganizationRecord::Task(t) => Some(&t.id),
        OrganizationRecord::Preset(p) => Some(&p.id),
        OrganizationRecord::Control(c) => Some(&c.id),
        _ => None,
    }
}

pub(crate) fn saved_preset(home: &Path) -> Option<Preset> {
    let path = home.join("organization-layouts.json");
    if !path.exists() {
        return None;
    }
    Preferences::load(&path)
        .ok()
        .map(|p| p.preset)
        .filter(|p| *p != Preset::Console)
}

pub(crate) fn select_console(home: &Path) -> std::io::Result<()> {
    let path = home.join("organization-layouts.json");
    let mut preferences = Preferences::load(&path)?;
    preferences.preset = Preset::Console;
    preferences.save(&path)
}

#[cfg(test)]
#[path = "organization_tests.rs"]
mod tests;

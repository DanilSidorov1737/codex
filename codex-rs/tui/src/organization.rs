//! Owner organization layouts. All authority stays in the existing app-server.
//! One bounded loader serves every preset; sleeping definitions create no model sessions.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use codex_app_server_client::AppServerRequestHandle;
use codex_app_server_protocol::*;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::organization_editor::Editor;
use crate::organization_preview::Preview;
use crate::tui::{FrameRequester, Tui, TuiEvent};

#[path = "organization_form.rs"]
mod form;
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct PaneSettings {
    inspector: bool,
    width: u16,
    inspector_focus: bool,
    vim_keys: bool,
}
impl Default for PaneSettings {
    fn default() -> Self {
        Self {
            inspector: true,
            width: 65,
            inspector_focus: false,
            vim_keys: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Preferences {
    preset: Preset,
    panes: [PaneSettings; 5],
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            preset: Preset::Operations,
            panes: std::array::from_fn(|_| PaneSettings::default()),
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
        for pane in &mut prefs.panes {
            pane.width = pane.width.clamp(25, 75);
        }
        Ok(prefs)
    }
    fn save(&self, path: &Path) -> std::io::Result<()> {
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
        serde_json::to_writer_pretty(&mut file, self)?;
        file.write_all(b"\n")?;
        file.as_file().sync_all()?;
        file.persist(path).map_err(|e| e.error)?;
        std::fs::File::open(parent)?.sync_all()
    }
}

#[derive(Default)]
struct Snapshot {
    records: Vec<OrganizationRecord>,
    schedules: Vec<OrganizationScheduleView>,
    workflows: Vec<OrganizationWorkflowView>,
    report: Option<OrganizationReport>,
}
enum Command {
    Refresh,
    Summary(String, reporting::Focus),
    Inventory(String),
    Inspect(String),
    MemorySearch(OrganizationMemorySearchParams),
    Definition(OrganizationMutation),
    Execution(OrganizationExecutionMutation),
    Schedule(OrganizationScheduleMutation),
    Workflow(OrganizationWorkflowMutation),
    WorkflowPreview(OrganizationWorkflowPreviewRequest),
    Project {
        params: ProjectCreateParams,
        goal_id: Option<String>,
        owner_id: Option<String>,
        criteria: Vec<String>,
    },
}
enum Update {
    Snapshot(Snapshot),
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

async fn load(handle: &AppServerRequestHandle) -> anyhow::Result<Snapshot> {
    let mut records = Vec::new();
    for kind in [
        OrganizationRecordKind::Nodes,
        OrganizationRecordKind::Roles,
        OrganizationRecordKind::Goals,
        OrganizationRecordKind::Projects,
        OrganizationRecordKind::Tasks,
        OrganizationRecordKind::Memberships,
    ] {
        let mut cursor = None;
        for page in 0..10 {
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
                page < 9 && response.next_cursor != cursor,
                "layout record limit reached (2,000 per kind); use paginated CLI inspection"
            );
            cursor = response.next_cursor;
        }
    }
    let mut schedules = Vec::new();
    let mut cursor = None;
    for page in 0..10 {
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
            page < 9 && response.next_cursor != cursor,
            "layout schedule limit reached (2,000); use paginated CLI inspection"
        );
        cursor = response.next_cursor;
    }
    let mut workflows = Vec::new();
    for kind in [
        OrganizationWorkflowKind::Templates,
        OrganizationWorkflowKind::Runs,
    ] {
        let mut cursor = None;
        for page in 0..10 {
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
                page < 9 && response.next_cursor != cursor,
                "layout workflow limit reached (2,000 per kind); use paginated CLI inspection"
            );
            cursor = response.next_cursor;
        }
    }
    Ok(Snapshot {
        records,
        schedules,
        workflows,
        report: Some(reporting::load(handle, None).await?),
    })
}

async fn worker(
    handle: AppServerRequestHandle,
    frame: FrameRequester,
    mut commands: mpsc::Receiver<Command>,
    updates: mpsc::Sender<Update>,
) {
    let mut selected: Option<String> = None;
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let command = tokio::select! { command = commands.recv() => { let Some(command) = command else { break }; command }, _ = interval.tick() => Command::Refresh };
        let memory_request = match &command {
            Command::MemorySearch(params) => Some(params.clone()),
            _ => None,
        };
        let mut apply_rejected = false;
        let operation = async {
            match command {
                Command::Summary(id, focus) => {
                    let report = reporting::load(&handle, focus.project_id())
                        .await
                        .map(Box::new)
                        .map_err(|error| error.to_string());
                    updates.send(Update::Report(id, report, focus)).await?;
                }
                Command::Inventory(id) => {
                    let result = handle
                        .request_typed::<OrganizationInventoryResponse>(
                            ClientRequest::OrganizationInventory {
                                request_id: request_id(),
                                params: OrganizationInventoryParams {},
                            },
                        )
                        .await
                        .map(|response| reporting::inventory_text(&response.inventory))
                        .map_err(|error| error.to_string());
                    updates.send(Update::ReportText(id, result)).await?;
                }
                Command::Refresh => {
                    updates.send(Update::Snapshot(load(&handle).await?)).await?;
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

/// Iterative traversal keeps deep user-defined hierarchies off the call stack.
fn tree(nodes: &[OrganizationNode], collapsed: &BTreeSet<String>, search: &str) -> Vec<Row> {
    let mut children: BTreeMap<Option<&str>, Vec<&OrganizationNode>> = BTreeMap::new();
    let ids: BTreeSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
    for node in nodes {
        let parent = node.parent_id.as_deref().filter(|p| ids.contains(p));
        children.entry(parent).or_default().push(node);
    }
    for siblings in children.values_mut() {
        siblings.sort_by_key(|n| (&n.label, &n.id));
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
        if !visited.insert(node.id.as_str()) {
            continue;
        }
        let branch = children.contains_key(&Some(node.id.as_str()));
        if search.is_empty()
            || node.label.to_lowercase().contains(search)
            || node.id.contains(search)
        {
            rows.push(Row {
                id: node.id.clone(),
                label: format!(
                    "{}{} {} [{}{}]",
                    "  ".repeat(depth.min(12)),
                    if branch {
                        if collapsed.contains(&node.id) {
                            "▸"
                        } else {
                            "▾"
                        }
                    } else {
                        "·"
                    },
                    node.label,
                    if node.kind == OrganizationNodeKind::Group {
                        "group"
                    } else {
                        "agent"
                    },
                    if node.enabled {
                        ""
                    } else {
                        "; retired / disabled"
                    }
                ),
            });
        }
        if (!collapsed.contains(&node.id) || !search.is_empty())
            && let Some(list) = children.get(&Some(node.id.as_str()))
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
    schedule_view: bool,
    workflow_view: bool,
    rows: Vec<Row>,
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
    editor_area: Rect,
    editor_file: usize,
    following: bool,
    applying: bool,
    provider: String,
    model: String,
    form: Option<form::Form>,
    prices: Option<crate::organization_pricing::Prices>,
    frame: FrameRequester,
    pub(crate) is_done: bool,
}

impl Drop for Dashboard {
    fn drop(&mut self) {
        self.worker.abort();
    }
}

impl Dashboard {
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
            schedule_view: false,
            workflow_view: false,
            rows: Vec::new(),
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
            editor_area: Rect::default(),
            editor_file: 0,
            following: false,
            applying: false,
            provider: provider.into(),
            model: model.into(),
            form: None,
            prices: crate::organization_pricing::Prices::load(home).ok(),
            frame,
            is_done: false,
        }
    }
    fn pane(&mut self) -> &mut PaneSettings {
        &mut self.preferences.panes[self.preferences.preset.index()]
    }
    fn send(&mut self, command: Command) {
        if let Err(error) = self.commands.try_send(command) {
            self.message = "Owner request queue busy; retry after refresh.".into();
            let command = error.into_inner();
            if matches!(&command, Command::Summary(..) | Command::Inventory(..)) {
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
        self.schedule_view = false;
        self.workflow_view = false;
        self.search.clear();
        self.selected = None;
        self.rebuild();
        self.inspect();
        self.save();
    }
    fn rebuild(&mut self) {
        let search = self.search.to_lowercase();
        self.rows = if self.preferences.preset == Preset::Review {
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
            && !self.schedule_view
            && !self.workflow_view
        {
            let nodes: Vec<_> = self
                .snapshot
                .records
                .iter()
                .filter_map(|r| {
                    if let OrganizationRecord::Node(n) = r {
                        Some(n.clone())
                    } else {
                        None
                    }
                })
                .collect();
            let mut rows = tree(&nodes, &self.collapsed, &search);
            let mut states: BTreeMap<&str, BTreeMap<&str, usize>> = BTreeMap::new();
            for record in &self.snapshot.records {
                if let OrganizationRecord::Task(task) = record {
                    *states
                        .entry(&task.owner_id)
                        .or_default()
                        .entry(&task.state)
                        .or_default() += 1;
                }
            }
            for row in &mut rows {
                if let Some(counts) = states.get(row.id.as_str()) {
                    row.label.push_str(&format!(
                        " · {}",
                        counts
                            .iter()
                            .map(|(state, n)| format!("{n} {state}"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            }
            rows
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
            if self.schedule_view || self.workflow_view {
                rows.clear();
            }
            if self.schedule_view
                || (self.preferences.preset == Preset::Projects && !self.workflow_view)
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
            if self.workflow_view
                || (self.preferences.preset == Preset::Projects && !self.schedule_view)
            {
                rows.extend(self.snapshot.workflows.iter().filter_map(|view| {
                    let (id, label) = match view {
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
            if self.workflow_view {
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
            _ => false,
        })
    }
    fn selected_schedule(&self) -> Option<&OrganizationScheduleView> {
        let id = self.selected.as_deref()?.strip_prefix("schedule:")?;
        self.snapshot.schedules.iter().find(|v| v.schedule.id == id)
    }
    fn editor_open(&mut self, key: KeyCode) {
        if self.applying {
            self.message = "Application outcome pending; editor inputs remain frozen".into();
            return;
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
            confined(
                Path::new(
                    project
                        .roots
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("project root missing"))?,
                ),
                &base,
            )?;
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
        if let Err(e) = result {
            self.message = e.to_string();
        }
    }
    fn decision(&mut self, key: char) {
        let command = if let Some(view) = self.selected_schedule() {
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
            'B' => form::Kind::Group,
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
            'H' => {
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
                form::Kind::Handoff {
                    task: view.task.clone(),
                    checkpoint: checkpoint.clone(),
                    receivers,
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
                if result.matched_count == 0 { text.push_str("No matching notes.\n"); }
                for note in &result.notes {
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
                    KeyCode::Char('Q') => {
                        self.selection_task();
                        return;
                    }
                    KeyCode::Char('e' | 'o' | 't' | 'd') => {
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
                    Ok(Command::WorkflowPreview(request)) => {
                        self.send(Command::WorkflowPreview(request));
                        self.message = "Loading exact workflow preview; no work authorized".into();
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
                if self.preferences.preset == Preset::Operations
                    && !self.schedule_view
                    && !self.workflow_view =>
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
            KeyCode::Char('w') => {
                self.workflow_view = !self.workflow_view;
                self.schedule_view = false;
                self.selected = None;
                self.search.clear();
                self.rebuild();
                self.inspect();
                self.message =
                    "Workflows · W saves selected task chain · s previews selected template".into();
            }
            KeyCode::Char('S') => {
                self.schedule_view = !self.schedule_view;
                self.workflow_view = false;
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
            KeyCode::Char('C') => {
                self.report_text = Some("Loading configured capabilities…".into());
                self.scroll = 0;
                let id = uuid::Uuid::new_v4().to_string();
                self.report_request = Some(id.clone());
                self.send(Command::Inventory(id));
            }
            KeyCode::Char('m') => self.open_memory(false),
            KeyCode::Char('M') => self.open_memory(true),
            KeyCode::Char(
                c @ ('U' | 'G' | 'N' | 'T' | 's' | 'A' | 'R' | 'W' | 'H' | 'B' | 'E' | 'L' | 'I'),
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
            KeyCode::Char(c @ ('a' | 'r' | 'v' | 'c' | 'p' | 'x' | 'u')) => self.decision(c),
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
                    self.snapshot = snapshot;
                    self.rebuild();
                    self.detail_text = clean(&self.details());
                    if self.confirmation.is_none()
                        && self.memory_request.is_none()
                        && !self.message_is_error
                        && !self.applying
                    {
                        self.message =
                            "Owner state loaded; definitions do not imply running agents.".into();
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
                    if self.form.as_ref().is_some_and(|form| matches!(form.command(), Ok(Command::WorkflowPreview(request)) if request.id == preview.id && request.template_id == preview.template.id && request.scopes == preview.scopes)) {
                        self.confirmation = Some(Command::Workflow(OrganizationWorkflowMutation::Run { preview }));
                        self.scroll = 0;
                        self.pane().inspector = true;
                        self.confirmation_text = clean(&self.details());
                        self.message = "Review exact workflow tasks/scopes · y start · Esc back".into();
                    }
                }
                Update::WorkflowSaved(view) => {
                    let id = workflow_id(&view);
                    self.snapshot.workflows.retain(|v| workflow_id(v) != id);
                    self.snapshot.workflows.push(*view);
                    self.selected = Some(id);
                    self.workflow_view = true;
                    self.schedule_view = false;
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
                        self.schedule_view = true;
                        self.workflow_view = false;
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
    fn details(&self) -> String {
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
                }
                OrganizationRecord::Goal(g) => {
                    text = format!(
                        "Goal\n{}\n{}\n\nCriteria:\n{}\n",
                        g.id,
                        g.objective,
                        g.criteria.join("\n")
                    )
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
                    )
                }
                _ => {}
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
                            "Model price unavailable: no matching trusted owner quote.\n".into()
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
            if let Command::Execution(OrganizationExecutionMutation::Authorize { scope, .. }) =
                command
            {
                text.push_str("\nStarting authorizes background model work in an isolated workspace.\nTools: exec_command, apply_patch; no agent network or owner credentials.\nFinal acceptance and Apply remain separate owner actions.\nToken ceilings permit an already-admitted request to finish.\n");
                if let Some(estimate) = self.prices.as_ref().and_then(|prices| {
                    prices.estimate(&self.provider, &scope.model, 0, scope.max_tokens)
                }) {
                    text.push_str(&estimate);
                } else {
                    text.push_str("Cost estimate unavailable: no matching owner price quote.\n");
                }
            }
            if matches!(
                command,
                Command::Execution(OrganizationExecutionMutation::Apply { .. })
            ) {
                // The form's payload is frozen independently of later selection.
                text.clear();
                text.push_str("\nOnly the selected whole files will be replaced; unselected files remain unchanged.\nOne Apply per accepted result, including uncertain outcomes; inspect the receipt before reconciliation.\nRecorded checks describe the full isolated result, not the installed subset.\n");
            }
            if matches!(
                command,
                Command::Execution(OrganizationExecutionMutation::Handoff { .. })
            ) {
                text.push_str("\nQueues one fresh attempt under the original grant and remaining budgets. The old worker stays fenced. Only verified checkpoint inputs/outputs are copied. Uncertain effects, missing usage, active descendants or changed authority prevent continuation. Acceptance and Apply remain separate owner decisions.\n");
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
                text.push_str(&format!("\nAuthorizes {} fresh tasks in background. Fixed per-step grants sum to {} tokens / {} requests.\nPrerequisites require owner acceptance. Expired/failed/interrupted grants block work; no automatic restart or Apply.\nToken ceilings allow already-admitted usage; cost estimate unavailable.\n", preview.template.items.len(), preview.max_tokens, preview.max_model_requests));
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
        let changed = matches!(&event, TuiEvent::Key(_) | TuiEvent::Paste(_));
        match event {
            TuiEvent::Key(key) => self.key(key),
            TuiEvent::Paste(value) => {
                if self.confirmation.is_some() {
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
                {
                    if let Err(e) = editor.resize(self.editor_area.width, self.editor_area.height) {
                        self.message = e.to_string();
                    }
                    if self.following && !editor.focused {
                        let refresh = (|| -> anyhow::Result<()> {
                            if !editor.dirty()?
                                && let Some(preview) = &mut self.preview
                                && preview.refresh()?
                            {
                                editor.reload_follow()?;
                            }
                            Ok(())
                        })();
                        if let Err(e) = refresh {
                            self.message = e.to_string();
                        }
                    }
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

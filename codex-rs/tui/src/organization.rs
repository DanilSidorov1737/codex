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
#[path = "organization_view.rs"]
mod view;

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
}
enum Command {
    Refresh,
    Inspect(String),
    Definition(OrganizationMutation),
    Execution(OrganizationExecutionMutation),
    Project {
        params: ProjectCreateParams,
        goal_id: Option<String>,
        owner_id: Option<String>,
        criteria: Vec<String>,
    },
}
enum Update {
    Snapshot(Snapshot),
    Detail(Box<OrganizationExecutionView>),
    Error(String),
    Changed,
    Saved(OrganizationRecord),
}

fn request_id() -> RequestId {
    // Background requests share the startup transport and can outlive a closed dashboard.
    RequestId::String(format!("organization-{}", uuid::Uuid::new_v4()))
}

async fn load(handle: &AppServerRequestHandle) -> anyhow::Result<Snapshot> {
    let mut records = Vec::new();
    for kind in [
        OrganizationRecordKind::Nodes,
        OrganizationRecordKind::Roles,
        OrganizationRecordKind::Goals,
        OrganizationRecordKind::Projects,
        OrganizationRecordKind::Tasks,
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
    Ok(Snapshot { records })
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
        let operation = async {
            match command {
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
                    let response: OrganizationExecutionMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationExecutionMutate {
                            request_id: request_id(),
                            params: OrganizationExecutionMutateParams { mutation },
                        })
                        .await?;
                    updates
                        .send(Update::Detail(Box::new(response.view)))
                        .await?;
                    updates.send(Update::Changed).await?;
                }
            }
            anyhow::Ok(())
        };
        match tokio::time::timeout(Duration::from_secs(15), operation).await {
            Ok(Ok(())) => {}, Ok(Err(e)) => { if updates.send(Update::Error(e.to_string())).await.is_err() { break; } },
            Err(_) => { if updates.send(Update::Error("Owner API timed out; consequential decisions have an unknown outcome. Refresh before retrying.".into())).await.is_err() { break; } },
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
                    if node.enabled { "" } else { "; paused" }
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
    rows: Vec<Row>,
    selected: Option<String>,
    collapsed: BTreeSet<String>,
    detail: Option<Box<OrganizationExecutionView>>,
    scroll: u16,
    detail_text: String,
    review_diff: Option<(String, String, String)>,
    review_file: usize,
    search: String,
    input: Option<String>,
    changes: bool,
    change_target: Option<OrganizationTask>,
    confirmation: Option<Command>,
    confirmation_text: String,
    message: String,
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
            rows: Vec::new(),
            selected: None,
            collapsed: BTreeSet::new(),
            detail: None,
            scroll: 0,
            detail_text: String::new(),
            review_diff: None,
            review_file: 0,
            search: String::new(),
            input: None,
            changes: false,
            change_target: None,
            confirmation: None,
            confirmation_text: String::new(),
            message,
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
        if self.commands.try_send(command).is_err() {
            self.message = "Owner request queue busy; retry after refresh.".into();
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
        self.search.clear();
        self.selected = None;
        self.rebuild();
        self.inspect();
        self.save();
    }
    fn rebuild(&mut self) {
        let search = self.search.to_lowercase();
        self.rows = if self.preferences.preset == Preset::Operations {
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
            self.snapshot
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
                .collect()
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
        let command = if let Some(view) = self
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
    }
    fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if self.preferences.preset == Preset::Workspace {
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
        if let Some(form) = &mut self.form {
            match key.code {
                KeyCode::Esc => self.form = None,
                KeyCode::F(6) => match form.command() {
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
                KeyCode::Esc => self.input = None,
                KeyCode::Enter => {
                    let value = self.input.take().unwrap_or_default();
                    if self.changes {
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
                KeyCode::Char(c) if input.len() < 4096 && !c.is_control() => input.push(c),
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
                if self.preferences.preset == Preset::Operations =>
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
            KeyCode::Char(c @ ('U' | 'G' | 'N' | 'T' | 's' | 'A')) => self.open_form(c),
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
            KeyCode::Char(c @ ('a' | 'r' | 'v' | 'c' | 'p' | 'x')) => self.decision(c),
            _ => {}
        }
    }
    fn update(&mut self) {
        while let Ok(update) = self.updates.try_recv() {
            match update {
                Update::Snapshot(snapshot) => {
                    let first = self.snapshot.records.is_empty();
                    let selected = self.selected.clone();
                    self.snapshot = snapshot;
                    self.rebuild();
                    self.detail_text = clean(&self.details());
                    if self.confirmation.is_none() {
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
                Update::Error(e) => self.message = e,
                Update::Changed => {
                    self.applying = false;
                    self.message = "Owner decision recorded.".into();
                    if let Some(editor) = &mut self.editor
                        && let Err(e) = editor.thaw()
                    {
                        self.message = e.to_string();
                    }
                    if let Some(editor) = &mut self.editor
                        && let Err(e) = editor.reload_follow()
                    {
                        self.message = e.to_string();
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
            }
        }
    }
    fn details(&self) -> String {
        let mut text = String::new();
        if let Some(record) = self.record() {
            match record {
                OrganizationRecord::Node(n) => {
                    text = format!(
                        "{}\n{} · {}\nModel: {}\n\n",
                        n.label,
                        n.id,
                        if n.enabled { "Enabled" } else { "Paused" },
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
                    && self.editor.as_ref().is_some_and(|e| e.focused)
                {
                    if let Some(editor) = &mut self.editor
                        && let Err(e) = editor.paste(&value)
                    {
                        self.message = e.to_string();
                    }
                } else if let Some(input) = &mut self.input {
                    for c in value.chars().filter(|c| !c.is_control()) {
                        if input.len() + c.len_utf8() > 4096 {
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
            TuiEvent::Mouse(mouse) if self.preferences.preset == Preset::Workspace => {
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

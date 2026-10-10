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

use crate::tui::{FrameRequester, Tui, TuiEvent};

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
            width: 45,
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
}
enum Update {
    Snapshot(Snapshot),
    Detail(Box<OrganizationExecutionView>),
    Error(String),
    Changed,
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
                    request_id: RequestId::Integer(0),
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
                                request_id: RequestId::Integer(0),
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
                            request_id: RequestId::Integer(0),
                            params: OrganizationExecutionReadParams { task_id },
                        })
                        .await?;
                    updates
                        .send(Update::Detail(Box::new(response.view)))
                        .await?;
                }
                Command::Definition(mutation) => {
                    let _: OrganizationMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationMutate {
                            request_id: RequestId::Integer(0),
                            params: OrganizationMutateParams { mutation },
                        })
                        .await?;
                    updates.send(Update::Changed).await?;
                }
                Command::Execution(mutation) => {
                    let response: OrganizationExecutionMutateResponse = handle
                        .request_typed(ClientRequest::OrganizationExecutionMutate {
                            request_id: RequestId::Integer(0),
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
        let worker = tokio::spawn(worker(handle, frame, command_rx, update_tx));
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
    fn key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }
        if self.confirmation.is_some() {
            match key.code {
                KeyCode::Char('y') => {
                    if let Some(c) = self.confirmation.take() {
                        self.send(c);
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
            KeyCode::Char(c @ ('a' | 'r' | 'v' | 'c' | 'p' | 'x')) => self.decision(c),
            _ => {}
        }
    }
    fn update(&mut self) {
        while let Ok(update) = self.updates.try_recv() {
            match update {
                Update::Snapshot(snapshot) => {
                    let first = self.snapshot.records.is_empty();
                    self.snapshot = snapshot;
                    self.rebuild();
                    self.detail_text = clean(&self.details());
                    self.message =
                        "Owner state loaded; definitions do not imply running agents.".into();
                    if self.confirmation.is_some() {
                        self.message =
                            "Confirm captured owner decision: y / Esc · PageUp/PageDown to inspect"
                                .into();
                    }
                    if first {
                        self.inspect();
                    }
                }
                Update::Detail(view) => {
                    if Some(&view.task.id) == self.selected.as_ref() {
                        self.detail = Some(view);
                        self.detail_text = clean(&self.details());
                    }
                }
                Update::Error(e) => self.message = e,
                Update::Changed => {
                    self.message = "Owner decision recorded.".into();
                    self.send(Command::Refresh);
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
        }
        if let Some(command) = &self.confirmation {
            let value = match command {
                Command::Execution(m) => serde_json::to_string_pretty(m),
                Command::Definition(m) => serde_json::to_string_pretty(m),
                _ => return text,
            };
            text = format!(
                "EXACT OWNER DECISION (y confirms)\n{}\n\n{text}",
                value.unwrap_or_default()
            );
        }
        text
    }
    fn render(&self, area: Rect, buffer: &mut Buffer) {
        Clear.render(area, buffer);
        let regions = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(1),
                Constraint::Length(3),
            ])
            .split(area);
        Paragraph::new("F1 Console · F2 Workspace · F3 Projects · F4 Operations · F5 Review")
            .style(Style::default().fg(Color::Cyan))
            .render(regions[0], buffer);
        let pane = &self.preferences.panes[self.preferences.preset.index()];
        let areas = Layout::default()
            .direction(if regions[1].width < 80 {
                Direction::Vertical
            } else {
                Direction::Horizontal
            })
            .constraints(if pane.inspector {
                vec![
                    Constraint::Percentage(pane.width),
                    Constraint::Percentage(100 - pane.width),
                ]
            } else {
                vec![Constraint::Percentage(100)]
            })
            .split(regions[1]);
        let selected = self
            .rows
            .iter()
            .position(|r| Some(&r.id) == self.selected.as_ref())
            .unwrap_or(0);
        let height = areas[0].height.saturating_sub(2) as usize;
        let start = selected.saturating_sub(height.saturating_sub(1));
        let lines: Vec<_> = self
            .rows
            .iter()
            .enumerate()
            .skip(start)
            .take(height)
            .map(|(i, r)| {
                Line::styled(
                    clean(&format!(
                        "{} {}",
                        if i == selected { "›" } else { " " },
                        r.label
                    )),
                    if i == selected {
                        Style::default().fg(Color::Yellow)
                    } else {
                        Style::default()
                    },
                )
            })
            .collect();
        Paragraph::new(lines)
            .block(Block::default().borders(Borders::ALL).title(format!(
                "{} · {} · /{}",
                self.preferences.preset.title(),
                self.rows.len(),
                clean(&self.search)
            )))
            .render(areas[0], buffer);
        if pane.inspector {
            Paragraph::new(if self.confirmation.is_some() {
                self.confirmation_text.clone()
            } else {
                self.detail_text.clone()
            })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(if pane.inspector_focus {
                        "Inspector [focused]"
                    } else {
                        "Inspector"
                    }),
            )
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0))
            .render(areas[1], buffer);
        }
        let prompt = self
            .input
            .as_ref()
            .map(|s| {
                format!(
                    "{}{}",
                    if self.changes {
                        "Changes: "
                    } else {
                        "Search: "
                    },
                    s
                )
            })
            .unwrap_or_else(|| self.message.clone());
        Paragraph::new(format!("{}\n↑↓ navigate · Tab focus · i pane · [ ] size · K keys · / search · f refresh\na approve · r reject · v accept · c changes · p pause/toggle · x cancel · q Console", clean(&prompt))).render(regions[2], buffer);
    }
    pub(crate) fn handle_event(&mut self, tui: &mut Tui, event: TuiEvent) -> std::io::Result<()> {
        self.update();
        let changed = matches!(&event, TuiEvent::Key(_) | TuiEvent::Paste(_));
        match event {
            TuiEvent::Key(key) => self.key(key),
            TuiEvent::Paste(value) => {
                if let Some(input) = &mut self.input {
                    for c in value.chars().filter(|c| !c.is_control()) {
                        if input.len() + c.len_utf8() > 4096 {
                            break;
                        }
                        input.push(c);
                    }
                }
            }
            TuiEvent::Draw | TuiEvent::Resize(_) | TuiEvent::Resume | TuiEvent::FocusGained => {
                tui.draw(u16::MAX, |frame| self.render(frame.area(), frame.buffer))?;
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

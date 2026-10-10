//! Fixed owner forms reuse the existing textarea and typed administration contracts.
use super::*;
use crate::bottom_pane::{TextArea, TextAreaState};
use crossterm::event::KeyModifiers;
use ratatui::layout::Margin;

pub(super) enum Kind {
    Agent,
    Goal,
    Project,
    Task,
    Authorize(OrganizationTask),
    Apply {
        task: OrganizationTask,
        result: OrganizationTaskResult,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(form: &mut Form, index: usize, value: &str) {
        form.fields[index].input.set_text_clearing_elements(value);
    }

    fn task() -> OrganizationTask {
        OrganizationTask {
            id: "original-task".into(),
            project_id: "project".into(),
            goal_id: None,
            owner_id: "agent".into(),
            instructions: "Repair addition".into(),
            criteria: vec!["Oracle passes".into()],
            prerequisites: vec![],
            state: "proposed".into(),
            revision: 7,
        }
    }

    #[test]
    fn authorization_captures_task_and_rejects_invalid_paths_budgets_and_checks() {
        let mut form = Form::new(Kind::Authorize(task()), &[], None, "native-model");
        set(&mut form, 0, "math.py\noracle.py");
        set(&mut form, 1, "oracle.py");
        set(&mut form, 2, "math.py");
        set(&mut form, 3, "[[\"python3\",\"oracle.py\"]]");
        let Command::Execution(OrganizationExecutionMutation::Authorize {
            task_id,
            expected_revision,
            scope,
        }) = form.command().unwrap()
        else {
            panic!("missing scope")
        };
        assert_eq!(task_id, "original-task");
        assert_eq!(expected_revision, 7);
        assert_eq!(scope.checks, vec![vec!["python3", "oracle.py"]]);
        assert!(scope.require_tool_approval);
        assert_eq!(scope.max_children, 0);
        for invalid in ["../math.py", "/math.py", "math.py\nmath.py"] {
            set(&mut form, 2, invalid);
            assert!(form.command().is_err());
        }
        set(&mut form, 2, "math.py");
        set(&mut form, 5, "0");
        assert!(form.command().is_err());
        set(&mut form, 5, "10000");
        set(&mut form, 3, "python3 oracle.py");
        assert!(form.command().is_err());
        set(&mut form, 3, "[]");
        form.fields[12].choice = 1;
        assert!(form.command().is_err());
    }

    #[test]
    fn project_reuses_one_identity_key_and_requires_an_existing_directory() {
        let root = tempfile::tempdir().unwrap();
        let mut form = Form::new(Kind::Project, &[], None, "model");
        assert!(form.command().is_err());
        set(&mut form, 0, "Local project");
        set(&mut form, 1, root.path().to_str().unwrap());
        set(&mut form, 4, "Meaningful check passes\nOwner accepts");
        let Command::Project {
            params: first,
            criteria,
            ..
        } = form.command().unwrap()
        else {
            panic!("missing project")
        };
        let Command::Project { params: second, .. } = form.command().unwrap() else {
            panic!("missing project")
        };
        assert_eq!(first.idempotency_key, second.idempotency_key);
        assert_eq!(criteria.len(), 2);
        assert_eq!(first.roots[0].path.as_path(), root.path());
        set(&mut form, 1, root.path().join("missing").to_str().unwrap());
        assert!(form.command().is_err());
    }

    #[test]
    fn multiline_unicode_draft_and_choice_ids_survive_navigation() {
        let records = vec![OrganizationRecord::Node(OrganizationNode {
            id: "stable-agent".into(),
            parent_id: None,
            kind: OrganizationNodeKind::Agent,
            label: "Arbitrary researcher".into(),
            role_version_id: None,
            model: None,
            enabled: true,
            revision: 1,
        })];
        let mut form = Form::new(Kind::Task, &records, None, "model");
        set(&mut form, 0, "project");
        form.focus = 3;
        form.paste("Read facts — café\nCite sources");
        form.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        form.paste("Contains exact facts");
        let Command::Definition(OrganizationMutation::PutTask {
            owner_id,
            instructions,
            ..
        }) = form.command().unwrap()
        else {
            panic!("missing task")
        };
        assert_eq!(owner_id, "stable-agent");
        assert_eq!(instructions, "Read facts — café\nCite sources");
        for size in [(1, 1), (50, 20), (160, 44)] {
            let area = Rect::new(0, 0, size.0, size.1);
            form.render(area, &mut Buffer::empty(area));
        }
    }

    #[test]
    fn apply_selection_preserves_exact_paths_and_requires_a_nonempty_set() {
        let result = OrganizationTaskResult {
            attempt_id: "attempt".into(),
            artifacts: ["report.txt", " notes café.md "]
                .into_iter()
                .map(|p| OrganizationArtifactEvidence {
                    path: p.into(),
                    sha256: "a".repeat(64),
                    bytes: 1,
                })
                .collect(),
            artifact_bases: vec![],
            checks: vec![],
            digest: "exact-result".into(),
            accepted: true,
        };
        let mut form = Form::new(
            Kind::Apply {
                task: task(),
                result,
            },
            &[],
            None,
            "unused",
        );
        form.key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        let Command::Execution(OrganizationExecutionMutation::Apply {
            task_id,
            expected_revision,
            digest,
            paths,
        }) = form.command().unwrap()
        else {
            panic!("missing apply")
        };
        assert_eq!(task_id, "original-task");
        assert_eq!(expected_revision, 7);
        assert_eq!(digest, "exact-result");
        assert_eq!(paths, Some(vec![" notes café.md ".into()]));
        form.key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        form.key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(form.command().is_err());
        for size in [(1, 1), (50, 20), (160, 48)] {
            let area = Rect::new(0, 0, size.0, size.1);
            form.render(area, &mut Buffer::empty(area));
        }
    }
}

struct Field {
    label: String,
    hint: &'static str,
    input: TextArea,
    choices: Vec<(String, String)>,
    choice: usize,
}

impl Field {
    fn text(label: impl Into<String>, hint: &'static str, value: &str) -> Self {
        let mut input = TextArea::new();
        input.set_text_clearing_elements(value);
        Self {
            label: clean(&label.into()),
            hint,
            input,
            choices: vec![],
            choice: 0,
        }
    }
    fn select(
        label: impl Into<String>,
        choices: Vec<(String, String)>,
        selected: Option<&str>,
    ) -> Self {
        let choice = choices
            .iter()
            .position(|(id, _)| Some(id.as_str()) == selected)
            .unwrap_or(0);
        Self {
            choices,
            choice,
            ..Self::text(label, "Left / Right choose", "")
        }
    }
    fn value(&self) -> &str {
        self.choices
            .get(self.choice)
            .map(|(id, _)| id.as_str())
            .unwrap_or(self.input.text().trim())
    }
}

pub(super) struct Form {
    kind: Kind,
    fields: Vec<Field>,
    focus: usize,
    project_key: String,
    pub(super) error: String,
}

impl Form {
    pub(super) fn new(
        kind: Kind,
        records: &[OrganizationRecord],
        selected: Option<&OrganizationRecord>,
        model: &str,
    ) -> Self {
        let optional = |mut choices: Vec<(String, String)>| {
            choices.insert(0, (String::new(), "None".into()));
            choices
        };
        let nodes: Vec<_> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Node(n) => Some((n.id.clone(), n.label.clone())),
                _ => None,
            })
            .collect();
        let agents: Vec<_> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Node(n)
                    if n.kind == OrganizationNodeKind::Agent && n.enabled =>
                {
                    Some((n.id.clone(), n.label.clone()))
                }
                _ => None,
            })
            .collect();
        let goals: Vec<_> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Goal(g) => Some((g.id.clone(), g.objective.clone())),
                _ => None,
            })
            .collect();
        let project = selected.and_then(|r| match r {
            OrganizationRecord::Project(p) => Some(p),
            OrganizationRecord::Task(t) => records.iter().find_map(|r| match r {
                OrganizationRecord::Project(p) if p.id == t.project_id => Some(p),
                _ => None,
            }),
            _ => None,
        });
        let goal_id = selected
            .and_then(|r| match r {
                OrganizationRecord::Goal(g) => Some(g.id.as_str()),
                _ => None,
            })
            .or_else(|| project.and_then(|p| p.goal_id.as_deref()));
        let owner = project.and_then(|p| p.owner_id.as_deref());
        let fields = match &kind {
            Kind::Apply { result, .. } => result
                .artifacts
                .iter()
                .map(|artifact| {
                    Field::select(
                        artifact.path.clone(),
                        vec![
                            (artifact.path.clone(), "[x] Apply this file".into()),
                            (String::new(), "[ ] Leave unchanged".into()),
                        ],
                        None,
                    )
                })
                .collect(),
            Kind::Agent => vec![
                Field::text("Agent name", "Your own name and hierarchy", ""),
                Field::select(
                    "Parent",
                    optional(nodes),
                    selected.and_then(|r| match r {
                        OrganizationRecord::Node(n) => Some(n.id.as_str()),
                        _ => None,
                    }),
                ),
                Field::select(
                    "Role version",
                    optional(
                        records
                            .iter()
                            .filter_map(|r| match r {
                                OrganizationRecord::Role(role) => Some((
                                    role.id.clone(),
                                    format!("{} v{}", role.name, role.version),
                                )),
                                _ => None,
                            })
                            .collect(),
                    ),
                    None,
                ),
                Field::text("Model", "Blank uses the configured default", ""),
            ],
            Kind::Goal => vec![
                Field::text("Objective", "Describe the desired outcome", ""),
                Field::text("Success criteria", "One criterion per line", ""),
                Field::select("Parent goal", optional(goals), goal_id),
            ],
            Kind::Project => vec![
                Field::text("Project name", "A local coding or document project", ""),
                Field::text(
                    "Project root",
                    "Existing absolute directory; no files are changed",
                    "",
                ),
                Field::select("Related goal", optional(goals), goal_id),
                Field::select("Owner", optional(agents), owner),
                Field::text("Completion criteria", "One criterion per line", ""),
            ],
            Kind::Task => vec![
                Field::select(
                    "Project",
                    records
                        .iter()
                        .filter_map(|r| match r {
                            OrganizationRecord::Project(p) => Some((p.id.clone(), p.name.clone())),
                            _ => None,
                        })
                        .collect(),
                    project.map(|p| p.id.as_str()),
                ),
                Field::select("Assigned agent", agents, owner),
                Field::select("Related goal", optional(goals), goal_id),
                Field::text("Instructions", "What should the agent do?", ""),
                Field::text("Acceptance criteria", "One criterion per line", ""),
                Field::text("Prerequisites", "Optional task IDs, one per line", ""),
            ],
            Kind::Authorize(task) => {
                let model = records
                    .iter()
                    .find_map(|r| match r {
                        OrganizationRecord::Node(n) if n.id == task.owner_id => n.model.as_deref(),
                        _ => None,
                    })
                    .unwrap_or(model);
                vec![
                    Field::text("Input files", "Project-relative paths, one per line", ""),
                    Field::text(
                        "Immutable inputs",
                        "Read-only inputs, including test oracles; one per line",
                        "",
                    ),
                    Field::text(
                        "Output artifacts",
                        "Project-relative paths, one per line",
                        "",
                    ),
                    Field::text(
                        "Checks (argv JSON)",
                        "Example: [[\"python3\",\"oracle.py\"]]; no shell parsing",
                        "[]",
                    ),
                    Field::text("Model", "Explicit model for this grant", model),
                    Field::text(
                        "Token ceiling",
                        "Reported tokens; an admitted request can exceed this",
                        "10000",
                    ),
                    Field::text("Time limit (seconds)", "1–3600", "240"),
                    Field::text("Model requests", "1–100; provider retries disabled", "5"),
                    Field::text("Attempts", "1–3", "1"),
                    Field::select(
                        "Approve each tool action",
                        vec![
                            ("true".into(), "Yes".into()),
                            (
                                "false".into(),
                                "No — routine in-scope tools preauthorized".into(),
                            ),
                        ],
                        None,
                    ),
                    Field::text(
                        "Allowed delegates",
                        "Optional agent IDs, one per line; hierarchy grants no authority",
                        "",
                    ),
                    Field::text(
                        "Maximum child tasks",
                        "0 disables delegation; maximum 32",
                        "0",
                    ),
                    Field::select(
                        "Accept checked children",
                        vec![
                            ("false".into(), "No".into()),
                            ("true".into(), "Yes — declared checks required".into()),
                        ],
                        None,
                    ),
                    Field::text(
                        "Start time (UTC epoch)",
                        "Optional one-shot time; blank starts when ready",
                        "",
                    ),
                ]
            }
        };
        Self {
            kind,
            fields,
            focus: 0,
            project_key: uuid::Uuid::new_v4().to_string(),
            error: String::new(),
        }
    }
    pub(super) fn title(&self) -> &'static str {
        match self.kind {
            Kind::Agent => "New agent",
            Kind::Goal => "New goal",
            Kind::Project => "New project",
            Kind::Task => "New task",
            Kind::Authorize(_) => "Authorize task",
            Kind::Apply { .. } => "Select files to Apply",
        }
    }
    pub(super) fn key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Tab => self.focus = (self.focus + 1) % self.fields.len(),
            KeyCode::BackTab => {
                self.focus = (self.focus + self.fields.len() - 1) % self.fields.len()
            }
            KeyCode::Enter if key.modifiers.is_empty() => {
                self.focus = (self.focus + 1) % self.fields.len()
            }
            _ => {
                let field = &mut self.fields[self.focus];
                if !field.choices.is_empty() {
                    match key.code {
                        KeyCode::Left | KeyCode::Up => {
                            field.choice =
                                (field.choice + field.choices.len() - 1) % field.choices.len()
                        }
                        KeyCode::Right | KeyCode::Down | KeyCode::Char(' ') => {
                            field.choice = (field.choice + 1) % field.choices.len()
                        }
                        _ => {}
                    }
                } else if field.input.text().len() < 64 * 1024
                    || matches!(key.code, KeyCode::Backspace | KeyCode::Delete)
                {
                    // Modified Enter inserts a line; ordinary Enter advances the form.
                    field.input.input(if key.code == KeyCode::Enter {
                        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
                    } else {
                        key
                    });
                }
            }
        }
    }
    pub(super) fn paste(&mut self, value: &str) {
        let field = &mut self.fields[self.focus];
        if field.choices.is_empty() {
            let value: String = value
                .chars()
                .filter(|c| !c.is_control() || *c == '\n')
                .collect();
            if field.input.text().len() + value.len() <= 64 * 1024 {
                field.input.insert_str_at(field.input.cursor(), &value);
            }
        }
    }
    pub(super) fn command(&self) -> anyhow::Result<Command> {
        let value = |i: usize| self.fields[i].value().to_string();
        let required = |i: usize| -> anyhow::Result<String> {
            let text = value(i);
            anyhow::ensure!(!text.is_empty(), "{} is required", self.fields[i].label);
            Ok(text)
        };
        let optional = |i: usize| (!value(i).is_empty()).then(|| value(i));
        let lines = |i: usize| {
            value(i)
                .lines()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        };
        let criteria = |i| -> anyhow::Result<Vec<String>> {
            required(i)?;
            Ok(lines(i))
        };
        Ok(match &self.kind {
            Kind::Apply { task, result } => {
                anyhow::ensure!(result.accepted, "Result must be accepted before Apply");
                let paths = self
                    .fields
                    .iter()
                    .filter(|field| !field.value().is_empty())
                    .map(|field| field.value().to_string())
                    .collect::<Vec<_>>();
                Command::Execution(OrganizationExecutionMutation::Apply {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    digest: result.digest.clone(),
                    paths: Some(
                        result
                            .application_paths(Some(&paths))
                            .map_err(anyhow::Error::msg)?,
                    ),
                })
            }
            Kind::Agent => Command::Definition(OrganizationMutation::PutNode {
                id: None,
                expected_revision: None,
                parent_id: optional(1),
                kind: OrganizationNodeKind::Agent,
                label: required(0)?,
                role_version_id: optional(2),
                model: optional(3),
                enabled: true,
            }),
            Kind::Goal => Command::Definition(OrganizationMutation::PutGoal {
                id: None,
                expected_revision: None,
                parent_id: optional(2),
                objective: required(0)?,
                criteria: criteria(1)?,
            }),
            Kind::Project => {
                let root = required(1)?;
                anyhow::ensure!(
                    Path::new(&root).is_absolute(),
                    "Project root must be absolute"
                );
                let root = std::fs::canonicalize(root)?;
                anyhow::ensure!(root.is_dir(), "Project root must be an existing directory");
                Command::Project {
                    params: ProjectCreateParams {
                        name: required(0)?,
                        roots: vec![ProjectRoot {
                            path: codex_utils_absolute_path::AbsolutePathBuf::from_absolute_path(
                                root,
                            )?,
                        }],
                        metadata: None,
                        idempotency_key: self.project_key.clone(),
                    },
                    goal_id: optional(2),
                    owner_id: optional(3),
                    criteria: criteria(4)?,
                }
            }
            Kind::Task => Command::Definition(OrganizationMutation::PutTask {
                id: None,
                expected_revision: None,
                project_id: required(0)?,
                owner_id: required(1)?,
                goal_id: optional(2),
                instructions: required(3)?,
                criteria: criteria(4)?,
                prerequisites: lines(5),
            }),
            Kind::Authorize(task) => {
                let number = |i: usize| -> anyhow::Result<u32> {
                    value(i).parse().map_err(|_| {
                        anyhow::anyhow!("{} must be a positive integer", self.fields[i].label)
                    })
                };
                let scope = OrganizationExecutionScope {
                    input_files: lines(0),
                    immutable_files: lines(1),
                    artifacts: criteria(2)?,
                    checks: serde_json::from_str(&value(3)).map_err(|e| {
                        anyhow::anyhow!("Checks must be an array of argv arrays: {e}")
                    })?,
                    model: required(4)?,
                    max_tokens: i64::from(number(5)?),
                    max_seconds: number(6)?,
                    max_model_requests: number(7)?,
                    max_attempts: number(8)?,
                    require_tool_approval: value(9) == "true",
                    delegates: lines(10),
                    max_children: number(11)?,
                    auto_accept_children: value(12) == "true",
                    not_before: optional(13).map(|s| s.parse::<i64>()).transpose()?,
                };
                anyhow::ensure!(
                    (1..=10_000_000).contains(&scope.max_tokens)
                        && (1..=3600).contains(&scope.max_seconds)
                        && (1..=100).contains(&scope.max_model_requests)
                        && (1..=3).contains(&scope.max_attempts)
                        && scope.max_children <= 32,
                    "Budget is outside the supported limits"
                );
                for paths in [&scope.input_files, &scope.immutable_files, &scope.artifacts] {
                    let mut unique = BTreeSet::new();
                    for path in paths {
                        anyhow::ensure!(
                            Path::new(path)
                                .components()
                                .all(|part| matches!(part, std::path::Component::Normal(_)))
                                && !path.contains('\0')
                                && unique.insert(path),
                            "File paths must be unique, relative and without traversal"
                        );
                    }
                }
                anyhow::ensure!(
                    scope
                        .immutable_files
                        .iter()
                        .all(|p| scope.input_files.contains(p)),
                    "Immutable files must also be inputs"
                );
                anyhow::ensure!(
                    !scope.auto_accept_children || !scope.checks.is_empty(),
                    "Checked child acceptance requires declared checks"
                );
                Command::Execution(OrganizationExecutionMutation::Authorize {
                    task_id: task.id.clone(),
                    expected_revision: task.revision,
                    scope,
                })
            }
        })
    }
    pub(super) fn render(&self, area: Rect, buffer: &mut Buffer) -> Option<(u16, u16)> {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " {} · {}/{} ",
                self.title(),
                self.focus + 1,
                self.fields.len()
            ))
            .style(Style::default().fg(super::view::foreground(super::view::BLUE)));
        let inner = block.inner(area);
        block.render(area, buffer);
        let regions = Layout::vertical([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(inner.inner(Margin::new(1, 0)));
        let help = if matches!(self.kind, Kind::Apply { .. }) {
            "Tab / Enter next · Shift-Tab previous · Arrows / Space toggle\nF6 review exact selection · Esc close (no changes)"
        } else {
            "Tab / Enter next · Shift-Tab previous · Left / Right choose\nAlt-Enter newline · F6 review · Esc close (no changes)"
        };
        Paragraph::new(help).render(regions[0], buffer);
        let height = 4;
        let visible = usize::from(regions[1].height / height).max(1);
        let start = self.focus.saturating_sub(visible - 1);
        let mut cursor = None;
        for (row, field) in self.fields.iter().skip(start).take(visible).enumerate() {
            let y = regions[1].y + row as u16 * height;
            let area = Rect::new(
                regions[1].x,
                y,
                regions[1].width,
                height.min(regions[1].bottom().saturating_sub(y)),
            );
            let focused = start + row == self.focus;
            let block = Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " {}{} ",
                    if focused { "› " } else { "" },
                    field.label
                ))
                .border_style(Style::default().fg(if focused {
                    super::view::foreground(super::view::BLUE)
                } else {
                    Color::DarkGray
                }));
            let inner = block.inner(area);
            block.render(area, buffer);
            if let Some((_, label)) = field.choices.get(field.choice) {
                Paragraph::new(clean(&format!("‹ {label} ›"))).render(inner, buffer);
            } else {
                let mut state = TextAreaState::default();
                field.input.render_ref_styled_with_highlights(
                    inner,
                    buffer,
                    &mut state,
                    Style::default().fg(Color::White),
                    &[],
                );
                if focused {
                    cursor = field.input.cursor_pos_with_state(inner, state);
                }
            }
        }
        let hint = if matches!(self.kind, Kind::Apply { .. }) {
            format!(
                "{} / {} files selected · One Apply per accepted result\nFull result checks do not certify the installed subset",
                self.fields.iter().filter(|f| !f.value().is_empty()).count(),
                self.fields.len()
            )
        } else {
            self.fields[self.focus].hint.to_string()
        };
        Paragraph::new(clean(&format!("{hint}\n{}", self.error)))
            .style(Style::default().fg(Color::Yellow))
            .wrap(Wrap { trim: false })
            .render(regions[2], buffer);
        cursor
    }
}

//! Concept-art shell rendered with existing terminal widgets and canonical records.
use super::*;
use ratatui::layout::Margin;
use ratatui::style::Modifier;
use ratatui::text::Span;
use ratatui::widgets::{BorderType, Gauge};

const BACKGROUND: (u8, u8, u8) = (8, 15, 23);
const PANEL: (u8, u8, u8) = (11, 20, 29);
const BORDER: (u8, u8, u8) = (35, 53, 70);
const TEXT: (u8, u8, u8) = (219, 230, 242);
const MUTED: (u8, u8, u8) = (144, 165, 190);
pub(super) const BLUE: (u8, u8, u8) = (77, 170, 255);
const GREEN: (u8, u8, u8) = (40, 235, 142);
const SELECTED: (u8, u8, u8) = (20, 43, 65);

fn background(rgb: (u8, u8, u8)) -> Color {
    crate::terminal_palette::best_color(rgb)
}

pub(super) fn foreground(rgb: (u8, u8, u8)) -> Color {
    crate::style::readable_color_on(background(rgb), Some(background(PANEL)))
}

fn panel(title: impl Into<String>, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(format!(" {} ", title.into()))
        .style(Style::default().bg(background(PANEL)).fg(foreground(TEXT)))
        .border_style(Style::default().fg(if focused {
            foreground(BLUE)
        } else {
            foreground(BORDER)
        }))
}

fn state_color(state: &str) -> Color {
    match state {
        "running" | "accepted" | "completed" => foreground(GREEN),
        "failed" | "cancelled" | "needsReconciliation" => foreground((255, 88, 102)),
        "waitingApproval" | "needsReview" | "paused" | "waitingDependency" | "blocked" => {
            foreground((255, 196, 70))
        }
        _ => foreground(BLUE),
    }
}

impl Dashboard {
    pub(super) fn refresh_diff(&mut self) {
        let Some(view) = &self.detail else {
            self.review_diff = None;
            return;
        };
        let Some(result) = &view.result else {
            self.review_diff = None;
            return;
        };
        if self
            .review_diff
            .as_ref()
            .is_some_and(|(task, digest, _)| task == &view.task.id && digest == &result.digest)
        {
            return;
        }
        let diff = (|| -> anyhow::Result<String> {
            let artifact = result.artifacts.get(self.review_file % result.artifacts.len().max(1)).ok_or_else(|| anyhow::anyhow!("No declared artifact"))?;
            anyhow::ensure!(artifact.bytes <= 65536, "Artifact exceeds the 64 KiB inline diff limit; use actual Neovim");
            let workspace = view.attempts.iter().find(|a| a.id == result.attempt_id).and_then(|a| a.workspace.as_ref()).ok_or_else(|| anyhow::anyhow!("Attempt workspace unavailable"))?;
            let project = self.selected_project().ok_or_else(|| anyhow::anyhow!("Project unavailable"))?;
            let root = Path::new(project.roots.first().ok_or_else(|| anyhow::anyhow!("Project root missing"))?);
            let preview = Preview::new(self.path.parent().ok_or_else(|| anyhow::anyhow!("Owner home missing"))?, Path::new(workspace), &artifact.path, Some(artifact))?;
            let modified = std::fs::read_to_string(preview.path())?;
            let mut base = String::new();
            match codex_utils_path::open_relative_regular_file(root, Path::new(&artifact.path)) {
                Ok(file) => {
                    file.take(65537).read_to_string(&mut base)?;
                    anyhow::ensure!(base.len() <= 65536, "Original exceeds the inline diff limit; use actual Neovim");
                },
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
                Err(e) => return Err(e.into()),
            }
            Ok(format!("{}\nOriginal snapshot at load → verified artifact\nApply rechecks the current original base.\n\n{}", artifact.path, diffy::create_patch(&base, &modified)))
        })().unwrap_or_else(|e| format!("Inline diff unavailable: {e}\nF2 Workspace, d opens the real Neovim diff."));
        self.review_diff = Some((view.task.id.clone(), result.digest.clone(), clean(&diff)));
    }

    pub(super) fn selected_project(&self) -> Option<&OrganizationProjectPlan> {
        let id = match self.record()? {
            OrganizationRecord::Project(p) => return Some(p),
            OrganizationRecord::Task(t) => &t.project_id,
            _ => return None,
        };
        self.snapshot.records.iter().find_map(|r| match r {
            OrganizationRecord::Project(p) if &p.id == id => Some(p),
            _ => None,
        })
    }

    fn sidebar(&self, area: Rect, buffer: &mut Buffer) {
        let regions = Layout::vertical([Constraint::Min(1), Constraint::Length(5)]).split(area);
        let mut lines = vec![
            Line::styled(" WORKSPACE", Style::default().fg(foreground(MUTED))),
            Line::default(),
        ];
        for (key, preset) in [
            (1, Preset::Console),
            (2, Preset::Workspace),
            (3, Preset::Projects),
            (4, Preset::Operations),
            (5, Preset::Review),
        ] {
            lines.push(Line::styled(
                format!(" {key}  {}", preset.title()),
                if self.preferences.preset == preset {
                    Style::default()
                        .fg(foreground(BLUE))
                        .bg(background(SELECTED))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(foreground(MUTED))
                },
            ));
        }
        if matches!(
            self.preferences.preset,
            Preset::Projects | Preset::Operations
        ) {
            lines.push(Line::default());
            lines.push(Line::styled(
                " w  Templates / runs",
                if self.workflow_view {
                    Style::default()
                        .fg(foreground(BLUE))
                        .bg(background(SELECTED))
                } else {
                    Style::default().fg(foreground(MUTED))
                },
            ));
            lines.push(Line::styled(
                " S  Schedules",
                Style::default().fg(foreground(MUTED)),
            ));
        }
        if self.preferences.preset == Preset::Workspace {
            lines.extend([
                Line::default(),
                Line::styled(" FILES", Style::default().fg(foreground(MUTED))),
            ]);
            if let Some(project) = self.selected_project() {
                lines.push(Line::styled(
                    clean(&project.name),
                    Style::default().fg(foreground(BLUE)),
                ));
            }
            if let Some(view) = &self.detail
                && Some(&view.task.id) == self.selected.as_ref()
                && let Some(grant) = &view.authorization
            {
                let files: BTreeSet<_> = grant
                    .scope
                    .artifacts
                    .iter()
                    .chain(&grant.scope.input_files)
                    .collect();
                for file in files {
                    lines.push(Line::from(clean(&format!("  · {file}"))));
                }
            }
            lines.extend([
                Line::default(),
                Line::styled(" t Follow AI", Style::default().fg(foreground(GREEN))),
                Line::styled(" d Live Diff", Style::default().fg(foreground(BLUE))),
                Line::styled(
                    " o Independent Editing",
                    Style::default().fg(foreground(MUTED)),
                ),
            ]);
            Paragraph::new(lines)
                .block(panel("Explorer", false))
                .render(regions[0], buffer);
            Paragraph::new("C O D E X\nOfficial Neovim\nCtrl-G host focus")
                .style(Style::default().fg(foreground(MUTED)))
                .block(panel("", false))
                .render(regions[1], buffer);
            return;
        }
        lines.extend([
            Line::default(),
            Line::styled(" PROJECTS      N +", Style::default().fg(foreground(MUTED))),
        ]);
        let selected = self.selected_project().map(|p| p.id.as_str());
        for record in &self.snapshot.records {
            if let OrganizationRecord::Project(p) = record {
                lines.push(Line::styled(
                    clean(&format!(
                        " {} {}",
                        if selected == Some(p.id.as_str()) {
                            "›"
                        } else {
                            "·"
                        },
                        p.name
                    )),
                    Style::default().fg(if selected == Some(p.id.as_str()) {
                        foreground(BLUE)
                    } else {
                        foreground(TEXT)
                    }),
                ));
            }
        }
        lines.extend([
            Line::default(),
            Line::styled(
                " AGENTS · enabled U +",
                Style::default().fg(foreground(MUTED)),
            ),
        ]);
        for record in &self.snapshot.records {
            if let OrganizationRecord::Node(n) = record
                && n.kind == OrganizationNodeKind::Agent
            {
                lines.push(Line::from(vec![
                    Span::styled(
                        if n.enabled { " ● " } else { " ○ " },
                        Style::default().fg(if n.enabled {
                            foreground(GREEN)
                        } else {
                            foreground(MUTED)
                        }),
                    ),
                    Span::raw(clean(&n.label)),
                ]));
            }
        }
        Paragraph::new(lines)
            .block(panel("Navigation", false))
            .render(regions[0], buffer);
        Paragraph::new("C O D E X\nLocal autonomous work\nOwner-controlled")
            .style(Style::default().fg(foreground(MUTED)))
            .block(panel("", false))
            .render(regions[1], buffer);
    }

    fn summary(&self, area: Rect, buffer: &mut Buffer) {
        let mut counts = [0usize; 6];
        for record in &self.snapshot.records {
            match record {
                OrganizationRecord::Node(n) if n.kind == OrganizationNodeKind::Agent => {
                    counts[0] += 1
                }
                OrganizationRecord::Task(t) => match t.state.as_str() {
                    "running" => counts[1] += 1,
                    "failed" => counts[2] += 1,
                    "waitingApproval" => counts[3] += 1,
                    "needsReview" => counts[4] += 1,
                    _ => {}
                },
                OrganizationRecord::Project(_) => counts[5] += 1,
                _ => {}
            }
        }
        let names = [
            "Agents", "Running", "Failed", "Approval", "Review", "Projects",
        ];
        let areas = Layout::horizontal([Constraint::Ratio(1, 6); 6]).split(area);
        for (i, area) in areas.iter().enumerate() {
            Paragraph::new(format!("{}", counts[i]))
                .style(
                    Style::default()
                        .fg(if i == 2 {
                            state_color("failed")
                        } else if i == 3 || i == 4 {
                            state_color("needsReview")
                        } else {
                            foreground(GREEN)
                        })
                        .add_modifier(Modifier::BOLD),
                )
                .block(panel(names[i], false))
                .render(*area, buffer);
        }
    }

    fn rows_view(&self, area: Rect, buffer: &mut Buffer, cards: bool) {
        let pane = &self.preferences.panes[self.preferences.preset.index()];
        let title = if self.workflow_view {
            "Workflows · w returns to default view"
        } else if self.schedule_view {
            "Schedules · S returns to default view"
        } else if self.preferences.preset == Preset::Review {
            "Action inbox · z summary · C capabilities"
        } else if cards {
            "Hierarchy tree"
        } else {
            "Tasks / goals / projects"
        };
        let block = panel(
            format!("{title} · {} · /{}", self.rows.len(), clean(&self.search)),
            !pane.inspector_focus,
        );
        let inner = block.inner(area);
        block.render(area, buffer);
        let selected = self
            .rows
            .iter()
            .position(|r| Some(&r.id) == self.selected.as_ref())
            .unwrap_or(0);
        let step = if cards && inner.width >= 44 && inner.height >= 8 {
            3
        } else {
            1
        };
        let height = usize::from(inner.height / step);
        let start = selected.saturating_sub(height.saturating_sub(1));
        if self.rows.is_empty() {
            Paragraph::new("No matching records.\n\nN New project   T New task\nG New goal      U New agent\n\nDefinitions stay idle until authorized.").style(Style::default().fg(foreground(MUTED))).wrap(Wrap { trim: false }).render(inner.inner(Margin::new(1, 0)), buffer);
        }
        for (line, (index, row)) in self
            .rows
            .iter()
            .enumerate()
            .skip(start)
            .take(height)
            .enumerate()
        {
            let y = inner.y + line as u16 * step;
            let area = Rect::new(inner.x, y, inner.width, step);
            let selected = index == selected;
            let state = self.snapshot.records.iter().find_map(|r| match r {
                OrganizationRecord::Task(t) if t.id == row.id => Some(t.state.as_str()),
                _ => None,
            });
            let style = Style::default()
                .fg(if selected {
                    foreground(BLUE)
                } else {
                    state.map(state_color).unwrap_or(foreground(TEXT))
                })
                .bg(if selected {
                    background(SELECTED)
                } else {
                    background(PANEL)
                });
            let content = clean(&format!(
                "{} {}",
                if selected { "›" } else { " " },
                row.label
            ));
            if step == 3 {
                Paragraph::new(content)
                    .style(style)
                    .block(panel("", selected))
                    .render(area, buffer);
            } else {
                Paragraph::new(content).style(style).render(area, buffer);
            }
        }
    }

    fn project_summary(&self, area: Rect, buffer: &mut Buffer) {
        let Some(project) = self.selected_project() else {
            Paragraph::new(
                "Select a project or task to view progress.\nN project · T task · G goal · U agent",
            )
            .style(Style::default().fg(foreground(MUTED)))
            .block(panel("Project plan", false))
            .render(area, buffer);
            return;
        };
        let tasks: Vec<_> = self
            .snapshot
            .records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Task(t) if t.project_id == project.id => Some(t),
                _ => None,
            })
            .collect();
        let accepted = tasks.iter().filter(|t| t.state == "accepted").count();
        let block = panel(clean(&project.name), false);
        let inner = block.inner(area);
        block.render(area, buffer);
        let regions = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(1),
        ])
        .split(inner);
        Paragraph::new(clean(&project.criteria.join(" · ")))
            .style(Style::default().fg(foreground(MUTED)))
            .wrap(Wrap { trim: false })
            .render(regions[0], buffer);
        Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(foreground(GREEN))
                    .bg(background(BORDER)),
            )
            .ratio(if tasks.is_empty() {
                0.0
            } else {
                accepted as f64 / tasks.len() as f64
            })
            .label(format!("{accepted} / {} tasks accepted", tasks.len()))
            .render(regions[1], buffer);
    }

    fn activity(&self, area: Rect, buffer: &mut Buffer) {
        let mut lines = Vec::new();
        if let Some(view) = &self.detail
            && Some(&view.task.id) == self.selected.as_ref()
        {
            for action in view.actions.iter().rev().take(5) {
                lines.push(Line::styled(
                    clean(&format!("● {} · {}", action.tool, action.state.as_str())),
                    Style::default().fg(state_color(action.state.as_str())),
                ));
            }
        }
        if lines.is_empty() {
            lines.push(Line::styled(
                "Select a task for recorded actions.",
                Style::default().fg(foreground(MUTED)),
            ));
        }
        Paragraph::new(lines)
            .block(panel("Recent activity", false))
            .render(area, buffer);
    }

    pub(super) fn render(&mut self, area: Rect, buffer: &mut Buffer) -> Option<(u16, u16)> {
        Clear.render(area, buffer);
        buffer.set_style(
            area,
            Style::default()
                .fg(foreground(TEXT))
                .bg(background(BACKGROUND)),
        );
        let regions = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(area);
        let project = self
            .selected_project()
            .map(|p| p.name.as_str())
            .unwrap_or("Local workspace");
        let running = self
            .snapshot
            .records
            .iter()
            .filter(|r| matches!(r, OrganizationRecord::Task(t) if t.state == "running"))
            .count();
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    " C O D E X  ›  ",
                    Style::default()
                        .fg(foreground(TEXT))
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    clean(&format!("Workspace / {project}")),
                    Style::default().fg(foreground(MUTED)),
                ),
                Span::styled(
                    format!("    ● {running} running    Owner control"),
                    Style::default().fg(foreground(GREEN)),
                ),
            ]),
            Line::styled(
                " F1 Console   F2 Workspace   F3 Projects   F4 Operations   F5 Review",
                Style::default().fg(foreground(BLUE)),
            ),
        ])
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(background(BORDER))),
        )
        .render(regions[0], buffer);
        let pane = &self.preferences.panes[self.preferences.preset.index()];
        let body = if pane.sidebar && area.width >= 110 {
            let columns =
                Layout::horizontal([Constraint::Length(pane.sidebar_width), Constraint::Min(1)])
                    .split(regions[1]);
            self.sidebar(columns[0], buffer);
            columns[1]
        } else {
            regions[1]
        };
        let areas = Layout::default()
            .direction(if body.width < 80 {
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
            .split(body);
        let mut cursor = None;
        if self.confirmation.is_some() {
            Paragraph::new(self.confirmation_text.as_str())
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .block(panel(
                    "Review exact owner decision · y confirm · Esc cancel",
                    true,
                ))
                .render(body, buffer);
        } else if let Some(text) = &self.report_text {
            Paragraph::new(text.as_str())
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .style(Style::default().fg(foreground(TEXT)))
                .block(panel(
                    "Evidence summary / capabilities · ↑↓ / PgUp/PgDn scroll · Esc close",
                    true,
                ))
                .render(body, buffer);
        } else if let Some(form) = &self.form {
            cursor = form.render(body, buffer);
        } else if self.memory_request.is_some() {
            Paragraph::new(self.memory_text())
                .wrap(Wrap { trim: false })
                .scroll((self.scroll, 0))
                .style(Style::default().fg(foreground(TEXT)))
                .block(panel(
                    "Scoped memory · / query · f refresh · ↑↓ scroll · Esc close",
                    true,
                ))
                .render(body, buffer);
        } else {
            let central = areas[0];
            let subtitle = match self.preferences.preset {
                Preset::Operations => {
                    "Monitor and supervise autonomous work across your organization."
                }
                Preset::Projects => "Define goals, assign tasks and review completion criteria.",
                Preset::Workspace => "Actual Neovim · Follow AI / Live Diff / Independent Editing.",
                Preset::Review => {
                    "Action inbox: approvals, blockers, requested changes and results."
                }
                Preset::Console => "Plan and build with your agents.",
            };
            let sections = Layout::vertical([
                Constraint::Length(if central.height >= 14 { 3 } else { 1 }),
                Constraint::Min(1),
            ])
            .split(central);
            Paragraph::new(vec![
                Line::styled(
                    format!(" {}", self.preferences.preset.title()),
                    Style::default()
                        .fg(foreground(TEXT))
                        .add_modifier(Modifier::BOLD),
                ),
                Line::styled(
                    format!(" {subtitle}"),
                    Style::default().fg(foreground(MUTED)),
                ),
            ])
            .render(sections[0], buffer);
            match self.preferences.preset {
                Preset::Operations => {
                    let parts = Layout::vertical([
                        Constraint::Length(if sections[1].height >= 16 && central.width >= 60 {
                            4
                        } else {
                            0
                        }),
                        Constraint::Min(1),
                        Constraint::Length(if sections[1].height >= 22 { 6 } else { 0 }),
                    ])
                    .split(sections[1]);
                    self.summary(parts[0], buffer);
                    self.rows_view(parts[1], buffer, !self.schedule_view);
                    let bottom = Layout::horizontal([
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ])
                    .split(parts[2]);
                    let mut upcoming: Vec<_> = self
                        .snapshot
                        .schedules
                        .iter()
                        .filter(|v| v.schedule.state == OrganizationScheduleState::Active)
                        .collect();
                    upcoming.sort_by_key(|v| v.schedule.next_run_at);
                    let lines: Vec<_> = upcoming
                        .iter()
                        .take(4)
                        .map(|v| {
                            Line::from(clean(&format!(
                                "{} · {}",
                                v.schedule.config.label,
                                v.schedule
                                    .next_run_at
                                    .map(utc_time)
                                    .unwrap_or_else(|| "None".into())
                            )))
                        })
                        .collect();
                    Paragraph::new(if lines.is_empty() {
                        vec![Line::from("No active schedules · R create / S inspect")]
                    } else {
                        lines
                    })
                    .style(Style::default().fg(foreground(MUTED)))
                    .block(panel("Upcoming schedules · UTC", false))
                    .render(bottom[0], buffer);
                    self.activity(bottom[1], buffer);
                }
                Preset::Projects => {
                    let parts = Layout::vertical([
                        Constraint::Length(if sections[1].height >= 16 { 6 } else { 0 }),
                        Constraint::Min(1),
                    ])
                    .split(sections[1]);
                    self.project_summary(parts[0], buffer);
                    self.rows_view(parts[1], buffer, false);
                }
                Preset::Workspace if self.editor.is_some() => {
                    let Some(editor) = &self.editor else {
                        return None;
                    };
                    let block = panel(
                        format!(
                            "Neovim · {} · {} · Ctrl-G host",
                            clean(&editor.mode),
                            if editor.focused {
                                "editor focus"
                            } else {
                                "host focus"
                            }
                        ),
                        editor.focused,
                    );
                    self.editor_area = block.inner(sections[1]);
                    block.render(sections[1], buffer);
                    cursor = editor.render(self.editor_area, buffer);
                    if let Some(error) = &editor.error {
                        Paragraph::new(clean(error)).render(self.editor_area, buffer);
                    }
                }
                Preset::Review
                    if self
                        .review_diff
                        .as_ref()
                        .is_some_and(|(id, _, _)| Some(id) == self.selected.as_ref()) =>
                {
                    let parts = Layout::vertical([
                        Constraint::Length(if sections[1].height >= 16 { 6 } else { 3 }),
                        Constraint::Min(1),
                    ])
                    .split(sections[1]);
                    self.rows_view(parts[0], buffer, false);
                    let Some((_, _, diff)) = &self.review_diff else {
                        return None;
                    };
                    let lines: Vec<_> = diff
                        .lines()
                        .map(|line| {
                            Line::styled(
                                line,
                                Style::default().fg(
                                    if line.starts_with('+') && !line.starts_with("+++") {
                                        foreground(GREEN)
                                    } else if line.starts_with('-') && !line.starts_with("---") {
                                        state_color("failed")
                                    } else if line.starts_with("@@") {
                                        foreground(BLUE)
                                    } else {
                                        foreground(MUTED)
                                    },
                                ),
                            )
                        })
                        .collect();
                    Paragraph::new(lines)
                        .scroll((self.scroll, 0))
                        .block(panel(
                            format!(
                                "Diff · file {} · n next · F2/d Neovim",
                                self.review_file + 1
                            ),
                            false,
                        ))
                        .render(parts[1], buffer);
                }
                _ => self.rows_view(sections[1], buffer, false),
            }
            if pane.inspector {
                let title = if self.selected_workflow().is_some() {
                    "Workflow Inspector"
                } else if self.selected_schedule().is_some() {
                    "Schedule Inspector"
                } else {
                    match self.preferences.preset {
                        Preset::Operations => "Node Inspector",
                        Preset::Workspace => "AI Collaboration",
                        Preset::Projects => "Task Details",
                        Preset::Review => "Change Inspector",
                        Preset::Console => "Task Inspector",
                    }
                };
                let right = Layout::vertical([
                    Constraint::Min(1),
                    Constraint::Length(if pane.actions && areas[1].height >= 14 {
                        6
                    } else {
                        0
                    }),
                ])
                .split(areas[1]);
                Paragraph::new(self.detail_text.as_str())
                    .wrap(Wrap { trim: false })
                    .scroll((self.scroll, 0))
                    .style(Style::default().fg(foreground(MUTED)))
                    .block(panel(
                        format!(
                            "{title}{}",
                            if pane.inspector_focus {
                                " [focused]"
                            } else {
                                ""
                            }
                        ),
                        pane.inspector_focus,
                    ))
                    .render(right[0], buffer);
                let actions = if self.selected_workflow().is_some() {
                    "[s] Preview template run\n[w] Workflows / default view\n[W] Save selected task chain\nRun tasks: select approval/result"
                } else if self.selected_schedule().is_some() {
                    "[p] Pause       [u] Resume\n[x] Cancel      [f] Refresh\n[S] Schedules / default view\nRun approvals and results: select task"
                } else if self.preferences.preset == Preset::Workspace {
                    "[t] Follow AI   [d] Live Diff\n[o] Independent Editing\n[e] Editor focus   [n] Next file\nCtrl-G host · Q task from selection"
                } else {
                    "[s] Authorize   [a] Approve\n[v] Accept      [c] Changes\n[A] Select files to Apply\n[p] Pause       [x] Cancel"
                };
                Paragraph::new(actions)
                    .style(Style::default().fg(foreground(BLUE)))
                    .block(panel("Actions", false))
                    .render(right[1], buffer);
            }
        }
        let prompt = self
            .input
            .as_ref()
            .map(|s| {
                format!(
                    "{}{}",
                    if self.memory_request.is_some() {
                        "Memory query: "
                    } else if self.changes {
                        "Changes: "
                    } else {
                        "Search: "
                    },
                    s
                )
            })
            .unwrap_or_else(|| self.message.clone());
        let hints = if self.memory_request.is_some() {
            [
                "Enter searches · / edit query · f refresh · Esc close memory",
                "↑↓ / PageUp / PageDown scroll · Empty query lists recent notes",
            ]
        } else {
            [
                "N project · T task · G goal · U agent · E edit · ↑↓ select · Tab focus · [ ] inspector · \\ sidebar · { } width · ~ actions · 0 reset",
                "z/Z summaries · C capabilities · W/w workflows · s authorize · R/S recurring · H blocker · m/M memory · v accept · A Apply · / search · f refresh · q Console",
            ]
        };
        Paragraph::new(vec![
            Line::styled(clean(&prompt), Style::default().fg(foreground(MUTED))),
            Line::styled(hints[0], Style::default().fg(foreground(BLUE))),
            Line::styled(hints[1], Style::default().fg(foreground(MUTED))),
        ])
        .render(regions[2], buffer);
        cursor
    }
}

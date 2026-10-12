//! A terminal viewport over the existing owner-defined forest; no execution authority.
use super::*;
use crossterm::event::KeyModifiers;
use view::{foreground, panel, state_color};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(super) struct Settings {
    pub compact: bool,
    pub overview: bool,
    pub list: bool,
    pub focus: Option<String>,
    pub filter: u8,
    pub inspector_tab: u8,
    pub pan: (u32, u32),
    #[serde(skip)]
    pub cpu_percent: Option<f64>,
    #[serde(skip)]
    centered: Option<String>,
    #[serde(skip)]
    size: (u16, u16),
}

#[derive(Debug, Clone, Default)]
pub(super) struct Counts {
    pub agents: usize,
    pub tasks: usize,
    pub running: usize,
    pub issues: usize,
    pub review: usize,
    pub accepted: usize,
}
impl Counts {
    fn add(&mut self, other: &Self) {
        self.agents += other.agents;
        self.tasks += other.tasks;
        self.running += other.running;
        self.issues += other.issues;
        self.review += other.review;
        self.accepted += other.accepted;
    }
}

#[derive(Debug, Clone)]
pub(super) struct Card {
    pub id: String,
    pub parent: Option<String>,
    pub label: String,
    pub role: String,
    pub counts: Counts,
    pub state: &'static str,
    pub children: Vec<usize>,
    pub child_count: usize,
    pub position: (u32, u32),
}

#[derive(Default)]
pub(super) struct Graph {
    pub cards: Vec<Card>,
    pub width: u32,
    pub height: u32,
    pub hits: Vec<(Rect, String)>,
    pub toggle_hits: Vec<(Rect, String)>,
    pub tab_hits: Vec<(Rect, u8)>,
    card_size: (u32, u32),
    goals: bool,
}

impl Graph {
    pub fn goals(
        records: &[OrganizationRecord],
        collapsed: &BTreeSet<String>,
        search: &str,
        settings: &Settings,
    ) -> Self {
        let tasks: BTreeMap<_, _> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Task(t) => Some((t.id.as_str(), t)),
                _ => None,
            })
            .collect();
        let goals: BTreeSet<_> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Goal(g) => Some(g.id.as_str()),
                _ => None,
            })
            .collect();
        let mut projected = Vec::new();
        for record in records {
            let (id, parent, label) = match record {
                OrganizationRecord::Goal(g) => {
                    (g.id.clone(), g.parent_id.clone(), g.objective.clone())
                }
                OrganizationRecord::Task(t)
                    if t.goal_id.as_deref().is_some_and(|id| goals.contains(id)) =>
                {
                    (
                        t.id.clone(),
                        t.goal_id.clone(),
                        format!(
                            "{} · {}",
                            t.instructions.chars().take(80).collect::<String>(),
                            t.state
                        ),
                    )
                }
                _ => continue,
            };
            projected.push(OrganizationRecord::Node(OrganizationNode {
                id,
                parent_id: parent,
                kind: OrganizationNodeKind::Group,
                label,
                role_version_id: None,
                model: None,
                enabled: true,
                revision: 1,
            }));
            if let OrganizationRecord::Task(t) = record {
                let mut t = t.clone();
                t.owner_id = t.id.clone();
                projected.push(OrganizationRecord::Task(t));
            }
        }
        let mut graph = Self::build(&projected, collapsed, search, settings);
        graph.goals = true;
        for card in &mut graph.cards {
            if goals.contains(card.id.as_str()) {
                card.role = "Goal · criteria need owner review".into();
                card.state = "defined";
            } else if let Some(task) = tasks.get(card.id.as_str()) {
                let blocked = task
                    .prerequisites
                    .iter()
                    .filter(|id| tasks.get(id.as_str()).is_none_or(|t| t.state != "accepted"))
                    .count();
                card.role = format!(
                    "{} prerequisites · {blocked} pending",
                    task.prerequisites.len()
                );
                card.state = match task.state.as_str() {
                    "accepted" => "accepted",
                    "waitingDependency" => "waitingDependency",
                    "running" => "running",
                    "blocked" => "blocked",
                    "needsReconciliation" => "needsReconciliation",
                    "failed" => "failed",
                    "waitingApproval" => "waitingApproval",
                    "needsReview" => "needsReview",
                    _ => "defined",
                };
            }
        }
        graph
    }
    pub fn build(
        records: &[OrganizationRecord],
        collapsed: &BTreeSet<String>,
        search: &str,
        settings: &Settings,
    ) -> Self {
        let nodes: BTreeMap<_, _> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Node(n) => Some((n.id.as_str(), n)),
                _ => None,
            })
            .collect();
        let roles: BTreeMap<_, _> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Role(r) => Some((r.id.as_str(), r.name.as_str())),
                _ => None,
            })
            .collect();
        let projects: BTreeMap<_, _> = records
            .iter()
            .filter_map(|r| match r {
                OrganizationRecord::Project(p) => Some((p.id.as_str(), p.name.as_str())),
                _ => None,
            })
            .collect();
        let mut counts: BTreeMap<&str, Counts> = nodes
            .iter()
            .map(|(id, n)| {
                (
                    *id,
                    Counts {
                        agents: usize::from(n.kind == OrganizationNodeKind::Agent),
                        ..Counts::default()
                    },
                )
            })
            .collect();
        let mut project_text: BTreeMap<&str, String> = BTreeMap::new();
        for r in records {
            if let OrganizationRecord::Task(t) = r {
                if let Some(c) = counts.get_mut(t.owner_id.as_str()) {
                    c.tasks += 1;
                    c.accepted += usize::from(t.state == "accepted");
                    c.running += usize::from(t.state == "running");
                    c.issues += usize::from(matches!(
                        t.state.as_str(),
                        "failed" | "blocked" | "needsReconciliation"
                    ));
                    c.review += usize::from(matches!(
                        t.state.as_str(),
                        "waitingApproval" | "needsReview"
                    ));
                }
                project_text
                    .entry(&t.owner_id)
                    .or_default()
                    .push_str(&format!(
                        " {} {} ",
                        t.project_id,
                        projects.get(t.project_id.as_str()).copied().unwrap_or("")
                    ));
            }
        }
        let mut children: BTreeMap<Option<&str>, Vec<&str>> = BTreeMap::new();
        for (id, n) in &nodes {
            children
                .entry(n.parent_id.as_deref().filter(|id| nodes.contains_key(id)))
                .or_default()
                .push(id);
        }
        for siblings in children.values_mut() {
            siblings.sort_by_key(|id| (&nodes[id].label, *id));
        }
        let roots = settings
            .focus
            .as_deref()
            .filter(|id| nodes.contains_key(id))
            .map(|id| vec![id])
            .unwrap_or_else(|| children.get(&None).cloned().unwrap_or_default());
        let mut order = Vec::new();
        let mut stack: Vec<_> = roots.into_iter().rev().collect();
        let mut seen = BTreeSet::new();
        while let Some(id) = stack.pop() {
            if !seen.insert(id) {
                continue;
            }
            order.push(id);
            if let Some(c) = children.get(&Some(id)) {
                stack.extend(c.iter().rev().copied());
            }
        }
        // Roll up once, independently of collapsed/displayed branches. Counts describe loaded tasks.
        for id in order.iter().rev() {
            if let Some(parent) = nodes[id].parent_id.as_deref().filter(|p| seen.contains(p)) {
                let c = counts[id].clone();
                if let Some(parent) = counts.get_mut(parent) {
                    parent.add(&c);
                }
            }
        }
        let query = search.to_lowercase();
        let mut keep = BTreeSet::new();
        let filtering = !query.is_empty() || !settings.filter.is_multiple_of(4);
        if filtering {
            for id in &order {
                let n = nodes[id];
                let role = n
                    .role_version_id
                    .as_deref()
                    .and_then(|id| roles.get(id))
                    .copied()
                    .unwrap_or("");
                let text = format!(
                    "{} {} {} {}",
                    n.id,
                    n.label,
                    role,
                    project_text.get(id).map(String::as_str).unwrap_or("")
                )
                .to_lowercase();
                let c = &counts[id];
                let matches = if let Some(project_query) = query.strip_prefix("project:") {
                    project_text
                        .get(id)
                        .is_some_and(|p| p.to_lowercase().contains(project_query))
                } else {
                    text.contains(&query)
                } && match settings.filter % 4 {
                    1 => c.running > 0,
                    2 => c.issues > 0,
                    3 => c.review > 0,
                    _ => true,
                };
                if matches {
                    let mut ancestor = Some(*id);
                    while let Some(id) = ancestor.filter(|id| seen.contains(id)) {
                        if !keep.insert(id) {
                            break;
                        }
                        ancestor = nodes[id].parent_id.as_deref();
                    }
                }
            }
        }
        let mut graph = Self {
            card_size: if settings.compact { (18, 3) } else { (28, 5) },
            ..Self::default()
        };
        let mut indices = BTreeMap::new();
        for id in order {
            if filtering && !keep.contains(id) {
                continue;
            }
            let n = nodes[id];
            let parent = n
                .parent_id
                .as_deref()
                .and_then(|id| indices.get(id))
                .copied();
            if !filtering
                && n.parent_id.as_deref().is_some_and(|p| seen.contains(p))
                && parent.is_none()
            {
                continue;
            }
            if !filtering && parent.is_some_and(|i: usize| collapsed.contains(&graph.cards[i].id)) {
                continue;
            }
            let c = counts[id].clone();
            let state = if !n.enabled {
                "disabled"
            } else if c.issues > 0 {
                "blocked"
            } else if c.running > 0 {
                "running"
            } else if c.review > 0 {
                "needsReview"
            } else {
                "idle"
            };
            let index = graph.cards.len();
            indices.insert(id, index);
            if let Some(parent) = parent {
                graph.cards[parent].children.push(index);
            }
            graph.cards.push(Card {
                id: id.into(),
                parent: parent.map(|i| graph.cards[i].id.clone()),
                label: clean(&n.label),
                role: clean(
                    n.role_version_id
                        .as_deref()
                        .and_then(|id| roles.get(id))
                        .copied()
                        .unwrap_or(if n.kind == OrganizationNodeKind::Group {
                            "Group"
                        } else {
                            "Agent"
                        }),
                ),
                counts: c,
                state,
                children: vec![],
                child_count: children.get(&Some(id)).map(Vec::len).unwrap_or_default(),
                position: (0, 0),
            });
        }
        // The concept uses one column per top-level branch, with descendant branches
        // stacked inside that column. Arbitrary deeper levels remain scrollable.
        let spacing = graph.card_size.0 + 10;
        let roots: Vec<_> = graph
            .cards
            .iter()
            .enumerate()
            .filter_map(|(i, c)| c.parent.is_none().then_some(i))
            .collect();
        let mut root_x = 0;
        for root in roots {
            let columns = graph.cards[root].children.len().max(1) as u32;
            graph.cards[root].position = (root_x + (columns - 1) * spacing / 2, 0);
            for (column, child) in graph.cards[root].children.clone().into_iter().enumerate() {
                let mut y = graph.card_size.1 + 3;
                let mut stack = vec![(child, 0u32)];
                while let Some((i, depth)) = stack.pop() {
                    graph.cards[i].position =
                        (root_x + column as u32 * spacing + 2 + depth.min(3) * 2, y);
                    y += graph.card_size.1 + 3;
                    stack.extend(
                        graph.cards[i]
                            .children
                            .iter()
                            .rev()
                            .map(|c| (*c, depth + 1)),
                    );
                }
            }
            root_x += columns * spacing;
        }
        for card in &graph.cards {
            graph.width = graph.width.max(card.position.0 + graph.card_size.0);
            graph.height = graph.height.max(card.position.1 + graph.card_size.1);
        }
        graph
    }

    pub fn rows(&self) -> Vec<Row> {
        self.cards
            .iter()
            .map(|c| Row {
                id: c.id.clone(),
                label: format!(
                    "{} [{}] · {} tasks · {} running",
                    c.label, c.role, c.counts.tasks, c.counts.running
                ),
            })
            .collect()
    }

    pub fn navigate(&self, selected: Option<&str>, direction: KeyCode) -> Option<String> {
        let current = self
            .cards
            .iter()
            .find(|c| Some(c.id.as_str()) == selected)?;
        if direction == KeyCode::Up {
            return current.parent.clone();
        }
        if direction == KeyCode::Down {
            return current.children.first().map(|i| self.cards[*i].id.clone());
        }
        let right = direction == KeyCode::Right;
        let position = (current.position.0, current.position.1);
        let mut siblings: Vec<_> = self
            .cards
            .iter()
            .filter(|c| {
                c.parent == current.parent
                    && if right {
                        (c.position.0, c.position.1) > position
                    } else {
                        (c.position.0, c.position.1) < position
                    }
            })
            .collect();
        siblings.sort_by_key(|c| (c.position.0, c.position.1));
        if right {
            siblings.first()
        } else {
            siblings.last()
        }
        .map(|c| c.id.clone())
    }

    pub fn render(
        &mut self,
        settings: &mut Settings,
        selected: Option<&str>,
        collapsed: &BTreeSet<String>,
        area: Rect,
        buffer: &mut Buffer,
    ) {
        let filter = ["All", "Running", "Issues", "Review"][usize::from(settings.filter % 4)];
        let block = panel(
            format!(
                "{} · {} · {filter} · Ctrl-F fit · / search",
                if self.goals {
                    "Goal dependencies"
                } else {
                    "Hierarchy tree"
                },
                self.cards.len()
            ),
            true,
        );
        let inner = block.inner(area);
        block.render(area, buffer);
        self.hits.clear();
        self.toggle_hits.clear();
        if inner.width == 0 || inner.height == 0 {
            return;
        }
        if self.cards.is_empty() {
            Paragraph::new("No matching nodes · Ctrl-N filter · Ctrl-T whole organization\nU creates an idle agent; execution needs authorization.").render(inner,buffer);
            return;
        }
        if settings.centered.as_deref() != selected || settings.size != (inner.width, inner.height)
        {
            if let Some(c) = self.cards.iter().find(|c| Some(c.id.as_str()) == selected) {
                settings.pan = (
                    c.position.0.saturating_sub(
                        u32::from(inner.width) / 2
                            - self.card_size.0.min(u32::from(inner.width)) / 2,
                    ),
                    c.position.1.saturating_sub(u32::from(inner.height) / 3),
                );
            }
            settings.centered = selected.map(str::to_owned);
            settings.size = (inner.width, inner.height);
        }
        if settings.overview {
            for c in &self.cards {
                let x = inner.x
                    + ((u64::from(c.position.0) * u64::from(inner.width.saturating_sub(1)))
                        / u64::from(self.width.max(1))) as u16;
                let y = inner.y
                    + ((u64::from(c.position.1) * u64::from(inner.height.saturating_sub(1)))
                        / u64::from(self.height.max(1))) as u16;
                buffer[(x, y)].set_symbol("·").set_fg(state_color(c.state));
            }
            if let Some(c) = self.cards.iter().find(|c| Some(c.id.as_str()) == selected) {
                let x = inner.x
                    + ((u64::from(c.position.0) * u64::from(inner.width.saturating_sub(1)))
                        / u64::from(self.width.max(1))) as u16;
                let y = inner.y
                    + ((u64::from(c.position.1) * u64::from(inner.height.saturating_sub(1)))
                        / u64::from(self.height.max(1))) as u16;
                buffer[(x, y)]
                    .set_symbol("◎")
                    .set_fg(foreground(view::BLUE));
            }
            Paragraph::new("Overview · dots may share cells · arrows select · Enter opens detail")
                .render(
                    Rect::new(inner.x, inner.bottom().saturating_sub(1), inner.width, 1),
                    buffer,
                );
            return;
        }
        settings.pan.0 = settings
            .pan
            .0
            .min(self.width.saturating_sub(u32::from(inner.width)));
        settings.pan.1 = settings
            .pan
            .1
            .min(self.height.saturating_sub(u32::from(inner.height)));
        let (px, py) = settings.pan;
        let mut line = |x1: u32, y1: u32, x2: u32, y2: u32| {
            if x1 == x2 && x1 >= px && x1 < px + u32::from(inner.width) {
                for y in y1.max(py)..=y2.min(py + u32::from(inner.height) - 1) {
                    buffer[(inner.x + (x1 - px) as u16, inner.y + (y - py) as u16)]
                        .set_symbol("│")
                        .set_fg(foreground(view::BLUE));
                }
            } else if y1 == y2 && y1 >= py && y1 < py + u32::from(inner.height) {
                for x in x1.max(px)..=x2.min(px + u32::from(inner.width) - 1) {
                    buffer[(inner.x + (x - px) as u16, inner.y + (y1 - py) as u16)]
                        .set_symbol("─")
                        .set_fg(foreground(view::BLUE));
                }
            }
        };
        for c in &self.cards {
            if c.children.is_empty() {
                continue;
            }
            if c.parent.is_none() {
                let x = c.position.0 + self.card_size.0 / 2;
                let bottom = c.position.1 + self.card_size.1;
                let fork = bottom + 1;
                line(x, bottom, x, fork);
                let first = &self.cards[c.children[0]];
                let last = &self.cards[*c.children.last().unwrap_or(&c.children[0])];
                line(
                    first.position.0 + self.card_size.0 / 2,
                    fork,
                    last.position.0 + self.card_size.0 / 2,
                    fork,
                );
                for i in &c.children {
                    let child = &self.cards[*i];
                    let x = child.position.0 + self.card_size.0 / 2;
                    line(x, fork, x, child.position.1.saturating_sub(1));
                }
            } else {
                let spine = c.position.0.saturating_sub(1);
                let start = c.position.1 + 1;
                let last = self.cards[*c.children.last().unwrap_or(&c.children[0])]
                    .position
                    .1
                    + 1;
                line(spine, start, c.position.0, start);
                line(spine, start, spine, last);
                for i in &c.children {
                    let child = &self.cards[*i];
                    line(
                        spine,
                        child.position.1 + 1,
                        child.position.0,
                        child.position.1 + 1,
                    );
                }
            }
        }
        for c in &self.cards {
            let (x, y) = c.position;
            let (w, h) = self.card_size;
            if x + w <= px
                || y + h <= py
                || x >= px + u32::from(inner.width)
                || y >= py + u32::from(inner.height)
            {
                continue;
            }
            // Paint only visible cards, using a bounded per-card buffer to preserve clipped borders/text.
            let local = Rect::new(0, 0, w as u16, h as u16);
            let mut tile = Buffer::empty(local);
            let is_selected = Some(c.id.as_str()) == selected;
            let title = format!(
                "{} {}",
                if c.child_count > 0 && collapsed.contains(&c.id) {
                    "▸"
                } else if !c.children.is_empty() {
                    "▾"
                } else {
                    "·"
                },
                c.label
            );
            let text = if self.goals && settings.compact {
                format!(
                    "{} · {}/{} accepted",
                    c.state, c.counts.accepted, c.counts.tasks
                )
            } else if self.goals {
                format!(
                    "{}\n{} / {} tasks accepted\n{}",
                    c.role, c.counts.accepted, c.counts.tasks, c.state
                )
            } else if settings.compact {
                format!("● {} · {} tasks", c.state, c.counts.tasks)
            } else {
                format!(
                    "{}\n● {} · {} active\n{} agents · {} tasks · {} issues",
                    c.role,
                    c.state,
                    c.counts.running,
                    c.counts.agents,
                    c.counts.tasks,
                    c.counts.issues
                )
            };
            Paragraph::new(text)
                .style(Style::default().fg(state_color(c.state)))
                .block(panel(title, is_selected))
                .render(local, &mut tile);
            let x0 = x.max(px);
            let y0 = y.max(py);
            let xend = (x + w).min(px + u32::from(inner.width));
            let yend = (y + h).min(py + u32::from(inner.height));
            for yy in y0..yend {
                for xx in x0..xend {
                    buffer[(inner.x + (xx - px) as u16, inner.y + (yy - py) as u16)] =
                        tile[((xx - x) as u16, (yy - y) as u16)].clone();
                }
            }
            self.hits.push((
                Rect::new(
                    inner.x + (x0 - px) as u16,
                    inner.y + (y0 - py) as u16,
                    (xend - x0) as u16,
                    (yend - y0) as u16,
                ),
                c.id.clone(),
            ));
            if x >= px && y >= py && c.child_count > 0 {
                let toggle = Rect::new(
                    inner.x + (x - px) as u16 + 1,
                    inner.y + (y - py) as u16,
                    3,
                    1,
                )
                .intersection(inner);
                if !toggle.is_empty() {
                    self.toggle_hits.push((toggle, c.id.clone()));
                }
            }
        }
    }
}

impl Dashboard {
    pub(super) fn node_subtree(&self, root: &str) -> BTreeSet<String> {
        let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for record in &self.snapshot.records {
            if let OrganizationRecord::Node(n) = record
                && let Some(parent) = n.parent_id.as_deref()
            {
                children.entry(parent).or_default().push(&n.id);
            }
        }
        let mut found = BTreeSet::new();
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if found.insert(id.to_owned())
                && let Some(children) = children.get(id)
            {
                pending.extend(children);
            }
        }
        found
    }

    pub(super) fn node_tab_text(&self) -> Option<String> {
        let OrganizationRecord::Node(node) = self.record()? else {
            return None;
        };
        if self.confirmation.is_some() || self.preferences.graph.inspector_tab.is_multiple_of(4) {
            return None;
        }
        let members = self.node_subtree(&node.id);
        let mut text = format!(
            "{}\n{}\nPrimary hierarchy · loaded records\n\n",
            node.label, node.id
        );
        match self.preferences.graph.inspector_tab % 4 {
            1 => {
                for record in &self.snapshot.records {
                    if let OrganizationRecord::Node(n) = record
                        && members.contains(&n.id)
                    {
                        text.push_str(&format!(
                            "{} · {:?} · {}\n{}\n",
                            n.label,
                            n.kind,
                            if n.enabled { "Enabled" } else { "Disabled" },
                            n.id
                        ));
                    }
                }
            }
            2 => {
                for workflow in &self.snapshot.workflows {
                    match workflow {
                        OrganizationWorkflowView::Template { template }
                            if template.items.iter().any(|s| members.contains(&s.owner_id)) =>
                        {
                            text.push_str(&format!(
                                "Template: {}\n{} · {} steps\n",
                                template.label,
                                template.id,
                                template.items.len()
                            ))
                        }
                        OrganizationWorkflowView::Run { run, tasks }
                            if tasks.iter().any(|t| members.contains(&t.owner_id)) =>
                        {
                            text.push_str(&format!("Run: {} · {:?}\n", run.id, run.state));
                            for t in tasks.iter().filter(|t| members.contains(&t.owner_id)) {
                                text.push_str(&format!("{} · {}\n", t.id, t.state));
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {
                text.push_str("Captured task evidence (owner at attempt claim)\n");
                if let Some(report) = &self.snapshot.report {
                    let references: BTreeSet<_> = self
                        .snapshot
                        .records
                        .iter()
                        .filter_map(|r| match r {
                            OrganizationRecord::Task(t) if members.contains(&t.owner_id) => {
                                Some(t.id.as_str())
                            }
                            _ => None,
                        })
                        .chain(members.iter().map(String::as_str))
                        .collect();
                    text.push_str(
                        "Recent owner audit · current subtree references
",
                    );
                    if let Some(events) = &report.audit_events {
                        for event in events.iter().filter(|e| {
                            e.reference
                                .as_deref()
                                .is_some_and(|id| references.contains(id))
                        }) {
                            text.push_str(&format!(
                                "#{} · {}
{} · {} · {}
",
                                event.sequence,
                                utc_time(event.created_at),
                                event.actor,
                                event.operation,
                                event.reference.as_deref().unwrap_or("Unknown reference")
                            ));
                        }
                        text.push_str("Global audit window is limited to 200 events; arbitrary payload bodies are excluded.

");
                    } else {
                        text.push_str(
                            "Audit window unavailable from this report.

",
                        );
                    }
                    if let Some(cards) = &report.scorecards {
                        for card in cards
                            .entries
                            .iter()
                            .filter(|c| c.owner_id.as_ref().is_some_and(|id| members.contains(id)))
                        {
                            text.push_str(&format!("{} · {}\n{} attempts · {} accepted · {} failed · {} uncertain\n{} checks passed / {} failed\n{} reported tokens · {}/{} requests reported\n\n",card.owner_id.as_deref().unwrap_or("Unknown"),card.model.as_deref().unwrap_or("Unknown model"),card.attempts,card.accepted_results,card.failed_attempts,card.uncertain_attempts,card.checks_passed,card.checks_failed,card.reported_tokens,card.reported_model_requests,card.model_requests));
                        }
                        if cards.truncated {
                            text.push_str(
                                "Evidence groups truncated; inspect paginated owner history.\n",
                            );
                        }
                    } else {
                        text.push_str("Captured evidence unavailable from this daemon.\n");
                    }
                    for task in report
                        .tasks
                        .iter()
                        .filter(|t| members.contains(&t.owner_id))
                    {
                        if let Some(blocker) = &task.blocker {
                            text.push_str(&format!(
                                "{} · {}\n{blocker}\n",
                                task.task_id, task.state
                            ));
                        }
                    }
                } else {
                    text.push_str("Owner report unavailable; refresh.\n");
                }
            }
        }
        Some(clean(&text))
    }

    pub(super) fn diagram_active(&self) -> bool {
        ((self.preferences.preset == Preset::Operations && self.projection == Projection::Default)
            || self.projection == Projection::Goals)
            && !self.preferences.graph.list
    }

    pub(super) fn graph_key(&mut self, key: KeyEvent) -> bool {
        if let KeyCode::F(n @ 13..=22) = key.code {
            let (c, modifiers) = [
                ('f', KeyModifiers::CONTROL),
                ('b', KeyModifiers::CONTROL),
                ('o', KeyModifiers::CONTROL),
                ('t', KeyModifiers::CONTROL),
                ('n', KeyModifiers::CONTROL),
                ('v', KeyModifiers::CONTROL),
                ('e', KeyModifiers::CONTROL),
                ('l', KeyModifiers::CONTROL),
                ('h', KeyModifiers::ALT),
                ('i', KeyModifiers::ALT),
            ][usize::from(n - 13)];
            return self.graph_key(KeyEvent::new(KeyCode::Char(c), modifiers));
        }
        if self.preferences.preset == Preset::Operations
            && key.code == KeyCode::Char('t')
            && key.modifiers == KeyModifiers::CONTROL
        {
            self.projection = Projection::Default;
            self.preferences.graph.focus = None;
            self.preferences.graph.filter = 0;
            self.search.clear();
            self.preferences.graph.centered = None;
            self.rebuild();
            self.inspect();
            self.save();
            return true;
        }
        if self.preferences.preset == Preset::Operations
            && key.modifiers == KeyModifiers::ALT
            && matches!(key.code, KeyCode::Char('h' | 'i'))
        {
            self.projection = if key.code == KeyCode::Char('h') {
                Projection::Health
            } else {
                Projection::Incidents
            };
            self.search.clear();
            self.selected = None;
            self.rebuild();
            self.inspect();
            return true;
        }
        if (self.preferences.preset != Preset::Operations || self.projection != Projection::Default)
            && self.projection != Projection::Goals
        {
            return false;
        }
        if key.code == KeyCode::Enter && key.modifiers == KeyModifiers::SHIFT {
            if let Some(id) = &self.selected {
                let members = self.node_subtree(id);
                self.collapsed.retain(|id| !members.contains(id));
                self.rebuild();
                self.inspect();
            }
            return true;
        }
        if key.modifiers == KeyModifiers::ALT && matches!(key.code, KeyCode::Char('1'..='4')) {
            if let KeyCode::Char(n) = key.code {
                self.preferences.graph.inspector_tab = n as u8 - b'1';
                self.inspect();
                self.save();
            }
            return true;
        }
        if key.modifiers == KeyModifiers::CONTROL {
            let s = &mut self.preferences.graph;
            match key.code {
                KeyCode::Left => s.pan.0 = s.pan.0.saturating_sub(12),
                KeyCode::Right => s.pan.0 = s.pan.0.saturating_add(12),
                KeyCode::Up => s.pan.1 = s.pan.1.saturating_sub(5),
                KeyCode::Down => s.pan.1 = s.pan.1.saturating_add(5),
                KeyCode::Char('b') => {
                    s.overview = false;
                    s.compact = !s.compact;
                    s.centered = None;
                }
                KeyCode::Char('f') => {
                    s.overview = true;
                    s.centered = None;
                    s.pan = (0, 0);
                }
                KeyCode::Char('n') => s.filter = (s.filter + 1) % 4,
                KeyCode::Char('o') => {
                    s.focus = self.selected.clone();
                    s.centered = None;
                }
                KeyCode::Char('t') => {
                    s.focus = None;
                    s.filter = 0;
                    self.search.clear();
                    s.centered = None;
                }
                KeyCode::Char('e') => self.collapsed.clear(),
                KeyCode::Char('l') => {
                    self.collapsed = self.graph.cards.iter().map(|c| c.id.clone()).collect()
                }
                KeyCode::Char('v') => s.list = !s.list,
                _ => return false,
            }
            if matches!(
                key.code,
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            ) {
                self.save();
                return true;
            }
            self.rebuild();
            self.inspect();
            self.save();
            return true;
        }
        if self.diagram_active() && key.code == KeyCode::Enter && self.preferences.graph.overview {
            self.preferences.graph.overview = false;
            self.preferences.graph.centered = None;
            self.save();
            return true;
        }
        if self.diagram_active()
            && !self.pane().inspector_focus
            && key.modifiers.is_empty()
            && matches!(
                key.code,
                KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
            )
        {
            if let Some(id) = self.graph.navigate(self.selected.as_deref(), key.code) {
                self.selected = Some(id);
                self.inspect();
            }
            return true;
        }
        false
    }
}

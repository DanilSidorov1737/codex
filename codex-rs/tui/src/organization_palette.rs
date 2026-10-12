//! Discovery routes to the same owner handlers; it never carries execution authority.
use super::*;

pub(super) struct Palette {
    pub query: String,
    pub selected: usize,
}

pub(super) struct Action {
    pub title: &'static str,
    pub shortcut: &'static str,
    pub code: KeyCode,
    pub context: &'static str,
}

const ACTIONS: &[Action] = &[
    Action {
        title: "Console",
        shortcut: "F1",
        code: KeyCode::F(1),
        context: "layout",
    },
    Action {
        title: "Workspace / official Neovim",
        shortcut: "F2",
        code: KeyCode::F(2),
        context: "layout",
    },
    Action {
        title: "Projects",
        shortcut: "F3",
        code: KeyCode::F(3),
        context: "layout",
    },
    Action {
        title: "Operations / hierarchy",
        shortcut: "F4",
        code: KeyCode::F(4),
        context: "layout",
    },
    Action {
        title: "Review / action inbox",
        shortcut: "F5",
        code: KeyCode::F(5),
        context: "layout",
    },
    Action {
        title: "Configure actual resource pressure thresholds",
        shortcut: "Commands",
        code: KeyCode::F(39),
        context: "owner disk/memory admission thresholds; unknown telemetry fails closed",
    },
    Action {
        title: "First-launch setup wizard",
        shortcut: "Commands",
        code: KeyCode::F(38),
        context: "empty organization; configuration only, execution stays paused",
    },
    Action {
        title: "New project",
        shortcut: "N",
        code: KeyCode::Char('N'),
        context: "owner proposal",
    },
    Action {
        title: "Inspect exact uncertain request/effects",
        shortcut: "Commands",
        code: KeyCode::F(40),
        context: "select task; record evidence without clearing charges, reservations or replay fences",
    },
    Action {
        title: "Save / edit execution profile",
        shortcut: "Commands",
        code: KeyCode::F(41),
        context: "optional select existing profile; versioned models/paths/checks/budgets/approval policy",
    },
    Action {
        title: "Authorize task from exact profile",
        shortcut: "Commands",
        code: KeyCode::F(42),
        context: "select task; separate reviewed grant, stale profile refuses",
    },
    Action {
        title: "Propose team creation / restructure / retirement",
        shortcut: "@",
        code: KeyCode::Char('@'),
        context: "select node; atomic definitions only; % reviews exact saved delta",
    },
    Action {
        title: "Draft finite local project discovery",
        shortcut: "?",
        code: KeyCode::Char('?'),
        context: "select project; saving creates no grant; exact paths/budgets require separate authorization",
    },
    Action {
        title: "Configure bounded safe startup recovery",
        shortcut: "^",
        code: KeyCode::Char('^'),
        context: "select authorized root task; no model/tool effects; original limits",
    },
    Action {
        title: "Compare instruction or workflow evaluation / propose adoption",
        shortcut: "$",
        code: KeyCode::Char('$'),
        context: "select agent or candidate workflow; accepted hash-pinned evaluations; % separately adopts",
    },
    Action {
        title: "Share reviewed project knowledge",
        shortcut: "S",
        code: KeyCode::Char('S'),
        context: "memory search exact source; target revision/body reviewed; private/team sources refused",
    },
    Action {
        title: "Goal supervision policy pause / resume / revoke",
        shortcut: "*",
        code: KeyCode::Char('*'),
        context: "select finite goal policy or its series; exact confirmation",
    },
    Action {
        title: "Propose checkpoint replan",
        shortcut: "&",
        code: KeyCode::Char('&'),
        context: "select blocked task; original grants and accounting",
    },
    Action {
        title: "Apply exact replan proposal",
        shortcut: "%",
        code: KeyCode::Char('%'),
        context: "select pending owner control; atomic delta confirmation",
    },
    Action {
        title: "Draft finite agent discussion",
        shortcut: "+",
        code: KeyCode::Char('+'),
        context: "arbitrary participants; contributions, conclusions, dissent and proposals",
    },
    Action {
        title: "Review knowledge retirement / restoration",
        shortcut: "-",
        code: KeyCode::Char('-'),
        context: "search active notes or select a knowledge control; preserve scope and history",
    },
    Action {
        title: "Organization lifetime ceilings",
        shortcut: "F10",
        code: KeyCode::F(10),
        context: "owner control; recorded historical usage; exact confirmation",
    },
    Action {
        title: "Emergency organization pause / resume",
        shortcut: "F12",
        code: KeyCode::F(12),
        context: "preserves grants, expiry and usage; exact confirmation",
    },
    Action {
        title: "Browse owner controls",
        shortcut: "F11",
        code: KeyCode::F(11),
        context: "view",
    },
    Action {
        title: "Project pause / complete / archive / reopen",
        shortcut: "F",
        code: KeyCode::Char('F'),
        context: "select project or task; accepted criterion evidence",
    },
    Action {
        title: "Task priority / fair dispatch",
        shortcut: "!",
        code: KeyCode::Char('!'),
        context: "select task; bounded priority and waiting age",
    },
    Action {
        title: "Fit hierarchy overview",
        shortcut: "Ctrl-F",
        code: KeyCode::F(13),
        context: "Operations; read-only view",
    },
    Action {
        title: "Compact hierarchy cards",
        shortcut: "Ctrl-B",
        code: KeyCode::F(14),
        context: "Operations; read-only view",
    },
    Action {
        title: "Focus selected subtree",
        shortcut: "Ctrl-O",
        code: KeyCode::F(15),
        context: "Operations; read-only view",
    },
    Action {
        title: "Whole organization tree",
        shortcut: "Ctrl-T",
        code: KeyCode::F(16),
        context: "Operations; read-only view",
    },
    Action {
        title: "Filter running / issues / review",
        shortcut: "Ctrl-N",
        code: KeyCode::F(17),
        context: "Operations; read-only view",
    },
    Action {
        title: "Hierarchy diagram / list",
        shortcut: "Ctrl-V",
        code: KeyCode::F(18),
        context: "Operations; read-only view",
    },
    Action {
        title: "Expand all hierarchy branches",
        shortcut: "Ctrl-E",
        code: KeyCode::F(19),
        context: "Operations; read-only view",
    },
    Action {
        title: "Collapse all hierarchy branches",
        shortcut: "Ctrl-L",
        code: KeyCode::F(20),
        context: "Operations; read-only view",
    },
    Action {
        title: "Health and serving-process resources",
        shortcut: "Alt-H",
        code: KeyCode::F(21),
        context: "Operations; read-only view",
    },
    Action {
        title: "Recorded incidents and reconciliation",
        shortcut: "Alt-I",
        code: KeyCode::F(22),
        context: "Operations; read-only view",
    },
    Action {
        title: "New goal",
        shortcut: "G",
        code: KeyCode::Char('G'),
        context: "owner proposal",
    },
    Action {
        title: "New task",
        shortcut: "T",
        code: KeyCode::Char('T'),
        context: "owner proposal",
    },
    Action {
        title: "New agent",
        shortcut: "U",
        code: KeyCode::Char('U'),
        context: "owner form",
    },
    Action {
        title: "New group",
        shortcut: "B",
        code: KeyCode::Char('B'),
        context: "owner form",
    },
    Action {
        title: "Edit node or goal / reparent / retire node",
        shortcut: "E",
        code: KeyCode::Char('E'),
        context: "select node or goal",
    },
    Action {
        title: "Create role version",
        shortcut: "L",
        code: KeyCode::Char('L'),
        context: "select node or role",
    },
    Action {
        title: "Edit memberships",
        shortcut: "I",
        code: KeyCode::Char('I'),
        context: "select node",
    },
    Action {
        title: "Plan goal / import accepted plan",
        shortcut: "P",
        code: KeyCode::Char('P'),
        context: "select goal or accepted planner",
    },
    Action {
        title: "Save / edit workflow",
        shortcut: "W",
        code: KeyCode::Char('W'),
        context: "select project or template",
    },
    Action {
        title: "Templates / workflow runs",
        shortcut: "w",
        code: KeyCode::Char('w'),
        context: "view",
    },
    Action {
        title: "Authorize task / preview workflow",
        shortcut: "s",
        code: KeyCode::Char('s'),
        context: "select task or template; exact confirmation",
    },
    Action {
        title: "Create recurring task",
        shortcut: "R",
        code: KeyCode::Char('R'),
        context: "select task; finite scope",
    },
    Action {
        title: "Recurring workflow / accepted-result trigger",
        shortcut: "Y",
        code: KeyCode::Char('Y'),
        context: "select template; finite preview and confirmation",
    },
    Action {
        title: "Schedules",
        shortcut: "S",
        code: KeyCode::Char('S'),
        context: "view",
    },
    Action {
        title: "Shared project / workflow limits",
        shortcut: "b",
        code: KeyCode::Char('b'),
        context: "select project, task, run or limit",
    },
    Action {
        title: "Approve exact action",
        shortcut: "a",
        code: KeyCode::Char('a'),
        context: "select waiting approval; confirmation",
    },
    Action {
        title: "Reject exact action",
        shortcut: "r",
        code: KeyCode::Char('r'),
        context: "select waiting approval; confirmation",
    },
    Action {
        title: "Accept checked result",
        shortcut: "v",
        code: KeyCode::Char('v'),
        context: "select reviewable task; confirmation",
    },
    Action {
        title: "Request changes",
        shortcut: "c",
        code: KeyCode::Char('c'),
        context: "select task; reviewed instructions",
    },
    Action {
        title: "Select files to Apply",
        shortcut: "A",
        code: KeyCode::Char('A'),
        context: "select accepted task; exact confirmation",
    },
    Action {
        title: "Pause",
        shortcut: "p",
        code: KeyCode::Char('p'),
        context: "select task, run or series; confirmation",
    },
    Action {
        title: "Resume",
        shortcut: "u",
        code: KeyCode::Char('u'),
        context: "select paused work; safe continuation",
    },
    Action {
        title: "Cancel",
        shortcut: "x",
        code: KeyCode::Char('x'),
        context: "select work; confirmation",
    },
    Action {
        title: "Resolve blocker / handoff",
        shortcut: "H",
        code: KeyCode::Char('H'),
        context: "select blocked task",
    },
    Action {
        title: "Search project knowledge",
        shortcut: "m",
        code: KeyCode::Char('m'),
        context: "select project or task",
    },
    Action {
        title: "Search agent memory",
        shortcut: "M",
        code: KeyCode::Char('M'),
        context: "select task",
    },
    Action {
        title: "Search team knowledge",
        shortcut: "J",
        code: KeyCode::Char('J'),
        context: "select task",
    },
    Action {
        title: "Selected evidence summary",
        shortcut: "z",
        code: KeyCode::Char('z'),
        context: "select project, agent or run",
    },
    Action {
        title: "Organization evidence summary / agent and model scorecards",
        shortcut: "Z",
        code: KeyCode::Char('Z'),
        context: "read-only",
    },
    Action {
        title: "Capability inventory",
        shortcut: "C",
        code: KeyCode::Char('C'),
        context: "read-only; no probes",
    },
    Action {
        title: "Refresh canonical state",
        shortcut: "f",
        code: KeyCode::Char('f'),
        context: "read-only",
    },
    Action {
        title: "Goal tree / milestone evidence",
        shortcut: "g",
        code: KeyCode::Char('g'),
        context: "view; owner reviews success criteria",
    },
    Action {
        title: "Reset current layout panes",
        shortcut: "0",
        code: KeyCode::Char('0'),
        context: "local preferences",
    },
    Action {
        title: "Organization presets",
        shortcut: "F9",
        code: KeyCode::F(9),
        context: "view; captured definition subtrees",
    },
    Action {
        title: "Save organization preset",
        shortcut: "D",
        code: KeyCode::Char('D'),
        context: "select node; definitions only",
    },
    Action {
        title: "Clone organization preset",
        shortcut: "V",
        code: KeyCode::Char('V'),
        context: "select preset; fresh identities; exact owner confirmation",
    },
    Action {
        title: "Save private Neovim session checkpoint",
        shortcut: "F7",
        code: KeyCode::F(7),
        context: "Workspace host focus; original files/splits/cursors/unsaved text",
    },
    Action {
        title: "Open selected task file in Neovim",
        shortcut: "palette",
        code: KeyCode::F(23),
        context: "any layout; declared project path and dirty-buffer guards",
    },
    Action {
        title: "Restore native Neovim session",
        shortcut: "F8",
        code: KeyCode::F(8),
        context: "Workspace host focus; explicit choice for unsaved recovery",
    },
    Action {
        title: "Select exact accepted hunks to Apply",
        shortcut: "h",
        code: KeyCode::Char('h'),
        context: "accepted result; one durable application",
    },
    Action {
        title: "Propose accepted artifact to native buffer",
        shortcut: "B",
        code: KeyCode::Char('B'),
        context: "Workspace host focus; pinned clean original; native undo; explicit save",
    },
    Action {
        title: "Load more records",
        shortcut: "Commands",
        code: KeyCode::F(25),
        context: "additional bounded pages; no execution",
    },
    Action {
        title: "Named views: save / restore / delete",
        shortcut: "Commands",
        code: KeyCode::F(24),
        context: "private layout, filter and focused branch",
    },
    Action {
        title: "Workspace mission / priorities / constraints",
        shortcut: "Commands",
        code: KeyCode::F(26),
        context: "owner-versioned intent; native grants remain",
    },
    Action {
        title: "Create optional editable playbook",
        shortcut: "Commands",
        code: KeyCode::F(27),
        context: "select project; no mandatory roles or execution",
    },
    Action {
        title: "Review goal criterion evidence",
        shortcut: "Commands",
        code: KeyCode::F(28),
        context: "select goal; exact accepted results and owner reasons",
    },
    Action {
        title: "Route local blocker assistance",
        shortcut: "Commands",
        code: KeyCode::F(29),
        context: "select task needing help; no new grants",
    },
    Action {
        title: "Capture accepted-result lesson",
        shortcut: "Commands",
        code: KeyCode::F(30),
        context: "select accepted task; private reviewable proposal",
    },
    Action {
        title: "Open native owner terminal beside Neovim",
        shortcut: "Commands",
        code: KeyCode::F(33),
        context: "select project/task; actual owner shell; separate from agent permissions",
    },
    Action {
        title: "Browse / search captured artifact library",
        shortcut: "Commands",
        code: KeyCode::F(31),
        context: "select project/task; bounded current results with provenance",
    },
    Action {
        title: "Preview captured text / Markdown / CSV",
        shortcut: "Commands",
        code: KeyCode::F(32),
        context: "select result task; n cycles outputs in Review; verified literal text",
    },
    Action {
        title: "Open captured native test panel",
        shortcut: "Commands",
        code: KeyCode::F(34),
        context: "select checked task; recorded argv/output/exit, no execution",
    },
    Action {
        title: "Review base / owner / agent in real Neovim",
        shortcut: "Commands",
        code: KeyCode::F(35),
        context: "select result task; protected three-way copies; owner resolves original explicitly",
    },
    Action {
        title: "Propose role version rollback",
        shortcut: "Commands",
        code: KeyCode::F(36),
        context: "select agent/role; new immutable version; % applies exact saved proposal",
    },
    Action {
        title: "Propose captured workflow rollback",
        shortcut: "Commands",
        code: KeyCode::F(37),
        context: "select template; source from prior run; history/counters/grants retained",
    },
];

impl Palette {
    pub fn paste(&mut self, value: &str) {
        for c in value.chars().filter(|c| !c.is_control()) {
            if self.query.len() + c.len_utf8() > 256 {
                break;
            }
            self.query.push(c);
        }
        self.selected = 0;
    }
    pub fn new() -> Self {
        Self {
            query: String::new(),
            selected: 0,
        }
    }
    pub fn actions(&self) -> Vec<&'static Action> {
        let query = self.query.to_lowercase();
        let mut actions: Vec<_> = ACTIONS
            .iter()
            .filter(|action| {
                query.split_whitespace().all(|word| {
                    format!("{} {} {}", action.title, action.shortcut, action.context)
                        .to_lowercase()
                        .contains(word)
                })
            })
            .collect();
        actions.sort_by_key(|action| {
            std::cmp::Reverse(
                query
                    .split_whitespace()
                    .filter(|word| action.title.to_lowercase().contains(word))
                    .count(),
            )
        });
        actions
    }
    pub fn key(&mut self, key: KeyEvent) -> Option<KeyCode> {
        match key.code {
            KeyCode::Enter => return self.actions().get(self.selected).map(|action| action.code),
            KeyCode::Up => self.selected = self.selected.saturating_sub(1),
            KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.actions().len().saturating_sub(1))
            }
            KeyCode::Backspace => {
                self.query.pop();
                self.selected = 0;
            }
            KeyCode::Char(c)
                if !c.is_control()
                    && (key.modifiers.is_empty()
                        || key.modifiers == crossterm::event::KeyModifiers::SHIFT)
                    && self.query.len() + c.len_utf8() <= 256 =>
            {
                self.query.push(c);
                self.selected = 0;
            }
            _ => {}
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn search_matches_context_and_dispatches_only_on_enter() {
        let mut palette = Palette::new();
        for c in "files accepted".chars() {
            assert_eq!(palette.key(KeyCode::Char(c).into()), None);
        }
        assert_eq!(palette.actions().len(), 1);
        assert_eq!(palette.key(KeyCode::Enter.into()), Some(KeyCode::Char('A')));
        palette.query = "unrecognized command".into();
        assert_eq!(palette.key(KeyCode::Enter.into()), None);
        palette.query.clear();
        palette.paste("Apply\naccepted\u{1b}");
        assert_eq!(palette.query, "Applyaccepted");
    }
    #[test]
    fn navigation_and_unicode_query_remain_bounded() {
        let mut palette = Palette::new();
        palette.key(KeyCode::Down.into());
        assert_eq!(palette.key(KeyCode::Enter.into()), Some(KeyCode::F(2)));
        for _ in 0..200 {
            palette.key(KeyCode::Char('é').into());
        }
        assert_eq!(palette.query.len(), 256);
        palette.key(KeyCode::Backspace.into());
        assert_eq!(palette.query.len(), 254);
        assert_eq!(palette.selected, 0);
    }
}

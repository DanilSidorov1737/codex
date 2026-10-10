# Organization layouts

Status: B4 locally qualified with real CLI/daemon/TUI interaction.
Actual Neovim and reviewed file application subsequently passed B5 qualification;
see [Workspace controls](neovim-workspace.md). Trusted owner quote estimates are
described in [resource settings](resource-limits.md).

Use `/layout [console|workspace|projects|operations|review]`. Console retains the
existing chat/composer. Other presets share the existing alternate-screen TUI,
Ratatui widgets and typed owner app-server connection. Alternate-screen-disabled
launches report the requirement rather than silently taking terminal ownership.

- F1 returns to Console; F2–F5 select Workspace, Projects, Operations and Review.
  Numeric 1–5 are alternatives while the dashboard has focus.
- Up/down select; optional j/k equivalents are toggled by K. Tab changes focus
  between navigation and inspector. PageUp/PageDown scroll the inspector.
- Operations is a primary hierarchy tree. Left collapses, right expands, Enter
  toggles. `/` searches labels/IDs, including inside collapsed branches.
- `i` shows/hides the inspector; `[`/`]` change the split ratio. Narrow terminals
  stack navigation and inspector. Each preset retains its own pane preferences.
- `f` refreshes real state. One bounded loader also refreshes every three seconds;
  it performs owner metadata requests, never model requests or per-node sessions.
- `p` toggles a selected node's enabled flag through revision-checked owner
  mutation, or pauses a selected task. `x` cancels a task.
- Review/task inspection: `a` approves the first pending exact action, `r` rejects
  it, `v` accepts the exact current result digest, and `c` requests changes with
  entered instructions. Decisions require an explicit `y` against the displayed
  captured payload; Escape cancels; PageUp/PageDown scroll the frozen confirmation. Request-changes
  drafts remain bound to their captured task/revision. Backend policy/revision/hash fences remain
  authoritative. A timeout reports an unknown outcome; refresh before retrying.
- `q`/Escape return to Console while remembering the last dashboard preset for
  the next launch. F1 explicitly selects Console. `/layout` can reopen the view.

Preferences live in `<CODEX_HOME>/organization-layouts.json`, outside project
configuration. Private atomic writes reuse tempfile; Linux reads reject links,
nonregular files, unsafe ownership/permissions and oversized data. Invalid
preferences report an error and use defaults. Ratios are clamped to 25–75%.

Records retain stable selection across refresh. Task-detail responses belonging
to another selection cannot enable its decisions. Confirmation captures its
exact hash/revision even if newer state arrives. Terminal-control text is filtered
with the existing transcript sanitizer. Inspector text is cached on data changes;
related-record display is bounded to 128, and tree traversal is iterative with
bounded indentation. The owner loader caps each kind at 2,000 records and reports
overflow rather than silently pretending the load is complete.

Organization creation, role versioning, project linking and execution grants
remain available through the documented owner CLI. These layouts display existing
state; no synthetic progress, CPU utilization, dollar costs or test results are
invented. See [command guide](organization-commands.md).

Qualification must physically exercise all presets, hierarchy search/collapse,
real owner controls, exact approvals, narrow/wide terminals and preference reload.
Meaningful automated checks complement that evidence; snapshots alone cannot
qualify the interaction. See [MVP criteria](mvp.md).

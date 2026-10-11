# Organization layouts

Status: B4 locally qualified with real CLI/daemon/TUI interaction.
Actual Neovim and reviewed file application subsequently passed B5 qualification;
see [Workspace controls](neovim-workspace.md). Trusted owner quote estimates are
described in [resource settings](resource-limits.md).

Use `/layout [console|workspace|projects|operations|review]`. Console retains the
existing chat/composer. Other presets share the existing alternate-screen TUI,
Ratatui widgets and typed owner app-server connection. Alternate-screen-disabled
launches report the requirement rather than silently taking terminal ownership.

- While the dashboard has focus, F1 returns to Console and F2–F5 select
  Workspace, Projects, Operations and Review. Numeric 1–5 are alternatives.
  From Console, reopen with `/layout operations` (or another preset); Console's
  function keys retain their existing chat/search bindings.
- Up/down select; optional j/k equivalents are toggled by K. Tab changes focus
  between navigation and inspector. PageUp/PageDown scroll the inspector.
- Operations is a primary hierarchy tree. Left collapses, right expands, Enter
  toggles. `/` searches labels/IDs, including inside collapsed branches.
- `i` shows/hides the inspector; `[`/`]` change the split ratio. Narrow terminals
  stack navigation and inspector. Each preset retains its own pane preferences.
- `f` refreshes real state. One bounded loader also refreshes every three seconds;
  it performs owner metadata requests, never model requests or per-node sessions.
- Select a project/task in Projects and press `m` for project memory; select a
  task and press `M` for that owner's private memory. Workspace and Review task
  selections also support these keys with host focus. Operations' nodes do not
  select a project; use F3 first. Enter searches, empty Enter lists recent notes,
  `/` edits the query, `f` refreshes, arrows/PageUp/PageDown scroll, and Esc closes
  the memory view or query entry. Scope, source IDs/timestamps and match counts
  remain visible. Memory captures its selected scope and owns keyboard/paste/mouse
  routing while open; close it before normal layout/editor shortcuts resume.
  See [memory search](memory-search.md) for the literal query contract and evidence.
- `p` toggles a selected node's enabled flag through revision-checked owner
  mutation, or pauses a selected task. `x` cancels a task.
- Select a task without prerequisites and press `R` for a finite recurring
  schedule. The existing authorization fields specify each run's explicit scope;
  loaded grants prefill these fields. Add interval, first/end UTC epoch times,
  run count and aggregate limits, then F6 reviews and `y` confirms. `S` shows
  schedules inside the existing layouts; Operations defaults to its hierarchy
  tree and displays upcoming UTC starts below it. Projects also lists schedules.
  A selected schedule exposes `p` pause, `u` resume and `x` cancel, each with
  exact revision/confirmation. Its inspector shows next time, usage, skips and
  recent task IDs; select those tasks for their approvals, checks and review.
  With actual Neovim focused, use Ctrl-G before host controls. See
  [recurring schedules](recurring-schedules.md) for bounds and recovery behavior.
  The slice is qualified on `e06def499e`: 28 native TUI/official-Neovim tests,
  actual form/controls/background/restart/Follow, 36 rapid switches across three
  sizes and narrow counter scrolling passed. Switching layouts clears selection;
  explicitly select the occurrence in Workspace before `t` Follow. Current
  captures are in `/workspace/artifacts/cli-recurring-schedules/`.
- Select a final task in Projects and `W` saves its loaded prerequisite closure as
  a workflow template. Select a template and `s` enters a common scope per step;
  F6 obtains a read-only canonical preview, then `y` confirms that exact run.
  `w` browses templates/runs and their linked tasks in Projects/Operations;
  Operations retains its default hierarchy tree. Inspectors show arbitrary
  owners, criteria, prerequisites, fixed summed limits and canonical states.
  Run tasks retain existing approval/actual-Neovim/acceptance/Apply controls.
  CLI supports distinct per-step scopes. Qualified on `2bd752a3d1` with 30 native
  TUI/Neovim tests and actual forms, dependencies, restart, Follow/diff,
  original conflict denial, exact Apply and 36 switches across three sizes.
  See [workflows](workflows.md); captures are at `/workspace/artifacts/cli-workflows/`.
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

Owner creation and explicit execution grants are available through the native
forms below and documented owner CLI. Advanced role versioning/membership remains
in the CLI. Dashboards display canonical state and evidence; unsupported CPU,
timeline and billing metrics are deferred. See [command guide](organization-commands.md).

Qualification must physically exercise all presets, hierarchy search/collapse,
real owner controls, exact approvals, narrow/wide terminals and preference reload.
Meaningful automated checks complement that evidence; snapshots alone cannot
qualify the interaction. See [MVP criteria](mvp.md).

## Owner creation and execution forms (qualified)

`N` opens a local project form, `T` a proposed task, `G` a goal and `U` an
arbitrarily named agent. Select a task and press `s` to review its execution scope.
Tab/Enter advances fields, Shift-Tab goes back, arrows choose named records,
Alt-Enter inserts a line, and bracketed paste supports multiline criteria and
file lists. F6 validates and captures the exact owner request. `y` confirms it;
Escape returns to the draft, and Escape from the draft makes no changes.

Creation uses the existing owner definition APIs. A project uses existing
project/create and links that returned identity; a link failure reports its ID
for recovery. It is not an atomic cross-store transaction. A task stays proposed
until separately authorized. Authorization captures the task ID/revision when
the form opens, defaults to exact tool approvals and no delegation, and names
all inputs/immutable files/artifacts/check argv/model/resource bounds. Check argv
is an explicit JSON array of arrays; it is not parsed as shell text. The frozen
review includes host tool/network/credential limits, separate acceptance/Apply,
in-flight token allowance and a matching owner price estimate when available.
The daemon remains authoritative for validation, permissions and stale revisions.

Two fresh real packaged-CLI/native-daemon workflows passed these controls, exact
approvals, detached completion, official-Neovim review, acceptance and Apply.
See [testing](testing.md) for commands/evidence and the deterministic provider limit.

## Concept-art dashboard shell (qualified)

The supplied Console/Workspace/Projects/Operations/Review art is the visual
reference. Dashboard surfaces now use its dark charcoal/navy palette, blue
selection, rounded panels, workspace header, left navigation and right inspector.
Operations keeps the interactive arbitrary hierarchy as an indented card tree;
wide terminals add canonical agent/task/project counts and recorded activity.
Projects shows accepted/total task progress. Workspace centers actual Neovim,
with declared-file exploration and Follow/Diff/Independent controls. Its clean
native grid has line numbers and matching background colors.

Review adds a hash-verified per-artifact inline diff through existing Preview,
regular-file checks and diffy. It is bounded to 64 KiB per side and cached by
result identity/digest; `n` cycles artifacts and aligns the next native Neovim file.
The original is a snapshot at load. Apply still checks
current base hashes independently. Other/large/binary files use native Neovim.
This preview neither accepts evidence nor applies files.

The approved selective Apply follow-up reuses the fixed owner form: `A` chooses
whole accepted files, arrows/Space toggle, Tab/Enter moves, F6 freezes exact paths
and `y` confirms. Empty selection is rejected; Esc retains a reviewed draft.
The confirmation warns that full-result checks do not certify an installed subset,
and that a subset consumes the result's one Apply. The existing native dirty-buffer
fence remains. All 25 UI tests, complete package and physical checks passed in
[selective Apply](selective-apply.md).

Terminal cells require adaptations: at narrow widths the sidebar disappears and
panes stack; inspectors can be hidden and ratios retain user preferences. Existing
Console conversation/composer is preserved. Wide connected organization diagrams,
unsupported CPU/memory/24-hour cost charts, timelines and future actions from the
art are not fabricated. The current interactive tree remains usable for deep,
user-defined hierarchies.

Fresh pane defaults allocate 65% to the center; existing saved ratios are preserved.
Color uses the shared terminal palette and contrast helpers: true-color targets the
reference RGB palette, 256-color terminals quantize it, and limited/no-color terminals
keep readable native defaults. Physical monochrome and RGB workflows both passed.
RGB captures used the real Neovim terminal with `FORCE_COLOR=3`, `COLORTERM=truecolor`
and inherited `NO_COLOR` removed only in that owned test terminal. Screenshots and
plain native cell captures are in `/workspace/artifacts/cli-ui-update/`.

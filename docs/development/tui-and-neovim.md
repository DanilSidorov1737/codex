# TUI layouts and actual Neovim integration

Foundation update: official Neovim v0.12.6 and its external UI API passed real
editing/redraw/conflict-check qualification. See [evidence](qualification.md).
B4 layouts are qualified; the actual Workspace pane and safe application are now
being implemented/tested in B5. See [current contracts](neovim-workspace.md).
The remaining design below is historical guidance where not superseded.

## Shared pane model

Retain Codex's terminal frontend and app-server client. Introduce a small split/tab
pane tree with stable pane IDs, focus, minimum sizes, visibility and saved ratios.
Presets are data describing existing panes, not separate application shells or
a general widget plugin framework.

Save per-user/per-workspace layout state outside source files, using existing
local-settings conventions where practical. Version settings, clamp impossible
dimensions, fall back safely on invalid configuration and retain a reset command.
Keyboard navigation must work without a mouse, color, special fonts or icons.
Narrow terminals show one main pane with toggleable navigation/inspector rather
than squeezing the entire mockup into unreadable columns.

| Layout     | Main surface                                   | Supporting panes                                   | MVP behavior                                                                               |
| ---------- | ---------------------------------------------- | -------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Console    | Existing conversation/composer and activity    | Organization/project navigation, task inspector    | Submit scoped requests, select agent/task, inspect evidence                                |
| Workspace  | Real official Neovim                           | Explorer, collaboration, terminal, tests/diff      | Open/save/edit real files; Follow AI/Live Diff/Independent modes with safe limits          |
| Projects   | Goals/project/task lists and dependency detail | Owners, milestones, artifacts and blockers         | Minimal creation/proposal, assignment, transitions and criteria; later timelines/templates |
| Operations | Interactive hierarchy tree                     | Node inspector, execution status, schedules/events | Tree is default; search/filter/collapse, select, pause/resume/disable; task list secondary |
| Review     | Exact patch/artifact and evidence              | Approvals, human actions, audit                    | Approve/reject/request changes and inspect tests; separate patch-apply authorization       |

Operations should render visible rows of a collapsible indented tree initially.
The wide connected-card concept remains an optional later visualization if useful.
Lazy child loading, path search and selected-node summaries support deep/large
organizations. Moving a node is an explicit command with validation and review;
the tree is not an implicit permission editor.

Pane switches preserve selection and user-entered drafts. Global shortcuts must
not steal Neovim's ordinary modal input. Provide a configurable host focus prefix,
visible focus indicator, and safe escape from an editor pane. Confirm Ctrl-P
behavior with the user configuration rather than breaking Neovim mappings.

## Integration alternatives

| Method                                                | Fit                                                               | Decision                                                                     |
| ----------------------------------------------------- | ----------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| External full-terminal Neovim with suspend/resume     | Actual editor and inexpensive initial compatibility baseline      | Spike/fallback only; does not fulfill simultaneous pane workspace            |
| Neovim native TUI in PTY, rendered via existing vt100 | Reuses terminal infrastructure and supports real modal editing    | Candidate fallback; terminal emulation/focus/OSC compatibility can be costly |
| `nvim --embed` + MessagePack RPC + external UI grid   | Structured redraw/input, explicit pane size, buffer activity APIs | Recommended embedded workspace                                               |
| In-process libnvim or Rust in-editor plugin bindings  | Coupled process/ABI/plugin lifecycle                              | Avoid for first integration                                                  |
| Custom editor/Vim imitation                           | Contradicts explicit requirement                                  | Prohibited                                                                   |

Ratatui draws cells emitted by Neovim; it does not parse/edit code, implement
motions, maintain undo, run LSP itself, or simulate Neovim. Neovim remains a
real separate executable from [neovim/neovim](https://github.com/neovim/neovim).

## Acquisition after authorization

No Neovim executable was installed or run in Phase 0; `nvim` is absent.
Official documentation/source files were downloaded for read-only research only.

1. Select an official stable release and supported host/architecture.
   At research time the official `stable` ref was
   `779c36969e31080503be2dfccbcc3f999e289606`; its CMake version is 0.12.6.
   Pin an immutable tag/commit and release asset/checksum at implementation time;
   a moving stable ref alone is not a reproducible pin.
2. Obtain the matching actual executable from official release assets, or build
   the official source if no asset fits. Preserve TLS/checksum/signature validation
   provided by the project; check provenance rather than changing expected hashes.
3. Record binary path, `nvim --version`, source/tag, checksum, licenses and update policy.
   Support an already-installed official Neovim when compatible; no forced editor bundle.
4. Use a clean, isolated config/state in tests, then qualify user's real config and
   supported plugins. Do not modify the user's editor files automatically.
5. Review redistribution notices and packaging separately; no bespoke editor binary.

## Embedded protocol plan

Use the official
[API](https://github.com/neovim/neovim/blob/7e1ae5115f1f7088abe329892948708fc809d45e/runtime/doc/api.txt),
[UI events](https://github.com/neovim/neovim/blob/7e1ae5115f1f7088abe329892948708fc809d45e/runtime/doc/api-ui-events.txt),
and [startup](https://github.com/neovim/neovim/blob/7e1ae5115f1f7088abe329892948708fc809d45e/runtime/doc/starting.txt)
contracts. This is researched behavior, not a completed integration.

- Spawn `nvim --embed` on private stdin/stdout pipes; stderr is separate.
  For a UI embedder, do **not** add `--headless`: normal embedded startup waits
  for `nvim_ui_attach` before loading user config.
- Query `nvim_get_api_info`, negotiate required APIs/extensions, identify the client,
  and attach with pane width/height, `rgb` and `ext_linegrid`.
  Keep ext_multigrid/ext_cmdline/ext_popupmenu disabled initially so Neovim draws
  its own splits/menus within one grid.
- Implement the documented cell/highlight/cursor/scroll/resize/mode events.
  Handle batches in order and expose a complete frame at `flush`.
  Respect Unicode width, double-width continuation cells, color defaults,
  clipping, cursor style, popup/menu messages and blink/focus behavior.
- Route keys through `nvim_input`, bulk paste through `nvim_paste`, mouse through
  `nvim_input_mouse`, and pane resize through the UI resize API. Handle partial
  input acceptance/backpressure rather than dropping keys.
- Bound RPC frames and event queues; match request IDs and handle nested requests
  and errors without blocking the render loop. Ignore documented compatible
  extension fields, but fail visibly on unsupported required behavior.
- Keep application control RPC separate from editor RPC. Prefer local pipes;
  optional sockets require private permissions. Do not expose an unauthenticated
  Neovim TCP listener.
- Editor exit/crash must preserve daemon work and restore host terminal state.
  Normal `--embed` channel loss can exit Neovim; initially recover saved files
  and Neovim swap/undo data. Persistent live editor processes across TUI detach
  need a separately qualified supervisor/socket mode and are deferred.

## RPC library choice

The existing lockfile contains no MessagePack or Neovim RPC client.
[nvim-rs](https://github.com/KillTheMule/nvim-rs) offers asynchronous Tokio RPC and
would reduce transport work, but the inspected manifest is 0.9.2, LGPL-3.0,
and its README describes unstable APIs. The repository's
[cargo-deny policy](../../codex-rs/deny.toml) does not allow LGPL-3.0.
Do not add it or weaken the license policy without explicit dependency/legal review.

Default proposal: use an approved permissive RPC client if the spike identifies
one that fits; otherwise use [rmpv](https://github.com/3Hren/msgpack-rust)
(inspected 1.3.1, MIT) for serialization plus a narrowly scoped Tokio adapter
for required Neovim requests/events. This is transport/render integration,
not a new editor. It still needs tests for framing, cancellation and nested
requests. No new library is installed or declared in Phase 0.
Choose and pin the smallest compatible dependency after the spike; avoid
building a generic multi-protocol RPC framework.

## Collaboration modes

**Follow AI:** select an agent/attempt; use correlated file-change events to open
its isolated worktree file and highlight changed locations. Respect user cursor/
focus changes, debounce updates and provide stop-follow. Do not steal focus during
manual input, run pending commands, or replace unsaved buffers.

**Live Diff:** compare captured base and proposed change in Neovim diff windows
or existing TUI diff panes. Each approval identifies exact content hashes.
Applying a reviewed change rechecks destination version and modified buffers.
Initial acceptance is whole reviewed patch/artifact; selective hunk acceptance
comes after base/conflict semantics are qualified.

**Independent Editing:** user workspace remains separate from the agent's
isolated changes. Agent activity notifications continue without cursor changes.
Shared disk writes do not imply safe shared-buffer collaboration.

**Later collaborative buffer editing:** subscribe with `nvim_buf_attach`,
track `changedtick`, base hashes and edit provenance, and propose edits through
a controlled API. Check the expected revision and apply together in one Neovim
main-loop operation; a separate remote check then set is a race. Reject/rebase
stale edits and preserve human undo boundaries. Byte-index APIs need Unicode
tests. Do not start with CRDT/OT or blind external file reloads.

Selection-to-agent requests package exact buffer text/range, path, changedtick,
task/project, intent, and permitted context. They do not grant write authority.
External AI edits, Neovim plugins, LSP and modelines/autocommands must respect
the human/editor trust boundary. Treat the editor as human-authorized code:
it can launch processes, so agent-originated editor commands must be constrained.

## Required future real-editor checks

Enter normal/insert/visual/command modes in actual Neovim; edit, undo, save and
reopen a file; use splits and a qualified LSP/plugin; resize the pane; paste Unicode;
switch focus/layouts; inspect a real diff; follow an actual file-changing attempt;
reject a conflicting patch while a human buffer is dirty; close/crash the editor
and demonstrate task continuation. Record versions and transcripts, not just
mocked RPC or a screenshot of painted text.

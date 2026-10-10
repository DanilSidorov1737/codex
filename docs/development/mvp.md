# Detailed first MVP plan

Status: **proposed; do not begin until the owner approves**.

## User outcome

A user defines one or several agents with their own role names and hierarchy,
creates a small coding or document project with explicit completion criteria,
and authorizes a bounded plan. The daemon runs and delegates tasks while the
TUI is closed, preserves state through a restart, and presents results with real
evidence for review. The user opens actual Neovim, follows agent edits, inspects
diffs and safely continues independent editing.

No required “CEO” or supervisor exists. First validation uses both a single-agent
configuration and a small custom organization. Multiple groups/levels are data,
not hard-coded roles.

## Included features

- Persistent agent/role/group definitions with versioned edits, arbitrary primary
  forest, team memberships, stable IDs, enabled/retired state and cycle checks.
- Existing projects extended with objective, owner, task list, prerequisites,
  explicit artifact/test criteria and one-shot due time.
- A limited planner that proposes validated tasks; approved scope allows subsequent
  authorized decomposition/delegation within resource and permission ceilings.
- Durable task ownership, attempt/lease/generation, status, checkpoint and correlated
  Codex thread/turn identity. Bounded queue and fair scheduling.
- Existing Codex tool/model/agent runtime integration. One verified provider,
  explicit model selection, no new multi-model routing engine.
- Deterministic tool/project/path ceilings, pause/revoke, durable approvals,
  action intent/evidence, token/time/concurrency limits and cost estimates.
- Scoped explicit project/agent notes and provenance; no vector store.
- All five minimal useful layouts, saved pane settings, keyboard focus, and
  primary Operations hierarchy tree.
- Actual official Neovim for file editing/save/undo, real diff viewing, Follow AI
  and Independent Editing. Human/agent changes remain isolated; conflicts block apply.
- Restart/reconciliation, backup/restore evidence and no blind side-effect replay.

## Excluded from first MVP

Financial transactions, procurement, account creation, outbound communications,
remote publishing/deployment, automatic authority expansion, mandatory shared-function
agents, distributed workers, thousands of simultaneous model calls, marketplace
development, rich calendar/meetings, automatic memory promotion, vector retrieval,
timeline designer, arbitrary workflow DSL and concurrent same-buffer CRDT editing.

Exclude these capabilities from execution policy as well as the UI. Real
interactive TUI/editor testing and recovery/security checks are part of the MVP,
not optional polish.

## Dependencies and prerequisites

| Dependency                                                 | State / action                                                                                                |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Rust 1.95.0, Node >=22, pnpm 10.34.5                       | Present from earlier onboarding; activate documented paths                                                    |
| Real full Codex CLI and runtime/native prerequisites       | Not built in Phase 0; build/qualify before feature work                                                       |
| Existing Tokio/SQLx/Serde/Ratatui/Crossterm/PTY/vt100      | Reuse pinned workspace versions                                                                               |
| Official Neovim executable                                 | Requires owner installation authorization; pin official source/release and verify integrity                   |
| Neovim MessagePack transport                               | Requires dependency decision after spike; permissive codec/client preferred; no LGPL exception without review |
| Ponytail                                                   | Session skill available; standalone upstream plugin install/hooks require authorization                       |
| Provider authentication and model quota                    | Use existing supported bindings; verify names/status and actual operation, never ask for raw secret in chat   |
| Protected service/state directories and owner vs agent IPC | Qualify permissions/sandbox access on target host                                                             |
| Isolated fixture/task workspaces and Git                   | Preserve user changes; future task-workspace creation needs explicit product policy                           |
| Process supervision                                        | Existing daemon first; reboot startup qualification is separate from TUI detach                               |

No new dependencies or services should be added beyond the approved slice.

## Implementation sequence

| Step | Implementation work and concrete seams                                                                              | Depends on / verification                                                                          |
| ---- | ------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| M0   | Build baseline CLI; physically use existing chat/commands; record build and native requirements                     | Owner approval; real binary and existing selected tests                                            |
| M1   | Spike official Neovim external UI, daemon detach/reconnect, durable pending approval handling, owner/agent IPC      | M0 and install permission; real input/resize and denied access evidence; stop/revise if seams fail |
| M2   | Add authoritative store, explicit migrations, protected file location, FULL durability and no-rebuild recovery      | M1; transaction/constraints, backup/restore and corruption/DB-full tests                           |
| M3   | Add versioned organization/role commands and project planning extension                                             | M2; arbitrary roles/depth, stable IDs, reparent/cycle tests, preserve existing project identity    |
| M4   | Add task graph/state machine, acceptance records, intents, approvals and audit/outbox                               | M3; single owner/revision constraints, stale approvals and atomic transitions                      |
| M5   | Add bounded scheduler/leases/reservations and Codex execution adapter using AgentControl/ThreadManager/contributors | M4; duplicate claim, budget, delegated-turn gates, fairness and supervisor wait/yield tests        |
| M6   | Add scoped notes/inbox and crash reconciliation                                                                     | M5; persisted context, lost delivery/dedupe, stale attempt fencing, unknown-action behavior        |
| M7   | Add shared panes/focus/presets; reuse Console; simple Projects and primary Operations tree                          | M3–M6; actual keyboard navigation, narrow/wide terminals and settings reload                       |
| M8   | Complete minimal Review and Workspace with actual Neovim, isolated Follow AI and safe diff application              | M1, M4, M7; exact result approval, dirty-buffer conflict and editor-close independence             |
| M9   | End-to-end pilot, detach/restart/restore/negative policy tests; document results and remaining scope                | All prior steps; every acceptance criterion below passes or blocks release                         |

These are logical commit-sized milestones, not a mandate to create ten modules.
Before each step inspect current callers and reuse existing contracts. Keep core
runtime changes narrow. Extend typed app-server schemas with fork-specific
commands only where existing endpoints lack the needed semantics; command names
and CLI syntax are decided after M1, not promised here.

## Testable acceptance criteria

| ID  | Test                                                                                                           | Passing result                                                                                                    |
| --- | -------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| A01 | Define one standalone agent and a custom multi-level forest with arbitrary role names                          | Both run without required CEO/manager levels; definitions survive daemon restart                                  |
| A02 | Rename/reparent nodes, reject a cycle, move a member between groups                                            | Stable IDs/assignments retained; invalid change rejected atomically; no authority gain                            |
| A03 | Create a project/task graph and a constrained natural-language plan                                            | Valid tasks/owners/criteria recorded; prerequisites respected; invalid/cyclic plans cannot execute                |
| A04 | Two claimers race for a task; simulate stale worker completion                                                 | One active claim; stale generation cannot settle/complete/release another attempt's reservation                   |
| A05 | Run a useful local project through real Codex model/tool execution and delegated work                          | Artifact delivered, existing test/check genuinely executed, acceptance evidence recorded                          |
| A06 | Close/detach TUI during work and reconnect; kill/restart daemon at defined checkpoints                         | Authorized work is UI-independent; restart reconstructs tasks and safely reconciles uncertain steps               |
| A07 | Pause/revoke agent/subtree and attempt direct/delegated/mailbox/automatic execution                            | New affected work denied; active work cancelled/checkpointed within documented bounds; unrelated task proceeds    |
| A08 | Request protected action with no reviewer, then approve/reject through authenticated owner                     | Pending request survives reconnect/restart; silence does not approve; agent self-approval and stale hash rejected |
| A09 | Exhaust token/time/concurrency budget; race child resource reservations                                        | Admission stops at configured limits with documented in-flight allowance; no invisible child or retry usage       |
| A10 | Simulate post-dispatch crash with unknown external result                                                      | Task/action enters reconciliation; no automatic duplicate non-idempotent action                                   |
| A11 | Put an untrusted instruction in a document/board message and request broader paths/tools                       | Cannot modify policy, DB, owner socket, secrets or unauthorized project; denial is audited                        |
| A12 | Use all five layouts, customize pane sizes/visibility, restart TUI                                             | Settings retained; real backend state consistent; keyboard-only operation works at narrow and wide sizes          |
| A13 | Navigate Operations tree using selection/search/collapse/expand and inspect/control a node                     | Tree remains primary; loaded children/status belong to correct stable node; controls use policy gate              |
| A14 | Use actual official Neovim to enter modes, edit/save/undo, open splits/diff, resize and paste Unicode          | Real file and editor behavior correct; user input/focus restored; no fake editor                                  |
| A15 | Follow real AI edits in an isolated task workspace, stop following, edit independently, apply a reviewed patch | No cursor theft/unsaved overwrite; dirty-base/buffer conflict blocks application; diff matches approved hashes    |
| A16 | Close/crash Neovim while another authorized task runs                                                          | Daemon task continues; saved files recover; unsaved data limitation/recovery route documented                     |
| A17 | Store scoped notes and resume with project context; query from another unauthorized project                    | Allowed provenance-aware notes retrieved; prohibited notes withheld before model context assembly                 |
| A18 | Corrupt/full organization DB, restore a verified backup in separate fixture state                              | New consequential execution stops; no empty automatic reset; restored tasks/grants/approvals/budgets consistent   |
| A19 | Generate 1,000 stored agents, test deep trees and run 2–4 bounded attempts                                     | No model session per sleeping agent; record memory/latency/query counts; no unbounded redraw/event queue          |
| A20 | Review output/test evidence, request changes and finally accept                                                | Acceptance references exact current artifact/tests; completion cannot be set solely by model assertion            |
| A21 | Fresh pilot startup and an idle interval                                                                       | Startup handshake succeeds, no idle inference loop, health/recovery limitations recorded                          |

Performance criterion proposal for A19: on the declared host, tree navigation
p95 below 100 ms after loaded data and first page below 1 second, with bounded
visible-row render work. Record hardware, release/debug mode, terminal size,
fixture shape, repetition and memory. Approve/revise targets after M1 baseline;
do not make an unsupported capacity promise from synthetic tests.

A05 needs an explicitly authorized, bounded live provider test in an isolated
fixture, plus a non-code artifact pilot. No remote communication/payment/deploy
is necessary. Scripted model fixtures are valuable for fault injection but do
not replace that live check or physical TUI/editor interaction.

## Release/approval gate

After implementation approval, M1 may require a plan update and another decision
if a dependency/license/platform assumption fails. MVP is complete only when
required evidence passes and limitations are explicit. Failed core safety,
recovery or editor criteria block completion; they are not waived by “tests
mostly pass.” Review and merge/publish actions remain separately authorized.

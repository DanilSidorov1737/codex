# Detailed first MVP plan

Status: **owner-approved MVP implementation; B1–B5 locally qualified; B6 release qualification underway**.
Updated after [M0/M1 foundation qualification](qualification.md). The actual CLI,
daemon and official Neovim are available. B1–B5 delivered durable organization
records, bounded execution/delegation, all five layouts and the real integrated
editor with exact reviewed application. B6 qualification is in progress; the
bounded live-provider release gate remains outstanding.

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

## First complete workflow

1. Define a standalone agent or a custom organization, with instructions and explicit grants.
2. Create a goal and linked local project, describe the desired result, and specify
   deliverables, meaningful checks, allowed paths/tools and resource limits.
3. Ask for a task plan or enter tasks manually. Inspect assignments, prerequisites
   and acceptance criteria; authorize the bounded plan.
4. Let the daemon execute, delegate and checkpoint within that authorization.
   Close the TUI, reconnect, pause work or answer a protected-action approval.
5. Open Workspace to follow isolated agent changes or edit independently in actual
   Neovim. Use Review to inspect exact diffs, artifacts and recorded test results.
6. Request changes or accept the final result. Applying changes to the user's
   workspace is a separate, version-checked action.

The first coding pilot fixes a small fixture-project bug with a regression check
and a written explanation. A second pilot produces a document from supplied local
material. Both must work with one agent; the coding pilot also exercises delegation
between user-named agents. Example role names do not become mandatory roles.

Authorizing the plan allows routine in-scope work without asking the owner at every
step. Internal prerequisite tasks may be accepted automatically only under explicit
preauthorized, machine-checkable criteria. Final project acceptance remains the
owner's decision. Model claims alone never satisfy either kind of acceptance.

## Included features

- Persistent agent/role/group definitions with versioned edits, arbitrary primary
  forest, team memberships, stable IDs, enabled/retired state and cycle checks.
- Durable goals with optional parent goals and explicit success criteria; existing
  projects linked to goals and extended with owner, task list, prerequisites,
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

## Initial operating envelope

One Linux host, one human owner, one validated Responses-compatible provider and
local coding/document projects. Default to two active executions; qualify four.
Qualify one and 1,000 stored agents without allocating a model session per stored
agent. Organization shape is user-configurable and independent of these execution
limits. Stored-agent capacity is a test target, not a demonstrated feature.

Use existing provider/model configuration for explicit selections. Broader provider
compatibility and automatic routing follow later. Enforce token, elapsed-time,
concurrency, task/delegation and retry ceilings; show reported usage and cost
estimates with their source. Document in-flight allowance rather than promising
an exact provider-billing cap that cannot be enforced locally.

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

| Dependency                                                 | State / action                                                                                       |
| ---------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| Rust 1.95.0, Node >=22, pnpm 10.34.5                       | Present from earlier onboarding; activate documented paths                                           |
| Real full Codex CLI and runtime/native prerequisites       | Qualified: CLI, Code Mode host, real Bubblewrap and complete package; reuse recorded setup           |
| Existing Tokio/SQLx/Serde/Ratatui/Crossterm/PTY/vt100      | Reuse pinned workspace versions                                                                      |
| Official Neovim executable                                 | Checksum-verified official v0.12.6 installed; real embedding and physical editing checks passed      |
| Neovim MessagePack transport                               | Qualified narrow external UI adapter using MIT rmpv 1.3.1/rmp 0.8.15       |
| Ponytail                                                   | Session skill available; standalone upstream plugin install/hooks require authorization              |
| Provider authentication and model quota                    | No live provider test yet; select binding/model and bounded budget before the live pilot             |
| Protected service/state directories and owner vs agent IPC | Actual owner socket/state/project/native helper isolation denials passed |
| Isolated fixture/task workspaces and Git                   | Preserve user changes; future task-workspace creation needs explicit product policy                  |
| Process supervision                                        | Existing daemon first; reboot startup qualification is separate from TUI detach                      |

No new dependencies or services should be added beyond the approved slice.

Reuse the Rust daemon, Tokio, SQLx, existing typed JSON-RPC, AgentControl,
ThreadManager, ThreadStore, sandbox/tools/MCP and Ratatui/Crossterm. Add a protected
`organization.sqlite` for business state rather than rewriting runtime history.
Use Ponytail for design, implementation and review; no additional plugin install
is needed to use the available session skills.

## Build order and usable checkpoints

These checkpoints break the same complete MVP into reviewable slices. They do not
remove the five layouts, actual editor, security or recovery from the release.
An approved MVP scope covers its ordinary implementation/test/doc commits; these
checkpoints are evidence gates, not requests for repeated permission.

| Build                        | Deliverable and implementation steps                                                                                                                                                                                                                                                           | Checkpoint evidence                                                                                                                                                                                                                                                                                                   |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| B1: durable definitions      | Open protected authoritative store with explicit migrations and fail-closed recovery; add only the records needed for agents, role versions, groups, goals, project links and proposed tasks. Expose owner operations through the existing app-server protocol. Preserve existing project IDs. | Create a standalone agent and custom forest, link a goal/project, rename/reparent, reject cycles and reload the same stable IDs after restart. Actual storage backup/restore and invalid-migration/corruption tests pass. M2–M3; A01–A03/A18 portions.                                                                |
| B2: one safe background task | Implement task transitions, attempts, action intents, durable approvals, audit/outbox and reservations. Add a bounded queue and map an admitted attempt to the existing Codex runtime. Capture exact artifacts/check results. Implement startup reconciliation before unattended use.          | A real local task produces a file only after required approval; closing the client does not stop it. Pause, stale approval, budget exhaustion, duplicate claim and interrupted-start checks pass. Pending approvals survive daemon restart without automatic acceptance. M4–M5 plus minimum M6 recovery; A04/A06–A11. |
| B3: bounded multi-agent work | Validate planner proposals and prerequisites; permit authorized delegation to arbitrary configured agents. Persist critical inbox messages and scoped notes, deduplicate delivery, fence stale attempts, and release active capacity while waiting. Capture role/policy versions per attempt.  | Two user-named agents complete dependent work under the same protected scope; a waiting parent cannot starve a child. Unauthorized memory, new authority and extra work are denied. Restart retains task ownership/context and uncertain actions require reconciliation. M5–M6; A03/A07/A09/A10/A17.                  |
| B4: five useful TUI layouts  | Add shared pane/focus/settings handling; reuse Console chat, project views, diff/history widgets and app-server events. Deliver Console, Projects, primary Operations tree, Review evidence/approval queue, and Workspace file/task inspector.                                                 | Physically navigate all five presets, search/collapse/expand the tree, inspect a task, decide an approval and retain pane sizes/visibility after reconnect. No screen invents status or test results. M7 and Review portion of M8; A12/A13/A20.                                                                       |
| B5: actual Neovim Workspace  | Launch official `nvim --embed`, attach its external UI and render bounded redraw batches in the existing TUI. Route keyboard/paste/mouse/resize and focus. Add isolated Follow AI, real diff splits, independent editing and explicit safe patch application.                                  | Real editor save/undo/Unicode/diff/focus checks pass inside the app. Human dirty buffers and changed disk bases block apply. Closing/crashing the editor leaves daemon work running. M8; A14–A16.                                                                                                                     |
| B6: release qualification    | Run coding and document pilots, first as one agent and then with delegation. Exercise detach, daemon restart, pending approvals, corrupt/full DB, backup restore, protected paths, stored-agent scale and idle behavior. Review simplicity/correctness and maintain evidence/docs.             | All A01–A21 pass on the declared host, including bounded live-model runs and physical TUI/editor interaction. Record remaining platform/provider limits. M9.                                                                                                                                                          |

B1 is the first product implementation slice after approval. B2 starts with
deterministic Responses fixtures for fault injection; they do not substitute for
B6's bounded live-model evidence. Workspace is not considered delivered at B4:
B5 must put the actual editor inside the app.

### Backend contracts to define before each slice

- Owner operations: versioned agent/role/group edits, goal/project/task planning,
  plan authorization, pause/resume/revoke, exact approval decisions, result
  acceptance, backup/restore and paginated inspection/events.
- Agent operations: scoped task proposals/delegation, checkpoints, evidence,
  notes and authenticated messages. They cannot call owner administration or
  approve their own protected actions.
- State ownership: coupled claims/reservations/transitions/audit/outbox commit in
  one organization transaction; runtime start occurs afterward using a recorded
  intent and correlated thread/turn IDs. No model or network call holds that transaction.
- Recovery: reissue durable unresolved approvals to the authenticated owner;
  reconcile interrupted dispatch against runtime/artifact evidence. A lease timeout
  alone never proves that an action did not happen.
- Editing: capture input/base hashes when creating a task workspace; keep human
  changes separate. Apply only the reviewed artifact against a matching filesystem
  base and matching Neovim buffer revision. Preserve conflicts for explicit resolution.

Define only the operations/records needed by the slice and follow repository schema
generation conventions. Endpoint and CLI names are implementation decisions;
this plan does not advertise commands that do not exist.

### Minimum useful layout content

| Layout     | MVP content                                                                                                                                            |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Console    | Existing conversation/composer plus selected project/agent/task and truthful event/output stream.                                                      |
| Workspace  | File/task inspector, actual Neovim, Follow AI/Independent Editing, real diffs and task/test output.                                                    |
| Projects   | Goal/objective, owners, task/prerequisite list, deliverables/criteria, progress from accepted work and inspector.                                      |
| Operations | Primary searchable expandable hierarchy tree, stable selected node, active/waiting state, resource limits and protected controls; task list secondary. |
| Review     | Exact artifact/diff, test/evidence/audit records, pending protected approvals, request changes and final acceptance.                                   |

Customization means saved preset, pane visibility/sizes and focus/key settings.
A general layout scripting language, graphical mockup replica and shared-buffer
editing engine are outside the MVP.

## Implementation sequence

| Step | Implementation work and concrete seams                                                                              | Depends on / verification                                                                                              |
| ---- | ------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| M0   | Complete: built real CLI/helpers/package; physically used existing TUI commands                                     | Recorded qualification evidence; do not repeat unless relevant foundations change                                      |
| M1   | Complete foundation spike: official editor embedding, daemon detach/reconnect, owner socket, approval replay        | Pending approval replay proved on reconnect, not crash/restart; agent IPC denial and product recovery remain MVP gates |
| M2   | Add authoritative store, explicit migrations, protected file location, FULL durability and no-rebuild recovery      | M1; transaction/constraints, backup/restore and corruption/DB-full tests                                               |
| M3   | Add versioned organization/role commands, goal records and project planning extension                               | M2; arbitrary roles/depth, stable IDs, reparent/cycle tests, preserve existing project identity                        |
| M4   | Add task graph/state machine, acceptance records, intents, approvals and audit/outbox                               | M3; single owner/revision constraints, stale approvals and atomic transitions                                          |
| M5   | Add bounded scheduler/leases/reservations and Codex execution adapter using AgentControl/ThreadManager/contributors | M4; duplicate claim, budget, delegated-turn gates, fairness and supervisor wait/yield tests                            |
| M6   | Add scoped notes/inbox and crash reconciliation                                                                     | M5; persisted context, lost delivery/dedupe, stale attempt fencing, unknown-action behavior                            |
| M7   | Add shared panes/focus/presets; reuse Console; simple Projects and primary Operations tree                          | M3–M6; actual keyboard navigation, narrow/wide terminals and settings reload                                           |
| M8   | Complete minimal Review and Workspace with actual Neovim, isolated Follow AI and safe diff application              | M1, M4, M7; exact result approval, dirty-buffer conflict and editor-close independence                                 |
| M9   | End-to-end pilot, detach/restart/restore/negative policy tests; document results and remaining scope                | All prior steps; every acceptance criterion below passes or blocks release                                             |

These are dependency milestones, each delivered through small logical commits,
not a mandate to create ten modules. B1–B6 above define usable checkpoints.
Before each step inspect current callers and reuse existing contracts. Keep core
runtime changes narrow. Extend typed app-server schemas with fork-specific
commands only where existing endpoints lack the needed semantics.

## Testable acceptance criteria

| ID  | Test                                                                                                           | Passing result                                                                                                                                                               |
| --- | -------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| A01 | Define one standalone agent and a custom multi-level forest with arbitrary role names                          | Both run without required CEO/manager levels; definitions survive daemon restart                                                                                             |
| A02 | Rename/reparent nodes, reject a cycle, move a member between groups                                            | Stable IDs/assignments retained; invalid change rejected atomically; no authority gain                                                                                       |
| A03 | Create a goal-linked project/task graph and a constrained natural-language plan                                | Valid tasks/owners/criteria recorded; accepted prerequisites respected; invalid/cyclic plans cannot execute; internal automatic acceptance follows explicit checked criteria |
| A04 | Two claimers race for a task; simulate stale worker completion                                                 | One active claim; stale generation cannot settle/complete/release another attempt's reservation                                                                              |
| A05 | Run a useful local project through real Codex model/tool execution and delegated work                          | Artifact delivered, existing test/check genuinely executed, acceptance evidence recorded                                                                                     |
| A06 | Close/detach TUI during work and reconnect; kill/restart daemon at defined checkpoints                         | Authorized work is UI-independent; restart reconstructs tasks and safely reconciles uncertain steps                                                                          |
| A07 | Pause/revoke agent/subtree and attempt direct/delegated/mailbox/automatic execution                            | New affected work denied; active work cancelled/checkpointed within documented bounds; unrelated task proceeds                                                               |
| A08 | Request protected action with no reviewer, then approve/reject through authenticated owner                     | Pending request survives reconnect/restart; silence does not approve; agent self-approval and stale hash rejected                                                            |
| A09 | Exhaust token/time/concurrency budget; race child resource reservations                                        | Admission stops at configured limits with documented in-flight allowance; no invisible child or retry usage                                                                  |
| A10 | Simulate post-dispatch crash with unknown external result                                                      | Task/action enters reconciliation; no automatic duplicate non-idempotent action                                                                                              |
| A11 | Put an untrusted instruction in a document/board message and request broader paths/tools                       | Cannot modify policy, DB, owner socket, secrets or unauthorized project; denial is audited                                                                                   |
| A12 | Use all five layouts, customize pane sizes/visibility, restart TUI                                             | Settings retained; real backend state consistent; keyboard-only operation works at narrow and wide sizes                                                                     |
| A13 | Navigate Operations tree using selection/search/collapse/expand and inspect/control a node                     | Tree remains primary; loaded children/status belong to correct stable node; controls use policy gate                                                                         |
| A14 | Use actual official Neovim to enter modes, edit/save/undo, open splits/diff, resize and paste Unicode          | Real file and editor behavior correct; user input/focus restored; no fake editor                                                                                             |
| A15 | Follow real AI edits in an isolated task workspace, stop following, edit independently, apply a reviewed patch | No cursor theft/unsaved overwrite; dirty-base/buffer conflict blocks application; diff matches approved hashes                                                               |
| A16 | Close/crash Neovim while another authorized task runs                                                          | Daemon task continues; saved files recover; unsaved data limitation/recovery route documented                                                                                |
| A17 | Store scoped notes and resume with project context; query from another unauthorized project                    | Allowed provenance-aware notes retrieved; prohibited notes withheld before model context assembly                                                                            |
| A18 | Corrupt/full organization DB, restore a verified backup in separate fixture state                              | New consequential execution stops; no empty automatic reset; restored tasks/grants/approvals/budgets consistent                                                              |
| A19 | Generate 1,000 stored agents, test deep trees and run 2–4 bounded attempts                                     | No model session per sleeping agent; record memory/latency/query counts; no unbounded redraw/event queue                                                                     |
| A20 | Review output/test evidence, request changes and finally accept                                                | Acceptance references exact current artifact/tests; completion cannot be set solely by model assertion                                                                       |
| A21 | Fresh pilot startup and an idle interval                                                                       | Startup handshake succeeds, no idle inference loop, health/recovery limitations recorded                                                                                     |

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

Foundation checks are complete. Production dependency/license/platform failures
must be resolved before claiming their affected features. MVP is complete only when
required evidence passes and limitations are explicit. Failed core safety,
recovery or editor criteria block completion; they are not waived by “tests
mostly pass.” Review and merge/publish actions remain separately authorized.

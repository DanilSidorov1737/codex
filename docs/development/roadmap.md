# Phased roadmap

Current execution: Phase 2 MVP implementation and B1–B6 qualification are complete,
including real GPT-5.5 coding/delegation/document pilots. Later phases require
the next bounded scope to be chosen with the owner. See
[qualification evidence](qualification.md) and [status](status.md).

Owner-approved usability slice (2026-10-10): TUI owner creation/authorization and
concept-art dashboard styling are delivered and physically qualified on source
`46d999bf5b`. Existing RPC/widgets, 23 affected tests and two fresh complete native
workflows passed; screenshots/evidence are linked from status.md. This is a bounded
usability continuation of the MVP. The owner subsequently approved whole-file
[selective Apply](selective-apply.md); backend/API checkpoint `32edc1cc71` is
committed; Review `51f914a851` and refresh-denial fix `9fadf32797` are qualified with
319 automated tests and real CLI/official-Neovim interaction. The owner then
approved N1 scoped memory search; backend/CLI `8d33c5e0d8` and TUI `d6d049cc49`
are delivered and qualified: 691 automated tests, real restart/scoped retrieval,
remote-owner denial, actual CLI/TUI/Neovim and three-size rapid switching passed.
N2 recurring schedules is delivered and qualified on `e06def499e`; see
[its bounded contract](recurring-schedules.md). Broader knowledge promotion,
hunks and staged repeated Apply remain later work.

## Recommended next slices

The owner requested an expanded plan based on `docs/Codex CLI Fork.txt` on
2026-10-10. The sequence below is proposed work after the qualified selective
Apply slice, not implementation authorization or a claim of delivered features.
The owner subsequently approved N1 with “Ok do this” and N2 with “ok lets do this”;
N3 reusable bounded workflows was approved with “ok do this”; N4–N8 remain proposals.
Memory and recurring schedules are the first two steps. The added
steps cover practical portions of the text's workflow, continuity, organization,
visibility, editor and capability requirements.

| Order | Capability and bounded scope | Reuse and dependencies | Testable completion |
| --- | --- | --- | --- |
| N1 | Scoped persistent memory search: find relevant notes and evidence from previous work; preserve source, author and time. Start with existing project/agent scopes and explicit capture. Broader knowledge promotion needs its own access rules. | Existing SQLite notes, coordination tool, owner RPC and layout inspectors. Apply scope filtering before ranking; use lexical search before considering embeddings. | Relevant authorized notes are retrievable after daemon restart; another project/agent's private notes are denied; retrieved content cannot change permissions. Physically search and inspect source evidence in the CLI/TUI. |
| N2 | Recurring schedules: repeat an authorized task template at an interval, with pause/cancel, next-run visibility, occurrence and aggregate resource limits. Start with UTC intervals and explicit missed-run behavior. | Existing daemon, one-shot due time, durable task creation, grants, budgets and reconciliation. Use a stable occurrence identity; skip missed occurrences by default and avoid overlapping runs. | Closing the TUI leaves scheduling active on a running host; restart does not duplicate an occurrence; pause/cancel and exhausted aggregate limits prevent admission. Each run retains existing exact-action approvals and recovery fences. No claim of exactly-once external effects. |
| N3 | Reusable workflows: save a small task/dependency template for coding, research or documents, preview assignments and success criteria, then authorize a bounded run. | Existing validated plan proposals, prerequisites and delegation. Use structured templates instead of a workflow language. Repeated use may reference N2; research may retrieve N1. | Invalid dependencies/cycles roll back; unmet prerequisites block dispatch; each invocation gets distinct task identities and cannot expand its authorization. Physically run a short dependency chain and inspect its evidence. |
| N4 | Structured handoffs and blocker handling: checkpoint completed work, artifacts, remaining steps and the blocker; let the owner reassign within permitted scope or escalate for a decision. | Existing task ownership, attempts, leases, inbox/acknowledgment and memory. Add explicit ownership-transition semantics before permitting reassignment; old attempts stay fenced. | The replacement receives permitted context after restart, without repeating completed effects; a stale worker cannot mutate the task; uncertain actions remain blocked for reconciliation. Unrelated work continues while a human decision is pending. |
| N5 | Complete organization editing in Operations: edit role instructions, memberships, hierarchy and enabled/retired state from the interactive tree. | Existing owner CLI/RPC versioned edits and TUI forms. Preserve arbitrary user-defined roles/levels and stable identities; inspect consequences for active grants before confirmation. | Physical create/edit/reparent/retire/reload works, cycles and stale edits are refused, and active attempts retain captured role/policy versions. Role labels cannot confer new permissions. |
| N6 | Operational summaries and human action visibility: show upcoming runs, blockers, approval requests, recorded checks, failures and reported usage with links to source records. Start with on-demand local reports. | Existing canonical task/action/approval/audit/usage records and the five layouts; add N2 schedule projections and N4 blockers as available. Scheduled local reports can follow N2. | Counts and drill-downs match canonical records across reconnect/restart; unknown measurements remain unavailable and price quotes remain labeled estimates. Reports preserve scope and redact secrets; no external delivery is implied. |
| N7 | Neovim requests from selected code: send an explicit file/range/context snapshot to a proposed explanation, refactoring, debugging or test task. Separately qualify supported user configuration/LSP/plugins. | Existing actual official Neovim embedding, task forms, isolated execution and exact Review/Apply. Pin file/buffer revision in the request; captured context grants no additional paths/tools. | Physically select Unicode code in actual Neovim, inspect/cancel/authorize the proposed task, review isolated changes and prove dirty/stale original buffers still block Apply. User configuration tests preserve existing files. Live shared-buffer edits and hunk/staged Apply remain separate. |
| N8 | Capability inventory and one additional integration: show available tools/providers and their required permissions; qualify one useful read-only integration before broader automation. | Existing MCP, plugins/skills and provider configuration. Inventory configured capabilities separately from ones actually tested. External access and paid compatibility checks need matching authority. | A granted task can use the selected integration; an ungranted task is denied; failures and compatibility status are visible and credentials stay out of prompts/logs. No automatic installation, purchases, outbound messaging or silent model fallback. |

These are bounded increments rather than separate services or mandatory permanent
agents for the shared system functions. Use Ponytail, meaningful automated checks,
physical CLI/TUI interaction, actual Neovim checks when affected, updated evidence
and logical local commits for each approved slice. N1 qualification is complete;
N2 recurring schedules is complete: 700 automated tests and actual background,
approval, pause/restart/resume, cancellation, budgets, crash/remote denial and
native TUI/official-Neovim evidence. N3 reusable workflows is complete on
`2bd752a3d1`: 712 automated tests and actual dependency/restart/approval/Neovim/
conflict-denial/Apply checks; see [workflow evidence](workflows.md). N4 structured
handoffs/blockers is the next proposal, requiring matching owner scope.
Distributed workers, multimodal production, meetings, full-environment backup
automation and protected self-improvement remain in their later roadmap phases.

## Expanded next batch

**Proposed on 2026-10-11; implementation has not started.** The owner asked to
expand the next list so the batch accomplishes more. This refines N4 and draws
bounded additions from N5–N8; it does not authorize those implementations.
N1–N3 remain complete. Start with recovery and handoffs, then add the owner
controls and visibility below. Qualify each coherent slice as it is delivered.

| Order | Capability and practical scope | Reuse and acceptance gate |
| --- | --- | --- |
| 1 | Fix Apply recovery: restore editor interaction and retain the error after a confirmed rejection before an application intent exists. | Reuse canonical application receipts and dashboard state. Test that a confirmed rejection permits a fresh explicit confirmation; a timeout, disconnect or uncertain intent cannot cause automatic replay or a duplicate write. |
| 2 | Durable task checkpoints: record verified completed work, artifact references, remaining steps and blockers. | Reuse task/attempt records, scoped notes and artifact hashes. Reload permitted context after daemon restart; checkpoint text cannot substitute for acceptance or authorize effects. |
| 3 | Safe reassignment: let the owner transfer unfinished work to another configured agent with an explicit ownership transition. | Reuse grants, leases and attempts. Fence the old worker; preserve consumed/remaining root and schedule/workflow accounting rather than resetting limits. Validate the receiver's scope and reconcile uncertain effects before continuing. |
| 4 | Blocker handling and escalation: show why work stopped and which owner decision is needed. | Reuse canonical task state and inbox records. Test resolution/reassignment without widening authority, restart persistence and unrelated work continuing while a decision is pending. |
| 5 | Organization editing in Operations: edit role instructions, memberships, hierarchy and enabled/retired state from the tree. | Expose existing versioned owner CLI/RPC edits through native forms. Physically create/edit/reparent/retire; reject cycles/stale changes and retain active attempts' captured policy. Arbitrary names and levels remain supported. |
| 6 | Fuller workflow UI: edit templates, assignments, prerequisites and success criteria, and review distinct per-step scopes. | Reuse N3's existing CLI/RPC template revisions and canonical previews; the current UI uses a common scope. Reject invalid graphs/stale edits and preserve atomic authorization, fixed step budgets and explicit prerequisite acceptance. |
| 7 | Review action inbox: bring pending approvals, blocked decisions, requested changes and results needing review into one navigable list. | Derive entries from canonical records, with links to exact tasks/actions/results. Counts match after reconnect/restart; resolving one item cannot approve a different or changed action. |
| 8 | Operational summaries: show project progress, upcoming runs, blockers, recorded checks and failures, with evidence links. | Start with on-demand local reports using existing state, audit and schedule projections. Verify counts/drill-downs and project scope; do not invent tests or deliver reports externally. |
| 9 | Resource visibility: show recorded requests/tokens, remaining configured limits and clearly labeled cost estimates by task, agent, project and workflow run. | Reuse authoritative usage/budget records and trusted rate configuration. Preserve descendant attribution and distinguish fixed step limits from a pooled budget; unknown measurements remain unavailable and estimates are not monetary hard caps. |
| 10 | Neovim selection-to-task requests: select real code and propose explain/refactor/debug/test work from its file, range and captured buffer revision. | Reuse official Neovim, task forms and isolated execution. Physically test Unicode selection, cancellation, exact authorization and review; dirty/stale original buffers still block Apply. Captured context grants no extra paths/tools. |
| 11 | Tool/provider inventory: show configured capabilities, required permissions and separately recorded qualification status. | Reuse MCP/plugin/provider configuration; configuration alone does not establish compatibility. Verify secret redaction and accurate unknown/failed states without installs, paid probes or automatic model fallback. Selecting/qualifying an additional external integration remains N8's separate bounded scope. |
| 12 | Complete qualification throughout the batch: meaningful automated checks and actual CLI/TUI/official-Neovim workflows, with evidence and logical commits. | Use Ponytail during implementation/review. Exercise checkpoint → blocker → owner reassignment → remaining work → recorded checks → review → explicit Apply, including restart, stale-worker denial, retained budgets and recovery failures. Use fresh authorized fixtures and preserve historical runs. |

Keep the five layouts and existing concept-art palette/panels as these controls
are added. This batch does not introduce a new orchestration service, workflow
language or dependency by default. Automatic Apply, whole-workflow recurrence,
shared-buffer merging, user plugin/LSP qualification, additional paid provider
pilots and distributed workers retain their separate scope and acceptance gates.

Sequence is based on technical dependencies and risk, not date estimates.
Implementation proceeds within the approved scope and records evidence at each gate.
Approval of the complete MVP covers ordinary slices/commits; do not request it
again for each checkpoint. Broader authority or a material scope change is separate.
Parallel independent work is useful once interfaces and ownership are clear;
the owner has approved MVP implementation with Ponytail and periodic local commits.

| Phase                           | Scope                                                                                                                              | Exit evidence                                                                                                       |
| ------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| 0: research/planning            | Review inputs and source, architecture, risks, roadmap, MVP, AGENTS.md                                                             | Documentation reviewed; owner approves implementation boundary                                                      |
| 1: baseline and spikes          | Completed scoped CLI build, daemon lifecycle/approval reconnect and official Neovim external-UI qualification                      | Foundation evidence recorded; production RPC, canonical state, crash recovery and agent denial subsequently qualified in Phase 2 |
| 2: durable vertical slice (MVP) | Stable definitions, tasks/claims/budgets/approvals, bounded execution, recovery, five minimal layouts and real editor              | All [MVP criteria](mvp.md) plus interactive/restart/restore evidence pass                                           |
| 3: collaboration and knowledge  | Safe selective patch review, version-checked buffer proposals, scoped knowledge retrieval/promotion, handoffs, recurring schedules | Dirty-buffer conflict/undo tests, isolation and memory ACL tests, schedule/timezone/restart tests                   |
| 4: workflow/integration breadth | Reusable DAG templates, events/meetings, more providers/tools, capability registry, authorized human/external workflows            | Provider matrix, idempotency/reconciliation, explicit integration grants and measured usefulness                    |
| 5: scale and operations         | Larger registries, fair dispatch, recovery/soak tests, platform supervision, optional multi-process/remote workers                 | Benchmarks, quotas, audit completeness, failure isolation and restore drill at declared scale                       |
| 6: protected improvement        | Outcome-based role/model/workflow optimization, controlled self-modification proposals and rollback                                | Improvements measured against baseline; no weakening of protected controls                                          |

## Scope discipline

The proposed MVP build order is B1 durable definitions -> B2 one safe background
task -> B3 bounded delegation/context -> B4 five useful layouts -> B5 actual
Neovim Workspace -> B6 complete release qualification. Detailed steps and checks
are maintained in [the MVP plan](mvp.md#build-order-and-usable-checkpoints).
B1–B5 have delivered and locally qualified durable definitions, bounded background
execution/delegation, five layouts, actual embedded Neovim and exact reviewed Apply.
B6 native coding/document pilots, default-two execution, registry/navigation,
idle and snapshot restore passed. Four-active qualification caught a heartbeat
contention bug; the corrected current-binary native checks passed. Final editor
quiet-Follow, navigation and idle checks also passed. A05's bounded real GPT-5.5
pilots passed native checks, physical official-Neovim review and Apply, at a
$0.345433 official-rate estimate including failed attempts. The MVP is qualified
for its stated single-owner Linux/native-provider envelope.


Deliver a complete small workflow rather than a screen full of invented metrics.
Keep features behind normal supported flags/settings until qualification.
Every phase records what is implemented, observed, deferred, failed and unknown.

Suggested qualification envelope for initial MVP: one Linux host, one human owner,
1–1,000 persisted agents/groups with configurable depth, 2 active executions by
default and up to 4 in tests, one validated Responses-compatible provider,
one local project with a short dependency graph. Stored-agent and local deterministic measurements are recorded in
[release qualification](release-qualification.md), with real-provider pilots recorded
separately from deterministic scale/fault checks. Single-agent use must remain convenient.

Future production scope includes Windows/macOS behavior, LSP/plugins, broader non-code
projects, greater active concurrency and remote workers. Do not infer those from
Linux baseline checks. Host sleep/cloud shutdown prevents unattended execution
regardless of how many agents are configured.

## Add complexity only when triggered

- PostgreSQL/worker coordination: actual multi-host ownership or sustained SQLite
  contention beyond measured single-host targets.
- Dedicated broker: measured durable message/dispatch volume that the outbox cannot meet.
- Temporal: workflows needing its established multi-host durable timer/retry
  semantics and operations support, after comparing migration cost.
- Vector retrieval: scoped lexical retrieval fails representative knowledge tasks.
- CRDT/OT: users require simultaneous shared-buffer edits and controlled proposals
  plus workspace isolation prove inadequate.
- Graph-card rendering: indented tree/navigation is insufficient on real terminals.
- New model adapter: actual provider requirement not covered by tested Responses
  support; evaluate server compatibility before writing an adapter.

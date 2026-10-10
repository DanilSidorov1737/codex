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
319 automated tests and real CLI/official-Neovim interaction. Knowledge retrieval,
recurring schedules, hunks and staged repeated Apply remain later work.

## Recommended next slices

The owner requested an expanded plan based on `docs/Codex CLI Fork.txt` on
2026-10-10. The sequence below is proposed work after the qualified selective
Apply slice, not implementation authorization or a claim of delivered features.
Memory and recurring schedules remain the first two recommendations. The added
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
and logical local commits for each approved slice. N1 is the recommended next
implementation step; choose and record scope before starting application changes.
Distributed workers, multimodal production, meetings, full-environment backup
automation and protected self-improvement remain in their later roadmap phases.

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

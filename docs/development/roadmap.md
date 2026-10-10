# Phased roadmap

Current execution: Phase 2 MVP implementation and B1–B6 qualification are complete,
including real GPT-5.5 coding/delegation/document pilots. Later phases require
the next bounded scope to be chosen with the owner. See
[qualification evidence](qualification.md) and [status](status.md).

Owner-approved next slice (2026-10-10): TUI owner creation/authorization and
concept-art dashboard styling, implemented with existing RPC/widgets and being
physically qualified. This is a bounded usability continuation of the MVP;
selective Apply, knowledge retrieval and recurring schedules remain later work.

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

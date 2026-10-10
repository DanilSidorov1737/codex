# Phased roadmap

Current execution: scoped Phase 1 / M0–M1 foundation checks are approved and complete. See
[qualification evidence](qualification.md) and [status](status.md).

Sequence is based on technical dependencies and risk, not date estimates.
Each implementation phase requires an approved scope and evidence at its gate.
Parallel independent work is useful once interfaces and ownership are clear;
no feature implementation or installations are authorized by Phase 0.

| Phase                           | Scope                                                                                                                                  | Exit evidence                                                                                                               |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| 0: research/planning            | Review inputs and source, architecture, risks, roadmap, MVP, AGENTS.md                                                                 | Documentation reviewed; owner approves implementation boundary                                                              |
| 1: baseline and spikes          | Build real CLI on selected host; inspect daemon detach/approval lifecycle; official Neovim external-UI spike; IPC/policy/storage tests | Actual TUI input, real editor editing/resize, safe daemon detach, selected dependencies/licenses; revise plan if seams fail |
| 2: durable vertical slice (MVP) | Stable definitions, tasks/claims/budgets/approvals, bounded execution, recovery, five minimal layouts and real editor                  | All [MVP criteria](mvp.md) plus interactive/restart/restore evidence pass                                                   |
| 3: collaboration and knowledge  | Safe selective patch review, version-checked buffer proposals, scoped knowledge retrieval/promotion, handoffs, recurring schedules     | Dirty-buffer conflict/undo tests, isolation and memory ACL tests, schedule/timezone/restart tests                           |
| 4: workflow/integration breadth | Reusable DAG templates, events/meetings, more providers/tools, capability registry, authorized human/external workflows                | Provider matrix, idempotency/reconciliation, explicit integration grants and measured usefulness                            |
| 5: scale and operations         | Larger registries, fair dispatch, recovery/soak tests, platform supervision, optional multi-process/remote workers                     | Benchmarks, quotas, audit completeness, failure isolation and restore drill at declared scale                               |
| 6: protected improvement        | Outcome-based role/model/workflow optimization, controlled self-modification proposals and rollback                                    | Improvements measured against baseline; no weakening of protected controls                                                  |

## Scope discipline

Deliver a complete small workflow rather than a screen full of invented metrics.
Keep features behind normal supported flags/settings until qualification.
Every phase records what is implemented, observed, deferred, failed and unknown.

Suggested qualification envelope for initial MVP: one Linux host, one human owner,
1–1,000 persisted agents/groups with configurable depth, 2 active executions by
default and up to 4 in tests, one validated Responses-compatible provider,
one local project with a short dependency graph. These are proposed test targets,
not current limits or promises. Single-agent use must remain convenient.

Future production scope includes Windows/macOS behavior, LSP/plugins, non-code
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

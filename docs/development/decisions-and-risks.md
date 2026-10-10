# Decisions, risks, and approval gate

Current approval: the owner approved M0/M1 CLI/daemon/official Neovim qualification
with “Okay, let's do this”. Required local tool installation is authorized for
that scope. See [qualification](qualification.md). Broader decisions below remain
proposals until accepted or validated in the relevant implementation scope.

## Proposed decisions

| ID  | Decision / rationale                                                                     | Alternatives considered                                                                | Status                                    |
| --- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------- |
| D01 | Extend existing app-server daemon and runtime; one host first                            | New daemon/framework/microservices add duplicated lifecycle/tool handling              | Proposed                                  |
| D02 | Stable configurable agent/group definitions independent of threads; no fixed role levels | Fixed CEO/manager hierarchy contradicts requirements                                   | Proposed                                  |
| D03 | Separate authoritative SQLite DB using SQLx patterns and protected recovery              | Existing runtime DB may rebuild; remote DB adds operations too early                   | Proposed                                  |
| D04 | Transactional task state/leases/reservations/audit/outbox, bounded execution             | Full event sourcing/broker/workflow engine deferred pending scale evidence             | Proposed                                  |
| D05 | Reuse existing project IDs and thread goals; add planning records                        | Replacing either duplicates working functionality; cross-store reconciliation required | Proposed                                  |
| D06 | One TUI pane model with five presets; Operations tree primary                            | Five independent frontends or graphical replica unnecessary                            | Proposed                                  |
| D07 | Official Neovim subprocess, MessagePack external UI, safe isolated editing first         | PTY/external editor fallback; custom editor prohibited                                 | Proposed                                  |
| D08 | Defer LGPL nvim-rs; approve permissive codec/client after spike                          | License-policy exception requires explicit review                                      | Proposed                                  |
| D09 | Deterministic control/budgets/approval separate from optional AI governance roles        | Prompt-only rules cannot protect tools/DB/secrets                                      | Proposed                                  |
| D10 | Ponytail throughout implementation/review; current session skill reused                  | Upstream standalone plugin/hooks installation remains separate                         | Active process rule; installation pending |
| D11 | Local owner/Linux pilot, real tests and bounded provider requests                        | Immediate cross-platform/distributed/financial autonomy too broad                      | Proposed scope                            |

## Major risks and mitigations

| Risk                                                          | Consequence                                    | Mitigation / proof needed                                                           |
| ------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------- |
| Runtime and organization identity conflated                   | Retiring/restarting a thread loses roles/tasks | Stable IDs and explicit mappings; restart/reparent tests                            |
| Existing goal continuations/delegation bypass scheduler gates | Unaccounted work or authority expansion        | Trace every start path; negative direct/delegated/automatic/mailbox tests           |
| Agent can reach owner socket or DB                            | Prompt injection becomes administrative access | Protected paths/IPC scoped identities and sandbox denial tests                      |
| Runtime SQLite rebuild behavior reused for canonical data     | Silent loss of budgets/approvals/work          | Dedicated fail-closed authoritative DB and restore test                             |
| Cross-store project/history writes interrupted                | Orphaned tasks/projects or duplicate starts    | Idempotent intents/outbox and reconciliation with correlated runtime evidence       |
| In-memory mailbox treated as durable                          | Lost cross-session instructions                | Persist critical inbox/dedupe; discussions use qualified board adapter              |
| Board ACL assumptions                                         | Cross-project information disclosure           | Current templates lack ACL configuration; service-enforced scope before broad reuse |
| Parent waits while holding all execution slots                | Deadlock/starvation                            | Yield/checkpoint waiting work and fair scheduling                                   |
| Provider usage/price is delayed or unbounded                  | Claimed monetary hard cap fails                | Reservations/in-flight bounds, provider ceilings or reject strict-money workflow    |
| Replay after uncertain external effect                        | Duplicate payment/message/change               | Intent/receipt/idempotency/reconciliation; no financial or messaging MVP            |
| Human dirty buffers/concurrent agent writes                   | Lost edits or wrong patch acceptance           | Isolated task changes, exact base hashes and buffer revision checks                 |
| Neovim input/render compatibility                             | Lost keys, broken mode/cursor/Unicode          | Actual-editor spike and real interaction matrix                                     |
| RPC library LGPL/API instability                              | License/build/maintenance conflict             | Existing deny policy respected; smaller permissive alternative reviewed             |
| Thousands of stored roles mistaken for active capacity        | Memory/token/rate-limit overload               | Bounded residency/execution; measured staged scale targets                          |
| UI invents status/CPU/test results                            | Misleading completion/safety claims            | Canonical state, timestamp/source, explicit estimates and unavailable metrics       |
| Updates/suspended host disrupt unattended work                | Interrupted tasks despite daemon               | Pin pilot binaries, supervise approved host and reconcile restarts                  |
| Knowledge promotion leaks/poisons context                     | Wrong decisions or secret exposure             | ACL-filter first, provenance/freshness, controlled promotion and redaction          |
| Main CLI/native dependencies not yet qualified                | Planning hides runtime build blocker           | M0 build before feature work, inspect toolchain/targets/resources                   |
| Self-improvement modifies safeguards                          | Lost independent human control                 | Protected policy/recovery/update boundary and separate authorization                |
| Missing separate PDF                                          | Undiscovered requirements                      | Record limitation; compare if supplied before scope freeze                          |

## Defaults and unresolved choices

Reasonable planning defaults: one Linux host; single owner; one verified provider;
two active executions initially; optional stored organizations; no external
financial/messaging/deploy authority; user Neovim configuration supported after
clean-config qualification.

Choices for implementation approval:

1. Approve the MVP scope, reuse architecture and Linux-first qualification.
2. Authorize installation/verification of an official pinned Neovim release for M1,
   and review the minimal RPC dependency. This is not an editor implementation approval alone.
3. Decide whether the existing session Ponytail skill is sufficient initially, or
   authorize a pinned standalone Ponytail plugin and trusting reviewed hooks.
4. Approve a small bounded live-provider test, including provider/auth route and
   maximum budget, after inspecting available credentials.
5. Confirm initial editing semantics: isolated AI task changes with safe review,
   then version-checked shared-buffer proposals later.
6. Provide any separate PDF or clarify additional non-local deployment/platform
   needs before freezing requirements. Its absence does not invalidate reviewed text/images.

There is no need to decide on Temporal, Redis, vector databases or distributed
workers now. These remain conditional future choices.

## Gate

The user explicitly required: “Once the exploration, documentation, and MVP plan
are complete, present the findings and stop for my approval before beginning
implementation.”

This gate comes from the user's Phase 0 instruction. The owner has now approved
the foundation qualification described above. Record subsequent scope decisions;
do not infer external spending or operational authority from this local approval.
No additional setup approval flow is introduced here.

# Decisions, risks, and approval gate

Current approval: after M0/M1 qualification, the owner approved implementing the
MVP with Ponytail and periodic local commits. The B1–B6 scope, single-host reuse
architecture and isolated-editing approach are accepted for implementation.
Production dependency choices must satisfy repository policy. The owner supplied
OPENAI_API_KEY and approved compatible-GPT pilots with minimal spending; retain
the conservative $1 total working ceiling. Model listing authenticated and exposed
gpt-5.5. Official prices and all three live pilots subsequently passed; rate-based
token cost is $0.345433 including failed attempts. Provider invoice is unverified.

## Decisions

| ID  | Decision / rationale                                                                     | Alternatives considered                                                                | Status                                    |
| --- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------- |
| D01 | Extend existing app-server daemon and runtime; one host first                            | New daemon/framework/microservices add duplicated lifecycle/tool handling              | Accepted; implemented and locally qualified |
| D02 | Stable configurable agent/group definitions independent of threads; no fixed role levels | Fixed CEO/manager hierarchy contradicts requirements                                   | Accepted; implemented and locally qualified |
| D03 | Separate authoritative SQLite DB using SQLx patterns and protected recovery              | Existing runtime DB may rebuild; remote DB adds operations too early                   | Accepted; fail-closed state and restore qualified |
| D04 | Transactional task state/leases/reservations/audit/outbox, bounded execution             | Full event sourcing/broker/workflow engine deferred pending scale evidence             | Accepted; implemented and locally qualified |
| D05 | Reuse existing project IDs and thread goals; add planning records                        | Replacing either duplicates working functionality; cross-store reconciliation required | Accepted; implemented and locally qualified |
| D06 | One TUI pane model with five presets; Operations tree primary                            | Five independent frontends or graphical replica unnecessary                            | Accepted; implemented and locally qualified |
| D07 | Official Neovim subprocess, MessagePack external UI, safe isolated editing first         | PTY/external editor fallback; custom editor prohibited                                 | Accepted; implemented and locally qualified |
| D08 | Use MIT rmpv/rmp for the narrow native Neovim transport                          | LGPL nvim-rs rejected; no policy exception needed                                      | Accepted; implemented and locally qualified |
| D09 | Deterministic control/budgets/approval separate from optional AI governance roles        | Prompt-only rules cannot protect tools/DB/secrets                                      | Accepted; implemented and locally qualified |
| D10 | Ponytail throughout implementation/review; current session skill reused                  | Upstream standalone plugin/hooks installation remains separate                         | Active process rule; installation pending |
| D11 | Local owner/Linux pilot, real tests and bounded provider requests                        | Immediate cross-platform/distributed/financial autonomy too broad                      | Qualified Linux/local native GPT-5.5 scope |

## Major risks and mitigations

| Risk                                                          | Consequence                                    | Mitigation / proof needed                                                                       |
| ------------------------------------------------------------- | ---------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| Runtime and organization identity conflated                   | Retiring/restarting a thread loses roles/tasks | Stable IDs and explicit mappings; restart/reparent tests                                        |
| Existing goal continuations/delegation bypass scheduler gates | Unaccounted work or authority expansion        | Trace every start path; negative direct/delegated/automatic/mailbox tests                       |
| Agent can reach owner socket or DB                            | Prompt injection becomes administrative access | Protected paths/IPC scoped identities and sandbox denial tests                                  |
| Runtime SQLite rebuild behavior reused for canonical data     | Silent loss of budgets/approvals/work          | Dedicated fail-closed authoritative DB and restore test                                         |
| Cross-store project/history writes interrupted                | Orphaned tasks/projects or duplicate starts    | Idempotent intents/outbox and reconciliation with correlated runtime evidence                   |
| In-memory mailbox treated as durable                          | Lost cross-session instructions                | Persist critical inbox/dedupe; discussions use qualified board adapter                          |
| Board ACL assumptions                                         | Cross-project information disclosure           | Current templates lack ACL configuration; service-enforced scope before broad reuse             |
| Parent waits while holding all execution slots                | Deadlock/starvation                            | Yield/checkpoint waiting work and fair scheduling                                               |
| Provider usage/price is delayed or unbounded                  | Claimed monetary hard cap fails                | Reservations/in-flight bounds, provider ceilings or reject strict-money workflow                |
| Replay after uncertain external effect                        | Duplicate payment/message/change               | Intent/receipt/idempotency/reconciliation; no financial or messaging MVP                        |
| Human dirty buffers/concurrent agent writes                   | Lost edits or wrong patch acceptance           | Isolated task changes, exact base hashes and buffer revision checks                             |
| Neovim input/render compatibility                             | Lost keys, broken mode/cursor/Unicode          | Actual-editor spike and real interaction matrix                                                 |
| RPC library LGPL/API instability                              | License/build/maintenance conflict             | Existing deny policy respected; smaller permissive alternative reviewed                         |
| Thousands of stored roles mistaken for active capacity        | Memory/token/rate-limit overload               | Bounded residency/execution; measured staged scale targets                                      |
| UI invents status/CPU/test results                            | Misleading completion/safety claims            | Canonical state, timestamp/source, explicit estimates and unavailable metrics                   |
| Updates/suspended host disrupt unattended work                | Interrupted tasks despite daemon               | Pin pilot binaries, supervise approved host and reconcile restarts                              |
| Knowledge promotion leaks/poisons context                     | Wrong decisions or secret exposure             | ACL-filter first, provenance/freshness, controlled promotion and redaction                      |
| Native build/package setup regresses on a new host            | Runtime/helper or sandbox startup fails        | Reuse qualified pinned setup; fresh-task restoration and additional platforms remain unverified |
| Self-improvement modifies safeguards                          | Lost independent human control                 | Protected policy/recovery/update boundary and separate authorization                            |
| Missing separate PDF                                          | Undiscovered requirements                      | Record limitation; compare if supplied before scope freeze                                      |

## Defaults and unresolved choices

Reasonable planning defaults: one Linux host; single owner; one verified provider;
two active executions initially; optional stored organizations; no external
financial/messaging/deploy authority; user Neovim configuration supported after
clean-config qualification.

Remaining decisions and qualification boundaries:

1. A05 passed real GPT-5.5 coding, one-child delegation and local-fact documents,
   actual Neovim review and Apply. Broader provider/model quality and invoices
   remain unqualified. Token ceilings permit already-admitted request usage;
   the live delegated run reported 101 tokens above its 25,000 ceiling.
2. User Neovim configuration/plugins and additional operating systems require
   their own compatibility matrix. Official Neovim 0.12.6 clean embedding is
   already obtained and locally qualified; no repeated installation approval.
3. The session's Ponytail instructions remain the implementation/review process.
   Installing standalone upstream hooks is optional and separately authorized.
4. Shared-buffer proposals, remote/distributed workers, provider routing, broader
   integrations and external financial/messaging/deploy authority remain later scope.
5. Supply any separate PDF if it exists. The supplied text and all seven concept
   images were reviewed; no separate PDF has been found.

There is no need to decide on Temporal, Redis, vector databases or distributed
workers now. These remain conditional future choices.

The approved MVP and isolated editing semantics are implemented and qualified in
the stated envelope. Choose later roadmap scope explicitly; retain bounded grants,
verified prices and existing security controls for any future paid work.

## Gate

The user explicitly required: “Once the exploration, documentation, and MVP plan
are complete, present the findings and stop for my approval before beginning
implementation.”

This gate comes from the user's Phase 0 instruction. The owner has now approved
the MVP implementation described above. Record subsequent scope decisions;
do not infer external spending or operational authority from this local approval.
No additional setup approval flow is introduced here.

## B6 scheduling correction

Four-active native qualification exposed a heartbeat waiting for the single SQLite
connection while its task work was suspended inside the heartbeat select branch.
Poll work and renewal as independent futures in the same select so work can release
its transaction and stop/deadline remain responsive. Keep one database connection
and short transactions; adding a pool or a scheduler service would hide the bug.
The failure and rerun evidence belong in testing/release qualification.

# Security, resources, approvals, and recovery

## Enforceable trust model

Initial scope is a local single-owner deployment, not hostile multi-tenant SaaS.
The owner, protected service configuration and sandbox are trusted control
boundaries. Models, tool output, retrieved memory, repository content and external
messages are untrusted inputs. User-configurable role instructions provide
behavioral guidance, never credentials or authority by themselves.

Compute effective authority from the intersection of owner policy, organization,
project/task grants, delegation scope, role restrictions and execution environment.
Explicit denials win. A child cannot enlarge its parent's authorized task scope.
Re-evaluate live revocations; captured config versions must not permit continued
execution after a protected grant is withdrawn.

Keep the organization database, policy configuration, approvals, budget records,
audit files, recovery material, sockets and credential stores outside agent-write
roots. Agents use validated service commands to mutate allowed business fields.
If arbitrary shell tools can edit canonical DB files or invoke an owner-authenticated
control socket, the policy design has failed.

Use existing tool-policy ceilings, command policy, managed configuration,
filesystem/sandbox restrictions and network proxy. Restrict shell network routes
and credentials as well as high-level tools; hiding a purchasing tool while
leaving its token reachable by curl is not financial control.
Execution gates must apply to direct/delegated/resumed/automatic/mailbox work.

Use authenticated human and agent control channels with distinct capabilities.
Private owner socket/peer validation alone identifies an OS user, not which
agent process is calling. Agent access needs scoped capabilities plus sandbox
access restrictions; do not give every model the owner connection.
IPC transport authentication and cross-platform enforcement are an MVP spike.

Credentials stay in existing supported authentication/secret mechanisms or
OS protected storage and are passed only to authorized destinations/actions.
Store references, never values, in roles, notes, event payloads or documentation.
Record useful decision summaries and evidence; do not require private chain-of-thought.

## Human approval and review

Classify actions as allowed, notify-after, approval-required or forbidden.
The owner's policy chooses scopes; no global “auto approvals” UI switch can
weaken a protected ceiling. Agents may recommend authority changes, not apply
them. Audit/risk agents can escalate independently of the task author.

Bind each approval to principal, task/attempt, action hash, resource version,
policy version, destination/amount where relevant, approver and expiry.
Recheck those facts at execution. Changed patch, arguments, destination or
policy invalidates stale approval. Reject self-approval and unsupported approvers.

Persist pending approval independently of a connected client; unrelated work
continues. Existing JSON-RPC approval requests are useful presentation/execution
seams but are not assumed durable after server restart. Restart must safely
reconstruct/reissue an unresolved action, not auto-approve an old request ID.

A reviewed artifact is not a merge/deployment authorization. Review should show
the exact diff/artifact version, real tests and limitations. Apply locally only
after rechecking the destination base and dirty state. Merge, push, publish,
financial actions, account creation, installation and physical effects remain
distinct permission categories.

## Resources and budgets

Enforce host/workspace/project/task/agent concurrency and execution deadlines.
Track context/token consumption using existing usage events and per-thread goal
budgets; add durable shared reservations for organizational admission.

Use integer fixed units for currency. Reserve worst-case authorized consumption
before admitting an attempt, including child tasks, model/tool requests and
provider retries. Settle against observed provider usage, recording estimates
separately from billed/reconciled charges. Limits and resets have explicit
period/timezone semantics.

A provider that cannot bound a request cannot support a claimed exact spend cap.
MVP uses token/time/concurrency caps with a documented bounded in-flight allowance
and spending estimates; either use a provider-side hard limit or disable actions
requiring a strict monetary guarantee. Unknown usage remains reserved pending
reconciliation. Never release a reservation solely because the worker died.

Bound retry counts and message queues; avoid scheduled busywork when idle.
Metrics warn; deterministic gates enforce. No financial transactions or purchases
are authorized in the MVP.

## Failure handling

| Failure point                                   | Required behavior                                                                                   |
| ----------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| Before claim commit                             | No executable intent; task remains runnable                                                         |
| After claim, before runtime start               | Reconcile recorded start intent; idempotently map or requeue only after proving no run exists       |
| After runtime start, before thread/turn mapping | Use durable correlation/start intent and runtime evidence; uncertain execution stays reconciling    |
| During inference/tool execution                 | Stop stale attempts using fencing/revocation; preserve outputs and usage; assess side effects       |
| After external dispatch, before result save     | Mark unknown, query provider receipt/state; never blindly replay                                    |
| After result save, before UI event delivery     | Replay durable sequence/outbox; client deduplicates                                                 |
| During approval, client disconnects             | Persist request and wait; do not grant from timeout/silence                                         |
| Task lease expires                              | Fenced completion rejected; stop/reconcile old worker before conflicting retry                      |
| Database full/corrupt/unavailable               | Stop new consequential work, preserve artifacts and DB files; owner recovery mode                   |
| Editor/TUI closes                               | Detach UI; daemon keeps authorized tasks; dirty editor buffers need save/recovery handling          |
| Daemon/host stops                               | Reconstruct definitions/tasks; reconcile active attempts, approvals, reservations, pending dispatch |
| Suspend cloud instance                          | No promise of 24/7 execution; resume/reconcile when host runs                                       |

Exactly-once external side effects cannot be guaranteed by a local SQL transaction.
Use provider idempotency keys where supported, intent/receipt records and
verification. Non-idempotent uncertain operations require reconciliation or
human intervention. Fencing prevents stale state commits; it cannot un-send
an email or stop an uncooperative external process by itself.

Repeated unchanged errors back off and escalate with evidence. Do not repeatedly
ask for the same credential or silently rerun a failed action.

## Workspace integrity

Initial agent coding runs use explicitly approved isolated workspaces/branches,
with serialized application of reviewed changes to the user workspace.
Development tasks already have an isolated checkout: do not create worktrees
during Phase 0 or routine environment setup. Future product-managed task
workspaces are a separate feature requiring explicit policy and tests.

Human unsaved buffers, expected file hashes, rename/deletion/symlink changes and
concurrent patches are part of conflict detection. A file lock alone does not
detect a user's dirty buffer. If isolation is unavailable, serialize affected
file edits and fail safely rather than overwriting.

## Backup, audit and shutdown

Use SQLite online backup/checkpoint-aware tooling, not blind copying of a live
DB while ignoring WAL. Coordinate organization DB, captured project references,
thread histories and artifact manifests. Version schema and record hashes.
Test restoration into a separate fixture state directory and verify tasks,
approvals, grants and budget balances before resuming execution.

Application append-only audit permissions are useful but do not make history
tamper-proof against the host owner. Strong independent audit requires protected
external storage or signatures plus independent key custody, deferred until needed.

Keep owner emergency pause/revoke/shutdown access outside optional supervisors.
An OS-level stop must remain possible when the daemon is unresponsive.
Shutdown drains or cancels work within a bounded timeout and records incomplete
attempts. Automatic self-improvement can propose versioned changes and evidence;
it cannot alter protected security, budgets, recovery controls or approval rules.

Use systemd/launchd/Windows supervision when an approved platform needs reboot
startup; existing detached process behavior alone does not prove reboot survival.
Control updater settings for reproducible pilot runs so background updates do
not interrupt qualification or silently change policies.

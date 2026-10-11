# B3 bounded coordination

Status: B3 is locally qualified through state, native API and actual packaged
CLI/detached-daemon checks. Subsequent B6 real GPT-5.5 one-child delegation passed
checked artifact import, background inference with TUI closed, owner review and
Apply. Earlier deterministic evidence remains distinct from the live checkpoint.

The existing detached daemon owns coordination. Agents receive one `organization`
tool inside their isolated Codex runtime. They cannot select worker identity,
policy, paths, model or approval authority through this tool.

## Owner grant

An execution scope explicitly names `delegates` (configured arbitrary agent IDs)
and `maxChildren` (0–32 across the entire root, including descendants). Proposed
tasks also consume that count. Each child inherits the exact root scope; all
parent/child model requests and reported tokens debit the same root budget.
Actual usage may exceed a reported-token ceiling by already-admitted provider
requests; request counts remain atomic.

`notBefore`, optional UTC epoch seconds, schedules one start within 31 days;
claims remain blocked before that time. Queue/prerequisite limits can delay the
start. Grants record host policy version 2 and attempts pin their immutable role
version. Legacy version-1 grants require fresh owner authorization before running
under coordination-capable policy; updates never silently add tools to old grants.

`autoAcceptChildren`, default false, permits internal acceptance only when the
owner supplied nonempty host checks and every real check exits zero. The root
always requires owner review. Failed checks cannot satisfy prerequisites.

Every parent's role/policy, lease, elapsed budget and revoke state fence its
descendants. Pause denies new child model calls, native dispatch, coordination
and queued claims. Heartbeats cancel active work; late provider receipts still
charge usage without restoring authority.

## Agent calls

```json
{"type":"proposePlan","items":[{"key":"check","ownerId":"allowed-agent-id","instructions":"Produce the declared artifact","criteria":["Owner check passes"],"prerequisites":[]}]}
{"type":"delegate","taskId":"returned-task-id"}
{"type":"wait","taskIds":["direct-child-task-id"]}
{"type":"writeNote","scope":"project","body":"Observed project context"}
{"type":"readNotes","scope":"agent"}
{"type":"searchNotes","scope":"project","query":"SQLite"}
{"type":"searchNotes","scope":"agent","query":"previous decision"}
{"type":"send","taskId":"authorized-task-id","body":"Critical instruction"}
{"type":"inbox"}
{"type":"acknowledge","messageId":"received-message-id"}
```

Proposals validate the roster, size, unique keys and acyclic prerequisites in one
transaction; invalid plans roll back completely. Returned tasks are proposed
data; delegation creates their inherited grant. Calls deduplicate on the
host-bound attempt/call identity; changed arguments under the same identity fail.
Only the recipient can acknowledge a message. Inbox delivery survives until ack.
Restart reconciles interrupted attempts rather than replaying unknown work.

Project notes are filtered by the caller's authorized project before retrieval.
Agent notes also require the caller's own agent ID. Provenance includes author,
project, task, attempt and timestamp. Sending requires the same root and project;
a recipient's stable task inbox survives fresh owner authorization after restart,
with the original source-grant ID preserved. Tool results are marked as external,
untrusted context; their content
cannot become role instructions or authority. MVP bounds: 128 notes/project,
256 messages/root, 4 KiB bodies, 16 notes/read, 32 messages/read and 256 coordination
calls/attempt. Exhausted limits fail explicitly.

N1 `searchNotes` uses these same host-bound scopes and fences. Literal Unicode
lowercase substring matching searches the existing bounded store, returning
`notes` (at most 16) and `matchedCount`, with unchanged provenance. Empty queries
browse recent notes. Unknown identity fields are rejected; another project or
agent's private notes cannot be selected. `readNotes` retains its legacy array
contract. There is no automatic capture or permission-bearing memory. See
[memory search](memory-search.md) for the complete contract and current evidence.

## Waiting and files

### Approved checkpoint and handoff continuation

The owner-approved expanded batch adds structured checkpoints and owner-controlled
reassignment. Reuse the canonical store, attempts, grants and isolated-workspace
artifact importer. A checkpoint records the source attempt, reported progress,
remaining steps, an optional blocker and host-captured artifact evidence. Progress
text remains an agent report; actual action/check receipts are the verification
source. A blocker fences the attempt and frees its capacity only after native
runtime cleanup. Pending/uncertain effects and unresolved usage prevent handoff.

The owner reviews an exact task revision and immutable checkpoint before choosing
another permitted configured agent, or resolving the blocker with the same agent.
The old attempt remains fenced. Continue with a fresh attempt under the same grant,
preserving request/token/attempt counters, expiry, scope and schedule/workflow
accounting. Do not use ordinary reauthorization to reset those limits. The receiver
must match the captured model and current permitted node/project policy; active
descendants or changed policy require resolution before transferring ownership.
Imported checkpoint bytes must still match host hashes and retain their original
Apply bases. Handoff is not result acceptance and cannot unlock prerequisites.

Required evidence: checkpoint/reassignment restart persistence, stale-worker and
changed-checkpoint denial, no limit reset, uncertain-effect denial, independent
work continuing, scoped artifact continuity, actual CLI/TUI/official-Neovim review
and a separately confirmed Apply. This contract is approved but not yet delivered;
update this section with the concrete API and measured limits after qualification.

Parents wait only for direct delegated children, yielding an acyclic wait tree.
A waiting parent releases its active slot after any streamed provider request's
usage receipt is recorded. Resume reacquires a slot before further inference.
The parent remains durable and heartbeating throughout.

Default active execution remains two. At most four resident roots are admitted;
each has room for at most 32 descendants. The runtime tracker is bounded at 132
entries so waiting chains cannot crowd out a root's final child. This is separate
from active inference and does not imply 132 simultaneous calls or an OS resource
quota. Four waiting roots can delay further roots; children retain queue priority.

Children copy their parent's declared input snapshot. Accepted prerequisite
artifacts enter the next child's isolated directory; accepted direct-child
artifacts enter the parent when it resumes. Real bytes, size and hashes must match
accepted evidence. Original artifact bases remain frozen. Linux directory
descriptors, no-follow opens and atomic rename prevent copy paths following
agent-created links. Traversal, hard-linked files and oversized snapshots fail.
Imports never write to the original project; reviewed application remains B5.

## Evidence and remaining work

`mvp-b3-state-recovery-final.log`: 259 state tests passed, including eight
coordination tests. `mvp-b3-state-final-clippy.log`: production state Clippy passed
with warnings denied. `mvp-b3-wire-tests.log`: both wire tests passed;
`mvp-b3-python-contract-tests.log`: 25 Python contract checks passed.
Four runtime units and four actual app-server/native API tests passed. The
two-agent case used durable context and messages, yielded its parent's slot,
imported a real checked child file, ran the root check and required owner final
acceptance. Both views show the shared budget: 13 mocked-model requests and 195
reported tokens, with per-attempt usage retained separately. Final production
Clippy/formatting passed. Physical packaged CLI qualification passed 13 assertions, using 13 deterministic
local provider requests and 195 reported tokens. It exercised a scheduled root,
custom three-level hierarchy, delegation, yielded waiting, notes/inbox/ack,
real checks, isolated imports and owner final acceptance. Fixture services stopped.
Layouts and actual Neovim remain B4/B5;
live-provider and broader release qualification remain B6.

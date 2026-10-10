# B3 bounded coordination

Status: state implementation passes local tests; runtime integration and physical
CLI qualification are in progress. B3 is not yet qualified as complete.

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

## Waiting and files

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
The preliminary runtime/API compilation passed; the full current native fixture
and physical daemon workflow remain in progress. Layouts and actual Neovim remain B4/B5;
live-provider and broader release qualification remain B6.

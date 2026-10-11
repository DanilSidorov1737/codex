# Organization commands

The experimental organization API uses the existing local Codex app-server.
The CLI commands below use its normal daemon/session bootstrap. An explicit
remote endpoint is rejected. Stdio, the in-process host and the protected local
owner socket are accepted; TCP WebSocket and Remote Control are not.

```bash
codex organization list nodes --limit 100
codex organization mutate agent.json
codex organization project-create "My project" --root ./project --idempotency-key my-project-1
codex organization memory-search PROJECT_ID 'SQLite'
codex organization memory-search PROJECT_ID 'decision' --agent-id AGENT_ID
codex organization schedule-list --project-id PROJECT_ID
codex organization schedule-mutate recurring.json
codex organization backup
codex organization restore --backup /absolute/path/snapshot.sqlite --destination /fresh/sqlite-home
```

`list` accepts `nodes`, `roles`, `goals`, `projects`, `tasks`, or `memberships`.
Follow `nextCursor` using `--cursor`; page limits are 1–200. Output is JSON.
`--no-daemon` uses the existing embedded server for isolated qualification.

## Checkpoints and blocked handoffs (qualification in progress)

`codex organization execution-read TASK_ID` returns canonical checkpoints and
handoffs with the normal task/grant/result evidence. In the TUI, select a blocked
task and press `H`: choose a permitted receiver, enter the owner resolution,
press F6 to review and `y` to confirm. Escape preserves the draft without sending
an owner decision. Reported checkpoint progress remains distinct from completed
action IDs and host-hashed files.

The matching CLI owner mutation is:

```json
{
  "type": "handoff",
  "taskId": "blocked-task-id",
  "expectedRevision": 5,
  "checkpointId": "immutable-latest-checkpoint-id",
  "ownerId": "permitted-receiver-id",
  "expectedOwnerRevision": 1,
  "instructions": "Owner resolved the blocker; finish only the remaining work."
}
```

Submit with `codex organization execute handoff.json`. This is continuation under
the original grant, not ordinary authorization. The receiver gets a fresh isolated
attempt with verified checkpoint bytes; counters, original artifact bases, expiry
and permissions remain unchanged. Busy runtimes, stale records, changed files,
unresolved usage/effects or exhausted limits reject the handoff. Acceptance and
Apply stay separate owner decisions. See [continuity](coordination.md#approved-checkpoint-and-handoff-continuation)
and [current qualification](status.md).

`memory-search` reads existing durable notes; omit the query to browse recent
notes. The default scope is project-shared; `--agent-id` selects that agent's
private notes within the named project for owner inspection. Output wraps
`notes` and `matchedCount` under `result`, with original note IDs, author,
task/attempt and timestamp. Literal queries use Unicode lowercase, at most 256
UTF-8 bytes and no control characters; up to 16 notes are returned. Notes are
untrusted context, never authorization. See [memory search](memory-search.md)
for agent retrieval, scope boundaries, limits and actual qualification evidence.

The mutation file is at most 128 KiB and uses camelCase fields. Create a
standalone agent with:

```json
{ "type": "putNode", "kind": "agent", "label": "Researcher", "enabled": true }
```

Create a group with `kind: "group"`. Optional `parentId` creates any primary
forest shape; parents can be agents or groups. `setMembership` adds an extra
group membership without changing the primary parent. Groups do not execute.
Returned IDs are stable. Updates must supply the existing `id` and exact
`expectedRevision`; omitted optional fields are cleared. Kind is immutable,
and a hierarchy cycle is rejected atomically.

```json
{
  "type": "createRoleVersion",
  "name": "Investigator",
  "instructions": "Read local sources and cite evidence."
}
```

To revise a role, supply its `roleId` and `expectedVersion`; the result is a
new immutable role-version ID. Assign `roleVersionId` to an agent explicitly.
Role instructions, hierarchy and memberships grant no additional authority.

```json
{
  "type": "putGoal",
  "objective": "Prepare a checked report",
  "criteria": ["Cites supplied sources."]
}
```

Goals support optional `parentId` and version-checked edits. Create a project
using the existing project API, then link its exact returned ID:

```json
{
  "type": "linkProject",
  "projectId": "EXISTING_PROJECT_ID",
  "goalId": "GOAL_ID",
  "ownerId": "AGENT_ID",
  "criteria": ["Owner accepts the cited report."]
}
```

The server reads the existing project record and captures its name/roots.
This is a link, not a second project identity. Project relinking requires its
current `expectedRevision`. Execution validates the captured roots against the
existing project record; changed or missing references stop execution. Full-host recovery still requires
both business and runtime/project stores.

```json
{
  "type": "putTask",
  "projectId": "EXISTING_PROJECT_ID",
  "ownerId": "AGENT_ID",
  "instructions": "Read the supplied files and prepare a report.",
  "criteria": ["A cited report exists."],
  "prerequisites": []
}
```

Tasks are proposed definitions in B1 and do not start a model. Prerequisites
must exist in the same project and be acyclic. Only proposed tasks can be
edited; they cannot be moved between projects. Each successful mutation and
its audit reference commit together. Execution, grants, budgets, approval and
artifact acceptance are implemented as described below.

Snapshots are consistent, verified and stored under the protected organization
directory. Restore copies a verified snapshot into a fresh destination; it
never overwrites existing organization state. Restore does not recreate the
separate runtime/thread/project database. Preserve both stores when recovering
a complete host, and reconcile their references before execution.

Linux directory/file ownership and modes are checked, including symlinks and
hard links. B2 native tests qualified same-user agent sandbox exclusion of canonical
state, owner home/socket and unrelated projects. File modes alone are not a boundary
against an unsandboxed same-user process.

## B2 execution commands

Execution uses the detached local daemon. Start it with the same configuration,
`CODEX_HOME` and `CODEX_SQLITE_HOME` as the owner commands:

```bash
codex app-server daemon start
codex organization execution-read TASK_ID
codex organization execute authorize.json
```

`execute` reads a typed execution mutation from a JSON file of at most 128 KiB.
It rejects `--no-daemon` and an embedded-server fallback. Definition commands
remain available without background execution. The RPC counterparts are
`organization/execution/read` and `organization/execution/mutate` with experimental
API capability enabled. They accept protected local owner origins only.

```json
{
  "type": "authorize",
  "taskId": "TASK_ID",
  "expectedRevision": 1,
  "scope": {
    "inputFiles": ["source.txt", "oracle.py"],
    "immutableFiles": ["oracle.py"],
    "artifacts": ["report.txt"],
    "checks": [["python3", "oracle.py"]],
    "model": "gpt-5.4",
    "maxTokens": 20000,
    "maxSeconds": 120,
    "maxModelRequests": 2,
    "maxAttempts": 1,
    "requireToolApproval": true,
    "delegates": [],
    "maxChildren": 0
  }
}
```

The example model is used by deterministic local tests; live-provider compatibility
must be qualified separately. `maxTokens` covers reported usage and admits at most
one unreported request per attempt; a request already in flight can exceed it.
Provider retries are disabled. These limits are not an exact money or OS-memory cap.
Linux/local execution is the current operating envelope. The owner boot setting
defaults to two active attempts and allows 1–4. Four resident roots, each with at
most 32 descendants, bound resident attempt workers
to 132. Waiting parents yield active capacity after reported usage is recorded;
see resource-limits.md and coordination.md.

The owner selects explicit project-relative inputs, artifacts and check argv.
Inputs are snapshotted into a private attempt workspace; the original project and
owner data/state remain unavailable in the native sandbox. Immutable inputs
remain read-only. Agent tools are restricted to native `exec_command` and
`apply_patch`; network, inherited credentials, remote environments and privilege
expansion arguments are denied. The daemon's normal private-home helper installation is supported by granting
read access to exact executable files, never their sibling data. Install helpers
outside the original project. A protected directory containing helpers that
intersects system default read roots is rejected; a blanket deny cannot safely
reopen a read-only helper on this native backend.
Artifacts are bounded regular files with real hashes; checks run in the same
restricted workspace. Input limits are 8 MiB/file and 64 MiB total. Native helpers
receive only the exact read access needed to run.

Inspect `execution-read` for the current task revision, authorization, attempts,
exact approvals/actions, usage and result. Decide a pending approval with its
exact returned ID/hash; silence does not approve it:

```json
{
  "type": "decideApproval",
  "taskId": "TASK_ID",
  "approvalId": "APPROVAL_ID",
  "expectedHash": "RETURNED_ACTION_HASH",
  "approve": true
}
```

An interrupted attempt enters `needsReconciliation`; dispatched actions become
`uncertain` and are not replayed. Pending approvals remain recorded but their old
lease cannot be accepted. Pause/cancel revokes authority immediately and retains
the active slot until native cleanup is joined. Restart reconciles before execution
reads/decisions, so an unexpired old approval cannot revive a crashed attempt.

```json
{ "type": "stop", "taskId": "TASK_ID", "expectedRevision": 3, "cancel": false }
```

Successful artifact/check collection produces `needsReview`, not acceptance.
Final acceptance references the exact returned evidence digest and current task
revision; failed checks cannot be accepted. Acceptance does not apply files to
the original project. Explicit safe file application is a separate owner operation,
documented below.

```json
{
  "type": "accept",
  "taskId": "TASK_ID",
  "expectedRevision": 4,
  "digest": "RETURNED_RESULT_DIGEST"
}
```

For revision work, use `requestChanges` with `taskId`, `expectedRevision` and bounded
`instructions`, then issue a new explicit authorization. Delegation and scoped
notes are implemented; see [coordination](coordination.md). Authorization names
the exact `delegates` roster and `maxChildren` bound. Optional `autoAcceptChildren`
authorizes only machine-checked internal child acceptance; final root acceptance
remains owner-only. All descendants share the root budget.

## Apply an accepted result

Read the accepted task's current revision/digest, then use another owner mutation.
Omit `paths` for legacy apply-all, or select exact whole-file artifact paths:

```json
{
  "type": "apply",
  "taskId": "TASK_ID",
  "expectedRevision": 5,
  "digest": "EXACT_ACCEPTED_RESULT_DIGEST",
  "paths": ["report.txt"]
}
```

`codex organization execute apply.json` verifies the selected accepted bytes and
original base hashes before a protected durable intent and per-file atomic replacement.
Explicit empty, duplicate and unknown selections are rejected before an intent/write.
Unselected files are untouched. The receipt records exact selected paths, and there
is still one Apply per accepted result: choosing a subset consumes that application.
Full-result checks do not certify the installed subset; inspect/test that composition
separately. See [selective Apply](selective-apply.md) for scope and current evidence.
An application receipt is separate from acceptance; completed/uncertain receipts
block replay. Close or explicitly coordinate other editors/writers first: a CLI
client cannot inspect the TUI's dirty buffers. The embedded Workspace also fences
its native Neovim buffers before dispatch. See neovim-workspace.md for conflicts,
partial-batch/unknown-outcome recovery and the actual editor keyboard controls.

## Reusable workflows (N3)

```bash
codex organization workflow-list templates --project-id PROJECT_ID
codex organization workflow-list runs --project-id PROJECT_ID
codex organization workflow-mutate template.json
codex organization workflow-preview preview-request.json
codex organization workflow-mutate exact-run.json
```

Lists accept `--cursor` and `--limit` (1–200), with `data` and `nextCursor`.
Mutations require the detached local owner daemon; `--no-daemon` is refused.
Saving content authorizes no execution. Example `template.json`:

```json
{
  "type": "put",
  "id": "OWNER_GENERATED_UUID",
  "expectedRevision": null,
  "projectId": "PROJECT_ID",
  "label": "Inspect and implement",
  "items": [
    {"key": "inspect", "ownerId": "AGENT_ID", "instructions": "Inspect the supplied bug and save a plan.", "criteria": ["Plan explains the bug."], "prerequisites": []},
    {"key": "implement", "ownerId": "OTHER_AGENT_ID", "instructions": "Implement the accepted plan.", "criteria": ["Immutable native checks pass."], "prerequisites": ["inspect"]}
  ]
}
```

Keys, instructions, criteria, configured agent assignments and dependencies are
editable content, without predefined roles. Templates contain 1–32 steps in one
project; update using the same ID and exact current `expectedRevision`.
The daemon rejects unknown/duplicate prerequisites and cycles atomically.

`preview-request.json` contains a fresh owner UUID `id`, the saved `templateId`,
its exact `expectedRevision`, and `scopes`: one `{"key":"inspect","scope":{...}}`
entry per step. Each scope uses the existing bounded authorization contract:
model, input/immutable/artifact paths, native checks, tool-approval policy,
request/token/time/attempt limits and optional permitted delegates. Explicitly
include a prerequisite artifact in the receiving step's `inputFiles` to import
its accepted bytes. Imports cannot replace immutable inputs; ambiguous producers
are rejected. No other run's workspace is imported.

Read the returned `preview` before authorizing. Write `exact-run.json` as
`{"type":"run","preview":THE_EXACT_RETURNED_PREVIEW}`. Confirmation pins the
template/project/agent revisions, policy, assignments, criteria, scopes and
summed limits. Fresh tasks, grants and dependency links commit together. A stale
or altered preview fails; request a fresh preview before confirming. A duplicate
run ID fails: after an uncertain RPC outcome, inspect that ID instead of silently
creating another invocation. Editing a template does not retarget old runs.

The sums describe fixed per-step grants, including descendant accounting; they
are not pooled token credits or a monetary hard cap. An admitted request may
overshoot its token ceiling. Every prerequisite requires explicit owner acceptance
before dependent dispatch. Failed, interrupted, expired or cancelled steps block
their dependents; ordinary reauthorization cannot reset workflow limits. Use
existing per-task pause/cancel, inspect uncertain effects and explicitly preview
a fresh invocation when appropriate. No automatic acceptance, Apply or replay.

In Projects, select a final task and `W` saves its loaded prerequisite closure.
Select a template and `s` opens a common per-step scope form; F6 obtains a read-only
canonical preview and `y` confirms that exact payload. CLI permits different
scopes per step. `w` browses templates/runs and their tasks in Projects/Operations;
Operations still opens its hierarchy tree by default. Select run tasks to use
existing approvals, checks, actual Neovim Follow/diff, acceptance and selective
whole-file Apply. Later edits to imported files carry the producer's original
base, so a user edit between steps still prevents Apply. See
[workflow contract and evidence](workflows.md).

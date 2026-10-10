# Organization commands

The experimental organization API uses the existing local Codex app-server.
The CLI commands below use its normal daemon/session bootstrap. An explicit
remote endpoint is rejected. Stdio, the in-process host and the protected local
owner socket are accepted; TCP WebSocket and Remote Control are not.

```bash
codex organization list nodes --limit 100
codex organization mutate agent.json
codex organization project-create "My project" --root ./project --idempotency-key my-project-1
codex organization backup
codex organization restore --backup /absolute/path/snapshot.sqlite --destination /fresh/sqlite-home
```

`list` accepts `nodes`, `roles`, `goals`, `projects`, `tasks`, or `memberships`.
Follow `nextCursor` using `--cursor`; page limits are 1–200. Output is JSON.
`--no-daemon` uses the existing embedded server for isolated qualification.

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
current `expectedRevision`. Cross-store execution reconciliation remains B2.

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
artifact acceptance follow in B2/B3.

Snapshots are consistent, verified and stored under the protected organization
directory. Restore copies a verified snapshot into a fresh destination; it
never overwrites existing organization state. Restore does not recreate the
separate runtime/thread/project database. Preserve both stores when recovering
a complete host, and reconcile their references before execution.

Linux directory/file ownership and modes are checked, including symlinks and
hard links. Same-user agent sandbox exclusion is a separate B2 requirement;
file modes alone are not a boundary against an unsandboxed same-user process.

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
Linux/local execution is the current operating envelope. Two active slots and at
most sixteen live attempt workers bound the daemon queue.

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
the original project. Explicit safe file application follows in B5.

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
notes are B3 work; nonempty delegation fields are not yet an implemented workflow.

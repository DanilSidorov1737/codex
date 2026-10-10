# Organization definitions: B1 commands

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

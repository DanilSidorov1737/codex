# Scoped persistent memory search

Status: N1 is owner-approved; implementation and qualification are in progress.
This extends the existing durable coordination notes rather than introducing a
new memory service, vector database or automatic knowledge promotion.

## Search contract

- Query the note body using a literal substring after trimming the query and
  applying Rust Unicode lowercase to both query and body. This is not stemming,
  semantic similarity or full Unicode case folding. `%`, `_` and SQL-like text
  have no wildcard/query-language meaning.
- Queries are at most 256 UTF-8 bytes and contain no control characters. An
  empty or whitespace-only query lists recent notes.
- Existing storage bounds remain 128 notes per project across both scopes and
  4 KiB per note. Search filters project/scope/agent in SQLite before decoding
  or matching, then scans at most 128 scoped records. A scope exceeding that
  ceiling fails explicitly. No schema migration or dependency is added.
- Return `notes` (up to 16, newest ID first) and `matchedCount` for the matching
  scoped records. The count makes truncation visible. Use a narrower query to
  find older notes; there is no search pagination in this slice.
- Each note retains its ID, project, scope, author, source task/attempt,
  timestamp and original body. These are source references, not certification
  of the note's claims. Bodies can include evidence/document references.

## Agent retrieval and authority

```json
{"type":"searchNotes","scope":"project","query":"SQLite"}
{"type":"searchNotes","scope":"agent","query":"previous decision"}
```

Use the existing `organization` tool. The host binds the current task's project
and agent identity; unknown fields such as `projectId`, `agentId`, `attemptId`
or `worker` are rejected. Project searches include project-shared notes; agent
searches include only that agent's private notes within the same project.
Private notes in another project are excluded even for the same agent.

Existing attempt/worker/lease/role/policy/pause/revoke/budget guards, the
256-call limit and exact call-identity deduplication apply. Cached search calls
cannot revive expired authority. Fresh authorized tasks can retrieve earlier
notes after restart without sharing the earlier task's grant. Legacy `readNotes`
still returns its original recent-note array; `writeNote` remains explicit.
Search adds no new write or cross-scope authority to the existing note capability.

Tool results remain untrusted context. A note cannot change instructions,
permissions, budgets or approvals. Automatic capture, project-wide promotion,
editing/retention and larger storage quotas are later scope.

## Owner CLI and TUI

```sh
codex organization memory-search PROJECT_ID 'SQLite'
codex organization memory-search PROJECT_ID
codex organization memory-search PROJECT_ID 'decision' --agent-id AGENT_ID
```

The experimental typed RPC is `organization/memory/search`, with `projectId`,
`scope`, optional/null `agentId`, and `query`; the response wraps the result under
`result`. Project searches reject an agent selector; private searches require an
existing agent. Only the protected local owner origins can use it. The owner
can inspect a selected agent's private notes; agents use their separately fenced
tool, never this administrative endpoint. Remote network clients are denied.

In Projects (F3), select a project or task and press `m` for project memory.
Select a task and press `M` for its owner's private memory. Task selection in
Workspace and Review supports the same keys. Operations shows organization
nodes; switch to Projects to select an explicit memory scope first.
Type a literal query and press Enter; empty Enter browses recent notes. The
shared memory view shows the captured project/agent, count, full note bodies and
source IDs/timestamps. `/` edits the query, `f` refreshes, arrows/PageUp/PageDown
scroll, and Esc closes the memory view or query entry. Then normal layout keys
resume. Workspace uses host focus; Ctrl-G leaves actual Neovim focus first.

Search requests capture their scope. Results for an earlier query or closed view
are ignored; changing selected records never silently retargets an open search.
Memory is a shared view within the five layouts, not a sixth layout. Existing
palette and text sanitization are reused; no editor or Apply semantics change.

## Qualification

See [testing](testing.md) and [status](status.md) for actual outcomes. Required
checks cover old-note discovery beyond the recent 16-note window, Unicode and
literal matching, count/limit visibility, provenance, fresh-task retrieval after
restart, other-project/private-agent exclusion, invalid/spoofed identities,
paused/stale calls, wire/schema/SDK contracts, and actual CLI/TUI operation.
Deterministic localhost provider evidence remains distinct from live-model work.

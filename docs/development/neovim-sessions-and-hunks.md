# Native sessions, exact hunks and buffer proposals

Tasks 16–18 use actual official Neovim and the existing accepted-result Apply
ledger. Implementation and actual restart/full-restore qualification pass; see
[the native evidence](operations-native-qualification.md) and current status. No editor engine, new service or new downloaded dependency is
introduced. The protocol reuses the workspace's existing diffy library so both
frontend previews and backend verification share the same canonical selection.

## Session checkpoints

In Workspace with host focus (Ctrl-G), F7 captures original project files,
native split structure, window dimensions, cursor positions and displayed unsaved
text into a private owner-controlled `organization-editor-session.json`.
Checkpoints support one tab, at most 16 windows/files, 1 MiB per file/buffer and
4 MiB total. Follow/diff previews, special buffers, hidden unsaved buffers, aliased
paths and paths outside the current project roots refuse. Neovim's native file
load records the disk base; changed originals refuse saving/restoration.

F8 opens an owner choice: restore saved files only, or explicitly recover captured
unsaved text into native buffers. Both paths retain the checkpoint and write no
original files. Unsaved recovery uses native undo; saving is a separate native
owner command. Restarted editor IDs are fresh. Corrupt, stale or unavailable
checkpoints remain for manual reconciliation. This is a data checkpoint, never a
replayed mksession/Ex script. Configuration and plugin behavior remain owner-selected.

Clean frontend exit attempts a checkpoint when no earlier unsaved checkpoint
would be overwritten. F7 explicitly advances it. A process crash can recover only
the last completed checkpoint, not edits made since it; this is not a keystroke
journal or a replacement for the owner's file backups. Full runtime bundles do
not include this private frontend checkpoint; retain it separately with the
original files/configuration when moving homes.

## Exact hunk Apply

h on an accepted task captures verified current bases and accepted artifact
bytes, then offers individual hunks from the canonical diff. F6 shows the exact
result/task revision, paths, ascending hunk indices, complete-patch hash and
selected-output hash; y submits the same captured selection. The existing A
whole-file form remains available.

The backend reconstructs the canonical selection from verified artifact bytes
and unchanged originals, rejects stale/invalid hashes and indices, then records
one durable application intent containing the selection. All original bases are
rechecked before writes and before each atomic replacement. Hunk Apply supports
existing nonbinary UTF-8 files up to 1 MiB; additions/binary/large files use the
existing whole-file path. An unchanged result has no hunks.

A partial selection still consumes the one application for that accepted result.
A second Apply, uncertain replay or staged top-up is refused. The acceptance
checks describe the full isolated result and do not certify its installed subset.
An interruption across multiple files can be partial/uncertain; inspect the receipt
and originals through the existing recovery workflow. Native dirty-buffer/frozen
input guards remain. Uncoordinated external writers are not a filesystem transaction.

## Pinned native buffer proposals

In Workspace host focus, B proposes the accepted artifact to the current writable
original buffer. It captures editor-session UUID, native buffer ID, path,
changedtick and exact text hash. The clean buffer must match the current original
and either the task's original base or a completed exact hunk receipt. Changed
owner files require reconciliation rather than being silently replaced. Current
support requires UTF-8/LF text up to 1 MiB; binary and nonmatching native file-format
representations refuse.

The exact confirmation changes the native buffer only. The native main-loop
operation rechecks identity/path/tick/hash/dirty/read-only state and uses native
buffer mutation and undo. A closed, renamed, edited or restarted target refuses;
undo increments the revision and cannot make an old proposal replayable. Agent
work stays in its isolated workspace. No autosave or second application to the
original file occurs; owner inspection, native undo and explicit :write remain
separate editing actions.

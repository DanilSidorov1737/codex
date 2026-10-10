# Actual Neovim Workspace

B5 is in progress. This document records delivered contracts separately from
qualification; the editor/application gates are not passed until recorded below.

## Integration

Workspace launches the actual `nvim` on PATH with `--embed -u NONE --noplugin
-i NONE`, disables modelines/exrc and attaches the official external line grid.
It preserves the owner's Neovim configuration on disk. Clean configuration is the
initial qualified mode; user-config/plugin compatibility remains a later gate.
Neovim owns text, modal commands, undo, splits, diff and file writes. Ratatui
renders native cells/highlights/cursor. No custom editor or Vim clone is used.

The narrow transport uses MIT `rmpv = 1.3.1` and its MIT `rmp = 0.8.15` dependency
(existing num-traits). Both require Rust 1.85, below this checkout's 1.95 toolchain.
Sources: https://github.com/3Hren/msgpack-rust. LGPL nvim-rs was not added and the
repository's license policy is unchanged. Only these codec dependencies are new;
unrelated Windows dependency consolidation from Cargo was removed from the lock.

One child has bounded eight-entry pipe queues, a 16 MiB message ceiling, depth 64,
512×256 cell ceiling and a three-second RPC deadline. Partial native input receipts
are checked. Nested editor requests cannot invoke owner/daemon administration.
Closing the frontend kills only its editor child; background tasks remain in the
daemon. The clean MVP editor disables swap files: saved files survive, unsaved
buffers are lost on crash/close. Save before leaving the frontend; normal host exit refuses dirty loaded buffers. Persistent
supervised editor sessions and crash restoration of unsaved buffers are deferred.

## Keyboard and files

Host Workspace controls: `e` enters/refocuses Neovim, `o` opens the selected task's
file in the original project for independent editing, `t` opens a protected snapshot of the isolated
attempt file read only for Follow AI, `d` opens native diff splits, and `n` cycles
through declared artifacts/inputs. `Ctrl-G` returns focus to the host. While editor
focus is active, ordinary keys, escape, digits, function keys, paste and mouse go
to Neovim; host layout shortcuts do not steal its editing keys.

Neovim never opens an agent-writable artifact directly. A shared directory-FD
reader rejects ancestor/leaf symlinks, nonregular files and hard links; reads are
bounded to 8 MiB. The exact bytes are atomically staged in an owner-only preview
directory under CODEX_HOME, which the agent sandbox excludes. Recorded artifact
hashes/sizes are checked before showing reviewed output. Follow writes only when
content changes, avoiding a redraw/reload feedback loop.

Follow reload is suppressed while editor focus or any dirty loaded buffer exists.
Native checktime retains the current window view. Task selection does not
implicitly replace an open editor file. Host-selected files reject traversal and
symlink redirection; native commands entered by the human retain human authority.
Mode changes reject dirty buffers instead of discarding changes.

## Application contract being qualified

Owner-only Apply captures the exact accepted task revision/digest. It validates
all artifact bytes, original SHA256/absence, relative paths, protected state and
.git exclusions before persisting an intent. Directory-descriptor-based private
staging and synchronized atomic replacement reuse the existing snapshot writer.
A separate application record distinguishes pending, completed and uncertain.
Interrupted intents become uncertain at startup and cannot be replayed.

The TUI checks all loaded buffers, captures native changedticks and freezes their
modifiable options in one Neovim main-loop operation before sending Apply. It
blocks editor input until the owner receipt, then checks ticks and restores the
previous options. Unknown outcomes remain frozen until manual inspection/restart.
Other owner CLI clients cannot see this frontend’s dirty buffers; they must be
used only with editors closed or explicitly coordinated by the owner.

Each file rename is atomic; the batch is not a filesystem transaction. Stop
external editors/writers during application. Original hashes are rechecked just
before replacement, but arbitrary processes ignoring coordination can race a
filesystem check/rename. A mid-batch failure may leave some files applied and
is explicitly uncertain. Resolve manually against recorded accepted hashes;
never blindly replay. No shared-buffer CRDT or arbitrary external-writer lock
protocol is claimed.

## Pending gates

The first two real-editor/encoding tests passed before adding application fences.
Repeat the augmented real integrated save/undo/Unicode/diff/resize/focus/follow/dirty-buffer checks,
then physical CLI interaction. Implement owner-only exact accepted-result
application with durable intent/receipt, original-base and artifact hash checks,
editor revision/dirty fence and fail-closed crash handling. Applying a result is
separate from accepting it. Record disk/alias/conflict and daemon independence
checks before calling B5 qualified.

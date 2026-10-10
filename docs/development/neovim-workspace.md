# Actual Neovim Workspace

B5 is locally qualified with official Neovim v0.12.6, the complete packaged CLI,
actual detached daemon and native sandbox. The provider replies were deterministic
localhost fixtures; live-model release qualification remains separate.

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
content changes. The pipe reader wakes the UI for native notifications, requests
and EOF/errors; synchronous RPC replies are consumed without requesting another
frame. This prevents the Follow dirty-buffer query from creating an idle redraw
loop. A real native regression checks query silence and redraw/EOF wakes.

Follow reload is suppressed while editor focus or any dirty loaded buffer exists.
Native checktime retains the current window view. Task selection does not
implicitly replace an open editor file. Host-selected files reject traversal and
symlink redirection; native commands entered by the human retain human authority.
Mode changes reject dirty buffers instead of discarding changes.

## Application contract

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

## Qualification

The final combined batch passed sixteen TUI/editor/preview/pricing tests, including
actual Neovim, direct frozen-input guards and query/redraw/EOF wake behavior. All
four native organization API cases passed with the final source: exact evidence,
original/artifact conflict denial, explicit Apply and duplicate receipt rejection,
restart fencing and delegated work. State regression passed all 259 tests; current
organization state cases remain passing. Production Clippy and affected formatting
passed.

The complete packaged CLI passed twelve physical assertions: modal Unicode edit
and save, actual bracketed paste and undo, dirty host exit and Apply refusal,
changed original preservation, protected read-only Follow, native diff splits,
real 50×20/120×30 resizing, exact accepted hash/Apply receipt and owner oracle.
SIGKILL of only the editor during a native 15-second command left the daemon alive;
the task reached NeedsReview and saved files reopened in a fresh editor.
Evidence: `/workspace/.onboarding/qualification/mvp-b5-cli/qualification.json`,
`tui-editor.log`, `tui-apply.log`, owner views and preserved human text. Final API
log: `mvp-b6-native-api-tests.log` (four passed, no failures/ignored). Earlier test
helper/error-ordering and fixture-wait failures remain documented in testing.

The final complete package and resource controls passed B6 local qualification.
User Neovim plugins/configurations, unsaved crash restoration, arbitrary concurrent
external writers and shared-buffer collaborative merging remain unqualified or
deferred as described above; none is claimed by this clean isolated MVP.

Actual staged native Follow also passed: two exact owner-approved file writes
changed phase one to phase two; the focused native buffer retained phase one,
then host focus reloaded phase two. Native winsaveview/cursor/readonly/modified
values were captured through commands physically entered in Neovim; line 1/column
7 and the entire view stayed unchanged, and the original project stayed intact.
Evidence: `mvp-b6-cli/follow-physical.json`, `follow-editor-*.json`,
`follow-finished.json` and `tui-final-follow.log`. Final packaged idle Follow passed
the same <=8-frame assertion with one frame in 3.25 seconds, versus the old 326.
See quiet-follow.json and release-qualification.md.

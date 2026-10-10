# Whole-file selective Apply

Owner-approved implementation slice, 2026-10-10 UTC. Backend and Review controls are
implemented; complete package/physical qualification is in progress. This follows the selective acceptance,
isolated workspace and conflict checks in `docs/Codex CLI Fork.txt`, lines 692–712.
Ponytail reuses the existing Apply RPC, application ledger, prepared native writer
and fixed TUI form. There is no new dependency, migration or patch framework.

## Contract and safety

An Apply mutation still captures the accepted task ID, revision and result digest.
Optional `paths` selects whole accepted artifact files. Omitted paths (or JSON null)
preserve legacy apply-all behavior. An explicit set must contain 1–64 unique exact
accepted artifact paths; empty, duplicate and unknown paths are rejected before an
application intent or filesystem write. Paths retain accepted artifact order.

Only selected artifact bytes/hashes and original bases are verified and written.
An unselected file can have changed independently and is left untouched. Existing
root/path protection, serialized writes, base rechecks, private staging, native
permission checks and owner-only RPC authority remain. Durable `application.paths`
records exactly the chosen files, including pending/uncertain recovery receipts.

There is still **one Apply per accepted result**, including partial selection and
uncertain outcomes. Selecting fewer files consumes that result's one application;
the remaining files cannot be installed by repeating Apply. Acceptance and recorded
checks describe the **complete isolated result**, and do not certify the installed
subset. Cross-file atomicity is not claimed. Inspect uncertain receipts and originals
before manual reconciliation; never blindly replay a request after a timeout.

## Review interaction

The Review form reuses the existing owner UI: `n` cycles inline artifact diffs and aligns the
next real-Neovim file; `A` opens whole-file choices with all files selected. Tab/Enter
move between files, arrows/Space toggle Apply/Leave unchanged, F6 displays the frozen
paths/revision/digest and subset warning, and `y` confirms. Esc retains a reviewed
draft or closes a local form without a mutation. From confirmation, use a second
Esc to close the retained draft before switching layouts. An empty selection cannot be sent.
Selection/refresh changes do not change a captured decision.

All loaded dirty Neovim buffers still block host Apply, including unselected buffers.
Inputs are frozen around dispatch. An unknown/failed Apply outcome stays conservatively
frozen for inspection; no new automatic thaw/retry mechanism is introduced. Actual
Neovim remains the editor; inline UTF-8 diffs are limited to 64 KiB per side.

## Evidence and remaining work

Evidence root: `/workspace/.onboarding/qualification/selective-apply-20261010/`.
Backend production Clippy, the complete state suite (259 tests) and all four schema
comparisons and 25 Python contract/signature tests passed. Stable/experimental schema
fixtures and Python contracts are regenerated. All five native API cases and 25
TUI cases passed, including actual Neovim. Final protocol compatibility (one test),
production TUI Clippy and affected formatting also passed; the 25 TUI cases were
rerun after Clippy's iterator simplification. Complete package and physical
CLI/Neovim workflow are being checked before release.
See [testing](testing.md) and [status](status.md) for exact results as they complete.

The initial protocol validator accidentally used a dev-only dependency and failed
production compilation; it now uses a standard-library Result. Initial SDK generation
used the wrong Python environment, then lacked the installed formatter on PATH;
activate `sdk/python/.venv/bin/activate` for both generation and contract tests. Keep
the original failure logs. No paid model request or new installation is needed.

The first physical run mistakenly used one Esc for both modal layers, then stopped
before any intent/write. Its accepted task was preserved. A resume exposed an existing
refresh bug: Snapshot replaced the native dirty-buffer denial message while the
confirmation remained open. The protection still rejected dispatch. Snapshot now
preserves the captured decision's status; the actual dirty-editor regression also
delivers a refresh and requires the denial/confirmation/no-dispatch to remain.
Both failure captures/logs remain in the evidence root's named failure folders.
The affected native UI checks and complete CLI are being rebuilt before resuming.

Hunk selection, staged repeated Apply, selective acceptance of execution results,
automatic checks of installed subsets, scoped-memory retrieval and recurring
schedules remain separate future slices.

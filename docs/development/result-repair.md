# Bounded repair of an exact result

Select an unaccepted `needsReview` task, then Ctrl-P → **Repair exact result
within original grant**. Enter a concrete repair instruction, press F6, inspect
the exact task revision/result digest and confirm `y`. Escape cancels. This is
separate from `c` (request changes and pause) and from creating a fresh grant.

The host rereads the declared input/output snapshot through its bounded no-follow
reader and verifies the exact result artifact bytes and captured immutable check
inputs. A host checkpoint retains all declared files, original artifact bases
and completed-action IDs. The transaction reuses the existing same-grant handoff
path with the same owner. The new attempt imports those captured bytes and runs
all original host checks; it receives no new path, tool, worker or budget authority.
The original grant expiry, token/request usage and attempt ceiling remain.

Repair requires recorded host checks with captured immutable evidence, a stopped
previous native runtime, current enabled definitions, an unexpired original grant,
remaining shared limits and reconciled effects/usage. Stale digest/revision,
changed output/oracle, exhausted limits, unresolved descendants/effects/usage or
changed definitions are refused. A failed transaction leaves no repair checkpoint,
handoff or queued task. An exact decision cannot replay after it queues the task.

Old results/check failures and usage remain in execution/audit history. A repaired
root returns to `needsReview`; acceptance and Apply remain explicit owner decisions.
No automatic paid repair loop is added. The native qualification uses finite local
Responses fixtures and actual daemon/sandbox/immutable checks/TUI confirmation;
it does not claim live-model quality. Read [current evidence](daily-use-qualification.md).

Source evidence: full state345, runtime/workspace19, CLI2, TUI109 (actual official
Neovim), protocol10, exact schema comparisons4 and SDK contracts/signatures25 pass.
Full root formatting and production CLI Clippy with warnings denied pass.
Clean66013d5a2a package and finite native failing-result/repair/Neovim-diff
workflow pass: unchanged grant/scope/counters/bases, failing then passing immutable
oracle, exact physical repair confirmation, exhausted retry refusal, original
files unchanged, normal CLI exit0/daemon stop0. Four localhost responses/60
synthetic tokens/zero paid; two attempts, one handoff, no acceptance or Apply.
Never replay its setup, approval, repair or accounting.

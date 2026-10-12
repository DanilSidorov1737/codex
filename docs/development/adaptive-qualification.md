# Adaptive batch qualification and safe resumption

Tasks 1–2 build and runner checkpoint; application changes are not yet qualified.
Evidence: `/workspace/.onboarding/qualification/adaptive-batch/`.

Activate `. /workspace/.onboarding/activate.sh`. Rust 1.95.0, pnpm 10.34.5,
actual official Neovim 0.12.6 remain installed. CI-pinned just 1.51.0 archive
SHA256 `c8f085ca3e885723c341d06243fc291b5abfdc8bbe3b2c076b117de490387b59`
was checked against Casey/just's official SHA256SUMS before extraction. Official
Facebook DotSlash v0.5.9 Ubuntu x86_64 archive SHA256
`ba6b6908c69db8264efdb8ee32f4906284ea3b77247c611339b5f60160cc5691`
was checked against the official release's asset digest before extraction.
GitHub API returned 403; the HTTPS release asset page supplied that digest.
Both binaries are outside the checkout in `.onboarding/tools/bin`. DotSlash
verifies the existing buildifier manifest's pinned download. No TLS/checksum
checks were weakened; no standalone Ponytail hooks were installed.

Initial full root format check failed only Rust import ordering. Correcting
25 existing files' imports makes `python3 scripts/format.py --check` pass (exit 0).
No application behavior changed in this formatting checkpoint. Initial/fix/pass
logs retain the stable-rustfmt warnings about nightly-only import options.
Use one `--locked --offline --profile dev-small` Cargo selection at a time,
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608`, cwd `codex-rs`. Check `df -h`
before cold compilation. All earlier fixture homes/packages/receipts are protected.

## Finite resumable runner

`python3 scripts/qualify_organization.py PLAN.json PRIVATE_EVIDENCE_DIR` runs a
finite plan without a shell or new dependency. Example plan:

```json
{
  "repository": "/workspace/codex",
  "inputs": ["/tmp/qualified-package/package.json"],
  "steps": [
    {
      "id": "state-tests",
      "kind": "check",
      "cwd": "/workspace/codex/codex-rs",
      "argv": [
        "cargo",
        "test",
        "--locked",
        "--offline",
        "--profile",
        "dev-small",
        "-p",
        "codex-state",
        "--lib",
        "-j4"
      ],
      "timeoutSeconds": 1800,
      "expectedExit": 0
    }
  ]
}
```

Evidence must be private and outside the repository. The manifest pins all Git
tracked/unignored source contents, canonical plan and declared immutable inputs.
Each step writes/fsyncs its intent before starting, then its exact argv/cwd/exit
and log hash. One file lock prevents simultaneous runners. Passed steps are
skipped only while their receipt/log and manifest match. Source/plan/input drift
requires a new evidence directory; preserve the old run. `--retry-checks` permits
only failed/interrupted explicitly read-only checks. Failed, timed-out or
interrupted **actions never replay**, even with that option. Inspect their actual
state and receipts, then prepare an independently reviewed remaining plan. Do
not relabel mutation scripts as checks. Inputs must be immutable files; mutable
fixture state belongs in action-specific evidence, not source fingerprints.

Seven real-subprocess automated tests pass (exit 0, `runner-tests-v2.log`): completed
skip/log integrity, failure no replay, interrupted intent no replay, timeout
uncertainty, dirty source/plan/input mismatch, read-only retry retaining old logs,
and private/outside-source evidence enforcement. This is runner qualification,
not qualification of unimplemented tasks 3–30.

## Setup and resource checkpoint (physical qualification pending)

The first empty dashboard opens a guided project/root/agent/instructions/model/
finite-lifetime-budget form; Commands also opens it. F6 captures the exact review
and owner confirmation is separate. Setup pauses organization execution before
saving definitions and never creates a task/grant. Partial setup is preserved;
the wizard refuses to replay it. Resume the saved components with individual
forms. Existing organizations keep their definitions and controls.

Task4 reserves nominal token slices in the same admission transaction. Each slice
is limited by remaining child/root/global/project/run/series allowance. Reported
usage replaces the reservation exactly once; late reports update accounting but
never restart work. Unknown/failed/interrupted requests retain their reservation.
Migration19 conservatively reserves old unknown requests without changing their
counters. This is a local nominal allowance, **not a hard provider output/billing
limit**. A returned response may exceed its reservation; subsequent admission
then stops. Execution inspection lists exact reservations, including released
receipts. Immutable profile grants and original owner authority remain required.

Task5 adds an owner ResourcePressure control and actual report samples. The chosen
filesystem's statvfs available bytes and Linux MemAvailable (limited further by
cgroup-v2 remaining memory when available) gate admission. Zero disables a
threshold; missing required telemetry pauses. Paused quiescent runtimes release
capacity and wait; in-flight usage/effects stay charged. Fixed expiry clocks keep
advancing; no concurrency increase or grant renewal occurs.

State suite exit0: **325 passed**, `resources-state-v2.log`; initial three failures
and the fixes remain in `resources-state-initial.log`. New cases prove concurrent
roots cannot spend the same nominal allowance, pressure pause/resume preserves
scopes/usage, and unknown reservations survive restart with late receipts fenced.
The older migration fixture now expects no invented historical released receipts,
while preserving exact grants, attempts and accounting. First cold TUI compilation
is still in progress; setup/physical CLI/TUI checks are not yet claimed.

Task6 is being added as conservative **offline cold archival**, retaining bytes
and all canonical ledgers. It will refuse active/uncertain/result/base/action/note/
checkpoint/dependency/workflow references. No deletion or reclaimed-disk claim is
made. Historical qualification fixtures are never retention targets.

## Archival, uncertainty and profile checkpoint

The selected state suite now passes **329 tests** (exit0, `profiles-state-v2.log`).
Task6 previews only closed, quiescent failed/cancelled attempts with reported
usage, no action/result/recovery/note/child/dependency/workflow references and no
original-base hashes. It archives exact content into the managed temporary
workspace parent's private archive directory. A durable rename intent precedes
movement; completion verifies the archive hash and remaps the attempt. Interrupted
archival is never renamed again: `organization retention-reconcile` only inspects
actual paths/hash. Ledgers and bytes remain; **bytes reclaimed = 0**. It supports
canonical managed workspace paths, not arbitrary paths or restored-home paths.
CLI commands require explicit `--offline` and a stopped daemon; all editor/runtime
writers must be stopped. Tests prove stale content refusal, unknown/base-reference
exclusion, exact byte retention and immutable grants/counters. Initial retention
SQL-column failures are preserved in `retention-state-archive-restore.log` and fixed.

Task7 records an immutable owner inspection of exact admitted requests and action
evidence. Its fingerprint excludes heartbeat-only changes; request/effect changes
reject stale review. Saving inspection does not clear uncertainty, release token
reservations, modify accounting or retry. Actual late host usage remains the only
implemented usage receipt path. Task8 saves versioned native execution profiles
(model, exact paths/checks, budgets, approval/delegation policy). Profile history
uses existing controls; `authorizeProfile` validates the exact captured version in
the same transaction that creates a separate grant. Later profile edits never
change old grants. State tests prove stale profile denial and independent grant
contents, as well as immutable uncertainty inspection with unchanged reservations.
TUI commands/forms are implemented but their latest check and physical operation
are still pending; no final capability qualification is claimed yet.

The first TUI run passed92/failed1 after cold compilation. Its menu ordering test
caught new actions displacing layout navigation; preserve the first five layout
entries and route new actions after those. Two shortcut collisions were corrected.
The later read-only runner keeps previous failed receipt exit codes as well as logs;
its seven subprocess tests still pass (`runner-history-tests.log`).

## Compiler storage recovery

Two prerequisite caches were pruned with metadata inventory receipts. Their
lower-layer deletion freed no overlay space; deleting Cargo archives also broke
offline dependency validation. That failed attempt is preserved, not called a
pass. Installed packages/extracted sources/fixtures were untouched. Locked selected
Cargo archives are being restored with normal TLS/checksum verification into
`/dev/shm/codex-adaptive-cargo-archives`, referenced by the registry-cache symlink.
State offline checks then pass; TUI's additional missing archives require the
same bounded restore before its next offline check.

External generated static compiler archives (2,009,002,124 bytes) were copied into
private `/dev/shm/codex-adaptive-compiler-data`, hash-verified and referenced by
symlinks at their old paths. This frees about1.9GiB of the separate /tmp mount;
source, binaries, packages, historical homes and receipts remain intact. Exact
paths/sizes/hashes are in `compiler-data-relocation.json`; use its recorded total
rather than this rounded explanation if they differ. Shared-memory data is
rebuildable and shares the host memory limit; it is not a durable package store.
Never run competing Cargo selections; continue checking disk and cgroup headroom.
A new environment must restore missing registry archives/compiler data before
claiming offline readiness. No network restriction or checksum was bypassed.

Latest TUI selection: **93 passed**, exit0, `profiles-tui-archive-restore.log`,
including actual official-Neovim cases. Seven runner subprocess tests pass.
This is a source/test checkpoint for tasks1–8; complete native CLI/TUI workflows,
contract regeneration, production lint/package and task30 endurance remain pending.
Tasks9–30 remain approved and unfinished. No historical services were restarted.

## Conversation and goal checkpoint (qualification continues)

Task9 now persists a conversation for one exact project, stable agent and topic.
An owner prompt atomically creates a proposed planning task without a grant;
each turn needs separately reviewed execution authority. Only the host-attributed
live task can append its reply. The caller cannot choose a different agent,
project, conversation or task. Replies are untrusted model content, not approval.
The transcript and history survive restart. At most16 turns are stored and only
the last four bounded pairs enter subsequent planning context. Direct owner
control setters cannot forge a transcript. Commands opens/continues the form and
reads the canonical transcript and task state.

Task10 reuses the existing planning artifact/workflow path: a natural-language
request creates a proposed task for a selected project. Its bounded `plan.json`
is reviewed, imported with W into an editable workflow, then separately scoped.
This avoids a second workflow engine. Tasks11–14 add responsibility metadata for
arbitrarily nested goals, configured agents or teams, exact acyclic prerequisites,
and business due dates. Existing goal/task forms retain arbitrary hierarchy.
Accepted descendant results can support parent criteria, but every criterion
and parent completion still need explicit owner review; unfinished descendants
prevent completion. Cross-project prerequisites require explicit metadata approval
and grant no data or tool access. Changed prerequisites/completion evidence park
new dispatch without rewriting existing grants, reservations or accounting.
Goal reparenting also checks the combined hierarchy/dependency graph.

Business dates pin the current target revision when edited. RFC3339 editing keeps
an explicit local timezone offset and canonical UTC timestamp; the inspector
shows the date/overdue status. This supports fixed offsets, not IANA timezone/DST
rules. Grant expiry and scheduling clocks are unchanged. Clearing a business
date is an independent metadata update, never grant renewal.

State selected checks:330 passed (`conversation-state-v2.log` and
`goals-state-initial.log`), then332 passed (`goals-state-v2.log`), all exit0.
The latter includes cross-project approval/cycle/stale-completion admission and
business-date/grant invariance. The descendant criterion test was subsequently
extended and passed in the final332-test state run (`planning-state-v3.log`,
exit0). TUI tests for conversation, project planning, local-date roundtrip and
closed-versus-overdue display passed in the98-test selection
(`planning-tui-v3.log`, exit0), including actual official-Neovim cases.
An initial TUI formatter found a
misplaced render insertion; its corrected parse passes. The first compile found
another misplaced date-render block, corrected before the rerun. Preserve
`goals-tui-format.log` and `goals-tui-initial.log`. A prematurely queued duplicate
Cargo check was stopped while waiting on the artifact lock; no competing build
selection was allowed to run. Full root formatting passed (exit0,
`planning-full-format.log`); selected document formatting and `git diff --check`
also pass. Ponytail/correctness review reuses the existing task mutation,
control/history writer, SQLite graph checks, Chrono and planning/workflow path;
no dependency or second workflow engine was added. Schema/SDK regeneration,
production lint/build/package, physical CLI/TUI and complete finite endurance
are pending.
No paid calls, external integrations, historical fixture mutations or GPT Dot
integration have occurred. Continue the approved batch; these are source/test
checkpoints, not delivery completion.

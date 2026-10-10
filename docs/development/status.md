# Current status and resumption instructions

Phase 0: **exploration and planning complete for available inputs**.
Current work: **approved MVP implementation; B1–B4 locally qualified; B5 Neovim underway**.
Application feature implementation: **B1–B4 locally qualified; B5–B6 outstanding**.

## B4 qualification

All five presets are physically qualified with the complete current CLI and
actual detached daemon at 120×30 and 50×20. Operations search/collapse/expand,
revision-checked pause/resume, pane customization/reload, real exact approval,
request changes and exact result acceptance passed. The localhost provider
produced a real isolated report, native owner checks exited zero and the original
project remained unchanged. The current successful grant used two requests/30
reported tokens; this is deterministic fixture evidence, not live-model evidence.

Review confirmation now freezes its displayed payload, permits scrolling and
captures a request-changes task before drafting. **152 targeted tests passed**:
six layout tests and 146 existing slash/overlay regressions. Production Clippy,
formatting and the corrected full CLI/package build passed. An old pending grant
expired during rebuilding and its stale approval was rejected without execution;
the fresh bounded grant and subsequent changed-result grant passed. Failure logs
remain retained. See [layouts](layouts.md) and [testing](testing.md).

B5 must now integrate actual official Neovim and safe separate artifact application;
B6 remains outstanding. Continue the entire approved MVP without another routine
implementation approval.

## B3 qualification

Added shared-root budgets, bounded plan/DAG validation and delegation, durable
scoped notes, deduplicated inbox/acknowledgement and child waits that yield
capacity after the current provider usage receipt. Attempts capture role versions.
State library: **259 passed**, including eight coordination tests; production
state Clippy passed with warnings denied. Legacy policy grants fail closed;
one-shot starts and inbox/context recovery across fresh authorization are tested.
The runtime adapter exposes a narrow coordination tool and copies accepted,
hash-verified child artifacts into isolated workspaces. Four runtime units and
all four native API tests passed, including real two-agent delegation, notes,
critical inbox/ack, yielded waiting, immutable owner checks, artifact import and
owner final acceptance. The workflow recorded 13 deterministic provider requests
and 195 reported tokens; both root and child views show the same root budget.
Final production Clippy and affected formatting passed. Two wire tests, four
schema comparisons and 25 Python contract checks passed. The packaged CLI qualification passed 13 assertions; its detached daemon ran
13 deterministic requests and both real owner checks, with exact owner acceptance. See [coordination](coordination.md).
B3 is qualified locally. Continue B4 layouts, B5 official Neovim and B6 gates.

## Current authorization

The owner's “Okay, let's do this” approved the described next step: build and
physically exercise the real CLI, check detached daemon/reconnect behavior, obtain
verified official Neovim and test its embedding API. Required local prerequisites
are included. No push, publication, external spending or broader operational
authority is inferred. This qualification is complete. The owner now requests
“Okay, let's implement this. Use Ponytail to implement, and then commit
periodically, follow the MVP plan”. This approves B1–B6's ordinary implementation,
tests, required local dependencies, documentation and local commits. It does not
authorize push/publication, outbound communications or unbounded provider spending.

## B1 implementation progress

- Added `OrganizationStore` to existing codex-state, reusing SQLx and the existing
  cross-platform private-directory helper. No new external dependency was added.
- Canonical database lives at `<sqlite_home>/organization/organization.sqlite`,
  inside an owner-only directory. Linux files are mode 0600; symlinks/hard links,
  unsafe ownership/permissions, incompatible migrations and corruption are rejected.
- Uses WAL, FULL synchronization, foreign keys and one connection for short
  transactions. No runtime backup-and-rebuild fallback applies to this store.
- Added verified consistent snapshots and restoration into a fresh directory.
  Missing/truncated initialized state is rejected rather than reset automatically.
- All 233 codex-state library tests passed, including five new meaningful store
  tests. Clippy with warnings denied and package formatting checks passed.
- Added typed, experimental owner RPC and real `codex organization` commands for
  arbitrary agent/group forests, immutable role versions, goals, existing-project
  links, proposed tasks, memberships and snapshots. See [command guide](organization-commands.md).
- Revision checks, hierarchy/dependency cycle rejection, same-project prerequisites,
  immutable node kinds and transactional audit references are enforced in SQLite.
- Owner administration accepts protected local origins and rejects network/Remote
  Control origins. Agents do not receive this administrative API.
- All 237 codex-state library tests passed, including concurrent restore protection. The app-server API restart test passed
  with stable agent and existing-project IDs. Actual CLI commands created and
  renamed records, rejected stale edits/cycles, backed up and restored a fresh
  database; all seven definition/audit tables matched exactly.
- Socket regression (11), owner guard (1), mutation/schema parity (1) and Python
  contract/signature (25) tests passed. Format, final Clippy and all four schema comparisons passed. The final CLI build and current-binary command requalification passed. No background organization task starts yet.
- Current B2 work: durable authorization, attempts/reservations, approvals/action intents,
  bounded runtime execution and fail-closed restart reconciliation. Agent sandbox
  exclusion must be proved before unattended execution.

## B2 ledger progress

- `b308b3f1f7` added durable authorization, atomic claims, attempts/leases, model-request ceilings, usage accounting and fail-closed startup reconciliation.
- The current storage slice adds exact-argument approvals, single-dispatch action intents/receipts, protected execution-owner locking, owner pause/cancel/review, frozen artifact bases, aggregate usage views and runtime reservations retained until native cleanup is joined.
- A waiting approval yields its inference reservation only after the current model request's usage is recorded. Pause alone does not free a running process's slot.
- All **250 state library tests passed**, including **22 organization tests**. State Clippy with warnings denied passed. These are ledger tests; constructed artifact hashes do not prove filesystem/editor behavior.
- The subsequent policy correction and approval/usage race test passed **251 state tests**. Model-request gates passed three unit and eight integration tests; the native denial test passes after fixing fatal-error propagation from the existing tool drain. Six existing tool-lifecycle observer regression tests passed after building their missing native MCP fixture.
- Interrupted work becomes `needsReconciliation`; pending approvals remain recorded and cannot be accepted against an interrupted attempt. No uncertain native action is automatically replayed.
- Core/ledger admission is committed. The runtime adapter now passes actual local-thread/native-tool artifact, exact approval, nine isolation probes and pending-approval SIGKILL/restart checks. This uses a deterministic provider; the current CLI and actual daemon transport still require qualification before B2 completion.
- The broader core test build initially raced cache cleanup; rerun uses `CARGO_INCREMENTAL=0`. Core integration also aborted without the repository CI's `RUST_MIN_STACK=8388608`; the CI-configured model gate rerun passed three unit and eight integration tests. Preserve failure logs.
- Tool qualification caught an incorrect adapter assumption: this checkout registers `exec_command` in one-shot mode when UnifiedExec is disabled, not `shell_command`. The working tree now uses that existing handler and rejects embedded CLI execution mutations so task execution requires the detached daemon. The corrected adapter and owner-origin/privilege guards now pass the application tests described below.
- The runtime candidate also rejects native privilege expansion arguments, isolates one local environment, disables transparent provider retries, rejects hard-linked snapshots and reconciles before execution reads/decisions. The actual artifact/crash paths subsequently passed application-level tests; CLI/mid-action checks remain outstanding.
- Core/ledger admission is qualified: final production Clippy with warnings denied and affected-package formatting passed. The only new lint failure was an unnecessary workspace-root clone, corrected without suppression.
- `058385cffc` committed the core/ledger admission slice. One wire-contract test, three application unit checks and three API tests passed. Explicit protected-root denies then passed production Clippy and repeated all six application checks. See testing.md for genuine native isolation evidence and corrected fixture failures. No B2 runtime completion is claimed before CLI/mid-action checks.
- Regenerated stable/experimental API bundles: four comparison checks passed; regenerated Python contracts/signatures: 25 passed. The current CLI initially built successfully. Its raw binary correctly refused daemon startup without a complete package; embedded execution was refused as intended.
- The first packaged manual task caught a real default-installation bug: native helpers live inside the private owner home and the preliminary blanket deny blocked them. Corrected the profile to grant only exact helper-file reads there, retaining a private allowlist and rejecting overlap with system read roots or the original project. Reused the native backend's unchanged system-root list through a shared constant.
- Corrective Clippy passed. All three API tests then passed with actual helpers staged inside the owner home; the successful case also places canonical state in that same home. All nine actual denial assertions remain intact. New fixture compile mistakes (borrow/path handling and a wrong rerun working directory) were corrected; no assertion or production security check was disabled. See testing.md.
- `a360de3115` committed the tested runtime/API, contract fixtures and command clients.
- Corrected CLI build and complete package qualification passed. Thirteen real CLI/native/IPC assertions cover exact approval/artifact/oracle/acceptance, detach, actual owner socket denial, private-home state, pause, post-dispatch SIGKILL/restart, uncertain action and no idle inference/replay. Cleanup also passes for a SIGTERM-ignoring native command, checked before ordinary timeout. Seven local fixture Responses were recorded; no live model was called. Fixture services were stopped.
- B2 is qualified for the local deterministic operating envelope; final provider/fault/scale qualification remains B6. See testing.md for commands, timing, failures and evidence.
- B3 is subsequently qualified. Continue B4–B6 ordinary work without another implementation approval.

## MVP planning completed before implementation

- Replaced stale build/editor prerequisites with the actual qualification results.
- Defined one complete goal -> authorized plan -> background work -> reviewed result
  workflow, covering standalone and custom multi-agent use, coding and documents.
- Organized the MVP into B1 durable definitions, B2 one safe background task,
  B3 bounded delegation/context, B4 five layouts, B5 actual Neovim Workspace and
  B6 live/interactive/recovery qualification. See [MVP](mvp.md).
- Clarified in-scope autonomy, checked internal task acceptance, owner final
  acceptance, separate safe patch application and initial operating limits.
- Product code was unchanged in that planning commit. Documentation verification is recorded in
  [testing](testing.md); foundation checks were not rerun for a planning-only change.

## Phase 0 completed

- Read the initial request, full 714-line capability/layout/Neovim write-up and
  visually inspected all seven concept/hierarchy images.
- Found updated documentation on origin/main and fast-forwarded the clean checkout
  to documentation baseline `8026444c98cd9c6db9e7c953b44a20ea4f93c7b6`.
  No application sources changed.
- Traced agent coordination, roles/limits/residency/mailbox, daemon/client/protocol,
  thread/SQLite storage, projects/goals, memory, policy/tool/provider and TUI seams.
- Researched official Neovim external UI/API/startup and official Ponytail setup.
  Read upstream docs/manifests; did not install/execute third-party tools/hooks.
- Wrote architecture/data/security/recovery/layout/editor plans, requirement map,
  phased roadmap, detailed MVP criteria, process/testing docs and root AGENTS.md.
- Ran existing tests: 17 Rust, 80 Python and 14 TypeScript unit tests passed.
  Invoked real existing file-search CLI in a PTY; Cargo.toml match verified.
- Checked documentation formatting, links, allowed change scope and whitespace
  before the final report.

## Foundation qualification results

- Built real CLI `0.0.0` with Rust 1.95.0 and the existing `dev-small` profile.
  Built the real Code Mode host and vendored Bubblewrap, then assembled the
  existing complete package. No application source/manifests/lockfiles changed.
- Installed checksum-verified official Neovim v0.12.6. Twelve real embedding
  assertions passed on two corrected reruns; physically edited/saved/undid/diffed
  real files in a terminal.
- Twelve real daemon assertions passed on two runs: idempotent start, private IPC,
  detached turn completion, pending approval replay on reconnect, blocked tool
  before approval, actual artifact after approval, restart and completed history.
- Physically used both the embedded TUI and daemon-attached TUI. `/status` reported
  `Local background server`; its thread ID matched the daemon's loaded-thread API.
  Quit and resumed the same thread with response and token usage restored.
- Full SDK retest passed **49/49 tests, 5/5 suites**, including both MCP conformance
  gates. Earlier missing binary/helper failures are retained as historical results.
- See [qualification](qualification.md) for commands, provenance, failures and
  machine-local evidence. Eleven existing socket transport tests passed unchanged
  outside the outer sandbox; the sandboxed attempt had seven passes/four failures
  caused by its masked socket root. No assertions were skipped or disabled.
- Repeated the checksum-preserving setup successfully and saved complete install/
  startup instructions to the environment draft. Review/save and publication in
  environment settings remain owner actions; fresh-task restoration is unverified.
- Verified the fixture daemon survived TUI quit, then stopped only that daemon
  and the local provider fixture. No qualification service remains intentionally running.

## Known limitations / remaining work

- Separate PDF is absent; compare it if provided.
- Actual Neovim is qualified independently; the Rust Workspace pane is not built.
- Upstream standalone Ponytail plugin installation/hooks are unverified;
  current session skill availability is separate.
- Targeted pending-approval and dispatched-native-action crash/restart checks,
  agent denial of actual owner IPC and native pause/cleanup now pass. Broader B6
  faults, scale, live-provider and all-Rust/Bazel coverage remain outstanding.
  Fixture Responses are deterministic local responses, not a live model.
- The outer cloud sandbox hides daemon sockets. The isolated daemon and attached
  TUI checks ran with authorized unsandboxed execution; the restriction was not
  weakened. Future daemon checks need that execution mode.
- B1 canonical state and definition/API decisions are implemented; later runtime,
  security, layout and editor decisions still require implementation and evidence.

## Resume safely

1. Read root AGENTS.md and this file, then decisions, MVP and testing.
2. Check Git status/log and preserve existing changes; use the existing isolated checkout.
3. Verify the owner's latest explicit approval and constraints. Without approval,
   answer planning questions or revise documentation only.
4. Review qualification evidence and remaining gates rather than repeating the
   baseline. Implement B5 official Neovim, then complete B6 gates.
   Do not restart the completed storage/definition work.
   Full Neovim pane integration remains B5/M8. Keep broader
   feature scope and live-provider budget explicit.
5. Recheck runtime/environment readiness and existing credential bindings without
   printing values. Use established proxy/TLS/auth routes.
6. Follow small implementation/test/review/doc/commit steps and update status with
   actual delivered behavior, evidence, failures and next action.

A local documentation commit does not authorize pushing, merging, deploying or
publishing. Never treat this status file as proof that future features work.

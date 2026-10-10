# Testing procedures and evidence

## B3 state and contract slice

- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -j4 -p codex-state --lib`, from `codex-rs`: **259 passed**, zero failures/ignored. Eight new coordination tests cover atomic DAG proposals, prerequisite admission, shared-root request races and token accounting, wait/capacity cleanup, real-check acceptance rules, scoped notes/inbox/deduplication/recovery, parent pause, one-shot schedules, role/policy snapshots and the four-resident-root bound. Log: `mvp-b3-state-recovery-final.log`.
- Production state Clippy with `-- -D warnings`: exit 0 (`mvp-b3-state-final-clippy.log`). Both execution/coordination wire tests passed; unknown agent authority fields are rejected (`mvp-b3-wire-tests.log`). Python contract generation/signatures: **25 passed** (`mvp-b3-python-contract-tests.log`).
- Regenerated stable/experimental contracts in the repository's existing format; **all four schema comparisons passed** (`mvp-b3-schema-final-comparisons.log`). Affected protocol/state/app-server package formatting passed (`mvp-b3-format.log`); the generator's ignored entry point was explicitly run for both bundles.
- Preliminary runtime library/API compilation passed (`mvp-b3-runtime-first-check.log`, `mvp-b3-api-test-check.log`). These checks do not qualify delegated native execution, physical CLI interaction or the new layouts/editor.
- Initial compile fixes: the plan ID map needed an explicit value type and check exit status is `Option<i32>`. A test fixture initially tried to admit three active tasks under limit two; corrected its explicit limit and reran all tests. A formatting invocation used paths relative to the wrong directory; corrected without changing checks.
- Schema regeneration with Prettier caused unrelated TypeScript changes and two comparison failures. Regenerated using the repository's unformatted mode; no unrelated edits are intended. Python generation initially lacked the installed virtual environment's `ruff` executable on PATH; activated it and reran successfully. Failure logs are retained alongside corrected runs.

Evidence lives under `/workspace/.onboarding/qualification/`. State tests use
constructed artifact evidence; they do not prove native file/check behavior.
Actual runtime/CLI qualification is the next gate; B3 remains in progress.

### B3 real runtime/API evidence

- `cargo test --locked --offline --profile dev-small -j4 -p codex-app-server --lib organization`: **four passed**, no failed/ignored. Log: `mvp-b3-runtime-unit-tests.log`.
- `cargo test --locked --offline --profile dev-small -j4 -p codex-app-server --test all suite::v2::organization -- --nocapture`, with `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608`: **four passed**, no failed/ignored. Initial native workflow: `mvp-b3-runtime-native-api-tests.log`; current shared-budget rerun: `mvp-b3-runtime-shared-budget-final-tests.log`.
- Actual native sandbox/application cases retained the B2 restart, exact approval, nine isolation probes and pending-approval SIGKILL behavior. The new case used two configured agents, proposed/delegated a task, read scoped project context and its inbox, acknowledged the message, yielded the parent, ran real Python checks, imported the hash-verified child artifact and accepted the root only through owner RPC. Original project had no output file. Provider responses were deterministic fixtures: **13 requests / 195 reported tokens**; root attempt 105, child 90. Both task views report the shared totals.
- State view correction reran all **259 state tests** (`mvp-b3-state-budget-view-tests.log`). Final production app-server/state Clippy passed with warnings denied (`mvp-b3-runtime-corrected-clippy.log`); affected formatting passed (`mvp-b3-runtime-final-format.log`). Earlier final Clippy rejected two `expect` calls and a borrowed async mutex guard; replaced with fallible, once-per-attempt schema construction and the existing owned-guard pattern. No lint was suppressed.

Tests ran outside the outer sandbox because it blocks real fixture sockets and
native sandbox setup. No paid model requests occurred. Full packaged CLI
interaction and B4–B6 remain outstanding.

## B2 complete local workflow qualification

`a360de3115` contains the runtime/API. The corrected current CLI build passed
(`CARGO_INCREMENTAL=0 cargo build --locked --offline --profile dev-small -p codex-cli --bin codex -j4`, exit 0, 5m06s). Built its complete package using
`scripts/build_codex_package.py` with the current CLI and previously qualified
native host/Bubblewrap/ripgrep inputs; no new dependency was installed.

Physically entered commands in a real Bash PTY using that package, with
`CODEX_HOME` and `CODEX_SQLITE_HOME` pointing to the same private fixture home.
Created/linked an agent/project/task, authorized it, observed waiting approval
with no artifact, approved the exact hash, waited for real artifact/check evidence
and accepted its digest. Each CLI process exited while daemon work continued.
All nine actual isolation probes passed, including the daemon's **actual owner
transport**, protected state read/write, fixture credentials, original project
read/write, immutable oracle write, provider network and inherited marker.
The actual owner socket's UID, 0600 mode and private parent were also checked.

Paused an actually running native command and observed joined capacity release.
Killed only the known fixture daemon after a persisted `dispatched` action and
an actual on-disk start marker. Restart immediately yielded `needsReconciliation`,
`uncertain` action, no result and no replay. Six-second idle observation showed
no additional provider requests or artifact/heartbeat writes.

Strengthened cleanup qualification with a native command that deliberately
ignores SIGTERM. Pause proof checks its heartbeat for two seconds **immediately**
after capacity release, beginning within five seconds of native start. Crash
proof checks two seconds after daemon SIGKILL and finishes within eight seconds
of native start. Both checks precede the ordinary command timeout, preventing a
late timeout from falsely qualifying cleanup. Both passed, then restart again
preserved the uncertain action without replay.

Thirteen physical/native/IPC assertions passed; seven deterministic Responses
requests were recorded across the local qualification cases. These are real
application/tool/sandbox tests with **mocked model responses**, not live inference.
`--no-daemon` execution and embedded fallback both refused execution. Fixtures
were stopped afterward. Current affected-package formatting and production
Clippy passed; schema comparisons (4), Python contract/signature tests (25),
wire mutation test (1), application units (3) and API tests (3) passed.

The first package cleanup left a retired fixture daemon log after removing its
stopped installation. Existing startup correctly refused an incomplete prior
installation. Archived that retired metadata (retaining evidence), then recreated
the complete fixture installation; canonical state was never reset. Preserve a
selected live installation or use the existing updater on subsequent rebuilds.

Evidence under `/workspace/.onboarding/qualification/mvp-b2-cli/`:
`qualification.json`, `positive-evidence.json`, `pause-evidence.json`,
`dispatched-before-crash.json`, `recovered.json`,
`immediate-stubborn-pause-evidence.json`, `stubborn-recovered.json`,
`provider-requests.jsonl` and archived daemon metadata. Build logs:
`mvp-b2-private-helpers-cli-build.log`, `mvp-b2-package-current-build.log`.
B3 delegation/notes, five layouts, integrated Neovim and broader B6 fault/scale/
live-provider qualification remain outstanding.

## B2 application qualification (CLI checks in progress)

- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -p codex-protocol --test organization_execution_contract -j4`: exit 0, **one passed**, zero failed/ignored. All five execution mutation wire variants match their schema fields; unknown authority and misspelled revision fields are rejected.
- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -p codex-app-server --lib organization -j4`: exit 0, **three passed**, zero failed/ignored. Tests cover owner origins, privilege-expansion arguments, traversal/symlink/hard-link rejection and bounded artifact reads.
- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -p codex-app-server --test all suite::v2::organization -j4`: exit 0, **three passed**, zero failed/ignored. Tests run the actual local Codex runtime and native sandbox with a deterministic Responses fixture. Exact approval precedes a real artifact; the immutable owner check runs and its output/hash/base evidence is recorded before exact-digest acceptance. SIGKILL/restart retains a pending approval, immediately reconciles the attempt and rejects its old lease without replay.
- Nine actual OS probes deny protected DB read/write, fixture credential read, original-project read/write, immutable-oracle write, owner-control Unix socket connect, provider-network connect and inherited credential-marker access. Fixture paths are checked to exist before the attempt; the marker is present in the local daemon's environment. The Unix socket in these API tests is a real local control fixture, not yet the daemon's actual owner transport.
- Added explicit denies for canonical owner homes/state and the original project, including custom paths under otherwise readable roots. Repeated production Clippy (`--lib -- -Dwarnings`), three unit checks and all three API tests: all passed. This preliminary policy rejected helpers inside owner state; the ensuing physical daemon test exposed the normal private-home installation overlap. Its correction and rerun are in progress; original-project helper overlap remains rejected.
- Initial integration failures were fixture defects: fallible path construction in a `ClientRequest` closure; using the success-only RPC helper to inspect an expected denial; and creating a denied network socket outside the probe's exception handler. Corrected the fixtures and reran, retaining the stale-lease and all nine isolation assertions. Production policy was not weakened.

Logs: `mvp-b2-execution-contract-tests.log`, `mvp-b2-worker-unit-tests.log`,
`mvp-b2-background-api-tests.log` and the corrective
`mvp-b2-background-api-current-tests.log` in the qualification directory.
Final logs: `mvp-b2-background-api-local-isolation-tests.log`,
`mvp-b2-protected-roots-clippy.log`, `mvp-b2-protected-roots-unit-tests.log`
and `mvp-b2-protected-roots-api-tests.log`.
The checks run outside the outer tool sandbox to permit the already-authorized
real native sandbox/socket qualification. Fixtures remain local; no live model
is claimed. Actual daemon transport/CLI and mid-native-action cleanup checks remain pending.

The cloud disk reached its build-cache limit. Removed unused pnpm registry
metadata and three identified obsolete unit-test executables. Current executables,
current libraries, active build outputs and recorded test logs were retained; incremental
compilation remains disabled.

The first physical run used the raw Cargo binary; daemon startup correctly
rejected its missing package, and `organization execute` rejected embedded-server
fallback. The repository packaging helper assembled the current binary and native
helpers without new installation or external requests. That real daemon then
exposed a policy bug: its helpers live inside `CODEX_HOME`, while the preliminary
profile blanket-denied that parent. Correcting this with exact-file read grants
under a private allowlist, sharing the existing native system-read-root constant,
and rejecting unsafe system-root overlap. The API regression now stages actual
helpers inside the owner home and retains all nine denial assertions. Corrective
Clippy passed with warnings denied; the three application unit checks passed.
The corrective API run passed **three tests**, zero failed/ignored, with both
private-home helper staging and canonical state co-located in that home. All nine
OS denial assertions and stale-lease/crash assertions remain. The fixture first
failed to compile on an unborrowed destination path; a later state-path conversion
missed a multiline `.path()` and one rerun used the wrong working directory.
Corrected these fixture errors and ran a full integration type check successfully
before the passing native run. No assertion was removed. Logs:
`mvp-b2-private-helpers-clippy.log`, `mvp-b2-private-helpers-unit-tests.log`,
`mvp-b2-private-helpers-fixture-check.log`,
`mvp-b2-private-helpers-api-verified-tests.log`. The corrected CLI rebuild and
manual default-daemon workflow remain pending.

Further disk cleanup removed identified obsolete standalone tests/B1 CLI,
pre-admission core archives not referenced by the active compiler, downloaded
package caches and stopped obsolete fixture daemon packages. Source/state,
current artifacts and evidence were retained. No compiler working directory was
removed.

## B2 core admission gates under qualification

`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -p codex-core --lib --test all model_request -j4`:
exit 0, **three unit and eight integration tests passed**. The negative admission
test proves no HTTP Responses request reaches the local fixture provider. Its
initial fixture incorrectly expected a request; that expectation was removed and
replaced with an explicit zero-request assertion, retaining the denial assertion.
Evidence: `mvp-b2-model-gate-ci-final-tests.log` in the qualification directory.

The native tool denial test initially reached an unexpected follow-up/404 instead
of its guard because `shell_command` is not registered in this checkout. Inspection
found the existing `exec_command` one-shot handler. Its corrective run exposed
fatal tool errors being panicked/logged instead of propagated during the turn's
tool drain. That path now returns the error. The denial test then **passed**:
the error is reported to the caller and the attempted artifact is absent.
The current core integration binary also reran all eight model-request matches
successfully. Logs: `mvp-b2-tool-gate-propagation-tests.log` and
`mvp-b2-model-gate-post-propagation-tests.log`.

An additional lifecycle-observer regression run passed five tests; the sixth
could not locate the existing `codex-rmcp-client` `test_stdio_server` fixture.
Built it with `CARGO_INCREMENTAL=0 cargo build --locked --offline --profile dev-small -p codex-rmcp-client --bin test_stdio_server -j4`, then reran the
unchanged six assertions: **six passed**, zero failed/ignored. The current core
integration binary was invoked with `RUST_MIN_STACK=8388608` and filters
`model_request` and `tool_start_`; it is the same binary built by the preceding
successful Cargo gate run. Final log: `mvp-b2-tool-observer-regression-final-tests.log`.
No integrated background worker qualification is claimed yet.

The corrected ledger policy and a new approval/budget race test passed the full
state library suite: **251 passed**, zero failed/ignored
(`mvp-b2-ledger-dispatch-tests.log`). Approval cannot revive an action after
recorded token exhaustion. Current application/CLI type checking passed before
the final runtime privilege-validation and startup-approval race corrections;
those require the ensuing application tests.

Final production Clippy passed with warnings denied for `codex-core`,
`codex-extension-api`, `codex-state` and `codex-app-server` library targets.
Affected-package formatting checks passed. The first combined Clippy run caught
one unnecessary workspace-root clone; it was replaced by `std::slice::from_ref`
without suppressing the lint. Logs: `mvp-b2-admission-worker-final-clippy.log` and
`mvp-b2-admission-worker-format.log`. These qualify the core/ledger admission
slice; application artifact/crash tests remain pending.

## B2 approval, evidence and reservation ledger

From `codex-rs`, with the documented activation:

- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -p codex-state --lib -j4`: exit 0, **250 passed**, zero failed/ignored. Twenty-two organization tests cover real SQLite approval/claim/dispatch races, token accounting, slot retention until cleanup, streamed inference completion before waiting-capacity release, owner locking, immutable runtime/artifact-base identity and restart/backup/restore.
- `CARGO_INCREMENTAL=0 cargo clippy --locked --offline --profile dev-small -p codex-state --lib -j4 -- -Dwarnings`: exit 0.
- Logs: `mvp-b2-ledger-current-tests.log`, `mvp-b2-ledger-current-clippy.log` in the established local qualification directory.

Evidence hashes/check outputs in ledger tests are constructed trusted-host inputs.
They verify transitions and review binding, not actual sandboxed artifacts/check execution.
The background runtime and owner execution commands require application-level verification.

Earlier runs passed 246 then 247 state tests while this slice was developed.
A core build failed when concurrent cache cleanup removed a compiler's incremental
working directory. Source/data were preserved; stopped-build caches were removed,
and subsequent builds disable incremental compilation. Three core model-request
unit tests then passed, but the broad integration filter aborted on stack overflow
in an existing pending-input test. Repository Rust CI explicitly sets
`RUST_MIN_STACK=8388608` (`.github/workflows/rust-ci.yml`); the rerun follows that setting.
One preliminary application/CLI check passed, but source changed afterward, so it
is not qualification of the current artifact. Current serial gate/build checks are pending.

## B2 ledger verification

From `codex-rs`, with the documented activation:

- `cargo test --locked --offline --profile dev-small -p codex-state --lib -j4`: exit 0, **241 passed**, no failed/ignored. Thirteen organization tests include actual SQLite claim races, token/request accounting, subtree fencing, late usage, restart reconciliation and backup/restore.
- `cargo clippy --locked --offline --profile dev-small -p codex-state --lib -j4 -- -Dwarnings`: exit 0.
- `cargo fmt -p codex-state -p codex-protocol -- --check`: exit 0.

Evidence: `mvp-b2-ledger-tests.log`, `mvp-b2-state-tests.log`,
`mvp-b2-state-clippy.log`, `mvp-b2-format.log` under the established local qualification directory.
These checks qualify storage behavior; runtime task execution and tool approvals are still unimplemented.

## B2 ledger candidate

`cargo test --locked --offline --profile dev-small -p codex-state --lib organization::tests -j 4`:
**13 passed, zero failed/ignored**, including four new B2 tests: concurrent claims
reserve one slot and commit audit/outbox together; unresolved/double/backwards
usage and exhausted budgets; paused subtree and stale-worker denial across
reconciliation/backup restore; invalid scope, stale authorization and expired lease.
Log: `/workspace/.onboarding/qualification/mvp-b2-ledger-tests.log`.

These are actual SQLite tests. Runtime/model/tool integration and B2 approvals
are not implemented or qualified by this candidate; full-state lint/verification
follows before committing the ledger.

## B1/M3 definitions and real commands

On 2026-10-10 UTC, using the same Linux/Rust activation and `dev-small` profile:

- `cargo test --locked --offline --profile dev-small -p codex-state --lib -j 4`:
  **237 passed, zero failed/ignored** on the final run. New tests add arbitrary forest,
  membership, Unicode, concurrent revision race, immutable role versions,
  goal/project links, task dependency cycles, rollback, pagination, restored
  IDs and exclusive concurrent restoration. Final log:
  `/workspace/.onboarding/qualification/mvp-b1-state-final-tests.log`.
- `cargo test --locked --profile dev-small -p codex-app-server --test all suite::v2::organization -j 4`:
  **1 passed, zero failed/ignored**. Real app-server create/link/list, graceful
  shutdown/restart and stable IDs. Local deterministic provider fixture; no
  inference is needed. Log: `mvp-b1-api-tests.log` in the qualification directory.
- `cargo build --locked --offline --profile dev-small -p codex-cli --bin codex -j 4`:
  exit 0. Built actual CLI with the new organization subcommand.
- Physically entered commands in an interactive Bash PTY using isolated
  `CODEX_HOME` and `CODEX_SQLITE_HOME` under
  `/workspace/.onboarding/qualification/mvp-b1/`: create agent/group/role/goal,
  create an existing project, add child/membership, rename agent, link project,
  propose task, backup, restore into fresh state, list restored records.
  Stale revision and self-cycle edits both exited 1 with expected errors.
  Read-only SQLite comparison confirmed identical nodes (3), role versions (1),
  goals (1), project links (1), tasks (1), memberships (1), audit entries (9).
  JSON outputs and error evidence remain in that fixture directory.
- The offline API build initially lacked pinned `assert_cmd` artifacts. Retried
  the same locked dependency graph through the configured network proxy; passed.
  Completed incremental compilation caches were removed to avoid the 32 GiB disk
  limit; active compiler cache, binaries, source and logs were retained.

- `cargo test --locked --offline --profile dev-small -p codex-app-server-transport --lib unix_socket_tests -j 4`:
  **11 passed**, including an actual owner-socket origin assertion. Ran outside
  the outer cloud sandbox because its masked socket root prevents real socket
  checks; existing socket security was preserved. Log: `mvp-b1-socket-tests.log`.
- `cargo test --locked --offline --profile dev-small -p codex-app-server --lib request_processors::organization -j 4`:
  **1 passed**; local owner origins allowed, WebSocket/Remote Control denied.
- `cargo test --locked --offline --profile dev-small -p codex-protocol --lib organization::tests -j 4`:
  **1 passed**; every mutation's actual JSON fields match its schema. Inspection
  caught unsupported enum-wide field renaming in Schemars; variant annotations
  fixed the generated contract before acceptance.
- Regenerated stable and experimental schema bundles with the repository's
  ignored schema-writer test and regenerated the Python SDK with its documented
  artifact updater. Used `dev-small` rather than creating another default-profile
  build. `uv` used the existing frozen Python test dependency group.
- Python SDK contract generation/public signature tests: **25 passed**, exit 0.
  Log: `mvp-b1-python-contract-tests.log`. Mutation types changed; no stable
  organization RPC method was introduced.
- Affected-package formatting check: exit 0. Initial Clippy reported a preexisting
  redundant binding in the touched app-server constructor; simplified it and
  reran. Final Clippy passed with warnings denied; all four schema bundle comparison
  checks passed (the schema-writer helper is intentionally ignored during normal
  tests and was run separately above). Final CLI rebuild/requalification passed. Logs: `mvp-b1-clippy-final.log`, `mvp-b1-schema-tests.log`.

- Rebuilt CLI from B1 commit `0ce412ec16`: exit 0. Physically entered
  reparenting, immutable role-version creation, one-row pagination, invalid page
  rejection, backup, fresh restore and restore-collision commands. Both invalid
  operations exited 1. Final read-only comparison matched all seven tables:
  nodes 3, role versions 2, goals 1, projects 1, tasks 1, memberships 1, audit 11.
  Evidence: `mvp-b1/` JSON/error files and `mvp-b1-cli-final-build.log`.

B1 defines proposed work;
no background task, integrated layout/editor, same-user sandbox denial or live
provider behavior is claimed from these checks.

## B1/M2 store verification

On 2026-10-10 UTC, Linux x86_64, Rust 1.95.0, from `codex-rs` after documented
activation:

- `cargo test --locked --offline --profile dev-small -p codex-state --lib -j 4`:
  **233 passed, zero failed/ignored**. Log:
  `/workspace/.onboarding/qualification/mvp-state-tests.log`.
- Five new store tests exercise actual SQLite WAL/FULL durability settings,
  stable identity after reopen, consistent backup/fresh restore, corruption,
  truncated/missing initialized data, migration checksum mismatch, real SQLITE_FULL
  rollback and unsafe directory/database symlink permissions. No storage mocks.
- `cargo clippy --locked --offline --profile dev-small -p codex-state --lib -j 4 -- -D warnings`:
  exit 0. `cargo fmt -p codex-state -- --check`: exit 0.
- Initial compilation caught SQLx 0.9's dynamic-SQL guard in the disk-full test;
  the integer-only PRAGMA assignment is explicitly audited using existing
  `AssertSqlSafe`, then checks were rerun. Full-repository formatting initially
  reported the new unformatted files; package formatting was fixed and rechecked.

This qualified the initial storage slice. Subsequent M3 commands and checks are
recorded above. Agent sandbox exclusion and the complete MVP are outstanding;
no live provider request or broad Bazel/platform suite was run.

## MVP planning verification

Planning-only update on 2026-10-10 UTC: no application source, dependency or
lockfile changed. Checked the revised scope against recorded foundation evidence;
pending approval replay on reconnect is not presented as approval crash recovery.
No application tests or live requests were rerun for this documentation change.

- From the repository root, activate the documented environment and run
  `pnpm exec prettier --check AGENTS.md 'docs/development/*.md'`: exit 0.
- Local Markdown path/anchor validation: 16 files checked; 85 local links/anchors
  valid after the architecture qualification link was added.
- `git diff --check`: exit 0. Reviewed changed paths/diffs for documentation-only
  scope; MVP A01–A21 remain future product acceptance tests, not passed results.

## Current M0/M1 evidence

Owner-authorized foundation work is recorded in [qualification](qualification.md).
Official Neovim v0.12.6 passed twelve real embedding assertions on the corrected
helper's two reruns and was physically edited/saved in a PTY. These checks verify
editor APIs; no Workspace pane or live model behavior is claimed.
Real CLI/package build and two twelve-assertion daemon runs passed. Full SDK retest:
49 passed, zero failed, five suites including MCP conformance. The actual attached
TUI accepted input and resumed its saved thread. The daemon requires authorized
unsandboxed execution in this cloud machine because the outer sandbox hides its
socket directory; see qualification for the diagnosis and limits.
Existing Unix socket transport tests passed 11/11 unchanged outside that sandbox;
the masked-root attempt had seven passes/four environment-related failures.

## Phase 0 results

Baseline `8026444c98cd9c6db9e7c953b44a20ea4f93c7b6`; Linux x86_64 cloud host.
Rust 1.95.0; Node 24.19.0; pnpm 10.34.5.
No application source changed. Existing tests were used; no new feature tests
were written because implementation is not authorized.

| Command / working directory                                                                                            | Result                                              | What this verifies                                                                             |
| ---------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `cargo test --locked -p codex-file-search -j 4`, codex-rs                                                              | 17 passed, 0 failed/ignored; binary harness 0 tests | Real Rust dependencies and file-search library behavior                                        |
| `python3 -m unittest discover -s .github/scripts -p 'test_*.py'`, repository root                                      | 80 passed                                           | Repository maintenance/release helper tests; test messages contain expected simulated failures |
| `pnpm --filter @openai/codex-sdk test --runInBand tests/exec.test.ts`, repository root                                 | 1 suite, 14 passed                                  | SDK execution unit contracts; subprocesses are mocked, not a live model/CLI validation         |
| `cargo run --locked -p codex-file-search -j 4 -- --json -C /workspace/codex/codex-rs/file-search Cargo.toml`, codex-rs | Exit 0, actual JSON match for Cargo.toml            | Real existing CLI invoked in a PTY; not the main Codex TUI                                     |
| New Markdown formatting, local links, whitespace and allowed-diff check                                                | Completed before final Phase 0 report               | Documentation integrity; not application readiness                                             |

Machine-local evidence is under `/workspace/.onboarding/phase0/`:
`file-search-tests.log`, `script-tests.log`, `sdk-exec-tests.log`,
`file-search-smoke.jsonl`, and final documentation checks. Logs are generated
outputs, not committed secrets or application files. Counts are summarized here
for sessions where the machine-local logs are unavailable.

Earlier onboarding built the TypeScript SDK and installed frozen dependencies.
Its full SDK suite had 18 passes and 31 failures because the full Codex binary
was missing. Phase 0 did not rerun that integration suite or call it passing.
The narrower 14-test unit run above is distinct.

Phase 0 did not build/run the full Codex CLI/TUI, install/run Neovim, invoke a
live model, trust/run upstream Ponytail hooks, run all Rust/Bazel suites, exercise
daemon crash/restore, or test 1,000 agents. Those remain implementation gates.

## Existing environment activation

From this prepared cloud environment, initialize each shell:

```bash
export CARGO_HOME=/workspace/.onboarding/cargo
export RUSTUP_HOME=/workspace/.onboarding/rustup
export npm_config_cache=/workspace/.onboarding/npm-cache
export XDG_DATA_HOME=/workspace/.onboarding/data
export XDG_CACHE_HOME=/workspace/.onboarding/cache
export XDG_STATE_HOME=/workspace/.onboarding/state
export PATH="/workspace/.onboarding/cargo/bin:/workspace/.onboarding/pnpm/node_modules/.bin:$PATH"
export PATH="/workspace/.onboarding/neovim/v0.12.6/nvim-linux-x86_64/bin:$PATH"
```

Use existing checkouts. No worktree creation is needed for environment setup.
Do not dump credential/environment values. Frozen installation is
`pnpm install --frozen-lockfile --store-dir /workspace/.onboarding/pnpm-store`;
Phase 0 used already installed dependencies and did not reinstall.

## Future validation layers

1. Unit/state-machine tests: task transitions, cycles, expected revisions,
   permission/grant composition, reservation arithmetic, revoked credentials,
   dirty-base/changedtick conflicts and Unicode indexing.
2. Storage/protocol integration: actual SQLite, transactions, migration/restore,
   duplicate inbox/outbox delivery, pagination/replay, schema compatibility and
   approval reissuance. Do not mock away durability or authorization boundaries.
3. Runtime integration: real app-server/daemon, existing execution adapters,
   sandbox/tool limits and fixture project tasks; inject interruptions at meaningful
   start/dispatch/persist checkpoints.
4. Real terminal/editor: actually launch the built application in a PTY/terminal,
   send keyboard commands, navigate views, change pane sizes, submit work,
   decide an approval, inspect results, and quit/reconnect. Then perform actual
   Neovim edit/save/undo/diff/follow/conflict behavior.
5. Live bounded pilot: real authorized provider request and real local artifact,
   with spending/usage recorded. Fixture models alone do not prove provider behavior.
6. Qualification: declared registry/concurrency targets, backpressure, idle
   behavior, crash/restore and platform matrix.

Use existing Rust tests/nextest, TypeScript Jest, Python unittest, PTY utilities
and meaningful fault-injection fixtures. Add a terminal driver only if existing
tools cannot exercise needed inputs; do not adopt a large UI framework merely
for screen snapshots. Snapshot tests complement, not replace, real interaction.

Choose tests for affected behavior and repository CI requirements. Do not expand
into unrelated suites once evidence is sufficient. Run formatting in check mode
for source; avoid update-snapshot/codegen commands unless the approved change
requires them.

## Interactive test recipe after implementation

- Create a temporary fixture project/state directory and record starting Git/
  filesystem state. Preserve existing user files.
- Start the real daemon with approved test settings and initialize handshake.
- Launch the actual TUI in a terminal; type real commands to define an agent,
  create/authorize a project and launch work. Record the exact supported syntax
  after implementation, not an invented Phase 0 command.
- Switch Console -> Projects -> Operations; navigate tree using keyboard; verify
  displayed owner/status against backend records.
- Enter Workspace; in actual Neovim insert text, Escape, undo, write, reopen,
  split/diff and resize. Verify disk output and dirty-buffer rejection.
- Enter Review, inspect current artifact/test evidence, reject/reapprove as
  authorized. Detach TUI while work runs and reconnect.
- Kill/restart only the fixture daemon and test safe attempt/approval/reservation
  recovery. Restore a fixture backup and compare canonical records.
- Capture failures, fix approved implementation/setup, rerun affected checks,
  and record any unsupported scenarios. Do not mark incomplete behavior done.

## Evidence record template

For each task/commit record: commit SHA, OS/architecture/tool versions, provider
and permissions scope (no secrets), fixture/input, exact command and directory,
exit code, test counts (passed/failed/skipped/unrun), interaction keys and observed
behavior, artifact/log path plus hash, failure diagnosis, rerun result and limitation.
Tie results to the current artifact revision. Mocked, manual, live and synthetic
evidence must be distinguishable.

## B3 packaged CLI qualification

The complete current package was built with the existing package script. Commands
were physically entered in a bash PTY against the actual detached daemon.
`/workspace/.onboarding/qualification/mvp-b3-cli/qualification.json` records
13 passing assertions: schedule, configured hierarchy, pinned role/policy, bounded
delegation, context/inbox/ack, yielded parent, shared root usage (13 requests/195
tokens), real child/root checks, hash-verified import, original isolation and
owner final acceptance. `waiting.json` records an inactive waiting parent.
The provider was deterministic localhost SSE; no external model was called.
Both fixture services were stopped after qualification.

The first attempt failed because gpt-5.4 is absent from this checkout’s catalog
and its fallback advertises no apply_patch. The fixture now uses catalogued
gpt-5.5; the exact three-tool assertion remains intact. A second run caught
an expected-revision conflict when relinking the reused project; the final run
created a fresh project and passed. Original failure evidence remains retained.
Current CLI build, production Clippy, state/native/wire/contracts and formatting
gates are documented above. B4/B5 and broader B6 gates remain outstanding.

## B4 layouts — automated checkpoint

From `codex-rs`, with onboarding activated and `CARGO_INCREMENTAL=0`:
`RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -j4
-p codex-tui --lib organization::tests` passed **5**, ignored **0**. It proves
1,000-deep iterative projection/search through collapsed branches, all-preset
private preferences and alias rejection, stable selection across refresh, stale
selection denial/frozen approval identity and narrow/wide sanitized rendering.
The same current test binary ran `slash_command::tests` (**5**),
`chatwidget::tests::slash_commands` (**136**) and
`app::tests::transcript_composer` (**5**), all passing without ignored tests.
These 151 tests are targeted; the 5,717-test frontend suite was not run wholesale.

`cargo clippy --locked --offline --profile dev-small -j4 -p codex-tui --lib
-- -D warnings` and `cargo fmt -p codex-tui -- --check` passed. No production
dependency was added. Machine-local logs: `mvp-b4-layout-tests.log`,
`mvp-b4-slash-regression.log`, `mvp-b4-slash-dispatch-tests.log`,
`mvp-b4-overlay-regression.log`, `mvp-b4-clippy.log`, `mvp-b4-format.log` under
`/workspace/.onboarding/qualification/`.

The first compile required a selection type annotation and exhaustive slash
routing. Offline testing then found missing already-pinned vt100/vte archives;
`cargo test --locked` downloaded those without changing the lockfile. The first
link failed on the 32 GiB workspace's full disk (no tests ran). Its log remains
`mvp-b4-test-link-disk-failure.log`. With no compiler running, obsolete first-party
library variants were retired using the latest CLI dependency fingerprints and
the actual test link inputs; current dependency sets, sources and state remained.
The corrected offline rerun passed. Physical TUI interaction/full CLI build remain
pending, so these checks do not yet qualify B4 or the complete MVP.

## B4 final physical qualification

Corrective `organization::tests`: **6 passed, 0 failed/ignored**. The current
binary repeated slash enum **5**, slash dispatch **136** and overlay **5** checks,
all passing. Production Clippy with warnings denied, formatting, full CLI build
and complete package assembly passed. Logs include `mvp-b4-captured-review-tests.log`,
`mvp-b4-captured-review-clippy.log`, `mvp-b4-corrected-cli-build.log` and
`mvp-b4-corrected-{slash,dispatch,overlay}.log` in the qualification directory.

Actual packaged CLI TUI was launched in a bash PTY through `script`, using the
private fixture home and localhost Responses provider. At 120×30, `/layout
operations`, collapse/expand/search, node pause/resume, pane widths/visibility,
Projects/Workspace/Review and Console were physically exercised. At 50×20 a
restart restored the selected layout and pane settings; all four dashboard
presets rendered stacked panes. Private preferences mode 0600 was verified.
Closing the TUI left a real approval pending with no report and no auto-approval.

The old 600-second grant expired during rebuilding; confirmation of its exact
approval was rejected as stale. `expired.json` retains that failure. A fresh
owner grant reached waiting approval; physical `a`, PageDown and `y` admitted the
exact native command. The real report and immutable oracle reached needsReview.
Physical `c`, typed instructions, Enter and `y` paused the task and persisted the
changes request. A fresh revision-checked grant reran the task; `v`/`y` accepted
its exact current digest. Inspection showed the actual SHA256 and check exit zero.
`qualification.json` verifies acceptance, current grant usage (2 requests/30
reported tokens), actual artifact SHA, real check and original-project isolation.
The full fixture sequence recorded five deterministic provider requests/75 tokens,
including the expired one. No external model was called.

Evidence: `/workspace/.onboarding/qualification/mvp-b4-cli/{tui-terminal.log,
tui-narrow.log,tui-corrected.log,paused-nodes.json,pending.json,expired.json,
changes.json,accepted.json,qualification.json}`. Actual Neovim inside Workspace,
safe application and broader release gates remain B5/B6.

## B5 intermediate evidence (not release qualification)

The initial real Neovim tests passed two checks with official v0.12.6. After
adding application/changedtick fences, the combined organization unit command
passed **5 application/runtime**, **31 state** and **10 layout/editor** tests,
with no failed or ignored tests. Logs: `mvp-b5-editor-tests.log` and
`mvp-b5-organization-unit-tests.log` under the qualification directory.
This covers actual Neovim edit/save/undo/Unicode/diff/resize/follow/exit,
changedtick/dirty flags, bounded decoding and real checked filesystem replacement
with permission preservation. Constructed hashes in state tests are ledger
fixtures; native API and physical application evidence are still required.

Production Clippy passed before the final small safety refinements; protected
preview Clippy then passed with the latest source (`mvp-b5-preview-corrected-clippy.log`).
The first pass rejected await-held MutexGuard; a bounded owned semaphore permit
now serializes application and remains held by native writes if the RPC cancels.
Frontend unwrap/deprecated-cell/style lints were corrected. A narrowly justified
RGB exception preserves native Neovim highlight values; the rest of the frontend
color policy is unchanged. The shared preview reader initially needed an explicit
existing path-utils dependency in app-server; that was added and the check passed.
No assertion or authority guard was disabled.

The two current wire contracts passed (`mvp-b5-wire-tests.log`), including the new
owner Apply variant. Shared-reader/preview/actual dirty-editor dispatch tests,
updated schemas, native API, full CLI physical editor/application and B6 gates
remain pending. Do not treat these intermediate checks as B5 completion.

The first schema comparison found an incorrectly selected experimental output
root: the generation command wrote a new nested directory instead of updating
the existing compressed experimental bundle. Stable generation and three
comparisons passed; the experimental comparison failed as intended. Wrong-output
files were moved intact to `mvp-b5-wrong-schema-root` in qualification evidence;
the corrected writer uses the same repository schema root with experimental=1.
The corrected comparison passed all four schema checks. The shared protected reader test and final combined unit run passed: five app-server, 31 state and 13 TUI/editor/preview tests, all with zero failures or ignored tests. Full state regression then passed all 259 tests, and regenerated Python contracts/signatures passed all 25 checks. With no
compiler running, two obsolete 435 MiB frontend test executables were retired;
current tests/dependencies/source/evidence were retained. No schema assertion
was changed or disabled.

Final intermediate logs: `mvp-b5-schema-final-comparisons.log`, `mvp-b5-shared-reader-tests.log`, `mvp-b5-preview-final-unit-tests.log`, `mvp-b5-state-full-tests.log`, `mvp-b5-sdk-generation.log` and `mvp-b5-sdk-contracts.log`. Four schema comparisons passed; the generator helper remains intentionally ignored in the comparison command and was run explicitly for both bundles. The complete CLI build and physical/native API gates remain pending.


## B5 physical qualification and final corrections

Actual packaged CLI/daemon with checksum-verified official Neovim v0.12.6 passed
12 physical assertions recorded in `mvp-b5-cli/qualification.json`. Logs:
`tui-editor.log`, `tui-apply.log`; owner views: `dirty-blocked.json`,
`base-blocked.json`, `applied.json`, `independence-current.json`. Real Unicode
editing/save, bracketed paste/undo, protected Follow, native diff, focus/resize,
dirty exit/Apply refusal, changed original preservation, exact owner Apply/hash
and post-Apply immutable oracle passed. The saved human text was preserved in
`human-saved.txt`. Killing only editor PID349467 during native execution left
daemon PID347317 running and the new task reached NeedsReview (2 requests/30
reported tokens). This provider is deterministic localhost; no paid call occurred.

The first API batch passed two/failed two: the new negative assertion used the
success-only response helper, and a parallel delegated case timed out. Correct
error-response handling and serial execution passed three/failed one: duplicate
Apply was correctly denied by changed-base validation before the expected receipt
message. Owner processing now detects an already recorded digest before preparing
files; the protected transaction uniqueness check remains. Final native rerun is
pending. Logs retain both failures (`mvp-b5-native-api-tests.log`,
`mvp-b5-native-api-corrected-tests.log`). No rejection guard was disabled.

The slow editor-crash fixture initially finalized before its native command
finished; `timeout_ms` was replaced with the supported `yield_time_ms` and a fresh
bounded authorization passed. First failed evidence remains. Direct editor
key/paste now reject frozen state; mouse is ignored while frozen or unfocused.
Final fence production Clippy passed. A redundant TUI-only test feature build was
explicitly cancelled (130) to avoid duplicate runtime compilation/disk exhaustion;
the combined package command then passed five app-server, 31 state and 13
TUI/editor/preview checks, zero failures or ignored. Logs:
`mvp-b5-final-fence-clippy.log`, `mvp-b5-final-fence-editor-tests.log` (cancelled),
`mvp-b6-combined-unit-tests.log` (successful B5 corrections; B6 runtime patch had
not yet been applied). Only verified stopped duplicate fixture executables and
obsolete pre-preview test executables were retired for disk; all state, source
and logs remain, and the current complete installed daemon package is retained.

B6 actual pilot, scale, concurrency, idle and restore evidence is recorded in
[release qualification](release-qualification.md). Current final source checks
remain pending; do not claim the full live-provider release gate passed.


Current B6 source passed production Clippy with warnings denied, affected-package
formatting and the combined library command: **7 app-server, 31 state and 15
TUI/editor/preview/pricing** tests, no failures/ignored. Logs:
`mvp-b6-corrected-production-clippy.log`, `mvp-b6-format-check.log`,
`mvp-b6-final-combined-tests.log`. New cases reject invalid owner limits and
CodeModeOnly before native execution, validate exact quote provenance and reject
negative usage, mismatched provider/model, unsafe price files/aliases/rates.
These are not yet proof of actual four-active/zero-request admission in the updated
packaged daemon. The final native API and full CLI stages remain underway.

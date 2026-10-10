# Testing procedures and evidence

## TUI creation slice — automated checkpoint, 2026-10-10 UTC

Added fixed owner forms using the existing TextArea and typed owner RPCs.
New checks reject invalid file paths/budgets/check syntax and missing project roots,
preserve multiline Unicode and stable choice IDs, retain canceled review drafts,
require explicit confirmation, and bind execution to the captured task revision.

From codex-rs with the pinned activation, ran
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -j4 -p codex-tui --lib organization -- --nocapture`:
**22 passed, 0 failed**, including actual official Neovim editing/diff/dirty-buffer
checks. Formatting and production `cargo clippy --locked --offline --profile dev-small -j4 -p codex-tui --lib -- -D warnings` passed. This checkpoint does not yet qualify the new production
TUI forms; a fresh full CLI/package and physical interactions remain required.
Ponytail review reused existing input/widgets/RPCs without new dependencies or a
form framework. Correctness review retained host enforcement and exact decisions.

## Completed live GPT qualification — 2026-10-10 UTC

Owner published the network changes and requested “begin”. The restored environment
retained the prepared private home, current package, state, pinned tools and official
Neovim. OpenAI API/pricing/model HTTPS requests now work through the supported
proxy and TLS route. Metadata lists all four allowed domains but still reports
unknown readiness; actual request/application results are recorded independently.

No application source, manifest, lockfile or dependency changed. Ponytail reused
the complete source package, detached daemon, typed owner RPC, native sandbox,
existing model_instructions_file override, actual Neovim and installed Python
MessagePack/terminal capture utilities. No broker, editor clone or retry layer was
added. Correctness/qualification and simplicity review retained all host safeguards.
Earlier affected Rust/Clippy/package/layout tests remain the unchanged source
baseline; the full repository/platform suite was not rerun for this live-only work.

### Commands and real operating scope

From `/workspace/codex`, activate `. /workspace/.onboarding/activate.sh`.
Set CODEX_HOME and CODEX_SQLITE_HOME to
`/workspace/.onboarding/qualification/mvp-followup/live-gpt/home` and use
`/workspace/.onboarding/qualification/mvp-followup/package-v2/bin/codex`.
`codex app-server daemon start` exited 0, owned PID 1606, unchanged hash-matched
private installation. Real owner IPC/native TUI runs used authorized unsandboxed
execution because the outer sandbox masks protected socket/process paths.

Fetched official `https://openai.com/api/pricing/`,
`https://developers.openai.com/api/docs/models/gpt-5.5` and
`https://developers.openai.com/api/docs/pricing`: HTTP 200, curl exit 0.
Model page explicitly lists input $5, cached input $0.50, output $30 per million;
HTML/text copies, SHA256 and timestamp are retained in `pricing.json` and sibling
`live-model-docs.*` / `live-rate-table.*`. Owner TUI quote is matching provider/model,
regular mode-0600 data with $30/million conservative total-token rate.

Provider: real official HTTPS Responses, env_key OPENAI_API_KEY, model gpt-5.5,
low reasoning and zero request/stream retries. No key value was printed or written.
Native agents retain isolated workspaces, immutable owner inputs, no network or
inherited credentials, exact declared artifacts/check argv, one attempt/grant and
240-second deadlines. Final scopes: single 10,000 tokens/5 requests with approvals;
delegated 25,000/14 with exactly one allowed child and checked internal acceptance;
document 12,000/6, immutable facts/oracle. Price-based admission checked all previous
receipts plus the next conservative scope before starting another grant.

Run `python /workspace/.onboarding/qualification/mvp-followup/live-gpt/run.py`
with `single`, `delegated` or `document`. Each final run exited 0; their corresponding
logs/JSON/captures retain physical input evidence. These accepted/applied tasks
cannot be rerun as fresh qualification; preserve IDs and use new explicit scope
only for authorized new work. `verify.py` exited 0, checking actual applied bytes,
immutable inputs, independent owner regressions, child identity, exact digests,
budget/usage parity and all five package hashes.

### Final live results

| Pilot | Requests/reported tokens in successful grant | Evidence |
| --- | --- | --- |
| Single coding | 4 / 8,015 | Positive/zero/negative/fractional addition and explanation oracle passed; two exact native tool approvals physically confirmed; approval survived actual /quit and reconnect |
| One-child coding | 11 / 25,101 | Arbitrary configured delegate proposed/started through organization tool, child immutable checks passed and artifacts imported; root/child shared usage agrees; further model requests occurred after actual /quit |
| Local-fact document | 4 / 8,019 | Exact supplied facts/bullets, heading and bounded length passed immutable provenance oracle |

Every pilot physically switched to Workspace, opened actual official Neovim readonly
native diff, accepted exact current evidence with v/y, proved original unchanged,
confirmed Apply with A/y, obtained Completed receipt, ran the independent original
oracle and reopened the saved original in real Neovim. All three native checks and
all three applied-original checks passed. Verification independently reran the
original checks: **3/3 exited 0**, and all artifact/immutable-input hashes matched.
No duplicate request-ID or owner-timeout frame was observed. Eleven native-cell
PNG/text captures are retained; all three native diff PNGs were visually inspected.
Archive/report in
`/workspace/artifacts/cli-live-gpt/`. These show actual GPT outputs, not concepts
or deterministic model fixtures.

### Failures, corrections and spending

Retained every failure; none counts as a passing pilot:

- Normal first-run CLI model migration dialog: harness waited for the dashboard;
  corrected physical selection of “Use existing model”. No task authorized/call
  made before that failure. `single-initial-model-dialog.log`.
- Harness searched task IDs in the Operations agent tree; corrected F5 Review
  selection. Two real requests/3,451 tokens had reached a pending native approval;
  owner paused the grant without tool release. `single-initial-task-filter.log`
  and matching result JSON.
- /quit persisted Console, while harness expected Operations on reopen; restore
  its own Operations preference before opening a new native terminal buffer.
  Paused another grant with two requests/3,453 tokens, without tool release.
  `single-initial-reopen-preset.log` and JSON. Successful retry proves approval
  persistence and correct physical reconnect; no user editor/config was overwritten.
- First delegated scope stopped at its real 16,000-token gate after eight
  responses/16,262 tokens; next request was denied. A checked child result was
  retained but root was not accepted. `delegated-token-ceiling.log` / JSON.
- Next scope stopped at its ten-request gate after ten responses/22,622 tokens,
  denying its final response. Real background progress with TUI closed was recorded.
  `delegated-request-ceiling.log` / JSON. Fresh final scope retained one child,
  14 requests and a 25,000 reported-token ceiling; completed at 11/25,101.
  Its last already-admitted request explains the 101-token excess, consistent with
  the existing documented in-flight allowance. No gate/approval was disabled.
- A proposed 26,000-token rerun projected $1.01126 including prior receipts and
  was refused before inference; corrected to 25,000 with $0.98126 conservative
  admission estimate. `delegated-conservative-admission.log`.
- Completed delegated output reached NeedsReview, then harness expected a
  nonexistent organization/list `records` field; corrected to the existing `data`
  field and resumed only physical review/Apply with **zero additional model calls**.
  `delegated-review-list-shape.log` / JSON. Current-grant child filtering excludes
  accepted children from earlier failed grants without deleting their evidence.

Seven root grants, including four earlier paid failures, report exactly **41
responses / 86,923 tokens**. Existing per-response token_usage_record entries were
deduplicated by provider response ID and matched canonical root budgets exactly.
Older interrupted token_count UI snapshots omit the latest receipt while a tool
is pending; the per-response records correctly include it. Totals: 82,841 input,
42,496 of those cached, 4,082 output; 319 reasoning tokens are already included
in output. Cost calculation:
`((82841 - 42496) * 5 + 42496 * 0.50 + 4082 * 30) / 1000000 = $0.345433`.
This includes all failed attempts and stays below the $1 working ceiling.
It is an official-rate token estimate; provider invoice/taxes/non-token charges
were not verified, and local reported-token limits are not monetary hard caps.

Full evidence under `mvp-followup/live-gpt/`: prepared/scoped IDs, original files,
pricing receipt, all failure logs/JSON, result-single/delegated/document.json,
verification.json and cleanup.json. Source and installed manifest/CLI/Code Mode
helper/ripgrep/Bubblewrap hashes match the qualified package; CLI SHA256 remains
`6152cbbd0e4b47877862685dc5d54f94f29ba3e6fd23f05de24b760089255ae7`.
Only owned daemon PID 1606 was stopped with its same home/current package, exit 0.
Retained zombie has zero execution; final receipt count remains 41. No qualification
service is intentionally running. A05 is closed for this bounded envelope;
broader providers/models, production billing guarantees and later roadmap phases
remain separate scope.

Saved updated complete, tested startup instructions in the environment draft
(start_skill only), preserving install/network/credential fields. It points to the
current qualified package, stopped services, completed paid-test status and real
evidence. Save returned status saved/requires_publish; future publication/restoration
is separate from the already successful current-instance qualification.

## Live GPT preparation checkpoint — 2026-10-10 UTC

The owner's “ok begin” resumed the already authorized compatible-GPT pilots.
No application source changed, and no previously qualified suite was rerun.
Ponytail reused the existing complete package, private daemon, typed owner RPC,
native-tool scope and meaningful immutable fixture checks.

- Supported unauthenticated `curl https://api.openai.com/v1/models` returned 401
  with exit 0, replacing the prior CONNECT denial. Python stdlib urllib with
  inherited proxy/TLS trust and the existing environment key returned authenticated
  HTTP 200; gpt-5.5 is listed. No key value was printed or written. Safe receipt:
  `mvp-followup/live-access.json`. Metadata still reports unknown readiness and
  omits the API host; actual operation success is not a metadata-readiness claim.
- Official pricing checks for `https://openai.com/api/pricing/`,
  `https://developers.openai.com/api/docs/models/gpt-5.5` and
  `https://platform.openai.com/docs/pricing` returned CONNECT 403, curl exit 56,
  target HTTP 000. Current rates are unverified; paid execution remains blocked.
  Saved their domains additively alongside api.openai.com, preserving the existing
  package-manager preset/key binding. Draft save succeeded with requires_publish;
  it does not apply runtime configuration. Updated saved startup instructions to
  remove stale missing-key/MVP claims and point to current status/live fixtures.
- From `/workspace/codex`, activated `/workspace/.onboarding/activate.sh` and ran
  the final package with CODEX_HOME and CODEX_SQLITE_HOME both set to
  `/workspace/.onboarding/qualification/mvp-followup/live-gpt/home`:
  `codex app-server daemon start` exited 0, installing the complete private package
  and starting owned PID 374053. Custom provider points directly to the official
  HTTPS Responses endpoint with env_key OPENAI_API_KEY, low reasoning, zero retries
  and the repository's unchanged model catalog. Concise instructions use the
  existing model_instructions_file override; host security/budget gates remain.
- `python .../live-gpt/prepare.py` exited 0 through protected owner IPC with
  authorized unsandboxed execution. It created two arbitrary agent names and
  three fresh proposed projects/tasks: single addition repair/explanation, one-child
  delegated repair and an exact-source local-fact document. Immutable checks cover
  positive/zero/negative/fractional addition and document provenance. Every task
  has zero attempts and zero requests; no authorization mutation was sent.
  `prepared.json` retains task IDs, original bytes and provisional scopes. Check
  those scopes against verified prices before authorizing; they are not a billing cap.
- `python -m py_compile .../live-gpt/prepare.py` exited 0. This is only a harness
  syntax check; the actual definition/zero-usage RPC assertions above validate setup.
  Independent SHA256 comparisons matched all five final source/private installed
  package files. `live-gpt/preparation-checkpoint.json` records their hashes and IDs.
- Stopped only this owned daemon with its same home/current package; exit 0 and
  stopped receipt. Existing fixture state/source/packages remain untouched. To
  free installation space, retired one regenerable completed TUI test executable
  after preserving its SHA256/size and successful test/build logs; receipt:
  `live-cache-retirement.json`. No build, application source, canonical state,
  user data, current package or test evidence was removed.

**Paid inference: zero. Live generation, delegation quality, approvals/editor review
of GPT output and Apply remain unrun.** Enable the saved official pricing domains
through environment settings, verify current rates, adjust scopes to the conservative
$1 total working ceiling, then restart the existing private daemon and run those
three prepared tasks. Keep fixture and live usage/results distinct.

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


Final native organization API rerun passed **four tests**, zero failed/ignored,
including exact Apply and both conflict/duplicate rejection paths plus native
isolation, crash/restart fencing and delegated checked work. Command uses the
existing `--test all suite::v2::organization -- --nocapture --test-threads=1`,
locked/offline/dev-small, CARGO_INCREMENTAL=0/RUST_MIN_STACK=8388608. Evidence:
`mvp-b6-native-api-tests.log`. B5 is locally qualified; full B6 release still needs
current-package capacity/admission/quote checks and the pending bounded live pilot.

## B6 four-active scheduling failure and correction

The complete CLI/package at `56929b5b18` built and installed successfully. Its
managed daemon executable matched the package SHA256 recorded in
`mvp-b6-cli/daemon-binary-proof.json`. Four of five native tasks reached NeedsReview;
the fifth and concurrent owner requests hit the single-connection pool timeout.
Logs `mvp-b6-concurrency-four.log` and `mvp-b6-model-admission.log`, canonical failed
task/action receipts and database logs are retained. No assertion was disabled.

Renewing a lease inside a selected branch suspended the work holding its database
connection. The corrected driver polls work/renewal together and keeps cancellation
and the time budget responsive. Added a deterministic shared-resource regression
that requires work to continue while renewal contends, then verifies deadline
expiry with renewal still blocked. Initial compilation caught a borrowed stop-token
move; use its clone, with the failure retained in `mvp-b6-heartbeat-unit-tests.log`.
Corrected combined units passed eight app-server (including contention/deadline),
31 state and 15 TUI/editor/pricing cases; none failed or were ignored. Production
Clippy with warnings denied passed. Native API/CLI/current-native rerun remains
underway. Logs: `mvp-b6-heartbeat-unit-corrected.log`,
`mvp-b6-heartbeat-clippy.log`.

Idle current-daemon admission rerun passed a catalog-declared CodeModeOnly rejection:
zero provider requests/tokens, no prepared workspace. An actual restart with active
limit 5 denied authorization before any grant/attempt or inference while owner
definition APIs stayed readable; the invalid-limit diagnostic is JSON-RPC -32603,
a server configuration error. Read-only canonical counts verified zero grants and
attempts. Evidence: `mvp-b6-model-admission-corrected.log`,
`mvp-b6-invalid-limit.log`, `mvp-b6-cli/admission-model.json` and
`admission-invalid-limit.json`. No provider calls occurred during either denial.

## B6 corrected package and live native Follow

At `8da15f446f`, corrected formatting/production Clippy, combined units (8/31/15),
all four native API cases and full CLI/package passed. Native API log:
`mvp-b6-heartbeat-native-api.log`; full CLI: `mvp-b6-heartbeat-cli-build.log`. Package
assembly initially omitted the required CODEX_REPO_ROOT; the script rejected it
before mutation, and the corrected assembly passed. The installed daemon matched
package SHA256 `e7b1897f6aca288807b3d66e02a90edbd455337edf0cc8d97ae1c02409ea1ad3`.

Four active/five tasks and restored-default two/three tasks passed: peak remained
within the configured limit, extra work queued, every task reached NeedsReview,
all immutable native checks exited zero and each charged exactly two requests/30
reported tokens. Evidence: `concurrency-4.json`, `concurrency-2.json`,
`mvp-b6-concurrency-four-corrected.log`, `mvp-b6-current-concurrency-two.log`. Current
model/invalid-limit denials also passed (`mvp-b6-current-*-admission.log`,
`mvp-b6-current-invalid-limit.log`). Actual current TUI quote provenance/billing
limitations were displayed; `price-physical.json`, `tui-price.log`,
`tui-final-follow.log`.

The first timed two-phase native file update completed and its owner oracle passed,
but interaction reached the second phase too late to prove live reload. Preserve
`initial-follow-*.json` and the initial result log. A new independently owner-
approved two-stage fixture made observation deterministic. Native writes passed
three requests/45 tokens and the immutable owner regression; original project
remained unchanged. Physical commands inside official Neovim wrote its actual
line, readonly/modified flags, getpid/has('nvim') and winsaveview to evidence files.
Focused editor retained phase one while the task file changed to phase two; host
focus reloaded phase two with line 1/column 7 and the entire native view retained.
Evidence: `follow-physical.json`, `follow-editor-{before,focused,after}.json`,
`follow-finished.json`, `tui-final-follow.log`. No custom editor model was used.

Live Follow also exposed a redraw loop: synchronous dirty-buffer-query replies
always scheduled another host frame. Suppress response-only wakeups; retain native
notifications, requests and EOF/error wakes. A real native test now requires 12
queries to schedule no draws, then explicit native redraw and editor exit to wake
the UI. Its rerun/current-binary idle measurement remain underway.

The first redraw regression passed the no-query-wakeup assertion but used `redraw!`
as a structured nvim_cmd command name; official Neovim correctly rejected that
syntax (15 passed/1 failed). The test now calls actual nvim_command("redraw!").
No production assertion was loosened; the failure remains in
`mvp-b6-editor-redraw-tests.log`. Corrected full editor regression is rerunning.

The same actual packaged CLI/official-editor idle Follow regression was run against
the prior installed executable: **326 frames in 3.2508 seconds**, failing the
unchanged <=8-frame limit as expected. No model requests or native project writes
were required to reproduce it. Log: `mvp-b6-quiet-follow-before-fix.log`; actual
terminal: `mvp-b6-cli/tui-quiet-follow-before-fix.log`. The corrected package must
pass the identical interaction/count assertion.

The corrected regression passed quiet queries and native redraw, then its EOF
assertion saw the scheduler channel close: the fixture had moved the host's last
FrameRequester into the editor. The real app retains its requester. Preserve that
parent handle in the fixture and pass its clone, exactly as the app does. The
second failure remains in `mvp-b6-editor-redraw-corrected-tests.log`; no EOF or
quiet assertion was removed. Final regression/Clippy/build is rerunning.

Final redraw regression passed, including actual query silence, native redraw and
EOF wake: combined batch **8 app-server + 31 state + 16 TUI/editor/pricing**, zero
failed/ignored. Warnings-denied production Clippy and formatting passed. Logs:
`mvp-b6-editor-redraw-final-tests.log`, `mvp-b6-editor-redraw-clippy.log`,
`mvp-b6-editor-redraw-format.log`. Final full CLI build/packaged quiet-Follow
regression remains in progress. App-server/native API source is unchanged from
the corrected four-test/2–4-active qualification.

The complete CLI rebuild passed in 1m32s (`mvp-b6-editor-redraw-cli-build.log`).
Package assembly and the unchanged physical idle Follow assertion are next.

## Final B6 local qualification — 2026-10-10 UTC

Source `f92e8c2292` was packaged and installed. Every executable/helper/manifest
matched its completed package; final CLI SHA256:
`81d31243e6c3d2cd261d4a538008cb2c029f51c51eb6a25b79b54185310095dc`.
Evidence: `final-package-proof.json`, `mvp-b6-editor-redraw-package.log`.

The first final physical harness matched a cosmetic status phrase as contiguous
bytes; Ratatui emitted it as cursor-positioned fragments. The real task/budget/file
already rendered. Preserve `mvp-b6-quiet-follow-corrected.log` and
`tui-quiet-follow-initial-status-fragment.log`. Readiness now requires that exact
stable task ID, Shared budget and declared report; the native readonly marker is
required. The original <=8-frame assertion is unchanged. The final physical
regression passed: **1 frame / 3.253s**, zero model requests,
real readonly Neovim and regained editor focus. Prior binary: 326 frames / 3.25s.
Evidence: `quiet-follow.json`, `mvp-b6-quiet-follow-final.log`,
`tui-quiet-follow.log`. No application guard/assertion was weakened.

Final-package 1,000-agent Operations PTY navigation passed all 30 keys and deep-node
search: p95 **27.5 ms**, full loaded-tree startup
**1.76s**, RSS TUI **274,776 KiB** / daemon
**290,924 KiB**. Latency includes the conservative 20 ms quiescence interval.
`mvp-b6-current-tui-performance.log`, `tui-performance.json`; original baseline
files retained with baseline- prefix. Exact stable tree/inspector IDs establish
readiness without relying on contiguous cosmetic status bytes.

The final daemon's 30-second idle interval generated no model requests and kept
the same process; `mvp-b6-current-idle.log`, `idle.json`. The owner daemon stop and
provider Ctrl-C completed; the container retains the exited daemon as a zombie
with no execution. Its PID record is removed and no provider process remains.
`final-daemon-stopped.json`, `final-cleanup.json`. All source/state/logs remain;
only redundant, verified stopped fixture executable copies were retired with
hash receipts to preserve build space. Final package is retained.

Production source was frozen during compilation; all final affected units (55),
Clippy, formatting, CLI/package and physical regressions passed. Native API (4),
state-wide (259), layout/slash/overlay (152), contracts/schemas and other recorded
checks remain applicable to unchanged source. Full repository/platform suites and
A05 live-provider pilots were not run. See release-qualification.md for each
criterion and limits; no paid model call was made.

Final documentation verification passed: all 111 local links across 22 development
Markdown files/root instructions resolve (including URL-decoded filenames), and
`git diff --check` is clean. Final evidence JSON and package retention were checked;
the final commit contains only documentation/instructions. Application source is
unchanged from the qualified `f92e8c2292` build.

## Requested CLI screenshots — 2026-10-10 UTC

Captured all five layouts from the actual final packaged CLI, including actual
embedded official Neovim in read-only Follow mode. The CLI ran in official Neovim's
native terminal at 120×36; its external-UI cells/highlights were rendered to five
1440×900 PNGs with installed Pillow. No generated concept art or application changes.
Images, source/hash metadata and the downloadable archive are retained at
`/workspace/artifacts/cli-screenshots/`.

An initial readiness predicate named a different fixture; it was corrected to the
actual loaded Bench agent. A later rapid layout-switch attempt saw
`organization/execution/read transport error: duplicate remote app-server request id 0`.
Waiting for settled refreshes with the existing localhost fixture available loaded
complete task details and actual Neovim. This request-ID issue was recorded, not
fixed or silently treated as passing. Investigate separately; no source regression
or additional release qualification is claimed by these screenshots.

Provider request count remained exactly 50 before/after capture: zero model requests.
Original pane preferences were restored; temporary daemon/provider were stopped.
The screenshots include preserved historical failure cases and intentional denials
in the test task list. These are real qualification fixtures, not live-model work.

## Request-ID and live-pilot follow-up — 2026-10-10 UTC

The owner approved fixing the screenshot startup collision, rapid switching,
real-model single/delegated coding and document pilots, and the complete background/
approval/Neovim/accept/Apply workflow, with Ponytail, testing and periodic commits.
They supplied `OPENAI_API_KEY`, requested minimal spending, and explicitly approved
using a compatible GPT after the catalog identified GPT-6 Luna as CodeModeOnly.
The conservative working ceiling is $1 total; it is a test budget, not a provider
billing guarantee. Do not add/bypass Code Mode or broaden the MVP for these pilots.

Five organization worker calls reused integer ID `0` on the startup transport.
An aborted dashboard can leave an outstanding request until its reply arrives.
The correction reuses existing UUID string IDs from other background TUI calls;
no dependency, transport change or weakening of duplicate detection.

In `codex-rs`, with the pinned environment activated:

```bash
CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -j4 -p codex-tui --lib background_requests_survive_startup_overlap_and_cancelled_dashboard -- --nocapture
CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -j4 -p codex-tui --lib organization -- --nocapture
cargo fmt -p codex-tui -- --check
CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo clippy --locked --offline --profile dev-small -j4 -p codex-tui --lib -- -D warnings
```

The new test uses the real remote WebSocket client with a simulated peer holding
startup ID `0` and a canceled dashboard reply. It exercises pagination, replacement
worker loading, inspection, both mutation types and selected-task refresh before
releasing those old replies. Before correction: **1 failed**, exact duplicate ID
`0` (exit 101). After correction: **17 passed**, including actual official Neovim
editor tests (exit 0). Formatting and warnings-denied production Clippy passed.
The initial test fixture failed compilation because `unwrap_err` required Snapshot
Debug; the cancellation assertion was corrected without changing production types.
Original compile/failure logs remain; no test assertion was relaxed.

Evidence root: `/workspace/.onboarding/qualification/mvp-followup/`.
`request-id-before.log`, `request-id-before-compiled.log`, `request-id-after.log`,
`fmt.log`, `clippy.log`. Complete CLI build, physical qualification and live pilots
are in progress; do not claim they passed until the subsequent evidence is recorded.

The API key became present without printing its value. A normal HTTPS connectivity
check to `https://api.openai.com/v1/models` failed at the proxy CONNECT with HTTP
403 / curl exit 56, before any authenticated or paid request. The running network
policy lacked that host. Using the cloud onboarding setup workflow, `api.openai.com`
was saved as the sole additional custom allowed domain, preserving package-manager
presets and existing credential bindings. Draft save returned `status: saved` and
`requires_publish: true`; saving is not runtime application/publication. Review,
save and publish in environment settings, then recheck normal proxy/TLS access.
Do not bypass the proxy, copy credentials into files/logs, or call a CodeModeOnly
model. Live pilot evidence remains unrun until the supported route succeeds.

Complete CLI build and native package assembly passed (exit 0), using
`cargo build --locked --offline --profile dev-small -j4 -p codex-cli --bin codex`
and the repository packager with pinned existing helpers and `--strip none`.
The private qualification daemon was updated with `app-server daemon update
--from-cli --yes`, then started. All five package files match the running installed
package; `package-proof.json`. CLI SHA256:
`3551b402d216ea5f33e218431efbb0b07449a10c3410bb436012d853de3d597a`.
Outer-sandbox `/proc` inspection was denied; the authorized native inspection
succeeded, matching the actual executable to the verified package.

`python rapid.py provider-offline` physically passed **185 layout commands**:
160 dashboard switches at 20 ms input intervals across 120×36 / 50×20, ten
Console-close/`/layout operations` reopen cycles, and individual preset checks.
No duplicate-ID or owner timeout frame was observed. Cold startup to loaded tree
was 1.452s; this is one functional observation, not a new latency benchmark.
Official Neovim's native terminal/external UI rendered actual CLI cells to PNGs.
Original preferences were restored. Evidence: `rapid-provider-offline.json`, log
and both corresponding PNG/text captures. This ran with the provider stopped and
made no inference request; owner metadata remained available through the daemon.

The first physical harness attempts misused Console's existing function-key
bindings and combined pasted slash text/Enter before native composer submission.
They reached chat search or retained a draft. The harness now waits for the correct
input context and sends a separate Enter for Console slash commands; dashboard
switches still run at 20 ms. These failures remain in the three
`rapid-provider-offline-initial-*.log` files. No application control/error/assertion
was disabled; layouts.md now makes shortcut context explicit.

Ponytail review: lean already; the five-line ID helper reuses installed UUIDs and
no transport framework/dependency is added. Correctness review checked all five
production request sites and late/canceled replies. Native provider-online and
full approval/editor/workflow checks are next, followed by the live access gate.

Provider-online rapid qualification also passed **185 layout commands** at both
sizes, including ten Console close/reopen cycles: `rapid-provider-online.json` and
its log/captures. Both offline/online runs made **zero inference requests**.
Fresh native single-agent coding, delegated coding and document pilots then passed
immutable isolated owner checks, exact acceptance and explicit Apply; independent
checks in the original project exited zero. Usage: 2/30, 7/105, 2/30 requests/
reported tokens. These are deterministic localhost Responses, not live inference.
Evidence: `native-pilots.log`, `pilots.json`, `provider-requests.jsonl` and fresh
`*-followup-v1` project directories. Reusing an old project idempotency key first
triggered the correct project revision conflict before authorization/inference;
`native-pilots-initial-project-key.log` remains. New unique keys passed.

The physical approval workflow discovered another real dashboard defect: a task
created after initial loading became selected through a refreshed search, but
only its basic definition rendered. The worker continued inspecting the previous
task because snapshot updates called Inspect only on the first load. No approval
was sent or executed; its bounded pending task expired. Preserve
`workflow-before-selection-fix.log` / `.json` (one fixture response, 15 reported
tokens, no tool execution). This is an application failure, not a passing workflow.

The minimal correction calls the existing Inspect path when a refresh changes the
selected ID, as well as on initial load. The existing selection regression now
checks deletion/fallback, unchanged selection (no redundant Inspect), and a newly
appearing search match through actual update channels. Tests, production checks,
CLI rebuild and the physical workflow rerun are in progress. No model/approval/
editor authority or backend state logic changed.

Selection correction qualification passed **17 organization/editor tests**, including
both request-ID and extended selection-refresh regressions, actual Neovim tests,
formatting, warnings-denied production Clippy, complete CLI build and package.
Commands match the preceding section; `selection-after.log`, `selection-fmt.log`,
`selection-clippy.log`, `selection-build.log`, `selection-package.log` (all exit 0).
The new source package is `mvp-followup/package-v2/`. Its five files hash-match the
running managed daemon, including CLI SHA256:
`6152cbbd0e4b47877862685dc5d54f94f29ba3e6fd23f05de24b760089255ae7`.
`package-proof-v2.json` records source-file hashes and actual process identity.

The full physical workflow then passed on this package:

```bash
cd /workspace/.onboarding/qualification/mvp-followup
. /workspace/.onboarding/activate.sh
python workflow.py
```

Task `01a1272d-0c5f-7e53-a3e5-9afdf9f85e9d` received two exact approvals using real
TUI `a`, confirmation and `y`. Actual official Neovim showed phase-one bytes in
readonly Follow. The second native command waited eight seconds and completed
phase two after `/quit` exited the TUI; owner reads confirmed Running after exit,
then NeedsReview and immutable check exit 0. Reopened CLI/Neovim showed native diff
against the unchanged original. `v`/`y` accepted the exact result while the original
remained unchanged. `A`/`y` produced a Completed application receipt; the independent
original-project oracle exited 0 and Neovim reopened the applied phase-two file.
Usage was three fixture responses/45 reported tokens; zero paid requests.
Evidence: `workflow.json`, `workflow-ready-checkpoint.json`, `workflow.log` and
`workflow-*.png` / `.txt`; no duplicate-ID or owner timeout frame was observed.

Two harness/setup corrections were needed after the real selection bug was fixed.
The nested editor needs the pinned `activate.sh` PATH, not just the absolute path
used by the outer terminal renderer; `workflow-initial-editor-path.log` remains.
Reusing the outer Neovim terminal after TUI quit requires a fresh clean buffer:
`enew!` discards only that owned, exited test terminal; no editor/project buffers.
`workflow-initial-terminal-reopen.log` and its captures remain; its background work
and checks had already passed. The delay fixture now requests a 30-second command
wait, within its 120-second grant. PTY-wrapper Ctrl-C left its owned provider child
running; after validating PID/cwd/command, that child was stopped and restarted.
`provider-restart-receipt.json`. No checks/authority/dirty-buffer protection were
weakened. Final full-workflow exit code is 0.

Ponytail/correctness review: reuse the existing Inspect path on changed selection,
retain stable-selection caching and frozen approval payloads, and leave stale-detail
identity guards intact. No new abstraction/dependency/backend authority is added.
Final-package native pilot/layout repetition and service cleanup are next.

## Final follow-up checkpoint — 2026-10-10 UTC

Application source `6a4a9f6329` (selection-refresh correction) follows
`4a19208b2c` (unique request IDs). The final complete package remains at
`/workspace/.onboarding/qualification/mvp-followup/package-v2/bin/codex`.
Final source-file hashes still match `package-proof-v2.json`; no source changed
between its successful checks, packaging and physical qualification.

Final-package single/delegated coding and document pilots repeated successfully
in fresh `*-followup-final-v2` projects. Each immutable check and applied original
oracle exited 0; all three explicit applications have Completed receipts.
Request/reported-token totals remain 2/30, 7/105 and 2/30. Evidence: `pilots.json`,
`native-pilots.log`; first-package results are retained with `first-package` names.
The successful physical full workflow is also on this final package.

Final-package online and offline layout checks each passed 185 physical commands
at 120×36 / 50×20, including 160 dashboard switches at 20 ms and ten Console
close/reopen cycles. **370 commands passed on the final source**, additional to
370 on the earlier request-ID-only package. No duplicate-ID or timeout frames;
no model requests were generated by these layout runs. Loaded-tree startup was
1.426s online / 1.422s offline; individual functional observations, not benchmarks.
Evidence: `rapid-final-provider-{online,offline}.json`, logs and PNG/text captures.

Actual CLI/official-Neovim captures were visually inspected and copied to
`/workspace/artifacts/cli-followup/`; six screenshots, their text, source/package
metadata and `cli-followup-screenshots.zip`. These show real terminal cells, not
concept artwork. The native diff displays readonly artifact and original buffers.

Only the owned provider/daemon were stopped after qualification. No provider
listener remains; managed daemon PID record is removed. The stopped PID is a
container zombie with zero execution. Package, canonical state, originals and
logs remain; `final-cleanup.json`. To free the constrained workspace, obsolete
regenerable test executables/duplicate source executables were retired with hash
receipts. The original B6 executable was losslessly gzipped, decompressed/hash
verified, and retained as `mvp-b6-cli/retained-baseline-codex.gz`; its helper files,
source and evidence remain. The final source/installed packages are complete.

The final live gate remains **unrun/blocked**, not passed: OPENAI_API_KEY is present
but its validity could not be tested; observed current network metadata still
excludes api.openai.com. Proxy CONNECT returned HTTP 403 before authentication.
The custom domain addition is confirmed saved in the draft; it is not applied or
published. **Zero paid model calls were made.** Review/save/publish that change,
then verify the supported HTTPS route and available native-tool GPT before pilots.
Do not ask again for the already authorized compatible-model/minimal-spend tests.

For A05 after access works, use an isolated private live-provider home and fresh
projects, retain no-retry/native tool gates, low reasoning, and small bounded
scopes. Verify actual model availability and current official rates before any
paid call; use the conservative $1 total working ceiling. Start with the addition
repair, then one-child delegation and immutable-fact document creation. Require
real immutable checks, original unchanged before explicit acceptance/Apply, and
original-project checks afterward. Record real usage/cost and failed/unrun cases.
The local fixture zero-dollar price quote is not an OpenAI price quote. These
steps reuse the qualified native flow; do not bypass CodeModeOnly admission or
expand Code Mode support in this follow-up.

The full repository/platform suite was not rerun: scope is two TUI corrections.
Existing unchanged backend/state/contract qualification remains applicable;
current affected units, production checks, full package and physical paths passed.

Final documentation verification: 111 local links across 22 development/root
Markdown files resolve; evidence JSON is readable; qualified source hashes are
unchanged; `git diff --check` is clean. `final-documentation-check.json`.

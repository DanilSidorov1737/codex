# Testing procedures and evidence

## Operations batch: discovery, finite forms and goal evidence

2026-10-11 (America/New_York). Tasks 2, 3 and 5 are qualified on `f863b37028`.
Full package `/workspace/.onboarding/qualification/operations-batch/package/`,
CLI SHA256 `ab34a1b88a48541620ddf8a99e2b64be40c9eea0e03498943b63a06acee1b868`.
Logs:
`/workspace/.onboarding/qualification/operations-batch/`.

After `. /workspace/.onboarding/activate.sh`, from `codex-rs`, with
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608`:

| Check | Result | Log |
| --- | --- | --- |
| `cargo test --locked --offline --profile dev-small -j4 -p codex-tui --lib organization` | 59 passed, 0 failed/ignored; actual official Neovim checks included; native IPC execution outside outer sandbox | `ui-tests-final.log` |
| `cargo clippy --locked --offline --profile dev-small -j4 -p codex-cli --bin codex -- -D warnings` | passed | `ui-clippy-final.log` |
| `cargo build --locked --offline --profile dev-small -j4 -p codex-cli --bin codex` | passed | `ui-build-final.log` |
| `cargo fmt --all`, `git diff --check` | passed | `ui-format-final.log` / terminal result |

Meaningful new checks cover Unicode-bounded palette navigation/search/paste,
cancel without dispatch, existing-handler routing, finite interval/accepted
trigger validation, exact source/draft preview fencing, parent/subgoal accepted
evidence and revision-preserving goal edits. Existing official-Neovim edits/save/
undo/diff/dirty refusal, configured plugin and redraw checks also pass. Earlier
compile failure from a missing test-fixture field is retained in `palette-tests.log`;
it was corrected, without disabling assertions. The goal-to-schedules projection
reset and goal view over an open editor were corrected in normal review.

Disk headroom required deleting only inactive rebuildable older Rust artifacts.
Hash/size receipts are `derived-artifact-cleanup.json`,
`derived-library-cleanup.json` and `derived-executable-cleanup-final.json`.
Every historical source/package/state/grant/bundle/log is preserved. No dependency
installation, new paid request, historical replay, host logout or reboot occurred.
The [host runbook](unattended-host-qualification.md) records unrun task 1 gates.

`native/prepare.py` and `physical.py` ran once, using a new private home/project,
the real packaged CLI/daemon and official Neovim 0.12.6's native terminal grid.
`native/physical.json` passes actual palette query/cancel, Console `/layout commands`
to Operations, goal/subgoal view and schedules switching, edited goal revision/
parent retention, stale revision/cycle API refusal, per-step finite interval form,
exact pinned-source accepted trigger form, 36 rapid switches at 120×36, 50×20 and
160×48, and normal CLI exit 0. Both created series retain zero issued runs/usage;
they and the future-bound source grant were cancelled without ever executing.
Owner originals are unchanged. `native/cleanup.json` records the stopped owned
daemon and zero paid requests. No synthetic provider was needed or started.

`native/final-readonly.json` independently verifies all five installed package
asset hashes, SQLite `quick_check=ok`, no FK violations, zero attempts/admitted
requests and cancelled series. Native PNG/text captures are preserved separately
from private JSON/state. A package-help inspection initially omitted the required
`CODEX_REPO_ROOT`; the actual builder used its documented variable and passed.
Never rerun these fixture mutations or reset limits to repeat the walkthrough.

## Autonomy batch final qualification

2026-10-11 UTC. All twelve approved bounded items are delivered. Final application
source `37eeac36a9`; later commits are documentation only. Complete package:
`/workspace/.onboarding/qualification/autonomy-batch/final/package/bin/codex`.
CLI SHA256: `1505455e2a0ea73250d5c4b3eab10b3390c3c762e72b700dba9eb6eb7bf4ca80`.
Rust 1.95.0, Node 24.19.0, pnpm 10.34.5, official Neovim 0.12.6 and official
rust-analyzer 1.95.0. Activate `. /workspace/.onboarding/activate.sh`.

Cargo commands run from `/workspace/codex/codex-rs` with
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608` and
`--locked --offline --profile dev-small -j4`. Evidence logs are in
`/workspace/.onboarding/qualification/`.

| Selected command (common Cargo options above) | Passed | Log |
| --- | --- | --- |
| `cargo test -p codex-protocol -p codex-state --lib` | 373 + 293 | `autonomy-final-libs.log` |
| `cargo test -p codex-protocol --test organization_execution_contract` | 2 | `autonomy-final-wire.log` |
| `cargo test -p codex-app-server-protocol --lib schema_fixtures_tests -- --skip write_schema_fixtures_from_env` | 4 | `autonomy-final-schema-check.log` |
| `cargo test -p codex-tui --lib organization` | 52, including actual official Neovim | `autonomy-final-ui.log` |
| `cargo test -p codex-app-server --lib organization` | 12 | `autonomy-final-runtime.log` |
| `cargo test -p codex-app-server --test all suite::v2::organization` | 8 actual native API cases | `autonomy-final-native-api.log` |
| `cargo test -p codex-cli --bin codex organization_service` | 1 | `autonomy-final-service-tests.log` |
| SDK `python -m pytest tests/test_contract_generation.py tests/test_public_api_signatures.py -q` | 25 | `autonomy-final-sdk-tests.log` |

**770 distinct selected checks; zero failures.** Repetitions and schema writers
are excluded. Complete state suite includes 65 organization cases: concurrency,
queue fairness, paused/exhausted admission, descendant/handoff accounting,
deduplicated finite series, accepted-source pinning/input precedence, project/team
filtering/correction/revocation, health exemptions/unknown effects and full recovery
integrity/path/lock refusals. Neither denied assertions nor integrity checks were
disabled. Native IPC/process/sandbox/Neovim cases use authorized native execution
because the outer cloud sandbox masks protected control paths.

Production `cargo clippy -p codex-cli --bin codex -- -D warnings`, full
`cargo build -p codex-cli --bin codex`, existing full-package builder and
`cargo fmt --all --check` pass: `autonomy-recovery-clippy-v4.log`,
`autonomy-final-build.log`, `autonomy-final-package.log`, `autonomy-final-format.log`.
The stable rustfmt import-option warning does not change the passing exit.
Raw stable/experimental schema writers run explicitly (the intentionally ignored
writer is not an omitted test): `autonomy-final-schema-{stable,experimental}-raw.log`.
JSON/TypeScript/precomputed exports and Python SDK types are regenerated; SDK
generation uses its existing activated venv so ruff is on PATH.
`final/final-asset-verification.json` independently matches all five package files
(CLI, code-mode-host, bwrap, rg and manifest) in both fresh restored installations.

Physical evidence root: `/workspace/.onboarding/qualification/autonomy-batch/`.
These scripts are completed evidence, **not commands to rerun**:

| Receipt | Actual interaction and independent outcome |
| --- | --- |
| `physical-v3.json`, `restart-controls.json` | Whole-run pause/refused paused approval/same-grant and attempt continuation/cancellation, shared limits, accepted prerequisites, immutable native checks, actual Neovim diff and one explicit Apply; 36 switches at three sizes and restart retain usage. |
| `planning-final-readonly.json` | Goal creates only proposed planner work; accepted plan opens editable idle workflow; pane preferences survive restart; actual owner config/matchit/Rust LSP diagnostics/definition/native completion/fix/save/config SHA and normal exit 0. |
| `mixed-models/{receipt,accounting}.json` | Four real OpenAI Responses requests: GPT-5.4 Mini + GPT-5.5, actual checks/acceptance, one Apply. New estimate $0.0198223; historical combined $0.3652553, below conservative $1 ceiling. |
| `final/physical-resume-v2.json` | Five immutable checked/accepted tasks; two chained exact accepted-result follow-ups with newer prerequisite bytes; owner team promotion/correction, stale denial, live outsider membership grant/revocation; two finite recurring invocations/pause/resume/cancel; 25 skipped missed/overlapping intervals; actual silent keepalive stall; 36 switches at 120×36 / 50×20 / 160×48; exit 0. |
| `final/unit-verify.json` | `systemd-analyze verify` exits 0 for the printed unit. Actual foreground command stays independent of the TUI and clients attach. This host has no running user manager: no logout/reboot/automatic systemd restart claim. |
| `final/recovery-resume.json` | Actual owned foreground SIGKILL/restart retains unknown request/grant/attempt and fences replay; live backup refuses; 30-file/2,005,973-byte full bundle restores all canonical tables/reports (except generatedAt), runtime JSONLs and seven workspaces to a fresh home; original workspace paths quarantined; corrupted restore refuses before destination creation. |
| `final/restored-review-finish.json` | Prior driver's actual official-Neovim relocated diff and exact two-file Apply retained; immutable owner-original oracle passes; frontend/backend duplicate refusal; corrected normal exit 0; another full bundle/fresh restore preserves the completed application and refuses replay without another intent/response. |
| `final/cleanup.json` | Owned native daemons/providers are stopped; all homes, grants, usage, artifacts, bundles and failures retained. |

Final deterministic provider accounting: **15 admitted / 195 reported fixture
tokens / zero paid requests**, unchanged across crash/restores/Apply. Corrected
fresh home: 13 admissions, 11 reported responses/165 tokens; one actual stall and
one crash remain unknown/fenced. Separate interrupted setup: two reported
responses/30 tokens. Never conflate fixture labels with live model intelligence
or billing. Real pilot estimates are not invoices/taxes or provider hard caps;
CodeModeOnly Luna remains refused and no additional paid calls were made.

Failures and corrected outcomes are retained. Compile/type/lint/schema issues are
recorded in the historical baseline below. A preparation command's process-group
cleanup terminated its foreground child; the corrected fixture used a dedicated
retained session without resetting the interrupted home. UUID search selected a
related limit row instead of a series; exact label selection corrected the driver.
A resume assertion missed immediate issuance of the final finite occurrence;
only pending work was continued. A report comparison included volatile generatedAt;
all other fields and tables match independently. Review is a pending-action inbox,
so restored accepted tasks were opened in Workspace. A final exit helper sent a
plain `1` while already in Console, submitting unintended text; credentials were
removed and no provider response was recorded. Global F1 then `/quit` passes.
The already-completed Apply was never replayed; only remaining exit/restore checks
continued. Preserve both failed review receipts/screens and the successful finish.

Screenshot evidence: **70 actual PNG/text pairs**, **142 CRC-checked ZIP entries**,
including SHA256 manifest and README:
`/workspace/artifacts/cli-autonomy-batch/cli-autonomy-batch-screenshots.zip`.
Archive SHA256: `e5fa3103f2912593bbc740b8dc353c3354a42b429af8ca78c7d6ede9fbfe513d`.
PNGs are unmodified captures of actual CLI/official-Neovim native grids, with
clearly named failure captures retained. Operations and restored Neovim diff were
visually inspected. Selected images are alongside the archive; no credentials,
private databases, bundle contents or session JSONLs are packaged in it.

Limits: single-owner Linux, existing default-two/max-four active executions,
finite explicit series/scopes and native-compatible models. Series creation is
CLI JSON with TUI visibility/controls; completed means all bounded invocations
issued, not their tasks accepted. Console uses native conversation settings.
Offline full bundles require stopping all writers; config/auth/owner originals
are separate, and historical rollout context is not rewritten into new authority.
Unsaved crash restoration, general plugin compatibility, shared buffers, greater
concurrency, other platforms, distributed workers, external integrations, full
Rust/Bazel suite and actual systemd logout/reboot remain unqualified.

Preserve all historical fixtures and `final/workspace-quarantine.json`; never
restart the quarantined source home, rerun drivers/grants/Apply, reset counters or
redo paid pilots. Use fresh scoped fixtures only for newly authorized checks.

## Review, reports, inventory and native selection qualification

2026-10-11 UTC, expanded-batch items 7–12. Workflow source
`fe5ac65949e43d09de9bbcec8990fe5829a9eb7d`, CLI SHA256
`920cdd42c577e67ff0318d3cecd31816bfab08768de282cf25d964605652c591`.
Final corrected source `338104eb992828811a7ad48b556045a51c33fbcb`, CLI SHA256
`0287fa8ea38ede2352b9e09cfa3d4fd0cc98cd49dd53f6879c5430193e2776d2`,
passed the relevant regression, full production CLI Clippy/build/format and actual
read-only CLI/TUI/Neovim checks in `review-selection/final-physical-v3.json`. Evidence root:
`/workspace/.onboarding/qualification/expanded-batch/`.

Activate `. /workspace/.onboarding/activate.sh`. Cargo commands run from
`/workspace/codex/codex-rs`, with `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608`
and `--locked --offline --profile dev-small -j4`.

| Selected check | Passed | Evidence log |
| --- | --- | --- |
| `cargo test -p codex-protocol -p codex-state --lib` | 373 + 277, plus one separately added workflow-descendant case = 278 state | `matrix-protocol-state.log`, `workflow-report-attribution-corrected.log` |
| `cargo test -p codex-app-server --lib organization` | 12 | `matrix-runtime-inventory.log` |
| `cargo test -p codex-app-server --test all suite::v2::organization` | 8 | `matrix-native-api.log` |
| `cargo test -p codex-tui --lib organization` | 46, including actual official Neovim | `matrix-tui-neovim.log` |
| `cargo test -p codex-app-server-protocol --lib schema_fixtures_tests -- --skip write_schema_fixtures_from_env` | 4 | `matrix-schema-comparisons.log` |
| `cargo test -p codex-protocol --test organization_execution_contract` | 2 | `matrix-wire-contract.log` |
| SDK `pytest tests/test_contract_generation.py tests/test_public_api_signatures.py -q` | 25 | `matrix-resume-sdk-contract.log` |

**748 distinct checks pass**; repetitions and the two explicit ignored schema
writers are not counted. Production `cargo clippy -p codex-cli --bin codex -- -D
warnings`, full CLI build and `cargo fmt --all -- --check` pass in
`matrix-final.json`. Schema writers run separately with experimental mode 0/1;
keep raw fixtures without Prettier rewriting. From `sdk/python`, activate
`. .venv/bin/activate` so its existing Python and ruff are both on PATH, then
`python scripts/update_sdk_artifacts.py generate-types --schema-dir
../../codex-rs/app-server-protocol/schema/json`. Native process/IPC/sandbox and
Neovim checks use authorized native execution; inner permission assertions remain.

Tests cover complete SQL totals versus pagination, canonical change receipts versus
forged instruction text, timeout checks, consistent audit revisions, project scope,
provider immutability/redaction/endpoint changes, legacy unknown metadata, captured
handoff owners, descendant/workflow attribution and shared-root deduplication.
The real WebSocket client refuses mixed report pages; late summary/inventory
successes and errors cannot retarget a modal. Actual Neovim tests check Unicode
character/line/exclusive ranges, dirty buffers, rename/staleness, block/size refusals
and unchanged authority at draft confirmation. Existing exact approvals remain.

Physical evidence in `review-selection/physical.json` and `physical-v2.json` uses
actual official Neovim 0.12.6 to render the native CLI terminal and a second real
editor embedded in Workspace. It physically selected Unicode, cancelled
explain/refactor/test drafts without work, rejected a changed native buffer,
created pinned proposed debug work without authority, then separately confirmed
its bounded grant. The first blocker fixture's empty artifact declaration was
correctly refused. Cleanup preserved/fenced the interrupted coding attempt and
superseded its pending approval: one request / 15 fixture tokens, no effect.
The guarded continuation created a fresh pinned coding task and grant, preserving
the original task/counters. No reauthorization/reset/replay was used.

The continuation physically navigated exact coding/document approvals, a native
blocked checkpoint and requested changes. Canonical report/inventory/check counts
matched owner records; prices used an explicitly synthetic private owner quote.
An unsaved real original buffer refused Apply without an application intent.
Native undo, actual Neovim diff, explicit acceptance/file confirmation and one
Apply passed the immutable original oracle. The document remains paused for
requested changes and its original output is absent. A full canonical report
(except generation time) matched across an actual daemon stop/start. Agent-resource
scrolling, 36 switches at 120×36 / 50×20 / 160×48, no duplicate-ID/timeout frames
and two native `on_exit` codes 0 passed.

`verification-v2.json` independently checks all five installed package assets,
current binary/source, native and separate immutable oracles, artifact hashes,
report own-usage sums, exact provider receipts and SQLite quick/FK integrity.
**6 localhost responses / 90 fixture tokens / 0 paid**, including the interrupted
attempt; **28 PNG/text pairs** before the final correction. State has one custom
agent, five tasks, five grants, four attempts, three actions/approvals, two results,
one checkpoint and one completed application. Owned daemon/provider are stopped
in `cleanup-v2.json`. Preserve this home and every receipt; never replay its
setup/authorization/acceptance/Apply or reset historical budgets.

Visual inspection found checkpoint calls omitted from organization-tool inventory.
The SQL projection now unions recorded checkpoint and ordinary coordination call
identities. The existing native-state regression checks retry deduplication,
separate note/checkpoint calls and rejected-call exclusion. Its correct full filter
`organization::tests::handoff::checkpoint_validation_dedupe_identity_and_blocker_do_not_stop_unrelated_work`
passed once in `inventory-correction-state-exact.log`; a shortened zero-case filter
in the wrapper is not evidence. Corrected package physical read-only verification passes: the supported update
retained the whole canonical report, all five installed hashes match, and the
actual UI shows one recorded checkpoint call linked to its native attempt. The
complete inbox, scoped summary, original Neovim selection and cancellation of an
exact proposed-task draft passed with no additional model request or mutation.
The final frontend's native exit is 0 (three successful recorded exits including
the main workflow). The initial read-only helpers failed to clear search/wait for
inbox rows and then compared a colon against a rendered middle dot; both failed
receipts/screens survive. Corrected waiting/layout switching and exact rendered
punctuation passed, with all canonical records unchanged and services stopped.
`inventory-correction-checks.json` records format/Clippy/build/format-check exits 0;
its zero-case shortened filter is excluded, with the correct regression pass
recorded separately.

Final screenshots: **38 actual PNG/text pairs**, **78 entries** including manifest
and explanatory README, CRC checked. Archive:
`/workspace/artifacts/cli-expanded-batch/reports-selection-screenshots.zip`, SHA256
`ba76215f82dbd9d550984a30c6d632229900d9d55748795dacae7f9a8e0babf9`.
Its final images were visually inspected; failure captures remain clearly labeled.

Initial compilation, stale fixture semantics, mistyped zero-case filters, missing
venv PATH and denied Clippy warnings remain beside corrected passes. No assertion,
permission, checksum, lint or TLS guard was disabled. This selected matrix and
localhost native exercise do not qualify new live-model reasoning/billing,
external integrations, higher concurrency, additional platforms, user Neovim
plugins/LSP, shared buffers or automatic Apply. The PNG renderer's DejaVu font
lacks the CJK glyph used in a native Unicode sample; stored/native text remains
correct, with character/range assertions in the actual-editor tests.

Documentation validation after final updates resolved 131 local links and heading
anchors across the checker's 11 selected documents; `git diff --check` passed.
The updated complete cloud start_skill was saved/read back exactly in draft
revision 14, preserving all other configuration fields. It points to the tested
package, commands, limits and non-replay instructions; saving is distinct from
publication and new-task restoration. No secret values were saved.

## Checkpoint and handoff qualification

2026-10-11 UTC, approved expanded-batch items 2–4, qualified on application source
`dcf53641b85dc299e5b1f0f1318d433be86ec74e`. Automated and physical checks pass.
Logs are in
`/workspace/.onboarding/qualification/expanded-batch/`. Activate
`. /workspace/.onboarding/activate.sh` from `codex-rs`, with
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608` and
`--locked --offline --profile dev-small -j4`.

| Check | Passed | Log |
| --- | --- | --- |
| `cargo test -p codex-protocol -p codex-state --lib` | 373 protocol + 273 state | `handoff-state-protocol-final.log` |
| `cargo test -p codex-state --lib organization::tests::handoff` | 5; one added recurring/workflow case plus four repeats | `handoff-state-accounting-tests.log` |
| `cargo test -p codex-app-server --lib organization` | 11 runtime/workspace/owner-origin | `handoff-runtime-tests.log` |
| `cargo test -p codex-app-server --test all suite::v2::organization` | 8 actual native API/sandbox | `handoff-native-api-tests.log` |
| `cargo test -p codex-tui --lib organization` | 34 TUI/actual official-Neovim | `handoff-tui-tests.log` |
| `cargo test -p codex-app-server-protocol --lib schema_fixtures_tests -- --skip write_schema_fixtures_from_env` | 4 contract comparisons | `handoff-schema-comparisons-current.log` |
| `cargo test -p codex-protocol --test organization_execution_contract` | 2 wire contracts | `handoff-wire-tests-current.log` |
| Python SDK generation/public signatures | 25 | `handoff-python-tests.log` |

**731 distinct automated tests passed**; repeated cases and explicit ignored schema
writers are not added to that total. This is the selected organization qualification
matrix, not the entire repository test suite. Native tests use localhost synthetic
Responses with real tools, filesystem checks and isolation; they do not qualify
additional real model reasoning or incur paid requests. Physical CLI/TUI/editor
qualification used a new `continuity/` fixture and `continuity-home`; canonical
records/receipts now exist, so never rerun its mutation scripts or reset limits.

Coverage includes immutable progress/action provenance, original grant/counter/
expiry/base retention, old-worker denial, busy cleanup, unresolved usage/effects,
exhausted limits, receiver/checkpoint/task revision mismatches, unrelated work,
v8 migration, restart and hash/alias/traversal checks. The native API rejects a
tampered checkpoint, imports pinned inputs after an original edit, stops the
source, then finishes with a receiver under one grant/four total requests and a
real immutable oracle. TUI forms require matching selected details and preserve
captured task/checkpoint/receiver revisions through later selection/refresh.
Recurring cancellation fences the replacement while retaining series counters;
workflow handoff cannot release unaccepted dependencies or pool fixed step grants.

Failures are retained. The first state test referenced nonexistent `settle_action`;
it was corrected to the existing native receipt method `finish_action` without
changing assertions. The first schema run enabled Prettier, unlike the committed
raw-generator format, causing two fixture comparisons to fail; both raw bundles
were regenerated and all four comparisons passed, removing unrelated formatting.
The wire test exposed an older assumption that optional Apply `paths` must always
be serialized; it now verifies required fields, the exact optional omission and
both all-file/selected-file variants. Both wire tests pass. Production library
and full CLI Clippy (`-D warnings`), formatting and the full CLI build pass
(`handoff-{clippy,cli-clippy,format-check,cli-build}.log`). The complete package
uses the current CLI plus already verified helpers; all five installed/source
hashes match (`continuity/preflight.json`). CLI SHA256:
`ded3972c27cfcbb70d3ba35cc8c79e9da4d44d85b9ffd5d18d8a658b4cc20905`.

The actual CLI/TUI pilot physically approved a native partial report, observed its
host checkpoint/blocker, then approved and completed unrelated native work while
the first task stayed blocked. Both frontend and owned daemon were restarted;
checkpoint, task, original grant and limits reloaded unchanged. After an intentional
test-owned original input edit, the owner chose the second permitted arbitrary
agent through `H`, reviewed the exact frozen payload and confirmed once. A fresh
generation imported the hash-verified checkpoint input/partial report, not the
changed original input, and retained all original Apply bases. A separately
approved receiver action passed its real immutable oracle. Official Neovim showed
the actual diff, followed by exact acceptance and one whole-file Apply. Thirty-six
rapid switches at 120×36, 50×20 and 160×48 passed; both normal frontend exits were 0.
Thirteen actual PNG/text pairs are retained.

Independent source/package/file/hash/oracle/accounting/application checks passed,
including SQLite `quick_check=ok` and no foreign-key failures. The transferred task
has one original grant, two attempts, one checkpoint/handoff and one completed
application; source/receiver each report two requests/30 fixture tokens. Total
pilot: **6 localhost Responses / 90 fixture tokens / 0 paid requests**. The unrelated
task remains needsReview. Owned daemon/provider were stopped; both tasks/state are
preserved (`continuity/{physical,verification,cleanup}.json`).

Additional setup/oracle failures remain. The package helper first required
`CODEX_REPO_ROOT`; its default musl manifest then refused the GNU build before
daemon/tasks/grants/inference. The corrected command explicitly supplied
`--target x86_64-unknown-linux-gnu`. `continuity/resume-guard.json` proves no database,
actors, grant payload or requests existed before continuing the unchanged
remaining setup once, without recreating the home or resetting state. The first
read-only verifier compared the stored `OrganizationRecord::Task` envelope directly
to the unwrapped API task; it now checks the record type and complete `data`, and
all independent assertions pass. No application-code change or assertion bypass
was needed for these setup/oracle corrections. Preserve all failed logs/manifests
and corrected receipts.

## Apply recovery qualification

2026-10-11 UTC, approved expanded-batch item 1. Evidence is in
`/workspace/.onboarding/qualification/expanded-batch/`. The server certifies
`application: notStarted` with the exact task/digest only while holding the
application lock, before trying the durable intent transaction. Recorded intents,
transport/timeout failures and commit/write uncertainty do not carry that signal.
The dashboard restores native editor input only on this confirmed rejection or
the matching successful Apply response; unrelated decisions cannot settle Apply.
Rejection/error messages survive periodic snapshots. No new dependency/schema.

From `/workspace/codex/codex-rs`, activate `. /workspace/.onboarding/activate.sh`;
use `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608` and
`--locked --offline --profile dev-small -j4`:

- `cargo test -p codex-tui --lib organization`: **32 passed**, including real
  Neovim rejection/thaw and uncertain-outcome/unrelated-success regressions
  (`apply-tui-tests-current.log`).
- `cargo test -p codex-app-server --test all suite::v2::organization`: **7 passed**,
  including exact pre-intent rejection identities and no safe-retry signal for
  an already recorded application (`apply-api-tests.log`). **39 targeted tests**;
  this does not claim a rerun of N3's entire 712-test matrix.
- `cargo clippy -p codex-app-server -p codex-tui --lib -- -D warnings` and
  `cargo clippy -p codex-cli --bin codex -- -D warnings`: exit 0.
- `cargo fmt --check -p codex-tui -p codex-app-server` and
  `cargo build -p codex-cli --bin codex`: exit 0. Full package builder reused
  the verified native helpers; `apply-package-build.log` and `apply-preflight.json`
  record the exact package and all five source/installed hashes.

Actual packaged CLI and official Neovim 0.12.6 ran one fresh report task through
exact native approval, immutable host check, real diff, owner acceptance, expected
original-conflict denial, unchanged owner bytes, persistent error after refresh,
a physically typed native editor command, and one separately confirmed Apply in
the **same frontend**. Thirty-six rapid switches at 120×36, 50×20 and 160×48 passed;
normal native CLI exit 0. Eleven actual PNG/text pairs are retained. Independent
artifact/oracle/receipt/accounting/SQLite quick/FK checks passed. **2 localhost
Responses / 30 fixture-reported tokens / 0 paid requests**. Private home `eb-home`
contains one accepted task with one Completed report-only application; owned
daemon/provider are stopped and state preserved (`apply-cleanup.json`).

Failures remain: the first new UI regression fixtures closed their command
receivers, causing queue-error text to replace the expected message; the corrected
live-channel fixtures passed without weakening assertions. The physical setup
runner used nonexistent `execution-mutate`; CLI exit 2 left the task proposed
with no grant/attempt/inference. `resume-apply.py` proved that canonical state,
then authorized the same exact payload once using `execute`; no reset or replay.
`apply-runner.json`, the failed logs and corrected receipts are preserved.
This qualifies the stated Linux/clean-editor/local deterministic envelope;
additional paid models, plugins/LSP and other platforms were not exercised.

## Reusable workflow qualification — 2026-10-11 UTC

Qualified application source `2bd752a3d1f09583c90d208e2081b7ee69335661`;
storage `05e04c6116`, owner API/runtime `52211ba6e8`, dashboard `c59f1b38f1`
and original-base carry-forward fix `dc299dfb77`. No dependency/installation.
Ponytail correctness review traced atomic creation, owner-origin ordering,
revision/scope pins, fixed descendant accounting, dependency acceptance and
same-run artifact imports. Review caught recapturing an imported file's base,
which could hide an owner edit between steps; the runtime now carries the
producer's base and meaningful native/physical regression checks pass.
Complexity review retained existing validated plan items, SQLx transactions,
daemon poller, task/grant controls and forms. The bounded 32-step validator uses
a simple deterministic topological scan; no workflow engine, broker or DSL.

### Automated checks

From `/workspace/codex/codex-rs`, `. /workspace/.onboarding/activate.sh`, with
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608` and test/build/Clippy flags
`--locked --offline --profile dev-small -j4`. Logs are in
`/workspace/.onboarding/qualification/workflows-n3/`.

| Command/target | Result | Log |
| --- | --- | --- |
| `cargo test -p codex-protocol -p codex-state --lib` | 372 protocol + 269 state; 0 failed/ignored | `state-protocol-tests.log` |
| `cargo test -p codex-state --lib organization::tests::workflow` | 4 repeated scenarios with added overlapping-producer assertion; 0 failed/ignored | `state-workflow-final.log` |
| `cargo test -p codex-app-server-protocol --lib schema_fixtures_tests -- --skip write_schema_fixtures_from_env` | 4 comparisons | `schema-comparisons.log` |
| `cargo test -p codex-app-server --test all suite::v2::organization` | 7 native API, rerun after base fix | `native-api-tests-current.log` |
| `cargo test -p codex-tui --lib organization` | 30 native TUI/actual official-Neovim, rerun after base fix | `native-tui-tests-current.log` |
| `cargo test -p codex-app-server --lib request_processors::organization` | 1 owner-origin | `owner-origin-test.log` |
| `cargo test -p codex-app-server --lib organization_workspace` | 4 native workspace, including accepted-byte/base regression | `workspace-tests-current.log` |
| `cargo clippy -p codex-protocol -p codex-state -p codex-app-server-protocol -p codex-app-server -p codex-tui --lib -- -D warnings` | Exit 0 | `clippy-final.log` |
| `cargo clippy -p codex-cli --bin codex -- -D warnings` | Exit 0 | `cli-clippy-current.log` |
| `cargo fmt --check -p codex-protocol -p codex-state -p codex-app-server-protocol -p codex-app-server -p codex-tui -p codex-cli` | Exit 0; known stable-toolchain nightly imports warning | `format-check.log` |
| `cargo build -p codex-cli --bin codex` | Complete final CLI, exit 0 | `cli-build-current.log` |

Python from `/workspace/codex/sdk/python`, `. .venv/bin/activate`:
`python -m pytest tests/test_contract_generation.py tests/test_public_api_signatures.py -q`
passed **25** (`python-contract-tests-current.log`). **712 distinct automated tests**;
`automated-checks.json` verifies counts and SHA256s. Repeats and two schema-writing
helpers are excluded from that distinct count. Native tests ran with authorized
native execution because the outer sandbox masks protected IPC/paths; inner
permission/integrity assertions remained enabled. Full Rust/Bazel/platform suites
were not run. No new paid/provider intelligence check was performed.

Four new state scenarios cover invalid/duplicate/cyclic graphs, missing/extra/
immutable/ambiguous scopes, read-only preview with no authority, late-grant
transaction rollback, tampered totals, stale node/template revisions, fresh
identities, duplicate run refusal, fixed request exhaustion, exact acceptance
before dependency admission, filtered same-run metadata, restart/pagination,
reauthorization refusal, migration from v7 and interrupted predecessor blocking.
The v6 migration test now reconstructs a genuine prefix by dropping later tables
and migration rows. Native API checks preserve frozen run content through edits
and restart. TUI checks freeze IDs/scopes/confirmation and ignore late previews
after closing/changing a form; narrow/untrusted controls remain safe. Existing
actual Neovim Unicode editing/save/undo/diff/dirty-buffer/focus/startup tests pass.

Schema generation used explicit `CODEX_APP_SERVER_SCHEMA_ROOT` ending in
`codex-rs/app-server-protocol/schema`, `CODEX_APP_SERVER_SCHEMA_EXPERIMENTAL=0`
then `1`, and the ignored exact schema writer. Both passed; four comparisons passed.
Experimental endpoint wrappers remain filtered from flat stable exports; shared
workflow DTOs are exported. SDK generation used
`python scripts/update_sdk_artifacts.py generate-types --schema-dir ../../codex-rs/app-server-protocol/schema/json`.
Generated JSON/TypeScript/precomputed/schema/SDK files are committed together.

### Complete package and actual interaction

Pinned Rust 1.95.0, official checksum-verified Neovim 0.12.6, Node 24.19.0 and
pnpm 10.34.5 were reused. Package build, from the Rust directory, exit 0:

```sh
CODEX_REPO_ROOT=/workspace/codex python3 /workspace/codex/scripts/build_codex_package.py --target x86_64-unknown-linux-gnu --package-dir /workspace/.onboarding/qualification/workflows-n3/package --entrypoint-bin /workspace/codex/codex-rs/target/dev-small/codex --code-mode-host-bin /workspace/.onboarding/qualification/memory-search-20261010/package/bin/codex-code-mode-host --bwrap-bin /workspace/.onboarding/qualification/memory-search-20261010/package/codex-resources/bwrap --rg-bin /workspace/.onboarding/qualification/memory-search-20261010/package/codex-path/rg --strip none
```

`package-build.log`, `preflight.json` and `verification.json` verify source/clean
application diff and all five source/installed hashes (CLI, Code Mode helper,
bwrap, rg, manifest). Reusing the helper does not qualify CodeModeOnly execution.
CLI SHA256: `2f3980aff324d7c998ce4190133960342c04a228510335327985185760ec5b5a`.
Fresh private home `/workspace/.onboarding/qualification/n3-home`; source project,
local provider and scripts in `workflows-n3/`. OPENAI_API_KEY was removed from
fixture process environments. Historical homes/results were not reset or replayed.

Actual commands below ran from the evidence directory, with activation and native
execution, logging to corresponding `.log` files. These are **single-use evidence,
not instructions to recreate/replay invocations**. Native Neovim terminal channels
physically entered keys; captures render actual external-UI cells/colors. Every
successful physical session has a native `on_exit` code 0 receipt.

| Actual stage | Evidence |
| --- | --- |
| `launch-provider.py`, `setup.py`, `preflight.py`, `coding-start.py` | Actual CLI creates two arbitrary agents/project, saves template with zero calls, previews and atomically authorizes three fresh fixed grants; closed frontend first waits for exact approval; dependents queued |
| `physical.py inspect`, `implement`, `verify` | Exact native approval, actual immutable checks, actual Neovim Follow; owner accepts first two to release next steps; accepted plan/math imported privately; final stays needsReview; original math remains subtraction |
| `lifecycle.py` then `lifecycle-resume.py` | Actual stop/start preserves canonical state/no extra calls; second future-gated invocation has distinct IDs; edits do not retarget first run; duplicate/cycle/stale/reset denied; existing fresh tasks cancelled correctly |
| `physical.py save-ui`, `run-ui` | Physically save prerequisite closure, Unicode template label, explicit common scopes, canonical read-only F6 preview and exact y confirmation; templates/runs visible; hierarchy remains Operations default |
| `physical.py ui-first`, `ui-second` | Real approved document commands and immutable checks; second stays queued until first accepted; imports first report privately; actual Neovim diff |
| `physical.py ui-apply`, then `ui-apply-final` | Owner accepts final result; Apply of the actual carried original base rejects a controlled edit made between steps before an intent; preserves edit; records restoration of this owned fixture; inspect canonical no-intent state and reconnect the conservatively frozen frontend, then new exact confirmation applies report once |
| `physical.py navigation` | 36 switches across 120×36, 50×20 and 160×48; actual narrow scrolling; no duplicate request IDs/timeouts; exit 0 |
| `remote-denial.py` | Real TCP WebSocket denies workflow list/preview/mutation before owner data/authority |
| `verify.py`, `cleanup.py` | Independent immutable checks/hashes, SQLite quick/FK, eight unique task/grant links, fixed counters, canonical states and all package files; stop only owned services, remove PID record, preserve state |

`verification.json` proves three runs/eight tasks: coding accepted/accepted/needsReview
(no Apply), UI accepted/accepted with one Completed `paths=["report.txt"]` receipt,
and the second coding invocation's three future-gated tasks cancelled. Five actual
immutable oracles are independently rerun in preserved workspaces; all pass.
Original coding files stay unchanged; document original changes only after the
explicit final Apply. Exactly **10 localhost Responses / 150 fixture-reported
tokens / zero paid requests**; five executed tasks each consumed two requests/30
reported tokens. The model label gpt-5.5 denotes a deterministic fixture, not
new live-model competence/billing evidence. Failed harness recovery added no
invocation or model request. Services/frontends are stopped; JSON/logs/workspaces
and prior failures remain. Actual PNG/text archive: `/workspace/artifacts/cli-workflows/`.

### Preserved failures and boundaries

- Initial safe-query compilation rejected dynamic SQL formatting; separate literal
  queries replaced it. Initial test/runtime binding, processor field/origin,
  private helper and unnecessary unwrap issues were corrected and rerun.
- State fixtures initially left migration 8 behind when deleting migration 7, and
  omitted artifact base metadata. Tests now reconstruct real migration prefixes
  and supply required metadata; checksums/base guards were not weakened.
- The initial native API negative helper panicked on the expected duplicate error
  because it accepted success only. Explicit error-stream assertions now verify
  -32602; all seven native API tests pass.
- Initial SDK checks selected nonexistent test paths (no tests ran); corrected
  contract/signature targets pass 25. Wrong initial Cargo working directories
  were corrected; all relevant original logs remain.
- Physical Follow initially waited for an absent “NORMAL” string. The actual
  editor showed `Neovim · normal · editor focus` and the checked plan. Failure
  capture/log/receipt remains; corrected physical inspection resumed the existing
  needsReview result, then accepted it without authorization/approval/model replay.
- Lifecycle cancellation initially submitted nonexistent `type:cancel`; the CLI
  correctly rejected it before mutation. Its future-gated invocation and first
  template edit were preserved. Correct `type:stop,cancel:true` cancelled those
  same tasks; restart/denials were checked again with zero extra Responses.

- Final acceptance initially succeeded, but the harness waited for an offscreen
  `Accepted: true` result field instead of the visible `State: accepted`. The
  failed receipt preserves the actual screen; corrected checks resumed the
  accepted result without a second acceptance or additional inference.
- Apply correctly refused the owner edit before an intent/write. A subsequent
  same-frontend retry remained blocked by its conservative `applying`/frozen-editor
  guard after that consequential RPC error. The preserved attempt failed its
  completion expectation with no receipt/write. After inspecting canonical state,
  a fresh actual frontend reviewed and explicitly applied the same accepted result
  once. This reconnect requirement is an existing usability limitation; automatic
  replay/unfreezing was not introduced. The first offscreen-field failure's full
  native screen survives in its receipt/log; the shared failure PNG filename was
  reused by the later attempt, whose PNG/text are separately retained.

Linux x86_64, clean official Neovim and local fixtures are the current qualification.
User configuration/LSP/plugins, new live-provider quality/invoice checks, distributed
workers, recurring whole-workflow triggers and high-volume workflow load are unrun.
Scopes are fixed per step, not pooled cash/token credits. Already-admitted requests
can overshoot token ceilings. Expired/interrupted/failed predecessors stay blocked;
there is no run-wide resume/retry/reset, automatic acceptance/Apply or uncertain
replay. A fresh invocation requires its own owner-reviewed preview. See
[workflow limits](workflows.md) and [safe resumption](status.md).

Startup guidance saved/read back exactly at cloud draft revision 13, with all
other draft fields unchanged (`cloud-draft-readback.json`). The archive contains
38 actual PNG/TXT capture pairs and 80 CRC-checked entries (`archive.json`).
Review/save and publication remain owner actions; fresh-task restore is unverified.

## Recurring schedule qualification — 2026-10-11 UTC

Application source `e06def499ef919e381bb8ec6e0ab7518960f280d`, storage
`54954cfb7d`, API/CLI `d5f2115211`. Ponytail reused daemon/SQLite/grant/approval/
budget and dashboard paths; no dependency or installation. Migration 7 preserves
canonical state. Correctness review checked transactions, scope pins, descendant
root accounting, late usage, occupied cleanup, owner-origin ordering and recovery.
Complexity review replaced a duplicate Create match with one `if let`; no scheduler
service, calendar parser or speculative framework. See [contract](recurring-schedules.md).

### Automated checks and package

From `/workspace/codex/codex-rs`, `. /workspace/.onboarding/activate.sh`, with
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608` and test/Clippy/build flags
`--locked --offline --profile dev-small -j4`:

| Command/target | Completed result | Log in `recurring-schedules-n2/testlogs/` |
| --- | --- | --- |
| `cargo test -p codex-protocol -p codex-state --lib` | 371 protocol + 265 state; 0 failed/ignored | `recurring-state-protocol-current.log` |
| `cargo test -p codex-app-server-protocol --lib schema_fixtures_tests -- --skip write_schema_fixtures_from_env` | 4 comparisons; 0 failed/ignored | `recurring-schema-comparisons.log` |
| `cargo test -p codex-app-server --test all suite::v2::organization -- --nocapture` | 6 native API; 0 failed/ignored | `recurring-native-api.log` |
| `cargo test -p codex-tui --lib organization -- --nocapture` | 28 native TUI/official-Neovim; 0 failed/ignored | `recurring-native-tui-current.log` |
| `cargo test -p codex-app-server --lib request_processors::organization` | 1 owner-origin; 0 failed/ignored | `recurring-owner-origin.log` |
| `cargo clippy -p codex-protocol -p codex-state -p codex-app-server-protocol -p codex-app-server -p codex-tui --lib -- -D warnings` | Exit 0 | `recurring-clippy-current.log` |
| `cargo clippy -p codex-cli --bin codex -- -D warnings` | Exit 0 | `recurring-cli-clippy.log` |
| `cargo fmt --check -p codex-protocol -p codex-state -p codex-app-server-protocol -p codex-app-server -p codex-tui -p codex-cli` | Exit 0 | `recurring-format-check.log` |
| `cargo build -p codex-cli --bin codex` | Complete CLI, exit 0 | `recurring-cli-build.log` |

Python from `/workspace/codex/sdk/python`, `. .venv/bin/activate`:
`python -m pytest tests/test_contract_generation.py tests/test_public_api_signatures.py -q`
passed 25 (`recurring-python-contracts.log`). **700 distinct tests passed**;
`automated-checks.json` independently verifies counts and log SHA256s. Repeats and
the two ignored schema-writing helpers are not double-counted. Native tests used
authorized native execution because the outer sandbox masks protected paths;
inner security assertions remained. Full Rust/Bazel/platform suites were unrun.

Five new state scenarios cover atomic concurrent/deduplicated occurrence creation,
frozen source despite edits, queued/live/approval/native-cleanup overlap skipping,
missed times/backward clock, reopen/backup/restore, exact pause/revoke/cancel,
future-only resume, stale revisions, scheduled-occurrence reauthorization denial,
aggregate descendant/late usage, separate request ceiling, interrupted/uncertain
work, scope/policy changes, migration from version 6 and paginated validation.
The new native API check persists controls through restart and rejects stale or
duplicate authority. TUI checks freeze identity/scope/config/revisions, keep
asynchronous controls from stealing newer selection, and render narrow/untrusted
content. Existing real Neovim edit/save/undo/diff/dirty/focus/startup checks pass.

Contract generation: set `CODEX_APP_SERVER_SCHEMA_ROOT=/workspace/codex/codex-rs/app-server-protocol/schema`
and `CODEX_APP_SERVER_SCHEMA_EXPERIMENTAL=0`, then `1`, sequentially; run
`cargo test -p codex-app-server-protocol --lib schema_fixtures_tests::write_schema_fixtures_from_env -- --exact --ignored`.
Both passed (`recurring-schema-stable.log`, `recurring-schema-experimental.log`).
All four comparisons passed. Experimental owner endpoint wrappers are intentionally
filtered out of flat stable exports; shared schedule DTOs remain exported.
SDK generation, with its venv activated:
`python scripts/update_sdk_artifacts.py generate-types --schema-dir ../../codex-rs/app-server-protocol/schema/json`.

Official checksum-verified Neovim 0.12.6, Rust 1.95.0, Node 24.19.0 and pnpm
10.34.5 were reused. Complete package, from the Rust directory:

```sh
CODEX_REPO_ROOT=/workspace/codex python3 /workspace/codex/scripts/build_codex_package.py --target x86_64-unknown-linux-gnu --package-dir /workspace/.onboarding/qualification/recurring-schedules-n2/package --entrypoint-bin /workspace/codex/codex-rs/target/dev-small/codex --code-mode-host-bin /workspace/.onboarding/qualification/memory-search-20261010/package/bin/codex-code-mode-host --bwrap-bin /workspace/.onboarding/qualification/memory-search-20261010/package/codex-resources/bwrap --rg-bin /workspace/.onboarding/qualification/memory-search-20261010/package/codex-path/rg --strip none
```

Exit 0 (`package-build.log`), unchanged verified helpers. CLI SHA256:
`f6cd6855de7a3fef9d08f8bf4a51282b73e92cb0f2dbc98b2142be5b3a972b1a`.
`preflight.json` confirms clean application diff, source commit and all five
source/installed hashes before physical checks. Stable rustfmt emitted its known
nightly-only imports warning; the format check passed. No install/network/secret
setting changed.

### Actual CLI/TUI/background/editor qualification

Evidence/scripts: `/workspace/.onboarding/qualification/recurring-schedules-n2/`.
Fresh owner-only home: `/workspace/.onboarding/qualification/n2-home`.
Fresh synthetic project, custom agent and proposed prerequisite-free template;
the injected OPENAI_API_KEY was removed from fixture process environments and
historical homes were excluded. Actual native CLI
started/installed the complete daemon. Only the owned localhost provider supplied
Responses, while actual native sandbox commands wrote isolated files and host
immutable oracles checked them. This is local control/accounting evidence rather
than new live-model intelligence, pricing or provider compatibility.

Commands below ran with `. /workspace/.onboarding/activate.sh` from the evidence
directory using native execution, with logs redirected to the matching files.
**They are single-use evidence, not instructions to replay the fixtures.**

| Actual command | Outcome / receipt |
| --- | --- |
| `python3 launch-provider.py`; `python3 setup.py`; `python3 preflight.py` | Owned provider/complete daemon and proposed source only; zero inference before creation; five installed hashes match |
| `python3 physical.py create` | Physically entered 20 fields, F6, paged exact scope/aggregate confirmation, y; first start +150 seconds, interval 60, maxRuns 2; default Operations tree/upcoming panel; exit 0 |
| `python3 lifecycle.py background-first` | Frontend closed; first task waitingApproval; next due time skipped, one response and no report before native approval |
| `python3 physical.py approve-first-pause` | Exact command approval, native oracle exit 0, needsReview; p/y paused the series; exit 0 |
| `python3 lifecycle.py restart-paused` | Waited past a due time while paused; actual stop/start changed PID and preserved the complete schedule view/identities/counters |
| `python3 physical.py resume`; `python3 lifecycle.py background-second` | u/y skipped overdue time, future start only; exit 0 then fresh second task with frontend closed; issuance cap reached |
| `python3 physical.py approve-second-review` | Exact second approval and native oracle passed; later Follow harness assertion failed after layout switch selected the source instead of the occurrence; failed receipt retained |
| `python3 physical.py review-existing` | Explicit Workspace selection of the already-completed occurrence, actual official Neovim Follow/Ctrl-G, 36 rapid switches at 120×36/50×20/160×48, actual PageDown to counters; exit 0; provider count unchanged at 4 |
| `python3 lifecycle.py create-cancel`; `python3 physical.py cancel` | Fresh pending native action, x/y cancelled task/series, revoked grant and superseded approval before dispatch; exit 0 |
| `python3 lifecycle.py create-budget` | One aggregate request allowed despite per-run ceiling 4; actual preauthorized local command completed, follow-up inference denied, failed task and completed series, no extra occurrence |
| `python3 lifecycle.py create-restart` | Graceful stop failed current task/superseded approval; helper incorrectly expected forced-crash reconciliation and failed; one distinct fresh future occurrence issued; owner cancelled it with freshly read revision |
| `python3 crash-recovery.py` | Verified owned daemon PID/identity/start time, actual SIGKILL, new PID, blocked series/needsReconciliation; helper then incorrectly expected pending approval to be superseded immediately; failed log retained |
| `python3 finish-crash-check.py` | Existing crashed state only: inactive runtime/zero lease, exact late approval denied (RPC -32602), no new occurrence/request over another interval; owner cancel finally revoked/superseded; corrected exit 0 |
| `python3 remote-denial.py` | Real TCP WebSocket schedule list/mutate both denied -32600 before returning owner data/authority; own remote server stopped |
| `python3 verify.py`; `python3 cleanup.py` | Independent read-only verification passed, actual owned daemon/provider stopped, PID record removed, state/evidence preserved |

`verification.json` confirms five series/seven unique occurrences, two distinct
primary tasks still needsReview with native and independent immutable oracle exits
0, approved exact actions, artifact hashes, unchanged original facts/oracles/no
original report, cancelled/non-dispatched cases, aggregate request exhaustion,
root/series sums, SQLite quick/FK checks and all five installed hashes. No
acceptance or Apply occurred. **9 localhost Responses / 135 fixture-reported
tokens / zero paid calls** includes the two graceful diagnostic occurrences.
`gpt-5.5` is a fixture catalog label. Cleanup confirms frontend exits 0 for all
five successful physical phases and known stopped/zombie processes; zombies
are not executing. The failed original Follow phase was forcibly closed and
does not have a normal-exit claim.

### Retained failures and limits

Initial state compile fixtures used nonexistent proposal names/fields, then an
incorrect plan-result JSON path; corrected code uses existing ProposePlan.items
and its array response. Initial Clippy identified the large Create variant;
boxing its scope preserved the wire contract. Initial UI test lacked a scope
fixture; corrected 28-test checks pass, and test preferences use a temporary
directory rather than leaving an `unused` file. All original logs remain in
`testlogs/`; no assertion or permission check was disabled.

Physical helper failures above were corrected by using actual layout selection,
distinguishing graceful shutdown from crash, and testing retained pending approval
through the persisted lease/state fence. The first negative helper used the
nonexistent `execution-mutate` subcommand, sent no RPC, and failed; corrected
`organization execute FILE` received the real stale/paused/revoked/expired-attempt
denial. Its failed log is retained. The failed SIGKILL helper asserted ownership
and PID change before failing, but did not persist its old numeric PID; the
corrected recovery receipt explicitly records that evidence limitation. Canonical
state/results were inspected, never reset or recreated to hide a failed check.

21 selected actual PNG/text captures and CRC-checked 47-file archive:
`/workspace/artifacts/cli-recurring-schedules/cli-recurring-schedules-screenshots.zip`.
The failed Follow frame remains privately in the evidence root. Native grid/RGB
capture uses the existing official-Neovim terminal renderer; it does not create a
fake editor. Selected captures were visually inspected. The reusable startup
guide was saved/read back exactly at cloud draft revision 12; all other config
fields matched. `requires_publish=true`: review/save settings and publish the
refreshed snapshot to retain local work; fresh-task restoration is unverified.

Unrun/later: calendars/cron/DST rules, infinite schedules, new paid-provider pilots,
arbitrary host reboot/service supervision, remote/distributed workers, larger
capacity, user Neovim plugins/LSP, automatic original Apply and N3–N8. Starts are
best effort on a running host; already-admitted usage may overshoot token ceilings.
Preserve every historical qualification home/task/package; do not rerun setup,
authorization or crash scripts or start background inference loops automatically.

## Scoped memory search qualification — 2026-10-10 UTC

Application source `d6d049cc49b42f3ed6f6a22c02974f051bfba5ff`, following backend/CLI
`8d33c5e0d8`. Ponytail and correctness review reused durable bounded notes, existing
coordination fences, protected owner RPC, CLI and dashboard input/panel/worker.
No dependency, migration, embeddings or automatic capture was added. Automated
checks and actual qualification passed; current details are in [status](status.md).

### Automated checks

From `/workspace/codex/codex-rs`, `. /workspace/.onboarding/activate.sh`, with
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608` and Cargo test/Clippy/build flags
`--locked --offline --profile dev-small -j4`:

| Command/target | Result | Evidence log |
| --- | --- | --- |
| `cargo test -p codex-protocol -p codex-state --lib` | 370 protocol + 260 state passed, 0 failed/ignored | `state-protocol-tests.log` |
| `cargo test -p codex-app-server-protocol --lib schema_fixtures_tests -- --skip write_schema_fixtures_from_env` | 4 fixture comparisons passed | `schema-comparisons.log` |
| `cargo test -p codex-app-server --test all suite::v2::organization -- --nocapture` | 5 native API tests passed | `native-api-tests-corrected.log` |
| `cargo test -p codex-tui --lib organization -- --nocapture` | 26 tests passed, including actual official Neovim | `native-tui-tests-final.log` |
| `cargo test -p codex-app-server --lib request_processors::organization -- --nocapture` | 1 owner-origin test passed | `owner-origin-unit.log` |
| `cargo clippy -p codex-protocol -p codex-state -p codex-app-server-protocol -p codex-app-server -p codex-tui --lib -- -D warnings` | Exit 0 | `production-clippy-final.log` |
| `cargo clippy -p codex-cli --bin codex -- -D warnings` | Exit 0 | `cli-clippy.log` |
| `cargo fmt -p codex-protocol -p codex-state -p codex-app-server-protocol -p codex-app-server -p codex-tui -p codex-cli -- --check` | Exit 0 | `format-check.log` |
| `cargo build -p codex-cli --bin codex` | Complete CLI, exit 0 | `cli-build.log` |

Python, from `/workspace/codex/sdk/python`, `. .venv/bin/activate`:
`python -m pytest tests/test_contract_generation.py tests/test_public_api_signatures.py -q`
passed 25 checks (`sdk-contract-tests.log`, exit 0). **691 distinct automated tests
passed**; targeted initial/repeated runs and the two schema-writing helpers are
not double-counted. Native API/TUI/process checks use authorized native execution
because the outer sandbox masks protected paths; inner security assertions remain.

Old-note discovery past the 16-note recent window, exact original provenance,
Unicode lowercase matching, literal `%_`/SQL-like text, empty browse/count/cap,
overlong multibyte/control queries, scope isolation, new-task retrieval after
store reopen, identity spoofing/deduplication and paused cached calls are covered.
TUI checks cover captured identities, wrong-query/project and closed-view replies,
query size, queue failure, narrow rendering and real Neovim focus/dirty guards.
The API's real delegated child uses `searchNotes` and produces checked artifacts.

Schema generation uses the existing ignored helper with
`CODEX_APP_SERVER_SCHEMA_ROOT=/workspace/codex/codex-rs/app-server-protocol/schema`
and `CODEX_APP_SERVER_SCHEMA_EXPERIMENTAL=0`, then `1`:
`cargo test -p codex-app-server-protocol --lib schema_fixtures_tests::write_schema_fixtures_from_env -- --exact --ignored`.
Both helpers passed; all four stable/experimental comparisons pass. The new owner
method is experimental; filtered flat stable exports intentionally omit its params
and response, like other organization methods. Shared note DTOs remain exported.
SDK regeneration: `python scripts/update_sdk_artifacts.py generate-types --schema-dir ../../codex-rs/app-server-protocol/schema/json` with the venv activated.

### Complete package and retained failures

Evidence root: `/workspace/.onboarding/qualification/memory-search-20261010/`.
Pinned Rust 1.95.0, Node 24.19.0, pnpm 10.34.5 and checksum-verified official
Neovim 0.12.6 are reused; no installation occurred in this slice.
From `/workspace/codex/codex-rs`, after the full CLI build:

```sh
CODEX_REPO_ROOT=/workspace/codex python3 /workspace/codex/scripts/build_codex_package.py --target x86_64-unknown-linux-gnu --package-dir /workspace/.onboarding/qualification/memory-search-20261010/package --entrypoint-bin /workspace/codex/codex-rs/target/dev-small/codex --code-mode-host-bin /workspace/.onboarding/qualification/selective-apply-20261010/package-v2/bin/codex-code-mode-host --bwrap-bin /workspace/.onboarding/qualification/selective-apply-20261010/package-v2/codex-resources/bwrap --rg-bin /workspace/.onboarding/qualification/selective-apply-20261010/package-v2/codex-path/rg --strip none
```

Exit 0 (`package-build.log`); unchanged verified helpers are reused. CLI SHA256:
`655660afa5a090b0806463f7a7d0b488c93deec549b7ad33c0b13bb776a9ab36`.
Fresh native daemon start installed the complete package. `preflight.json` confirms
all five source/installed hashes and source HEAD before physical checks.
Private home: `/workspace/.onboarding/qualification/mem-home`; both CODEX_HOME
and CODEX_SQLITE_HOME select it. Existing user/historical homes are preserved.

Initial evidence is retained, not counted as passing: TUI's protocol `Result`
alias compile collision (`production-clippy-initial.log`), prohibited unwrap
(`production-clippy-corrected.log`), SDK generation without the required `/json`
schema suffix (`sdk-generation.log`), and the API success-only helper panicking
on the correct expected `-32602` rejection (`native-api-tests.log`). Each was fixed
and the relevant final checks passed without weakening assertions.
Physical writer search/approval/checks passed, but `/quit` plus Enter in one burst
left Console's composer unsubmitted. Split keystrokes exited the CLI; polling a
released Neovim job then returned `-3` instead of its exit status. The harness now
records actual native `on_exit` status. Safe owner-read/exit recovery passed with
exit 0 and no task reauthorization or additional model calls. Initial logs,
`physical-write-initial.json` and `failure-write` capture remain. Final logs:
`physical-write-final.log` and `daemon-restart.log`; actual restart preserved notes.


### Physical memory workflow and independent verification

From `/workspace/codex`, with activation and authorized native execution, the
owned scripts under the evidence root ran in this order: `launch-provider.py`,
`setup.py`, `preflight.py`, `physical.py write`; after preserving its exit-harness
failures, `resume-write.py` completed only owner reads/normal exit; `restart.py`
performed a real daemon stop/start; `physical.py read` executed three fresh tasks;
`resume-read.py` completed only resize/focus/read/normal-exit checks. Do not rerun
these authorization scripts against the existing fixture. Two custom agents and
two projects were created through actual CLI commands; each of four tasks had
one exact native tool approval and a passing real immutable report check.
The writer was accepted; three readers remain needsReview; no Apply or original
project writes occurred. All new tasks have independent grants/source identities.

The writer's shared/private notes retained their exact IDs, author, source task/
attempt, timestamp and body after daemon PID 41696 stopped and PID 42714 started.
Owner CLI retrieval matched the prior result byte-for-byte. New agent-tool searches
returned shared/private matches 1/1 for the same agent/project, 1/0 for another
agent in that project, and 0/0 for the same agent in another project. Fixture
responses consume the actual returned notes and assert provenance before writing
reports; they do not fabricate retrieval. Physical TUI Unicode query, refresh,
empty recent browse, no-match, query cancellation and provenance views passed.

After launching actual official Neovim Follow, Ctrl-G returned host focus; memory
owned `e/o/t/d/n`, remained visible and left the editor unfocused after Esc. Final
36 rapid layout commands and memory queries passed at 120×36, 50×20 and 160×48,
with no duplicate-ID/timeout frames. Initial narrow selection checks incorrectly
required an unwrapped UUID, then included neighboring panels when normalizing;
`physical-read.log`, `physical-read-final.log`, `physical-read-initial.json` and
`failure-read` preserve those failures. Matching the inspector's actual cells
corrected the assertion without application changes. `physical-read-resumed.log`
records the passing recovery, with no new tasks/reauthorizations/model calls.

`remote-denial.py` initialized a real TCP WebSocket client and requested both
project and private `organization/memory/search`; each received `-32600` requiring
a protected local owner connection, with no notes/result. `verify.py` then exited
0 (`verification.log`/`verification.json`), independently confirming exactly two
canonical notes, six persisted reader `searchNotes` calls and their scoped cached
results, correct source links, four exact report bodies/hashes and independently
rerun immutable oracles, four completed actions/approved approvals, unchanged
originals, and all five source/installed package hashes. **16 localhost deterministic
Responses/240 fixture-reported tokens, zero paid calls**. `gpt-5.5` is a fixture
catalog label; earlier real-GPT evidence retains its original source/limits.

Visual inspection showed later metadata/body below the 50×20 viewport, so a final
read-only `scroll.py` follow-up physically verified PageDown shows the timestamp
and full wrapped body, PageUp restores the match count, and native frontend exit
is 0. `physical-memory-scroll.json`/`.log` passed with the provider stopped, no new
inference and the same 16-request ledger. Its owned daemon was started/stopped
through real lifecycle commands. `cleanup.json`/`cleanup-final.log` confirm the
last successful stop, removed actual PID record and known stopped/zombie services;
frontends are closed and canonical state/packages/failure evidence remain.

18 selected actual PNG/text captures, README and canonical/restart/denial/scroll/
cleanup evidence are at `/workspace/artifacts/cli-memory-search/`. The archive
`cli-memory-search-screenshots.zip` contains 42 files and passed CRC verification.
Images come from real native terminal/external-UI cells; no generated mockups or
custom editor. Narrow headers/hints clip and long content requires scrolling.
Controls and bounds are in [memory search](memory-search.md).

The cloud startup guide was saved and read back exactly (draft revision 10,
`cloud-draft-receipt.json`). Installation, network, repositories, secrets and
runtime requirement metadata remained unchanged. `requires_publish=true`:
review/save updated settings and publish the refreshed snapshot for later tasks.
No publication or fresh-task restoration is claimed.

Unrun/deferred: new live-model retrieval quality/provider billing, other operating
systems, user Neovim plugins/LSP, semantic search/pagination, larger note quotas,
automatic capture/promotion/retention, N2 recurring schedules and N3–N8. Ponytail
and correctness review found no remaining actionable complexity/contract issue
in this scope; this is not a whole-repository security audit. Initial failures
are separate from the 691 distinct passing automated checks.

## Completed selective Apply qualification — 2026-10-10 UTC

Application source `9fadf32797275291322fcc536376a58d27683938`, following backend/API
`32edc1cc71` and Review `51f914a851`. Read the source/text plan/Review art and relevant
instructions. Ponytail/correctness review reused the owner RPC, receipt, prepared
writer, fixed Form and actual Neovim; no dependency or migration was added.
See [selective Apply](selective-apply.md) for exact one-application/subset semantics.

### Automated checks

From `/workspace/codex/codex-rs`, `. /workspace/.onboarding/activate.sh`, with
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608` and Cargo flags
`--locked --offline --profile dev-small -j4`:

| Command/target | Result | Evidence log |
| --- | --- | --- |
| `cargo test -p codex-state --lib` | 259 passed, 0 failed/ignored | `state-tests.log` |
| `cargo test -p codex-protocol --lib application_tests` | 1 passed; old JSON omission and exact selection validation | `protocol-tests-final.log` |
| `cargo test -p codex-app-server-protocol --lib schema_fixtures_tests -- --skip write_schema_fixtures_from_env` | 4 passed; stable TS/JSON and both compressed exports | `schema-comparisons.log` |
| `cargo test -p codex-app-server --test all suite::v2::organization -- --nocapture` | 5 passed, 0 failed/ignored | `native-api-tests.log` |
| `cargo test -p codex-tui --lib organization -- --nocapture` | 25 passed, 0 failed/ignored, final refresh regression included | `tui-tests-refresh-fix.log` |
| `cargo clippy -p codex-protocol -p codex-state -p codex-app-server --lib -- -D warnings` | Exit 0 | `backend-clippy.log` |
| `cargo clippy -p codex-tui --lib -- -D warnings` | Exit 0 | `tui-clippy-refresh-fix.log` |
| `cargo fmt -p codex-protocol -p codex-state -p codex-app-server -p codex-tui -- --check` | Exit 0 | `format-final.log` |
| `cargo build -p codex-cli --bin codex` | Exit 0, final complete CLI | `cli-build-refresh-fix.log` |

Native API/TUI checks ran in authorized native execution: the outer sandbox masks
owner IPC/process paths. Existing security assertions were retained. State checks
prove invalid selections have no intent and chosen paths survive pending → uncertain;
selected stale bases/artifacts are denied, unselected human files preserved, legacy
apply-all, exact approvals/crash recovery/delegation and no-replay pass. TUI checks
cover captured task/revision/digest/paths despite later selection, cancel retention,
Unicode/whitespace paths, narrow rendering, verified cycling, actual editor behavior,
dirty-buffer refusal and denial surviving Snapshot without dispatch.

The schema writer explicitly ran twice (one helper test each, exit 0) with
`CODEX_APP_SERVER_SCHEMA_ROOT=/workspace/codex/codex-rs/app-server-protocol/schema`,
`CODEX_APP_SERVER_SCHEMA_EXPERIMENTAL=0`, then `=1`:
`cargo test -p codex-app-server-protocol --lib schema_fixtures_tests::write_schema_fixtures_from_env -- --exact --ignored`.
From `/workspace/codex/sdk/python`, `. .venv/bin/activate`,
`python scripts/update_sdk_artifacts.py generate-types --schema-dir ../../codex-rs/app-server-protocol/schema/json` exited 0;
`python -m pytest tests/test_contract_generation.py tests/test_public_api_signatures.py -q`:
**25 passed**, exit 0 (`sdk-contract-tests.log`). Distinct automated total: **319**;
repeated runs and the explicitly invoked generation helper are not double-counted.

### Package and actual physical workflow

From `/workspace/codex`, the existing package builder exited 0:
`CODEX_REPO_ROOT=/workspace/codex python scripts/build_codex_package.py --target x86_64-unknown-linux-gnu --package-dir /workspace/.onboarding/qualification/selective-apply-20261010/package-v2 --entrypoint-bin /workspace/codex/codex-rs/target/dev-small/codex --code-mode-host-bin /workspace/.onboarding/qualification/tui-creation-art-20261010/package/bin/codex-code-mode-host --bwrap-bin /workspace/.onboarding/qualification/tui-creation-art-20261010/package/codex-resources/bwrap --rg-bin /workspace/.onboarding/qualification/tui-creation-art-20261010/package/codex-path/rg --strip none`.
Unchanged native helpers were reused. Fresh daemon start installs a full package;
existing owned installations use `app-server daemon update --from-cli --yes`.
All five final installed files independently match package-v2. CLI SHA256:
`7da21117d388f56b0cfb4d0c4581a1ed8fbd29ec4cd2f3516ac9eddf3a43f0bb`.

Evidence root `/workspace/.onboarding/qualification/selective-apply-20261010/`;
private home `/workspace/.onboarding/qualification/select-home` (91-byte socket).
Set both CODEX_HOME and CODEX_SQLITE_HOME to that home for lifecycle commands.
`run.py` physically created two arbitrarily named related agents, goal/project/task,
retained canceled drafts, proved zero inference before explicit bounded authorization,
approved two exact native tool actions, used official Neovim v0.12.6 Follow/diff,
physically `/quit`, and reopened after detached completion. Complete isolated
report/notes checks passed, originals stayed unchanged, and exact result was accepted.

After the recorded corrections below, `resume.py` verified the same task was accepted
with **no application intent** and original files unchanged before interacting.
Final package-v2 physically cycled both verified artifacts, opened native notes diff,
made an unsaved Unicode edit, refused selected Apply and kept that denial visible
through three seconds of live refresh. Two Escapes closed confirmation/draft; actual
Neovim undo/save restored the original. Empty selection was rejected, report-only
choice reviewed, canceled/retained and confirmed. Completed receipt records only
`report.txt`; owner `notes.md` stayed byte-identical. Replay was refused. Reopened
report in real Neovim, physically edited/saved Unicode, undid/saved again, then passed
**60 rapid layout commands at 120×36, 50×20 and 160×48**, with zero duplicate-ID/timeout
frames. Final log: `physical-resume-final.log`.

Independent `verify.py <evidence root>` exited 0 before and after an actual daemon
stop/start. It confirms canonical links/accepted result/checks, one completed receipt
with exact selected paths, immutable oracle hashes, selected artifact SHA256,
unchanged owner notes, all five source/installed package hashes and native RGB palette.
Three localhost deterministic Responses/45 fixture-reported tokens, **zero paid calls**;
`gpt-5.5` is a fixture catalog label, not live GPT qualification. Existing real-GPT
evidence remains separate below. The partial original composition intentionally
fails the complete two-file oracle (expected exit 1); the isolated complete check
and independent selected-file check pass. `expected-full-check-after-subset.json`
records this expected failure, which is not counted as passing.

All owned services/frontends stopped (`cleanup.json`): successful native stop,
actual daemon PID record removed and known provider/daemon absent/zombie, no execution.
Do not broadly kill other processes or rerun the accepted/applied task. State, private
scope, originals, scripts and every failure remain. Downloadable archive contains
26 actual native RGB captures and verification/cleanup; earlier creation captures
use `51f914a851`, final selector/editor/layout captures use `9fadf32797`.
`/workspace/artifacts/cli-selective-apply/cli-selective-apply-screenshots.zip`.

### Failures, corrections and limits

- Initial production build rejected accidentally dev-only `anyhow` in the protocol
  helper; corrected to std Result without adding a dependency. Full state/Clippy
  passed. Initial formatter cwd was the repo root; reran correctly in `codex-rs`.
- SDK generation first used system Python, then a venv interpreter without installed
  `ruff` on PATH. Activating the existing venv fixed generation and 25 tests.
- TUI Clippy denied `filter_map_bool_then`; direct filter/map fixed it, followed by
  successful tests/Clippy. Initial failure logs were retained.
- `initial-modal-navigation-failure/`: harness used one Escape for confirmation and
  retained draft. No intent/write occurred. Preserve accepted result, close both
  layers and do not rerun inference. Owned unsaved test buffer was lost when the
  failed frontend closed; clean Neovim's documented unsaved-crash limit remains.
- `refresh-masked-denial-failure/`: Snapshot overwrote the dirty-buffer rejection
  status while confirmation stayed captured. Protection still blocked dispatch.
  `9fadf32797` preserves that status; the actual dirty-editor regression and final
  live-refresh physical check passed. No retry layer or weakened guard was added.
- Package builder refused nonempty initial `package/` output. An unnecessary update
  selected that unchanged initial package; then built fresh `package-v2/`, updated
  and independently matched every installed hash before final interaction. Both
  packages and failure/update logs are retained. No state/task was reset.
- The daemon has no CLI `status` subcommand (safe inspection exit 2 retained);
  cleanup uses its successful stop, known PID state and actual PID-record removal.
- Full-repository/Bazel/other-platform suites, user Neovim plugins, hunks/staged Apply
  and new real-provider runs are unrun. A draft can retain its previous validation
  text after choices change; F6 validates current choices and displays the exact
  valid payload. This cosmetic polish item does not affect selection enforcement.

Final evidence checks: 126 local links across 23 Markdown files resolved; archive
CRC passed; a heuristic credential-pattern scan flagged no intended changed text
files. This is not a security audit. No application/SDK diff remains after the
qualified `9fadf32797` source. Cloud `start_skill` was saved with the tested final
package/lifecycle/resume instructions; other config fields were preserved. The tool
requires publication for a refreshed snapshot; new publication/fresh-task restoration
were not claimed. Receipt: `cloud-config-receipt.json`.

## Completed TUI creation and concept-art qualification — 2026-10-10 UTC

Application source `46d999bf5bf868d4ac7b8eaf2f96c66ac8b51cac`, following owner
forms commit `05a313c3f3`. Root/scoped instructions, all seven supplied images and
the existing owner API/TextArea/editor/palette paths were inspected. Ponytail and
correctness review reused those contracts plus installed Ratatui/diffy/UUIDs;
there is no new form framework, backend/protocol/schema or dependency change.
Frozen confirmations, stale-revision/hash fences, dirty-buffer protection and
bounded artifact reads remain enforced. Project create/link crosses two stores;
partial link failures report the created identity and require inspection before retry.

### Automated and package checks

From `/workspace/codex/codex-rs`, after `. /workspace/.onboarding/activate.sh`:

- `cargo fmt -p codex-tui -- --check`: exit 0.
- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo clippy --locked --offline --profile dev-small -j4 -p codex-tui --lib -- -D warnings`: exit 0.
- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo test --locked --offline --profile dev-small -j4 -p codex-tui --lib organization -- --nocapture`: **23 passed, 0 failed/ignored**, including actual official-Neovim editing/diff/dirty-buffer behavior, request-ID overlap, project/create → returned-ID linking, captured authorization revision, canceled drafts, multiline Unicode, meaningful scope validation and tampered-artifact rejection.
- `CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608 cargo build --locked --offline --profile dev-small -j4 -p codex-cli --bin codex`: exit 0.

Logs: `tui-creation-art-20261010/{clippy,tests,build}.log` under
`/workspace/.onboarding/qualification/`. The existing package builder exited 0:
from `/workspace/codex`, `CODEX_REPO_ROOT=/workspace/codex python scripts/build_codex_package.py --target x86_64-unknown-linux-gnu --package-dir /workspace/.onboarding/qualification/tui-creation-art-20261010/package --entrypoint-bin /workspace/codex/codex-rs/target/dev-small/codex --code-mode-host-bin /workspace/.onboarding/qualification/mvp-followup/package-v2/bin/codex-code-mode-host --bwrap-bin /workspace/.onboarding/qualification/mvp-followup/package-v2/codex-resources/bwrap --rg-bin /workspace/.onboarding/qualification/mvp-followup/package-v2/codex-path/rg --strip none`.
Unchanged native helpers were reused. All five source package files independently
matched the private installed package; CLI SHA256 is
`3c5c1741035d8960897d369656e7840ac7aa5b2bdaeb3eb8ac2272416d4ebb05`.

### Actual physical workflows and screenshots

Two fresh owner homes used the final complete package, protected native owner
socket, official Neovim v0.12.6 and the existing localhost deterministic Responses
fixture. Native checks used authorized unsandboxed execution because the outer
sandbox masks owner IPC/process paths; permission checks were retained.
Set both CODEX_HOME and CODEX_SQLITE_HOME to `ui-art-home` or `ui-rgb-home`
under the qualification root; run the package's `app-server daemon start`/`stop`
with that same home. Fresh startup installs the complete package. Both lifecycle
commands exited 0. Physical input entered the actual CLI running in official
Neovim's native terminal, and Workspace launched its own actual embedded editor.
Owner RPC assertions were read-only; creation/decisions used TUI commands.

Each workflow physically:

1. Created two arbitrarily named related agents with `U`, canceled a captured
   review without creating records, retained the draft, then used F6/`y`.
   Selected/collapsed/expanded the actual hierarchy.
2. Created a goal, project with existing absolute root/owner/goal, and linked task
   with `G`/`N`/`T`. Verified canonical IDs/criteria and **zero inference** while
   proposed and through canceled execution review.
3. Used `s` to declare inputs, immutable oracle, output, argv check, model and
   resource bounds; F6/`y` authorized work. Approved two exact native tool actions
   using `a`/`y`, and saw the first-stage file in official-Neovim Follow.
4. Entered Console and physically `/quit`; the delayed native command completed
   to NeedsReview with the frontend closed. Reopened the CLI with stable definitions.
   Original project remained unchanged; both exact approval receipts persisted.
5. Inspected the verified inline diff and actual Neovim diff. Used `v`/`y` for
   exact acceptance, proved the original unchanged, then `A`/`y` for Completed
   Apply. Ran the independent original oracle (exit 0) and reopened the applied file.
6. Physically edited/saved Unicode in real Neovim, verified disk contents, undid
   and saved again. Resized the real terminal to 120×36, 50×20 and 160×48;
   20 switches per size at 30 ms intervals passed without duplicate-ID/timeout frames.

Each run used **three localhost Responses/45 fixture-reported tokens, zero paid
calls**; combined 120 layout switches. The `gpt-5.5` catalog identity is a fixture
label here. Previous real GPT qualification remains separately recorded below.
The independent `verify.py` read fresh canonical records and confirmed two nodes,
one goal/project/task, immutable oracle hashes, completed actions, accepted artifact
SHA256, completed Apply, usage and all installed helper/manifest hashes.

Evidence roots: `tui-creation-art-20261010/` and `tui-creation-art-rgb-20261010/`.
Reproduction scripts are `provider.py`, `terminal.py`, `run.py`; independent check
is `tui-creation-art-20261010/verify.py <RGB evidence root>`. Preserve all result JSON,
prepared scopes, provider request logs, native PNG/text captures and cleanup records.
Actual 16-image RGB archive: `/workspace/artifacts/cli-ui-update/cli-ui-update-screenshots.zip`.
Both owned daemons/providers and native frontends are stopped; PID records removed.
The container retains stopped daemon zombies with no execution; do not broadly kill
other processes or rerun accepted/applied tasks. State and original failures remain.

### Setup corrections and qualification limits

Initial `daemon update --from-cli --yes` correctly rejected a fresh home with no
selected package; ordinary `daemon start` installed it. The first long private-home
path produced a 110-byte Unix socket and client `AF_UNIX path too long` before any
tasks/inference. Only that owned daemon was stopped; moved its private home intact
to a 91-byte socket path. Its absolute package symlink then pointed at the old home;
changed only that owned symlink to its verified relative release path and restarted.
`initial-socket-path-failure.json` preserves the original failure.

The first entire workflow inherited `NO_COLOR=1` and passed monochrome fallback.
A read-only capture with NO_COLOR removed showed basic color; explicit
`FORCE_COLOR=3` selected the native terminal's supported RGB output. A second **fresh**
workflow then passed and independently verified actual background/panel/blue/green
pixels against the requested palette. Only test terminal environment changed;
production color detection/accessibility were preserved. The native screenshots
were visually reviewed against the concepts; no concept art was edited into evidence.

Visual adaptations are documented in layouts.md: original Console chat/composer,
indented card hierarchy, narrow pane stacking, no unsupported CPU/cost/timeline
widgets. Inline review shows the first UTF-8 artifact up to 64 KiB per side and an
original snapshot at load; Apply independently rechecks the current base. Advanced
role/membership operations remain in the owner CLI. The full repository/other-platform
suite and user Neovim plugins were not rerun for this TUI-only slice. These checks
qualify the changed native UI flow; they do not assert new live model quality,
distributed scale or provider billing results.

Final documentation/artifact verification: **113 local links across 22 files**
resolved, added credential-pattern check and `git diff --check` passed, and the
16-image archive passed ZIP CRC verification. Receipt: RGB evidence root's
`documentation-check.json`. Application source stayed unchanged after qualification.

## Concept-art shell — automated checkpoint, 2026-10-10 UTC

Added responsive dashboard shell, canonical Operations counts/card tree, actual
project task progress, declared-file explorer, native Neovim presentation and a
bounded verified inline diff using existing diffy/Preview. The owner-RPC overlap
regression also exercises real project/create → link with the returned identity.
An inline-diff test verifies exact artifact bytes and rejects altered artifacts;
render checks cover 1×1, 36×12, 50×20, 120×30 and 160×48.

The affected organization/editor suite passed **23/23** on the first visual source.
Production Clippy then rejected fixed RGB constructors under the existing terminal
palette policy and two guarded unwraps. An indexed-color attempt was also rejected. Reused the existing
terminal palette quantization/contrast helpers and pattern matching instead; no lint or assertion was suppressed. Final formatting, warnings-denied production Clippy and **23/23** affected
tests passed after correction. Full package/physical qualification remain
pending at this checkpoint. Logs: tui-creation-art-20261010/{clippy,tests}.log
under the qualification root.

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


## Operations and workflow controls qualification

Source `55af0a0b5b`, official Neovim 0.12.6, Rust 1.95, native Linux package.
39 selected TUI tests, CLI production Clippy, workspace formatting, complete
CLI/package passed. Logs: `expanded-batch/operations-function-key-*`.
Physical `expanded-batch/owner-ui/resume-v6.json` and `verification.json` pass:
27 PNG/text captures, 36 switches at three sizes, two normal exit assertions,
exact group/node/role/membership editing, retire/restore, stale/cycle rejection,
F7/F8 native steps, separate scopes (100/200 tokens and 2/3 requests), restart,
acceptance-fenced dispatch, immutable checks, actual Neovim diff and one Apply.
Independent asset/oracle/result hashes, counters, SQLite quick/FK checks pass.
Four successful localhost synthetic requests / 60 fixture tokens / zero paid.
The initial Console helper generated 60 rejected localhost POSTs separately;
these have no recorded usage or task grants. Failed harness continuations and
F8 regression, stale-package preflight are retained alongside corrected passes.
The first normal exit is established by `stop_normally()` returning before the
v4 NameError; the second exit is recorded directly in the successful continuation.
All owned services stopped (`cleanup-resume-v6.json`). Never replay these scripts.

## Autonomy batch contracts and recovery baseline

Current implementation source: `912fe220c9` plus CLI path-type correction and
regenerated native contracts. Full protocol **373**, full state **293** (including
**65 organization**), wire-contract **2**, schema-comparison **4** and Python
SDK contract/signature **25** checks pass. Both schema writers were run explicitly;
the comparison's ignored writer helper is not a skipped qualification test.
Production CLI Clippy passes with warnings denied (`autonomy-recovery-clippy-v4.log`).
The full new CLI build and native/API/TUI/service qualification are in progress.
Logs are in `/workspace/.onboarding/qualification/autonomy-final-*`.

Preserved failures: dynamic SQL/unimported test Write trait/wrong view field,
a trailing-slash runtime CWD relocation mismatch, production unwrap/expect lints
and CLI AbsolutePathBuf/PathBuf mismatches. Fixes use literal SQL, proper types,
canonical relocation and existing JSON values; no assertion/lint was disabled.
One schema writer used optional Prettier, which reformatted unrelated exports;
raw canonical writers regenerated the expected format, and all four comparisons
pass. Unrelated formatting is absent from the final diff.

Actual earlier native planner/config/LSP receipt `planning-final-readonly.json`
passes saved preferences across restart, accepted planner → editable idle workflow,
official Neovim/rust-analyzer diagnostics/definition/native completion/fix/save,
bundled matchit behavior and normal CLI exit 0. Preserve its initial harness
failures (insert/command timing, invalid completion position, dirty scratch close)
as well as corrected receipts. Its owned daemon is stopped and its future fixture
grant must not be replayed. Mixed GPT-5.4 Mini/GPT-5.5 live pilot receipt is passed:
four requests, new estimated spend $0.0198223, combined historical estimate
$0.3652553; no more paid inference is planned. These are estimates, not invoices.

## Organization presets qualification

Source `a3e0fcca67` adds definition-only snapshots and atomic fresh clones through
existing protected owner mutations. Full state **295**, full protocol **373**,
targeted TUI/actual-editor **60**, schema comparisons **4** and Python contract
checks **25** pass. Production CLI Clippy/build/package pass. Commands use locked
offline `dev-small`/four jobs; SDK generation uses its existing activated venv.
Logs: `/workspace/.onboarding/qualification/operations-batch/preset-*.log`.
Initial dynamic-SQL/model-binding test errors, legacy downgrade fixture errors,
redundant clone lint, missing Ruff PATH and test-only formatting failures are
retained with corrected passing reruns; assertions/security checks are preserved.

Fresh actual CLI/official-Neovim terminal evidence:
`operations-batch/presets/physical.json` and `restart-v3.json`. Physical save and
version-pinned clone forms/exact confirmation create fresh roles/nodes and only
internal memberships. Original edits preserve the snapshot; stale clone refuses,
missing parent rolls back, original task/grant stay separate, zero requests or
attempts execute. Restart preserves revision/mapping; both native CLI exits are 0.
Three viewport sizes/projection transitions, all five private installed assets,
SQLite quick/FK checks and unchanged owner file pass. The future source task is
cancelled and owned daemon stopped. Both failed read-only restart harnesses are
retained: the saved Console layout requires `/layout operations`; the fifth
package asset is its manifest. Corrected driver passes without replaying mutations.

Package: `operations-batch/preset-package/bin/codex`, SHA256
`782d3145a153dd92c21bc0c96f5830380218a7158bbd83d2664f06841d06b45e`.
Nine actual PNG/text pairs archived at
`/workspace/artifacts/cli-organization-presets/cli-organization-presets-screenshots.zip`.
Preserve all fixtures and never rerun setup/physical mutations or reset counters.

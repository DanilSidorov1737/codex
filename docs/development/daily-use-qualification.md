# Daily-use qualification and resumption

Evidence root: `/workspace/.onboarding/qualification/daily-use-batch/`.
Read the [approved forty-task scope](daily-use-batch.md) and current status.

## Initial publication

`git push --set-upstream origin HEAD:refs/heads/work` succeeded before tracked
changes. `git ls-remote --heads origin work` independently returned full baseline
`4f7e0f12eb31e46da36304833941a89d60ac3c74`. `main` was preserved. The scope-recording
local commit is `f980fa5cdf`. No merge/deployment/environment publication occurred.

## Actual TUI diagnosis and terminal driver: tasks1–2

The earlier failed checks did not establish a production connection defect.
The raw-PTY driver lacked terminal-query handling and sent input without complete
startup readiness. The first actual-terminal diagnostic also accepted a loading
screen as ready and sent text+Enter together. The captured final screen shows
the unsent composer text; its native rollout contains only `session_meta`, no
user message, started turn or token report. Loopback GET/DELETE failures were
observed during shutdown and are not attributed as the turn's cause.

The corrected fresh `native-diagnosis-v2` waits for the actual model/loaded screen,
then separates pasted text from the submit key. It uses the verified complete
`3eca8520e0` fork package and actual official Neovim0.12.6 terminal renderer.
Provider configuration reuses `env_key = "OPENAI_API_KEY"` on the official HTTPS
Responses endpoint, with HTTP/stream retries0 and short fixture instructions.
No key is written to configuration/auth files or printed. Exact model:
`gpt-5-mini`; this legacy model is marked deprecated in the current official
docs and is not the planned default for new pilots.

The actual terminal displayed `TUI_LIVE_OK` after a prompt which did not contain
that contiguous string, then typed `/quit` exited the CLI0. Owned daemon stop0;
no auth file created. Captures, intent, source/CLI hash, completed receipt and
native usage readback are retained under `native-diagnosis-v2/`; the first failed
diagnostic and no-turn proof remain intact. This is a genuine paid provider/TUI
smoke, not a coding/delegation qualification or claim of all40 completed tasks.

`scripts/organization_terminal.py` reuses the previous proven actual-Neovim RPC/
terminal-grid approach without hardcoded historical homes or automatic replay.
It waits for visible results, separates paste/submission, refuses capture
overwrite and requires actual CLI exit. A timeout retains the screen and fails;
forced cleanup is never counted as a normal exit.

From repository root with `. /workspace/.onboarding/activate.sh`:
`python3 -W error::ResourceWarning -m unittest discover -s scripts -p test_organization_terminal.py -v`
passes **2 actual-editor tests**, no skipped assertions. They exercise Unicode
input/terminal output/native exit, missing-result failure/capture and live-job
exit timeout. Initial ResourceWarnings exposed unclosed RPC streams; closing
both streams fixes them and the warnings-as-errors rerun passes. Missing `nvim`
fails the test with activation instructions rather than reporting a skip/pass.

Current official model/rate receipts are under `model-docs/`. GPT-5 Mini rates
are0.25 input/0.025 cached/2.00 output USD per million. GPT-5.4 Mini documentation
is fetched directly from its official Markdown page: Responses/streaming/native
functions/apply-patch, none/low/medium/high/xhigh reasoning,400k total context,
272k max input and128k max output; rates0.75/0.075/4.50. Supported catalog/source
and bounded current-model coding/delegation qualification are next.

## Storage headroom: task9 preparation and evidence

No Rust/Cargo selection or package executable was running from the affected
installations. Read-only inventory found341 unique asset inodes and39 duplicate
content groups. A native hardlink operation replaced299 identical immutable
asset paths, preserving exact SHA256/size/mode/owner/group. Original per-path
timestamps/inodes are retained in the receipt; shared inodes now have the
canonical timestamp. Atomic sibling-link replacement preserves each file's
bytes; no database, project original, grant, accounting or evidence file is a
target. Packages remain protected from editing in place.

`immutable-asset-preview.json` and durable per-replacement
`immutable-asset-dedup.json` record before/after metadata, verification and
**18,256,592,896 bytes reclaimed**. Earlier packages and recovery archives remain
byte-identical. Subsequently the generated profile cache was copied from
`/tmp/codex-hierarchy-build/dev-small` to
`/workspace/.onboarding/build-cache/daily-use-dev-small`, with every regular file
hash, metadata and link target verified before switching the logical target
symlink and removing only the original generated cache. No Cargo/rustc selection
or cache executable was running. `generated-cache-relocation.json` retains24,208
entries/9,891,692,076 regular-file bytes and the completed operation. Some compiler
links still target rebuildable volatile `/dev/shm` data. Root has approximately4GiB
free during the current build and `/tmp` approximately11GiB. Inspect before each
selection; run one locked dev-small selection and never prune while compiling.

## Current low-cost model and provider controls: tasks3/7

Fresh `native-model-mini-v3` uses the verified baseline application binary and
the new explicit local model catalog. Actual TUI readiness displays
`GPT-5.4 Mini`, its genuine reply displays `TUI_LIVE_OK`, `/quit` exits0, daemon
stop exits0 and no credential file is created. Native usage is10,966 input/21
output tokens, estimated$0.008319 from the retained official rates. This adds
to the separately recorded legacy diagnostic$0.0026165, not a reset allowance.
The first metadata attempt omitted a required vector and the next driver looked
for a slug rather than the display name; neither submitted a turn. Failed homes,
captures and intents remain intact and stopped. Never replay these scripts.

The catalog test also preserves existing CodeModeOnly admission. New optional
`response_max_output_tokens` is validated, transported and serialized in native
Responses HTTP/WebSocket requests; changed caps prohibit stale response reuse.
An explicit cap is narrowed to the task's existing token ceiling. Existing native
retry settings are reused, not a second retry layer. No default provider behavior
is changed when the field is omitted. See [provider usage](low-cost-provider.md).

Official protoc33.0 was installed to private onboarding tools after verifying the
upstream release asset SHA256
`d99c011b799e9e412064244f0be417e5d76c9b6ace13a2ac735330fa7d57ad8f`.
No system apt configuration or permissions were changed. Native protobuf/schema
generators pass. The initial wrong API structure insertion and private enum import
failed compilation; both are corrected, with failed logs retained. The corrected
locked/offline four-library run passes **641 tests** (API190/config362/provider34/
models55), no failed or ignored checks. Log `provider-model-config-tests-v3.log`.
The core output-limit reuse check now passes1 test; the first core compilation
found an uninitialized guardian request and the next missed the nested test
import. All fixes and failed logs are retained. Production CLI Clippy passes
with warnings denied (`cli-clippy-v3.log`) and the locked build passes4m22s.
Doctor checks pass130 tests with one pre-existing subprocess fixture ignored;
package-builder checks pass28 and actual-terminal checks pass2. Full root format
check passes after formatting the two earlier terminal helper files.

The actual preview package's TUI displays its release and a capped fixture reply.
CLI, managed package and passive daemon probe all match0.0.0-daily.preview1;
doctor correctly reports the exact CLI SHA256. The walkthrough then fails because
the generic secret redactor hides a numeric diagnostic whose label contains
`token`. Renaming the label to `response output limit` fixes the cause without
weakening redaction. Existing build-info supplies compiled commit/target fields;
an unstamped Cargo executable reports `dev`, not a fabricated current Git SHA.
The fresh second preview/native rerun now passes. The first failed home stays stopped.
Its two localhost requests include background title generation; the retained
provider receipt records **30 synthetic tokens**, zero paid inference.
Do not replay that script or presume a plain TUI chat has only one
provider request. Current-model paid smoke usage remains an observed rollout
estimate with an uncertainty allowance for other provider activity.

`native-cap-proof-v2/result.json` records normal TUI exit0, owned daemon stop0,
matching CLI/TUI/managed-package/daemon release0.0.0-daily.preview2, exact doctor
executable SHA256 and visible37-token output limit. The new package embeds the
verified GPT-5.4 Mini catalog. Actual request readback records37 for the plain TUI
and its title request; a captured30-token organization scope narrows both native
task requests to30. The real native command creates an isolated artifact and its
immutable Python oracle passes; owner originals remain unchanged, with no
acceptance or Apply. Four localhost requests/60 synthetic tokens/zero paid calls.
All services are stopped; preserve the fresh home/result/grant/receipts and do
not rerun the script or its owner mutations. API client integration checks pass9
tests, including legacy omission behavior. Truecolor PNGs retain actual Neovim
terminal cells/attributes with the existing installed font; they are not an
editor imitation. Concept-art layout comparison remains task34.

After the first relocation, an idle-only second move preserves34 generated
compiler files/6,133,637,182 bytes with exact hashes, modes, ownership and timestamps
before replacing their generated cache paths by symlinks into `/tmp`.
`generated-large-cache-relocation.json` records every intent and verification.
No authoritative file or installed package is a target. Root now has approximately
5GiB free during tests; generated `/tmp` copies remain volatile/rebuildable.

## Next work

Source checkpoints: model17235eb152, output cap74896653f0, package identity
cf57bad0a1 and lockfile/native-color captures0752f85977; GitHub `work` is updated
through0752f85977. The corrected TUI test import passes106 selected checks,
including actual Neovim (`tui-organization-tests-v2.log`). The idle-only final
generated-cache move verifies33 files/3,173,697,144 bytes into `/dev/shm`, with
receipt `generated-shm-cache-relocation.json`. These compiler files are volatile;
packages, state and receipts remain persistent and untouched.

Task8 [cost admission](cost-admission.md) now has uncommitted typed/state/provider/
TUI source,115 selected passing state checks and106 selected cost TUI checks;
schema/SDK, production and actual priced native checks are unfinished. New cost
behavior is absent from preview2; do not qualify it using that older binary.

Stable/experimental native schema generation and all4 fixture comparisons now
pass. Python SDK generation and its5 contract checks pass. The updated state
library passes343 checks, followed by one added passing queued-restore/resume
regression (344 distinct tests). Full root formatting passes.

The task14 offline rehearsal retains both failed attempts: the first inspection
assumed an `id` column instead of `attempt_id` in results; it started no backup.
`native-recovery-v2` then backs up/restores successfully and preserves counts,
grants, usage and results, but fails its safety-pause assertion. Its original
snapshot had no global control and the existing restorer did not create a pause.
Neither daemon was started. Keep that restored home stopped; it is not qualified
safe recovery. Separate nonsecret configuration/project copies and bundle remain.

The new restorer holds the execution lock before publishing the database and
sets a captured global pause while preserving existing lifetime limits. It does
not reset grants, usage, decisions or results. Tests cover absent control, ready
grant refusal until explicit owner resume, preserved limits/source control and
concurrent destination refusal. Production lint/new package and a fresh actual
recovery rerun are pending. Preview2 predates this fix.
Current production lint passes (`cost-binding-clippy.log`,6m01s); full formatting,
the regenerated safe-binding schemas4 and Python contracts5 pass. The approved
forty-task work is still unfinished. [Coverage](fork-text-coverage.md) maps all38
original text sections to bounded evidence and explicit remaining capabilities.

An independent official-editor check passes (`native-editor-swap/result.json`):
real Neovim0.12.6 loads its bundled matchit plugin, physically receives an unsaved
line, writes a private native swap, is killed by its recorded owned PID and
recovers identical dirty lines using native `:recover`. The original file remains
byte-identical throughout. Actual before/after RGB captures and private swap
evidence are retained; no paid inference. This qualifies selected editor mechanics,
not CLI attachment or arbitrary plugin/LSP compatibility. The product defaults
to noswapfile; an owner-selected private-swap configuration is required for this
case. Tasks36–37 application walkthroughs remain pending.

Clean application source53e37b68ab now builds successfully in6m24s with its
actual full Git revision stamped through existing build-info. The complete
five-asset candidate is `package-cost-preview3`, release0.0.0-daily.cost.preview3,
CLI SHA256 `4116af89095e281faf4ca332a21478ddca7dee8c91374e973fd18db55ea556a3`.
`cost-build-intent.json` pins clean source and argv; `cost-package-proof.json`
records all assets. Native priced proof passes rejected USD/endpoint bounds,
captured quote/30-token scope/native immutable checks and actual TUI/CLI/daemon
identity/normal exit. Four localhost requests/60 synthetic tokens/zero paid calls;
all services stopped. Never replay its grants or script.

Corrected native full backup/fresh restore also passes on that package. The
first final inspector misunderstood the tagged control wrapper; its failure is
retained and only read-only verification continued. No backup/restore was repeated.
`native-recovery-v3/read-only-continuation-result.json` verifies restored pause,
identical grants/quotes/usage/results/counts and exact artifact hashes; config and
owner oracle are retained separately, credential values absent. Source and restore
stay stopped; neither restored daemon nor model was started. Preserve all three
failed/successful recovery directories. Upgrade/rollback and actual restored
editor walkthroughs remain.

## Real provider coding and retained delegation failure: tasks5–6

`live-code-pilot/` uses the actual clean53e37b68ab package and official HTTPS
GPT-5.4 Mini, retries0 and a1200-token response cap. A captured30,000-token /
3-request /240-second grant admits a conservative16-cent quote, including5,000
unknown tokens. The real model corrects addition in `calculator.py`, an owner
physically approves its action through the TUI, and the native immutable Python
oracle passes. Actual official-Neovim diff review, exact owner acceptance and
exactly one Apply produce a passing original-project oracle. `read-only-finish-
result.json` verifies normal CLI exit0, daemon stop0 and one completed Application.
There are3 admitted/reported requests and6,083 tokens. Deduplicated native usage
is5,871 input (3,072 cached) /212 output: standard estimate$0.00328365; uncached
estimate$0.00535725. These estimates are not provider invoice receipts.

Two driver failures remain: F1 was sent while the actual embedded editor owned
focus, opening Neovim help; later the applied equal-file diff folded the code.
The continuations reused the exact checked result, returned focus with Ctrl-G,
opened native folds with `zR`, and never repeated inference or the completed Apply.
Never rerun its original, review or acceptance/Apply scripts. All services stopped.

`live-content-pilots/` fails its finite delegated run before starting the document
pilot. A coordinator correctly proposes, delegates and waits; the child wastes
calls guessing note syntax and trying to propose another plan, then exhausts the
shared12-request ceiling. Native usage is28,550 tokens,27,490 input (19,968 cached)
and1,060 output, estimated$0.0119091 ($0.0253875 uncached). There is no root result,
acceptance or Apply. Owner originals and immutable oracles remain unchanged;
daemon stop0. The failed grant, child, rollouts and usage proof remain charged.
Do not restart this home, reset its limits or replay its intents.

Diagnosis finds two production guidance problems: the generic5000-byte schema
compactor discards the host-owned coordination DTO's nested plan fields/enums,
and the runtime prompt asks even direct-edit workers to plan delegation. The
small fix reuses the existing lossless trusted-schema parser, supplies literal
string note-search syntax and makes context/delegation conditional on the task.
The new regression checks the actual exposed plan requirements and note enum;
its initial test assumed draft2020 `$defs` instead of the generator's draft7
`definitions`, a retained test failure corrected by supporting both native forms.
Current-package production qualification and a fresh bounded pilot are pending.

The correction811b456376 passes18 runtime/workspace checks and production CLI
Clippy in53.94s; the clean stamped build passes85s. `delegation-package-proof.json`
pins all five assets of `package-delegation-preview4`, release0.0.0-daily.delegation.
preview4, CLI SHA256 `ab67244500b8e2f50bcb60b76f1785d582697561bcfa584b565c5ccf42847210`.
Normal GitHub push/readback verifies work811b456376 and unchanged main8026444c98cd.

Fresh `live-content-pilots-v2` passes on that package: coordinator propose/delegate/
wait, child native multiplication oracle/internal checked acceptance, exact quote
inheritance and root artifact import/check all pass. Root7 requests/18,038 tokens;
document3 requests/7,256 tokens. Both root results remain needsReview, with no
root acceptance/Apply and byte-identical owner files/oracles. Ten deduplicated
native Responses usage records total24,692 input (15,360 cached) /602 output,
standard estimate$0.01086, uncached$0.021228. No invalid coordination arguments.
Daemon stop0; no TUI was attached during these jobs, qualifying background runs.
The failed first pilot remains charged and is not restarted. The original$1
envelope and additional unknown reserve remain; no further paid loop is planned.

## Package lifecycle and actual editor: tasks11–12/14/36

`native-lifecycle/continuation-v4-result.json` passes stopped managed package
selection via existing `daemon update --from-cli --yes`, then actual preview4
CLI/TUI/daemon attachment. Exact proposed task/agent/role/project/mission/pause
definitions remain. Fresh rollback uses the verified pre-upgrade full bundle,
preview3 and separately retained nonsecret config; restored global pause and
unchanged owner original pass. Two normal TUI exits0; all services stopped,
zero inference. This release pair has the same state format; future backward
compatibility is not presumed. Malformed fixture control discriminator/limits,
unsupported status/passive-version expectations and a continuation indentation
failure remain. Successful definition/control/backup/update intents were not
replayed, and no paused grant was resumed.

`native-editor-lsp/finish-result.json` passes actual CLI project explorer and
owner-selected Neovim config from the documented example. Official editor0.12.6
loads bundled matchit, attaches actual installed rust-analyzer1.95 and returns a
native definition link from the multiplication call to its line0 declaration.
Native diagnostics empty; line numbering and omnifunc enabled. Owner Rust file
unchanged, normal CLI exit0/daemon stop0, zero inference. Failed drivers used an
unbound Unicode palette key, unbracketed long editor input, Console-only readiness
on a remembered layout, and a dict expectation for Neovim's numeric-client array.
Physical bracketed paste and the existing command palette fix those driver
assumptions. The actual successful definition receipt was inspected rather than
replayed. No general plugin compatibility or mocked LSP is claimed.

Idle immutable-package dedup preserves SHA256/mode/ownership and retains previous
metadata in durable receipts: paid-pilot dedup2,632,187,088 bytes/6 paths and
lifecycle dedup3,650,854,424 bytes/56 paths reclaimed. No state/original/log/recovery
asset is deleted. All shared immutable package assets must remain unedited in
place. Generated cache/compiler work does not run during these operations.

Continue Owner source tests/package/actual checks, embedded-editor dirty recovery
and remaining independent tasks15–40. Task13 stays blocked on an actual suitable
host. Preserve every diagnostic/paid/lifecycle/editor fixture, original$1
envelope, unknown reserve and historical quarantine.

`native-editor-crash/exact-recovery-result.json` now passes actual embedded-editor
dirty recovery. Native private per-buffer swap records the original plus an
unsaved line; the captured owned Neovim PID is killed with SIGKILL under its
verified CLI parent. The precise preserved swap/hash recovers identical modified
lines into a fresh actual CLI/Neovim buffer. Owner original stays byte-identical;
recovery CLI exit0 and daemon stop0. No model calls, new edit or kill during the
recovery continuation. `swapname(0)` initially inspected the alternate buffer;
using the actual buffer ID fixes the driver. Another driver incorrectly expanded
percent signs in Neovim's swap filename; `nvim_cmd` literal-file semantics recover
the exact captured swap. Earlier unexpected pipe-exit1 was not counted as SIGKILL.
All failures, source private swaps and the verified preserved copy remain.
Default product noswapfile is unchanged; this selected owner configuration and
explicit per-buffer opt-in qualify this recovery case, not automatic recovery
of every editor session/plugin combination.

Owner source now passes108 selected TUI tests (`owner-overview-tests-v6.log`).
The first build hits the32GiB memory limit (one recorded OOM kill); the next reaches
linking but runs out of root disk. Idle generated-only pruning records5,389,914,112
volatile bytes and5,249,748,808 root bytes reclaimed, excluding current test link
inputs and preserving every package/state/original/evidence. A77,459,456 allocated-
byte incomplete test-link temporary is removed with a hashed receipt. One-job
rebuild exposes the new palette's ordering regression and a report-modal test
assumption; preserving layout ordering and closing the read-only view fix them.
A private-field test compile fails and a wrong public-title run is interrupted;
all logs remain. Current production/native Owner qualification is pending.

Approved task16 native `--worktree` creates one managed detached checkout through
the actual fork at `native-self-development/worktrees/09ba/codex`, base811b456376.
The first driver stops at the actual folder trust dialog; continuation reuses that
worktree rather than creating another. Fresh native inspection/quit then passes
(`worktree-finish-result.json`): actual CLI exit0/daemon stop0, unchanged development
checkout's dirty file hashes and HEAD, no usage records/model turns, and automatic
cleanup disabled. Late rollout flushing is verified after daemon stop. The prior
disk-full and recaptured-dirty-baseline driver failures remain. Keep this worktree
and home for the bounded self-development pilot; no proposed change/grant/Apply
exists there yet. This product qualification is within approved tasks16/24;
the cloud development checkout itself remains `/workspace/codex`.

## Owner package and supervised self-development checkpoint

Owner application c4f000e538 is committed/pushed to GitHub work; main remains
8026444c98cd. Clean stamped one-job build501s, production Clippy506s,108 selected
TUI checks and full formatting pass. Five-asset `package-owner-preview5`, release
0.0.0-daily.owner.preview5, CLI SHA256
`a3c310529a5771d4361094a92be0e1d375c5ac5ed2bb6fc9439555a37652b1c3`.
`native-owner-overview/continuation-result.json` passes actual mission/outcome
brief, exact arbitrary-agent contact, cancelled existing conversation draft,
restart/persist/clear and two normal CLI exits0, stop0, no canonical authority
change or inference. First driver selected a group rather than its child agent;
the corrected physical Down navigation succeeds without mutating definitions.

The actual managed fork worktree has a captured branch/base/dirty/path/check
preview. Its full31-check baseline isolates a real defect: valid single-quoted
TOML workspace version is rejected by the manual regex reader. All28 existing
package checks remain; the three owner regressions use actual files. Declared
input rehearsal initially lacks the WinGet helper; adding that immutable helper
retains every required cross-component check. Python3.11+/C compiler/strip and
all20 input files are captured. This is private owner setup, not a model grant.

Two separately bounded paid grants fail and remain charged/stopped. First4
responses/18,875 tokens estimate$0.0095505 ($0.0164625 uncached); it exhausts its
context allowance during unnecessary reads, before editing. The finite corrected
grant receives the complete source,30,000 tokens/4 requests and a15-cent quote;
4 responses/18,651 tokens estimate$0.01074075 ($0.01938075 uncached). It produces
the standard-library TOML fix and regression tests, but their package import is
wrong; the request ceiling stops repair. No scope/counters are reset, no backend
result is forged and neither failed paid task is accepted or Applied. No further
paid loop runs. The lack of autonomous completion is explicitly retained.

The owner corrects only the new test's import/sys.path in a separate preserved
proposal. All34 real-file/package checks pass; original worktree remains unchanged.
`verification-result.json` then verifies those exact owner-provided bytes through
the actual fork daemon/sandbox and immutable oracle. Transport is a localhost
deterministic fixture, NOT a successful autonomous paid-model pilot. Two responses /
30 synthetic tokens/zero paid; exact physical staging-action approval, recorded
34 native checks, original artifact bases/hashes and normal CLI exit0/stop0.

`review-continuation-result.json` passes actual official-Neovim parser diff, exact
acceptance and exactly one two-file Apply into the managed worktree. All34 checks
pass on the applied bytes. `local-commit-result.json` records physically entered
Git commands in the actual owner Neovim terminal, clean-index/exact-hash preflight
and local commit325b20ae88fb, parent811b456376, containing exactly version.py and
test_version.py. Unrelated proposal/oracle entries remain uncommitted; original
development checkout unchanged. Normal CLI exit0/daemon stop0, no inference or
Application replay. This is a qualified human-assisted self-development workflow;
dedicated commit controls, final-byte recheck and bounded repair features remain.
Never replay any script/grant/acceptance/Apply/commit. Preserve all failed grants,
private proposal, exact one Application and detached commit. The reviewed commit is cherry-picked into the development checkout as77d9d637fc;
all31 package tests pass (`integrated-self-development-tests.log`). The post-Apply
full offline bundle is verified and retained separately from configuration, owner
originals and private frontend preferences. Reusable repository-preview source has
a passing native Git regression; actual packaged CLI qualification remains.

## Repository preview source checkpoint

Owner-only local `organization repository-preview` reuses native Git, typed scope
JSON and the existing no-follow relative-file reader. It returns branch/base/dirty
state, requested paths/checks and declared output-base SHA256; no daemon/model/
task/grant or index mutation. One native Git regression passes; the final rerun
adds byte-identical index/original assertions, symlink refusal and significant
repository-path whitespace preservation. Read [usage and limits](repository-preview.md).
Post-self-development package dedup preserves exact hashes/mode/ownership and
reclaims2,246,262,784 allocated bytes across nine identical immutable files.
No authoritative state, original, recovery asset or evidence is removed.

Final source checks pass: native Git regression1 (`repository-preview-tests-v2.log`,
34.73s), production CLI Clippy with denied warnings (`repository-preview-clippy-
v2.log`,12.92s) and full root formatting (`repository-preview-full-format-v2.log`).
The first Clippy hits root disk exhaustion; verified idle generated-file relocation
preserves SHA256 and nanosecond mtime:219,276,264 bytes plus3,249,907,441 bytes,
with durable receipts. The original source file is removed only after its exact
copy is verified when the full disk prevents allocating a replacement symlink.
No live compiler or authoritative data is affected. The first full formatter
finds two Python line-wrap differences;3d63e32264 formats the reviewed TOML reader.
One package-test rerun omits required CODEX_REPO_ROOT and fails before testing;
corrected documented environment runs all31 checks (`integrated-self-development-
tests-v3.log`). These failures remain; no assertion is disabled.

Clean be6cef7014 build passes27.56s and packages as0.0.0-daily.repo.preview6
(`repository-preview-package-proof.json`), CLI SHA256
`d4a457b911bd05d09a91e383eeb27bb23569152734c774b8ec01c6865488f0ba`.
`repository-preview-native/result.json` passes actual official-Neovim terminal
CLI exit0, significant repository whitespace, dirty file/index preservation,
detached and unborn states, and symlink/traversal/.git refusal. No daemon or
model starts and no task/grant is created. Independent direct CLI reads confirm
the rendered metadata. Owner fixture unrelated body remains unread. This qualifies
task17 for the documented bounded Git/file preview, not final40 completion.
GitHub work is independently verified atbe6cef7014; main remains8026444c98cd.

## Original-grant repair source qualification in progress

`repairResult` pins an unaccepted task revision/result digest and captures the
actual declared files and immutable check inputs. Its state transaction reuses
existing same-grant handoff fences and counters, with original bases and expiry
retained. Failed fences roll back the checkpoint/queue change. The private runtime
helper and TUI exact-confirmation form are under qualification; no paid repair
loop is added. Read [the repair contract](result-repair.md).

`result-repair-state-tests-v2.log` passes one multi-case regression: original
grant/usage/bases/history preserved; active-runtime, stale revision/digest, changed
artifact, out-of-scope input, expired/exhausted grants and replay refused. The
initial test-file append uses the wrong working directory and its selection runs
zero tests; that run does not validate repair. The corrected append/rerun passes.
The first combined Cargo command repeats --lib and is rejected before testing;
the corrected command rebuilds affected feature variants. Before linking, the
owned build is deliberately interrupted for low disk headroom; its log and
exact process-group receipt are retained. Idle SHA/mtime-preserving relocation
moves1,999,924,543 generated bytes; eleven inactive old generated executables
reclaim4,353,650,688 volatile bytes. Packages/state/originals/evidence stay preserved.
The current one-job continuation is running; no native final repair claim yet.

Updated reusable start_skill is saved as a nonblocking cloud draft, with verified
tool versions, one-job resource limits, the qualified preview6 identity, required
package-test environment, actual-terminal rules and every protected-state boundary.
Installation/network/repository/credential fields are preserved. Draft saving
does not apply or publish the refreshed snapshot.

The corrected runtime/UI continuation now passes (`result-repair-runtime-ui-
tests-v3.log`,7m40s):19 app-server workspace/runtime checks, two selected CLI
checks and109 TUI/form/actual official-Neovim checks. CLI library selection has
zero matching tests and is not counted. The full state suite passes345
(`result-repair-full-state.log`,27.64s execution), including the new multi-case
repair regression. Existing checks remain intact. Schema/SDK, production lint,
clean package and finite native failing-result/repair/Neovim walkthrough remain.
The preview adds labeled output-folder/file-type check advice while preserving
every captured argv; it performs no test discovery or dependency analysis.

Protocol selection passes10 wire/schema/hunk tests (`result-repair-protocol.log`).
The stable-schema producer needs another dependency feature variant; its first
run is stopped before full-disk linking, with owned process-group intent retained.
Idle pruning removes604 obsolete pre-preview-build generated library variants,
keeping two newest per crate:882,876,416 root and7,840,604,160 volatile allocated
bytes reclaimed. Every package/state/original/recovery/evidence remains. Further
verified SHA/mtime-preserving relocation moves1,999,912,099 bytes across93 generated
files. Schema production/validation resumes with one job and preserved failed logs.

The first schema comparison run fails two of four checks because the optional
Prettier flag reformats unrelated generated TypeScript. Regenerating stable and
experimental fixtures with the actual compiled schema producer and no formatter
restores the existing raw style. All four exact comparisons now pass
(`result-repair-schema-comparisons-v2.log`); the diff includes only the repair
contract and its two precomputed export bundles, not hundreds of formatting edits.
The first SDK generation invokes its interpreter without activating the venv and
cannot find the installed ruff executable. Activating the existing SDK environment
fixes PATH; generation and all25 contract/signature checks pass
(`result-repair-sdk-generate-v2.log`, `result-repair-sdk-contracts-v2.log`).
No dependency or assertion is changed. Production lint/format/package/native
repair qualification remain in progress.

Full root formatting now passes (`result-repair-full-format.log`). Correctness
review confirms the API remains owner-only; the repair payload accepts no new
scope or budget fields, host snapshot validation precedes the transaction, and
existing handoff fences enforce stopped runtime, reconciled usage/effects, current
authority and remaining original limits. Ponytail review finds no new dependency,
framework or replacement scheduler: existing checkpoint readers and same-grant
handoff transactions are reused. Native package qualification remains pending.

Production CLI Clippy passes with warnings denied in8m42s
(`result-repair-production-clippy.log`). Current source checkpoint has514 distinct
selected checks: state345, protocol10, app-server19, CLI2, TUI109, schema4 and
SDK25. Generator-only/zero-test runs and repeats are excluded. The native repair
package/workflow is the next gate; this is not final40 completion.

## Native repair and check-advice qualification

Clean application source66013d5a2a is committed/pushed to GitHub work; main remains
8026444c98cd. The production build is paused before disk-full linking, then resumes
without authority replay after verified idle relocation of2,799,999,999 generated
bytes/173 files. Continuation builds in6m08s (`result-repair-build-v2.log`).
The brief exec transport disconnect does not interrupt the owned compiler; current
process/receipts are inspected before continuation. No duplicate build is started.

Package `package-repair-preview7`, version0.0.0-daily.repair.preview7, has verified
five-asset proof; CLI SHA256
`4fb50c09e7001702591fbce54b85ee2694609904c34f7c12098deb0288acf905`.
`result-repair-native/result.json` passes the actual packaged CLI/TUI/daemon and
official-Neovim workflow. A first local-fixture candidate fails the protected
Python multiplication oracle. Physical tool approval and F6/y exact result-repair
confirmation queue a second checked attempt under the same grant, original
artifact bases and expiry. The second immutable oracle passes; original checks,
request/token counters and scope remain. Another repair is refused with the
original allowance exhausted and no state change. The actual native editor shows
the corrected diff; normal CLI exit0/daemon stop0. Four localhost responses/60
synthetic tokens/zero paid; two attempts, one handoff, unaccepted root result and
zero Applications. Owner originals stay byte-identical. This is native host/control
qualification, not live-model autonomous repair quality. Never replay its setup,
approvals, repair or counters; preserve both results/grant/usage/checkpoint.

`repository-check-advice-native/result.json` also passes actual terminal CLI exit0
and independent read-only JSON verification. Declared folder/file-type advice is
labeled; all captured check argv remain unchanged and required. No dependency
analysis, check execution, daemon/model/task/grant or owner/index mutation occurs.
This qualifies bounded tasks19–20; revision-pinned comments, final-file recheck,
reviewed Git controls and all other remaining independent daily-use work continue.

## Pinned owner review and local Git source checkpoint in progress

Tasks21–23 reuse immutable versioned controls, the native bounded check runner and
Git. ReviewComment pins current task revision/result digest/complete artifact SHA
and one-based line range; host rechecks real text before saving, with current/
historical labels in the inspector. `review-comment-state-tests.log` passes one
state persistence/stale/file/range/authority regression (2m48s build).

Independent final-check captures saved owner files, freezes all copied inputs,
runs every captured native argv, rechecks snapshot/original hashes and saves a
private owner verification. It creates no backend result, grant, acceptance or
Apply. Commit preview rechecks every captured input/output, current Git identity,
empty index and common secret signatures, then pins changed declared outputs and
message. A private durable intent consumes the reviewed plan before staging;
staged/committed paths, blobs and parent are checked. Failures retain inspection
receipts; no automatic index reset/replay or push. Signed/clean-filter operations
retain configuration and use the native owner terminal. Read owner-review-and-git.md.

`owner-reviewed-git-tests-v2.log` passes an actual native Git regression (5m40s):
dirty unrelated work/index preserved during preview, stale hash and unrelated
staging refused, exact reviewed local commit verified, replay refused and unsafe
receipt permissions rejected. Its check receipt is explicitly synthetic unit
input; actual final-check/native application qualification remains. The first
compiler run is stopped before full-disk linking; verified idle relocation moves
1,699,508,250 generated bytes/39 files with hashes/mtime preserved. Runtime/TUI,
contracts, lint/package and actual owner workflow are still in progress.

The resumed runtime selection passes20 app-server,3 CLI and110 TUI checks
(`owner-review-runtime-ui-tests-v3.log`), including actual official Neovim. The
already compiled current state executable passes its full346 tests, exit0 in
14.56s (`owner-review-full-state-result.json`). Stable/experimental generators
pass and Python SDK generation/25 contract/signature checks pass. Full repository
formatting passes, exit0 (`owner-review-full-format-result.json`); the generated
schema comparisons and production lint/build/native workflow remain pending.

The first schema tool session disappears without a completion receipt and leaves
no live compiler; its incomplete log is retained. A persistent runner resumes
only reversible compilation/generation and records exit0 after209.57s. Subsequent
commands retain explicit intent/running/result records to distinguish interruption
from success. Twelve hash/mode/owner-verified links deduplicate776,863,744 bytes
of stopped immutable package assets while preserving every path/byte/state; the
environment execution server is outside those assets and remains untouched.

All four regenerated schema comparisons pass, exit0. Production CLI Clippy with
`-D warnings` passes, exit0 in419.55s (`owner-review-production-clippy-result.json`).
The source avoids unnecessary Apply-base blobs during independent final checks,
and rechecks every frozen check input immediately before native Git commit.
Full repository formatting passes again after SDK generation. Final selected CLI
tests, clean stamped package and actual native walkthroughs continue; this is not
the completed forty-task batch.

## Pinned feedback and owner-file/Git native qualification

Application source `6bf3e340938faefe453e7b3018051ce5107c4fcc` is committed and
pushed/independently verified on `work`; `main` stays8026444c98cd. Final selected
CLI3 tests pass, exit0 after337.47s (`owner-review-cli-final-tests-result.json`).
The clean stamped CLI builds in446.58s, exit0. Package
`package-review-preview8`, version0.0.0-daily.review.preview8, retains a verified
five-asset proof. CLI SHA256:
`34ad32349802fb04301d1e9475cfc0940c7e57c4d114542597bc99c6be659f5b`.

`review-comment-native/continuation/result.json` passes actual physical TUI
palette/form/F6/y feedback, exact task revision/result/file SHA/lines1–2, invalid
line/hash refusals, unchanged canonical execution view and actual official Neovim
diff. Normal CLI exit0/daemon stop0. Its original task retains2 localhost
responses/30 synthetic tokens, unaccepted root result, no Application and unchanged
owner originals. The continuation creates no grant or model request. Full offline
bundle/fresh restore and read-only CLI verification preserve the exact comment,
control history, grant/result and counters; restored global pause is verified.

`owner-check-git-native-v2/result.json` passes physically entered actual CLI
commands in official-Neovim terminals. Wrong saved code fails the immutable
oracle. Actual Neovim normal/insert/save/quit edits multiplication; fresh native
checks pass its saved bytes. A second captured argv proves frozen-file write
denial, protected original/home read denial and absent provider credentials.
Both argv/check/input pins remain in the verification. A further independent
check proves a nonsecret parent environment marker is excluded from native checks.
No organization database/base-blob store, grant, result, acceptance or Apply is
created by these owner-only checks.

Preview leaves index bytes unchanged. Changed output or immutable check input,
unrelated staging, configured signing/clean filters and missing confirmation are
refused. Configuration and unrelated index bytes stay intact. The exact one-file
local commit is `cb00393f6d42878a909433d1f03cdd074ef9265b`; parent/path/blob and
private completed receipt are verified, unrelated dirty files remain and replay
is refused. All command terminals and actual editor exit0. Additional real checks
pass for harmless `.env.demo`/dummy private-key-signature fixtures, but preview
refuses both before staging; HEAD/index unchanged and no new commit. These are
bounded secret screens, not a comprehensive detector or provider quality claim.
Git previews/receipts are separate owner JSON evidence, not SQLite tool actions.

Failures remain: first check driver caught EACCES but native read-only mounts
return EROFS; corrected guard requires errno13/30. The first comment driver
typed21 at a prefilled cursor rather than2 and the form refused it. Feedback-only
continuation corrects cursor input without replaying its completed grant/tool
approval or altering usage. Neither failed driver is described as a normal exit.
All owned services are stopped. Never rerun native setup/grants/feedback/Git
commit or reset counters/index; retain both failed fixtures and completed receipts.

After compiler writers stop,180 unreferenced obsolete relocated generated files
are hash-recorded/pruned (4,595,937,280 volatile bytes). Verified idle generated
relocations free1,509,519,360 root bytes/30 files and1,001,082,880 bytes/11 files,
preserving content, mtime and cache paths. Source, packages, owner/state/recovery
data and every receipt remain. Continue tasks8/27–28/31–35/38–40 and remaining
final qualification; suitable-host task13 remains externally blocked.

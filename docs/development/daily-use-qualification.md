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

Finish current-source lint/protobuf/selected TUI checks and logical commits,
then conservative cost admission, current-release live coding/delegation and
the local package lifecycle. Tasks
5–8/10–40 remain unfinished. Task13 remains gated on an actual suitable host.
Use fresh native fixtures, retain unknown usage and the remaining $1 envelope,
and preserve every historical quarantine. Do not rerun either diagnostic script:
the first has preserved unsent input; the second is a completed paid turn.

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
**18,256,592,896 bytes reclaimed**. Root storage now has approximately18GiB
available. Earlier packages and recovery archives remain byte-identical. The
profile cache under `/tmp` still has limited headroom; inspect it before builds,
use one locked dev-small selection and never prune while compilation runs.

## Next work

Task3 low-cost catalog qualification and task4 diagnostics, then supported
response/retry/cost controls and current-release live coding/delegation. Tasks
5–8/10–40 remain unfinished. Task13 remains gated on an actual suitable host.
Use fresh native fixtures, retain unknown usage and the remaining $1 envelope,
and preserve every historical quarantine. Do not rerun either diagnostic script:
the first has preserved unsent input; the second is a completed paid turn.

# Codex CLI fork development instructions

## Current phase and approval

This project extends Codex CLI into a general-purpose autonomous multi-agent
system. Phase 0 research and planning are complete. The owner approved the next
step with “Okay, let's do this”: M0/M1 qualification of the real CLI, detached
daemon and official Neovim embedding, including the tools needed for those checks.
These foundations are qualified; see [qualification](docs/development/qualification.md).
The owner subsequently approved implementation: “Okay, let's implement this.
Use Ponytail to implement, and then commit periodically, follow the MVP plan”.
Implement B1–B6 in order with meaningful tests, real interactions and logical
local commits. No repeated approval is needed for ordinary MVP slices.
External authority and live-provider spending remain separately bounded.
The owner also approved the next usability slice: TUI project/task creation,
explicit bounded authorization and matching the supplied concept art as closely
as terminal behavior permits, using Ponytail and periodic local commits. Keep
this scope/evidence current in status.md; later memory/scheduling
work requires its matching scope.
The owner approved the next slice with “continue and begin”: whole-file selective
Apply in Review, using Ponytail and periodic commits. Preserve exact acceptance,
owner confirmation, base/artifact checks, native dirty-buffer protection and the
one-application-per-result recovery rule. Memory retrieval/schedules were later
scope at that checkpoint; N1 was subsequently approved below.
This selective Apply slice is qualified on `9fadf32797`; see
[selective Apply](docs/development/selective-apply.md) and [testing](docs/development/testing.md).
Record new approvals and scope in status.md; a plan alone does not authorize work.
The owner approved N1 scoped persistent memory search with “Ok do this”: reuse
durable notes, enforce existing project/agent boundaries, expose agent retrieval
and owner CLI/TUI search with provenance, and qualify restart/isolation through
meaningful tests and actual interaction. Continue Ponytail and logical local
commits. N1 is qualified on `d6d049cc49` with 691 automated tests and actual
CLI/TUI/official-Neovim restart/isolation evidence; see
[memory search](docs/development/memory-search.md) and testing/status.
The owner approved N2 recurring schedules with “ok lets do this”: bounded interval
templates, daemon execution, pause/resume/cancel, next-run visibility, aggregate
limits, restart and physical CLI/TUI qualification. Use Ponytail and periodic
local commits. N2 is qualified on `e06def499e`: 700 automated tests and actual
background/approval/pause/restart/resume/cancel/budget/crash/remote-denial/TUI/
official-Neovim evidence. Five bounded series/seven occurrences and all failures
are preserved; owned services are stopped. Never blindly rerun N2 fixture scripts
or reauthorize its historical occurrences. N4–N8 remain later scope; see
recurring-schedules.md, testing.md and status.md.
The owner approved N3 with “ok do this”: reusable bounded workflow templates,
explicit run previews/scopes, fresh task identities, existing dependency/approval
enforcement, Projects/Operations UI, actual CLI/TUI/official-Neovim qualification,
Ponytail and periodic local commits. N3 is qualified on `2bd752a3d1`: 712 automated
tests and actual CLI/TUI/official-Neovim dependency/restart/approval/conflict-denial/
Apply evidence. Preserve all three invocations/eight tasks; services are stopped.
Never rerun N3 fixture setup/authorization/lifecycle/Apply or reset its limits.
N4–N8 remain later scope. See workflows.md, testing.md and current status.
The owner approved the 12-item expanded batch with “ok lets do this” on
2026-10-11: Apply recovery, checkpoints, fenced reassignment/blockers, Operations
editing, fuller workflow UI, Review action inbox, evidence summaries, recorded
resource visibility, actual-Neovim selection requests, capability inventory and
complete physical/automated qualification. Follow the bounded
[expanded batch](docs/development/roadmap.md#expanded-next-batch) in order with
Ponytail and logical local commits; no repeated approval is needed for its slices.
External integration qualification, paid provider pilots, automatic Apply,
whole-workflow recurrence, plugins/LSP and distributed workers remain separate.
Expanded-batch items 1–4 are qualified: Apply recovery on `f0175b9076` and
checkpoints/blocked owner handoffs on `dcf53641b8`, with 731 selected automated
tests and actual CLI/TUI/official-Neovim restart, unrelated work, same-grant
transfer, checks, acceptance and one Apply. Items 5–6 are also qualified on `55af0a0b5b`; preserve `owner-ui-home`, its four
tasks, two grants/results and one Apply. Never replay its fixture mutations.
Items 7–12 are qualified on workflow source `fe5ac65949` and final corrected
inventory source `338104eb99`: 748 distinct selected checks, actual CLI/TUI/official
Neovim selections/approvals/changes/dirty refusal/diff/acceptance/one Apply/restart,
36 layout switches and final read-only package verification. All 12 approved items
are complete. Preserve `review-selection-home`, its five tasks/grants, four
attempts, two results and one Apply, including the interrupted task and consumed
usage. Six localhost responses/90 fixture tokens/zero paid; services stopped.
Never replay its authorization/acceptance/Apply or reset counters. See
[reports and selection](docs/development/reports-and-selection.md) and final
[testing evidence](docs/development/testing.md#review-reports-inventory-and-native-selection-qualification).
Further external integration, paid probes, plugins/LSP, shared buffers, whole
workflow recurrence and distributed execution retain their separately stated
scope. Read current status before new work.
Preserve `continuity-home` and its two historical tasks; never rerun the
continuity setup/authorization/handoff/Apply scripts or reset limits. See status
and testing for all failures, corrected evidence and exact resume instructions.
Preserve N1's accepted writer
and three needsReview reader fixtures; never blindly rerun authorization scripts.

The owner approved the revised twelve-item autonomy/usability batch with “ok lets
do this; use ponytail, commit periodically, document and test”. Follow
[its bounded scope](docs/development/autonomy-batch.md), starting with whole-run
controls and shared limits. External integrations are explicitly deferred. These
approved items supersede older deferrals for recurring workflows, supported
Neovim configuration/LSP and other listed local features. Preserve exact grants,
review/Apply, uncertainty fences and minimal spending. No repeated approval is
needed for ordinary slices; update progress and evidence continuously.

This revised autonomy batch is now complete/qualified on application source
`37eeac36a9`: 770 selected automated checks and actual CLI/TUI/official-Neovim,
mixed-GPT, finite automation/team knowledge, crash/full restore and one restored
Apply pass. See the latest status and final testing section. All owned services
are stopped. Preserve every autonomy fixture/home, failed driver, grant, usage,
bundle and completed Apply. Never rerun their setup/authorization/review/Apply,
reset ceilings, redo paid pilots or restart the original quarantined source home;
read `final/workspace-quarantine.json`. Linux unit syntax/foreground recovery pass,
but systemd user-manager/logout/reboot remains unqualified. No approved batch
implementation remains; choose new scope with the owner. External integrations,
distributed workers, shared buffers and automatic acceptance/Apply remain deferred.

Read [docs/development/README.md](docs/development/README.md) and
[status.md](docs/development/status.md) at the start of every session.

The owner subsequently approved the [next twenty tasks](docs/development/next-twenty.md)
with “ok lets do this”. Implement their bounded local scope with Ponytail,
meaningful automated/physical checks, continuous docs and logical local commits.
This supersedes older deferrals for the twenty listed capabilities. External
integrations, distributed workers and automatic root acceptance/Apply remain
deferred. Complete independent work when real-host systemd/reboot qualification
is unavailable; never claim container restart proves host reboot. Preserve all
historical state/accounting/evidence and use fresh qualification fixtures.
Tasks 2–20 are now implemented/qualified on final application source `18cc403424`,
with 790 selected automated checks and actual CLI/TUI/official-Neovim/background/
crash/full-restore evidence. Task 1 remains approved but blocked on a real-host
user manager/logout/reboot test; no independent implementation remains in this
batch. Read [operations native qualification](docs/development/operations-native-qualification.md)
and current status for package hashes, retained failures and bounded limits.
The original reviewed-autonomy fixture has one charged unknown request; keep it
paused/stopped. Never restart either quarantined reviewed-autonomy-v2 or controls
source home; read their workspace-quarantine mappings before any read-only work.
The controls code result has one exact partial hunk Application; never replay its
pre-Apply driver or write a second backend Application. Later explicit native
owner save produces the full calculator without rewriting that receipt. Preserve
all grants/accounting/private checkpoints/bundles and use fresh restores for
further review. All owned batch services are stopped; zero new paid calls ran.
The owner subsequently approved the next twenty hierarchy/autonomy tasks with
“ok lets do this”. Follow [hierarchy/autonomy batch](docs/development/hierarchy-autonomy-batch.md)
through all independent tasks, starting with the concept-like branching tree.
This supersedes prior deferrals for its listed local capabilities, including
bounded safe recovery and native multi-tab checkpoints. Preserve external-action,
paid-provider and automatic root acceptance/Apply boundaries. Continue Ponytail,
real testing, documentation and logical local commits; no repeated approval is
needed for ordinary slices. Historical qualified homes stay protected/stopped.

This hierarchy/autonomy batch is now complete on final application source
`970ba69195`: all twenty bounded tasks, 806 selected automated checks and actual
CLI/TUI/official-Neovim/background/10,005-node/crash/full-restore checks pass.
Read [hierarchy native qualification](docs/development/hierarchy-native-qualification.md)
and the latest status/testing before historical checkpoints. All owned services
are stopped. Original `hierarchy-batch/autonomy-native/native-home` is quarantined;
NEVER restart it, replay fixture grants/acceptance/Apply or reset usage. Read its
workspace-quarantine.json and review only a fresh restore. Preserve the completed
Application, 36 admitted/35 reported requests, 525 synthetic tokens and one charged
unknown; the fresh restored home stays paused/stopped. Bundles exclude owner
originals, configuration/credentials and private frontend Neovim checkpoints;
retain those separately. Actual multi-tab checkpoints support at most eight tabs
and 16 total windows/files. Dashboard loads at most 20,000 nodes/goals; active
execution remains default two/max four. No independent batch implementation
remains. New scope needs owner direction; prior real-host logout/reboot qualification
is still approved but requires a suitable host. External integrations, distributed
workers, new paid probes and automatic root acceptance/Apply remain separate.

The owner has now approved the entire next 25-task mission/usability batch with
“ok lets implement. ponytail, commit periodically, document, test”. Follow
[mission batch](docs/development/mission-batch.md) through all independent tasks,
using existing code, Ponytail, meaningful automated/actual CLI/TUI/official-Neovim
checks, maintained docs and logical local commits. No repeated approval is needed
for ordinary slices. Preserve external/paid/automatic root acceptance/Apply and
active-capacity boundaries and all protected historical fixtures. Do not mark
this batch complete at a source-only checkpoint; its finite 30-minute local
qualification is required. Real-host logout/reboot remains separately blocked.

The 25-task mission/usability batch is now implemented/qualified on final
application source `8cd392eaaa`: 836 selected checks and actual CLI/TUI/official
Neovim, 1846.319-second detached run, one owner-confirmed Apply and crash/fresh
full restore pass. Read [native qualification](docs/development/mission-native-qualification.md)
and latest status/testing first. All owned batch services are stopped. **NEVER
restart `/tmp/mission-native-home`**; it is quarantined. Read
`mission-batch/native-workflow/workspace-quarantine.json`. Preserve its one
Application, 38 admitted/37 reported requests, 555 synthetic tokens and one
charged unknown; never replay setup/grants/acceptance/Apply or reset limits.
Fresh `/tmp/mission-restored-home` is paused/stopped; further review uses a fresh
restore. UI home/source, all older protected fixtures, packages, failed drivers,
private state and receipts stay preserved. Generated dev-small cache was pruned
only after tests/lint/build; it needs a cold rebuild with checked disk headroom.
No independent approved batch implementation remains; select new scope with the
owner. Full root formatting needs missing just/dotslash and untouched legacy
formatting work; changed-source checks pass. Real-host reboot/logout and external
integrations/paid probes/distributed workers/increased concurrency/automatic root
acceptance or Apply remain separately scoped. No push/publication is implied.

The owner approved all [30 adaptive-operation/usability tasks](docs/development/adaptive-batch.md)
with “ok lets do this”. Implement the whole bounded list in order using Ponytail,
logical local commits, maintained docs, meaningful tests and actual CLI/TUI/
official-Neovim. This supersedes older local deferrals for the listed capabilities;
no repeated approval is needed for ordinary slices or necessary verified build
prerequisites. Preserve previous fixture/accounting/quarantine and explicit
owner acceptance/Apply controls. No new paid inference/external integrations/
increased concurrency/push/publication are implied. Code Mode must be qualified
through existing host controls before enabling its models. Do not finish at a
source-only checkpoint; complete all independent work and finite endurance
qualification. Real-host reboot/logout still needs a suitable host.

Adaptive tasks1–28 now have source/test checkpoints; see
[adaptive evidence](docs/development/adaptive-qualification.md). Selected state339,
protocol377, app-server16, TUI106 (including official Neovim) and runner7 checks pass.
Subsequent Code Mode work on `200bc7222b` passes state341/protocol377,
actual installed-host isolation/CPU interruption, schema4 and Python contracts5,
with preserved earlier app-server17/TUI106 checks. Fresh compatible-model native
coding/approval/pause/path/immutable/budget/deadline/stale-owner checks passed;
Only-model admission and abandoned-cell cleanup remain unfinished/blocked.
The environment replaced itself and cleared all `/tmp` and shared-memory data.
Persistent receipts retain22 reported localhost requests/330 synthetic tokens;
the old adaptive native database/workspaces and preview packages are unavailable.
Never recreate their authority or replay intents from inspection JSON. Read the
environment-replacement section of adaptive evidence. Use a new persistent fixture
home/evidence directory, save offline checkpoints/packages, and continue full
qualification after the running cold rebuild. Earlier fully qualified archives
and historical quarantine rules remain protected; no historical daemon starts.
The later `e1212cd218` shared authorization fix passes342 state/protocol377 and
exact native refusal. Actual official-Neovim workflow now has one protected Apply,
36 layout switches and normal exit0; crash/full backup/fresh restore also pass.
`recovery-native` and its paused restore are stopped/quarantined with20 admitted /
19 reported requests,285 tokens, one charged unknown and1000 reserved nominal
tokens. Never restart or replay them. `final-native` uses fresh residual58-request /
2385-token /two-active ceilings; its definition checks pass without inference.
Only-model source admission now requires captured Code Mode scope and the actual
host before admission; app-server17/TUI106/full formatting/Python-contract5 pass.
The final catalog-matched pilot, remaining selected checks/package and endurance
are pending. Preserve failed drivers/builds and all package/state/usage evidence.
Continue tasks29–30 and all remaining schema/SDK, production lint/package and
actual CLI/TUI/endurance qualification. Do not mistake this checkpoint for a
completed approved batch. The last fully qualified application package remains
`8cd392eaaa` until the new complete native qualification passes.

Then read [decisions](docs/development/decisions-and-risks.md),
[MVP plan](docs/development/mvp.md), [testing](docs/development/testing.md) and
[process](docs/development/process.md). Reconcile current user instructions with
the recorded approval state; do not infer approval from a plan.

Preserve all existing instructions, including scoped AGENTS.md files.
For future touched directories, discover and read the applicable scoped files.
No earlier root AGENTS.md existed when these instructions were added.

## Design rules

- Users choose their own agent roles, names, hierarchy levels, groups and scale.
  Example CEO/director/manager/worker organizations are optional templates.
- Reuse Codex runtime, AgentControl, daemon, thread store, providers, tools,
  security controls and TUI where practical. Inspect existing code and callers
  before changes; avoid unrelated rewrites and speculative abstraction.
- Use Ponytail during implementation and review. Apply reuse/stdlib/native/installed
  dependency first; keep the smallest correct solution. Do not simplify away
  requirements, security, error handling, accessibility or meaningful tests.
  See [Ponytail plan](docs/development/ponytail.md). Use the available skill;
  report missing integration rather than installing without authorization.
- For UI work, inspect the supplied concept images in docs and
  codex-rs/tui/styles.md. Reuse terminal_palette/style contrast helpers rather
  than bypassing palette checks. Show canonical state and real evidence; never
  paint invented CPU/cost/test metrics or imitation editor functionality.
- Use actual official Neovim from https://github.com/neovim/neovim.
  Never implement a custom editor, fake Neovim or Vim clone.
- Preserve Console, Workspace, Projects, Operations and Review as customizable
  layouts. Operations' primary view is an interactive configurable hierarchy tree.
- Organization, task and approval state must outlive the TUI/editor. Protected
  authority/budgets are enforced in code, not only agent instructions.
- Each cloud task already has an isolated environment. Use existing checkouts;
  do not create a Git worktree unless explicitly requested. Future product-managed
  isolated agent workspaces are a separate approved feature.

## Implementation and verification after approval

- Write meaningful automated tests that fail for broken behavior. Use existing
  frameworks and relevant integration/state/security/recovery tests.
- **Actually run the application, physically enter commands, interact with the
  TUI, and verify affected features.** Inspection, snapshots and mocked tests
  alone are insufficient.
- Test real Neovim functionality using the actual editor: modes, file edits/save,
  undo/diff/follow, focus/resize and dirty-buffer conflict handling as applicable.
- Fix failed approved behavior/setup and rerun affected tests before marking work
  complete. Do not disable assertions, skip required checks or weaken TLS/signature/
  checksum/permission verification to obtain a passing result.
- Record exact commands/directories, versions, exit codes, test counts, interaction
  steps, artifacts/evidence and limitations. Distinguish mocked/live/manual/
  synthetic tests and passed/failed/skipped/unrun outcomes.
- Run appropriate repository-required checks. Preserve source/lockfiles during
  environment onboarding; implementation changes require approved feature scope.
- Keep docs continuously updated: what changed, why, how it was tested, remaining
  issues, decisions, scope and next session's concrete action. Update status and
  roadmap rather than leaving future sessions to reconstruct intent.
- Make frequent logical Git commits with meaningful messages, including related
  tests/docs. Inspect staged paths; avoid unnecessary dependencies and complexity.
- Preserve user changes, files and editor configuration. Never commit secrets,
  credentials, unrelated work or generated logs. Do not reset or overwrite work
  to hide test failures.
- Local commits do not imply permission to push, merge, publish, deploy, purchase,
  message third parties, install tools or expand operational authority.

## Completion

Use the approved [acceptance criteria](docs/development/mvp.md),
[current release matrix](docs/development/release-qualification.md),
[operating limits](docs/development/resource-limits.md) and test evidence.
Do not claim a feature works if it was not exercised or its checks failed.
Document blockers concretely and stop at the user's approval gates.

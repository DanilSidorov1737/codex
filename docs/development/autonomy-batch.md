# Revised autonomy and usability batch

The owner approved the revised twelve-item plan with “ok lets do this; use
ponytail, commit periodically, document and test”. External integrations are
deferred. This supersedes the older deferrals for the items below; outbound
messages, publishing, purchases and distributed workers remain outside scope.

Reuse the daemon, protected SQLite transactions, owner RPC, native forms, finite
schedules, scoped notes and actual official Neovim. Preserve exact-action
approvals, immutable checks, original-file conflicts and uncertain-effect fences.
Start with whole-workflow controls and shared limits before more automatic work.

| Item | Bounded delivery and acceptance |
| --- | --- |
| 1 | TUI customization: adjustable panes, saved per-layout preferences and consistent navigation in all five concept-art layouts; physical resize/switch/restart checks at multiple sizes. |
| 2 | Mixed-model qualification: explicit compatible model selections in one workflow, recorded observed compatibility and unsupported-mode denials. Reuse the current provider; minimal paid checks stay within the remaining conservative $1 total pilot ceiling. Label fixtures separately. |
| 3 | Shared limits: aggregate project/workflow requests, reported tokens and concurrency enforced alongside step/root limits; descendants/handoffs count once, consumption survives controls/restart. Document in-flight allowance; estimates are not money caps. |
| 4 | Goal-to-plan: request a bounded planner for an existing goal, inspect proposed milestones/tasks/assignments and save a workflow. Execution still needs exact explicit scopes. |
| 5 | Whole-workflow pause/cancel/continue: versioned owner controls atomically fence run roots/descendants, preserve completed work/accounting and reject unsafe continuation. No automatic acceptance/Apply or grant reset. |
| 6 | Recurring workflows: finite UTC intervals pin template/scopes, retain aggregate ceilings, skip missed/overlapping runs and admit fresh identities with existing review. |
| 7 | Internal triggers: finite preauthorized follow-ups on accepted task results, durable deduplication, project checks and ceilings. No external webhooks or automatic acceptance. |
| 8 | Shared knowledge: owner-mediated note promotion into project/team knowledge, provenance, explicit visibility and corrections; membership/project filtering precedes retrieval. |
| 9 | Neovim configuration/LSP: opt-in owner configuration preserving existing files; physically test actual diagnostics/navigation/completion and a supported local plugin. Default clean mode remains; no automatic downloads. |
| 10 | Background reliability: Linux user-service support, canonical health/stall visibility and bounded stop/escalation. Prove recovery without uncertain replay; running-host and reboot evidence remain distinct. |
| 11 | Full backup/restore: owner bundle covers business/runtime state, task workspaces/artifacts with integrity manifest; restore to a fresh home and prove review/accounting/recovery. Preserve originals/credentials; uncertainty remains fenced. |
| 12 | Qualification throughout: meaningful negative/restart/concurrency tests, actual CLI/TUI/official-Neovim workflows, exact evidence/failures/limits, maintained docs and logical local commits. |

## Progress and resumption

Initial source: `db0c2161e93a5bfee345fec8894e067b900ee384`; worktree clean.
Status: **in progress**. Item 5 implementation checkpoint adds versioned run
pause/resume/cancel through existing owner RPC/CLI and confirmed TUI decisions.
Seven workflow state tests pass; the production CLI check passes. Native API/TUI
and physical qualification are in progress; this is not release qualification.
Evidence logs: `autonomy-workflow-controls.log` and `autonomy-workflow-check.log`
in the existing qualification parent. Item 3 now has migration 10, root-ledger
projections, transactional admission/concurrency fences, exact owner edits and
optional caps in immutable workflow previews. **57 organization state tests pass**,
including concurrent reservation and 32-item paused/exhausted queue fairness;
12 runtime, 47 TUI/official-Neovim and 8 native API checks pass. Production CLI
Clippy passes with warnings denied. Extra form assertions, schema regeneration
and physical qualification continue before release.

Items 3/5 now pass 373 protocol, 285 state, 2 wire-contract, 12 runtime,
48 TUI/official-Neovim, 8 native API, 4 schema comparisons and 25 SDK checks.
Generated JSON/TypeScript/precomputed exports and Python types are refreshed.
The fresh actual CLI/TUI/official-Neovim receipt `physical-v3.json` passes pause,
refused paused approval, same-grant/attempt continuation, two exact approvals,
accepted prerequisite fencing, immutable subprocess checks, actual editor diff,
one explicit Apply, failed-run cancellation and 36 switches at three sizes.
The successful run consumed four localhost responses/60 fixture tokens; no paid
calls. Two earlier runs are retained: provider startup ordering left one unknown
request; fixture command quoting consumed two responses before failing. No limits
were reset and no failed grant was replayed. The first physical helper failed
before a mutation due to a missing helper name; corrected evidence is retained.
This is qualification of these slices, not completion of all twelve items.
Fresh evidence: `/workspace/.onboarding/qualification/autonomy-batch/`.
Preserve all historical fixtures/evidence/packages; never replay grants/Apply.
Missing provider access, local LSP binary or service manager is a prerequisite to
diagnose while independent work continues, not a reason to claim fixtures are live.

## Whole-run control contract

`workflow-mutate` accepts `control` with run `id`, `expectedRevision` and
`action` (`pause`, `resume`, `cancel`). In Projects/Operations, select a run and
press p/u/x, then confirm the exact displayed owner decision. Legacy runs load as
active revision 1. New controls increment the run revision and audit history.

Pause stops new model/native-tool/coordination admissions. Previously admitted
requests/effects may finish and record receipts/results; pause is not rollback.
Exact approvals cannot be approved while paused. Quiescent paused runtimes release
active capacity and reacquire it before continuing. The existing four resident
root ceiling remains; deadline/expiry clocks keep advancing while paused.
Resume keeps task/grant/attempt identities, counters, expiry and completed work.
Stale controls, cancelled runs, expired/changed live scopes and uncertain or
restart-interrupted work refuse resume. Owner checkpoint resolution is separate.
Cancel atomically fences unfinished roots/descendants; completed results and
reconciliation evidence remain. No automatic acceptance, Apply or new grants.

## Shared-limit contract

`workflow-list limits` returns canonical settings and usage. `workflow-mutate`
accepts `setLimits` with `projectId`, optional `runId`, nullable
`expectedRevision`, `maxTokens`, `maxModelRequests` and `maxActive`. New records
require null revision; edits require the displayed revision. The TUI b key opens
the same exact form for a project/task/run/limit. Shared workflow caps may instead
be supplied as `sharedLimits` in the immutable preview and installed atomically
with its tasks/grants. Blank shared preview fields retain existing fixed grants.

Project usage includes all historical root grants, including revoked ones; run
usage includes its original step roots. Each root already includes descendants.
Handoffs, pause/resume, ordinary reauthorization and restart cannot reset totals.
Existing step/root/schedule ceilings and the daemon's active limit still apply.
Requests reserve counters before inference in the same SQLite transaction.
Reported token ceilings stop subsequent inference/tool dispatch; admitted requests
may overrun before reporting. Unknown receipts remain unresolved, never free.
Terminated unreported requests block further work under the corresponding shared
limit until a genuine late usage receipt reconciles them. Live bounded in-flight
requests retain their existing allowance. No model/owner text resets accounting.
Lowering concurrency below currently occupied capacity is refused: pause/drain
work first. Changing a cap requires an explicit versioned owner decision.
Paused/exhausted work is filtered before the 32-candidate bound so unrelated work
can proceed. The same SQLite projection serves eligibility, enforcement and usage.

The initial legacy migration fixture left migration 10 while deleting migration 9;
it failed checksum/version validation as intended. Its simulated old schema now
removes all subsequent schema/records, without changing historical checksums or
weakening restore validation; corrected tests pass. An early UI build overlapped
protocol edits and saw stale dependency types; the fresh build passes. Failed logs
remain alongside corrected ones in the qualification parent.

## Planner and layout checkpoint

The P action on a goal opens a proposed planner task with captured goal content,
measurable criteria and a bounded configurable roster. Saving that form grants
no execution. The owner authorizes the planner with existing fixed scopes and
reviews its `plan.json` artifact. P on the accepted planner imports verified
artifact bytes into the existing editable workflow form; the server validates
its graph/assignments before saving. Running still needs an immutable preview
and explicit step scopes. Milestones are represented by step instructions and
criteria; there is no new planning DSL or automatic execution authority.

Dashboard panes now save sidebar visibility/16–40-cell width and Actions-panel
visibility independently per layout, alongside the existing inspector width,
visibility, focus and navigation preferences. Host keys: `\\` sidebar, `{`/`}`
sidebar width, `~` Actions panel, `0` reset current layout, `[`/`]` inspector.
Small terminals still collapse the sidebar and stack the inspector. Console
uses the existing Codex conversation composer and its native theme/status-line
settings; dashboard-only pane controls apply in the four dashboard layouts.
These source changes are in validation, not yet physically qualified.

Official Neovim keeps clean mode by default. An explicit owner-selected absolute
`CODEX_ORGANIZATION_NVIM_CONFIG` opts into a regular owner-controlled config and
its installed plugins/native LSP setup. Codex does not overwrite that config or
download plugins. An actual official Rust `rust-analyzer` toolchain component is
being installed locally for LSP qualification; it is not a new application
library dependency. Tests and physical LSP evidence remain required.

## Real mixed-model qualification

Item 2 passes on the controls package/application source `ed7f9ba22c` using
explicit GPT-5.4 Mini and GPT-5.5 selections in one two-step workflow. Four real
OpenAI Responses requests, two actual native subprocess checks, accepted
prerequisite fencing, explicit owner acceptance and exactly one Apply passed.
The owner original files were absent until Apply. No retries were configured.
Read-only model discovery returned HTTP 200; official model pages were retrieved
through the inherited proxy with TLS verification. Verified rates per million
input/cached/output tokens were 0.75/0.075/4.50 and 5/0.50/30 USD respectively.
Eight preserved provider usage dimensions are not invented invoice data: the
four response receipts estimate **$0.0198223 new / $0.3652553 combined** with the
prior $0.345433 pilots, below the $1 working ceiling. This excludes invoice/tax
adjustments and is not a provider monetary hard cap. No Luna Code Mode bypass,
automatic model routing or external integration was added. The owned daemon is
stopped. Preserve the fresh mixed-model home, workflow, grants, accepted results,
usage and completed Apply; never replay its run script. Evidence:
`autonomy-batch/mixed-models/{receipt,accounting}.json` and `run.log`.

The planner/layout/config checkpoint `59e309181d` passes **51 TUI/editor tests**,
including actual official Neovim loading the explicitly selected owner config
and its bundled matchit plugin without modifying the config. Physical planner,
customization and native LSP qualification remain in progress.

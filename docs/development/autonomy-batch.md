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
in the existing qualification parent. Item 3 shared limits is next.
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

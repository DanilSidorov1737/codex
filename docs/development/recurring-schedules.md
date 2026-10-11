# Recurring schedules (N2)

Status: owner-approved N2 delivered and qualified on application source
`e06def499ef919e381bb8ec6e0ab7518960f280d` (2026-10-11 UTC).
Storage `54954cfb7d`, owner API/CLI `d5f2115211`, dashboard `e06def499e`.

Reuse the existing organization store, daemon poller, task grants, delegation,
approval ledger and isolated workspace execution. No scheduler service, cron
parser or dependency is needed for bounded UTC intervals.

## Contract and acceptance

- Owner creates an immutable snapshot of a task without prerequisites and its
  explicit execution scope. Source task edits do not change the snapshot.
  Agent/project revisions and host policy remain pinned; changes require a new
  schedule. Agents and network clients cannot administer schedules.
- Each schedule has a stable owner-selected UUID, first UTC epoch time, interval,
  end time, run count, aggregate reported-token ceiling and request ceiling.
  Limits are finite: intervals of 10 seconds–31 days, at most 31 days of duration,
  1,000 issued runs and 128 live schedules. The daemon polls about every 5 seconds;
  start times are best effort within their interval, rather than real-time timers.
  Per-run task/delegation limits and exact native approvals remain enforced.
- Each occurrence has a unique (schedule, due time) identity and a fresh task and
  root grant. Occurrence creation, authorization and advancement commit together.
  Root grant counters include descendants; schedule counters sum those roots.
  Admission checks aggregate usage transactionally. Already admitted provider
  usage may exceed a token ceiling; this is not an invoice or hard monetary cap.
- Never overlap executions from one schedule, including waiting approvals,
  children, native cleanup and unresolved usage/actions. Completed results may
  await human review while later isolated runs execute. Acceptance and Apply
  remain separate owner decisions.
  A completed series has finished issuing occurrences; its last task can still
  run or wait for approval. Inspect the occurrence's task state separately.
- Ordinary polling runs a due occurrence only within its interval. If a whole
  interval was missed, advance to the next future time without a catch-up burst.
  Daemon startup and resume skip overdue times. Skips do not consume the run cap.
  Clock rollback cannot repeat an issued due time. Host shutdown/sleep prevents
  execution; a detached daemon does not make the host always available.
- Pause/cancel stop future admission and revoke current occurrence grants;
  native cleanup is still joined before capacity is released. Resume creates
  only future occurrences, never revives an old attempt or approval. Interrupted
  or uncertain effects block further automatic runs for owner investigation.
- CLI and existing Projects/Operations layouts show canonical state, next UTC
  start, usage, skipped/issued counts and recent occurrence task IDs. Controls
  use exact revisions and confirmation. Keep the Operations tree primary and
  preserve the concept-art palette/panels and actual Neovim focus behavior.

Meaningful state/API tests must cover due times, concurrency/deduplication,
restart/migration, missed/overlapping work, pause/resume/cancel, stale edits,
scope/policy changes, descendant/aggregate accounting and recovery fences.
Build the complete package and physically create/control a schedule, close the
TUI, observe native background work and approvals, restart the real daemon,
inspect results and test narrow/wide layouts and actual official Neovim focus.
Use fresh deterministic local fixtures; no new paid pilots are required.

## Implementation sequence

1. Durable schedule/occurrence records, transaction helpers and budget fences.
2. Protected owner API and thin CLI, schemas and meaningful integration tests.
3. Existing dashboard forms, inspectors and controls; no sixth layout.
4. Automated checks, full package and physical qualification, Ponytail/correctness
   review, evidence, current status/roadmap and logical local commits.

## Implementation and qualification

Implemented schedule/occurrence DTOs and migration 7, atomic materialization,
scope snapshots and revision checks, missed-run skipping, nonoverlap, finite
limits and schedule fences around existing model/tool/coordination paths.
Existing authorization and stop logic now have transaction helpers so occurrence
creation and revoke/approval supersession use the same validation as ordinary
tasks. Scheduled roots and children cannot bypass the series by reauthorization.
Late usage remains accounted after pause. Unresolved usage and uncertain effects
block automatic recurrence rather than granting another budget.

The protected owner API, thin CLI, daemon polling/reconciliation, generated
TypeScript/JSON/SDK contracts and dashboard controls are implemented. Default
first start is one hour ahead to allow review of the full authorization form.
**700 distinct automated tests passed**: protocol 371, state 265 (five new
schedule scenarios), native API 6, native TUI/official Neovim 28, schema
comparisons 4, Python contracts 25 and owner-origin 1. Production libraries and
CLI passed Clippy with warnings denied, affected formatting and complete build.

Actual packaged CLI/TUI created the primary series through its 20-field form
and exact confirmation. With the frontend closed, the daemon started its first
run and skipped an overlapping due time while waiting for approval. The exact
native command and immutable oracle passed. Owner pause persisted through real
daemon stop/start; resume skipped the overdue time and issued a fresh future
task with the frontend closed. Both primary runs passed native and independent
immutable report checks and remain needsReview, with no acceptance or Apply.
Actual official Neovim Follow, Ctrl-G host focus, narrow counter scrolling and
36 rapid switches at 120×36, 50×20 and 160×48 passed with normal frontend exits 0.

Fresh bounded cases proved cancellation before native dispatch, aggregate
request exhaustion, and actual SIGKILL/restart recovery. The crashed series
became blocked; its task/attempt require reconciliation, runtime is inactive and
lease is zero. A late exact owner approval was denied. Pending approval is
retained as evidence until owner cancellation supersedes it. No old attempt or
new occurrence replayed after the crash. Real TCP WebSocket reads/controls were
denied. Graceful shutdown is distinct: it cleanly fails the current attempt and
supersedes its approval; the still-active series may issue a fresh future task.
The observed second graceful-case task was cancelled and retained.

Independent verification confirms five finite series, seven unique occurrences,
root/series counters, SQLite quick/FK checks, immutable originals and all five
installed package hashes. Total **9 localhost deterministic Responses / 135
fixture-reported tokens / zero paid calls** includes all diagnostic cases.
This qualifies local schedule controls, not new live-model quality or billing.
Initial compile/test/harness failures remain with corrected evidence; see
[testing](testing.md). No failure was hidden by resetting state or replaying work.

Package: `/workspace/.onboarding/qualification/recurring-schedules-n2/package/`.
CLI SHA256: `f6cd6855de7a3fef9d08f8bf4a51282b73e92cb0f2dbc98b2142be5b3a972b1a`.
Evidence: `/workspace/.onboarding/qualification/recurring-schedules-n2/`.
Private home: `/workspace/.onboarding/qualification/n2-home`.
All owned services/frontends are stopped; cleanup retains canonical state,
packages, artifacts and failures. Do not rerun setup/authorization/crash scripts.
21 actual PNG/text captures and the CRC-checked 47-file archive are in
`/workspace/artifacts/cli-recurring-schedules/`.

## Owner usage

Use the complete N2 package, including its installed daemon.
An existing daemon still uses its previously installed API: inspect active/queued
work before updating it with the new package's
`codex app-server daemon update --from-cli --yes`. Preserve canonical state and
old qualification homes. Use a fresh private home for deterministic checks.

```sh
codex organization schedule-list --project-id PROJECT_ID --limit 100
codex organization schedule-list --cursor LAST_ID --limit 100
codex organization schedule-mutate recurring.json
```

The experimental RPCs are `organization/schedule/list` and
`organization/schedule/mutate`. List takes optional `projectId`, `cursor` and
`limit` (1–200); it returns `data` and `nextCursor`. Each view includes the
immutable schedule, aggregate reported tokens/admitted requests, and the newest
16 occurrence task IDs, due times and task states. Older evidence stays in the
ordinary task store. Owner-origin checks precede reads/writes; agents have no
schedule administration tool. Mutations require the detached daemon in the CLI.

To build a concrete creation file, save the proposed task's CLI mutation response
as `template-record.json` and prepare an explicit ordinary `authorize.json` scope
as documented in the command guide. This script only writes the reviewed JSON:

```sh
python3 - <<'PY'
import json, time, uuid
task = json.load(open('template-record.json'))['record']['data']
scope = json.load(open('authorize.json'))['scope']
scope['notBefore'] = None
now = int(time.time())
series = {
    'type': 'create', 'id': str(uuid.uuid4()),
    'taskId': task['id'], 'expectedRevision': task['revision'], 'scope': scope,
    'config': {
        'label': 'Hourly local report', 'intervalSeconds': 3600,
        'firstRunAt': now + 3600, 'endsAt': now + 86400,
        'maxRuns': 3, 'maxTokens': 30000, 'maxModelRequests': 15,
    },
}
with open('recurring.json', 'x') as out:
    json.dump(series, out, indent=2)
PY
```

Review paths, checks, provider/model, approval choice, per-run and aggregate
limits before executing `schedule-mutate`. Creating a series authorizes future
model work; the source task itself is not authorized by this command. The stable
UUID makes duplicate creation fail rather than creating a second series. After
an uncertain RPC outcome, list state by that UUID before any retry.

Control JSON is `{"type":"control","id":"SCHEDULE_ID","expectedRevision":7,"action":"pause"}`;
`action` also accepts `resume` or `cancel`. Supply the actual freshly listed
revision. Polling advances revisions; stale confirmations fail and require a
refresh. Pause revokes existing run grants and supersedes pending approvals.
Resume skips overdue times and never reauthorizes a stopped occurrence. Cancel
is final. A completed series can still be cancelled to stop its last live run.

In Projects, select a task and press `R`; the recurring form reuses authorization
fields and adds finite interval/count/aggregate bounds. F6 reviews and `y`
confirms; Esc cancels. `S` toggles schedule inspection; `p/u/x` control the
selected series with confirmation. The Operations hierarchy remains the default
view, with an upcoming-runs panel. Inspector times are explicitly UTC, not local
wall-clock calendar rules. Select a recent occurrence task for approval/review;
no result is automatically accepted or applied to original files.
Layout switching clears selection; explicitly select the occurrence in Workspace
before `t` Follow. PageDown/PageUp scroll the inspector at narrow sizes.

Calendar/cron/timezone rules, infinite schedules, automatic original-workspace
Apply, external effects, distributed workers and workflow templates remain later
scope. Preserve all earlier qualification homes, tasks and evidence.

# Recurring schedules (N2)

Status: owner-approved implementation in progress; not yet qualified.

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

## Current storage checkpoint

Implemented schedule/occurrence DTOs and migration 7, atomic materialization,
scope snapshots and revision checks, missed-run skipping, nonoverlap, finite
limits and schedule fences around existing model/tool/coordination paths.
Existing authorization and stop logic now have transaction helpers so occurrence
creation and revoke/approval supersession use the same validation as ordinary
tasks. Scheduled roots and children cannot bypass the series by reauthorization.
Late usage remains accounted after pause. Unresolved usage and uncertain effects
block automatic recurrence rather than granting another budget.

The full state suite passed 265 tests (five new schedule scenarios); protocol
passed 371 tests after adding the wire-contract scenario. Production-library
Clippy passed with warnings denied after boxing the large create-scope payload;
affected format checks and final-source qualification remain in progress.
Initial compile/fixture errors are retained under the qualification directory.
The protected owner API, thin schedule CLI commands, runtime polling/reconciliation,
generated TypeScript/JSON/SDK contracts and native API test are implemented.
All six organization API tests, four schema comparisons and 25 Python contracts
passed; API/CLI checkpoint `d5f2115211` is committed. Dashboard creation, exact
controls, canonical inspectors and Operations upcoming panel are implemented.
The current native TUI/official-Neovim suite passed 28 tests; default first start
is one hour ahead so reviewing the full form does not exhaust a one-minute window.
Final lint/build and physical qualification remain in progress; this checkpoint
is not an end-to-end schedule release.

## Owner usage

Use the complete N2 package when available, including its installed daemon.
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

Calendar/cron/timezone rules, infinite schedules, automatic original-workspace
Apply, external effects, distributed workers and workflow templates remain later
scope. Preserve all earlier qualification homes, tasks and evidence.

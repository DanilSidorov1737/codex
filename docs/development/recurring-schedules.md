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
  Limits are finite: at most 31 days, 1,000 issued runs and 128 live schedules.
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

Calendar/cron/timezone rules, infinite schedules, automatic original-workspace
Apply, external effects, distributed workers and workflow templates remain later
scope. Preserve all earlier qualification homes, tasks and evidence.

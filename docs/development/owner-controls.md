# Operations and workflow owner controls

## Adaptive conversation, planning and goal controls (qualification in progress)

The approved adaptive batch adds the following searchable Commands actions.
They use the existing local owner RPCs and F6 / y exact confirmation. See
[current checkpoint evidence](adaptive-qualification.md); source availability
does not yet establish full native workflow qualification.

- **Talk to selected agent / continue conversation** creates a proposed planning
  task for one project, stable agent and topic. Authorize that turn separately.
  Select its saved control with F11 and **Read persistent agent conversation**
  to see the durable prompts, host-attributed replies and canonical task state.
  Replies are model content. Changing a conversation's project/agent/topic
  requires a new conversation. Finish or cancel the previous draft first.
- **Plan project from a natural-language request** takes an owner request and
  creates a planning draft for the selected project. Grant its local inputs,
  `plan.json` artifact and finite limits separately. Review the result; W imports
  a bounded proposal into the existing editable workflow. Follow-up execution
  has separate reviewed scopes.
- **Goal responsibility / versioned prerequisites** binds an existing goal to a
  project and any configured responsible agent or team. G / E retain arbitrary
  nested goals and their criteria; tasks can point to those goals. Enter loaded
  prerequisite goal IDs, with explicit cross-project metadata approval when
  needed. This shares no files, memory or execution authority. Changing a captured
  prerequisite or its owner completion evidence pauses dispatch until reviewed.
- **Set business due date / local timezone** accepts RFC3339 with an explicit
  offset, for example `2026-10-20T17:00:00-04:00`. Blank clears the date. The
  inspector distinguishes upcoming, overdue and closed work. Fixed offsets are
  retained; IANA zone/DST rules are not supported. Execution grant expiry remains
  independent and immutable.

Owner criterion review accepts verified descendant results as evidence for a
parent's criteria. This never infers completion: the owner maps exact accepted
results to every criterion, and unfinished descendants prevent closing a goal.
Goal hierarchy edits and explicit dependencies share combined cycle checks.

The next adaptive checkpoint adds **Inspect agent readiness / matching owner
actions**. It uses captured canonical definitions/reports, including disabled
ancestors, missing grants, task/goal prerequisites, owner decisions, expiry,
reported budgets, pending receipts and pressure. It marks missing or stale
evidence as unknown and links the existing owner keys/Commands actions. Refresh
the view before acting; it neither probes a provider nor reserves capacity.

The `@` team form now supports **F7 add change / F8 remove added change**, up to32
changes in one reviewed atomic transaction. Each added row selects a new or
captured existing target, its operation, parent, label, role and model. Fresh IDs
stay stable in the draft; existing versions and parents remain pinned. You can
choose an earlier fresh definition as a parent. F6 reviews the entire proposal;
`%` applies its exact saved version. No grants or memory are copied.

Authorized agents can call `proposeRole` or `proposeTeam` to save drafts only.
Role application through `%` creates a new immutable version; E assigns it
separately. Team application reuses the same owner transaction. Agents cannot
supply `applied`, authority or captured revision fields. Existing team references
are restricted to the caller, ancestors and original approved worker roster.
`proposeNextBatch` cites current checked accepted results from the exact original
project/goal/grant, then reuses bounded planning/delegation without renewing
expiry, budgets or cumulative child limits. All new runtime and physical CLI/TUI
qualification remains part of the unfinished approved batch.

**Review meeting conclusion / draft linked follow-up** requires an accepted
`discussion/discussion-conclusion.json` artifact of at most16 KiB. Its exact source
bytes, task revision and result digest are pinned. F6 includes conclusions,
agreements, dissent, evidence and proposals for review. Saving creates a linked
task draft; authorize its actual work separately.

Agents can use `proposeKnowledge` to draft a synthesis and conflict claims from
permitted current notes with exact source hashes. Select its saved control and
`%` reviews the unchanged proposal and current sources before creating one private
synthesis. Claims remain untrusted; this does not settle truth, replace originals
or publish them. Existing separate retirement/correction/promotion actions remain.

**Publish / revoke owner-approved workspace knowledge library** opens opt-in
project/source toggles. Search owner-reviewed PROJECT notes first to provide
source candidates. Select every participating project, including each source's
project, and the exact notes to publish. At most32 projects /64 sources are allowed.
Private notes cannot be published. F6 / y saves a versioned policy; clearing its
selections revokes future retrieval. Hash drift, retirement or correction also
exclude a source. Search/context enforce membership before reading foreign rows,
retain original provenance, and grant no execution or original-file access.
Previously delivered content is not erased by withdrawal. These source/test
checkpoints still await the full native workflow qualification.

Expanded-batch items 5–6 reuse existing guarded owner RPCs and native forms.
Qualified on `55af0a0b5b`; see status.md and testing.md for evidence.

In Operations, `U` creates an agent and `B` creates a passive group. Select a
node and press `E` to edit its name, primary parent, immutable role assignment,
model and enabled state. Groups have no execution role/model. Retirement uses
the existing reversible disabled flag and retains records/history. Any user
names and hierarchy levels are supported; the store rejects cycles and stale
captured revisions. Editing definitions grants no execution authority. Revision
changes can fence existing grants; attempts retain captured role/policy versions.

`L` creates a new immutable role version, based on the latest loaded version of
the selected node's role. Bind it separately with `E`; creating a version does
not change old assignments. If the role is not loaded, it creates a separate
role. `I` adds/removes secondary group memberships, which do not change the
primary parent or grant permissions. The inspector shows canonical membership
records. Membership presence edits use the existing idempotent owner contract.

In Projects, select a final task and press `W` to draft its prerequisite chain
as a reusable template. Press `w` to browse workflows, select a template and
press `E` to edit it. Native fields cover name, step keys, assigned agents,
instructions, success criteria and prerequisite keys. `F7` adds a draft step;
`F8` removes the focused step, retaining at least one. Templates contain at most
32 steps. The store validates the whole graph and captured revision atomically;
saving content starts no work and does not change existing run snapshots.

Select a template and press `s` to draft separate scopes for every step. Each
has its own input/immutable/output paths, checks, model, requests, tokens,
time/attempt limits, approval policy and allowed delegates. `F6` loads the
canonical preview; `y` confirms that frozen preview. Changes to template,
project, nodes or policy invalidate it. Limits are fixed per step; their sum is
not a reusable pooled budget. Prerequisites need exact acceptance before later
steps can dispatch. Acceptance and Apply remain separate owner decisions.

All forms use Tab/Enter, Shift-Tab, arrows for choices and Alt-Enter for newlines.
`F6` reviews the exact payload; Escape cancels confirmation or closes the draft.
Drafts and confirmations do not follow subsequent selection/background changes.
No new dependency, workflow language or background service is added.

## Project lifecycle and organization execution controls

Approved next-twenty items 6–9 add concrete separately versioned controls through
existing local-owner definition RPCs. No new dependency, worker engine or policy
DSL is introduced. Current controls and immutable versions live in protected
SQLite; `organization list controls` / `organization list controlHistory` inspect
them. Revisions are independent of project/task definition and execution-grant
revisions. Changing pause/priority never resets usage, extends deadlines or
reauthorizes uncertain work.

F10 opens organization execution/lifetime reported-token, admitted-request and
active ceilings. All three blank removes optional global ceilings by an exact
owner decision; the native default-two/max-four runtime envelope remains.
F12 previews a version-pinned emergency pause/resume; y confirms. Pausing stops
new inference/tool/coordination admission and automatic occurrences. Existing
admitted receipts can finish; quiescent attempts release active capacity. Global
usage sums root grant ledgers once, including revoked/historical grants and
handoffs. Descendants are counted once. Unknown terminal requests fence the
configured pool; late exact receipts can clear uncertainty without replay. Token
ceilings constrain recorded usage at admission; already admitted inference may
overshoot and is charged. Lowering active capacity below current use refuses.

F opens a selected project/task's lifecycle form. Active projects can pause or
complete; pause/completion precedes archive; resume/reopen returns to active.
Completed/archived definitions cannot be edited until reopen. Completion requires
no unfinished tasks, an exact currently accepted same-project result and an
owner explanation for every criterion. This is evidence-backed owner assessment;
model claims do not complete projects. Completion proofs remain in immutable
history after archive/reopen, along with original results, grants and accounting.
Paused/closed projects are filtered before the scheduler's bounded candidate page.

! opens a task's -10…10 priority form, bound to its current task/control revisions.
Among currently eligible tasks, scheduling adds one per queued minute from the
later of first authorization and declared start time, then chooses deterministic
age/ID ties. Waiting time includes paused/dependency waiting; it does not grant
eligibility. Child work retains the existing reservation precedence so waiting
roots cannot starve their descendants. Priority changes preserve the original
queue timestamp and grant. Old queued roots backfill their admission time from
native audit; missing historical times sort as oldest, without invented metrics.
F11 browses current controls; the palette exposes every action. Existing K pane
navigation and Q actual-Neovim selection behavior are preserved.

Full state 301, protocol 373 and targeted TUI/actual-Neovim 62 checks pass,
along with four generated schema comparisons and 25 Python SDK contract checks.
Older daemons that omit global usage show unavailable rather than an invented
zero. Production CLI Clippy with warnings denied, CLI build and formatting pass.
Native qualification passes on `753464cd53`: actual global ceilings/priorities,
pause/refused approval/same-grant resume, coding/document subprocess checks and
acceptance, completion/archive/reopen, exhausted global admission refusal,
36 layout switches, restart and two native exits 0. Five installed asset hashes
and SQLite integrity pass. Four localhost responses/60 fixture tokens/zero paid;
final global state is paused and owned services are stopped. Two accepted results
remain unapplied for later approved hunk/editor qualification. Preserve
`operations-batch/controls/`; never replay its setup or decisions.

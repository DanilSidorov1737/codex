# Operations and workflow owner controls

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

## Project lifecycle and organization execution controls (in progress)

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
Native qualification remains in progress; do not present this slice as delivered
before physical checks.

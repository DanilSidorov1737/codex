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

# Strategic native Owner overview

Approved daily-use tasks25–26/29–30 reuse existing mission, report, escalation,
task/dependency and conversation records. The overview fits the existing five
layouts. It introduces no required executive role or hierarchy level.

Use Ctrl-P → **Owner overview / strategic brief**. The captured view shows the
versioned mission, canonical task/check outcomes, current owner-decision queue,
unresolved escalation responsibility/revisions, recorded usage, and linked
mission/goal/project/task/grant/shared-root/result evidence. `execution-read`
references lead to actual actions, approvals, checks and review. Dependency advice
names prerequisites with unavailable or unaccepted evidence. No completion-time
forecast is invented. Definitions may be partial; the brief labels its scope,
audit revision, unavailable evidence and bounded first32 entries. Reopen it after
refresh. Escape closes this read-only view before other commands.

An executive contact is optional. Select any enabled agent in Operations, then
Ctrl-P → **Choose selected agent as executive contact**. Review its exact identity
and confirm with `y`; Escape cancels. **Talk to executive contact** opens the
existing scoped conversation draft for that agent. Existing project/topic/prompt
review and explicit execution authorization still apply. Selecting a contact
creates no role, task, grant, model request or management policy. The agent's
actual responsibility remains user-defined. A disabled/missing contact is shown
as unavailable, never silently replaced. **Clear executive contact / direct owner
control** restores direct use after the same local confirmation.

The contact lives in the private, atomically saved layout preferences and survives
TUI restarts. Save failures retain the prior selection and report failure. It is
a user interface preference, not backend executive authority or the deferred Dot
network API. Older packages may reject a preferences file containing this new
optional field; retain the matching old frontend settings for rollback. Omission
keeps the existing preferences format. Business state/grants remain untouched.

Current checkpoint:108 selected TUI tests pass, including actual official Neovim,
unchanged palette navigation, exact contact confirmation, existing conversation
draft routing, no backend mutation, save-failure preservation and unavailable/
stale/partial outcome evidence. Production CLI Clippy passes with warnings denied;
the clean c4f000e538 package and physical Owner walkthrough now pass. The actual
CLI shows the mission, selects a user-named agent, cancels its existing conversation
draft, remembers the contact after restart and clears it. Two normal CLI exits0,
daemon stop0, unchanged canonical roles/tasks/controls and zero inference. Evidence:
`daily-use-batch/native-owner-overview/continuation-result.json`. The first driver
selected the agent's group; actual Down navigation selected the correct agent
without changing any definition. Never describe this slice as all40 tasks complete.

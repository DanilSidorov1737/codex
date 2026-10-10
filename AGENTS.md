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
this scope/evidence current in status.md; later memory/scheduling/selective-Apply
work requires its matching scope.
The owner approved the next slice with “continue and begin”: whole-file selective
Apply in Review, using Ponytail and periodic commits. Preserve exact acceptance,
owner confirmation, base/artifact checks, native dirty-buffer protection and the
one-application-per-result recovery rule. Memory retrieval/schedules remain later scope.
Record new approvals and scope in status.md; a plan alone does not authorize work.

Read [docs/development/README.md](docs/development/README.md) and
[status.md](docs/development/status.md) at the start of every session.
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

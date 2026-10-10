# Development process and continuous documentation

## Session protocol

Read root AGENTS.md, status, decisions, MVP and the touched source/scoped AGENTS.md.
Check Git status first. Identify approved phase, current commit, existing changes,
test environment and outstanding blockers. Do not restart completed work or infer
implementation approval from the existence of this plan.

At the end of every meaningful change record:

- What was implemented or investigated, and why.
- Which existing code/contracts were reused and which tradeoffs were chosen.
- Exact meaningful tests actually run, outcomes, evidence and limitations.
- Known failures, deferred scope and next concrete action.
- Updated status/roadmap/decisions when the plan changes.

Document decisions with context, proposed/accepted/superseded status, alternatives,
reason, affected requirements, risks and verification. Keep current guidance near
the implementation; avoid an unmaintained duplicate design document per session.

## Implementation loop after approval

1. Trace existing callers and real control/data flow; apply Ponytail's reuse ladder.
2. Select the smallest coherent authorized change and meaningful failure-sensitive tests.
3. Implement with validation/error handling/security/recovery intact.
4. Run relevant checks and physically use the app/TUI/actual Neovim when affected.
5. Diagnose failures, fix them within approved scope and rerun. Never disable
   assertions/checks or weaken verification merely to claim success.
6. Review correctness/security/integration as well as Ponytail complexity.
7. Update docs and make a logical commit with a specific message.

Use repository-pinned tools and frozen lockfiles. Proposed future dependencies
need justification, maintenance/license review and agreed scope. Avoid rewrites,
generic framework layers or refactors unrelated to the feature.

## Commit and review policy

Make frequent, logical Git commits that include related implementation/tests/docs;
a single coherent Phase 0 documentation commit is appropriate.
Stage explicit paths; inspect staged content and user changes before committing.
Never commit secrets, authentication files, generated logs or unrelated user work.
Do not overwrite another contributor's work or reset the checkout to clear failures.

A local commit is different from push, merge, publish and deployment. Those actions
need their own authorization. Phase 0 permits documentation-only commits; it does
not permit implementation commits or installing tools.

Use PR descriptions to explain the concrete behavior, reason and verified evidence.
“Tests pass” without commands/counts/interactive limits is inadequate.
If checks cannot run, record a precise blocker and unrun status.

## Simplicity with non-negotiable safeguards

Do not create services for every capability family, always-running shared supervisors,
a custom editor, a general workflow DSL or distributed infrastructure in the first MVP.
Do not simplify away security enforcement, dirty-buffer protection, durable approvals,
budget reservations, failure handling, accessibility or user-requested real tests.

Discussion diagrams and role labels are configurable examples. Architecture docs
describe proposals until accepted; progress docs describe only delivered behavior.
Do not confuse those categories.

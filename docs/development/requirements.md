# Inputs, requirements, and visual review

## Reviewed sources

The original checkout was `751e7edd0b2012ca831c7fdcdd3400631e4cfe14`.
Read-only remote inspection found a documentation-only commit on `origin/main`;
the clean checkout was fast-forwarded to
`8026444c98cd9c6db9e7c953b44a20ea4f93c7b6`. Application sources did not change.

The user's Phase 0 prompt and [Codex CLI Fork.txt](../Codex%20CLI%20Fork.txt)
are the available written requirements. The text was read through its final
layout notes. All seven images below were opened and visually inspected.
There is no PDF in this checkout or the inspected shared input directories.
Do not claim to have reviewed a PDF; reconcile any additional PDF before
finalizing scope if the owner supplies one.

| Input                                                                  | Observed intent and design consequence                                                                                                                                                                                |
| ---------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Console](../01_Console.png)                                           | Conversation and command composer, project/agent navigation, execution progress and test output, task inspector, artifact links. Reuse Codex chat and stream handling; statuses must derive from runtime evidence.    |
| [Workspace / Neovim](../02_Workspace_Neovim.png)                       | Real modal editor, file explorer, tabs, terminal/tests/diffs, agent collaboration inspector. Follow AI, Live Diff, and Independent Editing are distinct modes; selected-code requests and focus ownership are needed. |
| [Projects](../03_Projects.png)                                         | Goals, project plan, milestones, dependencies, assignees, labels, deliverables, activity, inspector. Support non-code projects; compute progress from accepted work and explicit criteria.                            |
| [Updated Operations tree](../04_Operations_Hierarchy_Tree_UPDATED.png) | Interactive organization tree is the PRIMARY Operations view; search, filter, expand/collapse, node inspector, control actions, health/schedule/events. Tree structures and labels must be configurable.              |
| [Review](../05_Review.png)                                             | Diff/files/tests/audit views, evidence, approval/reject/request changes/escalate, human-action queue. Approving a result, applying a patch, and merging are separate commands.                                        |
| [Earlier Operations concept](../concept_img_4.png)                     | Task table and operational metrics. Retain task-list access as a secondary view, rather than making it replace the updated tree.                                                                                      |
| [Example hierarchy](../example_hierarchy.png)                          | Human owner/interface, central intelligence, directors, managers, workers, temporary agents, projects, shared functions. These are optional examples, not required levels or system roles.                            |

The images depict large graphical layouts. Translate information hierarchy and
interaction into terminal cells; do not promise exact fonts, shadows, pixel art,
or icon availability. Their names, dates, percentages, dollar amounts, test
results, CPU values, and deployment examples are illustrative data.

The Console mockup simultaneously shows a completed inspector and a running
test step. Resolve this in implementation by using one canonical task state and
displaying timestamps, not by preserving inconsistent example labels.

The hierarchy image places projects in the organization tree. The canonical
project/goal/task model remains separate; tree projection can include linked
project rows without treating projects as executing agents.

## Requirement map

| ID  | Requirement                                                                 | Proposed delivery                                                                                                                  |
| --- | --------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| R01 | Single agent through user-defined organizations; no predefined levels/roles | MVP persistent forest, arbitrary role templates, groups and memberships; later capacity qualification                              |
| R02 | Autonomous goals, projects, tasks, delegation, coordination                 | MVP constrained goal-to-task proposal, ownership and dependency execution; later reusable workflows                                |
| R03 | Background execution, persistence, memory and restart recovery              | MVP daemon ownership, durable tasks and scoped notes, reconciliation                                                               |
| R04 | Tools, integrations, multi-model support, resource management               | Reuse existing execution/MCP/plugins/provider configuration; MVP one verified provider; later broaden compatibility and accounting |
| R05 | Security, approvals, auditing, budgets, recovery                            | MVP enforced grants, limits, review and crash tests; progressive external authority                                                |
| R06 | Five customizable TUI layouts                                               | MVP minimal useful Console, Workspace, Projects, Operations, Review; presets and bounded pane customization                        |
| R07 | Interactive hierarchy tree as primary Operations view                       | MVP keyboard navigation, search, collapse/expand, inspector and controls                                                           |
| R08 | Actual official Neovim, coding, follow activity, diffs and collaboration    | Authorized editor spike, then MVP real editor and safe follow/diff; later controlled shared-buffer collaboration                   |
| R09 | Simplicity and continuous Ponytail implementation/review                    | Already available session skill; explicit future setup and pinned-version review                                                   |
| R10 | Meaningful tests plus real application/TUI/editor interaction               | Mandatory future acceptance evidence; Phase 0 baseline is limited                                                                  |
| R11 | Continuous docs, logical commits, user-change/secret protection             | Root AGENTS.md and development process                                                                                             |
| R12 | Stop for approval after planning; no Phase 0 feature implementation/install | Current gate                                                                                                                       |

## Capability inventory coverage

The write-up describes eight shared functions and 38 capability families.
It explicitly says these do not require separate subsystems. Apply that guidance:

| Families                                                                               | Architectural home and sequence                                                                                            |
| -------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| 1–4: missions, strategy, discovery, project lifecycle                                  | Goal/project/task records and bounded planning; MVP core, later proactive discovery                                        |
| 5–8: organization, agent lifecycle, delegation, communication                          | Durable definitions and memberships, execution adapter, inbox/board; MVP core, later meetings/calendar                     |
| 9–12: workflows, unattended execution, continuity, supervision                         | Scheduler, dependencies, leases, reconciliation and watchdogs; MVP simple flow, later recurring templates                  |
| 13–16: state, memory, artifacts, audit                                                 | Authoritative SQLite, scoped knowledge references, artifact/evidence records; MVP minimal records, later quality/promotion |
| 17–23: research, acquisition, tools, computer control, multimodal, accounts, workforce | Existing tools/MCP/plugins plus narrowly authorized integrations; research/local artifacts first, external actions later   |
| 24–29: resources, human oversight, security, governance, side effects, audit           | Deterministic policy/action gates and review records; required MVP boundaries, financial transactions deferred             |
| 30–31: reliability, backup, incident response                                          | Daemon supervision, safe restart, verified SQLite backups, recovery mode; MVP crash/restore, later operational automation  |
| 32–35: learning, self-modification, multi-model routing, evaluation                    | Evidence-backed configuration proposals and provider adapters; no autonomous modification of protected controls            |
| 36–38: visibility, efficiency/scale, readiness                                         | Five views, bounded residency/concurrency, measurements and phased real tests                                              |

Shared functions—resource/finance, organization/HR, audit/risk, memory/knowledge,
orchestration/improvement, infrastructure, security/governance, procurement—are
optional user-configurable agents or workflows backed by deterministic services.
Do not start eight always-running supervisors or let a “Security” agent grant
itself authority. Human administrative control exists independently of any
optional “CEO” agent.

## Boundary interpretations

- Organization depth is independent of execution concurrency and role names.
  Configurable protective limits must not encode mandatory levels.
- An idle persistent role consumes storage, not an always-running model session.
- Same-project collaboration must preserve dirty buffers and user files.
  Concurrent same-buffer OT/CRDT editing is a later capability, not an MVP claim.
- External accounts, publishing, procurement, finances and physical actions need
  explicit delegated authority and are outside initial local-only execution.
- Continuous availability requires a running host and process supervisor;
  closing a TUI is distinct from a suspended cloud machine.

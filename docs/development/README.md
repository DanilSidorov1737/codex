# Codex CLI fork development

Phase: **B1–B6 implemented/qualified, including bounded real GPT-5.5 pilots**.
The owner-approved TUI creation/authorization and concept-art dashboard usability
slice is also qualified on application source `46d999bf5b`; see status.md and layouts.md.
The subsequent whole-file selective Apply slice is qualified on `9fadf32797`, with
319 automated tests and actual CLI/official-Neovim evidence; see selective-apply.md.
The owner-approved N1 memory search slice is qualified on `d6d049cc49`, with
691 distinct automated tests, real daemon restart/isolation and native CLI/TUI/
Neovim interaction. N2 recurring schedules is qualified on `e06def499e`: 700
automated tests, real background/approval/pause/restart/resume/cancel/budget/crash
and native CLI/TUI/official-Neovim checks. See recurring-schedules.md and status.md;
N3 reusable workflow templates is approved and undergoing final native qualification;
see [workflow contract](workflows.md). N4–N8 remain proposed later scope.
Phase 0 repository baseline: `8026444c98cd9c6db9e7c953b44a20ea4f93c7b6`, from `origin/main`.
Research recorded on 2026-10-09 (America/New_York).

The objective is a general-purpose autonomous system that extends Codex CLI.
A user may run one agent or define an organization with thousands of agents.
Names, roles, levels, teams, and supervision are user choices. The example
organization is a template, not the product's fixed structure.

## Start here

1. Read root [AGENTS.md](../../AGENTS.md).
2. Read [status and resumption](status.md) and [requirements and visual review](requirements.md).
3. Read [repository analysis](repository-analysis.md), [architecture](architecture.md),
   and [data model](data-model.md).
4. Check [decisions and risks](decisions-and-risks.md) against current approvals
   and remaining release gates in status.md.
5. Use [roadmap](roadmap.md) and [detailed MVP plan](mvp.md) for approved work.
6. Follow [process](process.md) and [testing](testing.md); update this documentation
   with every meaningful implementation or scope change.

B1 usage: [organization command guide](organization-commands.md).
B3 contracts and bounds: [coordination](coordination.md).
B4 layout usage and qualification: [layouts](layouts.md).
Current recurring screenshots: `/workspace/artifacts/cli-recurring-schedules/cli-recurring-schedules-screenshots.zip`.
Historical memory screenshots: `/workspace/artifacts/cli-memory-search/cli-memory-search-screenshots.zip`.
Selective Apply captures: `/workspace/artifacts/cli-selective-apply/cli-selective-apply-screenshots.zip`.
Previous UI captures: `/workspace/artifacts/cli-ui-update/cli-ui-update-screenshots.zip`.
B5 actual editor and application: [Neovim Workspace](neovim-workspace.md).
Approved follow-up: [whole-file selective Apply](selective-apply.md).
Owner-approved N1: [scoped persistent memory search](memory-search.md).
Owner-approved N2: [bounded recurring schedules](recurring-schedules.md).
Owner-approved N3: [reusable bounded workflows](workflows.md).
Execution settings/prices: [resource limits](resource-limits.md).
Release evidence/gates: [qualification](release-qualification.md).

Other focused plans: [security and recovery](security-and-recovery.md),
[TUI and Neovim](tui-and-neovim.md), [Ponytail setup](ponytail.md).
Proposed follow-up order: [recommended next slices](roadmap.md#recommended-next-slices).
Current foundation evidence: [CLI, daemon and real Neovim qualification](qualification.md).

## Recommended direction

Extend the existing Rust app-server daemon. Keep orchestration independent of
the TUI and editor. Add durable agent definitions and task scheduling around the
existing Codex execution, coordination, and tool infrastructure.

Use SQLite/SQLx, Tokio, typed app-server JSON-RPC, Ratatui/Crossterm, and existing
PTY/Git/MCP infrastructure. Separate authoritative organization state from
Codex's rebuildable runtime metadata. Embed **actual official Neovim** through its
MessagePack external UI API. Do not implement an editor engine.

First MVP: a single-host, owner-controlled system with a small bounded number
of active executions, arbitrary persistent organization structure, one useful
project workflow, all five minimal layouts, safe review, and real Neovim.
Local measurements cover 1,000 additional stored agents and two/four active
native executions. Larger registries and greater simultaneous execution remain
later capacity targets; see release-qualification.md for measured results.

## Evidence and limitations

Historical Phase 0 checks: 17 file-search Rust tests, 80 repository script tests,
and 14 SDK execution unit tests passed; a real file-search CLI invocation found
`Cargo.toml`. These checks establish a narrow development baseline.

No new application features, editor integration, installations, full CLI build,
TUI interaction, live model calls, or scale/recovery validation occurred in
Phase 0. Architecture and acceptance criteria were proposals at that checkpoint;
current implementation and qualification are recorded in status.md and the release
matrix. The separate requirements PDF mentioned in the prompt is absent; the
714-line text file and all seven supplied images were reviewed.

Subsequent owner-approved M0/M1 checks built the full CLI/package, exercised the
actual TUI/daemon and installed/tested official Neovim. See qualification for
evidence and limits. Subsequent B1–B6 work delivered and locally qualified the
integrated MVP. The bounded live-provider gate subsequently passed real single-agent
coding, one-child delegation, local-fact documents and actual review/Apply. See
the release matrix for evidence, the $0.345433 rate estimate and operating limits.

The owner approved the [MVP build plan](mvp.md#build-order-and-usable-checkpoints),
Ponytail implementation and periodic local commits. Follow the recorded scope and
evidence gates; do not infer external operational or spending authority.

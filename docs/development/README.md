# Codex CLI fork development

Phase: **approved MVP implementation; B1–B4 locally qualified; B5 Neovim underway**.
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
4. Before implementation, resolve the approval gate in [decisions and risks](decisions-and-risks.md).
5. Use [roadmap](roadmap.md) and [detailed MVP plan](mvp.md) for approved work.
6. Follow [process](process.md) and [testing](testing.md); update this documentation
   with every meaningful implementation or scope change.

B1 usage: [organization command guide](organization-commands.md).
B3 contracts and bounds: [coordination](coordination.md).
B4 layout usage and qualification: [layouts](layouts.md).
B5 actual editor and application: [Neovim Workspace](neovim-workspace.md).

Other focused plans: [security and recovery](security-and-recovery.md),
[TUI and Neovim](tui-and-neovim.md), [Ponytail setup](ponytail.md).
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
Thousands of registered agents and thousands of concurrent agents are separate
capacity goals. Neither is a demonstrated capability today.

## Evidence and limitations

Current Phase 0 checks: 17 file-search Rust tests, 80 repository script tests,
and 14 SDK execution unit tests passed; a real file-search CLI invocation found
`Cargo.toml`. These checks establish a narrow development baseline.

No new application features, editor integration, installations, full CLI build,
TUI interaction, live model calls, or scale/recovery validation occurred in
Phase 0. Architecture and acceptance criteria below are proposals, not implemented
behavior. The separate requirements PDF mentioned in the prompt is absent; the
714-line text file and all seven supplied images were reviewed.

Subsequent owner-approved M0/M1 checks built the full CLI/package, exercised the
actual TUI/daemon and installed/tested official Neovim. See qualification for
evidence and limits; these are foundations, not the MVP's integrated features.

The owner approved the [MVP build plan](mvp.md#build-order-and-usable-checkpoints),
Ponytail implementation and periodic local commits. Follow the recorded scope and
evidence gates; do not infer external operational or spending authority.

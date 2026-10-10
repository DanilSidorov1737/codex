# Repository analysis and reusable components

Inspected baseline: `8026444c98cd9c6db9e7c953b44a20ea4f93c7b6`.
This is a source-backed architecture review, not an exhaustive security audit.
The Rust workspace has approximately 161 Cargo manifests across components.
A complete build was not necessary to edit documentation and was not attempted.

## Existing foundations

| Component / source                                                                                                                                                   | What exists                                                                                | Reuse and limitation                                                                                                                               |
| -------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| [CLI entry](../../codex-rs/cli/src/main.rs)                                                                                                                          | Interactive CLI, execution, app-server/daemon, plugin commands                             | Extend command routing; retain ordinary Codex use                                                                                                  |
| [AgentControl](../../codex-rs/core/src/agent/api.rs), [local adapter](../../codex-rs/core/src/agent/control/api.rs)                                                  | Spawn, delivery, interruption, listing and coordination contract                           | Existing execution seam; Rust trait is not a wire protocol or durable organization registry                                                        |
| [Registry](../../codex-rs/core/src/agent/registry.rs), [types](../../codex-rs/core/src/agent/types.rs)                                                               | Agent paths, metadata, thread identities and reservations                                  | Useful transient execution identity; registry uses in-memory maps                                                                                  |
| [Roles](../../codex-rs/core/src/agent/role.rs), [role loader](../../codex-rs/agent-roles/src/loader.rs)                                                              | User roles/configuration layered over parent authority, plus defaults                      | Reuse parser/config precedence; persist role versions outside individual thread lifetime                                                           |
| [Execution limits](../../codex-rs/core/src/agent/control/execution.rs), [residency](../../codex-rs/core/src/agent/control/residency.rs)                              | Active-turn capacity and unload/reload management                                          | Build organization scheduling around bounded sessions; do not instantiate every member at once                                                     |
| [Core configuration](../../codex-rs/core/src/config/mod.rs)                                                                                                          | Legacy default max threads 6 and depth 1; V2 default concurrency 4                         | These are runtime defaults, not organization-model constraints; verify feature/version semantics before change                                     |
| [Mailbox](../../codex-rs/core/src/agent/control/mailbox.rs), [handoff](../../codex-rs/core/src/agent/control/root_handoff.rs)                                        | Queue-only/trigger delivery, eviction retention and context selection                      | Mailbox is in-memory; handoff is explicitly best-effort without saved handoff state                                                                |
| [Message board](../../codex-rs/ext/agent-message-board/README.md), [local setup](../../codex-rs/ext/agent-message-board/src/local/setup.rs)                          | SQLite-backed channels/posts, atomic pinned templates, local/in-memory/host contracts      | Reuse discussion mechanism; current local template accepts channels/descriptions, not ACLs or roles                                                |
| [App-server protocol](../../codex-rs/app-server-protocol/src/protocol/common.rs), [TUI session](../../codex-rs/tui/src/app_server_session.rs)                        | Typed JSON-RPC and events; thread/turn lifecycle; clients already separated from server    | Add narrow fork protocol methods; do not invent a second transport                                                                                 |
| [Daemon](../../codex-rs/app-server-daemon/README.md)                                                                                                                 | Detached background process, control socket, serialized lifecycle and initialize readiness | Reuse process lifecycle; worker scheduling/reboot supervision still needs qualification                                                            |
| [Thread Store](../../codex-rs/thread-store/README.md)                                                                                                                | Canonical append interface, LiveThread, JSONL history and SQLite metadata                  | Keep history here; organization IDs and business state should not be inferred from transcripts                                                     |
| [SQLite infrastructure](../../codex-rs/state/src/sqlite.rs), [recovery](../../codex-rs/state/src/runtime/recovery.rs)                                                | SQLx pools/migrations and database-specific backup/rebuild                                 | Runtime default synchronization is Normal; corrupt runtime databases may rebuild. Authoritative state needs different durability/recovery settings |
| [Projects](../../codex-rs/state/src/runtime/projects.rs), [project protocol](../../codex-rs/app-server-protocol/src/protocol/v2/project.rs)                          | Project roots/metadata, ordering/recency, idempotent creation and thread associations      | Reuse identity and roots; APIs are experimental, not full project planning                                                                         |
| [Goals](../../codex-rs/ext/goal/src/extension.rs), [goal store](../../codex-rs/state/src/runtime/goals.rs)                                                           | Thread objective, budgets, accounting and continuation hooks                               | Reuse per-thread execution goals; not a durable cross-project goal/task graph                                                                      |
| [Extension API](../../codex-rs/ext/extension-api/src/contributors.rs), [ToolPolicy](../../codex-rs/ext/extension-api/src/tool_policy.rs)                             | Lifecycle contributors, captured tool-selection ceiling                                    | Prefer existing hooks; tool selection does not replace runtime authorization                                                                       |
| [Turn admission](../../codex-rs/ext/extension-api/src/turn_admission.rs)                                                                                             | Host shutdown-drain gate                                                                   | Explicit delegated/mailbox bypasses mean it alone cannot enforce all budgets or permissions                                                        |
| [Memory pipeline](../../codex-rs/memories/README.md)                                                                                                                 | Extraction/consolidation, leased jobs/backoff, citations, redaction and artifacts          | Reuse patterns; startup/root-session-oriented, not scoped organizational knowledge authorization                                                   |
| [Execution policy](../../codex-rs/execpolicy/README.md), [network proxy](../../codex-rs/network-proxy/README.md), [sandboxing](../../codex-rs/sandboxing/src/lib.rs) | Command decisions, sandbox/environment boundaries, network enforcement                     | Extend under protected ceilings; prompt instructions cannot enforce security                                                                       |
| [Provider info](../../codex-rs/model-provider-info/src/lib.rs)                                                                                                       | Built-in/custom providers, capabilities, authentication, local-provider entries            | Responses-compatible integration is available. Legacy chat wire API is rejected; arbitrary provider support needs testing                          |
| [TUI](../../codex-rs/tui/src/tui.rs), [multi-agent presentation](../../codex-rs/tui/src/multi_agents.rs), [diff rendering](../../codex-rs/tui/src/diff_render.rs)    | Ratatui/Crossterm, input/events, agent picker/history and diff display                     | Extend into shared panes/presets; organization tree and editor pane are new                                                                        |
| [External editor](../../codex-rs/tui/src/external_editor.rs)                                                                                                         | VISUAL/EDITOR subprocess and prompt editing                                                | Not a Neovim workspace integration; reuse terminal lifecycle lessons only                                                                          |
| [PTY utilities](../../codex-rs/utils/pty/README.md), [Git utilities](../../codex-rs/git-utils/README.md)                                                             | Subprocess IO/terminal sizing and Git primitives                                           | Reuse terminal pane and review operations; worktree/conflict policy still needs design                                                             |

## Existing dependencies and build workflow

Use Rust `1.95.0` from `codex-rs/rust-toolchain.toml`, including rustfmt/clippy/rust-src.
The root package pins pnpm `10.34.5`; Node must be at least 22.
Existing relevant dependencies include Tokio, SQLx SQLite, Serde/TOML/UUID,
Ratatui 0.30.2, Crossterm, portable-pty, vt100, tracing, and MCP libraries.
MessagePack/neovim RPC libraries are not in the inspected Cargo lockfile.

`justfile` provides Cargo, nextest and Bazel workflows. Use package-local Cargo
tests for a changed component and CI's required checks for the final change.
Code-mode, native dependencies and cross-platform targets can make full builds
costly; inspect actual dependencies and available resources before choosing
parallelism. Do not assume a feature toggle removes every native build input.

The checkout's two scoped AGENTS.md files were read and left intact.
No root AGENTS.md existed before Phase 0. Maintain scoped composer documentation
and the sandbox-integrity telemetry-only contract when future work touches them.

## Concrete gaps

1. Agent/role/group identity independent of threads, with configurable organization relationships.
2. Canonical task ownership, dependency graph, scheduling, attempts and safe crash reconciliation.
3. Durable action approvals, resource reservations, evidence-backed acceptance and replayable client updates.
4. Project planning beyond existing root/metadata containers.
5. Scoped knowledge access beyond existing root-session memory consolidation.
6. Five shared TUI layouts and an organization tree suited to large registries.
7. Real Neovim embedding and explicit human/agent editing conflict rules.
8. Qualification for unattended operation, scaling, provider compatibility and recovery.

Prefer one orchestration extension/service under app-server plus a protected
SQLite store. Avoid replacing AgentControl, ThreadStore, the tool system, or
the daemon before measured constraints justify doing so.

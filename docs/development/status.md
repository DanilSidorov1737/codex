# Current status and resumption instructions

Phase 0: **exploration and planning complete for available inputs**.
Next phase: **awaiting explicit owner approval**.
Application feature implementation: **not started**.

## Completed

- Read the initial request, full 714-line capability/layout/Neovim write-up and
  visually inspected all seven concept/hierarchy images.
- Found updated documentation on origin/main and fast-forwarded the clean checkout
  to documentation baseline `8026444c98cd9c6db9e7c953b44a20ea4f93c7b6`.
  No application sources changed.
- Traced agent coordination, roles/limits/residency/mailbox, daemon/client/protocol,
  thread/SQLite storage, projects/goals, memory, policy/tool/provider and TUI seams.
- Researched official Neovim external UI/API/startup and official Ponytail setup.
  Read upstream docs/manifests; did not install/execute third-party tools/hooks.
- Wrote architecture/data/security/recovery/layout/editor plans, requirement map,
  phased roadmap, detailed MVP criteria, process/testing docs and root AGENTS.md.
- Ran existing tests: 17 Rust, 80 Python and 14 TypeScript unit tests passed.
  Invoked real existing file-search CLI in a PTY; Cargo.toml match verified.
- Checked documentation formatting, links, allowed change scope and whitespace
  before the final report.

## Known limitations / remaining work

- Separate PDF is absent; compare it if provided.
- Full Codex CLI binary/TUI remains unbuilt/unrun; this baseline does not establish
  main application readiness.
- Actual Neovim is absent and no editor behavior was tested.
- Upstream standalone Ponytail plugin installation/hooks are unverified;
  current session skill availability is separate.
- No live provider request, daemon recovery, protected IPC, full integration suite
  or scale test occurred.
- Earlier full SDK integration run failed due to missing CLI binary, as documented
  in [testing](testing.md).
- All architecture decisions except Phase 0 documentation/process rules remain
  proposed. No schemas/features/dependency changes are implemented.

## Resume safely

1. Read root AGENTS.md and this file, then decisions, MVP and testing.
2. Check Git status/log and preserve existing changes; use the existing isolated checkout.
3. Verify the owner's latest explicit approval and constraints. Without approval,
   answer planning questions or revise documentation only.
4. If implementation and required installations are approved, start M0/M1:
   qualify the full CLI and daemon, actual official Neovim, policy/IPC and storage.
   Do not jump straight to a large organization/agent framework.
5. Recheck runtime/environment readiness and existing credential bindings without
   printing values. Use established proxy/TLS/auth routes.
6. Follow small implementation/test/review/doc/commit steps and update status with
   actual delivered behavior, evidence, failures and next action.

A local documentation commit does not authorize pushing, merging, deploying or
publishing. Never treat this status file as proof that future features work.

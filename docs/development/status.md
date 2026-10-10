# Current status and resumption instructions

Phase 0: **exploration and planning complete for available inputs**.
Current work: **approved MVP implementation; B1/M2 store tested, M3 definitions next**.
Application feature implementation: **protected store implemented; B1 not yet complete**.

## Current authorization

The owner's “Okay, let's do this” approved the described next step: build and
physically exercise the real CLI, check detached daemon/reconnect behavior, obtain
verified official Neovim and test its embedding API. Required local prerequisites
are included. No push, publication, external spending or broader operational
authority is inferred. This qualification is complete. The owner now requests
“Okay, let's implement this. Use Ponytail to implement, and then commit
periodically, follow the MVP plan”. This approves B1–B6's ordinary implementation,
tests, required local dependencies, documentation and local commits. It does not
authorize push/publication, outbound communications or unbounded provider spending.

## B1 implementation progress

- Added `OrganizationStore` to existing codex-state, reusing SQLx and the existing
  cross-platform private-directory helper. No new external dependency was added.
- Canonical database lives at `<sqlite_home>/organization/organization.sqlite`,
  inside an owner-only directory. Linux files are mode 0600; symlinks/hard links,
  unsafe ownership/permissions, incompatible migrations and corruption are rejected.
- Uses WAL, FULL synchronization, foreign keys and one connection for short
  transactions. No runtime backup-and-rebuild fallback applies to this store.
- Added verified consistent snapshots and restoration into a fresh directory.
  Missing/truncated initialized state is rejected rather than reset automatically.
- All 233 codex-state library tests passed, including five new meaningful store
  tests. Clippy with warnings denied and package formatting checks passed.
- This is a storage library slice; no new CLI/RPC command or background organization
  task is delivered yet. Real command/TUI interaction follows when M3 exposes it.
- Next: durable arbitrary nodes/role versions, goals, existing-project links and
  proposed tasks, with revision/cycle checks, typed RPC and actual command checks.

## MVP planning completed before implementation

- Replaced stale build/editor prerequisites with the actual qualification results.
- Defined one complete goal -> authorized plan -> background work -> reviewed result
  workflow, covering standalone and custom multi-agent use, coding and documents.
- Organized the MVP into B1 durable definitions, B2 one safe background task,
  B3 bounded delegation/context, B4 five layouts, B5 actual Neovim Workspace and
  B6 live/interactive/recovery qualification. See [MVP](mvp.md).
- Clarified in-scope autonomy, checked internal task acceptance, owner final
  acceptance, separate safe patch application and initial operating limits.
- Product code was unchanged in that planning commit. Documentation verification is recorded in
  [testing](testing.md); foundation checks were not rerun for a planning-only change.

## Phase 0 completed

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

## Foundation qualification results

- Built real CLI `0.0.0` with Rust 1.95.0 and the existing `dev-small` profile.
  Built the real Code Mode host and vendored Bubblewrap, then assembled the
  existing complete package. No application source/manifests/lockfiles changed.
- Installed checksum-verified official Neovim v0.12.6. Twelve real embedding
  assertions passed on two corrected reruns; physically edited/saved/undid/diffed
  real files in a terminal.
- Twelve real daemon assertions passed on two runs: idempotent start, private IPC,
  detached turn completion, pending approval replay on reconnect, blocked tool
  before approval, actual artifact after approval, restart and completed history.
- Physically used both the embedded TUI and daemon-attached TUI. `/status` reported
  `Local background server`; its thread ID matched the daemon's loaded-thread API.
  Quit and resumed the same thread with response and token usage restored.
- Full SDK retest passed **49/49 tests, 5/5 suites**, including both MCP conformance
  gates. Earlier missing binary/helper failures are retained as historical results.
- See [qualification](qualification.md) for commands, provenance, failures and
  machine-local evidence. Eleven existing socket transport tests passed unchanged
  outside the outer sandbox; the sandboxed attempt had seven passes/four failures
  caused by its masked socket root. No assertions were skipped or disabled.
- Repeated the checksum-preserving setup successfully and saved complete install/
  startup instructions to the environment draft. Review/save and publication in
  environment settings remain owner actions; fresh-task restoration is unverified.
- Verified the fixture daemon survived TUI quit, then stopped only that daemon
  and the local provider fixture. No qualification service remains intentionally running.

## Known limitations / remaining work

- Separate PDF is absent; compare it if provided.
- Actual Neovim is qualified independently; the Rust Workspace pane is not built.
- Upstream standalone Ponytail plugin installation/hooks are unverified;
  current session skill availability is separate.
- No live provider request, comprehensive crash/approval recovery, agent-scoped IPC
  denial, all-Rust/Bazel suite or scale test occurred. Fixture Responses are
  deterministic local responses, not a live model.
- The outer cloud sandbox hides daemon sockets. The isolated daemon and attached
  TUI checks ran with authorized unsandboxed execution; the restriction was not
  weakened. Future daemon checks need that execution mode.
- All architecture decisions except Phase 0 documentation/process rules remain
  proposed. No schemas/features/dependency changes are implemented.

## Resume safely

1. Read root AGENTS.md and this file, then decisions, MVP and testing.
2. Check Git status/log and preserve existing changes; use the existing isolated checkout.
3. Verify the owner's latest explicit approval and constraints. Without approval,
   answer planning questions or revise documentation only.
4. Review qualification evidence and remaining gates rather than repeating the
   baseline. The next proposed product slice is M2's protected organization store
   and recovery checks, followed by B1's definitions/goal/project operations.
   Full Neovim pane integration remains B5/M8. Keep broader
   feature scope and live-provider budget explicit.
5. Recheck runtime/environment readiness and existing credential bindings without
   printing values. Use established proxy/TLS/auth routes.
6. Follow small implementation/test/review/doc/commit steps and update status with
   actual delivered behavior, evidence, failures and next action.

A local documentation commit does not authorize pushing, merging, deploying or
publishing. Never treat this status file as proof that future features work.

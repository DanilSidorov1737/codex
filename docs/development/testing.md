# Testing procedures and evidence

## B1/M2 store verification

On 2026-10-10 UTC, Linux x86_64, Rust 1.95.0, from `codex-rs` after documented
activation:

- `cargo test --locked --offline --profile dev-small -p codex-state --lib -j 4`:
  **233 passed, zero failed/ignored**. Log:
  `/workspace/.onboarding/qualification/mvp-state-tests.log`.
- Five new store tests exercise actual SQLite WAL/FULL durability settings,
  stable identity after reopen, consistent backup/fresh restore, corruption,
  truncated/missing initialized data, migration checksum mismatch, real SQLITE_FULL
  rollback and unsafe directory/database symlink permissions. No storage mocks.
- `cargo clippy --locked --offline --profile dev-small -p codex-state --lib -j 4 -- -D warnings`:
  exit 0. `cargo fmt -p codex-state -- --check`: exit 0.
- Initial compilation caught SQLx 0.9's dynamic-SQL guard in the disk-full test;
  the integer-only PRAGMA assignment is explicitly audited using existing
  `AssertSqlSafe`, then checks were rerun. Full-repository formatting initially
  reported the new unformatted files; package formatting was fixed and rechecked.

This qualifies the storage slice, not the complete B1, agent sandbox exclusion,
or MVP. No new commands exist yet to physically exercise; M3 must add and test
them. No live provider request or broad Bazel/platform suite was run.

## MVP planning verification

Planning-only update on 2026-10-10 UTC: no application source, dependency or
lockfile changed. Checked the revised scope against recorded foundation evidence;
pending approval replay on reconnect is not presented as approval crash recovery.
No application tests or live requests were rerun for this documentation change.

- From the repository root, activate the documented environment and run
  `pnpm exec prettier --check AGENTS.md 'docs/development/*.md'`: exit 0.
- Local Markdown path/anchor validation: 16 files checked; 85 local links/anchors
  valid after the architecture qualification link was added.
- `git diff --check`: exit 0. Reviewed changed paths/diffs for documentation-only
  scope; MVP A01–A21 remain future product acceptance tests, not passed results.

## Current M0/M1 evidence

Owner-authorized foundation work is recorded in [qualification](qualification.md).
Official Neovim v0.12.6 passed twelve real embedding assertions on the corrected
helper's two reruns and was physically edited/saved in a PTY. These checks verify
editor APIs; no Workspace pane or live model behavior is claimed.
Real CLI/package build and two twelve-assertion daemon runs passed. Full SDK retest:
49 passed, zero failed, five suites including MCP conformance. The actual attached
TUI accepted input and resumed its saved thread. The daemon requires authorized
unsandboxed execution in this cloud machine because the outer sandbox hides its
socket directory; see qualification for the diagnosis and limits.
Existing Unix socket transport tests passed 11/11 unchanged outside that sandbox;
the masked-root attempt had seven passes/four environment-related failures.

## Phase 0 results

Baseline `8026444c98cd9c6db9e7c953b44a20ea4f93c7b6`; Linux x86_64 cloud host.
Rust 1.95.0; Node 24.19.0; pnpm 10.34.5.
No application source changed. Existing tests were used; no new feature tests
were written because implementation is not authorized.

| Command / working directory                                                                                            | Result                                              | What this verifies                                                                             |
| ---------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| `cargo test --locked -p codex-file-search -j 4`, codex-rs                                                              | 17 passed, 0 failed/ignored; binary harness 0 tests | Real Rust dependencies and file-search library behavior                                        |
| `python3 -m unittest discover -s .github/scripts -p 'test_*.py'`, repository root                                      | 80 passed                                           | Repository maintenance/release helper tests; test messages contain expected simulated failures |
| `pnpm --filter @openai/codex-sdk test --runInBand tests/exec.test.ts`, repository root                                 | 1 suite, 14 passed                                  | SDK execution unit contracts; subprocesses are mocked, not a live model/CLI validation         |
| `cargo run --locked -p codex-file-search -j 4 -- --json -C /workspace/codex/codex-rs/file-search Cargo.toml`, codex-rs | Exit 0, actual JSON match for Cargo.toml            | Real existing CLI invoked in a PTY; not the main Codex TUI                                     |
| New Markdown formatting, local links, whitespace and allowed-diff check                                                | Completed before final Phase 0 report               | Documentation integrity; not application readiness                                             |

Machine-local evidence is under `/workspace/.onboarding/phase0/`:
`file-search-tests.log`, `script-tests.log`, `sdk-exec-tests.log`,
`file-search-smoke.jsonl`, and final documentation checks. Logs are generated
outputs, not committed secrets or application files. Counts are summarized here
for sessions where the machine-local logs are unavailable.

Earlier onboarding built the TypeScript SDK and installed frozen dependencies.
Its full SDK suite had 18 passes and 31 failures because the full Codex binary
was missing. Phase 0 did not rerun that integration suite or call it passing.
The narrower 14-test unit run above is distinct.

Phase 0 did not build/run the full Codex CLI/TUI, install/run Neovim, invoke a
live model, trust/run upstream Ponytail hooks, run all Rust/Bazel suites, exercise
daemon crash/restore, or test 1,000 agents. Those remain implementation gates.

## Existing environment activation

From this prepared cloud environment, initialize each shell:

```bash
export CARGO_HOME=/workspace/.onboarding/cargo
export RUSTUP_HOME=/workspace/.onboarding/rustup
export npm_config_cache=/workspace/.onboarding/npm-cache
export XDG_DATA_HOME=/workspace/.onboarding/data
export XDG_CACHE_HOME=/workspace/.onboarding/cache
export XDG_STATE_HOME=/workspace/.onboarding/state
export PATH="/workspace/.onboarding/cargo/bin:/workspace/.onboarding/pnpm/node_modules/.bin:$PATH"
export PATH="/workspace/.onboarding/neovim/v0.12.6/nvim-linux-x86_64/bin:$PATH"
```

Use existing checkouts. No worktree creation is needed for environment setup.
Do not dump credential/environment values. Frozen installation is
`pnpm install --frozen-lockfile --store-dir /workspace/.onboarding/pnpm-store`;
Phase 0 used already installed dependencies and did not reinstall.

## Future validation layers

1. Unit/state-machine tests: task transitions, cycles, expected revisions,
   permission/grant composition, reservation arithmetic, revoked credentials,
   dirty-base/changedtick conflicts and Unicode indexing.
2. Storage/protocol integration: actual SQLite, transactions, migration/restore,
   duplicate inbox/outbox delivery, pagination/replay, schema compatibility and
   approval reissuance. Do not mock away durability or authorization boundaries.
3. Runtime integration: real app-server/daemon, existing execution adapters,
   sandbox/tool limits and fixture project tasks; inject interruptions at meaningful
   start/dispatch/persist checkpoints.
4. Real terminal/editor: actually launch the built application in a PTY/terminal,
   send keyboard commands, navigate views, change pane sizes, submit work,
   decide an approval, inspect results, and quit/reconnect. Then perform actual
   Neovim edit/save/undo/diff/follow/conflict behavior.
5. Live bounded pilot: real authorized provider request and real local artifact,
   with spending/usage recorded. Fixture models alone do not prove provider behavior.
6. Qualification: declared registry/concurrency targets, backpressure, idle
   behavior, crash/restore and platform matrix.

Use existing Rust tests/nextest, TypeScript Jest, Python unittest, PTY utilities
and meaningful fault-injection fixtures. Add a terminal driver only if existing
tools cannot exercise needed inputs; do not adopt a large UI framework merely
for screen snapshots. Snapshot tests complement, not replace, real interaction.

Choose tests for affected behavior and repository CI requirements. Do not expand
into unrelated suites once evidence is sufficient. Run formatting in check mode
for source; avoid update-snapshot/codegen commands unless the approved change
requires them.

## Interactive test recipe after implementation

- Create a temporary fixture project/state directory and record starting Git/
  filesystem state. Preserve existing user files.
- Start the real daemon with approved test settings and initialize handshake.
- Launch the actual TUI in a terminal; type real commands to define an agent,
  create/authorize a project and launch work. Record the exact supported syntax
  after implementation, not an invented Phase 0 command.
- Switch Console -> Projects -> Operations; navigate tree using keyboard; verify
  displayed owner/status against backend records.
- Enter Workspace; in actual Neovim insert text, Escape, undo, write, reopen,
  split/diff and resize. Verify disk output and dirty-buffer rejection.
- Enter Review, inspect current artifact/test evidence, reject/reapprove as
  authorized. Detach TUI while work runs and reconnect.
- Kill/restart only the fixture daemon and test safe attempt/approval/reservation
  recovery. Restore a fixture backup and compare canonical records.
- Capture failures, fix approved implementation/setup, rerun affected checks,
  and record any unsupported scenarios. Do not mark incomplete behavior done.

## Evidence record template

For each task/commit record: commit SHA, OS/architecture/tool versions, provider
and permissions scope (no secrets), fixture/input, exact command and directory,
exit code, test counts (passed/failed/skipped/unrun), interaction keys and observed
behavior, artifact/log path plus hash, failure diagnosis, rerun result and limitation.
Tie results to the current artifact revision. Mocked, manual, live and synthetic
evidence must be distinguishable.

# M0/M1 foundation qualification

The owner approved this step with “Okay, let's do this” after discussion of
checking the actual CLI, detached daemon and official Neovim. This is a foundation
check before organization/task implementation, not evidence that the MVP exists.

## Environment and isolation

Linux x86_64; Rust 1.95.0; repository commit `ab94019758` plus documentation changes.
Use `/workspace/.onboarding/activate.sh` in each shell. Tools, fixtures and generated
logs live outside the checkout under `/workspace/.onboarding/qualification/`.
The prepared cloud snapshot must retain these files; they are not Git artifacts.
No existing user authentication, editor configuration or project state is changed.
Current runtime has no configured model credentials. Local deterministic Responses
fixtures test actual runtime/tool behavior without a paid provider request.

## Actual Neovim

Installed official release **v0.12.6**, API level **14**, Linux x86_64 archive:
https://github.com/neovim/neovim/releases/download/v0.12.6/nvim-linux-x86_64.tar.gz

SHA-256 from the official release asset page, verified before extraction:
`474430d53e6264f6d6dd18db42d6dc9df3a1b56ca9e88a325bbf860e1a811d87`.

Binary: `/workspace/.onboarding/neovim/v0.12.6/nvim-linux-x86_64/bin/nvim`.
No custom editor, Neovim clone, application dependency or upstream hooks were added.

Real embedding helper invocation:

```bash
python3 /workspace/.onboarding/qualification/neovim-check.py
```

It spawns `nvim --clean --embed`, negotiates API metadata and attaches an 80×24
external UI with RGB and `ext_linegrid`. It sends keys through `nvim_input`, handles
accepted byte counts and waits for actual editor state. Twelve assertions passed:
API version, UI startup, Unicode insert/normal modes, typed save to disk, undo/redo,
100×30 resize, real diff splits, rejected stale edit, preserved human text, matching
atomic edit, changedtick buffer notifications and expected redraw events.
The version check and edit run together in Neovim's own Lua event loop.
This is a proof of the proposed conflict-check mechanism, not a shipped editing policy.

Physical PTY input also verified insert, Escape, `:w`, `u`, Ctrl-R,
`:vertical diffsplit` and `:qa!`. Saved content includes `café 界` and a second line.
The recorded session appended `Terminal save verified` and verified the actual file.
Evidence: `neovim-check.json`, `neovim-rerun.log`, `neovim-repeat.log`,
`neovim-terminal.log`, `editor/manual.txt`, `neovim-embed.stderr`.

Two setup/helper failures were diagnosed and corrected:

- Default swap state attempted a write to read-only `/home/agent/.local/state`.
  Set writable `XDG_STATE_HOME=/workspace/.onboarding/state`; retain swap recovery.
- Terminating an edited embedded fixture left a swap file and a recovery prompt
  blocked reuse of its name. The helper now uses distinct fixture names and clean
  `:qa!` shutdown. Both subsequent reruns passed all twelve assertions.

Remaining Neovim work: actual Ratatui pane rendering, focus/mouse handling, bounded
asynchronous RPC, dirty-buffer policy, isolated AI artifact following and editor
failure handling. External UI events alone do not prove pane integration. The
qualification helper uses already installed Python MessagePack; it adds no Rust
dependency and does not decide the final production RPC implementation.

## CLI and daemon

Native CLI build uses the existing smaller development profile, four workers and
the locked dependencies:

```bash
cd /workspace/codex/codex-rs
. /workspace/.onboarding/activate.sh
cargo build --locked --profile dev-small -p codex-cli --bin codex -j 4
```

Native CLI build passed (11m 14s); binary reports `codex-cli 0.0.0`. Logs:
`cli-build.log`, `host-build.log`, `bwrap-build.log`, `bwrap-rebuild.log`,
`package-build.log`. The separate Code Mode host is required by default runtime
features. Omitting it produced real SDK error events; building it resolved those.
The full SDK rerun passed **49 tests, five suites**, including MCP conformance:

```bash
cd /workspace/codex
. /workspace/.onboarding/activate.sh
CODEX_EXEC_PATH=/workspace/codex/codex-rs/target/dev-small/codex \
  pnpm --filter @openai/codex-sdk test --runInBand
```

`sdk-tests.log`: 45 passed, four failed (three missing-host event mismatches and
one timeout). `sdk-retest.log`: 49 passed, zero failed; unchanged assertions.

The package helper's `CODEX_REPO_ROOT=/workspace/codex` is required even for `--help`.
Daemon fixture settings disable remote control and automatic updates. Complete
package helpers are required to bootstrap a fresh daemon from the invoking package.

Bubblewrap initially failed because `libcap-dev` was missing. Downloaded
`libcap-dev` **1:2.75-10+deb13u1+b3**, amd64, through Debian's signed trixie index
and extracted it under `/workspace/.onboarding/native`. System package state was
not modified. Its pkg-config file advertises `/usr/lib64` while the archive stores
libraries under `/usr/lib/x86_64-linux-gnu`; a local sysroot `usr/lib64` symlink
resolves that mismatch. Build with the local `PKG_CONFIG_PATH` and
`PKG_CONFIG_SYSROOT_DIR`; no sandbox stub or skip-build flag is used.
Pinned V8 archives/bindings were fetched using the repository's package helper,
which validates the release manifest against committed hashes before artifacts.

Daemon helper invocation (requires authorized unsandboxed execution here):

```bash
python3 /workspace/.onboarding/qualification/provider-fixture.py
# In another terminal, after fixture startup:
python3 /workspace/.onboarding/qualification/daemon-check.py
```

The real daemon passed twelve assertions twice: idempotent start; 0600 socket;
correct owner; private parent; surviving client detach; completing and reading the
detached turn; blocking the local tool write before approval; replaying its pending
approval after reconnect; completing after acceptance; actual disk artifact;
changed PID on restart; completed history surviving restart. The fixture approval
allows only `printf daemon-tool-ok > daemon-artifact.txt` in its workspace.
Repeat cleanup removes only that generated fixture artifact.
Evidence: `daemon-check.log`, `daemon-repeat.log`, `daemon-check.json`,
`daemon-rpc.json` and `cli-workspace/daemon-artifact.txt`.

The first attempt failed because the outer execution sandbox masks the fixed
`/tmp/codex-daemon-1000` directory with a read-only, mode-000 mount. Mount metadata
confirmed the restriction. Authorized unsandboxed fixture execution worked without
changing its permissions or application checks. Another helper failure assumed
`pid` was always present in lifecycle JSON; it is optional for `alreadyRunning`.
The helper now verifies idempotence using status and the daemon's own PID record.

Physical terminal steps: launch raw CLI with `--disable daemon_auto_start`;
type `/status`, a fixture prompt, `/permissions`, Escape and `/quit`. Then launch
the complete package normally; `/status` reports `Local background server`.
Its session ID matched `thread/loaded/list`. A typed prompt returned the fixture
answer; `/quit` detached; `codex resume <same-id>` restored response and usage;
`/status` still reported the background server. Evidence: `cli-terminal.log`,
`daemon-terminal.log`, `daemon-resume-terminal.log`, `tui-loaded-threads.json`.

This qualifies completed-history restart and live pending-approval reconnect.
It does not establish approval persistence across crashes, reboot supervision,
agent-scoped administrative IPC denial, canonical organization durability or
provider billing behavior. Those remain later acceptance gates.

Existing socket transport tests:

```bash
cd /workspace/codex/codex-rs
. /workspace/.onboarding/activate.sh
cargo test --locked --profile dev-small -p codex-app-server-transport \
  --lib transport::unix_socket_tests -j 4
```

The sandboxed run had seven passes/four failures, all from the masked daemon
socket root. The unchanged authorized unsandboxed rerun passed **11/11 tests**,
zero ignored, 149 unrelated tests filtered. It covers unsafe-parent rejection,
private sockets, distinct long paths, symlink collision, startup locks, directory
pinning, WebSocket forwarding and shutdown acceptance only for the managed socket's
own PID. Logs: `socket-tests.log`, `socket-retest.log`.

## Reusable setup and cleanup

The tested refresh helper is
`/workspace/.onboarding/qualification/refresh-foundations.sh`. It verifies the
Neovim archive and native development package, builds the CLI/helpers with locked
dependencies, assembles the existing package and writes shell activation. The
native package SHA-256, from Debian's authenticated package index, is
`795983cf2cbc2e12e9440383d701229966c35bc40712ccbfd03e5c93d9af619d`.
Its first scripted repeat assumed a filename without the Debian epoch; actual
download filenames include `1%3a`. Corrected the exact filename, retained the pin
and verification, then reran successfully. Evidence: `setup-repeat.log` (failed),
`setup-retest.log` and `setup-final.log` (passed).

Saved complete `install_script` and `start_skill` updates in the existing
environment configuration draft. Repository membership, network rules and secret
requirements were preserved. Saving does not publish: the owner must review/save
in environment settings and publish to retain the prepared environment. No
fresh-task restoration was verified.

After the physical TUI quit, the daemon still reported `running`. Stopped only
the fixture daemon through its supported command and terminated only the provider
fixture started here. Evidence: `daemon-final-version.json`, `daemon-stop.json`.
No fixture service is intended to remain running or survive environment restore.

## Review and next gate

Use the existing daemon and editor APIs; no new orchestration framework is needed
to qualify them. Ponytail complexity review is paired with normal correctness and
permission review. Preserve checksum/TLS verification, real editor recovery and
approval boundaries. Live provider, organization storage, thousands-of-agent scale
and all five new layouts remain separate acceptance checks.

# Local fork package, upgrade and recovery

Use the complete fork package, rather than a different upstream `codex` on PATH.
The [current status](status.md) identifies the last fully qualified package.
Daily-use tasks4/10–12/14 pass native package identity, stopped release
selection and recovery. Final preview10 also passes stopped upgrade and fresh
paused full restore; see daily-use-qualification.md. A suitable-host reboot/logout test
remains separate from local daemon restart.

## Identity and diagnostics

The source workspace version remains0.0.0. The new shared install-context helper
uses the containing package's valid manifest version for the CLI version, TUI
banner/status and daemon version probe. Unpackaged binaries retain the workspace
version. Existing build-info supplies the compiled commit and target; unstamped
executables explicitly report `dev`. `codex doctor` separately records the compiled version, package version,
actual executable path and, in its detailed report, streamed executable SHA256.
It also shows the native provider wire API, output cap and HTTP/stream retry caps.
Existing redacted auth/route/readiness checks are reused. Doctor does not start
the daemon; an absent daemon is distinct from an unreachable existing socket.

The package version is a packager declaration. It cannot prove which revision
produced an arbitrary prebuilt executable. Qualification must retain the source
revision/dirty-source fingerprint, exact build command, binary and every helper
hash separately. A manifest can be edited; compare a reviewed external receipt.
Never print credential values or login-status key fragments while diagnosing.

## Build and fresh-prefix installation

Use the existing `scripts/build_codex_package.py`, canonical layout and pinned
lockfile. Source builds now invoke `cargo build --locked`; a lock mismatch fails.
Use an explicit target for this GNU Linux host, since the builder's Linux default
is musl. Inspect `--help` for supported prebuilt helper arguments and provide the
verified official helpers when retaining a qualified helper build. Prebuilt
inputs are not evidence that the current source was compiled.

From repository root, after activating the qualified toolchain:

```bash
CODEX_REPO_ROOT="$PWD" python3 scripts/build_codex_package.py --help
CODEX_REPO_ROOT="$PWD" PYTHONPATH="$PWD/scripts" \
  python3 -m unittest discover -s scripts/codex_package -p 'test_*.py' -v
```

The package directory contains `codex-package.json`, `bin/codex`, the real
`bin/codex-code-mode-host`, bundled ripgrep and Linux bubblewrap. Copy the entire
verified directory into a new, empty owner-controlled installation prefix.
Keep both releases intact; do not overwrite binaries inside a running or shared
immutable package. Launch the absolute `bin/codex` path for qualification. Only
after verification should the owner change their launcher or PATH. Do not fetch
upstream OpenAI's installer to replace this fork inadvertently.

Use a fresh private `CODEX_HOME`, preserve inherited session proxy/TLS settings
and bind any provider credential by environment name. Check `--version`, public
command discovery, actual TUI readiness/quit, all helper hashes and passive
daemon diagnostics. Do not log in by writing a test key file. The catalog is
embedded in the new binary; an explicit external catalog is a separately pinned
input. Model turns, coding, checks and Code Mode need their own bounded tests.

## Upgrade and rollback preflight

1. Read canonical inventory/execution records and current daemon identity. Finish
   or explicitly pause finite work. An uncertain request/action remains charged
   and requires reconciliation; a version change cannot reset or replay it.
2. Stop owner CLI/editor writers and the owned daemon. Verify the stopped state.
   Create the existing offline full organization bundle, retain its integrity
   manifest and hash, and keep the exact previous package. Record the package,
   provider configuration and project roots which produced those records.
3. Retain configuration and owner project originals separately. Full organization
   bundles intentionally exclude credentials, provider configuration, originals
   and frontend editor settings. A state bundle alone cannot restore a working
   installation. Never export secrets into public evidence or the repository.
4. Verify the new package's complete assets against its reviewed receipt. Restore
   the old bundle into a **fresh** test home and inspect it using the new package
   before switching the owner launcher. Preserve grant IDs, counters, approvals,
   accepted hashes and Applications; do not start historical quarantined homes.
   The new restorer holds execution ownership before publishing its database and
   sets a global pause while retaining lifetime limits. This behavior is in
   source/test qualification; older preview2 does not add a pause when the backup
   had no global control. Verify the actual restored pause before any attachment.
5. For an actual native managed installation, `codex app-server daemon update
--from-cli` copies/pins the complete current package. It can restart work:
   invoke it only after the quiescence/recovery preflight, with an explicit owner
   confirmation. Do not use public stable update as a fork upgrade.
6. Verify the running daemon reports the expected package release and normal TUI
   attachment. Exercise only fresh, finite fixture authority. Rollback qualification
   uses the old package, matching separately retained old configuration and a
   **fresh restore** of the pre-upgrade bundle. Do not point the old binary at a
   database already migrated by a newer release or copy a live SQLite file.

No forward/backward compatibility is presumed from a matching Cargo version.
Every release must test its actual state/configuration formats and restore path.
The real-host user service/logout/reboot procedure is in
[host qualification](unattended-host-qualification.md).

Actual task12 evidence now passes for preview3/53e37b68ab → preview4/811b456376:
stopped `update --from-cli --yes`, pinned new daemon identity, unchanged canonical
definitions, actual TUI attachment/exit0 and fresh pre-upgrade bundle rollback
with separately retained nonsecret config and verified pause/original hashes.
Both releases share the state format. Evidence/failures/no-replay rules:
[daily-use qualification](daily-use-qualification.md#package-lifecycle-and-actual-editor-tasks11–121436).

## Evidence required before marking this slice complete

Retain exact source/build/package hashes; fresh-prefix CLI/TUI/daemon and actual
editor outcomes; failed compatibility attempts; pre/post-upgrade canonical IDs
and accounting; offline bundle verification; separately retained nonsecret
configuration/project hashes; fresh rollback and cleanup results. Label actual
model calls, localhost fixtures, automated checks and unrun host gates separately.

Final preview10 managed update returns installedVersion0.0.0-daily.visible.preview10
and runningVersion=null when stopped; it selects/pins without starting the daemon.
Start the selected release explicitly only after the quiescence/pause preflight,
then probe its live version. The original final driver assumed a restart and
failed its version probe; a paused continuation verifies the already completed
update without repeating it. Never start historical quarantined source homes.

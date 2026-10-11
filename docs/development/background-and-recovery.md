# Local background health and complete recovery

Approved autonomy batch items 10/11. Use existing native daemon/process/IPC and
SQLite facilities. No new supervisor, archive library or application dependency.
Automated state qualification passes; final native service/recovery evidence is
being collected. This page will be updated with final receipts.

## Linux user service

From a complete qualified package, print a reviewable unit:

```sh
codex app-server daemon service-unit --home /absolute/private/codex-home \
  --environment-file /absolute/private/owner.env --stall-seconds 120
```

Optional environment file must be a regular current-user-owned 0600 single-link
file. Do not put credentials in version control or unit text. Paths are escaped
for systemd; the foreground command disables environment substitution in argv.
The unit uses `Type=simple`, the existing `app-server --listen unix://
--managed-daemon`, bounded restart rate, `Restart=on-failure`, private umask and
330 seconds for native shutdown (including the maximum 300-second configured
grace). It deliberately has no PIDFile: Codex's PID record is JSON.

Owner installation on a systemd host: stop the PID-backed daemon, save the unit
as `~/.config/systemd/user/codex-organization.service`, run
`systemctl --user daemon-reload`, then `systemctl --user enable --now
codex-organization.service`. Logout/background boot behavior may require the
host administrator to enable user lingering. The unit pins the package used to
render it; update/re-render/restart it explicitly after installing a new package.
Native TUI/CLI clients attach to its existing ready control socket without
starting another process. Use systemctl for this service's stop/restart;
PID-backed lifecycle commands do not own the foreground process.

This cloud has `systemd-analyze` but PID 1 is `tail`; it cannot prove user-manager
activation, logout, reboot or automatic systemd restart. Unit validation and
actual foreground startup/attachment/crash/restart are distinct evidence.

## Canonical progress and stalls

`CODEX_ORGANIZATION_STALL_SECONDS=10..3600` opts into idle stopping. Unset leaves
the existing finite task deadline as the bound; invalid settings disable
organization execution until corrected and restarted. Execution-read and the
inspector show `lastProgressAt`, `stallSeconds` and `stopReason` on attempts.
Model output/tool completion count as progress. Lease renewal, response metadata
and SSE keepalives do not. Approval/dependency waits and whole-run pauses are
exempt; deadlines and grant expiry still advance. Resume gets a fresh idle baseline
while retaining the recorded last actual progress.

A stall atomically fails the current attempt, releases admission capacity and
marks dispatched effects uncertain. Unknown model usage remains charged as an
unresolved admission; shared pools block further work. Investigate/reconcile
before owner recovery; do not automatically replay requests or reset ceilings.

## Offline directory bundles

Stop the service and all runtime writers, including separate Console/exec clients:

```sh
codex organization bundle-backup --destination /absolute/fresh/bundle --offline
codex organization bundle-restore --bundle /absolute/fresh/bundle \
  --destination /absolute/fresh/recovered-home
```

The source uses CODEX_HOME and CODEX_SQLITE_HOME (defaults to CODEX_HOME).
The CLI refuses a live control socket, and the business execution-owner lock
excludes another organization host. `--offline` acknowledges quiescence across
other runtime clients: this is not an atomic hot snapshot across databases.
SQLite VACUUM INTO creates consistent per-database images including committed
WAL contents. Business migration checksums and foreign keys remain verified.

The private bundle contains a versioned SHA-256/byte-count manifest, verified
business database, every existing native runtime database, sessions and archived
sessions, and full recorded isolated workspaces/artifacts. Credentials,
configuration, sockets, package binaries and owner original project files are
excluded. Retain configuration securely outside the bundle and configure the
recovered home explicitly before starting it. The bundle itself may contain
private prompts/artifacts: never commit it.

Limits: 10,000 files, 64 MiB per file, 512 MiB total, 32 directory levels. Unsafe
ownership/write permissions, symlinks, hard links, nonregular files, noncanonical
relative paths, hash/size failures, missing workspace inventory and unsupported
business migrations refuse recovery. All manifest entries are validated before
the fresh destination is created. Source workspaces must still exist; missing
workspaces are a failed full backup, not silently omitted data.

Restoration retains database/task/attempt/grant IDs, scopes, usage, review and
Apply receipts. It moves workspace references and native runtime rollout/CWD
indexes into the new private home. Old audit text stays historical; originals
remain at their explicit external project paths. Runtime settings/auth must be
provided separately. Startup fences interrupted attempts as needsReconciliation;
unknown work does not resume automatically. Reviews recheck artifact hashes;
Apply rechecks original bases and still permits one application per accepted
result. A completed Apply receipt survives recovery and cannot be replayed.

Destinations must be fresh; existing data is never replaced. A failure after
creating a private directory leaves it for inspection without a completed
manifest/receipt. Do not start a partial recovered home or remove it to conceal
an error. Validate on a fresh distinct destination after fixing the cause.

Historical rollout event contents keep their original context/permission evidence;
restore relocates the native indexes and protected business workspace binding.
Managed interrupted tasks continue only through the existing owner checkpoint
recovery, which creates a new guarded runtime/workspace. Do not use a generic
Console resume to replay restored managed-task work or bypass its uncertainty.

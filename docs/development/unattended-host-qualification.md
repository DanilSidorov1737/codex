# Real-host unattended service qualification

Task 1 is **unqualified on the current cloud host**. PID 1 is `tail`; unit syntax,
native detached startup and foreground crash recovery from the autonomy batch
remain valid evidence, but they do not demonstrate systemd automatic restart,
logout/lingering or reboot. Never reboot the shared cloud workspace or replace
its PID 1 to manufacture this result.

## Required test host and boundaries

Use a disposable Linux VM or an owner-designated Linux machine with a working
systemd user manager and permission to test that machine's logout/reboot. Keep a
separate test account, a fresh private `CODEX_HOME`, a fresh project and the exact
qualified package. Preserve every grant, admission, receipt and failed check.
Use a localhost synthetic provider or an already bounded owner-authorized pilot;
provider configuration/credentials stay outside bundles and public evidence.
An inference limit is not a hard monetary ceiling.

Collect `systemctl --version`, `systemctl --user is-system-running`,
`loginctl show-user "$USER" -p Linger`, kernel/version and package asset hashes.
Do not print environment contents or credential files. If there is no user
manager, stop this qualification and report the host limitation.

Generate the reviewed service with the existing command:

```bash
codex app-server daemon service-unit --help
```

Use its actual supported absolute CLI/home/environment-file options; keep the
generated unit and private environment file in the test account. Unit generation
alone does not install or enable a service. Install the reviewed unit only on the
designated host, then use `systemctl --user daemon-reload`, `enable --now`, `show`
and a narrowly selected `journalctl --user -u` to record its startup. Confirm one
canonical execution owner and attach the real CLI to that service.

## Acceptance sequence

1. Create a fresh finite coding/document fixture through the native owner API.
   Record exact scopes, request/reported-token/run limits, IDs and original file
   hashes. Keep one future-bound occurrence and one approval-waiting occurrence;
   neither should be duplicated by a frontend restart.
2. Physically open the CLI, inspect canonical state and quit normally. Confirm
   the systemd `MainPID` survives and recorded work/approval remain available.
   Reattach the CLI and match the same IDs, admitted counters and artifact hashes.
3. At a documented safe point kill only the service's recorded main process.
   Observe a changed `MainPID` and increased `NRestarts` under `Restart=on-failure`.
   Confirm exactly one owner, unchanged grants/counters and retained approvals.
   Unknown inference/tool effects must require reconciliation; do not replay them
   to make the test look successful.
4. Enable lingering only for the designated test account if the owner has
   authorized it. Log out every session for that account. From a separate
   authorized observer confirm the service remains alive; log back in and verify
   canonical IDs, accounting, approvals and finite expiry. Merely closing the TUI
   does not qualify logout.
5. Record the boot ID and canonical counters, reboot only the designated test
   host, then record a different boot ID. Verify automatic user-service startup
   without manually starting the daemon, one execution owner and unchanged
   historical identities/accounting. Missed interval occurrences are skipped;
   accepted-result triggers deduplicate; interrupted effects remain fenced.
6. Physically review the checked result in actual official Neovim. Acceptance
   and any Apply require their own exact owner decisions. A completed application
   must have one durable receipt and refuse a replay after service/reboot restore.
7. Cancel every remaining finite series/future grant, stop the owned service and
   synthetic provider, disable the test unit and remove only its installed unit
   link/file. Preserve state, logs, packages, failed attempts and evidence. Restore
   the test account's prior lingering setting if it was changed for this test.

## Required evidence

Record commands/exit codes, frontend exit codes, before/after boot IDs and service
PIDs/restart counts, timestamps, original/checked artifact hashes, native task and
series reads, limits/admissions/reported and unknown usage, exact decisions,
application receipts and cleanup. Report each of startup, attachment, frontend
exit, automatic restart, logout and reboot separately as passed/failed/unrun.
No host access, logout or reboot was performed while preparing this runbook.

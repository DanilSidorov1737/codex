# Operations batch native qualification

The owner approved all twenty tasks. Use [next-twenty.md](next-twenty.md) for
scope and [status.md](status.md) for the current checkpoint. Existing fixtures,
accepted results, grants, usage and application receipts must never be replayed
or reset to make a test pass. External integrations remain deferred.

## Reviewed autonomy and background recovery

Source `a8fb65ebf4`, package `operations-batch/editor-package/`, CLI SHA-256
`f197e2dd12469cd5bf158504ab7c9b7c1499622759262ceab8f2cec65e96a262`.
The subsequent editor notification fix changes only local refusal visibility;
it has no server authority, accounting or wire-contract change.

Fresh fixture `operations-batch/reviewed-autonomy-v2/` records:

| Capability | Actual evidence |
| --- | --- |
| Finite discussions | `discussion.json`: physical template/scope/run review, two arbitrary participants and one conclusion author, one round, per-step file isolation, approved real subprocesses, immutable checks and exact acceptance. Conclusion waits for both accepted contributions and retains dissent/evidence/follow-up proposals. Six local responses/90 synthetic tokens. |
| Goal policies/supervision | `goal-native.json`: exact goal/template/work-type/scopes, one finite run, physical pause/resume/revoke, native private provenance and evidence, immutable checks/acceptance. Original three-request/45-token grant and usage survive controls. |
| Knowledge maintenance | `knowledge-native.json`: private note search at the correct agent, physical retirement/restoration, stale revision refusal, retained original body/provenance/control history. Other agent retrieval stays empty. |
| Reviewed replans | `replan-native.json`: actual blocked checkpoint after approved partial work, content-only pending proposal, changed receiver revision refusal, fresh exact delta and one native same-grant handoff. Two attempts report two requests/30 tokens each. Grant ID/scope/accounting remain; nodeRevision correctly rebinds to the reviewed receiver. Prior accepted results remain accepted. |
| Scorecards | `replan-native.json`: captured source/receiver owners, provider/model, actual immutable checks and request/token receipts; physical canonical summary. Current task ownership does not rewrite source attribution. |
| Detached completion | `background-ready.json` and `background-native.json`: approved document tool followed by a real immutable subprocess check still running after normal TUI exit; completion occurs without the frontend, then a new frontend exactly accepts it under the same three-request/45-token grant. Original document remains unchanged. |
| Crash/restore | `recovery.json`: actual SIGKILL of only the recorded managed-daemon PID after an admitted silent local response, same-grant/attempt unknown fence on restart, no replay, live-bundle refusal, offline business/runtime/session/workspace bundle, quarantined source workspaces, remapped fresh-home restore and corrupt-bundle refusal before destination creation. Controls/history/notes/workflows/results/scorecards and unknown accounting survive. |
| Restored native review | `restored-native.json`: actual restored CLI/TUI/official-Neovim Follow and Diff, versioned controls and canonical unknown-usage summary, normal exit 0, no inference or original writes. |

The successful fixture records **17 admitted / 16 reported requests, 240
synthetic tokens, one unknown request and zero paid calls**. Final global state
is paused, and source/restored daemons and the tracked provider are stopped.
Original owner project files are outside the bundle and remain unchanged.
Owner-only fixture configuration is copied separately; credentials/config are
excluded from the data bundle. Frontend Neovim checkpoints are separate data,
not part of runtime backup.

Do not restart the quarantined source home. The mapping is
`reviewed-autonomy-v2/workspace-quarantine.json`. Use only a fresh restore for
further read-only review, retaining the unknown fence and original accounting.

The original `operations-batch/reviewed-autonomy/` fixture is also protected:
the provider child exited before serving its first admitted request. Its one
charged unknown request correctly fences the configured pool. Queued grants
were cancelled; failed evidence remains, global state is paused and services
are stopped. Never reset or restart it. The replacement fixture has fresh
identity/authority and an explicitly tracked provider session. All failed
setup/input/metadata/grant-comparison drivers and logs are retained.

## Larger dormant registry

`operations-batch/registry-native/{prepared,physical,restart}.json` pass.
Native owner APIs create one 128-definition preset, 76 fresh clones and 68
additional arbitrary agents: **10,000 unique stored nodes**, no task grants,
notes or model requests. Fifty 200-record pages load through the actual TUI.
Physical stable-tail search, tree collapse/navigation, refresh, three sizes and
native restart retain the same IDs; both CLI exits are 0. All five installed
asset hashes and SQLite quick/foreign-key checks pass. The owned daemon stops.

Observed startup is 2.735 s; tail search including terminal settling 0.852 s;
refresh/search 2.204 s. Frontend RSS is 284,404–300,480 KiB. These are measured
observations on this cloud host, not an SLA. The independent 10,000-deep tree
unit check takes 124–279 ms in recorded runs. The default-two/max-four active
execution envelope remains unchanged; stored capacity does not prove thousands
of concurrent sessions. Other record/report pagination remains capped at 2,000.

## Editor qualification continuation

The protected controls code result has **one completed exact hunk Application**;
`editor-native/partial-receipt.json` pins index 0, canonical patch/output hashes
and the completed receipt. Dirty actual-Neovim buffers refused Apply before any
intent; repeated backend Apply refused after completion. The partial original
fixes addition and preserves the unselected multiplication hunk; the full-result
oracle correctly fails on that partial subset. Later buffer proposals and owner
`:write` are native editing, never a second backend Apply.

A stale original-file proposal retained the external owner's line, but its local
refusal message was cleared by background refresh. Source `18cc403424` retains
it using the existing error flag; 70 affected TUI/official-Neovim checks pass.
The first test compile named a private helper type; the corrected test uses the
existing serde contract without expanding visibility. Remaining package/editor
session restart qualification is recorded in status/testing. Do not rerun any
pre-Apply driver. Resume only from the one receipt and current original bytes.

## Remaining host boundary

Task 1 needs a real Linux user manager with logout/lingering and reboot support.
This cloud container has PID 1 `tail`; no host logout or reboot was performed.
The reviewable service-unit/foreground checks pass. Follow
[unattended-host-qualification.md](unattended-host-qualification.md) on a suitable
host; container restart is not evidence for host reboot continuity.

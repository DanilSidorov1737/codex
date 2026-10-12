# Mission/usability batch: final native qualification

All 25 approved bounded tasks are implemented and qualified on application source
`8cd392eaaaff72da7159d16fb4c175432fd838d0` (2026-10-12). Scope and implementation
contracts: [mission batch](mission-batch.md). Resume rules: [status](status.md).
Ponytail implementation/review reused existing controls/transactions, forms,
workflow/delegation/notes/evidence and actual Neovim; no dependency or editor
engine was added. Local source commits were made throughout implementation.

## Source and package

Logical commits: `186a9e65f8` scope, `d09c59918a` production formatter regression,
`53c9b6741c` first-five UI, `def0cdfa77` mission/evidence/coordination,
`3d4374ebe9` playbooks/artifacts/editor/advice, `c29de75575` required-base recovery
and generated contracts, `f2087501ed` production lint/wire coverage,
`8cd392eaaa` native pane order/labels and owner-setting restoration.

Package `0.0.0-mission.preview2.8cd392eaaa`:

- Working directory: `/tmp/mission-final-package-v2`.
- Persistent archive: `/workspace/.onboarding/qualification/mission-batch/final-package.tar.gz`.
- CLI SHA256: `35f13f68ebcc6fe25c13336ddfa1300e42305c2504c686422e01964eaf9d693d`.
- Archive SHA256: `aca3d1f7ce4c456c0efada3c3d401d8f54fc4b00eda3a604a0f79d705fc35709`.
- All five archive and final installed assets match `final-package-hashes-v2.json`;
  installed copies are private single-link files. Static CLI version is `0.0.0`;
  package metadata/content hashes identify this preview's actual source.

Activate `. /workspace/.onboarding/activate.sh`. Cargo cwd is
`/workspace/codex/codex-rs`, Rust1.95.0, locked/offline dev-small, `-j4`,
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608`. Final build used
`cargo rustc -p codex-cli --bin codex -- -C link-arg=-Wl,--strip-all` with the
same locked/offline/profile arguments to avoid a large intermediate symbol copy.
Packaging uses existing `scripts/build_codex_package.py`,
`CODEX_REPO_ROOT=/workspace/codex`, explicit existing helper paths and `--strip all`.
No download/tool installation occurred. Generated cache is now cold: only owned
compilation outputs were cleared after validation; check headroom before rebuilding.
All source, qualified packages, state and retained evidence remain intact.

## Automated checks

See [final testing](testing.md#missionusability-batch-final-qualification).
**836 distinct selected checks pass**: state322, protocol375, schema4, SDK25,
formatter-driver1, runtime15, TUI/actual-editor92 and wire contracts2.
Production CLI Clippy `-D warnings`, final build/package, generated contracts,
30 changed Rust files and changed Markdown formatting pass. This is selected
qualification, not an all-platform/all-CI claim. Whole-root formatting retains
missing just/dotslash and untouched legacy Rust ordering failures.

## Actual application evidence

Evidence root: `/workspace/.onboarding/qualification/mission-batch/`.
All model responses are deterministic localhost fixtures; native tools, sandbox,
checks and official Neovim0.12.6 are real. No new paid call ran.

| Receipt                                                      | Observed behavior                                                                                                                                                                                                                  |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `ui-native/physical.json`                                    | One guided fresh agent proposal/exact application; named view save/restore; 37-node concept-style tree; 36 rapid switches at120x36/50x20/280x80; normal exit0, zero inference, historical source DB unchanged.                     |
| `native-workflow/pilot.json`                                 | Editable coding/reviewer playbook, distinct configured owners, exact native-action approvals, signed-addition and review/dissent oracles, separate root owner acceptance, verified text and actual captured-test panel.            |
| `lesson-private-evidence.json`, `context-qualification.json` | Accepted-result private lesson with provenance; actual scoped organization context and native proof; public/other-agent/project searches exclude private content. No promotion.                                                    |
| `delegate-qualification.json`                                | Actual ready discovery/proposal/delegate/wait and one checked accepted child within original shared scope; separate owner root acceptance, 7 shared requests.                                                                      |
| `loop-qualification.json`, `native-assistance-evidence.json` | Ninth identical inbox read stops into a verified blocker/checkpoint; exact team assistance resolution changes no grant/task/accounting or execution.                                                                               |
| `owner-controls-evidence.json`                               | Exact role rollback produces immutable version3 without rebinding captured attempts/nodes; captured-run template rollback produces revision4; exact accepted evidence completes both goal criteria.                                |
| `background-complete.json`                                   | Seven distinct bounded documents/checks over1846.319 detached seconds while TUI closed; finite UTC interval/max-runs/request/token limits retained.                                                                                |
| `editor-terminal-evidence.json`, `three-way-evidence.json`   | Actual owner shell/proof in Neovim, bounded artifact catalog/provenance; dirty refusal and three protected original/owner/agent native diff copies.                                                                                |
| `apply-evidence.json`                                        | Actual native manual resolution/save, dirty and changed-base refusal, explicit original-base restoration, exactly one owner-confirmed completed Apply; installed oracle passes and repeat choice refuses.                          |
| `recovery-evidence.json`                                     | Owned-daemon SIGKILL after one admitted silent response; same unknown stays charged without retry; live-backup refusal; stopped full bundle/fresh remapped restore; corrupt/omitted required-base restore refuses before creation. |
| `restored-physical-evidence.json`                            | Final installed package: base/owner/agent labels and exact left-to-right contents/options; unknown remains visible, completed Apply refuses repeat; 36 further rapid switches, normal exit0, identical accounting/originals.       |
| `final-cleanup.json`                                         | Five final installed asset hashes, 21 business/runtime SQLite quick/FK checks, owner originals, one Application and accounting verified; all owned batch services/sockets stopped.                                                 |

Final accounting: **38 admitted /37 reported requests, 555 synthetic tokens,
one charged unknown, zero active and zero paid**. Global ceiling remains40
requests/2000 reported tokens/maxActive2; no reset or expansion occurred. The
unknown's enabled safe-recovery policy never retried it. These fixture totals
are not billing or model-quality evidence. Historical paid estimates are unchanged.

## Protected state and recovery

**NEVER restart `/tmp/mission-native-home`**. It is stopped and its 14 recorded
isolated workspaces are quarantined; read `native-workflow/workspace-quarantine.json`.
Never replay setup, grants, accepted results, goal decisions, rollback application
or Apply, or reset ceilings/uncertainty. Original owner project files remain at
`native-workflow/native-project`; their final post-Apply hashes are recorded.

`/tmp/mission-restored-home` is globally paused/stopped and preserves exact
business records/history, usage, task/grant/result/checkpoint/handoff/Application
records; only isolated workspace paths were remapped. Further review uses a fresh
restore. `/tmp/mission-ui-home` and every historical fixture also stay protected.

The stopped full bundle at `native-workflow/bundle` includes required original
base bytes under protected owner-home storage, separately from agent grants.
Captured bases distinguish required snapshots from legacy optional records without
changing old digest serialization; migration18 refuses unsupported old hosts.
Bundles exclude owner originals/config/credentials/packages/private frontend
state; keep those separately. Restore hash/path/ownership/size/SQLite/FK/migration
checks precede destination creation. Never infer missing base bytes from a current
owner file or merge/apply automatically.

## Failures, limits and artifacts

All failures remain distinct from passes. Finite disks filled with generated
feature variants; only compilation cache artifacts were pruned, with receipts.
Cache-reset receipts hash path/size inventories, not file contents. Corrected
build commands retained source/state and did not replay successful actions.
Copied helpers used obsolete CLI aliases, wrong cwd/env/tool names and unsuitable
confirmation/selection assumptions. Long per-key editor commands and native
paste/newline/toast timing were harness failures; continuations inspected saved
state. Daemon version requires a live socket; absence after successful stop is
verified by owned process/socket checks, not a fabricated stopped version reply.

Actual source findings were fixed: lint-only redundant copies/match/nested if,
stale optional-hunks contract expectation, reversed/unlabeled native panes, and
owner window-option capture after edit. Meaningful checks were rerun; no assertion,
TLS/signature/hash/permission protection was weakened. Some repeatedly named failed
screenshots overwrote earlier captures; surviving captures are preserved.

Screenshot archive:
`/workspace/artifacts/cli-mission-batch/cli-mission-batch-screenshots.zip`,
40 actual PNG/text pairs,84 CRC/hash-verified entries, SHA256
`541a0898d1d30f0e6e4bb592dcd77b8323c0625a37b04f3d7c15a0405fdcd9e1`.
Selected tree/three-way/owner-terminal PNGs accompany it. These render actual
Neovim external UI grids, not illustrative mockups. No homes/config/credentials
or rollouts enter the screenshot archive. Local commit bundle:
`/workspace/artifacts/codex-mission-batch-local-commits.bundle`, prerequisite
`ddb67e5f32`; verify before manual recovery. No push is authorized.

All 25 bounded tasks are complete. The approved real-host systemd/logout/reboot
gate remains unrun here (PID1 `tail`). Large definition registries do not prove
thousands of simultaneous workers; active default2/max4 is unchanged. External
integrations, distributed workers, new paid probes, increased concurrency and
automatic root acceptance/Apply remain separate. Configuration draft save does
not publish/apply a snapshot or prove fresh-task restore; retain archives/bundle.

# MVP release qualification

B1–B6 implementation and local qualification are complete on 2026-10-10 UTC.
Heartbeat/database contention, idle editor redraw, overlapping dashboard IDs and
refresh-triggered task inspection bugs were fixed and verified. **The MVP
is not fully release-qualified: A05's bounded live-provider pilot is unrun.**
The owner supplied OPENAI_API_KEY and approved compatible-GPT pilots with minimal
spending; the conservative working ceiling is $1 total. Authenticated API model
listing now returns 200 and lists the native-tool gpt-5.5. Three isolated proposed
tasks and a hash-matched private daemon package are prepared with zero usage.
Official pricing pages remain blocked by proxy CONNECT 403; their domain additions
are saved in the draft for review/save/publication. Current rates must be verified
and scopes checked against the working budget before inference. No paid call was made.
Deterministic Responses fixtures exercise the real packaged CLI, detached daemon,
native sandbox, actual filesystem and independent immutable owner checks. They
cannot establish live model quality, provider compatibility or real billing.

## Acceptance evidence

| Criterion | Local result and evidence | Remaining boundary |
| --- | --- | --- |
| A01 arbitrary agents/forests | CLI creation/restart and native API preserved stable IDs; standalone and custom delegation ran | No mandatory role names or hierarchy levels |
| A02 versioned hierarchy/membership | Real stale edit/cycle denials; transactional reparent/membership/state tests passed | Moving nodes never grants execution authority |
| A03 goals/tasks/plans | Persistent goal/project/task contracts and bounded plan/DAG/child acceptance tests; real native delegated plan ran | Owner grants constrain every child; no autonomous authority expansion |
| A04 claim/generation fences | Concurrent claims, stale completion/settlement and reservation tests passed | Single owner host |
| A05 useful live model workflow | Native fixture pilots passed; live API authentication/model listing passed; three fresh real-provider tasks prepared at zero usage | **Blocked: official pricing access; paid inference/quality unrun** |
| A06 detach/restart | Real CLI detach/reconnect and post-dispatch/pending-approval SIGKILL/restart tests passed | Interrupted work reconciles; arbitrary reboot/service supervision unqualified |
| A07 pause/revocation | Physical pause/resume plus inherited authorization/delegation/context state gates passed | Existing native cleanup retains reservations until joined |
| A08 exact human approvals | Actual protected owner approval/rejection; stale approval, pending crash and self-administration denial passed | Silence never approves |
| A09 budgets/limits | Real reported usage, exhausted request/tool gates, child shared-budget/capacity races; invalid boot limit creates no grant/attempt | Corrected four-active/default-two native checks passed; provider billing is not capped |
| A10 uncertain effects | Native post-dispatch crash and application intent restart become reconciliation/uncertain, with no automatic replay | External financial/messaging effects excluded |
| A11 protected authority | Nine real sandbox denial assertions cover canonical state, owner IPC/home, other roots and privileged arguments | Notes/instructions remain untrusted input |
| A12 five layouts | Final source: 370 physical commands at 120×36/50×20, 20 ms switches and 20 Console reopen cycles; 17 organization/editor tests and prior 152 layout/slash/overlay checks passed | Linux clean-terminal envelope |
| A13 Operations tree | Actual selection/search/collapse/expand and pause/resume; 1,000-agent deep-node PTY benchmark passed | Frontend bounds record loading; owner CLI/API pagination extends inspection |
| A14 official Neovim | Actual v0.12.6 modal Unicode save/paste/undo, splits/diff, focus and resize passed | Clean configuration; user plugins/configs unqualified |
| A15 isolated editing/review | Protected read-only preview, native diff, dirty buffers/changed originals, exact hash Apply and owner oracle passed | Actual two-stage native Follow retained focus/cursor; external writers must be coordinated |
| A16 editor independence | SIGKILL only the owned editor during native 15-second work left daemon running; task reached NeedsReview; saved files reopened | Clean mode disables swap: unsaved crash data is lost |
| A17 scoped persistent context | Durable notes/inbox/ack across fresh grants, cross-project denial and real two-agent context exchange passed | No embedding/vector retrieval or automatic global memory promotion |
| A18 canonical recovery | Corruption/missing/incompatible state and actual SQLite full error tests passed; native CLI verified/fresh snapshot restore matched 21 tables | Business DB snapshot excludes runtime history and workspace artifacts |
| A19 stored scale/active bound | 1,000 additional agents, 200-level hierarchy, six owner pages, real PTY navigation and default-two/third-queued work passed | Corrected four-active/fifth-queued and default-two/third-queued checks passed |
| A20 exact review | Physical request changes/acceptance; artifact/check/base digest and one application receipt enforced; all four native API cases passed | Model claims cannot set completion |
| A21 startup/idle | Complete package/helper startup and real 30-second idle interval passed without model requests | Final corrected package passed the 30-second no-inference interval |

## Measured envelope

One x86_64 Linux owner host; Rust 1.95.0, Node 24.19.0,
pnpm 10.34.5 and official Neovim v0.12.6.
Use the exact installed version output in qualification.md; no extra installation
was needed for B6. `dev-small` is an unoptimized qualification build.
Hardware evidence is `mvp-b6-cli/hardware.json`: five CPUs in affinity, cgroup quota
equivalent to four CPUs, 32 GiB cgroup memory and 32 GiB workspace filesystem.

The daemon accepted 1,000 additional stored agents, including a 200-level primary
hierarchy, without model requests. It returned 1,002 nodes in six owner pages of
at most 200 records. Reads were 2.3–23.5 ms; mutation p95 was 21.5 ms. Sleeping
agent definitions do not allocate model sessions. Evidence: `scale.json`.

Actual PTY tree navigation passed 30 keys and a deep-node search at 120×30
xterm-256color. Final-package p95 was 27.5 ms, measured to 20 ms terminal-output quiescence;
full startup to loaded tree was 1.76 seconds. Startup includes CLI/provider/UI
handshake and is separate from first owner page latency. Final TUI RSS was 274,776 KiB,
daemon RSS 290,924 KiB. Evidence: `tui-performance.json` and its terminal log.
These are final unoptimized-package measurements on this host; the earlier
30.9 ms/2.79 second baseline remains in baseline-tui-performance.json.

The corrected package passed default-two/third-queued and four-active/fifth-queued
native execution. All eight tasks reached NeedsReview, all immutable checks exited
zero, each used two requests/30 reported tokens and peaks stayed within each limit.
The first four-active failure remains recorded in testing.md; its heartbeat bug
was fixed with concurrent future polling and a contention/deadline regression.
Current evidence: concurrency-2.json, concurrency-4.json and corrected logs.

## Native pilots and recovery

Single addition repair, delegated addition repair and a document generated from
immutable local facts passed real Python owner regressions, exact acceptance and
explicit application. Root grants used respectively 2/30, 7/105 and 2/30
requests/reported tokens. No live model was called. The initial fixture named its
module `math.py`, colliding with Python's native module; the independent regression
caught it. Fresh `calculator.py` projects passed. Original failures remain.
Evidence: `pilots.json`, `mvp-b6-native-pilots-corrected.log`.

CLI backup/fresh restore matched all 21 canonical tables, including 1,006 nodes,
grants, budgets and four completed application receipts; both integrity checks
passed. Evidence: `restore-qualification.json`. A real 30-second idle interval
retained the same daemon and generated no model requests: `idle.json`.
Canonical full-disk qualification uses actual SQLite `SQLITE_FULL` through a page
limit on a fixture DB; it never deliberately fills the cloud host disk.

## Resource, review and Ponytail review

Owner boot limits accept 1–4 active attempts, default two. Invalid settings disable
consequential execution, retaining definition inspection. Cached catalog entries
that declare CodeModeOnly are denied before workspace/thread/inference creation.
Names alone do not imply tool mode on an arbitrary custom provider. Four resident
root workflows and 32 descendants/root remain independent bounds. See
[resource limits](resource-limits.md) and [coordination](coordination.md).

Trusted owner price quotes match provider/model/source and show usage/ceiling
estimates. Missing quotes display unavailable. The localhost fixture has a real
zero-billing owner quote; this is not invented public provider pricing. Estimates
exclude unreported and non-token charges and are never a monetary hard cap.

Ponytail implementation/review reused SQLx/SQLite, Tokio, existing daemon/thread/
sandbox/tool contracts, shared safe file descriptors, Ratatui and official Neovim.
Only the MIT MessagePack codecs are new editor dependencies. No duplicate daemon,
broker, custom editor, workflow framework or speculative distributed service was
added. The heartbeat fix retains the single connection and corrects future polling
instead of adding connections to hide contention. Correctness/security/native
checks accompany the simplicity review; no standalone hooks were installed.

## Release gate and resumption

All independent local gates passed: four/two-active native execution, zero-effect
model/limit denial, real quote/native Follow, corrected idle redraw, final hierarchy
navigation and the final 30-second no-inference interval. Final source is
`f92e8c2292`. Combined final checks passed **8 + 31 + 16** tests, production Clippy,
formatting and the complete CLI/package. All four native API cases passed on the
unchanged backend source. Other targeted/wire/schema/SDK results are in testing.md.

Final CLI SHA256: `81d31243e6c3d2cd261d4a538008cb2c029f51c51eb6a25b79b54185310095dc`.
`final-package-proof.json` matches CLI, Code Mode helper, bwrap, ripgrep and manifest
between the completed package and actual installed daemon. The final actual idle
Follow measured **1 frame in 3.25s**, versus
326 before the fix with the same <=8 assertion. Final idle daemon evidence:
`idle.json`; final navigation: `tui-performance.json`. Fixture daemon/provider were
stopped, retaining package/state/logs; `final-cleanup.json`. The container retains
a stopped zombie (no execution), and its managed PID record was removed.

A05 stays unrun until current official pricing is reachable and the prepared scopes
fit the $1 working ceiling. Credentials, compatible-GPT and minimal-spend authority
are already present; authenticated model listing passed. This is a release blocker,
not unfinished local feature implementation. Do not repeat those approvals or infer
broader external authority. Resume with the bounded live pilot and
record genuine model quality, native check results, usage and test spend. Broader
roadmap phases remain separate scope.

Evidence roots are `/workspace/.onboarding/qualification/mvp-b2-cli/` through
`mvp-b6-cli/`, with sibling `mvp-b*.log` files. Logs/state/binaries stay outside Git;
committed docs retain reproducible commands, results and limits. Qualification on
additional platforms, user editor configurations, broader providers, simultaneous
shared-buffer merging, overlapping parallel child outputs and arbitrary external
writers remains unqualified/deferred. Prefer disjoint child artifacts.


## Startup/workflow follow-up — 2026-10-10 UTC

Current application source is `6a4a9f6329`, following `4a19208b2c`.
The final complete source/managed package has CLI SHA256
`6152cbbd0e4b47877862685dc5d54f94f29ba3e6fd23f05de24b760089255ae7`;
all helpers/manifests and running process identity were matched. Package:
`/workspace/.onboarding/qualification/mvp-followup/package-v2/`.

Two small TUI fixes reuse existing UUID and Inspect paths. A real remote-client
regression failed with the original duplicate ID 0, then passed overlapping startup,
canceled dashboard, pagination and every worker operation. Physical qualification
found the missing execution inspection when refresh selected a newly visible task;
the extended regression covers deletion, stable selection and new search matches.
All 17 organization/editor tests, production Clippy/format, full CLI/package passed.

On this final source, all three native fixture pilots again passed isolated immutable
checks, exact acceptance, explicit Apply and independent original-project oracles.
The full physical workflow passed two exact approvals, actual Neovim Follow/native
diff, delayed native command completing after TUI quit, original unchanged through
acceptance, explicit Completed Apply, owner oracle and reopening the applied file.
370 actual layout commands passed online/offline across two sizes; no duplicate ID
or owner timeout frame and zero inference requests during layout checks.

Detailed failure/correction/commands/evidence are in testing.md and `mvp-followup/`.
Actual captures/archive: `/workspace/artifacts/cli-followup/`. Owned temporary
services are stopped; final package and state are retained. Earlier B6 performance/
recovery evidence still applies to unchanged backend code and is not a new benchmark.
A05 remains blocked; local fixture responses cannot establish live model quality,
real provider compatibility or billing. No CodeModeOnly bypass or new editor was built.

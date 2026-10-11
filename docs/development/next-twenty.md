# Next twenty tasks: approved implementation scope

The owner approved the preceding twenty-task proposal with “ok lets do this”.
Use Ponytail, logical local commits, maintained documentation, meaningful tests
and actual CLI/TUI/official-Neovim interaction throughout. External integrations
remain deferred. Initial source: `41820fc11e`; worktree was clean. The qualified
autonomy batch and every historical fixture/grant/receipt remain protected.

| Task | Bounded implementation and evidence |
| --- | --- |
| 1. Unattended startup | Prepare a real Linux user-manager test covering attachment, frontend exit, process crash/restart, logout/lingering and reboot continuity without replay. Current cloud PID 1 is `tail`; no real host reboot claim from a container restart. Unit syntax/foreground evidence already passes. |
| 2. Command palette/help | Search the existing owner actions/layouts, show context/shortcuts, invoke their existing handlers. Cancel/search must grant no authority; exact decisions still confirm. Physically exercise keyboard discovery in the TUI. |
| 3. Automation forms | Create finite recurring workflow and accepted-result series from captured native previews/configuration, with exact confirmation and existing server validation. No JSON editing or automatic acceptance/Apply. |
| 4. Organization presets | Save/clone configurable roles/nodes/memberships without copying grants, tasks, private notes or identity. Validate tree/role/member bindings and atomically create fresh records. Presets are optional. |
| 5. Goals/milestones | Reuse existing parent IDs and goal/task links for a navigable goal tree and evidence-derived milestone progress. Reject cycles/stale changes; model assertions cannot mark goals complete. |
| 6. Project lifecycle | Explicit owner pause/complete/archive/reopen, preserve definitions/results/history and fence new admissions while paused/archived. Completion requires reviewable criteria evidence; reopen never resets accounting. |
| 7. Priority/fairness | Owner-controlled bounded priorities with deterministic aging across eligible work; paused/exhausted/dependency-blocked tasks cannot monopolize selection. Test starvation/concurrency and unchanged grants. |
| 8. Organization limits | Enforce owner-versioned global request/reported-token/concurrency ceilings from native root ledgers. Descendants/handoffs count once; historical/revoked grants and unknown usage remain charged. In-flight token overshoot stays explicit. |
| 9. Emergency pause | One versioned owner action fences new admissions across the organization. Existing in-flight receipts may finish; quiescent work releases capacity. Resume preserves grants/expiry/usage and refuses unknown replay. |
| 10. Bounded autonomy policies | Owner-approved finite templates, models, projects, work types and resource bounds; changes/revocation are versioned. Reuse exact scopes and existing series; policy text never expands protected authority. |
| 11. Goal supervision | Persist approved goal supervision using finite policy/template executions, source evidence and existing scheduler. Advance through proposals/delegation/checks/review within captured bounds; no automatic root acceptance/Apply. |
| 12. Replanning | Propose versioned replacements for unfinished work after failure/priority changes; preserve accepted work, original accounting and uncertain fences. Owner reviews the delta before new authority. |
| 13. Agent discussions | Finite participants/rounds/requests/tokens, durable contributions and reviewable conclusions/disagreements/follow-up proposals. Participants receive only permitted context; discussion output grants no authority. |
| 14. Knowledge maintenance | Owner review of stale/conflicting/duplicate notes, provenance and history retained. Scope/membership filtering occurs before search; corrections/retirement cannot leak private knowledge. |
| 15. Scorecards | Derive accepted outcomes, failures, real checks and recorded usage by captured agent/model. Unknown dimensions remain unavailable; no invented quality/cost or automatic model fallback. |
| 16. Neovim sessions | Preserve verified native file/split/cursor state across clean restart; explicit owner-controlled unsaved recovery. Keep actual official Neovim, project/path/privacy checks and no overwrites. |
| 17. Hunk Apply | Capture exact selected hunks against verified accepted artifacts/bases, owner confirm, conflict/buffer checks and one durable application receipt. No automatic replay or repeat staged write. |
| 18. Buffer proposals | Native Neovim proposals target pinned buffer identity/path/changedtick/text; explicit apply uses native undo and rejects stale/dirty conflicts. Agent isolated work must not overwrite live user buffers. |
| 19. Larger registries | Measure thousands of stored arbitrary nodes and tree navigation/fair selection before changing active capacity. Optimize measured bottlenecks, preserve default-two/max-four active envelope unless separately qualified. |
| 20. Extended qualification | Fresh bounded coding/document fixtures and meaningful denied/stale/concurrent/crash/restore cases, actual frontend/editor interaction, extended operation and evidence. No historical replay/counter reset or unbounded paid inference. |

## Implementation checkpoint

Work has started. Task 1 needs a real user-manager/reboot-capable host; the
[host qualification runbook](unattended-host-qualification.md) records the
separate acceptance sequence. No host logout or reboot was performed.

Tasks **2, 3 and 5 are complete and qualified on `f863b37028`**: 59 targeted
TUI/editor checks, production CLI Clippy/build/package and physical real CLI/
official-Neovim terminal interaction pass. Palette search/cancel and Console
discovery, goal editing/stale/cycle refusal, finite interval and exact-source
accepted-trigger forms, 36 layout switches at three sizes and native exit 0 pass.
Five installed asset hashes and SQLite quick/FK checks pass; zero model requests
or attempts ran. Both series and the future source grant were cancelled; the
owned daemon is stopped. Preserve `operations-batch/native/` and never rerun
its setup or physical mutation driver.

These features reuse existing owner handlers, native previews, protected
mutations and goal/task records; no dependency was added.

Task **4 is complete and qualified on `a3e0fcca67`**: a protected SQLite
preset table stores at most 128 captured subtree nodes, their role versions and
512 internal memberships within 128 KiB. Owner cloning creates fresh role/node
identities beneath a named group, remaps internal parents/memberships and advances
the preset revision with the last-clone mapping. Stale retries refuse; no grants,
tasks, runtime identity or private notes are copied. The original snapshot is
retained despite later source edits. TUI D saves a selected node; F9 browses
presets; V opens a version-pinned clone form. One projection enum replaces the
separate dashboard view flags.

Both new state checks, full state **295** and full protocol **373** pass.
The first state attempt exposed a test's dynamic SQL and mismatched model binding;
the broader run exposed three downgrade fixtures retaining the new table. The
tests now use literal SQL, matching captured model scopes and genuinely older
fixture schemas; original assertions remain. All 60 targeted TUI/editor checks,
production CLI Clippy/build, four schema comparisons and 25 Python SDK contract
checks pass. Python generation initially missed the installed Ruff executable;
activating the existing SDK virtual environment fixed its PATH without installing
anything. Physical native save/clone, immutable source snapshot, fresh identities,
internal-only memberships, unchanged original grant/task/knowledge boundary,
stale clone refusal and invalid-parent rollback pass. Native restart retains the
same preset revision and mapping. Both CLI exits are 0; SQLite quick/FK checks and
all five installed package hashes pass. Zero model requests or attempts ran;
the sole future source task was cancelled and the owned daemon stopped.
Preserve `operations-batch/presets/`; never replay its setup/physical decisions.
Two read-only restart harness failures (saved Console layout and an incorrect
asset list) are retained; corrected `restart-v3.json` passes. The initial final
format check exposed test-only formatting; it is corrected and the rerun passes.

Tasks **6–9 are complete and qualified on `753464cd53`**: project lifecycle,
priority/fair selection, global limits and emergency pause use versioned controls
and immutable control history so pause/priority do not invalidate execution
grants or reset original accounting. No other remaining task is qualified yet.
Subsequent slices update this page, status and testing with exact commits,
commands, actual interactions, failures and limits.
No push, deployment, external message, new integration, purchase or automatic
Apply is authorized by this local batch.

Controls checkpoint: all 301 state and 373 protocol checks pass, including six
new failure-sensitive checks for pause/late receipts, exact completion/history,
priority aging/40 paused candidates, root/descendant accounting, concurrent global
claims/requests and automatic workflow materialization. Legacy downgrade fixtures
now remove the new migration's views/tables/queue column before simulating older
schemas; assertions remain. The first TUI run exposed existing K/Q shortcut
collisions; project lifecycle is now F and priority !, retaining K navigation and
Q native selection. The corrected 62 targeted TUI/actual-Neovim checks, four
generated schema comparisons and 25 SDK contract checks pass. Missing global
usage from older daemons is explicitly unavailable. Production CLI Clippy with
warnings denied, CLI build and formatting pass. Fresh physical coding/document/
control qualification and restart pass: two immutable real subprocess checks,
exact owner acceptance, pause/refused approval/same-grant resume, completion/
archive/reopen, exhausted admission refusal, unchanged originals and accounting,
36 layout switches, two exits 0, installed asset hashes and SQLite integrity.
Four localhost responses/60 synthetic tokens/zero paid. Final global state is
paused; owned services are stopped. Preserve both accepted unapplied results and
the cancelled zero-usage ceiling probe. All 26 native PNG/text pairs are archived
under `/workspace/artifacts/cli-owner-controls/`. Next work is tasks 10–15;
their implementation is in progress, with no further item qualified yet.

Tasks 10–15 are implemented with native qualification pending. All 310 state,
373 protocol and 63 targeted TUI/official-Neovim checks pass. The corrected
discussion fixture releases completed attempt capacity, matching the real host.
The new finite policies, atomic checkpoint replans, typed discussions, scoped
knowledge retirement/history and captured scorecards are described in
[reviewed autonomy](reviewed-autonomy.md). Generated contracts, production
Clippy/package and fresh physical checks follow; tasks 16–20 remain in progress.

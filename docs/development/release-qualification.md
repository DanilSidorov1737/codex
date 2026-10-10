# MVP release qualification

B1–B4 are qualified locally. B5 is undergoing actual editor/application checks;
B6 release qualification is incomplete. Deterministic localhost provider tests
exercise the real packaged CLI, daemon, native sandbox, filesystem and immutable
owner checks; they do not establish live model quality or paid provider compatibility.

## Current scale measurement

The existing isolated daemon accepted 1,000 additional stored agent definitions,
including a 200-level arbitrary hierarchy, without model requests. It returned
1,002 total nodes in six actual owner API queries of at most 200 records. Read
latencies were 2.3–23.5 ms; mutation p95 was 21.5 ms. Evidence:
`/workspace/.onboarding/qualification/mvp-b6-cli/scale.json` and
`mvp-b6-scale-seed.log`. This is the existing B4 fixture backend's baseline;
current release TUI navigation, memory, four-active execution and restart
qualification remain pending. It is not a claim of 1,000 simultaneous executions.

## Outstanding release gates

- Integrated official Neovim physical editing, conflicts, explicit Apply and
  daemon independence while the editor closes/crashes.
- Configurable bounded two/four-active executions and fifth-work queuing.
- Current packaged CLI navigation timing and daemon idle/memory measurement.
- Coding and documentation fixture pilots, including delegated coding.
- Application-inclusive verified snapshot restore and uncertain-action recovery.
- Explicit bounded live provider/model pilot. No credentials or test-spend binding
  has been supplied; the earlier question remains pending.
- Cost estimates require owner-supplied provider rates and provenance. Reported
  token counts are evidence; unconfigured price must remain unavailable, not zero.

Use the A01–A21 criteria in [the MVP](mvp.md). Update this file with actual
commands, versions, results and limits; do not turn outstanding gates into claims.

## Additional completed local evidence

- Official Neovim inside the full packaged CLI passed 12 physical editor/Apply
  checks at 120×30 and 50×20. Exact artifact application passed its owner oracle;
  dirty buffers and changed originals created no application intent. SIGKILL of
  only the owned editor during a native 15-second command left the daemon alive
  and the task reached NeedsReview. Saved original files reopened. See B5 evidence.
- Single-agent addition repair, delegated addition repair and a document produced
  from immutable local facts passed native immutable regressions, owner acceptance
  and explicit application. Root grants used respectively 2/30, 7/105 and 2/30
  requests/reported tokens. Deterministic replies are not live-model evidence.
  The first fixture used a filename colliding with Python's built-in math module;
  its unchanged regression caught the failure. Fresh calculator fixtures passed;
  the original failure log and files remain. Logs: `mvp-b6-native-pilots.log` and
  `mvp-b6-native-pilots-corrected.log`; results: `mvp-b6-cli/pilots.json`.
- Actual default-two capacity qualified three native tasks: peak two active, the
  third queued, all immutable checks passed, each used two requests/30 reported
  tokens. `mvp-b6-concurrency-two.log`, `mvp-b6-cli/concurrency-2.json`.
- Actual 1,000-agent hierarchy PTY navigation passed 30 keys and a deep-node search:
  p95 30.9 ms, measured to 20 ms terminal-output quiescence. Full startup to loaded
  tree was 2.79 seconds; this is separate from first owner API page latency. TUI
  RSS 272,924 KiB and daemon RSS 314,640 KiB. Unoptimized dev-small, 120×30
  xterm-256color. `mvp-b6-tui-performance.log`, `mvp-b6-cli/tui-performance.json`.
- Actual CLI backup/fresh restore matched all 21 canonical tables, including
  1,006 nodes, tasks/grants/budgets and four completed application receipts; both
  integrity checks passed. `mvp-b6-cli/restore-qualification.json`.
- A real 30-second idle daemon interval generated no model requests and retained
  the same process. `mvp-b6-idle.log`, `mvp-b6-cli/idle.json`.

Four-active configuration and early Code Mode rejection are written but await
current-binary qualification. Protected owner quote estimates are written with
provider/model/source matching; the first check found a signed token-type mismatch,
which was corrected without panicking conversions. See [resource settings](resource-limits.md).
The final warnings-denied, combined unit, native API and full CLI pipeline is running.

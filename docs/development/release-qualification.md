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

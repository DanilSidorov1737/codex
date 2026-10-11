# Reusable bounded workflows (N3)

Owner approved with “ok do this” after the N3 proposal. Implementation is in
progress; this contract is not qualification evidence.

Templates save 1–32 structured plan items: stable keys, arbitrary configured
agent assignments, instructions, success criteria and prerequisite keys. They
are project-bound, revisioned owner records, with no executable grant. The
existing plan validation and SQLite transaction paths will be reused. No workflow
language, scheduler service, new dependency or predefined hierarchy is needed.

Each invocation starts with a read-only preview of the exact template revision,
project/agent revisions, per-step execution scopes and their summed token/request
ceilings. Confirmation submits this immutable preview with an owner-selected UUID.
The daemon revalidates it before atomically creating fresh tasks, prerequisites,
grants and the run record. Stale previews, duplicate run IDs, cycles, unknown keys,
missing/extra scopes and invalid grants fail without partially creating work.
An uncertain RPC outcome must be inspected by run ID before another invocation.

Each step retains its own existing root grant, including delegated usage, native
approval policy, model, paths, checks, requests, tokens, time and attempt limits.
Summed ceilings describe those fixed grants, not a new shared monetary budget.
Already-admitted model usage may exceed a token ceiling. Authorization expiry can
block a long human review; a new run requires a fresh explicit preview.

Prerequisites must be accepted by the owner before dispatch, not merely finish
model execution. Accepted artifacts may be copied into the dependent isolated
workspace only when explicitly declared as its inputs, from the same run, with
recorded hashes rechecked. Immutable inputs cannot be replaced by workflow outputs;
ambiguous overlapping producers are rejected. If an imported file is edited again, its producer's original base is carried
forward; a new project hash must not hide an intervening owner edit. Original
project files remain protected by existing Review/Apply checks. Acceptance and Apply are separate.
Failed/cancelled/interrupted prerequisites block dependents without automatic
retry, acceptance, Apply or authority expansion. Ordinary task reauthorization
must not reset an invocation's fixed limits.

CLI: paginated template/run listing, owner template mutation, preview and explicit
run mutation. TUI: save a selected task and its prerequisite closure as a template,
inspect templates/runs, preview scopes before starting, and open run tasks in the
existing layouts. Reuse the concept-art panels/palette and actual official Neovim.
CLI supports per-step scopes; the first UI form can apply one reviewed scope to
all steps and displays the resulting total ceilings and every task/criterion.

Qualification must cover invalid/cyclic graphs, atomic rollback, stale previews,
distinct invocations, dependency blocking, approval/budget denials, restart,
scoped artifact transfer, actual UI creation/confirmation/navigation, and actual
Neovim review. Use fresh synthetic state and deterministic localhost Responses;
do not replay N1/N2/Apply fixtures or automatically repeat paid pilots.

Calendar triggers and recurrent whole-workflow runs, structured handoff/reassignment,
parallel shared-buffer editing, distributed execution and automatic Apply are
later scope. Existing N1 memory remains available within each grant; N2 continues
to schedule its existing prerequisite-free task templates.

## Implementation checkpoints

Storage/protocol checkpoint: migration 8, revisioned templates, immutable previews,
atomic fresh task/grant creation, canonical run pagination and host-only scoped
artifact selection are implemented. Agent-plan graph validation is shared. Ordinary
reauthorization is refused for workflow roots/descendants. 372 protocol and 269
state tests passed, including four new workflow scenarios and migration from
versions 6/7. Native runtime/API/UI and physical qualification are in progress.
Evidence: `/workspace/.onboarding/qualification/workflows-n3/state-protocol-tests.log`.
Initial fixture failures are preserved in the qualification parent directory;
no production migration checksum, integrity or artifact-base check was weakened.

Owner API/CLI checkpoint: experimental `organization/workflow/list`, `/preview`
and `/mutate`, protected local-owner origin checks, and `workflow-list`,
`workflow-preview FILE`, `workflow-mutate FILE` are implemented. Template mutations
and run authorization use the detached daemon. Existing artifact import now accepts
same-invocation, explicitly declared prerequisite inputs, with one combined 64 MiB
input ceiling and existing acceptance/hash/immutable protection. Seven native owner
API tests, four schema comparisons and 25 Python contract/signature tests passed;
all generated JSON/TypeScript/precomputed/SDK contracts were refreshed. An initial
negative test used the success-only client helper: the server correctly denied the
request; the test now reads and asserts the actual JSON-RPC error (-32602).
Full CLI/UI build and physical workflow qualification remain in progress.

Dashboard checkpoint: Projects lists templates/runs; `w` opens workflow/task
navigation in Projects or Operations, `W` saves the selected task's prerequisite
closure, and `s` previews a selected template with a reviewed common per-step
scope. F6 requests a read-only canonical preview; `y` authorizes that exact frozen
payload. Late responses cannot reopen closed/changed drafts. Inspectors display
assignments, criteria, dependencies, fixed ceilings and canonical task states;
run tasks use existing approval/review/Neovim controls. The Operations hierarchy
remains the default. Thirty native TUI/official-Neovim tests passed, including
save/preview/frozen identity, late response and narrow/untrusted-content checks.
Physical interaction and package qualification are still pending.

Correctness checkpoint: prerequisite handoff now carries the producer's original
artifact base for every imported file edited again. Four native workspace tests
passed, including accepted-byte revalidation and preservation/refusal of an
intervening owner edit. The preview regression also rejects overlapping direct
producers; all four workflow state scenarios passed again. Physical conflict
qualification and the final package rebuild remain in progress.

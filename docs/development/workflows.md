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
ambiguous overlapping producers are rejected. Original project files remain
protected by existing Review/Apply checks. Acceptance and Apply are separate.
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

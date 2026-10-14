# Proposed data model and invariants

This is a logical design, not a migration or application implementation.
Use existing Serde/UUID/timestamp conventions. Store UTC instants; display the
user's configured timezone. Keep IDs stable through renames and reparenting.

## Entities

| Entity                       | Essential fields / relationships                                                                                                                     | Important rule                                                      |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| Organization                 | id, name, owner principal, root policy version, status                                                                                               | No required organizational template                                 |
| OrgNode                      | id, organization_id, parent_id nullable, label, kind agent/group, order, revision                                                                    | Primary acyclic forest; kinds do not encode hierarchy levels        |
| Agent                        | stable id, node_id, role_version_id optional, enabled, default provider/model, instruction overrides                                                 | Separate from live thread; retired identity preserved               |
| RoleVersion                  | id, role_id/name, version, instructions, capability defaults, provenance                                                                             | Arbitrary user role names, immutable execution snapshots            |
| Membership                   | node_id or agent_id, group_id, relationship label                                                                                                    | Extra team/project structure; membership is not automatic authority |
| PrincipalGrant / Policy      | subject, resource scope, actions, ceiling, policy version, issuer                                                                                    | Protected owner ceiling; deny overrides allow                       |
| Goal                         | id, organization_id, parent_goal_id, objective, owner, priority, success criteria, status, revision                                                  | Independent goal tree; no inference from chat                       |
| ProjectPlan                  | existing project_id, organization_id, captured name/roots, goal links, milestones, status                                                            | Extend existing projects; reconcile cross-store references          |
| Task                         | id, project_id optional, goal_id optional, parent_task_id optional, accountable owner, instructions, criteria, state, priority, not_before, revision | One accountable owner; parent task differs from prerequisite graph  |
| TaskDependency               | task_id, prerequisite_task_id                                                                                                                        | Acyclic graph; accepted prerequisites gate dispatch                 |
| WorkflowVersion              | id, template name/version, validated task/dependency definition                                                                                      | Later reusable DAG templates; no arbitrary embedded code DSL        |
| ExecutionAttempt             | id, task_id, agent_id, attempt_number, lease owner/expiry/generation, thread_id/turn_id, config/policy versions, checkpoint, outcome                 | New attempt on retry; stale generation cannot commit                |
| Message / Dispatch           | id, sender principal, recipients, task correlation, payload reference, delivery mode, status, dedupe key                                             | Authenticate sender; at-least-once delivery with dedupe             |
| ActionIntent                 | id, attempt_id, action type, structured parameters/hash, expected resource version, idempotency key, provider reference, status                      | Record before execution; unknown outcome is explicit                |
| Approval                     | id, action_id, policy version, request hash, approver, state, scope, expiry, decision evidence                                                       | Bind approval to exact action; no agent self-approval               |
| Budget / Reservation / Usage | scoped cap, unit/currency, period, reservation, actual usage, settlement, source                                                                     | Atomic spend admission; no floating-point money                     |
| Artifact                     | id, task/attempt, kind, path/reference, content hash, provenance, created_at                                                                         | Canonicalize local paths; credentials never stored here             |
| Evidence / Acceptance        | criteria ID, artifact hash, test run ID/command/exit/count, reviewer decision, accepted revision                                                     | Completion requires current evidence                                |
| Knowledge                    | id, scope, ACL, content/reference, provenance, sensitivity, confidence, freshness, supersedes                                                        | Not canonical task/permission state                                 |
| AuditEvent / Outbox          | sequence, actor, correlation IDs, type, versioned payload, timestamp, dispatch status                                                                | Commit with business transition; redact sensitive fields            |
| LayoutPreset                 | user/workspace, preset id, pane tree/sizes/visibility, focus and key overrides, schema version                                                       | UI settings; cannot change backend permissions                      |

MVP tables should include only fields needed for the approved slice. Workflow
templates, meetings, financial transaction ledgers and broad capability inventory
are future records, not scaffolding to create now.

## Hierarchy examples that must work

- One standalone agent with no supervisory agent.
- One user-named coordinator with ten agents.
- User-defined groups with arbitrary names and nesting, containing agents.
- Multiple top-level agents/groups; one agent belongs to several project teams.
- A large organization with sleeping persistent members and few active runs.

The UI can show linked projects/shared functions as a projection. They do not
become mandatory executing node types. Deleting a node with children requires an
explicit move/retire decision. Reparenting rejects cycles transactionally and
does not change policy grants automatically. Display paths are derived and can
change; stable IDs do not.

## State transitions

Task lifecycle:
`proposed -> queued -> running -> review -> completed`.
Approval-required proposals pass through approval before queuing.
Additional states: `blocked`, `paused`, `failed`, `cancelled`, `archived`.

- A running task may checkpoint to blocked/paused/review/failed/cancelled.
- An approved safe retry goes from failed/blocked back to queued with a new attempt.
- Review rejection/request changes creates tracked follow-up/requeue.
- Completion records acceptance; archive preserves history.
- Internal task review may be performed by deterministic checks only when the
  owner has preauthorized those exact criteria. Record checked evidence and the
  policy version; final project acceptance remains an owner action.
- Reopening completed work is explicit and invalidates dependent assumptions
  according to a recorded policy; never silently erase old acceptance.
- Pausing a subtree affects dispatch, requests cancellation/checkpoints for active
  work, and records unfinished/uncertain actions. It is not an instantaneous
  reversal of already completed external effects.

Attempt outcomes: queued/admitted/running/waiting/reconciling/succeeded/failed/
interrupted. Task review success and attempt technical success are different.
Agent configured/enabled/retired state differs from current run idle/running/
waiting status.

Action lifecycle:
`planned -> authorization_required/authorized -> dispatched -> verified`.
Other terminal/intermediate states: denied, failed, cancelled, unknown,
reconciling. After a crash, dispatched without evidence becomes unknown;
expiry of a task lease does not prove an action failed.

## Database constraints and query patterns

- Enforce foreign keys within the authoritative database, uniqueness of attempt
  numbers and dedupe keys, valid status transitions, and non-negative limits.
- Use a short write transaction/compare-and-swap on expected task revision and
  lease generation; persist related audit/outbox changes in that transaction.
- Reject organization and task dependency cycles; bounds protect pathological
  depth/size while remaining user configurable and independent of role labels.
- Index node parent/order, task project/state/priority/due time, dependency
  endpoints, attempt leases, message recipients and event sequence.
- Paginate large lists; fetch children lazily and update selected-node summaries.
- Distinguish persisted wall-clock expiry from monotonic in-process timeout;
  restart/clock changes need lease reconciliation.
- Do not silently rebuild lost canonical records from model output.

## Qualified audit coverage

The native organization SQLite store records definitions/versions, task/grant
lineage, attempts/leases, model admission and reported or unresolved usage, tool
intents/receipts, approvals, checkpoints, results/checks, owner acceptance and
Applications. Transaction-linked audit events reference these canonical records;
`organization execution-read <task-id>` and `organization report` expose linked
evidence. The current Owner overview connects mission/goal/project/task/grant/
result; it does not invent missing links or expose private model reasoning.

This is not a universal recording of everything on the host. Ordinary manual
terminal commands, editor keystrokes and unobserved external actions are outside
that ledger. Native reviewed owner Git commands retain separate private JSON
intents/receipts, and those files are separately backed up. The host owner can
modify local storage; an independent tamper-proof audit service is not claimed.
See security-and-recovery.md, owner-review-and-git.md and current qualification.

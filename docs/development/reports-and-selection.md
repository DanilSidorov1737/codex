# Action inbox, evidence reports and native selection requests

Expanded-batch items 7–11 reuse the protected owner connection, canonical SQLite
records and actual official Neovim. Implementation is under qualification.
No dependency, model summary service, integration probe or editor engine is added.

## Review and reporting

Review shows tasks with current pending approvals, explicit blockers, canonical
requested-change receipts, results needing review and failed/reconciliation
states. Titles cannot claim an inbox status. Select a task to inspect exact action
hashes, checkpoint IDs, result digests and checks; existing confirmation commands
retain their identity/revision fences. Resolving one entry never approves another.

`z` opens an on-demand local summary for the selected project, agent or workflow
run; `Z` covers the whole store. Scroll with arrows/PageUp/PageDown and close with
Escape. `codex organization report --project-id ID --limit 100 --cursor ID`
provides bounded JSON pages. Complete SQL totals are independent of pagination;
TUI readers refuse pages from different audit revisions. The UI admits at most
2,000 tasks, and usage detail is bounded to 2,000 groups per task page. Explicit
errors/truncation retain honest limits; CLI pagination supports larger stores.

Reports link task/attempt/checkpoint/result/change receipts to
`codex organization execution-read ID`, upcoming schedule IDs and native checks.
Attempt usage is counted once and attributed to the agent captured at claim,
original grant model and captured provider endpoint fingerprint. Historical
missing metadata stays unknown. Delegated attempts share one root ledger;
workflow descendants are linked through that root. Handoffs retain historical
agent attribution. Per-task, agent, project and workflow usage dimensions are
separate and must not be added together. Root remaining limits are shown once,
with per-task attempt limits/expiry/revocation. Already admitted usage may exceed
a token ceiling; unreported requests are not treated as free or zero.

Optional private owner price quotes supply labeled reported-token estimates for
matching provider/model only. No quote means unavailable. These estimates omit
unreported usage/non-token charges and are not invoices or hard billing caps.
CPU and RAM are not invented. Reports are local reads and trigger no model work.

## Configured capabilities and evidence

`C` opens the inventory; `codex organization inventory` returns JSON.
Providers show configuration/authentication requirements separately from recorded
request/usage/native-check evidence, models and attempt IDs. The selected provider
uses the effective configuration. An observation for another endpoint does not
qualify the current endpoint. Task failures are labeled task failures; they are
not interpreted as provider incompatibility. Default host-resolved endpoints stay
unknown. Endpoint evidence strips credentials/query/fragment and displays only
the safe origin; endpoint paths are hashed, not disclosed.

Native tool observations include completed, failed and uncertain effects plus
permission requirements. MCP server names/enabled flags are configuration only;
none are admitted to bounded tasks or tested by this inventory. Plugins/skills
and external integration qualification remain separate. No headers, credential
values, MCP launch arguments/env or probes/installations are returned/performed.

## Actual Neovim selection to proposed task

Open an original project file in Workspace (`o`), select characters (`v`) or lines
(`V`) in actual Neovim, then Ctrl-G for host focus and `Q` to propose a task.
Choose explain/refactor/debug/test, agent, instructions and criteria. F6 reviews
exact pinned content; `y` creates a proposed task. Authorization is a separate
existing decision. Escape cancels a draft or confirmation without starting work.

Neovim's native `getregion`/`getregionpos` capture Unicode, reverse and exclusive
ranges. Context pins the buffer ID/name/changedtick, modified flag, native one-based
line/byte-column positions, exact text and SHA256. Capture is limited to 512 lines /
12 KiB and regular files under the selected original project. Private previews,
other projects, block/virtual selections and stale/closed/renamed buffers are
refused. Semantic refusal keeps the editor usable. Buffer revision is checked at
both F6 and confirmation; project identity is fixed in the draft. Captured context
is untrusted content and grants no paths/tools. Original dirty buffers still block
Apply; immutable checks, acceptance and explicit file Apply remain mandatory.

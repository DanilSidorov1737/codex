# Pinned feedback, fresh saved-file checks and reviewed local Git

Approved daily-use tasks21–23 reuse versioned owner controls, the isolated native
check runner and native Git. Source `6bf3e34093` is qualified through selected
checks and actual packaged CLI/TUI/official-Neovim workflows. Choose packages
from current status/evidence; this is not the final forty-task release.

Select a current reviewable/accepted result and its output, then Ctrl-P →
**Comment on exact artifact revision / file / lines**. Enter one-based inclusive
lines (1–64 existing lines) and feedback, press F6, review the exact task revision,
result digest, file SHA256 and range, then `y`. Host rereads the exact UTF-8 artifact
before saving. Comments are immutable; create a follow-up rather than rewriting
history. The inspector shows first32 loaded comments and labels retained pins on
superseded results. Feedback is separate from approval, repair and acceptance.

`codex organization final-check --root /absolute/project --scope scope.json
--output /private/fresh-verification.json` snapshots saved owner files and runs
every captured argv through the existing Linux native sandbox. Declared outputs
must exist; every copied input/output is frozen read-only. Original project and
Codex state stay inaccessible to checks; credentials are not inherited. It
records current hashes/check outputs, compares snapshot and original hashes again,
and emits an independent owner verification. It creates no model request, task,
grant, backend result, acceptance or Apply. Unsaved editor buffers are excluded;
save intended final bytes first. Check `passed`, not merely process exit0.

Limits: 1,024 captured files,32 outputs,8 checks,64 arguments/check,8 MiB/file,
64 MiB total snapshot and30 seconds/check. Scope contains the existing execution
scope JSON shape as owner context; its model/budget fields create no authority.
Temporary check snapshots have no business/recovery references and are removed.
Output is a new private owner file; existing evidence is never overwritten.

`codex organization commit-preview --verification /private/verification.json
--message 'fix: concrete behavior' --output /private/fresh-preview.json` rechecks
all captured inputs/outputs, passing argv/evidence/digest, current native Git
HEAD/branch and an empty index. It captures only checked outputs differing from
HEAD, with exact bytes/message. Unrelated working changes remain separate.
The current bounded flow requires verification at the Git working-tree root.

Review that exact preview, then `codex organization commit-reviewed --preview
/private/preview.json --yes`. A private durable intent consumes the plan once
before Git staging. Staged paths/blobs, repository identity, resulting parent,
committed paths and committed bytes are checked against the preview. Local commit
only; no push or hook execution. Existing signing configuration and configured
clean filters are preserved: use the native owner terminal for those cases.

Nonempty indexes, stale files/base/branch, incomplete check pins, malformed/private
receipts, common secret filenames and recognized private-key/token signatures
are refused. The signature screen is bounded and is not a general secret detector;
inspect selected source/diff/message before confirmation. Credential/state paths
must stay outside declared outputs. Receipts under
`CODEX_HOME/organization-git-receipts/` retain completed or needsInspection state.
Timeouts/failures may leave staging or a commit; inspect HEAD/index and the receipt,
never automatically retry or reset the index. Native Git commands time out after
five seconds. A new owner plan requires a new explicit review; receipt deletion
is not a recovery procedure.

Keep private verification/preview files and `organization-git-receipts/` with
the separately retained owner project/configuration recovery files. The existing
business-state bundle does not claim to include these local owner JSON receipts;
restoring a database never authorizes another staging/commit or replays a plan.

Keep grants, counters, failed/superseded results and Applications separate from
these owner checks/Git receipts. Existing accepted-result checks never validate
changed owner bytes; only fresh checks describe the new saved-file snapshot.

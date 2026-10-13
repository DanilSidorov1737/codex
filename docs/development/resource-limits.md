# MVP execution limits and cost estimates

The default active execution limit is two. The owner may start/restart the daemon
with `CODEX_ORGANIZATION_MAX_ACTIVE=1`, `2`, `3` or `4`. The setting is captured
once in the daemon and its attempt guards; agent shells do not inherit it. Invalid
values disable consequential organization execution instead of increasing the
limit or silently falling back. Four resident root workflows and at most 32
bounded descendants per root remain separate limits; waiting parents yield active
capacity to children. Corrected four-active/fifth-queued and default-two/third-queued
native qualification passed after fixing the initial heartbeat/database contention;
see testing.md.

A model catalog entry declaring `code_mode_only` requires the explicit captured
Code Mode scope/current policy and the actual installed host before admission.
The adaptive release qualifies these controls using localhost model responses;
live Luna provider quality remains unqualified. Use a direct-tool model for the
inexpensive pilots. GPT-5.5 has earlier bounded paid evidence; the current daily-use
candidate adds verified GPT-5.4 Mini metadata and an actual paid TUI smoke.
No automatic model substitution or Code Mode bypass is implemented. Read
[Code Mode](code-mode.md) and [low-cost provider](low-cost-provider.md).

The daily-use candidate's optional provider `response_max_output_tokens` is a
per-response limit which includes reasoning output. An explicit limit is narrowed
to the captured task's token ceiling; omission preserves provider defaults. Native
HTTP/stream retries remain0 for bounded organization execution. Actual localhost
TUI requests carry37 and native task requests30 after narrowing; unsupported
providers fail normally rather than silently retrying without the cap. This does
not cap input/context, all task usage or provider invoices.

## Continuation limits (qualified)

Expanded-batch continuation keeps the original grant's
token/request counters, attempt ceiling and expiry. Every stopped source and fresh
receiver attempt consumes that same attempt allowance; handoff never creates a
replacement grant. Time ceilings remain per attempt, as in the existing runtime;
the fixed original expiry is never extended. Recurring usage and fixed workflow
step accounting remain attached to their original grants. Checkpoints share the
256 coordination-call attempt limit, with at most 32 checkpoints per task and a
combined 64 MiB permitted-file snapshot. See [continuity](coordination.md#approved-checkpoint-and-handoff-continuation).

## Optional owner price quote

On-demand `z`/`Z` summaries and `codex organization report` show own-attempt
requests/tokens/usage receipts by task, captured agent, project and workflow.
Descendants map through the original grant root; each shared root's remaining
limits are displayed once. Agent handoffs retain the earlier attempt's owner.
Historical unknown providers and unreported requests are labeled unknown.
Optional reported-token estimates match the captured provider/model to the
private owner quote below; different usage dimensions must not be summed.
Reports trigger no model requests. CPU/RAM and provider invoices are unavailable.

Create `<CODEX_HOME>/organization-prices.json` as an owner-controlled regular
single-link mode-0600 file. The TUI loads it when the dashboard opens. Symlinks,
hard links, unsafe ownership/permissions, oversized files, unsupported currency,
invalid rates and missing provider/model matches cannot supply an estimate.

Example **invented quote for demonstrating the format**, not provider pricing:

```json
{
  "provider": "my-configured-provider-id",
  "currency": "USD",
  "source": "Owner quote: provider rate page URL and retrieval date",
  "usd_per_million_total_tokens": { "my-selected-model": 2.0 }
}
```

Use the highest applicable input/output token rate for this conservative total-token
estimate. The inspector shows reported usage and the authorized token-ceiling
estimate with the exact owner quote's provider/model and source. Quotes must match
the configured provider and model; keep the daemon and frontend on the same verified
provider binding. A quote is an owner assumption, not a provider billing receipt.
Rates may change; reopen the dashboard after updating them. Missing or mismatched
quotes show **price unavailable**, never an invented zero cost.

This estimate excludes unreported usage, non-token tool/media charges and taxes.
It is not a monetary cap. The daemon enforces the actual token/request/time/attempt
and delegation ceilings; strict provider-billing guarantees require provider-side
controls and remain outside the first MVP. Do not invent real rates during tests.

## Admission evidence

Actual owner restart with limit 5 denied authorization before any canonical grant
or attempt and before inference/native effects; definition inspection remained
available. A real cached catalog declaring CodeModeOnly denied the task with zero
charged requests/tokens and no workspace. The same model name on a custom provider
without that metadata may use its explicit-provider fallback; no hardcoded model
name list or automatic model routing is imposed.

The real TUI displayed the local zero-billing quote's provider/model/source and
reported/token-ceiling values, plus the explicit billing limitations. Evidence:
`mvp-b6-cli/admission-model.json`, `admission-invalid-limit.json`,
`price-physical.json` and `tui-price.log`. Corrected package qualification passed;
real GPT-5.5 pilots subsequently passed, with verified official rates and a
$0.345433 token-cost estimate including all failures. The successful delegated
grant reported 25,101 tokens against a 25,000 ceiling because its final request
was already admitted. The request ceiling remained intact; estimates do not
guarantee provider billing. See release-qualification.md for full receipts/limits.

# MVP execution limits and cost estimates

The default active execution limit is two. The owner may start/restart the daemon
with `CODEX_ORGANIZATION_MAX_ACTIVE=1`, `2`, `3` or `4`. The setting is captured
once in the daemon and its attempt guards; agent shells do not inherit it. Invalid
values disable consequential organization execution instead of increasing the
limit or silently falling back. Four resident root workflows and at most 32
bounded descendants per root remain separate limits; waiting parents yield active
capacity to children. Four-active native qualification is still pending.

A model catalog entry declaring `code_mode_only` is rejected before creating its
managed thread or admitting inference. Code Mode is disabled in bounded MVP
execution. Use a model supporting direct native tools; gpt-5.5 is qualified against
the deterministic Responses-compatible fixture. This does not establish live
provider compatibility, and no automatic model routing is implemented.

## Optional owner price quote

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

# Captured USD admission estimates

Daily-use task8 is in progress. The new source adds an optional `costBudget` to
the existing execution scope, with no separate financial service or dependency.
Shared state validates it before creating a canonical grant. A quote captures
provider ID/endpoint fingerprint, model, rate revision/source, highest applicable
token rate, USD limit and a positive allowance for unknown/in-flight tokens.
All amounts use integer microdollars; admission rounds up the estimate of
`(maxTokens + unknownTokenAllowance) * usdMicrosPerMillionTokens / 1000000`.
Invalid, overflowing, mismatched-model, missing-provenance or over-limit quotes
fail before grant/outbox writes. Existing unpriced scopes omit the optional field.

Direct/profile authorization checks the actual configured provider and its
existing secret-free endpoint fingerprint. Runtime checks the binding again
before inference, covering restart and schedule/workflow materialization. The
root's existing token/request ledgers and uncertainty fences still enforce shared
delegation accounting; children inherit the exact quote rather than new money.
A quote is an immutable owner assumption, not independently obtained pricing.
Use reviewed official rates, their retrieval date and the highest applicable
input/output rate, including any selected regional uplift. Never infer a free
rate from missing information.

Example **invented owner quote for a local unit test**, not provider pricing:

```json
{
  "providerId": "local",
  "providerEndpointSha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
  "model": "fixture",
  "rateVersion": "synthetic-owner-v1",
  "rateSource": "Invented local test quote; not provider pricing",
  "usdMicrosPerMillionTokens": 5000000,
  "unknownTokenAllowance": 10,
  "maxUsdMicros": 550
}
```

For100 authorized tokens plus10 unknown tokens, this synthetic estimate is550
microdollars ($0.000550). Obtain the actual provider fingerprint from native
capability inventory; never copy the example fingerprint into a real grant.
Attach the object as `scope.costBudget` in existing owner execution JSON. The
advanced TUI scope form accepts the same optional JSON, preserves captured quotes
through profile/schedule prefill and displays the planned USD allowance in task
details. Model changes require a matching quote. Rates do not change existing
grants when the separate optional display-price file changes.

This is **per-root grant admission**, not a provider invoice cap or an aggregate
USD wallet. A request already admitted can exceed reported-token ceilings; the
unknown allowance is a conservative owner reserve, not a measured maximum or a
refund. Price changes, missing usage, taxes, tools/media and other non-token
charges remain outside the estimate. Separate plain TUI chat/title requests are
not organization grants and must be allowed for in pilot accounting. Schedule
occurrences materialize their own captured scopes; shared lifetime token/request
ceilings remain the series/workflow aggregate controls.

Current evidence:115 selected state checks pass, including atomic over-limit/
invalid quote refusal, integer rounding, retained provenance and legacy omission.
Cost-form selected TUI checks now also pass106, including actual Neovim and
captured quote preservation. The first provider-binding compile omitted propagation
of the existing fingerprint helper's Result; the fix and failed log are retained.
Stable/experimental contracts regenerate; schema comparisons4 and Python SDK
contracts5 pass. Production lint caught growth of the existing execution enum;
boxing only the optional quote reduces its footprint without changing JSON or
introducing a new abstraction or lint exemption. Production Clippy now passes
with warnings denied. Native CLI/TUI inventory exposes the safe endpoint binding;
legacy missing fingerprints remain unknown. The new clean-source53e37b68ab
package passes native quoted-task admission: over-budget and wrong-endpoint
quotes are refused before authorization, the valid captured quote is retained,
both native requests carry30-token output caps and the immutable check passes.
Four localhost requests/60 synthetic tokens/zero paid calls; normal TUI exit0
and owned daemon stop0. Priced unknown/delegation and paid-provider pilots remain
pending. Do not mark task8
complete or replay the already completed native-cap fixtures to qualify it.

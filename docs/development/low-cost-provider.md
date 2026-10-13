# Low-cost OpenAI provider qualification

The daily-use batch selects **GPT-5.4 Mini** explicitly; other models are not
silently substituted. Official model documentation, fetched2026-10-13:
<https://developers.openai.com/api/docs/models/gpt-5.4-mini.md>.
Private HTTP200/source SHA/rate evidence is under
`/workspace/.onboarding/qualification/daily-use-batch/model-docs/`.

Documented: Responses/streaming/native function calls/apply-patch, text/image
input, text output, none/low/medium/high/xhigh reasoning;400k total context,
272k maximum input and128k maximum output. The local descriptor conservatively
uses272k for usable input context. Input/cached/output rates are0.75/0.075/4.50
USD per million on the ordinary endpoint; regional processing has a10% uplift.
No browsing, external hosted tools or original-detail image capability is
inferred from this local coding qualification. Existing CodeModeOnly entries
and actual-host admission checks are unchanged.

GPT-5 Mini remains a separately observed legacy smoke model; current official
docs mark it deprecated. The new profile recommends GPT-5.4 Mini and never
falls back to the legacy model after a failure.

For a fresh private test home, use existing configuration support:

```toml
model = "gpt-5.4-mini"
model_provider = "bounded-openai"
model_catalog_json = "/workspace/codex/codex-rs/models-manager/models.json"
model_reasoning_effort = "low"
model_reasoning_summary = "none"
analytics.enabled = false

[model_providers.bounded-openai]
name = "OpenAI bounded qualification"
base_url = "https://api.openai.com/v1"
env_key = "OPENAI_API_KEY"
wire_api = "responses"
supports_websockets = false
request_max_retries = 0
stream_max_retries = 0
# Once using the new output-limit build, cap output including reasoning:
# response_max_output_tokens = 1200
```

The catalog path is this checkout's verified descriptor; a packaged installation
must retain its corresponding catalog separately. Configure the key through the
existing secure environment binding, never a value in TOML/auth fixtures/chat.
Choose captured native task paths/checks/requests/tokens/deadline/approval bounds
separately; provider configuration alone authorizes no organization task.

The actual official-Neovim terminal displayed a genuine GPT-5.4 Mini reply,
then `/quit` exited0 and the owned daemon stopped0. The verified baseline CLI
loaded this checkout's new catalog explicitly, without model fallback. Native
usage was10,966 input/21 output tokens, estimated$0.008319 at the recorded rates.
The initial missing required metadata field and a readiness predicate expecting
the model slug instead of its display name both failed before submission; their
captures are retained. These are not coding/delegation qualification.

The new optional provider setting `response_max_output_tokens` validates1–128000,
travels through remote configuration and HTTP/WebSocket Responses requests, and
counts reasoning output as well as visible text. The daemon narrows an explicit
cap to the captured task token ceiling. Omission preserves existing provider
defaults. A provider which rejects this Responses parameter fails normally;
there is no silent uncapped fallback. HTTP/stream retry limits already use native
provider settings; set both0 for finite paid pilots. This is a per-response output
cap, not an input/context, request-count, total-task or invoice cap.

Provider/configuration/model/API library checks pass641 tests. The core reuse
guard, production packaging and new-cap native request qualification are still
in progress. Preserve consumed/unknown usage and the remaining original $1 test
ceiling across every attempted live pilot.

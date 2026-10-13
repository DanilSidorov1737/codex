# Bounded Code Mode

Application source: `3eca8520e0`. Code Mode uses the existing actual Codex host,
with no substitute interpreter or in-process fallback for organization tasks.
The compatible direct-tool default stays available. CodeModeOnly models require
an explicitly captured Code Mode scope before any model request is admitted.

Use a complete Codex package containing the CLI, `codex-code-mode-host`, bwrap,
ripgrep and package manifest. Enable the actual host in the daemon configuration:

```toml
[features]
code_mode_host = true
```

Restart/update the owned daemon using the ordinary documented lifecycle; startup
alone does not replace an installed release. Verify the installed package before
authorizing work. In the task scope form, select Code Mode, choose the exact model
and retain the ordinary file/check/approval/time/token/request/attempt limits.
JSON scopes use `"codeMode": true` and policy version3; omitted/false remains the
legacy native policy2. Profiles/schedules/workflows capture the same scope and
cannot silently enable it or switch models.

The host allows one live cell per thread, a64-MiB cell heap and at most1-second
host waits. These are cell limits, not a claim about total organization memory.
The original active-execution limits remain default two /maximum four. Tool calls
inside cells use the same native scope, identity, approval and receipt checks.
Model admission waits without spending while a yielded native action awaits
approval/capacity. Cancellation is never treated as proof that an effect did not
run. Unresolved usage or dispatched/uncertain effects block fresh authorization;
an owner inspection record does not clear that fence.

Qualification includes real installed-host ambient-I/O denial, session isolation,
CPU interruption, exact native approvals/pause, original-project write denial,
immutable checks, budget/deadline/stale-owner/no-grant fences, awaited abandoned
cell cleanup and crash/no-replay/full restore. A catalog-matched GPT-6 Luna fixture
also passes real host/tool/check/owner acceptance/official-Neovim review. The
responses are deterministic localhost fixtures, not live Luna/provider-quality
or billing evidence. Historical live GPT pilots remain separately documented.

Read [adaptive evidence](adaptive-qualification.md) and [current status](status.md)
for final package, endurance and recovery qualification. Preserve failed scripts,
grants, counters and quarantines; never replay acceptance or Apply to complete a
test. Full unattended real-host reboot/logout qualification remains separate.

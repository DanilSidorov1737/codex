# Ponytail availability and setup plan

## What is available now

The original Phase0 session exposed `engineering-suite-ponytail` skills and
used its entry workflow/rules. In the current final daily-use session, both
cloud/executor skill catalogs are empty. The documented Ponytail reuse/review
process is applied; no standalone plugin/hook installation or trust is claimed.

The session bundle describes `ponytail-review` as a complexity-focused review
and uses `ponytail:` debt markers. Current upstream documentation differs:
the inspected plugin manifest is **5.1.0**, upstream review includes correctness,
security/load/test considerations, and debt markers are `shortcut:`.
Do not silently substitute current upstream behavior for the session bundle.

Authoritative upstream:
[DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail).
Observed HEAD: `9cc65d03aa2da1db7121b912d03596409ee340b8`.
Read its [README](https://github.com/DietrichGebert/ponytail/blob/9cc65d03aa2da1db7121b912d03596409ee340b8/README.md),
[INSTALL](https://github.com/DietrichGebert/ponytail/blob/9cc65d03aa2da1db7121b912d03596409ee340b8/INSTALL.md),
[manifest](https://github.com/DietrichGebert/ponytail/blob/9cc65d03aa2da1db7121b912d03596409ee340b8/.codex-plugin/plugin.json),
and [hook configuration](https://github.com/DietrichGebert/ponytail/blob/9cc65d03aa2da1db7121b912d03596409ee340b8/hooks/claude-codex-hooks.json).
These are research sources; marketing benchmark numbers are not evidence of
this fork's performance.

## Installation after separate authorization

The upstream documented Codex commands are:

```bash
codex plugin marketplace add DietrichGebert/ponytail
codex plugin add ponytail@ponytail
```

These commands were **not run** in Phase 0.

Before executing them:

1. Obtain explicit permission to install the selected version and trust hooks.
2. Inspect `codex plugin --help` and marketplace help in the actual built CLI.
   This source contains Marketplace/Add/List/Remove commands, but they were
   not executed without a full CLI binary.
3. Review the pinned upstream source/manifest/hook scripts, license (MIT), network
   and filesystem behavior, update policy and exact version selection mechanism.
   Use supported pinning or an inspected local marketplace if supported; do not
   invent a version-pinning CLI flag.
4. Make Node available to noninteractive hook execution. Node 24.19.0 is present
   in this cloud baseline; no additional installation is needed here.
5. Run the documented install only once authorized; open `/hooks`, inspect the
   actual configured events, and trust the intended commands explicitly.
6. Start a new thread; verify mode activation, skill listing, a simple reuse-first
   task and a review. Verify delegated session instructions too.
7. Record source commit/plugin version, commands, outcomes and removal path in
   status/testing docs.

Upstream INSTALL says “two lifecycle hooks,” but the inspected configuration
declares SessionStart, SubagentStart and UserPromptSubmit. This repository
recognizes those event names and supplies CLAUDE_PLUGIN_ROOT for compatibility.
Review what is actually installed rather than assuming the prose's count.
No hooks were trusted or executed in Phase 0.

Preserve existing AGENTS.md and local plugin/config files. Never replace root
instructions by copying upstream AGENTS.md wholesale. A manual skill installation
is an alternative only after reviewing the host's supported skill paths and
preserving current skills. This project already states the essential principles.

## Implementation and review rules

Apply the ladder: needed at all -> existing code -> standard library -> native
platform feature -> installed dependency -> smallest correct new code.
Inspect callers and real data/control flow first. Simplicity must not remove
validation, security, error handling, meaningful tests, accessibility, recovery
or an explicit user requirement.

Use Ponytail in every implementation task and complexity review. Also perform
correctness, security, integration and real TUI/editor review; the session bundle's
complexity review is not sufficient by itself. Annotate deliberate shortcuts with
the marker expected by the selected version, naming ceiling and upgrade trigger.
Do not create speculative scaffolding for thousands of active agents.

If Ponytail is absent in a future session, follow AGENTS.md's principles, report
the missing integration, and complete the authorized research/setup work.
Do not install a replacement without authorization.

Final daily-use review keeps the smallest coherent correction: existing
viewport arithmetic/iterators, BTreeSet/BTreeMap and palette/contrast helpers
replace hidden sidebar/per-dot work. No dependency, default-layout change,
state migration or new abstraction is added. Meaningful full-tree/search tests
and actual packaged10,005-node/Neovim/layout/recovery checks pass. Unnecessary
default-proportion code was withdrawn; existing saved pane controls are used.

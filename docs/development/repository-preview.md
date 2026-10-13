# Repository preview

`codex organization repository-preview --root /absolute/project --scope scope.json`
reads native Git branch/HEAD/dirty paths and an owner-requested execution scope.
It starts no daemon, model request, task or grant. Scope uses the existing
`OrganizationExecutionScope` JSON contract; the later authorization separately
validates captured identity, permissions, checks, immutable files and budgets.

The JSON includes repository/root paths, nullable HEAD (unborn repository),
nullable branch (detached HEAD), porcelain dirty status, requested input/immutable
paths, declared outputs, check argv and SHA256 bases for existing outputs. A
missing output base is null. Only declared output bodies are read; unrelated
paths can appear in Git status, but their contents are not read. It does not
read credentials or remote URLs, stage files, run hooks/checks or alter authority.

Limits: 1,024 inputs, 32 outputs, 8 MiB per output, 2,000 changed entries and
1 MiB Git-status output; native Git commands time out after five seconds.
Traversal, `.git` components, symlinked outputs and nonregular files are refused.
Non-UTF-8 Git paths are refused rather than ambiguously decoded. Git fsmonitor
and optional locking are disabled, and inherited Git root/index overrides are
removed for this command. Repository paths retain significant whitespace.

One native Git regression verifies dirty-byte/index preservation, exact output
hashes, a new-file base, unrelated-body isolation and traversal/symlink refusal.
Production lint/build/package and actual terminal CLI pass on clean be6cef7014.
Dirty/detached/unborn and whitespace-path cases pass; all refusal checks preserve
owner/index bytes. Detailed evidence is recorded in
[daily-use qualification](daily-use-qualification.md); do not assume a source
checkpoint is the packaged release. Dedicated reviewed commit and final-byte
verification remain separate daily-use tasks.

The next source checkpoint adds bounded check advice from the declared output
folders and file types. Rust, Python and configuration outputs receive relevant
check categories; every owner-captured argv remains present and required. This
is labeled file-type advice, not dependency analysis, test discovery or evidence
that any check ran. Required cross-component checks must be added, never dropped.
The native regression asserts unchanged captured argv and Python package advice.
Current package preview6 predates this additional advice.

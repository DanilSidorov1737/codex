# Adaptive batch qualification and safe resumption

Tasks 1–2 build and runner checkpoint; application changes are not yet qualified.
Evidence: `/workspace/.onboarding/qualification/adaptive-batch/`.

Activate `. /workspace/.onboarding/activate.sh`. Rust 1.95.0, pnpm 10.34.5,
actual official Neovim 0.12.6 remain installed. CI-pinned just 1.51.0 archive
SHA256 `c8f085ca3e885723c341d06243fc291b5abfdc8bbe3b2c076b117de490387b59`
was checked against Casey/just's official SHA256SUMS before extraction. Official
Facebook DotSlash v0.5.9 Ubuntu x86_64 archive SHA256
`ba6b6908c69db8264efdb8ee32f4906284ea3b77247c611339b5f60160cc5691`
was checked against the official release's asset digest before extraction.
GitHub API returned 403; the HTTPS release asset page supplied that digest.
Both binaries are outside the checkout in `.onboarding/tools/bin`. DotSlash
verifies the existing buildifier manifest's pinned download. No TLS/checksum
checks were weakened; no standalone Ponytail hooks were installed.

Initial full root format check failed only Rust import ordering. Correcting
25 existing files' imports makes `python3 scripts/format.py --check` pass (exit 0).
No application behavior changed in this formatting checkpoint. Initial/fix/pass
logs retain the stable-rustfmt warnings about nightly-only import options.
Use one `--locked --offline --profile dev-small` Cargo selection at a time,
`CARGO_INCREMENTAL=0 RUST_MIN_STACK=8388608`, cwd `codex-rs`. Check `df -h`
before cold compilation. All earlier fixture homes/packages/receipts are protected.

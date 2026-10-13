"""Version discovery for Codex packages."""

import tomllib

from .targets import REPO_ROOT


def read_workspace_version() -> str:
    cargo_toml = REPO_ROOT / "codex-rs" / "Cargo.toml"
    with open(cargo_toml, "rb") as fh:
        data = tomllib.load(fh)

    try:
        version = data["workspace"]["package"]["version"]
    except (KeyError, TypeError):
        raise RuntimeError(f"Could not find [workspace.package].version in {cargo_toml}")

    if not isinstance(version, str):
        raise RuntimeError(f"Could not find [workspace.package].version in {cargo_toml}")

    return version

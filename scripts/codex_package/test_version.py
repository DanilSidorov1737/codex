import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch


sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from codex_package import version


class ReadWorkspaceVersionTests(unittest.TestCase):
    def _write_cargo_toml(self, content: str) -> Path:
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        (root / "codex-rs").mkdir()
        (root / "codex-rs" / "Cargo.toml").write_text(content, encoding="utf-8")
        return root

    def test_reads_single_quoted_workspace_version(self):
        root = self._write_cargo_toml(
            """
[workspace.package]
version = '1.2.3'
"""
        )
        with patch.object(version, "REPO_ROOT", root):
            self.assertEqual(version.read_workspace_version(), "1.2.3")

    def test_ignores_other_tables(self):
        root = self._write_cargo_toml(
            """
[package]
version = "9.9.9"

[workspace.package]
name = "codex"
version = '2.3.4'
"""
        )
        with patch.object(version, "REPO_ROOT", root):
            self.assertEqual(version.read_workspace_version(), "2.3.4")

    def test_missing_or_non_string_version_raises_runtime_error(self):
        missing_root = self._write_cargo_toml(
            """
[workspace.package]
name = "codex"
"""
        )
        with patch.object(version, "REPO_ROOT", missing_root):
            with self.assertRaises(RuntimeError):
                version.read_workspace_version()

        non_string_root = self._write_cargo_toml(
            """
[workspace.package]
version = 123
"""
        )
        with patch.object(version, "REPO_ROOT", non_string_root):
            with self.assertRaises(RuntimeError):
                version.read_workspace_version()


if __name__ == "__main__":
    unittest.main()

"""Meaningful actual-editor checks; run with the pinned nvim on PATH."""

import os
from pathlib import Path
import shutil
import sys
import tempfile
import unittest

from organization_terminal import Terminal


class ActualTerminalTests(unittest.TestCase):
    def test_ready_unicode_input_and_real_exit(self):
        nvim = os.environ.get("CODEX_TEST_NVIM") or shutil.which("nvim")
        self.assertIsNotNone(nvim, "activate the actual pinned Neovim before testing")
        with tempfile.TemporaryDirectory() as directory:
            terminal = Terminal(nvim, Path(directory) / "evidence")
            try:
                terminal.start(
                    [sys.executable, "-u", "-c", "print('READY');print('OBSERVED:'+input())"],
                    directory,
                )
                terminal.wait_for(lambda screen: "READY" in screen)
                terminal.submit("native-Ω")
                terminal.wait_for(lambda screen: "OBSERVED:native-Ω" in screen)
                self.assertEqual(terminal.wait_exit(), 0)
                terminal.capture("actual-roundtrip")
                with self.assertRaises(FileExistsError):
                    terminal.capture("actual-roundtrip")
            finally:
                terminal.close()

    def test_missing_visible_result_fails_with_capture(self):
        nvim = os.environ.get("CODEX_TEST_NVIM") or shutil.which("nvim")
        self.assertIsNotNone(nvim)
        with tempfile.TemporaryDirectory() as directory:
            evidence = Path(directory) / "evidence"
            terminal = Terminal(nvim, evidence)
            try:
                terminal.start([sys.executable, "-u", "-c", "print('READY');input()"], directory)
                terminal.wait_for(lambda screen: "READY" in screen)
                with self.assertRaises(TimeoutError):
                    terminal.wait_for(lambda screen: "missing reply" in screen, timeout=0.2)
                self.assertEqual(len(list(evidence.glob("timeout-*.txt"))), 1)
                with self.assertRaises(TimeoutError):
                    terminal.wait_exit(timeout=0.2)
                terminal.send("done\r")
                self.assertEqual(terminal.wait_exit(), 0)
            finally:
                terminal.close()


if __name__ == "__main__":
    unittest.main()

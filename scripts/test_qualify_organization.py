import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "qualification", Path(__file__).with_name("qualify_organization.py")
)
qualification = importlib.util.module_from_spec(spec)
spec.loader.exec_module(qualification)


class QualificationTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.repo = self.root / "repo"
        self.repo.mkdir()
        subprocess.run(["git", "init", "-q", str(self.repo)], check=True)
        (self.repo / "source.txt").write_text("source")
        self.evidence = self.root / "receipts"
        self.plan_path = self.root / "plan.json"
        self.counter = self.root / "effects.txt"
        self.step = {
            "id": "effect",
            "kind": "action",
            "cwd": str(self.repo),
            "argv": [
                sys.executable,
                "-c",
                "from pathlib import Path; p=Path("
                + repr(str(self.counter))
                + "); p.write_text(p.read_text()+'x' if p.exists() else 'x')",
            ],
        }
        self.write_plan()

    def write_plan(self, steps=None, inputs=None):
        self.plan_path.write_text(
            json.dumps(
                {
                    "repository": str(self.repo),
                    "steps": steps or [self.step],
                    "inputs": inputs or [],
                }
            )
        )

    def run_plan(self, retry=False):
        return qualification.run(self.plan_path, self.evidence, retry)

    def test_completed_action_is_skipped_and_log_is_bound(self):
        self.assertEqual(self.run_plan(), 0)
        self.assertEqual(self.run_plan(), 0)
        self.assertEqual(self.counter.read_text(), "x")
        (self.evidence / "effect.log").write_text("changed")
        with self.assertRaisesRegex(ValueError, "log changed"):
            self.run_plan()
        self.assertEqual(self.counter.read_text(), "x")

    def test_failed_action_is_never_replayed_even_with_retry(self):
        self.step["expectedExit"] = 7
        self.write_plan()
        self.assertEqual(self.run_plan(), 1)
        with self.assertRaisesRegex(ValueError, "replay is forbidden"):
            self.run_plan(True)
        self.assertEqual(self.counter.read_text(), "x")

    def test_crashed_action_intent_blocks_replay(self):
        self.run_plan()
        path = self.evidence / "effect.json"
        receipt = json.loads(path.read_text())
        receipt["state"] = "started"
        qualification.save(path, receipt)
        with self.assertRaisesRegex(ValueError, "replay is forbidden"):
            self.run_plan(True)
        self.assertEqual(self.counter.read_text(), "x")

    def test_timeout_is_uncertain_and_never_replayed(self):
        self.step["argv"][2] += "; import time; time.sleep(1)"
        self.step["timeoutSeconds"] = 0.1
        self.write_plan()
        self.assertEqual(self.run_plan(), 1)
        receipt = json.loads((self.evidence / "effect.json").read_text())
        self.assertEqual(receipt["state"], "uncertain")
        with self.assertRaisesRegex(ValueError, "replay is forbidden"):
            self.run_plan(True)
        self.assertEqual(self.counter.read_text(), "x")

    def test_source_plan_and_input_changes_refuse_resume(self):
        external = self.root / "input.txt"
        external.write_text("pinned")
        self.write_plan(inputs=[str(external)])
        self.run_plan()
        external.write_text("drift")
        with self.assertRaisesRegex(ValueError, "input changed"):
            self.run_plan()
        external.write_text("pinned")
        (self.repo / "source.txt").write_text("changed")
        with self.assertRaisesRegex(ValueError, "source, plan"):
            self.run_plan()
        (self.repo / "source.txt").write_text("source")
        self.step["argv"].append("changed")
        self.write_plan(inputs=[str(external)])
        with self.assertRaisesRegex(ValueError, "source, plan"):
            self.run_plan()
        self.assertEqual(self.counter.read_text(), "x")

    def test_retry_readonly_check_preserves_failed_log(self):
        self.step["kind"] = "check"
        self.step["argv"] = [
            sys.executable,
            "-c",
            "from pathlib import Path; import sys; p=Path("
            + repr(str(self.counter))
            + "); print('check'); sys.exit(0 if p.exists() else 1)",
        ]
        self.write_plan()
        self.assertEqual(self.run_plan(), 1)
        self.counter.write_text("external fix")
        self.assertEqual(self.run_plan(True), 0)
        self.assertEqual((self.evidence / "effect.log").read_text(), "check\ncheck\n")

    def test_evidence_inside_source_and_public_directory_denied(self):
        self.evidence = self.repo / "receipts"
        with self.assertRaisesRegex(ValueError, "outside"):
            self.run_plan()
        self.evidence = self.root / "public"
        self.evidence.mkdir(mode=0o755)
        os.chmod(self.evidence, 0o755)
        with self.assertRaisesRegex(ValueError, "private"):
            self.run_plan()


if __name__ == "__main__":
    unittest.main()

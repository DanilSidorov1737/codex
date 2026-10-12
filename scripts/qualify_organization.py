#!/usr/bin/env python3
"""Run a finite argv plan with durable receipts; never replay uncertain actions."""

import argparse
import fcntl
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time


def digest(value):
    return hashlib.sha256(value).hexdigest()


def encoded(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode()


def save(path, value):
    """Publish a receipt only after its contents and directory are durable."""
    temporary = path.with_suffix(".tmp")
    with open(temporary, "xb") as stream:
        os.chmod(temporary, 0o600)
        stream.write(encoded(value))
        stream.flush()
        os.fsync(stream.fileno())
    os.replace(temporary, path)
    directory = os.open(path.parent, os.O_RDONLY | os.O_DIRECTORY)
    try:
        os.fsync(directory)
    finally:
        os.close(directory)


def fingerprint(repo, inputs):
    paths = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=repo,
    ).split(b"\0")
    files = {}
    for raw in paths:
        if raw:
            path = repo / os.fsdecode(raw)
            files[os.fsdecode(raw)] = (
                digest(path.read_bytes()) if path.is_file() else None
            )
    return {
        "source": digest(encoded(files)),
        "inputs": {str(path): digest(path.read_bytes()) for path in inputs},
    }


def run(plan_path, evidence, retry_checks=False):
    plan = json.loads(plan_path.read_text())
    repo = Path(plan["repository"]).resolve(strict=True)
    steps = plan["steps"]
    if not isinstance(steps, list) or not 1 <= len(steps) <= 256:
        raise ValueError("plan needs 1–256 finite steps")
    ids = set()
    for step in steps:
        name = step["id"]
        if not re.fullmatch(r"[a-zA-Z0-9_-]{1,80}", name) or name in ids:
            raise ValueError("step IDs must be unique safe filenames")
        ids.add(name)
        if step["kind"] not in ("check", "action"):
            raise ValueError("step kind must be check or action")
        argv = step["argv"]
        if (
            not isinstance(argv, list)
            or not argv
            or not all(isinstance(arg, str) and "\0" not in arg for arg in argv)
        ):
            raise ValueError("steps require explicit argv, never shell commands")
        Path(step["cwd"]).resolve(strict=True)
        if not 0 < step.get("timeoutSeconds", 300) <= 7200:
            raise ValueError("step timeout must be 1–7200 seconds")
    if evidence.is_symlink():
        raise ValueError("evidence must be a real directory")
    evidence = evidence.resolve()
    # Keep source fingerprints independent of mutable test receipts.
    if evidence.is_relative_to(repo):
        raise ValueError("evidence must be outside the repository")
    evidence.mkdir(mode=0o700, parents=True, exist_ok=True)
    if evidence.is_symlink() or evidence.stat().st_mode & 0o077:
        raise ValueError("evidence must be a private real directory")
    identity = {
        "plan": digest(encoded(plan)),
        **fingerprint(
            repo, [Path(p).resolve(strict=True) for p in plan.get("inputs", [])]
        ),
    }
    lock_path = evidence / "run.lock"
    with open(lock_path, "a") as lock:
        os.chmod(lock_path, 0o600)
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        manifest = evidence / "manifest.json"
        if manifest.exists():
            if json.loads(manifest.read_text()) != identity:
                raise ValueError(
                    "source, plan or input changed; preserve this run and use a new directory"
                )
        else:
            save(manifest, identity)
        for step in steps:
            receipt_path = evidence / (step["id"] + ".json")
            history = []
            if receipt_path.exists():
                receipt = json.loads(receipt_path.read_text())
                if receipt["state"] == "passed":
                    log = evidence / (step["id"] + ".log")
                    if receipt.get("step") != step or receipt.get(
                        "logSha256"
                    ) != digest(log.read_bytes()):
                        raise ValueError(step["id"] + ": receipt or log changed")
                    print(step["id"] + ": already passed", flush=True)
                    continue
                if step["kind"] == "action" or not retry_checks:
                    raise ValueError(
                        step["id"]
                        + ": incomplete/failed receipt; action replay is forbidden"
                    )
                history = receipt.get("previousReceipts", []) + [
                    {
                        key: value
                        for key, value in receipt.items()
                        if key != "previousReceipts"
                    }
                ]
            receipt = {
                "step": step,
                "state": "started",
                "startedAt": time.time(),
                "previousReceipts": history,
            }
            save(receipt_path, receipt)
            # Append attempts so retrying a read-only check preserves its failures.
            log_path = evidence / (step["id"] + ".log")
            try:
                with open(log_path, "ab") as log:
                    os.chmod(log_path, 0o600)
                    result = subprocess.run(
                        step["argv"],
                        cwd=step["cwd"],
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=step.get("timeoutSeconds", 300),
                        check=False,
                    )
                    log.flush()
                    os.fsync(log.fileno())
                receipt.update(
                    exitCode=result.returncode,
                    state="passed"
                    if result.returncode == step.get("expectedExit", 0)
                    else "failed",
                )
            except (OSError, subprocess.TimeoutExpired) as error:
                receipt.update(
                    state="uncertain" if step["kind"] == "action" else "failed",
                    error=str(error),
                )
            receipt.update(
                finishedAt=time.time(),
                logSha256=digest(log_path.read_bytes()) if log_path.exists() else None,
            )
            save(receipt_path, receipt)
            print(step["id"] + ": " + receipt["state"], flush=True)
            if receipt["state"] != "passed":
                return 1
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("plan", type=Path)
    parser.add_argument("evidence", type=Path)
    parser.add_argument("--retry-checks", action="store_true")
    args = parser.parse_args()
    try:
        return run(args.plan, args.evidence, args.retry_checks)
    except (ValueError, OSError, KeyError) as error:
        print(str(error), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())

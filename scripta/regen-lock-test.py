#!/usr/bin/env python3
"""Fixture tests for cista/scripta/regen-lock.

Proves D4 of component-release-streamline Stage 3 for the cista lock: --check
verifies freshness without writing; regen runs the cargo step through a mock
(never a real workspace lock update, never a test suite). The stale-lock
negative covers F2. Cista is a standalone package (no [workspace] section).
"""

from __future__ import annotations

import importlib.machinery
import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("regen-lock")


def load_module():
    loader = importlib.machinery.SourceFileLoader("cista_regen_lock", str(SCRIPT))
    spec = importlib.util.spec_from_loader("cista_regen_lock", loader)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot load {SCRIPT}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    loader.exec_module(module)
    return module


def build_fixture(root: Path, *, stale: bool = False) -> None:
    # cista: standalone [package] at the repo root, no [workspace].
    (root / "Cargo.toml").write_text(
        '[package]\nname = "cista"\nversion = "0.1.0"\nedition = "2021"\n\n'
        '[dependencies]\n',
        encoding="utf-8",
    )
    (root / "Cargo.lock").write_text(
        'version = 4\n\n'
        '[[package]]\nname = "cista"\n'
        f'version = "{"0.0.9" if stale else "0.1.0"}"\n'
        '\n[[package]]\nname = "hygiene-ratchet"\nversion = "0.1.0"\n',
        encoding="utf-8",
    )


class CistaRegenLockTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.module = load_module()

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def run_script(self, *argv: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), "--root", str(self.root), *argv],
            check=False,
            capture_output=True,
            text=True,
        )

    def test_check_passes_on_fresh_lock(self) -> None:
        build_fixture(self.root)
        res = self.run_script("--check")
        self.assertEqual(res.returncode, 0, res.stdout + res.stderr)

    def test_check_fails_on_stale_lock(self) -> None:
        build_fixture(self.root, stale=True)
        res = self.run_script("--check")
        self.assertNotEqual(res.returncode, 0)
        self.assertIn("cista", res.stderr)

    def test_check_never_writes(self) -> None:
        build_fixture(self.root, stale=True)
        before = (self.root / "Cargo.lock").read_text(encoding="utf-8")
        self.run_script("--check")
        after = (self.root / "Cargo.lock").read_text(encoding="utf-8")
        self.assertEqual(before, after)

    def test_regen_runs_cargo_offline_and_verifies(self) -> None:
        build_fixture(self.root, stale=True)
        calls: list[list[str]] = []

        def fake_cargo(argv: list[str], **kwargs) -> object:
            calls.append(argv)
            (self.root / "Cargo.lock").write_text(
                'version = 4\n\n[[package]]\nname = "cista"\nversion = "0.1.0"\n'
                '\n[[package]]\nname = "hygiene-ratchet"\nversion = "0.1.0"\n',
                encoding="utf-8",
            )
            return subprocess.CompletedProcess(argv, 0, "", "")

        self.module.subprocess_run = fake_cargo
        code, message = self.module.regen(self.root, "cargo", offline=True)
        self.assertEqual(code, 0, message)
        self.assertEqual(calls, [["cargo", "update", "--offline"]])

    def test_regen_reports_cargo_failure(self) -> None:
        build_fixture(self.root)

        def fake_cargo(argv: list[str], **kwargs) -> object:
            return subprocess.CompletedProcess(argv, 1, "", "index error")

        self.module.subprocess_run = fake_cargo
        code, message = self.module.regen(self.root, "cargo", offline=True)
        self.assertEqual(code, 1)
        self.assertIn("cargo update failed", message)


if __name__ == "__main__":
    unittest.main()

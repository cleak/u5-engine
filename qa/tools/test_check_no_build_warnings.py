#!/usr/bin/env python3
"""Unit test for `check_no_build_warnings.sh`, the CI warning-line gate.

Run with `python3 -m unittest qa/tools/test_check_no_build_warnings.py` from
the repository root (or `python3` on this file). Each case points the script's
`CARGO` at a stand-in that prints a canned log and exits with a chosen status;
nothing is compiled.

Design 1.3.13: build with `RUSTFLAGS=-Dwarnings`, then fail on any output line
matching `^\\s*warning(\\[..\\])?:`, which catches the `cargo:warning` lines that
`-D warnings` does not.
"""

import os
import pathlib
import stat
import subprocess
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parent / "check_no_build_warnings.sh"

CLEAN_LOG = """\
   Compiling u5-runtime v1.0.1 (/src/crates/u5-runtime)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.00s
"""


class WarningLineGate(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.directory.name)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def run_gate(self, log: str, exit_status: int = 0, *args: str):
        """Runs the gate against a fake cargo; returns (status, output, calls)."""
        (self.root / "log.txt").write_text(log, encoding="utf-8")
        calls = self.root / "calls.txt"
        fake = self.root / "cargo"
        fake.write_text(
            "#!/usr/bin/env bash\n"
            f'echo "RUSTFLAGS=$RUSTFLAGS $*" >> "{calls}"\n'
            f'cat "{self.root / "log.txt"}"\n'
            f"exit {exit_status}\n",
            encoding="utf-8",
        )
        fake.chmod(fake.stat().st_mode | stat.S_IXUSR)
        env = dict(os.environ, CARGO=str(fake))
        env.pop("RUSTFLAGS", None)
        result = subprocess.run(
            ["bash", str(SCRIPT), *args],
            capture_output=True,
            text=True,
            env=env,
            check=False,
        )
        recorded = calls.read_text(encoding="utf-8").splitlines() if calls.exists() else []
        return result.returncode, result.stdout + result.stderr, recorded

    def test_clean_build_passes(self) -> None:
        status, output, _ = self.run_gate(CLEAN_LOG)
        self.assertEqual(status, 0, output)
        self.assertIn("No build warnings.", output)

    def test_default_runs_workspace_and_visual_builds_with_warnings_denied(self) -> None:
        _, _, calls = self.run_gate(CLEAN_LOG)
        self.assertEqual(
            calls,
            [
                "RUSTFLAGS=-Dwarnings build --locked --workspace --all-targets",
                "RUSTFLAGS=-Dwarnings build --locked -p u5-tui --features visual --all-targets",
            ],
        )

    def test_arguments_replace_the_default_builds(self) -> None:
        _, _, calls = self.run_gate(CLEAN_LOG, 0, "-p", "u5-runtime")
        self.assertEqual(calls, ["RUSTFLAGS=-Dwarnings build --locked -p u5-runtime"])

    def test_build_script_warning_fails_although_cargo_succeeds(self) -> None:
        log = CLEAN_LOG + "warning: u5-tui@1.0.1: icon resource not embedded\n"
        status, output, _ = self.run_gate(log)
        self.assertEqual(status, 1, output)
        self.assertIn("printed warning lines", output)

    def test_coded_and_indented_warning_lines_fail(self) -> None:
        for line in ("warning[E0602]: unknown lint\n", "   warning: unused manifest key\n"):
            with self.subTest(line=line):
                status, output, _ = self.run_gate(CLEAN_LOG + line)
                self.assertEqual(status, 1, output)

    def test_coloured_warning_line_fails(self) -> None:
        log = CLEAN_LOG + "\x1b[1m\x1b[33mwarning\x1b[0m\x1b[1m:\x1b[0m unused import\n"
        status, output, _ = self.run_gate(log)
        self.assertEqual(status, 1, output)

    def test_warning_text_mid_line_passes(self) -> None:
        log = CLEAN_LOG + "   Compiling warning-free v0.1.0\nnote: no warning: here\n"
        status, output, _ = self.run_gate(log)
        self.assertEqual(status, 0, output)

    def test_failed_build_fails(self) -> None:
        log = "error: unused variable: `x`\n"
        status, output, _ = self.run_gate(log, 101)
        self.assertEqual(status, 1, output)
        self.assertIn("failed with -Dwarnings (exit 101)", output)

    def test_cargo_stop_note_is_not_listed_as_a_warning(self) -> None:
        log = "error: unused variable: `x`\nwarning: build failed, waiting for other jobs to finish...\n"
        status, output, _ = self.run_gate(log, 101, "-p", "u5-runtime")
        self.assertEqual(status, 1, output)
        self.assertNotIn("printed warning lines", output)


if __name__ == "__main__":
    unittest.main()

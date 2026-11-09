#!/usr/bin/env python3
"""Unit test for `paired_suite.py`'s parity lock (Phase B design 1.3.11).

Run with `python3 -m unittest qa/tools/test_paired_suite_parity_lock.py` from
the repository root (or `python3` on this file). The artifacts are synthetic
temporary folders; nothing is launched.

The suite exports `U5_PARITY_LOCK=1` beside its other exports, and a run
counts only when the engine's `engine.log` carries the exact lock line.
"""

import inspect
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import paired_suite  # noqa: E402


class ParityLock(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.artifact = pathlib.Path(self.directory.name)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def log(self, text: str) -> str:
        (self.artifact / "engine.log").write_text(text)
        return str(self.artifact)

    def test_export_sets_the_variable(self) -> None:
        environ = {"OTHER": "kept"}
        paired_suite.export_parity_lock(environ)
        self.assertEqual(environ, {"OTHER": "kept", "U5_PARITY_LOCK": "1"})

    def test_main_exports_before_any_run(self) -> None:
        source = inspect.getsource(paired_suite.main)
        export = source.index("export_parity_lock()")
        self.assertLess(export, source.index("run_scenario(cmd, env)"))
        self.assertLess(export, source.index("os.environ[EMULATOR_LOAD_LATENCY_ENV]"))

    def test_scenario_environment_carries_the_lock(self) -> None:
        scenario = self.artifact / "pinned.tsv"
        scenario.write_text("# fixed-clock: 12:00:00.00\nboth\tkey\tj\n")
        base = {}
        paired_suite.export_parity_lock(base)
        env, _notes = paired_suite.scenario_environment(scenario, base)
        self.assertEqual(env["U5_PARITY_LOCK"], "1")

    def test_the_exact_line_confirms(self) -> None:
        artifact = self.log(
            "launching\r\nenhancements: parity-locked (all off)\r\nwindow up\n"
        )
        self.assertTrue(paired_suite.parity_lock_confirmed(artifact))

    def test_anything_else_does_not(self) -> None:
        for text in (
            "",
            "enhancements: classic\n",
            "enhancements: custom (display.grow_left)\n",
            "warning: enhancements: parity-locked (all off)\n",
            "enhancements: parity-locked (all off) and more\n",
        ):
            self.assertFalse(paired_suite.parity_lock_confirmed(self.log(text)), text)

    def test_missing_log_or_artifact_does_not(self) -> None:
        self.assertFalse(paired_suite.parity_lock_confirmed(str(self.artifact)))
        self.assertFalse(paired_suite.parity_lock_confirmed(""))

    def test_the_line_matches_the_engine(self) -> None:
        engine = (
            pathlib.Path(__file__).resolve().parents[2]
            / "crates/u5-tui/src/enhancement_settings.rs"
        ).read_text()
        self.assertIn(f'PARITY_LOCK_LINE: &str = "{paired_suite.PARITY_LOCK_LINE}";', engine)
        self.assertIn(f'PARITY_LOCK_ENV: &str = "{paired_suite.PARITY_LOCK_ENV}";', engine)


if __name__ == "__main__":
    unittest.main()

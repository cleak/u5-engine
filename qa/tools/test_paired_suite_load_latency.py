#!/usr/bin/env python3
"""Unit test for `paired_suite.py`'s emulator-profile load latency setting.

Run with `python3 -m unittest qa/tools/test_paired_suite_load_latency.py` from
the repository root (or `python3` on this file). The scenarios are synthetic
temporary files; nothing is launched.

`intro.md` Section 7, "Timing of the load path": the latency is "a test
setting, not a game rule". A scenario may pin it with an
`# emulator-load-latency: MS` header, the way `# fixed-clock:` pins the clock,
and with neither header the run inherits the suite's environment unchanged.
"""

import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import paired_suite  # noqa: E402


class EmulatorLoadLatencyHeader(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.directory.name)

    def tearDown(self) -> None:
        self.directory.cleanup()

    def scenario(self, name: str, header: str) -> pathlib.Path:
        path = self.root / f"{name}.tsv"
        path.write_text(header + "both\tkey\tj\n")
        return path

    def test_header_is_read_as_whole_milliseconds(self) -> None:
        path = self.scenario("pinned", "# emulator-load-latency: 280\n")
        self.assertEqual(paired_suite.scenario_emulator_load_latency(path), "280")

    def test_malformed_or_absent_header_pins_nothing(self) -> None:
        for header in ("", "# emulator-load-latency: 0.28\n", "# emulator-load-latency: 280ms\n"):
            path = self.scenario("other", header)
            self.assertIsNone(paired_suite.scenario_emulator_load_latency(path), header)

    def test_no_header_inherits_the_suite_environment(self) -> None:
        path = self.scenario("plain", "")
        env, notes = paired_suite.scenario_environment(path, {"U5_EMULATOR_LOAD_LATENCY_MS": "100"})
        self.assertIsNone(env)
        self.assertEqual(notes, [])

    def test_header_wins_over_the_flag_and_keeps_the_rest(self) -> None:
        path = self.scenario(
            "both-headers",
            "# fixed-clock: 12:00:00.00\n# emulator-load-latency: 280\n",
        )
        base = {"U5_EMULATOR_LOAD_LATENCY_MS": "100", "OTHER": "kept"}
        env, notes = paired_suite.scenario_environment(path, base)
        self.assertEqual(env["U5_EMULATOR_LOAD_LATENCY_MS"], "280")
        self.assertEqual(env["U5_FIXED_HOST_CLOCK"], "12:00:00.00")
        self.assertEqual(env["OTHER"], "kept")
        self.assertEqual(len(notes), 2)
        self.assertEqual(base["U5_EMULATOR_LOAD_LATENCY_MS"], "100", "the base is not mutated")

    def test_fixed_clock_header_alone_is_unchanged(self) -> None:
        path = self.scenario("clock-only", "# fixed-clock: 12:00:00.00\n")
        env, notes = paired_suite.scenario_environment(path, {})
        self.assertEqual(env, {"U5_FIXED_HOST_CLOCK": "12:00:00.00"})
        self.assertEqual(notes, ["clock clock-only\tfixed at 12:00:00.00 by its header"])


if __name__ == "__main__":
    unittest.main()

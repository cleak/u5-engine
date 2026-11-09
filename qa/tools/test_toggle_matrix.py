#!/usr/bin/env python3
"""Unit test for `toggle_matrix.py` (Phase B design 1.3.12, plan M0-19).

Run with `python3 -m unittest qa/tools/test_toggle_matrix.py` from the
repository root. The reports are synthetic; nothing is launched.
"""

import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import toggle_matrix  # noqa: E402

HEADER = (
    "# Ultima V enhancement equivalence report\n"
    "# configuration classic: no toggle on\n"
    "# case\tconfiguration\tresult\tsteps\tsaved-files\tfinal-digest\tdetail\n"
)


def line(case: str, configuration: str, result: str = "equal", detail: str = "") -> str:
    return f"{case}\t{configuration}\t{result}\t3\t6\tstate 0\t{detail}\n"


class ToggleMatrix(unittest.TestCase):
    def test_row_names_skip_comments_and_blanks(self) -> None:
        text = "# name\tsettings\n\nalone:a\ta=true\npairwise:01\t\n  # note\nall\tx=1\n"
        self.assertEqual(
            toggle_matrix.matrix_row_names(text), ["alone:a", "pairwise:01", "all"]
        )

    def test_the_committed_matrix_has_rows(self) -> None:
        names = toggle_matrix.matrix_row_names(toggle_matrix.DEFAULT_MATRIX.read_text())
        self.assertIn("all-on-except-cheats", names)
        self.assertNotIn("classic", names)

    def test_an_all_equal_report_passes(self) -> None:
        report = HEADER + line("a", "classic") + line("a", "row") + line("b", "classic") + line(
            "b", "row", detail="as classic"
        )
        summary = toggle_matrix.summarize_report(report)
        self.assertEqual(summary.cases, {"a", "b"})
        self.assertEqual(summary.configurations, ["classic", "row"])
        self.assertEqual(summary.results, {"equal": 4})
        problems = toggle_matrix.check(summary, ["row"], "abc", "abc", 10, 600, 0)
        self.assertEqual(problems, [])

    def test_every_failure_is_listed(self) -> None:
        report = HEADER + line("a", "classic") + line("a", "row", "DIFFER", "final digest: x")
        summary = toggle_matrix.summarize_report(report)
        problems = toggle_matrix.check(summary, ["row", "gone"], "abc", "def", 700, 600, 1)
        joined = "\n".join(problems)
        self.assertIn("exited with 1", joined)
        self.assertIn("configuration gone is missing", joined)
        self.assertIn("a row: DIFFER final digest: x", joined)
        self.assertIn("digest abc is not def", joined)
        self.assertIn("over the 600 s budget", joined)

    def test_an_empty_report_fails(self) -> None:
        problems = toggle_matrix.check(
            toggle_matrix.summarize_report(HEADER), [], None, None, 1, 600, 0
        )
        self.assertTrue(any("no case" in p for p in problems))
        self.assertTrue(any("classic is missing" in p for p in problems))

    def test_the_manifest_digest_is_the_first_16_hex_of_its_sha256(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "m.tsv"
            path.write_bytes(b"abc")
            self.assertEqual(toggle_matrix.manifest_digest(path), "ba7816bf8f01cfea")
            self.assertIsNone(toggle_matrix.manifest_digest(pathlib.Path(directory) / "none"))

    def test_the_budget_is_ten_minutes(self) -> None:
        self.assertEqual(toggle_matrix.BUDGET_SECONDS, 600)
        self.assertEqual(toggle_matrix.ROUTE_DIGEST_1_0_1, "22254c83f2af767b")


if __name__ == "__main__":
    unittest.main()

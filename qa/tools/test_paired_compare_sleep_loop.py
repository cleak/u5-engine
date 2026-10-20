#!/usr/bin/env python3
"""Unit test for `paired_compare.py`'s sleep-loop beat declarations.

Run with `python3 -m unittest qa/tools/test_paired_compare_sleep_loop.py`
from the repository root (or `python3` on this file). The rows below are
placeholders, not game text: `Z` stands for any repeated pass row and `P` for
any row printed after the loop ends.
"""

import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import paired_compare as pc  # noqa: E402


class SleepLoopDeclarationTest(unittest.TestCase):
    def test_the_asleep_headers_declare_their_beats(self) -> None:
        self.assertIn("idle1", pc.sleep_phase_beats("asleep-dungeon"))
        self.assertIn("pass12", pc.sleep_phase_beats("asleep-dungeon"))
        self.assertIn("long4", pc.sleep_phase_beats("asleep-dungeon-clock"))
        self.assertIn("idle9", pc.sleep_count_beats("asleep-town"))
        # Only a header that says so is declared, and neither declaration
        # turns its scenario into a whole-scenario lottery, which would
        # excuse the roster panel too.
        self.assertEqual(pc.sleep_phase_beats("asleep-town"), frozenset())
        self.assertEqual(pc.sleep_count_beats("sail-calm"), frozenset())
        for scenario in ("asleep-dungeon", "asleep-dungeon-clock", "asleep-town"):
            self.assertFalse(pc.is_lottery(scenario), scenario)

    def test_the_pass_count_folds_but_the_rows_after_it_do_not(self) -> None:
        three = ["", "", "", "", "Z", "", "Z", "", "Z", "", "P", "P", "P"]
        five = ["Z", "", "Z", "", "Z", "", "Z", "", "Z", "", "P", "P", "P"]
        self.assertEqual(
            pc.collapse_repeated_units(three), pc.collapse_repeated_units(five)
        )
        # One more echo after the loop is a real difference.
        four_echoes = ["Z", "", "Z", "", "Z", "", "P", "P", "P", "P"]
        self.assertNotEqual(
            pc.collapse_repeated_units(four_echoes),
            pc.collapse_repeated_units(five),
        )

    def test_a_pass_without_its_blank_row_does_not_fold(self) -> None:
        stock = ["Z", "", "Z", "", "Z", "", "Z", ""]
        no_blank = ["Z", "Z", "Z", "Z"]
        self.assertNotEqual(
            pc.collapse_repeated_units(stock), pc.collapse_repeated_units(no_blank)
        )

    def test_an_unfinished_bottom_unit_folds_into_the_one_above(self) -> None:
        self.assertEqual(pc.collapse_repeated_units(["Z", "", "Z"]), ["Z"])
        self.assertEqual(pc.collapse_repeated_units(["", "Z", "", "Z", ""]), ["Z"])


if __name__ == "__main__":
    unittest.main()

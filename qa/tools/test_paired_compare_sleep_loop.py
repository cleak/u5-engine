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

    @staticmethod
    def _frame(*rows: str) -> list[list[str]]:
        """Decoded rows: a space is the empty cell, `?` an unmatched one."""
        return [
            ["<00>" if ch == " " else ch for ch in row.ljust(pc.COLS)]
            for row in rows
        ]

    def test_a_stock_frame_caught_mid_scroll_is_torn(self) -> None:
        engine = self._frame("", "?Z", "", "?Z", "", "?Z", "", "?Z", "")
        # Rows above the tear still at the old position, a half-drawn row,
        # rows below already moved: `idle1` of 20261023-020457.
        half_row = self._frame("", "?Z", "", "?Z", "?Z", "???", "", "?Z", "")
        self.assertTrue(pc.torn_sleep_frame(half_row, engine))
        # Two clean rows adjacent: `idle9` of 20261022-194158.
        adjacent = self._frame("", "?Z", "", "?Z", "?Z", "", "?Z", "", "")
        self.assertTrue(pc.torn_sleep_frame(adjacent, engine))
        # Only the moving row's marker cell drawn: rc4's `idle7`.
        marker = self._frame("?Z", "", "?Z", "?", "", "?Z", "", "?Z", "")
        self.assertTrue(pc.torn_sleep_frame(marker, engine))

    def test_a_settled_or_woken_stock_frame_is_not_torn(self) -> None:
        engine = self._frame("", "?Z", "", "?Z", "", "?Z", "", "?Z", "")
        # A settled frame one row out of phase is the declared phase, not a
        # tear.
        settled = self._frame("?Z", "", "?Z", "", "?Z", "", "?Z", "", "")
        self.assertFalse(pc.torn_sleep_frame(settled, engine))
        # A wake echo under a pass row decodes cleanly and is new text.
        woken = self._frame("", "?Z", "", "?Z", "", "?Z", "?P", "", "")
        self.assertFalse(pc.torn_sleep_frame(woken, engine))
        # Two half-drawn rows are more than one scroll in flight.
        two_torn = self._frame("", "?Z", "???", "?Z", "?Z", "???", "", "", "")
        self.assertFalse(pc.torn_sleep_frame(two_torn, engine))
        # Against an engine frame with adjacent rows nothing is excused: it is
        # not a clean loop frame.
        engine_adjacent = self._frame("", "?Z", "?Z", "", "?Z", "", "", "", "")
        self.assertFalse(pc.torn_sleep_frame(engine_adjacent, engine_adjacent))

    def test_an_unfinished_bottom_unit_folds_into_the_one_above(self) -> None:
        self.assertEqual(pc.collapse_repeated_units(["Z", "", "Z"]), ["Z"])
        self.assertEqual(pc.collapse_repeated_units(["", "Z", "", "Z", ""]), ["Z"])


if __name__ == "__main__":
    unittest.main()

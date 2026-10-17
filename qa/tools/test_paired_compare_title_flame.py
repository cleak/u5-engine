#!/usr/bin/env python3
"""Unit test for `paired_compare.py`'s title-screen flame-band mask.

Run with `python3 -m unittest qa/tools/test_paired_compare_title_flame.py`
from the repository root (or `python3` on this file). It needs only the
comparator module, which reads the two font files at import.
"""

import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import paired_compare as pc  # noqa: E402


def rect(rows: int, fill: str) -> list[list[str]]:
    return [[fill] * pc.COLS for _ in range(rows)]


class TitleFlameMaskTest(unittest.TestCase):
    def test_the_chargen_header_declares_the_four_title_beats(self) -> None:
        self.assertEqual(
            pc.title_flame_beats("chargen-journey-basics"),
            {"menu", "chargen-name", "chargen-gender", "menu-after-chargen"},
        )
        # Only a header that says so is masked.
        self.assertEqual(pc.title_flame_beats("sail-calm"), frozenset())
        self.assertEqual(pc.title_flame_beats("no-such-scenario"), frozenset())

    def test_only_the_flame_rows_of_the_message_window_are_masked(self) -> None:
        stock, engine = rect(pc.ROWS, "a"), rect(pc.ROWS, "b")
        left, right = pc.mask_title_flame(stock, engine, pc.TOP)
        differing = [
            pc.TOP + index
            for index, (a, b) in enumerate(zip(left, right))
            if pc.row_text(a) != pc.row_text(b)
        ]
        # Absolute rows 13 and 14 agree after the mask; 11-12 and 15-23 -
        # the box border and prompt rows a desynchronised run differs on -
        # still differ.
        self.assertEqual(differing, [11, 12] + list(range(15, 24)))
        # The inputs are not modified in place.
        self.assertTrue(all(cell == "a" for row in stock for cell in row))

    def test_only_the_flame_rows_of_the_panel_are_masked(self) -> None:
        stock, engine = rect(pc.PANEL_ROWS, "a"), rect(pc.PANEL_ROWS, "b")
        left, right = pc.mask_title_flame(stock, engine, pc.PANEL_TOP)
        differing = [
            pc.PANEL_TOP + index
            for index, (a, b) in enumerate(zip(left, right))
            if pc.row_text(a) != pc.row_text(b)
        ]
        self.assertEqual(differing, list(range(0, 9)))


if __name__ == "__main__":
    unittest.main()

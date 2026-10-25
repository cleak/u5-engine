#!/usr/bin/env python3
"""Unit test for `paired_compare.py`'s screen-chrome comparison.

Run with `python3 -m unittest qa/tools/test_paired_compare_chrome.py` from the
repository root (or `python3` on this file). The frames are synthetic: a
320x200 border drawn from the published geometry (`display-driver.md §7`),
with labels set in `IBM.CH` glyphs, written out at both capture shapes the
comparator reads - DOSBox's 640x400 and the engine's letterboxed 1280x720.
No game asset other than the font is read, and no capture is written outside
a temporary directory.
"""

import pathlib
import sys
import tempfile
import unittest

from PIL import Image

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import paired_compare as pc  # noqa: E402

RIBBON = pc.CHROME_RIBBON
WHITE = (255, 255, 255)
YELLOW = (255, 255, 85)
BLACK = (0, 0, 0)

# The ribbon bands' filled extents, inclusive, in frame pixels.
RIBBON_BANDS = [
    (0, 0, 319, 6),
    (0, 185, 191, 191),
    (0, 0, 6, 191),
    (185, 0, 191, 191),
    (313, 0, 319, 87),
    (192, 80, 312, 87),
    (192, 57, 312, 63),
]


class Frame:
    """A 320x200 gameplay frame with the border and nothing in its gaps."""

    def __init__(self) -> None:
        self.image = Image.new("RGB", (320, 200), BLACK)
        for x0, y0, x1, y1 in RIBBON_BANDS:
            self.fill(x0, y0, x1, y1, RIBBON)

    def fill(self, x0: int, y0: int, x1: int, y1: int, colour: tuple) -> None:
        pixels = self.image.load()
        for y in range(y0, y1 + 1):
            for x in range(x0, x1 + 1):
                pixels[x, y] = colour

    def gap(self, row: int, first_cap: int, last_cap: int) -> "Frame":
        """Black out a ribbon gap from cap column to cap column."""
        self.fill(first_cap * 8, row * 8, last_cap * 8 + 7, row * 8 + 7, BLACK)
        return self

    def text(self, row: int, col: int, text: str, colour: tuple = WHITE) -> "Frame":
        pixels = self.image.load()
        for offset, char in enumerate(text):
            glyph = pc.TABLES["ibm"][ord(char)]
            for j in range(8):
                for i in range(8):
                    if glyph[j][i]:
                        pixels[(col + offset) * 8 + i, row * 8 + j] = colour
        return self

    def icon(self, col: int) -> "Frame":
        """A lit sky-strip cell: a plain blob, standing in for a sun/moon."""
        self.fill(col * 8 + 2, 1, col * 8 + 5, 6, YELLOW)
        return self

    # The gameplay shapes the census met.

    def sky(self, *icons: int) -> "Frame":
        self.gap(0, 5, 18)
        for col in icons:
            self.icon(col)
        return self

    def wind(self, word: str) -> "Frame":
        return self.gap(23, 6, 18).text(23, 7, f"{word:<5} Winds")

    def level(self, digit: int) -> "Frame":
        return self.gap(0, 10, 13).text(0, 11, f"L{digit}")

    def facing(self, word: str) -> "Frame":
        return self.gap(23, 6, 17).text(23, 7, f"Dir: {word:>5}")


class ChromeComparisonTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.root = pathlib.Path(self.directory.name)
        self.count = 0

    def tearDown(self) -> None:
        self.directory.cleanup()

    def capture(self, frame: Frame, shape: str = "dosbox") -> pc.FrameSampler:
        self.count += 1
        path = self.root / f"{shape}-{self.count}.png"
        if shape == "dosbox":
            frame.image.resize((640, 400), Image.NEAREST).save(path)
        else:
            # The engine shape: the frame at 4:3, centred in a 16:9 capture.
            canvas = Image.new("RGB", (1280, 720), (12, 12, 12))
            canvas.paste(frame.image.resize((960, 720), Image.NEAREST), (160, 0))
            canvas.save(path)
        sampler = pc.FrameSampler.open(path)
        self.assertIsNotNone(sampler)
        return sampler

    def differences(self, stock: Frame, engine: Frame, **options) -> dict:
        return pc.chrome_differences(
            self.capture(stock), self.capture(engine, "engine"), **options
        )

    def test_the_same_chrome_agrees_across_both_capture_shapes(self) -> None:
        found = self.differences(
            Frame().sky(9, 14).wind("North"), Frame().sky(9, 14).wind("North")
        )
        self.assertEqual(found["readings"], ["sky[2]", "sky[2]", "wind", "wind"])
        self.assertIsNone(pc.describe_chrome(found))

    def test_a_blank_strip_against_a_present_one_differs(self) -> None:
        # The rc12 combat-arena regression: the stock keeps its sky strip and
        # wind banner, and the engine draws the ribbon straight through.
        found = self.differences(Frame().sky(9).wind("East"), Frame())
        self.assertEqual(found["readings"], ["sky[1]", "ribbon", "wind", "ribbon"])
        self.assertTrue(found["top"])
        self.assertTrue(found["bottom"])
        self.assertIn("top sky[1]/ribbon", pc.describe_chrome(found))
        # A strip blacked out entirely differs too.
        blank = Frame().sky(9).wind("East")
        blank.fill(0, 184, 191, 191, BLACK)
        found = self.differences(Frame().sky(9).wind("East"), blank)
        self.assertEqual(found["readings"][2:], ["wind", "blank"])
        self.assertTrue(found["bottom"])

    def test_the_wind_mask_never_hides_a_missing_banner(self) -> None:
        # Only the direction word is masked, and only when both sides show
        # the banner: an empty gap against a banner still differs inside the
        # word's own columns.
        found = self.differences(Frame().sky(9).wind("North"), Frame().sky(9).gap(23, 6, 18))
        self.assertEqual(found["readings"][2:], ["wind", "other"])
        self.assertTrue(set(pc.WIND_WORD_COLS) & set(found["bottom"]))

    def test_a_different_wind_word_agrees(self) -> None:
        found = self.differences(
            Frame().sky(9).wind("North"), Frame().sky(9).wind("West")
        )
        self.assertEqual(found["readings"][2:], ["wind", "wind"])
        self.assertIsNone(pc.describe_chrome(found))
        found = self.differences(Frame().sky(9).wind("Calm"), Frame().sky(9).wind("South"))
        self.assertIsNone(pc.describe_chrome(found))

    def test_the_dungeon_level_label_is_compared(self) -> None:
        found = self.differences(
            Frame().level(1).facing("North"), Frame().level(2).facing("North")
        )
        self.assertEqual(found["readings"], ["L1", "L2", "facing", "facing"])
        self.assertEqual(found["top"], [12])
        self.assertFalse(found["bottom"])
        found = self.differences(
            Frame().level(3).facing("North"), Frame().level(3).facing("North")
        )
        self.assertIsNone(pc.describe_chrome(found))

    def test_the_dungeon_facing_is_not_masked(self) -> None:
        # Unlike the wind, the facing is the party's own state.
        found = self.differences(
            Frame().level(4).facing("North"), Frame().level(4).facing("West")
        )
        self.assertTrue(found["bottom"])

    def test_sky_icon_positions_differ_unless_declared_random(self) -> None:
        stock, engine = Frame().sky(9).wind("East"), Frame().sky(12).wind("East")
        self.assertEqual(self.differences(stock, engine)["top"], [9, 12])
        found = self.differences(stock, engine, random_sky=True)
        self.assertIsNone(pc.describe_chrome(found))
        # The mask is never the whole strip: a strip with nothing lit
        # against one with an icon still differs, and so does the ribbon.
        found = self.differences(stock, Frame().sky().wind("East"), random_sky=True)
        self.assertIn("icons", found["top"])
        found = self.differences(stock, Frame().wind("East"), random_sky=True)
        self.assertTrue(found["top"])

    def test_a_frame_cell_difference_is_reported(self) -> None:
        engine = Frame().sky(9).wind("East")
        engine.fill(313, 0, 319, 87, BLACK)
        found = self.differences(Frame().sky(9).wind("East"), engine)
        self.assertTrue(found["frame"])
        self.assertIn("frame", pc.describe_chrome(found))

    def test_screens_without_the_border_are_not_compared(self) -> None:
        # Title and character-creation art: no border on either side.
        stock, engine = Frame(), Frame()
        stock.fill(0, 0, 319, 199, (85, 85, 255))
        engine.fill(0, 0, 319, 199, (255, 85, 85))
        self.assertIsNone(pc.describe_chrome(self.differences(stock, engine)))
        # But a border on one side only is compared.
        self.assertIsNotNone(pc.describe_chrome(self.differences(Frame(), engine)))

    def test_random_sky_beats_are_read_from_the_header(self) -> None:
        self.assertEqual(pc.random_sky_beats("no-such-scenario"), frozenset())
        self.assertEqual(pc.random_sky_beats("sail-calm"), frozenset())


if __name__ == "__main__":
    unittest.main()

#!/usr/bin/env python3
"""Which published game lines does the paired suite ever actually see?

`spec_literal_coverage.py` asks whether the *engine* contains each published
literal. This asks the harder question: whether any *scenario* has ever put it
on screen. A line the engine implements perfectly and no scenario exercises is
unverified, and until now nothing counted them.

It reads the stock side of every capture, so the answer is about the original's
behaviour rather than this engine's - a literal the original never printed in
any run is one the suite does not reach, whatever the engine does with it.

Wrapping is undone before matching: the message window is sixteen columns, so
almost every published line is split across rows in the capture.

What "reached" means precisely: the line was visible in a *sampled* stock
frame. That is the right bar rather than a strict one, because the comparator
only ever compares sampled frames - a line printed between two beats and
scrolled away before the next shot is never checked against the original by
anything. So an unreached line is an unverified line.

This is still a coverage reading rather than a defect list. A line can be
unreached because no scenario goes there, because its branch needs a game
state none of the seeds carry, because the shot lands after it scrolled, or
because it is prose the extractor mistook for a game line.

Usage: scenario_literal_coverage.py <SPEC_DIR> <artifact-dir>...
"""

import pathlib
import re
import sys

from paired_compare import COLS, LEFT, decode, decode_panel, decode_region
from spec_literal_coverage import candidates

WHITESPACE = re.compile(r"\s+")
# The glyph matcher in `paired_compare.decode` cannot separate this font's
# letter O from its zero, so captures read `A TRAPD00R!` and `:H0NESTY`.
# Folded on both sides. Only that pair - widening the fold to digits that
# merely resemble letters invents matches.
CONFUSABLE = str.maketrans({"0": "o"})


def normalise(text: str) -> str:
    folded = WHITESPACE.sub(" ", text).strip().lower()
    return folded.translate(CONFUSABLE)


def captured_text(artifact: pathlib.Path) -> str:
    """Every stock message-window and roster-panel row in one artifact.

    The panel was missing, and it carries published text of its own: the
    `F:`/`G:` provisions row and the character rows the roster draws. Counting
    only the message window reported those as unreached while every capture in
    the suite had them on screen.

    Still outside the count: the viewport, and the full-screen surfaces drawn
    into it - the Z-stats pages in particular, whose `STR:`, `DEX:`, `INT:`,
    `HP:`, `Exp:`, `Level:`, `Class:` and `Sex:` labels are therefore reported
    unreached whatever any scenario does. That region is tiles most of the
    time, and a tile field decodes to noise that would match a two-character
    literal by accident, so widening to it needs a texty-row test rather than
    a wider rectangle. Read the figure as "reached in the two text regions the
    comparator compares", which is also the bar for a line being *verified*.
    """
    pieces = []
    for shot in sorted(artifact.glob("dosbox-*.png")):
        for rows in (decode(shot), decode_panel(shot)):
            if rows is None:
                continue
            for row in rows:
                pieces.append("".join(cell if len(cell) == 1 else " " for cell in row))
    # Rows are joined with a space, so a line wrapped across two rows
    # reassembles with single spacing.
    return normalise(" ".join(pieces))


def captured_fullscreen_text(artifact: pathlib.Path) -> str:
    """The whole 40x25 character grid of every stock frame in one artifact.

    Some published lines are never drawn in the two text regions above.
    Character creation is the clearest case: `chargen.md`'s name and sex
    prompts are painted across the whole screen over the title art, and the
    message-window rectangle clips `By what name shalt thou be known?` down
    to the fragment `ou be known`, which matches nothing. The Z-stats pages
    are the same shape.

    The catch is that most of this grid is tiles most of the time, and the
    glyph matcher maps every cell to its best-scoring glyph rather than
    rejecting it, so a tile field decodes to junk letters. Junk will match a
    two-character literal like `G:` by accident and say nothing true.

    So this corpus is only consulted for **long** literals, where an
    accidental match is not a realistic worry. The threshold is deliberately
    generous rather than tuned.
    """
    pieces = []
    for shot in sorted(artifact.glob("dosbox-*.png")):
        rows = decode_region(shot, 0, 0, 40, 25)
        if rows is None:
            continue
        for row in rows:
            pieces.append("".join(cell if len(cell) == 1 else " " for cell in row))
    return normalise(" ".join(pieces))


# Shortest literal the full-screen corpus is allowed to answer for. Below
# this, only the message window and roster panel count.
FULLSCREEN_MIN_LEN = 14


def main() -> int:
    spec_dir = pathlib.Path(sys.argv[1])
    published: set[str] = set()
    for path in spec_dir.rglob("*.md"):
        published |= candidates(path.read_text(errors="replace"))

    seen_text = []
    seen_fullscreen = []
    for arg in sys.argv[2:]:
        artifact = pathlib.Path(arg)
        seen_text.append(captured_text(artifact))
        seen_fullscreen.append(captured_fullscreen_text(artifact))
    haystack = " ".join(seen_text)
    fullscreen = " ".join(seen_fullscreen)

    reached = {lit for lit in published if normalise(lit) in haystack}
    # The full-screen corpus answers only for long literals; see
    # `captured_fullscreen_text`.
    elsewhere = {
        lit
        for lit in published - reached
        if len(normalise(lit)) >= FULLSCREEN_MIN_LEN and normalise(lit) in fullscreen
    }
    missing = sorted(published - reached - elsewhere)

    print(
        f"{len(reached)} of {len(published)} published lines reached in the "
        f"message window or roster panel"
    )
    if elsewhere:
        print(
            f"{len(elsewhere)} more reached only on a full-screen surface "
            f"(chargen, Z-stats and the like)"
        )
    print(f"{len(missing)} never appear in any stock capture\n")
    for literal in sorted(elsewhere):
        print(f"  full-screen  {literal}")
    if elsewhere:
        print()
    for literal in missing:
        print(f"    {literal}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

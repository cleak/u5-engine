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

from paired_compare import decode
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
    """Every stock message-window row in one artifact, joined."""
    pieces = []
    for shot in sorted(artifact.glob("dosbox-*.png")):
        rows = decode(shot)
        if rows is None:
            continue
        for row in rows:
            pieces.append("".join(cell if len(cell) == 1 else " " for cell in row))
    # Rows are joined with a space, so a line wrapped across two rows
    # reassembles with single spacing.
    return normalise(" ".join(pieces))


def main() -> int:
    spec_dir = pathlib.Path(sys.argv[1])
    published: set[str] = set()
    for path in spec_dir.rglob("*.md"):
        published |= candidates(path.read_text(errors="replace"))

    seen_text = []
    for arg in sys.argv[2:]:
        seen_text.append(captured_text(pathlib.Path(arg)))
    haystack = " ".join(seen_text)

    reached = {lit for lit in published if normalise(lit) in haystack}
    missing = sorted(published - reached)

    print(f"{len(reached)} of {len(published)} published lines reached by the suite")
    print(f"{len(missing)} never appear in any stock capture\n")
    for literal in missing:
        print(f"    {literal}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

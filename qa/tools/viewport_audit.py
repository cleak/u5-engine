#!/usr/bin/env python3
"""Compare the *map viewport* of paired captures, which nothing else does.

`qa/tools/paired_compare.py` decodes two text regions - the message window and
the roster panel - and every "match" this suite has ever reported means those
two agreed. The map viewport, which is the largest thing on screen, has never
been compared at all.

That gap hides whole presentations. `karma.md` Section 7 has shrine meditation
suspend the active-object table, load the shrine's own eleven-by-eleven grid
from `MISCMAPS.DAT` record 1, walk an avatar up it over nine animation frames
and kneel; this engine leaves the overworld on screen throughout, and
`shrine-three-mantras` still reported a clean match because the text agreed.

Each viewport tile cell is reduced to a colour histogram, and a cell counts
as different when its histograms are far enough apart. That is blind to
animation phase - open water rearranges about half the viewport's pixels
between frames while keeping the same colours - and still catches a
different picture.

Usage: viewport_audit.py <artifact-dir>...
"""

import collections
import pathlib
import sys

from PIL import Image

from paired_compare import capture_is_frame_filling, frame_rect

VIEWPORT_LEFT, VIEWPORT_TOP = 8, 8
VIEWPORT_SIZE = 176
CELL = 16
CELLS = VIEWPORT_SIZE // CELL
# Per-cell colour-histogram distance above which that cell is called
# different, as a fraction of the cell's pixels.
CELL_DISTANCE = 0.35
# Cells that must differ before the beat is called different. One moving
# sprite occupies one or two.
CELL_TOLERANCE = 3


def viewport_cell_histograms(path: pathlib.Path) -> list[dict] | None:
    """One colour histogram per viewport tile cell.

    Comparing pixels directly does not work here: animated terrain is the
    same picture at a different phase, and the wave tiles of open water move
    about half the viewport's pixels between frames. A histogram is blind to
    that rearrangement - animated water keeps the same two colours in the
    same proportions - while a genuinely different picture, such as the
    overworld where the shrine's own grid belongs, does not.
    """
    try:
        image = Image.open(path).convert("RGB")
    except OSError:
        return None
    # Same guard as the text decoders: a capture that is not the frame
    # samples every cell from the wrong pixels, and the viewport's version of
    # that is a large constant difference on every beat.
    if not capture_is_frame_filling(image):
        return None
    origin_x, origin_y, scale_x, scale_y = frame_rect(image.size)
    box = (
        int(origin_x + VIEWPORT_LEFT * scale_x),
        int(origin_y + VIEWPORT_TOP * scale_y),
        int(origin_x + (VIEWPORT_LEFT + VIEWPORT_SIZE) * scale_x),
        int(origin_y + (VIEWPORT_TOP + VIEWPORT_SIZE) * scale_y),
    )
    patch = image.crop(box).resize((VIEWPORT_SIZE, VIEWPORT_SIZE), Image.NEAREST)
    pixels = patch.load()
    histograms = []
    for row in range(CELLS):
        for col in range(CELLS):
            counts: collections.Counter = collections.Counter()
            for y in range(row * CELL, (row + 1) * CELL):
                for x in range(col * CELL, (col + 1) * CELL):
                    counts[pixels[x, y]] += 1
            histograms.append(counts)
    return histograms


def histogram_distance(left: dict, right: dict) -> float:
    total = CELL * CELL
    keys = set(left) | set(right)
    return sum(abs(left.get(k, 0) - right.get(k, 0)) for k in keys) / (2 * total)


def compare_beat(stock: pathlib.Path, engine: pathlib.Path) -> int | None:
    left = viewport_cell_histograms(stock)
    right = viewport_cell_histograms(engine)
    if left is None or right is None:
        return None
    return sum(
        1 for a, b in zip(left, right) if histogram_distance(a, b) > CELL_DISTANCE
    )


def main() -> int:
    total_beats = total_differ = 0
    for arg in sys.argv[1:]:
        artifact = pathlib.Path(arg)
        beats = sorted(
            p.name[len("dosbox-") : -len(".png")]
            for p in artifact.glob("dosbox-*.png")
        )
        worst = []
        for beat in beats:
            differing = compare_beat(
                artifact / f"dosbox-{beat}.png", artifact / f"engine-{beat}.png"
            )
            if differing is None:
                continue
            total_beats += 1
            if differing > CELL_TOLERANCE:
                total_differ += 1
                worst.append((differing, beat))
        worst.sort(reverse=True)
        flag = "VIEWPORT" if worst else "ok      "
        detail = " ".join(f"{beat}:{n}" for n, beat in worst[:4])
        print(f"{flag} {artifact.name}  {detail}")
    print(f"\n{total_beats} beat(s) compared, {total_differ} differ in the viewport")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

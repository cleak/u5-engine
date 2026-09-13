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

The comparison is a pixel fraction over the normalised viewport, with a
tolerance that absorbs sprite animation phase and torch flicker while still
catching "this is a different picture".

Usage: viewport_audit.py <artifact-dir>...
"""

import collections
import pathlib
import sys

from PIL import Image

from paired_compare import frame_rect

VIEWPORT_LEFT, VIEWPORT_TOP = 8, 8
VIEWPORT_SIZE = 176
# A beat differs when this fraction of the viewport's pixels do. Sprite
# animation phase, a torch flicker or a one-cell sprite offset move a few per
# cent; a different picture moves tens.
PIXEL_TOLERANCE = 0.08


def viewport_pixels(path: pathlib.Path) -> list[tuple[int, int, int]] | None:
    """The viewport, normalised to its logical 176x176 pixels.

    An earlier revision reduced each of the eleven-by-eleven tile cells to its
    most common colour. That was too crude: a cell split about evenly between
    white stone and black tie-breaks either way, so identical pictures scored
    four differing cells and the audit reported 499 differing beats where the
    real figure is a fraction of that.
    """
    try:
        image = Image.open(path).convert("RGB")
    except OSError:
        return None
    origin_x, origin_y, scale_x, scale_y = frame_rect(image.size)
    box = (
        int(origin_x + VIEWPORT_LEFT * scale_x),
        int(origin_y + VIEWPORT_TOP * scale_y),
        int(origin_x + (VIEWPORT_LEFT + VIEWPORT_SIZE) * scale_x),
        int(origin_y + (VIEWPORT_TOP + VIEWPORT_SIZE) * scale_y),
    )
    patch = image.crop(box).resize((VIEWPORT_SIZE, VIEWPORT_SIZE), Image.NEAREST)
    return list(patch.getdata())


def compare_beat(stock: pathlib.Path, engine: pathlib.Path) -> float | None:
    left, right = viewport_pixels(stock), viewport_pixels(engine)
    if left is None or right is None:
        return None
    differing = sum(1 for a, b in zip(left, right) if a != b)
    return differing / len(left)


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
            if differing > PIXEL_TOLERANCE:
                total_differ += 1
                worst.append((differing, beat))
        worst.sort(reverse=True)
        flag = "VIEWPORT" if worst else "ok      "
        detail = " ".join(f"{beat}:{n:.0%}" for n, beat in worst[:4])
        print(f"{flag} {artifact.name}  {detail}")
    print(f"\n{total_beats} beat(s) compared, {total_differ} differ in the viewport")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

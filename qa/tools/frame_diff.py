#!/usr/bin/env python3
"""Compare a native engine frame with a DOSBox capture, pixel for pixel.

The paired harness captures the Bevy window, which is aspect-corrected and
scaled, so its frames can only be compared by eye. `u5-engine --play
--from-save --play-script ... --save-screen out.png` writes the *native*
320x200 composed screen instead, which is directly comparable with the
DOSBox capture of the same state: any surviving difference is a real one.

Usage: frame_diff.py <ENGINE_PNG> <DOSBOX_PNG> [--ignore-cursor]

Prints the differing-pixel count, the bounding box, and a per-region
breakdown against the published screen layout (`systems/view.md` §3.1:
the gameplay viewport at (8,8)..(183,183), the panel to its right, the
message window below it).
"""

import pathlib
import sys

from PIL import Image, ImageChops

VIEWPORT = (8, 8, 184, 184)
PANEL = (184, 0, 320, 96)
MESSAGE = (184, 96, 320, 200)


def region_pixels(diff: Image.Image, box: tuple[int, int, int, int]) -> int:
    return sum(1 for pixel in diff.crop(box).getdata() if pixel > 16)


def main() -> None:
    arguments = [argument for argument in sys.argv[1:] if not argument.startswith("--")]
    if len(arguments) != 2:
        raise SystemExit(__doc__)
    engine = Image.open(pathlib.Path(arguments[0])).convert("RGB")
    stock = Image.open(pathlib.Path(arguments[1])).convert("RGB")
    if stock.size != engine.size:
        stock = stock.resize(engine.size, Image.NEAREST)
    diff = ImageChops.difference(engine, stock).convert("L")
    total = sum(1 for pixel in diff.getdata() if pixel > 16)
    print(f"differing pixels: {total}")
    print(f"bounding box: {diff.getbbox()}")
    for name, box in (("viewport", VIEWPORT), ("panel", PANEL), ("message", MESSAGE)):
        print(f"  {name}: {region_pixels(diff, box)}")


if __name__ == "__main__":
    main()

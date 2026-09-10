#!/usr/bin/env python3
"""Stitch a paired run's beats into a side-by-side video.

The suite already writes one PNG per beat per side. Read as a pair they show
whether the engine matches; read in sequence they show the run. This builds the
second view: the original on the left, the engine on the right, one titled
frame per beat, as an MP4.

The video is a progress artifact, not a comparison oracle - `paired_compare.py`
remains the thing that decides agreement, because it reads the message window's
cells rather than pixels.

It stays under `~/artifacts`, which is not in Git: these frames are the
original game's output.

Usage: paired_video.py <SCENARIO> [--seconds-per-beat 2.0]
"""

import argparse
import json
import pathlib
import subprocess
import sys

from PIL import Image, ImageDraw

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import paired_compare  # noqa: E402

LABEL_BAND = 28
GAP = 12


def build(scenario: str, seconds: float) -> pathlib.Path:
    found = paired_compare.latest_artifacts([scenario])
    if not found:
        raise SystemExit(f"no run found for scenario `{scenario}`")
    artifact = found[0]
    record = json.loads((artifact / "record.json").read_text())
    labels = [c.get("label") for c in record.get("captures", [])]

    frames = artifact / "video-frames"
    frames.mkdir(exist_ok=True)
    for stale in frames.glob("*.png"):
        stale.unlink()

    index = 0
    for label in labels:
        stock = artifact / f"dosbox-{label}.png"
        engine = artifact / f"engine-{label}.png"
        if not stock.is_file() or not engine.is_file():
            continue
        left, right = Image.open(stock).convert("RGB"), Image.open(engine).convert("RGB")
        height = max(left.height, right.height)
        width = left.width + GAP + right.width
        canvas = Image.new("RGB", (width, height + LABEL_BAND), (16, 16, 16))
        canvas.paste(left, (0, LABEL_BAND))
        canvas.paste(right, (left.width + GAP, LABEL_BAND))
        draw = ImageDraw.Draw(canvas)
        draw.text((6, 8), f"original - {scenario}/{label}", fill=(220, 220, 220))
        draw.text((left.width + GAP + 6, 8), f"engine - {scenario}/{label}", fill=(220, 220, 220))
        canvas.save(frames / f"{index:04d}.png")
        index += 1

    if index == 0:
        raise SystemExit(f"{artifact.name} has no beat with both sides captured")

    out = artifact / "paired.mp4"
    subprocess.run(
        [
            "ffmpeg", "-y", "-loglevel", "error",
            "-framerate", f"{1 / seconds:.4f}",
            "-i", str(frames / "%04d.png"),
            # Even dimensions, or H.264 refuses the stream.
            "-vf", "pad=ceil(iw/2)*2:ceil(ih/2)*2",
            "-c:v", "libx264", "-pix_fmt", "yuv420p", "-r", "30",
            str(out),
        ],
        check=True,
    )
    for frame in frames.glob("*.png"):
        frame.unlink()
    frames.rmdir()
    return out


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("scenario")
    ap.add_argument("--seconds-per-beat", type=float, default=2.0)
    args = ap.parse_args()
    print(build(args.scenario, args.seconds_per_beat))


if __name__ == "__main__":
    main()

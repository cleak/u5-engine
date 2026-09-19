#!/usr/bin/env python3
"""Measure the spacing between sound bursts in a paired recording.

`audio.md §8.7` describes the endgame tableau's introductory train - one
short two-part sting per actor movement, with five shared world-animation
ticks around each - and then declines to put a number on the interval:
its "roughly 275 ms spacing and occasional roughly 55 ms extension" are
marked "comparisons with the issue's capture, **not a new wall-clock
measurement or a universal fixed interval**".

`audio_compare.py` reports a span's total duration, which is enough to
see that a train is missing but not enough to pace one. This reports the
bursts inside a span: how many, where they start, and how far apart they
are. A short two-part sting is about 25 ms of sound with silence either
side, so a burst is a run of sounding windows bounded by quiet ones.

The window here is finer than `audio_compare`'s 100 ms, because a 25 ms
sting inside a 275 ms gap is invisible at that resolution.

Usage: audio_burst_spacing.py <artifact-dir> <span-name>...
"""

import pathlib
import sys

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
from audio_compare import load_mono

WINDOW_SECS = 0.010
SOUND_FRACTION = 0.10


def bursts(path: pathlib.Path) -> tuple[list[float], float] | None:
    loaded = load_mono(path)
    if loaded is None:
        return None
    samples, rate = loaded
    size = max(1, int(rate * WINDOW_SECS))
    rms = []
    for start in range(0, len(samples) - size + 1, size):
        chunk = samples[start : start + size]
        rms.append((sum(v * v for v in chunk) / size) ** 0.5)
    if not rms:
        return None
    peak = max(rms)
    if peak < 1.0:
        return [], 0.0
    threshold = peak * SOUND_FRACTION
    onsets = []
    sounding = False
    for index, value in enumerate(rms):
        if value > threshold and not sounding:
            onsets.append(index * WINDOW_SECS)
            sounding = True
        elif value <= threshold:
            sounding = False
    return onsets, len(rms) * WINDOW_SECS


def main() -> int:
    artifact = pathlib.Path(sys.argv[1])
    for span in sys.argv[2:]:
        print(f"== {artifact.name} / {span}")
        for side, prefix in (("stock ", "dosbox"), ("engine", "engine")):
            path = artifact / f"audio-{prefix}-{span}.wav"
            found = bursts(path)
            if found is None:
                print(f"   {side}: capture missing")
                continue
            onsets, total = found
            if not onsets:
                print(f"   {side}: silent over {total:.1f}s")
                continue
            gaps = [b - a for a, b in zip(onsets, onsets[1:])]
            spacing = (
                f"gaps {min(gaps) * 1000:.0f}..{max(gaps) * 1000:.0f} ms, "
                f"median {sorted(gaps)[len(gaps) // 2] * 1000:.0f} ms"
                if gaps
                else "one burst"
            )
            print(
                f"   {side}: {len(onsets)} burst(s) over {total:.1f}s, "
                f"first at {onsets[0]:.2f}s, last at {onsets[-1]:.2f}s; {spacing}"
            )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

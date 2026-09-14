#!/usr/bin/env python3
"""Compare the paired audio captures, which nothing else does.

The harness records a `.wav` from each side for every `audio-start` /
`audio-stop` span in a scenario, and six scenarios use them. Nothing has ever
read those files: `paired_compare.py` decodes the two text regions and, since
2026-09-13, the map viewport - the recordings sit in the artifact unexamined.

What is worth comparing is not the samples, which will never agree, but the
shape `systems/audio.md` actually specifies: when sound starts and stops, how
long it lasts, and what frequency it is doing over that time. The PC speaker
emits square waves, so a zero-crossing count over a short window recovers the
tone directly, and no spectral machinery is needed.

`audio.md` Section 8.2 gives the Stonegate trapdoor sweep as "every integer
frequency from 1000 down through 251 Hz, 750 tones total", and Section 10
marks its ~26.5 s duration as **derived and unverified**. That is the sort of
question this can answer from the recordings.

Usage: audio_compare.py <artifact-dir>...
"""

import array
import pathlib
import sys
import wave

WINDOW_SECS = 0.10
# A window counts as sounding when its RMS clears this fraction of the
# capture's own peak window. The captures include a deliberate silence
# control, so an absolute threshold would need calibrating per host.
SOUND_FRACTION = 0.10
# How far a single window may rise and still count as part of a descending
# sweep. The zero-crossing estimate is noisy; the sweep itself steps by about
# one hertz per tone.
SWEEP_TOLERANCE_HZ = 60.0


def load_mono(path: pathlib.Path) -> tuple[list[int], int] | None:
    try:
        with wave.open(str(path)) as handle:
            if handle.getsampwidth() != 2:
                return None
            rate = handle.getframerate()
            channels = handle.getnchannels()
            raw = handle.readframes(handle.getnframes())
    except (OSError, wave.Error):
        return None
    samples = array.array("h")
    samples.frombytes(raw)
    if channels > 1:
        samples = samples[::channels]
    return list(samples), rate


def windows(samples: list[int], rate: int) -> list[tuple[float, int]]:
    """(rms, zero crossings) per window."""
    size = max(1, int(rate * WINDOW_SECS))
    out = []
    for start in range(0, len(samples) - size + 1, size):
        chunk = samples[start : start + size]
        total = 0
        crossings = 0
        previous = chunk[0]
        for value in chunk:
            total += value * value
            if (value >= 0) != (previous >= 0):
                crossings += 1
            previous = value
        out.append(((total / size) ** 0.5, crossings))
    return out


def describe(path: pathlib.Path) -> dict | None:
    loaded = load_mono(path)
    if loaded is None:
        return None
    samples, rate = loaded
    frames = windows(samples, rate)
    if not frames:
        return None
    peak = max(rms for rms, _ in frames)
    threshold = peak * SOUND_FRACTION
    sounding = [i for i, (rms, _) in enumerate(frames) if rms > threshold]
    if not sounding or peak < 1.0:
        return {"silent": True, "duration": 0.0, "peak": peak}
    first, last = sounding[0], sounding[-1]
    tones = [
        crossings / 2.0 / WINDOW_SECS
        for rms, crossings in frames[first : last + 1]
        if rms > threshold
    ]
    # `audio.md` Section 8.2's descending sweep is the longest run of
    # windows whose tone falls monotonically. Isolating it gives a duration
    # that can be held against the section's own figure, which Section 10
    # marks as derived and unverified.
    # Smoothed, and tolerant of the odd upward window: a zero-crossing tone
    # estimate is noisy enough that a strictly monotonic run finds only a
    # fraction of a second of a twenty-five second sweep.
    smooth = [
        sorted(tones[max(0, i - 2) : i + 3])[len(tones[max(0, i - 2) : i + 3]) // 2]
        for i in range(len(tones))
    ]
    best_start = best_len = 0
    run_start = 0
    for i in range(1, len(smooth)):
        if smooth[i] <= smooth[i - 1] + SWEEP_TOLERANCE_HZ:
            if i - run_start > best_len:
                best_start, best_len = run_start, i - run_start
        else:
            run_start = i
    tones_for_sweep = smooth
    sweep_from = tones_for_sweep[best_start] if best_len else 0.0
    sweep_to = tones_for_sweep[best_start + best_len] if best_len else 0.0
    return {
        "silent": False,
        "sweep_secs": (best_len + 1) * WINDOW_SECS,
        "sweep_from": sweep_from,
        "sweep_to": sweep_to,
        "onset": first * WINDOW_SECS,
        "duration": (last - first + 1) * WINDOW_SECS,
        "peak": peak,
        "first_tone": tones[0] if tones else 0.0,
        "last_tone": tones[-1] if tones else 0.0,
        "tones": tones,
    }


def main() -> int:
    for arg in sys.argv[1:]:
        artifact = pathlib.Path(arg)
        tags = sorted(
            p.name[len("audio-dosbox-") : -len(".wav")]
            for p in artifact.glob("audio-dosbox-*.wav")
        )
        if not tags:
            continue
        print(f"== {artifact.name}")
        for tag in tags:
            left = describe(artifact / f"audio-dosbox-{tag}.wav")
            right = describe(artifact / f"audio-engine-{tag}.wav")
            if left is None or right is None:
                print(f"   {tag}: capture missing")
                continue
            if left["silent"] and right["silent"]:
                print(f"   {tag}: both silent")
                continue
            for name, side in (("stock ", left), ("engine", right)):
                if side["silent"]:
                    print(f"   {tag} {name}: silent")
                else:
                    print(
                        f"   {tag} {name}: onset {side['onset']:5.1f}s "
                        f"duration {side['duration']:5.1f}s "
                        f"tone {side['first_tone']:6.0f} -> {side['last_tone']:5.0f} Hz"
                    )
            if not left["silent"] and not right["silent"]:
                delta = right["duration"] - left["duration"]
                print(f"   {tag} delta : duration {delta:+.1f}s")
                # Endpoints are the least informative part of a sweep - the
                # first and last sounding windows catch onset and tail. The
                # trajectory is what `audio.md` Section 8.2 actually
                # describes.
                for name, side in (("stock ", left), ("engine", right)):
                    print(
                        f"   {tag} {name}: descending sweep "
                        f"{side['sweep_secs']:5.1f}s, "
                        f"{side['sweep_from']:5.0f} -> {side['sweep_to']:4.0f} Hz"
                    )
                for name, side in (("stock ", left), ("engine", right)):
                    tones = side["tones"]
                    picks = [
                        tones[i * (len(tones) - 1) // 8] for i in range(9)
                    ] if len(tones) > 8 else tones
                    shape = " ".join(f"{t:5.0f}" for t in picks)
                    print(f"   {tag} {name}: {shape}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

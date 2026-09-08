#!/usr/bin/env python3
"""Count the production emit sites of every `SoundEffect` variant.

`systems/audio.md` §8 publishes the trigger inventory and §11 the caller
scope for each cue. A variant the engine *models* but never *emits* is a
published cue the player never hears - the audio analogue of
`engine_message_audit.py`'s unpublished-line list, read the other way.

This is a reading list, not a defect list: a variant may be emitted only
from a helper this scan attributes to that helper's file, and a few cues
are legitimately produced by one shared site.

One variant is expected to report zero and is correct at zero:
`DissolveExit`. §8.6.1 puts the dissolve's single silencing point on "the
dissolve's shared exit block", and production lowers the whole run with
`audio::dissolve_click_run`, which appends that one `Stop` itself - so the
standalone event exists only for a caller that assembles the run by hand.

Usage: audio_trigger_census.py <ENGINE_SRC_DIR>...
"""

import pathlib
import re
import sys

VARIANT = re.compile(r"^\s{4}([A-Z][A-Za-z0-9]*)\s*(\{|,)")
EMIT = re.compile(r"SoundEffect::([A-Z][A-Za-z0-9]*)")
# Files that define or render the effects rather than triggering them.
NON_TRIGGER = {"audio.rs", "audio_render.rs"}


def skipped_lines(text: str) -> set[int]:
    """Line numbers inside `#[cfg(test)]` items.

    Brace-matched from the attribute, the same way
    `engine_message_audit.py` does it: a sticky "skip the rest of the file"
    flag silently drops every emit site below the first test module, which is
    exactly the mistake this comment exists to stop.
    """
    skipped: set[int] = set()
    lines = text.splitlines()
    for index, line in enumerate(lines):
        if line.strip() not in ("#[cfg(test)]", "#[test]"):
            continue
        depth = 0
        started = False
        for number in range(index, len(lines)):
            depth += lines[number].count("{") - lines[number].count("}")
            started = started or "{" in lines[number]
            skipped.add(number + 1)
            if started and depth <= 0:
                break
    return skipped


def variants(path: pathlib.Path) -> list[str]:
    text = path.read_text()
    start = text.find("pub enum SoundEffect")
    if start < 0:
        return []
    depth = 0
    out = []
    for line in text[start:].splitlines():
        depth += line.count("{") - line.count("}")
        match = VARIANT.match(line)
        if match:
            out.append(match.group(1))
        if depth <= 0 and out:
            break
    return out


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    roots = [pathlib.Path(arg) for arg in sys.argv[1:]]
    root = roots[0]
    names = variants(root / "audio.rs")
    if not names:
        raise SystemExit("no SoundEffect enum found")
    sites: dict[str, int] = {name: 0 for name in names}
    for path in sorted(p for r in roots for p in r.rglob("*.rs")):
        if path.name in NON_TRIGGER or "tests_inline" in path.parts:
            continue
        text = path.read_text(errors="replace")
        skipped = skipped_lines(text)
        for number, line in enumerate(text.splitlines(), start=1):
            if number in skipped:
                continue
            stripped = line.strip()
            if stripped.startswith("//"):
                continue
            for name in EMIT.findall(line):
                if name in sites:
                    sites[name] += 1
    silent = [name for name, count in sites.items() if count == 0]
    for name in names:
        print(f"{sites[name]:>3}  {name}")
    print(f"\n{len(names)} variant(s), {len(silent)} with no production emit site")
    for name in silent:
        print(f"  never emitted: {name}")


if __name__ == "__main__":
    main()

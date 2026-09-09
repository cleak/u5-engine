#!/usr/bin/env python3
"""Compare the message window of both sides of a paired run, beat by beat.

`game-dev-u5-paired` records `"result": "pass"` when the *run* completed; it
does not compare the two sides. `qa/paired/README.md` said comparison had to be
done by eye because "the engine side cannot be [decoded], at any window size:
the Bevy shell presents its 320x200 frame at the 4:3 display aspect
(`DISPLAY_PIXEL_ASPECT`, 1.20), stretching the 200 rows and letterboxing the
remainder".

That is true of a naive grid, and false of the frame's own geometry. The
letterbox is deterministic: the frame is `4:3` and centred, so a capture of
height `h` holds the 320x200 frame in a `4h/3` by `h` rect at
`x = (w - 4h/3) / 2`. Sampling each glyph cell's own pixels through that map
recovers the same text `IBM.CH`/`RUNES.CH` matching reads off the DOSBox side.

So this compares them. Only the sixteen-by-thirteen message window: the
viewport carries host-clock NPC positions and the wind banner drifts on its own
roll, both of which the README documents as legitimately different.

Reads captures and prints agreement; writes nothing. The decoded text stays in
the process - it is game text, and only the verdict leaves.

Usage: paired_compare.py <ARTIFACT_DIR>...
       paired_compare.py --latest [SCENARIO]...

`--latest` compares the newest artifact directory for each scenario - every
scenario in `qa/paired` when none are named - which is what a suite pass
leaves behind. Runs that captured nothing are reported rather than skipped
silently.
"""

import json
import pathlib
import re
import sys

from PIL import Image

ASSETS = pathlib.Path("/srv/u5-clean/assets/gog-1.0-cs-28045")
FONTS = {"ibm": ASSETS / "IBM.CH", "rune": ASSETS / "RUNES.CH"}
LEFT, TOP, COLS, ROWS = 24, 11, 16, 13
# `gameplay_chrome.rs`: the four-frame barber-pole prompt cursor. It animates
# from a free-running counter on both sides, so a cell holding one is not a
# difference.
CURSOR_GLYPHS = {0x05, 0x06, 0x07, 0x08}


def glyph_table(path: pathlib.Path) -> dict[int, tuple]:
    data = path.read_bytes()
    return {
        code: tuple(
            tuple((row >> (7 - bit)) & 1 for bit in range(8))
            for row in data[code * 8 : (code + 1) * 8]
        )
        for code in range(len(data) // 8)
    }


TABLES = {name: glyph_table(path) for name, path in FONTS.items()}


def frame_rect(size: tuple[int, int]) -> tuple[int, int, float, float]:
    """Origin and per-pixel scale of the 320x200 frame inside a capture."""
    width, height = size
    if width == 640 and height == 400:
        # A DOSBox capture is an exact 2:1 downscale with no letterbox.
        return 0, 0, width / 320.0, height / 200.0
    frame_w = round(height * 4 / 3)
    return (width - frame_w) // 2, 0, frame_w / 320.0, height / 200.0


def decode(path: pathlib.Path) -> list[list[str]] | None:
    image = Image.open(path).convert("RGB")
    width, height = image.size
    if width < 320 or height < 200:
        return None
    origin_x, origin_y, scale_x, scale_y = frame_rect(image.size)
    pixels = image.load()
    rows = []
    for row in range(ROWS):
        line = []
        for col in range(COLS):
            bits = []
            for j in range(8):
                for i in range(8):
                    x = int(origin_x + ((LEFT + col) * 8 + i + 0.5) * scale_x)
                    y = int(origin_y + ((TOP + row) * 8 + j + 0.5) * scale_y)
                    r, g, b = pixels[
                        min(max(x, 0), width - 1), min(max(y, 0), height - 1)
                    ]
                    bits.append(1 if r + g + b > 200 else 0)
            cell = tuple(tuple(bits[j * 8 : (j + 1) * 8]) for j in range(8))
            best = None
            for name, table in TABLES.items():
                for code, glyph in table.items():
                    score = sum(
                        1
                        for j in range(8)
                        for i in range(8)
                        if glyph[j][i] == cell[j][i]
                    )
                    if best is None or score > best[0]:
                        best = (score, name, code)
            score, name, code = best
            if score < 60:
                line.append("?")
            elif name == "ibm" and code in CURSOR_GLYPHS:
                line.append("~")
            elif name == "ibm":
                line.append(chr(code) if 32 <= code < 127 else f"<{code:02x}>")
            else:
                line.append(f"[r{code:02x}]")
        rows.append(line)
    return rows


# The window's empty cell is glyph code zero, which `decode` renders as its
# escaped form rather than as a space.
BLANK_CELLS = {"<00>", "~", "?"}


def row_text(row: list[str]) -> str:
    """A decoded row as comparable text, with blanks and the cursor dropped."""
    return "".join(" " if cell in BLANK_CELLS else cell for cell in row).rstrip()


def classify(left: list[list[str]], right: list[list[str]], rows: list[int]) -> str:
    """Why a beat differs, so a suite pass can be triaged without eyeballing it.

    The buckets are the ones that actually recur. `stock-idle` and `engine-idle`
    are not conformance differences at all: one side simply had not reached the
    beat when the shot was taken, which the walk-up scenarios do routinely
    because NPC positions run off the host clock. `offset` is the same text a
    row or two adrift - a row-accounting difference, not a wording one. `cursor`
    is a single trailing cell. Only `text` is a wording difference.
    """
    stock = [row_text(row) for row in left]
    engine = [row_text(row) for row in right]
    if not any(stock):
        return "stock-idle"
    if not any(engine):
        return "engine-idle"
    # The window is `ROWS` tall, so a transcript that is out of step can be
    # adrift by almost all of it - an 8-row shift turned up in the arms shop,
    # and probing only +-3 reported it as a wording difference. Nearest shifts
    # first, so the smallest explanation wins.
    for shift in sorted(
        (s for s in range(-(ROWS - 1), ROWS) if s), key=lambda s: (abs(s), s)
    ):
        lo, hi = max(0, shift), min(ROWS, ROWS + shift)
        window = range(lo, hi)
        if not any(stock[index] for index in window):
            continue
        if all(stock[index] == engine[index - shift] for index in window):
            return f"offset{shift:+d}"
    # Two sides can also be in different *places*: a walk-up scenario whose
    # NPC did not reach the counter on one side leaves that side in the world
    # loop pressing its scripted shop keys as world commands, and every beat
    # after it disagrees on every row. That is a scenario-reliability problem,
    # not a wording one, and counting it as `text` overstates the conformance
    # queue. Rows the two sides share are the signal: a real wording difference
    # still has most of the window in common.
    stock_lines = {line for line in stock if line}
    engine_lines = {line for line in engine if line}
    if stock_lines and engine_lines:
        shared = len(stock_lines & engine_lines) / len(stock_lines | engine_lines)
        if shared < 0.2:
            return "diverged"
    if len(rows) == 1:
        a, b = stock[rows[0]], engine[rows[0]]
        if a.rstrip() == b.rstrip():
            return "cursor"
        if abs(len(a) - len(b)) <= 1 and (a.startswith(b) or b.startswith(a)):
            return "cursor"
    return "text"


def compare(artifact: pathlib.Path) -> tuple[int, int, int]:
    record = json.loads((artifact / "record.json").read_text())
    scenario = record.get("scenario", artifact.name)
    same = differ = skipped = 0
    for capture in record.get("captures", []):
        label = capture.get("label")
        stock = artifact / f"dosbox-{label}.png"
        engine = artifact / f"engine-{label}.png"
        if not stock.exists() or not engine.exists():
            skipped += 1
            continue
        left, right = decode(stock), decode(engine)
        if left is None or right is None:
            skipped += 1
            continue
        rows = [
            index
            for index, (a, b) in enumerate(zip(left, right))
            # `?` is an unmatched cell on either side - a mid-resize capture or
            # a partially drawn row - and is not a claim about the other side.
            if any(x != y and "?" not in (x, y) for x, y in zip(a, b))
        ]
        if rows:
            differ += 1
            kind = classify(left, right, rows)
            KINDS[kind] = KINDS.get(kind, 0) + 1
            print(
                f"  differ {scenario}/{label}: {kind}, rows {[r + TOP for r in rows]}"
            )
        else:
            same += 1
    return same, differ, skipped


KINDS: dict[str, int] = {}

ARTIFACTS = pathlib.Path.home() / "artifacts/u5/paired"
SCENARIOS = pathlib.Path(__file__).resolve().parent.parent / "paired"


def latest_artifacts(names: list[str]) -> list[pathlib.Path]:
    """The newest artifact directory for each named scenario."""
    if not names:
        names = sorted(
            path.stem
            for path in SCENARIOS.glob("*.tsv")
            if path.stem != "seeds"
        )
    found = []
    for name in names:
        # The artifact directory is `<scenario>-<YYYYMMDD>-<HHMMSS>`, so a bare
        # `<name>-*` glob also matches every longer scenario sharing the
        # prefix: `nb-inn` picked up `nb-inn-refusals`, and `cove-healer`
        # picked up `cove-healer-services`, silently comparing the wrong
        # scenario. Require the timestamp.
        runs = sorted(
            (
                path
                for path in ARTIFACTS.glob(f"{name}-*")
                if (path / "record.json").is_file()
                and re.fullmatch(rf"{re.escape(name)}-\d{{8}}-\d{{6}}", path.name)
            ),
            key=lambda path: path.name,
        )
        if runs:
            found.append(runs[-1])
        else:
            print(f"  no run  {name}")
    return found


def main() -> None:
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    args = sys.argv[1:]
    if args[0] == "--latest":
        args = [str(path) for path in latest_artifacts(args[1:])]
    total = [0, 0, 0]
    for arg in args:
        same, differ, skipped = compare(pathlib.Path(arg))
        status = "match" if differ == 0 else "DIFFER"
        print(f"{status} {pathlib.Path(arg).name}: {same} beat(s) agree, {differ} differ, {skipped} skipped")
        for index, value in enumerate((same, differ, skipped)):
            total[index] += value
    print(f"\n{total[0]} beat(s) agree, {total[1]} differ, {total[2]} skipped")
    if KINDS:
        # `stock-idle`/`engine-idle` are re-run candidates, `offset`/`cursor`
        # are row accounting, and `text` is the conformance queue.
        print(
            "by kind: "
            + ", ".join(f"{kind} {count}" for kind, count in sorted(KINDS.items()))
        )
    sys.exit(1 if total[1] else 0)


if __name__ == "__main__":
    main()

"""Extract the TUI play loop, rendering, and script handlers from u5-runtime.

Source line ranges (1-based, inclusive):
  part_05.rs 295-312    `pub fn run()` -- dead helper, DELETE
  part_05.rs 523-668    play loop + rendering helpers
  part_05.rs 765-897    terminal input parsers + script handler
  part_06.rs   1-55     script idle/split/label helpers

Output:
  crates/u5-tui/src/play_loop.rs  -- moved code with explicit u5_runtime imports
"""

from __future__ import annotations

from pathlib import Path

PART_05 = Path("crates/u5-runtime/src/parts/part_05.rs")
PART_06 = Path("crates/u5-runtime/src/parts/part_06.rs")
DEST = Path("crates/u5-tui/src/play_loop.rs")


def lines(p: Path) -> list[str]:
    return p.read_text(encoding="utf-8").splitlines(keepends=True)


def write_lines(p: Path, ls: list[str]) -> None:
    p.write_text("".join(ls), encoding="utf-8")


def slice_1based(ls: list[str], start: int, end_inclusive: int) -> list[str]:
    return ls[start - 1:end_inclusive]


def remove_1based(ls: list[str], start: int, end_inclusive: int) -> list[str]:
    return ls[:start - 1] + ls[end_inclusive:]


def main() -> int:
    p05 = lines(PART_05)
    p06 = lines(PART_06)

    extracted: list[str] = []
    extracted.extend(slice_1based(p05, 523, 668))
    extracted.append("\n")
    extracted.extend(slice_1based(p05, 765, 897))
    extracted.append("\n")
    extracted.extend(slice_1based(p06, 1, 55))

    header = (
        "//! Terminal play loop, rendering, input parsing, and script harness.\n"
        "//!\n"
        "//! Moved out of `u5-runtime` -- these helpers are TUI-only. Game logic\n"
        "//! and the cross-shell input dispatcher (`handle_play_key_input`,\n"
        "//! `PlayInputDisposition`) stay in `u5-runtime`.\n"
        "\n"
        "use std::collections::VecDeque;\n"
        "use std::io::{self, Write};\n"
        "use std::path::Path;\n"
        "\n"
        "use u5_runtime::{\n"
        "    Direction, PLAY_IGNORED_INPUT_KEY, PLAY_SCRIPT_MAX_IDLE_TICKS,\n"
        "    PLAY_TYPEAHEAD_TOGGLE_KEY, PlayInputDisposition, PlayOptions, PlayState, TileAtlas,\n"
        "    TileGraphicsDepth, handle_play_key_input, hash_bytes, hash_palette_indices,\n"
        "    load_tile_atlas,\n"
        "};\n"
        "\n"
    )
    write_lines(DEST, [header] + extracted)
    print(f"Wrote {DEST} ({sum(len(l) for l in extracted)} bytes, {sum(1 for l in extracted)} body lines)")

    # Now remove from sources, in reverse order so indices stay valid.
    p05 = remove_1based(p05, 765, 897)  # remove later block first
    p05 = remove_1based(p05, 523, 668)
    p05 = remove_1based(p05, 295, 312)
    write_lines(PART_05, p05)
    print(f"Updated {PART_05} ({len(p05)} lines)")

    p06 = remove_1based(p06, 1, 55)
    write_lines(PART_06, p06)
    print(f"Updated {PART_06} ({len(p06)} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

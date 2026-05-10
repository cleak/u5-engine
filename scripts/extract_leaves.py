"""Extract a batch of leaf modules from u5-runtime/src/parts/part_01.rs.

Each module is a coherent leaf (no cross-deps outside its file beyond the
small set we explicitly import). We move blocks by line range.

Targets in part_01.rs (1-based, inclusive):
  scene.rs         lines 362-611  Family + Scene + DungeonScene + PlayTarget + WorldPlane
  shrine_virtue.rs lines 612-686  ShrineVirtue (incl. preceding blank line)
  wind.rs          lines 687-761  WindState (incl. preceding blank line)

After removal we update part_01.rs and write the three new modules.
"""

from __future__ import annotations

from pathlib import Path

PART_01 = Path("crates/u5-runtime/src/parts/part_01.rs")
RUNTIME_SRC = Path("crates/u5-runtime/src")


def lines(p: Path) -> list[str]:
    return p.read_text(encoding="utf-8").splitlines(keepends=True)


def write_lines(p: Path, ls: list[str]) -> None:
    p.write_text("".join(ls), encoding="utf-8")


def slice_1based(ls: list[str], start: int, end_inclusive: int) -> str:
    return "".join(ls[start - 1:end_inclusive])


def remove_1based(ls: list[str], start: int, end_inclusive: int) -> list[str]:
    return ls[:start - 1] + ls[end_inclusive:]


def main() -> int:
    p01 = lines(PART_01)

    scene_body = slice_1based(p01, 362, 611)
    shrine_body = slice_1based(p01, 613, 686)
    wind_body = slice_1based(p01, 688, 761)

    scene_header = (
        "//! Scene partitioning: which file/index a town/castle/dungeon scene maps to,\n"
        "//! plus the world-plane (Britannia/Underworld) and the unified `PlayTarget` enum.\n"
        "\n"
        "use std::io;\n"
        "\n"
        "use crate::parse_u8_literal;\n"
        "\n"
    )
    (RUNTIME_SRC / "scene.rs").write_text(scene_header + scene_body, encoding="utf-8")
    print(f"Wrote scene.rs ({scene_body.count(chr(10))} body lines)")

    shrine_header = (
        "//! Eight-virtue shrine system: parsing, indexing, mantras.\n"
        "\n"
    )
    (RUNTIME_SRC / "shrine_virtue.rs").write_text(shrine_header + shrine_body, encoding="utf-8")
    print(f"Wrote shrine_virtue.rs ({shrine_body.count(chr(10))} body lines)")

    wind_header = (
        "//! Wind state: direction (or calm), parsing, and the `Rel Hur` cycle.\n"
        "\n"
        "use std::io;\n"
        "\n"
        "use crate::Direction;\n"
        "\n"
    )
    (RUNTIME_SRC / "wind.rs").write_text(wind_header + wind_body, encoding="utf-8")
    print(f"Wrote wind.rs ({wind_body.count(chr(10))} body lines)")

    # Remove from part_01.rs in reverse line order so indices stay valid.
    p01 = remove_1based(p01, 687, 761)  # wind
    p01 = remove_1based(p01, 612, 686)  # shrine
    p01 = remove_1based(p01, 362, 611)  # scene
    write_lines(PART_01, p01)
    print(f"Updated part_01.rs ({len(p01)} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

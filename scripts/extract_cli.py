"""Extract CliArgs and CLI parsing helpers from u5-runtime to u5-tui.

Source line ranges (1-based, inclusive) AFTER the play-loop extraction:
  part_05.rs  183-192    CliArgs struct
  part_06.rs    2-9      split_play_script
  part_06.rs   11-233    parse_cli_args + CLI_USAGE (the help string)
  part_06.rs  776-853    parse_start_arg + parse_pending_vehicle_arg (to EOF)
  part_07.rs    1-54     parse_transport_arg + parse_time_arg

Output:
  crates/u5-tui/src/cli.rs  -- moved code with explicit u5_runtime imports
"""

from __future__ import annotations

from pathlib import Path

PART_05 = Path("crates/u5-runtime/src/parts/part_05.rs")
PART_06 = Path("crates/u5-runtime/src/parts/part_06.rs")
PART_07 = Path("crates/u5-runtime/src/parts/part_07.rs")
DEST = Path("crates/u5-tui/src/cli.rs")


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
    p07 = lines(PART_07)

    cli_args_block = slice_1based(p05, 183, 192)
    split_block = slice_1based(p06, 2, 9)
    parse_cli_block = slice_1based(p06, 11, 233)
    parse_start_block = slice_1based(p06, 776, len(p06))
    parse_transport_block = slice_1based(p07, 1, 54)

    extracted: list[str] = []
    extracted.extend(cli_args_block)
    extracted.append("\n")
    extracted.extend(split_block)
    extracted.append("\n")
    extracted.extend(parse_cli_block)
    extracted.append("\n")
    extracted.extend(parse_start_block)
    extracted.append("\n")
    extracted.extend(parse_transport_block)

    header = (
        "//! CLI argument parsing, `CliArgs`, and the `--help` text.\n"
        "//!\n"
        "//! Lifted out of `u5-runtime`. The runtime owns parsing of inline\n"
        "//! command suffixes typed at the play prompt; this module owns the\n"
        "//! command-line surface.\n"
        "\n"
        "use std::env;\n"
        "use std::io;\n"
        "use std::path::PathBuf;\n"
        "\n"
        "use u5_runtime::{\n"
        "    DEFAULT_GAME_DIR, Direction, GameClock, PendingVehicleAcquisition, PlayOptions,\n"
        "    PlayTarget, SaveTemplateSource, Scene, TileGraphicsDepth, TransportState,\n"
        "    WindState, WorldPlane, decode_reagent_stock, load_play_options_from_init,\n"
        "    load_play_options_from_save, parse_cardinal_direction, parse_u8_literal,\n"
        "    split_play_script as _runtime_split_play_script, validate_dungeon_start,\n"
        "    validate_start, validate_world_start_for_transport,\n"
        "};\n"
        "\n"
    )
    # We're moving split_play_script too, so don't import it from runtime.
    # Strip the alias import we added speculatively above by rewriting.
    header = header.replace(
        "    split_play_script as _runtime_split_play_script, validate_dungeon_start,\n",
        "    validate_dungeon_start,\n",
    )

    write_lines(DEST, [header] + extracted)
    print(f"Wrote {DEST} ({len(extracted)} body lines)")

    # Remove from sources, in reverse order so indices stay valid.
    p05 = remove_1based(p05, 183, 192)
    write_lines(PART_05, p05)
    print(f"Updated {PART_05} ({len(p05)} lines)")

    # part_06: remove parse_start_arg .. EOF first, then parse_cli + CLI_USAGE,
    # then split_play_script.
    p06 = remove_1based(p06, 776, len(p06))
    p06 = remove_1based(p06, 11, 233)
    p06 = remove_1based(p06, 2, 9)
    write_lines(PART_06, p06)
    print(f"Updated {PART_06} ({len(p06)} lines)")

    p07 = remove_1based(p07, 1, 54)
    write_lines(PART_07, p07)
    print(f"Updated {PART_07} ({len(p07)} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

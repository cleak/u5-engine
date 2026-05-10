"""Carve another batch of types out of u5-runtime/src/parts/part_01.rs and part_02.rs.

Extracts (1-based, inclusive):
  party.rs       part_01.rs lines  8-32 + part_02.rs lines 1-136
                 Area, Player, PartyMember, MoonstoneGateSlot, AvatarStats,
                 increase_capped_stat, default_party, party_status_name,
                 party_member_unavailable_message
  transport.rs   part_02.rs lines 137-339
                 TransportState, PendingVehicleAcquisition,
                 BoardVehicleCandidate
  timing.rs      part_02.rs lines 340-437
                 TimingStatusTag, SaveTemplateSource, DungeonFieldEffect
  animation.rs   part_02.rs lines 439-552
                 AnimationClock, ActiveObject, PhaseTick, ActiveShipWind
  clock.rs       part_02.rs lines 554-647
                 GameClock
  npc_runtime.rs part_02.rs lines 649-735
                 RuntimeNpc, DoorTracker, LocationMarkers

Each new module uses `use crate::*;` so it picks up everything we've
already pub-used at the crate root (constants, Direction, Scene, etc.).
"""

from __future__ import annotations

from pathlib import Path

PART_01 = Path("crates/u5-runtime/src/parts/part_01.rs")
PART_02 = Path("crates/u5-runtime/src/parts/part_02.rs")
RUNTIME_SRC = Path("crates/u5-runtime/src")


def lines(p: Path) -> list[str]:
    return p.read_text(encoding="utf-8").splitlines(keepends=True)


def write_lines(p: Path, ls: list[str]) -> None:
    p.write_text("".join(ls), encoding="utf-8")


def slice_1based(ls: list[str], start: int, end_inclusive: int) -> str:
    return "".join(ls[start - 1:end_inclusive])


def remove_1based(ls: list[str], start: int, end_inclusive: int) -> list[str]:
    return ls[:start - 1] + ls[end_inclusive:]


COMMON_HEADER_TMPL = (
    "//! {summary}\n"
    "\n"
    "use std::collections::{{HashMap, VecDeque}};\n"
    "use std::fs;\n"
    "use std::io;\n"
    "use std::path::Path;\n"
    "\n"
    "use crate::*;\n"
    "\n"
)


def header(summary: str) -> str:
    return COMMON_HEADER_TMPL.format(summary=summary)


def main() -> int:
    p01 = lines(PART_01)
    p02 = lines(PART_02)

    party_body = slice_1based(p01, 8, 32) + "\n" + slice_1based(p02, 1, 136)
    transport_body = slice_1based(p02, 137, 339)
    timing_body = slice_1based(p02, 340, 437)
    animation_body = slice_1based(p02, 439, 552)
    clock_body = slice_1based(p02, 554, 647)
    npc_body = slice_1based(p02, 649, 735)

    (RUNTIME_SRC / "party.rs").write_text(
        header("Area, party roster, avatar stats, moonstone gate slots.") + party_body,
        encoding="utf-8",
    )
    print(f"Wrote party.rs")

    (RUNTIME_SRC / "transport.rs").write_text(
        header(
            "Transport state (foot/horse/ship/skiff/carpet/balloon), "
            "pending-vehicle acquisitions, and board-vehicle candidates."
        )
        + transport_body,
        encoding="utf-8",
    )
    print(f"Wrote transport.rs")

    (RUNTIME_SRC / "timing.rs").write_text(
        header(
            "Timing status tag, save-template source, dungeon field effect."
        )
        + timing_body,
        encoding="utf-8",
    )
    print(f"Wrote timing.rs")

    (RUNTIME_SRC / "animation.rs").write_text(
        header(
            "Animation clock, active object, phase ticking, active-ship wind state."
        )
        + animation_body,
        encoding="utf-8",
    )
    print(f"Wrote animation.rs")

    (RUNTIME_SRC / "clock.rs").write_text(
        header("In-game wall clock: year, month, day, hour, minute.") + clock_body,
        encoding="utf-8",
    )
    print(f"Wrote clock.rs")

    (RUNTIME_SRC / "npc_runtime.rs").write_text(
        header(
            "Runtime NPC + door tracker + location markers."
        )
        + npc_body,
        encoding="utf-8",
    )
    print(f"Wrote npc_runtime.rs")

    # Remove from sources, in reverse order so indices stay valid.
    p02 = remove_1based(p02, 649, 735)  # npc
    p02 = remove_1based(p02, 554, 647)  # clock
    p02 = remove_1based(p02, 439, 552)  # animation
    p02 = remove_1based(p02, 340, 437)  # timing
    p02 = remove_1based(p02, 137, 339)  # transport
    p02 = remove_1based(p02, 1, 136)    # party (part_02 portion)
    write_lines(PART_02, p02)
    print(f"Updated part_02.rs ({len(p02)} lines)")

    p01 = remove_1based(p01, 8, 32)
    write_lines(PART_01, p01)
    print(f"Updated part_01.rs ({len(p01)} lines)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

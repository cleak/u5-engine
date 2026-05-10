"""Split the over-sized table-IO modules into per-table files."""
from pathlib import Path
import sys
sys.path.insert(0, "scripts")
from carve_items import carve_to_module

SRC = Path("crates/u5-runtime/src")


def main() -> int:
    # World tables: split by table type.
    carve_to_module(
        dest=SRC / "world_tables_io_locations.rs",
        summary="Loaders/parsers for world location, shrine, and plane-transition tables.",
        sources=[SRC / "world_tables_io.rs"],
        items=[
            "load_world_location_entries",
            "parse_world_location_entries",
            "load_shrine_entries",
            "parse_shrine_entries",
            "load_world_plane_transition_entries",
            "parse_world_plane_transition_entries",
        ],
    )
    carve_to_module(
        dest=SRC / "world_tables_io_get_pickup.rs",
        summary="Loaders/parsers for world get-tile and object-pickup tables.",
        sources=[SRC / "world_tables_io.rs"],
        items=[
            "load_world_get_tile_entries",
            "parse_world_get_tile_entries",
            "parse_tile_get_tail",
            "parse_tile_get_guard",
            "parse_tile_get_grant",
            "load_object_pickup_entries",
            "parse_object_pickup_entries",
        ],
    )
    # The remainder of world_tables_io.rs (waterfalls, damage, encounters)
    # stays in that file.

    # Dungeon tables: split.
    carve_to_module(
        dest=SRC / "dungeon_tables_io_movement.rs",
        summary="Loaders/parsers for dungeon deeper-transition, teleport, and chest tables.",
        sources=[SRC / "dungeon_tables_io.rs"],
        items=[
            "load_dungeon_deeper_transition_entries",
            "parse_dungeon_deeper_transition_entries",
            "load_dungeon_teleport_entries",
            "parse_dungeon_teleport_entries",
            "load_dungeon_chest_content_entries",
            "parse_dungeon_chest_content_entries",
        ],
    )
    # The remainder (wind, exit, doors, secret doors) stays.

    # Town tables: split into stairs/locks/exits vs everything else.
    carve_to_module(
        dest=SRC / "town_tables_io_movement.rs",
        summary="Loaders/parsers for town stair, trap-door, exit, and lock tables.",
        sources=[SRC / "town_tables_io.rs"],
        items=[
            "load_town_stair_entries",
            "parse_town_stair_entries",
            "parse_town_stair_kind",
            "load_town_trap_door_entries",
            "parse_town_trap_door_entries",
            "load_town_exit_tile_entries",
            "parse_town_exit_tile_entries",
            "load_town_lock_entries",
            "parse_town_lock_entries",
            "parse_town_lock_kind",
        ],
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

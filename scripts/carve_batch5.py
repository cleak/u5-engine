"""Batch 5: clear out parts 05/09-15 into proper modules."""

from __future__ import annotations

from pathlib import Path

from carve_items import carve_to_module

PARTS = Path("crates/u5-runtime/src/parts")
SRC = Path("crates/u5-runtime/src")


def main() -> int:
    # World table loaders/parsers.
    carve_to_module(
        dest=SRC / "world_tables_io.rs",
        summary="Loaders and parsers for the world TSV tables.",
        sources=[PARTS / "part_09.rs", PARTS / "part_10.rs"],
        items=[
            "load_world_location_entries",
            "parse_world_location_entries",
            "load_shrine_entries",
            "parse_shrine_entries",
            "load_world_plane_transition_entries",
            "parse_world_plane_transition_entries",
            "load_world_get_tile_entries",
            "parse_world_get_tile_entries",
            "parse_tile_get_tail",
            "parse_tile_get_guard",
            "parse_tile_get_grant",
            "load_object_pickup_entries",
            "parse_object_pickup_entries",
            "load_world_waterfall_entries",
            "parse_world_waterfall_entries",
            "load_world_damage_tile_entries",
            "parse_world_damage_tile_entries",
            "load_world_encounter_entries",
            "parse_world_encounter_entries",
        ],
    )

    # Dungeon table loaders/parsers.
    carve_to_module(
        dest=SRC / "dungeon_tables_io.rs",
        summary="Loaders and parsers for the dungeon TSV tables.",
        sources=[PARTS / "part_10.rs", PARTS / "part_11.rs"],
        items=[
            "load_dungeon_deeper_transition_entries",
            "parse_dungeon_deeper_transition_entries",
            "load_dungeon_teleport_entries",
            "parse_dungeon_teleport_entries",
            "load_dungeon_chest_content_entries",
            "parse_dungeon_chest_content_entries",
            "load_dungeon_wind_tile_entries",
            "parse_dungeon_wind_tile_entries",
            "load_dungeon_exit_tile_entries",
            "parse_dungeon_exit_tile_entries",
            "load_dungeon_door_entries",
            "parse_dungeon_door_entries",
            "load_secret_door_entries",
            "parse_secret_door_entries",
        ],
    )

    # Town table loaders/parsers.
    carve_to_module(
        dest=SRC / "town_tables_io.rs",
        summary="Loaders and parsers for the town TSV tables.",
        sources=[PARTS / "part_11.rs", PARTS / "part_12.rs", PARTS / "part_13.rs"],
        items=[
            "load_town_fire_source_entries",
            "parse_town_fire_source_entries",
            "load_town_pushable_entries",
            "parse_town_pushable_entries",
            "load_town_get_tile_entries",
            "parse_town_get_tile_entries",
            "load_town_rest_bed_entries",
            "parse_town_rest_bed_entries",
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

    # Misc table loaders + small parsing helpers.
    carve_to_module(
        dest=SRC / "misc_tables_io.rs",
        summary="Loaders/parsers for blink targets, moongates, location floor pages, location entry-y, world overlay objects.",
        sources=[PARTS / "part_13.rs"],
        items=[
            "load_blink_target_entries",
            "parse_blink_target_entries",
            "validate_blink_target_bounds",
            "parse_cardinal_direction_field",
            "parse_optional_u8_literal",
            "load_moongate_entries",
            "parse_moongate_entries",
            "parse_moongate_tile_field",
            "parse_hour_field",
            "load_location_floor_entries",
            "parse_location_floor_entries",
            "load_location_entry_y",
            "load_location_entry_y_entries",
            "parse_location_entry_y_entries",
            "load_world_overlay_objects",
            "load_init_overlay_objects",
        ],
    )

    # Active-object encode/decode, save mirroring.
    carve_to_module(
        dest=SRC / "active_object_io.rs",
        summary="Active-object encode/decode for SAVED.OOL mirroring + write helpers.",
        sources=[PARTS / "part_14.rs"],
        items=[
            "refresh_saved_ool_mirrors_for_load",
            "read_saved_ool_bytes",
            "encode_active_object_table",
            "encode_ool_plane_objects",
            "write_active_object_record",
            "decode_ool_plane_objects",
            "decode_saved_active_objects",
            "decode_active_object_table",
        ],
    )

    # Map decoders + analysis.
    carve_to_module(
        dest=SRC / "map_decoders.rs",
        summary="Britannia/underworld chunk decoders, BRIT.DAT chunk index finder, map analysis (analyze_map, harvest_location_markers, etc.), pathfinding helpers.",
        sources=[PARTS / "part_14.rs"],
        items=[
            "decode_world_map_bytes",
            "decode_underworld_map_bytes",
            "find_britannia_chunk_index",
            "validate_britannia_chunk_index",
            "decode_britannia_map_bytes",
            "analyze_map",
            "harvest_location_markers",
            "scrub_location_entry_markers",
            "is_location_entry_marker",
            "is_spawn_marker",
            "is_npc_start_marker",
            "append_map_stats",
            "find_path",
            "door_probe",
            "neighbors",
            "first_walkable",
            "first_distinct_walkable",
        ],
    )

    # Tile predicates + table-match helpers.
    carve_to_module(
        dest=SRC / "predicates.rs",
        summary="Tile passability/water/lava/door predicates plus table-match helpers used during runtime checks.",
        sources=[PARTS / "part_14.rs"],
        items=[
            "is_probe_walkable",
            "is_tile_walkable",
            "is_base_tile_passable",
            "is_tile_walkable_for_transport",
            "is_water_tile",
            "static_tile_animation_family_base",
            "is_lava_tile",
            "is_mountain_tile",
            "is_outdoor_climbable_tile",
            "is_mountain_or_lava",
            "is_wall_or_closed_door_tile",
            "is_talk_through_tile",
            "is_horse_fast_stride_tile",
            "is_town_night_hour",
            "cell_in_visibility_radius",
            "surface_line_unblocked",
            "rounded_div",
            "surface_tile_blocks_sight",
            "town_fire_source_is_adjacent",
            "town_fire_source_tile_matches",
            "dungeon_wind_tile_matches",
            "dungeon_teleport_matches",
            "dungeon_exit_tile_matches",
            "dungeon_closed_door_matches",
            "town_pushable_matches",
            "world_get_tile_matches",
            "object_pickup_matches",
            "world_waterfall_matches",
            "world_damage_tile_matches",
            "world_damage_tile_entry_at",
            "town_get_tile_matches",
            "town_rest_bed_matches",
            "town_stair_matches",
            "town_trap_door_matches",
            "town_exit_tile_matches",
            "town_lock_matches",
            "apply_dawn_dusk_substitution",
            "world_cell_index",
            "first_world_walkable_for_transport",
            "world_start_safe_for_transport",
        ],
    )

    # Misc helpers from part_15.
    carve_to_module(
        dest=SRC / "tile_helpers.rs",
        summary="Tile/glyph rendering, NPC tile helpers, transport conversion, direction phase helpers, hashing, world scroll math, byte readers.",
        sources=[PARTS / "part_15.rs"],
        items=[
            "place_pending_vehicle_acquisition",
            "dungeon_cell_index",
            "first_dungeon_walkable",
            "is_dungeon_walkable",
            "is_dungeon_fall_trap",
            "is_dungeon_bomb_trap",
            "dungeon_field_effect",
            "is_dungeon_room_trigger",
            "is_dungeon_room_helper_state",
            "dungeon_room_slot",
            "dungeon_room_arena_index",
            "stair_delta",
            "town_climb_delta",
            "dungeon_ladder_delta",
            "render_glyph",
            "render_dungeon_glyph",
            "npc_tile",
            "npc_active_object",
            "active_object_matches_runtime_npc",
            "player_phantom_active_object",
            "step_toward",
            "tile_class",
            "transport_from_vehicle_object",
            "transport_from_save_marker",
            "active_object_frame_tile",
            "is_ambient_wanderer_object",
            "is_ship_object",
            "direction_from_active_object_phase",
            "active_object_phase_from_direction",
            "active_object_phase_toward_player",
            "cardinal_direction_from_active_object_phase",
            "is_vehicle_object_tile",
            "dungeon_cell_class",
            "dungeon_look_description",
            "dungeon_search_description",
            "render_class_byte",
            "waypoint_for_hour",
            "in_wrapping_range",
            "names",
            "contains_all",
            "contains_any",
            "sample_names",
            "hash_palette_indices",
            "hash_bytes",
            "compact",
            "manhattan",
            "world_scroll_base",
            "world_scroll_base_axis",
            "world_scroll_neighborhood_contains",
            "world_scroll_axis_offset",
            "u16_at",
            "u32_at",
            "write_u16_at",
        ],
    )

    # The original Lord-British verification report.
    carve_to_module(
        dest=SRC / "report.rs",
        summary="The Lord-British throne-room verification report (`run_report`).",
        sources=[PARTS / "part_05.rs"],
        items=["run_report"],
    )

    # Cross-shell input dispatcher (used by both u5-tui and u5-bevy).
    carve_to_module(
        dest=SRC / "input_dispatch.rs",
        summary="The shell-agnostic input dispatcher: takes a key + suffix, mutates PlayState, returns whether to keep going. Used by both u5-tui (terminal) and u5-bevy (window).",
        sources=[PARTS / "part_05.rs"],
        items=["PlayInputDisposition", "handle_play_key_input"],
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

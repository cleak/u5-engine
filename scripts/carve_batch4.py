"""Batch 4: PlayOptions + small option enums, save loaders, start validation,
binary readers, LZW codec, tile atlas IO, graphics IO, TLK, NPC block, map loaders."""

from __future__ import annotations

from pathlib import Path

from carve_items import carve_to_module

PARTS = Path("crates/u5-runtime/src/parts")
SRC = Path("crates/u5-runtime/src")


def main() -> int:
    # Play options + small action enums.
    carve_to_module(
        dest=SRC / "play_options.rs",
        summary="PlayOptions + small action enums + moonstone gate helpers + initial overlay cache.",
        sources=[PARTS / "part_05.rs"],
        items=[
            "PlayOptions",
            "initial_world_overlay_cache",
            "GateTravelDestination",
            "gate_travel_destination",
            "moonstone_slot_matches_world",
            "moonstone_slot_matches_town",
            "moonstone_bury_tile_allowed",
            "MoveOutcome",
            "UseItemRequest",
            "AreaTransition",
            "ClimbIntent",
            "NpcSlot",
            "MapStats",
        ],
    )

    # Save/load: PlayOptions seeders + decode helpers.
    carve_to_module(
        dest=SRC / "save_load.rs",
        summary="Loaders that turn SAVED.GAM/SAVED.OOL/INIT.GAM into PlayOptions.",
        sources=[PARTS / "part_06.rs"],
        items=[
            "load_play_options_from_save",
            "load_play_options_from_init",
            "load_save_image_template",
            "read_save_image_file",
            "load_play_options_from_save_file",
            "play_options_from_save_bytes",
            "play_options_from_save_bytes_named",
            "decode_reagent_stock",
            "encode_reagent_stock",
            "decode_avatar_stats",
            "decode_save_party",
            "decode_moonstone_gate_slots",
            "saved_game_has_avatar_name",
        ],
    )

    # Inline parsers + prompt messages + spell helpers (the rest of part_06).
    carve_to_module(
        dest=SRC / "inline_parsers.rs",
        summary="Inline parsers for command suffixes typed at the play prompt, spell-code helpers, and prompt messages.",
        sources=[PARTS / "part_06.rs"],
        items=[
            "parse_u8_literal",
            "parse_i8_literal",
            "parse_cardinal_direction",
            "parse_inline_hours",
            "moonstone_phase_from_inline_number",
            "parse_inline_use_request",
            "parse_inline_cardinal_direction",
            "parse_inline_yes_no",
            "parse_inline_party_index",
            "parse_inline_target_party_index",
            "parse_inline_party_swap",
            "parse_inline_gate_phase_index",
            "InlineMixRequest",
            "InlineShrineRequest",
            "parse_inline_mix_request",
            "inline_mix_candidate",
            "parse_inline_shrine_request",
            "mix_prompt_message",
            "shrine_prompt_message",
            "cast_prompt_message",
            "use_prompt_message",
            "new_order_prompt_message",
            "inline_spell_code",
            "spell_index_from_code",
            "spell_scene_bit_for_area",
            "spell_allowed_in_area",
            "selected_reagent_indices",
        ],
    )

    # Start validation + small binary readers.
    carve_to_module(
        dest=SRC / "start_validation.rs",
        summary="Validation of start coordinates against passability, plus tiny IO/format helpers.",
        sources=[PARTS / "part_07.rs"],
        items=[
            "validate_start",
            "validate_dungeon_start",
            "is_public_dungeon_reaction_seed",
            "validate_world_start_for_transport",
            "pass_fail",
            "read",
            "load_tile_passability",
            "load_look_table",
            "parse_look2_dat",
        ],
    )

    # LZW + tile atlas + graphic image directory + sprite sheet IO.
    carve_to_module(
        dest=SRC / "graphics_io.rs",
        summary="LZW codec, tile-atlas loading, GraphicImage* parsing, sprite-sheet parsing, monochrome-bitmap parsing.",
        sources=[PARTS / "part_07.rs", PARTS / "part_08.rs"],
        items=[
            "LzwBitReader",
            "reset_lzw_dictionary",
            "decode_lzw_envelope",
            "decode_gif_lzw_payload",
            "load_tile_atlas",
            "parse_tile_atlas",
            "unpack_tile_atlas_body",
            "blit_tile_to_viewport",
            "tile_graphics_file_name",
            "load_graphic_image_directory",
            "parse_graphic_image_directory",
            "parse_graphic_image_directory_body",
            "load_graphic_sprite_sheet",
            "parse_graphic_sprite_sheet",
            "parse_graphic_sprite_sheet_body",
            "parse_graphic_image_block",
            "parse_graphic_mask_block",
            "graphic_image_row_stride",
            "unpack_graphic_pixels",
            "load_title_bit",
            "parse_title_bit",
            "parse_title_bit_body",
            "load_british_bit",
            "parse_british_bit",
            "load_wd_bit",
            "parse_wd_bit",
            "parse_single_image_bit_body",
            "parse_monochrome_bitmap_block",
            "parse_monochrome_bitmap_payload",
            "monochrome_bitmap_payload_len",
            "unpack_monochrome_bits",
            "load_ch_font",
            "load_hcs_font",
            "parse_fixed_font_body",
            "load_proportional_font",
            "parse_proportional_font",
            "parse_proportional_font_body",
            "monochrome_row_stride",
            "unpack_monochrome_rows",
            "prepare_fixed_text_cell",
            "rasterize_fixed_text_line",
            "measure_proportional_text",
            "rasterize_proportional_text_line",
            "blit_monochrome_bitmap",
        ],
    )

    # TLK / NPC block + scene/dungeon/world loading.
    carve_to_module(
        dest=SRC / "map_io.rs",
        summary="TLK parsing, NPC block parsing, and scene/dungeon/world map loaders + decoders.",
        sources=[PARTS / "part_08.rs"],
        items=[
            "parse_tlk",
            "parse_tlk_bytes",
            "decode_tlk_field",
            "non_empty_talk_keyword",
            "talk_keyword_response",
            "talk_keyword_matches",
            "talk_keyword_compare_text",
            "parse_npc_block",
            "load_floor",
            "load_town_runtime_floor",
            "normalize_town_runtime_floor",
            "resolve_location_floor_page",
            "load_dungeon_record",
            "load_world_map",
        ],
    )

    return 0


if __name__ == "__main__":
    raise SystemExit(main())

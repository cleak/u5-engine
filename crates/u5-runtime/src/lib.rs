//! Game runtime for the Ultima V clean-room implementation.
//!
//! This crate owns the simulation, parsers, and rules. It has no UI
//! dependencies. UI shells (`u5-tui`, `u5-bevy`) consume its public API.

pub mod active_object_io;
pub mod animation;
pub mod blackthorn;
pub mod boot;
pub mod character_record;
pub mod chargen;
pub mod clock;
pub mod combat_actor;
pub mod commands;
pub mod combat_arena;
pub mod combat_frame;
pub mod combat_setup;
pub mod combat_stats;
pub mod constants;
pub mod containers;
pub mod directed_step;
pub mod direction;
pub mod dungeon_tables;
pub mod dungeon_tables_io;
pub mod dungeon_tables_io_movement;
pub mod endgame;
pub mod equipment;
pub mod fonts_io;
pub mod graphics;
pub mod graphics_io;
pub mod hidden_treasures;
pub mod inline_parsers;
pub mod input_codes;
pub mod input_dispatch;
pub mod intro;
pub mod jimmy;
pub mod karma;
pub mod lighting;
pub mod magic;
pub mod main_loop;
pub mod moongate;
pub mod lzw;
pub mod map_decoders;
pub mod map_io;
pub mod misc_tables;
pub mod misc_tables_io;
pub mod npc_runtime;
pub mod party;
pub mod play_options;
pub mod play_state_impl;
pub mod play_state_struct;
pub mod predicates;
pub mod prng;
pub mod report;
pub mod save_load;
pub mod scene;
pub mod end_io;
pub mod endmsg_io;
pub mod miscmsg_io;
pub mod question_io;
pub mod quest_flags;
pub mod shops;
pub mod shrine_virtue;
pub mod signs_io;
pub mod stat_arithmetic;
pub mod story_io;
pub mod text_wrap;
pub mod tlk_control_codes;
pub mod view_classes;
pub mod start_validation;
pub mod test_fixtures;
pub mod tile_classes;
pub mod tile_helpers;
pub mod timing;
pub mod town_mode;
pub mod town_tables;
pub mod town_tables_io;
pub mod town_tables_io_movement;
pub mod transport;
pub mod traps;
pub mod visibility;
pub mod u4_transfer;
pub mod wind;
pub mod world_tables;
pub mod world_tables_io;
pub mod world_tables_io_get_pickup;
pub mod world_tables_io_locations;

pub use active_object_io::*;
pub use animation::{ActiveObject, ActiveShipWind, AnimationClock, PhaseTick};
pub use character_record::{
    CharacterClass, CharacterStatus, character_class_for_byte, character_status_for_byte,
};
pub use chargen::*;
pub use clock::{
    GameClock, SKY_STRIP_CELL_COUNT, SkyStripMarker, shop_time_of_day_word,
    sky_strip_marker_position,
};
pub use directed_step::{
    Axis, axis_first_choice, directed_step_offsets, terrain_chance_gate_denominator,
    type_bypasses_terrain_chance_gate,
};
pub use end_io::{EndNarrative, decode_end_window, load_end_narrative};
pub use blackthorn::{
    BLACKTHORN_RESCUE_HANDOFF_SCENE, BLACKTHORN_RESCUE_HANDOFF_X, BLACKTHORN_RESCUE_HANDOFF_Y,
    BLACKTHORN_RESCUE_STANDING_FLOOR, blackthorn_rescue_verdict_record,
    lord_british_camp_verdict_record,
};
pub use boot::{
    DisplayDriverFamily, GraphicsCapability, TANDY_LOW_MEMORY_THRESHOLD_KB,
    parse_explicit_driver_selector, resolve_driver_family, tandy_low_memory_downgrades,
};
pub use commands::{Command, command_for_letter};
pub use containers::{
    DUNGEON_CHEST_ROWS, DungeonChestReward, DungeonChestRow, InventoryAddClass,
    TABLE_FOOD_TILE_A, TABLE_FOOD_TILE_B, dungeon_chest_row_awarded,
    dungeon_chest_row_gate_max, equipment_grant_quantity, inventory_add_class,
    table_food_get_resulting_tile,
};
pub use intro::{IntroMenuAction, intro_menu_action};
pub use hidden_treasures::{
    HIDDEN_TREASURE_RECORD_DAILY_CACHE, HIDDEN_TREASURE_RECORD_KEY_NPC_GATED,
    HIDDEN_TREASURE_RECORD_SINGLE_USE_NPC_GATED, HiddenTreasureRule,
    hidden_treasure_can_stage, hidden_treasure_rule,
};
pub use input_codes::{
    INPUT_CODE_EAST, INPUT_CODE_NORTH, INPUT_CODE_NORTHEAST, INPUT_CODE_NORTHWEST,
    INPUT_CODE_SOUTH, INPUT_CODE_SOUTHEAST, INPUT_CODE_SOUTHWEST, INPUT_CODE_WEST,
    InputDirection, input_case_fold, input_code_direction,
};
pub use jimmy::{
    DOOR_AUTO_CLOSE_TURNS, JIMMY_DOOR_DIE_HIGH, JIMMY_DOOR_DIE_LOW, JIMMY_OBJECT_DIE_HIGH,
    JIMMY_OBJECT_DIE_LOW, dungeon_chest_jimmy_succeeds, dungeon_chest_jimmy_threshold,
    jimmy_door_succeeds, object_chest_jimmy_succeeds, object_chest_jimmy_threshold,
};
pub use karma::{KarmaAction, apply_karma_action};
pub use magic::{
    CastGateOutcome, RUNE_SYLLABLE_VOCABULARY, cast_dispatcher_gate,
    heal_spell_amount_from_raw_roll_u8, is_resident_rune_syllable,
    spell_common_name, spell_indoor_absorbs,
};
pub use moongate::{
    NARRATIVE_GATE_X, NARRATIVE_GATE_Y, NaturalMoongateCounterStep, SURFACE_CHASM_X,
    SURFACE_CHASM_Y, is_surface_chasm_cell, natural_moongate_advance_counter,
    natural_moongate_cached_glyph_slot, natural_moongate_counter_step,
    natural_moongate_dispatches_meditate, natural_moongate_slot_eligible,
};
pub use main_loop::{
    DUNGEON_FACING_EAST, DUNGEON_FACING_NORTH, DUNGEON_FACING_SOUTH, DUNGEON_FACING_WEST,
    DungeonEntrySeed, SCENE_COMBAT_TEMPORARY, SCENE_DUNGEON_FAMILY_FIRST,
    SCENE_DUNGEON_FAMILY_LAST, SCENE_DUNGEON_NAMED_FIRST, SCENE_DUNGEON_NAMED_LAST,
    SCENE_INTRO_FIRST, SCENE_INTRO_LAST, SCENE_OVERWORLD, SCENE_TOWN_FAMILY_FIRST,
    SCENE_TOWN_FAMILY_LAST, SceneRoute, dungeon_entry_seed, dungeon_record_index,
    dungeon_resident_name, mode_minute_increment, scene_route,
};
pub use lighting::{
    GREAT_LIGHT_SPELL_DURATION, LIGHT_SPELL_DURATION, ambient_is_sentinel,
    apply_personal_light, daylight_base_value, decay_light_counter, dungeon_blackout,
    ignite_torch_dungeon, ignite_torch_surface,
};
pub use endmsg_io::{EndgameMessages, load_endgame_messages, parse_endgame_messages};
pub use miscmsg_io::{MiscMessages, load_misc_messages, parse_misc_messages};
pub use quest_flags::{
    ConversationLetterAction, conversation_letter_action, tlk_scene_branch_is_set,
    tlk_scene_branch_mask, tlk_scene_branch_set,
};
pub use question_io::{QuestionRecords, load_question_records, parse_question_records};
pub use signs_io::{
    SignRecord, decode_sign_payload, find_sign, load_sign_records, parse_sign_records,
};
pub use stat_arithmetic::{capped_add_u8, capped_add_word, floor_sub_u8, floor_sub_word};
pub use story_io::{
    INTRO_AUTO_OPENING_STEP, INTRO_INLINE_DOORWAY_STEP, INTRO_STORY_STEP_COUNT,
    IntroStoryArtPlacement, StoryRecords, intro_story_art_file_for_step,
    intro_story_art_placement_for_step, load_story_records, parse_story_records,
};
pub use text_wrap::{
    ParagraphByteKind, WRAP_MIN_LINE_BUFFER, WrapByteKind, WrappedLine, paragraph_byte_kind,
    wrap_byte_kind, wrap_text,
};
pub use tlk_control_codes::{
    TLK_CODE_ACTION_DISPATCH, TLK_CODE_ASK_PARTY_NAME, TLK_CODE_ASK_WHO,
    TLK_CODE_CURSE_CHECK, TLK_CODE_END_OF_RESPONSE, TLK_CODE_END_STREAM,
    TLK_CODE_GOLD_PAYMENT, TLK_CODE_GOTO_LABEL_FIRST, TLK_CODE_GOTO_LABEL_LAST,
    TLK_CODE_IF_ELSE, TLK_CODE_IF_ELSE_ALT, TLK_CODE_LITERAL_NEWLINE,
    TLK_CODE_PANEL_NEWLINE, TLK_CODE_PAUSE, TLK_CODE_PRINT_AVATAR_NAME,
    TLK_CODE_PROTECT_RUN, TLK_CODE_SET_FLAG, TLK_CODE_WAIT_KEY,
    TLK_INPUT_MAX_LEN, TLK_LABEL_FIRST, TLK_LABEL_LAST, ReservedKeywordEffect,
    TlkByteKind, classify_tlk_byte, is_tlk_label_byte, reserved_keyword_effect,
    tlk_introducer_argument_count, tlk_keyword_matches,
};
pub use tile_classes::{
    TILE_BARRIER_FIRST, TILE_BARRIER_LAST, TILE_DECORATION_FIRST, TILE_DECORATION_LAST,
    TILE_DOOR_FIRST, TILE_DOOR_LAST, TILE_FURNITURE_FIRST, TILE_FURNITURE_LAST,
    TILE_NPC_FIRST, TILE_NPC_LAST, TILE_PATH_FIRST, TILE_PATH_LAST, TILE_SPECIAL_FIRST,
    TILE_SPECIAL_LAST, TILE_TERRAIN_FIRST, TILE_TERRAIN_LAST, TILE_VEHICLE_ART_FIRST,
    TILE_VEHICLE_ART_LAST, TILE_VEHICLE_FIRST, TILE_VEHICLE_LAST, TILE_WALL_FIRST,
    TILE_WALL_LAST, TILE_WATER_FIRST, TILE_WATER_LAST, TileClass, TileSuperCategory,
    coarse_tile_class, tile_animation_cycle_length, tile_super_category,
};
pub use view_classes::{fc_sprite_proximity_mask_hits, tile_view_class};
pub use combat_actor::*;
pub use combat_arena::*;
pub use combat_frame::*;
pub use combat_setup::*;
pub use combat_stats::*;
pub use constants::*;
pub use direction::Direction;
pub use dungeon_tables::*;
pub use dungeon_tables_io::*;
pub use dungeon_tables_io_movement::*;
pub use endgame::*;
pub use equipment::*;
pub use fonts_io::*;
pub use graphics::*;
pub use graphics_io::*;
pub use inline_parsers::*;
pub use input_dispatch::{PlayInputDisposition, handle_play_key_input};
pub use lzw::*;
pub use map_decoders::*;
pub use map_io::*;
pub use misc_tables::*;
pub use misc_tables_io::*;
pub use npc_runtime::{
    DoorTracker, LocationMarkers, NPC_DYNAMIC_OBSTACLE_MANHATTAN_RADIUS,
    NPC_STATE_ASCEND_TOWARD_TARGET, NPC_STATE_CLIMB_DOWN_OFF_FLOOR,
    NPC_STATE_CLIMB_UP_OFF_FLOOR, NPC_STATE_DESCEND_TOWARD_TARGET, NPC_STATE_EMPTY,
    NPC_STATE_IDLE, NPC_STATE_INPLANE_MOVE, NPC_STATE_PARKED_OFF_FLOOR, NPC_STATE_REPLAY_QUEUE,
    NPC_FLOOR_LINK_TILE_C8, NPC_FLOOR_LINK_TILE_C9, NPC_PATH_DIR_EAST, NPC_PATH_DIR_NORTH,
    NPC_PATH_DIR_SOUTH, NPC_PATH_DIR_WEST, NPC_PATHFIND_QUEUE_CAPACITY,
    NPC_STUCK_REPLAN_THRESHOLD, NpcAiBehavior, NpcShopTrigger, RuntimeNpc, npc_ai_behavior,
    npc_path_direction_offset, npc_path_direction_opposite, npc_shop_trigger,
    schedule_floor_state,
};
pub use party::{
    Area, AvatarStats, MoonstoneGateSlot, PartyMember, Player, class_refreshed_mana, default_party,
    default_party_experience, default_party_intelligence, default_party_names,
    default_party_stay_counters, heal_spell_amount_from_raw_roll, increase_capped_stat,
    party_member_unavailable_message, party_name_to_string, party_status_name,
    potion_effect_index_after_variation, potion_label, recompute_level_from_experience,
    resurrection_adjusted_experience,
};
pub use play_options::*;
pub use play_state_struct::{PlayState, WorldOverlayCache, WorldReturn};
pub use predicates::*;
pub use prng::*;
pub use report::run_report;
pub use save_load::*;
pub use scene::{DungeonScene, Family, PlayTarget, Scene, WorldPlane};
pub use shops::*;
pub use shrine_virtue::{
    CodexUrnReadOutcome, ShrineQuestState, ShrineVirtue, all_virtues_complete, read_codex_urn,
};
pub use start_validation::*;
pub use tile_helpers::*;
pub use timing::{DungeonFieldEffect, SaveTemplateSource, TimingStatusTag};
pub use town_mode::{
    TOWN_GRID_BYTES, TOWN_GRID_SIDE, TOWN_NPC_BLOCK_BYTES, TOWN_NPC_ROSTER_SLOTS,
    TownLocationClass, town_floor_offset, town_location_class, town_per_class_index,
    town_resident_name,
};
pub use town_tables::*;
pub use town_tables_io::*;
pub use town_tables_io_movement::*;
pub use transport::{
    BoardVehicleCandidate, BoardableFamily, CARPET_MOUNTED, CARPET_PARKED,
    HORSE_MOUNTED_FIRST, HORSE_MOUNTED_LAST, HORSE_PARKED_FIRST, HORSE_PARKED_LAST,
    PendingVehicleAcquisition, SHIP_BOARDING_HULL_WARNING_THRESHOLD, SHIP_PARKED_FIRST,
    SHIP_PARKED_LAST, SKIFF_PARKED_FIRST, SKIFF_PARKED_LAST, TransportState,
    boardable_family, mount_horse_marker, ship_boarding_warns,
};
pub use traps::*;
pub use u4_transfer::*;
pub use visibility::{
    LightRadiusBranch, TERRAIN_BAND_ROW_STRIDE, VIEWPORT_PLAYER_COL, VIEWPORT_PLAYER_ROW,
    VIEWPORT_ROW_STRIDE, VIEWPORT_SIDE, VISIBILITY_ALREADY_RENDERED, VISIBILITY_CLEAR,
    VISIBILITY_DIM_PERIPHERY, VISIBILITY_HIDDEN, VISIBILITY_USE_COMPANION, VisibilityMarker,
    light_radius_branch, visibility_marker,
};
pub use wind::WindState;
pub use world_tables::*;
pub use world_tables_io::*;
pub use world_tables_io_get_pickup::*;
pub use world_tables_io_locations::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use super::*;
    use crate::test_fixtures::*;
    include!("tests_inline/chunk_01.rs");
    include!("tests_inline/chunk_02.rs");
    include!("tests_inline/chunk_03.rs");
    include!("tests_inline/chunk_04.rs");
    include!("tests_inline/chunk_05.rs");
    include!("tests_inline/chunk_06.rs");
    include!("tests_inline/chunk_07.rs");
    include!("tests_inline/chunk_08.rs");
    include!("tests_inline/chunk_09.rs");
    include!("tests_inline/chunk_10.rs");
    include!("tests_inline/chunk_11.rs");
    include!("tests_inline/chunk_12.rs");
    include!("tests_inline/chunk_13.rs");
    include!("tests_inline/chunk_14.rs");
    include!("tests_inline/chunk_15.rs");
    include!("tests_inline/chunk_16.rs");
    include!("tests_inline/chunk_17.rs");
    include!("tests_inline/chunk_18.rs");
    include!("tests_inline/chunk_19.rs");
    include!("tests_inline/chunk_20.rs");
    include!("tests_inline/chunk_21.rs");
    include!("tests_inline/chunk_22.rs");
    include!("tests_inline/chunk_23.rs");
}

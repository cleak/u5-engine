//! Game runtime for the Ultima V clean-room implementation.
//!
//! This crate owns the simulation, parsers, and rules. It has no UI
//! dependencies. UI shells (`u5-tui`, `u5-bevy`) consume its public API.

pub mod active_object_io;
pub mod animation;
pub mod chargen;
pub mod clock;
pub mod combat_actor;
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
pub mod inline_parsers;
pub mod input_dispatch;
pub mod jimmy;
pub mod karma;
pub mod lighting;
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
pub use containers::{
    DUNGEON_CHEST_ROWS, DungeonChestReward, DungeonChestRow, TABLE_FOOD_TILE_A,
    TABLE_FOOD_TILE_B, dungeon_chest_row_awarded, dungeon_chest_row_gate_max,
    table_food_get_resulting_tile,
};
pub use jimmy::{
    DOOR_AUTO_CLOSE_TURNS, JIMMY_DOOR_DIE_HIGH, JIMMY_DOOR_DIE_LOW, JIMMY_OBJECT_DIE_HIGH,
    JIMMY_OBJECT_DIE_LOW, dungeon_chest_jimmy_succeeds, dungeon_chest_jimmy_threshold,
    jimmy_door_succeeds, object_chest_jimmy_succeeds, object_chest_jimmy_threshold,
};
pub use karma::{KarmaAction, apply_karma_action};
pub use lighting::{
    GREAT_LIGHT_SPELL_DURATION, LIGHT_SPELL_DURATION, ambient_is_sentinel,
    apply_personal_light, decay_light_counter, dungeon_blackout, ignite_torch_dungeon,
    ignite_torch_surface,
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
pub use text_wrap::{WrappedLine, wrap_text};
pub use tlk_control_codes::{
    TLK_CODE_ACTION_DISPATCH, TLK_CODE_ASK_PARTY_NAME, TLK_CODE_ASK_WHO,
    TLK_CODE_CURSE_CHECK, TLK_CODE_END_OF_RESPONSE, TLK_CODE_END_STREAM,
    TLK_CODE_GOLD_PAYMENT, TLK_CODE_GOTO_LABEL_FIRST, TLK_CODE_GOTO_LABEL_LAST,
    TLK_CODE_IF_ELSE, TLK_CODE_IF_ELSE_ALT, TLK_CODE_LITERAL_NEWLINE,
    TLK_CODE_PANEL_NEWLINE, TLK_CODE_PAUSE, TLK_CODE_PRINT_AVATAR_NAME,
    TLK_CODE_PROTECT_RUN, TLK_CODE_SET_FLAG, TLK_CODE_WAIT_KEY,
    TLK_LABEL_FIRST, TLK_LABEL_LAST, TlkByteKind, classify_tlk_byte,
    is_tlk_label_byte, tlk_introducer_argument_count,
};
pub use tile_classes::{
    TILE_BARRIER_FIRST, TILE_BARRIER_LAST, TILE_DECORATION_FIRST, TILE_DECORATION_LAST,
    TILE_DOOR_FIRST, TILE_DOOR_LAST, TILE_FURNITURE_FIRST, TILE_FURNITURE_LAST,
    TILE_NPC_FIRST, TILE_NPC_LAST, TILE_PATH_FIRST, TILE_PATH_LAST, TILE_SPECIAL_FIRST,
    TILE_SPECIAL_LAST, TILE_TERRAIN_FIRST, TILE_TERRAIN_LAST, TILE_VEHICLE_ART_FIRST,
    TILE_VEHICLE_ART_LAST, TILE_VEHICLE_FIRST, TILE_VEHICLE_LAST, TILE_WALL_FIRST,
    TILE_WALL_LAST, TILE_WATER_FIRST, TILE_WATER_LAST, TileClass, coarse_tile_class,
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
    NPC_STUCK_REPLAN_THRESHOLD, RuntimeNpc,
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
pub use shrine_virtue::{CodexUrnReadOutcome, ShrineVirtue, read_codex_urn};
pub use start_validation::*;
pub use tile_helpers::*;
pub use timing::{DungeonFieldEffect, SaveTemplateSource, TimingStatusTag};
pub use town_tables::*;
pub use town_tables_io::*;
pub use town_tables_io_movement::*;
pub use transport::{BoardVehicleCandidate, PendingVehicleAcquisition, TransportState};
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

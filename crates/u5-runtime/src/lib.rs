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
pub mod shops;
pub mod shrine_virtue;
pub mod signs_io;
pub mod story_io;
pub mod view_classes;
pub mod start_validation;
pub mod test_fixtures;
pub mod tile_helpers;
pub mod timing;
pub mod town_tables;
pub mod town_tables_io;
pub mod town_tables_io_movement;
pub mod transport;
pub mod traps;
pub mod u4_transfer;
pub mod wind;
pub mod world_tables;
pub mod world_tables_io;
pub mod world_tables_io_get_pickup;
pub mod world_tables_io_locations;

pub use active_object_io::*;
pub use animation::{ActiveObject, ActiveShipWind, AnimationClock, PhaseTick};
pub use chargen::*;
pub use clock::{GameClock, SKY_STRIP_CELL_COUNT, SkyStripMarker, sky_strip_marker_position};
pub use end_io::{EndNarrative, decode_end_window, load_end_narrative};
pub use endmsg_io::{EndgameMessages, load_endgame_messages, parse_endgame_messages};
pub use miscmsg_io::{MiscMessages, load_misc_messages, parse_misc_messages};
pub use question_io::{QuestionRecords, load_question_records, parse_question_records};
pub use signs_io::{
    SignRecord, decode_sign_payload, find_sign, load_sign_records, parse_sign_records,
};
pub use story_io::{StoryRecords, load_story_records, parse_story_records};
pub use view_classes::tile_view_class;
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
pub use npc_runtime::{DoorTracker, LocationMarkers, RuntimeNpc};
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
pub use shrine_virtue::ShrineVirtue;
pub use start_validation::*;
pub use tile_helpers::*;
pub use timing::{DungeonFieldEffect, SaveTemplateSource, TimingStatusTag};
pub use town_tables::*;
pub use town_tables_io::*;
pub use town_tables_io_movement::*;
pub use transport::{BoardVehicleCandidate, PendingVehicleAcquisition, TransportState};
pub use traps::*;
pub use u4_transfer::*;
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

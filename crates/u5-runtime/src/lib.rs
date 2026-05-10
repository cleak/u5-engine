//! Game runtime for the Ultima V clean-room implementation.
//!
//! This crate owns the simulation, parsers, and rules. It has no UI
//! dependencies. UI shells (`u5-tui`, `u5-bevy`) consume its public API.

pub mod active_object_io;
pub mod animation;
pub mod clock;
pub mod constants;
pub mod direction;
pub mod dungeon_tables;
pub mod dungeon_tables_io;
pub mod dungeon_tables_io_movement;
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
pub mod report;
pub mod save_load;
pub mod scene;
pub mod shrine_virtue;
pub mod start_validation;
pub mod test_fixtures;
pub mod tile_helpers;
pub mod timing;
pub mod town_tables;
pub mod town_tables_io;
pub mod town_tables_io_movement;
pub mod transport;
pub mod wind;
pub mod world_tables;
pub mod world_tables_io;
pub mod world_tables_io_get_pickup;
pub mod world_tables_io_locations;

pub use active_object_io::*;
pub use animation::{ActiveObject, ActiveShipWind, AnimationClock, PhaseTick};
pub use clock::GameClock;
pub use constants::*;
pub use direction::Direction;
pub use dungeon_tables::*;
pub use dungeon_tables_io::*;
pub use dungeon_tables_io_movement::*;
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
    Area, AvatarStats, MoonstoneGateSlot, PartyMember, Player, default_party,
    increase_capped_stat, party_member_unavailable_message, party_status_name,
};
pub use play_options::*;
pub use play_state_struct::{PlayState, WorldOverlayCache, WorldReturn};
pub use predicates::*;
pub use report::run_report;
pub use save_load::*;
pub use scene::{DungeonScene, Family, PlayTarget, Scene, WorldPlane};
pub use shrine_virtue::ShrineVirtue;
pub use start_validation::*;
pub use tile_helpers::*;
pub use timing::{DungeonFieldEffect, SaveTemplateSource, TimingStatusTag};
pub use town_tables::*;
pub use town_tables_io::*;
pub use town_tables_io_movement::*;
pub use transport::{BoardVehicleCandidate, PendingVehicleAcquisition, TransportState};
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

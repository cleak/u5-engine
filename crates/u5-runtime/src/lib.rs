//! Game runtime for the Ultima V clean-room implementation.
//!
//! This crate owns the simulation, parsers, and rules. It has no UI
//! dependencies. UI shells (`u5-tui`, `u5-bevy`) consume its public API.
//!
//! The lib body is split across `parts/part_NN.rs` files via `include!`
//! to satisfy the <1000-lines-per-file rule while preserving the original
//! flat namespace. Future work can carve these into proper modules.

pub mod animation;
pub mod clock;
pub mod constants;
pub mod direction;
pub mod dungeon_tables;
pub mod fonts_io;
pub mod graphics;
pub mod graphics_io;
pub mod inline_parsers;
pub mod lzw;
pub mod map_io;
pub mod misc_tables;
pub mod npc_runtime;
pub mod party;
pub mod play_options;
pub mod play_state_struct;
pub mod save_load;
pub mod scene;
pub mod shrine_virtue;
pub mod start_validation;
pub mod test_fixtures;
pub mod timing;
pub mod town_tables;
pub mod transport;
pub mod wind;
pub mod world_tables;

pub use animation::{ActiveObject, ActiveShipWind, AnimationClock, PhaseTick};
pub use clock::GameClock;
pub use constants::*;
pub use direction::Direction;
pub use dungeon_tables::*;
pub use fonts_io::*;
pub use graphics::*;
pub use graphics_io::*;
pub use inline_parsers::*;
pub use lzw::*;
pub use map_io::*;
pub use misc_tables::*;
pub use npc_runtime::{DoorTracker, LocationMarkers, RuntimeNpc};
pub use party::{
    Area, AvatarStats, MoonstoneGateSlot, PartyMember, Player, default_party,
    increase_capped_stat, party_member_unavailable_message, party_status_name,
};
pub use play_options::*;
pub use play_state_struct::{PlayState, WorldOverlayCache, WorldReturn};
pub use save_load::*;
pub use scene::{DungeonScene, Family, PlayTarget, Scene, WorldPlane};
pub use shrine_virtue::ShrineVirtue;
pub use start_validation::*;
pub use timing::{DungeonFieldEffect, SaveTemplateSource, TimingStatusTag};
pub use town_tables::*;
pub use transport::{BoardVehicleCandidate, PendingVehicleAcquisition, TransportState};
pub use wind::WindState;
pub use world_tables::*;

include!("parts/part_01.rs");
include!("parts/part_02.rs");
include!("parts/part_03.rs");
include!("parts/part_04.rs");
include!("parts/part_05.rs");
include!("parts/part_06.rs");
include!("parts/part_07.rs");
include!("parts/part_08.rs");
include!("parts/part_09.rs");
include!("parts/part_10.rs");
include!("parts/part_11.rs");
include!("parts/part_12.rs");
include!("parts/part_13.rs");
include!("parts/part_14.rs");
include!("parts/part_15.rs");
include!("parts/part_16.rs");

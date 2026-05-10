//! Game runtime for the Ultima V clean-room implementation.
//!
//! This crate owns the simulation, parsers, and rules. It has no UI
//! dependencies. UI shells (`u5-tui`, `u5-bevy`) consume its public API.
//!
//! The lib body is split across `parts/part_NN.rs` files via `include!`
//! to satisfy the <1000-lines-per-file rule while preserving the original
//! flat namespace. Future work can carve these into proper modules.

pub mod constants;
pub mod direction;
pub mod scene;
pub mod shrine_virtue;
pub mod test_fixtures;
pub mod wind;

pub use constants::*;
pub use direction::Direction;
pub use scene::{DungeonScene, Family, PlayTarget, Scene, WorldPlane};
pub use shrine_virtue::ShrineVirtue;
pub use wind::WindState;

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

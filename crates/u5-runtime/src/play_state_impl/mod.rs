//! Implementation methods for `PlayState`, split across chunks for the <1000-line rule.
//!
//! These were originally `parts/play_state_impl/chunk_NN.rs` and lived under an `include!` wrapper; they're now proper modules each wrapping their own `impl PlayState { ... }` block.

mod chunk_01;
pub(crate) mod chunk_02;
mod chunk_03;
mod chunk_04;
mod chunk_05;
mod chunk_06;
mod chunk_07;
mod chunk_08;
mod chunk_09;
mod chunk_10;
mod chunk_11;
mod chunk_12;

pub(crate) use chunk_09::surface_local_light_mask_index;
pub use chunk_09::{
    town_free_roaming_direction, town_free_roaming_facing_byte, town_free_roaming_object_eligible,
    town_free_roaming_pen_tile_blocks,
};

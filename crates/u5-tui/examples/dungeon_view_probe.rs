//! Report the first-person renderer's per-band choices.
//!
//! `dungeon-mode.md §6.4`/`§6.5`: the sweep runs a forward test per band
//! and paints the two side cells, choosing an image family from each
//! cell's high nibble. A paired capture of Deceit level 1 shows the
//! stock game drawing angled side walls at the nearest band where this
//! engine draws flat ones, so this prints what the engine decided:
//! per band, the forward outcome and the two side roles, plus the class
//! nibble each came from.
//!
//! Sanitized by construction: class nibbles and role names only, no map
//! dump.
//!
//! Usage: `cargo run -p u5-tui --example dungeon_view_probe -- <PROFILE_DIR>`

use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: <PROFILE_DIR>");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("SAVED.GAM must seed the scene");
    let mut state = PlayState::load_scene(dir, options).expect("scene must load");
    let turns: usize = std::env::args()
        .nth(2)
        .and_then(|value| value.parse().ok())
        .unwrap_or(0);
    for _ in 0..turns {
        state.turn_dungeon(true);
    }
    let Area::Dungeon { scene, level } = state.area else {
        println!("not in a dungeon");
        return;
    };
    println!(
        "{} level {level} at ({}, {}) facing {:?}",
        scene.key(),
        state.player.x,
        state.player.y,
        state.player.facing
    );
    let (fdx, fdy) = state.player.facing.delta();
    let left = state
        .player
        .facing
        .turn_left_cardinal()
        .expect("cardinal facing");
    let right = state
        .player
        .facing
        .turn_right_cardinal()
        .expect("cardinal facing");
    let (ldx, ldy) = left.delta();
    let (rdx, rdy) = right.delta();
    for band in 0..DUNGEON_BANDS {
        let step = band as isize;
        let ahead = state.dungeon_renderer_offset_cell(level, fdx * step, fdy * step);
        let outcome = dungeon_forward_outcome(ahead, band);
        let left_cell =
            state.dungeon_renderer_offset_cell(level, fdx * step + ldx, fdy * step + ldy);
        let right_cell =
            state.dungeon_renderer_offset_cell(level, fdx * step + rdx, fdy * step + rdy);
        println!(
            "  band {band}: ahead class {:x}? see_through {} blocker {:?} point_blank {} | left class {:x}? role {:?} | right class {:x}? role {:?}",
            ahead >> 4,
            outcome.see_through,
            outcome.blocker,
            outcome.point_blank,
            left_cell >> 4,
            dungeon_side_role(left_cell),
            right_cell >> 4,
            dungeon_side_role(right_cell)
        );
    }
}

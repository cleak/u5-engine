//! Sanitized census of the endgame tableau surface after a walked-in
//! absorption.
//!
//! `cleak/u5-engine#7`: the original's chamber floor is a different
//! colour from the arena the party walked out of, and this engine's is
//! not. This reports which tile bytes the endgame renderer would draw -
//! counts by byte, no grid dump - so the two candidate causes can be
//! told apart: a tableau that was never installed, or one installed and
//! drawn with the wrong graphics.
//!
//! Usage: `cargo run -p u5-tui --example endgame_tableau_probe -- <PROFILE_DIR>`

use std::collections::BTreeMap;
use std::path::Path;
use u5_runtime::*;

fn census(label: &str, cells: impl Iterator<Item = u8>) {
    let mut counts: BTreeMap<u8, usize> = BTreeMap::new();
    for tile in cells {
        *counts.entry(tile).or_default() += 1;
    }
    let mut summary: Vec<(u8, usize)> = counts.into_iter().collect();
    summary.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    let shown: Vec<String> = summary
        .iter()
        .take(6)
        .map(|(tile, count)| format!("0x{tile:02x}={count}"))
        .collect();
    println!("{label}: {} distinct, {}", summary.len(), shown.join(" "));
}

fn main() {
    let dir = std::env::args().nth(1).expect("usage: <PROFILE_DIR>");
    let dir = Path::new(&dir);
    let map = require_miscmaps_cutscene_map(dir, ENDGAME_TABLEAU_CUTSCENE_MAP_RECORD)
        .expect("MISCMAPS.DAT must carry the tableau record");
    census(
        "miscmaps record",
        (0..ENDGAME_TABLEAU_HEIGHT)
            .flat_map(|y| (0..ENDGAME_TABLEAU_WIDTH).map(move |x| (x, y)))
            .filter_map(|(x, y)| map.tile(x, y)),
    );

    let options = PlayOptions::default();
    let mut state = PlayState::load_scene(dir, options).expect("scene must load");
    state
        .enter_endgame_from_game_dir(Some(dir))
        .expect("direct endgame entry");
    census(
        "grid after direct entry",
        (0..ENDGAME_TABLEAU_HEIGHT)
            .flat_map(|y| (0..ENDGAME_TABLEAU_WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| state.grid[y * TOWN_GRID_SIDE + x]),
    );
}

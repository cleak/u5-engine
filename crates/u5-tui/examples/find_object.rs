//! Print the live active objects of a saved scene, tile id first.
//!
//! `find_tile` answers "which map cells hold this terrain byte"; chests,
//! dropped items and vehicles are not map cells but active-object
//! records, so locating one for a QA seed needs this instead. Optional
//! second argument filters to one tile id (decimal or `0x` hex).

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: find_object <PROFILE_DIR> [TILE_ID]");
    let dir = Path::new(&dir);
    let wanted = args.next().map(|text| {
        let text = text.trim().to_string();
        match text.strip_prefix("0x") {
            Some(hex) => u8::from_str_radix(hex, 16).expect("tile id"),
            None => text.parse::<u8>().expect("tile id"),
        }
    });
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");
    println!(
        "{:?}: party at ({}, {})",
        state.area, state.player.x, state.player.y
    );
    for object in state
        .active_objects
        .iter()
        .filter(|object| !object.is_empty())
        .filter(|object| wanted.is_none_or(|tile| object.tile == tile))
    {
        println!(
            "tile {:#04x} type {:#04x} at ({}, {}, {}) phase {}",
            object.tile, object.type_byte, object.x, object.y, object.z, object.phase
        );
    }
}

//! `StaticMaps` over the player's own game files (design 4.3.4).
//!
//! Counts only: no map byte, tile or cell is printed or committed.
//! Skipped unless `U5_MOCKUP_PROFILE` names a validated game-data copy
//! (`game-dev-u5 test-assets` or `prepare-profile`).

use std::path::PathBuf;

use u5_automap::{DungeonLook, StaticMaps, perceived_dungeon_cell};
use u5_runtime::WorldPlane;

#[test]
fn real_static_maps_load_with_their_lengths() {
    let Some(profile) = std::env::var_os("U5_MOCKUP_PROFILE").map(PathBuf::from) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE");
        return;
    };
    let mut maps = StaticMaps::new(&profile);
    for plane in [WorldPlane::Britannia, WorldPlane::Underworld] {
        assert_eq!(maps.world_map(plane).expect("world map").len(), 256 * 256);
    }
    let mut floors = 0;
    for scene in 1..=32u8 {
        if maps.floor(scene, 0).is_ok() {
            floors += 1;
        }
    }
    assert_eq!(floors, 32, "every location has a ground floor");
    let mut walls = 0usize;
    for record in 0..8u8 {
        for level in 0..8u8 {
            let cells = maps.dungeon_level(record, level).expect("dungeon level");
            walls += cells
                .iter()
                .filter(|&&raw| perceived_dungeon_cell(raw).look == DungeonLook::Wall)
                .count();
        }
    }
    assert!(walls > 0);
    // 2 planes + 32 floors + 8 records, each loaded once.
    assert_eq!(maps.cached_len(), 42);
    eprintln!("static maps: {floors} ground floors; {walls} perceived wall cells");
}

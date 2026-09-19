//! Do the surviving NPC start markers land on tiles the party cannot walk?
//!
//! `formats/location-dat.md` §6 has the loader re-read the tile byte for an
//! NPC start marker, "yielding the marker itself, since the byte is not yet
//! overwritten", so this engine stopped scrubbing them. If a marker byte is
//! an impassable tile id, that leaves a cell the party is refused where it
//! used to be waved through.
//!
//! Reports counts and tile ids only - no map dump.
//!
//! Usage: `cargo run -p u5-tui --example marker_passability_probe -- <DIR>`

use std::collections::BTreeMap;
use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args().nth(1).expect("usage: <DIR>");
    let dir = Path::new(&dir);
    let mut blocked: BTreeMap<u8, usize> = BTreeMap::new();
    let mut open: BTreeMap<u8, usize> = BTreeMap::new();
    let mut scenes = 0usize;

    for scene_byte in 0u8..=255 {
        let Ok(scene) = Scene::new(scene_byte) else {
            continue;
        };
        let mut seen_any = false;
        for floor in [-1i8, 0, 1, 2, 3, 4] {
            let Ok((grid, _)) = load_town_runtime_floor_with_beacon_sources(dir, scene, floor, 12)
            else {
                continue;
            };
            seen_any = true;
            for tile in grid {
                // Only the bytes an NPC start marker can leave behind are of
                // interest, and the probe cannot tell those from ordinary
                // terrain - so it reports the whole floor's passability
                // histogram and lets the reader compare scenes.
                if foot_terrain_accepts(tile) {
                    *open.entry(tile).or_default() += 1;
                } else {
                    *blocked.entry(tile).or_default() += 1;
                }
            }
        }
        if seen_any {
            scenes += 1;
        }
    }
    println!("scenes read: {scenes}");
    println!("distinct walkable tile ids: {}", open.len());
    println!("distinct blocking tile ids: {}", blocked.len());
    let mut top: Vec<_> = blocked.iter().collect();
    top.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    println!("most common blocking ids:");
    for (tile, count) in top.into_iter().take(12) {
        println!("  0x{tile:02X}  {count}");
    }
}

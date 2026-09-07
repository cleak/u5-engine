//! Report every town-family cell of the shipped maps holding a given tile
//! byte, across every scene and floor.
//!
//! `find_tile` answers the same question for the one scene a profile's save
//! is standing in. Building a seed for a feature that appears *somewhere* -
//! the crystal-sphere Look tile, say - needs the wider sweep first, so this
//! walks each scene's published floor range and prints the hits.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: find_tile_everywhere <PROFILE_DIR> <TILE_HEX>");
    let wanted = args
        .next()
        .expect("usage: find_tile_everywhere <PROFILE_DIR> <TILE_HEX>");
    let wanted = u8::from_str_radix(wanted.trim_start_matches("0x"), 16).expect("tile byte");
    let dir = Path::new(&dir);
    println!("looking for 0x{wanted:02x}");
    let mut found = 0;
    for scene_byte in SCENE_TOWN_FAMILY_FIRST..=SCENE_TOWN_FAMILY_LAST {
        let Ok(scene) = Scene::new(scene_byte) else {
            continue;
        };
        let (lowest, highest) = location_page_run(scene).floor_range();
        for floor in lowest..=highest {
            let Ok(grid) = load_floor(dir, scene, floor) else {
                continue;
            };
            for y in 0..TOWN_GRID_SIDE {
                for x in 0..TOWN_GRID_SIDE {
                    if grid[y * TOWN_GRID_SIDE + x] == wanted {
                        println!("  {} floor {floor} ({x:>2}, {y:>2})", scene.key());
                        found += 1;
                    }
                }
            }
        }
    }
    println!("{found} cell(s)");
}

//! Report every Britannia or Underworld cell holding a given tile byte.
//!
//! `find_tile_everywhere` sweeps the town-family scenes; the overworld planes
//! need their own scan, and a landmark whose published coordinate does not
//! match the shipped map is exactly the case that needs one.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: find_world_tile <PROFILE_DIR> <TILE_HEX>");
    let wanted = args
        .next()
        .expect("usage: find_world_tile <PROFILE_DIR> <TILE_HEX>");
    let wanted = u8::from_str_radix(wanted.trim_start_matches("0x"), 16).expect("tile byte");
    let dir = Path::new(&dir);
    for plane in [WorldPlane::Britannia, WorldPlane::Underworld] {
        let Ok(grid) = load_world_map(dir, plane) else {
            continue;
        };
        let mut found = 0;
        for y in 0..WORLD_SIDE {
            for x in 0..WORLD_SIDE {
                if grid[world_cell_index(x, y)] == wanted {
                    println!("{plane:?} ({x}, {y})");
                    found += 1;
                }
            }
        }
        println!("{plane:?}: {found} cell(s)");
    }
}

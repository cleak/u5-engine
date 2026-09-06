//! Report every cell of the loaded scene holding a given tile byte.
//!
//! Crafting a seed for a scenario means putting the party on a specific
//! cell - a trapdoor, a room trigger, a counter - and the map is not
//! something a clean workspace may transcribe. This asks the loaded
//! grid instead: give it a profile and a tile byte, and it prints the
//! coordinates, which is enough to build the seed with `--at`.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .expect("usage: find_tile <PROFILE_DIR> <TILE_HEX>");
    let wanted = args
        .next()
        .expect("usage: find_tile <PROFILE_DIR> <TILE_HEX>");
    let wanted = u8::from_str_radix(wanted.trim_start_matches("0x"), 16).expect("tile byte");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");
    println!("scene {:?}, looking for 0x{wanted:02x}", state.area);
    let mut found = 0;
    // A dungeon level is eight cells square and is read through its own
    // accessor; the town/world grid is thirty-two and is a flat buffer.
    // Scanning a dungeon as though it were the larger one reports
    // coordinates that no dungeon command will accept - `--at 13,15` is
    // refused with "dungeon coordinate must be inside 0..7".
    if let Area::Dungeon { level, .. } = state.area {
        for y in 0..DUNGEON_SIDE {
            for x in 0..DUNGEON_SIDE {
                if state.dungeon_cell(level, x, y) == wanted {
                    println!("  ({x}, {y})");
                    found += 1;
                }
            }
        }
    } else {
        for y in 0..TOWN_GRID_SIDE {
            for x in 0..TOWN_GRID_SIDE {
                if state.grid[y * TOWN_GRID_SIDE + x] == wanted {
                    println!("  ({x:>2}, {y:>2})");
                    found += 1;
                }
            }
        }
    }
    println!("{found} cell(s)");
}

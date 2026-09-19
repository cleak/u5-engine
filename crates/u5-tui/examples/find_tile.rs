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
        .expect("usage: find_tile <PROFILE_DIR> <TILE_HEX|all>");
    // `all` asks the level what it holds rather than whether it holds one
    // thing. Hunting a tile across eight dungeons and eight levels
    // otherwise costs 256 questions a level, which is how a scan for the
    // cure fountain came back empty everywhere and meant nothing - the
    // control byte was absent too.
    let histogram = wanted == "all";
    // `at=<x>,<y>` is the inverse question: not "where is this tile" but
    // "what is on this cell", which is what checking a candidate seed
    // coordinate needs.
    let at = wanted.strip_prefix("at=").map(|pair| {
        let (x, y) = pair.split_once(',').expect("at= takes <x>,<y>");
        (
            x.trim().parse::<usize>().expect("x is a number"),
            y.trim().parse::<usize>().expect("y is a number"),
        )
    });
    let wanted = if histogram || at.is_some() {
        0
    } else {
        u8::from_str_radix(wanted.trim_start_matches("0x"), 16).expect("tile byte")
    };
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");
    if histogram || at.is_some() {
        println!("scene {:?}", state.area);
    } else {
        println!("scene {:?}, looking for 0x{wanted:02x}", state.area);
    }
    if let Some((x, y)) = at {
        let side = match state.area {
            Area::World { .. } => WORLD_SIDE,
            Area::Dungeon { .. } => DUNGEON_SIDE,
            _ => TOWN_GRID_SIDE,
        };
        let tile = match state.area {
            Area::Dungeon { level, .. } => state.dungeon_cell(level, x, y),
            _ => state.grid[y * side + x],
        };
        println!("  ({x}, {y})  0x{tile:02x}");
        return;
    }
    let mut found = 0;
    let mut counts = std::collections::BTreeMap::<u8, usize>::new();
    // A dungeon level is eight cells square and is read through its own
    // accessor; the town/world grid is thirty-two and is a flat buffer.
    // Scanning a dungeon as though it were the larger one reports
    // coordinates that no dungeon command will accept - `--at 13,15` is
    // refused with "dungeon coordinate must be inside 0..7".
    if let Area::Dungeon { level, .. } = state.area {
        for y in 0..DUNGEON_SIDE {
            for x in 0..DUNGEON_SIDE {
                let tile = state.dungeon_cell(level, x, y);
                if histogram {
                    *counts.entry(tile).or_default() += 1;
                } else if tile == wanted {
                    println!("  ({x}, {y})");
                    found += 1;
                }
            }
        }
    } else {
        // The world map is `WORLD_SIDE` square, not `TOWN_GRID_SIDE`.
        // Scanning it as a town grid reads the top-left thirty-two cells
        // of each of the first thirty-two rows and reports "0 cell(s)"
        // for everything else in the world - which is what a waterfall
        // hunt at `(54, 136)` came back with.
        let side = match state.area {
            Area::World { .. } => WORLD_SIDE,
            _ => TOWN_GRID_SIDE,
        };
        for y in 0..side {
            for x in 0..side {
                let tile = state.grid[y * side + x];
                if histogram {
                    *counts.entry(tile).or_default() += 1;
                } else if tile == wanted {
                    println!("  ({x:>2}, {y:>2})");
                    found += 1;
                }
            }
        }
    }
    if histogram {
        for (tile, count) in &counts {
            println!("  0x{tile:02x}  {count}");
        }
        println!("{} distinct tile id(s)", counts.len());
    } else {
        println!("{found} cell(s)");
    }
}

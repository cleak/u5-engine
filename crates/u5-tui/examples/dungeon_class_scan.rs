//! Where in `DUNGEON.DAT` does a given cell class live?
//!
//! `find_tile` answers that for the one level a profile is standing on.
//! Several unreached lines need a *different* level of a *different*
//! dungeon - the fountain drink results of `dungeon-mode.md §8`
//! (`Cured!`, `Healed!`, `Poisoned!`, `Bad taste.`) are one class byte
//! each, and nothing says which of the sixty-four levels carries one.
//! Asking a level at a time is sixty-four runs; this is one.
//!
//! Clean-room safe on the same terms as `find_tile`: the class comes
//! from the command line and the output is an aggregate location
//! report, not a map dump.
//!
//! Usage:
//!     dungeon_class_scan <GAME_DIR> <class-nibble hex, e.g. 5>
//!     dungeon_class_scan <GAME_DIR> byte=<hex, e.g. 52>

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: dungeon_class_scan <GAME_DIR> <class>");
    let dir = Path::new(&dir);
    let wanted = args.next().expect("usage: dungeon_class_scan <GAME_DIR> <class>");
    let exact = wanted.strip_prefix("byte=").map(|value| {
        u8::from_str_radix(value, 16).expect("byte= takes two hex digits")
    });
    let class = match exact {
        Some(_) => None,
        None => Some(u8::from_str_radix(&wanted, 16).expect("class is one hex digit")),
    };

    let mut hits = 0usize;
    for record in 0..DUNGEON_DAT_RECORD_COUNT as u8 {
        let scene = DungeonScene::from_record(record).expect("record index is in range");
        let grid = load_dungeon_record(dir, scene).expect("DUNGEON.DAT must hold the record");
        for level in 0..DUNGEON_LEVELS_PER_RECORD as u8 {
            for y in 0..DUNGEON_SIDE {
                for x in 0..DUNGEON_SIDE {
                    let tile = grid[dungeon_cell_index(level, x, y)];
                    let matched = match (exact, class) {
                        (Some(byte), _) => tile == byte,
                        (None, Some(nibble)) => tile >> 4 == nibble,
                        _ => false,
                    };
                    if matched {
                        hits += 1;
                        println!(
                            "scene {} record {record} level {level} ({x}, {y})  0x{tile:02x}",
                            scene.byte
                        );
                    }
                }
            }
        }
    }
    println!("{hits} cell(s)");
}

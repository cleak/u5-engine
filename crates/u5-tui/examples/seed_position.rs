//! Put a profile's party somewhere, so a scenario can reach a cell no
//! seed stands on.
//!
//! `seed_inventory` stocks a save with items and companions and is what
//! makes the item and party flows measurable. The same gap exists for
//! *places*: `qa/coverage/unreached-lines.md` has ten
//! `hidden-treasures.md` finds - `a sack of gold!`, `a moldy corpse!`,
//! `a ring of keys!` and the rest - whose records all sit at one
//! Underworld cell, and no seed in the suite stands anywhere near it.
//! Walking there is a long scripted route; setting the coordinate is a
//! line.
//!
//! Clean-room safe on the same terms as `seed_inventory`: the plane and
//! the coordinate come from the command line, not from any asset.
//!
//! Usage:
//!     seed_position <PROFILE_DIR> [plane=britannia|underworld]
//!                                 [dungeon=<scene byte> level=<N>]
//!                                 [town=<scene byte> floor=<N>]
//!                                 [x=<N>] [y=<N>]
//!
//! `plane`, `dungeon` and `town` are alternatives; the last one given
//! wins. The dungeon form is what the fountain results want - `Cured!`,
//! `Healed!`, `Poisoned!`, `Bad taste.` all need a party standing on a
//! specific dungeon cell, and none of them has ever been captured.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: seed_position <PROFILE_DIR> ...");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let mut state = PlayState::load_scene(dir, options).expect("scene must load");
    for arg in args {
        let (what, value) = arg
            .split_once('=')
            .expect("each argument is <what>=<value>");
        match what {
            "plane" => {
                let plane = match value {
                    "britannia" => WorldPlane::Britannia,
                    "underworld" => WorldPlane::Underworld,
                    other => panic!("plane is britannia or underworld, got `{other}`"),
                };
                state.area = Area::World { plane };
            }
            "dungeon" => {
                let byte: u8 = value.parse().expect("dungeon is a scene byte");
                let scene = DungeonScene::new(byte).expect("dungeon scene byte must resolve");
                let level = match state.area {
                    Area::Dungeon { level, .. } => level,
                    _ => 0,
                };
                state.area = Area::Dungeon { scene, level };
            }
            "town" => {
                let byte: u8 = value.parse().expect("town is a scene byte");
                let scene = Scene::new(byte).expect("town scene byte must resolve");
                let floor = match state.area {
                    Area::Town { floor, .. } => floor,
                    _ => 0,
                };
                state.area = Area::Town { scene, floor };
            }
            "level" => {
                let level: u8 = value.parse().expect("level is a number");
                match state.area {
                    Area::Dungeon { scene, .. } => state.area = Area::Dungeon { scene, level },
                    _ => panic!("`level` needs `dungeon` first"),
                }
            }
            "floor" => {
                let floor: i8 = value.parse().expect("floor is a number");
                match state.area {
                    Area::Town { scene, .. } => state.area = Area::Town { scene, floor },
                    _ => panic!("`floor` needs `town` first"),
                }
            }
            "x" => state.player.x = value.parse().expect("x is a number"),
            "y" => state.player.y = value.parse().expect("y is a number"),
            other => panic!("unknown field `{other}`"),
        }
    }
    // A saved game carries the live dungeon working buffer, and
    // `load_dungeon_scene` prefers that buffer over the `DUNGEON.DAT`
    // record for the scene byte. Setting `state.area` alone therefore
    // moves the *label* into a dungeon while the reload keeps whichever
    // map the profile already held - two different dungeon scene bytes
    // read back byte-identical grids. Refresh the grid alongside the
    // area so the seed is the dungeon it claims to be.
    if let Area::Dungeon { scene, .. } = state.area {
        let mut grid = load_dungeon_record(dir, scene).expect("DUNGEON.DAT must hold the record");
        apply_dungeon_room_clear_bitmap(&mut grid, scene, &state.dungeon_room_clear_bitmap);
        state.grid = grid;
    }
    state.sync_player_object();
    state.write_save_files(dir).expect("save must write");
    println!(
        "placed {:?} at ({}, {}) in {}",
        state.area,
        state.player.x,
        state.player.y,
        dir.display()
    );
}

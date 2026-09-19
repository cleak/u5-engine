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
//!     seed_position <PROFILE_DIR> [plane=britannia|underworld] [x=<N>] [y=<N>]

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
            "x" => state.player.x = value.parse().expect("x is a number"),
            "y" => state.player.y = value.parse().expect("y is a number"),
            other => panic!("unknown field `{other}`"),
        }
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

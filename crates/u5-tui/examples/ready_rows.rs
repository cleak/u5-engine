//! Print the R-Ready picker's rows, in order, for a profile's save.
//!
//! The companion of `use_rows`: a scenario that has to reach a particular
//! readyable item needs its row index, and counting keypresses blind is how
//! several measurement runs landed on the wrong one.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: ready_rows <PROFILE_DIR>");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");
    for (index, item) in state.ready_picker_items(0).into_iter().enumerate() {
        println!("{index:>2}  0x{item:02x}  {}", equipment_name(item));
    }
}

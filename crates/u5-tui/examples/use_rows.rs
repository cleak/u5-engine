//! Print the U-Use picker's rows, in order, for a profile's save.
//!
//! Addressing a picker row from a paired scenario means counting keypresses,
//! and counting blind is how three measurement runs in a row landed on the
//! wrong item. The engine reads the same save the harness seeds DOSBox with,
//! so its row order is the count the scenario needs.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: use_rows <PROFILE_DIR>");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");
    for (index, row) in state.use_item_picker_rows().iter().enumerate() {
        match row.quantity {
            Some(count) => println!("{index:>2}  {count:>2}  {}", row.label),
            None => println!("{index:>2}      {}", row.label),
        }
    }
}

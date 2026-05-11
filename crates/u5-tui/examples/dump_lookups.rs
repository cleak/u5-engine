//! Dump every tile id 0..255 with its LOOK2.DAT description and current
//! runtime walkability classification. Lets us spot mismatches between the
//! game's canonical labeling and our spec-derived class ranges.

use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from(r"C:\Games\U5-Clean"));
    let dir = Path::new(&dir);
    let look = load_look_table(dir).unwrap();
    println!("id   hex  walk water mtn  sight  description");
    println!("---  ---- ---- ----- ---- ----- ------------");
    for tid in 0..=255u8 {
        let desc = look.description(tid as usize).unwrap_or("?");
        let walk = is_tile_walkable(tid, None);
        let water = is_water_tile(tid);
        let mtn = is_mountain_tile(tid);
        let sight = surface_tile_blocks_sight(tid);
        println!(
            "{tid:3}  0x{tid:02x}  {:4} {:5} {:4} {:5}  {desc}",
            if walk { "yes" } else { "no" },
            if water { "yes" } else { "no" },
            if mtn { "yes" } else { "no" },
            if sight { "yes" } else { "no" },
        );
    }
}

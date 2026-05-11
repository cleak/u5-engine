//! Find all positions on Britannia where each landmark tile id appears.

use std::collections::BTreeMap;
use std::path::Path;
use u5_runtime::*;

fn main() {
    let dir = Path::new(r"C:\Games\U5-Clean");
    let grid = load_world_map(dir, WorldPlane::Britannia).unwrap();
    let look = load_look_table(dir).unwrap();

    let mut by_tile: BTreeMap<u8, Vec<(usize, usize)>> = BTreeMap::new();
    for y in 0..WORLD_SIDE {
        for x in 0..WORLD_SIDE {
            let tile = grid[world_cell_index(x, y)];
            // Skip ordinary terrain.
            if matches!(tile, 0..=15 | 0x30..=0x37) {
                continue;
            }
            by_tile.entry(tile).or_default().push((x, y));
        }
    }

    for (tile, positions) in &by_tile {
        let desc = look.description(*tile as usize).unwrap_or("?");
        if positions.len() > 30 {
            println!("tile 0x{tile:02x} {desc:>25}  {} occurrences", positions.len());
        } else {
            print!("tile 0x{tile:02x} {desc:>25}  ");
            for (x, y) in positions.iter().take(20) {
                print!("({x},{y}) ");
            }
            if positions.len() > 20 {
                print!("... +{} more", positions.len() - 20);
            }
            println!();
        }
    }
}

//! Dump the world tile id and walkability for a 5x5 block around a position.
//! Usage: cargo run --release --example probe_tiles -- <game_dir> <x> <y>

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .unwrap_or_else(|| String::from(r"C:\Games\U5-Clean"));
    let x0: usize = args
        .next()
        .unwrap_or_else(|| String::from("245"))
        .parse()
        .unwrap();
    let y0: usize = args
        .next()
        .unwrap_or_else(|| String::from("6"))
        .parse()
        .unwrap();

    let dir = Path::new(&dir);
    let grid = load_world_map(dir, WorldPlane::Britannia).unwrap();
    let passability = load_tile_passability(dir).unwrap();
    let damage = load_world_damage_tile_entries(dir)
        .unwrap()
        .unwrap_or_default();
    let look = load_look_table(dir).unwrap();

    println!(
        "Tiles around ({x0}, {y0}) on Britannia. Each cell shows tile id (hex), \
         is_tile_walkable_for_transport(Foot) result, and damage-tile classification."
    );
    for y in y0.saturating_sub(5)..=y0 + 5 {
        for x in x0.saturating_sub(5)..=x0 + 5 {
            let tile = grid[world_cell_index(x, y)];
            let walk =
                is_tile_walkable_for_transport(tile, passability.as_ref(), TransportState::Foot);
            let base = is_base_tile_passable(tile, passability.as_ref());
            let dmg = world_damage_tile_entry_at(&damage, WorldPlane::Britannia, x, y, tile);
            let here = if (x, y) == (x0, y0) { "*" } else { " " };
            let desc = look.description(tile as usize).unwrap_or("?");
            println!(
                "  {here}({x:3},{y:3}) tile=0x{tile:02x} {desc:>20} base_pass={base:5} \
                 walk_foot={walk:5} dmg={dmg:?}"
            );
        }
        println!();
    }
}

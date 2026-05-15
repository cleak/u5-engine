//! Dump the tile id at each cell of the rendered viewport, including any
//! active-object overlay. Useful for correlating visible black areas in a
//! screenshot to specific tile ids.
//! Usage: cargo run --release --example probe_viewport -- <game_dir>

use std::path::Path;

use u5_runtime::*;
use u5_tui::run_save_frame;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args
        .next()
        .unwrap_or_else(|| String::from(r"C:\Games\U5-Clean"));
    let dir = Path::new(&dir);

    // Match the --scene BRITANNIA default-start case.
    let mut options = PlayOptions::default();
    options.target = PlayTarget::World(WorldPlane::Britannia);
    let state = PlayState::load_scene(dir, options).unwrap();
    let radius: isize = 5;
    let px = state.player.x as isize;
    let py = state.player.y as isize;
    let plane = match state.area {
        Area::World { plane } => plane,
        _ => panic!("expected world"),
    };
    let grid = load_world_map(dir, plane).unwrap();

    println!(
        "Viewport at player ({}, {}), radius {radius}. Each row is the static map tile id.\n",
        state.player.x, state.player.y
    );
    for dy in -radius..=radius {
        let y = ((py + dy).rem_euclid(WORLD_SIDE as isize)) as usize;
        let row_marker = if dy == 0 { '*' } else { ' ' };
        print!("{row_marker} y={y:3}: ");
        for dx in -radius..=radius {
            let x = ((px + dx).rem_euclid(WORLD_SIDE as isize)) as usize;
            let tile = grid[world_cell_index(x, y)];
            let here = if dy == 0 && dx == 0 { 'P' } else { ' ' };
            print!("{here}{tile:02x} ");
        }
        println!();
    }

    println!(
        "\nActive objects on the current plane (z=={}):",
        plane.save_floor()
    );
    for (i, obj) in state.active_objects.iter().enumerate() {
        if obj.tile == 0 || (obj.x == 0 && obj.y == 0 && obj.tile == 0) {
            continue;
        }
        let dx = (obj.x as isize - px).rem_euclid(WORLD_SIDE as isize);
        let dy = (obj.y as isize - py).rem_euclid(WORLD_SIDE as isize);
        if dx <= radius || dx >= WORLD_SIDE as isize - radius {
            if dy <= radius || dy >= WORLD_SIDE as isize - radius {
                println!(
                    "  slot {i}: type=0x{:02x} tile=0x{:02x} at ({}, {}, z={})",
                    obj.type_byte, obj.tile, obj.x, obj.y, obj.z
                );
            }
        }
    }

    // Also render the viewport so we can compare side-by-side.
    let out = Path::new("screenshots/14-probe-viewport.png");
    run_save_frame(
        dir,
        PlayOptions::default(),
        TileGraphicsDepth::Ega16,
        None,
        out,
    )
    .unwrap();
}

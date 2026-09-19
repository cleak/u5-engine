//! Print the tiles around a cell on every floor of a scene, so a capture of
//! the original can be matched against the floor it was taken on.
//!
//! Usage: scene_floor_probe <PROFILE_DIR> <SCENE_BYTE> <X> <Y>

use std::path::Path;
use u5_runtime::*;

fn main() {
    let mut args = std::env::args().skip(1);
    let dir = args.next().expect("usage: scene_floor_probe <DIR> <SCENE> <X> <Y>");
    let scene_byte: u8 = args.next().expect("scene byte").parse().unwrap();
    let cx: isize = args.next().expect("x").parse().unwrap();
    let cy: isize = args.next().expect("y").parse().unwrap();
    let dir = Path::new(&dir);
    let scene = Scene::new(scene_byte).expect("scene byte must resolve");
    println!("scene {scene_byte} = {} ({:?})", scene.key(), scene.family);
    for floor in [-1i8, 0, 1, 2, 3, 4] {
        let loaded = load_town_runtime_floor_with_beacon_sources(dir, scene, floor, 12);
        let Ok((grid, _)) = loaded else {
            continue;
        };
        println!("\nfloor {floor}, cells around ({cx}, {cy}):");
        for dy in -3..=3 {
            let mut row = String::new();
            for dx in -5..=5 {
                let (x, y) = (cx + dx, cy + dy);
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    row.push_str(".. ");
                    continue;
                }
                row.push_str(&format!("{:02x} ", grid[y as usize * 32 + x as usize]));
            }
            println!("  {row}");
        }
    }
}

//! Report the visibility gate and the carve's painted cell count for a saved
//! profile, so a viewport difference can be attributed to the threshold or to
//! the line-of-sight carve.
//!
//! `visibility.md` §4 gives the cells inside the distance gate for each
//! threshold, and §3 says opacity "governs whether the carve *continues past*
//! a cell, never whether that cell itself is seen". A painted count below the
//! gate's count is the carve doing its job; a painted count equal to it means
//! nothing is blocking.
//!
//! Usage: visibility_probe <PROFILE_DIR>

use std::path::Path;
use u5_runtime::*;

fn gate_cells(threshold: u32, radius: isize) -> usize {
    let mut count = 0;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let d2 = (dx * dx + dy * dy) as u32;
            if d2 <= threshold {
                count += 1;
            }
        }
    }
    count
}

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: visibility_probe <PROFILE_DIR>");
    let dir = Path::new(&dir);
    let options = load_play_options_from_save(dir).expect("profile must hold a save");
    let state = PlayState::load_scene(dir, options).expect("scene must load");
    let radius: isize = 5;
    let px = state.player.x as isize;
    let py = state.player.y as isize;
    let threshold = state.surface_visibility_light_threshold();
    let pitch_dark = state.surface_visibility_pitch_dark();
    let town = matches!(state.area, Area::Town { .. });
    let mut painted = 0;
    for dy in -radius..=radius {
        for dx in -radius..=radius {
            let (x, y) = (px + dx, py + dy);
            let visible = if town {
                state.town_cell_visible_with_light_threshold(px, py, x, y, radius as usize, threshold)
            } else {
                state.world_cell_visible_with_light_threshold(px, py, x, y, radius as usize, threshold)
            };
            if visible {
                painted += 1;
            }
        }
    }
    println!("area           {:?}", state.area);
    println!("party          ({}, {})", state.player.x, state.player.y);
    println!("ambient_light  {}", state.ambient_light);
    println!("threshold      {threshold}");
    println!("pitch dark     {pitch_dark}");
    println!("gate cells     {} of 121", gate_cells(threshold, radius));
    println!("painted cells  {painted} of 121");
    println!();
    println!("threshold sweep: painted cells at each published gate value");
    for probe in [2u32, 5, 10, 13, 18, 20, 24, 34, 49, 50] {
        let mut lit = 0;
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let (x, y) = (px + dx, py + dy);
                let visible = if town {
                    state.town_cell_visible_with_light_threshold(px, py, x, y, radius as usize, probe)
                } else {
                    state.world_cell_visible_with_light_threshold(px, py, x, y, radius as usize, probe)
                };
                if visible {
                    lit += 1;
                }
            }
        }
        println!("  L={probe:<3} gate={:<4} painted={lit}", gate_cells(probe, radius));
    }
    println!();
    for dy in -radius..=radius {
        let mut row = String::new();
        for dx in -radius..=radius {
            let (x, y) = (px + dx, py + dy);
            let visible = if town {
                state.town_cell_visible_with_light_threshold(px, py, x, y, radius as usize, threshold)
            } else {
                state.world_cell_visible_with_light_threshold(px, py, x, y, radius as usize, threshold)
            };
            row.push(if visible { '#' } else { '.' });
        }
        let mut tiles = String::new();
        for dx in -radius..=radius {
            let (x, y) = (px + dx, py + dy);
            let tile = if (0..32).contains(&x) && (0..32).contains(&y) {
                state.grid[(y as usize) * 32 + (x as usize)]
            } else {
                0
            };
            tiles.push_str(&format!("{tile:02x} "));
        }
        println!("{row}   {tiles}");
    }
}

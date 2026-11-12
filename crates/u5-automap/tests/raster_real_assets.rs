//! The tile colour table over the player's own TILES.16 (9.3.4, M3-09).
//!
//! Counts only: no tile, pixel or colour of the game is printed or
//! committed. Skipped unless `U5_MOCKUP_PROFILE` names a validated
//! game-data copy (`game-dev-u5 test-assets` or `prepare-profile`).
//! With `U5_RASTER_DUMP` set to a directory, the 4 px reductions at 100%
//! and 70% are written there as review-only PPM sheets (never committed).

use std::path::PathBuf;

use u5_automap::{MapLight, ReducedTiles, TILE_COLOUR_TABLE_LEN, TileColourTable};
use u5_runtime::{TileGraphicsDepth, load_tile_atlas};

#[test]
fn real_atlas_builds_a_full_table_at_every_light() {
    let Some(profile) = std::env::var_os("U5_MOCKUP_PROFILE").map(PathBuf::from) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE");
        return;
    };
    let atlas = load_tile_atlas(&profile, TileGraphicsDepth::Ega16).expect("TILES.16");
    let full = TileColourTable::build(&atlas, MapLight::FULL);
    assert_eq!(full.len(), TILE_COLOUR_TABLE_LEN);
    let drawn = (0..TILE_COLOUR_TABLE_LEN)
        .filter(|&tile| full.entry(tile).is_some_and(|entry| entry.coverage > 0))
        .count();
    assert!(drawn > 400, "{drawn} tiles drawn");
    let mut darker = 0usize;
    for percent in [85u8, 70, 55, 40, 25] {
        let dim = TileColourTable::build(&atlas, MapLight::from_percent(percent));
        assert_eq!(dim.len(), TILE_COLOUR_TABLE_LEN);
        for tile in 0..TILE_COLOUR_TABLE_LEN {
            let (a, b) = (dim.colour(tile).unwrap(), full.colour(tile).unwrap());
            assert!(a.iter().zip(b).all(|(a, b)| *a <= b));
            darker += usize::from(a != b);
        }
    }
    assert!(darker > 0);
    for size in [1, 4, 6, 7, 8] {
        for light in [MapLight::FULL, MapLight::from_percent(70)] {
            let reduced = ReducedTiles::build(&atlas, size, light);
            for tile in 0..TILE_COLOUR_TABLE_LEN {
                assert_eq!(reduced.tile(tile).unwrap().len(), size * size);
            }
        }
    }
    eprintln!("tile colour table: {TILE_COLOUR_TABLE_LEN} entries, {drawn} drawn");

    if let Some(dir) = std::env::var_os("U5_RASTER_DUMP").map(PathBuf::from) {
        std::fs::create_dir_all(&dir).unwrap();
        // `layer-070` is the gem map's order (10.2.10): the reductions at
        // full light, then the whole map layer through the LUT.
        let layer = MapLight::from_percent(70).lut();
        for (name, size, light, dim_layer) in [
            ("reduced4-100", 4, MapLight::FULL, false),
            ("reduced4-070", 4, MapLight::from_percent(70), false),
            ("reduced4-layer-070", 4, MapLight::FULL, true),
            ("reduced7-100", 7, MapLight::FULL, false),
            ("reduced6-100", 6, MapLight::FULL, false),
            ("reduced8-100", 8, MapLight::FULL, false),
        ] {
            let reduced = ReducedTiles::build(&atlas, size, light);
            let (w, h) = (32 * size, 16 * size);
            let mut ppm = format!("P6\n{w} {h}\n255\n").into_bytes();
            for y in 0..h {
                for x in 0..w {
                    let tile = (y / size) * 32 + x / size;
                    let mut pixel = reduced.tile(tile).unwrap()[(y % size) * size + x % size];
                    if dim_layer {
                        pixel = pixel.map(|c| layer.apply(c));
                    }
                    ppm.extend_from_slice(&pixel);
                }
            }
            std::fs::write(dir.join(format!("{name}.ppm")), ppm).unwrap();
        }
    }
}

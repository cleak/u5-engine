//! Map light and the tile colour table (9.3.4, 9.6; M3-09).

use super::*;
use u5_runtime::enhancements::{
    CliSetting, GEM_G2, HostCapabilities, LIGHT_P1, LIGHT_P2, LockState, ResolveInput, SoftDep,
    TOGGLES, Toggle, resolve_as_shipped,
};
use u5_runtime::{STOCK_EGA_BROWN, TILE_ATLAS_TILE_PIXELS, TileGraphicsDepth};

const SIDE: usize = TILE_ATLAS_SIDE;

fn atlas_from(tiles: impl Fn(usize, usize, usize) -> u8) -> TileAtlas {
    let mut pixels = vec![0u8; TILE_COLOUR_TABLE_LEN * TILE_ATLAS_TILE_PIXELS];
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let tile = index / TILE_ATLAS_TILE_PIXELS;
        let within = index % TILE_ATLAS_TILE_PIXELS;
        *pixel = tiles(tile, within % SIDE, within / SIDE) & 0x0f;
    }
    TileAtlas {
        depth: TileGraphicsDepth::Ega16,
        pixels,
        dungeon_billboards: None,
        dungeon_sprites: None,
        runic_font: None,
    }
}

/// A deterministic mixed atlas: every tile a different pattern with some
/// black, from all-black to fully drawn.
fn mixed_atlas() -> TileAtlas {
    atlas_from(|tile, x, y| {
        let mut h = (tile as u32).wrapping_mul(2_654_435_761) ^ ((x * 31 + y * 17) as u32);
        h ^= h >> 13;
        h = h.wrapping_mul(0x5bd1_e995);
        h ^= h >> 15;
        // Tile id picks how much is black: tile % 9 of every 8 pixels.
        if (h % 8) < (tile % 9) as u32 {
            0
        } else {
            (1 + h % 15) as u8
        }
    })
}

/// The colour without any scale: chapter 4's table before chapter 9
/// added `MapLight` (decode, then `coverage ^ 1.1` alone).
fn unscaled(entry: LowpassEntry) -> [u8; 3] {
    let Premultiplied([r, g, b, a]) = entry.premultiplied();
    let coverage = f32::from(a) / 255.0;
    if coverage <= 0.0001 {
        return [0, 0, 0];
    }
    let curve = coverage.powf(u5_runtime::enhancements::BRIGHTNESS_CURVE_EXPONENT);
    [r, g, b].map(|p| {
        let decoded = (f32::from(p) / 255.0) / coverage;
        (linear_to_srgb(srgb_to_linear(decoded) * curve) * 255.0).round() as u8
    })
}

// --- ninth-virtue's vectors (9.6). ---------------------------------------

#[test]
fn a_solid_tile_keeps_its_colour_at_full_coverage() {
    let entry = LowpassEntry::from_pixels(std::iter::repeat_n([10, 20, 30], 256));
    assert_eq!(
        entry,
        LowpassEntry {
            rgb: [10, 20, 30],
            coverage: 255
        }
    );
}

#[test]
fn a_half_black_checker_keeps_its_colour_at_half_coverage() {
    let pixels = (0..256).map(|i| {
        if (i % 16 + i / 16) % 2 == 0 {
            [80, 120, 200]
        } else {
            [0, 0, 0]
        }
    });
    assert_eq!(
        LowpassEntry::from_pixels(pixels),
        LowpassEntry {
            rgb: [80, 120, 200],
            coverage: 128
        }
    );
}

#[test]
fn composite_over_matches_the_premultiplied_vectors() {
    let object = LowpassEntry {
        rgb: [220, 120, 20],
        coverage: 128,
    };
    let terrain = LowpassEntry {
        rgb: [20, 40, 60],
        coverage: 255,
    };
    assert_eq!(
        object.premultiplied().over(terrain.premultiplied()),
        Premultiplied([120, 80, 40, 255])
    );
    let object = LowpassEntry {
        rgb: [200, 80, 40],
        coverage: 64,
    };
    let terrain = LowpassEntry {
        rgb: [80, 120, 200],
        coverage: 128,
    };
    assert_eq!(
        object.premultiplied().over(terrain.premultiplied()),
        Premultiplied([80, 65, 85, 160])
    );
}

#[test]
fn the_mean_is_the_truncated_integer_mean_of_the_non_black_bytes() {
    // Three pixels of 1, 2 and 2: sum 5 / 3 truncates to 1.
    let pixels = [[1, 1, 1], [2, 2, 2], [2, 2, 2], [0, 0, 0]];
    let entry = LowpassEntry::from_pixels(pixels);
    assert_eq!(entry.rgb, [1, 1, 1]);
    // 3 of 4 drawn: (3 * 255 + 2) / 4 = 191.
    assert_eq!(entry.coverage, 191);
    // A whole tile uses (n * 255 + 128) / 256.
    for visible in [0usize, 1, 100, 128, 200, 255, 256] {
        let pixels = (0..256).map(|i| if i < visible { [9, 9, 9] } else { [0, 0, 0] });
        let expected = ((visible * 255 + 128) / 256) as u8;
        assert_eq!(
            LowpassEntry::from_pixels(pixels).coverage,
            expected,
            "{visible}"
        );
    }
    assert_eq!(LowpassEntry::from_pixels([]), LowpassEntry::default());
    assert_eq!(LowpassEntry::default().display(MapLight::FULL), [0, 0, 0]);
}

#[test]
fn full_coverage_at_full_light_is_the_colour_itself() {
    for value in 0..=u8::MAX {
        let entry = LowpassEntry {
            rgb: [value, value / 2, 255 - value],
            coverage: 255,
        };
        assert_eq!(
            entry.display(MapLight::FULL),
            entry.rgb,
            "{value}: coverage 1.0 ^ 1.1 at scale 1.0 changes nothing"
        );
    }
}

// --- MapLight. -------------------------------------------------------------

#[test]
fn scale_one_is_byte_identical_to_the_table_without_a_scale() {
    let atlas = mixed_atlas();
    let full = TileColourTable::build(&atlas, MapLight::FULL);
    assert_eq!(full, TileColourTable::build(&atlas, MapLight::default()));
    assert_eq!(
        full,
        TileColourTable::build(&atlas, MapLight::from_percent(100))
    );
    assert_eq!(full.len(), TILE_COLOUR_TABLE_LEN);
    for tile in 0..TILE_COLOUR_TABLE_LEN {
        let entry = full.entry(tile).unwrap();
        assert_eq!(full.colour(tile), Some(unscaled(entry)), "tile {tile}");
    }
    // The reductions too, at every size: none goes through a table at 1.0.
    for size in 1..=MAX_REDUCED_SIZE {
        let reduced = ReducedTiles::build(&atlas, size, MapLight::FULL);
        assert_eq!(reduced.size(), size);
        for tile in [0, 5, 77, 300, 511] {
            assert_eq!(reduced.tile(tile).unwrap().len(), size * size);
        }
    }
    // At 16 px a fully drawn tile is its own pixels.
    let solid = atlas_from(|tile, x, y| (1 + (tile + x + y) % 15) as u8);
    let reduced = ReducedTiles::build(&solid, 16, MapLight::FULL);
    for tile in [0, 9, 400] {
        let own: Vec<[u8; 3]> = solid
            .tile_pixels(tile)
            .unwrap()
            .iter()
            .map(|&i| MAP_PALETTE[usize::from(i)])
            .collect();
        assert_eq!(reduced.tile(tile).unwrap(), own.as_slice());
    }
}

#[test]
fn size_one_reductions_are_the_colour_table() {
    let atlas = mixed_atlas();
    for percent in [100u8, 85, 70, 40, 25] {
        let light = MapLight::from_percent(percent);
        let table = TileColourTable::build(&atlas, light);
        let reduced = ReducedTiles::build(&atlas, 1, light);
        for tile in 0..TILE_COLOUR_TABLE_LEN {
            assert_eq!(reduced.tile(tile).unwrap(), &[table.colour(tile).unwrap()]);
        }
    }
}

#[test]
fn the_table_is_monotonic_in_the_scale() {
    let atlas = mixed_atlas();
    let tables: Vec<TileColourTable> = (25..=100u8)
        .step_by(5)
        .map(|percent| TileColourTable::build(&atlas, MapLight::from_percent(percent)))
        .collect();
    for pair in tables.windows(2) {
        for tile in 0..TILE_COLOUR_TABLE_LEN {
            let (dim, bright) = (pair[0].colour(tile).unwrap(), pair[1].colour(tile).unwrap());
            for channel in 0..3 {
                assert!(
                    dim[channel] <= bright[channel],
                    "tile {tile}: {:?} then {:?}",
                    pair[0].light(),
                    pair[1].light()
                );
            }
        }
    }
    // Strictly darker somewhere at 70%, and never brighter.
    let full = TileColourTable::build(&atlas, MapLight::FULL);
    let dim = TileColourTable::build(&atlas, MapLight::from_percent(70));
    assert_ne!(full.colours(), dim.colours());
    // Box-filter sizes through BrightnessLut are monotonic too.
    for size in [6, 7, 8] {
        let mut last: Option<ReducedTiles> = None;
        for percent in (25..=100u8).step_by(15) {
            let now = ReducedTiles::build(&atlas, size, MapLight::from_percent(percent));
            if let Some(last) = &last {
                for tile in 0..TILE_COLOUR_TABLE_LEN {
                    for (a, b) in last.tile(tile).unwrap().iter().zip(now.tile(tile).unwrap()) {
                        assert!(
                            a.iter().zip(b).all(|(a, b)| a <= b),
                            "size {size} tile {tile}"
                        );
                    }
                }
            }
            last = Some(now);
        }
    }
}

#[test]
fn box_filter_sizes_go_through_the_brightness_lut() {
    let atlas = mixed_atlas();
    for size in [6, 7, 8] {
        let full = ReducedTiles::build(&atlas, size, MapLight::FULL);
        let light = MapLight::from_percent(70);
        let lut = light.lut();
        let dim = ReducedTiles::build(&atlas, size, light);
        for tile in 0..TILE_COLOUR_TABLE_LEN {
            let expected: Vec<[u8; 3]> = full
                .tile(tile)
                .unwrap()
                .iter()
                .map(|p| p.map(|c| lut.apply(c)))
                .collect();
            assert_eq!(dim.tile(tile).unwrap(), expected.as_slice(), "size {size}");
        }
    }
}

#[test]
fn a_size_that_does_not_divide_sixteen_keeps_every_pixels_share() {
    // A tile whose left 8 columns are white and right 8 black: at 7 px the
    // widened tile is 112 wide, block 16, so column 3 straddles the middle
    // (pixels 48..64 of 112, 56 of them white = 8 widened pixels of 16).
    let atlas = atlas_from(|_, x, _| if x < 8 { 15 } else { 0 });
    let reduced = ReducedTiles::build(&atlas, 7, MapLight::FULL);
    let row = &reduced.tile(0).unwrap()[..7];
    assert_eq!(&row[..3], &[[255, 255, 255]; 3]);
    assert_eq!(&row[4..], &[[0, 0, 0]; 3]);
    let half = LowpassEntry {
        rgb: [255, 255, 255],
        coverage: 128,
    }
    .display(MapLight::FULL);
    assert_eq!(row[3], half);
    // 6 px: widen 3, block 8; column 2 covers widened 16..24 = source
    // 5.33..8, all white; column 3 covers 24..32 = 8..10.67, all black.
    let reduced = ReducedTiles::build(&atlas, 6, MapLight::FULL);
    let row = &reduced.tile(0).unwrap()[..6];
    assert_eq!(&row[..3], &[[255, 255, 255]; 3]);
    assert_eq!(&row[3..], &[[0, 0, 0]; 3]);
}

#[test]
fn display_brightness_alone_leaves_the_table_unchanged() {
    let atlas = mixed_atlas();
    let classic = TileColourTable::build(&atlas, MapLight::FULL);
    let shipped = [LIGHT_P1, LIGHT_P2, GEM_G2];
    for value in ["25", "55", "70", "85"] {
        let cli = [CliSetting::Set {
            key: "display.brightness".into(),
            value: value.into(),
        }];
        let resolved = resolve_as_shipped(
            ResolveInput {
                stored: None,
                cli: &cli,
                lock: LockState::Off,
                caps: HostCapabilities::ALL,
            },
            &shipped,
        );
        assert!(resolved.display_light().brightness_on(), "{value}");
        let light = MapLight::from_percent(resolved.display_map_brightness());
        assert!(light.is_full(), "{value}");
        assert_eq!(TileColourTable::build(&atlas, light), classic, "{value}");
    }
    // The map key itself does reach the table.
    let cli = [CliSetting::Set {
        key: "display.map_brightness".into(),
        value: "70".into(),
    }];
    let resolved = resolve_as_shipped(
        ResolveInput {
            stored: None,
            cli: &cli,
            lock: LockState::Off,
            caps: HostCapabilities::ALL,
        },
        &shipped,
    );
    let light = MapLight::from_percent(resolved.display_map_brightness());
    assert_eq!(light, MapLight { scale: 0.7 });
    assert!(!resolved.display_light().brightness_on());
    assert_ne!(TileColourTable::build(&atlas, light), classic);
}

#[test]
fn map_brightness_prefers_the_gem_maps_brightness_link() {
    let spec = TOGGLES
        .iter()
        .find(|spec| spec.toggle == Toggle::DisplayMapBrightness)
        .unwrap();
    assert_eq!(
        spec.soft_deps,
        &[SoftDep {
            prefers: "gem_map.brightness_link",
            fallback: "inert",
        }]
    );
    assert!(spec.needs.is_empty());
    // LIGHT_P2 ships with brightness_link in M3-11, not here.
    assert!(!u5_runtime::enhancements::SHIPPED_PHASES.contains(&LIGHT_P2));
    assert!(
        u5_runtime::enhancements::OPTIONS
            .iter()
            .any(|option| option.key == "gem_map.brightness_link")
    );
}

#[test]
fn the_map_palette_is_the_presented_one_with_brown() {
    assert_eq!(MAP_PALETTE[6], STOCK_EGA_BROWN);
    assert_eq!(MAP_PALETTE[6], [0xaa, 0x55, 0x00]);
    assert_eq!(MAP_PALETTE[0], [0, 0, 0]);
    // Only index 0 is black, so skipping black is skipping palette 0.
    assert!(MAP_PALETTE[1..].iter().all(|rgb| *rgb != [0, 0, 0]));
    assert_eq!(MAP_PALETTE[15], [0xff, 0xff, 0xff]);
}

#[test]
fn a_short_atlas_gives_black_entries_not_a_panic() {
    let mut atlas = mixed_atlas();
    atlas.pixels.truncate(10 * TILE_ATLAS_TILE_PIXELS + 3);
    let table = TileColourTable::build(&atlas, MapLight::from_percent(70));
    assert_eq!(table.len(), TILE_COLOUR_TABLE_LEN);
    assert_eq!(table.entry(10), Some(LowpassEntry::default()));
    assert_eq!(table.colour(511), Some([0, 0, 0]));
    assert_eq!(table.colour(512), None);
    let reduced = ReducedTiles::build(&atlas, 4, MapLight::FULL);
    assert_eq!(reduced.tile(300).unwrap(), &[[0, 0, 0]; 16]);
    assert!(reduced.tile(512).is_none());
}

//! Chapter 9's sprite background masks and the compositing eligibility
//! rule (Phase B design 9.3.6, 9.3.9).
//!
//! The faithful renderer draws an object's whole 16x16 tile opaquely,
//! black bounding box included. Sprite compositing replaces the box's
//! *background* black with the terrain the cell would show if it were
//! empty. This module holds the pure half of that: which pixels of each
//! actor-half tile are background, and which cells may be composited at
//! all.
//!
//! - **Background.** A flood fill seeded with every border pixel whose
//!   palette index is 0, spreading to the four edge neighbours that are
//!   also index 0. Black the fill does not reach is interior and stays
//!   (an ankh's loop, a ladder's rungs). Diagonal contact does not
//!   connect.
//! - **Halo.** The background minus every background pixel with a
//!   non-background pixel among its eight neighbours, which keeps a
//!   1-pixel black outline that also closes the corners.
//!
//! Masks are built once from the player's own `TileAtlas` at load time.
//! Nothing here is cached to disk, and no mask or pixel count is
//! committed: the tests use invented tiles. There is no Bevy, settings,
//! OS or GPU code here (foundation decision 5).

use crate::{ACTOR_TILE_BANK_BASE, TILE_ATLAS_SIDE, TILE_ATLAS_TILE_PIXELS, TileAtlas};

/// One bit per pixel of a 16x16 tile: bit `y * 16 + x`, word `bit / 64`.
pub type TileMask = [u64; 4];

/// The number of actor-half tiles the masks cover (atlas 256..511).
pub const ACTOR_HALF_TILES: usize = 256;

/// Tiles whose art paints a background into the box, so a flood fill
/// would leave a ragged patch or remove the picture (9.3.9).
pub const KEEP_THE_BOX: [usize; 12] = [
    396, // shark, frame 1: dark water drawn into the tile
    397, // shark, frame 2: dark water drawn into the tile
    398, // shark, frame 3: dark water drawn into the tile
    399, // shark, frame 4: dark water drawn into the tile
    500, // dotted full-tile texture: vanishes on grass
    501, // dotted full-tile texture: vanishes on grass
    502, // dotted full-tile texture: vanishes on grass
    503, // dotted full-tile texture: vanishes on grass
    504, // dotted full-tile texture: vanishes on grass
    505, // dotted full-tile texture: vanishes on grass
    506, // dotted full-tile texture: vanishes on grass
    507, // dotted full-tile texture: vanishes on grass
];

/// Tiles whose black is the creature's body, drawn in dots on black, so
/// the fill shrinks them to a few dots (9.3.9).
pub const DARK_BODY: [usize; 13] = [
    256, // stars on black: the fill leaves a few points
    444, // eyes on a dark body, frame 1
    445, // eyes on a dark body, frame 2
    446, // eyes on a dark body, frame 3
    447, // eyes on a dark body, frame 4
    468, // wisp drawn in dots, frame 1
    469, // wisp drawn in dots, frame 2
    470, // wisp drawn in dots, frame 3
    471, // wisp drawn in dots, frame 4
    480, // creature drawn in dots on black, frame 1
    481, // creature drawn in dots on black, frame 2
    482, // creature drawn in dots on black, frame 3
    483, // creature drawn in dots on black, frame 4
];

/// Any other tile left with fewer non-background pixels than this keeps
/// its box (the near-empty fallback, 9.3.9).
pub const NEAR_EMPTY_PIXELS: u32 = 20;

/// Combat field tiles: their pixels are re-randomised noise at runtime,
/// so they keep their box (9.3.9, D9.9).
pub const COMBAT_FIELDS: std::ops::RangeInclusive<usize> = 0x1E8..=0x1EB;

/// Terrain under which the compositor stamps an occupied-fixture tile:
/// chair, bed, mirror, stocks, manacles, floor link. The substituted tile
/// already draws the fixture, so the cell keeps its box (9.3.9, D9.8).
pub const FURNITURE_UNDER: [u8; 11] = [
    0x84, 0x85, 0x90, 0x91, 0x92, 0x93, 0x9D, 0x9E, 0xAB, 0xC8, 0xC9,
];

const EMPTY_MASK: TileMask = [0; 4];
const SIDE: usize = TILE_ATLAS_SIDE;

/// The background and halo masks of the 256 actor-half tiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SpriteMasks {
    background: [TileMask; ACTOR_HALF_TILES],
    halo: [TileMask; ACTOR_HALF_TILES],
}

impl SpriteMasks {
    /// Build the masks from the player's atlas. A tile the atlas does not
    /// hold gets an empty mask, so it keeps its box.
    pub fn from_atlas(atlas: &TileAtlas) -> Self {
        let mut background = [EMPTY_MASK; ACTOR_HALF_TILES];
        let mut halo = [EMPTY_MASK; ACTOR_HALF_TILES];
        for slot in 0..ACTOR_HALF_TILES {
            if let Some(pixels) = atlas.tile_pixels(ACTOR_TILE_BANK_BASE + slot) {
                background[slot] = flood_mask(pixels);
                halo[slot] = halo_of(&background[slot]);
            }
        }
        Self { background, halo }
    }

    /// The pixels to replace with terrain for atlas id `tile`: the whole
    /// background, or the background minus its 1-pixel halo. A tile
    /// outside 256..511 has an empty mask.
    pub fn background(&self, tile: usize, halo: bool) -> &TileMask {
        let Some(slot) = actor_slot(tile) else {
            return &EMPTY_MASK;
        };
        if halo {
            &self.halo[slot]
        } else {
            &self.background[slot]
        }
    }

    /// The number of pixels the flood fill leaves (not background).
    pub fn kept_pixels(&self, tile: usize) -> u32 {
        TILE_ATLAS_TILE_PIXELS as u32 - mask_count(self.background(tile, false))
    }
}

fn actor_slot(tile: usize) -> Option<usize> {
    tile.checked_sub(ACTOR_TILE_BANK_BASE)
        .filter(|&slot| slot < ACTOR_HALF_TILES)
}

/// Whether pixel `(x, y)` is set in `mask`.
pub fn mask_bit(mask: &TileMask, x: usize, y: usize) -> bool {
    let bit = y * SIDE + x;
    mask[bit / 64] & (1 << (bit % 64)) != 0
}

fn set_bit(mask: &mut TileMask, x: usize, y: usize) {
    let bit = y * SIDE + x;
    mask[bit / 64] |= 1 << (bit % 64);
}

/// The number of set pixels in `mask`.
pub fn mask_count(mask: &TileMask) -> u32 {
    mask.iter().map(|word| word.count_ones()).sum()
}

/// The 4-connected border flood over palette index 0 (`flood_mask` in
/// the mockup script). `pixels` is one 16x16 tile, row-major.
pub fn flood_mask(pixels: &[u8]) -> TileMask {
    let black = |x: usize, y: usize| pixels.get(y * SIDE + x) == Some(&0);
    let mut mask = EMPTY_MASK;
    let mut queue = Vec::with_capacity(TILE_ATLAS_TILE_PIXELS);
    let seed = |x: usize, y: usize, mask: &mut TileMask, queue: &mut Vec<(usize, usize)>| {
        if black(x, y) && !mask_bit(mask, x, y) {
            set_bit(mask, x, y);
            queue.push((x, y));
        }
    };
    for i in 0..SIDE {
        seed(i, 0, &mut mask, &mut queue);
        seed(i, SIDE - 1, &mut mask, &mut queue);
        seed(0, i, &mut mask, &mut queue);
        seed(SIDE - 1, i, &mut mask, &mut queue);
    }
    while let Some((x, y)) = queue.pop() {
        if x > 0 {
            seed(x - 1, y, &mut mask, &mut queue);
        }
        if x + 1 < SIDE {
            seed(x + 1, y, &mut mask, &mut queue);
        }
        if y > 0 {
            seed(x, y - 1, &mut mask, &mut queue);
        }
        if y + 1 < SIDE {
            seed(x, y + 1, &mut mask, &mut queue);
        }
    }
    mask
}

/// The background minus every background pixel with a non-background
/// pixel among its eight neighbours (the replace set of `halo_of` in the
/// mockup script, which marks the kept outline).
pub fn halo_of(background: &TileMask) -> TileMask {
    let mut replace = EMPTY_MASK;
    for y in 0..SIDE {
        for x in 0..SIDE {
            if !mask_bit(background, x, y) {
                continue;
            }
            let outline = (y.saturating_sub(1)..=(y + 1).min(SIDE - 1)).any(|ny| {
                (x.saturating_sub(1)..=(x + 1).min(SIDE - 1))
                    .any(|nx| !mask_bit(background, nx, ny))
            });
            if !outline {
                set_bit(&mut replace, x, y);
            }
        }
    }
    replace
}

/// Whether atlas id `tile` keeps its box whatever lies under it: the two
/// lists, the combat fields and the near-empty fallback.
pub fn keeps_box(masks: &SpriteMasks, tile: usize) -> bool {
    KEEP_THE_BOX.contains(&tile)
        || DARK_BODY.contains(&tile)
        || COMBAT_FIELDS.contains(&tile)
        || actor_slot(tile).is_none()
        || masks.kept_pixels(tile) < NEAR_EMPTY_PIXELS
        || mask_count(masks.background(tile, false)) == 0
}

/// Whether the object cell that drew atlas id `drawn` over terrain
/// `under` may be composited (9.3.9). `drawn` is `None` for the
/// draw-nothing byte `0x16`, and `under` is `None` when no terrain lies
/// under the cell; both keep the faithful pixels. The rule is the same
/// with and without the halo.
pub fn composite_eligible(masks: &SpriteMasks, drawn: Option<usize>, under: Option<u8>) -> bool {
    let (Some(drawn), Some(under)) = (drawn, under) else {
        return false;
    };
    !FURNITURE_UNDER.contains(&under) && !keeps_box(masks, drawn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TILE_ATLAS_PIXEL_LEN, TileGraphicsDepth, actor_tile_for_byte};

    type Tile = [u8; TILE_ATLAS_TILE_PIXELS];

    fn tile_from(rows: [&str; 16]) -> Tile {
        let mut tile = [0u8; TILE_ATLAS_TILE_PIXELS];
        for (y, row) in rows.iter().enumerate() {
            assert_eq!(row.len(), 16);
            for (x, c) in row.bytes().enumerate() {
                tile[y * 16 + x] = if c == b'#' { 2 } else { 0 };
            }
        }
        tile
    }

    fn filled(colour: u8) -> Tile {
        [colour; TILE_ATLAS_TILE_PIXELS]
    }

    /// A tile with `kept` coloured pixels in a solid block at the top left
    /// of the interior, the rest black.
    fn with_kept(kept: usize) -> Tile {
        let mut tile = [0u8; TILE_ATLAS_TILE_PIXELS];
        for i in 0..kept {
            tile[(1 + i / 14) * 16 + 1 + i % 14] = 3;
        }
        tile
    }

    fn atlas_with(tiles: &[(usize, Tile)]) -> TileAtlas {
        let mut pixels = vec![0u8; TILE_ATLAS_PIXEL_LEN];
        // Every actor tile defaults to a plain sprite: a 10x10 block.
        let plain = {
            let mut t = [0u8; TILE_ATLAS_TILE_PIXELS];
            for y in 3..13 {
                for x in 3..13 {
                    t[y * 16 + x] = 4;
                }
            }
            t
        };
        for id in 256..512 {
            pixels[id * 256..(id + 1) * 256].copy_from_slice(&plain);
        }
        for (id, tile) in tiles {
            pixels[id * 256..(id + 1) * 256].copy_from_slice(tile);
        }
        TileAtlas {
            depth: TileGraphicsDepth::Ega16,
            pixels,
            dungeon_billboards: None,
            dungeon_sprites: None,
            runic_font: None,
        }
    }

    const RING: [&str; 16] = [
        "................",
        "................",
        "................",
        "....########....",
        "....#......#....",
        "....#......#....",
        "....#......#....",
        "....#......#....",
        "....#......#....",
        "....#......#....",
        "....########....",
        "................",
        "................",
        "................",
        "................",
        "................",
    ];

    #[test]
    fn a_ring_keeps_its_black_inside() {
        let mask = flood_mask(&tile_from(RING));
        for y in 4..10 {
            for x in 5..11 {
                assert!(!mask_bit(&mask, x, y), "inside ({x},{y}) is interior");
            }
        }
        assert!(!mask_bit(&mask, 4, 3), "the ring itself is not background");
        assert!(mask_bit(&mask, 0, 0) && mask_bit(&mask, 15, 15));
        assert!(mask_bit(&mask, 2, 6) && mask_bit(&mask, 13, 6));
        // 256 - 28 ring pixels - 36 interior pixels.
        assert_eq!(mask_count(&mask), 256 - 28 - 36);
    }

    #[test]
    fn a_diagonal_only_join_stays_interior() {
        // A one-pixel black pocket closed on its four sides, whose
        // diagonal neighbours are outside black.
        let tile = tile_from([
            "................",
            ".##.............",
            "#.#.............",
            "##..............",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
            "................",
        ]);
        let mask = flood_mask(&tile);
        // (1,2) is black, enclosed on its four sides; only its diagonals
        // (0,1), (2,3) touch the outside black.
        assert!(!mask_bit(&mask, 1, 2), "diagonal contact does not connect");
        assert!(mask_bit(&mask, 0, 1) && mask_bit(&mask, 2, 3));
        assert!(mask_bit(&mask, 0, 0));
    }

    #[test]
    fn no_black_border_gives_an_empty_mask() {
        let mut tile = filled(5);
        // Interior black that is not connected to the border.
        tile[8 * 16 + 8] = 0;
        assert_eq!(flood_mask(&tile), EMPTY_MASK);
        assert_eq!(halo_of(&EMPTY_MASK), EMPTY_MASK);
    }

    #[test]
    fn an_all_black_tile_is_all_background_and_has_no_outline() {
        let mask = flood_mask(&filled(0));
        assert_eq!(mask_count(&mask), 256);
        assert_eq!(halo_of(&mask), mask);
    }

    #[test]
    fn the_halo_keeps_exactly_the_background_with_an_opaque_neighbour() {
        let background = flood_mask(&tile_from(RING));
        let replace = halo_of(&background);
        for y in 0..16usize {
            for x in 0..16usize {
                let bg = mask_bit(&background, x, y);
                let mut opaque_neighbour = false;
                for ny in y.saturating_sub(1)..=(y + 1).min(15) {
                    for nx in x.saturating_sub(1)..=(x + 1).min(15) {
                        opaque_neighbour |= !mask_bit(&background, nx, ny);
                    }
                }
                let kept_outline = bg && opaque_neighbour;
                assert_eq!(
                    mask_bit(&replace, x, y),
                    bg && !kept_outline,
                    "pixel ({x},{y})"
                );
            }
        }
        // The ring's 8x8 outline gets a 10x10 halo: 100 - 64 = 36 pixels,
        // corners included.
        assert_eq!(mask_count(&background) - mask_count(&replace), 36);
        assert!(!mask_bit(&replace, 3, 2), "the halo closes the corner");
        assert!(mask_bit(&replace, 2, 2));
    }

    #[test]
    fn masks_come_from_the_actor_half_per_atlas_id() {
        let atlas = atlas_with(&[(300, tile_from(RING)), (301, filled(6))]);
        let masks = SpriteMasks::from_atlas(&atlas);
        assert_eq!(masks.background(300, false), &flood_mask(&tile_from(RING)));
        assert_eq!(
            masks.background(300, true),
            &halo_of(&flood_mask(&tile_from(RING)))
        );
        assert_eq!(masks.background(301, false), &EMPTY_MASK);
        // Outside the actor half: empty.
        assert_eq!(masks.background(5, false), &EMPTY_MASK);
        assert_eq!(masks.background(512, true), &EMPTY_MASK);
        assert_eq!(masks.kept_pixels(300), 64);
    }

    #[test]
    fn a_short_atlas_gives_empty_masks_and_no_panic() {
        let atlas = TileAtlas {
            depth: TileGraphicsDepth::Ega16,
            pixels: vec![0; 300 * 256],
            dungeon_billboards: None,
            dungeon_sprites: None,
            runic_font: None,
        };
        let masks = SpriteMasks::from_atlas(&atlas);
        assert_eq!(mask_count(masks.background(299, false)), 256);
        assert_eq!(masks.background(300, false), &EMPTY_MASK);
        assert!(!composite_eligible(&masks, Some(300), Some(5)));
    }

    // ---- 9.3.9, one test per row the rule owns ----

    const GRASS: Option<u8> = Some(0x05);

    fn plain_masks() -> SpriteMasks {
        SpriteMasks::from_atlas(&atlas_with(&[]))
    }

    #[test]
    fn an_ordinary_sprite_on_grass_composites() {
        let masks = plain_masks();
        assert!(composite_eligible(&masks, Some(300), GRASS));
    }

    #[test]
    fn furniture_under_keeps_the_box() {
        let masks = plain_masks();
        assert_eq!(FURNITURE_UNDER.len(), 11);
        for under in FURNITURE_UNDER {
            assert!(
                !composite_eligible(&masks, Some(300), Some(under)),
                "{under:#04x}"
            );
        }
        for under in [0x83u8, 0x86, 0x8F, 0x94, 0x9C, 0x9F, 0xAA, 0xAC, 0xC7, 0xCA] {
            assert!(
                composite_eligible(&masks, Some(300), Some(under)),
                "{under:#04x}"
            );
        }
    }

    #[test]
    fn the_draw_nothing_byte_is_skipped() {
        let masks = plain_masks();
        let drawn = actor_tile_for_byte(0x16);
        assert_eq!(drawn, None);
        assert!(!composite_eligible(&masks, drawn, GRASS));
        assert!(composite_eligible(&masks, actor_tile_for_byte(0x17), GRASS));
    }

    #[test]
    fn combat_fields_keep_the_box() {
        let masks = plain_masks();
        for tile in 0x1E8..=0x1EB {
            assert!(!composite_eligible(&masks, Some(tile), GRASS), "{tile}");
        }
        assert!(composite_eligible(&masks, Some(0x1E7), GRASS));
        assert!(composite_eligible(&masks, Some(0x1EC), GRASS));
    }

    #[test]
    fn no_terrain_under_keeps_the_box() {
        let masks = plain_masks();
        assert!(!composite_eligible(&masks, Some(300), None));
    }

    #[test]
    fn black_terrain_still_composites() {
        let masks = plain_masks();
        // Tile 0 (a black terrain byte) is not a furniture id.
        assert!(composite_eligible(&masks, Some(300), Some(0x00)));
    }

    #[test]
    fn the_keep_the_box_list_keeps_the_box() {
        let masks = plain_masks();
        let ids: Vec<usize> = (396..=399).chain(500..=507).collect();
        assert_eq!(KEEP_THE_BOX.to_vec(), ids);
        for tile in KEEP_THE_BOX {
            assert!(!composite_eligible(&masks, Some(tile), GRASS), "{tile}");
        }
        assert!(composite_eligible(&masks, Some(395), GRASS));
        assert!(composite_eligible(&masks, Some(508), GRASS));
    }

    #[test]
    fn the_dark_body_list_keeps_the_box() {
        let masks = plain_masks();
        let ids: Vec<usize> = std::iter::once(256)
            .chain(444..=447)
            .chain(468..=471)
            .chain(480..=483)
            .collect();
        assert_eq!(DARK_BODY.to_vec(), ids);
        assert_eq!(KEEP_THE_BOX.len() + DARK_BODY.len(), 25);
        for tile in DARK_BODY {
            assert!(!composite_eligible(&masks, Some(tile), GRASS), "{tile}");
        }
        assert!(composite_eligible(&masks, Some(257), GRASS));
        assert!(composite_eligible(&masks, Some(443), GRASS));
    }

    #[test]
    fn nineteen_pixels_left_is_opaque_and_twenty_composites() {
        let masks =
            SpriteMasks::from_atlas(&atlas_with(&[(266, with_kept(19)), (267, with_kept(20))]));
        assert_eq!(masks.kept_pixels(266), 19);
        assert_eq!(masks.kept_pixels(267), 20);
        assert!(!composite_eligible(&masks, Some(266), GRASS));
        assert!(composite_eligible(&masks, Some(267), GRASS));
    }

    #[test]
    fn enclosed_black_stays_and_still_composites() {
        let masks = SpriteMasks::from_atlas(&atlas_with(&[(300, tile_from(RING))]));
        assert!(composite_eligible(&masks, Some(300), GRASS));
        for halo in [false, true] {
            assert!(!mask_bit(masks.background(300, halo), 7, 7));
        }
    }

    #[test]
    fn a_full_bleed_tile_needs_no_rule() {
        let masks = SpriteMasks::from_atlas(&atlas_with(&[(300, filled(9))]));
        assert_eq!(masks.background(300, false), &EMPTY_MASK);
        assert!(!composite_eligible(&masks, Some(300), GRASS));
    }

    #[test]
    fn the_rule_is_the_same_with_and_without_the_halo() {
        // composite_eligible takes no halo argument; the lists and the
        // threshold count the flood fill without the halo.
        let masks = SpriteMasks::from_atlas(&atlas_with(&[(267, with_kept(20))]));
        assert!(mask_count(masks.background(267, true)) < mask_count(masks.background(267, false)));
        assert_eq!(masks.kept_pixels(267), 20);
    }

    #[test]
    fn transports_and_the_party_follow_the_ordinary_rule() {
        // No id outside the lists, fields and fallback is special-cased.
        let masks = plain_masks();
        let special: Vec<usize> = KEEP_THE_BOX
            .iter()
            .chain(DARK_BODY.iter())
            .copied()
            .chain(COMBAT_FIELDS)
            .collect();
        for tile in 256..512 {
            assert_eq!(
                composite_eligible(&masks, Some(tile), GRASS),
                !special.contains(&tile),
                "{tile}"
            );
        }
    }
}

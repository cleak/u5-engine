//! Map light and the tile colour table (Phase B design chapter 9 P2,
//! 9.3.4, D9.3; chapter 4, 4.3.5; chapter 10, 10.2.4).
//!
//! A map draws each cell as its real 16x16 tile shrunk to a few pixels.
//! A shrunk pixel takes ninth-virtue's low-pass sample of its block:
//!
//! 1. skip every pixel of palette index 0 (EGA black: outline, shadow);
//! 2. `rgb` = the integer mean of the remaining pixels' sRGB bytes (sum
//!    divided by count, truncated), or black if none remain;
//! 3. `coverage` = the drawn share of the block as a byte,
//!    `(visible * 255 + total / 2) / total`, which is
//!    `(visible * 255 + 128) / 256` for a whole tile;
//! 4. the sample is stored premultiplied, `(c * coverage + 127) / 255`,
//!    as ninth-virtue's low-pass texture holds it, and displayed by
//!    decoding it and darkening it in linear light:
//!    `linear_to_srgb(srgb_to_linear(c) * coverage_scale(coverage, s))`,
//!    where `coverage_scale` is chapter 9's `coverage ^ 1.1 * s`.
//!
//! `s` is [`MapLight`]'s scale, `display.map_brightness / 100`; the main
//! view's `display.brightness` never reaches a map (D9.4b). At 1.0, the
//! default, every output is byte-identical to the table without a scale.
//!
//! Sizes: 1 to 4 px per cell fold the scale into step 4, as above (the
//! tile colour table's path). From 5 px up the block sample is the box
//! filter: it is decoded at full light and the result goes through
//! chapter 9's [`BrightnessLut`] for `s`. A size that does not divide 16
//! (6 or 7 px) widens the tile by the smallest whole factor first, so
//! every source pixel keeps its exact share of each block.
//!
//! Everything is a CPU table over bytes: no Bevy, shader or OS code
//! (foundation decision 5). Nothing here reads object bytes, so no person
//! or monster can reach a map through this module.

use u5_runtime::enhancements::{BrightnessLut, coverage_scale, linear_to_srgb, srgb_to_linear};
use u5_runtime::{
    MonitorModel, SHIPPED_PALETTE_REGISTERS, TILE_ATLAS_SIDE, TILE_ATLAS_TILE_COUNT, TileAtlas,
    resolve_palette_register,
};

/// The sixteen colours a map is drawn in: the shipped register values
/// resolved under the period 200-line display, as the frontend presents
/// the game, so index 6 is brown `#AA5500`.
pub const MAP_PALETTE: [[u8; 3]; 16] = map_palette();

const fn map_palette() -> [[u8; 3]; 16] {
    let mut palette = [[0u8; 3]; 16];
    let mut index = 0;
    while index < palette.len() {
        palette[index] = resolve_palette_register(
            SHIPPED_PALETTE_REGISTERS[index],
            MonitorModel::Period200Line,
        );
        index += 1;
    }
    palette
}

/// Entries in a tile colour table: one per atlas tile.
pub const TILE_COLOUR_TABLE_LEN: usize = TILE_ATLAS_TILE_COUNT;

/// The largest cell size a reduction takes: the tile's own 16 px.
pub const MAX_REDUCED_SIZE: usize = TILE_ATLAS_SIDE;

/// The first cell size drawn by the box filter rather than the colour
/// table's path (9.3.4: the table covers 1 to 4 px per cell).
pub const BOX_FILTER_MIN_SIZE: usize = 5;

/// The maps' brightness (9.3.4): `scale = display.map_brightness / 100`.
/// 1.0, the default, changes nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MapLight {
    pub scale: f32,
}

impl MapLight {
    /// Full light: the maps exactly as chapter 4 draws them.
    pub const FULL: Self = Self { scale: 1.0 };

    /// The light for a `display.map_brightness` percent. Above 100 reads
    /// as 100.
    pub fn from_percent(percent: u8) -> Self {
        Self {
            scale: f32::from(percent.min(100)) / 100.0,
        }
    }

    /// Whether this is full light (the scale is exactly 1.0).
    pub fn is_full(self) -> bool {
        self.scale == 1.0
    }

    /// Chapter 9's channel table for this scale.
    pub fn lut(self) -> BrightnessLut {
        BrightnessLut::new(self.scale)
    }
}

impl Default for MapLight {
    fn default() -> Self {
        Self::FULL
    }
}

/// ninth-virtue's low-pass sample of a block: the mean of its non-black
/// pixels and the share of the block they cover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LowpassEntry {
    pub rgb: [u8; 3],
    pub coverage: u8,
}

impl LowpassEntry {
    /// The sample of a block of sRGB pixels. A pixel counts as black, and
    /// is skipped, when it is exactly `[0, 0, 0]`. An empty block is black
    /// with no coverage.
    pub fn from_pixels(pixels: impl IntoIterator<Item = [u8; 3]>) -> Self {
        let mut sum = [0u64; 3];
        let mut visible = 0u64;
        let mut total = 0u64;
        for pixel in pixels {
            total += 1;
            if pixel == [0, 0, 0] {
                continue;
            }
            visible += 1;
            for (sum, channel) in sum.iter_mut().zip(pixel) {
                *sum += u64::from(channel);
            }
        }
        if total == 0 || visible == 0 {
            return Self::default();
        }
        let coverage = (visible * 255 + total / 2) / total;
        Self {
            rgb: sum.map(|sum| (sum / visible) as u8),
            coverage: coverage as u8,
        }
    }

    /// The sample stored premultiplied, as ninth-virtue's low-pass
    /// texture holds it: `(c * coverage + 127) / 255`, alpha = coverage.
    pub fn premultiplied(self) -> Premultiplied {
        let cov = u32::from(self.coverage);
        let [r, g, b] = self.rgb.map(|c| ((u32::from(c) * cov + 127) / 255) as u8);
        Premultiplied([r, g, b, self.coverage])
    }

    /// The displayed colour at `light` (step 4).
    pub fn display(self, light: MapLight) -> [u8; 3] {
        self.premultiplied().display(light)
    }
}

/// A premultiplied RGBA texel: colour bytes already multiplied by alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Premultiplied(pub [u8; 4]);

impl Premultiplied {
    /// ninth-virtue's `composite_over`: this texel over `below`, both
    /// premultiplied, `out = top + below * (255 - top_alpha) / 255`,
    /// rounded per channel.
    pub fn over(self, below: Self) -> Self {
        let inverse = 255 - u32::from(self.0[3]);
        let mut out = [0u8; 4];
        for ((out, top), under) in out.iter_mut().zip(self.0).zip(below.0) {
            let value = u32::from(top) + (u32::from(under) * inverse + 127) / 255;
            *out = value.min(255) as u8;
        }
        Self(out)
    }

    /// The displayed colour: the texel decoded (colour divided by its
    /// coverage) and darkened in linear light by `coverage ^ 1.1` times
    /// the light's scale. No coverage is black.
    pub fn display(self, light: MapLight) -> [u8; 3] {
        let coverage = f32::from(self.0[3]) / 255.0;
        if coverage <= 0.0001 {
            return [0, 0, 0];
        }
        let scale = coverage_scale(coverage, light.scale);
        let [r, g, b, _] = self.0;
        [r, g, b].map(|p| {
            let decoded = (f32::from(p) / 255.0) / coverage;
            // `linear_to_srgb` clamps to 0..1, so the cast cannot wrap.
            (linear_to_srgb(srgb_to_linear(decoded) * scale) * 255.0).round() as u8
        })
    }
}

/// The tile colour table (4.3.5, 9.3.4): one low-pass entry and one
/// displayed colour per atlas tile, built once per atlas and light.
#[derive(Clone, Debug, PartialEq)]
pub struct TileColourTable {
    light: MapLight,
    entries: Vec<LowpassEntry>,
    colours: Vec<[u8; 3]>,
}

impl TileColourTable {
    /// The table for the player's atlas in [`MAP_PALETTE`].
    pub fn build(atlas: &TileAtlas, light: MapLight) -> Self {
        Self::build_with_palette(atlas, &MAP_PALETTE, light)
    }

    /// The table in an explicit palette. A tile the atlas does not hold
    /// is black with no coverage; a palette index past 15 wraps, as the
    /// viewport's conversion does.
    pub fn build_with_palette(atlas: &TileAtlas, palette: &[[u8; 3]; 16], light: MapLight) -> Self {
        let entries: Vec<LowpassEntry> = (0..TILE_COLOUR_TABLE_LEN)
            .map(|tile| {
                atlas
                    .tile_pixels(tile)
                    .map_or_else(LowpassEntry::default, |pixels| {
                        LowpassEntry::from_pixels(
                            pixels.iter().map(|&index| rgb_of(palette, index)),
                        )
                    })
            })
            .collect();
        let colours = entries.iter().map(|entry| entry.display(light)).collect();
        Self {
            light,
            entries,
            colours,
        }
    }

    /// The light the table was built for.
    pub fn light(&self) -> MapLight {
        self.light
    }

    /// The number of entries, [`TILE_COLOUR_TABLE_LEN`].
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Never true: the table always holds every tile.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The low-pass entry of `tile`, before the curve.
    pub fn entry(&self, tile: usize) -> Option<LowpassEntry> {
        self.entries.get(tile).copied()
    }

    /// The displayed colour of `tile`.
    pub fn colour(&self, tile: usize) -> Option<[u8; 3]> {
        self.colours.get(tile).copied()
    }

    /// Every displayed colour, by tile id.
    pub fn colours(&self) -> &[[u8; 3]] {
        &self.colours
    }
}

/// Every atlas tile reduced to `size` x `size` pixels for one light, by
/// the block sample of the module notes. Size 1 is the colour table.
#[derive(Clone, Debug, PartialEq)]
pub struct ReducedTiles {
    size: usize,
    light: MapLight,
    pixels: Vec<[u8; 3]>,
}

impl ReducedTiles {
    /// The reductions of the player's atlas in [`MAP_PALETTE`]. `size`
    /// is clamped to 1..=16.
    pub fn build(atlas: &TileAtlas, size: usize, light: MapLight) -> Self {
        Self::build_with_palette(atlas, &MAP_PALETTE, size, light)
    }

    /// The reductions in an explicit palette.
    pub fn build_with_palette(
        atlas: &TileAtlas,
        palette: &[[u8; 3]; 16],
        size: usize,
        light: MapLight,
    ) -> Self {
        let size = size.clamp(1, MAX_REDUCED_SIZE);
        let lut = (size >= BOX_FILTER_MIN_SIZE && !light.is_full()).then(|| light.lut());
        let mut pixels = Vec::with_capacity(TILE_COLOUR_TABLE_LEN * size * size);
        for tile in 0..TILE_COLOUR_TABLE_LEN {
            match atlas.tile_pixels(tile) {
                Some(source) => {
                    let start = pixels.len();
                    reduce_into(&mut pixels, source, palette, size, light);
                    if let Some(lut) = &lut {
                        for pixel in &mut pixels[start..] {
                            *pixel = pixel.map(|c| lut.apply(c));
                        }
                    }
                }
                None => pixels.extend(std::iter::repeat_n([0, 0, 0], size * size)),
            }
        }
        Self {
            size,
            light,
            pixels,
        }
    }

    /// Pixels per cell side.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The light the reductions were built for.
    pub fn light(&self) -> MapLight {
        self.light
    }

    /// The `size * size` pixels of `tile`, row-major.
    pub fn tile(&self, tile: usize) -> Option<&[[u8; 3]]> {
        let len = self.size * self.size;
        let start = tile.checked_mul(len)?;
        self.pixels.get(start..start.checked_add(len)?)
    }
}

/// One tile (`TILE_ATLAS_SIDE`-square palette indices) reduced to
/// `size` x `size`, appended row-major to `out`. Sizes from
/// [`BOX_FILTER_MIN_SIZE`] up are decoded at full light (the caller's
/// [`BrightnessLut`] scales them); smaller sizes fold the scale into the
/// curve.
fn reduce_into(
    out: &mut Vec<[u8; 3]>,
    source: &[u8],
    palette: &[[u8; 3]; 16],
    size: usize,
    light: MapLight,
) {
    let side = TILE_ATLAS_SIDE;
    // The smallest whole widening that makes `size` divide the side.
    let widen = (1..=side)
        .find(|widen| (side * widen).is_multiple_of(size))
        .unwrap_or(side);
    let block = side * widen / size;
    let curve_light = if size >= BOX_FILTER_MIN_SIZE {
        MapLight::FULL
    } else {
        light
    };
    for by in 0..size {
        for bx in 0..size {
            let pixels = (0..block).flat_map(|j| {
                (0..block).map(move |i| {
                    let x = (bx * block + i) / widen;
                    let y = (by * block + j) / widen;
                    rgb_of(palette, source[y * side + x])
                })
            });
            out.push(LowpassEntry::from_pixels(pixels).display(curve_light));
        }
    }
}

fn rgb_of(palette: &[[u8; 3]; 16], index: u8) -> [u8; 3] {
    palette[usize::from(index) % palette.len()]
}

#[cfg(test)]
#[path = "raster_tests.rs"]
mod tests;

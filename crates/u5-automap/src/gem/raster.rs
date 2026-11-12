//! The gem map's CPU raster (Phase B design 10.3.4; build plan M3-10).
//!
//! [`render_gem_map`] draws one open gem view into an RGBA image the size
//! of the canvas, edge to edge, with no frame or panel:
//!
//! 1. the map layer: each shown cell's real tile reduced to 4 px
//!    (ninth-virtue's non-black mean and `coverage ^ 1.1` curve, chapter 9
//!    9.3.4), on black;
//! 2. the map layer dimmed in linear light through chapter 9's
//!    [`BrightnessLut`] to `gem_map.map_brightness` (70% by default), or to
//!    `display.map_brightness` when `gem_map.brightness_link` is on and it
//!    is below 100 (10.2.10);
//! 3. the steady party marker at full brightness: the party's own sprite
//!    (its tile and transport, never an object byte) reduced to 8 px on
//!    black, inside a 1 px black, 2 px EGA 14 and 1 px black box, 16 px in
//!    all at 4 px cells (10.2.7). It never blinks.
//!
//! **In a town** (10.2.5, M3-12) the same steps run at the town's cell size
//! (7 px at 240 lines, 6 px at 200 or 216), by the box filter: the floor at
//! the map brightness, the strips and the seen land beside it at
//! [`TOWN_SIDE_PERCENT`] of it, so the town reads as the subject, unseen
//! side cells black, then the party marker on its floor cell.
//!
//! Marks, labels and the "?" (10.2.7) come later (M3-16); dungeons (M3-13)
//! add their own arm. Until then that view renders cleared.
//!
//! The render reads `&PlayState` and the view's cached shown set only: it
//! changes nothing in the game, draws no random number and reads no file.

use std::sync::OnceLock;

use u5_runtime::enhancements::{BrightnessLut, GemMapToggles};
use u5_runtime::{ACTOR_TILE_BANK_BASE, PlayState, TileAtlas, TransportState};

use super::knowledge::GemKnowledge;
use super::layout::GemCanvas;
use super::shown::{GemShown, GemViewState, OutdoorShown, TownPart, TownShown, gem_view_open};
use crate::raster::{MAP_PALETTE, MAX_REDUCED_SIZE, MapLight, ReducedTiles};

/// The land beside a town and the strips, as a share of the map
/// brightness in percent (10.2.5, 10.3.4 step 3).
pub const TOWN_SIDE_PERCENT: u8 = 55;

/// The party sprite's side in pixels (10.2.7): two cells at 4 px.
pub const PARTY_SPRITE_PX: u32 = 8;

/// The party marker's outer side: the sprite, 1 px black, 2 px EGA 14 and
/// 1 px black on each side.
pub const PARTY_MARKER_PX: u32 = PARTY_SPRITE_PX + 8;

/// EGA 14, kept for the party and the dungeon link boxes (10.2.7).
pub const PARTY_YELLOW: [u8; 3] = MAP_PALETTE[14];

const OPAQUE_BLACK: [u8; 4] = [0, 0, 0, 255];

/// A plain RGBA8 image, row-major, 4 bytes a pixel.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GemImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl GemImage {
    /// A transparent image of `canvas`'s size.
    pub fn new(canvas: GemCanvas) -> Self {
        Self {
            w: canvas.w,
            h: canvas.h,
            rgba: vec![0; canvas.pixels() * 4],
        }
    }

    /// Resizes to `canvas` and makes every pixel transparent black.
    pub fn clear(&mut self, canvas: GemCanvas) {
        self.w = canvas.w;
        self.h = canvas.h;
        self.rgba.clear();
        self.rgba.resize(canvas.pixels() * 4, 0);
    }

    /// Whether every pixel is transparent black.
    pub fn is_cleared(&self) -> bool {
        self.rgba.iter().all(|&byte| byte == 0)
    }

    /// The pixel at `(x, y)`, or `None` off the image.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.w || y >= self.h {
            return None;
        }
        let at = (y as usize * self.w as usize + x as usize) * 4;
        self.rgba.get(at..at + 4).map(|p| [p[0], p[1], p[2], p[3]])
    }

    fn fill(&mut self, rgba: [u8; 4]) {
        for pixel in self.rgba.as_chunks_mut::<4>().0 {
            *pixel = rgba;
        }
    }

    /// Sets `(x, y)` when it is on the image.
    fn put(&mut self, x: i32, y: i32, rgb: [u8; 3]) {
        if x < 0 || y < 0 || x as u32 >= self.w || y as u32 >= self.h {
            return;
        }
        let at = (y as usize * self.w as usize + x as usize) * 4;
        self.rgba[at..at + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }

    /// The outline of the rectangle `x0..=x1` by `y0..=y1`, `width` px
    /// thick, inwards.
    fn frame(&mut self, (x0, y0, x1, y1): (i32, i32, i32, i32), width: i32, rgb: [u8; 3]) {
        for k in 0..width {
            for x in x0 + k..=x1 - k {
                self.put(x, y0 + k, rgb);
                self.put(x, y1 - k, rgb);
            }
            for y in y0 + k..=y1 - k {
                self.put(x0 + k, y, rgb);
                self.put(x1 - k, y, rgb);
            }
        }
    }
}

/// The art the gem map draws with, built once per atlas (10.3.4): every
/// tile reduced to 4 px for the map and to 8 px for the party sprite, both
/// at full light (the map's dimming is a separate step, so the marker
/// stays at full brightness). The town sizes (6 and 7 px) are reduced on
/// first use.
#[derive(Clone, Debug, PartialEq)]
pub struct MapArt {
    pub cells: ReducedTiles,
    pub sprites: ReducedTiles,
    atlas: TileAtlas,
    sized: [OnceLock<ReducedTiles>; MAX_REDUCED_SIZE + 1],
}

impl MapArt {
    /// The art from the player's own TILES.16.
    pub fn build(atlas: &TileAtlas) -> Self {
        Self {
            cells: ReducedTiles::build(
                atlas,
                super::layout::OUTDOOR_CELL_PX as usize,
                MapLight::FULL,
            ),
            sprites: ReducedTiles::build(atlas, PARTY_SPRITE_PX as usize, MapLight::FULL),
            atlas: atlas.clone(),
            sized: std::array::from_fn(|_| OnceLock::new()),
        }
    }

    /// Every tile reduced to `size` px at full light (clamped to 1..=16),
    /// reduced once and kept.
    pub fn cells_at(&self, size: u32) -> &ReducedTiles {
        let size = (size as usize).clamp(1, MAX_REDUCED_SIZE);
        if size == self.cells.size() {
            return &self.cells;
        }
        if size == self.sprites.size() {
            return &self.sprites;
        }
        self.sized[size].get_or_init(|| ReducedTiles::build(&self.atlas, size, MapLight::FULL))
    }
}

/// What a render drew.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GemRender {
    /// Nothing: the image is transparent, and the game's own view shows.
    Cleared,
    Outdoor,
    Town,
}

/// The map layer's brightness in percent for this view (10.2.10).
pub fn map_percent(t: &GemMapToggles, view: &GemViewState) -> u8 {
    t.map_brightness(view.display_map_brightness)
}

/// Draws the open gem view into `out`, sized to the view's canvas
/// (10.3.2, 10.3.4). `out` is cleared, and [`GemRender::Cleared`]
/// returned, unless the feature is on and the game's view overlay is a gem
/// view with a shown set this build draws. `know` is the record the marks
/// and labels will read (M3-16); the map itself never depends on it.
pub fn render_gem_map(
    state: &PlayState,
    know: &GemKnowledge,
    art: &MapArt,
    t: &GemMapToggles,
    view: &GemViewState,
    out: &mut GemImage,
) -> GemRender {
    let _ = know;
    out.clear(view.canvas);
    if !t.enhanced || !gem_view_open(state) {
        return GemRender::Cleared;
    }
    match &view.shown {
        GemShown::Empty => GemRender::Cleared,
        GemShown::Outdoor(shown) => {
            let lut = BrightnessLut::for_percent(map_percent(t, view));
            draw_outdoor_map(shown, art, &lut, out);
            let (px, py) = shown.layout.party_px;
            draw_party_marker(state, art, (px, py), shown.layout.cell, out);
            GemRender::Outdoor
        }
        GemShown::Town(shown) => {
            let percent = map_percent(t, view);
            draw_town_map(shown, art, percent, out);
            let (fx, fy) = (i32::from(shown.party.0), i32::from(shown.party.1));
            let (px, py, _, _) = shown.layout.cell_rect(fx, fy);
            draw_party_marker(state, art, (px, py), shown.layout.cell, out);
            GemRender::Town
        }
    }
}

/// The brightness table for the land beside a town and the strips:
/// [`TOWN_SIDE_PERCENT`] of the map's `percent`, as one linear-light scale.
pub fn town_side_lut(percent: u8) -> BrightnessLut {
    BrightnessLut::new(f32::from(percent.min(100)) / 100.0 * f32::from(TOWN_SIDE_PERCENT) / 100.0)
}

/// Steps 1 to 3 in a town: black, then the floor through the map's table
/// and the strips and seen sides through the darker one. Black cells stay
/// black.
fn draw_town_map(shown: &TownShown, art: &MapArt, percent: u8, out: &mut GemImage) {
    out.fill(OPAQUE_BLACK);
    let cells = art.cells_at(shown.layout.cell);
    let size = cells.size();
    let floor_lut = BrightnessLut::for_percent(percent);
    let side_lut = town_side_lut(percent);
    let (w, h) = (out.w as i32, out.h as i32);
    for (fx, fy, part, tile) in shown.cells() {
        let Some(pixels) = tile.and_then(|tile| cells.tile(usize::from(tile))) else {
            continue;
        };
        let lut = match part {
            TownPart::Floor => &floor_lut,
            TownPart::Strip | TownPart::Side => &side_lut,
        };
        let (x0, y0, cw, ch) = shown.layout.cell_rect(fx, fy);
        for j in 0..ch.min(size as u32) as i32 {
            let y = y0 + j;
            if !(0..h).contains(&y) {
                continue;
            }
            for i in 0..cw.min(size as u32) as i32 {
                let x = x0 + i;
                if !(0..w).contains(&x) {
                    continue;
                }
                let rgb = pixels[j as usize * size + i as usize];
                let at = (y as usize * out.w as usize + x as usize) * 4;
                out.rgba[at] = lut.apply(rgb[0]);
                out.rgba[at + 1] = lut.apply(rgb[1]);
                out.rgba[at + 2] = lut.apply(rgb[2]);
            }
        }
    }
}

/// Steps 1 and 2 outdoors: black, then every shown cell's reduced tile
/// through the dimming table.
fn draw_outdoor_map(shown: &OutdoorShown, art: &MapArt, lut: &BrightnessLut, out: &mut GemImage) {
    out.fill(OPAQUE_BLACK);
    let size = art.cells.size();
    let (w, h) = (out.w as i32, out.h as i32);
    for (dx, dy, tile) in shown.cells() {
        let Some(pixels) = art.cells.tile(usize::from(tile)) else {
            continue;
        };
        let (x0, y0, cw, ch) = shown.layout.cell_rect(dx, dy);
        for j in 0..ch.min(size as u32) as i32 {
            let y = y0 + j;
            if !(0..h).contains(&y) {
                continue;
            }
            for i in 0..cw.min(size as u32) as i32 {
                let x = x0 + i;
                if !(0..w).contains(&x) {
                    continue;
                }
                let rgb = pixels[j as usize * size + i as usize];
                let at = (y as usize * out.w as usize + x as usize) * 4;
                out.rgba[at] = lut.apply(rgb[0]);
                out.rgba[at + 1] = lut.apply(rgb[1]);
                out.rgba[at + 2] = lut.apply(rgb[2]);
            }
        }
    }
}

/// The atlas tile the party marker shows: the party's own tile from its
/// transport (on foot, a horse, a ship, a skiff or the carpet), in the
/// actor bank. `None` when the transport draws nothing (`vehicles.md §2`).
pub fn party_sprite_tile(state: &PlayState) -> Option<usize> {
    match state.player.transport {
        TransportState::SpriteSuppressed => None,
        transport => Some(ACTOR_TILE_BANK_BASE + usize::from(transport.avatar_tile())),
    }
}

/// Step 3: the steady party marker centred on the party cell whose
/// top-left is `cell_px` (10.2.7), at full brightness.
fn draw_party_marker(
    state: &PlayState,
    art: &MapArt,
    cell_px: (i32, i32),
    cell: u32,
    out: &mut GemImage,
) {
    let s = PARTY_SPRITE_PX.max(cell) as i32;
    let x = cell_px.0 + cell as i32 / 2 - s / 2;
    let y = cell_px.1 + cell as i32 / 2 - s / 2;
    let sprite = party_sprite_tile(state).and_then(|tile| art.sprites.tile(tile));
    let size = art.sprites.size();
    for j in 0..s {
        for i in 0..s {
            let rgb = sprite
                .filter(|_| size as i32 == s)
                .map_or([0, 0, 0], |pixels| pixels[j as usize * size + i as usize]);
            out.put(x + i, y + j, rgb);
        }
    }
    out.frame((x - 4, y - 4, x + s + 3, y + s + 3), 1, [0, 0, 0]);
    out.frame((x - 3, y - 3, x + s + 2, y + s + 2), 2, PARTY_YELLOW);
    out.frame((x - 1, y - 1, x + s, y + s), 1, [0, 0, 0]);
}

#[cfg(test)]
#[path = "raster_tests.rs"]
mod tests;

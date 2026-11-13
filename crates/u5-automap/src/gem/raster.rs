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
//! **In a dungeon** (10.2.6, M3-13) the level panels of [`DungeonLayout`]:
//! each shown cell's 8 x 8 gem glyph from the engine's own
//! `draw_dungeon_view_glyph` with the IBM.CH and RUNES.CH fonts of
//! [`MapArt::with_fonts`], scaled to whole pixels ([`GlyphScale`]); flood
//! cells at the map brightness and remembered cells first dimmed to
//! [`DUNGEON_REMEMBERED_PERCENT`] ([`remembered_channel`]); with
//! `gem_map.wrap_rows` on, the
//! level repeated above and below each panel (and beside a lone panel) at
//! [`DUNGEON_WRAP_PERCENT`] of that, with a 1 px dark-grey edge where the
//! level's own 8 x 8 ends; then the yellow ladder link boxes (M3-14,
//! [`DungeonShown::linked`]), the party's yellow arrowhead pointing its
//! facing, and the panel titles in IBM.CH with a 1 px black outline. A lone
//! panel shows the paged level (`GemViewState::page`, M3-14).
//!
//! Marks, labels and the "?" (10.2.7) come later (M3-15, M3-16).
//!
//! The render reads `&PlayState` and the view's cached shown set only: it
//! changes nothing in the game, draws no random number and reads no file.

use std::sync::OnceLock;

use u5_runtime::enhancements::{BrightnessLut, GemMapToggles};
use u5_runtime::{
    ACTOR_TILE_BANK_BASE, Direction, DungeonViewFonts, FixedCellFont, PlayState, TileAtlas,
    TileGraphicsDepth, TileViewport, TransportState, draw_dungeon_view_glyph,
    dungeon_minimap_glyph,
};

use super::dungeon::{DungeonLit, DungeonShown, DungeonSlot};
use super::knowledge::GemKnowledge;
use super::layout::{DUNGEON_PANEL_CELLS, DungeonLayout, GemCanvas, GlyphScale};
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

/// The dungeon ladder link boxes' colour, the party's EGA 14 (10.2.6).
pub const LINK_YELLOW: [u8; 3] = PARTY_YELLOW;

/// Remembered dungeon cells, as a share of the map brightness in percent
/// (10.2.6, 10.3.4 step 3).
pub const DUNGEON_REMEMBERED_PERCENT: u8 = 70;

/// The dungeon wrap rows, as a share of the panel's own brightness in
/// percent (10.2.6, D10.2).
pub const DUNGEON_WRAP_PERCENT: u8 = 30;

/// The 1 px edge where a level's own 8 x 8 ends and its wrap begins: EGA 8.
pub const DUNGEON_EDGE_GREY: [u8; 3] = MAP_PALETTE[8];

/// The panel titles' colour: EGA 15.
pub const DUNGEON_TITLE_WHITE: [u8; 3] = MAP_PALETTE[15];

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
    /// IBM.CH and RUNES.CH, for the dungeon glyphs, the party arrowhead
    /// and the titles (10.3.4 step 2).
    fonts: Option<Box<(FixedCellFont, FixedCellFont)>>,
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
            fonts: None,
        }
    }

    /// The art with the fonts the shell already holds: `text` IBM.CH and
    /// `runic` RUNES.CH. Without them a dungeon's glyphs fall back to the
    /// painter's own vector stand-ins and no arrowhead or title is drawn.
    pub fn with_fonts(mut self, text: FixedCellFont, runic: FixedCellFont) -> Self {
        self.fonts = Some(Box::new((text, runic)));
        self
    }

    /// The dungeon view's fonts, when the art has them.
    pub fn fonts(&self) -> Option<DungeonViewFonts<'_>> {
        self.fonts
            .as_deref()
            .map(|(text, runic)| DungeonViewFonts { text, runic })
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
    Dungeon,
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
        GemShown::Dungeon(shown) => {
            let Some(layout) = DungeonLayout::new(view.canvas, t.opts.dungeon_stack) else {
                return GemRender::Cleared;
            };
            draw_dungeon(shown, &layout, view.page, art, t, map_percent(t, view), out);
            GemRender::Dungeon
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

/// The level each panel shows (10.2.6), as `(panel index, level)`: the
/// centre panel's level is the party's, or on a lone panel the paged one
/// (`view.page`, M3-14); the others are the levels above (left) and below
/// (right). Level -1 is the surface, 8 the Underworld.
pub fn dungeon_panel_levels(
    shown: &DungeonShown,
    layout: &DungeonLayout,
    page: i8,
) -> Vec<(u8, i32)> {
    let centre = i32::from(shown.level)
        + if layout.panels == 1 {
            i32::from(page)
        } else {
            0
        };
    (0..layout.panels)
        .map(|index| (index, centre + layout.offset(index)))
        .collect()
}

/// One gem glyph as the engine paints it (`draw_dungeon_view_glyph`,
/// 10.3.4), 8 x 8 palette indices, row-major.
pub fn dungeon_glyph_pixels(drawn: u8, fonts: Option<DungeonViewFonts<'_>>) -> [u8; 64] {
    let side = DUNGEON_PANEL_CELLS as usize;
    let mut viewport = TileViewport {
        depth: TileGraphicsDepth::Ega16,
        cells_wide: 1,
        cells_high: 1,
        width: side,
        height: side,
        pixels: vec![0; side * side],
    };
    draw_dungeon_view_glyph(
        &mut viewport,
        0,
        0,
        side,
        dungeon_minimap_glyph(drawn),
        fonts,
    );
    let mut out = [0u8; 64];
    out.copy_from_slice(&viewport.pixels);
    out
}

/// A remembered dungeon cell's channel (10.2.6): the sRGB byte times
/// [`DUNGEON_REMEMBERED_PERCENT`], truncated, before the map's own
/// brightness. This is the approved mockup's dimming (g16's `MEM_LEVEL`,
/// ninth-virtue's fog multiply), so remembered cells match it exactly.
pub fn remembered_channel(c: u8) -> u8 {
    (f64::from(c) * f64::from(DUNGEON_REMEMBERED_PERCENT) / 100.0) as u8
}

/// The brightness steps of a dungeon panel (10.2.6, 10.3.4 step 3): the
/// remembered dimming, the map brightness, and for a wrap row
/// [`DUNGEON_WRAP_PERCENT`] of the result, each through chapter 9's
/// [`BrightnessLut`] in linear light.
#[derive(Clone, Debug)]
pub struct DungeonLuts {
    map: BrightnessLut,
    wrap: BrightnessLut,
}

impl DungeonLuts {
    /// The steps at map brightness `percent`.
    pub fn new(percent: u8) -> Self {
        Self {
            map: BrightnessLut::for_percent(percent.min(100)),
            wrap: BrightnessLut::for_percent(DUNGEON_WRAP_PERCENT),
        }
    }

    /// The drawn colour of a glyph pixel `rgb` on a cell lit `lit`, in the
    /// level's own 8 x 8 (`core`) or in a wrap row.
    pub fn colour(&self, rgb: [u8; 3], lit: DungeonLit, core: bool) -> [u8; 3] {
        let rgb = match lit {
            DungeonLit::Flood => rgb,
            DungeonLit::Remembered => rgb.map(remembered_channel),
        };
        let rgb = rgb.map(|c| self.map.apply(c));
        if core {
            rgb
        } else {
            rgb.map(|c| self.wrap.apply(c))
        }
    }
}

/// The dungeon arm (10.2.6): the panels' map layer, their edges, the
/// party's arrowhead and the titles.
fn draw_dungeon(
    shown: &DungeonShown,
    layout: &DungeonLayout,
    page: i8,
    art: &MapArt,
    t: &GemMapToggles,
    percent: u8,
    out: &mut GemImage,
) {
    out.fill(OPAQUE_BLACK);
    let fonts = art.fonts();
    let luts = DungeonLuts::new(percent);
    let scale = layout.glyph_scale();
    let wrap = t.opts.wrap_rows;
    let mut glyphs: Vec<Option<[u8; 64]>> = vec![None; 256];
    let panels = dungeon_panel_levels(shown, layout, page);
    for &(index, level) in &panels {
        let DungeonSlot::Level { level, .. } = shown.slot(level) else {
            continue;
        };
        draw_dungeon_panel(
            shown,
            layout,
            index,
            level,
            &mut |drawn| {
                *glyphs[usize::from(drawn)]
                    .get_or_insert_with(|| dungeon_glyph_pixels(drawn, fonts))
            },
            scale,
            &luts,
            wrap,
            out,
        );
    }
    // The ladder link boxes, at full brightness, on the panel's own 8 x 8.
    for &(index, level) in &panels {
        if let DungeonSlot::Level { level, cells: true } = shown.slot(level) {
            draw_link_boxes(shown, layout, index, level, out);
        }
    }
    let Some(fonts) = fonts else {
        return;
    };
    // The party: the IBM.CH arrow of its facing in EGA 14, on black.
    if let Some(&(index, _)) = panels
        .iter()
        .find(|(_, level)| *level == i32::from(shown.level))
    {
        let (px, py) = layout.panel_origin(index);
        let c = layout.cell as i32;
        let (x0, y0) = (
            px + i32::from(shown.party.0) * c,
            py + i32::from(shown.party.1) * c,
        );
        for j in 0..c {
            for i in 0..c {
                out.put(x0 + i, y0 + j, [0, 0, 0]);
            }
        }
        let k = (layout.cell / DUNGEON_PANEL_CELLS).max(1) as i32;
        let off = (c - 8 * k) / 2;
        let code = facing_arrow(shown.facing);
        for row in 0..8 {
            let bits = fonts.text.glyph_row(code, row as usize).unwrap_or(0);
            for col in 0..8 {
                if bits & (0x80 >> col) == 0 {
                    continue;
                }
                for dy in 0..k {
                    for dx in 0..k {
                        out.put(
                            x0 + off + col * k + dx,
                            y0 + off + row * k + dy,
                            PARTY_YELLOW,
                        );
                    }
                }
            }
        }
    }
    // The titles, centred over each panel; with no room above (4:3), in the
    // margin to the left, on two lines when one is too wide.
    let p = layout.panel_px() as i32;
    for &(index, level) in &panels {
        let Some(title) = shown.title(level) else {
            continue;
        };
        let (px, py) = layout.panel_origin(index);
        let (w, h) = text_tag_size(&title);
        let ty = py - h - 3;
        if ty >= 0 {
            draw_text_tag(out, fonts.text, &title, px + (p - w) / 2, ty);
            continue;
        }
        let lines = if w <= px {
            vec![title.clone()]
        } else {
            let words = title.split(' ').collect::<Vec<_>>();
            let cut = words.len().saturating_sub(2);
            vec![words[..cut].join(" "), words[cut..].join(" ")]
        };
        for (k, line) in lines.iter().filter(|line| !line.is_empty()).enumerate() {
            let (w, _) = text_tag_size(line);
            draw_text_tag(
                out,
                fonts.text,
                line,
                ((px - w) / 2).max(0),
                6 + 10 * k as i32,
            );
        }
    }
}

/// One level's panel: its 8 x 8 and, with `wrap` on, the level repeated
/// over the canvas's height (and beside a lone panel), then the edge.
#[allow(clippy::too_many_arguments)]
fn draw_dungeon_panel(
    shown: &DungeonShown,
    layout: &DungeonLayout,
    index: u8,
    level: u8,
    glyph: &mut dyn FnMut(u8) -> [u8; 64],
    scale: GlyphScale,
    luts: &DungeonLuts,
    wrap: bool,
    out: &mut GemImage,
) {
    let (px, py) = layout.panel_origin(index);
    let p = layout.panel_px() as i32;
    let c = layout.cell as i32;
    let side = if wrap { layout.wrap_side() as i32 } else { 0 };
    let (w, h) = (out.w as i32, out.h as i32);
    let xs = (px - side).max(0)..(px + p + side).min(w);
    let ys = if wrap {
        0..h
    } else {
        py.max(0)..(py + p).min(h)
    };
    let side_cells = DUNGEON_PANEL_CELLS as usize;
    for y in ys {
        let v = (y - py).rem_euclid(p);
        let (cy, j) = (v / c, (v % c) as u32);
        let row_core = (py..py + p).contains(&y);
        for x in xs.clone() {
            let u = (x - px).rem_euclid(p);
            let (cx, i) = (u / c, (u % c) as u32);
            let Some(cell) = shown.cell(level, cx as u8, cy as u8) else {
                continue;
            };
            let core = row_core && (px..px + p).contains(&x);
            let pixels = glyph(cell.drawn);
            let at = scale.source(j) as usize * side_cells + scale.source(i) as usize;
            let rgb = MAP_PALETTE[usize::from(pixels[at] & 0x0F)];
            out.put(x, y, luts.colour(rgb, cell.lit, core));
        }
    }
    if wrap {
        out.frame((px - 1, py - 1, px + p, py + p), 1, DUNGEON_EDGE_GREY);
    }
}

/// The yellow link boxes of one panel (10.2.6): a 1 px EGA 14 outline on
/// the outer pixels of each [`DungeonShown::linked`] cell, inside the cell.
fn draw_link_boxes(
    shown: &DungeonShown,
    layout: &DungeonLayout,
    index: u8,
    level: u8,
    out: &mut GemImage,
) {
    let (px, py) = layout.panel_origin(index);
    let c = layout.cell as i32;
    for (x, y, _) in shown.cells(level) {
        if !shown.linked(level, x, y) {
            continue;
        }
        let (x0, y0) = (px + i32::from(x) * c, py + i32::from(y) * c);
        out.frame((x0, y0, x0 + c - 1, y0 + c - 1), 1, LINK_YELLOW);
    }
}

/// The IBM.CH arrow for a facing: up, down, right, left (`0x18`..`0x1B`).
pub fn facing_arrow(facing: Direction) -> u8 {
    match facing {
        Direction::North | Direction::NorthWest | Direction::NorthEast => 0x18,
        Direction::South | Direction::SouthWest | Direction::SouthEast => 0x19,
        Direction::East => 0x1A,
        Direction::West => 0x1B,
    }
}

/// A text tag's size: 8 px a character and a 1 px outline round it.
pub fn text_tag_size(text: &str) -> (i32, i32) {
    (8 * text.chars().count() as i32 + 2, 10)
}

/// `text` in IBM.CH at `(x, y)`, EGA 15 with a 1 px black outline on all
/// eight sides of the ink (10.2.7 "Labels"); the rest of the tag shows the
/// map through.
fn draw_text_tag(out: &mut GemImage, font: &FixedCellFont, text: &str, x: i32, y: i32) {
    let (w, h) = text_tag_size(text);
    let mut ink = vec![false; (w * h) as usize];
    for (n, ch) in text.chars().enumerate() {
        let code = (ch as u32 & 0x7F) as u8;
        for row in 0..8 {
            let bits = font.glyph_row(code, row).unwrap_or(0);
            for col in 0..8 {
                if bits & (0x80 >> col) != 0 {
                    let (ix, iy) = (1 + 8 * n as i32 + col, 1 + row as i32);
                    ink[(iy * w + ix) as usize] = true;
                }
            }
        }
    }
    let inked = |ix: i32, iy: i32| {
        (0..w).contains(&ix) && (0..h).contains(&iy) && ink[(iy * w + ix) as usize]
    };
    for iy in 0..h {
        for ix in 0..w {
            if inked(ix, iy) {
                out.put(x + ix, y + iy, DUNGEON_TITLE_WHITE);
            } else if (-1..=1).any(|dy| (-1..=1).any(|dx| inked(ix + dx, iy + dy))) {
                out.put(x + ix, y + iy, [0, 0, 0]);
            }
        }
    }
}

#[cfg(test)]
#[path = "raster_tests.rs"]
mod tests;

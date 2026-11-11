//! The grow-left compositor, steps 1-4 (Phase B design 2.3.7; build plan
//! M1-09, M1-12): the canvas chrome, the map (the ring from the tile atlas and the
//! faithful core copied byte for byte), and the flashes on the ring.
//!
//! [`compose_grow_left`] is a pure function of the faithful frame, the
//! ring's raster, the canvas and the flash. Every pixel it places comes from
//! the faithful frame, the tile atlas (through the ring raster) or the flat
//! chrome geometry: blue fill (EGA 1), 1 px white rules (EGA 15) and the
//! real rounded outer corners, the staircase of
//! [`CHROME_CORNER_PROFILE`]. Later packages add the ribbon, the tabs, the
//! crawl and the column on top (steps 5-9).
//!
//! **Dissolves** (step 4) need nothing here: the faithful frame render
//! drains the pending map dissolves and presents their end state, and the
//! canvas copies that frame.

use u5_runtime::{
    CHROME_CORNER_PROFILE, PeekWindow, PotionFlashPlayback, TILE_ATLAS_SIDE, VIEWPORT_ORIGIN_X,
    VIEWPORT_ORIGIN_Y,
};

use super::bands::{BandSnapshot, paint_bands, paint_neutral_bands};
use bevy::math::UVec2;

use super::column::{COLUMN_RULE_X, ColumnText, ColumnView, paint_column};
use super::crawl_paint::{CrawlPaint, Painted, paint_crawl_painted};
use super::{CORE_SIDE, Canvas};
use crate::enh::present::MapRegion;
use crate::{EGA_PALETTE_RGB, VISUAL_PLAY_FRAME_HEIGHT, VISUAL_PLAY_FRAME_WIDTH};

/// The band blue, EGA 1.
pub const BAND_BLUE: u8 = 1;
/// The rules' white, EGA 15.
pub const RULE_WHITE: u8 = 15;

const FRAME_W: usize = VISUAL_PLAY_FRAME_WIDTH as usize;
const FRAME_H: usize = VISUAL_PLAY_FRAME_HEIGHT as usize;
const TILE: usize = TILE_ATLAS_SIDE;

fn rgba_of(index: u8) -> [u8; 4] {
    let [r, g, b] = EGA_PALETTE_RGB[usize::from(index)];
    [r, g, b, 0xff]
}

/// The cells the canvas's map window shows, from the party's cell: every
/// whole or partial cell between the party tile and each side of the map
/// window, capped at the peek's reach. 16:9 collapsed: 15, 15, 7, 7.
pub fn peek_window(canvas: &Canvas) -> PeekWindow {
    let (x0, y0, x1, y1) = canvas.map_window();
    let tile = TILE as u32;
    let reach = |pixels: u32| -> u8 {
        let cells = pixels.div_ceil(tile);
        cells.clamp(
            u32::from(u5_runtime::PEEK_CORE_REACH),
            u32::from(u5_runtime::PEEK_MAX_REACH),
        ) as u8
    };
    PeekWindow {
        west: reach(canvas.party_x - x0),
        east: reach(x1 + 1 - (canvas.party_x + tile)),
        north: reach(canvas.party_y - y0),
        south: reach(y1 + 1 - (canvas.party_y + tile)),
    }
}

/// The faithful core's rectangle on the canvas, `(x, y, side)`.
pub fn core_rect(canvas: &Canvas) -> (usize, usize, usize) {
    let (x, y) = canvas.core_origin();
    (x as usize, y as usize, CORE_SIDE as usize)
}

/// Step 1: the canvas chrome. Band blue everywhere, the black map window,
/// the 1 px white rule round it (x 7 and `W - 8`, y 15 and `H - 8`), and
/// the rounded outer corners at all four corners of the canvas (the
/// faithful frame's staircase, black outside it).
pub fn paint_chrome(rgba: &mut [u8], canvas: &Canvas) {
    let (w, h) = (canvas.w as usize, canvas.h as usize);
    let blue = rgba_of(BAND_BLUE);
    let white = rgba_of(RULE_WHITE);
    let black = rgba_of(0);
    for pixel in rgba.as_chunks_mut::<4>().0.iter_mut() {
        *pixel = blue;
    }
    let mut fill = |x0: usize, y0: usize, x1: usize, y1: usize, colour: [u8; 4]| {
        for y in y0..=y1 {
            for x in x0..=x1 {
                let at = (y * w + x) * 4;
                rgba[at..at + 4].copy_from_slice(&colour);
            }
        }
    };
    let (mx0, my0, mx1, my1) = canvas.map_window();
    let (mx0, my0, mx1, my1) = (mx0 as usize, my0 as usize, mx1 as usize, my1 as usize);
    // The rule, one pixel outside the map window on every side.
    fill(mx0 - 1, my0 - 1, mx1 + 1, my1 + 1, white);
    fill(mx0, my0, mx1, my1, black);
    // The corners: on row `r` in from an outer edge the fill starts at
    // column `CHROME_CORNER_PROFILE[r]`, mirrored at the far edge.
    for (r, first) in CHROME_CORNER_PROFILE.iter().enumerate() {
        let first = usize::from(*first);
        if first == 0 {
            continue;
        }
        for y in [r, h - 1 - r] {
            fill(0, y, first - 1, y, black);
            fill(w - first, y, w - 1, y, black);
        }
    }
}

/// Step 2, the ring: the ring raster (`window`'s cells, 16 pixels each, as
/// RGBA) placed with the party's cell on the canvas party tile and clipped
/// to the map window. Hidden cells are black in the raster.
pub fn paint_ring(rgba: &mut [u8], canvas: &Canvas, window: PeekWindow, ring: &[u8]) {
    let w = canvas.w as usize;
    let ring_w = window.columns() * TILE;
    let ring_h = window.rows() * TILE;
    assert_eq!(ring.len(), ring_w * ring_h * 4, "the ring raster's size");
    let origin_x = canvas.party_x as isize - (isize::from(window.west) * TILE as isize);
    let origin_y = canvas.party_y as isize - (isize::from(window.north) * TILE as isize);
    let (mx0, my0, mx1, my1) = canvas.map_window();
    for y in my0..=my1 {
        let ry = y as isize - origin_y;
        if ry < 0 || ry >= ring_h as isize {
            continue;
        }
        let rx0 = (mx0 as isize - origin_x).max(0);
        let rx1 = (mx1 as isize - origin_x).min(ring_w as isize - 1);
        if rx1 < rx0 {
            continue;
        }
        let from = (ry as usize * ring_w + rx0 as usize) * 4;
        let to = from + (rx1 - rx0 + 1) as usize * 4;
        let at = (y as usize * w + (origin_x + rx0) as usize) * 4;
        rgba[at..at + (to - from)].copy_from_slice(&ring[from..to]);
    }
}

/// Step 2, the core: the faithful viewport (frame x and y 8..183) copied
/// byte for byte to [`core_rect`].
pub fn copy_core(rgba: &mut [u8], canvas: &Canvas, faithful: &[u8]) {
    assert_eq!(faithful.len(), FRAME_W * FRAME_H * 4, "the faithful frame");
    let w = canvas.w as usize;
    let (cx, cy, side) = core_rect(canvas);
    for row in 0..side {
        let from = ((VIEWPORT_ORIGIN_Y + row) * FRAME_W + VIEWPORT_ORIGIN_X) * 4;
        let at = ((cy + row) * w + cx) * 4;
        rgba[at..at + side * 4].copy_from_slice(&faithful[from..from + side * 4]);
    }
}

/// Step 3: the flash on the ring only. The faithful core already holds the
/// inversion (the shell applies it to the frame's playfield rectangle), so
/// every map-window pixel outside the core gets the same palette XOR, with
/// the canvas's own stride. The core is never inverted twice.
pub fn flash_ring(rgba: &mut [u8], canvas: &Canvas, playback: PotionFlashPlayback) {
    let w = canvas.w as usize;
    let (mx0, my0, mx1, my1) = canvas.map_window();
    let (cx, cy, side) = core_rect(canvas);
    for y in my0 as usize..=my1 as usize {
        let in_core_rows = (cy..cy + side).contains(&y);
        for x in mx0 as usize..=mx1 as usize {
            if in_core_rows && (cx..cx + side).contains(&x) {
                continue;
            }
            let at = (y * w + x) * 4;
            let old = crate::ega_palette_index_from_rgba(&rgba[at..at + 4]);
            let rgb = EGA_PALETTE_RGB[usize::from(old ^ playback.palette_xor_mask)];
            rgba[at..at + 3].copy_from_slice(&rgb);
        }
    }
}

/// What steps 5 and 6 draw from: the snapshot taken when the faithful
/// frame last changed, the font and `display.ribbon_hp_colours`. Whether
/// the column is open comes from the compositor's [`ColumnView`].
#[derive(Clone, Copy)]
pub struct Bands<'a> {
    pub snapshot: &'a BandSnapshot,
    pub font: &'a u5_runtime::FixedCellFont,
    pub hp_colours: bool,
}

/// Steps 1-8 of the grow-left compositor (2.3.7): the canvas for
/// `canvas`, with the ring raster `ring` (RGBA of `window`'s cells) in the
/// map window, the faithful viewport of `faithful` (the 320x200 frame,
/// RGBA) copied to the core, and, while `flash` is set, the ring inverted
/// as the faithful frame inverted its core; then, with `bands`, the ribbon
/// on the top band and the sky and wind tabs at their shared centre; then,
/// with `crawl`, the message crawl over the map (step 7); then, while
/// `column` is shown (open, or a page is up), the docked column (step 8),
/// whose message window shows the crawl's rows in place of the crawl, or
/// in page mode the original's message window. The bands follow the
/// column: a shown column re-centres the tabs over the visible map. The
/// map is drawn where `canvas` puts the party; the `centre_party` pan is
/// the caller's ([`super::Canvas::for_column`]).
#[allow(clippy::too_many_arguments)]
pub fn compose_grow_left(
    faithful: &[u8],
    ring: &[u8],
    window: PeekWindow,
    canvas: &Canvas,
    flash: Option<PotionFlashPlayback>,
    bands: Option<Bands<'_>>,
    crawl: Option<CrawlPaint<'_>>,
    column: ColumnView,
) -> Vec<u8> {
    compose_grow_left_with(
        faithful,
        ring,
        window,
        canvas,
        flash,
        bands,
        crawl,
        column,
        BandMode::Follow,
    )
}

/// How step 5 and 6 draw the bands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BandMode {
    /// The ribbon and the tabs for the column's state.
    Follow,
    /// The open and close effect's neutral bands
    /// ([`paint_neutral_bands`]): no tabs, no moving ribbon members.
    Neutral,
}

/// [`compose_grow_left`] with the bands drawn as `mode` says.
#[allow(clippy::too_many_arguments)]
pub fn compose_grow_left_with(
    faithful: &[u8],
    ring: &[u8],
    window: PeekWindow,
    canvas: &Canvas,
    flash: Option<PotionFlashPlayback>,
    bands: Option<Bands<'_>>,
    crawl: Option<CrawlPaint<'_>>,
    column: ColumnView,
    mode: BandMode,
) -> Vec<u8> {
    compose_grow_left_mapped(
        faithful, ring, window, canvas, flash, bands, crawl, column, mode,
    )
    .0
}

/// The map region of a composed canvas (build plan D-M2.5): the map
/// window, less the docked column while it is `shown` (x up to its rule,
/// [`COLUMN_RULE_X`]), and less the crawl's glyphs, their outlines (the
/// eight neighbours clipped to the map window, as drawn) and its end cap,
/// from what the crawl `painted`. The ribbon and tabs are on the bands,
/// outside the map window.
pub fn view_map_region(canvas: &Canvas, shown: bool, painted: Option<&Painted>) -> MapRegion {
    let (x0, y0, x1, y1) = canvas.map_window();
    let left = if shown { COLUMN_RULE_X + 1 } else { x0 };
    let mut region = MapRegion::rect(UVec2::new(canvas.w, canvas.h), (left, y0, x1, y1));
    let Some(painted) = painted else {
        return region;
    };
    for ink in &painted.ink {
        for dy in -1i64..=1 {
            for dx in -1i64..=1 {
                let (x, y) = (i64::from(ink.x) + dx, i64::from(ink.y) + dy);
                if (i64::from(x0)..=i64::from(x1)).contains(&x)
                    && (i64::from(y0)..=i64::from(y1)).contains(&y)
                {
                    region.exclude(x as u32, y as u32);
                }
            }
        }
    }
    for &(x, y) in &painted.cap {
        region.exclude(x, y);
    }
    region
}

/// [`compose_grow_left_with`], with the canvas's map region
/// ([`view_map_region`]).
#[allow(clippy::too_many_arguments)]
pub fn compose_grow_left_mapped(
    faithful: &[u8],
    ring: &[u8],
    window: PeekWindow,
    canvas: &Canvas,
    flash: Option<PotionFlashPlayback>,
    bands: Option<Bands<'_>>,
    crawl: Option<CrawlPaint<'_>>,
    column: ColumnView,
    mode: BandMode,
) -> (Vec<u8>, MapRegion) {
    let mut rgba = vec![0; canvas.w as usize * canvas.h as usize * 4];
    paint_chrome(&mut rgba, canvas);
    paint_ring(&mut rgba, canvas, window, ring);
    if let Some(playback) = flash {
        flash_ring(&mut rgba, canvas, playback);
    }
    copy_core(&mut rgba, canvas, faithful);
    let shown = column.shown();
    if let (Some(bands), BandMode::Neutral) = (bands, mode) {
        paint_neutral_bands(
            &mut rgba,
            canvas,
            bands.snapshot,
            bands.font,
            bands.hp_colours,
        );
    } else if let Some(bands) = bands {
        paint_bands(
            &mut rgba,
            canvas,
            faithful,
            bands.snapshot,
            bands.font,
            bands.hp_colours,
            shown,
        );
    }
    if shown {
        let text = crawl.map(|crawl| ColumnText {
            rows: crawl.rows,
            typed_colour: crawl.typed_colour,
        });
        paint_column(&mut rgba, canvas, faithful, text, column.page_mode);
        (rgba, view_map_region(canvas, true, None))
    } else if let Some(crawl) = crawl {
        let painted = paint_crawl_painted(&mut rgba, canvas, crawl);
        let region = view_map_region(canvas, false, Some(&painted));
        (rgba, region)
    } else {
        (rgba, view_map_region(canvas, false, None))
    }
}

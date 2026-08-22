//! Gameplay-screen border chrome: the blue ribbon frame, its white
//! rules, the ribbon end-cap sprites, the top sun/moon sky strip, the
//! bottom wind banner, and the stats-divider timing-glyph slot.
//!
//! # Provenance
//!
//! `systems/intro.md §7` says only that the gameplay frame is "formed
//! by filled rectangles and box-drawing corner glyphs" and publishes
//! no coordinates, colours or glyph codes — unlike `§6.1`, which fully
//! specifies the analogous intro frame. Every rectangle, rule segment
//! and cell column in this module is therefore measured by black-box
//! observation of the original running from the shipped assets
//! (`u5-spec/capture/ultima_000.png` at native 320x200, cross-checked
//! against local DOSBox captures downsampled 6x->1x, and the dungeon
//! variant against the published `U5dgn` PC-EGA frame). Colours are
//! EGA palette *indices* read straight out of the indexed capture.
//!
//! The pending spec question covering this geometry — including the
//! end-cap sprite's provenance and the sky strip's glyph bank — is
//! `cleak/u5-spec#79`.
//!
//! # Art derivation
//!
//! No pixel art is copied. The ribbon end-cap is a two-colour 8x8
//! sprite whose *union* is exactly `IBM.CH` glyph `0x02` (right) /
//! `0x01` (left). Its blue component is that glyph eroded by one row
//! vertically (each row ANDed with its two neighbours, rows outside
//! the cell reading as zero) and its white component is the union XOR
//! the blue, so both halves are computed from the shipped font at
//! runtime — see [`ribbon_cap_sprite`]. The sky strip's moon phases
//! and hour marker are plain glyphs drawn out of the shipped
//! `RUNES.CH` alphabet at code points `0x30..=0x37` and `0x2A`.

use crate::clock::{SKY_STRIP_CELL_COUNT, SkyStripMarker, sky_strip_composed_cells};
use crate::constants::CH_CELL_SIDE;
use crate::graphics::{EGA_PALETTE_RGB, FixedCellFont};

/// EGA index of the border ribbon fill.
pub const CHROME_RIBBON_INDEX: u8 = 1;
/// EGA index of the 1px white rules and the chrome label text.
pub const CHROME_RULE_INDEX: u8 = 15;
/// EGA index the sky strip draws moon-phase glyphs in.
pub const SKY_STRIP_MOON_INDEX: u8 = 7;
/// EGA index the sky strip draws the fixed hour marker in.
pub const SKY_STRIP_HOUR_MARKER_INDEX: u8 = 14;

/// Top-left pixel of the 11x11 map viewport interior, immediately
/// inside the white frame rule at `x=7` / `y=7`.
pub const VIEWPORT_ORIGIN_X: usize = 8;
/// See [`VIEWPORT_ORIGIN_X`].
pub const VIEWPORT_ORIGIN_Y: usize = 8;

/// Last pixel row the chrome occupies. Text row 24 (pixel rows
/// `192..=199`) is left entirely black by the gameplay screen.
pub const CHROME_BOTTOM_Y: usize = 191;
/// Last pixel column of the left-hand (viewport) half of the frame,
/// i.e. the right edge of the middle ribbon band.
pub const CHROME_MIDDLE_BAND_RIGHT_X: usize = 190;

/// Rounded outer-corner profile shared by the original's two blue
/// chrome frames.
///
/// `CHROME_CORNER_PROFILE[r]` is the first column the fill occupies on
/// the row `r` pixel rows in from the band's outer edge; rows past the
/// profile start at column 0, and the far edge mirrors each entry.
///
/// The intro menu frame carves its corners with the same measured
/// staircase (`intro::INTRO_MENU_FRAME_CORNER_PROFILE`, published as
/// `cleak/u5-spec#78`), which re-exports this constant so the numbers
/// live in exactly one place.
pub const CHROME_CORNER_PROFILE: [u16; 6] = [5, 3, 2, 1, 1, 0];

/// `IBM.CH` glyph whose solid triangle is the union of the
/// right-pointing end cap's two colour masks.
pub const RIBBON_CAP_RIGHT_SOURCE_GLYPH: u8 = 0x02;
/// `IBM.CH` glyph for the left-pointing (mirrored) end cap.
pub const RIBBON_CAP_LEFT_SOURCE_GLYPH: u8 = 0x01;

/// `RUNES.CH` code point carrying the eight-point fixed hour marker.
pub const SKY_STRIP_HOUR_MARKER_RUNE: u8 = 0x2A;
/// `RUNES.CH` code point of moon phase `0`; phases `0..=7` occupy
/// `0x30..=0x37`, i.e. the phase digit's own ASCII byte.
pub const SKY_STRIP_MOON_PHASE_RUNE_BASE: u8 = 0x30;

/// Screen row hosting the twelve-cell sun/moon strip.
pub const SKY_STRIP_ROW: u8 = 0;
/// Column of sky strip cell 0; cell `i` sits at `6 + i`.
pub const SKY_STRIP_FIRST_COLUMN: u8 = 6;

/// Four-frame barber-pole cursor drawn in the cell after a live input
/// line's ribbon cap.
///
/// `IBM.CH` glyphs `0x05..=0x08` are one cycle of the same two-pixel
/// diagonal stripe, each frame advanced by one phase step, so playing
/// them in code order scrolls the stripe smoothly. Observation of the
/// shipped build confirms the cursor is drawn from this set and that it
/// animates rather than blinking: three captures of the gameplay input
/// line show frame `0x06` and a fourth shows `0x07`. The intro menu's
/// `Select:` caption cursor is the same set (`intro` pins frame index
/// 3). See the module header for the pending spec question.
pub const PROMPT_CURSOR_FRAME_GLYPHS: [u8; 4] = [0x05, 0x06, 0x07, 0x08];

/// Barber-pole cursor glyph for an animation frame counter.
pub fn prompt_cursor_glyph(frame: u64) -> u8 {
    let count = PROMPT_CURSOR_FRAME_GLYPHS.len() as u64;
    PROMPT_CURSOR_FRAME_GLYPHS[(frame % count) as usize]
}

/// Screen row hosting the prevailing-wind banner.
pub const WIND_BANNER_ROW: u8 = 23;
/// First column of the eleven-cell wind banner field.
pub const WIND_BANNER_FIRST_COLUMN: u8 = 7;
/// Width in cells of the wind banner field.
pub const WIND_BANNER_CELLS: usize = 11;
/// Width in cells of the banner's left-aligned direction field.
pub const WIND_BANNER_DIRECTION_CELLS: usize = 5;
/// Shared suffix printed at columns 13..=17.
pub const WIND_BANNER_SUFFIX: &str = "Winds";

/// Row of the first stats-panel divider band, which hosts the
/// timing/status glyph slot.
pub const TIMING_GLYPH_ROW: u8 = 7;
/// Column of the timing/status glyph, flanked by end caps at 30 and 32.
pub const TIMING_GLYPH_COLUMN: u8 = 31;
/// Row of the second (always plain) stats-panel divider band.
pub const LOWER_DIVIDER_ROW: u8 = 10;

/// Message/command window interior, in cells. The box has only a left
/// rule at `x=191` and a top rule at `y=87`; it runs to the screen edge.
pub const MESSAGE_WINDOW_LEFT: u8 = 24;
/// See [`MESSAGE_WINDOW_LEFT`].
pub const MESSAGE_WINDOW_TOP: u8 = 11;
/// See [`MESSAGE_WINDOW_LEFT`].
pub const MESSAGE_WINDOW_RIGHT: u8 = 39;
/// See [`MESSAGE_WINDOW_LEFT`]. Row 24 is never written.
pub const MESSAGE_WINDOW_BOTTOM: u8 = 23;

/// Roster box interior, in cells.
pub const STATS_ROSTER_TOP: u8 = 1;
/// See [`STATS_ROSTER_TOP`].
pub const STATS_ROSTER_BOTTOM: u8 = 6;
/// Food/gold/date box interior, in cells.
pub const STATS_COUNTER_TOP: u8 = 8;
/// See [`STATS_COUNTER_TOP`].
pub const STATS_COUNTER_BOTTOM: u8 = 9;

/// Inclusive pixel rectangles filled with [`CHROME_RIBBON_INDEX`]
/// before the rounded outer corners are carved back.
const RIBBON_BANDS: [(usize, usize, usize, usize); 7] = [
    // Top band, stopping at the stats panel's right rule.
    (0, 0, 312, 6),
    // Left band.
    (0, 0, 6, 191),
    // Middle band, between the viewport and the stats panel.
    (185, 0, 190, 191),
    // Right band. Absent below y=86: the message box runs to the edge.
    (313, 0, 319, 86),
    // Bottom band, stopping at the middle band's right edge.
    (0, 185, 190, 191),
    // Stats divider bands, which merge the middle and right bands.
    (191, 57, 312, 62),
    (191, 81, 312, 86),
];

/// Inclusive pixel rectangles of the 1px white rules that are never
/// interrupted by a ribbon gap.
const FIXED_RULES: [(usize, usize, usize, usize); 9] = [
    (191, 7, 312, 7),   // top of the roster box
    (191, 87, 319, 87), // top of the message box
    (7, 7, 7, 184),     // viewport left edge
    (184, 7, 184, 184), // viewport right edge
    (191, 7, 191, 56),  // stats panel left edge, above divider row 7
    (191, 63, 191, 80),
    (191, 87, 191, 191),
    (312, 7, 312, 56), // stats panel right edge
    (312, 63, 312, 80),
];

/// Horizontal rules bounding the two stats divider bands. The first
/// pair may be interrupted by the timing-glyph slot.
const DIVIDER_RULE_ROWS: [usize; 4] = [56, 63, 80, 87];

/// Which pointing direction a ribbon end cap has. The ribbon always
/// points *into* the gap it terminates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonCapDirection {
    /// Drawn on the left side of a gap.
    Right,
    /// Drawn on the right side of a gap.
    Left,
}

impl RibbonCapDirection {
    /// `IBM.CH` glyph whose solid triangle is the union of this cap's
    /// two colour masks.
    pub const fn source_glyph(self) -> u8 {
        match self {
            Self::Right => RIBBON_CAP_RIGHT_SOURCE_GLYPH,
            Self::Left => RIBBON_CAP_LEFT_SOURCE_GLYPH,
        }
    }
}

/// The two 8x8 colour masks of a ribbon end-cap sprite.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RibbonCapSprite {
    /// Rows drawn in [`CHROME_RULE_INDEX`].
    pub white: [u8; CH_CELL_SIDE],
    /// Rows drawn in [`CHROME_RIBBON_INDEX`].
    pub ribbon: [u8; CH_CELL_SIDE],
}

/// One painted cell of the sky strip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyStripCell {
    /// `RUNES.CH` code point to draw.
    pub rune_code: u8,
    /// EGA index to draw it in.
    pub palette_index: u8,
}

/// What occupies the gap punched through a ribbon band.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum ChromeGap {
    /// No gap: the ribbon and its rule run unbroken.
    #[default]
    Unbroken,
    /// Twelve sun/moon cells drawn from `RUNES.CH`.
    SkyStrip(Box<[Option<SkyStripCell>; SKY_STRIP_CELL_COUNT as usize]>),
    /// Plain white label drawn from `IBM.CH`, centred in the band's
    /// nominal gap with the end caps closed up around it.
    Label(String),
}

/// Everything the chrome pass needs beyond fixed geometry.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct GameplayChromeContent {
    /// Row 0 gap: sky strip on the surface/town family, dungeon level
    /// label in dungeon-class scenes.
    pub top: ChromeGap,
    /// Row 23 gap: wind banner on the surface/town family, dungeon
    /// facing label in dungeon-class scenes.
    pub bottom: ChromeGap,
    /// Timing/status tag byte. `None` (or zero) leaves divider row 7 a
    /// plain ribbon band with no caps and no placeholder.
    pub timing_glyph: Option<u8>,
}

/// The two fixed-cell fonts the chrome pass draws from.
#[derive(Clone, Copy)]
pub struct ChromeFonts<'a> {
    /// `IBM.CH`: end-cap source triangles and label text.
    pub ibm: &'a FixedCellFont,
    /// `RUNES.CH`: sky-strip moon phases and hour marker.
    pub runes: &'a FixedCellFont,
}

/// Derive the ribbon end-cap sprite from the shipped `IBM.CH` font.
///
/// The blue component is the source triangle eroded by one row
/// vertically; the white outline is the triangle minus that erosion.
/// See the module header for provenance.
pub fn ribbon_cap_sprite(font: &FixedCellFont, direction: RibbonCapDirection) -> RibbonCapSprite {
    let code = direction.source_glyph();
    let mut solid = [0u8; CH_CELL_SIDE];
    for (row, bits) in solid.iter_mut().enumerate() {
        *bits = font.glyph_row(code, row).unwrap_or_else(|| {
            panic!("IBM.CH glyph {code:#04x} row {row} missing; ribbon end cap needs it")
        });
    }
    let mut ribbon = [0u8; CH_CELL_SIDE];
    for row in 0..CH_CELL_SIDE {
        let above = if row == 0 { 0 } else { solid[row - 1] };
        let below = if row + 1 == CH_CELL_SIDE {
            0
        } else {
            solid[row + 1]
        };
        ribbon[row] = above & solid[row] & below;
    }
    let mut white = [0u8; CH_CELL_SIDE];
    for row in 0..CH_CELL_SIDE {
        white[row] = solid[row] ^ ribbon[row];
    }
    RibbonCapSprite { white, ribbon }
}

/// Screen column of sky strip cell `index`.
pub const fn sky_strip_cell_column(index: usize) -> u8 {
    SKY_STRIP_FIRST_COLUMN + index as u8
}

/// `RUNES.CH` code point for a moon-phase glyph byte. The phase digits
/// `b'0'..=b'7'` index the rune alphabet directly.
pub const fn sky_strip_moon_rune(phase_byte: u8) -> Option<u8> {
    if phase_byte >= SKY_STRIP_MOON_PHASE_RUNE_BASE
        && phase_byte <= SKY_STRIP_MOON_PHASE_RUNE_BASE + 7
    {
        Some(phase_byte)
    } else {
        None
    }
}

/// Compose the twelve sky-strip cells for an hour and the two cached
/// moon-phase glyph bytes. Cell positions and phase bytes come
/// unchanged from `moons.md §2`; only the glyph bank and the screen
/// placement are observation-derived.
pub fn sky_strip_cells(
    hour: u8,
    cached_moon_glyph_bytes: [u8; 2],
) -> [Option<SkyStripCell>; SKY_STRIP_CELL_COUNT as usize] {
    let mut cells = [None; SKY_STRIP_CELL_COUNT as usize];
    for (index, marker) in sky_strip_composed_cells(hour).into_iter().enumerate() {
        cells[index] =
            match marker {
                None => None,
                Some(SkyStripMarker::FixedHour) => Some(SkyStripCell {
                    rune_code: SKY_STRIP_HOUR_MARKER_RUNE,
                    palette_index: SKY_STRIP_HOUR_MARKER_INDEX,
                }),
                Some(SkyStripMarker::Trammel) => sky_strip_moon_rune(cached_moon_glyph_bytes[0])
                    .map(|rune_code| SkyStripCell {
                        rune_code,
                        palette_index: SKY_STRIP_MOON_INDEX,
                    }),
                Some(SkyStripMarker::Felucca) => sky_strip_moon_rune(cached_moon_glyph_bytes[1])
                    .map(|rune_code| SkyStripCell {
                        rune_code,
                        palette_index: SKY_STRIP_MOON_INDEX,
                    }),
            };
    }
    cells
}

/// Format the eleven-cell wind banner: the direction name left-aligned
/// in a five-column field, one space, then the shared `Winds` suffix.
/// `None` (an out-of-range saved wind byte) prints the suffix alone,
/// per `weather.md §2`.
pub fn wind_banner_text(direction_name: Option<&str>) -> String {
    let direction = direction_name.unwrap_or("");
    let direction: String = direction
        .chars()
        .take(WIND_BANNER_DIRECTION_CELLS)
        .collect();
    format!("{direction:<WIND_BANNER_DIRECTION_CELLS$} {WIND_BANNER_SUFFIX}")
}

/// Dungeon-class replacement for the sky strip: the current level.
pub fn dungeon_level_label(level: u8) -> String {
    format!("L{level}")
}

/// Dungeon-class replacement for the wind banner: the party facing.
pub fn dungeon_direction_label(direction_name: &str) -> String {
    format!("{:<6}{direction_name}", "Dir:")
}

/// Nominal gap for a ribbon band: the cell columns the caps and their
/// content occupy when the gap is at its widest. A shorter label is
/// centred inside this span and the caps close up around it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GapSpan {
    content_first_column: u8,
    content_cells: usize,
    band_row: u8,
}

const TOP_GAP: GapSpan = GapSpan {
    content_first_column: SKY_STRIP_FIRST_COLUMN,
    content_cells: SKY_STRIP_CELL_COUNT as usize,
    band_row: SKY_STRIP_ROW,
};

const BOTTOM_GAP: GapSpan = GapSpan {
    content_first_column: WIND_BANNER_FIRST_COLUMN,
    content_cells: WIND_BANNER_CELLS,
    band_row: WIND_BANNER_ROW,
};

const TIMING_GAP: GapSpan = GapSpan {
    content_first_column: TIMING_GLYPH_COLUMN,
    content_cells: 1,
    band_row: TIMING_GLYPH_ROW,
};

/// A resolved gap: the cap columns and where content starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResolvedGap {
    /// Column holding the right-pointing cap, on the left of the gap.
    pub left_cap_column: u8,
    /// Column holding the left-pointing cap, on the right of the gap.
    pub right_cap_column: u8,
    /// First content column, immediately right of the left cap.
    pub content_first_column: u8,
    /// Number of content cells between the caps.
    pub content_cells: usize,
}

impl ResolvedGap {
    /// First pixel column the gap blacks out of the ribbon.
    pub const fn first_pixel_x(&self) -> usize {
        self.left_cap_column as usize * CH_CELL_SIDE
    }

    /// Last pixel column the gap blacks out of the ribbon.
    pub const fn last_pixel_x(&self) -> usize {
        self.right_cap_column as usize * CH_CELL_SIDE + CH_CELL_SIDE - 1
    }
}

/// Centre `cells` content columns inside `span` and place the caps
/// immediately outside them.
fn resolve_gap(span: GapSpan, cells: usize) -> ResolvedGap {
    let cells = cells.min(span.content_cells);
    let offset = (span.content_cells - cells) / 2;
    let content_first_column = span.content_first_column + offset as u8;
    ResolvedGap {
        left_cap_column: content_first_column - 1,
        right_cap_column: content_first_column + cells as u8,
        content_first_column,
        content_cells: cells,
    }
}

/// Resolve the row-0 gap for the given content, if any.
pub fn top_gap(content: &ChromeGap) -> Option<ResolvedGap> {
    gap_for(TOP_GAP, content)
}

/// Resolve the row-23 gap for the given content, if any.
pub fn bottom_gap(content: &ChromeGap) -> Option<ResolvedGap> {
    gap_for(BOTTOM_GAP, content)
}

/// Resolve the divider row-7 timing-glyph gap, if the tag byte is
/// nonzero. A zero byte leaves the band plain — no caps, no
/// placeholder.
pub fn timing_glyph_gap(timing_glyph: Option<u8>) -> Option<ResolvedGap> {
    timing_glyph
        .filter(|byte| *byte != 0)
        .map(|_| resolve_gap(TIMING_GAP, 1))
}

fn gap_for(span: GapSpan, content: &ChromeGap) -> Option<ResolvedGap> {
    match content {
        ChromeGap::Unbroken => None,
        ChromeGap::SkyStrip(_) => Some(resolve_gap(span, SKY_STRIP_CELL_COUNT as usize)),
        ChromeGap::Label(label) => {
            let cells = label.chars().count();
            if cells == 0 {
                None
            } else {
                Some(resolve_gap(span, cells))
            }
        }
    }
}

/// Paint the gameplay border chrome into a 320x200 RGBA buffer.
///
/// Runs before the viewport blit and before the text-window surface is
/// composited: the viewport interior and every text box interior are
/// black here, so a later non-black overlay leaves the chrome intact.
pub fn paint_gameplay_frame_chrome(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    content: &GameplayChromeContent,
    fonts: ChromeFonts<'_>,
) {
    let top = top_gap(&content.top);
    let bottom = bottom_gap(&content.bottom);
    let timing = timing_glyph_gap(content.timing_glyph);

    for (x0, y0, x1, y1) in RIBBON_BANDS {
        fill_rect(rgba, width, height, x0, y0, x1, y1, CHROME_RIBBON_INDEX);
    }
    carve_rounded_corners(rgba, width, height);

    for (gap, band_row) in [
        (top, SKY_STRIP_ROW),
        (bottom, WIND_BANNER_ROW),
        (timing, TIMING_GLYPH_ROW),
    ] {
        let Some(gap) = gap else { continue };
        let y0 = band_row as usize * CH_CELL_SIDE;
        fill_rect(
            rgba,
            width,
            height,
            gap.first_pixel_x(),
            y0,
            gap.last_pixel_x(),
            y0 + CH_CELL_SIDE - 1,
            0,
        );
    }

    for (x0, y0, x1, y1) in FIXED_RULES {
        fill_rect(rgba, width, height, x0, y0, x1, y1, CHROME_RULE_INDEX);
    }
    paint_interrupted_rule(rgba, width, height, 7, 7, 184, top);
    paint_interrupted_rule(rgba, width, height, 184, 7, 184, bottom);
    for row_y in DIVIDER_RULE_ROWS {
        let gap = if row_y == 56 || row_y == 63 {
            timing
        } else {
            None
        };
        paint_interrupted_rule(rgba, width, height, row_y, 191, 312, gap);
    }

    if let Some(gap) = top {
        paint_gap_caps(rgba, width, height, fonts.ibm, gap, SKY_STRIP_ROW);
        paint_gap_content(rgba, width, height, fonts, gap, SKY_STRIP_ROW, &content.top);
    }
    if let Some(gap) = bottom {
        paint_gap_caps(rgba, width, height, fonts.ibm, gap, WIND_BANNER_ROW);
        paint_gap_content(
            rgba,
            width,
            height,
            fonts,
            gap,
            WIND_BANNER_ROW,
            &content.bottom,
        );
    }
    if let (Some(gap), Some(byte)) = (timing, content.timing_glyph) {
        paint_gap_caps(rgba, width, height, fonts.ibm, gap, TIMING_GLYPH_ROW);
        draw_glyph(
            rgba,
            width,
            height,
            fonts.ibm,
            byte & 0x7f,
            gap.content_first_column,
            TIMING_GLYPH_ROW,
            CHROME_RULE_INDEX,
        );
    }
}

/// Paint one ribbon end-cap sprite at an absolute cell.
///
/// This is the build's single two-colour bracket primitive. The same
/// sprite terminates every interrupted ribbon, prefixes every echoed
/// command line, and forms the `>`/`<` brackets around the intro
/// menu's border captions - all four uses were measured byte-for-byte
/// identical in the shipped build.
pub fn paint_ribbon_cap(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    direction: RibbonCapDirection,
    column: u8,
    row: u8,
) {
    draw_ribbon_cap(rgba, width, height, font, direction, column, row);
}

/// Paint the message window's per-line right-pointing cap prefix.
pub fn paint_message_line_cap(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    row: u8,
) {
    paint_ribbon_cap(
        rgba,
        width,
        height,
        font,
        RibbonCapDirection::Right,
        MESSAGE_WINDOW_LEFT,
        row,
    );
}

fn carve_rounded_corners(rgba: &mut [u8], width: usize, height: usize) {
    for (row_from_edge, start_column) in CHROME_CORNER_PROFILE.into_iter().enumerate() {
        let start_column = usize::from(start_column);
        if start_column == 0 {
            continue;
        }
        let top = row_from_edge;
        let bottom = CHROME_BOTTOM_Y - row_from_edge;
        // Top-left and bottom-left carves.
        fill_rect(rgba, width, height, 0, top, start_column - 1, top, 0);
        fill_rect(rgba, width, height, 0, bottom, start_column - 1, bottom, 0);
        // Top-right carve. The right band ends at y=86, so it has no
        // bottom-right corner to round.
        fill_rect(rgba, width, height, 320 - start_column, top, 319, top, 0);
    }
}

fn paint_interrupted_rule(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    row_y: usize,
    x0: usize,
    x1: usize,
    gap: Option<ResolvedGap>,
) {
    match gap {
        None => fill_rect(rgba, width, height, x0, row_y, x1, row_y, CHROME_RULE_INDEX),
        Some(gap) => {
            let left_end = gap.left_cap_column as usize * CH_CELL_SIDE;
            let right_start = gap.right_cap_column as usize * CH_CELL_SIDE + CH_CELL_SIDE - 1;
            if x0 <= left_end {
                fill_rect(
                    rgba,
                    width,
                    height,
                    x0,
                    row_y,
                    left_end.min(x1),
                    row_y,
                    CHROME_RULE_INDEX,
                );
            }
            if right_start <= x1 {
                fill_rect(
                    rgba,
                    width,
                    height,
                    right_start.max(x0),
                    row_y,
                    x1,
                    row_y,
                    CHROME_RULE_INDEX,
                );
            }
        }
    }
}

fn paint_gap_caps(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    gap: ResolvedGap,
    row: u8,
) {
    draw_ribbon_cap(
        rgba,
        width,
        height,
        font,
        RibbonCapDirection::Right,
        gap.left_cap_column,
        row,
    );
    draw_ribbon_cap(
        rgba,
        width,
        height,
        font,
        RibbonCapDirection::Left,
        gap.right_cap_column,
        row,
    );
}

fn paint_gap_content(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    fonts: ChromeFonts<'_>,
    gap: ResolvedGap,
    row: u8,
    content: &ChromeGap,
) {
    match content {
        ChromeGap::Unbroken => {}
        ChromeGap::SkyStrip(cells) => {
            for (index, cell) in cells.iter().enumerate() {
                let Some(cell) = cell else { continue };
                draw_glyph(
                    rgba,
                    width,
                    height,
                    fonts.runes,
                    cell.rune_code,
                    gap.content_first_column + index as u8,
                    row,
                    cell.palette_index,
                );
            }
        }
        ChromeGap::Label(label) => {
            for (index, ch) in label.chars().take(gap.content_cells).enumerate() {
                draw_glyph(
                    rgba,
                    width,
                    height,
                    fonts.ibm,
                    (ch as u32 as u8) & 0x7f,
                    gap.content_first_column + index as u8,
                    row,
                    CHROME_RULE_INDEX,
                );
            }
        }
    }
}

fn draw_ribbon_cap(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    direction: RibbonCapDirection,
    column: u8,
    row: u8,
) {
    let sprite = ribbon_cap_sprite(font, direction);
    draw_mask(
        rgba,
        width,
        height,
        &sprite.ribbon,
        column,
        row,
        CHROME_RIBBON_INDEX,
    );
    draw_mask(
        rgba,
        width,
        height,
        &sprite.white,
        column,
        row,
        CHROME_RULE_INDEX,
    );
}

fn draw_glyph(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    code: u8,
    column: u8,
    row: u8,
    palette_index: u8,
) {
    let mut mask = [0u8; CH_CELL_SIDE];
    for (glyph_row, bits) in mask.iter_mut().enumerate() {
        *bits = font
            .glyph_row(code, glyph_row)
            .unwrap_or_else(|| panic!("fixed-cell glyph {code:#04x} row {glyph_row} missing"));
    }
    draw_mask(rgba, width, height, &mask, column, row, palette_index);
}

/// Draw an 8x8 1-bit mask with a transparent background: only set bits
/// are painted, so caps and glyphs compose over whatever is beneath.
fn draw_mask(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    mask: &[u8; CH_CELL_SIDE],
    column: u8,
    row: u8,
    palette_index: u8,
) {
    let base_x = column as usize * CH_CELL_SIDE;
    let base_y = row as usize * CH_CELL_SIDE;
    let rgb = EGA_PALETTE_RGB[usize::from(palette_index & 0x0f)];
    for (glyph_y, bits) in mask.iter().enumerate() {
        let y = base_y + glyph_y;
        if y >= height {
            break;
        }
        for glyph_x in 0..CH_CELL_SIDE {
            if bits & (1 << (7 - glyph_x)) == 0 {
                continue;
            }
            let x = base_x + glyph_x;
            if x >= width {
                continue;
            }
            let offset = (y * width + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
    }
}

fn fill_rect(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    palette_index: u8,
) {
    debug_assert!(x0 <= x1 && y0 <= y1, "chrome rect is inverted");
    let rgb = EGA_PALETTE_RGB[usize::from(palette_index & 0x0f)];
    let pixel = [rgb[0], rgb[1], rgb[2], 0xff];
    for y in y0..=y1.min(height.saturating_sub(1)) {
        for x in x0..=x1.min(width.saturating_sub(1)) {
            let offset = (y * width + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
}

/// Derive the chrome's variable content from live play state.
///
/// Surface and town-family scenes carry the twelve-cell sky strip and
/// the prevailing-wind banner. Dungeon-class scenes replace them with
/// the level label and the party facing (`weather.md §Autonomous Wind
/// Drift`: the wind helper does not run in dungeon-class scenes). The
/// Underworld has no published chrome content for either gap, so both
/// ribbons stay unbroken there rather than inventing one.
pub fn gameplay_chrome_content(state: &crate::PlayState) -> GameplayChromeContent {
    let timing = state.timing_status.save_byte();
    let timing_glyph = (timing != 0).then_some(timing);
    match state.area {
        crate::Area::Dungeon { level, .. } => GameplayChromeContent {
            top: ChromeGap::Label(dungeon_level_label(level)),
            bottom: ChromeGap::Label(dungeon_direction_label(state.player.facing.name())),
            timing_glyph,
        },
        crate::Area::World { plane } if plane == crate::WorldPlane::Underworld => {
            GameplayChromeContent {
                top: ChromeGap::Unbroken,
                bottom: ChromeGap::Unbroken,
                timing_glyph,
            }
        }
        crate::Area::World { .. } | crate::Area::Town { .. } => GameplayChromeContent {
            top: ChromeGap::SkyStrip(Box::new(sky_strip_cells(
                state.clock.hour,
                state.cached_moon_glyph_bytes,
            ))),
            bottom: ChromeGap::Label(wind_banner_text(wind_banner_direction_name(state))),
            timing_glyph,
        },
    }
}

/// `weather.md §2`: an out-of-range saved wind byte prints no direction
/// label but still prints the shared suffix.
fn wind_banner_direction_name(state: &crate::PlayState) -> Option<&'static str> {
    match state.wind_save_byte {
        0..=4 => Some(state.wind.name()),
        _ => None,
    }
}

/// Paint one fixed-cell glyph with a transparent background at an
/// absolute cell position. Used by the message window, which places
/// rows at exact screen cells rather than through a text window.
pub fn paint_fixed_cell_glyph(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    code: u8,
    column: u8,
    row: u8,
    palette_index: u8,
) {
    draw_glyph(rgba, width, height, font, code, column, row, palette_index);
}

/// Paint a run of fixed-cell glyphs starting at an absolute cell.
/// Characters that would run past column 39 are dropped.
pub fn paint_fixed_cell_text(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    text: &str,
    column: u8,
    row: u8,
    palette_index: u8,
) {
    for (offset, ch) in text.chars().enumerate() {
        let Ok(offset) = u8::try_from(offset) else {
            break;
        };
        let Some(cell_column) = column.checked_add(offset) else {
            break;
        };
        if cell_column > MESSAGE_WINDOW_RIGHT {
            break;
        }
        draw_glyph(
            rgba,
            width,
            height,
            font,
            (ch as u32 as u8) & 0x7f,
            cell_column,
            row,
            palette_index,
        );
    }
}

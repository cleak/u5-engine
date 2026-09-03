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
//! `cleak/u5-spec#79` is now **answered and closed** against spec head
//! `3bbcd5e`, and every rectangle, rule run, cell column and colour
//! measured here was confirmed against the shipped executable. What the
//! answer added, and what this module now implements, is the published
//! *mechanism* rather than the observed result: a three-phase paint
//! (`systems/display-driver.md` section 7) of filled rectangles, then
//! three reserved corner glyphs emitted opaquely so their clear bits
//! carve the bevel, then four accent polylines. The two colours are
//! user-interface colour-table slots — chrome and accent — which are
//! EGA 1 and 15 in this family.
//!
//! # Art derivation
//!
//! No pixel art is copied. The ribbon end cap is not stored art at all
//! — which is why byte-scanning the shipped files for either colour
//! component always failed. It is composited at draw time in two
//! passes: emit the font's opaque solid-triangle glyph (`IBM.CH`
//! `0x02` right, `0x01` left) in the chrome colour on black, then
//! stroke two straight lines in the accent colour along the triangle's
//! hypotenuse. The strokes terminate on the cell's outer column at its
//! top and bottom rows — the same rows the ribbon rules occupy — which
//! is what makes an adjoining rule appear to run into the cap. See
//! [`ribbon_cap_sprite`].
//!
//! The same composite is the build's single bracket primitive: every
//! ribbon interruption, the border captions, and the message window's
//! line prompt all use it. The line prompt draws the right-pointing
//! cap *alone*, with no closing cap.
//!
//! The rounded outer corners are likewise font glyphs, not arithmetic:
//! `IBM.CH` `0x7B`, `0x7C` and `0x7D` emitted opaquely after the fills.
//! There is deliberately no bottom-right corner glyph — the font has
//! one, the frame does not use it, because that corner is the message
//! window.
//!
//! The sky strip's moon phases and hour marker are plain glyphs drawn
//! out of the shipped `RUNES.CH` alphabet: the renderer switches the
//! active font slot for the duration of the strip and restores the main
//! font afterwards, emitting the ASCII bytes `'0'..='7'` and `'*'`,
//! which in that alphabet hold moon art and an eight-point starburst.

use crate::clock::{SKY_STRIP_CELL_COUNT, SkyStripMarker, sky_strip_composed_cells};
use crate::constants::CH_CELL_SIDE;
use crate::graphics::{EGA_PALETTE_RGB, FixedCellFont};

// `display-driver.md §2` publishes the driver-family UI colour table.
// The v1 EGA baseline and Tandy both use its high-colour values; CGA and
// Hercules use the separately published low-colour values and remain outside
// the v1 alternate-hardware parity boundary.

/// High-colour UI-table slot 2: border ribbon fill ("chrome").
pub const CHROME_RIBBON_INDEX: u8 = 1;
/// High-colour UI-table slot 1: rules and chrome label text ("accent").
pub const CHROME_RULE_INDEX: u8 = 15;
/// High-colour UI-table slot 6 used for sky-strip moon-phase glyphs.
pub const SKY_STRIP_MOON_INDEX: u8 = 7;
/// High-colour UI-table slot 5 used for the fixed hour marker.
pub const SKY_STRIP_HOUR_MARKER_INDEX: u8 = 14;

/// The two colour-table slots the chrome paint uses, plus the
/// background it clears to.
///
/// The published paint is specified in terms of "the chrome colour"
/// and "the accent colour" rather than raw indices, and it reads no
/// gameplay state - only these two values. Passing them in keeps the
/// bracket primitive reusable by the three call sites that consume it
/// (the gameplay border, the intro menu's border captions and the
/// message window's line prompt), each of which supplies its own pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChromePalette {
    /// Ribbon fill and the solid pass of a bracket end cap.
    pub chrome: u8,
    /// Rules, label text and the stroked pass of a bracket end cap.
    pub accent: u8,
    /// Cleared background behind the frame.
    pub background: u8,
}

impl ChromePalette {
    /// Published EGA/Tandy high-colour-family values.
    pub const EGA: Self = Self {
        chrome: CHROME_RIBBON_INDEX,
        accent: CHROME_RULE_INDEX,
        background: 0,
    };
}

impl Default for ChromePalette {
    fn default() -> Self {
        Self::EGA
    }
}

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
/// The intro menu frame carves its corners with the same bevel
/// (`intro::INTRO_MENU_FRAME_CORNER_PROFILE`, published as
/// `cleak/u5-spec#78`), which re-exports this constant so the numbers
/// live in exactly one place. The gameplay frame no longer reads it:
/// per `cleak/u5-spec#79` that bevel is carved by stamping the font's
/// own corner glyphs (see [`CHROME_CORNER_GLYPHS`]), and this row-wise
/// form is the same staircase written out.
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
/// 3). This is published in `text-output.md §10.6`.
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
pub const WIND_BANNER_SUFFIX: &str = " Winds";

/// Row of the first stats-panel divider band, which hosts the
/// timing/status glyph slot.
pub const TIMING_GLYPH_ROW: u8 = 7;
/// Column of the timing/status glyph, flanked by end caps at 30 and 32.
pub const TIMING_GLYPH_COLUMN: u8 = 31;
/// Row of the second (always plain) stats-panel divider band.
pub const LOWER_DIVIDER_ROW: u8 = 10;

/// Anchor column of the stats-window label strip's cap formula
/// (`stats-panel.md` section 9). This is the *cap* anchor, not the
/// caption's centre: the centre of the fifteen-cell field is column 31.
pub const STATS_LABEL_STRIP_CAP_ANCHOR: u8 = 30;
/// Row of the stats-window label strip.
pub const STATS_LABEL_STRIP_ROW: u8 = 0;

/// Cell span of the stats-window label strip for a label of `length`
/// characters: opening cap column, first text column, closing cap
/// column.
///
/// The published rule is an opening cap at `30 - (length / 2)` with
/// integer division, the label in the next `length` columns, then the
/// closing cap. Seven-character `Select:` gives caps at 27 and 35 with
/// text at 28..=34, centred on 31; an even-length label straddles 30
/// and 31 because the division truncates. This is the one genuinely
/// centred label on the screen - everything else that looks centred is
/// fixed-column with fixed-width content.
pub fn stats_label_strip_span(length: usize) -> (u8, u8, u8) {
    let opening = STATS_LABEL_STRIP_CAP_ANCHOR.saturating_sub((length / 2) as u8);
    let first_text = opening + 1;
    let closing = first_text + length as u8;
    (opening, first_text, closing)
}

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

/// Phase 1 of the published paint: the seven chrome fills, in order,
/// as inclusive pixel rectangles (`display-driver.md` section 7).
///
/// These are the *filled* extents, which are not the visible ones: the
/// accent polylines of phase 3 overpaint a row or column of several of
/// them. The right band is filled to `y=87` and the message window's
/// top rule then covers its last row, which is why it looks like it
/// stops at 86. Implementing the visible extents instead gets the
/// label gaps wrong, so the raw list is what is reproduced here.
const RIBBON_BANDS: [(usize, usize, usize, usize); 7] = [
    (0, 0, 319, 6),
    (0, 185, 191, 191),
    (0, 0, 6, 191),
    (185, 0, 191, 191),
    (313, 0, 319, 87),
    (192, 80, 312, 87),
    (192, 57, 312, 63),
];

/// Phase 2: the three reserved corner glyphs, emitted opaquely in the
/// chrome colour on black so their clear bits carve the bevel back out
/// of the fills. There is deliberately no bottom-right glyph.
const CHROME_CORNER_GLYPHS: [(u8, u8, u8); 3] = [(0x7b, 0, 0), (0x7c, 39, 0), (0x7d, 0, 23)];

/// Inclusive pixel rectangles of the 1px white rules that are never
/// interrupted by a ribbon gap.
const FIXED_RULES: [(usize, usize, usize, usize); 8] = [
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

/// The [`RIBBON_BANDS`] entry that fills the divider band between the
/// roster box and the counters box.
const STATS_DIVIDER_BAND: (usize, usize, usize, usize) = (192, 57, 312, 63);

/// The two rule fragments that close the panel's left and right edges
/// across that band when the panel is drawn as one tall box.
const STATS_SINGLE_BOX_RULES: [(usize, usize, usize, usize); 2] =
    [(191, 57, 191, 62), (312, 57, 312, 62)];

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
    /// Optional label punched through the top rule of the stats/roster
    /// box. Modal selectors use `Select:` or `Items:`; the arms sell
    /// browser temporarily replaces them with `Arms`.
    pub stats_label: Option<String>,
    /// Timing/status tag byte. `None` (or zero) leaves divider row 7 a
    /// plain ribbon band with no caps and no placeholder. While the arms
    /// sell browser is open, its page-status glyph owns this slot instead.
    pub timing_glyph: Option<u8>,
    /// Draw the stats panel as one tall box, rows 1..=9, instead of the
    /// standing roster box + divider band + counters box.
    ///
    /// Runtime observation, spec silent: `inventory.md §4.7` has the
    /// Z-stats attribute page clear the whole panel, and a capture of
    /// the original's stat sheet shows the divider band's two rules at
    /// `y = 56` and `y = 63` gone with the left and right panel rules
    /// running unbroken from `y = 7` to `y = 80`. The counters and date
    /// rows the band separated are not drawn while a page is open, so
    /// there is nothing left for it to divide.
    pub stats_panel_single_box: bool,
}

/// The two fixed-cell fonts the chrome pass draws from.
#[derive(Clone, Copy)]
pub struct ChromeFonts<'a> {
    /// `IBM.CH`: end-cap source triangles and label text.
    pub ibm: &'a FixedCellFont,
    /// `RUNES.CH`: sky-strip moon phases and hour marker.
    pub runes: &'a FixedCellFont,
}

/// The two accent strokes each cap direction lays along its triangle's
/// hypotenuse, as cell-relative pixel endpoints
/// (`display-driver.md` section 7, "Bracket end-caps").
const fn ribbon_cap_strokes(direction: RibbonCapDirection) -> [((i32, i32), (i32, i32)); 2] {
    match direction {
        RibbonCapDirection::Right => [((0, 0), (5, 3)), ((5, 4), (0, 7))],
        RibbonCapDirection::Left => [((7, 0), (2, 3)), ((2, 4), (7, 7))],
    }
}

/// Compose the ribbon end cap from the shipped `IBM.CH` font.
///
/// Two passes, per the published contract: the opaque solid-triangle
/// glyph in the chrome colour, then two accent strokes along its
/// hypotenuse. The accent mask is the strokes; the chrome mask is the
/// triangle minus them. See the module header.
pub fn ribbon_cap_sprite(font: &FixedCellFont, direction: RibbonCapDirection) -> RibbonCapSprite {
    let code = direction.source_glyph();
    let mut solid = [0u8; CH_CELL_SIDE];
    for (row, bits) in solid.iter_mut().enumerate() {
        *bits = font.glyph_row(code, row).unwrap_or_else(|| {
            panic!("IBM.CH glyph {code:#04x} row {row} missing; ribbon end cap needs it")
        });
    }
    let mut white = [0u8; CH_CELL_SIDE];
    for (start, end) in ribbon_cap_strokes(direction) {
        stroke_cell_line(&mut white, start, end);
    }
    let mut ribbon = [0u8; CH_CELL_SIDE];
    for row in 0..CH_CELL_SIDE {
        ribbon[row] = solid[row] & !white[row];
    }
    RibbonCapSprite { white, ribbon }
}

/// Rasterise one straight line into an 8x8 cell mask.
fn stroke_cell_line(mask: &mut [u8; CH_CELL_SIDE], start: (i32, i32), end: (i32, i32)) {
    let (mut x, mut y) = start;
    let (x1, y1) = end;
    let dx = (x1 - x).abs();
    let dy = (y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut err = dx - dy;
    loop {
        if (0..CH_CELL_SIDE as i32).contains(&x) && (0..CH_CELL_SIDE as i32).contains(&y) {
            mask[y as usize] |= 1 << (7 - x);
        }
        if x == x1 && y == y1 {
            break;
        }
        let doubled = 2 * err;
        if doubled > -dy {
            err -= dy;
            x += sx;
        }
        if doubled < dx {
            err += dx;
            y += sy;
        }
    }
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
/// Format the wind banner (`weather.md` section 2.1).
///
/// All five direction labels are stored padded to exactly five
/// characters and the shared suffix carries its own leading space, so
/// `Calm`, `East` and `West` render with two spaces before `Winds`
/// while `North` and `South` render with one. The result is eleven
/// cells, columns 7..=17, with caps at 6 and 18.
///
/// An out-of-range saved wind byte falls out of the label selection
/// entirely rather than clamping to Calm: no direction is printed, the
/// suffix lands at columns 7..=12, and the closing cap therefore sits
/// at column 13, leaving a visibly short banner.
pub fn wind_banner_text(direction_name: Option<&str>) -> String {
    match direction_name {
        // The suffix keeps its own leading space here too, so it
        // occupies columns 7..=12 and the closing cap lands at 13.
        None => WIND_BANNER_SUFFIX.to_string(),
        Some(direction) => {
            let direction: String = direction
                .chars()
                .take(WIND_BANNER_DIRECTION_CELLS)
                .collect();
            format!("{direction:<WIND_BANNER_DIRECTION_CELLS$}{WIND_BANNER_SUFFIX}")
        }
    }
}

/// Dungeon-class replacement for the sky strip: the current level.
///
/// `dungeon-mode.md §4.1` (`cleak/u5-spec#81`): the level is stored
/// zero-based and **displayed one-based**, range one through eight, and
/// the rendered label is always exactly four cells — right cap, `L`,
/// digit, left cap. (The stored literal is `L` plus a placeholder space
/// that the status redraw seeks back over and prints the digit into,
/// which is why the digit sits immediately after the letter.)
pub fn dungeon_level_label(level: u8) -> String {
    format!("L{}", crate::dungeon_display_level(level))
}

/// Dungeon-class replacement for the wind banner: the party facing.
/// `Dir:` label literal preceding the facing field.
pub const DUNGEON_FACING_LABEL: &str = "Dir:";
/// Cells of the dungeon facing field.
pub const DUNGEON_FACING_CELLS: usize = 5;

/// Dungeon-class replacement for the wind banner
/// (`dungeon-mode.md` section 4.1): `Dir:` in columns 7..=10, a pad
/// space at 11, then a five-character facing field at 12..=16, with
/// caps at 6 and 17.
///
/// The stored names for East and West carry their own leading space,
/// mirroring the wind banner's pad trick, so the label reads `Dir:`
/// plus two spaces for East and West and `Dir:` plus one space for
/// North and South. The published literals are these terse
/// abbreviations; the paraphrases "Dungeon Level N" and "Facing North"
/// do not exist anywhere in the shipped build.
pub fn dungeon_direction_label(direction_name: &str) -> String {
    format!(
        "{DUNGEON_FACING_LABEL} {}",
        dungeon_facing_field(direction_name)
    )
}

/// The five-cell facing field, right-aligned the way the shipped names
/// are stored (East and West carry a leading space, North and South do
/// not).
pub fn dungeon_facing_field(direction_name: &str) -> String {
    let name: String = direction_name.chars().take(DUNGEON_FACING_CELLS).collect();
    format!("{name:>DUNGEON_FACING_CELLS$}")
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

/// Resolve the modal label punched through the stats/roster box's top
/// ribbon. Unlike the viewport labels, this uses the published cap-anchor
/// formula rather than centring within a nominal fixed-width field.
pub fn stats_label_gap(label: Option<&str>) -> Option<ResolvedGap> {
    let label = label.filter(|label| !label.is_empty())?;
    let cells = label.chars().count();
    let (left_cap_column, content_first_column, right_cap_column) = stats_label_strip_span(cells);
    Some(ResolvedGap {
        left_cap_column,
        right_cap_column,
        content_first_column,
        content_cells: cells,
    })
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
    palette: ChromePalette,
) {
    let top = top_gap(&content.top);
    let bottom = bottom_gap(&content.bottom);
    let stats_label = stats_label_gap(content.stats_label.as_deref());
    let timing = timing_glyph_gap(content.timing_glyph);

    for (x0, y0, x1, y1) in RIBBON_BANDS {
        if content.stats_panel_single_box && (x0, y0, x1, y1) == STATS_DIVIDER_BAND {
            continue;
        }
        fill_rect(rgba, width, height, x0, y0, x1, y1, palette.chrome);
    }
    carve_rounded_corners(rgba, width, height, fonts.ibm, palette);

    for (gap, band_row) in [
        (top, SKY_STRIP_ROW),
        (bottom, WIND_BANNER_ROW),
        (stats_label, STATS_LABEL_STRIP_ROW),
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
        fill_rect(rgba, width, height, x0, y0, x1, y1, palette.accent);
    }
    if content.stats_panel_single_box {
        // Close the left and right panel rules across the vacated band.
        for (x0, y0, x1, y1) in STATS_SINGLE_BOX_RULES {
            fill_rect(rgba, width, height, x0, y0, x1, y1, palette.accent);
        }
    }
    paint_interrupted_rule(rgba, width, height, 7, 7, 184, top, palette);
    paint_interrupted_rule(rgba, width, height, 184, 7, 184, bottom, palette);
    paint_interrupted_rule(rgba, width, height, 7, 191, 312, stats_label, palette);
    for row_y in DIVIDER_RULE_ROWS {
        let band_rule = row_y == 56 || row_y == 63;
        if band_rule && content.stats_panel_single_box {
            continue;
        }
        let gap = if band_rule { timing } else { None };
        paint_interrupted_rule(rgba, width, height, row_y, 191, 312, gap, palette);
    }

    if let Some(gap) = top {
        paint_gap_caps(rgba, width, height, fonts.ibm, gap, SKY_STRIP_ROW, palette);
        paint_gap_content(rgba, width, height, fonts, gap, SKY_STRIP_ROW, &content.top);
    }
    if let Some(gap) = bottom {
        paint_gap_caps(
            rgba,
            width,
            height,
            fonts.ibm,
            gap,
            WIND_BANNER_ROW,
            palette,
        );
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
    if let (Some(gap), Some(label)) = (stats_label, content.stats_label.as_deref()) {
        paint_gap_caps(
            rgba,
            width,
            height,
            fonts.ibm,
            gap,
            STATS_LABEL_STRIP_ROW,
            palette,
        );
        paint_label_content(
            rgba,
            width,
            height,
            fonts.ibm,
            gap,
            STATS_LABEL_STRIP_ROW,
            label,
        );
    }
    if let (Some(gap), Some(byte)) = (timing, content.timing_glyph) {
        paint_gap_caps(
            rgba,
            width,
            height,
            fonts.ibm,
            gap,
            TIMING_GLYPH_ROW,
            palette,
        );
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
    palette: ChromePalette,
) {
    draw_ribbon_cap(rgba, width, height, font, direction, column, row, palette);
}

/// Paint the message window's per-line right-pointing cap prefix.
pub fn paint_message_line_cap(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    row: u8,
    palette: ChromePalette,
) {
    paint_ribbon_cap(
        rgba,
        width,
        height,
        font,
        RibbonCapDirection::Right,
        MESSAGE_WINDOW_LEFT,
        row,
        palette,
    );
}

/// Phase 2: stamp the three reserved corner glyphs opaquely over the
/// fills. Set bits paint chrome, clear bits paint black, so the glyph's
/// carve shapes the bevel out of the band that was filled underneath.
fn carve_rounded_corners(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    palette: ChromePalette,
) {
    for (code, column, row) in CHROME_CORNER_GLYPHS {
        let mut mask = [0u8; CH_CELL_SIDE];
        for (glyph_row, bits) in mask.iter_mut().enumerate() {
            *bits = font.glyph_row(code, glyph_row).unwrap_or_else(|| {
                panic!("IBM.CH glyph {code:#04x} row {glyph_row} missing; chrome corner needs it")
            });
        }
        draw_opaque_mask(
            rgba,
            width,
            height,
            &mask,
            column,
            row,
            palette.chrome,
            palette.background,
        );
    }
}

/// Draw an 8x8 mask with both colours written: set bits take
/// `foreground`, clear bits take `background`. This is the "emit an
/// opaque glyph" primitive the published paint relies on for the corner
/// bevel and the end caps' triangle pass.
fn draw_opaque_mask(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    mask: &[u8; CH_CELL_SIDE],
    column: u8,
    row: u8,
    foreground: u8,
    background: u8,
) {
    let base_x = column as usize * CH_CELL_SIDE;
    let base_y = row as usize * CH_CELL_SIDE;
    for (glyph_y, bits) in mask.iter().enumerate() {
        let y = base_y + glyph_y;
        if y >= height {
            break;
        }
        for glyph_x in 0..CH_CELL_SIDE {
            let x = base_x + glyph_x;
            if x >= width {
                continue;
            }
            let index = if bits & (1 << (7 - glyph_x)) != 0 {
                foreground
            } else {
                background
            };
            let rgb = EGA_PALETTE_RGB[usize::from(index & 0x0f)];
            let offset = (y * width + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
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
    palette: ChromePalette,
) {
    match gap {
        None => fill_rect(rgba, width, height, x0, row_y, x1, row_y, palette.accent),
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
                    palette.accent,
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
                    palette.accent,
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
    palette: ChromePalette,
) {
    draw_ribbon_cap(
        rgba,
        width,
        height,
        font,
        RibbonCapDirection::Right,
        gap.left_cap_column,
        row,
        palette,
    );
    draw_ribbon_cap(
        rgba,
        width,
        height,
        font,
        RibbonCapDirection::Left,
        gap.right_cap_column,
        row,
        palette,
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
            paint_label_content(rgba, width, height, fonts.ibm, gap, row, label);
        }
    }
}

fn paint_label_content(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    font: &FixedCellFont,
    gap: ResolvedGap,
    row: u8,
    label: &str,
) {
    for (index, ch) in label.chars().take(gap.content_cells).enumerate() {
        draw_glyph(
            rgba,
            width,
            height,
            font,
            (ch as u32 as u8) & 0x7f,
            gap.content_first_column + index as u8,
            row,
            CHROME_RULE_INDEX,
        );
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
    palette: ChromePalette,
) {
    let sprite = ribbon_cap_sprite(font, direction);
    draw_mask(
        rgba,
        width,
        height,
        &sprite.ribbon,
        column,
        row,
        palette.chrome,
    );
    draw_mask(
        rgba,
        width,
        height,
        &sprite.white,
        column,
        row,
        palette.accent,
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
        *bits = font.glyph_row(code, glyph_row).unwrap_or_else(|| {
            panic!(
                "fixed-cell glyph {code:#04x} row {glyph_row} missing at column {column}                  row {row}: the caller handed the renderer a code this font does not                  cover. That usually means an unmasked high byte reached the text path                  — `paint_label_content` masks with 0x7F, so a code above 0x7F here came                  from a rune/sky-strip caller or from asset text that was not decoded."
            )
        });
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
/// Drift`: the wind helper does not run in dungeon-class scenes).
///
/// The top gap has a third case, resolved by issue #190: the marker
/// painter's **erase arm**. `moons.md §2.2` publishes it as live on four
/// routes - scene 25 (Ararat) by the scene test, and a below-surface
/// party Z (the Underworld plane, or a below-entry floor inside a
/// town-family location) by the level test. On the arm "the strip is not
/// rendered at all": the painter "flat-fills the strip footprint and
/// rules the scanline under it. Nothing of the hour marker or of either
/// moon is left on screen, and both end-caps are erased with them",
/// leaving a plain ribbon. That is [`ChromeGap::Unbroken`].
///
/// The Underworld top gap was already unbroken, for want of published
/// content; it is now unbroken *because* the erase arm says so, and
/// Ararat and every basement floor join it. The bottom gap is not the
/// strip's business - the wind banner has its own erase branch in
/// `weather.md §2.1` - so it is left as it was.
pub fn gameplay_chrome_content(state: &crate::PlayState) -> GameplayChromeContent {
    let browser = crate::stats_panel::active_arms_sell_browser(state);
    let stats_label = browser
        .map(|_| crate::stats_panel::ARMS_SELL_BROWSER_STATS_LABEL.to_string())
        .or_else(|| state.roster_box_label());
    let timing_glyph = if let Some(browser) = browser {
        use crate::shop_runtime::ArmsSellPageIndicator;

        match browser.page_indicator(&state.equipment_stock) {
            ArmsSellPageIndicator::None => None,
            ArmsSellPageIndicator::Down => {
                Some(crate::stats_panel::ARMS_SELL_BROWSER_PAGE_GLYPH_DOWN)
            }
            ArmsSellPageIndicator::Up => Some(crate::stats_panel::ARMS_SELL_BROWSER_PAGE_GLYPH_UP),
            ArmsSellPageIndicator::Both => {
                Some(crate::stats_panel::ARMS_SELL_BROWSER_PAGE_GLYPH_BOTH)
            }
        }
    } else {
        state.active_effect_tag.filter(|tag| *tag != 0)
    };
    let stats_panel_single_box = state.active_z_stats.is_some();
    match state.area {
        crate::Area::Dungeon { level, .. } => GameplayChromeContent {
            top: ChromeGap::Label(dungeon_level_label(level)),
            bottom: ChromeGap::Label(dungeon_direction_label(state.player.facing.name())),
            stats_label,
            timing_glyph,
            stats_panel_single_box,
        },
        crate::Area::World { plane } if plane == crate::WorldPlane::Underworld => {
            GameplayChromeContent {
                top: ChromeGap::Unbroken,
                bottom: ChromeGap::Unbroken,
                stats_label,
                timing_glyph,
                stats_panel_single_box,
            }
        }
        // `moons.md §2.2`, the erase arm inside the surface/town family:
        // Ararat by the scene test, a below-entry floor by the level test.
        // The renderer is reached (so the cached pair is still written by
        // the callers of §3) but paints no strip.
        crate::Area::Town { scene, floor } if crate::sky_strip_erase_arm(scene.byte, floor < 0) => {
            GameplayChromeContent {
                top: ChromeGap::Unbroken,
                bottom: ChromeGap::Label(wind_banner_text(wind_banner_direction_name(state))),
                stats_label,
                timing_glyph,
                stats_panel_single_box,
            }
        }
        crate::Area::World { .. } | crate::Area::Town { .. } => GameplayChromeContent {
            top: ChromeGap::SkyStrip(Box::new(sky_strip_cells(
                state.clock.hour,
                state.cached_moon_glyph_bytes,
            ))),
            bottom: ChromeGap::Label(wind_banner_text(wind_banner_direction_name(state))),
            stats_label,
            timing_glyph,
            stats_panel_single_box,
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

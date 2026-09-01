//! Proportional paragraph layout for the intro story slides and the
//! character-creation screens.
//!
//! The renderer contract is `systems/text-output.md` sections 8.1 to 8.5, the
//! advance table is `formats/font-pcs.md` section 4.1, the per-step paragraph
//! boxes are `systems/intro.md` section 10 ("Per-step paragraph box"), and the
//! step-6 inline doorway lines are `systems/intro.md` section 10.1. Published
//! in answer to `cleak/u5-spec#69` and `cleak/u5-spec#70`.
//!
//! The layout descriptor is not a single rectangle: it carries two horizontal
//! margin pairs plus a vertical band, and the pair is re-selected after every
//! line break. That is what makes a paragraph flow around the slide artwork.
//!
//! No story text is stored here beyond the two published inline doorway lines
//! that have no `STORY.DAT` record; everything else is read from the shipped
//! files at runtime.

use std::io;

use crate::{
    INTRO_INLINE_DOORWAY_STEP, INTRO_STORY_STEP_COUNT, PCS_GLYPH_ADVANCE_GAP, PCS_GLYPH_HEIGHT,
    PCS_SPACE_ADVANCE, ProportionalWidthTable, STORY_HARD_NEWLINE_MARKER,
    STORY_PARAGRAPH_START_MARKER, STORY_RECORD_END_MARKER, STORY_SOFT_BREAK_MARKER,
};

/// `text-output.md §8.5`: the pen advances a fixed nine pixels per line, for
/// every step, independent of margins and band.
pub const PROPORTIONAL_LINE_STRIDE: u16 = PCS_GLYPH_HEIGHT as u16 + 1;

/// `text-output.md §8.5`: glyph drawing is clipped at the bottom. Once the pen
/// row reaches 192 glyphs stop being drawn, but the pen still advances exactly
/// as if they were, so layout does not change when text runs off the bottom.
pub const PROPORTIONAL_DRAW_CLIP_Y: u16 = 192;

/// `text-output.md §8.2`: `{` is a first-line indent marker measured as a flat
/// fifteen pixels. The renderer draws nothing and the pen still advances.
pub const PROPORTIONAL_BRACE_INDENT: u16 = 15;

/// `text-output.md §8.1`: the shipped resident default space advance. Only
/// character creation overrides it, and only for one paragraph.
pub const PROPORTIONAL_DEFAULT_SPACE_ADVANCE: u8 = PCS_SPACE_ADVANCE;

/// `font-pcs.md §4.1`: the renderer reads the hyphen's own advance-table entry
/// once per line, for the soft-hyphen fit test and for the hyphen it draws at
/// a hyphenated break.
const HYPHEN_CODE: u8 = b'-';

/// The renderer's layout descriptor (`text-output.md §8.1`), in pixels.
///
/// Margin pair B is selected while `band_low < pen_y < band_high` - strictly
/// inside, both ends excluded - and pair A otherwise. The check runs at entry
/// and again after every line break. A band of `200..200` can never match, so
/// such a descriptor always uses pair A.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProportionalLayoutDescriptor {
    pub left_a: u16,
    pub right_a: u16,
    pub left_b: u16,
    pub right_b: u16,
    pub band_low: u16,
    pub band_high: u16,
    /// Pixels a space contributes, before justification padding.
    pub space_advance: u8,
    /// Pen origin. `pen_x - left` is treated as width already consumed on the
    /// first line, so a caller can start a paragraph part-way along a line;
    /// the pen resets to the margin at the first line break.
    pub pen_x: u16,
    pub pen_y: u16,
}

impl ProportionalLayoutDescriptor {
    /// A full-width descriptor whose band can never select pair B.
    pub const fn full_width(pen_x: u16, pen_y: u16) -> Self {
        Self {
            left_a: 0,
            right_a: 320,
            left_b: 0,
            right_b: 320,
            band_low: 200,
            band_high: 200,
            space_advance: PROPORTIONAL_DEFAULT_SPACE_ADVANCE,
            pen_x,
            pen_y,
        }
    }

    /// Inclusive-left, exclusive-right margins for a line whose pen row is `pen_y`.
    pub fn margins_for(&self, pen_y: u16) -> (u16, u16) {
        if pen_y > self.band_low && pen_y < self.band_high {
            (self.left_b, self.right_b)
        } else {
            (self.left_a, self.right_a)
        }
    }
}

const fn step_box(
    left_a: u16,
    right_a: u16,
    left_b: u16,
    right_b: u16,
    band_low: u16,
    band_high: u16,
    pen_x: u16,
    pen_y: u16,
) -> ProportionalLayoutDescriptor {
    ProportionalLayoutDescriptor {
        left_a,
        right_a,
        left_b,
        right_b,
        band_low,
        band_high,
        space_advance: PROPORTIONAL_DEFAULT_SPACE_ADVANCE,
        pen_x,
        pen_y,
    }
}

/// `systems/intro.md §10` "Per-step paragraph box", published in answer to
/// `cleak/u5-spec#70`: both margin pairs, the band bounds, and the pen origin
/// for each of the twenty-one intro story steps.
const INTRO_STORY_PARAGRAPH_BOXES: [ProportionalLayoutDescriptor; INTRO_STORY_STEP_COUNT] = [
    //        A left/right   B left/right   band low/high   pen origin
    step_box(180, 320, 0, 320, 180, 200, 180, 128),
    step_box(0, 320, 172, 320, 70, 200, 0, 0),
    step_box(0, 132, 0, 320, 131, 200, 0, 40),
    step_box(0, 320, 210, 320, 32, 160, 0, 0),
    step_box(0, 320, 0, 148, 70, 200, 0, 9),
    step_box(176, 320, 0, 320, 133, 200, 176, 0),
    step_box(0, 320, 0, 320, 200, 200, 32, 9),
    step_box(188, 320, 0, 320, 168, 200, 188, 136),
    step_box(0, 320, 0, 320, 200, 200, 0, 0),
    step_box(0, 320, 0, 320, 200, 200, 0, 0),
    step_box(0, 320, 0, 320, 200, 200, 0, 0),
    step_box(0, 320, 0, 320, 200, 200, 0, 0),
    step_box(0, 320, 0, 320, 200, 200, 0, 0),
    step_box(0, 170, 0, 320, 114, 200, 0, 0),
    step_box(184, 320, 0, 320, 114, 200, 184, 32),
    step_box(0, 170, 0, 320, 96, 200, 0, 0),
    step_box(0, 320, 148, 320, 33, 137, 0, 0),
    step_box(0, 320, 0, 170, 70, 200, 0, 0),
    // Step 18's pen X (174) deliberately differs from its left margin (148):
    // the renderer treats the difference as width already consumed on the
    // first line and resets the pen to the margin at the first line break.
    step_box(148, 320, 0, 320, 96, 200, 174, 0),
    step_box(0, 320, 0, 170, 51, 146, 0, 9),
    step_box(0, 320, 156, 320, 79, 200, 0, 0),
];

/// Paragraph box for an intro story step, or `None` outside `0..=20`.
///
/// Step 6 has an entry like every other step: it renders the inline doorway
/// lines rather than a `STORY.DAT` record, but through the same renderer with
/// the same descriptor model (`systems/intro.md §10.1`).
pub fn intro_story_paragraph_box(step: usize) -> Option<ProportionalLayoutDescriptor> {
    INTRO_STORY_PARAGRAPH_BOXES.get(step).copied()
}

/// `systems/intro.md §10.1`: the two inline doorway lines for step 6, the one
/// step whose narrative text is not a `STORY.DAT` record. Published verbatim
/// in answer to `cleak/u5-spec#69`; each is exactly 45 characters and carries
/// none of the markers a `STORY.DAT` record does.
pub const INTRO_DOORWAY_LINES: [&str; 2] = [
    "Instantly, a shimmering blue door springs up!",
    "With heart beating rapidly, you step into it.",
];

/// `systems/intro.md §10.1`: line 2 re-issues the step's left pen origin with
/// the vertical origin pinned to this row. The two lines are placed by
/// explicit origins, not by the renderer's line advance.
pub const INTRO_DOORWAY_SECOND_LINE_PEN_Y: u16 = 180;

/// Descriptors for step 6's two paragraph calls, in draw order.
pub fn intro_doorway_paragraph_boxes() -> [ProportionalLayoutDescriptor; 2] {
    let first = INTRO_STORY_PARAGRAPH_BOXES[INTRO_INLINE_DOORWAY_STEP];
    let mut second = first;
    second.pen_y = INTRO_DOORWAY_SECOND_LINE_PEN_Y;
    [first, second]
}

/// `cleak/u5-spec#70`, measured off a capture of the original: the
/// character-creation gypsy narrative runs full width above the `CREATE`
/// opening panel at `(0, 96)` and to the right of it below, its first line at
/// pen `(0, 9)` with the ordinary brace indent.
pub const CHARGEN_GYPSY_PARAGRAPH_BOX: ProportionalLayoutDescriptor =
    step_box(0, 320, 175, 320, 89, 200, 0, 9);

/// `cleak/u5-spec#70`: the character-creation result text runs full width
/// above the `CREATE` result panel at `(168, 100)` and to its left below.
/// This is the one caller `text-output.md §8.1` records as overriding the
/// space advance, to 4, for a single paragraph.
///
/// `systems/chargen.md §5.1.2` publishes the result column of the descriptor
/// table as "Band low, high | 90, 200", restated in prose as margin pair B
/// (0..166, left of the 152-wide art at x = 168) applying "for every line
/// whose pen has passed y = 90". The band low is 90, not 93.
pub const CHARGEN_RESULT_PARAGRAPH_BOX: ProportionalLayoutDescriptor =
    ProportionalLayoutDescriptor {
        left_a: 0,
        right_a: 320,
        left_b: 0,
        right_b: 166,
        band_low: 90,
        band_high: 200,
        space_advance: CHARGEN_RESULT_SPACE_ADVANCE,
        pen_x: 0,
        pen_y: 0,
    };

/// `text-output.md §8.1`: character creation uses a 4-pixel space advance for
/// one paragraph and restores the shipped 5 immediately afterwards.
pub const CHARGEN_RESULT_SPACE_ADVANCE: u8 = 4;

/// `cleak/u5-spec#70`: the questionnaire prompt is a plain full-width block
/// below the two incense-bowl backings, which occupy rows 0..147.
pub const CHARGEN_QUESTION_PARAGRAPH_BOX: ProportionalLayoutDescriptor =
    ProportionalLayoutDescriptor::full_width(0, 152);

// ---------------------------------------------------------------------
// `systems/endgame.md` section 8 - the six fixed final narrative windows.
//
// The endgame's prose uses the same proportional paragraph renderer as
// the intro slides and the chargen screens, so its per-window layout
// descriptors live here beside them. The panel bindings and title
// strips travel with the descriptors because 8.1 and 8.2 publish them
// as parallel per-window tables: "All of the values below are fixed
// resident data laid out as parallel per-window tables; nothing about
// them is computed at run time."
// ---------------------------------------------------------------------

/// `systems/endgame.md §8`: the six fixed `END.DAT` narrative windows.
pub const ENDGAME_NARRATIVE_WINDOWS: usize = 6;

/// `systems/endgame.md §8.1`: the first endgame panel archive. Windows
/// 1 to 3 take their panel from it, slots 0 to 2 in order.
pub const ENDGAME_PANEL_ARCHIVE_1: &str = "END1";
/// `systems/endgame.md §8.1`: the second endgame panel archive. Windows
/// 4 to 6 take theirs from it, slots 0 to 2 in order.
pub const ENDGAME_PANEL_ARCHIVE_2: &str = "END2";
/// `systems/endgame.md §8.2`: the shared strip archive the two
/// decorative chapter titles come from.
pub const ENDGAME_TITLE_STRIP_ARCHIVE: &str = "TEXT";

/// One row of the `systems/endgame.md §8.1` per-window binding table.
///
/// "Each of the six windows binds one panel from the endgame panel
/// archives to one `END.DAT` record and one paragraph rectangle." The
/// panel is drawn opaque, with no border, no shadow and no frame of its
/// own, and the window numbering matches the `END.DAT` record numbering
/// of `formats/end-dat.md` section 4.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndgameNarrativePanel {
    /// `END1` or `END2`. "The archive is opened when it first becomes
    /// the required one and released when the next window needs a
    /// different archive."
    pub archive: &'static str,
    pub slot: u8,
    pub width: u16,
    pub height: u16,
    pub top_left_x: u16,
    pub top_left_y: u16,
    /// The window's `END.DAT` record, which is also its window number.
    pub end_dat_record: u8,
}

/// `systems/endgame.md §8.1` "Per-window bindings", verbatim:
///
/// ```text
/// | Window | Archive | Slot | Panel size | Panel top-left | END.DAT record |
/// |      1 | END1    |    0 | 167 x 124  | (0, 0)         |              1 |
/// |      2 | END1    |    1 | 191 x 90   | (64, 0)        |              2 |
/// |      3 | END1    |    2 | 192 x 95   | (0, 52)        |              3 |
/// |      4 | END2    |    0 | 173 x 98   | (0, 0)         |              4 |
/// |      5 | END2    |    1 | 157 x 90   | (0, 92)        |              5 |
/// |      6 | END2    |    2 | 153 x 110  | (160, 0)       |              6 |
/// ```
pub const ENDGAME_NARRATIVE_PANELS: [EndgameNarrativePanel; ENDGAME_NARRATIVE_WINDOWS] = [
    EndgameNarrativePanel {
        archive: ENDGAME_PANEL_ARCHIVE_1,
        slot: 0,
        width: 167,
        height: 124,
        top_left_x: 0,
        top_left_y: 0,
        end_dat_record: 1,
    },
    EndgameNarrativePanel {
        archive: ENDGAME_PANEL_ARCHIVE_1,
        slot: 1,
        width: 191,
        height: 90,
        top_left_x: 64,
        top_left_y: 0,
        end_dat_record: 2,
    },
    EndgameNarrativePanel {
        archive: ENDGAME_PANEL_ARCHIVE_1,
        slot: 2,
        width: 192,
        height: 95,
        top_left_x: 0,
        top_left_y: 52,
        end_dat_record: 3,
    },
    EndgameNarrativePanel {
        archive: ENDGAME_PANEL_ARCHIVE_2,
        slot: 0,
        width: 173,
        height: 98,
        top_left_x: 0,
        top_left_y: 0,
        end_dat_record: 4,
    },
    EndgameNarrativePanel {
        archive: ENDGAME_PANEL_ARCHIVE_2,
        slot: 1,
        width: 157,
        height: 90,
        top_left_x: 0,
        top_left_y: 92,
        end_dat_record: 5,
    },
    EndgameNarrativePanel {
        archive: ENDGAME_PANEL_ARCHIVE_2,
        slot: 2,
        width: 153,
        height: 110,
        top_left_x: 160,
        top_left_y: 0,
        end_dat_record: 6,
    },
];

/// One decorative `TEXT` strip drawn over a narrative window
/// (`systems/endgame.md §8.2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndgameTitleStrip {
    pub slot: u8,
    pub x: u16,
    pub y: u16,
}

/// `systems/endgame.md §8.2`: "Two windows also draw decorative title
/// strips from the shared `TEXT` strip archive ... before the prose is
/// laid out."
///
/// ```text
/// | Window | Strips drawn, in order                             |
/// |      1 | TEXT slot 0 at (216, 0), then slot 4 at (152, 28)   |
/// |      4 | TEXT slot 5 at (224, 0), then slot 0 at (176, 0)    |
/// ```
///
/// The pairs read as the chapter titles "The Homecoming" and "The
/// Dream"; the words are part of the artwork, not typeset text. The
/// overlap inside window 4's pair is intentional kerning - the second
/// strip is drawn opaque over the first.
const ENDGAME_WINDOW_1_TITLE_STRIPS: [EndgameTitleStrip; 2] = [
    EndgameTitleStrip {
        slot: 0,
        x: 216,
        y: 0,
    },
    EndgameTitleStrip {
        slot: 4,
        x: 152,
        y: 28,
    },
];
const ENDGAME_WINDOW_4_TITLE_STRIPS: [EndgameTitleStrip; 2] = [
    EndgameTitleStrip {
        slot: 5,
        x: 224,
        y: 0,
    },
    EndgameTitleStrip {
        slot: 0,
        x: 176,
        y: 0,
    },
];

/// The section 8.2 title strips for a zero-based window index, in draw
/// order. Only windows 1 and 4 (indices 0 and 3) have any.
pub fn endgame_narrative_title_strips(window_index: usize) -> &'static [EndgameTitleStrip] {
    match window_index {
        0 => &ENDGAME_WINDOW_1_TITLE_STRIPS,
        3 => &ENDGAME_WINDOW_4_TITLE_STRIPS,
        _ => &[],
    }
}

/// `systems/endgame.md §8.2` "Per-window paragraph rectangles",
/// verbatim:
///
/// ```text
/// | Window | Pen start | Outside: l, r | Inside: l, r | Band low, high |
/// |      1 | (172, 66) | 172, 320      | 0, 320       | 126, 200       |
/// |      2 | (0, 92)   | 0, 320        | 0, 320       | 126, 200       |
/// |      3 | (0, 9)    | 0, 320        | 196, 320     | 42, 148        |
/// |      4 | (179, 38) | 179, 320      | 0, 320       | 100, 200       |
/// |      5 | (0, 9)    | 0, 320        | 161, 320     | 82, 200        |
/// |      6 | (0, 0)    | 0, 154        | 0, 320       | 112, 200       |
/// ```
///
/// "Line advance is nine pixels and glyph output stops once the pen
/// reaches vertical position 192. The endgame never writes the space
/// advance, so all six windows lay out with the shipped default of
/// five." Those are already this module's [`PROPORTIONAL_LINE_STRIDE`],
/// [`PROPORTIONAL_DRAW_CLIP_Y`] and
/// [`PROPORTIONAL_DEFAULT_SPACE_ADVANCE`], so `step_box` is the right
/// constructor here.
pub const ENDGAME_NARRATIVE_PARAGRAPH_BOXES: [ProportionalLayoutDescriptor;
    ENDGAME_NARRATIVE_WINDOWS] = [
    //        A left/right   B left/right   band low/high   pen origin
    step_box(172, 320, 0, 320, 126, 200, 172, 66),
    step_box(0, 320, 0, 320, 126, 200, 0, 92),
    step_box(0, 320, 196, 320, 42, 148, 0, 9),
    step_box(179, 320, 0, 320, 100, 200, 179, 38),
    step_box(0, 320, 161, 320, 82, 200, 0, 9),
    step_box(0, 154, 0, 320, 112, 200, 0, 0),
];

/// Paragraph descriptor for a zero-based section 8 narrative window.
pub fn endgame_narrative_paragraph_box(
    window_index: usize,
) -> Option<ProportionalLayoutDescriptor> {
    ENDGAME_NARRATIVE_PARAGRAPH_BOXES.get(window_index).copied()
}

/// Panel binding for a zero-based section 8 narrative window.
pub fn endgame_narrative_panel(window_index: usize) -> Option<EndgameNarrativePanel> {
    ENDGAME_NARRATIVE_PANELS.get(window_index).copied()
}

/// One laid-out glyph: the byte to draw and its top-left pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedProportionalGlyph {
    pub x: u16,
    pub y: u16,
    pub code: u8,
}

/// `text-output.md §8.2`: every byte at or below `0x20` is handled as a space
/// (NUL and line feed are terminators, tested before this).
fn is_space_byte(byte: u8) -> bool {
    byte <= b' '
}

/// Pixels a byte contributes to the measured advance.
fn measured_advance(
    widths: &ProportionalWidthTable,
    space_advance: u16,
    byte: u8,
) -> io::Result<u16> {
    Ok(match byte {
        STORY_PARAGRAPH_START_MARKER => PROPORTIONAL_BRACE_INDENT,
        STORY_SOFT_BREAK_MARKER => 0,
        byte if is_space_byte(byte) => space_advance,
        byte => glyph_advance(widths, byte)?,
    })
}

/// `font-pcs.md §4.2`: a drawn glyph advances by its table entry plus the
/// one-pixel inter-glyph gap, including the last glyph on a line.
fn glyph_advance(widths: &ProportionalWidthTable, byte: u8) -> io::Result<u16> {
    let width = widths.width_for_byte(byte)?;
    u16::try_from(width + usize::from(PCS_GLYPH_ADVANCE_GAP))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "glyph advance overflows"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LineStop {
    /// NUL or end of buffer: the paragraph is finished.
    EndOfText,
    /// Line feed: the line ends cleanly and is not justified.
    Newline,
    /// The measured advance reached the available width.
    Overflow,
}

/// Lays out a NUL-terminated proportional paragraph.
///
/// Implements `systems/text-output.md` sections 8.2 to 8.5 directly:
/// measurement stops at NUL, at a line feed, or as soon as
/// `available <= accumulated` (so the right edge is exclusive); overflow
/// backtracks to a space or to a soft hyphen that fits; exactly one break byte
/// is skipped after each line; accepted lines that ended on a break are fully
/// justified, with the truncating division's remainder landing on the last
/// spaces; and the pen advances nine pixels per line.
///
/// `draw_clip_y` is the pen row at which glyph drawing stops; layout continues
/// past it unchanged. Callers pass [`PROPORTIONAL_DRAW_CLIP_Y`].
pub fn layout_proportional_paragraph_glyphs(
    widths: &ProportionalWidthTable,
    descriptor: &ProportionalLayoutDescriptor,
    text: &[u8],
    draw_clip_y: u16,
) -> io::Result<Vec<PlacedProportionalGlyph>> {
    let space_advance = u16::from(descriptor.space_advance);
    // `text-output.md` section 8.3: the fit test uses the hyphen's raw
    // advance-table entry (3 in the shipped font) and adds the inter-glyph
    // gap itself; the accepted line then grows by the full glyph advance.
    let hyphen_width = u16::try_from(widths.width_for_byte(HYPHEN_CODE)?)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "hyphen width overflows"))?;
    let hyphen_advance = glyph_advance(widths, HYPHEN_CODE)?;
    let mut placed = Vec::new();
    let mut pen_y = descriptor.pen_y;
    let mut index = 0usize;
    let mut first_line = true;

    loop {
        let (left, right) = descriptor.margins_for(pen_y);
        let available = right.saturating_sub(left);
        // §8.1: only the entry pen may sit right of the margin, and the
        // difference counts as width already consumed. Margin selection is
        // re-evaluated after every break, so later lines start at whichever
        // margin the new pen row selects.
        let consumed = if first_line {
            descriptor.pen_x.saturating_sub(left)
        } else {
            0
        };

        // --- Measure (§8.3) ---
        let mut accumulated = consumed;
        let mut spaces = 0usize;
        let mut cursor = index;
        let stop = loop {
            let Some(&byte) = text.get(cursor) else {
                break LineStop::EndOfText;
            };
            if byte == STORY_RECORD_END_MARKER {
                break LineStop::EndOfText;
            }
            if byte == STORY_HARD_NEWLINE_MARKER {
                break LineStop::Newline;
            }
            if available <= accumulated {
                break LineStop::Overflow;
            }
            accumulated =
                accumulated.saturating_add(measured_advance(widths, space_advance, byte)?);
            if is_space_byte(byte) {
                spaces += 1;
            }
            cursor += 1;
        };

        // --- Resolve the break point (§8.3) ---
        let mut line_end = cursor;
        let mut measured = accumulated;
        let mut line_spaces = spaces;
        let mut draw_hyphen = false;
        let mut skip_break_byte = !matches!(stop, LineStop::EndOfText);

        if stop == LineStop::Overflow {
            let mut back = cursor;
            let mut resolved = false;
            while back > index {
                back -= 1;
                let byte = text[back];
                if byte == STORY_PARAGRAPH_START_MARKER {
                    continue;
                }
                if byte == STORY_SOFT_BREAK_MARKER {
                    if hyphen_width + measured + 1 < available {
                        line_end = back;
                        measured = measured.saturating_add(hyphen_advance);
                        draw_hyphen = true;
                        resolved = true;
                        break;
                    }
                    continue;
                }
                if is_space_byte(byte) {
                    line_end = back;
                    measured = measured.saturating_sub(space_advance);
                    line_spaces = line_spaces.saturating_sub(1);
                    resolved = true;
                    break;
                }
                measured = measured.saturating_sub(measured_advance(widths, space_advance, byte)?);
            }
            if !resolved {
                // Degenerate single-token overflow (`text-output.md` section
                // 8.3): the walk consumes one byte, so an over-long
                // unbreakable token is emitted one byte per line rather than
                // looping forever.
                line_end = (index + 1).min(cursor.max(index + 1));
                measured = consumed.saturating_add(measured_advance(
                    widths,
                    space_advance,
                    *text.get(index).unwrap_or(&0),
                )?);
                line_spaces = 0;
                skip_break_byte = false;
            }
        }

        // --- Render (§8.2, §8.4) ---
        // Justification is skipped when the line ended at NUL or a line feed.
        let justify = stop == LineStop::Overflow && line_spaces > 0;
        let mut slack = if justify {
            available.saturating_sub(measured)
        } else {
            0
        };
        let mut spaces_remaining = line_spaces;
        let mut x = left.saturating_add(consumed);
        for &byte in &text[index..line_end] {
            match byte {
                STORY_PARAGRAPH_START_MARKER => x = x.saturating_add(PROPORTIONAL_BRACE_INDENT),
                STORY_SOFT_BREAK_MARKER => {}
                byte if is_space_byte(byte) => {
                    x = x.saturating_add(space_advance);
                    if spaces_remaining > 0 {
                        let extra = slack / spaces_remaining as u16;
                        x = x.saturating_add(extra);
                        slack -= extra;
                        spaces_remaining -= 1;
                    }
                }
                byte => {
                    // §8.5: past the clip the pen still advances, so the walk
                    // is identical - only the glyph is dropped.
                    if pen_y < draw_clip_y {
                        placed.push(PlacedProportionalGlyph {
                            x,
                            y: pen_y,
                            code: byte,
                        });
                    }
                    x = x.saturating_add(glyph_advance(widths, byte)?);
                }
            }
        }
        if draw_hyphen && pen_y < draw_clip_y {
            placed.push(PlacedProportionalGlyph {
                x,
                y: pen_y,
                code: HYPHEN_CODE,
            });
        }

        if stop == LineStop::EndOfText {
            break;
        }
        // §8.3: exactly one break byte is skipped after the line is drawn.
        index = if skip_break_byte {
            line_end + 1
        } else {
            line_end
        };
        pen_y = pen_y.saturating_add(PROPORTIONAL_LINE_STRIDE);
        first_line = false;
    }

    Ok(placed)
}

#[cfg(test)]
mod chargen_descriptor_tests {
    use super::*;

    /// `systems/chargen.md §5.1.2` "Paragraph rectangles" publishes one column
    /// per chargen paragraph. This pins all three literally against that table
    /// so a drifted band bound cannot pass unnoticed again.
    #[test]
    fn chargen_paragraph_descriptors_match_published_table() {
        // (name, left_a, right_a, left_b, right_b, band_low, band_high,
        //  space_advance, pen_x, pen_y)
        let published: [(&str, ProportionalLayoutDescriptor); 3] = [
            (
                "opening gypsy paragraph",
                ProportionalLayoutDescriptor {
                    left_a: 0,
                    right_a: 320,
                    left_b: 175,
                    right_b: 320,
                    band_low: 89,
                    band_high: 200,
                    space_advance: 5,
                    pen_x: 0,
                    pen_y: 9,
                },
            ),
            (
                "per-question paragraph",
                ProportionalLayoutDescriptor {
                    left_a: 0,
                    right_a: 320,
                    // The band is disabled by collapsing its low bound onto
                    // its high bound, so pair B can never be selected.
                    left_b: 0,
                    right_b: 320,
                    band_low: 200,
                    band_high: 200,
                    space_advance: 5,
                    pen_x: 0,
                    pen_y: 152,
                },
            ),
            (
                "result paragraph",
                ProportionalLayoutDescriptor {
                    left_a: 0,
                    right_a: 320,
                    left_b: 0,
                    right_b: 166,
                    band_low: 90,
                    band_high: 200,
                    space_advance: 4,
                    pen_x: 0,
                    pen_y: 0,
                },
            ),
        ];

        let actual = [
            CHARGEN_GYPSY_PARAGRAPH_BOX,
            CHARGEN_QUESTION_PARAGRAPH_BOX,
            CHARGEN_RESULT_PARAGRAPH_BOX,
        ];

        for ((name, expected), got) in published.into_iter().zip(actual) {
            assert_eq!(
                got, expected,
                "{name} descriptor drifted from chargen.md §5.1.2"
            );
        }

        // Line advance is nine pixels for every chargen paragraph.
        assert_eq!(PROPORTIONAL_LINE_STRIDE, 9);
    }

    /// `systems/chargen.md §5.1.2`: margin pair B applies to "every line whose
    /// pen has passed y = 90", so the band must admit a pen at y = 91 and must
    /// not admit one at y = 90 (the band test is strict at both ends).
    #[test]
    fn result_paragraph_band_admits_first_line_past_ninety() {
        let d = CHARGEN_RESULT_PARAGRAPH_BOX;
        let inside = |pen_y: u16| pen_y > d.band_low && pen_y < d.band_high;
        assert!(!inside(90), "y = 90 has not yet passed the band low");
        assert!(
            inside(91),
            "the first line past y = 90 must use margin pair B"
        );
        // With band_low = 93 the pen rows 91 and 92 would wrongly keep the
        // full-width pair A.
        assert_eq!(d.band_low, 90);
    }
}

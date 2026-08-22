//! Observation-derived proportional paragraph layout for the intro story
//! slides (`cleak/u5-spec#70`).
//!
//! `systems/text-output.md §8` publishes the proportional renderer's byte
//! contract (space is a wrap candidate, line feed is a hard newline,
//! underscore is a soft hyphen, left brace is a paragraph marker) but
//! explicitly leaves the paragraph rectangle, the line stride, the paragraph
//! indent, and whether lines are justified to the caller, and
//! `systems/intro.md §10` publishes every story-art placement without ever
//! publishing the narrative-text rectangle.
//!
//! Everything in this module was recovered by black-box measurement of the
//! original running the `U` introduction: each of the twenty story slides was
//! captured, the glyph bitmaps decoded from the local `PROPORT.PCS` were
//! matched against the captured pixels, and every glyph's pixel position was
//! read off. The rules below reproduce all 8,995 measured glyph positions
//! exactly. No decompiled source, disassembly, or private analysis was used,
//! and no story text is stored here - the text is read from `STORY.DAT` at
//! runtime.

use std::io;

use crate::{
    INTRO_INLINE_DOORWAY_STEP, INTRO_STORY_STEP_COUNT, PCS_GLYPH_HEIGHT, PCS_SPACE_ADVANCE,
    ProportionalWidthTable, STORY_HARD_NEWLINE_MARKER, STORY_PARAGRAPH_START_MARKER,
    STORY_RECORD_END_MARKER, STORY_SOFT_BREAK_MARKER,
};

/// Observation-derived (`cleak/u5-spec#70`): consecutive proportional text
/// lines are 9 pixel rows apart, i.e. the 8-row glyph cell plus one blank
/// separator row. Measured on every multi-line slide.
pub const PROPORTIONAL_LINE_STRIDE: u16 = PCS_GLYPH_HEIGHT as u16 + 1;

/// Observation-derived (`cleak/u5-spec#70`): the `{` paragraph marker
/// indents the line it opens by 15 pixels from the line's left edge (three
/// natural space advances). Measured on every paragraph opening in the
/// twenty story records.
pub const PROPORTIONAL_PARAGRAPH_INDENT: u16 = 3 * PCS_SPACE_ADVANCE as u16;

/// Observation-derived (`cleak/u5-spec#70`): full-width story text runs from
/// pixel column 0 through column 318 inclusive. Justified full-width lines
/// end their last glyph exactly on column 318 on every slide.
pub const INTRO_STORY_TEXT_LEFT: u16 = 0;
pub const INTRO_STORY_TEXT_RIGHT: u16 = 318;

/// A horizontal band of a text region where the art panel narrows the usable
/// columns. Lines whose top row `y` satisfies `top_y <= y <= bottom_y` are
/// laid out between `left` and `right` inclusive; every other line uses the
/// region's full-width bounds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProportionalTextGutter {
    pub top_y: u16,
    pub bottom_y: u16,
    pub left: u16,
    pub right: u16,
}

/// The paragraph rectangle for one text-consuming intro story step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProportionalTextRegion {
    /// Pixel row of the first text line, before any leading `\n` in the
    /// record advances the cursor.
    pub top_y: u16,
    pub left: u16,
    pub right: u16,
    /// Optional narrowed band beside the step's story art.
    pub gutter: Option<ProportionalTextGutter>,
    /// Left edge used by the record's very first line when it differs from
    /// the band that contains it. Only intro story step 18 needs this.
    pub first_line_left: Option<u16>,
    /// Width of an unstretched space, in pixels.
    ///
    /// Observation-derived (`cleak/u5-spec#70`): this is not a font constant.
    /// Every intro story slide, the character-creation gypsy narrative, and
    /// the questionnaire prompts advance a natural space by
    /// [`PCS_SPACE_ADVANCE`] = 5 pixels, but the character-creation result
    /// screen advances it by 4 - measured on its unjustified paragraph-final
    /// lines, and confirmed by the fact that the 4-pixel space reproduces all
    /// twenty-two of that screen's measured line breaks while the 5-pixel one
    /// does not.
    pub space_advance: u8,
}

impl ProportionalTextRegion {
    pub const fn full_width(top_y: u16) -> Self {
        Self {
            top_y,
            left: INTRO_STORY_TEXT_LEFT,
            right: INTRO_STORY_TEXT_RIGHT,
            gutter: None,
            first_line_left: None,
            space_advance: PCS_SPACE_ADVANCE,
        }
    }

    pub const fn with_gutter(top_y: u16, gutter: ProportionalTextGutter) -> Self {
        Self {
            top_y,
            left: INTRO_STORY_TEXT_LEFT,
            right: INTRO_STORY_TEXT_RIGHT,
            gutter: Some(gutter),
            first_line_left: None,
            space_advance: PCS_SPACE_ADVANCE,
        }
    }

    /// Inclusive `(left, right)` pixel columns available to the line whose
    /// top row is `y`.
    pub fn line_bounds(&self, y: u16) -> (u16, u16) {
        match self.gutter {
            Some(gutter) if y >= gutter.top_y && y <= gutter.bottom_y => {
                (gutter.left, gutter.right)
            }
            _ => (self.left, self.right),
        }
    }
}

const fn gutter(top_y: u16, bottom_y: u16, left: u16, right: u16) -> ProportionalTextGutter {
    ProportionalTextGutter {
        top_y,
        bottom_y,
        left,
        right,
    }
}

/// Observation-derived paragraph rectangles for the twenty-one intro story
/// steps (`cleak/u5-spec#70`). Step 6 has no entry because it renders the
/// unpublished inline doorway lines rather than a `STORY.DAT` record
/// (`cleak/u5-spec#69`).
///
/// Each entry was read off a capture of the original: `top_y` is the pixel
/// row of the record's first line minus 9 per leading `\n` in the record,
/// and the gutter's `left`/`right` are the exact justified line edges
/// measured beside that step's story art. The gutter's vertical bounds sit
/// between the last observed full-width line and the first observed narrowed
/// line (or vice versa); where the published art rectangle from
/// `systems/intro.md §10` falls inside that interval the art-derived value is
/// used, and the exceptions are called out per step.
const INTRO_STORY_TEXT_REGIONS: [Option<ProportionalTextRegion>; INTRO_STORY_STEP_COUNT] = [
    // Step 0: art STORY1.16 #0 at (0, 0), 176x192. Text starts at y=128
    // beside the art at x 180..318; the last two lines (y=182, y=191) run
    // full width straight over the bottom of the art, so the gutter bottom
    // is not the art bottom (191) but sits in 174..181.
    Some(ProportionalTextRegion::with_gutter(
        128,
        gutter(0, 181, 180, 318),
    )),
    // Step 1: art STORY1.16 #1 at (0, 74), 168x126. Lines y=9..63 are full
    // width, y=72..171 sit at x 172..318. Gutter top 64 = art top - 10.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(64, 199, 172, 318),
    )),
    // Step 2: art STORY2.16 #0 at (136, 0), 184x131. Lines y=67..130 sit at
    // x 0..130, y=139..166 run full width. Gutter bottom 133 = art bottom+3.
    Some(ProportionalTextRegion::with_gutter(
        40,
        gutter(0, 133, 0, 130),
    )),
    // Step 3: art STORY2.16 #1 at (0, 38), 200x121. Lines y=0..27 and
    // y=162..180 are full width; y=36..153 sit at x 210..318. Gutter bounds
    // 28 = art top - 10 and 161 = art bottom + 3.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(28, 161, 210, 318),
    )),
    // Step 4: art STORY2.16 #2 at (152, 76), 168x124. Lines y=18..72 are
    // full width, y=81..189 sit at x 0..146. Gutter top 66 = art top - 10.
    Some(ProportionalTextRegion::with_gutter(
        9,
        gutter(66, 199, 0, 146),
    )),
    // Step 5: art STORY2.16 #2 at (0, 0), 168x124. Lines y=0..126 sit at
    // x 176..318, y=135..171 run full width. Gutter bottom 126 = art
    // bottom + 3.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(0, 126, 176, 318),
    )),
    // Step 6 renders the unpublished inline doorway lines; see
    // `cleak/u5-spec#69`.
    None,
    // Step 7: art STORY3.16 #0 at (0, 0), 183x167. Lines y=136..163 sit at
    // x 188..318, y=172..181 run full width. Gutter bottom 169 = art
    // bottom + 3.
    Some(ProportionalTextRegion::with_gutter(
        136,
        gutter(0, 169, 188, 318),
    )),
    // Steps 8-12 draw a full-width 320x118 panel at (0, 82); all of their
    // text sits above it and runs full width.
    Some(ProportionalTextRegion::full_width(0)),
    Some(ProportionalTextRegion::full_width(0)),
    Some(ProportionalTextRegion::full_width(0)),
    Some(ProportionalTextRegion::full_width(0)),
    Some(ProportionalTextRegion::full_width(0)),
    // Step 13: art STORY6.16 #0 at (176, 0), 144x112. Lines y=0..108 sit at
    // x 0..168, y=117..171 run full width. Gutter bottom 114 = art
    // bottom + 3.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(0, 114, 0, 168),
    )),
    // Step 14: art STORY6.16 #1 at (0, 0), 176x113. Lines y=41..113 sit at
    // x 184..318, y=122..185 run full width. Gutter bottom 115 = art
    // bottom + 3.
    Some(ProportionalTextRegion::with_gutter(
        32,
        gutter(0, 115, 184, 318),
    )),
    // Step 15: art STORY6.16 #2 at (176, 0) plus the secondary #3 at
    // (176, 55), together 141x94. Lines y=0..90 sit at x 0..168, y=99..162
    // run full width. Gutter bottom 96 = combined art bottom + 3.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(0, 96, 0, 168),
    )),
    // Step 16: art STORY6.16 #6 at (0, 46) plus the secondary #5 at
    // (0, 101), together 141x94. Lines y=0..27 and y=144..171 are full
    // width; y=36..135 sit at x 148..318. Gutter bounds 36 and 142 = combined
    // art bottom + 3; the gutter starts one line above `art top - 10`.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(36, 142, 148, 318),
    )),
    // Step 17: art STORY6.16 #4 at (176, 78) plus the secondary #7 at
    // (176, 133). Lines y=9..54 are full width, y=63..162 sit at x 0..168.
    // The gutter starts at 68, above the art top, so the observed interval
    // (64..72) rather than a single art-derived offset fixes it.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(68, 199, 0, 168),
    )),
    // Step 18: art STORY6.16 #2 at (0, 0) plus the secondary #5 at (0, 55),
    // together 141x94. Lines y=9..90 sit at x 148..318, y=99..180 run full
    // width. Gutter bottom 93 = combined art bottom. The record's first line
    // (y=0) is the one measured line in the whole sequence that does not
    // start at its band's left edge: it is justified from x=174 rather than
    // x=148, so it carries an explicit first-line left edge.
    Some(ProportionalTextRegion {
        top_y: 0,
        left: INTRO_STORY_TEXT_LEFT,
        right: INTRO_STORY_TEXT_RIGHT,
        gutter: Some(gutter(0, 93, 148, 318)),
        first_line_left: Some(174),
        space_advance: PCS_SPACE_ADVANCE,
    }),
    // Step 19: art STORY6.16 #6 at (176, 55) plus the secondary #3 at
    // (176, 110), together 141x94. Lines y=9..45 and y=153..171 are full
    // width; y=54..144 sit at x 0..168. Gutter bounds 46 = art top - 9 and
    // 151 = combined art bottom + 3.
    Some(ProportionalTextRegion::with_gutter(
        9,
        gutter(46, 151, 0, 168),
    )),
    // Step 20: art STORY6.16 #4 at (0, 87) plus the secondary #3 at
    // (0, 142), together 141x94. Lines y=9..72 are full width, y=81..171 sit
    // at x 156..318. Gutter top 73 = art top - 14.
    Some(ProportionalTextRegion::with_gutter(
        0,
        gutter(73, 199, 156, 318),
    )),
];

/// Paragraph rectangle for a text-consuming intro story step, or `None` for
/// the inline-doorway step and out-of-range steps.
pub fn intro_story_text_region(step: usize) -> Option<ProportionalTextRegion> {
    if step == INTRO_INLINE_DOORWAY_STEP {
        return None;
    }
    INTRO_STORY_TEXT_REGIONS.get(step).copied().flatten()
}

/// One laid-out glyph: the byte to draw and its top-left pixel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlacedProportionalGlyph {
    pub x: u16,
    pub y: u16,
    pub code: u8,
}

#[derive(Clone, Debug)]
enum ParagraphItem {
    ParagraphStart,
    HardNewline,
    Word(Vec<u8>),
}

fn paragraph_items(text: &[u8]) -> Vec<ParagraphItem> {
    let mut items = Vec::new();
    let mut word = Vec::new();
    for &byte in text {
        if byte == STORY_RECORD_END_MARKER {
            break;
        }
        match byte {
            STORY_PARAGRAPH_START_MARKER | STORY_HARD_NEWLINE_MARKER => {
                if !word.is_empty() {
                    items.push(ParagraphItem::Word(std::mem::take(&mut word)));
                }
                items.push(if byte == STORY_PARAGRAPH_START_MARKER {
                    ParagraphItem::ParagraphStart
                } else {
                    ParagraphItem::HardNewline
                });
            }
            b' ' => {
                if !word.is_empty() {
                    items.push(ParagraphItem::Word(std::mem::take(&mut word)));
                }
            }
            _ => word.push(byte),
        }
    }
    if !word.is_empty() {
        items.push(ParagraphItem::Word(word));
    }
    items
}

/// Renderings a word can take on one line, longest first: the whole word with
/// its soft-break markers removed, then each soft-break prefix with a hyphen
/// appended plus the remainder that moves to the next line.
fn word_renderings(word: &[u8]) -> Vec<(Vec<u8>, Option<Vec<u8>>)> {
    let segments: Vec<&[u8]> = word.split(|&b| b == STORY_SOFT_BREAK_MARKER).collect();
    let mut out = Vec::with_capacity(segments.len());
    out.push((segments.concat(), None));
    for split in (1..segments.len()).rev() {
        let mut head = segments[..split].concat();
        head.push(b'-');
        let mut tail = Vec::new();
        for (index, segment) in segments[split..].iter().enumerate() {
            if index > 0 {
                tail.push(STORY_SOFT_BREAK_MARKER);
            }
            tail.extend_from_slice(segment);
        }
        out.push((head, Some(tail)));
    }
    out
}

fn word_advance(widths: &ProportionalWidthTable, word: &[u8]) -> io::Result<usize> {
    let mut total = 0usize;
    for &code in word {
        let advance = widths.width_for_byte(code)?;
        if advance == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "proportional advance table has no advance for code {code}; refusing to lay out unmeasured text"
                ),
            ));
        }
        total = total
            .checked_add(advance)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "line width overflows"))?;
    }
    Ok(total)
}

/// Lays out a NUL-terminated proportional paragraph inside `region`.
///
/// Observation-derived rules (`cleak/u5-spec#70`), all verified against the
/// original's twenty intro story slides:
///
/// * Lines advance by [`PROPORTIONAL_LINE_STRIDE`]; the x range of a line is
///   [`ProportionalTextRegion::line_bounds`] for that line's top row.
/// * `{` indents the line it opens by [`PROPORTIONAL_PARAGRAPH_INDENT`];
///   `\n` ends the current line; `_` marks a legal hyphenation point and
///   renders a `-` when the break is taken there.
/// * A word joins the current line while the line's rightmost ink column,
///   measured with natural [`PCS_SPACE_ADVANCE`] spaces, would stay strictly
///   left of `right`. Otherwise the longest hyphenated prefix that satisfies
///   the same test is taken, and failing that the word wraps whole.
/// * Every line except the last of a paragraph is fully justified: the extra
///   pixels are spread over the line's spaces, `total / count` each with the
///   `total % count` rightmost spaces taking one extra pixel, so the last
///   glyph's rightmost ink column lands exactly on `right`.
pub fn layout_proportional_justified_paragraph(
    widths: &ProportionalWidthTable,
    region: &ProportionalTextRegion,
    text: &[u8],
    bottom_limit: u16,
) -> io::Result<Vec<PlacedProportionalGlyph>> {
    let mut items = paragraph_items(text);
    let mut placed = Vec::new();
    let mut y = region.top_y;
    let mut index = 0usize;
    let mut line: Vec<Vec<u8>> = Vec::new();
    let mut indent = 0u16;
    let mut is_first_line = true;

    let line_left = |y: u16, is_first_line: bool| -> (u16, u16) {
        let (left, right) = region.line_bounds(y);
        match region.first_line_left {
            Some(first_left) if is_first_line => (first_left, right),
            _ => (left, right),
        }
    };

    while index < items.len() && y.saturating_add(PCS_GLYPH_HEIGHT as u16) <= bottom_limit {
        match &items[index] {
            ParagraphItem::ParagraphStart => {
                if line.is_empty() {
                    indent = indent.saturating_add(PROPORTIONAL_PARAGRAPH_INDENT);
                }
                index += 1;
            }
            ParagraphItem::HardNewline => {
                emit_line(
                    widths,
                    region.space_advance,
                    line_left(y, is_first_line),
                    y,
                    indent,
                    &line,
                    false,
                    &mut placed,
                )?;
                line.clear();
                indent = 0;
                is_first_line = false;
                y = y.saturating_add(PROPORTIONAL_LINE_STRIDE);
                index += 1;
            }
            ParagraphItem::Word(word) => {
                let (left, right) = line_left(y, is_first_line);
                let mut chosen: Option<(Vec<u8>, Option<Vec<u8>>)> = None;
                for (rendering, remainder) in word_renderings(word) {
                    let mut trial_advance = word_advance(widths, &rendering)?;
                    for existing in &line {
                        trial_advance += word_advance(widths, existing)?;
                    }
                    let spaces = line.len() * usize::from(region.space_advance);
                    // The line's rightmost ink column with natural spacing is
                    // `left + indent + advances + spaces - 2`; the measured
                    // wrap point keeps that strictly left of `right`.
                    let natural_end =
                        usize::from(left) + usize::from(indent) + trial_advance + spaces;
                    if natural_end <= usize::from(right) + 1 {
                        chosen = Some((rendering, remainder));
                        break;
                    }
                }
                match chosen {
                    Some((rendering, remainder)) => {
                        line.push(rendering);
                        match remainder {
                            None => index += 1,
                            Some(tail) => {
                                items[index] = ParagraphItem::Word(tail);
                                emit_line(
                                    widths,
                                    region.space_advance,
                                    (left, right),
                                    y,
                                    indent,
                                    &line,
                                    true,
                                    &mut placed,
                                )?;
                                line.clear();
                                indent = 0;
                                is_first_line = false;
                                y = y.saturating_add(PROPORTIONAL_LINE_STRIDE);
                            }
                        }
                    }
                    None if line.is_empty() => {
                        // A single word wider than the whole rectangle: place
                        // it unbroken rather than dropping text.
                        line.push(word_renderings(word).swap_remove(0).0);
                        index += 1;
                    }
                    None => {
                        emit_line(
                            widths,
                            region.space_advance,
                            (left, right),
                            y,
                            indent,
                            &line,
                            true,
                            &mut placed,
                        )?;
                        line.clear();
                        indent = 0;
                        is_first_line = false;
                        y = y.saturating_add(PROPORTIONAL_LINE_STRIDE);
                    }
                }
            }
        }
    }

    if !line.is_empty() && y.saturating_add(PCS_GLYPH_HEIGHT as u16) <= bottom_limit {
        emit_line(
            widths,
            region.space_advance,
            line_left(y, is_first_line),
            y,
            indent,
            &line,
            false,
            &mut placed,
        )?;
    }
    Ok(placed)
}

fn emit_line(
    widths: &ProportionalWidthTable,
    space_advance: u8,
    bounds: (u16, u16),
    y: u16,
    indent: u16,
    line: &[Vec<u8>],
    justify: bool,
    placed: &mut Vec<PlacedProportionalGlyph>,
) -> io::Result<()> {
    if line.is_empty() {
        return Ok(());
    }
    let (left, right) = bounds;
    let gaps = line.len() - 1;
    let mut word_pixels = 0usize;
    for word in line {
        word_pixels += word_advance(widths, word)?;
    }
    let start = usize::from(left) + usize::from(indent);
    let total_space = if justify && gaps > 0 {
        (usize::from(right) + 2).saturating_sub(start + word_pixels)
    } else {
        gaps * usize::from(space_advance)
    };
    let base = if gaps > 0 { total_space / gaps } else { 0 };
    let extra = if gaps > 0 { total_space % gaps } else { 0 };

    let mut x = start;
    for (word_index, word) in line.iter().enumerate() {
        for &code in word {
            placed.push(PlacedProportionalGlyph {
                x: u16::try_from(x).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "glyph x exceeds framebuffer")
                })?,
                y,
                code,
            });
            x += widths.width_for_byte(code)?;
        }
        if word_index < gaps {
            x += base + usize::from(word_index >= gaps - extra);
        }
    }
    Ok(())
}

/// Observation-derived paragraph rectangle for the character-creation gypsy
/// narrative (`QUESTION.DAT` record 0, `cleak/u5-spec#70`).
///
/// Measured off a capture of the original: the first line sits at y=9 with
/// the usual 15-pixel paragraph indent and lines run full width to column 318
/// until the `CREATE` opening panel at `(0, 96)` (168x96) takes the left of
/// the screen; from line y=90 the text runs at x 175..318, which is the art's
/// right edge plus eight. The gutter's top is the art-derived
/// `art_top - (glyph height - 1)`, i.e. the first line whose 8-row band
/// touches the panel.
pub const CHARGEN_GYPSY_TEXT_REGION: ProportionalTextRegion =
    ProportionalTextRegion::with_gutter(9, gutter(89, 199, 175, INTRO_STORY_TEXT_RIGHT));

/// Observation-derived (`cleak/u5-spec#70`): the character-creation result
/// screen is the one measured caller whose unstretched space is 4 pixels
/// rather than [`PCS_SPACE_ADVANCE`]'s 5. Measured on its paragraph-final
/// lines, which are never justified.
pub const CHARGEN_RESULT_SPACE_ADVANCE: u8 = 4;

/// Observation-derived paragraph rectangle for the character-creation
/// tournament result text (`QUESTION.DAT` record 1, `cleak/u5-spec#70`).
///
/// Lines y=0..90 run full width; the `CREATE` result panel at `(168, 100)`
/// (152x100) then takes the right of the screen and lines y=99..189 run at
/// x 0..164, the art's left edge minus four.
pub const CHARGEN_RESULT_TEXT_REGION: ProportionalTextRegion = ProportionalTextRegion {
    top_y: 0,
    left: INTRO_STORY_TEXT_LEFT,
    right: INTRO_STORY_TEXT_RIGHT,
    gutter: Some(gutter(93, 199, INTRO_STORY_TEXT_LEFT, 164)),
    first_line_left: None,
    space_advance: CHARGEN_RESULT_SPACE_ADVANCE,
};

/// Observation-derived paragraph rectangle for the character-creation
/// questionnaire prompts (`QUESTION.DAT` dilemma records, `cleak/u5-spec#70`).
///
/// The two `CREATE` incense-bowl backings occupy rows 0..147, so the prompt
/// is a plain full-width four-line block starting at y=152. Dilemma records
/// carry no `{` or `_` markers, so these lines have neither a paragraph
/// indent nor hyphenation.
pub const CHARGEN_QUESTION_TEXT_REGION: ProportionalTextRegion =
    ProportionalTextRegion::full_width(152);

//! Parser for `STORY.DAT`: twenty NUL-terminated text records driving the
//! intro story sequence. Spec: `formats/story-dat.md` §2-§3.

use std::io;
use std::path::Path;

use crate::read_optional_disk_file;

/// `formats/story-dat.md §2` published filename for the intro
/// story sequence's 11,679-byte text file.
pub const STORY_DAT_FILE: &str = "STORY.DAT";
const EXPECTED_RECORD_COUNT: usize = 20;

/// `formats/story-dat.md §2`: shipped DOS file size in bytes.
pub const STORY_DAT_LEN: usize = 11_679;
/// `formats/story-dat.md §2`: number of NUL-terminated text records.
pub const STORY_DAT_RECORDS: usize = 20;

/// `intro.md §10` — total intro narrative steps (zero-based 0..=20).
pub const INTRO_STORY_STEP_COUNT: usize = 21;

/// `formats/story-dat.md §3` renderer-visible marker bytes the
/// proportional-font renderer recognises inside a `STORY.DAT`
/// record's plain-ASCII payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoryTextMarker {
    /// `{` — paragraph or page-start marker. The renderer walks past
    /// it without emitting a glyph; the caller owns the actual
    /// wait-for-key or slide-advance behavior.
    ParagraphStart,
    /// `_` — soft hyphen / syllable break. Permits a line break but
    /// is not rendered as an underscore glyph.
    SoftBreak,
    /// `\n` — hard newline inside the current record.
    HardNewline,
    /// `\0` — end of the current story record. The reader stops
    /// consuming bytes here and advances to the next record.
    RecordEnd,
}

/// `formats/story-dat.md §3` ASCII byte values for the four named
/// markers above. Paragraph-start and soft-break markers are the
/// same `{` / `_` bytes the shared proportional-font paragraph
/// renderer recognises across STORY.DAT / END.DAT / QUESTION.DAT
/// (see formats/font-pcs.md). Anchored to
/// [`crate::QUESTION_PARAGRAPH_START_MARKER`] /
/// [`crate::QUESTION_SOFT_BREAK_MARKER`] so all three
/// renderer-targeted formats share one source of truth.
pub const STORY_PARAGRAPH_START_MARKER: u8 = crate::QUESTION_PARAGRAPH_START_MARKER;
pub const STORY_SOFT_BREAK_MARKER: u8 = crate::QUESTION_SOFT_BREAK_MARKER;
pub const STORY_HARD_NEWLINE_MARKER: u8 = b'\n';
pub const STORY_RECORD_END_MARKER: u8 = 0;

/// `formats/story-dat.md §3`: classify one record-payload byte into
/// its renderer marker, or return `None` for ordinary glyph bytes
/// the renderer prints directly.
pub const fn story_text_marker(byte: u8) -> Option<StoryTextMarker> {
    Some(match byte {
        STORY_PARAGRAPH_START_MARKER => StoryTextMarker::ParagraphStart,
        STORY_SOFT_BREAK_MARKER => StoryTextMarker::SoftBreak,
        STORY_HARD_NEWLINE_MARKER => StoryTextMarker::HardNewline,
        STORY_RECORD_END_MARKER => StoryTextMarker::RecordEnd,
        _ => return None,
    })
}

/// `intro.md §10` — story-art file in use for each zero-based step.
/// Steps 0-1 use STORY1.16; 2-6 use STORY2.16; 7-8 use STORY3.16;
/// 9-10 use STORY4.16; 11-12 use STORY5.16; 13-20 use STORY6.16.
pub const fn intro_story_art_file_for_step(step: usize) -> Option<&'static str> {
    Some(match step {
        0..=1 => "STORY1.16",
        2..=6 => "STORY2.16",
        7..=8 => "STORY3.16",
        9..=10 => "STORY4.16",
        11..=12 => "STORY5.16",
        13..=20 => "STORY6.16",
        _ => return None,
    })
}

/// `intro.md §10` — step 6 uses two inline doorway-transition text lines
/// owned by intro code rather than consuming a `STORY.DAT` record.
pub const INTRO_INLINE_DOORWAY_STEP: usize = 6;

/// `intro.md §10` — step 0 is the automatic opening transition that does
/// not wait for input.
pub const INTRO_AUTO_OPENING_STEP: usize = 0;

/// `intro.md §10` static transition-strip pre-draw steps (each
/// draws two `TEXT.16` transition subimages before the primary
/// story-art draw).
pub const INTRO_TRANSITION_STRIP_STEPS: [usize; 3] = [0, 7, 14];

/// `intro.md §10` secondary `STORY6.16` art-pass steps (each draws
/// a second STORY6 subimage at the primary X coordinate 55 pixels
/// below the primary Y).
pub const INTRO_STORY6_SECONDARY_PASS_STEPS: [usize; 6] = [15, 16, 17, 18, 19, 20];

/// `intro.md §10` Y-pixel delta for the secondary `STORY6.16` pass.
pub const INTRO_STORY6_SECONDARY_Y_DELTA: u16 = 55;

/// `intro.md §10`: returns `true` for steps that require the static
/// `TEXT.16` transition-strip pre-draw before the primary story-art
/// draw.
pub const fn intro_step_has_transition_strip(step: usize) -> bool {
    matches!(step, 0 | 7 | 14)
}

/// `intro.md §10`: returns `true` when the intro story-loop step
/// blocks on a keyboard poll before advancing. Step 0 is the
/// automatic opening transition that advances on its own; every
/// other step in `1..=20` waits for a non-zero key. Steps outside
/// the published `0..=20` range are not produced by the loop and
/// return `false`.
pub const fn intro_story_step_waits_for_input(step: usize) -> bool {
    matches!(step, 1..=20)
}

/// `intro.md §10`: returns `true` for the six secondary
/// `STORY6.16` art-pass steps.
pub const fn intro_step_has_story6_secondary_pass(step: usize) -> bool {
    matches!(step, 15..=20)
}

/// `intro.md §10` secondary `STORY6.16` subimage selected for each
/// art-pass step. Steps 15 and 20 use subimage 3; steps 16 and 18
/// use subimage 5; steps 17 and 19 use subimage 7. Returns `None`
/// for any other step.
pub const fn intro_story6_secondary_subimage(step: usize) -> Option<u8> {
    Some(match step {
        15 | 20 => 3,
        16 | 18 => 5,
        17 | 19 => 7,
        _ => return None,
    })
}

/// `intro.md §10` step 1 post-wait `STORY1.16` extra draw. After the
/// player advances step 1, the intro composes subimage 2 at this fixed
/// pixel coordinate on the hidden surface and reveals it with one
/// blocking rectangle dissolve over [`INTRO_STEP_1_RECT_TRANSITION`]
/// (inclusive on both ends); see [`crate::RectangleDissolve`].
pub const INTRO_STEP_1_EXTRA_ART_X: u16 = 40;
pub const INTRO_STEP_1_EXTRA_ART_Y: u16 = 86;
pub const INTRO_STEP_1_EXTRA_SUBIMAGE: u8 = 2;
pub const INTRO_STEP_1_RECT_TRANSITION: (u16, u16, u16, u16) = (40, 86, 75, 120);
/// `intro.md §10` / `display-driver.md §10`: animated `STARTSC`
/// start/menu loader reveal rectangle. Callers that pass the nonzero
/// animated-loader argument reveal this inclusive rectangle with one
/// blocking rectangle dissolve ([`crate::RectangleDissolve`]) and sample
/// input only after it completes. `cleak/u5-spec#53` withdrew the former
/// left-to-right one-column-per-title-tick reading, and its 320-tick
/// figure, in full.
pub const INTRO_START_MENU_REVEAL_RECT: (u16, u16, u16, u16) = (0, 0, 319, 100);

/// `intro.md §10.1` step 6 extra `STORY2.16` doorway-transition art
/// draw. Step 6 also replaces the usual `STORY.DAT` record with the two
/// published inline doorway lines; see [`crate::INTRO_DOORWAY_LINES`].
pub const INTRO_STEP_6_EXTRA_ART_X: u16 = 96;
pub const INTRO_STEP_6_EXTRA_ART_Y: u16 = 39;
pub const INTRO_STEP_6_EXTRA_SUBIMAGE: u8 = 3;

/// `intro.md §10` static transition-strip pre-draws for the three
/// steps that draw two `TEXT.16` transition subimages before the
/// primary story-art draw. Each entry is `(subimage_a, x_a, y_a,
/// subimage_b, x_b, y_b)`.
pub const INTRO_STEP_0_TRANSITION_STRIPS: [(u8, u16, u16); 2] = [(0, 224, 30), (1, 168, 58)];
pub const INTRO_STEP_7_TRANSITION_STRIPS: [(u8, u16, u16); 2] = [(0, 232, 26), (2, 200, 54)];
pub const INTRO_STEP_14_TRANSITION_STRIPS: [(u8, u16, u16); 2] = [(0, 184, 0), (3, 248, 0)];

/// `intro.md §10`: returns the two `TEXT.16` transition-strip
/// placements for steps 0, 7, and 14; returns `None` for any other
/// step (which has no transition-strip pre-draw).
pub const fn intro_step_transition_strips(step: usize) -> Option<[(u8, u16, u16); 2]> {
    Some(match step {
        0 => INTRO_STEP_0_TRANSITION_STRIPS,
        7 => INTRO_STEP_7_TRANSITION_STRIPS,
        14 => INTRO_STEP_14_TRANSITION_STRIPS,
        _ => return None,
    })
}

/// `intro.md §10` — primary story-art placement for one zero-based intro
/// step. Coordinates use 320-by-200 pixel space with origin at upper-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntroStoryArtPlacement {
    pub subimage: u8,
    pub top_left_x: u16,
    pub top_left_y: u16,
}

const INTRO_STORY_ART_PLACEMENTS: [IntroStoryArtPlacement; INTRO_STORY_STEP_COUNT] = [
    IntroStoryArtPlacement {
        subimage: 0,
        top_left_x: 0,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 1,
        top_left_x: 0,
        top_left_y: 74,
    },
    IntroStoryArtPlacement {
        subimage: 0,
        top_left_x: 136,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 1,
        top_left_x: 0,
        top_left_y: 38,
    },
    IntroStoryArtPlacement {
        subimage: 2,
        top_left_x: 152,
        top_left_y: 76,
    },
    IntroStoryArtPlacement {
        subimage: 2,
        top_left_x: 0,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 2,
        top_left_x: 72,
        top_left_y: 38,
    },
    IntroStoryArtPlacement {
        subimage: 0,
        top_left_x: 0,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 1,
        top_left_x: 0,
        top_left_y: 82,
    },
    IntroStoryArtPlacement {
        subimage: 0,
        top_left_x: 0,
        top_left_y: 82,
    },
    IntroStoryArtPlacement {
        subimage: 1,
        top_left_x: 0,
        top_left_y: 82,
    },
    IntroStoryArtPlacement {
        subimage: 0,
        top_left_x: 0,
        top_left_y: 82,
    },
    IntroStoryArtPlacement {
        subimage: 1,
        top_left_x: 0,
        top_left_y: 82,
    },
    IntroStoryArtPlacement {
        subimage: 0,
        top_left_x: 176,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 1,
        top_left_x: 0,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 2,
        top_left_x: 176,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 6,
        top_left_x: 0,
        top_left_y: 46,
    },
    IntroStoryArtPlacement {
        subimage: 4,
        top_left_x: 176,
        top_left_y: 78,
    },
    IntroStoryArtPlacement {
        subimage: 2,
        top_left_x: 0,
        top_left_y: 0,
    },
    IntroStoryArtPlacement {
        subimage: 6,
        top_left_x: 176,
        top_left_y: 55,
    },
    IntroStoryArtPlacement {
        subimage: 4,
        top_left_x: 0,
        top_left_y: 87,
    },
];

/// `intro.md §10` — return the primary story-art placement for the given
/// zero-based intro step, or `None` for out-of-range steps.
pub const fn intro_story_art_placement_for_step(step: usize) -> Option<IntroStoryArtPlacement> {
    if step < INTRO_STORY_STEP_COUNT {
        Some(INTRO_STORY_ART_PLACEMENTS[step])
    } else {
        None
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoryRecords {
    pub records: Vec<String>,
}

impl StoryRecords {
    pub fn record(&self, index: usize) -> Option<&str> {
        self.records.get(index).map(String::as_str)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, String> {
        self.records.iter()
    }
}

pub fn load_story_records(game_dir: &Path) -> io::Result<Option<StoryRecords>> {
    let path = game_dir.join(STORY_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    parse_story_records(&bytes).map(Some)
}

pub fn parse_story_records(bytes: &[u8]) -> io::Result<StoryRecords> {
    let mut records = Vec::with_capacity(EXPECTED_RECORD_COUNT);
    let mut start = 0;
    for record_index in 0..EXPECTED_RECORD_COUNT {
        let (record, next_start) = read_story_record(bytes, start, record_index)?;
        records.push(record);
        start = next_start;
    }
    validate_story_tail(bytes, start)?;
    Ok(StoryRecords { records })
}

fn read_story_record(
    bytes: &[u8],
    start: usize,
    record_index: usize,
) -> io::Result<(String, usize)> {
    let end = bytes[start..]
        .iter()
        .position(|&b| b == 0x00)
        .map(|offset| start + offset);
    let Some(end) = end else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{STORY_DAT_FILE}: record {record_index} is not NUL-terminated"),
        ));
    };
    if end == start {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{STORY_DAT_FILE}: required record {record_index} is empty"),
        ));
    }
    let record = decode_story_record(record_index, &bytes[start..end])?;
    Ok((record, end + 1))
}

/// Strips the layout markers (`formats/story-dat.md` section 3) from a
/// decoded story record, leaving the prose a plain-text consumer wants.
/// The proportional paragraph renderer uses the marked-up record instead.
pub fn story_record_display_text(record: &str) -> String {
    record
        .chars()
        .filter(|ch| {
            *ch != STORY_PARAGRAPH_START_MARKER as char && *ch != STORY_SOFT_BREAK_MARKER as char
        })
        .collect()
}

fn validate_story_tail(bytes: &[u8], mut start: usize) -> io::Result<()> {
    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|&b| b == 0x00)
            .map(|offset| start + offset);
        let Some(end) = end else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{STORY_DAT_FILE}: extra record starting at byte {start} is not NUL-terminated"
                ),
            ));
        };
        if end > start {
            decode_story_record(EXPECTED_RECORD_COUNT, &bytes[start..end])?;
        }
        start = end + 1;
    }
    Ok(())
}

fn decode_story_record(record_index: usize, bytes: &[u8]) -> io::Result<String> {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x0a | 0x0d => out.push('\n'),
            // formats/story-dat.md §3: `{` paragraph marker and `_` soft
            // hyphen are layout markup rather than visible glyphs.
            // They are preserved rather than dropped because the
            // proportional paragraph layout consumes them: `{` sets the
            // paragraph indent and `_` marks a legal hyphenation point.
            // Plain-text consumers use `story_record_display_text`.
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{STORY_DAT_FILE}: record {record_index} has unsupported byte 0x{byte:02x}"
                    ),
                ));
            }
        }
    }
    if out.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{STORY_DAT_FILE}: record {record_index} decodes to empty text"),
        ));
    }
    Ok(out)
}

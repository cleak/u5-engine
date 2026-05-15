//! Parser for `STORY.DAT`: twenty NUL-terminated text records driving the
//! intro story sequence. Spec: `formats/story-dat.md` §2-§3.

use std::fs;
use std::io;
use std::path::Path;

const STORY_DAT_FILE: &str = "STORY.DAT";
const EXPECTED_RECORD_COUNT: usize = 20;

/// `intro.md §10` — total intro narrative steps (zero-based 0..=20).
pub const INTRO_STORY_STEP_COUNT: usize = 21;

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

/// `intro.md §10` — primary story-art placement for one zero-based intro
/// step. Coordinates use 320-by-200 pixel space with origin at upper-left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntroStoryArtPlacement {
    pub subimage: u8,
    pub top_left_x: u16,
    pub top_left_y: u16,
}

const INTRO_STORY_ART_PLACEMENTS: [IntroStoryArtPlacement; INTRO_STORY_STEP_COUNT] = [
    IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 1, top_left_x: 0, top_left_y: 74 },
    IntroStoryArtPlacement { subimage: 0, top_left_x: 136, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 1, top_left_x: 0, top_left_y: 38 },
    IntroStoryArtPlacement { subimage: 2, top_left_x: 152, top_left_y: 76 },
    IntroStoryArtPlacement { subimage: 2, top_left_x: 0, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 2, top_left_x: 72, top_left_y: 38 },
    IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 1, top_left_x: 0, top_left_y: 82 },
    IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 82 },
    IntroStoryArtPlacement { subimage: 1, top_left_x: 0, top_left_y: 82 },
    IntroStoryArtPlacement { subimage: 0, top_left_x: 0, top_left_y: 82 },
    IntroStoryArtPlacement { subimage: 1, top_left_x: 0, top_left_y: 82 },
    IntroStoryArtPlacement { subimage: 0, top_left_x: 176, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 1, top_left_x: 0, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 2, top_left_x: 176, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 6, top_left_x: 0, top_left_y: 46 },
    IntroStoryArtPlacement { subimage: 4, top_left_x: 176, top_left_y: 78 },
    IntroStoryArtPlacement { subimage: 2, top_left_x: 0, top_left_y: 0 },
    IntroStoryArtPlacement { subimage: 6, top_left_x: 176, top_left_y: 55 },
    IntroStoryArtPlacement { subimage: 4, top_left_x: 0, top_left_y: 87 },
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
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_story_records(&bytes).map(Some)
}

pub fn parse_story_records(bytes: &[u8]) -> io::Result<StoryRecords> {
    let mut records = Vec::with_capacity(EXPECTED_RECORD_COUNT);
    let mut start = 0;
    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|&b| b == 0x00)
            .map(|offset| start + offset);
        let Some(end) = end else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{STORY_DAT_FILE}: unterminated record starting at byte {start}"
                ),
            ));
        };
        records.push(decode_story_record(&bytes[start..end]));
        start = end + 1;
    }
    while records.last().is_some_and(|record| record.is_empty()) {
        records.pop();
    }
    if records.len() < EXPECTED_RECORD_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{STORY_DAT_FILE}: expected {EXPECTED_RECORD_COUNT} records, found {}",
                records.len()
            ),
        ));
    }
    Ok(StoryRecords { records })
}

fn decode_story_record(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x0a | 0x0d => out.push('\n'),
            // formats/story-dat.md §3: `{` paragraph marker and `_` soft
            // hyphen are layout markup, not visible glyphs.
            b'{' | b'_' => {}
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

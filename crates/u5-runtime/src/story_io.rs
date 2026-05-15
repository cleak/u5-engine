//! Parser for `STORY.DAT`: twenty NUL-terminated text records driving the
//! intro story sequence. Spec: `formats/story-dat.md` §2-§3.

use std::fs;
use std::io;
use std::path::Path;

const STORY_DAT_FILE: &str = "STORY.DAT";
const EXPECTED_RECORD_COUNT: usize = 20;

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

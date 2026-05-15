//! Parser for `QUESTION.DAT`: 30 NUL-terminated chargen narrative + dilemma
//! records. Spec: `formats/question-dat.md` §2-§4.
//!
//! Records 0-1 are the gypsy arrival narrative and the gypsy invitation;
//! records 2..=29 are the 28 A/B virtue dilemma paragraphs in the order
//! produced by the resident symmetric pair table (Honesty/Compassion through
//! Spirituality/Humility).
//!
//! The decoder strips the proportional-font paragraph marker (`{`) and the
//! soft-hyphen syllable-break marker (`_`) per §3, so the returned records
//! are display-ready prose.

use std::fs;
use std::io;
use std::path::Path;

const QUESTION_DAT_FILE: &str = "QUESTION.DAT";
const EXPECTED_RECORD_COUNT: usize = 30;
const FIRST_DILEMMA_RECORD: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionRecords {
    pub records: Vec<String>,
}

impl QuestionRecords {
    pub fn gypsy_arrival(&self) -> Option<&str> {
        self.records.first().map(String::as_str)
    }

    pub fn gypsy_invitation(&self) -> Option<&str> {
        self.records.get(1).map(String::as_str)
    }

    pub fn dilemma(&self, record_ordinal: usize) -> Option<&str> {
        self.records.get(record_ordinal).map(String::as_str)
    }

    pub fn dilemmas(&self) -> &[String] {
        let start = FIRST_DILEMMA_RECORD.min(self.records.len());
        &self.records[start..]
    }
}

pub fn load_question_records(game_dir: &Path) -> io::Result<Option<QuestionRecords>> {
    let path = game_dir.join(QUESTION_DAT_FILE);
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
    parse_question_records(&bytes).map(Some)
}

pub fn parse_question_records(bytes: &[u8]) -> io::Result<QuestionRecords> {
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
                    "{QUESTION_DAT_FILE}: unterminated record starting at byte {start}"
                ),
            ));
        };
        records.push(decode_question_record(&bytes[start..end]));
        start = end + 1;
    }
    while records.last().is_some_and(|record| record.is_empty()) {
        records.pop();
    }
    if records.len() < EXPECTED_RECORD_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{QUESTION_DAT_FILE}: expected {EXPECTED_RECORD_COUNT} records, found {}",
                records.len()
            ),
        ));
    }
    Ok(QuestionRecords { records })
}

fn decode_question_record(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x0a | 0x0d => out.push('\n'),
            // formats/question-dat.md §3: `{` is a paragraph-start renderer
            // marker and `_` is a soft hyphen / syllable-break marker; both
            // are layout markup, not visible glyphs.
            b'{' | b'_' => {}
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

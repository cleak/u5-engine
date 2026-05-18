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

/// `formats/question-dat.md §2` published filename. Chargen reads
/// virtue-pair dilemma paragraphs out of this file.
pub const QUESTION_DAT_FILE: &str = "QUESTION.DAT";
const EXPECTED_RECORD_COUNT: usize = 30;
const FIRST_DILEMMA_RECORD: usize = 2;

/// `formats/question-dat.md §4`: ordinal record number for a sorted
/// virtue pair `(a, b)` with `a < b` and both indices in `0..=7`. The
/// lexicographic walk starts at record 2 (Honesty/Compassion) and
/// advances through every unique unordered pair to record 29
/// (Spirituality/Humility). Returns `None` for `a >= b`, `a > 7`, or
/// `b > 7`.
pub const fn question_dat_dilemma_record_for_pair(
    sorted_lo_virtue: usize,
    sorted_hi_virtue: usize,
) -> Option<usize> {
    if sorted_lo_virtue >= sorted_hi_virtue {
        return None;
    }
    if sorted_hi_virtue > 7 {
        return None;
    }
    // Sum of pair-counts skipped before the lo-virtue's row: walking
    // virtues 0..lo, each contributes (7 - virtue) records.
    // a*(15-a)/2 closed form.
    let prior = sorted_lo_virtue * (15 - sorted_lo_virtue) / 2;
    let within_row = sorted_hi_virtue - sorted_lo_virtue - 1;
    Some(QUESTION_DAT_FIRST_DILEMMA_RECORD + prior + within_row)
}

/// `formats/question-dat.md §2`: total NUL-terminated record count
/// (two leading narrative records plus 28 virtue-dilemma
/// paragraphs). Anchored to
/// [`QUESTION_DAT_FIRST_DILEMMA_RECORD`] +
/// [`QUESTION_DAT_DILEMMA_COUNT`] so the total and the partition
/// stay one value.
pub const QUESTION_DAT_RECORDS: usize =
    QUESTION_DAT_FIRST_DILEMMA_RECORD + QUESTION_DAT_DILEMMA_COUNT;
/// `formats/question-dat.md §2`: first dilemma record (records 0 and
/// 1 are the gypsy arrival narrative and the gypsy invitation).
pub const QUESTION_DAT_FIRST_DILEMMA_RECORD: usize = 2;
/// `formats/question-dat.md §4`: number of virtue-dilemma paragraphs
/// (`C(8,2) = 28`). One paragraph per unordered pair of distinct
/// virtues, so the count is `VIRTUE_COUNT * (VIRTUE_COUNT - 1) / 2`
/// = `8 * 7 / 2` = 28. Anchored to [`crate::VIRTUE_COUNT`] so the
/// dilemma count derives from the published virtue count.
pub const QUESTION_DAT_DILEMMA_COUNT: usize = crate::VIRTUE_COUNT * (crate::VIRTUE_COUNT - 1) / 2;
/// `systems/chargen.md §5`: shipped `QUESTION.DAT` size in bytes.
/// The thirty NUL-terminated text records pack to exactly this
/// total in the DOS data set. A byte-compatible reader should still
/// scan to the published record count rather than relying on this
/// number to delimit records.
pub const QUESTION_DAT_LEN: usize = 7_746;

/// `formats/question-dat.md §3` paragraph/page-start marker consumed
/// by the proportional-font paragraph renderer. The renderer walks
/// past it without emitting a glyph; it is layout markup only.
pub const QUESTION_PARAGRAPH_START_MARKER: u8 = b'{';
/// `formats/question-dat.md §3` soft-hyphen / syllable-break marker.
/// Gives the renderer an additional wrap point inside a word; not
/// emitted as an underscore glyph.
pub const QUESTION_SOFT_BREAK_MARKER: u8 = b'_';

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
                format!("{QUESTION_DAT_FILE}: unterminated record starting at byte {start}"),
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
            // formats/question-dat.md §3: paragraph-start and soft-hyphen
            // markers are layout markup, not visible glyphs.
            QUESTION_PARAGRAPH_START_MARKER | QUESTION_SOFT_BREAK_MARKER => {}
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

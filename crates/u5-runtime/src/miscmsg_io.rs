//! Parser for `MISCMSG.DAT`: forty-seven NUL-terminated records used by the
//! Blackthorn audience cluster, shrine/virtue presentation, and Codex/urn
//! prophecy pages. Spec: `formats/miscmsg-dat.md` §2-§4.
//!
//! The Codex/prophecy records use a tile-glyph convention (`@` inter-word
//! space, `[` TH digraph, `]` NG digraph, `_` ER digraph). The parser keeps
//! those bytes intact in the returned record so the caller can route them
//! through the appropriate renderer (the sign-style tile-glyph path) versus
//! the ordinary prose printer.

use std::fs;
use std::io;
use std::path::Path;

const MISCMSG_DAT_FILE: &str = "MISCMSG.DAT";
const EXPECTED_RECORD_COUNT: usize = 47;

/// `formats/miscmsg-dat.md §2`: shipped DOS file size in bytes.
pub const MISCMSG_DAT_LEN: usize = 2_745;
/// `formats/miscmsg-dat.md §2`: NUL-terminated record count.
pub const MISCMSG_DAT_RECORDS: usize = 47;

/// `formats/miscmsg-dat.md §3` consumer cluster a record index belongs
/// to. The clusters are consumer contracts, not in-file structure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MiscMsgFamily {
    /// Records 0..=11 — Blackthorn capture audience templates and
    /// punishment/release presentation text.
    BlackthornAudience,
    /// Records 12..=19 — virtue-failing or weakness phrases keyed by
    /// the eight virtues.
    VirtueWeaknessPhrases,
    /// Records 20..=27 — virtue aphorism paragraphs keyed by the
    /// eight virtues.
    VirtueAphorisms,
    /// Records 28..=35 — shrine meditation prompts/altar/ordained
    /// presentation.
    ShrineMeditation,
    /// Records 36..=46 — urn / Codex prophecy pages, including
    /// tile-glyph text.
    UrnCodexProphecy,
}

/// `formats/miscmsg-dat.md §3`: classify a record index `0..=46` into
/// its consumer cluster. Returns `None` for indices outside the file.
pub const fn miscmsg_family(record_index: usize) -> Option<MiscMsgFamily> {
    Some(match record_index {
        0..=11 => MiscMsgFamily::BlackthornAudience,
        12..=19 => MiscMsgFamily::VirtueWeaknessPhrases,
        20..=27 => MiscMsgFamily::VirtueAphorisms,
        28..=35 => MiscMsgFamily::ShrineMeditation,
        36..=46 => MiscMsgFamily::UrnCodexProphecy,
        _ => return None,
    })
}

const BLACKTHORN_AUDIENCE_RANGE: std::ops::RangeInclusive<usize> = 0..=11;
const VIRTUE_FAILING_RANGE: std::ops::RangeInclusive<usize> = 12..=19;
const VIRTUE_APHORISM_RANGE: std::ops::RangeInclusive<usize> = 20..=27;
const SHRINE_MEDITATION_RANGE: std::ops::RangeInclusive<usize> = 28..=35;
const URN_CODEX_RANGE: std::ops::RangeInclusive<usize> = 36..=46;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiscMessages {
    pub records: Vec<String>,
}

impl MiscMessages {
    pub fn record(&self, index: usize) -> Option<&str> {
        self.records.get(index).map(String::as_str)
    }

    pub fn blackthorn_audience(&self) -> &[String] {
        slice_range(&self.records, &BLACKTHORN_AUDIENCE_RANGE)
    }

    pub fn virtue_failing_text(&self) -> &[String] {
        slice_range(&self.records, &VIRTUE_FAILING_RANGE)
    }

    pub fn virtue_aphorism(&self) -> &[String] {
        slice_range(&self.records, &VIRTUE_APHORISM_RANGE)
    }

    pub fn shrine_meditation(&self) -> &[String] {
        slice_range(&self.records, &SHRINE_MEDITATION_RANGE)
    }

    pub fn urn_codex(&self) -> &[String] {
        slice_range(&self.records, &URN_CODEX_RANGE)
    }
}

fn slice_range<'a>(
    records: &'a [String],
    range: &std::ops::RangeInclusive<usize>,
) -> &'a [String] {
    let start = (*range.start()).min(records.len());
    let end = (*range.end() + 1).min(records.len());
    if start >= end {
        return &[];
    }
    &records[start..end]
}

pub fn load_misc_messages(game_dir: &Path) -> io::Result<Option<MiscMessages>> {
    let path = game_dir.join(MISCMSG_DAT_FILE);
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
    parse_misc_messages(&bytes).map(Some)
}

pub fn parse_misc_messages(bytes: &[u8]) -> io::Result<MiscMessages> {
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
                    "{MISCMSG_DAT_FILE}: unterminated record starting at byte {start}"
                ),
            ));
        };
        records.push(decode_misc_record(&bytes[start..end]));
        start = end + 1;
    }
    if records.len() < EXPECTED_RECORD_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{MISCMSG_DAT_FILE}: expected at least {EXPECTED_RECORD_COUNT} records, found {}",
                records.len()
            ),
        ));
    }
    Ok(MiscMessages { records })
}

fn decode_misc_record(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x0a | 0x0d => out.push('\n'),
            // Tile-glyph and printable ASCII bytes pass through; the caller
            // decides whether to render them through the prose printer or the
            // sign-style tile-glyph renderer per the spec.
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

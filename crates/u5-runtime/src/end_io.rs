//! Loader/decoder for `END.DAT`: ~3,698 bytes of narrative text used by the
//! endgame's six fixed final-presentation windows. Spec:
//! `formats/end-dat.md` §2-§4.
//!
//! The on-disk asset has no in-file table. The consumer (the endgame's
//! final-presentation helper) supplies a file-relative seek window. This
//! module exposes the decoded full text plus a `decode_end_window` helper
//! that strips the proportional-text layout markers (`{` page/paragraph,
//! `_` soft hyphen) for any byte slice the caller selects.

use std::fs;
use std::io;
use std::path::Path;

use crate::parse_u8_literal;

/// `formats/end-dat.md §2` published filename for the final-narrative
/// text file.
pub const END_DAT_FILE: &str = "END.DAT";
/// Optional clean sidecar for the six caller-selected `END.DAT` seek windows.
/// The public spec names the six semantic windows but does not publish their
/// byte ranges, so this table lets the runtime render cleanly provided ranges
/// without inferring them from layout markers.
pub const END_NARRATIVE_WINDOW_TABLE_FILE: &str = "end_narrative_windows.tsv";

/// `formats/end-dat.md §2`: shipped DOS file size in bytes.
pub const END_DAT_LEN: usize = 3_698;
/// `formats/end-dat.md §4`: number of fixed final-presentation
/// windows the endgame helper selects from the loaded text.
pub const END_DAT_WINDOW_COUNT: usize = 6;

/// `formats/end-dat.md §3` page/paragraph-start marker the
/// proportional-font renderer walks past without emitting a glyph.
/// Identical convention to QUESTION_PARAGRAPH_START_MARKER (the
/// same renderer handles END.DAT and QUESTION.DAT — see
/// formats/font-pcs.md). Anchored to
/// [`crate::QUESTION_PARAGRAPH_START_MARKER`] so the two
/// renderer-targeted formats share one paragraph-marker byte.
pub const END_PARAGRAPH_START_MARKER: u8 = crate::QUESTION_PARAGRAPH_START_MARKER;
/// `formats/end-dat.md §3` soft-hyphen / syllable-break marker the
/// proportional-font renderer treats as a line-break opportunity
/// without rendering it as an underscore glyph. Anchored to
/// [`crate::QUESTION_SOFT_BREAK_MARKER`] for the same reason.
pub const END_SOFT_BREAK_MARKER: u8 = crate::QUESTION_SOFT_BREAK_MARKER;

/// `formats/end-dat.md §4` semantic role for each fixed window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndNarrativeWindow {
    /// Window 1 — return-home opening at the circle of stones.
    ReturnHomeOpening,
    /// Window 2 — Avatar's homecoming and laying down the long quest.
    Homecoming,
    /// Window 3 — restless night after returning home.
    RestlessNight,
    /// Window 4 — Blackthorn's closing judgment scene opens.
    BlackthornJudgmentOpen,
    /// Window 5 — Blackthorn's sentence and choice continues.
    BlackthornSentence,
    /// Window 6 — orb/gate exile resolution and final Blackthorn
    /// departure.
    OrbExileResolution,
}

impl EndNarrativeWindow {
    /// Spec one-based window number.
    pub const fn number(self) -> u8 {
        match self {
            EndNarrativeWindow::ReturnHomeOpening => 1,
            EndNarrativeWindow::Homecoming => 2,
            EndNarrativeWindow::RestlessNight => 3,
            EndNarrativeWindow::BlackthornJudgmentOpen => 4,
            EndNarrativeWindow::BlackthornSentence => 5,
            EndNarrativeWindow::OrbExileResolution => 6,
        }
    }

    /// `systems/endgame.md §8` narrative-arc group this window belongs
    /// to. Windows 1-3 form the return-home arc; windows 4-6 form the
    /// Blackthorn judgment and gate arc.
    pub const fn group(self) -> EndNarrativeGroup {
        match self {
            EndNarrativeWindow::ReturnHomeOpening
            | EndNarrativeWindow::Homecoming
            | EndNarrativeWindow::RestlessNight => EndNarrativeGroup::ReturnHome,
            EndNarrativeWindow::BlackthornJudgmentOpen
            | EndNarrativeWindow::BlackthornSentence
            | EndNarrativeWindow::OrbExileResolution => EndNarrativeGroup::BlackthornJudgment,
        }
    }
}

/// `systems/endgame.md §8` two-group narrative split inside the six
/// fixed `END.DAT` windows. The endgame draws the return-home arc
/// first, then the Blackthorn judgment / orb arc, with blocking
/// waits between narrative beats.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndNarrativeGroup {
    /// Windows 1-3 — the Avatar returns from Britannia to the
    /// familiar circle of stones, enters the old home, and confronts
    /// the emotional aftermath.
    ReturnHome,
    /// Windows 4-6 — Lord British and Blackthorn share the closing
    /// judgment scene, the Orb/Gate choice is presented, and
    /// Blackthorn's exile resolution is shown.
    BlackthornJudgment,
}

/// `formats/end-dat.md §4`: classify a one-based window number into
/// the published role.
pub const fn end_narrative_window(number: u8) -> Option<EndNarrativeWindow> {
    Some(match number {
        1 => EndNarrativeWindow::ReturnHomeOpening,
        2 => EndNarrativeWindow::Homecoming,
        3 => EndNarrativeWindow::RestlessNight,
        4 => EndNarrativeWindow::BlackthornJudgmentOpen,
        5 => EndNarrativeWindow::BlackthornSentence,
        6 => EndNarrativeWindow::OrbExileResolution,
        _ => return None,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndNarrativeWindowRange {
    /// One-based window number, `1..=6`.
    pub window: u8,
    /// File-relative start byte, inclusive.
    pub start: usize,
    /// File-relative end byte, exclusive.
    pub end: usize,
}

impl EndNarrativeWindowRange {
    pub const fn index(self) -> usize {
        (self.window - 1) as usize
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndNarrative {
    pub raw: Vec<u8>,
    pub window_ranges: [Option<EndNarrativeWindowRange>; END_DAT_WINDOW_COUNT],
}

impl EndNarrative {
    pub fn new(raw: Vec<u8>) -> Self {
        Self {
            raw,
            window_ranges: [None; END_DAT_WINDOW_COUNT],
        }
    }

    pub fn full_text(&self) -> String {
        decode_end_window(&self.raw)
    }

    pub fn window(&self, start: usize, end: usize) -> Option<String> {
        if end > self.raw.len() || start > end {
            return None;
        }
        let text = decode_end_window(&self.raw[start..end]);
        if text.is_empty() { None } else { Some(text) }
    }

    pub fn window_by_number(&self, number: u8) -> Option<String> {
        let window = end_narrative_window(number)?;
        let range = self.window_ranges[window.number() as usize - 1]?;
        self.window(range.start, range.end)
    }

    pub fn with_window_ranges(
        mut self,
        ranges: [Option<EndNarrativeWindowRange>; END_DAT_WINDOW_COUNT],
    ) -> Self {
        self.window_ranges = ranges;
        self
    }
}

pub fn load_end_narrative(game_dir: &Path) -> io::Result<Option<EndNarrative>> {
    let path = game_dir.join(END_DAT_FILE);
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
    let mut narrative = parse_end_narrative(&bytes)?;
    if let Some(ranges) = load_end_narrative_window_ranges(game_dir)? {
        narrative = narrative.with_window_ranges(ranges);
    }
    Ok(Some(narrative))
}

pub fn require_end_narrative(game_dir: &Path) -> io::Result<EndNarrative> {
    load_end_narrative(game_dir)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "{}: required endgame narrative resource is missing",
                game_dir.join(END_DAT_FILE).display()
            ),
        )
    })
}

pub fn parse_end_narrative(bytes: &[u8]) -> io::Result<EndNarrative> {
    if bytes.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{END_DAT_FILE}: empty narrative file"),
        ));
    }
    let mut has_renderable_text = false;
    for (offset, &byte) in bytes.iter().enumerate() {
        match byte {
            0x00 | 0x0a | 0x0d | END_PARAGRAPH_START_MARKER | END_SOFT_BREAK_MARKER => {}
            ch if (0x20..=0x7e).contains(&ch) => has_renderable_text = true,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{END_DAT_FILE}: unsupported byte 0x{byte:02x} at offset {offset}"),
                ));
            }
        }
    }
    if !has_renderable_text {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{END_DAT_FILE}: no renderable narrative text"),
        ));
    }
    Ok(EndNarrative::new(bytes.to_vec()))
}

pub fn load_end_narrative_window_ranges(
    game_dir: &Path,
) -> io::Result<Option<[Option<EndNarrativeWindowRange>; END_DAT_WINDOW_COUNT]>> {
    let path = game_dir.join(END_NARRATIVE_WINDOW_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_end_narrative_window_ranges(&text).map(Some)
}

pub fn parse_end_narrative_window_ranges(
    text: &str,
) -> io::Result<[Option<EndNarrativeWindowRange>; END_DAT_WINDOW_COUNT]> {
    let mut ranges = [None; END_DAT_WINDOW_COUNT];
    for (line_index, line) in text.lines().enumerate() {
        let line_number = line_index + 1;
        let line = line
            .split_once('#')
            .map_or(line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        let parts: Vec<_> = line
            .split(|ch: char| ch == ',' || ch == '\t' || ch.is_whitespace())
            .filter(|part| !part.is_empty())
            .collect();
        if parts.len() != 3 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} must be: WINDOW START END"
                ),
            ));
        }
        let window = parse_u8_literal(parts[0]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} has invalid window `{}`: {err}",
                    parts[0]
                ),
            )
        })?;
        if end_narrative_window(window).is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} window must be 1..={END_DAT_WINDOW_COUNT}, got {window}"
                ),
            ));
        }
        let start = parse_usize_literal(parts[1]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} has invalid start `{}`: {err}",
                    parts[1]
                ),
            )
        })?;
        let end = parse_usize_literal(parts[2]).map_err(|err| {
            io::Error::new(
                err.kind(),
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} has invalid end `{}`: {err}",
                    parts[2]
                ),
            )
        })?;
        if start >= end {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} start must be before end, got {start}..{end}"
                ),
            ));
        }
        if end > END_DAT_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} end {end} exceeds {END_DAT_FILE} length {END_DAT_LEN}"
                ),
            ));
        }
        let range = EndNarrativeWindowRange { window, start, end };
        let slot = range.index();
        if ranges[slot].is_some() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{END_NARRATIVE_WINDOW_TABLE_FILE} line {line_number} duplicates window {window}"
                ),
            ));
        }
        ranges[slot] = Some(range);
    }
    Ok(ranges)
}

fn parse_usize_literal(text: &str) -> io::Result<usize> {
    let value = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X"));
    if let Some(hex) = value {
        usize::from_str_radix(hex, 16)
    } else {
        text.parse::<usize>()
    }
    .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
}

pub fn decode_end_window(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x00 => break,
            0x0a | 0x0d => out.push('\n'),
            // formats/end-dat.md §3: page/paragraph marker and soft hyphen
            // are layout hints, not visible glyphs.
            END_PARAGRAPH_START_MARKER | END_SOFT_BREAK_MARKER => {}
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

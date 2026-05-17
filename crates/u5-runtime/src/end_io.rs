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

/// `formats/end-dat.md §2` published filename for the final-narrative
/// text file.
pub const END_DAT_FILE: &str = "END.DAT";

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndNarrative {
    pub raw: Vec<u8>,
}

impl EndNarrative {
    pub fn full_text(&self) -> String {
        decode_end_window(&self.raw)
    }

    pub fn window(&self, start: usize, end: usize) -> Option<String> {
        if end > self.raw.len() || start > end {
            return None;
        }
        Some(decode_end_window(&self.raw[start..end]))
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
    Ok(Some(EndNarrative { raw: bytes }))
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

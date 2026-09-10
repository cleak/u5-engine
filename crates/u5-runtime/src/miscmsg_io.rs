//! Parser for `MISCMSG.DAT`: forty-seven NUL-terminated records used by the
//! Blackthorn audience cluster, shrine/virtue presentation, and Codex/urn
//! prophecy pages. Spec: `formats/miscmsg-dat.md` §2-§4.
//!
//! The Codex/prophecy records use a tile-glyph convention (`@` inter-word
//! space, `[` TH digraph, `]` NG digraph, `_` ER digraph). The parser keeps
//! those bytes intact in the returned record so the caller can route them
//! through the appropriate renderer (the sign-style tile-glyph path) versus
//! the ordinary prose printer.

use std::io;
use std::path::Path;

use crate::read_optional_disk_file;

/// `formats/miscmsg-dat.md §2` published filename. The Blackthorn
/// audience overlay, shrine/virtue presentation, and Codex/urn
/// prophecy pages all read records out of this file.
pub const MISCMSG_DAT_FILE: &str = "MISCMSG.DAT";
const EXPECTED_RECORD_COUNT: usize = 47;

/// `formats/miscmsg-dat.md §2`: shipped DOS file size in bytes.
pub const MISCMSG_DAT_LEN: usize = 2_745;
/// `formats/miscmsg-dat.md §2`: NUL-terminated record count.
pub const MISCMSG_DAT_RECORDS: usize = 47;

/// `formats/miscmsg-dat.md §4` tile-glyph digraph classifier. Some
/// Codex/prophecy records embed these byte codes that the
/// sign-style tile-glyph renderer expands into multi-character
/// graphemes. Ordinary prose records do not use them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileGlyphDigraph {
    /// `@` — inter-word space in tile-glyph text.
    InterWordSpace,
    /// `[` — `TH` digraph.
    Th,
    /// `]` — `NG` digraph.
    Ng,
    /// `_` — `ST` digraph.
    ///
    /// `formats/miscmsg-dat.md` §4 (`RETRACTIONS.md` R455): "The earlier `ER`
    /// label for `_` is retracted; Codex and endgame text use the same runic
    /// `ST` glyph."
    St,
}

impl TileGlyphDigraph {
    /// `formats/miscmsg-dat.md §4`: expansion the tile-glyph
    /// renderer prints for this digraph.
    pub const fn expansion(self) -> &'static str {
        match self {
            Self::InterWordSpace => " ",
            Self::Th => "TH",
            Self::Ng => "NG",
            Self::St => "ST",
        }
    }
}

/// `formats/miscmsg-dat.md §4`: classify a record payload byte as a
/// tile-glyph digraph code. Returns `None` for ordinary text bytes
/// that the renderer prints as-is.
pub const fn tile_glyph_digraph(byte: u8) -> Option<TileGlyphDigraph> {
    Some(match byte {
        b'@' => TileGlyphDigraph::InterWordSpace,
        b'[' => TileGlyphDigraph::Th,
        b']' => TileGlyphDigraph::Ng,
        b'_' => TileGlyphDigraph::St,
        _ => return None,
    })
}

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
    /// Records 37..=44 — urn / Codex prophecy pages, including
    /// tile-glyph text.
    UrnCodexProphecy,
    /// Record 45 — the shrine's approach narration, "before the kneeling and
    /// virtue-input records" (`RETRACTIONS.md` R417).
    ShrineEntry,
    /// Record 46 — the Codex presentation's approach narration.
    CodexEntry,
}

/// `formats/miscmsg-dat.md §3`: classify a record index `0..=46` into
/// `formats/miscmsg-dat.md §3` record-family ranges. Each family is
/// addressed by hardcoded ordinal or loaded-window offset by the
/// owning system; this catalog publishes the cluster boundaries so
/// the family classifier and the per-family slice accessors share
/// one source of truth.
pub const MISCMSG_BLACKTHORN_AUDIENCE_RANGE: std::ops::RangeInclusive<usize> = 0..=11;
/// `blackthorn.md §4.1`: the four escalating audience demands are records
/// `0..=3`. "The first three records receive the virtue name and the
/// question-mark/closing-quote suffix"; the fourth "has an opening quotation
/// mark but **no closing quotation mark**, and its caller adds none."
pub const MISCMSG_BLACKTHORN_DEMAND_RANGE: std::ops::RangeInclusive<usize> = 0..=3;
/// §4.1: record `11` is "the Wait/Avatarhood speech, then acknowledgement and
/// two line feeds before the first demand". The engine took the audience's
/// opening from the first nonblank record of the whole family instead, which
/// is record `0` - the first demand's own template, printed without its
/// virtue.
pub const MISCMSG_BLACKTHORN_AUDIENCE_PREAMBLE: usize = 11;
/// §4.1's reaction records: `4` is "the unquoted pendulum narration", `5` "the
/// merciful-death speech", `9` "the truth/life reward speech" for a correct
/// answer with only one nondead member, and `10` the child/dungeon line.
pub const MISCMSG_BLACKTHORN_PENDULUM_NARRATION: usize = 4;
pub const MISCMSG_BLACKTHORN_MERCIFUL_DEATH: usize = 5;
/// §5: "After acknowledgement, record `6` supplies the quoted
/// unfairness/treachery speech, including its leading two line feeds."
pub const MISCMSG_BLACKTHORN_TREACHERY_SPEECH: usize = 6;
pub const MISCMSG_BLACKTHORN_TRUTH_REWARD: usize = 9;
pub const MISCMSG_BLACKTHORN_DUNGEON_THREAT: usize = 10;
/// §4.1, the first wrong answer with at least two nondead members: record `7`,
/// "the quoted laughing-at-me rebuke", then record `8`, "which begins with two
/// line feeds and the quoted sand/threat prefix".
pub const MISCMSG_BLACKTHORN_FIRST_WRONG_REBUKE: usize = 7;
pub const MISCMSG_BLACKTHORN_SAND_THREAT_PREFIX: usize = 8;
pub const MISCMSG_VIRTUE_FAILING_RANGE: std::ops::RangeInclusive<usize> = 12..=19;
pub const MISCMSG_VIRTUE_APHORISM_RANGE: std::ops::RangeInclusive<usize> = 20..=27;
/// `formats/miscmsg-dat.md §3`: "28-36 | Shrine meditation | Meditation
/// prompts, altar text, offering text, and ordained/quest turn-in
/// presentation". `RETRACTIONS.md` R423 moved record 36 here from the
/// Codex group - "it is the shrine's Codex-read quest turn-in response".
pub const MISCMSG_SHRINE_MEDITATION_RANGE: std::ops::RangeInclusive<usize> = 28..=36;
/// §3: "37-44 | Urn/Codex prophecy". The engine started this group at 36,
/// so every urn page was read one record early - virtue 0 returned the
/// shrine's turn-in response and the last virtue fell off the end.
pub const MISCMSG_URN_CODEX_RANGE: std::ops::RangeInclusive<usize> = 37..=44;
/// §3: "45 | Shrine entry | Approach narration before the kneeling and
/// virtue-input records" (`RETRACTIONS.md` R417).
pub const MISCMSG_SHRINE_ENTRY_NARRATION: usize = 45;
/// §3: "46 | Codex entry | Approach narration for the Codex presentation".
pub const MISCMSG_CODEX_ENTRY_NARRATION: usize = 46;

/// The Codex presentation's shared preamble, measured 2026-09-09
/// (`qa/paired/codex-enter.tsv`): record `46`, then record `37`, then record
/// `38`, then the page itself.
///
/// `formats/miscmsg-dat.md §3` calls `37-44` "Urn/Codex prophecy" and this
/// engine read it as eight per-virtue pages indexed by virtue. It is not an
/// array: `37` and `38` are this preamble, `39` is the page an unordained
/// party reads, `40` is a page-turn line, and `41`-`44` are four sequential
/// runic pages. Asked on `cleak/u5-spec#253`.
pub const MISCMSG_CODEX_PAGE_OPENED: usize = 37;
pub const MISCMSG_CODEX_READS_PREFIX: usize = 38;
/// The page an unordained party turns to. Measured: the original prints it
/// where `karma.md §8` publishes no line for the no-ordained branch.
pub const MISCMSG_CODEX_NO_QUEST_PAGE: usize = 39;

/// Keys the Codex presentation absorbs after its first record is drawn.
///
/// `karma.md` §8.2 publishes three lengths, counted from after the approach
/// record `46`: "a no-ordained visit consumes **three** keys, an ordained
/// visit with an incomplete resulting read mask consumes **four**, and an
/// ordained visit with a complete resulting read mask consumes **nine**."
///
/// Measured 2026-09-09 (`qa/paired/codex-enter.tsv`) on the no-ordained path:
/// three keys stepped the original through the rest of the presentation and
/// printed nothing of their own; the fourth and fifth printed the ordinary
/// ` Pass` echo. The engine applied that count to every path.
pub const CODEX_PRESENTATION_KEY_STEPS_NO_ORDAINED: u8 = 3;
pub const CODEX_PRESENTATION_KEY_STEPS_ORDAINED: u8 = 4;
pub const CODEX_PRESENTATION_KEY_STEPS_COMPLETION: u8 = 9;

/// `miscmsg-dat.md` §3: record `40` is the "Page-turn transition, printed once
/// before the shared completion pages", and `41`-`44` are "Four shared runic
/// pages, presented sequentially when the selected virtue leaves the read mask
/// complete".
pub const MISCMSG_CODEX_PAGE_TURN: usize = 40;
pub const MISCMSG_CODEX_COMPLETION_PAGES: std::ops::RangeInclusive<usize> = 41..=44;

/// `karma.md` §8.1: after the page-turn record and its key, the presentation
/// prints this lead-in in the ordinary font before the four runic pages.
pub const CODEX_COMPLETION_READ_LEAD_IN: &str = "Thou dost read:\n\n";

/// `karma.md §12`, the Codex-unread arm: record `31` is "the altar's quest
/// announcement", record `32` opens the quest sentence that the virtue's own
/// `12..19` record completes, and record `33` is "the instruction to return
/// after the quest".
pub const MISCMSG_SHRINE_ALTAR_ANNOUNCEMENT: usize = 31;
pub const MISCMSG_SHRINE_QUEST_SENTENCE: usize = 32;
pub const MISCMSG_SHRINE_RETURN_INSTRUCTION: usize = 33;
/// `karma.md §12` "Unfocused result": "If the virtue answer or any of the
/// three mantra answers was wrong, render record `30` after the third nonblank
/// mantra, then return without quest progress."
pub const MISCMSG_SHRINE_UNFOCUSED_RESULT: usize = 30;
/// `karma.md §12` "Completed-quest offering": "Record `34` asks for a number
/// of hundreds of gold pieces. Its authored leading blank row and trailing
/// space belong to the prompt."
pub const MISCMSG_SHRINE_OFFERING_PROMPT: usize = 34;
/// `karma.md §12`: "Insufficient gold | Print record `35`, the
/// insufficient-gold response, then repeat record `34` and the single-digit
/// chooser."
pub const MISCMSG_SHRINE_OFFERING_INSUFFICIENT_GOLD: usize = 35;

/// its consumer cluster. Returns `None` for indices outside the file.
pub const fn miscmsg_family(record_index: usize) -> Option<MiscMsgFamily> {
    Some(
        if record_index <= *MISCMSG_BLACKTHORN_AUDIENCE_RANGE.end() {
            MiscMsgFamily::BlackthornAudience
        } else if record_index <= *MISCMSG_VIRTUE_FAILING_RANGE.end() {
            MiscMsgFamily::VirtueWeaknessPhrases
        } else if record_index <= *MISCMSG_VIRTUE_APHORISM_RANGE.end() {
            MiscMsgFamily::VirtueAphorisms
        } else if record_index <= *MISCMSG_SHRINE_MEDITATION_RANGE.end() {
            MiscMsgFamily::ShrineMeditation
        } else if record_index <= *MISCMSG_URN_CODEX_RANGE.end() {
            MiscMsgFamily::UrnCodexProphecy
        } else if record_index == MISCMSG_SHRINE_ENTRY_NARRATION {
            MiscMsgFamily::ShrineEntry
        } else if record_index == MISCMSG_CODEX_ENTRY_NARRATION {
            MiscMsgFamily::CodexEntry
        } else {
            return None;
        },
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MiscMessages {
    pub records: Vec<String>,
}

impl MiscMessages {
    pub fn record(&self, index: usize) -> Option<&str> {
        self.records.get(index).map(String::as_str)
    }

    pub fn blackthorn_audience(&self) -> &[String] {
        slice_range(&self.records, &MISCMSG_BLACKTHORN_AUDIENCE_RANGE)
    }

    pub fn virtue_failing_text(&self) -> &[String] {
        slice_range(&self.records, &MISCMSG_VIRTUE_FAILING_RANGE)
    }

    pub fn virtue_aphorism(&self) -> &[String] {
        slice_range(&self.records, &MISCMSG_VIRTUE_APHORISM_RANGE)
    }

    pub fn shrine_meditation(&self) -> &[String] {
        slice_range(&self.records, &MISCMSG_SHRINE_MEDITATION_RANGE)
    }

    pub fn urn_codex(&self) -> &[String] {
        slice_range(&self.records, &MISCMSG_URN_CODEX_RANGE)
    }

    pub fn urn_codex_for_virtue_index(&self, virtue_index: usize) -> Option<&str> {
        self.urn_codex().get(virtue_index).map(String::as_str)
    }
}

fn slice_range<'a>(records: &'a [String], range: &std::ops::RangeInclusive<usize>) -> &'a [String] {
    let start = (*range.start()).min(records.len());
    let end = (*range.end() + 1).min(records.len());
    if start >= end {
        return &[];
    }
    &records[start..end]
}

pub fn load_misc_messages(game_dir: &Path) -> io::Result<Option<MiscMessages>> {
    let path = game_dir.join(MISCMSG_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    parse_misc_messages(&bytes).map(Some)
}

pub fn parse_misc_messages(bytes: &[u8]) -> io::Result<MiscMessages> {
    let mut records = Vec::with_capacity(EXPECTED_RECORD_COUNT);
    let mut start = 0;
    for record_index in 0..EXPECTED_RECORD_COUNT {
        let (record, next_start) = read_misc_record(bytes, start, record_index)?;
        records.push(record);
        start = next_start;
    }
    validate_misc_tail(bytes, start)?;
    Ok(MiscMessages { records })
}

fn read_misc_record(
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
            format!("{MISCMSG_DAT_FILE}: record {record_index} is not NUL-terminated"),
        ));
    };
    let record = decode_misc_record(record_index, &bytes[start..end])?;
    Ok((record, end + 1))
}

fn validate_misc_tail(bytes: &[u8], mut start: usize) -> io::Result<()> {
    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|&b| b == 0x00)
            .map(|offset| start + offset);
        let Some(end) = end else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{MISCMSG_DAT_FILE}: extra record starting at byte {start} is not NUL-terminated"
                ),
            ));
        };
        if end > start {
            decode_misc_record(EXPECTED_RECORD_COUNT, &bytes[start..end])?;
        }
        start = end + 1;
    }
    Ok(())
}

fn decode_misc_record(record_index: usize, bytes: &[u8]) -> io::Result<String> {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x0a | 0x0d => out.push('\n'),
            // Tile-glyph and printable ASCII bytes pass through; the caller
            // decides whether to render them through the prose printer or the
            // sign-style tile-glyph renderer per the spec.
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{MISCMSG_DAT_FILE}: record {record_index} has unsupported byte 0x{byte:02x}"
                    ),
                ));
            }
        }
    }
    Ok(out)
}

pub fn render_miscmsg_tile_glyph_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if let Some(digraph) = tile_glyph_digraph(byte) {
            out.push_str(digraph.expansion());
        } else {
            out.push(byte as char);
        }
    }
    out
}

//! Which words of the conversation window to mark (Phase B design 8.3.10;
//! build plan M6-05). Pure: the shell's `[highlight]` step feeds it the
//! window's rows and draws the spans it returns on the presented copy.
//!
//! - A word is the bytes between spaces; a run of runic glyphs is one word
//!   (`conversation.md §7.1`). Punctuation at either end of an ordinary
//!   word is left off, because the matcher's word-start rule would refuse
//!   a word that begins with a quote mark and a mark under a comma reads
//!   as noise.
//! - A word broken across a full row and the next one is joined.
//! - Each word is asked of the session's own matcher,
//!   [`tlk_keyword_lookup`], so the 15-character cap, the word-start rule
//!   and first-in-blob-order are the game's. Reserved words (the Name, Job
//!   and Bye entries), rebukes and labelled-block arms are never marked.
//! - Nothing is marked while the session waits on a scoped answer, an
//!   ASK-WHO name or the gold refusal, nor on rows from before the
//!   session (the caller says which rows are eligible).

use std::collections::HashMap;

use crate::conversation_session::{ConversationSessionPhase, TlkKeywordLookup, tlk_keyword_lookup};
use crate::enhancements::ClueHighlight;
use crate::tlk_control_codes::{
    TLK_ASK_WHO_PROMPT, TLK_KEYWORD_PROMPT, TLK_KEYWORD_PROMPT_OPEN_LINE, TLK_SCOPED_PROMPT,
};
use crate::{TlkGlyphFont, TlkRenderedGlyph, tlk_keyword_matches};

use super::{AskedState, ClueEntry, RecordAddr, TestInputs, asked_state, record_dependence};

/// Glyphs per window row (`systems/text-output.md §10.1`).
const ROW_WIDTH: usize = crate::MESSAGE_WINDOW_WIDTH;

/// How a marked word looks (8.3.10 "Three states").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Mark {
    /// Asked of this person, and the answer cannot have changed.
    Same,
    /// Asked of this person, but the answer may differ now.
    MayDiffer,
    /// `asked_elsewhere`: a topic asked of someone else.
    Elsewhere,
    /// `new` (dev): a topic this person answers that was never asked.
    New,
}

/// One placed row of the window as the highlighter reads it.
#[derive(Clone, Copy, Debug)]
pub struct HighlightRow<'a> {
    /// Absolute screen row, 11..=23.
    pub row: u8,
    /// Absolute column of the first glyph.
    pub column: u8,
    pub glyphs: &'a [TlkRenderedGlyph],
    /// Printed in this session and not an echo of the player's line: the
    /// only rows a mark may go on.
    pub eligible: bool,
}

/// The cells of one word on one row: `len` glyphs from `column`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub row: u8,
    pub column: u8,
    pub len: u8,
    pub mark: Mark,
}

/// What has been asked, as the highlighter needs it.
#[derive(Clone, Copy, Debug, Default)]
pub struct Asked<'a> {
    /// The entries of this person's dialog (the log's, or the session-only
    /// set's). Other people's entries are not read.
    pub here: &'a [ClueEntry],
    /// The keywords (bit 7 clear, upper case) of topics asked of someone
    /// else. Empty without the log.
    pub elsewhere: &'a [Vec<u8>],
}

/// The conversation the window belongs to.
#[derive(Clone, Copy, Debug)]
pub struct HighlightSession<'a> {
    pub fields: &'a [Vec<u8>],
    pub decoded: &'a [String],
    pub phase: ConversationSessionPhase,
    /// The test inputs on the current state (8.3.13 step 3).
    pub now: TestInputs,
}

/// Whether `phase` suppresses every mark (8.3.10 "Suppressed").
pub fn phase_suppresses(phase: ConversationSessionPhase) -> bool {
    matches!(
        phase,
        ConversationSessionPhase::AwaitingScopedKeyword { .. }
            | ConversationSessionPhase::AwaitingAskWho { .. }
            | ConversationSessionPhase::AwaitingGoldRefusalKeyword
    )
}

/// Whether a window row is the player's or the prompt's, never the
/// person's: an echo on the open `:` row, or a prompt's own row above it.
pub fn is_prompt_or_echo_row(text: &str) -> bool {
    let heads = [TLK_KEYWORD_PROMPT, TLK_SCOPED_PROMPT, TLK_ASK_WHO_PROMPT]
        .into_iter()
        .flat_map(|prompt| prompt.split('\n'))
        .filter(|row| !row.is_empty());
    text.starts_with(TLK_KEYWORD_PROMPT_OPEN_LINE) || heads.into_iter().any(|row| row == text)
}

/// The keyword of `record` in `fields`, as [`Asked::elsewhere`] holds it:
/// only an ordinary pair has one.
pub fn topic_keyword(fields: &[Vec<u8>], record: RecordAddr) -> Option<Vec<u8>> {
    let RecordAddr::Pair(k) = record else {
        return None;
    };
    let field = fields.get(2 * usize::from(k) + 5)?;
    let keyword: Vec<u8> = field
        .iter()
        .map(|b| (b & 0x7F).to_ascii_uppercase())
        .collect();
    let keyword = keyword.trim_ascii().to_vec();
    (!keyword.is_empty()).then_some(keyword)
}

/// One cell of the flattened eligible text.
#[derive(Clone, Copy)]
struct Cell {
    row: u8,
    column: u8,
    glyph: TlkRenderedGlyph,
}

/// The words of `rows`: the eligible rows only, wrapped words joined, a
/// runic run kept whole and ordinary words trimmed of punctuation.
fn words(rows: &[HighlightRow<'_>]) -> Vec<Vec<Cell>> {
    let mut words = Vec::new();
    let mut word: Vec<Cell> = Vec::new();
    let mut previous: Option<&HighlightRow<'_>> = None;
    let flush = |word: &mut Vec<Cell>, words: &mut Vec<Vec<Cell>>| {
        let taken = std::mem::take(word);
        if let Some(trimmed) = trim(taken) {
            words.push(trimmed);
        }
    };
    for row in rows {
        let joins = previous.is_some_and(|prev| {
            prev.eligible
                && row.eligible
                && row.row == prev.row + 1
                && prev.glyphs.len() >= ROW_WIDTH
                && prev.glyphs.last().is_some_and(|g| !is_space(*g))
                && row.glyphs.first().is_some_and(|g| !is_space(*g))
        });
        if !joins {
            flush(&mut word, &mut words);
        }
        previous = Some(row);
        if !row.eligible {
            continue;
        }
        for (offset, glyph) in row.glyphs.iter().enumerate() {
            let cell = Cell {
                row: row.row,
                column: row.column.saturating_add(offset as u8),
                glyph: *glyph,
            };
            let runic = glyph.font == TlkGlyphFont::Runic;
            let breaks_font = word
                .last()
                .is_some_and(|last| (last.glyph.font == TlkGlyphFont::Runic) != runic);
            if breaks_font {
                flush(&mut word, &mut words);
            }
            if !runic && is_space(*glyph) {
                flush(&mut word, &mut words);
            } else {
                word.push(cell);
            }
        }
    }
    flush(&mut word, &mut words);
    words
}

fn is_space(glyph: TlkRenderedGlyph) -> bool {
    glyph.byte & 0x7F == b' '
}

/// An ordinary word without leading and trailing punctuation; a runic run
/// without its outer spaces. `None` when nothing is left.
fn trim(mut word: Vec<Cell>) -> Option<Vec<Cell>> {
    let keep = |cell: &Cell| {
        if cell.glyph.font == TlkGlyphFont::Runic {
            !is_space(cell.glyph)
        } else {
            (cell.glyph.byte & 0x7F).is_ascii_alphanumeric()
        }
    };
    let end = word.iter().rposition(keep)? + 1;
    word.truncate(end);
    let start = word.iter().position(keep)?;
    word.drain(..start);
    Some(word)
}

fn text_of(word: &[Cell]) -> String {
    word.iter()
        .map(|cell| char::from(cell.glyph.byte & 0x7F))
        .collect()
}

/// The pair a lookup answered with, when it is an ordinary topic.
fn candidate(lookup: TlkKeywordLookup) -> Option<RecordAddr> {
    match lookup {
        TlkKeywordLookup::Field(n) if n >= 6 && n % 2 == 0 => {
            u8::try_from((n - 6) / 2).ok().map(RecordAddr::Pair)
        }
        _ => None,
    }
}

/// Reserved words, rebukes and the empty line are never marked, not even
/// as heard elsewhere.
fn never_marked(lookup: TlkKeywordLookup) -> bool {
    match lookup {
        TlkKeywordLookup::EmptyLine | TlkKeywordLookup::Rebuke => true,
        TlkKeywordLookup::Field(n) => n < 6 || n % 2 == 1,
        TlkKeywordLookup::NoMatch => false,
    }
}

/// 8.3.10: the spans to mark in `rows` under `mode`, for the conversation
/// `session` (`None`: no session, no marks).
pub fn highlight_spans(
    rows: &[HighlightRow<'_>],
    session: Option<&HighlightSession<'_>>,
    asked: &Asked<'_>,
    mode: ClueHighlight,
) -> Vec<Span> {
    let Some(session) = session else {
        return Vec::new();
    };
    if mode == ClueHighlight::Off || phase_suppresses(session.phase) {
        return Vec::new();
    }
    let elsewhere_on = matches!(mode, ClueHighlight::AskedElsewhere | ClueHighlight::New);
    let mut states: HashMap<RecordAddr, Option<AskedState>> = HashMap::new();
    let mut spans = Vec::new();
    for word in words(rows) {
        let text = text_of(&word);
        let lookup = tlk_keyword_lookup(Some(session.fields), session.decoded, &text);
        if never_marked(lookup) {
            continue;
        }
        let record = candidate(lookup);
        let state = record.and_then(|record| {
            *states.entry(record).or_insert_with(|| {
                let entries: Vec<ClueEntry> = asked
                    .here
                    .iter()
                    .filter(|entry| entry.record == record)
                    .copied()
                    .collect();
                (!entries.is_empty()).then(|| {
                    asked_state(
                        &entries,
                        &record_dependence(session.fields, record),
                        &session.now,
                    )
                })
            })
        });
        let mark = match state {
            Some(AskedState::Same) => Some(Mark::Same),
            Some(AskedState::MayDiffer) => Some(Mark::MayDiffer),
            None if mode == ClueHighlight::New && record.is_some() => Some(Mark::New),
            None if elsewhere_on && heard_elsewhere(asked.elsewhere, &text) => {
                Some(Mark::Elsewhere)
            }
            None => None,
        };
        if let Some(mark) = mark {
            push_spans(&mut spans, &word, mark);
        }
    }
    spans
}

/// Whether `word` would hit any keyword asked of someone else, by the
/// game's own word-start rule.
fn heard_elsewhere(elsewhere: &[Vec<u8>], word: &str) -> bool {
    let upper: Vec<u8> = word.bytes().map(|b| b.to_ascii_uppercase()).collect();
    elsewhere
        .iter()
        .any(|keyword| tlk_keyword_matches(keyword, &upper))
}

/// One span per row the word covers.
fn push_spans(spans: &mut Vec<Span>, word: &[Cell], mark: Mark) {
    let mut start = 0;
    while start < word.len() {
        let row = word[start].row;
        let len = word[start..]
            .iter()
            .take_while(|cell| cell.row == row)
            .count();
        spans.push(Span {
            row,
            column: word[start].column,
            len: len as u8,
            mark,
        });
        start += len;
    }
}

#[cfg(test)]
#[path = "highlight_tests.rs"]
mod tests;

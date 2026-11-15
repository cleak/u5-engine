//! What a clue's body shows under `clue_text` (Phase B design 8.3.6; build
//! plan M6-04).
//!
//! - **off.** No text is shown or kept.
//! - **live.** The rows the player saw this session, from RAM
//!   ([`SessionText`]), so they are always right, branches included. After
//!   a relaunch only ids remain: a record that [`record_dependence`] finds
//!   with no tests, no reply, no `0x86` and no label code is laid out
//!   statically ([`static_render`]); any other record shows
//!   [`REPLAY_LINE`]. No branch is guessed.
//! - **kept.** The rows seen, from the sidecar's TEXT section (D8.1; the
//!   resolver does not offer `kept` in this build).
//!
//! The rows are the window's own: at most 16 bytes, with a runic mask, so
//! the page paints them in the game's fixed-cell fonts as the player read
//! them. Nothing here is written to disk; [`SessionText`] lives in RAM only.

use super::dependence::record_dependence;
use super::{ClueEntry, DialogKey, NpcKey, RecordAddr, TextRow};
use crate::enhancements::journal::ClueText;
use crate::tlk_runner::{TlkGlyphFont, TlkRenderedGlyph, TlkRunInputs, run_tlk_stream};
use crate::{MessageEntry, PUBLISHED_COMMON_WORD_DICTIONARY, wrap_message_window_rows};

/// The body of a record that cannot be laid out without the conversation
/// (8.2.3). Our own words.
pub const REPLAY_LINE: &str = "Replay the conversation to read it.";

/// The most rows kept for one answer: a long reply, held pages and all.
pub const MAX_TEXT_ROWS: usize = 64;
/// The most answers [`SessionText`] keeps; past it the oldest go.
pub const MAX_SESSION_TEXTS: usize = 1024;

/// The cells of a row.
pub const ROW_CELLS: usize = 16;

/// One transcript entry as the window draws it: wrapped at the window's 16
/// cells, each row with a bit per runic cell. An empty entry is one empty
/// row.
pub fn text_rows_of(entry: &MessageEntry) -> Vec<TextRow> {
    let glyphs: Vec<TlkRenderedGlyph> = if entry.glyphs.len() == entry.text.len() {
        entry.glyphs.clone()
    } else {
        entry.text.bytes().map(TlkRenderedGlyph::ordinary).collect()
    };
    let rows: Vec<TextRow> = wrap_message_window_rows(&glyphs)
        .iter()
        .map(|glyphs| {
            let bytes: Vec<u8> = glyphs.iter().map(|glyph| glyph.byte).collect();
            row_of_glyphs(glyphs, &bytes)
        })
        .collect();
    if rows.is_empty() {
        vec![TextRow {
            bytes: Vec::new(),
            runic: 0,
        }]
    } else {
        rows
    }
}

/// A row from glyphs (preferred) or, when the producer kept none, its text
/// in the ordinary font.
fn row_of_glyphs(glyphs: &[TlkRenderedGlyph], text: &[u8]) -> TextRow {
    let mut row = TextRow {
        bytes: Vec::new(),
        runic: 0,
    };
    if glyphs.len() == text.len() && !glyphs.is_empty() {
        for (index, glyph) in glyphs.iter().take(ROW_CELLS).enumerate() {
            row.bytes.push(glyph.byte);
            if glyph.font == TlkGlyphFont::Runic {
                row.runic |= 1 << index;
            }
        }
    } else {
        row.bytes = text.iter().copied().take(ROW_CELLS).collect();
    }
    row
}

/// Which answer some rows belong to: who, which dialog and record, and the
/// variant (8.3.3). The test bits are left out: the rows are what was read.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TextKey {
    pub npc: NpcKey,
    pub dialog: DialogKey,
    pub record: RecordAddr,
    pub variant: u16,
}

impl TextKey {
    pub fn of(entry: &ClueEntry) -> Self {
        Self {
            npc: entry.npc,
            dialog: entry.dialog,
            record: entry.record,
            variant: entry.variant,
        }
    }
}

/// The rows seen this session (`clue_text = live`), in RAM only and never
/// saved. A relaunch, or turning the log off, drops it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SessionText {
    texts: Vec<(TextKey, Vec<TextRow>)>,
}

impl SessionText {
    pub fn new() -> Self {
        Self::default()
    }

    /// Keeps the rows of one answer (the newest hearing wins), cut to
    /// [`MAX_TEXT_ROWS`]. An answer with no rows is not kept.
    pub fn remember(&mut self, entry: &ClueEntry, rows: &[TextRow]) {
        if rows.is_empty() {
            return;
        }
        let key = TextKey::of(entry);
        self.texts.retain(|(old, _)| *old != key);
        if self.texts.len() >= MAX_SESSION_TEXTS {
            self.texts.remove(0);
        }
        self.texts
            .push((key, rows.iter().take(MAX_TEXT_ROWS).cloned().collect()));
    }

    pub fn rows(&self, entry: &ClueEntry) -> Option<&[TextRow]> {
        let key = TextKey::of(entry);
        self.texts
            .iter()
            .find(|(old, _)| *old == key)
            .map(|(_, rows)| rows.as_slice())
    }

    pub fn len(&self) -> usize {
        self.texts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.texts.is_empty()
    }
}

/// The static render of `record` after a relaunch (8.3.6), or `None` when
/// the record has a branch, a test, a reply or an action (then the body is
/// [`REPLAY_LINE`]). The field is run through the byte runner on its own,
/// which applies the `0xA2` quote collapse, the dictionary's spacing and
/// the runic spans (`formats/tlk.md §9.2, §10.1`); the rows are wrapped at
/// the window's 16 columns. `avatar` fills `0x81`.
pub fn static_render(fields: &[Vec<u8>], record: RecordAddr, avatar: &str) -> Option<Vec<TextRow>> {
    let field = fields.get(record.field()?)?;
    if !record_dependence(fields, record).renders_statically() {
        return None;
    }
    // An alias (`0x87`) runs another record, ASK-WHO (`0x88`) reads the
    // player, and `0x84`/`0x85` depend on the party: none of these is laid
    // out without the conversation.
    if field
        .iter()
        .any(|byte| matches!(byte, 0x84 | 0x85 | 0x87 | 0x88))
    {
        return None;
    }
    let dictionary = PUBLISHED_COMMON_WORD_DICTIONARY;
    let inputs = TlkRunInputs {
        avatar_name: avatar,
        branch_flags: 0,
        moral_standing: 0,
        dictionary: Some(&dictionary),
        gold_available: None,
        npc_slot: None,
        ask_who_response: 0,
        yield_on_pause: false,
        mark_holds: false,
        yield_on_ask: false,
    };
    let output = run_tlk_stream(field, &inputs);
    // `conversation.md §6`'s framing: the response in double quotes, the
    // closing one hugging the text, and a quote never after another
    // (`formats/tlk.md §9.2`).
    let mut glyphs = output.rendered_glyphs;
    let body_len = glyphs
        .iter()
        .rposition(|glyph| glyph.byte != b'\n')
        .map_or(0, |index| index + 1);
    glyphs.truncate(body_len);
    let opens_quoted = glyphs
        .iter()
        .find(|glyph| !glyph.byte.is_ascii_whitespace())
        .is_some_and(|glyph| glyph.byte == b'"');
    if !opens_quoted {
        glyphs.insert(0, TlkRenderedGlyph::ordinary(b'"'));
        if glyphs.last().is_none_or(|glyph| glyph.byte != b'"') {
            glyphs.push(TlkRenderedGlyph::ordinary(b'"'));
        }
    }
    let rows: Vec<TextRow> = wrap_message_window_rows(&glyphs)
        .iter()
        .take(MAX_TEXT_ROWS)
        .map(|glyphs| {
            let bytes: Vec<u8> = glyphs.iter().map(|glyph| glyph.byte).collect();
            row_of_glyphs(glyphs, &bytes)
        })
        .collect();
    let first = rows.iter().position(|row| !row.bytes.is_empty())?;
    let last = rows.iter().rposition(|row| !row.bytes.is_empty())?;
    Some(rows[first..=last].to_vec())
}

/// What a clue's body shows (8.2.3).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TextBody {
    /// `clue_text = off`: the one-line summary only.
    Off,
    /// The rows, as the window drew them.
    Rows(Vec<TextRow>),
    /// [`REPLAY_LINE`].
    Replay,
}

/// The body of `entry` under `mode`: the rows seen this session, then (for
/// `kept`) the sidecar's rows, then the static render, else the replay
/// line. `fields` is the answering dialog's blob, read live.
pub fn text_body(
    mode: ClueText,
    entry: &ClueEntry,
    session: Option<&[TextRow]>,
    kept: Option<&[TextRow]>,
    fields: Option<&[Vec<u8>]>,
    avatar: &str,
) -> TextBody {
    if mode == ClueText::Off {
        return TextBody::Off;
    }
    if let Some(rows) = session.filter(|rows| !rows.is_empty()) {
        return TextBody::Rows(rows.to_vec());
    }
    if mode == ClueText::Kept
        && let Some(rows) = kept.filter(|rows| !rows.is_empty())
    {
        return TextBody::Rows(rows.to_vec());
    }
    match fields.and_then(|fields| static_render(fields, entry.record, avatar)) {
        Some(rows) => TextBody::Rows(rows),
        None => TextBody::Replay,
    }
}

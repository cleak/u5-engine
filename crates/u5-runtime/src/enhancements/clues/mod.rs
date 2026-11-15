//! Chapter 8's clue log: the in-memory model (Phase B design 8.3.2, 8.3.3;
//! build plan M6-01).
//!
//! The log holds ids, never text: who answered (`NpcKey`, `DialogKey`),
//! which record answered (`RecordAddr`), a hash of the rows seen, when,
//! how often, the player's own marks, and the pass/fail bits of the tests
//! the answer met (8.3.13). Names, keywords and replies are read at
//! runtime from the player's own `.TLK` files and are never stored here.
//!
//! - [`attribute`]: which record answered each typed line (8.3.4).
//! - [`codec`]: the `QJRN` payload, version 1 (8.3.7).
//! - [`reconcile`]: what a sidecar read gives the log, by freshness.
//! - [`dependence`]: which state an answer can depend on, and the
//!   asked-state rule (8.3.13).
//! - [`highlight`]: which words of the conversation window to mark
//!   (8.3.10).
//!
//! `ClueLog` is a shell resource in `u5-bevy`, never a `PlayState` field.
//! Everything here is pure: no RNG, no file, no session method.

pub mod attribute;
pub mod codec;
pub mod dependence;
pub mod highlight;
pub mod reconcile;
pub mod scrollback;
pub mod text;

pub use attribute::{
    ConversationSnapshot, Turn, TurnAttributor, TurnContext, attribute_turns, game_minute,
    inline_turn_record, test_inputs,
};
pub use codec::{
    CLUE_ENTRY_LEN, CodecError, EncodeOptions, JOURNAL_PAYLOAD_VERSION, MAX_PAYLOAD_LEN,
    decode_payload, encode_payload,
};
pub use dependence::{
    AskedState, Dependence, MAX_TESTS, Test, TestInputs, asked_state, block_count,
    declaration_index, evaluate, record_dependence,
};
pub use highlight::{
    Asked, HighlightRow, HighlightSession, Mark, Span, highlight_spans, is_prompt_or_echo_row,
    phase_suppresses, topic_keyword,
};
pub use reconcile::{ReconcileNote, Reconciled, reconcile};
pub use scrollback::{MAX_SCROLLBACK_ENTRIES, Scrollback};
pub use text::{
    REPLAY_LINE, SessionText, TextBody, TextKey, static_render, text_body, text_rows_of,
};

use crate::conversation_session::TlkKeywordLookup;

/// The most entries a log holds (8.3.7). At the cap the oldest entry that
/// is not starred, pinned or noted is evicted (8.2.9).
pub const MAX_ENTRIES: usize = 4096;

/// A person, by place (8.3.2): the scene byte and the roster slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct NpcKey {
    pub scene: u8,
    pub slot: u8,
}

/// A dialog, by source (8.3.2): the `.TLK` family and the dialog id. One
/// blob shared by several slots counts once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DialogKey {
    pub family: u8,
    pub dialog_id: u8,
}

/// Which record answered (8.3.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RecordAddr {
    /// Leading record 0..=4 (`formats/tlk.md §7`). Only 0 (Name, kept as
    /// the person's "name heard" mark, never a topic) and 3 (Job) are
    /// logged.
    Leading(u8),
    /// Ordinary pair `k`: keyword record `2k+5`, response `2k+6`.
    Pair(u8),
    /// A scoped answer inside a labelled block: `decl` is the nth label
    /// declaration in blob order; `arm` is [`RecordAddr::ARM_ANSWERED`]
    /// until spec request S8.1 gives arms stable addresses.
    Scoped { decl: u8, arm: u8 },
    /// No match (only with `clue_misses`).
    Miss,
}

impl RecordAddr {
    /// "Prompt answered": the scoped arm is not known (S8.1).
    pub const ARM_ANSWERED: u8 = 0xFF;

    /// The response record's field index in the blob, for a top-level
    /// record.
    pub fn field(self) -> Option<usize> {
        match self {
            RecordAddr::Leading(n) => Some(usize::from(n)),
            RecordAddr::Pair(k) => Some(2 * usize::from(k) + 6),
            RecordAddr::Scoped { .. } | RecordAddr::Miss => None,
        }
    }

    /// Whether this is a topic of the log: not the name-heard mark and not
    /// a miss.
    pub fn is_topic(self) -> bool {
        !matches!(self, RecordAddr::Leading(0) | RecordAddr::Miss)
    }
}

/// 8.3.4 step 3: the record a top-level lookup answered with, or `None`
/// when the turn is not logged. `Field(0)` is the name-heard mark,
/// `Field(3)` the Job, `Field(n)` for even `n >= 6` the pair `(n - 6) / 2`.
/// Bye, rebukes, the empty line and any other field are dropped; a miss is
/// kept only with `clue_misses` on.
pub fn record_of(lookup: TlkKeywordLookup, misses: bool) -> Option<RecordAddr> {
    match lookup {
        TlkKeywordLookup::Field(0) => Some(RecordAddr::Leading(0)),
        TlkKeywordLookup::Field(3) => Some(RecordAddr::Leading(3)),
        TlkKeywordLookup::Field(n) if n >= 6 && n % 2 == 0 => {
            u8::try_from((n - 6) / 2).ok().map(RecordAddr::Pair)
        }
        TlkKeywordLookup::NoMatch if misses => Some(RecordAddr::Miss),
        _ => None,
    }
}

/// Minutes since year 0 on the game's calendar of 13 months of 28 days
/// (8.3.3; `formats/saved-gam.md §5`). Monotone in the date.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GameMinute(pub u32);

impl GameMinute {
    /// `((year * 13 + month - 1) * 28 + day - 1) * 1440 + hour * 60 +
    /// minute`, saturating. Month and day count from 1.
    pub fn from_calendar(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> Self {
        let days = (u64::from(year) * 13 + u64::from(month.saturating_sub(1))) * 28
            + u64::from(day.saturating_sub(1));
        let minutes = days * 1440 + u64::from(hour) * 60 + u64::from(minute);
        Self(u32::try_from(minutes).unwrap_or(u32::MAX))
    }
}

/// An entry's flag byte (8.3.7): read, star, hidden, panel, runic, stale,
/// has_text, reply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct EntryFlags(pub u8);

impl EntryFlags {
    pub const READ: u8 = 1 << 0;
    pub const STAR: u8 = 1 << 1;
    pub const HIDDEN: u8 = 1 << 2;
    /// Pinned to the side panel.
    pub const PANEL: u8 = 1 << 3;
    pub const RUNIC: u8 = 1 << 4;
    /// The slot's dialog id no longer matches (8.2.9).
    pub const STALE: u8 = 1 << 5;
    pub const HAS_TEXT: u8 = 1 << 6;
    /// The answer asks the player something back (8.3.13).
    pub const REPLY: u8 = 1 << 7;

    pub fn has(self, flag: u8) -> bool {
        self.0 & flag == flag
    }

    pub fn set(&mut self, flag: u8, on: bool) {
        if on {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }
}

/// The tests an answer met and how they came out (8.3.13 step 2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TestBits {
    /// Tests in reach; [`TestBits::UNKNOWN`] when the scan could not tell.
    pub n: u8,
    /// Outcome bits on the state before the turn.
    pub pre: u8,
    /// Outcome bits on the state after the turn.
    pub post: u8,
}

impl TestBits {
    pub const UNKNOWN: u8 = 0xFF;

    /// The bits of an answer whose reach is `dep`, heard with `pre` before
    /// the turn and `post` after it.
    pub fn capture(dep: &Dependence, pre: &TestInputs, post: &TestInputs) -> Self {
        match dep {
            Dependence::Unknown => Self {
                n: Self::UNKNOWN,
                pre: 0,
                post: 0,
            },
            Dependence::Known { tests, .. } => Self {
                n: tests.len() as u8,
                pre: evaluate(tests, pre),
                post: evaluate(tests, post),
            },
        }
    }

    pub fn is_unknown(self) -> bool {
        self.n == Self::UNKNOWN
    }
}

/// One logged answer (8.3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClueEntry {
    pub npc: NpcKey,
    pub dialog: DialogKey,
    pub record: RecordAddr,
    /// A 16-bit FNV fold of the rows seen ([`variant_hash`]); never shown.
    pub variant: u16,
    pub first: GameMinute,
    pub last: GameMinute,
    /// How often it was heard, saturating.
    pub count: u16,
    pub flags: EntryFlags,
    pub tests: TestBits,
}

impl ClueEntry {
    /// A first hearing at `at`.
    pub fn heard(
        npc: NpcKey,
        dialog: DialogKey,
        record: RecordAddr,
        variant: u16,
        tests: TestBits,
        at: GameMinute,
    ) -> Self {
        Self {
            npc,
            dialog,
            record,
            variant,
            first: at,
            last: at,
            count: 1,
            flags: EntryFlags::default(),
            tests,
        }
    }

    /// Whether `other` is the same hearing (8.3.3 folding): person,
    /// dialog, record, variant and test bits equal.
    pub fn folds_with(&self, other: &ClueEntry) -> bool {
        self.npc == other.npc
            && self.dialog == other.dialog
            && self.record == other.record
            && self.variant == other.variant
            && self.tests == other.tests
    }
}

/// The 16-bit variant hash of the rows seen (8.3.3): FNV-1a 32 folded to
/// 16 bits. It tells variants apart without storing text.
pub fn variant_hash<'a>(rows: impl IntoIterator<Item = &'a [u8]>) -> u16 {
    let mut hash = 0x811c_9dc5_u32;
    for row in rows {
        for byte in row.iter().chain(std::iter::once(&0x0A)) {
            hash ^= u32::from(*byte);
            hash = hash.wrapping_mul(0x0100_0193);
        }
    }
    ((hash >> 16) ^ (hash & 0xFFFF)) as u16
}

/// A pin of an entry to a quest card, a step or (L1) a map pin (LINKS).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Link {
    pub entry: u16,
    /// 0 quest card, 1 step, 2 map pin.
    pub kind: u8,
    pub target: u16,
}

/// What a player note is attached to (NOTES `owner_kind`).
pub mod note_owner {
    use super::NpcKey;

    pub const CLUE: u8 = 0;
    pub const PERSON: u8 = 1;
    pub const SAVE: u8 = 2;

    /// A person's `owner`: the scene byte, then the slot.
    pub fn person(npc: NpcKey) -> u16 {
        u16::from(npc.scene) << 8 | u16::from(npc.slot)
    }

    /// The person a `PERSON` owner names.
    pub fn person_of(owner: u16) -> NpcKey {
        NpcKey {
            scene: (owner >> 8) as u8,
            slot: owner as u8,
        }
    }
}

/// The longest note the overlay field takes, in bytes (ASCII only).
pub const NOTE_MAX_LEN: usize = 60;

/// A player-authored note (NOTES).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Note {
    pub owner_kind: u8,
    pub owner: u16,
    pub text: String,
}

/// A line nobody answered (MISSES; only with `clue_misses`).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Miss {
    pub npc: NpcKey,
    pub at: GameMinute,
    /// What the player typed, at most 15 bytes.
    pub typed: Vec<u8>,
}

/// The most bytes a miss keeps: the input cap of `conversation.md §6`.
pub const MISS_MAX_LEN: usize = 15;

/// One row of kept text (TEXT; `clue_text = kept` only).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TextRow {
    /// At most 16 bytes.
    pub bytes: Vec<u8>,
    /// Bit `i` set when byte `i` is in the runic font.
    pub runic: u16,
}

/// The rows seen for one entry (TEXT).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SeenRows {
    pub entry: u16,
    pub rows: Vec<TextRow>,
}

/// What [`ClueLog::fold`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoldOutcome {
    /// An equal entry was heard again: its count rose.
    Folded(usize),
    /// A new entry.
    Added(usize),
    /// A new entry, after the oldest evictable entry was dropped.
    Evicted(usize),
    /// The log is full of starred, pinned or noted entries; nothing kept.
    Full,
}

/// The clue log (8.3.3). Section 4 (PINS) and unknown sections are kept
/// as bytes and written back unchanged.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ClueLog {
    pub(crate) entries: Vec<ClueEntry>,
    pub(crate) links: Vec<Link>,
    pub(crate) notes: Vec<Note>,
    pub(crate) misses: Vec<Miss>,
    pub(crate) known: Vec<u16>,
    pub(crate) text: Vec<SeenRows>,
    /// `(section id, body)` for PINS and every id this build does not know,
    /// in file order.
    pub(crate) passthrough: Vec<(u8, Vec<u8>)>,
    dirty: bool,
    unsaved: u32,
    eviction_toast: EvictionToast,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum EvictionToast {
    #[default]
    None,
    Pending,
    Shown,
}

impl ClueLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[ClueEntry] {
        &self.entries
    }

    pub fn links(&self) -> &[Link] {
        &self.links
    }

    pub fn notes(&self) -> &[Note] {
        &self.notes
    }

    pub fn misses(&self) -> &[Miss] {
        &self.misses
    }

    pub fn known(&self) -> &[u16] {
        &self.known
    }

    pub fn text(&self) -> &[SeenRows] {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
            && self.links.is_empty()
            && self.notes.is_empty()
            && self.misses.is_empty()
            && self.known.is_empty()
            && self.text.is_empty()
    }

    /// Whether anything changed since the last save.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Captures since the last save: "N clues since your last save"
    /// (8.2.9).
    pub fn unsaved(&self) -> u32 {
        self.unsaved
    }

    /// The log was written with a save.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
        self.unsaved = 0;
    }

    /// Entries heard from one dialog, any person.
    pub fn entries_for(&self, dialog: DialogKey) -> impl Iterator<Item = &ClueEntry> {
        self.entries.iter().filter(move |e| e.dialog == dialog)
    }

    /// Folds one capture into the log (8.3.3). An equal entry gets its
    /// count raised (saturating) and its `last` moved; otherwise the entry
    /// is added, evicting at the cap.
    pub fn fold(&mut self, capture: ClueEntry) -> FoldOutcome {
        self.dirty = true;
        self.unsaved = self.unsaved.saturating_add(1);
        if let Some(index) = self.entries.iter().position(|e| e.folds_with(&capture)) {
            let entry = &mut self.entries[index];
            entry.count = entry.count.saturating_add(capture.count.max(1));
            entry.last = entry.last.max(capture.last);
            entry.first = entry.first.min(capture.first);
            if capture.flags.has(EntryFlags::RUNIC) {
                entry.flags.set(EntryFlags::RUNIC, true);
            }
            return FoldOutcome::Folded(index);
        }
        let mut evicted = false;
        if self.entries.len() >= MAX_ENTRIES {
            let Some(victim) = self.eviction_victim() else {
                self.unsaved -= 1;
                return FoldOutcome::Full;
            };
            self.evict(victim);
            evicted = true;
            if self.eviction_toast == EvictionToast::None {
                self.eviction_toast = EvictionToast::Pending;
            }
        }
        self.entries.push(capture);
        let index = self.entries.len() - 1;
        if evicted {
            FoldOutcome::Evicted(index)
        } else {
            FoldOutcome::Added(index)
        }
    }

    /// Whether the one-time "log full" toast is due; true once per log.
    pub fn take_eviction_toast(&mut self) -> bool {
        if self.eviction_toast == EvictionToast::Pending {
            self.eviction_toast = EvictionToast::Shown;
            true
        } else {
            false
        }
    }

    /// Whether entry `index` is protected from eviction: starred, pinned
    /// (to the panel or by a link) or noted (8.2.9).
    pub fn is_protected(&self, index: usize) -> bool {
        let Some(entry) = self.entries.get(index) else {
            return false;
        };
        let Ok(id) = u16::try_from(index) else {
            return false;
        };
        entry.flags.has(EntryFlags::STAR)
            || entry.flags.has(EntryFlags::PANEL)
            || self.links.iter().any(|link| link.entry == id)
            || self
                .notes
                .iter()
                .any(|note| note.owner_kind == note_owner::CLUE && note.owner == id)
    }

    /// The oldest entry (smallest `last`, then lowest index) that is not
    /// protected.
    fn eviction_victim(&self) -> Option<usize> {
        (0..self.entries.len())
            .filter(|index| !self.is_protected(*index))
            .min_by_key(|index| (self.entries[*index].last, *index))
    }

    /// Drops entry `victim` and renumbers what points past it. A
    /// protected entry is never evicted, so no link or clue note points at
    /// it; its kept text goes with it.
    fn evict(&mut self, victim: usize) {
        self.entries.remove(victim);
        let victim = victim as u16;
        let shift = |id: &mut u16| {
            if *id > victim {
                *id -= 1;
            }
        };
        self.text.retain(|seen| seen.entry != victim);
        self.text.iter_mut().for_each(|seen| shift(&mut seen.entry));
        self.links
            .iter_mut()
            .for_each(|link| shift(&mut link.entry));
        self.notes
            .iter_mut()
            .filter(|note| note.owner_kind == note_owner::CLUE)
            .for_each(|note| shift(&mut note.owner));
    }

    /// Sets or clears a flag on entry `index` (read, star, hide, pin).
    pub fn set_flag(&mut self, index: usize, flag: u8, on: bool) {
        if let Some(entry) = self.entries.get_mut(index)
            && entry.flags.has(flag) != on
        {
            entry.flags.set(flag, on);
            self.dirty = true;
        }
    }

    /// Adds a player note.
    pub fn add_note(&mut self, note: Note) {
        self.notes.push(note);
        self.dirty = true;
    }

    /// The player's note on `(owner_kind, owner)`, if any.
    pub fn note_for(&self, owner_kind: u8, owner: u16) -> Option<&Note> {
        self.notes
            .iter()
            .find(|note| note.owner_kind == owner_kind && note.owner == owner)
    }

    /// Sets the note on `(owner_kind, owner)`, replacing the one there;
    /// blank text removes it. The text is cut to [`NOTE_MAX_LEN`] bytes.
    /// A clue note on an entry that does not exist is ignored.
    pub fn set_note(&mut self, owner_kind: u8, owner: u16, text: &str) {
        if owner_kind == note_owner::CLUE && usize::from(owner) >= self.entries.len() {
            return;
        }
        let mut text = text.trim().to_string();
        while text.len() > NOTE_MAX_LEN {
            text.pop();
        }
        let at = self
            .notes
            .iter()
            .position(|note| note.owner_kind == owner_kind && note.owner == owner);
        match (at, text.is_empty()) {
            (Some(at), true) => {
                self.notes.remove(at);
            }
            (Some(at), false) if self.notes[at].text != text => self.notes[at].text = text,
            (Some(_), false) | (None, true) => return,
            (None, false) => self.notes.push(Note {
                owner_kind,
                owner,
                text,
            }),
        }
        self.dirty = true;
    }

    /// Adds a link from an entry to a quest card or step.
    pub fn add_link(&mut self, link: Link) {
        if !self.links.contains(&link) {
            self.links.push(link);
            self.dirty = true;
        }
    }

    /// Keeps a miss (only with `clue_misses`), cut to 15 bytes.
    pub fn add_miss(&mut self, mut miss: Miss) {
        miss.typed.truncate(MISS_MAX_LEN);
        self.misses.push(miss);
        self.dirty = true;
    }

    /// Marks a step "I know this".
    pub fn mark_known(&mut self, step: u16) {
        if !self.known.contains(&step) {
            self.known.push(step);
            self.dirty = true;
        }
    }

    /// Keeps the rows seen for an entry (`clue_text = kept` only).
    pub fn keep_text(&mut self, seen: SeenRows) {
        if let Some(entry) = self.entries.get_mut(usize::from(seen.entry)) {
            entry.flags.set(EntryFlags::HAS_TEXT, true);
            self.text.retain(|old| old.entry != seen.entry);
            self.text.push(seen);
            self.dirty = true;
        }
    }

    /// Drops every kept row (leaving `kept`, 8.3.6).
    pub fn drop_text(&mut self) {
        if !self.text.is_empty() {
            self.text.clear();
            for entry in &mut self.entries {
                entry.flags.set(EntryFlags::HAS_TEXT, false);
            }
            self.dirty = true;
        }
    }
}

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;

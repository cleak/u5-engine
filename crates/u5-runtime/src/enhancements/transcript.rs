//! The transcript cursor shared by the observers (Phase B design 10.3.3,
//! 8.3.4 item 10; build plan M3-05).
//!
//! An observer that reads the message window needs to know which entries
//! of [`PlayState::message_entries`] are new since it last looked. The
//! transcript is not an append-only log:
//!
//! - it holds at most [`MESSAGE_TRANSCRIPT_CAPACITY`] entries and drops the
//!   oldest from the front when a push goes past that;
//! - `clear_message_window` empties it;
//! - the runtime takes back its newest echo or a bare marker row, and edits
//!   an open row in place;
//! - a scene rebuild or a load replaces it, and
//!   [`PlayState::message_transcript_revision`] can go down or be copied
//!   across states, so the revision alone cannot order two transcripts.
//!
//! A [`TranscriptCursor`] is therefore the entry count, the revision and a
//! fingerprint of the last [`CURSOR_TAIL`] entries. [`TranscriptCursor::diff`]
//! finds that fingerprint in the new transcript, the same slice-overlap rule
//! as chapter 2's `MessageHistory::sync`: the old tail must still be there,
//! at its old place or (only when the transcript is at its cap) shifted
//! towards the front. The diff then names the first new entry. When the
//! tail cannot be found exactly once, the diff is
//! [`TranscriptDiff::Unanchored`] and the caller drops whatever it would
//! have read from the rows: a missing entry is preferred to a wrong one.
//!
//! The cursor is `Copy`, holds no text, and reads `&PlayState` only. It is
//! used by the gem map's probe (chapter 10) and by the clue log's observer
//! (chapter 8).

use std::hash::{Hash, Hasher};

use crate::{MESSAGE_TRANSCRIPT_CAPACITY, MessageEntry, PlayState};

/// How many trailing entries the cursor fingerprints (8.3.4 item 10).
pub const CURSOR_TAIL: usize = 3;

/// Where an observer last read the message transcript.
///
/// `Default` is the cursor of an empty transcript.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct TranscriptCursor {
    /// Entry count when the cursor was taken.
    len: u16,
    /// `message_transcript_revision` when the cursor was taken.
    revision: u64,
    /// Fingerprints of the last `tail_len` entries, oldest first.
    tail: [u64; CURSOR_TAIL],
    tail_len: u8,
}

/// Why a diff could not anchor. Kept for the observers' debug counters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Unanchored {
    /// The transcript is now empty, or shorter and holds none of the tail:
    /// the window was cleared.
    Cleared,
    /// The transcript is at its cap and the tail scrolled out of it.
    Capped,
    /// The tail is in the transcript more than once, so the new entries
    /// cannot be told from old ones.
    Ambiguous,
    /// Anything else: a rebuild or a load replaced the transcript.
    Rebuilt,
}

/// What changed in the transcript since a cursor was taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TranscriptDiff {
    /// The same entries as before.
    Unchanged,
    /// The old entries are still there; `now[first_new..]` are new.
    Anchored {
        /// Index into the new transcript of the first new entry. It may be
        /// `now.len()` when the only change was a withdrawal.
        first_new: usize,
        /// The cursor's last entry was edited in place or taken back, so
        /// `now[first_new]` (if any) replaces it rather than following it.
        replaced_last: bool,
    },
    /// The new entries cannot be told apart from old ones.
    Unanchored(Unanchored),
}

impl TranscriptDiff {
    /// The new entries, or `None` when the diff did not anchor.
    pub fn new_entries<'a>(&self, now: &'a [MessageEntry]) -> Option<&'a [MessageEntry]> {
        match *self {
            Self::Unchanged => Some(&[]),
            Self::Anchored { first_new, .. } => now.get(first_new..),
            Self::Unanchored(_) => None,
        }
    }

    /// Whether the diff anchored (including no change).
    pub fn is_anchored(&self) -> bool {
        !matches!(self, Self::Unanchored(_))
    }
}

impl TranscriptCursor {
    /// The cursor of `state`'s transcript as it stands.
    pub fn of(state: &PlayState) -> Self {
        Self::at(state.message_entries(), state.message_transcript_revision())
    }

    /// The cursor of the transcript `entries` at `revision`.
    pub fn at(entries: &[MessageEntry], revision: u64) -> Self {
        let tail_len = entries.len().min(CURSOR_TAIL);
        let mut tail = [0u64; CURSOR_TAIL];
        for (slot, entry) in tail.iter_mut().zip(&entries[entries.len() - tail_len..]) {
            *slot = fingerprint(entry);
        }
        Self {
            len: entries.len().min(u16::MAX as usize) as u16,
            revision,
            tail,
            tail_len: tail_len as u8,
        }
    }

    /// The entry count when the cursor was taken.
    pub fn len(&self) -> usize {
        self.len as usize
    }

    /// Whether the cursor was taken on an empty transcript.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The diff of `state`'s transcript against this cursor.
    pub fn diff_state(&self, state: &PlayState) -> TranscriptDiff {
        self.diff(state.message_entries(), state.message_transcript_revision())
    }

    /// The diff of the transcript `now` (at `revision`) against this cursor.
    /// Pure: it reads the slice and nothing else.
    pub fn diff(&self, now: &[MessageEntry], revision: u64) -> TranscriptDiff {
        let k = self.tail_len as usize;
        if k == 0 {
            // Nothing was there, so everything there now is new.
            return if now.is_empty() {
                TranscriptDiff::Unchanged
            } else {
                TranscriptDiff::Anchored {
                    first_new: 0,
                    replaced_last: false,
                }
            };
        }
        if now.is_empty() {
            return TranscriptDiff::Unanchored(Unanchored::Cleared);
        }
        // The common frame: nothing pushed and nothing edited.
        if revision == self.revision
            && now.len() == self.len()
            && now[now.len() - k..]
                .iter()
                .map(fingerprint)
                .eq(self.tail[..k].iter().copied())
        {
            return TranscriptDiff::Unchanged;
        }
        let prints: Vec<u64> = now.iter().map(fingerprint).collect();
        let tail = &self.tail[..k];
        let start = self.len() - k;

        // 1. The whole tail, at its place or shifted forward at the cap.
        match self.find(&prints, tail, start) {
            Found::One(q) => {
                let first_new = q + k;
                return if q == start && first_new == now.len() {
                    TranscriptDiff::Unchanged
                } else {
                    TranscriptDiff::Anchored {
                        first_new,
                        replaced_last: false,
                    }
                };
            }
            Found::Many => return TranscriptDiff::Unanchored(Unanchored::Ambiguous),
            Found::None => {}
        }
        // 2. The tail less its last entry: that entry was edited in place or
        //    withdrawn. One remembered entry alone cannot tell an edit from a
        //    replacement, so this needs two.
        if k >= 2 {
            match self.find(&prints, &tail[..k - 1], start) {
                Found::One(q) => {
                    return TranscriptDiff::Anchored {
                        first_new: q + k - 1,
                        replaced_last: true,
                    };
                }
                Found::Many => return TranscriptDiff::Unanchored(Unanchored::Ambiguous),
                Found::None => {}
            }
        }
        let cause = if now.len() >= MESSAGE_TRANSCRIPT_CAPACITY {
            Unanchored::Capped
        } else if now.len() < self.len() {
            Unanchored::Cleared
        } else {
            Unanchored::Rebuilt
        };
        TranscriptDiff::Unanchored(cause)
    }

    /// Where `needle` (a prefix of the tail, which started at `start`) is in
    /// `prints`. Allowed places: `start` itself, or an earlier place when the
    /// transcript is at its cap and has dropped entries from its front.
    fn find(&self, prints: &[u64], needle: &[u64], start: usize) -> Found {
        if needle.is_empty() || prints.len() < needle.len() {
            return Found::None;
        }
        let at_cap = prints.len() >= MESSAGE_TRANSCRIPT_CAPACITY;
        let last = (prints.len() - needle.len()).min(start);
        let mut found = Found::None;
        for q in 0..=last {
            if q != start && !at_cap {
                continue;
            }
            if &prints[q..q + needle.len()] == needle {
                found = match found {
                    Found::None => Found::One(q),
                    _ => Found::Many,
                };
            }
        }
        found
    }
}

impl std::fmt::Debug for TranscriptCursor {
    /// Counts only: the fingerprints are not printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TranscriptCursor")
            .field("len", &self.len)
            .field("revision", &self.revision)
            .field("tail", &self.tail_len)
            .finish()
    }
}

enum Found {
    None,
    One(usize),
    Many,
}

/// FNV-1a-64 over an entry's `Hash` form: stable within a build, which is
/// all a session-only cursor needs. It never uses `DefaultHasher`, whose
/// keys a future release could randomise.
fn fingerprint(entry: &MessageEntry) -> u64 {
    let mut hasher = Fnv64::default();
    entry.hash(&mut hasher);
    hasher.finish()
}

struct Fnv64(u64);

impl Default for Fnv64 {
    fn default() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }
}

impl Hasher for Fnv64 {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 ^= u64::from(*byte);
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

#[cfg(test)]
#[path = "transcript/tests.rs"]
mod tests;

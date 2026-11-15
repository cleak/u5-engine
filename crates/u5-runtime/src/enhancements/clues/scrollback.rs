//! The conversation scrollback's rows (Phase B design 8.2.6, 8.3.11; build
//! plan M6-04).
//!
//! [`Scrollback::step`] runs once a frame after the game's step. It keeps
//! its own [`TranscriptCursor`], so it needs neither the clue observer nor
//! the log. When a conversation opens it starts from the Talk command's own
//! echo row; each frame it copies the new transcript entries through the
//! shared slice-diff, so the transcript's 64-entry cap loses nothing; when
//! the conversation ends the rows are cleared. Each entry is wrapped at the
//! window's 16 cells, as the window draws it. Nothing is saved, and no name
//! is added that the window did not show.

use super::TextRow;
use super::text::text_rows_of;
use crate::enhancements::{TranscriptCursor, TranscriptDiff, Unanchored};
use crate::{MESSAGE_TRANSCRIPT_CAPACITY, MessageEntry, PlayState};

/// The most transcript entries one conversation keeps; past it the oldest
/// go.
pub const MAX_SCROLLBACK_ENTRIES: usize = 1024;

/// The fewest entries an overlap must hold to anchor on the scrollback's
/// own rows.
const MIN_OVERLAP: usize = 8;

/// The scrollback of the conversation in progress.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scrollback {
    cursor: Option<TranscriptCursor>,
    /// `Some` while a conversation is open: each entry's window rows.
    entries: Option<Vec<Vec<TextRow>>>,
    /// The same rows, flattened.
    rows: Vec<TextRow>,
    /// The last entry kept is the transcript's last entry, so an in-place
    /// edit of that entry replaces it.
    owns_last: bool,
    /// Diffs that could not be placed.
    dropped: u32,
    /// The transcript's revision at the last step.
    revision: u64,
}

impl Scrollback {
    pub fn new() -> Self {
        Self::default()
    }

    /// The window rows of the conversation in progress, oldest first;
    /// `None` outside a conversation.
    pub fn rows(&self) -> Option<&[TextRow]> {
        self.entries.as_ref().map(|_| self.rows.as_slice())
    }

    /// Diffs that could not anchor while a conversation was open.
    pub fn dropped(&self) -> u32 {
        self.dropped
    }

    /// One frame: follows `active_conversation` and the transcript.
    pub fn step(&mut self, state: &PlayState) {
        let entries = state.message_entries();
        let mut diff = self.cursor.map(|cursor| cursor.diff_state(state));
        self.cursor = Some(TranscriptCursor::of(state));
        let revision = state.message_transcript_revision();
        let pushed = std::mem::replace(&mut self.revision, revision) != revision;
        // At the cap the shared diff can read "nothing new" when the rows it
        // remembers recur at the very end (every prompt ends the same way);
        // a push since the last step says otherwise, so anchor on the
        // scrollback's own rows instead.
        let nothing_new = match diff {
            Some(TranscriptDiff::Unchanged) => true,
            Some(TranscriptDiff::Anchored {
                first_new,
                replaced_last,
            }) => first_new >= entries.len() && !replaced_last,
            _ => false,
        };
        if pushed && nothing_new && entries.len() >= MESSAGE_TRANSCRIPT_CAPACITY {
            diff = Some(TranscriptDiff::Unanchored(Unanchored::Ambiguous));
        }
        let talking = state.active_conversation.is_some();
        let owns_last = self.owns_last;
        match (&mut self.entries, talking) {
            (None, false) => return,
            (Some(_), false) => {
                self.entries = None;
                self.rows.clear();
                self.owns_last = false;
                return;
            }
            (None, true) => {
                // The conversation opened this frame: start at the Talk
                // command's echo, the last echo at or before the first new
                // entry.
                let first_new = match diff {
                    Some(TranscriptDiff::Anchored { first_new, .. }) => first_new,
                    _ => entries.len(),
                }
                .min(entries.len());
                let start = entries[..first_new]
                    .iter()
                    .rposition(|entry| entry.is_command_echo)
                    .or_else(|| {
                        entries[first_new..]
                            .iter()
                            .position(|entry| entry.is_command_echo)
                            .map(|at| at + first_new)
                    })
                    .unwrap_or(first_new);
                self.entries = Some(entries[start..].iter().map(text_rows_of).collect());
                self.owns_last = !entries.is_empty();
            }
            (Some(kept), true) => match diff {
                Some(TranscriptDiff::Unchanged) | None => return,
                Some(TranscriptDiff::Anchored {
                    first_new,
                    replaced_last,
                }) => {
                    if replaced_last && owns_last {
                        kept.pop();
                    }
                    append(kept, &entries[first_new.min(entries.len())..]);
                    self.owns_last = !entries.is_empty();
                }
                Some(TranscriptDiff::Unanchored(_)) => {
                    // The shared diff remembers three entries; the
                    // scrollback holds the whole conversation, so it can
                    // anchor on a long overlap instead. Without one, a
                    // missing row is preferred to a wrong one.
                    let now: Vec<Vec<TextRow>> = entries.iter().map(text_rows_of).collect();
                    match reanchor(kept, &now, owns_last) {
                        Some((first_new, replaced_last)) => {
                            if replaced_last {
                                kept.pop();
                            }
                            kept.extend_from_slice(&now[first_new..]);
                            self.owns_last = !entries.is_empty();
                        }
                        None => {
                            self.dropped = self.dropped.saturating_add(1);
                            self.owns_last = false;
                        }
                    }
                }
            },
        }
        if let Some(kept) = self.entries.as_mut() {
            if kept.len() > MAX_SCROLLBACK_ENTRIES {
                let surplus = kept.len() - MAX_SCROLLBACK_ENTRIES;
                kept.drain(..surplus);
            }
            self.rows = kept.iter().flatten().cloned().collect();
        }
    }
}

fn append(kept: &mut Vec<Vec<TextRow>>, new: &[MessageEntry]) {
    kept.extend(new.iter().map(text_rows_of));
}

/// Where the entries not yet kept start in `now`: just past the one place
/// in `now` where the longest run of the entries kept ends (or the entries
/// kept but for the last, edited in place). `(first_new, replaced_last)`;
/// `None` when no run of [`MIN_OVERLAP`] entries (or all of them) is found
/// once.
fn reanchor(kept: &[Vec<TextRow>], now: &[Vec<TextRow>], owns_last: bool) -> Option<(usize, bool)> {
    let best = |held: &[Vec<TextRow>]| -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize)> = None;
        let mut tie = false;
        for end in 1..=now.len() {
            let run = (0..end.min(held.len()))
                .take_while(|i| now[end - 1 - i] == held[held.len() - 1 - i])
                .count();
            if run == 0 {
                continue;
            }
            match best {
                Some((_, longest)) if run < longest => {}
                Some((_, longest)) if run == longest => tie = true,
                _ => {
                    best = Some((end, run));
                    tie = false;
                }
            }
        }
        best.filter(|(_, run)| !tie && (*run >= MIN_OVERLAP || *run == held.len()))
    };
    let plain = best(kept).map(|(end, run)| (end, run, false));
    let edited = (owns_last && kept.len() > 1)
        .then(|| best(&kept[..kept.len() - 1]))
        .flatten()
        .filter(|(end, _)| *end < now.len())
        .map(|(end, run)| (end, run, true));
    let pick = match (plain, edited) {
        (Some(a), Some(b)) => Some(if b.1 > a.1 { b } else { a }),
        (a, b) => a.or(b),
    };
    pick.map(|(end, _, replaced)| (end, replaced))
}

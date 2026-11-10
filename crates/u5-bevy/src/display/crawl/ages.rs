//! The crawl's row ages (Phase B design 2.2.4, 2.3.8; build plan M1-12).
//!
//! A row's age is the number of game turns since it first appeared
//! (`PlayState::turn`). [`CrawlAges`] keeps, for each entry of the
//! transcript the window projects, the turn it was first seen at, aligned
//! across frames by comparing slices exactly as [`MessageHistory::sync`]
//! does (the transcript can drop entries from the front, edit its last entry
//! in place, take back its newest echo or be cleared). While a conversation
//! is open the crawl's clock stands still, so a whole exchange stays
//! readable; it runs on when the conversation ends.
//!
//! [`MessageHistory::sync`]: super::MessageHistory::sync

use u5_runtime::MessageEntry;

/// Ages at which a row reaches each colour tier: fresh at 0, aged at 1-2,
/// oldest at 3-4, gone from 5 (2.2.4).
pub const AGED_FROM: u64 = 1;
pub const OLDEST_FROM: u64 = 3;
pub const GONE_FROM: u64 = 5;

/// The colour tier of a row of `age` turns: 0 fresh, 1 aged, 2 oldest,
/// `None` gone.
pub fn tier_of(age: u64) -> Option<usize> {
    match age {
        0 => Some(0),
        a if a < OLDEST_FROM => Some(1),
        a if a < GONE_FROM => Some(2),
        _ => None,
    }
}

/// When each transcript entry was first seen, on the crawl's own clock.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrawlAges {
    /// The crawl's clock: game turns counted while no conversation is open.
    clock: u64,
    /// `PlayState::turn` at the last sync.
    last_turn: Option<u64>,
    /// Mirror of the projected transcript at the last sync.
    entries: Vec<MessageEntry>,
    /// `clock` when each entry of `entries` first appeared.
    first_seen: Vec<u64>,
}

impl CrawlAges {
    pub fn new() -> Self {
        Self::default()
    }

    /// The crawl's clock.
    pub fn clock(&self) -> u64 {
        self.clock
    }

    /// Bring the ages up to date: the clock follows `turn` unless
    /// `conversation` is open, then the entries of `now` are aligned with the
    /// last mirror. An entry kept (or edited in place) keeps its first-seen
    /// time; a new one is seen now.
    pub fn sync(&mut self, now: &[MessageEntry], turn: u64, conversation: bool) {
        if let Some(last) = self.last_turn
            && turn > last
            && !conversation
        {
            self.clock += turn - last;
        }
        self.last_turn = Some(turn);
        if self.entries.as_slice() == now {
            return;
        }
        let first_seen = self.align(now);
        self.entries.clear();
        self.entries.extend_from_slice(now);
        self.first_seen = first_seen;
    }

    /// First-seen times for `now`, from the mirror.
    fn align(&self, now: &[MessageEntry]) -> Vec<u64> {
        let Some(dropped) = super::front_drop(&self.entries, now) else {
            return vec![self.clock; now.len()];
        };
        let rest = &self.entries[dropped..];
        let seen = &self.first_seen[dropped..];
        let mut out = Vec::with_capacity(now.len());
        let mut i = 0;
        for entry in now {
            if i < rest.len() && rest[i] == *entry {
                out.push(seen[i]);
                i += 1;
            } else if i + 1 < rest.len() && rest[i + 1] == *entry {
                // One entry withdrawn (the newest echo taken back, a bare
                // marker row withdrawn).
                out.push(seen[i + 1]);
                i += 2;
            } else if i + 1 == rest.len() {
                // The last entry edited in place, or an open row completed:
                // the row first appeared when it was opened.
                out.push(seen[i]);
                i += 1;
            } else {
                out.push(self.clock);
                i = rest.len();
            }
        }
        out
    }

    /// The age in turns of entry `index` of the last `now`.
    pub fn age_of(&self, index: usize) -> u64 {
        self.first_seen
            .get(index)
            .map_or(0, |seen| self.clock.saturating_sub(*seen))
    }
}

#[cfg(test)]
#[path = "ages_tests.rs"]
mod tests;

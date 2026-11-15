//! Chapter 8's journal features in the Bevy shell (Phase B design 8.3.1;
//! build plan M6).
//!
//! - [`clues`]: the clue observer, the live `.TLK` cache, the clue log
//!   resource and its sidecar I/O (M6-02).
//!
//! Chapter 5's quest journal page stays in `crate::quest_journal_view`.

pub mod clues;

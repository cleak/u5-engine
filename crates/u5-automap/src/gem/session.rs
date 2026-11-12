//! One play session's gem map record, as both frontends drive it (Phase B
//! design 10.3.5, 10.3.6; build plan M3-08).
//!
//! The Bevy plugin and the terminal play loop do the same five things, and
//! this type does them once, without Bevy, configuration or OS paths of its
//! own (the save folder comes in as a [`SidecarStore`] the frontend opened):
//!
//! - [`attach`](GemSession::attach): read `gemmap.u5e` against the save on
//!   disk, then learn where the party already is (10.2.8);
//! - [`before`](GemSession::before) and [`after`](GemSession::after): a
//!   probe before a command and the events after it (10.3.3);
//! - [`frame`](GemSession::frame): the cells the faithful view shows, and
//!   the gem view's footprint on the frame it opens;
//! - [`saved`](GemSession::saved): write the record after a detected save,
//!   stamped with it, and never at any other time (D10.3);
//! - [`new_game`](GemSession::new_game): a new character forgets the old
//!   record and removes its file.
//!
//! Every reader takes `&PlayState`; nothing here can change the game.

use std::io;

use u5_runtime::enhancements::{SaveStamp, SidecarStore, SidecarWrite};
use u5_runtime::{PlayState, ViewOverlayMode};

use super::knowledge::{GEM_PAYLOAD_VERSION, GEM_SIDECAR_TAG, GemKnowledge, KnowEvent};
use super::payload::{AttachOutcome, attach, encode_payload};
use super::probe::{KnowProbe, attach_events, observe_commands, probe};
use super::seen::{GemWindow, frame_seen_events, gem_view_events};

/// The `@gemmap` line's first word (design 10.3.6).
pub const GEM_SUMMARY_PREFIX: &str = "gemmap:";

/// The record, the probe in flight and the store it is kept in.
#[derive(Debug, Default)]
pub struct GemSession {
    know: GemKnowledge,
    pending: Option<KnowProbe>,
    /// A gem view was open on the last frame: its footprint is taken once,
    /// on the frame it opens.
    gem_view_open: bool,
    store: Option<SidecarStore>,
    outcome: Option<AttachOutcome>,
}

impl GemSession {
    /// A session kept in memory only: nothing is read or written.
    pub fn new() -> Self {
        Self::default()
    }

    /// A session kept in `store` (`None`: in memory only, for a read-only
    /// install). Nothing is read until [`attach`](Self::attach).
    pub fn with_store(store: Option<SidecarStore>) -> Self {
        Self {
            store,
            ..Self::default()
        }
    }

    /// The record so far.
    pub fn knowledge(&self) -> &GemKnowledge {
        &self.know
    }

    /// What the last attach did with the file, if it read one.
    pub fn outcome(&self) -> Option<AttachOutcome> {
        self.outcome
    }

    /// The store the record is written to, if any.
    pub fn store(&self) -> Option<&SidecarStore> {
        self.store.as_ref()
    }

    /// Whether [`attach`](Self::attach) has run.
    pub fn attached(&self) -> bool {
        self.outcome.is_some()
    }

    /// Reads `gemmap.u5e` against `now`, the stamp of the save on disk,
    /// replacing the record, then learns where the party is (a save loaded
    /// inside a town counts as having been there). With no store or no save
    /// on disk the record starts empty. Any probe in flight is dropped: the
    /// game it was taken from is gone.
    pub fn attach(&mut self, state: &PlayState, now: Option<&SaveStamp>) -> AttachOutcome {
        let (know, outcome) = match (self.store.as_ref(), now) {
            (Some(store), Some(now)) => attach(store, now),
            _ => (
                GemKnowledge::new(),
                AttachOutcome::Empty(super::payload::EmptyReason::Missing),
            ),
        };
        self.know = know;
        self.outcome = Some(outcome);
        self.pending = None;
        self.gem_view_open = false;
        self.fold(&attach_events(state));
        outcome
    }

    /// Takes the probe before a command (or, in Bevy, before the frame's
    /// keys reach the game). Attaches in memory first if nothing has.
    pub fn before(&mut self, state: &PlayState) {
        if !self.attached() {
            self.attach(state, None);
        }
        self.pending = Some(probe(state));
    }

    /// Folds the events since [`before`](Self::before); `commands` is how
    /// many commands ran in between, so that two in one frame keep only
    /// the unambiguous events. Then the frame's seen cells. True when the
    /// record changed.
    pub fn after(&mut self, state: &PlayState, commands: usize) -> bool {
        let mut changed = false;
        if let Some(before) = self.pending.take() {
            changed |= self.fold(&observe_commands(&before, state, commands));
        }
        changed | self.frame(state, None)
    }

    /// The cells the faithful view shows on this frame, and a gem view's
    /// footprint on the frame it opens (`window`: the wider outdoor window
    /// the enhanced map drew, if it drew one).
    pub fn frame(&mut self, state: &PlayState, window: Option<GemWindow>) -> bool {
        let mut changed = self.fold(&frame_seen_events(state));
        let open = state
            .active_view_overlay
            .as_ref()
            .is_some_and(|overlay| overlay.mode == ViewOverlayMode::GemView);
        if open && !self.gem_view_open {
            changed |= self.fold(&gem_view_events(state, window));
        }
        self.gem_view_open = open;
        changed
    }

    /// Writes the record after the save stamped `stamp` was detected. A
    /// record read from a newer build is never written; with no store
    /// nothing is. An error never touches the save itself.
    pub fn saved(&mut self, stamp: &SaveStamp) -> io::Result<Option<SidecarWrite>> {
        let Some(store) = self.store.as_ref() else {
            return Ok(None);
        };
        if self.outcome.is_some_and(|outcome| !outcome.may_write()) {
            return Ok(None);
        }
        store
            .write(
                GEM_SIDECAR_TAG,
                stamp,
                GEM_PAYLOAD_VERSION,
                &encode_payload(&self.know),
            )
            .map(Some)
    }

    /// A new character (creation or the Ultima IV transfer): the record
    /// starts empty and its file is removed, so the next save writes the
    /// new character's record. The session is attached again by the next
    /// [`attach`](Self::attach).
    pub fn new_game(&mut self) -> io::Result<()> {
        self.know = GemKnowledge::new();
        self.pending = None;
        self.gem_view_open = false;
        self.outcome = None;
        match self.store.as_ref() {
            Some(store) => store.reset(&[GEM_SIDECAR_TAG]),
            None => Ok(()),
        }
    }

    /// The `@gemmap` line: how much is recorded, counts only (no place,
    /// coordinate or name; design 10.3.6).
    pub fn summary(&self) -> String {
        summary_line(&self.know)
    }

    fn fold(&mut self, events: &[KnowEvent]) -> bool {
        let mut changed = false;
        for event in events {
            changed |= self.know.apply(event);
        }
        changed
    }
}

/// [`GemSession::summary`] for any record.
pub fn summary_line(know: &GemKnowledge) -> String {
    let [shrines, flags, named] = know.flag_bytes();
    let side_cells: u32 = know
        .sides()
        .values()
        .map(|bits| bits.iter().map(|b| b.count_ones()).sum::<u32>())
        .sum();
    format!(
        "{GEM_SUMMARY_PREFIX} places {}, shrines {}, flags {}, entrances {}, gates {}, signs {}, \
         trades {}, keepers {}, links {}, exits {}, cells {}, marks {}, side cells {}",
        know.entered().len(),
        shrines.count_ones(),
        flags.count_ones(),
        named.count_ones(),
        know.gates().len(),
        know.signs().len(),
        know.trades().len(),
        know.keepers().len(),
        know.town_links().len(),
        know.exits().len(),
        know.cells().total(),
        know.marks().len(),
        side_cells,
    )
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

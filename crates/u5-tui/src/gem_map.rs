//! The enhanced gem map's terminal half (Phase B design 10.3.6; build plan
//! M3-08).
//!
//! The terminal draws no gem map, but it keeps the same record the Bevy
//! shell keeps, so the record and its file can be tested without a window:
//!
//! - `--gem-map` turns it on for a `--play` run (interactive or scripted)
//!   and for `--route-smoke`. Terminal modes read no `config.toml`
//!   (foundation 1.2.4); under the parity lock the flag is ignored and the
//!   foundation's lock line is the only thing printed;
//! - a probe is taken before each command and its events are folded after
//!   it ([`GemSession`]);
//! - in `--play`, the record is read from `gemmap.u5e` beside the save at
//!   the start, and written after each save the [`crate::TerminalSaveWatch`]
//!   detects, stamped with it. `--route-smoke` keeps it in memory only;
//! - `@gemmap` prints a count-only summary ([`GEM_MAP_TOKEN`]). With the
//!   feature off the token is not intercepted and runs as 1.0.1 runs it.

use std::path::{Path, PathBuf};

use u5_automap::gem::{AttachOutcome, GemSession};
use u5_runtime::PlayState;
use u5_runtime::enhancements::{EffectClass, SaveStamp, SidecarStore, Toggle};

use crate::equivalence::TerminalFeature;

/// The gem map's play-script token: a count-only summary of the record.
pub const GEM_MAP_TOKEN: &str = "@gemmap";

/// Printed once when the record belongs to the same Avatar's later save
/// (design 1.3.10, `SameLineage`).
pub const GEM_STALE_NOTE: &str = "gem map: the discoveries are from another save of this game";
/// Printed when the record could not be written. The save itself is
/// already on disk and is never touched.
pub const GEM_WRITE_ERROR: &str = "gem map: the discoveries could not be saved";

/// The record of one terminal run.
#[derive(Debug, Default)]
pub struct TerminalGemMap {
    session: GemSession,
    /// The folder holding `SAVED.GAM` and its sidecar folder, when the
    /// record is kept on disk.
    save_dir: Option<PathBuf>,
}

impl TerminalGemMap {
    /// A record kept in memory only: nothing is read or written.
    pub fn in_memory() -> Self {
        Self::default()
    }

    /// A record kept beside the save in `save_dir`. Nothing is read until
    /// [`attach`](Self::attach).
    pub fn for_save_dir(save_dir: &Path) -> Self {
        Self {
            session: GemSession::with_store(SidecarStore::open(save_dir)),
            save_dir: Some(save_dir.to_path_buf()),
        }
    }

    /// Reads `gemmap.u5e` against the save on disk, then learns where the
    /// party is. Prints the staleness note when the record is another
    /// save's of the same game.
    pub fn attach(&mut self, state: &PlayState) -> AttachOutcome {
        let now = self.save_dir.as_deref().and_then(SaveStamp::read);
        let outcome = self.session.attach(state, now.as_ref());
        if outcome.shows_stale_note() {
            eprintln!("{GEM_STALE_NOTE}");
        }
        outcome
    }

    /// Before a command.
    pub fn before(&mut self, state: &PlayState) {
        self.session.before(state);
    }

    /// After a command.
    pub fn after(&mut self, state: &PlayState) {
        self.session.after(state, 1);
    }

    /// After the save stamped `stamp` was detected: writes the record. An
    /// error is printed and never fails the run.
    pub fn saved(&mut self, stamp: &SaveStamp) {
        if let Err(error) = self.session.saved(stamp) {
            eprintln!("{GEM_WRITE_ERROR}: {error}");
        }
    }

    /// The record's session.
    pub fn session(&self) -> &GemSession {
        &self.session
    }

    /// The `@gemmap` line.
    pub fn summary(&self) -> String {
        self.session.summary()
    }

    /// Whether `command` is the gem map's token.
    pub fn owns(command: &str) -> bool {
        command.trim() == GEM_MAP_TOKEN
    }
}

impl TerminalFeature for TerminalGemMap {
    fn toggle(&self) -> Toggle {
        Toggle::GemMapEnhanced
    }

    fn class(&self) -> EffectClass {
        EffectClass::Presentation
    }

    fn owns_token(&self, token: &str) -> bool {
        Self::owns(token)
    }

    fn on_token(&mut self, _token: &str, _state: &PlayState) -> Vec<String> {
        println!("{}", self.summary());
        Vec::new()
    }

    fn before_command(&mut self, state: &PlayState) {
        self.before(state);
    }

    fn after_command(&mut self, state: &PlayState) {
        self.after(state);
    }
}

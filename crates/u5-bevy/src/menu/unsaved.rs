//! Unsaved progress (Phase B design 3.3.5 "Unsaved progress", 3.2.6; plan
//! M4-11).
//!
//! The menu keeps `last_saved_turn`, the game's turn counter
//! (`PlayState.turn`) when the working save last matched the game, from
//! three moments:
//!
//! 1. after a menu save, once the game's own `Q` and `Y` wrote it (the
//!    slot plugin calls [`SaveTracker::saved_at`]);
//! 2. at the first frame of play after a load: gameplay always starts
//!    from the save on disk (Journey Onward, a new character's first save,
//!    the boot `J` after a slot load);
//! 3. after a typed `Q` save the game answered, as the foundation's save
//!    watcher announces it ([`SaveStamped`]).
//!
//! It only reads the game. The top menu shows the turns since in its
//! footer, and the confirmations (M4-13) ask before losing them.

use std::path::PathBuf;

use bevy::prelude::*;
use u5_runtime::enhancements::{FeatureToggles, Toggle};

use crate::VisualState;
use crate::enh::EnhSet;
use crate::enh::sidecar_io::{SaveStamped, SaveWatch, install_save_watcher, save_watcher};

/// The turn of the last save or load seen this session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveTracker {
    /// `None` until play starts.
    pub last_saved_turn: Option<u64>,
}

impl SaveTracker {
    /// The working save matched the game at `turn`.
    pub fn saved_at(&mut self, turn: u64) {
        self.last_saved_turn = Some(turn);
    }

    /// Turns since then at `turn`; `None` before the first.
    pub fn turns_since(&self, turn: u64) -> Option<u64> {
        self.last_saved_turn.map(|last| turn.saturating_sub(last))
    }
}

/// Adds the tracker. With a save folder it also asks for the save
/// watcher (it only reads `SAVED.GAM`), so a typed save counts.
pub(crate) fn install_save_tracker(app: &mut App, save_dir: Option<PathBuf>) {
    app.init_resource::<SaveTracker>().add_systems(
        Update,
        track_play_start
            .run_if(play_started)
            .in_set(EnhSet::Observe),
    );
    if let Some(save_dir) = save_dir {
        let mut requesters = FeatureToggles::CLASSIC;
        requesters.set(Toggle::MenuOverlay, true);
        install_save_watcher(app, save_dir, requesters);
    }
    if app.world().contains_resource::<SaveWatch>() {
        app.add_systems(
            Update,
            track_typed_saves
                .run_if(on_event::<SaveStamped>)
                .in_set(EnhSet::Observe)
                .after(save_watcher),
        );
    }
}

fn play_started(visual: Option<Res<VisualState>>) -> bool {
    visual.is_some_and(|visual| visual.is_added())
}

/// Moment 2: the first frame of play.
fn track_play_start(visual: Res<VisualState>, mut tracker: ResMut<SaveTracker>) {
    tracker.saved_at(visual.state.turn);
}

/// Moment 3: a save the game wrote.
fn track_typed_saves(
    mut stamped: EventReader<SaveStamped>,
    visual: Option<Res<VisualState>>,
    mut tracker: ResMut<SaveTracker>,
) {
    let saved = stamped.read().count() > 0;
    if let (true, Some(visual)) = (saved, visual) {
        tracker.saved_at(visual.state.turn);
    }
}

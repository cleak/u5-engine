//! The save watcher in the Bevy shell (Phase B design 1.3.9, 1.3.10).
//!
//! Features that keep a per-save sidecar write it only after a save, stamped
//! with that save's bytes. The runtime's [`SaveWatcher`] detects the save
//! from outside the game; this module runs it once per frame in
//! `EnhSet::Observe` and announces each new save as a [`SaveStamped`]
//! event:
//!
//! - In play, a frame that changed `VisualState` polls the watcher: the
//!   `Q`, `Y` through the prompt, a buffered `Q`, `Y` inside one frame, and
//!   the menu's slot save (which types the game's own `Q`, `Y`) alike.
//! - A new game (character creation or the Ultima IV transfer, counted by
//!   the intro) resets the watcher and polls it, so the fresh `SAVED.GAM`
//!   gives a stamp with `new_game` set; the features reset their sidecars
//!   on it.
//! - Gameplay starting (the play app's first frame, Journey Onward) takes
//!   the save on disk as seen, without a stamp. So does a sidecar feature
//!   turning on in play.
//! - A slot load asks for one hash of the rewritten file
//!   ([`SaveWatch::request_rewrite_poll`]).
//!
//! Zero footprint: the watcher is installed only while a sidecar feature is
//! on at launch ([`sidecar_feature_on`]) and the host handed in a save
//! folder; its system runs only while one of the toggles that asked for it
//! is on. It only reads `SAVED.GAM`: the game's save bytes are the same
//! with and without it.

use std::path::{Path, PathBuf};

use bevy::prelude::*;
use u5_runtime::enhancements::{
    FeatureToggles, SIDECAR_KEYS, SaveStamp, SaveWatcher, Toggle, sidecar_feature_on,
};

use super::{EnhSet, Toggles};
use crate::{VisualIntroState, VisualState};

/// A save was written: its stamp. Sent at most once per save.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveStamped {
    pub stamp: SaveStamp,
    /// The save of a new character (creation or the Ultima IV transfer):
    /// the features reset their sidecars before they write.
    pub new_game: bool,
}

/// The watcher, the folder it watches and what it was asked to do. Exists
/// only while [`install_save_watcher`] put it in the app.
#[derive(Resource, Debug)]
pub struct SaveWatch {
    save_dir: PathBuf,
    /// The toggles whose features asked for the watcher; it runs while
    /// one of them is on.
    requesters: FeatureToggles,
    /// `None` until gameplay or a new game first gives it a save to know.
    watcher: Option<SaveWatcher>,
    /// The intro's new-game count already handled.
    new_games_seen: u64,
    prime_requested: bool,
    rewrite_requested: bool,
}

impl SaveWatch {
    fn new(save_dir: PathBuf, requesters: FeatureToggles) -> Self {
        Self {
            save_dir,
            requesters,
            watcher: None,
            new_games_seen: 0,
            prime_requested: false,
            rewrite_requested: false,
        }
    }

    /// The folder holding `SAVED.GAM`, as the host handed it in.
    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }

    /// The stamp of the save on disk as last seen: what a feature compares
    /// its sidecar with when it reads it.
    pub fn last_stamp(&self) -> Option<SaveStamp> {
        self.watcher.as_ref().and_then(SaveWatcher::last_stamp)
    }

    /// A slot load rewrote `SAVED.GAM`: the next frame hashes it and
    /// stamps it when it is another save.
    pub fn request_rewrite_poll(&mut self) {
        self.rewrite_requested = true;
    }

    /// Whether the watcher should run under `toggles`.
    pub fn wanted(&self, toggles: FeatureToggles) -> bool {
        self.requesters.iter_on().any(|toggle| toggles.on(toggle))
    }

    /// The toggles changed in play from `before` to `after`. When a
    /// requesting feature turned on, the save on disk is taken as seen,
    /// so a save made while it was off gives no stamp.
    pub(super) fn toggles_changed(&mut self, before: FeatureToggles, after: FeatureToggles) {
        if self.wanted(after) && !self.wanted(before) {
            self.prime_requested = true;
        }
    }
}

/// The catalogue toggles in [`SIDECAR_KEYS`].
fn sidecar_toggles() -> FeatureToggles {
    let mut toggles = FeatureToggles::CLASSIC;
    for toggle in SIDECAR_KEYS.iter().filter_map(|key| Toggle::from_key(key)) {
        toggles.set(toggle, true);
    }
    toggles
}

/// Adds the watcher when a sidecar feature is on and the host gave a save
/// folder; otherwise adds nothing. Called from [`super::EnhancementsPlugin`].
pub(super) fn add_save_watcher(app: &mut App, toggles: FeatureToggles, save_dir: Option<&Path>) {
    if sidecar_feature_on(toggles)
        && let Some(save_dir) = save_dir
    {
        install_save_watcher(app, save_dir.to_path_buf(), sidecar_toggles());
    }
}

/// The watcher, its event and its system, for `save_dir`, run while one of
/// `requesters` is on. A second call adds its requesters to the first.
pub fn install_save_watcher(app: &mut App, save_dir: PathBuf, requesters: FeatureToggles) {
    if let Some(mut watch) = app.world_mut().get_resource_mut::<SaveWatch>() {
        for toggle in requesters.iter_on() {
            watch.requesters.set(toggle, true);
        }
        return;
    }
    app.insert_resource(SaveWatch::new(save_dir, requesters))
        .add_event::<SaveStamped>()
        .add_systems(
            Update,
            save_watcher
                .run_if(save_watch_wanted)
                .in_set(EnhSet::Observe),
        );
}

fn save_watch_wanted(watch: Res<SaveWatch>, toggles: Option<Res<Toggles>>) -> bool {
    toggles.is_some_and(|toggles| watch.wanted(toggles.0))
}

/// Polls the watcher for this frame and sends a [`SaveStamped`] per new
/// save. See the module documentation for when it hashes.
pub(super) fn save_watcher(
    mut watch: ResMut<SaveWatch>,
    visual: Option<Res<VisualState>>,
    intro: Option<Res<VisualIntroState>>,
    mut stamped: EventWriter<SaveStamped>,
) {
    let SaveWatch {
        save_dir,
        watcher,
        new_games_seen,
        prime_requested,
        rewrite_requested,
        ..
    } = &mut *watch;
    let mut send = |stamp: Option<SaveStamp>, new_game: bool| {
        if let Some(stamp) = stamp {
            stamped.write(SaveStamped { stamp, new_game });
        }
    };

    if let Some(intro) = intro
        && intro.new_game_saves != *new_games_seen
    {
        *new_games_seen = intro.new_game_saves;
        let watcher = watcher.get_or_insert_with(SaveWatcher::default);
        watcher.reset();
        send(watcher.poll_rewritten(save_dir), true);
    }

    let Some(visual) = visual else {
        return;
    };
    let watcher = match watcher {
        Some(watcher) if !visual.is_added() && !*prime_requested => watcher,
        _ => {
            *prime_requested = false;
            *rewrite_requested = false;
            *watcher = Some(SaveWatcher::new(save_dir));
            return;
        }
    };
    if std::mem::take(rewrite_requested) {
        send(watcher.poll_rewritten(save_dir), false);
    } else if visual.is_changed() {
        send(watcher.poll(&visual.state, save_dir), false);
    }
}

#[cfg(test)]
mod tests;

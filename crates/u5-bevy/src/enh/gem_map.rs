//! The enhanced gem map in the Bevy shell, record half (Phase B design
//! 10.3.5; build plan M3-08).
//!
//! The record of what the player has discovered ([`GemSession`], in
//! `u5-automap`) is filled and kept here; the gem map image arrives with
//! M3-11. Four systems, each in one of the foundation's [`EnhSet`]s:
//!
//! - `gem_attach` (`Feed`, before the probe): reads `gemmap.u5e` when a
//!   `VisualState` appears, after a slot load ([`GemMap::request_attach`])
//!   and when the toggle turns on at the idle prompt;
//! - `gem_probe_pre` (`Feed`, last, after every synthetic key source):
//!   the probe, before the input driver runs this frame's keys;
//! - `gem_probe_post` (`Observe`): the events, the seen cells and, on the
//!   frame a gem view opens, its footprint;
//! - `gem_persist` (`Observe`, after the save watcher): writes the record
//!   on each new save, stamped with it. A new character's stamp
//!   (character creation or the Ultima IV transfer) first forgets the old
//!   record and removes its file.
//!
//! Zero footprint: the plugin is added only when `gem_map.enhanced` is on
//! at launch, never under the parity lock (the foundation plugin itself is
//! absent then). Every system runs only while the toggle is on, and all
//! but the new-game reset only while a `VisualState` exists: the reset
//! answers a stamp the intro sends before gameplay starts. No key is bound
//! and nothing is drawn, so the game's frame, keys and state are the 1.0.1
//! ones; every reader takes `&PlayState`.

use bevy::prelude::*;
use u5_automap::gem::{AttachOutcome, GEM_SIDECAR_TAG, GemKnowledge, GemSession};
use u5_runtime::enhancements::{
    FeatureToggles, SaveStamp, SidecarStore, Toggle, sidecar_file_name,
};

use super::input::is_modifier;
use super::sidecar_io::{SaveStamped, SaveWatch};
use super::toast::Toasts;
use super::{EnhLaunch, EnhSet, VisualLaunchConfig, feature_wanted, toggle_on};
use crate::VisualState;

/// The toggle this module belongs to.
const GEM: Toggle = Toggle::GemMapEnhanced;

/// Shown once when the record belongs to the same Avatar's later save
/// (design 1.3.10, `SameLineage`).
pub const GEM_STALE_NOTE: &str = "Gem map: the discoveries are from another save of this game.";
/// Shown once when the record could not be written. The save itself is
/// already on disk and is never touched.
pub const GEM_WRITE_ERROR: &str = "Gem map: the discoveries could not be saved.";
/// Logged when a new character's old record could not be removed.
const GEM_RESET_ERROR: &str = "gem map: the old discoveries could not be removed";

/// The record and what the systems carry between them. Exists only while
/// the plugin is in the app.
#[derive(Resource, Debug)]
pub struct GemMap {
    session: GemSession,
    /// How many commands may run between this frame's probe and its
    /// observation: the keys waiting plus the keys pressed this frame. Two
    /// or more keep only the unambiguous events (10.3.3).
    commands: usize,
    attach_requested: bool,
}

impl Default for GemMap {
    fn default() -> Self {
        Self {
            session: GemSession::new(),
            commands: 0,
            attach_requested: true,
        }
    }
}

impl GemMap {
    /// The record so far.
    pub fn knowledge(&self) -> &GemKnowledge {
        self.session.knowledge()
    }

    /// What the last attach did with `gemmap.u5e`.
    pub fn outcome(&self) -> Option<AttachOutcome> {
        self.session.outcome()
    }

    /// A slot load rewrote the save: the next frame reads its record.
    pub fn request_attach(&mut self) {
        self.attach_requested = true;
    }

    /// The toggles changed in play from `before` to `after` (called by the
    /// foundation's `apply_pending_toggles`). Turning the gem map on reads
    /// the record of the save on disk at the next frame.
    pub(super) fn toggles_changed(&mut self, before: FeatureToggles, after: FeatureToggles) {
        if after.on(GEM) && !before.on(GEM) {
            self.attach_requested = true;
        }
    }
}

/// Adds the plugin when the gem map is on at launch. Called from
/// [`super::EnhancementsPlugin`], which is itself absent under Classic and
/// under the parity lock.
pub(super) fn add_gem_map(app: &mut App, launch: &VisualLaunchConfig) {
    if feature_wanted(launch.enhancements.toggles, GEM) {
        app.add_plugins(GemMapPlugin);
    }
}

/// The gem map's feature plugin.
pub struct GemMapPlugin;

impl Plugin for GemMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GemMap>();
        app.add_systems(
            Update,
            (
                gem_attach.before(gem_probe_pre),
                gem_probe_pre.after(super::synthetic::feed_synthetic_keys),
            )
                .in_set(EnhSet::Feed)
                .run_if(resource_exists::<VisualState>)
                .run_if(toggle_on(GEM)),
        )
        .add_systems(
            Update,
            gem_probe_post
                .in_set(EnhSet::Observe)
                .run_if(resource_exists::<VisualState>)
                .run_if(toggle_on(GEM)),
        );
        // The save watcher exists only when the host gave a save folder; it
        // owns the `SaveStamped` event.
        if app.world().contains_resource::<SaveWatch>() {
            app.add_systems(
                Update,
                gem_persist
                    .in_set(EnhSet::Observe)
                    .after(super::sidecar_io::save_watcher)
                    .after(gem_probe_post)
                    .run_if(toggle_on(GEM)),
            );
        }
    }
}

/// The save folder the host handed in, if any.
fn save_dir(launch: &EnhLaunch, watch: Option<&SaveWatch>) -> Option<std::path::PathBuf> {
    watch
        .map(|watch| watch.save_dir().to_path_buf())
        .or_else(|| launch.0.save_dir.clone())
}

/// Reads the record when gameplay starts, after a slot load and when the
/// toggle turns on (10.3.5). With no save folder, or a read-only install,
/// the record lives in memory only.
fn gem_attach(
    mut gem: ResMut<GemMap>,
    visual: Res<VisualState>,
    launch: Res<EnhLaunch>,
    watch: Option<Res<SaveWatch>>,
    toasts: Option<ResMut<Toasts>>,
) {
    if !visual.is_added() && !gem.attach_requested {
        return;
    }
    gem.attach_requested = false;
    let dir = save_dir(&launch, watch.as_deref());
    let store = dir.as_deref().and_then(SidecarStore::open);
    let now = dir.as_deref().and_then(SaveStamp::read);
    gem.session = GemSession::with_store(store);
    let outcome = gem.session.attach(&visual.state, now.as_ref());
    if outcome.shows_stale_note() {
        info!("{GEM_STALE_NOTE}");
        if let Some(mut toasts) = toasts {
            toasts.note(GEM_STALE_NOTE);
        }
    }
}

/// The probe, just before the input driver runs this frame's keys.
fn gem_probe_pre(
    mut gem: ResMut<GemMap>,
    visual: Res<VisualState>,
    keyboard: Option<Res<ButtonInput<KeyCode>>>,
) {
    let waiting = visual
        .typed_keys
        .iter()
        .filter(|typed| !is_modifier(typed.key))
        .count();
    let pressed = keyboard.map_or(0, |keyboard| {
        keyboard
            .get_just_pressed()
            .filter(|key| !is_modifier(**key))
            .count()
    });
    gem.commands = waiting + pressed;
    gem.session.before(&visual.state);
}

/// The events since the probe, and this frame's seen cells.
fn gem_probe_post(mut gem: ResMut<GemMap>, visual: Res<VisualState>) {
    let commands = gem.commands;
    gem.session.after(&visual.state, commands);
}

/// Writes the record after each new save (10.3.7). A new character's stamp
/// first removes the old record, so the new character starts empty.
fn gem_persist(
    mut gem: ResMut<GemMap>,
    mut stamped: EventReader<SaveStamped>,
    launch: Res<EnhLaunch>,
    watch: Res<SaveWatch>,
    visual: Option<Res<VisualState>>,
    mut toasts: Option<ResMut<Toasts>>,
) {
    for event in stamped.read() {
        if event.new_game {
            // Forget the old record and remove its file through the save
            // folder: in the intro no session has attached a store yet.
            gem.session = GemSession::new();
            gem.attach_requested = true;
            if let Some(store) = save_dir(&launch, Some(&watch))
                .as_deref()
                .and_then(SidecarStore::open)
                && let Err(error) = store.reset(&[GEM_SIDECAR_TAG])
            {
                warn!("{GEM_RESET_ERROR}: {error}");
            }
            continue;
        }
        if visual.is_none() || !gem.session.attached() {
            continue;
        }
        if let Err(error) = gem.session.saved(&event.stamp) {
            warn!(
                "{GEM_WRITE_ERROR} ({}): {error}",
                sidecar_file_name(GEM_SIDECAR_TAG).unwrap_or_default()
            );
            if let Some(toasts) = toasts.as_mut() {
                toasts.note(GEM_WRITE_ERROR);
            }
        }
    }
}

#[cfg(test)]
mod tests;

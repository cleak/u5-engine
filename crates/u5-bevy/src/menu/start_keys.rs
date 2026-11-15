//! The start keys (Phase B design 3.2.7 "Game", 3.3.8 "Injection", 3.4.2;
//! plan M4-13).
//!
//! `audio.start_sound_off` and `audio.start_buffer_on` type the game's own
//! Ctrl-S and Ctrl-B once per process, at the first idle exploration
//! prompt, and only when the live value still differs from the one wanted:
//!
//! 1. the first eligible frame types Ctrl-S if `start_sound_off` is on and
//!    `music_enabled` is still true;
//! 2. a later frame, once the game has read that key and is back at its
//!    prompt, types Ctrl-B if `start_buffer_on` is on and
//!    `typeahead_buffer_enabled` is still false.
//!
//! A preference whose value already matches is done without a key. Neither
//! value is saved in the game and a new process starts with Sound On and
//! Buffer Off (`audio.md §3`, `input.md §6`), so a title Continue in the
//! same process needs no second key, and a relaunch is a new process with
//! its own plugin.
//!
//! [`StartKeysPlugin`] (owner [`SyntheticOwner::StartKeys`]) is added by a
//! start preference alone, so the keys work with the overlay off (3.4.2).
//! With both preferences off it is not in the app. The game prints its own
//! echo, and neither key costs a turn (`commands.md §9`).

use std::time::Duration;

use bevy::prelude::*;
use u5_runtime::enhancements::{AudioSettings, IdlePromptKind};

use super::inject::{BUFFER_KEY, SOUND_KEY, STEP_LIMIT};
use crate::VisualState;
use crate::enh::idle::IdleProbe;
use crate::enh::overlay::{OverlayState, apply_overlay_requests};
use crate::enh::synthetic::{
    SyntheticInputOwner, SyntheticKey, SyntheticOwner, install_synthetic_input,
};
use crate::enh::{EnhSet, VisualLaunchConfig};

/// Adds [`StartKeysPlugin`] when a start preference is on at launch;
/// otherwise nothing. Called from the enhancements plugin, which is absent
/// under Classic and the parity lock.
pub(crate) fn add_start_keys(app: &mut App, launch: &VisualLaunchConfig) {
    let audio = launch.enhancements.audio();
    if audio.types_start_keys() {
        app.add_plugins(StartKeysPlugin::new(audio));
    }
}

/// The start keys' feature plugin.
pub struct StartKeysPlugin {
    sound_off: bool,
    buffer_on: bool,
}

impl StartKeysPlugin {
    pub fn new(audio: AudioSettings) -> Self {
        Self {
            sound_off: audio.start_sound_off,
            buffer_on: audio.start_buffer_on,
        }
    }
}

impl Plugin for StartKeysPlugin {
    fn build(&self, app: &mut App) {
        install_synthetic_input(app);
        app.insert_resource(StartKeys::new(self.sound_off, self.buffer_on))
            .add_systems(
                Update,
                start_keys
                    .run_if(start_keys_left)
                    .in_set(EnhSet::Apply)
                    .after(apply_overlay_requests),
            );
    }
}

/// Where one start preference is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartStep {
    /// Wanted, not yet decided.
    Wanted,
    /// Its key is queued or typed and not yet read.
    Typing,
    /// Typed and read, found already matching, or not wanted.
    Done,
}

/// The two start preferences' progress, kept for the whole process.
#[derive(Resource, Clone, Debug, PartialEq, Eq)]
pub struct StartKeys {
    pub sound: StartStep,
    pub buffer: StartStep,
    /// Keys this plugin typed, in order (for the log and the tests).
    pub typed: Vec<SyntheticKey>,
    waited: Duration,
}

impl StartKeys {
    fn new(sound_off: bool, buffer_on: bool) -> Self {
        let step = |on: bool| {
            if on {
                StartStep::Wanted
            } else {
                StartStep::Done
            }
        };
        Self {
            sound: step(sound_off),
            buffer: step(buffer_on),
            typed: Vec::new(),
            waited: Duration::ZERO,
        }
    }

    /// Both preferences are done.
    pub fn done(&self) -> bool {
        self.sound == StartStep::Done && self.buffer == StartStep::Done
    }
}

fn start_keys_left(keys: Res<StartKeys>) -> bool {
    !keys.done()
}

const EXPLORATION: &[IdlePromptKind] = &[IdlePromptKind::Exploration];

/// One step a frame: wait for a typed key to be read, or decide the next
/// preference at the idle exploration prompt.
pub(crate) fn start_keys(
    time: Option<Res<Time>>,
    mut keys: ResMut<StartKeys>,
    mut owner: ResMut<SyntheticInputOwner>,
    probe: IdleProbe,
    visual: Option<Res<VisualState>>,
    overlay: Option<Res<OverlayState>>,
) {
    let Some(visual) = visual else {
        return;
    };
    let keys = &mut *keys;
    if let Some(step) = [&mut keys.sound, &mut keys.buffer]
        .into_iter()
        .find(|step| **step == StartStep::Typing)
    {
        let delta = time.map_or(Duration::ZERO, |time| time.delta());
        keys.waited = keys.waited.saturating_add(delta);
        let read = owner.queued() == 0 && visual.typed_keys.is_empty();
        if read || keys.waited >= STEP_LIMIT {
            owner.release(SyntheticOwner::StartKeys);
            *step = StartStep::Done;
        }
        return;
    }
    let overlay_open = overlay.is_some_and(|overlay| overlay.is_open());
    if overlay_open || !probe.view().at(EXPLORATION) || owner.holder().is_some() {
        return;
    }
    let (step, differs, key) = if keys.sound == StartStep::Wanted {
        (&mut keys.sound, visual.state.music_enabled, SOUND_KEY)
    } else if keys.buffer == StartStep::Wanted {
        (
            &mut keys.buffer,
            !visual.state.typeahead_buffer_enabled,
            BUFFER_KEY,
        )
    } else {
        return;
    };
    if !differs {
        *step = StartStep::Done;
        return;
    }
    if owner.take(SyntheticOwner::StartKeys).is_err()
        || owner.queue(SyntheticOwner::StartKeys, [key]).is_err()
    {
        owner.release(SyntheticOwner::StartKeys);
        return;
    }
    *step = StartStep::Typing;
    keys.typed.push(key);
    keys.waited = Duration::ZERO;
}

#[cfg(test)]
#[path = "start_keys_tests.rs"]
mod tests;

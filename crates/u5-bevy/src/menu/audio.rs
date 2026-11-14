//! Output gain and focus mute (Phase B design 3.3.7; build plan M4-08).
//!
//! Two plugins, each added from the enhancements plugin only by its own
//! condition in the 3.4.2 table, which is also the condition the Audio
//! tab's apply points read (`u5_runtime::enhancements::installed_at_launch`):
//!
//! - [`VolumePlugin`]: the menu overlay is on, or `audio.volume` is below
//!   100, or `audio.muted` is on, or focus mute brings it. It keeps
//!   [`OutputGain`] and `sync_gain`, which puts the gain on Bevy's
//!   `GlobalVolume` (for the voices started later) and on the sink of the
//!   [`SpeakerVoice`] sounding now, at `SPEAKER_VOLUME * gain`.
//! - [`FocusMutePlugin`]: `audio.mute_on_focus_loss` is on, or the overlay
//!   is (so the Audio tab can turn it on in play). `focus_mute` sets the
//!   automatic mute while the window is unfocused and
//!   [`FocusMuteEnabled`] holds; with only the overlay on it never does.
//!
//! What this never touches: the game's Sound flag (`music_enabled`), a
//! queued program's `audible` flag and the voice's
//! `SpeakerVoiceLifetime`. Muting here is output gain only, so a muted
//! voice still owns the speaker for its full time and speaker-busy gating
//! and type-ahead behave as with sound (3.2.7). With every value at its
//! default the gain is 1.0, `GlobalVolume` stays 1.0 and voices stay at
//! 0.9: the 1.0.1 output.
//!
//! Both follow [`LiveSettings`], so a change on the Audio tab applies on
//! the next frame, including to a sound already playing.

use bevy::audio::{AudioSink, AudioSinkPlayback, GlobalVolume, Volume};
use bevy::prelude::*;
use bevy::window::WindowFocused;
use u5_runtime::enhancements::{AudioSettings, FeatureToggles, Toggle, installed_at_launch};

use crate::enh::{EnhSet, LiveSettings, VisualLaunchConfig, apply_pending_changes};
use crate::{SPEAKER_VOLUME, SpeakerVoice};

/// The output gain the player chose, and the automatic mute (3.3.7).
/// Exists only while [`VolumePlugin`] is in the app.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct OutputGain {
    /// `audio.volume / 100`.
    pub volume: f32,
    /// The player's own mute (`audio.muted`): set only from the settings.
    pub user_muted: bool,
    /// Set only by `focus_mute`.
    pub auto_muted: bool,
}

impl OutputGain {
    /// The gain for `settings`, with no automatic mute.
    pub fn of(settings: &AudioSettings) -> Self {
        Self {
            volume: settings.volume(),
            user_muted: settings.muted,
            auto_muted: false,
        }
    }

    /// The gain applied to the output: 0 while either mute holds.
    pub fn effective(&self) -> f32 {
        if self.user_muted || self.auto_muted {
            0.0
        } else {
            self.volume
        }
    }
}

/// Whether focus mute is on now (`audio.mute_on_focus_loss`). Exists only
/// while [`FocusMutePlugin`] is in the app.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FocusMuteEnabled(pub bool);

/// Whether [`VolumePlugin`] belongs in an app launched with `toggles`
/// (3.4.2): the overlay, a volume, the player's mute, or focus mute.
pub fn volume_wanted(toggles: FeatureToggles) -> bool {
    installed_at_launch(Toggle::AudioVolume, toggles)
}

/// Whether [`FocusMutePlugin`] belongs in an app launched with `toggles`
/// (3.4.2): focus mute or the overlay.
pub fn focus_mute_wanted(toggles: FeatureToggles) -> bool {
    installed_at_launch(Toggle::AudioMuteOnFocusLoss, toggles)
}

/// Adds the audio plugins `launch` asks for, each by its own condition.
/// Called from the enhancements plugin, which is absent under Classic and
/// the parity lock.
pub(crate) fn add_audio(app: &mut App, launch: &VisualLaunchConfig) {
    let toggles = launch.enhancements.toggles;
    let settings = launch.enhancements.audio();
    // Focus mute needs the gain: an internal dependency of one feature, not
    // of another toggle (3.3.7). `volume_wanted` holds whenever
    // `focus_mute_wanted` does, so the gain starts from the launch's values.
    if volume_wanted(toggles) {
        app.add_plugins(VolumePlugin::new(settings));
    }
    if focus_mute_wanted(toggles) {
        app.add_plugins(FocusMutePlugin::new(settings.mute_on_focus_loss));
    }
}

/// Keeps [`OutputGain`] and puts it on the output (3.3.7).
pub struct VolumePlugin {
    settings: AudioSettings,
}

impl VolumePlugin {
    pub fn new(settings: AudioSettings) -> Self {
        Self { settings }
    }
}

impl Plugin for VolumePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(OutputGain::of(&self.settings))
            .add_systems(
                Update,
                (
                    follow_audio_settings
                        .run_if(resource_exists_and_changed::<LiveSettings>)
                        .after(apply_pending_changes),
                    sync_gain.run_if(resource_changed::<OutputGain>),
                )
                    .chain()
                    .in_set(EnhSet::Apply),
            );
    }
}

/// The automatic mute while the window is unfocused (3.3.7).
pub struct FocusMutePlugin {
    enabled: bool,
}

impl FocusMutePlugin {
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }
}

impl Plugin for FocusMutePlugin {
    fn build(&self, app: &mut App) {
        if !app.is_plugin_added::<VolumePlugin>() {
            // `add_audio` passes the launch's settings; a plugin added on
            // its own starts from the defaults.
            app.add_plugins(VolumePlugin::new(AudioSettings {
                mute_on_focus_loss: self.enabled,
                ..AudioSettings::default()
            }));
        }
        // The window plugin registers it; a headless app may not have one.
        app.add_event::<WindowFocused>()
            .insert_resource(FocusMuteEnabled(self.enabled))
            .add_systems(
                Update,
                (
                    follow_focus_mute_setting
                        .run_if(resource_exists_and_changed::<LiveSettings>)
                        .after(apply_pending_changes),
                    focus_mute,
                )
                    .chain()
                    .before(sync_gain)
                    .in_set(EnhSet::Apply),
            );
    }
}

/// A change in play (the Audio tab) reaches the gain: the volume and the
/// player's mute. Written only when something changed, so `sync_gain`
/// runs only for a real change.
fn follow_audio_settings(live: Res<LiveSettings>, mut gain: ResMut<OutputGain>) {
    let settings = live.now.audio();
    let (volume, user_muted) = (settings.volume(), settings.muted);
    if gain.volume != volume || gain.user_muted != user_muted {
        gain.volume = volume;
        gain.user_muted = user_muted;
    }
}

/// A change in play reaches [`FocusMuteEnabled`].
fn follow_focus_mute_setting(live: Res<LiveSettings>, mut enabled: ResMut<FocusMuteEnabled>) {
    enabled.set_if_neq(FocusMuteEnabled(live.now.audio().mute_on_focus_loss));
}

/// Sets the automatic mute from the window's focus while focus mute is
/// on, and releases it at once when focus mute is turned off. Only a focus
/// change or turning it off releases it; the player's own mute is never
/// touched.
fn focus_mute(
    mut focus: EventReader<WindowFocused>,
    enabled: Res<FocusMuteEnabled>,
    mut gain: ResMut<OutputGain>,
) {
    if enabled.is_changed() && !enabled.0 && gain.auto_muted {
        gain.auto_muted = false;
    }
    for event in focus.read() {
        let want = enabled.0 && !event.focused;
        if gain.auto_muted != want {
            gain.auto_muted = want;
        }
    }
}

/// Puts the effective gain on `GlobalVolume`, which Bevy applies to every
/// voice that starts later, and on the sink of the voice sounding now, at
/// `SPEAKER_VOLUME * gain`. A value already in place is not written, so
/// with the defaults neither changes.
fn sync_gain(
    gain: Res<OutputGain>,
    global: Option<ResMut<GlobalVolume>>,
    mut live: Query<&mut AudioSink, With<SpeakerVoice>>,
) {
    let g = gain.effective();
    if let Some(mut global) = global
        && global.volume.to_linear() != g
    {
        global.volume = Volume::Linear(g);
    }
    let voice = SPEAKER_VOLUME * g;
    for mut sink in &mut live {
        if sink.volume().to_linear() != voice {
            sink.set_volume(Volume::Linear(voice));
        }
    }
}

#[cfg(test)]
#[path = "audio_tests.rs"]
mod tests;

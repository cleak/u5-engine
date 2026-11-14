//! Output gain and focus mute (design 3.3.7, 3.4.2; build plan M4-08):
//! the 1.0.1 output with the defaults, the gain on live and later voices,
//! the two mutes kept apart, nothing of the game or the speaker's timing
//! touched, and the census of both plugins.

use std::collections::BTreeSet;
use std::time::Duration;

use bevy::audio::{AudioSink, AudioSinkPlayback, GlobalVolume, Volume};
use bevy::ecs::schedule::Schedules;
use bevy::window::WindowFocused;
use u5_runtime::audio::SoundEffect;
use u5_runtime::enhancements::{
    CliSetting, HostCapabilities, LockReason, LockState, MENU_P1, ResolveInput,
    ResolvedEnhancements, SettingValue, TOGGLES, Toggle, resolve_as_shipped,
};
use u5_runtime::test_fixtures::{open_grid, test_state};

use super::*;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};
use crate::enh::{PendingToggleChanges, add_enhancements};
use crate::{SpeakerEffect, SpeakerVoiceLifetime};

/// A launch resolved from `cli` as a host would, with chapter 3's Phase 1
/// rows shipped (they ship in M4-13).
fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    VisualLaunchConfig {
        enhancements: resolve_as_shipped(
            ResolveInput {
                stored: None,
                cli,
                lock: LockState::Off,
                caps: HostCapabilities::ALL,
            },
            &[MENU_P1],
        ),
        ..VisualLaunchConfig::classic()
    }
}

fn enable(key: &str) -> CliSetting {
    CliSetting::Enable(key.to_string())
}

fn set(key: &str, value: &str) -> CliSetting {
    CliSetting::Set {
        key: key.to_string(),
        value: value.to_string(),
    }
}

/// The overlay alone, every audio value at its default.
fn overlay() -> VisualLaunchConfig {
    launch(&[enable("menu.overlay")])
}

// ---------------------------------------------------------------------------
// The plugins on their own, over a voice with a real (idle) sink.

/// A bare app with both plugins, the settings of `resolved` live, Bevy's
/// `GlobalVolume` at its default and one sounding voice at
/// `SPEAKER_VOLUME`, as `play_speaker_effects` starts it.
struct Rig {
    app: App,
    voice: Entity,
}

impl Rig {
    fn new(resolved: &ResolvedEnhancements) -> Self {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<GlobalVolume>()
            .insert_resource(LiveSettings::at_launch(resolved));
        let settings = resolved.audio();
        app.add_plugins(VolumePlugin::new(settings))
            .add_plugins(FocusMutePlugin::new(settings.mute_on_focus_loss));
        let (sink, _queue) = rodio::Sink::new_idle();
        let mut sink = AudioSink::new(sink);
        sink.set_volume(Volume::Linear(crate::SPEAKER_VOLUME));
        let voice = app.world_mut().spawn((SpeakerVoice, sink)).id();
        Self { app, voice }
    }

    fn frame(&mut self) -> &mut Self {
        self.app.update();
        self
    }

    fn global(&self) -> f32 {
        self.app
            .world()
            .resource::<GlobalVolume>()
            .volume
            .to_linear()
    }

    fn voice(&self) -> f32 {
        self.app
            .world()
            .get::<AudioSink>(self.voice)
            .unwrap()
            .volume()
            .to_linear()
    }

    fn gain(&self) -> OutputGain {
        *self.app.world().resource::<OutputGain>()
    }

    /// A change on the Audio tab, as `apply_pending_changes` leaves it.
    fn change(&mut self, edit: impl FnOnce(&mut ResolvedEnhancements)) -> &mut Self {
        let mut live = self.app.world_mut().resource_mut::<LiveSettings>();
        edit(&mut live.now);
        self.frame()
    }

    fn volume(&mut self, percent: i64) -> &mut Self {
        self.change(|now| set_volume(now, percent))
    }

    fn user_mute(&mut self, on: bool) -> &mut Self {
        self.change(|now| now.toggles.set(Toggle::AudioMuted, on))
    }

    fn focus_mute(&mut self, on: bool) -> &mut Self {
        self.change(|now| now.toggles.set(Toggle::AudioMuteOnFocusLoss, on))
    }

    fn focus(&mut self, focused: bool) -> &mut Self {
        let window = self.voice;
        self.app
            .world_mut()
            .send_event(WindowFocused { window, focused });
        self.frame()
    }

    fn global_changed_at(&self) -> bevy::ecs::component::Tick {
        self.app
            .world()
            .get_resource_change_ticks::<GlobalVolume>()
            .unwrap()
            .changed
    }

    fn voice_changed_at(&self) -> bevy::ecs::component::Tick {
        self.app
            .world()
            .entity(self.voice)
            .get_change_ticks::<AudioSink>()
            .unwrap()
            .changed
    }
}

/// Puts a volume on `now` as `apply_value` does once the row ships
/// (M4-13; until then the shipped catalogue refuses it in play).
fn set_volume(now: &mut ResolvedEnhancements, percent: i64) {
    assert!(now.values.set("audio.volume", SettingValue::Int(percent)));
    now.toggles.set(Toggle::AudioVolume, percent != 100);
}

fn assert_close(actual: f32, wanted: f32, what: &str) {
    assert!(
        (actual - wanted).abs() < 1e-6,
        "{what}: {actual} != {wanted}"
    );
}

#[test]
fn the_defaults_keep_global_volume_at_one_and_voices_at_nine_tenths() {
    let mut rig = Rig::new(&overlay().enhancements);
    let (global, voice) = (rig.global_changed_at(), rig.voice_changed_at());
    rig.frame().frame().focus(true).frame();
    assert_eq!(rig.global(), 1.0);
    assert_eq!(rig.voice(), crate::SPEAKER_VOLUME);
    assert_eq!(crate::SPEAKER_VOLUME, 0.9);
    assert_eq!(rig.gain().effective(), 1.0);
    // Not even written with the same value.
    assert_eq!(rig.global_changed_at(), global);
    assert_eq!(rig.voice_changed_at(), voice);
}

#[test]
fn the_gain_reaches_the_sounding_voice_and_later_ones_at_once() {
    let mut rig = Rig::new(&overlay().enhancements);
    rig.volume(50);
    assert_close(rig.global(), 0.5, "later voices");
    assert_close(rig.voice(), 0.45, "the voice sounding now");
    rig.user_mute(true);
    assert_eq!((rig.global(), rig.voice()), (0.0, 0.0));
    rig.user_mute(false);
    assert_close(rig.voice(), 0.45, "unmuted at the chosen volume");
    rig.volume(100);
    assert_eq!(rig.global(), 1.0);
    assert_close(rig.voice(), 0.9, "back to 1.0.1");
}

#[test]
fn a_launch_volume_applies_on_the_first_frame() {
    let quieter = launch(&[set("audio.volume", "70")]);
    let mut rig = Rig::new(&quieter.enhancements);
    rig.frame();
    assert_close(rig.global(), 0.7, "global");
    assert_close(rig.voice(), 0.63, "voice");
    let muted = launch(&[enable("audio.muted")]);
    let mut rig = Rig::new(&muted.enhancements);
    rig.frame();
    assert_eq!((rig.global(), rig.voice()), (0.0, 0.0));
}

#[test]
fn a_user_mute_survives_refocus() {
    let mut rig = Rig::new(&launch(&[enable("audio.mute_on_focus_loss")]).enhancements);
    rig.user_mute(true);
    rig.focus(false);
    assert!(rig.gain().auto_muted);
    rig.focus(true);
    let gain = rig.gain();
    assert!(!gain.auto_muted);
    assert!(gain.user_muted, "focus never touches the player's mute");
    assert_eq!(rig.voice(), 0.0);
}

#[test]
fn only_a_focus_change_releases_an_automatic_mute() {
    let mut rig = Rig::new(&launch(&[enable("audio.mute_on_focus_loss")]).enhancements);
    rig.focus(false);
    assert!(rig.gain().auto_muted);
    assert_eq!(rig.voice(), 0.0);
    // Neither the volume nor the player's mute releases it.
    rig.volume(60).user_mute(true).user_mute(false).frame();
    assert!(rig.gain().auto_muted);
    assert_eq!((rig.global(), rig.voice()), (0.0, 0.0));
    rig.focus(true);
    assert!(!rig.gain().auto_muted);
    assert_close(rig.global(), 0.6, "released at the chosen volume");
}

#[test]
fn turning_focus_mute_off_releases_it_at_once() {
    let mut rig = Rig::new(&launch(&[enable("audio.mute_on_focus_loss")]).enhancements);
    rig.focus(false);
    assert!(rig.gain().auto_muted);
    rig.focus_mute(false);
    assert!(!rig.gain().auto_muted, "the same frame");
    assert_eq!(rig.global(), 1.0);
    rig.focus(true).focus(false);
    assert!(!rig.gain().auto_muted, "off: focus no longer mutes");
    // On again in play: the next focus loss mutes.
    rig.focus_mute(true).focus(false);
    assert!(rig.gain().auto_muted);
}

#[test]
fn with_only_the_overlay_focus_never_mutes() {
    let mut rig = Rig::new(&overlay().enhancements);
    assert!(!rig.app.world().resource::<FocusMuteEnabled>().0);
    rig.focus(false).focus(false);
    assert!(!rig.gain().auto_muted);
    assert_eq!(rig.global(), 1.0);
}

// ---------------------------------------------------------------------------
// In the real shell: the game and the speaker's timing are untouched.

fn town() -> u5_runtime::PlayState {
    test_state(open_grid(), 5, 5)
}

/// Every audio value away from its default, focus mute on and the overlay.
fn everything_quiet() -> VisualLaunchConfig {
    launch(&[
        enable("menu.overlay"),
        set("audio.volume", "30"),
        enable("audio.muted"),
        enable("audio.mute_on_focus_loss"),
    ])
}

/// The voice of one blocked step: its lifetime and the frames it owns the
/// speaker for, in a shell launched with `launch`.
fn voice_run(launch: &VisualLaunchConfig, focused: bool) -> (Duration, usize, bool) {
    let mut shell = Shell::new(town(), &Scene::synthetic(), launch, |_| {});
    shell.step(&[]);
    if !focused {
        let window = Entity::PLACEHOLDER;
        shell.app.world_mut().send_event(WindowFocused {
            window,
            focused: false,
        });
    }
    shell
        .app
        .world_mut()
        .send_event(SpeakerEffect::always_audible(SoundEffect::BlockedStep));
    shell.step(&[]);
    let world = shell.app.world_mut();
    let lifetime = world
        .query::<&SpeakerVoiceLifetime>()
        .single(world)
        .unwrap()
        .0
        .duration();
    let mut frames = 1;
    while shell
        .app
        .world_mut()
        .query::<&SpeakerVoice>()
        .iter(shell.app.world())
        .next()
        .is_some()
    {
        shell.step(&[]);
        frames += 1;
        assert!(frames < 600, "the voice never ended");
    }
    let music = shell.visual().state.music_enabled;
    (lifetime, frames, music)
}

#[test]
fn a_muted_voice_keeps_its_full_lifetime_and_the_sound_flag() {
    let classic = voice_run(&VisualLaunchConfig::classic(), true);
    assert!(classic.0 > Duration::ZERO);
    assert!(classic.2, "a new game has its sound on");
    for focused in [true, false] {
        assert_eq!(
            voice_run(&everything_quiet(), focused),
            classic,
            "focused {focused}"
        );
    }
    let shell = Shell::new(town(), &Scene::synthetic(), &everything_quiet(), |_| {});
    assert!(shell.app.world().contains_resource::<OutputGain>());
}

/// Sends a focus change on the given frames, as the window would.
fn focus_changes(app: &mut App) {
    app.add_systems(
        Update,
        (|mut frame: Local<usize>, mut out: EventWriter<WindowFocused>| {
            *frame += 1;
            if let Some(focused) = match *frame {
                30 => Some(false),
                70 => Some(true),
                90 => Some(false),
                _ => None,
            } {
                out.write(WindowFocused {
                    window: Entity::PLACEHOLDER,
                    focused,
                });
            }
        })
        .before(focus_mute),
    );
}

/// G3 for the gain: with every audio value changed, focus lost and
/// regained, and a change made in play, the game equals the Classic game
/// frame by frame.
#[test]
fn the_gain_and_focus_mute_leave_the_game_as_classic() {
    let mut timeline = Timeline::new(140);
    for (frame, key) in [
        (10, KeyCode::ArrowUp),
        (35, KeyCode::ArrowRight),
        (60, KeyCode::ArrowDown),
        (85, KeyCode::ArrowLeft),
        (110, KeyCode::ArrowUp),
    ] {
        timeline.game_tap(frame, key);
    }
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &timeline,
        &everything_quiet(),
        |app| {
            focus_changes(app);
            app.add_systems(
                Update,
                (|mut frame: Local<usize>,
                  mut toggles: ResMut<PendingToggleChanges>,
                  mut live: ResMut<LiveSettings>| {
                    *frame += 1;
                    if *frame == 50 {
                        toggles.0.push((Toggle::AudioMuted, false));
                    }
                    if *frame == 60 {
                        set_volume(&mut live.now, 80);
                    }
                })
                .before(crate::enh::apply_pending_changes),
            );
        },
    );
    assert_eq!(mismatches, []);
}

// ---------------------------------------------------------------------------
// The census (3.4.2).

fn system_names(app: &App) -> BTreeSet<String> {
    app.world()
        .resource::<Schedules>()
        .iter()
        .flat_map(|(label, schedule)| {
            schedule
                .graph()
                .systems()
                .map(move |(_, system, _)| format!("{label:?} {}", system.name()))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn audio_systems(launch: &VisualLaunchConfig) -> (App, BTreeSet<String>) {
    let mut app = App::new();
    add_enhancements(&mut app, launch);
    let systems = system_names(&app)
        .into_iter()
        .filter(|name| name.contains("::menu::audio::"))
        .map(|name| name.rsplit("::").next().unwrap().to_string())
        .collect();
    (app, systems)
}

const VOLUME: [&str; 2] = ["follow_audio_settings", "sync_gain"];
const FOCUS: [&str; 2] = ["focus_mute", "follow_focus_mute_setting"];

fn assert_plugins(launch: &VisualLaunchConfig, volume: bool, focus: bool, what: &str) {
    let (app, systems) = audio_systems(launch);
    let mut wanted: BTreeSet<String> = BTreeSet::new();
    if volume {
        wanted.extend(VOLUME.iter().map(|s| s.to_string()));
    }
    if focus {
        wanted.extend(FOCUS.iter().map(|s| s.to_string()));
    }
    assert_eq!(systems, wanted, "{what}");
    let world = app.world();
    assert_eq!(world.contains_resource::<OutputGain>(), volume, "{what}");
    assert_eq!(app.is_plugin_added::<VolumePlugin>(), volume, "{what}");
    assert_eq!(
        world.contains_resource::<FocusMuteEnabled>(),
        focus,
        "{what}"
    );
    assert_eq!(app.is_plugin_added::<FocusMutePlugin>(), focus, "{what}");
    // Never Bevy's own output resource: the shell's audio plugin owns it.
    assert!(!world.contains_resource::<GlobalVolume>(), "{what}");
}

#[test]
fn the_census_rows_for_the_gain_and_focus_mute() {
    // Off: Classic, the lock (with every audio value changed) and every
    // toggle but the four that bring them.
    assert_plugins(&VisualLaunchConfig::classic(), false, false, "Classic");
    let mut locked = everything_quiet();
    locked.enhancements.lock = ResolvedEnhancements::locked(LockReason::QaEnv).lock;
    assert_plugins(&locked, false, false, "the lock");
    let mut rest = VisualLaunchConfig::classic();
    let brings = [
        Toggle::MenuOverlay,
        Toggle::AudioVolume,
        Toggle::AudioMuted,
        Toggle::AudioMuteOnFocusLoss,
    ];
    for spec in TOGGLES {
        if !brings.contains(&spec.toggle) {
            rest.enhancements.toggles.set(spec.toggle, true);
        }
    }
    assert_plugins(&rest, false, false, "every other toggle");

    // The 3.4.2 rows, each condition alone.
    assert_plugins(&overlay(), true, true, "the overlay");
    assert_plugins(
        &launch(&[set("audio.volume", "50")]),
        true,
        false,
        "a volume",
    );
    assert_plugins(&launch(&[enable("audio.muted")]), true, false, "the mute");
    assert_plugins(
        &launch(&[enable("audio.mute_on_focus_loss")]),
        true,
        true,
        "focus mute brings the gain",
    );
    assert_plugins(
        &launch(&[enable("menu.title_layer")]),
        false,
        false,
        "the title layer alone",
    );
    // The volume at its default is no condition.
    assert_plugins(
        &launch(&[set("audio.volume", "100"), enable("display.fullscreen")]),
        false,
        false,
        "volume 100",
    );
}

#[test]
fn the_conditions_match_the_apply_points() {
    for spec in TOGGLES {
        let mut toggles = u5_runtime::enhancements::FeatureToggles::CLASSIC;
        toggles.set(spec.toggle, true);
        assert_eq!(
            volume_wanted(toggles),
            [
                Toggle::MenuOverlay,
                Toggle::AudioVolume,
                Toggle::AudioMuted,
                Toggle::AudioMuteOnFocusLoss
            ]
            .contains(&spec.toggle),
            "{:?}",
            spec.toggle
        );
        assert_eq!(
            focus_mute_wanted(toggles),
            [Toggle::MenuOverlay, Toggle::AudioMuteOnFocusLoss].contains(&spec.toggle),
            "{:?}",
            spec.toggle
        );
    }
}

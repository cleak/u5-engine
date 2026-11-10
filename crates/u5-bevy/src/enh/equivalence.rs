//! The trajectory-equivalence harness, Bevy layer (Phase B design 1.3.12,
//! layer B; build plan M0-19).
//!
//! The terminal layer (`u5_tui::equivalence`) proves the runtime features;
//! this layer proves the shell. It runs the real `--visual` app, built by
//! the same builder the player's binary uses, headless:
//!
//! - `MinimalPlugins` with [`TimeUpdateStrategy::ManualDuration`] of one
//!   sixtieth of a second, the asset plugin and the input plugin, so every
//!   frame is the same length and keys reach the game through Bevy's own
//!   keyboard path. The speaker has no playback system
//!   ([`SpeakerOutput::Silent`](crate::SpeakerOutput)): each voice ends on
//!   its timer, on the manual clock, never on a sound card's real time;
//! - a [`Timeline`] of `KeyboardInput` events, one list per frame, each
//!   event owned by the game or by the feature under test.
//!
//! Both runs start from one cloned `PlayState`, so the seed does not matter
//! and no environment variable is read.
//!
//! - The **on** run is built with the feature's launch config (and any
//!   test-only install) and runs the whole timeline.
//! - The **off** run is the Classic app, the 1.0.1 schedule, and runs the
//!   timeline with the feature's events removed: a Live overlay interval
//!   becomes empty frames, as if the player had stopped typing. Keys a
//!   class (b) feature fed the game through the synthetic-key owner are
//!   replayed into the off run's key queue in the same frame, at the same
//!   point of the frame, as the feed puts them there.
//!
//! The two are compared on every frame: the [`TrajectoryDigest`] of the
//! game state, the original 320x200 frame and the game's key queue.

use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::{ButtonState, InputPlugin};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use u5_runtime::enhancements::{TrajectoryDigest, fnv1a64, trajectory_digest};
use u5_runtime::test_fixtures::synthetic_tile_atlas;
use u5_runtime::{
    CH_FONT_LEN, FixedCellFont, IBM_CH_FILE, PlayState, TileAtlas, TileGraphicsDepth,
    load_ibm_ch_font, load_runes_ch_font, load_tile_atlas, parse_ch_font,
};

use super::input::is_modifier;
use super::synthetic::SyntheticInputOwner;
use super::{EnhSet, VisualLaunchConfig};
use crate::{
    Bootstrap, ScreenshotConfig, SpeakerOutput, TypedKey, VisualState,
    build_visual_app_with_speaker, drive_visual,
};

/// One host frame: the manual clock's step.
pub(crate) const FRAME: Duration = Duration::from_nanos(1_000_000_000 / 60);

/// The files and fonts a run draws with, shared by both runs.
#[derive(Clone)]
pub(crate) struct Scene {
    pub(crate) game_dir: PathBuf,
    pub(crate) atlas: TileAtlas,
    pub(crate) text_font: FixedCellFont,
    pub(crate) rune_font: FixedCellFont,
}

impl Scene {
    /// No game files: the synthetic atlas and a blank font.
    pub(crate) fn synthetic() -> Self {
        let font = parse_ch_font(&vec![0x00; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        Self {
            game_dir: PathBuf::from("."),
            atlas: synthetic_tile_atlas(TileGraphicsDepth::Ega16),
            text_font: font.clone(),
            rune_font: font,
        }
    }

    /// The real atlas and fonts of the game in `dir`.
    pub(crate) fn from_game_dir(dir: &Path) -> io::Result<Self> {
        Ok(Self {
            game_dir: dir.to_path_buf(),
            atlas: load_tile_atlas(dir, TileGraphicsDepth::Ega16)?,
            text_font: load_ibm_ch_font(dir)?,
            rune_font: load_runes_ch_font(dir)?,
        })
    }
}

/// Whose event it is: the player's, which both runs see, or the feature's,
/// which the off run drops.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Owner {
    Game,
    Feature,
}

/// One keyboard event of a timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TimedKey {
    pub(crate) key: KeyCode,
    pub(crate) state: ButtonState,
    pub(crate) repeat: bool,
    pub(crate) owner: Owner,
}

/// The keyboard events of a run, one list per frame, in delivery order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Timeline {
    frames: Vec<Vec<TimedKey>>,
}

impl Timeline {
    /// `frames` frames with no event.
    pub(crate) fn new(frames: usize) -> Self {
        Self {
            frames: vec![Vec::new(); frames],
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.frames.len()
    }

    pub(crate) fn frame(&self, index: usize) -> &[TimedKey] {
        &self.frames[index]
    }

    /// Adds one event at `frame`, growing the timeline to reach it.
    pub(crate) fn event(
        &mut self,
        frame: usize,
        key: KeyCode,
        state: ButtonState,
        repeat: bool,
        owner: Owner,
    ) -> &mut Self {
        if self.frames.len() <= frame {
            self.frames.resize(frame + 1, Vec::new());
        }
        self.frames[frame].push(TimedKey {
            key,
            state,
            repeat,
            owner,
        });
        self
    }

    /// `key` pressed at `frame` and released at the next.
    pub(crate) fn tap(&mut self, frame: usize, key: KeyCode, owner: Owner) -> &mut Self {
        self.event(frame, key, ButtonState::Pressed, false, owner)
            .event(frame + 1, key, ButtonState::Released, false, owner)
    }

    /// The player taps `key` at `frame`.
    pub(crate) fn game_tap(&mut self, frame: usize, key: KeyCode) -> &mut Self {
        self.tap(frame, key, Owner::Game)
    }

    /// The feature's own tap, such as an overlay's hotkey.
    pub(crate) fn feature_tap(&mut self, frame: usize, key: KeyCode) -> &mut Self {
        self.tap(frame, key, Owner::Feature)
    }

    /// The feature's key held from `from` to `to`, with OS repeat every
    /// third frame in between.
    pub(crate) fn feature_hold(&mut self, from: usize, to: usize, key: KeyCode) -> &mut Self {
        self.event(from, key, ButtonState::Pressed, false, Owner::Feature);
        for frame in (from + 3..to).step_by(3) {
            self.event(frame, key, ButtonState::Pressed, true, Owner::Feature);
        }
        self.event(to, key, ButtonState::Released, false, Owner::Feature)
    }

    /// The off run's timeline: every frame kept, the feature's events gone.
    pub(crate) fn without_feature(&self) -> Timeline {
        Timeline {
            frames: self
                .frames
                .iter()
                .map(|events| {
                    events
                        .iter()
                        .copied()
                        .filter(|event| event.owner == Owner::Game)
                        .collect()
                })
                .collect(),
        }
    }
}

/// What one frame of a run is compared on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrameRecord {
    pub(crate) digest: TrajectoryDigest,
    /// FNV-1a-64 of the original frame's pixels.
    pub(crate) frame: u64,
    /// FNV-1a-64 of the game's key queue: keys typed and not yet read. A
    /// lone modifier (Ctrl, Shift, Alt) is left out: it maps to no byte in
    /// either input mode and every read drops it, so it is no key the game
    /// sees (the router's own rule). The Ctrl of a display chord such as
    /// Ctrl+P is queued this way while the game is busy.
    pub(crate) queue: u64,
}

/// The keys the synthetic-key owner fed the game, per frame, in order.
#[derive(Resource, Default)]
struct FedKeys {
    frames: Vec<Vec<TypedKey>>,
    queue_before_feed: usize,
}

/// Before the feed: how long the game's key queue is.
fn mark_queue_before_feed(visual: Option<Res<VisualState>>, mut fed: ResMut<FedKeys>) {
    fed.queue_before_feed = visual.map_or(0, |visual| visual.typed_keys.len());
}

/// After the feed: what it added to the queue this frame.
fn record_fed_keys(visual: Option<Res<VisualState>>, mut fed: ResMut<FedKeys>) {
    let added = visual.map_or_else(Vec::new, |visual| {
        visual
            .typed_keys
            .iter()
            .skip(fed.queue_before_feed)
            .copied()
            .collect()
    });
    fed.frames.push(added);
}

/// The off run's stand-in for the feed: the on run's fed keys, by frame.
#[derive(Resource, Default)]
struct ReplayKeys(VecDeque<Vec<TypedKey>>);

/// Queues this frame's replayed keys where the feed queued them: before
/// the input driver reads, ahead of the keys typed this frame.
fn replay_fed_keys(mut replay: ResMut<ReplayKeys>, visual: Option<ResMut<VisualState>>) {
    let Some(mut visual) = visual else {
        return;
    };
    if let Some(keys) = replay.0.pop_front() {
        visual.typed_keys.extend(keys);
    }
}

/// The `--visual` app, headless, one frame at a time.
pub(crate) struct Shell {
    pub(crate) app: App,
}

impl Shell {
    /// The app the builder makes for `launch`, starting from `state`, then
    /// `install` for test-only systems a feature would add.
    pub(crate) fn new(
        state: PlayState,
        scene: &Scene,
        launch: &VisualLaunchConfig,
        install: impl FnOnce(&mut App),
    ) -> Self {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default(), InputPlugin))
            .init_asset::<Image>()
            .insert_resource(TimeUpdateStrategy::ManualDuration(FRAME));
        build_visual_app_with_speaker(
            &mut app,
            Bootstrap {
                game_dir: scene.game_dir.clone(),
                state,
                atlas: scene.atlas.clone(),
                text_font: scene.text_font.clone(),
                rune_font: scene.rune_font.clone(),
            },
            ScreenshotConfig {
                path: None,
                frame_delay: 0,
                preset_keys: Vec::new(),
            },
            launch,
            SpeakerOutput::Silent,
        );
        install(&mut app);
        Self { app }
    }

    /// Records the keys a synthetic-key feature feeds, if one is installed.
    fn record_feed(&mut self) {
        if !self.app.world().contains_resource::<SyntheticInputOwner>() {
            return;
        }
        self.app.init_resource::<FedKeys>().add_systems(
            Update,
            (
                mark_queue_before_feed
                    .after(EnhSet::Apply)
                    .before(EnhSet::Feed),
                record_fed_keys.after(EnhSet::Feed).before(drive_visual),
            ),
        );
    }

    /// Replays `fed` into the game's key queue, one list per frame.
    fn replay_feed(&mut self, fed: &[Vec<TypedKey>]) {
        if fed.iter().all(Vec::is_empty) {
            return;
        }
        self.app
            .insert_resource(ReplayKeys(fed.iter().cloned().collect()))
            .add_systems(Update, replay_fed_keys.before(drive_visual));
    }

    /// Delivers `events` as the window would, then runs one frame.
    pub(crate) fn step(&mut self, events: &[TimedKey]) -> FrameRecord {
        for event in events {
            self.app.world_mut().send_event(KeyboardInput {
                key_code: event.key,
                logical_key: Key::Unidentified(NativeKey::Unidentified),
                state: event.state,
                text: None,
                repeat: event.repeat,
                window: Entity::PLACEHOLDER,
            });
        }
        self.app.update();
        self.record()
    }

    pub(crate) fn visual(&self) -> &VisualState {
        self.app.world().resource::<VisualState>()
    }

    /// This frame's record.
    pub(crate) fn record(&self) -> FrameRecord {
        let visual = self.visual();
        let frame = self
            .app
            .world()
            .resource::<Assets<Image>>()
            .get(&visual.image_handle)
            .and_then(|image| image.data.as_deref())
            .map_or(0, fnv1a64);
        let queue: Vec<u8> = visual
            .typed_keys
            .iter()
            .filter(|typed| !is_modifier(typed.key))
            .flat_map(|typed| {
                format!("{:?}{}{};", typed.key, typed.shift, typed.control).into_bytes()
            })
            .collect();
        FrameRecord {
            digest: trajectory_digest(&visual.state),
            frame,
            queue: fnv1a64(&queue),
        }
    }
}

/// One run: every frame's record, and the keys a feature fed per frame.
#[derive(Clone, Debug, Default)]
pub(crate) struct ShellRun {
    pub(crate) frames: Vec<FrameRecord>,
    pub(crate) fed: Vec<Vec<TypedKey>>,
}

/// Runs `timeline` on a fresh app; `replay` feeds the on run's fed keys.
fn run_shell(
    state: PlayState,
    scene: &Scene,
    launch: &VisualLaunchConfig,
    install: impl FnOnce(&mut App),
    timeline: &Timeline,
    replay: Option<&[Vec<TypedKey>]>,
) -> ShellRun {
    let mut shell = Shell::new(state, scene, launch, install);
    match replay {
        Some(fed) => shell.replay_feed(fed),
        None => shell.record_feed(),
    }
    let frames = (0..timeline.len())
        .map(|index| shell.step(timeline.frame(index)))
        .collect();
    let fed = shell
        .app
        .world_mut()
        .remove_resource::<FedKeys>()
        .map_or_else(Vec::new, |fed| fed.frames);
    ShellRun { frames, fed }
}

/// The first frame at which two runs part, for one compared part.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ShellMismatch {
    pub(crate) frame: usize,
    pub(crate) what: &'static str,
    pub(crate) off: String,
    pub(crate) on: String,
}

impl std::fmt::Display for ShellMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "frame {} {}: off {} / on {}",
            self.frame, self.what, self.off, self.on
        )
    }
}

/// Compares an on run with its off run, frame by frame. Empty means
/// equivalent. Each compared part reports the first frame it differs at.
pub(crate) fn compare_shell_runs(off: &ShellRun, on: &ShellRun) -> Vec<ShellMismatch> {
    let mut mismatches = Vec::new();
    if off.frames.len() != on.frames.len() {
        mismatches.push(ShellMismatch {
            frame: off.frames.len().min(on.frames.len()),
            what: "frames run",
            off: off.frames.len().to_string(),
            on: on.frames.len().to_string(),
        });
    }
    type Part = fn(&FrameRecord) -> String;
    let parts: [(&'static str, Part); 3] = [
        ("digest", |r| r.digest.to_string()),
        ("original frame", |r| format!("{:016x}", r.frame)),
        ("key queue", |r| format!("{:016x}", r.queue)),
    ];
    for (what, part) in parts {
        if let Some((frame, (a, b))) = off
            .frames
            .iter()
            .zip(&on.frames)
            .enumerate()
            .find(|(_, (a, b))| part(a) != part(b))
        {
            mismatches.push(ShellMismatch {
                frame,
                what,
                off: part(a),
                on: part(b),
            });
        }
    }
    mismatches
}

/// Runs `timeline` from `start` with the feature on (`on_launch`, then
/// `install_on`), and the Classic app with the feature's events removed and
/// its fed keys replayed, and compares the two (design 1.3.12, layer B).
pub(crate) fn check_shell_equivalence(
    start: &PlayState,
    scene: &Scene,
    timeline: &Timeline,
    on_launch: &VisualLaunchConfig,
    install_on: impl FnOnce(&mut App),
) -> Vec<ShellMismatch> {
    let on = run_shell(start.clone(), scene, on_launch, install_on, timeline, None);
    let off = run_shell(
        start.clone(),
        scene,
        &VisualLaunchConfig::classic(),
        |_| {},
        &timeline.without_feature(),
        Some(&on.fed),
    );
    compare_shell_runs(&off, &on)
}

mod tests;

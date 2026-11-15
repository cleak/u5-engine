//! Phase 3b, second half (design 6.2.4, 6.3.4, 6.3.7, 6.6.3; plan M7-06):
//! the feed, the interrupt and the observer in a headless engaged app, and
//! the replay of the fed keys with the feature off.

use std::fs;
use std::path::Path;

use bevy::input::ButtonState;
use u5_runtime::SAVED_GAM_FILENAME;
use u5_runtime::enhancements::{TrajectoryDigest, trajectory_digest};
use u5_runtime::test_fixtures::{debug_game_dir, saved_game_seed_bytes, write_empty_ool_mirrors};
use u5_runtime::{SAVED_OOL_FILENAME, SAVED_OOL_LEN};

use super::feed_tests::arena;
use super::shell_tests::{Chord, Game, launch};
use super::*;
use crate::drive_visual;
use crate::enh::equivalence::{Owner, Scene, Shell, Timeline};
use crate::enh::overlay::OverlayId;

/// The keys the feed put in the game's queue each frame, recorded between
/// `EnhSet::Feed` and the input driver (the human's keys join later, in
/// the driver).
#[derive(Resource, Default)]
struct Fed {
    before: usize,
    frames: Vec<Vec<TypedKey>>,
}

fn mark_before_feed(visual: Option<Res<VisualState>>, mut fed: ResMut<Fed>) {
    fed.before = visual.map_or(0, |visual| visual.typed_keys.len());
}

fn record_after_feed(visual: Option<Res<VisualState>>, mut fed: ResMut<Fed>) {
    let before = fed.before;
    let added = visual.map_or_else(Vec::new, |visual| {
        visual.typed_keys.iter().skip(before).copied().collect()
    });
    fed.frames.push(added);
}

fn record_feed(app: &mut App) {
    app.init_resource::<Fed>().add_systems(
        Update,
        (
            mark_before_feed.after(EnhSet::Apply).before(EnhSet::Feed),
            record_after_feed.after(EnhSet::Feed).before(drive_visual),
        ),
    );
}

/// The arena with a one-hit skeleton, so a fight ends in a few turns.
fn weak_arena() -> PlayState {
    let mut state = arena();
    state.combat_actors[16].hp_or_wound = 1;
    state
}

/// The settings of a fast engaged test: no pause between keys, no HP stop.
fn fast(settings: &mut CombatSettings) {
    settings.step_ms = 0;
    settings.pause_below_hp_percent = 0;
}

/// An engaged-test game from `state` in `scene`: auto on, `settings`
/// applied, the feed recorded.
fn game_in(state: PlayState, scene: &Scene, settings: impl FnOnce(&mut CombatSettings)) -> Game {
    let mut shell = Shell::new(state, scene, &launch(true), |_| {});
    settings(
        &mut shell
            .app
            .world_mut()
            .resource_mut::<AutoCombatShell>()
            .settings,
    );
    record_feed(&mut shell.app);
    Game {
        shell,
        timeline: Timeline::new(0),
        frame: 0,
    }
}

fn new_game(settings: impl FnOnce(&mut CombatSettings)) -> Game {
    game_in(arena(), &Scene::synthetic(), settings)
}

impl Game {
    fn fed(&self) -> &[Vec<TypedKey>] {
        &self.world().resource::<Fed>().frames
    }

    fn fed_now(&self) -> &[TypedKey] {
        self.fed().last().map_or(&[], Vec::as_slice)
    }

    fn prompt(&self) -> AutoPrompt {
        auto_prompt(&self.shell.visual().state)
    }

    /// Engages with `chord` and runs only the chord's press frame.
    fn engage(&mut self, chord: Chord) {
        chord.tap(&mut self.timeline, self.frame, Owner::Feature);
        self.frames(1);
    }

    /// The player taps `keys` at the next frame, in that order, then
    /// releases them a frame later.
    fn type_keys(&mut self, keys: &[KeyCode]) {
        for key in keys {
            self.timeline
                .event(self.frame, *key, ButtonState::Pressed, false, Owner::Game);
        }
        for key in keys {
            self.timeline.event(
                self.frame + 1,
                *key,
                ButtonState::Released,
                false,
                Owner::Game,
            );
        }
    }

    /// Runs until auto is no longer engaged nor handing back, checking
    /// each frame, at most `limit` frames.
    fn run_until_handed_back(&mut self, limit: usize) {
        for _ in 0..limit {
            self.frames(1);
            assert!(self.fed_now().len() <= 1, "frame {}", self.frame);
            let auto = self.auto().unwrap();
            if !auto.engaged.any() && auto.handing_back.is_none() {
                return;
            }
        }
        panic!("auto still engaged after {limit} frames");
    }

    fn digest(&self) -> TrajectoryDigest {
        trajectory_digest(&self.shell.visual().state)
    }
}

fn plain(key: KeyCode) -> TypedKey {
    TypedKey {
        key,
        shift: false,
        control: false,
    }
}

// ---------------------------------------------------------------------------
// The feed.

#[test]
fn auto_feeds_one_key_per_frame_at_most_and_wins_the_fight() {
    let mut game = game_in(weak_arena(), &Scene::synthetic(), fast);
    game.frames(2);
    game.engage(Chord::UntilDone);
    assert_eq!(game.fed_now().len(), 1, "fed in the chord's frame");
    game.run_until_handed_back(2000);
    assert!(game.fed().iter().all(|keys| keys.len() <= 1));
    let fed: Vec<TypedKey> = game.fed().iter().flatten().copied().collect();
    assert!(fed.len() >= 3, "{fed:?}");
    assert_eq!(game.toast(), Some(AUTO_OFF_VICTORY));
    assert_eq!(game.engaged(), AutoEngagement::default());
    assert_eq!(game.auto().unwrap().fight, AutoFightRuntime::default());
    assert_eq!(
        game.world().resource::<SyntheticInputOwner>().holder(),
        None,
        "the token goes with the hand-back"
    );
    // The prompt is the player's now: nothing more is fed.
    let count = fed.len();
    game.frames(60);
    assert_eq!(game.fed().iter().flatten().count(), count);
}

#[test]
fn step_ms_spaces_the_keys() {
    // 150 ms is nine frames of the manual clock.
    let mut game = game_in(weak_arena(), &Scene::synthetic(), |settings| {
        settings.pause_below_hp_percent = 0;
    });
    game.frames(2);
    game.engage(Chord::UntilDone);
    game.run_until_handed_back(4000);
    let frames: Vec<usize> = game
        .fed()
        .iter()
        .enumerate()
        .filter(|(_, keys)| !keys.is_empty())
        .map(|(frame, _)| frame)
        .collect();
    assert!(frames.len() >= 3, "{frames:?}");
    for pair in frames.windows(2) {
        assert!(pair[1] - pair[0] >= 9, "{frames:?}");
    }
}

#[test]
fn nothing_is_fed_under_an_overlay_or_while_another_owner_holds_the_token() {
    // Another owner holds the token.
    let mut game = new_game(fast);
    game.frames(2);
    game.shell
        .app
        .world_mut()
        .resource_mut::<SyntheticInputOwner>()
        .take(SyntheticOwner::MenuSlotSave)
        .unwrap();
    game.engage(Chord::Party);
    game.frames(30);
    assert!(game.fed().iter().all(Vec::is_empty));
    assert_eq!(game.engaged(), PARTY_ALL);
    game.shell
        .app
        .world_mut()
        .resource_mut::<SyntheticInputOwner>()
        .release(SyntheticOwner::MenuSlotSave);
    game.frames(1);
    assert_eq!(game.fed_now().len(), 1);

    // An overlay is open.
    let mut game = new_game(fast);
    game.frames(2);
    game.shell
        .app
        .insert_resource(OverlayState::opened(OverlayId::Cheats));
    game.engage(Chord::Party);
    game.frames(30);
    assert!(game.fed().iter().all(Vec::is_empty));
    game.shell.app.insert_resource(OverlayState::default());
    game.frames(1);
    assert_eq!(game.fed_now().len(), 1);
}

#[test]
fn a_paused_or_busy_shell_feeds_nothing() {
    // Focus lost.
    let mut game = new_game(fast);
    game.frames(2);
    game.focus(false);
    game.engage(Chord::Party);
    game.frames(20);
    assert!(game.fed().iter().all(Vec::is_empty));
    game.focus(true);
    assert_eq!(game.fed_now().len(), 1);

    // A key already waits in the game's queue.
    let mut game = new_game(fast);
    game.frames(2);
    game.shell
        .app
        .world_mut()
        .resource_mut::<VisualState>()
        .state
        .sound_effect_serial += 1;
    game.engage(Chord::Party);
    assert!(
        game.fed_now().is_empty(),
        "an effect not yet sent owns the thread"
    );
}

const PARTY_ALL: AutoEngagement = AutoEngagement {
    members: [true; 6],
    until_done: false,
};

// ---------------------------------------------------------------------------
// The interrupt.

/// A game engaged at the command prompt with the feed held by a focus
/// pause, so the player's key meets a prompt auto owns and no open attempt.
fn engaged_at_the_prompt(mode: InterruptMode) -> Game {
    let mut game = new_game(|settings| {
        fast(settings);
        settings.interrupt = mode;
    });
    game.frames(2);
    game.focus(false);
    game.engage(Chord::Party);
    game.frames(3);
    assert!(matches!(game.prompt(), AutoPrompt::Command { .. }));
    assert!(game.fed().iter().all(Vec::is_empty));
    game
}

/// The Classic game typing `key` (or nothing) at `frame`, its digest after
/// `frames` frames.
fn classic_digest(key: Option<(usize, KeyCode)>, frames: usize) -> TrajectoryDigest {
    let mut timeline = Timeline::new(frames);
    if let Some((frame, key)) = key {
        timeline.game_tap(frame, key);
    }
    let mut shell = Shell::new(
        arena(),
        &Scene::synthetic(),
        &VisualLaunchConfig::classic(),
        |_| {},
    );
    for frame in 0..frames {
        shell.step(timeline.frame(frame));
    }
    trajectory_digest(&shell.visual().state)
}

#[test]
fn swallow_stops_auto_and_drops_the_key() {
    let mut game = engaged_at_the_prompt(InterruptMode::Swallow);
    let at = game.frame;
    game.type_keys(&[KeyCode::ArrowUp]);
    game.frames(1);
    game.focus(true);
    game.run_until_handed_back(10);
    assert_eq!(game.toast(), Some(AUTO_OFF_KEY));
    assert!(
        game.fed().iter().all(Vec::is_empty),
        "no auto key, no key back"
    );
    game.frames(30);
    let frames = game.frame;
    assert_eq!(
        game.digest(),
        classic_digest(None, frames),
        "the key never reached the game"
    );
    assert_ne!(
        game.digest(),
        classic_digest(Some((at, KeyCode::ArrowUp)), frames)
    );
}

#[test]
fn deliver_stops_auto_and_answers_the_prompt_with_the_key() {
    let mut game = engaged_at_the_prompt(InterruptMode::Deliver);
    let at = game.frame;
    game.type_keys(&[KeyCode::ArrowUp]);
    game.frames(1);
    game.focus(true);
    game.run_until_handed_back(10);
    assert_eq!(game.toast(), Some(AUTO_OFF_KEY));
    assert!(game.fed().iter().all(Vec::is_empty));
    game.frames(30);
    let frames = game.frame;
    assert_eq!(
        game.digest(),
        classic_digest(Some((at, KeyCode::ArrowUp)), frames),
        "the player's own key, as in 1.0.1"
    );
}

#[test]
fn auto_finishes_its_open_attempt_and_the_held_keys_follow_in_order() {
    for mode in [InterruptMode::Swallow, InterruptMode::Deliver] {
        let mut game = new_game(|settings| {
            fast(settings);
            settings.interrupt = mode;
        });
        game.frames(2);
        game.engage(Chord::Party);
        // Until auto's own `A` opens the cursor.
        for _ in 0..100 {
            if game.fed_now() == [plain(KeyCode::KeyA)] {
                break;
            }
            game.frames(1);
        }
        assert_eq!(game.fed_now(), [plain(KeyCode::KeyA)]);
        // Three keys in one frame while auto's cursor is open.
        game.type_keys(&[KeyCode::KeyX, KeyCode::KeyY, KeyCode::KeyZ]);
        let mut fed = Vec::new();
        let mut engaged_until = None;
        for _ in 0..200 {
            game.frames(1);
            fed.extend(game.fed_now().iter().map(|typed| typed.key));
            let auto = game.auto().unwrap();
            if engaged_until.is_none() && !auto.engaged.any() {
                engaged_until = Some(fed.len());
                assert!(
                    !matches!(game.prompt(), AutoPrompt::Cursor { .. }),
                    "{mode:?}: never handed back inside the cursor"
                );
            }
            if !auto.engaged.any() && auto.handing_back.is_none() {
                break;
            }
        }
        let held = match mode {
            InterruptMode::Swallow => vec![KeyCode::KeyY, KeyCode::KeyZ],
            InterruptMode::Deliver => vec![KeyCode::KeyX, KeyCode::KeyY, KeyCode::KeyZ],
        };
        let mut expected = vec![KeyCode::ArrowRight, KeyCode::Enter];
        expected.extend(&held);
        assert_eq!(fed, expected, "{mode:?}: the attempt, then the held keys");
        assert_eq!(engaged_until, Some(2), "{mode:?}");
        assert_eq!(game.toast(), Some(AUTO_OFF_KEY), "{mode:?}");
        assert_eq!(
            game.world().resource::<SyntheticInputOwner>().holder(),
            None
        );
    }
}

#[test]
fn ctrl_and_shift_never_interrupt() {
    let mut game = new_game(fast);
    game.frames(2);
    game.engage(Chord::UntilDone);
    for modifier in [
        KeyCode::ShiftLeft,
        KeyCode::ControlLeft,
        KeyCode::ShiftRight,
    ] {
        game.type_keys(&[modifier]);
        game.frames(2);
        let auto = game.auto().unwrap();
        assert_eq!(auto.fight.stopping, None, "{modifier:?}");
        assert!(auto.engaged.until_done, "{modifier:?}");
    }
    // Ctrl+Shift+A while auto runs changes the mode; its Ctrl and Shift are
    // no interrupt.
    game.chord(Chord::Party);
    assert_eq!(game.toast(), Some(AUTO_PARTY));
    let auto = game.auto().unwrap();
    assert_eq!(auto.engaged, PARTY_ALL);
    assert_eq!(auto.fight.stopping, None);
    assert_eq!(auto.handing_back, None);
}

#[test]
fn the_fight_over_resets_the_fight_and_the_engagement() {
    let mut game = new_game(fast);
    game.frames(2);
    game.engage(Chord::Party);
    game.frames(2);
    game.shell
        .app
        .world_mut()
        .resource_mut::<VisualState>()
        .state
        .combat_active = false;
    game.run_until_handed_back(10);
    assert_eq!(game.toast(), Some(AUTO_OFF));
    assert_eq!(game.auto().unwrap().fight, AutoFightRuntime::default());

    // `keep_between_fights` keeps member and party auto, never "until done".
    for (chord, kept) in [(Chord::Party, true), (Chord::UntilDone, false)] {
        let mut game = new_game(|settings| {
            fast(settings);
            settings.keep_between_fights = true;
        });
        game.frames(2);
        game.engage(chord);
        game.frames(2);
        game.shell
            .app
            .world_mut()
            .resource_mut::<VisualState>()
            .state
            .combat_active = false;
        game.frames(3);
        let auto = game.auto().unwrap();
        assert_eq!(auto.engaged.any(), kept, "{chord:?}");
        assert_eq!(auto.fight, AutoFightRuntime::default(), "{chord:?}");
    }
}

// ---------------------------------------------------------------------------
// Replay equivalence (6.6.3).

/// A save folder with a seed save, for the run that ends with Q and Y.
fn save_dir() -> std::path::PathBuf {
    let dir = debug_game_dir();
    fs::write(
        dir.join(SAVED_GAM_FILENAME),
        saved_game_seed_bytes(0, 0, 5, 5),
    )
    .unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);
    dir
}

fn scene_in(dir: &Path) -> Scene {
    Scene {
        game_dir: dir.to_path_buf(),
        ..Scene::synthetic()
    }
}

/// The off run's replay of the recorded keys, where the feed put them.
#[derive(Resource)]
struct Replay(Vec<Vec<TypedKey>>, usize);

fn replay_keys(mut replay: ResMut<Replay>, visual: Option<ResMut<VisualState>>) {
    let frame = replay.1;
    replay.1 += 1;
    if let (Some(mut visual), Some(keys)) = (visual, replay.0.get(frame)) {
        visual.typed_keys.extend(keys.iter().copied());
    }
}

/// Every frame's state digest, at the end of the frame.
#[derive(Resource, Default)]
struct Digests(Vec<TrajectoryDigest>);

fn push_digest(visual: Option<Res<VisualState>>, mut digests: ResMut<Digests>) {
    if let Some(visual) = visual {
        digests.0.push(trajectory_digest(&visual.state));
    }
}

fn record_digests(app: &mut App) {
    app.init_resource::<Digests>()
        .add_systems(Last, push_digest);
}

fn shifted_tap(timeline: &mut Timeline, frame: usize, key: KeyCode) {
    timeline
        .event(
            frame,
            KeyCode::ShiftLeft,
            ButtonState::Pressed,
            false,
            Owner::Game,
        )
        .event(frame + 1, key, ButtonState::Pressed, false, Owner::Game)
        .event(frame + 2, key, ButtonState::Released, false, Owner::Game)
        .event(
            frame + 3,
            KeyCode::ShiftLeft,
            ButtonState::Released,
            false,
            Owner::Game,
        );
}

#[test]
fn the_recorded_keys_replayed_with_the_feature_off_give_the_same_game_and_save() {
    // The on run: auto wins the fight, the player walks out and saves.
    let on_dir = save_dir();
    let mut game = game_in(weak_arena(), &scene_in(&on_dir), fast);
    record_digests(&mut game.shell.app);
    game.frames(3);
    game.engage(Chord::UntilDone);
    game.run_until_handed_back(2000);
    assert_eq!(game.toast(), Some(AUTO_OFF_VICTORY));
    for _ in 0..40 {
        if !game.shell.visual().state.combat_active {
            break;
        }
        game.timeline.game_tap(game.frame, KeyCode::ArrowRight);
        game.frames(20);
    }
    assert!(!game.shell.visual().state.combat_active, "walked out");
    let at = game.frame + 5;
    shifted_tap(&mut game.timeline, at, KeyCode::KeyQ);
    shifted_tap(&mut game.timeline, at + 10, KeyCode::KeyY);
    game.frames(40);
    let on_digests = game.world().resource::<Digests>().0.clone();
    let fed = game.fed().to_vec();
    let frames = game.frame;
    let timeline = game.timeline.without_feature();
    let on_save = fs::read(on_dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_ne!(on_save, saved_game_seed_bytes(0, 0, 5, 5), "the run saved");

    // The off run: Classic, the player's keys only, the fed keys replayed
    // at the same frames through the test hook.
    let off_dir = save_dir();
    let mut off = Shell::new(
        weak_arena(),
        &scene_in(&off_dir),
        &VisualLaunchConfig::classic(),
        |app| {
            app.insert_resource(Replay(fed.clone(), 0))
                .add_systems(Update, replay_keys.before(drive_visual));
            record_digests(app);
        },
    );
    for frame in 0..frames {
        let events = if frame < timeline.len() {
            timeline.frame(frame).to_vec()
        } else {
            Vec::new()
        };
        off.step(&events);
    }
    let off_digests = &off.app.world().resource::<Digests>().0;
    assert_eq!(off_digests.len(), on_digests.len());
    if let Some(frame) = (0..on_digests.len()).find(|f| off_digests[*f] != on_digests[*f]) {
        panic!(
            "frame {frame}: off {} / on {}",
            off_digests[frame], on_digests[frame]
        );
    }
    assert_eq!(fs::read(off_dir.join(SAVED_GAM_FILENAME)).unwrap(), on_save);
    let _ = fs::remove_dir_all(on_dir);
    let _ = fs::remove_dir_all(off_dir);
}

// ---------------------------------------------------------------------------
// No spoiler (1.4.7, 6.2.3): a hand-back names nothing but a party member.

#[test]
fn hand_back_toasts_use_fixed_words_and_member_names_only() {
    let state = arena();
    let mut names = HashMap::new();
    let fixed = [
        AUTO_OFF_KEY,
        AUTO_OFF_VICTORY,
        AUTO_OFF_NO_TARGET,
        AUTO_OFF_STUCK,
        AUTO_OFF_CAP,
        AUTO_OFF_UNEXPECTED,
        AUTO_OFF,
    ];
    for reason in [
        HandBack::Interrupted,
        HandBack::Victory,
        HandBack::NoTarget,
        HandBack::Stuck,
        HandBack::ActionCap,
        HandBack::UnexpectedPrompt,
        HandBack::FightOver,
    ] {
        assert!(fixed.contains(&hand_back_toast(reason, &state, &mut names)));
    }
    let mut named = state.clone();
    if named.party_names.len() < 2 {
        named.party_names.resize(2, Default::default());
    }
    named.party_names[1] = Default::default();
    named.party_names[1][..4].copy_from_slice(b"Iolo");
    let hurt = hand_back_toast(HandBack::Hurt(1), &named, &mut names);
    assert_eq!(hurt, "Auto off: Iolo is hurt");
    assert!(std::ptr::eq(
        hurt,
        hand_back_toast(HandBack::Hurt(1), &named, &mut names)
    ));
}

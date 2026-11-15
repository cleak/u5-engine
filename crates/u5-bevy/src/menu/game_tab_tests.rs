//! The Game tab and the unsaved-progress question in the real `--visual`
//! app, headless (design 3.2.6, 3.2.7 "Game", 3.3.8 "Injection"; plan
//! M4-13): Sound and Buffer from the menu equal Ctrl-S and Ctrl-B typed,
//! Escape at the bare prompt is still the game's, and the question shows
//! only when the turn moved.

use bevy::input::ButtonState;
use u5_runtime::PlayState;
use u5_runtime::enhancements::{SettingValue, apply_value};
use u5_runtime::test_fixtures::{open_grid, test_state};

use super::inject::{GameKeyInjection, SOUND_KEY};
use super::pages::{Dialog, GameKey, Leave, MenuCommand, Page, Unsaved};
use super::*;
use crate::TypedKey;
use crate::drive_visual;
use crate::enh::equivalence::{
    FrameRecord, Owner, Scene, Shell, TimedKey, Timeline, check_shell_equivalence,
};

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn overlay_launch() -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(OVERLAY, true);
    launch
}

/// The keys queued for the game just before the input driver read, per
/// frame.
#[derive(Resource, Default)]
struct QueueAtRead(Vec<Vec<TypedKey>>);

fn watch_queue(visual: Option<Res<VisualState>>, mut seen: ResMut<QueueAtRead>) {
    let queue = visual.map_or_else(Vec::new, |visual| {
        visual.typed_keys.iter().copied().collect()
    });
    seen.0.push(queue);
}

fn install_watch(app: &mut App) {
    app.init_resource::<QueueAtRead>()
        .add_systems(Update, watch_queue.after(EnhSet::Feed).before(drive_visual));
}

const OPEN_AT: usize = 10;
const CHOOSE_AT: usize = 20;
const FRAMES: usize = 60;

/// The menu opened with F12 and `key` chosen on the Game tab, as the
/// reducer gives it; the frame records and the keys the game found queued.
fn menu_run(key: GameKey) -> (Vec<FrameRecord>, Vec<Vec<TypedKey>>, Shell) {
    let mut shell = Shell::new(
        town(),
        &Scene::synthetic(),
        &overlay_launch(),
        install_watch,
    );
    let mut timeline = Timeline::new(FRAMES);
    timeline.feature_tap(OPEN_AT, KeyCode::F12);
    let mut records = Vec::new();
    for index in 0..timeline.len() {
        if index == CHOOSE_AT {
            assert_eq!(
                shell.app.world().resource::<OverlayState>().open(),
                Some(OverlayId::Menu)
            );
            shell
                .app
                .world_mut()
                .send_event(MenuCommandEvent(MenuCommand::TypeGameKey(key)));
        }
        records.push(shell.step(timeline.frame(index)));
    }
    let queues = shell.app.world().resource::<QueueAtRead>().0.clone();
    (records, queues, shell)
}

/// A player's Ctrl+`key` at `frame`, released at the next.
fn ctrl_tap(timeline: &mut Timeline, frame: usize, key: KeyCode) {
    for code in [KeyCode::ControlLeft, key] {
        timeline.event(frame, code, ButtonState::Pressed, false, Owner::Game);
    }
    for code in [key, KeyCode::ControlLeft] {
        timeline.event(frame + 1, code, ButtonState::Released, false, Owner::Game);
    }
}

#[test]
fn sound_and_buffer_from_the_menu_equal_the_typed_keys() {
    for (key, code) in [
        (GameKey::Sound, KeyCode::KeyS),
        (GameKey::Buffer, KeyCode::KeyB),
    ] {
        let (menu, queues, shell) = menu_run(key);
        // The menu closed and fed the one key, then let the token go.
        assert_eq!(shell.app.world().resource::<OverlayState>().open(), None);
        let fed: Vec<(usize, TypedKey)> = queues
            .iter()
            .enumerate()
            .flat_map(|(frame, keys)| keys.iter().map(move |key| (frame, *key)))
            .collect();
        assert_eq!(fed.len(), 1, "{key:?}: {fed:?}");
        let (frame, typed) = fed[0];
        assert_eq!(
            (typed.key, typed.shift, typed.control),
            (code, false, true),
            "{key:?}"
        );
        assert_eq!(frame, CHOOSE_AT, "{key:?}: fed as the menu closed");
        let owner = shell.app.world().resource::<SyntheticInputOwner>();
        assert_eq!(owner.holder(), None, "{key:?}");
        assert_eq!(
            shell.app.world().resource::<GameKeyInjection>().typing(),
            None
        );
        let state = &shell.visual().state;
        match key {
            GameKey::Sound => assert!(!state.music_enabled),
            GameKey::Buffer => assert!(state.typeahead_buffer_enabled),
        }

        // The player types Ctrl and the key in that frame instead.
        let mut timeline = Timeline::new(FRAMES);
        ctrl_tap(&mut timeline, frame, code);
        let mut typed_shell = Shell::new(
            town(),
            &Scene::synthetic(),
            &VisualLaunchConfig::classic(),
            |_| {},
        );
        let typed: Vec<FrameRecord> = (0..FRAMES)
            .map(|index| typed_shell.step(timeline.frame(index)))
            .collect();
        for (at, (a, b)) in menu.iter().zip(&typed).enumerate() {
            assert_eq!(
                a.digest, b.digest,
                "{key:?} frame {at}: state, PRNG, turn, transcript"
            );
            assert_eq!(a.frame, b.frame, "{key:?} frame {at}: the game's picture");
            assert_eq!(a.queue, b.queue, "{key:?} frame {at}: the key queue");
        }
    }
}

#[test]
fn a_game_key_waits_while_another_owner_holds_the_token() {
    let mut shell = Shell::new(town(), &Scene::synthetic(), &overlay_launch(), |_| {});
    let mut timeline = Timeline::new(OPEN_AT + 4);
    timeline.feature_tap(OPEN_AT, KeyCode::F12);
    for index in 0..timeline.len() {
        shell.step(timeline.frame(index));
    }
    shell
        .app
        .world_mut()
        .resource_mut::<SyntheticInputOwner>()
        .take(crate::enh::synthetic::SyntheticOwner::StartKeys)
        .unwrap();
    shell
        .app
        .world_mut()
        .send_event(MenuCommandEvent(MenuCommand::TypeGameKey(GameKey::Sound)));
    for _ in 0..5 {
        shell.step(&[]);
    }
    assert!(shell.visual().state.music_enabled, "nothing typed");
    assert_eq!(
        shell.app.world().resource::<OverlayState>().open(),
        Some(OverlayId::Menu),
        "the menu stays open"
    );
    assert_eq!(SOUND_KEY.key, KeyCode::KeyS);
}

/// The game's own text window, as the screen shows it.
fn text_window(shell: &Shell) -> String {
    shell.visual().state.clone().render_text_window_frame(None)
}

#[test]
fn escape_at_the_bare_prompt_still_prints_what() {
    let mut timeline = Timeline::new(30);
    timeline.game_tap(10, KeyCode::Escape);
    let mut shell = Shell::new(town(), &Scene::synthetic(), &overlay_launch(), |_| {});
    for index in 0..timeline.len() {
        shell.step(timeline.frame(index));
    }
    assert_eq!(shell.app.world().resource::<OverlayState>().open(), None);
    let text = text_window(&shell);
    assert!(text.contains("What?"), "{text}");
    // And it is exactly the Classic game's answer.
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &timeline,
        &overlay_launch(),
        |_| {},
    );
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

/// The `--visual` app with the menu, stepped one frame at a time.
struct Game {
    shell: Shell,
}

impl Game {
    fn new(launch: &VisualLaunchConfig) -> Self {
        let mut game = Self {
            shell: Shell::new(town(), &Scene::synthetic(), launch, |_| {}),
        };
        game.frames(10);
        game
    }

    fn frames(&mut self, count: usize) {
        for _ in 0..count {
            self.shell.step(&[]);
        }
    }

    fn tap(&mut self, key: KeyCode) {
        let event = |state| TimedKey {
            key,
            state,
            repeat: false,
            owner: Owner::Game,
        };
        self.shell.step(&[event(ButtonState::Pressed)]);
        self.shell.step(&[event(ButtonState::Released)]);
        self.frames(8);
    }

    fn world(&self) -> &World {
        self.shell.app.world()
    }

    fn page(&self) -> Option<Page> {
        self.world()
            .resource::<MenuState>()
            .model()
            .map(|model| model.current().page)
    }

    /// Taps `key` and counts the exits the press frame asked for.
    fn press_exits(&mut self, key: KeyCode) -> usize {
        let event = |state| TimedKey {
            key,
            state,
            repeat: false,
            owner: Owner::Game,
        };
        self.shell.step(&[event(ButtonState::Pressed)]);
        let exits = self.exits();
        self.shell.step(&[event(ButtonState::Released)]);
        exits
    }

    fn exits(&self) -> usize {
        self.world()
            .resource::<Events<AppExit>>()
            .iter_current_update_events()
            .count()
    }
}

#[test]
fn the_question_shows_only_when_the_turn_moved() {
    let mut game = Game::new(&overlay_launch());
    game.tap(KeyCode::F12);
    assert_eq!(game.page(), Some(Page::Top));
    assert_eq!(world_snapshot(game.world()).unsaved, None, "nothing moved");
    game.tap(KeyCode::Escape);
    assert_eq!(game.page(), None);

    // A step moves the turn.
    let turn = game.shell.visual().state.turn;
    game.tap(KeyCode::ArrowRight);
    let moved = game.shell.visual().state.turn - turn;
    assert!(moved > 0, "the step took a turn");
    game.tap(KeyCode::F12);
    assert_eq!(
        world_snapshot(game.world()).unsaved,
        Some(Unsaved::Turns(moved))
    );
    // The short menu: Return to the game, Settings, Leave the game.
    game.tap(KeyCode::ArrowDown);
    game.tap(KeyCode::ArrowDown);
    assert_eq!(game.press_exits(KeyCode::Enter), 0, "asked first: no exit");
    assert_eq!(
        game.page(),
        Some(Page::Dialog(Dialog::Unsaved(Leave::Desktop)))
    );
    assert_eq!(game.press_exits(KeyCode::ArrowLeft), 0);
    assert_eq!(game.press_exits(KeyCode::Enter), 1, "Yes leaves");
}

#[test]
fn the_question_is_not_asked_with_the_warning_off() {
    let mut launch = overlay_launch();
    apply_value(
        &mut launch.enhancements,
        "menu.confirm_unsaved",
        SettingValue::Bool(false),
    )
    .unwrap();
    let mut game = Game::new(&launch);
    game.tap(KeyCode::ArrowRight);
    game.tap(KeyCode::F12);
    assert!(world_snapshot(game.world()).turns_since_save.unwrap() > 0);
    assert_eq!(world_snapshot(game.world()).unsaved, None);
}

#[test]
fn unsaved_is_asked_only_in_play_with_the_warning_on() {
    assert_eq!(unsaved(true, true, Some(0)), None);
    assert_eq!(unsaved(true, true, Some(3)), Some(Unsaved::Turns(3)));
    assert_eq!(unsaved(true, true, None), Some(Unsaved::Session));
    assert_eq!(unsaved(false, true, Some(3)), None);
    assert_eq!(unsaved(true, false, Some(3)), None, "before play");
    assert_eq!(
        text::unsaved_line(Unsaved::Turns(1)),
        "1 turn since thy last save will be lost."
    );
    assert_eq!(
        text::unsaved_line(Unsaved::Turns(37)),
        "37 turns since thy last save will be lost."
    );
    assert_eq!(text::unsaved_line(Unsaved::Session), text::UNSAVED_SESSION);
}

//! The menu plugin in the real `--visual` app, headless (design 3.2.1,
//! 3.3.4, 3.4.2, 3.4.3; plan M4-04): where F12 opens the menu, that no
//! menu key reaches the game, that an open and close equals no menu at
//! equal idle ticks, the QA script, the doors, and the censuses.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use bevy::ecs::schedule::Schedules;
use bevy::input::ButtonState;
use u5_runtime::PlayState;
use u5_runtime::enhancements::{LockReason, ResolvedEnhancements, TOGGLES};
use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::z_stats::{YesNoPromptKind, YesNoPromptSession};

use super::pages::{MenuDoor, Page, PageState, SettingsTab};
use super::*;
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey, Timeline, check_shell_equivalence};
use crate::enh::overlay::CloseReason;
use crate::enh::toast::ToastKind;
use crate::enh::{EnhancementsPlugin, add_enhancements};
use crate::{HeldDirectionRepeat, key_code_to_input_byte, key_code_to_line_input_byte};

/// A synthetic town at its idle command prompt.
fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

fn launch_with(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

/// The `--visual` app on [`town`], driven one frame at a time, with the
/// keyboard events of the next frame queued by the helpers.
struct Driver {
    shell: Shell,
    next: Vec<TimedKey>,
}

impl Driver {
    fn new(launch: &VisualLaunchConfig) -> Self {
        let mut driver = Self {
            shell: Shell::new(town(), &Scene::synthetic(), launch, |_| {}),
            next: Vec::new(),
        };
        // Startup and a few idle frames: the prompt is idle.
        driver.frames(10);
        driver
    }

    fn event(&mut self, key: KeyCode, state: ButtonState, repeat: bool) -> &mut Self {
        self.next.push(TimedKey {
            key,
            state,
            repeat,
            owner: Owner::Game,
        });
        self
    }

    fn press(&mut self, key: KeyCode) -> &mut Self {
        self.event(key, ButtonState::Pressed, false)
    }

    fn repeat(&mut self, key: KeyCode) -> &mut Self {
        self.event(key, ButtonState::Pressed, true)
    }

    fn release(&mut self, key: KeyCode) -> &mut Self {
        self.event(key, ButtonState::Released, false)
    }

    fn frame(&mut self) -> &mut Self {
        let events = std::mem::take(&mut self.next);
        self.shell.step(&events);
        self
    }

    fn frames(&mut self, count: usize) -> &mut Self {
        for _ in 0..count {
            self.frame();
        }
        self
    }

    /// Presses `key` for one frame and releases it in the next.
    fn tap(&mut self, key: KeyCode) -> &mut Self {
        self.press(key).frame().release(key).frame()
    }

    fn world(&self) -> &World {
        self.shell.app.world()
    }

    fn open(&self) -> Option<OverlayId> {
        self.world().resource::<OverlayState>().open()
    }

    fn page(&self) -> Option<PageState> {
        self.world()
            .resource::<MenuState>()
            .model()
            .map(MenuModel::current)
    }

    fn toast(&self) -> Option<String> {
        self.world()
            .get_resource::<Toasts>()
            .and_then(|toasts| toasts.current().map(str::to_string))
    }

    fn events<E: Event + Copy>(&self) -> Vec<E> {
        self.world()
            .resource::<Events<E>>()
            .iter_current_update_events()
            .copied()
            .collect()
    }

    /// The Avatar's cell and the turn: what a step changes.
    fn avatar(&self) -> (usize, usize, u64) {
        let state = &self.shell.visual().state;
        (state.player.x, state.player.y, state.turn)
    }

    fn assert_nothing_typed(&self, what: &str) {
        assert!(
            self.shell.visual().typed_keys.is_empty(),
            "{what}: {:?}",
            self.shell.visual().typed_keys
        );
    }
}

fn top(cursor: usize) -> Option<PageState> {
    Some(PageState {
        page: Page::Top,
        cursor,
    })
}

// ---------------------------------------------------------------------------
// Opening and closing (3.2.1).

#[test]
fn f12_at_the_idle_prompt_opens_the_menu_and_f12_or_escape_closes_it() {
    let mut game = Driver::new(&launch_with(&[OVERLAY]));
    assert_eq!(game.open(), None);
    assert_eq!(game.page(), None);
    assert!(!game.world().resource::<MenuShell>().intro_shell);

    game.press(KeyCode::F12).frame();
    assert_eq!(game.open(), Some(OverlayId::Menu));
    assert_eq!(game.page(), top(0));
    assert_eq!(game.toast(), None);
    game.release(KeyCode::F12).frame();
    assert_eq!(game.open(), Some(OverlayId::Menu));

    game.press(KeyCode::Escape).frame();
    assert_eq!(game.open(), None);
    assert_eq!(game.page(), None);
    assert_eq!(
        game.events::<MenuCommandEvent>(),
        [MenuCommandEvent(MenuCommand::Close)]
    );
    game.release(KeyCode::Escape).frame();

    // Reopened, the menu starts afresh; F12 closes it again.
    game.tap(KeyCode::F12).tap(KeyCode::ArrowDown);
    assert_eq!(game.page(), top(1));
    game.press(KeyCode::F12).frame();
    assert_eq!(game.open(), None);
    game.release(KeyCode::F12).frame().tap(KeyCode::F12);
    assert_eq!(game.page(), top(0));
    game.assert_nothing_typed("open and close");
}

/// While the menu is open no key reaches the game's queue, OS repeat and
/// the closing frame included; a key held across close stays latched, so
/// its repeats never type; released and pressed again it is the game's.
#[test]
fn menu_keys_never_reach_the_game() {
    let mut game = Driver::new(&launch_with(&[OVERLAY]));
    let start = game.avatar();
    game.tap(KeyCode::F12);
    assert_eq!(game.open(), Some(OverlayId::Menu));

    // A held arrow with OS repeat for well past the repeat delay.
    game.press(KeyCode::ArrowUp);
    for frame in 0..90 {
        game.frame();
        game.assert_nothing_typed(&format!("held arrow, frame {frame}"));
        assert_eq!(
            game.shell.visual().held_direction_repeat,
            HeldDirectionRepeat::default()
        );
        if frame % 3 == 0 {
            game.repeat(KeyCode::ArrowUp);
        }
    }
    game.release(KeyCode::ArrowUp).frame();
    // The short menu: Resume, Settings, Leave the game. Down and Enter open
    // Settings; Left and Right walk its tabs; other keys do nothing.
    for key in [
        KeyCode::KeyA,
        KeyCode::KeyQ,
        KeyCode::Space,
        KeyCode::Tab,
        KeyCode::Backspace,
        KeyCode::F11,
        KeyCode::ArrowDown,
        KeyCode::Enter,
        KeyCode::ArrowRight,
        KeyCode::ArrowRight,
        KeyCode::ArrowLeft,
    ] {
        game.tap(key);
        game.assert_nothing_typed(&format!("{key:?}"));
    }
    assert_eq!(
        game.page(),
        Some(PageState {
            page: Page::Settings(SettingsTab::Display),
            cursor: 0
        })
    );
    game.tap(KeyCode::Escape);
    assert_eq!(game.page(), top(1));
    assert_eq!(game.avatar(), start);

    // Escape closes it with the arrow held: not in the close frame, not on
    // repeat after it.
    game.press(KeyCode::ArrowUp).frame();
    game.press(KeyCode::Escape).frame();
    assert_eq!(game.open(), None);
    game.assert_nothing_typed("the close frame");
    for _ in 0..30 {
        game.repeat(KeyCode::ArrowUp)
            .repeat(KeyCode::Escape)
            .frame();
        game.assert_nothing_typed("repeat after close");
    }
    assert_eq!(game.avatar(), start);
    game.release(KeyCode::ArrowUp)
        .release(KeyCode::Escape)
        .frame();
    game.press(KeyCode::ArrowUp).frame();
    game.release(KeyCode::ArrowUp).frames(20);
    assert_ne!(game.avatar(), start, "a fresh press moves");
}

/// A walk held into the menu stops: the held direction is released on
/// open and the arrow's repeats never move the Avatar while it is open.
#[test]
fn held_arrows_stop_on_open() {
    let mut game = Driver::new(&launch_with(&[OVERLAY]));
    let start = game.avatar();
    game.press(KeyCode::ArrowRight).frame();
    // Hold it until the Avatar walks on its own (the held repeat), then
    // press F12 on each frame until the idle prompt takes it.
    let mut frames = 0;
    while game.open().is_none() {
        frames += 1;
        assert!(frames < 600, "the menu never opened");
        if frames % 3 == 0 {
            game.repeat(KeyCode::ArrowRight);
        }
        if frames > 60 {
            game.press(KeyCode::F12);
        }
        game.frame();
        if frames > 60 {
            game.release(KeyCode::F12);
        }
    }
    assert_ne!(game.avatar(), start, "the held arrow walked");
    // Let any step in flight finish.
    game.frames(30);
    let stopped = game.avatar();
    for frame in 0..120 {
        if frame % 3 == 0 {
            game.repeat(KeyCode::ArrowRight);
        }
        game.frame();
        assert_eq!(game.avatar(), stopped, "frame {frame}");
        assert_eq!(
            game.shell.visual().held_direction_repeat,
            HeldDirectionRepeat::default()
        );
        game.assert_nothing_typed("held into the menu");
    }
    assert_eq!(game.open(), Some(OverlayId::Menu));
}

/// F12 away from the idle prompt opens nothing, shows the 2 s toast, and
/// leaves the game exactly as a player who pressed nothing.
#[test]
fn f12_away_from_the_idle_prompt_is_refused_and_changes_nothing() {
    let mut off = Driver::new(&VisualLaunchConfig::classic());
    let mut on = Driver::new(&launch_with(&[OVERLAY]));
    let prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    off.shell
        .app
        .world_mut()
        .resource_mut::<VisualState>()
        .state
        .active_yes_no_prompt = prompt;
    on.shell
        .app
        .world_mut()
        .resource_mut::<VisualState>()
        .state
        .active_yes_no_prompt = prompt;
    assert_eq!(off.shell.record(), on.shell.record());
    on.press(KeyCode::F12);
    for frame in 0..40 {
        if frame == 1 {
            on.release(KeyCode::F12);
        }
        off.frame();
        on.frame();
        assert_eq!(off.shell.record(), on.shell.record(), "frame {frame}");
        assert_eq!(on.open(), None, "frame {frame}");
        if frame == 0 {
            assert_eq!(on.toast().as_deref(), Some(text::OPENS_AT_PROMPT));
            let toasts = on.world().resource::<Toasts>();
            assert_eq!(toasts.kind(), Some(ToastKind::Short));
        }
    }
    assert!(on.shell.visual().state.active_yes_no_prompt.is_some());
}

/// F12 behind another key in the same frame breaks the first-key rule: it
/// goes to the game, which drops it as 1.0.1 does, and a toast explains.
#[test]
fn f12_as_the_second_key_of_a_frame_falls_through() {
    let mut game = Driver::new(&launch_with(&[OVERLAY]));
    game.press(KeyCode::KeyX).press(KeyCode::F12).frame();
    assert_eq!(
        game.events::<HotkeyFellThrough>(),
        [HotkeyFellThrough(EnhAction::OpenMenu)]
    );
    assert_eq!(game.open(), None);
    game.release(KeyCode::KeyX).release(KeyCode::F12).frames(5);
    assert_eq!(game.open(), None);
    assert_eq!(game.toast().as_deref(), Some(text::OPENS_AT_PROMPT));

    // The game sees exactly what the Classic game sees for the same keys.
    let mut timeline = Timeline::new(120);
    for frame in [20, 60] {
        timeline
            .event(
                frame,
                KeyCode::ArrowUp,
                ButtonState::Pressed,
                false,
                Owner::Game,
            )
            .event(
                frame,
                KeyCode::F12,
                ButtonState::Pressed,
                false,
                Owner::Game,
            )
            .event(
                frame + 1,
                KeyCode::ArrowUp,
                ButtonState::Released,
                false,
                Owner::Game,
            )
            .event(
                frame + 1,
                KeyCode::F12,
                ButtonState::Released,
                false,
                Owner::Game,
            );
    }
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &timeline,
        &launch_with(&[OVERLAY]),
        |_| {},
    );
    assert_eq!(mismatches, []);
}

/// With everything off F11 and F12 map to no byte, and pressing them is
/// pressing nothing.
#[test]
fn f11_and_f12_give_no_byte_with_everything_off() {
    for key in [KeyCode::F11, KeyCode::F12] {
        for shift in [false, true] {
            for ctrl in [false, true] {
                assert_eq!(key_code_to_input_byte(key, shift, ctrl), None);
                assert_eq!(key_code_to_line_input_byte(key, shift, ctrl), None);
            }
        }
    }
    let classic = VisualLaunchConfig::classic();
    let mut timeline = Timeline::new(100);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .feature_tap(30, KeyCode::F12)
        .feature_tap(40, KeyCode::F11)
        .game_tap(50, KeyCode::ArrowLeft)
        .feature_tap(70, KeyCode::F12);
    assert_eq!(
        check_shell_equivalence(&town(), &Scene::synthetic(), &timeline, &classic, |_| {}),
        []
    );
    // Pressed while the game is busy, the key waits in the queue until
    // the game reads it and drops it; the game itself is untouched.
    timeline.feature_tap(51, KeyCode::F12);
    let mismatches =
        check_shell_equivalence(&town(), &Scene::synthetic(), &timeline, &classic, |_| {});
    assert!(
        mismatches.iter().all(|m| m.what == "key queue"),
        "{mismatches:?}"
    );
    let game = Driver::new(&classic);
    assert!(!game.world().contains_resource::<OverlayState>());
    assert!(!game.world().contains_resource::<Toasts>());
    assert!(!game.world().contains_resource::<MenuState>());
}

/// Whether the menu was open at the end of each frame.
#[derive(Resource, Clone, Default)]
struct OpenFrames(Arc<Mutex<Vec<bool>>>);

fn note_open(state: Res<OverlayState>, frames: Res<OpenFrames>) {
    frames
        .0
        .lock()
        .unwrap()
        .push(state.open() == Some(OverlayId::Menu));
}

/// Design 3.4.3 (a): the player walks, opens the menu, moves about in it
/// (a held arrow with repeat, a page and its tabs), closes it and walks
/// on. For the same game keys at the same frames the game equals a player
/// who never opened it: the menu's frames are idle frames.
#[test]
fn an_open_and_close_equals_no_menu_at_equal_idle_ticks() {
    let mut timeline = Timeline::new(240);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::Space)
        .game_tap(50, KeyCode::ArrowDown)
        .feature_tap(80, KeyCode::F12)
        .feature_tap(90, KeyCode::ArrowDown)
        .feature_tap(95, KeyCode::Enter)
        .feature_tap(100, KeyCode::ArrowRight)
        .feature_hold(105, 130, KeyCode::ArrowDown)
        .feature_tap(135, KeyCode::KeyN)
        .feature_tap(140, KeyCode::Escape)
        .feature_tap(150, KeyCode::F12)
        .game_tap(165, KeyCode::ArrowUp)
        .game_tap(180, KeyCode::ArrowRight)
        .feature_tap(200, KeyCode::F12)
        .feature_tap(215, KeyCode::Escape)
        .game_tap(230, KeyCode::ArrowDown);
    let open = OpenFrames::default();
    let recorder = open.clone();
    let mismatches = check_shell_equivalence(
        &town(),
        &Scene::synthetic(),
        &timeline,
        &launch_with(&[OVERLAY]),
        move |app| {
            app.insert_resource(recorder)
                .add_systems(Update, note_open.after(EnhSet::Observe));
        },
    );
    assert_eq!(mismatches, []);
    let open = open.0.lock().unwrap();
    assert!(!open[79], "closed before F12");
    assert!(open[80..150].iter().all(|o| *o), "open from F12");
    assert!(open[150..200].iter().all(|o| !*o), "F12 closed it");
    assert!(open[200..215].iter().all(|o| *o), "open again");
    assert!(open[215..].iter().all(|o| !*o), "Escape closed it");
}

// ---------------------------------------------------------------------------
// Closing from outside, doors, the title layer, the script.

#[test]
fn the_menu_closes_when_the_prompt_stops_being_idle() {
    let mut game = Driver::new(&launch_with(&[OVERLAY]));
    game.tap(KeyCode::F12).tap(KeyCode::ArrowDown);
    assert_eq!(game.page(), top(1));
    game.shell
        .app
        .world_mut()
        .resource_mut::<VisualState>()
        .state
        .active_yes_no_prompt = Some(YesNoPromptSession {
        kind: YesNoPromptKind::SaveGame,
    });
    game.frame();
    assert_eq!(game.open(), None);
    assert_eq!(
        game.events::<OverlayChanged>(),
        [OverlayChanged::Closed {
            id: OverlayId::Menu,
            why: CloseReason::NotIdle,
        }]
    );
    assert_eq!(game.page(), None);
}

/// A door (pushed by its owner's plugin; none ships yet, D-M4.9) closes
/// the menu and opens its overlay; Tab is the map's door key.
#[test]
fn a_door_closes_the_menu_and_opens_its_overlay() {
    let mut game = Driver::new(&launch_with(&[OVERLAY]));
    assert!(game.world().resource::<MenuDoors>().0.is_empty());
    game.tap(KeyCode::F12).tap(KeyCode::Tab);
    assert_eq!(
        game.open(),
        Some(OverlayId::Menu),
        "no map door: Tab is unbound"
    );

    game.shell
        .app
        .world_mut()
        .resource_mut::<MenuDoors>()
        .0
        .push(MenuDoor {
            label: "A test map",
            opens: OverlayId::Map,
        });
    game.press(KeyCode::Tab).frame();
    assert_eq!(game.open(), Some(OverlayId::Map));
    assert_eq!(
        game.events::<OverlayChanged>(),
        [
            OverlayChanged::Closed {
                id: OverlayId::Menu,
                why: CloseReason::Replaced,
            },
            OverlayChanged::Opened(OverlayId::Map),
        ]
    );
    assert_eq!(game.page(), None);
    game.release(KeyCode::Tab).frame();
    game.assert_nothing_typed("through the door");
}

/// With only the title layer on, F12 in play is consumed by the router
/// and opens nothing (D-M4.3); no toast.
#[test]
fn with_only_the_title_layer_f12_in_play_opens_nothing() {
    let mut game = Driver::new(&launch_with(&[TITLE_LAYER]));
    let start = game.avatar();
    game.press(KeyCode::F12).frame();
    assert_eq!(
        game.events::<EnhActionEvent>(),
        [EnhActionEvent(EnhAction::OpenMenu)]
    );
    game.release(KeyCode::F12).frames(5);
    assert_eq!(game.open(), None);
    assert_eq!(game.page(), None);
    assert_eq!(game.toast(), None);
    game.assert_nothing_typed("F12");
    assert_eq!(game.avatar(), start);
    // The game's keys are the game's.
    game.tap(KeyCode::ArrowUp).frames(20);
    assert_ne!(game.avatar(), start);
}

/// The QA script (D-M4.6) is fed one key per frame, only while the menu is
/// open, and none of it reaches the game.
#[test]
fn the_qa_script_feeds_one_key_per_frame_while_open() {
    let mut launch = launch_with(&[OVERLAY]);
    launch.menu_script = Some("↓\r→↓\u{1b}↓\r".chars().collect());
    let mut game = Driver::new(&launch);
    let left = |game: &Driver| game.world().resource::<MenuScript>().0.len();
    game.frames(30);
    assert_eq!(left(&game), 7, "nothing is fed while the menu is shut");

    game.press(KeyCode::F12).frame();
    assert_eq!(game.page(), top(0));
    assert_eq!(left(&game), 7, "the opening frame feeds nothing");
    game.release(KeyCode::F12).frame();
    assert_eq!(game.page(), top(1));
    game.frame();
    let settings = |tab, cursor| {
        Some(PageState {
            page: Page::Settings(tab),
            cursor,
        })
    };
    assert_eq!(game.page(), settings(SettingsTab::Audio, 0));
    game.frame();
    assert_eq!(game.page(), settings(SettingsTab::Display, 0));
    game.frame().frame();
    assert_eq!(game.page(), top(1), "Escape went back");
    game.frame();
    assert_eq!(game.page(), top(2));
    game.frame();
    assert_eq!(
        game.events::<MenuCommandEvent>(),
        [MenuCommandEvent(MenuCommand::QuitToDesktop)]
    );
    assert_eq!(left(&game), 0);
    game.frames(5);
    assert_eq!(game.open(), Some(OverlayId::Menu));
    game.assert_nothing_typed("the script");

    // No script: no script resource or system.
    let game = Driver::new(&launch_with(&[OVERLAY]));
    assert!(!game.world().contains_resource::<MenuScript>());
}

#[test]
fn script_characters_press_the_keys_they_name() {
    let key = |k| Some(OverlayKey::plain(k));
    assert_eq!(script_input('\u{1b}'), (key(KeyCode::Escape), None));
    assert_eq!(script_input('\r'), (key(KeyCode::Enter), None));
    assert_eq!(script_input('\n'), (key(KeyCode::Enter), None));
    assert_eq!(script_input('\t'), (key(KeyCode::Tab), None));
    assert_eq!(script_input('\u{8}'), (key(KeyCode::Backspace), None));
    assert_eq!(script_input('\u{7f}'), (key(KeyCode::Delete), None));
    assert_eq!(script_input('\u{c}'), (key(KeyCode::F12), None));
    assert_eq!(script_input('↑'), (key(KeyCode::ArrowUp), None));
    assert_eq!(script_input('↓'), (key(KeyCode::ArrowDown), None));
    assert_eq!(script_input('←'), (key(KeyCode::ArrowLeft), None));
    assert_eq!(script_input('→'), (key(KeyCode::ArrowRight), None));
    assert_eq!(script_input(' '), (key(KeyCode::Space), Some(' ')));
    assert_eq!(script_input('j'), (key(KeyCode::KeyJ), Some('j')));
    assert_eq!(
        script_input('J'),
        (
            Some(OverlayKey {
                key: KeyCode::KeyJ,
                shift: true,
                ctrl: false
            }),
            Some('J')
        )
    );
    assert_eq!(script_input('7'), (key(KeyCode::Digit7), Some('7')));
    assert_eq!(script_input('-'), (None, Some('-')));
    assert_eq!(script_input('é'), (None, None));
    assert_eq!(script_input('\u{0}'), (None, None));
}

// ---------------------------------------------------------------------------
// The censuses (3.4.2).

/// Every system of every schedule of `app`, by schedule and name.
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

/// The systems `launch` adds to a bare app, by schedule and name.
fn systems_of(launch: &VisualLaunchConfig) -> (App, BTreeSet<String>) {
    let bare = system_names(&App::new());
    let mut app = App::new();
    add_enhancements(&mut app, launch);
    let added = system_names(&app).difference(&bare).cloned().collect();
    (app, added)
}

const MENU_SYSTEMS: [&str; 4] = [
    "menu::menu_actions",
    "menu::menu_input",
    "menu::menu_follow_overlay",
    "menu::menu_fell_through",
];

fn assert_no_menu(app: &App, what: &str) {
    assert!(!app.is_plugin_added::<MenuPlugin>(), "{what}");
    let world = app.world();
    assert!(!world.contains_resource::<MenuState>(), "{what}");
    assert!(!world.contains_resource::<MenuDoors>(), "{what}");
    assert!(!world.contains_resource::<MenuShell>(), "{what}");
    assert!(!world.contains_resource::<MenuScript>(), "{what}");
    assert!(
        !world.contains_resource::<Events<MenuCommandEvent>>(),
        "{what}"
    );
}

/// Off: Classic, the lock (with the menu toggles and a script), and every
/// toggle but the two menu toggles on add nothing of the menu.
#[test]
fn the_off_census_finds_no_menu() {
    let (app, systems) = systems_of(&VisualLaunchConfig::classic());
    assert!(systems.is_empty(), "{systems:?}");
    assert_no_menu(&app, "Classic");

    let mut locked = launch_with(&[OVERLAY, TITLE_LAYER]);
    locked.enhancements.lock = ResolvedEnhancements::locked(LockReason::QaEnv).lock;
    locked.menu_script = Some(vec!['\r']);
    let (app, systems) = systems_of(&locked);
    assert!(systems.is_empty(), "{systems:?}");
    assert!(!app.is_plugin_added::<EnhancementsPlugin>());
    assert_no_menu(&app, "the lock");

    let mut rest = VisualLaunchConfig::classic();
    for spec in TOGGLES {
        if ![OVERLAY, TITLE_LAYER].contains(&spec.toggle) {
            rest.enhancements.toggles.set(spec.toggle, true);
        }
    }
    rest.menu_script = Some(vec!['\r']);
    let (app, systems) = systems_of(&rest);
    assert!(app.is_plugin_added::<EnhancementsPlugin>());
    assert_no_menu(&app, "every other toggle");
    // The audio plugins (M4-08) are their own toggles' (3.4.2), not the
    // menu's: they live beside it and work without it.
    assert!(
        systems
            .iter()
            .all(|name| !name.contains("::menu::") || name.contains("::menu::audio::")),
        "{systems:?}"
    );
}

/// The 3.4.2 table, row by row: each chapter 3 toggle alone, beside a
/// stand-in toggle that brings only the foundation, adds only its own
/// plugin. Of the table's plugins M4-04 builds `MenuPlugin` and M4-08
/// `VolumePlugin` and `FocusMutePlugin` (their own census is in
/// `audio_tests`); the overlay also keeps the display chapter's plugin it
/// has brought since M1, whose window wish serves as `VsyncPlugin`
/// (M4-08), and the gem map's, installed dormant (M4-06, D-M4.4), and
/// either menu toggle brings the router, the overlay model and the toasts
/// the menu runs on. Every other chapter 3 toggle adds nothing yet.
#[test]
fn each_chapter_3_toggle_alone_adds_only_its_own_plugin() {
    let stand_in = Toggle::EnhancementsFreezeOverlays;
    let (_, base) = systems_of(&launch_with(&[stand_in]));
    let mut seen = 0;
    for spec in TOGGLES.iter().filter(|spec| spec.ships.chapter == 3) {
        let toggle = spec.toggle;
        let (app, systems) = systems_of(&launch_with(&[stand_in, toggle]));
        let added: Vec<&String> = systems.difference(&base).collect();
        let allowed: &[&str] = match toggle {
            OVERLAY => &[
                "u5_bevy::enh::input::",
                "u5_bevy::enh::overlay::",
                "u5_bevy::enh::toast::",
                "u5_bevy::display",
                // The display plugin's presentation host.
                "u5_bevy::enh::present::",
                // The gem map, installed dormant (D-M4.4).
                "u5_bevy::enh::gem_map",
                "u5_bevy::menu::",
            ],
            TITLE_LAYER => &[
                "u5_bevy::enh::input::",
                "u5_bevy::enh::overlay::",
                "u5_bevy::enh::toast::",
                "u5_bevy::menu::",
            ],
            // `VolumePlugin` and `FocusMutePlugin` (M4-08).
            Toggle::AudioVolume | Toggle::AudioMuted | Toggle::AudioMuteOnFocusLoss => {
                &["u5_bevy::menu::audio::"]
            }
            _ => &[],
        };
        let audio_only = allowed == ["u5_bevy::menu::audio::"];
        for name in &added {
            let path = name.split_once(' ').map_or(name.as_str(), |(_, path)| path);
            assert!(
                allowed.iter().any(|prefix| path.starts_with(prefix)),
                "{toggle:?} added {name}"
            );
        }
        if allowed.is_empty() {
            assert!(added.is_empty(), "{toggle:?}: {added:?}");
            assert_no_menu(&app, spec.key);
        } else if audio_only {
            assert!(!added.is_empty(), "{toggle:?}");
            assert_no_menu(&app, spec.key);
        } else {
            assert!(app.is_plugin_added::<MenuPlugin>(), "{toggle:?}");
            for system in MENU_SYSTEMS {
                assert!(
                    added.iter().any(|name| name.ends_with(system)),
                    "{toggle:?} lacks {system}"
                );
            }
            assert_eq!(
                app.world().resource::<MenuDoors>(),
                &MenuDoors::default(),
                "doors ship empty"
            );
        }
        seen += 1;
    }
    assert!(seen >= 9, "the chapter 3 toggles: {seen}");
}

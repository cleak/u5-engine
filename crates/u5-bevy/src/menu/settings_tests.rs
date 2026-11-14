//! The menu's settings plumbing in the real `--visual` app, headless
//! (design 3.2.7, 3.3.6; build plan M4-06, D-M4.4, D-M4.5): the menu's rows
//! are the wizard's, valued changes apply at their point and are stored
//! once, the command-line mark, "next launch" rows change nothing now, and
//! the dormant installs leave the game equal to all off.

use std::sync::{Arc, Mutex};

use u5_runtime::PlayState;
use u5_runtime::enhancements::page::{RowKind, RowValue};
use u5_runtime::enhancements::{
    ApplyAt, CliSetting, HostCapabilities, LockState, ResolveInput, SettingValue, SettingsSink,
    Toggle, resolve, settings_rows,
};
use u5_runtime::test_fixtures::{open_grid, test_state};

use super::*;
use crate::display::Display;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};
use crate::enh::{Toggles, VisualLaunchConfig};

const BRIGHTNESS: &str = "display.brightness";

fn town() -> PlayState {
    test_state(open_grid(), 5, 5)
}

/// A launch resolved from the command line, as the host does it, with the
/// menu overlay on. Its row has not shipped yet (it ships with the Phase 1
/// exit), so the resolver would refuse it; the shell does not care.
fn launch(cli: &[CliSetting]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements = resolve(ResolveInput {
        stored: None,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(
        launch.enhancements.notes.is_empty(),
        "{:?}",
        launch.enhancements.notes
    );
    launch.enhancements.toggles.set(Toggle::MenuOverlay, true);
    launch
}

fn enable(key: &str) -> CliSetting {
    CliSetting::Enable(key.to_string())
}

/// The menu overlay alone: a 4:3 launch with the menu.
fn menu_only() -> VisualLaunchConfig {
    let launch = launch(&[]);
    assert_eq!(
        launch.enhancements.toggles.iter_on().collect::<Vec<_>>(),
        [Toggle::MenuOverlay]
    );
    launch
}

#[derive(Default)]
struct RecordingSink(Mutex<Vec<Vec<(String, SettingValue)>>>);

impl SettingsSink for RecordingSink {
    fn store(&self, changes: &[(String, SettingValue)]) -> Result<(), String> {
        self.0.lock().unwrap().push(changes.to_vec());
        Ok(())
    }
}

struct Game {
    shell: Shell,
    sink: Arc<RecordingSink>,
}

impl Game {
    fn new(mut launch: VisualLaunchConfig) -> Self {
        let sink = Arc::new(RecordingSink::default());
        launch.settings_sink = Some(sink.clone());
        let mut game = Self {
            shell: Shell::new(town(), &Scene::synthetic(), &launch, |_| {}),
            sink,
        };
        for _ in 0..5 {
            game.frame();
        }
        game
    }

    fn frame(&mut self) {
        self.shell.step(&[]);
    }

    fn world(&self) -> &World {
        self.shell.app.world()
    }

    fn command(&mut self, command: MenuCommand) {
        self.shell
            .app
            .world_mut()
            .send_event(MenuCommandEvent(command));
        self.frame();
    }

    fn view(&self) -> SettingsView {
        let world = self.world();
        settings_view(
            world.resource::<LiveSettings>(),
            world.resource::<EnhLaunch>().0.enhancements.toggles,
            world.resource::<PendingToggleChanges>(),
            world.resource::<PendingSettingChanges>(),
        )
    }

    fn row(&self, key: &str) -> (SettingRow, RowNote) {
        let view = self.view();
        let (row, note) = view.row(key).unwrap();
        (row.clone(), note)
    }

    fn display(&self) -> &u5_runtime::enhancements::DisplaySettings {
        &self.world().resource::<Display>().0
    }

    fn toggle(&self, toggle: Toggle) -> bool {
        self.world().resource::<Toggles>().0.on(toggle)
    }

    fn stored(&self) -> Vec<Vec<(String, SettingValue)>> {
        self.sink.0.lock().unwrap().clone()
    }
}

/// D-M4.5: for the same resolution the menu's rows are the wizard's (the
/// wizard's `wizard_settings` is this generator over the file's
/// resolution; `u5-tui`'s wizard tests compare the two through the
/// desktop host).
#[test]
fn the_menus_rows_equal_the_wizards_for_the_same_resolution() {
    let launch = launch(&[
        enable("display.sprite_composite"),
        CliSetting::Set {
            key: BRIGHTNESS.to_string(),
            value: "70".to_string(),
        },
    ]);
    let game = Game::new(launch.clone());
    let view = game.view();
    assert_eq!(view.page.rows, settings_rows(&launch.enhancements));
    assert_eq!(
        view.page,
        u5_runtime::enhancements::settings_page_rows(&launch.enhancements, false, Vec::new())
    );
    assert_eq!(view.notes.len(), view.page.rows.len());
}

#[test]
fn note_words_name_the_point_and_the_command_line() {
    let note = |apply, by_cli| RowNote { apply, by_cli };
    assert!(note_words(note(ApplyAt::Frame, false)).is_empty());
    assert_eq!(
        note_words(note(ApplyAt::IdlePrompt, false)),
        [t::NEXT_PROMPT]
    );
    assert_eq!(
        note_words(note(ApplyAt::NextLaunch, true)),
        [t::NEXT_LAUNCH, t::SET_BY_COMMAND_LINE]
    );
    assert_eq!(
        note_words(note(ApplyAt::Frame, true)),
        [t::SET_BY_COMMAND_LINE]
    );
}

#[test]
fn a_step_moves_one_place_and_stops_at_the_ends() {
    let rows = settings_rows(&u5_runtime::enhancements::ResolvedEnhancements::classic());
    let brightness = rows.iter().find(|row| row.key == BRIGHTNESS).unwrap();
    let RowKind::Choice(choices) = &brightness.kind else {
        panic!("brightness is a choice");
    };
    // Listed 100 first (9.2.1). Left and Right step the slider by 5%
    // (3.2.7, M4-07); Flip walks the named stops.
    assert_eq!(choices[0].0, "100");
    assert_eq!(brightness.value, RowValue::Choice("100".into()));
    assert_eq!(stepped(brightness, 1), None, "stops at the top");
    assert_eq!(stepped(brightness, -1), Some(RowValue::Choice("95".into())));
    assert_eq!(stepped(brightness, 0), Some(RowValue::Choice("85".into())));
    let mut lowest = brightness.clone();
    lowest.value = RowValue::Choice("25".into());
    assert_eq!(stepped(&lowest, -1), None, "stops at the bottom");
    assert_eq!(stepped(&lowest, 1), Some(RowValue::Choice("30".into())));
    assert_eq!(stepped(&lowest, 0), Some(RowValue::Choice("100".into())));
    // A value between steps goes to the step beside it.
    let mut between = brightness.clone();
    between.value = RowValue::Choice("72".into());
    assert_eq!(stepped(&between, -1), Some(RowValue::Choice("70".into())));
    assert_eq!(stepped(&between, 1), Some(RowValue::Choice("75".into())));
    let scale = rows
        .iter()
        .find(|row| row.key == "display.scale_mode")
        .unwrap();
    let RowKind::Choice(words) = &scale.kind else {
        panic!("the scale mode is a choice");
    };
    assert_eq!(scale.value, RowValue::Choice(words[0].0.clone()));
    assert_eq!(stepped(scale, -1), None);
    assert_eq!(
        stepped(scale, 1),
        Some(RowValue::Choice(words[1].0.clone()))
    );
    let halo = rows
        .iter()
        .find(|row| row.key == "display.composite_halo")
        .unwrap();
    assert_eq!(stepped(halo, 0), Some(RowValue::Switch(false)));
    assert_eq!(stepped(halo, 1), None, "already on");
    assert_eq!(stepped(halo, -1), Some(RowValue::Switch(false)));
}

/// A valued change from the menu applies on the next frame to the
/// `Display` the display chapter follows, sets its switch, and is stored
/// once.
#[test]
fn a_valued_change_applies_at_its_point_and_is_stored_once() {
    let mut game = Game::new(menu_only());
    assert_eq!(game.display().light.brightness, 100);
    let (row, note) = game.row(BRIGHTNESS);
    assert_eq!(
        note.apply,
        ApplyAt::Frame,
        "brightness is installed dormant"
    );
    let Some(RowValue::Choice(lower)) = stepped(&row, -1) else {
        panic!("a lower stop");
    };
    let lower: i64 = lower.parse().unwrap();

    game.command(MenuCommand::Step(BRIGHTNESS, -1));
    assert_eq!(i64::from(game.display().light.brightness), lower);
    assert!(
        game.toggle(Toggle::DisplayBrightness),
        "the valued toggle's switch follows"
    );
    let live = game.world().resource::<LiveSettings>();
    assert_eq!(live.now.values.int(BRIGHTNESS), Some(lower));
    assert_eq!(live.chosen.values.int(BRIGHTNESS), Some(lower));
    assert!(
        game.world()
            .resource::<PendingSettingChanges>()
            .0
            .is_empty()
    );
    let stored = vec![vec![(BRIGHTNESS.to_string(), SettingValue::Int(lower))]];
    assert_eq!(game.stored(), stored);
    for _ in 0..5 {
        game.frame();
    }
    assert_eq!(game.stored(), stored, "stored once");
    assert_eq!(
        game.row(BRIGHTNESS).0.value,
        RowValue::Choice(lower.to_string()),
        "the row shows it"
    );

    // Back to 100 switches it off again.
    game.command(MenuCommand::Step(BRIGHTNESS, 1));
    assert_eq!(game.display().light.brightness, 100);
    assert!(!game.toggle(Toggle::DisplayBrightness));
}

/// A grow-left row from a 4:3 launch reads "next launch" and changes
/// nothing now; the choice is stored and the row shows it.
#[test]
fn a_grow_left_row_from_a_4_3_launch_changes_nothing_now() {
    let mut game = Game::new(menu_only());
    let (_, note) = game.row("display.grow_left");
    assert_eq!(note.apply, ApplyAt::NextLaunch);
    assert_eq!(note_words(note), [t::NEXT_LAUNCH]);
    let display = game.display().clone();
    let toggles = game.world().resource::<Toggles>().0;

    game.command(MenuCommand::Flip("display.grow_left"));
    for _ in 0..3 {
        game.frame();
    }
    assert_eq!(*game.display(), display, "the display is untouched");
    assert_eq!(game.world().resource::<Toggles>().0, toggles);
    let live = game.world().resource::<LiveSettings>();
    assert!(!live.now.toggles.on(Toggle::DisplayGrowLeft));
    assert!(live.chosen.toggles.on(Toggle::DisplayGrowLeft));
    assert_eq!(
        game.stored(),
        [vec![(
            "display.grow_left".to_string(),
            SettingValue::Bool(true)
        )]]
    );
    assert_eq!(
        game.row("display.grow_left").0.value,
        RowValue::Switch(true)
    );

    // So does the scale mode from a faithful launch.
    let (_, note) = game.row("display.scale_mode");
    assert_eq!(note.apply, ApplyAt::NextLaunch);
    game.command(MenuCommand::Step("display.scale_mode", 1));
    assert_eq!(game.display().scale_mode, display.scale_mode);
    assert_eq!(game.stored().len(), 2);
}

/// 3.3.6: a key the command line set is marked, and a change from the
/// menu is still stored.
#[test]
fn a_key_the_command_line_set_is_marked_and_still_stored_when_changed() {
    let mut game = Game::new(launch(&[CliSetting::Set {
        key: BRIGHTNESS.to_string(),
        value: "70".to_string(),
    }]));
    let (row, note) = game.row(BRIGHTNESS);
    assert!(note.by_cli);
    assert_eq!(note_words(note), [t::SET_BY_COMMAND_LINE]);
    assert!(!game.row("display.composite_halo").1.by_cli);
    assert_eq!(row.value, RowValue::Choice("70".to_string()));

    game.command(MenuCommand::Step(BRIGHTNESS, 1));
    let Some(RowValue::Choice(higher)) = stepped(&row, 1) else {
        panic!("a higher stop");
    };
    let higher: i64 = higher.parse().unwrap();
    assert_eq!(i64::from(game.display().light.brightness), higher);
    assert_eq!(
        game.stored(),
        [vec![(BRIGHTNESS.to_string(), SettingValue::Int(higher))]]
    );
    assert!(game.row(BRIGHTNESS).1.by_cli, "the mark is the launch's");
}

/// A toggle row flips through the toggle queue at its point: compositing,
/// installed dormant, applies on the next frame.
#[test]
fn a_dormant_toggle_switches_on_in_play() {
    let mut game = Game::new(menu_only());
    assert!(!game.display().light.composite);
    game.command(MenuCommand::Flip("display.sprite_composite"));
    assert!(game.display().light.composite);
    assert!(game.toggle(Toggle::DisplaySpriteComposite));
    assert_eq!(
        game.stored(),
        [vec![(
            "display.sprite_composite".to_string(),
            SettingValue::Bool(true)
        )]]
    );
}

/// Other commands, and a command for a key with no row, queue nothing.
#[test]
fn other_commands_queue_nothing() {
    let mut game = Game::new(menu_only());
    game.command(MenuCommand::Step("no.such_key", 1));
    game.command(MenuCommand::Flip("enhancements.schema"));
    game.command(MenuCommand::Close);
    assert!(game.stored().is_empty());
    assert_eq!(
        *game.world().resource::<LiveSettings>(),
        LiveSettings::at_launch(&menu_only().enhancements)
    );
}

/// D-M4.4: with `menu.overlay` on and everything else off, the dormant
/// installs (the side fill, brightness, compositing, the gem map) are in
/// the app, and the game and its frame equal all off, frame by frame.
#[test]
fn the_dormant_arms_leave_frame_and_trajectory_equal_to_all_off() {
    let launch = menu_only();
    let game = Game::new(launch.clone());
    assert!(
        game.shell
            .app
            .is_plugin_added::<crate::enh::gem_map::GemMapPlugin>(),
        "the gem map is installed dormant"
    );
    assert!(
        game.world()
            .contains_resource::<crate::display::fill::FillState>()
    );
    drop(game);

    let mut timeline = Timeline::new(150);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::Space)
        .game_tap(50, KeyCode::ArrowDown)
        .game_tap(70, KeyCode::ArrowRight)
        .game_tap(90, KeyCode::ArrowRight)
        .game_tap(110, KeyCode::ArrowUp)
        .game_tap(130, KeyCode::ArrowLeft);
    let mismatches =
        check_shell_equivalence(&town(), &Scene::synthetic(), &timeline, &launch, |_| {});
    assert_eq!(mismatches, []);
}

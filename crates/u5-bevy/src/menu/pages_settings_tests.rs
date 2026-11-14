//! The top menu's Settings pages and Leave the game (design 3.2.1, 3.2.5,
//! 3.2.7; build plan M4-07): each tab's rows for fixed settings (rows, not
//! pixels), the preset row and "Custom", the spoiler labels, no game names
//! in any label or help line, the lock and the Cheats group, and that
//! leaving the game saves nothing.

use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::sync::{Arc, Mutex, OnceLock};

use u5_runtime::enhancements::page::{
    PresetChoice, RowKind as PageRowKind, RowRole, RowValue, SettingRow, SettingsRows,
};
use u5_runtime::enhancements::{
    ApplyAt, CatalogueRow, CliSetting, HostCapabilities, LockReason, LockState, PRESETS, Phase,
    Preset, ResolveInput, ResolvedEnhancements, RowNote, SETTING_GROUPS, SHIPPED_PHASES,
    SettingValue, SettingsSink, Tag, Toggle, help_line, resolve, settings_rows_in,
};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, configured_test_dir, open_grid, test_state, unique_scratch_dir,
};

use super::pages::{
    MenuCommand, MenuInput, MenuModel, Page, PageState, Row, RowAction, RowKind, SettingsTab,
    ShellSnapshot, Shown, page_rows, reduce, settle,
};
use super::render::{SLIDER_INNER, VALUE_COL, label_room, setting_help, setting_line, tag_fits};
use super::settings::*;
use super::text as t;
use super::{MenuCommandEvent, MenuShell, MenuState};
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey};
use crate::enh::input::OverlayKey;
use crate::enh::{LiveSettings, PendingSettingChanges, PendingToggleChanges, VisualLaunchConfig};
use bevy::input::ButtonState;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Helpers.

/// Every phase any catalogue row ships in: the build as it will be when
/// all of them have shipped.
fn every_phase() -> &'static [Phase] {
    static PHASES: OnceLock<Vec<Phase>> = OnceLock::new();
    PHASES.get_or_init(|| {
        let mut phases: Vec<Phase> = Vec::new();
        for row in CatalogueRow::all() {
            if !phases.contains(&row.ships()) {
                phases.push(row.ships());
            }
        }
        phases
    })
}

/// This build's phases and the menu's own Phases 1-4, which ship at the
/// end of the milestone (M4-13).
pub(super) fn with_menu_phases() -> &'static [Phase] {
    static PHASES: OnceLock<Vec<Phase>> = OnceLock::new();
    PHASES.get_or_init(|| {
        let mut phases = SHIPPED_PHASES.to_vec();
        phases.extend([
            u5_runtime::enhancements::MENU_P1,
            u5_runtime::enhancements::MENU_P2,
            u5_runtime::enhancements::MENU_P3,
            u5_runtime::enhancements::MENU_P4,
        ]);
        phases
    })
}

fn resolved(cli: &[CliSetting]) -> ResolvedEnhancements {
    let resolved = resolve(ResolveInput {
        stored: None,
        cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(resolved.notes.is_empty(), "{:?}", resolved.notes);
    resolved
}

/// The view a run launched with `launch` shows before any change, as a
/// build with `phases` lists it.
fn view_of(launch: &ResolvedEnhancements, phases: &[Phase]) -> SettingsView {
    settings_view_in(
        &LiveSettings::at_launch(launch),
        launch.toggles,
        &PendingToggleChanges::default(),
        &PendingSettingChanges::default(),
        phases,
    )
}

fn tabs(launch: &ResolvedEnhancements, phases: &[Phase]) -> [Vec<Row>; 4] {
    settings_tabs(&view_of(launch, phases), None)
}

fn labels(rows: &[Row]) -> Vec<&'static str> {
    rows.iter().map(|row| row.label).collect()
}

fn key_of(row: &Row) -> Option<&'static str> {
    match row.action {
        RowAction::Setting(key) => Some(key),
        _ => None,
    }
}

fn keys_of(rows: &[Row]) -> Vec<&'static str> {
    rows.iter().filter_map(key_of).collect()
}

fn key(code: KeyCode) -> MenuInput {
    MenuInput::Key(OverlayKey::plain(code))
}

// ---------------------------------------------------------------------------
// The rows of each tab (3.2.7).

/// Audio: Volume with its slider, Mute and Mute on focus loss, as the
/// catalogue holds them; none of them before the menu's Phase 1 ships.
#[test]
fn the_audio_tab_lists_volume_and_the_two_mutes() {
    let classic = ResolvedEnhancements::classic();
    assert!(
        tabs(&classic, SHIPPED_PHASES)[SettingsTab::Audio.index()].is_empty(),
        "hidden until Phase 1 ships"
    );
    let audio = &tabs(&classic, with_menu_phases())[SettingsTab::Audio.index()];
    assert_eq!(keys_of(audio), AUDIO_KEYS);
    assert_eq!(audio.len(), 3);
    assert_eq!(labels(audio), [t::VOLUME, t::MUTE, t::MUTE_ON_FOCUS_LOSS]);
    assert_eq!(
        audio[0].kind,
        RowKind::Value(Shown::Percent {
            value: 100,
            fill: 100
        })
    );
    assert_eq!(audio[1].kind, RowKind::Check { on: false });
    assert_eq!(audio[2].kind, RowKind::Check { on: false });
    // Volume and the focus mute have help lines; Mute says enough, and the
    // footer shows the tab's help under it.
    assert!(audio[0].help.is_some() && audio[2].help.is_some());
    assert_eq!(audio[1].help, None);
    // Each row's tag is its effective apply point in this run (D-M4.4).
    let launch = ResolvedEnhancements::classic().toggles;
    for row in audio {
        assert!(row.enabled && row.indent == 0, "{row:?}");
        let tag = match u5_runtime::enhancements::effective_apply(key_of(row).unwrap(), launch) {
            ApplyAt::Frame => None,
            ApplyAt::IdlePrompt => Some(t::NEXT_PROMPT),
            ApplyAt::NextLaunch => Some(t::NEXT_LAUNCH),
        };
        assert_eq!(row.tag, tag, "{row:?}");
    }
}

/// Volume steps by 5 and stops at the ends; the slider fills over 0-100.
#[test]
fn volume_steps_by_five() {
    let rows = settings_rows_in(&ResolvedEnhancements::classic(), with_menu_phases());
    let volume = rows.iter().find(|row| row.key == "audio.volume").unwrap();
    assert_eq!(volume.value, RowValue::Choice("100".into()));
    assert_eq!(stepped(volume, 1), None, "stops at the top");
    assert_eq!(stepped(volume, -1), Some(RowValue::Choice("95".into())));
    let mut quiet = volume.clone();
    quiet.value = RowValue::Choice("0".into());
    assert_eq!(stepped(&quiet, -1), None, "stops at the bottom");
    assert_eq!(stepped(&quiet, 1), Some(RowValue::Choice("5".into())));
    let row = setting_row(&quiet, note(ApplyAt::Frame), 0).unwrap();
    assert_eq!(
        row.kind,
        RowKind::Value(Shown::Percent { value: 0, fill: 0 })
    );
    let mut seventy = volume.clone();
    seventy.value = RowValue::Choice("70".into());
    let row = setting_row(&seventy, note(ApplyAt::Frame), 0).unwrap();
    let line = setting_line(&row, 38);
    let text: String = line.cells.iter().map(|(c, _)| *c as char).collect();
    // `[######--]` with 8 cells inside (70% of 8 rounds to 6), then the
    // percent ending at the last text cell.
    let at = VALUE_COL as usize;
    assert_eq!(&text[at..at + SLIDER_INNER as usize + 2], "[######--]");
    assert!(text.trim_end().ends_with("70%"), "{text:?}");
    assert_eq!(text.len() - text.trim_end().len(), 1, "the margin cell");
}

fn note(apply: ApplyAt) -> RowNote {
    RowNote {
        apply,
        by_cli: false,
    }
}

/// Display: every `[display]` row the generator lists, in its order; the
/// grow-left sub-rows indented directly under "Wider world view";
/// chapter 9's rows; no Tile graphics row.
#[test]
fn the_display_tab_is_generated_from_the_catalogue() {
    for launch in [
        ResolvedEnhancements::classic(),
        resolved(&[CliSetting::Enable("display.grow_left".into())]),
    ] {
        let display = &tabs(&launch, SHIPPED_PHASES)[SettingsTab::Display.index()];
        let generated: Vec<String> = settings_rows_in(&launch, SHIPPED_PHASES)
            .into_iter()
            .filter(|row| row.key.starts_with("display."))
            .map(|row| row.key)
            .collect();
        assert_eq!(keys_of(display), generated);
        assert_eq!(display.len(), generated.len(), "only setting rows");
        assert!(
            !keys_of(display).contains(&"display.raster"),
            "no Tile graphics"
        );
        for key in [
            "display.brightness",
            "display.brightness_scope",
            "display.map_brightness",
            "display.sprite_composite",
            "display.composite_halo",
            "display.vsync",
            "display.scale_mode",
        ] {
            assert!(keys_of(display).contains(&key), "{key}");
        }
        // The grow-left rows sit under their parent, indented two cells.
        let at = display
            .iter()
            .position(|row| key_of(row) == Some("display.grow_left"))
            .unwrap();
        assert_eq!(display[at].label, "Wider world view");
        assert_eq!(display[at].indent, 0);
        let subs: Vec<&str> = display[at + 1..]
            .iter()
            .take_while(|row| row.indent == SUB_INDENT)
            .filter_map(key_of)
            .collect();
        assert_eq!(
            subs,
            [
                "display.grow_left_light",
                "display.crawl",
                "display.typed_colour",
                "display.ribbon_hp_colours",
                "display.dither",
                "display.centre_party",
            ]
        );
        assert_eq!(
            labels(&display[at + 2..at + 4]),
            ["Messages over map", "Cyan typed text"]
        );
        // Each row shows its value as the generator has it.
        let generated = settings_rows_in(&launch, SHIPPED_PHASES);
        for row in display {
            let source = generated
                .iter()
                .find(|g| Some(g.key.as_str()) == key_of(row))
                .unwrap();
            match (&source.value, row.kind) {
                (RowValue::Switch(on), RowKind::Check { on: shown }) => assert_eq!(*on, shown),
                (RowValue::Choice(text), RowKind::Value(Shown::Word(word))) => {
                    assert_eq!(text, word)
                }
                (RowValue::Choice(text), RowKind::Value(Shown::Percent { value, .. })) => {
                    assert_eq!(text, &value.to_string())
                }
                other => panic!("{}: {other:?}", source.key),
            }
        }
    }
    // On a 4:3 launch the grow-left rows apply at the next launch (D-M4.4).
    let display = &tabs(&ResolvedEnhancements::classic(), SHIPPED_PHASES)[1];
    let grow = display
        .iter()
        .find(|row| key_of(row) == Some("display.grow_left"))
        .unwrap();
    assert_eq!(grow.tag, Some(t::NEXT_LAUNCH));
    assert!(tag_fits(grow, 38));
}

/// Game: the six rows of 3.2.7, greyed until they arrive (M4-13), with the
/// game's own live Sound and Buffer values.
#[test]
fn the_game_tab_is_greyed_with_the_live_values() {
    let view = view_of(&ResolvedEnhancements::classic(), with_menu_phases());
    let game = &settings_tabs(
        &view,
        Some(GameValues {
            sound_on: true,
            buffer_on: false,
        }),
    )[SettingsTab::Game.index()];
    assert_eq!(
        labels(game),
        [
            t::SOUND,
            t::BUFFER,
            t::START_SOUND_OFF,
            t::START_BUFFER_ON,
            t::CONFIRM_UNSAVED,
            t::PACING
        ]
    );
    assert_eq!(game[0].kind, RowKind::Value(Shown::Word(t::ON)));
    assert_eq!(game[1].kind, RowKind::Value(Shown::Word(t::OFF)));
    assert_eq!(game[2].kind, RowKind::Check { on: false });
    assert_eq!(game[3].kind, RowKind::Check { on: false });
    assert_eq!(game[4].kind, RowKind::Check { on: true }, "the default");
    assert_eq!(game[5].kind, RowKind::Value(Shown::Word(t::RESERVED)));
    for row in game {
        assert!(!row.enabled, "{row:?}");
        assert_eq!(row.reason, Some(t::GAME_LATER));
        assert_eq!(row.action, RowAction::None);
    }
    // Before play there are no live values.
    let game = &settings_tabs(&view, None)[SettingsTab::Game.index()];
    assert_eq!(game[0].kind, RowKind::Value(Shown::Blank));
    // Every key on the greyed tab gives no command; Left and Right walk the
    // tabs from any row.
    let mut snap = ShellSnapshot::default();
    snap.settings[SettingsTab::Game.index()] = game.clone();
    for cursor in 0..game.len() {
        for (code, tab) in [
            (KeyCode::ArrowLeft, SettingsTab::Display),
            (KeyCode::ArrowRight, SettingsTab::Enhancements),
        ] {
            let mut model = on(Page::Settings(SettingsTab::Game), cursor);
            assert_eq!(reduce(&mut model, key(code), &snap), []);
            assert_eq!(model.current().page, Page::Settings(tab));
        }
        for code in [KeyCode::Enter, KeyCode::Space] {
            let mut model = on(Page::Settings(SettingsTab::Game), cursor);
            assert_eq!(reduce(&mut model, key(code), &snap), []);
        }
    }
}

fn on(page: Page, cursor: usize) -> MenuModel {
    let mut model = MenuModel::new();
    let snap = ShellSnapshot::default();
    // Settings is the short menu's second row.
    let short = ShellSnapshot {
        intro_shell: false,
        ..snap
    };
    reduce(&mut model, key(KeyCode::ArrowDown), &short);
    reduce(&mut model, key(KeyCode::Enter), &short);
    let mut snap = ShellSnapshot::default();
    while model.current().page != page {
        reduce(&mut model, key(KeyCode::ArrowRight), &snap);
    }
    // Put the cursor where it is wanted, over a page of plain items.
    snap.settings = std::array::from_fn(|_| vec![Row::item("x", RowAction::None); 9]);
    for _ in 0..cursor {
        reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    }
    assert_eq!(
        model.current(),
        PageState { page, cursor },
        "the model on {page:?} row {cursor}"
    );
    model
}

/// Enhancements: the preset row, then every row of the shared generator in
/// its group, under the group's heading, in `SETTING_GROUPS` order; a
/// group's rows set in one cell and a sub-option three; no Cheats group.
#[test]
fn the_enhancements_tab_is_the_wizard_s_rows_grouped() {
    for phases in [SHIPPED_PHASES, with_menu_phases()] {
        let launch = ResolvedEnhancements::classic();
        let extras = &tabs(&launch, phases)[SettingsTab::Enhancements.index()];
        assert_eq!(
            extras[0].kind,
            RowKind::Preset {
                current: Some(0),
                count: PRESETS.len() as u8
            },
            "Classic"
        );
        assert_eq!(extras[0].action, RowAction::Preset);
        let generated = settings_rows_in(&launch, phases);
        assert!(generated.iter().all(|row| !row.role.is_cheat_group()));
        assert_eq!(
            keys_of(extras),
            generated
                .iter()
                .map(|row| row.key.as_str())
                .collect::<Vec<_>>()
        );
        let headers: Vec<&str> = extras
            .iter()
            .filter(|row| row.kind == RowKind::Header)
            .map(|row| row.label)
            .collect();
        let mut groups: Vec<&str> = Vec::new();
        for row in &generated {
            if groups.last() != Some(&row.group.as_str()) {
                groups.push(
                    SETTING_GROUPS
                        .iter()
                        .find(|(g, _)| *g == row.group)
                        .unwrap()
                        .0,
                );
            }
        }
        assert_eq!(headers, groups);
        let order: Vec<usize> = headers
            .iter()
            .map(|h| SETTING_GROUPS.iter().position(|(g, _)| g == h).unwrap())
            .collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{headers:?}");
        for (row, source) in extras
            .iter()
            .filter(|row| key_of(row).is_some())
            .zip(&generated)
        {
            let want = GROUP_INDENT + if source.indent { SUB_INDENT } else { 0 };
            assert_eq!(row.indent, want, "{}", source.key);
        }
        assert!(!labels(extras).contains(&t::CHEATS_GROUP));
    }
    // With the menu's phases the menu's own rows are in "Menu and saves".
    let extras = &tabs(&ResolvedEnhancements::classic(), with_menu_phases())[3];
    let at = labels(extras)
        .iter()
        .position(|l| *l == "Menu and saves")
        .unwrap();
    assert_eq!(
        labels(&extras[at + 1..at + 4]),
        ["Menu overlay (F12)", "Warn if unsaved", "Return to title"],
        "the overlay's sub-options under it"
    );
}

/// The cursor never rests on a heading: it starts on the preset row and
/// steps over every heading.
#[test]
fn the_cursor_passes_over_the_headings() {
    let extras = tabs(&ResolvedEnhancements::classic(), with_menu_phases())[3].clone();
    let mut snap = ShellSnapshot::default();
    snap.settings[SettingsTab::Enhancements.index()] = extras.clone();
    let mut model = on(Page::Settings(SettingsTab::Enhancements), 0);
    let mut seen = vec![model.current().cursor];
    for _ in 0..extras.len() + 2 {
        reduce(&mut model, key(KeyCode::ArrowDown), &snap);
        seen.push(model.current().cursor);
    }
    for _ in 0..extras.len() + 2 {
        reduce(&mut model, key(KeyCode::ArrowUp), &snap);
        seen.push(model.current().cursor);
    }
    assert!(seen.iter().all(|at| extras[*at].selectable()));
    let selectable = extras.iter().filter(|row| row.selectable()).count();
    assert_eq!(seen.iter().collect::<BTreeSet<_>>().len(), selectable);
    // A cursor asked for on a heading settles on the row below it.
    let heading = extras
        .iter()
        .position(|row| row.kind == RowKind::Header)
        .unwrap();
    assert_eq!(settle(&extras, heading), heading + 1);
}

// ---------------------------------------------------------------------------
// Presets and Custom (3.2.7).

/// The preset row's keys: Left and Right choose the preset before or after
/// the one in force, stopping at the ends; from Custom, Right chooses the
/// first and Left the last.
#[test]
fn left_and_right_choose_a_preset() {
    let mut snap = ShellSnapshot::default();
    for (current, left, right) in [
        (Some(0), None, Some(1)),
        (Some(1), Some(0), Some(2)),
        (Some(2), Some(1), None),
        (None, Some(2), Some(0)),
    ] {
        snap.settings[3] = vec![Row::preset(current, 3)];
        for (code, want) in [(KeyCode::ArrowLeft, left), (KeyCode::ArrowRight, right)] {
            let mut model = on(Page::Settings(SettingsTab::Enhancements), 0);
            let got = reduce(&mut model, key(code), &snap);
            assert_eq!(
                got,
                want.map(MenuCommand::Preset)
                    .into_iter()
                    .collect::<Vec<_>>()
            );
            assert_eq!(
                model.current().page,
                Page::Settings(SettingsTab::Enhancements)
            );
        }
        let mut model = on(Page::Settings(SettingsTab::Enhancements), 0);
        assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    }
}

/// A preset choice gives every row it governs the preset's value, and only
/// those that differ.
#[test]
fn a_preset_sets_every_row_it_governs() {
    let view = view_of(&ResolvedEnhancements::classic(), SHIPPED_PHASES);
    let everything = PRESETS
        .iter()
        .position(|p| *p == Preset::Everything)
        .unwrap();
    let preset: &PresetChoice = &view.page.presets[everything];
    let changes = command_changes(&view, MenuCommand::Preset(everything as u8));
    let differing: Vec<&str> = preset
        .values
        .iter()
        .filter(|(key, value)| view.row(key).unwrap().0.value != *value)
        .map(|(key, _)| key.as_str())
        .collect();
    assert!(!differing.is_empty());
    assert_eq!(
        changes.iter().map(|(key, _)| *key).collect::<Vec<_>>(),
        differing
    );
    for (key, value) in &changes {
        let want = &preset.values.iter().find(|(k, _)| k == key).unwrap().1;
        assert_eq!(*value, want.to_setting(key));
    }
    // Classic over Classic changes nothing; a preset out of range neither.
    assert_eq!(command_changes(&view, MenuCommand::Preset(0)), []);
    assert_eq!(command_changes(&view, MenuCommand::Preset(9)), []);
}

#[derive(Default)]
struct RecordingSink(Mutex<Vec<Vec<(String, SettingValue)>>>);

impl SettingsSink for RecordingSink {
    fn store(&self, changes: &[(String, SettingValue)]) -> Result<(), String> {
        self.0.lock().unwrap().push(changes.to_vec());
        Ok(())
    }
}

/// The menu-on launch (4:3) with a recording sink.
fn menu_launch(sink: &Arc<RecordingSink>) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements = ResolvedEnhancements::classic();
    launch.enhancements.toggles.set(Toggle::MenuOverlay, true);
    launch.settings_sink = Some(sink.clone());
    launch
}

struct Game {
    shell: Shell,
    next: Vec<TimedKey>,
}

impl Game {
    fn new(launch: &VisualLaunchConfig, scene: &Scene) -> Self {
        let mut game = Self {
            shell: Shell::new(test_state(open_grid(), 5, 5), scene, launch, |_| {}),
            next: Vec::new(),
        };
        game.frames(10);
        game
    }

    fn frames(&mut self, n: usize) -> &mut Self {
        for _ in 0..n {
            let events = std::mem::take(&mut self.next);
            self.shell.step(&events);
        }
        self
    }

    fn tap(&mut self, code: KeyCode) -> &mut Self {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            self.next.push(TimedKey {
                key: code,
                state,
                repeat: false,
                owner: Owner::Game,
            });
            self.frames(1);
        }
        self
    }

    fn world(&self) -> &World {
        self.shell.app.world()
    }

    fn snapshot(&self) -> ShellSnapshot {
        super::world_snapshot(self.world())
    }

    fn page(&self) -> Option<PageState> {
        self.world()
            .resource::<MenuState>()
            .model()
            .map(MenuModel::current)
    }

    fn rows(&self) -> Vec<Row> {
        page_rows(self.page().unwrap().page, &self.snapshot())
    }

    /// Moves the cursor down to the row `label`.
    fn down_to(&mut self, label: &str) -> &mut Self {
        let at = self
            .rows()
            .iter()
            .position(|row| row.label == label)
            .unwrap_or_else(|| panic!("no row {label}"));
        while self.page().unwrap().cursor < at {
            self.tap(KeyCode::ArrowDown);
        }
        assert_eq!(self.page().unwrap().cursor, at);
        self
    }

    fn preset_row(&self) -> RowKind {
        self.snapshot().settings[SettingsTab::Enhancements.index()][0].kind
    }
}

/// In the shell: choosing Everything sets its rows, the preset row shows
/// it, and a later change shows Custom; every change is stored.
#[test]
fn a_preset_then_a_change_shows_custom() {
    let sink = Arc::new(RecordingSink::default());
    let mut game = Game::new(&menu_launch(&sink), &Scene::synthetic());
    game.tap(KeyCode::F12);
    game.down_to(t::SETTINGS).tap(KeyCode::Enter);
    for _ in 0..3 {
        game.tap(KeyCode::ArrowRight);
    }
    assert_eq!(
        game.page(),
        Some(PageState {
            page: Page::Settings(SettingsTab::Enhancements),
            cursor: 0
        })
    );
    assert_eq!(
        game.preset_row(),
        RowKind::Preset {
            current: Some(0),
            count: 3
        }
    );
    // Right twice: Recommended, then Everything.
    game.tap(KeyCode::ArrowRight).frames(2);
    assert_eq!(
        game.preset_row(),
        RowKind::Preset {
            current: Some(1),
            count: 3
        }
    );
    game.tap(KeyCode::ArrowRight).frames(2);
    assert_eq!(
        game.preset_row(),
        RowKind::Preset {
            current: Some(2),
            count: 3
        }
    );
    let view = settings_view(
        game.world().resource::<LiveSettings>(),
        game.world()
            .resource::<crate::enh::EnhLaunch>()
            .0
            .enhancements
            .toggles,
        game.world().resource::<PendingToggleChanges>(),
        game.world().resource::<PendingSettingChanges>(),
    );
    for (key, value) in &view.page.presets[2].values {
        assert_eq!(view.row(key).unwrap().0.value, *value, "{key}");
    }
    let stored: usize = sink.0.lock().unwrap().iter().map(Vec::len).sum();
    assert!(stored > 0, "the preset's rows are stored");

    // A governed check box flipped: Custom.
    let governed: Vec<&str> = view.page.presets[2]
        .values
        .iter()
        .map(|(key, _)| key.as_str())
        .collect();
    let extras = game.rows();
    let flip = extras
        .iter()
        .find(|row| {
            matches!(row.kind, RowKind::Check { .. })
                && key_of(row).is_some_and(|key| governed.contains(&key))
        })
        .unwrap()
        .label;
    game.down_to(flip).tap(KeyCode::Space).frames(2);
    assert_eq!(
        game.preset_row(),
        RowKind::Preset {
            current: None,
            count: 3
        }
    );
}

// ---------------------------------------------------------------------------
// Spoiler labels and names (U17, 1.4.7).

/// Every row that may show the undiscovered (3.2.7's list as the catalogue
/// has it, and every `Tag::SpoilerRisk` row) starts its label and its help
/// with "Spoiler:"; no other row does.
#[test]
fn every_spoiler_row_says_so() {
    for key in t::SPOILER_KEYS {
        assert!(
            CatalogueRow::find(key).is_some(),
            "{key} is a catalogue row"
        );
    }
    let mut spoilers = 0;
    for row in CatalogueRow::all() {
        let key = row.key();
        let spoiler = t::SPOILER_KEYS.contains(&key) || row.tags().contains(&Tag::SpoilerRisk);
        assert_eq!(is_spoiler_row(key), spoiler, "{key}");
        let Some(label) = t::setting_label(key) else {
            continue;
        };
        assert_eq!(label.starts_with(t::SPOILER), spoiler, "{key}: {label}");
        if spoiler {
            spoilers += 1;
            let help = t::setting_help(key).expect("a spoiler row's own help");
            assert!(help.starts_with(t::SPOILER), "{key}: {help}");
        }
    }
    assert!(spoilers >= t::SPOILER_KEYS.len());
    // On the tabs, as the rows show them: label and help.
    let extras = &tabs(&ResolvedEnhancements::classic(), every_phase())[3];
    for row in extras
        .iter()
        .filter(|row| key_of(row).is_some_and(is_spoiler_row))
    {
        assert!(row.label.starts_with(t::SPOILER), "{row:?}");
        assert!(
            row.help.is_some_and(|h| h.starts_with(t::SPOILER)),
            "{row:?}"
        );
        let line = setting_line(row, 38);
        let at = 1 + usize::from(row.indent);
        assert!(
            line.cells[at..at + t::SPOILER.len()]
                .iter()
                .all(|(_, ink)| *ink == super::frame::Ink::Accent),
            "the word in brown"
        );
    }
}

/// Every row a page can list, in any build, has a label of the wording
/// table that fits its label cells at the deepest indent it can have
/// (a sub-option in its group on the Extras tab).
#[test]
fn every_row_has_a_label_that_fits() {
    let rows = settings_rows_in(&ResolvedEnhancements::classic(), every_phase());
    assert!(rows.len() > 30);
    for row in &rows {
        let label =
            t::setting_label(&row.key).unwrap_or_else(|| panic!("{} has no label", row.key));
        let indent = GROUP_INDENT + if row.indent { SUB_INDENT } else { 0 };
        let probe = Row::item(label, RowAction::None).indented(indent);
        assert!(
            label.len() <= label_room(&probe),
            "{}: {label:?} is {} cells, room {}",
            row.key,
            label.len(),
            label_room(&probe)
        );
        // The value fits after the label column too.
        let menu = setting_row(row, note(ApplyAt::Frame), indent).unwrap();
        let line = setting_line(&menu, 38);
        assert_eq!(line.cells.len(), 38);
        if let RowKind::Value(Shown::Word(word)) = menu.kind {
            assert!(VALUE_COL as usize + word.len() <= 37, "{word}");
        }
    }
}

/// The footer: the reason of a greyed row, else its help; the apply point
/// when the row had no room for its tag, and the command-line mark, on
/// lines of their own that the help gives way to.
#[test]
fn the_footer_says_the_help_and_the_notes() {
    let mut row = Row::shown("Scale mode", "display.scale_mode", Shown::Word("integer"))
        .with_help(Some("How the picture is scaled to the window"));
    assert_eq!(
        setting_help(Some(&row), None, 36, 4),
        ["How the picture is scaled to the", "window"]
    );
    row.tag = Some(t::NEXT_LAUNCH);
    assert!(
        !tag_fits(&row, 38),
        "a long word leaves no room for the tag"
    );
    row.by_cli = true;
    assert_eq!(
        setting_help(Some(&row), None, 36, 4),
        [
            "How the picture is scaled to the",
            "window",
            t::NOTE_NEXT_LAUNCH,
            t::SET_BY_COMMAND_LINE
        ]
    );
    for note in [
        t::NOTE_NEXT_LAUNCH,
        t::NOTE_NEXT_PROMPT,
        t::SET_BY_COMMAND_LINE,
    ] {
        assert!(note.len() <= 36, "{note} fits one footer line");
    }
    assert_eq!(
        setting_help(Some(&row), None, 36, 3),
        [
            "How the picture is scaled to the",
            t::NOTE_NEXT_LAUNCH,
            t::SET_BY_COMMAND_LINE
        ]
    );
    let greyed = row.greyed(t::GAME_LATER);
    assert_eq!(setting_help(Some(&greyed), None, 36, 4)[0], t::GAME_LATER);
    assert_eq!(
        setting_help(None, Some(t::NOTHING_HERE), 36, 4),
        [t::NOTHING_HERE]
    );
}

/// Words that would be a name of the game's world if a row said them, and
/// the menu's text checked against them: every label and help line a
/// Settings row can show (the wording table's and the catalogue's, which
/// is the footer's help), and the menu's own lines on these pages.
fn shown_texts() -> Vec<&'static str> {
    let mut texts: Vec<&'static str> = vec![
        t::MENU_TITLE,
        t::RESUME,
        t::SAVE,
        t::LOAD,
        t::SETTINGS,
        t::QUIT_TO_TITLE,
        t::QUIT_TO_DESKTOP,
        t::WAITING,
        t::SAVE_UNAVAILABLE,
        t::KEY_CLOSE,
        t::KEY_BACK,
        t::AUDIO_HELP,
        t::PRESET,
        t::CUSTOM,
        t::PRESET_HELP,
        t::GAME_LATER,
        t::NOTHING_HERE,
        t::NOTE_NEXT_PROMPT,
        t::NOTE_NEXT_LAUNCH,
        t::SET_BY_COMMAND_LINE,
        t::NEXT_PROMPT,
        t::NEXT_LAUNCH,
        t::CHEATS_GROUP,
        t::CHEATS_GROUP_HELP,
        t::ON,
        t::OFF,
        t::RESERVED,
    ];
    texts.extend(t::SETTINGS_TABS);
    texts.extend(t::SETTINGS_ROW_LABELS);
    texts.extend(PRESETS.iter().map(|preset| preset.label()));
    texts.extend(SETTING_GROUPS.iter().map(|(group, _)| *group));
    for row in CatalogueRow::all() {
        let key = row.key();
        texts.push(row.label());
        texts.extend(t::setting_label(key));
        texts.extend(t::setting_help(key));
        texts.extend(help_line(key));
    }
    texts
}

/// The words of `text`: its runs of letters, lowercased.
fn words(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_ascii_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// Whether `phrase` (a name, perhaps of several words) is in `text` as
/// whole words, ignoring case.
fn says(text: &str, phrase: &str) -> bool {
    let text = words(text);
    let phrase = words(phrase);
    !phrase.is_empty() && text.windows(phrase.len()).any(|w| w == phrase.as_slice())
}

/// The names the engine itself knows (published in the clean
/// specification): every place of the world's location table, the
/// dungeons, and the special items, potions and equipment.
fn engine_names() -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for entry in u5_runtime::published_world_location_entries() {
        names.extend(entry.proper_name.map(str::to_string));
    }
    for record in 0..8 {
        names.push(
            u5_runtime::DungeonScene::from_record(record)
                .unwrap()
                .name()
                .to_string(),
        );
    }
    for index in 0..16 {
        names.push(u5_runtime::z_stats::special_item_name(index).to_string());
    }
    for index in 0..8 {
        names.push(u5_runtime::z_stats::potion_inventory_name(index).to_string());
    }
    for index in 0..0x40 {
        let name = u5_runtime::equipment_name(index);
        if name != "Unknown equipment" {
            names.push(name.to_string());
        }
    }
    names.sort();
    names.dedup();
    names.retain(|name| !name.trim().is_empty());
    names
}

/// No label or help line names a place or item the engine knows.
#[test]
fn no_label_or_help_names_a_place_or_item() {
    let names = engine_names();
    assert!(names.len() > 20, "{names:?}");
    for text in shown_texts() {
        for name in &names {
            assert!(!says(text, name), "{text:?} names {name:?}");
        }
    }
}

/// FNV-1a over a lowercased word: the only form game words are kept in.
fn word_hash(word: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in word.bytes() {
        hash ^= u64::from(byte.to_ascii_lowercase());
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// No label or help line says a proper name of the game's text: a word the
/// game's talk, signs, looks and messages only ever write capitalised
/// (the names of people, places and things). The game's words are hashed
/// as they are read and only the hashes are kept. Runs when
/// `U5_TEST_ASSET_DIR` (or `U5_MOCKUP_PROFILE`) names a copy of the game.
#[test]
fn no_label_or_help_says_a_name_of_the_game_s_text() {
    let Some(dir) =
        configured_original_asset_dir().or_else(|| configured_test_dir("U5_MOCKUP_PROFILE"))
    else {
        eprintln!("skipped: set U5_TEST_ASSET_DIR or U5_MOCKUP_PROFILE");
        return;
    };
    let mut capital: HashSet<u64> = HashSet::new();
    let mut lower: HashSet<u64> = HashSet::new();
    let mut files = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_uppercase();
        // The world's words: talk, looks, signs, shops and stories. The
        // program's own strings (the overlays) are interface words, such as
        // a command's name at the start of a line; they count only as
        // evidence that a word is an ordinary one.
        let world_text = name.ends_with(".TLK")
            || [
                "LOOK2.DAT",
                "SIGNS.DAT",
                "SHOPPE.DAT",
                "MISCMSG.DAT",
                "ENDMSG.DAT",
                "STORY.DAT",
                "QUESTION.DAT",
            ]
            .contains(&name.as_str());
        if !world_text && !name.ends_with(".OVL") {
            continue;
        }
        files += usize::from(world_text);
        let bytes = fs::read(&path).unwrap();
        let mut word = String::new();
        for byte in bytes.iter().map(|b| b & 0x7f).chain([0]) {
            if byte.is_ascii_alphabetic() {
                word.push(byte as char);
                continue;
            }
            if word.len() >= 3 {
                let first = word.as_bytes()[0];
                let rest_lower = word[1..].bytes().all(|b| b.is_ascii_lowercase());
                if first.is_ascii_uppercase() && rest_lower {
                    if world_text {
                        capital.insert(word_hash(&word));
                    }
                } else if word.bytes().all(|b| b.is_ascii_lowercase()) {
                    lower.insert(word_hash(&word));
                }
            }
            word.clear();
        }
    }
    assert!(
        files > 5,
        "the game's world text files in {}",
        dir.display()
    );
    let names: HashSet<u64> = capital.difference(&lower).copied().collect();
    assert!(names.len() > 100, "the game's proper names");
    let mut hits = BTreeSet::new();
    for text in shown_texts() {
        for word in words(text) {
            if word.len() >= 3 && names.contains(&word_hash(&word)) {
                hits.insert((word, text));
            }
        }
    }
    let hits: Vec<_> = hits
        .into_iter()
        .filter(|(word, _)| !NOT_NAMES.contains(&word.as_str()))
        .collect();
    assert!(
        hits.is_empty(),
        "names of the game in the menu's text: {hits:?}"
    );
}

/// Words of the menu's text the game only ever writes capitalised but
/// that name nothing of its world: key names and words of the menu's own
/// that the game prints only at the start of a line.
const NOT_NAMES: &[&str] = &[];

// ---------------------------------------------------------------------------
// The lock and the Cheats group (3.2.7, D-M4.9).

/// A page with one ordinary row and the three kinds of cheat row.
fn rows_with_cheats(locked: bool) -> SettingsView {
    let ordinary = settings_rows_in(&ResolvedEnhancements::classic(), SHIPPED_PHASES)
        .into_iter()
        .find(|row| row.key == "display.grow_left")
        .unwrap();
    let cheat = |key: &str, role| SettingRow {
        key: key.to_string(),
        label: key.to_string(),
        group: "Cheats".into(),
        tags: Vec::new(),
        kind: PageRowKind::Switch,
        role,
        value: RowValue::Switch(false),
        default: RowValue::Switch(false),
        indent: false,
        help: None,
    };
    let rows = vec![
        ordinary,
        cheat("cheats.enabled", RowRole::AllowCheats),
        cheat("cheats.teleport", RowRole::Cheat),
        cheat("cheats.state_watch", RowRole::WatchState),
    ];
    SettingsView {
        page: SettingsRows {
            rows,
            presets: Vec::new(),
            preset: 0,
            locked,
            notes: Vec::new(),
        },
        notes: vec![note(ApplyAt::Frame); 4],
    }
}

/// Under the lock the Cheats group is absent; without it the group shows
/// collapsed, greyed until chapter 7 fills it; no cheat row is ever listed
/// on its own. The catalogue has no cheat rows, so in this build the group
/// is absent either way.
#[test]
fn the_lock_hides_the_cheats_group() {
    let locked = &settings_tabs(&rows_with_cheats(true), None)[3];
    assert!(!labels(locked).contains(&t::CHEATS_GROUP));
    assert_eq!(keys_of(locked), ["display.grow_left"]);
    let open = &settings_tabs(&rows_with_cheats(false), None)[3];
    assert_eq!(keys_of(open), ["display.grow_left"]);
    let group = open.last().unwrap();
    assert_eq!(group.label, t::CHEATS_GROUP);
    assert!(!group.enabled);
    assert_eq!(group.reason, Some(t::CHEATS_GROUP_HELP));
    let line = setting_line(group, 38);
    assert_eq!(line.cells[1].0, b'+');
    assert_eq!(
        line.cells[3].1,
        super::frame::Ink::Accent,
        "Cheats in brown"
    );

    let mut lock = ResolvedEnhancements::classic();
    lock.lock = LockState::Locked(LockReason::CliFlag);
    for resolved in [ResolvedEnhancements::classic(), lock] {
        let extras = &tabs(&resolved, every_phase())[3];
        assert!(!labels(extras).contains(&t::CHEATS_GROUP));
        assert!(!keys_of(extras).iter().any(|key| key.starts_with("cheats.")));
    }
}

// ---------------------------------------------------------------------------
// Leave the game (3.2.5).

/// "Leave the game" writes `AppExit` as the window's close button does,
/// and saves nothing: no setting is stored and the game folder is not
/// written.
#[test]
fn leaving_the_game_saves_nothing() {
    let dir = unique_scratch_dir("menu-quit");
    let mut scene = Scene::synthetic();
    scene.game_dir = dir.clone();
    let sink = Arc::new(RecordingSink::default());
    let mut game = Game::new(&menu_launch(&sink), &scene);
    let before = game.shell.visual().state.clone();
    game.tap(KeyCode::F12);
    assert!(game.page().is_some());
    game.down_to(t::QUIT_TO_DESKTOP);
    game.next.push(TimedKey {
        key: KeyCode::Enter,
        state: ButtonState::Pressed,
        repeat: false,
        owner: Owner::Game,
    });
    game.frames(1);
    let exits: Vec<AppExit> = game
        .world()
        .resource::<Events<AppExit>>()
        .iter_current_update_events()
        .cloned()
        .collect();
    assert_eq!(exits, [AppExit::Success]);
    let commands: Vec<MenuCommandEvent> = game
        .world()
        .resource::<Events<MenuCommandEvent>>()
        .iter_current_update_events()
        .copied()
        .collect();
    assert_eq!(commands, [MenuCommandEvent(MenuCommand::QuitToDesktop)]);
    assert!(sink.0.lock().unwrap().is_empty(), "no setting stored");
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 0, "nothing written");
    assert_eq!(game.shell.visual().state.turn, before.turn);
    let _ = fs::remove_dir_all(&dir);
}

/// The menu's own phases make the shell list their rows when a test asks
/// (the review shots do); the plugin itself lists this build's.
#[test]
fn the_plugin_lists_this_build_s_rows() {
    let sink = Arc::new(RecordingSink::default());
    let game = Game::new(&menu_launch(&sink), &Scene::synthetic());
    assert_eq!(game.world().resource::<MenuShell>().shipped, SHIPPED_PHASES);
    let mut shell = Shell::new(
        test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        &menu_launch(&sink),
        |app| {
            app.world_mut().resource_mut::<MenuShell>().shipped = with_menu_phases();
        },
    );
    shell.step(&[]);
    let snap = super::world_snapshot(shell.app.world());
    assert_eq!(keys_of(&snap.settings[0]), AUDIO_KEYS);
    assert!(game.snapshot().settings[0].is_empty());
}

// ---------------------------------------------------------------------------
// Pixels: the bar never meets a rule (render check, M4-07).

/// Every Settings tab with its real rows, at 320x200 and on a grow-left
/// canvas, with the cursor on every row: the list box's first and last
/// cell rows, and the preset box's, are margin rows of black paper only,
/// so neither the selection bar nor a glyph ever touches the rule above or
/// below its box; every opaque pixel is an EGA colour; the presets fit
/// their two lines.
#[test]
fn the_bar_never_meets_a_rule() {
    use super::frame::{PAPER, ega, ega_index};
    use super::geometry::{MenuPicture, PageShape, menu_picture, panel_rect};
    use super::render::{PRESET_LINES, PRESET_ROWS, preset_places, render_page};
    use crate::display::grow::Canvas;
    use crate::display::grow::column::ColumnView;

    // A font whose every glyph is solid, so any text shows.
    let font = u5_runtime::parse_ch_font(
        &vec![0xff; u5_runtime::CH_FONT_LEN],
        u5_runtime::IBM_CH_FILE,
    )
    .unwrap();
    let canvas =
        Canvas::for_window(2560, 1440, u5_runtime::enhancements::PixelAspect::Dos).unwrap();
    let mut words: Vec<&str> = PRESETS.iter().map(|p| p.label()).collect();
    words.push(t::CUSTOM);
    let places = preset_places(&words, 38);
    assert!(
        places.iter().all(|(line, _)| *line < PRESET_LINES),
        "{places:?}"
    );
    for ((_, col), word) in places.iter().zip(&words) {
        assert!(
            *col >= 2 && col + (word.len() as u32) < 37,
            "{word} with its blanks"
        );
    }

    let view = view_of(&ResolvedEnhancements::classic(), with_menu_phases());
    let snap = ShellSnapshot {
        settings: settings_tabs(
            &view,
            Some(GameValues {
                sound_on: true,
                buffer_on: true,
            }),
        ),
        ..ShellSnapshot::default()
    };
    for picture in [MenuPicture::FRAME, menu_picture(Some(&canvas))] {
        let rect = panel_rect(PageShape::Wide, &picture, ColumnView::default());
        for tab in SettingsTab::ALL {
            let rows = snap.settings[tab.index()].len();
            for cursor in 0..rows {
                let state = PageState {
                    page: Page::Settings(tab),
                    cursor,
                };
                let drawn = render_page(&font, &picture, ColumnView::default(), state, &snap);
                let img = &drawn.image;
                let black_row = |row: u32| {
                    (rect.col + 1..rect.right()).all(|col| {
                        (0..8).all(|dy| {
                            (0..8).all(|dx| img.at(col * 8 + dx, row * 8 + dy) == ega(PAPER))
                        })
                    })
                };
                let lower = rect.bottom() - 5;
                let (list_top, list_bottom) = match tab {
                    SettingsTab::Enhancements => {
                        let upper = rect.row + 1 + PRESET_ROWS;
                        assert!(
                            black_row(rect.row + 1),
                            "{tab:?} {cursor}: preset top margin"
                        );
                        assert!(
                            black_row(upper - 1),
                            "{tab:?} {cursor}: preset bottom margin"
                        );
                        (upper + 1, lower - 1)
                    }
                    _ => (rect.row + 1, lower - 1),
                };
                assert!(black_row(list_top), "{tab:?} {cursor}: list top margin");
                assert!(
                    black_row(list_bottom),
                    "{tab:?} {cursor}: list bottom margin"
                );
                for y in 0..img.h {
                    for x in 0..img.w {
                        let pixel = img.at(x, y);
                        assert!(
                            pixel[3] != 0xff || ega_index(pixel).is_some(),
                            "{tab:?} {cursor}: ({x}, {y}) is no EGA colour"
                        );
                    }
                }
            }
        }
    }
}

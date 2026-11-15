//! The M4-13 render check: the Game tab and the unsaved-progress question
//! in the real shell over real engine frames, at 4:3 and on the grow-left
//! canvas, read back as [`super::review_tests`] does. Review output only,
//! never committed, and `#[ignore]`d:
//!
//! ```text
//! U5_MOCKUP_PROFILE=<copy> U5_MENU_SHOTS=<folder> \
//!   cargo test -p u5-bevy --lib menu::game_tab_review_tests -- --ignored
//! ```
//!
//! The scene is the c04f forest road west of Britain at 11:50, walked five
//! steps, so the question has turns to warn of. The Load page lists one
//! stand-in slot (the host's store is not in this shell) so its question
//! can be drawn too. After the questions, Sound is chosen: the menu sweeps
//! out and the game prints its own line.

use std::fs;
use std::sync::Arc;

use bevy::input::ButtonState;
use bevy::window::WindowResolution;
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{GameClock, PlayState, PlayTarget, load_play_options_from_save};

use super::host::SlotInfo;
use super::pages::{Dialog, Leave, Page, SettingsTab, SlotEntry, page_rows};
use super::render::MenuView;
use super::review_tests::{CASES, composed, launch, save_png, scaled};
use super::slots::SlotListing;
use super::text as t;
use super::{MenuShell, MenuState, world_snapshot};
use crate::enh::equivalence::{Owner, Scene, Shell, TimedKey, Timeline};
use bevy::prelude::*;

fn tap(shell: &mut Shell, key: KeyCode) {
    for state in [ButtonState::Pressed, ButtonState::Released] {
        shell.step(&[TimedKey {
            key,
            state,
            repeat: false,
            owner: Owner::Game,
        }]);
    }
}

fn current(shell: &Shell) -> super::pages::PageState {
    shell
        .app
        .world()
        .resource::<MenuState>()
        .model()
        .unwrap()
        .current()
}

/// Down (or up) to the row labelled `label` on the page showing.
fn to(shell: &mut Shell, label: &str) {
    let page = current(shell).page;
    let rows = page_rows(page, &world_snapshot(shell.app.world()));
    let at = rows
        .iter()
        .position(|row| row.label == label)
        .unwrap_or_else(|| panic!("no row {label} on {page:?}"));
    for _ in 0..rows.len() {
        let cursor = current(shell).cursor;
        if cursor == at {
            break;
        }
        tap(
            shell,
            if cursor < at {
                KeyCode::ArrowDown
            } else {
                KeyCode::ArrowUp
            },
        );
    }
    assert_eq!(current(shell).cursor, at, "{label}");
}

fn settle(shell: &mut Shell) {
    while shell
        .app
        .world()
        .resource::<MenuView>()
        .sweep
        .step
        .is_some()
    {
        shell.step(&[]);
    }
}

#[test]
#[ignore = "review output: needs U5_MOCKUP_PROFILE and U5_MENU_SHOTS"]
fn game_tab_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_MENU_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_MENU_SHOTS");
        return;
    };
    fs::create_dir_all(&out).unwrap();
    let game = unique_scratch_dir("menu-game-tab-shots");
    for entry in fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    for case in [&CASES[0], &CASES[1], &CASES[2], &CASES[3]] {
        let mut options = load_play_options_from_save(&game).unwrap();
        options.target = PlayTarget::from_key("BRITANNIA").unwrap();
        options.saved_active_objects = None;
        options.start = Some((74, 111));
        options.clock = GameClock::new(11, 50).unwrap();
        let state = PlayState::load_scene(&game, options).unwrap();
        let walk = [
            KeyCode::ArrowUp,
            KeyCode::ArrowUp,
            KeyCode::ArrowRight,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
        ];
        let mut timeline = Timeline::new(10 + walk.len() * 30 + 80);
        for (index, key) in walk.iter().enumerate() {
            timeline.game_tap(10 + index * 30, *key);
        }
        if case.column {
            let at = 10 + walk.len() * 30 + 10;
            for (frame, key, state) in [
                (at, KeyCode::ControlLeft, ButtonState::Pressed),
                (at, KeyCode::KeyP, ButtonState::Pressed),
                (at + 1, KeyCode::KeyP, ButtonState::Released),
                (at + 1, KeyCode::ControlLeft, ButtonState::Released),
            ] {
                timeline.event(frame, key, state, false, Owner::Feature);
            }
        }
        let mut shell = Shell::new(state, &scene, &launch(case.grow), |app| {
            app.insert_resource(MenuShell {
                intro_shell: true,
                slots: true,
                shipped: u5_runtime::enhancements::SHIPPED_PHASES,
            });
            app.insert_resource(SlotListing {
                slots: Arc::new(vec![SlotEntry {
                    info: SlotInfo {
                        name: "Second try".into(),
                        avatar: "Avatar".into(),
                        place: "Britannia".into(),
                        clock: [139, 4, 2, 11, 40],
                        turn: 120,
                        ..SlotInfo::default()
                    },
                    thumb: None,
                }]),
            });
        });
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(case.window.0 as f32, case.window.1 as f32),
            ..default()
        });
        for index in 0..timeline.len() {
            shell.step(timeline.frame(index));
        }
        let label = case.label;
        let shot = |shell: &mut Shell, name: &str| {
            let (w, h, rgba) = composed(shell);
            save_png(&out.join(format!("{label}-{name}.png")), w, h, &rgba);
            save_png(
                &out.join(format!("{label}-{name}-4x.png")),
                w * 4,
                h * 4,
                &scaled(w, h, &rgba, 4),
            );
        };

        tap(&mut shell, KeyCode::F12);
        settle(&mut shell);
        shot(&mut shell, "top");

        // The three questions.
        for (row, what) in [
            (t::QUIT_TO_TITLE, Leave::Title),
            (t::QUIT_TO_DESKTOP, Leave::Desktop),
        ] {
            to(&mut shell, row);
            tap(&mut shell, KeyCode::Enter);
            assert_eq!(current(&shell).page, Page::Dialog(Dialog::Unsaved(what)));
            shell.step(&[]);
            shot(&mut shell, &format!("unsaved-{what:?}").to_lowercase());
            tap(&mut shell, KeyCode::ArrowLeft);
            shot(&mut shell, &format!("unsaved-{what:?}-yes").to_lowercase());
            tap(&mut shell, KeyCode::Escape);
        }
        to(&mut shell, t::LOAD);
        tap(&mut shell, KeyCode::Enter);
        assert_eq!(current(&shell).page, Page::Load);
        tap(&mut shell, KeyCode::Enter);
        assert_eq!(
            current(&shell).page,
            Page::Dialog(Dialog::Unsaved(Leave::Load(0)))
        );
        shell.step(&[]);
        shot(&mut shell, "unsaved-load");
        tap(&mut shell, KeyCode::Escape);
        tap(&mut shell, KeyCode::Escape);

        // The Game tab, each row selected.
        to(&mut shell, t::SETTINGS);
        tap(&mut shell, KeyCode::Enter);
        to(&mut shell, t::MUTE);
        tap(&mut shell, KeyCode::ArrowRight);
        to(&mut shell, "Fullscreen");
        tap(&mut shell, KeyCode::ArrowRight);
        assert_eq!(current(&shell).page, Page::Settings(SettingsTab::Game));
        shell.step(&[]);
        shot(&mut shell, "game-sound");
        for (row, name) in [
            (t::BUFFER, "game-buffer"),
            (t::START_SOUND_OFF, "game-start-sound"),
            (t::START_BUFFER_ON, "game-start-buffer"),
            (t::CONFIRM_UNSAVED, "game-confirm"),
            (t::PACING, "game-pacing"),
        ] {
            to(&mut shell, row);
            shell.step(&[]);
            shot(&mut shell, name);
        }

        // Sound: the menu closes and the game answers for itself.
        to(&mut shell, t::SOUND);
        tap(&mut shell, KeyCode::Enter);
        for _ in 0..30 {
            shell.step(&[]);
        }
        assert!(!shell.visual().state.music_enabled, "{label}: Sound Off");
        shot(&mut shell, "after-sound");
        tap(&mut shell, KeyCode::F12);
        settle(&mut shell);
        to(&mut shell, t::SETTINGS);
        tap(&mut shell, KeyCode::Enter);
        to(&mut shell, t::MUTE);
        tap(&mut shell, KeyCode::ArrowRight);
        to(&mut shell, "Fullscreen");
        tap(&mut shell, KeyCode::ArrowRight);
        shell.step(&[]);
        shot(&mut shell, "game-sound-off");
        eprintln!("{label}: written");
    }
}

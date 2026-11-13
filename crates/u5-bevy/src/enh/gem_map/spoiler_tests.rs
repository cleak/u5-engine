//! The gem map's fresh-save spoiler test in the shell (Phase B design
//! 1.4.7 and 10.6; build plan M3-18), on the pattern of the display
//! chapter's (`display/spoiler_tests.rs`).
//!
//! From the factory seed save after character creation, with the gem map
//! on and every `[gem_map]` key at its most revealing value (labels, the
//! land beside a town, the stack and the wrap rows; the sweep off so the
//! picture is the whole map at once), the player uses a gem where the game
//! starts, walks out and about using more, in a 16:9 and a 4:3 window. On
//! every frame the map is up:
//!
//! - the view the window holds passes the whole no-spoiler audit of
//!   `u5_automap::gem::spoiler` against the record the shell keeps;
//! - the picture on screen is exactly that view drawn from that record, so
//!   the audit judged what the player sees;
//! - at the first gem, used before any step, no land beside a town is
//!   drawn: the party has seen none of it from outside;
//!
//! and the game's trajectory, original frame and key queue are Classic's.
//!
//! Runs only when `U5_TEST_ASSET_DIR` names a writable copy of the game.

use std::fs;
use std::path::PathBuf;

use bevy::window::WindowResolution;
use u5_automap::gem::{GemImage, GemShown, MapArt, TownPart, audit_gem_view, render_gem_map};
use u5_runtime::enhancements::{CliSetting, HostCapabilities, LockState, ResolveInput, resolve};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, copy_asset_writable, unique_scratch_dir,
};
use u5_runtime::{ChargenStats, PlayState, commit_chargen_save, load_play_options_from_save};

use super::*;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline, check_shell_equivalence};

/// The gem map on, every key at its most revealing value.
fn gem_everything_on() -> VisualLaunchConfig {
    let on = |key: &str| CliSetting::Enable(key.into());
    let settings = [
        on("gem_map.enhanced"),
        on("gem_map.labels"),
        on("gem_map.town_surroundings"),
        on("gem_map.dungeon_stack"),
        on("gem_map.wrap_rows"),
        CliSetting::Disable("gem_map.sweep".into()),
        CliSetting::Set {
            key: "gem_map.map_brightness".into(),
            value: "100".into(),
        },
    ];
    let enhancements = resolve(ResolveInput {
        stored: None,
        cli: &settings,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(enhancements.notes.is_empty(), "{:?}", enhancements.notes);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

/// A gem where the game starts, then out of the door and about with a gem
/// every few steps; each view closed by the next key, as in 1.0.1.
fn timeline() -> Timeline {
    let keys = [
        KeyCode::KeyV,
        KeyCode::Space,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::ArrowRight,
        KeyCode::ArrowDown,
        KeyCode::KeyV,
        KeyCode::Space,
        KeyCode::ArrowDown,
        KeyCode::ArrowLeft,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::KeyV,
        KeyCode::Space,
        KeyCode::ArrowRight,
        KeyCode::ArrowUp,
        KeyCode::ArrowDown,
        KeyCode::ArrowDown,
        KeyCode::KeyV,
        KeyCode::ArrowDown,
    ];
    let mut timeline = Timeline::new(5 + keys.len() * 20 + 40);
    for (index, key) in keys.iter().enumerate() {
        timeline.game_tap(5 + index * 20, *key);
    }
    timeline
}

fn node_picture(shell: &mut Shell) -> Option<Vec<u8>> {
    let world = shell.app.world_mut();
    let mut query = world.query_filtered::<&ImageNode, With<GemMapNode>>();
    let handle = query.iter(world).next()?.image.clone();
    world
        .resource::<Assets<Image>>()
        .get(&handle)
        .and_then(|image| image.data.clone())
}

#[test]
fn a_fresh_save_with_the_gem_map_on_reveals_nothing() {
    let Some(dir) = configured_original_asset_dir() else {
        return;
    };
    if !dir.join("INIT.GAM").exists() {
        return;
    }
    let game: PathBuf = unique_scratch_dir("gem-spoiler");
    for entry in fs::read_dir(&dir).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    commit_chargen_save(
        &game,
        b"Tester",
        true,
        ChargenStats {
            strength: 15,
            dexterity: 15,
            intelligence: 15,
        },
    )
    .unwrap();
    let options = load_play_options_from_save(&game).unwrap();
    let mut start = PlayState::load_scene(&game, options).unwrap();
    start.gems = 6;
    start.prng_state = 0x1234;
    let scene = Scene::from_game_dir(&game).unwrap();
    let art =
        MapArt::build(&scene.atlas).with_fonts(scene.text_font.clone(), scene.rune_font.clone());
    let launch = gem_everything_on();
    let mut t = launch.enhancements.gem_map();
    t.enhanced = true;
    let timeline = timeline();

    for window in [(2560.0, 1440.0), (1600.0, 1200.0)] {
        let spawn = move |app: &mut App| {
            app.world_mut().spawn(Window {
                resolution: WindowResolution::new(window.0, window.1),
                ..default()
            });
        };
        let mut shell = Shell::new(start.clone(), &scene, &launch, |_| {});
        spawn(&mut shell.app);
        let (mut audited, mut first) = (0, true);
        for frame in 0..timeline.len() {
            shell.step(timeline.frame(frame));
            let Some(view) = shell.app.world().resource::<GemShow>().view().cloned() else {
                continue;
            };
            let state = shell.visual().state.clone();
            let know = shell.app.world().resource::<GemMap>().knowledge().clone();
            let report = audit_gem_view(&state, &know, &view, &t, &art);
            assert!(
                report.violations.is_empty(),
                "{window:?} frame {frame}:\n{}",
                report.violations.join("\n")
            );
            // The picture on screen is that view, from that record.
            let mut expected = GemImage::default();
            render_gem_map(&state, &know, &art, &t, &view, &mut expected);
            assert_eq!(
                node_picture(&mut shell).as_deref(),
                Some(&expected.rgba[..]),
                "{window:?} frame {frame}: the picture is not the audited view"
            );
            if first {
                first = false;
                if let GemShown::Town(shown) = &view.shown {
                    assert!(
                        shown
                            .cells()
                            .all(|(_, _, part, tile)| part != TownPart::Side || tile.is_none()),
                        "the first gem shows land beside the town"
                    );
                }
            }
            audited += 1;
        }
        assert!(audited > 0, "{window:?}: the gem map never showed");
        assert!(
            shell.visual().state.gems < start.gems,
            "{window:?}: no gem was used"
        );
        let mismatches = check_shell_equivalence(&start, &scene, &timeline, &launch, spawn);
        assert_eq!(mismatches, [], "{window:?}");
    }
    let _ = fs::remove_dir_all(&game);
}

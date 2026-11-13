//! The M3 review shots (build plan M3-18): every approved mockup scene of
//! `mockups/gem/final`, rendered by the real `--visual` shell with the
//! gem map on, for the side-by-side the user reviews. Review only: skipped
//! unless `U5_MOCKUP_PROFILE` names a game copy and `U5_GEM_SHOTS` a
//! folder; nothing is committed.
//!
//! Each scene is the mockup's own place and time; a gem is used with V
//! and the map's picture is written once the sweep has finished (and,
//! for the open GIF, on every frame of the sweep). The record is the
//! mockups' demo record, read from the review-only files the variables
//! `U5_GEM_KNOWLEDGE` (mid game), `U5_GEM_KNOWLEDGE_EARLY`,
//! `U5_GEM_KNOWLEDGE_DECEIT` and `U5_GEM_DUNGEON_MEMORY` name, in the
//! format of `u5-automap/tests/gem_real_assets.rs`; it is seeded into the
//! shell's record before the gem is used.

use bevy::window::WindowResolution;
use u5_automap::gem::{
    DungeonMark, EdgeSide, ExitRun, GateCell, KnowEvent, LinkDir, MarkKind, SIDE_WINDOW_HEIGHT,
    SIDE_WINDOW_WIDTH, SignCell, TownLink, Trade,
};
use u5_runtime::enhancements::{CliSetting, HostCapabilities, LockState, ResolveInput, resolve};
use u5_runtime::test_fixtures::{configured_test_dir, copy_asset_writable, unique_scratch_dir};
use u5_runtime::{
    Direction, DungeonScene, GameClock, PlayOptions, PlayState, PlayTarget, Scene as GameScene,
    WorldPlane,
};

use super::*;
use crate::enh::VisualLaunchConfig;
use crate::enh::equivalence::{Scene, Shell, Timeline};

const W169: (f32, f32) = (2560.0, 1440.0);
const W219: (f32, f32) = (3440.0, 1440.0);
const W43: (f32, f32) = (1600.0, 1200.0);

/// The review-only record format (see `gem_real_assets.rs`).
fn demo_events(var: &str) -> Vec<KnowEvent> {
    let Some(path) = configured_test_dir(var) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        let n = |i: usize| -> i64 { words[i].parse().unwrap() };
        match words.first().copied() {
            Some("place") => out.push(KnowEvent::PlaceEntered { scene: n(1) as u8 }),
            Some("shrine") => out.push(KnowEvent::ShrineEntered { virtue: n(1) as u8 }),
            Some("codex") => out.push(KnowEvent::CodexEntered),
            Some("entrance") => out.push(KnowEvent::EntranceNamed { record: n(1) as u8 }),
            Some("gate") => out.push(KnowEvent::GateUsed(GateCell {
                plane: n(1) as u8,
                x: n(2) as u8,
                y: n(3) as u8,
            })),
            Some("sign") => out.push(KnowEvent::SignRead(SignCell {
                scene: n(1) as u8,
                floor: n(2) as i8,
                x: n(3) as u8,
                y: n(4) as u8,
            })),
            Some("trade") => out.push(KnowEvent::ShopOpened {
                scene: n(1) as u8,
                trade: Trade::from_code(n(2) as u8).unwrap(),
                keeper: None,
            }),
            Some("keeper") => out.push(KnowEvent::ShopOpened {
                scene: n(1) as u8,
                trade: Trade::from_code(n(2) as u8).unwrap(),
                keeper: Some((n(3) as i8, n(4) as u8, n(5) as u8)),
            }),
            Some("link") => out.push(KnowEvent::TownLinkUsed(TownLink {
                scene: n(1) as u8,
                floor: n(2) as i8,
                x: n(3) as u8,
                y: n(4) as u8,
                dir: if words[5] == "up" {
                    LinkDir::Up
                } else {
                    LinkDir::Down
                },
            })),
            Some("exit") => out.push(KnowEvent::TownExit(ExitRun {
                scene: n(1) as u8,
                side: match words[2] {
                    "N" => EdgeSide::North,
                    "E" => EdgeSide::East,
                    "S" => EdgeSide::South,
                    _ => EdgeSide::West,
                },
                at: n(3) as u8,
            })),
            Some("sides") => out.extend(
                (0..u16::from(SIDE_WINDOW_WIDTH) * u16::from(SIDE_WINDOW_HEIGHT)).map(|index| {
                    KnowEvent::SideCellSeen {
                        scene: n(1) as u8,
                        index,
                    }
                }),
            ),
            Some("cell") if words.len() == 5 => {
                let mut rows = [0; 8];
                rows[n(4) as usize] = 1 << n(3);
                out.push(KnowEvent::DungeonCellsSeen {
                    record: n(1) as u8,
                    level: n(2) as u8,
                    rows,
                });
            }
            Some("mark") => out.push(KnowEvent::DungeonMarked(DungeonMark {
                record: n(1) as u8,
                level: n(2) as u8,
                x: n(3) as u8,
                y: n(4) as u8,
                kind: [
                    MarkKind::Ladder,
                    MarkKind::Fountain,
                    MarkKind::Chest,
                    MarkKind::Trap,
                    MarkKind::SecretDoor,
                ][n(5) as usize],
            })),
            _ => {}
        }
    }
    out
}

/// `U5_GEM_DUNGEON_MEMORY`'s `cell L X Y` and `trap L X Y` lines, for
/// Despise (record 1).
fn despise_memory() -> Vec<KnowEvent> {
    let Some(path) = configured_test_dir("U5_GEM_DUNGEON_MEMORY") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in std::fs::read_to_string(path).unwrap().lines() {
        let words: Vec<_> = line.split_whitespace().collect();
        let [kind, l, x, y] = words[..] else { continue };
        let (l, x, y): (u8, u8, u8) = (l.parse().unwrap(), x.parse().unwrap(), y.parse().unwrap());
        match kind {
            "cell" => {
                let mut rows = [0; 8];
                rows[usize::from(y)] = 1 << x;
                out.push(KnowEvent::DungeonCellsSeen {
                    record: 1,
                    level: l,
                    rows,
                });
            }
            "trap" => out.push(KnowEvent::DungeonMarked(DungeonMark {
                record: 1,
                level: l,
                x,
                y,
                kind: MarkKind::Trap,
            })),
            _ => {}
        }
    }
    out
}

fn launch(more: &[CliSetting]) -> VisualLaunchConfig {
    let mut cli = vec![CliSetting::Enable("gem_map.enhanced".into())];
    cli.extend(more.iter().cloned());
    let enhancements = resolve(ResolveInput {
        stored: None,
        cli: &cli,
        lock: LockState::Off,
        caps: HostCapabilities::ALL,
    });
    assert!(enhancements.notes.is_empty(), "{:?}", enhancements.notes);
    VisualLaunchConfig {
        enhancements,
        ..VisualLaunchConfig::classic()
    }
}

fn load(game: &std::path::Path, options: PlayOptions) -> PlayState {
    let mut state = PlayState::load_scene(game, options).unwrap();
    state.gems = 3;
    state
}

fn picture(shell: &Shell) -> Option<image::RgbaImage> {
    let handle = shell.app.world().resource::<GemShow>().image()?.clone();
    let image = shell.app.world().resource::<Assets<Image>>().get(&handle)?;
    let size = image.size();
    image::RgbaImage::from_raw(size.x, size.y, image.data.clone()?)
}

#[test]
fn gem_shell_review_shots() {
    let (Some(profile), Some(out)) = (
        configured_test_dir("U5_MOCKUP_PROFILE"),
        configured_test_dir("U5_GEM_SHOTS"),
    ) else {
        eprintln!("skipped: set U5_MOCKUP_PROFILE and U5_GEM_SHOTS");
        return;
    };
    let game = unique_scratch_dir("gem-review-shots");
    for entry in std::fs::read_dir(&profile).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_file() {
            copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
        }
    }
    let scene = Scene::from_game_dir(&game).unwrap();
    let mid = demo_events("U5_GEM_KNOWLEDGE");
    let early = demo_events("U5_GEM_KNOWLEDGE_EARLY");
    let deceit_know = demo_events("U5_GEM_KNOWLEDGE_DECEIT");
    let mut despise_know = mid.clone();
    despise_know.extend(despise_memory());

    // The mockups' S1: the forest road west of Britain at 12:06.
    let outdoors = || {
        load(
            &game,
            PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((76, 108)),
                clock: GameClock::new(12, 6).unwrap(),
                ..PlayOptions::default()
            },
        )
    };
    // S2: Britain, the party at (15, 17), 10:00.
    let britain = || {
        load(
            &game,
            PlayOptions {
                target: PlayTarget::Town(GameScene::from_key("TOWNE:1").unwrap()),
                floor: 0,
                start: Some((15, 17)),
                clock: GameClock::new(10, 0).unwrap(),
                ..PlayOptions::default()
            },
        )
    };
    // Despise level 3 at (6, 1) facing north; Deceit level 2 at (1, 6).
    let dungeon = |record: u8, level: i8, at: (usize, usize)| {
        let mut state = load(
            &game,
            PlayOptions {
                target: PlayTarget::Dungeon(DungeonScene::from_record(record).unwrap()),
                floor: level,
                start: Some(at),
                clock: GameClock::new(12, 0).unwrap(),
                ..PlayOptions::default()
            },
        );
        state.player.facing = Direction::North;
        state
    };
    let pct = |p: &str| {
        vec![CliSetting::Set {
            key: "gem_map.map_brightness".into(),
            value: p.into(),
        }]
    };
    type Shot<'a> = (
        &'a str,
        PlayState,
        &'a [KnowEvent],
        (f32, f32),
        Vec<CliSetting>,
    );
    let shots: Vec<Shot> = vec![
        ("O1-169", outdoors(), &mid, W169, vec![]),
        ("O1e-169", outdoors(), &early, W169, vec![]),
        ("O2-219", outdoors(), &mid, W219, vec![]),
        ("O3-43", outdoors(), &mid, W43, vec![]),
        ("BR-100", outdoors(), &mid, W169, pct("100")),
        ("BR-085", outdoors(), &mid, W169, pct("85")),
        ("BR-070", outdoors(), &mid, W169, pct("70")),
        ("T1-169", britain(), &mid, W169, vec![]),
        ("T1e-169", britain(), &early, W169, vec![]),
        ("T3-43", britain(), &mid, W43, vec![]),
        ("D1-169", dungeon(1, 2, (6, 1)), &despise_know, W169, vec![]),
        (
            "D1n-169",
            dungeon(1, 2, (6, 1)),
            &despise_know,
            W169,
            vec![CliSetting::Disable("gem_map.wrap_rows".into())],
        ),
        ("D2-219", dungeon(1, 2, (6, 1)), &despise_know, W219, vec![]),
        ("D3-43", dungeon(1, 2, (6, 1)), &despise_know, W43, vec![]),
        ("D1f-169", dungeon(0, 1, (1, 6)), &deceit_know, W169, vec![]),
    ];
    for (name, state, events, window, more) in shots {
        let mut shell = Shell::new(state, &scene, &launch(&more), |_| {});
        shell.app.world_mut().spawn(Window {
            resolution: WindowResolution::new(window.0, window.1),
            ..default()
        });
        let mut timeline = Timeline::new(60);
        timeline.game_tap(8, KeyCode::KeyV);
        let gif = name == "O1-169";
        for frame in 0..timeline.len() {
            shell.step(timeline.frame(frame));
            if frame == 2 {
                shell
                    .app
                    .world_mut()
                    .resource_mut::<super::GemMap>()
                    .learn(events);
            }
            if gif
                && (8..=40).contains(&frame)
                && let Some(image) = picture(&shell)
            {
                image
                    .save(out.join(format!("engine-shell-open-169-f{frame:02}.png")))
                    .unwrap();
            }
        }
        let image = picture(&shell).unwrap_or_else(|| panic!("{name}: the map shows"));
        image
            .save(out.join(format!("engine-shell-{name}.png")))
            .unwrap();
        assert!(
            u5_automap::gem::gem_view_open(&shell.visual().state),
            "{name}"
        );
    }
    let _ = std::fs::remove_dir_all(game);
}

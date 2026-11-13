//! The no-spoiler suite of design 10.6 (build plan M3-18): the audit of
//! [`super::spoiler`] over every rendered gem map of a corpus, at the
//! three canvases, under both glyph policies, with the dungeon stack and
//! as one paged panel.
//!
//! - **Synthetic**, in every `cargo test`: random outdoor views on both
//!   planes (place markers, gates, signs and objects strewn about the
//!   party), random towns (signs, shops, ladders, stairs, walls and gates,
//!   with a random world map beside them) and random dungeons (ladders,
//!   pits, hidden pits and doors, fountains and chests on every level),
//!   each under random records.
//! - **Real**, when `U5_TEST_ASSET_DIR` names a writable copy of the game:
//!   the fresh save after character creation and a scripted walk from it
//!   (the record kept as play keeps it, a gem used along the way), the
//!   first level of each dungeon, the Underworld before and after its
//!   visit, and random records over places all over Britannia.
//!
//! Every word a real view draws is checked against `SIGNS.DAT` and the
//! four conversation files at test time: no label equals, contains or
//! hashes to any sign or conversation text. Nothing read is kept or
//! printed; a failure names only the label.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use u5_runtime::enhancements::{GemMapToggles, GlyphPolicy, fnv1a64};
use u5_runtime::test_fixtures::{
    configured_original_asset_dir, copy_asset_writable, dungeon_state, open_dungeon_record,
    open_world_grid, synthetic_tile_atlas, test_state, unique_scratch_dir, world_state,
};
use u5_runtime::{
    ActiveObject, Area, ChargenStats, Direction, DungeonScene, GameClock, PlayOptions, PlayState,
    PlayTarget, SCENE_OVERWORLD, Scene, TileGraphicsDepth, ViewOverlay, ViewOverlayKind,
    ViewOverlayMode, WorldPlane, commit_chargen_save, dungeon_cell_index, handle_play_key_input,
    load_ibm_ch_font, load_play_options_from_save, load_runes_ch_font, load_sign_records,
    load_tile_atlas, parse_tlk, world_cell_index,
};

use super::*;
use crate::StaticMaps;
use crate::gem::knowledge::{
    DungeonMark, EdgeSide, ExitRun, GateCell, LinkDir, MarkKind, SignCell, TownLink,
};
use crate::gem::seen::side_window;
use crate::memory::DUNGEON_LEVEL_COUNT;

const CANVASES: [GemCanvas; 3] = [
    GemCanvas { w: 320, h: 200 },
    GemCanvas { w: 512, h: 240 },
    GemCanvas { w: 688, h: 240 },
];

/// A small deterministic generator (no crate dependency).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as u32
    }

    fn below(&mut self, n: u32) -> u32 {
        self.next() % n.max(1)
    }

    fn chance(&mut self, percent: u32) -> bool {
        self.below(100) < percent
    }

    fn pick<T: Copy>(&mut self, items: &[T]) -> T {
        items[self.below(items.len() as u32) as usize]
    }
}

fn toggles(glyphs: GlyphPolicy, stack: bool) -> GemMapToggles {
    let mut t = GemMapToggles {
        enhanced: true,
        ..GemMapToggles::default()
    };
    t.opts.glyphs = glyphs;
    t.opts.dungeon_stack = stack;
    t.opts.town_surroundings = true;
    t
}

fn gem_overlay(kind: ViewOverlayKind) -> Option<ViewOverlay> {
    Some(ViewOverlay {
        title: String::new(),
        text_map: String::new(),
        kind,
        mode: ViewOverlayMode::GemView,
    })
}

/// What the audits of a corpus saw, for the non-vacuity checks.
#[derive(Default)]
struct Tally {
    views: usize,
    questions: usize,
    known: usize,
    texts: BTreeSet<String>,
}

/// Opens `state`'s gem view at every canvas, under both glyph policies,
/// with the stack and as a lone panel paged both ways, and audits each.
fn audit_everywhere(
    name: &str,
    state: &PlayState,
    know: &GemKnowledge,
    world: Option<&[u8]>,
    art: &MapArt,
    tally: &mut Tally,
) {
    // The glyph policy and the stack matter only in a dungeon.
    let dungeon = matches!(state.area, Area::Dungeon { .. });
    let policies: &[GlyphPolicy] = if dungeon {
        &[GlyphPolicy::Perceived, GlyphPolicy::GemFaithful]
    } else {
        &[GlyphPolicy::Perceived]
    };
    let stacks: &[bool] = if dungeon { &[true, false] } else { &[true] };
    for canvas in CANVASES {
        for &glyphs in policies {
            for &stack in stacks {
                let t = toggles(glyphs, stack);
                let mut view = GemViewState::open_all(
                    state,
                    canvas,
                    100,
                    TownInputs {
                        know: Some(know),
                        world,
                        surroundings: true,
                    },
                    DungeonInputs {
                        know: Some(know),
                        glyphs,
                    },
                );
                let mut pages = vec![view.clone()];
                if view.is_one_panel_dungeon(stack) {
                    for up in [true, false] {
                        let mut paged = view.clone();
                        while paged.turn_page(up, stack) {
                            pages.push(paged.clone());
                        }
                    }
                }
                view.page = 0;
                for view in pages {
                    let report = audit_gem_view(state, know, &view, &t, art);
                    assert!(
                        report.violations.is_empty(),
                        "{name} at {}x{} {glyphs:?} stack {stack} page {}:\n{}",
                        canvas.w,
                        canvas.h,
                        view.page,
                        report.violations.join("\n")
                    );
                    tally.views += 1;
                    tally.questions += report.questions;
                    tally.known += report.known;
                    tally.texts.extend(report.texts);
                }
                let flips = flip_changes(&view, know);
                assert!(flips.is_empty(), "{name}: {}", flips.join("\n"));
            }
        }
    }
}

/// A random record: places entered, shrines, the Codex, entrances named,
/// gates used and signs read near `around`, trades, links, exits, dungeon
/// cells seen and marks, town side cells seen, the Underworld visit.
fn random_record(rng: &mut Lcg, around: (u8, u8), record: u8) -> GemKnowledge {
    let mut know = GemKnowledge::new();
    let near = |rng: &mut Lcg, v: u8| v.wrapping_add(rng.below(40) as u8).wrapping_sub(20);
    for _ in 0..rng.below(40) {
        let event = match rng.below(14) {
            0 => KnowEvent::PlaceEntered {
                scene: rng.below(48) as u8,
            },
            1 => KnowEvent::ShrineEntered {
                virtue: rng.below(8) as u8,
            },
            2 => KnowEvent::CodexEntered,
            3 => KnowEvent::EntranceNamed {
                record: rng.below(8) as u8,
            },
            4 => KnowEvent::GateUsed(GateCell {
                plane: rng.below(2) as u8,
                x: near(rng, around.0),
                y: near(rng, around.1),
            }),
            5 => KnowEvent::SignRead(SignCell {
                scene: if rng.chance(50) {
                    SCENE_OVERWORLD
                } else {
                    rng.below(33) as u8
                },
                floor: 0,
                x: near(rng, around.0),
                y: near(rng, around.1),
            }),
            6 => KnowEvent::ShopOpened {
                scene: rng.below(33) as u8,
                trade: rng.pick(&TRADES),
                keeper: rng
                    .chance(50)
                    .then(|| (0, rng.below(32) as u8, rng.below(32) as u8)),
            },
            7 => KnowEvent::TownLinkUsed(TownLink {
                scene: rng.below(33) as u8,
                floor: 0,
                x: rng.below(32) as u8,
                y: rng.below(32) as u8,
                dir: if rng.chance(50) {
                    LinkDir::Up
                } else {
                    LinkDir::Down
                },
            }),
            8 => KnowEvent::TownExit(ExitRun {
                scene: rng.below(33) as u8,
                side: rng.pick(&[
                    EdgeSide::North,
                    EdgeSide::East,
                    EdgeSide::South,
                    EdgeSide::West,
                ]),
                at: rng.below(32) as u8,
            }),
            9 | 10 => KnowEvent::DungeonCellsSeen {
                record,
                level: rng.below(u32::from(DUNGEON_LEVEL_COUNT)) as u8,
                rows: std::array::from_fn(|_| rng.next() as u8),
            },
            11 => KnowEvent::DungeonMarked(DungeonMark {
                record,
                level: rng.below(u32::from(DUNGEON_LEVEL_COUNT)) as u8,
                x: rng.below(8) as u8,
                y: rng.below(8) as u8,
                kind: rng.pick(&[
                    MarkKind::Ladder,
                    MarkKind::Fountain,
                    MarkKind::Chest,
                    MarkKind::Trap,
                    MarkKind::SecretDoor,
                ]),
            }),
            12 => KnowEvent::SideCellSeen {
                scene: rng.below(33) as u8,
                index: rng.below(48 * 48) as u16,
            },
            _ => KnowEvent::UnderworldVisited,
        };
        know.apply(&event);
    }
    know
}

/// A synthetic font with ink on every glyph.
fn inked_font() -> u5_runtime::FixedCellFont {
    let bytes = (0..u5_runtime::CH_FONT_LEN)
        .map(|i| ((i / 8) as u8).wrapping_mul(31) ^ (i % 8) as u8 | 0x81)
        .collect::<Vec<_>>();
    u5_runtime::parse_ch_font(&bytes, u5_runtime::IBM_CH_FILE).unwrap()
}

fn synthetic_art() -> MapArt {
    MapArt::build(&synthetic_tile_atlas(TileGraphicsDepth::Ega16))
        .with_fonts(inked_font(), inked_font())
}

// ---------------------------------------------------------------------------
// Synthetic corpora.

const OUTDOOR_TILES: [u8; 18] = [
    0x10, 0x11, 0x14, 0x17, 0x1B, 0xDF, 0xDC, 0xDC, 0x89, 0x8A, 0xA0, 0xA4, 0xF8, 0x05, 0x06, 0x3E,
    0x44, 0x4E,
];

fn random_outdoors(rng: &mut Lcg, plane: WorldPlane) -> PlayState {
    let (px, py) = (60 + rng.below(60) as usize, 60 + rng.below(80) as usize);
    let mut state = world_state(open_world_grid(), px, py);
    state.area = Area::World { plane };
    state.active_objects[0].z = plane.save_floor();
    for _ in 0..rng.below(120) {
        let x = (px + rng.below(80) as usize).wrapping_sub(40) & 0xFF;
        let y = (py + rng.below(40) as usize).wrapping_sub(20) & 0xFF;
        state.grid[world_cell_index(x, y)] = rng.pick(&OUTDOOR_TILES);
    }
    // Objects about: people, a ship, a moongate object over a marker.
    let z = state.active_objects[0].z;
    for _ in 0..rng.below(6) {
        let tile = rng.pick(&[0x14, 0xDC, 0x80, 0x90, 0xA4]);
        state.active_objects.push(ActiveObject {
            type_byte: tile,
            tile,
            x: (px + rng.below(20) as usize).wrapping_sub(10) & 0xFF,
            y: (py + rng.below(20) as usize).wrapping_sub(10) & 0xFF,
            z,
            ..ActiveObject::empty()
        });
    }
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    state
}

#[test]
fn random_outdoor_views_on_both_planes_keep_every_rule() {
    let art = synthetic_art();
    let mut tally = Tally::default();
    for seed in 0..24 {
        let mut rng = Lcg(seed);
        let plane = if seed % 3 == 0 {
            WorldPlane::Underworld
        } else {
            WorldPlane::Britannia
        };
        let state = random_outdoors(&mut rng, plane);
        let around = (state.player.x as u8, state.player.y as u8);
        for k in 0..3 {
            let know = if k == 0 {
                GemKnowledge::new()
            } else {
                random_record(&mut rng, around, 0)
            };
            audit_everywhere(
                &format!("outdoors seed {seed} record {k}"),
                &state,
                &know,
                None,
                &art,
                &mut tally,
            );
        }
    }
    assert!(tally.questions > 100 && tally.known > 10, "vacuous corpus");
}

const TOWN_TILES: [u8; 20] = [
    0xA4, 0x89, 0xF0, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF9, 0xC4, 0xC5, 0xC6, 0xC7, 0xC8,
    0xC9, 0x4D, 0x4E, 0x44,
];

fn random_town(rng: &mut Lcg) -> PlayState {
    let mut grid = vec![0x44; 1024];
    for i in 0..32 {
        for (x, y) in [(i, 0), (i, 31), (0, i), (31, i)] {
            grid[y * 32 + x] = 0x4D;
        }
    }
    for _ in 0..rng.below(4) + 1 {
        let side = rng.below(4);
        let at = 2 + rng.below(28) as usize;
        let (x, y) = match side {
            0 => (at, 0),
            1 => (31, at),
            2 => (at, 31),
            _ => (0, at),
        };
        grid[y * 32 + x] = 0x44;
    }
    for _ in 0..rng.below(40) {
        let (x, y) = (1 + rng.below(30) as usize, 1 + rng.below(30) as usize);
        grid[y * 32 + x] = rng.pick(&TOWN_TILES);
    }
    let (px, py) = (2 + rng.below(28) as usize, 2 + rng.below(28) as usize);
    grid[py * 32 + px] = 0x44;
    let mut state = test_state(grid, px, py);
    // A town with a side window, so the sides are drawn from the world.
    let scene = side_windows_scene(rng);
    state.area = Area::Town {
        scene: Scene::new(scene).unwrap(),
        floor: 0,
    };
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Surface);
    state
}

fn side_windows_scene(rng: &mut Lcg) -> u8 {
    let scenes: Vec<u8> = (1..=32).filter(|s| side_window(*s).is_some()).collect();
    assert!(!scenes.is_empty());
    rng.pick(&scenes)
}

/// A random world map, every tile something the sides might show.
fn random_world(rng: &mut Lcg) -> Vec<u8> {
    (0..256 * 256).map(|_| rng.pick(&OUTDOOR_TILES)).collect()
}

#[test]
fn random_town_views_keep_every_rule_and_their_unseen_sides_black() {
    let art = synthetic_art();
    let mut tally = Tally::default();
    let mut rng = Lcg(77);
    let world = random_world(&mut rng);
    for seed in 0..24 {
        let mut rng = Lcg(1000 + seed);
        let state = random_town(&mut rng);
        let Area::Town { scene, .. } = state.area else {
            unreachable!()
        };
        for k in 0..3 {
            let mut know = if k == 0 {
                GemKnowledge::new()
            } else {
                random_record(&mut rng, (16, 16), 0)
            };
            // Some of this town's sides seen, and its trades and links.
            for _ in 0..rng.below(400) * u32::from(k > 0) {
                know.apply(&KnowEvent::SideCellSeen {
                    scene: scene.byte,
                    index: rng.below(48 * 48) as u16,
                });
            }
            if k == 2 {
                for trade in TRADES {
                    know.apply(&KnowEvent::ShopOpened {
                        scene: scene.byte,
                        trade,
                        keeper: Some((0, rng.below(32) as u8, rng.below(32) as u8)),
                    });
                }
            }
            audit_everywhere(
                &format!("town seed {seed} record {k}"),
                &state,
                &know,
                Some(&world),
                &art,
                &mut tally,
            );
        }
    }
    assert!(tally.questions > 100 && tally.known > 10, "vacuous corpus");
}

const DUNGEON_BYTES: [u8; 20] = [
    0x00, 0x00, 0x00, 0xB0, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x61, 0x62, 0x68, 0x69, 0x70, 0xD0,
    0xD3, 0xDF, 0xE0, 0xF0,
];

fn random_dungeon(rng: &mut Lcg) -> PlayState {
    let level = rng.below(u32::from(DUNGEON_LEVEL_COUNT)) as u8;
    let (px, py) = (rng.below(8) as usize, rng.below(8) as usize);
    let mut state = dungeon_state(open_dungeon_record(), level, px, py);
    for lvl in 0..DUNGEON_LEVEL_COUNT {
        for y in 0..8 {
            for x in 0..8 {
                state.grid[dungeon_cell_index(lvl, x, y)] = rng.pick(&DUNGEON_BYTES);
            }
        }
    }
    state.grid[dungeon_cell_index(level, px, py)] = 0x00;
    state.player.facing = rng.pick(&[
        Direction::North,
        Direction::East,
        Direction::South,
        Direction::West,
    ]);
    state.active_view_overlay = gem_overlay(ViewOverlayKind::Dungeon { level });
    state
}

#[test]
fn random_dungeon_views_keep_every_rule() {
    let art = synthetic_art();
    let mut tally = Tally::default();
    for seed in 0..24 {
        let mut rng = Lcg(5000 + seed);
        let state = random_dungeon(&mut rng);
        let Area::Dungeon { scene, .. } = state.area else {
            unreachable!()
        };
        for k in 0..3 {
            let know = if k == 0 {
                GemKnowledge::new()
            } else {
                random_record(&mut rng, (0, 0), scene.record as u8)
            };
            audit_everywhere(
                &format!("dungeon seed {seed} record {k}"),
                &state,
                &know,
                None,
                &art,
                &mut tally,
            );
        }
    }
    assert!(tally.questions > 50 && tally.known > 5, "vacuous corpus");
    // A title says Underworld only where a record held the visit.
    assert!(tally.texts.iter().any(|t| t.contains("level")));
}

// ---------------------------------------------------------------------------
// The audit is not vacuous: a view built from more than the record shows
// is caught.

fn audit(state: &PlayState, know: &GemKnowledge, view: &GemViewState) -> Vec<String> {
    audit_gem_view(
        state,
        know,
        view,
        &toggles(GlyphPolicy::Perceived, true),
        &synthetic_art(),
    )
    .violations
}

#[test]
fn a_view_drawn_from_a_fuller_record_is_caught() {
    // Dungeon: cells remembered and the Underworld visited in the record
    // the view was opened with, but not in the one audited.
    let mut rng = Lcg(9);
    let mut state = random_dungeon(&mut rng);
    let Area::Dungeon { scene, level } = state.area else {
        unreachable!()
    };
    // A ladder down on the deepest level, so the Underworld has a panel.
    state.grid[dungeon_cell_index(7, 3, 3)] = 0x20;
    let record = scene.record as u8;
    let mut fuller = GemKnowledge::new();
    for lvl in 0..DUNGEON_LEVEL_COUNT {
        fuller.apply(&KnowEvent::DungeonCellsSeen {
            record,
            level: lvl,
            rows: [0xFF; 8],
        });
    }
    fuller.apply(&KnowEvent::UnderworldVisited);
    let view = GemViewState::open_all(
        &state,
        CANVASES[1],
        100,
        TownInputs::default(),
        DungeonInputs {
            know: Some(&fuller),
            glyphs: GlyphPolicy::Perceived,
        },
    );
    assert!(audit(&state, &fuller, &view).is_empty());
    let found = audit(&state, &GemKnowledge::new(), &view);
    assert!(
        found.iter().any(|v| v.contains("drawn without memory")),
        "{found:?}"
    );
    let _ = level;

    // Town: every side cell seen in the view's record, none in the audited.
    let mut rng = Lcg(10);
    let state = random_town(&mut rng);
    let Area::Town { scene, .. } = state.area else {
        unreachable!()
    };
    let world = random_world(&mut rng);
    let mut seen = GemKnowledge::new();
    for index in 0..48 * 48 {
        seen.apply(&KnowEvent::SideCellSeen {
            scene: scene.byte,
            index,
        });
    }
    let view = GemViewState::open_with(
        &state,
        CANVASES[1],
        100,
        TownInputs {
            know: Some(&seen),
            world: Some(&world),
            surroundings: true,
        },
    );
    assert!(audit(&state, &seen, &view).is_empty());
    let found = audit(&state, &GemKnowledge::new(), &view);
    assert!(
        found.iter().any(|v| v.contains("drawn but never seen")),
        "{found:?}"
    );

    // Outdoors: a gate used in the view's record, gone from the cell, is a
    // known mark; audited without the record it is a mark without a gate.
    let mut state = random_outdoors(&mut Lcg(11), WorldPlane::Britannia);
    let (x, y) = (state.player.x, state.player.y + 2);
    state.grid[world_cell_index(x, y)] = 0x05;
    let mut used = GemKnowledge::new();
    used.apply(&KnowEvent::GateUsed(GateCell {
        plane: 0,
        x: x as u8,
        y: y as u8,
    }));
    let view = GemViewState::open(&state, CANVASES[1], 100);
    let report = audit_gem_view(
        &state,
        &used,
        &view,
        &toggles(GlyphPolicy::Perceived, true),
        &synthetic_art(),
    );
    assert!(report.violations.is_empty(), "{:?}", report.violations);
    assert!(report.texts.iter().any(|t| t == "Moongate"));
    assert!(
        flip_changes(&view, &GemKnowledge::new()).is_empty(),
        "a flipped gate changes its own mark only"
    );
}

#[test]
fn every_feature_key_has_its_event() {
    for key in [
        FeatureKey::Place { scene: 1 },
        FeatureKey::Entrance { record: 2 },
        FeatureKey::Shrine { virtue: 3 },
        FeatureKey::Codex,
        FeatureKey::Underworld,
        FeatureKey::Gate(GateCell {
            plane: 0,
            x: 1,
            y: 2,
        }),
        FeatureKey::Sign(SignCell {
            scene: 0,
            floor: 0,
            x: 1,
            y: 2,
        }),
        FeatureKey::Trade {
            scene: 1,
            trade: TRADES[0],
        },
        FeatureKey::TownLink {
            scene: 1,
            floor: 0,
            x: 2,
            y: 3,
        },
        FeatureKey::Exit(ExitRun {
            scene: 1,
            side: EdgeSide::South,
            at: 15,
        }),
    ] {
        let mut know = GemKnowledge::new();
        know.apply(&event_for_key(key).unwrap());
        assert!(know.is_known(&key), "{key:?}");
    }
}

// ---------------------------------------------------------------------------
// Real game files.

/// A writable copy of the game, removed on drop.
struct Game(PathBuf);

impl Game {
    fn copy(dir: &Path) -> Self {
        let game = unique_scratch_dir("gem-spoiler");
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_file() {
                copy_asset_writable(&entry.path(), &game.join(entry.file_name())).unwrap();
            }
        }
        Self(game)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Game {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn real_game() -> Option<Game> {
    let dir = configured_original_asset_dir()?;
    dir.join("BRIT.DAT").exists().then(|| Game::copy(&dir))
}

fn real_art(game: &Path) -> MapArt {
    let atlas = load_tile_atlas(game, TileGraphicsDepth::Ega16).unwrap();
    MapArt::build(&atlas).with_fonts(
        load_ibm_ch_font(game).unwrap(),
        load_runes_ch_font(game).unwrap(),
    )
}

/// The game's own text, normalised (lower case, runs of space as one),
/// as hashes and strings held only for the comparison.
struct GameText {
    texts: Vec<String>,
    hashes: BTreeSet<u64>,
}

fn normalise(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

impl GameText {
    fn load(game: &Path) -> Self {
        let mut texts = Vec::new();
        for record in load_sign_records(game).unwrap().unwrap_or_default() {
            texts.push(normalise(&record.body));
        }
        for file in ["CASTLE.TLK", "TOWNE.TLK", "DWELLING.TLK", "KEEP.TLK"] {
            let path = game.join(file);
            if !path.exists() {
                continue;
            }
            // Conversation text is what a person says: every field of
            // more than one word. A one-word field is a keyword the player
            // types to them ("job", a town's name); a published place name
            // or a fixed word such as "Sign" may be one, and that is no
            // text of the conversation.
            for fields in parse_tlk(&path).unwrap().into_values() {
                texts.extend(
                    fields
                        .iter()
                        .map(|f| normalise(f))
                        .filter(|f| f.contains(' ')),
                );
            }
        }
        texts.retain(|t| !t.is_empty());
        let hashes = texts.iter().map(|t| fnv1a64(t.as_bytes())).collect();
        Self { texts, hashes }
    }

    /// Why `label` is the game's text, or `None`. A label "contains" a
    /// sign's text only when that text is more than one word: a published
    /// place name may hold a single word a sign also shows.
    fn clash(&self, label: &str) -> Option<&'static str> {
        let label = normalise(label);
        if self.hashes.contains(&fnv1a64(label.as_bytes())) {
            return Some("hashes to");
        }
        for text in &self.texts {
            if *text == label {
                return Some("equals");
            }
            if text.contains(' ') && label.contains(text.as_str()) {
                return Some("contains");
            }
        }
        None
    }
}

/// Opens the gem view with a gem, as the player does.
fn use_gem(state: &mut PlayState, game: &Path) -> bool {
    state.gems = state.gems.max(1);
    handle_play_key_input(state, 'v', "", game).unwrap();
    gem_view_open(state)
}

/// Closes it with a key the game throws away.
fn close_gem(state: &mut PlayState, game: &Path) {
    handle_play_key_input(state, ' ', "", game).unwrap();
}

fn audit_real(
    name: &str,
    state: &PlayState,
    know: &GemKnowledge,
    statics: &mut StaticMaps,
    art: &MapArt,
    tally: &mut Tally,
) {
    let world = town_world_plane(state).and_then(|plane| statics.world_map(plane).ok());
    audit_everywhere(name, state, know, world, art, tally);
}

/// The whole real corpus in one test, so the copy, the art and the game's
/// text are read once.
#[test]
fn the_real_corpus_reveals_nothing() {
    let Some(game) = real_game() else {
        return;
    };
    let game = game.path();
    let art = real_art(game);
    let mut statics = StaticMaps::new(game);
    let mut tally = Tally::default();

    // 1. The fresh save after character creation, and a walk from it with
    // the record kept as play keeps it; a gem at the start and every few
    // steps.
    commit_chargen_save(
        game,
        b"Tester",
        true,
        ChargenStats {
            strength: 15,
            dexterity: 15,
            intelligence: 15,
        },
    )
    .unwrap();
    let mut state =
        PlayState::load_scene(game, load_play_options_from_save(game).unwrap()).unwrap();
    let mut session = GemSession::new();
    session.attach(&state, None);
    assert!(
        use_gem(&mut state, game),
        "a gem opens the view at the start"
    );
    audit_real(
        "fresh save",
        &state,
        session.knowledge(),
        &mut statics,
        &art,
        &mut tally,
    );
    // In the starting town, nothing beside it has been seen yet.
    if let Area::Town { .. } = state.area {
        assert!(
            session.knowledge().sides().is_empty(),
            "the fresh record has seen no land outside"
        );
    }
    close_gem(&mut state, game);
    let walk = "sssssddssaasssswwddssssddddsssaaaasss";
    for (step, key) in walk.chars().enumerate() {
        session.before(&state);
        handle_play_key_input(&mut state, key, "", game).unwrap();
        session.after(&state, 1);
        if step % 6 == 5 {
            session.before(&state);
            if use_gem(&mut state, game) {
                session.after(&state, 1);
                audit_real(
                    &format!("walk step {step}"),
                    &state,
                    session.knowledge(),
                    &mut statics,
                    &art,
                    &mut tally,
                );
                close_gem(&mut state, game);
            } else {
                session.after(&state, 1);
            }
        }
    }

    // 2. Each dungeon, first level, from its entrance, with no record and
    // with random ones (cells of every level, marks, the Underworld).
    let mut dungeons = 0;
    for record in 0..8u8 {
        let scene = DungeonScene::from_record(record).unwrap();
        let Ok(mut state) = PlayState::load_scene(
            game,
            PlayOptions {
                target: PlayTarget::Dungeon(scene),
                clock: GameClock::new(12, 0).unwrap(),
                ..PlayOptions::default()
            },
        ) else {
            continue;
        };
        if !use_gem(&mut state, game) {
            continue;
        }
        dungeons += 1;
        let mut rng = Lcg(u64::from(record) + 300);
        for k in 0..4 {
            let know = if k == 0 {
                GemKnowledge::new()
            } else {
                random_record(&mut rng, (0, 0), record)
            };
            audit_real(
                &format!("dungeon {record} record {k}"),
                &state,
                &know,
                &mut statics,
                &art,
                &mut tally,
            );
        }
    }

    assert!(
        dungeons >= 6,
        "a gem opened the view in {dungeons} dungeon(s)"
    );

    // 3. The Underworld: before the visit nothing names it; after it, the
    // record holds the bit.
    let mut under = PlayState::load_scene(
        game,
        PlayOptions {
            target: PlayTarget::World(WorldPlane::Underworld),
            clock: GameClock::new(12, 0).unwrap(),
            ..PlayOptions::default()
        },
    )
    .unwrap();
    assert!(use_gem(&mut under, game));
    let mut before = Tally::default();
    audit_real(
        "the Underworld, never visited",
        &under,
        &GemKnowledge::new(),
        &mut statics,
        &art,
        &mut before,
    );
    assert!(
        before.texts.iter().all(|t| !t.contains("Underworld")),
        "{:?}",
        before.texts
    );
    assert_eq!(before.known, 0, "an empty record names nothing");
    let mut visited = GemKnowledge::new();
    visited.apply(&KnowEvent::UnderworldVisited);
    audit_real(
        "the Underworld, visited",
        &under,
        &visited,
        &mut statics,
        &art,
        &mut tally,
    );
    tally.texts.extend(before.texts);

    // 4. Random records over places all over Britannia and the Underworld.
    let mut rng = Lcg(4242);
    for k in 0..24 {
        let plane = if k % 4 == 3 {
            WorldPlane::Underworld
        } else {
            WorldPlane::Britannia
        };
        let at = (rng.below(256) as usize, rng.below(256) as usize);
        let Ok(mut state) = PlayState::load_scene(
            game,
            PlayOptions {
                target: PlayTarget::World(plane),
                start: Some(at),
                clock: GameClock::new(12, 0).unwrap(),
                ..PlayOptions::default()
            },
        ) else {
            continue;
        };
        if !use_gem(&mut state, game) {
            continue;
        }
        let mut know = random_record(&mut rng, (at.0 as u8, at.1 as u8), 0);
        // Every place known, so names really are drawn.
        if k % 2 == 0 {
            for scene in 0..48 {
                know.apply(&KnowEvent::PlaceEntered { scene });
            }
            for virtue in 0..8 {
                know.apply(&KnowEvent::ShrineEntered { virtue });
            }
            know.apply(&KnowEvent::CodexEntered);
        }
        audit_real(
            &format!("world {k}"),
            &state,
            &know,
            &mut statics,
            &art,
            &mut tally,
        );
    }

    // 5. Every town with its record full: names, trades, links.
    for scene in 1..=32u8 {
        let Ok(mut state) = PlayState::load_scene(
            game,
            PlayOptions {
                target: PlayTarget::Town(Scene::new(scene).unwrap()),
                clock: GameClock::new(12, 0).unwrap(),
                ..PlayOptions::default()
            },
        ) else {
            continue;
        };
        if !use_gem(&mut state, game) {
            continue;
        }
        let mut rng = Lcg(u64::from(scene) + 900);
        let mut know = random_record(&mut rng, (16, 16), 0);
        for trade in TRADES {
            know.apply(&KnowEvent::ShopOpened {
                scene,
                trade,
                keeper: None,
            });
        }
        for index in 0..48 * 48 {
            if rng.chance(30) {
                know.apply(&KnowEvent::SideCellSeen { scene, index });
            }
        }
        audit_real(
            &format!("town {scene}"),
            &state,
            &know,
            &mut statics,
            &art,
            &mut tally,
        );
    }

    assert!(
        tally.views > 500 && tally.questions > 200 && tally.known > 100,
        "vacuous real corpus: {} views, {} \"?\", {} known",
        tally.views,
        tally.questions,
        tally.known
    );
    assert!(tally.texts.len() > 20, "few words drawn");

    // No word drawn is the game's own sign or conversation text.
    let text = GameText::load(game);
    assert!(text.texts.len() > 100, "the game's text was read");
    let clashes: Vec<String> = tally
        .texts
        .iter()
        .filter_map(|label| {
            text.clash(label)
                .map(|why| format!("`{label}` {why} game text"))
        })
        .collect();
    assert!(clashes.is_empty(), "{}", clashes.join("\n"));
}

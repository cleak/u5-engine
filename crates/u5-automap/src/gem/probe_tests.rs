//! M3-05 observer tests: one synthetic before/after pair per place event,
//! each giving exactly its event; the ambiguity fold; purity; and a source
//! scan that keeps `&mut self` out of the observer.

use std::path::Path;

use u5_runtime::enhancements::trajectory_digest;
use u5_runtime::shop_session::{ActiveShopSession, shop_session_for_talk_context};
use u5_runtime::test_fixtures::{
    dungeon_state, open_grid, open_world_grid, test_state, world_state,
};
use u5_runtime::{
    Area, DungeonScene, NATURAL_MOONGATE_TERRAIN_TILE, PlayState, RuntimeNpc, ShrineSession,
    ShrineVirtue, TOWN_GRID_SIDE, WorldPlane, world_cell_index,
};

use super::*;
use crate::gem::knowledge::{FeatureKey, GemKnowledge};

const TOWN: u8 = 0x11;

fn town(x: usize, y: usize) -> PlayState {
    test_state(open_grid(), x, y)
}

fn britannia(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state
}

fn set_floor(state: &mut PlayState, floor: i8) {
    if let Area::Town { scene, .. } = state.area {
        state.area = Area::Town { scene, floor };
    }
}

fn set_town_tile(state: &mut PlayState, x: usize, y: usize, tile: u8) {
    state.grid[y * TOWN_GRID_SIDE + x] = tile;
}

fn raise_gate(state: &mut PlayState, x: usize, y: usize) -> usize {
    let idx = world_cell_index(x, y);
    state.grid[idx] = NATURAL_MOONGATE_TERRAIN_TILE;
    state.natural_moongate_live_cells.push(idx);
    idx
}

fn npc(slot: usize, dialog_id: u8, x: usize, y: usize) -> RuntimeNpc {
    RuntimeNpc {
        slot,
        type_byte: 0x40,
        dialog_id,
        schedule: [0; 16],
        state: 1,
        x,
        y,
        z: 0,
        cached_wp: 1,
        move_queue: Vec::new(),
        move_queue_pos: 0,
        stuck_counter: 0,
        active_object: None,
    }
}

fn session(dialog_id: u8) -> ActiveShopSession {
    shop_session_for_talk_context(dialog_id, None).expect("a shop trigger")
}

const TAVERN_TRIGGER: u8 = 0x82;
const HEALER_TRIGGER: u8 = 0x87;

// ---- one event per pair --------------------------------------------------

#[test]
fn entering_a_town_records_the_place_only() {
    let before = probe(&britannia(100, 100));
    let after = town(15, 15);
    assert_eq!(
        observe_command(&before, &after),
        vec![KnowEvent::PlaceEntered { scene: TOWN }]
    );
}

#[test]
fn entering_a_dungeon_records_the_place_only() {
    let before = probe(&britannia(100, 100));
    let after = dungeon_state(u5_runtime::test_fixtures::open_dungeon_record(), 0, 1, 1);
    let Area::Dungeon { scene, .. } = after.area else {
        unreachable!()
    };
    assert_eq!(
        observe_command(&before, &after),
        vec![KnowEvent::PlaceEntered { scene: scene.byte }]
    );
}

#[test]
fn a_new_floor_or_a_step_is_not_a_new_place() {
    let mut state = town(15, 15);
    let before = probe(&state);
    state.player.x = 16;
    assert!(observe_command(&before, &state).is_empty());
    set_floor(&mut state, 1);
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn the_underworld_is_recorded_on_arrival_only() {
    let before = probe(&britannia(50, 50));
    let after = world_state(open_world_grid(), 50, 50);
    assert_eq!(
        observe_command(&before, &after),
        vec![KnowEvent::UnderworldVisited]
    );
    // Walking about the Underworld adds nothing.
    let again = probe(&after);
    assert!(observe_command(&again, &world_state(open_world_grid(), 51, 50)).is_empty());
}

#[test]
fn a_shrine_session_opening_records_its_virtue() {
    let mut state = britannia(80, 80);
    let before = probe(&state);
    state.active_shrine = Some(ShrineSession::entering(ShrineVirtue::Valor));
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::ShrineEntered {
            virtue: ShrineVirtue::Valor as u8
        }]
    );
    // Still open on the next command: nothing new.
    assert!(observe_command(&probe(&state), &state).is_empty());
}

#[test]
fn the_codex_reader_opening_records_the_codex() {
    let mut state = britannia(80, 80);
    let before = probe(&state);
    state.codex_presentation_steps = 1;
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::CodexEntered]
    );
    let mid = probe(&state);
    state.codex_presentation_steps = 2;
    assert!(observe_command(&mid, &state).is_empty());
}

#[test]
fn a_signed_shop_records_its_trade_and_no_keeper() {
    let mut state = town(10, 10);
    state.npcs.push(npc(3, HEALER_TRIGGER, 10, 9));
    let before = probe(&state);
    state.active_shop = Some(session(HEALER_TRIGGER));
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::ShopOpened {
            scene: TOWN,
            trade: Trade::Healer,
            keeper: None
        }]
    );
}

#[test]
fn a_tavern_records_its_keeper_cell() {
    let mut state = town(10, 10);
    state.npcs.push(npc(3, TAVERN_TRIGGER, 11, 10));
    let before = probe(&state);
    state.active_shop = Some(session(TAVERN_TRIGGER));
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::ShopOpened {
            scene: TOWN,
            trade: Trade::Tavern,
            keeper: Some((0, 11, 10))
        }]
    );
}

#[test]
fn a_tavern_turning_to_its_sage_records_the_sage_at_the_keeper() {
    let mut state = town(10, 10);
    state.npcs.push(npc(3, TAVERN_TRIGGER, 11, 10));
    state.active_shop = Some(session(TAVERN_TRIGGER));
    let before = probe(&state);
    state.active_shop = Some(ActiveShopSession::Sage(Default::default()));
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::ShopOpened {
            scene: TOWN,
            trade: Trade::Sage,
            keeper: Some((0, 11, 10))
        }]
    );
}

#[test]
fn two_possible_keepers_record_the_trade_without_a_cell() {
    let mut state = town(10, 10);
    state.npcs.push(npc(3, TAVERN_TRIGGER, 11, 10));
    state.npcs.push(npc(4, TAVERN_TRIGGER, 9, 10));
    let before = probe(&state);
    state.active_shop = Some(session(TAVERN_TRIGGER));
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::ShopOpened {
            scene: TOWN,
            trade: Trade::Tavern,
            keeper: None
        }]
    );
}

#[test]
fn a_moongate_transit_records_both_ends() {
    let mut state = britannia(100, 100);
    let gate = raise_gate(&mut state, 101, 100);
    let before = probe(&state);
    assert!(!before.on_live_gate);
    // The transit rewrites the gate it used and puts the party elsewhere.
    state.grid[gate] = 5;
    state.natural_moongate_live_cells.clear();
    state.player.x = 30;
    state.player.y = 200;
    assert_eq!(
        observe_command(&before, &state),
        vec![
            KnowEvent::GateUsed(GateCell {
                plane: 0,
                x: 101,
                y: 100
            }),
            KnowEvent::GateUsed(GateCell {
                plane: 0,
                x: 30,
                y: 200
            }),
        ]
    );
}

#[test]
fn a_moongate_to_the_underworld_records_both_planes() {
    let mut state = britannia(0, 0);
    raise_gate(&mut state, 255, 255);
    let before = probe(&state);
    let mut after = world_state(open_world_grid(), 40, 41);
    after.natural_moongate_live_cells.clear();
    let events = observe_command(&before, &after);
    assert_eq!(
        events,
        vec![
            KnowEvent::UnderworldVisited,
            KnowEvent::GateUsed(GateCell {
                plane: 0,
                x: 255,
                y: 255
            }),
            KnowEvent::GateUsed(GateCell {
                plane: 1,
                x: 40,
                y: 41
            }),
        ]
    );
}

#[test]
fn a_jump_without_a_vanished_gate_records_no_gate() {
    // A gate beside the party that is still standing after a jump was not
    // the way the party went (some other teleport).
    let mut state = britannia(100, 100);
    raise_gate(&mut state, 101, 100);
    let before = probe(&state);
    state.player.x = 30;
    assert!(observe_command(&before, &state).is_empty());
    // A step onto the gate's neighbour is not a jump.
    let mut walk = britannia(100, 100);
    let gate = raise_gate(&mut walk, 101, 100);
    let before = probe(&walk);
    walk.grid[gate] = 5;
    walk.player.x = 99;
    assert!(observe_command(&before, &walk).is_empty());
    // No live gate near the party: a jump is not a moongate.
    let mut far = britannia(100, 100);
    let before = probe(&far);
    far.player.x = 10;
    assert!(observe_command(&before, &far).is_empty());
}

#[test]
fn a_sunk_gate_tile_without_the_live_list_is_not_a_gate() {
    // The tile alone is not a live gate (10.2.7: "only while the gate is up").
    let mut state = britannia(100, 100);
    let idx = world_cell_index(101, 100);
    state.grid[idx] = NATURAL_MOONGATE_TERRAIN_TILE;
    let before = probe(&state);
    assert_eq!(before.live_gates, 0);
    state.grid[idx] = 5;
    state.player.x = 10;
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn a_town_ladder_records_both_ends_with_their_directions() {
    let mut state = town(12, 7);
    set_town_tile(&mut state, 12, 7, 0xC4);
    let before = probe(&state);
    set_floor(&mut state, 1);
    set_town_tile(&mut state, 12, 7, 0xC5);
    assert_eq!(
        observe_command(&before, &state),
        vec![
            KnowEvent::TownLinkUsed(TownLink {
                scene: TOWN,
                floor: 0,
                x: 12,
                y: 7,
                dir: LinkDir::Up
            }),
            KnowEvent::TownLinkUsed(TownLink {
                scene: TOWN,
                floor: 1,
                x: 12,
                y: 7,
                dir: LinkDir::Down
            }),
        ]
    );
}

#[test]
fn town_stairs_stepped_onto_record_the_stair_cell_going_down() {
    let mut state = town(12, 7);
    set_floor(&mut state, 1);
    set_town_tile(&mut state, 13, 7, 0xC8);
    let before = probe(&state);
    // The step moves the party onto the stairs and the floor changes.
    state.player.x = 13;
    set_floor(&mut state, 0);
    set_town_tile(&mut state, 13, 7, 0x01);
    assert_eq!(
        observe_command(&before, &state),
        vec![KnowEvent::TownLinkUsed(TownLink {
            scene: TOWN,
            floor: 1,
            x: 13,
            y: 7,
            dir: LinkDir::Down
        })]
    );
}

#[test]
fn a_trapdoor_floor_change_is_not_a_ladder() {
    let mut state = town(12, 7);
    set_town_tile(&mut state, 12, 7, 0x01);
    let before = probe(&state);
    set_floor(&mut state, -1);
    assert!(observe_command(&before, &state).is_empty());
}

#[test]
fn leaving_a_town_across_an_edge_records_the_exit() {
    let state = town(0, 10);
    let before = probe(&state);
    let after = britannia(100, 100);
    assert_eq!(
        observe_command(&before, &after),
        vec![KnowEvent::TownExit(ExitRun {
            scene: TOWN,
            side: EdgeSide::West,
            at: 10
        })]
    );
}

#[test]
fn arriving_on_an_edge_records_the_place_and_the_exit() {
    let before = probe(&britannia(100, 100));
    let after = town(15, 31);
    assert_eq!(
        observe_command(&before, &after),
        vec![
            KnowEvent::PlaceEntered { scene: TOWN },
            KnowEvent::TownExit(ExitRun {
                scene: TOWN,
                side: EdgeSide::South,
                at: 15
            }),
        ]
    );
}

#[test]
fn a_corner_or_inner_cell_records_no_exit() {
    for (x, y) in [(0, 0), (31, 31), (5, 5)] {
        let before = probe(&town(x, y));
        assert!(observe_command(&before, &britannia(100, 100)).is_empty());
    }
    assert_eq!(edge_side(0, 0), None);
    assert_eq!(edge_side(31, 4), Some((EdgeSide::East, 4)));
    assert_eq!(edge_side(4, 0), Some((EdgeSide::North, 4)));
}

#[test]
fn attach_records_the_place_or_the_underworld() {
    assert_eq!(
        attach_events(&town(3, 3)),
        vec![KnowEvent::PlaceEntered { scene: TOWN }]
    );
    assert_eq!(
        attach_events(&world_state(open_world_grid(), 3, 3)),
        vec![KnowEvent::UnderworldVisited]
    );
    assert!(attach_events(&britannia(3, 3)).is_empty());
}

// ---- two commands in one frame -------------------------------------------

#[test]
fn two_commands_fold_only_the_unambiguous_events() {
    // World to town onto an edge, with a tavern open: the place and the
    // trade stand, the exit cell and the keeper cell are dropped.
    let before = probe(&britannia(100, 100));
    let mut after = town(15, 31);
    after.npcs.push(npc(3, TAVERN_TRIGGER, 15, 30));
    after.active_shop = Some(session(TAVERN_TRIGGER));
    assert_eq!(
        observe_commands(&before, &after, 2),
        vec![
            KnowEvent::PlaceEntered { scene: TOWN },
            KnowEvent::ShopOpened {
                scene: TOWN,
                trade: Trade::Tavern,
                keeper: None
            },
        ]
    );
    // One command gives both cells.
    assert_eq!(observe_commands(&before, &after, 1).len(), 3);
}

#[test]
fn two_commands_drop_gates_ladders_and_exits() {
    let mut gate = britannia(100, 100);
    let idx = raise_gate(&mut gate, 101, 100);
    let before = probe(&gate);
    gate.grid[idx] = 5;
    gate.natural_moongate_live_cells.clear();
    gate.player.x = 30;
    assert!(observe_commands(&before, &gate, 2).is_empty());

    let mut ladder = town(12, 7);
    set_town_tile(&mut ladder, 12, 7, 0xC4);
    let before = probe(&ladder);
    set_floor(&mut ladder, 1);
    assert!(observe_commands(&before, &ladder, 2).is_empty());

    let before = probe(&town(0, 10));
    assert!(observe_commands(&before, &britannia(1, 1), 2).is_empty());

    // The shrine, Codex and Underworld stand.
    let mut state = britannia(5, 5);
    let before = probe(&state);
    state.area = Area::World {
        plane: WorldPlane::Underworld,
    };
    state.codex_presentation_steps = 1;
    state.active_shrine = Some(ShrineSession::entering(ShrineVirtue::Honor));
    assert_eq!(observe_commands(&before, &state, 2).len(), 3);
}

// ---- the record ----------------------------------------------------------

#[test]
fn every_event_folds_into_the_record_once() {
    let mut know = GemKnowledge::new();
    let mut state = town(10, 10);
    state.npcs.push(npc(3, TAVERN_TRIGGER, 11, 10));
    let before = probe(&britannia(100, 100));
    state.active_shop = Some(session(TAVERN_TRIGGER));
    let events = observe_command(&before, &state);
    for event in &events {
        assert!(know.apply(event));
        assert!(!know.apply(event), "a second fold changes nothing");
    }
    assert!(know.is_known(&FeatureKey::Place { scene: TOWN }));
    assert!(know.is_known(&FeatureKey::Trade {
        scene: TOWN,
        trade: Trade::Tavern
    }));
    assert_eq!(know.keepers_in(TOWN).count(), 1);
    // Nothing else is known: no other trade, no Underworld, no link.
    assert!(!know.is_known(&FeatureKey::Trade {
        scene: TOWN,
        trade: Trade::Healer
    }));
    assert!(!know.is_known(&FeatureKey::Underworld));
    assert!(!know.is_known(&FeatureKey::Codex));
}

#[test]
fn a_dungeon_entrance_is_known_once_entered() {
    let mut know = GemKnowledge::new();
    let scene = DungeonScene::from_record(2).unwrap();
    assert!(!know.is_known(&FeatureKey::Entrance { record: 2 }));
    know.apply(&KnowEvent::PlaceEntered { scene: scene.byte });
    assert!(know.is_known(&FeatureKey::Entrance { record: 2 }));
    assert!(!know.is_known(&FeatureKey::Entrance { record: 3 }));
    know.apply(&KnowEvent::EntranceNamed { record: 3 });
    assert!(know.is_known(&FeatureKey::Entrance { record: 3 }));
}

#[test]
fn a_town_ladder_reports_the_directions_used() {
    let mut know = GemKnowledge::new();
    let mut state = town(12, 7);
    set_town_tile(&mut state, 12, 7, 0xC4);
    let before = probe(&state);
    set_floor(&mut state, 1);
    set_town_tile(&mut state, 12, 7, 0xC5);
    for event in observe_command(&before, &state) {
        know.apply(&event);
    }
    let key = |floor| FeatureKey::TownLink {
        scene: TOWN,
        floor,
        x: 12,
        y: 7,
    };
    assert!(know.is_known(&key(0)) && know.is_known(&key(1)));
    assert_eq!(
        know.town_link_dirs(TOWN, 0, 12, 7).collect::<Vec<_>>(),
        [LinkDir::Up]
    );
    assert_eq!(
        know.town_link_dirs(TOWN, 1, 12, 7).collect::<Vec<_>>(),
        [LinkDir::Down]
    );
}

// ---- purity --------------------------------------------------------------

#[test]
fn probe_and_observe_leave_the_state_unchanged() {
    let mut states = [
        town(10, 10),
        britannia(100, 100),
        world_state(open_world_grid(), 3, 3),
        dungeon_state(u5_runtime::test_fixtures::open_dungeon_record(), 2, 3, 3),
    ];
    states[0].npcs.push(npc(3, TAVERN_TRIGGER, 11, 10));
    states[0].active_shop = Some(session(TAVERN_TRIGGER));
    raise_gate(&mut states[1], 101, 100);
    for (i, state) in states.iter_mut().enumerate() {
        state.prng_state = 0x1234 + i as u16;
        let digest = trajectory_digest(state);
        let copy = state.clone();
        let before = probe(&copy);
        let _ = observe_command(&before, &copy);
        let _ = observe_commands(&before, state, 2);
        let _ = attach_events(state);
        assert_eq!(trajectory_digest(&copy), digest, "state {i}");
        assert_eq!(trajectory_digest(state), digest, "state {i}");
        assert_eq!(state.prng_state, 0x1234 + i as u16);
    }
}

/// The names of every `&mut self` method of `PlayState`: each `impl
/// PlayState {` block (rustfmt puts it at column 0 and closes it with a
/// lone `}`), each `fn` at four spaces in it, its receiver on the same or
/// the next line.
fn play_state_mut_methods() -> Vec<String> {
    fn walk(dir: &Path, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).unwrap();
                let lines: Vec<&str> = text.lines().collect();
                let mut inside = false;
                for (i, line) in lines.iter().enumerate() {
                    if line.starts_with("impl PlayState {") {
                        inside = true;
                        continue;
                    }
                    if inside && *line == "}" {
                        inside = false;
                        continue;
                    }
                    if !inside {
                        continue;
                    }
                    let Some(rest) = line.strip_prefix("    ") else {
                        continue;
                    };
                    let Some(at) = rest.find("fn ") else { continue };
                    let head = &rest[..at];
                    if !head
                        .split_whitespace()
                        .all(|w| matches!(w, "pub" | "pub(crate)" | "const" | "async"))
                    {
                        continue;
                    }
                    let sig = &rest[at + 3..];
                    let name: String = sig
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    let receiver = format!("{sig} {}", lines.get(i + 1).unwrap_or(&""));
                    if receiver.contains("&mut self") {
                        out.push(name);
                    }
                }
            }
        }
    }
    let root = u5_runtime::test_fixtures::workspace_file("crates/u5-runtime/src");
    let mut names = Vec::new();
    walk(&root, &mut names);
    names.sort();
    names.dedup();
    names
}

#[test]
fn the_observer_names_no_mutating_method() {
    let names = play_state_mut_methods();
    // The scan itself must see the game's mutators.
    for known in ["clear_message_window", "push_message_entry", "advance_turn"] {
        assert!(names.iter().any(|n| n == known), "scan missed {known}");
    }
    // The observer's files: the place events (M3-05), Look and the dungeon
    // events, and the seen cells (M3-06).
    for (file, source) in [
        ("probe.rs", include_str!("probe.rs")),
        ("probe_more.rs", include_str!("probe_more.rs")),
        ("seen.rs", include_str!("seen.rs")),
    ] {
        let code: String = source
            .split("#[cfg(test)]")
            .next()
            .unwrap()
            .lines()
            .map(|line| line.split("//").next().unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!code.contains("&mut PlayState"), "{file}");
        assert!(!code.contains("&mut self"), "{file}");
        for name in &names {
            for call in [format!(".{name}("), format!("::{name}(")] {
                assert!(
                    !code.contains(&call),
                    "{file} calls the mutating PlayState method {name}"
                );
            }
        }
    }
}

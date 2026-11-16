//! Chapter 7 Phase 4 (design 7.3.6, D7.8, D7.9; build plan M8-04): the
//! carry-options refactor's golden test and the teleport runtime.

use std::fs;
use std::path::Path;

use super::*;
use crate::test_fixtures::{
    debug_game_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
    saved_game_seed_bytes, test_state, world_state, write_britannia_world_files,
    write_empty_ool_mirrors,
};
use crate::text_hold::PendingTextHold;

/// FNV-1a, so a golden digest never depends on the std hasher.
fn fnv(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn britannia(x: usize, y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), x, y);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state
        .rebuild_world_live_chunks_from_grid(WorldPlane::Britannia)
        .unwrap();
    state
}

/// Every `.OOL` file in the game directory, by name.
fn ool_bytes(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.to_ascii_uppercase().ends_with(".OOL"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|name| {
            let bytes = fs::read(dir.join(&name)).unwrap();
            (name, bytes)
        })
        .collect()
}

/// The fixture's eight moonstone slots: both planes, a far and a near
/// overworld warp, two castle floors, two dungeons and one empty slot.
fn golden_slots() -> [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT] {
    let slot = |scene, x, y, z| MoonstoneGateSlot { scene, x, y, z };
    let mut slots = [slot(MOONSTONE_INVALID_SCENE, 0, 0, 0); MOONSTONE_SLOT_COUNT];
    let wanted = [
        slot(0, 100, 100, 0),
        slot(0, 50, 60, 1),
        slot(17, 6, 7, 0),
        slot(18, 15, 15, 0),
        slot(33, 1, 1, 0),
        slot(34, 3, 5, 2),
        slot(0, 7, 6, 0),
    ];
    for (index, wanted) in wanted.into_iter().enumerate().take(MOONSTONE_SLOT_COUNT) {
        slots[index] = wanted;
    }
    slots
}

/// The digest of one gate travel from a fixed overworld state into one
/// slot: the watch snapshot, the transcript, the sound serials and the
/// `.OOL` bytes, with the destination's own outcome.
fn gate_travel_digest(phase: usize) -> u64 {
    let dir = debug_game_dir();
    let mut state = britannia(5, 5);
    state.moonstone_slots = golden_slots();
    state.player.transport = TransportState::Horse {
        type_byte: 160,
        tile: 160,
    };
    state.emit_message_line("one".to_string());
    state.emit_sound_effect(SoundEffect::BlockedStep);
    state.emit_sound_effect(SoundEffect::ActionSnap);
    let destination = gate_travel_destination_in(&dir, state.moonstone_slots[phase]).unwrap();
    let mut text = format!("{destination:?}\n");
    if let GateTravelDestination::Ready {
        target,
        floor,
        start,
    } = destination
    {
        let outcome = state.apply_gate_travel(&dir, phase, target, floor, start);
        text.push_str(&format!("{:?}\n", outcome.map_err(|err| err.to_string())));
    }
    // The PRNG is seeded from the host clock at every scene load, so it is
    // the one watched value a golden digest cannot hold.
    let watched: Vec<_> = watch_snapshot(&state)
        .into_iter()
        .filter(|(field, _)| *field != WatchField::Prng)
        .collect();
    text.push_str(&format!("{watched:?}\n"));
    text.push_str(&format!(
        "{:?} {:?} {}\n",
        state.message_transcript, state.message, state.message_row_open_mid_line
    ));
    text.push_str(&format!(
        "{} {:?}\n",
        state.sound_effect_serial,
        state.sound_effects_with_serials_after(0)
    ));
    text.push_str(&format!(
        "{:?} {} {:?}\n",
        state.player, state.turn, state.clock
    ));
    let mut bytes = text.into_bytes();
    for (name, file) in ool_bytes(&dir) {
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&file);
    }
    let _ = fs::remove_dir_all(dir);
    fnv(&bytes)
}

/// D7.8: the extraction of `carried_play_options` and
/// `carry_presentation_into` is byte-identical for every moonstone slot.
/// The digests were recorded from `apply_gate_travel` before the refactor.
#[test]
fn gate_travel_golden_digest_per_moonstone_slot_is_unchanged_by_the_carry_refactor() {
    let digests: Vec<u64> = (0..MOONSTONE_SLOT_COUNT).map(gate_travel_digest).collect();
    assert_eq!(digests, GOLDEN_GATE_TRAVEL_DIGESTS.to_vec());
}

const GOLDEN_GATE_TRAVEL_DIGESTS: [u64; MOONSTONE_SLOT_COUNT] = [
    0x5036_4669_2f8b_d6a3,
    0xff64_aefc_b890_7278,
    0x7117_1bb5_7472_253c,
    0xaa8d_ff24_f993_6b16,
    0x03be_a89f_110d_b20e,
    0x9e65_5e79_a8d0_c843,
    0x9ef8_b9a0_bc65_22f4,
    0xc526_e62b_82d0_20d9,
];

// --- the teleport -----------------------------------------------------------

const HORSE: TransportState = TransportState::Horse {
    type_byte: 160,
    tile: 160,
};
const CARPET: TransportState = TransportState::Carpet {
    type_byte: 184,
    tile: 184,
};

fn castle(floor: i8) -> TeleportArea {
    TeleportArea::Location {
        scene: Scene::new(17).unwrap(),
        floor,
    }
}

fn dungeon(level: u8) -> TeleportArea {
    TeleportArea::Dungeon {
        scene: DungeonScene::new(33).unwrap(),
        level,
    }
}

fn at(area: TeleportArea, x: usize, y: usize) -> TeleportTarget {
    TeleportTarget { area, x, y }
}

fn ready(target: TeleportTarget, nudged: bool) -> TeleportPlan {
    TeleportPlan::Ready { target, nudged }
}

/// Every file in the directory, by name, with its bytes.
fn dir_files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let bytes = fs::read(entry.path()).unwrap();
            (name, bytes)
        })
        .collect();
    files.sort();
    files
}

fn teleport(state: &mut PlayState, dir: &Path, want: TeleportTarget) -> TeleportPlan {
    let plan = plan_teleport(state, dir, want);
    assert!(
        matches!(plan, TeleportPlan::Ready { .. }),
        "{want:?}: {plan:?}"
    );
    commit_teleport(state, dir, &plan).unwrap();
    plan
}

/// The Phase 4 entry check (7.3.6): `load_scene` writes no file, so the
/// cross-area plan may probe a scratch state built by it.
#[test]
fn entry_check_load_scene_writes_no_file() {
    let dir = debug_game_dir();
    let state = britannia(5, 5);
    for (target, floor, start) in [
        (PlayTarget::World(WorldPlane::Britannia), 0, (100, 100)),
        (PlayTarget::World(WorldPlane::Underworld), -1, (50, 60)),
        (PlayTarget::Town(Scene::new(17).unwrap()), 0, (6, 7)),
        (
            PlayTarget::Dungeon(DungeonScene::new(33).unwrap()),
            2,
            (1, 1),
        ),
    ] {
        let before = dir_files(&dir);
        PlayState::load_scene(&dir, state.carried_play_options(target, floor, start)).unwrap();
        assert_eq!(dir_files(&dir), before, "{target:?}");
    }
    let _ = fs::remove_dir_all(dir);
}

/// Planning is read-only: no file and no state changes.
#[test]
fn plan_teleport_changes_nothing() {
    let dir = debug_game_dir();
    let state = britannia(5, 5);
    let before_files = dir_files(&dir);
    let before = format!("{state:?}");
    for want in [
        at(TeleportArea::World(WorldPlane::Underworld), 50, 60),
        at(castle(0), 6, 7),
        at(dungeon(3), 2, 2),
        at(TeleportArea::World(WorldPlane::Britannia), 9, 9),
    ] {
        let _ = plan_teleport(&state, &dir, want);
    }
    assert_eq!(format!("{state:?}"), before);
    assert_eq!(dir_files(&dir), before_files);
    let _ = fs::remove_dir_all(dir);
}

/// The validity matrix: every scene class lands; the bounds and floors are
/// the moongate's own; the gates refuse in their fixed order.
#[test]
fn validity_matrix() {
    let dir = debug_game_dir();
    let state = britannia(5, 5);
    for want in [
        at(TeleportArea::World(WorldPlane::Britannia), 255, 255),
        at(TeleportArea::World(WorldPlane::Underworld), 0, 0),
        at(castle(0), 31, 31),
        at(dungeon(0), 0, 0),
        at(dungeon(7), 7, 7),
    ] {
        assert_eq!(
            plan_teleport(&state, &dir, want),
            ready(want, false),
            "{want:?}"
        );
    }
    for (want, refusal) in [
        (
            at(TeleportArea::World(WorldPlane::Britannia), 256, 0),
            TeleportRefusal::OutOfBounds,
        ),
        (at(castle(0), 0, 32), TeleportRefusal::OutOfBounds),
        (at(dungeon(0), 8, 0), TeleportRefusal::OutOfBounds),
        (at(dungeon(8), 0, 0), TeleportRefusal::BadFloor),
    ] {
        assert_eq!(
            plan_teleport(&state, &dir, want),
            TeleportPlan::Refused(refusal),
            "{want:?}"
        );
    }
    // A floor outside the location's published run (the P23-03 slot).
    let _ = fs::remove_file(dir.join(LOCATION_FLOOR_TABLE_FILE));
    assert_eq!(
        plan_teleport(&state, &dir, at(castle(5), 15, 15)),
        TeleportPlan::Refused(TeleportRefusal::BadFloor)
    );

    let want = at(castle(0), 6, 7);
    let refused = |state: &PlayState| match plan_teleport(state, &dir, want) {
        TeleportPlan::Refused(refusal) => refusal,
        plan => panic!("{plan:?}"),
    };
    for (transport, refusal) in [
        (
            TransportState::Ship {
                type_byte: SHIP_TRANSPORT_FURLED_FIRST,
                tile: SHIP_TRANSPORT_FURLED_FIRST,
                sails_hoisted: false,
                hull: 50,
                skiffs: 1,
            },
            TeleportRefusal::AboardVessel,
        ),
        (
            TransportState::Skiff {
                type_byte: 0,
                tile: 0,
            },
            TeleportRefusal::AboardVessel,
        ),
    ] {
        let mut aboard = britannia(5, 5);
        aboard.player.transport = transport;
        assert_eq!(refused(&aboard), refusal);
    }
    let mut combat = britannia(5, 5);
    combat.combat_active = true;
    combat.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    combat.pending_combat_actor_slot = Some(0);
    assert_eq!(refused(&combat), TeleportRefusal::InCombat);
    let mut busy = britannia(5, 5);
    busy.pending_text_hold = Some(PendingTextHold::default());
    assert_eq!(refused(&busy), TeleportRefusal::NotAtIdlePrompt);

    // A refused plan cannot be committed.
    let mut state = britannia(5, 5);
    let before = format!("{state:?}");
    let plan = TeleportPlan::Refused(TeleportRefusal::NoOpenCell);
    assert!(commit_teleport(&mut state, &dir, &plan).is_err());
    assert_eq!(format!("{state:?}"), before);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn rings_run_rows_top_to_bottom_then_columns_and_clip_to_the_map() {
    assert_eq!(
        teleport_ring(5, 5, 1, 32),
        [
            (4, 4),
            (5, 4),
            (6, 4),
            (4, 5),
            (6, 5),
            (4, 6),
            (5, 6),
            (6, 6)
        ]
    );
    assert_eq!(teleport_ring(0, 0, 1, 32), [(1, 0), (0, 1), (1, 1)]);
    assert_eq!(
        teleport_ring(7, 7, 2, 8),
        [(5, 5), (6, 5), (7, 5), (5, 6), (5, 7)]
    );
    assert_eq!(teleport_ring(10, 10, 3, 32).len(), 24);
}

/// D7.9: a blocked target takes the first open cell of the rings, in
/// order; an NPC or an object blocks like ground; no open cell refuses.
#[test]
fn nudge_order_and_no_open_cell() {
    let dir = debug_game_dir();
    let area = castle(0);
    let mut state = test_state(open_grid(), 20, 20);
    assert!(!state.tile_walkable(BRIT_DEEP_WATER_TILE));
    let want = at(area, 10, 10);
    assert_eq!(plan_teleport(&state, &dir, want), ready(want, false));

    state.grid[10 * 32 + 10] = BRIT_DEEP_WATER_TILE;
    assert_eq!(
        plan_teleport(&state, &dir, want),
        ready(at(area, 9, 9), true)
    );
    // (9, 9) water, (10, 9) an NPC, (11, 9) an object: (9, 10) is next.
    state.grid[9 * 32 + 9] = BRIT_DEEP_WATER_TILE;
    state.npcs.push(RuntimeNpc {
        slot: 1,
        type_byte: 0x50,
        dialog_id: 0,
        schedule: [0; 16],
        state: 0,
        x: 10,
        y: 9,
        z: 0,
        cached_wp: 0,
        move_queue: Vec::new(),
        move_queue_pos: 0,
        stuck_counter: 0,
        active_object: None,
    });
    state.active_objects.push(ActiveObject {
        type_byte: 0x70,
        tile: 0x70,
        x: 11,
        y: 9,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(
        plan_teleport(&state, &dir, want),
        ready(at(area, 9, 10), true)
    );

    // Everything within radius 3 blocked: the radius-4 cell does not count.
    for y in 6..=14 {
        for x in 6..=14 {
            state.grid[y * 32 + x] = BRIT_DEEP_WATER_TILE;
        }
    }
    state.grid[6 * 32 + 6] = 16;
    assert_eq!(
        plan_teleport(&state, &dir, want),
        TeleportPlan::Refused(TeleportRefusal::NoOpenCell)
    );
    state.grid[7 * 32 + 7] = 16;
    assert_eq!(
        plan_teleport(&state, &dir, want),
        ready(at(area, 7, 7), true)
    );
    let _ = fs::remove_dir_all(dir);
}

/// In a dungeon only an ordinary passage cell is a landing cell: walls,
/// doors, room triggers and ladders never are.
#[test]
fn dungeon_landing_is_a_passage_cell() {
    let dir = debug_game_dir();
    let area = dungeon(2);
    let mut state = dungeon_state(open_dungeon_record(), 2, 0, 0);
    let want = at(area, 4, 4);
    assert_eq!(plan_teleport(&state, &dir, want), ready(want, false));
    for tile in [0x10u8, 0x20, 0x30, 0x40, 0xB0, 0xE0, 0xF0] {
        state.grid[dungeon_cell_index(2, 4, 4)] = tile;
        assert_eq!(
            plan_teleport(&state, &dir, want),
            ready(at(area, 3, 3), true),
            "{tile:#04x}"
        );
    }
    let _ = fs::remove_dir_all(dir);
}

/// Inside one floor the party is placed where it stands: no file, no
/// rebuild, the facing kept, the visibility marked dirty.
#[test]
fn same_floor_teleport_moves_the_party_only() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 2, 0, 0);
    state.player.facing = Direction::West;
    state.visibility_dirty = false;
    let files = dir_files(&dir);
    teleport(&mut state, &dir, at(dungeon(2), 5, 6));
    assert_eq!((state.player.x, state.player.y), (5, 6));
    assert_eq!(state.player.facing, Direction::West);
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (5, 6)
    );
    assert!(state.visibility_dirty);
    assert_eq!(dir_files(&dir), files);
    let _ = fs::remove_dir_all(dir);
}

/// A horse or carpet is kept at overworld and location targets; a dungeon
/// target always puts the party on foot.
#[test]
fn mounts_are_kept_except_in_a_dungeon() {
    for mount in [HORSE, CARPET] {
        for (want, expected) in [
            (
                at(TeleportArea::World(WorldPlane::Britannia), 100, 100),
                mount,
            ),
            (
                at(TeleportArea::World(WorldPlane::Underworld), 50, 60),
                mount,
            ),
            (at(castle(0), 6, 7), mount),
            (at(dungeon(1), 2, 2), TransportState::Foot),
        ] {
            let dir = debug_game_dir();
            let mut state = britannia(5, 5);
            state.player.transport = mount;
            state.sync_player_object();
            teleport(&mut state, &dir, want);
            assert_eq!(state.player.transport, expected, "{want:?}");
            assert!(want.area.is_current(state.area), "{:?}", state.area);
            assert_eq!((state.player.x, state.player.y), (want.x, want.y));
            let _ = fs::remove_dir_all(dir);
        }
    }
}

/// No time passes, nothing is narrated and no cue plays; the first real
/// turn at the destination runs its normal epilogue.
#[test]
fn no_turn_no_clock_no_narration_and_the_next_turn_runs() {
    for want in [
        at(castle(0), 6, 7),
        at(TeleportArea::World(WorldPlane::Underworld), 50, 60),
        at(TeleportArea::World(WorldPlane::Britannia), 100, 100),
        at(dungeon(0), 1, 1),
    ] {
        let dir = debug_game_dir();
        let mut state = britannia(5, 5);
        state.turn = 41;
        state.emit_message_line("one".to_string());
        state.emit_sound_effect(SoundEffect::BlockedStep);
        let (clock, food) = (state.clock, state.food);
        let transcript = format!("{:?}", state.message_transcript);
        let message = state.message.clone();
        let serial = state.sound_effect_serial;
        let sounds = state.sound_effects_with_serials_after(0);
        teleport(&mut state, &dir, want);
        assert_eq!(
            (state.turn, state.clock, state.food),
            (41, clock, food),
            "{want:?}"
        );
        assert_eq!(format!("{:?}", state.message_transcript), transcript);
        assert_eq!(state.message, message);
        assert_eq!(state.sound_effect_serial, serial, "{want:?}: no cue");
        assert_eq!(state.sound_effects_with_serials_after(0), sounds);
        assert!(
            !state
                .diagnostics
                .iter()
                .any(|line| line.contains("Gate Travel"))
        );

        state.pass_turn_with_game_dir(Some(&dir)).unwrap();
        assert_eq!(state.turn, 42, "{want:?}");
        assert_ne!(state.clock, clock, "{want:?}: the epilogue ran");
        let _ = fs::remove_dir_all(dir);
    }
}

/// Leaving the overworld writes the departing plane's `.OOL`, exactly as
/// any transit does.
#[test]
fn an_outdoor_departure_writes_the_departing_plane_ool() {
    let dir = debug_game_dir();
    let expected_dir = debug_game_dir();
    let mut state = britannia(5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: 160,
        tile: 160,
        x: 7,
        y: 7,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    state
        .write_current_world_plane_object_table(&expected_dir, true)
        .unwrap();
    teleport(&mut state, &dir, at(castle(0), 6, 7));
    assert_eq!(
        fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(),
        fs::read(expected_dir.join(BRIT_OOL_FILENAME)).unwrap()
    );
    let _ = fs::remove_dir_all(dir);
    let _ = fs::remove_dir_all(expected_dir);
}

/// A save at the destination records it, and a reload starts there.
#[test]
fn save_and_reload_at_the_destination() {
    for (want, location) in [
        (at(castle(0), 6, 7), (17, 0, 6, 7)),
        (at(dungeon(3), 2, 5), (33, 3, 2, 5)),
        (
            at(TeleportArea::World(WorldPlane::Underworld), 50, 60),
            (0, 0xff, 50, 60),
        ),
    ] {
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(17, 0, 15, 15);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
        write_empty_ool_mirrors(&dir);
        let mut state = britannia(5, 5);
        teleport(&mut state, &dir, want);
        assert_eq!(state.current_save_location(), Some(location));
        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let options = load_play_options_from_save(&dir).unwrap();
        let (target, _, _) = teleport_destination(want).unwrap();
        assert_eq!(options.target, target, "{want:?}");
        assert_eq!(options.start, Some((want.x, want.y)), "{want:?}");
        let _ = fs::remove_dir_all(dir);
    }
}

/// A town exit after a teleport uses the location's fixed plane
/// coordinate, as a direct entry does.
#[test]
fn a_town_exit_follows_the_direct_entry_rule() {
    let dir = debug_game_dir();
    let mut state = world_state(open_world_grid(), 10, 20);
    teleport(&mut state, &dir, at(castle(0), 0, 1));
    assert!(state.return_world.is_none());
    assert_eq!(
        state
            .step_with_game_dir(Direction::West, Some(&dir))
            .unwrap(),
        MoveOutcome::Observed
    );
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (86, 107));
    let _ = fs::remove_dir_all(dir);
}

/// A dungeon exit after a teleport uses the published surface coordinate,
/// on foot, as a direct entry does.
#[test]
fn a_dungeon_exit_follows_the_direct_entry_rule() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(33).unwrap();
    let mut state = britannia(5, 5);
    state.player.transport = HORSE;
    state.sync_player_object();
    teleport(&mut state, &dir, at(dungeon(0), 1, 1));
    assert!(state.return_world.is_none());
    state.grid[dungeon_cell_index(0, 1, 1)] = 0x10;
    assert_eq!(
        state.climb(&dir, ClimbIntent::Up).unwrap(),
        MoveOutcome::Transition(AreaTransition::ExitedDungeonToWorldPlane {
            scene,
            plane: WorldPlane::Britannia,
        })
    );
    assert_eq!((state.player.x, state.player.y), (240, 73));
    assert_eq!(state.player.transport, TransportState::Foot);
    let _ = fs::remove_dir_all(dir);
}

/// D7.9 in another area: a target the scene load itself refuses as a start
/// cell is nudged on the scratch state like any blocked cell, and the
/// nudged plan commits. An area with no open cell near the target refuses
/// with `NoOpenCell`, never `LoadFailed`.
#[test]
fn a_blocked_target_in_another_area_is_nudged() {
    let dir = debug_game_dir();
    let mut castle_dat = vec![16u8; 16 * TOWN_GRID_BYTES];
    castle_dat[10 * 32 + 10] = BRIT_DEEP_WATER_TILE;
    castle_dat[9 * 32 + 9] = BRIT_DEEP_WATER_TILE;
    fs::write(dir.join("CASTLE.DAT"), &castle_dat).unwrap();
    let mut state = britannia(5, 5);
    let plan = plan_teleport(&state, &dir, at(castle(0), 10, 10));
    assert_eq!(plan, ready(at(castle(0), 10, 9), true));
    commit_teleport(&mut state, &dir, &plan).unwrap();
    assert_eq!((state.player.x, state.player.y), (10, 9));

    // Every cell within radius 3 is water.
    for y in 7..=13 {
        for x in 7..=13 {
            castle_dat[y * 32 + x] = BRIT_DEEP_WATER_TILE;
        }
    }
    fs::write(dir.join("CASTLE.DAT"), &castle_dat).unwrap();
    let state = britannia(5, 5);
    assert_eq!(
        plan_teleport(&state, &dir, at(castle(0), 10, 10)),
        TeleportPlan::Refused(TeleportRefusal::NoOpenCell)
    );

    // An all-water Britannia, reached from a town.
    write_britannia_world_files(&dir, BRIT_DEEP_WATER_TILE);
    let town = test_state(open_grid(), 20, 20);
    assert_eq!(
        plan_teleport(
            &town,
            &dir,
            at(TeleportArea::World(WorldPlane::Britannia), 7, 7)
        ),
        TeleportPlan::Refused(TeleportRefusal::NoOpenCell)
    );
    let _ = fs::remove_dir_all(dir);
}

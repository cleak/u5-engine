#[test]
fn routed_world_k_plane_transition_does_not_retrigger_reciprocal_landing_row() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 11 20 UNDERWORLD 30 40\nUNDERWORLD 30 40 BRITANNIA 11 20\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(11, 20)] = 0x0c;
    let mut state = britannia_state(grid, 10, 20);
    state.climbing_gear = 1;
    state.player.facing = Direction::East;

    assert!(
        state
            .handle_top_down_key_with_inline('K', &dir, None, None, None, None)
            .unwrap()
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((state.player.x, state.player.y), (30, 40));
    assert_eq!(state.turn, 1);
    // `doors-and-z-transitions.md §9`: a clean climb prints nothing, and
    // `RETRACTIONS.md` R320 removed the invented narration the sidecar plane
    // transition used to add, so nothing at all reaches the slot here.
    assert!(!state.message.contains("F-A-L-L-S"));
    assert!(!state.message.contains("Ascended from the underworld"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_k_low_climb_stat_falls_but_still_moves() {
    let mut grid = open_world_grid();
    grid[world_cell_index(11, 20)] = 0x0c;
    let mut state = world_state(grid, 10, 20);
    state.climbing_gear = 1;
    state.player.facing = Direction::East;
    state.party = vec![PartyMember {
        slot: 0,
        class_byte: b'A',
        status: b'G',
        climb_stat: 0,
        mana: 8,
        hp: 10,
        max_hp: 20,
        level: 8,
    }];
    state.prng_state = 0;
    let mut expected_prng = state.prng_state;
    let _stat_roll = u5_prng_range_u16(&mut expected_prng, 1, 30);
    let expected_damage = u5_prng_range_u16(&mut expected_prng, 1, 5) as u16;

    assert_eq!(
        state.klimb_command(Path::new("")).unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (11, 20));
    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].hp, 10 - expected_damage);
    assert_eq!(state.prng_state, expected_prng);
    assert_eq!(state.party[0].status, b'G');
    // `doors-and-z-transitions.md §9`: one failed roll prints one `Fell!`.
    assert_eq!(state.message, "Fell!");
}

#[test]
fn world_k_skips_dead_or_ashes_members_for_fall_checks() {
    let mut grid = open_world_grid();
    grid[world_cell_index(11, 20)] = 0x0c;
    let mut state = world_state(grid, 10, 20);
    state.climbing_gear = 1;
    state.player.facing = Direction::East;
    state.party = vec![
        PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'D',
            climb_stat: 0,
            mana: 8,
            hp: 10,
            max_hp: 20,
            level: 8,
        },
        PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'A',
            climb_stat: 0,
            mana: 8,
            hp: 9,
            max_hp: 20,
            level: 8,
        },
    ];

    assert_eq!(
        state.klimb_command(Path::new("")).unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (11, 20));
    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].hp, 10);
    assert_eq!(state.party[1].hp, 9);
    // `doors-and-z-transitions.md §9`: no member fell, so nothing is printed.
    assert!(state.message.is_empty());
}

#[test]
fn world_enter_ignores_retired_location_entry_y_sidecar() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 CASTLE:0 7 0x15 CASTLE\n",
    )
    .unwrap();
    fs::write(dir.join(LOCATION_ENTRY_Y_TABLE_FILE), "CASTLE:0 7\n").unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 0x15;
    let mut state = britannia_state(grid, 10, 20);

    assert_eq!(
        state.enter_current_location(&dir).unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );

    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert_eq!((state.player.x, state.player.y), (15, 30));
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (15, 30)
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn town_exit_uses_clean_location_table_when_no_return_snapshot() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 CASTLE:0\n",
    )
    .unwrap();
    let mut state = test_state(open_grid(), 0, 3);

    assert_eq!(
        state
            .step_with_game_dir(Direction::West, Some(&dir))
            .unwrap(),
        MoveOutcome::Observed
    );
    assert_eq!(
        handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (10, 20));
    assert_eq!(state.active_objects[0].z, 0);
    assert_eq!(state.grid[world_cell_index(10, 20)], 5);
    // `doors-and-z-transitions.md §12.1` accepted town-family exit.
    assert!(state.message.contains(TOWN_EXIT_ACCEPTED_NARRATION));
    assert!(state.message.ends_with(TOWN_EXIT_TO_BRITANNIA_NARRATION));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_surface_reset_uses_clean_location_table_when_no_return_snapshot() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "UNDERWORLD 10 20 DUNGEON:0\n",
    )
    .unwrap();
    // dungeon-mode.md §13: the up arm at level zero is a level edge, and
    // every exit runs the one shared surface-reset contract of §13.2.
    // (The plain pit `0x60` used to stand in for this route here; that
    // bypass is withdrawn - a pit is an ordinary descent.)
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x10;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert!(state.handle_dungeon_key('k', &dir).unwrap());

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (10, 20));
    assert_eq!(state.active_objects[0].z, 0);
    assert_eq!(state.message, DUNGEON_EXIT_TO_BRITANNIA_NARRATION);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn entering_a_location_keeps_the_frontend_paced_combat_flag() {
    // A scene entry rebuilds the whole `PlayState`, so every frontend
    // presentation flag defaults off unless it is carried over. The graphical
    // shell sets `pace_combat_presentations` once at bootstrap: dropping it
    // here left a fight entered after any location transition resolving a
    // whole sixteen-actor round inside one host frame.
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 7;
    let mut state = world_state(grid, 10, 20);
    state.pace_combat_presentations = true;

    assert_eq!(
        state
            .enter_world_target(&dir, WorldPlane::Underworld, PlayTarget::Town(scene), true)
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );

    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert!(state.pace_combat_presentations);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn debug_enter_town_exit_uses_fixed_plane_and_canonical_object_mirror() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 7;
    let mut state = world_state(grid, 10, 20);
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 11,
        y: 20,
        z: -1,
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(
        state
            .enter_world_target(&dir, WorldPlane::Underworld, PlayTarget::Town(scene), true)
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );
    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert!(state.return_world.is_none());
    let persisted_underworld =
        decode_full_ool_plane_table(&fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap()).unwrap();
    assert_eq!(persisted_underworld[1].x, 11);
    assert_eq!(persisted_underworld[1].y, 20);

    state.player.x = 0;
    state.player.y = 1;
    assert_eq!(
        state
            .step_with_game_dir(Direction::West, Some(&dir))
            .unwrap(),
        MoveOutcome::Observed
    );
    assert_eq!(
        handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (86, 107));
    assert_eq!(state.world_object_at(11, 20), None);
    assert_eq!(state.turn, 0);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn return_world_restore_repairs_missing_player_slot_without_shifting_objects() {
    let mut state = test_state(open_grid(), 0, 0);
    let vehicle = ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 12,
        y: 20,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x22,
        aux1: 3,
        aux3: 4,
    };
    state.return_world = Some(WorldReturn {
        plane: WorldPlane::Underworld,
        x: 10,
        y: 20,
        transport: TransportState::Foot,
        sail_cadence: 0,
        grid: open_world_grid(),
        active_objects: vec![
            ActiveObject {
                type_byte: 192,
                tile: 192,
                x: 1,
                y: 1,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            },
            ActiveObject::empty(),
            vehicle,
        ],
        pending_vehicle: None,
    });

    assert!(state.restore_return_world());

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((state.player.x, state.player.y), (10, 20));
    assert_eq!(state.active_objects.len(), 3);
    assert_eq!(
        state.active_objects[0],
        ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 10,
            y: 20,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }
    );
    assert!(state.active_objects[1].is_empty());
    assert_eq!(state.active_objects[2], vehicle);
}

#[test]
fn debug_enter_town_preserves_ship_marker_without_a_return_snapshot() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: true,
        hull: 0,
        skiffs: 0,
    };
    let mut state = world_state(open_world_grid(), 10, 20);
    state.player.transport = transport;
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = QUICKNESS_ACTIVE_EFFECT_DURATION;
    state.sail_cadence = 1;
    state.sync_player_object();
    assert_eq!(
        state
            .enter_world_target(&dir, WorldPlane::Underworld, PlayTarget::Town(scene), true)
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );
    assert!(state.return_world.is_none());
    assert_eq!(state.player.transport, transport);
    assert_eq!(
        state.active_effect_timing_status(),
        TimingStatusTag::HalfTime
    );
    state.grid[31 * 32 + 31] = BRIT_DEEP_WATER_TILE;

    state.player.x = 0;
    state.player.y = 1;
    assert_eq!(
        state
            .step_with_game_dir(Direction::West, Some(&dir))
            .unwrap(),
        MoveOutcome::Observed
    );
    assert_eq!(
        handle_play_key_input(&mut state, 'Y', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 3,
            tile: FIRST_PLAYABLE_FRIGATE_TILE + 3,
            sails_hoisted: true,
            hull: 0,
            skiffs: 0,
        }
    );
    assert_eq!(
        state.active_effect_timing_status(),
        TimingStatusTag::HalfTime
    );
    assert_eq!(state.active_effect_tag, Some(QUICKNESS_ACTIVE_EFFECT_TAG));
    assert_eq!(state.sail_cadence, 0);
    assert_eq!(
        state.active_objects[0].tile,
        TRANSPORT_MARKER_SHIP_HOISTED_FIRST + 3
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn debug_enter_dungeon_surface_reset_uses_published_plane_coordinate_and_foot() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(33).unwrap();
    write_save_template_and_empty_overlays(&dir, 0, 0xff, 10, 20);
    fs::write(
        dir.join(UNDER_DAT_FILENAME),
        vec![BRIT_DEEP_WATER_TILE; UNDER_DAT_LEN],
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 10, 20);
    let transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull: 0,
        skiffs: 0,
    };
    state.player.transport = transport;
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = QUICKNESS_ACTIVE_EFFECT_DURATION;
    state.sail_cadence = 1;
    state.sync_player_object();
    assert_eq!(
        state
            .enter_world_target(
                &dir,
                WorldPlane::Underworld,
                PlayTarget::Dungeon(scene),
                true,
            )
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredDungeon(scene))
    );
    assert_eq!(state.area, Area::Dungeon { scene, level: 7 });
    assert_eq!(
        state
            .return_world
            .as_ref()
            .map(|ret| (ret.transport, ret.sail_cadence)),
        Some((transport, 1))
    );
    assert_eq!(
        state.active_effect_timing_status(),
        TimingStatusTag::HalfTime
    );

    // An up ladder on level zero is the published climb-out route
    // (dungeon-mode.md §13); the withdrawn `0x60` pit bypass is not.
    state.area = Area::Dungeon { scene, level: 0 };
    state.player.x = 1;
    state.player.y = 1;
    state.sync_player_object();
    state.grid[dungeon_cell_index(0, 1, 1)] = 0x10;
    assert_eq!(
        state.climb(&dir, ClimbIntent::Up).unwrap(),
        MoveOutcome::Transition(AreaTransition::ExitedDungeonToWorldPlane {
            scene,
            plane: WorldPlane::Britannia,
        })
    );
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (240, 73));
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(
        state.active_effect_timing_status(),
        TimingStatusTag::HalfTime
    );
    assert_eq!(state.active_effect_tag, Some(QUICKNESS_ACTIVE_EFFECT_TAG));
    assert_eq!(state.sail_cadence, 0);
    assert_eq!(state.active_objects[0].tile, PLAYER_TILE);

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(options.target, PlayTarget::World(WorldPlane::Britannia));
    assert_eq!(options.start, Some((240, 73)));
    assert_eq!(options.transport, TransportState::Foot);
    let reloaded = PlayState::load_scene(&dir, options).unwrap();
    assert_eq!(
        reloaded.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((reloaded.player.x, reloaded.player.y), (240, 73));
    assert_eq!(reloaded.player.transport, TransportState::Foot);
    assert!(reloaded.return_world.is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn debug_enter_underworld_non_doom_dungeon_allows_trigger_seed_and_faces_west() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(33).unwrap();
    let mut dungeon_dat = vec![0; DUNGEON_DAT_LEN];
    dungeon_dat[dungeon_cell_index(7, 7, 7)] = 0xf0;
    fs::write(dir.join("DUNGEON.DAT"), dungeon_dat).unwrap();
    let mut state = world_state(open_world_grid(), 10, 20);
    assert_eq!(
        state
            .enter_world_target(
                &dir,
                WorldPlane::Underworld,
                PlayTarget::Dungeon(scene),
                true,
            )
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredDungeon(scene))
    );

    assert_eq!(state.area, Area::Dungeon { scene, level: 7 });
    assert_eq!((state.player.x, state.player.y), (7, 7));
    assert_eq!(state.player.facing, Direction::West);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_start_rejects_non_seed_trigger_cell() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(7, 7, 7)] = 0xf0;
    grid[dungeon_cell_index(7, 6, 7)] = 0xf0;

    assert!(validate_dungeon_start(&grid, scene, 7, (7, 7)).is_ok());
    assert!(validate_dungeon_start(&grid, scene, 7, (6, 7)).is_err());
}

#[test]
fn dungeon_movement_accepts_passage_and_blocks_walls() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(1, 2, 1)] = 0xb0;
    grid[dungeon_cell_index(1, 1, 2)] = 0xe0;
    let mut state = dungeon_state(grid, 1, 0, 0);

    assert_eq!(state.step(Direction::South), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (0, 1));
    assert_eq!(state.active_objects[0].z, 1);
    assert_eq!(state.turn, 1);

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.turn, 2);
    assert_eq!(state.message, "Blocked!");

    assert_eq!(state.step(Direction::South), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (1, 2));
    assert_eq!(state.turn, 3);
    assert_eq!(state.message, "");
}

#[test]
fn dungeon_back_step_rejects_room_families_but_allows_e_variant() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 0, 1)] = 0xa0;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.player.facing = Direction::East;

    assert!(state.handle_dungeon_key('s', Path::new("")).unwrap());
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.player.facing, Direction::East);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, "Blocked!");

    state.grid[dungeon_cell_index(0, 0, 1)] = 0xf0;
    assert!(state.handle_dungeon_key('s', Path::new("")).unwrap());
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.player.facing, Direction::East);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, "Blocked!");

    state.grid[dungeon_cell_index(0, 0, 1)] = 0xe0;
    assert!(state.handle_dungeon_key('s', Path::new("")).unwrap());
    assert_eq!((state.player.x, state.player.y), (0, 1));
    assert_eq!(state.player.facing, Direction::East);
    assert_eq!(state.turn, 1);
    // An accepted step prints no result line, so the transcript ends on the
    // move's own echo and the compatibility slot keeps the previous line
    // (`commit_command_echo` restores it when a handler printed nothing).
    assert!(
        state
            .message_entries()
            .last()
            .is_some_and(|entry| entry.is_command_echo),
        "{:?}",
        state.message_entries().last()
    );
    assert_eq!(state.message, "Blocked!");
}

#[test]
fn dungeon_fall_trap_drops_one_level_and_marks_destination() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x61;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(
        state.step(Direction::East),
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 1 })
    );

    assert_eq!(state.area, Area::Dungeon { scene, level: 1 });
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.active_objects[0].z, 1);
    assert_eq!(state.grid[dungeon_cell_index(1, 2, 1)], 0x08);
    assert_eq!(state.turn, 1);
}

#[test]
fn dungeon_fall_trap_chain_freshens_the_monster_on_every_accepted_level() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = vec![0x90; DUNGEON_RECORD_LEN];
    grid[dungeon_cell_index(0, 2, 1)] = 0x61;
    grid[dungeon_cell_index(1, 2, 1)] = 0x61;
    grid[dungeon_cell_index(2, 2, 1)] = 0x90;
    let mut state = dungeon_state(grid.clone(), 0, 2, 1);
    let mut expected = dungeon_state(grid, 0, 2, 1);

    expected.area = Area::Dungeon { scene, level: 1 };
    expected.sync_player_object();
    assert!(!expected.setup_dungeon_active_monster_fresh());
    expected.area = Area::Dungeon { scene, level: 2 };
    expected.sync_player_object();
    assert!(!expected.setup_dungeon_active_monster_fresh());

    assert_eq!(
        state
            .resolve_dungeon_fall_trap_transition(scene, 0, 2, 1, None, false)
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 2 })
    );
    assert_eq!(state.prng_state, expected.prng_state);
    assert_eq!(
        state.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT],
        expected.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT]
    );
}

#[test]
fn dungeon_fall_trap_chain_missing_return_metadata_stays_in_dungeon() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(6, 2, 1)] = 0x61;
    grid[dungeon_cell_index(7, 2, 1)] = 0x61;
    let mut state = dungeon_state(grid, 6, 1, 1);

    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);

    assert_eq!(state.area, Area::Dungeon { scene, level: 7 });
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.active_objects[0].z, 7);
    assert_eq!(state.grid[dungeon_cell_index(7, 2, 1)], 0x61);
    assert_eq!(state.turn, 1);
    assert!(
        state
            .message
            .contains("missing clean return-coordinate metadata")
    );
}

#[test]
fn dungeon_fall_trap_chain_uses_underworld_and_ignores_location_row_plane() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(33).unwrap();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 DUNGEON:0\n",
    )
    .unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(6, 2, 1)] = 0x61;
    grid[dungeon_cell_index(7, 2, 1)] = 0x61;
    let mut state = dungeon_state(grid, 6, 1, 1);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::ExitedDungeonToWorldPlane {
            scene,
            plane: WorldPlane::Underworld
        })
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.active_objects[0].z, -1);
    assert_eq!(state.grid[world_cell_index(2, 1)], 5);
    assert!(state.message.contains("cleared dungeon scene"));
    assert!(state.message.contains("trap-chain coordinate (2, 1)"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_k_dexterity_equal_to_roll_does_not_fall() {
    let mut grid = open_world_grid();
    grid[world_cell_index(11, 20)] = 0x0c;
    let mut state = world_state(grid, 10, 20);
    state.climbing_gear = 1;
    state.player.facing = Direction::East;
    state.prng_state = 0;
    let mut expected_prng = state.prng_state;
    let roll = u5_prng_range_u16(&mut expected_prng, 1, 30) as u8;
    state.party = vec![PartyMember {
        slot: 0,
        class_byte: b'A',
        status: b'G',
        climb_stat: roll,
        mana: 8,
        hp: 10,
        max_hp: 20,
        level: 8,
    }];

    assert_eq!(
        state.klimb_command(Path::new("")).unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (11, 20));
    assert_eq!(state.party[0].hp, 10);
    assert_eq!(state.prng_state, expected_prng);
    assert!(state.message.is_empty());
}

#[test]
fn dungeon_fall_trap_chain_restores_snapshot_grid_without_exterior_coordinate_reset() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(6, 2, 1)] = 0x61;
    grid[dungeon_cell_index(7, 2, 1)] = 0x61;
    let mut state = dungeon_state(grid, 6, 1, 1);
    let mut world_grid = open_world_grid();
    world_grid[world_cell_index(2, 1)] = 7;
    world_grid[world_cell_index(10, 20)] = 9;
    state.return_world = Some(WorldReturn {
        plane: WorldPlane::Underworld,
        x: 10,
        y: 20,
        transport: TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        },
        sail_cadence: 3,
        grid: world_grid,
        active_objects: vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 10,
            y: 20,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        pending_vehicle: None,
    });

    assert_eq!(
        state.step(Direction::East),
        MoveOutcome::Transition(AreaTransition::ExitedDungeonToWorldPlane {
            scene,
            plane: WorldPlane::Underworld
        })
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.active_effect_timing_status(), TimingStatusTag::Normal);
    assert_eq!(state.sail_cadence, 0);
    assert_eq!(state.grid[world_cell_index(2, 1)], 7);
    assert_eq!(state.grid[world_cell_index(10, 20)], 9);
    assert_eq!(state.active_objects[0].x, 2);
    assert_eq!(state.active_objects[0].y, 1);
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("trap-chain coordinate (2, 1)"));
}

#[test]
fn dungeon_bomb_trap_marks_cell_without_level_change() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x62;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(
        state.area,
        Area::Dungeon {
            scene: DungeonScene::new(33).unwrap(),
            level: 0,
        }
    );
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x6a);
    assert_eq!(state.turn, 1);
}

#[test]
fn dungeon_marked_trap_variants_keep_trap_movement_effects() {
    let scene = DungeonScene::new(33).unwrap();
    let mut fall_grid = open_dungeon_record();
    fall_grid[dungeon_cell_index(0, 2, 1)] = 0x69;
    let mut fall = dungeon_state(fall_grid, 0, 1, 1);

    assert_eq!(
        fall.step(Direction::East),
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 1 })
    );

    assert_eq!(fall.area, Area::Dungeon { scene, level: 1 });
    assert_eq!((fall.player.x, fall.player.y), (2, 1));
    assert_eq!(fall.grid[dungeon_cell_index(0, 2, 1)], 0x61);
    assert_eq!(fall.grid[dungeon_cell_index(1, 2, 1)], 0x08);
    assert_eq!(fall.turn, 1);

    let mut bomb_grid = open_dungeon_record();
    bomb_grid[dungeon_cell_index(0, 2, 1)] = 0x6a;
    let mut bomb = dungeon_state(bomb_grid, 0, 1, 1);

    assert_eq!(bomb.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((bomb.player.x, bomb.player.y), (2, 1));
    assert_eq!(bomb.area, Area::Dungeon { scene, level: 0 });
    assert_eq!(bomb.grid[dungeon_cell_index(0, 2, 1)], 0x6a);
    assert_eq!(bomb.turn, 1);
}

#[test]
fn consumed_dungeon_action_on_fall_trap_applies_underfoot_fall_after_turn() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x61;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert!(state.handle_dungeon_key('i', Path::new("")).unwrap());

    assert_eq!(state.area, Area::Dungeon { scene, level: 1 });
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.active_objects[0].z, 1);
    assert_eq!(state.grid[dungeon_cell_index(1, 1, 1)], 0x08);
    assert_eq!(state.turn, 1);
    assert!(state.torch_counter > 0, "torch ignited");
    // `dungeon-mode.md §8.1`: the three-line pit group, once per descent
    // step, ending on the six-space splat.
    let lines: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(lines.contains(&"Pit Trap!"));
    assert!(lines.contains(&"Falling..."));
    assert!(lines.contains(&"      ...splat!"));
}

#[test]
fn no_turn_dungeon_action_on_fall_trap_skips_underfoot_fall() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x61;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert!(state.handle_dungeon_key('l', Path::new("")).unwrap());

    assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.active_objects[0].z, 0);
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x61);
    assert_eq!(state.turn, 0);
    assert!(!state.message.contains("pit trap"));
}

#[test]
fn pass_turn_on_dungeon_fall_trap_applies_underfoot_fall_after_turn() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x61;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(
        state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 1 })
    );

    assert_eq!(state.area, Area::Dungeon { scene, level: 1 });
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.grid[dungeon_cell_index(1, 1, 1)], 0x08);
    assert_eq!(state.turn, 1);
    // `dungeon-mode.md §8.1` pit group.
    assert_eq!(state.message, DUNGEON_SPLAT_LINE);
}

#[test]
fn inline_cast_on_dungeon_bomb_trap_marks_underfoot_without_extra_turn() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x62;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
    state.party[0].mana = IN_LOR_COST;
    state.party[0].level = IN_LOR_COST;

    assert_eq!(
        handle_play_key_input(&mut state, 'C', "1IL", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!(state.area, Area::Dungeon { scene, level: 0 });
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x6a);
    assert_eq!(state.turn, 1);
    assert_eq!(state.spell_charges[IN_LOR_SPELL_INDEX], 0);
    assert_eq!(state.party[0].mana, 0);
    // `dungeon-mode.md §8.1`: `Bomb Trap!` then `KABOOM!!`.
    let lines: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(lines.iter().any(|line| line.contains("Light!")));
    assert!(lines.contains(&"Bomb Trap!"));
    assert!(lines.contains(&"KABOOM!!"));
}

#[test]
fn dungeon_poison_field_entry_poison_living_party_without_blocking_movement() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x81;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party = vec![
        PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: 0,
            mana: 8,
            hp: 10,
            max_hp: 20,
            level: 8,
        },
        PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'D',
            climb_stat: 31,
            mana: 8,
            hp: 0,
            max_hp: 20,
            level: 8,
        },
    ];

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].status, b'P');
    assert_eq!(state.party[1].status, b'D');
    // `dungeon-mode.md §8.1`: the field line is the whole of the narration,
    // and it prints before the per-member rolls.
    assert_eq!(state.message, DUNGEON_POISON_FIELD_LINE);
}

#[test]
fn dungeon_generic_energy_field_moves_without_status_damage_or_placeholder() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x84;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].status = b'G';
    state.party[0].hp = 10;

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].status, b'G');
    assert_eq!(state.party[0].hp, 10);
    // `dungeon-mode.md §8.1`, "Any other underfoot byte: nothing".
    assert!(state.message.is_empty());
}

#[test]
fn dungeon_secondary_field_visual_family_is_not_contact_field() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x90;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].status = b'G';
    state.party[0].hp = 10;

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].status, b'G');
    assert_eq!(state.party[0].hp, 10);
    assert_eq!(state.message, "");
}

#[test]
fn dungeon_sleep_field_marker_variant_sets_living_party_asleep_and_clears_cell() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x88;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party = vec![
        PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'P',
            climb_stat: 0,
            mana: 8,
            hp: 10,
            max_hp: 20,
            level: 8,
        },
        PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'A',
            climb_stat: 31,
            mana: 8,
            hp: 10,
            max_hp: 20,
            level: 8,
        },
    ];

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.party[0].status, b'S');
    assert_eq!(state.party[1].status, b'A');
    assert_eq!(
        state.grid[dungeon_cell_index(0, 2, 1)],
        DUNGEON_VISIT_MARKER_BIT
    );
    // `dungeon-mode.md §8.1` sleep field.
    assert_eq!(state.message, DUNGEON_SLEEP_FIELD_LINE);
}

#[test]
fn dungeon_sleep_field_base_variant_clears_to_open_passage() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x80;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].climb_stat = 0;

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x00);
    assert_eq!(state.party[0].status, b'S');
}

#[test]
fn dungeon_fire_and_electric_fields_damage_living_party_members() {
    let mut fire_grid = open_dungeon_record();
    fire_grid[dungeon_cell_index(0, 2, 1)] = 0x82;
    let mut fire = dungeon_state(fire_grid, 0, 1, 1);
    fire.party = vec![
        PartyMember {
            slot: 0,
            class_byte: b'A',
            status: b'G',
            climb_stat: 30,
            mana: 8,
            hp: 10,
            max_hp: 20,
            level: 8,
        },
        PartyMember {
            slot: 1,
            class_byte: b'A',
            status: b'D',
            climb_stat: 30,
            mana: 8,
            hp: 9,
            max_hp: 20,
            level: 8,
        },
    ];
    fire.prng_state = 0;
    let mut expected_fire_prng = fire.prng_state;
    let expected_fire_damage = u5_prng_range_u16(&mut expected_fire_prng, 1, 8) as u8;

    assert_eq!(fire.step(Direction::East), MoveOutcome::Moved);

    assert_eq!(fire.party[0].hp, 10 - expected_fire_damage as u16);
    assert_eq!(fire.party[1].hp, 9);
    assert_eq!(fire.prng_state, expected_fire_prng);
    // `dungeon-mode.md §8.1`: `Fire!!`, two exclamation marks, and no
    // per-member damage narration.
    assert_eq!(fire.message, DUNGEON_FIRE_FIELD_LINE);

    // `dungeon-mode.md §8`: the movement-time electric test "is an exact
    // comparison against `0x83`", so the base byte - not the marker variant -
    // is the case that reacts.
    let mut electric_grid = open_dungeon_record();
    electric_grid[dungeon_cell_index(0, 2, 1)] = 0x83;
    let mut electric = dungeon_state(electric_grid, 0, 1, 1);
    electric.party[0].hp = 10;
    electric.prng_state = 0;
    let mut expected_electric_prng = electric.prng_state;
    let expected_electric_damage = u5_prng_range_u16(&mut expected_electric_prng, 1, 8) as u8;

    assert_eq!(electric.step(Direction::East), MoveOutcome::Moved);

    assert_eq!(electric.party[0].hp, 10 - expected_electric_damage as u16);
    assert_eq!(electric.prng_state, expected_electric_prng);
    assert_eq!((electric.player.x, electric.player.y), (1, 1));
    // `dungeon-mode.md §8.1`: `Ouch!` then `Electric field!`, both printed
    // before the destination-class test.
    assert_eq!(electric.message, DUNGEON_ELECTRIC_FIELD_LINE);
}

/// `dungeon-mode.md §8.1`: "**Byte `0x8B` - the marked electric variant - is
/// inert.** The movement-time test is an exact comparison against `0x83` on
/// the raw cell byte, and the post-action pass has no `0x8B` arm; the byte
/// therefore triggers nothing on either path, no line and no effect."
#[test]
fn dungeon_marked_electric_variant_is_inert_on_contact() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x8b;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].hp = 10;
    state.party[0].status = b'G';
    state.prng_state = 0;

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    // No line: neither the electric pair nor any post-action field line.
    assert_eq!(state.message, "");
    assert!(!state
        .message_transcript
        .iter()
        .any(|entry| entry.text.contains("Ouch!") || entry.text.contains("Electric field!")));
    // No effect: no damage, no status change, no PRNG draw, and the cell
    // byte is left exactly as it was.
    assert_eq!(state.party[0].hp, 10);
    assert_eq!(state.party[0].status, b'G');
    assert_eq!(state.prng_state, 0);
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0x8b);
    // No displacement reversal either - the step completes onto the cell,
    // exactly as any other inert underfoot byte.
    assert_eq!((state.player.x, state.player.y), (2, 1));
}

#[test]
fn dungeon_field_status_gate_uses_dexterity_equality_and_no_clamp() {
    assert!(dungeon_field_status_applies(17, 17));
    assert!(dungeon_field_status_applies(17, 18));
    assert!(!dungeon_field_status_applies(17, 16));
    assert!(!dungeon_field_status_applies(31, 30));
    assert_eq!(DUNGEON_FIELD_STATUS_ROLL_LOW, 1);
    assert_eq!(DUNGEON_FIELD_STATUS_ROLL_HIGH, 30);
}

#[test]
fn electric_field_backstep_wraps_to_contact_then_returns_to_origin() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 7, 0)] = DUNGEON_FIELD_ELECTRIC_BASE;
    let mut state = dungeon_state(grid, 0, 0, 0);
    state.player.facing = Direction::East;
    state.party[0].hp = 20;

    assert_eq!(
        state
            .step_dungeon_back(
                Direction::West,
                -1,
                0,
                DungeonScene::new(33).unwrap(),
                0,
                None,
            )
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!((state.player.x, state.player.y), (0, 0));
    assert_eq!(state.player.facing, Direction::East);
    assert_eq!(state.turn, 1);
}

#[test]
fn consumed_dungeon_action_on_field_applies_underfoot_field_after_turn() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x81;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].status = b'G';
    state.party[0].climb_stat = 0;

    assert!(state.handle_dungeon_key('i', Path::new("")).unwrap());

    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].status, b'P');
    assert_eq!((state.player.x, state.player.y), (1, 1));
    let lines: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(lines.iter().any(|line| line.contains("Ignite torch")));
    assert!(lines.contains(&"Poison!"));
}

#[test]
fn consumed_dungeon_action_on_sleep_field_clears_underfoot_field_after_turn() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x80;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].status = b'G';
    state.party[0].climb_stat = 0;

    assert!(state.handle_dungeon_key('i', Path::new("")).unwrap());

    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].status, b'S');
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x00);
    // `dungeon-mode.md §8.1` sleep field. The consequence line owns the
    // compatibility slot, so the command's own line is checked on the
    // transcript instead.
    let lines: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(lines.iter().any(|line| line.contains("Ignite torch")));
    assert!(lines.contains(&"Sleep spell!"));
}

#[test]
fn no_turn_dungeon_action_on_field_skips_underfoot_field() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x80;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].status = b'G';

    assert!(state.handle_dungeon_key('l', Path::new("")).unwrap());

    assert_eq!(state.turn, 0);
    assert_eq!(state.party[0].status, b'G');
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert!(!state.message.contains("sleep field"));
    assert!(!state.message.contains("asleep"));
    assert_eq!(state.grid[dungeon_cell_index(0, 1, 1)], 0x80);
}

#[test]
fn pass_turn_on_dungeon_fire_field_damages_without_extra_turn() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x82;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.party[0].hp = 20;
    state.prng_state = 0;
    let mut expected_prng = state.prng_state;
    let expected_damage = u5_prng_range_u16(&mut expected_prng, 1, 8) as u8;

    assert_eq!(
        state.pass_turn_with_game_dir(Some(Path::new(""))).unwrap(),
        MoveOutcome::Passed
    );

    assert_eq!(state.turn, 1);
    assert_eq!(state.party[0].hp, 20 - expected_damage as u16);
    assert_eq!(state.prng_state, expected_prng);
    let lines: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(lines.contains(&"Fire!!"));
}

#[test]
fn consumed_dungeon_turn_on_gust_art_does_not_extinguish_underfoot_torch() {
    let dir = debug_game_dir();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x70;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.torch_counter = 5;
    state.light_spell_counter = 5;
    state.visibility_dirty = false;

    assert!(state.handle_dungeon_key('a', &dir).unwrap());

    assert_eq!(state.turn, 1);
    assert_eq!(state.torch_counter, 4);
    assert_eq!(state.light_spell_counter, 4);
    assert!(state.visibility_dirty);
    assert!(state.message.contains("Turned to face"));
    assert!(!state.message.contains("breeze blows out the torch"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn dungeon_fall_landing_on_room_trigger_enters_the_room_without_a_further_turn() {
    // `dungeon-mode.md §8.1`: "If the chain lands within the dungeon on a
    // room-helper or room-trigger cell, dungeon mode immediately runs the
    // same room-entry helper as ordinary underfoot room triggers." This is
    // the only route into Doom's final room, whose level-seven neighbours
    // are all wall cells, so deferring it to the next loop head leaves the
    // endgame handoff behind an extra keypress.
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0x61;
    grid[dungeon_cell_index(1, 2, 1)] = 0xf3;
    let mut state = dungeon_state(grid, 0, 1, 1);

    assert_eq!(
        state.step(Direction::East),
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 1 })
    );

    assert_eq!(state.area, Area::Dungeon { scene, level: 1 });
    assert_eq!((state.player.x, state.player.y), (2, 1));
    // The room helper promotes the resolved trigger to `0xA?` state and keeps
    // the low nibble as the arena slot (§5).
    assert_eq!(state.grid[dungeon_cell_index(1, 2, 1)], 0xa3);
    assert_eq!(state.message, DUNGEON_ROOM_ENTRY_NARRATION);
}

#[test]
fn endgame_elapsed_report_skips_every_zero_component() {
    // `endgame.md §9.4`: each component is "formatted as a decimal number
    // followed by ` year`, ` month` or ` day`, with a trailing `s` when the
    // value is greater than one. **A zero component is skipped entirely**,
    // and the `, ` separator is emitted only when a later component will
    // also be printed".
    assert_eq!(elapsed_time_label(1, 0, 0), "1 year");
    assert_eq!(elapsed_time_label(0, 1, 0), "1 month");
    assert_eq!(elapsed_time_label(0, 0, 1), "1 day");
    assert_eq!(elapsed_time_label(2, 0, 3), "2 years, 3 days");
    assert_eq!(
        elapsed_time_label(2, 3, 4),
        "2 years, 3 months, 4 days"
    );
    // All three zero is the rule applied three times, not a special case.
    assert_eq!(elapsed_time_label(0, 0, 0), "");
}

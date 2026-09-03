#[test]
fn sync_player_object_refreshes_slot_zero_without_treating_actor_bytes_as_duplicates() {
    let mut state = world_state(open_world_grid(), 4, 5);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.active_objects[0] = ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 9,
        y: 9,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x21,
        aux1: 7,
        aux3: 8,
    };
    state.active_objects.push(ActiveObject {
        type_byte: SHADOWLORD_ACTOR_TILE,
        tile: 201,
        x: 1,
        y: 1,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x66,
        aux1: 0x55,
        aux3: 0x77,
    });

    state.sync_player_object();

    assert_eq!(
        state.active_objects[0],
        ActiveObject {
            type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
            tile: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
            x: 4,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x21,
            aux1: 0,
            aux3: 0,
        }
    );
    assert_eq!(
        state.active_objects[1],
        ActiveObject {
            type_byte: SHADOWLORD_ACTOR_TILE,
            tile: 201,
            x: 1,
            y: 1,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x66,
            aux1: 0x55,
            aux3: 0x77,
        }
    );
    assert!(!state.active_objects[1].is_empty());
}

#[test]
fn sync_player_object_recreates_empty_active_object_table() {
    let mut state = dungeon_state(open_dungeon_record(), 3, 2, 4);
    state.active_objects.clear();

    state.sync_player_object();

    assert_eq!(
        state.active_objects,
        vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x: 2,
            y: 4,
            z: 3,
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: 0,
            aux3: 0,
        }]
    );
}

#[test]
fn movement_blocks_impassable_tiles_but_still_spends_the_town_turn() {
    let mut grid = open_grid();
    grid[32 + 2] = 0x0c;
    let mut state = test_state(grid, 1, 1);
    state.ambient_light = FULL_DAYLIGHT;
    state.visibility_dirty = false;

    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    // `town-mode.md §15`: a refused town step "Consumes one normal town
    // turn: advance the clock by one minute, run underfoot/post-action
    // processing, and run one NPC schedule step". This assertion used to
    // read `state.turn, 0` with no citation behind it; `combat.md §11`
    // ("A blocked step re-prompts at no cost") is the arena, not here.
    assert_eq!(state.turn, 1);
    // The consumed turn runs the ordinary epilogue, so the animator
    // advances exactly as it does behind an accepted step; only the
    // party coordinate and the viewport stay put.
    assert_eq!(state.animation.frame, 1);
    assert!(!state.visibility_dirty);
}

#[test]
fn movement_ignores_optional_passability_bitmap_for_promoted_transport() {
    let mut grid = open_grid();
    grid[32 + 2] = 0x0c;
    let mut state = test_state(grid, 1, 1);
    state.passability = Some(passability_with_tiles(&[0x0c]));

    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);

    assert_eq!((state.player.x, state.player.y), (1, 1));
    // `town-mode.md §15`: a refused town step still costs one town turn.
    assert_eq!(state.turn, 1);
}

#[test]
fn movement_blocks_same_floor_active_object_but_still_spends_the_town_turn() {
    let mut state = test_state(open_grid(), 1, 1);
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
    assert_eq!((state.player.x, state.player.y), (1, 1));
    // `town-mode.md §15` scores the ordinary town refusal at "one normal
    // town turn". The table names the terrain arm; this occupancy arm is
    // the same wrapper's other refusal, reaching the same blocked-feedback
    // tail, so it is charged alike. (`audio.md §7.4`'s "share one tail" is
    // about the beep and the type-ahead flush, not the clock.) Open spec
    // question - see `turn-clock-wind-report.md`.
    assert_eq!(state.turn, 1);
}

#[test]
fn movement_ignores_other_floor_active_object() {
    let mut state = test_state(open_grid(), 1, 1);
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 2,
        y: 1,
        z: 1,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);
    assert_eq!((state.player.x, state.player.y), (2, 1));
    assert_eq!(state.turn, 1);
}

#[test]
fn movement_out_of_bounds_starts_prompt_before_return_resolution() {
    let scene = Scene::new(0x11).unwrap();
    let mut state = test_state(open_grid(), 0, 3);

    assert_eq!(state.step(Direction::West), MoveOutcome::Observed);
    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert_eq!((state.player.x, state.player.y), (0, 3));
    assert_eq!(state.active_objects[0].z, 0);
    assert_eq!(state.turn, 0);
    assert_eq!(state.message, TOWN_EXIT_PROMPT);
}

#[test]
fn world_movement_wraps_and_advances_outdoor_time() {
    let mut state = world_state(open_world_grid(), 255, 0);
    state.clock = GameClock::new(12, 58).unwrap();

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (0, 0));
    assert_eq!(state.active_objects[0].x, 0);
    assert_eq!(state.active_objects[0].z, -1);
    assert_eq!(state.clock, GameClock::new(13, 0).unwrap());
    assert_eq!(state.turn, 1);
}

#[test]
fn skiff_world_movement_uses_normal_time_without_a_quickness_effect() {
    let mut state = world_state(vec![1; WORLD_CELLS], 0, 0);
    state.player.transport = TransportState::Skiff {
        type_byte: 176,
        tile: 176,
    };
    state.sync_player_object();
    state.clock = GameClock::new(12, 58).unwrap();

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.clock, GameClock::new(13, 0).unwrap());
    assert_eq!(state.turn, 1);
}

#[test]
fn quickness_effect_halves_world_time_without_skiff() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = QUICKNESS_ACTIVE_EFFECT_DURATION;
    state.clock = GameClock::new(12, 58).unwrap();
    state.torch_counter = 3;
    state.light_spell_counter = 2;

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.clock, GameClock::new(12, 59).unwrap());
    assert_eq!(state.torch_counter, 2);
    assert_eq!(state.light_spell_counter, 1);
    assert_eq!(state.turn, 1);
}

#[test]
fn negate_time_effect_skips_minutes_and_light_but_runs_cleanup() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    state.clock = GameClock::new(12, 58).unwrap();
    state.torch_counter = 5;
    state.light_spell_counter = 4;
    state.visibility_dirty = false;

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.clock, GameClock::new(12, 58).unwrap());
    assert_eq!(state.torch_counter, 5);
    assert_eq!(state.light_spell_counter, 4);
    // Both counters burn, so the brighter spell floor wins (#83).
    assert_eq!(state.ambient_light, LIGHT_SPELL_FLOOR);
    assert!(state.visibility_dirty);
    // `animation.md §13.1`: "For the effect's full duration nothing advances:
    // no water rotation, no fire flicker, no fountain, no banner, no clock or
    // bellows, no object animation ..." and `magic.md §8` says "the
    // overworld epilogue returns before animating anything". A consumed turn
    // under Negate Time therefore leaves the shared phase counter alone; an
    // earlier revision asserted `1` here and pinned the withdrawn behaviour.
    assert_eq!(state.animation.frame, 0);
    assert_eq!(state.water_scroll.phase, 0);
    assert_eq!(state.turn, 1);
}

#[test]
fn world_movement_blocks_impassable_tiles_without_turn() {
    let mut grid = open_world_grid();
    grid[world_cell_index(1, 0)] = 0x0c;
    let mut state = world_state(grid, 0, 0);

    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);

    assert_eq!((state.player.x, state.player.y), (0, 0));
    // `movement.md §8`: "Actions that fail before commit do not move the
    // actor. Whether they consume a turn is owned by the caller; ordinary
    // rejected movement is generally a consumed movement attempt only when
    // the mode explicitly treats the bump or attack as a turn-taking
    // action." Nothing in `overworld.md` makes the outdoor bump such an
    // action - unlike `town-mode.md §15`, which does - so the refused
    // overworld step stays free. See the open spec question in
    // `turn-clock-wind-report.md`.
    assert_eq!(state.clock, GameClock::default());
    assert_eq!(state.turn, 0);
}

#[test]
fn world_movement_blocks_active_object_without_turn() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.active_objects.push(ActiveObject {
        type_byte: 170,
        tile: 170,
        x: 1,
        y: 0,
        z: -1,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);

    assert_eq!((state.player.x, state.player.y), (0, 0));
    assert_eq!(state.clock, GameClock::default());
    assert_eq!(state.turn, 0);
    // `audio.md §7.4`: a refusal by a blocking object prints the shared
    // refusal line. The arena/class selection it used to narrate is
    // asserted directly, where no player-facing string has to carry it.
    assert_eq!(state.message, "Blocked!");
    let note = state
        .terrain_encounter_note(None, WorldPlane::Britannia, state.active_objects[1])
        .unwrap();
    assert!(note.contains("selected BRIT.CBT arena 2"), "{note}");
    assert!(note.contains("base class Mimic (26)"), "{note}");
    assert!(!note.contains("out of scope"), "{note}");
}

#[test]
fn world_movement_into_combat_class_object_selects_brit_cbt_arena() {
    let dir = debug_game_dir();
    let record = synthetic_combat_arena_record();
    fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
    let mut state = world_state(open_world_grid(), 0, 0);
    state.active_objects.push(ActiveObject {
        type_byte: 0xc0,
        tile: 0xc0,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Used
    );

    assert_eq!((state.player.x, state.player.y), (0, 0));
    assert_eq!(state.turn, 1);
    assert!(state.combat_active);
    assert_eq!(state.pending_combat_terrain_trigger_slot, Some(1));
    assert_eq!(state.message, combat_banner_line());
    // `combat.md §5`: monster descriptors start at index six, but their
    // active-object records "continue from the first record left free by
    // the seated party", so "the descriptor's active-object link byte is
    // the authoritative pairing ... an engine should follow the link
    // rather than assume the two indexes are equal." The stock roster is
    // one live member, so the first monster's record is one.
    let first_monster = state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS];
    let first_monster_record = usize::from(first_monster.active_object_slot);
    assert_eq!(first_monster_record, 1);
    assert_eq!(state.active_objects[first_monster_record].tile, 0xc0);
    assert_eq!(
        (
            state.active_objects[first_monster_record].x,
            state.active_objects[first_monster_record].y
        ),
        (0, 15)
    );
    assert!(!state.message.contains("out of scope"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn board_vehicle_uses_facing_active_object_and_clears_parked_slot() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.facing = Direction::East;
    let parked = ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 77,
        aux3: 2,
    };
    state.active_objects.push(parked);

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 77,
            skiffs: 2,
        }
    );
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(
        state.active_objects[0].tile,
        TRANSPORT_MARKER_SHIP_FURLED_FIRST
    );
    assert_eq!(
        state.active_objects[1],
        ActiveObject {
            type_byte: 0,
            ..parked
        }
    );
    assert!(state.active_objects[1].is_empty());
    assert!(state.world_object_at(1, 0).is_none());
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "Boarded ship.");
}

#[test]
fn board_vehicle_removal_is_written_to_the_live_saved_gam_table() {
    let dir = debug_game_dir();
    fs::write(dir.join("INIT.GAM"), saved_game_seed_bytes(0, 0xff, 0, 0)).unwrap();
    write_empty_ool_mirrors(&dir);
    fs::write(
        dir.join(UNDER_DAT_FILENAME),
        vec![BRIT_DEEP_WATER_TILE; UNDER_DAT_LEN],
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 77,
        aux3: 2,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert!(state.active_objects[1].is_empty());

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved_ool = fs::read(dir.join("SAVED.OOL")).unwrap();
    let underworld = decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..SAVED_OOL_LEN]).unwrap();
    assert!(underworld[0].is_empty());

    let saved_gam = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(
        saved_gam[SAVE_TRANSPORT_MARKER_OFFSET],
        TRANSPORT_MARKER_SHIP_FURLED_FIRST
    );
    let saved_active = decode_active_object_table(
        &saved_gam[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
        "SAVED.GAM",
    )
    .unwrap();
    assert!(saved_active[0].is_empty());

    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(options.target, PlayTarget::World(WorldPlane::Underworld));
    assert_eq!(options.start, Some((0, 0)));
    assert_eq!(
        options.transport,
        TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull: 77,
            skiffs: 2,
        }
    );
    assert!(options.saved_active_objects.as_ref().unwrap()[0].is_empty());
    let reloaded = PlayState::load_scene(&dir, options).unwrap();
    assert_eq!(
        reloaded.player.transport,
        TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull: 77,
            skiffs: 2,
        }
    );
    assert!(reloaded.active_objects[1].is_empty());
    assert!(reloaded.world_object_at(1, 0).is_none());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn board_and_exit_skiff_do_not_change_shared_magic_effect() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 20;
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 176,
        tile: 176,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

    assert_eq!(
        state.player.transport,
        TransportState::Skiff {
            type_byte: 176,
            tile: 176,
        }
    );
    assert_eq!(state.active_effect_tag, Some(PROTECTION_ACTIVE_EFFECT_TAG));
    assert_eq!(state.active_effect_counter, 19);

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.active_effect_tag, Some(PROTECTION_ACTIVE_EFFECT_TAG));
    assert_eq!(state.active_effect_counter, 18);
}

#[test]
fn board_ship_accepts_waterborne_transport_state() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.transport = TransportState::Skiff {
        type_byte: 176,
        tile: 176,
    };
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = QUICKNESS_ACTIVE_EFFECT_DURATION;
    state.player.facing = Direction::East;
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 0,
        }
    );
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(
        state.active_objects[0].tile,
        TRANSPORT_MARKER_SHIP_FURLED_FIRST
    );
    assert!(state.active_objects[1].is_empty());
    assert_eq!(state.active_effect_tag, Some(QUICKNESS_ACTIVE_EFFECT_TAG));
    assert_eq!(
        state.active_effect_counter,
        QUICKNESS_ACTIVE_EFFECT_DURATION - 1
    );
    assert_eq!(state.turn, 1);
    assert_eq!(
        state.message,
        format!("Boarded ship. {SHIP_BADLY_DAMAGED_WARNING} {SHIP_NO_SKIFFS_WARNING}")
    );
}

#[test]
fn board_ship_accepts_public_object_byte_and_preserves_save_marker() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1,
        tile: 0,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 88,
        aux3: 1,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1,
            tile: FIRST_PLAYABLE_FRIGATE_TILE + 1,
            sails_hoisted: false,
            hull: 88,
            skiffs: 1,
        }
    );
    assert_eq!(
        state.player.transport.save_marker(),
        TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1
    );
    assert_eq!(
        state.active_objects[0].tile,
        TRANSPORT_MARKER_SHIP_FURLED_FIRST + 1
    );
    assert!(state.active_objects[1].is_empty());
}

#[test]
fn vehicle_directional_step_refreshes_transport_marker_and_player_tile() {
    let mut state = world_state(open_world_grid(), 4, 4);
    state.player.transport = TransportState::Carpet {
        type_byte: TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
        tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
    };
    state.sync_player_object();

    assert_eq!(
        state
            .step_with_game_dir(Direction::West, None)
            .expect("world carpet step is in-memory"),
        MoveOutcome::Moved
    );

    assert_eq!(
        state.player.transport.save_marker(),
        TRANSPORT_MARKER_MAGIC_CARPET_LAST
    );
    assert_eq!(
        state.active_objects[0].tile,
        TRANSPORT_MARKER_MAGIC_CARPET_LAST
    );
}

#[test]
fn board_ship_accepts_carpet_north_east_and_stows_carpet() {
    for marker in [
        TRANSPORT_MARKER_MAGIC_CARPET_FIRST,
        TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 1,
    ] {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.transport = TransportState::Carpet {
            type_byte: marker,
            tile: transport_visual_tile_for_marker(marker).unwrap(),
        };
        state.player.facing = Direction::East;
        state.sync_player_object();
        state.active_objects.push(ActiveObject {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
            tile: 0,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 77,
            aux3: 2,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert_eq!(state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX], 1);
        assert!(matches!(
            state.player.transport,
            TransportState::Ship { .. }
        ));
        assert_eq!(state.message, "Boarded ship.");
    }
}

#[test]
fn there_are_no_south_or_west_carpet_markers_to_refuse() {
    // vehicles.md §2/§6: the carpet has TWO frames only, 0x14
    // (east) and 0x15 (west). §6 explicitly withdraws the earlier
    // reading that called them "the north/east carpet markers"
    // with south/west counterparts sitting outside the ship
    // boarding precondition - "the two carpet-compatible values
    // 0x14 and 0x15 are the only carpet marker values", so any
    // airborne party can board a ship and there is no carpet
    // state left for this branch to refuse.
    for marker in [
        TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 2,
        TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 3,
    ] {
        assert_eq!(transport_family(marker), None, "marker {marker:#04x}");
        assert_eq!(
            transport_visual_tile_for_marker(marker),
            None,
            "marker {marker:#04x}"
        );
    }
    assert_eq!(
        TRANSPORT_MARKER_MAGIC_CARPET_LAST,
        TRANSPORT_MARKER_MAGIC_CARPET_FIRST + 1
    );
}

#[test]
fn board_ship_with_zero_hull_reports_badly_damaged_warning() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 2,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 0,
            skiffs: 2,
        }
    );
    assert_eq!(
        state.message,
        format!("Boarded ship. {SHIP_BADLY_DAMAGED_WARNING}")
    );
    assert_eq!(state.turn, 1);
}

#[test]
fn board_ship_with_hull_below_ten_reports_badly_damaged_warning() {
    // vehicles.md §4: ship boarding warns when hull condition is below
    // ten, not just zero.
    for hull in [1u8, 5, 9] {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: hull,
            aux3: 2,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert!(
            state.message.contains(SHIP_BADLY_DAMAGED_WARNING),
            "hull={hull} should report badly-damaged"
        );
    }
}

#[test]
fn board_ship_with_hull_at_ten_or_above_omits_badly_damaged_warning() {
    // vehicles.md §4: hull condition of ten or higher does not warn.
    for hull in [10u8, 50, 100] {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 168,
            tile: 168,
            x: 1,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: hull,
            aux3: 2,
        });

        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

        assert!(
            !state.message.contains(SHIP_BADLY_DAMAGED_WARNING),
            "hull={hull} should not report badly-damaged"
        );
    }
}

#[test]
fn board_non_ship_vehicle_still_requires_foot() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.transport = TransportState::Skiff {
        type_byte: 176,
        tile: 176,
    };
    state.player.facing = Direction::East;
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 160,
        tile: 160,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);

    assert_eq!(
        state.player.transport,
        TransportState::Skiff {
            type_byte: 176,
            tile: 176,
        }
    );
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(state.turn, 0);
    assert!(state.message.contains("On foot"));
}

#[test]
fn board_vehicle_accepts_magic_carpet_from_foot() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 184,
        tile: 184,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

    assert_eq!(
        state.player.transport,
        TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        }
    );
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(
        state.active_objects[0].tile,
        TRANSPORT_MARKER_MAGIC_CARPET_FIRST
    );
    assert!(state.active_objects[1].is_empty());
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("carpet"));
}

/// Gap #17 regression guard for the balloon-family removal.
///
/// `vehicles.md §11` "Balloon boundary": "Settled, not merely untraced.
/// Balloon sprites are catalog assets only... Do not invent boarding,
/// landing, or wind-driven balloon movement." §2: "**There is no balloon
/// and no sixth vehicle family.**"
///
/// Three `WorldDamageEffect::allows_transport` rows used to carry a balloon
/// alternative. §11 forbids re-homing balloon capabilities, so each
/// alternative was deleted outright rather than transferred to another
/// family. These are the surviving rows, and no family gained an
/// immunity it did not already hold for its own §3 reason.
#[test]
fn world_damage_immunities_survive_balloon_removal_without_transferring() {
    let carpet = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    let skiff = TransportState::Skiff {
        type_byte: FIRST_PLAYABLE_SKIFF_TILE,
        tile: FIRST_PLAYABLE_SKIFF_TILE,
    };
    let ship = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 2,
    };
    let horse = TransportState::Horse {
        type_byte: 160,
        tile: 160,
    };

    // Lava: carpet only. The balloon alternative did not move to the
    // horse, the skiff or the ship.
    assert!(WorldDamageEffect::Lava.allows_transport(carpet));
    assert!(!WorldDamageEffect::Lava.allows_transport(TransportState::Foot));
    assert!(!WorldDamageEffect::Lava.allows_transport(horse));
    assert!(!WorldDamageEffect::Lava.allows_transport(skiff));
    assert!(!WorldDamageEffect::Lava.allows_transport(ship));

    // Native lava: foot and carpet only.
    assert!(WorldDamageEffect::NativeLava.allows_transport(TransportState::Foot));
    assert!(WorldDamageEffect::NativeLava.allows_transport(carpet));
    assert!(!WorldDamageEffect::NativeLava.allows_transport(horse));
    assert!(!WorldDamageEffect::NativeLava.allows_transport(skiff));
    assert!(!WorldDamageEffect::NativeLava.allows_transport(ship));

    // Drowning: foot, ship, skiff and carpet -- the four water-capable or
    // damage-taking families §3 already names. The horse still drowns.
    assert!(WorldDamageEffect::Drowning.allows_transport(TransportState::Foot));
    assert!(WorldDamageEffect::Drowning.allows_transport(ship));
    assert!(WorldDamageEffect::Drowning.allows_transport(skiff));
    assert!(WorldDamageEffect::Drowning.allows_transport(carpet));
    assert!(!WorldDamageEffect::Drowning.allows_transport(horse));
    assert!(!WorldDamageEffect::Drowning.allows_transport(TransportState::SpriteSuppressed));
}

/// Gap #17: `vehicles.md §2` -- the balloon art band is catalog data and
/// never decodes into a transport state. `catalogs/tile-catalog.md §5`:
/// "Balloon art has no promoted live transport predicate in the analyzed
/// baseline."
#[test]
fn balloon_art_bytes_never_decode_into_a_transport_state() {
    for byte in FIRST_PLAYABLE_BALLOON_TILE..=(FIRST_PLAYABLE_BALLOON_TILE + 3) {
        assert_eq!(
            transport_from_vehicle_object(byte, byte, 0, 0),
            None,
            "balloon art byte {byte} must not decode into a transport state"
        );
        assert_eq!(
            transport_from_save_marker(byte),
            TransportState::Foot,
            "balloon art byte {byte} is not a published transport marker; it must fall back to the foot default, not a balloon"
        );
    }
}

/// `vehicles.md §4`: "No balloon object byte is accepted by the traced
/// B-Board handler; balloon art or manual references are not boardable
/// command behavior in the analyzed baseline." §2: "**There is no
/// balloon and no sixth vehicle family.**" Tile `188` (`0xBC`) is the
/// first balloon art frame; it stays a catalog asset and never becomes a
/// transport state.
#[test]
fn board_vehicle_refuses_unpromoted_balloon_family() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 188,
        tile: 188,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.active_objects.len(), 2);
    assert_eq!(state.turn, 0);
}

#[test]
fn board_town_horse_refuses_occupied_object_with_nay_without_turn() {
    let mut state = test_state(vec![5; 32 * 32], 0, 0);
    state.player.facing = Direction::South;
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 0,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state.active_objects.push(ActiveObject {
        type_byte: 160,
        tile: 160,
        x: 0,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.message, "Nay!");
    assert_eq!(state.turn, 0);
    assert!(!state.active_objects[1].is_empty());
    assert!(!state.active_objects[2].is_empty());
}

#[test]
fn board_world_horse_ignores_town_occupancy_refusal() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 192,
        tile: 192,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state.active_objects.push(ActiveObject {
        type_byte: 160,
        tile: 160,
        x: 1,
        y: 0,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);

    assert_eq!(
        state.player.transport,
        TransportState::Horse {
            type_byte: 160,
            tile: 160,
        }
    );
    assert!(!state.active_objects[1].is_empty());
    assert!(state.active_objects[2].is_empty());
    assert_eq!(state.turn, 1);
}

#[test]
fn carpet_world_movement_uses_standard_outdoor_time() {
    let mut state = world_state(open_world_grid(), 0, 0);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.sync_player_object();
    state.clock = GameClock::new(12, 58).unwrap();

    assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

    assert_eq!((state.player.x, state.player.y), (1, 0));
    assert_eq!(state.clock, GameClock::new(13, 0).unwrap());
    assert_eq!(state.turn, 1);
}

#[test]
fn exit_vehicle_parks_magic_carpet_object_and_returns_to_foot() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Carpet {
        type_byte: 184,
        tile: 184,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 184
            && object.tile == 184
            && object.x == 5
            && object.y == 5
            && object.z == WorldPlane::Underworld.save_floor()
    }));
    assert_eq!(state.turn, 1);
    assert!(state.message.contains("carpet"));
}

#[test]
fn exit_vehicle_reports_on_foot_without_turn_when_walking() {
    let mut state = world_state(open_world_grid(), 5, 5);

    assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);

    assert_eq!(state.message, "On foot!");
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.turn, 0);
}

#[test]
fn exit_vehicle_refuses_dungeon_before_vehicle_landing_logic() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(state.exit_vehicle(), MoveOutcome::Blocked);

    assert_eq!(
        state.player.transport,
        TransportState::Ship {
            type_byte: 168,
            tile: 168,
            sails_hoisted: false,
            hull: 77,
            skiffs: 2,
        }
    );
    assert_eq!((state.player.x, state.player.y), (1, 1));
    assert_eq!(state.message, "Not here!");
    assert_eq!(state.turn, 0);
}

#[test]
fn exit_vehicle_skips_occupied_landing_cells() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 194,
        tile: 194,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });

    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 168
            && object.x == 5
            && object.y == 5
            && object.z == WorldPlane::Underworld.save_floor()
    }));
    assert_eq!(state.turn, 1);
}

#[test]
fn exit_vehicle_skips_clean_lava_sidecar_for_foot_landing() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
        "UNDERWORLD 6 5 LAVA 5\n",
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(
        state.exit_vehicle_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::ExitedVehicle
    );

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert!(state.active_objects.iter().skip(1).any(|object| {
        object.type_byte == 168
            && object.x == 5
            && object.y == 5
            && object.z == WorldPlane::Underworld.save_floor()
    }));
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "ship!");
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn exit_vehicle_skips_foot_damaging_sidecar_landing_cells() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_DAMAGE_TILE_TABLE_FILE),
        "UNDERWORLD 6 5 DROWNING 5\n",
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();

    assert_eq!(
        state.exit_vehicle_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::ExitedVehicle
    );

    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.party[0].hp, DEFAULT_PARTY_HP);
    assert_eq!(state.turn, 1);
    assert_eq!(state.message, "ship!");
    let _ = fs::remove_dir_all(dir);
}

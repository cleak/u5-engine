#[test]
fn animation_tick_never_advances_the_gate_presence_counter() {
    // `overworld.md §9.1` (spec HEAD c00bf63): the gate-presence
    // counter "is **not** a member of the global tile-animation
    // families in `systems/animation.md` Section 6. It is not
    // advanced by the animation tick, it has no frame selector, and
    // skipping a rendered frame does not advance it."
    //
    // This replaces `moongate_animation_frame_advances_only_for_visible_active_gates`,
    // which was written against the per-render-frame moongate
    // animator `overworld.md §9` retracts in full.
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.clock = GameClock::new(21, 0).unwrap();
    state.natural_moongate_counter = 7;

    // Deliberately not a multiple of `STATIC_TILE_ANIMATION_PERIOD_TICKS`:
    // the shared phase counter wraps at that period, so a whole number
    // of periods would land back on zero and the "the tick really ran"
    // assertion below would pass vacuously.
    for _ in 0..(STATIC_TILE_ANIMATION_PERIOD_TICKS * 3 + 1) {
        state.advance_animation_clock();
    }

    assert_eq!(state.natural_moongate_counter, 7);
    // The families' own selector did advance, so the tick really ran.
    assert_ne!(state.animation.frame, 0);
}

/// The fall itself deals no damage — no "fall damage" line and no "party
/// slot" report reaches the transcript.
///
/// Slot 1 still loses a point, because the step consumed a turn and
/// `time.md §5` runs the status/provision pass "once per turn-consuming
/// action in overworld mode, town mode, and dungeon mode", where "a member
/// whose status is exactly Poisoned loses **exactly 1 current hit point**
/// … This is per member per turn, independently, not a shared roll and not
/// an hourly effect." Slots 2 and 3 are Dead and Sleeping, which the same
/// section "skips entirely: they take no poison damage".
#[test]
fn world_step_uses_clean_plane_transition_table_without_falls_narration() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 11 20 UNDERWORLD 30 40\n",
    )
    .unwrap();
    let mut under_ool = vec![0; OOL_PLANE_LEN];
    let slot = OOL_RECORD_LEN;
    under_ool[slot] = 168;
    under_ool[slot + 1] = 169;
    under_ool[slot + 2] = 31;
    under_ool[slot + 3] = 40;
    under_ool[slot + 4] = 0xff;
    under_ool[slot + 6] = 0x22;
    fs::write(dir.join("UNDER.OOL"), under_ool).unwrap();
    let mut state = world_state(open_world_grid(), 10, 20);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.wind = WindState::East;
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 0,
        skiffs: 0,
    };
    state.sail_cadence = 1;
    state.sail_stall_pending = true;
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state.sync_player_object();
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
            status: b'P',
            climb_stat: 0,
            mana: 8,
            hp: 6,
            max_hp: 20,
            level: 8,
        },
        PartyMember {
            slot: 2,
            class_byte: b'A',
            status: b'D',
            climb_stat: 0,
            mana: 8,
            hp: 9,
            max_hp: 20,
            level: 8,
        },
        PartyMember {
            slot: 3,
            class_byte: b'A',
            status: b'S',
            climb_stat: 0,
            mana: 8,
            hp: 8,
            max_hp: 20,
            level: 8,
        },
    ];

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
            from: WorldPlane::Britannia,
            to: WorldPlane::Underworld
        })
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((state.player.x, state.player.y), (30, 40));
    assert_eq!(state.active_objects[0].z, -1);
    assert_eq!(state.player.transport, TransportState::Foot);
    assert_eq!(state.active_effect_timing_status(), TimingStatusTag::Normal);
    assert_eq!(state.sail_cadence, 0);
    assert!(!state.sail_stall_pending);
    assert_eq!(state.grid[world_cell_index(30, 40)], 5);
    assert_eq!(
        state.active_objects[1],
        ActiveObject {
            type_byte: 168,
            tile: 169,
            x: 31,
            y: 40,
            z: -1,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        }
    );
    assert_eq!(state.clock, GameClock::new(12, 2).unwrap());
    assert_eq!(state.turn, 1);
    // `RETRACTIONS.md` R320 / `overworld.md` Section 8.1: a sidecar plane
    // transition is not the falls chain and narrates nothing at all - the
    // banner belongs to the waterfall handler, and the coordinate/wind/fall
    // damage prose this used to pin had no counterpart in the original.
    assert!(!state.message.contains("F-A-L-L-S"));
    assert!(!state.message.contains("fall damage"));
    assert!(!state.message.contains("East Winds"));
    assert_eq!(state.party[0].hp, 10);
    assert_eq!(state.party[1].hp, 5);
    assert_eq!(state.party[2].hp, 9);
    assert_eq!(state.party[3].hp, 8);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn falls_chain_fires_on_a_waterfall_south_of_the_party_and_gates_the_plane() {
    // `overworld.md` Section 8 + `RETRACTIONS.md` R320: the trigger is the
    // waterfall tile family, here in the cell immediately south of the party.
    // `(54, 138)` is the *landing* cell the handler tests after its two
    // forced southward steps, and only a landing there writes the plane.
    let dir = debug_game_dir();
    let brink_y = usize::from(SURFACE_CHASM_Y) - 2;
    let mut grid = open_world_grid();
    grid[world_cell_index(usize::from(SURFACE_CHASM_X), brink_y + 1)] = WATERFALL_TILE_FIRST;
    let mut state = britannia_state(grid, usize::from(SURFACE_CHASM_X), brink_y);
    state.party[0].hp = 10;
    state.party[0].climb_stat = 0;

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
            from: WorldPlane::Britannia,
            to: WorldPlane::Underworld
        })
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    // Two forced one-cell steps south.
    assert_eq!(
        (state.player.x, state.player.y),
        (usize::from(SURFACE_CHASM_X), usize::from(SURFACE_CHASM_Y))
    );
    // `overworld.md` Section 8.1, the whole player-visible transcript: two
    // lines, in this order, and no per-member narration.
    let lines: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .filter(|text| !text.is_empty())
        .collect();
    assert_eq!(lines, vec!["F-A-L-L-S!!!", "Falling into underworld!!"]);
    // Dexterity zero is less than or equal to every roll in `1..30`, so the
    // one point of damage always lands - and prints nothing.
    assert_eq!(state.party[0].hp, 9);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn falls_chain_fires_underfoot_on_a_brink_that_never_reaches_the_gate() {
    // `overworld.md` Section 8: "The chain is unconditional; only a landing
    // on Britannia `(54, 138)` also flips the plane." Britannia's other two
    // brinks run the whole presentation and change no plane.
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(46, 90)] = WATERFALL_TILE_LAST;
    let mut state = britannia_state(grid, 46, 90);
    state.party[0].hp = 10;
    state.party[0].climb_stat = 0;

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (46, 92));
    let lines: Vec<&str> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .filter(|text| !text.is_empty())
        .collect();
    assert_eq!(lines, vec!["F-A-L-L-S!!!"]);
    assert_eq!(state.party[0].hp, 9);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn falls_chain_plays_the_published_descending_sweep_once() {
    // `audio.md` Section 10.6: one site, played once per fall, and it fires
    // "on every waterfall brink on either plane, including the ones that
    // produce no plane change".
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(100, 96)] = 0xD6;
    let mut state = britannia_state(grid, 100, 96);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Passed
    );

    let sweeps = state
        .sound_effects_after(serial)
        .into_iter()
        .filter(|effect| matches!(effect, SoundEffect::SurfaceFallsDescent))
        .count();
    assert_eq!(sweeps, 1);
    let frequencies = crate::audio::surface_falls_descent().frequencies();
    assert_eq!(frequencies.len(), crate::audio::SURFACE_FALLS_DESCENT_UPDATES);
    assert_eq!(frequencies[0], crate::audio::SURFACE_FALLS_DESCENT_INITIAL_HZ as u32);
    assert_eq!(frequencies[1], 2495);
    assert_eq!(
        *frequencies.last().unwrap(),
        crate::audio::SURFACE_FALLS_DESCENT_LAST_HZ
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn falls_damage_uses_the_shared_skewed_roll_and_an_inclusive_gate() {
    // `RETRACTIONS.md` R321: the draw is the shared skewed closed-interval
    // `1..30` roll, and damage lands when Dexterity is **less than or equal
    // to** the roll - not a `0..255` byte with a strictly-greater gate, which
    // made fall damage nearly impossible.
    assert_eq!(WORLD_PLANE_FALL_SAVE_RAW_ROLL_LOW, 0);
    assert_eq!(WORLD_PLANE_FALL_SAVE_RAW_ROLL_HIGH, 60);
    assert_eq!(WORLD_PLANE_FALL_DAMAGE, 1);
    for raw in WORLD_PLANE_FALL_SAVE_RAW_ROLL_LOW..=WORLD_PLANE_FALL_SAVE_RAW_ROLL_HIGH {
        let roll = combat_skewed_roll_1_to_30(raw);
        assert!((1..=30).contains(&roll), "raw {raw} produced {roll}");
    }
    // Inclusive here, strict for outdoor K-Klimb - the two must not share an
    // implementation (`doors-and-z-transitions.md` Section 12.1).
    assert!(world_plane_fall_member_takes_damage(20, 20));
    assert!(world_plane_fall_member_takes_damage(20, 21));
    assert!(!world_plane_fall_member_takes_damage(20, 19));
    assert!(!outdoor_klimb_member_falls(20, 20));
    assert!(outdoor_klimb_member_falls(20, 21));
}

#[test]
fn world_plane_transition_table_overrides_base_tile_blocking() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 11 20 UNDERWORLD 30 40 12\n",
    )
    .unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(11, 20)] = 0x0c;
    let mut state = britannia_state(grid, 10, 20);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
            from: WorldPlane::Britannia,
            to: WorldPlane::Underworld
        })
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((state.player.x, state.player.y), (30, 40));
    assert_eq!(state.turn, 1);
    // `RETRACTIONS.md` R320: a sidecar plane transition is not the falls
    // chain, and the falls banner belongs to the waterfall handler.
    assert!(!state.message.contains("F-A-L-L-S"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn pass_turn_on_clean_plane_transition_applies_underfoot_transition() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 11 20 UNDERWORLD 30 40 5\n",
    )
    .unwrap();
    let state_grid = open_world_grid();
    let mut state = britannia_state(state_grid, 11, 20);

    assert_eq!(
        state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
        MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
            from: WorldPlane::Britannia,
            to: WorldPlane::Underworld
        })
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((state.player.x, state.player.y), (30, 40));
    assert_eq!(state.turn, 1);
    // `RETRACTIONS.md` R320: see above - the sidecar arm narrates nothing.
    assert!(!state.message.contains("F-A-L-L-S"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_plane_transition_tile_guard_mismatch_keeps_normal_movement() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_PLANE_TRANSITION_TABLE_FILE),
        "BRITANNIA 11 20 UNDERWORLD 30 40 24\n",
    )
    .unwrap();
    let mut state = britannia_state(open_world_grid(), 10, 20);

    assert_eq!(
        state
            .step_with_game_dir(Direction::East, Some(&dir))
            .unwrap(),
        MoveOutcome::Moved
    );

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (11, 20));
    assert_eq!(state.turn, 1);
    assert!(!state.message.contains("F-A-L-L-S"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_plane_transition_preserves_runtime_overlay_cache_between_planes() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 10, 20);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 11,
        y: 20,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    let mut cached_underworld = vec![ActiveObject::empty(); OOL_SLOTS - 1];
    cached_underworld[4] = ActiveObject {
        type_byte: 194,
        tile: 194,
        x: 31,
        y: 40,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    };
    state
        .world_overlays
        .set(WorldPlane::Underworld, cached_underworld);

    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert!(state.active_objects[1].is_empty());

    state
        .apply_world_plane_transition(
            &dir,
            WorldPlaneTransitionEntry {
                from_plane: WorldPlane::Britannia,
                x: 11,
                y: 20,
                to_plane: WorldPlane::Underworld,
                to_x: 30,
                to_y: 40,
                expected_tile: None,
                preserves_transport: false,
            },
        )
        .unwrap();

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!(state.player.transport, TransportState::Foot);
    assert!(state.world_overlays.get(WorldPlane::Britannia).unwrap()[0].is_empty());
    assert_eq!(
        state.world_object_at(31, 40),
        Some(&ActiveObject {
            type_byte: 194,
            tile: 194,
            x: 31,
            y: 40,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        })
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_plane_transition_save_load_uses_live_table_and_staged_disk_mirrors() {
    let dir = debug_game_dir();
    write_save_template_and_empty_overlays(&dir, 0, 0, 10, 20);
    let britannia_object = ActiveObject {
        type_byte: 168,
        tile: 168,
        x: 11,
        y: 20,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 7,
        aux3: 1,
    };
    let underworld_object = ActiveObject {
        type_byte: 194,
        tile: 194,
        x: 31,
        y: 40,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x22,
        aux1: 0,
        aux3: 0,
    };
    let updated_underworld_object = ActiveObject {
        type_byte: 194,
        tile: 195,
        x: 32,
        y: 41,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0x33,
        aux1: 4,
        aux3: 5,
    };
    let mut state = britannia_state(open_world_grid(), 10, 20);
    state.active_objects.push(britannia_object);
    let mut cached_underworld = vec![ActiveObject::empty(); OOL_SLOTS - 1];
    cached_underworld[0] = underworld_object;
    state
        .world_overlays
        .set(WorldPlane::Underworld, cached_underworld);

    state
        .apply_world_plane_transition(
            &dir,
            WorldPlaneTransitionEntry {
                from_plane: WorldPlane::Britannia,
                x: 11,
                y: 20,
                to_plane: WorldPlane::Underworld,
                to_x: 30,
                to_y: 40,
                expected_tile: None,
                preserves_transport: false,
            },
        )
        .unwrap();
    assert_eq!(state.active_objects[1], underworld_object);
    state.active_objects[1] = updated_underworld_object;

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved_gam = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
    assert_eq!(saved_gam[SAVE_SCENE_OFFSET], 0);
    assert_eq!(saved_gam[SAVE_Z_OFFSET], 0xff);
    assert_eq!(saved_gam[SAVE_X_OFFSET], 30);
    assert_eq!(saved_gam[SAVE_Y_OFFSET], 40);
    let active_table = decode_saved_active_objects(&saved_gam).unwrap();
    assert_eq!(active_table[0], updated_underworld_object);

    let saved_ool = fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap();
    let britannia_overlay = decode_ool_plane_objects(&saved_ool[..OOL_PLANE_LEN]).unwrap();
    let underworld_overlay = decode_ool_plane_objects(&saved_ool[OOL_PLANE_LEN..]).unwrap();
    assert!(britannia_overlay[0].is_empty());
    assert!(underworld_overlay[0].is_empty());
    assert_eq!(
        fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap(),
        saved_ool[..OOL_PLANE_LEN].to_vec()
    );
    assert_eq!(
        fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap(),
        saved_ool[OOL_PLANE_LEN..].to_vec()
    );

    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(options.target, PlayTarget::World(WorldPlane::Underworld));
    assert_eq!(options.start, Some((30, 40)));
    assert_eq!(
        options.saved_active_objects.as_ref().unwrap()[0],
        updated_underworld_object
    );
    let reloaded = PlayState::load_scene(&dir, options).unwrap();
    assert_eq!(
        reloaded.area,
        Area::World {
            plane: WorldPlane::Underworld
        }
    );
    assert_eq!((reloaded.player.x, reloaded.player.y), (30, 40));
    assert_eq!(reloaded.active_objects[1], updated_underworld_object);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn ool_decoder_keeps_non_player_slot_shape_and_skips_slot_zero() {
    let mut bytes = vec![0; OOL_PLANE_LEN];
    bytes[0] = 0xaa;
    bytes[1] = 0xab;
    let empty_payload_slot = OOL_RECORD_LEN * 3;
    bytes[empty_payload_slot + 1] = 0x44;
    bytes[empty_payload_slot + 2] = 250;
    bytes[empty_payload_slot + 3] = 251;
    bytes[empty_payload_slot + 4] = 0xfe;
    bytes[empty_payload_slot + 5] = 0x55;
    bytes[empty_payload_slot + 6] = 0x66;
    bytes[empty_payload_slot + 7] = 0x77;
    let slot = OOL_RECORD_LEN * 7;
    bytes[slot] = 168;
    bytes[slot + 1] = 169;
    bytes[slot + 2] = 12;
    bytes[slot + 3] = 34;
    bytes[slot + 4] = 0xff;
    bytes[slot + 5] = 88;
    bytes[slot + 6] = 0x22;
    bytes[slot + 7] = 3;

    let objects = decode_ool_plane_objects(&bytes).unwrap();

    assert_eq!(objects.len(), OOL_SLOTS - 1);
    assert!(objects[..6].iter().all(|object| object.is_empty()));
    assert_eq!(
        objects[2],
        ActiveObject {
            type_byte: 0,
            tile: 0x44,
            x: 250,
            y: 251,
            z: -2,
            phase: 0x66,
            aux1: 0x55,
            aux3: 0x77,
        }
    );
    assert_eq!(
        objects[6],
        ActiveObject {
            type_byte: 168,
            tile: 169,
            x: 12,
            y: 34,
            z: -1,
            phase: 0x22,
            aux1: 88,
            aux3: 3,
        }
    );
    assert!(objects[7..].iter().all(|object| object.is_empty()));
    assert!(decode_ool_plane_objects(&bytes[..OOL_PLANE_LEN - 1]).is_err());
}

#[test]
fn ool_encoder_round_trips_empty_slot_payload_bytes() {
    let payload = ActiveObject {
        type_byte: 0,
        tile: 0x44,
        x: 250,
        y: 251,
        z: -2,
        phase: 0x66,
        aux1: 0x55,
        aux3: 0x77,
    };

    let bytes = encode_ool_plane_objects(&[payload]).unwrap();
    let slot = OOL_RECORD_LEN;

    assert_eq!(
        &bytes[slot..slot + OOL_RECORD_LEN],
        &[0, 0x44, 250, 251, 0xfe, 0x55, 0x66, 0x77]
    );
    assert_eq!(decode_ool_plane_objects(&bytes).unwrap()[0], payload);
}

#[test]
fn active_object_encoder_writes_new_empty_records_as_all_zero() {
    let bytes = encode_active_object_table(&[ActiveObject::empty()]).unwrap();

    assert_eq!(&bytes[..OOL_RECORD_LEN], &[0; OOL_RECORD_LEN]);
}

#[test]
fn world_overlay_loader_uses_saved_ool_plane_half() {
    let dir = debug_game_dir();
    let mut saved = vec![0; SAVED_OOL_LEN];
    let slot = OOL_PLANE_LEN + OOL_RECORD_LEN;
    saved[slot] = 170;
    saved[slot + 1] = 170;
    saved[slot + 2] = 4;
    saved[slot + 3] = 5;
    saved[slot + 4] = 0xff;
    saved[slot + 5] = 99;
    saved[slot + 6] = STEADY_PHASE;
    saved[slot + 7] = 4;
    fs::write(dir.join("SAVED.OOL"), saved).unwrap();
    let mut under = vec![0; OOL_PLANE_LEN];
    under[OOL_RECORD_LEN] = 171;
    under[OOL_RECORD_LEN + 1] = 171;
    fs::write(dir.join("UNDER.OOL"), under).unwrap();

    let objects = load_world_overlay_objects(&dir, WorldPlane::Underworld).unwrap();

    assert_eq!(objects.len(), OOL_SLOTS - 1);
    assert_eq!(
        objects[0],
        ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 4,
            y: 5,
            z: -1,
            phase: STEADY_PHASE,
            aux1: 99,
            aux3: 4,
        }
    );
    assert!(objects.iter().skip(1).all(|object| object.is_empty()));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_load_from_save_uses_live_active_object_table() {
    let dir = debug_game_dir();
    let mut under = vec![0; OOL_PLANE_LEN];
    let slot = OOL_RECORD_LEN;
    under[slot] = 171;
    under[slot + 1] = 171;
    under[slot + 2] = 4;
    under[slot + 3] = 5;
    under[slot + 4] = 0xff;
    fs::write(dir.join("UNDER.OOL"), under).unwrap();
    let options = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        floor: -1,
        start: Some((10, 20)),
        clock: GameClock::default(),
        food: DEFAULT_FOOD_STOCK,
        gold: DEFAULT_GOLD_STOCK,
        keys: DEFAULT_KEY_STOCK,
        gems: DEFAULT_GEM_STOCK,
        climbing_gear: DEFAULT_CLIMBING_GEAR,
        special_items: [0; SPECIAL_ITEM_COUNT],
        party: default_party(),
        party_names: default_party_names(1),
        party_experience: default_party_experience(1),
        party_stay_counters: default_party_stay_counters(1),
        party_strengths: default_party_strengths(1),
        party_intelligence: default_party_intelligence(1),
        party_equipment: default_party_equipment(1),
        party_roster: default_party_roster(1),
        equipment_stock: [0; EQUIPMENT_COUNT],
        spell_charges: [0; SPELL_COUNT],
        scroll_stock: [0; SCROLL_COUNT],
        potion_stock: [0; POTION_COUNT],
        reagents: DEFAULT_REAGENTS,
        rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
            RARE_REAGENT_HARVEST_POINT_COUNT],
        fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
        fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
        dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
        saved_dungeon_working_buffer: None,
        moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
        shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
        removed_town_npc_flags: HashMap::new(),
        talk_branch_flags: HashMap::new(),
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
        moral_standing: 0,
        toll_progress: 0,
        natural_moongate_counter: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        wind: WindState::default(),
        wind_save_byte: 0,
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        fortunes_of_war: 0,
        camp_cooldown: 0,
        camp_month_cookie: 0,
        active_player: None,
        combat_round_counter: 0,
        combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
        transport: TransportState::Foot,
        facing: None,
        door_tracker: None,
        pending_vehicle: None,
        pending_vehicle_save: PendingVehicleSaveState::default(),
        inn_registry: Vec::new(),
        initial_britannia_overlay: None,
        debug_enter: None,
        saved_active_objects: Some(vec![ActiveObject {
            type_byte: 170,
            tile: 170,
            x: 11,
            y: 20,
            z: -1,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        }]),
        town_npc_mutations: Vec::new(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
    };

    let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

    assert_eq!(
        state.active_objects,
        vec![
            ActiveObject {
                type_byte: PLAYER_TILE,
                tile: PLAYER_TILE,
                x: 10,
                y: 20,
                z: -1,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            },
            ActiveObject {
                type_byte: 170,
                tile: 170,
                x: 11,
                y: 20,
                z: -1,
                phase: 0x22,
                aux1: 0,
                aux3: 0,
            },
        ]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn world_load_reports_current_wind_status_through_the_banner_not_the_message() {
    let dir = debug_game_dir();
    let options = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        floor: -1,
        start: Some((10, 20)),
        clock: GameClock::default(),
        food: DEFAULT_FOOD_STOCK,
        gold: DEFAULT_GOLD_STOCK,
        keys: DEFAULT_KEY_STOCK,
        gems: DEFAULT_GEM_STOCK,
        climbing_gear: DEFAULT_CLIMBING_GEAR,
        special_items: [0; SPECIAL_ITEM_COUNT],
        party: default_party(),
        party_names: default_party_names(1),
        party_experience: default_party_experience(1),
        party_stay_counters: default_party_stay_counters(1),
        party_strengths: default_party_strengths(1),
        party_intelligence: default_party_intelligence(1),
        party_equipment: default_party_equipment(1),
        party_roster: default_party_roster(1),
        equipment_stock: [0; EQUIPMENT_COUNT],
        spell_charges: [0; SPELL_COUNT],
        scroll_stock: [0; SCROLL_COUNT],
        potion_stock: [0; POTION_COUNT],
        reagents: DEFAULT_REAGENTS,
        rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
            RARE_REAGENT_HARVEST_POINT_COUNT],
        fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
        fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
        dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
        saved_dungeon_working_buffer: None,
        moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
        shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
        removed_town_npc_flags: HashMap::new(),
        talk_branch_flags: HashMap::new(),
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
        moral_standing: 0,
        toll_progress: 0,
        natural_moongate_counter: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        wind: WindState::West,
        wind_save_byte: 0,
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        fortunes_of_war: 0,
        camp_cooldown: 0,
        camp_month_cookie: 0,
        active_player: None,
        combat_round_counter: 0,
        combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
        transport: TransportState::Foot,
        facing: None,
        door_tracker: None,
        pending_vehicle: None,
        pending_vehicle_save: PendingVehicleSaveState::default(),
        inn_registry: Vec::new(),
        initial_britannia_overlay: None,
        debug_enter: None,
        saved_active_objects: Some(Vec::new()),
        town_npc_mutations: Vec::new(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
    };

    let state = PlayState::load_world_scene(&dir, WorldPlane::Underworld, options).unwrap();

    // `text-output.md §10.7`: the prevailing wind is a viewport bottom-ribbon
    // label, and the entry itself prints nothing. Loading a plane therefore
    // leaves the message window empty and reports the wind only through the
    // banner text the chrome draws.
    assert!(state.message.is_empty());
    assert_eq!(state.wind_status_message(), "West Winds");
    let _ = fs::remove_dir_all(dir);
}



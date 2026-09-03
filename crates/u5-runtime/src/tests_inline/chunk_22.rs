    #[test]
    fn ambient_world_actor_directs_step_toward_player_on_phase_zero() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (4, 5));
        // `active-objects.md §8`: the walker's only record writes on a
        // committed step are the position and the packed facing. The engine
        // used to reseed the phase's low nibble to two here, which is the
        // "do I move at all" pacing the section denies outdoors; the
        // countdown nibble is the animator's and is left alone.
        assert_eq!(object.phase, 0x60);
        assert_eq!(object.tile, 192);
    }

    #[test]
    fn active_object_walker_uses_high_to_low_outdoor_order() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 194,
            tile: 194,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (4, 5)
        );
        assert_eq!(
            (state.active_objects[2].x, state.active_objects[2].y),
            (6, 4)
        );
        assert_eq!(state.active_objects[1].phase, 0x60);
        assert_eq!(state.active_objects[2].phase, 0x00);
    }

    #[test]
    fn ambient_world_actor_moves_on_every_walker_turn_whatever_its_countdown() {
        // `active-objects.md §8`: "**There is no outdoor equivalent of the
        // town wander gate.** An ordinary land monster has no per-turn 'do I
        // move at all' roll: every turn the walker runs, an eligible slot
        // goes straight to the directed step planner."
        //
        // This test previously asserted the opposite - that a slot with a
        // non-zero animation countdown "animates without wandering" - which
        // is the pacing gate that sentence withdraws. The countdown still
        // ticks; it just does not decide whether the actor moves.
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (4, 5));
        assert_eq!(object.phase & 0x0f, 0x01, "the countdown still ticked");
    }

    #[test]
    fn steady_marked_outdoor_slot_is_not_one_of_the_walker_s_slots() {
        // `active-objects.md §8` phase table: the all-ones nibble is
        // "Steady; do not animate this slot. The animator skips", and the
        // outdoor walker "applies only to the outdoor animated/monster
        // predicate". A parked vehicle carries that marker and stays put.
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
        assert_eq!(object.phase, STEADY_PHASE);
    }

    #[test]
    fn half_time_world_epilogue_alternates_active_object_animation() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = QUICKNESS_ACTIVE_EFFECT_DURATION;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        // `encounters.md §2.1` gate 2: the Quickness parity bit "flips each
        // turn and the block returns on the turns it comes up set". What
        // §2.1 publishes about a gate that fires is only that the three
        // gates "sit ahead of the encounter probe *and* ahead of the outdoor
        // creature walker, so a gate that fires costs the turn its probe as
        // well as its creature movement" - the position assertion below.
        //
        // The phase assertion is *not* a published claim about the animator.
        // R315/R316 move the animator off the movement path entirely, and
        // this engine's outdoor per-turn walker ticks the countdown nibble
        // itself as the first step of its own pass, so a gated-out walker
        // leaves the nibble alone as a consequence of the walker not running.
        // The separately published per-render-frame animator
        // (`active-objects.md §8`) is not on this path at all.
        state.advance_turn();
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.active_objects[1].tile, 192);
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 5));

        // On the alternate turn the whole block runs: the countdown ticks and
        // the walker takes its step, with no separate "do I move" roll
        // (`active-objects.md §8`).
        state.advance_turn();
        assert_eq!(state.active_objects[1].phase & 0x0f, 0x01);
        assert_ne!((state.active_objects[1].x, state.active_objects[1].y), (5, 5));
    }

    #[test]
    fn no_minute_light_world_epilogue_skips_active_object_animation() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        state.active_effect_counter = 10;
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.active_objects[1].tile, 192);
    }

    #[test]
    fn ambient_world_actor_wander_respects_terrain_and_player_collision() {
        let mut blocked_grid = open_world_grid();
        blocked_grid[world_cell_index(4, 5)] = 1;
        let mut terrain_blocked = world_state(blocked_grid, 0, 0);
        terrain_blocked.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        terrain_blocked.advance_turn();

        let object = terrain_blocked
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 4));
        assert_eq!(object.phase, 0x00);

        let mut player_blocked = world_state(open_world_grid(), 4, 5);
        player_blocked.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        player_blocked.advance_turn();

        let object = player_blocked
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (5, 5));
        assert_eq!(object.phase, 0x60);
    }

    // ---------------------------------------------------------------
    // active-objects.md §8.1 -- overworld off-screen prune pass.
    // ---------------------------------------------------------------

    /// Steady, prunable world actor that will not wander during the
    /// animate pass, so a prune assertion measures pruning alone.
    fn prunable_marker(x: usize, y: usize) -> ActiveObject {
        ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x,
            y,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }
    }

    #[test]
    fn prune_pass_runs_from_the_overworld_turn_epilogue() {
        // active-objects.md §8.1: "The overworld per-turn epilogue runs
        // two passes over the table: the animate pass described above,
        // and then a separate prune pass." Before this was wired the
        // predicate existed and nothing called it, so occupancy drifted
        // from the original over play.
        //
        // Player (100, 100) -> scroll base (80, 80); the window is the
        // 32 cells forward of the base on each axis, i.e. 80..=111.
        let mut state = world_state(open_world_grid(), 100, 100);
        assert_eq!(world_scroll_base(100, 100), (80, 80));
        state.active_objects.push(prunable_marker(111, 100)); // offset 31 -> keep
        state.active_objects.push(prunable_marker(112, 100)); // offset 32 -> prune
        state.active_objects.push(prunable_marker(100, 112)); // y fails -> prune

        state.advance_turn();

        assert_eq!(state.active_objects[1], prunable_marker(111, 100));
        assert!(state.active_objects[2].is_empty());
        assert!(state.active_objects[3].is_empty());
    }

    #[test]
    fn prune_pass_clears_record_fields_zero_through_five_only() {
        // active-objects.md §8.1 correction: pruning is not the ordinary
        // one-byte free. The shared record writer clears TYPE through DEP1
        // while leaving PHASE and DEP3 byte-identical.
        let mut state = world_state(open_world_grid(), 100, 100);
        let mut marker = prunable_marker(112, 100); // offset 32 -> prune
        marker.tile = 0x06;
        marker.z = -1;
        marker.aux1 = 0x66;
        marker.phase = 0xab;
        marker.aux3 = 0x77;
        state.active_objects.push(marker);

        state.prune_far_overworld_objects();

        assert_eq!(
            state.active_objects[1],
            ActiveObject {
                type_byte: 0,
                tile: 0,
                x: 0,
                y: 0,
                z: 0,
                aux1: 0,
                phase: 0xab,
                aux3: 0x77,
            }
        );
    }

    #[test]
    fn prune_pass_never_releases_slot_zero() {
        // active-objects.md §8.1: "The pass walks the slots above zero
        // only. The player slot cannot be released by this path however
        // far the scroll base moves, and an implementation that
        // includes slot zero in the sweep can delete the player."
        let mut state = world_state(open_world_grid(), 100, 100);
        // Stamp slot zero well outside the window and prune directly, so
        // the per-turn sync cannot mask an over-broad sweep.
        state.active_objects[0].x = 200;
        state.active_objects[0].y = 200;
        let player_record = state.active_objects[0];

        state.prune_far_overworld_objects();

        assert_eq!(state.active_objects[0], player_record);
        assert!(!state.active_objects[0].is_empty());
    }

    #[test]
    fn prune_pass_uses_type_only_classifier_before_the_position_test() {
        // active-objects.md §8.1: only byte 0 selects a classifier row.
        // Byte 1 must not veto a prunable type or promote an excluded one.
        let mut state = world_state(open_world_grid(), 100, 100);
        let mut excluded_type = prunable_marker(200, 200);
        excluded_type.type_byte = 0x10; // parked vehicle: excluded
        excluded_type.tile = 0x80; // prunable-looking tile must not promote it
        let mut prunable_type = prunable_marker(200, 200);
        prunable_type.type_byte = 0x2c; // water-creature/pirate: prunable
        prunable_type.tile = 0x10; // vehicle-looking tile must not veto it
        state.active_objects.push(excluded_type);
        state.active_objects.push(prunable_type);

        state.prune_far_overworld_objects();

        assert_eq!(state.active_objects[1], excluded_type);
        assert!(state.active_objects[2].is_empty());
    }

    #[test]
    fn prune_pass_wraps_across_the_map_seam() {
        // active-objects.md §8.1: "The difference is formed in unsigned
        // eight-bit arithmetic against the scroll base, so it wraps
        // naturally with the 256-cell coordinate space rather than
        // needing a special case at the map seam."
        //
        // Player (250, 250) -> scroll base (240, 240); the window runs
        // 240..=255 and then 0..=15. A naive `slot - base` in signed or
        // wider arithmetic reports ~235 cells for a slot at (5, 5) and
        // frees the entire far side of the seam.
        let mut state = world_state(open_world_grid(), 250, 250);
        assert_eq!(world_scroll_base(250, 250), (240, 240));
        state.active_objects.push(prunable_marker(5, 5)); // wrapped offset 21 -> keep
        state.active_objects.push(prunable_marker(15, 15)); // wrapped offset 31 -> keep
        state.active_objects.push(prunable_marker(16, 5)); // wrapped offset 32 -> prune

        state.prune_far_overworld_objects();

        assert_eq!(state.active_objects[1], prunable_marker(5, 5));
        assert_eq!(state.active_objects[2], prunable_marker(15, 15));
        assert!(state.active_objects[3].is_empty());
    }

    #[test]
    fn prune_pass_does_not_run_outside_the_overworld() {
        // active-objects.md §8.1: "Town, dungeon and combat loops do not
        // run it."
        let mut town = test_state(town_free_roaming_grid(), 1, 1);
        town.active_objects.push(prunable_marker(200, 200));
        let marker = town.active_objects[1];

        town.advance_active_objects();

        assert_eq!(town.active_objects[1], marker);
    }

    // ---------------------------------------------------------------
    // active-objects.md §4 -- acquisition-time eviction cascade.
    // ---------------------------------------------------------------

    /// Fill the whole table with one type byte so the ordinary range is
    /// full and the cascade must choose a victim.
    fn table_packed_with(type_byte: u8, x: usize, y: usize) -> Vec<ActiveObject> {
        (0..OOL_SLOTS)
            .map(|_| ActiveObject {
                type_byte,
                tile: type_byte,
                x,
                y,
                z: WorldPlane::Underworld.save_floor(),
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            })
            .collect()
    }

    #[test]
    fn eviction_cascade_never_takes_the_protected_type_byte() {
        // active-objects.md §4: "The last-resort phase can still take any
        // byte except 0xB5, so 0xB5 is the only universally protected
        // byte-0 value in this allocator."
        let mut state = world_state(open_world_grid(), 100, 100);
        state.active_objects = table_packed_with(ACTIVE_OBJECT_PROTECTED_TYPE_BYTE, 200, 200);

        assert_eq!(state.active_object_eviction_victim(), None);
        assert_eq!(
            state.allocate_active_object_slot(prunable_marker(101, 100)),
            None
        );
    }

    #[test]
    fn eviction_cascade_spares_the_player_slot_and_the_reserved_band() {
        // active-objects.md §4: the ordinary acquisition path "searches
        // only slots one through twenty-three. Slot zero is the
        // canonical player slot, and slots twenty-four through
        // thirty-one are reserved for setup paths outside this
        // allocator." Make only slot 0 and 24..=31 evictable-looking;
        // the cascade must still refuse.
        let mut state = world_state(open_world_grid(), 100, 100);
        state.active_objects = table_packed_with(ACTIVE_OBJECT_PROTECTED_TYPE_BYTE, 200, 200);
        state.active_objects[ACTIVE_OBJECT_PLAYER_SLOT].type_byte = 0x05;
        for slot in ACTIVE_OBJECT_RESERVED_FIRST..=ACTIVE_OBJECT_RESERVED_LAST {
            state.active_objects[slot].type_byte = 0x05;
        }

        assert_eq!(state.active_object_eviction_victim(), None);

        // One ordinary slot made evictable is taken instead.
        state.active_objects[7].type_byte = 0x05;
        assert_eq!(state.active_object_eviction_victim(), Some(7));
    }

    #[test]
    fn eviction_cascade_prefers_off_screen_phases_over_on_screen_ones() {
        // active-objects.md §4: phases 2..=5 are the off-screen passes and
        // 6..=9 repeat the same classes with visible allowed, so an
        // off-screen dynamic actor (phase 3) outranks an on-screen
        // scenery byte (phase 6) even though scenery is the
        // higher-priority class when both are off-screen.
        let mut state = world_state(open_world_grid(), 100, 100);
        state.active_objects = table_packed_with(ACTIVE_OBJECT_PROTECTED_TYPE_BYTE, 100, 100);
        // Slot 3: scenery class, but standing on the player -> on-screen.
        state.active_objects[3].type_byte = 0x05;
        // Slot 9: dynamic-actor class, well outside the five-cell window.
        state.active_objects[9].type_byte = 0x90;
        state.active_objects[9].x = 140;

        assert_eq!(state.active_object_eviction_victim(), Some(9));

        // With the off-screen candidate gone, phase 6 takes the
        // on-screen scenery slot.
        state.active_objects[9].type_byte = ACTIVE_OBJECT_PROTECTED_TYPE_BYTE;
        assert_eq!(state.active_object_eviction_victim(), Some(3));
    }

    #[test]
    fn eviction_cascade_off_screen_gate_wraps_across_the_map_seam() {
        // Public #107: a candidate three cells from the player across the seam
        // is on-screen and must not be taken by an off-screen phase.
        let mut state = world_state(open_world_grid(), 2, 2);
        state.active_objects = table_packed_with(ACTIVE_OBJECT_PROTECTED_TYPE_BYTE, 2, 2);
        // Slot 5: dynamic actor three cells away across the seam.
        state.active_objects[5].type_byte = 0x90;
        state.active_objects[5].x = 255;
        // Slot 6: dynamic actor genuinely far away.
        state.active_objects[6].type_byte = 0x90;
        state.active_objects[6].x = 100;

        // Phase 3 (off-screen dynamic) must reach slot 6, not slot 5.
        assert_eq!(state.active_object_eviction_victim(), Some(6));
    }

    #[test]
    fn eviction_off_screen_gate_reads_player_globals_and_ignores_floor() {
        let mut state = world_state(open_world_grid(), 2, 2);
        state.active_objects[0].x = 100;
        state.active_objects[0].y = 100;
        let mut candidate = ActiveObject::empty();
        candidate.x = 2;
        candidate.y = 2;
        candidate.z = 7;

        assert!(!state.active_object_off_screen(candidate));
    }

    fn town_free_roaming_grid() -> Vec<u8> {
        vec![0x05; TOWN_GRID_SIDE * TOWN_GRID_SIDE]
    }

    #[test]
    fn town_free_roaming_actor_uses_prng_and_rewrites_record_on_success() {
        let mut state = test_state(town_free_roaming_grid(), 1, 1);
        state.prng_state = 0x0070;
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0x66,
            aux1: 7,
            aux3: 9,
        });

        state.advance_active_objects();

        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 6));
        assert_eq!(state.active_objects[1].type_byte, 0x10);
        assert_eq!(state.active_objects[1].tile, 0x10);
        assert_eq!(state.active_objects[1].phase, 0x66);
        assert_eq!(state.active_objects[1].z, 0);
        assert_eq!(state.active_objects[1].aux1, 7);
        assert_eq!(state.active_objects[1].aux3, 9);
        let mut expected_prng = 0x0070;
        for _ in 0..3 {
            expected_prng = u5_prng_advance_state(expected_prng);
        }
        assert_eq!(state.prng_state, expected_prng);
        assert!(state.visibility_dirty);
    }

    #[test]
    fn town_free_roaming_direction_bits_and_facing_bytes_match_spec() {
        assert_eq!(town_free_roaming_direction(0, 0), Direction::North);
        assert_eq!(town_free_roaming_direction(0, 1), Direction::South);
        assert_eq!(town_free_roaming_direction(1, 0), Direction::West);
        assert_eq!(town_free_roaming_direction(1, 1), Direction::East);

        assert_eq!(town_free_roaming_facing_byte(Direction::East, 0x11), 0x10);
        assert_eq!(town_free_roaming_facing_byte(Direction::West, 0x10), 0x11);
        assert_eq!(town_free_roaming_facing_byte(Direction::North, 0x11), 0x11);
        assert_eq!(town_free_roaming_facing_byte(Direction::South, 0x10), 0x10);
    }

    #[test]
    fn town_free_roaming_actor_skips_ineligible_and_off_floor_without_prng() {
        let mut state = test_state(town_free_roaming_grid(), 1, 1);
        state.prng_state = 0x1234;
        state.active_objects.push(ActiveObject {
            type_byte: 0x12,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 6,
            y: 5,
            z: 1,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        state.advance_active_objects();

        assert_eq!(state.prng_state, 0x1234);
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 5));
        assert_eq!((state.active_objects[2].x, state.active_objects[2].y), (6, 5));
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn town_free_roaming_walker_skips_linked_npc_sprites_without_prng() {
        // `town-mode.md §16` eligible-object-bytes row: "Empty slots, the
        // avatar's slot-zero record, linked NPC sprite classes, and all
        // other object families are skipped", and "No PRNG value is
        // consumed for ineligible or off-floor slots". The three shipped
        // `CASTLE:0` stable horses are roster slots carrying the horse
        // tags `0x10`/`0x11` (`catalogs/npc-roster.md §4`), so their
        // linked sprites must not also be driven by this walker.
        let mut state = test_state(town_free_roaming_grid(), 1, 1);
        state.prng_state = 0x1234;
        state.active_objects.push(npc_active_object(0x10, 5, 5, 0));
        state.npcs.push(RuntimeNpc {
            slot: 15,
            type_byte: 0x10,
            dialog_id: 0,
            schedule: [0; NPC_SCHEDULE_RECORD_LEN],
            state: NPC_STATE_IDLE,
            x: 5,
            y: 5,
            z: 0,
            cached_wp: 0,
            move_queue: Vec::new(),
            move_queue_pos: 0,
            stuck_counter: 0,
            active_object: Some(1),
        });

        state.advance_active_objects();

        assert_eq!(state.prng_state, 0x1234);
        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 5));
        assert!(!state.visibility_dirty);

        // Unlink it and the same record becomes an ordinary free-roaming
        // animal again.
        state.npcs.clear();
        state.advance_active_objects();
        assert_ne!(state.prng_state, 0x1234);
    }

    #[test]
    fn town_free_roaming_actor_chance_skip_consumes_one_draw() {
        let mut state = test_state(town_free_roaming_grid(), 1, 1);
        state.prng_state = 0x0008;
        state.active_objects.push(ActiveObject {
            type_byte: 0x11,
            tile: 0x11,
            x: 5,
            y: 5,
            z: 0,
            phase: 0x22,
            aux1: 0,
            aux3: 0,
        });

        state.advance_active_objects();

        assert_eq!((state.active_objects[1].x, state.active_objects[1].y), (5, 5));
        assert_eq!(state.active_objects[1].phase, 0x22);
        assert_eq!(state.prng_state, u5_prng_advance_state(0x0008));
        assert!(!state.visibility_dirty);
    }

    #[test]
    fn town_free_roaming_actor_pen_and_destination_failures_do_not_dirty() {
        let mut pen_blocked_grid = town_free_roaming_grid();
        pen_blocked_grid[5 * TOWN_GRID_SIDE + 6] = 0x43;
        let mut pen_blocked = test_state(pen_blocked_grid, 1, 1);
        pen_blocked.prng_state = 0x0070;
        pen_blocked.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        pen_blocked.advance_active_objects();
        assert_eq!((pen_blocked.active_objects[1].x, pen_blocked.active_objects[1].y), (5, 5));
        assert_eq!(pen_blocked.prng_state, u5_prng_advance_state(0x0070));
        assert!(!pen_blocked.visibility_dirty);

        let mut destination_blocked_grid = town_free_roaming_grid();
        destination_blocked_grid[6 * TOWN_GRID_SIDE + 5] = 0x04;
        let mut object_blocked = test_state(destination_blocked_grid, 1, 1);
        object_blocked.prng_state = 0x0070;
        object_blocked.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        object_blocked.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        });

        object_blocked.advance_active_objects();
        assert_eq!((object_blocked.active_objects[1].x, object_blocked.active_objects[1].y), (5, 5));
        let mut expected_prng = 0x0070;
        for _ in 0..3 {
            expected_prng = u5_prng_advance_state(expected_prng);
        }
        assert_eq!(object_blocked.prng_state, expected_prng);
        assert!(!object_blocked.visibility_dirty);
    }

    #[test]
    fn town_free_roaming_edge_actor_can_choose_inward_or_fail_outward_after_all_draws() {
        let mut inward = test_state(town_free_roaming_grid(), 1, 1);
        inward.prng_state = 0x0070;
        inward.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 5,
            y: 0,
            z: 0,
            phase: 0x44,
            aux1: 7,
            aux3: 9,
        });

        inward.advance_active_objects();

        assert_eq!((inward.active_objects[1].x, inward.active_objects[1].y), (5, 1));
        assert_eq!(inward.active_objects[1].phase, 0x44);
        let mut expected_inward_prng = 0x0070;
        for _ in 0..3 {
            expected_inward_prng = u5_prng_advance_state(expected_inward_prng);
        }
        assert_eq!(inward.prng_state, expected_inward_prng);
        assert!(inward.visibility_dirty);

        let mut outward = test_state(town_free_roaming_grid(), 1, 1);
        // This stream passes the chance gate and selects north, which is out
        // of bounds from the top edge. The bounds rejection comes only after
        // the axis and sign draws.
        outward.prng_state = 0;
        outward.active_objects.push(ActiveObject {
            type_byte: 0x11,
            tile: 0x11,
            x: 5,
            y: 0,
            z: 0,
            phase: 0x55,
            aux1: 3,
            aux3: 4,
        });

        outward.advance_active_objects();

        assert_eq!((outward.active_objects[1].x, outward.active_objects[1].y), (5, 0));
        assert_eq!(outward.active_objects[1].phase, 0x55);
        let mut expected_outward_prng = 0;
        for _ in 0..3 {
            expected_outward_prng = u5_prng_advance_state(expected_outward_prng);
        }
        assert_eq!(outward.prng_state, expected_outward_prng);
        assert!(!outward.visibility_dirty);
    }

    #[test]
    fn outdoor_water_creature_uses_water_predicate_and_rewrites_facing() {
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = 1;
        let mut state = world_state(grid, 0, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0x2c,
            tile: 0x2c,
            x: 5,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: SEA_CREATURE_SPAWN_AUX_SEED,
            aux3: 0,
        });

        state.advance_turn();

        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (4, 5)
        );
        assert_eq!(state.active_objects[1].type_byte, 0x2f);
        assert_eq!(state.active_objects[1].tile, 0x2f);
        assert_eq!(state.active_objects[1].phase, 0x60);
    }

    #[test]
    fn world_post_turn_adjacent_hostile_enters_terrain_combat() {
        let dir = debug_game_dir();
        let record = synthetic_combat_arena_record();
        fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        let outcome = state.pass_turn_with_game_dir(Some(&dir)).unwrap();

        assert_eq!(outcome, MoveOutcome::Used);
        assert!(state.combat_active);
        assert_eq!(state.pending_combat_terrain_trigger_slot, Some(1));
        assert_eq!(state.message, combat_banner_line());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terrain_combat_main_path_uses_class_default_spawn_count() {
        let dir = debug_game_dir();
        let record = synthetic_combat_arena_record();
        fs::write(dir.join(BRIT_CBT_FILE), record.repeat(BRIT_CBT_RECORDS)).unwrap();
        let replacement_seed = (0..=u16::MAX)
            .find(|seed| {
                let mut prng = *seed;
                u5_prng_range_u16(
                    &mut prng,
                    0,
                    u16::from(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR - 1),
                ) == 0
            })
            .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.prng_state = replacement_seed;
        let object = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let message = state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, object)
            .unwrap();

        assert!(message.contains("BRIT.CBT arena 2"));
        assert!(message.contains("requested Orc"));
        assert_eq!(state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].tile, 0xc0);
        // `catalogs/monster-bestiary.md §2.1`: the substitution is keyed
        // to the base class, not the arena - Orc bands mix in Trolls.
        assert_eq!(combat_class_companion(32), Some(41));
        let _ = fs::remove_dir_all(dir);
    }

    fn shadow_lord_trigger_object() -> ActiveObject {
        let shadow_lord_type = COMBAT_CLASS_SHADOW_LORD * 4 + OUTDOOR_COMBAT_TYPE_FIRST;
        ActiveObject {
            type_byte: shadow_lord_type,
            tile: shadow_lord_type,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }
    }

    /// `encounters.md §4`, Shadow Lord arena branch. "The arena is 10 either
    /// way, and the branch runs entirely inside encounter setup, before the
    /// combat scene is entered." In order, and only when the sceptre is held:
    ///
    /// 1. "Print exactly `The Sceptre is reclaimed!` followed by one newline.
    ///    There is no terminating period and no leading blank line."
    /// 2. "Play the sceptre-reclaimed sting (`audio.md` section 8.4.1)."
    /// 3. "Clear the sceptre flag."
    ///
    /// "The order matters for a transcript: the line completes before the
    /// sting starts, and the flag clears last."
    #[test]
    fn terrain_combat_shadow_lord_entry_reclaims_the_sceptre() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;
        let before = state.sound_effect_serial;

        let message = state
            .enter_terrain_combat_from_world_object(
                &dir,
                WorldPlane::Britannia,
                1,
                shadow_lord_trigger_object(),
            )
            .unwrap();

        assert!(message.contains("BRIT.CBT arena 10"), "{message}");
        assert_eq!(state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX], 0);

        // Step 1: the exact published line, with no terminating period, and
        // printed after the conflict banner rather than before it.
        assert_eq!(SCEPTRE_RECLAIMED_LINE, "The Sceptre is reclaimed!");
        let texts: Vec<&str> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        let banner_at = texts
            .iter()
            .position(|text| *text == combat_banner_line())
            .expect("the conflict banner is printed");
        let line_at = texts
            .iter()
            .position(|text| *text == SCEPTRE_RECLAIMED_LINE)
            .expect("the sceptre line is printed");
        assert!(banner_at < line_at, "{texts:?}");

        // Step 2: the sting. `audio.md §8.4.1` — "It is the only caller of
        // this recipe."
        assert_eq!(
            state.sound_effects_after(before),
            vec![SoundEffect::SceptreReclaimed]
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `encounters.md §4`: "If the sceptre is not held, none of the three
    /// happen and setup proceeds directly to arena entry."
    #[test]
    fn terrain_combat_shadow_lord_entry_without_the_sceptre_is_silent() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 0;
        let before = state.sound_effect_serial;

        let message = state
            .enter_terrain_combat_from_world_object(
                &dir,
                WorldPlane::Britannia,
                1,
                shadow_lord_trigger_object(),
            )
            .unwrap();

        // The arena is 10 either way.
        assert!(message.contains("BRIT.CBT arena 10"), "{message}");
        assert!(
            !state
                .message_entries()
                .iter()
                .any(|entry| entry.text == SCEPTRE_RECLAIMED_LINE)
        );
        assert!(
            !state
                .sound_effects_after(before)
                .contains(&SoundEffect::SceptreReclaimed)
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `combat.md §5` order of operations: the party is seated from the
    /// arena record's own six party entry coordinates at step 2, before
    /// the banner at step 3 and the count roll at step 4, so the seats
    /// never depend on how many monsters spawned.
    #[test]
    fn terrain_combat_entry_seats_party_from_arena_record_and_prints_banner() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let record =
            CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        let object = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, object)
            .unwrap();

        let entry_x = record.outdoor_setup_table_a();
        let entry_y = record.outdoor_setup_table_b();
        assert_eq!(
            (state.active_objects[0].x, state.active_objects[0].y),
            (usize::from(entry_x[0]), usize::from(entry_y[0]))
        );
        assert_eq!(
            (state.combat_actors[0].x, state.combat_actors[0].y),
            (entry_x[0], entry_y[0])
        );
        assert_eq!(state.message, combat_banner_line());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terrain_combat_pirate_family_uses_class_one_and_fixed_banner() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        let object = ActiveObject {
            type_byte: 0x2f,
            tile: 0x2f,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let message = state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, object)
            .unwrap();

        assert!(message.contains("BRIT.CBT arena 12"), "{message}");
        assert!(message.contains("requested Pirates combatant(s)"), "{message}");
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].owner_target_class,
            1
        );
        assert_eq!(combat_class_stats(1).unwrap().default_spawn_count, 9);
        assert_eq!(combat_class_stats(1).unwrap().max_hp, 15);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terrain_combat_zero_spawn_identity_gap_creates_only_lead_actor() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        let object = ActiveObject {
            type_byte: 0xe8,
            tile: 0xe8,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let message = state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, object)
            .unwrap();

        assert!(message.contains("spawned 1 of 1 requested"), "{message}");
        assert_eq!(
            state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].owner_target_class,
            42
        );
        assert!(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn terrain_combat_main_path_uses_selected_brit_cbt_record_metadata() {
        let dir = debug_game_dir();
        let mut bank = Vec::with_capacity(BRIT_CBT_FILE_LEN);
        for arena in 0..BRIT_CBT_RECORDS {
            let mut record = synthetic_combat_arena_record();
            let terrain_tag = 0x40 + arena as u8;
            record[0] = terrain_tag;
            record[6 * COMBAT_ARENA_ROW_STRIDE + 11] = arena as u8;
            record[7 * COMBAT_ARENA_ROW_STRIDE + 11] = 15 - arena as u8;
            bank.extend_from_slice(&record);
        }
        fs::write(dir.join(BRIT_CBT_FILE), bank).unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        let object = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 6,
            y: 5,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };

        let message = state
            .enter_terrain_combat_from_world_object(&dir, WorldPlane::Britannia, 1, object)
            .unwrap();

        assert!(message.contains("BRIT.CBT arena 2"));
        assert_eq!(state.combat_terrain[0][0], 0x42);
        // `combat.md §5`: the monster's descriptor is index six but its
        // renderer record "continues from the first record left free by
        // the seated party", so index the record through the
        // descriptor's link byte. One live party member leaves record
        // one free.
        let first_monster_record =
            usize::from(state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].active_object_slot);
        assert_eq!(first_monster_record, 1);
        assert_eq!(
            (
                state.active_objects[first_monster_record].x,
                state.active_objects[first_monster_record].y,
            ),
            (2, 13)
        );
        assert_eq!(
            (
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].x,
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].y,
            ),
            (2, 13)
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn overworld_prunes_far_classified_objects_but_keeps_excluded_vehicles() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 40,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 0x10,
            tile: 0x10,
            x: 80,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            // `0xA8` is inside the published `0x80..=0xB3` prunable
            // range; the old semantic "vehicle-like" shortcut kept it.
            type_byte: 0xa8,
            tile: 0xa8,
            x: 80,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        assert_eq!(state.active_objects.len(), 4);
        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.active_objects[2].type_byte, 0x10);
        assert_eq!(state.active_objects[2].x, 80);
        assert!(state.active_objects[3].is_empty());
    }

    #[test]
    fn overworld_pruning_uses_public_scroll_base_window() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 15,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        state.active_objects.push(ActiveObject {
            type_byte: 193,
            tile: 193,
            x: 224,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });

        assert_eq!(
            world_scroll_base(state.player.x, state.player.y),
            (240, 240)
        );

        state.advance_turn();

        assert_eq!(state.active_objects[1].type_byte, 192);
        assert_eq!(
            (state.active_objects[1].x, state.active_objects[1].y),
            (15, 0)
        );
        assert!(state.active_objects[2].is_empty());
    }

    #[test]
    fn overworld_prunes_after_post_tick_wander_position() {
        let mut state = world_state(open_world_grid(), 0, 0);
        state.active_objects.push(ActiveObject {
            type_byte: 192,
            tile: 192,
            x: 16,
            y: 0,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0x60,
            aux1: 0,
            aux3: 0,
        });

        state.advance_turn();

        let object = state
            .active_objects
            .iter()
            .find(|object| object.type_byte == 192)
            .unwrap();
        assert_eq!((object.x, object.y), (15, 0));
        assert_eq!(object.phase, 0x60);
    }

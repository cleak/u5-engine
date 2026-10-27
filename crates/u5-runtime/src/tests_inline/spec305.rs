// The u5-spec#305 answers: the dungeon contact launch (Q1), the fallback
// four-direction walker (Q2), the encounter candidate draws (Q3), contact
// combat (Q4) and the session's first world tick (Q5).
mod spec305 {
    use super::*;

    // ---- Q1: `dungeon-mode.md §14` contact launch ----------------------

    /// Party at `(1, 0)` on level 3 facing north, the monster west of it at
    /// `(0, 0)`, and only the party's cell open around the monster, so the
    /// walk's first east draw is the accepted step and it contacts.
    fn dungeon_contact_state(lit: bool) -> (PlayState, DungeonScene, u8) {
        let level = 3u8;
        let mut grid = open_dungeon_record();
        for (x, y) in [(0usize, 7usize), (0, 1), (7, 0)] {
            grid[dungeon_cell_index(level, x, y)] = 0xb0;
        }
        let mut state = dungeon_state(grid, level, 1, 0);
        state.player.facing = Direction::North;
        state.torch_counter = u8::from(lit) * 50;
        state.light_spell_counter = 0;
        state.active_objects.push(ActiveObject {
            type_byte: 0,
            tile: 0,
            x: 0,
            y: 0,
            z: 0,
            phase: 0x20,
            aux1: 20,
            aux3: 0,
        });
        state.prng_state = (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 3) == 1
            })
            .unwrap();
        let Area::Dungeon { scene, .. } = state.area else {
            unreachable!()
        };
        (state, scene, level)
    }

    /// The reference launch: the walk's draws, `pose_draws` inclusive
    /// `0..100` draws, then the §14.1 painter and setup.
    fn reference_contact_prng(mut state: PlayState, level: u8, pose_draws: usize) -> u16 {
        let object = state.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT];
        let step = state
            .dungeon_active_monster_step(DUNGEON_ACTIVE_MONSTER_SLOT, object, level)
            .expect("the walk contacts");
        assert!(step.contact);
        for _ in 0..pose_draws {
            let _ = u5_prng_range_u16(&mut state.prng_state, 0, 100);
        }
        state.player.facing = Direction::West;
        state.free_active_object_slot(DUNGEON_ACTIVE_MONSTER_SLOT);
        state
            .enter_dungeon_active_monster_combat(level, object)
            .expect("the launch");
        state.prng_state
    }

    /// Issue #305 Q1: "The only random draws between the contact step's
    /// last draw and the room painter's first draw are the step 3 redraw's
    /// own. In the usual case those are exactly the active monster's two
    /// inclusive `0..100` pose draws".
    #[test]
    fn a_lit_contact_takes_the_monsters_two_pose_draws_before_the_painter() {
        let (mut state, scene, level) = dungeon_contact_state(true);
        let expected = reference_contact_prng(state.clone(), level, 2);

        let outcome = state.apply_dungeon_active_monster_step(scene, level).unwrap();

        assert_eq!(outcome, Some(MoveOutcome::Used));
        assert!(state.combat_active);
        assert_eq!(state.player.facing, Direction::West);
        assert_eq!(state.prng_state, expected);
        assert!(!state.message_row_open_mid_line);
    }

    /// "The pair is absent when the view is unlit (no torch and no light
    /// spell: the repaint shows darkness and paints no sprite)".
    #[test]
    fn an_unlit_contact_takes_no_draw_before_the_painter() {
        let (mut state, scene, level) = dungeon_contact_state(false);
        let expected = reference_contact_prng(state.clone(), level, 0);

        state.apply_dungeon_active_monster_step(scene, level).unwrap();

        assert!(state.combat_active);
        assert_eq!(state.prng_state, expected);
    }

    /// The redraw on its own: a lit monster one cell ahead is painted once,
    /// two `0..100` draws; under Negate Time "forced poses, no draws".
    #[test]
    fn the_first_person_redraw_draws_only_for_a_visible_monster() {
        let level = 0u8;
        let mut state = dungeon_state(open_dungeon_record(), level, 3, 3);
        state.player.facing = Direction::North;
        state.torch_counter = 50;
        state.active_objects.push(ActiveObject {
            type_byte: 1,
            tile: 1,
            x: 3,
            y: 2,
            z: 0,
            phase: 0x20,
            aux1: 21,
            aux3: 0,
        });
        state.prng_state = 0x09eb;
        let mut expected = state.prng_state;
        let _ = u5_prng_range_u16(&mut expected, 0, 100);
        let _ = u5_prng_range_u16(&mut expected, 0, 100);

        let mut lit = state.clone();
        lit.redraw_dungeon_first_person_view();
        assert_eq!(lit.prng_state, expected, "two pose draws");

        let mut negated = state.clone();
        negated.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        negated.active_effect_counter = 5;
        negated.redraw_dungeon_first_person_view();
        assert_eq!(negated.prng_state, state.prng_state, "Negate Time");

        let mut dark = state.clone();
        dark.torch_counter = 0;
        dark.redraw_dungeon_first_person_view();
        assert_eq!(dark.prng_state, state.prng_state, "unlit");

        let mut behind = state.clone();
        behind.player.facing = Direction::South;
        behind.redraw_dungeon_first_person_view();
        assert_eq!(behind.prng_state, state.prng_state, "not in view");
    }

    // ---- Q2: `active-objects.md §8` fallback walker ---------------------

    /// "Worked example: a land monster at a dead end with both directed axes
    /// refused leaves it only on the draw that points back toward the cell
    /// it came from ... on any other draw whose cell is water or occupied it
    /// stays put for the turn." Water north, east and south of the monster;
    /// the player is south-east, so both directed axes are refused.
    #[test]
    fn a_dead_end_monster_backs_out_only_on_the_draw_toward_its_open_cell() {
        for seed in 0..64u16 {
            let mut state = world_state(open_world_grid(), 12, 12);
            state.active_objects.truncate(1);
            state.active_objects.push(ActiveObject {
                type_byte: 0x98,
                tile: 0x98,
                x: 10,
                y: 10,
                z: WorldPlane::Underworld.save_floor(),
                phase: 0,
                aux1: 0,
                aux3: 0,
            });
            for (x, y) in [(10usize, 9usize), (11, 10), (10, 11)] {
                state.grid[world_cell_index(x, y)] = BRIT_DEEP_WATER_TILE;
            }
            state.prng_state = 0x2b5d ^ seed.wrapping_mul(0x9e37);
            let mut expected = state.prng_state;
            let _axis_coin = u5_prng_range_u16(&mut expected, 0, 1);
            let roll = u5_prng_range_u16(&mut expected, 0, 3);

            let moved = state.try_wander_active_object(1);

            let at = (state.active_objects[1].x, state.active_objects[1].y);
            if roll == 3 {
                assert!(moved, "seed {seed}: 3 is west, the open cell");
                assert_eq!(at, (9, 10));
            } else {
                assert!(!moved, "seed {seed}: draw {roll} names water");
                assert_eq!(at, (10, 10));
                assert_eq!(
                    state.prng_state, expected,
                    "seed {seed}: no further draw and no second attempt"
                );
            }
        }
    }

    // ---- Q3: `encounters.md §4` candidate coordinates ------------------

    /// "Each candidate takes two inclusive `0..31` draws from the shared
    /// generator, the column first and then the row. Each draw is added to
    /// the matching axis of the scroll base".
    #[test]
    fn the_spawn_candidate_is_column_then_row_off_the_scroll_base() {
        // Every cell is a land-bucket tile, so the first candidate that
        // clears the distance bands spawns.
        let mut state = world_state(vec![0x05; WORLD_CELLS], 100, 100);
        state.active_objects.truncate(1);
        let base = world_scroll_base(100, 100);
        let seed = (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                let column = u5_prng_range_u16(&mut probe, 0, 31) as usize;
                let row = u5_prng_range_u16(&mut probe, 0, 31) as usize;
                encounter_spawner_separation_ok(
                    (base.0 + column) as u8,
                    (base.1 + row) as u8,
                    100,
                    100,
                ) && column != row
            })
            .unwrap();
        state.prng_state = seed;
        let mut probe = seed;
        let column = u5_prng_range_u16(&mut probe, 0, 31) as usize;
        let row = u5_prng_range_u16(&mut probe, 0, 31) as usize;

        let slot = state
            .spawn_native_world_encounter(WorldPlane::Underworld)
            .expect("a land tile spawns");

        let object = state.active_objects[slot];
        assert_eq!((object.x, object.y), (base.0 + column, base.1 + row));
    }

    /// R633: "Only candidates rejected by terrain classification (a zero
    /// bucket, step 3) or by the shore filter (step 4) count toward the
    /// one-hundred-twenty-eight limit. A candidate rejected by the distance
    /// bands of step 2 goes straight back to the two coordinate draws
    /// without counting".
    #[test]
    fn distance_band_rejections_do_not_count_toward_the_give_up_limit() {
        // Hard-reject terrain everywhere: every candidate that clears the
        // bands is a counted, drawless step 3 rejection.
        let mut state = world_state(vec![0x0c; WORLD_CELLS], 100, 100);
        state.active_objects.truncate(1);
        state.prng_state = 0x09eb;
        let base = world_scroll_base(100, 100);
        let mut expected = state.prng_state;
        let mut counted = 0;
        let mut banded = 0;
        while counted < ENCOUNTER_SPAWNER_RETRY_LIMIT {
            let column = u5_prng_range_u16(&mut expected, 0, 31) as usize;
            let row = u5_prng_range_u16(&mut expected, 0, 31) as usize;
            if encounter_spawner_separation_ok(
                ((base.0 + column) % WORLD_SIDE) as u8,
                ((base.1 + row) % WORLD_SIDE) as u8,
                100,
                100,
            ) {
                counted += 1;
            } else {
                banded += 1;
            }
        }
        assert!(banded > 0, "the seed must exercise a band rejection");

        assert_eq!(state.spawn_native_world_encounter(WorldPlane::Underworld), None);
        assert_eq!(state.prng_state, expected);
    }

    // ---- Q5: the session's first world tick ----------------------------

    fn quiet_wind_seed() -> u16 {
        (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 63) != 0
            })
            .unwrap()
    }

    fn loaded_world(gate: u8) -> (PlayState, std::path::PathBuf) {
        let dir = debug_game_dir();
        let options = PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            start: Some((40, 40)),
            world_tick_gate: gate,
            ..PlayOptions::default()
        };
        let mut state = PlayState::load_scene(&dir, options).unwrap();
        state.active_objects.truncate(1);
        state.prng_state = quiet_wind_seed();
        (state, dir)
    }

    /// R639: "on Journey Onward the session's first world tick, the input
    /// helper's entry step at the first prompt, is full: it animates
    /// eligible objects and takes the `0..63` wind draw. It is bare only
    /// when the loaded file holds zero."
    #[test]
    fn the_first_prompts_entry_step_draws_unless_the_save_holds_zero() {
        let (mut steady, dir) = loaded_world(WORLD_TICK_GATE_STEADY);
        let seed = steady.prng_state;
        assert!(!steady.session_start_world_step_frozen);
        assert!(steady.run_overworld_input_helper_entry_step());
        assert_eq!(steady.prng_state, u5_prng_advance_state(seed), "one wind draw");
        let _ = fs::remove_dir_all(dir);

        let (mut frozen, dir) = loaded_world(0);
        let seed = frozen.prng_state;
        assert!(frozen.session_start_world_step_frozen);
        assert!(frozen.run_overworld_input_helper_entry_step());
        assert_eq!(frozen.prng_state, seed, "one draw fewer at the first prompt");
        let _ = fs::remove_dir_all(dir);
    }

    /// `formats/saved-gam.md §10`: "The load restores it", and the world
    /// tick "sets the byte back to one at its own tail". A zero survives a
    /// save taken before any world step and is gone after one.
    #[test]
    fn the_gate_byte_round_trips_through_the_save() {
        let (mut state, dir) = loaded_world(0);
        let mut seed = saved_game_seed_bytes(0, 0, 40, 40);
        seed[SAVE_WORLD_TICK_GATE_OFFSET] = 0;
        fs::write(dir.join("INIT.GAM"), seed).unwrap();
        write_empty_ool_mirrors(&dir);
        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved[SAVE_WORLD_TICK_GATE_OFFSET], 0, "no tick has run");
        assert_eq!(load_play_options_from_save(&dir).unwrap().world_tick_gate, 0);

        let _ = state.advance_visual_tick();
        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved[SAVE_WORLD_TICK_GATE_OFFSET], WORLD_TICK_GATE_STEADY);
        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.world_tick_gate, WORLD_TICK_GATE_STEADY);
        assert!(!PlayState::load_scene(&dir, options).unwrap().session_start_world_step_frozen);
        let _ = fs::remove_dir_all(dir);
    }

    /// R642: "The impact presentation's world tick draws as any world tick
    /// does: one `0..63` wind check ... ahead of the `N` absorption draws".
    #[test]
    fn rough_seas_takes_the_impact_ticks_wind_draw_before_the_damage_draws() {
        let mut grid = open_world_grid();
        grid[world_cell_index(5, 5)] = BRIT_DEEP_WATER_TILE;
        let mut state = world_state(grid, 5, 5);
        state.active_objects.truncate(1);
        state.player.transport = TransportState::Skiff {
            type_byte: TRANSPORT_MARKER_SKIFF_FIRST,
            tile: TRANSPORT_MARKER_SKIFF_FIRST,
        };
        state.prng_state = quiet_wind_seed();
        let mut expected = u5_prng_advance_state(state.prng_state);
        let living = state.party.iter().filter(|m| m.status != PARTY_STATUS_DEAD).count();
        let rolls: Vec<u8> = (0..living)
            .map(|_| u5_prng_range_u16(&mut expected, 1, 8) as u8)
            .collect();

        let Some(OutdoorImpactAbsorption::PartyDamaged(damage)) =
            state.apply_rough_seas_if_eligible()
        else {
            panic!("rough seas");
        };

        assert_eq!(damage.iter().map(|d| d.roll).collect::<Vec<_>>(), rolls);
        assert_eq!(state.prng_state, expected);
    }

    /// `overworld.md §5` (issue #305 Q5): "on the Underworld plane only ...
    /// one `0..255` draw on every consumed turn, where the value 105 prints
    /// `EARTHQUAKE!`, runs the full-viewport flash ... (1856 draws) and then
    /// one `1..8` damage draw per non-Dead member".
    #[test]
    fn the_underworld_takes_one_earthquake_draw_per_consumed_turn() {
        let mut state = world_state(open_world_grid(), 5, 5);
        let quiet = (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 255) != 105
            })
            .unwrap();
        state.prng_state = quiet;
        assert!(!state.apply_underworld_earthquake_check());
        assert_eq!(state.prng_state, u5_prng_advance_state(quiet));

        let quake = (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 255) == 105
            })
            .unwrap();
        state.prng_state = quake;
        let hp_before: Vec<u16> = state.party.iter().map(|m| m.hp).collect();
        let mut expected = u5_prng_advance_state(quake);
        for _ in 0..crate::audio::FLASH_BAND_COUNT {
            expected = u5_prng_advance_state(expected);
        }
        let mut rolls = Vec::new();
        for member in &state.party {
            if member.status != PARTY_STATUS_DEAD {
                rolls.push(u5_prng_range_u16(&mut expected, 1, 8) as u16);
            }
        }

        assert!(state.apply_underworld_earthquake_check());

        assert_eq!(state.prng_state, expected);
        assert!(state
            .message_entries()
            .iter()
            .any(|entry| entry.text == "EARTHQUAKE!"));
        assert!(!state.message_row_open_mid_line);
        for (index, roll) in rolls.iter().enumerate() {
            assert_eq!(state.party[index].hp, hp_before[index].saturating_sub(*roll));
        }

        let mut surface = state.clone();
        surface.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        let before = surface.prng_state;
        assert!(!surface.apply_underworld_earthquake_check());
        assert_eq!(surface.prng_state, before, "no draw on the surface");
    }
}

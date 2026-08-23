// `overworld.md §9` / `§9.1` (spec HEAD c00bf63): the moon-gate
// gate-presence phase model.
//
// These tests are written against the current spec, which **retracts** the
// per-render-frame moongate animator earlier revisions described. The
// tests that encoded the animator (a moongate frame ring on
// `AnimationClock`, and an ambient-light "daytime threshold") were deleted
// rather than adapted, as `§9` instructs.
//
// They are *not* written against the "not animated at all / painted like
// any other tile" claim `catalogs/tile-catalog.md §4` briefly carried:
// that claim contradicted §11 of the same document and is corrected at
// spec HEAD `38b0231`. Only the animator was withdrawn, not the composed
// intermediate frame.

mod moongate_gate_presence_phase {
    use super::*;

    const GATE_ROW_BASE: u8 = 100;
    const GROUND_PIXEL: u8 = 200;
    const ENDGAME_GROUND_PIXEL: u8 = 210;
    const SCRATCH_SHIPPED_PIXEL: u8 = 250;

    /// A tile atlas whose four interesting tiles are pixel-distinguishable:
    /// the grass ground plate, the endgame throne-room ground plate, the
    /// moon-gate tile (each row carries its own row number, so "the top
    /// *N* rows" is observable), and the `0x116` scratch slot filled with
    /// a sentinel standing in for its shipped artwork.
    fn gate_phase_atlas() -> TileAtlas {
        let mut atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let fill = |atlas: &mut TileAtlas, tile: usize, value: u8| {
            let start = tile * TILE_ATLAS_TILE_PIXELS;
            atlas.pixels[start..start + TILE_ATLAS_TILE_PIXELS].fill(value);
        };
        fill(&mut atlas, MOONGATE_PHASE_GROUND_TILE as usize, GROUND_PIXEL);
        fill(
            &mut atlas,
            MOONGATE_PHASE_ENDGAME_GROUND_TILE as usize,
            ENDGAME_GROUND_PIXEL,
        );
        fill(
            &mut atlas,
            MOONGATE_PHASE_SCRATCH_TILE,
            SCRATCH_SHIPPED_PIXEL,
        );
        let gate_start = moongate_phase_gate_tile() as usize * TILE_ATLAS_TILE_PIXELS;
        for row in 0..TILE_ATLAS_SIDE {
            let start = gate_start + row * TILE_ATLAS_SIDE;
            atlas.pixels[start..start + TILE_ATLAS_SIDE].fill(GATE_ROW_BASE + row as u8);
        }
        atlas
    }

    fn tile_of(atlas: &TileAtlas, tile: usize) -> Vec<u8> {
        atlas.tile_pixels(tile).unwrap().to_vec()
    }

    /// The pixel value the viewport holds at `row` of the tile cell whose
    /// top-left corner is `(cell_x, cell_y)`.
    fn viewport_row_pixel(viewport: &TileViewport, cell_x: usize, cell_y: usize, row: usize) -> u8 {
        viewport
            .pixel(cell_x * TILE_ATLAS_SIDE, cell_y * TILE_ATLAS_SIDE + row)
            .unwrap()
    }

    fn expected_composed_rows(rows: u8, ground_pixel: u8) -> Vec<u8> {
        let ground_rows = TILE_ATLAS_SIDE - rows as usize;
        (0..TILE_ATLAS_SIDE)
            .map(|row| {
                if row < ground_rows {
                    ground_pixel
                } else {
                    GATE_ROW_BASE + (row - ground_rows) as u8
                }
            })
            .collect()
    }

    #[test]
    fn gate_presence_counter_is_a_sixteen_step_position_not_an_on_off_flag() {
        // `overworld.md §9.1` phase table: 0 is "not a gate, the refresh
        // has already restored the cell to terrain 5"; 1..15 are composed
        // transition frames; 16 is the whole moon-gate tile through the
        // ordinary tile path.
        assert_eq!(moongate_phase_draw(0), MoongatePhaseDraw::Ground);
        for counter in 1..16u8 {
            assert_eq!(
                moongate_phase_draw(counter),
                MoongatePhaseDraw::Composed { rows: counter },
                "counter {counter} must compose {counter} risen pixel rows"
            );
        }
        assert_eq!(moongate_phase_draw(16), MoongatePhaseDraw::WholeGate);
        // Sixteen is the only phase showing the authored artwork intact,
        // and it is where the once-per-turn refresh saturates.
        assert_eq!(MOONGATE_PHASE_FULL, 16);
        assert_eq!(MOONGATE_PHASE_FULL, NATURAL_MOONGATE_COUNTER_MAX);
    }

    #[test]
    fn refresh_counts_up_to_sixteen_and_saturates_there() {
        // `overworld.md §9`: during night hours the counter increases
        // toward sixteen. Counting up makes the gate rise; sixteen is the
        // fully open gate.
        let mut counter = 0u8;
        let mut seen = Vec::new();
        for _ in 0..24 {
            counter = natural_moongate_advance_counter(counter, 21);
            seen.push(counter);
        }
        assert_eq!(&seen[..16], &(1..=16u8).collect::<Vec<_>>()[..]);
        assert!(
            seen[16..].iter().all(|value| *value == MOONGATE_PHASE_FULL),
            "the night refresh saturates at sixteen, it does not wrap"
        );
        // Daytime counts back down and floors at zero.
        for _ in 0..24 {
            counter = natural_moongate_advance_counter(counter, 8);
        }
        assert_eq!(counter, 0);
    }

    #[test]
    fn composed_frame_takes_its_bottom_rows_from_the_gate_tiles_top_rows() {
        // `overworld.md §9.1`: "the ground tile, with its bottom *N* pixel
        // rows replaced by the top *N* pixel rows of the moon-gate tile".
        let atlas = gate_phase_atlas();
        let ground = tile_of(&atlas, MOONGATE_PHASE_GROUND_TILE as usize);
        let gate = tile_of(&atlas, moongate_phase_gate_tile() as usize);

        for rows in 0..=MOONGATE_PHASE_FULL {
            let mut scratch = vec![SCRATCH_SHIPPED_PIXEL; TILE_ATLAS_TILE_PIXELS];
            compose_moongate_phase_frame(&mut scratch, &ground, &gate, rows).unwrap();
            let observed: Vec<u8> = (0..TILE_ATLAS_SIDE)
                .map(|row| scratch[row * TILE_ATLAS_SIDE])
                .collect();
            assert_eq!(
                observed,
                expected_composed_rows(rows, GROUND_PIXEL),
                "phase {rows} must show {rows} gate rows rising out of the ground"
            );
            // Every row is uniform: the replacement is whole pixel rows.
            for row in 0..TILE_ATLAS_SIDE {
                let start = row * TILE_ATLAS_SIDE;
                assert!(
                    scratch[start..start + TILE_ATLAS_SIDE]
                        .iter()
                        .all(|pixel| *pixel == scratch[start]),
                    "phase {rows} row {row} must come from exactly one source tile"
                );
            }
        }
    }

    #[test]
    fn composition_rejects_a_phase_past_sixteen_rather_than_inventing_one() {
        // No-fallback: the phase is a sixteen-step position, so there is
        // no seventeenth frame to guess at.
        let atlas = gate_phase_atlas();
        let ground = tile_of(&atlas, MOONGATE_PHASE_GROUND_TILE as usize);
        let gate = tile_of(&atlas, moongate_phase_gate_tile() as usize);
        let mut scratch = vec![0; TILE_ATLAS_TILE_PIXELS];
        let error =
            compose_moongate_phase_frame(&mut scratch, &ground, &gate, MOONGATE_PHASE_FULL + 1)
                .unwrap_err();
        assert!(
            error.to_string().contains("sixteen-step position"),
            "unexpected message: {error}"
        );
    }

    #[test]
    fn scratch_tile_0x116_is_saved_and_restored_around_every_composition() {
        // `overworld.md §9.1`: "The composed frame is written into a
        // dedicated scratch tile, id `0x116`. That slot is saved and
        // restored around every composition, so its shipped artwork
        // survives; but an implementation must not treat `0x116` as a
        // stable authored tile while a gate is on screen. The same id
        // doubles as the party-vanishing sprite in Section 9.2."
        assert_eq!(MOONGATE_PHASE_SCRATCH_TILE, 0x116);
        let atlas = gate_phase_atlas();
        let ground = tile_of(&atlas, MOONGATE_PHASE_GROUND_TILE as usize);
        let gate = tile_of(&atlas, moongate_phase_gate_tile() as usize);
        let shipped = tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE);
        let mut scratch = shipped.clone();

        for rows in [4u8, 11, 1] {
            let seen = with_moongate_phase_scratch_tile(
                &mut scratch,
                &ground,
                &gate,
                rows,
                |composed| composed.to_vec(),
            )
            .unwrap();
            // While the composition is live the slot holds the frame, not
            // the shipped artwork.
            assert_ne!(seen, shipped, "phase {rows} must occupy the scratch slot");
            let observed: Vec<u8> = (0..TILE_ATLAS_SIDE)
                .map(|row| seen[row * TILE_ATLAS_SIDE])
                .collect();
            assert_eq!(observed, expected_composed_rows(rows, GROUND_PIXEL));
            // Once the composition is done the shipped artwork is back, so
            // §9.2's party-vanishing sprite is never corrupted.
            assert_eq!(scratch, shipped, "phase {rows} must restore the slot");
        }

        // The slot is restored even when the composition fails.
        let error = with_moongate_phase_scratch_tile(
            &mut scratch,
            &ground,
            &gate,
            MOONGATE_PHASE_FULL + 1,
            |_| (),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(scratch, shipped);
    }

    #[test]
    fn ground_plate_is_grass_in_play_and_throne_room_floor_in_the_endgame() {
        // `overworld.md §9.1`: "In ordinary play the ground plate is
        // terrain `5`, grass - the same tile the daytime pass restores.
        // The endgame scene substitutes tile `0x44`, its throne-room
        // floor, which is why the endgame's gate appears to rise out of
        // flagstones rather than turf."
        assert_eq!(MOONGATE_PHASE_GROUND_TILE, 5);
        assert_eq!(MOONGATE_PHASE_GROUND_TILE, NATURAL_MOONGATE_UNDERLYING_TILE);
        assert_eq!(MOONGATE_PHASE_ENDGAME_GROUND_TILE, 0x44);
        assert_eq!(moongate_phase_ground_tile(false), 5);
        assert_eq!(moongate_phase_ground_tile(true), 0x44);

        let mut state = britannia_state(open_world_grid(), 5, 5);
        assert_eq!(state.natural_moongate_phase_ground_tile(), 5);
        state.endgame = Some(EndgameState::awaiting_first_confirmation());
        assert_eq!(state.natural_moongate_phase_ground_tile(), 0x44);
    }

    #[test]
    fn endgame_scene_composes_the_gate_over_its_throne_room_floor() {
        // The same composition routine, driven with the substituted ground
        // plate, is what makes the endgame gate rise out of flagstones.
        let atlas = gate_phase_atlas();
        let ground = tile_of(&atlas, MOONGATE_PHASE_ENDGAME_GROUND_TILE as usize);
        let gate = tile_of(&atlas, moongate_phase_gate_tile() as usize);
        let mut scratch = vec![SCRATCH_SHIPPED_PIXEL; TILE_ATLAS_TILE_PIXELS];
        compose_moongate_phase_frame(&mut scratch, &ground, &gate, 6).unwrap();
        let observed: Vec<u8> = (0..TILE_ATLAS_SIDE)
            .map(|row| scratch[row * TILE_ATLAS_SIDE])
            .collect();
        assert_eq!(observed, expected_composed_rows(6, ENDGAME_GROUND_PIXEL));
        // The grass plate is not what the endgame draws.
        assert_ne!(observed, expected_composed_rows(6, GROUND_PIXEL));
    }

    /// A Britannia state with live moon-gate terrain at both cells
    /// flanking the party, so one view holds two gates.
    fn two_gate_state() -> PlayState {
        let mut grid = open_world_grid();
        grid[world_cell_index(4, 5)] = NATURAL_MOONGATE_LIVE_TILE;
        grid[world_cell_index(6, 5)] = NATURAL_MOONGATE_LIVE_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.ambient_light = FULL_DAYLIGHT;
        state
    }

    #[test]
    fn renderer_composes_a_live_gate_cell_at_the_shared_phase() {
        let atlas = gate_phase_atlas();
        for counter in [1u8, 7, 15] {
            let mut state = two_gate_state();
            state.natural_moongate_counter = counter;
            let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();
            let observed: Vec<u8> = (0..TILE_ATLAS_SIDE)
                .map(|row| viewport_row_pixel(&viewport, 0, 1, row))
                .collect();
            assert_eq!(
                observed,
                expected_composed_rows(counter, GROUND_PIXEL),
                "phase {counter} must draw a composed transition frame"
            );
        }
    }

    #[test]
    fn phase_sixteen_draws_the_whole_moon_gate_tile_and_phase_zero_draws_ground() {
        let atlas = gate_phase_atlas();

        let mut open = two_gate_state();
        open.natural_moongate_counter = MOONGATE_PHASE_FULL;
        let viewport = open.render_top_down_viewport(1, &atlas).unwrap().unwrap();
        let observed: Vec<u8> = (0..TILE_ATLAS_SIDE)
            .map(|row| viewport_row_pixel(&viewport, 0, 1, row))
            .collect();
        // The authored artwork, intact: every row of tile 0xDC.
        assert_eq!(
            observed,
            (0..TILE_ATLAS_SIDE)
                .map(|row| GATE_ROW_BASE + row as u8)
                .collect::<Vec<_>>()
        );

        // Phase zero is not a gate: zero risen rows, i.e. the bare ground
        // plate the refresh restores.
        let mut closed = two_gate_state();
        closed.natural_moongate_counter = 0;
        let viewport = closed.render_top_down_viewport(1, &atlas).unwrap().unwrap();
        for row in 0..TILE_ATLAS_SIDE {
            assert_eq!(viewport_row_pixel(&viewport, 0, 1, row), GROUND_PIXEL);
        }
    }

    #[test]
    fn every_visible_gate_composes_at_the_same_global_phase() {
        // `overworld.md §9.1`: "The composition is per-cell but the phase
        // is global. Every visible moon-gate cell is composed at the same
        // phase, so a view containing more than one gate shows them rising
        // and sinking in lockstep. There is no per-gate phase."
        let atlas = gate_phase_atlas();
        let mut state = two_gate_state();
        state.natural_moongate_counter = 9;
        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();

        let west: Vec<u8> = (0..TILE_ATLAS_SIDE)
            .map(|row| viewport_row_pixel(&viewport, 0, 1, row))
            .collect();
        let east: Vec<u8> = (0..TILE_ATLAS_SIDE)
            .map(|row| viewport_row_pixel(&viewport, 2, 1, row))
            .collect();
        assert_eq!(west, east, "two gates in one view must be in lockstep");
        assert_eq!(west, expected_composed_rows(9, GROUND_PIXEL));

        // Advancing the one shared counter moves both, together.
        state.natural_moongate_counter = 10;
        let viewport = state.render_top_down_viewport(1, &atlas).unwrap().unwrap();
        let west: Vec<u8> = (0..TILE_ATLAS_SIDE)
            .map(|row| viewport_row_pixel(&viewport, 0, 1, row))
            .collect();
        let east: Vec<u8> = (0..TILE_ATLAS_SIDE)
            .map(|row| viewport_row_pixel(&viewport, 2, 1, row))
            .collect();
        assert_eq!(west, expected_composed_rows(10, GROUND_PIXEL));
        assert_eq!(east, expected_composed_rows(10, GROUND_PIXEL));
    }

    #[test]
    fn rendering_a_gate_leaves_the_shipped_scratch_tile_artwork_intact() {
        let atlas = gate_phase_atlas();
        let shipped = tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE);
        let mut state = two_gate_state();
        state.natural_moongate_counter = 5;
        state.render_top_down_viewport(1, &atlas).unwrap().unwrap();
        state.natural_moongate_counter = 12;
        state.render_top_down_viewport(1, &atlas).unwrap().unwrap();
        assert_eq!(
            tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE),
            shipped,
            "tile 0x116 must not be left holding a gate frame"
        );
    }

    #[test]
    fn gate_presence_counter_round_trips_through_saved_gam_offset_0x02e1() {
        // `overworld.md §9.1`: "There is exactly one such byte in the
        // whole engine, it is save-backed at `SAVED.GAM` offset `0x02E1`,
        // and it survives turns, mode changes, scene changes and save/load
        // alike." Modelling it as turn-scoped "breaks save/load round-trip
        // and loses the mid-rise state, so a game saved at 20:07 reloads
        // with a gate at the wrong height."
        assert_eq!(SAVE_NATURAL_MOONGATE_COUNTER_OFFSET, 0x02e1);

        // Decode side.
        let mut bytes = saved_game_seed_bytes(0, 0, 10, 20);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        bytes[SAVE_NATURAL_MOONGATE_COUNTER_OFFSET] = 9;
        let decoded = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(decoded.natural_moongate_counter, 9);

        // Encode side, then a real load back off disk.
        let dir = debug_game_dir();
        let mut template = saved_game_seed_bytes(0, 0, 10, 20);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        write_saved_clock(&mut template, GameClock::new(20, 7).unwrap());
        fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
        fs::write(dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
        fs::write(dir.join(UNDER_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();

        let mut state = britannia_state(open_world_grid(), 10, 20);
        state.clock = GameClock::new(20, 7).unwrap();
        // Saved mid-rise: seven of sixteen pixel rows out of the ground.
        state.natural_moongate_counter = 7;
        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );

        let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(saved[SAVE_NATURAL_MOONGATE_COUNTER_OFFSET], 7);
        let reloaded = load_play_options_from_save(&dir).unwrap();
        assert_eq!(
            reloaded.natural_moongate_counter, 7,
            "a game saved mid-rise must reload with its gates at that height"
        );
        let reloaded_state =
            PlayState::load_scene(&dir, reloaded).unwrap();
        assert_eq!(reloaded_state.natural_moongate_counter, 7);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn transit_leaves_the_shared_counter_at_zero_and_sinks_an_unrelated_gate() {
        // `overworld.md §9.2` stage B drives the shared counter from 15
        // down to 1 and "the countdown ends with the counter at zero".
        // `§9.1`: "Because it is shared, the blocking transit sequence in
        // Section 9.2 leaves it at zero when it finishes. A gate that was
        // mid-rise elsewhere in view is therefore driven to zero by an
        // unrelated party's transit and rises again from zero on
        // subsequent turns. That is the original's behaviour, not a defect
        // to design around."
        let dir = debug_game_dir();
        let entered_idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[entered_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        grid[world_cell_index(4, 5)] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(11, 58).unwrap();
        state.set_cached_moon_glyph_slots(Some(1), None);
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: WorldPlane::Britannia.save_floor() as u8,
        };
        // Both gates are mid-rise on the one shared counter.
        state.natural_moongate_counter = 11;

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.natural_moongate_counter, 0,
            "the transit countdown ends with the shared counter at zero"
        );
        // The bystander gate was mid-rise at eleven and nothing about it
        // changed; the unrelated transit alone drove it to phase zero, and
        // it rises again from zero on subsequent turns rather than
        // resuming at eleven.
        assert_eq!(moongate_phase_draw(0), MoongatePhaseDraw::Ground);
        state.clock = GameClock::new(21, 0).unwrap();
        state.refresh_natural_moongates();
        assert_eq!(state.natural_moongate_counter, 1);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn shipped_save_opens_with_no_gate_up() {
        // `overworld.md §9`: "the shipped starting save holds zero there -
        // correct, because the game opens at hour eight with no gate up."
        let mut bytes = saved_game_seed_bytes(0, 0, 10, 20);
        assert_eq!(bytes[SAVE_NATURAL_MOONGATE_COUNTER_OFFSET], 0);
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        let options = play_options_from_save_bytes(&bytes).unwrap();
        assert_eq!(options.natural_moongate_counter, 0);
        assert_eq!(options.clock.hour, 8);
        assert_eq!(moongate_phase_draw(0), MoongatePhaseDraw::Ground);
    }
}

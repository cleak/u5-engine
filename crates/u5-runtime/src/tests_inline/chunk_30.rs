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
        state.set_cached_moon_glyph_slots(1, 0);
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

// `overworld.md §9.2` (spec HEAD `c00bf63`): the two-stage **blocking**
// transit transition the overworld live-gate entry hook plays before the
// party is relocated.
//
// Both `0x116` uses meet for the first time here: `§9.1` composes gate
// frames into that scratch slot, and `§9.2` draws the vanishing party from
// the same id. Every write below therefore goes through
// `with_moongate_phase_scratch_tile`, and the tests check the shipped
// artwork is intact afterwards - including while a gate is mid-rise.
mod moongate_transit_transition {
    use super::*;

    const GROUND_PIXEL: u8 = 200;
    const SCRATCH_SHIPPED_PIXEL: u8 = 250;

    /// An atlas whose gate tile is pixel-distinguishable (every pixel
    /// nonzero, and no two neighbours equal), whose ground plate is one
    /// flat value, and whose `0x116` slot holds a sentinel standing in for
    /// its shipped artwork.
    fn transit_atlas() -> TileAtlas {
        let mut atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let ground_start = MOONGATE_PHASE_GROUND_TILE as usize * TILE_ATLAS_TILE_PIXELS;
        atlas.pixels[ground_start..ground_start + TILE_ATLAS_TILE_PIXELS].fill(GROUND_PIXEL);
        let scratch_start = MOONGATE_PHASE_SCRATCH_TILE * TILE_ATLAS_TILE_PIXELS;
        atlas.pixels[scratch_start..scratch_start + TILE_ATLAS_TILE_PIXELS]
            .fill(SCRATCH_SHIPPED_PIXEL);
        let gate_start = moongate_phase_gate_tile() as usize * TILE_ATLAS_TILE_PIXELS;
        for pixel in 0..TILE_ATLAS_TILE_PIXELS {
            // 1..=255: never colour zero, so "left at colour zero" is
            // observable, and never equal to the ground or scratch fills
            // by accident at more than one index.
            atlas.pixels[gate_start + pixel] = (pixel % 255) as u8 + 1;
        }
        atlas
    }

    fn tile_of(atlas: &TileAtlas, tile: usize) -> Vec<u8> {
        atlas.tile_pixels(tile).unwrap().to_vec()
    }

    /// Every frame one presentation run produced, as owned copies.
    struct RecordedFrame {
        step: MoongateTransitStep,
        party_sprite: MoongateTransitPartySprite,
        cell: Vec<u8>,
        party_pixels: Option<Vec<u8>>,
    }

    fn record_transit(
        atlas: &mut TileAtlas,
        counter: &mut u8,
    ) -> (MoongateTransitPlayback, Vec<RecordedFrame>) {
        let mut frames = Vec::new();
        let playback = run_moongate_transit_presentation(
            &mut atlas.pixels,
            MOONGATE_PHASE_GROUND_TILE as usize,
            counter,
            &mut |frame| {
                frames.push(RecordedFrame {
                    step: frame.step,
                    party_sprite: frame.party_sprite,
                    cell: frame.cell.to_vec(),
                    party_pixels: frame.party_pixels.map(<[u8]>::to_vec),
                });
            },
        )
        .unwrap();
        (playback, frames)
    }

    #[test]
    fn stage_a_spends_two_hundred_fifty_six_dispatch_steps() {
        // `§9.2`: "The frame counts are `15` for stage B and `256`
        // dispatch steps for stage A; both are exact." One step clears the
        // cell to colour zero; the other 255 plot a pixel each.
        let steps = moongate_transit_steps().unwrap();
        let clears = steps
            .iter()
            .filter(|step| matches!(step, MoongateTransitStep::StageAClearCell { .. }))
            .count();
        let plots = steps
            .iter()
            .filter(|step| matches!(step, MoongateTransitStep::StageAPlotPixel { .. }))
            .count();
        assert_eq!(clears, 1, "the cell is first cleared to colour zero");
        assert_eq!(
            plots, MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS,
            "255 of the cell's 256 pixels are plotted, one per step"
        );
        assert_eq!(
            clears + plots,
            MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS,
            "256 dispatch steps for stage A, exact"
        );
        assert_eq!(MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS, 256);
        assert_eq!(MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS, 255);
        assert_eq!(TILE_ATLAS_TILE_PIXELS, 256, "the cell is one 16x16 tile");
    }

    #[test]
    fn stage_a_visit_order_is_the_shared_dissolve_primitive() {
        // `§9.2`: the pixels are plotted "in a fixed pseudo-random order",
        // and "the shuffle that orders the pixels never reaches one of
        // them". That is exactly an eight-bit maximal-length register over
        // 255 of 256 pixels, so this reuses the engine's existing
        // `DissolveVisitOrder` rather than adding a second dissolve.
        let order = moongate_transit_stage_a_pixel_order().unwrap();
        let mut shared = DissolveVisitOrder::new(MOONGATE_TRANSIT_STAGE_A_PLOTTED_PIXELS).unwrap();
        let mut expected = Vec::new();
        while let Some(index) = shared.next_index() {
            expected.push(index);
        }
        assert_eq!(
            order, expected,
            "stage A uses the shared dissolve visit order, not a second one"
        );

        let mut seen = vec![false; TILE_ATLAS_TILE_PIXELS];
        for pixel in &order {
            assert!(*pixel < TILE_ATLAS_TILE_PIXELS);
            assert!(!seen[*pixel], "no pixel is plotted twice");
            seen[*pixel] = true;
        }
        assert_eq!(
            seen.iter().filter(|plotted| !**plotted).count(),
            1,
            "exactly one of the 256 pixels is never reached"
        );
        // Not row-major and not column-major: the order reads as scattered
        // single-pixel updates, the same shape `display-driver-abi.md
        // §9.6` publishes for the driver-level dissolve.
        assert_ne!(order[0], 0);
        assert!(order.windows(2).any(|pair| pair[1] + 1 < pair[0]));
    }

    #[test]
    fn stage_a_paces_one_world_tick_every_eight_dispatch_steps() {
        // `§9.2`: stage A "is paced by a world tick every eight steps
        // rather than by a fixed wait, so it also advances ambient
        // animation while it runs".
        let steps = moongate_transit_steps().unwrap();
        let ticked: Vec<usize> = steps
            .iter()
            .filter_map(|step| match step {
                MoongateTransitStep::StageAPlotPixel {
                    dispatch_index,
                    world_tick: true,
                    ..
                } => Some(*dispatch_index),
                _ => None,
            })
            .collect();
        assert_eq!(
            ticked.len(),
            MOONGATE_TRANSIT_STAGE_A_WORLD_TICKS,
            "256 dispatch steps at one tick per eight is 32 ticks"
        );
        assert_eq!(MOONGATE_TRANSIT_STAGE_A_WORLD_TICKS, 32);
        for pair in ticked.windows(2) {
            assert_eq!(
                pair[1] - pair[0],
                MOONGATE_TRANSIT_STAGE_A_WORLD_TICK_EVERY,
                "the ticks are eight dispatch steps apart"
            );
        }

        // Step 1 opens the sequence with one world-tick pause of its own.
        let mut counter = MOONGATE_PHASE_FULL;
        let mut world_ticks = 0usize;
        run_moongate_transit(&mut counter, &mut |step, _| {
            world_ticks += step.world_ticks() as usize;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            world_ticks,
            1 + MOONGATE_TRANSIT_STAGE_A_WORLD_TICKS,
            "the opening pause plus stage A's pacing"
        );
    }

    #[test]
    fn stage_a_draws_the_party_as_the_scratch_tile_id() {
        // `§9.2`: "The party sprite is switched to tile `0x116`" - the
        // very id `§9.1` composes its gate frames into.
        assert_eq!(MOONGATE_TRANSIT_PARTY_VANISH_TILE, 0x116);
        assert_eq!(
            MOONGATE_TRANSIT_PARTY_VANISH_TILE, MOONGATE_PHASE_SCRATCH_TILE,
            "`§9.1`: the same id doubles as the party-vanishing sprite"
        );
        for step in moongate_transit_steps().unwrap() {
            match step {
                MoongateTransitStep::StageAClearCell { .. }
                | MoongateTransitStep::StageAPlotPixel { .. } => assert_eq!(
                    step.party_sprite(),
                    MoongateTransitPartySprite::Tile(MOONGATE_PHASE_SCRATCH_TILE)
                ),
                MoongateTransitStep::StageBPhase { .. }
                | MoongateTransitStep::ClearGateCell { .. } => assert_eq!(
                    step.party_sprite(),
                    MoongateTransitPartySprite::Suppressed,
                    "stage B suppresses the party sprite entirely"
                ),
                MoongateTransitStep::OpeningPause { .. } => {
                    assert_eq!(step.party_sprite(), MoongateTransitPartySprite::Party);
                }
            }
        }
    }

    #[test]
    fn stage_b_counts_fifteen_down_to_one_at_two_bios_ticks_a_phase() {
        // `§9.2`: "the shared presence counter is driven from `15` down to
        // `1`, one phase per step, with a wait of **two BIOS timer ticks**
        // between phases", and "The frame counts are `15` for stage B".
        let steps = moongate_transit_steps().unwrap();
        let phases: Vec<u8> = steps
            .iter()
            .filter_map(|step| match step {
                MoongateTransitStep::StageBPhase { phase, .. } => Some(*phase),
                _ => None,
            })
            .collect();
        assert_eq!(phases, (1..=15).rev().collect::<Vec<u8>>());
        assert_eq!(phases.len(), MOONGATE_TRANSIT_STAGE_B_STEPS);
        assert_eq!(MOONGATE_TRANSIT_STAGE_B_STEPS, 15);
        for step in &steps {
            if matches!(step, MoongateTransitStep::StageBPhase { .. }) {
                assert_eq!(
                    step.wait_bios_ticks(),
                    MOONGATE_TRANSIT_STAGE_B_STEP_BIOS_TICKS,
                    "two BIOS timer ticks between phases"
                );
                assert_eq!(
                    step.world_ticks(),
                    0,
                    "stage B is paced by BIOS ticks, not world ticks"
                );
            }
        }
        assert_eq!(MOONGATE_TRANSIT_STAGE_B_STEP_BIOS_TICKS, 2);
        // ~110 ms a phase at 18.2 Hz, ~1.65 s for the stage.
        assert_eq!(
            MoongateTransitPlayback::complete().stage_b_bios_ticks,
            30,
            "fifteen phases at two ticks each"
        );
    }

    #[test]
    fn the_transit_runs_every_published_step_in_one_call() {
        // `§9.2`: the hook "runs a **blocking** transition to completion
        // before the party is relocated and before any key is read ... it
        // cannot be skipped by the player - the abort poll that some other
        // presentation effects offer is disabled in overworld scenes".
        const { assert!(MOONGATE_TRANSIT_IS_BLOCKING) };
        const { assert!(!MOONGATE_TRANSIT_ABORT_POLL_ENABLED) };

        let mut counter = MOONGATE_PHASE_FULL;
        let mut seen = 0usize;
        let playback = run_moongate_transit(&mut counter, &mut |_, _| {
            seen += 1;
            Ok(())
        })
        .unwrap();
        assert_eq!(
            seen,
            moongate_transit_steps().unwrap().len(),
            "one call spends every dispatch step"
        );
        assert_eq!(playback, MoongateTransitPlayback::complete());
        assert!(playback.ran_to_completion);
    }

    #[test]
    fn the_countdown_ends_with_the_shared_counter_at_zero() {
        // `§9.2`: "The countdown ends with the counter at zero." The
        // counter walks 15..1 on the way, so a caller watching it sees the
        // gate sink rather than blink out.
        let mut counter = MOONGATE_PHASE_FULL;
        let mut walked = Vec::new();
        let playback = run_moongate_transit(&mut counter, &mut |_, phase| {
            walked.push(phase);
            Ok(())
        })
        .unwrap();
        assert_eq!(counter, MOONGATE_TRANSIT_END_COUNTER);
        assert_eq!(counter, 0);
        assert_eq!(playback.ended_counter, 0);
        let mut sink: Vec<u8> = Vec::new();
        for phase in walked {
            if sink.last() != Some(&phase) {
                sink.push(phase);
            }
        }
        assert_eq!(
            sink,
            (0..=MOONGATE_PHASE_FULL).rev().collect::<Vec<u8>>(),
            "the gate sinks phase by phase and lands on zero"
        );
    }

    #[test]
    fn stage_a_dissolves_the_cell_into_the_gate_leaving_one_pixel_at_colour_zero() {
        // `§9.2`: "the cell is first cleared to colour zero, then **255**
        // of its 256 pixels are plotted in a fixed pseudo-random order ...
        // a single pixel of the cell is left at colour zero when the stage
        // ends. It is repainted a moment later by step 4".
        let mut atlas = transit_atlas();
        let gate = tile_of(&atlas, moongate_phase_gate_tile() as usize);
        let ground = tile_of(&atlas, MOONGATE_PHASE_GROUND_TILE as usize);
        let mut counter = MOONGATE_PHASE_FULL;
        let (_, frames) = record_transit(&mut atlas, &mut counter);

        let cleared = frames
            .iter()
            .find(|frame| matches!(frame.step, MoongateTransitStep::StageAClearCell { .. }))
            .unwrap();
        assert!(
            cleared.cell.iter().all(|pixel| *pixel == 0),
            "the cell is first cleared to colour zero"
        );

        let last_stage_a = frames
            .iter()
            .rfind(|frame| matches!(frame.step, MoongateTransitStep::StageAPlotPixel { .. }))
            .unwrap();
        let unplotted: Vec<usize> = (0..TILE_ATLAS_TILE_PIXELS)
            .filter(|pixel| last_stage_a.cell[*pixel] != gate[*pixel])
            .collect();
        assert_eq!(unplotted.len(), 1, "255 of 256 pixels are plotted");
        assert_eq!(
            last_stage_a.cell[unplotted[0]], MOONGATE_TRANSIT_CLEAR_COLOUR,
            "the pixel the shuffle never reaches is left at colour zero"
        );

        // Step 4 rewrites the live cell to terrain `5` and repaints it,
        // which is what puts that stray pixel right.
        let repainted = frames.last().unwrap();
        assert!(matches!(
            repainted.step,
            MoongateTransitStep::ClearGateCell { terrain } if terrain == MOONGATE_TRANSIT_CLEARED_TERRAIN
        ));
        assert_eq!(MOONGATE_TRANSIT_CLEARED_TERRAIN, 5);
        assert_eq!(repainted.cell, ground);
    }

    #[test]
    fn stage_b_frames_are_the_gate_phase_composition() {
        // `§9.2`: "Each phase draws the composed frame of Section 9.1 at
        // the gate cell, so the visible effect is the gate sinking back
        // into the ground with the party already gone." One composition
        // routine, three callers - not a second frame builder here.
        let mut atlas = transit_atlas();
        let gate = tile_of(&atlas, moongate_phase_gate_tile() as usize);
        let ground = tile_of(&atlas, MOONGATE_PHASE_GROUND_TILE as usize);
        let mut counter = MOONGATE_PHASE_FULL;
        let (_, frames) = record_transit(&mut atlas, &mut counter);

        let mut phases = 0;
        for frame in &frames {
            let MoongateTransitStep::StageBPhase { phase, .. } = frame.step else {
                continue;
            };
            let mut expected = vec![0u8; TILE_ATLAS_TILE_PIXELS];
            compose_moongate_phase_frame(&mut expected, &ground, &gate, phase).unwrap();
            assert_eq!(
                frame.cell, expected,
                "stage B phase {phase} draws the `§9.1` composed frame"
            );
            assert_eq!(frame.party_pixels, None, "the party sprite is suppressed");
            phases += 1;
        }
        assert_eq!(phases, MOONGATE_TRANSIT_STAGE_B_STEPS);
    }

    #[test]
    fn the_transit_restores_the_scratch_tiles_shipped_artwork() {
        // `§9.1`: the composed frame goes into scratch tile `0x116`, "saved
        // and restored around every composition, so its shipped artwork
        // survives", and "the same id doubles as the party-vanishing
        // sprite in Section 9.2". Stage B writes that slot; stage A reads
        // it as the party. Both must see the shipped artwork afterwards.
        let mut atlas = transit_atlas();
        let shipped = tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE);
        assert!(shipped.iter().all(|pixel| *pixel == SCRATCH_SHIPPED_PIXEL));
        let mut counter = MOONGATE_PHASE_FULL;
        let (_, frames) = record_transit(&mut atlas, &mut counter);

        assert_eq!(
            tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE),
            shipped,
            "the transit leaves the scratch slot's shipped artwork intact"
        );
        let mut party_frames = 0;
        for frame in &frames {
            if !matches!(
                frame.step,
                MoongateTransitStep::StageAClearCell { .. }
                    | MoongateTransitStep::StageAPlotPixel { .. }
            ) {
                continue;
            }
            assert_eq!(
                frame.party_sprite,
                MoongateTransitPartySprite::Tile(MOONGATE_PHASE_SCRATCH_TILE)
            );
            assert_eq!(
                frame.party_pixels.as_deref(),
                Some(shipped.as_slice()),
                "the vanishing party is drawn from the shipped `0x116` artwork"
            );
            party_frames += 1;
        }
        assert_eq!(party_frames, MOONGATE_TRANSIT_STAGE_A_DISPATCH_STEPS);
    }

    #[test]
    fn a_transit_while_a_gate_is_mid_rise_leaves_the_scratch_artwork_intact() {
        // The collision the two `0x116` uses can produce: one gate is
        // mid-rise, so the renderer is composing frames into `0x116`
        // (`§9.1`), while a transit elsewhere draws the party from that
        // same id (`§9.2`). Because every composition saves and restores
        // the slot, the shipped artwork survives both.
        let mut atlas = transit_atlas();
        let shipped = tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE);
        let ground = tile_of(&atlas, MOONGATE_PHASE_GROUND_TILE as usize);
        let gate = tile_of(&atlas, moongate_phase_gate_tile() as usize);

        // A bystander gate seven of sixteen rows out of the ground, drawn
        // through the same scratch slot the transit is about to use.
        let mut counter = 7u8;
        let scratch_start = MOONGATE_PHASE_SCRATCH_TILE * TILE_ATLAS_TILE_PIXELS;
        let scratch_end = scratch_start + TILE_ATLAS_TILE_PIXELS;
        let mut mid_rise = Vec::new();
        with_moongate_phase_scratch_tile(
            &mut atlas.pixels[scratch_start..scratch_end],
            &ground,
            &gate,
            counter,
            |composed| mid_rise = composed.to_vec(),
        )
        .unwrap();
        assert_ne!(mid_rise, shipped, "the mid-rise frame is not the artwork");
        assert_eq!(
            tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE),
            shipped,
            "`§9.1` restores the slot after every composition"
        );

        let (playback, frames) = record_transit(&mut atlas, &mut counter);

        assert_eq!(
            tile_of(&atlas, MOONGATE_PHASE_SCRATCH_TILE),
            shipped,
            "a transit run while a gate is mid-rise leaves `0x116` authored pixels intact"
        );
        assert!(
            frames
                .iter()
                .filter_map(|frame| frame.party_pixels.as_deref())
                .all(|pixels| pixels == shipped.as_slice()),
            "the vanishing party never picks up a composed gate frame"
        );
        // And the shared counter: the bystander gate was mid-rise at seven
        // and this unrelated transit drove it to zero. `§9.1` calls that
        // "the original's behaviour, not a defect to design around".
        assert_eq!(counter, 0);
        assert_eq!(playback.ended_counter, 0);
    }

    #[test]
    fn the_entry_hook_plays_the_whole_transit_before_the_warp() {
        // `§9.2`: on `0xDC` the hook "runs a **blocking** transition to
        // completion before the party is relocated and before any key is
        // read". One key press therefore both plays the transit and lands
        // the party at the destination - nothing is left pending.
        let dir = debug_game_dir();
        let idx = world_cell_index(5, 5);
        let mut grid = open_world_grid();
        grid[idx] = NATURAL_MOONGATE_TERRAIN_TILE;
        let mut state = britannia_state(grid, 5, 5);
        state.clock = GameClock::new(11, 58).unwrap();
        state.natural_moongate_counter = MOONGATE_PHASE_FULL;
        state.set_cached_moon_glyph_slots(1, 0);
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: WorldPlane::Britannia.save_floor() as u8,
        };
        assert!(state.last_natural_moongate_transit.is_none());
        let transport_before = state.player.transport;

        assert_eq!(
            handle_play_key_input(&mut state, 'q', "", &dir).unwrap(),
            PlayInputDisposition::Continue
        );

        assert_eq!(
            state.last_natural_moongate_transit,
            Some(MoongateTransitPlayback::complete()),
            "the whole published sequence ran inside the one hook call"
        );
        let playback = state.last_natural_moongate_transit.unwrap();
        assert_eq!(playback.stage_a_dispatch_steps, 256);
        assert_eq!(playback.stage_a_plotted_pixels, 255);
        assert_eq!(playback.stage_a_world_ticks, 32);
        assert_eq!(playback.stage_b_phase_steps, 15);
        assert_eq!(playback.stage_b_first_phase, 15);
        assert_eq!(playback.stage_b_last_phase, 1);
        assert_eq!(playback.stage_b_bios_ticks, 30);
        assert!(playback.ran_to_completion);
        // Step 4, after the countdown: terrain `5` and a zeroed counter.
        assert_eq!(state.natural_moongate_counter, 0);
        assert_eq!(state.grid[idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
        assert!(state.natural_moongate_live_cells.is_empty());
        // `§9.2`: "The party's transport marker is restored on every path
        // that ran the transition." The vanishing sprite of stage A is a
        // presentation substitution, so the party is on foot on the far
        // side of the gate exactly as it was on this one.
        assert_eq!(state.player.transport, transport_before);
        // The party has already been relocated by the time the hook
        // returns; the transit played at the gate cell it left, never at
        // the destination.
        assert_eq!((state.player.x, state.player.y), (6, 7));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn stage_a_world_ticks_do_not_advance_the_gate_presence_counter() {
        // `§9.1`: the presence counter "is not advanced by the animation
        // tick, it has no frame selector". Stage A's 32 world ticks
        // advance ambient animation and nothing else; only stage B moves
        // the counter.
        let mut counter = MOONGATE_PHASE_FULL;
        let mut animation = AnimationClock::default();
        let mut during_stage_a = Vec::new();
        run_moongate_transit(&mut counter, &mut |step, phase| {
            for _ in 0..step.world_ticks() {
                animation.tick_static_tiles();
            }
            if matches!(
                step,
                MoongateTransitStep::StageAClearCell { .. }
                    | MoongateTransitStep::StageAPlotPixel { .. }
            ) {
                during_stage_a.push(phase);
            }
            Ok(())
        })
        .unwrap();
        assert!(
            during_stage_a
                .iter()
                .all(|phase| *phase == MOONGATE_PHASE_FULL),
            "the counter sits still through stage A"
        );
        // Ambient animation did move, which is the point of pacing stage
        // A by world ticks: 33 ticks - the opening pause plus stage A's 32
        // - against an `animation.md §6` period of eight.
        assert_eq!(
            animation,
            AnimationClock::at_static_tile_phase(
                (1 + MOONGATE_TRANSIT_STAGE_A_WORLD_TICKS) as u8
                    % STATIC_TILE_ANIMATION_PERIOD_TICKS
            )
        );
        assert_eq!(animation, AnimationClock::at_static_tile_phase(1));
    }
}

// `visibility.md §12.6`: the night-time rotating light beacon.
//
// The beacon is a rotating *beam*, distinct from the disc-shaped local
// light sources of `§12.1`-`§12.3`, writing into the same 32x32
// local-light mask. It owns the resident scratch block earlier spec
// revisions misattributed to a moongate animator; that attribution is
// withdrawn in full, and nothing here draws a gate.
//
// These tests are written against the light gate as `§12.6` states it —
// **strictly below** the full-daylight value of fifty, i.e. from the first
// step of the dusk ramp to the last step of the dawn ramp. That is the
// opposite polarity to the withdrawn `ambient >= FULL_DAYLIGHT` reading
// this tree used to carry.

mod light_beacon_sources {
    use super::*;

    fn floor_with(tiles: &[((usize, usize), u8)]) -> Vec<u8> {
        let mut grid = vec![16u8; TOWN_GRID_BYTES];
        for &((x, y), tile) in tiles {
            grid[y * TOWN_GRID_SIDE + x] = tile;
        }
        grid
    }

    /// `visibility.md §12.6`: "the chunk loader scans each freshly loaded
    /// thirty-two-by-thirty-two window for the lighthouse tile and records
    /// the first hit as the single beacon position ... It never fills the
    /// second position."
    #[test]
    fn outdoor_harvest_records_first_lighthouse_hit_and_never_the_second_slot() {
        let window = floor_with(&[
            ((20, 3), BEACON_LIGHTHOUSE_TILE),
            ((4, 9), BEACON_LIGHTHOUSE_TILE),
        ]);
        let sources = harvest_outdoor_beacon_sources((0, 0), |x, y| {
            window[y * TOWN_GRID_SIDE + x]
        });

        assert_eq!(sources[0], Some((20, 3)), "first hit in scan order");
        assert_eq!(
            sources[1], None,
            "the outdoor harvest never fills the second slot"
        );
    }

    /// `visibility.md §12.6`: the loader "records a 'no beacon' sentinel
    /// when the window holds none".
    #[test]
    fn outdoor_harvest_records_no_beacon_sentinel_for_a_window_without_a_lighthouse() {
        let window = floor_with(&[((5, 5), BEACON_BRIGHT_LIGHT_TILE)]);
        let sources = harvest_outdoor_beacon_sources((0, 0), |x, y| {
            window[y * TOWN_GRID_SIDE + x]
        });

        assert_eq!(sources, [None; BEACON_SOURCE_SLOTS]);
    }

    /// The window origin is a world coordinate, and the recorded position
    /// is the source's world cell, not its offset inside the window.
    #[test]
    fn outdoor_harvest_records_world_coordinates_not_window_offsets() {
        let window = floor_with(&[((2, 1), BEACON_LIGHTHOUSE_TILE)]);
        let origin = (96, 112);
        let sources = harvest_outdoor_beacon_sources(origin, |x, y| {
            let (local_x, local_y) = (x - origin.0, y - origin.1);
            window[local_y * TOWN_GRID_SIDE + local_x]
        });

        assert_eq!(sources[0], Some((98, 113)));
    }

    /// `visibility.md §12.6`: "inside a location, the map setup clears both
    /// positions and then records up to **two** hits on the bright-light
    /// tile".
    #[test]
    fn location_harvest_records_at_most_two_bright_lights() {
        let grid = floor_with(&[
            ((11, 12), BEACON_BRIGHT_LIGHT_TILE),
            ((19, 12), BEACON_BRIGHT_LIGHT_TILE),
            ((3, 20), BEACON_BRIGHT_LIGHT_TILE),
        ]);

        let sources = harvest_location_beacon_sources(&grid);
        assert_eq!(sources[0], Some((11, 12)));
        assert_eq!(sources[1], Some((19, 12)));
    }

    #[test]
    fn location_harvest_records_no_beacon_sentinel_for_a_floor_without_a_bright_light() {
        let grid = floor_with(&[((8, 8), BEACON_LIGHTHOUSE_TILE)]);
        assert_eq!(
            harvest_location_beacon_sources(&grid),
            [None; BEACON_SOURCE_SLOTS]
        );
    }

    /// "Map setup **clears both positions**" first, so a floor with one
    /// bright light leaves the second slot at the sentinel even when the
    /// previous map had filled it.
    #[test]
    fn location_harvest_clears_both_positions_before_recording() {
        let mut state = test_state(open_grid(), 4, 4);
        state.light_beacon.sources = [Some((1, 1)), Some((2, 2))];
        state.grid = floor_with(&[((7, 9), BEACON_BRIGHT_LIGHT_TILE)]);

        state.harvest_location_light_beacon();

        assert_eq!(state.light_beacon.sources[0], Some((7, 9)));
        assert_eq!(state.light_beacon.sources[1], None);
    }

    /// `visibility.md §12.6`: "combat entry switches the beacon off
    /// outright."
    #[test]
    fn combat_entry_switches_the_beacon_off() {
        let mut beacon = LightBeaconState {
            sources: [Some((11, 12)), Some((19, 12))],
            bearing: 6,
        };

        beacon.switch_off();

        assert!(beacon.is_off());
        assert_eq!(beacon.sources, [None; BEACON_SOURCE_SLOTS]);
        assert_eq!(beacon.bearing, BEACON_INITIAL_BEARING);
    }

    /// The shipped data image "starts with both positions at the 'no
    /// beacon' sentinel, so nothing is lit until a loader finds a source".
    #[test]
    fn default_beacon_state_is_both_sentinels_at_the_initial_bearing() {
        let beacon = LightBeaconState::default();
        assert!(beacon.is_off());
        assert_eq!(beacon.bearing, BEACON_INITIAL_BEARING);
    }
}

mod light_beacon_light_gate {
    use super::*;

    /// `visibility.md §12.6`: the pass "runs only while the value is
    /// **strictly below** fifty — that is, from the first step of the dusk
    /// ramp until the last step of the dawn ramp".
    ///
    /// `lighting.md §3` makes 49 the first dusk step and the last dawn
    /// step, and 50 the full-daylight band, so the boundary is exactly
    /// there.
    #[test]
    fn gate_runs_strictly_below_full_daylight_at_both_ramp_boundaries() {
        assert_eq!(DAWN_DUSK_LIGHT[5], FULL_DAYLIGHT - 1);
        assert!(
            beacon_pass_runs(DAWN_DUSK_LIGHT[5]),
            "the brightest ramp step still runs the beacon"
        );
        assert!(beacon_pass_runs(FULL_DAYLIGHT - 1));
        assert!(
            !beacon_pass_runs(FULL_DAYLIGHT),
            "full daylight is the clear-and-draw-nothing path"
        );
        assert!(!beacon_pass_runs(FULL_DAYLIGHT + 1));
    }

    #[test]
    fn gate_runs_through_every_ramp_step_and_full_darkness() {
        assert!(beacon_pass_runs(FULL_DARKNESS));
        for step in DAWN_DUSK_LIGHT {
            assert!(beacon_pass_runs(step), "ramp step {step} runs the beacon");
        }
    }

    /// The gate is a day/night test, not a distance threshold
    /// (`lighting.md §7.2`): it never widens or narrows with the value, it
    /// only switches.
    #[test]
    fn gate_is_a_day_night_switch_over_the_whole_ambient_range() {
        for ambient in 0..=u8::MAX {
            assert_eq!(beacon_pass_runs(ambient), ambient < 50);
        }
    }

    /// "At or above fifty the pass clears its state and draws nothing, and
    /// the rotation restarts from its initial bearing the next time
    /// darkness falls."
    #[test]
    fn daylight_clears_beam_state_and_resets_the_bearing() {
        let mut state = test_state(open_grid(), 4, 4);
        state.light_beacon.sources = [Some((7, 9)), None];
        state.light_beacon.bearing = 11;
        state.ambient_light = FULL_DAYLIGHT;
        state.visibility_dirty = false;

        state.advance_light_beacon();

        assert_eq!(state.light_beacon.bearing, BEACON_INITIAL_BEARING);
        assert!(
            state.visibility_dirty,
            "the pass sets the visibility-dirty flag when it changes anything"
        );

        // Already cleared: nothing changes, so nothing is dirtied.
        state.visibility_dirty = false;
        state.advance_light_beacon();
        assert_eq!(state.light_beacon.bearing, BEACON_INITIAL_BEARING);
        assert!(!state.visibility_dirty);
    }

    /// The next nightfall restarts from the initial bearing rather than
    /// resuming where daylight interrupted it.
    #[test]
    fn rotation_restarts_from_the_initial_bearing_after_daylight() {
        let mut state = test_state(open_grid(), 4, 4);
        state.light_beacon.sources = [Some((7, 9)), None];
        state.ambient_light = FULL_DARKNESS;
        for _ in 0..5 {
            state.advance_light_beacon();
        }
        assert_ne!(state.light_beacon.bearing, BEACON_INITIAL_BEARING);

        state.ambient_light = FULL_DAYLIGHT;
        state.advance_light_beacon();
        state.ambient_light = FULL_DARKNESS;
        state.advance_light_beacon();

        assert_eq!(
            state.light_beacon.bearing,
            beacon_next_bearing(BEACON_INITIAL_BEARING)
        );
    }
}

mod light_beacon_rotation {
    use super::*;

    /// `visibility.md §12.6`: "the cone advances one sixteenth of a
    /// revolution per turn and completes a full revolution every sixteen
    /// turns. The bearing counter wraps at sixteen."
    #[test]
    fn rotation_advances_one_bearing_per_turn_and_wraps_at_sixteen() {
        let mut state = test_state(open_grid(), 4, 4);
        state.light_beacon.sources = [Some((7, 9)), None];
        state.ambient_light = FULL_DARKNESS;

        for expected in 1..=BEACON_BEARING_COUNT {
            state.advance_light_beacon();
            assert_eq!(
                state.light_beacon.bearing,
                expected % BEACON_BEARING_COUNT,
                "turn {expected}"
            );
        }
        assert_eq!(
            state.light_beacon.bearing, BEACON_INITIAL_BEARING,
            "a full revolution is sixteen turns"
        );
    }

    #[test]
    fn next_bearing_wraps_at_sixteen() {
        assert_eq!(beacon_next_bearing(0), 1);
        assert_eq!(beacon_next_bearing(BEACON_BEARING_COUNT - 1), 0);
    }

    /// A beacon with both positions at the sentinel changes nothing, so it
    /// does not rotate and does not set the visibility-dirty flag.
    #[test]
    fn a_beacon_with_no_source_does_not_rotate_or_dirty_visibility() {
        let mut state = test_state(open_grid(), 4, 4);
        state.ambient_light = FULL_DARKNESS;
        state.visibility_dirty = false;

        state.advance_light_beacon();

        assert_eq!(state.light_beacon.bearing, BEACON_INITIAL_BEARING);
        assert!(!state.visibility_dirty);
    }

    /// "Three adjacent bearings are lit at any moment — a cone roughly
    /// three sixteenths of the compass wide."
    #[test]
    fn the_cone_is_three_adjacent_bearings_and_wraps() {
        assert_eq!(beacon_cone_bearings(0), [0, 1, 2]);
        assert_eq!(beacon_cone_bearings(7), [7, 8, 9]);
        assert_eq!(beacon_cone_bearings(14), [14, 15, 0]);
        assert_eq!(beacon_cone_bearings(15), [15, 0, 1]);
        for bearing in 0..BEACON_BEARING_COUNT {
            let cone = beacon_cone_bearings(bearing);
            assert_eq!(cone.len(), BEACON_CONE_BEARINGS);
            assert_eq!(cone[1], beacon_next_bearing(cone[0]));
            assert_eq!(cone[2], beacon_next_bearing(cone[1]));
        }
    }

    /// One turn "clears the trailing bearing and lights the next leading
    /// bearing", so consecutive cones overlap in exactly two bearings.
    #[test]
    fn one_turn_swaps_the_trailing_bearing_for_the_next_leading_one() {
        for bearing in 0..BEACON_BEARING_COUNT {
            let before = beacon_cone_bearings(bearing);
            let after = beacon_cone_bearings(beacon_next_bearing(bearing));
            let shared = before.iter().filter(|b| after.contains(b)).count();
            assert_eq!(shared, BEACON_CONE_BEARINGS - 1, "bearing {bearing}");
            assert!(!after.contains(&before[0]), "trailing bearing cleared");
            assert!(!before.contains(&after[2]), "leading bearing added");
        }
    }
}

mod light_beacon_stencils {
    use super::*;

    /// A scratch directory holding only the shipped files these tests
    /// read. The pristine install is never opened directly and never
    /// written to (`CLAUDE.md` clean-room rules).
    fn shipped_asset_copy(files: &[&str]) -> Option<std::path::PathBuf> {
        let source = Path::new(DEFAULT_GAME_DIR);
        if files.iter().any(|file| !source.join(file).exists()) {
            return None;
        }
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("u5-beacon-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert_writable_game_dir(&dir, "beacon stencil fixture");
        for file in files {
            copy_asset_writable(&source.join(file), &dir.join(file)).unwrap();
        }
        Some(dir)
    }

    fn shipped_stencils() -> Option<BeaconBearingStencils> {
        let dir = shipped_asset_copy(&[DATA_OVL_FILENAME])?;
        let table = load_beacon_bearing_stencils(&dir).unwrap();
        let _ = fs::remove_dir_all(&dir);
        table
    }

    /// `visibility.md §12.6`: "each bearing is a fixed set of at most
    /// sixteen cell offsets relative to the source, so a bearing is a
    /// stencil, not a computed sweep", and the beam reaches "up to seven
    /// tiles from the source".
    ///
    /// The offsets are shipped data, not published prose, so this locates
    /// the table structurally in `DATA.OVL` (`formats/tiles.md §5.1.1`:
    /// sixteen thirty-two-byte records of sixteen signed byte pairs) and
    /// checks the published shape against it.
    #[test]
    fn shipped_stencils_have_at_most_sixteen_offsets_reaching_seven_tiles() {
        let Some(stencils) = shipped_stencils() else {
            return;
        };

        let mut longest_reach = 0;
        for bearing in 0..BEACON_BEARING_COUNT {
            let offsets = stencils.bearing(bearing);
            assert!(!offsets.is_empty(), "bearing {bearing} lights cells");
            assert!(
                offsets.len() <= BEACON_STENCIL_MAX_OFFSETS,
                "bearing {bearing} has {} offsets",
                offsets.len()
            );
            for &(dx, dy) in offsets {
                assert_ne!((dx, dy), (0, 0), "the source cell is not an offset");
                let reach = dx.unsigned_abs().max(dy.unsigned_abs());
                assert!(reach <= BEACON_BEAM_MAX_REACH, "reach {reach}");
                longest_reach = longest_reach.max(reach);
            }
        }
        assert_eq!(
            longest_reach, BEACON_BEAM_MAX_REACH,
            "the beam reaches its full seven tiles somewhere"
        );
    }

    /// "Bearing one points due north, five due east, nine due south,
    /// thirteen due west, four bearings fall on the diagonals, and the
    /// remaining eight sit halfway between those."
    ///
    /// `formats/tiles.md §5.1.1` indexes the records modulo sixteen, which
    /// puts bearing sixteen at index zero.
    #[test]
    fn shipped_stencils_point_along_the_published_compass() {
        let Some(stencils) = shipped_stencils() else {
            return;
        };

        // Screen coordinates: +x east, +y south.
        type BearingPredicate = fn(i8, i8) -> bool;
        let cardinals: [(u8, BearingPredicate); 4] = [
            (1, |dx, dy| dy < 0 && dx.abs() <= 1),
            (5, |dx, dy| dx > 0 && dy.abs() <= 1),
            (9, |dx, dy| dy > 0 && dx.abs() <= 1),
            (13, |dx, dy| dx < 0 && dy.abs() <= 1),
        ];
        for (bearing, predicate) in cardinals {
            for &(dx, dy) in stencils.bearing(bearing) {
                assert!(predicate(dx, dy), "bearing {bearing} offset ({dx}, {dy})");
            }
        }

        // The four diagonals: 3 north-east, 7 south-east, 11 south-west,
        // 15 north-west.
        for (bearing, sx, sy) in [(3u8, 1i8, -1i8), (7, 1, 1), (11, -1, 1), (15, -1, -1)] {
            for &(dx, dy) in stencils.bearing(bearing) {
                assert_eq!(dx.signum(), sx, "bearing {bearing} offset ({dx}, {dy})");
                assert_eq!(dy.signum(), sy, "bearing {bearing} offset ({dx}, {dy})");
            }
        }

        // Bearing sixteen is one step anticlockwise of due north, so it
        // leans west while still pointing north.
        for &(dx, dy) in stencils.bearing(0) {
            assert!(dx < 0 && dy < 0, "bearing sixteen offset ({dx}, {dy})");
        }
    }

    /// The table is located by shape, and the shipped image yields exactly
    /// one candidate — a rejected or ambiguous image is an error, never a
    /// silently dark beacon.
    #[test]
    fn stencil_table_location_refuses_an_image_without_a_unique_table() {
        let err = find_beacon_bearing_stencils(&[0; BEACON_STENCIL_TABLE_BYTES]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);

        let err = find_beacon_bearing_stencils(&[0; 8]).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    /// A record whose live pairs are followed by padding is accepted; one
    /// with a live pair *after* padding, or an offset past seven cells, or
    /// an offset pointing away from its bearing, is not.
    #[test]
    fn stencil_parser_enforces_the_published_record_shape() {
        let mut bytes = vec![0u8; BEACON_STENCIL_TABLE_BYTES];
        let write = |bytes: &mut [u8], index: usize, pairs: &[(i8, i8)]| {
            let start = index * BEACON_STENCIL_RECORD_BYTES;
            for slot in 0..BEACON_STENCIL_MAX_OFFSETS {
                let (dx, dy) = pairs.get(slot).copied().unwrap_or((0, 0));
                bytes[start + slot * 2] = dx as u8;
                bytes[start + slot * 2 + 1] = dy as u8;
            }
        };
        // Index r carries heading (r - 1) * 22.5 degrees clockwise from
        // north, so index 1 is due north and index 9 is due south.
        let heading_offsets: [(i8, i8); BEACON_BEARING_COUNT as usize] = [
            (-1, -2),
            (0, -1),
            (1, -2),
            (1, -1),
            (2, -1),
            (1, 0),
            (2, 1),
            (1, 1),
            (1, 2),
            (0, 1),
            (-1, 2),
            (-1, 1),
            (-2, 1),
            (-1, 0),
            (-2, -1),
            (-1, -1),
        ];
        for (index, offset) in heading_offsets.iter().enumerate() {
            write(&mut bytes, index, &[*offset]);
        }
        assert!(parse_beacon_bearing_stencils(&bytes).is_some());

        let mut padded = bytes.clone();
        padded[BEACON_STENCIL_RECORD_BYTES + 4] = 0;
        padded[BEACON_STENCIL_RECORD_BYTES + 5] = (-1i8) as u8;
        assert!(
            parse_beacon_bearing_stencils(&padded).is_none(),
            "a live pair after padding is rejected"
        );

        let mut too_far = bytes.clone();
        too_far[BEACON_STENCIL_RECORD_BYTES + 1] = (-8i8) as u8;
        assert!(
            parse_beacon_bearing_stencils(&too_far).is_none(),
            "an offset past seven tiles is rejected"
        );

        let mut wrong_way = bytes.clone();
        wrong_way[BEACON_STENCIL_RECORD_BYTES + 1] = 1;
        assert!(
            parse_beacon_bearing_stencils(&wrong_way).is_none(),
            "an offset pointing away from its bearing is rejected"
        );

        let empty = vec![0u8; BEACON_STENCIL_TABLE_BYTES];
        assert!(parse_beacon_bearing_stencils(&empty).is_none());
    }
}

mod light_beacon_mask_stamp {
    use super::*;

    fn stencils_for_test() -> BeaconBearingStencils {
        let mut bytes = vec![0u8; BEACON_STENCIL_TABLE_BYTES];
        // One offset per bearing, each along its own heading; enough to
        // exercise the stamp without depending on shipped geometry.
        let heading_offsets: [(i8, i8); BEACON_BEARING_COUNT as usize] = [
            (-1, -2),
            (0, -1),
            (1, -2),
            (1, -1),
            (2, -1),
            (1, 0),
            (2, 1),
            (1, 1),
            (1, 2),
            (0, 1),
            (-1, 2),
            (-1, 1),
            (-2, 1),
            (-1, 0),
            (-2, -1),
            (-1, -1),
        ];
        for (index, (dx, dy)) in heading_offsets.iter().enumerate() {
            let start = index * BEACON_STENCIL_RECORD_BYTES;
            bytes[start] = *dx as u8;
            bytes[start + 1] = *dy as u8;
        }
        parse_beacon_bearing_stencils(&bytes).unwrap()
    }

    fn beacon_state_at(source: (u8, u8), bearing: u8, ambient: u8) -> PlayState {
        let mut state = test_state(open_grid(), 16, 16);
        state.beacon_bearing_stencils = Some(stencils_for_test());
        state.light_beacon = LightBeaconState {
            sources: [Some(source), None],
            bearing,
        };
        state.ambient_light = ambient;
        state
    }

    fn lit_cells(state: &PlayState) -> Vec<(isize, isize)> {
        let mut mask = vec![false; TOWN_GRID_BYTES];
        state.stamp_light_beacon(&mut mask, 0, 0, false);
        mask.iter()
            .enumerate()
            .filter(|(_, lit)| **lit)
            .map(|(index, _)| {
                (
                    (index % LOCAL_LIGHT_MASK_SIDE) as isize,
                    (index / LOCAL_LIGHT_MASK_SIDE) as isize,
                )
            })
            .collect()
    }

    /// `visibility.md §12.6`: "lit cells are written straight into the
    /// local-light mask", one cone of three adjacent bearings per source.
    #[test]
    fn the_stamp_lights_the_three_cone_bearings_of_every_source() {
        let state = beacon_state_at((10, 10), 1, FULL_DARKNESS);
        let mut lit = lit_cells(&state);
        lit.sort_unstable();

        // Bearings 1, 2, 3 of the fixture: (0, -1), (1, -2), (1, -1).
        let mut expected = vec![(10, 9), (11, 8), (11, 9)];
        expected.sort_unstable();
        assert_eq!(lit, expected);
    }

    #[test]
    fn both_source_slots_stamp_and_overlapping_cones_union() {
        let mut state = beacon_state_at((10, 10), 1, FULL_DARKNESS);
        state.light_beacon.sources[1] = Some((11, 10));
        let lit = lit_cells(&state);

        assert!(lit.contains(&(10, 9)), "first source lit");
        assert!(lit.contains(&(11, 9)), "shared cell lit once");
        assert!(lit.contains(&(12, 8)), "second source lit");
    }

    /// The gate comes first: at or above full daylight the pass "draws
    /// nothing".
    #[test]
    fn the_stamp_draws_nothing_at_or_above_full_daylight() {
        let state = beacon_state_at((10, 10), 1, FULL_DAYLIGHT);
        assert!(lit_cells(&state).is_empty());

        let state = beacon_state_at((10, 10), 1, FULL_DAYLIGHT - 1);
        assert!(!lit_cells(&state).is_empty());
    }

    #[test]
    fn the_stamp_draws_nothing_without_a_source() {
        let mut state = beacon_state_at((10, 10), 1, FULL_DARKNESS);
        state.light_beacon.switch_off();
        assert!(lit_cells(&state).is_empty());
    }

    /// Cells the cone would light outside the 32x32 window are dropped
    /// rather than wrapped into the wrong row.
    #[test]
    fn the_stamp_drops_cells_outside_the_window() {
        let state = beacon_state_at((10, 0), 1, FULL_DARKNESS);
        for (_, y) in lit_cells(&state) {
            assert!((0..LOCAL_LIGHT_MASK_SIDE as isize).contains(&y));
        }
    }

    /// A beacon whose stencils were never loaded lights nothing; the
    /// engine never substitutes an invented geometry.
    #[test]
    fn the_stamp_draws_nothing_without_a_loaded_stencil_table() {
        let mut state = beacon_state_at((10, 10), 1, FULL_DARKNESS);
        state.beacon_bearing_stencils = None;
        assert!(lit_cells(&state).is_empty());
    }
}

mod light_beacon_shipped_map_sources {
    use super::*;

    fn shipped_world_copy() -> Option<std::path::PathBuf> {
        let source = Path::new(DEFAULT_GAME_DIR);
        let files = [DATA_OVL_FILENAME, "BRIT.DAT", "UNDER.DAT"];
        if files.iter().any(|file| !source.join(file).exists()) {
            return None;
        }
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir()
            .join(format!("u5-beacon-world-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        assert_writable_game_dir(&dir, "beacon world fixture");
        for file in files {
            copy_asset_writable(&source.join(file), &dir.join(file)).unwrap();
        }
        Some(dir)
    }

    /// `catalogs/gazetteer.md §8.1`: "the four lighthouses. Stormcrow
    /// `(152, 24)`, Fogsbane `(88, 120)`, Waveguide `(216, 120)`, and
    /// Greyhaven `(104, 216)`", each also "the outdoor night-time light
    /// source". `visibility.md §12.6` adds that "the Underworld map
    /// contains none, so the outdoor beacon is a surface-only effect".
    #[test]
    fn shipped_surface_map_carries_exactly_the_four_published_lighthouses() {
        let Some(dir) = shipped_world_copy() else {
            return;
        };

        let britannia = load_world_map(&dir, WorldPlane::Britannia).unwrap();
        let underworld = load_world_map(&dir, WorldPlane::Underworld).unwrap();
        let _ = fs::remove_dir_all(&dir);

        let published = [(152usize, 24usize), (88, 120), (216, 120), (104, 216)];
        for (x, y) in published {
            assert_eq!(
                britannia[world_cell_index(x, y)],
                BEACON_LIGHTHOUSE_TILE,
                "gazetteer §8.1 lighthouse at ({x}, {y})"
            );
        }

        let surface_hits = britannia
            .iter()
            .filter(|tile| **tile == BEACON_LIGHTHOUSE_TILE)
            .count();
        assert_eq!(surface_hits, published.len());

        assert_eq!(
            underworld
                .iter()
                .filter(|tile| **tile == BEACON_LIGHTHOUSE_TILE)
                .count(),
            0,
            "the Underworld map contains no lighthouse"
        );

        // The bright light is the indoor source only: it appears on
        // neither outdoor map (`visibility.md §12.6`).
        for map in [&britannia, &underworld] {
            assert_eq!(
                map.iter()
                    .filter(|tile| **tile == BEACON_BRIGHT_LIGHT_TILE)
                    .count(),
                0
            );
        }
    }

    /// End to end over shipped data: the chunk loader's window around a
    /// published lighthouse harvests it as the single beacon source, and
    /// the shipped stencils then light real cells around it after dark
    /// (`visibility.md §12.6`).
    #[test]
    fn a_shipped_lighthouse_window_harvests_and_lights_after_dark() {
        let Some(dir) = shipped_world_copy() else {
            return;
        };

        let grid = load_world_map(&dir, WorldPlane::Britannia).unwrap();
        // Stormcrow, `catalogs/gazetteer.md §8.1`.
        let (lighthouse_x, lighthouse_y) = (152usize, 24usize);
        let buffer = WorldLiveChunkBuffer::from_full_grid(
            WorldPlane::Britannia,
            &grid,
            lighthouse_x,
            lighthouse_y,
            |_| false,
        )
        .unwrap();
        let sources =
            harvest_outdoor_beacon_sources(buffer.scroll_base, |x, y| buffer.tile_at(x, y));
        assert_eq!(
            sources[0],
            Some((lighthouse_x as u8, lighthouse_y as u8)),
            "the loaded window harvests its lighthouse"
        );
        assert_eq!(sources[1], None);

        let stencils = load_beacon_bearing_stencils(&dir).unwrap();
        let _ = fs::remove_dir_all(&dir);
        let Some(stencils) = stencils else {
            return;
        };

        let mut state = world_state(grid, lighthouse_x, lighthouse_y);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.world_live_chunks = Some(buffer);
        state.beacon_bearing_stencils = Some(stencils);
        state.light_beacon = LightBeaconState {
            sources,
            bearing: BEACON_INITIAL_BEARING,
        };

        let origin_x = lighthouse_x as isize - (LOCAL_LIGHT_MASK_SIDE / 2) as isize;
        let origin_y = lighthouse_y as isize - (LOCAL_LIGHT_MASK_SIDE / 2) as isize;

        state.ambient_light = FULL_DARKNESS;
        let mut night = vec![false; TOWN_GRID_BYTES];
        state.stamp_light_beacon(&mut night, origin_x, origin_y, true);
        let night_cells = night.iter().filter(|lit| **lit).count();
        assert!(night_cells > 0, "the beam lights cells after dark");
        assert!(
            night_cells <= BEACON_CONE_BEARINGS * BEACON_STENCIL_MAX_OFFSETS,
            "one cone of three bearings, {night_cells} cells"
        );

        state.ambient_light = FULL_DAYLIGHT;
        let mut day = vec![false; TOWN_GRID_BYTES];
        state.stamp_light_beacon(&mut day, origin_x, origin_y, true);
        assert!(
            day.iter().all(|lit| !lit),
            "the beam draws nothing in full daylight"
        );
    }

    /// Our world-location table's four lighthouse rows carry exactly the
    /// coordinates `catalogs/gazetteer.md §8.1` publishes.
    #[test]
    fn world_location_table_lighthouse_rows_match_the_gazetteer() {
        let published = [
            (SCENE_FOGSBANE, 88u8, 120u8),
            (SCENE_STORMCROW, 152, 24),
            (SCENE_GREYHAVEN, 104, 216),
            (SCENE_WAVEGUIDE, 216, 120),
        ];
        let entries = published_world_location_entries();
        for (scene, x, y) in published {
            let row = entries
                .iter()
                .find(|entry| entry.target == PlayTarget::Town(Scene::new(scene).unwrap()))
                .unwrap_or_else(|| panic!("no world-location row for scene {scene}"));
            assert_eq!(
                (row.plane, row.x, row.y),
                (WorldPlane::Britannia, usize::from(x), usize::from(y))
            );
        }
    }
}

mod light_beacon_floor_transition_harvest {
    use super::*;

    /// `visibility.md §12.6`: a location floor's beacon sources must be
    /// harvested from the **raw** floor, before the runtime normalisation
    /// pass rewrites any cell.
    ///
    /// This is a regression test for a divergence between our own two entry
    /// paths, not for a spec disagreement. `load_town_scene` already
    /// harvested from the raw grid; every floor *transition* loaded through
    /// `load_town_runtime_floor` and then harvested from the **scrubbed**
    /// result. `scrub_location_entry_markers` rewrites the marker byte the
    /// beacon looks for, so a floor reached by stairs found no source while
    /// the same floor reached by initial load found one.
    ///
    /// It mattered because the four shipped floors carrying that tile are
    /// lighthouse lantern rooms, and a lighthouse lantern room is reached by
    /// stairs — so the broken path was the only one that could occur in
    /// play. Nothing in the 493-case route suite reaches it either way.
    #[test]
    fn normalisation_scrubs_the_byte_the_beacon_harvests() {
        let mut grid = vec![0u8; 32 * 32];
        grid[5 * 32 + 7] = BEACON_BRIGHT_LIGHT_TILE;

        let raw_sources = harvest_location_beacon_sources(&grid);
        assert_eq!(
            raw_sources[0],
            Some((7, 5)),
            "the raw floor must yield the bright-light cell"
        );

        // The normalisation pass rewrites it, so harvesting afterwards finds
        // nothing. This assertion is the *cause* of the bug, pinned so that
        // if the scrub ever stops claiming this byte the ordering fix can be
        // simplified deliberately rather than by accident.
        normalize_town_runtime_floor(&mut grid, 12);
        assert_ne!(
            grid[5 * 32 + 7],
            BEACON_BRIGHT_LIGHT_TILE,
            "normalisation is expected to rewrite the marker byte"
        );
        assert_eq!(
            harvest_location_beacon_sources(&grid),
            [None, None],
            "harvesting after normalisation finds nothing - which is why every \
             floor-transition path must harvest before it"
        );
    }
}

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
        ]);

        let sources = harvest_location_beacon_sources(&grid);
        assert_eq!(sources[0], Some((11, 12)));
        assert_eq!(sources[1], Some((19, 12)));
    }

    /// `formats/location-dat.md §6`, the harvest rule as corrected: "the
    /// walk tests only whether the *first* slot is still empty. So the
    /// **first** hit takes slot one and, once slot one is filled, **every
    /// later hit overwrites slot two** — meaning the **last** hit wins slot
    /// two, not the second."
    ///
    /// This tree implemented first-hit-then-second-hit and stopped walking
    /// once both slots were filled. No shipped floor carries three sources
    /// (see `shipped_location_files_carry_the_published_beacon_source_layout`),
    /// so only custom data can tell the two rules apart — which is why it
    /// needs a fixture rather than an asset.
    #[test]
    fn a_third_bright_light_overwrites_the_second_slot_rather_than_being_ignored() {
        let grid = floor_with(&[
            ((3, 20), BEACON_BRIGHT_LIGHT_TILE),
            ((11, 12), BEACON_BRIGHT_LIGHT_TILE),
            ((19, 12), BEACON_BRIGHT_LIGHT_TILE),
        ]);

        let sources = harvest_location_beacon_sources(&grid);

        assert_eq!(sources[0], Some((3, 20)), "the first hit keeps slot one");
        assert_eq!(
            sources[1],
            Some((19, 12)),
            "the last hit wins slot two - not the second hit, (11, 12)"
        );
    }

    /// `formats/location-dat.md §6`: the walk covers "every cell of the
    /// freshly-read tile grid in loader order (column 0 north-to-south,
    /// then column 1, and so on)". Column-major order decides which hit is
    /// first and which is last, so the beacon harvest — which shares the
    /// NPC markers' single walk, "one walk, two purposes" — shares it.
    #[test]
    fn location_harvest_walks_in_column_major_loader_order() {
        // Row-major order would see (20, 1) first; column-major sees
        // (1, 20) first, because column one comes before column twenty.
        let grid = floor_with(&[
            ((20, 1), BEACON_BRIGHT_LIGHT_TILE),
            ((1, 20), BEACON_BRIGHT_LIGHT_TILE),
        ]);

        assert_eq!(
            harvest_location_beacon_sources(&grid),
            [Some((1, 20)), Some((20, 1))]
        );
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
    pub(super) fn shipped_asset_copy(files: &[&str]) -> Option<std::path::PathBuf> {
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
        Some(table)
    }

    fn shipped_data_ovl() -> Option<Vec<u8>> {
        let dir = shipped_asset_copy(&[DATA_OVL_FILENAME])?;
        let bytes = fs::read(dir.join(DATA_OVL_FILENAME)).unwrap();
        let _ = fs::remove_dir_all(&dir);
        Some(bytes)
    }

    /// `visibility.md §12.6`: "each bearing is a fixed set of at most
    /// sixteen cell offsets relative to the source, so a bearing is a
    /// stencil, not a computed sweep", and the beam reaches "up to seven
    /// tiles from the source".
    ///
    /// `formats/tiles.md §5.1.1` publishes the table's offset, so this
    /// reads it there and checks the published shape against it.
    #[test]
    fn shipped_stencils_have_at_most_sixteen_offsets_reaching_seven_tiles() {
        let Some(stencils) = shipped_stencils() else {
            return;
        };

        let mut longest_reach = 0;
        for bearing in 0..BEACON_BEARING_COUNT {
            let offsets = stencils.cells(bearing);
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

    /// `formats/tiles.md §5.1.1`: "cell counts follow the heading class
    /// exactly: the four **cardinals** (records 1, 5, 9, 13) light
    /// **fifteen** cells, the four **diagonals** (3, 7, 11, 15) light
    /// **eleven**, and the eight **halfway** bearings light **nine**."
    ///
    /// Checked twice over: that our class rule reproduces the published
    /// per-record numbers, and that the shipped table matches it.
    #[test]
    fn shipped_stencil_cell_counts_follow_the_published_heading_class() {
        let published = [
            9, 15, 9, 11, 9, 15, 9, 11, 9, 15, 9, 11, 9, 15, 9, 11usize,
        ];
        for (index, expected) in published.into_iter().enumerate() {
            assert_eq!(
                beacon_record_cell_count(index),
                expected,
                "record {index}"
            );
        }
        assert_eq!(beacon_record_cell_count(1), BEACON_CARDINAL_CELLS);
        assert_eq!(beacon_record_cell_count(3), BEACON_DIAGONAL_CELLS);
        assert_eq!(beacon_record_cell_count(0), BEACON_HALFWAY_CELLS);

        let Some(stencils) = shipped_stencils() else {
            return;
        };
        for (index, expected) in published.into_iter().enumerate() {
            assert_eq!(
                stencils.cells(index as u8).len(),
                expected,
                "shipped record {index}"
            );
        }
    }

    /// `formats/tiles.md §5.1.1`: "the stamp always runs **all sixteen
    /// iterations** of a record - there is no early exit on the `(0, 0)`
    /// padding, so a padded pair writes at the record's own origin cell".
    ///
    /// Every shipped record is padded (the longest lights fifteen of
    /// sixteen), so the padding path is taken on every bearing.
    #[test]
    fn every_shipped_record_carries_padding_the_stamp_still_walks() {
        let Some(stencils) = shipped_stencils() else {
            return;
        };
        for bearing in 0..BEACON_BEARING_COUNT {
            let record = stencils.bearing(bearing);
            let cells = stencils.cells(bearing);
            assert_eq!(
                record.len(),
                BEACON_STENCIL_MAX_OFFSETS,
                "the stamp walks all sixteen pairs of bearing {bearing}"
            );
            assert!(
                cells.len() < BEACON_STENCIL_MAX_OFFSETS,
                "bearing {bearing} has no padded pair, so this rule would be untestable"
            );
            assert!(
                record[cells.len()..].iter().all(|pair| *pair == (0, 0)),
                "bearing {bearing} pads with exactly (0, 0)"
            );
        }
    }

    /// `formats/tiles.md §5.1.1` publishes the offset **and** records that
    /// a structural search of the shipped overlay "yields **exactly one**
    /// candidate, and it is this table". This engine anchors to the offset;
    /// the search is kept only so that agreement stays asserted. If the two
    /// ever disagree, the published offset is wrong or the shipped image is
    /// not the one the spec was traced from — either way, loudly.
    #[test]
    fn the_structural_search_agrees_with_the_published_offset() {
        let Some(data) = shipped_data_ovl() else {
            return;
        };
        assert_eq!(
            scan_beacon_bearing_stencil_offsets(&data),
            vec![BEACON_STENCIL_TABLE_OFFSET],
            "exactly one structural candidate, at the published offset"
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
            for &(dx, dy) in stencils.cells(bearing) {
                assert!(predicate(dx, dy), "bearing {bearing} offset ({dx}, {dy})");
            }
        }

        // The four diagonals: 3 north-east, 7 south-east, 11 south-west,
        // 15 north-west.
        for (bearing, sx, sy) in [(3u8, 1i8, -1i8), (7, 1, 1), (11, -1, 1), (15, -1, -1)] {
            for &(dx, dy) in stencils.cells(bearing) {
                assert_eq!(dx.signum(), sx, "bearing {bearing} offset ({dx}, {dy})");
                assert_eq!(dy.signum(), sy, "bearing {bearing} offset ({dx}, {dy})");
            }
        }

        // Bearing sixteen is one step anticlockwise of due north, so it
        // leans west while still pointing north.
        for &(dx, dy) in stencils.cells(0) {
            assert!(dx < 0 && dy < 0, "bearing sixteen offset ({dx}, {dy})");
        }
    }

    /// `formats/tiles.md §5.1.1`: an implementation "should fail loudly on
    /// zero candidates rather than silently lighting nothing".
    ///
    /// This used to be the loader's one soft spot: an image with no table
    /// returned `Ok(None)` and the beacon lit nothing, indistinguishable in
    /// play from a beacon that was simply never in view. All three ways of
    /// having no table are errors now.
    #[test]
    fn the_stencil_loader_fails_loudly_when_the_published_offset_has_no_table() {
        let too_short = read_beacon_bearing_stencils(&[0; 8]).unwrap_err();
        assert_eq!(too_short.kind(), io::ErrorKind::InvalidData);

        let all_zero = vec![0u8; BEACON_STENCIL_TABLE_OFFSET + BEACON_STENCIL_TABLE_BYTES];
        let no_table = read_beacon_bearing_stencils(&all_zero).unwrap_err();
        assert_eq!(no_table.kind(), io::ErrorKind::InvalidData);

        let dir = debug_game_dir();
        fs::remove_file(dir.join(DATA_OVL_FILENAME)).unwrap();
        let missing = load_beacon_bearing_stencils(&dir).unwrap_err();
        assert_eq!(missing.kind(), io::ErrorKind::NotFound);
        let _ = fs::remove_dir_all(&dir);
    }

    /// The table is read at the published offset and nowhere else: the same
    /// bytes one byte earlier are not a table.
    #[test]
    fn the_stencil_loader_reads_only_the_published_offset() {
        let table = synthetic_beacon_stencil_table();
        let mut shifted = vec![0u8; BEACON_STENCIL_TABLE_OFFSET - 1];
        shifted.extend_from_slice(&table);
        shifted.resize(BEACON_STENCIL_TABLE_OFFSET + BEACON_STENCIL_TABLE_BYTES, 0);
        assert!(read_beacon_bearing_stencils(&shifted).is_err());

        let mut anchored = vec![0u8; BEACON_STENCIL_TABLE_OFFSET];
        anchored.extend_from_slice(&table);
        assert!(read_beacon_bearing_stencils(&anchored).is_ok());
    }

    /// A record whose live pairs are followed by padding is accepted; one
    /// with a live pair *after* padding, an offset past seven cells, an
    /// offset pointing away from its bearing, a repeated pair, or a cell
    /// count that does not match the record's heading class, is not.
    #[test]
    fn stencil_parser_enforces_the_published_record_shape() {
        let bytes = synthetic_beacon_stencil_table();
        assert!(parse_beacon_bearing_stencils(&bytes).is_some());

        // Record 1 is due north and lights fifteen cells, so its sixteenth
        // pair is its only padding.
        let record1 = BEACON_STENCIL_RECORD_BYTES;
        let last_pair = record1 + (BEACON_CARDINAL_CELLS - 1) * 2;

        let mut padded = bytes.clone();
        padded[last_pair] = 0;
        padded[last_pair + 1] = 0;
        padded[record1 + 30] = 0;
        padded[record1 + 31] = (-1i8) as u8;
        assert!(
            parse_beacon_bearing_stencils(&padded).is_none(),
            "a live pair after padding is rejected"
        );

        let mut too_far = bytes.clone();
        too_far[record1 + 1] = (-8i8) as u8;
        assert!(
            parse_beacon_bearing_stencils(&too_far).is_none(),
            "an offset past seven tiles is rejected"
        );

        let mut wrong_way = bytes.clone();
        wrong_way[record1 + 1] = 1;
        assert!(
            parse_beacon_bearing_stencils(&wrong_way).is_none(),
            "an offset pointing away from its bearing is rejected"
        );

        let mut repeated = bytes.clone();
        repeated[record1 + 2] = repeated[record1];
        repeated[record1 + 3] = repeated[record1 + 1];
        assert!(
            parse_beacon_bearing_stencils(&repeated).is_none(),
            "a repeated pair inside one record is rejected"
        );

        let mut short_count = bytes.clone();
        short_count[last_pair] = 0;
        short_count[last_pair + 1] = 0;
        assert!(
            parse_beacon_bearing_stencils(&short_count).is_none(),
            "a cardinal record lighting fourteen cells is rejected"
        );

        let empty = vec![0u8; BEACON_STENCIL_TABLE_BYTES];
        assert!(parse_beacon_bearing_stencils(&empty).is_none());
    }
}

mod light_beacon_mask_stamp {
    use super::*;

    fn stencils_for_test() -> BeaconBearingStencils {
        synthetic_beacon_bearing_stencils()
    }

    fn beacon_state_at(source: (u8, u8), bearing: u8, ambient: u8) -> PlayState {
        let mut state = test_state(open_grid(), 16, 16);
        state.beacon_bearing_stencils = stencils_for_test();
        state.light_beacon = LightBeaconState {
            sources: [Some(source), None],
            bearing,
        };
        state.ambient_light = ambient;
        state
    }

    /// Every cell the stamp is expected to write for one source at one
    /// bearing: the union of all sixteen pairs of each of the three cone
    /// bearings, padding included (`formats/tiles.md §5.1.1`).
    fn expected_cone_cells(source: (u8, u8), bearing: u8) -> Vec<(isize, isize)> {
        let stencils = stencils_for_test();
        let mut cells: Vec<(isize, isize)> = beacon_cone_bearings(bearing)
            .into_iter()
            .flat_map(|bearing| stencils.bearing(bearing).to_owned())
            .map(|(dx, dy)| {
                (
                    isize::from(source.0) + isize::from(dx),
                    isize::from(source.1) + isize::from(dy),
                )
            })
            .collect();
        cells.sort_unstable();
        cells.dedup();
        cells
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

        assert_eq!(lit, expected_cone_cells((10, 10), 1));
    }

    /// `formats/tiles.md §5.1.1`: "the stamp always runs **all sixteen
    /// iterations** of a record - there is no early exit on the `(0, 0)`
    /// padding, so a padded pair writes at the record's own origin cell,
    /// which is harmless because that cell is the source."
    ///
    /// This tree stopped at the live prefix. Every record is padded, so the
    /// source cell is written on every bearing, and skipping the padding is
    /// a real difference in what reaches the mask — one cell, invisible in
    /// play only because a lit source cell looks like a lit source cell.
    #[test]
    fn the_stamp_walks_the_padding_and_so_writes_the_source_cell() {
        let state = beacon_state_at((10, 10), 1, FULL_DARKNESS);
        assert!(
            lit_cells(&state).contains(&(10, 10)),
            "the padded (0, 0) pairs write at the source"
        );

        // Not an artefact of some record listing (0, 0) as a live cell.
        let stencils = stencils_for_test();
        for bearing in beacon_cone_bearings(1) {
            assert!(!stencils.cells(bearing).contains(&(0, 0)));
            assert!(stencils.bearing(bearing).contains(&(0, 0)));
        }
    }

    #[test]
    fn both_source_slots_stamp_and_overlapping_cones_union() {
        let mut state = beacon_state_at((10, 10), 1, FULL_DARKNESS);
        state.light_beacon.sources[1] = Some((14, 10));
        let mut lit = lit_cells(&state);
        lit.sort_unstable();

        let mut expected = expected_cone_cells((10, 10), 1);
        expected.extend(expected_cone_cells((14, 10), 1));
        expected.sort_unstable();
        let overlap = expected.len();
        expected.dedup();
        assert!(
            expected.len() < overlap,
            "the two cones must overlap for this to test the union"
        );
        assert_eq!(lit, expected);
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

        let mut state = world_state(grid, lighthouse_x, lighthouse_y);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.world_live_chunks = Some(buffer);
        state.beacon_bearing_stencils = stencils;
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

    /// `formats/location-dat.md §6`: "the harvest converts tile positions
    /// into resident coordinate *words* at load time, and the beacon never
    /// re-reads the map afterwards. A later pass that rewrites the cell
    /// therefore cannot switch the source off. **An implementation that
    /// instead harvests from a normalised or scrubbed copy of the floor
    /// loses this property**, and will find the source present on one entry
    /// path and absent on another."
    ///
    /// We had exactly that divergence, because the scrub claimed `0x2A`
    /// under the withdrawn spawn-marker reading. The scrub no longer
    /// touches the byte, so both orders agree on shipped data — but the
    /// raw-floor harvest is kept, because the published property is about
    /// *where the harvest reads from*, not about which passes happen to
    /// rewrite the cell today. This test pins the weaker fact that makes
    /// the ordering currently indifferent, so that a future pass claiming
    /// the byte fails here rather than silently darkening the beacon.
    #[test]
    fn normalisation_leaves_the_byte_the_beacon_harvests_alone() {
        let mut grid = vec![0u8; 32 * 32];
        grid[5 * 32 + 7] = BEACON_BRIGHT_LIGHT_TILE;

        let raw_sources = harvest_location_beacon_sources(&grid);
        assert_eq!(
            raw_sources[0],
            Some((7, 5)),
            "the raw floor must yield the bright-light cell"
        );

        normalize_town_runtime_floor(&mut grid, 12);
        assert_eq!(
            grid[5 * 32 + 7],
            BEACON_BRIGHT_LIGHT_TILE,
            "normalisation must not rewrite the beacon's light source"
        );
        assert_eq!(
            harvest_location_beacon_sources(&grid),
            raw_sources,
            "so the raw and normalised harvests agree"
        );
    }

    /// The floor-transition loader harvests from the raw page, whatever the
    /// normalisation pass does to it afterwards.
    #[test]
    fn the_floor_transition_loader_harvests_from_the_raw_page() {
        let dir = debug_game_dir();
        let scene = Scene::new(17).unwrap();
        let mut pages = vec![16u8; 16 * 1024];
        pages[7 * 32 + 5] = BEACON_BRIGHT_LIGHT_TILE;
        pages[8 * 32 + 6] = 0x48;
        fs::write(dir.join("CASTLE.DAT"), pages).unwrap();

        let (grid, sources) =
            load_town_runtime_floor_with_beacon_sources(&dir, scene, 0, 12).unwrap();
        let _ = fs::remove_dir_all(&dir);

        assert_eq!(sources, [Some((5, 7)), None]);
        assert_eq!(grid[8 * 32 + 6], LOCATION_MARKER_CLEANUP_TILE);
    }
}

mod light_beacon_shipped_location_sources {
    use super::*;

    /// `formats/location-dat.md §6`, the argument that withdraws the spawn
    /// marker without appealing to any code: "**`0x2A` appears in zero
    /// town, castle and keep floors.** It occurs **five times across four
    /// floors, every one of them dwelling-class** ... A player town-entry
    /// spawn marker that exists in no town is not a spawn marker."
    ///
    /// And the fact that makes the second slot worth implementing:
    /// "**Exactly one shipped floor carries two.** Three of the four carry
    /// one. So the second slot is exercised in exactly one place in the
    /// whole shipped data set, and an implementation that silently handles
    /// one source per floor is correct on three floors and wrong on the
    /// fourth."
    ///
    /// Reproduced here against the shipped class files rather than taken on
    /// trust.
    #[test]
    fn shipped_location_files_carry_the_published_beacon_source_layout() {
        const PAGE_BYTES: usize = 32 * 32;
        let classes = ["TOWNE.DAT", "DWELLING.DAT", "CASTLE.DAT", "KEEP.DAT"];
        // Never read the pristine install in place; work from a scratch
        // copy, as every other shipped-asset test here does.
        let Some(source) = super::light_beacon_stencils::shipped_asset_copy(&classes) else {
            return;
        };

        let mut cells_by_class = Vec::new();
        let mut floors_with_one = 0;
        let mut floors_with_two = 0;
        let mut floors_with_more = 0;
        for class in classes {
            let bytes = fs::read(source.join(class)).unwrap();
            let mut class_cells = 0;
            for page in bytes.chunks_exact(PAGE_BYTES) {
                let hits = page
                    .iter()
                    .filter(|tile| **tile == BEACON_BRIGHT_LIGHT_TILE)
                    .count();
                class_cells += hits;
                match hits {
                    0 => {}
                    1 => floors_with_one += 1,
                    2 => floors_with_two += 1,
                    _ => floors_with_more += 1,
                }
                // Whatever the count, the harvest fills at most two slots.
                let filled = harvest_location_beacon_sources(page)
                    .iter()
                    .flatten()
                    .count();
                assert_eq!(filled, hits.min(BEACON_SOURCE_SLOTS));
            }
            cells_by_class.push((class, class_cells));
        }
        let _ = fs::remove_dir_all(&source);

        assert_eq!(
            cells_by_class,
            vec![
                ("TOWNE.DAT", 0),
                ("DWELLING.DAT", 5),
                ("CASTLE.DAT", 0),
                ("KEEP.DAT", 0),
            ],
            "zero town, castle and keep floors; five cells, all dwelling-class"
        );
        assert_eq!(floors_with_one, 3, "three floors carry one source");
        assert_eq!(floors_with_two, 1, "exactly one floor carries two");
        assert_eq!(floors_with_more, 0, "no shipped floor carries three");
    }

    /// The end-to-end shipped-data case, through the same loader a floor
    /// transition uses.
    ///
    /// `formats/location-dat.md §4.1` gives the four lighthouses page runs
    /// `0-2`, `3-5`, `6-8` and `9-11`, entering on the lowest page of each,
    /// so the four pages carrying `0x2A` are exactly their floor `+2` — the
    /// lantern rooms. `§6` notes "a lantern room is reached by climbing,
    /// [so] the stairs path is the only one reachable in play", which is
    /// the path this exercises.
    ///
    /// Nothing in the route suite reaches these floors: every
    /// `stock-location-enter` case stops on floor 0 and none climbs. This
    /// test is the only end-to-end coverage of the indoor beacon over
    /// shipped data.
    #[test]
    fn every_shipped_lighthouse_lantern_room_lights_its_beacon() {
        let Some(dir) = super::light_beacon_stencils::shipped_asset_copy(&["DWELLING.DAT"]) else {
            return;
        };

        let expected = [
            (SCENE_FOGSBANE, [Some((8, 16)), None]),
            (SCENE_STORMCROW, [Some((16, 20)), None]),
            (SCENE_GREYHAVEN, [Some((15, 15)), None]),
            // The one shipped floor with two sources.
            (SCENE_WAVEGUIDE, [Some((11, 12)), Some((19, 12))]),
        ];
        for (scene, sources) in expected {
            let scene = Scene::new(scene).unwrap();
            let (grid, harvested) =
                load_town_runtime_floor_with_beacon_sources(&dir, scene, 2, 12).unwrap();
            assert_eq!(harvested, sources, "{} lantern room", scene.key());
            for source in harvested.iter().flatten() {
                assert_eq!(
                    grid[usize::from(source.1) * TOWN_GRID_SIDE + usize::from(source.0)],
                    BEACON_BRIGHT_LIGHT_TILE,
                    "{}: the runtime buffer still carries the source tile",
                    scene.key()
                );
            }
        }

        let _ = fs::remove_dir_all(&dir);
    }
}

    #[test]
    fn stair_delta_uses_request_direction_for_public_stair_family() {
        assert_eq!(stair_delta(80, ClimbIntent::Up), Some(1));
        assert_eq!(stair_delta(80, ClimbIntent::Down), Some(-1));
        assert_eq!(stair_delta(81, ClimbIntent::Down), Some(-1));
        assert_eq!(stair_delta(81, ClimbIntent::Up), Some(1));
        assert_eq!(stair_delta(16, ClimbIntent::Up), None);
    }

    /// Regression: without a save and without `--at`, the world fallback
    /// used to pick the first walkable cell in linear scan order, which
    /// landed on a single-tile island where movement was impossible.
    /// The current rule requires >=5 walkable cells in the 3x3 neighbourhood
    /// so the player can actually explore.
    #[test]
    fn first_world_walkable_skips_isolated_walkable_cells() {
        // Build a world that's all water (sentinel) except a single grass
        // tile near the origin and a 3x3 island of grass further on.
        const GRASS: u8 = 5;
        const WATER: u8 = 1;
        let mut grid = vec![WATER; WORLD_CELLS];

        // Lone island at (10, 0): walkable but no walkable neighbours.
        grid[world_cell_index(10, 0)] = GRASS;

        // 3x3 island centred on (20, 20).
        for dy in 0..3 {
            for dx in 0..3 {
                grid[world_cell_index(20 + dx, 20 + dy)] = GRASS;
            }
        }

        let picked = first_world_walkable_for_transport(
            &grid,
            WorldPlane::Britannia,
            None,
            TransportState::Foot,
            &[],
        )
        .expect("should find the 3x3 island");

        // The 3x3 island's centre (21, 21) has all 8 walkable neighbours;
        // the corners have 3. The earliest-in-scan cell that satisfies the
        // >=5-neighbours rule is the top edge (20, 20) or (21, 20).
        let (x, y) = picked;
        assert!(
            (20..=22).contains(&x) && (20..=22).contains(&y),
            "expected to land on the 3x3 island, got ({x}, {y})"
        );
        assert_ne!(picked, (10, 0), "must not pick the isolated 1x1 island");
    }

    /// LOOK2.DAT cross-check: tile ids 0x0a..=0x0f are six DISTINCT terrain
    /// types -- not all "mountains". Per the canonical labels:
    ///   0x0a tropical forest  (dense forest, blocks sight, IMPASSABLE)
    ///   0x0b foothills        (rolling hills, doesn't block, WALKABLE)
    ///   0x0c mountains        (true mountain, blocks sight, IMPASSABLE)
    ///   0x0d high peaks       (true mountain, blocks sight, IMPASSABLE)
    ///   0x0e foothills        (rolling hills, doesn't block, WALKABLE)
    ///   0x0f foothills        (rolling hills, doesn't block, WALKABLE)
    /// Per u5-spec/catalogs/tile-catalog.md Section 5: mountains are
    /// impassable for everything except the balloon. Foothills are not
    /// mountains -- they are hills, walkable on foot.
    #[test]
    fn foothills_are_walkable_per_look2_dat() {
        assert!(
            is_probe_walkable(0x0b),
            "0x0b 'foothills' must be walkable on foot"
        );
        assert!(
            is_probe_walkable(0x0e),
            "0x0e 'foothills' must be walkable on foot"
        );
        assert!(
            is_probe_walkable(0x0f),
            "0x0f 'foothills' must be walkable on foot"
        );
    }

    #[test]
    fn true_mountains_and_dense_forest_are_impassable_per_spec() {
        assert!(
            !is_probe_walkable(0x0a),
            "0x0a 'tropical forest' (dense) must be impassable on foot"
        );
        assert!(
            !is_probe_walkable(0x0c),
            "0x0c 'mountains' must be impassable on foot"
        );
        assert!(
            !is_probe_walkable(0x0d),
            "0x0d 'high peaks' must be impassable on foot"
        );
    }

    /// Per u5-spec/systems/visibility.md Section 6: forest interior /
    /// mountains block sight; open ground including paths, grass, water,
    /// and HILLS does not. The "see over the mountain from a hill"
    /// mechanic does not exist -- but hills themselves are transparent.
    #[test]
    fn foothills_do_not_block_sight_on_overworld() {
        assert!(
            !world_surface_tile_blocks_sight(0x0b),
            "0x0b foothills must be see-through on the overworld"
        );
        assert!(
            !world_surface_tile_blocks_sight(0x0e),
            "0x0e foothills must be see-through on the overworld"
        );
        assert!(
            !world_surface_tile_blocks_sight(0x0f),
            "0x0f foothills must be see-through on the overworld"
        );
    }

    #[test]
    fn mountains_peaks_and_dense_forest_block_sight_on_overworld() {
        assert!(
            world_surface_tile_blocks_sight(0x0a),
            "0x0a 'tropical forest' (dense) must block sight"
        );
        assert!(
            world_surface_tile_blocks_sight(0x0c),
            "0x0c 'mountains' must block sight"
        );
        assert!(
            world_surface_tile_blocks_sight(0x0d),
            "0x0d 'high peaks' must block sight"
        );
    }

    /// Per u5-spec/catalogs/tile-catalog.md Section 4: water is a 4-frame
    /// cycle; LOOK2.DAT shows 0x04 is "swamp" (distinct terrain) so the
    /// actual water cycle in this game is 3 frames (0x01..=0x03). Each
    /// cell preserves its identity offset within the cycle.
    #[test]
    fn water_animation_cycles_three_frames_preserving_per_cell_identity() {
        let mut clock = AnimationClock::default();
        // Cell stored as "deep water" (0x01) and cell stored as "shoals"
        // (0x03) are out of phase by 2 in the 3-frame cycle.
        for tick in 0..6 {
            let deep = clock.resolve_static_tile(0x01);
            let shoals = clock.resolve_static_tile(0x03);
            assert!(
                (1..=3).contains(&deep),
                "deep water cell at tick {tick} resolved to {deep:#x}, outside the cycle"
            );
            assert!(
                (1..=3).contains(&shoals),
                "shoals cell at tick {tick} resolved to {shoals:#x}, outside the cycle"
            );
            // Identity offset preserved: shoals - deep is 2 mod 3.
            let phase_diff = (3 + shoals - deep) % 3;
            assert_eq!(
                phase_diff, 2,
                "stored-id offset must be preserved across frames"
            );
            clock.tick_static_tiles();
        }
    }

    /// Per u5-spec the water animator runs as part of the per-turn
    /// epilogue. After enough ticks the displayed tile of any single
    /// water cell must cycle through every frame in its family.
    #[test]
    fn water_cells_visit_every_frame_across_ticks() {
        let mut clock = AnimationClock::default();
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..12 {
            seen.insert(clock.resolve_static_tile(0x02));
            clock.tick_static_tiles();
        }
        assert!(
            seen.contains(&0x01) && seen.contains(&0x02) && seen.contains(&0x03),
            "water cell stored as 0x02 must visit 0x01, 0x02, and 0x03 across the cycle, got {seen:?}"
        );
    }

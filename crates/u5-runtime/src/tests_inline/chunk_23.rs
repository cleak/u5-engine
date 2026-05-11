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
    fn true_mountains_are_impassable_per_spec() {
        // Tropical forest 0x0a is walkable but blocks sight (see
        // dense_forest_is_walkable_but_blocks_sight). Only mountains
        // and high peaks block on-foot movement.
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

    /// Per u5-spec/systems/animation.md Section 6 the water animator
    /// uses a shared frame selector: every water cell shows the same
    /// frame at the same tick, cycling through the family.
    #[test]
    fn water_animation_cycles_three_frames_shared_selector() {
        let mut clock = AnimationClock::default();
        for tick in 0..9 {
            let resolved = clock.resolve_static_tile(0x01);
            let expected = 0x01 + (tick % 3);
            assert_eq!(
                resolved, expected,
                "tick {tick}: water-family base 0x01 must show frame 0x{expected:02x}"
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

    /// Per u5-spec/systems/animation.md Section 6: "A map cell continues
    /// to mean 'water'; the renderer resolves that semantic tile through
    /// the current water-frame selector at draw time. This keeps the map
    /// stable and makes one frame-counter update affect every visible
    /// cell in the same family."
    /// I.e. the animation is a SHARED FRAME SELECTOR -- at any given
    /// tick, every water-family cell displays the same frame, regardless
    /// of what its stored id is.
    #[test]
    fn water_animation_is_shared_frame_selector() {
        for frame in 0..6u8 {
            let clock = AnimationClock {
                frame,
                moongate_frame: 0,
            };
            let a = clock.resolve_static_tile(0x01);
            let b = clock.resolve_static_tile(0x02);
            let c = clock.resolve_static_tile(0x03);
            assert_eq!(
                a, b,
                "water cells 0x01 and 0x02 must show the same frame at tick {frame}"
            );
            assert_eq!(
                b, c,
                "water cells 0x02 and 0x03 must show the same frame at tick {frame}"
            );
        }
    }

    /// Per actual Ultima V gameplay: swamp tiles are walkable on foot
    /// (you take poison damage stepping through). 0x04 is "swamp" per
    /// LOOK2.DAT. The visual sprite at 0x04 (green dots over blue) is a
    /// distinct terrain type from water; it must NOT participate in the
    /// water animation cycle and must NOT block on-foot movement.
    #[test]
    fn swamp_is_walkable_and_static() {
        assert!(
            is_probe_walkable(0x04),
            "0x04 'swamp' must be walkable on foot"
        );
        for frame in 0..6u8 {
            let clock = AnimationClock {
                frame,
                moongate_frame: 0,
            };
            assert_eq!(
                clock.resolve_static_tile(0x04),
                0x04,
                "swamp must stay 0x04 across all animation frames"
            );
        }
    }

    /// Tropical forest (0x0a) is dense forest interior. Per the
    /// visibility spec Section 6 it blocks sight; per actual U5
    /// gameplay the player CAN walk into a forest -- dense forest just
    /// limits visibility to one cell out before the interior wraps.
    #[test]
    fn dense_forest_is_walkable_but_blocks_sight() {
        assert!(
            is_probe_walkable(0x0a),
            "0x0a 'tropical forest' must be walkable on foot"
        );
        assert!(
            world_surface_tile_blocks_sight(0x0a),
            "0x0a 'tropical forest' must block line of sight"
        );
    }

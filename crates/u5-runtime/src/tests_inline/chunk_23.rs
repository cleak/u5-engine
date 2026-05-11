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

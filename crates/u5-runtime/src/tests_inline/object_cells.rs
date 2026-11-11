// Phase B M2-05: the object-cell peek (design 9.3.5, 9.3.7, 9.3.9;
// D-M2.1).
mod object_cells {
    use super::*;
    use crate::enhancements::sprite_mask::{KEEP_THE_BOX, SpriteMasks};

    const PERSON: u8 = 0x5A;
    const SITTER: u8 = 0x44;
    const CHAIR_FACING_DOWN: u8 = 0x92;
    const SUPPRESSING_TERRAIN: u8 = 0x0A;
    const TOWN_WALL: u8 = 0x4E;
    const GRASS: u8 = 0x05;
    const FRIGATE: u8 = 168;
    const FIELD: u8 = 0xE8;

    /// The synthetic atlas, with every actor-half tile drawn as an 8x8
    /// coloured square on a black box, so each has a background to
    /// composite. Terrain tiles stay solid (`id % 16`).
    fn boxed_atlas() -> TileAtlas {
        let mut atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        for tile in ACTOR_TILE_BANK_BASE..TILE_ATLAS_TILE_COUNT {
            let colour = (tile % 15) as u8 + 1;
            let base = tile * TILE_ATLAS_TILE_PIXELS;
            for y in 0..TILE_ATLAS_SIDE {
                for x in 0..TILE_ATLAS_SIDE {
                    let inside = (4..12).contains(&x) && (4..12).contains(&y);
                    atlas.pixels[base + y * TILE_ATLAS_SIDE + x] = if inside { colour } else { 0 };
                }
            }
        }
        atlas
    }

    fn object(tile: u8, x: usize, y: usize, z: i8) -> ActiveObject {
        ActiveObject {
            type_byte: tile,
            tile,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }
    }

    fn with_objects(mut state: PlayState, objects: &[ActiveObject]) -> PlayState {
        state.active_objects.truncate(1);
        state.active_objects.extend_from_slice(objects);
        state
    }

    fn town_at_noon(grid: Vec<u8>) -> PlayState {
        let mut state = test_state(grid, 16, 16);
        state.ambient_light = FULL_DAYLIGHT;
        state.rebuild_surface_local_light_mask();
        state
    }

    fn world_at_noon(grid: Vec<u8>) -> PlayState {
        let mut state = world_state(grid, 100, 100);
        state.ambient_light = FULL_DAYLIGHT;
        state
    }

    /// Render the faithful viewport as the shell does, then peek. Every
    /// peeked cell is checked against the frame (`drawn` is what the
    /// render drew there).
    fn render_and_peek(state: &mut PlayState, atlas: &TileAtlas) -> Option<ObjectCellPeek> {
        let masks = SpriteMasks::from_atlas(atlas);
        let viewport = state
            .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, atlas)
            .unwrap()
            .unwrap();
        let peek = state.peek_object_cells(atlas, &masks);
        if let Some(peek) = &peek {
            assert_drawn_matches_frame(state, atlas, &viewport, peek);
        }
        peek
    }

    fn tile_cell(atlas: &TileAtlas, tile: usize) -> Vec<u8> {
        let mut scratch = TileViewport {
            depth: atlas.depth,
            cells_wide: 1,
            cells_high: 1,
            width: TILE_ATLAS_SIDE,
            height: TILE_ATLAS_SIDE,
            pixels: vec![0; TILE_ATLAS_TILE_PIXELS],
        };
        blit_terrain_tile_to_viewport(
            &mut scratch,
            atlas,
            tile,
            0,
            0,
            WaterScrollClock::default(),
            &FireFlickerClock::default(),
        )
        .unwrap();
        scratch.pixels
    }

    fn assert_drawn_matches_frame(
        state: &PlayState,
        atlas: &TileAtlas,
        viewport: &TileViewport,
        peek: &ObjectCellPeek,
    ) {
        let cursor = state.combat_overlay_draw_cells().cursor_draw_cell;
        for cell in &peek.cells {
            let expected = tile_cell(atlas, usize::from(cell.drawn));
            let on_cursor = cursor == Some((cell.col, cell.row));
            for y in 0..TILE_ATLAS_SIDE {
                for x in 0..TILE_ATLAS_SIDE {
                    let ring = [0, 1, 14, 15];
                    if on_cursor && (ring.contains(&x) || ring.contains(&y)) {
                        continue;
                    }
                    let at = (usize::from(cell.row) * TILE_ATLAS_SIDE + y) * viewport.width
                        + usize::from(cell.col) * TILE_ATLAS_SIDE
                        + x;
                    assert_eq!(
                        viewport.pixels[at],
                        expected[y * TILE_ATLAS_SIDE + x],
                        "cell {cell:?} pixel ({x}, {y})"
                    );
                }
            }
        }
    }

    fn cell_at(peek: &ObjectCellPeek, col: u8, row: u8) -> Option<&ObjectCell> {
        peek.cells
            .iter()
            .find(|cell| (cell.col, cell.row) == (col, row))
    }

    /// The viewport cell of the world/town coordinate `(x, y)`.
    fn view_cell(state: &PlayState, x: usize, y: usize) -> (u8, u8) {
        let reach = VIEWPORT_PLAYER_ROW as isize;
        (
            (x as isize - state.player.x as isize + reach) as u8,
            (y as isize - state.player.y as isize + reach) as u8,
        )
    }

    fn grid_byte(state: &PlayState, col: u8, row: u8) -> u8 {
        state.visibility_grid
            [visibility_grid_active_index(usize::from(row), usize::from(col)).unwrap()]
    }

    fn band_byte(state: &PlayState, col: u8, row: u8) -> u8 {
        state.terrain_band[terrain_band_active_index(usize::from(row), usize::from(col)).unwrap()]
    }

    fn solid(colour: u8) -> [u8; TILE_ATLAS_TILE_PIXELS] {
        [colour; TILE_ATLAS_TILE_PIXELS]
    }

    #[test]
    fn an_object_on_grass_peeks_its_actor_tile_over_the_grass() {
        let atlas = boxed_atlas();
        let state = world_at_noon(open_world_grid());
        let z = state.current_floor().unwrap();
        let mut state = with_objects(state, &[object(PERSON, 102, 99, z)]);
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let (col, row) = view_cell(&state, 102, 99);
        let cell = cell_at(&peek, col, row).expect("the person's cell");
        assert_eq!(cell.drawn, u16::from(PERSON) + 256);
        assert_eq!(cell.under, GRASS);
        assert_eq!(cell.under_pixels, solid(GRASS));
        assert!(!peek.animated_under);
        // The party's own cell is an object cell too; nothing else is.
        let party = cell_at(&peek, 5, 5).expect("the party's cell");
        assert_eq!(party.under, GRASS);
        assert_eq!(peek.cells.len(), 2);
    }

    #[test]
    fn a_town_object_peeks_the_floor_and_the_cells_are_row_major() {
        let atlas = boxed_atlas();
        let mut grid = open_grid();
        grid[14 * 32 + 18] = 0x44;
        let mut state = with_objects(
            town_at_noon(grid),
            &[object(PERSON, 18, 14, 0), object(PERSON, 13, 19, 0)],
        );
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let positions: Vec<(u8, u8)> = peek.cells.iter().map(|c| (c.row, c.col)).collect();
        let mut sorted = positions.clone();
        sorted.sort();
        assert_eq!(positions, sorted);
        let (col, row) = view_cell(&state, 18, 14);
        let cell = cell_at(&peek, col, row).unwrap();
        assert_eq!(cell.under, state.animation.resolve_static_tile(0x44));
        let (col, row) = view_cell(&state, 13, 19);
        assert_eq!(cell_at(&peek, col, row).unwrap().under, 0x10);
    }

    #[test]
    fn an_object_on_a_chair_keeps_its_box() {
        let atlas = boxed_atlas();
        let mut grid = open_grid();
        grid[15 * 32 + 18] = CHAIR_FACING_DOWN;
        let mut state = with_objects(town_at_noon(grid), &[object(SITTER, 18, 15, 0)]);
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let (col, row) = view_cell(&state, 18, 15);
        // The render stamped the occupied-chair tile there...
        assert_eq!(grid_byte(&state, col, row), VISIBILITY_USE_COMPANION);
        // ...and the furniture rule leaves the cell out.
        assert!(cell_at(&peek, col, row).is_none());
    }

    #[test]
    fn a_suppressed_stamp_is_no_object_cell() {
        let atlas = boxed_atlas();
        let mut grid = open_grid();
        grid[15 * 32 + 18] = SUPPRESSING_TERRAIN;
        let mut state = with_objects(town_at_noon(grid), &[object(PERSON, 18, 15, 0)]);
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let (col, row) = view_cell(&state, 18, 15);
        assert_ne!(grid_byte(&state, col, row), VISIBILITY_USE_COMPANION);
        assert!(cell_at(&peek, col, row).is_none());
    }

    #[test]
    fn an_object_on_a_hidden_cell_is_never_peeked() {
        let atlas = boxed_atlas();
        let mut grid = open_grid();
        for y in 0..32 {
            grid[y * 32 + 18] = TOWN_WALL;
        }
        let mut state = with_objects(town_at_noon(grid), &[object(PERSON, 20, 16, 0)]);
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let (col, row) = view_cell(&state, 20, 16);
        assert_eq!(grid_byte(&state, col, row), VISIBILITY_HIDDEN);
        assert!(cell_at(&peek, col, row).is_none());
        // Only the party remains.
        assert_eq!(peek.cells.len(), 1);
    }

    #[test]
    fn an_object_on_a_hidden_door_peeks_the_wall_tile() {
        let atlas = boxed_atlas();
        let mut grid = open_grid();
        grid[15 * 32 + 17] = TOWN_HIDDEN_DOOR_TILE;
        let mut state = with_objects(town_at_noon(grid), &[object(PERSON, 17, 15, 0)]);
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let (col, row) = view_cell(&state, 17, 15);
        let cell = cell_at(&peek, col, row).expect("the person on the hidden door");
        // The cell shows what the render paints there when it is empty:
        // the hidden door's own wall-looking tile, not an opened door.
        assert_eq!(cell.under, TOWN_HIDDEN_DOOR_TILE);
        assert_eq!(cell.under_pixels, solid(TOWN_HIDDEN_DOOR_TILE % 16));
    }

    #[test]
    fn the_draw_nothing_byte_is_skipped() {
        let atlas = boxed_atlas();
        let mut state = with_objects(
            town_at_noon(open_grid()),
            &[object(ACTOR_TILE_TRANSPARENT_BYTE, 18, 15, 0)],
        );
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let (col, row) = view_cell(&state, 18, 15);
        assert_eq!(grid_byte(&state, col, row), VISIBILITY_USE_COMPANION);
        assert_eq!(band_byte(&state, col, row), ACTOR_TILE_TRANSPARENT_BYTE);
        assert!(cell_at(&peek, col, row).is_none());
    }

    #[test]
    fn the_party_on_a_ship_peeks_the_animated_water() {
        let atlas = boxed_atlas();
        let water = (0..=u8::MAX)
            .find(|&tile| water_pass_rotates_tile(tile))
            .expect("a rotated water id");
        let mut state = world_at_noon(vec![water; WORLD_CELLS]);
        state.player.transport = TransportState::Ship {
            type_byte: FRIGATE,
            tile: FRIGATE,
            sails_hoisted: false,
            hull: 100,
            skiffs: 1,
        };
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let cell = cell_at(&peek, 5, 5).expect("the ship's cell");
        // The party's object carries the transport's save marker.
        assert_eq!(
            cell.drawn,
            u16::from(state.player.transport.save_marker()) + 256
        );
        assert_eq!(cell.under, state.animation.resolve_static_tile(water));
        assert!(peek.animated_under);
        // The pixels are the water as the render draws it at this scroll.
        let mut scratch = TileViewport {
            depth: atlas.depth,
            cells_wide: 1,
            cells_high: 1,
            width: TILE_ATLAS_SIDE,
            height: TILE_ATLAS_SIDE,
            pixels: vec![0; TILE_ATLAS_TILE_PIXELS],
        };
        blit_terrain_tile_to_viewport(
            &mut scratch,
            &atlas,
            usize::from(cell.under),
            0,
            0,
            state.water_scroll,
            &state.fire_flicker,
        )
        .unwrap();
        assert_eq!(cell.under_pixels.to_vec(), scratch.pixels);
    }

    #[test]
    fn a_live_moon_gate_under_an_object_draws_its_phase_frame() {
        let atlas = boxed_atlas();
        let mut grid = open_world_grid();
        grid[world_cell_index(102, 99)] = NATURAL_MOONGATE_LIVE_TILE;
        let state = world_at_noon(grid);
        let z = state.current_floor().unwrap();
        let mut state = with_objects(state, &[object(PERSON, 102, 99, z)]);
        state.natural_moongate_counter = 8;
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let (col, row) = view_cell(&state, 102, 99);
        let cell = cell_at(&peek, col, row).expect("the person on the gate");
        assert_eq!(cell.under, NATURAL_MOONGATE_LIVE_TILE);
        assert!(peek.animated_under);
        let ground = state.natural_moongate_phase_ground_tile() % 16;
        let gate = moongate_phase_gate_tile() % 16;
        // Half risen: the top rows are ground, the bottom rows the gate.
        assert_eq!(cell.under_pixels[0], ground);
        assert_eq!(cell.under_pixels[TILE_ATLAS_TILE_PIXELS - 1], gate);
    }

    #[test]
    fn combat_peeks_sprites_over_the_arena_terrain_and_keeps_fields_boxed() {
        let atlas = boxed_atlas();
        let mut state = combat_player_command_state(6, 5);
        state.combat_terrain = [[GRASS; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[5][6] = 0x44;
        state.active_objects[9] = object(FIELD, 2, 2, 0);
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let rat = cell_at(&peek, 6, 5).expect("the monster's cell");
        assert_eq!(rat.drawn, 0x190);
        assert_eq!(rat.under, state.animation.resolve_static_tile(0x44));
        let party = cell_at(&peek, 5, 5).expect("the party's cell");
        assert_eq!(party.under, GRASS);
        assert_eq!(party.under_pixels, solid(GRASS));
        // A field's pixels are noise: it keeps its box.
        assert_eq!(state.combat_render_sprite_at(2, 2), Some(0x1E8));
        assert!(cell_at(&peek, 2, 2).is_none());
        assert!(!peek.animated_under);
    }

    #[test]
    fn a_dungeon_room_arena_swaps_the_planes_under_the_sprite_only() {
        let atlas = boxed_atlas();
        let mut state = combat_player_command_state(6, 5);
        state.combat_terrain = [[GRASS; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        let plain = render_and_peek(&mut state.clone(), &atlas).unwrap();
        assert_eq!(cell_at(&plain, 6, 5).unwrap().under_pixels, solid(GRASS));

        state.combat_frame_snapshot = Some(CombatFrameSnapshot {
            area: Area::Dungeon {
                scene: DungeonScene::new(40).unwrap(),
                level: 0,
            },
            player: state.player,
            active_objects: state.active_objects.clone(),
            active_player: state.active_player,
            combat_terrain: state.combat_terrain,
            dungeon_room_clear_on_success: None,
            enter_endgame_after_successful_combat: false,
            endgame_messages: None,
            endgame_tableau_map: None,
            encounter_mode_high_bit: false,
            suppress_controlled_faint_sleep_tick: false,
            exit_announced: false,
            established_exit_direction_code: None,
            dungeon_post_combat: None,
            any_foe_seated: true,
        });
        assert!(state.dungeon_room_arena_plane_swap_active());
        let swapped = render_and_peek(&mut state, &atlas).unwrap();
        let rat = cell_at(&swapped, 6, 5).unwrap();
        assert_eq!(rat.under, GRASS);
        assert_eq!(rat.under_pixels, solid(swap_red_green_plane_index(GRASS)));
        assert_ne!(swap_red_green_plane_index(GRASS), GRASS);
        // The sprite itself is drawn unswapped (the frame check in
        // `render_and_peek` compared it with the atlas tile).
        assert_eq!(rat.drawn, 0x190);
    }

    #[test]
    fn the_aim_marker_cell_is_left_out() {
        let atlas = boxed_atlas();
        let mut state = combat_player_command_state(6, 5);
        state.combat_cursor_blink = true;
        state.active_player = Some(0);
        state.combat_aim_marker_gate = true;
        state.combat_aim_marker_cell = Some((6, 5));
        let report = state.combat_overlay_draw_cells();
        assert!(report.cursor_draw_cell.is_some(), "{report:?}");
        assert_eq!(report.secondary_marker_cell, Some((6, 5)));
        let masks = SpriteMasks::from_atlas(&atlas);
        let peek = state.peek_object_cells(&atlas, &masks).unwrap();
        assert!(cell_at(&peek, 6, 5).is_none());
        // The cursor's cell is white strokes over the sprite: it stays.
        let (cx, cy) = report.cursor_draw_cell.unwrap();
        assert!(cell_at(&peek, cx, cy).is_some());

        // With the marker gate down every sprite is peeked.
        state.combat_aim_marker_gate = false;
        let peek = state.peek_object_cells(&atlas, &masks).unwrap();
        assert!(cell_at(&peek, 6, 5).is_some());
    }

    #[test]
    fn keep_the_box_tiles_and_near_empty_sprites_are_left_out() {
        let mut atlas = boxed_atlas();
        // A near-empty sprite: four pixels on black.
        let near_empty = 0x5Bu8;
        let base = (usize::from(near_empty) + 256) * TILE_ATLAS_TILE_PIXELS;
        atlas.pixels[base..base + TILE_ATLAS_TILE_PIXELS].fill(0);
        for at in [7 * 16 + 7, 7 * 16 + 8, 8 * 16 + 7, 8 * 16 + 8] {
            atlas.pixels[base + at] = 9;
        }
        let shark = (KEEP_THE_BOX[0] - 256) as u8;
        let state = world_at_noon(open_world_grid());
        let z = state.current_floor().unwrap();
        let mut state = with_objects(
            state,
            &[
                object(near_empty, 102, 99, z),
                object(shark, 98, 99, z),
                object(PERSON, 100, 102, z),
            ],
        );
        let peek = render_and_peek(&mut state, &atlas).unwrap();
        let near = view_cell(&state, 102, 99);
        let shark_cell = view_cell(&state, 98, 99);
        let person = view_cell(&state, 100, 102);
        assert!(cell_at(&peek, near.0, near.1).is_none());
        assert!(cell_at(&peek, shark_cell.0, shark_cell.1).is_none());
        assert!(cell_at(&peek, person.0, person.1).is_some());
    }

    #[test]
    fn no_peek_without_a_top_down_or_combat_map() {
        let atlas = boxed_atlas();
        let masks = SpriteMasks::from_atlas(&atlas);
        let dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
        assert_eq!(dungeon.map_view_kind(), Some(MapViewKind::Dungeon));
        assert!(dungeon.peek_object_cells(&atlas, &masks).is_none());

        let mut gem = test_state(open_grid(), 5, 5);
        gem.gems = 1;
        assert_eq!(gem.view_gem(), MoveOutcome::Observed);
        assert!(gem.peek_object_cells(&atlas, &masks).is_none());

        let mut endgame = world_state(open_world_grid(), 10, 20);
        endgame.endgame = Some(EndgameState::awaiting_first_confirmation());
        assert!(endgame.peek_object_cells(&atlas, &masks).is_none());

        // Before the buffers were ever filled there is nothing to read.
        let fresh = town_at_noon(open_grid());
        assert!(!fresh.visibility_buffers_ready);
        assert_eq!(
            fresh.peek_object_cells(&atlas, &masks),
            Some(ObjectCellPeek::default())
        );
    }

    /// Design 9.6: clone a state, peek 100 times, and compare the whole
    /// state, including `prng_state`.
    #[test]
    fn the_peek_is_pure() {
        let atlas = boxed_atlas();
        let masks = SpriteMasks::from_atlas(&atlas);
        let mut grid = open_grid();
        grid[15 * 32 + 18] = CHAIR_FACING_DOWN;
        let mut town = with_objects(
            town_at_noon(grid),
            &[
                object(SITTER, 18, 15, 0),
                object(PERSON, 13, 19, 0),
                object(PERSON, 19, 18, 0),
            ],
        );
        town.render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
            .unwrap();
        let mut combat = combat_player_command_state(6, 5);
        combat.combat_cursor_blink = true;
        combat
            .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
            .unwrap();
        for state in [town, combat] {
            let pristine = state.clone();
            let first = state.peek_object_cells(&atlas, &masks);
            assert!(first.as_ref().is_some_and(|peek| !peek.cells.is_empty()));
            for _ in 0..100 {
                assert_eq!(state.peek_object_cells(&atlas, &masks), first);
            }
            assert_eq!(state.prng_state, pristine.prng_state);
            assert_eq!(format!("{state:?}"), format!("{pristine:?}"));
        }
    }

    /// Design 9.3.7, "What it never does": object_cells.rs names none of
    /// the render's mutating entry points and calls no `&mut self` method
    /// of the crate.
    #[test]
    fn the_peek_source_calls_nothing_that_mutates_the_state() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let source = std::fs::read_to_string(root.join("play_state_impl/object_cells.rs")).unwrap();
        for banned in [
            "refresh_top_down_visibility_buffers",
            "draw_active_object_composite_variant",
            "sync_player_object",
            "&mut self",
        ] {
            assert!(!source.contains(banned), "object_cells.rs names {banned}");
        }

        // Every `fn name(&mut self` in the crate's sources.
        let mut mutating = std::collections::BTreeSet::new();
        let mut dirs = vec![root];
        while let Some(dir) = dirs.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    dirs.push(path);
                    continue;
                }
                if path.extension().is_none_or(|ext| ext != "rs") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                for (at, _) in text.match_indices("(&mut self") {
                    let mut head = &text[..at];
                    // A generic method: `fn name<T>(&mut self`.
                    if head.ends_with('>') {
                        head = head.rsplit_once('<').map_or(head, |(left, _)| left);
                    }
                    let start = head
                        .rfind(|c: char| !(c.is_alphanumeric() || c == '_'))
                        .map_or(0, |i| i + 1);
                    let name = &head[start..];
                    if !name.is_empty() && head[..start].trim_end().ends_with("fn") {
                        mutating.insert(name.to_string());
                    }
                }
            }
        }
        assert!(mutating.contains("sync_player_object"), "the scan finds methods");
        for name in &mutating {
            let call = format!(".{name}(");
            assert!(
                !source.contains(&call),
                "object_cells.rs calls the mutating method {name}"
            );
        }
    }
}

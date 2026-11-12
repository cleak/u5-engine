// Phase B M2-02: the map-view kind (design 9.3.7) and the one predicate it
// shares with the ring peek's fixed-screen test (2.2.8).
mod map_view_kind {
    use super::*;

    fn cutscene_map() -> MiscmapsCutsceneMap {
        MiscmapsCutsceneMap {
            record_index: 0,
            tiles: vec![0x44; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS],
        }
    }

    fn sweep() -> VisibilitySweep {
        VisibilitySweep {
            frames_remaining: 7,
            pause_bios_ticks_per_frame: POTION_WHITE_SWEEP_BIOS_TICKS_PER_FRAME,
            center_x: 10,
            center_y: 20,
            visible_cells: [true; VIEWPORT_SIDE * VIEWPORT_SIDE],
        }
    }

    fn rescue_state(viewport: crate::blackthorn::BlackthornRescueViewport) -> PlayState {
        let mut state = test_state(open_grid(), 5, 5);
        state.pending_blackthorn_rescue = Some(crate::blackthorn::BlackthornRescuePhase::Narration);
        state.blackthorn_rescue_viewport = viewport;
        state
    }

    /// Every state the kind distinguishes, with design 9.3.7's answer.
    fn kind_cases() -> Vec<(&'static str, PlayState, Option<MapViewKind>)> {
        use crate::blackthorn::BlackthornRescueViewport;
        use MapViewKind::{Combat, Dungeon, Town, World};
        let mut cases = vec![
            ("town", test_state(open_grid(), 5, 5), Some(Town)),
            ("world", world_state(open_world_grid(), 10, 20), Some(World)),
            (
                "dungeon",
                dungeon_state(open_dungeon_record(), 0, 1, 1),
                Some(Dungeon),
            ),
            ("combat", combat_player_command_state(6, 5), Some(Combat)),
        ];

        let mut gem = test_state(open_grid(), 5, 5);
        gem.gems = 1;
        assert_eq!(gem.view_gem(), MoveOutcome::Observed);
        assert!(gem.active_view_overlay.is_some());
        cases.push(("gem view", gem, None));

        let mut endgame = world_state(open_world_grid(), 10, 20);
        endgame.endgame = Some(EndgameState::awaiting_first_confirmation());
        cases.push(("endgame", endgame, None));

        let mut world_sweep = world_state(open_world_grid(), 10, 20);
        world_sweep.visibility_sweep = Some(sweep());
        cases.push(("visibility sweep", world_sweep, None));

        // The overlay, the endgame and the sweep come before combat and the
        // dungeon in the order, as in the fixed-screen predicate.
        let mut combat_sweep = combat_player_command_state(6, 5);
        combat_sweep.visibility_sweep = Some(sweep());
        cases.push(("sweep in combat", combat_sweep, None));
        let mut dungeon_endgame = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon_endgame.endgame = Some(EndgameState::awaiting_first_confirmation());
        cases.push(("endgame over a dungeon", dungeon_endgame, None));

        let mut audience = test_state(open_grid(), 5, 5);
        audience.blackthorn_audience_map = Some(cutscene_map());
        cases.push(("Blackthorn audience", audience, None));

        let mut shrine = world_state(open_world_grid(), 10, 20);
        shrine.shrine_presentation_map = Some(cutscene_map());
        cases.push(("shrine presentation", shrine, None));

        let mut held_black = test_state(open_grid(), 5, 5);
        held_black.cutscene_black_viewport = true;
        cases.push(("held black cutscene field", held_black, None));

        let mut playback = test_state(open_grid(), 5, 5);
        playback
            .pending_blackthorn_rescue_playbacks
            .push(crate::blackthorn::blackthorn_rescue_playback());
        cases.push(("queued rescue playback", playback, None));

        cases.push((
            "rescue, black field",
            rescue_state(BlackthornRescueViewport::Black),
            None,
        ));
        cases.push((
            "rescue, party on black",
            rescue_state(BlackthornRescueViewport::PartyOnBlack),
            None,
        ));
        cases.push((
            "rescue, live grid",
            rescue_state(BlackthornRescueViewport::LiveGrid),
            Some(Town),
        ));
        cases
    }

    #[test]
    fn the_kind_follows_the_render_order() {
        for (label, state, expected) in kind_cases() {
            assert_eq!(state.map_view_kind(), expected, "{label}");
        }
    }

    /// One predicate (9.3.7): the fixed-screen test is the world and town
    /// kinds and nothing else.
    #[test]
    fn the_fixed_screen_predicate_is_the_top_down_kinds() {
        for (label, state, expected) in kind_cases() {
            assert_eq!(
                state.top_down_world_view_active(),
                matches!(expected, Some(MapViewKind::World | MapViewKind::Town)),
                "{label}"
            );
        }
        assert!(MapViewKind::World.is_top_down());
        assert!(MapViewKind::Town.is_top_down());
        assert!(!MapViewKind::Combat.is_top_down());
        assert!(!MapViewKind::Dungeon.is_top_down());
    }

    /// The runtime half of the pin: for the states the runtime render
    /// decides, the kind names the branch `render_top_down_viewport` takes.
    /// Combat keeps a previous frame, the dungeon clears it, and the map
    /// keeps one too. The shell's agreement test pins the rest.
    #[test]
    fn the_kind_agrees_with_the_runtime_render_branch() {
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let mut rendered_kinds = 0;
        for (label, state, expected) in kind_cases() {
            let Some(kind) = expected else { continue };
            let combat = state.combat_active;
            let mut rendered = state.clone();
            rendered.top_down_previous_viewport = None;
            let viewport = rendered
                .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
                .unwrap();
            assert!(viewport.is_some(), "{label}");
            let dungeon = rendered.top_down_previous_viewport.is_none();
            let branch = if combat {
                MapViewKind::Combat
            } else if dungeon {
                MapViewKind::Dungeon
            } else if matches!(rendered.area, Area::World { .. }) {
                MapViewKind::World
            } else {
                MapViewKind::Town
            };
            assert_eq!(kind, branch, "{label}");
            rendered_kinds += 1;
        }
        assert!(rendered_kinds >= 4);
    }

    /// The brightness step's kind (9.3.3, M2 gate review): the map-view
    /// kind, except that a visibility sweep counts as the map it sweeps
    /// over, so the White potion's sweep keeps the map's brightness. Combat
    /// and the dungeon draw their own view through a sweep, and every
    /// other `None` stays `None`.
    #[test]
    fn the_light_kind_counts_a_sweep_as_its_map() {
        for (label, state, expected) in kind_cases() {
            let light = match label {
                "visibility sweep" => Some(MapViewKind::World),
                "sweep in combat" => Some(MapViewKind::Combat),
                _ => expected,
            };
            assert_eq!(state.light_view_kind(), light, "{label}");
        }
        let mut town_sweep = test_state(open_grid(), 5, 5);
        town_sweep.visibility_sweep = Some(sweep());
        assert_eq!(town_sweep.map_view_kind(), None);
        assert_eq!(town_sweep.light_view_kind(), Some(MapViewKind::Town));
        let mut dungeon_sweep = dungeon_state(open_dungeon_record(), 0, 1, 1);
        dungeon_sweep.visibility_sweep = Some(sweep());
        assert_eq!(dungeon_sweep.light_view_kind(), Some(MapViewKind::Dungeon));
        // The overlay and the endgame still come first.
        let mut endgame_sweep = world_state(open_world_grid(), 10, 20);
        endgame_sweep.visibility_sweep = Some(sweep());
        endgame_sweep.endgame = Some(EndgameState::awaiting_first_confirmation());
        assert_eq!(endgame_sweep.light_view_kind(), None);
    }

    #[test]
    fn the_kind_leaves_the_state_untouched() {
        for (label, state, _) in kind_cases() {
            let before = format!("{state:?}");
            let prng = state.prng_state;
            for _ in 0..50 {
                let _ = state.map_view_kind();
                let _ = state.light_view_kind();
            }
            assert_eq!(state.prng_state, prng, "{label}");
            assert_eq!(format!("{state:?}"), before, "{label}");
        }
    }
}

// Phase B M1-05: the ring peek's types and the fixed-screen predicate
// (design 2.3.6, 2.2.8).
mod ring_peek_predicate {
    use super::*;

    fn rescue_state(viewport: crate::blackthorn::BlackthornRescueViewport) -> PlayState {
        let mut state = test_state(open_grid(), 5, 5);
        state.pending_blackthorn_rescue = Some(crate::blackthorn::BlackthornRescuePhase::Narration);
        state.blackthorn_rescue_viewport = viewport;
        state
    }

    fn cutscene_map() -> MiscmapsCutsceneMap {
        MiscmapsCutsceneMap {
            record_index: 0,
            tiles: vec![0x44; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS],
        }
    }

    /// Every state the predicate distinguishes, with the answer design
    /// 2.2.8 gives for it.
    fn predicate_cases() -> Vec<(&'static str, PlayState, bool)> {
        use crate::blackthorn::BlackthornRescueViewport;
        let mut cases = vec![
            ("town", test_state(open_grid(), 5, 5), true),
            ("world", world_state(open_world_grid(), 10, 20), true),
            (
                "dungeon",
                dungeon_state(open_dungeon_record(), 0, 1, 1),
                false,
            ),
            ("combat", combat_player_command_state(6, 5), false),
        ];

        let mut gem = test_state(open_grid(), 5, 5);
        gem.gems = 1;
        assert_eq!(gem.view_gem(), MoveOutcome::Observed);
        assert!(gem.active_view_overlay.is_some());
        cases.push(("gem view", gem, false));

        let mut endgame = world_state(open_world_grid(), 10, 20);
        endgame.endgame = Some(EndgameState::awaiting_first_confirmation());
        cases.push(("endgame", endgame, false));

        let mut sweep = world_state(open_world_grid(), 10, 20);
        sweep.visibility_sweep = Some(VisibilitySweep {
            frames_remaining: 7,
            pause_bios_ticks_per_frame: POTION_WHITE_SWEEP_BIOS_TICKS_PER_FRAME,
            center_x: 10,
            center_y: 20,
            visible_cells: [true; VIEWPORT_SIDE * VIEWPORT_SIDE],
        });
        cases.push(("visibility sweep", sweep, false));

        let mut audience = test_state(open_grid(), 5, 5);
        audience.blackthorn_audience_map = Some(cutscene_map());
        cases.push(("Blackthorn audience", audience, false));

        let mut shrine = world_state(open_world_grid(), 10, 20);
        shrine.shrine_presentation_map = Some(cutscene_map());
        cases.push(("shrine presentation", shrine, false));

        let mut held_black = test_state(open_grid(), 5, 5);
        held_black.cutscene_black_viewport = true;
        cases.push(("held black cutscene field", held_black, false));

        let mut playback = test_state(open_grid(), 5, 5);
        playback
            .pending_blackthorn_rescue_playbacks
            .push(crate::blackthorn::blackthorn_rescue_playback());
        cases.push(("queued rescue playback", playback, false));

        cases.push((
            "rescue, black field",
            rescue_state(BlackthornRescueViewport::Black),
            false,
        ));
        cases.push((
            "rescue, party on black",
            rescue_state(BlackthornRescueViewport::PartyOnBlack),
            false,
        ));
        // Before the dissolve the rescue draws nothing of its own and the
        // live map shows, so the frame is still the top-down view.
        cases.push((
            "rescue, live grid",
            rescue_state(BlackthornRescueViewport::LiveGrid),
            true,
        ));
        cases
    }

    #[test]
    fn the_predicate_is_true_only_for_the_plain_top_down_view() {
        for (label, state, expected) in predicate_cases() {
            assert_eq!(
                state.top_down_world_view_active(),
                expected,
                "{label}"
            );
        }
    }

    /// The runtime half of the agreement test: the branch
    /// `render_top_down_viewport` really takes. Combat keeps a previous
    /// frame, the dungeon clears it, and the map keeps one too, so the
    /// map is the branch that is neither.
    #[test]
    fn the_predicate_agrees_with_the_runtime_render_branch() {
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        for (label, state, expected) in predicate_cases() {
            // The cases the shell decides before the runtime render (the
            // cutscene fields, the overlay, the endgame and the sweep) are
            // pinned by the shell's own agreement test.
            let shell_fixed = state.blackthorn_audience_map.is_some()
                || state.shrine_presentation_map.is_some()
                || state.cutscene_black_viewport
                || !state.pending_blackthorn_rescue_playbacks.is_empty()
                || (state.blackthorn_rescue_active()
                    && state.blackthorn_rescue_viewport
                        != crate::blackthorn::BlackthornRescueViewport::LiveGrid)
                || state.active_view_overlay.is_some()
                || state.endgame.is_some()
                || state.visibility_sweep.is_some();
            if shell_fixed {
                assert!(!expected, "{label}");
                continue;
            }
            let combat = state.combat_active;
            let mut rendered = state.clone();
            rendered.top_down_previous_viewport = None;
            let viewport = rendered
                .render_top_down_base_frame(VIEWPORT_PLAYER_ROW, &atlas)
                .unwrap();
            assert!(viewport.is_some(), "{label}");
            let dungeon = rendered.top_down_previous_viewport.is_none();
            let map = !combat && !dungeon;
            assert_eq!(state.top_down_world_view_active(), map, "{label}");
            assert_eq!(map, expected, "{label}");
        }
    }

    const WIDE: PeekWindow = PeekWindow {
        west: 15,
        east: 15,
        north: 7,
        south: 7,
    };

    #[test]
    fn the_window_geometry_round_trips_and_counts_the_ring() {
        assert!(WIDE.is_valid());
        assert_eq!((WIDE.columns(), WIDE.rows()), (31, 15));
        for index in 0..WIDE.cell_count() {
            let (dx, dy) = WIDE.offset_of(index).unwrap();
            assert_eq!(WIDE.index_of(dx, dy), Some(index));
        }
        assert_eq!(WIDE.offset_of(WIDE.cell_count()), None);
        assert_eq!(WIDE.offset_of(0), Some((-15, -7)));
        assert_eq!(WIDE.index_of(0, 0), Some(7 * 31 + 15));
        assert_eq!(WIDE.index_of(16, 0), None);
        assert_eq!(WIDE.index_of(0, -8), None);
        assert_eq!(WIDE.index_of(isize::MIN, 0), None);

        // The design's cost figure: the 21:9 window is 43 by 15 cells, a
        // ring of 524.
        let ultrawide = PeekWindow {
            west: 21,
            east: 21,
            north: 7,
            south: 7,
        };
        assert_eq!((ultrawide.columns(), ultrawide.rows()), (43, 15));
        let core = (0..ultrawide.cell_count())
            .filter(|&index| {
                let (dx, dy) = ultrawide.offset_of(index).unwrap();
                PeekWindow::is_core(dx, dy)
            })
            .count();
        assert_eq!(core, 121);
        assert_eq!(ultrawide.cell_count() - core, 524);
    }

    #[test]
    fn a_window_must_hold_the_core_and_stay_within_forty_cells() {
        let state = test_state(open_grid(), 5, 5);
        for window in [
            PeekWindow { west: 4, ..WIDE },
            PeekWindow { south: 4, ..WIDE },
            PeekWindow { east: 41, ..WIDE },
            PeekWindow { north: 41, ..WIDE },
        ] {
            assert!(!window.is_valid(), "{window:?}");
            assert_eq!(
                state.peek_top_down_ring(window, PeekLightPolicy::Scaled),
                None,
                "{window:?}"
            );
        }
        for window in [
            PeekWindow {
                west: PEEK_CORE_REACH,
                east: PEEK_CORE_REACH,
                north: PEEK_CORE_REACH,
                south: PEEK_CORE_REACH,
            },
            PeekWindow {
                west: PEEK_MAX_REACH,
                east: PEEK_MAX_REACH,
                north: PEEK_MAX_REACH,
                south: PEEK_MAX_REACH,
            },
        ] {
            assert!(window.is_valid(), "{window:?}");
            assert!(
                state
                    .peek_top_down_ring(window, PeekLightPolicy::Scaled)
                    .is_some()
            );
        }
    }

    #[test]
    fn the_peek_follows_the_predicate_and_reveals_nothing_yet() {
        for (label, state, expected) in predicate_cases() {
            for light in [PeekLightPolicy::Scaled, PeekLightPolicy::Faithful] {
                let ring = state.peek_top_down_ring(WIDE, light);
                assert_eq!(ring.is_some(), expected, "{label}");
                let Some(ring) = ring else { continue };
                assert_eq!(ring.window, WIDE);
                assert!(!ring.animated, "{label}");
                assert_eq!(ring.cells.len(), WIDE.cell_count());
                for (index, cell) in ring.cells.iter().enumerate() {
                    let (dx, dy) = WIDE.offset_of(index).unwrap();
                    if PeekWindow::is_core(dx, dy) {
                        assert_eq!(*cell, None, "{label} core ({dx}, {dy})");
                    } else {
                        assert_eq!(*cell, Some(PeekCell::HIDDEN), "{label} ({dx}, {dy})");
                    }
                }
            }
        }
        assert_eq!(PeekCell::HIDDEN.tile, VISIBILITY_HIDDEN);
        assert_eq!(PeekCell::HIDDEN.shade, PeekShade::Hidden);
    }

    #[test]
    fn the_peek_leaves_the_state_untouched() {
        for (label, state, _) in predicate_cases() {
            let before = format!("{state:?}");
            let prng = state.prng_state;
            for call in 0..100 {
                let light = if call % 2 == 0 {
                    PeekLightPolicy::Scaled
                } else {
                    PeekLightPolicy::Faithful
                };
                let _ = state.top_down_world_view_active();
                let _ = state.peek_top_down_ring(WIDE, light);
            }
            assert_eq!(state.prng_state, prng, "{label}");
            assert_eq!(format!("{state:?}"), before, "{label}");
        }
    }
}

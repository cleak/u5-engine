// The u5-spec#307 answers: the controlled active player (Q1), the mountain
// reject on the Underworld plane (Q2) and the `Rough seas!` line feed (Q3).
mod spec307 {
    use super::*;

    // ---- Q1: `combat.md §8.4` eligibility includes the group test -------

    /// Roster `[0, 1]` seated in descriptors 0 and 1.
    fn two_member_arena() -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        let mut member = state.party[0];
        member.slot = 1;
        state.party.push(member);
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        state.combat_actors[1] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 6, 5]);
        state
    }

    #[test]
    fn q1_a_controlled_member_is_refused_and_the_selection_is_left_as_it_was() {
        // `combat.md §8.4` (R643): eligibility needs "the slot-to-group
        // helper of Section 6.1a puts it in the player-command group". A
        // controlled member "is refused exactly as if no member matched: the
        // ordinary invalid case ... and the selection is left as it was".
        let mut state = two_member_arena();
        assert_eq!(
            state.apply_combat_active_player_digit('1'),
            CombatActivePlayerSelectionOutcome::SelectPartySlot(0)
        );
        state.combat_actors[1].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;

        assert_eq!(
            state.apply_combat_active_player_digit('2'),
            CombatActivePlayerSelectionOutcome::Invalid
        );
        assert_eq!(state.active_player, Some(0), "the old selection stands");

        // Unset, it stays unset.
        state.active_player = None;
        assert_eq!(
            state.apply_combat_active_player_digit('2'),
            CombatActivePlayerSelectionOutcome::Invalid
        );
        assert_eq!(state.active_player, None);

        // Once Charm clears the bit (writer 2's toggle) the member is back in
        // the player-command group and selectable again.
        state.combat_actors[1].flags &= !COMBAT_ACTOR_FLAG_CONTROLLED;
        assert_eq!(
            state.apply_combat_active_player_digit('2'),
            CombatActivePlayerSelectionOutcome::SelectPartySlot(1)
        );
        assert_eq!(state.active_player, Some(1));
    }

    #[test]
    fn q1_the_refusal_prints_both_rows_of_the_invalid_case() {
        // §8.4: "`Set active plr:` on one row and `Invalid!` on the next,
        // with no separate message".
        let mut state = two_member_arena();
        state.combat_active = true;
        state.combat_actors[1].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        let application = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('2'))
            .expect("the selected-none handler prompts party slot 0");
        assert!(matches!(
            application.action,
            CombatPlayerCommandAction::ActivePlayerSelection(
                CombatActivePlayerSelectionOutcome::Invalid
            )
        ));
        assert_eq!(state.active_player, None);
    }

    #[test]
    fn q1_a_selection_can_never_leave_the_group_zero_side_unprompted() {
        // `combat.md §6.1a` (issue #307 Q1): "The active-player selection can
        // never name a party slot that this helper puts in the automatic
        // group", so "the command handler's sentinel gate therefore never
        // skips the whole group-0 side while the selected member sits on the
        // automatic driver" (playtest P13-13).
        let mut state = two_member_arena();
        state.combat_actors[1].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        for key in ['1', '2'] {
            let _ = state.apply_combat_active_player_digit(key);
            let prompted = (0..COMBAT_PARTY_ACTOR_SLOTS).any(|slot| {
                combat_slot_prompted_by_player_command_handler(
                    state.active_player,
                    slot,
                    state.combat_actors[slot],
                )
            });
            assert!(prompted, "after {key}: {:?}", state.active_player);
        }
        assert_eq!(state.active_player, Some(0));
    }

    // ---- Q2: `encounters.md §4` mountain reject on both planes ----------

    #[test]
    fn q2_mountains_and_high_peaks_are_rejected_on_both_planes() {
        // R644: "Tiles `0x0C` and `0x0D` are rejected on both planes, before
        // the plane is consulted ... never reaches either land bucket or the
        // water allowance die."
        for underworld in [false, true] {
            for tile in [0x0Cu8, 0x0D] {
                assert_eq!(
                    spawn_terrain_branch(tile, underworld),
                    SpawnTerrainBranch::HardReject,
                    "tile {tile:#04x} underworld {underworld}"
                );
            }
            // "Terrain tile 7 (parched desert), on either plane".
            assert_eq!(
                spawn_terrain_branch(0x07, underworld),
                SpawnTerrainBranch::SandTrapParchedDesert
            );
        }
        // A control tile that does reach the plane's land bucket.
        assert_eq!(
            spawn_terrain_branch(0x0A, true),
            SpawnTerrainBranch::LandBucket
        );
    }

    // ---- Q3: `Rough seas!` ends with one line feed ----------------------

    fn rough_seas_state() -> PlayState {
        let mut grid = open_world_grid();
        grid[world_cell_index(5, 5)] = BRIT_DEEP_WATER_TILE;
        let mut state = world_state(grid, 5, 5);
        state.player.transport = TransportState::Skiff {
            type_byte: TRANSPORT_MARKER_SKIFF_FIRST,
            tile: TRANSPORT_MARKER_SKIFF_FIRST,
        };
        state.emit_message_line("Pass\n");
        state.message_row_open_mid_line = false;
        state
    }

    fn rows(state: &PlayState) -> Vec<String> {
        state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect()
    }

    #[test]
    fn q3_rough_seas_closes_its_row() {
        // `overworld.md §6.2.5`: "The stored line is `Rough seas!` followed
        // by one line feed, with no leading feed ... it lands on the row
        // directly below the echo ... and it leaves the cursor at column 0
        // of a fresh row."
        let mut state = rough_seas_state();
        assert!(state.apply_rough_seas_if_eligible().is_some());
        assert!(!state.message_row_open_mid_line);
        let rows = rows(&state);
        assert_eq!(
            rows[rows.len() - 2..],
            ["Pass".to_string(), "Rough seas!".to_string()],
            "{rows:?}"
        );
        // A closed row: the window derives the blank above the next prompt
        // ("No engagement | `Rough seas!`, blank, next prompt row").
        assert!(!state.message_transcript.last().unwrap().leaves_row_open);
    }

    #[test]
    fn q3_attacked_after_rough_seas_spends_a_blank_row() {
        // §6.2.5 table: "Generic hostile engages | `Rough seas!`, blank,
        // `Attacked!`, blank, next prompt row" (playtest P13-04 case 2).
        let mut state = rough_seas_state();
        assert!(state.apply_rough_seas_if_eligible().is_some());
        state.emit_encounter_line("Attacked!");
        let rows = rows(&state);
        assert_eq!(
            rows[rows.len() - 3..],
            [
                "Rough seas!".to_string(),
                String::new(),
                "Attacked!".to_string()
            ],
            "{rows:?}"
        );
    }

    #[test]
    fn q3_whirlpool_after_rough_seas_spends_a_blank_row() {
        // §6.2.5 table: "Whirlpool engages | `Rough seas!`, blank,
        // `WHIRLPOOL!`, blank, next prompt row" (playtest P13-04 case 3).
        let mut state = rough_seas_state();
        assert!(state.apply_rough_seas_if_eligible().is_some());
        state.emit_overworld_whirlpool_banner();
        let rows = rows(&state);
        assert_eq!(
            rows[rows.len() - 3..],
            [
                "Rough seas!".to_string(),
                String::new(),
                "WHIRLPOOL!".to_string()
            ],
            "{rows:?}"
        );
    }
}

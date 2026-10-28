// Regressions for the thirteenth exploratory playtest's movement findings
// (P13-xx). Each test names the finding and the published sentence it rests
// on.
mod playfix13_move13 {
    use super::*;

    fn orc_at(x: usize, y: usize) -> ActiveObject {
        ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x,
            y,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        }
    }

    #[test]
    fn p13_01_difficult_step_that_fights_still_charges_its_terrain_and_turn() {
        // `movement.md §8.1`: "Suppression does not remove the terrain's time
        // charge" and "The ordinary consumed-action tail subsequently performs
        // its own clock and upkeep work"; `active-objects.md §8`: "A generic
        // terrain-combat call can therefore return". A hills step whose first
        // extra call opens a fight still costs 4 + 2 minutes and a turn once
        // the fight returns.
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(6, 5)] = 0x0a;
        let mut state = world_state(grid, 5, 5);
        state.active_objects.push(orc_at(7, 5));
        let clock = state.clock;

        let outcome = state
            .step_world(Direction::East, 6, 5, WorldPlane::Underworld, Some(&dir))
            .unwrap();

        assert!(state.combat_active, "outcome {outcome:?}");
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, clock);
        assert_eq!(
            state.pending_difficult_terrain_step.map(|p| p.remaining_calls),
            Some(1)
        );

        state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
        // The orc is gone (say, slain), so the second extra call fights no one.
        state.active_objects.truncate(1);
        state
            .resume_difficult_terrain_step_after_combat(&dir)
            .unwrap();

        assert!(!state.combat_active);
        assert!(state.pending_difficult_terrain_step.is_none());
        assert_eq!((state.player.x, state.player.y), (6, 5));
        assert_eq!(state.turn, 1);
        let mut expected = clock;
        expected.advance_minutes(6);
        assert_eq!(state.clock, expected);
        // The call that fought reported a hostile interaction.
        assert!(
            !state
                .message_entries()
                .iter()
                .any(|entry| entry.text.contains("Very slow!"))
        );
        let _ = fs::remove_dir_all(dir);
    }

    fn rows(state: &PlayState) -> Vec<String> {
        state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect()
    }

    #[test]
    fn p13_15_attacked_after_a_completed_row_spends_its_leading_feed_as_a_blank() {
        // `text-output.md §10.2`: `Attacked!` is "a literal that begins with
        // a line feed"; §10.4: "a line feed always advances the row". After
        // the `Pass\n` echo the cursor is at column 0, so the feed spends a
        // blank row (playtest P13-15, P13-04 case 1).
        let mut state = world_state(open_world_grid(), 5, 5);
        state.emit_message_line("Pass\n");
        state.message_row_open_mid_line = false;

        state.emit_encounter_line("Attacked!");

        let rows = rows(&state);
        assert_eq!(
            rows[rows.len() - 3..],
            ["Pass".to_string(), String::new(), "Attacked!".to_string()],
            "{rows:?}"
        );
    }

    #[test]
    fn p13_15_attacked_after_an_open_row_closes_it_without_a_blank() {
        // The same leading feed closes a row still open mid-line.
        // (This used `Rough seas!` as its open row; issue #307 Q3 settled
        // that that line ends in a feed, so a neutral open row stands in.)
        let mut state = world_state(open_world_grid(), 5, 5);
        state.emit_message_line("Open row");
        state.message_row_open_mid_line = true;

        state.emit_encounter_line("Attacked!");

        let rows = rows(&state);
        assert_eq!(
            rows[rows.len() - 2..],
            ["Open row".to_string(), "Attacked!".to_string()],
            "{rows:?}"
        );
    }
}

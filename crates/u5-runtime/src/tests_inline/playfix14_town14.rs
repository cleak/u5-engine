// Regressions for the fourteenth exploratory playtest's town findings
// (P14-xx). Each test names the finding and the published sentence it rests
// on.
mod playfix14_town14 {
    use super::*;

    fn rows(state: &PlayState) -> Vec<String> {
        state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect()
    }

    #[test]
    fn p14_12_town_attacked_spends_its_leading_feed_after_a_completed_row() {
        // `town-mode.md §14`: the conflict outcome "prints `Attacked!` with
        // its leading line feed"; `text-output.md §10.4`: after a result that
        // ends in a feed, the leading feed of a same-turn `Attacked!` "spends
        // a blank row of its own".
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 5, 5);
        state.load_scheduled_npcs(&[
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            NpcSlot {
                slot: 1,
                type_byte: 0x50,
                dialog_id: 2,
                schedule: [7, 7, 7, 6, 6, 6, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        state.emit_message_line("North\n");
        state.message_row_open_mid_line = false;

        assert_eq!(
            state.pass_turn_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::Used
        );

        let rows = rows(&state);
        let at = rows
            .iter()
            .rposition(|row| row == TOWN_NPC_CONTACT_ATTACKED_LINE)
            .unwrap_or_else(|| panic!("{rows:?}"));
        assert_eq!(rows[at - 1], "", "{rows:?}");
        assert_eq!(rows[at - 2], "North", "{rows:?}");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn p14_12_town_attacked_on_an_open_row_only_closes_it() {
        let mut state = test_state(open_grid(), 5, 5);
        state.emit_message_line("Blocked!");
        state.message_row_open_mid_line = true;

        state.emit_town_attacked_line();

        let rows = rows(&state);
        assert_eq!(
            rows[rows.len() - 2..],
            ["Blocked!".to_string(), "Attacked!".to_string()],
            "{rows:?}"
        );
    }
}

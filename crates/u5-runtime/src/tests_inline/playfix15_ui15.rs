// Regressions for the fifteenth exploratory playtest's UI findings
// (P15-xx). Each test names the finding and the published sentence it rests
// on.
mod playfix15_ui15 {
    use super::*;

    fn rows(state: &PlayState) -> Vec<String> {
        state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect()
    }

    fn three_member_town_state() -> PlayState {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.party.push(audit_g5_companion(1));
        state.party.push(audit_g5_companion(2));
        state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0", *b"IOLO\0\0\0\0\0"];
        state.active_player = None;
        state
    }

    fn search_east_through_the_member_exchange(state: &mut PlayState) {
        let dir = debug_game_dir();
        // The direction form of Search: `Search-East`, then the shared
        // acting-member exchange (three able members, so it prompts).
        state.emit_command_echo_line("Search-East");
        assert_eq!(
            state.start_party_selector(PartySelectorTarget::Search {
                direction: Direction::East
            }),
            MoveOutcome::Observed
        );
        assert!(state.step_active_party_selector_with_game_dir('2', "", &dir));
        assert!(state.step_active_party_selector_with_game_dir('\r', "", &dir));
        let _ = fs::remove_dir_all(dir);
    }

    fn player_rows(state: &PlayState) -> usize {
        rows(state)
            .iter()
            .filter(|row| row.starts_with("Player:"))
            .count()
    }

    /// P15-10: `commands.md §5.8` - "The direction form of Search always
    /// invokes this acting-member selection before searching", and its
    /// corpse row is the one "after Search-North and the member exchange";
    /// `containers.md §9`: "Before the lookup, Search chooses the searching
    /// member through the shared acting-member selection". One exchange, so
    /// the corpse lookup must not open a second `Player: ` picker.
    #[test]
    fn p15_10_search_on_a_moldy_corpse_asks_for_the_member_once() {
        let mut state = three_member_town_state();
        state
            .active_objects
            .push(ActiveObject::fixed_hidden_treasure_pickup(40, 2, 1, 0));
        assert!(state.moldy_corpse_search_slot_at(2, 1).is_some());
        let turn = state.turn;

        search_east_through_the_member_exchange(&mut state);

        assert!(state.active_party_selector.is_none());
        assert!(
            state.active_surface_chest.is_none(),
            "no second picker opens: {:?}",
            rows(&state)
        );
        assert!(
            player_rows(&state) <= 1,
            "one `Player:` row: {:?}",
            rows(&state)
        );
        assert_eq!(state.turn, turn + 1, "the corpse search ran");
        assert!(
            state.message.contains("Thou dost find"),
            "{:?}",
            state.message
        );
    }

    /// P15-10, the container row of the same table: `... no trap!` follows
    /// the single member exchange.
    #[test]
    fn p15_10_search_on_a_town_chest_asks_for_the_member_once() {
        let mut state = three_member_town_state();
        state.active_objects.push(ActiveObject {
            type_byte: 0x01,
            tile: 0x50,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        assert!(state.surface_object_chest_slot_at(2, 1).is_some());

        search_east_through_the_member_exchange(&mut state);

        assert!(state.active_party_selector.is_none());
        assert!(
            state.active_surface_chest.is_none(),
            "no second picker opens: {:?}",
            rows(&state)
        );
        assert!(
            player_rows(&state) <= 1,
            "one `Player:` row: {:?}",
            rows(&state)
        );
        assert!(state.message.contains("no trap!"), "{:?}", state.message);
        // The member-pinned lookup leaves nothing behind for a later caller.
        assert!(state.search_acting_member.is_none());
    }

    /// P15-11: `inventory.md §5.2` - "Ordinary successful equip and unequip
    /// print no result message, no item name echo and no fresh `Item:_`
    /// prompt. The picker remains active." A refusal's block already ends in
    /// `Item: `, so a success after it, then a highlight move, must not log
    /// a second consecutive `Item:` row.
    #[test]
    fn p15_11_a_ready_success_after_a_refusal_adds_no_second_item_row() {
        const INV_RING: usize = 42;
        const PROT_RING: usize = 43;
        const REGEN_RING: usize = 44;
        const SP_COLLAR: usize = 46;
        const ANKH: usize = 47;
        let dir = debug_game_dir();
        let mut state = p4window_world_state();
        for item in INV_RING..=ANKH {
            state.equipment_stock[item] = 1;
        }
        state.party_equipment[0] = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
        assert_eq!(
            state.ready_equipment(InlineReadyRequest {
                party_index: 0,
                item_id: ANKH,
            }),
            MoveOutcome::Used
        );
        state.message.clear();

        p4window_keys(&mut state, &dir, &['R', '\r']);
        assert!(state.active_ready.as_ref().unwrap().selected_party_index == Some(0));
        // The amulet slot is taken: refused.
        state.active_ready.as_mut().unwrap().cursor = SP_COLLAR;
        p4window_keys(&mut state, &dir, &['\r']);
        // Then a ring, which readies silently.
        state.active_ready.as_mut().unwrap().cursor = PROT_RING;
        p4window_keys(&mut state, &dir, &['\r']);
        assert!(state.party_equipment[0].contains(&(PROT_RING as u8)));
        // Then a highlight move.
        p4window_keys(&mut state, &dir, &[char::from(INPUT_CODE_SOUTH)]);

        let rows = p4window_rows(&state);
        let tail = p4window_rows_from(&rows, "amulet!");
        assert_eq!(tail, ["amulet!", "", "Item:"], "{rows:?}");

        // The finding's variant: a ring refusal next prints its block under
        // the one `Item:` row.
        state.active_ready.as_mut().unwrap().cursor = REGEN_RING;
        p4window_keys(&mut state, &dir, &['\r']);
        let rows = p4window_rows(&state);
        assert!(
            !rows
                .windows(2)
                .any(|pair| pair[0] == "Item:" && pair[1] == "Item:"),
            "{rows:?}"
        );
        let _ = fs::remove_dir_all(dir);
    }
}

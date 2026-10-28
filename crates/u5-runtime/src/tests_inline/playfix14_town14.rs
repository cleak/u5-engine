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

    fn treasury_chest_state() -> PlayState {
        // The roster chest of the treasury: kind 1, a furniture tile, and a
        // content byte of zero.
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
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
        state
    }

    #[test]
    fn p14_13_a_class_zero_kind_one_chest_opens_empty() {
        // `magic.md`: the chest-on-floor scan matches "object kind 1 — the
        // chest object class"; `doors-and-z-transitions.md §3.2`: "There is
        // no reachable state in which a container exists but cannot be
        // opened"; `containers.md §4`: the helper clears the record and
        // debits standing by two in town.
        let mut state = treasury_chest_state();
        state.moral_standing = 50;

        assert_eq!(state.open_facing(), MoveOutcome::ContainerOpened);

        assert!(state.active_objects[1].is_empty());
        assert_eq!(state.moral_standing, 48);
        assert!(state.surface_object_chest_slot_at(2, 1).is_none());
    }

    #[test]
    fn p14_13_a_class_zero_kind_one_chest_is_searched_and_picked() {
        // `commands.md §7`: Search "accepts only the container class" and
        // narrates its trap state; `doors §3.2`: an unlocked container is
        // not re-rolled - the broken-key result, one key spent.
        let mut state = treasury_chest_state();
        assert!(state.search_surface_object_trap_at(2, 1).is_some());
        assert!(state.message.contains("no trap!"), "{}", state.message);
        state.keys = 3;

        state.jimmy_facing();

        assert_eq!(state.keys, 2);
        assert_eq!(state.message, "Key broke!");
        assert!(!state.active_objects[1].is_empty());
    }

    #[test]
    fn p14_13_get_on_a_kind_one_chest_still_answers_open_it_first() {
        // `containers.md §3` step 4: "a container record answers
        // `Open_it_first!`" and changes nothing.
        let dir = debug_game_dir();
        let mut state = treasury_chest_state();

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(state.message, GET_MUST_OPEN_FIRST_REFUSAL);
        assert!(!state.active_objects[1].is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn p14_06_floor_relink_keeps_a_parked_carpet_and_unlinks_npcs() {
        // `town-mode.md §11`: on a floor change "NPCs not on the new floor
        // are unlinked, NPCs on the new floor are linked"; nothing removes a
        // loose record, and `vehicles.md §2` keeps a parked vehicle as an
        // active-object record with its floor marker.
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
                schedule: [7, 7, 7, 1, 1, 1, 5, 5, 5, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        let npc_slot = state.npcs[0].active_object.expect("NPC linked on floor 0");
        let carpet = ActiveObject {
            type_byte: CARPET_TRANSPORT_FIRST,
            tile: CARPET_TRANSPORT_FIRST,
            x: 3,
            y: 4,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        let carpet_slot = state.allocate_active_object_slot(carpet).unwrap();

        // Up to floor 1, where the NPC is not, and back down.
        let Area::Town { scene, .. } = state.area else {
            unreachable!()
        };
        state.area = Area::Town { scene, floor: 1 };
        state.relink_npc_objects();
        assert_eq!(state.active_objects[carpet_slot], carpet);
        assert!(state.active_objects[npc_slot].is_empty());
        assert!(state.npcs[0].active_object.is_none());

        state.area = Area::Town { scene, floor: 0 };
        state.relink_npc_objects();
        assert_eq!(state.active_objects[carpet_slot], carpet);
        assert!(state.npcs[0].active_object.is_some());
        let linked = state
            .active_objects
            .iter()
            .filter(|object| object.type_byte == 0x50)
            .count();
        assert_eq!(linked, 1, "the NPC is linked exactly once");
    }

    #[test]
    fn p14_07_town_jimmy_with_no_keys_refuses_before_the_direction_prompt() {
        // `doors-and-z-transitions.md §3`: other scenes "check key inventory
        // up front: if the key inventory is zero, Jimmy prints \"No keys!\"
        // and returns"; `combat.md §7.1`: "before the direction prompt".
        let dir = debug_game_dir();
        for mut state in [test_state(open_grid(), 5, 5), p4window_world_state()] {
            state.keys = 0;
            let turn = state.turn;

            p4window_keys(&mut state, &dir, &['J']);

            assert!(state.active_direction_prompt.is_none());
            assert_eq!(state.turn, turn + 1);
            let rows = p4window_rows(&state);
            assert!(
                rows.iter().any(|row| row.ends_with("Jimmy-No keys!")),
                "{rows:?}"
            );
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn p14_08_prisoner_thanks_is_bracketed_by_line_feeds() {
        // `doors-and-z-transitions.md §3.1`: "the visible quoted line `I
        // thank thee!`, bracketed by newlines". The `Jimmy-East` echo ends
        // its row, so the leading feed leaves one blank row.
        let dir = debug_game_dir();
        let mut grid = open_grid();
        grid[32 + 2] = JIMMY_MANACLES_TILE;
        let mut state = test_state(grid, 1, 1);
        state.party[0].climb_stat = 30;
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
                type_byte: 0x0E,
                dialog_id: 2,
                schedule: [1, 2, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0],
                name: None,
            },
        ]);

        p4window_keys(
            &mut state,
            &dir,
            &['J', char::from(input_direction_code(InputDirection::East))],
        );

        let rows = p4window_rows(&state);
        assert_eq!(
            p4window_rows_from(&rows, ">Jimmy-East")[..3],
            [
                ">Jimmy-East".to_string(),
                String::new(),
                "I thank thee!".to_string()
            ],
            "{rows:?}"
        );
        let _ = fs::remove_dir_all(dir);
    }
}

// Regressions for the p7conv exploratory-playtest findings (L01-L06).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `overworld.md §9.2`: from the overworld "the warp writes the outdoor object
/// table to the plane's canonical `.OOL` while Stage B still holds the marker
/// at zero". An Underworld-to-surface Gate Travel therefore leaves a parked
/// carpet in `UNDER.OOL`, so a save on the surface keeps it (L04).
#[test]
fn playtest_l04_gate_travel_writes_the_departed_plane_table_to_its_ool() {
    let dir = debug_game_dir();
    write_empty_ool_mirrors(&dir);
    let mut state = world_state(open_world_grid(), 40, 40);
    let carpet = ActiveObject {
        type_byte: 0x14,
        tile: 0x14,
        x: 41,
        y: 40,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.active_objects.push(carpet);
    state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    state.party[0].mana = 99;
    state.party[0].level = 8;
    state.moonstone_slots[0] = MoonstoneGateSlot {
        scene: 0,
        x: 50,
        y: 50,
        z: 0,
    };

    handle_play_key_input(&mut state, 'C', "1PRV1", &dir).unwrap();

    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    let under = fs::read(dir.join(UNDER_OOL_FILENAME)).unwrap();
    let table = decode_full_ool_plane_table(&under).unwrap();
    assert_eq!(table[1], carpet);
    // Slot zero's type byte is the zeroed Stage B marker.
    assert_eq!(under[0], 0);
    let _ = fs::remove_dir_all(dir);
}

/// `overworld.md §8.1` "The falls plane arm's disk handoff": "the handler
/// writes the surface active-object file from the live object table" before
/// the Underworld is set up, so a parked horse survives a save taken below
/// (L04).
#[test]
fn playtest_l04_world_plane_transition_writes_the_departed_plane_ool() {
    let dir = debug_game_dir();
    write_empty_ool_mirrors(&dir);
    let mut state = britannia_state(open_world_grid(), 54, 136);
    let horse = ActiveObject {
        type_byte: 0x10,
        tile: 0x10,
        x: 225,
        y: 133,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.active_objects.push(horse);
    state
        .apply_world_plane_transition(
            &dir,
            WorldPlaneTransitionEntry {
                from_plane: WorldPlane::Britannia,
                x: 54,
                y: 138,
                to_plane: WorldPlane::Underworld,
                to_x: 54,
                to_y: 138,
                expected_tile: None,
                preserves_transport: true,
            },
        )
        .unwrap();
    let brit =
        decode_full_ool_plane_table(&fs::read(dir.join(BRIT_OOL_FILENAME)).unwrap()).unwrap();
    assert_eq!(brit[1], horse);
    let _ = fs::remove_dir_all(dir);
}

/// `overworld.md §9.3`: the gate streams "contain one leading LF and one
/// trailing LF" and run after the step's closed `North` echo, so one blank
/// row separates the echo from the opening quote on both arms (L06).
#[test]
fn playtest_l06_codex_gate_keeps_its_leading_line_feed_as_a_blank_row() {
    for (mask, first_row) in [
        (0u8, "\"Thou art not upon a Sacred Quest!"),
        (1u8, "\"Pass, Seeker!\""),
    ] {
        let dir = debug_game_dir();
        let mut state = britannia_state(
            open_world_grid(),
            NARRATIVE_GATE_X as usize,
            NARRATIVE_GATE_Y as usize + 1,
        );
        state.shrine_ordained_mask = mask;

        handle_play_key_input(&mut state, '\u{3}', "", &dir).unwrap();
        state.flush_message_slot();

        let entries = state.message_entries();
        let echo = entries
            .iter()
            .rposition(|entry| entry.is_command_echo && entry.text == "North")
            .expect("North echo");
        let texts: Vec<&str> = entries[echo + 1..]
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        assert_eq!(texts.first(), Some(&""), "mask {mask}: {texts:?}");
        assert_eq!(texts.get(1), Some(&first_row), "mask {mask}: {texts:?}");
        assert!(!state.message_row_open_mid_line);
        let _ = fs::remove_dir_all(dir);
    }
}

/// `commands.md §5.5` Enter class table: "| ruins | `ruins` with no newline;
/// this is a direct non-transition arm |". A ruined shrine underfoot is not
/// the `What?` refusal (L05).
#[test]
fn playtest_l05_enter_on_a_ruined_shrine_prints_ruins_without_a_newline() {
    let dir = debug_game_dir();
    let mut grid = open_world_grid();
    grid[world_cell_index(233, 66)] = WORLD_RUINED_SHRINE_TILE;
    let mut state = britannia_state(grid, 233, 66);
    let turn = state.turn;
    state.begin_command_echo_for(Command::Enter);

    let outcome = state.enter_current_location(&dir).unwrap();

    assert_eq!(outcome, MoveOutcome::Blocked);
    assert_eq!(state.message_entries()[0].text, "Enter ruins");
    // `text-output.md §10.4`: a result with no trailing feed leaves its row
    // open, so the next prompt lands directly beneath it.
    assert!(state.message_entries()[0].leaves_row_open);
    assert!(state.message_row_open_mid_line);
    assert!(matches!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    ));
    assert_eq!((state.player.x, state.player.y), (233, 66));
    assert_eq!(state.turn, turn);
    let _ = fs::remove_dir_all(dir);
}

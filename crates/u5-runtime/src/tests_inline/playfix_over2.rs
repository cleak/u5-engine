// Regressions for the second overworld exploratory play-test batch
// (`over2`: F18..F23, F56). Each test names the finding it reproduces and
// the published sentence it rests on.

/// F23: `inventory.md §5` timing contract - outside combat R-Ready's "only
/// charge is Once in the post-action mode epilogue", and `commands.md §4`
/// has R and Z "always report the default 'acted'", consumed by the mode
/// loop after the handler returns. No clock minute and no world turn may
/// run while `Player: ` still waits; closing the exchange pays exactly one.
#[test]
fn playfix_f23_ready_and_z_stats_charge_after_their_prompts_close() {
    for key in ['R', 'Z'] {
        let mut state = world_state(open_world_grid(), 5, 5);
        let turn = state.turn;
        let clock = state.clock;
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert!(
            state.active_ready.is_some() || state.active_party_selector.is_some(),
            "{key}: the member prompt is open"
        );
        assert_eq!(state.turn, turn, "{key}: no world turn while `Player: ` waits");
        assert_eq!(state.clock, clock, "{key}: no clock minute while `Player: ` waits");

        handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
        assert!(state.active_ready.is_none() && state.active_party_selector.is_none());
        assert_eq!(state.turn, turn + 1, "{key}: the exchange costs one turn");
        assert_ne!(state.clock, clock, "{key}: the epilogue advances the clock");
        assert!(!state.exploration_acted_charge_owed);
    }
}

/// F23: a browsed Z-stats sheet is charged once, when the sheet closes, not
/// when the member is chosen.
#[test]
fn playfix_f23_z_stats_sheet_charges_on_exit() {
    let mut state = world_state(open_world_grid(), 5, 5);
    let turn = state.turn;
    handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    assert!(state.active_z_stats.is_some());
    assert_eq!(state.turn, turn);
    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert!(state.active_z_stats.is_none());
    assert_eq!(state.turn, turn + 1);
}

/// F18: `overworld.md §9.2` - "Nothing about ... party transport ... gates
/// it" and "The party's transport marker is restored on every path that ran
/// the transition"; the hook "calls the same saved-slot warp helper used by
/// Gate Travel". A mounted horse or a boarded carpet arrives with the party
/// instead of being dropped (and, for the carpet, lost for good).
#[test]
fn playfix_f18_saved_slot_warp_keeps_the_horse_or_carpet() {
    for transport in [
        TransportState::Horse {
            type_byte: 160,
            tile: 160,
        },
        TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        },
    ] {
        let dir = debug_game_dir();
        let mut state = britannia_state(open_world_grid(), 5, 5);
        state.player.transport = transport;
        state.sync_player_object();
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = 9;
        state.party[0].level = 8;
        state.moonstone_slots[1] = MoonstoneGateSlot {
            scene: 0,
            x: 6,
            y: 7,
            z: 0,
        };

        handle_play_key_input(&mut state, 'C', "1PRV2", &dir).unwrap();

        assert_eq!((state.player.x, state.player.y), (6, 7), "{transport:?}");
        assert_eq!(state.player.transport, transport);
        let slot_zero = state.active_objects[0];
        assert_eq!(
            (slot_zero.x as usize, slot_zero.y as usize),
            (6, 7),
            "slot zero follows the party"
        );
        let _ = fs::remove_dir_all(dir);
    }
}

/// F20/F56: `commands.md §5.5` and `dungeon-mode.md §11` print the watch
/// question as `\nWilt thou set a watch? ` (trailing space, no feed),
/// answered `Yes\n\n`/`No\n\n`, and the guard prompt as `Who will stand
/// guard? ` in the same form, its cancel printing `None posted!\n\n`. Each
/// answer continues its question's row, and while either waits no
/// command-row end cap is drawn (`§5.1`). (`No` starts the camp body, which
/// needs the asset tables, so the `Yes` and guard-cancel arms carry it.)
#[test]
fn playfix_f20_watch_answers_continue_the_prompt_row_without_a_marker() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    let rows = |state: &PlayState| -> Vec<String> {
        state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect()
    };
    for key in ['h', '3'] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
    }
    assert_eq!(state.message, REST_WATCH_PROMPT);
    assert!(
        state.message_window_live_row_suppressed(),
        "the watch question keeps the cursor inline"
    );

    handle_play_key_input(&mut state, 'y', "", Path::new("")).unwrap();
    let answered = format!(
        "{}{}",
        REST_WATCH_PROMPT.trim_start(),
        REST_WATCH_YES_LITERAL.trim_end()
    );
    assert!(rows(&state).contains(&answered), "{:?}", rows(&state));
    assert_eq!(state.message, REST_WATCH_MEMBER_PROMPT);
    assert!(
        state.message_window_live_row_suppressed(),
        "the guard prompt keeps the cursor inline"
    );

    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    let cancelled = format!(
        "{REST_WATCH_MEMBER_PROMPT}{}",
        REST_NO_WATCH_LITERAL.trim_end()
    );
    assert!(rows(&state).contains(&cancelled), "{:?}", rows(&state));
    assert_eq!(state.message, REST_WATCH_PROMPT);
}

/// F19: `overworld.md §6.2.5` - every released refused under-sail step
/// "prints COLLISION!" (`BREAKING UP!` on shoal) before the rumble and the
/// hull draw, and `weather.md §5.1` clears the cached heading so the next
/// key tries again. A repeat is byte-identical to the line before it and
/// must still reach the window.
#[test]
fn playfix_f19_each_under_sail_collision_prints_its_own_line() {
    for (tile, line) in [(0x05_u8, "COLLISION!"), (0x03, "BREAKING UP!")] {
        let mut state = world_state(open_world_grid(), 5, 5);
        for _ in 0..3 {
            let _ = state.apply_sailing_collision(tile);
        }
        state.flush_message_slot();
        let printed = state
            .message_transcript
            .iter()
            .filter(|entry| entry.text == line)
            .count();
        assert_eq!(printed, 3, "{line}");
    }
}

/// F21: `vehicles.md §4` - a mounted party over a skiff gets "the stock
/// \"On foot\" refusal", and `commands.md §5.5` publishes that stock
/// literal as `On_foot!\n`, not `On foot.`.
#[test]
fn playfix_f21_board_refusal_is_the_stock_on_foot_literal() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport = TransportState::Horse {
        type_byte: 160,
        tile: 160,
    };
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: SKIFF_PARKED_FIRST,
        tile: SKIFF_PARKED_FIRST,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(state.board_vehicle(), MoveOutcome::Blocked);
    assert_eq!(state.message, "On foot!");
}

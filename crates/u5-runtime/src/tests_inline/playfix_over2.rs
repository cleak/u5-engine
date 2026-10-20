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

/// F23 follow-up: the charge paid at prompt close can start a fight, and
/// the exchange's closing reply must not come back after the conflict
/// banner. The world pass joins the slot onto the engagement's text, so a
/// reply left there was reprinted as `Status: Done *** CONFLICT ***` (and
/// `Item: Done *** CONFLICT ***` for R-Ready) on a row under the banner.
#[test]
fn playfix_f23_closing_reply_is_not_rejoined_onto_the_conflict_banner() {
    for (key, closing) in [('R', "\u{1b}"), ('Z', "\r\u{1b}")] {
        let dir = debug_game_dir();
        fs::write(
            dir.join(BRIT_CBT_FILE),
            synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
        )
        .unwrap();
        let mut state = world_state(open_world_grid(), 5, 5);
        state.active_objects.push(ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        });
        handle_play_key_input(&mut state, key, "", &dir).unwrap();
        assert!(!state.combat_active, "{key}: no fight while the prompt waits");
        for ch in closing.chars() {
            handle_play_key_input(&mut state, ch, "", &dir).unwrap();
        }
        assert!(state.combat_active, "{key}: the paid turn starts the fight");
        state.flush_message_slot();
        let rows: Vec<&str> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        let banner = combat_banner_line();
        assert_eq!(
            rows.iter().filter(|row| row.contains("CONFLICT")).count(),
            1,
            "{key}: {rows:?}"
        );
        // Nothing of the closing reply comes back after the banner. Since
        // F35 (`combat.md §8.1`) the first actor's turn banner follows it,
        // and that is all that does.
        let conflict = rows
            .iter()
            .position(|row| *row == banner.as_str())
            .unwrap_or_else(|| panic!("{key}: {rows:?}"));
        let after: Vec<&str> = rows[conflict + 1..]
            .iter()
            .copied()
            .filter(|row| !row.is_empty())
            .collect();
        assert_eq!(after.len(), 1, "{key}: {rows:?}");
        assert!(after[0].contains(", armed with "), "{key}: {rows:?}");
        let _ = fs::remove_dir_all(dir);
    }
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
/// \"On foot\" refusal", which §4.1 (issue #299 Q25) publishes as Board's
/// own `\nOn foot\n` - not the `On_foot!\n` of `commands.md §5.5`.
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
    // `vehicles.md §4.1` (issue #299 Q25): Board's own `\nOn foot\n`, with
    // no `!`.
    assert_eq!(state.message, BOARD_ON_FOOT_REFUSAL);
}

/// `overworld.md §9.2` (issue #299 Q29): the midnight route "never changes
/// the party's coordinates, plane or transport. The party is left on the
/// overworld on the former gate cell ... still mounted or airborne if it
/// was", and the gate cannot rise again while the presentation runs.
#[test]
fn midnight_moongate_leaves_a_mounted_party_on_the_former_gate_cell() {
    let dir = debug_game_dir();
    let origin_idx = world_cell_index(5, 5);
    let mut grid = open_world_grid();
    grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
    let mut state = britannia_state(grid, 5, 5);
    let horse = TransportState::Horse {
        type_byte: HORSE_MOUNTED_FIRST,
        tile: FIRST_PLAYABLE_HORSE_TILE,
    };
    state.player.transport = horse;
    state.sync_player_object();
    state.clock = GameClock::new(0, 2).unwrap();
    state.set_cached_moon_glyph_slots(1, 0);
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 6,
        y: 7,
        z: WorldPlane::Britannia.save_floor() as u8,
    };

    run_overworld_loop_head_hooks(&mut state, &dir).unwrap();

    assert!(state.active_shrine.is_some());
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia
        }
    );
    assert_eq!((state.player.x, state.player.y), (5, 5));
    assert_eq!(state.player.transport, horse);
    assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
    assert!(!state.natural_moongate_live_cells.contains(&origin_idx));
    let _ = fs::remove_dir_all(dir);
}

/// F22: `overworld.md §9.2` - in hour `0` with the minute below `10` the
/// live-gate hook "reports success to the outer loop; the outer loop then
/// dispatches the same shrine/urn kneel overlay used by E-Enter at a
/// shrine", and `karma.md §7` resolves a position that matches no mapped
/// shrine to Spirituality. The engine used to require a shrine tile under
/// the gate cell, so the dispatch was dropped silently.
#[test]
fn playfix_f22_midnight_moongate_dispatches_the_shrine_kneel_overlay() {
    let dir = debug_game_dir();
    let origin_idx = world_cell_index(5, 5);
    let mut grid = open_world_grid();
    grid[origin_idx] = NATURAL_MOONGATE_TERRAIN_TILE;
    let mut state = britannia_state(grid, 5, 5);
    state.clock = GameClock::new(0, 2).unwrap();
    state.set_cached_moon_glyph_slots(1, 0);
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 6,
        y: 7,
        z: WorldPlane::Britannia.save_floor() as u8,
    };

    run_overworld_loop_head_hooks(&mut state, &dir).unwrap();

    assert_eq!(state.grid[origin_idx], NATURAL_MOONGATE_RESTORED_TERRAIN_TILE);
    assert_eq!((state.player.x, state.player.y), (5, 5), "no warp at midnight");
    let session = state.active_shrine.as_ref().expect("the kneel overlay runs");
    assert_eq!(session.virtue, ShrineVirtue::Spirituality);
    let _ = fs::remove_dir_all(dir);
}

// Regressions for the overworld exploratory play-test findings (E21..E31).
// Each test names the finding it reproduces and the published sentence it
// rests on.

/// Board the parked object `object_byte` from the party's own cell, then
/// X-it and return the record left behind.
fn playfix_board_then_exit(object_byte: u8, aux1: u8, aux3: u8) -> ActiveObject {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: object_byte,
        tile: object_byte,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1,
        aux3,
    });
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded, "{object_byte:#04x}");
    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle, "{object_byte:#04x}");
    *state
        .active_objects
        .iter()
        .skip(1)
        .find(|object| !object.is_empty())
        .expect("X-it parks a stand-in object")
}

/// E21: `vehicles.md §2` names `0x10/0x11`, `0x1B`, `0x24..0x27` and
/// `0x28..0x2B` as "the objects boarding consumes and exiting leaves
/// behind", and byte 1 of the record is the displayed-frame (actor) byte
/// (`active-objects.md`). The parked record must carry that object byte,
/// not the engine's internal visual index (which drew a frigate as a
/// chest and a horse as a slime).
#[test]
fn playfix_e21_xit_parks_vehicle_with_its_own_display_byte() {
    for (object_byte, parked_byte) in [
        (HORSE_PARKED_FIRST, HORSE_PARKED_FIRST),
        (HORSE_PARKED_FIRST + 1, HORSE_PARKED_FIRST + 1),
        (CARPET_PARKED, CARPET_PARKED),
        (SHIP_PARKED_FIRST + 2, SHIP_PARKED_FIRST + 2),
        (SKIFF_PARKED_FIRST + 1, SKIFF_PARKED_FIRST + 1),
    ] {
        let parked = playfix_board_then_exit(object_byte, 50, 1);
        assert_eq!(parked.type_byte, parked_byte, "{object_byte:#04x}");
        assert_eq!(parked.tile, parked_byte, "{object_byte:#04x} display byte");
        assert_eq!(
            actor_tile_for_byte(parked.tile),
            Some(ACTOR_TILE_BANK_BASE + parked_byte as usize)
        );
    }
}

/// E21: the furled-ship exit into a carried skiff parks the hull as the
/// furled-frigate object too.
#[test]
fn playfix_e21_ship_exit_into_skiff_parks_furled_frigate_art() {
    // All-water grid: no landing support, so the carried skiff launches.
    let mut state = world_state(vec![1; WORLD_CELLS], 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: SHIP_PARKED_FIRST,
        tile: SHIP_PARKED_FIRST,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 77,
        aux3: 2,
    });
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);
    assert!(matches!(state.player.transport, TransportState::Skiff { .. }));
    let parked = state
        .active_objects
        .iter()
        .skip(1)
        .find(|object| !object.is_empty())
        .copied()
        .unwrap();
    assert_eq!(parked.type_byte, SHIP_PARKED_FIRST);
    assert_eq!(parked.tile, SHIP_PARKED_FIRST);
    assert_eq!((parked.aux1, parked.aux3), (77, 1));
}

/// E22: `vehicles.md §4` B-Board probes only the party's own cell and §5
/// says "a vehicle left behind must remain boardable through active-object
/// persistence". The party must be able to step onto a parked vehicle's
/// cell - including the water under a frigate - and then board it.
#[test]
fn playfix_e22_party_steps_onto_parked_vehicles_and_boards_them() {
    for (object_byte, under) in [
        (SHIP_PARKED_FIRST, BRIT_DEEP_WATER_TILE),
        (SKIFF_PARKED_FIRST + 3, BRIT_DEEP_WATER_TILE),
        (HORSE_PARKED_FIRST, 5),
        (CARPET_PARKED, 5),
    ] {
        let mut grid = open_world_grid();
        grid[world_cell_index(6, 5)] = under;
        let mut state = world_state(grid, 5, 5);
        state.active_objects.push(ActiveObject {
            type_byte: object_byte,
            tile: object_byte,
            x: 6,
            y: 5,
            z: WorldPlane::Underworld.save_floor(),
            phase: STEADY_PHASE,
            aux1: 60,
            aux3: 1,
        });
        assert_eq!(state.step(Direction::East), MoveOutcome::Moved, "{object_byte:#04x}");
        assert_eq!((state.player.x, state.player.y), (6, 5));
        assert_eq!(state.board_vehicle(), MoveOutcome::Boarded, "{object_byte:#04x}");
    }
}

/// E22: the round trip of the finding - park a frigate with X-it, step off
/// in its skiff, step back onto it and board it again.
#[test]
fn playfix_e22_frigate_left_by_skiff_can_be_reboarded() {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: SHIP_PARKED_FIRST,
        tile: SHIP_PARKED_FIRST,
        x: 5,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 77,
        aux3: 2,
    });
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert_eq!(state.exit_vehicle(), MoveOutcome::ExitedVehicle);
    assert_eq!(state.step(Direction::South), MoveOutcome::Moved);
    assert_eq!(state.step(Direction::North), MoveOutcome::Moved);
    assert_eq!(state.board_vehicle(), MoveOutcome::Boarded);
    assert!(matches!(
        state.player.transport,
        TransportState::Ship { hull: 77, skiffs: 1, .. }
    ));
}

/// E22: a parked vehicle the current transport cannot board still blocks
/// (a mounted party cannot board a second horse, `vehicles.md §4`).
#[test]
fn playfix_e22_unboardable_parked_vehicle_still_blocks() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.player.transport =
        transport_from_vehicle_object(HORSE_PARKED_FIRST, HORSE_PARKED_FIRST, 0, 0).unwrap();
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(state.step(Direction::East), MoveOutcome::Blocked);
    assert_eq!((state.player.x, state.player.y), (5, 5));
}

/// E23: `blackthorn.md §7` "Party restoration": per in-party slot the
/// cinematic revives, copies maximum into current hit points "and repaints
/// the whole stats panel, before moving to the next slot". The panel must
/// not keep the dead roster once the rescue has restored the party.
#[test]
fn playfix_e23_rescue_restoration_repaints_the_stats_panel() {
    let mut state = test_state(open_grid(), 1, 1);
    for member in &mut state.party {
        member.hp = 0;
        member.status = CharacterStatus::Dead.save_byte();
    }
    state.repaint_stats_panel();
    assert_eq!(
        state.displayed_stats_panel().statuses[0],
        Some(CharacterStatus::Dead.save_byte())
    );
    state.pending_blackthorn_rescue =
        Some(crate::blackthorn::BlackthornRescuePhase::AwaitingAcknowledgement);
    let game_dir = std::path::Path::new("/nonexistent");
    assert_eq!(state.step_blackthorn_rescue(game_dir, true).unwrap(), None);
    state.flush_staged_narration();

    let good = CharacterStatus::Good.save_byte();
    assert_eq!(state.party[0].status, good);
    assert_eq!(state.displayed_stats_panel().statuses[0], Some(good));
    assert_eq!(
        state.displayed_stats_panel(),
        &crate::stats_panel::stats_panel_snapshot_of(&state)
    );
}

/// E24: `overworld.md §9`: when the refresh stamps `0xDC`, "Any actual tile
/// change marks the viewport dirty and refreshes local light", and §9.1 draws
/// the cell by its counter, not by whether the party has moved. The stamp
/// must reach the live chunk window the viewport and light pass read.
#[test]
fn playfix_e24_gate_rising_while_party_stands_still_reaches_live_window() {
    let mut state = britannia_state(open_world_grid(), 4, 5);
    state.clock = GameClock::new(21, 0).unwrap();
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 5,
        y: 5,
        z: WorldPlane::Britannia.save_floor() as u8,
    };

    assert!(state.refresh_natural_moongates());

    assert_eq!(state.grid[world_cell_index(5, 5)], NATURAL_MOONGATE_TERRAIN_TILE);
    assert_eq!(state.world_live_tile_at(5, 5), NATURAL_MOONGATE_TERRAIN_TILE);
    assert_eq!(
        state.world_live_chunks.as_ref().unwrap().tile_at(5, 5),
        NATURAL_MOONGATE_TERRAIN_TILE
    );
}

fn playfix_gate_state(party_x: usize) -> PlayState {
    let mut grid = open_world_grid();
    grid[world_cell_index(5, 5)] = NATURAL_MOONGATE_TERRAIN_TILE;
    let mut state = britannia_state(grid, party_x, 5);
    state.clock = GameClock::new(11, 0).unwrap();
    state.natural_moongate_counter = MOONGATE_PHASE_FULL;
    state.set_cached_moon_glyph_slots(1, 0);
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 6,
        y: 7,
        z: WorldPlane::Britannia.save_floor() as u8,
    };
    state
}

/// E25: `overworld.md §9.2`: the live-gate hook "runs before normal input
/// dispatch, on every iteration" and runs the transit "before the party is
/// relocated and before any key is read". A step onto `0xDC` transits at
/// the end of that step's iteration, not on the next key.
#[test]
fn playfix_e25_step_onto_a_live_gate_transits_without_waiting_for_a_key() {
    let dir = debug_game_dir();
    let mut state = playfix_gate_state(4);
    let turn = state.turn;

    handle_play_key_input(&mut state, '\u{2}', "", &dir).unwrap();

    assert_eq!((state.player.x, state.player.y), (6, 7), "transited after the step");
    assert_eq!(state.turn, turn + 1, "the step's own turn only");
    let _ = fs::remove_dir_all(dir);
}

/// E25: a party already standing on a live gate when a key arrives transits
/// first, and the key is then the next iteration's command - not eaten.
#[test]
fn playfix_e25_key_read_on_a_live_gate_is_dispatched_after_the_transit() {
    let dir = debug_game_dir();
    let mut state = playfix_gate_state(5);
    let turn = state.turn;

    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();

    assert_eq!((state.player.x, state.player.y), (6, 7));
    assert_eq!(state.turn, turn + 1, "the Pass ran after the transit");
    let _ = fs::remove_dir_all(dir);
}

/// E25: the transit and the warp print nothing and clear nothing; the
/// message window still holds what the step left there.
#[test]
fn playfix_e25_gate_transit_keeps_the_message_window() {
    let dir = debug_game_dir();
    let mut state = playfix_gate_state(4);
    state.emit_message_line("Earlier line\n");
    state.message_row_open_mid_line = false;
    let before = state.message_transcript.len();

    handle_play_key_input(&mut state, '\u{2}', "", &dir).unwrap();

    assert_eq!((state.player.x, state.player.y), (6, 7));
    assert!(state.message_transcript.len() >= before);
    assert!(
        state
            .message_transcript
            .iter()
            .any(|entry| entry.text.contains("Earlier line"))
    );
    let _ = fs::remove_dir_all(dir);
}

/// E26: `weather.md §5`: "A later Pass command reports the stalled-sailing
/// feedback `Sheets in irons!\n` and clears the cached sailing state. That
/// Pass adds no ordinary `Pass` echo" (`vehicles.md §6` likewise).
#[test]
fn playfix_e26_sheets_in_irons_pass_prints_no_pass_echo() {
    let dir = debug_game_dir();
    let mut state = britannia_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_HOISTED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: true,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();
    state.sail_cached_direction = Some(Direction::North);
    let before = state.message_transcript.len();

    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();

    let rows: Vec<&str> = state.message_transcript[before..]
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    assert!(state.sail_cached_direction.is_none());
    assert!(rows.iter().any(|row| row.contains(SHIP_SAIL_STALLED_LINE)), "{rows:?}");
    assert!(!rows.iter().any(|row| row.contains("Pass")), "{rows:?}");
    let stalled = state.message_transcript[before..]
        .iter()
        .find(|entry| entry.text.contains(SHIP_SAIL_STALLED_LINE))
        .unwrap();
    assert!(stalled.is_command_echo, "the stalled line sits on the command row");

    // The next Pass, with the cache clear, is an ordinary Pass again.
    let before = state.message_transcript.len();
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert!(
        state.message_transcript[before..]
            .iter()
            .any(|entry| entry.text.contains("Pass"))
    );
    let _ = fs::remove_dir_all(dir);
}

/// E27: `animation.md §13.5` lists "the resident ship-repair hole-up" among
/// the nine walker call sites and says "every one of the nine is a
/// turn-consuming path that also advances the clock".
#[test]
fn playfix_e27_ship_repair_hole_up_consumes_a_turn_and_time() {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 5, 5);
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.sync_player_object();
    state.clock = GameClock::new(12, 0).unwrap();
    let turn = state.turn;

    assert_eq!(state.start_rest_prompt(), MoveOutcome::Rested);

    assert_eq!(state.turn, turn + 1);
    assert_ne!(state.clock, GameClock::new(12, 0).unwrap());
    assert!(matches!(state.player.transport, TransportState::Ship { hull, .. } if hull > 77));
}

/// E28: `overworld.md §6.2.1`: the pirate's announcement is `* BOOOM! *`
/// and a blank line; the player's broadside prints "no narration line"
/// (§6.2.4) and no spec text prints the verb echo a second time.
#[test]
fn playfix_e28_pirate_reply_does_not_reprint_the_fire_echo() {
    let dir = debug_game_dir();
    let mut state = britannia_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 100, 128);
    state.player.transport = TransportState::Ship {
        type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: 77,
        skiffs: 2,
    };
    state.player.facing = Direction::North;
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 0x2C,
        tile: 0x2C,
        x: 102,
        y: 128,
        z: WorldPlane::Britannia.save_floor(),
        phase: 0x00,
        aux1: 0x1e,
        aux3: 0,
    });
    let before = state.message_transcript.len();

    handle_play_key_input(&mut state, 'F', "", &dir).unwrap();
    handle_play_key_input(&mut state, '\u{2}', "", &dir).unwrap();

    let rows: Vec<String> = state.message_transcript[before..]
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    assert!(
        rows.iter().any(|row| row.contains("BOOOM")),
        "the fixture must draw the pirate's reply: {rows:?}"
    );
    assert_eq!(
        rows.iter().filter(|row| row.contains("Fire")).count(),
        1,
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

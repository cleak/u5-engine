// Byte-level save-fidelity conformance. Every expectation here was
// measured against the DOS build under DOSBox with the same input
// SAVED.GAM: load it through Journey Onward, take the stated number of
// turns, Q-save, and diff the file.

fn save_fidelity_game_dir(template: Vec<u8>) -> std::path::PathBuf {
    let dir = debug_game_dir();
    fs::write(dir.join("SAVED.GAM"), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);
    dir
}

/// `time.md §11` tabulates `0x02DE` as the "twelve-hour display value
/// recomputed on hour changes" (the rule itself is `time.md §2`), and the
/// same section says what a save writer owes it: "Only `0x02CE`, `0x02D7`,
/// `0x02D8`, `0x02D9`, and `0x02DB` are the canonical calendar fields. The
/// derived and adjacent bytes are still persistent engine state, so
/// compatibility implementations should round-trip them rather than
/// regenerating the whole span from the calendar alone." The byte therefore
/// round-trips out of the template rather than being derived from the live
/// clock at save time. The DOS build agrees: it left the byte at zero after
/// no turns and after four turns that carried 08:59 to 09:03 across an hour
/// boundary.
#[test]
fn save_round_trips_the_twelve_hour_display_byte_instead_of_deriving_it() {
    let mut template = saved_game_seed_bytes(17, 0, 15, 15);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    template[SAVE_AMPM_DISPLAY_OFFSET] = 0x5a;
    let dir = save_fidelity_game_dir(template);

    let mut state = world_state(open_world_grid(), 10, 20);
    state.clock = GameClock::with_date(140, 13, 28, 18, 45).unwrap();
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(saved[SAVE_HOUR_OFFSET], 18);
    assert_eq!(saved[SAVE_AMPM_DISPLAY_OFFSET], 0x5a);
    assert_ne!(saved[SAVE_AMPM_DISPLAY_OFFSET], state.clock.display_hour());
}

/// `formats/saved-gam.md §5`: `0x02DA` is the "saved-hour snapshot ...
/// used by the time cleanup to detect hour crossings", and `time.md §2`
/// has it "taken at the start of every cleanup pass". Four one-minute
/// turns from 08:59 left the original with `0x02DA` = 9, matching the
/// hour the last cleanup pass started in.
#[test]
fn save_writes_the_cleanup_pre_cascade_hour_snapshot() {
    let mut template = saved_game_seed_bytes(17, 0, 15, 15);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    let dir = save_fidelity_game_dir(template);

    let mut state = world_state(open_world_grid(), 10, 20);
    state.clock = GameClock::with_date(139, 4, 5, 8, 59).unwrap();
    state.cleanup_previous_hour = 0;

    state.advance_turn();
    assert_eq!(state.cleanup_previous_hour, 8);
    assert_eq!(state.clock.hour, 9);
    state.advance_turn();
    assert_eq!(state.cleanup_previous_hour, 9);

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(saved[SAVE_SAVED_HOUR_SNAPSHOT_OFFSET], 9);
}

/// Runtime observation; the spec publishes the field but not the value.
/// `active-objects.md §5` scopes the per-frame refresh of the player's
/// record to "bytes 0..4 of slot zero", which does not reach byte `+0x06`,
/// and the DOS build leaves that byte at zero: a no-turn load and save of
/// the shipped file wrote `1C 1C 0F 0F 00 00 00 00` into the first record.
/// This engine used to stamp the steady marker there. See
/// `PLAYER_ACTIVE_OBJECT_PHASE` for the readers that skip slot zero.
#[test]
fn player_active_object_record_saves_a_zero_phase_byte() {
    let mut template = saved_game_seed_bytes(17, 0, 15, 15);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    let dir = save_fidelity_game_dir(template);

    let mut state = world_state(open_world_grid(), 10, 20);
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(saved[SAVE_ACTIVE_OBJECTS_OFFSET + 2], 10);
    assert_eq!(saved[SAVE_ACTIVE_OBJECTS_OFFSET + 3], 20);
    assert_eq!(saved[SAVE_ACTIVE_OBJECTS_OFFSET + 6], 0);
}

/// `doors-and-z-transitions.md §5`: the countdown starts at four and the
/// door re-closes when it "hits zero". Runtime observation, spec silent
/// on what happens next: the DOS build keeps the four-byte block. After
/// one Open and fourteen turns the original saved `0x03A9..0x03AC` as
/// `B8 0F 13 F6` — previous tile, X and Y intact and the countdown at
/// `4 - 14 = -10` — so the counter keeps running and only the single
/// zero crossing closes the door.
#[test]
fn door_auto_close_keeps_its_block_and_runs_the_countdown_past_zero() {
    let mut state = test_state(open_grid(), 5, 5);
    let closed_tile = 0xb8;
    state.grid[3 * 32 + 15] = TOWN_DOOR_CLEARED_TILE;
    state.door_tracker = Some(DoorTracker {
        previous_tile: closed_tile,
        x: 15,
        y: 3,
        turns_remaining: DOOR_AUTO_CLOSE_TURNS,
    });
    state.door_tracker_closed = false;

    for _ in 0..DOOR_AUTO_CLOSE_TURNS {
        state.tick_door_tracker();
    }
    assert_eq!(state.grid[3 * 32 + 15], closed_tile, "the door re-closes");
    let tracker = state.door_tracker.expect("the block is not cleared");
    assert_eq!(tracker.previous_tile, closed_tile);
    assert_eq!(tracker.x, 15);
    assert_eq!(tracker.y, 3);
    assert_eq!(tracker.turns_remaining, 0);

    // Ten more turns: 4 - 14 = -10 = 0xF6, the value the original saved.
    for _ in 0..10 {
        state.tick_door_tracker();
    }
    let tracker = state.door_tracker.expect("the block is still resident");
    assert_eq!(tracker.turns_remaining, 0xf6);
    assert_eq!(tracker.previous_tile, closed_tile);
    assert_eq!(state.grid[3 * 32 + 15], closed_tile);
}

/// A loaded save disarms the tracker by clearing only the previous-tile
/// byte (`doors-and-z-transitions.md §5`), so a retained block from an
/// earlier visit must not tick or re-close anything.
#[test]
fn loaded_door_block_with_a_cleared_previous_tile_stays_inert() {
    let mut state = test_state(open_grid(), 5, 5);
    state.door_tracker = Some(DoorTracker {
        previous_tile: 0,
        x: 15,
        y: 3,
        turns_remaining: 0xf6,
    });
    for _ in 0..5 {
        state.tick_door_tracker();
    }
    let tracker = state.door_tracker.expect("inert block round-trips");
    assert_eq!(tracker.turns_remaining, 0xf6);
    assert_eq!(state.grid[3 * 32 + 15], open_grid()[3 * 32 + 15]);
}

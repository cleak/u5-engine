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

/// `formats/saved-gam.md §5` / `time.md §11` (spec `0170809`): `0x02DE`
/// is "written with the twelve-hour form of the hour when the cleanup
/// finds the snapshot at `0x02DA` disagreeing with the hour at `0x02D9`,
/// then counted down toward zero by the ambient-audio tick", and "the
/// byte-compatible behaviour is: write on a snapshot mismatch, then decay
/// on the audio cadence above".
///
/// This replaces a test that pinned the withdrawn "12-hour display value
/// recomputed on hour changes" wording and the round-trip-from-template
/// behaviour it justified (`RETRACTIONS.md` R338). The old test's
/// evidence survives as the decay half: the DOS build left the byte at
/// zero after no turns and after four turns that carried 08:59 to 09:03
/// across an hour boundary, which is what write-then-decay produces once
/// any idle world ticks have run.
#[test]
fn twelve_hour_byte_is_written_on_a_snapshot_mismatch_and_then_decays() {
    let mut template = saved_game_seed_bytes(17, 0, 15, 15);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    template[SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET] = 0x5a;
    let dir = save_fidelity_game_dir(template);

    let mut state = world_state(open_world_grid(), 10, 20);
    // A restored save's byte is live state, not a value to rederive.
    state.twelve_hour_audio_repeats = 0x5a;
    state.clock = GameClock::with_date(139, 4, 5, 18, 59).unwrap();
    state.cleanup_previous_hour = 18;

    // No hour crossing: the snapshot still agrees, so nothing is written.
    assert_eq!(state.twelve_hour_audio_repeats, 0x5a);

    state.advance_turn();
    assert_eq!(state.clock.hour, 19);
    assert_eq!(
        state.twelve_hour_audio_repeats,
        display_hour_12h(19),
        "the crossing writes the twelve-hour form of the new hour"
    );

    // The ambient-audio tick is the byte's only consumer, it "runs once
    // per idle world tick", and it takes the byte down two calls in every
    // eight. `display_hour_12h(19)` is 7, so four periods of eight world
    // steps are more than enough; drive them through the production step
    // rather than the helper.
    for _ in 0..(4 * usize::from(AMBIENT_AUDIO_SUB_TICK_PERIOD)) {
        let _ = state.advance_visual_tick();
    }
    assert_eq!(state.twelve_hour_audio_repeats, 0);

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(saved[SAVE_HOUR_OFFSET], 19);
    assert_eq!(saved[SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET], 0);
    let _ = fs::remove_dir_all(dir);
}

/// `time.md §11`: the decrement runs "on **two of every eight** of its
/// own calls". The identity of the two residues is not published; the
/// rate is.
#[test]
fn ambient_audio_tick_decrements_two_calls_in_every_eight() {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.twelve_hour_audio_repeats = 200;
    for _ in 0..(AMBIENT_AUDIO_SUB_TICK_PERIOD * 4) {
        state.tick_ambient_audio_repeats();
    }
    assert_eq!(state.twelve_hour_audio_repeats, 200 - 4 * 2);
}

/// `animation.md §13.1`, the Negate Time freeze: "For the effect's full
/// duration nothing advances: no water rotation, no fire flicker, no
/// fountain, no banner, no clock or bellows, no object animation, no AI
/// roll, no wind check, no moongate refresh, no beacon step, and **no
/// shrine/lava ambience tick**." `§13.2` identifies that ambience tick as
/// the pass that "on two of every eight enabled steps, decrements a
/// shared countdown byte", which `time.md §11` owns as save byte
/// `0x02DE`. So a world tick taken while Negate Time runs must leave the
/// byte exactly where it stood.
#[test]
fn negate_time_freezes_the_ambient_audio_decay_of_the_twelve_hour_byte() {
    let mut frozen = world_state(open_world_grid(), 10, 20);
    frozen.twelve_hour_audio_repeats = 200;
    frozen.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    frozen.active_effect_counter = 40;
    assert!(frozen.negate_time_active());
    for _ in 0..(AMBIENT_AUDIO_SUB_TICK_PERIOD * 4) {
        let _ = frozen.advance_visual_tick();
    }
    assert_eq!(
        frozen.twelve_hour_audio_repeats, 200,
        "Negate Time freezes the ambience tick, so `0x02DE` does not decay"
    );

    // The same tick count with the effect clear does decay it, at the
    // published two-in-eight rate.
    let mut running = world_state(open_world_grid(), 10, 20);
    running.twelve_hour_audio_repeats = 200;
    assert!(!running.negate_time_active());
    for _ in 0..(AMBIENT_AUDIO_SUB_TICK_PERIOD * 4) {
        let _ = running.advance_visual_tick();
    }
    assert_eq!(running.twelve_hour_audio_repeats, 200 - 4 * 2);
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

/// `formats/saved-gam.md §8.1` (spec `0170809`): "the player's own record
/// carries **zero** in this byte in a shipped save, not the all-ones
/// freeze marker", and `RETRACTIONS.md` R340 adds that "an engine that
/// writes the sentinel there diverges on every save". The byte is an
/// animation-script step plus a frame-delay countdown, never a facing.
///
/// This test previously carried the value as an unbacked runtime
/// observation ("the spec publishes the field but not the value"); the
/// value is now published, and the DOS build's `1C 1C 0F 0F 00 00 00 00`
/// slot-zero record agrees with it.
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
    assert_eq!(saved[SAVE_ACTIVE_OBJECTS_OFFSET + 6], PLAYER_ACTIVE_OBJECT_PHASE);
    assert_ne!(saved[SAVE_ACTIVE_OBJECTS_OFFSET + 6], STEADY_PHASE);
    let _ = fs::remove_dir_all(dir);
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


/// Iolo's Hut is scene 13 - `DWELLING` block 4 - and the shipped save
/// starts the party there at 08:35 on day five. The fixture reproduces
/// that layout with a populated `.NPC` roster so a *fresh* entry would
/// seat a cast, which is what makes the preserving-entry assertions below
/// mean something.
fn shipped_layout_iolos_hut_dir() -> std::path::PathBuf {
    let dir = debug_game_dir();
    fs::write(dir.join("DWELLING.DAT"), vec![16; 16 * TOWN_GRID_BYTES]).unwrap();
    let mut npc = vec![0u8; NPC_SLOTS_PER_SUB_MAP * NPC_SUB_MAP_LEN];
    let base = SHIPPED_IOLOS_HUT_BLOCK * NPC_SUB_MAP_LEN;
    for slot in 1..5usize {
        let record = base + slot * NPC_SCHEDULE_RECORD_LEN;
        for waypoint in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
            npc[record + NPC_SCHEDULE_X_OFFSET + waypoint] = 10 + slot as u8;
            npc[record + NPC_SCHEDULE_Y_OFFSET + waypoint] = 12;
            npc[record + NPC_SCHEDULE_Z_OFFSET + waypoint] = 0;
        }
        npc[base + NPC_TYPE_ARRAY_OFFSET + slot] = TOWN_NPC_ORDINARY_TYPE_FIRST;
        npc[base + NPC_DIALOG_ARRAY_OFFSET + slot] = slot as u8;
    }
    fs::write(dir.join("DWELLING.NPC"), npc).unwrap();
    fs::write(dir.join("DWELLING.TLK"), [0, 0]).unwrap();
    fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "DWELLING:4 0\n").unwrap();
    dir
}

/// The shipped `SAVED.GAM` control bytes, byte for byte, for the offsets
/// this test cares about. Everything else comes from the ordinary seed
/// fixture; the Avatar name is the one addition, because the pristine
/// shipped file carries nine zero bytes there and the original itself
/// answers "No active game" to Journey Onward on it.
fn shipped_layout_iolos_hut_save() -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(SHIPPED_IOLOS_HUT_SCENE, 0, 15, 15);
    bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
    bytes[SAVE_SAVED_HOUR_SNAPSHOT_OFFSET] = 0;
    bytes[SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET] = 0;
    bytes[SAVE_CACHED_TRAMMEL_GLYPH_OFFSET] = 0;
    bytes[SAVE_CACHED_FELUCCA_GLYPH_OFFSET] = 0;
    bytes[SAVE_WIND_OFFSET] = WindState::Calm.save_byte();
    bytes[SAVE_REDRAW_ENABLE_OFFSET] = 1;
    bytes[SAVE_AMBIENT_LIGHT_OFFSET] = 5;
    bytes[SAVE_RESIDENT_SHADOWLORD_OFFSET] = 0;
    bytes[SAVE_FORTUNES_OF_WAR_OFFSET] = 1;
    bytes
}

const SHIPPED_IOLOS_HUT_SCENE: u8 = 13;
const SHIPPED_IOLOS_HUT_BLOCK: usize = 4;

/// The whole of `cleak/u5-spec#184` in one diff: load the shipped-layout
/// save through Journey Onward, Q-save immediately, and compare every
/// offset the answer settles against the bytes the original writes.
///
/// * `0x02DE` - `time.md §5`: the shipped seed's snapshot is zero against
///   a start hour of eight, so "a stale snapshot fires the bundle once at
///   scene entry, with no turn consumed" and the byte takes the
///   twelve-hour form of hour eight - the literal `8`. This test takes the
///   Q-save with no world tick in between, so `8` is what the shipped path
///   writes, and that is what is asserted. The DOS reference file holds
///   `0x00` because `§11`'s decay had run: "a save taken any appreciable
///   time after the last hour crossing reads zero here". Reaching that
///   zero needs idle world ticks, and it is driven through the production
///   tick - not a helper - in
///   [`twelve_hour_byte_reaches_the_original_zero_over_shipped_idle_world_ticks`].
/// * `0x02DF`/`0x02E0` - `formats/saved-gam.md §5.1`: the cached Trammel
///   and Felucca digits for the day of the month, "stored as the printable
///   character for a digit". Day five of the shipped start date gives
///   `'2'`/`'3'`, deterministically.
/// * `0x02EC` - `weather.md §9`: "**Neither is recomputed, normalised or
///   rerolled by the load path or by scene entry**: a load restores
///   whatever the file held, and the banner that follows is a reprint."
/// * `0x02FF` - `formats/saved-gam.md §10`: `50` at 08:35 on floor zero of
///   a lit location; the seed's `5` "is a stale sample the first clock call
///   overwrites".
/// * `0x03B2` - `formats/saved-gam.md §10` / `town-mode.md §5` step 6:
///   town-family entry stamps the no-host marker `0xFF` unconditionally.
///   "The factory seed's `0` is a stale, semantically wrong value."
/// * slot zero's `+6` - `formats/saved-gam.md §8.1`: zero, not the freeze
///   sentinel. The whole record is `1C 1C 0F 0F 00 00 00 00`.
/// * slots one through thirty-one - `RETRACTIONS.md` R341: Journey Onward
///   passes the preserving entry mode, which suppresses the roster load and
///   the reseat, so this hut resumes with slot zero only.
#[test]
fn shipped_layout_town_save_round_trips_every_answered_offset() {
    let dir = shipped_layout_iolos_hut_dir();
    fs::write(dir.join("SAVED.GAM"), shipped_layout_iolos_hut_save()).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let options = load_play_options_from_save(&dir).unwrap();
    let scene = Scene::new(SHIPPED_IOLOS_HUT_SCENE).unwrap();
    let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();

    // R341: "the per-tick NPC walker skips every roster slot whose type
    // byte is zero and the type array was never loaded", so the four
    // roster NPCs this location seats on a walk-in are not part of a
    // resumed cast.
    assert!(
        state.npcs.is_empty(),
        "a preserving entry seats no NPC the save image did not carry"
    );
    assert_eq!(state.ambient_light, FULL_DAYLIGHT);

    // `time.md §5`: the entry's mode-zero call compares a snapshot it does
    // not refresh, so the bundle fires once here and writes the twelve-hour
    // form of hour eight. Nothing else runs before the save below, so this
    // is the value the shipped writer flushes.
    assert_eq!(state.twelve_hour_audio_repeats, 8);

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();

    // The entry write, unattenuated: no world tick has run, so the decay
    // that takes the DOS reference file to `0x00` has not started. See the
    // doc comment and the companion decay test.
    assert_eq!(saved[SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET], 8);
    assert_eq!(saved[SAVE_CACHED_TRAMMEL_GLYPH_OFFSET], 0x32);
    assert_eq!(saved[SAVE_CACHED_FELUCCA_GLYPH_OFFSET], 0x33);
    assert_eq!(saved[SAVE_WIND_OFFSET], WindState::Calm.save_byte());
    assert_eq!(saved[SAVE_AMBIENT_LIGHT_OFFSET], 50);
    assert_eq!(
        saved[SAVE_RESIDENT_SHADOWLORD_OFFSET],
        SAVE_RESIDENT_SHADOWLORD_NONE
    );
    assert_eq!(
        &saved[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_RECORD_LEN],
        &[0x1c, 0x1c, 0x0f, 0x0f, 0x00, 0x00, 0x00, 0x00],
        "slot zero is the shipped player record"
    );
    assert!(
        saved[SAVE_ACTIVE_OBJECTS_OFFSET + OOL_RECORD_LEN
            ..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN]
            .iter()
            .all(|byte| *byte == 0),
        "no linked-NPC cast record is written in this town-family save"
    );
    // Two bytes the answer says a producer must leave alone.
    assert_eq!(saved[SAVE_SAVED_HOUR_SNAPSHOT_OFFSET], 0);
    assert_eq!(saved[SAVE_REDRAW_ENABLE_OFFSET], 1);
    let _ = fs::remove_dir_all(dir);
}

/// The decay half of `0x02DE`, driven end to end through the shipped code
/// path: load the same shipped-layout save, run the production idle world
/// tick, Q-save through the real save writer, and land on the byte the DOS
/// reference file holds.
///
/// `time.md §11`: "The audio tick runs once per idle world tick", and it
/// "decrements it toward zero on **two of every eight** of its own calls".
/// The scene-entry write of the twelve-hour form of hour eight is `8`, so
/// eight decrements - four whole periods of eight world steps - take it to
/// zero, and "a save taken any appreciable time after the last hour
/// crossing reads zero here". That is the measured DOS load-and-save.
///
/// Nothing here calls the ambient helper directly: the only route to the
/// decrement is [`PlayState::advance_visual_tick`], the same world step
/// `u5-bevy` drives, so the whole shipped chain is pinned - entry write,
/// world step, save writer, file bytes. The intermediate assertion is on
/// the published two-in-eight *rate*; which two of the eight sub-ticks
/// carry the decrement is not published and is not asserted.
#[test]
fn twelve_hour_byte_reaches_the_original_zero_over_shipped_idle_world_ticks() {
    let dir = shipped_layout_iolos_hut_dir();
    fs::write(dir.join("SAVED.GAM"), shipped_layout_iolos_hut_save()).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let options = load_play_options_from_save(&dir).unwrap();
    let scene = Scene::new(SHIPPED_IOLOS_HUT_SCENE).unwrap();
    let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();
    assert_eq!(state.twelve_hour_audio_repeats, 8);

    let period = usize::from(AMBIENT_AUDIO_SUB_TICK_PERIOD);
    for _ in 0..period {
        let _ = state.advance_visual_tick();
    }
    assert_eq!(
        state.twelve_hour_audio_repeats, 6,
        "one period of eight world steps spends two decrements"
    );

    for _ in 0..(3 * period) {
        let _ = state.advance_visual_tick();
    }
    assert_eq!(
        state.twelve_hour_audio_repeats, 0,
        "four periods of eight world steps spend the whole entry write of 8"
    );

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(
        saved[SAVE_TWELVE_HOUR_AUDIO_REPEAT_OFFSET], 0x00,
        "the DOS load-and-save reference byte, reached through the world step"
    );
    // The world step touches neither of these, so the answered offsets the
    // headline test pins survive the idle time as well.
    assert_eq!(saved[SAVE_CACHED_TRAMMEL_GLYPH_OFFSET], 0x32);
    assert_eq!(saved[SAVE_CACHED_FELUCCA_GLYPH_OFFSET], 0x33);
    assert_eq!(saved[SAVE_SAVED_HOUR_SNAPSHOT_OFFSET], 0);
    let _ = fs::remove_dir_all(dir);
}

/// `weather.md §9` (spec `0170809`): "Save and load are therefore verbatim
/// for wind in both directions", and `§2.1` adds that the load path passes
/// the set-and-repaint helper's print-only sentinel, so "the line the intro
/// prints after a Journey Onward" is a reprint of the restored state. Every
/// preserved direction has to survive a load and an immediate save.
#[test]
fn wind_survives_load_and_save_unrerolled_for_every_direction() {
    for wind_byte in 0..=4u8 {
        let dir = shipped_layout_iolos_hut_dir();
        let mut template = shipped_layout_iolos_hut_save();
        template[SAVE_WIND_OFFSET] = wind_byte;
        fs::write(dir.join("SAVED.GAM"), template).unwrap();
        fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
        write_empty_ool_mirrors(&dir);

        let options = load_play_options_from_save(&dir).unwrap();
        let scene = Scene::new(SHIPPED_IOLOS_HUT_SCENE).unwrap();
        let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();
        assert_eq!(state.wind_save_byte, wind_byte);
        assert_eq!(state.wind, WindState::from_save_byte(wind_byte));

        assert_eq!(
            state.save_game_command(&dir, Some(true)).unwrap(),
            MoveOutcome::Saved
        );
        let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
        assert_eq!(saved[SAVE_WIND_OFFSET], wind_byte);
        let _ = fs::remove_dir_all(dir);
    }
}

/// `formats/saved-gam.md §5.1`: "Natural-moongate transit selects its
/// destination from these two cached bytes and from nothing else", so a
/// restored save has to carry the pair the day of the month gives, not a
/// zeroed scratch pair.
#[test]
fn cached_moon_glyph_digits_reload_and_reselect_the_same_moonstone_slots() {
    let dir = shipped_layout_iolos_hut_dir();
    fs::write(dir.join("SAVED.GAM"), shipped_layout_iolos_hut_save()).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let options = load_play_options_from_save(&dir).unwrap();
    let scene = Scene::new(SHIPPED_IOLOS_HUT_SCENE).unwrap();
    let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();
    assert_eq!(state.cached_moon_glyph_bytes, [b'2', b'3']);
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let reloaded = load_play_options_from_save(&dir).unwrap();
    assert_eq!(reloaded.cached_moon_glyph_bytes, [b'2', b'3']);
    assert_eq!(
        moonstone_slot_from_glyph_byte(reloaded.cached_moon_glyph_bytes[0]),
        trammel_moonstone_slot_for_day(5)
    );
    assert_eq!(
        moonstone_slot_from_glyph_byte(reloaded.cached_moon_glyph_bytes[1]),
        felucca_moonstone_slot_for_day(5)
    );
    let _ = fs::remove_dir_all(dir);
}

/// `formats/saved-gam.md §10`: the resident-Shadowlord latch is written
/// "for any save taken inside a location"; a save taken outside one leaves
/// the template byte alone, because "a save tool should preserve whatever
/// it finds".
#[test]
fn resident_shadowlord_latch_is_only_stamped_inside_a_location() {
    let mut template = saved_game_seed_bytes(0, 0, 10, 20);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    template[SAVE_RESIDENT_SHADOWLORD_OFFSET] = 0x02;
    let dir = save_fidelity_game_dir(template);

    let mut state = world_state(open_world_grid(), 10, 20);
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(saved[SAVE_RESIDENT_SHADOWLORD_OFFSET], 0x02);
    let _ = fs::remove_dir_all(dir);
}

/// `formats/saved-gam.md §10` / `time.md §6`: "a stored value of `51` or
/// higher makes the recompute skip entirely and freezes ambient light for
/// that call", so the byte has to reach the recompute from the save image
/// rather than being reseeded at construction.
#[test]
fn stored_ambient_light_above_the_sentinel_freezes_the_recompute() {
    let dir = shipped_layout_iolos_hut_dir();
    let mut template = shipped_layout_iolos_hut_save();
    template[SAVE_AMBIENT_LIGHT_OFFSET] = 0x33;
    fs::write(dir.join("SAVED.GAM"), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(options.ambient_light, 0x33);
    let scene = Scene::new(SHIPPED_IOLOS_HUT_SCENE).unwrap();
    let mut state = PlayState::load_town_scene(&dir, scene, options).unwrap();
    assert_eq!(
        state.ambient_light, 0x33,
        "the sentinel freezes the recompute"
    );

    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );
    let saved = fs::read(dir.join("SAVED.GAM")).unwrap();
    assert_eq!(saved[SAVE_AMBIENT_LIGHT_OFFSET], 0x33);
    let _ = fs::remove_dir_all(dir);
}

/// `RETRACTIONS.md` R341: a save taken inside a town "already carries its
/// complete live cast", and the preserving entry "restores the cast
/// **exactly as it stood at the save**, mid-route positions and queued
/// paths included - it does not snap NPCs back to their scheduled
/// waypoint".
#[test]
fn preserving_entry_resumes_the_cast_the_save_image_carried() {
    let dir = shipped_layout_iolos_hut_dir();
    let mut template = shipped_layout_iolos_hut_save();
    // Roster slot 2's scheduled waypoint is (12, 12); the save caught it
    // mid-route at (20, 7).
    write_ool_object(
        &mut template[SAVE_ACTIVE_OBJECTS_OFFSET..SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN],
        1,
        ActiveObject {
            type_byte: TOWN_NPC_ORDINARY_TYPE_FIRST,
            tile: TOWN_NPC_ORDINARY_TYPE_FIRST,
            x: 20,
            y: 7,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        },
    );
    fs::write(dir.join("SAVED.GAM"), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let options = load_play_options_from_save(&dir).unwrap();
    let scene = Scene::new(SHIPPED_IOLOS_HUT_SCENE).unwrap();
    let state = PlayState::load_town_scene(&dir, scene, options).unwrap();

    assert_eq!(state.npcs.len(), 1, "one restored record, one live NPC");
    assert_eq!((state.npcs[0].x, state.npcs[0].y), (20, 7));
    assert_eq!(state.npcs[0].active_object, Some(1));
    let _ = fs::remove_dir_all(dir);
}

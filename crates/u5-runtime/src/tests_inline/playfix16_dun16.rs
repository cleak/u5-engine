// Regressions for the sixteenth exploratory playtest's dungeon-room findings
// (P16-04..P16-09). Each test names the finding and the published sentence
// it rests on.

/// A three-member dungeon party standing south of a `0xF7` room cell in
/// scene 35, stepped north into the room so the room fight is running.
fn playfix16_room_fight(dir: &std::path::Path) -> PlayState {
    let scene = DungeonScene::new(35).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 2, 1)] = 0xf7;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.area = Area::Dungeon { scene, level: 0 };
    handle_play_key_input(&mut state, char::from(INPUT_CODE_NORTH), "", dir).unwrap();
    assert!(state.combat_active, "the room fight started");
    state
}

/// P16-04, as settled by u5-spec#308 Q2: `dungeon-mode.md §5` - "The in-place
/// patch, and the room-clear bitmap write that goes with it, are made on the
/// normal return from room combat ...; when room combat returns the loss
/// result (the party side emptied while foes remain) the helper skips both."
/// The patch and the durable bit still agree, as P16-04 asked, but a lost
/// room now keeps neither: the trigger stays `0xF?` and the bit stays clear.
/// (This replaces the earlier expectation that a lost fight sets the bit.)
#[test]
fn playfix16_p16_04_a_lost_room_fight_skips_both_the_patch_and_the_bit() {
    let dir = playfix2_room_game_dir();
    let scene = DungeonScene::new(35).unwrap();
    let mut state = playfix16_room_fight(&dir);
    assert_eq!(
        state.grid[dungeon_cell_index(0, 2, 1)],
        0xf7,
        "the patch waits for the return from room combat"
    );

    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Defeat);

    assert!(!state.combat_active);
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xf7);
    assert!(!dungeon_room_clear_bit_is_set(
        &state.dungeon_room_clear_bitmap,
        scene,
        7
    ));
    let _ = fs::remove_dir_all(dir);
}

/// u5-spec#308 Q2: on the normal return the helper patches the trigger
/// "for every room including the deny-listed rooms", and the bit goes with
/// it (the bitmap writer itself refuses a deny-listed pair).
#[test]
fn spec308_q2_a_won_room_fight_patches_the_trigger_and_sets_the_bit() {
    let dir = playfix2_room_game_dir();
    let scene = DungeonScene::new(35).unwrap();
    let mut state = playfix16_room_fight(&dir);

    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);

    assert!(!state.combat_active);
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa7);
    assert!(dungeon_room_clear_bit_is_set(
        &state.dungeon_room_clear_bitmap,
        scene,
        7
    ));
    let _ = fs::remove_dir_all(dir);
}

/// u5-spec#308 Q2 (`RETRACTIONS.md` R647): the demotion pass "runs once,
/// before the loop's first underfoot check and first command, and does not
/// run again within the visit: K-Klimb ... a level change only changes the
/// party's level index without reloading or rescanning the record". The
/// executed probe "left a destination-level marker at `0xF?`".
#[test]
fn spec308_q2_klimb_does_not_rerun_the_room_clear_demotion() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(37).unwrap();
    let mut grid = open_dungeon_record();
    // `0x2F` is a down ladder (`dungeon-mode.md §13.1`).
    grid[dungeon_cell_index(0, 1, 1)] = 0x2f;
    grid[dungeon_cell_index(1, 3, 3)] = 0xf4;
    let mut state = dungeon_state(grid, 0, 1, 1);
    state.area = Area::Dungeon { scene, level: 0 };
    // Room 4 counts as cleared from here on, as after a won fight elsewhere
    // in this visit.
    assert!(set_dungeon_room_clear_bit(
        &mut state.dungeon_room_clear_bitmap,
        scene,
        4
    ));

    assert!(state.handle_dungeon_key('k', &dir).unwrap());

    assert_eq!(state.area, Area::Dungeon { scene, level: 1 });
    assert_eq!(state.grid[dungeon_cell_index(1, 3, 3)], 0xf4);
    let _ = fs::remove_dir_all(dir);
}

/// P16-05, restated for u5-spec#308 Q2 (`RETRACTIONS.md` R647, R648):
/// "The working dungeon record (eight levels) is part of the save image and a
/// save made in a dungeon restores it, patched room triggers included; the
/// entry demotion pass then runs once" over "all eight levels of the working
/// record". Resuming inside the dungeon keeps the saved working copy, and a
/// cleared room's other `0xF?` cells in it are demoted on every level; an
/// uncleared room's cell and every other byte are kept.
#[test]
fn playfix16_p16_05_a_save_resume_replays_the_room_clear_demotion() {
    let dir = debug_game_dir();
    let scene = DungeonScene::new(37).unwrap();
    let mut buffer = open_dungeon_record();
    buffer[dungeon_cell_index(0, 3, 6)] = 0xf4;
    buffer[dungeon_cell_index(1, 7, 6)] = 0xf4;
    buffer[dungeon_cell_index(0, 5, 5)] = 0xf6;
    buffer[dungeon_cell_index(0, 4, 4)] = 0x68;
    let mut options = PlayOptions {
        target: PlayTarget::Dungeon(scene),
        floor: 0,
        start: Some((2, 6)),
        saved_dungeon_working_buffer: Some(buffer.clone()),
        ..PlayOptions::default()
    };
    assert!(set_dungeon_room_clear_bit(
        &mut options.dungeon_room_clear_bitmap,
        scene,
        4
    ));

    let state = PlayState::load_dungeon_scene(&dir, scene, options).unwrap();

    assert_eq!(state.grid[dungeon_cell_index(0, 3, 6)], 0xa4);
    assert_eq!(state.grid[dungeon_cell_index(1, 7, 6)], 0xa4);
    assert_eq!(state.grid[dungeon_cell_index(0, 5, 5)], 0xf6, "room 6 is not cleared");
    assert_eq!(state.grid[dungeon_cell_index(0, 4, 4)], 0x68, "saved bytes are kept");
    let _ = fs::remove_dir_all(dir);
}

/// P16-06: `formats/cbt.md §5` names special id `0xE8` "Poison field", and
/// `combat.md §11` lists `0xE8` among "the common hook's recognized placed
/// markers"; the Poison row turns a Good party target Poisoned. A party
/// member standing on a room-placed `0xE8` is poisoned by the hook.
#[test]
fn playfix16_p16_06_a_room_placed_poison_field_poisons_a_good_member() {
    let mut state = combat_player_command_state(8, 0);
    state.party[0].status = b'G';
    // The seated Avatar's real linked byte (`combat_party_actor_byte`): the
    // Poison row rejects a linked byte of `0x80` or more, which the generic
    // fixture's stand-in sprite would trip.
    state.active_objects[0].type_byte = combat_party_actor_byte(b'A');
    state.active_objects[0].tile = combat_party_actor_byte(b'A');
    // Plain floor underfoot: the fixture's `0x04` is swamp, whose terrain
    // arm would poison without any marker.
    state.combat_terrain = [[GET_TILE_WALL_TORCH_REPLACEMENT; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.active_objects[9] = ActiveObject {
        type_byte: DUNGEON_ROOM_SPECIAL_SOURCE_E8,
        tile: DUNGEON_ROOM_SPECIAL_SOURCE_E8,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    };

    let application = state.apply_combat_post_dispatch_contact_for_actor_position(0);

    assert!(application.is_some(), "the hook recognized the marker");
    assert_eq!(state.party[0].status, b'P');
}

/// A seated, Good Avatar on plain floor with a room-placed marker `id`
/// under it, as in the P16-06 fixture.
fn spec308_member_on_room_marker(id: u8) -> PlayState {
    let mut state = combat_player_command_state(8, 0);
    state.party[0].status = b'G';
    state.active_objects[0].type_byte = combat_party_actor_byte(b'A');
    state.active_objects[0].tile = combat_party_actor_byte(b'A');
    state.combat_terrain = [[GET_TILE_WALL_TORCH_REPLACEMENT; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.active_objects[9] = ActiveObject {
        type_byte: id,
        tile: id,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    };
    state
}

/// u5-spec#308 Q3 (`RETRACTIONS.md` R649): "The four marker ids are `0xE8`
/// Poison, `0xE9` Sleep, `0xEA` Fire and `0xEB` Energy".
#[test]
fn spec308_q3_room_field_ids_name_poison_sleep_fire_energy() {
    assert_eq!(
        CombatArenaFieldKind::from_kind_byte(0xe8),
        Some(CombatArenaFieldKind::Poison)
    );
    assert_eq!(
        CombatArenaFieldKind::from_kind_byte(0xe9),
        Some(CombatArenaFieldKind::Sleep)
    );
    assert_eq!(
        CombatArenaFieldKind::from_kind_byte(0xea),
        Some(CombatArenaFieldKind::Fire)
    );
    assert_eq!(
        CombatArenaFieldKind::from_kind_byte(0xeb),
        Some(CombatArenaFieldKind::Energy)
    );
    assert_eq!(CombatArenaFieldKind::from_kind_byte(0xec), None);
}

/// u5-spec#308 Q3: "`0xE9` sends the actor that ends a dispatch on it
/// through the shared sleep routine with no draw, message or sound", and
/// "contact leaves either marker unchanged". (The hook itself makes no draw;
/// the shared routine's own world tick of `combat.md §12` still runs.)
#[test]
fn spec308_q3_a_room_placed_sleep_field_sleeps_the_member() {
    let mut state = spec308_member_on_room_marker(0xe9);
    let marker = state.active_objects[9];

    let application = state
        .apply_combat_post_dispatch_contact_for_actor_position(0)
        .expect("the hook recognized the marker");

    assert_eq!(application.field_contact.field, CombatArenaFieldKind::Sleep);
    assert_eq!(state.party[0].status, b'S');
    assert_eq!(state.active_objects[9], marker);
}

/// u5-spec#308 Q3: "`0xEA` plays the target sound, makes one inclusive
/// `0..10` draw, passes that raw value to the shared damage/status endpoint
/// with no attacker".
#[test]
fn spec308_q3_a_room_placed_fire_field_takes_the_fire_arm() {
    let mut state = spec308_member_on_room_marker(0xea);
    let marker = state.active_objects[9];

    let application = state
        .apply_combat_post_dispatch_contact_for_actor_position(0)
        .expect("the hook recognized the marker");

    assert_eq!(application.field_contact.field, CombatArenaFieldKind::Fire);
    assert_eq!(state.active_objects[9], marker);
}

/// u5-spec#308 Q3 (R649): `0xEB` is "the movement validator's only blocking
/// field id"; `0xE9` and `0xEA` "are passable".
#[test]
fn spec308_q3_a_room_placed_energy_field_blocks_and_the_others_do_not() {
    for (id, walkable) in [(0xe9u8, true), (0xea, true), (0xeb, false)] {
        let mut state = spec308_member_on_room_marker(id);
        // Move the marker one cell east of the Avatar at (5, 5).
        state.active_objects[9].x = 6;
        let east = 2;
        assert_eq!(
            state.combat_destination_walkable_for_direction(0, east),
            Some(walkable),
            "marker {id:#04x}"
        );
    }
}

/// u5-spec#308 Q3 (`RETRACTIONS.md` R650): the field pass gives "every
/// `0xE8..0xEB` field marker, cast or room-placed, one inclusive `0..255`
/// draw, consumed either way, and silently removes it below 16"; "Other
/// active objects ... are not examined and cost nothing."
#[test]
fn spec308_q3_the_field_pass_draws_once_per_marker_and_removes_below_16() {
    let mut state = combat_player_command_state(8, 0);
    let ids = [0xe8u8, 0xe9, 0xea, 0xeb, COMBAT_FIELD_KIND_FIRE];
    for (index, id) in ids.iter().enumerate() {
        state.active_objects[10 + index] = ActiveObject {
            type_byte: *id,
            tile: *id,
            x: index,
            y: 1,
            ..ActiveObject::empty()
        };
    }
    // A corpse is not a field and costs no draw.
    state.active_objects[20] = ActiveObject {
        type_byte: 0x1e,
        tile: 0x1e,
        x: 9,
        y: 9,
        ..ActiveObject::empty()
    };
    let corpse = state.active_objects[20];
    let other_records: Vec<usize> = (0..state.active_objects.len())
        .filter(|slot| {
            !(10..10 + ids.len()).contains(slot)
                && CombatArenaFieldKind::from_kind_byte(state.active_objects[*slot].type_byte)
                    .is_some()
        })
        .collect();
    assert!(other_records.is_empty(), "the fixture holds no other field");

    // Find a seed whose five draws remove some markers and keep others.
    let mut seed = 0u16;
    let draws = loop {
        let mut probe = seed;
        let draws: Vec<u16> = (0..ids.len())
            .map(|_| u5_prng_range_u16(&mut probe, 0, 255))
            .collect();
        if draws.iter().any(|draw| *draw < 16) && draws.iter().any(|draw| *draw >= 16) {
            break draws;
        }
        seed = seed.wrapping_add(1);
    };
    state.prng_state = seed;
    let mut expected_prng = seed;
    for _ in 0..ids.len() {
        let _ = u5_prng_range_u16(&mut expected_prng, 0, 255);
    }

    let removed = state.apply_combat_field_dissolution_pass();

    assert_eq!(state.prng_state, expected_prng, "one draw per marker");
    assert_eq!(removed, draws.iter().filter(|draw| **draw < 16).count());
    for (index, draw) in draws.iter().enumerate() {
        assert_eq!(
            state.active_objects[10 + index].is_empty(),
            *draw < 16,
            "marker {:#04x} draw {draw}",
            ids[index]
        );
    }
    assert_eq!(state.active_objects[20], corpse);
}

/// u5-spec#308 Q3 (R650): the endpoint "runs after a committed
/// command-prompted action", in the order ring hook, field pass, ageing.
#[test]
fn spec308_q3_the_committed_action_tail_runs_the_field_pass() {
    let mut state = combat_player_command_state(8, 0);
    state.combat_terrain = [[GET_TILE_WALL_TORCH_REPLACEMENT; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.active_objects[12] = ActiveObject {
        type_byte: 0xe9,
        tile: 0xe9,
        x: 1,
        y: 1,
        ..ActiveObject::empty()
    };
    let mut without = state.clone();
    without.active_objects[12] = ActiveObject::empty();

    let _ = state.apply_combat_committed_action_tail(0);
    let _ = without.apply_combat_committed_action_tail(0);

    let mut expected = without.prng_state;
    let _ = u5_prng_range_u16(&mut expected, 0, 255);
    assert_eq!(state.prng_state, expected, "one extra draw for the marker");
}

/// P16-07: `combat.md §8`'s G row runs "the shared Get handler", whose
/// `containers.md §3` step 5 falls through to the live-tile cascade, and
/// `containers.md §7` borrows a wall torch "from any approach direction".
/// Get beside an arena wall torch borrows it.
#[test]
fn playfix16_p16_07_arena_get_borrows_a_wall_torch() {
    let mut state = combat_player_command_state(8, 0);
    state.combat_terrain[5][4] = GET_TILE_WALL_TORCH_SOURCE_SECOND;
    state.torch_counter = 0;
    let serial = state.sound_effect_serial;

    let outcome = state.get_combat_actor_direction(0, Direction::West);

    assert_eq!(outcome, MoveOutcome::Got);
    assert_eq!(state.combat_terrain[5][4], GET_TILE_WALL_TORCH_REPLACEMENT);
    assert_eq!(state.torch_counter, BORROWED_FIXTURE_TORCH_DURATION);
    assert_eq!(state.message, GET_TILE_BORROWED_LINE);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );

    // Any other tile still answers the cascade's refusal.
    let outcome = state.get_combat_actor_direction(0, Direction::North);
    assert_eq!(outcome, MoveOutcome::Blocked);
    assert_eq!(state.message, GET_NOTHING_REFUSAL);
}

/// P16-08: `combat.md §4` - "Bit value 2 marks a dungeon context (it refuses
/// the In Por-style and Klimb arena actions ...)" - and `magic.md §8`: "The
/// refusal state is tested first; when it is set the cast fails at once and
/// nothing is drawn." Combat Blink in a dungeon room prints `Failed!`, plays
/// no variant, draws nothing and leaves the caster in place.
#[test]
fn playfix16_p16_08_combat_blink_fails_at_once_in_a_dungeon_fight() {
    let dir = playfix2_room_game_dir();
    let mut state = playfix16_room_fight(&dir);
    state.spell_charges[BLINK_SPELL_INDEX] = 1;
    state.party[0].mana = 8;
    state.party[0].level = 8;
    let before = (state.combat_actors[0].x, state.combat_actors[0].y);
    let prng_before = state.prng_state;
    let serial = state.sound_effect_serial;

    assert_eq!(state.cast_combat_blink(0), MoveOutcome::Blocked);

    assert_eq!(state.message, "Failed!");
    assert_eq!((state.combat_actors[0].x, state.combat_actors[0].y), before);
    assert_eq!(state.prng_state, prng_before, "nothing is drawn");
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::CastFailure]
    );
    let _ = fs::remove_dir_all(dir);
}

/// P16-09: `dungeon-mode.md §8.1` puts combat's blank row between
/// `Entering room...` and the first turn banner. After an edge exit lost a
/// fight, the loss line's row is closed (the stock capture's blank row under
/// `BATTLE IS LOST!`), so the arena's column flag must not still say it is
/// open: the next fight's first banner keeps its blank row.
#[test]
fn playfix16_p16_09_an_edge_loss_leaves_no_open_arena_row_for_the_next_fight() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_player_command_state(8, 0);
    state.combat_actors[0].y = 0;
    state.active_objects[0].y = 0;
    state.pending_combat_actor_slot = Some(0);
    state.next_combat_actor_slot = 1;

    handle_play_key_input(&mut state, 'w', "", game_dir).unwrap();

    assert!(!state.combat_active, "the edge exit ended the fight");
    assert!(!state.combat_transcript_row_open);

    // The next fight's banner, printed with its leading newline.
    state.emit_combat_print("\nAvatar, armed");
    let entries = state.message_entries();
    let banner = entries
        .iter()
        .rposition(|entry| entry.text == "Avatar, armed")
        .expect("the banner is printed");
    assert!(
        entries[banner - 1].explicit_blank,
        "a blank row precedes the banner: {:?}",
        entries.iter().map(|entry| &entry.text).collect::<Vec<_>>()
    );
}

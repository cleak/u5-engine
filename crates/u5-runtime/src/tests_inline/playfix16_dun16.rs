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

/// P16-04: `formats/dungeon-dat.md §4` patches the trigger after room combat
/// and "also sets a saved room-clear bit keyed by dungeon and room id", with
/// no outcome condition, and `combat.md §14` has the framer discard the
/// round loop's word. A room the party walked out of while foes remained is
/// already `0xA?` for the visit, so its durable bit is set too.
#[test]
fn playfix16_p16_04_a_lost_room_fight_still_sets_the_room_clear_bit() {
    let dir = playfix2_room_game_dir();
    let scene = DungeonScene::new(35).unwrap();
    let mut state = playfix16_room_fight(&dir);
    assert!(!dungeon_room_clear_bit_is_set(
        &state.dungeon_room_clear_bitmap,
        scene,
        7
    ));

    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Defeat);

    assert!(!state.combat_active);
    assert_eq!(state.grid[dungeon_cell_index(0, 2, 1)], 0xa7);
    assert!(
        dungeon_room_clear_bit_is_set(&state.dungeon_room_clear_bitmap, scene, 7),
        "the durable bit agrees with the visit-local 0xA? patch"
    );
    let _ = fs::remove_dir_all(dir);
}

/// P16-05: `systems/save-load.md` - "save/load restores room completion by
/// replaying the cleared-room demotion pass" - and `dungeon-mode.md §5`'s
/// "When a level is loaded or reloaded". Resuming inside the dungeon keeps
/// the saved working copy, but a cleared room's other `0xF?` cells in it are
/// demoted; an uncleared room's cell and every other byte are kept.
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

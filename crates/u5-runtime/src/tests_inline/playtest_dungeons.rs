// Regressions for the dungeon exploratory playtest (findings E08..E20).
// Each test names the finding and the published sentence it rests on.

fn playtest_transcript_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// E08: `dungeon-mode.md §13.1` - "A K that applies something - a climb
/// up, a climb down ... - reports \"acted\" to the dungeon loop, so the
/// turn's post-action pass runs", and `§8.1` lists the fall trap among the
/// post-action underfoot consequences. A climb down onto a fall trap falls
/// at once, not after the next command.
#[test]
fn playtest_e08_klimb_down_onto_a_fall_trap_falls_before_the_next_key() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(2, 1, 1)] = 0x20;
    grid[dungeon_cell_index(3, 1, 1)] = 0x69;
    let mut state = dungeon_state(grid, 2, 1, 1);

    handle_play_key_input(&mut state, 'k', "", Path::new("")).unwrap();

    assert_eq!(state.area, Area::Dungeon { scene, level: 4 });
    let rows = playtest_transcript_rows(&state);
    let down = rows
        .iter()
        .position(|row| row.ends_with("Down!"))
        .unwrap_or_else(|| panic!("the climb word prints: {rows:?}"));
    assert_eq!(
        rows[down + 1..]
            .iter()
            .filter(|row| !row.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>(),
        [
            DUNGEON_PIT_TRAP_LINE.trim_end(),
            DUNGEON_FALLING_LINE.trim_end(),
            DUNGEON_SPLAT_LINE.trim_end()
        ],
        "{rows:?}"
    );
}

/// E08: the same pass enters a room the climb lands on (`§8.1`, "Room
/// trigger or room-helper cell | `Entering room...`"), on the one-direction
/// route and on the up-or-down prompt's answer alike.
#[test]
fn playtest_e08_klimb_onto_a_room_cell_enters_it_at_once() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(2, 1, 1)] = 0x10;
    grid[dungeon_cell_index(1, 1, 1)] = 0xF2;
    let mut up = dungeon_state(grid.clone(), 2, 1, 1);
    handle_play_key_input(&mut up, 'k', "", Path::new("")).unwrap();
    assert_eq!(up.area, Area::Dungeon { scene, level: 1 });
    assert_eq!(up.message, DUNGEON_ROOM_ENTRY_NARRATION);
    assert_eq!(up.dungeon_cell(1, 1, 1), 0xA2, "the room helper ran");

    grid[dungeon_cell_index(2, 1, 1)] = 0x30;
    let mut two_way = dungeon_state(grid, 2, 1, 1);
    handle_play_key_input(&mut two_way, 'k', "", Path::new("")).unwrap();
    handle_play_key_input(&mut two_way, '<', "", Path::new("")).unwrap();
    assert_eq!(two_way.area, Area::Dungeon { scene, level: 1 });
    assert_eq!(two_way.message, DUNGEON_ROOM_ENTRY_NARRATION);
    assert_eq!(two_way.dungeon_cell(1, 1, 1), 0xA2, "the room helper ran");
}

/// E09: `dungeon-mode.md §13.1` - "Up or Down spell, destination refused |
/// the climb word (already printed), then `Failed!`", and an ordinary level
/// change prints "the same `Up!` or `Down!`, and only that". The word is
/// printed once by the shared apply routine.
#[test]
fn playtest_e09_level_change_spell_prints_the_climb_word_once() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(1, 1, 1)] = 0xC0;
    let mut refused = dungeon_state(grid, 0, 1, 1);
    arm_spell(&mut refused, DES_POR_SPELL_INDEX);
    handle_play_key_input(&mut refused, 'C', "1DP", Path::new("")).unwrap();
    assert_eq!(refused.area, Area::Dungeon { scene, level: 0 });
    let rows = playtest_transcript_rows(&refused);
    let down = rows
        .iter()
        .position(|row| row == "Down!")
        .expect("climb word");
    assert_eq!(rows[down..], ["Down!", "Failed!"], "{rows:?}");

    let mut accepted = dungeon_state(open_dungeon_record(), 0, 1, 1);
    arm_spell(&mut accepted, DES_POR_SPELL_INDEX);
    handle_play_key_input(&mut accepted, 'C', "1DP", Path::new("")).unwrap();
    assert_eq!(accepted.area, Area::Dungeon { scene, level: 1 });
    let rows = playtest_transcript_rows(&accepted);
    assert_eq!(
        rows.iter().filter(|row| *row == "Down!").count(),
        1,
        "{rows:?}"
    );
    assert!(!accepted.message_slot_needs_flush());
}

/// E10: `dungeon-mode.md §11` - "The rest concludes either with `Party
/// rested!\n` or `Ambushed!\n\n` (and an immediate combat)". An interrupted
/// dungeon camp prints the ambush line alone, once, with no leading space.
#[test]
fn playtest_e10_dungeon_camp_ambush_prints_only_the_ambush_line() {
    let dir = debug_game_dir();
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.clock = GameClock::new(8, 55).unwrap();
    // The first hour rollover's 1-in-64 interruption roll comes up zero.
    state.prng_state = (0..=u16::MAX)
        .find(|seed| {
            let mut scratch = *seed;
            u5_prng_range_u16(&mut scratch, 0, 63) == 0
        })
        .expect("some seed interrupts");

    let _ = state.hole_up_command(
        &dir,
        InlineRestRequest {
            hours: Some(9),
            watcher: None,
        },
    );

    let rows = playtest_transcript_rows(&state);
    let sleep = rows
        .iter()
        .position(|row| row == CAMP_BODY_SLEEP_LINE.trim_end())
        .unwrap_or_else(|| panic!("the camp body prints: {rows:?}"));
    assert_eq!(rows[sleep + 1], "", "{rows:?}");
    assert_eq!(rows[sleep + 2], "Ambushed!", "{rows:?}");
    assert_eq!(
        rows.iter().filter(|row| row.contains("Ambushed")).count(),
        1,
        "{rows:?}"
    );
    assert!(
        !rows.iter().any(|row| row.contains("rested")),
        "the result line is replaced, not followed: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

/// E11: `formats/saved-gam.md §12.1` - "`0x105D` | Dungeon facing" - and
/// `dungeon-mode.md §4.1`: "a saved facing survives untouched and the party
/// resumes facing whatever the save recorded ... a save whose facing field is
/// zero starts the party facing north".
#[test]
fn playtest_e11_dungeon_facing_round_trips_through_the_save() {
    for (facing, code) in [
        (Direction::North, 0u8),
        (Direction::East, 1),
        (Direction::South, 2),
        (Direction::West, 3),
    ] {
        let mut template = saved_game_seed_bytes(33, 0, 1, 1);
        template[SAVE_AVATAR_NAME_OFFSET] = b'A';
        let dir = save_fidelity_game_dir(template);
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = facing;
        state.write_save_files(&dir).unwrap();

        let bytes = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(
            bytes[SAVE_NPC_BAND_DUNGEON_FACING_OFFSET], code,
            "{facing:?}"
        );
        let options = load_play_options_from_save(&dir).unwrap();
        assert_eq!(options.facing, Some(facing));
        let loaded = PlayState::load_scene(&dir, options).unwrap();
        assert_eq!(loaded.player.facing, facing);
        let _ = fs::remove_dir_all(dir);
    }
}

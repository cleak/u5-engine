// `systems/audio.md` trigger-boundary regressions: jimmy get.
//
// Each test names the published clause it pins. Add tests here rather
// than to the numbered chunks so the audio work stays reviewable as a
// unit.

    /// `audio.md §8.1` Jimmy key breaks: "Failure only: print the break line,
    /// play the 40-update action snap, then decrement the key count."
    ///
    /// The native magic-door arm auto-breaks before any dexterity roll, so it
    /// is always a failure and always sounds.
    #[test]
    fn town_jimmy_magic_door_key_break_snaps_between_the_line_and_the_decrement() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_MAGIC_PLAIN_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        let serial = state.sound_effect_serial;

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
        assert_eq!(state.sound_effect_serial, serial + 1);
    }

    /// `audio.md §8.1`. The same arm reached through `jimmy_town_direction`
    /// rather than through the preflight, so both copies of the magic-door
    /// break are pinned.
    #[test]
    fn town_jimmy_direction_magic_door_key_break_snaps() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_MAGIC_PLAIN_TILE;
        let mut state = test_state(grid, 1, 1);
        let scene = Scene::new(17).unwrap();
        let serial = state.sound_effect_serial;

        assert_eq!(
            state
                .jimmy_town_direction(None, scene, 0, 0, Direction::East)
                .unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
    }

    /// `audio.md §8.1`. Visible locked door, dexterity roll failed.
    #[test]
    fn town_jimmy_visible_door_roll_failure_snaps() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 0;
        state.prng_state = 0x1234;
        let serial = state.sound_effect_serial;

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.grid[32 + 2], TOWN_DOOR_PLAIN_LOCKED_TILE);
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
    }

    /// `audio.md §8.1`. Sidecar-declared ordinary lock, dexterity roll failed.
    #[test]
    fn town_jimmy_sidecar_lock_roll_failure_snaps() {
        let dir = debug_game_dir();
        fs::write(dir.join(TOWN_LOCK_TABLE_FILE), "CASTLE:0 0 2 1 185 184\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 0;
        state.prng_state = 0x1234;
        let serial = state.sound_effect_serial;

        assert_eq!(
            state.jimmy_facing_with_game_dir(Some(&dir)).unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `audio.md §8.1`. Occupied restraint, dexterity roll failed.
    #[test]
    fn town_jimmy_restraint_roll_failure_snaps() {
        let mut grid = open_grid();
        grid[32 + 2] = JIMMY_STOCKS_TILE;
        let mut state = test_state(grid, 1, 1);
        state.player.facing = Direction::East;
        state.party[0].climb_stat = 0;
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
        let serial = state.sound_effect_serial;

        assert_eq!(state.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
    }

    /// `audio.md §8.1`. A surface container with no pickable threshold breaks
    /// the key outright; a failed container roll breaks it after the roll.
    /// Both are failures and both sound.
    #[test]
    fn town_jimmy_surface_container_key_breaks_snap() {
        let mut unpickable = test_state(open_grid(), 1, 1);
        unpickable.player.facing = Direction::East;
        unpickable.keys = 2;
        unpickable.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x01,
            aux3: 0,
        });
        let serial = unpickable.sound_effect_serial;

        assert_eq!(unpickable.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(unpickable.message, "Key broke!");
        assert_eq!(unpickable.keys, 1);
        assert_eq!(
            unpickable.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );

        let mut failure = test_state(open_grid(), 1, 1);
        failure.player.facing = Direction::East;
        failure.party[0].climb_stat = 0;
        failure.prng_state = 0x1234;
        failure.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0xff,
            aux3: 0,
        });
        let serial = failure.sound_effect_serial;

        assert_eq!(failure.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(failure.message, "Key broke!");
        assert_eq!(failure.keys, DEFAULT_KEY_STOCK - 1);
        assert_eq!(
            failure.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
    }

    /// `audio.md §8.1` is mode-agnostic — it says "failure only", not "failure
    /// only outside a dungeon". `§8.5` silences dungeon *walking and turning*,
    /// which is a different command, so a dungeon Jimmy key break sounds too.
    #[test]
    fn dungeon_jimmy_key_breaks_snap() {
        let mut plain = open_dungeon_record();
        plain[dungeon_cell_index(0, 1, 1)] = 0x40;
        let mut state = dungeon_state(plain, 0, 1, 1);
        state.keys = 2;
        let serial = state.sound_effect_serial;

        assert_eq!(
            state
                .jimmy_facing_with_game_dir_and_member(None, Some(0))
                .unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.keys, 1);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );

        let mut marked = open_dungeon_record();
        marked[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut state = dungeon_state(marked, 0, 1, 1);
        state.keys = 2;
        state.party[0].climb_stat = u8::MAX;
        state.prng_state = 0x1234;
        let serial = state.sound_effect_serial;

        assert_eq!(
            state
                .jimmy_facing_with_game_dir_and_member(None, Some(0))
                .unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(state.message, "Key broke!");
        assert_eq!(state.keys, 1);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
    }

    /// `audio.md §8.1`: "Success is silent", and the last row of the same
    /// table repeats that a successful Jimmy has no confirmed cue.
    #[test]
    fn successful_jimmy_picks_are_silent() {
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut door = test_state(grid, 1, 1);
        door.player.facing = Direction::East;
        door.prng_state = 0x1234;
        let serial = door.sound_effect_serial;

        assert_eq!(door.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(door.message, "Unlocked!");
        assert_eq!(door.grid[32 + 2], TOWN_DOOR_PLAIN_UNLOCKED_TILE);
        assert!(door.sound_effects_after(serial).is_empty());

        let mut container = test_state(open_grid(), 1, 1);
        container.player.facing = Direction::East;
        container.party[0].climb_stat = 30;
        container.prng_state = 0x1234;
        container.active_objects.push(ActiveObject {
            type_byte: 0x4f,
            tile: 0x4f,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0x80,
            aux3: 0,
        });
        let serial = container.sound_effect_serial;

        assert_eq!(container.jimmy_facing(), MoveOutcome::LockTried);

        assert_eq!(container.message, "Unlocked!");
        assert!(container.sound_effects_after(serial).is_empty());

        let mut chest_grid = open_dungeon_record();
        chest_grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut chest = dungeon_state(chest_grid, 0, 1, 1);
        chest.party[0].climb_stat = 30;
        chest.prng_state = 0x1234;
        let serial = chest.sound_effect_serial;

        assert_eq!(
            chest
                .jimmy_facing_with_game_dir_and_member(None, Some(0))
                .unwrap(),
            MoveOutcome::LockTried
        );

        assert_eq!(chest.message, "Unlocked!");
        assert!(chest.sound_effects_after(serial).is_empty());
    }

    /// `audio.md §8.1` sounds the key *break*, not the Jimmy command. Every
    /// refusal, prompt, and declined selection reaches its result without
    /// breaking a key, so `§9`'s "generic successful commands" silence holds.
    #[test]
    fn jimmy_paths_that_break_no_key_are_silent() {
        // "No keys!" — refused before the tile probe.
        let mut grid = open_grid();
        grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut no_keys = test_state(grid, 1, 1);
        no_keys.player.facing = Direction::East;
        no_keys.keys = 0;
        let serial = no_keys.sound_effect_serial;
        assert_eq!(no_keys.jimmy_facing(), MoveOutcome::Blocked);
        assert_eq!(no_keys.message, "No keys!");
        assert!(no_keys.sound_effects_after(serial).is_empty());

        // "No lock!" — the target tile carries no lock at all.
        let mut no_lock = test_state(open_grid(), 1, 1);
        no_lock.player.facing = Direction::East;
        let serial = no_lock.sound_effect_serial;
        assert_eq!(no_lock.jimmy_facing(), MoveOutcome::Blocked);
        assert_eq!(no_lock.message, "No lock!");
        assert!(no_lock.sound_effects_after(serial).is_empty());

        // "No lock!" — off-grid target, refused before the lock tables.
        let mut off_grid = test_state(open_grid(), 0, 1);
        off_grid.player.facing = Direction::West;
        let serial = off_grid.sound_effect_serial;
        assert_eq!(off_grid.jimmy_facing(), MoveOutcome::Blocked);
        assert_eq!(off_grid.message, "No lock!");
        assert!(off_grid.sound_effects_after(serial).is_empty());

        // "No one is there!" — an empty restraint exits before the roll.
        let mut restraint_grid = open_grid();
        restraint_grid[32 + 2] = JIMMY_MANACLES_TILE;
        let mut empty_restraint = test_state(restraint_grid, 1, 1);
        empty_restraint.player.facing = Direction::East;
        let serial = empty_restraint.sound_effect_serial;
        assert_eq!(empty_restraint.jimmy_facing(), MoveOutcome::LockTried);
        assert_eq!(empty_restraint.message, "No one is there!");
        assert_eq!(empty_restraint.keys, DEFAULT_KEY_STOCK);
        assert!(empty_restraint.sound_effects_after(serial).is_empty());

        // The party prompt is not yet an answer, so no key has broken.
        let mut prompt_grid = open_grid();
        prompt_grid[32 + 2] = TOWN_DOOR_PLAIN_LOCKED_TILE;
        let mut prompt = test_state(prompt_grid, 1, 1);
        prompt.player.facing = Direction::East;
        let serial = prompt.sound_effect_serial;
        assert_eq!(
            prompt
                .jimmy_facing_with_game_dir_and_member(None, None)
                .unwrap(),
            MoveOutcome::Observed
        );
        assert_eq!(prompt.message, PARTY_SELECTION_PROMPT);
        assert!(prompt.sound_effects_after(serial).is_empty());

        // A declined member selection never reaches a lock.
        let mut declined_grid = open_dungeon_record();
        declined_grid[dungeon_cell_index(0, 1, 1)] = 0x4b;
        let mut declined = dungeon_state(declined_grid, 0, 1, 1);
        declined.party[0].status = b'D';
        let serial = declined.sound_effect_serial;
        assert_eq!(
            declined
                .jimmy_facing_with_game_dir_and_member(None, Some(0))
                .unwrap(),
            MoveOutcome::PromptDeclined
        );
        assert!(declined.sound_effects_after(serial).is_empty());
    }

    /// `audio.md §8.1` borrowed fixed object: "After the live tile is
    /// rewritten and the borrowing line is printed, play the 40-update action
    /// snap." The town and world Get-tile handlers are the two live-tile
    /// rewrite sites.
    #[test]
    fn borrowed_fixed_object_get_snaps_after_the_rewrite_and_the_line() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_GET_TILE_TABLE_FILE),
            "CASTLE:0 0 2 1 16 55 KEYS 2\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut town = test_state(grid, 1, 1);
        town.player.facing = Direction::East;
        let serial = town.sound_effect_serial;

        assert_eq!(
            town.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(town.grid[32 + 2], 16);
        assert!(town.message.contains("Got tile 55"));
        assert_eq!(
            town.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
        assert_eq!(town.sound_effect_serial, serial + 1);

        // A borrowed fixture that grants nothing at all still sounds: the
        // published boundary is the tile rewrite plus the line, not the grant.
        fs::write(dir.join(TOWN_GET_TILE_TABLE_FILE), "CASTLE:0 0 2 1 16 55\n").unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut bare = test_state(grid, 1, 1);
        bare.player.facing = Direction::East;
        let serial = bare.sound_effect_serial;

        assert_eq!(
            bare.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );
        assert_eq!(
            bare.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );

        fs::write(
            dir.join(WORLD_GET_TILE_TABLE_FILE),
            "UNDERWORLD 0 0 5 55 GOLD 7\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut world = world_state(grid, 255, 0);
        world.player.facing = Direction::East;
        let serial = world.sound_effect_serial;

        assert_eq!(
            world.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(world.grid[world_cell_index(0, 0)], 5);
        assert!(world.message.contains("Got world tile 55"));
        assert_eq!(
            world.sound_effects_after(serial),
            vec![SoundEffect::ActionSnap]
        );
        let _ = fs::remove_dir_all(dir);
    }

    /// `audio.md §8.1` last row: "Ordinary active-object pickup, crop pickup,
    /// and successful Jimmy — no generic pickup cue is confirmed. Do not reuse
    /// the borrowing or ring sound for them."
    ///
    /// The crop branch is the Get-tile entry that grants food and therefore
    /// takes the `karma.md §4` crop/table-food debit; `karma.md §4` records the
    /// borrowed-furniture branch as the one with no such debit.
    #[test]
    fn crop_and_table_food_pickup_stay_silent() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(TOWN_GET_TILE_TABLE_FILE),
            "CASTLE:0 0 2 1 16 55 FOOD 1\n",
        )
        .unwrap();
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut town_crop = test_state(grid, 1, 1);
        town_crop.player.facing = Direction::East;
        town_crop.moral_standing = 3;
        let serial = town_crop.sound_effect_serial;

        assert_eq!(
            town_crop.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(town_crop.moral_standing, 2);
        assert!(town_crop.message.contains("added 1 food"));
        assert!(town_crop.sound_effects_after(serial).is_empty());

        fs::write(
            dir.join(WORLD_GET_TILE_TABLE_FILE),
            "UNDERWORLD 0 0 5 55 FOOD 4\n",
        )
        .unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(0, 0)] = 55;
        let mut world_crop = world_state(grid, 255, 0);
        world_crop.player.facing = Direction::East;
        world_crop.food = 12;
        world_crop.moral_standing = 3;
        let serial = world_crop.sound_effect_serial;

        assert_eq!(
            world_crop.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(world_crop.moral_standing, 2);
        assert!(world_crop.sound_effects_after(serial).is_empty());

        // Eating from a reachable plate is the table-food branch, not a
        // borrowed fixture.
        let mut plate_grid = open_grid();
        plate_grid[32 + 2] = 0x9b;
        let mut plate = test_state(plate_grid, 2, 2);
        plate.player.facing = Direction::North;
        plate.food = 12;
        plate.moral_standing = 3;
        let serial = plate.sound_effect_serial;

        assert_eq!(
            plate.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(plate.grid[32 + 2], 0x95);
        assert_eq!(plate.moral_standing, 2);
        assert!(plate.sound_effects_after(serial).is_empty());
        let _ = fs::remove_dir_all(dir);
    }

    /// `audio.md §8.1` last row and `§9`: ordinary active-object pickup has no
    /// confirmed cue. The object-table path never reaches the live-tile
    /// rewrite, so it must not borrow the borrowing snap.
    #[test]
    fn ordinary_active_object_pickup_and_get_refusals_stay_silent() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(OBJECT_PICKUP_TABLE_FILE),
            "CASTLE:0 0 2 1 KEYS 1 210\n",
        )
        .unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.active_objects.push(ActiveObject {
            type_byte: 210,
            tile: 210,
            x: 2,
            y: 1,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        });
        let serial = state.sound_effect_serial;

        assert_eq!(
            state.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Got
        );

        assert_eq!(state.keys, DEFAULT_KEY_STOCK + 1);
        assert!(state.message.contains("Got 1 keys"));
        assert!(state.sound_effects_after(serial).is_empty());

        // "Nothing to get here." — no table entry matched, no tile rewrite.
        let mut grid = open_grid();
        grid[32 + 2] = 55;
        let mut refused = test_state(grid, 1, 1);
        refused.player.facing = Direction::East;
        let serial = refused.sound_effect_serial;

        assert_eq!(
            refused.get_facing_with_game_dir(&dir).unwrap(),
            MoveOutcome::Blocked
        );

        assert_eq!(refused.message, "Nothing to get here.");
        assert!(refused.sound_effects_after(serial).is_empty());
        let _ = fs::remove_dir_all(dir);
    }

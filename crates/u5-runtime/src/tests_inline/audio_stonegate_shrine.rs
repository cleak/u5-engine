// `systems/audio.md` trigger-boundary regressions: stonegate shrine.
//
// Each test names the published clause it pins. Add tests here rather
// than to the numbered chunks so the audio work stays reviewable as a
// unit.

#[test]
fn stonegate_trapdoor_script_sounds_the_descent_then_one_rumble_per_death() {
    // `audio.md §8.2`: after the black viewport fill, the 750-tone descent;
    // stop, then one trap-class rumble for each party member as that member
    // is killed and the stats panel is repainted.
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town { scene, floor: 0 };
    let members = state.party.len();
    assert!(members > 0, "the fixture party must contain a member");
    let serial = state.sound_effect_serial;

    state.apply_stonegate_trapdoor_script(0);

    let mut expected = vec![SoundEffect::StonegateDescent];
    expected.extend(std::iter::repeat(SoundEffect::StonegateMemberDeath).take(members));
    assert_eq!(state.sound_effects_after(serial), expected);
    assert_eq!(state.sound_effect_serial, serial + 1 + members as u64);
    assert!(state.party.iter().all(|member| member.hp == 0));
}

#[test]
fn stonegate_entry_presentation_stays_silent() {
    // The only Stonegate row in the `audio.md §8.2` inventory is the trapdoor
    // scripted death. Entry narration is a `§9` generic command result.
    let scene = Scene::new(STONEGATE_SCENE_BYTE).unwrap();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town { scene, floor: 0 };
    state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;
    let serial = state.sound_effect_serial;

    state.append_stonegate_entry_presentation_message();

    assert!(state.message.contains("Stonegate entry:"));
    assert!(state.sound_effects_after(serial).is_empty());
}

#[test]
fn successful_ruined_shrine_restoration_flashes_a_second_time() {
    // `audio.md §8.4`: the recognized Word flashes before the location test,
    // and a successful restoration invokes the shared effect again at its own
    // success boundary.
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    let mut world = world_state(open_world_grid(), shrine_x + 1, shrine_y);
    world.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    let shrine_index = world_cell_index(shrine_x, shrine_y);
    world.grid[shrine_index] = WORLD_RUINED_SHRINE_TILE;
    world.shrine_ruin_flags[0] = 0x85;
    let serial = world.sound_effect_serial;

    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    let after_word = world.sound_effects_after(serial);
    assert_eq!(after_word.len(), 1, "the recognized Word flashes once");
    assert!(matches!(after_word[0], SoundEffect::MajorFlash { .. }));

    // The first three responses only collect prompts and are silent.
    assert_eq!(world.step_active_shrine_restoration('H', "onesty"), None);
    assert_eq!(world.step_active_shrine_restoration('A', "hm"), None);
    assert_eq!(world.step_active_shrine_restoration('x', "AHM x"), None);
    assert_eq!(world.sound_effects_after(serial).len(), 1);

    assert_eq!(
        world.step_active_shrine_restoration('a', "hm forever"),
        Some(MoveOutcome::Used)
    );

    let effects = world.sound_effects_after(serial);
    assert_eq!(effects.len(), 2, "restoration adds a second shared flash");
    assert!(
        effects
            .iter()
            .all(|effect| matches!(effect, SoundEffect::MajorFlash { .. }))
    );
    assert_eq!(world.grid[shrine_index], WORLD_SHRINE_TILE);
    assert!(world.message.contains(SHRINE_RESTORATION_SUCCESS_BANNER));
}

#[test]
fn failed_ruined_shrine_restoration_keeps_the_word_flash_alone() {
    // `audio.md §8.4`: only a *successful* restoration invokes the shared
    // effect a second time. A wrong virtue, mantra, or coordinate is silent.
    let mut world = world_state(open_world_grid(), 11, 10);
    let ruined_index = world_cell_index(10, 10);
    world.grid[ruined_index] = WORLD_RUINED_SHRINE_TILE;
    world.shrine_ruin_flags[0] = 0x83;
    world.refresh_world_live_chunks_for_current_area().unwrap();
    let serial = world.sound_effect_serial;

    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    assert_eq!(world.sound_effects_after(serial).len(), 1);

    assert_eq!(world.step_active_shrine_restoration('\r', ""), None);
    assert_eq!(world.step_active_shrine_restoration('A', "hm"), None);
    assert_eq!(world.step_active_shrine_restoration('A', "hm"), None);
    assert_eq!(
        world.step_active_shrine_restoration('A', "hm"),
        Some(MoveOutcome::Used)
    );

    let effects = world.sound_effects_after(serial);
    assert_eq!(effects.len(), 1, "the failed restoration adds no flash");
    assert!(matches!(effects[0], SoundEffect::MajorFlash { .. }));
    assert_eq!(world.grid[ruined_index], WORLD_RUINED_SHRINE_TILE);
}

#[test]
fn combat_jimmy_key_breaks_snap_on_every_failure_arm() {
    // `audio.md §8.1`: failure only — print the break line, play the 40-update
    // action snap, then decrement the key count.
    let mut magic = combat_player_command_state(10, 10);
    magic.keys = 1;
    magic.party[0].climb_stat = 30;
    magic.combat_terrain[5][6] = TOWN_DOOR_MAGIC_PLAIN_TILE;
    let serial = magic.sound_effect_serial;

    assert_eq!(
        magic.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(magic.message, "Key broke!");
    assert_eq!(magic.keys, 0);
    assert_eq!(
        magic.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );

    let mut restraint = combat_player_command_state(10, 10);
    restraint.keys = 1;
    restraint.party[0].climb_stat = 0;
    restraint.combat_terrain[5][6] = JIMMY_MANACLES_TILE;
    let serial = restraint.sound_effect_serial;

    assert_eq!(
        restraint.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(restraint.message, "Key broke!");
    assert_eq!(restraint.keys, 0);
    assert_eq!(
        restraint.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );

    let mut door = combat_player_command_state(10, 10);
    door.keys = 1;
    door.party[0].climb_stat = 0;
    door.combat_terrain[5][6] = TOWN_DOOR_PLAIN_LOCKED_TILE;
    let serial = door.sound_effect_serial;

    assert_eq!(
        door.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(door.message, "Key broke!");
    assert_eq!(door.keys, 0);
    assert_eq!(
        door.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );
}

#[test]
fn combat_jimmy_successes_and_refusals_emit_nothing() {
    // `audio.md §8.1` last row and `§9`: a successful Jimmy has no cue, and
    // neither do the refusals that never reach a lock.
    let mut unlocked = combat_player_command_state(10, 10);
    unlocked.keys = 1;
    unlocked.party[0].climb_stat = 30;
    unlocked.combat_terrain[5][6] = TOWN_DOOR_PLAIN_LOCKED_TILE;
    let serial = unlocked.sound_effect_serial;

    assert_eq!(
        unlocked.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(unlocked.message, "Unlocked!");
    assert_eq!(unlocked.keys, 1);
    assert!(unlocked.sound_effects_after(serial).is_empty());

    let mut freed = combat_player_command_state(10, 10);
    freed.keys = 1;
    freed.party[0].climb_stat = 30;
    freed.combat_terrain[5][6] = JIMMY_STOCKS_TILE;
    let serial = freed.sound_effect_serial;

    assert_eq!(
        freed.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::LockTried
    );
    assert_eq!(freed.message, "Unlocked");
    assert!(freed.sound_effects_after(serial).is_empty());

    let mut keyless = combat_player_command_state(10, 10);
    keyless.keys = 0;
    keyless.combat_terrain[5][6] = TOWN_DOOR_MAGIC_PLAIN_TILE;
    let serial = keyless.sound_effect_serial;

    assert_eq!(
        keyless.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::Blocked
    );
    assert_eq!(keyless.message, "No keys!");
    assert!(keyless.sound_effects_after(serial).is_empty());

    let mut no_lock = combat_player_command_state(10, 10);
    no_lock.keys = 1;
    no_lock.combat_terrain[5][6] = TOWN_DOOR_CLEARED_TILE;
    let serial = no_lock.sound_effect_serial;

    assert_eq!(
        no_lock.jimmy_combat_actor_direction(0, Direction::East),
        MoveOutcome::Blocked
    );
    assert_eq!(no_lock.message, "No lock!");
    assert_eq!(no_lock.keys, 1);
    assert!(no_lock.sound_effects_after(serial).is_empty());
}

/// `town-mode.md §7.1` steps 1-5 and `audio.md §8.2`: the scripted death
/// sounds the 750-step descent **before** the live grid is rewritten to lava,
/// then one trap-class rumble per party member as that member is killed.
/// Ordering matters: the descent is step 2 and the grid rewrite is step 3, so
/// an emit placed after the rewrite is one published step late.
#[test]
fn stonegate_scripted_death_sounds_the_descent_before_the_grid_rewrite() {
    let mut state = test_state(open_grid(), 4, 4);
    state.area = Area::Town {
        scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
        floor: 0,
    };
    let party_len = state.party.len();
    assert!(party_len > 0, "the fixture must carry a party to kill");
    // A cell the rewrite will change, so the grid is observably pre-rewrite.
    state.grid[0] = 0x00;
    let serial = state.sound_effect_serial;

    state.apply_stonegate_trapdoor_script(0);

    let effects = state.sound_effects_after(serial);
    let mut expected = vec![SoundEffect::StonegateDescent];
    expected.extend(std::iter::repeat_n(
        SoundEffect::StonegateMemberDeath,
        party_len,
    ));
    assert_eq!(
        effects, expected,
        "the descent leads, then exactly one rumble per party member",
    );
    assert!(
        state.grid.iter().all(|tile| *tile == STONEGATE_TRAPDOOR_GRID_TILE),
        "step 3 still rewrites every cell to lava",
    );
    assert!(
        state.party.iter().all(|member| member.hp == 0),
        "step 5 still kills every in-party slot",
    );
}

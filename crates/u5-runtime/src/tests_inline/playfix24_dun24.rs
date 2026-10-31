// Playtest round 24, dungeon findings (P24-01 .. P24-03).

/// A one-member dungeon arena at level zero with a Giant Rat still standing,
/// the party actor on the arena's up ladder, and the world table the surface
/// exit reads.
fn p24_dungeon_ladder_fight(applier: DungeonPostCombatApplier) -> (PlayState, std::path::PathBuf) {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_LOCATION_TABLE_FILE),
        "BRITANNIA 10 20 DUNGEON:0\n",
    )
    .unwrap();
    let mut state = dungeon_post_combat_state(0, applier, 5, 5, false);
    state.combat_terrain[5][5] = 0xc8;
    state.combat_round_loop_prologue_ran = true;
    state.active_objects[8] = ActiveObject {
        type_byte: 0x90,
        tile: 0x90,
        x: 8,
        y: 8,
        ..ActiveObject::empty()
    };
    state.combat_actors[8] = CombatActorDescriptor::from_row([
        10,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        COMBAT_CLASS_GIANT_RAT,
        8,
        0,
        8,
        8,
    ]);
    state.clock = GameClock::new(12, 2).unwrap();
    (state, dir)
}

/// Playtest P24-01. `dungeon-mode.md §14.1`: the post-combat movement
/// applier changes the level with codes 5/6 "including both surface exits",
/// and neither handler "advances the world clock itself"; `combat.md §4`'s
/// framer exit makes a zero-increment clock call, and `§4` step 4 says
/// nothing on the way to the overworld runs the clock. A room walk-in or a
/// contact step already charged its minute before the fight, so climbing
/// off the top in that fight adds none. The A-Attack fight, which runs
/// inside its own dispatch, adds nothing past its loop-head minute either.
#[test]
fn playtest_p24_01_post_action_fight_surface_exit_charges_no_second_minute() {
    for (applier, loop_head_owed) in [
        (DungeonPostCombatApplier::MovementApplier, false),
        (DungeonPostCombatApplier::AttackLevelsOnly, true),
    ] {
        let (mut state, dir) = p24_dungeon_ladder_fight(applier);
        state.dungeon_loop_minute_charged = loop_head_owed;
        let turn_before = state.turn;

        handle_play_key_input(&mut state, 'K', "", &dir).unwrap();

        assert!(!state.combat_active, "{applier:?}");
        assert!(
            matches!(state.area, Area::World { plane: WorldPlane::Britannia }),
            "{applier:?}: {:?}",
            state.area
        );
        assert_eq!(state.clock, GameClock::new(12, 2).unwrap(), "{applier:?}");
        if !loop_head_owed {
            assert_eq!(state.turn, turn_before, "no second turn either");
        }
        let _ = fs::remove_dir_all(dir);
    }
}

/// Playtest P24-02. `combat.md §14`: `BATTLE IS LOST!` "begins with a
/// newline and has no trailing newline"; `doors-and-z-transitions.md §12.1`:
/// the exit's first print is `\nExit to `, whose feed closes that open row.
/// So `Exit to` sits on the row directly beneath the loss line.
#[test]
fn playtest_p24_02_exit_to_follows_the_loss_line_without_a_blank_row() {
    let (mut state, dir) = p24_dungeon_ladder_fight(DungeonPostCombatApplier::AttackLevelsOnly);
    state.dungeon_loop_minute_charged = true;

    handle_play_key_input(&mut state, 'K', "", &dir).unwrap();

    assert!(matches!(state.area, Area::World { .. }));
    let texts: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    let lost = texts
        .iter()
        .rposition(|text| text == "BATTLE IS LOST!")
        .unwrap_or_else(|| panic!("the loss line: {texts:?}"));
    assert_eq!(
        texts[lost + 1],
        "Exit to Britannia!",
        "no blank row between the two: {texts:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

// Regressions for the nineteenth exploratory playtest's dungeon findings
// (P19-04..P19-06). Each test names the finding and the published sentence
// it rests on.

/// A live Giant Bat record at `(x, y)` in the dungeon monster slot.
fn playfix19_monster(state: &mut PlayState, x: usize, y: usize) {
    state.active_objects.push(ActiveObject {
        type_byte: 1,
        tile: 1,
        x,
        y,
        z: 0,
        phase: 0x20,
        aux1: 21,
        aux3: 0,
    });
}

/// P19-04: `dungeon-mode.md §10` gives the post-action pass as "the wake
/// roll, the active-monster step, the encounter auto-face, the underfoot
/// reaction", and `§14` steps the monster "After a successful dungeon
/// action". A party standing in a poison field used to freeze the monster:
/// the field arm returned before the step ran.
#[test]
fn playfix19_p19_04_the_monster_steps_while_the_party_stands_in_a_field() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 1, 1)] = 0x81;
    let mut state = dungeon_state(grid, 0, 1, 1);
    for member in state.party.iter_mut() {
        member.climb_stat = 0;
    }
    playfix19_monster(&mut state, 5, 5);
    state.prng_state = 0x1234;
    let turn_before = state.turn;
    state.advance_turn();

    state
        .apply_dungeon_post_turn_effects_after_turn(turn_before, &debug_game_dir())
        .unwrap();

    let monster = state.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT];
    assert_ne!((monster.x, monster.y), (5, 5), "the monster took its step");
    assert_eq!(state.party[0].status, b'P', "the field still reacted");
}

/// P19-04, contact: `dungeon-mode.md §4.1` (issue #304 Q1), "When the turn's
/// monster step reports contact before a fall chain, the contact fight ends
/// ... The chain then runs." The step's contact comes first; the underfoot
/// reaction waits for the fight's return. (The step never accepts a field or
/// pit cell, so the vehicle here is `§14`'s contact without an accepted
/// step: a walled-in monster on the party's own cell.)
#[test]
fn playfix19_p19_04_a_contact_fight_comes_before_the_underfoot_reaction() {
    let level = 3u8;
    let mut grid = open_dungeon_record();
    for (x, y) in [(2usize, 1usize), (3, 2), (2, 3), (1, 2)] {
        grid[dungeon_cell_index(level, x, y)] = 0xb0;
    }
    grid[dungeon_cell_index(level, 2, 2)] = 0x81;
    let mut state = dungeon_state(grid, level, 2, 2);
    for member in state.party.iter_mut() {
        member.climb_stat = 0;
    }
    playfix19_monster(&mut state, 2, 2);
    let dir = debug_game_dir();
    let turn_before = state.turn;
    state.advance_turn();

    let outcome = state
        .apply_dungeon_post_turn_effects_after_turn(turn_before, &dir)
        .unwrap();

    assert_eq!(outcome, Some(MoveOutcome::Used));
    assert!(state.combat_active, "the contact fight launched");
    assert_ne!(state.party[0].status, b'P', "the field waits for the fight");
    assert_eq!(state.pending_dungeon_underfoot_reaction, Some((level, 2, 2)));

    state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);
    state
        .resume_dungeon_underfoot_reaction_after_combat(&dir)
        .unwrap();

    assert_eq!(state.pending_dungeon_underfoot_reaction, None);
    assert_eq!(state.party[0].status, b'P', "the field reacted after the fight");
}

/// P19-05: `dungeon-mode.md §14`'s contact sequence, "3. One composite
/// redraw ... 4. Flush type-ahead." The contact asks its frontend to drop
/// the queued keys before the arena opens.
#[test]
fn playfix19_p19_05_a_contact_requests_a_type_ahead_flush() {
    let level = 3u8;
    let mut grid = open_dungeon_record();
    for (x, y) in [(0usize, 7usize), (0, 1), (7, 0)] {
        grid[dungeon_cell_index(level, x, y)] = 0xb0;
    }
    let mut state = dungeon_state(grid, level, 1, 0);
    playfix19_monster(&mut state, 0, 0);
    state.prng_state = (1u16..)
        .find(|seed| {
            let mut probe = *seed;
            u5_prng_range_u16(&mut probe, 0, 3) == 1
        })
        .unwrap();
    let Area::Dungeon { scene, .. } = state.area else {
        unreachable!()
    };
    assert!(!state.typeahead_flush_requested);

    state.apply_dungeon_active_monster_step(scene, level).unwrap();

    assert!(state.combat_active);
    assert!(state.typeahead_flush_requested);
}

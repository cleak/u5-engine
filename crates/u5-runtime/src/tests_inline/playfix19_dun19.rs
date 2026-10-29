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

/// A lit party at `(3, 3)` facing north; with `in_view` the monster stands
/// one cell ahead, otherwise behind the party where no paint reaches it.
fn playfix19_lit_view(in_view: bool) -> PlayState {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 3, 4)] = 0xb0;
    let mut state = dungeon_state(grid, 0, 3, 3);
    state.player.facing = Direction::North;
    state.torch_counter = 50;
    state.light_spell_counter = 0;
    if in_view {
        playfix19_monster(&mut state, 3, 2);
    } else {
        playfix19_monster(&mut state, 6, 6);
    }
    state.prng_state = 0x09eb;
    state
}

/// The generator `state` holds after two further `0..100` pose draws.
fn playfix19_after_pose_draws(state: &PlayState) -> u16 {
    let mut prng = state.prng_state;
    let _ = u5_prng_range_u16(&mut prng, 0, 100);
    let _ = u5_prng_range_u16(&mut prng, 0, 100);
    prng
}

/// P19-06: `dungeon-mode.md §4.1` "Render and poll", "Each no-key pass first
/// waits one timer tick ... and then repaints, so every pass consumes the
/// corridor painter's draws: in a lit view with the active monster visible,
/// its two pose draws".
#[test]
fn playfix19_p19_06_each_idle_pass_takes_the_visible_monsters_pose_draws() {
    let mut seen = playfix19_lit_view(true);
    let mut unseen = playfix19_lit_view(false);

    seen.idle_wait_pass(None).unwrap();
    unseen.idle_wait_pass(None).unwrap();

    assert_eq!(seen.prng_state, playfix19_after_pose_draws(&unseen));
}

/// P19-06: `§8.1`, "a blocked move and a turn ... each runs the shared
/// redraw", which paints the view (`§6.9`).
#[test]
fn playfix19_p19_06_turns_and_blocked_steps_repaint_with_the_pose_draws() {
    // Turn back to face the monster from the east.
    let mut seen = playfix19_lit_view(true);
    let mut unseen = playfix19_lit_view(false);
    for state in [&mut seen, &mut unseen] {
        state.player.facing = Direction::East;
        let _ = state.turn_dungeon(false);
    }
    assert_eq!(seen.player.facing, Direction::North);
    assert_eq!(seen.prng_state, playfix19_after_pose_draws(&unseen), "turn");

    // Back up into the wall behind the party, the monster still ahead.
    let mut seen = playfix19_lit_view(true);
    let mut unseen = playfix19_lit_view(false);
    for state in [&mut seen, &mut unseen] {
        let Area::Dungeon { scene, level } = state.area else {
            unreachable!()
        };
        let outcome = state
            .step_dungeon_back(Direction::South, 3, 4, scene, level, None)
            .unwrap();
        assert_eq!(outcome, MoveOutcome::Blocked);
        assert_eq!(state.player.facing, Direction::North);
    }
    assert_eq!(seen.prng_state, playfix19_after_pose_draws(&unseen), "blocked");
}

/// P19-06: the draws are tied to the loop's redraw points, "not to how
/// often a frontend renders" - a frontend frame repaints the poses the last
/// runtime paint chose and takes no draw.
#[test]
fn playfix19_p19_06_a_frontend_render_takes_no_pose_draw() {
    let mut state = playfix19_lit_view(true);
    state.redraw_dungeon_first_person_view();
    let prng = state.prng_state;
    let phase = state.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT].phase;

    for _ in 0..3 {
        state
            .render_dungeon_viewport(VIEWPORT_PLAYER_ROW, TileGraphicsDepth::Ega16, None, None)
            .unwrap();
    }

    assert_eq!(state.prng_state, prng);
    assert_eq!(state.active_objects[DUNGEON_ACTIVE_MONSTER_SLOT].phase, phase);
}

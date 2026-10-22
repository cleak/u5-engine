// Regressions for the residual items found while assembling `rc6` from the
// fourth-pass playtest fixes (u5-engine#49).
//
// Each test names the published sentence it pins.

/// `dungeon-mode.md §10`: the dungeon A-Attack handler "calls the combat
/// framer" from inside the dispatch; `combat.md §4`: "combat is a function
/// call" and "the only time a fight adds is its round-wrap minutes";
/// `dungeon-mode.md §15`: the dungeon's one minute is charged at the head of
/// the loop iteration and §4 step 6's post-action pass runs after the
/// dispatch returns. The engine's dungeon arm advanced the turn (and its
/// upkeep) before framing the arena, the sibling of playtest H21's overworld
/// arm.
#[test]
fn rc6_dungeon_attack_spends_no_turn_before_the_arena() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0,
        tile: 0,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 20,
        aux3: 0,
    });
    let turn = state.turn;
    let clock = state.clock;

    assert!(state.handle_dungeon_key('A', Path::new("")).unwrap());

    assert!(state.combat_active, "the attack frames the arena");
    assert_eq!(state.turn, turn, "no turn is spent before the arena");
    let mut loop_head = clock;
    loop_head.advance_minutes(1);
    assert_eq!(state.clock, loop_head, "only the loop-head minute has run");

    // A round-wrap inside the fight adds its own minute (`combat.md §4`);
    // the loop-head flag still held for the exit does not swallow it.
    state.combat_round_counter = COMBAT_ROUND_COUNTER_WRAP - 1;
    state.advance_combat_round_counter();
    let mut after_wrap = loop_head;
    after_wrap.advance_minutes(COMBAT_ROUND_WRAP_TIME_ADVANCE_MINUTES);
    assert_eq!(state.clock, after_wrap);
    let turn_in_arena = state.turn;

    // The exit spends the attack's turn, with no minute of its own: the
    // iteration's minute was charged at the loop head.
    state.apply_combat_round_loop_exit(CombatRoundLoopExit::LeaveCombat);
    assert!(!state.combat_active);
    assert_eq!(
        state.turn,
        turn_in_arena + 1,
        "the turn is spent after the arena"
    );
    assert_eq!(state.clock, after_wrap);
}

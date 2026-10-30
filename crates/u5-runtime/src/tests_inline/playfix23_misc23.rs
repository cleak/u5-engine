// Regressions for the pass-23 exploratory playtest findings (misc23).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// Playtest P23-09. `combat.md §8.1`/`§12`: the arena-cell test asks whether
/// "this mover may stand there", its terrain "passable for the parent's own
/// tile"; `movement.md §4` gives the `0x80..0x8F` Sea Horse, Squid, Sea
/// Serpent and Shark runs the glyph-restricted (water) predicate, and
/// `combat.md §5.4` seats them on arena 11's water. Sharks never left their
/// placement cells, so a melee-only party aboard ship could not end the fight.
#[test]
fn playtest_p23_09_a_shark_swims_toward_the_party_across_arena_water() {
    const SHARK: u8 = 19;
    assert_eq!(crate::combat_class_sprite_base(SHARK), Some(0x8c));
    let mut state = combat_ai_turn_state(5, 8);
    state.combat_terrain = [[0x01; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    // The party stands on a deck plank, the shark in the water below it.
    state.combat_terrain[5][5] = 0x44;
    state.combat_actors[8].owner_target_class = SHARK;
    state.active_objects[8].type_byte = 0x8c;
    state.active_objects[8].tile = 0x8c;

    let application = state
        .apply_combat_ai_turn_with_inputs(
            8,
            false,
            0,
            false,
            255,
            255,
            &[],
            None,
            0,
            false,
            None,
            false,
            &[1, 2, 3, 4],
            None,
        )
        .unwrap();

    assert!(
        !matches!(
            application.movement,
            Some(CombatAiMovementOutcome::Blocked { .. })
        ),
        "movement: {:?}",
        application.movement
    );
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), (5, 7));
    // The shark never climbs onto the deck: the deck plank is not water.
    let mask = state.combat_legal_cell_mask_for_mover_class(SHARK);
    assert!(mask[6][5]);
    state.combat_terrain[6][5] = 0x44;
    let mask = state.combat_legal_cell_mask_for_mover_class(SHARK);
    assert!(!mask[6][5]);
    // A land class keeps the shared arena test, which refuses water.
    assert!(!state.combat_legal_cell_mask_for_mover_class(COMBAT_CLASS_GIANT_RAT)[8][5]);
}

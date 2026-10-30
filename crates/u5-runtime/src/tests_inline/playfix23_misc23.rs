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

/// Playtest P23-08. `inventory.md §6`, "Already readied by this character":
/// R-Ready "clears the first matching readied slot, then returns one copy to
/// the shared equipment counter if that counter is below `99`". Unreadying a
/// Chain Coif from a counter of 149 cut it to 99, destroying fifty coifs.
#[test]
fn playtest_p23_08_unready_leaves_a_counter_at_or_above_99_unchanged() {
    let coif = crate::EQUIPMENT_SHORT_LABELS
        .iter()
        .position(|label| *label == "Chain Coif")
        .expect("published short label");
    for (before, after) in [(149u8, 149u8), (99, 99), (254, 254), (98, 99), (0, 1)] {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_strengths = vec![99];
        state.party_equipment = default_party_equipment(state.party.len());
        state.party_equipment[0][0] = coif as u8;
        state.equipment_stock[coif] = before;

        state.ready_equipment(InlineReadyRequest {
            party_index: 0,
            item_id: coif,
        });

        assert!(
            state.party_equipment[0].iter().all(|slot| *slot == EQUIPMENT_EMPTY),
            "the coif is unreadied from {before}"
        );
        assert_eq!(state.equipment_stock[coif], after, "counter from {before}");
    }
}

/// Playtest P23-04. `commands.md §10`, town-family locations: "The dispatcher
/// first requires the party to be on a bed/inn-style tile ... If it succeeds,
/// control enters the hours-prompt rest path." `On land or ship!` belongs to
/// the outdoor/dungeon rest handler; a mounted party on an inn bed got it.
#[test]
fn playtest_p23_04_a_mounted_party_on_an_inn_bed_gets_the_hours_prompt() {
    let dir = crate::test_fixtures::debug_game_dir();
    std::fs::write(dir.join(TOWN_REST_BED_TABLE_FILE), "CASTLE:0 0 1 1 55\n").unwrap();
    for transport in [
        TransportState::Horse {
            type_byte: 0x12,
            tile: 0x12,
        },
        TransportState::Carpet {
            type_byte: 0x14,
            tile: 0x14,
        },
    ] {
        let mut grid = open_grid();
        grid[32 + 1] = 55;
        let mut state = test_state(grid, 1, 1);
        state.clock = GameClock::new(8, 0).unwrap();
        state.player.transport = transport;

        handle_play_key_input(&mut state, 'H', "", &dir).unwrap();

        assert!(state.active_rest.is_some(), "{transport:?}: {:?}", state.message);
        assert_eq!(
            state.message,
            crate::commands::TOWN_BED_HOURS_PROMPT,
            "{transport:?}"
        );
    }
    let _ = std::fs::remove_dir_all(dir);
}

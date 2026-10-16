// Regression tests for two paired-capture differences in the town fight
// scenarios (`combat-town-attack-after-entry`, `combat-damage-narration`).
// Both turned out to be the original's own dice, not an engine defect: the
// stock side shows both outcomes across runs from the same seed and keys.
// These tests pin the deterministic half of each contract so a real
// regression is told apart from the lottery.

/// The geometry of Britain's arms shop at 10:00, as the paired walk-in
/// meets it: the resident's active waypoint (AI 4) sits two cells east of
/// the party's final cell and one row above the counter, and the counter
/// row blocks everything south of the resident.
const P15_WAYPOINT: (u8, u8, u8) = (4, 19, 0);

fn p15_shop_state(party: (usize, usize), merchant: (usize, usize)) -> PlayState {
    let mut grid = g6_open_grid();
    for x in 2..=7 {
        grid[20 * 32 + x] = 0x4E;
    }
    let mut state = test_state(grid, party.0, party.1);
    state.npcs = vec![g6_npc(1, merchant.0, merchant.1, 4, P15_WAYPOINT)];
    state.relink_npc_objects();
    state
}

#[test]
fn p15_resident_wanders_while_the_party_is_four_from_its_waypoint() {
    // `npc-schedules.md §9` value `4`: "While the player is four or more
    // tiles from the *waypoint*, the NPC takes the ordinary bounded wander
    // step"; `§9.1`: one coin, one direction, one candidate per turn.
    // Party at (1,20) is exactly four from (4,19). The chase would always
    // step west; the wander has to show the coin (no move) and the other
    // directions too. The capture's stock merchant stood still through
    // this window while this engine's stepped east and north - both are
    // outcomes of this one draw, which `prng.md §3` seeds from the host
    // clock on both sides.
    let mut outcomes = std::collections::BTreeSet::new();
    for seed in 0..0x1000u16 {
        let mut state = p15_shop_state((1, 20), (3, 19));
        state.prng_state = seed;
        state.advance_npc_schedules();
        assert_eq!(state.pending_town_npc_contact, None, "seed {seed:#x}");
        outcomes.insert(g6_pos(&state, 0));
    }
    assert!(
        outcomes.contains(&(3, 19)),
        "the lost coin leaves it in place"
    );
    assert!(outcomes.contains(&(4, 19)), "east is a legal wander step");
    assert!(outcomes.contains(&(3, 18)), "north is a legal wander step");
    assert!(
        !outcomes.contains(&(3, 20)),
        "the counter refuses the south step"
    );
}

#[test]
fn p15_resident_closes_in_and_greets_one_turn_later_at_distance_three() {
    // `npc-schedules.md §9`: at distance three the NPC engages, the
    // approach chooser takes the first of east, north, west, south that
    // strictly closes the distance, and "When the NPC's own move first
    // creates adjacency, contact waits until a later engagement
    // invocation." No draw decides any of this, so every seed agrees: the
    // arrival turn moves the resident beside the party with no greeting, and
    // the next acted command (the scenario's Look) raises the shop contact.
    for seed in (0..0x1000u16).step_by(97) {
        let mut state = p15_shop_state((1, 19), (3, 19));
        state.prng_state = seed;
        state.advance_npc_schedules();
        assert_eq!(g6_pos(&state, 0), (2, 19), "seed {seed:#x}");
        assert_eq!(state.pending_town_npc_contact, None, "seed {seed:#x}");

        state.advance_npc_schedules();
        assert_eq!(g6_pos(&state, 0), (2, 19), "contact produces no movement");
        assert_eq!(
            state.pending_town_npc_contact,
            Some(TownNpcContactEvent {
                npc_slot: 1,
                behavior: NpcAiBehavior::ApproachAndAttack,
            }),
            "seed {seed:#x}"
        );
    }
}

#[test]
fn p16_killed_town_merchant_leaves_a_chest_or_a_corpse_by_the_drop_roll() {
    // `town-mode.md §14`: a town fight is an ordinary arena fight, so the
    // merchant dies through `combat.md §6.3`'s default death branch. Its
    // class is `encounters.md §4`'s `(sprite - 0x40) / 4` = 5, whose
    // `catalogs/monster-bestiary.md` row carries a drop cap of 10. The first
    // `1..30` roll at or under the cap leaves `0x01`, the chest, with the
    // cap in byte 5; a higher roll leaves `0x1F`, the moldy corpse.
    // Measured: the stock side left a chest in two of four kills and the
    // corpse in the other two, so neither marker is a difference.
    assert_eq!(outdoor_combat_class_id(0x54), Some(5));
    let stats = combat_class_stats(5).unwrap();
    assert_eq!(stats.default_drop_cap, 10);

    for (first_accepts, tile, aux1) in [
        (true, COMBAT_DEFAULT_DEATH_DROP_TILE, 10),
        (false, COMBAT_DEFAULT_DEATH_NO_DROP_TILE, 0),
    ] {
        let mut state = world_state(open_world_grid(), 10, 20);
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 9;
        place_death_side_effect_monster(&mut state, 5, actor_slot, active_object_slot);
        state.prng_state = seed_for_default_death_gates(10, first_accepts, false);
        state
            .apply_combat_weapon_damage_to_target(
                None,
                actor_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();
        let object = state.active_objects[active_object_slot];
        assert_eq!((object.type_byte, object.tile), (tile, tile));
        assert_eq!(object.aux1, aux1);
        // `tile-catalog.md §3.1`: an actor byte draws from the upper bank,
        // so the chest marker is drawn as tile 257, not terrain tile 1.
        assert_eq!(actor_tile_for_byte(tile), Some(256 + usize::from(tile)));
    }

    // The gate's rate over every twelve-bit host-clock seed: the roll is
    // `max(1, u / 2)` for a uniform `u` in `0..=60`, so 22 of the 61 raw
    // values land at or under 10 - a chest about 36% of the time.
    let accepted = (0..0x1000u16)
        .filter(|&seed| {
            let mut prng = seed;
            let first = combat_skewed_roll_1_to_30(u5_prng_range_u16(&mut prng, 0, 60) as u8);
            combat_default_death_drop_gate_accepts_inclusive(10, first)
        })
        .count();
    assert!(
        (0x1000 * 30 / 100..0x1000 * 42 / 100).contains(&accepted),
        "{accepted} of 4096 seeds leave a chest"
    );
}

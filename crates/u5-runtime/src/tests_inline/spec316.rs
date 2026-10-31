// u5-spec#315 (ranged scatter, party routing, projectile pass) and #316
// (town A-Attack karma), answered in u5-spec 2099ce9.

/// `combat.md §11.1` (u5-spec#315 Q1 and Q2): "two separate draws from the
/// shared range generator, each over the inclusive range 1..3 and read as an
/// offset of -1, 0 or +1 from the aim cell: the **column first, then the
/// row**. Both axes are drawn on every attempt. If either coordinate falls
/// outside the arena (0..10 on either axis), both are drawn again ... An
/// off-arena landing cell is never clamped and never becomes an empty miss
/// ... Each attempt costs exactly two generator calls."
#[test]
fn spec315_q1_scatter_draws_column_then_row_and_redraws_off_arena() {
    let aim = (0u8, 0u8);
    let attacker = (1u8, 1u8);
    let mut first_attempt_accepted = 0;
    let mut redrawn = 0;
    for seed in 0u16..2000 {
        // `prng.md`'s generator has a fixed point; a stream parked there
        // repeats one value forever, and no rejection loop can leave it.
        if u5_prng_advance_state(seed) == seed {
            continue;
        }
        let mut state = combat_ai_turn_state(8, 5);
        state.prng_state = seed;
        let mut replay = state.clone();
        let landing = state.draw_combat_ranged_miss_scatter_cell(aim, attacker);
        assert!(
            combat_arena_coordinate_in_bounds(i16::from(landing.0), i16::from(landing.1)),
            "seed {seed:#06X}: never off the arena"
        );
        assert_ne!(landing, attacker, "seed {seed:#06X}: never the attacker's cell");
        assert!(landing.0 <= 1 && landing.1 <= 1, "inside the block centred on the aim");

        // Replay the stream two draws at a time: column, then row.
        let mut attempts = 0;
        loop {
            attempts += 1;
            let column = replay.random_range_u8(1, 3);
            let row = replay.random_range_u8(1, 3);
            let candidate = (i16::from(aim.0) + i16::from(column) - 2, i16::from(aim.1) + i16::from(row) - 2);
            if combat_arena_coordinate_in_bounds(candidate.0, candidate.1)
                && (candidate.0 as u8, candidate.1 as u8) != attacker
            {
                assert_eq!(landing, (candidate.0 as u8, candidate.1 as u8), "seed {seed:#06X}");
                break;
            }
        }
        assert_eq!(state.prng_state, replay.prng_state, "two generator calls per attempt");
        if attempts == 1 {
            first_attempt_accepted += 1;
        } else {
            redrawn += 1;
        }
    }
    // A corner aim rejects five of the nine offsets plus the attacker's
    // cell, so both outcomes occur.
    assert!(first_attempt_accepted > 0 && redrawn > 0);
}

/// `combat.md §11` (R662): the forced scatter's block "includes its centre",
/// so the turned shot can land on the wearer; the old eight-way draw never
/// could.
#[test]
fn spec315_q1_scatter_can_land_on_the_aim_cell() {
    let mut landed_on_aim = false;
    for seed in 0u16..500 {
        let mut state = combat_ai_turn_state(8, 5);
        state.prng_state = seed;
        if state.draw_combat_ranged_miss_scatter_cell((5, 5), (8, 5)) == (5, 5) {
            landed_on_aim = true;
            break;
        }
    }
    assert!(landed_on_aim);
}

/// `combat.md §11.1`, "The projectile pass": "Over all 120 in-arena offsets:
/// at the ordinary interval the landing cell is tested in only 6 geometries
/// ...; with effect code 0 in 10 ...; with effect code 7 in 110. Non-launch
/// tests per shot range over 0..10, 0..11 and 0..18 respectively." This
/// fixture reproduces those published counts for a shooter at the arena
/// centre.
#[test]
fn spec315_q4_projectile_walker_sampling_matches_the_published_sweep() {
    assert_eq!(projectile_walker_sampling_for_effect(6), (0, 13));
    assert_eq!(projectile_walker_sampling_for_effect(7), (0, 8));
    assert_eq!(projectile_walker_sampling_for_effect(0), (4, 13));
    let shooter = (5i32, 5i32);
    for (effect_code, landing_tested, max_tests) in [(6u8, 6, 10), (0, 10, 11), (7, 110, 18)] {
        let mut geometries = 0;
        let mut most = 0;
        let mut offsets = 0;
        for y in 0..11i32 {
            for x in 0..11i32 {
                if (x, y) == shooter {
                    continue;
                }
                offsets += 1;
                let samples = projectile_walker_samples_for_effect(shooter, (x, y), effect_code);
                let tested: Vec<_> = samples
                    .iter()
                    .filter(|sample| sample.tested && (sample.column, sample.row) != shooter)
                    .collect();
                if tested.iter().any(|sample| (sample.column, sample.row) == (x, y)) {
                    geometries += 1;
                }
                most = most.max(tested.len());
            }
        }
        assert_eq!(offsets, 120);
        assert_eq!(geometries, landing_tested, "effect code {effect_code}");
        assert_eq!(most, max_tests, "effect code {effect_code}");
    }
}

/// `combat.md §11.1`: "At distance one the ordinary interval can never
/// block; with the Sling's effect code, terrain on the target cell itself
/// can."
#[test]
fn spec315_q4_distance_one_blocks_only_at_the_sling_interval() {
    let blocks_target = |column: i32, row: i32| (column, row) == (6, 5);
    assert_eq!(
        trace_projectile_walk_for_effect((5, 5), (6, 5), 6, blocks_target),
        OutdoorRangedAttackOutcome::Connects
    );
    assert_eq!(
        trace_projectile_walk_for_effect((5, 5), (6, 5), 0, blocks_target),
        OutdoorRangedAttackOutcome::Connects
    );
    assert_eq!(
        trace_projectile_walk_for_effect((5, 5), (6, 5), 7, blocks_target),
        OutdoorRangedAttackOutcome::Obstructed { column: 6, row: 5 }
    );
}

/// A wall tile from `overworld.md §6.2.2`'s blocking set.
const SPEC315_BLOCKING_TILE: u8 = 0x0C;

/// `combat.md §11.1`: on the monster arm a blocked pass prints nothing
/// "whether the to-hit roll succeeded or failed", nobody is struck, and the
/// resolver returns before the damage roller.
#[test]
fn spec315_q4_a_blocked_monster_shot_strikes_nobody_even_on_a_hit() {
    assert!(outdoor_projectile_tile_blocks(SPEC315_BLOCKING_TILE));
    let setup = |blocked: bool| {
        let mut state = combat_ai_turn_state(8, 5);
        // Class 12 (Guard): range 15, payload 2, no special branch.
        state.combat_actors[8].owner_target_class = 12;
        state.party[0].status = b'G';
        state.party[0].hp = 20;
        state.party[0].max_hp = 20;
        state.party_combat_defense = vec![7];
        if blocked {
            state.combat_terrain[5][7] = SPEC315_BLOCKING_TILE;
        }
        state
    };

    let mut clear = setup(false);
    let hit = clear
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .unwrap();
    assert!(
        matches!(
            hit.resolution,
            Some(CombatWeaponAttackResolution::Hit {
                route: CombatWeaponAttackRangeRoute::Ranged { .. },
                ..
            })
        ),
        "{hit:?}"
    );
    assert!(clear.party[0].hp < 20);

    let mut blocked = setup(true);
    let prng_before = blocked.prng_state;
    let miss = blocked
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .unwrap();
    assert!(matches!(
        miss.resolution,
        Some(CombatWeaponAttackResolution::Miss { .. })
    ));
    assert_eq!(miss.damage_application, None);
    assert_eq!(blocked.party[0].hp, 20);
    assert_eq!(blocked.prng_state, prng_before, "no defence draw after a block");
}

fn spec315_party_cursor_state(monster_x: u8, item_id: u8) -> PlayState {
    let mut state = combat_player_command_state(monster_x, 5);
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = item_id;
    state.party_strengths = vec![30];
    state.party_experience = vec![0];
    state
}

fn spec315_confirm_at(
    state: &mut PlayState,
    steps_east: usize,
    forced_hit: bool,
) -> CombatAttackWalkApplication {
    assert!(state.begin_combat_attack_walk(0, true).cursor_open);
    for _ in 0..steps_east {
        state
            .apply_combat_targeting_cursor_key_with_inputs(char::from(INPUT_CODE_EAST), None)
            .unwrap();
    }
    state
        .apply_combat_targeting_cursor_key_with_inputs(
            '\r',
            Some(CombatPlayerWeaponAttackInputs {
                damage_roll: Some(0),
                forced_hit: Some(forced_hit),
                ..CombatPlayerWeaponAttackInputs::default()
            }),
        )
        .unwrap()
}

/// `combat.md §11.1`: on the party arm a blocked pass prints "`<aimed
/// target> missed!` ... when the originally aimed cell held a real actor,
/// **even if the roll succeeded**".
#[test]
fn spec315_q4_a_blocked_party_shot_names_the_aimed_actor_even_on_a_hit() {
    let bow = EQUIPMENT_ID_BOW as u8;
    let mut clear = spec315_party_cursor_state(8, bow);
    let hp_before = clear.combat_actors[8].hp_or_wound;
    let walk = spec315_confirm_at(&mut clear, 3, true);
    let (slot, attack) = walk.attack.unwrap();
    assert_eq!(slot, 8);
    assert!(matches!(attack.resolution, CombatWeaponAttackResolution::Hit { .. }));
    assert!(clear.combat_actors[8].hp_or_wound < hp_before);

    let mut blocked = spec315_party_cursor_state(8, bow);
    blocked.combat_terrain[5][7] = SPEC315_BLOCKING_TILE;
    let serial = blocked.sound_effect_serial;
    let walk = spec315_confirm_at(&mut blocked, 3, true);
    let (slot, attack) = walk.attack.unwrap();
    assert_eq!(slot, 8, "the miss names the originally aimed actor");
    assert_eq!(
        attack.resolution,
        CombatWeaponAttackResolution::Miss {
            route: CombatWeaponAttackRangeRoute::Ranged { effect_code: 0 },
            hit_score: attack_hit_score(attack.resolution),
        }
    );
    assert_eq!(attack.damage_application, None);
    assert_eq!(blocked.combat_actors[8].hp_or_wound, hp_before);
    assert_eq!(
        blocked.sound_effects_after(serial),
        vec![SoundEffect::PartyRangedAttackSwing],
        "the swing still played before the roll; no impact presentation"
    );
}

fn attack_hit_score(resolution: CombatWeaponAttackResolution) -> i16 {
    match resolution {
        CombatWeaponAttackResolution::Miss { hit_score, .. } => hit_score,
        _ => i16::MIN,
    }
}

/// `combat.md §11` (u5-spec#315 Q3, R661): "any non-zero cap takes the
/// ranged/effect arm at **every** distance, including an adjacent target
/// ... a failed roll against an adjacent target scatters".
#[test]
fn spec315_q3_an_adjacent_bow_miss_scatters() {
    let mut state = spec315_party_cursor_state(6, EQUIPMENT_ID_BOW as u8);
    let mut expected = state.clone();
    let _ = expected.draw_combat_ranged_miss_scatter_cell((6, 5), (5, 5));
    let walk = spec315_confirm_at(&mut state, 1, false);
    let (_, attack) = walk.attack.unwrap();
    assert!(matches!(
        attack.resolution,
        CombatWeaponAttackResolution::Miss {
            route: CombatWeaponAttackRangeRoute::Ranged { .. },
            ..
        } | CombatWeaponAttackResolution::Hit {
            route: CombatWeaponAttackRangeRoute::Ranged { .. },
            ..
        }
    ));
    if matches!(attack.resolution, CombatWeaponAttackResolution::Miss { .. }) {
        assert_eq!(state.prng_state, expected.prng_state, "the scatter draws, nothing else");
    }

    // A zero-cap item keeps the melee arm and spends no scatter draw.
    let mut club = spec315_party_cursor_state(6, 18);
    let prng_before = club.prng_state;
    let walk = spec315_confirm_at(&mut club, 1, false);
    let (_, attack) = walk.attack.unwrap();
    assert!(matches!(
        attack.resolution,
        CombatWeaponAttackResolution::Miss {
            route: CombatWeaponAttackRangeRoute::Melee,
            ..
        }
    ));
    assert_eq!(club.prng_state, prng_before);
}

/// `combat.md §11.1`: "**Morning Star and Halberd skip the walk entirely**:
/// on a hit they strike the aimed cell directly."
#[test]
fn spec315_q4_morning_star_and_halberd_strike_through_a_blocking_cell() {
    for item in [EQUIPMENT_ID_MORNING_STAR, EQUIPMENT_ID_HALBERD] {
        assert!(combat_party_ranged_item_skips_projectile_pass(item));
        let mut state = spec315_party_cursor_state(7, item as u8);
        state.combat_terrain[5][6] = 0x7F;
        assert!(outdoor_projectile_tile_blocks(0x7F));
        let hp_before = state.combat_actors[8].hp_or_wound;
        let walk = spec315_confirm_at(&mut state, 2, true);
        let (slot, attack) = walk.attack.unwrap();
        assert_eq!(slot, 8);
        assert!(
            matches!(attack.resolution, CombatWeaponAttackResolution::Hit { .. }),
            "item {item}: {:?}",
            attack.resolution
        );
        assert!(state.combat_actors[8].hp_or_wound < hp_before);
    }
    assert!(!combat_party_ranged_item_skips_projectile_pass(EQUIPMENT_ID_BOW));
}

fn spec316_attack_fixture(type_byte: u8, standing: u8) -> (std::path::PathBuf, PlayState) {
    let (dir, mut state) = aud_town_attack_fixture(type_byte);
    state.moral_standing = standing;
    (dir, state)
}

/// `town-mode.md §10`, "Karma cost of A-Attack" (u5-spec#316 Q1): the
/// acceptance debit is 5 for types below `0x80`, guards included, taken
/// "before the terrain test, the conflict chain, its group banner and any
/// arena"; `0xD8` and `0xFC` pay nothing on an ordinary cell.
#[test]
fn spec316_q1_acceptance_debit_before_the_conflict() {
    for (type_byte, expected) in [(0x50u8, 70u8), (0x70, 70), (0x78, 70), (0xD8, 75), (0xFC, 75)] {
        let (dir, mut state) = spec316_attack_fixture(type_byte, 75);
        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();
        assert!(state.combat_active, "0x{type_byte:02X} opens the arena");
        assert_eq!(state.moral_standing, expected, "0x{type_byte:02X}");
        // "Nothing is written to the selector after a fight, on any outcome."
        state.apply_combat_round_loop_exit(CombatRoundLoopExit::Victory);
        assert_eq!(state.moral_standing, expected, "0x{type_byte:02X} after the fight");
        let _ = fs::remove_dir_all(dir);
    }
}

/// "each floored at zero with no other clamp": a start of 7 falls to 0.
#[test]
fn spec316_q1_acceptance_debit_floors_at_zero() {
    let (dir, mut state) = spec316_attack_fixture(0x50, 3);
    handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();
    assert_eq!(state.moral_standing, 0);
    let _ = fs::remove_dir_all(dir);
}

/// "The refused outcomes cost nothing ... `Nothing to attack!` ... end[s]
/// before either debit."
#[test]
fn spec316_q1_refused_targets_cost_nothing() {
    for refused in [0x0Eu8, 0xB5] {
        let (dir, mut state) = spec316_attack_fixture(refused, 75);
        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();
        assert_eq!(state.message, ATTACK_NOTHING_TO_ATTACK_REFUSAL);
        assert_eq!(state.moral_standing, 75, "0x{refused:02X}");
        let _ = fs::remove_dir_all(dir);
    }
}

/// `town-mode.md §10`, "Kill-arm debit", and its net-cost table: on a
/// stocks, manacles, `0x9F` or bed cell every type except `0x78` prints
/// `Murdered!` and pays a further 5; `0x78` prints `Missed!`.
#[test]
fn spec316_q1_kill_arm_debit_and_net_cost_table() {
    for tile in TOWN_ATTACK_KILL_ARM_TILES {
        assert!(town_attack_kill_arm_tile(tile));
    }
    for (type_byte, expected, message) in [
        (0x50u8, 65u8, TOWN_ATTACK_KILL_ARM_MURDERED_MESSAGE),
        (0x70, 65, TOWN_ATTACK_KILL_ARM_MURDERED_MESSAGE),
        (0x78, 70, TOWN_ATTACK_KILL_ARM_MISSED_MESSAGE),
        (0xC0, 70, TOWN_ATTACK_KILL_ARM_MURDERED_MESSAGE),
        (0xD8, 70, TOWN_ATTACK_KILL_ARM_MURDERED_MESSAGE),
        (0xFC, 70, TOWN_ATTACK_KILL_ARM_MURDERED_MESSAGE),
    ] {
        let (dir, mut state) = spec316_attack_fixture(type_byte, 75);
        state.grid[5 * TOWN_GRID_SIDE + 6] = 0xAB;
        handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();
        assert!(!state.combat_active, "0x{type_byte:02X}: the kill arm opens no arena");
        assert_eq!(state.message, message, "0x{type_byte:02X}");
        assert_eq!(state.moral_standing, expected, "0x{type_byte:02X}");
        let killed = message == TOWN_ATTACK_KILL_ARM_MURDERED_MESSAGE;
        assert_eq!(
            state.npcs.iter().any(|npc| npc.slot == 9),
            !killed,
            "0x{type_byte:02X}: the killed NPC's slot is cleared"
        );
        // The removal mark follows the removal-mask rule: a townsperson is
        // recorded, a guard or a type at or above `0x80` is not.
        let recorded = state.removed_town_npc_flags.values().any(|mask| *mask != 0);
        assert_eq!(recorded, killed && type_byte == 0x50, "0x{type_byte:02X}");
        let _ = fs::remove_dir_all(dir);
    }
}

/// "It is also taken when the accepted target has no linked NPC and so no
/// fight opens; nothing is printed after the `Attack-<dir>` echo in that
/// case."
#[test]
fn spec316_q1_an_unlinked_accepted_target_pays_and_prints_nothing() {
    let dir = debug_game_dir();
    let mut state = g6_town_with(Vec::new(), 5, 5);
    state.moral_standing = 75;
    state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
    state.active_objects[5] = ActiveObject {
        type_byte: 0x50,
        tile: 0x50,
        x: 6,
        y: 5,
        z: 0,
        ..ActiveObject::empty()
    };
    handle_play_key_input(&mut state, 'A', "6", &dir).unwrap();
    assert!(!state.combat_active);
    assert_eq!(state.message, "");
    assert_eq!(state.moral_standing, 70);
    let _ = fs::remove_dir_all(dir);
}

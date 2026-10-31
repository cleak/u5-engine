// u5-spec#302 closed-audit fixes for `systems/combat.md` §11.1, "The ranged
// carve-out": a failed ranged or thrown to-hit scatters the shot into the
// three-by-three neighbourhood of the aim cell, and the full hit chain runs
// against whoever stands on the landing cell.

/// A monster class whose attack takes the ranged route at range three and
/// reaches ordinary damage there.
fn auditfix1_ranged_monster_class() -> u8 {
    (0..COMBAT_CLASS_COUNT as u8)
        .find(|&class| {
            combat_ranged_effect_stats(class).is_some_and(|ranged| {
                ranged.range_effect_selector >= 3 && !ranged.food_theft_branch
            }) && combat_class_stats(class).is_some_and(|stats| stats.attack_value > 0)
                && class != COMBAT_CLASS_GAZER
        })
        .expect("a ranged monster class exists")
}

/// Fill every cell of the 3x3 block centred on `centre` except `skip` with a
/// Giant Rat, using actor slots from 9 upward.
fn auditfix1_ring_of_rats(state: &mut PlayState, centre: (u8, u8)) -> Vec<usize> {
    let mut slots = Vec::new();
    let mut slot = 9usize;
    for dy in 0..3u8 {
        for dx in 0..3u8 {
            let (x, y) = (centre.0 + dx - 1, centre.1 + dy - 1);
            if (x, y) == centre {
                continue;
            }
            state.combat_actors[slot] = CombatActorDescriptor::from_row([
                20,
                1,
                COMBAT_ACTOR_FLAG_SELECTABLE_40,
                COMBAT_CLASS_GIANT_RAT,
                slot as u8,
                0,
                x,
                y,
            ]);
            slots.push(slot);
            slot += 1;
        }
    }
    slots
}

fn auditfix1_monster_scatter_state(ranged_class: u8) -> PlayState {
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8] = CombatActorDescriptor::from_row([
        40,
        1,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        ranged_class,
        8,
        0,
        8,
        5,
    ]);
    state.party[0].status = b'G';
    state.party[0].hp = 200;
    state.party[0].max_hp = 200;
    state
}

/// `combat.md §11.1`: "On a failed roll the shot scatters ... The resolver
/// then reports whoever occupies the landing cell, and the **full hit chain
/// runs against that actor** - impact presentation, newline, damage, result
/// line - whether that actor is a monster or a party member."
#[test]
fn auditfix1_monster_failed_ranged_roll_runs_the_hit_chain_on_the_scatter_victim() {
    let class = auditfix1_ranged_monster_class();
    for seed in [0x0001u16, 0x1234, 0xbeef, 0x7777] {
        let mut state = auditfix1_monster_scatter_state(class);
        // The aim cell is party member 0's (5,5); every other cell of the
        // block holds somebody, so any landing cell reports an occupant.
        let mut victims = auditfix1_ring_of_rats(&mut state, (5, 5));
        victims.push(0);
        state.prng_state = seed;
        let serial = state.sound_effect_serial;
        let attack = state
            .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(false))
            .expect("the attack resolves");
        assert!(
            matches!(
                attack.resolution,
                Some(CombatWeaponAttackResolution::Hit { .. })
            ),
            "seed {seed:#x}: a scatter onto an occupied cell runs the hit chain, got {:?}",
            attack.resolution
        );
        assert!(
            victims.contains(&attack.target_slot),
            "the victim stands in the block"
        );
        assert!(
            attack.damage_application.is_some(),
            "the hit chain applies damage"
        );
        assert_eq!(
            state
                .sound_effects_after(serial)
                .iter()
                .filter(|cue| **cue == SoundEffect::MonsterAttackSwing)
                .count(),
            1,
            "the swing played before the roll and the scatter adds no second one"
        );
    }
}

/// `combat.md §11.1`: "The attack stays silent in exactly three cases ...
/// the landing cell is empty", and "The aim cell is therefore itself a legal
/// landing cell."
#[test]
fn auditfix1_monster_failed_ranged_roll_is_silent_only_when_the_landing_cell_is_empty() {
    let class = auditfix1_ranged_monster_class();
    let (mut silent, mut aim_cell_hits) = (0, 0);
    for seed in 0..64u16 {
        let mut state = auditfix1_monster_scatter_state(class);
        state.prng_state = seed.wrapping_mul(977).wrapping_add(1);
        let before = state.prng_state;
        let attack = state
            .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(false))
            .expect("the attack resolves");
        assert_ne!(state.prng_state, before, "the scatter draw is taken");
        match attack.resolution {
            Some(CombatWeaponAttackResolution::Miss { .. }) => {
                assert_eq!(attack.damage_application, None);
                silent += 1;
            }
            Some(CombatWeaponAttackResolution::Hit { .. }) => {
                assert_eq!(attack.target_slot, 0, "only the aim cell is occupied");
                aim_cell_hits += 1;
            }
            other => panic!("unexpected resolution {other:?}"),
        }
    }
    assert!(silent > 0, "an empty landing cell keeps the miss silent");
    assert!(
        aim_cell_hits > 0,
        "a landing on the aim cell hits the aimed actor"
    );
}

fn auditfix1_party_bow_state() -> PlayState {
    let mut state = combat_player_command_state(8, 5);
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = 26; // Bow
    state.combat_actors[8].hp_or_wound = 200;
    state
}

fn auditfix1_party_fire_forced_miss(state: &mut PlayState) -> CombatAttackWalkApplication {
    assert!(state.begin_combat_attack_walk(0, true).cursor_open);
    for _ in 0..3 {
        let _ = state.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST));
    }
    assert_eq!(state.combat_aim_marker_cell, Some((8, 5)));
    state
        .apply_combat_targeting_cursor_key_with_inputs(
            '\r',
            Some(CombatPlayerWeaponAttackInputs {
                hit_raw_roll_0_to_60: 0,
                damage_roll: Some(5),
                forced_hit: Some(false),
            }),
        )
        .expect("a cursor is open")
}

/// `combat.md §11.1`: "The party ranged and thrown arm has the same gap ...
/// When the roll fails and the scattered shot lands on somebody, the hit
/// chain runs and the miss line is skipped entirely."
#[test]
fn auditfix1_party_failed_ranged_roll_hits_the_scatter_victim_instead_of_missing() {
    for seed in [0x0001u16, 0x1234, 0xbeef, 0x7777] {
        let mut state = auditfix1_party_bow_state();
        let mut victims = auditfix1_ring_of_rats(&mut state, (8, 5));
        victims.push(8);
        state.prng_state = seed;
        let walk = auditfix1_party_fire_forced_miss(&mut state);
        let (victim, attack) = walk.attack.expect("the confirm resolved an attack");
        assert!(
            matches!(attack.resolution, CombatWeaponAttackResolution::Hit { .. }),
            "seed {seed:#x}: the scattered shot hits, got {:?}",
            attack.resolution
        );
        assert!(victims.contains(&victim));
        assert!(attack.damage_application.is_some());
        let line = crate::input_dispatch::combat_weapon_attack_narrated_result_message(
            &mut state, victim, attack,
        );
        assert!(
            !line.as_deref().unwrap_or_default().contains("missed!"),
            "the miss line is skipped: {line:?}"
        );
    }
}

/// `combat.md §11.1`: the party miss line "prints only when the resolver
/// reports nobody **and** the originally aimed cell held a real actor - and
/// it names that originally aimed actor".
#[test]
fn auditfix1_party_failed_ranged_roll_misses_the_aimed_actor_only_on_an_empty_landing() {
    let (mut missed, mut aim_cell_hits) = (0, 0);
    for seed in 0..64u16 {
        let mut state = auditfix1_party_bow_state();
        state.prng_state = seed.wrapping_mul(977).wrapping_add(1);
        let walk = auditfix1_party_fire_forced_miss(&mut state);
        let (target, attack) = walk.attack.expect("the confirm resolved an attack");
        assert_eq!(target, 8, "only the aimed actor can be named or struck");
        match attack.resolution {
            CombatWeaponAttackResolution::Miss { .. } => missed += 1,
            CombatWeaponAttackResolution::Hit { .. } => aim_cell_hits += 1,
            other => panic!("unexpected resolution {other:?}"),
        }
    }
    assert!(
        missed > 0 && aim_cell_hits > 0,
        "missed {missed}, aim-cell hits {aim_cell_hits}"
    );
}

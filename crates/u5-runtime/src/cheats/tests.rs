//! Chapter 7's runtime rules (design 7.2.4, 7.2.5, 7.3.3, 7.3.8; build plan
//! M8-01): the limits table, every quick action on a party of G/P/S/D/A
//! members in and out of combat, aboard and on foot, the locks, the slot-0
//! mirrors through a save, and the change watch.

use std::collections::BTreeSet;
use std::fs;

use super::*;
use crate::test_fixtures::{
    debug_game_dir, open_grid, open_world_grid, saved_game_seed_bytes, test_state, workspace_file,
    world_state, write_empty_ool_mirrors,
};
use crate::text_hold::PendingTextHold;

fn member(slot: u8, class_byte: u8, status: u8, hp: u16, max_hp: u16, mana: u8) -> PartyMember {
    PartyMember {
        slot,
        class_byte,
        status,
        climb_stat: 20,
        mana,
        hp,
        max_hp,
        level: 3,
    }
}

/// Five travelling members: Good, Poisoned, Sleeping, Dead and Ashes. The
/// Ashes member also carries an out-of-range STR of 45.
fn with_party(mut state: PlayState) -> PlayState {
    state.party = vec![
        member(0, b'A', b'G', 50, 240, 3),
        member(1, b'B', b'P', 60, 180, 1),
        member(2, b'M', b'S', 70, 150, 0),
        member(3, b'F', b'D', 0, 150, 0),
        member(4, b'R', b'A', 0, 120, 0),
    ];
    let len = state.party.len();
    let mut names = vec![[0u8; SAVE_CHARACTER_NAME_LEN]; len];
    for (index, name) in names.iter_mut().enumerate() {
        name[..2].copy_from_slice(&[b'M', b'0' + index as u8]);
    }
    state.party_names = names;
    state.party_experience = vec![100, 200, 300, 400, 500];
    state.party_stay_counters = vec![0; len];
    state.avatar_stats = AvatarStats {
        strength: 20,
        dexterity: 20,
        intelligence: 24,
    };
    state.party_strengths = vec![20, 21, 22, 23, 45];
    state.party_intelligence = vec![24, 16, 27, 10, 12];
    state.party_combat_defense = vec![0; len];
    state.party_equipment = vec![[EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]; len];
    state.party_roster = Vec::new();
    state
}

fn town() -> PlayState {
    with_party(test_state(open_grid(), 5, 5))
}

fn world() -> PlayState {
    with_party(world_state(open_world_grid(), 10, 20))
}

fn aboard(hull: u8) -> PlayState {
    let mut state = world();
    state.player.transport = TransportState::Ship {
        type_byte: SHIP_TRANSPORT_FURLED_FIRST,
        tile: SHIP_TRANSPORT_FURLED_FIRST,
        sails_hoisted: false,
        hull,
        skiffs: 1,
    };
    state.sync_player_object();
    state
}

/// A combat member's bare command prompt (design 1.3.7).
fn combat() -> PlayState {
    let mut state = world();
    state.combat_active = true;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.pending_combat_actor_slot = Some(0);
    state
}

fn fixtures() -> Vec<(&'static str, PlayState)> {
    vec![
        ("town", town()),
        ("world", world()),
        ("aboard", aboard(40)),
        ("combat", combat()),
    ]
}

const EDITS: &[CheatEdit] = &[
    CheatEdit::HealAll,
    CheatEdit::CurePoison,
    CheatEdit::ResurrectAll,
    CheatEdit::MaxGold,
    CheatEdit::MaxFood,
    CheatEdit::RefillArrows { quarrels: false },
    CheatEdit::RefillArrows { quarrels: true },
    CheatEdit::MaxReagents,
    CheatEdit::RepairHull { parked: false },
    CheatEdit::RepairHull { parked: true },
];

/// The fields that changed, from the whole watch snapshot.
fn changed_fields(before: &PlayState, after: &PlayState) -> BTreeSet<WatchField> {
    watch_diff(&watch_snapshot(before), &watch_snapshot(after))
        .into_iter()
        .map(|(field, _, _)| field)
        .collect()
}

fn fields(list: &[WatchField]) -> BTreeSet<WatchField> {
    list.iter().copied().collect()
}

fn statuses(state: &PlayState) -> Vec<u8> {
    state.party.iter().map(|member| member.status).collect()
}

fn hps(state: &PlayState) -> Vec<u16> {
    state.party.iter().map(|member| member.hp).collect()
}

// --- the limits table -------------------------------------------------------

#[test]
fn every_limit_reuses_the_engine_anchor() {
    assert_eq!(CheatLimit::Food.range(), (0, 9999));
    assert_eq!(CHEAT_FOOD_MAX, PARTY_FOOD_CAP);
    assert_eq!(CheatLimit::Gold.range(), (0, 9999));
    assert_eq!(CHEAT_GOLD_MAX, PARTY_GOLD_CAP);
    assert_eq!(CheatLimit::ByteStock.range(), (0, 99));
    assert_eq!(CHEAT_BYTE_STOCK_MAX, PARTY_BYTE_STOCK_CAP);
    assert_eq!(CheatLimit::PrimaryStat.range(), (1, 30));
    assert_eq!(CHEAT_STAT_MAX, AVATAR_STAT_MAX);
    assert_eq!(CheatLimit::Experience.range(), (0, 9999));
    assert_eq!(CHEAT_EXPERIENCE_MAX, COMBAT_EXPERIENCE_CAP);
    assert_eq!(CheatLimit::Hull.range(), (0, 99));
    assert_eq!(CHEAT_HULL_TARGET, FRIGATE_PURCHASE_HULL);
    assert_eq!(CHEAT_PARTY_FRIGATE_TYPES, 0x20..=0x27);
    assert!(!CHEAT_PARTY_FRIGATE_TYPES.contains(&OUTDOOR_PIRATE_TYPE_FIRST));
    assert!(!CHEAT_PARTY_FRIGATE_TYPES.contains(&OUTDOOR_PIRATE_TYPE_LAST));
    for limit in CHEAT_LIMITS {
        assert!(limit.source().starts_with("systems/"), "{limit:?}");
        let (min, max) = limit.range();
        assert_eq!(limit.check(min), Ok(()));
        assert_eq!(limit.check(max), Ok(()));
        assert_eq!(
            limit.check(max + 1),
            Err(CheatRefusal::OutOfRange { min, max })
        );
        if min > 0 {
            assert_eq!(
                limit.check(min - 1),
                Err(CheatRefusal::OutOfRange { min, max })
            );
        }
    }
}

#[test]
fn level_hp_and_mana_follow_the_camp_rules() {
    for (xp, level) in [(0, 1), (99, 1), (100, 2), (6399, 7), (6400, 8), (9999, 8)] {
        assert_eq!(cheat_level_for_experience(xp), level, "{xp}");
        assert_eq!(cheat_hp_for_level(level), 30 * u16::from(level));
    }
    assert_eq!(cheat_mana_target(b'A', 25), Some(25));
    assert_eq!(cheat_mana_target(b'M', 25), Some(25));
    assert_eq!(cheat_mana_target(b'B', 25), Some(12));
    assert_eq!(cheat_mana_target(b'F', 25), None);
}

// --- quick actions ------------------------------------------------------------

#[test]
fn heal_all_on_foot_outside_combat() {
    let before = world();
    let mut state = before.clone();
    let report = apply_cheat_edit(&mut state, &CheatEdit::HealAll).unwrap();
    assert_eq!(report.members_touched, 3);
    assert!(report.changed && report.panel_refresh);
    assert_eq!(hps(&state), [240, 180, 150, 0, 0]);
    assert_eq!(statuses(&state), *b"GGGDA");
    assert_eq!(
        changed_fields(&before, &state),
        fields(&[
            WatchField::MemberStatus(1),
            WatchField::MemberStatus(2),
            WatchField::MemberHp(0),
            WatchField::MemberHp(1),
            WatchField::MemberHp(2),
        ])
    );
    assert_eq!(
        apply_cheat_edit(&mut state, &CheatEdit::HealAll),
        Err(CheatRefusal::NothingToDo)
    );
}

#[test]
fn heal_all_in_combat_raises_hp_only() {
    let before = combat();
    let mut state = before.clone();
    apply_cheat_edit(&mut state, &CheatEdit::HealAll).unwrap();
    assert_eq!(hps(&state), [240, 180, 150, 0, 0]);
    assert_eq!(statuses(&state), *b"GPSDA", "status untouched in combat");
    assert_eq!(
        changed_fields(&before, &state),
        fields(&[
            WatchField::MemberHp(0),
            WatchField::MemberHp(1),
            WatchField::MemberHp(2),
        ])
    );
}

#[test]
fn heal_all_aboard_repairs_the_hull_and_mirrors_slot_zero() {
    let before = aboard(40);
    let mut state = before.clone();
    apply_cheat_edit(&mut state, &CheatEdit::HealAll).unwrap();
    assert!(matches!(
        state.player.transport,
        TransportState::Ship { hull: 99, .. }
    ));
    assert_eq!(state.active_objects[0].aux1, 99, "sync_player_object ran");
    assert!(changed_fields(&before, &state).contains(&WatchField::Hull));
}

#[test]
fn cure_poison_cures_only_p_and_refuses_in_combat() {
    let before = town();
    let mut state = before.clone();
    let report = apply_cheat_edit(&mut state, &CheatEdit::CurePoison).unwrap();
    assert_eq!(report.members_touched, 1);
    assert_eq!(statuses(&state), *b"GGSDA");
    assert_eq!(hps(&state), hps(&before), "HP unchanged");
    assert_eq!(
        changed_fields(&before, &state),
        fields(&[WatchField::MemberStatus(1)])
    );
    assert_eq!(
        apply_cheat_edit(&mut state, &CheatEdit::CurePoison),
        Err(CheatRefusal::NothingToDo)
    );
    let mut fight = combat();
    let snapshot = format!("{fight:?}");
    assert_eq!(
        apply_cheat_edit(&mut fight, &CheatEdit::CurePoison),
        Err(CheatRefusal::InCombat)
    );
    assert_eq!(format!("{fight:?}"), snapshot);
}

#[test]
fn resurrect_all_raises_exactly_the_dead_and_keeps_ashes() {
    let before = town();
    let mut state = before.clone();
    let report = apply_cheat_edit(&mut state, &CheatEdit::ResurrectAll).unwrap();
    assert_eq!(report.members_touched, 1);
    assert_eq!(statuses(&state), *b"GPSGA");
    assert_eq!(state.party[3].hp, 150);
    assert_eq!(state.party[4], before.party[4], "Ashes is preserved");
    assert_eq!(
        changed_fields(&before, &state),
        fields(&[WatchField::MemberStatus(3), WatchField::MemberHp(3)]),
        "level, XP and max HP unchanged"
    );
    let mut fight = combat();
    assert_eq!(
        apply_cheat_edit(&mut fight, &CheatEdit::ResurrectAll),
        Err(CheatRefusal::InCombat)
    );
}

#[test]
fn stores_rise_to_their_caps_and_never_fall() {
    for (label, before) in fixtures() {
        let mut state = before.clone();
        state.gold = 10;
        state.food = 10;
        state.reagents[3] = 120; // out of range from an edited save
        apply_cheat_edit(&mut state, &CheatEdit::MaxGold).unwrap();
        apply_cheat_edit(&mut state, &CheatEdit::MaxFood).unwrap();
        apply_cheat_edit(&mut state, &CheatEdit::MaxReagents).unwrap();
        assert_eq!((state.gold, state.food), (9999, 9999), "{label}");
        assert_eq!(state.reagents[3], 120, "{label}: never lowered");
        assert!(
            state
                .reagents
                .iter()
                .enumerate()
                .all(|(i, n)| i == 3 || *n == 99),
            "{label}"
        );
        for edit in [
            CheatEdit::MaxGold,
            CheatEdit::MaxFood,
            CheatEdit::MaxReagents,
        ] {
            assert_eq!(
                apply_cheat_edit(&mut state, &edit),
                Err(CheatRefusal::NothingToDo),
                "{label} {edit:?}"
            );
        }
        let mut rich = before.clone();
        rich.gold = 12_000;
        assert_eq!(
            apply_cheat_edit(&mut rich, &CheatEdit::MaxGold),
            Err(CheatRefusal::NothingToDo)
        );
        assert_eq!(rich.gold, 12_000);
    }
}

#[test]
fn refill_arrows_touches_quarrels_only_when_asked() {
    let mut state = world();
    state.equipment_stock[EQUIPMENT_ID_ARROWS] = 3;
    state.equipment_stock[EQUIPMENT_ID_QUARRELS] = 4;
    let before = state.clone();
    apply_cheat_edit(&mut state, &CheatEdit::RefillArrows { quarrels: false }).unwrap();
    assert_eq!(state.equipment_stock[EQUIPMENT_ID_ARROWS], 99);
    assert_eq!(state.equipment_stock[EQUIPMENT_ID_QUARRELS], 4);
    assert_eq!(
        changed_fields(&before, &state),
        fields(&[WatchField::Arrows])
    );
    apply_cheat_edit(&mut state, &CheatEdit::RefillArrows { quarrels: true }).unwrap();
    assert_eq!(state.equipment_stock[EQUIPMENT_ID_QUARRELS], 99);
    let mut changed = before.equipment_stock;
    changed[EQUIPMENT_ID_ARROWS] = 99;
    changed[EQUIPMENT_ID_QUARRELS] = 99;
    assert_eq!(state.equipment_stock, changed, "no other item");
}

fn parked(type_byte: u8, hull: u8) -> ActiveObject {
    ActiveObject {
        type_byte,
        tile: type_byte,
        x: 12,
        y: 20,
        z: 0,
        phase: 0,
        aux1: hull,
        aux3: 1,
    }
}

#[test]
fn repair_hull_on_foot_repairs_parked_frigates_only() {
    let mut state = world();
    state.active_objects.extend([
        parked(0x20, 30),
        parked(0x27, 100),
        parked(OUTDOOR_PIRATE_TYPE_FIRST, 30),
        parked(OUTDOOR_PIRATE_TYPE_LAST, 5),
    ]);
    let before = state.clone();
    assert_eq!(
        apply_cheat_edit(&mut state, &CheatEdit::RepairHull { parked: false }),
        Err(CheatRefusal::NotAboard)
    );
    assert_eq!(format!("{state:?}"), format!("{before:?}"));
    apply_cheat_edit(&mut state, &CheatEdit::RepairHull { parked: true }).unwrap();
    let hulls: Vec<u8> = state.active_objects.iter().map(|o| o.aux1).collect();
    let mut expected: Vec<u8> = before.active_objects.iter().map(|o| o.aux1).collect();
    expected[1] = 99;
    assert_eq!(
        hulls, expected,
        "100 is left alone; pirates are never repaired"
    );
    // Nothing left to repair, and not aboard.
    assert_eq!(
        apply_cheat_edit(&mut state, &CheatEdit::RepairHull { parked: true }),
        Err(CheatRefusal::NotAboard)
    );
    // In combat the table is the arena's: nothing parked is touched.
    let mut fight = combat();
    fight.active_objects.push(parked(0x24, 30));
    assert_eq!(
        apply_cheat_edit(&mut fight, &CheatEdit::RepairHull { parked: true }),
        Err(CheatRefusal::NotAboard)
    );
    assert_eq!(fight.active_objects.last().unwrap().aux1, 30);
}

/// 7.2.4: no action raises a hull to 100 or more, and one already at 99 or
/// above never changes.
#[test]
fn no_hull_ever_reaches_one_hundred() {
    for hull in [0u8, 1, 50, 98, 99, 100, 150, 255] {
        for edit in [
            CheatEdit::HealAll,
            CheatEdit::RepairHull { parked: false },
            CheatEdit::RepairHull { parked: true },
        ] {
            let mut state = aboard(hull);
            state.active_objects.push(parked(0x22, hull));
            let _ = apply_cheat_edit(&mut state, &edit);
            let TransportState::Ship { hull: after, .. } = state.player.transport else {
                panic!("still aboard");
            };
            let expected = if hull >= 99 { hull } else { 99 };
            assert_eq!(after, expected, "{hull} {edit:?}");
            let parked_after = state.active_objects.last().unwrap().aux1;
            assert!(
                parked_after == hull || parked_after == 99,
                "{hull} {edit:?}"
            );
            assert!(parked_after < 100 || parked_after == hull);
        }
    }
    let mut state = aboard(99);
    assert_eq!(
        apply_cheat_edit(&mut state, &CheatEdit::RepairHull { parked: false }),
        Err(CheatRefusal::NothingToDo)
    );
}

#[test]
fn every_edit_waits_for_the_idle_prompt() {
    for (label, mut state) in fixtures() {
        state.pending_text_hold = Some(PendingTextHold::default());
        let before = format!("{state:?}");
        for edit in EDITS {
            assert_eq!(
                apply_cheat_edit(&mut state, edit),
                Err(CheatRefusal::NotAtIdlePrompt),
                "{label} {edit:?}"
            );
        }
        assert!(!apply_locks(&mut state, &all_locks()));
        assert_eq!(format!("{state:?}"), before, "{label}");
    }
}

/// 7.2.4: no action draws a random number, advances the clock or runs a
/// turn.
#[test]
fn no_edit_touches_the_prng_the_turn_or_the_clock() {
    for (label, before) in fixtures() {
        for edit in EDITS {
            let mut state = before.clone();
            state.gold = 1;
            state.equipment_stock[EQUIPMENT_ID_ARROWS] = 1;
            let _ = apply_cheat_edit(&mut state, edit);
            assert_eq!(state.prng_state, before.prng_state, "{label} {edit:?}");
            assert_eq!(state.turn, before.turn, "{label} {edit:?}");
            assert_eq!(state.clock, before.clock, "{label} {edit:?}");
        }
        let mut state = before.clone();
        apply_locks(&mut state, &all_locks());
        assert_eq!(
            (state.prng_state, state.turn, state.clock),
            (before.prng_state, before.turn, before.clock),
            "{label}"
        );
    }
}

// --- locks ---------------------------------------------------------------------

fn all_locks() -> EngagedLocks {
    EngagedLocks {
        health: true,
        mana: true,
        food: true,
        gold: true,
        keys: true,
        gems: true,
        torches: true,
        arrows: true,
        quarrels: true,
        reagents: [true; REAGENT_COUNT],
    }
}

/// Each lock alone, with the fields it may change.
fn single_locks() -> Vec<(&'static str, EngagedLocks, Vec<WatchField>)> {
    let none = EngagedLocks::default();
    let members = |make: fn(u8) -> WatchField| (0..5).map(make).collect::<Vec<_>>();
    let mut health = members(WatchField::MemberHp);
    health.extend(members(WatchField::MemberStatus));
    let mut reagents = none;
    reagents.reagents[2] = true;
    vec![
        (
            "health",
            EngagedLocks {
                health: true,
                ..none
            },
            health,
        ),
        (
            "mana",
            EngagedLocks { mana: true, ..none },
            members(WatchField::MemberMp),
        ),
        (
            "food",
            EngagedLocks { food: true, ..none },
            vec![WatchField::Food],
        ),
        (
            "gold",
            EngagedLocks { gold: true, ..none },
            vec![WatchField::Gold],
        ),
        (
            "keys",
            EngagedLocks { keys: true, ..none },
            vec![WatchField::Keys],
        ),
        (
            "gems",
            EngagedLocks { gems: true, ..none },
            vec![WatchField::Gems],
        ),
        (
            "torches",
            EngagedLocks {
                torches: true,
                ..none
            },
            vec![WatchField::Torches],
        ),
        (
            "arrows",
            EngagedLocks {
                arrows: true,
                ..none
            },
            vec![WatchField::Arrows],
        ),
        ("reagent 3", reagents, vec![WatchField::Reagent(2)]),
    ]
}

/// A state with every counter low, so every lock has work.
fn low(mut state: PlayState) -> PlayState {
    state.food = 1;
    state.gold = 2;
    state.keys = 3;
    state.gems = 4;
    state.torches = 5;
    state.equipment_stock[EQUIPMENT_ID_ARROWS] = 6;
    state.equipment_stock[EQUIPMENT_ID_QUARRELS] = 7;
    state.reagents = [8; REAGENT_COUNT];
    state
}

#[test]
fn every_lock_is_isolated_idempotent_and_monotone() {
    for (label, fixture) in fixtures() {
        for (lock, locks, allowed) in single_locks() {
            let before = low(fixture.clone());
            let mut state = before.clone();
            assert!(apply_locks(&mut state, &locks), "{label} {lock} has work");
            let changed = changed_fields(&before, &state);
            assert!(
                changed.is_subset(&fields(&allowed)),
                "{label} {lock}: {changed:?}"
            );
            // Idempotent: a second call changes nothing at all.
            let once = format!("{state:?}");
            assert!(!apply_locks(&mut state, &locks), "{label} {lock}");
            assert_eq!(format!("{state:?}"), once, "{label} {lock}");
            // Monotone: no number fell.
            for (field, old, new) in watch_diff(&watch_snapshot(&before), &watch_snapshot(&state)) {
                if let (WatchValue::Num(old), WatchValue::Num(new)) = (old, new) {
                    assert!(new > old, "{label} {lock}: {field:?} fell");
                }
            }
        }
    }
}

#[test]
fn the_health_lock_never_resurrects_and_keeps_status_in_combat() {
    let mut state = combat();
    assert!(apply_health_lock(
        &mut state.party,
        LockContext { in_combat: true }
    ));
    assert_eq!(statuses(&state), *b"GPSDA");
    assert_eq!(hps(&state), [240, 180, 150, 0, 0]);
    let mut state = town();
    apply_health_lock(&mut state.party, LockContext { in_combat: false });
    assert_eq!(statuses(&state), *b"GGGDA");
    // A value above max (an edited save) is not lowered.
    state.party[0].hp = 300;
    assert!(!apply_health_lock(
        &mut state.party,
        LockContext { in_combat: false }
    ));
    assert_eq!(state.party[0].hp, 300);
}

#[test]
fn the_mana_lock_raises_to_the_class_target() {
    let mut state = town();
    state.party[2].mana = 40; // above the Mage's target of 27
    assert!(apply_locks(
        &mut state,
        &EngagedLocks {
            mana: true,
            ..EngagedLocks::default()
        }
    ));
    let mana: Vec<u8> = state.party.iter().map(|m| m.mana).collect();
    // Avatar: INT from avatar_stats (24); Bard: 16 / 2; Mage kept at 40;
    // Fighter has no target; the dead are left alone.
    assert_eq!(mana, [24, 8, 40, 0, 0]);
}

#[test]
fn the_store_lock_works_on_plain_values() {
    let (mut food, mut gold, mut keys, mut gems, mut torches) = (0u16, 10_000u16, 0u8, 120u8, 0u8);
    let mut stock = [0u8; EQUIPMENT_COUNT];
    let mut reagents = [0u8; REAGENT_COUNT];
    let mut stores = StoreFields {
        food: &mut food,
        gold: &mut gold,
        keys: &mut keys,
        gems: &mut gems,
        torches: &mut torches,
        equipment_stock: &mut stock,
        reagents: &mut reagents,
    };
    let mut locks = all_locks();
    locks.quarrels = false;
    locks.reagents = [false; REAGENT_COUNT];
    locks.reagents[0] = true;
    assert!(apply_store_locks(&mut stores, &locks));
    assert!(!apply_store_locks(&mut stores, &locks));
    assert_eq!(
        (food, gold, keys, gems, torches),
        (9999, 10_000, 99, 120, 99)
    );
    assert_eq!(stock[EQUIPMENT_ID_ARROWS], 99);
    assert_eq!(
        stock[EQUIPMENT_ID_QUARRELS], 0,
        "quarrels need their switch"
    );
    assert_eq!(reagents, [99, 0, 0, 0, 0, 0, 0, 0]);
    assert!(all_locks().any());
    assert!(!EngagedLocks::default().any());
}

// --- setters and the save round trip ------------------------------------------

#[test]
fn slot_zero_setters_keep_every_mirror() {
    let mut state = town();
    assert_eq!(
        set_member_stat(&mut state, 0, PrimaryStat::Strength, 17),
        Ok(true)
    );
    set_member_stat(&mut state, 0, PrimaryStat::Dexterity, 18).unwrap();
    set_member_stat(&mut state, 0, PrimaryStat::Intelligence, 19).unwrap();
    assert_eq!(
        state.avatar_stats,
        AvatarStats {
            strength: 17,
            dexterity: 18,
            intelligence: 19
        }
    );
    assert_eq!(state.party_strengths[0], 17);
    assert_eq!(state.party[0].climb_stat, 18);
    assert_eq!(state.party_intelligence[0], 19);
    assert_eq!(
        set_member_stat(&mut state, 0, PrimaryStat::Strength, 17),
        Ok(false)
    );

    set_member_stat(&mut state, 2, PrimaryStat::Dexterity, 9).unwrap();
    assert_eq!(state.party[2].climb_stat, 9);
    assert_eq!(state.avatar_stats.dexterity, 18, "slot 2 leaves slot 0");

    for bad in [0, 31] {
        assert_eq!(
            set_member_stat(&mut state, 1, PrimaryStat::Strength, bad),
            Err(CheatRefusal::OutOfRange { min: 1, max: 30 })
        );
    }
    assert_eq!(
        set_member_stat(&mut state, 5, PrimaryStat::Strength, 10),
        Err(CheatRefusal::NoSuchMember)
    );
}

/// Slot-0 mirrors, an Ashes member and an out-of-range STR, through a real
/// save and `load_play_options_from_save`.
#[test]
fn mirrors_ashes_and_out_of_range_values_survive_a_save() {
    let mut template = saved_game_seed_bytes(17, 0, 15, 15);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    let dir = debug_game_dir();
    fs::write(dir.join(SAVED_GAM_FILENAME), template).unwrap();
    fs::write(dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN]).unwrap();
    write_empty_ool_mirrors(&dir);

    let mut state = world();
    set_member_stat(&mut state, 0, PrimaryStat::Strength, 17).unwrap();
    set_member_stat(&mut state, 0, PrimaryStat::Dexterity, 18).unwrap();
    set_member_stat(&mut state, 0, PrimaryStat::Intelligence, 19).unwrap();
    set_member_stat(&mut state, 1, PrimaryStat::Strength, 12).unwrap();
    apply_cheat_edit(&mut state, &CheatEdit::HealAll).unwrap();
    apply_cheat_edit(&mut state, &CheatEdit::ResurrectAll).unwrap();
    assert_eq!(
        state.save_game_command(&dir, Some(true)).unwrap(),
        MoveOutcome::Saved
    );

    let options = load_play_options_from_save(&dir).unwrap();
    assert_eq!(
        options.avatar_stats,
        AvatarStats {
            strength: 17,
            dexterity: 18,
            intelligence: 19
        }
    );
    assert_eq!(options.party.len(), 5);
    assert_eq!(options.party[0].climb_stat, 18);
    assert_eq!(options.party_strengths[0], 17);
    assert_eq!(options.party_intelligence[0], 19);
    assert_eq!(options.party_strengths[1], 12);
    let loaded: Vec<u8> = options.party.iter().map(|m| m.status).collect();
    assert_eq!(loaded, *b"GGGGA", "Ashes is not resurrected");
    let loaded_hp: Vec<u16> = options.party.iter().map(|m| m.hp).collect();
    assert_eq!(loaded_hp, [240, 180, 150, 150, 0]);
    assert_eq!(options.party_strengths[4], 45, "out-of-range STR kept");
    let _ = fs::remove_dir_all(dir);
}

// --- the change watch -------------------------------------------------------------

#[test]
fn the_snapshot_is_ordered_and_pure() {
    for (label, state) in fixtures() {
        let before = format!("{state:?}");
        let snapshot = watch_snapshot(&state);
        assert_eq!(format!("{state:?}"), before, "{label}");
        assert!(
            snapshot.windows(2).all(|pair| pair[0].0 < pair[1].0),
            "{label}: in WatchField order, no repeats"
        );
        assert_eq!(watch_diff(&snapshot, &snapshot), Vec::new());
        let hull = snapshot.iter().any(|(field, _)| *field == WatchField::Hull);
        assert_eq!(hull, label == "aboard", "{label}");
        for (field, value) in &snapshot {
            assert!(!field.label().is_empty());
            assert!(!value.text().is_empty(), "{label} {field:?}");
        }
    }
}

#[test]
fn the_diff_reports_changes_and_absent_fields() {
    use WatchValue::{Num, Text};
    let old = vec![
        (WatchField::Food, Num(211)),
        (WatchField::Hull, Num(40)),
        (WatchField::MemberStatus(0), Text("G".into())),
    ];
    let new = vec![
        (WatchField::Food, Num(210)),
        (WatchField::MemberStatus(0), Text("G".into())),
        (WatchField::MemberHp(1), Num(5)),
    ];
    assert_eq!(
        watch_diff(&old, &new),
        vec![
            (WatchField::Hull, Num(40), WatchValue::absent()),
            (WatchField::Food, Num(211), Num(210)),
            (WatchField::MemberHp(1), WatchValue::absent(), Num(5)),
        ]
    );
}

#[test]
fn the_engine_group_holds_the_prng_and_the_turn() {
    assert_eq!(WatchField::Prng.group(), WatchGroup::Engine);
    assert_eq!(WatchField::Turn.group(), WatchGroup::Engine);
    assert_eq!(WatchField::MemberHp(0).label(), "Party 1 HP");
    assert_eq!(WatchField::Reagent(7).label(), "Reagent 8");
}

// --- clean-room and zero-footprint checks -----------------------------------------

/// The module's own code (its tests excluded).
fn module_source() -> String {
    let text = fs::read_to_string(workspace_file("crates/u5-runtime/src/cheats.rs")).unwrap();
    let end = text.find("#[cfg(test)]").expect("the tests marker");
    text[..end].to_string()
}

/// No ninth-virtue names, no save offsets, no raw byte literals and no
/// game-text lines: every limit comes from a named engine constant and
/// every string is an engine label.
#[test]
fn the_module_names_no_trainer_offsets_or_game_text() {
    let source = module_source();
    let lower = source.to_ascii_lowercase();
    for banned in [
        "ninth", "virtue", "_offset", "save_", "0x", "\\n", "_line", "_message",
    ] {
        assert!(!lower.contains(banned), "cheats.rs contains {banned:?}");
    }
}

/// 7.4.2: `cheats::` is referenced only by tests (the plugin arrives in
/// M8-02).
#[test]
fn nothing_but_tests_calls_the_module() {
    let crates = workspace_file("crates");
    let mut offenders = Vec::new();
    let mut stack = vec![crates];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|ext| ext != "rs") {
                continue;
            }
            let text = path.to_string_lossy().replace('\\', "/");
            let is_test = text.contains("/tests/")
                || text.contains("/tests_inline/")
                || text.ends_with("/tests.rs")
                || text.ends_with("_tests.rs");
            let is_module = text.ends_with("u5-runtime/src/cheats.rs");
            if is_test || is_module {
                continue;
            }
            if fs::read_to_string(&path).unwrap().contains("cheats::") {
                offenders.push(text);
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "cheats:: used outside tests: {offenders:?}"
    );
}

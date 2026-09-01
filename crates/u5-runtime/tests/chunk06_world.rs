//! Spec-gap conformance tests for the `chunk06-world` batch.
//!
//! Each test names the published contract it pins:
//!
//! * `systems/traps.md §4` — the surface/town container combat-scene cleanup.
//! * `systems/lighting.md §8` / `systems/containers.md §7` — the G-Get
//!   "borrow a lit fixture" torch-counter writer.
//! * `systems/containers.md §9` — corpse-search arm split, slot fate, and
//!   the corpse-search odds table.

use std::fs;

use u5_runtime::test_fixtures::{debug_game_dir, world_state};
use u5_runtime::*;

fn open_underworld_grid() -> Vec<u8> {
    vec![5; WORLD_CELLS]
}

// ---------------------------------------------------------------------------
// `traps.md §4`: surface/town container combat-scene cleanup
// ---------------------------------------------------------------------------

/// Builds a world state carrying a trapped surface object chest at
/// `(6, 5)` and a combat actor record for party slot 0 linked to active
/// object slot 2.
fn trapped_chest_combat_state() -> PlayState {
    let mut state = world_state(open_underworld_grid(), 5, 5);
    let z = WorldPlane::Underworld.save_floor();
    // Slot 1: the trapped container. Stat high bit set == trap armed.
    state.active_objects.push(ActiveObject {
        type_byte: TILE_FURNITURE_FIRST,
        tile: TILE_FURNITURE_FIRST,
        x: 6,
        y: 5,
        z,
        phase: STEADY_PHASE,
        aux1: 0x85,
        aux3: 0,
    });
    // Slot 2: the world-object entry the combatant record points at.
    state.active_objects.push(ActiveObject {
        type_byte: 0x40,
        tile: 0x40,
        x: 5,
        y: 5,
        z,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state.combat_actors[0] = CombatActorDescriptor::from_row([9, 1, 0, 0, 2, STEADY_PHASE, 5, 5]);
    state.active_player = Some(0);
    // `traps.md §4` guards on the chosen member being Dead *after* the
    // resolver returns, so the fixture puts the slot there ahead of the
    // open; the shared resolver leaves an already-dead slot at zero HP
    // whichever effect family it rolls.
    state.party[0].hp = 0;
    state
}

#[test]
fn surface_container_trap_stamps_corpse_into_the_linked_world_object_in_combat() {
    // `traps.md §4`: "The surface/town site alone performs a combat-scene
    // cleanup after the resolver returns: if the chosen member is then Dead,
    // it (a) finds that member's live combatant record and **sets a marker
    // bit on it**, (b) **stamps a fixed non-zero marker value into the
    // leading bytes of the world-object entry that record points at**, and
    // (c) clears the active-character hint when that hint named the member."
    //
    // "The constant ... is **decimal thirty**, written as two separate byte
    // stores of the same value into **both leading bytes** of the record,
    // and it is a **corpse**" — written "as a corpse-class object, not ...
    // a tile".
    let mut state = trapped_chest_combat_state();
    state.combat_active = true;

    assert_eq!(
        state.consume_surface_object_chest_at(6, 5, 0, "Opened"),
        Some(MoveOutcome::ContainerOpened)
    );

    // (a) marker bit set, record NOT removed or freed — §4 withdraws the
    // "removes that member from the live combatant records" reading.
    assert!(state.combat_actors[0].is_marked_dead());
    assert_eq!(state.combat_actors[0].active_object_slot, 2);
    assert_eq!(state.combat_actors[0].hp_or_wound, 9);
    assert!(!state.combat_actors[0].is_empty());

    // (b) decimal thirty into both leading bytes, as a corpse-class object.
    assert_eq!(COMBAT_PARTY_CORPSE_TILE, 30);
    assert_eq!(state.active_objects[2].type_byte, COMBAT_PARTY_CORPSE_TILE);
    assert_eq!(state.active_objects[2].tile, COMBAT_PARTY_CORPSE_TILE);
    // §4: the entry is **not** blanked — its position survives.
    assert_eq!(
        (state.active_objects[2].x, state.active_objects[2].y),
        (5, 5)
    );

    // (c) active-character hint cleared.
    assert_eq!(state.active_player, None);
}

#[test]
fn surface_container_trap_outside_combat_leaves_the_world_object_alone() {
    // `traps.md §4` fourth difference: the `O` Open dispatcher "routes every
    // other scene - combat-class scenes included - to the surface/town
    // handler", and "only the surface/town handler carries combat-scene code
    // at all". With no combat scene live there is no combatant record to
    // clean up, so nothing is stamped.
    let mut state = trapped_chest_combat_state();
    state.combat_active = false;

    assert_eq!(
        state.consume_surface_object_chest_at(6, 5, 0, "Opened"),
        Some(MoveOutcome::ContainerOpened)
    );

    assert!(!state.combat_actors[0].is_marked_dead());
    assert_eq!(state.active_objects[2].type_byte, 0x40);
    assert_eq!(state.active_objects[2].tile, 0x40);
}

#[test]
fn surface_container_without_a_trap_never_reaches_the_combat_cleanup() {
    // `traps.md §4`: the cleanup runs "after the resolver returns". An
    // untrapped container never invokes the resolver.
    let mut state = trapped_chest_combat_state();
    state.combat_active = true;
    // Clear the trap flag, keeping the content class.
    state.active_objects[1].aux1 = 0x05;

    assert_eq!(
        state.consume_surface_object_chest_at(6, 5, 0, "Opened"),
        Some(MoveOutcome::ContainerOpened)
    );

    assert!(!state.combat_actors[0].is_marked_dead());
    assert_eq!(state.active_objects[2].type_byte, 0x40);
}

// ---------------------------------------------------------------------------
// `lighting.md §8` / `containers.md §7`: borrowed lit fixture
// ---------------------------------------------------------------------------

fn borrow_state(source_tile: u8) -> (PlayState, std::path::PathBuf) {
    let dir = debug_game_dir();
    fs::write(
        dir.join(WORLD_GET_TILE_TABLE_FILE),
        format!("underworld 6 5 0x44 0x{source_tile:02x}\n"),
    )
    .unwrap();
    let mut grid = open_underworld_grid();
    grid[world_cell_index(6, 5)] = source_tile;
    let mut state = world_state(grid, 5, 5);
    state.moral_standing = 40;
    state.torches = 4;
    state.torch_counter = 0;
    (state, dir)
}

#[test]
fn getting_a_lit_fixture_tile_sets_the_borrowed_fixture_torch_counter() {
    // `lighting.md §8`: "The G-Get "borrow" branch, which lifts a lit fixture
    // out of a town or castle cell, sets the torch counter to 100 counter
    // units and consumes no carried torch."
    //
    // `containers.md §7`: "the party's torch counter is set to 100 counter
    // units — borrowing a lit fixture is a light source, not an inventory
    // item, and it consumes no carried torch ... The traced branch does not
    // debit the shared moral-standing selector."
    //
    // `0xBC` is a published local-light source tile (`visibility.md §12`) and
    // `RETRACTIONS.md` R151 independently confirms it as a light source.
    let (mut state, dir) = borrow_state(0xbc);

    assert_eq!(
        state
            .get_world_direction(&dir, WorldPlane::Underworld, Direction::East)
            .unwrap(),
        MoveOutcome::Got
    );

    assert_eq!(state.torch_counter, BORROWED_FIXTURE_TORCH_DURATION);
    assert_eq!(state.torch_counter, 100);
    // Consumes no carried torch and adds no inventory item.
    assert_eq!(state.torches, 4);
    // Does not debit the shared moral-standing selector.
    assert_eq!(state.moral_standing, 40);
    // `lighting.md §4`: a non-zero torch counter floors the ambient value on
    // the same turn the counter is written.
    assert!(state.ambient_light >= TORCH_LIGHT_FLOOR);
}

#[test]
fn getting_an_ordinary_tile_leaves_the_torch_counter_alone() {
    // Only the borrow branch writes the counter: `lighting.md §8` names
    // I-Ignite, this branch, and the Blackthorn restoration as "the only
    // three torch-counter writers besides decay". `0x2E` is not in the
    // published light-source set, so it is ordinary tile handling.
    let (mut state, dir) = borrow_state(0x2e);

    assert_eq!(
        state
            .get_world_direction(&dir, WorldPlane::Underworld, Direction::East)
            .unwrap(),
        MoveOutcome::Got
    );

    assert_eq!(state.torch_counter, 0);
}

// ---------------------------------------------------------------------------
// `containers.md §9`: corpse searches and corpse-search odds
// ---------------------------------------------------------------------------

/// Record 40 of the fixed hidden-treasure table is a moldy-corpse row.
const CORPSE_RECORD: usize = 40;

fn corpse_search_state(seed: u16) -> PlayState {
    let mut state = world_state(open_underworld_grid(), 5, 5);
    state.prng_state = seed;
    state
        .active_objects
        .push(ActiveObject::fixed_hidden_treasure_pickup(
            CORPSE_RECORD,
            6,
            5,
            WorldPlane::Underworld.save_floor(),
        ));
    state
}

#[test]
fn corpse_search_fixture_is_really_on_the_corpse_path() {
    let mut state = corpse_search_state(1);
    assert_eq!(
        state.search_active_object_treasure_marker_at(6, 5),
        Some(MoveOutcome::Searched)
    );
    assert!(state.message.starts_with("Thou dost find\n"));
    assert!(state.message.ends_with('\n'));
}

#[test]
fn native_moldy_corpse_enters_the_branch_but_rotting_body_does_not() {
    let mut moldy = world_state(open_underworld_grid(), 5, 5);
    moldy.active_objects.push(ActiveObject {
        type_byte: 0x1f,
        tile: 0x1f,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 7,
        aux3: 0,
    });
    assert_eq!(moldy.moldy_corpse_search_slot_at(6, 5), Some(1));
    assert_eq!(
        moldy.search_active_object_treasure_marker_at(6, 5),
        Some(MoveOutcome::Searched)
    );
    assert!(moldy.message.starts_with("Thou dost find\n"));

    let mut rotting = world_state(open_underworld_grid(), 5, 5);
    rotting.active_objects.push(ActiveObject {
        type_byte: COMBAT_PARTY_CORPSE_TILE,
        tile: COMBAT_PARTY_CORPSE_TILE,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: STEADY_PHASE,
        aux1: 7,
        aux3: 0,
    });
    assert_eq!(rotting.moldy_corpse_search_slot_at(6, 5), None);
    assert_eq!(rotting.search_active_object_treasure_marker_at(6, 5), None);
}

/// Runs one corpse search per seed and reports `(minority_arm_count,
/// plague_count, nothing_counts)`.
fn corpse_search_census(samples: u16) -> (u32, u32, [u32; 4]) {
    let mut minority = 0;
    let mut plague = 0;
    let mut nothing = [0u32; 4];
    for seed in 1..=samples {
        let mut state = corpse_search_state(seed);
        state.search_active_object_treasure_marker_at(6, 5);
        let message = state.message.clone();
        let slot_cleared = state.active_objects[1].is_empty();
        if !slot_cleared {
            minority += 1;
            // The minority arm leaves the slot in place.
            assert!(matches!(
                message.as_str(),
                "Thou dost find\nfood!\n" | "Thou dost find\ngold!\n"
            ));
        } else {
            // Every majority-arm outcome — plague included — leaves the slot
            // cleared and stages neither food nor gold.
            assert!(slot_cleared, "majority arm kept the slot: {message}");
            if message == "Thou dost find\nPlague!\n" {
                plague += 1;
            } else if message == "Thou dost find\nnothing!\n" {
                nothing[0] += 1;
            } else if message == "Thou dost find\nworms!\n" {
                nothing[1] += 1;
            } else if message == "Thou dost find\nguts!\n" {
                nothing[2] += 1;
            } else if message == "Thou dost find\na bloody pulp!\n" {
                nothing[3] += 1;
            } else {
                panic!("unclassified corpse-search narration: {message}");
            }
        }
    }
    (minority, plague, nothing)
}

#[test]
fn corpse_search_arm_split_is_one_in_eight_and_owns_the_slot_fate() {
    // `containers.md §9`: "A corpse search splits on a single roll before
    // anything is narrated, and the fate of the corpse slot follows that
    // split, not the narration. The majority arm -- seven outcomes in eight
    // -- **clears the corpse slot first** ... Only the minority arm -- one
    // outcome in eight -- leaves the slot in place and rewrites it into a
    // later food/gold pickup."
    //
    // `RETRACTIONS.md` R200 reverses the older reading; this pins the
    // corrected one.
    const SAMPLES: u16 = 4000;
    let (minority, _, _) = corpse_search_census(SAMPLES);
    let share = f64::from(minority) / f64::from(SAMPLES);
    assert!(
        (0.10..=0.15).contains(&share),
        "minority arm share {share} is not the published 1 in 8 (observed {minority}/{SAMPLES})"
    );
}

#[test]
fn corpse_search_minority_arm_rewrites_the_slot_into_a_food_or_gold_pickup() {
    // `containers.md §9`: the minority arm "leaves the slot in place and
    // rewrites it into a later food/gold pickup; the eventual pickup then
    // follows the ordinary object-table grant rule above", split "1 in 4
    // food, 3 in 4 gold" — `containers.md §8` class codes `0x0F` food and
    // `0x02` gold.
    let mut food = 0u32;
    let mut gold = 0u32;
    for seed in 1..=4000u16 {
        let mut state = corpse_search_state(seed);
        state.search_active_object_treasure_marker_at(6, 5);
        if state.active_objects[1].is_empty() {
            continue;
        }
        let staged = state.active_objects[1];
        assert!(!staged.is_empty());
        assert_eq!((staged.x, staged.y), (6, 5));
        match staged.type_byte {
            0x0f => food += 1,
            0x02 => gold += 1,
            other => panic!("minority arm staged unexpected class {other:#04x}"),
        }
        assert!((1..=3).contains(&staged.aux1));
        // The staged object must no longer re-enter the corpse path.
        assert_eq!(staged.fixed_hidden_treasure_record(), None);
    }
    assert!(food > 0 && gold > 0);
    let food_share = f64::from(food) / f64::from(food + gold);
    assert!(
        (0.18..=0.32).contains(&food_share),
        "food share {food_share} is not the published 1 in 4 ({food} food, {gold} gold)"
    );
}

#[test]
fn corpse_search_cleared_arm_rolls_plague_at_one_in_thirty_two() {
    // `containers.md §9`: "Inside the cleared arm | Plague on **1 in 32**."
    // An earlier revision published one in thirty-one; that figure is
    // withdrawn.
    const SAMPLES: u16 = 8000;
    let (minority, plague, _) = corpse_search_census(SAMPLES);
    let cleared = u32::from(SAMPLES) - minority;
    let share = f64::from(plague) / f64::from(cleared);
    assert!(
        (0.020..=0.047).contains(&share),
        "plague share {share} is not the published 1 in 32 ({plague}/{cleared})"
    );
}

#[test]
fn corpse_search_plague_sounds_then_overwrites_the_selected_member_status() {
    let mut observed = false;
    for seed in 1..=8000u16 {
        let mut state = corpse_search_state(seed);
        // The active-member override admits imported statuses without a
        // status re-check; Plague overwrites that selected slot directly.
        state.active_player = Some(0);
        state.party[0].status = b'C';
        let hp_before = state.party[0].hp;
        let serial_before = state.sound_effect_serial;
        state.resolve_corpse_search(1, 0);
        if state.message != "Thou dost find\nPlague!\n" {
            continue;
        }
        observed = true;
        assert_eq!(state.party[0].status, b'P');
        assert_eq!(state.party[0].hp, hp_before);
        assert_eq!(state.active_player, Some(0));
        assert_eq!(
            state.sound_effects_after(serial_before),
            &[SoundEffect::CorpsePlagueRumble]
        );
        break;
    }
    assert!(observed, "seed census did not reach the Plague value 19");
}

#[test]
fn corpse_search_narration_uses_the_published_upper_then_selector_tree() {
    // `containers.md §9`: draw upper in 0..3, then selector in 0..upper.
    // This produces exact conditional shares 25/48, 13/48, 7/48, 3/48.
    for seed in 1..=8000u16 {
        let mut expected_state = seed;
        let arm = prng::u5_prng_range_u16(&mut expected_state, 0, 7);
        if arm == 0 {
            continue;
        }
        let plague = prng::u5_prng_range_u16(&mut expected_state, 0, 31);
        if plague == 19 {
            continue;
        }
        let upper = prng::u5_prng_range_u16(&mut expected_state, 0, 3);
        let selector = prng::u5_prng_range_u16(&mut expected_state, 0, upper) as usize;
        let expected = ["nothing!", "worms!", "guts!", "a bloody pulp!"][selector];

        let mut state = corpse_search_state(seed);
        state.search_active_object_treasure_marker_at(6, 5);
        assert_eq!(state.message, format!("Thou dost find\n{expected}\n"));
        assert_eq!(state.prng_state, expected_state);
    }
}

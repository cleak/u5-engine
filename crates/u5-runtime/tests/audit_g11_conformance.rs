//! Conformance tests for the 2026-10-16 audit, cluster g11
//! (`systems/containers.md`, `systems/inventory.md`,
//! `catalogs/quest-graph.md`, `systems/doors-and-z-transitions.md`,
//! `systems/rest-and-camp.md`). Each test names the clause it pins.

use std::path::Path;

use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::*;

fn town_chest_state(stat: u8) -> PlayState {
    let mut grid = open_grid();
    grid[32 + 2] = 0x4f;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    state.active_objects.push(ActiveObject {
        type_byte: 0x4f,
        tile: 0x4f,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: stat,
        aux3: 0,
    });
    state
}

fn staged_at(state: &PlayState, x: usize, y: usize) -> Vec<(usize, ActiveObject)> {
    state
        .active_objects
        .iter()
        .copied()
        .enumerate()
        .skip(1)
        .filter(|(_, object)| !object.is_empty() && (object.x, object.y) == (x, y))
        .collect()
}

/// `containers.md §4`: "Every successful row from either pool is committed
/// the same way: the engine acquires a free active-object slot, writes the
/// result family and the rolled quantity into it". The chest credits no
/// inventory; a later `G` Get collects each staged object.
#[test]
fn a060_surface_chest_rows_are_staged_as_floor_objects_for_a_later_get() {
    // Class 7 is eligible for Food, Torches and Gold, and for the one
    // threshold-5 equipment index.
    let mut state = town_chest_state(0x07);
    let food = state.food;
    let torches = state.torches;
    let gold = state.gold;

    // Find a seed whose opening grants at least one row.
    let mut seed = 0u16;
    loop {
        let mut probe = town_chest_state(0x07);
        probe.prng_state = seed;
        probe.consume_surface_object_chest_at(2, 1, 0, "Opened");
        if probe.message.starts_with(CHEST_TRANSCRIPT_FOUND_HEADING) {
            break;
        }
        seed += 1;
    }
    state.prng_state = seed;
    assert_eq!(
        state.consume_surface_object_chest_at(2, 1, 0, "Opened"),
        Some(MoveOutcome::ContainerOpened)
    );
    assert_eq!((state.food, state.torches, state.gold), (food, torches, gold));
    let staged = staged_at(&state, 2, 1);
    let lines = state.message.lines().count() - 1;
    assert_eq!(staged.len(), lines, "{:?} / {:?}", staged, state.message);
    for (_, object) in &staged {
        assert!(matches!(object.type_byte, 0x02 | 0x09 | 0x0d | 0x0f), "{object:?}");
        assert_ne!(object.aux1, 0);
    }
    // Downward scan: the first staged row takes the highest index.
    assert_eq!(staged.last().unwrap().0, OOL_SLOTS - 1);

    // Each Get transfers one staged object through the class dispatcher.
    let mut gets = 0;
    while !staged_at(&state, 2, 1).is_empty() {
        let outcome = state
            .get_town_facing(Path::new(""), Scene::new(0x11).unwrap(), 0)
            .unwrap();
        assert_eq!(outcome, MoveOutcome::Got, "{:?}", state.message);
        gets += 1;
    }
    assert_eq!(gets, lines);
    assert!(state.food > food || state.torches > torches || state.gold > gold);
}

/// `containers.md §4`: "when every entry is occupied it returns index zero
/// as well. The caller reads a returned index of zero as 'no slot
/// available' and abandons that row's grant", and `§4.1` step 2: a
/// dropped grant "prints nothing".
#[test]
fn a060_rows_beyond_the_free_slots_are_dropped_silently() {
    let mut state = town_chest_state(0x7f);
    state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
    for slot in 2..OOL_SLOTS {
        state.active_objects[slot] = ActiveObject {
            type_byte: 0x50,
            tile: 0x50,
            x: 20,
            y: 20,
            z: 0,
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
    }
    let food = state.food;
    // Put the chest in the top slot and fill slot 1, so once the chest's
    // record is cleared its slot is the only free one in the table.
    state.active_objects.swap(1, OOL_SLOTS - 1);
    assert_eq!(
        state.consume_surface_object_chest_at(2, 1, 0, "Opened"),
        Some(MoveOutcome::ContainerOpened)
    );
    // The chest's own cleared slot takes exactly one row; the rest drop.
    assert_eq!(staged_at(&state, 2, 1).len(), 1);
    assert_eq!(state.message.lines().count(), 2, "{:?}", state.message);
    assert_eq!(state.food, food);
}

/// `containers.md §4.1` "Draw-order details": every roll is a shared PRNG
/// draw. A class-2 chest is below every primary and equipment threshold, so
/// only the equipment pool's `floor(2 / 2) + 1 = 2` index draws are made -
/// ineligible rows and indices draw nothing more.
#[test]
fn a061_ineligible_rows_and_indices_skip_their_gate_draw() {
    let mut state = town_chest_state(0x02);
    state.prng_state = 0x4321;
    state.consume_surface_object_chest_at(2, 1, 0, "Opened");
    let mut prng = U5Prng::new(0x4321);
    prng.next_range_u8(0, 47);
    prng.next_range_u8(0, 47);
    assert_eq!(state.prng_state, prng.state());
    assert_eq!(state.message, CHEST_TRANSCRIPT_EMPTY_LINE);
}

/// `containers.md §4.1`: "for the Chest-marker and Gold rows that first draw
/// (`1..10` and `1..90`) is **discarded** and the family's own roll ... is
/// made afterwards". Replays a class-3 chest (Gold is its only eligible
/// primary row; the equipment pool makes two index draws).
#[test]
fn a061_gold_row_takes_a_discarded_first_draw() {
    for seed in 0..64u16 {
        let mut state = town_chest_state(0x03);
        state.prng_state = seed;
        state.consume_surface_object_chest_at(2, 1, 0, "Opened");

        let mut prng = U5Prng::new(seed);
        let gate = prng.next_range_u8(1, 30);
        let mut expected_gold = None;
        if gate >= 3 {
            prng.next_range_u8(1, 90);
            expected_gold = Some(prng.next_range_u8(1, 9));
        }
        prng.next_range_u8(0, 47);
        prng.next_range_u8(0, 47);
        assert_eq!(state.prng_state, prng.state(), "seed {seed}");
        let staged = staged_at(&state, 2, 1);
        match expected_gold {
            Some(amount) => {
                assert_eq!(staged.len(), 1);
                assert_eq!(staged[0].1.type_byte, 0x02);
                assert_eq!(staged[0].1.aux1, amount);
                assert_eq!(
                    state.message,
                    format!("{CHEST_TRANSCRIPT_FOUND_HEADING}{CHEST_TRANSCRIPT_GOLD_LINE}")
                );
            }
            None => assert!(staged.is_empty()),
        }
    }
}

/// `containers.md §5`: the narrator uses "the acting member's
/// **Intelligence**" - not the class letter - and "a uniform `1..30`
/// roll". An untrapped container's threshold is `(30 - INT) / 2`, so an
/// Intelligence of 30 detects every time and reads `no trap!`.
#[test]
fn a062_search_trap_assessment_uses_intelligence_and_the_shared_prng() {
    let mut state = town_chest_state(0x05);
    state.party_intelligence[0] = 30;
    let before = state.prng_state;
    assert_eq!(state.search_facing_secret(&[], None), MoveOutcome::Searched);
    assert_eq!(state.message, format!("{SEARCH_PREAMBLE}{SEARCH_TRAP_NONE_LINE}"));
    assert_eq!(state.prng_state, u5_prng_advance_state(before));
}

/// `containers.md §5` with `traps.md §2.1`: the assessing member comes from
/// the shared acting-member selection, so a party with two able members is
/// asked `Player: ` first and the chosen member's Intelligence is used.
#[test]
fn a062_search_trap_assessment_prompts_for_the_acting_member() {
    let mut state = town_chest_state(0x05);
    let mut second = state.party[0];
    second.slot = 1;
    state.party.push(second);
    state.party_names = default_party_names(2);
    state.party_intelligence = vec![60, 30];

    assert_eq!(state.search_facing_secret(&[], None), MoveOutcome::Observed);
    assert!(state.active_surface_chest.is_some());
    assert_eq!(
        state.step_active_surface_chest('2', "").unwrap(),
        Some(MoveOutcome::Searched)
    );
    // Member 2's Intelligence 30 always detects: `no trap!`. Member 1's 60
    // would have wrapped the threshold and reported a phantom trap.
    assert_eq!(state.message, format!("{SEARCH_PREAMBLE}{SEARCH_TRAP_NONE_LINE}"));
}

/// `containers.md §7`: crops `0x2D` and the wall torches `0xB0`/`0xB1` are
/// tile-class rules that "apply at any coordinate" - no per-coordinate
/// sidecar row is needed.
#[test]
fn a064_crops_and_wall_torches_are_tile_class_get_rules() {
    let mut grid = open_grid();
    grid[32 + 2] = 0x2d;
    grid[2 * 32 + 1] = 0xb1;
    let mut state = test_state(grid, 1, 1);
    state.moral_standing = 10;
    state.food = 9999;
    let scene = Scene::new(0x11).unwrap();
    let empty = std::env::temp_dir().join("u5-audit-g11-no-sidecars");

    let turn = state.turn;
    assert_eq!(
        state
            .get_town_direction(&empty, scene, 0, Direction::East)
            .unwrap(),
        MoveOutcome::Got
    );
    assert_eq!(state.grid[32 + 2], 0x2c);
    assert_eq!(state.message, GET_TILE_CROPS_PICKED_LINE);
    assert_eq!(state.food, 9999, "9999 cap");
    assert!(state.moral_standing < 10);
    assert_eq!(state.turn, turn + 1);

    state.torch_counter = 0;
    assert_eq!(
        state
            .get_town_direction(&empty, scene, 0, Direction::South)
            .unwrap(),
        MoveOutcome::Got
    );
    assert_eq!(state.grid[2 * 32 + 1], 0x44);
    assert_eq!(state.message, GET_TILE_BORROWED_LINE);
    assert_eq!(state.torch_counter, 100);
}

fn combat_snapshot(state: &PlayState, exit_announced: bool, any_foe_seated: bool) -> CombatFrameSnapshot {
    CombatFrameSnapshot {
        area: state.area,
        player: state.player,
        active_objects: state.active_objects.clone(),
        active_player: None,
        combat_terrain: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        dungeon_room_clear_on_success: None,
        enter_endgame_after_successful_combat: false,
        endgame_messages: None,
        endgame_tableau_map: None,
        encounter_mode_high_bit: false,
        suppress_controlled_faint_sleep_tick: false,
        exit_announced,
        established_exit_direction_code: None,
        any_foe_seated,
    }
}

/// `inventory.md §5.2`/§6: the body-armour lock is voiced as
/// `\n\nThou canst not change armour in heated battle!` while the battle is
/// undecided, and "lifts as soon as the outcome has been announced"; an
/// arena that starts empty "already has its announcement guard set".
#[test]
fn a100_combat_armour_lock_is_voiced_and_lifts_once_the_outcome_is_announced() {
    let fresh = || {
        let mut state = test_state(open_grid(), 1, 1);
        state.combat_active = true;
        state.party_strengths = vec![60];
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_ARMOUR] = EQUIPMENT_EMPTY;
        state.equipment_stock[10] = 1;
        state
    };

    let mut undecided = fresh();
    undecided.combat_frame_snapshot = Some(combat_snapshot(&undecided, false, true));
    assert_eq!(undecided.ready_equipment_from_suffix("1/10"), MoveOutcome::Blocked);
    assert_eq!(
        undecided.message,
        format!("\n\n{READY_COMBAT_ARMOUR_LOCK_REFUSAL}")
    );

    let mut announced = fresh();
    announced.combat_frame_snapshot = Some(combat_snapshot(&announced, true, true));
    assert_eq!(announced.ready_equipment_from_suffix("1/10"), MoveOutcome::Used);
    assert_eq!(announced.party_equipment[0][EQUIP_SLOT_ARMOUR], 10);

    let mut empty_arena = fresh();
    empty_arena.combat_frame_snapshot = Some(combat_snapshot(&empty_arena, false, false));
    assert_eq!(empty_arena.ready_equipment_from_suffix("1/10"), MoveOutcome::Used);
}

/// `inventory.md §7.1`: in dungeon exploration the Skull Key prints
/// `Not here!` without asking `Direction-`, its key already spent.
#[test]
fn a103_dungeon_skull_key_refuses_without_a_direction_prompt() {
    let dir = u5_runtime::test_fixtures::debug_game_dir();
    let mut state = u5_runtime::test_fixtures::dungeon_state(
        u5_runtime::test_fixtures::open_dungeon_record(),
        0,
        1,
        1,
    );
    state.special_items = [0; 16];
    state.scroll_stock = [0; SCROLL_COUNT];
    state.potion_stock = [0; POTION_COUNT];
    state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] = 1;

    handle_play_key_input(&mut state, 'u', "", &dir).unwrap();
    assert!(state.active_use.is_some());
    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();

    assert!(state.active_use.is_none(), "no Direction- prompt is pending");
    assert_eq!(state.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX], 0);
    let transcript: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .chain(std::iter::once(state.message.clone()))
        .collect();
    assert!(transcript.iter().any(|row| row.contains("Not here!")), "{transcript:?}");
    assert!(!transcript.iter().any(|row| row.contains("Direction-")), "{transcript:?}");
    let _ = std::fs::remove_dir_all(dir);
}

/// `inventory.md §7.1` "The Sceptre's fallback", dungeon arm: "it tests the
/// party's own cell and then the cell one step ahead; a field cell is
/// cleared to its visit-marker bit, the helper itself prints `Field
/// destroyed!`, and the Sceptre adds nothing" - under the wielding line that
/// prints "on every accepted selection".
#[test]
fn a102_dungeon_sceptre_runs_the_an_grav_helper_ahead_of_the_party() {
    let mut grid = u5_runtime::test_fixtures::open_dungeon_record();
    // Party at (1, 1) facing the fixture's default; put a marked fire field
    // on the cell ahead.
    let mut state = u5_runtime::test_fixtures::dungeon_state(grid.clone(), 0, 1, 1);
    let (dx, dy) = state.player.facing.delta();
    let ahead = dungeon_cell_index(0, (1 + dx) as usize, (1 + dy) as usize);
    grid[ahead] = 0x8a;
    let mut state = u5_runtime::test_fixtures::dungeon_state(grid, 0, 1, 1);
    state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;

    assert_eq!(state.use_sceptre_of_lord_british(), MoveOutcome::Used);
    assert_eq!(state.grid[ahead], 0x08);
    assert_eq!(
        state.message,
        format!("{USE_SCEPTRE_WIELDED}\n{DISPEL_FIELD_DESTROYED_LINE}")
    );
}

/// `inventory.md §7.1`, arena arm: when the nearby scan dissolves nothing
/// the helper "shows the ordinary direction prompt: ... a chosen cell
/// holding a field marker has that marker removed and returns 1 so the
/// Sceptre prints `Field dissolved!`, and otherwise it returns 0 (`No
/// effect!`)".
#[test]
fn a102_arena_sceptre_fallback_takes_a_direction() {
    let mut state = test_state(open_grid(), 1, 1);
    state.combat_active = true;
    state.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = 1;
    state.combat_actors[0] = CombatActorDescriptor::from_row([30, 1, 0, 0, 0, STEADY_PHASE, 5, 5]);
    state.pending_combat_actor_slot = Some(0);
    assert!(state.sceptre_arena_fallback_prompts());

    assert_eq!(
        state.use_sceptre_arena_field_direction(Direction::East),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "No effect!");

    state.active_objects.push(ActiveObject {
        type_byte: COMBAT_FIELD_KIND_FIRE,
        tile: COMBAT_FIELD_KIND_FIRE,
        x: 6,
        y: 5,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    assert_eq!(
        state.use_sceptre_arena_field_direction(Direction::East),
        MoveOutcome::Used
    );
    assert_eq!(state.message, "Field dissolved!");
    assert!(state.find_combat_arena_field_marker(6, 5).is_none());
}

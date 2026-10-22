// Regressions for the fourth-pass combat exploratory-playtest findings
// (`p4combat`, H02-H32).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn p4combat_rows_text(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// `combat.md §8`, the `P` row: "Prints the `Push-` label and calls the
/// shared movable-static-tile handler directly", and `commands.md §8.1` row
/// C's exact text `Push-<Dir>\nWon't budge!\n`. Playtest H23: the world
/// dispatcher's verb echo was opened as well, so `Push-` stood on two rows.
#[test]
fn playtest_h23_combat_push_prints_its_label_once() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.ensure_pending_combat_player_turn();
    let before = state.message_entries().len();

    handle_play_key_input(&mut state, 'P', "", game_dir).unwrap();

    let rows = p4combat_rows_text(&state);
    let labels = rows[before..]
        .iter()
        .filter(|row| row.starts_with("Push"))
        .count();
    assert_eq!(labels, 1, "rows: {rows:?}");
    assert_eq!(
        rows.last().map(String::as_str),
        Some("Push-"),
        "rows: {rows:?}"
    );
}

/// The same `P` row in a dungeon-room arena: the dungeon-mode `Push` echo
/// (whose `Not here!` refusal never runs in combat) must not open above the
/// arena's `Push-` prompt. Playtest H27.
#[test]
fn playtest_h27_dungeon_room_combat_push_opens_no_stray_echo_row() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.area = Area::Dungeon {
        scene: DungeonScene::new(0x1B).unwrap_or(DungeonScene {
            byte: 0x1B,
            record: 0,
        }),
        level: 1,
    };
    state.ensure_pending_combat_player_turn();
    let before = state.message_entries().len();

    handle_play_key_input(&mut state, 'P', "", game_dir).unwrap();

    let rows = p4combat_rows_text(&state);
    let new_rows: Vec<&String> = rows[before..]
        .iter()
        .filter(|row| !row.is_empty())
        .collect();
    assert_eq!(new_rows, [&"Push-".to_string()], "rows: {rows:?}");
}

/// `overworld.md §5`: the per-turn block (two-minute clock, party upkeep)
/// is step 6 and runs once, after step 5's dispatch returns; `combat.md §4`:
/// "the only time a fight adds is its round-wrap minutes". Playtest H21: an
/// overworld A-Attack advanced the turn before framing the arena as well as
/// after it, so the arena opened two minutes late and a poisoned member
/// could die before being seated.
#[test]
fn playtest_h21_overworld_attack_spends_no_turn_before_the_arena() {
    let dir = debug_game_dir();
    fs::write(
        dir.join(BRIT_CBT_FILE),
        synthetic_combat_arena_record().repeat(BRIT_CBT_RECORDS),
    )
    .unwrap();
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: 6,
        y: 5,
        z: WorldPlane::Underworld.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    let turn = state.turn;
    let clock = (state.clock.hour, state.clock.minute);

    handle_play_key_input(&mut state, 'A', "", &dir).unwrap();
    handle_play_key_input(&mut state, char::from(INPUT_CODE_EAST), "", &dir).unwrap();

    assert!(state.combat_active, "the attack frames the arena");
    assert_eq!(state.turn, turn, "no turn is spent before the arena");
    assert_eq!((state.clock.hour, state.clock.minute), clock);
    let _ = fs::remove_dir_all(dir);
}

/// `combat.md §11.1`, census: "Monster damaged, split class | party attacker
/// | `<monster> divides!` inside the damage handler, and the ordinary result
/// line **still** prints after it"; `Order, stated once` step 4 then step 5.
/// Playtest H20: the line went only into the message slot, the result line
/// overwrote it, and no `divides!` was ever shown.
#[test]
fn playtest_h20_a_dividing_monster_prints_divides_before_its_wound_line() {
    let game_dir = std::path::Path::new(".");
    let mut seen = false;
    for seed in 0..400u16 {
        let mut state = combat_state_after_the_conflict_banner(6, 5);
        state.combat_actors[8].owner_target_class = 24;
        state.combat_actors[8].hp_or_wound = combat_class_stats(24).unwrap().max_hp;
        state.prng_state = seed;
        state.ensure_pending_combat_player_turn();
        handle_play_key_input(
            &mut state,
            'A',
            &format!("{}\r", char::from(INPUT_CODE_EAST)),
            game_dir,
        )
        .unwrap();
        state.flush_message_slot();
        let rows = p4combat_rows_text(&state);
        let Some(wound) = rows.iter().position(|row| {
            row.starts_with("Slime ")
                && row.ends_with('!')
                && !row.ends_with("divides!")
                && !row.ends_with("missed!")
                && !row.ends_with("grazed!")
                && !row.ends_with("killed!")
        }) else {
            continue;
        };
        seen = true;
        let divides = rows
            .iter()
            .position(|row| row == "Slime divides!")
            .unwrap_or_else(|| panic!("seed {seed}: rows: {rows:?}"));
        // Step 3's hit newline precedes it, and step 5's result line
        // follows on the very next row.
        assert_eq!(divides + 1, wound, "seed {seed}: rows: {rows:?}");
        assert_eq!(rows[divides - 1], "", "seed {seed}: rows: {rows:?}");
        break;
    }
    assert!(seen, "some seed lands a wounding hit");
}

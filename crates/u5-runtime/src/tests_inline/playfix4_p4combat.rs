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

/// The Crown-bypass fixture of `combat_audit_g1`: a lone fleeing Wanderer in
/// the arena's far corner whose first offered axis carries it off the grid.
fn p4combat_lone_fleeing_hostile_state() -> PlayState {
    let mut state = combat_ai_turn_state(10, 10);
    state.combat_actors[8].owner_target_class = 13;
    state.combat_actors[0].x = 0;
    state.combat_actors[0].y = 0;
    state.active_objects[0].x = 0;
    state.active_objects[0].y = 0;
    state.active_effect_tag = Some(CROWN_LB_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = u8::MAX;
    state
}

/// `combat.md §11.1`, "The self-acting arena exit line": "a line feed, the
/// actor's name, the stored line ` escapes!`, and one further line feed",
/// the leading feed printed "as a single character, before anything else
/// on the turn". Playtest H03: under an already-closed row (`North`) the
/// leading feed was swallowed, so no blank row separated them.
#[test]
fn playtest_h03_the_escape_line_keeps_its_leading_line_feed() {
    let mut state = p4combat_lone_fleeing_hostile_state();
    state.emit_combat_print("North\n");
    let before = state.message_entries().len();

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
            Some((9, 9)),
            true,
            &[1, 2, 3, 4],
            None,
        )
        .unwrap();

    assert_eq!(
        application.movement,
        Some(CombatAiMovementOutcome::ArenaExit { direction_code: 2 })
    );
    let rows = p4combat_rows_text(&state);
    assert_eq!(
        rows[before..],
        ["".to_string(), "Wanderer escapes!".to_string()],
        "rows: {rows:?}"
    );
    assert!(!state.combat_transcript_row_open);
}

/// `combat.md §14`, "The departure counts toward the side recount
/// immediately": "the round loop runs the census as soon as the dispatch
/// returns, before advancing to the next slot ... An escape by the last
/// hostile therefore arms `VICTORY!` on that same turn", so "a lone fleeing
/// hostile against one party member prints the escape line and then the
/// victory line back to back". Playtest H02: `VICTORY!` waited for the next
/// party action.
#[test]
fn playtest_h02_the_last_hostile_escaping_arms_victory_on_its_own_turn() {
    let mut state = p4combat_lone_fleeing_hostile_state();
    let before = state.message_entries().len();

    let application = state.apply_combat_actor_slot_dispatch_with_inputs(
        8,
        30,
        false,
        0,
        false,
        255,
        255,
        &[],
        None,
        0,
        false,
        Some((9, 9)),
        true,
        &[1, 2, 3, 4],
        &[],
    );

    assert!(matches!(
        application,
        CombatActorSlotDispatchApplication::Slot {
            action: CombatActorDispatchAction::MonsterAi { .. },
            ..
        }
    ));
    let rows = p4combat_rows_text(&state);
    let tail: Vec<&str> = rows[before..]
        .iter()
        .map(String::as_str)
        .filter(|row| !row.is_empty())
        .collect();
    assert_eq!(tail, ["Wanderer escapes!", "VICTORY!"], "rows: {rows:?}");
}

/// `combat.md §11.1`, "Order, stated once": "the per-turn standing hazard
/// reuses the same result narrator with **no newline before it at all**, and
/// with no attacker, so a party target on that tier always reads the flat
/// `<target> hit!`"; `§7` step 7's middle (fire) tier feeds the
/// damage-and-status resolver. Playtest H22: the hazard took HP every turn
/// and printed nothing.
#[test]
fn playtest_h22_standing_in_fire_narrates_the_flat_hit_line() {
    let game_dir = std::path::Path::new(".");
    let mut seen = false;
    for seed in 0..64u16 {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.party[0].hp = 60;
        state.party[0].max_hp = 60;
        state.combat_terrain[5][5] = COMBAT_CONTACT_TERRAIN_MOLTEN_LAVA;
        state.prng_state = seed;
        state.ensure_pending_combat_player_turn();
        let before = state.message_entries().len();

        handle_play_key_input(&mut state, ' ', "", game_dir).unwrap();

        if state.party[0].hp >= 60 {
            continue;
        }
        seen = true;
        let rows = p4combat_rows_text(&state);
        let pass = before
            + rows[before..]
                .iter()
                .position(|row| row == "Pass")
                .unwrap_or_else(|| panic!("seed {seed}: rows: {rows:?}"));
        assert_eq!(rows[pass + 1], "Avatar hit!", "seed {seed}: rows: {rows:?}");
        break;
    }
    assert!(seen, "some seed lands fire damage");
}

/// `combat.md §8.1`: "**Short re-prompt, banner not reprinted:** an
/// unrecognised key (`What?`), the two toggles Ctrl-S and Ctrl-B ... The
/// handler emits one newline, refreshes the prompt window ... and reads
/// another key", and `§8.4` gives `Sound Off\n`, `Buffer Off\n` and `What?\n`.
/// Playtest H24: after Ctrl-S, Ctrl-B or a function key the next prompt sat
/// on the very next row, where the unmapped `7` leaves a blank row.
#[test]
fn playtest_h24_toggles_and_function_keys_take_the_short_re_prompt_blank_row() {
    let game_dir = std::path::Path::new(".");
    for key in [
        PLAY_MUSIC_TOGGLE_KEY,
        PLAY_TYPEAHEAD_TOGGLE_KEY,
        char::from(0xC9u8),
        '7',
    ] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.active_player = Some(0);
        state.ensure_pending_combat_player_turn();

        handle_play_key_input(&mut state, key, "", game_dir).unwrap();

        assert_eq!(state.pending_combat_actor_slot, Some(0), "{key:?}");
        let rows = combat_message_window_rows(&state);
        let prompt = rows
            .iter()
            .rposition(|row| row.starts_with('>'))
            .expect("the marker row is drawn");
        assert!(
            rows[prompt - 1].is_empty(),
            "{key:?}: the re-prompt keeps its blank row: {rows:?}"
        );
    }
}

/// `combat.md §8.1`: the banner ends with a colon and one line feed before
/// the key read, so whatever the player types next is echoed on the row
/// that feed opened - the same row for every letter. Playtest H32: under a
/// banner whose last row is short, R-Ready, U-Use and Y-Yell opened their
/// world verb echo under an extra blank row while D, C, A, S and G did not.
#[test]
fn playtest_h32_every_letter_echoes_on_the_row_under_the_banner() {
    let game_dir = std::path::Path::new(".");
    for key in ['D', 'R', 'U', 'Y'] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        // `Iolo, armed with` / `bare hands:` - the banner's last row does
        // not fill the window's width.
        state.party_names[0] = *b"Iolo\0\0\0\0\0";
        state.active_player = Some(0);
        state.ensure_pending_combat_player_turn();

        handle_play_key_input(&mut state, key, "", game_dir).unwrap();

        let rows = combat_message_window_rows(&state);
        let banner_end = rows
            .iter()
            .position(|row| row == "bare hands:")
            .unwrap_or_else(|| panic!("{key}: rows: {rows:?}"));
        assert!(
            rows[banner_end + 1].starts_with('>') && rows[banner_end + 1].len() > 1,
            "{key}: the echo sits on the row under the banner: {rows:?}"
        );
    }
}

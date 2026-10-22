// Regressions for the combat U-Use / Gate Travel exploratory-playtest
// findings (F28-F34).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// A fight under way with two party members, party slot 1 holding the open
/// command prompt (slot 0 is the Avatar), as in `sanct_lor_flash_state`.
/// Descriptor 1's owner/target/class byte names roster slot 1 (`combat.md
/// §5`).
fn combat2b_two_member_state() -> PlayState {
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: 1,
        status: b'G',
        climb_stat: 0,
        mana: 0,
        hp: 20,
        max_hp: 20,
        level: 1,
    });
    state.party_experience.push(0);
    state.party_strengths.push(20);
    state.party_intelligence.push(20);
    state.party_equipment = default_party_equipment(2);
    state.active_objects[1] = ActiveObject {
        type_byte: 0x81,
        tile: 0x81,
        x: 4,
        y: 5,
        ..ActiveObject::empty()
    };
    state.combat_actors[1] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 4, 5]);
    state.next_combat_actor_slot = 1;
    state.ensure_pending_combat_player_turn();
    assert_eq!(state.pending_combat_actor_slot, Some(1));
    state
}

/// Open U-Use, walk the picker's highlight onto the row whose request
/// `wanted` accepts, then accept it.
fn combat2b_use_row(
    state: &mut PlayState,
    wanted: impl Fn(UseItemRequest) -> bool,
    game_dir: &std::path::Path,
) {
    handle_play_key_input(state, 'U', "", game_dir).unwrap();
    assert!(state.active_use.is_some(), "the U-Use picker opens");
    let index = state
        .use_item_picker_rows()
        .iter()
        .position(|row| wanted(row.request))
        .expect("the wanted item has a picker row");
    for _ in 0..index {
        handle_play_key_input(state, char::from(INPUT_CODE_SOUTH), "", game_dir).unwrap();
    }
    handle_play_key_input(state, '\r', "", game_dir).unwrap();
}

fn combat2b_rows_text(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// `inventory.md §7.1`: "The Resurrection scroll and non-combat potion share
/// `On who:_` ... Combat potion use binds the current party combatant
/// without this prompt." Playtest F28: combat opened the `On who:` selector
/// and let the acting member's turn heal anybody.
#[test]
fn playtest_f28_a_combat_potion_binds_the_acting_combatant_without_on_who() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat2b_two_member_state();
    state.party[0].hp = 5;
    state.party[1].hp = 5;
    state.potion_stock[POTION_YELLOW_INDEX] = 1;
    // Pin the bottle's variation roll to its own colour for party slot 1 so
    // the heal can be read off the bound member.
    let hour = (0..24u8)
        .find(|hour| {
            state.clock.hour = *hour;
            state.potion_variation_roll(POTION_YELLOW_INDEX, 1)
                <= POTION_VARIATION_SELECTED_THRESHOLD
        })
        .expect("some hour keeps the selected colour");
    state.clock.hour = hour;

    combat2b_use_row(
        &mut state,
        |request| matches!(request, UseItemRequest::Potion { .. }),
        game_dir,
    );

    assert!(state.active_use.is_none(), "no target selector may open");
    assert_eq!(state.potion_stock[POTION_YELLOW_INDEX], 0);
    let rows = combat2b_rows_text(&state);
    assert!(
        !rows.iter().any(|row| row.contains("On who")),
        "rows: {rows:?}"
    );
    assert!(
        rows.iter().any(|row| row.contains("Healed!")),
        "rows: {rows:?}"
    );
    assert!(state.party[1].hp > 5, "the acting member drank it");
    assert_eq!(state.party[0].hp, 5, "the Avatar was not the target");
}

/// `catalogs/item-list.md §7.1`, scroll 6: "Prints `Resurrection!`.
/// Non-combat scenes prompt for a party member ... Combat-class scenes print
/// `Not here!`." Playtest F29: combat opened `On who:` and refused only
/// after a member had been chosen.
#[test]
fn playtest_f29_the_resurrection_scroll_refuses_in_combat_without_on_who() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat2b_two_member_state();
    state.scroll_stock[SCROLL_RESURRECTION_INDEX] = 1;

    combat2b_use_row(
        &mut state,
        |request| {
            matches!(
                request,
                UseItemRequest::Scroll {
                    index: SCROLL_RESURRECTION_INDEX,
                    ..
                }
            )
        },
        game_dir,
    );

    assert!(state.active_use.is_none(), "no target selector may open");
    // `inventory.md §8`: debited before the branch's scene gate.
    assert_eq!(state.scroll_stock[SCROLL_RESURRECTION_INDEX], 0);
    let rows = combat2b_rows_text(&state);
    assert!(
        !rows.iter().any(|row| row.contains("On who")),
        "rows: {rows:?}"
    );
    let banner = rows
        .iter()
        .position(|row| row == SCROLL_RESURRECTION_RESULT)
        .unwrap_or_else(|| panic!("the banner prints: {rows:?}"));
    assert_eq!(rows.get(banner + 1).map(String::as_str), Some("Not here!"));
}

/// `inventory.md §7.1`: one `On who:_` line, which "The selected member's
/// name completes". Playtest F30: the first arrow or digit under the
/// Resurrection scroll's prompt logged a second `On who:` row.
#[test]
fn playtest_f30_moving_the_on_who_selection_does_not_reprint_the_prompt() {
    let game_dir = std::path::Path::new("");
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(PartyMember {
        slot: 1,
        class_byte: 1,
        status: b'D',
        climb_stat: 0,
        mana: 0,
        hp: 0,
        max_hp: 20,
        level: 1,
    });
    state.scroll_stock[SCROLL_RESURRECTION_INDEX] = 1;

    handle_play_key_input(&mut state, 'U', "", game_dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    assert!(state.active_use.is_some());
    handle_play_key_input(&mut state, char::from(INPUT_CODE_SOUTH), "", game_dir).unwrap();
    handle_play_key_input(&mut state, '2', "", game_dir).unwrap();
    handle_play_key_input(&mut state, 'x', "", game_dir).unwrap();
    assert!(state.active_use.is_some());
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    assert!(state.active_use.is_none());
    assert_eq!(state.party[1].status, b'G');

    let rows = combat2b_rows_text(&state);
    let prompts: Vec<_> = rows.iter().filter(|row| row.contains("On who")).collect();
    assert_eq!(prompts.len(), 1, "rows: {rows:?}");
    assert!(
        prompts[0].starts_with(USE_POTION_TARGET_PROMPT),
        "rows: {rows:?}"
    );
    assert!(
        prompts[0].len() > USE_POTION_TARGET_PROMPT.len(),
        "completed: {rows:?}"
    );
}

/// The same single-row rule for the Wind Change scroll's `Direction-`
/// prompt, whose unrecognised keys "wait for another input" (§7.1).
#[test]
fn playtest_f30_an_ignored_key_does_not_reprint_the_wind_scroll_prompt() {
    let game_dir = std::path::Path::new("");
    let mut state = test_state(open_grid(), 5, 5);
    state.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;

    handle_play_key_input(&mut state, 'U', "", game_dir).unwrap();
    handle_play_key_input(&mut state, '\r', "", game_dir).unwrap();
    handle_play_key_input(&mut state, 'x', "", game_dir).unwrap();
    handle_play_key_input(&mut state, 'q', "", game_dir).unwrap();
    assert!(state.active_use.is_some());
    handle_play_key_input(&mut state, '6', "", game_dir).unwrap();
    assert!(state.active_use.is_none());

    let rows = combat2b_rows_text(&state);
    assert_eq!(
        rows.iter()
            .filter(|row| row.starts_with(SPELL_DIRECTION_PROMPT_PREFIX))
            .count(),
        1,
        "rows: {rows:?}"
    );
}

/// `catalogs/item-list.md §7.1`, scroll 5: combat "invoke[s] the summon
/// **placement** helper", which "create[s] a hostile actor at the first
/// accepted cell". The caster self-check that stamps the controlled bit is
/// the Kal Xen Corp spell's (`magic.md §8`, `spell-list.md` row 43).
/// Playtest F31: the scroll's Daemon came up at the player's command prompt.
#[test]
fn playtest_f31_the_summon_daemon_scroll_places_a_hostile_daemon() {
    for seed in [0x0001u16, 0x1234, 0xBEEF, 0x7777, 0xFFFF] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.prng_state = seed;
        // Intelligence 1 would make the spell's self-check always fail; the
        // scroll must not consult it either way.
        state.party_intelligence = vec![99; state.party.len()];
        state.scroll_stock[SCROLL_SUMMON_DAEMON_INDEX] = 1;

        let outcome = state.use_scroll(SCROLL_SUMMON_DAEMON_INDEX, None, None);

        let daemons: Vec<_> = state
            .combat_actors
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, actor)| actor.owner_target_class == COMBAT_CLASS_DAEMON)
            .collect();
        if outcome == MoveOutcome::Used {
            assert_eq!(state.message, "Summon Daemon!");
            assert_eq!(daemons.len(), 1, "seed {seed:#06x}");
            for (slot, actor) in daemons {
                assert_eq!(
                    actor.flags & COMBAT_ACTOR_FLAG_CONTROLLED,
                    0,
                    "seed {seed:#06x}: slot {slot} must stay AI-driven"
                );
            }
        } else {
            assert_eq!(state.message, "Summon Daemon!\nFailed!");
            assert!(daemons.is_empty());
        }
    }
}

/// `catalogs/item-list.md §7.1`: the scroll path "prints the scroll-use
/// label and the branch's effect label" before its gate - scroll 4 "Prints
/// `View!`. Combat-class scenes print `Not here!`", scroll 5 "Prints `Summon
/// Daemon!`. ... non-combat scenes print `Not here!`". Playtest F32: both
/// refusals dropped their banner.
#[test]
fn playtest_f32_refused_scrolls_print_their_banner_first() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat2b_two_member_state();
    state.scroll_stock[SCROLL_VIEW_INDEX] = 1;
    combat2b_use_row(
        &mut state,
        |request| {
            matches!(
                request,
                UseItemRequest::Scroll {
                    index: SCROLL_VIEW_INDEX,
                    ..
                }
            )
        },
        game_dir,
    );
    let rows = combat2b_rows_text(&state);
    let banner = rows
        .iter()
        .position(|row| row == "View!")
        .unwrap_or_else(|| panic!("View! prints in combat: {rows:?}"));
    assert_eq!(rows.get(banner + 1).map(String::as_str), Some("Not here!"));

    let mut state = test_state(open_grid(), 5, 5);
    state.scroll_stock[SCROLL_SUMMON_DAEMON_INDEX] = 1;
    handle_play_key_input(&mut state, 'U', "", std::path::Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\r', "", std::path::Path::new("")).unwrap();
    let rows = combat2b_rows_text(&state);
    let banner = rows
        .iter()
        .position(|row| row == "Summon Daemon!")
        .unwrap_or_else(|| panic!("Summon Daemon! prints outside combat: {rows:?}"));
    assert_eq!(rows.get(banner + 1).map(String::as_str), Some("Not here!"));
}

/// `inventory.md §7.1`: "A wrong destruction position adds `\n\nNo
/// effect!\n`" directly after the shard text, and the next actor's banner
/// (`combat.md §8.1`) belongs to the following turn. Playtest F33: the
/// unpaced walk printed the next banner before the staged result.
#[test]
fn playtest_f33_a_combat_shard_result_prints_before_the_next_banner() {
    let game_dir = std::path::Path::new("");
    let mut state = combat_state_after_the_conflict_banner(8, 5);
    state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    state.ensure_pending_combat_player_turn();
    assert!(state.pending_combat_actor_slot.is_some());

    combat2b_use_row(
        &mut state,
        |request| matches!(request, UseItemRequest::ShadowlordShard(_)),
        game_dir,
    );
    state.flush_staged_narration();
    state.flush_message_slot();

    let rows = combat2b_rows_text(&state);
    let aloft = rows
        .iter()
        .rposition(|row| row.contains("Falsehood..."))
        .unwrap_or_else(|| panic!("the aloft line: {rows:?}"));
    let result = rows
        .iter()
        .rposition(|row| row.contains("No effect!"))
        .unwrap_or_else(|| panic!("the result: {rows:?}"));
    assert!(result > aloft, "rows: {rows:?}");
    assert!(
        !rows[aloft + 1..result]
            .iter()
            .any(|row| row.contains("armed")),
        "no turn banner between the shard text and its result: {rows:?}"
    );
}

/// `magic.md §8` handler-family map, Gate travel: "prompts `To phase:_`,
/// echoes the typed key and a line feed, accepts a digit `1`..`8`".
/// Playtest F34: the digit never appeared on the prompt row.
#[test]
fn playtest_f34_gate_travel_echoes_the_phase_key() {
    for key in ['3', 'x'] {
        let mut state = britannia_state(open_world_grid(), 1, 1);
        state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
        state.party[0].mana = GATE_TRAVEL_COST;
        state.party[0].level = GATE_TRAVEL_COST;
        state.active_player = Some(0);
        assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
        assert!(
            state
                .step_active_cast('P', "RV", Path::new(""))
                .unwrap()
                .is_none()
        );
        assert!(
            state
                .step_active_cast(' ', "", Path::new(""))
                .unwrap()
                .is_none()
        );
        state.flush_message_slot();

        let _ = state
            .step_active_cast_followup(key, "", Path::new(""))
            .unwrap()
            .expect("the phase key ends the prompt");
        state.flush_message_slot();

        let rows = combat2b_rows_text(&state);
        // `input.md §6` folds the key to upper case before the echo (G20).
        let echoed = format!("{GATE_TRAVEL_PHASE_PROMPT}{}", key.to_ascii_uppercase());
        let prompt = rows
            .iter()
            .position(|row| *row == echoed)
            .unwrap_or_else(|| panic!("key {key:?} echoes on the prompt row: {rows:?}"));
        assert_eq!(
            rows.get(prompt + 1).map(String::as_str),
            Some("Failed!"),
            "rows: {rows:?}"
        );
    }
}

/// F28 with a packed party (`combat.md §5`): a dead roster member 1 has no
/// descriptor, so roster member 2 acts from descriptor 1. The potion binds
/// the roster member the descriptor names, not the descriptor index.
#[test]
fn playtest_f28_a_combat_potion_binds_the_roster_member_of_a_packed_descriptor() {
    let game_dir = std::path::Path::new(".");
    let mut state = combat2b_two_member_state();
    state.party[1].status = b'D';
    state.party[1].hp = 0;
    state.party.push(PartyMember {
        slot: 2,
        class_byte: 1,
        status: b'G',
        climb_stat: 0,
        mana: 0,
        hp: 5,
        max_hp: 20,
        level: 1,
    });
    state.party_experience.push(0);
    state.party_strengths.push(20);
    state.party_intelligence.push(20);
    state.party_equipment = default_party_equipment(3);
    state.combat_actors[1].owner_target_class = 2;
    state.party[0].hp = 5;
    state.potion_stock[POTION_YELLOW_INDEX] = 1;
    let hour = (0..24u8)
        .find(|hour| {
            state.clock.hour = *hour;
            state.potion_variation_roll(POTION_YELLOW_INDEX, 2)
                <= POTION_VARIATION_SELECTED_THRESHOLD
        })
        .expect("some hour keeps the selected colour");
    state.clock.hour = hour;

    combat2b_use_row(
        &mut state,
        |request| matches!(request, UseItemRequest::Potion { .. }),
        game_dir,
    );

    assert!(state.active_use.is_none(), "no target selector may open");
    assert!(state.party[2].hp > 5, "roster member 2 drank it");
    assert_eq!(state.party[1].hp, 0, "the dead member was not the target");
    assert_eq!(state.party[1].status, b'D');
    assert_eq!(state.party[0].hp, 5, "the Avatar was not the target");
}

/// F34: Escape has no glyph to echo, so the prompt row stays bare and the
/// ordinary `Failed!` follows on its own row.
#[test]
fn playtest_f34_gate_travel_escape_leaves_the_prompt_row_bare() {
    let mut state = britannia_state(open_world_grid(), 1, 1);
    state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    state.party[0].mana = GATE_TRAVEL_COST;
    state.party[0].level = GATE_TRAVEL_COST;
    state.active_player = Some(0);
    assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
    assert!(state.step_active_cast('P', "RV", Path::new("")).unwrap().is_none());
    assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_none());
    state.flush_message_slot();

    let _ = state
        .step_active_cast_followup('\u{1b}', "", Path::new(""))
        .unwrap()
        .expect("Escape ends the phase prompt");
    state.flush_message_slot();

    let rows = combat2b_rows_text(&state);
    let prompt = rows
        .iter()
        .rposition(|row| row.trim_end() == GATE_TRAVEL_PHASE_PROMPT.trim_end())
        .unwrap_or_else(|| panic!("the bare prompt row: {rows:?}"));
    assert_eq!(rows.get(prompt + 1).map(String::as_str), Some("Failed!"), "rows: {rows:?}");
}

/// F34: a successful travel echoes its digit on the prompt row too.
#[test]
fn playtest_f34_gate_travel_echoes_a_successful_phase_digit() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 1, 1);
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 6,
        y: 7,
        z: WorldPlane::Britannia.save_floor() as u8,
    };
    state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    state.party[0].mana = GATE_TRAVEL_COST;
    state.party[0].level = GATE_TRAVEL_COST;
    state.active_player = Some(0);
    assert_eq!(state.start_cast_spell_prompt(), MoveOutcome::Observed);
    assert!(state.step_active_cast('P', "RV", Path::new("")).unwrap().is_none());
    assert!(state.step_active_cast(' ', "", Path::new("")).unwrap().is_none());
    state.flush_message_slot();

    let _ = state
        .step_active_cast_followup('2', "", &dir)
        .unwrap()
        .expect("the phase digit ends the prompt");
    state.flush_message_slot();

    assert_eq!((state.player.x, state.player.y), (6, 7), "the party travelled");
    let rows = combat2b_rows_text(&state);
    let echoed = format!("{GATE_TRAVEL_PHASE_PROMPT}2");
    assert_eq!(
        rows.iter().filter(|row| **row == echoed).count(),
        1,
        "rows: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

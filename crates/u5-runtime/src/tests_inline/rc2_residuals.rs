// Residual items from the rc2 integration of the playtest fixes
// (u5-engine#46). Each test names the item it covers.

/// Item 1: the tavern states the row its message leaves from the outcome it
/// printed, never from the message's trailing character. The no-assets menu
/// fallback ends `or Space.` - no closing quote and space - yet the tavern
/// still waits on that row for its letter, which echoes there.
#[test]
fn rc2_tavern_row_state_comes_from_the_outcome_not_the_text() {
    use crate::input_dispatch::tavern_outcome_prompt_row;
    use crate::shop_runtime::{TavernOutcome, TavernState};
    use crate::shop_session::ActiveShopSession;

    let menu = TavernOutcome::EnteredMenu {
        tavern: Tavern::TheCatsLair,
        round_letter: 'A',
        secondary_letter: 'B',
        provisions_letter: None,
        lore_letter: 'C',
    };
    assert_eq!(tavern_outcome_prompt_row(&menu, false), Some(true));
    assert_eq!(
        tavern_outcome_prompt_row(&TavernOutcome::RefusedNoNeed, true),
        Some(false),
        "`yells $.\\n` closes its row"
    );
    assert_eq!(
        tavern_outcome_prompt_row(&TavernOutcome::EnteredSagePrompt, false),
        None
    );
    assert_eq!(
        tavern_outcome_prompt_row(&TavernOutcome::InvalidInput, false),
        None
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Greeting {
        tavern: Tavern::TheCatsLair,
    }));
    state.emit_message_line("\n\"Welcome!\"\n\n:");
    // A stale closed row, which the guess left alone for this text.
    state.close_message_row();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.ends_with("or Space."), "{:?}", state.message);
    assert!(state.message_row_open_mid_line);

    let round = state
        .message
        .split("drink round (")
        .nth(1)
        .and_then(|rest| rest.chars().next())
        .unwrap();
    handle_play_key_input(&mut state, round, "", Path::new("")).unwrap();
    assert!(
        state
            .message_entries()
            .iter()
            .any(|entry| entry.text.ends_with(&format!("or Space.{round}"))),
        "{:?}",
        state.message_entries()
    );
}

/// Item 2: `shops.md §8.C`, healers: "otherwise print `\n\n"Who needs my
/// aid?" ` and use the party selector. Cancellation adds `No one`, with no
/// trailing line feed", then the continuation question. `No one` belongs on
/// the question's row, not a row of its own.
#[test]
fn rc2_healer_selector_cancel_adds_no_one_after_the_question() {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    three_member_party(&mut state);
    state.gold = 1000;
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::PickService,
        Healer::TheHealersMission,
    ));
    state.emit_message_line("\"What is the nature of thy need?\" ");
    handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();

    let rows: Vec<(String, bool)> = state
        .message_entries()
        .iter()
        .map(|entry| (entry.text.clone(), entry.explicit_blank))
        .collect();
    let at = rows
        .iter()
        .position(|(text, _)| text == "\"Who needs my aid?\" No one")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert!(rows[at + 1].0.is_empty(), "{rows:?}");
    assert_eq!(rows[at + 2].0, "\"Is there any other way in which I may");
    assert_eq!(rows[at + 3].0, "aid thee?\" ");
    assert!(!rows.iter().any(|(text, _)| text == "No one"), "{rows:?}");
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Healer(HealerShopState::Continuation, _))
    ));
}

/// Item 3: `shops.md §8.C`: "The final prompt is `Thy choice?" `", and
/// "Blue Boar wine selection accepted | Uppercase selected letter, then
/// `\n\n"Ah, a fine\nchoice, `, honorific and `.`; ... paid purchase adds
/// `\nEnjoy!"`, then `\n\n`".
#[test]
fn rc2_blue_boar_choice_prompt_and_uppercase_letter_echo() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Menu {
        tavern: Tavern::TheBlueBoarTavern,
        continuation_ready: false,
    }));
    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    assert!(
        state.message.ends_with("\n\nThy choice?\" "),
        "{:?}",
        state.message
    );

    handle_play_key_input(&mut state, 'f', "", Path::new("")).unwrap();
    assert_eq!(state.gold, 102);
    let rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    let at = rows
        .iter()
        .position(|text| text == "Thy choice?\" F")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(
        &rows[at + 1..],
        &[
            "".to_string(),
            "\"Ah, a fine".to_string(),
            "choice, sir.".to_string(),
            "Enjoy!\"".to_string(),
            "".to_string(),
            "\"Anything else".to_string(),
            "for thee?\" ".to_string(),
        ]
    );
}

/// Item 4: `combat.md §8.2`'s cursor table - "Enter, or the letter `A`
/// (either case)" confirms, and the corner keys move diagonally - holds for
/// the spell `Aim! ` cursor as for the Attack cursor (`magic.md §8`: "the
/// shared `Aim! ` cursor"). `§8.3`: a typed digit with the numpad flag clear
/// is inert inside the cursor, the same flag-clear reading the combat
/// command prompt gives digits (playtest E36).
#[test]
fn rc2_spell_aim_cursor_takes_a_diagonals_and_ignores_digits() {
    let dir = std::path::Path::new("");
    let spell_index = spell_index_from_code("GP").unwrap();
    let stats = combat_class_stats(32).unwrap();
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.party[0].mana = 1;
    state.party[0].level = 1;
    state.spell_charges[spell_index] = 1;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 4, 5]);
    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS] = CombatActorDescriptor::for_monster_placement(
        stats,
        7,
        5,
        6,
        COMBAT_ACTOR_FLAG_SELECTABLE_40,
        0,
    );
    state.start_combat_cast_spell_prompt(0, false);
    state.step_active_cast('G', "P", dir).unwrap();
    state.step_active_cast(' ', "", dir).unwrap();
    assert_eq!(state.message, "Aim! ");
    let cursor = |state: &PlayState| state.active_cast_followup.as_ref().map(|s| s.kind);

    // Typed digits are inert, including the keypad corners `3` and `9`.
    for digit in ['1', '2', '3', '4', '6', '7', '8', '9'] {
        assert!(
            state
                .step_active_cast_followup(digit, "", dir)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            cursor(&state),
            Some(CastFollowupKind::CombatAimCursor { x: 4, y: 5 }),
            "{digit}"
        );
    }
    // `A` on the caster's own cell: nothing happens.
    assert!(
        state
            .step_active_cast_followup('a', "", dir)
            .unwrap()
            .is_none()
    );
    assert!(state.active_cast_followup.is_some());

    // PgDn (south-east) is a diagonal step onto the orc.
    state
        .step_active_cast_followup(char::from(INPUT_CODE_SOUTHEAST), "", dir)
        .unwrap();
    assert_eq!(
        cursor(&state),
        Some(CastFollowupKind::CombatAimCursor { x: 5, y: 6 })
    );
    assert_eq!(state.combat_aim_marker_cell, Some((5, 6)));

    // `A` confirms there.
    let result = state
        .step_active_cast_followup('A', "", dir)
        .unwrap()
        .expect("`A` confirms the occupied cell");
    assert_eq!(result.0, MoveOutcome::Cast);
    assert!(state.active_cast_followup.is_none());
}

/// Item 4, the field spells' cursor (`magic.md §8`: "the standard combat
/// arena cursor"): the same table - diagonal moves within range, digits
/// inert, `A` confirming off the caster's cell.
#[test]
fn rc2_field_spell_cursor_takes_a_and_diagonals() {
    let dir = std::path::Path::new("");
    let mut state = playtest_spell_cursor_state();
    state.step_active_cast('F', "GI", dir).unwrap();
    state.step_active_cast('\r', "", dir).unwrap();
    assert_eq!(state.message, "Aim! ");
    state.step_active_cast_followup('7', "", dir).unwrap();
    assert_eq!(state.combat_aim_marker_cell, Some((5, 5)));
    state
        .step_active_cast_followup(char::from(INPUT_CODE_NORTHWEST), "", dir)
        .unwrap();
    assert_eq!(state.combat_aim_marker_cell, Some((4, 4)));
    let result = state
        .step_active_cast_followup('a', "", dir)
        .unwrap()
        .expect("`A` confirms the field cell");
    assert_eq!(result.0, MoveOutcome::Cast);
}

/// Item 4, the Attack cursor: the one key table both cursors classify with.
#[test]
fn rc2_attack_cursor_table_matches_the_spell_cursor() {
    assert_eq!(
        combat_targeting_cursor_input('A'),
        CombatTargetingCursorInput::Confirm
    );
    assert_eq!(
        combat_targeting_cursor_input('a'),
        CombatTargetingCursorInput::Confirm
    );
    assert_eq!(
        combat_targeting_cursor_input(char::from(INPUT_CODE_NORTHEAST)),
        CombatTargetingCursorInput::Move(InputDirection::Northeast)
    );
    for digit in '0'..='9' {
        assert_eq!(
            combat_targeting_cursor_input(digit),
            CombatTargetingCursorInput::Ignored,
            "{digit}"
        );
    }
}

/// Item 5: the dungeon Jimmy picker runs `commands.md §5.8`'s exchange
/// (`dungeon-mode.md §8.1`, "Who acts"): "A pick whose status is neither
/// Good nor Poisoned answers `Disabled!\n\n` and re-prompts from
/// `Player: `"; `0` is ignored at the chest sites; Escape answers `None!`.
#[test]
fn rc2_dungeon_jimmy_picker_rejects_ineligible_and_ignores_zero() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    let index = dungeon_cell_index(0, 1, 1);
    grid[index] = 0x4b;
    let mut state = dungeon_state(grid, 0, 1, 1);
    let _ = scene;
    state.keys = 3;
    let mut second = state.party[0];
    second.slot = 1;
    state.party.push(second);
    let mut asleep = state.party[0];
    asleep.slot = 2;
    asleep.status = b'S';
    state.party.push(asleep);

    assert_eq!(
        state
            .jimmy_direction_with_game_dir_and_member(Direction::North, None, None)
            .unwrap(),
        MoveOutcome::Observed
    );
    assert!(state.active_jimmy.is_some());
    let dir = std::path::Path::new("");

    // `0` is read past; the prompt stays open and nothing is spent.
    assert_eq!(state.step_active_jimmy('0', "", dir).unwrap(), None);
    assert!(state.active_jimmy.is_some());
    assert_eq!(state.turn, 0);

    // The Asleep member is refused with `Disabled!` and the prompt repeats.
    assert_eq!(state.step_active_jimmy('3', "\r", dir).unwrap(), None);
    assert!(state.active_jimmy.is_some());
    // ... and re-prompts from `Player: ` (playtest F13).
    assert!(state
        .message_entries()
        .iter()
        .any(|entry| entry.text == DUNGEON_ACTING_MEMBER_DISABLED.trim_end()));
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
    assert_eq!(state.grid[index], 0x4b);
    assert_eq!(state.keys, 3);

    // Escape cancels with `None!`, "on the prompt row" (`commands.md
    // §5.8`) - after the re-prompt that is the bare `Player: ` row.
    assert_eq!(
        state.step_active_jimmy('\u{1b}', "", dir).unwrap(),
        Some(MoveOutcome::PromptDeclined)
    );
    assert!(state.active_jimmy.is_none());
    assert_eq!(state.message, "Player: None!");
    assert_eq!(
        state.message_transcript.last().map(|entry| entry.text.as_str()),
        Some("Player: None!")
    );
    assert_eq!(state.grid[index], 0x4b);
}

/// Item 5: Return and Space commit the picker's indicated row
/// (`inventory.md §4.3`, the roster picker the exchange opens); the
/// direction keys move it. Neither is a cancel at the Jimmy or chest sites.
#[test]
fn rc2_acting_member_pickers_commit_the_indicated_row_on_space() {
    assert_eq!(
        acting_member_picker_key(' ', 1, 3),
        ActingMemberPickerKey::Pick(1)
    );
    assert_eq!(
        acting_member_picker_key('\r', 0, 3),
        ActingMemberPickerKey::Pick(0)
    );
    assert_eq!(
        acting_member_picker_key('0', 0, 3),
        ActingMemberPickerKey::Ignore
    );
    assert_eq!(
        acting_member_picker_key('4', 0, 3),
        ActingMemberPickerKey::Ignore
    );
    // `inventory.md §4.3`: a digit moves the indicator; it does not
    // commit (playtest F10).
    assert_eq!(
        acting_member_picker_key('2', 0, 3),
        ActingMemberPickerKey::Move(1)
    );
    assert_eq!(
        acting_member_picker_key('\u{1b}', 0, 3),
        ActingMemberPickerKey::Cancel
    );
    assert_eq!(
        acting_member_picker_key(char::from(INPUT_CODE_SOUTH), 2, 3),
        ActingMemberPickerKey::Move(2)
    );
    assert_eq!(
        acting_member_picker_key(char::from(INPUT_CODE_SOUTH), 0, 3),
        ActingMemberPickerKey::Move(1)
    );

    // The town chest picker: move to the second member and commit on Space.
    let mut grid = open_grid();
    grid[32 + 2] = 0x4f;
    let mut state = test_state(grid, 1, 1);
    state.player.facing = Direction::East;
    let mut second = state.party[0];
    second.slot = 1;
    state.party.push(second);
    state.active_objects.push(ActiveObject {
        type_byte: 0x4f,
        tile: 0x4f,
        x: 2,
        y: 1,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0x7f,
        aux3: 0,
    });
    assert_eq!(state.open_facing(), MoveOutcome::Observed);
    assert_eq!(state.selector_highlight(), Some(0));
    assert_eq!(
        state
            .step_active_surface_chest(char::from(INPUT_CODE_SOUTH), "")
            .unwrap(),
        None
    );
    assert_eq!(state.selector_highlight(), Some(1));
    assert_ne!(
        state.step_active_surface_chest(' ', "").unwrap(),
        Some(MoveOutcome::PromptDeclined)
    );
    assert!(state.active_surface_chest.is_none());
    assert_ne!(state.message, "None!");
}

/// Item 6: `dungeon-mode.md §13.1` - a level change "reports \"acted\" to
/// the dungeon loop, so the turn's post-action pass runs", and Uus Por /
/// Des Por are the same transition (they print the climb's own word and
/// nothing else). The pass ran for K-Klimb (playtest E08) but the cast
/// paths sent the spell's level change through the generic hook, which
/// skips every transition. The pass's immediate panel repaint
/// (`stats-panel.md §2.2.1`) and its underfoot read now follow the spell.
#[test]
fn rc2_level_change_spells_run_the_post_action_pass() {
    let scene = DungeonScene::new(33).unwrap();
    for (route, interactive) in [("suffix", false), ("prompt", true)] {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        arm_spell(&mut state, DES_POR_SPELL_INDEX);
        state.repaint_stats_panel();
        let stale = state.displayed_stats_panel().clone();
        // A change the panel has not been repainted for: only a repaint
        // shows it.
        state.gold += 5;
        if interactive {
            handle_play_key_input(&mut state, 'C', "", Path::new("")).unwrap();
            for key in ['D', 'P', '\r'] {
                if state.active_cast.is_none() && state.active_cast_followup.is_none() {
                    break;
                }
                handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
            }
        } else {
            handle_play_key_input(&mut state, 'C', "1DP", Path::new("")).unwrap();
        }
        assert_eq!(state.area, Area::Dungeon { scene, level: 1 }, "{route}");
        assert_ne!(state.party[0].mana, 8, "{route}: the cast spent mana");
        assert_ne!(state.displayed_stats_panel(), &stale, "{route}");
        assert_eq!(
            state.displayed_stats_panel(),
            &crate::stats_panel::stats_panel_snapshot_of(&state),
            "{route}: the post-action pass repainted the panel"
        );
    }
}

/// Item 6: the shared pass, driven with a spell's level-change outcome,
/// reads the landing cell the same way it does for K-Klimb.
#[test]
fn rc2_level_change_pass_reads_the_landing_cell() {
    let scene = DungeonScene::new(33).unwrap();
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(1, 1, 1)] = 0xF2;
    let mut state = dungeon_state(grid, 1, 1, 1);
    let turn_before = state.turn;
    state.advance_turn();
    let outcome = MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel { scene, level: 1 });
    state
        .apply_post_turn_effects_after_dungeon_level_change(turn_before, Path::new(""), outcome)
        .unwrap();
    assert_eq!(state.message, DUNGEON_ROOM_ENTRY_NARRATION);
    assert_eq!(state.dungeon_cell(1, 1, 1), 0xA2, "the room helper ran");
}

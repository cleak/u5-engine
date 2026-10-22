// Regressions for the magic exploratory play-test findings (G05..G12).
// Each test names the finding it reproduces and the published sentence it
// rests on.

fn p3magic_transcript_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

fn p3magic_three_member_world_state() -> PlayState {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0", *b"IOLO\0\0\0\0\0"];
    state.active_player = None;
    state
}

/// G05: `commands.md §5.8` - "With two or more, it prints `Player: ` and
/// opens the roster picker; a confirmed eligible pick completes that row
/// with the member's name. An ineligible pick prints `Disabled!\n\n` and
/// repeats the prompt", and "The direction form of Search always invokes
/// this acting-member selection before searching. Cast does too."
/// Eligible is Good or Poisoned (`traps.md §2.1`).
#[test]
fn playfix3_g05_cast_and_search_pickers_refuse_a_dead_or_sleeping_member() {
    for (status, target) in [
        (b'D', PartySelectorTarget::Cast),
        (b'S', PartySelectorTarget::Cast),
        (b'D', PartySelectorTarget::Search { direction: Direction::North }),
        (b'S', PartySelectorTarget::Search { direction: Direction::North }),
    ] {
        let mut state = p3magic_three_member_world_state();
        state.party[1].status = status;
        assert_eq!(state.start_party_selector(target), MoveOutcome::Observed);

        // Move to Shamino and commit.
        assert!(state.step_active_party_selector('2', ""));
        assert!(state.step_active_party_selector('\r', ""));
        assert!(
            state.active_party_selector.is_some(),
            "{target:?}/{}: the prompt repeats",
            status as char
        );
        assert!(state.active_cast.is_none(), "{target:?}: no cast opens");
        let rows = p3magic_transcript_rows(&state);
        assert!(
            !rows.iter().any(|row| row.contains("SHAMINO")),
            "{target:?}: the ineligible name is not echoed: {rows:?}"
        );
        let disabled = rows
            .iter()
            .rposition(|row| row == DUNGEON_ACTING_MEMBER_DISABLED.trim_end())
            .unwrap_or_else(|| panic!("{target:?}: `Disabled!` prints: {rows:?}"));
        assert_eq!(rows[disabled + 1], "", "{target:?}: {rows:?}");
        assert_eq!(state.message, PARTY_SELECTION_PROMPT);
        assert_eq!(state.turn, 0);

        // An eligible pick then proceeds as before.
        assert!(state.step_active_party_selector('3', ""));
        assert!(state.step_active_party_selector('\r', ""));
        assert!(state.active_party_selector.is_none());
        if target == PartySelectorTarget::Cast {
            assert_eq!(state.active_cast.as_ref().map(|cast| cast.caster_index), Some(2));
        }
    }
}

/// G08: `combat.md §15` - "Combat advances time at one specific point -
/// when the round counter wraps inside a round ... The wrap fires the
/// per-turn cleanup with a one-minute increment"; `§4`: "the only time a
/// fight adds is its round-wrap minutes". A combat cast spent the
/// overworld's two minutes and a turn.
#[test]
fn playfix3_g08_a_combat_cast_charges_no_clock_minutes_or_turn() {
    let spell = spell_index_from_code("IS").unwrap();
    for combat in [false, true] {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.combat_active = combat;
        state.party[0].mana = 50;
        state.party[0].level = 8;
        state.spell_charges[spell] = 1;
        state.active_player = Some(0);
        state.clock = GameClock::new(13, 0).unwrap();

        handle_play_key_input(&mut state, 'C', "1IS", Path::new("")).unwrap();
        assert_eq!(state.spell_charges[spell], 0, "combat {combat}: the cast resolved");
        let elapsed = (state.clock.hour, state.clock.minute, state.turn);
        if combat {
            assert_eq!(elapsed, (13, 0, 0), "a fight charges no per-cast time");
        } else {
            assert_eq!(elapsed, (13, 2, 1), "the overworld turn is two minutes");
        }
    }
}

/// G06: `magic.md §6` step 6 - the handler "prints `Mixing...`" once, and
/// step 7's wrong-mix branch "then emits a line break ... and invokes the
/// shared trap-effect resolver". Nothing prints `Mixing...` a second time.
#[test]
fn playfix3_g06_a_wrong_recipe_prints_the_mixing_beat_once() {
    for suffix in ["AS/0x80/1", "IL/0x41/1"] {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [5; REAGENT_COUNT];
        assert_eq!(state.mix_reagents_from_suffix(suffix), MoveOutcome::Blocked);
        state.flush_message_slot();
        let rows = p3magic_transcript_rows(&state);
        let mixing: Vec<usize> = rows
            .iter()
            .enumerate()
            .filter(|(_, row)| *row == MMIX_MIXING_MESSAGE)
            .map(|(index, _)| index)
            .collect();
        assert_eq!(mixing.len(), 1, "{suffix}: {rows:?}");
        assert!(
            matches!(
                rows[mixing[0] + 1].as_str(),
                "ACID!" | "POISON!" | "BOMB!" | "GAS!"
            ),
            "{suffix}: the trap word follows directly: {rows:?}"
        );
    }
}

/// G09: `main-loop.md §6` runs dispatch (step 3) before the per-turn
/// cleanup (step 4), and `§8` step 1: "If the cleanup sees the `Q` tag,
/// halve the increment ... If it sees the `T` tag, skip the minute".
/// `magic.md §8`: the countdown ages at "the per-turn cleanup that follows
/// command dispatch". The casting turn's own cleanup therefore sees the new
/// tag and spends the new duration's first unit.
#[test]
fn playfix3_g09_a_timed_effect_spell_takes_hold_on_its_own_casting_turn() {
    // (code, spell index, expected clock minute after the cast, tag)
    for (code, spell, minute, tag) in [
        ("RT", QUICKNESS_SPELL_INDEX, 1, b'Q'),
        ("AT", TIME_STOP_SPELL_INDEX, 0, b'T'),
        ("IS", PROTECTION_SPELL_INDEX, 2, b'P'),
    ] {
        let mut state = world_state(open_world_grid(), 5, 5);
        state.party[0].mana = 50;
        state.party[0].level = 8;
        state.spell_charges[spell] = 1;
        state.active_player = Some(0);
        state.clock = GameClock::new(13, 0).unwrap();

        handle_play_key_input(&mut state, 'C', &format!("1{code}"), Path::new("")).unwrap();
        assert_eq!(state.spell_charges[spell], 0, "{code}: the cast resolved");
        assert_eq!(state.active_effect_tag, Some(tag), "{code}");
        assert_eq!(
            (state.clock.hour, state.clock.minute),
            (13, minute),
            "{code}: the casting turn's cleanup sees the tag"
        );
        let duration = match tag {
            b'Q' => 30,
            b'T' => TIME_STOP_DURATION,
            _ => 20,
        };
        assert_eq!(
            state.active_effect_counter,
            duration - 1,
            "{code}: the casting turn ages the new duration"
        );
    }
}

/// G11: `magic.md §5.1` - "submitting the name advances to the next row
/// before any result/refusal is printed", and `§7`: "Combat's subsequent
/// actor banner has its own leading feed, which leaves a blank row after a
/// completed result". In Sanct prints no result line, and a cancelled
/// `On who:` ends on `None!`; either way one blank row stands before the
/// next banner, as it does after `Not here!` (E40).
#[test]
fn playfix3_g11_a_resultless_combat_cast_leaves_a_blank_row_before_the_banner() {
    let game_dir = std::path::Path::new(".");
    for (keys, last_exchange_row) in [
        (&['C', 'I', 'S', '\r'][..], ":IN SANCT"),
        (&['C', 'M', '\r', '\u{1b}'][..], "On who: None!"),
    ] {
        let mut state = combat_state_after_the_conflict_banner(8, 5);
        state.active_player = Some(0);
        state.party[0].mana = 50;
        state.party[0].level = 8;
        state.spell_charges = [9; SPELL_COUNT];
        state.ensure_pending_combat_player_turn();

        for &key in keys {
            handle_play_key_input(&mut state, key, "", game_dir).unwrap();
        }

        let rows = combat_message_window_rows(&state);
        let exchange = rows
            .iter()
            .rposition(|row| row == last_exchange_row)
            .unwrap_or_else(|| panic!("{last_exchange_row} is drawn: {rows:?}"));
        assert_eq!(rows[exchange + 1], "", "{last_exchange_row}: {rows:?}");
        assert_eq!(rows[exchange + 2], "Avatar, armed", "{last_exchange_row}: {rows:?}");
    }
}

/// G12: `inventory.md §7.1`'s completion table gives the Sextant
/// `Sextant\n\n`, so one blank row stands between the completed `Item:`
/// row and `Position:`, as it does for the Spyglass and the Watch.
#[test]
fn playfix3_g12_a_sextant_reading_opens_under_a_blank_row() {
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
    state.clock = GameClock::new(22, 0).unwrap();
    let row = state
        .use_item_picker_rows()
        .iter()
        .position(|row| row.request == UseItemRequest::Sextant)
        .expect("the Sextant row is listed");

    handle_play_key_input(&mut state, 'U', "", Path::new("")).unwrap();
    for _ in 0..row {
        handle_play_key_input(&mut state, char::from(INPUT_CODE_SOUTH), "", Path::new(""))
            .unwrap();
    }
    handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap();
    state.flush_message_slot();

    let rows = p3magic_transcript_rows(&state);
    let item = rows
        .iter()
        .rposition(|row| row == "Item: Sextant")
        .unwrap_or_else(|| panic!("the item row completes: {rows:?}"));
    assert_eq!(rows[item + 1], "", "{rows:?}");
    assert_eq!(rows[item + 2], USE_SEXTANT_READING_LABEL, "{rows:?}");
}

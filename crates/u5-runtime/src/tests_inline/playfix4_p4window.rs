// Regressions for the message-window exploratory play-test findings
// (H06..H45). Each test names the finding it reproduces and the published
// sentence it rests on.

/// The thirteen rows the gameplay window draws for `state`, composed the way
/// the headless `--save-screen` path and `message_window_probe` compose them:
/// the transcript, the unflushed slot, the live row and its kind. The end-cap
/// marker is written as `>` and a blank row as an empty string.
fn p4window_rows(state: &PlayState) -> Vec<String> {
    let keep = |text: &str| (!text.trim().is_empty()).then(|| text.to_string());
    let mut log = message_log_from_entries(state.displayed_message_entries(), keep);
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    if let Some(text) = state.displayed_unflushed_message().and_then(keep) {
        log.push_output(&text);
    }
    let open_prompt = state.open_prompt_line();
    let spell_echo = state.typed_prompt_echo();
    let live_row = if state.message_window_live_row_suppressed() {
        None
    } else {
        Some(spell_echo.as_deref().unwrap_or(""))
    };
    let live_row_kind =
        if spell_echo.is_some() || dungeon_fountain_prompt_row_is_continuation(state) {
            LiveRowKind::Continuation
        } else {
            LiveRowKind::CommandRow
        };
    let layout = layout_message_window_with_continuation(
        &log,
        live_row,
        open_prompt.as_deref(),
        combat_prompt_row_follows_history(state),
        live_row_kind,
    );
    let mut rows = vec![String::new(); MESSAGE_WINDOW_ROWS];
    for row in &layout.rows {
        rows[usize::from(row.row - MESSAGE_WINDOW_TOP)] =
            format!("{}{}", if row.prefixed { ">" } else { "" }, row.text);
    }
    rows
}

/// The rows from the last one equal to `anchor` to the bottom of the window.
fn p4window_rows_from(rows: &[String], anchor: &str) -> Vec<String> {
    let start = rows
        .iter()
        .rposition(|row| row == anchor)
        .unwrap_or_else(|| panic!("`{anchor}` is on screen: {rows:?}"));
    let mut tail = rows[start..].to_vec();
    while tail.last().is_some_and(String::is_empty) {
        tail.pop();
    }
    tail
}

fn p4window_keys(state: &mut PlayState, dir: &Path, keys: &[char]) {
    for key in keys {
        handle_play_key_input(state, *key, "", dir).unwrap();
        // Let any staged presentation (a spell's flash hold) play out, as
        // the headless runner does between keys.
        let mut clock = 0.0;
        while state.advance_staged_narration_to(clock) && clock < 1000.0 {
            clock += 10.0;
        }
    }
}

fn p4window_world_state() -> PlayState {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_player = Some(0);
    state
}

/// H25: `commands.md §5.2` - `Get-North` ⏎ `[blank]` ⏎ `Nothing to get!` -
/// and `text-output.md §10.4`: "nothing tracks an open row", so an earlier
/// cast cannot take that blank away. `magic.md §5.1`: "submitting the name
/// advances to the next row before any result/refusal is printed".
#[test]
fn playfix4_h25_a_submitted_spell_name_leaves_no_open_row_behind() {
    let dir = debug_game_dir();
    let mut plain = p4window_world_state();
    p4window_keys(&mut plain, &dir, &['G', '\u{3}']);
    let expected = p4window_rows_from(&p4window_rows(&plain), ">Get-North");

    let mut state = p4window_world_state();
    // `I` alone matches no token: `No effect!`.
    p4window_keys(&mut state, &dir, &['C', 'I', '\r']);
    assert!(
        state.message_transcript.iter().any(|entry| entry.text == "No effect!"),
        "the cast ran: {:?}",
        p4window_rows(&state)
    );
    assert!(!state.message_row_open_mid_line, "the submitted name closed its row");
    p4window_keys(&mut state, &dir, &['G', '\u{3}']);
    let after_cast = p4window_rows_from(&p4window_rows(&state), ">Get-North");
    assert_eq!(after_cast, expected);
    assert_eq!(after_cast.get(1).map(String::as_str), Some(""), "{after_cast:?}");
    let _ = fs::remove_dir_all(dir);
}

/// H31 (a): `magic.md §5.1` - In Lor prints no result, and "submitting the
/// name advances to the next row", so `:IN LOR`, a blank row, then the
/// marker. `text-output.md §10.2`/`§10.4`: that pair is emitted once, before
/// the key read, so typing the next key must not take the blank away.
#[test]
fn playfix4_h31_the_blank_under_a_resultless_cast_survives_the_next_key() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.spell_charges[spell_index_from_code("IL").unwrap()] = 1;
    state.party[0].mana = 50;
    state.party[0].level = 8;
    p4window_keys(&mut state, &dir, &['C', 'I', 'L', '\r']);
    let idle = p4window_rows_from(&p4window_rows(&state), ":IN LOR");
    assert_eq!(idle, [":IN LOR", "", ">"], "waiting frame");

    p4window_keys(&mut state, &dir, &['R']);
    let rows = p4window_rows(&state);
    let after = p4window_rows_from(&rows, ":IN LOR");
    assert_eq!(after.get(1).map(String::as_str), Some(""), "{rows:?}");
    assert_eq!(after.get(2).map(String::as_str), Some(">Ready..."), "{rows:?}");
    let _ = fs::remove_dir_all(dir);
}

/// H31 (b): Escape at R-Ready's `Player: ` prints nothing (`inventory.md
/// §4.3`, §5.1), so the row is still open and `text-output.md §10.4`'s next
/// poll closes it: "the prompt marker lands on the row directly beneath".
/// The waiting frame and the echo typed into it must agree.
#[test]
fn playfix4_h31_an_escaped_ready_player_prompt_keeps_its_marker_beneath() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.party.push(audit_g5_companion(1));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0"];
    state.equipment_stock[0x10] = 1;
    p4window_keys(&mut state, &dir, &['R', '\u{1b}']);
    let idle = p4window_rows_from(&p4window_rows(&state), "Player:");
    assert_eq!(idle, ["Player:", ">"], "waiting frame");

    p4window_keys(&mut state, &dir, &['S']);
    let after = p4window_rows_from(&p4window_rows(&state), "Player:");
    assert_eq!(after.get(1).map(String::as_str), Some(">Search-"), "{after:?}");
    let _ = fs::remove_dir_all(dir);
}

fn p4window_two_member_world_state() -> PlayState {
    let mut state = p4window_world_state();
    state.party.push(audit_g5_companion(1));
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0"];
    state
}

/// H26/H06: `magic.md §8` (Awaken/Cure) - the party-target spells "ask
/// **`On who: `**, echo the chosen member's name and complete the row";
/// `text-output.md §10.2` - "a prompt a command parks at in the middle of its
/// own run ... emits no feed, no marker"; `inventory.md §4.3` - every command
/// that needs a party member shares the `Select:` label and the inverted row,
/// and "A digit moves the indicator".
#[test]
fn playfix4_h26_a_spell_on_who_prompt_waits_inline_on_the_selection_surface() {
    let dir = debug_game_dir();
    let mut state = p4window_two_member_world_state();
    state.spell_charges[HEAL_SPELL_INDEX] = 1;
    state.party[0].mana = 20;
    state.party[0].level = 8;
    p4window_keys(&mut state, &dir, &['C', 'M', '\r']);
    assert!(
        state
            .active_cast_followup
            .as_ref()
            .is_some_and(|session| session.kind == CastFollowupKind::PartyTarget),
        "the target prompt waits"
    );
    assert_eq!(state.open_prompt_line().as_deref(), Some(USE_POTION_TARGET_PROMPT));
    let rows = p4window_rows_from(&p4window_rows(&state), ":MANI");
    assert_eq!(rows, [":MANI", "On who:"], "no blank and no marker row beneath");
    assert_eq!(state.roster_box_label().as_deref(), Some("Select:"));
    assert_eq!(state.selector_highlight(), Some(0));

    p4window_keys(&mut state, &dir, &['2']);
    assert_eq!(state.selector_highlight(), Some(1), "a digit moves the indicator");
    p4window_keys(&mut state, &dir, &['\r']);
    assert!(state.active_cast_followup.is_none());
    let completed = format!("On who: {}", state.party_member_display_name(1));
    assert!(
        state.message_transcript.iter().any(|entry| entry.text == completed),
        "{:?}",
        p4window_rows(&state)
    );
    assert_eq!(state.selector_highlight(), None);
    let _ = fs::remove_dir_all(dir);
}

/// H26: Gate Travel "prompts `To phase:_`" (`magic.md §8`) and echoes the
/// typed key on that row, so it is the same parked mid-command prompt.
#[test]
fn playfix4_h26_the_gate_travel_phase_prompt_keeps_the_cursor_on_its_row() {
    let mut state = p4window_world_state();
    state.active_cast_followup = Some(CastFollowupSession {
        caster_index: 0,
        spell_code: "PRV".to_string(),
        kind: CastFollowupKind::GatePhase,
        buffer: String::new(),
        combat_actor_slot: None,
        combat_had_foe: false,
    });
    assert_eq!(state.open_prompt_line().as_deref(), Some(GATE_TRAVEL_PHASE_PROMPT));
}

/// H06: the potion's `On who: ` already drew `Select:`, but `inventory.md
/// §4.3`'s inverted row never followed its indicator.
#[test]
fn playfix4_h06_the_potion_on_who_prompt_inverts_the_indicated_member() {
    let mut state = p4window_two_member_world_state();
    state.active_use = Some(UseSession {
        cursor: 0,
        pending: Some(UsePendingAction::PotionTarget { index: 0, highlight: 1 }),
        strip_scrolls: 0,
    });
    assert_eq!(state.roster_box_label().as_deref(), Some("Select:"));
    assert_eq!(state.selector_highlight(), Some(1));
}

/// H37: `input.md §11` - "Space is accepted as `Pass`: it echoes `Pass`",
/// as the cardinals echo their labels; `text-output.md §10.3` - the
/// direction prompt appends `North` ... or `Pass` "to the open line".
#[test]
fn playfix4_h37_space_at_a_spell_direction_prompt_completes_the_row_with_pass() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.spell_charges[spell_index_from_code("AS").unwrap()] = 1;
    state.party[0].mana = 20;
    state.party[0].level = 8;
    p4window_keys(&mut state, &dir, &['C', 'A', 'S', '\r']);
    assert!(
        state.active_cast_followup.as_ref().is_some_and(|session| matches!(
            session.kind,
            CastFollowupKind::Direction { .. }
        )),
        "{:?}",
        p4window_rows(&state)
    );
    p4window_keys(&mut state, &dir, &[' ']);
    let texts: Vec<&str> =
        state.message_transcript.iter().map(|entry| entry.text.as_str()).collect();
    assert!(texts.contains(&"Direction-Pass"), "{texts:?}");
    assert!(!texts.contains(&"Pass"), "{texts:?}");
    let rows = p4window_rows_from(&p4window_rows(&state), "Direction-Pass");
    assert_eq!(rows, ["Direction-Pass", "", ">"]);
    let _ = fs::remove_dir_all(dir);
}

/// H07: `inventory.md §6` - arrows and quarrels: "selecting either row exits
/// the cascade at the very top, with no mutation and no message at all - the
/// refusal is silent"; `§5.2`: "Selecting an ammunition row itself is
/// silent", so no `\n\n<msg>\n\nItem:_` re-prompt follows it.
#[test]
fn playfix4_h07_readying_an_arrows_row_prints_nothing() {
    let dir = debug_game_dir();
    let mut state = p4window_world_state();
    state.equipment_stock[EQUIPMENT_ID_ARROWS] = 99;
    p4window_keys(&mut state, &dir, &['R', '\r']);
    let session = state.active_ready.clone().expect("the item picker is open");
    assert_eq!(session.selected_party_index, Some(0));
    let before: Vec<String> =
        state.message_transcript.iter().map(|entry| entry.text.clone()).collect();
    let rows_before = p4window_rows(&state);

    p4window_keys(&mut state, &dir, &['\r', '\r']);
    let after: Vec<String> =
        state.message_transcript.iter().map(|entry| entry.text.clone()).collect();
    assert_eq!(after, before, "no row is printed");
    assert_eq!(p4window_rows(&state), rows_before);
    assert!(state.active_ready.is_some(), "the picker stays open");
    assert_eq!(state.equipment_stock[EQUIPMENT_ID_ARROWS], 99);
    let _ = fs::remove_dir_all(dir);
}

/// H34/H45: `commands.md §5` - `U` echoes `Use_item\n\n` and `M` echoes
/// `Mix_Reagents\n\n`, and "The echo is printed **before** the handler's
/// precondition check", so `No usable items!` (`inventory.md §7.1`) and
/// `No reagents owned!` (`magic.md §6` step 1) sit under a blank row, exactly
/// where `Item: ` and `For what spell?` would.
#[test]
fn playfix4_h34_h45_empty_use_and_mix_refusals_keep_the_echo_blank_row() {
    let dir = debug_game_dir();
    // The dungeon fixture carries nothing usable (no moonstone can be
    // buried underground) and no reagents.
    for (key, echo, refusal) in [
        ('U', "Use item", USE_NO_USABLE_ITEMS_REFUSAL),
        ('M', "Mix Reagents", MMIX_NO_REAGENTS_OWNED_MESSAGE),
    ] {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        p4window_keys(&mut state, &dir, &[key]);
        let texts: Vec<&str> =
            state.message_transcript.iter().map(|entry| entry.text.as_str()).collect();
        assert_eq!(texts, [echo, "", refusal], "{key}");
    }
    let mut state = p4window_world_state();
    state.reagents = [0; REAGENT_COUNT];
    p4window_keys(&mut state, &dir, &['M']);
    let texts: Vec<&str> =
        state.message_transcript.iter().map(|entry| entry.text.as_str()).collect();
    assert_eq!(texts, ["Mix Reagents", "", MMIX_NO_REAGENTS_OWNED_MESSAGE]);
    let _ = fs::remove_dir_all(dir);
}

/// H38: `text-output.md §5` - the per-cell emitter wraps the cursor once a
/// glyph fills the row's last cell; `§6` - "A line feed arriving from any
/// **later** call is unconditional and always costs a row". A typed Yell word
/// that fills all sixteen cells is followed by two blank rows, a shorter one
/// by one (`commands.md §5`: `:WORD` ⏎ `[blank]` ⏎ `No effect!`).
#[test]
fn playfix4_h38_a_typed_answer_that_fills_its_row_is_followed_by_two_blank_rows() {
    let dir = debug_game_dir();
    for (word, blanks) in [("abcdefghijklmno", 2), ("abcdefghijklmn", 1)] {
        let mut state = p4window_world_state();
        p4window_keys(&mut state, &dir, &['Y']);
        // The play-script runner hands a typed word over as one key plus
        // its suffix.
        handle_play_key_input(&mut state, word.chars().next().unwrap(), &word[1..], &dir)
            .unwrap();
        let echo = format!(":{}", word.to_ascii_uppercase());
        let rows = p4window_rows(&state);
        let tail = p4window_rows_from(&rows, &echo);
        let mut expected = vec![echo.clone()];
        expected.extend(std::iter::repeat_n(String::new(), blanks));
        expected.push(YELL_NO_EFFECT_MESSAGE.to_string());
        assert_eq!(tail[..expected.len()], expected[..], "{word}: {rows:?}");
    }
    let _ = fs::remove_dir_all(dir);
}

/// H26 through the key path: the Gate Travel `To phase: ` row is the open
/// line the cursor waits on, with no blank and no marker row beneath it, and
/// the typed key completes that same row.
#[test]
fn playfix4_h26_the_gate_travel_phase_prompt_waits_inline_through_the_key_path() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 1, 1);
    state.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    state.party[0].mana = GATE_TRAVEL_COST;
    state.party[0].level = GATE_TRAVEL_COST;
    state.active_player = Some(0);
    p4window_keys(&mut state, &dir, &['C', 'P', 'R', 'V', '\r']);
    assert!(
        state
            .active_cast_followup
            .as_ref()
            .is_some_and(|session| session.kind == CastFollowupKind::GatePhase),
        "{:?}",
        p4window_rows(&state)
    );
    let rows = p4window_rows_from(&p4window_rows(&state), ":POR REL VAS");
    assert_eq!(rows, [":POR REL VAS", "To phase:"], "no blank and no marker row beneath");
    p4window_keys(&mut state, &dir, &['9']);
    let texts: Vec<&str> =
        state.message_transcript.iter().map(|entry| entry.text.as_str()).collect();
    assert!(texts.contains(&"To phase: 9"), "{texts:?}");
    let _ = fs::remove_dir_all(dir);
}

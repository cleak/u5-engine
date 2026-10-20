// Conformance for u5-spec a031913 (issue #299 follow-ups 13-19, R569-R572).
// Each test names the answer it pins and the published sentence it rests on.

/// u5-spec#299 Q17 / `RETRACTIONS.md` R571, `blackthorn.md §7` step 11: the
/// rescue spins "the ordinary clock advance in nine-minute steps until the
/// hour is 6. The hour is checked before each step and the minute is never
/// reset." The worked vectors: "4-28-139 23:55 becomes 5-1-139 06:04, and
/// 13-28-139 13:10 becomes 1-1-140 06:07"; "an entry hour of 6 makes no step
/// and keeps its date and minute unchanged"; an entry hour of 0 to 5 "ends the
/// same day at 06:00 to 06:08".
#[test]
fn spec299_q17_rescue_clock_spins_in_nine_minute_steps_to_hour_six() {
    let cases = [
        ((139, 4, 28, 23, 55), (139, 5, 1, 6, 4)),
        ((139, 13, 28, 13, 10), (140, 1, 1, 6, 7)),
        ((139, 3, 9, 6, 30), (139, 3, 9, 6, 30)),
        ((139, 3, 9, 5, 55), (139, 3, 9, 6, 4)),
        ((139, 3, 9, 0, 0), (139, 3, 9, 6, 0)),
    ];
    for ((year, month, day, hour, minute), expected) in cases {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::with_date(year, month, day, hour, minute).unwrap();
        state.party[0].status = b'D';
        state.party[0].hp = 0;
        state.repaint_stats_panel();

        state.apply_blackthorn_rescue_refuge(&dir).unwrap();
        state.run_blackthorn_rescue_to_handoff(&dir).unwrap();

        let clock = state.clock;
        assert_eq!(
            (clock.year, clock.month, clock.day, clock.hour, clock.minute),
            expected,
            "rescue from {year}-{month}-{day} {hour:02}:{minute:02}"
        );
        // "A midnight crossing runs the ordinary day rollover once, which
        // repaints the whole stats panel straight away, so the date row shows
        // the new date."
        let live = crate::stats_panel::stats_panel_snapshot_of(&state);
        assert_eq!(state.displayed_stats_panel().date_row, live.date_row);
        let _ = fs::remove_dir_all(dir);
    }
}

/// u5-spec#299 Q16, `shops.md §8.5` "A negative quantity" and `input.md §8`:
/// "The reader keeps a leading `-`, and the sign uses one of its two
/// positions, so `-1` is the only practical negative entry: `-99` echoes `-9`
/// and drops the last key, and `-0` or a bare `-` reads as zero (the
/// `Hrumph.` path). A leading `+` is accepted and ignored, so `+5` buys five."
#[test]
fn spec299_q16_provision_reader_sign_uses_one_of_two_positions() {
    fn typed(keys: &str) -> crate::shop_runtime::TypedQuantityEntry {
        let mut entry = crate::shop_runtime::TypedQuantityEntry::default();
        for byte in keys.bytes() {
            assert!(!entry.apply(byte));
        }
        entry
    }
    assert_eq!(typed("-99").as_str(), "-9");
    assert_eq!(typed("99").as_str(), "99");
    assert_eq!(typed("999").as_str(), "99");
    assert_eq!(typed("+5").quantity(), 5);
    assert_eq!(typed("+55").as_str(), "+5");
    assert_eq!(typed("-0").quantity(), 0);
    assert_eq!(typed("-").quantity(), 0);
    assert_eq!(typed("-1").quantity(), u16::MAX);
}

/// u5-spec#299 Q16, `shops.md §8.5`: the worked vectors "at a unit price of
/// 14" for a `-1` entry. "Each pass pays for one pack and adds its 25
/// servings **before** the loop tests the 9999 food stop"; "Food reaches
/// 9999 ... takes the completed-purchase tail"; "Gold runs out first ...
/// `"Thou canst\nafford only `, the number of packs actually bought"; "Not
/// one pack affordable ... the two nothing-served rows above apply".
#[test]
fn spec299_q16_negative_provision_quantity_buys_until_gold_or_ceiling() {
    use crate::shops::{
        ProvisionPurchaseCompletion, ProvisionPurchaseError, Tavern,
        apply_provision_purchase_at_unit_price,
    };
    let minus_one = u16::MAX;
    for (gold, food, packs, gold_after, food_after, completion) in [
        (100, 100, 7, 2, 275, ProvisionPurchaseCompletion::GoldExhausted),
        (9999, 100, 396, 4455, 9999, ProvisionPurchaseCompletion::Completed),
        (9999, 9990, 1, 9985, 9999, ProvisionPurchaseCompletion::Completed),
        (100, 9999, 1, 86, 9999, ProvisionPurchaseCompletion::Completed),
        (14, 9990, 1, 0, 9999, ProvisionPurchaseCompletion::Completed),
    ] {
        let (mut g, mut f) = (gold, food);
        let outcome = apply_provision_purchase_at_unit_price(
            &mut g,
            &mut f,
            Tavern::TheHonestMeal,
            minus_one,
            14,
        )
        .unwrap_or_else(|err| panic!("{gold}/{food}: {err:?}"));
        assert_eq!(outcome.purchased_quantity, packs, "{gold}/{food}");
        assert_eq!((g, f), (gold_after, food_after), "{gold}/{food}");
        assert_eq!(outcome.completion, completion, "{gold}/{food}");
    }
    for (gold, food) in [(13u16, 9999u16), (0, 100)] {
        let (mut g, mut f) = (gold, food);
        assert_eq!(
            apply_provision_purchase_at_unit_price(
                &mut g,
                &mut f,
                Tavern::TheHonestMeal,
                minus_one,
                14
            )
            .unwrap_err(),
            ProvisionPurchaseError::NoNeed
        );
        assert_eq!((g, f), (gold, food));
    }
    let (mut g, mut f) = (0u16, 2u16);
    let scraps = apply_provision_purchase_at_unit_price(
        &mut g,
        &mut f,
        Tavern::TheHonestMeal,
        minus_one,
        14,
    )
    .unwrap();
    assert_eq!(scraps.completion, ProvisionPurchaseCompletion::Charity);
    assert!(matches!(f, 3 | 4), "table scraps: {f}");
    assert_eq!(g, 0);
}

/// u5-spec#299 Q18, `dungeon-mode.md §11`: `For how many hours? (1-9) ` is
/// "printed once, after which the reader waits in the cell just after the
/// prompt's trailing space. Every key other than a digit or Space is
/// discarded silently - no echo, no reprint, no line feed, no message, no
/// cursor movement - and Escape does not cancel." Modifier-held top-row
/// digits arrive as direction codes and "are discarded the same way". "The
/// rows are therefore `Hole up & camp!`, a blank row, then `For how many
/// hours? (1-9) N`", and "The wait never advances the clock".
#[test]
fn spec299_q18_hours_prompt_discards_invalid_keys_without_reprinting() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    handle_play_key_input(&mut state, 'h', "", Path::new("")).unwrap();
    assert!(state.active_rest.is_some());
    state.flush_message_slot();
    let rows_at_prompt = playtest_transcript_rows(&state);
    let clock_at_prompt = state.clock;
    let turn_at_prompt = state.turn;

    for key in [
        'x',
        'Q',
        '\u{1b}',
        '\r',
        char::from(crate::INPUT_CODE_NORTH),
        char::from(crate::INPUT_CODE_WEST),
        char::from(crate::INPUT_CODE_NORTHEAST),
    ] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert!(state.active_rest.is_some(), "{key:?} must not end the prompt");
        state.flush_message_slot();
        assert_eq!(
            playtest_transcript_rows(&state),
            rows_at_prompt,
            "{key:?} must leave the window untouched"
        );
        assert_eq!(state.clock, clock_at_prompt, "{key:?} advanced the clock");
        assert_eq!(state.turn, turn_at_prompt, "{key:?} spent a turn");
    }

    handle_play_key_input(&mut state, '0', "", Path::new("")).unwrap();
    assert!(state.active_rest.is_none(), "`0` aborts");
    let rows = playtest_transcript_rows(&state);
    assert_eq!(
        rows.iter()
            .filter(|row| row.contains(REST_HOURS_PROMPT.trim_end()))
            .count(),
        1,
        "the prompt is printed once: {rows:?}"
    );
    assert_eq!(
        rows.last().map(String::as_str),
        Some(format!("{REST_HOURS_PROMPT}0").as_str()),
        "{rows:?}"
    );
}

/// u5-spec#299 Q18, `dungeon-mode.md §11`: "when two or more members are
/// Good or Poisoned, `\nWilt thou set a watch? `, answered `No\n\n` or
/// `Yes\n\n` (every other key at that prompt is discarded the same silent
/// way; with only one eligible member the question is skipped and camp starts
/// directly after the hours row's line feed)".
#[test]
fn spec299_q18_watch_question_takes_only_y_or_n_and_counts_good_or_poisoned() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.party[2].status = b'P';
    for key in ['h', '3'] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
    }
    assert_eq!(state.message, REST_WATCH_PROMPT);
    for key in ['\u{1b}', ' ', '0', '\r', '\n', 'q', '1'] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert!(state.active_rest.is_some(), "{key:?} must be discarded");
        assert_eq!(state.turn, 0, "{key:?} started the camp");
        assert!(
            !playtest_transcript_rows(&state)
                .iter()
                .any(|row| row.contains(REST_WATCH_NO_LITERAL.trim_end())),
            "{key:?} answered the question"
        );
    }

    // A Sleeping member is neither Good nor Poisoned: with only one
    // eligible member left the question is skipped.
    let mut sleepy = dungeon_state(open_dungeon_record(), 0, 1, 1);
    sleepy.party.push(audit_g5_companion(1));
    sleepy.party[1].status = b'S';
    handle_play_key_input(&mut sleepy, 'h', "", Path::new("")).unwrap();
    // The camp itself needs the shipped data files this fixture lacks, so
    // only the skipped question is checked, not the camp's result.
    let _ = handle_play_key_input(&mut sleepy, '3', "", Path::new(""));
    assert!(sleepy.active_rest.is_none(), "no watch question");
    assert_ne!(sleepy.message, REST_WATCH_PROMPT);
}

fn spec299_inn_guest(registry_slot: u8, scene_marker: u8, name: &[u8]) -> InnGuestRecord {
    let mut padded = [0u8; SAVE_CHARACTER_NAME_LEN];
    padded[..name.len()].copy_from_slice(name);
    InnGuestRecord {
        registry_slot,
        scene_marker,
        name: padded,
        member: PartyMember {
            slot: registry_slot,
            class_byte: b'F',
            status: b'G',
            climb_stat: 10,
            mana: 0,
            hp: 30,
            max_hp: 30,
            level: 2,
        },
        strength: 15,
        intelligence: 12,
        experience: 0,
        equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
        stay_counter: 0,
    }
}

fn spec299_register_state() -> PlayState {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(2).unwrap(),
        floor: 0,
    };
    // Three guests at this inn and one at another, listed out of roster
    // order in the engine's guest list.
    state.inn_registry = vec![
        spec299_inn_guest(8, 2, b"CHARLIE"),
        spec299_inn_guest(6, 2, b"ALICE"),
        spec299_inn_guest(9, 3, b"FOREIGN"),
        spec299_inn_guest(7, 2, b"BOB"),
    ];
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::ServiceMenu {
        inn: Inn::TheWayfarerInn,
    }));
    handle_play_key_input(&mut state, 'P', "", Path::new("")).unwrap();
    assert!(state.message.contains("\"Who will\ncheck out?\" "), "{:?}", state.message);
    state
}

fn spec299_register_highlight(state: &PlayState) -> Option<u8> {
    use crate::shop_runtime::InnkeeperState;
    use crate::shop_session::ActiveShopSession;
    match state.active_shop {
        Some(ActiveShopSession::Innkeeper(InnkeeperState::PickUpCompanion {
            highlight, ..
        })) => Some(highlight),
        _ => None,
    }
}

/// u5-spec#299 Q15, `shops.md §8.C` "Guest Register keys": "The highlight
/// starts on the topmost listed guest. West or North moves it up one listed
/// guest and East or South moves it down one. There is no wrap: at either end
/// the key does nothing. Return or Space commits the highlighted guest."
/// "Every other key is ignored without a redraw: the digits `0`-`9`, letters,
/// the four diagonal codes and line feed. Digits never move the highlight and
/// never commit." `§8.4` (Q14): the register scans "records 1 through 15".
#[test]
fn spec299_q15_guest_register_moves_by_direction_and_commits_on_space() {
    let mut state = spec299_register_state();
    let north = char::from(crate::INPUT_CODE_NORTH);
    let south = char::from(crate::INPUT_CODE_SOUTH);
    let east = char::from(crate::INPUT_CODE_EAST);
    let west = char::from(crate::INPUT_CODE_WEST);
    assert_eq!(spec299_register_highlight(&state), Some(0));
    state.flush_message_slot();
    let rows = playtest_transcript_rows(&state);
    for (key, expected) in [
        ('1', 0),
        ('2', 0),
        ('3', 0),
        ('0', 0),
        ('x', 0),
        ('\n', 0),
        (char::from(crate::INPUT_CODE_SOUTHEAST), 0),
        (north, 0),
        (west, 0),
        (south, 1),
        (east, 2),
        (south, 2),
        ('2', 2),
        (west, 1),
    ] {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
        assert_eq!(
            spec299_register_highlight(&state),
            Some(expected),
            "after {key:?}"
        );
        assert_eq!(state.party.len(), 1, "{key:?} committed a guest");
        state.flush_message_slot();
        assert_eq!(playtest_transcript_rows(&state), rows, "{key:?} redrew");
    }
    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();
    assert_eq!(state.party.len(), 2);
    assert_eq!(&state.party_names[1][..3], b"BOB", "the second record in roster order");
}

/// u5-spec#299 Q15: "Escape prints `No one\n\n` and picks no one, leaving the
/// party, the inn markers and gold unchanged."
#[test]
fn spec299_q15_guest_register_escape_picks_no_one() {
    let mut state = spec299_register_state();
    let registry = state.inn_registry.clone();
    handle_play_key_input(&mut state, char::from(crate::INPUT_CODE_SOUTH), "", Path::new(""))
        .unwrap();
    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();
    assert_eq!(state.party.len(), 1);
    assert_eq!(state.gold, 999);
    assert_eq!(state.inn_registry, registry);
    assert!(
        playtest_transcript_rows(&state).iter().any(|row| row.ends_with("No one")),
        "{:?}",
        playtest_transcript_rows(&state)
    );
}

/// u5-spec#299 Q19, `combat.md §8.4`: for a monster acting under player
/// control, "Starting from the turn banner, R gives `Orc:`, `Ready...`, a
/// blank row, `Can't!`, a blank row, `Orc:`; U is the same with `Use item`."
/// "The dash labels of G, J, O and S carry no newline, so the refusal shares
/// their row (`Get-Can't!` ...). Cast's own copy of the test prints `Cast...`
/// and its newline, then `Can't!` on the next row." No tone; the refusal is
/// free.
#[test]
fn spec299_q19_controlled_monster_shape_a_refusal_rows() {
    let game_dir = std::path::Path::new(".");
    for (key, expected) in [
        ('R', &["Ready...", "", "Can't!", ""][..]),
        ('U', &["Use item", "", "Can't!", ""][..]),
        ('G', &["Get-Can't!", ""][..]),
        ('J', &["Jimmy-Can't!", ""][..]),
        ('O', &["Open-Can't!", ""][..]),
        ('S', &["Search-Can't!", ""][..]),
        ('C', &["Cast...", "Can't!", ""][..]),
    ] {
        let mut state = combat_player_command_state(10, 10);
        state.keys = 1;
        state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        state.open_pending_combat_player_turn(Some(8));
        let banner = state
            .combat_turn_banner_for_actor(8)
            .expect("a controlled monster gets the reduced banner");
        let banner_row = banner.trim_matches('\n').to_string();
        state.flush_message_slot();
        let before = playtest_transcript_rows(&state);
        assert_eq!(before.last(), Some(&banner_row), "{key}: {before:?}");
        let round_before = state.combat_round_counter;
        let sounds_before = state.sound_effect_serial;

        handle_play_key_input(&mut state, key, "", game_dir).unwrap();
        state.flush_message_slot();

        let rows = playtest_transcript_rows(&state);
        let printed = rows[before.len()..]
            .iter()
            .map(|row| row.trim_end().to_string())
            .collect::<Vec<_>>();
        let mut want = expected.iter().map(|row| row.to_string()).collect::<Vec<_>>();
        want.push(banner_row.clone());
        assert_eq!(printed, want, "{key}");
        assert_eq!(state.combat_round_counter, round_before, "{key} is free");
        assert_eq!(state.pending_combat_actor_slot, Some(8), "{key}");
        assert_eq!(state.sound_effect_serial, sounds_before, "{key}: no tone");
    }
}

// rc4 residuals of u5-spec#299 (spec commits b42d672..a3ce96b).

fn rc4_enc(text: &str) -> Vec<u8> {
    text.bytes().map(|b| b ^ 0x80).collect()
}

fn rc4_decoded(raw: &[Vec<u8>]) -> Vec<String> {
    raw.iter()
        .map(|field| field.iter().map(|b| char::from(b ^ 0x80)).collect())
        .collect()
}

/// A blob whose Description ends in an IF-ELSE on the speaker's branch-flag
/// bit followed by a bare GOTO (`conversation.md §9` step 2's census shape:
/// "either a bare GOTO or an IF-ELSE followed by one"). The taken arm goes
/// to block `0x92`, the fall-through to block `0x91`.
fn rc4_description_goto_blob(head_91: Vec<u8>, head_92: Vec<u8>) -> Vec<Vec<u8>> {
    let mut description = rc4_enc("a stern lord.");
    description.extend([TLK_CODE_IF_ELSE, 0x92, 0x91]);
    let mut block_91 = vec![TLK_CODE_LABEL_RECORD, 0x91];
    block_91.extend(head_91);
    let mut block_92 = vec![TLK_CODE_LABEL_RECORD, 0x92];
    block_92.extend(head_92);
    vec![
        rc4_enc("Lord"),
        description,
        rc4_enc("Hail, friend."),
        rc4_enc("I rule."),
        rc4_enc("Farewell."),
        rc4_enc("RULE"),
        rc4_enc("I rule well."),
        block_91,
        rc4_enc("Pity."),
        rc4_enc("y"),
        rc4_enc("Good."),
        block_92,
        rc4_enc("Pity."),
        rc4_enc("y"),
        rc4_enc("Good."),
        vec![TLK_CODE_LABEL_RECORD, TLK_LABEL_LAST],
    ]
}

/// u5-spec#299 Q22(d), `conversation.md §9` step 2: "26 Descriptions end
/// with a live GOTO as their last byte ... None of those NPCs ever prints its
/// Greeting or introduces itself, whatever the acquaintance bit ... no
/// Greeting, no host-clock read, reseed or coin flip, and the opener returns
/// a stop." Every way the jumped-to block can leave the opener - a scoped
/// prompt, a nested `0xFF` prompt, an ASK-WHO, a `0x82` stop - is checked
/// with the bit clear and set.
#[test]
fn rc4_a_description_goto_always_stops_the_opener_with_no_greeting_or_reseed() {
    let mut ask_who_head = rc4_enc("Thy name?");
    ask_who_head.push(TLK_CODE_ASK_WHO);
    ask_who_head.extend(rc4_enc("Hm."));
    let mut stop_head = rc4_enc("Begone.");
    stop_head.push(TLK_CODE_END_STREAM);
    let mut nested_head = rc4_enc("Speak.\"\n\n");
    nested_head.push(TLK_CODE_END_OF_RESPONSE);
    let heads: [(&str, Vec<u8>); 4] = [
        ("scoped prompt", rc4_enc("Art thou loyal?")),
        ("nested 0xFF prompt", nested_head),
        ("ASK-WHO", ask_who_head),
        ("0x82 stop", stop_head),
    ];
    for (shape, head) in heads {
        for knows_party in [false, true] {
            let raw = rc4_description_goto_blob(head.clone(), head.clone());
            let decoded = rc4_decoded(&raw);
            let mut state = test_state(open_grid(), 1, 1);
            state.active_conversation_npc_slot = Some(1);
            if knows_party {
                state.set_active_talk_branch_flag(1);
            }
            let mut session = crate::conversation_session::ConversationSession::new(raw, decoded);
            session.set_npc_slot(Some(1));
            state.active_conversation = Some(Box::new(session));
            state.prng_state = 0x0aaa;

            let opening = state.active_conversation_greeting_rendered_with_host_seed(0x0456);
            let label = format!("{shape}, knows_party={knows_party}");
            assert!(!opening.text.contains("Hail"), "{label}: {:?}", opening.text);
            assert!(!opening.text.contains("I am called"), "{label}: {:?}", opening.text);
            assert_eq!(state.prng_state, 0x0aaa, "{label}: no reseed or coin flip");
            let session = state.active_conversation.as_ref().unwrap();
            assert!(
                session.opening_suspended() || session.has_ended(),
                "{label}: the opener returns"
            );
        }
    }
}

fn rc4_conversation_ctx<'a>() -> crate::conversation_session::ConversationContext<'a> {
    crate::conversation_session::ConversationContext {
        avatar_name: "Avatar",
        branch_flags: 0,
        moral_standing: 0,
        dictionary: None,
        gold_available: Some(0),
        party_member_names: &[],
    }
}

/// A blob with one ordinary keyword and one labelled block whose scoped
/// keywords are `n` then `y` (`conversation.md §7.7`'s matcher table).
fn rc4_scoped_blob() -> Vec<Vec<u8>> {
    let mut ask = rc4_enc("Well?");
    ask.push(0x91);
    let mut block = vec![TLK_CODE_LABEL_RECORD, 0x91];
    block.extend(rc4_enc("Shall I?"));
    vec![
        rc4_enc("Ada"),
        rc4_enc("a smith"),
        rc4_enc("Greetings."),
        rc4_enc("I mend gear."),
        rc4_enc("Farewell."),
        rc4_enc("JOIN"),
        ask,
        block,
        rc4_enc("Default."),
        rc4_enc("n"),
        rc4_enc("No answer."),
        rc4_enc("y"),
        rc4_enc("Yes answer."),
        vec![TLK_CODE_LABEL_RECORD, TLK_LABEL_LAST],
    ]
}

/// R577 / u5-spec#299 Q22(c), `conversation.md §6` step 5 and §7.7: both the
/// top-level and the scoped scans use the shared text matcher of
/// `commands.md §11.3` - its restart quirk included - and accept only a first
/// occurrence that starts the line or follows a literal space. `JACK JACK
/// OFF` shows the restart rule: the failed partial `JACK ` skips the second
/// `JACK`, so the rebuke word is never found (an ordinary substring search
/// would find it after the space).
#[test]
fn rc4_both_keyword_scans_use_the_published_matcher() {
    use crate::conversation_session::{ConversationSession, TlkKeywordLookup, tlk_keyword_lookup};
    assert!(!tlk_keyword_matches(b"JACK OFF", b"JACK JACK OFF"));
    assert!(tlk_keyword_matches(b"JACK OFF", b"SO JACK OFF"));

    let raw = rc4_scoped_blob();
    let decoded = rc4_decoded(&raw);
    // Top level: word-start, no end check, first occurrence only, and the
    // walk ends at the first declaration.
    for (line, expected) in [
        ("JOINED", TlkKeywordLookup::Field(6)),
        ("I WANT TO JOIN", TlkKeywordLookup::Field(6)),
        ("REJOIN", TlkKeywordLookup::NoMatch),
        ("JO IN", TlkKeywordLookup::NoMatch),
        ("REJOIN JOIN", TlkKeywordLookup::NoMatch),
        ("Y", TlkKeywordLookup::NoMatch),
        ("JOBS", TlkKeywordLookup::Field(3)),
        ("WELL BYE", TlkKeywordLookup::Field(4)),
        ("ASS", TlkKeywordLookup::Rebuke),
        ("JACK JACK OFF", TlkKeywordLookup::NoMatch),
        ("", TlkKeywordLookup::EmptyLine),
    ] {
        assert_eq!(tlk_keyword_lookup(Some(&raw), &decoded, line), expected, "{line}");
        let mut session = ConversationSession::new(raw.clone(), decoded.clone());
        session.present_greeting(&rc4_conversation_ctx());
        let out = session.submit_keyword(line, &rc4_conversation_ctx());
        match expected {
            TlkKeywordLookup::NoMatch => {
                assert!(out.text.contains(TLK_NO_KEYWORD_MATCH_MESSAGE.trim_end()), "{line}: {:?}", out.text)
            }
            TlkKeywordLookup::Rebuke => {
                assert!(out.text.contains(TLK_RESERVED_REBUKE_MESSAGE.trim_end()), "{line}: {:?}", out.text)
            }
            _ => assert!(!out.text.contains(TLK_NO_KEYWORD_MATCH_MESSAGE.trim_end()), "{line}"),
        }
    }

    // Scoped: §7.7's executed table; in `XN Y` the `n` keyword's first
    // occurrence is mid-word, so `n` fails and `y` answers.
    for (line, record) in [
        ("YONDER", "Yes answer."),
        ("GO YES", "Yes answer."),
        ("NAME", "No answer."),
        ("MAYBE YES", "Default."),
        ("BYE", "Default."),
        ("XN Y", "Yes answer."),
    ] {
        let mut session = ConversationSession::new(raw.clone(), decoded.clone());
        session.present_greeting(&rc4_conversation_ctx());
        session.submit_keyword("join", &rc4_conversation_ctx());
        assert!(session.awaiting_scoped_keyword(), "{line}");
        let out = session.submit_keyword(line, &rc4_conversation_ctx());
        assert!(out.text.contains(record), "{line}: {:?}", out.text);
    }
}

/// R579, `conversation.md §6` steps 3 and 4, §7.2 and §7.7 step 5: "A typed
/// `BYE` or `THANK` runs the Bye entry **without** printing the `BYE` line;
/// only the empty-line shortcut of step 3 prints it" - at the top-level
/// prompt, at a prompt nested by `0xFF`, and at a block's fallback loop. The
/// Bye entry runs once, and the conversation ends.
#[test]
fn rc4_a_typed_bye_or_thank_prints_no_bye_line_on_any_prompt() {
    use crate::conversation_session::ConversationSession;
    let mut nested = rc4_enc("Ask on.\"\n\n");
    nested.push(TLK_CODE_END_OF_RESPONSE);
    let mut raw = rc4_scoped_blob();
    raw.splice(7..7, [rc4_enc("MORE"), nested]);
    let decoded = rc4_decoded(&raw);
    let ctx = rc4_conversation_ctx();
    let bye_line = TLK_EMPTY_INPUT_BYE_MESSAGE.trim_end();
    for leave in ["BYE", "THANK", "WELL BYE", ""] {
        for route in ["top", "nested", "fallback"] {
            let mut session = ConversationSession::new(raw.clone(), decoded.clone());
            session.present_greeting(&ctx);
            match route {
                "nested" => {
                    let out = session.submit_keyword("more", &ctx);
                    assert!(!out.ended);
                }
                "fallback" => {
                    session.submit_keyword("join", &ctx);
                    let out = session.submit_keyword("y", &ctx);
                    assert!(!out.ended && !session.awaiting_scoped_keyword());
                }
                _ => {}
            }
            let out = session.submit_keyword(leave, &ctx);
            let label = format!("{leave:?} at {route}");
            assert!(out.ended, "{label}");
            assert_eq!(out.text.matches("Farewell.").count(), 1, "{label}: {:?}", out.text);
            assert_eq!(
                out.text.lines().any(|line| line == bye_line),
                leave.is_empty(),
                "{label}: {:?}",
                out.text
            );
        }
    }
}

/// The inline `T<keyword>` Talk route asks the conversation session's own
/// matcher (`conversation.md §6` steps 3 to 5): a keyword that lives inside
/// a labelled block is not a top-level keyword there either, and a rebuke
/// word answers the rebuke.
#[test]
fn rc4_the_inline_talk_keyword_route_uses_the_session_matcher() {
    let raw_fields = rc4_scoped_blob();
    let decoded = rc4_decoded(&raw_fields);
    let dialogue = HashMap::from([(2u16, decoded)]);
    let raw = HashMap::from([(2u16, raw_fields)]);
    let talk = |keyword: &str| {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.load_scheduled_npcs(&[
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            NpcSlot {
                slot: 1,
                type_byte: 1,
                dialog_id: 2,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: Some("Ada".to_string()),
            },
        ]);
        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, Some(keyword)),
            MoveOutcome::Talked
        );
        state.message.clone()
    };
    assert_eq!(talk("y").trim_end(), TLK_NO_KEYWORD_MATCH_MESSAGE.trim_end());
    assert_eq!(talk("rejoin").trim_end(), TLK_NO_KEYWORD_MATCH_MESSAGE.trim_end());
    assert_eq!(talk("ass").trim_end(), TLK_RESERVED_REBUKE_MESSAGE.trim_end());
    assert_eq!(talk("jobs"), "I mend gear.");
    assert!(talk("i want to join").starts_with("Well?"));
    assert_eq!(resolve_keyword_response_field_index(&rc4_decoded(&rc4_scoped_blob()), "ass"), None);
}

/// u5-spec#299 Q5, `rest-and-camp.md §7.1` step 4: "Each living member gets
/// one separate flash that lasts for the long envelope" - the inversion
/// rides each living member's long sting, after the short one, and a dead
/// member (who "skips steps 4 and 5") gets none.
#[test]
fn rc4_the_camp_apparition_flashes_once_per_living_member() {
    let mut state = britannia_state(open_world_grid(), 2, 0);
    let member = |slot: u8, status: u8, hp: u16| PartyMember {
        slot,
        class_byte: b'F',
        status,
        climb_stat: 20,
        mana: 0,
        hp,
        max_hp: 30,
        level: 1,
    };
    state.party = vec![member(0, b'G', 30), member(1, b'D', 0), member(2, b'P', 4)];
    state.party_experience = vec![0, 0, 0];
    let (_, sounds) = state.resolve_lord_british_camp_event_paged(None).unwrap();
    let effects: Vec<SoundEffect> = sounds.into_iter().map(|(_, effect)| effect).collect();
    let flashes: Vec<usize> = effects
        .iter()
        .enumerate()
        .filter(|(_, effect)| effect.holds_viewport_inversion())
        .map(|(index, _)| index)
        .collect();
    assert_eq!(flashes.len(), 2, "two living members, two flashes: {effects:?}");
    let short = SoundEffect::LordBritishCampEnvelope(
        crate::lord_british_camp::LORD_BRITISH_CAMP_STING_SHORT,
    );
    for index in flashes {
        assert_eq!(effects[index - 1], short, "the short sting comes first");
    }
    // Only the long sting carries the flash.
    assert!(!short.holds_viewport_inversion());
    assert!(
        !SoundEffect::LordBritishCampEnvelope(
            crate::lord_british_camp::LORD_BRITISH_CAMP_APPARITION_ENVELOPE
        )
        .holds_viewport_inversion()
    );
    assert!(!state.presentation_camp_flash_inverted, "the frontend owns the flash");
}

/// The rows a frontend lays out for `state`, the way the Bevy shell and
/// `--save-screen` assemble them.
fn rc4_message_layout(state: &PlayState) -> crate::MessageWindowLayout {
    let keep = |text: &str| (!text.trim().is_empty()).then(|| text.to_string());
    let mut log = crate::message_log_from_entries(state.displayed_message_entries(), keep);
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    if let Some(text) = state.displayed_unflushed_message().and_then(keep) {
        log.push_output(&text);
    }
    let live_row = (!state.message_window_live_row_suppressed()).then_some("");
    let kind = if crate::blackthorn_exit_key_wait_on_feed_row(state) {
        crate::LiveRowKind::KeyWaitOnFeedRow
    } else {
        crate::LiveRowKind::CommandRow
    };
    crate::layout_message_window_with_continuation(&log, live_row, None, false, kind)
}

/// `blackthorn.md §4.2` / §5 "Rows on the correct-answer arm" (u5-spec#299
/// Q9): after record `5` and step 9's line feed, the exit beat's key wait
/// draws the animated key-wait cursor "at column 0, two rows below the
/// record's last row", with one blank row between and no end cap. During
/// the blade hold before it there is "no key-wait cursor".
#[test]
fn rc4_the_record_five_exit_beat_key_wait_draws_its_cursor_two_rows_down() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.repaint_stats_panel();
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    assert!(state.staged_narration_active(), "the blade hold");
    assert!(!crate::blackthorn_exit_key_wait_on_feed_row(&state));
    assert!(state.message_window_cursor_suppressed(), "no cursor during the blade hold");

    state.flush_staged_narration();
    assert!(crate::blackthorn_exit_key_wait_on_feed_row(&state));
    assert!(!state.message_window_cursor_suppressed());
    let layout = rc4_message_layout(&state);
    let text = |row: &crate::MessageWindowRow| -> String {
        row.glyphs.iter().map(|glyph| glyph.byte as char).collect()
    };
    let record_row = layout
        .rows
        .iter()
        .rev()
        .find(|row| text(row).contains("death!"))
        .expect("record 5's last row")
        .row;
    let cursor = layout.rows.last().expect("the key-wait row");
    assert_eq!(cursor.row, record_row + 2, "{:?}", layout.rows.iter().map(text).collect::<Vec<_>>());
    assert!(cursor.glyphs.is_empty());
    assert_eq!(cursor.column, crate::MESSAGE_WINDOW_LEFT, "column 0 of the window");
    assert!(!cursor.prefixed, "a key wait, not a command row: no end cap");

    // The key starts the beat, which draws no cursor and no row.
    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    assert!(state.pending_blackthorn_audience_exit);
    assert!(state.message_window_cursor_suppressed());
    assert!(state.message_window_live_row_suppressed());
    let _ = fs::remove_dir_all(dir);
}

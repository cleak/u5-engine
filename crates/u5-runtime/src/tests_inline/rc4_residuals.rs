// rc4 residuals of u5-spec#299 (spec commits b42d672..a3ce96b).

fn rc4_enc(text: &str) -> Vec<u8> {
    text.bytes().map(|b| b ^ 0x80).collect()
}

fn rc4_decoded(raw: &[Vec<u8>]) -> Vec<String> {
    raw.iter()
        .map(|field| field.iter().map(|b| char::from(b ^ 0x80)).collect())
        .collect()
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

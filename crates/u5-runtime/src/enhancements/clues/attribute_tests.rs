//! Turn attribution (design 8.6: T4 matcher agreement, T5 attribution;
//! build plan M6-02). Every blob is synthetic ([`super::super::fixtures`]):
//! invented words in the `formats/tlk.md` layout. The conversations run
//! through the real `PlayState` input path, and the observer's side is
//! the transcript diff alone.

use std::collections::HashMap;
use std::path::Path;

use super::super::fixtures::{blob, block, decoded, record, text};
use super::*;
use crate::conversation_session::{TlkKeywordLookup, top_level_probe};
use crate::enhancements::{TranscriptCursor, TranscriptDiff};
use crate::test_fixtures::{open_grid, test_state};
use crate::tlk_control_codes::RESERVED_KEYWORD_REBUKE_WORDS;
use crate::{Direction, NpcSlot, handle_play_key_input};

const DIALOG_ID: u8 = 0x10;
const SLOT: usize = 1;

/// Pairs with prefix overlaps, a held reply, a gold demand, an ASK-WHO,
/// a GOTO into a block with a scoped prompt, a runic span; then the block.
fn fields() -> Vec<Vec<u8>> {
    blob(
        &[
            ("gran", text("Old one.")),
            ("grandpa", text("Never reached.")),
            ("mill", record(&[&text("Wheels "), &[0x83], &text("turn.")])),
            (
                "toll",
                record(&[&text("Pay "), &[0x85], &text("900"), &text(" now.")]),
            ),
            ("who", record(&[&text("Thy name? "), &[0x88]])),
            ("tree", record(&[&text("Climb? "), &[0x91]])),
            (
                "rune",
                record(&[&text("See "), &[0x8E], &text("abc"), &[0x8E]]),
            ),
        ],
        &[block(
            0x91,
            &text("Yes or no?"),
            &[
                text("y"),
                text("Up thou goest."),
                text("n"),
                text("Stay then."),
            ],
        )],
    )
}

/// One conversation as the shell's observer would see it: the state, the
/// cursor kept from the last step, and the attributor.
struct Talk {
    state: PlayState,
    fields: Vec<Vec<u8>>,
    decoded: Vec<String>,
    cursor: TranscriptCursor,
    attributor: TurnAttributor,
    misses: bool,
}

impl Talk {
    fn open() -> Self {
        Self::open_with(fields())
    }

    fn open_with(fields: Vec<Vec<u8>>) -> Self {
        let decoded = decoded(&fields);
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::East;
        state.gold = 10;
        state.load_scheduled_npcs(&[
            NpcSlot {
                slot: 0,
                type_byte: 0,
                dialog_id: 0,
                schedule: [0; 16],
                name: None,
            },
            NpcSlot {
                slot: SLOT,
                type_byte: 1,
                dialog_id: DIALOG_ID,
                schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
                name: None,
            },
        ]);
        let mut talk = Self {
            state,
            fields,
            decoded,
            cursor: TranscriptCursor::default(),
            attributor: TurnAttributor::new(),
            misses: false,
        };
        talk.reopen();
        talk
    }

    /// Opens the conversation again, as Talk does; the opening is not a
    /// turn.
    fn reopen(&mut self) {
        let dialogue = HashMap::from([(u16::from(DIALOG_ID), self.decoded.clone())]);
        let raw = HashMap::from([(u16::from(DIALOG_ID), self.fields.clone())]);
        assert!(
            self.state
                .open_conversation_session(&dialogue, &raw)
                .is_some()
        );
        self.state.flush_message_slot();
        self.attributor = TurnAttributor::new();
        let turns = self.step(None);
        assert!(turns.is_empty(), "an opening is not a turn: {turns:?}");
    }

    fn type_line(&mut self, line: &str) {
        let mut chars = line.chars();
        let key = chars.next().unwrap_or('\r');
        let rest: String = chars.collect();
        handle_play_key_input(&mut self.state, key, &rest, Path::new("")).unwrap();
    }

    /// One frame: `lines` submitted, then the observer's post step.
    fn frame(&mut self, lines: &[&str]) -> Vec<Turn> {
        let phase = self.state.active_conversation.as_ref().map(|s| s.phase);
        for line in lines {
            self.type_line(line);
        }
        self.state.flush_message_slot();
        let mut turns = self.step(phase);
        if self.state.active_conversation.is_none() {
            turns.extend(self.attributor.finish());
        }
        turns
    }

    /// The observer's post step over the diff since the last step.
    fn step(&mut self, phase: Option<ConversationSessionPhase>) -> Vec<Turn> {
        let ctx = TurnContext {
            fields: &self.fields,
            decoded: &self.decoded,
            phase,
            misses: self.misses,
        };
        let now = self.state.message_entries();
        let turns = match self.cursor.diff_state(&self.state) {
            TranscriptDiff::Unchanged => self.attributor.step(
                &ctx,
                now,
                now.len(),
                false,
                self.state.pending_text_hold.is_some(),
            ),
            TranscriptDiff::Anchored {
                first_new,
                replaced_last,
            } => self.attributor.step(
                &ctx,
                now,
                first_new,
                replaced_last,
                self.state.pending_text_hold.is_some(),
            ),
            TranscriptDiff::Unanchored(_) => {
                self.attributor.unanchored();
                Vec::new()
            }
        };
        self.cursor = TranscriptCursor::of(&self.state);
        turns
    }

    /// A frame with no key: releases a pending hold the way a timed pause
    /// does outside the observer's window, then steps.
    fn idle(&mut self) -> Vec<Turn> {
        if self.state.pending_text_hold.is_some() {
            self.state.flush_text_hold();
            self.state.flush_message_slot();
        }
        let phase = self.state.active_conversation.as_ref().map(|s| s.phase);
        self.step(phase)
    }

    /// `line` asked and answered: its frame and the idle frames until the
    /// turn closes.
    fn ask(&mut self, line: &str) -> Vec<Turn> {
        let mut turns = self.frame(&[line]);
        for _ in 0..3 {
            turns.extend(self.idle());
        }
        turns
    }

    fn records(turns: &[Turn]) -> Vec<RecordAddr> {
        turns.iter().map(|turn| turn.record).collect()
    }
}

fn pair(k: u8) -> RecordAddr {
    RecordAddr::Pair(k)
}

// ---------------------------------------------------------------------------
// T4: the observer's record equals the field the session ran.

fn rebuke_word() -> String {
    String::from_utf8(RESERVED_KEYWORD_REBUKE_WORDS[0].to_vec()).unwrap()
}

#[test]
fn t4_observer_and_session_pick_the_same_record() {
    let rebuke = rebuke_word();
    let lines = [
        "gran",
        "GRAN",
        "grandpa",
        "gra",
        " gran",
        "   mill",
        "xgran",
        "x gran",
        "mill gran",
        "job",
        "jobs",
        "work",
        "name",
        rebuke.as_str(),
        "toll",
        "nothing here",
        "abcdefghijklmno",
        "abcdefghijklmnogran",
        "abcdefghijk gran",
        "abcdefghijklm gran",
        "tree",
        "rune",
    ];
    for line in lines {
        let mut talk = Talk::open();
        top_level_probe::take();
        let turns = talk.ask(line);
        let ran = top_level_probe::take().expect("the session ran its top-level scan");
        // The probe and the observer's lookup on the echo agree ...
        let echoed = line
            .chars()
            .take(crate::tlk_control_codes::TLK_INPUT_MAX_LEN)
            .collect::<String>();
        assert_eq!(
            tlk_keyword_lookup(Some(&talk.fields), &talk.decoded, echoed.trim()),
            ran,
            "{line:?}"
        );
        // ... and the turn logged is the record the session ran.
        let logged = turns.first().map(|turn| turn.record);
        assert_eq!(logged, record_of(ran, false), "{line:?}: {turns:?}");
    }
}

#[test]
fn t4_gold_refusal_reenters_the_top_level_scan() {
    let mut talk = Talk::open();
    let turns = talk.ask("toll");
    assert_eq!(Talk::records(&turns), [pair(3)]);
    assert_eq!(
        talk.state.active_conversation.as_ref().map(|s| s.phase),
        Some(ConversationSessionPhase::AwaitingGoldRefusalKeyword)
    );
    top_level_probe::take();
    let turns = talk.ask("gran");
    assert_eq!(top_level_probe::take(), Some(TlkKeywordLookup::Field(6)));
    assert_eq!(Talk::records(&turns), [pair(0)]);
}

// ---------------------------------------------------------------------------
// T5: attribution.

#[test]
fn t5_echo_rows_name_the_record() {
    let mut talk = Talk::open();
    assert_eq!(Talk::records(&talk.ask("job")), [RecordAddr::Leading(3)]);
    assert_eq!(Talk::records(&talk.ask("gran")), [pair(0)]);
    // `grandpa` is reached through `gran`'s prefix: the first keyword wins.
    assert_eq!(Talk::records(&talk.ask("grandpa")), [pair(0)]);
    assert_eq!(Talk::records(&talk.ask("rune")), [pair(6)]);
}

#[test]
fn t5_name_gives_only_the_name_heard_mark() {
    let mut talk = Talk::open();
    let turns = talk.ask("name");
    assert_eq!(Talk::records(&turns), [RecordAddr::Leading(0)]);
    assert!(!turns[0].record.is_topic());
}

#[test]
fn t5_bye_rebuke_miss_and_empty_are_not_logged() {
    let mut talk = Talk::open();
    assert!(talk.ask(&rebuke_word()).is_empty(), "rebuke");
    assert!(
        talk.ask("nothing here").is_empty(),
        "miss without clue_misses"
    );
    assert!(talk.ask("bye").is_empty(), "bye");
    talk.reopen();
    assert!(talk.ask("").is_empty(), "empty line");
    assert!(talk.state.active_conversation.is_none());
}

#[test]
fn t5_a_miss_keeps_its_typed_bytes_only_with_clue_misses() {
    let mut talk = Talk::open();
    talk.misses = true;
    let turns = talk.ask("nothing here");
    assert_eq!(Talk::records(&turns), [RecordAddr::Miss]);
    assert_eq!(turns[0].typed, b"NOTHING HERE");
    let turns = talk.ask("gran");
    assert!(turns[0].typed.is_empty(), "only a miss carries bytes");
}

#[test]
fn t5_variant_tells_replies_apart_and_repeats_agree() {
    let mut talk = Talk::open();
    let first = talk.ask("gran");
    let again = talk.ask("gran");
    let job = talk.ask("job");
    assert_eq!(first[0].variant, again[0].variant);
    assert_ne!(first[0].variant, job[0].variant);
}

#[test]
fn t5_runic_flag_follows_the_glyph_font() {
    let mut talk = Talk::open();
    assert!(talk.ask("rune")[0].runic);
    assert!(!talk.ask("gran")[0].runic);
}

#[test]
fn t5_held_pages_stay_in_one_turn() {
    let mut held = Talk::open();
    let turns = held.frame(&["mill"]);
    assert!(turns.is_empty(), "the turn stays open while the hold waits");
    assert!(held.state.pending_text_hold.is_some());
    assert!(held.attributor.is_open());
    // The rest of the reply is released outside the pre/post window (a
    // timed pause in `animate_static_tiles`) and still belongs to it.
    let mut turns = held.idle();
    turns.extend(held.idle());
    assert_eq!(Talk::records(&turns), [pair(2)]);
    // The variant covers the whole reply: a second hearing agrees.
    let again = held.ask("mill");
    assert_eq!(turns[0].variant, again[0].variant);
}

#[test]
fn t5_scoped_turns_attach_under_their_opener() {
    let mut talk = Talk::open();
    assert_eq!(Talk::records(&talk.ask("tree")), [pair(5)]);
    assert!(matches!(
        talk.state.active_conversation.as_ref().map(|s| s.phase),
        Some(ConversationSessionPhase::AwaitingScopedKeyword { .. })
    ));
    let turns = talk.ask("y");
    let scoped = RecordAddr::Scoped {
        decl: 0,
        arm: RecordAddr::ARM_ANSWERED,
    };
    assert_eq!(Talk::records(&turns), [scoped]);
    assert_eq!(turns[0].parent, Some(pair(5)));
}

#[test]
fn t5_ask_who_answers_are_not_logged() {
    let mut talk = Talk::open();
    assert_eq!(Talk::records(&talk.ask("who")), [pair(4)]);
    assert!(matches!(
        talk.state.active_conversation.as_ref().map(|s| s.phase),
        Some(ConversationSessionPhase::AwaitingAskWho { .. })
    ));
    assert!(talk.ask("gran").is_empty(), "a party name is not a clue");
    assert_eq!(Talk::records(&talk.ask("gran")), [pair(0)]);
}

#[test]
fn t5_several_submits_in_one_frame() {
    let mut talk = Talk::open();
    let mut turns = talk.frame(&["gran", "job", "mill"]);
    for _ in 0..3 {
        turns.extend(talk.idle());
    }
    assert_eq!(
        Talk::records(&turns),
        [pair(0), RecordAddr::Leading(3), pair(2)]
    );
    // After a GOTO the second line answers the scoped prompt.
    let mut turns = talk.frame(&["tree", "y"]);
    turns.extend(talk.idle());
    assert_eq!(Talk::records(&turns)[0], pair(5));
    assert_eq!(turns.len(), 1, "the block is not known from rows: dropped");
    assert_eq!(talk.attributor.dropped(), 1);
}

#[test]
fn t5_the_last_turn_is_kept_when_the_conversation_ends() {
    let mut talk = Talk::open();
    let turns = talk.frame(&["gran", ""]);
    assert!(talk.state.active_conversation.is_none());
    assert_eq!(Talk::records(&turns), [pair(0)]);
    // The slot is cleared on the end: the snapshot is gone with it.
    assert_eq!(ConversationSnapshot::of(&talk.state), None);
}

#[test]
fn t5_an_unanchored_diff_drops_the_open_turn() {
    let mut talk = Talk::open();
    assert!(talk.frame(&["mill"]).is_empty());
    talk.state.clear_message_window();
    talk.state.flush_text_hold();
    talk.state.flush_message_slot();
    let turns = talk.idle();
    assert!(turns.is_empty(), "{turns:?}");
    assert_eq!(talk.attributor.dropped(), 1);
    assert!(!talk.attributor.is_open());
}

#[test]
fn t5_snapshot_reads_who_and_the_test_inputs() {
    let mut talk = Talk::open();
    let snap = ConversationSnapshot::of(&talk.state).unwrap();
    // `test_state` is scene 0x11, the castle family.
    assert_eq!(
        snap.npc,
        NpcKey {
            scene: 0x11,
            slot: 1
        }
    );
    assert_eq!(
        snap.dialog,
        DialogKey {
            family: 2,
            dialog_id: DIALOG_ID
        }
    );
    assert_eq!(snap.inputs.gold, 10);
    assert_eq!(snap.phase, ConversationSessionPhase::AwaitingKeyword);
    // Opened by contact: the same snapshot, the runtime set the slot.
    talk.state.talk_entered_by_contact = true;
    assert_eq!(ConversationSnapshot::of(&talk.state).unwrap().npc, snap.npc);
}

#[test]
fn t5_inline_talk_uses_the_sessions_matcher() {
    let fields = fields();
    let decoded = decoded(&fields);
    assert_eq!(
        inline_turn_record(&fields, &decoded, "gran", false),
        Some(pair(0))
    );
    assert_eq!(
        inline_turn_record(&fields, &decoded, "job", false),
        Some(RecordAddr::Leading(3))
    );
    assert_eq!(inline_turn_record(&fields, &decoded, "bye", false), None);
}

#[test]
fn turn_entry_carries_tests_and_flags() {
    let fields = fields();
    let turn = Turn {
        record: pair(4),
        parent: None,
        variant: 7,
        runic: true,
        typed: Vec::new(),
        rows: Vec::new(),
    };
    let npc = NpcKey { scene: 3, slot: 1 };
    let dialog = DialogKey {
        family: 0,
        dialog_id: 2,
    };
    let inputs = TestInputs::default();
    let entry = turn.entry(npc, dialog, &fields, &inputs, &inputs, GameMinute(5));
    assert!(entry.flags.has(EntryFlags::RUNIC));
    assert!(entry.flags.has(EntryFlags::REPLY), "0x88 asks back");
    assert_eq!(
        (entry.variant, entry.count, entry.first),
        (7, 1, GameMinute(5))
    );
}

// ---------------------------------------------------------------------------
// M6-04: the rows a turn carries, the static render and the scrollback.

use super::super::scrollback::{MAX_SCROLLBACK_ENTRIES, Scrollback};
use super::super::text::{SessionText, TextBody, static_render, text_body};
use crate::enhancements::journal::ClueText;

fn row_text(row: &TextRow) -> String {
    row.bytes.iter().map(|b| char::from(*b)).collect()
}

#[test]
fn m6_04_a_turn_carries_the_rows_the_window_drew() {
    let mut talk = Talk::open();
    let turns = talk.ask("gran");
    assert_eq!(turns.len(), 1);
    let rows: Vec<String> = turns[0].rows.iter().map(row_text).collect();
    assert!(rows.iter().any(|row| row.contains("Old one.")), "{rows:?}");
    assert!(turns[0].rows.iter().all(|row| row.runic == 0));
    assert!(turns[0].rows.iter().all(|row| row.bytes.len() <= 16));
    let runic = talk.ask("rune");
    assert!(runic[0].runic);
    assert!(
        runic[0].rows.iter().any(|row| row.runic != 0),
        "the runic span keeps its mask: {:?}",
        runic[0].rows
    );
}

#[test]
fn m6_04_static_render_matches_the_rows_seen_for_a_plain_record() {
    let mut talk = Talk::open();
    let turns = talk.ask("gran");
    let seen: Vec<String> = turns[0].rows.iter().map(row_text).collect();
    let fields = talk.fields.clone();
    let render = static_render(&fields, pair(0), "Cal").expect("no branch codes");
    let render: Vec<String> = render.iter().map(row_text).collect();
    assert_eq!(render, seen);
}

#[test]
fn m6_04_static_render_only_for_a_record_with_no_branch_codes() {
    let fields = fields();
    // gran: plain text; rune: a runic span, still static.
    assert!(static_render(&fields, pair(0), "Cal").is_some());
    let rune = static_render(&fields, pair(6), "Cal").expect("a runic span is no branch");
    assert!(rune.iter().any(|row| row.runic != 0));
    // toll (0x85 gold), who (0x88 ASK-WHO), tree (a GOTO): never guessed.
    for k in [3, 4, 5] {
        assert_eq!(static_render(&fields, pair(k), "Cal"), None, "pair {k}");
    }
    // A name-known branch (0x8C) and a standing branch (0xFE).
    let branchy = blob(
        &[
            (
                "knew",
                record(&[&text("Ah. "), &[0x8C, 0x91], &text("Hm.")]),
            ),
            (
                "deed",
                record(&[&text("So. "), &[0xFE, 0x10, 0x91], &text("Hm.")]),
            ),
            ("give", record(&[&text("Take it."), &[0x86, b'A' | 0x80]])),
        ],
        &[block(0x91, &text("Yes?"), &[text("y"), text("Good.")])],
    );
    for k in 0..3 {
        assert_eq!(static_render(&branchy, pair(k), "Cal"), None, "pair {k}");
    }
    // Scoped answers and misses have no field of their own.
    let scoped = RecordAddr::Scoped {
        decl: 0,
        arm: RecordAddr::ARM_ANSWERED,
    };
    assert_eq!(static_render(&fields, scoped, "Cal"), None);
    assert_eq!(static_render(&fields, RecordAddr::Miss, "Cal"), None);
}

#[test]
fn m6_04_text_body_prefers_the_session_then_the_render_then_replay() {
    let fields = fields();
    let npc = NpcKey { scene: 3, slot: 1 };
    let dialog = DialogKey {
        family: 0,
        dialog_id: DIALOG_ID,
    };
    let inputs = TestInputs::default();
    let plain = ClueEntry::heard(npc, dialog, pair(0), 1, TestBits::default(), GameMinute(1));
    let branch = ClueEntry::heard(npc, dialog, pair(5), 2, TestBits::default(), GameMinute(1));
    let _ = inputs;
    let mut session = SessionText::new();
    let seen = vec![TextRow {
        bytes: b"what I saw".to_vec(),
        runic: 0,
    }];
    // Off: nothing, whatever is in RAM.
    session.remember(&branch, &seen);
    assert_eq!(
        text_body(
            ClueText::Off,
            &branch,
            session.rows(&branch),
            None,
            Some(&fields),
            "Cal"
        ),
        TextBody::Off
    );
    // Live, seen this session: the rows from RAM, branches included.
    assert_eq!(
        text_body(
            ClueText::Live,
            &branch,
            session.rows(&branch),
            None,
            Some(&fields),
            "Cal"
        ),
        TextBody::Rows(seen.clone())
    );
    // Live after a relaunch: a branch record is the replay line ...
    let relaunched = SessionText::new();
    assert_eq!(
        text_body(
            ClueText::Live,
            &branch,
            relaunched.rows(&branch),
            None,
            Some(&fields),
            "Cal"
        ),
        TextBody::Replay
    );
    // ... a plain one renders statically, and with no blob it is replay.
    assert!(matches!(
        text_body(ClueText::Live, &plain, None, None, Some(&fields), "Cal"),
        TextBody::Rows(_)
    ));
    assert_eq!(
        text_body(ClueText::Live, &plain, None, None, None, "Cal"),
        TextBody::Replay
    );
    // Kept rows count only under kept.
    assert_eq!(
        text_body(
            ClueText::Live,
            &branch,
            None,
            Some(&seen),
            Some(&fields),
            "Cal"
        ),
        TextBody::Replay
    );
    assert_eq!(
        text_body(
            ClueText::Kept,
            &branch,
            None,
            Some(&seen),
            Some(&fields),
            "Cal"
        ),
        TextBody::Rows(seen)
    );
}

#[test]
fn m6_04_session_text_keys_by_variant_and_is_bounded() {
    let npc = NpcKey { scene: 3, slot: 1 };
    let dialog = DialogKey {
        family: 0,
        dialog_id: DIALOG_ID,
    };
    let a = ClueEntry::heard(npc, dialog, pair(0), 1, TestBits::default(), GameMinute(1));
    let b = ClueEntry { variant: 2, ..a };
    let row = |s: &str| TextRow {
        bytes: s.as_bytes().to_vec(),
        runic: 0,
    };
    let mut session = SessionText::new();
    session.remember(&a, &[row("one")]);
    session.remember(&b, &[row("two")]);
    session.remember(&a, &[]);
    assert_eq!(
        session.rows(&a),
        Some(&[row("one")][..]),
        "no rows keep the old"
    );
    assert_eq!(session.rows(&b), Some(&[row("two")][..]));
    for n in 0..(super::super::text::MAX_SESSION_TEXTS as u16 + 5) {
        session.remember(
            &ClueEntry {
                variant: 100 + n,
                ..a
            },
            &[row("x")],
        );
    }
    assert_eq!(session.len(), super::super::text::MAX_SESSION_TEXTS);
}

/// A Talk harness with a scrollback stepped after every frame.
fn scroll_frame(talk: &mut Talk, scroll: &mut Scrollback, lines: &[&str]) {
    let _ = talk.frame(lines);
    scroll.step(&talk.state);
    for _ in 0..3 {
        let _ = talk.idle();
        scroll.step(&talk.state);
    }
}

#[test]
fn m6_04_scrollback_starts_at_the_talk_echo_and_clears_at_the_end() {
    let mut talk = Talk::open();
    let mut scroll = Scrollback::new();
    // End the conversation the test harness opened, then see one open.
    scroll_frame(&mut talk, &mut scroll, &["bye"]);
    scroll.step(&talk.state);
    assert!(talk.state.active_conversation.is_none());
    assert_eq!(scroll.rows(), None, "no rows outside a conversation");
    talk.state.push_message_entry("Talk-East", true);
    talk.state.flush_message_slot();
    talk.reopen();
    scroll.step(&talk.state);
    let rows = scroll.rows().expect("a conversation is open");
    assert_eq!(row_text(&rows[0]), "Talk-East", "{rows:?}");
    scroll_frame(&mut talk, &mut scroll, &["gran"]);
    let rows: Vec<String> = scroll.rows().unwrap().iter().map(row_text).collect();
    assert!(rows.iter().any(|row| row.contains("Old one.")), "{rows:?}");
    scroll_frame(&mut talk, &mut scroll, &["bye"]);
    assert_eq!(scroll.rows(), None, "cleared when the conversation ends");
}

#[test]
fn m6_04_scrollback_loses_nothing_to_the_transcript_cap() {
    // Distinct topics: the shared diff anchors on the last rows, and rows
    // seen twice in the window cannot be told apart (it drops them then).
    let words: Vec<String> = (0..30u8)
        .map(|n| {
            format!(
                "w{}{}",
                char::from(b'a' + n / 10),
                char::from(b'a' + n % 10)
            )
        })
        .collect();
    let replies: Vec<(String, Vec<u8>)> = words
        .iter()
        .map(|word| (word.clone(), text(&format!("Reply {word}."))))
        .collect();
    let pairs: Vec<(&str, Vec<u8>)> = replies
        .iter()
        .map(|(word, reply)| (word.as_str(), reply.clone()))
        .collect();
    let mut talk = Talk::open_with(blob(&pairs, &[]));
    let mut scroll = Scrollback::new();
    scroll.step(&talk.state);
    for word in &words {
        scroll_frame(&mut talk, &mut scroll, &[word]);
    }
    assert!(talk.state.active_conversation.is_some());
    assert_eq!(scroll.dropped(), 0);
    let rows: Vec<String> = scroll.rows().unwrap().iter().map(row_text).collect();
    for word in &words {
        let reply = format!("Reply {word}.");
        assert_eq!(
            rows.iter().filter(|row| row.contains(&reply)).count(),
            1,
            "{word} kept once past the cap"
        );
    }
    assert_eq!(
        talk.state.message_entries().len(),
        64,
        "the transcript itself is at its cap"
    );
    assert!(rows.len() > 64, "more rows than the transcript holds");
    assert!(rows.len() <= MAX_SCROLLBACK_ENTRIES * 4);
    assert_eq!(scroll.dropped(), 0);
}

#[test]
fn m6_04_rows_are_the_window_rows_of_a_long_reply() {
    let long = "One two three four five six seven eight.";
    let mut talk = Talk::open_with(blob(&[("long", text(long))], &[]));
    let mut scroll = Scrollback::new();
    scroll.step(&talk.state);
    let turns = talk.ask("long");
    scroll_frame(&mut talk, &mut scroll, &[]);
    let rows = &turns[0].rows;
    assert!(rows.len() >= 3, "{rows:?}");
    assert!(rows.iter().all(|row| row.bytes.len() <= 16));
    let joined = rows.iter().map(row_text).collect::<Vec<_>>().join(" ");
    for word in long.split(' ') {
        assert!(
            joined.contains(word.trim_end_matches('.')),
            "{word}: {joined}"
        );
    }
    // The static render lays the record out the same way.
    let render = static_render(&talk.fields, pair(0), "Cal").unwrap();
    assert_eq!(&render, rows);
    // The scrollback keeps the same window rows.
    let kept = scroll.rows().unwrap();
    assert!(kept.iter().all(|row| row.bytes.len() <= 16));
    assert!(kept.windows(rows.len()).any(|w| w == rows.as_slice()));
}

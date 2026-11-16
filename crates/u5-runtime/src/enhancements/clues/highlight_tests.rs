//! T10 highlight spans and T11 modes (design 8.6) on synthetic rows and
//! blobs: invented words only, no game text.

use super::*;
use crate::enhancements::clues::fixtures::{blob, block, decoded, record, text};
use crate::enhancements::clues::{DialogKey, GameMinute, NpcKey, TestBits, record_dependence};

const NPC: NpcKey = NpcKey { scene: 3, slot: 4 };
const DIALOG: DialogKey = DialogKey {
    family: 0,
    dialog_id: 9,
};
const BASE: TestInputs = TestInputs {
    acquainted: false,
    standing: 20,
    gold: 100,
    party_size: 1,
};

/// Pairs: 0 `glad` (fixed), 1 `abby` (tests the acquaintance bit),
/// 2 `torq` (fixed), then a labelled block whose arm is `yew`.
fn fields() -> Vec<Vec<u8>> {
    let tested = record(&[&text("Hmm. "), &[0x8C, 0x91], &text("Who?")]);
    let head = record(&[&text("Friend!"), &[0x82]]);
    blob(
        &[
            ("glad", text("The glade is green.")),
            ("abby", tested),
            ("torq", text("A torque of brass.")),
        ],
        &[block(0x92, &text("Well?"), &[text("yew"), text("Aye.")])],
    )
    .into_iter()
    .chain(block(0x91, &head, &[]))
    .collect()
}

fn glyphs(s: &str) -> Vec<TlkRenderedGlyph> {
    s.bytes().map(TlkRenderedGlyph::ordinary).collect()
}

fn runic(s: &str) -> Vec<TlkRenderedGlyph> {
    s.bytes().map(TlkRenderedGlyph::runic).collect()
}

fn rows(lines: &[Vec<TlkRenderedGlyph>]) -> Vec<HighlightRow<'_>> {
    lines
        .iter()
        .enumerate()
        .map(|(i, glyphs)| HighlightRow {
            row: 11 + i as u8,
            column: 24,
            glyphs,
            eligible: true,
        })
        .collect()
}

fn heard(record: RecordAddr, pre: &TestInputs, post: &TestInputs, f: &[Vec<u8>]) -> ClueEntry {
    let dep = record_dependence(f, record);
    ClueEntry::heard(
        NPC,
        DIALOG,
        record,
        0,
        TestBits::capture(&dep, pre, post),
        GameMinute(10),
    )
}

struct Fixture {
    fields: Vec<Vec<u8>>,
    decoded: Vec<String>,
}

impl Fixture {
    fn new() -> Self {
        let fields = fields();
        let decoded = decoded(&fields);
        Self { fields, decoded }
    }

    fn session(&self, phase: ConversationSessionPhase, now: TestInputs) -> HighlightSession<'_> {
        HighlightSession {
            fields: &self.fields,
            decoded: &self.decoded,
            phase,
            now,
        }
    }

    fn spans(
        &self,
        lines: &[Vec<TlkRenderedGlyph>],
        asked: &Asked<'_>,
        mode: ClueHighlight,
    ) -> Vec<Span> {
        let session = self.session(ConversationSessionPhase::AwaitingKeyword, BASE);
        highlight_spans(&rows(lines), Some(&session), asked, mode)
    }
}

fn span(row: u8, column: u8, len: u8, mark: Mark) -> Span {
    Span {
        row,
        column,
        len,
        mark,
    }
}

// ---------------------------------------------------------------------------
// T10 spans.

#[test]
fn t10_an_asked_fixed_topic_is_marked_same_without_its_punctuation() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    let spans = fx.spans(&[glyphs("\"Past the GLADE,")], &asked, ClueHighlight::Asked);
    assert_eq!(spans, [span(11, 34, 5, Mark::Same)]);
}

#[test]
fn t10_the_prefix_rule_is_the_games_word_start_rule() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    // `gladly` starts with the keyword; `unglad` holds it mid-word.
    let spans = fx.spans(&[glyphs("gladly unglad")], &asked, ClueHighlight::Asked);
    assert_eq!(spans, [span(11, 24, 6, Mark::Same)]);
}

#[test]
fn t10_a_word_broken_across_a_full_row_is_joined() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(2), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    // Sixteen cells, the last four the start of the word.
    let lines = [glyphs("aaaaaaaaaaa torq"), glyphs("uemaker here")];
    let spans = fx.spans(&lines, &asked, ClueHighlight::Asked);
    assert_eq!(
        spans,
        [span(11, 36, 4, Mark::Same), span(12, 24, 7, Mark::Same)]
    );
    // A row that is not full ends its last word.
    let lines = [glyphs("torq"), glyphs("uemaker")];
    let spans = fx.spans(&lines, &asked, ClueHighlight::Asked);
    assert_eq!(spans, [span(11, 24, 4, Mark::Same)]);
}

#[test]
fn t10_a_runic_run_is_one_word() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(2), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    let mut line = glyphs("see ");
    line.extend(runic("TORQ OF OLD"));
    line.extend(glyphs(" ok"));
    let spans = fx.spans(&[line], &asked, ClueHighlight::Asked);
    assert_eq!(spans, [span(11, 28, 11, Mark::Same)]);
}

#[test]
fn t10_reserved_rebuke_and_scoped_words_are_never_marked() {
    let fx = Fixture::new();
    let here = [
        heard(RecordAddr::Leading(0), &BASE, &BASE, &fx.fields),
        heard(RecordAddr::Leading(3), &BASE, &BASE, &fx.fields),
    ];
    let elsewhere = [b"NAME".to_vec(), b"JOB".to_vec(), b"YEW".to_vec()];
    let asked = Asked {
        here: &here,
        elsewhere: &elsewhere,
    };
    // NAME, JOB, BYE and the scoped arm `yew` (only after the block's
    // declaration, so not a top-level topic). `yew` is heard elsewhere.
    let lines = [glyphs("name job bye"), glyphs("yew")];
    let spans = fx.spans(&lines, &asked, ClueHighlight::New);
    assert_eq!(spans, [span(12, 24, 3, Mark::Elsewhere)]);
    // A word of the engine's rebuke table is never marked either, even
    // when it equals a topic asked elsewhere.
    let word: String = crate::RESERVED_KEYWORD_REBUKE_WORDS[0]
        .iter()
        .map(|b| char::from(b & 0x7F))
        .collect();
    assert_eq!(
        crate::conversation_session::tlk_keyword_lookup(Some(&fx.fields), &fx.decoded, &word),
        TlkKeywordLookup::Rebuke
    );
    let elsewhere = [word.to_ascii_uppercase().into_bytes()];
    let asked = Asked {
        here: &here,
        elsewhere: &elsewhere,
    };
    assert!(
        fx.spans(&[glyphs(&word)], &asked, ClueHighlight::New)
            .is_empty()
    );
}

#[test]
fn t10_a_tested_answer_whose_input_changed_may_differ() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(1), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    let lines = [glyphs("the abbyl")];
    let same = fx.spans(&lines, &asked, ClueHighlight::Asked);
    assert_eq!(same, [span(11, 28, 5, Mark::Same)]);
    let known = TestInputs {
        acquainted: true,
        ..BASE
    };
    let session = fx.session(ConversationSessionPhase::AwaitingKeyword, known);
    let differ = highlight_spans(&rows(&lines), Some(&session), &asked, ClueHighlight::Asked);
    assert_eq!(differ, [span(11, 28, 5, Mark::MayDiffer)]);
}

#[test]
fn t10_suppressed_in_scoped_ask_who_and_gold_phases_and_without_a_session() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    let lines = [glyphs("glad")];
    for phase in [
        ConversationSessionPhase::AwaitingScopedKeyword { label: 0x92 },
        ConversationSessionPhase::AwaitingAskWho {
            field_idx: 6,
            cursor: 0,
        },
        ConversationSessionPhase::AwaitingGoldRefusalKeyword,
    ] {
        let session = fx.session(phase, BASE);
        assert!(
            highlight_spans(&rows(&lines), Some(&session), &asked, ClueHighlight::New).is_empty()
        );
    }
    assert!(highlight_spans(&rows(&lines), None, &asked, ClueHighlight::New).is_empty());
    // Rows from before the session are not eligible.
    let mut old = rows(&lines);
    old[0].eligible = false;
    let session = fx.session(ConversationSessionPhase::AwaitingKeyword, BASE);
    assert!(highlight_spans(&old, Some(&session), &asked, ClueHighlight::New).is_empty());
}

#[test]
fn t10_off_marks_nothing() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[b"GLAD".to_vec()],
    };
    assert!(
        fx.spans(&[glyphs("glad torq")], &asked, ClueHighlight::Off)
            .is_empty()
    );
}

// ---------------------------------------------------------------------------
// T11 modes.

#[test]
fn t11_asked_marks_only_this_persons_asked_records() {
    let fx = Fixture::new();
    // `torq` was asked of someone else only: not marked in `asked`.
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    let elsewhere = [b"TORQ".to_vec()];
    let asked = Asked {
        here: &here,
        elsewhere: &elsewhere,
    };
    let spans = fx.spans(&[glyphs("glad torq")], &asked, ClueHighlight::Asked);
    assert_eq!(spans, [span(11, 24, 4, Mark::Same)]);
}

#[test]
fn t11_asked_elsewhere_adds_the_sparse_mark_under_any_equal_word() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    // `torq` is this person's topic too; `mill` is not.
    let elsewhere = [b"TORQ".to_vec(), b"MILL".to_vec()];
    let asked = Asked {
        here: &here,
        elsewhere: &elsewhere,
    };
    let spans = fx.spans(
        &[glyphs("glad torq mills")],
        &asked,
        ClueHighlight::AskedElsewhere,
    );
    assert_eq!(
        spans,
        [
            span(11, 24, 4, Mark::Same),
            span(11, 29, 4, Mark::Elsewhere),
            span(11, 34, 5, Mark::Elsewhere),
        ]
    );
}

#[test]
fn t11_new_tints_only_unasked_candidates() {
    let fx = Fixture::new();
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    let spans = fx.spans(&[glyphs("glad torq stone")], &asked, ClueHighlight::New);
    assert_eq!(
        spans,
        [span(11, 24, 4, Mark::Same), span(11, 29, 4, Mark::New)]
    );
    // Without `new` the unasked topic stays plain.
    let spans = fx.spans(
        &[glyphs("glad torq stone")],
        &asked,
        ClueHighlight::AskedElsewhere,
    );
    assert_eq!(spans, [span(11, 24, 4, Mark::Same)]);
}

#[test]
fn t11_a_session_only_set_marks_what_this_conversation_asked() {
    let fx = Fixture::new();
    // Session-only mode hands the conversation's own entries and nothing
    // asked elsewhere: an empty set marks nothing in `asked`.
    let none = Asked::default();
    assert!(
        fx.spans(&[glyphs("glad")], &none, ClueHighlight::AskedElsewhere)
            .is_empty()
    );
    let here = [heard(RecordAddr::Pair(0), &BASE, &BASE, &fx.fields)];
    let asked = Asked {
        here: &here,
        elsewhere: &[],
    };
    assert_eq!(
        fx.spans(&[glyphs("glad")], &asked, ClueHighlight::Asked),
        [span(11, 24, 4, Mark::Same)]
    );
}

#[test]
fn topic_keywords_are_upper_case_and_only_for_pairs() {
    let fx = Fixture::new();
    assert_eq!(
        topic_keyword(&fx.fields, RecordAddr::Pair(1)),
        Some(b"ABBY".to_vec())
    );
    assert_eq!(topic_keyword(&fx.fields, RecordAddr::Leading(3)), None);
    assert_eq!(topic_keyword(&fx.fields, RecordAddr::Pair(40)), None);
}

#[test]
fn prompt_rows_and_echoes_are_the_players() {
    assert!(is_prompt_or_echo_row(":GLAD"));
    assert!(is_prompt_or_echo_row(":"));
    for prompt in [TLK_KEYWORD_PROMPT, TLK_SCOPED_PROMPT, TLK_ASK_WHO_PROMPT] {
        let head = prompt.split('\n').next().unwrap();
        assert!(is_prompt_or_echo_row(head));
    }
    assert!(!is_prompt_or_echo_row("The glade is green."));
}

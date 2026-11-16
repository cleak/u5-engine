//! The clue model, codec, reconcile and dependence scan (design 8.6: T7,
//! T8, T9 reconcile classes, T9b encode, T16; build plan M6-01). Every
//! blob is synthetic ([`super::fixtures`]).

use super::fixtures::{blob, block, decoded, record, text};
use super::*;
use crate::conversation_session::{TlkKeywordLookup, tlk_keyword_lookup};
use crate::enhancements::sidecar::{
    ContainerError, Freshness, SaveStamp, SidecarPayload, TAG_JOURNAL, decode_container,
    encode_container,
};

const NPC: NpcKey = NpcKey { scene: 3, slot: 7 };
const DIALOG: DialogKey = DialogKey {
    family: 1,
    dialog_id: 12,
};

fn inputs(acquainted: bool, standing: u8, gold: u16, party_size: u8) -> TestInputs {
    TestInputs {
        acquainted,
        standing,
        gold,
        party_size,
    }
}

const BASE: TestInputs = TestInputs {
    acquainted: false,
    standing: 40,
    gold: 10,
    party_size: 3,
};

/// One hearing of `record` with reach `dep`, before `pre` and after
/// `post`.
fn heard(record: RecordAddr, dep: &Dependence, pre: &TestInputs, post: &TestInputs) -> ClueEntry {
    ClueEntry::heard(
        NPC,
        DIALOG,
        record,
        0x1234,
        TestBits::capture(dep, pre, post),
        GameMinute(100),
    )
}

fn entry(n: u32) -> ClueEntry {
    ClueEntry::heard(
        NpcKey {
            scene: (n % 200) as u8,
            slot: (n / 200) as u8,
        },
        DIALOG,
        RecordAddr::Pair((n % 250) as u8),
        n as u16,
        TestBits::default(),
        GameMinute(n),
    )
}

fn stamp() -> SaveStamp {
    SaveStamp {
        fnv: 0x0123_4567_89ab_cdef,
        len: 4192,
        avatar: *b"Wanderer\0",
        date: [0x8B, 0x00, 4, 12, 9, 30],
    }
}

// --- Model. ---------------------------------------------------------------

#[test]
fn record_of_maps_the_lookup_to_a_record() {
    assert_eq!(
        record_of(TlkKeywordLookup::Field(0), false),
        Some(RecordAddr::Leading(0))
    );
    assert_eq!(
        record_of(TlkKeywordLookup::Field(3), false),
        Some(RecordAddr::Leading(3))
    );
    assert_eq!(
        record_of(TlkKeywordLookup::Field(6), false),
        Some(RecordAddr::Pair(0))
    );
    assert_eq!(
        record_of(TlkKeywordLookup::Field(12), false),
        Some(RecordAddr::Pair(3))
    );
    for dropped in [
        TlkKeywordLookup::Field(4),
        TlkKeywordLookup::Field(1),
        TlkKeywordLookup::Field(7),
        TlkKeywordLookup::Rebuke,
        TlkKeywordLookup::EmptyLine,
        TlkKeywordLookup::NoMatch,
    ] {
        assert_eq!(record_of(dropped, false), None, "{dropped:?}");
    }
    assert_eq!(
        record_of(TlkKeywordLookup::NoMatch, true),
        Some(RecordAddr::Miss)
    );
    assert!(!RecordAddr::Leading(0).is_topic());
    assert!(RecordAddr::Leading(3).is_topic());
    assert_eq!(RecordAddr::Pair(2).field(), Some(10));
}

#[test]
fn record_of_agrees_with_the_lookup_on_a_synthetic_blob() {
    let fields = blob(
        &[
            ("GLIMMER", text("It shines.")),
            ("GLIM", text("A shorter word.")),
        ],
        &[],
    );
    let decoded = decoded(&fields);
    let lookup = |line| record_of(tlk_keyword_lookup(Some(&fields), &decoded, line), false);
    assert_eq!(lookup("GLIMMER"), Some(RecordAddr::Pair(0)));
    assert_eq!(lookup("NAME"), Some(RecordAddr::Leading(0)));
    assert_eq!(lookup("JOB"), Some(RecordAddr::Leading(3)));
    assert_eq!(lookup("BYE"), None);
    assert_eq!(lookup("ZZZZ"), None);
}

#[test]
fn game_minutes_follow_the_calendar() {
    let at = GameMinute::from_calendar;
    assert_eq!(at(0, 1, 1, 0, 0), GameMinute(0));
    assert_eq!(at(0, 1, 1, 1, 5), GameMinute(65));
    assert_eq!(at(0, 1, 2, 0, 0), GameMinute(1440));
    assert_eq!(at(0, 2, 1, 0, 0), GameMinute(28 * 1440));
    assert_eq!(at(1, 1, 1, 0, 0), GameMinute(13 * 28 * 1440));
    // Monotone across a month, a year and the end of a day.
    let order = [
        at(139, 4, 28, 23, 59),
        at(139, 5, 1, 0, 0),
        at(139, 13, 28, 23, 59),
        at(140, 1, 1, 0, 0),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(at(u16::MAX, 13, 28, 23, 59), GameMinute(u32::MAX));
}

#[test]
fn the_variant_hash_tells_rows_apart() {
    let a = variant_hash([b"one".as_slice(), b"two".as_slice()]);
    let b = variant_hash([b"one".as_slice(), b"twO".as_slice()]);
    let c = variant_hash([b"onetwo".as_slice()]);
    assert_ne!(a, b);
    assert_ne!(a, c, "row breaks count");
    assert_eq!(a, variant_hash([b"one".as_slice(), b"two".as_slice()]));
}

// --- T7 folding. ---------------------------------------------------------

#[test]
fn t7_identical_repeats_fold_with_the_right_count() {
    let mut log = ClueLog::new();
    let first = heard(RecordAddr::Pair(0), &Dependence::Unknown, &BASE, &BASE);
    assert_eq!(log.fold(first), FoldOutcome::Added(0));
    for n in 2..=5u16 {
        let mut again = first;
        again.first = GameMinute(100 + u32::from(n));
        again.last = again.first;
        assert_eq!(log.fold(again), FoldOutcome::Folded(0));
        assert_eq!(log.entries()[0].count, n);
    }
    assert_eq!(log.entries().len(), 1);
    assert_eq!(log.entries()[0].first, GameMinute(100));
    assert_eq!(log.entries()[0].last, GameMinute(105));
    assert_eq!(log.unsaved(), 5);
    assert!(log.is_dirty());
    log.mark_saved();
    assert_eq!(log.unsaved(), 0);
    assert!(!log.is_dirty());

    // Another variant, person or record is its own entry.
    let mut other = first;
    other.variant ^= 1;
    assert_eq!(log.fold(other), FoldOutcome::Added(1));
    let mut other = first;
    other.npc.slot += 1;
    assert_eq!(log.fold(other), FoldOutcome::Added(2));
}

#[test]
fn t7_count_saturates() {
    let mut log = ClueLog::new();
    let mut first = entry(1);
    first.count = u16::MAX - 1;
    log.fold(first);
    log.fold(entry(1));
    log.fold(entry(1));
    assert_eq!(log.entries()[0].count, u16::MAX);
}

#[test]
fn t7_a_branch_heard_before_and_after_ask_who_is_two_entries() {
    // The introduce-yourself shape: ASK-WHO, then a test of the bit that
    // ASK-WHO sets, branching to a block.
    let greeting = record(&[&text("Who art thou? "), &[0x88, 0x8C, 0x91]]);
    let fields = blob(
        &[("TRUMBLE", greeting)],
        &[block(0x91, &record(&[&text("Well met."), &[0x82]]), &[])],
    );
    let dep = record_dependence(&fields, RecordAddr::Pair(0));
    assert_eq!(dep.tests(), [Test::Acquainted]);
    assert!(dep.reply(), "0x88 asks something back");

    let before = BASE;
    let after = TestInputs {
        acquainted: true,
        ..BASE
    };
    let mut log = ClueLog::new();
    log.fold(heard(RecordAddr::Pair(0), &dep, &before, &after));
    log.fold(heard(RecordAddr::Pair(0), &dep, &after, &after));
    let entries = log.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!(
        (entries[0].tests.pre, entries[0].tests.post),
        (0, 1),
        "pre != post: the turn wrote the bit it tests"
    );
    assert_eq!((entries[1].tests.pre, entries[1].tests.post), (1, 1));
    assert_ne!(entries[0].tests, entries[1].tests);
}

// --- Eviction (8.2.9). ---------------------------------------------------

fn full_log() -> ClueLog {
    let mut log = ClueLog::new();
    for n in 0..MAX_ENTRIES as u32 {
        assert!(matches!(log.fold(entry(n + 10)), FoldOutcome::Added(_)));
    }
    log
}

#[test]
fn t8_the_oldest_unprotected_entry_is_evicted_at_the_cap() {
    let mut log = full_log();
    log.mark_saved();
    // Protect the three oldest: starred, pinned to the panel, linked; the
    // fourth is noted.
    log.set_flag(0, EntryFlags::STAR, true);
    log.set_flag(1, EntryFlags::PANEL, true);
    log.add_link(Link {
        entry: 2,
        kind: 0,
        target: 9,
    });
    log.add_note(Note {
        owner_kind: note_owner::CLUE,
        owner: 3,
        text: "mine".into(),
    });
    log.add_link(Link {
        entry: 4000,
        kind: 1,
        target: 2,
    });
    let victim = log.entries()[4];
    let newest = entry(999_999);
    assert_eq!(log.fold(newest), FoldOutcome::Evicted(MAX_ENTRIES - 1));
    assert_eq!(log.entries().len(), MAX_ENTRIES);
    assert!(!log.entries().contains(&victim));
    assert_eq!(log.entries()[MAX_ENTRIES - 1], newest);
    // Indices past the victim moved down by one; protected ones stayed.
    assert_eq!(log.links()[0].entry, 2);
    assert_eq!(log.links()[1].entry, 3999);
    assert_eq!(log.notes()[0].owner, 3);
    for index in 0..4 {
        assert!(log.is_protected(index), "{index}");
    }
    // The toast is due once.
    assert!(log.take_eviction_toast());
    assert!(!log.take_eviction_toast());
    log.fold(entry(1_000_000));
    assert!(!log.take_eviction_toast(), "one-time toast");
}

#[test]
fn t8_a_log_full_of_protected_entries_keeps_nothing_new() {
    let mut log = full_log();
    for index in 0..MAX_ENTRIES {
        log.set_flag(index, EntryFlags::STAR, true);
    }
    let before = log.clone();
    assert_eq!(log.fold(entry(5_000_000)), FoldOutcome::Full);
    assert_eq!(log.entries(), before.entries());
    // A repeat still folds.
    assert_eq!(log.fold(entry(10)), FoldOutcome::Folded(0));
}

// --- T8 codec. -----------------------------------------------------------

fn sample_log() -> ClueLog {
    let mut log = ClueLog::new();
    let records = [
        RecordAddr::Leading(0),
        RecordAddr::Leading(3),
        RecordAddr::Pair(17),
        RecordAddr::Scoped {
            decl: 2,
            arm: RecordAddr::ARM_ANSWERED,
        },
        RecordAddr::Miss,
    ];
    for (i, record) in records.into_iter().enumerate() {
        let mut capture = ClueEntry::heard(
            NpcKey {
                scene: i as u8,
                slot: 30,
            },
            DIALOG,
            record,
            0xBEEF ^ i as u16,
            TestBits {
                n: if i == 2 { TestBits::UNKNOWN } else { i as u8 },
                pre: 0b101,
                post: 0b111,
            },
            GameMinute::from_calendar(139, 4, 12, 9, i as u8),
        );
        capture.last = GameMinute(capture.first.0 + 77);
        capture.flags = EntryFlags(0b0101_0011);
        log.fold(capture);
    }
    log.add_link(Link {
        entry: 1,
        kind: 0,
        target: 4,
    });
    log.add_link(Link {
        entry: 2,
        kind: 1,
        target: 513,
    });
    log.add_note(Note {
        owner_kind: note_owner::CLUE,
        owner: 2,
        text: "ask the smith twice".into(),
    });
    log.add_note(Note {
        owner_kind: note_owner::SAVE,
        owner: 0,
        text: String::new(),
    });
    log.add_miss(Miss {
        npc: NPC,
        at: GameMinute(9),
        typed: b"QWERTYUIOPASDFGHJ".to_vec(),
    });
    log.mark_known(7);
    log.mark_known(300);
    log.keep_text(SeenRows {
        entry: 1,
        rows: vec![
            TextRow {
                bytes: b"0123456789abcdef".to_vec(),
                runic: 0x00F0,
            },
            TextRow {
                bytes: Vec::new(),
                runic: 0,
            },
        ],
    });
    log.passthrough.push((4, vec![1, 2, 3, 4, 5, 0, 0xAA]));
    log.passthrough.push((42, vec![9; 11]));
    log
}

const KEEP_ALL: EncodeOptions = EncodeOptions {
    keep_text: true,
    keep_misses: true,
};

/// The log as a fresh read gives it back: clean.
fn clean(mut log: ClueLog) -> ClueLog {
    log.mark_saved();
    log
}

#[test]
fn t8_round_trip() {
    let log = sample_log();
    let bytes = encode_payload(&log, KEEP_ALL).unwrap();
    let back = decode_payload(&bytes).unwrap();
    assert_eq!(back, clean(log.clone()));
    assert_eq!(back.misses()[0].typed.len(), MISS_MAX_LEN, "cut at 15");
    // Encoding is a fixed point.
    assert_eq!(encode_payload(&back, KEEP_ALL).unwrap(), bytes);
    // The empty log is one empty CLUES section.
    let empty = encode_payload(&ClueLog::new(), EncodeOptions::default()).unwrap();
    assert_eq!(empty, [1, 2, 0, 0, 0, 0, 0]);
    assert_eq!(decode_payload(&empty).unwrap(), ClueLog::new());
}

#[test]
fn t8_a_clues_entry_is_24_bytes() {
    assert_eq!(CLUE_ENTRY_LEN, 24);
    let mut log = ClueLog::new();
    let bytes = encode_payload(&log, EncodeOptions::default()).unwrap();
    for n in 1..=3 {
        log.fold(entry(n));
        let grown = encode_payload(&log, EncodeOptions::default()).unwrap();
        assert_eq!(grown.len(), bytes.len() + 24 * n as usize);
        let len = u32::from_le_bytes([grown[1], grown[2], grown[3], grown[4]]);
        assert_eq!(len as usize, 2 + 24 * n as usize);
    }
}

#[test]
fn t8_truncation_at_every_offset_is_corrupt() {
    let payload = encode_payload(&sample_log(), KEEP_ALL).unwrap();
    let file = encode_container(TAG_JOURNAL, &stamp(), JOURNAL_PAYLOAD_VERSION, &payload).unwrap();
    let container = decode_container(&file).unwrap();
    assert_eq!(container.payload, payload);
    for cut in 0..file.len() {
        assert!(decode_container(&file[..cut]).is_err(), "cut at {cut}");
    }
    // Inside the payload, a cut within a section never decodes.
    let mut boundaries = vec![0];
    let mut at = 0;
    while at < payload.len() {
        let len = u32::from_le_bytes(payload[at + 1..at + 5].try_into().unwrap()) as usize;
        at += 5 + len;
        boundaries.push(at);
    }
    for cut in 0..payload.len() {
        let result = decode_payload(&payload[..cut]);
        assert_eq!(result.is_ok(), boundaries.contains(&cut), "cut at {cut}");
    }
}

#[test]
fn t8_bad_magic_and_checksum_are_corrupt() {
    let payload = encode_payload(&sample_log(), KEEP_ALL).unwrap();
    let file = encode_container(TAG_JOURNAL, &stamp(), JOURNAL_PAYLOAD_VERSION, &payload).unwrap();
    let mut bad = file.clone();
    bad[0] ^= 0x20;
    assert_eq!(decode_container(&bad), Err(ContainerError::BadMagic));
    let mut bad = file.clone();
    let mid = bad.len() / 2;
    bad[mid] ^= 1;
    assert_eq!(decode_container(&bad), Err(ContainerError::BadChecksum));
}

#[test]
fn t8_malformed_payloads_do_not_decode() {
    let good = encode_payload(&sample_log(), KEEP_ALL).unwrap();
    // A trailing byte inside a section.
    let bad = [6, 5, 0, 0, 0, 1, 0, 7, 0, 9];
    assert_eq!(decode_payload(&bad), Err(CodecError::TrailingBytes(6)));
    // A known section twice.
    let twice = [&[6u8, 2, 0, 0, 0, 0, 0][..], &[6, 2, 0, 0, 0, 0, 0]].concat();
    assert_eq!(decode_payload(&twice), Err(CodecError::DuplicateSection(6)));
    // A bad record kind.
    let mut log = ClueLog::new();
    log.fold(entry(1));
    let mut bytes = encode_payload(&log, EncodeOptions::default()).unwrap();
    bytes[7 + 4] = 9;
    assert_eq!(decode_payload(&bytes), Err(CodecError::BadValue(1)));
    // A link to an entry that does not exist.
    let dangling = [
        &[1u8, 2, 0, 0, 0, 0, 0][..],
        &[2, 7, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0],
    ]
    .concat();
    assert_eq!(decode_payload(&dangling), Err(CodecError::BadReference(2)));
    // A note that is not UTF-8.
    let note = [&[3u8, 8, 0, 0, 0, 1, 0, 2, 0, 0, 1, 0][..], &[0xFF]].concat();
    assert_eq!(decode_payload(&note), Err(CodecError::BadValue(3)));
    assert!(decode_payload(&good).is_ok());
}

#[test]
fn t8_unknown_sections_and_pins_are_kept_unchanged() {
    let log = sample_log();
    let bytes = encode_payload(&log, KEEP_ALL).unwrap();
    let back = decode_payload(&bytes).unwrap();
    assert_eq!(
        back.passthrough,
        [(4, vec![1, 2, 3, 4, 5, 0, 0xAA]), (42, vec![9; 11])]
    );
    // An unknown section ahead of CLUES is skipped by length.
    let first = [&[200u8, 3, 0, 0, 0, 1, 2, 3][..], &[1, 2, 0, 0, 0, 0, 0]].concat();
    let decoded = decode_payload(&first).unwrap();
    assert!(decoded.entries().is_empty());
    let again = encode_payload(&decoded, EncodeOptions::default()).unwrap();
    assert!(again.ends_with(&[200, 3, 0, 0, 0, 1, 2, 3]));
}

#[test]
fn t8_caps() {
    // Entries: 4096 decode, 4097 do not.
    let mut log = full_log();
    let bytes = encode_payload(&log, EncodeOptions::default()).unwrap();
    assert_eq!(bytes.len(), 7 + 24 * MAX_ENTRIES);
    assert_eq!(decode_payload(&bytes).unwrap().entries().len(), MAX_ENTRIES);
    let mut over = bytes.clone();
    let len = 2 + 24 * (MAX_ENTRIES + 1);
    over[1..5].copy_from_slice(&(len as u32).to_le_bytes());
    over[5..7].copy_from_slice(&((MAX_ENTRIES + 1) as u16).to_le_bytes());
    over.extend_from_slice(&[0; 24]);
    assert_eq!(decode_payload(&over), Err(CodecError::TooManyEntries));
    log.entries.push(entry(1));
    assert_eq!(
        encode_payload(&log, EncodeOptions::default()),
        Err(CodecError::TooManyEntries)
    );
    // The payload: 1 MiB.
    let mut huge = ClueLog::new();
    for _ in 0..20 {
        huge.add_note(Note {
            owner_kind: note_owner::SAVE,
            owner: 0,
            text: "x".repeat(60_000),
        });
    }
    assert_eq!(
        encode_payload(&huge, EncodeOptions::default()),
        Err(CodecError::TooLarge)
    );
    assert_eq!(
        decode_payload(&vec![0; MAX_PAYLOAD_LEN + 1]),
        Err(CodecError::TooLarge)
    );
}

#[test]
fn t8_text_and_misses_are_written_only_when_kept() {
    let log = sample_log();
    let plain = decode_payload(&encode_payload(&log, EncodeOptions::default()).unwrap()).unwrap();
    assert!(plain.text().is_empty());
    assert!(plain.misses().is_empty());
    assert!(
        plain
            .entries()
            .iter()
            .all(|e| !e.flags.has(EntryFlags::HAS_TEXT)),
        "no entry claims text that was not written"
    );
    assert_eq!(plain.notes(), log.notes(), "notes are always kept");
    let mut dropped = log.clone();
    dropped.drop_text();
    assert!(dropped.text().is_empty());
}

// --- T9b clean-room guard. -----------------------------------------------

#[test]
fn t9b_the_encoded_log_holds_no_name_keyword_or_text() {
    const MARKERS: &[&str] = &["XQMARKNAME", "XQMARKWORD", "XQMARKREPLY", "XQMARKJOB"];
    let mut fields = blob(&[("XQMARKWORD", text("XQMARKREPLY in full."))], &[]);
    fields[0] = text("XQMARKNAME");
    fields[3] = text("XQMARKJOB is my trade.");
    let decoded = decoded(&fields);
    let mut log = ClueLog::new();
    for (i, line) in ["NAME", "JOB", "XQMARKWORD", "xqmarkword please"]
        .into_iter()
        .enumerate()
    {
        let lookup = tlk_keyword_lookup(Some(&fields), &decoded, line);
        let record = record_of(lookup, false).expect("logged");
        let dep = record_dependence(&fields, record);
        let rows = [fields[record.field().unwrap()].as_slice()];
        log.fold(ClueEntry::heard(
            NPC,
            DIALOG,
            record,
            variant_hash(rows),
            TestBits::capture(&dep, &BASE, &BASE),
            GameMinute(i as u32),
        ));
    }
    assert_eq!(log.entries().len(), 3, "the last line folds");
    for options in [
        EncodeOptions::default(),
        EncodeOptions {
            keep_text: false,
            keep_misses: true,
        },
    ] {
        let bytes = encode_payload(&log, options).unwrap();
        for marker in MARKERS {
            for form in [marker.as_bytes().to_vec(), text(marker)] {
                assert!(
                    !bytes.windows(form.len()).any(|w| w == form.as_slice()),
                    "{marker} leaked"
                );
            }
        }
    }
}

// --- T9 reconcile classes. -----------------------------------------------

fn payload(log: &ClueLog, version: u16) -> SidecarPayload {
    SidecarPayload {
        version,
        bytes: encode_payload(log, KEEP_ALL).unwrap(),
    }
}

#[test]
fn t9_reconcile_by_freshness() {
    let log = sample_log();
    let file = payload(&log, JOURNAL_PAYLOAD_VERSION);

    let fresh = reconcile(Freshness::Fresh, Some(&file));
    assert_eq!(fresh.log, clean(log.clone()));
    assert_eq!(fresh.note, None);
    assert!(!fresh.read_only && !fresh.rename_bad);
    assert_eq!(fresh.log.unsaved(), 0);

    let gap = reconcile(Freshness::SameLineage, Some(&file));
    assert_eq!(gap.log.entries(), log.entries(), "every entry is kept");
    assert_eq!(gap.note, Some(ReconcileNote::GapNotLogged));

    let foreign = reconcile(Freshness::Foreign, Some(&file));
    assert!(foreign.log.is_empty());
    assert_eq!(foreign.note, Some(ReconcileNote::Foreign));
    assert!(!foreign.read_only, "replaced at the next save");

    let missing = reconcile(Freshness::Missing, None);
    assert!(missing.log.is_empty());
    assert_eq!(missing.note, None);

    let corrupt = reconcile(Freshness::Corrupt, None);
    assert!(corrupt.log.is_empty());
    assert_eq!(corrupt.note, Some(ReconcileNote::Corrupt));

    // A stamp that matches but a payload that does not decode.
    let broken = SidecarPayload {
        version: 1,
        bytes: vec![1, 9, 0, 0, 0],
    };
    let broken = reconcile(Freshness::Fresh, Some(&broken));
    assert!(broken.log.is_empty() && broken.rename_bad);
    assert_eq!(broken.note, Some(ReconcileNote::Corrupt));

    for note in [
        ReconcileNote::GapNotLogged,
        ReconcileNote::Foreign,
        ReconcileNote::Corrupt,
        ReconcileNote::Newer,
    ] {
        assert!(!note.text().is_empty());
    }
}

#[test]
fn t9_a_newer_payload_is_read_only() {
    let log = sample_log();
    let newer = reconcile(Freshness::Fresh, Some(&payload(&log, 2)));
    assert!(newer.read_only);
    assert!(!newer.rename_bad, "never renamed");
    assert_eq!(newer.log.entries(), log.entries());
    let unreadable = SidecarPayload {
        version: 2,
        bytes: vec![1, 9, 0, 0, 0],
    };
    let unreadable = reconcile(Freshness::Fresh, Some(&unreadable));
    assert!(unreadable.read_only && !unreadable.rename_bad);
    assert_eq!(unreadable.note, Some(ReconcileNote::Newer));
}

// --- T16 answers that change. --------------------------------------------

fn pair_dep(response: Vec<u8>, blocks: &[Vec<Vec<u8>>]) -> (Vec<Vec<u8>>, Dependence) {
    let fields = blob(&[("SNORBLE", response)], blocks);
    let dep = record_dependence(&fields, RecordAddr::Pair(0));
    (fields, dep)
}

fn state(dep: &Dependence, heard_with: &[TestInputs], now: &TestInputs) -> AskedState {
    let entries: Vec<ClueEntry> = heard_with
        .iter()
        .map(|inputs| heard(RecordAddr::Pair(0), dep, inputs, inputs))
        .collect();
    asked_state(&entries, dep, now)
}

#[test]
fn t16_a_fixed_record_is_same() {
    let (_, dep) = pair_dep(text("The lantern is blue."), &[]);
    assert!(dep.is_fixed() && dep.renders_statically());
    assert_eq!(state(&dep, &[BASE], &BASE), AskedState::Same);
    let anything = inputs(true, 99, 9999, 6);
    assert_eq!(state(&dep, &[BASE], &anything), AskedState::Same);
}

#[test]
fn t16_the_acquaintance_bit_before_and_after() {
    let response = record(&[&text("Hmm. "), &[0x8C, 0x91], &text("Who?")]);
    let head = record(&[&text("Friend!"), &[0x82]]);
    let (_, dep) = pair_dep(response, &[block(0x91, &head, &[])]);
    assert_eq!(dep.tests(), [Test::Acquainted]);
    assert!(!dep.reply() && !dep.renders_statically());
    let known = TestInputs {
        acquainted: true,
        ..BASE
    };
    assert_eq!(state(&dep, &[BASE], &BASE), AskedState::Same);
    assert_eq!(state(&dep, &[BASE], &known), AskedState::MayDiffer);
    assert_eq!(state(&dep, &[known], &known), AskedState::Same);
}

#[test]
fn t16_standing_on_each_side_of_the_threshold() {
    let response = record(&[&[0xFE, 50, 0x91], &text("Begone.")]);
    let head = record(&[&text("Welcome."), &[0x82]]);
    let (_, dep) = pair_dep(response, &[block(0x91, &head, &[])]);
    assert_eq!(dep.tests(), [Test::Standing(50)]);
    let at = |standing| TestInputs { standing, ..BASE };
    assert_eq!(state(&dep, &[at(40)], &at(49)), AskedState::Same);
    assert_eq!(state(&dep, &[at(40)], &at(50)), AskedState::MayDiffer);
    assert_eq!(state(&dep, &[at(50)], &at(99)), AskedState::Same);
}

#[test]
fn t16_gold_affordable_and_not() {
    let response = record(&[&text("Pay "), &[0x85], &text("050"), &text(" coins.")]);
    let (_, dep) = pair_dep(response, &[]);
    assert_eq!(dep.tests(), [Test::Gold(50)]);
    let with = |gold| TestInputs { gold, ..BASE };
    assert_eq!(state(&dep, &[with(10)], &with(49)), AskedState::Same);
    assert_eq!(state(&dep, &[with(10)], &with(50)), AskedState::MayDiffer);
    // A non-digit amount is malformed.
    let (_, bad) = pair_dep(record(&[&[0x85], &text("0x5")]), &[]);
    assert_eq!(bad, Dependence::Unknown);
}

#[test]
fn t16_party_room_with_room_and_full() {
    let (_, dep) = pair_dep(record(&[&text("I will come."), &[0x84]]), &[]);
    assert_eq!(dep.tests(), [Test::PartyRoom]);
    let size = |party_size| TestInputs { party_size, ..BASE };
    assert_eq!(state(&dep, &[size(6)], &size(6)), AskedState::Same);
    assert_eq!(state(&dep, &[size(6)], &size(5)), AskedState::MayDiffer);
}

#[test]
fn t16_a_follow_up_block_and_ask_who_set_reply() {
    let head = text("Dost thou agree?");
    let follow_up = block(
        0x91,
        &head,
        &[
            text("So be it."),
            text("YES"),
            record(&[&text("Good."), &[0x89]]),
            text("NO"),
            text("Pity."),
        ],
    );
    let (_, dep) = pair_dep(record(&[&text("A question. "), &[0x91]]), &[follow_up]);
    assert!(dep.reply());
    assert!(dep.tests().is_empty());
    assert_eq!(state(&dep, &[BASE], &BASE), AskedState::MayDiffer);

    let (_, ask) = pair_dep(record(&[&text("Thy name? "), &[0x88]]), &[]);
    assert!(ask.reply());
    assert_eq!(state(&ask, &[BASE], &BASE), AskedState::MayDiffer);
}

#[test]
fn t16_a_head_that_stops_opens_no_prompt() {
    let head = record(&[&text("Enough."), &[0x82]]);
    let stopped = block(0x91, &head, &[record(&[&[0xFE, 10, 0x91]])]);
    let (_, dep) = pair_dep(vec![0x91], &[stopped]);
    assert!(!dep.reply());
    assert!(dep.tests().is_empty(), "the arms are out of reach");
}

#[test]
fn t16_an_alias_chain_is_followed() {
    let fields = blob(
        &[
            ("ALPHA", vec![0x87]),
            ("BETA", vec![0x87]),
            ("GAMMA", record(&[&[0xFE, 30, 0x91], &text("Hm.")])),
        ],
        &[block(0x91, &record(&[&text("Yes."), &[0x82]]), &[])],
    );
    for k in 0..3 {
        let dep = record_dependence(&fields, RecordAddr::Pair(k));
        assert_eq!(dep.tests(), [Test::Standing(30)], "pair {k}");
    }
    // An alias with nothing two records ahead is malformed.
    let short = blob(&[("OMEGA", vec![0x87])], &[]);
    assert_eq!(
        record_dependence(&short, RecordAddr::Pair(0)),
        Dependence::Unknown
    );
}

#[test]
fn t16_a_goto_into_a_block_carrying_a_standing_test_is_followed() {
    let first = block(
        0x91,
        &record(&[&[0xFE, 20, 0x92], &text("Low."), &[0x82]]),
        &[],
    );
    let second = block(0x92, &record(&[&text("High."), &[0x82]]), &[]);
    let (fields, dep) = pair_dep(record(&[&text("Let me see. "), &[0x91]]), &[first, second]);
    assert_eq!(dep.tests(), [Test::Standing(20)]);
    assert!(!dep.reply());
    assert_eq!(block_count(&fields), 2);
    assert_eq!(declaration_index(&fields, 0x92), Some(1));
    assert_eq!(declaration_index(&fields, 0x95), None);
}

#[test]
fn t16_a_self_goto_ends_the_walk() {
    let looping = block(0x91, &record(&[&text("Again "), &[0x84, 0x91]]), &[]);
    let (_, dep) = pair_dep(vec![0x91], &[looping]);
    assert_eq!(dep.tests(), [Test::PartyRoom]);
}

#[test]
fn t16_nine_tests_or_a_missing_label_are_unknown() {
    let (_, eight) = pair_dep(vec![0x84; 8], &[]);
    assert_eq!(eight.tests().len(), 8);
    let (_, nine) = pair_dep(vec![0x84; 9], &[]);
    assert_eq!(nine, Dependence::Unknown);
    assert_eq!(state(&nine, &[BASE], &BASE), AskedState::MayDiffer);
    let (_, missing) = pair_dep(record(&[&text("Go "), &[0x93]]), &[]);
    assert_eq!(missing, Dependence::Unknown);
    let (_, truncated) = pair_dep(vec![0xFE, 10], &[]);
    assert_eq!(truncated, Dependence::Unknown);
    let fields = blob(&[], &[]);
    assert_eq!(
        record_dependence(&fields, RecordAddr::Pair(4)),
        Dependence::Unknown,
        "no such record"
    );
    assert_eq!(
        record_dependence(&fields, RecordAddr::Miss),
        Dependence::Unknown
    );
}

#[test]
fn t16_end_of_response_stops_the_walk() {
    let (_, dep) = pair_dep(record(&[&text("Done."), &[0xFF, 0x84, 0x88]]), &[]);
    assert!(dep.is_fixed());
    // `0x8C 0xFF` opens a prompt: the test counts, no block is followed.
    let (_, prompt) = pair_dep(record(&[&[0x8C, 0xFF], &text("Hm.")]), &[]);
    assert_eq!(prompt.tests(), [Test::Acquainted]);
}

#[test]
fn t16_an_action_code_blocks_the_static_render() {
    let (_, dep) = pair_dep(record(&[&text("Take this."), &[0x86, b'C' | 0x80]]), &[]);
    assert!(dep.is_fixed(), "a grant repeats with the same text");
    assert!(!dep.renders_statically());
}

#[test]
fn t16_scoped_answers_may_differ_until_s8_1() {
    let follow_up = block(
        0x91,
        &text("Well?"),
        &[text("Hm."), text("AYE"), text("Good.")],
    );
    let fields = blob(&[("QUERY", vec![0x91])], &[follow_up]);
    let record = RecordAddr::Scoped {
        decl: 0,
        arm: RecordAddr::ARM_ANSWERED,
    };
    let dep = record_dependence(&fields, record);
    assert!(dep.reply());
    let entry = heard(record, &dep, &BASE, &BASE);
    assert_eq!(asked_state(&[entry], &dep, &BASE), AskedState::MayDiffer);
    assert_eq!(
        record_dependence(
            &fields,
            RecordAddr::Scoped {
                decl: 1,
                arm: RecordAddr::ARM_ANSWERED
            }
        ),
        Dependence::Unknown
    );
}

/// The display rule's five rows (8.3.13 step 3).
#[test]
fn t16_the_display_rule() {
    let fixed = Dependence::Known {
        tests: Vec::new(),
        reply: false,
        action: false,
        branches: false,
    };
    let reply = Dependence::Known {
        tests: Vec::new(),
        reply: true,
        action: false,
        branches: true,
    };
    let gated = Dependence::Known {
        tests: vec![Test::Acquainted, Test::Standing(50)],
        reply: false,
        action: false,
        branches: true,
    };
    let known = TestInputs {
        acquainted: true,
        ..BASE
    };
    // 1. No tests, no reply: same.
    assert_eq!(state(&fixed, &[BASE], &known), AskedState::Same);
    // 2. Reply: may differ.
    assert_eq!(state(&reply, &[BASE], &BASE), AskedState::MayDiffer);
    // 3. Unknown: may differ.
    assert_eq!(
        state(&Dependence::Unknown, &[BASE], &BASE),
        AskedState::MayDiffer
    );
    // 4. An entry with pre == post == now: same.
    assert_eq!(state(&gated, &[known], &known), AskedState::Same);
    // 5. Otherwise: may differ.
    assert_eq!(state(&gated, &[BASE], &known), AskedState::MayDiffer);
    // Heard on both branches: same whatever the state.
    for now in [BASE, known] {
        assert_eq!(state(&gated, &[BASE, known], &now), AskedState::Same);
    }
    // An entry whose turn crossed the test (pre != post) never matches.
    let crossed = heard(RecordAddr::Pair(0), &gated, &BASE, &known);
    assert_eq!(
        asked_state(&[crossed], &gated, &known),
        AskedState::MayDiffer
    );
    // An entry with unknown bits never matches; nothing heard is not asked.
    let unknown = heard(RecordAddr::Pair(0), &Dependence::Unknown, &BASE, &BASE);
    assert_eq!(
        asked_state(&[unknown], &gated, &BASE),
        AskedState::MayDiffer
    );
    assert_eq!(asked_state(&[], &fixed, &BASE), AskedState::MayDiffer);
}

#[test]
fn evaluate_sets_one_bit_per_test_in_order() {
    let tests = [
        Test::Acquainted,
        Test::Standing(50),
        Test::Gold(100),
        Test::PartyRoom,
    ];
    assert_eq!(evaluate(&tests, &inputs(false, 49, 99, 6)), 0b0000);
    assert_eq!(evaluate(&tests, &inputs(true, 50, 100, 5)), 0b1111);
    assert_eq!(evaluate(&tests, &inputs(false, 50, 0, 6)), 0b0010);
    assert_eq!(evaluate(&[], &BASE), 0);
}

// --- M6-04: notes, misses and kept text through the codec. ----------------

#[test]
fn m6_04_notes_round_trip_through_the_codec() {
    let mut log = ClueLog::new();
    log.fold(entry(1));
    log.fold(entry(2));
    let person = note_owner::person(NpcKey { scene: 9, slot: 4 });
    log.set_note(note_owner::CLUE, 1, "  check the glade at night ");
    log.set_note(note_owner::PERSON, person, "owes me a favour");
    log.set_note(note_owner::SAVE, 0, "before the long ride");
    // A clue note on an entry that does not exist is ignored.
    log.set_note(note_owner::CLUE, 9, "nowhere");
    assert_eq!(log.notes().len(), 3);
    assert_eq!(
        log.note_for(note_owner::CLUE, 1).unwrap().text,
        "check the glade at night"
    );
    assert_eq!(note_owner::person_of(person), NpcKey { scene: 9, slot: 4 });
    let back = decode_payload(&encode_payload(&log, EncodeOptions::default()).unwrap()).unwrap();
    assert_eq!(back.notes(), log.notes());
    // Editing replaces, blank removes; a noted clue is protected.
    let mut edited = back.clone();
    edited.mark_saved();
    edited.set_note(note_owner::CLUE, 1, "the glade, at dawn");
    assert!(edited.is_dirty());
    assert!(edited.is_protected(1));
    edited.set_note(note_owner::SAVE, 0, "   ");
    assert_eq!(edited.notes().len(), 2);
    assert!(edited.note_for(note_owner::SAVE, 0).is_none());
    let long = "x".repeat(NOTE_MAX_LEN + 20);
    edited.set_note(note_owner::SAVE, 0, &long);
    assert_eq!(
        edited.note_for(note_owner::SAVE, 0).unwrap().text.len(),
        NOTE_MAX_LEN
    );
    let again =
        decode_payload(&encode_payload(&edited, EncodeOptions::default()).unwrap()).unwrap();
    assert_eq!(again.notes(), edited.notes());
}

#[test]
fn m6_04_misses_are_never_written_with_clue_misses_off() {
    let mut log = ClueLog::new();
    log.fold(entry(1));
    log.add_miss(Miss {
        npc: NPC,
        at: GameMinute(5),
        typed: b"XQMARKMISSWORD!!!".to_vec(),
    });
    assert_eq!(log.misses()[0].typed.len(), MISS_MAX_LEN);
    let off = encode_payload(&log, EncodeOptions::default()).unwrap();
    assert!(
        !off.windows(5).any(|w| w == b"XQMAR"),
        "the typed bytes are not in the file"
    );
    assert!(decode_payload(&off).unwrap().misses().is_empty());
    let on = encode_payload(
        &log,
        EncodeOptions {
            keep_misses: true,
            ..EncodeOptions::default()
        },
    )
    .unwrap();
    assert_eq!(decode_payload(&on).unwrap().misses(), log.misses());
}

#[test]
fn t9_text_is_dropped_after_leaving_kept() {
    let mut log = ClueLog::new();
    log.fold(entry(1));
    log.keep_text(SeenRows {
        entry: 0,
        rows: vec![TextRow {
            bytes: b"XQMARKROW".to_vec(),
            runic: 0b10,
        }],
    });
    let kept = EncodeOptions {
        keep_text: true,
        ..EncodeOptions::default()
    };
    // Saved under `kept`: the file holds the rows.
    let mut read = decode_payload(&encode_payload(&log, kept).unwrap()).unwrap();
    assert_eq!(read.text(), log.text());
    assert!(read.entries()[0].flags.has(EntryFlags::HAS_TEXT));
    // `clue_text` leaves `kept`: the next save drops TEXT, and the log read
    // back from it has no rows and no entry claiming any.
    read.drop_text();
    let bytes = encode_payload(&read, EncodeOptions::default()).unwrap();
    assert!(!bytes.windows(6).any(|w| w == b"XQMARK"));
    let after = decode_payload(&bytes).unwrap();
    assert!(after.text().is_empty());
    assert!(!after.entries()[0].flags.has(EntryFlags::HAS_TEXT));
}

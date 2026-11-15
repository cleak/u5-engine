//! The quest journal's derivation (design 5.6; build plan M5-01): the phase
//! and ruin tables, the summary, tier monotonicity, the evidence gates, the
//! no-leak rules, the snapshot sources and the any-byte property.

use super::*;
use crate::test_fixtures::{debug_game_dir, saved_game_seed_bytes};
use crate::{
    SAVE_AVATAR_NAME_OFFSET, SAVE_CHARACTER_RECORD_LEN, SAVE_CLIMBING_GEAR_OFFSET,
    SAVE_INN_REGISTRY_OFFSET, SAVE_MORAL_STANDING_OFFSET, SAVE_NPC_NAME_KNOWN_MASKS_OFFSET,
    SAVE_PARTY_SIZE_OFFSET, SAVE_ROSTER_OFFSET, SAVE_SHADOWLORD_HIDEOUTS_OFFSET,
    SAVE_SHRINE_CODEX_MASK_OFFSET, SAVE_SHRINE_ORDAINED_MASK_OFFSET, SAVE_SHRINE_RUIN_FLAGS_OFFSET,
    SAVE_SPECIAL_ITEM_OFFSET, SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET, SAVED_GAM_LEN,
    SPECIAL_ITEM_CROWN_LB_INDEX, SPECIAL_ITEM_SPYGLASS_INDEX,
};

const PHASE_TABLE: &[(u8, u8, ShrinePhase)] = &[
    (0, 0, ShrinePhase::NotStarted),
    (1, 0, ShrinePhase::Ordained),
    (1, 1, ShrinePhase::CodexRead),
    (0, 1, ShrinePhase::Complete),
];

/// A fixed-seed generator for the property tests; never the game RNG.
struct XorShift(u64);

impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn byte(&mut self) -> u8 {
        self.next() as u8
    }

    /// Mostly the spec's values, sometimes any byte.
    fn pick(&mut self, usual: &[u8]) -> u8 {
        if self.next().is_multiple_of(4) {
            self.byte()
        } else {
            usual[(self.next() % usual.len() as u64) as usize]
        }
    }
}

/// The fresh-save snapshot: factory values after chargen, the Avatar alone.
fn fresh() -> QuestSnapshot {
    QuestSnapshot {
        ordained_mask: 0,
        codex_mask: 0,
        ruin: [0; 8],
        seals: [0; 8],
        shadowlords: [ShadowlordState::NotPlaced; 3],
        special_items: [0; SPECIAL_ITEM_COUNT],
        climbing_gear: 0,
        moral_standing: 0,
        companions: vec![CompanionFact {
            name: "Avatar".into(),
            place: CompanionPlace::Party,
        }],
        introduced: Vec::new(),
        current_scene: 0,
    }
}

fn random_snapshot(rng: &mut XorShift) -> QuestSnapshot {
    let mut snap = fresh();
    snap.ordained_mask = rng.byte();
    snap.codex_mask = rng.byte();
    for byte in snap.ruin.iter_mut() {
        *byte = rng.pick(&[0, 0, 0x80, 0x81, 0x01]);
    }
    for byte in snap.seals.iter_mut() {
        *byte = rng.pick(&[0, 0, 0x80]);
    }
    for state in snap.shadowlords.iter_mut() {
        *state = ShadowlordState::from_slot(rng.pick(&[0, 3, 0xFF]));
    }
    for byte in snap.special_items.iter_mut() {
        *byte = rng.pick(&[0, 0, 1]);
    }
    snap.climbing_gear = rng.pick(&[0, 1]);
    snap.moral_standing = rng.byte();
    for _ in 0..rng.next() % 4 {
        let place = match rng.next() % 3 {
            0 => CompanionPlace::Party,
            1 => CompanionPlace::Lodged {
                scene: rng.pick(&[2, 5, 40]),
            },
            _ => CompanionPlace::Executed,
        };
        snap.companions.push(CompanionFact {
            name: "Iolo".into(),
            place,
        });
    }
    if rng.next().is_multiple_of(2) {
        snap.introduced = vec![(2, rng.byte().max(1)), (14, 1)];
    }
    snap.current_scene = rng.pick(&[0, 2, 33, 40]);
    snap
}

fn section_ids(model: &JournalModel) -> Vec<SectionId> {
    model.sections.iter().map(|section| section.id).collect()
}

fn rows(model: &JournalModel, id: SectionId) -> Vec<&JournalRow> {
    model
        .section(id)
        .map(|section| section.rows.iter().collect())
        .unwrap_or_default()
}

fn row_keys(model: &JournalModel) -> Vec<RowKey> {
    model
        .sections
        .iter()
        .flat_map(|section| section.rows.iter().map(|row| row.key))
        .collect()
}

// --- T1, T2, P1. ---------------------------------------------------------

#[test]
fn shrine_phase_table() {
    for v in ShrineVirtue::ALL {
        let bit = virtue_bit(v);
        assert_eq!(bit, v.bit());
        for &(o, c, want) in PHASE_TABLE {
            let om = if o == 1 { bit } else { 0 };
            let cm = if c == 1 { bit } else { 0 };
            // Noise in every other virtue's bits.
            let noise = !bit;
            assert_eq!(shrine_phase(om | noise, cm, v), want);
            assert_eq!(shrine_phase(om, cm | noise, v), want);
        }
    }
}

#[test]
fn ruin_table() {
    assert_eq!(shrine_condition(0x00), ShrineCondition::Intact);
    for byte in [0x80, 0x81, 0xFF] {
        assert_eq!(shrine_condition(byte), ShrineCondition::Ruined, "{byte:#x}");
    }
    for byte in [0x01, 0x7F] {
        assert_eq!(
            shrine_condition(byte),
            ShrineCondition::Restored,
            "{byte:#x}"
        );
    }
}

#[test]
fn the_summary_is_a_bit_count_for_every_mask_pair() {
    for ordained in 0..=255u8 {
        for codex in 0..=255u8 {
            let mut snap = fresh();
            snap.ordained_mask = ordained;
            snap.codex_mask = codex;
            let summary = snap.virtue_summary();
            let not_started = ShrineVirtue::ALL
                .into_iter()
                .filter(|v| snap.phase(*v) == ShrinePhase::NotStarted)
                .count() as u8;
            assert_eq!(summary.complete + summary.in_progress + not_started, 8);
            assert_eq!(summary.complete, (codex & !ordained).count_ones() as u8);
            assert_eq!(summary.in_progress, ordained.count_ones() as u8);
            assert_eq!(summary.ruined, 0);
            assert_eq!(
                summary.complete == 8,
                (ordained, codex) == (0x00, 0xFF),
                "{ordained:#x} {codex:#x}"
            );
            assert_eq!(
                summary.complete == 8,
                crate::all_virtues_complete(ordained, codex)
            );
        }
    }
}

#[test]
fn the_header_counts_only_what_was_done() {
    assert_eq!(
        derive(&fresh(), SpoilerTier::Goals).header,
        "Nothing recorded yet"
    );
    let mut snap = fresh();
    snap.ordained_mask = 0b0000_0110;
    snap.codex_mask = 0b1001_0101;
    snap.ruin[6] = 0x80;
    let model = derive(&snap, SpoilerTier::Progress);
    assert_eq!(model.header, "3 complete, 2 in progress, 1 ruined");
    snap.ruin[6] = 0x01;
    assert_eq!(
        derive(&snap, SpoilerTier::Full).header,
        "3 complete, 2 in progress"
    );
    for tier in SpoilerTier::ALL {
        assert!(!derive(&snap, tier).header.contains('/'));
    }
}

// --- Monotonicity. -------------------------------------------------------

#[test]
fn derive_is_monotone_in_the_tier() {
    let mut rng = XorShift(0x5EED_0005_0001);
    for _ in 0..2000 {
        let snap = random_snapshot(&mut rng);
        let models: Vec<JournalModel> = SpoilerTier::ALL
            .into_iter()
            .map(|tier| derive(&snap, tier))
            .collect();
        for model in &models {
            assert_eq!(model.spoilers, model.tier.is_spoiler());
            assert_eq!(model.summary, snap.virtue_summary());
            for row in model.sections.iter().flat_map(|section| &section.rows) {
                assert!(
                    row.secret.is_none() || model.tier == SpoilerTier::Full,
                    "{:?} has a secret at {:?}",
                    row.key,
                    model.tier
                );
            }
        }
        for pair in models.windows(2) {
            let (low, high) = (&pair[0], &pair[1]);
            // Sections keep their order and only grow.
            let high_ids = section_ids(high);
            assert!(section_ids(low).iter().all(|id| high_ids.contains(id)));
            for section in &low.sections {
                let upper = high.section(section.id).unwrap();
                for row in &section.rows {
                    let same = upper
                        .rows
                        .iter()
                        .find(|other| other.key == row.key)
                        .unwrap_or_else(|| panic!("{:?} lost at {:?}", row.key, high.tier));
                    assert!(
                        same.detail.starts_with(&row.detail),
                        "{:?}: {:?} is not a prefix of {:?}",
                        row.key,
                        row.detail,
                        same.detail
                    );
                }
            }
        }
    }
}

// --- Evidence gates. -----------------------------------------------------

#[test]
fn a_secret_appears_only_with_its_evidence() {
    let mut rng = XorShift(0x5EED_0005_0002);
    for _ in 0..2000 {
        let snap = random_snapshot(&mut rng);
        let model = derive(&snap, SpoilerTier::Full);
        for virtue in ShrineVirtue::ALL {
            let row = rows(&model, SectionId::Virtues)[virtue.index()];
            let evidence = snap.phase(virtue) != ShrinePhase::NotStarted
                || snap.condition(virtue) != ShrineCondition::Intact;
            assert_eq!(
                row.secret,
                evidence.then(|| shrine_mantra_for(virtue)),
                "{virtue:?}"
            );
        }
        for row in rows(&model, SectionId::Dungeons) {
            let RowKey::Dungeon(record) = row.key else {
                panic!("{:?}", row.key)
            };
            let open = snap.seal_open(usize::from(record));
            assert_eq!(row.secret.is_some(), open, "{record}");
            if let Some(word) = row.secret {
                assert_eq!(Some(word), dungeon_word_of_power(33 + record));
            }
        }
        for row in rows(&model, SectionId::Shadowlords) {
            let RowKey::Shadowlord(slot) = row.key else {
                panic!("{:?}", row.key)
            };
            let vanquished = snap.shadowlords[usize::from(slot)] == ShadowlordState::Vanquished;
            assert_eq!(row.secret.is_some(), vanquished, "{slot}");
        }
        // A restoration Word appears only with its seal open.
        for virtue in ShrineVirtue::ALL {
            let row = rows(&model, SectionId::Virtues)[virtue.index()];
            let word = dungeon_word_of_power(33 + virtue.index() as u8).unwrap();
            let shows_word = row.detail.iter().any(|line| line.as_str().contains(word));
            if shows_word {
                assert!(snap.seal_open(virtue.index()), "{virtue:?}");
                assert_eq!(snap.condition(virtue), ShrineCondition::Ruined);
            }
        }
    }
}

/// "Sealed" reads only on the row of the dungeon the party is inside, at
/// every tier (5.2.8, 2); an unopened dungeon elsewhere has no state.
#[test]
fn sealed_reads_only_for_the_dungeon_the_party_is_inside() {
    let mut rng = XorShift(0x5EED_0005_0009);
    for _ in 0..500 {
        let snap = random_snapshot(&mut rng);
        for tier in SpoilerTier::ALL {
            for row in rows(&derive(&snap, tier), SectionId::Dungeons) {
                let RowKey::Dungeon(record) = row.key else {
                    panic!()
                };
                let opened = snap.seal_open(usize::from(record));
                let inside = snap.current_scene == 33 + record;
                let expected = if opened {
                    text::OPENED
                } else if inside {
                    text::SEALED
                } else {
                    ""
                };
                assert_eq!(row.value.as_str(), expected, "{record} at {tier:?}");
            }
        }
    }
}

#[test]
fn tiers_one_and_two_show_only_discovered_rows() {
    let mut rng = XorShift(0x5EED_0005_0003);
    for _ in 0..2000 {
        let snap = random_snapshot(&mut rng);
        for tier in [SpoilerTier::Progress, SpoilerTier::Goals] {
            let model = derive(&snap, tier);
            assert!(!model.spoilers);
            assert!(model.section(SectionId::Endgame).is_none());
            assert_eq!(rows(&model, SectionId::Virtues).len(), 8);
            for row in rows(&model, SectionId::Dungeons) {
                let RowKey::Dungeon(record) = row.key else {
                    panic!()
                };
                assert!(
                    snap.seal_open(usize::from(record)) || snap.current_scene == 33 + record,
                    "{record}"
                );
            }
            for row in rows(&model, SectionId::Shadowlords) {
                let RowKey::Shadowlord(slot) = row.key else {
                    panic!()
                };
                let slot = usize::from(slot);
                let vanquished = snap.shadowlords[slot] == ShadowlordState::Vanquished;
                assert!(vanquished || snap.special_items[6 + slot] != 0);
                // The name only once vanquished.
                let name = text::shadowlord_display_name(shadowlord_name_for_slot(slot).unwrap());
                assert_eq!(row.label.as_str() == name, vanquished);
            }
            for row in rows(&model, SectionId::Regalia) {
                let RowKey::Regalia(index) = row.key else {
                    panic!()
                };
                assert_ne!(snap.special_items[usize::from(index)], 0);
            }
            for virtue_row in rows(&model, SectionId::Virtues) {
                // A virtue not begun never gets a goal at tiers 1 and 2.
                if virtue_row.mark == StatusMark::NONE {
                    assert!(virtue_row.value.is_empty());
                    for line in &virtue_row.detail {
                        assert!(
                            [text::RUIN_LINE, text::RESTORED_LINE].contains(&line.as_str()),
                            "{line}"
                        );
                    }
                }
            }
        }
    }
}

// --- No leaks. -----------------------------------------------------------

#[test]
fn no_hideout_town_for_any_slot_value() {
    let towns: Vec<&str> = (1..=8).filter_map(town_resident_name).collect();
    assert_eq!(towns.len(), 8);
    for slot in 0..SHADOWLORD_COUNT {
        for byte in 0..=255u8 {
            let mut hideouts = [0u8; SHADOWLORD_COUNT];
            hideouts[slot] = byte;
            let mut snap = fresh();
            snap.shadowlords = hideouts.map(ShadowlordState::from_slot);
            snap.special_items[6 + slot] = byte & 1;
            for tier in SpoilerTier::ALL {
                let model = derive(&snap, tier);
                for row in rows(&model, SectionId::Shadowlords) {
                    for text in row.strings() {
                        for town in &towns {
                            assert!(!text.contains(town), "{byte:#x} {tier:?}: {text}");
                        }
                    }
                }
            }
        }
    }
    // Out-of-range values read Unknown.
    for byte in [9u8, 0x10, 0x7F, 0x80, 0xFE] {
        assert_eq!(ShadowlordState::from_slot(byte), ShadowlordState::Unknown);
    }
    for byte in 1..=8u8 {
        assert_eq!(ShadowlordState::from_slot(byte), ShadowlordState::AtLarge);
    }
}

#[test]
fn a_ruined_spirituality_shrine_has_no_restoration_line() {
    let spirit = ShrineVirtue::Spirituality.index();
    let mut snap = fresh();
    snap.ruin[spirit] = 0x80;
    snap.seals = [0x80; 8];
    for tier in SpoilerTier::ALL {
        let model = derive(&snap, tier);
        let row = rows(&model, SectionId::Virtues)[spirit];
        assert_eq!(row.note.as_ref().unwrap().1.as_str(), "Ruined");
        for line in &row.detail {
            let line = line.as_str();
            assert!(
                !line.contains("Restore") && !line.contains("Yell") && !line.contains("Glinkie"),
                "{tier:?}: {line}"
            );
        }
        let want: &[&str] = if tier >= SpoilerTier::Hints {
            &[text::RUIN_LINE, text::RUIN_NO_RESTORE]
        } else {
            &[text::RUIN_LINE]
        };
        assert_eq!(
            row.detail
                .iter()
                .map(JournalText::as_str)
                .collect::<Vec<_>>(),
            want
        );
    }
    // Another ruined shrine is offered the method at tier 4 with its seal.
    snap.ruin[7] = 0x80;
    let model = derive(&snap, SpoilerTier::Full);
    let humility = rows(&model, SectionId::Virtues)[7];
    assert!(
        humility
            .detail
            .iter()
            .any(|line| line.as_str().contains("VERAMOCOR"))
    );
}

#[test]
fn no_text_says_met_killed_or_interred() {
    let banned = ["met", "killed", "interred", "kill", "dead", "died"];
    let check = |text: &str| {
        for word in text
            .split(|c: char| !c.is_ascii_alphanumeric())
            .map(str::to_ascii_lowercase)
        {
            assert!(!banned.contains(&word.as_str()), "`{word}` in: {text}");
        }
    };
    let mut rng = XorShift(0x5EED_0005_0004);
    for _ in 0..500 {
        let snap = random_snapshot(&mut rng);
        for tier in SpoilerTier::ALL {
            for text in derive(&snap, tier).strings() {
                check(text);
            }
        }
    }
    for text in [
        text::EXECUTED,
        text::LODGED,
        text::IN_PARTY,
        text::INTRODUCED_LABEL,
    ] {
        check(text);
    }
    assert_eq!(text::EXECUTED, "Executed by Blackthorn");
}

// --- The fresh save and one event at a time (1.4.7, 5.6). ----------------

#[test]
fn the_fresh_save_shows_nothing_undiscovered() {
    let names = crate::test_fixtures::journal_undiscovered_names();
    for tier in [SpoilerTier::Progress, SpoilerTier::Goals] {
        let model = derive(&fresh(), tier);
        assert_eq!(model.header, "Nothing recorded yet");
        assert_eq!(section_ids(&model), [SectionId::Virtues, SectionId::People]);
        for row in rows(&model, SectionId::Virtues) {
            assert_eq!(row.mark, StatusMark::NONE);
            assert_eq!(row.mark.glyph, '-');
            assert!(row.value.is_empty() && row.detail.is_empty());
        }
        for text in model.strings() {
            for name in &names {
                assert!(
                    !text
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
                        .any(|word| word == name)
                        && !text.contains(name.as_str()),
                    "{tier:?}: {name} in {text}"
                );
            }
            assert!(!text.contains('/'), "{text}");
            assert!(!text.contains(" of "), "{text}");
        }
        assert!(!model.spoilers);
    }
    for tier in [SpoilerTier::Hints, SpoilerTier::Full] {
        let model = derive(&fresh(), tier);
        assert!(model.spoilers);
        assert!(model.section(SectionId::Endgame).is_some());
    }
}

#[test]
fn one_event_adds_exactly_its_own_row() {
    let base = row_keys(&derive(&fresh(), SpoilerTier::Goals));
    let events: Vec<(&str, Box<dyn Fn(&mut QuestSnapshot)>, Vec<RowKey>)> = vec![
        ("ordained", Box::new(|s| s.ordained_mask = 0b10), vec![]),
        (
            "seal",
            Box::new(|s| s.seals[3] = 0x80),
            vec![RowKey::Dungeon(3)],
        ),
        (
            "vanquished",
            Box::new(|s| s.shadowlords[1] = ShadowlordState::Vanquished),
            vec![RowKey::Shadowlord(1)],
        ),
        (
            "shard",
            Box::new(|s| s.special_items[8] = 1),
            vec![RowKey::Shadowlord(2)],
        ),
        (
            "crown",
            Box::new(|s| s.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] = 1),
            vec![RowKey::Regalia(SPECIAL_ITEM_CROWN_LB_INDEX as u8)],
        ),
        (
            "spyglass",
            Box::new(|s| s.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 1),
            vec![RowKey::Item(3)],
        ),
        (
            "grapple",
            Box::new(|s| s.climbing_gear = 1),
            vec![RowKey::Item(2)],
        ),
        (
            "lodged",
            Box::new(|s| {
                s.companions.push(CompanionFact {
                    name: "Shamino".into(),
                    place: CompanionPlace::Lodged { scene: 2 },
                })
            }),
            vec![RowKey::Companion(1)],
        ),
        (
            "at large",
            Box::new(|s| s.shadowlords = [ShadowlordState::AtLarge; 3]),
            vec![],
        ),
    ];
    for (name, apply, added) in events {
        let mut snap = fresh();
        apply(&mut snap);
        let keys = row_keys(&derive(&snap, SpoilerTier::Goals));
        let mut want = base.clone();
        want.extend(added.iter().copied());
        let mut got = keys.clone();
        got.sort_by_key(|key| format!("{key:?}"));
        want.sort_by_key(|key| format!("{key:?}"));
        assert_eq!(got, want, "{name}");
    }
    // Ordination changes only Honesty... here Compassion's own row.
    let mut snap = fresh();
    snap.ordained_mask = 0b10;
    let model = derive(&snap, SpoilerTier::Goals);
    let virtues = rows(&model, SectionId::Virtues);
    assert_eq!(virtues[1].value.as_str(), "Read the Codex");
    assert!(
        virtues
            .iter()
            .enumerate()
            .all(|(i, row)| i == 1 || row.value.is_empty())
    );
    assert_eq!(model.header, "0 complete, 1 in progress");
    // A lodged companion names the inn's town, a shard alone its virtue word.
    let mut snap = fresh();
    snap.companions.push(CompanionFact {
        name: "Shamino".into(),
        place: CompanionPlace::Lodged { scene: 2 },
    });
    snap.special_items[6] = 1;
    let model = derive(&snap, SpoilerTier::Goals);
    let people = rows(&model, SectionId::People);
    assert_eq!(
        people[1].detail[0].as_str(),
        "Waiting at the inn in Britain."
    );
    assert_eq!(
        rows(&model, SectionId::Shadowlords)[0].label.as_str(),
        "Falsehood shard"
    );
}

#[test]
fn the_tier_labels_follow_the_design() {
    use crate::shrine_virtue::ShrineQuestState::*;
    let expect = [
        (NotStarted, ["", "", "", ""]),
        (
            Ordained,
            [
                "Ordained",
                "Read the Codex",
                "Read the Codex",
                "Read the Codex",
            ],
        ),
        (
            CodexRead,
            [
                "Codex read",
                "Codex read",
                "Return to the shrine",
                "Return to the shrine",
            ],
        ),
        (Complete, ["Complete"; 4]),
    ];
    for (phase, labels) in expect {
        for (tier, label) in SpoilerTier::ALL.into_iter().zip(labels) {
            assert_eq!(text::phase_value(phase, tier), label, "{phase:?} {tier:?}");
        }
    }
    let mut snap = fresh();
    snap.ruin[0] = 0x05;
    let model = derive(&snap, SpoilerTier::Progress);
    assert_eq!(
        rows(&model, SectionId::Virtues)[0].suffix,
        Some(StatusMark::RESTORED)
    );
    assert_eq!(StatusMark::for_phase(Ordained), StatusMark::new('>', 14));
    assert_eq!(StatusMark::for_phase(CodexRead), StatusMark::new('<', 11));
    assert_eq!(StatusMark::for_phase(Complete), StatusMark::new('*', 10));
    assert_eq!(StatusMark::ALERT, StatusMark::new('!', 12));
}

#[test]
fn people_count_introductions_and_never_an_unrecruited_record() {
    let mut snap = fresh();
    snap.introduced = vec![(1, 3), (2, 1)];
    snap.companions.push(CompanionFact {
        name: "Dupre".into(),
        place: CompanionPlace::Executed,
    });
    let model = derive(&snap, SpoilerTier::Progress);
    let people: Vec<(String, String)> = rows(&model, SectionId::People)
        .into_iter()
        .map(|row| (row.label.to_string(), row.value.to_string()))
        .collect();
    let want = [
        ("Avatar", "In the party"),
        ("Dupre", "Executed by Blackthorn"),
        ("Introduced yourself to", "4 people"),
        ("Moonglow", "3"),
        ("Britain", "1"),
        ("Karma", "0"),
    ];
    assert_eq!(
        people,
        want.map(|(label, value)| (label.to_string(), value.to_string()))
    );
}

// --- The snapshot sources. -----------------------------------------------

/// A synthetic SAVED.GAM: the Avatar and Iolo in the party, Shamino lodged
/// at Britain's inn, Dupre executed, and one of each quest fact.
fn synthetic_save() -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(0, 0, 10, 10);
    let names: [&[u8]; 4] = [b"Avatar", b"Iolo", b"Shamino", b"Dupre"];
    for (slot, name) in names.iter().enumerate() {
        let record = SAVE_ROSTER_OFFSET + slot * SAVE_CHARACTER_RECORD_LEN;
        bytes[record..record + name.len()].copy_from_slice(name);
    }
    assert_eq!(SAVE_AVATAR_NAME_OFFSET, SAVE_ROSTER_OFFSET);
    bytes[SAVE_PARTY_SIZE_OFFSET] = 2;
    bytes[SAVE_INN_REGISTRY_OFFSET + 2 * SAVE_CHARACTER_RECORD_LEN] = 2;
    bytes[SAVE_INN_REGISTRY_OFFSET + 3 * SAVE_CHARACTER_RECORD_LEN] = 0x7F;
    bytes[SAVE_SHRINE_ORDAINED_MASK_OFFSET] = 0b0000_0010;
    bytes[SAVE_SHRINE_CODEX_MASK_OFFSET] = 0b0000_0001;
    bytes[SAVE_SHRINE_RUIN_FLAGS_OFFSET + 6] = 0x80;
    bytes[SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET] = 0x80;
    bytes[SAVE_SHADOWLORD_HIDEOUTS_OFFSET] = 0xFF;
    bytes[SAVE_SHADOWLORD_HIDEOUTS_OFFSET + 1] = 4;
    bytes[SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_CROWN_LB_INDEX] = 1;
    bytes[SAVE_SPECIAL_ITEM_OFFSET + 7] = 1;
    bytes[SAVE_CLIMBING_GEAR_OFFSET] = 1;
    bytes[SAVE_MORAL_STANDING_OFFSET] = 42;
    // Two people introduced in Moonglow (scene 1).
    bytes[SAVE_NPC_NAME_KNOWN_MASKS_OFFSET] = 0b101;
    bytes
}

fn golden() -> QuestSnapshot {
    QuestSnapshot {
        ordained_mask: 0b10,
        codex_mask: 0b01,
        ruin: [0, 0, 0, 0, 0, 0, 0x80, 0],
        seals: [0x80, 0, 0, 0, 0, 0, 0, 0],
        shadowlords: [
            ShadowlordState::Vanquished,
            ShadowlordState::AtLarge,
            ShadowlordState::NotPlaced,
        ],
        special_items: {
            let mut items = [0; SPECIAL_ITEM_COUNT];
            items[SPECIAL_ITEM_CROWN_LB_INDEX] = 1;
            items[7] = 1;
            items
        },
        climbing_gear: 1,
        moral_standing: 42,
        companions: vec![
            CompanionFact {
                name: "Avatar".into(),
                place: CompanionPlace::Party,
            },
            CompanionFact {
                name: "Iolo".into(),
                place: CompanionPlace::Party,
            },
            CompanionFact {
                name: "Shamino".into(),
                place: CompanionPlace::Lodged { scene: 2 },
            },
            CompanionFact {
                name: "Dupre".into(),
                place: CompanionPlace::Executed,
            },
        ],
        introduced: vec![(1, 2)],
        current_scene: 0,
    }
}

#[test]
fn from_save_bytes_equals_the_golden_snapshot_and_needs_no_file() {
    // Bytes only: there is no game directory to open.
    let snap = QuestSnapshot::from_save_bytes(&synthetic_save()).unwrap();
    assert_eq!(snap, golden());
    let model = derive(&snap, SpoilerTier::Goals);
    assert_eq!(model.header, "1 complete, 1 in progress, 1 ruined");
    assert_eq!(
        section_ids(&model),
        [
            SectionId::Virtues,
            SectionId::Dungeons,
            SectionId::Shadowlords,
            SectionId::Regalia,
            SectionId::Items,
            SectionId::People,
        ]
    );
    let shadowlords = rows(&model, SectionId::Shadowlords);
    assert_eq!(shadowlords.len(), 2);
    assert_eq!(shadowlords[0].label.as_str(), "Faulinei");
    assert_eq!(
        shadowlords[0].note.as_ref().unwrap().1.as_str(),
        "Shard destroyed"
    );
    assert_eq!(shadowlords[1].label.as_str(), "Hatred shard");
    assert_eq!(
        rows(&model, SectionId::Dungeons)[0].label.as_str(),
        "Deceit"
    );

    // A short or empty save is an error, never a partial snapshot.
    assert!(QuestSnapshot::from_save_bytes(&[0; 16]).is_err());
    let mut no_game = synthetic_save();
    no_game[SAVE_AVATAR_NAME_OFFSET] = 0;
    assert!(QuestSnapshot::from_save_bytes(&no_game).is_err());
}

// --- The offline report (5.2.11; R2). ------------------------------------

/// R2: the synthetic save's report as golden text at the default tier and
/// at tier 4; at every tier the first line carries SPOILERS exactly at the
/// spoiler tiers, and a tier's report holds every line of the tier below.
#[test]
fn the_report_of_the_synthetic_save_matches_the_golden_text() {
    let snap = QuestSnapshot::from_save_bytes(&synthetic_save()).unwrap();
    let text = |tier| report::format_text(&derive(&snap, tier));
    assert_eq!(text(SpoilerTier::Goals), REPORT_GOLDEN_TIER_2);
    assert_eq!(text(SpoilerTier::Full), REPORT_GOLDEN_TIER_4);
    let mut below: Option<String> = None;
    for tier in SpoilerTier::ALL {
        let report = text(tier);
        let first = report.lines().next().unwrap();
        assert_eq!(first.contains("SPOILERS"), tier.is_spoiler(), "{first}");
        assert!(report.ends_with('\n'));
        if let Some(below) = below {
            // Labels and markers are stable; values may sharpen (tier 2
            // turns "Ordained" into "Read the Codex"), so compare the
            // detail lines, which only grow.
            for line in below.lines().filter(|line| line.starts_with(DETAIL)) {
                assert!(report.contains(line), "tier {}: {line}", tier.number());
            }
        }
        below = Some(report);
    }
}

const DETAIL: &str = "       ";

const REPORT_GOLDEN_TIER_2: &str = "\
Quest journal - tier 2 (named goals)
Virtues: 1 complete, 1 in progress, 1 ruined
  *  Honesty       Complete
       This virtue's quest is complete.
  >  Compassion    Read the Codex
       You were ordained at its shrine.
       The shrine sent you to read the Codex.
  -  Valor
  -  Justice
  -  Sacrifice
  -  Honor
  -  Spirituality                  ! Ruined
       The shrine lies in ruin.
  -  Humility
Dungeons
  *  Deceit  Opened
       Its entrance has been opened.
Shadowlords
  *  Faulinei      Vanquished    Shard destroyed
       Its shard has been destroyed.
  -  Hatred shard                Shard carried
       Its shard is carried.
Regalia
  *  Crown  Carried
Items
  *  Grapple  Carried
People
     Avatar                  In the party
     Iolo                    In the party
     Shamino                 Lodged
       Waiting at the inn in Britain.
  !  Dupre                   Executed by Blackthorn
     Introduced yourself to  2 people
     Moonglow                2
     Karma                   42
";

const REPORT_GOLDEN_TIER_4: &str = "\
Quest journal - tier 4 (full checklist) - SPOILERS
Virtues: 1 complete, 1 in progress, 1 ruined
  *  Honesty       Complete
       This virtue's quest is complete.
       Mantra: Ahm
  >  Compassion    Read the Codex
       You were ordained at its shrine.
       The shrine sent you to read the Codex.
       Mantra: Mu
  -  Valor
       Meditate at its shrine and give its mantra to begin.
       Thorne knows its mantra.
  -  Justice
       Meditate at its shrine and give its mantra to begin.
       Chamfort knows its mantra.
  -  Sacrifice
       Meditate at its shrine and give its mantra to begin.
  -  Honor
       Meditate at its shrine and give its mantra to begin.
       Gruman knows its mantra.
  -  Spirituality                  ! Ruined
       The shrine lies in ruin.
       No way to restore this shrine is known.
       Mantra: Om
  -  Humility
       Meditate at its shrine and give its mantra to begin.
       Wartow knows its mantra.
Dungeons
  *  Deceit    Opened
       Its entrance has been opened.
       Word of Power: FALLAX
  -  Despise
       Learn its Word of Power.
       Annon knows the word.
  -  Destard
       Learn its Word of Power.
       Trian and Goeth in Jhelom lead to the word.
  -  Wrong
       Learn its Word of Power.
       Felespar knows the word.
  -  Covetous
       Learn its Word of Power.
       Fiona knows the word.
  -  Shame
       Learn its Word of Power.
       Sindar knows the word.
  -  Hythloth
       Learn its Word of Power.
       Hassad knows the word.
  -  Doom
       Learn its Word of Power.
       No one is known to teach this word.
Shadowlords
  *  Faulinei   Vanquished    Shard destroyed
       Its shard has been destroyed.
       Flame: Flame of Truth
  -  Astaroth                 Shard carried
       Its shard is carried.
       Its name summons it.
       A shard is destroyed in an Eternal Flame while its Shadowlord stands there.
  -  Nosfentor
       Its name summons it.
       A shard is destroyed in an Eternal Flame while its Shadowlord stands there.
Regalia
  *  Crown           Carried
  -  Sceptre
       Sir Simon asks for the Crown, the Sceptre and the Amulet.
       Froed starts the trail to it.
  -  Amulet
       Sir Simon asks for the Crown, the Sceptre and the Amulet.
       Lady Tessa knows where it lies.
       It lets the party pass through unholy darkness.
  -  Sandalwood Box
       Saduj speaks of a hidden box in Lord British's chamber.
Items
  *  Grapple  Carried
People
     Avatar                  In the party
     Iolo                    In the party
     Shamino                 Lodged
       Waiting at the inn in Britain.
  !  Dupre                   Executed by Blackthorn
     Introduced yourself to  2 people
     Moonglow                2
     Karma                   42
Endgame
  -  All three Shadowlords vanquished
       Doom admits the party only once all three are vanquished.
  -  The Doom entrance opened
       The final room lies at the bottom of Doom.
       The Amulet helps the party through unholy darkness.
  -  The Sandalwood Box carried
       The last scene asks for it.
";

#[test]
fn from_play_options_equals_from_play_state_after_load_scene() {
    let bytes = synthetic_save();
    let options =
        crate::play_options_from_save_bytes_named(&bytes, "SAVED.GAM", "test", false).unwrap();
    let from_options = QuestSnapshot::from_play_options(&options);
    let game_dir = debug_game_dir();
    let state = PlayState::load_scene(&game_dir, options).unwrap();
    assert_eq!(QuestSnapshot::from_play_state(&state), from_options);
    assert_eq!(from_options, golden());
    let _ = std::fs::remove_dir_all(game_dir);
}

#[test]
fn any_save_bytes_never_panic() {
    let mut rng = XorShift(0x5EED_0005_0005);
    let base = synthetic_save();
    for round in 0..300 {
        let mut bytes = if round % 3 == 0 {
            (0..SAVED_GAM_LEN).map(|_| rng.byte()).collect()
        } else {
            let mut bytes = base.clone();
            for _ in 0..64 {
                let at = (rng.next() % SAVED_GAM_LEN as u64) as usize;
                bytes[at] = rng.byte();
            }
            bytes
        };
        bytes[SAVE_AVATAR_NAME_OFFSET] = b'A';
        if let Ok(snap) = QuestSnapshot::from_save_bytes(&bytes) {
            for tier in SpoilerTier::ALL {
                let model = derive(&snap, tier);
                assert!(model.section(SectionId::Virtues).is_some());
                assert!(model.section(SectionId::People).is_some());
            }
        }
    }
    // Every byte of every snapshot field, one field at a time.
    for byte in 0..=255u8 {
        let mut snap = fresh();
        snap.ordained_mask = byte;
        snap.codex_mask = byte.rotate_left(3);
        snap.ruin = [byte; 8];
        snap.seals = [byte; 8];
        snap.shadowlords = [ShadowlordState::from_slot(byte); 3];
        snap.special_items = [byte; SPECIAL_ITEM_COUNT];
        snap.climbing_gear = byte;
        snap.moral_standing = byte;
        snap.introduced = vec![(byte, byte)];
        snap.current_scene = byte;
        snap.companions.push(CompanionFact {
            name: decode_name(&[byte; 8]),
            place: CompanionPlace::Lodged { scene: byte },
        });
        for tier in SpoilerTier::ALL {
            let _ = derive(&snap, tier);
        }
    }
}

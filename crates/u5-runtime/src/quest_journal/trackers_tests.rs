//! The quest trackers (design 8.6; build plan M6-06): T12 evidence on
//! synthetic save images, T13's tracker part (monotone, nothing named at
//! tier 2 without evidence, no denominator below tier 4, no spec link
//! below tier 3), T14 the tier 4 gate, the ids and the order.

use super::*;
use crate::enhancements::clues::{
    ClueEntry, DialogKey, GameMinute, Link, NpcKey, RecordAddr, TestBits,
};
use crate::test_fixtures::saved_game_seed_bytes;
use crate::{
    SAVE_AVATAR_NAME_OFFSET, SAVE_CLIMBING_GEAR_OFFSET, SAVE_EQUIPMENT_INVENTORY_OFFSET,
    SAVE_SHADOWLORD_HIDEOUTS_OFFSET, SAVE_SHRINE_CODEX_MASK_OFFSET,
    SAVE_SHRINE_ORDAINED_MASK_OFFSET, SAVE_SHRINE_RUIN_FLAGS_OFFSET, SAVE_SPECIAL_ITEM_OFFSET,
    SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET,
};

const TIERS: [SpoilerTier; 4] = SpoilerTier::ALL;

/// A synthetic SAVED.GAM: the Avatar alone, nothing done.
fn blank_save() -> Vec<u8> {
    let mut bytes = saved_game_seed_bytes(0, 0, 10, 10);
    bytes[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 6].copy_from_slice(b"Avatar");
    bytes
}

fn snap_of(bytes: &[u8]) -> QuestSnapshot {
    QuestSnapshot::from_save_bytes(bytes).unwrap()
}

fn view(snap: &QuestSnapshot, tier: SpoilerTier) -> TrackerView {
    derive_trackers(
        snap,
        None,
        &EdgeTable::EMPTY,
        tier,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    )
}

fn with_log(snap: &QuestSnapshot, log: &ClueLog, tier: SpoilerTier) -> TrackerView {
    derive_trackers(
        snap,
        Some(log),
        &EdgeTable::EMPTY,
        tier,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    )
}

fn quests(view: &TrackerView) -> Vec<QuestId> {
    view.cards.iter().map(|card| card.quest).collect()
}

fn shrine(virtue: ShrineVirtue) -> QuestId {
    QuestId::new(QuestKind::Shrine, virtue.index() as u8)
}

fn entry(at: u32) -> ClueEntry {
    ClueEntry::heard(
        NpcKey { scene: 1, slot: 2 },
        DialogKey {
            family: 0,
            dialog_id: 3,
        },
        RecordAddr::Pair(1),
        0x1234,
        TestBits::default(),
        GameMinute(at),
    )
}

fn log_with(links: &[Link], known: &[u16]) -> ClueLog {
    let mut log = ClueLog::new();
    log.entries = vec![entry(100), entry(200)];
    log.links = links.to_vec();
    log.known = known.to_vec();
    log
}

// --- Ids and table. ------------------------------------------------------

#[test]
fn quest_and_step_codes_round_trip() {
    for chain in QUEST_CHAINS {
        for quest in chain.quests() {
            assert_eq!(QuestId::from_code(quest.code()), Some(quest));
            for spec in chain.steps {
                let step = StepId::new(quest, spec.step);
                assert_eq!(StepId::from_code(step.code()), Some(step));
            }
        }
    }
    assert_eq!(QuestId::from_code(0xF0), None);
}

#[test]
fn every_chain_row_cites_the_spec() {
    let kinds: Vec<QuestKind> = QUEST_CHAINS.iter().map(|chain| chain.kind).collect();
    assert_eq!(kinds, QuestKind::ALL);
    for chain in QUEST_CHAINS {
        assert!(!chain.spec_ref.is_empty(), "{:?}", chain.kind);
        assert!(
            chain.spec_ref.contains(".md §"),
            "{:?}: {}",
            chain.kind,
            chain.spec_ref
        );
        assert!(!chain.steps.is_empty());
        for (at, spec) in chain.steps.iter().enumerate() {
            assert_eq!(usize::from(spec.step), at);
        }
    }
}

// --- T12 evidence. -------------------------------------------------------

#[test]
fn t12_the_offsets_are_the_design_table() {
    assert_eq!(SAVE_SHRINE_ORDAINED_MASK_OFFSET, 0x0326);
    assert_eq!(SAVE_SHRINE_CODEX_MASK_OFFSET, 0x0328);
    assert_eq!(SAVE_SHRINE_RUIN_FLAGS_OFFSET, 0x0332);
    assert_eq!(SAVE_WORD_OF_POWER_SEAL_FLAGS_OFFSET, 0x032A);
    assert_eq!(SAVE_SHADOWLORD_HIDEOUTS_OFFSET, 0x0322);
    assert_eq!(SAVE_SPECIAL_ITEM_OFFSET, 0x020A);
    assert_eq!(SAVE_CLIMBING_GEAR_OFFSET, 0x0209);
    assert_eq!(SAVE_EQUIPMENT_INVENTORY_OFFSET + 39, 0x0241);
}

#[test]
fn t12_the_blank_save_has_no_card_below_tier_3() {
    let snap = snap_of(&blank_save());
    for tier in [SpoilerTier::Progress, SpoilerTier::Goals] {
        let view = view(&snap, tier);
        assert!(view.cards.is_empty(), "{tier:?}: {:?}", quests(&view));
        assert!(view.clue_log_off);
    }
    // Tier 3 reaches the rest of the table: every shrine, dungeon,
    // Shadowlord, the regalia, the box, Doom and the passwords.
    let hints = view(&snap, SpoilerTier::Hints);
    assert_eq!(hints.cards.len(), 8 + 8 + 3 + 1 + 1 + 1 + 2);
    assert!(hints.spoilers);
}

#[test]
fn t12_shrine_masks_give_each_phase() {
    for (ordained, codex, state, mark) in [
        (1, 0, "Ordained", StatusMark::ORDAINED),
        (1, 1, "Codex read", StatusMark::CODEX_READ),
        (0, 1, "Complete", StatusMark::DONE),
    ] {
        for virtue in ShrineVirtue::ALL {
            let bit = 1 << virtue.index();
            let mut bytes = blank_save();
            bytes[0x0326] = if ordained == 1 { bit } else { 0 };
            bytes[0x0328] = if codex == 1 { bit } else { 0 };
            let view = view(&snap_of(&bytes), SpoilerTier::Goals);
            assert_eq!(quests(&view), [shrine(virtue)]);
            let card = &view.cards[0];
            assert_eq!(card.state.as_str(), state);
            assert_eq!(card.mark, mark);
            assert!(card.boxed);
        }
    }
}

#[test]
fn t12_a_ruin_byte_gives_a_ruined_card() {
    for virtue in ShrineVirtue::ALL {
        let mut bytes = blank_save();
        bytes[0x0332 + virtue.index()] = 0x80;
        let view = view(&snap_of(&bytes), SpoilerTier::Goals);
        assert_eq!(quests(&view), [shrine(virtue)]);
        assert_eq!(view.cards[0].state.as_str(), text::RUINED);
        assert_eq!(view.cards[0].mark, StatusMark::ALERT);
        // Tier 2 names no step of a ruined shrine not begun.
        assert!(view.cards[0].steps.is_empty());
        assert_eq!(view.cards[0].next, [JournalText::from(text::RUIN_LINE)]);
    }
}

#[test]
fn t12_a_seal_toggle_shows_its_current_state() {
    for record in 0..8u8 {
        let mut bytes = blank_save();
        bytes[0x032A + usize::from(record)] = 0x80;
        let opened = view(&snap_of(&bytes), SpoilerTier::Goals);
        let quest = QuestId::new(QuestKind::WordOfPower, record);
        assert_eq!(quests(&opened), [quest]);
        assert_eq!(opened.cards[0].state.as_str(), text::UNSEALED);
        assert!(
            opened
                .strings()
                .all(|line| !line.to_ascii_lowercase().contains("complete"))
        );
        // Speaking the word again re-seals: tier 2 drops the card (no
        // evidence left), tier 3 shows the current state.
        bytes[0x032A + usize::from(record)] = 0x00;
        let snap = snap_of(&bytes);
        assert!(view(&snap, SpoilerTier::Goals).cards.is_empty());
        let hints = view(&snap, SpoilerTier::Hints);
        assert_eq!(
            hints.card(quest).unwrap().state.as_str(),
            text::SEALED_STATE
        );
    }
}

#[test]
fn t12_shadowlord_slots_ff_is_vanquished_and_no_other_value_shows() {
    for slot in 0..3u8 {
        for value in [0u8, 1, 4, 8, 9, 0x7F, 0xFE] {
            let mut bytes = blank_save();
            bytes[0x0322 + usize::from(slot)] = value;
            let Ok(snap) = QuestSnapshot::from_save_bytes(&bytes) else {
                continue;
            };
            assert!(view(&snap, SpoilerTier::Goals).cards.is_empty(), "{value}");
        }
        let mut bytes = blank_save();
        bytes[0x0322 + usize::from(slot)] = 0xFF;
        let view = view(&snap_of(&bytes), SpoilerTier::Goals);
        assert_eq!(quests(&view), [QuestId::new(QuestKind::Shadowlord, slot)]);
        assert_eq!(view.cards[0].state.as_str(), text::VANQUISHED);
    }
}

#[test]
fn t12_items_show_only_while_carried() {
    let shards = QuestId::new(QuestKind::Shard, 0);
    let mut bytes = blank_save();
    bytes[0x0210] = 1;
    bytes[0x0212] = 1;
    let snap = snap_of(&bytes);
    let card = view(&snap, SpoilerTier::Goals);
    assert_eq!(card.card(shards).unwrap().state.as_str(), "2");

    for (offset, kind) in [
        (0x020D, QuestKind::Regalia),
        (0x020E, QuestKind::Regalia),
        (0x020F, QuestKind::Regalia),
        (0x0219, QuestKind::SandalwoodBox),
        (0x0209, QuestKind::Tools),
        (0x020A, QuestKind::Tools),
        (0x020B, QuestKind::Tools),
        (0x0214, QuestKind::Tools),
        (0x0216, QuestKind::Tools),
        (0x0218, QuestKind::Tools),
        (0x0241, QuestKind::Tools),
    ] {
        let mut bytes = blank_save();
        bytes[offset] = 1;
        let view = view(&snap_of(&bytes), SpoilerTier::Goals);
        assert_eq!(
            view.cards
                .iter()
                .map(|card| card.quest.kind)
                .collect::<Vec<_>>(),
            [kind],
            "{offset:#06x}"
        );
        assert!(!view.cards[0].boxed);
    }
    let mut bytes = blank_save();
    bytes[0x0241] = 1;
    let snap = snap_of(&bytes);
    assert_eq!(snap.glass_sword, 1);
    let tools = view(&snap, SpoilerTier::Full);
    let tools = tools.card(QuestId::new(QuestKind::Tools, 0)).unwrap();
    assert_eq!(tools.notes, [JournalText::from(text::GLASS_SWORD)]);
}

#[test]
fn t12_doom_readiness_is_the_conjunction_at_tier_3() {
    let doom = QuestId::new(QuestKind::Doom, 0);
    for (seal, slots, ready) in [(0x80, 0xFF, true), (0x00, 0xFF, false), (0x80, 0x03, false)] {
        let mut bytes = blank_save();
        bytes[0x0331] = seal;
        bytes[0x0322] = 0xFF;
        bytes[0x0323] = 0xFF;
        bytes[0x0324] = slots;
        let snap = snap_of(&bytes);
        assert_eq!(doom_ready(&snap), ready);
        assert!(view(&snap, SpoilerTier::Goals).card(doom).is_none());
        let card = view(&snap, SpoilerTier::Hints);
        let card = card.card(doom).unwrap();
        let want = if ready {
            text::DOOM_READY
        } else {
            text::DOOM_NOT_READY
        };
        assert_eq!(card.state.as_str(), want);
    }
}

// --- T13 tracker part. ---------------------------------------------------

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

    fn pick(&mut self, usual: &[u8]) -> u8 {
        usual[(self.next() % usual.len() as u64) as usize]
    }
}

fn random_save(rng: &mut XorShift) -> Vec<u8> {
    let mut bytes = blank_save();
    bytes[0x0326] = rng.next() as u8;
    bytes[0x0328] = rng.next() as u8;
    for at in 0..8 {
        bytes[0x0332 + at] = rng.pick(&[0, 0, 0, 0x80, 0x01]);
        bytes[0x032A + at] = rng.pick(&[0, 0, 0x80]);
    }
    for at in 0..3 {
        bytes[0x0322 + at] = rng.pick(&[0, 3, 0xFF]);
    }
    for at in 0x0209..=0x0219 {
        bytes[at] = rng.pick(&[0, 0, 0, 1]);
    }
    bytes[0x0241] = rng.pick(&[0, 1]);
    bytes
}

fn random_log(rng: &mut XorShift) -> ClueLog {
    let mut links = Vec::new();
    let mut known = Vec::new();
    for _ in 0..(rng.next() % 4) {
        let chain = &QUEST_CHAINS[(rng.next() % QUEST_CHAINS.len() as u64) as usize];
        let quest = QuestId::new(chain.kind, (rng.next() % u64::from(chain.count)) as u8);
        let step = StepId::new(quest, (rng.next() % chain.steps.len() as u64) as u8);
        match rng.next() % 3 {
            0 => links.push(Link {
                entry: (rng.next() % 2) as u16,
                kind: 0,
                target: quest.code(),
            }),
            1 => links.push(Link {
                entry: (rng.next() % 2) as u16,
                kind: 1,
                target: step.code(),
            }),
            _ => known.push(step.code()),
        }
    }
    log_with(&links, &known)
}

#[test]
fn t13_derive_trackers_is_monotone_in_the_tier() {
    let mut rng = XorShift(0x5EED_6006);
    for _ in 0..300 {
        let snap = snap_of(&random_save(&mut rng));
        let log = random_log(&mut rng);
        for logged in [None, Some(&log)] {
            let views: Vec<TrackerView> = TIERS
                .iter()
                .map(|tier| {
                    derive_trackers(
                        &snap,
                        logged,
                        &EdgeTable::EMPTY,
                        *tier,
                        TrackerEvidence::Record,
                        &TrackerActivity::default(),
                    )
                })
                .collect();
            for pair in views.windows(2) {
                let (low, high) = (&pair[0], &pair[1]);
                for card in &low.cards {
                    let up = high
                        .card(card.quest)
                        .unwrap_or_else(|| panic!("{:?} lost at {:?}", card.quest, high.tier));
                    assert_eq!(card.title, up.title);
                    let steps: Vec<StepId> = up.steps.iter().map(|step| step.step).collect();
                    let mut from = 0;
                    for step in &card.steps {
                        let at = steps[from..]
                            .iter()
                            .position(|id| *id == step.step)
                            .unwrap_or_else(|| panic!("{:?} lost", step.step));
                        from += at + 1;
                    }
                    assert!(up.next.starts_with(&card.next), "{:?}", card.quest);
                    if low.tier < SpoilerTier::Full {
                        assert!(card.answer.is_none());
                    }
                }
            }
        }
    }
}

/// "2/8", "0/3", "2 of 3": a count over a total.
fn has_denominator(line: &str) -> bool {
    let words: Vec<&str> = line.split_whitespace().collect();
    line.split('/').count() > 1 && line.chars().any(|c| c.is_ascii_digit())
        || words
            .windows(3)
            .any(|w| w[1] == "of" && w[0].parse::<u32>().is_ok() && w[2].parse::<u32>().is_ok())
}

#[test]
fn t13_no_denominator_below_tier_4_and_no_step_without_evidence() {
    let mut rng = XorShift(0xD0_0D);
    for _ in 0..300 {
        let snap = snap_of(&random_save(&mut rng));
        let log = random_log(&mut rng);
        for tier in [
            SpoilerTier::Progress,
            SpoilerTier::Goals,
            SpoilerTier::Hints,
        ] {
            for logged in [None, Some(&log)] {
                let view = derive_trackers(
                    &snap,
                    logged,
                    &EdgeTable::EMPTY,
                    tier,
                    TrackerEvidence::Record,
                    &TrackerActivity::default(),
                );
                for line in view.strings() {
                    assert!(!has_denominator(line), "{tier:?}: {line}");
                }
                if tier > SpoilerTier::Goals {
                    continue;
                }
                for card in &view.cards {
                    // A card whose quest has no save evidence names no
                    // step: a pinned-only card.
                    let chain = chain(card.quest.kind);
                    let any_save = chain.steps.iter().any(|spec| match spec.evidence {
                        Evidence::Save(test) => test(&snap, card.quest.index),
                        Evidence::Knowledge => false,
                    });
                    let ruined = card.quest.kind == QuestKind::Shrine
                        && snap.ruin[usize::from(card.quest.index)] != 0;
                    let inside = card.quest.kind == QuestKind::WordOfPower;
                    if !any_save && !ruined && !inside {
                        assert!(card.steps.is_empty(), "{:?}", card.quest);
                    }
                    assert_eq!(card.heard, 0);
                }
            }
        }
    }
    // Tier 4 is the one tier with a total.
    let mut bytes = blank_save();
    bytes[0x0210] = 1;
    let full = view(&snap_of(&bytes), SpoilerTier::Full);
    let shards = full.card(QuestId::new(QuestKind::Shard, 0)).unwrap();
    assert_eq!(shards.state.as_str(), "1 of 3");
}

#[test]
fn t13_a_pinned_card_at_tier_2_names_no_step() {
    let snap = snap_of(&blank_save());
    let honesty = shrine(ShrineVirtue::Honesty);
    let log = log_with(
        &[
            Link {
                entry: 0,
                kind: 0,
                target: honesty.code(),
            },
            Link {
                entry: 1,
                kind: 0,
                target: honesty.code(),
            },
        ],
        &[],
    );
    let view = with_log(&snap, &log, SpoilerTier::Goals);
    assert_eq!(quests(&view), [honesty]);
    let card = &view.cards[0];
    assert_eq!(card.state.as_str(), "?");
    assert!(card.steps.is_empty());
    assert!(card.next.is_empty());
    assert_eq!(card.pinned, 2);
    assert!(!view.clue_log_off);
    // The pin's hearing orders the card.
    assert_eq!(card.recency, Some(GameMinute(200)));
}

#[test]
fn t13_no_spec_link_is_shown_or_counted_below_tier_3() {
    let honesty = shrine(ShrineVirtue::Honesty);
    let edges = [ClueEdge {
        dialog: DialogKey {
            family: 0,
            dialog_id: 3,
        },
        record: Some(RecordAddr::Pair(1)),
        step: StepId::new(honesty, 0),
        level: EdgeLevel::Record,
        spec_ref: "catalogs/quest-graph.md §8",
    }];
    let table = EdgeTable { edges: &edges };
    let mut bytes = blank_save();
    bytes[0x0326] = 1;
    let snap = snap_of(&bytes);
    let log = log_with(&[], &[]);
    let at = |tier| {
        derive_trackers(
            &snap,
            Some(&log),
            &table,
            tier,
            TrackerEvidence::Record,
            &TrackerActivity::default(),
        )
    };
    let goals = at(SpoilerTier::Goals);
    let card = goals.card(honesty).unwrap();
    assert_eq!(card.heard, 0);
    assert_eq!(card.steps[0].tick, Tick::Unknown);
    let hints = at(SpoilerTier::Hints);
    let card = hints.card(honesty).unwrap();
    assert_eq!(card.heard, 2);
    assert_eq!(card.steps[0].tick, Tick::Hollow);
}

#[test]
fn t13_knowledge_steps_never_get_a_full_tick() {
    let mut bytes = blank_save();
    bytes[0x0328] = 0xFF;
    let snap = snap_of(&bytes);
    let log = log_with(&[], &[StepId::new(shrine(ShrineVirtue::Valor), 0).code()]);
    for tier in TIERS {
        for card in with_log(&snap, &log, tier).cards {
            for step in &card.steps {
                let spec = chain(card.quest.kind).steps[usize::from(step.step.step)];
                if matches!(spec.evidence, Evidence::Knowledge) {
                    assert_ne!(step.tick, Tick::Done);
                }
            }
        }
    }
    let view = with_log(&snap, &log, SpoilerTier::Goals);
    let valor = view.card(shrine(ShrineVirtue::Valor)).unwrap();
    assert_eq!(valor.steps[0].tick, Tick::Hollow);
    assert_eq!(valor.state.as_str(), "Complete");
}

#[test]
fn next_lines_follow_the_chapter_5_tiers() {
    let mut bytes = blank_save();
    bytes[0x0326] = 1 << ShrineVirtue::Compassion.index();
    let snap = snap_of(&bytes);
    let quest = shrine(ShrineVirtue::Compassion);
    assert!(
        view(&snap, SpoilerTier::Progress)
            .card(quest)
            .unwrap()
            .next
            .is_empty()
    );
    let goals = view(&snap, SpoilerTier::Goals);
    let card = goals.card(quest).unwrap();
    assert_eq!(card.cell.as_str(), "Read Codex");
    assert_eq!(
        card.next,
        [JournalText::from("The shrine sent you to read the Codex.")]
    );
    // The game gave the Codex step, so tier 2 names it, open.
    let labels: Vec<(&str, Tick)> = card
        .steps
        .iter()
        .map(|step| (step.label.as_str(), step.tick))
        .collect();
    assert_eq!(
        labels,
        [
            (text::STEP_LEARN_MANTRA, Tick::Unknown),
            (text::STEP_CHANT, Tick::Done),
            (text::STEP_READ_CODEX, Tick::Open),
        ]
    );
    // Codex read: tier 2 names no "return" step; tier 3 does.
    let mut bytes = blank_save();
    bytes[0x0326] = 1;
    bytes[0x0328] = 1;
    let snap = snap_of(&bytes);
    let honesty = shrine(ShrineVirtue::Honesty);
    let goals = view(&snap, SpoilerTier::Goals);
    let goals = goals.card(honesty).unwrap();
    assert!(goals.cell.is_empty());
    assert!(
        goals
            .steps
            .iter()
            .all(|s| s.label.as_str() != text::STEP_RETURN)
    );
    let hints = view(&snap, SpoilerTier::Hints);
    let hints = hints.card(honesty).unwrap();
    assert_eq!(hints.cell.as_str(), "Return");
    assert_eq!(hints.steps.len(), 4);
}

// --- T14 tier 4 gate. ----------------------------------------------------

#[test]
fn t14_an_answer_needs_one_of_the_evidence_sources() {
    let snap = snap_of(&blank_save());
    // Tier 4 reaches every card, but no answer without evidence.
    let full = view(&snap, SpoilerTier::Full);
    assert!(full.cards.iter().all(|card| card.answer.is_none()));

    // (a) the save shows progress past the step.
    let mut bytes = blank_save();
    bytes[0x0326] = 1;
    bytes[0x032A] = 0x80;
    bytes[0x0322] = 0xFF;
    let full = view(&snap_of(&bytes), SpoilerTier::Full);
    let answered: Vec<QuestId> = full
        .cards
        .iter()
        .filter(|card| card.answer.is_some())
        .map(|card| card.quest)
        .collect();
    assert_eq!(
        answered,
        [
            shrine(ShrineVirtue::Honesty),
            QuestId::new(QuestKind::WordOfPower, 0),
            QuestId::new(QuestKind::Shadowlord, 0),
        ]
    );
    // Never below tier 4.
    for tier in [
        SpoilerTier::Progress,
        SpoilerTier::Goals,
        SpoilerTier::Hints,
    ] {
        assert!(
            view(&snap_of(&bytes), tier)
                .cards
                .iter()
                .all(|card| card.answer.is_none())
        );
    }

    // (c) a pin on the step; a pin on the quest alone is not enough.
    let valor = shrine(ShrineVirtue::Valor);
    let quest_pin = log_with(
        &[Link {
            entry: 0,
            kind: 0,
            target: valor.code(),
        }],
        &[],
    );
    let full = with_log(&snap, &quest_pin, SpoilerTier::Full);
    assert!(full.card(valor).unwrap().answer.is_none());
    let step_pin = log_with(
        &[Link {
            entry: 0,
            kind: 1,
            target: StepId::new(valor, 0).code(),
        }],
        &[],
    );
    let full = with_log(&snap, &step_pin, SpoilerTier::Full);
    let answer = full.card(valor).unwrap().answer.unwrap();
    assert_eq!(answer.label, text::ANSWER_MANTRA);
    assert_eq!(answer.secret, shrine_mantra_for(ShrineVirtue::Valor));
    // An "I know this" mark is not one of the sources.
    let known = log_with(&[], &[StepId::new(valor, 0).code()]);
    let full = with_log(&snap, &known, SpoilerTier::Full);
    assert!(full.card(valor).unwrap().answer.is_none());

    // (b) a Record-level clue for the step; an Npc-level one only with
    // `tracker_evidence = npc`.
    let log = log_with(&[], &[]);
    for (level, evidence, answered) in [
        (EdgeLevel::Record, TrackerEvidence::Record, true),
        (EdgeLevel::Npc, TrackerEvidence::Record, false),
        (EdgeLevel::Npc, TrackerEvidence::Npc, true),
    ] {
        let edges = [ClueEdge {
            dialog: DialogKey {
                family: 0,
                dialog_id: 3,
            },
            record: (level == EdgeLevel::Record).then_some(RecordAddr::Pair(1)),
            step: StepId::new(valor, 0),
            level,
            spec_ref: "catalogs/quest-graph.md §8",
        }];
        let full = derive_trackers(
            &snap,
            Some(&log),
            &EdgeTable { edges: &edges },
            SpoilerTier::Full,
            evidence,
            &TrackerActivity::default(),
        );
        assert_eq!(
            full.card(valor).unwrap().answer.is_some(),
            answered,
            "{level:?} {evidence:?}"
        );
    }
}

// --- Order. --------------------------------------------------------------

#[test]
fn cards_order_by_recency_then_virtue_order() {
    let mut bytes = blank_save();
    bytes[0x0326] = 0b1000_0001;
    let first = snap_of(&bytes);
    let mut activity = TrackerActivity::default();
    activity.note(&first, GameMinute(10));
    let order = |snap: &QuestSnapshot, activity: &TrackerActivity| {
        quests(&derive_trackers(
            snap,
            None,
            &EdgeTable::EMPTY,
            SpoilerTier::Goals,
            TrackerEvidence::Record,
            activity,
        ))
    };
    // Nothing stamped on the first note: virtue order.
    assert_eq!(
        order(&first, &activity),
        [
            shrine(ShrineVirtue::Honesty),
            shrine(ShrineVirtue::Humility)
        ]
    );
    // Humility's Codex is read: it moves to the top.
    bytes[0x0328] = 0b1000_0000;
    let second = snap_of(&bytes);
    activity.note(&second, GameMinute(20));
    assert_eq!(
        activity.last(shrine(ShrineVirtue::Humility)),
        Some(GameMinute(20))
    );
    assert_eq!(
        order(&second, &activity),
        [
            shrine(ShrineVirtue::Humility),
            shrine(ShrineVirtue::Honesty)
        ]
    );
}

#[test]
fn the_panel_lines_stay_at_tier_2_and_fit() {
    let mut bytes = blank_save();
    bytes[0x0326] = 0b0100_0100;
    bytes[0x0210] = 1;
    bytes[0x0241] = 1;
    let lines = panel_card_lines(&snap_of(&bytes), 16);
    assert_eq!(
        lines,
        [
            "Read Codex:",
            " Valor",
            " Spirituality",
            "Shards held 1",
            "Glass Sword"
        ]
    );
    assert!(panel_card_lines(&snap_of(&blank_save()), 16).is_empty());
}

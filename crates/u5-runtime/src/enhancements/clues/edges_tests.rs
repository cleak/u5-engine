//! T15 citations (design 8.6) and the evidence links on the real table
//! (build plan M6-07).

use super::*;
use crate::enhancements::clues::{ClueEntry, ClueLog, GameMinute, NpcKey, RecordAddr, TestBits};
use crate::enhancements::journal::SpoilerTier;
use crate::quest_journal::QuestSnapshot;
use crate::quest_journal::trackers::{
    Evidence, LinkTarget, QUEST_CHAINS, TrackerActivity, chain, derive_trackers, link_lines,
    linked_entries, pin_targets,
};
use crate::test_fixtures::saved_game_seed_bytes;

/// A synthetic roster: `(location, slot, dialog id)` coordinates of
/// `catalogs/npc-roster.md` §3 for the rows the table cites. Ids only,
/// copied by hand from the spec's coordinates; no names.
const ROSTER: &[(&str, u8, u8)] = &[
    ("TOWNE:0", 3, 1),
    ("TOWNE:0", 4, 2),
    ("TOWNE:0", 5, 3),
    ("TOWNE:1", 5, 8),
    ("TOWNE:1", 6, 9),
    ("TOWNE:1", 8, 11),
    ("TOWNE:1", 13, 12),
    ("TOWNE:1", 14, 6),
    ("TOWNE:2", 8, 14),
    ("TOWNE:2", 9, 15),
    ("TOWNE:2", 10, 16),
    ("TOWNE:3", 10, 22),
    ("TOWNE:3", 13, 18),
    ("TOWNE:3", 14, 17),
    ("TOWNE:3", 19, 36),
    ("TOWNE:3", 21, 35),
    ("TOWNE:4", 3, 26),
    ("TOWNE:5", 5, 32),
    ("TOWNE:5", 9, 34),
    ("TOWNE:6", 4, 37),
    ("TOWNE:6", 5, 38),
    ("TOWNE:6", 6, 39),
    ("TOWNE:7", 6, 46),
    ("TOWNE:7", 7, 47),
    ("TOWNE:7", 8, 48),
    ("DWELLING:2", 3, 7),
    ("DWELLING:4", 4, 13),
    ("DWELLING:6", 1, 11),
    ("CASTLE:0", 18, 3),
    ("CASTLE:0", 26, 7),
    ("CASTLE:1", 5, 10),
    ("CASTLE:1", 31, 17),
    ("CASTLE:3", 2, 21),
    ("CASTLE:5", 6, 30),
    ("CASTLE:6", 3, 31),
    ("CASTLE:6", 4, 32),
    ("CASTLE:7", 7, 35),
    ("CASTLE:7", 8, 36),
    ("CASTLE:7", 9, 37),
    ("CASTLE:7", 12, 40),
    ("KEEP:0", 1, 1),
    ("KEEP:1", 1, 2),
    ("KEEP:1", 2, 3),
    ("KEEP:1", 4, 30),
    ("KEEP:1", 5, 31),
    ("KEEP:2", 1, 4),
    ("KEEP:2", 3, 6),
    ("KEEP:3", 2, 9),
    ("KEEP:5", 5, 11),
    ("KEEP:5", 6, 12),
    ("KEEP:5", 19, 27),
    ("KEEP:6", 10, 16),
    ("KEEP:6", 15, 28),
    ("KEEP:6", 16, 32),
    ("KEEP:7", 11, 22),
    ("KEEP:7", 16, 23),
    ("KEEP:7", 17, 24),
    ("KEEP:7", 18, 25),
    ("KEEP:7", 19, 29),
];

/// The family of a roster location (`TOWNE:3` is family 0).
fn family_of(location: &str) -> u8 {
    match location.split(':').next() {
        Some("TOWNE") => TOWNE,
        Some("DWELLING") => DWELLING,
        Some("CASTLE") => CASTLE,
        Some("KEEP") => KEEP,
        other => panic!("not a roster family: {other:?}"),
    }
}

/// The roster coordinate an edge cites: `npc-roster.md §3 <LOC> slot <n>`.
fn coordinate(spec_ref: &str) -> (&str, u8) {
    let rest = spec_ref
        .split("catalogs/npc-roster.md §3 ")
        .nth(1)
        .unwrap_or_else(|| panic!("no roster coordinate in {spec_ref:?}"));
    let mut words = rest.split_whitespace();
    let location = words.next().unwrap();
    assert_eq!(words.next(), Some("slot"), "{spec_ref}");
    (location, words.next().unwrap().parse().unwrap())
}

#[test]
fn t15_every_chain_row_and_edge_cites_the_spec() {
    for chain in QUEST_CHAINS {
        assert!(chain.spec_ref.contains("catalogs/") || chain.spec_ref.contains("formats/"));
    }
    for edge in all_edges() {
        let source = edge.spec_ref.split(';').next().unwrap();
        let quest_graph = ["§3", "§4", "§5", "§6", "§7", "§8", "§9", "§10"]
            .iter()
            .any(|section| source == format!("catalogs/quest-graph.md {section}"));
        assert!(
            quest_graph || source.starts_with("catalogs/sage-rumours.md §4 row "),
            "{edge:?}"
        );
    }
}

#[test]
fn t15_every_dialog_is_in_the_roster_fixture() {
    for edge in all_edges() {
        let (location, slot) = coordinate(edge.spec_ref);
        let row = ROSTER
            .iter()
            .find(|(at, at_slot, _)| *at == location && *at_slot == slot)
            .unwrap_or_else(|| panic!("{location} slot {slot} not in the fixture"));
        assert_eq!(edge.dialog.family, family_of(location), "{edge:?}");
        assert_eq!(edge.dialog.dialog_id, row.2, "{edge:?}");
    }
}

#[test]
fn t15_every_edge_points_at_a_step_of_the_table() {
    for edge in all_edges() {
        let chain = chain(edge.step.quest.kind);
        assert_eq!(chain.kind, edge.step.quest.kind);
        assert!(edge.step.quest.index < chain.count, "{edge:?}");
        assert!(
            chain.steps.iter().any(|spec| spec.step == edge.step.step),
            "{edge:?}"
        );
        // Npc-level rows hold no record; Record-level rows wait on S8.3.
        assert_eq!(edge.record.is_none(), edge.level == EdgeLevel::Npc);
    }
    assert!(RECORD_EDGES.is_empty(), "Record-level rows need S8.3");
    assert!(edge_table(TrackerEvidence::Record).edges.is_empty());
    assert_eq!(
        edge_table(TrackerEvidence::Npc).edges.len(),
        NPC_EDGES.len()
    );
}

#[test]
fn t15_conflicts_are_reported_with_both_references() {
    let found = conflicts();
    let hythloth = StepId::new(QuestId::new(WordOfPower, 6), 0);
    let crown = StepId::new(QuestId::new(Regalia, 0), 0);
    assert_eq!(
        found.iter().map(|(step, _)| *step).collect::<Vec<_>>(),
        [hythloth, crown]
    );
    for (_, refs) in &found {
        assert_eq!(refs.len(), 2);
        assert!(
            refs.iter()
                .any(|r| r.starts_with("catalogs/quest-graph.md"))
        );
        assert!(
            refs.iter()
                .any(|r| r.starts_with("catalogs/sage-rumours.md"))
        );
    }
}

#[test]
fn the_table_holds_no_text() {
    // Ids and spec paths only: no keyword, name or answer.
    for edge in all_edges() {
        assert!(
            edge.spec_ref
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || " /.;:-§_".contains(ch)),
            "{edge:?}"
        );
        assert!(!edge.spec_ref.contains("row ") || edge.spec_ref.contains("sage-rumours"));
    }
}

// --- The table in the tracker. --------------------------------------------

fn snap() -> QuestSnapshot {
    let mut bytes = saved_game_seed_bytes(0, 0, 10, 10);
    bytes[crate::SAVE_AVATAR_NAME_OFFSET..crate::SAVE_AVATAR_NAME_OFFSET + 6]
        .copy_from_slice(b"Avatar");
    QuestSnapshot::from_save_bytes(&bytes).unwrap()
}

fn heard(scene: u8, slot: u8, family: u8, dialog_id: u8, pair: u8) -> ClueEntry {
    ClueEntry::heard(
        NpcKey { scene, slot },
        DialogKey { family, dialog_id },
        RecordAddr::Pair(pair),
        0,
        TestBits::default(),
        GameMinute(100),
    )
}

/// The Valor mantra's source (quest-graph §8), heard on two topics, and
/// one other source.
fn log() -> ClueLog {
    let mut log = ClueLog::new();
    log.fold(heard(3, 10, TOWNE, 16, 0));
    log.fold(heard(3, 10, TOWNE, 16, 1));
    log.fold(heard(1, 4, TOWNE, 2, 0));
    log
}

fn valor() -> QuestId {
    QuestId::new(Shrine, 2)
}

#[test]
fn npc_level_counts_count_sources_not_records() {
    let log = log();
    let edges = edge_table(TrackerEvidence::Npc);
    let view = derive_trackers(
        &snap(),
        Some(&log),
        &edges,
        SpoilerTier::Hints,
        TrackerEvidence::Npc,
        &TrackerActivity::default(),
    );
    // Two topics from one source: one heard.
    assert_eq!(view.card(valor()).unwrap().heard, 1);
    assert_eq!(view.card(QuestId::new(WordOfPower, 0)).unwrap().heard, 1);
    // Under `record` the Npc-level rows do not count.
    let view = derive_trackers(
        &snap(),
        Some(&log),
        &edge_table(TrackerEvidence::Record),
        SpoilerTier::Hints,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    );
    assert_eq!(view.card(valor()).unwrap().heard, 0);
}

#[test]
fn no_spec_link_is_shown_or_counted_below_tier_3() {
    let log = log();
    let edges = edge_table(TrackerEvidence::Npc);
    for tier in [SpoilerTier::Progress, SpoilerTier::Goals] {
        let view = derive_trackers(
            &snap(),
            Some(&log),
            &edges,
            tier,
            TrackerEvidence::Npc,
            &TrackerActivity::default(),
        );
        // No card from a spec link alone, and no count.
        assert!(view.card(valor()).is_none(), "{tier:?}");
        assert!(view.cards.iter().all(|card| card.heard == 0));
        // The Quest filter lists no spec-linked clue, and the detail no
        // link line.
        assert!(
            linked_entries(
                &log,
                &edges,
                LinkTarget::quest(valor()),
                tier,
                TrackerEvidence::Npc
            )
            .is_empty()
        );
        for index in 0..log.entries().len() {
            assert!(link_lines(&log, &edges, &view, index, TrackerEvidence::Npc).is_empty());
        }
    }
    let view = derive_trackers(
        &snap(),
        Some(&log),
        &edges,
        SpoilerTier::Hints,
        TrackerEvidence::Npc,
        &TrackerActivity::default(),
    );
    assert_eq!(
        linked_entries(
            &log,
            &edges,
            LinkTarget::step(StepId::new(valor(), 0)),
            SpoilerTier::Hints,
            TrackerEvidence::Npc
        ),
        [0, 1]
    );
    assert_eq!(
        link_lines(&log, &edges, &view, 0, TrackerEvidence::Npc),
        [crate::quest_journal::text::LINK_NPC]
    );
}

#[test]
fn a_pin_unlocks_the_tier_4_cell_and_shows_at_every_tier() {
    let mut log = log();
    let edges = edge_table(TrackerEvidence::Record);
    let derive = |log: &ClueLog, tier| {
        derive_trackers(
            &snap(),
            Some(log),
            &edges,
            tier,
            TrackerEvidence::Record,
            &TrackerActivity::default(),
        )
    };
    let full = derive(&log, SpoilerTier::Full);
    assert!(full.card(valor()).unwrap().answer.is_none());
    // D8.6: the player's pin on the step unlocks the cell.
    let step = LinkTarget::step(StepId::new(valor(), 0));
    log.add_link(step.link(2));
    assert!(
        derive(&log, SpoilerTier::Full)
            .card(valor())
            .unwrap()
            .answer
            .is_some()
    );
    // "pinned by you" at every tier; the Quest filter keeps the pin.
    for tier in SpoilerTier::ALL {
        let view = derive(&log, tier);
        assert_eq!(view.card(valor()).unwrap().pinned, 1, "{tier:?}");
        assert_eq!(
            linked_entries(&log, &edges, step, tier, TrackerEvidence::Record),
            [2]
        );
        assert_eq!(
            link_lines(&log, &edges, &view, 2, TrackerEvidence::Record),
            ["Pinned to: Valor"]
        );
    }
    // The card target keeps its steps' pins; another step's does not.
    assert_eq!(
        linked_entries(
            &log,
            &edges,
            LinkTarget::quest(valor()),
            SpoilerTier::Goals,
            TrackerEvidence::Record
        ),
        [2]
    );
    assert!(
        linked_entries(
            &log,
            &edges,
            LinkTarget::step(StepId::new(valor(), 1)),
            SpoilerTier::Goals,
            TrackerEvidence::Record
        )
        .is_empty()
    );
    log.remove_link(step.link(2));
    assert!(
        derive(&log, SpoilerTier::Full)
            .card(valor())
            .unwrap()
            .answer
            .is_none()
    );
}

#[test]
fn the_picker_offers_only_what_the_tier_shows() {
    let log = log();
    let view = derive_trackers(
        &snap(),
        Some(&log),
        &EdgeTable::EMPTY,
        SpoilerTier::Goals,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    );
    let rows = pin_targets(&view, &log, 0);
    // A blank save at tier 2: the eight shrines and the two passwords,
    // no step, nothing linked.
    assert_eq!(rows.len(), 10);
    assert!(rows.iter().all(|row| row.depth == 0 && !row.linked));
    assert!(
        rows.iter()
            .all(|row| matches!(row.target.quest.kind, Shrine | Password))
    );
    // A password's role is a hint (U17): the cards are numbered only.
    let passwords: Vec<&str> = rows
        .iter()
        .filter(|row| row.target.quest.kind == Password)
        .map(|row| row.label.as_str())
        .collect();
    assert_eq!(passwords, ["Password 1", "Password 2"]);
    // At tier 4 every card, with its Knowledge steps only.
    let view = derive_trackers(
        &snap(),
        Some(&log),
        &EdgeTable::EMPTY,
        SpoilerTier::Full,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    );
    let rows = pin_targets(&view, &log, 0);
    assert!(rows.iter().any(|row| row.target.quest.kind == WordOfPower));
    for row in rows.iter().filter(|row| row.depth == 1) {
        let step = row.target.step.unwrap();
        let spec = chain(row.target.quest.kind)
            .steps
            .iter()
            .find(|spec| spec.step == step)
            .unwrap();
        assert!(matches!(spec.evidence, Evidence::Knowledge));
    }
}

#[test]
fn a_known_mark_gives_a_hollow_tick_and_clears() {
    let mut log = log();
    let word = QuestId::new(WordOfPower, 0);
    let step = StepId::new(word, 0);
    log.mark_known(step.code());
    let view = derive_trackers(
        &snap(),
        Some(&log),
        &EdgeTable::EMPTY,
        SpoilerTier::Hints,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    );
    let tick = view.card(word).unwrap().steps[0].tick;
    assert_eq!(tick, crate::quest_journal::trackers::Tick::Hollow);
    log.unmark_known(step.code());
    assert!(log.known().is_empty());
}

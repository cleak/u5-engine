//! T13b, the fresh-save no-spoiler test of chapter 8 (foundation 1.4.7,
//! design 8.6; build plan M6-08).
//!
//! From the fresh save with no sidecar and `clues`, `quest_tracker` and
//! highlighting on at tier 2, the Clues tab (both views), the Topics tab,
//! the Quests tab and the side panel's lines name no person, town,
//! keyword, dungeon, Shadowlord, item or tool, show no total, and there is
//! no card. Then one event at a time (a NAME turn, one topic asked,
//! ordination, one seal, one item carried) adds only its own name or line.
//! The `SPOILERS` label of `new` and `clue_misses` is the shell's strip
//! (`u5-bevy`'s journal spoiler tests).
//!
//! Every blob is synthetic ([`crate::enhancements::clues::fixtures`]).

use std::collections::{BTreeSet, HashMap};

use super::clues::{
    ClueContext, ClueQuery, ClueView, CluesTab, CluesView, GroupKey, RowKind, derive_clues,
    panel_clue_lines,
};
use super::trackers::{QuestKind, TrackerActivity, TrackerView, derive_trackers, panel_card_lines};
use super::*;
use crate::enhancements::clues::edges::edge_table;
use crate::enhancements::clues::fixtures::{blob, text};
use crate::enhancements::clues::{
    ClueEntry, ClueLog, DialogKey, GameMinute, NpcKey, RecordAddr, TestBits,
};
use crate::enhancements::journal::TrackerEvidence;
use crate::test_fixtures::{journal_undiscovered_names, saved_game_seed_bytes};
use crate::{SAVE_AVATAR_NAME_OFFSET, SPECIAL_ITEM_CROWN_LB_INDEX};

const TIER: SpoilerTier = SpoilerTier::Goals;
/// A town scene (the gazetteer's fourth).
const TOWN: u8 = 4;
const PANEL_COLUMNS: usize = 16;
/// The invented person's name and keyword.
const PERSON: &str = "Vorpelin";
const KEYWORD: &str = "glimmer";

fn dialog() -> DialogKey {
    DialogKey {
        family: 0,
        dialog_id: 1,
    }
}

fn person() -> NpcKey {
    NpcKey {
        scene: TOWN,
        slot: 3,
    }
}

fn source() -> HashMap<DialogKey, Vec<Vec<u8>>> {
    HashMap::from([(dialog(), blob(&[(KEYWORD, text("It glows."))], &[]))])
}

fn now() -> GameMinute {
    GameMinute::from_calendar(139, 4, 1, 9, 0)
}

/// The fresh save: the factory seed, the Avatar alone.
fn fresh() -> QuestSnapshot {
    let mut bytes = saved_game_seed_bytes(0, 0, 10, 10);
    bytes[SAVE_AVATAR_NAME_OFFSET..SAVE_AVATAR_NAME_OFFSET + 6].copy_from_slice(b"Avatar");
    QuestSnapshot::from_save_bytes(&bytes).unwrap()
}

/// The party in the town, nobody introduced, at tier 2.
fn ctx() -> ClueContext {
    ClueContext {
        scene: TOWN,
        now: now(),
        introduced: HashMap::new(),
        standing: 0,
        gold: 0,
        party_size: 1,
        tier: TIER,
        pins_at_top: false,
    }
}

fn groups(view: &CluesView) -> Vec<GroupKey> {
    view.rows
        .iter()
        .filter_map(|row| match &row.kind {
            RowKind::Group { key, .. } => Some(key.clone()),
            _ => None,
        })
        .collect()
}

/// A tab with every group opened, so nothing is folded away.
fn open_tab(log: &ClueLog, tab: CluesTab, view: ClueView) -> CluesView {
    let query = ClueQuery {
        tab,
        view,
        ..ClueQuery::default()
    };
    let mut open = groups(&derive_clues(log, &source(), &ctx(), &query));
    loop {
        let shown = derive_clues(
            log,
            &source(),
            &ctx(),
            &ClueQuery {
                open: &open,
                ..query
            },
        );
        let more = groups(&shown);
        if more.len() == open.len() {
            return shown;
        }
        open = more;
    }
}

fn quests(snap: &QuestSnapshot, log: &ClueLog) -> TrackerView {
    derive_trackers(
        snap,
        Some(log),
        &edge_table(TrackerEvidence::Record),
        TIER,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    )
}

/// Every string the four surfaces show for `snap` and `log`.
fn surfaces(snap: &QuestSnapshot, log: &ClueLog) -> Vec<String> {
    let mut texts = Vec::new();
    for (tab, view) in [
        (CluesTab::Clues, ClueView::People),
        (CluesTab::Clues, ClueView::Place),
        (CluesTab::Topics, ClueView::People),
    ] {
        texts.extend(open_tab(log, tab, view).strings().map(str::to_string));
    }
    texts.extend(quests(snap, log).strings().map(str::to_string));
    texts.extend(panel_clue_lines(log, &source(), &ctx(), PANEL_COLUMNS));
    texts.extend(panel_card_lines(snap, PANEL_COLUMNS));
    texts
}

/// Every name the suite forbids: the undiscovered names of chapter 5 (the
/// towns, the dungeons, the Shadowlords, the regalia, the tools, the
/// companions, the Words of Power and the mantras), and the invented
/// person and keyword.
fn forbidden() -> Vec<String> {
    let mut names = journal_undiscovered_names();
    names.push(PERSON.to_string());
    names.push(KEYWORD.to_string());
    names
}

/// The forbidden names `texts` show, ignoring case: a one-word name as a
/// whole word, a longer one anywhere.
fn named(texts: &[String]) -> BTreeSet<String> {
    let names = forbidden();
    texts
        .iter()
        .flat_map(|text| {
            let lower = text.to_ascii_lowercase();
            names
                .iter()
                .filter(move |name| {
                    let name = name.to_ascii_lowercase();
                    if name.contains(' ') {
                        lower.contains(&name)
                    } else {
                        lower
                            .split(|c: char| !c.is_ascii_alphanumeric() && c != '\'')
                            .any(|word| word == name)
                    }
                })
                .cloned()
                .collect::<Vec<_>>()
        })
        .collect()
}

/// No `N/M` total (8.2.8). "Asked of 1 person." counts what was heard,
/// never what is left, so ` of ` alone is not a total here.
fn no_total(texts: &[String]) {
    for text in texts {
        assert!(!text.contains('/'), "a total: {text}");
    }
}

fn entry(record: RecordAddr) -> ClueEntry {
    ClueEntry::heard(person(), dialog(), record, 0, TestBits::default(), now())
}

#[test]
fn t13b_the_fresh_save_names_nothing_and_shows_no_card() {
    let snap = fresh();
    let log = ClueLog::new();
    let texts = surfaces(&snap, &log);
    assert_eq!(named(&texts), BTreeSet::new(), "{texts:?}");
    no_total(&texts);
    assert!(quests(&snap, &log).cards.is_empty());
    assert!(panel_card_lines(&snap, PANEL_COLUMNS).is_empty());
    assert!(panel_clue_lines(&log, &source(), &ctx(), PANEL_COLUMNS).is_empty());
}

#[test]
fn t13b_one_event_adds_only_its_own_name_or_line() {
    let base = surfaces(&fresh(), &ClueLog::new());
    let town = super::clues::town_label(TOWN).to_string();
    type Event = Box<dyn Fn(&mut QuestSnapshot, &mut ClueLog)>;
    // (event, the names it may add, the card kind it may add)
    let events: Vec<(&str, Event, Vec<String>, Option<QuestKind>)> = vec![
        (
            "a NAME turn",
            Box::new(|_, log| {
                log.fold(entry(RecordAddr::Leading(0)));
            }),
            vec![PERSON.into(), town.clone()],
            None,
        ),
        (
            "one topic asked",
            Box::new(|_, log| {
                log.fold(entry(RecordAddr::Pair(0)));
            }),
            vec![KEYWORD.into(), town.clone()],
            None,
        ),
        (
            "ordination",
            Box::new(|snap, _| snap.ordained_mask = 0b100),
            vec![],
            Some(QuestKind::Shrine),
        ),
        (
            "one seal",
            Box::new(|snap, _| snap.seals[3] = 0x80),
            vec![],
            Some(QuestKind::WordOfPower),
        ),
        (
            "one item carried",
            Box::new(|snap, _| snap.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] = 1),
            vec![],
            Some(QuestKind::Regalia),
        ),
    ];
    for (label, apply, may_name, card) in events {
        let mut snap = fresh();
        let mut log = ClueLog::new();
        apply(&mut snap, &mut log);
        let texts = surfaces(&snap, &log);
        no_total(&texts);
        let added: Vec<String> = texts
            .iter()
            .filter(|text| !base.contains(text))
            .cloned()
            .collect();
        assert!(!added.is_empty(), "{label} adds its line");
        let view = quests(&snap, &log);
        let cards: Vec<QuestKind> = view.cards.iter().map(|card| card.quest.kind).collect();
        assert_eq!(cards, card.into_iter().collect::<Vec<_>>(), "{label}");
        // The names it shows: the ones it may add, or, for a save event,
        // at most the one its own card names.
        let shown = named(&texts);
        match view.cards.first() {
            Some(card) => {
                let own: Vec<String> = card.strings().map(str::to_string).collect();
                assert!(named(&own).len() <= 1, "{label}: {own:?}");
                assert!(shown.is_subset(&named(&own)), "{label}: {shown:?}");
            }
            None => {
                let allowed: BTreeSet<String> = may_name.iter().cloned().collect();
                assert!(shown.is_subset(&allowed), "{label}: {shown:?}");
                assert!(
                    may_name
                        .iter()
                        .take(1)
                        .all(|name| shown.contains(name.as_str())),
                    "{label} names its own word: {shown:?}"
                );
            }
        }
    }
}

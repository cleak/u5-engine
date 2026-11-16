//! `derive_clues` and the panel lines (plan M6-03; design 8.2.3-8.2.5,
//! 8.2.8, part of T13). Every blob is synthetic: invented words in the
//! `formats/tlk.md` layout.

use std::collections::HashMap;

use super::*;
use crate::enhancements::clues::fixtures::{blob, block, record, text};
use crate::enhancements::clues::{ClueEntry, ClueLog, EntryFlags, TestBits};

const YEW: u8 = 4;
const MINOC: u8 = 5;
const MOONGLOW: u8 = 1;

fn dialog(id: u8) -> DialogKey {
    DialogKey {
        family: 0,
        dialog_id: id,
    }
}

fn npc(scene: u8, slot: u8) -> NpcKey {
    NpcKey { scene, slot }
}

/// Two invented dialogs: 1 answers GLIMMER and ROOK, 2 answers GLIMMER
/// and a standing-tested OXBOW.
fn source() -> HashMap<DialogKey, Vec<Vec<u8>>> {
    let tested = record(&[&[0xFE, 50, 0x91], &text("Begone.")]);
    let head = record(&[&text("Welcome."), &[0x82]]);
    let mut second = blob(
        &[("glimmer", text("Ask the ferrier.")), ("oxbow", tested)],
        &[block(0x91, &head, &[])],
    );
    second[0] = text("Tamsk");
    second[1] = text("a stooped ferrier");
    HashMap::from([
        (
            dialog(1),
            blob(
                &[("glimmer", text("It glows.")), ("rook", text("A bird."))],
                &[],
            ),
        ),
        (dialog(2), second),
    ])
}

fn at(day: u32, minute: u32) -> GameMinute {
    GameMinute(GameMinute::from_calendar(139, 4, 1, 0, 0).0 + day * 1440 + minute)
}

fn entry(who: NpcKey, id: u8, record: RecordAddr, when: GameMinute) -> ClueEntry {
    ClueEntry::heard(who, dialog(id), record, 0, TestBits::default(), when)
}

fn ctx() -> ClueContext {
    ClueContext {
        scene: YEW,
        now: at(2, 600),
        introduced: HashMap::new(),
        standing: 10,
        gold: 0,
        party_size: 1,
        tier: SpoilerTier::Goals,
        pins_at_top: true,
    }
}

/// Vorpelin in Yew (named) asked GLIMMER and ROOK; Tamsk in Minoc (never
/// named) asked GLIMMER and OXBOW.
fn log() -> ClueLog {
    let mut log = ClueLog::new();
    let vorpelin = npc(YEW, 3);
    let tamsk = npc(MINOC, 7);
    log.fold(entry(vorpelin, 1, RecordAddr::Leading(0), at(0, 60)));
    log.fold(entry(vorpelin, 1, RecordAddr::Pair(0), at(0, 61)));
    log.fold(entry(vorpelin, 1, RecordAddr::Pair(1), at(0, 62)));
    log.fold(entry(tamsk, 2, RecordAddr::Pair(0), at(2, 500)));
    let mut oxbow = entry(tamsk, 2, RecordAddr::Pair(1), at(2, 501));
    oxbow.tests = TestBits {
        n: 1,
        pre: 0,
        post: 0,
    };
    log.fold(oxbow);
    log
}

fn every_group(view: &CluesView) -> Vec<GroupKey> {
    view.rows
        .iter()
        .filter_map(|row| match &row.kind {
            RowKind::Group { key, .. } => Some(key.clone()),
            _ => None,
        })
        .collect()
}

fn derive_open(log: &ClueLog, ctx: &ClueContext, query: ClueQuery<'_>) -> CluesView {
    // Open every group the closed view lists, then derive again.
    let mut open = every_group(&derive_clues(log, &source(), ctx, &query));
    loop {
        let view = derive_clues(
            log,
            &source(),
            ctx,
            &ClueQuery {
                open: &open,
                ..query
            },
        );
        let more = every_group(&view);
        if more.len() == open.len() {
            return view;
        }
        open = more;
    }
}

fn labels(view: &CluesView) -> Vec<(u8, String)> {
    view.rows
        .iter()
        .map(|row| (row.depth, row.label.clone()))
        .collect()
}

#[test]
fn people_view_names_only_after_a_name_turn_most_recent_first() {
    let view = derive_open(&log(), &ctx(), ClueQuery::default());
    assert_eq!(
        labels(&view),
        [
            (0, "a person in Minoc".to_string()),
            (1, "GLIMMER".to_string()),
            (1, "OXBOW".to_string()),
            (0, "Vorpelin (Yew)".to_string()),
            (1, "GLIMMER".to_string()),
            (1, "ROOK".to_string()),
        ]
    );
    // The description is read live and shown in the details only.
    assert_eq!(
        view.rows[0].detail[0],
        "a person in Minoc, a stooped ferrier"
    );
    assert_eq!(
        view.rows[1].detail[0],
        "You asked a person in Minoc about GLIMMER."
    );
    assert_eq!(view.rows[4].detail[0], "You asked Vorpelin about GLIMMER.");
    assert_eq!(view.unread, 4);
    assert_eq!(view.starred, 0);
}

#[test]
fn groups_start_closed() {
    let view = derive_clues(&log(), &source(), &ctx(), &ClueQuery::default());
    assert_eq!(view.rows.len(), 2);
    assert!(view.rows.iter().all(|row| row.depth == 0));
    assert_eq!(view.rows[0].right, "2 topics");
    assert_eq!(view.rows[0].mark, RowMark::Unread);
}

#[test]
fn the_name_known_bank_seeds_people_labelled_by_place() {
    let mut ctx = ctx();
    ctx.introduced.insert(MOONGLOW, 0b1000);
    ctx.introduced.insert(YEW, 1 << 3); // Vorpelin, already logged
    let view = derive_open(&ClueLog::new(), &ctx, ClueQuery::default());
    assert_eq!(
        labels(&view),
        [
            (0, INTRODUCED.to_string()),
            (1, "a person in Moonglow".to_string()),
            (1, "a person in Yew".to_string()),
        ]
    );
    let with_log = derive_open(&log(), &ctx, ClueQuery::default());
    assert_eq!(
        with_log.rows.last().unwrap().label,
        "a person in Moonglow",
        "a person with topics is not listed again"
    );
}

#[test]
fn place_view_lists_towns_in_gazetteer_order_and_only_known_ones() {
    let query = ClueQuery {
        view: ClueView::Place,
        ..ClueQuery::default()
    };
    let view = derive_open(&log(), &ctx(), query);
    assert_eq!(
        labels(&view),
        [
            (0, "Yew".to_string()),
            (1, "Vorpelin".to_string()),
            (2, "GLIMMER".to_string()),
            (2, "ROOK".to_string()),
            (0, "Minoc".to_string()),
            (1, "a person in Minoc".to_string()),
            (2, "GLIMMER".to_string()),
            (2, "OXBOW".to_string()),
        ]
    );
}

/// 8.2.8 and foundation 1.4.7: the gazetteer orders, it never lists.
#[test]
fn no_town_appears_without_a_logged_or_introduced_person() {
    let query = ClueQuery {
        view: ClueView::Place,
        ..ClueQuery::default()
    };
    let empty = derive_open(&ClueLog::new(), &ctx(), query);
    assert_eq!(labels(&empty), [(0, NO_CLUES.to_string())]);
    for tier in SpoilerTier::ALL {
        let mut ctx = ctx();
        ctx.tier = tier;
        ctx.introduced.insert(MOONGLOW, 1);
        let view = derive_open(&ClueLog::new(), &ctx, query);
        let towns: Vec<_> = view
            .rows
            .iter()
            .filter(|row| matches!(row.kind, RowKind::Group { .. }))
            .map(|row| row.label.as_str())
            .collect();
        assert_eq!(towns, ["Moonglow"], "tier {tier:?}");
        for scene in 2..=32 {
            if let Some(town) = crate::town_resident_name(scene) {
                assert!(
                    view.strings().all(|s| !s.contains(town)),
                    "{town} at {tier:?}"
                );
            }
        }
    }
}

#[test]
fn topics_group_by_the_exact_keyword_alphabetically() {
    let query = ClueQuery {
        tab: CluesTab::Topics,
        ..ClueQuery::default()
    };
    let view = derive_open(&log(), &ctx(), query);
    assert_eq!(
        labels(&view),
        [
            (0, "GLIMMER".to_string()),
            (1, "Vorpelin (Yew)".to_string()),
            (1, "a person in Minoc".to_string()),
            (0, "OXBOW".to_string()),
            (1, "a person in Minoc".to_string()),
            (0, "ROOK".to_string()),
            (1, "Vorpelin (Yew)".to_string()),
        ]
    );
    assert_eq!(view.rows[0].right, "2 people");
    assert_eq!(view.rows[3].right, "1 person");
}

#[test]
fn filters_and_hidden_entries() {
    let mut log = log();
    log.set_flag(1, EntryFlags::READ, true);
    log.set_flag(2, EntryFlags::STAR, true);
    log.set_flag(3, EntryFlags::HIDDEN, true);
    let topics = |filter| {
        let query = ClueQuery {
            tab: CluesTab::Topics,
            filter,
            ..ClueQuery::default()
        };
        let view = derive_open(&log, &ctx(), query);
        view.rows
            .iter()
            .filter_map(ClueRow::entry)
            .collect::<Vec<_>>()
    };
    // Topics order: GLIMMER, OXBOW, ROOK.
    assert_eq!(topics(ClueFilter::All), [1, 4, 2]);
    assert_eq!(topics(ClueFilter::Unread), [4, 2]);
    assert_eq!(topics(ClueFilter::Starred), [2]);
    // Here is Yew; today is day 2.
    assert_eq!(topics(ClueFilter::Here), [1, 2]);
    assert_eq!(topics(ClueFilter::Today), [4]);
    assert_eq!(topics(ClueFilter::Hidden), [3]);
    assert_eq!(ClueFilter::All.step(-1), ClueFilter::Hidden);
    assert_eq!(ClueFilter::Hidden.step(1), ClueFilter::All);
}

#[test]
fn search_matches_topic_name_and_place_and_opens_groups() {
    let search = |text: &str| {
        let query = ClueQuery {
            search: text,
            ..ClueQuery::default()
        };
        let view = derive_clues(&log(), &source(), &ctx(), &query);
        (view.matches, labels(&view))
    };
    let (matches, rows) = search("glim");
    assert_eq!(matches, 2);
    assert_eq!(rows.len(), 4, "groups open while searching: {rows:?}");
    assert_eq!(search("minoc").0, 2);
    assert_eq!(search("VORP").0, 2);
    let (none, rows) = search("zzz");
    assert_eq!(none, 0);
    assert_eq!(rows, [(0, NO_MATCHES.to_string())]);
}

#[test]
fn the_may_differ_line_follows_the_asked_state_rule() {
    let query = ClueQuery {
        tab: CluesTab::Topics,
        ..ClueQuery::default()
    };
    let oxbow = |standing| {
        let mut ctx = ctx();
        ctx.standing = standing;
        let view = derive_open(&log(), &ctx, query);
        let row = view
            .rows
            .iter()
            .find(|row| row.entry() == Some(4))
            .unwrap()
            .clone();
        row.detail[1].contains(MAY_DIFFER)
    };
    assert!(!oxbow(10), "heard below the line, still below");
    assert!(oxbow(60), "standing crossed the line");
    let view = derive_open(&log(), &ctx(), query);
    let fixed = view.rows.iter().find(|row| row.entry() == Some(1)).unwrap();
    assert!(!fixed.detail[1].contains(MAY_DIFFER), "{:?}", fixed.detail);
}

/// Part of T13: every tier shows what the tier below shows, the runic mark
/// only from tier 3, and nothing is ever starred but by the player.
#[test]
fn derive_clues_is_monotone_in_the_tier_and_never_stars() {
    let mut log = log();
    let mut runic = entry(npc(YEW, 3), 1, RecordAddr::Pair(1), at(1, 0));
    runic.variant = 9;
    runic.flags.set(EntryFlags::RUNIC, true);
    log.fold(runic);
    for tab in [CluesTab::Clues, CluesTab::Topics] {
        for view_kind in [ClueView::People, ClueView::Place] {
            let mut previous: Option<Vec<String>> = None;
            for tier in SpoilerTier::ALL {
                let mut ctx = ctx();
                ctx.tier = tier;
                let query = ClueQuery {
                    tab,
                    view: view_kind,
                    ..ClueQuery::default()
                };
                let view = derive_open(&log, &ctx, query);
                assert!(
                    view.rows.iter().all(|row| row.mark != RowMark::Star),
                    "no star at {tier:?}"
                );
                assert_eq!(view.starred, 0);
                let strings: Vec<String> = view.strings().map(str::to_string).collect();
                let runic = strings.iter().any(|s| s.contains(RUNIC));
                assert_eq!(runic, tier >= SpoilerTier::Hints, "{tier:?}");
                if let Some(lower) = &previous {
                    for line in lower {
                        assert!(
                            strings.iter().any(|s| s.contains(line.as_str())),
                            "{line:?} lost at {tier:?}"
                        );
                    }
                }
                previous = Some(strings);
            }
        }
    }
}

#[test]
fn pins_show_at_the_top_without_the_panel_and_in_the_panel_lines() {
    let mut log = log();
    log.set_flag(2, EntryFlags::PANEL, true);
    let view = derive_clues(&log, &source(), &ctx(), &ClueQuery::default());
    assert_eq!(view.rows[0].label, PINNED);
    assert_eq!(view.rows[1].label, "ROOK Vorpelin");
    assert_eq!(view.pinned, [2]);
    let mut panel = ctx();
    panel.pins_at_top = false;
    let view = derive_clues(&log, &source(), &panel, &ClueQuery::default());
    assert_ne!(view.rows[0].label, PINNED);

    let lines = panel_clue_lines(&log, &source(), &panel, 16);
    assert_eq!(lines, ["Clues o4 *0", "Pinned", " ROOK Vorpelin"]);
    log.set_flag(3, EntryFlags::PANEL, true);
    let lines = panel_clue_lines(&log, &source(), &panel, 16);
    assert_eq!(lines[3], " GLIMMER Minoc", "an unnamed person by town");
    let wide = panel_clue_lines(&log, &source(), &panel, 26);
    assert_eq!(wide[0], "Clues: 4 unread * 0");
    assert!(panel_clue_lines(&ClueLog::new(), &source(), &panel, 16).is_empty());
}

#[test]
fn unsaved_lines_and_dates() {
    assert_eq!(unsaved_line(0), None);
    assert_eq!(unsaved_line(1).unwrap(), "1 clue since your last save");
    assert_eq!(unsaved_line(3).unwrap(), "3 clues since your last save");
    let when = GameMinute::from_calendar(139, 13, 28, 23, 5);
    assert_eq!(date_label(when), "13-28-139");
    assert_eq!(time_label(when), "23:05");
    assert_eq!(
        derive_clues(&log(), &NoDialogs, &ctx(), &ClueQuery::default()).unsaved,
        5
    );
}

#[test]
fn without_blobs_labels_fall_back_to_the_place() {
    let query = ClueQuery {
        tab: CluesTab::Topics,
        ..ClueQuery::default()
    };
    let view = derive_clues(&log(), &NoDialogs, &ctx(), &query);
    assert!(view.rows.iter().all(|row| row.label == UNKNOWN_TOPIC));
    let people = derive_open(&log(), &ctx(), ClueQuery::default());
    assert!(people.strings().all(|s| !s.contains("Hail")), "no greeting");
}

// --- M6-04: the Notes tab and notes in search. ----------------------------

fn notes_query(search: &str) -> ClueQuery<'_> {
    ClueQuery {
        tab: CluesTab::Notes,
        search,
        ..ClueQuery::default()
    }
}

#[test]
fn m6_04_notes_tab_lists_the_save_then_people_then_clues() {
    let source = source();
    let mut log = log();
    let empty = derive_clues(&log, &source, &ctx(), &notes_query(""));
    assert_eq!(empty.rows.len(), 1, "only the save's own row");
    assert_eq!(empty.rows[0].label, SAVE_NOTE);
    assert_eq!(empty.rows[0].detail, [NO_NOTE]);
    assert_eq!(
        empty.rows[0].note_owner(),
        Some((note_owner::SAVE, 0)),
        "N writes the save's note from it"
    );
    log.set_note(note_owner::CLUE, 2, "ask about the nest");
    log.set_note(note_owner::PERSON, note_owner::person(npc(YEW, 3)), "kind");
    log.set_note(note_owner::SAVE, 0, "the long road");
    let view = derive_clues(&log, &source, &ctx(), &notes_query(""));
    let labels: Vec<&str> = view.rows.iter().map(|row| row.label.as_str()).collect();
    assert_eq!(labels, [SAVE_NOTE, "Vorpelin (Yew)", "ROOK Vorpelin"]);
    assert_eq!(view.rows[2].note_owner(), Some((note_owner::CLUE, 2)));
    assert_eq!(view.rows[2].entry(), Some(2));
    assert_eq!(view.rows[0].detail, ["Note: the long road"]);
    // Search keeps the rows whose label or note matches.
    let found = derive_clues(&log, &source, &ctx(), &notes_query("nest"));
    assert_eq!(found.rows.len(), 1);
    assert_eq!(found.rows[0].label, "ROOK Vorpelin");
}

#[test]
fn m6_04_notes_show_on_their_clue_and_are_searched() {
    let source = source();
    let mut log = log();
    log.set_note(note_owner::CLUE, 2, "ask about the nest");
    let query = ClueQuery {
        tab: CluesTab::Clues,
        search: "nest",
        ..ClueQuery::default()
    };
    let view = derive_clues(&log, &source, &ctx(), &query);
    let rook = view
        .rows
        .iter()
        .find(|row| row.entry() == Some(2))
        .expect("search reaches the note");
    assert_eq!(view.matches, 1);
    assert!(
        rook.detail
            .iter()
            .any(|line| line == "Note: ask about the nest")
    );
    // N on an entry row or a person writes on that clue or person.
    assert_eq!(rook.note_owner(), Some((note_owner::CLUE, 2)));
    let all = derive_clues(&log, &source, &ctx(), &ClueQuery::default());
    let person = all
        .rows
        .iter()
        .find(|row| {
            matches!(
                row.kind,
                RowKind::Group {
                    key: GroupKey::Person(_),
                    ..
                }
            )
        })
        .unwrap();
    assert!(matches!(person.note_owner(), Some((note_owner::PERSON, _))));
}

/// 8.2.7 "Enter on a step opens the Clues tab filtered to that step": the
/// Quest filter keeps only the entries it is given, and the panel pins'
/// group steps aside.
#[test]
fn the_quest_filter_keeps_only_its_entries() {
    let mut log = log();
    log.set_flag(1, EntryFlags::PANEL, true);
    let only = [2];
    let query = ClueQuery {
        only: Some(&only),
        ..ClueQuery::default()
    };
    let mut ctx = ctx();
    ctx.pins_at_top = true;
    let view = derive_open(&log, &ctx, query);
    assert_eq!(view.matches, 1);
    let entries: Vec<usize> = view.rows.iter().filter_map(ClueRow::entry).collect();
    assert_eq!(entries, [2]);
    assert!(view.rows.iter().all(|row| !matches!(
        row.kind,
        RowKind::Group {
            key: GroupKey::Pinned,
            ..
        }
    )));
    let none: [usize; 0] = [];
    let query = ClueQuery {
        only: Some(&none),
        ..ClueQuery::default()
    };
    let view = derive_open(&log, &ctx, query);
    assert_eq!(labels(&view), [(0, NO_MATCHES.to_string())]);
}

#[test]
fn a_name_keeps_its_dictionary_words() {
    // A Name field whose middle word is the dictionary's first token, as
    // the window prints it ("Vorpelin the Grey"), never with it dropped;
    // a closing quote the field carries for the window is not the name's.
    let mut fields = blob(&[("glimmer", text("It glows."))], &[]);
    fields[0] = record(&[&text("Vorpelin"), &[0x01], &text(" Grey\"")]);
    let dictionary = crate::PUBLISHED_COMMON_WORD_DICTIONARY;
    assert_eq!(dictionary[0], "the");
    let source = HashMap::from([(dialog(1), fields)]);
    let mut log = ClueLog::new();
    let who = npc(YEW, 3);
    log.fold(entry(who, 1, RecordAddr::Leading(0), at(0, 60)));
    log.fold(entry(who, 1, RecordAddr::Pair(0), at(0, 61)));
    let view = derive_clues(&log, &source, &ctx(), &ClueQuery::default());
    assert_eq!(view.rows[0].label, "Vorpelin the Grey (Yew)");
}

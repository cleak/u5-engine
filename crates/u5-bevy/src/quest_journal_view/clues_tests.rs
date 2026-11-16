//! The Clues and Topics tabs on the journal page (plan M6-03; design
//! 8.2.3-8.2.5, 8.6): the tabs and their strip with and without the page,
//! Ctrl+J with only chapter 8's tabs, search through the overlay's keys and
//! never into the game or the sidecar, the marks, pins and their limit, the
//! panel's clue lines, the eviction toast and the grid. Every blob is
//! synthetic: invented words in the `formats/tlk.md` layout.

use u5_runtime::enhancements::ClueText;
use u5_runtime::enhancements::clues::{
    ClueEntry, DialogKey, EncodeOptions, EntryFlags, GameMinute, MAX_ENTRIES, NOTE_MAX_LEN, NpcKey,
    REPLAY_LINE, RecordAddr, TestBits, TextBody, encode_payload,
};
use u5_runtime::quest_journal::clues::{CluesView, RowKind};

use super::clues::*;
use super::*;
use crate::enh::add_enhancements;
use crate::enh::journal::clues::{CLUE_LOG_FULL, ClueLogResource, TlkLiveCache};
use crate::enh::overlay::tests::Game;
use crate::enh::toast::Toasts;

fn launch(toggles: &[Toggle]) -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in toggles {
        launch.enhancements.toggles.set(*toggle, true);
    }
    launch
}

fn game(launch: &VisualLaunchConfig) -> Game {
    let mut game = Game::new(false);
    add_enhancements(&mut game.app, launch);
    game.frame();
    game
}

fn tap(game: &mut Game, key: KeyCode) {
    game.press(key);
    game.frame();
    game.release(key);
    game.frame();
}

fn chord(game: &mut Game, modifier: KeyCode, key: KeyCode) {
    game.press(modifier);
    game.press(key);
    game.frame();
    game.release(key);
    game.release(modifier);
    game.frame();
}

fn open(game: &Game) -> bool {
    game.app.world().resource::<OverlayState>().open() == Some(OverlayId::Journal)
}

fn ui(game: &Game) -> &QuestJournalUi {
    game.app.world().resource::<QuestJournalUi>()
}

fn text(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b | 0x80).collect()
}

const DIALOG: DialogKey = DialogKey {
    family: 0,
    dialog_id: 7,
};

/// An invented person: the five leading records and two pairs.
fn blob() -> Vec<Vec<u8>> {
    [
        "Quorvel",
        "a dusty quillwright",
        "Hail.",
        "I mend quills.",
        "Fare well.",
        "gran",
        "Old one.",
        "mill",
        "Wheels turn.",
    ]
    .iter()
    .map(|s| text(s))
    .collect()
}

fn heard(slot: u8, record: RecordAddr, minute: u32) -> ClueEntry {
    ClueEntry::heard(
        NpcKey { scene: 4, slot },
        DIALOG,
        record,
        0,
        TestBits::default(),
        GameMinute(minute),
    )
}

/// A game with `toggles`, the synthetic blob in the cache and two topics
/// heard from one person.
fn clue_game(toggles: &[Toggle]) -> Game {
    let mut game = game(&launch(toggles));
    let world = game.app.world_mut();
    world.insert_resource(TlkLiveCache::with_blobs([(DIALOG, blob())]));
    let mut log = world.resource_mut::<ClueLogResource>();
    log.log_mut().fold(heard(3, RecordAddr::Pair(0), 10));
    log.log_mut().fold(heard(3, RecordAddr::Pair(1), 20));
    game.frame();
    game
}

fn log(game: &Game) -> &u5_runtime::enhancements::clues::ClueLog {
    game.app.world().resource::<ClueLogResource>().log()
}

#[test]
fn the_tabs_follow_the_keys() {
    assert_eq!(tabs_for(true, false), [JournalTab::Quests]);
    assert_eq!(
        tabs_for(false, true),
        [JournalTab::Clues, JournalTab::Topics]
    );
    assert_eq!(
        tabs_for(true, true),
        [JournalTab::Quests, JournalTab::Clues, JournalTab::Topics]
    );
    let mut settings = JournalSettings {
        page: true,
        ..JournalSettings::default()
    };
    assert!(!QuestJournalUi::new(settings).draws_tab_strip());
    settings.clues.clues = true;
    assert!(QuestJournalUi::new(settings).draws_tab_strip());
    // The clue log alone wants the plugin and the page.
    assert!(journal_wanted(&launch(&[Toggle::JournalClues])));
    assert!(!journal_wanted(&launch(&[Toggle::JournalClueHighlight])));
}

/// Ctrl+J with `journal.page` off opens the page with only chapter 8's
/// tabs; Tab cycles them, with and without the Quests tab.
#[test]
fn ctrl_j_opens_the_clue_tabs_and_tab_cycles_them() {
    for (page, expected) in [
        (false, vec![JournalTab::Clues, JournalTab::Topics]),
        (
            true,
            vec![JournalTab::Quests, JournalTab::Clues, JournalTab::Topics],
        ),
    ] {
        let mut toggles = vec![Toggle::JournalClues];
        if page {
            toggles.push(Toggle::JournalPage);
        }
        let mut game = clue_game(&toggles);
        chord(&mut game, KeyCode::ControlLeft, KeyCode::KeyJ);
        assert!(open(&game), "page {page}");
        assert_eq!(ui(&game).tabs(), expected);
        let mut seen = vec![ui(&game).tab()];
        for _ in 0..expected.len() {
            tap(&mut game, KeyCode::Tab);
            seen.push(ui(&game).tab());
        }
        assert_eq!(seen[..expected.len()], expected[..]);
        assert_eq!(seen[expected.len()], expected[0], "wraps");
        chord(&mut game, KeyCode::ShiftLeft, KeyCode::Tab);
        assert_eq!(ui(&game).tab(), *expected.last().unwrap());
        assert!(game.visual().typed_keys.is_empty());
    }
}

#[test]
fn search_types_into_the_page_never_the_game_or_the_sidecar() {
    let mut game = clue_game(&[Toggle::JournalClues]);
    chord(&mut game, KeyCode::ControlLeft, KeyCode::KeyJ);
    assert_eq!(ui(&game).tab(), JournalTab::Clues);
    tap(&mut game, KeyCode::Slash);
    assert!(ui(&game).clue_ui().searching);
    for key in [KeyCode::KeyM, KeyCode::KeyI, KeyCode::KeyJ, KeyCode::KeyX] {
        tap(&mut game, key);
    }
    tap(&mut game, KeyCode::Backspace);
    assert_eq!(ui(&game).clue_ui().search, "MIJ");
    assert!(open(&game), "J without Ctrl types");
    assert!(game.visual().typed_keys.is_empty());
    let payload = encode_payload(log(&game), EncodeOptions::default()).unwrap();
    assert!(!payload.windows(3).any(|w| w == b"MIJ"));
    // Esc clears the search and keeps the page; the next Esc closes it.
    tap(&mut game, KeyCode::Escape);
    assert!(open(&game));
    assert!(ui(&game).clue_ui().search.is_empty());
    tap(&mut game, KeyCode::Slash);
    tap(&mut game, KeyCode::KeyM);
    tap(&mut game, KeyCode::Enter);
    assert_eq!(ui(&game).clue_ui().search, "M");
    assert_eq!(ui(&game).clue_ui().last.as_ref().unwrap().matches, 1);
    tap(&mut game, KeyCode::Escape);
    assert!(!open(&game));
    assert!(ui(&game).clue_ui().search.is_empty(), "never kept");
    assert!(game.visual().typed_keys.is_empty());
}

#[test]
fn open_star_hide_and_pin_with_a_limit_of_five() {
    let mut game = clue_game(&[Toggle::JournalClues]);
    chord(&mut game, KeyCode::ControlLeft, KeyCode::KeyJ);
    let rows = |game: &Game| ui(game).clue_ui().rows().to_vec();
    assert_eq!(rows(&game).len(), 1, "one person, closed");
    assert!(
        log(&game)
            .entries()
            .iter()
            .all(|e| !e.flags.has(EntryFlags::READ))
    );
    tap(&mut game, KeyCode::Enter);
    assert_eq!(rows(&game).len(), 3);
    assert!(
        log(&game)
            .entries()
            .iter()
            .all(|e| e.flags.has(EntryFlags::READ)),
        "opening the group reads it"
    );
    assert!(
        log(&game)
            .entries()
            .iter()
            .all(|e| !e.flags.has(EntryFlags::STAR))
    );
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::KeyS);
    assert!(log(&game).entries()[0].flags.has(EntryFlags::STAR));
    tap(&mut game, KeyCode::KeyP);
    assert!(log(&game).entries()[0].flags.has(EntryFlags::PANEL));
    // Pins show at the top of the tab with the side panel off.
    assert_eq!(rows(&game)[0].label, "Pinned");
    tap(&mut game, KeyCode::KeyH);
    assert!(
        log(&game)
            .entries()
            .iter()
            .any(|e| e.flags.has(EntryFlags::HIDDEN))
    );

    // A sixth pin is refused with a toast.
    let mut clues = ClueUi::default();
    let mut log = u5_runtime::enhancements::clues::ClueLog::new();
    for slot in 0..6 {
        log.fold(heard(slot, RecordAddr::Pair(0), u32::from(slot)));
    }
    for index in 0..5 {
        log.set_flag(index, EntryFlags::PANEL, true);
    }
    clues.take_view(CluesView {
        rows: vec![u5_runtime::quest_journal::clues::ClueRow {
            kind: RowKind::Entry(5),
            depth: 1,
            mark: Default::default(),
            label: String::new(),
            count: None,
            right: String::new(),
            detail: Vec::new(),
        }],
        ..CluesView::default()
    });
    assert_eq!(
        clues.key(
            u5_runtime::quest_journal::clues::CluesTab::Clues,
            KeyCode::KeyP,
            false,
            Some(&mut log)
        ),
        KeyOutcome::Refused(PINS_FULL)
    );
    assert!(!log.entries()[5].flags.has(EntryFlags::PANEL));
}

/// With `journal.clues` off the page is chapter 5's: one tab, no strip,
/// the same pixels as with another chapter 8 key on.
#[test]
fn zero_footprint_with_the_clue_log_off() {
    let pixels = |toggles: &[Toggle]| {
        let mut game = game(&launch(toggles));
        chord(&mut game, KeyCode::ControlLeft, KeyCode::KeyJ);
        assert!(open(&game));
        assert_eq!(ui(&game).tabs(), [JournalTab::Quests]);
        let world = game.app.world();
        world
            .resource::<Assets<Image>>()
            .get(world.resource::<QuestJournalUi>().page_image().unwrap())
            .and_then(|image| image.data.clone())
            .unwrap()
    };
    assert_eq!(
        pixels(&[Toggle::JournalPage]),
        pixels(&[Toggle::JournalPage, Toggle::JournalClueHighlight])
    );
    // Every journal key off: Ctrl+J opens nothing and reaches nothing.
    let mut quiet = game(&launch(&[Toggle::GemMapEnhanced]));
    chord(&mut quiet, KeyCode::ControlLeft, KeyCode::KeyJ);
    assert!(!quiet.app.world().contains_resource::<QuestJournalUi>());
    assert!(quiet.visual().typed_keys.is_empty());
}

/// Every clue cell stays inside the text columns and the body rows.
#[test]
fn no_glyph_outside_the_grid() {
    let long = "W".repeat(60);
    let mut rows = Vec::new();
    for depth in 0..3 {
        for kind in [
            RowKind::Group {
                key: u5_runtime::quest_journal::clues::GroupKey::Introduced,
                open: depth % 2 == 0,
                entries: vec![],
            },
            RowKind::Entry(0),
            RowKind::Person(NpcKey { scene: 1, slot: 0 }),
            RowKind::Note,
        ] {
            rows.push(u5_runtime::quest_journal::clues::ClueRow {
                kind,
                depth,
                mark: u5_runtime::quest_journal::clues::RowMark::Star,
                label: long.clone(),
                count: Some(u16::MAX),
                right: "13-28-139".to_string(),
                detail: vec![long.clone(), long.clone()],
            });
        }
    }
    for searching in [false, true] {
        let mut clues = ClueUi {
            search: "Q".repeat(20),
            searching,
            ..ClueUi::default()
        };
        clues.take_view(CluesView {
            rows: rows.clone(),
            unsaved: u32::MAX,
            matches: usize::MAX,
            ..CluesView::default()
        });
        for tab in [
            u5_runtime::quest_journal::clues::CluesTab::Clues,
            u5_runtime::quest_journal::clues::CluesTab::Topics,
        ] {
            for cell in clue_cells(&clues, tab) {
                assert!(cell.column >= TEXT_COLUMN, "{cell:?}");
                assert!(cell.last_column() <= LAST_TEXT_COLUMN as usize, "{cell:?}");
                assert!(
                    (HEADER_ROW..=DETAIL_LAST_ROW).contains(&cell.row)
                        && cell.row != UPPER_DIVIDER_ROW
                        && cell.row != LOWER_DIVIDER_ROW,
                    "{cell:?}"
                );
            }
            assert!(clue_key_help(&clues, tab).chars().count() <= TEXT_CELLS);
        }
    }
    for help in [
        KEY_HELP_GROUP,
        KEY_HELP_TOPICS_GROUP,
        KEY_HELP_ENTRY,
        KEY_HELP_SEARCH,
        KEY_HELP_EMPTY,
    ] {
        assert!(help.chars().count() <= TEXT_CELLS, "{help}");
    }
}

#[test]
fn the_side_panel_adds_the_clue_line_and_pins() {
    let mut game = clue_game(&[Toggle::JournalClues, Toggle::JournalSidePanel]);
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .log_mut()
        .set_flag(1, EntryFlags::PANEL, true);
    game.frame();
    let lines = game
        .app
        .world()
        .resource::<JournalPanelClues>()
        .0
        .lock()
        .unwrap()
        .clone();
    assert_eq!(lines[0], "Clues o2 *0");
    assert_eq!(lines[1], "Pinned");
    assert_eq!(lines[2], " MILL Yew");
    let model = SidePanelModel::from_play_state(&game.visual().state);
    let with = model.lines_with(&lines);
    assert!(with.windows(lines.len()).any(|w| w == lines.as_slice()));
    assert!(with.iter().any(|line| line == "Pinned"));
    // The log turned off: the lines go.
    game.app
        .world_mut()
        .resource_mut::<Toggles>()
        .0
        .set(Toggle::JournalClues, false);
    game.frame();
    game.frame();
    assert!(
        game.app
            .world()
            .resource::<JournalPanelClues>()
            .0
            .lock()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn the_first_eviction_toasts_once() {
    let mut game = clue_game(&[Toggle::JournalClues]);
    {
        let mut log = game.app.world_mut().resource_mut::<ClueLogResource>();
        for index in 0..MAX_ENTRIES as u32 {
            let mut entry = heard(0, RecordAddr::Pair(0), index);
            entry.variant = index as u16;
            entry.npc.slot = (index >> 16) as u8;
            log.log_mut().fold(entry);
        }
    }
    game.frame();
    assert_eq!(
        game.app.world().resource::<Toasts>().current(),
        Some(CLUE_LOG_FULL)
    );
}

// ---------------------------------------------------------------------------
// M6-04: notes and the clue's text on the page.

fn type_word(game: &mut Game, word: &str) {
    for c in word.chars() {
        let key = match c {
            'n' => KeyCode::KeyN,
            'e' => KeyCode::KeyE,
            's' => KeyCode::KeyS,
            't' => KeyCode::KeyT,
            ' ' => KeyCode::Space,
            other => panic!("no key for {other:?}"),
        };
        tap(game, key);
    }
}

#[test]
fn m6_04_the_notes_tab_needs_the_key_and_the_log() {
    use crate::quest_journal_view::tabs_with_notes;
    assert_eq!(
        tabs_with_notes(true, true, true),
        [
            JournalTab::Quests,
            JournalTab::Clues,
            JournalTab::Topics,
            JournalTab::Notes
        ]
    );
    assert_eq!(
        tabs_with_notes(false, false, true),
        [],
        "notes need the clue log"
    );
    let game = clue_game(&[Toggle::JournalClues]);
    assert!(!ui(&game).tabs().contains(&JournalTab::Notes));
    let game = clue_game(&[Toggle::JournalClues, Toggle::JournalNotes]);
    assert!(ui(&game).tabs().contains(&JournalTab::Notes));
}

#[test]
fn m6_04_a_note_typed_on_the_page_round_trips_and_never_reaches_the_game() {
    let mut game = clue_game(&[Toggle::JournalClues, Toggle::JournalNotes]);
    chord(&mut game, KeyCode::ControlLeft, KeyCode::KeyJ);
    assert!(open(&game));
    tap(&mut game, KeyCode::Enter); // open the person
    tap(&mut game, KeyCode::ArrowDown); // the first topic
    tap(&mut game, KeyCode::KeyN);
    assert!(ui(&game).clue.note.is_some());
    type_word(&mut game, "nest");
    tap(&mut game, KeyCode::Escape);
    assert!(open(&game), "Esc cancels the note, not the page");
    assert!(
        log(&game).notes().is_empty(),
        "a cancelled note is not kept"
    );
    tap(&mut game, KeyCode::KeyN);
    type_word(&mut game, "nest");
    game.press(KeyCode::ShiftLeft);
    tap(&mut game, KeyCode::Slash);
    game.release(KeyCode::ShiftLeft);
    tap(&mut game, KeyCode::Enter);
    assert!(
        game.visual().typed_keys.is_empty(),
        "never typed into the game"
    );
    let index = ui(&game)
        .clue
        .selected()
        .and_then(|row| row.entry())
        .unwrap();
    let note = log(&game)
        .note_for(
            u5_runtime::enhancements::clues::note_owner::CLUE,
            index as u16,
        )
        .unwrap();
    assert_eq!(note.text, "nest?");
    // Saved and read back as the sidecar would.
    let bytes = encode_payload(log(&game), EncodeOptions::default()).unwrap();
    let back = u5_runtime::enhancements::clues::decode_payload(&bytes).unwrap();
    assert_eq!(back.notes(), log(&game).notes());
    // N again edits it; the Notes tab lists it after the save's row.
    tap(&mut game, KeyCode::KeyN);
    assert_eq!(ui(&game).clue.note.as_ref().unwrap().text, "nest?");
    tap(&mut game, KeyCode::Escape);
    tap(&mut game, KeyCode::Tab);
    tap(&mut game, KeyCode::Tab);
    assert_eq!(ui(&game).tab(), JournalTab::Notes);
    game.frame();
    let rows = ui(&game).clue.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1].detail, ["Note: nest?"]);
    // Enter on the save's row writes the save's note.
    tap(&mut game, KeyCode::Enter);
    type_word(&mut game, "test");
    tap(&mut game, KeyCode::Enter);
    assert_eq!(
        log(&game)
            .note_for(u5_runtime::enhancements::clues::note_owner::SAVE, 0)
            .unwrap()
            .text,
        "test"
    );
}

#[test]
fn m6_04_n_writes_nothing_with_notes_off() {
    let mut game = clue_game(&[Toggle::JournalClues]);
    chord(&mut game, KeyCode::ControlLeft, KeyCode::KeyJ);
    tap(&mut game, KeyCode::Enter);
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::KeyN);
    assert!(ui(&game).clue.note.is_none());
    assert!(log(&game).notes().is_empty());
}

/// A blob whose second pair branches on the name-known bit.
fn branch_blob() -> Vec<Vec<u8>> {
    let mut fields = blob();
    let mut branch = text("Ah. ");
    branch.extend([0x8C, 0x91]);
    branch.extend(text("Hm."));
    fields[8] = branch;
    let mut rune = text("See ");
    rune.push(0x8E);
    rune.extend(text("abc"));
    rune.push(0x8E);
    fields[6] = rune;
    fields.push(vec![0x90, 0x91]);
    fields
}

#[test]
fn m6_04_the_text_pane_renders_a_plain_record_and_replays_a_branch() {
    let mut game = clue_game(&[Toggle::JournalClues]);
    game.app
        .world_mut()
        .insert_resource(TlkLiveCache::with_blobs([(DIALOG, branch_blob())]));
    game.app
        .world_mut()
        .resource_mut::<QuestJournalUi>()
        .clue
        .text = ClueText::Live;
    chord(&mut game, KeyCode::ControlLeft, KeyCode::KeyJ);
    tap(&mut game, KeyCode::Enter);
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::Enter);
    game.frame();
    let clue = &ui(&game).clue;
    assert!(clue.reading.is_some());
    let Some(TextBody::Rows(rows)) = &clue.body else {
        panic!("a plain record renders statically: {:?}", clue.body);
    };
    assert!(
        rows.iter().any(|row| row.runic != 0),
        "the runic span: {rows:?}"
    );
    let cells = clue_cells(clue, u5_runtime::quest_journal::clues::CluesTab::Clues);
    let runic: Vec<&Cell> = cells.iter().filter(|cell| cell.runic).collect();
    assert_eq!(runic.len(), 1, "{cells:?}");
    assert_eq!(runic[0].text, "abc");
    for cell in cells.iter().filter(|cell| cell.row >= TEXT_PANE_FIRST_ROW) {
        if cell.row <= BODY_LAST_ROW {
            assert!(cell.column >= TEXT_PANE_COLUMN, "{cell:?}");
            assert!(
                cell.last_column() < TEXT_PANE_COLUMN as usize + 16,
                "{cell:?}"
            );
        }
    }
    assert_eq!(
        clue_key_help(clue, u5_runtime::quest_journal::clues::CluesTab::Clues),
        KEY_HELP_READING
    );
    // Esc goes back to the list, not out of the page.
    tap(&mut game, KeyCode::Escape);
    assert!(open(&game));
    assert!(ui(&game).clue.reading.is_none());
    // The branch record: the replay line, never a guessed branch.
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::Enter);
    game.frame();
    assert_eq!(ui(&game).clue.body, Some(TextBody::Replay));
    let cells = clue_cells(
        &ui(&game).clue,
        u5_runtime::quest_journal::clues::CluesTab::Clues,
    );
    assert!(cells.iter().any(|cell| cell.text == REPLAY_LINE));
    // With clue_text off Enter only marks read.
    tap(&mut game, KeyCode::Escape);
    game.app
        .world_mut()
        .resource_mut::<QuestJournalUi>()
        .clue
        .text = ClueText::Off;
    tap(&mut game, KeyCode::Enter);
    assert!(ui(&game).clue.reading.is_none());
}

#[test]
fn m6_04_note_and_text_cells_stay_on_the_grid() {
    let long = "W".repeat(NOTE_MAX_LEN);
    let mut clues = ClueUi {
        notes: true,
        note: Some(NoteEdit {
            owner_kind: 0,
            owner: 0,
            text: long.clone(),
        }),
        ..ClueUi::default()
    };
    for text in [String::new(), "W".repeat(10), "W".repeat(30), long] {
        clues.note.as_mut().unwrap().text = text;
        for cell in clue_cells(&clues, u5_runtime::quest_journal::clues::CluesTab::Clues) {
            assert!(cell.column >= TEXT_COLUMN, "{cell:?}");
            assert!(cell.last_column() <= LAST_TEXT_COLUMN as usize, "{cell:?}");
            assert!(
                (DETAIL_FIRST_ROW..=DETAIL_LAST_ROW).contains(&cell.row)
                    || cell.row < DETAIL_FIRST_ROW
            );
        }
    }
    for help in [
        KEY_HELP_ENTRY_NOTES,
        KEY_HELP_NOTES,
        KEY_HELP_NOTE,
        KEY_HELP_READING,
        KEY_HELP_READING_NOTES,
    ] {
        assert!(help.chars().count() <= TEXT_CELLS, "{help}");
    }
    assert_eq!(note_char(KeyCode::KeyA, false), Some('a'));
    assert_eq!(note_char(KeyCode::KeyA, true), Some('A'));
    assert_eq!(note_char(KeyCode::Digit1, true), Some('!'));
    assert_eq!(note_char(KeyCode::Digit1, false), Some('1'));
    assert_eq!(note_char(KeyCode::F1, false), None);
}

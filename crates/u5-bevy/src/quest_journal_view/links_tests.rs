//! Evidence links on the page (build plan M6-07; design 8.2.5, 8.2.7,
//! 8.3.9): `Q` pins a clue to a card or step through the picker, the
//! pinned card shows "Pinned by you" at tier 2, Enter on a card or step
//! opens the Clues tab with the Quest filter, `K` sets and clears "I know
//! this", and a step pin unlocks the tier 4 cell (D8.6). Every blob is
//! synthetic: invented words in the `formats/tlk.md` layout.

use u5_runtime::enhancements::clues::{
    ClueEntry, ClueLog, DialogKey, GameMinute, Link, NpcKey, RecordAddr, TestBits,
};
use u5_runtime::quest_journal::trackers::{LINK_QUEST, LINK_STEP, QuestId, QuestKind, StepId};
use u5_runtime::{ShrineVirtue, shrine_mantra_for};

use super::clues::*;
use super::*;
use crate::enh::add_enhancements;
use crate::enh::journal::clues::{ClueLogResource, TlkLiveCache};
use crate::enh::overlay::tests::Game;

const DIALOG: DialogKey = DialogKey {
    family: 0,
    dialog_id: 7,
};

fn text(s: &str) -> Vec<u8> {
    s.bytes().map(|b| b | 0x80).collect()
}

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

fn heard(record: RecordAddr, minute: u32) -> ClueEntry {
    ClueEntry::heard(
        NpcKey { scene: 4, slot: 3 },
        DIALOG,
        record,
        0,
        TestBits::default(),
        GameMinute(minute),
    )
}

/// The clue log and the tracker on, without chapter 5's page: the tabs
/// are Quests, Clues and Topics. Two topics heard from one person.
fn link_game() -> Game {
    let mut launch = VisualLaunchConfig::classic();
    for toggle in [Toggle::JournalClues, Toggle::JournalQuestTracker] {
        launch.enhancements.toggles.set(toggle, true);
    }
    let mut game = Game::new(false);
    add_enhancements(&mut game.app, &launch);
    game.frame();
    let world = game.app.world_mut();
    world.insert_resource(TlkLiveCache::with_blobs([(DIALOG, blob())]));
    let mut log = world.resource_mut::<ClueLogResource>();
    log.log_mut().fold(heard(RecordAddr::Pair(0), 10));
    log.log_mut().fold(heard(RecordAddr::Pair(1), 20));
    game.frame();
    game
}

fn tap(game: &mut Game, key: KeyCode) {
    game.press(key);
    game.frame();
    game.release(key);
    game.frame();
}

fn ctrl_j(game: &mut Game) {
    game.press(KeyCode::ControlLeft);
    game.press(KeyCode::KeyJ);
    game.frame();
    game.release(KeyCode::KeyJ);
    game.release(KeyCode::ControlLeft);
    game.frame();
}

fn ui(game: &Game) -> &QuestJournalUi {
    game.app.world().resource::<QuestJournalUi>()
}

fn ui_mut(game: &mut Game) -> Mut<'_, QuestJournalUi> {
    game.app.world_mut().resource_mut::<QuestJournalUi>()
}

fn log(game: &Game) -> &ClueLog {
    game.app.world().resource::<ClueLogResource>().log()
}

/// The text of the tab showing, one string per cell run.
fn texts(game: &Game) -> Vec<String> {
    let ui = ui(game);
    let cells = match ui.tab().clue_tab() {
        Some(tab) => clue_cells(ui.clue_ui(), tab),
        None => page_cells(&ui.view()),
    };
    cells.into_iter().map(|cell| cell.text).collect()
}

fn has(game: &Game, text: &str) -> bool {
    texts(game).iter().any(|cell| cell == text)
}

fn valor() -> QuestId {
    QuestId::new(QuestKind::Shrine, ShrineVirtue::Valor.index() as u8)
}

/// From the Quests tab: the Clues tab, the person opened, the first topic
/// picked.
fn to_first_clue(game: &mut Game) {
    tap(game, KeyCode::Tab);
    assert_eq!(ui(game).tab(), JournalTab::Clues);
    tap(game, KeyCode::Enter);
    tap(game, KeyCode::ArrowDown);
    assert_eq!(ui(game).clue_ui().selected().unwrap().entry(), Some(0));
}

/// Picks the picker row whose label is `label` with Down.
fn pick(game: &mut Game, label: &str) {
    for _ in 0..40 {
        let picker = ui(game).clue_ui().picking.clone().unwrap();
        if picker.rows[picker.cursor].label.as_str() == label {
            return;
        }
        tap(game, KeyCode::ArrowDown);
    }
    panic!("{label} not offered");
}

#[test]
fn q_pins_to_a_card_at_tier_2_and_enter_filters_the_clues() {
    let mut game = link_game();
    ctrl_j(&mut game);
    assert_eq!(ui(&game).tabs()[0], JournalTab::Quests);
    // A blank save: no card at tier 2.
    assert!(has(&game, text::NO_CARDS), "{:?}", texts(&game));

    to_first_clue(&mut game);
    assert_eq!(
        clue_key_help(ui(&game).clue_ui(), CluesTab::Clues),
        KEY_HELP_ENTRY_QUEST
    );
    tap(&mut game, KeyCode::KeyQ);
    let picker = ui(&game).clue_ui().picking.clone().unwrap();
    // Tier 2: the shrines and the passwords, no step, nothing else.
    assert_eq!(picker.rows.len(), 10);
    assert!(picker.rows.iter().all(|row| row.depth == 0));
    assert!(has(&game, text::PIN_TO_QUEST));
    pick(&mut game, "Valor");
    tap(&mut game, KeyCode::Enter);
    assert_eq!(
        log(&game).links(),
        [Link {
            entry: 0,
            kind: LINK_QUEST,
            target: valor().code(),
        }]
    );
    assert!(has(&game, text::PIN_LINKED));
    // Esc back to the clues; the detail names the card.
    tap(&mut game, KeyCode::Escape);
    assert!(ui(&game).clue_ui().picking.is_none());
    let detail = &ui(&game).clue_ui().selected().unwrap().detail;
    assert!(
        detail.iter().any(|line| line == "Pinned to: Valor"),
        "{detail:?}"
    );

    // The Valor card exists now, its steps unnamed, the pin counted.
    tap(&mut game, KeyCode::Tab);
    tap(&mut game, KeyCode::Tab);
    assert_eq!(ui(&game).tab(), JournalTab::Quests);
    assert_eq!(
        ui(&game).tracker_ui().unwrap().selected().unwrap().quest,
        valor()
    );
    tap(&mut game, KeyCode::Enter);
    assert!(has(&game, &text::pinned_by_you(1)), "{:?}", texts(&game));
    assert!(
        ui(&game)
            .tracker_ui()
            .unwrap()
            .selected()
            .unwrap()
            .steps
            .is_empty()
    );
    // Enter on the card: the Clues tab, filtered to it.
    tap(&mut game, KeyCode::Enter);
    assert_eq!(ui(&game).tab(), JournalTab::Clues);
    assert!(has(&game, text::QUEST_FILTER));
    assert!(has(&game, "Valor"));
    assert_eq!(ui(&game).clue_ui().last.as_ref().unwrap().matches, 1);
    // Esc clears the filter.
    tap(&mut game, KeyCode::Escape);
    assert!(ui(&game).clue_ui().quest.is_none());
    assert_eq!(ui(&game).clue_ui().last.as_ref().unwrap().matches, 2);
    assert!(game.visual().typed_keys.is_empty());
    // Q again on the pinned target unpins (the group is still open).
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::KeyQ);
    pick(&mut game, "Valor");
    tap(&mut game, KeyCode::Enter);
    assert!(log(&game).links().is_empty());
}

#[test]
fn a_step_pin_unlocks_the_tier_4_cell_and_k_marks_a_step() {
    let mut game = link_game();
    ui_mut(&mut game).settings.tier = SpoilerTier::Full;
    ctrl_j(&mut game);
    to_first_clue(&mut game);
    tap(&mut game, KeyCode::KeyQ);
    // Tier 4: the card's Knowledge step is offered under it.
    pick(&mut game, "Valor");
    tap(&mut game, KeyCode::ArrowDown);
    let picker = ui(&game).clue_ui().picking.clone().unwrap();
    assert_eq!(picker.rows[picker.cursor].depth, 1);
    tap(&mut game, KeyCode::Enter);
    let step = StepId::new(valor(), 0);
    assert_eq!(
        log(&game).links(),
        [Link {
            entry: 0,
            kind: LINK_STEP,
            target: step.code(),
        }]
    );
    tap(&mut game, KeyCode::Escape);

    // D8.6: the card's answer cell exists, masked.
    tap(&mut game, KeyCode::Tab);
    tap(&mut game, KeyCode::Tab);
    let at = ui(&game)
        .tracker_ui()
        .unwrap()
        .cards()
        .iter()
        .position(|card| card.quest == valor())
        .unwrap();
    tap(&mut game, KeyCode::Home);
    for _ in 0..at {
        tap(&mut game, KeyCode::ArrowDown);
    }
    tap(&mut game, KeyCode::Enter);
    assert!(has(&game, "Mantra: "), "{:?}", texts(&game));
    assert!(has(&game, text::HIDDEN));
    assert!(!has(&game, shrine_mantra_for(ShrineVirtue::Valor)));
    assert_eq!(key_help(&ui(&game).view()), trackers::KEY_HELP_REVEAL_CLUES);

    // K on the step: "I know this", and K again clears it.
    let step_at = |game: &Game| {
        ui(game)
            .tracker_ui()
            .unwrap()
            .selected_step()
            .map(|line| line.step)
    };
    assert_eq!(step_at(&game), Some(step));
    tap(&mut game, KeyCode::KeyK);
    assert_eq!(log(&game).known(), [step.code()]);
    tap(&mut game, KeyCode::KeyK);
    assert!(log(&game).known().is_empty());
    // A save step takes no mark.
    tap(&mut game, KeyCode::ArrowDown);
    assert_ne!(step_at(&game), Some(step));
    tap(&mut game, KeyCode::KeyK);
    assert!(log(&game).known().is_empty());

    // Enter on the step: the Clues tab filtered to it.
    tap(&mut game, KeyCode::ArrowUp);
    tap(&mut game, KeyCode::Enter);
    assert_eq!(ui(&game).tab(), JournalTab::Clues);
    let quest = ui(&game).clue_ui().quest.clone().unwrap();
    assert_eq!(quest.target.step, Some(0));
    assert!(quest.label.starts_with("Valor, "));
    assert_eq!(ui(&game).clue_ui().only, [0]);
}

#[test]
fn zero_footprint_without_the_tracker() {
    // The clue log alone: no Q, no picker, no link lines.
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(Toggle::JournalClues, true);
    let mut game = Game::new(false);
    add_enhancements(&mut game.app, &launch);
    game.frame();
    game.app
        .world_mut()
        .insert_resource(TlkLiveCache::with_blobs([(DIALOG, blob())]));
    game.app
        .world_mut()
        .resource_mut::<ClueLogResource>()
        .log_mut()
        .fold(heard(RecordAddr::Pair(0), 10));
    game.frame();
    ctrl_j(&mut game);
    tap(&mut game, KeyCode::Enter);
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::KeyQ);
    assert!(ui(&game).clue_ui().picking.is_none());
    assert_eq!(
        clue_key_help(ui(&game).clue_ui(), CluesTab::Clues),
        KEY_HELP_ENTRY
    );
    assert!(log(&game).links().is_empty());
}

#[test]
fn picker_and_filter_cells_stay_on_the_grid() {
    let mut game = link_game();
    ui_mut(&mut game).settings.tier = SpoilerTier::Full;
    ctrl_j(&mut game);
    to_first_clue(&mut game);
    tap(&mut game, KeyCode::KeyQ);
    for _ in 0..40 {
        let ui = ui(&game);
        for cell in clue_cells(ui.clue_ui(), CluesTab::Clues) {
            assert!(cell.column >= TEXT_COLUMN, "{cell:?}");
            assert!(
                cell.last_column() <= usize::from(LAST_TEXT_COLUMN),
                "{cell:?}"
            );
            assert!(cell.row <= BODY_LAST_ROW, "{cell:?}");
        }
        tap(&mut game, KeyCode::ArrowDown);
    }
}

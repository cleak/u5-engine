//! The tracker cards on the page (build plan M6-06; design 8.2.7, 8.3.9,
//! 8.4.2, 8.4.4): zero footprint off, chapter 5's page unchanged with
//! `quest_tracker` off, the cards alone and in the Virtues section's
//! place, a card's steps, the tier 4 gate (T14) and the R-twice reveal for
//! the session, the clue-log-off line, the bounds and the side panel line.

use u5_runtime::quest_journal::trackers::{QuestId, QuestKind};
use u5_runtime::{ShrineVirtue, shrine_mantra_for};

use super::*;
use crate::enh::add_enhancements;
use crate::enh::overlay::tests::Game;

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

fn tap_ctrl_j(game: &mut Game) {
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

fn page_pixels(game: &Game) -> Vec<u8> {
    let world = game.app.world();
    let handle = world.resource::<QuestJournalUi>().page_image().unwrap();
    world
        .resource::<Assets<Image>>()
        .get(handle)
        .unwrap()
        .data
        .clone()
        .unwrap()
}

/// The page's text, one string per cell run, in drawing order.
fn texts(game: &Game) -> Vec<String> {
    page_cells(&ui(game).view())
        .into_iter()
        .map(|cell| cell.text)
        .collect()
}

fn has(game: &Game, text: &str) -> bool {
    texts(game).iter().any(|cell| cell == text)
}

fn ordain(game: &mut Game, virtue: ShrineVirtue) {
    game.visual_mut().state.shrine_ordained_mask |= 1 << virtue.index();
}

#[test]
fn quest_tracker_off_adds_nothing_and_alone_adds_the_quests_tab() {
    // Off: no page, no plugin state (8.4.2).
    let off = game(&launch(&[]));
    assert!(!off.app.world().contains_resource::<QuestJournalUi>());
    assert!(!journal_wanted(&launch(&[])));

    let alone = launch(&[Toggle::JournalQuestTracker]);
    assert!(journal_wanted(&alone));
    let game = game(&alone);
    let ui = ui(&game);
    assert_eq!(ui.tabs(), [JournalTab::Quests]);
    assert!(!ui.draws_tab_strip());
    assert!(ui.tracker_ui().is_some());
    assert!(ui.tracker_mode());
}

#[test]
fn chapter_5_page_is_unchanged_with_quest_tracker_off() {
    let mut game = game(&launch(&[Toggle::JournalPage]));
    ordain(&mut game, ShrineVirtue::Valor);
    assert!(ui(&game).tracker_ui().is_none());
    tap_ctrl_j(&mut game);
    assert!(!ui(&game).tracker_mode());
    assert!(ui(&game).view().tracker.is_none());
    let model = ui(&game).model().unwrap().clone();
    let mut want = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
    let font = game.visual_mut().text_font.clone();
    paint_page(
        &mut want,
        &font,
        &PageView::new(Some(&model), model.section(SectionId::Virtues)),
    );
    assert_eq!(page_pixels(&game), want, "chapter 5's Virtues page");
    assert!(has(&game, "Virtues"));
}

#[test]
fn the_cards_alone_with_the_clue_log_off() {
    let mut game = game(&launch(&[Toggle::JournalQuestTracker]));
    tap_ctrl_j(&mut game);
    // The fresh save: no card, no step, no total.
    assert!(has(&game, text::NO_CARDS), "{:?}", texts(&game));
    assert!(has(&game, text::TRACKER_SECTION));

    ordain(&mut game, ShrineVirtue::Spirituality);
    game.visual_mut().state.word_of_power_seal_flags[0] = 0x80;
    game.frame();
    let cells = texts(&game);
    for want in [
        "Quests",
        "Quest",
        "State",
        "Next",
        "Spirituality",
        "Ordained  ",
        "Read Codex",
        "Deceit",
        "Unsealed  ",
    ] {
        assert!(cells.iter().any(|cell| cell == want), "{want}: {cells:?}");
    }
    // No switcher arrows without chapter 5's page.
    assert!(!cells.iter().any(|cell| cell == "\u{1b}"));
    assert_eq!(key_help(&ui(&game).view()), trackers::KEY_HELP_LIST_ALONE);

    // Left and Right do nothing; Enter opens the selected card.
    tap(&mut game, KeyCode::ArrowRight);
    assert!(ui(&game).tracker_mode());
    let first = ui(&game).tracker_ui().unwrap().selected().unwrap().quest;
    assert_eq!(
        first,
        QuestId::new(QuestKind::Shrine, ShrineVirtue::Spirituality.index() as u8)
    );
    tap(&mut game, KeyCode::Enter);
    assert!(ui(&game).tracker_ui().unwrap().detail);
    let cells = texts(&game);
    for want in ["[?]", "[x]", "[ ]", "Chant at the shrine", "Clue log off"] {
        assert!(cells.iter().any(|cell| cell == want), "{want}: {cells:?}");
    }
    // Esc goes back to the list, then closes the page.
    tap(&mut game, KeyCode::Escape);
    assert!(!ui(&game).tracker_ui().unwrap().detail);
    assert_eq!(game.open(), Some(OverlayId::Journal));
    tap(&mut game, KeyCode::Escape);
    assert_eq!(game.open(), None);
}

#[test]
fn with_the_page_the_cards_take_the_virtues_place_only() {
    let mut game = game(&launch(&[Toggle::JournalPage, Toggle::JournalQuestTracker]));
    ordain(&mut game, ShrineVirtue::Honesty);
    game.visual_mut().state.word_of_power_seal_flags[0] = 0x80;
    tap_ctrl_j(&mut game);
    assert_eq!(ui(&game).tabs(), [JournalTab::Quests]);
    assert!(ui(&game).tracker_mode());
    let cells = texts(&game);
    assert!(cells.iter().any(|cell| cell == "Honesty"));
    // The dungeon stays in chapter 5's own section.
    assert!(!cells.iter().any(|cell| cell == "Deceit"), "{cells:?}");
    assert!(cells.iter().any(|cell| cell == "\u{1b}"));
    assert_eq!(key_help(&ui(&game).view()), trackers::KEY_HELP_LIST);

    // Right: chapter 5's Dungeons section, drawn as chapter 5 draws it.
    tap(&mut game, KeyCode::ArrowRight);
    assert_eq!(ui(&game).section_id(), SectionId::Dungeons);
    assert!(!ui(&game).tracker_mode());
    let model = ui(&game).model().unwrap().clone();
    let mut want = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
    let font = game.visual_mut().text_font.clone();
    paint_page(
        &mut want,
        &font,
        &PageView::new(Some(&model), model.section(SectionId::Dungeons)),
    );
    assert_eq!(page_pixels(&game), want);
    tap(&mut game, KeyCode::ArrowLeft);
    assert!(ui(&game).tracker_mode());
}

#[test]
fn the_tracker_follows_its_toggle_in_play() {
    let mut game = game(&launch(&[Toggle::JournalPage]));
    assert!(ui(&game).tracker_ui().is_none());
    game.app
        .world_mut()
        .resource_mut::<Toggles>()
        .0
        .set(Toggle::JournalQuestTracker, true);
    game.frame();
    assert!(ui(&game).tracker_ui().is_some());
    game.app
        .world_mut()
        .resource_mut::<Toggles>()
        .0
        .set(Toggle::JournalQuestTracker, false);
    game.frame();
    assert!(ui(&game).tracker_ui().is_none());
}

#[test]
fn t14_the_reveal_needs_evidence_r_twice_and_lasts_the_session() {
    let mut game = game(&launch(&[Toggle::JournalQuestTracker]));
    ui_mut(&mut game).settings.tier = SpoilerTier::Full;
    tap_ctrl_j(&mut game);
    // Tier 4 lists Honesty, not begun: no answer without evidence.
    tap(&mut game, KeyCode::Enter);
    assert_eq!(
        ui(&game).tracker_ui().unwrap().selected().unwrap().quest,
        QuestId::new(QuestKind::Shrine, 0)
    );
    assert!(!has(&game, text::HIDDEN), "{:?}", texts(&game));
    assert!(!has(&game, "Mantra: "));
    tap(&mut game, KeyCode::Escape);

    // Ordained: the save shows progress past the mantra (gate (a)).
    ordain(&mut game, ShrineVirtue::Honesty);
    game.frame();
    tap(&mut game, KeyCode::Enter);
    assert!(has(&game, "Mantra: "));
    assert!(has(&game, text::HIDDEN));
    assert_eq!(key_help(&ui(&game).view()), trackers::KEY_HELP_REVEAL);
    tap(&mut game, KeyCode::KeyR);
    assert!(has(&game, text::HIDDEN), "one R only asks");
    assert_eq!(key_help(&ui(&game).view()), trackers::KEY_HELP_CONFIRM);
    tap(&mut game, KeyCode::KeyR);
    let mantra = shrine_mantra_for(ShrineVirtue::Honesty);
    assert!(has(&game, mantra), "{:?}", texts(&game));

    // Closed and opened again: still revealed (the session), on the list.
    tap(&mut game, KeyCode::Escape);
    tap(&mut game, KeyCode::Escape);
    tap_ctrl_j(&mut game);
    assert!(!ui(&game).tracker_ui().unwrap().detail);
    tap(&mut game, KeyCode::Enter);
    assert!(has(&game, mantra));

    // `mantras = never` blanks the cell.
    ui_mut(&mut game).settings.mantras = MantraDisplay::Never;
    ui_mut(&mut game).dirty = true;
    game.frame();
    assert!(!has(&game, mantra));
    assert!(!has(&game, "Mantra: "));
}

#[test]
fn every_cell_stays_inside_the_text_columns_and_body_rows() {
    let mut game = game(&launch(&[Toggle::JournalQuestTracker]));
    {
        let state = &mut game.visual_mut().state;
        state.shrine_ordained_mask = 0b0011_0110;
        state.shrine_codex_mask = 0b1001_0111;
        state.shrine_ruin_flags = [0x80, 0x01, 0, 0, 0x7f, 0, 0x80, 0xff];
        state.word_of_power_seal_flags = [0x80; 8];
        state.shadowlord_hideouts = [0xFF; 3];
        state.special_items = [1; u5_runtime::SPECIAL_ITEM_COUNT];
        state.climbing_gear = 1;
    }
    for tier in SpoilerTier::ALL {
        ui_mut(&mut game).settings.tier = tier;
        if game.open() != Some(OverlayId::Journal) {
            tap_ctrl_j(&mut game);
        }
        game.frame();
        let count = ui(&game).tracker_ui().unwrap().cards().len();
        for at in 0..count {
            for detail in [false, true] {
                {
                    let mut ui = ui_mut(&mut game);
                    let tracker = ui.tracker.as_mut().unwrap();
                    tracker.cursor = at;
                    tracker.scroll = (at + 1).saturating_sub(VISIBLE_ROWS);
                    tracker.detail = detail;
                }
                for cell in page_cells(&ui(&game).view()) {
                    let label = format!("{tier:?} card {at} detail {detail}: {cell:?}");
                    assert!(cell.column >= TEXT_COLUMN, "{label}");
                    assert!(cell.last_column() <= LAST_TEXT_COLUMN as usize, "{label}");
                    assert!(
                        (HEADER_ROW..=DETAIL_LAST_ROW).contains(&cell.row)
                            && cell.row != LOWER_DIVIDER_ROW
                            && cell.row != UPPER_DIVIDER_ROW,
                        "{label}"
                    );
                }
            }
        }
    }
}

#[test]
fn the_side_panel_adds_the_card_lines_only_with_the_tracker() {
    let mut snap = QuestSnapshot::from_play_state(&u5_runtime::test_fixtures::test_state(
        u5_runtime::test_fixtures::open_grid(),
        5,
        5,
    ));
    snap.ordained_mask = 1 << ShrineVirtue::Valor.index();
    let plain = SidePanelModel::from_snapshot(&snap);
    let cards = plain.clone().with_cards(&snap);
    assert!(plain.cards.is_empty());
    let lines = cards.lines();
    assert!(lines.iter().any(|line| line == "Read Codex:"), "{lines:?}");
    assert!(lines.iter().any(|line| line == " Valor"));
    assert!(lines.iter().all(|line| line.chars().count() <= 16));
    // Without the tracker the panel is chapter 5's.
    assert_eq!(plain.lines(), SidePanelModel::from_snapshot(&snap).lines());
}

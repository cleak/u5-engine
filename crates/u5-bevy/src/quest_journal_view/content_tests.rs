//! The page's content (plan M5-04; design 5.2.4-5.2.9, 5.6 G2-G4): the
//! section cycle, the cursor and scroll keys, the per-section columns, the
//! detail rows, the tier 4 mantra cell and its reveal, the re-derive while
//! open, the no-spoiler rule on the page, and the trajectory and Live
//! guarantees with the page in use.

use std::sync::{Arc, Mutex};

use bevy::input::ButtonState;
use u5_runtime::enhancements::SpoilerTier;
use u5_runtime::quest_journal::{CompanionFact, CompanionPlace, JournalText, ShadowlordState};
use u5_runtime::test_fixtures::{open_grid, test_state};
use u5_runtime::{ShrineVirtue, dungeon_word_of_power, shrine_mantra_for};

use super::*;
use crate::enh::add_enhancements;
use crate::enh::equivalence::{Owner, Scene, Timeline, check_shell_equivalence};
use crate::enh::overlay::tests::Game;

// ---------------------------------------------------------------------------
// Fixtures.

fn page_launch() -> VisualLaunchConfig {
    let mut launch = VisualLaunchConfig::classic();
    launch.enhancements.toggles.set(Toggle::JournalPage, true);
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

fn is_open(game: &Game) -> bool {
    game.open() == Some(OverlayId::Journal)
}

fn fresh() -> QuestSnapshot {
    QuestSnapshot::from_play_state(&test_state(open_grid(), 5, 5))
}

/// Everything at its fullest: every virtue in some phase, every ruin
/// condition, every seal, every Shadowlord state, every item, a full roster
/// of 8-letter names in every place, introductions in every town with the
/// largest counts, karma 255.
fn loaded() -> QuestSnapshot {
    let mut snap = fresh();
    snap.ordained_mask = 0b0011_0110;
    snap.codex_mask = 0b1001_0111;
    snap.ruin = [0x80, 0x01, 0, 0, 0x7f, 0, 0x80, 0xff];
    snap.seals = [0x80, 0x80, 0, 0x80, 0, 0x80, 0, 0x80];
    snap.shadowlords = [
        ShadowlordState::Vanquished,
        ShadowlordState::Unknown,
        ShadowlordState::AtLarge,
    ];
    snap.special_items = [1; u5_runtime::SPECIAL_ITEM_COUNT];
    snap.climbing_gear = 9;
    snap.moral_standing = 255;
    snap.companions = (0..16)
        .map(|at| CompanionFact {
            name: format!("Wwwwwww{}", at % 10),
            place: match at % 4 {
                0 => CompanionPlace::Party,
                1 => CompanionPlace::Executed,
                _ => CompanionPlace::Lodged { scene: at as u8 },
            },
        })
        .collect();
    // A town's count is the set bits of a 32-bit bank.
    snap.introduced = (1..=40).map(|scene| (scene, 32)).collect();
    snap
}

/// Every snapshot the bounds tests walk: fresh, loaded, and the loaded one
/// with each Shadowlord shard-only (the longest Shadowlord label).
fn snapshots() -> Vec<QuestSnapshot> {
    let mut shard_only = loaded();
    shard_only.shadowlords = [ShadowlordState::AtLarge; 3];
    let mut inside = loaded();
    inside.seals = [0; 8];
    inside.current_scene = 40;
    vec![fresh(), loaded(), shard_only, inside]
}

/// Every view of `model`: each section, each cursor position (with the
/// scroll that keeps it in view), nothing or everything revealed, both
/// mantra settings.
fn every_view(model: &JournalModel, mut check: impl FnMut(&PageView<'_>, &str)) {
    let all: Vec<RowKey> = model
        .sections
        .iter()
        .flat_map(|section| section.rows.iter().map(|row| row.key))
        .collect();
    for section in &model.sections {
        for cursor in 0..section.rows.len() {
            for revealed in [&[][..], &all[..]] {
                for mantras in [MantraDisplay::Reveal, MantraDisplay::Never] {
                    let mut view = PageView::new(Some(model), Some(section));
                    view.cursor = cursor;
                    view.scroll = (cursor + 1).saturating_sub(VISIBLE_ROWS);
                    view.revealed = revealed;
                    view.mantras = mantras;
                    let label = format!(
                        "{:?} {:?} row {cursor} revealed {} {mantras:?}",
                        model.tier,
                        section.id,
                        !revealed.is_empty()
                    );
                    check(&view, &label);
                }
            }
        }
    }
}

fn texts(cells: &[Cell]) -> String {
    cells
        .iter()
        .map(|cell| cell.text.as_str())
        .collect::<Vec<_>>()
        .join("|")
}

// ---------------------------------------------------------------------------
// Sections, cursor and scroll.

/// At a fresh start only Virtues and People cycle (5.2.8): Right and Tab go
/// forward and wrap, Left and Shift+Tab go back, a digit jumps, and a digit
/// for a skipped section does nothing.
#[test]
fn the_section_cycle_skips_empty_sections() {
    let mut game = game(&page_launch());
    tap_ctrl_j(&mut game);
    assert!(is_open(&game));
    assert_eq!(ui(&game).cycle(), [SectionId::Virtues, SectionId::People]);
    assert_eq!(ui(&game).section_id(), SectionId::Virtues);

    tap(&mut game, KeyCode::ArrowRight);
    assert_eq!(ui(&game).section_id(), SectionId::People);
    tap(&mut game, KeyCode::ArrowRight);
    assert_eq!(ui(&game).section_id(), SectionId::Virtues, "wraps");
    tap(&mut game, KeyCode::ArrowLeft);
    assert_eq!(ui(&game).section_id(), SectionId::People, "wraps back");
    tap(&mut game, KeyCode::Tab);
    assert_eq!(ui(&game).section_id(), SectionId::Virtues);
    game.press(KeyCode::ShiftLeft);
    tap(&mut game, KeyCode::Tab);
    game.release(KeyCode::ShiftLeft);
    game.frame();
    assert_eq!(ui(&game).section_id(), SectionId::People, "Shift+Tab");

    for skipped in [KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit7] {
        tap(&mut game, skipped);
        assert_eq!(ui(&game).section_id(), SectionId::People, "{skipped:?}");
    }
    tap(&mut game, KeyCode::Digit1);
    assert_eq!(ui(&game).section_id(), SectionId::Virtues);
    tap(&mut game, KeyCode::Numpad6);
    assert_eq!(ui(&game).section_id(), SectionId::People);
    assert!(is_open(&game));
    assert!(game.visual().typed_keys.is_empty());
}

/// Every section of a loaded save is in the cycle at tier 2 but Endgame,
/// which exists only at tiers 3 and 4.
#[test]
fn digits_reach_every_discovered_section() {
    let mut ui = QuestJournalUi::new(JournalSettings::default());
    ui.take_model(derive(&loaded(), SpoilerTier::Goals));
    assert_eq!(ui.cycle(), &SectionId::ALL[..6]);
    for (place, id) in SectionId::ALL.iter().enumerate().take(6) {
        ui.jump(place + 1);
        assert_eq!(ui.section_id(), *id);
    }
    ui.jump(7);
    assert_eq!(ui.section_id(), SectionId::People, "no Endgame at tier 2");
    ui.take_model(derive(&loaded(), SpoilerTier::Hints));
    ui.jump(7);
    assert_eq!(ui.section_id(), SectionId::Endgame);
}

/// Up/Down stop at the ends, PgUp/PgDn move a page, Home/End go to the
/// first and last row, and the scroll always keeps the cursor in view.
#[test]
fn the_cursor_and_scroll_keys() {
    let mut ui = QuestJournalUi::new(JournalSettings::default());
    ui.take_model(derive(&loaded(), SpoilerTier::Goals));
    ui.jump(6);
    let rows = ui.current_section().unwrap().rows.len();
    assert!(rows > 2 * VISIBLE_ROWS, "{rows} rows");
    let in_view = |ui: &QuestJournalUi| {
        ui.cursor() >= ui.scroll() && ui.cursor() < ui.scroll() + VISIBLE_ROWS
    };

    ui.move_cursor(-1);
    assert_eq!((ui.cursor(), ui.scroll()), (0, 0), "stops at the top");
    for step in 1..=VISIBLE_ROWS + 2 {
        ui.move_cursor(1);
        assert_eq!(ui.cursor(), step);
        assert!(in_view(&ui));
    }
    assert_eq!(ui.scroll(), 3);
    ui.home();
    assert_eq!((ui.cursor(), ui.scroll()), (0, 0));
    ui.page(1);
    assert_eq!((ui.cursor(), ui.scroll()), (VISIBLE_ROWS, VISIBLE_ROWS));
    ui.page(1);
    ui.page(1);
    ui.page(1);
    assert_eq!(ui.cursor(), rows - 1, "PgDn stops on the last row");
    assert_eq!(ui.scroll(), rows - VISIBLE_ROWS, "and the last page");
    ui.move_cursor(1);
    assert_eq!(ui.cursor(), rows - 1, "stops at the bottom");
    ui.page(-1);
    assert!(in_view(&ui));
    assert_eq!(ui.scroll(), rows - 2 * VISIBLE_ROWS);
    ui.home();
    ui.end();
    assert_eq!((ui.cursor(), ui.scroll()), (rows - 1, rows - VISIBLE_ROWS));
    ui.page(-9);
    assert_eq!((ui.cursor(), ui.scroll()), (0, 0));
    // A new section starts at its top.
    ui.end();
    ui.step_section(1);
    assert_eq!((ui.cursor(), ui.scroll()), (0, 0));
}

/// The keys reach the page through the router while it is open.
#[test]
fn the_page_keys_move_the_cursor_in_play() {
    let mut game = game(&page_launch());
    tap_ctrl_j(&mut game);
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::ArrowDown);
    assert_eq!(ui(&game).cursor(), 2);
    tap(&mut game, KeyCode::End);
    assert_eq!(ui(&game).cursor(), 7);
    tap(&mut game, KeyCode::PageUp);
    assert_eq!(ui(&game).cursor(), 0);
    tap(&mut game, KeyCode::PageDown);
    assert_eq!(ui(&game).cursor(), 7);
    tap(&mut game, KeyCode::Home);
    assert_eq!(ui(&game).cursor(), 0);
    assert!(game.visual().typed_keys.is_empty());
}

/// The model is re-derived every frame while open (the world is Live). A
/// change keeps the section and the selected row; a section that empties
/// falls back to the first.
#[test]
fn a_model_change_while_open_keeps_the_place() {
    let mut game = game(&page_launch());
    tap_ctrl_j(&mut game);
    tap(&mut game, KeyCode::Digit6);
    tap(&mut game, KeyCode::End);
    let karma = ui(&game).current_section().unwrap().rows[ui(&game).cursor()].key;
    assert_eq!(karma, RowKey::Karma);

    // A seal opens under the page: Dungeons joins the cycle, People stays,
    // and the cursor stays on Karma.
    game.visual_mut().state.word_of_power_seal_flags[0] = 0x80;
    game.frame();
    assert!(ui(&game).cycle().contains(&SectionId::Dungeons));
    assert_eq!(ui(&game).section_id(), SectionId::People);
    let row = ui(&game).current_section().unwrap().rows[ui(&game).cursor()].key;
    assert_eq!(row, RowKey::Karma);

    // Showing Dungeons, the seal closes again: back to Virtues.
    tap(&mut game, KeyCode::Digit2);
    assert_eq!(ui(&game).section_id(), SectionId::Dungeons);
    game.visual_mut().state.word_of_power_seal_flags[0] = 0;
    game.frame();
    assert_eq!(ui(&game).section_id(), SectionId::Virtues);
    assert_eq!(ui(&game).cursor(), 0);
    assert!(!ui(&game).dirty, "repainted on the change");
}

// ---------------------------------------------------------------------------
// Columns, details and bounds.

/// No glyph leaves the 40x25 grid or the frame's inside: every cell is on
/// a text row (1, 3-20, 22-23) in columns 2-37, and no two runs of text on
/// a row overlap (the bar is under them). Walks every section, cursor,
/// reveal and mantra setting of every snapshot at every tier.
#[test]
fn every_glyph_stays_inside_the_frame_on_the_grid() {
    for snap in snapshots() {
        for tier in SpoilerTier::ALL {
            let model = derive(&snap, tier);
            every_view(&model, |view, label| {
                let cells = page_cells(view);
                for cell in &cells {
                    assert!(
                        cell.column >= TEXT_COLUMN
                            && cell.last_column() <= LAST_TEXT_COLUMN as usize,
                        "{label}: {cell:?}"
                    );
                    assert!(
                        cell.row == HEADER_ROW
                            || (SECTION_ROW..=BODY_LAST_ROW).contains(&cell.row)
                            || (DETAIL_FIRST_ROW..=DETAIL_LAST_ROW).contains(&cell.row),
                        "{label}: {cell:?}"
                    );
                }
                for (at, a) in cells.iter().enumerate() {
                    for b in &cells[at + 1..] {
                        if a.row != b.row
                            || a.paper == Some(Paper::Bar)
                            || b.paper == Some(Paper::Bar)
                        {
                            continue;
                        }
                        let apart = a.last_column() < b.column as usize
                            || b.last_column() < a.column as usize;
                        assert!(apart, "{label}: {a:?} overlaps {b:?}");
                    }
                }
                // The tabs fit their bands.
                assert!(key_help(view).len() <= PAGE_WIDTH / CH_CELL_SIDE - 4);
            });
        }
    }
}

/// The painter draws nothing outside the cells it lists: with a solid
/// font, every pixel off the frame, the tabs and the listed cells is black.
#[test]
fn the_painter_draws_only_the_listed_cells() {
    let font = u5_runtime::parse_ch_font(
        &vec![0xff; u5_runtime::CH_FONT_LEN],
        u5_runtime::IBM_CH_FILE,
    )
    .unwrap();
    let model = derive(&loaded(), SpoilerTier::Full);
    let section = model.section(SectionId::Virtues).unwrap();
    let view = PageView::new(Some(&model), Some(section));
    let mut rgba = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
    paint_page(&mut rgba, &font, &view);
    let mut blank = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
    paint_page(&mut blank, &font, &PageView::new(None, None));
    let covered = |x: usize, y: usize| {
        page_cells(&view).iter().any(|cell| {
            let (cx, cy) = (x / CH_CELL_SIDE, y / CH_CELL_SIDE);
            let bar = cell.paper == Some(Paper::Bar)
                && x + CH_CELL_SIDE / 2 >= cell.column as usize * CH_CELL_SIDE
                && x < (cell.last_column() + 1) * CH_CELL_SIDE + CH_CELL_SIDE / 2;
            cy == cell.row as usize
                && (bar || (cx >= cell.column as usize && cx <= cell.last_column()))
        })
    };
    for y in CH_CELL_SIDE..KEY_HELP_ROW as usize * CH_CELL_SIDE {
        for x in 0..PAGE_WIDTH {
            let at = (y * PAGE_WIDTH + x) * 4;
            if rgba[at..at + 3] != blank[at..at + 3] {
                assert!(covered(x, y), "stray pixel at ({x}, {y})");
            }
        }
    }
}

/// The columns line up down a section: every row's marker and value start
/// on the section's columns, whatever the row's text.
#[test]
fn the_columns_line_up() {
    let model = derive(&loaded(), SpoilerTier::Goals);
    for section in &model.sections {
        let view = PageView::new(Some(&model), Some(section));
        let cells = page_cells(&view);
        for (offset, row) in section.rows.iter().take(VISIBLE_ROWS).enumerate() {
            let y = ROWS_FIRST_ROW + offset as u8;
            let columns = columns(section.id, model.tier, Some(row));
            let on_row: Vec<&Cell> = cells
                .iter()
                .filter(|cell| cell.row == y && cell.paper != Some(Paper::Bar))
                .collect();
            assert_eq!(on_row[0].column, columns.label, "{:?}", row.key);
            if !row.value.is_empty() && section.id != SectionId::Endgame {
                assert!(
                    on_row.iter().any(|cell| Some(cell.column) == columns.value),
                    "{:?}: {}",
                    row.key,
                    texts(&cells)
                );
            }
        }
    }
}

/// The detail rows show the selected row's lines, wrapped at spaces to the
/// 36 text cells, and never more than two rows.
#[test]
fn the_detail_rows_follow_the_cursor() {
    let mut snap = fresh();
    snap.ordained_mask = 0b10;
    let model = derive(&snap, SpoilerTier::Goals);
    let section = model.section(SectionId::Virtues).unwrap();
    let mut view = PageView::new(Some(&model), Some(section));
    let detail = |view: &PageView<'_>| {
        page_cells(view)
            .into_iter()
            .filter(|cell| cell.row >= DETAIL_FIRST_ROW)
            .map(|cell| cell.text)
            .collect::<Vec<_>>()
            .join(" ")
    };
    assert_eq!(detail(&view), "", "Honesty: nothing to say at tier 2");
    view.cursor = 1;
    let want = section.rows[1]
        .detail
        .iter()
        .map(JournalText::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    assert_eq!(detail(&view), want);

    assert_eq!(wrap("aa bb cc", 5, 2), ["aa bb", "cc"]);
    assert_eq!(wrap("aaa bbb ccc ddd eee", 7, 2), ["aaa bbb", "ccc.."]);
    assert_eq!(wrap("", 7, 2), Vec::<String>::new());
    for line in wrap(&"word ".repeat(40), TEXT_CELLS, 2) {
        assert!(line.chars().count() <= TEXT_CELLS);
    }
}

// ---------------------------------------------------------------------------
// Mantras and reveals (5.2.7).

fn mantra_cells(view: &PageView<'_>) -> Vec<Cell> {
    let column = columns(SectionId::Virtues, SpoilerTier::Full, None)
        .mantra
        .unwrap();
    page_cells(view)
        .into_iter()
        .filter(|cell| cell.column == column && cell.row >= ROWS_FIRST_ROW)
        .filter(|cell| cell.row <= BODY_LAST_ROW)
        .collect()
}

/// The mantra column is only at tier 4: `[hidden]` dark grey on blue where
/// the evidence rule allows, blank where it does not, blank everywhere with
/// `mantras = "never"`; a revealed mantra is white.
#[test]
fn the_mantra_cell_masks_reveals_and_blanks() {
    let snap = loaded();
    for tier in [
        SpoilerTier::Progress,
        SpoilerTier::Goals,
        SpoilerTier::Hints,
    ] {
        let model = derive(&snap, tier);
        every_view(&model, |view, label| {
            let all = texts(&page_cells(view));
            assert!(!all.contains(text::HIDDEN), "{label}");
            assert!(!all.contains(text::HEAD_MANTRA), "{label}");
        });
    }
    let model = derive(&snap, SpoilerTier::Full);
    let section = model.section(SectionId::Virtues).unwrap();
    let mut view = PageView::new(Some(&model), Some(section));
    let masked = mantra_cells(&view);
    let known = section
        .rows
        .iter()
        .filter(|row| row.secret.is_some())
        .count();
    assert!(known > 0 && known < 8, "the fixture has known and unknown");
    assert_eq!(masked.len(), known);
    for cell in &masked {
        assert_eq!(cell.text, text::HIDDEN);
        assert_eq!(cell.text.chars().count(), MANTRA_CELLS);
        assert_eq!((cell.ink, cell.paper), (DARK_GREY, Some(Paper::Mask)));
    }
    let keys: Vec<RowKey> = section.rows.iter().map(|row| row.key).collect();
    view.revealed = &keys;
    for (cell, row) in mantra_cells(&view)
        .iter()
        .zip(section.rows.iter().filter(|row| row.secret.is_some()))
    {
        assert_eq!(cell.text, row.secret.unwrap());
        assert_eq!((cell.ink, cell.paper), (WHITE, None));
        assert!(cell.text.chars().count() <= MANTRA_CELLS);
    }
    view.mantras = MantraDisplay::Never;
    view.revealed = &[];
    assert!(mantra_cells(&view).is_empty(), "never: the column is blank");
}

/// Every mantra fits the fixed cell, so a reveal never moves anything: the
/// row's other cells are the same, and outside the mantra cell the pixels
/// are too.
#[test]
fn a_revealed_mantra_never_shifts_the_row() {
    for virtue in ShrineVirtue::ALL {
        assert!(shrine_mantra_for(virtue).chars().count() <= MANTRA_CELLS);
    }
    let font = u5_runtime::parse_ch_font(
        &vec![0xff; u5_runtime::CH_FONT_LEN],
        u5_runtime::IBM_CH_FILE,
    )
    .unwrap();
    let model = derive(&loaded(), SpoilerTier::Full);
    let section = model.section(SectionId::Virtues).unwrap();
    let column = columns(SectionId::Virtues, SpoilerTier::Full, None)
        .mantra
        .unwrap() as usize;
    for (cursor, row) in section.rows.iter().enumerate() {
        if row.secret.is_none() {
            continue;
        }
        let mut hidden = PageView::new(Some(&model), Some(section));
        hidden.cursor = (cursor + 1) % section.rows.len();
        let revealed_keys = [row.key];
        let mut shown = hidden;
        shown.revealed = &revealed_keys;
        let y = ROWS_FIRST_ROW + cursor as u8;
        let others = |view: &PageView<'_>| {
            page_cells(view)
                .into_iter()
                .filter(|cell| !(cell.row == y && cell.column as usize == column))
                .collect::<Vec<_>>()
        };
        assert_eq!(others(&hidden), others(&shown), "{:?}", row.key);
        let mut a = vec![0; PAGE_WIDTH * PAGE_HEIGHT * 4];
        let mut b = a.clone();
        paint_page(&mut a, &font, &hidden);
        paint_page(&mut b, &font, &shown);
        for py in 0..PAGE_HEIGHT {
            for px in 0..PAGE_WIDTH {
                let in_cell = py / CH_CELL_SIDE == y as usize
                    && (column..column + MANTRA_CELLS).contains(&(px / CH_CELL_SIDE));
                let at = (py * PAGE_WIDTH + px) * 4;
                if !in_cell {
                    assert_eq!(a[at..at + 4], b[at..at + 4], "({px}, {py})");
                }
            }
        }
    }
}

/// Enter or Space reveals the selected row's mantra; the key help offers
/// Enter only while there is something to reveal; closing (by key or
/// because the game got busy) masks every mantra again.
#[test]
fn a_reveal_lasts_until_the_page_closes() {
    let mut game = game(&page_launch());
    ui_mut(&mut game).settings.tier = SpoilerTier::Full;
    game.visual_mut().state.shrine_codex_mask = 0b11;
    tap_ctrl_j(&mut game);
    assert_eq!(key_help(&ui(&game).view()), KEY_HELP_REVEAL);
    tap(&mut game, KeyCode::Enter);
    assert_eq!(ui(&game).revealed(), [RowKey::Virtue(0)]);
    assert_eq!(key_help(&ui(&game).view()), KEY_HELP);
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::Space);
    assert_eq!(ui(&game).revealed(), [RowKey::Virtue(0), RowKey::Virtue(1)]);
    // Justice: not begun, intact: nothing to reveal.
    tap(&mut game, KeyCode::ArrowDown);
    tap(&mut game, KeyCode::Enter);
    assert_eq!(ui(&game).revealed().len(), 2);
    assert!(game.visual().typed_keys.is_empty(), "Enter is the page's");

    tap(&mut game, KeyCode::Escape);
    assert!(!is_open(&game));
    assert!(ui(&game).revealed().is_empty(), "cleared on close");
    tap_ctrl_j(&mut game);
    assert!(is_open(&game));
    assert!(ui(&game).revealed().is_empty(), "masked again on reopen");

    tap(&mut game, KeyCode::Home);
    tap(&mut game, KeyCode::Enter);
    assert_eq!(ui(&game).revealed().len(), 1);
    // The game gets busy: the foundation closes the page.
    game.visual_mut().state.active_yes_no_prompt = Some(u5_runtime::z_stats::YesNoPromptSession {
        kind: u5_runtime::z_stats::YesNoPromptKind::SaveGame,
    });
    game.frame();
    assert!(!is_open(&game));
    assert!(ui(&game).revealed().is_empty(), "cleared on a forced close");
}

/// `mantras = "never"`: Enter reveals nothing and the key help never
/// offers it on a virtue.
#[test]
fn never_reveals_no_mantra() {
    let mut ui = QuestJournalUi::new(JournalSettings {
        page: true,
        tier: SpoilerTier::Full,
        mantras: MantraDisplay::Never,
        ..JournalSettings::default()
    });
    ui.take_model(derive(&loaded(), SpoilerTier::Full));
    for cursor in 0..8 {
        ui.cursor = cursor;
        ui.reveal();
        assert_eq!(key_help(&ui.view()), KEY_HELP);
    }
    assert!(ui.revealed().is_empty());
}

/// A Word of Power or a flame is masked on the detail rows the same way,
/// and revealed with Enter.
#[test]
fn a_word_or_a_flame_is_masked_in_the_details() {
    let model = derive(&loaded(), SpoilerTier::Full);
    for (id, label) in [
        (SectionId::Dungeons, text::secret_label(SectionId::Dungeons)),
        (
            SectionId::Shadowlords,
            text::secret_label(SectionId::Shadowlords),
        ),
    ] {
        let section = model.section(id).unwrap();
        let (cursor, row) = section
            .rows
            .iter()
            .enumerate()
            .find(|(_, row)| row.secret.is_some())
            .unwrap();
        let mut view = PageView::new(Some(&model), Some(section));
        view.cursor = cursor;
        let cells = page_cells(&view);
        let last: Vec<&Cell> = cells
            .iter()
            .filter(|cell| cell.row == DETAIL_LAST_ROW)
            .collect();
        assert!(last[0].text.starts_with(label), "{id:?}");
        assert_eq!(last[1].text, text::HIDDEN);
        assert!(!texts(&cells).contains(row.secret.unwrap()));
        let keys = [row.key];
        view.revealed = &keys;
        assert!(texts(&page_cells(&view)).contains(row.secret.unwrap()));
    }
}

/// No-spoiler rule (foundation 1.4.7) on the page: at tiers 1 and 2 no
/// view shows a mantra, a Word of Power, a flame or a mask; at tier 4 an
/// answer shows only on a revealed row; the title carries SPOILERS only at
/// tiers 3 and 4.
#[test]
fn the_page_spoils_nothing_below_tier_3() {
    let mut secrets: Vec<&str> = ShrineVirtue::ALL
        .into_iter()
        .map(shrine_mantra_for)
        .collect();
    secrets.extend((33..=40).filter_map(dungeon_word_of_power));
    secrets.extend((0..3).map(text::flame_for));
    for snap in snapshots() {
        for tier in SpoilerTier::ALL {
            let model = derive(&snap, tier);
            assert_eq!(model.spoilers, tier.is_spoiler());
            every_view(&model, |view, label| {
                let cells = page_cells(view);
                let all = texts(&cells);
                if tier <= SpoilerTier::Goals {
                    assert!(!all.contains(text::HIDDEN), "{label}");
                    for secret in &secrets {
                        let shown = cells.iter().any(|cell| {
                            cell.text == *secret || cell.text.contains(&format!(" {secret}"))
                        });
                        assert!(!shown, "{label}: {secret}");
                    }
                } else if view.revealed.is_empty() {
                    for secret in &secrets {
                        assert!(
                            !cells.iter().any(|cell| cell.text == *secret),
                            "{label}: {secret} unrevealed"
                        );
                    }
                }
            });
        }
    }
}

// ---------------------------------------------------------------------------
// G2-G4: the game under the page.

const FRAMES: usize = 260;

/// The player walks, opens the page, uses every page key, closes it, walks,
/// opens it again and closes it with Ctrl+J.
fn page_timeline() -> Timeline {
    let mut timeline = Timeline::new(FRAMES);
    timeline
        .game_tap(5, KeyCode::ArrowUp)
        .game_tap(20, KeyCode::ArrowLeft)
        .game_tap(35, KeyCode::Space);
    ctrl_j(&mut timeline, 60);
    for (frame, key) in [
        (64, KeyCode::ArrowDown),
        (68, KeyCode::ArrowDown),
        (72, KeyCode::Enter),
        (76, KeyCode::ArrowRight),
        (80, KeyCode::Tab),
        (84, KeyCode::Digit6),
        (88, KeyCode::PageDown),
        (92, KeyCode::End),
        (96, KeyCode::PageUp),
        (100, KeyCode::Home),
        (104, KeyCode::Space),
        (108, KeyCode::ArrowLeft),
        (112, KeyCode::Digit2),
        (116, KeyCode::KeyA),
    ] {
        timeline.feature_tap(frame, key);
    }
    timeline
        .feature_hold(120, 150, KeyCode::ArrowDown)
        .feature_tap(160, KeyCode::Escape)
        .game_tap(175, KeyCode::ArrowDown)
        .game_tap(190, KeyCode::ArrowRight);
    ctrl_j(&mut timeline, 205);
    timeline.feature_tap(212, KeyCode::ArrowUp);
    ctrl_j(&mut timeline, 225);
    timeline.game_tap(240, KeyCode::ArrowUp);
    timeline
}

fn ctrl_j(timeline: &mut Timeline, frame: usize) {
    for key in [KeyCode::ControlLeft, KeyCode::KeyJ] {
        timeline.event(frame, key, ButtonState::Pressed, false, Owner::Feature);
    }
    for key in [KeyCode::KeyJ, KeyCode::ControlLeft] {
        timeline.event(frame + 1, key, ButtonState::Released, false, Owner::Feature);
    }
}

#[derive(Resource, Clone, Default)]
struct OpenFrames(Arc<Mutex<Vec<bool>>>);

fn note_open(open: Res<OpenFrames>, state: Res<OverlayState>) {
    open.0
        .lock()
        .unwrap()
        .push(state.open() == Some(OverlayId::Journal));
}

/// G2: the run with the page opened, used and closed equals the Classic run
/// with those frames idle, frame for frame: the game state's digest
/// (`prng_state`, `turn`, the clock, the transcript, the owed flags), the
/// original frame and the key queue.
#[test]
fn g2_using_the_page_equals_the_off_run() {
    let open = OpenFrames::default();
    let noted = open.clone();
    let mismatches = check_shell_equivalence(
        &test_state(open_grid(), 5, 5),
        &Scene::synthetic(),
        &page_timeline(),
        &page_launch(),
        move |app| {
            app.insert_resource(noted)
                .add_systems(Update, note_open.after(EnhSet::Observe));
        },
    );
    assert_eq!(mismatches, []);
    let open = open.0.lock().unwrap();
    assert!(!open[59], "closed before Ctrl+J");
    assert!(open[61..160].iter().all(|o| *o), "open from Ctrl+J to Esc");
    assert!(open[161..205].iter().all(|o| !*o), "closed by Esc");
    assert!(open[207..225].iter().all(|o| *o), "open again");
    assert!(open[226..].iter().all(|o| !*o), "closed by Ctrl+J");
}

/// The Avatar's place and the idle pump's phase: what a held key or a lost
/// idle tick would change.
fn pump(game: &Game) -> (f32, f32) {
    let pump = game.app.world().resource::<crate::AnimationPump>();
    (pump.accumulator, pump.hold_cursor_accumulator)
}

/// G3: an arrow held through open, page use, close and release never walks
/// the Avatar, and the first step after the release needs a fresh press. A
/// second direction pressed after the close while the first is still held
/// repeats exactly as in the off run, where only that key is pressed.
#[test]
fn g3_a_held_arrow_never_walks() {
    let mut on = game(&page_launch());
    let mut off = game(&VisualLaunchConfig::classic());
    let both = |on: &mut Game, off: &mut Game| {
        on.frame();
        off.frame();
    };
    let start = on.where_the_avatar_is();
    // Down is held from the open, through the page, to past the close.
    on.press(KeyCode::ControlLeft);
    on.press(KeyCode::KeyJ);
    on.press(KeyCode::ArrowDown);
    both(&mut on, &mut off);
    on.release(KeyCode::KeyJ);
    on.release(KeyCode::ControlLeft);
    assert!(is_open(&on));
    for frame in 0..60 {
        if frame % 3 == 0 {
            on.repeat(KeyCode::ArrowDown);
        }
        both(&mut on, &mut off);
    }
    assert_eq!(on.where_the_avatar_is().0, start.0);
    assert_eq!(on.where_the_avatar_is().1, start.1);
    on.press(KeyCode::Escape);
    both(&mut on, &mut off);
    on.release(KeyCode::Escape);
    assert!(!is_open(&on));
    for _ in 0..30 {
        on.repeat(KeyCode::ArrowDown);
        both(&mut on, &mut off);
    }
    assert_eq!(on.where_the_avatar_is(), off.where_the_avatar_is());
    assert_eq!(on.game_view(), off.game_view(), "no step after the close");

    // A fresh Right while Down is still held: the same steps, frame for
    // frame, as Right alone in the off run.
    on.press(KeyCode::ArrowRight);
    off.press(KeyCode::ArrowRight);
    for frame in 0..60 {
        if frame % 3 == 2 {
            on.repeat(KeyCode::ArrowRight);
            off.repeat(KeyCode::ArrowRight);
        }
        both(&mut on, &mut off);
        assert_eq!(on.game_view(), off.game_view(), "frame {frame}");
    }
    assert_ne!(on.where_the_avatar_is().0, start.0, "Right walked");
    on.release(KeyCode::ArrowRight);
    off.release(KeyCode::ArrowRight);
    on.release(KeyCode::ArrowDown);
    both(&mut on, &mut off);
    assert_eq!(on.game_view(), off.game_view());
    let before = on.where_the_avatar_is();
    both(&mut on, &mut off);
    assert_eq!(
        on.where_the_avatar_is().1,
        before.1,
        "released Down is idle"
    );
    on.press(KeyCode::ArrowDown);
    both(&mut on, &mut off);
    assert_ne!(on.where_the_avatar_is().1, before.1, "a fresh press walks");
}

/// G4: the page is Live. With the page open and keys typed into it, the
/// idle pump fires on exactly the frames it fires in the off run with no
/// keys: the game view and the pump's phase are equal every frame.
#[test]
fn g4_the_world_ticks_on_the_same_frames_under_the_page() {
    let mut off = game(&VisualLaunchConfig::classic());
    let mut on = game(&page_launch());
    // The off run idles through the two frames Ctrl+J takes.
    tap_ctrl_j(&mut on);
    off.frame();
    off.frame();
    assert!(is_open(&on));
    assert_eq!(off.game_view(), on.game_view());
    let keys = [
        KeyCode::ArrowDown,
        KeyCode::Enter,
        KeyCode::ArrowRight,
        KeyCode::PageDown,
        KeyCode::Digit1,
        KeyCode::End,
        KeyCode::Space,
        KeyCode::KeyN,
    ];
    let first_turn = on.visual().state.turn;
    for frame in 0..600 {
        if frame % 20 == 5 {
            on.press(keys[(frame / 20) % keys.len()]);
        }
        if frame % 20 == 6 {
            on.release(keys[(frame / 20) % keys.len()]);
        }
        off.frame();
        on.frame();
        assert!(is_open(&on), "frame {frame}");
        assert_eq!(off.game_view(), on.game_view(), "frame {frame}");
        assert_eq!(pump(&off), pump(&on), "frame {frame}");
    }
    assert!(
        off.visual().prompt_cursor_frame > 0,
        "the idle pump ran in both"
    );
    assert_eq!(on.visual().state.turn, first_turn, "no turn passed");
}

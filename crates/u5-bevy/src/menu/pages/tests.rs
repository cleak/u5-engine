//! The page model and the reducer, without a window (design 3.2.1, 3.3.4
//! "Pages"; plan M4-04): every key on every page, and no command from a
//! greyed row.

use super::*;

const KEYS: [KeyCode; 14] = [
    KeyCode::F12,
    KeyCode::Escape,
    KeyCode::ArrowUp,
    KeyCode::ArrowDown,
    KeyCode::ArrowLeft,
    KeyCode::ArrowRight,
    KeyCode::Enter,
    KeyCode::NumpadEnter,
    KeyCode::Space,
    KeyCode::Tab,
    KeyCode::Backspace,
    KeyCode::Delete,
    KeyCode::F2,
    KeyCode::KeyA,
];

const VOLUME: &str = "audio.volume";
const MUTE: &str = "audio.muted";

fn key(key: KeyCode) -> MenuInput {
    MenuInput::Key(OverlayKey::plain(key))
}

fn every_page() -> impl Iterator<Item = Page> {
    [Page::Top]
        .into_iter()
        .chain(SettingsTab::ALL.map(Page::Settings))
}

/// The default launch's shell with slots, Save available, and fake rows on
/// the Audio tab: a value, a check box and a greyed check box.
fn full_shell() -> ShellSnapshot {
    let mut snap = ShellSnapshot {
        intro_shell: true,
        slots: true,
        save_available: true,
        ..ShellSnapshot::default()
    };
    snap.settings[SettingsTab::Audio.index()] = vec![
        Row::value(t::VOLUME, VOLUME),
        Row::check(t::MUTE, MUTE, false),
        Row::check(t::MUTE_ON_FOCUS_LOSS, "audio.mute_on_focus_loss", true).greyed("A reason."),
    ];
    snap
}

fn labels(rows: &[Row]) -> Vec<&'static str> {
    rows.iter().map(|row| row.label).collect()
}

/// The model on `page` with the cursor on `cursor`.
fn model_on(page: Page, cursor: usize) -> MenuModel {
    let mut model = MenuModel::new();
    if page != Page::Top {
        model.push(page);
    }
    model.top_mut().cursor = cursor;
    model
}

// ---------------------------------------------------------------------------
// The top menu's items (3.2.1).

#[test]
fn the_visual_shell_offers_the_short_menu() {
    for slots in [false, true] {
        let snap = ShellSnapshot {
            intro_shell: false,
            slots,
            save_available: true,
            doors: vec![MenuDoor {
                label: "A test door",
                opens: OverlayId::Map,
            }],
            ..ShellSnapshot::default()
        };
        assert_eq!(
            labels(&page_rows(Page::Top, &snap)),
            [t::RESUME, t::SETTINGS, t::QUIT_TO_DESKTOP],
            "slots {slots}"
        );
    }
}

#[test]
fn the_default_shell_lists_each_item_only_with_its_feature() {
    let bare = ShellSnapshot {
        intro_shell: true,
        ..ShellSnapshot::default()
    };
    assert_eq!(
        labels(&page_rows(Page::Top, &bare)),
        [t::RESUME, t::SETTINGS, t::QUIT_TO_TITLE, t::QUIT_TO_DESKTOP]
    );

    // Doors: the journal and the map before Settings, the cheats after.
    let doors = vec![
        MenuDoor {
            label: "Test cheats",
            opens: OverlayId::Cheats,
        },
        MenuDoor {
            label: "Test journal",
            opens: OverlayId::Journal,
        },
        MenuDoor {
            label: "Test map",
            opens: OverlayId::Map,
        },
    ];
    let snap = ShellSnapshot {
        doors,
        ..full_shell()
    };
    let rows = page_rows(Page::Top, &snap);
    assert_eq!(
        labels(&rows),
        [
            t::RESUME,
            t::SAVE,
            t::LOAD,
            "Test journal",
            "Test map",
            t::SETTINGS,
            "Test cheats",
            t::QUIT_TO_TITLE,
            t::QUIT_TO_DESKTOP,
        ]
    );
    assert!(rows.iter().all(|row| row.enabled && row.reason.is_none()));

    // Save is greyed, with its reason, where `Q` would not save.
    let snap = ShellSnapshot {
        save_available: false,
        ..full_shell()
    };
    let save = page_rows(Page::Top, &snap)[1];
    assert_eq!(save.action, RowAction::Save);
    assert!(!save.enabled);
    assert_eq!(save.reason, Some(t::SAVE_UNAVAILABLE));
}

#[test]
fn doors_ship_empty() {
    assert_eq!(MenuDoors::default(), MenuDoors(Vec::new()));
}

// ---------------------------------------------------------------------------
// The keys.

#[test]
fn f12_closes_from_every_page() {
    let snap = full_shell();
    for page in every_page() {
        let mut model = model_on(page, 0);
        assert_eq!(
            reduce(&mut model, key(KeyCode::F12), &snap),
            [MenuCommand::Close],
            "{page:?}"
        );
    }
}

#[test]
fn escape_goes_back_then_closes() {
    let snap = full_shell();
    // The full shell's top menu: Resume, Save, Load, Settings, ...
    let rows = page_rows(Page::Top, &snap);
    assert_eq!(rows[3].action, RowAction::Settings);
    let mut model = MenuModel::new();
    for _ in 0..3 {
        assert_eq!(reduce(&mut model, key(KeyCode::ArrowDown), &snap), []);
    }
    assert_eq!(model.current().cursor, 3);
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    assert_eq!(
        model.current(),
        PageState {
            page: Page::Settings(SettingsTab::Audio),
            cursor: 0
        }
    );
    assert_eq!(model.stack().len(), 2);
    assert_eq!(reduce(&mut model, key(KeyCode::Escape), &snap), []);
    assert_eq!(
        model.current(),
        PageState {
            page: Page::Top,
            cursor: 3
        },
        "back where it was"
    );
    assert_eq!(
        reduce(&mut model, key(KeyCode::Escape), &snap),
        [MenuCommand::Close]
    );
    assert_eq!(model.stack().len(), 1, "the top page stays");
}

#[test]
fn the_cursor_stops_at_the_ends() {
    let snap = full_shell();
    let rows = page_rows(Page::Top, &snap).len();
    let mut model = MenuModel::new();
    reduce(&mut model, key(KeyCode::ArrowUp), &snap);
    assert_eq!(model.current().cursor, 0);
    for _ in 0..rows + 3 {
        reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    }
    assert_eq!(model.current().cursor, rows - 1);
    // A cursor past the rows (they shrank) is pulled back first.
    model.top_mut().cursor = 99;
    reduce(&mut model, key(KeyCode::KeyA), &snap);
    assert_eq!(model.current().cursor, rows - 1);
    // A page with no rows keeps the cursor at 0.
    let mut model = model_on(Page::Settings(SettingsTab::Game), 0);
    reduce(&mut model, key(KeyCode::ArrowDown), &snap);
    assert_eq!(model.current().cursor, 0);
}

#[test]
fn enter_chooses_each_top_item() {
    let doors = vec![MenuDoor {
        label: "Test journal",
        opens: OverlayId::Journal,
    }];
    let snap = ShellSnapshot {
        doors,
        ..full_shell()
    };
    let expected = [
        Some(MenuCommand::Close),
        Some(MenuCommand::OpenSave),
        Some(MenuCommand::OpenLoad),
        Some(MenuCommand::OpenDoor(OverlayId::Journal)),
        None, // Settings: a page
        Some(MenuCommand::QuitToTitle),
        Some(MenuCommand::QuitToDesktop),
    ];
    assert_eq!(page_rows(Page::Top, &snap).len(), expected.len());
    for (cursor, want) in expected.into_iter().enumerate() {
        for enter in [KeyCode::Enter, KeyCode::NumpadEnter] {
            let mut model = model_on(Page::Top, cursor);
            let got = reduce(&mut model, key(enter), &snap);
            assert_eq!(got, want.into_iter().collect::<Vec<_>>(), "row {cursor}");
            // Settings, Save and Load are pages; Save and Load also ask
            // the slot plugin to list the slots.
            let pushed = model.current().page != Page::Top;
            let page = matches!(
                want,
                None | Some(MenuCommand::OpenSave | MenuCommand::OpenLoad)
            );
            assert_eq!(pushed, page, "row {cursor}");
        }
    }
}

#[test]
fn setting_rows_take_space_enter_left_and_right() {
    let snap = full_shell();
    let audio = Page::Settings(SettingsTab::Audio);
    // The value row: Left and Right step it; Space and Enter do nothing.
    let mut model = model_on(audio, 0);
    assert_eq!(
        reduce(&mut model, key(KeyCode::ArrowLeft), &snap),
        [MenuCommand::Step(VOLUME, -1)]
    );
    assert_eq!(
        reduce(&mut model, key(KeyCode::ArrowRight), &snap),
        [MenuCommand::Step(VOLUME, 1)]
    );
    assert_eq!(model.current().page, audio, "no tab change on a value");
    assert_eq!(reduce(&mut model, key(KeyCode::Space), &snap), []);
    assert_eq!(reduce(&mut model, key(KeyCode::Enter), &snap), []);
    // The check box: Space and Enter flip it; Right switches the tab.
    let mut model = model_on(audio, 1);
    assert_eq!(
        reduce(&mut model, key(KeyCode::Space), &snap),
        [MenuCommand::Flip(MUTE)]
    );
    assert_eq!(
        reduce(&mut model, key(KeyCode::Enter), &snap),
        [MenuCommand::Flip(MUTE)]
    );
    assert_eq!(reduce(&mut model, key(KeyCode::ArrowRight), &snap), []);
    assert_eq!(
        model.current(),
        PageState {
            page: Page::Settings(SettingsTab::Display),
            cursor: 0
        }
    );
}

#[test]
fn left_and_right_walk_the_tabs_and_stop_at_the_ends() {
    let snap = ShellSnapshot::default();
    let mut model = model_on(Page::Settings(SettingsTab::Audio), 0);
    reduce(&mut model, key(KeyCode::ArrowLeft), &snap);
    assert_eq!(model.current().page, Page::Settings(SettingsTab::Audio));
    for tab in [
        SettingsTab::Display,
        SettingsTab::Game,
        SettingsTab::Enhancements,
        SettingsTab::Enhancements,
    ] {
        reduce(&mut model, key(KeyCode::ArrowRight), &snap);
        assert_eq!(model.current().page, Page::Settings(tab));
    }
    reduce(&mut model, key(KeyCode::ArrowLeft), &snap);
    assert_eq!(model.current().page, Page::Settings(SettingsTab::Game));
    assert_eq!(model.stack().len(), 2, "a tab change is no page push");
    // Left and Right on the top menu do nothing.
    let mut model = MenuModel::new();
    for k in [KeyCode::ArrowLeft, KeyCode::ArrowRight] {
        assert_eq!(reduce(&mut model, key(k), &full_shell()), []);
        assert_eq!(model, MenuModel::new());
    }
    assert_eq!(
        SettingsTab::ALL.map(SettingsTab::label),
        [
            t::TAB_AUDIO,
            t::TAB_DISPLAY,
            t::TAB_GAME,
            t::TAB_ENHANCEMENTS
        ]
    );
}

/// Every key on every page with the cursor on every row: a greyed row
/// gives no command, whatever the key; only F12 and Escape (on the top
/// page) close over one.
#[test]
fn no_greyed_row_gives_a_command() {
    let mut snap = full_shell();
    snap.save_available = false;
    let mut greyed = 0;
    for page in every_page() {
        let rows = page_rows(page, &snap);
        for (cursor, row) in rows.iter().enumerate() {
            for k in KEYS {
                let mut model = model_on(page, cursor);
                let commands = reduce(&mut model, key(k), &snap);
                assert!(commands.len() <= 1, "{page:?} {cursor} {k:?}");
                if row.enabled {
                    continue;
                }
                let allowed: &[MenuCommand] = match k {
                    KeyCode::F12 => &[MenuCommand::Close],
                    KeyCode::Escape if page == Page::Top => &[MenuCommand::Close],
                    _ => &[],
                };
                assert_eq!(commands, allowed, "{page:?} row {cursor} ({row:?}) {k:?}");
                greyed += 1;
            }
        }
    }
    assert_eq!(greyed, 2 * KEYS.len(), "Save and the greyed check box");
}

/// Keys that mean nothing on these pages, and typed text, change nothing.
#[test]
fn other_keys_and_text_change_nothing() {
    let snap = full_shell();
    for page in every_page() {
        for k in [
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::F2,
            KeyCode::KeyA,
            KeyCode::Tab,
        ] {
            let mut model = model_on(page, 1);
            let rows = page_rows(page, &snap).len();
            assert_eq!(reduce(&mut model, key(k), &snap), [], "{page:?} {k:?}");
            assert_eq!(model.current().page, page);
            assert_eq!(model.current().cursor, 1.min(rows.saturating_sub(1)));
        }
        let mut model = model_on(page, 0);
        let before = model.clone();
        for ch in ['a', 'Z', ' ', '~'] {
            assert_eq!(reduce(&mut model, MenuInput::Text(ch), &snap), []);
        }
        assert_eq!(model, before);
    }
    // Modifiers do not change a key's meaning on these pages.
    let mut model = MenuModel::new();
    let ctrl_down = OverlayKey {
        key: KeyCode::ArrowDown,
        shift: true,
        ctrl: true,
    };
    reduce(&mut model, MenuInput::Key(ctrl_down), &snap);
    assert_eq!(model.current().cursor, 1);
}

/// Tab opens the full map only while its door is in the menu (D-M4.9).
#[test]
fn tab_opens_the_map_only_through_its_door() {
    let mut model = MenuModel::new();
    assert_eq!(reduce(&mut model, key(KeyCode::Tab), &full_shell()), []);
    let snap = ShellSnapshot {
        doors: vec![MenuDoor {
            label: "Test map",
            opens: OverlayId::Map,
        }],
        ..full_shell()
    };
    for page in [Page::Top, Page::Settings(SettingsTab::Game)] {
        let mut model = model_on(page, 0);
        assert_eq!(
            reduce(&mut model, key(KeyCode::Tab), &snap),
            [MenuCommand::OpenDoor(OverlayId::Map)],
            "{page:?}"
        );
    }
}

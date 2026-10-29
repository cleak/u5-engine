// Regressions for the pass-18 exploratory-playtest findings (text18, P18-xx).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// Two `Get-North` commands, each checked for `commands.md §5.8`'s
/// `Get-North` ⏎ `[blank]` ⏎ `Nothing to get!` rows.
fn p18_assert_get_north_keeps_its_blank_row(state: &mut PlayState, dir: &Path) {
    for _ in 0..2 {
        handle_play_key_input(state, 'G', "", dir).unwrap();
        handle_play_key_input(state, char::from(INPUT_CODE_NORTH), "", dir).unwrap();
        state.flush_staged_narration();
        let tail = p4window_rows_from(&p4window_rows(state), ">Get-North");
        assert_eq!(
            tail.get(..3).map(<[String]>::to_vec),
            Some(vec![
                ">Get-North".to_string(),
                String::new(),
                "Nothing to get!".to_string()
            ]),
            "{:?}",
            p4window_rows(state)
        );
    }
}

/// P18-03: `save-load.md §5.2` prints `Yes`, `Saving...` and `Done.` and
/// returns to the mode loop; nothing there changes later commands' spacing,
/// so every later `Get-North` keeps the blank row `commands.md §5.8` shows.
#[test]
fn playtest_p18_03_save_does_not_leave_the_row_open_for_later_commands() {
    let dir = debug_game_dir();
    let mut template = saved_game_seed_bytes(0, 0, 5, 5);
    template[SAVE_AVATAR_NAME_OFFSET] = b'A';
    fs::write(dir.join("SAVED.GAM"), template).unwrap();
    write_empty_ool_mirrors(&dir);
    let mut state = britannia_state(open_world_grid(), 5, 5);
    handle_play_key_input(&mut state, 'Q', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    assert_eq!(state.message, SAVE_DONE_MESSAGE);
    assert!(dir.join("SAVED.OOL").exists());
    p18_assert_get_north_keeps_its_blank_row(&mut state, &dir);
    let _ = fs::remove_dir_all(dir);
}

/// P18-03, troll variant: `overworld.md §7.1` step 3 echoes the accepted
/// letter "and a line feed", and step 5's paid toll prints nothing more, so
/// the answered question's row is closed and later results keep their blank.
#[test]
fn playtest_p18_03_paid_troll_toll_does_not_leave_the_row_open() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.gold = 100;
    state.emit_message_line(format!(
        "{TROLL_BRIDGE_CAUGHT_HEADING}54{TROLL_BRIDGE_TOLL_SUFFIX}{TROLL_BRIDGE_TOLL_QUESTION}"
    ));
    assert!(state.message_row_open_mid_line);
    state.resolve_troll_bridge_toll_payment(54, &dir).unwrap();
    assert_eq!(state.gold, 46);
    p18_assert_get_north_keeps_its_blank_row(&mut state, &dir);
    let _ = fs::remove_dir_all(dir);
}

/// P18-04: `rest-and-camp.md §4` - "The trailing space fills the second row
/// exactly, so the cursor waits at column 0 of the next row", the
/// `text-output.md §6` wrap the per-cell emitter has already taken when a
/// chunk fills its row. At the bottom row that wrap scrolls the window.
#[test]
fn playtest_p18_04_row_filling_trailing_space_puts_the_cursor_on_the_next_row() {
    let mut log = GameplayMessageLog::default();
    log.push_command("Hole up- For");
    log.push_output("how many hours? ");
    let layout = layout_message_window_with_open_prompt(&log, None, None);
    let hours = layout
        .rows
        .iter()
        .find(|row| row.text == "how many hours?")
        .expect("the prompt's second row is drawn");
    assert_eq!(layout.inline_cursor, Some((MESSAGE_WINDOW_LEFT, hours.row + 1)));

    let mut full = GameplayMessageLog::default();
    for index in 0..MESSAGE_WINDOW_ROWS {
        full.push_output(&format!("row {index}"));
    }
    full.push_command("Hole up- For");
    full.push_output("how many hours? ");
    let layout = layout_message_window_with_open_prompt(&full, None, None);
    let hours = layout
        .rows
        .iter()
        .find(|row| row.text == "how many hours?")
        .expect("the prompt's second row is drawn");
    assert_eq!(hours.row, MESSAGE_WINDOW_BOTTOM - 1, "the wrap scrolled the window");
    assert_eq!(layout.inline_cursor, Some((MESSAGE_WINDOW_LEFT, MESSAGE_WINDOW_BOTTOM)));
    assert!(layout.rows.iter().all(|row| row.row >= MESSAGE_WINDOW_TOP));

    // A trailing space that does not fill the row keeps the cursor past it.
    let mut short = GameplayMessageLog::default();
    short.push_output("How many? ");
    let layout = layout_message_window_with_open_prompt(&short, None, None);
    let row = layout.rows.last().unwrap();
    assert_eq!(layout.inline_cursor, Some((MESSAGE_WINDOW_LEFT + 10, row.row)));
}

/// P18-07: `shops.md §6` - a sage/lore letter "pressed before a prior
/// accepted menu branch has established continuation ... is ignored and the
/// menu waits for another post-list key", and the §8.A table puts it with the
/// invalid letters. It echoes nothing onto the open question row, however
/// often it is pressed.
#[test]
fn playtest_p18_07_gated_tavern_lore_letter_echoes_nothing() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 100;
    let menu = TavernState::Menu {
        tavern: Tavern::TheCatsLair,
        continuation_ready: false,
    };
    state.active_shop = Some(ActiveShopSession::Tavern(menu));
    state.emit_message_line("thy travels?\" ");
    let rows_before: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    let lore = crate::shops::tavern_menu_letters(Tavern::TheCatsLair).lore;

    for _ in 0..2 {
        handle_play_key_input(&mut state, lore, "", Path::new("")).unwrap();
    }

    let rows_after: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    assert_eq!(rows_after, rows_before);
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Tavern(TavernState::Menu {
            continuation_ready: false,
            ..
        }))
    ));
}

/// P18-05: `view.md §4.2.2` step 6 prints `the night sky! ` "note the
/// trailing space and the absence of a newline - then busy-wait[s] until any
/// key is available". While it waits the command has not returned, so no
/// marker row is drawn (`text-output.md §10.4`, deferral); once the key is
/// spent the next poll's feed closes the open row and the marker lands
/// directly beneath it, with no blank between.
#[test]
fn playtest_p18_05_night_sky_row_stays_open_for_the_next_marker() {
    let dir = debug_game_dir();
    let mut state = britannia_state(open_world_grid(), 5, 5);
    state.activate_night_sky_overlay(Some("Looking..."));
    state.flush_message_slot();

    let waiting = p4window_rows_from(&p4window_rows(&state), "the night sky!");
    assert_eq!(waiting, vec!["the night sky!".to_string()], "{:?}", p4window_rows(&state));

    handle_play_key_input(&mut state, 'x', "", &dir).unwrap();
    assert!(state.active_view_overlay.is_none());
    let dismissed = p4window_rows_from(&p4window_rows(&state), "the night sky!");
    assert_eq!(
        dismissed,
        vec!["the night sky!".to_string(), ">".to_string()],
        "{:?}",
        p4window_rows(&state)
    );
    let _ = fs::remove_dir_all(dir);
}

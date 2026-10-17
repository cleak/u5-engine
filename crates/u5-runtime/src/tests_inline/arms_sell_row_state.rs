// Row state through the arms shop's Sell flow (`shop-arms-sell-flow`).

fn arms_sell_flow_state() -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x81,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.equipment_stock = [0; EQUIPMENT_COUNT];
    state.equipment_stock[0] = 3;
    state.equipment_stock[1] = 3;
    state
}

fn arms_sell_rows(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

fn arms_rows_after(keys: &[char]) -> Vec<String> {
    let mut state = arms_sell_flow_state();
    assert_eq!(
        state.talk_facing_with_dialogue(&HashMap::new()),
        MoveOutcome::Talked
    );
    state.flush_message_slot();
    for &key in keys {
        handle_play_key_input(&mut state, key, "", Path::new("")).unwrap();
    }
    arms_sell_rows(&state)
        .into_iter()
        .map(|row| row.trim_end().to_string())
        .collect()
}

fn ends_with_rows(rows: &[String], tail: &[&str]) -> bool {
    rows.len() >= tail.len()
        && rows[rows.len() - tail.len()..]
            .iter()
            .zip(tail)
            .all(|(row, want)| row == want)
}

/// `shops.md` §8.B/§8.1 and the stock captures of `shop-arms-sell-flow`
/// (2026-09-18 through 2026-10-16, every run): each echo lands on the row its
/// prompt left open, and each later print opens with `\n\n` - a close and a
/// blank. Measured 2026-10-16 (`shop-arms-sell-flow-20261016-223816`): this
/// engine put `Sell`, `No` and `Yes` on rows of their own, dropped the blank
/// rows above the offer, the continuation and `"Done!"`, and printed the
/// offer without its opening quote.
#[test]
fn arms_sell_flow_echoes_continue_their_prompt_rows() {
    let sell = arms_rows_after(&[' ', 'S']);
    assert!(
        ends_with_rows(
            &sell,
            &[
                "\"Hail, friend! Wouldst thou Buy or Sell?\" Sell",
                "",
                "\"Which item wouldst thou like to sell?\"",
            ]
        ),
        "{sell:#?}"
    );

    let offer = arms_rows_after(&[' ', 'S', '\r']);
    assert!(
        ends_with_rows(
            &offer,
            &[
                "\"Which item wouldst thou like to sell?\"",
                "",
                "\"Leather Helm costs 14 gold.",
                "",
                "Deal?\"",
            ]
        ),
        "{offer:#?}"
    );

    let declined = arms_rows_after(&[' ', 'S', '\r', 'N']);
    assert!(
        ends_with_rows(
            &declined,
            &["", "Deal?\" No", "", "\"What else can ye offer me?\""]
        ),
        "{declined:#?}"
    );

    let sold = arms_rows_after(&[' ', 'S', '\r', 'N', '\r', 'Y']);
    assert!(
        ends_with_rows(
            &sold,
            &[
                "\"What else can ye offer me?\"",
                "",
                "\"Leather Helm costs 14 gold.",
                "",
                "Deal?\" Yes",
                "",
                "\"Done!\"",
                "says Max.",
                "",
                "\"What other arms wilt thou sell?\"",
            ]
        ),
        "{sold:#?}"
    );
}

/// The buy side's stock letter takes the same row. Measured 2026-10-16
/// (`shop-arms-buy-confirm-20261016-214848`, beat `quote`): the original reads
/// `interest?" a`, a blank row, then the quote; this engine had the `a` on a
/// row of its own.
#[test]
fn arms_buy_stock_letter_continues_the_stock_call_row() {
    let rows = arms_rows_after(&[' ', 'B', 'a']);
    let echo = rows
        .iter()
        .position(|row| row.ends_with("?\" a"))
        .unwrap_or_else(|| panic!("no `?\" a` row: {rows:#?}"));
    assert_eq!(
        rows.get(echo + 1).map(String::as_str),
        Some(""),
        "{rows:#?}"
    );
    assert!(rows[echo + 2].starts_with('"'), "{rows:#?}");
    assert!(!rows.iter().any(|row| row == "a"), "{rows:#?}");
}

fn shipwright_rows_after(keys: &[char], game_dir: &Path) -> Vec<String> {
    let mut grid = open_grid();
    grid[2 * 32 + 1] = 0x05;
    let mut state = test_state(grid, 1, 1);
    state.area = Area::Town {
        scene: Scene::new(24).unwrap(),
        floor: 0,
    };
    state.player.facing = Direction::East;
    state.gold = 150;
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x84,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    assert_eq!(
        state.talk_facing_with_dialogue(&HashMap::new()),
        MoveOutcome::Talked
    );
    state.flush_message_slot();
    for &key in keys {
        handle_play_key_input(&mut state, key, "", game_dir).unwrap();
    }
    arms_sell_rows(&state)
        .into_iter()
        .map(|row| row.trim_end().to_string())
        .collect()
}

/// `shops.md` §8.B: the shipwright's Yes is bare `Yes` and record `119`
/// "carries its own leading spacing"; §8.C: `F` echoes with `\n\n`. Stock
/// captures of `bd-shipwright` (2026-09-18 through 2026-10-16): `?" Yes`, one
/// blank row, the menu; `see?" F`, one blank, the offer; `take it?" No`, one
/// blank, the bark. This engine had three blank rows under `Yes`, a fresh
/// command row under the offer, and `No` on a row of its own with no blank.
#[test]
fn shipwright_echoes_continue_their_rows_and_take_one_blank() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let rows = shipwright_rows_after(&['Y', 'F', 'N'], &dir);
    let yes = rows
        .iter()
        .position(|row| row.ends_with("Yes"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[yes + 1], "", "{rows:#?}");
    assert!(rows[yes + 2].starts_with("\"We sell"), "{rows:#?}");
    let f = rows
        .iter()
        .position(|row| row.ends_with("?\" F"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[f + 1], "", "{rows:#?}");
    assert!(!rows[f + 2].is_empty(), "{rows:#?}");
    let no = rows
        .iter()
        .position(|row| row.ends_with("take it?\" No"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[no + 1], "", "{rows:#?}");
    assert!(rows[no + 2].starts_with('"'), "{rows:#?}");
    assert!(rows[no + 3].starts_with("says "), "{rows:#?}");
    assert_eq!(rows.len(), no + 4, "{rows:#?}");
}

/// The offer's question keeps the cursor inline, so `N` echoes onto it -
/// asset-free, through the fallback offer text.
#[test]
fn shipwright_offer_question_keeps_the_cursor_inline() {
    let rows = shipwright_rows_after(&['Y', 'F', 'N'], Path::new(""));
    assert!(
        rows.iter()
            .any(|row| row.starts_with("Wilt thou take it?") && row.ends_with("No")),
        "{rows:#?}"
    );
    assert!(!rows.iter().any(|row| row == "No"), "{rows:#?}");
}

/// `shops.md` §8.B, arms entry: "Wait for `B`, `S`, or Space; ignore other
/// keys without reprinting". The non-arms `Y`/`N` echo used to fire here too,
/// appending `NoYes` to the greeting row.
#[test]
fn arms_entry_ignores_y_and_n_without_echo() {
    let rows = arms_rows_after(&[' ', 'N', 'Y']);
    assert_eq!(
        rows.last().map(String::as_str),
        Some("\"Hail, friend! Wouldst thou Buy or Sell?\""),
        "{rows:#?}"
    );
}

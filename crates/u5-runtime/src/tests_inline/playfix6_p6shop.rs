// Playtest fixes from the p6shop exploratory pass. Each test names the
// finding it covers.

fn p6shop_enough_drink_state() -> PlayState {
    let mut state = test_state(open_grid(), 15, 15);
    state.active_shop = Some(crate::shop_session::ActiveShopSession::Tavern(
        crate::shop_runtime::TavernState::Menu {
            tavern: Tavern::TheBlueBoarTavern,
            continuation_ready: true,
        },
    ));
    state.tavern_secondary_drink_count = 3;
    state.moral_standing = 5;
    state.gold = 100;
    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    state
}

fn p6shop_row_texts(state: &mut PlayState) -> Vec<String> {
    state.flush_message_slot();
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// K25: `shops.md §8.C` gives the question as ending `to drink?" ` with no
/// line feed, and "Y prints `Yes\n\n` and returns to continuation" - so the
/// echo continues the question's row, then a blank row, then the
/// continuation question.
#[test]
fn p6shop_k25_enough_drink_yes_continues_the_question_row() {
    let mut state = p6shop_enough_drink_state();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    let rows = p6shop_row_texts(&mut state);
    let at = rows
        .iter()
        .position(|row| row == "to drink?\" Yes")
        .unwrap_or_else(|| panic!("no `to drink?\" Yes` row in {rows:?}"));
    assert_eq!(rows[at + 1], "");
    assert_eq!(rows[at + 2], "\"Anything else");
    assert!(!rows.iter().any(|row| row == "Yes"), "{rows:?}");
}

/// K24: "N prints `No!` and proceeds with the purchase and existing drinking
/// effects" - the `No!` lands on the question's row before the wine list.
#[test]
fn p6shop_k24_enough_drink_no_echoes_no_bang_then_proceeds() {
    let mut state = p6shop_enough_drink_state();
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    assert_eq!(state.town_drunkenness_counter, 25);
    assert_eq!(state.moral_standing, 4);
    let rows = p6shop_row_texts(&mut state);
    let at = rows
        .iter()
        .position(|row| row == "to drink?\" No!")
        .unwrap_or_else(|| panic!("no `to drink?\" No!` row in {rows:?}"));
    assert_eq!(rows[at + 1], "\"Our wine list,", "{rows:?}");
}


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

/// The rows after the buy quote's confirmation prompt: the prompt row itself
/// (whichever of the four prompts was drawn) and everything below it.
fn p6shop_arms_confirm_rows(state: &mut PlayState) -> Vec<String> {
    let rows = p6shop_row_texts(state);
    let at = rows
        .iter()
        .rposition(|row| {
            ["buy one?\"", "take it?\"", "Wish ye it?\"", "for thee?\""]
                .iter()
                .any(|prompt| row.contains(prompt))
        })
        .unwrap_or_else(|| panic!("no confirmation prompt row in {rows:?}"));
    rows[at..].to_vec()
}

/// K07: `shops.md §8.A`, "Arms buy decline echo": "`N` prints `No` followed
/// by a blank line", then §8.1's post-item prompt.
#[test]
fn p6shop_k07_arms_buy_decline_echoes_no_and_a_blank_row() {
    let mut state = arms_buy_list_state(500);
    arms_state_after(&mut state, &['a', 'N']);
    let rows = p6shop_arms_confirm_rows(&mut state);
    assert!(rows[0].ends_with("?\" No"), "{rows:?}");
    assert_eq!(rows[1], "", "{rows:?}");
    assert!(rows[2].starts_with("\"Anything else,"), "{rows:?}");
    assert_eq!(state.gold, 500);
}

/// K07/K26: §8.1 "`Y` echoes `Yes`" before the sale.
#[test]
fn p6shop_k26_arms_buy_accept_echoes_yes_before_sold() {
    let mut state = arms_buy_list_state(500);
    arms_state_after(&mut state, &['a', 'Y']);
    let rows = p6shop_arms_confirm_rows(&mut state);
    assert!(rows[0].ends_with("?\" Yes"), "{rows:?}");
    assert_eq!(rows[1], "Sold!", "{rows:?}");
}

/// K26: "`Y` echoes `Yes` and first checks whether the carried equipment
/// counter ... is already `99`" - the echo precedes the carry-cap refusal.
#[test]
fn p6shop_k26_arms_buy_yes_echo_precedes_carry_cap_refusal() {
    let mut state = arms_buy_list_state(500);
    let dagger = ArmsShop::IolosBows.stock_table().item_ids[0];
    state.equipment_stock[usize::from(dagger)] = 99;
    arms_state_after(&mut state, &['a', 'Y']);
    let rows = p6shop_arms_confirm_rows(&mut state);
    assert!(rows[0].ends_with("?\" Yes"), "{rows:?}");
    assert!(rows[1].starts_with("\"Thou canst not carry"), "{rows:?}");
}

/// K26: the echo also precedes the short-funds no-credit bark.
#[test]
fn p6shop_k26_arms_buy_yes_echo_precedes_no_credit_bark() {
    let mut state = arms_buy_list_state(0);
    arms_state_after(&mut state, &['a', 'Y']);
    let rows = p6shop_arms_confirm_rows(&mut state);
    assert!(rows[0].ends_with("?\" Yes"), "{rows:?}");
    assert!(rows.len() > 1 && !rows[1].is_empty(), "{rows:?}");
}

// Residual items from the rc2 integration of the playtest fixes
// (u5-engine#46). Each test names the item it covers.

/// Item 1: the tavern states the row its message leaves from the outcome it
/// printed, never from the message's trailing character. The no-assets menu
/// fallback ends `or Space.` - no closing quote and space - yet the tavern
/// still waits on that row for its letter, which echoes there.
#[test]
fn rc2_tavern_row_state_comes_from_the_outcome_not_the_text() {
    use crate::input_dispatch::tavern_outcome_prompt_row;
    use crate::shop_runtime::{TavernOutcome, TavernState};
    use crate::shop_session::ActiveShopSession;

    let menu = TavernOutcome::EnteredMenu {
        tavern: Tavern::TheCatsLair,
        round_letter: 'A',
        secondary_letter: 'B',
        provisions_letter: None,
        lore_letter: 'C',
    };
    assert_eq!(tavern_outcome_prompt_row(&menu, false), Some(true));
    assert_eq!(
        tavern_outcome_prompt_row(&TavernOutcome::RefusedNoNeed, true),
        Some(false),
        "`yells $.\\n` closes its row"
    );
    assert_eq!(
        tavern_outcome_prompt_row(&TavernOutcome::EnteredSagePrompt, false),
        None
    );
    assert_eq!(
        tavern_outcome_prompt_row(&TavernOutcome::InvalidInput, false),
        None
    );

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Greeting {
        tavern: Tavern::TheCatsLair,
    }));
    state.emit_message_line("\n\"Welcome!\"\n\n:");
    // A stale closed row, which the guess left alone for this text.
    state.close_message_row();
    handle_play_key_input(&mut state, 'Y', "", Path::new("")).unwrap();
    assert!(state.message.ends_with("or Space."), "{:?}", state.message);
    assert!(state.message_row_open_mid_line);

    let round = state
        .message
        .split("drink round (")
        .nth(1)
        .and_then(|rest| rest.chars().next())
        .unwrap();
    handle_play_key_input(&mut state, round, "", Path::new("")).unwrap();
    assert!(
        state
            .message_entries()
            .iter()
            .any(|entry| entry.text.ends_with(&format!("or Space.{round}"))),
        "{:?}",
        state.message_entries()
    );
}

/// Item 2: `shops.md §8.C`, healers: "otherwise print `\n\n"Who needs my
/// aid?" ` and use the party selector. Cancellation adds `No one`, with no
/// trailing line feed", then the continuation question. `No one` belongs on
/// the question's row, not a row of its own.
#[test]
fn rc2_healer_selector_cancel_adds_no_one_after_the_question() {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    three_member_party(&mut state);
    state.gold = 1000;
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::PickService,
        Healer::TheHealersMission,
    ));
    state.emit_message_line("\"What is the nature of thy need?\" ");
    handle_play_key_input(&mut state, 'H', "", Path::new("")).unwrap();
    handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap();

    let rows: Vec<(String, bool)> = state
        .message_entries()
        .iter()
        .map(|entry| (entry.text.clone(), entry.explicit_blank))
        .collect();
    let at = rows
        .iter()
        .position(|(text, _)| text == "\"Who needs my aid?\" No one")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert!(rows[at + 1].0.is_empty(), "{rows:?}");
    assert_eq!(rows[at + 2].0, "\"Is there any other way in which I may");
    assert_eq!(rows[at + 3].0, "aid thee?\" ");
    assert!(!rows.iter().any(|(text, _)| text == "No one"), "{rows:?}");
    assert!(matches!(
        state.active_shop,
        Some(ActiveShopSession::Healer(HealerShopState::Continuation, _))
    ));
}

/// Item 3: `shops.md §8.C`: "The final prompt is `Thy choice?" `", and
/// "Blue Boar wine selection accepted | Uppercase selected letter, then
/// `\n\n"Ah, a fine\nchoice, `, honorific and `.`; ... paid purchase adds
/// `\nEnjoy!"`, then `\n\n`".
#[test]
fn rc2_blue_boar_choice_prompt_and_uppercase_letter_echo() {
    use crate::shop_runtime::TavernState;
    use crate::shop_session::ActiveShopSession;

    let mut state = test_state(open_grid(), 1, 1);
    state.gold = 200;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::Menu {
        tavern: Tavern::TheBlueBoarTavern,
        continuation_ready: false,
    }));
    handle_play_key_input(&mut state, 'W', "", Path::new("")).unwrap();
    assert!(
        state.message.ends_with("\n\nThy choice?\" "),
        "{:?}",
        state.message
    );

    handle_play_key_input(&mut state, 'f', "", Path::new("")).unwrap();
    assert_eq!(state.gold, 102);
    let rows: Vec<String> = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    let at = rows
        .iter()
        .position(|text| text == "Thy choice?\" F")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(
        &rows[at + 1..],
        &[
            "".to_string(),
            "\"Ah, a fine".to_string(),
            "choice, sir.".to_string(),
            "Enjoy!\"".to_string(),
            "".to_string(),
            "\"Anything else".to_string(),
            "for thee?\" ".to_string(),
        ]
    );
}

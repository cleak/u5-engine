// Exploratory playtest round 2, shops (`shops2`): healer, inn, guild,
// herbalist, shipwright and selector row-state findings.

fn pf2_rows(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.trim_end().to_string())
        .collect()
}

fn pf2_keys(state: &mut PlayState, keys: &[char]) {
    for &key in keys {
        handle_play_key_input(state, key, "", Path::new("")).unwrap();
    }
}

fn pf2_row_index(rows: &[String], want: &str) -> usize {
    rows.iter()
        .rposition(|row| row == want)
        .unwrap_or_else(|| panic!("no row {want:?}: {rows:#?}"))
}

/// A Trinsic healer visit already at its service question, with a party of
/// three: Shamino poisoned, Iolo dead.
fn pf2_healer_state() -> PlayState {
    use crate::shop_runtime::HealerShopState;
    use crate::shop_session::ActiveShopSession;
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(6).unwrap(),
        floor: 0,
    };
    three_member_party(&mut state);
    state.gold = 3000;
    state.party[1].status = b'P';
    state.party[2].status = b'D';
    state.party[2].hp = 0;
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::WoundsOfHonour,
    ));
    state.emit_message_line("\n\"Welcome!\" ");
    state.open_message_row();
    // Yes prints the powers line and the service question through the
    // production path, so the question's own tail is what the echo meets.
    pf2_keys(&mut state, &['Y']);
    state
}

/// Playtest F47. `shops.md §8.B`: the question ends `need?" ` (closing quote
/// and space). §8.C: the echo is followed by `\n\n"Who needs my aid?" `, and
/// the paid introduction, the untreatable refusal and the continuation each
/// open with `\n\n` - one blank row each.
#[test]
fn pf2_healer_transcript_takes_one_blank_before_each_reply() {
    let mut state = pf2_healer_state();
    pf2_keys(&mut state, &['C']);
    let rows = pf2_rows(&state);
    let echo = pf2_row_index(&rows, "\"What is the nature of thy need?\" Curing");
    assert_eq!(rows[echo + 1..], ["", "\"Who needs my aid?\""], "{rows:#?}");

    pf2_keys(&mut state, &['2', '\r']);
    let rows = pf2_rows(&state);
    let aid = pf2_row_index(&rows, "\"Who needs my aid?\"");
    assert_eq!(rows[aid + 1], "", "{rows:#?}");
    assert!(
        rows[aid + 2].starts_with("\"I can cure thy poisoned body"),
        "{rows:#?}"
    );

    pf2_keys(&mut state, &['Y']);
    let rows = pf2_rows(&state);
    let paid = rows
        .iter()
        .rposition(|row| row.ends_with("pay?\" Yes"))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert_eq!(rows[paid + 1], "", "{rows:#?}");
    assert_eq!(
        rows[paid + 2], "\"Is there any other way in which I may",
        "{rows:#?}"
    );

    // Heal on the dead Iolo: the refusal sits one blank row under the
    // selector's question.
    let mut state = pf2_healer_state();
    pf2_keys(&mut state, &['H', '3', '\r']);
    let rows = pf2_rows(&state);
    let aid = pf2_row_index(&rows, "\"Who needs my aid?\"");
    assert_eq!(rows[aid + 1], "", "{rows:#?}");
    assert_eq!(rows[aid + 2], "\"Thou hast no need of this art!\"", "{rows:#?}");
}

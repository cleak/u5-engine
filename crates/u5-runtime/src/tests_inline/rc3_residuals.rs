// rc3 residuals after the pass-2 playtest merges (u5-engine#47).

fn rc3_rows(state: &PlayState) -> Vec<String> {
    state
        .message_entries()
        .iter()
        .map(|entry| entry.text.trim_end().to_string())
        .collect()
}

fn rc3_keys(state: &mut PlayState, keys: &[char]) {
    for &key in keys {
        handle_play_key_input(state, key, "", Path::new("")).unwrap();
    }
}

/// `inventory.md §5.1`: Escape in R-Ready's picker prints `Done\n` on the
/// `Item: ` row. The `Item: ` prompt is only a live row while the picker
/// waits, so the completed row has to be logged - it used to live in the
/// slot alone and never reached the transcript.
#[test]
fn rc3_ready_picker_escape_logs_item_done_and_closes_the_row() {
    let mut state = test_state(open_grid(), 1, 1);
    state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
    rc3_keys(&mut state, &['R', '1', '\r', '\u{1b}']);
    assert!(state.active_ready.is_none());
    state.flush_message_slot();
    let rows = rc3_rows(&state);
    assert_eq!(
        rows.iter()
            .filter(|row| row.as_str() == "Item: Done")
            .count(),
        1,
        "{rows:#?}"
    );
    assert!(!rows.iter().any(|row| row.contains("None!")), "{rows:#?}");
    assert!(!state.message_row_open_mid_line);
    // Logged once: the slot holds the completed row as already flushed.
    assert!(!state.flush_message_slot());
}

/// `inventory.md §5.1`: `None!` is the *other* picker caller's Escape
/// literal and R-Ready never prints it; `Done` is the item picker's Escape
/// only. Escape at R-Ready's `Player: ` prompt shows no `Item: ` row and no
/// `None!`.
#[test]
fn rc3_ready_member_escape_prints_neither_item_nor_none() {
    let mut state = test_state(open_grid(), 1, 1);
    state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
    rc3_keys(&mut state, &['R', '\u{1b}']);
    assert!(state.active_ready.is_none());
    state.flush_message_slot();
    let rows = rc3_rows(&state);
    assert!(!rows.iter().any(|row| row.contains("None!")), "{rows:#?}");
    assert!(
        !rows.iter().any(|row| row.starts_with("Item:")),
        "{rows:#?}"
    );
    assert!(!rows.iter().any(|row| row.contains("Done")), "{rows:#?}");
    assert_eq!(
        rows.last().map(String::as_str),
        Some("Player:"),
        "{rows:#?}"
    );
    assert!(!state.message_row_open_mid_line);
    assert!(!state.message.contains("None!"));
}

/// `shops.md §8.C` reagent offer: "If the resulting local column exceeds
/// 12, append `\n:`." The column is the cursor's after the trailing space.
#[test]
fn rc3_reagent_offer_appends_the_colon_only_past_column_twelve() {
    // `"Ab. Is this thy` / `need?" ` - the cursor stands at column 7.
    let short = crate::input_dispatch::reagent_offer_text("Ab.");
    assert_eq!(short, "\n\n\"Ab. Is this thy need?\" ");
    assert!(crate::wrapped_final_row_columns(&short) + 1 <= 12);
    // A record that leaves the question's tail on a row of its own past
    // column 12 takes the colon continuation.
    let long = crate::input_dispatch::reagent_offer_text("Abcdefgh.");
    let question = "\n\n\"Abcdefgh. Is this thy need?\"";
    assert!(
        crate::wrapped_final_row_columns(question) + 1 > 12,
        "{long:?}"
    );
    assert_eq!(long, format!("{question} \n:"));
}

/// `shops.md §8.C` Guild: "D renders record `164`, then redraws the stock
/// menu" - nothing between them. The record (synthetic here) carries the
/// text that continues the `Thy concern?" ` row and its own closing feeds.
#[test]
fn rc3_guild_describe_adds_no_spacing_of_its_own() {
    let record = "x\n\n\"Synthetic.\n\n";
    let text = crate::input_dispatch::guild_described_text(record);
    assert!(text.starts_with(record));
    assert!(text[record.len()..].starts_with("a........."), "{text:?}");
    assert!(!text.contains("\n\n\n"), "{text:?}");
}

/// The same, end to end against the configured original assets (skipped
/// when `U5_TEST_ASSET_DIR` is unset): record `164`'s first text continues
/// the `Thy concern?" ` row, and exactly one blank row stands above the
/// redrawn stock menu.
#[test]
fn rc3_guild_describe_continues_the_prompt_row_with_one_blank_before_the_menu() {
    use crate::shop_runtime::GuildShopState;
    use crate::shop_session::ActiveShopSession;
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(22).unwrap(),
        floor: 0,
    };
    state.gold = 3000;
    state.active_shop = Some(ActiveShopSession::Guild(GuildShopState::Greeting {
        shop: crate::shops::GuildShop::TheGuild,
    }));
    state.emit_message_line("\n\"Welcome!\" ");
    state.open_message_row();
    handle_play_key_input(&mut state, 'Y', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'D', "", &dir).unwrap();
    state.flush_message_slot();
    let rows = rc3_rows(&state);
    let prompt = rows
        .iter()
        .position(|row| row.starts_with("Thy concern?\" ") && row.len() > "Thy concern?\" ".len())
        .unwrap_or_else(|| panic!("record 164 did not continue the prompt row: {rows:#?}"));
    let menu = rows
        .iter()
        .rposition(|row| row.starts_with("a........."))
        .unwrap_or_else(|| panic!("{rows:#?}"));
    assert!(menu > prompt, "{rows:#?}");
    assert_eq!(rows[menu - 1], "", "{rows:#?}");
    assert_ne!(rows[menu - 2], "", "one blank row only: {rows:#?}");
    assert_eq!(
        rows.last().map(String::as_str),
        Some("Thy concern?\""),
        "{rows:#?}"
    );
}

/// Playtest F53 remainder. `containers.md §8` class `0xB4`: the shard
/// subtype "is **masked to two bits**, not validated". Subtypes congruent to
/// 0..2 grant the real shard and print its line; congruent to 3 write the
/// unused byte at `0x0213` and print the Cowardice line, granting nothing.
#[test]
fn rc3_shard_grant_masks_its_subtype_to_two_bits() {
    let shard_flags = SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX..=SPECIAL_ITEM_SHARD_COWARDICE_INDEX;
    for (subtype, flag, line) in [
        (0u8, SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX, "Falsehood"),
        (2, SPECIAL_ITEM_SHARD_COWARDICE_INDEX, "Cowardice"),
        (3, SPECIAL_ITEM_UNUSED_AFTER_SHARDS_INDEX, "Cowardice"),
        (4, SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX, "Falsehood"),
        (5, SPECIAL_ITEM_SHARD_HATRED_INDEX, "Hatred"),
        (0xFF, SPECIAL_ITEM_UNUSED_AFTER_SHARDS_INDEX, "Cowardice"),
    ] {
        let mut state = test_state(open_grid(), 1, 1);
        let z = state.active_objects[0].z;
        state.active_objects.push(ActiveObject {
            type_byte: INVENTORY_ADD_CLASS_SHADOWLORD_SHARD,
            tile: INVENTORY_ADD_CLASS_SHADOWLORD_SHARD,
            x: 2,
            y: 1,
            z,
            phase: 0,
            aux1: subtype,
            aux3: 0,
        });
        let outcome = state.get_native_object_pickup_at(2, 1);
        assert_eq!(outcome, Some(MoveOutcome::Got), "subtype {subtype}");
        assert_eq!(
            state.special_items[flag],
            crate::commands::USE_PICKER_NO_QUANTITY,
            "subtype {subtype}"
        );
        let written = state
            .special_items
            .iter()
            .enumerate()
            .filter(|(_, value)| **value != 0)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        assert_eq!(written, vec![flag], "subtype {subtype}: one byte only");
        assert_eq!(
            shard_flags
                .clone()
                .any(|index| state.special_items[index] != 0),
            subtype & 3 != 3,
            "subtype {subtype}"
        );
        assert!(
            state.message.ends_with(&format!("The Shard of\n{line}!")),
            "subtype {subtype}: {:?}",
            state.message
        );
    }
}

/// Playtest F58 remainder. `endgame.md §8.3`: a narrative window is a hard
/// cut onto the full-screen window. The plain-text frames used to print the
/// prose in the sixteen-column message window beside the stats panel and
/// over the dungeon viewport; they now carry the full-screen page alone.
#[test]
fn rc3_endgame_narrative_text_frames_use_the_full_screen_window() {
    let mut raw = Vec::new();
    let mut table = String::new();
    for index in 0..6 {
        let start = raw.len();
        raw.extend_from_slice(b"{The Avatar returns home across the quiet fields at dusk.");
        raw.push(0);
        table.push_str(&format!("{} {start} {}\n", index + 1, raw.len()));
    }
    let ranges = parse_end_narrative_window_ranges(&table).unwrap();
    let narrative = EndNarrative::new(raw).with_window_ranges(ranges);
    let mut endgame = EndgameState::terminal(
        true,
        true,
        true,
        endgame_certificate_fields("MARIA", GameClock::with_date(141, 5, 6, 12, 0).unwrap()),
        endgame_final_report(GameClock::with_date(141, 5, 6, 12, 0).unwrap()),
        None,
        Some(narrative),
    );
    endgame.advance_cinematic();
    assert!(endgame.advance_cinematic_frame_operation());
    let prose = endgame.current_cinematic_text();
    let mut state = test_state(open_grid(), 1, 1);
    state.party_names = vec![*b"MARIA\0\0\0\0"];
    state.message = prose.clone();
    state.endgame = Some(endgame);
    assert_eq!(
        crate::endgame_narrative_frame::endgame_narrative_window_on_screen(&state),
        Some(0)
    );

    let frame = state.render_text_window_frame(None);
    let rows: Vec<&str> = frame.lines().collect();
    assert_eq!(rows.len(), usize::from(TEXT_SCREEN_ROWS), "{frame}");
    // The prose starts at the page's top-left and uses the full forty
    // columns, so its first row is wider than the message window.
    assert!(rows[0].starts_with("The Avatar returns home"), "{frame}");
    assert!(rows[0].trim_end().len() > 16, "{frame}");
    // Nothing of the gameplay layout survives: no roster name in the panel.
    assert!(!frame.contains("MARIA"), "{frame}");
    let viewport = state.render_text_frame(5);
    assert!(viewport.contains("narrative window 1"), "{viewport}");
}

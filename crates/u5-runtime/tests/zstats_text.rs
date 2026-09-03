//! Batch `zstats-text` regression pins.
//!
//! Every assertion here traces to published spec text quoted at the test,
//! or to a capture of the original marked as a runtime observation.

use std::path::Path;

use u5_runtime::test_fixtures::{
    open_grid, saved_game_seed_bytes, test_state, write_empty_ool_mirrors,
};
use u5_runtime::*;

fn panel_text(state: &PlayState) -> Vec<String> {
    let session = state
        .active_z_stats
        .clone()
        .expect("a live Z-stats page to read");
    state.z_stats_panel_rows(&session)
}

/// `commands.md §5.2` verb-echo table, last row: an unmapped key prints
/// `What?` plus a newline, and "the same text answers a key that is
/// recognised but meaningless in the current mode".
/// `text-output.md §10.3` repeats it and adds that it "consumes no turn".
///
/// The engine printed an `Unhandled command` line naming the raw input
/// code, leaking a hex byte to the player.
#[test]
fn an_unrecognised_key_prints_the_published_refusal_and_no_input_code() {
    for key in ['?', '\r', '\u{1b}', char::from(0xD3)] {
        let mut state = test_state(open_grid(), 5, 5);
        assert_eq!(
            handle_play_key_input(&mut state, key, "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, UNRECOGNISED_COMMAND_MESSAGE);
        assert_eq!(state.message, "What?");
        assert!(!state.message.contains("Unhandled"));
        assert!(!state.message.contains("0x"));
        assert_eq!(state.turn, 0);
    }
}

/// `commands.md §5.3`: a `-` suffix means "a **direction** is awaited.
/// The chosen direction's name is appended on the same line", and §5.4
/// adds that the shared prompt "prints **nothing** before waiting. The
/// hyphen at the end of the verb echo *is* the prompt."
///
/// The engine dropped the direction word, so a capture read `>Open-`
/// rather than the original's `>Open-South`.
#[test]
fn a_prompted_direction_completes_the_open_verb_echo_on_its_own_line() {
    // Every hyphen verb `commands.md` §5.3 lists that this fixture can
    // reach without a game directory, with the two directions the
    // play-test report named spelled out (`Attack-East`, `Open-South`).
    // `complete_open_direction_echo` no-ops when the open line is not
    // exactly the verb literal, so covering the whole family is what
    // keeps a drifting echo literal from silently dropping the
    // direction word again.
    for (key, verb, direction_key, direction) in [
        ('O', "Open-", '2', "South"),
        ('G', "Get-", '2', "South"),
        ('A', "Attack-", '6', "East"),
        ('J', "Jimmy-", '8', "North"),
        ('K', "Klimb-", '4', "West"),
        ('P', "Push-", '2', "South"),
        ('S', "Search-", '6', "East"),
    ] {
        let mut state = test_state(open_grid(), 5, 5);
        assert_eq!(
            handle_play_key_input(&mut state, key, "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.message, verb);

        assert_eq!(
            handle_play_key_input(&mut state, direction_key, "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        let echo = state
            .message_entries()
            .iter()
            .find(|entry| entry.is_command_echo && entry.text.starts_with(verb))
            .map(|entry| entry.text.clone())
            .unwrap_or_default();
        assert_eq!(
            echo,
            format!("{verb}{direction}"),
            "{key} echo was {echo:?}"
        );
    }
}

/// The same rule for `Space`: `commands.md §5.4` gives it `Pass` on the
/// open verb line, "the same word the Pass command echoes".
#[test]
fn a_cancelled_direction_prompt_puts_pass_on_the_open_verb_line() {
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(
        handle_play_key_input(&mut state, 'O', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    let texts = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    assert!(
        texts.iter().any(|text| text == "Open-Pass"),
        "entries were {texts:?}"
    );
}

/// `commands.md §5.2`: the `A` echo is `Attack-` outside dungeons and the
/// `F` echo is `Fire-`; §5.4 says the direction prompt prints nothing of
/// its own. The engine printed `Attack where?` and
/// `Fire- which direction?`, which produced the doubled `Attack-Attack
/// where?` line a capture shows.
#[test]
fn the_attack_prompt_uses_its_published_literal() {
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(
        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.message, "Attack-");
    assert!(!state.message.contains("where?"));
}

/// `inventory.md §4.3`: "Number keys `1` through `6` select directly,
/// bounded by the current party size; the four direction keys move the
/// indicator." Moving "inverts the old row back and inverts the new
/// one", which the panel paints from the selector highlight.
///
/// The engine ignored the direction keys entirely, so the `Select:` bar
/// could not be moved.
#[test]
fn the_party_selector_bar_moves_on_the_four_direction_keys() {
    let mut state = test_state(open_grid(), 5, 5);
    let leader = state.party[0];
    state.party = vec![leader, leader, leader];
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0", *b"IOLO\0\0\0\0\0"];
    state.party_strengths = vec![15, 15, 15];
    state.party_intelligence = vec![15, 15, 15];
    state.party_experience = vec![150, 5, 90];
    assert_eq!(
        handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.selector_highlight(), Some(0));
    assert_eq!(state.roster_box_label().as_deref(), Some("Select:"));
    let party = state.party.len();
    assert!(party >= 2, "fixture needs at least two members");

    let east = char::from(INPUT_CODE_EAST);
    let west = char::from(INPUT_CODE_WEST);
    let north = char::from(INPUT_CODE_NORTH);
    let south = char::from(INPUT_CODE_SOUTH);

    assert_eq!(
        handle_play_key_input(&mut state, east, "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.selector_highlight(), Some(1));
    assert_eq!(
        handle_play_key_input(&mut state, west, "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.selector_highlight(), Some(0));
    // The bar does not run off either end of the travelling party.
    assert_eq!(
        handle_play_key_input(&mut state, north, "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.selector_highlight(), Some(0));
    for _ in 0..8 {
        assert_eq!(
            handle_play_key_input(&mut state, south, "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
    }
    assert_eq!(state.selector_highlight(), Some(party - 1));
    assert!(state.active_party_selector.is_some());
    assert_eq!(state.message, PARTY_SELECTION_PROMPT);
}

/// `commands.md §5.6`: `Player:_` is "colon then exactly one trailing
/// space", so the chosen member's name lands on that same open line -
/// the original renders `Player: Avatar` and then the page loop's own
/// `Status:_` sub-prompt (`inventory.md §4.7`).
#[test]
fn confirming_the_selector_completes_the_player_line_and_opens_status() {
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(
        handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_party_selector.is_none());
    assert!(state.active_z_stats.is_some());
    assert_eq!(state.message, Z_STATS_STATUS_PROMPT);

    let name = state.party_member_display_name(0);
    let texts = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    assert!(
        texts.contains(&format!("{PARTY_SELECTION_PROMPT}{name}")),
        "entries were {texts:?}"
    );
    // The engine's prose dump is gone from the message window entirely.
    assert!(!texts.iter().any(|text| text.contains("Z-stats:")));
}

/// `text-output.md §10.6`: "the visible layout is a log whose final line
/// is being edited - for example `Player: ` followed by the input
/// cursor". The prompt's own line stays open, so no fresh live row is
/// started and the cursor sits one cell past the prompt text.
#[test]
fn an_open_prompt_keeps_the_cursor_on_its_own_line() {
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(
        handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        state.open_prompt_line().as_deref(),
        Some(PARTY_SELECTION_PROMPT)
    );

    let mut log = message_log_from_entries(state.message_entries(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    log.push_output(state.message.as_str());
    let open = state.open_prompt_line();
    let layout = layout_message_window_with_open_prompt(&log, Some(""), open.as_deref());

    let live = layout.rows.last().expect("a placed row");
    // `Player:` is drawn unprefixed from column 24, so its seven cells
    // end in column 30 and the cursor lands in column 32.
    assert_eq!(live.text, "Player:");
    assert!(!live.prefixed);
    assert_eq!(live.column, 24);
    assert_eq!(layout.inline_cursor, Some((32, live.row)));

    // With no open prompt the ordinary live row is placed instead.
    let plain = layout_message_window_with_open_prompt(&log, Some(""), None);
    assert!(plain.inline_cursor.is_none());
    assert!(plain.rows.last().expect("a live row").prefixed);
}

/// `save-load.md §5.2` steps 1, 2 and 8: the handler prints `Save game?`
/// and blocks on a keystroke; on `Y` it "prints `Yes` followed by
/// `Saving...`"; after the write it "prints `Done.`".
///
/// The engine emitted one `Yes. Saving... Done.` line, which the
/// fifteen-cell message window wrapped as `Yes. Saving...` / `Done.`.
#[test]
fn save_prompt_reply_lands_on_the_open_prompt_line() {
    let dir = std::env::temp_dir().join("u5-zstats-text-save-prompt");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("SAVED.GAM"), saved_game_seed_bytes(0, 0, 5, 5)).unwrap();
    write_empty_ool_mirrors(&dir);

    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(state.start_save_game_prompt(), MoveOutcome::Observed);
    assert_eq!(state.message, SAVE_PROMPT_MESSAGE);
    assert_eq!(state.open_prompt_line().as_deref(), Some(SAVE_PROMPT_LINE));
    assert!(state.flush_message_slot());

    state
        .step_active_yes_no_prompt('Y', "", &dir)
        .expect("the save prompt accepts Y");

    let tail = state
        .message_entries()
        .iter()
        .rev()
        .take(3)
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        tail,
        vec![
            SAVE_DONE_MESSAGE.to_string(),
            SAVE_IN_PROGRESS_MESSAGE.to_string(),
            format!("{SAVE_PROMPT_MESSAGE} {SAVE_PROMPT_YES_REPLY}"),
        ]
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// `inventory.md §4.7` attribute page: the published labels are `_Lv-`,
/// `Str=`, `__HP:`, `Int=`, `__HM:`, `Dex=`, `__Ex:` and `____Magic:`,
/// each carrying its own interior spacing, drawn over the panel that
/// `§4.1` gives the roster. The leading glyph is the record's gender
/// byte (`formats/saved-gam.md §3.1`).
///
/// Runtime observations, spec silent: the centred condition line under
/// the name, and the four-cell right-justified value column.
#[test]
fn the_attribute_page_draws_the_published_sheet_over_the_panel() {
    let mut state = test_state(open_grid(), 5, 5);
    state.party[0].hp = 60;
    state.party[0].max_hp = 60;
    state.party[0].mana = 0;
    state.party[0].level = 2;
    state.party[0].status = b'G';
    state.party[0].climb_stat = 15;
    state.party_strengths[0] = 15;
    state.party_intelligence[0] = 15;
    state.party_experience[0] = 150;

    assert_eq!(state.z_stats_for_party(0), MoveOutcome::Observed);
    let rows = panel_text(&state);
    assert_eq!(rows.len(), Z_STATS_PANEL_ROWS);

    let name = state.party_member_display_name(0);
    assert!(rows[0].contains(" Lv-2 "));
    assert!(rows[0].ends_with(&name));
    assert!(rows[0].contains(char::from(SAVE_GENDER_MALE_BYTE)));
    assert_eq!(rows[1].trim(), "Good Health");
    assert_eq!(rows[2], "");
    assert_eq!(rows[3], "Str=15  HP:  60");
    // Runtime observation over `§4.7`'s label table: the capture pairs
    // `__HM:` with maximum hit points and `____Magic:` with the magic
    // points the table assigns to `__HM:`.
    assert_eq!(rows[4], "Int=15  HM:  60");
    assert_eq!(rows[5], "Dex=15  Ex: 150");
    assert_eq!(rows[6], "");
    assert_eq!(rows[7], "    Magic: 0");
    // Every drawn row fits the fifteen content cells of `§4.1`.
    for row in &rows {
        assert!(row.chars().count() <= STATS_PANEL_WIDTH, "row {row:?}");
    }

    // The border band carries the member's name, not a page label.
    assert_eq!(state.roster_box_label().as_deref(), Some(name.as_str()));
    // The message window carries only the page loop's sub-prompt.
    assert_eq!(state.message, Z_STATS_STATUS_PROMPT);
}

/// The page body replaces the roster, the food/gold line and the date
/// line, because `inventory.md §4.7` has the attribute page clear the
/// whole panel and `§4.1` puts all three in that one window.
#[test]
fn a_live_page_replaces_the_whole_panel_and_the_divider_band() {
    let mut state = test_state(open_grid(), 5, 5);
    state.food = 63;
    state.gold = 150;

    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    paint_stats_panel_text_window(&mut system, &state, None);
    let roster = system.screen_rows(b' ').join("\n");
    assert!(roster.contains("F:63"));
    assert!(!gameplay_chrome_content(&state).stats_panel_single_box);

    assert_eq!(state.z_stats_for_party(0), MoveOutcome::Observed);
    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    paint_stats_panel_text_window(&mut system, &state, None);
    let page = system.screen_rows(b' ').join("\n");
    assert!(!page.contains("F:63"), "page frame was {page:?}");
    assert!(page.contains("Lv-"));
    // Runtime observation: with nothing left to divide, the panel is one
    // tall box - the original's stat sheet has no divider band rules.
    assert!(gameplay_chrome_content(&state).stats_panel_single_box);
}

/// `stats-panel.md §4.1`: the roster arrow is drawn "on the row whose
/// slot equals the resident active-player selector". A capture of the
/// original's combat panel shows the acting member's row inverted with
/// no arrow on it and no arrow anywhere else, so the combat round walk
/// must not write that selector - it used to, which put a `0x1A` arrow
/// in the middle of the highlight bar.
#[test]
fn the_combat_round_walk_leaves_the_roster_arrow_alone() {
    let mut state = test_state(open_grid(), 5, 5);
    state.active_player = None;
    state.combat_active = true;
    state.pending_combat_actor_slot = Some(0);

    assert_eq!(state.active_player, None);
    let panel = render_stats_panel(&state, state.active_player);
    assert!(
        !panel.contains(STATS_PANEL_ACTIVE_MARKER_ASCII),
        "combat panel was {panel:?}"
    );
}

/// `inventory.md §4.3`: while the shared selector is live "the currently
/// indicated member is shown by **inverting a rectangle covering the full
/// fifteen content cells of that row** - an exact-width video inversion of
/// screen columns 24 through 38 across the whole of that text row. It is
/// not a cursor character and it does not extend to column 39." Moving the
/// indicator "inverts the old row back and inverts the new one".
///
/// The engine painted no highlight at all, so the `Select:` bar was
/// invisible and arrow keys appeared to do nothing.
#[test]
fn the_selector_inverts_exactly_the_indicated_roster_row() {
    let mut state = test_state(open_grid(), 5, 5);
    let leader = state.party[0];
    state.party = vec![leader, leader];
    state.party_names = vec![*b"AVATAR\0\0\0", *b"SHAMINO\0\0"];
    state.party_strengths = vec![15, 15];
    state.party_intelligence = vec![15, 15];
    state.party_experience = vec![150, 5];

    assert_eq!(
        handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.selector_highlight(), Some(0));

    let inverted_columns = |state: &PlayState, row: u8| {
        let mut system = TextWindowSystem::new();
        configure_play_text_windows(&mut system);
        paint_stats_panel_text_window(&mut system, state, None);
        (24u8..=39)
            .filter(|column| system.cell(*column, row).is_some_and(|cell| cell.inverse))
            .collect::<Vec<_>>()
    };

    // Roster slot 0 is screen row 1.
    assert_eq!(inverted_columns(&state, 1), (24u8..=38).collect::<Vec<_>>());
    assert!(inverted_columns(&state, 2).is_empty());

    assert_eq!(
        handle_play_key_input(&mut state, char::from(INPUT_CODE_SOUTH), "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.selector_highlight(), Some(1));
    assert!(inverted_columns(&state, 1).is_empty());
    assert_eq!(inverted_columns(&state, 2), (24u8..=38).collect::<Vec<_>>());

    // Cancelling puts the panel back to plain, and completes the open
    // `Player:_` line rather than starting a new one (`commands.md §5.6`).
    assert_eq!(
        handle_play_key_input(&mut state, '\u{1b}', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(state.selector_highlight(), None);
    assert!(inverted_columns(&state, 1).is_empty());
    assert!(inverted_columns(&state, 2).is_empty());
    // `commands.md §5.6`: `None!` is "the universal cancel response".
    assert_eq!(state.message, "Player: None!");
    let texts = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.starts_with("Player:"))
            .count(),
        1,
        "the cancel word continues the prompt line: {texts:?}"
    );
}

/// `text-output.md §10.6` publishes the whole key set of the shared
/// active-player picker: "digit keys `1` through `6`, up and down,
/// Return or Space to accept, Escape to cancel, and `0` for 'no active
/// player', which prints `None!` and a newline". `inventory.md §4.3`
/// makes it one surface for "Z-stats, R-Ready, New Order, and the rest",
/// and `inventory.md §5` step 4 has the picker it opens next confirm on
/// "**Enter or Space**".
///
/// The engine mapped Space to *cancel*, so a player who accepted with
/// Space got `Player: None!` instead of the page.
#[test]
fn the_selector_accepts_on_return_or_space_and_cancels_on_escape_or_zero() {
    for accept in ['\r', '\n', ' '] {
        let mut state = test_state(open_grid(), 5, 5);
        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(state.selector_highlight(), Some(0));
        assert_eq!(
            handle_play_key_input(&mut state, accept, "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(
            state.active_z_stats.is_some(),
            "{accept:?} must accept the indicated row"
        );
        assert_eq!(state.message, Z_STATS_STATUS_PROMPT);
        assert_ne!(state.message, "Player: None!");
    }
    for cancel in ['\u{1b}', '0'] {
        let mut state = test_state(open_grid(), 5, 5);
        assert_eq!(
            handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert_eq!(
            handle_play_key_input(&mut state, cancel, "", Path::new("")).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.active_z_stats.is_none(), "{cancel:?} must cancel");
        assert_eq!(state.message, "Player: None!");
    }
}

/// `inventory.md §4.7` publishes the page loop's sub-prompt as
/// `\nStatus:_`, leading newline included, and `text-output.md §10.4`
/// makes a line feed a combined carriage return and line feed: landing
/// on a row the previous line already closed, it leaves one blank row.
///
/// Measured on `playtest/orig/zz/z1.png`: `>Z-stats...` on text row 20,
/// `Player: Avatar` on 21, row 22 blank, `Status:_` and its cursor on
/// 23. The engine drew those three lines on rows 21, 22 and 23 with no
/// blank between them, so the whole window sat one row high.
#[test]
fn the_status_sub_prompt_keeps_one_blank_row_under_the_player_line() {
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(
        handle_play_key_input(&mut state, 'Z', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert_eq!(
        handle_play_key_input(&mut state, '\r', "", Path::new("")).unwrap(),
        PlayInputDisposition::Continue
    );
    assert!(state.active_z_stats.is_some());

    // The blank is stored as its own transcript entry, between the
    // completed `Player:_` line and the sub-prompt.
    let entries = state.message_entries();
    let player = entries
        .iter()
        .position(|entry| entry.text.starts_with(PARTY_SELECTION_PROMPT))
        .expect("the completed Player: line");
    assert!(
        entries[player + 1].explicit_blank,
        "entries were {:?}",
        entries.iter().map(|e| e.text.clone()).collect::<Vec<_>>()
    );

    // And it survives a renderer whose text filter drops empty lines,
    // which is how both the Bevy compositor and `--save-screen` build
    // the window.
    let log = message_log_from_entries(state.message_entries(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    let open = state.open_prompt_line();
    let layout = layout_message_window_with_open_prompt(&log, Some(""), open.as_deref());
    let placed = layout
        .rows
        .iter()
        .map(|row| (row.row, row.text.clone()))
        .collect::<Vec<_>>();
    let prompt = layout.rows.last().expect("the open Status: row");
    assert_eq!(prompt.text, Z_STATS_STATUS_PROMPT.trim_end());
    assert_eq!(prompt.row, 23);
    let above = layout.rows[layout.rows.len() - 2].row;
    assert_eq!(
        above, 21,
        "one blank row must sit between the Player: line and the prompt: {placed:?}"
    );
}

/// `text-output.md §10.2`: every mode loop "1. Emit a line feed into the
/// message window. 2. Draw the right-pointing bracket end-cap ... 3.
/// Read the key", and §10.4: a completed turn "leaves the cursor at
/// column 0 of a fresh row, and the next cycle's leading line feed
/// advances again - producing exactly one blank row after each completed
/// command turn".
///
/// So the live prompt row the turn loop is waiting on always has a blank
/// row above it. Measured on `playtest/orig/qsave2/00_Y.png`,
/// `playtest/orig/walk/07_DOWN.png` and `playtest/orig/exit/06_DOWN.png`:
/// the marker-and-cursor row 23 sits under a blank row 22 in all three,
/// where the engine packed its last output line into row 22.
#[test]
fn the_live_prompt_row_keeps_one_blank_row_above_the_last_output() {
    let mut log = GameplayMessageLog::new();
    log.push_command("Quit:");
    log.push_output("Save game? Yes");
    log.push_output("Saving...");
    log.push_output("Done.");

    let layout = layout_message_window(&log, Some(""));
    let placed = layout
        .rows
        .iter()
        .map(|row| (row.row, row.text.clone()))
        .collect::<Vec<_>>();
    assert_eq!(
        placed,
        vec![
            (18, "Quit:".to_string()),
            (19, "Save game? Yes".to_string()),
            (20, "Saving...".to_string()),
            (21, "Done.".to_string()),
            (23, String::new()),
        ],
        "row 22 must stay blank"
    );

    // Exactly one blank, though: a log that already ends in the blank an
    // earlier `end_turn` stored must not gain a second.
    log.end_turn();
    let layout = layout_message_window(&log, Some(""));
    let rows = layout.rows.iter().map(|row| row.row).collect::<Vec<_>>();
    assert_eq!(rows, vec![18, 19, 20, 21, 23]);
}

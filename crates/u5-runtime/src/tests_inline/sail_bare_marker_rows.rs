// The overworld prompt pair in front of shipboard directions that echo
// nothing: `text-output.md §10.2`'s overworld gate together with
// `vehicles.md §6` / `weather.md §5` as corrected by u5-spec#297.

/// A hoisted frigate on open deep water facing north, with `prior` already in
/// the transcript as the last command's echo.
fn bare_marker_frigate(wind: WindState, hull: u8, prior: &str) -> PlayState {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 100, 100);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: true,
        hull,
        skiffs: 1,
    };
    state.sync_player_object();
    state.wind = wind;
    state.push_message_entry(prior, true);
    assert_eq!(state.overworld_frigate_facing(), Some(Direction::North));
    state
}

/// The window as the headless oracle and the Bevy shell assemble it: the
/// transcript, then a live row whose kind follows the overworld gate.
fn bare_marker_layout(state: &PlayState) -> Vec<(bool, String)> {
    let log = message_log_from_entries(state.message_entries(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    let kind = if overworld_prompt_pair_is_suppressed(state) {
        LiveRowKind::Continuation
    } else {
        LiveRowKind::CommandRow
    };
    let layout = layout_message_window_with_continuation(&log, Some(""), None, false, kind);
    // Rows with their absolute positions folded into explicit blanks, so a
    // gap in the window shows up as a `(false, "")` row.
    let mut rows = Vec::new();
    let mut next_row = None;
    for row in layout.rows {
        if let Some(expected) = next_row {
            for _ in expected..row.row {
                rows.push((false, String::new()));
            }
        }
        next_row = Some(row.row + 1);
        rows.push((row.prefixed, row.text));
    }
    rows
}

/// `sail-calm` (2026-10-16, beat `heading`): with the ship already facing the
/// key and a crosswind, the hoisted key's step is attempted on that same
/// command and the shoal refuses it. The stock game reads `>Yell HOIST!` /
/// blank / `>BREAKING UP!`: the key echoes nothing, so the refusal is written
/// on the marker row the prompt pair drew before the key.
#[test]
fn a_hoisted_key_that_keeps_the_facing_writes_its_refusal_on_the_marker_row() {
    let mut state = bare_marker_frigate(WindState::East, 99, "Yell HOIST!");
    state.grid[world_cell_index(100, 99)] = 0x03;
    handle_play_key_input(&mut state, 'w', "", Path::new("")).unwrap();

    let rows = bare_marker_layout(&state);
    let yell = rows
        .iter()
        .position(|row| row.1 == "Yell HOIST!")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[yell], (true, "Yell HOIST!".to_string()), "{rows:?}");
    assert_eq!(rows[yell + 1], (false, String::new()), "{rows:?}");
    assert_eq!(
        rows[yell + 2],
        (true, "BREAKING UP!".to_string()),
        "the refusal continues the marker row: {rows:?}"
    );
}

/// `sail-heading-echo` (2026-10-16, beats `heading` and `sailing`): a course
/// change under hoisted sail prints `Head West` and `Hull weak!` and re-arms
/// the prompt flag (R526), so an ordinary pair follows - a blank row, then
/// the end cap with the cursor. That row stays on screen while the ship waits
/// upwind and while released steps sail on with nothing to print: the held
/// passes are not commands, and a released step's suppressed prompt draws no
/// new row (`text-output.md §10.2`).
#[test]
fn a_course_change_keeps_its_prompt_row_through_held_and_released_sailing_passes() {
    let mut state = bare_marker_frigate(WindState::West, 40, "Yell HOIST!");
    handle_play_key_input(&mut state, 'a', "", Path::new("")).unwrap();
    assert!(!overworld_prompt_pair_is_suppressed(&state));

    let expected_tail = vec![
        (true, "Head West".to_string()),
        (false, "Hull weak!".to_string()),
        (false, String::new()),
        (true, String::new()),
    ];
    let tail = |state: &PlayState| {
        let rows = bare_marker_layout(state);
        rows[rows.len() - 4..].to_vec()
    };
    assert_eq!(tail(&state), expected_tail);

    let start_x = state.player.x;
    let mut released = 0;
    for _ in 0..12 {
        let before = state.player.x;
        state.idle_wait_pass(None).unwrap();
        if state.player.x != before {
            released += 1;
        }
        assert_eq!(
            tail(&state),
            expected_tail,
            "after {released} released step(s), the prompt row must not move"
        );
    }
    assert!(released >= 2, "an upwind heading still releases: {start_x} -> {}", state.player.x);
}

/// A pass the wind holds back is not a command, so it neither clears nor
/// re-arms the prompt flag; only a released step takes the direction.
#[test]
fn a_held_sailing_pass_leaves_the_overworld_prompt_flag_alone() {
    let mut state = bare_marker_frigate(WindState::Calm, 99, "Yell HOIST!");
    state.sail_cached_direction = Some(Direction::North);
    for _ in 0..6 {
        state.idle_wait_pass(None).unwrap();
        assert!(!overworld_prompt_pair_is_suppressed(&state));
    }
    assert_eq!((state.player.x, state.player.y), (100, 100));
    assert!(state.message_entries().iter().all(|entry| !entry.bare_marker));
}

/// The transcript is capped at [`MESSAGE_TRANSCRIPT_CAPACITY`], and a long
/// session keeps it full. A released step that is refused must still write
/// its refusal on the marker row the pair drew before the release, however
/// many old entries the refusal pushes off the front: a sinking hull pushes
/// `BREAKING UP!` and `Ship sunk!` straight into the transcript, and an
/// insertion index taken before the step then lands after them.
#[test]
fn a_released_refusal_with_a_full_transcript_is_written_on_the_marker_row() {
    let mut state = bare_marker_frigate(WindState::West, 5, "Yell HOIST!");
    for index in 0..MESSAGE_TRANSCRIPT_CAPACITY {
        state.push_message_entry(format!("old {index}"), true);
    }
    // A shoal one tile west of the ship.
    state.grid[world_cell_index(99, 100)] = 0x03;
    handle_play_key_input(&mut state, 'a', "", Path::new("")).unwrap();
    assert_eq!(state.message_entries().len(), MESSAGE_TRANSCRIPT_CAPACITY);

    for _ in 0..12 {
        state.idle_wait_pass(None).unwrap();
        if state.message.contains("BREAKING UP!")
            || state.message_entries().iter().any(|entry| entry.text.contains("BREAKING UP!"))
        {
            break;
        }
    }
    state.flush_message_slot();
    let rows = bare_marker_layout(&state);
    let head = rows
        .iter()
        .position(|row| row.1 == "Head West")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[head + 1], (false, "Hull weak!".to_string()), "{rows:?}");
    assert_eq!(rows[head + 2], (false, String::new()), "{rows:?}");
    assert_eq!(
        rows[head + 3],
        (true, "BREAKING UP!".to_string()),
        "the released refusal continues the marker row: {rows:?}"
    );
}

/// `sail-heading-echo` (stock runs `20261016-204248` and `-222947`, beat
/// `sailing-more`): a released sailing step that runs into a hostile reads
/// `Hull weak!` / blank / the bare `>` marker row / `Attacked!`. The step's
/// own refusal is written on the marker row, but an encounter line starts
/// the row below it.
#[test]
fn an_encounter_after_a_bare_marker_row_starts_the_row_below_it() {
    let mut state = bare_marker_frigate(WindState::West, 40, "Head West");
    state.emit_message_line("Hull weak!\n");
    state.push_explicit_blank_message_entry();
    state.push_overworld_bare_marker_entry(state.message_entries().len());
    state.emit_encounter_line("Attacked!");
    state.flush_message_slot();

    let rows = bare_marker_layout(&state);
    let hull = rows
        .iter()
        .position(|row| row.1 == "Hull weak!")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[hull + 1], (false, String::new()), "{rows:?}");
    assert_eq!(
        rows[hull + 2],
        (true, String::new()),
        "the marker row stands alone: {rows:?}"
    );
    assert_eq!(
        rows[hull + 3],
        (false, "Attacked!".to_string()),
        "the encounter line is on the row below the marker: {rows:?}"
    );

    // The step's own output still continues the marker row.
    let mut state = bare_marker_frigate(WindState::West, 40, "Head West");
    state.push_overworld_bare_marker_entry(state.message_entries().len());
    state.emit_message_line("BREAKING UP!");
    let rows = bare_marker_layout(&state);
    assert!(
        rows.contains(&(true, "BREAKING UP!".to_string())),
        "{rows:?}"
    );
}

/// u5-spec#299 Q8 (`text-output.md §10.2`, "Held passes and where the next
/// line lands"): after a hoisted direction that echoes nothing, "a literal
/// that begins with a line feed, such as `Attacked!` (both copies) or
/// `WHIRLPOOL!`, moves to the row below and leaves the marker row holding
/// only the marker", and "a literal with no leading line feed, such as
/// `BREAKING UP!`, `COLLISION!`, `Docked!` or a pirate's `* BOOOM! *`, is
/// written on the marker row itself".
#[test]
fn the_literal_after_a_bare_marker_row_lands_by_its_own_leading_feed() {
    let mut state = bare_marker_frigate(WindState::West, 40, "Head West");
    state.push_overworld_bare_marker_entry(state.message_entries().len());
    state.emit_overworld_whirlpool_banner();
    state.flush_message_slot();
    let rows = bare_marker_layout(&state);
    let head = rows
        .iter()
        .position(|row| row.1 == "Head West")
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(rows[head + 2], (true, String::new()), "{rows:?}");
    assert_eq!(
        rows[head + 3],
        (false, "WHIRLPOOL!".to_string()),
        "the banner's leading feed takes it to the row below, no blank: {rows:?}"
    );

    for line in ["COLLISION!", "Docked!"] {
        let mut state = bare_marker_frigate(WindState::West, 40, "Head West");
        state.push_overworld_bare_marker_entry(state.message_entries().len());
        state.emit_message_line(line);
        let rows = bare_marker_layout(&state);
        assert!(rows.contains(&(true, line.to_string())), "{line}: {rows:?}");
    }

    let mut state = bare_marker_frigate(WindState::West, 40, "Head West");
    state.push_overworld_bare_marker_entry(state.message_entries().len());
    // `push_impact_line`, the broadside's emitter, is `emit_message_line`.
    state.emit_message_line("* BOOOM! *");
    state.flush_message_slot();
    let rows = bare_marker_layout(&state);
    assert!(
        rows.contains(&(true, "* BOOOM! *".to_string())),
        "the broadside continues the marker row: {rows:?}"
    );
}

/// The window during the drowning descent, as the Bevy shell lays it out: the
/// live row is drawn unless a staged presentation stopped part-way along a
/// row, and it continues the block above it (no blank, no end cap) while the
/// prompt pair is suppressed.
fn drowning_layout(state: &PlayState) -> Vec<(bool, String)> {
    let log = message_log_from_entries(state.message_entries(), |text| {
        (!text.trim().is_empty()).then(|| text.to_string())
    });
    let live = (!state.message_window_live_row_suppressed()).then_some("");
    let layout = layout_message_window_with_continuation(
        &log,
        live,
        None,
        false,
        LiveRowKind::Continuation,
    );
    let mut rows = Vec::new();
    let mut next_row = None;
    for row in layout.rows {
        if let Some(expected) = next_row {
            for _ in expected..row.row {
                rows.push((false, String::new()));
            }
        }
        next_row = Some(row.row + 1);
        rows.push((row.prefixed, row.text));
    }
    rows
}

/// `overworld.md §6.3` step 6: the impact payload "may add `Ship sunk!\n` and
/// then either `Abandon ship!\n` or `DROWNING!!!\n`", and `audio.md §8.9`
/// puts the long descent between the first and the last. Measured
/// 2026-10-20 (`sail-heading-echo`, rc4 sweep `20261020-142358`, beats
/// `sailing` and `sailing-more`, both sides sunk): through the descent the
/// stock window reads `Ship sunk!` and then the blank row its feed opened,
/// where this engine left `Ship sunk!` as the bottom row (`offset-1`).
#[test]
fn ship_sunk_closes_its_row_before_the_drowning_descent() {
    let mut state = bare_marker_frigate(WindState::West, 40, "Head West");
    if let TransportState::Ship { hull, skiffs, .. } = &mut state.player.transport {
        *hull = 1;
        *skiffs = 0;
    }
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 0;
    state.push_overworld_bare_marker_entry(state.message_entries().len());
    state.emit_message_line("BREAKING UP!");
    let OutdoorImpactAbsorption::ShipDestroyed { fallback, .. } = state.apply_outdoor_impact()
    else {
        panic!("hull one is always destroyed");
    };
    assert_eq!(fallback, ShipLossFallback::Drown);
    assert!(state.staged_narration_active());
    // The shell's live-row kind for a staged hold (no end cap, no blank).
    assert!(shop_pause_row_is_continuation(&state));
    state.flush_message_slot();

    let rows = drowning_layout(&state);
    let sunk = rows
        .iter()
        .position(|row| row.1 == SHIP_SUNK_MESSAGE)
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(
        &rows[sunk + 1..],
        &[(false, String::new())],
        "the descent holds the row `Ship sunk!`'s feed opened: {rows:?}"
    );

    // Release only `DROWNING!!!`: the death loop's passes are still held.
    // `overworld.md §6.3` row 6 gives the line as `DROWNING!!!\n`, so it
    // closes its own row and the loop runs with the cursor on the row under
    // it - measured on `sail-calm` (20261024-003707, beat `pass`): the stock
    // window holds an empty row under `DROWNING!!!` while the hit points
    // fall, where this engine left `DROWNING!!!` as the last row.
    let descent = SoundEffect::LongDescent
        .program(&mut crate::audio::RumbleJitter::default())
        .duration(true)
        .as_secs_f64();
    state.advance_staged_narration_to(100.0);
    state.advance_staged_narration_to(100.0 + descent + 0.01);
    assert!(state.staged_narration_active(), "the passes are still held");
    state.flush_message_slot();
    let rows = drowning_layout(&state);
    let sunk = rows
        .iter()
        .position(|row| row.1 == SHIP_SUNK_MESSAGE)
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(
        &rows[sunk + 1..],
        &[(false, DROWNING_MESSAGE.to_string()), (false, String::new())],
        "`DROWNING!!!` on the row `Ship sunk!` opened, then its own feed's row: {rows:?}"
    );

    state.flush_staged_narration();
    state.flush_message_slot();
    let rows = drowning_layout(&state);
    let sunk = rows
        .iter()
        .position(|row| row.1 == SHIP_SUNK_MESSAGE)
        .unwrap_or_else(|| panic!("{rows:?}"));
    assert_eq!(
        rows[sunk + 1],
        (false, DROWNING_MESSAGE.to_string()),
        "`DROWNING!!!` is printed on that row, no blank between: {rows:?}"
    );
}

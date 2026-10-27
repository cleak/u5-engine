// Regressions for the pass-12 shrine exploratory-playtest findings
// (shr12, P12-16 through P12-19).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

fn shr12_rows(state: &PlayState) -> Vec<String> {
    state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect()
}

/// A Britannia party one cell east of the Honesty shrine, which is ruined.
fn shr12_ruined_honesty_state() -> PlayState {
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    let mut world = world_state(open_world_grid(), shrine_x + 1, shrine_y);
    world.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    world.grid[world_cell_index(shrine_x, shrine_y)] = WORLD_RUINED_SHRINE_TILE;
    world.shrine_ruin_flags[0] = 0x81;
    world
}

/// `karma.md §7.1`: the restoration prints the uttered-word line, then
/// `\nUpon what virtue\ndost thou\nmeditate?\n\n:`, then `\nMantra:` three
/// times, and nothing rewrites an earlier row - the window only scrolls
/// (`text-output.md §10`). The session used to write its whole cumulative
/// transcript back into the slot on every answer, which re-logged the
/// question and left stale prompt rows above the uttered-word line (P12-16).
#[test]
fn playtest_p12_16_yell_restoration_logs_each_row_once() {
    let mut world = shr12_ruined_honesty_state();
    world.emit_message_line("Yell what?\n:");
    let before = shr12_rows(&world).len();

    assert_eq!(world.yell_command(Some("fallax")), MoveOutcome::Used);
    assert!(world.active_shrine_restoration.is_some());
    world.flush_message_slot();
    let opened = shr12_rows(&world);
    assert_eq!(opened[before - 1], ":FALLAX");
    // The virtue's answer row is live, not logged.
    assert!(
        opened.ends_with(&["meditate?".to_string(), String::new()]),
        "{opened:?}"
    );
    assert_eq!(world.typed_prompt_echo().as_deref(), Some(":"));

    assert_eq!(world.step_active_shrine_restoration('\r', "honesty"), None);
    assert_eq!(world.typed_prompt_echo().as_deref(), Some("Mantra:"));
    assert_eq!(world.step_active_shrine_restoration('\r', "ahm"), None);
    assert_eq!(world.step_active_shrine_restoration('\r', "ahm"), None);
    assert_eq!(
        world.step_active_shrine_restoration('\r', "ahm"),
        Some(MoveOutcome::Used)
    );
    world.flush_message_slot();
    let rows = shr12_rows(&world);
    // Everything up to the open answer row is untouched...
    assert_eq!(rows[..opened.len()], opened[..], "{rows:?}");
    // ...and each answer, then the banner, is appended beneath it once.
    assert_eq!(
        rows[opened.len()..],
        [
            ":HONESTY",
            "Mantra:AHM",
            "Mantra:AHM",
            "Mantra:AHM",
            "",
            "The Shrine is",
            "restored!",
        ]
        .map(String::from),
        "{rows:?}"
    );
    let (shrine_x, shrine_y) = WORLD_SHRINE_COORDINATES[0];
    assert_eq!(world.grid[world_cell_index(shrine_x, shrine_y)], WORLD_SHRINE_TILE);
    assert!(!world.message_row_open_mid_line);
    assert!(world.typed_prompt_echo().is_none());
}

/// `text-output.md §10.6`: the "General typed string" reader, "used for most
/// free-text prompts", has "keys are uppercased"; `karma.md §7.1`'s four
/// reads are ordinary ones. Matching stays case-insensitive (P12-19).
#[test]
fn playtest_p12_19_yell_restoration_echoes_answers_upper_cased() {
    let mut world = shr12_ruined_honesty_state();
    assert_eq!(world.yell_command(Some("FALLAX")), MoveOutcome::Used);
    assert!(world.typed_prompt_echo_uppercases());
    assert_eq!(world.step_active_shrine_restoration('h', "onesty"), None);
    assert_eq!(world.step_active_shrine_restoration('a', "hm"), None);
    let rows = shr12_rows(&world);
    assert!(
        rows.ends_with(&[":HONESTY".to_string(), "Mantra:AHM".to_string()]),
        "{rows:?}"
    );
    assert_eq!(world.step_active_shrine_restoration('a', "hm"), None);
    assert_eq!(
        world.step_active_shrine_restoration('a', "hm"),
        Some(MoveOutcome::Used)
    );
    assert_eq!(world.shrine_ruin_flags[0], 0x01);
}

// Regressions for the pass-18 clean-room data-handling finding (lit18,
// P18-06).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `karma.md §7` "Entry text and input order": "The authored narration comes
/// from `MISCMSG.DAT`", the Approach being "Record `45`, including its leading
/// newline and trailing blank row", the Kneel "Record `28` ... followed by its
/// blank row" and the question "Record `29`: the question asking which
/// virtue, then a blank row and `:`". The engine printed compiled-in copies of
/// all three, so a file with other text still showed the built-in wording
/// (P18-06). Placeholder records stand in for the shipped ones.
#[test]
fn playtest_p18_06_shrine_entry_text_is_read_from_miscmsg_dat() {
    let dir = debug_game_dir();
    fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
    let mut miscmsg = Vec::new();
    for index in 0..MISCMSG_DAT_RECORDS {
        let record = match index {
            MISCMSG_SHRINE_ENTRY_NARRATION => "\nApproach placeholder.\n\n".to_string(),
            MISCMSG_SHRINE_KNEEL_NARRATION => "Kneel placeholder.\n\n".to_string(),
            MISCMSG_SHRINE_VIRTUE_QUESTION => "Question placeholder?\n\n:".to_string(),
            _ => format!("rec{index}"),
        };
        miscmsg.extend_from_slice(record.as_bytes());
        miscmsg.push(0);
    }
    fs::write(dir.join(MISCMSG_DAT_FILE), miscmsg).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
    let mut state = britannia_state(grid, 10, 20);

    assert_eq!(
        handle_play_key_input(&mut state, 'E', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );
    state.flush_staged_narration();
    assert!(state.active_shrine.is_some());

    let rows: Vec<String> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect();
    let approach = rows
        .iter()
        .position(|row| row == "Approach placeholder.")
        .unwrap_or_else(|| panic!("record 45 not printed: {rows:?}"));
    assert_eq!(
        rows[approach..],
        [
            "Approach placeholder.".to_string(),
            String::new(),
            "Kneel placeholder.".to_string(),
            String::new(),
            "Question placeholder?".to_string(),
            String::new(),
        ],
        "the record's own `:` is the live input row, not a logged one: {rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

// `endgame.md §5`/`§5.1`/`§6`: the message window the Bevy shell draws
// across the Doom final-room dialogue, composed the way the shell composes
// it (transcript, unflushed slot, open prompt, live-row kind).

fn endgame_window_synthetic_messages() -> EndgameMessages {
    // The shipped records' line-feed shapes (`§5.1`'s record table), with
    // placeholder words of the same lengths so the rows wrap alike: record 0
    // opens with one feed, records 1 and 2 end `You reply: `, record 10 opens
    // and closes with a feed.
    let records: [&str; 11] = [
        "\nLord British says:\n\n\"Well met,\n",
        "\"Query word asked of box?\"\n\nYou reply: ",
        "\"Box now!\"\n\n\"Query word asked it?\"\n\nYou reply: ",
        "Rite 1",
        "Rite 2\n\n",
        "Rite 3\n\n",
        "Rite 4\n\n",
        "Rite 5\n\n",
        "Rite 6\n\n",
        "Rite 7\n",
        "\n\"Pull up a chair.\"\n\n\"A while.\"\n",
    ];
    let mut bytes = Vec::new();
    for record in records {
        bytes.extend_from_slice(record.as_bytes());
        bytes.push(0);
    }
    parse_endgame_messages(&bytes).unwrap()
}

fn endgame_window_shell_rows(state: &PlayState) -> Vec<String> {
    let keep = |text: &str| (!text.trim().is_empty()).then(|| text.to_string());
    let mut log = message_log_from_entries(state.message_entries(), keep);
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    if let Some(text) = state
        .message_slot_needs_flush()
        .then(|| keep(&state.message))
        .flatten()
    {
        log.push_output(&text);
    }
    let open_prompt = state.open_prompt_line();
    let live_row = if state.message_window_live_row_suppressed() {
        None
    } else {
        Some("")
    };
    let kind = if endgame_page_row_is_key_wait(state) {
        LiveRowKind::KeyWaitRow
    } else {
        LiveRowKind::CommandRow
    };
    let layout = layout_message_window_with_continuation(
        &log,
        live_row,
        open_prompt.as_deref(),
        combat_prompt_row_follows_history(state),
        kind,
    );
    let mut rows = vec![String::new(); 13];
    for row in &layout.rows {
        let index = usize::from(row.row - MESSAGE_WINDOW_TOP);
        rows[index] = if row.prefixed {
            format!(">{}", row.text)
        } else {
            row.text.clone()
        };
    }
    if let Some((_, row)) = layout.inline_cursor {
        let index = usize::from(row - MESSAGE_WINDOW_TOP);
        rows[index].push('_');
    } else if live_row.is_some()
        && let Some(last) = layout.rows.last()
    {
        let index = usize::from(last.row - MESSAGE_WINDOW_TOP);
        rows[index].push('_');
    }
    rows
}

fn endgame_window_walked_in_state() -> PlayState {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.party_names = vec![*b"Avatar\0\0\0"];
    state.emit_message_line("Avatar, armed\nwith bare hands:");
    state.emit_command_echo_line("North");
    state.emit_message_line("Avatar, armed\nwith bare hands:");
    state.emit_command_echo_line("North");
    state.enter_endgame_with_messages(Some(endgame_window_synthetic_messages()));
    // What the walked-in contact owes the first entry frame (u5-spec#311 Q5).
    state.endgame.as_mut().unwrap().entry_absorption_line =
        Some(crate::combat_absorbed_message("Avatar"));
    state.endgame_entry_redraw_pending = true;
    state
}

fn endgame_window_settle(state: &mut PlayState) {
    for _ in 0..500 {
        if !state.advance_endgame_display_frame() {
            break;
        }
    }
}

fn endgame_window_tail(rows: &[String], count: usize) -> Vec<&str> {
    rows[rows.len() - count..].iter().map(String::as_str).collect()
}

/// Drive the walked-in entry through the greeting page key.
fn endgame_window_at_first_question(dir: &Path) -> PlayState {
    let mut state = endgame_window_walked_in_state();
    endgame_window_settle(&mut state);
    handle_play_key_input(&mut state, 'y', "", dir).unwrap();
    state
}

#[test]
fn endgame_greeting_page_keeps_the_blank_row_under_the_absorption_line() {
    // `endgame.md §5.1`: the absorption line closes its own row, so the
    // greeting's leading feed is one blank row - on the greeting page and
    // still after the page key flushes it. Measured 2026-10-16
    // (`doom-final-room-refusal`, beat `greeting-page`).
    let dir = debug_game_dir();
    let mut state = endgame_window_walked_in_state();
    endgame_window_settle(&mut state);
    let settled = endgame_window_shell_rows(&state);
    // u5-spec#311 Q5: `\nAvatar is absorbed!\n` (wrapped in the sixteen
    // columns), a blank row, then the greeting.
    let greeting = settled.iter().position(|row| row == "Lord British").unwrap();
    assert_eq!(
        &settled[greeting - 3..greeting],
        ["Avatar is", "absorbed!", ""],
        "{settled:?}"
    );
    handle_play_key_input(&mut state, 'y', "", &dir).unwrap();
    let rows = endgame_window_shell_rows(&state);
    assert_eq!(
        endgame_window_tail(&rows, 11),
        [
            "",
            "Lord British",
            "says:",
            "",
            "\"Well met,",
            "Avatar!\"",
            "",
            "\"Query word",
            "asked of box?\"",
            "",
            "You reply:_",
        ]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn endgame_first_no_leaves_one_blank_row_before_the_sandalwood_question() {
    // `§5.1`: the echo, one blank line, then record 2. Measured 2026-10-16
    // (`doom-final-room-refusal`, beats `answered-no` and `after-key`).
    let dir = debug_game_dir();
    let mut state = endgame_window_at_first_question(&dir);
    handle_play_key_input(&mut state, 'n', "", &dir).unwrap();
    let rows = endgame_window_shell_rows(&state);
    assert_eq!(
        endgame_window_tail(&rows, 8),
        [
            "You reply: No",
            "",
            "\"Box now!\"",
            "",
            "\"Query word",
            "asked it?\"",
            "",
            "You reply:_",
        ],
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn endgame_second_no_is_echoed_then_the_lead_in_pauses_before_record_ten() {
    // `§5.1`: the second answer is echoed like the first, then the waiting
    // lead-in, a forty-tick pause, and record 10; `§6`'s terminal branch
    // opens no command row. Measured 2026-10-16 (`doom-final-room-waiting`,
    // beats `waiting`, `tableau1`, `tableau2`).
    let dir = debug_game_dir();
    let mut state = endgame_window_at_first_question(&dir);
    handle_play_key_input(&mut state, 'n', "", &dir).unwrap();
    handle_play_key_input(&mut state, 'n', "", &dir).unwrap();
    let rows = endgame_window_shell_rows(&state);
    assert_eq!(
        endgame_window_tail(&rows, 4),
        ["", "You reply: No", "", "\"I see..."],
        "{rows:?}"
    );
    assert!(!rows.iter().any(|row| row.contains("Pull up")), "{rows:?}");
    // The pause frame owes the forty-tick hold before the record prints.
    assert!(state.advance_endgame_display_frame());
    assert!(state.staged_narration_active());
    assert!(!endgame_window_shell_rows(&state).iter().any(|row| row.contains("Pull up")));
    assert!(state.advance_endgame_display_frame());
    let rows = endgame_window_shell_rows(&state);
    assert_eq!(
        endgame_window_tail(&rows, 10),
        [
            "",
            "You reply: No",
            "",
            "\"I see...",
            "",
            "\"Pull up a",
            "chair.\"",
            "",
            "\"A while.\"",
            "",
        ],
        "{rows:?}"
    );
    // A key during the terminal loop prints nothing.
    endgame_window_settle(&mut state);
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert_eq!(endgame_window_shell_rows(&state), rows);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn endgame_first_yes_without_the_box_ends_on_record_ten_with_no_command_row() {
    // Measured 2026-10-16 (`doom-final-room`, beat `answered`, reading
    // `offset+1`): record 10's last row sits above one empty row, with no
    // end-capped cursor row beneath it.
    let dir = debug_game_dir();
    let mut state = endgame_window_at_first_question(&dir);
    handle_play_key_input(&mut state, 'y', "", &dir).unwrap();
    endgame_window_settle(&mut state);
    let rows = endgame_window_shell_rows(&state);
    assert_eq!(
        endgame_window_tail(&rows, 9),
        [
            "You reply: Yes",
            "",
            "\"I see...",
            "",
            "\"Pull up a",
            "chair.\"",
            "",
            "\"A while.\"",
            "",
        ]
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn endgame_waiting_branch_draws_no_cursor_after_record_ten() {
    // `§6`: the waiting branch is terminal and reads no key, so no cursor
    // follows record 10's last row - on the answering frame, after the
    // record, through the tableau walk and in the terminal jitter.
    // Measured 2026-10-16 on rc1
    // (`doom-final-room/answered`, `doom-final-room-waiting/waiting` and
    // `tableau1`): stock leaves the cell after the closing `ENDMSG.DAT` row empty.
    let dir = debug_game_dir();
    for answers in [&['y'][..], &['n', 'n'][..]] {
        let mut state = endgame_window_at_first_question(&dir);
        for &answer in answers {
            handle_play_key_input(&mut state, answer, "", &dir).unwrap();
        }
        let mut frames = 0;
        loop {
            assert!(state.message_window_cursor_suppressed(), "{answers:?}: frame {frames}");
            // A row left open mid-line must not bring the cursor back.
            let open = state.message_row_open_mid_line;
            state.message_row_open_mid_line = true;
            assert!(state.message_window_cursor_suppressed(), "{answers:?}: open row");
            state.message_row_open_mid_line = open;
            frames += 1;
            if frames > 500 || !state.advance_endgame_display_frame() {
                break;
            }
        }
        assert_eq!(
            state.endgame.as_ref().unwrap().refusal_phase,
            EndgameRefusalPhase::Jitter
        );
        // Between jitter beats nothing is staged, which is when the shell
        // drew the cursor.
        state.flush_staged_narration();
        assert!(!state.staged_narration_active());
        assert!(state.message_window_cursor_suppressed(), "{answers:?}: jitter");
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
        state.flush_staged_narration();
        assert!(state.message_window_cursor_suppressed(), "{answers:?}: after a key");
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn endgame_waiting_branch_walks_the_tableau_after_record_ten() {
    // `§6` steps 1-2, staged as display frames after the record: the
    // leader's nudge carries one movement beat, then Lord British walks to
    // the bed and the leader to the chair, each step with its beat.
    let dir = debug_game_dir();
    let mut state = endgame_window_at_first_question(&dir);
    handle_play_key_input(&mut state, 'y', "", &dir).unwrap();
    let leader = state.active_objects[0];
    let king = state.active_objects[ENDGAME_TABLEAU_LORD_BRITISH_SLOT];
    assert_eq!((king.x, king.y), (5, 3), "nothing moves on the answering key");
    assert_eq!((leader.x, leader.y), (5, 5));
    assert!(state.advance_endgame_display_frame()); // pause
    assert!(state.advance_endgame_display_frame()); // record 10
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (5, 5)
    );
    assert!(state.advance_endgame_display_frame()); // nudge
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (5, 4)
    );
    assert!(state.staged_narration_active(), "the nudge carries a sting beat");
    endgame_window_settle(&mut state);
    let king = state.active_objects[ENDGAME_TABLEAU_LORD_BRITISH_SLOT];
    assert_eq!((king.x, king.y), (4, 1));
    assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (8, 4));
    assert_eq!(
        state.endgame.as_ref().unwrap().refusal_phase,
        EndgameRefusalPhase::Jitter
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn endgame_absorption_and_greeting_holds_bracket_the_tableau_walk() {
    // `endgame.md §5.2`'s published holds (see the constants): the absorption
    // is followed by the untimed block's thirty-three-tick stand-in before
    // Lord British's forty paced iterations, and the greeting by forty paced
    // iterations after the last member's three-iteration tail.
    let mut state = endgame_window_walked_in_state();
    assert!(state.advance_endgame_display_frame());
    assert!(state.staged_narration_active(), "absorption hold");
    assert!(state.endgame.as_ref().unwrap().entry_lord_british_pause_owed);
    let mut greeting_frame = None;
    for frame in 0..500 {
        if !state.advance_endgame_display_frame() {
            break;
        }
        if greeting_frame.is_none() && state.message.contains("Well met") {
            greeting_frame = Some(frame);
        }
        if state.endgame.as_ref().unwrap().entry_party_slot == ENDGAME_ENTRY_GREETING_SLOT {
            assert!(!state.message.contains("Well met"), "greeting waits out its hold");
        }
    }
    assert!(greeting_frame.is_some());
    assert_eq!(ENDGAME_ABSORPTION_HOLD_BIOS_TICKS, 33);
    assert_eq!(ENDGAME_GREETING_HOLD_BIOS_TICKS, 40);
}

#[test]
fn endgame_tableau_actors_on_the_bed_and_chair_draw_occupied() {
    // `visibility.md §8`'s default helper, applied to the tableau grid.
    let mut grid = vec![0x44u8; TOWN_GRID_BYTES];
    grid[TOWN_GRID_SIDE + 4] = 0xAB;
    grid[4 * TOWN_GRID_SIDE + 8] = 0x92;
    grid[5 * TOWN_GRID_SIDE + 8] = 0x9B;
    let mut king = ActiveObject::empty();
    king.type_byte = ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE;
    king.tile = ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE;
    king.x = 4;
    king.y = 1;
    assert_eq!(endgame_tableau_actor_drawn_byte(&grid, king), Some(0x1A));
    king.y = 3;
    assert_eq!(
        endgame_tableau_actor_drawn_byte(&grid, king),
        Some(ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE)
    );
    let mut leader = ActiveObject::empty();
    leader.type_byte = 0x4C;
    leader.tile = 0x4C;
    leader.x = 8;
    leader.y = 4;
    assert_eq!(endgame_tableau_actor_drawn_byte(&grid, leader), Some(0x32));
}

#[test]
fn endgame_tableau_script_a_steps_frames_then_snaps_or_restarts() {
    // `active-objects.md §8` script A - f2, f3, f4, snap, restart - as the
    // tableau sprites run it (`endgame.md §4`, issue #298 Q43).
    let mut king = ActiveObject::empty();
    king.type_byte = ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE;
    king.tile = ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE;
    let never = || -> bool { panic!("only the snap step draws") };
    let mut frames = Vec::new();
    for _ in 0..3 {
        endgame_tableau_script_a_decision(&mut king, never);
        frames.push(king.tile);
    }
    assert_eq!(frames, [0x7D, 0x7E, 0x7F], "tiles 381, 382, 383");
    // A refused snap (three times in four) restarts at frame 2 in the same pass.
    endgame_tableau_script_a_decision(&mut king, || false);
    assert_eq!(king.tile, 0x7D);
    assert_eq!(king.phase & 0x0f, 0);
    endgame_tableau_script_a_decision(&mut king, never);
    endgame_tableau_script_a_decision(&mut king, never);
    // An accepted snap returns him to the seated 380 and holds six ticks,
    // with the position kept on the snap step.
    endgame_tableau_script_a_decision(&mut king, || true);
    assert_eq!(king.tile, ENDGAME_TABLEAU_LORD_BRITISH_ACTOR_BYTE);
    assert_eq!(king.phase & 0x0f, ENDGAME_TABLEAU_SNAP_HOLD_TICKS);
    assert_eq!(
        endgame_tableau_role_for_slot(ENDGAME_TABLEAU_LORD_BRITISH_SLOT, king),
        Some(EndgameTableauActorRole::LordBritish)
    );
}

#[test]
fn endgame_lord_british_and_the_party_animate_while_the_box_stays_static() {
    // `endgame.md §4` (issue #298 Q43, `RETRACTIONS.md` R559): the tableau
    // sprites run on the ordinary per-slot animator through every hold and
    // key wait; the box and the Orb lie below the animator's class floor.
    let mut state = endgame_window_walked_in_state();
    endgame_window_settle(&mut state);
    assert!(state.endgame.as_ref().unwrap().greeting_page_pending);
    if let Some(slot) = state.active_objects.get_mut(ENDGAME_TABLEAU_BOX_SLOT) {
        *slot = endgame_tableau_object_from_placement(endgame_tableau_box_placement());
    }
    let mut king_tiles = std::collections::BTreeSet::new();
    let mut leader_tiles = std::collections::BTreeSet::new();
    for _ in 0..200 {
        state.animate_endgame_tableau_sprites();
        let king = state.active_objects[ENDGAME_TABLEAU_LORD_BRITISH_SLOT];
        assert_eq!(
            endgame_tableau_role_for_slot(ENDGAME_TABLEAU_LORD_BRITISH_SLOT, king),
            Some(EndgameTableauActorRole::LordBritish)
        );
        king_tiles.insert(king.tile);
        leader_tiles.insert(state.active_objects[0].tile);
        assert_eq!(
            state.active_objects[ENDGAME_TABLEAU_BOX_SLOT].tile,
            ENDGAME_TABLEAU_BOX_ACTOR_BYTE
        );
    }
    assert_eq!(king_tiles.into_iter().collect::<Vec<_>>(), [0x7C, 0x7D, 0x7E, 0x7F]);
    assert!(leader_tiles.len() > 1, "the party sprite cycles too");
}

#[test]
fn endgame_tableau_animation_runs_one_world_tick_per_bios_tick() {
    // The wall-clock pump: the first look anchors, later looks run one
    // animator tick per BIOS tick, and nothing runs while the absorption
    // hold still shows the arena (`endgame.md §5.2`).
    let mut state = endgame_window_walked_in_state();
    endgame_window_settle(&mut state);
    let tick = crate::narration::BIOS_TICK_SECS;
    assert!(!state.advance_endgame_tableau_animation_to(10.0 * tick + tick / 2.0));
    let prng_before = state.prng_state;
    state.advance_endgame_tableau_animation_to(10.0 * tick + tick / 2.0);
    assert_eq!(state.prng_state, prng_before, "no tick has passed");
    state.advance_endgame_tableau_animation_to(12.0 * tick + tick / 2.0);
    assert_ne!(state.prng_state, prng_before, "two ticks ran the animator");

    let mut arena = endgame_window_walked_in_state();
    arena.endgame_absorption_arena = Some(Box::new(arena.clone()));
    assert!(!arena.advance_endgame_tableau_animation_to(0.0));
    assert!(!arena.advance_endgame_tableau_animation_to(100.0));
    assert!(arena.endgame.as_ref().unwrap().tableau_animation_tick.is_none());
}

#[test]
fn endgame_terminal_jitter_round_costs_four_paced_iterations() {
    // `endgame.md §6` (issue #298 Q42): "a terminal round of four calls always
    // costs four paced iterations whatever the party size" - one world tick
    // and a one-tick wait per call, empty slots included.
    let dir = debug_game_dir();
    let mut state = endgame_window_at_first_question(&dir);
    handle_play_key_input(&mut state, 'y', "", &dir).unwrap();
    state.finish_endgame_refusal_staging();
    state.flush_staged_narration();
    assert!(!state.staged_narration_active());
    state.advance_endgame_terminal_tableau_jitter();
    // One paced iteration per call, between the calls rather than one block
    // after the round.
    let holds: Vec<f64> = state.staged_narration.beats().map(|beat| beat.delay_secs).collect();
    assert_eq!(holds.len(), 4);
    for hold in holds {
        assert!((hold - crate::narration::bios_ticks_secs(1)).abs() < 1e-9);
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn endgame_waiting_walk_steps_one_actor_per_beat_in_round_order() {
    // `endgame.md §6` step 2 (issue #298 Q42): "Each round is one call per
    // actor in exactly that order (slot 2, slot 31, slot 0), and every actual
    // one-cell step carries its own two-iteration, sting, three-iteration
    // beat". One walk frame therefore moves exactly one actor and leaves its
    // beat's hold pending, so no two actors step at the same instant.
    let dir = debug_game_dir();
    let mut state = endgame_window_at_first_question(&dir);
    handle_play_key_input(&mut state, 'y', "", &dir).unwrap();
    assert!(state.advance_endgame_display_frame()); // pause
    assert!(state.advance_endgame_display_frame()); // record 10
    assert!(state.advance_endgame_display_frame()); // nudge
    let walkers = [2usize, ENDGAME_TABLEAU_LORD_BRITISH_SLOT, 0];
    let positions = |state: &PlayState| {
        walkers.map(|slot| (state.active_objects[slot].x, state.active_objects[slot].y))
    };
    let mut movers = Vec::new();
    loop {
        state.flush_staged_narration();
        let before = positions(&state);
        if !state.advance_endgame_refusal_staging_frame() {
            assert_eq!(positions(&state), before, "the closing round steps nobody");
            break;
        }
        let after = positions(&state);
        let moved: Vec<usize> = (0..walkers.len()).filter(|&i| before[i] != after[i]).collect();
        assert_eq!(moved.len(), 1, "one actor per walk frame");
        assert!(state.staged_narration_active(), "each step carries its beat");
        movers.push(walkers[moved[0]]);
    }
    assert!(!movers.is_empty());
    // Lord British's first step precedes the leader's first step, and any
    // slot-2 step of the first round precedes both.
    let first_king = movers.iter().position(|&slot| slot == ENDGAME_TABLEAU_LORD_BRITISH_SLOT);
    let first_leader = movers.iter().position(|&slot| slot == 0);
    if let (Some(king), Some(leader)) = (first_king, first_leader) {
        assert!(king < leader);
    }
    if movers.contains(&2) {
        assert_eq!(movers[0], 2);
    }
    assert_eq!(
        state.endgame.as_ref().unwrap().refusal_phase,
        EndgameRefusalPhase::Jitter
    );
    let _ = fs::remove_dir_all(dir);
}

/// `endgame.md §3` "The viewport ribbons are left as the dungeon drew them"
/// (added 2026-10-25, u5-spec#300 Q5): the top band keeps the final room's
/// level label (`L8` on the deepest Doom level, zero-based seven) and the
/// bottom band the `Dir:` facing the party fell in with; nothing blanks them
/// or draws an endgame caption.
#[test]
fn spec300_q5_endgame_keeps_the_dungeon_level_and_facing_bands() {
    let mut state = dungeon_state(open_dungeon_record(), 7, 1, 1);
    state.player.facing = Direction::North;
    state.party_names = vec![*b"Avatar\0\0\0"];
    state.enter_endgame_with_messages(Some(endgame_window_synthetic_messages()));
    endgame_window_settle(&mut state);

    let chrome = gameplay_chrome_content(&state);
    assert_eq!(chrome.top, ChromeGap::Label("L8".to_string()));
    assert_eq!(chrome.bottom, ChromeGap::Label(dungeon_direction_label("North")));
}

/// Playtest S34. `commands.md §5.1` gives the end-cap triangle to the turn
/// loop, and the endgame runs none: `endgame.md §5.1` reads the rite pages
/// through the ordinary reader, like the greeting's wait. The box hand-over
/// (`§7` steps 4-5) and the forty-iteration pause after record `3` read no
/// key at all, so they draw no cursor either.
#[test]
fn endgame_victory_rite_draws_no_command_row_and_no_cursor_while_it_reads_no_key() {
    let dir = debug_game_dir();
    let mut state = test_state(endgame_tableau_test_grid(), 5, 5);
    state.party_names = vec![*b"Avatar\0\0\0"];
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.enter_endgame_with_messages(Some(endgame_window_synthetic_messages()));
    state.finish_endgame_entry_presentation();
    state.flush_staged_narration();
    state.resolve_endgame_confirmation(true);
    let no_command_row = |state: &PlayState, beat: &str| {
        assert!(endgame_page_row_is_key_wait(state), "{beat}");
        let rows = endgame_window_shell_rows(state);
        assert!(!rows.iter().any(|row| row.starts_with('>')), "{beat}: {rows:?}");
    };
    // The automatic stretch: hand-over, box, record 3 and its pause, up to
    // the frame that prints `He says:` and the second page.
    let mut frames = 0;
    while state.endgame.as_ref().unwrap().cinematic.step
        == crate::endgame_cinematic::EndgameCinematicStep::RiteMessage(0)
    {
        no_command_row(&state, &format!("automatic frame {frames}"));
        assert!(
            state.message_window_cursor_suppressed(),
            "automatic frame {frames} reads no key"
        );
        state.flush_staged_narration();
        frames += 1;
        assert!(frames < 500);
        assert!(state.advance_endgame_display_frame());
    }
    assert!(frames > 40, "the forty-iteration pause ran on display frames");
    let endgame = state.endgame.as_ref().unwrap();
    assert_eq!(endgame.cinematic.rite_pause_ticks_remaining, 0);
    assert_eq!(endgame.victory_staging, EndgameVictoryStagingPhase::Inactive);
    // The key-paced pages read a key: a bare cursor, still no triangle.
    for page in 0..8 {
        if state.endgame.as_ref().unwrap().victory_tableau_phase
            != EndgameVictoryTableauPhase::Inactive
        {
            break;
        }
        no_command_row(&state, &format!("page {page}"));
        assert!(!state.message_window_cursor_suppressed(), "page {page} reads a key");
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
        state.flush_staged_narration();
        while state.advance_endgame_display_frame() {
            state.flush_staged_narration();
        }
    }
    let _ = fs::remove_dir_all(dir);
}

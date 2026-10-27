// u5-spec#304 Q4 (R628, R629): which presentation key waits go through the
// shared wait - one world step per no-key pass - and which do not.
//
// `input.md §2`: the shrine's typed virtue reader, three mantra readers, the
// ordaining arm's two command-key waits, the offering chooser and the Codex
// urn's key waits step; so do the endgame's ten shared-wait sites (greeting,
// both box questions, six rite pages, the Orb wait). The endgame's six final
// narrative page waits are raw-key tight loops and do not. The counted holds
// between the reads read no key at all.

/// A Britannia state on a Honesty shrine marker, at the virtue question with
/// the entry's holds spent.
fn shared_wait_shrine_at_virtue(dir: &Path) -> PlayState {
    fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
    let mut state = britannia_state(grid, 10, 20);
    handle_play_key_input(&mut state, 'E', "", dir).unwrap();
    state.flush_staged_narration();
    state
}

/// One pass at an open shrine or Codex read is the presentation's own step:
/// the same state a hold step leaves, draws included.
fn assert_pass_is_one_presentation_step(state: &mut PlayState, beat: &str) {
    assert!(state.prompt_waits_in_shared_wait(), "{beat}: the read is the shared wait");
    let mut expected = state.clone();
    expected.advance_presentation_world_step();
    let (prng, frame) = (state.prng_state, state.animation.frame);
    assert!(state.prompt_wait_pass(), "{beat}");
    assert_eq!(state.prng_state, expected.prng_state, "{beat}: the hold step's draws");
    assert_eq!(state.animation.frame, expected.animation.frame, "{beat}");
    assert_ne!(state.animation.frame, frame, "{beat}: the clock moved");
    assert_ne!(state.prng_state, prng, "{beat}: the wind check draws");
}

#[test]
fn spec304_q4_every_shrine_read_steps_the_presentation_on_each_no_key_pass() {
    let dir = debug_game_dir();
    let mut state = shared_wait_shrine_at_virtue(&dir);
    assert_eq!(
        state.active_shrine.as_ref().map(|session| session.phase),
        Some(ShrinePhase::Virtue)
    );
    assert_pass_is_one_presentation_step(&mut state, "virtue reader");
    // A typed answer is untouched: a pass is not a key.
    handle_play_key_input(&mut state, 'H', "ON", &dir).unwrap();
    assert_pass_is_one_presentation_step(&mut state, "virtue reader, part typed");
    assert_eq!(state.active_shrine.as_ref().unwrap().virtue_buffer, "HON");
    for phase in [
        ShrinePhase::Mantra,
        ShrinePhase::AltarAnnouncement,
        ShrinePhase::AltarQuest,
        ShrinePhase::Offering,
    ] {
        state.active_shrine.as_mut().unwrap().phase = phase;
        assert_pass_is_one_presentation_step(&mut state, &format!("{phase:?}"));
    }
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn spec304_q4_shrine_holds_read_no_key_and_take_no_extra_step() {
    let dir = debug_game_dir();
    fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 25\n").unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = SHRINE_MARKER_TILE;
    let mut state = britannia_state(grid, 10, 20);
    handle_play_key_input(&mut state, 'E', "", &dir).unwrap();
    // The walk and the ten-step question hold are queued: `karma.md §7`, "no
    // hold has an abort-on-key path, and none reads the keyboard at all".
    assert!(state.staged_narration_active());
    assert!(state.active_shrine.is_some());
    assert!(!state.prompt_waits_in_shared_wait());
    let before = state.clone();
    assert!(!state.prompt_wait_pass());
    assert_eq!(state.prng_state, before.prng_state);
    assert_eq!(state.animation.frame, before.animation.frame);
    // The third mantra's hold, which runs its branch once drained.
    state.flush_staged_narration();
    let session = state.active_shrine.take().unwrap();
    state.pending_shrine_third_mantra = Some(session);
    assert!(!state.prompt_waits_in_shared_wait());
    assert!(!state.prompt_wait_pass());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn spec304_q4_the_codex_reader_steps_and_no_scene_exempts_the_reads() {
    // `karma.md §8.2`: the urn's key waits "run one world step per no-key
    // pass". `§7`: "An implementation must not skip these steps in any
    // scene, the Abyss included" - the suspended scene value is in force.
    let mut codex = britannia_state(open_world_grid(), 10, 20);
    codex.codex_presentation_steps = 1;
    assert_pass_is_one_presentation_step(&mut codex, "Codex reader");
    codex.codex_presentation_steps = 0;
    assert!(!codex.prompt_waits_in_shared_wait(), "no reader open");

    // A first-person dungeon scene lies inside the no-step band, where an
    // ordinary prompt steps nothing; the shrine's reads still step.
    let mut dungeon = dungeon_state(open_dungeon_record(), 0, 1, 1);
    dungeon.active_shrine = Some(ShrineSession::entering(ShrineVirtue::Spirituality));
    assert_pass_is_one_presentation_step(&mut dungeon, "a dungeon-band scene");
}

#[test]
fn spec304_q4_the_yell_restoration_readers_step_like_any_typed_word() {
    // `karma.md §7.1`'s four typed reads run from the outdoor Yell handler,
    // outside any presentation: typed words, which `input.md §2` puts on the
    // shared wait, at the overworld scene value.
    let mut state = britannia_state(open_world_grid(), 10, 20);
    state.active_shrine_restoration = Some(crate::z_stats::ShrineRestorationSession {
        word_index: 0,
        target_x: 10,
        target_y: 21,
        response_index: 0,
        all_responses_match: true,
        transcript: String::new(),
        buffer: String::new(),
    });
    assert!(state.prompt_waits_in_shared_wait());
    let frame = state.animation.frame;
    assert!(state.prompt_wait_pass());
    assert_ne!(state.animation.frame, frame);
}

/// The virtue answer's Return leaves its row open through the six-step hold;
/// the handler's newline and the ask's leading feed print when it ends.
#[test]
fn spec304_virtue_return_holds_both_feeds_until_the_six_step_wait_ends() {
    let dir = debug_game_dir();
    let mut state = shared_wait_shrine_at_virtue(&dir);
    handle_play_key_input(&mut state, 'H', "ONESTY", &dir).unwrap();
    let entries_before = state.message_entries().len();
    handle_play_key_input(&mut state, '\r', "", &dir).unwrap();
    // `karma.md §7`: "Return submits without echoing a newline". The answer
    // row is logged and stays the cursor's row; nothing under it yet.
    let texts: Vec<String> = state
        .message_entries()
        .iter()
        .skip(entries_before)
        .map(|entry| entry.text.clone())
        .collect();
    assert_eq!(texts, [":HONESTY"], "only the answer row so far");
    assert!(state.message_row_open_mid_line, "the answer's row is still open");
    assert!(state.staged_narration_active(), "the six-step hold is pending");
    assert!(state.message_window_live_row_suppressed(), "no fresh row under it");
    assert!(state.message_window_cursor_suppressed(), "the hold reads no key");
    assert_eq!(state.shrine_prompt_echo(), None, "the ask is not on screen yet");
    let beat = state.staged_narration.beats().next().unwrap().clone();
    assert_eq!(beat.world_steps, SHRINE_MANTRA_FIRST_WORLD_STEPS);
    assert!(beat.continues_open_row);
    // The hold ends: §12's "emit a newline" closes the row and `\nMantra:`'s
    // leading feed leaves one blank above the live mantra row.
    state.flush_staged_narration();
    let texts: Vec<String> = state
        .message_entries()
        .iter()
        .skip(entries_before)
        .map(|entry| entry.text.clone())
        .collect();
    assert_eq!(texts, [":HONESTY", ""], "the answer row, then one blank");
    assert!(!state.message_row_open_mid_line);
    assert_eq!(state.shrine_prompt_echo().as_deref(), Some("Mantra:"));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn spec304_q4_the_endgame_greeting_and_box_questions_step_the_tableau() {
    let dir = debug_game_dir();
    let mut state = endgame_window_walked_in_state();
    endgame_window_settle(&mut state);
    state.flush_staged_narration();
    // The greeting's any-key read.
    assert!(state.endgame.as_ref().unwrap().greeting_page_pending);
    assert!(state.endgame_key_wait_in_shared_wait());
    let mut expected = state.clone();
    expected.endgame_shared_wait_step();
    assert!(state.prompt_wait_pass());
    assert_eq!(state.prng_state, expected.prng_state, "the tableau step's draws");
    assert_eq!(state.active_objects, expected.active_objects);
    // The wall-clock tableau pump yields to the wait's own step, so the
    // sprites are not stepped twice a tick, and it does not catch up later.
    let tick = crate::narration::BIOS_TICK_SECS;
    let prng = state.prng_state;
    assert!(!state.advance_endgame_tableau_animation_to(10.0 * tick));
    assert!(!state.advance_endgame_tableau_animation_to(20.0 * tick));
    assert_eq!(state.prng_state, prng);
    assert_eq!(
        state.endgame.as_ref().unwrap().tableau_animation_tick,
        Some(20),
        "re-anchored"
    );
    // The first box question, then the clarification after a No.
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert!(!state.endgame.as_ref().unwrap().greeting_page_pending);
    assert!(state.endgame_key_wait_in_shared_wait(), "first box question");
    handle_play_key_input(&mut state, 'n', "", &dir).unwrap();
    state.flush_staged_narration();
    assert!(state.endgame.as_ref().unwrap().outcome.is_none());
    assert!(state.endgame_key_wait_in_shared_wait(), "the clarification");
    let frame = state.animation.frame;
    assert!(state.prompt_wait_pass());
    assert_ne!(state.animation.frame, frame);
    // A final No ends on the waiting branch, which reads no key.
    handle_play_key_input(&mut state, 'n', "", &dir).unwrap();
    assert!(!state.endgame_key_wait_in_shared_wait());
    assert!(!state.prompt_wait_pass());
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn spec304_q4_victory_rite_pages_and_the_orb_step_and_the_narrative_pages_do_not() {
    let dir = debug_game_dir();
    let mut state = test_state(endgame_tableau_test_grid(), 5, 5);
    state.party_names = vec![*b"Avatar\0\0\0"];
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.enter_endgame_with_messages(Some(endgame_window_synthetic_messages()));
    state.finish_endgame_entry_presentation();
    state.flush_staged_narration();
    state.resolve_endgame_confirmation(true);
    // The hand-over, record 3 and the forty-iteration pause read no key.
    let mut frames = 0;
    while state.endgame.as_ref().unwrap().cinematic.step
        == crate::endgame_cinematic::EndgameCinematicStep::RiteMessage(0)
    {
        assert!(!state.endgame_key_wait_in_shared_wait(), "automatic frame {frames}");
        state.flush_staged_narration();
        frames += 1;
        assert!(frames < 500);
        assert!(state.advance_endgame_display_frame());
    }
    // The six rite page waits, each a shared wait.
    let mut pages = 0;
    while matches!(
        state.endgame.as_ref().unwrap().cinematic.step,
        crate::endgame_cinematic::EndgameCinematicStep::RiteMessage(_)
    ) {
        assert!(state.endgame_key_wait_in_shared_wait(), "rite page {pages}");
        let frame = state.animation.frame;
        assert!(state.prompt_wait_pass(), "rite page {pages}");
        assert_ne!(state.animation.frame, frame);
        handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
        state.flush_staged_narration();
        pages += 1;
        assert!(pages < 20);
    }
    assert_eq!(pages, 6, "after `He says:` and each of the next five records");
    // The box becomes the Orb, and that key wait is the tenth site.
    while state.advance_endgame_display_frame() {
        if state.endgame.as_ref().unwrap().victory_tableau_phase
            == EndgameVictoryTableauPhase::OrbAwaitingAcknowledgement
        {
            break;
        }
    }
    assert_eq!(
        state.endgame.as_ref().unwrap().victory_tableau_phase,
        EndgameVictoryTableauPhase::OrbAwaitingAcknowledgement
    );
    assert!(state.endgame_key_wait_in_shared_wait(), "the Orb wait");
    assert!(state.prompt_wait_pass());
    // The gate sequence after the Orb's key reads no key.
    handle_play_key_input(&mut state, ' ', "", &dir).unwrap();
    assert!(!state.endgame_key_wait_in_shared_wait(), "the gate sequence");
    // The final narrative's page waits are raw-key tight loops.
    state.endgame.as_mut().unwrap().cinematic.step =
        crate::endgame_cinematic::EndgameCinematicStep::NarrativeWindow(2);
    assert!(state.endgame_raw_key_page_wait_open());
    assert!(!state.endgame_key_wait_in_shared_wait());
    let before = state.clone();
    assert!(!state.prompt_wait_pass(), "no world step");
    assert_eq!(state.prng_state, before.prng_state);
    assert_eq!(state.animation.frame, before.animation.frame);
    let _ = fs::remove_dir_all(dir);
}

/// u5-spec#304 Q3 (`conversation.md §10` item 1): each pick is one draw over
/// `0..2` - `0` keys, `1` gems, `2` torches - redrawn, one draw an attempt,
/// until it lands on a counter the party holds.
#[test]
fn spec304_q3_falsehood_theft_maps_picks_to_keys_gems_torches_and_redraws() {
    let theft_state = |keys: u8, gems: u8, torches: u8| {
        let mut state = test_state(open_grid(), 1, 1);
        state.area = Area::Town {
            scene: Scene::new(4).unwrap(),
            floor: 0,
        };
        state.resident_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
        state.keys = keys;
        state.gems = gems;
        state.torches = torches;
        state
    };
    // With all three held, the first pick decides, for every result.
    let mut seen = [false; 3];
    for seed in 0u16..64 {
        let mut stream = seed;
        let pick = u5_prng_range_u16(&mut stream, 0, 2);
        let mut state = theft_state(5, 5, 5);
        assert!(state.run_final_conversation_cleanup_with_seed(seed));
        let taken = [5 - state.keys, 5 - state.gems, 5 - state.torches];
        let mut expected = [0u8; 3];
        expected[usize::from(pick)] = 1;
        assert_eq!(taken, expected, "seed {seed:#x}: pick {pick}");
        assert_eq!(state.prng_state, stream, "one draw");
        seen[usize::from(pick)] = true;
    }
    assert_eq!(seen, [true; 3], "every pick value was exercised");
    // With only torches held, picks of 0 and 1 are redrawn, one draw each.
    for seed in 0u16..64 {
        let mut stream = seed;
        let mut draws = 0;
        loop {
            draws += 1;
            if u5_prng_range_u16(&mut stream, 0, 2) == 2 {
                break;
            }
        }
        let mut state = theft_state(0, 0, 3);
        state.run_final_conversation_cleanup_with_seed(seed);
        assert_eq!(state.torches, 2, "seed {seed:#x}");
        assert_eq!(state.prng_state, stream, "seed {seed:#x}: {draws} draws");
    }
}

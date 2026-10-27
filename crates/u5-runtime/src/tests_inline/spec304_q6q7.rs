// u5-spec#304 Q6 and Q7 (spec fa63a69, R630, R631).

fn spec304_capture_cues(
    beats: &[crate::blackthorn::BlackthornCaptureBeat],
    wanted: crate::blackthorn::BlackthornCaptureCue,
) -> usize {
    beats.iter().filter(|beat| beat.cue == Some(wanted)).count()
}

fn spec304_capture_ticks(beats: &[crate::blackthorn::BlackthornCaptureBeat]) -> u32 {
    beats
        .iter()
        .map(|beat| match beat.hold {
            crate::blackthorn::BlackthornCaptureHold::Ticks(ticks) => u32::from(ticks),
            crate::blackthorn::BlackthornCaptureHold::ThroneEnvelope => 0,
        })
        .sum()
}

/// u5-spec#304 Q6, `blackthorn.md §3` "Capture narration timeline": the
/// executed captures "confirmed the line order, the sting counts 5, 18, 8 and
/// 5 and the hold durations".
#[test]
fn spec304_q6_capture_timeline_has_the_published_holds_and_stings() {
    use crate::blackthorn::{
        BLACKTHORN_CAPTURE_CHAINED_LINE, BLACKTHORN_CAPTURE_DRAGGED_LINE,
        BLACKTHORN_CAPTURE_FOOTSTEPS_LINE, BlackthornCaptureCue, BlackthornCaptureHold,
        blackthorn_capture_timeline,
    };
    let beats = blackthorn_capture_timeline(true, true);
    // Lines in order; nothing else prints before the greeting.
    let lines: Vec<_> = beats.iter().filter_map(|beat| beat.text).collect();
    assert_eq!(
        lines,
        vec![
            BLACKTHORN_CAPTURE_DRAGGED_LINE,
            BLACKTHORN_CAPTURE_CHAINED_LINE,
            BLACKTHORN_CAPTURE_FOOTSTEPS_LINE,
        ]
    );
    // "one before the first line, and two before each later one".
    assert!(crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE.starts_with('\n'));
    assert!(!crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE.starts_with("\n\n"));
    for line in &lines {
        assert!(line.starts_with("\n\n") && !line.starts_with("\n\n\n"), "{line:?}");
    }
    // 5 + 18 timer-paced stings and 8 footsteps, then the script's 5.
    assert_eq!(spec304_capture_cues(&beats, BlackthornCaptureCue::Sting), 5 + 18 + 8);
    let approach = beats
        .iter()
        .filter(|beat| matches!(beat.cue, Some(BlackthornCaptureCue::GuardApproachSting(_))))
        .count();
    assert_eq!(approach, 5);
    for cue in [
        BlackthornCaptureCue::Blindfold,
        BlackthornCaptureCue::RevealPrisoners,
        BlackthornCaptureCue::PlaceGuards,
        BlackthornCaptureCue::ThroneEnvelope,
        BlackthornCaptureCue::ThroneRevealed,
    ] {
        assert_eq!(spec304_capture_cues(&beats, cue), 1, "{cue:?}");
    }
    assert_eq!(
        beats
            .iter()
            .filter(|beat| beat.hold == BlackthornCaptureHold::ThroneEnvelope)
            .count(),
        1
    );

    // Hold arithmetic between the lines, in ticks.
    let dragged = beats
        .iter()
        .position(|beat| beat.text == Some(BLACKTHORN_CAPTURE_DRAGGED_LINE))
        .unwrap();
    let chained = beats
        .iter()
        .position(|beat| beat.text == Some(BLACKTHORN_CAPTURE_CHAINED_LINE))
        .unwrap();
    let footsteps = beats
        .iter()
        .position(|beat| beat.text == Some(BLACKTHORN_CAPTURE_FOOTSTEPS_LINE))
        .unwrap();
    // Steps 3 to 6: quiet 2, then five {5-tick wait, sting} - 27 ticks
    // (captured: +0.12 s to +1.65 s, about 27.9 ticks with the print).
    assert_eq!(spec304_capture_ticks(&beats[..=dragged]), 2 + 5 * 5);
    // Steps 7 to 10: eighteen {5-tick wait, sting}, then quiet 16.
    assert_eq!(spec304_capture_ticks(&beats[dragged + 1..=chained]), 18 * 5 + 16);
    // Steps 11 and 12: "Quiet 50-tick pause (2.79 s)."
    assert_eq!(spec304_capture_ticks(&beats[chained + 1..=footsteps]), 50);
    // Steps 13 to 17: 8 x 2 footsteps, 5 x 2 + 8 approach, the envelope,
    // the 18-tick reveal. The greeting's quiet 8 is pushed by the caller.
    assert_eq!(
        spec304_capture_ticks(&beats[footsteps + 1..]),
        8 * 2 + 5 * 2 + 8 + crate::blackthorn::BLACKTHORN_THRONE_REVEAL_HOLD_TICKS as u32
    );
    // The blindfold comes after the quiet 2-tick pause; the prisoners paint
    // with no wait after the last timer-paced sting.
    assert_eq!(beats[0].hold, BlackthornCaptureHold::Ticks(2));
    assert_eq!(beats[0].cue, Some(BlackthornCaptureCue::Blindfold));
    assert_eq!(beats[chained - 1].cue, Some(BlackthornCaptureCue::RevealPrisoners));
    assert_eq!(beats[chained - 1].hold, BlackthornCaptureHold::Ticks(0));
    assert_eq!(beats[chained].hold, BlackthornCaptureHold::Ticks(16));
}

/// u5-spec#304 Q6, `blackthorn.md §3` timeline step 7: "With sound on,
/// eighteen repetitions of {5-tick timer wait, sting}; with sound off,
/// three." And: "With cinematic animation off, every quiet pause does
/// nothing, including its world ticks ... The timer waits and stings still
/// run."
#[test]
fn spec304_q6_capture_timeline_sound_and_animation_arms() {
    use crate::blackthorn::{BlackthornCaptureCue, blackthorn_capture_timeline};
    let sound_off = blackthorn_capture_timeline(false, true);
    assert_eq!(spec304_capture_cues(&sound_off, BlackthornCaptureCue::Sting), 5 + 3 + 8);
    let on_ticks = spec304_capture_ticks(&blackthorn_capture_timeline(true, true));
    assert_eq!(on_ticks - spec304_capture_ticks(&sound_off), 15 * 5);

    let still = blackthorn_capture_timeline(true, false);
    // Only the 23 timer waits and the CPU-bound reveal still take time.
    assert_eq!(
        spec304_capture_ticks(&still),
        23 * 5 + crate::blackthorn::BLACKTHORN_THRONE_REVEAL_HOLD_TICKS as u32
    );
    assert_eq!(spec304_capture_cues(&still, BlackthornCaptureCue::Sting), 5 + 18 + 8);
    assert_eq!(spec304_capture_cues(&still, BlackthornCaptureCue::RevealPrisoners), 1);
}

/// u5-spec#304 Q6, `blackthorn.md §6.3` "Guard approach" row and timeline
/// step 15: the script "plays five stings, each with a quiet 2-tick pause
/// (0.55 s together), and ends with its quiet 8-tick pause". The envelope,
/// the throne reveal and the second quiet pause are the caller's.
#[test]
fn spec304_q6_guard_approach_script_ends_before_the_envelope() {
    let mut vm = BlackthornCutsceneVm::new(vec![0; 32 * 11]);
    for placement in BLACKTHORN_AUDIENCE_ACTOR_PLACEMENTS {
        if matches!(
            placement.actor,
            BlackthornCutsceneActor::LeftGuard | BlackthornCutsceneActor::RightGuard
        ) {
            vm.set_actor(
                placement.actor,
                BlackthornCutsceneActorState { x: placement.x, y: placement.y, actor_byte: placement.tile },
            );
        }
    }
    vm.run(blackthorn_cutscene_beat_commands(BlackthornCutsceneBeat::AudienceThroneApproach));
    assert_eq!(vm.stinger_count, 5);
    assert_eq!(vm.bios_ticks, 5 * 2 + 8);
    assert!(vm.cell_reveals.is_empty(), "the reveal follows the envelope");
    assert_eq!(vm.actor(BlackthornCutsceneActor::SeatedBlackthorn), None);
    let cells: Vec<_> = vm
        .actors_at_each_sting
        .iter()
        .map(|actors| {
            let at = |actor: BlackthornCutsceneActor| {
                actors[actor.slot_index() as usize].map(|state| (state.x, state.y))
            };
            (at(BlackthornCutsceneActor::LeftGuard), at(BlackthornCutsceneActor::RightGuard))
        })
        .collect();
    assert_eq!(
        cells,
        vec![
            (Some((4, 10)), Some((6, 10))),
            (Some((4, 9)), Some((6, 9))),
            (Some((3, 9)), Some((7, 9))),
            (Some((2, 9)), Some((8, 9))),
            (Some((1, 9)), Some((9, 9))),
        ]
    );
}

fn spec304_capture_state(dir: &Path) -> PlayState {
    let mut state = test_state(open_grid(), 5, 5);
    state.area = Area::Town {
        scene: Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap(),
        floor: 0,
    };
    state.begin_blackthorn_audience_capture(dir).unwrap();
    state
}

fn spec304_stings_after(state: &PlayState, serial: u64) -> usize {
    state
        .sound_effects_after(serial)
        .iter()
        .filter(|effect| **effect == SoundEffect::BlackthornMovementStinger)
        .count()
}

/// u5-spec#304 Q6 (R630, R631), `blackthorn.md §3` "Capture narration
/// timeline" run on the wall clock: the first line alone at the arrest, the
/// blindfold, the stings, the prisoners painted after the dragged-away
/// stretch, the guards only after the footsteps, slot 8 empty until the
/// reveal, and the greeting last with its row left open.
#[test]
fn spec304_q6_capture_narration_runs_on_the_published_timeline() {
    use crate::narration::bios_ticks_secs;
    let dir = debug_game_dir();
    let mut state = spec304_capture_state(&dir);
    let serial = state.sound_effect_serial;
    let text = |state: &PlayState| {
        state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>()
            .join("|")
    };

    // Step 1 alone; nothing of setup steps 3 to 6 yet.
    assert!(text(&state).ends_with("Thou art subdued and blindfolded!"), "{}", text(&state));
    assert!(state.staged_narration_active());
    assert!(state.cinematic_hold_owns_input());
    assert!(state.blackthorn_audience_map.is_none());
    assert!(!state.cutscene_black_viewport);

    state.advance_staged_narration_to(0.0);
    state.advance_staged_narration_to(bios_ticks_secs(2) + 1e-6);
    assert!(state.cutscene_black_viewport, "step 4's blindfold after quiet 2");
    assert_eq!(spec304_stings_after(&state, serial), 0);

    // Step 6 at 2 + 25 ticks, after five stings.
    state.advance_staged_narration_to(bios_ticks_secs(27) - 1e-3);
    assert!(!text(&state).contains("Strong guards"));
    state.advance_staged_narration_to(bios_ticks_secs(27) + 1e-6);
    assert!(text(&state).ends_with("Strong guards drag thee away!"), "{}", text(&state));
    assert_eq!(spec304_stings_after(&state, serial), 5);
    assert!(state.blackthorn_audience_map.is_none(), "still blindfolded");

    // Steps 8 and 9 after eighteen more stings (sound on).
    state.advance_staged_narration_to(bios_ticks_secs(27 + 90) + 1e-6);
    assert_eq!(spec304_stings_after(&state, serial), 23);
    assert!(state.blackthorn_audience_map.is_some(), "the prisoners are painted");
    assert!(!state.cutscene_black_viewport);
    let slot = |state: &PlayState, actor: BlackthornCutsceneActor| {
        state.active_objects.get(actor.slot_index() as usize).copied().unwrap_or(ActiveObject::empty())
    };
    assert_eq!((slot(&state, BlackthornCutsceneActor::Avatar).x, slot(&state, BlackthornCutsceneActor::Avatar).y), (3, 5));
    assert!(slot(&state, BlackthornCutsceneActor::LeftGuard).is_empty(), "guards come after the footsteps");
    assert!(slot(&state, BlackthornCutsceneActor::SeatedBlackthorn).is_empty(), "R631: slot 8 is zeroed");

    // Step 10 after the quiet 16-tick pause, step 12 after the quiet 50.
    state.advance_staged_narration_to(bios_ticks_secs(117 + 16) + 1e-6);
    assert!(text(&state).ends_with("Thou hast been chained and manacled!"));
    state.advance_staged_narration_to(bios_ticks_secs(133 + 50) + 1e-6);
    assert!(text(&state).ends_with("Footsteps!"));
    assert!(slot(&state, BlackthornCutsceneActor::LeftGuard).is_empty());

    // Step 13's eight footsteps, then step 14's guards.
    state.advance_staged_narration_to(bios_ticks_secs(183 + 16) - 1e-3);
    assert_eq!(spec304_stings_after(&state, serial), 31);
    assert!(slot(&state, BlackthornCutsceneActor::LeftGuard).is_empty());
    state.advance_staged_narration_to(bios_ticks_secs(183 + 16) + 1e-6);
    let left = slot(&state, BlackthornCutsceneActor::LeftGuard);
    let right = slot(&state, BlackthornCutsceneActor::RightGuard);
    assert_eq!(((left.x, left.y), (right.x, right.y)), ((4, 10), (6, 10)));
    assert_eq!(spec304_stings_after(&state, serial), 32, "the approach's first sting");

    // Step 15 ends after 5 x 2 + 8 ticks; step 16's envelope starts there.
    state.advance_staged_narration_to(bios_ticks_secs(199 + 18) + 1e-6);
    assert_eq!(spec304_stings_after(&state, serial), 36);
    assert!(
        state
            .sound_effects_after(serial)
            .contains(&SoundEffect::BlackthornThroneApproachEnvelope)
    );
    let left = slot(&state, BlackthornCutsceneActor::LeftGuard);
    assert_eq!((left.x, left.y), (1, 9));
    assert!(slot(&state, BlackthornCutsceneActor::SeatedBlackthorn).is_empty());
    assert!(!text(&state).contains("Blackthorn says"));

    // The rest: envelope, reveal, quiet 8, greeting.
    state.advance_staged_narration_to(bios_ticks_secs(217 + 18 + 8 + 12));
    assert!(!state.staged_narration_active());
    assert_eq!(
        slot(&state, BlackthornCutsceneActor::SeatedBlackthorn).tile,
        BLACKTHORN_SEATED_ACTOR_BYTE
    );
    assert!(text(&state).contains("Blackthorn says:"), "{}", text(&state));
    assert!(state.message_row_open_mid_line, "the greeting ends mid-row");
    assert!(!state.blackthorn_capture_narration_active());
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| {
        c.phase == crate::blackthorn_session::BlackthornChallengePhase::AwaitingGreetingAcknowledgement
    }));
    let _ = fs::remove_dir_all(dir);
}

/// u5-spec#304 Q6 (R630): "If every flag is non-zero the whole audience is
/// abandoned, and the first narration line is the only one printed."
#[test]
fn spec304_q6_abandoned_audience_prints_only_the_first_line() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    for flag in state.shrine_ruin_flags.iter_mut() {
        *flag = 1;
    }
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    assert!(!state.staged_narration_active());
    assert!(state.active_blackthorn.is_none());
    let joined = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.clone())
        .collect::<Vec<_>>()
        .join("|");
    assert!(joined.contains("Thou art subdued and blindfolded!"));
    assert!(!joined.contains("Strong guards"), "{joined}");
    let _ = fs::remove_dir_all(dir);
}

fn spec304_type_during_narration(state: &mut PlayState, dir: &Path, keys: &str) {
    for key in keys.chars() {
        crate::input_dispatch::handle_play_key_input(state, key, "", dir).unwrap();
    }
}

/// u5-spec#304 Q7, `blackthorn.md §3` "Keys typed during the capture
/// narration": "None of the narration's steps polls the keyboard ... Under
/// the default type-ahead setting (off, "Buffer Off"), exactly one of the
/// typed-ahead keys is used up by that acknowledgement. No further key is
/// needed, and the preamble of step 9 then waits for a fresh key."
#[test]
fn spec304_q7_buffer_off_greeting_takes_one_queued_key_and_flushes_the_rest() {
    let dir = debug_game_dir();
    let mut state = spec304_capture_state(&dir);
    state.typeahead_buffer_enabled = false;
    spec304_type_during_narration(&mut state, &dir, "abcde");
    assert_eq!(state.keys_buffered_during_hold.len(), 5, "the narration reads no key");
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| {
        c.phase == crate::blackthorn_session::BlackthornChallengePhase::AwaitingGreetingAcknowledgement
    }));

    state.flush_staged_narration();
    assert!(crate::input_dispatch::drain_buffered_hold_key(&mut state, &dir).unwrap().is_some());
    assert!(state.keys_buffered_during_hold.is_empty(), "the rest are discarded");
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| {
        c.phase == crate::blackthorn_session::BlackthornChallengePhase::AwaitingPreambleAcknowledgement
    }));
    assert!(crate::input_dispatch::drain_buffered_hold_key(&mut state, &dir).unwrap().is_none());
    let _ = fs::remove_dir_all(dir);
}

/// u5-spec#304 Q7: "With type-ahead on ("Buffer On"), nothing is discarded.
/// The queued keys answer the greeting, then the preamble, and the rest
/// become the first mantra response, in order."
#[test]
fn spec304_q7_buffer_on_queued_keys_answer_greeting_then_preamble() {
    let dir = debug_game_dir();
    let mut state = spec304_capture_state(&dir);
    state.typeahead_buffer_enabled = true;
    spec304_type_during_narration(&mut state, &dir, "abcde");
    state.flush_staged_narration();

    crate::input_dispatch::drain_buffered_hold_key(&mut state, &dir).unwrap();
    assert_eq!(state.keys_buffered_during_hold.len(), 4);
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| {
        c.phase == crate::blackthorn_session::BlackthornChallengePhase::AwaitingPreambleAcknowledgement
    }));
    crate::input_dispatch::drain_buffered_hold_key(&mut state, &dir).unwrap();
    let rest: String = state.keys_buffered_during_hold.iter().map(|(key, _)| *key).collect();
    assert_eq!(rest, "cde", "kept, in order, for the mantra response");
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_some()));
    let _ = fs::remove_dir_all(dir);
}

// u5-spec#304 Q6 and Q7 (spec fa63a69, R630, R631).

fn spec304_capture_cues(
    beats: &[crate::blackthorn::BlackthornCaptureBeat],
    wanted: crate::blackthorn::BlackthornCaptureCue,
) -> usize {
    beats.iter().filter(|beat| beat.cue == Some(wanted)).count()
}

fn spec304_capture_ticks(beats: &[crate::blackthorn::BlackthornCaptureBeat]) -> u32 {
    beats.iter().map(|beat| u32::from(beat.hold.tick_requests())).sum()
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
            crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE,
            BLACKTHORN_CAPTURE_DRAGGED_LINE,
            BLACKTHORN_CAPTURE_CHAINED_LINE,
            BLACKTHORN_CAPTURE_FOOTSTEPS_LINE,
        ]
    );
    // "one before the first line, and two before each later one".
    assert!(crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE.starts_with('\n'));
    assert!(!crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE.starts_with("\n\n"));
    for line in &lines[1..] {
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
        BlackthornCaptureCue::Greeting,
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
    // Steps 3 to 6: quiet 2, then five {5-tick wait, sting} - 27 tick
    // requests.
    assert_eq!(spec304_capture_ticks(&beats[..=dragged]), 2 + 5 * 5);
    // Steps 7 to 10: eighteen {5-tick wait, sting}, then quiet 16.
    assert_eq!(spec304_capture_ticks(&beats[dragged + 1..=chained]), 18 * 5 + 16);
    // Steps 11 and 12: "Quiet 50-tick pause (2.79 s)."
    assert_eq!(spec304_capture_ticks(&beats[chained + 1..=footsteps]), 50);
    // Steps 13 to 19: 8 x 2 footsteps, 5 x 2 + 8 approach, the envelope and
    // the CPU-bound reveal (no requests), the greeting's quiet 8.
    assert_eq!(spec304_capture_ticks(&beats[footsteps + 1..]), 8 * 2 + 5 * 2 + 8 + 8);
    // The first line is the measured span after the arrest; the blindfold
    // comes after the quiet 2-tick pause; the prisoners paint after the
    // setup's CPU time with no wait after the last timer-paced sting.
    assert_eq!(beats[0].hold, BlackthornCaptureHold::FirstLine);
    assert_eq!(beats[1].hold, BlackthornCaptureHold::Quiet(2));
    assert_eq!(beats[1].cue, Some(BlackthornCaptureCue::Blindfold));
    assert_eq!(beats[2].hold, BlackthornCaptureHold::TimerWait(5));
    assert_eq!(beats[chained - 1].cue, Some(BlackthornCaptureCue::RevealPrisoners));
    assert_eq!(beats[chained - 1].hold, BlackthornCaptureHold::Setup);
    assert_eq!(beats[chained].hold, BlackthornCaptureHold::Quiet(16));
    assert_eq!(beats.last().unwrap().hold, BlackthornCaptureHold::Quiet(8));
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
    // Only the 23 timer waits still issue requests.
    assert_eq!(spec304_capture_ticks(&still), 23 * 5);
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

fn spec304_capture_releases(sound_on: bool) -> (Vec<crate::blackthorn::BlackthornCaptureBeat>, Vec<u32>) {
    let beats = crate::blackthorn::blackthorn_capture_timeline(sound_on, true);
    let envelope = crate::blackthorn::secs_milliticks(
        SoundEffect::BlackthornThroneApproachEnvelope
            .program(&mut crate::audio::RumbleJitter::default())
            .duration(true)
            .as_secs_f64(),
    );
    let greeting = crate::blackthorn::blackthorn_greeting_line("Avatar");
    let releases = crate::blackthorn::blackthorn_capture_release_milliticks(&beats, 0, envelope, |beat| {
        (beat.cue == Some(crate::blackthorn::BlackthornCaptureCue::Greeting)).then(|| greeting.clone())
    });
    (beats, releases)
}

fn spec304_release_of(
    beats: &[crate::blackthorn::BlackthornCaptureBeat],
    releases: &[u32],
    wanted: impl Fn(&crate::blackthorn::BlackthornCaptureBeat) -> bool,
) -> f64 {
    let index = beats.iter().position(wanted).expect("beat present");
    crate::blackthorn::milliticks_secs(releases[index])
}

/// `karma.md §7` "Pacing rule" (issue #304 Q8) applied to `blackthorn.md §3`'s
/// counts lands each beat on the capture's published seconds ("timed on the
/// emulated clock from the frame where the `Talk-North` echo appears. They are
/// not normative; the counts are"). Holding the counts as whole ticks of wall
/// clock put the guards about 0.3 s early (`bt-audience`/`ask1`,
/// `bt-capture-pacing`, 2026-10-27).
#[test]
fn spec304_q6_capture_pacing_lands_on_the_published_seconds() {
    use crate::blackthorn::{
        BLACKTHORN_CAPTURE_CHAINED_LINE, BLACKTHORN_CAPTURE_DRAGGED_LINE,
        BLACKTHORN_CAPTURE_FOOTSTEPS_LINE, BLACKTHORN_CAPTURE_SUBDUED_LINE, BlackthornCaptureCue,
    };
    let (beats, releases) = spec304_capture_releases(true);
    assert!(releases.windows(2).all(|pair| pair[0] <= pair[1]));
    let text = |line: &'static str| move |beat: &crate::blackthorn::BlackthornCaptureBeat| beat.text == Some(line);
    let cue = |wanted: BlackthornCaptureCue| move |beat: &crate::blackthorn::BlackthornCaptureBeat| beat.cue == Some(wanted);
    let footsteps = 10.30;
    let published = [
        // Step 1, "+0.12 s".
        (spec304_release_of(&beats, &releases, text(BLACKTHORN_CAPTURE_SUBDUED_LINE)), 0.12),
        // Step 4, "at +0.25 s".
        (spec304_release_of(&beats, &releases, cue(BlackthornCaptureCue::Blindfold)), 0.25),
        // Step 6, "+1.65 s".
        (spec304_release_of(&beats, &releases, text(BLACKTHORN_CAPTURE_DRAGGED_LINE)), 1.65),
        // Step 9, "paints the cutscene map with the manacled prisoners (+6.66 s)".
        (spec304_release_of(&beats, &releases, cue(BlackthornCaptureCue::RevealPrisoners)), 6.66),
        // Step 10, "+7.51 s".
        (spec304_release_of(&beats, &releases, text(BLACKTHORN_CAPTURE_CHAINED_LINE)), 7.51),
        // Step 12, "+10.30 s".
        (spec304_release_of(&beats, &releases, text(BLACKTHORN_CAPTURE_FOOTSTEPS_LINE)), footsteps),
        // Step 13, "about 0.11 s each and 0.93 s in all", then step 14.
        (spec304_release_of(&beats, &releases, cue(BlackthornCaptureCue::PlaceGuards)), footsteps + 0.93),
        // Step 15: "0.55 s together", then "about 0.42 s".
        (
            spec304_release_of(&beats, &releases, cue(BlackthornCaptureCue::ThroneEnvelope)),
            footsteps + 0.93 + 0.55 + 0.42,
        ),
        // Step 19, "+14.20 s".
        (spec304_release_of(&beats, &releases, cue(BlackthornCaptureCue::Greeting)), 14.20),
    ];
    for (model, stock) in published {
        assert!((model - stock).abs() <= 0.12, "model {model:.3} s against the published {stock:.2} s");
    }
    // The whole-tick holds this replaces: the counts alone reach the greeting
    // about 0.3 s before the stock game does.
    let counts_alone = crate::narration::bios_ticks_secs(spec304_capture_ticks(&beats) as u16)
        + 0.559
        + 0.99;
    assert!(14.20 - counts_alone > 0.25, "{counts_alone}");

    // "A repaint plus a sting fits inside one tick, so the sting never costs
    // an extra edge" (`karma.md §7`): the timer-paced {5-tick wait, sting} is
    // five ticks ("about 0.274 s per repetition", `audio.md §8.6`) and the
    // footstep {sting, quiet 2} two ("about 0.11 s each").
    let stings: Vec<f64> = beats
        .iter()
        .zip(&releases)
        .filter(|(beat, _)| beat.cue == Some(BlackthornCaptureCue::Sting))
        .map(|(_, &release)| crate::blackthorn::milliticks_secs(release))
        .collect();
    assert!((stings[2] - stings[1] - 0.2747).abs() < 1e-3);
    assert!((stings[25] - stings[24] - 0.1099).abs() < 1e-3);
}

/// `text-output.md §6`, "Line feeds and full rows": `and blindfolded!` fills
/// the sixteen-cell row, so the next line's two feeds both cost a row.
#[test]
fn spec304_q6_first_line_ends_on_a_full_row() {
    use crate::blackthorn::{presentation_print_leaves_row_open, presentation_printed_rows};
    assert_eq!(
        crate::text_wrap::wrap_text("Thou art subdued and blindfolded!", crate::MESSAGE_WINDOW_WIDTH, 0),
        vec!["Thou art subdued".to_string(), "and blindfolded!".to_string()]
    );
    assert!(!presentation_print_leaves_row_open(crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE));
    assert_eq!(presentation_printed_rows(crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE), 3);
    for line in [
        crate::blackthorn::BLACKTHORN_CAPTURE_DRAGGED_LINE,
        crate::blackthorn::BLACKTHORN_CAPTURE_CHAINED_LINE,
        crate::blackthorn::BLACKTHORN_CAPTURE_FOOTSTEPS_LINE,
    ] {
        assert!(presentation_print_leaves_row_open(line), "{line:?}");
    }
    assert_eq!(presentation_printed_rows(crate::blackthorn::BLACKTHORN_CAPTURE_CHAINED_LINE), 4);
}

/// u5-spec#304 Q6 (R630, R631), `blackthorn.md §3` "Capture narration
/// timeline" run on the wall clock: the first line, the blindfold, the
/// stings, the prisoners painted after the dragged-away stretch, the guards
/// only after the footsteps, slot 8 empty until the reveal, and the greeting
/// last with its row left open - each at its paced release.
#[test]
fn spec304_q6_capture_narration_runs_on_the_published_timeline() {
    use crate::blackthorn::{BlackthornCaptureCue, milliticks_secs};
    let dir = debug_game_dir();
    let mut state = spec304_capture_state(&dir);
    state.music_enabled = true;
    let serial = state.sound_effect_serial;
    let (beats, releases) = spec304_capture_releases(true);
    let at = |cue: BlackthornCaptureCue| {
        milliticks_secs(releases[beats.iter().position(|beat| beat.cue == Some(cue)).unwrap()])
    };
    let line_at = |line: &'static str| {
        milliticks_secs(releases[beats.iter().position(|beat| beat.text == Some(line)).unwrap()])
    };
    let text = |state: &PlayState| {
        state
            .message_transcript
            .iter()
            .map(|entry| entry.text.clone())
            .collect::<Vec<_>>()
            .join("|")
    };
    let slot = |state: &PlayState, actor: BlackthornCutsceneActor| {
        state.active_objects.get(actor.slot_index() as usize).copied().unwrap_or(ActiveObject::empty())
    };
    let before = |state: &mut PlayState, secs: f64| state.advance_staged_narration_to(secs - 1e-4);
    let after = |state: &mut PlayState, secs: f64| state.advance_staged_narration_to(secs + 1e-4);

    // Nothing yet: the first line is the narration's first beat.
    state.advance_staged_narration_to(0.0);
    assert!(!text(&state).contains("subdued"), "{}", text(&state));
    assert!(state.staged_narration_active());
    assert!(state.cinematic_hold_owns_input());
    assert!(state.blackthorn_audience_map.is_none());
    let first = line_at(crate::blackthorn::BLACKTHORN_CAPTURE_SUBDUED_LINE);
    after(&mut state, first);
    assert!(text(&state).ends_with("Thou art subdued and blindfolded!"), "{}", text(&state));
    assert!(!state.message_row_open_mid_line, "the line ends on a full row");
    assert!(!state.cutscene_black_viewport);

    after(&mut state, at(BlackthornCaptureCue::Blindfold));
    assert!(state.cutscene_black_viewport, "step 4's blindfold after quiet 2");
    assert_eq!(spec304_stings_after(&state, serial), 0);

    let dragged = line_at(crate::blackthorn::BLACKTHORN_CAPTURE_DRAGGED_LINE);
    before(&mut state, dragged);
    assert!(!text(&state).contains("Strong guards"));
    after(&mut state, dragged);
    assert!(text(&state).ends_with("|||Strong guards drag thee away!"), "{}", text(&state));
    assert_eq!(spec304_stings_after(&state, serial), 5);
    assert!(state.blackthorn_audience_map.is_none(), "still blindfolded");

    // Steps 8 and 9 after eighteen more stings (sound on).
    after(&mut state, at(BlackthornCaptureCue::RevealPrisoners));
    assert_eq!(spec304_stings_after(&state, serial), 23);
    assert!(state.blackthorn_audience_map.is_some(), "the prisoners are painted");
    assert!(!state.cutscene_black_viewport);
    assert_eq!((slot(&state, BlackthornCutsceneActor::Avatar).x, slot(&state, BlackthornCutsceneActor::Avatar).y), (3, 5));
    assert!(slot(&state, BlackthornCutsceneActor::LeftGuard).is_empty(), "guards come after the footsteps");
    assert!(slot(&state, BlackthornCutsceneActor::SeatedBlackthorn).is_empty(), "R631: slot 8 is zeroed");

    after(&mut state, line_at(crate::blackthorn::BLACKTHORN_CAPTURE_CHAINED_LINE));
    assert!(text(&state).ends_with("Thou hast been chained and manacled!"));
    after(&mut state, line_at(crate::blackthorn::BLACKTHORN_CAPTURE_FOOTSTEPS_LINE));
    assert!(text(&state).ends_with("Footsteps!"));
    assert!(slot(&state, BlackthornCutsceneActor::LeftGuard).is_empty());

    // Step 13's eight footsteps, then step 14's guards.
    let guards = at(BlackthornCaptureCue::PlaceGuards);
    before(&mut state, guards);
    assert_eq!(spec304_stings_after(&state, serial), 31);
    assert!(slot(&state, BlackthornCutsceneActor::LeftGuard).is_empty());
    after(&mut state, guards);
    let left = slot(&state, BlackthornCutsceneActor::LeftGuard);
    let right = slot(&state, BlackthornCutsceneActor::RightGuard);
    assert_eq!(((left.x, left.y), (right.x, right.y)), ((4, 10), (6, 10)));
    assert_eq!(spec304_stings_after(&state, serial), 32, "the approach's first sting");

    // Step 15 ends; step 16's envelope starts there.
    after(&mut state, at(BlackthornCaptureCue::ThroneEnvelope));
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

    before(&mut state, at(BlackthornCaptureCue::ThroneRevealed));
    assert!(slot(&state, BlackthornCutsceneActor::SeatedBlackthorn).is_empty());
    let greeting = at(BlackthornCaptureCue::Greeting);
    before(&mut state, greeting);
    assert_eq!(
        slot(&state, BlackthornCutsceneActor::SeatedBlackthorn).tile,
        BLACKTHORN_SEATED_ACTOR_BYTE
    );
    assert!(!text(&state).contains("Blackthorn says:"));
    after(&mut state, greeting);
    assert!(!state.staged_narration_active());
    assert!(text(&state).contains("Blackthorn says:"), "{}", text(&state));
    assert!(state.message_row_open_mid_line, "the greeting ends mid-row");
    assert!(!state.blackthorn_capture_narration_active());
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| {
        c.phase == crate::blackthorn_session::BlackthornChallengePhase::AwaitingGreetingAcknowledgement
    }));
    let _ = fs::remove_dir_all(dir);
}

/// `conversation.md §2`: the dispatcher's own leading line feed precedes the
/// regime handler, so a palace refusal leaves two blank rows - that feed and
/// the first line's own - between the walk's echo and `Thou art subdued and
/// blindfolded!`, and that line's full last row leaves two more before the
/// dragged-away line (`bt-capture-pacing`, 2026-10-27).
#[test]
fn spec304_q6_palace_refusal_lays_out_the_stock_rows() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.area = Area::Town {
        scene: Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap(),
        floor: 0,
    };
    state.load_scheduled_npcs(&[
        NpcSlot { slot: 0, type_byte: 0, dialog_id: 0, schedule: [0; 16], name: None },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: BLACKTHORN_GUARD_DEMAND_DIALOG_ID,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state.emit_command_echo_line("North");
    let outcome = state.begin_blackthorn_guard_demand(2, 1, false, Some(&dir)).unwrap();
    assert_eq!(outcome, MoveOutcome::Used);
    assert!(state.blackthorn_capture_narration_active());
    state.flush_staged_narration();
    let rows: Vec<String> = state.message_transcript.iter().map(|entry| entry.text.clone()).collect();
    let echo = rows.iter().rposition(|row| row == "North").unwrap();
    assert_eq!(
        rows[echo..echo + 7].to_vec(),
        vec![
            "North",
            "",
            "",
            "Thou art subdued and blindfolded!",
            "",
            "",
            "Strong guards drag thee away!",
        ]
    );
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
    // The guard-release route is staged; the preamble's read follows it.
    assert!(crate::input_dispatch::drain_buffered_hold_key(&mut state, &dir).unwrap().is_none());
    state.flush_staged_narration();
    crate::input_dispatch::drain_buffered_hold_key(&mut state, &dir).unwrap();
    let rest: String = state.keys_buffered_during_hold.iter().map(|(key, _)| *key).collect();
    assert_eq!(rest, "cde", "kept, in order, for the mantra response");
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_some()));
    let _ = fs::remove_dir_all(dir);
}

/// `blackthorn.md §3` setup steps 8 and 9 with §6.3's "Guard release route":
/// after the greeting's key the release order prints, then "quiet pause 11;
/// the acting guard moves west once, north four times, and east once, with a
/// stinger-plus-two-tick redraw after every step", and only then the preamble
/// record. It used to run in one go with the preamble printed at the key
/// (`bt-capture-pacing`/`s06`, `bt-correct`/`typed`, 2026-10-27).
#[test]
fn spec304_guard_release_route_is_staged_before_the_preamble() {
    use crate::blackthorn::{BlackthornCaptureCue, milliticks_secs};
    let dir = debug_game_dir();
    let mut state = spec304_capture_state(&dir);
    state.flush_staged_narration();
    let serial = state.sound_effect_serial;
    let start = state.active_objects[BlackthornCutsceneActor::LeftGuard.slot_index() as usize];
    assert_eq!((start.x, start.y), (1, 9));

    state.submit_blackthorn_audience_answer("", &dir).unwrap();
    let text = |state: &PlayState| {
        state.message_transcript.iter().map(|entry| entry.text.clone()).collect::<Vec<_>>().join("|")
    };
    assert!(text(&state).ends_with("GUARD! Release this goodman at once!\""), "{}", text(&state));
    assert!(state.staged_narration_active(), "the route holds the screen");
    assert!(state.cinematic_hold_owns_input());

    let route = crate::blackthorn::blackthorn_guard_release_timeline();
    let start_work = crate::blackthorn::presentation_printed_rows("\n\nGUARD! Release this goodman at once!\"")
        * crate::blackthorn::PRESENTATION_ROW_PRINT_MILLITICKS;
    let releases = crate::blackthorn::blackthorn_capture_release_milliticks(&route, start_work, 0, |_| None);
    // Quiet 11 plus six {move, sting, quiet 2}: 23 requests, and the
    // release order's rows lose the first edge - 24 ticks to the preamble.
    assert_eq!(releases.last().copied(), Some(24_000));

    state.advance_staged_narration_to(0.0);
    state.advance_staged_narration_to(milliticks_secs(releases[0]) - 1e-4);
    assert_eq!(state.sound_effects_after(serial).len(), 0);
    let guard = |state: &PlayState| {
        let object = state.active_objects[BlackthornCutsceneActor::LeftGuard.slot_index() as usize];
        (object.x, object.y)
    };
    assert_eq!(guard(&state), (1, 9), "quiet pause 11 first");
    let mut cells = Vec::new();
    for (index, &release) in releases.iter().enumerate().take(6) {
        state.advance_staged_narration_to(milliticks_secs(release) + 1e-4);
        cells.push(guard(&state));
        assert_eq!(
            route[index].cue,
            Some(BlackthornCaptureCue::GuardReleaseStep(index as u8))
        );
    }
    assert_eq!(cells, vec![(0, 9), (0, 8), (0, 7), (0, 6), (0, 5), (1, 5)]);
    let before_preamble = text(&state);
    state.advance_staged_narration_to(milliticks_secs(releases[6]) + 1e-4);
    assert!(!state.staged_narration_active());
    assert!(!state.blackthorn_capture_narration_active());
    // The fixture directory may carry no `MISCMSG.DAT`; the preamble prints
    // here when it does, and nothing prints before this beat either way.
    match state.blackthorn_audience_opening_text(&dir).unwrap() {
        Some(_) => {
            assert_ne!(text(&state), before_preamble);
            assert!(state.message_row_open_mid_line, "the preamble ends on its trailing space");
        }
        None => assert_eq!(text(&state), before_preamble),
    }
    assert!(state.active_blackthorn.as_ref().is_some_and(|c| {
        c.phase == crate::blackthorn_session::BlackthornChallengePhase::AwaitingPreambleAcknowledgement
    }));
    let _ = fs::remove_dir_all(dir);
}

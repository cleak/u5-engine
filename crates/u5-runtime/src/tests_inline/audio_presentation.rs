// `systems/audio.md` trigger-boundary regressions: presentation.
//
// Each test names the published clause it pins. Add tests here rather
// than to the numbered chunks so the audio work stays reviewable as a
// unit.

use crate::dissolve::DissolveVisit;
use crate::return_to_view::{RTV_CHIME_CYCLE_TICKS, RTV_SINGLE_CELL_CHECKPOINTS};

fn empty_return_to_view_strips() -> ReturnToViewMapStrips {
    ReturnToViewMapStrips {
        strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
    }
}

fn return_to_view_tick_sounds(commands: Vec<ReturnToViewCommand>) -> Vec<Option<SoundEffect>> {
    let strips = empty_return_to_view_strips();
    let script = ReturnToViewScript { commands };
    run_return_to_view_playback_until_restart(&strips, &script, 64)
        .unwrap()
        .frames
        .iter()
        .map(|frame| frame.sound.clone())
        .collect()
}

#[test]
fn return_to_view_strip_two_percusses_on_every_scheduled_preview_tick() {
    // audio.md §8.6 / intro.md §12: strip 2 ("The Arrival") "emits a
    // random-pitch percussive speaker effect on every preview tick".
    let sounds = return_to_view_tick_sounds(vec![
        ReturnToViewCommand::LoadMapStrip { strip: 2 },
        ReturnToViewCommand::RunPreviewTick { ticks: 5 },
    ]);
    assert_eq!(sounds, vec![Some(SoundEffect::ReturnToViewStrip2); 5]);
}

#[test]
fn return_to_view_strips_zero_and_one_are_silent() {
    // intro.md §12: "strips 0 and 1 are silent". A preview that has not
    // loaded a strip yet leaves `current_strip` unset, which is silent too.
    for strip in [0u8, 1] {
        let sounds = return_to_view_tick_sounds(vec![
            ReturnToViewCommand::LoadMapStrip { strip },
            ReturnToViewCommand::RunPreviewTick { ticks: 9 },
        ]);
        assert_eq!(sounds, vec![None; 9], "strip {strip} must be silent");
    }
    let sounds = return_to_view_tick_sounds(vec![ReturnToViewCommand::RunPreviewTick { ticks: 4 }]);
    assert_eq!(sounds, vec![None; 4], "no strip loaded yet is silent");
}

#[test]
fn return_to_view_strip_three_chimes_only_at_local_phases_zero_and_four() {
    // audio.md §8.6: "At local phase 0 play a 3000 Hz blocking tone for 3
    // calibrated units; at phase 4 play 2000 Hz for 3." intro.md §12 calls
    // it "a two-tone chime on an eight-tick cycle", so the other six phases
    // of every cycle are silent.
    let ticks = 2 * RTV_CHIME_CYCLE_TICKS as u8 + 1;
    let sounds = return_to_view_tick_sounds(vec![
        ReturnToViewCommand::LoadMapStrip { strip: 3 },
        ReturnToViewCommand::RunPreviewTick { ticks },
    ]);
    let expected: Vec<Option<SoundEffect>> = (0..ticks)
        .map(|index| match index % 8 {
            0 => Some(SoundEffect::ReturnToViewStrip3 { phase: 0 }),
            4 => Some(SoundEffect::ReturnToViewStrip3 { phase: 4 }),
            _ => None,
        })
        .collect();
    assert_eq!(sounds, expected);
    assert_eq!(sounds.iter().filter(|sound| sound.is_some()).count(), 5);
}

#[test]
fn return_to_view_fixed_wipe_sounds_its_rectangle_ticks_but_not_the_actor_draw() {
    // audio.md §8.6 ties the cue to the scheduled tick. The five wipe
    // rectangles and the three trailing ticks each advance one title tick;
    // the fixed actor draw between them reuses the fifth rectangle's tick
    // and schedules none of its own, so it carries no cue.
    let strips = empty_return_to_view_strips();
    let script = ReturnToViewScript {
        commands: vec![
            ReturnToViewCommand::LoadMapStrip { strip: 2 },
            ReturnToViewCommand::SetActor {
                slot: 0,
                tile: 3,
                x: 1,
                y: 1,
            },
            ReturnToViewCommand::FixedWipeAndActorDraw {
                reserved0: 0,
                reserved1: 0,
                slot: 0,
            },
        ],
    };
    let playback = run_return_to_view_playback_until_restart(&strips, &script, 64).unwrap();
    let observed: Vec<_> = playback
        .frames
        .iter()
        .map(|frame| (frame.kind, frame.sound.clone()))
        .collect();

    assert_eq!(observed.len(), usize::from(RTV_FIXED_WIPE_TOTAL_TICKS) + 1);
    for step in 0..RTV_FIXED_WIPE_STEPS {
        assert_eq!(
            observed[usize::from(step)],
            (
                ReturnToViewFrameKind::FixedWipeRectangle { step },
                Some(SoundEffect::ReturnToViewStrip2)
            )
        );
    }
    assert_eq!(
        observed[usize::from(RTV_FIXED_WIPE_STEPS)],
        (ReturnToViewFrameKind::FixedWipeActorDraw, None),
        "the fixed actor draw shares the last wipe tick and stays silent",
    );
    for tick in 0..RTV_FIXED_WIPE_TRAILING_TICKS {
        assert_eq!(
            observed[usize::from(RTV_FIXED_WIPE_STEPS) + 1 + usize::from(tick)],
            (
                ReturnToViewFrameKind::FixedWipeTrailingTick { tick },
                Some(SoundEffect::ReturnToViewStrip2)
            )
        );
    }
}

#[test]
fn return_to_view_temporary_actor_draw_is_silent_on_its_unscheduled_final_group() {
    // `u5-spec#117`: the single-cell convergence checks input "through a
    // full preview tick after every eight writes except the final group",
    // giving 31 scheduled ticks for 32 groups. audio.md §8.6 attaches the
    // cue to the scheduled tick, so group 32 sounds nothing.
    let strips = empty_return_to_view_strips();
    let script = ReturnToViewScript {
        commands: vec![
            ReturnToViewCommand::LoadMapStrip { strip: 2 },
            ReturnToViewCommand::SetActor {
                slot: 0,
                tile: 3,
                x: 2,
                y: 1,
            },
            ReturnToViewCommand::TemporaryActorDraw { slot: 0 },
        ],
    };
    let sounds: Vec<_> = run_return_to_view_playback_until_restart(&strips, &script, 64)
        .unwrap()
        .frames
        .iter()
        .map(|frame| frame.sound.clone())
        .collect();

    assert_eq!(sounds.len(), usize::from(RTV_SINGLE_CELL_CHECKPOINTS) + 1);
    let (scheduled, final_group) = sounds.split_at(usize::from(RTV_SINGLE_CELL_CHECKPOINTS));
    assert_eq!(
        scheduled,
        vec![Some(SoundEffect::ReturnToViewStrip2); usize::from(RTV_SINGLE_CELL_CHECKPOINTS)]
    );
    assert_eq!(final_group, [None]);
}

/// Runs a four-by-four gated dissolve. `key_pending_from_copy` is when the
/// player's key starts being held down; the driver's own alternating flag, not
/// this helper, decides whether that key is ever seen.
fn run_gated_dissolve(
    mut gate: DissolveAbortGate,
    key_pending_from_copy: Option<u32>,
) -> (Vec<DissolveVisit>, bool) {
    let mut dissolve = RectangleDissolve::new((0, 0, 3, 3)).unwrap();
    let mut visits = Vec::new();
    let outcome = dissolve.run_gated(&mut gate, |visit| {
        visits.push(visit.clone());
        let key_pending = key_pending_from_copy.is_some_and(|first| visit.copied_pixels() >= first);
        visit.poll(key_pending)
    });
    (visits, outcome.aborted)
}

#[test]
fn armed_rectangle_dissolve_clicks_on_every_second_visited_pixel() {
    // audio.md §8.6.1: "every second visited pixel advances a driver-local
    // pitch state and retunes a continuously running speaker carrier ...
    // The same points poll keyboard status."
    let (visits, aborted) = run_gated_dissolve(DissolveAbortGate::on_driver_load(), None);
    assert!(!aborted);
    assert_eq!(visits.len(), 16);
    for visit in &visits {
        assert_eq!(
            visit.sound().is_some(),
            visit.samples_input(),
            "the click and the keyboard poll are the same points",
        );
        assert_eq!(visit.samples_input(), visit.copied_pixels() % 2 == 1);
    }
    // §8.6.1: a 16-pixel rectangle produces ceil(16 / 2) = 8 clicks, and from
    // a freshly loaded driver they are the published opening frequencies.
    assert_eq!(
        dissolve_click_pitches(&visits),
        vec![118, 105, 101, 110, 108, 113, 113, 123],
        "one retune per checked visit, off the shared driver-local state",
    );
}

/// `audio.md §8.6.1`: a click carries its own emitted frequency, not a
/// fraction of the rectangle — "Not the fraction of the rectangle copied, and
/// not the pixel coordinate."
fn dissolve_click_pitches(visits: &[DissolveVisit]) -> Vec<u16> {
    visits
        .iter()
        .filter_map(|visit| match visit.sound() {
            Some(SoundEffect::DissolveClick { frequency_hz }) => Some(*frequency_hz),
            _ => None,
        })
        .collect()
}

#[test]
fn an_even_pixel_rectangle_takes_its_last_dissolve_click_one_copy_short_of_the_end() {
    // display-driver-abi.md §9.6: "The first gated dissolve therefore checks
    // visits `1, 3, 5, ...`, not `2, 4, 6, ...`." The final copy of an
    // even-sized rectangle is an unchecked visit, so the transfer ends after a
    // silent pixel. audio.md §8.6.1 states the count directly: "A gated
    // rectangle of `P` pixels produces `ceil(P / 2)` clicks."
    let (visits, aborted) = run_gated_dissolve(DissolveAbortGate::on_driver_load(), None);
    assert!(!aborted);
    assert_eq!(visits.len(), 16);
    assert!(
        visits.last().unwrap().sound().is_none(),
        "the last copy of an even-sized rectangle is not a checked visit",
    );
    assert_eq!(dissolve_click_pitches(&visits).len(), 16_u32.div_ceil(2) as usize);

    // An odd pixel count does end on a checked visit, so its last copy clicks.
    // Three-by-three checks copies 1, 3, 5, 7 and 9 of 9.
    let mut odd = RectangleDissolve::new((0, 0, 2, 2)).unwrap();
    let mut odd_visits = Vec::new();
    let outcome = odd.run_gated(&mut DissolveAbortGate::on_driver_load(), |visit| {
        odd_visits.push(visit.clone());
        visit.poll(false)
    });
    assert!(!outcome.aborted);
    assert_eq!(odd_visits.len(), 9);
    assert!(
        odd_visits.last().unwrap().sound().is_some(),
        "the last copy of an odd-sized rectangle is a checked visit",
    );
    assert_eq!(dissolve_click_pitches(&odd_visits).len(), 9_u32.div_ceil(2) as usize);
}

#[test]
fn a_disarmed_dissolve_gate_is_silent_and_unabortable() {
    // audio.md §8.6: "The first ordinary glyph draw permanently disables
    // this gate, so later dissolves in the same run are silent and cannot
    // be aborted through this gate."
    let mut gate = DissolveAbortGate::on_driver_load();
    gate.note_fixed_cell_glyph_drawn();
    assert!(!gate.is_armed());

    let (visits, aborted) = run_gated_dissolve(gate, Some(1));
    assert!(!aborted, "a disarmed gate never polls, so it never aborts");
    assert_eq!(visits.len(), 16);
    assert!(visits.iter().all(|visit| visit.sound().is_none()));
    assert!(visits.iter().all(|visit| !visit.samples_input()));
}

#[test]
fn a_dissolve_cannot_abort_from_a_visit_that_never_polled() {
    // display-driver-abi.md §9.6: "Both the click and the poll sit behind the
    // same alternating flag, so neither happens on every pixel." A key that is
    // only ever pending while the driver is not looking cannot stop the
    // transfer, and the unchecked visits carry no click to stop either.
    let mut dissolve = RectangleDissolve::new((0, 0, 3, 3)).unwrap();
    let mut offered = Vec::new();
    let outcome = dissolve.run_gated(&mut DissolveAbortGate::on_driver_load(), |visit| {
        // Held down across every even copy, released across every odd one.
        let key_pending = visit.copied_pixels() % 2 == 0;
        if key_pending {
            offered.push(visit.copied_pixels());
        }
        visit.poll(key_pending)
    });

    assert!(
        !outcome.aborted,
        "an unchecked visit has no status test to abort from",
    );
    assert_eq!(outcome.copied_pixels, 16, "the rectangle transfers in full");
    assert_eq!(outcome.clicks, 8);
    assert_eq!(offered, vec![2, 4, 6, 8, 10, 12, 14, 16]);
}

#[test]
fn a_key_already_pending_leaves_exactly_one_start_menu_pixel_transferred() {
    // display-driver-abi.md §9.6: "The four-plane copy of the checked pixel
    // happens before its click and status test. Thus a key already pending
    // when the first start/menu dissolve begins leaves exactly one pixel
    // transferred before abort: the first visit, `(1,0)`."
    let mut dissolve = RectangleDissolve::new(INTRO_START_MENU_REVEAL_RECT).unwrap();
    let mut transferred = Vec::new();
    let outcome = dissolve.run_gated(&mut DissolveAbortGate::on_driver_load(), |visit| {
        transferred.push((visit.x(), visit.y()));
        visit.poll(true)
    });
    assert!(outcome.aborted);
    assert_eq!(outcome.copied_pixels, 1);
    assert_eq!(outcome.clicks, 1, "the aborting visit still clicks");
    assert_eq!(
        transferred,
        vec![(1, 0)],
        "the one transferred pixel is the published first visit",
    );
}

#[test]
fn a_dissolve_abort_completes_the_current_pixel_and_its_click() {
    // audio.md §8.6: "A pending key aborts after the current copied pixel
    // and the abort stops the speaker." Every DissolveClick program ends
    // with a speaker stop, so completing the aborting visit's click is what
    // leaves the speaker stopped.
    let (visits, aborted) = run_gated_dissolve(DissolveAbortGate::on_driver_load(), Some(5));
    assert!(aborted);
    assert_eq!(
        visits.len(),
        5,
        "the aborting pixel is copied, then nothing"
    );
    let last = visits.last().unwrap();
    assert_eq!(last.copied_pixels(), 5);
    assert!(last.samples_input());
    let click = last
        .sound()
        .cloned()
        .expect("the aborting visit still retunes");

    // audio.md §8.6.1 and RETRACTIONS.md R230: the retune does *not* silence
    // the speaker — "nothing disables it until the dissolve exits". The abort
    // completes this retune and then reaches the dissolve's shared exit, which
    // is the single silencing point for both the abort and normal completion.
    let mut jitter = crate::audio::RumbleJitter::new();
    assert!(
        !click.program(&mut jitter).ends_with_stop(),
        "an aborting retune must not stop the speaker by itself",
    );
    assert!(
        SoundEffect::DissolveExit
            .program(&mut jitter)
            .ends_with_stop(),
        "the shared exit the abort falls through is what stops the speaker",
    );
}

#[test]
fn endgame_restoration_sounds_once_per_restored_dead_member_and_nowhere_else() {
    // audio.md §8.7: "When a Dead party member is restored for the endgame
    // tableau, the sequence announces the restoration, fills the gameplay
    // rectangle once, runs software envelope (1, 5000, 40000, 1, 8800), and
    // redraws the full stats panel. It is a single blocking flourish per
    // restored member."
    let mut state = test_state(open_grid(), 5, 5);
    state.party = vec![state.party[0]; 3];
    state.party[1].slot = 1;
    state.party[1].class_byte = b'B';
    state.party[1].status = CharacterStatus::Dead.save_byte();
    state.party[1].hp = 0;
    state.party[1].max_hp = 77;
    state.party[2].slot = 2;
    state.party[2].class_byte = b'B';
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"SHAMINO\0\0"];

    state.enter_endgame();
    let serial = state.sound_effect_serial;

    // Lord British's walk-in and slot zero's walk are ordinary movement
    // beats: audio.md §8.7 names no cue for them.
    for _ in 0..9 {
        assert!(state.advance_endgame_entry_presentation());
    }
    assert!(state.sound_effects_after(serial).is_empty());

    // The restoration beat: the announcement, then the flourish.
    assert!(state.advance_endgame_entry_presentation());
    assert!(state.message.ends_with("\nIOLO lives!\n"));
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::EndgameRestoration]
    );

    // Slot two is Good, never Dead, so it is placed and walked in silence.
    let after_restoration = state.sound_effect_serial;
    for _ in 0..32 {
        if !state.advance_endgame_entry_presentation() {
            break;
        }
    }
    assert!(
        state.sound_effects_after(after_restoration).is_empty(),
        "only a Dead member's restoration flourishes",
    );
}

#[test]
fn endgame_orb_sting_lands_on_the_acknowledgement_not_on_the_box_to_orb_swap() {
    // endgame.md §7 step 7: "Change slot 6's actor byte to `0x08`, the Orb
    // spark - the box opens. A blocking key read and a speaker sting
    // follow." The key read precedes the sting, so the frame that performs
    // the swap must be silent and the acknowledgement that resolves that
    // read owns the cue. audio.md §8.7 supplies only the envelope: "The
    // later box/tableau presentation uses envelope (1, 10000, 50000, 1,
    // 5200)"; it defers the ordering to the owning system spec.
    let mut state = dungeon_state(endgame_tableau_test_grid(), 0, 1, 1);
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.enter_endgame();
    state.resolve_endgame_confirmation(true);
    state.resolve_endgame_confirmation(true);
    state.endgame.as_mut().unwrap().final_narrative = Some(synthetic_end_narrative());
    assert_eq!(
        endgame_tableau_role_for_slot(
            ENDGAME_TABLEAU_BOX_SLOT,
            state.active_objects[ENDGAME_TABLEAU_BOX_SLOT]
        ),
        Some(EndgameTableauActorRole::SandalwoodBox)
    );

    // Step 7, first half: the box opens into the Orb spark, and the
    // presentation parks on the blocking key read. No cue yet.
    let before_swap = state.sound_effect_serial;
    assert!(state.advance_endgame_display_frame());
    assert_eq!(
        state.active_objects[ENDGAME_TABLEAU_BOX_SLOT].type_byte,
        ENDGAME_TABLEAU_ORB_ACTOR_BYTE
    );
    assert_eq!(
        state.endgame.as_ref().map(|e| e.victory_tableau_phase),
        Some(EndgameVictoryTableauPhase::OrbAwaitingAcknowledgement),
        "the swap frame must stop at the blocking key read",
    );
    assert_eq!(
        state.sound_effects_after(before_swap),
        Vec::new(),
        "the box-to-Orb swap frame precedes the key read and is silent",
    );

    // Further automatic frames cannot slip past the key read, so no cue can
    // arrive before the player acknowledges.
    for _ in 0..8 {
        assert!(!state.advance_endgame_display_frame());
    }
    assert_eq!(
        state.sound_effects_after(before_swap),
        Vec::new(),
        "nothing sounds while the tableau waits on the key read",
    );

    // Step 7, second half: the acknowledgement resolves the key read and the
    // sting follows it. Step 8's slot clear and gate cell write happen on the
    // same beat, after the cue.
    let before_acknowledgement = state.sound_effect_serial;
    state.resolve_endgame_confirmation(true);
    assert_eq!(
        state.sound_effects_after(before_acknowledgement),
        vec![SoundEffect::EndgameTableau],
        "the sting sounds exactly once, on the acknowledgement",
    );
    assert_eq!(
        state.active_objects[ENDGAME_TABLEAU_BOX_SLOT].type_byte, 0,
        "step 8 clears slot 6 on the acknowledgement beat",
    );

    // The gate rise, hold, sink and actor-exit phases that follow have no
    // published cue in §8.7, and the post-certificate two-part rumble is
    // explicitly "not a live endgame trigger".
    let after_tableau = state.sound_effect_serial;
    for _ in 0..256 {
        if !state.advance_endgame_display_frame() {
            break;
        }
    }
    assert_eq!(state.sound_effects_after(after_tableau), Vec::new());
}

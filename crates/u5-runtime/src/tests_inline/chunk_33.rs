use crate::audio::HARPSICHORD_TUNE;

// `town-mode.md §13` — the Lord British's Castle harpsichord puzzle, at the
// town dispatch boundary. The pure predicates are covered in
// `harpsichord.rs`; the re-sync arithmetic is covered in `audio.rs`.

/// Party on the chair at (10, 10), harpsichord at (10, 11), so the passage
/// wall is (10, 6). `test_state` already binds `CASTLE:0`.
fn harpsichord_state() -> PlayState {
    let mut state = test_state(open_grid(), 10, 10);
    let Area::Town { scene, .. } = state.area else {
        unreachable!("town fixture");
    };
    state.area = Area::Town {
        scene,
        floor: HARPSICHORD_FLOOR,
    };
    state.grid[11 * 32 + 10] = HARPSICHORD_TILE;
    state.visibility_dirty = false;
    state
}

fn play_digits(state: &mut PlayState, digits: &[u8]) {
    let game_dir = Path::new(".");
    for digit in digits {
        assert_eq!(
            handle_play_key_input(
                state,
                char::from_digit(u32::from(*digit), 10).unwrap(),
                "",
                game_dir,
            )
            .unwrap(),
            PlayInputDisposition::Continue
        );
    }
}

#[test]
fn town_digit_falls_through_to_the_ordinary_dispatcher_when_not_seated() {
    let mut state = harpsichord_state();
    state.grid[11 * 32 + 10] = 16;

    // `6` already carries an ordinary meaning in this harness — the numeric
    // step east — and an un-seated digit must produce exactly that, turn
    // included.
    play_digits(&mut state, &[6]);
    assert_eq!(state.message, "");
    assert_eq!((state.player.x, state.player.y), (11, 10));
    assert_eq!(state.turn, 1);

    // `5` has no ordinary binding, so the forwarded result is the
    // dispatcher's own refusal rather than anything the instrument printed.
    play_digits(&mut state, &[5]);
    assert_eq!(state.message, "Unhandled command `5`.");

    assert_eq!(state.harpsichord_progress(), 0);
    assert!(state.sound_effects_after(0).is_empty());
}

#[test]
fn town_digit_reaches_the_instrument_only_from_the_cell_north_of_it() {
    let mut state = harpsichord_state();
    // The harpsichord one cell *north* of the party is not the chair.
    state.grid[11 * 32 + 10] = 16;
    state.grid[9 * 32 + 10] = HARPSICHORD_TILE;

    play_digits(&mut state, &[5]);
    assert_eq!(state.message, "Unhandled command `5`.");
    assert!(state.sound_effects_after(0).is_empty());

    state.grid[9 * 32 + 10] = 16;
    state.grid[11 * 32 + 10] = HARPSICHORD_TILE;
    state.message = String::new();

    play_digits(&mut state, &[5]);
    assert_eq!(
        state.sound_effects_after(0),
        vec![SoundEffect::HarpsichordNote { digit: 5 }]
    );
}

#[test]
fn seated_harpsichord_digit_consumes_no_turn_and_advances_no_world_time() {
    let mut state = harpsichord_state();
    let turn_before = state.turn;
    let clock_before = state.clock;

    play_digits(&mut state, &[6, 7, 8]);

    assert_eq!(state.turn, turn_before);
    assert_eq!(state.clock, clock_before);
    assert!(!state.visibility_dirty);
    assert_eq!(state.harpsichord_progress(), 3);
}

#[test]
fn harpsichord_note_is_recorded_only_while_the_sound_setting_is_on() {
    let mut state = harpsichord_state();
    play_digits(&mut state, &[6]);
    assert_eq!(
        state.sound_effects_after(0),
        vec![SoundEffect::HarpsichordNote { digit: 6 }]
    );

    let serial = state.sound_effect_serial;
    state.music_enabled = false;
    play_digits(&mut state, &[7]);

    assert!(state.sound_effects_after(serial).is_empty());
    // The note is still counted even though it was not sounded.
    assert_eq!(state.harpsichord_progress(), 2);
}

#[test]
fn thirteen_note_tune_opens_the_wall_five_squares_north_of_the_harpsichord() {
    let mut state = harpsichord_state();
    assert_eq!(state.grid[6 * 32 + 10], 16);

    play_digits(&mut state, &HARPSICHORD_TUNE[..HARPSICHORD_TUNE.len() - 1]);
    assert_eq!(state.grid[6 * 32 + 10], 16);
    assert!(!state.visibility_dirty);
    assert_eq!(state.harpsichord_progress(), 12);

    play_digits(&mut state, &HARPSICHORD_TUNE[HARPSICHORD_TUNE.len() - 1..]);

    assert_eq!(state.grid[6 * 32 + 10], HARPSICHORD_PASSAGE_CLEARED_TILE);
    assert!(state.visibility_dirty);
    assert_eq!(state.harpsichord_progress(), 0);
    assert_eq!(state.turn, 0);
}

#[test]
fn wrong_note_resyncs_to_the_longest_still_playable_beginning_of_the_tune() {
    // Each published worked case is driven to completion from the progress it
    // claims to leave, so the surviving progress is observed in the tile
    // buffer and not just in the counter.
    for (correct, stray, expected) in [(10usize, 8u8, 3usize), (11, 7, 2), (4, 6, 1), (4, 2, 0)] {
        let mut state = harpsichord_state();
        play_digits(&mut state, &HARPSICHORD_TUNE[..correct]);
        play_digits(&mut state, &[stray]);

        assert_eq!(
            state.harpsichord_progress(),
            expected,
            "{correct} correct notes then a stray {stray}"
        );
        assert_eq!(state.grid[6 * 32 + 10], 16);

        play_digits(&mut state, &HARPSICHORD_TUNE[expected..]);
        assert_eq!(
            state.grid[6 * 32 + 10],
            HARPSICHORD_PASSAGE_CLEARED_TILE,
            "{correct} correct notes then a stray {stray}"
        );
    }
}

#[test]
fn harpsichord_progress_is_not_cleared_by_leaving_the_chair() {
    let mut state = harpsichord_state();
    play_digits(&mut state, &HARPSICHORD_TUNE[..6]);
    assert_eq!(state.harpsichord_progress(), 6);

    // Step off the chair, key a digit the instrument never sees, then sit
    // back down.
    state.player.x = 4;
    state.player.y = 4;
    play_digits(&mut state, &[5]);
    assert_eq!(state.message, "Unhandled command `5`.");
    assert_eq!(state.harpsichord_progress(), 6);

    state.player.x = 10;
    state.player.y = 10;
    play_digits(&mut state, &HARPSICHORD_TUNE[6..]);

    assert_eq!(state.grid[6 * 32 + 10], HARPSICHORD_PASSAGE_CLEARED_TILE);
}

#[test]
fn harpsichord_completion_is_gated_on_lord_britishs_castle_and_floor_two() {
    for (scene_byte, floor) in [
        (SCENE_LORD_BRITISHS_CASTLE, 0i8),
        (SCENE_LORD_BRITISHS_CASTLE, 1),
        (SCENE_LORD_BLACKTHORNS_CASTLE, HARPSICHORD_FLOOR),
    ] {
        let mut state = harpsichord_state();
        state.area = Area::Town {
            scene: Scene::new(scene_byte).unwrap(),
            floor,
        };

        play_digits(&mut state, &HARPSICHORD_TUNE);

        // The instrument still plays and the tune still completes — arming and
        // scoring are position-only — but no passage opens.
        assert_eq!(state.grid[6 * 32 + 10], 16, "scene {scene_byte} floor {floor}");
        assert!(!state.visibility_dirty);
        assert_eq!(state.harpsichord_progress(), 0);
        assert_eq!(
            state.sound_effects_after(0).len(),
            HARPSICHORD_TUNE.len(),
            "scene {scene_byte} floor {floor}"
        );
    }
}

#[test]
fn harpsichord_passage_rewrite_is_live_buffer_only() {
    let mut state = harpsichord_state();
    play_digits(&mut state, &HARPSICHORD_TUNE);

    assert_eq!(state.grid[6 * 32 + 10], HARPSICHORD_PASSAGE_CLEARED_TILE);
    // Nothing about the rewrite is recorded for the save or for a floor
    // reload the way an opened door is.
    assert!(state.opened_town_doors.is_empty());
    assert!(state.revealed_town_secret_doors.is_empty());
    assert!(state.town_npc_mutations.is_empty());
}

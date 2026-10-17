// `endgame.md §7` steps 4-6: the victory branch's box hand-over reads no key.

/// "Step slot 0 from its settled position to (5, 4), then back to (5, 5)
/// ... After four more paced iterations, create the **sandalwood box** ...
/// Print the seven rite messages" - none of which reads a key. A key typed
/// while they run stays queued until they finish; it used to finish the
/// hand-over at once and land on record `3`.
#[test]
fn a_key_during_the_box_hand_over_waits_until_it_finishes() {
    let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 1;
    state.enter_endgame();
    // `endgame.md §5`: a first Yes with the box owned is the effective answer.
    state.resolve_endgame_confirmation(true);
    assert!(state.endgame_victory_staging_pending());
    let message_before = state.message.clone();
    let phase_before = state.endgame.as_ref().unwrap().victory_staging;

    handle_play_key_input(&mut state, ' ', "", Path::new("")).unwrap();

    // Nothing moved on: the hand-over is where it was, no box, no record 3.
    assert!(state.endgame_victory_staging_pending());
    assert_eq!(state.endgame.as_ref().unwrap().victory_staging, phase_before);
    assert_ne!(
        state.active_objects[ENDGAME_TABLEAU_BOX_SLOT].type_byte,
        ENDGAME_TABLEAU_BOX_ACTOR_BYTE
    );
    assert_eq!(state.message, message_before);
    assert_eq!(state.keys_buffered_during_hold.len(), 1);
    assert!(!state.buffered_hold_key_ready());
    assert_eq!(
        crate::input_dispatch::drain_buffered_hold_key(&mut state, Path::new("")).unwrap(),
        None
    );
    assert_eq!(state.keys_buffered_during_hold.len(), 1);

    // The display pump runs the hand-over to the box and record `3`.
    for _ in 0..64 {
        if !state.endgame_victory_staging_pending() {
            break;
        }
        state.advance_endgame_display_frame();
        state.flush_staged_narration();
    }
    assert!(!state.endgame_victory_staging_pending());
    assert_eq!(
        state.active_objects[ENDGAME_TABLEAU_BOX_SLOT].type_byte,
        ENDGAME_TABLEAU_BOX_ACTOR_BYTE
    );
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (5, 5)
    );

    // Only now is the queued key read.
    assert!(state.buffered_hold_key_ready());
    assert!(
        crate::input_dispatch::drain_buffered_hold_key(&mut state, Path::new(""))
            .unwrap()
            .is_some()
    );
    assert!(state.keys_buffered_during_hold.is_empty());
}

// Regressions for the u5-spec #298 answer leftovers (spec 395e73f..4506225),
// triage batch B. Each test names the published sentence it pins.

/// W16. `active-objects.md §3`: byte 6 is the "Packed animation byte ...
/// Advanced by the animator", and §8's outdoor walker "performs no drawing,
/// no frame advance and no cleanup at all". A consumed turn's walker pass
/// must leave a slot's frame-delay countdown alone; only the world-tick
/// animator decrements it.
#[test]
fn triage_b_outdoor_walker_leaves_the_animator_phase_byte_alone() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.active_objects.push(ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: 20,
        y: 20,
        z: WorldPlane::Underworld.save_floor(),
        // Script step 2, a four-tick frame delay still to run.
        phase: 0x24,
        aux1: 0,
        aux3: 0,
    });

    state.advance_outdoor_active_objects();
    assert_eq!(
        state.active_objects[1].phase, 0x24,
        "the walker does not advance the animator's countdown"
    );

    // The animator is the byte's owner: one world tick takes one off.
    state.animate_active_objects();
    assert_eq!(state.active_objects[1].phase, 0x23);
}

/// W12. `blackthorn.md §5`: the correct-answer execution is record `5`, the
/// quiet pause, the swell, the impact, the sprite clear, the pendulum reset,
/// the roster edit and its repaint, and it "also finishes with one line feed"
/// - step 9, the last step. The feed (the blank row under record `5`) comes at
/// the end of the blade hold with the reduced panel, not with the record.
#[test]
fn triage_b_merciful_death_closing_feed_follows_the_blade_hold() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.repaint_stats_panel();
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }

    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    assert!(state.staged_narration_active(), "the blade hold is running");
    assert!(state.message.contains("merciful death"), "{:?}", state.message);
    let record_tail = state.message_entries().last().expect("record 5 is on screen").clone();
    assert!(
        !record_tail.text.is_empty(),
        "no feed yet: the last row is still record 5's, {record_tail:?}"
    );

    state.flush_staged_narration();
    assert_eq!(state.party.len(), 2, "the hold's end lifts the companion");
    let entries = state.message_entries();
    let last = entries.last().unwrap();
    assert!(last.text.is_empty(), "step 9's feed closes the routine: {last:?}");
    assert_eq!(
        entries[entries.len() - 2].text, record_tail.text,
        "and it sits directly under record 5"
    );
    let _ = fs::remove_dir_all(dir);
}

/// W11. `inventory.md §4.4` (u5-spec#296): once a picker pass leaves the
/// panel window's inverse bit set, "every later panel repaint" draws inverted;
/// "Turns, movement and messages do not clear it ... It flips back only when
/// a later picker session leaves odd parity again." Entering a location
/// rebuilds the whole `PlayState`, which must not reset it.
#[test]
fn triage_b_panel_inverse_bit_survives_a_location_entry() {
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 7;
    let mut state = world_state(grid, 10, 20);
    state.stats_panel_window_inverse = true;

    assert_eq!(
        state
            .enter_world_target(&dir, WorldPlane::Underworld, PlayTarget::Town(scene), true)
            .unwrap(),
        MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
    );
    assert_eq!(state.area, Area::Town { scene, floor: 0 });
    assert!(state.stats_panel_window_inverse, "the leak outlives the scene change");
    let _ = fs::remove_dir_all(dir);
}

/// W10. `endgame.md §6` step 3 (issue #298 Q42): each wander call's
/// display-tick advance "is exactly **one paced iteration** - a world tick
/// plus a one-tick wait request - and it is paid by every call". The pump
/// therefore makes one call per frame, each followed by its own one-tick hold,
/// and cycles slots 1, 3, 4, 5.
#[test]
fn triage_b_terminal_jitter_pays_one_tick_between_each_wander_call() {
    let mut state = test_state(endgame_tableau_test_grid(), 5, 5);
    state.party = vec![state.party[0]; 6];
    state.enter_endgame();
    state.finish_endgame_entry_presentation();
    state.resolve_endgame_confirmation(true);
    assert_eq!(
        state.endgame.as_ref().and_then(|endgame| endgame.outcome),
        Some(EndgameOutcome::MissingBoxOrRefused)
    );
    state.finish_endgame_refusal_staging();
    state.flush_staged_narration();

    let jitter = [1usize, 3, 4, 5];
    for round in 0..3 {
        for (index, &slot) in jitter.iter().enumerate() {
            assert_eq!(
                usize::from(state.endgame.as_ref().unwrap().terminal_jitter_next_call),
                index,
                "round {round}: call {index} is slot {slot}'s"
            );
            let before: Vec<(usize, usize)> = jitter
                .iter()
                .map(|&s| (state.active_objects[s].x, state.active_objects[s].y))
                .collect();
            assert!(state.advance_endgame_display_frame());
            let moved: Vec<usize> = jitter
                .iter()
                .enumerate()
                .filter(|&(i, &s)| {
                    (state.active_objects[s].x, state.active_objects[s].y) != before[i]
                })
                .map(|(_, &s)| s)
                .collect();
            assert!(moved.is_empty() || moved == vec![slot], "only slot {slot} may step: {moved:?}");
            let holds: Vec<f64> =
                state.staged_narration.beats().map(|beat| beat.delay_secs).collect();
            assert_eq!(holds.len(), 1, "one paced iteration per call");
            assert!((holds[0] - crate::narration::bios_ticks_secs(1)).abs() < 1e-9);
            state.flush_staged_narration();
        }
    }
}

/// W8. `endgame.md §7` steps 4-6 (issue #298 Q41): "Step slot 0 from its
/// settled position to (5, 4), then back to (5, 5) ... Eight paced iterations
/// precede the first step. After four more paced iterations, create the
/// **sandalwood box** in slot 6 with actor byte `0x0E` at (5, 4)", then the
/// rite pages "(record 3, forty paced iterations, `He says:`, ...)".
#[test]
fn triage_b_victory_hands_the_box_over_on_paced_frames_before_record_three() {
    let labels = [
        "Welcome back",
        "Hast thou brought my box?",
        "The sandalwood box itself?",
        "Rite 1",
        "Rite 2",
        "Rite 3",
        "Rite 4",
        "Rite 5",
        "Rite 6",
        "Rite 7",
        "Wait here without the box",
    ];
    let mut bytes = Vec::new();
    for label in labels {
        bytes.extend_from_slice(label.as_bytes());
        bytes.push(0x00);
    }
    let messages = parse_endgame_messages(&bytes).unwrap();
    let mut state = test_state(endgame_tableau_test_grid(), 5, 5);
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    state.enter_endgame_with_messages(Some(messages));
    state.finish_endgame_entry_presentation();
    state.flush_staged_narration();
    assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (5, 5));

    state.resolve_endgame_confirmation(true);
    let staging = |state: &PlayState| state.endgame.as_ref().unwrap().victory_staging;
    let box_slot = |state: &PlayState| state.active_objects.get(ENDGAME_TABLEAU_BOX_SLOT).copied();
    let holds = |state: &PlayState| -> Vec<f64> {
        state.staged_narration.beats().map(|beat| beat.delay_secs).collect()
    };
    let ticks = |n: u16| crate::narration::bios_ticks_secs(n);
    assert_eq!(staging(&state), EndgameVictoryStagingPhase::LeadPauseOwed);
    assert_eq!(state.message, "", "record 3 waits for the box");
    assert!(box_slot(&state).is_none_or(|object| object.is_empty()));

    // Step 4: eight paced iterations, then the walk up.
    assert!(state.advance_endgame_display_frame());
    assert_eq!(holds(&state).len(), 1);
    assert!((holds(&state)[0] - ticks(8)).abs() < 1e-9);
    assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (5, 5));
    state.flush_staged_narration();

    let serial = state.sound_effect_serial;
    assert!(state.advance_endgame_display_frame());
    assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (5, 4));
    assert!(
        state
            .sound_effects_after(serial)
            .contains(&SoundEffect::EndgameTableauMovementSting),
        "a target-directed step carries the movement sting (audio.md §8.7)"
    );
    assert!((holds(&state)[0] - ticks(5)).abs() < 1e-9);
    state.flush_staged_narration();

    // ... and back.
    assert!(state.advance_endgame_display_frame());
    assert_eq!((state.active_objects[0].x, state.active_objects[0].y), (5, 5));
    state.flush_staged_narration();

    // Step 5: four more paced iterations, then the box.
    assert!(state.advance_endgame_display_frame());
    assert!((holds(&state)[0] - ticks(4)).abs() < 1e-9);
    assert!(box_slot(&state).is_none_or(|object| object.is_empty()));
    assert_eq!(state.message, "");
    state.flush_staged_narration();

    assert!(state.advance_endgame_display_frame());
    let placed = box_slot(&state).expect("the box slot exists");
    assert_eq!(
        (placed.type_byte, placed.x, placed.y),
        (ENDGAME_TABLEAU_BOX_ACTOR_BYTE, 5, 4)
    );
    // Step 6: record 3, with the §5.1 forty-iteration pause still to run.
    assert_eq!(state.message, "Rite 1");
    assert_eq!(staging(&state), EndgameVictoryStagingPhase::Inactive);
    assert_eq!(
        state.endgame.as_ref().unwrap().cinematic.rite_pause_ticks_remaining,
        crate::endgame_cinematic::ENDGAME_RITE_LEAD_IN_PAUSE_TICKS
    );
}

/// W9. `endgame.md §4` (issue #298 Q43): "The Blackthorn rescue's crowned
/// spectral figure (`0x74`, also script A) likewise animates during its
/// verdict key wait" - frames `0x174..0x177` on the shared one-in-two coin,
/// world ticks running while step 19 blocks.
#[test]
fn triage_b_rescue_crowned_figure_animates_through_the_verdict_key_wait() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].status = b'D';
    state.party[0].hp = 0;
    state.apply_blackthorn_rescue_refuge(&dir).unwrap();
    assert!(!state.blackthorn_rescue_awaiting_acknowledgement());
    assert_eq!(
        state.blackthorn_rescue_spectral_atlas_index(),
        BLACKTHORN_RESCUE_SPECTRAL_ATLAS_INDEX,
        "the reveal draws the plain 0x174"
    );
    // Nothing animates while the narration is still running.
    assert!(!state.advance_blackthorn_rescue_animation_to(0.0));

    state.flush_staged_narration();
    state.step_blackthorn_rescue(&dir, false).unwrap();
    assert!(state.blackthorn_rescue_awaiting_acknowledgement());
    let figure = state.blackthorn_rescue_spectral_actor;
    assert_eq!((figure.type_byte, figure.tile), (0x74, 0x74));
    assert_eq!((figure.x, figure.y), (5, 2));

    // The first look anchors the clock; every later tick runs the animator.
    let tick = crate::narration::BIOS_TICK_SECS;
    state.advance_blackthorn_rescue_animation_to(0.5 * tick);
    let mut frames = std::collections::BTreeSet::new();
    for n in 1..=200u32 {
        state.advance_blackthorn_rescue_animation_to((f64::from(n) + 0.5) * tick);
        let figure = state.blackthorn_rescue_spectral_actor;
        assert_eq!(figure.tile & !0x03, 0x74, "it stays in its own four-frame run");
        assert_eq!(
            state.blackthorn_rescue_spectral_atlas_index(),
            0x100 + u16::from(figure.tile)
        );
        frames.insert(figure.tile);
    }
    assert!(frames.len() > 1, "the figure animates: {frames:?}");

    // The key ends the wait; the animator stops with it.
    state.step_blackthorn_rescue(&dir, true).unwrap();
    assert!(!state.blackthorn_rescue_awaiting_acknowledgement());
    let held = state.blackthorn_rescue_spectral_actor;
    assert!(!state.advance_blackthorn_rescue_animation_to(1000.0 * tick));
    assert_eq!(state.blackthorn_rescue_spectral_actor, held);
    let _ = fs::remove_dir_all(dir);
}

/// W15. `dungeon-mode.md §6.4`: the flavour-3 flourish's draw "is consumed on
/// every render that tests such a wall", and the dungeon loop renders once per
/// iteration (`§10` inner loop, step 2 "Render and poll"). However often a
/// frontend repaints between keys, the iteration takes one draw, and every
/// repaint shows that draw's result.
#[test]
fn triage_b_dungeon_flourish_takes_one_draw_per_loop_iteration() {
    let mut grid = open_dungeon_record();
    grid[dungeon_cell_index(0, 5, 4)] = 0xc0;
    let mut state = dungeon_state(grid, 0, 4, 4);
    state.area = Area::Dungeon {
        // Record 0 is Deceit, a flavour-3 dungeon.
        scene: DungeonScene::from_record(0).unwrap(),
        level: 0,
    };
    state.player.facing = Direction::East;
    state.torch_counter = 10;
    let hit = (1..=u16::MAX)
        .find(|&seed| u5_prng_range_u16(&mut seed.clone(), 0, 64) < 4)
        .unwrap();
    state.prng_state = hit;
    let eyes = |state: &mut PlayState| {
        let viewport = state
            .render_dungeon_viewport(4, TileGraphicsDepth::Ega16, None, None)
            .unwrap();
        DUNGEON_FLOURISH_RUNS
            .iter()
            .flat_map(|&(y, first, last)| (first..=last).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                viewport.pixels[(y as usize - VIEWPORT_ORIGIN_Y) * viewport.width
                    + (x as usize - VIEWPORT_ORIGIN_X)]
                    == DUNGEON_FLOURISH_PEN
            })
            .count()
    };

    let mut after_one = hit;
    u5_prng_range_u16(&mut after_one, 0, 64);
    assert_eq!(eyes(&mut state), 10);
    assert_eq!(state.prng_state, after_one);
    // Repaints inside the same iteration reuse the draw.
    for _ in 0..5 {
        assert_eq!(eyes(&mut state), 10, "the lit eyes stay lit until the next iteration");
        assert_eq!(state.prng_state, after_one, "and no further draw is taken");
    }

    // The next iteration renders afresh and draws again.
    let iteration = state.dungeon_loop_iteration;
    state.advance_dungeon_loop_minute();
    assert_eq!(state.dungeon_loop_iteration, iteration + 1);
    let before = state.prng_state;
    let mut expected = before;
    let lit = u5_prng_range_u16(&mut expected, 0, 64) < 4;
    assert_eq!(eyes(&mut state), if lit { 10 } else { 0 });
    assert_eq!(state.prng_state, expected);
}

/// W10 follow-up. A keystroke-driven round is four wander calls on the same
/// cursor the display pump uses, so a key that lands mid-round continues the
/// slot order (`endgame.md §6` step 3: slots 1, 3, 4, 5) rather than
/// restarting it at slot 1.
#[test]
fn triage_b_keyed_jitter_round_continues_the_pump_cursor() {
    let mut state = test_state(endgame_tableau_test_grid(), 5, 5);
    state.party = vec![state.party[0]; 6];
    state.enter_endgame();
    state.finish_endgame_entry_presentation();
    state.resolve_endgame_confirmation(true);
    state.finish_endgame_refusal_staging();
    state.flush_staged_narration();

    assert!(state.advance_endgame_display_frame());
    assert!(state.advance_endgame_display_frame());
    state.flush_staged_narration();
    assert_eq!(state.endgame.as_ref().unwrap().terminal_jitter_next_call, 2);

    state.advance_endgame_terminal_tableau_jitter();
    assert_eq!(
        state.endgame.as_ref().unwrap().terminal_jitter_next_call,
        2,
        "a full round of four calls lands back on the same slot"
    );
    assert_eq!(state.staged_narration.beats().count(), 4);
}

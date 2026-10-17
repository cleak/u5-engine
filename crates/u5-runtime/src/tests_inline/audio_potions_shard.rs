// `systems/audio.md` trigger-boundary regressions: potions shard.
//
// Each test names the published clause it pins. Add tests here rather
// than to the numbered chunks so the audio work stays reviewable as a
// unit.

#[test]
fn accepted_potion_takes_its_shared_variant_from_the_selected_bottle() {
    // audio.md §7.2: "The selected bottle id, not the later variation roll,
    // chooses variant 0 through 7 from Section 6." Every bottle is driven with
    // a deliberately mismatched effect id so the cue can only be following the
    // selection.
    for selected in 0..POTION_COUNT {
        let mut state = test_state(open_grid(), 4, 4);
        let serial = state.sound_effect_serial;

        state.use_potion_with_effect(selected, 0, POTION_BLUE_INDEX);

        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::SharedVariant {
                variant: selected as u8
            }],
            "bottle {selected} must own its own variant"
        );
    }
}

#[test]
fn accepted_potion_command_emits_one_variant_before_its_narration() {
    // audio.md §7.2: the bottle is decremented and a party-member target is
    // accepted before the shared presentation begins, and the presentation is
    // the first thing the accepted use does.
    let mut state = test_state(open_grid(), 4, 4);
    state.potion_stock[POTION_YELLOW_INDEX] = 1;
    state.party[0].hp = 5;
    let serial = state.sound_effect_serial;

    assert_eq!(
        state.use_potion(POTION_YELLOW_INDEX, Some(0)),
        MoveOutcome::Used
    );

    assert_eq!(state.potion_stock[POTION_YELLOW_INDEX], 0);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant {
            variant: POTION_YELLOW_INDEX as u8
        }]
    );
    // audio.md §6 brackets the two envelopes with the paired viewport
    // inversions, which is the flash this command also publishes.
    assert!(state.pending_potion_flash.is_some());
    assert_eq!(state.message, POTION_RESULT_HEALED);
}

#[test]
fn potion_refusals_and_cancellations_before_target_acceptance_are_silent() {
    // audio.md §7.2: "Cancellation before target acceptance skips the
    // presentation."
    let mut no_stock = test_state(open_grid(), 4, 4);
    let serial = no_stock.sound_effect_serial;
    assert_eq!(
        no_stock.use_potion(POTION_BLUE_INDEX, Some(0)),
        MoveOutcome::Blocked
    );
    assert!(no_stock.sound_effects_after(serial).is_empty());

    // Decremented, but no target was ever named.
    let mut no_target = test_state(open_grid(), 4, 4);
    no_target.potion_stock[POTION_RED_INDEX] = 1;
    let serial = no_target.sound_effect_serial;
    assert_eq!(
        no_target.use_potion(POTION_RED_INDEX, None),
        MoveOutcome::Blocked
    );
    assert_eq!(no_target.potion_stock[POTION_RED_INDEX], 0);
    assert_eq!(no_target.message, USE_POTION_TARGET_PROMPT);
    assert!(no_target.sound_effects_after(serial).is_empty());

    // Decremented, but the named party slot does not exist, so the target is
    // never accepted.
    let mut bad_target = test_state(open_grid(), 4, 4);
    bad_target.potion_stock[POTION_RED_INDEX] = 1;
    let serial = bad_target.sound_effect_serial;
    let missing_slot = bad_target.party.len();
    assert_eq!(
        bad_target.use_potion(POTION_RED_INDEX, Some(missing_slot)),
        MoveOutcome::Blocked
    );
    assert!(bad_target.sound_effects_after(serial).is_empty());
    assert!(bad_target.pending_potion_flash.is_none());
}

#[test]
fn a_scroll_use_plays_its_own_scroll_index_not_its_spells_variant() {
    // audio.md §6.1: "A scroll supplies its **scroll index**, 0 through 7",
    // and "a frontend must not reuse the spell's variant for the scroll".
    // Light is scroll 0 against In Lor's variant 1, View is scroll 4, and
    // Protection is scroll 2 against the spell's variant 4.
    let mut state = test_state(open_grid(), 4, 4);
    state.scroll_stock[SCROLL_LIGHT_INDEX] = 1;
    state.scroll_stock[SCROLL_VIEW_INDEX] = 1;
    state.scroll_stock[SCROLL_PROTECTION_INDEX] = 1;
    let serial = state.sound_effect_serial;

    assert_eq!(
        state.use_scroll(SCROLL_LIGHT_INDEX, None, None),
        MoveOutcome::Used
    );
    assert_eq!(
        state.use_scroll(SCROLL_VIEW_INDEX, None, None),
        MoveOutcome::Observed
    );
    assert_eq!(
        state.use_scroll(SCROLL_PROTECTION_INDEX, None, None),
        MoveOutcome::Used
    );

    assert_eq!(
        state.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 0 },
            SoundEffect::SharedVariant { variant: 4 },
            SoundEffect::SharedVariant { variant: 2 },
        ]
    );
}

#[test]
fn a_scroll_refused_outside_its_scene_class_stays_silent() {
    // audio.md §6.1: the View and Summon Daemon scrolls are "Refused with
    // `Not here!` and **no sound** outside the permitted scene class"; §9
    // lists both refusals among the explicit silence boundaries.
    let mut state = test_state(open_grid(), 4, 4);
    state.combat_active = true;
    state.scroll_stock[SCROLL_VIEW_INDEX] = 1;
    let serial = state.sound_effect_serial;
    assert_eq!(
        state.use_scroll(SCROLL_VIEW_INDEX, None, None),
        MoveOutcome::Blocked
    );
    assert_eq!(state.message, "Not here!");
    assert!(state.sound_effects_after(serial).is_empty());

    let mut out_of_combat = test_state(open_grid(), 4, 4);
    out_of_combat.scroll_stock[SCROLL_SUMMON_DAEMON_INDEX] = 1;
    let serial = out_of_combat.sound_effect_serial;
    assert_eq!(
        out_of_combat.use_scroll(SCROLL_SUMMON_DAEMON_INDEX, None, None),
        MoveOutcome::Blocked
    );
    assert_eq!(out_of_combat.message, "Not here!");
    assert!(out_of_combat.sound_effects_after(serial).is_empty());
}

#[test]
fn the_negate_time_scroll_sounds_the_failure_glissando_in_its_two_dead_scenes() {
    // audio.md §6.1 scroll 7: "In two specific scenes it instead prints
    // `No effect!` and plays the 50-update cast-failure glissando."
    let mut dead_scene = test_state(open_grid(), 4, 4);
    dead_scene.area = Area::Town {
        scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
        floor: 0,
    };
    dead_scene.scroll_stock[SCROLL_NEGATE_TIME_INDEX] = 1;
    let serial = dead_scene.sound_effect_serial;
    assert_eq!(
        dead_scene.use_scroll(SCROLL_NEGATE_TIME_INDEX, None, None),
        MoveOutcome::Blocked
    );
    assert_eq!(dead_scene.message, "No effect!");
    assert_eq!(
        dead_scene.sound_effects_after(serial),
        vec![SoundEffect::CastFailure]
    );

    // Anywhere else the scroll takes its own index, 7.
    let mut ordinary = test_state(open_grid(), 4, 4);
    ordinary.scroll_stock[SCROLL_NEGATE_TIME_INDEX] = 1;
    let serial = ordinary.sound_effect_serial;
    assert_eq!(
        ordinary.use_scroll(SCROLL_NEGATE_TIME_INDEX, None, None),
        MoveOutcome::Used
    );
    assert_eq!(
        ordinary.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 7 }]
    );
}

#[test]
fn the_wind_change_scroll_plays_variant_one_whatever_the_old_wind_was() {
    // audio.md §7.3, as corrected: "The variant is chosen by the caller tag,
    // not by the wind." The scroll tag is variant 1 on every accepted
    // transition, "so requesting the already-active direction still sounds",
    // and the old wind does not participate. The previous-wind matrix this
    // test used to pin is withdrawn (RETRACTIONS.md). A cancelled direction
    // prompt is still silent - "the cast never reaches the setter".
    let mut cancelled = test_state(open_grid(), 4, 4);
    cancelled.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 1;
    let serial = cancelled.sound_effect_serial;
    assert_eq!(
        cancelled.use_scroll(SCROLL_WIND_CHANGE_INDEX, None, None),
        MoveOutcome::Blocked
    );
    assert!(cancelled.sound_effects_after(serial).is_empty());

    let mut state = test_state(open_grid(), 4, 4);
    state.scroll_stock[SCROLL_WIND_CHANGE_INDEX] = 2;
    state.wind = WindState::Calm;
    let serial = state.sound_effect_serial;
    assert_eq!(
        state.use_scroll(SCROLL_WIND_CHANGE_INDEX, Some(Direction::North), None),
        MoveOutcome::Used
    );
    assert_eq!(
        state.use_scroll(SCROLL_WIND_CHANGE_INDEX, Some(Direction::North), None),
        MoveOutcome::Used
    );
    assert_eq!(
        state.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 1 },
            SoundEffect::SharedVariant { variant: 1 },
        ],
        "the scroll's caller tag is variant 1 out of Calm and out of a direction alike"
    );
}

#[test]
fn the_wind_change_spell_plays_variant_two_where_the_scroll_plays_one() {
    // audio.md §7.3 caller-tag table: the spell (Rel Hur, id 8, circle 2) is
    // variant 2 and the scroll (index 1) is variant 1. §6.1: "A frontend must
    // not reuse the spell's variant for the scroll."
    let mut spell = test_state(open_grid(), 4, 4);
    spell.wind = WindState::Calm;
    let serial = spell.sound_effect_serial;
    assert!(spell.apply_wind_state(WindState::North));
    // Re-requesting the already-active direction changes nothing but still
    // sounds: "requesting the already-active direction still sounds".
    assert!(!spell.apply_wind_state(WindState::North));
    assert_eq!(
        spell.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 2 },
            SoundEffect::SharedVariant { variant: 2 },
        ],
        "every accepted spell-tagged transition is variant 2"
    );

    let mut scroll = test_state(open_grid(), 4, 4);
    scroll.wind = WindState::Calm;
    let serial = scroll.sound_effect_serial;
    assert!(scroll.apply_wind_state_from_scroll(WindState::North));
    assert_eq!(
        scroll.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 1 }]
    );
}

#[test]
fn silent_wind_setter_commits_the_transition_without_a_cue() {
    // audio.md §7.3 is titled "Accepted wind change / Rel Hur" and every row of
    // its table is a requested transition. The unprompted weather drift has no
    // published trigger, so it commits through the silent entry point.
    let mut state = test_state(open_grid(), 4, 4);
    state.wind = WindState::Calm;
    let serial = state.sound_effect_serial;

    assert!(state.apply_wind_state_without_sound(WindState::North));
    assert_eq!(state.wind, WindState::North);
    assert!(state.sound_effects_after(serial).is_empty());

    // The prompted setter on the same transition still sounds.
    let mut prompted = test_state(open_grid(), 4, 4);
    prompted.wind = WindState::Calm;
    let serial = prompted.sound_effect_serial;
    assert!(prompted.apply_wind_state(WindState::North));
    assert_eq!(
        prompted.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 2 }]
    );
}

#[test]
fn shadowlord_destruction_plays_no_major_flash() {
    // `audio.md §8.4` (u5-spec#296, `RETRACTIONS.md` R523): whether a
    // successful destruction invokes the shared full-viewport flash "is **not
    // established**", so the engine no longer runs it there - neither its
    // sound nor its 1,856 gameplay-PRNG draws.
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 15, 9);
    state.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 2,
    };
    state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
    state.summoned_shadowlord = Some(SHADOWLORD_FALSEHOOD_INDEX);
    let z = state.current_floor().unwrap();
    state.active_objects.push(
        state
            .shadowlord_name_encounter_object(SHADOWLORD_FALSEHOOD_INDEX, 15, 8, z)
            .unwrap(),
    );
    let serial = state.sound_effect_serial;
    let prng_before = state.prng_state;

    assert_eq!(
        state
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
            .unwrap(),
        MoveOutcome::Used
    );

    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::ShardSwell],
        "quest-graph.md §5 step 2's swell, and no flash"
    );
    assert_eq!(state.prng_state, prng_before);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn shadowlord_shard_refusals_never_flash() {
    // No refusal arm flashes: `audio.md §8.4` (R523) "On the no-effect arm no
    // flash follows", and the silent gates add nothing after the swell.
    let dir = debug_game_dir();

    let mut no_shard = test_state(open_grid(), 15, 9);
    let serial = no_shard.sound_effect_serial;
    assert_eq!(
        no_shard
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert!(no_shard.sound_effects_after(serial).is_empty());

    let mut vanquished = test_state(open_grid(), 15, 9);
    vanquished.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    vanquished.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 0;
    let serial = vanquished.sound_effect_serial;
    assert_eq!(
        vanquished
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
            .unwrap(),
        MoveOutcome::Blocked
    );
    // Off any destruction row, so step 2's swell plays, then step 3's
    // cast-failure glissando (u5-spec#296); no flash does.
    assert_eq!(
        vanquished.sound_effects_after(serial),
        vec![SoundEffect::ShardSwell, SoundEffect::CastFailure]
    );

    // Correct shard, live Shadowlord, but no Eternal Flame underfoot. The
    // position gate's refusal follows `quest-graph.md §5` step 2's swell,
    // which is unconditional, and "the 50-update cast-failure glissando ...
    // plays after it" (step 3, u5-spec#296); what must not play is the flash.
    let mut no_flame = test_state(open_grid(), 1, 1);
    no_flame.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    no_flame.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
    let serial = no_flame.sound_effect_serial;
    assert_eq!(
        no_flame
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(
        no_flame.sound_effects_after(serial),
        vec![SoundEffect::ShardSwell, SoundEffect::CastFailure]
    );

    // On the flame, but the Shadowlord is not present to the north.
    let mut absent = test_state(open_grid(), 15, 9);
    absent.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 2,
    };
    absent.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    absent.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
    let serial = absent.sound_effect_serial;
    assert_eq!(
        absent
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, Some(&dir))
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(
        absent.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1,
        "a refusal must not consume the shard"
    );
    assert_eq!(
        absent.sound_effects_after(serial),
        vec![SoundEffect::ShardSwell],
        "step 2's swell is unconditional; the silent gates add nothing"
    );

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn shard_swell_matches_the_published_shard_use_row() {
    // `audio.md §5.4.7` (u5-spec#296, R523 relabelled the row): "Shard use
    // ... | 2640 | 200 | 184,000 | about 937 Hz | 7.91 s / 6.13 s". The stock
    // capture (`qa/paired/use-shard-sweep.tsv`, 891 Hz, 8.25-8.27 s) sits
    // inside the calibration band of those approximations. `quest-graph.md
    // §5` step 2 (R522): "one held pitch", not a pitch sweep.
    assert_eq!(audio::SHARD_SWELL_PERIOD, 2640);
    assert_eq!(audio::SHARD_SWELL_ITERATIONS, 200);
    let program = SoundEffect::ShardSwell.program(&mut crate::audio::RumbleJitter::default());
    let secs = program.duration(true).as_secs_f64();
    assert!((7.8..=8.0).contains(&secs), "swell lasts {secs} s");
    let first = audio::shrine_swell_segment(
        0,
        audio::SHARD_SWELL_PERIOD,
        audio::SHARD_SWELL_ITERATIONS,
    );
    let hz = first.gate_frequency_hz(true);
    assert!((925.0..=950.0).contains(&hz), "gate at {hz} Hz");
    // Every run shares the one period: the pitch never moves.
    let audio::SpeakerOp::Envelope(last) = program.ops[program.ops.len() - 2] else {
        panic!("the swell ends on an envelope run and a stop");
    };
    assert_eq!(last.period, first.period);
}

#[test]
fn off_position_shard_holds_no_effect_until_the_swell_ends() {
    // `quest-graph.md §5` steps 1-3: the aloft line prints at once, the
    // swell runs, and only then the position gate's `No effect!`. Measured
    // (`use-specials/sceptre`): three seconds after Return the stock window
    // still ends on the aloft line.
    let mut state = test_state(open_grid(), 1, 1);
    state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;

    assert_eq!(
        state
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, None)
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(
        state.message,
        "Thou dost hold above thee the evil Shard of Falsehood..."
    );
    assert!(state.staged_narration_active());

    let swell = SoundEffect::ShardSwell
        .program(&mut crate::audio::RumbleJitter::default())
        .duration(true)
        .as_secs_f64();
    // The hold is measured from the first advance.
    assert!(state.advance_staged_narration_to(100.0));
    assert!(state.advance_staged_narration_to(100.0 + 3.0));
    assert_eq!(
        state.staged_narration.beats().count(),
        1,
        "the refusal is still waiting three seconds in"
    );
    assert!(!state.advance_staged_narration_to(100.0 + swell + 0.01));
    assert_eq!(state.message, "\n\nNo effect!\n");
    // The position-gate refusal still consumes nothing.
    assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1);
}

#[test]
fn on_position_shard_holds_the_cast_line_until_the_swell_ends() {
    // `quest-graph.md §5` steps 1-4: the aloft line, then the one-pitch
    // swell "unconditionally" (R522) - before the position gate - and only
    // then, on the matching row, the cast-into-the-flame line. The engine
    // used to play the swell on the refusal branch alone and print the cast
    // line at once (audit A001).
    let mut state = test_state(open_grid(), 15, 9);
    state.area = Area::Town {
        scene: Scene::new(SCENE_THE_LYCAEUM).unwrap(),
        floor: 2,
    };
    state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] = 1;
    state.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = 1;
    let serial = state.sound_effect_serial;

    assert_eq!(
        state
            .use_shadowlord_shard(SHADOWLORD_FALSEHOOD_INDEX, None)
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::ShardSwell]
    );
    assert_eq!(
        state.message,
        "Thou dost hold above thee the evil Shard of Falsehood..."
    );
    let swell = SoundEffect::ShardSwell
        .program(&mut crate::audio::RumbleJitter::default())
        .duration(true)
        .as_secs_f64();
    assert!(state.advance_staged_narration_to(100.0));
    assert!(state.advance_staged_narration_to(100.0 + 3.0));
    assert_eq!(
        state.staged_narration.beats().count(),
        1,
        "the cast line is still waiting three seconds in"
    );
    assert!(!state.advance_staged_narration_to(100.0 + swell + 0.01));
    assert_eq!(state.message, "\n\n...and cast it into the Flame of Truth!\n");
    // The silent actor gate failed: the shard stays and no turn passes.
    assert_eq!(state.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX], 1);
    assert_eq!(state.turn, 0);
}

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
    assert!(state.message.starts_with("yellow potion:"));
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
    assert!(no_target.message.starts_with("Who?"));
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
fn shadowlord_destruction_runs_the_shared_major_flash() {
    // audio.md §8.4: Shadowlord destruction shares the turbulent full-viewport
    // flash - 1,856 bands, each one gameplay-PRNG draw, each an inclusive
    // 19..150 Hz frequency.
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

    let effects = state.sound_effects_after(serial);
    assert_eq!(effects.len(), 1, "one flash per destruction");
    match &effects[0] {
        SoundEffect::MajorFlash { bands } => {
            assert_eq!(bands.len(), audio::FLASH_BAND_COUNT as usize);
            assert!(bands.iter().all(|band| {
                (audio::FLASH_MIN_FREQUENCY_HZ as u8..=audio::FLASH_MAX_FREQUENCY_HZ as u8)
                    .contains(band)
            }));
        }
        other => panic!("audio.md §8.4 expects the shared major flash, got {other:?}"),
    }
    // The 1,856 band draws come out of the gameplay stream, not a sound-only
    // jitter state.
    assert_ne!(state.prng_state, prng_before);
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn shadowlord_shard_refusals_never_flash() {
    // audio.md §8.4 attaches the flash to destruction. Every gate above the
    // commit returns Blocked and must stay silent.
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
    assert!(vanquished.sound_effects_after(serial).is_empty());

    // Correct shard, live Shadowlord, but no Eternal Flame underfoot.
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
    assert!(no_flame.sound_effects_after(serial).is_empty());

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
    assert!(absent.sound_effects_after(serial).is_empty());

    let _ = fs::remove_dir_all(dir);
}

// `systems/audio.md §8.8` and `§8.9` trigger regressions.
//
// `cleak/u5-spec#150` asked which events produce the two effects the contract
// carried with no attributed trigger. Both are now published: the descending
// 220/150 Hz pair is a combat command refused as inapplicable (`§8.8`), and the
// long descent is the sea taking the party, by drowning or by whirlpool
// (`§8.9`). Each test below pins one row of those two tables, or one of the
// silences they draw around themselves.

/// `audio.md §8.8`: "`B` Board, `E` Enter, `F` Fire, `H` Hole up, `I` Ignite,
/// `L` Look, `M` Mix, `N` New order, `Q` Quit, `T` Talk, `V` View, `X` X-it."
const INAPPLICABLE_COMBAT_VERB_KEYS: [char; 12] =
    ['B', 'E', 'F', 'H', 'I', 'L', 'M', 'N', 'Q', 'T', 'V', 'X'];

#[test]
fn every_inapplicable_combat_verb_sounds_the_identical_pair() {
    // audio.md §8.8: "**The message tail varies; the sound does not.** ... All
    // three arms, and the out-of-range fall-through, play the identical
    // two-tone pair. One sound, one event class, twelve keys."
    let mut seen_tails = Vec::new();
    for key in INAPPLICABLE_COMBAT_VERB_KEYS {
        let mut state = combat_player_command_state(8, 5);
        let serial = state.sound_effect_serial;

        let applied = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(key))
            .unwrap();

        let CombatPlayerCommandAction::Branch {
            branch: CombatCommandBranch::SceneMessageAbort(verb),
            ..
        } = applied.action
        else {
            panic!("{key} should reach the shared scene-abort responder");
        };
        assert_eq!(combat_scene_abort_verb_key(verb), key);
        seen_tails.push(combat_scene_abort_tail(verb));

        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::CombatCommandRefused],
            "{key} plays the two-tone pair exactly once"
        );
    }

    // All three published tails are exercised above, and all three sounded.
    assert!(seen_tails.contains(&CombatSceneAbortTail::What));
    assert!(seen_tails.contains(&CombatSceneAbortTail::NotHere));
    assert!(seen_tails.contains(&CombatSceneAbortTail::FunnyNoResponse));
}

#[test]
fn the_three_refusal_tails_lower_to_one_program() {
    // §8.8 again: the tail selects only the printed line. `X-it` takes the
    // `" what?"` arm, `Look` the `"-Not here"` arm and `Talk` the
    // `"-Funny, no response!"` arm, and one sound answers all three.
    let mut jitter = audio::RumbleJitter::new();
    let program = SoundEffect::CombatCommandRefused.program(&mut jitter);
    assert_eq!(program.frequencies(), vec![220, 150]);

    for verb in [
        CombatSceneAbortVerb::Xit,
        CombatSceneAbortVerb::Look,
        CombatSceneAbortVerb::Talk,
    ] {
        assert!(audio::combat_command_refusal_sounds(
            combat_scene_abort_verb_key(verb)
        ));
    }
}

#[test]
fn the_d_and_w_combat_refusals_and_unrecognised_keys_stay_silent() {
    // audio.md §8.8: "**Do not generalise to the neighbouring keys.** `D` and
    // `W` print their own short `What?` line with **no sound**, and any
    // unrecognised key prints a bare `What?` with no sound. That silence is
    // real behaviour, not an omission here."
    for (key, expected) in [
        ('D', CombatCommandBranch::DWhatRefusal),
        ('W', CombatCommandBranch::WWhatRefusal),
        ('?', CombatCommandBranch::Invalid),
        ('/', CombatCommandBranch::Invalid),
    ] {
        let mut state = combat_player_command_state(8, 5);
        let serial = state.sound_effect_serial;

        let applied = state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key(key))
            .unwrap();

        assert_eq!(
            applied.action,
            CombatPlayerCommandAction::Branch {
                branch: expected,
                live_actor_gate: CombatCommandLiveActorGate::NotRequired,
            },
            "{key} should reach its own What? branch"
        );
        assert!(
            state.sound_effects_after(serial).is_empty(),
            "{key} must stay silent"
        );
    }
}

#[test]
fn an_inapplicable_combat_command_costs_no_turn() {
    // audio.md §8.8, turn cost column: "**None.** The same combatant is
    // re-prompted and the committed-action bookkeeping is skipped, so the
    // refusal does not spend the actor's turn."
    let mut state = combat_player_command_state(8, 5);
    let hp_before = state.party[0].hp;
    let effect_counter_before = state.active_effect_counter;
    let serial = state.sound_effect_serial;

    let applied = state
        .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Key('L'))
        .unwrap();

    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::CombatCommandRefused]
    );
    assert!(applied.reprompt, "the same combatant is re-prompted");
    assert_eq!(applied.control_after, CombatRoundLoopControl::ContinueActorWalk);
    // The committed-action tail is what carries maintenance; a free refusal
    // bypasses it entirely.
    assert!(applied.absorbable_contact.is_none());
    assert!(applied.post_dispatch_contact.is_none());
    assert!(applied.active_effect_age.is_none());
    assert_eq!(state.active_effect_counter, effect_counter_before);
    assert_eq!(state.party[0].hp, hp_before);
}

#[test]
fn the_blocked_step_beep_is_not_the_refusal_pair() {
    // audio.md §7.4: "**The two-tone 220/150 Hz pair is no longer
    // unidentified** ... It is still **not** the blocked-step recipe, and must
    // not be conflated with it."
    let mut jitter = audio::RumbleJitter::new();
    let beep = SoundEffect::BlockedStep.program(&mut jitter);
    let refusal = SoundEffect::CombatCommandRefused.program(&mut jitter);
    assert_eq!(beep.frequencies(), vec![audio::BLOCKED_STEP_HZ]);
    assert_eq!(refusal.frequencies(), vec![220, 150]);
    assert_ne!(beep, refusal);
}

// ---------------------------------------------------------------------------
// audio.md §8.9 - the sea takes the party
// ---------------------------------------------------------------------------

/// The `0xEC..=0xEF` whirlpool family; `tile_classes.rs` owns the range.
const WHIRLPOOL_OBJECT_TYPE: u8 = 0xEC;

fn whirlpool_world_state(object_x: usize, object_y: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.active_objects[0].z = WorldPlane::Britannia.save_floor();
    state.active_objects.push(ActiveObject {
        type_byte: WHIRLPOOL_OBJECT_TYPE,
        tile: WHIRLPOOL_OBJECT_TYPE,
        x: object_x,
        y: object_y,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    });
    state
}

fn aboard_intact_frigate(state: &mut PlayState) {
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 100,
        skiffs: 2,
    };
    state.sync_player_object();
}

#[test]
fn an_orthogonal_whirlpool_aboard_a_vehicle_sounds_the_long_descent() {
    // audio.md §8.9, whirlpool row: "The party, **in any vehicle**, moves
    // orthogonally adjacent to a whirlpool active object." The sequence is
    // "the whirlpool object is cleared, `WHIRLPOOL!` prints ... **then** the
    // long descent - then the sprite is restored, the shared impact payload
    // runs, and the party is teleported".
    let dir = debug_game_dir();
    // (5, 4) is directly north of the party at (5, 5).
    let mut state = whirlpool_world_state(5, 4);
    aboard_intact_frigate(&mut state);
    let serial = state.sound_effect_serial;

    let outcome = state
        .apply_world_whirlpool_engagement(&dir, WorldPlane::Britannia)
        .expect("whirlpool engagement should not error");

    let effects = state.sound_effects_after(serial);
    assert_eq!(
        effects.first(),
        Some(&SoundEffect::LongDescent),
        "the sweep leads the payload"
    );
    assert_eq!(
        effects
            .iter()
            .filter(|effect| **effect == SoundEffect::LongDescent)
            .count(),
        1,
        "one sweep per engagement"
    );
    // "The state commit - the teleport - happens strictly after it."
    assert_eq!(
        outcome,
        Some(MoveOutcome::Transition(
            AreaTransition::ChangedWorldPlane {
                from: WorldPlane::Britannia,
                to: WorldPlane::Underworld,
            },
        ))
    );
    assert_eq!((state.player.x, state.player.y), (34, 18));
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_diagonal_whirlpool_approach_plays_nothing() {
    // audio.md §8.9: "Diagonal adjacency does not trigger it."
    let dir = debug_game_dir();
    // (4, 4) is diagonally adjacent to the party at (5, 5).
    let mut state = whirlpool_world_state(4, 4);
    aboard_intact_frigate(&mut state);
    let serial = state.sound_effect_serial;

    let outcome = state
        .apply_world_whirlpool_engagement(&dir, WorldPlane::Britannia)
        .expect("whirlpool engagement should not error");

    assert_eq!(outcome, None, "a diagonal whirlpool is not engaged");
    assert!(state.sound_effects_after(serial).is_empty());
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia,
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn a_whirlpool_engaged_on_foot_plays_no_long_descent() {
    // audio.md §8.9: "**On foot there is no long descent.** Both the
    // whirlpool's plane change and this sound require a vehicle." §9 lists
    // "whirlpool engagement while the party is on foot, which plays no long
    // descent" as an explicit silence boundary. The on-foot arm still reaches
    // the shared impact payload, which has its own damage rumbles - so this
    // asserts the absence of the sweep, not of all sound.
    let dir = debug_game_dir();
    let mut state = whirlpool_world_state(5, 4);
    state.party = six_member_party(40);
    // world_state leaves the party on foot.
    assert!(state.player.transport.is_foot());
    let serial = state.sound_effect_serial;

    let outcome = state
        .apply_world_whirlpool_engagement(&dir, WorldPlane::Britannia)
        .expect("whirlpool engagement should not error");

    assert_eq!(outcome, Some(MoveOutcome::Used));
    assert!(
        !state
            .sound_effects_after(serial)
            .contains(&SoundEffect::LongDescent),
        "the on-foot arm is silent of the sweep"
    );
    // The plane change is skipped too, which is why the sound is.
    assert_eq!(
        state.area,
        Area::World {
            plane: WorldPlane::Britannia,
        }
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn drowning_sounds_the_long_descent_before_the_death_loop() {
    // audio.md §8.9, drowning row: "`Ship sunk!` prints, the party sprite is
    // cleared, the stats panel refreshes, and the viewport is rebuilt so the
    // empty ocean is on screen - **then** the long descent - then
    // `DROWNING!!!`, then the death loop."
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party = six_member_party(4);
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 0;
    // Hull one with no skiffs: destroyed on every roll, and the ladder reaches
    // its last rung.
    aboard_frigate(&mut state, 1, 0);
    let serial = state.sound_effect_serial;

    let OutdoorImpactAbsorption::ShipDestroyed {
        fallback, drowning, ..
    } = state.apply_outdoor_impact()
    else {
        panic!("hull one is always destroyed");
    };
    assert_eq!(fallback, ShipLossFallback::Drown);
    assert!(!drowning.is_empty());

    let effects = state.sound_effects_after(serial);
    assert_eq!(
        effects.first(),
        Some(&SoundEffect::LongDescent),
        "the sweep leads the death loop"
    );
    assert_eq!(
        effects
            .iter()
            .filter(|effect| **effect == SoundEffect::LongDescent)
            .count(),
        1,
        "the sweep plays once, not once per loop pass"
    );
    // "the loop alternates the damage presentation, which plays its own rumble
    // on every pass, with unavoidable damage to every living member".
    assert!(effects[1..].iter().all(|effect| *effect
        == SoundEffect::DamageRumble));
    assert_eq!(state.player.transport, TransportState::SpriteSuppressed);
    let lines = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>();
    assert!(lines.ends_with(&[SHIP_SUNK_MESSAGE, DROWNING_MESSAGE]));
}

#[test]
fn a_skiff_aboard_suppresses_the_drowning_long_descent() {
    // audio.md §8.9: "With a skiff or a carpet available the game prints
    // `Abandon ship!`, substitutes the vehicle, and plays **no** long sound."
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party = six_member_party(40);
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 0;
    aboard_frigate(&mut state, 1, 2);
    let serial = state.sound_effect_serial;

    let OutdoorImpactAbsorption::ShipDestroyed { fallback, .. } = state.apply_outdoor_impact()
    else {
        panic!("hull one is always destroyed");
    };

    assert_eq!(fallback, ShipLossFallback::Skiff);
    assert!(state.sound_effects_after(serial).is_empty());
    let lines = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>();
    assert!(lines.ends_with(&[SHIP_SUNK_MESSAGE, ABANDON_SHIP_MESSAGE]));
}

#[test]
fn a_carried_carpet_suppresses_the_drowning_long_descent() {
    // The second half of the same clause: the carpet rung is silent too.
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party = six_member_party(40);
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 2;
    aboard_frigate(&mut state, 1, 0);
    let serial = state.sound_effect_serial;

    let OutdoorImpactAbsorption::ShipDestroyed { fallback, .. } = state.apply_outdoor_impact()
    else {
        panic!("hull one is always destroyed");
    };

    assert_eq!(fallback, ShipLossFallback::Carpet);
    assert!(state.sound_effects_after(serial).is_empty());
    let lines = state
        .message_entries()
        .iter()
        .map(|entry| entry.text.as_str())
        .collect::<Vec<_>>();
    assert!(lines.ends_with(&[SHIP_SUNK_MESSAGE, ABANDON_SHIP_MESSAGE]));
}

#[test]
fn the_long_descent_has_exactly_the_two_published_trigger_sites() {
    // audio.md §8.9: "Two overworld events share one sound ... Those two sites
    // are the only users of that recipe in the shipped game." This is the
    // cheap structural guard: the engine emits `LongDescent` from exactly two
    // places, the whirlpool engagement and the drowning rung.
    let sources = include_str!("../play_state_impl/chunk_07.rs").matches("SoundEffect::LongDescent")
        .count()
        + include_str!("../play_state_impl/chunk_09.rs")
            .matches("SoundEffect::LongDescent")
            .count();
    assert_eq!(sources, 2, "§8.9 attributes the recipe to two sites only");
}

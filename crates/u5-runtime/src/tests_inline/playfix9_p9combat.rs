// Regressions for the pass-9 exploratory-playtest findings (p9combat).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `audio.md §6.1` "Cast rejection sounds": "a spell rejected by the
/// castability mask prints `Not here!\n` and plays the 50-update
/// cast-failure glissando before any resource debit, with no additional
/// Failed line" (`magic.md §7` result table). The dispatcher's scene gate
/// printed the line and played nothing (Q17).
#[test]
fn playtest_q17_scene_gate_not_here_plays_the_failure_glissando() {
    let mut state = test_state(open_grid(), 5, 5);
    state.spell_charges[UUS_POR_SPELL_INDEX] = 1;
    state.party[0].mana = 4;
    state.party[0].level = 4;
    let serial = state.sound_effect_serial;

    handle_play_key_input(&mut state, 'C', "1PU", Path::new("")).unwrap();

    assert_eq!(state.message, "Not here!");
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::CastFailure]
    );
    assert_eq!(state.spell_charges[UUS_POR_SPELL_INDEX], 1, "nothing spent");
    assert_eq!(state.party[0].mana, 4);
}

/// `combat.md §8.2`: a member with no qualifying item "makes a single
/// bare-handed attempt, which behaves as melee with range one", and the
/// `§11.1` census row "Swing begins | party melee" plays the rising sweep
/// "unconditionally, before the roll" (`audio.md §7.4`). Bare-handed attacks
/// were silent; only an armed melee attack swung (Q18).
#[test]
fn playtest_q18_bare_handed_attack_plays_the_party_melee_swing() {
    for forced_hit in [true, false] {
        let mut state = combat_ai_turn_state(6, 5);
        let serial = state.sound_effect_serial;
        let application = state
            .resolve_and_apply_combat_bare_handed_attack(0, 8, 30, 10, 0, Some(forced_hit))
            .expect("an adjacent bare-handed attempt resolves");
        assert!(!matches!(
            application.resolution,
            CombatWeaponAttackResolution::OutOfRange { .. }
        ));
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::PartyMeleeAttackSwing),
            "the swing precedes the to-hit branch (forced_hit {forced_hit})"
        );
    }

    // Out of reach the attempt reaches no attack application and stays
    // silent, exactly like the equipment path's `OutOfRange` arm.
    let mut far = combat_ai_turn_state(9, 5);
    let serial = far.sound_effect_serial;
    let application = far
        .resolve_and_apply_combat_bare_handed_attack(0, 8, 30, 10, 0, Some(true))
        .expect("the attempt resolves");
    assert!(matches!(
        application.resolution,
        CombatWeaponAttackResolution::OutOfRange { .. }
    ));
    assert!(far.sound_effects_after(serial).is_empty());
}

/// `combat.md §7` restarts the table walk at slot 32 while actors remain, and
/// `§8.1`'s dragged-under arm ends each held turn "with no command read", so
/// the walk carries on until a keyboard-driven actor actually needs a key.
/// The driver's non-termination guard counted every table walk, so a victim
/// held for a few turns exhausted it and the walk stopped with no prompt
/// open - the round's remaining output (a last foe's `escapes!` and
/// `VICTORY!`) then printed only after the next key (Q02).
#[test]
fn playtest_q02_dragged_under_turns_do_not_stall_the_round_walk() {
    let mut longest_hold = 0;
    for seed in 0..64u16 {
        let mut state = worked_bat_arena(&[], 0);
        state.prng_state = seed.wrapping_mul(2_654).wrapping_add(1);
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_DRAGGED_UNDER;
        state.active_objects[0].tile = 0;

        open_owed_combat_player_turn(&mut state);

        let held_turns = state
            .message_entries()
            .iter()
            .filter(|entry| entry.text.starts_with(COMBAT_DRAGGED_UNDER_TURN_LINE))
            .count();
        longest_hold = longest_hold.max(held_turns);
        assert!(
            !state.combat_actors[0].is_dragged_under(),
            "seed {seed}: the walk stopped while the victim was still held \
             after {held_turns} ARGH! turns"
        );
        assert_eq!(
            state.pending_combat_actor_slot,
            Some(0),
            "seed {seed}: the released victim's prompt is open"
        );
    }
    assert!(
        longest_hold >= 3,
        "some seed must hold the victim past the old thirty-seven-walk guard"
    );
}

/// `audio.md §8.3`: "For combat cursor spells, confirmation plays the spell
/// effect before the coordinate/projectile-impact resolver", and `§6.1`'s
/// combat effect template makes that effect the circle-scaled rumble lead;
/// only the descending glissando depends on a resolved effect. `magic.md §8`
/// removes only the text for an empty shot: "a shot with no resolved actor
/// and no original aimed actor has no result line". A Magic Missile
/// confirmed on an empty cell ended the turn in silence (Q19). Escape is a
/// cancel, not a confirm, and stays silent.
#[test]
fn playtest_q19_aimed_spell_confirmed_on_an_empty_cell_still_plays_its_effect() {
    let spell_index = spell_index_from_code("GP").unwrap();
    let open_cursor = || {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.party[0].mana = 1;
        state.party[0].level = 1;
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            12,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            5,
        ]);
        state.start_combat_cast_spell_prompt(0, false);
        state.step_active_cast('G', "P", Path::new("")).unwrap();
        state.step_active_cast(' ', "", Path::new("")).unwrap();
        assert!(matches!(
            state.active_cast_followup.as_ref().map(|s| s.kind),
            Some(CastFollowupKind::CombatAimCursor { .. })
        ));
        state
    };

    let mut confirmed = open_cursor();
    confirmed
        .step_active_cast_followup(char::from(INPUT_CODE_EAST), "", Path::new(""))
        .unwrap();
    let serial = confirmed.sound_effect_serial;
    assert!(
        confirmed
            .step_active_cast_followup('\r', "", Path::new(""))
            .unwrap()
            .is_none()
    );
    assert!(confirmed.active_cast_followup.is_none());
    assert_eq!(
        confirmed.sound_effects_after(serial),
        vec![SoundEffect::CircleRumbleLead { circle: 1 }]
    );
    assert!(confirmed.message.is_empty(), "no result line");

    let mut cancelled = open_cursor();
    let serial = cancelled.sound_effect_serial;
    cancelled
        .step_active_cast_followup('\u{1b}', "", Path::new(""))
        .unwrap();
    assert!(cancelled.active_cast_followup.is_none());
    assert!(cancelled.sound_effects_after(serial).is_empty());
}

/// `moons.md §2.2`/`§3` and `weather.md §2.1` keep combat outside the strip
/// and banner renderers, so nothing repaints either ribbon gap during a fight
/// - the arena used to follow the live clock, wind and facing (Q10). Q10 then
/// blanked both gaps, as `moons.md §3`'s "plain chrome" words it, but every
/// stock arena capture keeps what the scene drew before the fight: a town or
/// surface arena shows the strip and the wind banner (`combat-town-attack`,
/// `combat-escape-ordinary`, `overworld-night-walk`), a dungeon room shows
/// `L1` and `Dir: North` (`combat-dungeon-room`). The captures win: the gaps
/// are frozen at combat entry.
#[test]
fn playtest_q10_combat_arena_keeps_the_pre_combat_ribbon_gaps_frozen() {
    fn enter(state: &mut PlayState) {
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            4,
            4,
        ]);
        state
            .enter_combat_frame(vec![ActiveObject::empty(); OOL_SLOTS], actors)
            .expect("combat frame should enter");
    }

    let mut town = test_state(open_grid(), 1, 1);
    town.wind = WindState::South;
    town.wind_save_byte = WindState::South.save_byte();
    let before = gameplay_chrome_content(&town);
    assert!(matches!(before.top, ChromeGap::SkyStrip(_)));
    assert!(matches!(before.bottom, ChromeGap::Label(_)));
    enter(&mut town);
    assert!(town.combat_active);
    // A wind shift or an hour rolling over mid-fight repaints nothing.
    town.wind = WindState::North;
    town.wind_save_byte = WindState::North.save_byte();
    town.clock.hour = (town.clock.hour + 6) % 24;
    assert_ne!(
        gameplay_chrome_content(&{
            let mut live = town.clone();
            live.combat_active = false;
            live
        }),
        before,
        "the live scene chrome did move"
    );
    let arena = gameplay_chrome_content(&town);
    assert_eq!((arena.top, arena.bottom), (before.top, before.bottom));

    let mut surface = world_state(open_world_grid(), 10, 20);
    let before = gameplay_chrome_content(&surface);
    enter(&mut surface);
    let arena = gameplay_chrome_content(&surface);
    assert_eq!((arena.top, arena.bottom), (before.top, before.bottom));

    let mut room = dungeon_state(open_dungeon_record(), 0, 1, 1);
    let before = gameplay_chrome_content(&room);
    assert!(matches!(before.top, ChromeGap::Label(_)));
    enter(&mut room);
    room.player.facing = match room.player.facing {
        Direction::East => Direction::West,
        _ => Direction::East,
    };
    let arena = gameplay_chrome_content(&room);
    assert_eq!((arena.top, arena.bottom), (before.top, before.bottom));
}

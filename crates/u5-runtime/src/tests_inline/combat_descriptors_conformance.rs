    // `systems/combat.md` §6.1 / §6.1a / §6.3 / §14 / §16.1 conformance
    // regressions for the combat descriptor bits, the death paths, the
    // faction tags, the shared action-result field, and the victory /
    // defeat / escape transitions.

    fn combat_descriptor_transcript(state: &PlayState) -> String {
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>()
            .join("
")
    }

    fn combat_descriptor_state() -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state
            .active_objects
            .resize(COMBAT_ACTOR_SLOTS, ActiveObject::empty());
        state
    }

    #[test]
    fn party_side_bit_is_the_damage_resolver_branch_discriminator() {
        // `combat.md §6.1`, bit `0x80`: "It is the discriminator the
        // damage/death resolver uses to choose the party-death branch over
        // the monster-death branch, so an engine that also sets it for live
        // monsters routes every monster death through the party path." The
        // engine keyed the choice on the raw slot index instead.
        let mut party_side = combat_descriptor_state();
        party_side.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        assert!(matches!(
            party_side.apply_combat_weapon_damage_to_target(None, 0, 3, false),
            Some(CombatWeaponDamageApplication::Party { .. })
        ));

        // A descriptor in the same slot that does not carry `0x80` is not a
        // party-death target, whatever its index.
        let mut monster_side = combat_descriptor_state();
        let stats = combat_class_stats(32).unwrap();
        monster_side.combat_actors[0] = CombatActorDescriptor::for_monster_placement(
            stats,
            0,
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            0,
        );
        assert!(matches!(
            monster_side.apply_combat_weapon_damage_to_target(None, 0, 3, false),
            Some(CombatWeaponDamageApplication::Monster { .. })
        ));
    }

    #[test]
    fn monster_death_overwrites_the_whole_flags_byte() {
        // `combat.md §6.1`, bit `0x20`: "Monster death overwrites the whole
        // flags byte with this value; party death ORs it in." `§6.3` repeats
        // it: "all other per-round flag state on that descriptor is lost."
        let stats = combat_class_stats(32).unwrap();
        let mut monster = CombatActorDescriptor::for_monster_placement(
            stats,
            7,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_40
                | COMBAT_ACTOR_FLAG_FLEEING
                | COMBAT_ACTOR_FLAG_CONTROLLED,
            0,
        );
        let outcome = monster
            .apply_monster_damage(COMBAT_INSTANT_KILL_DAMAGE, true)
            .unwrap();
        assert!(outcome.killed);
        assert_eq!(monster.flags, COMBAT_ACTOR_FLAG_MARKED_DEAD);
    }

    #[test]
    fn party_death_ors_the_marked_dead_bit_into_the_descriptor() {
        // `combat.md §6.3` party-member row: "marked-dead bit ORed in" is one
        // of the death's own writes, not a later sweep's. `§6.1` keeps the
        // party form an OR, so `0x80` survives alongside `0x20`.
        let mut state = combat_descriptor_state();
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        state.party[0].hp = 5;
        state.active_player = Some(0);
        state.visibility_dirty = false;

        state
            .apply_combat_weapon_damage_to_target(None, 0, COMBAT_INSTANT_KILL_DAMAGE, false)
            .unwrap();

        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.party[0].hp, 0);
        assert_eq!(
            state.combat_actors[0].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD
        );
        // The slot itself is not released.
        assert!(!state.combat_actors[0].is_free_for_allocation());
        assert_eq!(state.active_player, None);
        // `§11.1`, "Target dies": "no cue of its own; the party death arm
        // runs a full stats redraw".
        assert!(state.visibility_dirty);
    }

    #[test]
    fn blink_phase_uses_the_phase_filter_bit_not_ordinary_invisibility() {
        // `combat.md §6.1`: `0x10` is the phase/blink filter and `0x04` is
        // "Hidden / not-yet-revealed (invisible)". `§9` keeps them apart:
        // only the phase filter has a bypass.
        let mut actor = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            23,
            1,
            0,
            4,
            5,
        ]);
        let mut active_objects = vec![ActiveObject::empty(); 2];
        active_objects[1] = ActiveObject {
            type_byte: 0x9c,
            tile: 0x9c,
            ..ActiveObject::empty()
        };

        toggle_combat_blink_phase(&mut actor, &mut active_objects).unwrap();

        assert_eq!(
            actor.flags & COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER,
            COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER
        );
        assert!(actor.is_phase_suppressed());
        assert!(!actor.is_hidden_or_unrevealed());
    }

    #[test]
    fn the_doom_suppression_bypass_does_not_reach_ordinary_invisibility() {
        // `combat.md §9`: after the bypassable phase/hidden test, "the
        // 'invisible / not-yet-revealed' flag is still rejected after the
        // phase/hidden check. This ordinary invisibility filter is not the
        // same as the special suppression-filter bypass above." The engine
        // fed `0x04` into the bypassable test, so a Doom-scene monster
        // happily targeted an invisible party member.
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
        // A second, plainly visible party member so the assertion below
        // pins that the picker *chose the visible candidate*, not merely
        // that it failed to choose anybody.
        state.combat_actors[1] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1, 1, 0, 7, 5]);
        state.active_objects[1] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 7,
            y: 5,
            ..ActiveObject::empty()
        };
        state.combat_frame_snapshot = Some(CombatFrameSnapshot {
            area: Area::Dungeon {
                scene: DungeonScene {
                    byte: DUNGEON_DOOM_SCENE_BYTE,
                    record: DOOM_DUNGEON_RECORD,
                },
                level: 0,
            },
            player: state.player,
            active_objects: state.active_objects.clone(),
            active_player: state.active_player,
            combat_terrain: state.combat_terrain,
            dungeon_room_clear_on_success: None,
            enter_endgame_after_successful_combat: false,
            endgame_messages: None,
            endgame_tableau_map: None,
            encounter_mode_high_bit: false,
            suppress_controlled_faint_sleep_tick: false,
            exit_announced: false,
            established_exit_direction_code: None,
        });

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                1,
                1,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[4, 1, 3, 2],
                None,
            )
            .unwrap();

        assert_eq!(
            application.target,
            CombatAiTargetResolution::ChosenActor {
                slot: 1,
                x: 7,
                y: 5,
            },
            "ordinary invisibility is rejected even in the Doom bypass context,              so the visible party member is the chosen target"
        );
    }

    #[test]
    fn summoned_creatures_are_monster_side_and_controlled() {
        // `combat.md §6.1a` writer 3 + `§6.1`: Conjure/Swarm/Summon "are
        // still placed through the ordinary monster placement path, so their
        // class byte is the monster-side one", and "Monster and object
        // descriptors never carry" `0x80`.
        assert_eq!(
            combat_summoned_actor_flags(COMBAT_CLASS_GIANT_SPIDER),
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_CONTROLLED
        );

        let mut state = combat_descriptor_state();
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        let application = state
            .apply_combat_summon_class_around_slot(COMBAT_CLASS_GIANT_SPIDER, 0, 10)
            .unwrap();
        let summoned = state.combat_actors[application.actor_slot];
        assert_eq!(
            summoned.flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_CONTROLLED
        );
        assert_eq!(summoned.flags & COMBAT_ACTOR_FLAG_SELECTABLE_80, 0);
    }

    #[test]
    fn gazer_death_spawn_carries_the_hostile_tag_and_no_control_bit() {
        // `combat.md §6.3`: the Gazer's Insect Swarm is seeded "exactly as
        // any other monster placement would (§ 5): ... the hostile faction
        // tag, class id `31`".
        let mut state = combat_descriptor_state();
        let gazer_slot = COMBAT_PARTY_ACTOR_SLOTS;
        place_death_side_effect_monster(&mut state, 28, gazer_slot, 12);

        state
            .apply_combat_weapon_damage_to_target(
                None,
                gazer_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();

        let swarm_slot = (COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS)
            .find(|slot| {
                state.combat_actors[*slot].owner_target_class == COMBAT_CLASS_INSECT_SWARM
                    && !state.combat_actors[*slot].is_empty()
            })
            .expect("the Gazer death places a live Insect Swarm");
        assert_eq!(
            state.combat_actors[swarm_slot].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_40
        );
        assert!(!state.combat_actors[swarm_slot].is_controlled());
    }

    #[test]
    fn a_fled_actor_release_preserves_the_two_trailing_auxiliary_bytes() {
        // `combat.md §6.3`: the negative-form release "used by a vanishing
        // monster or fled actor ... zeros linked active-object bytes 0
        // through 5 while preserving that record's two trailing auxiliary
        // bytes. ... an all-zero record is not required."
        let mut state = combat_player_command_state(8, 5);
        state.combat_actors[0].x = 10;
        state.active_objects[0].x = 10;
        state.active_objects[0].phase = 0x0a;
        state.active_objects[0].aux1 = 0x33;
        state.active_objects[0].aux3 = 0x5a;

        state
            .apply_combat_player_command_with_inputs(0, CombatPlayerCommandInput::Direction(2))
            .unwrap();

        let record = state.active_objects[0];
        assert_eq!(record.type_byte, 0);
        assert_eq!(record.tile, 0);
        assert_eq!(record.x, 0);
        assert_eq!(record.y, 0);
        assert_eq!(record.z, 0);
        assert_eq!(record.aux1, 0);
        assert_eq!(record.phase, 0x0a);
        assert_eq!(record.aux3, 0x5a);
    }

    #[test]
    fn side_counting_runs_through_the_group_resolver() {
        // `combat.md §16.1`: "Side counting skips empty, dead, and passive
        // descriptors, then uses the same group resolver. Group 1 counts as
        // foes and group 0 as friends. Control and the traitor identity
        // therefore affect victory detection."
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        actors[6] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_CONTROLLED,
            COMBAT_CLASS_GIANT_RAT,
            6,
            0,
            4,
            4,
        ]);

        // A controlled monster resolves to group 0, so it is a friend and
        // stops holding the hostile count above zero.
        let census = combat_side_census(&actors);
        assert_eq!(census.foes, 0);
        assert_eq!(census.friends, 2);
        assert!(resolve_combat_victory(&actors));

        // The shipped traitor roster identity resolves a party descriptor to
        // group 1, so it counts as a foe and suppresses the announcement.
        actors[6] = CombatActorDescriptor::empty();
        actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            TRAITOR_ROSTER_RECORD,
            1,
            0,
            6,
            5,
        ]);
        let census = combat_side_census(&actors);
        assert_eq!(census.friends, 1);
        assert_eq!(census.foes, 1);
        assert!(!resolve_combat_victory(&actors));
    }

    #[test]
    fn a_high_bit_mode_escape_is_refused_after_the_exit_announcement() {
        // `combat.md §14`: branch 1 - "such a party-side descriptor exists
        // and the encounter mode's high bit is set" - is unconditional on
        // the announcement, and branch 3's accept arm is scoped to "the
        // ordinary-mode exit announcement". The engine tested the
        // announcement first, so a high-bit Escape after the announcement
        // wrongly accepted.
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

        assert_eq!(
            resolve_combat_escape_cleanup(&actors, true, true),
            CombatEscapeCleanupDecision::RefusedNotHere
        );
        assert_eq!(
            resolve_combat_escape_cleanup(&actors, true, false),
            CombatEscapeCleanupDecision::RefusedNotHere
        );
        assert_eq!(
            resolve_combat_escape_cleanup(&actors, false, false),
            CombatEscapeCleanupDecision::RefusedNotYet
        );
        assert_eq!(
            resolve_combat_escape_cleanup(&actors, false, true),
            CombatEscapeCleanupDecision::Accepted
        );
        // No qualifying party-side descriptor accepts in either mode.
        let empty = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        assert_eq!(
            resolve_combat_escape_cleanup(&empty, true, false),
            CombatEscapeCleanupDecision::Accepted
        );
    }

    #[test]
    fn the_defeat_recount_offers_the_control_faint_helper_a_restore_first() {
        // `combat.md §14` Defeat: "the engine first runs the party
        // control/faint helper. If it cannot restore an actor, the engine
        // prints `BATTLE IS LOST!`". `§7`'s post-dispatch recount is the
        // same transition. The engine returned the defeat exit straight from
        // the side census, so the helper had exactly one caller - the vanish
        // death arm.
        let mut state = combat_ai_turn_state(8, 5);
        // `§16.1`: the controlled bit moves this party descriptor to group 1,
        // so the census sees no friend and one extra foe.
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;

        let control = state.combat_round_loop_control(false, false);

        assert_eq!(control, CombatRoundLoopControl::ContinueActorWalk);
        assert!(
            state.message.contains("passes out!"),
            "message was {:?}",
            state.message
        );
        assert_eq!(
            state.combat_actors[0].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_STATUS_DISABLED
        );
        assert_eq!(state.party[0].status, b'S');
    }

    #[test]
    fn the_defeat_recount_concedes_when_the_helper_restores_nobody() {
        // The other arm of the same branch: with no controlled party-side
        // descriptor the helper returns its no-match sentinel and the round
        // loop takes the defeat exit.
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0] = CombatActorDescriptor::empty();

        assert_eq!(
            state.combat_round_loop_control(false, false),
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
        );
        assert!(!state.message.contains("passes out!"));
    }

    #[test]
    fn a_vanish_death_suppresses_the_generic_killed_line() {
        // `combat.md §6.3`: the shared action-result field's "only relevant
        // bit reader" is the common attack result narrator. "It first clears
        // the field's kill-narrated bit `0x01`; when `0x02` is still present,
        // the combined suppression test skips the generic killed/slept/hit
        // chain, produces no message or sound, and clears `0x02` in cleanup."
        // Nothing in the engine read the field at all, so a vanish death
        // printed both `<name> vanishes!` and `<name> killed!`.
        let game_dir = std::path::Path::new(".");
        // Class 47 (Shadow Lord) is a vanish-on-death class that still
        // takes physical damage, so this attack's own damage resolution
        // takes the vanish branch and stores `0x02` before the narrator
        // builds its line.
        let mut state = combat_player_command_state(6, 5);
        state.combat_actors[8].owner_target_class = 47;
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = 16;
        state.party_strengths = vec![255];
        state.party_experience = vec![0];
        state.combat_actors[8].hp_or_wound = 1;
        // `combat.md §5.1` / `§11`: the shared fixture seats a base-step-1
        // actor, which after `RETRACTIONS.md` R334 is a Dexterity-1
        // character who essentially never lands a blow. This test needs the
        // blow to land, so it seats a fast one.
        state.combat_actors[0].base_step = 30;

        // `combat.md §8.2`: `A` opens a targeting cursor, and the attempt
        // resolves on the confirm, not on the direction key.
        handle_play_key_input(
            &mut state,
            'A',
            &format!("{}\r", char::from(INPUT_CODE_EAST)),
            game_dir,
        )
        .unwrap();

        let transcript = combat_descriptor_transcript(&state);
        // The branch-specific line is still printed.
        assert!(
            transcript.contains("Shadow Lord vanishes!"),
            "transcript was {transcript:?}"
        );
        assert!(
            !state.message.contains("killed!"),
            "message was {:?}",
            state.message
        );
        assert!(
            !transcript.contains("killed!"),
            "transcript was {transcript:?}"
        );
        // The kill itself still happened; only its narration was suppressed.
        assert!(state.combat_actors[8].is_free_for_allocation());
        // "clears `0x02` in cleanup"
        assert_eq!(
            state.combat_action_result & COMBAT_ACTION_RESULT_VANISH_NARRATED,
            0
        );
    }

    #[test]
    fn the_faint_sleep_overwrite_lets_the_generic_kill_line_through() {
        // `combat.md §6.3`, the ordering collision: when the vanish branch's
        // control/faint scan finds a party member and sleep succeeds, "the
        // sleep helper replaces the whole result field with sleep bit
        // `0x04`, losing `0x02`", so the later common narrator "appends the
        // vanished target's `<name> killed!` line - not `<name> slept!` -
        // after the faint tail and plays no additional sound. It sets
        // kill-narrated bit `0x01`, clears the transient sleep bit". Neither
        // the kill-narrated bit nor the collision existed in the engine.
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(6, 5);
        state.combat_actors[8].owner_target_class = 47;
        state.party_equipment = default_party_equipment(1);
        state.party_equipment[0][EQUIP_SLOT_WEAPON] = 16;
        state.party_strengths = vec![255];
        state.party_experience = vec![0];
        state.combat_actors[8].hp_or_wound = 1;
        // A controlled party-side descriptor for the same roster member, so
        // the vanish tail's faint scan matches and its sleep helper
        // overwrites the result field before the narrator reads it.
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED,
            0,
            1,
            0,
            5,
            8,
        ]);
        // `combat.md §5.1` / `§11`: the shared fixture seats a base-step-1
        // actor, which after `RETRACTIONS.md` R334 is a Dexterity-1
        // character who essentially never lands a blow. This test needs the
        // blow to land, so it seats a fast one.
        state.combat_actors[0].base_step = 30;

        // `combat.md §8.2`: `A` opens a targeting cursor, and the attempt
        // resolves on the confirm, not on the direction key.
        handle_play_key_input(
            &mut state,
            'A',
            &format!("{}\r", char::from(INPUT_CODE_EAST)),
            game_dir,
        )
        .unwrap();

        let transcript = combat_descriptor_transcript(&state);
        assert!(
            transcript.contains("Shadow Lord vanishes!"),
            "transcript was {transcript:?}"
        );
        assert!(
            transcript.contains("passes out!"),
            "transcript was {transcript:?}"
        );
        // The sleep overwrite lost `0x02`, so the generic chain runs after
        // the faint tail and appends the kill line rather than `slept!`.
        assert!(
            transcript.contains("Shadow Lord killed!"),
            "transcript was {transcript:?}"
        );
        assert!(!transcript.contains("slept!"), "transcript was {transcript:?}");
    }

    #[test]
    fn the_narrator_gate_matches_the_published_result_field_transitions() {
        // `combat.md §6.3` in full: clear `0x01` first; suppress on a
        // surviving `0x02` and clear it in cleanup; otherwise narrate, and on
        // the kill arm set `0x01` and clear the sleep bit.
        let suppressed = resolve_combat_attack_narrator_gate(
            COMBAT_ACTION_RESULT_VANISH_NARRATED | COMBAT_ACTION_RESULT_KILL_NARRATED,
            true,
        );
        assert!(!suppressed.run_generic_chain);
        assert_eq!(suppressed.result_after, 0);

        let collision =
            resolve_combat_attack_narrator_gate(COMBAT_ACTION_RESULT_SLEEP, true);
        assert!(collision.run_generic_chain);
        assert_eq!(collision.result_after, COMBAT_ACTION_RESULT_KILL_NARRATED);

        let ordinary_hit = resolve_combat_attack_narrator_gate(0, false);
        assert!(ordinary_hit.run_generic_chain);
        assert_eq!(ordinary_hit.result_after, 0);
    }

    #[test]
    fn a_successful_faint_sleep_replaces_the_vanish_bit_with_the_sleep_bit() {
        // `combat.md §6.3`: "the sleep helper replaces the whole result field
        // with sleep bit `0x04`, losing `0x02`. ... Do not preserve both
        // `0x02` and `0x04` across the successful-sleep overwrite."
        let mut state = combat_descriptor_state();
        let leader = state.party[0];
        state.party.push(PartyMember { slot: 1, ..leader });
        state.party_names = default_party_names(2);
        state.party_equipment = default_party_equipment(2);
        state.combat_actors[1] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED,
            1,
            1,
            0,
            5,
            8,
        ]);
        state.combat_action_result = COMBAT_ACTION_RESULT_VANISH_NARRATED;

        assert_eq!(state.apply_combat_party_control_faint_scan(), Some(1));

        assert_eq!(state.combat_action_result, COMBAT_ACTION_RESULT_SLEEP);
        assert_eq!(
            state.combat_actors[1].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_STATUS_DISABLED
        );
        assert_eq!(state.party[1].status, b'S');
    }

    #[test]
    fn the_victory_census_uses_the_group_resolver_on_both_halves() {
        // `combat.md §16.1`: "Side counting skips empty, dead, and passive
        // descriptors, then uses the same group resolver. Group 1 counts as
        // foes and group 0 as friends. Control and the traitor identity
        // therefore affect victory detection." The party half of the
        // announcement gate was a raw `0x80` scan, so a charmed party member
        // still counted as a friend while the round loop's own census had
        // already moved it to the hostile side.
        let mut charmed_party = combat_descriptor_state();
        charmed_party.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED,
            0,
            0,
            0,
            5,
            5,
        ]);
        let census = combat_side_census(&charmed_party.combat_actors);
        assert_eq!((census.friends, census.foes), (0, 1));
        assert!(!charmed_party.announce_combat_victory_if_needed());

        // The mirror case: an ordinary party member plus a charmed monster.
        // §16.1 resolves `0x41` to group 0, so no foe remains and the
        // announcement fires.
        let mut charmed_monster = combat_descriptor_state();
        charmed_monster.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        charmed_monster.combat_actors[6] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_CONTROLLED,
            COMBAT_CLASS_GIANT_RAT,
            6,
            0,
            4,
            4,
        ]);
        let census = combat_side_census(&charmed_monster.combat_actors);
        assert_eq!((census.friends, census.foes), (2, 0));
        assert!(charmed_monster.announce_combat_victory_if_needed());

        // And the ordinary shape is unchanged: a live hostile suppresses it.
        let mut ordinary = combat_descriptor_state();
        ordinary.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        ordinary.combat_actors[6] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            COMBAT_CLASS_GIANT_RAT,
            6,
            0,
            4,
            4,
        ]);
        assert!(!ordinary.announce_combat_victory_if_needed());
    }

    #[test]
    fn a_slot_visit_that_dispatches_no_action_does_not_run_the_faint_tail() {
        // `combat.md §7` places the recount that "first gives the party
        // control/faint helper a chance to restore one actor" after a
        // *dispatched action*, and `§14` puts the helper on the defeat
        // transition. The helper is not read-only - it prints
        // `<name> passes out!`, plays the blocking faint envelope, removes
        // the Sword of Chaos, sleeps the member and advances a world tick -
        // so a slot visit that takes no turn must not reach it.
        let mut state = combat_ai_turn_state(8, 5);
        // `§16.1`: the controlled bit moves this party descriptor to group 1,
        // so the census sees no friend and the visit's recount reports defeat.
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;

        // Slot 1 is empty, so the visit takes the inactive arm.
        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            1, 30, false, false, 0, false, 1, 1, &[], None, 0, false, None, true, &[1, 2, 3, 4],
            &[],
        );

        assert_eq!(
            application,
            CombatActorSlotDispatchApplication::Slot {
                slot: 1,
                phase_tick: Some(CombatActorPhaseTick::Inactive),
                action: CombatActorDispatchAction::Inactive,
                control_after: CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat),
            }
        );
        assert!(
            !state.message.contains("passes out!"),
            "message was {:?}",
            state.message
        );
        assert_eq!(
            state.combat_actors[0].flags,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED
        );
        assert_eq!(state.party[0].status, b'G');

        // The end-of-round check is the same: `§7` gives it the terminal
        // table state, not the restore attempt.
        let end_of_round = state.apply_combat_actor_slot_dispatch_with_inputs(
            COMBAT_ACTOR_SLOTS,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );
        assert_eq!(
            end_of_round,
            CombatActorSlotDispatchApplication::EndOfRound {
                control: CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat),
            }
        );
        assert!(!state.message.contains("passes out!"));
        assert_eq!(state.party[0].status, b'G');
    }

    #[test]
    fn the_narrator_gate_is_scoped_to_the_generic_chain() {
        // `combat.md §6.3` scopes the suppression to "the generic
        // killed/slept/hit chain", and `§11.1` restates the reader as "a
        // wider test that halts the narrator before the kill, sleep, hit and
        // wound chain". The miss line is a separate producer there - "The
        // routine that prints a miss line has exactly two call sites, both
        // inside party-side attack helpers" - so a live `0x02` must not
        // swallow it. The engine applied the gate to the whole result line.
        use crate::input_dispatch::combat_weapon_resolution_reaches_generic_chain as reaches;

        let route = CombatWeaponAttackRangeRoute::Melee;
        assert!(reaches(CombatWeaponAttackResolution::Hit {
            route,
            raw_damage: 3,
        }));
        assert!(!reaches(CombatWeaponAttackResolution::Miss {
            route,
            hit_score: -1,
        }));
        // `§6.3`: "If that narrator is not reached, the combat walker
        // replaces the whole field with zero before the next actor
        // dispatch", so a resolution that prints nothing must not clear
        // `0x02` or set `0x01` on its own.
        assert!(!reaches(CombatWeaponAttackResolution::OutOfRange {
            target_range: 4,
            range_cap: 1,
        }));
        assert!(!reaches(CombatWeaponAttackResolution::NoOrdinaryDamage {
            route
        }));
        assert!(!reaches(CombatWeaponAttackResolution::Special {
            route,
            shattered: false,
        }));

        // End to end: a party melee miss still prints its line with the
        // vanish bit standing.
        let game_dir = std::path::Path::new(".");
        let mut miss = combat_player_command_state(6, 5);
        miss.party_strengths = vec![0];
        miss.party_experience = vec![0];
        miss.combat_actors[8].base_step = 30;
        miss.combat_action_result = COMBAT_ACTION_RESULT_VANISH_NARRATED;
        miss.prng_state = 0xFFFF;
        handle_play_key_input(
            &mut miss,
            'A',
            &format!("{}\r", char::from(INPUT_CODE_EAST)),
            game_dir,
        )
        .unwrap();
        let transcript = combat_descriptor_transcript(&miss);
        assert!(
            transcript.contains("missed!"),
            "transcript was {transcript:?}"
        );
    }

    #[test]
    fn the_party_death_arm_emits_no_cue_of_its_own() {
        // `combat.md §11.1` declares itself "the complete printed-and-audible
        // census of one attack outcome in the arena, in both directions", and
        // its "Target dies" row reads "no cue of its own; the party death arm
        // runs a full stats redraw, the monster death arms write their tiles
        // (Section 6.3)". `§6.3`'s own party-member row still lists "death
        // audio played" among that branch's writes, but it names no envelope
        // and `audio.md` publishes no combat party-death trigger at all, so
        // the later, complete and more specific census governs: this arm
        // emits the redraw and no sound. The wording conflict is filed as an
        // open spec question.
        let mut state = combat_descriptor_state();
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        state.party[0].hp = 5;
        state.visibility_dirty = false;
        let sounds_before = state.sound_effect_history.len();

        state
            .apply_combat_weapon_damage_to_target(None, 0, COMBAT_INSTANT_KILL_DAMAGE, false)
            .unwrap();

        assert_eq!(state.party[0].status, b'D');
        assert_eq!(state.party[0].hp, 0);
        assert_eq!(
            state.sound_effect_history.len(),
            sounds_before,
            "the party death arm has no cue of its own; history was {:?}",
            state.sound_effect_history
        );
        assert!(state.visibility_dirty);
    }

    #[test]
    fn the_narrator_gate_runs_at_dispatch_time_not_after_the_round_walk() {
        // `combat.md §6.3`: "If that narrator is not reached, the combat
        // walker replaces the whole field with zero before the next actor
        // dispatch." The shared action-result field therefore describes one
        // dispatch only. A round walk visits several slots before any
        // transcript is assembled, so a gate applied after the walk reads a
        // field that a later slot has already zeroed and the vanish
        // suppression silently does not fire.
        let mut state = combat_descriptor_state();
        state.party_names = default_party_names(1);
        state.party[0].hp = 30;
        // Party member far from the vanish kill, adjacent to the second
        // hostile.
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 2, 2]);
        // A hostile monster next to a controlled (group-0) vanish-class
        // creature: `§16.1` "Controlled/charmed bit on an ordinary monster
        // descriptor" resolves to "Group 0", so it is a legal target for the
        // group-1 attacker beside it.
        state.combat_actors[8] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            COMBAT_CLASS_GIANT_RAT,
            8,
            1,
            6,
            5,
        ]);
        state.combat_actors[9] = CombatActorDescriptor::from_row([
            1,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_CONTROLLED,
            47,
            9,
            1,
            7,
            5,
        ]);
        // A second hostile whose own dispatch zeroes the field before any
        // transcript is built.
        state.combat_actors[10] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            COMBAT_CLASS_GIANT_RAT,
            10,
            1,
            3,
            2,
        ]);
        for slot in [0usize, 8, 9, 10] {
            let actor = state.combat_actors[slot];
            state.active_objects[slot] = ActiveObject {
                type_byte: 0x90,
                tile: 0x90,
                x: usize::from(actor.x),
                y: usize::from(actor.y),
                ..ActiveObject::empty()
            };
        }
        let strike = CombatMonsterAttackInputs {
            forced_hit: Some(true),
            ..CombatMonsterAttackInputs::default()
        };

        let walk = state.apply_combat_round_walk_from_slot_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[(7, 5)],
            None,
            0,
            false,
            None,
            true,
            &[],
            &[(8, strike), (10, strike)],
        );
        crate::input_dispatch::append_combat_round_walk_messages(&mut state, &walk);

        let transcript = combat_descriptor_transcript(&state);
        // The vanish branch's own line still prints.
        assert!(
            transcript.contains("Shadow Lord vanishes!"),
            "transcript was {transcript:?}"
        );
        // `§6.3`: the surviving `0x02` "skips the generic killed/slept/hit
        // chain", even though a later slot's dispatch has since zeroed the
        // field.
        assert!(
            !state.message.contains("killed!"),
            "message was {:?}",
            state.message
        );
        assert!(
            !transcript.contains("killed!"),
            "transcript was {transcript:?}"
        );
        // The later dispatch's own line is unaffected.
        assert!(
            state.message.contains("hit!"),
            "message was {:?}",
            state.message
        );
    }

    /// `combat.md §11.1` announcement table, the two monster-side rows read
    /// against each other:
    ///
    /// | **Ordinary hostile monster** | **nothing whatsoever** ... and on a
    ///   melee miss no line either |
    /// | Monster carrying the controlled/charmed bit (Section 6.1a) | the
    ///   **reduced** banner ... then one fixed attempt: `Attack-`, `Aim! `,
    ///   and on a failed roll `<target> missed!` |
    ///
    /// The line that does print obeys rule 1 - "**Every result line names the
    /// target, never the attacker.** ... An engine that prints the attacker's
    /// name in the miss line produces a transcript that is wrong on every line
    /// it emits." - and §11.1's scope note says the difference between the two
    /// rows is which routine runs, not which actor runs it: "Note the scoping:
    /// *party-side helper* describes the routine, not the actor - Section
    /// 6.1a's controlled bit lets a monster reach it and lets a party member
    /// bypass it."
    fn controlled_monster_miss_state() -> PlayState {
        let mut state = controlled_monster_dispatch_state(7, 5);
        state.party_names = default_party_names(1);
        // Attacker and target must have different class names, or a
        // target-named line and an attacker-named line read the same.
        state.combat_actors[8].owner_target_class = COMBAT_CLASS_GIANT_SPIDER;
        state
    }

    fn walk_controlled_monster_slot(
        state: &mut PlayState,
        inputs: CombatMonsterAttackInputs,
    ) -> String {
        let walk = state.apply_combat_round_walk_from_slot_with_inputs(
            8,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[(7, 5)],
            None,
            0,
            false,
            None,
            true,
            &[],
            &[(8, inputs)],
        );
        crate::input_dispatch::append_combat_round_walk_messages(state, &walk);
        state.message.clone()
    }

    #[test]
    fn the_controlled_monster_dispatch_narrates_its_miss_through_the_target_named_producer() {
        let missed = CombatMonsterAttackInputs {
            forced_hit: Some(false),
            ..CombatMonsterAttackInputs::default()
        };

        let mut controlled = controlled_monster_miss_state();
        let attacker_name = combat_class_stats(COMBAT_CLASS_GIANT_SPIDER).unwrap().name;
        let target_name = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap().name;
        let message = walk_controlled_monster_slot(&mut controlled, missed);

        assert!(
            message.contains(&format!("{target_name} missed!")),
            "message was {message:?}"
        );
        assert!(
            !message.contains(&format!("{attacker_name} missed!")),
            "message was {message:?}"
        );

        // The same slot without the controlled bit is an ordinary hostile,
        // and §11.1 gives its melee miss "nothing at all" - "no newline, no
        // name, no line, no tone".
        let mut hostile = controlled_monster_miss_state();
        hostile.combat_actors[8].flags &= !COMBAT_ACTOR_FLAG_CONTROLLED;
        let hostile_message = walk_controlled_monster_slot(&mut hostile, missed);
        assert!(
            !hostile_message.contains("missed!"),
            "message was {hostile_message:?}"
        );
    }

    #[test]
    fn the_controlled_monster_dispatch_narrates_its_hit_through_the_shared_wound_grader() {
        // `combat.md §11.1` "The graded wound lines are monster-target only":
        // the controlled monster resolves to group 0, so `§16.1` target
        // selection gives it a group-1 monster, and the landed hit is graded
        // "by the target's remaining HP against its class maximum". The
        // quarter is "the class maximum divided by four with truncation, and
        // the three thresholds are one, two and three of those truncated
        // quarters".
        let landed = CombatMonsterAttackInputs {
            forced_hit: Some(true),
            ..CombatMonsterAttackInputs::default()
        };

        let mut state = controlled_monster_miss_state();
        let message = walk_controlled_monster_slot(&mut state, landed);

        let stats = combat_class_stats(COMBAT_CLASS_GIANT_RAT).unwrap();
        let remaining = state.combat_actors[9].hp_or_wound;
        assert!(remaining < stats.max_hp, "the attempt must land damage");
        assert!(remaining > 0, "this case is a wound, not a kill");
        let quarter = stats.max_hp / 4;
        let expected = if remaining >= quarter * 3 {
            "barely wounded"
        } else if remaining >= quarter * 2 {
            "lightly wounded"
        } else if remaining >= quarter {
            "heavily wounded"
        } else {
            "critical"
        };
        assert!(
            message.contains(&format!("{} {expected}!", stats.name)),
            "remaining {remaining} of {}; message was {message:?}",
            stats.max_hp
        );
    }

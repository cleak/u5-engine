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
    fn blink_phase_uses_the_invisibility_bit_not_the_dragged_under_bit() {
        // `combat.md §6.1`: `0x10` is "**Invisible / phase-hidden.** The
        // phase/blink filter", and `0x04` is "**Dragged-under
        // (Corpser-held).**" - that row's earlier name "Hidden /
        // not-yet-revealed (invisible)" is withdrawn (`RETRACTIONS.md`
        // R380). Blinking writes the phase/blink bit and nothing else.
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
        assert!(!actor.is_dragged_under());
    }

    #[test]
    fn the_doom_suppression_bypass_does_not_reach_the_dragged_under_filter() {
        // `combat.md §9`: after the bypassable phase/hidden test, "the
        // 'invisible / not-yet-revealed' flag is still rejected after the
        // phase/hidden check. This ordinary invisibility filter is not the
        // same as the special suppression-filter bypass above." `§6.1` names
        // the second bit `0x04` "**Dragged-under (Corpser-held).**" and
        // records that the §9 two-state prose "predates this correction and
        // has not been re-derived against the bit layout" (`RETRACTIONS.md`
        // R380), so `0x04` is what the unbypassable filter reads. The engine
        // fed `0x04` into the bypassable test instead, so a Doom-scene
        // monster happily targeted a dragged-under party member.
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_DRAGGED_UNDER;
        // A second, plainly targetable party member so the assertion below
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
            "the dragged-under filter is rejected even in the Doom bypass              context, so the unflagged party member is the chosen target"
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
            1, 30, false, 0, false, 1, 1, &[], None, 0, false, None, true, &[1, 2, 3, 4],
            &[]);

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
        // The `Special` route's *result* line is the same generic chain -
        // it is produced by the same landed-damage narrator the `Hit` route
        // uses - so it is inside the gate. Its `Thy sword hath shattered!`
        // is not: `§11.1` prints that "**inside** the damage roll", ahead
        // of the narrator, and the producer keeps the two apart so the gate
        // can withhold one without the other.
        assert!(reaches(CombatWeaponAttackResolution::Special {
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
        // The attacker's flat attack byte has to clear the Shadow Lord's
        // defence rating outright. `combat.md §12`: the roller "subtracts
        // an inclusive `1..rating` draw" against a non-zero rating, and the
        // Shadow Lord's class defence is `10`; a Giant Rat's attack byte of
        // `6` therefore grazes on any draw of six or more, which makes this
        // case a hostage of wherever the shared stream happens to stand.
        // A Troll's `15` clears `10` for every draw the rating can produce,
        // so the vanish kill this test is about holds whatever the stream
        // does. (It became load-bearing when the arena animator started
        // spending one coin per animated arena record in the round-loop
        // entry prologue, moving this draw four steps along.)
        state.combat_actors[8] = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            COMBAT_CLASS_TROLL,
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

    /// The turn is **prompted**, not synthesized (`§16.1`, `RETRACTIONS.md`
    /// R377), so the transcript is produced by accepting `A` and confirming
    /// the cursor on the adjacent foe - `§8.2`'s single monster-side attempt
    /// with its fixed pseudo-item, which "sends the dispatcher to the
    /// monster-side reach and effect rows of that actor's class".
    fn walk_controlled_monster_slot(
        state: &mut PlayState,
        inputs: CombatMonsterAttackInputs,
    ) -> String {
        let mut message = String::new();
        let walk = state.begin_combat_attack_walk(8, true);
        message.push_str(&walk.text);
        assert!(walk.cursor_open, "a melee-reach class opens the cursor");
        // One cursor step east, from the attacker's own cell onto the foe.
        // `§8.2`: the loop reads internal direction codes, "never by the
        // characters `1`-`4` reaching the loop unremapped".
        state.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST));
        let walk = state
            .apply_combat_targeting_cursor_key_with_monster_inputs('A', inputs)
            .expect("the confirm closes the attempt");
        message.push_str(&walk.text);
        if let Some((_, attack)) = walk.monster_attack
            && let Some(line) = crate::input_dispatch::combat_monster_attack_narrated_result_message(
                state,
                attack,
                crate::input_dispatch::CombatMonsterAttackNarration::Controlled,
            )
        {
            message.push('\n');
            message.push_str(&line);
        }
        message
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

        // The same slot without the controlled bit is an ordinary hostile. It
        // never reaches the prompt at all - `§6.1a`'s slot-to-group helper
        // sends it to the automatic driver - and `§11.1` gives its melee miss
        // "nothing at all": "no newline, no name, no line, no tone".
        let mut hostile = controlled_monster_miss_state();
        hostile.combat_actors[8].flags &= !COMBAT_ACTOR_FLAG_CONTROLLED;
        assert!(!combat_slot_prompted_by_player_command_handler(
            None,
            8,
            hostile.combat_actors[8]
        ));
        let hostile_attack = hostile
            .resolve_and_apply_combat_monster_attack(8, 9, 0, false, 0, Some(false))
            .expect("the hostile still swings on its own driver turn");
        assert_eq!(
            crate::input_dispatch::combat_monster_attack_narrated_result_message(
                &hostile,
                hostile_attack,
                crate::input_dispatch::CombatMonsterAttackNarration::SelfActingHostile,
            ),
            None
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

    // ---------------------------------------------------------------- SPEC-9057C0D
    // `systems/combat.md` 3 / 6.1 / 6.1a / 6.3 / 8 / 8.1 / 8.2 / 11 / 11.1 /
    // 16.1 at spec commit 9057c0d, and `RETRACTIONS.md` R377-R382.

    fn spec_9057c0d_transcript(state: &PlayState) -> String {
        state
            .message_entries()
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<String>()
    }

    /// `combat.md` §8.1: "Only two things precede the banner: the active-player
    /// gate and the Sword-of-Chaos gate. The turn's two status early-outs -
    /// the dragged-under (Corpser-held) arm that prints `ARGH!` and the asleep
    /// arm that prints `Zzzzz...` - come **after** it, so a dragged-under or
    /// sleeping combatant does get its full banner and then that line in place
    /// of a command." (`RETRACTIONS.md` R380.)
    #[test]
    fn the_status_early_outs_follow_the_turn_banner() {
        for (flag, line) in [
            (COMBAT_ACTOR_FLAG_DRAGGED_UNDER, COMBAT_DRAGGED_UNDER_TURN_LINE),
            (COMBAT_ACTOR_FLAG_STATUS_DISABLED, COMBAT_ASLEEP_TURN_LINE),
        ] {
            let mut state = combat_player_command_state(8, 5);
            state.combat_actors[0].flags |= flag;
            state.combat_actors[0].phase_counter = 1;
            state.message_transcript.clear();

            let application = state.apply_combat_actor_slot_dispatch(0, 30);

            let CombatActorSlotDispatchApplication::Slot { action, .. } = application else {
                panic!("the party slot should dispatch");
            };
            assert!(!matches!(action, CombatActorDispatchAction::PlayerReady));
            let printed = spec_9057c0d_transcript(&state);
            let banner = state
                .combat_turn_banner_for_actor(0)
                .expect("a prompted slot gets its banner");
            let banner_line = banner.trim_matches('\n');
            let banner_at = printed
                .find(banner_line)
                .unwrap_or_else(|| panic!("banner missing from {printed:?}"));
            let line_at = printed
                .find(line)
                .unwrap_or_else(|| panic!("{line} missing from {printed:?}"));
            assert!(
                banner_at < line_at,
                "the banner must precede {line}, got {printed:?}"
            );
        }
    }

    /// `combat.md` §8.1: the two re-prompt shapes. "Short re-prompt, banner not
    /// reprinted: an unrecognised key (`What?`), the two toggles Ctrl-S and
    /// Ctrl-B, and a refused step or refused climb." "Full re-prompt, the whole
    /// banner reprinted from its leading newline: every shape-B ... letter, `D`
    /// and `W`, every `Can't!` refusal ..., a failed `1`-`6` selection, and a
    /// declined Escape." (`RETRACTIONS.md` R380.)
    #[test]
    fn the_two_reprompt_shapes_are_classified_as_published() {
        use CombatCommandRepromptShape::{FullBanner, Short};

        let refused_gate = CombatPlayerCommandAction::Branch {
            branch: CombatCommandBranch::Get,
            party_side_gate: CombatCommandPartySideGate::RefusedMonsterSide,
        };
        let full = [
            refused_gate,
            CombatPlayerCommandAction::Branch {
                branch: CombatCommandBranch::CastSpell,
                party_side_gate: CombatCommandPartySideGate::RefusedMonsterSide,
            },
            CombatPlayerCommandAction::Branch {
                branch: CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Quit),
                party_side_gate: CombatCommandPartySideGate::NotRequired,
            },
            CombatPlayerCommandAction::Branch {
                branch: CombatCommandBranch::DWhatRefusal,
                party_side_gate: CombatCommandPartySideGate::NotRequired,
            },
            CombatPlayerCommandAction::Branch {
                branch: CombatCommandBranch::WWhatRefusal,
                party_side_gate: CombatCommandPartySideGate::NotRequired,
            },
            CombatPlayerCommandAction::ActivePlayerSelection(
                CombatActivePlayerSelectionOutcome::Invalid,
            ),
            CombatPlayerCommandAction::EscapeCleanup {
                application: CombatEscapeCleanupApplication::refused(
                    CombatEscapeCleanupDecision::RefusedNotYet,
                ),
            },
        ];
        for action in full {
            assert_eq!(
                combat_player_command_action_reprompt_shape(&action),
                Some(FullBanner),
                "{action:?}"
            );
        }

        let short = [
            CombatPlayerCommandAction::Branch {
                branch: CombatCommandBranch::ToggleMusic,
                party_side_gate: CombatCommandPartySideGate::NotRequired,
            },
            CombatPlayerCommandAction::Branch {
                branch: CombatCommandBranch::Invalid,
                party_side_gate: CombatCommandPartySideGate::NotRequired,
            },
            CombatPlayerCommandAction::InvalidDirection { direction_code: 5 },
            CombatPlayerCommandAction::StepOrAttack {
                direction_code: 1,
                outcome: CombatStepOrAttackPrimitiveOutcome::BlockedWall,
            },
        ];
        for action in short {
            assert_eq!(
                combat_player_command_action_reprompt_shape(&action),
                Some(Short),
                "{action:?}"
            );
        }

        // "Neither shape spends the turn": a committed action has no shape.
        assert_eq!(
            combat_player_command_action_reprompt_shape(
                &CombatPlayerCommandAction::OpenTargetingCursor
            ),
            None
        );
    }

    /// `combat.md` §8.2, the melee occupancy lookup: it "rejects, in addition to
    /// an empty cell: a slot carrying neither the party-side nor the
    /// monster-side class bit (an empty or decoration record), a dead-marked
    /// slot, a **dragged-under (Corpser-held)** slot, and a slot whose linked
    /// presentation record carries the hidden-frame marker". It "tests **no
    /// invisibility flag at all**" (`RETRACTIONS.md` R380).
    #[test]
    fn the_melee_occupancy_lookup_rejects_the_published_list_and_no_invisibility() {
        let mut state = combat_player_command_state(8, 5);
        let cell = (state.combat_actors[8].x, state.combat_actors[8].y);
        assert_eq!(state.combat_targeting_occupant_at(cell), Some(8));

        // Invisibility is not on the list: an invisible occupant is still
        // returned.
        state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER;
        assert_eq!(state.combat_targeting_occupant_at(cell), Some(8));
        state.combat_actors[8].flags &= !COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER;

        for flag in [
            COMBAT_ACTOR_FLAG_DRAGGED_UNDER,
            COMBAT_ACTOR_FLAG_MARKED_DEAD,
        ] {
            state.combat_actors[8].flags |= flag;
            assert_eq!(state.combat_targeting_occupant_at(cell), None, "flag {flag}");
            state.combat_actors[8].flags &= !flag;
        }

        // "a slot carrying neither the party-side nor the monster-side class
        // bit (an empty or decoration record)".
        let class_bits = state.combat_actors[8].flags
            & (COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_SELECTABLE_40);
        state.combat_actors[8].flags &= !class_bits;
        assert_eq!(state.combat_targeting_occupant_at(cell), None);
        state.combat_actors[8].flags |= class_bits;
        assert_eq!(state.combat_targeting_occupant_at(cell), Some(8));

        // "a slot whose linked presentation record carries the hidden-frame
        // marker".
        let linked = usize::from(state.combat_actors[8].active_object_slot);
        state.active_objects[linked].tile = COMBAT_HIDDEN_ACTIVE_OBJECT_TILE;
        assert_eq!(state.combat_targeting_occupant_at(cell), None);
    }

    /// `combat.md` §8.2: "`Nothing!` therefore has three routes on the melee arm,
    /// not one. Escape; Space while the cursor sits on the attacker's own cell;
    /// and a **confirm** on a cell that is in range but holds nobody the lookup
    /// accepts. The third is not a cancellation ... but it reaches the same line
    /// and ends the turn the same way. Enter and `A` on the attacker's own cell
    /// are the one input pair that does nothing at all."
    #[test]
    fn the_melee_arm_reaches_nothing_by_three_routes_and_holds_on_enter() {
        // Escape.
        let mut escaped = combat_player_command_state(8, 5);
        escaped.begin_combat_attack_walk(0, true);
        let walk = escaped
            .apply_combat_targeting_cursor_key('\u{1b}')
            .expect("the cursor is open");
        assert!(walk.text.starts_with(COMBAT_TARGETING_NOTHING_LINE));
        assert!(!walk.cursor_open, "the turn is spent");

        // Space on the attacker's own cell.
        let mut spaced = combat_player_command_state(8, 5);
        spaced.begin_combat_attack_walk(0, true);
        let walk = spaced
            .apply_combat_targeting_cursor_key(' ')
            .expect("the cursor is open");
        assert!(walk.text.starts_with(COMBAT_TARGETING_NOTHING_LINE));

        // A confirm on an in-range cell holding nobody the lookup accepts.
        let mut confirmed = combat_player_command_state(8, 5);
        confirmed.begin_combat_attack_walk(0, true);
        let attacker = confirmed.combat_actor_cell(0).unwrap();
        confirmed.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST));
        let cursor = confirmed
            .active_combat_targeting
            .as_ref()
            .map(|session| session.cursor)
            .unwrap();
        assert_ne!(cursor, attacker, "the cursor moved one cell in range");
        assert_eq!(confirmed.combat_targeting_occupant_at(cursor), None);
        let walk = confirmed
            .apply_combat_targeting_cursor_key('\r')
            .expect("the cursor is open");
        assert!(walk.text.starts_with(COMBAT_TARGETING_NOTHING_LINE));
        assert!(!walk.cursor_open, "the confirm still spends the turn");

        // "Enter and `A` on the attacker's own cell ... nothing at all".
        for key in ['\r', 'A', 'a'] {
            let mut held = combat_player_command_state(8, 5);
            held.begin_combat_attack_walk(0, true);
            let walk = held
                .apply_combat_targeting_cursor_key(key)
                .expect("the cursor is open");
            assert!(walk.text.is_empty(), "key {key:?} printed {:?}", walk.text);
            assert!(walk.cursor_open, "key {key:?} closed the cursor");
        }
    }

    /// `combat.md` §8.2's seed gate: "that slot must be neither dead-marked nor
    /// blink-hidden". Bit `0x04` is the dragged-under state and is not on
    /// that list (`RETRACTIONS.md` R380).
    #[test]
    fn the_melee_cursor_seed_gate_rejects_blink_hidden_not_dragged_under() {
        let attacker = (5, 5);
        let target = CombatActorDescriptor::from_row([
            10,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            20,
            9,
            0,
            5,
            6,
        ]);
        assert_eq!(
            combat_targeting_cursor_start(attacker, Some(target), true, 1),
            (5, 6)
        );

        let mut blinked = target;
        blinked.flags |= COMBAT_ACTOR_FLAG_PHASE_BLINK_FILTER;
        assert_eq!(
            combat_targeting_cursor_start(attacker, Some(blinked), true, 1),
            attacker
        );

        let mut dead = target;
        dead.flags |= COMBAT_ACTOR_FLAG_MARKED_DEAD;
        assert_eq!(
            combat_targeting_cursor_start(attacker, Some(dead), true, 1),
            attacker
        );

        // "its linked presentation record must be displayed".
        assert_eq!(
            combat_targeting_cursor_start(attacker, Some(target), false, 1),
            attacker
        );

        // The dragged-under bit does not disturb the seed.
        let mut dragged = target;
        dragged.flags |= COMBAT_ACTOR_FLAG_DRAGGED_UNDER;
        assert_eq!(
            combat_targeting_cursor_start(attacker, Some(dragged), true, 1),
            (5, 6)
        );
    }

    /// `combat.md` §8.2: "For a **monster-side** actor under player control there
    /// is no equipment to walk and the walker is skipped outright: `A` makes
    /// **exactly one attempt**, unconditionally and without a loop, carrying a
    /// fixed pseudo-item that sends the dispatcher to the monster-side reach and
    /// effect rows of that actor's class."
    ///
    /// §11's dispatcher row folds selector `1` "to zero, selecting the **melee /
    /// Aim-cursor arm**", while "a selector above `1`" selects "the cast/effect
    /// arm unconditionally". `magic.md` §8 and `catalogs/spell-list.md` route the
    /// summoned-creature transcript through the same selector rather than a
    /// fixed transcript (`RETRACTIONS.md` R382).
    #[test]
    fn a_monster_side_actor_takes_one_attempt_chosen_by_its_class_reach_selector() {
        // Swarm's Insect Swarm is melee-reach; Summon's Daemon is not.
        assert_eq!(
            combat_ranged_effect_stats(COMBAT_CLASS_INSECT_SWARM)
                .unwrap()
                .range_effect_selector,
            1
        );
        assert!(
            combat_ranged_effect_stats(38).unwrap().range_effect_selector > 1,
            "Summon's Daemon is a non-melee-reach class"
        );

        // §8.2: "On the monster side, a class reach of exactly 1 is normalised
        // to zero, so it takes the fixed-range-one melee path rather than a
        // one-cell ranged cursor." §11's dispatcher table publishes the same
        // row as selector `1` "folded to zero, selecting the **melee /
        // Aim-cursor arm**".
        let melee = CombatAttackAttempt::for_monster_class(1);
        assert!(melee.melee_arm && !melee.class_effect_arm && melee.max_range == 1);
        // Selector `0` is an **engine fallback, not a published row**: §11's
        // table publishes only "Selector `1` (most classes)" and "A selector
        // above `1`", and the sentence above pins selector 1. §11 does say
        // both side tables are "dense forty-eight-entry arrays with a defined
        // byte for every class", so no class reaches this arm without a
        // selector; folding 0 in with 1 keeps it off the cast/effect arm,
        // which is the conservative choice.
        assert_eq!(CombatAttackAttempt::for_monster_class(0), melee);
        let effect = CombatAttackAttempt::for_monster_class(9);
        assert!(effect.class_effect_arm && !effect.melee_arm);

        for (class, opens_cursor) in [(COMBAT_CLASS_INSECT_SWARM, true), (38, false)] {
            let mut state = controlled_monster_dispatch_state(7, 5);
            state.combat_actors[8].owner_target_class = class;
            let attempts = state.combat_attack_attempts_for_actor(8);
            assert_eq!(attempts.len(), 1, "class {class} must take one attempt");

            state.open_pending_combat_player_turn(Some(8));
            let walk = state.begin_combat_attack_walk(8, true);
            // "`Attack-` and the reduced banner are unaffected: they precede
            // the dispatcher and print for every class."
            assert!(walk.text.starts_with(COMBAT_ATTACK_LABEL), "class {class}");
            assert_eq!(
                walk.text.contains(COMBAT_ATTACK_AIM_PROMPT),
                opens_cursor,
                "class {class} printed {:?}",
                walk.text
            );
            assert_eq!(walk.cursor_open, opens_cursor, "class {class}");
        }
    }

    /// `combat.md` §8, the `Z` row: "for a **party-side** actor it opens that
    /// character's own sheet silently, with no prompt; for a **monster-side**
    /// actor under player control it prints `Player: ` and runs the ordinary
    /// roster picker" (`RETRACTIONS.md` R381).
    #[test]
    fn z_stats_prompts_for_a_monster_side_actor_and_not_for_a_party_one() {
        let game_dir = std::path::Path::new(".");

        let mut party = combat_player_command_state(8, 5);
        party.open_pending_combat_player_turn(Some(0));
        handle_play_key_input(&mut party, 'Z', "", game_dir).unwrap();
        assert!(party.active_party_selector.is_none());
        assert!(party.active_z_stats.is_some());

        let mut monster = combat_player_command_state(8, 5);
        monster.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        monster.open_pending_combat_player_turn(Some(8));
        handle_play_key_input(&mut monster, 'Z', "", game_dir).unwrap();
        assert!(
            monster.active_party_selector.is_some(),
            "a monster-side actor runs the roster picker"
        );
        assert!(monster.active_z_stats.is_none());
        assert!(monster.message.contains(PARTY_SELECTION_PROMPT));
    }

    /// `combat.md` §3: "Only a party-side actor participates in direction
    /// sharing ... Monster-side actors neither seed nor test it." §8's
    /// direction bullet says the same: "a monster acting under player control
    /// neither seeds nor tests the shared direction and simply leaves".
    #[test]
    fn the_arena_exit_skips_the_same_exit_constraint_for_a_monster_side_actor() {
        // A party-side actor in a constrained encounter is refused.
        assert!(matches!(
            resolve_combat_out_of_arena_leave(false, 2, false, true, Some(1), true, true),
            CombatOutOfArenaLeaveOutcome::RefusedConstrainedDirection { .. }
        ));
        // The same request from a monster-side actor leaves, and leaves the
        // shared byte exactly as it found it.
        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 2, false, true, Some(1), true, false),
            CombatOutOfArenaLeaveOutcome::Accepted {
                direction_code: 2,
                presentation: CombatOutOfArenaLeavePresentation::EscapeWithFoes,
                established_direction_code: Some(1),
            }
        );
        // An unseeded encounter is not seeded by a monster-side departure.
        assert_eq!(
            resolve_combat_out_of_arena_leave(false, 2, false, true, None, true, false),
            CombatOutOfArenaLeaveOutcome::Accepted {
                direction_code: 2,
                presentation: CombatOutOfArenaLeavePresentation::EscapeWithFoes,
                established_direction_code: None,
            }
        );
    }

    /// `combat.md` §6.3, the party-member death row, "in this exact order:
    /// character HP forced to zero; marked-dead bit ORed into the descriptor
    /// flags byte; roster status byte set to `'D'`; the corpse tile written into
    /// both tile bytes; active-player sentinel set to `0xFF` if the dead
    /// character was active; **a full stats-panel redraw**. **No sound at any
    /// point**." (`RETRACTIONS.md` R379; `audio.md` §9 lists this among its
    /// silence boundaries.)
    #[test]
    fn the_party_death_arm_writes_in_the_published_order_and_plays_no_cue() {
        let mut state = combat_player_command_state(8, 5);
        state.party[0].hp = 4;
        state.party[0].status = b'G';
        state.active_player = Some(0);
        state.visibility_dirty = false;
        let sounds_before = state.sound_effect_history.len();

        let outcome = state
            .apply_combat_party_damage_to_slot(0, 99)
            .expect("the party slot takes the death branch");

        assert!(outcome.killed);
        assert_eq!(state.party[0].hp, 0);
        assert!(state.combat_actors[0].is_marked_dead());
        assert_eq!(state.party[0].status, b'D');
        let linked = usize::from(state.combat_actors[0].active_object_slot);
        assert_eq!(state.active_objects[linked].tile, COMBAT_PARTY_CORPSE_TILE);
        assert_eq!(
            state.active_objects[linked].type_byte,
            COMBAT_PARTY_CORPSE_TILE
        );
        assert_eq!(state.active_player, None);
        assert!(state.visibility_dirty, "the full stats-panel redraw runs");
        assert_eq!(
            state.sound_effect_history.len(),
            sounds_before,
            "no party-death cue: {:?}",
            state.sound_effect_history
        );

        // The helper that owns the interleaving leaves the roster letter to the
        // caller so the marked-dead bit can precede it.
        let mut member = state.party[0];
        member.hp = 4;
        member.status = b'G';
        let deferred = apply_combat_party_damage_deferring_death_letter(&mut member, 99);
        assert!(deferred.killed);
        assert_eq!(member.hp, 0);
        assert_eq!(member.status, b'G', "the letter is deferred to the caller");
    }

    /// `combat.md` §8.2: "A move is applied only if the destination stays inside
    /// the eleven-by-eleven arena **and** its distance from the attacker does
    /// not exceed the maximum range. If either test fails the cursor simply
    /// does not move: no message, no beep, no turn consumed, and the loop reads
    /// another key."
    ///
    /// `RETRACTIONS.md` R378: "This is the row to grep for: an engine that
    /// applied the withdrawn universal form to a controlled monster refuses
    /// that monster's keystroke and gives it one distance-gated automatic
    /// strike, which is exactly the behaviour R377 also withdraws." Same
    /// distance-one number, opposite mechanism.
    #[test]
    fn the_prompted_paths_distance_one_number_is_a_cursor_clamp_not_a_refusal() {
        let mut state = combat_player_command_state(8, 5);
        let walk = state.begin_combat_attack_walk(0, true);
        assert!(walk.cursor_open);
        let attacker = state.combat_actor_cell(0).unwrap();

        // One step east is in range; a second is not.
        state.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST));
        let in_range = state
            .active_combat_targeting
            .as_ref()
            .map(|session| session.cursor)
            .unwrap();
        assert_ne!(in_range, attacker);

        let held = state
            .apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST))
            .expect("the cursor is still open");
        assert!(held.text.is_empty(), "no message: {:?}", held.text);
        assert!(held.cursor_open, "no turn is consumed");
        assert_eq!(
            state
                .active_combat_targeting
                .as_ref()
                .map(|session| session.cursor),
            Some(in_range),
            "the cursor silently refuses to move out of range"
        );
    }

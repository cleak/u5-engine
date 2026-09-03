// `systems/combat.md`, `systems/audio.md` and `systems/encounters.md`
// conformance regressions for the monster-AI and effect rows of the combat
// verification checklist (C-192, C-209, C-210, C-218, C-239, C-281, C-284,
// C-293, C-314, C-319, C-323, C-388, C-392, C-398, C-406, C-420, C-425, E-03).
//
// Every test below names the published sentence it pins.

    /// `combat.md §7` step 7 fixture: one party actor standing on the cell
    /// whose hazard the test selects.
    fn hazard_pass_state() -> PlayState {
        let mut state = combat_ai_turn_state(9, 9);
        // The shared fixture's arena byte is `0x04`, which is the swamp the
        // Poison arm keys on, so start from a cell that selects nothing and
        // let each case name its own hazard.
        state.combat_terrain = [[TOWN_DOOR_CLEARED_TILE; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        // `combat.md §11`: an "ordinary live entry" is a linked record whose
        // tile/class byte is below `0x80`, which is what a seated party
        // member's class tile is (`§5`).
        state.active_objects[0].tile = 0x03;
        state.active_objects[0].type_byte = 0x03;
        state.party[0].status = b'G';
        state.party[0].hp = 30;
        state.party[0].max_hp = 30;
        state.prng_state = 0x2222;
        state
    }

    #[test]
    fn hazard_pass_damaging_tiers_record_the_hit_sound_and_only_the_middle_leaves_combat() {
        // `combat.md §7` step 7: "Three damaging kinds are recognized, each
        // with its own effect: a low tier that applies the party status/damage
        // path with the no-attacker sentinel and plays the hit sound, but only
        // while the actor's own object entry is an ordinary live entry; a
        // middle tier that plays the hit sound, rolls a small random amount,
        // feeds it to the damage-and-status resolver, runs the shared finalize
        // hook and raises the leave-combat flag".
        //
        // `§11` keys those two: Poison is the tier whose contact is rejected
        // when the linked record's tile byte "is at least `0x80`" and whose
        // accepted arm runs "with no attacker credit"; Fire is the tier that
        // passes a rolled raw value to the shared endpoint "then run[s] the
        // ordinary no-attacker finalization and status-panel refresh".
        //
        // "The hit sound" and `§11`'s "target sound" name a sound but publish
        // no recipe, and `§11.1` lists the standing-hazard tier under "Not
        // covered", so the tier is recorded and no cue is emitted. These
        // assertions pin the gate and the silence together.

        // Low tier: the swamp arena byte, an ordinary live linked record.
        let mut low = hazard_pass_state();
        low.combat_terrain[5][5] = COMBAT_CONTACT_TERRAIN_SWAMP;
        let serial = low.sound_effect_serial;
        let contact = low
            .apply_combat_post_dispatch_contact_for_actor_position(0)
            .expect("swamp terrain is a recognised hazard kind");
        assert_eq!(contact.tier, Some(CombatHazardTier::Low));
        assert!(contact.hit_sound_played);
        assert!(!contact.finalize_hook_ran);
        assert!(!contact.raises_leave_combat_flag);
        assert!(
            low.sound_effects_after(serial).is_empty(),
            "no document publishes a program for the hazard tier's hit sound"
        );

        // Low tier, linked record at or above `0x80`: `§7` withholds the hit
        // sound because the actor's own object entry is not an ordinary live
        // entry.
        let mut suppressed = hazard_pass_state();
        suppressed.combat_terrain[5][5] = COMBAT_CONTACT_TERRAIN_SWAMP;
        suppressed.active_objects[0].tile = 0x90;
        let serial = suppressed.sound_effect_serial;
        let contact = suppressed
            .apply_combat_post_dispatch_contact_for_actor_position(0)
            .expect("the kind is still recognised");
        assert_eq!(contact.tier, Some(CombatHazardTier::Low));
        assert!(!contact.hit_sound_played);
        assert!(suppressed.sound_effects_after(serial).is_empty());

        // Middle tier: molten lava.
        let mut middle = hazard_pass_state();
        middle.combat_terrain[5][5] = COMBAT_CONTACT_TERRAIN_MOLTEN_LAVA;
        let serial = middle.sound_effect_serial;
        let contact = middle
            .apply_combat_post_dispatch_contact_for_actor_position(0)
            .expect("lava is a recognised hazard kind");
        assert_eq!(contact.tier, Some(CombatHazardTier::Middle));
        assert!(contact.hit_sound_played);
        assert!(contact.finalize_hook_ran);
        assert!(contact.raises_leave_combat_flag);
        assert!(
            middle.sound_effects_after(serial).is_empty(),
            "no document publishes a program for the hazard tier's target sound"
        );

        // `§11`: the Sleep marker still writes its own status result, but it
        // is not one of the damaging kinds, so it costs no hit sound, no
        // finalize hook and no leave-combat flag.
        let mut sleep = hazard_pass_state();
        sleep.active_objects[1] = ActiveObject {
            type_byte: COMBAT_FIELD_KIND_SLEEP,
            tile: COMBAT_FIELD_KIND_SLEEP,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        let serial = sleep.sound_effect_serial;
        let contact = sleep
            .apply_combat_post_dispatch_contact_for_actor_position(0)
            .expect("a sleep marker is still recognised by the hook");
        assert_eq!(contact.tier, None);
        assert!(!contact.hit_sound_played);
        assert!(!contact.finalize_hook_ran);
        assert!(!contact.raises_leave_combat_flag);
        assert!(sleep.sound_effects_after(serial).is_empty());
        assert_eq!(sleep.party[0].status, b'S');

        // `§11`: "The Energy marker is not recognized by this contact hook."
        let mut energy = hazard_pass_state();
        energy.active_objects[1] = ActiveObject {
            type_byte: COMBAT_FIELD_KIND_ENERGY,
            tile: COMBAT_FIELD_KIND_ENERGY,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        let serial = energy.sound_effect_serial;
        assert_eq!(
            energy.apply_combat_post_dispatch_contact_for_actor_position(0),
            None
        );
        assert!(energy.sound_effects_after(serial).is_empty());
    }

    #[test]
    fn charm_cursor_refuses_a_restraint_cell_that_every_other_reader_still_targets() {
        // `combat.md §7.1`: a restrained actor "is returned by the
        // cell-occupancy lookup, so it can be targeted, attacked and killed
        // normally. The one exception is the Charm spell, whose own cursor
        // explicitly refuses restraint cells."
        for restraint in [JIMMY_STOCKS_TILE, JIMMY_MANACLES_TILE] {
            let mut state = combat_ai_turn_state(6, 5);
            assert!(
                state.charm_prompt_target_is_eligible(8, COMBAT_TARGET_GROUP_PARTY),
                "an unrestrained hostile is an ordinary Charm target"
            );

            state.combat_terrain[5][6] = restraint;
            assert!(
                !state.charm_prompt_target_is_eligible(8, COMBAT_TARGET_GROUP_PARTY),
                "Charm's cursor refuses restraint tile {restraint:#04x}"
            );
            // Everything else still sees the actor: it keeps its cell, so the
            // occupancy lookup and the ordinary attack path reach it.
            assert!(
                combat_actor_occupies_arena_cell(state.combat_actors[8], 6, 5),
                "the cell-occupancy lookup still returns a restrained actor"
            );
        }
    }

    #[test]
    fn combat_jimmy_with_no_keys_refuses_before_the_direction_prompt() {
        // `combat.md §7.1`: "Jimmy first requires the party to hold at least
        // one key: with a key count of zero it prints `No keys!` and returns
        // immediately, **before the direction prompt** and before any tile is
        // examined."
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.keys = 0;
        state.combat_terrain[5][6] = JIMMY_STOCKS_TILE;

        assert_eq!(
            handle_play_key_input(&mut state, 'J', "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );

        // `combat.md §8` Shape A "prints the verb label, then requires ...",
        // so the delegate's own refusal follows the label instead of erasing
        // it - the same order the dead-actor arm of this shape uses.
        assert!(
            state.message.starts_with(&format!(
                "{}{COMBAT_JIMMY_NO_KEYS_MESSAGE}",
                combat_command_branch_published_label(CombatCommandBranch::Jimmy)
                    .expect("`§8` publishes the Jimmy label")
            )),
            "message was {:?}",
            state.message
        );
        assert!(
            state.active_direction_prompt.is_none(),
            "the refusal precedes the direction prompt"
        );
        // "before any tile is examined": the restraint tile is untouched and
        // no key was spent.
        assert_eq!(state.combat_terrain[5][6], JIMMY_STOCKS_TILE);
        assert_eq!(state.keys, 0);

        // With a key the same command reaches the direction prompt instead.
        let mut keyed = combat_player_command_state(10, 10);
        keyed.keys = 1;
        handle_play_key_input(&mut keyed, 'J', "", game_dir).unwrap();
        assert!(matches!(
            keyed.active_direction_prompt.map(|session| session.kind),
            Some(DirectionPromptKind::CombatSjog {
                actor_slot: 0,
                branch: CombatCommandBranch::Jimmy,
            })
        ));
    }

    #[test]
    fn combat_shape_a_dead_actor_prints_the_published_refusal() {
        // `combat.md §8` Shape A: "The helper prints the verb label, then
        // requires that the acting combatant is still alive. A dead actor gets
        // the short \"Can't!\" refusal and the prompt is re-issued at no cost."
        let game_dir = std::path::Path::new(".");
        for (key, branch) in [
            ('G', CombatCommandBranch::Get),
            ('J', CombatCommandBranch::Jimmy),
            ('O', CombatCommandBranch::Open),
            ('R', CombatCommandBranch::Ready),
            ('S', CombatCommandBranch::Search),
            ('U', CombatCommandBranch::UseItem),
        ] {
            assert!(combat_command_branch_requires_live_active_actor(branch));
            let mut state = combat_player_command_state(10, 10);
            state.keys = 1;
            state.pending_combat_actor_slot = Some(0);
            state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_MARKED_DEAD;
            let round_before = state.combat_round_counter;

            handle_play_key_input(&mut state, key, "", game_dir).unwrap();

            assert!(
                state.message.ends_with(COMBAT_LIVE_ACTOR_REFUSAL),
                "{key} printed {:?}",
                state.message
            );
            if let Some(label) = combat_command_branch_published_label(branch) {
                assert!(
                    state.message.starts_with(label),
                    "{key} must print the verb label before the refusal, got {:?}",
                    state.message
                );
            }
            assert!(
                state.active_direction_prompt.is_none(),
                "{key} must not open a follow-up prompt"
            );
            assert_eq!(
                state.combat_round_counter, round_before,
                "the re-prompt is free"
            );

            // A key that is not one of the six Shape-A letters stays silent on
            // the same non-acting slot.
            let mut quiet = combat_player_command_state(10, 10);
            quiet.pending_combat_actor_slot = Some(0);
            quiet.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_MARKED_DEAD;
            handle_play_key_input(&mut quiet, 'Z', "", game_dir).unwrap();
            assert_eq!(quiet.message, "");
        }
        assert_eq!(COMBAT_LIVE_ACTOR_REFUSAL, "Can't!");
    }

    #[test]
    fn combat_typeahead_toggle_byte_flips_the_engine_wide_setting_and_reprompts() {
        // `combat.md §8`: "**Ctrl-B** - combat's own copy of the
        // typeahead-buffer toggle, writing the same engine-wide setting as the
        // resident one (`commands.md`). Re-prompts."
        //
        // The frontend binding that turns the physical chord into this byte is
        // pinned in `u5-bevy`; this half pins that the byte is the toggle and
        // that it costs the actor nothing.
        let game_dir = std::path::Path::new(".");
        let mut state = combat_player_command_state(10, 10);
        state.typeahead_buffer_enabled = false;
        let round_before = state.combat_round_counter;
        let actor_x = state.combat_actors[0].x;

        assert_eq!(
            handle_play_key_input(&mut state, PLAY_TYPEAHEAD_TOGGLE_KEY, "", game_dir).unwrap(),
            PlayInputDisposition::Continue
        );
        assert!(state.typeahead_buffer_enabled);
        assert_eq!(state.message, "Buffer On.");
        assert_eq!(state.combat_round_counter, round_before);
        assert_eq!(state.combat_actors[0].x, actor_x);

        handle_play_key_input(&mut state, PLAY_TYPEAHEAD_TOGGLE_KEY, "", game_dir).unwrap();
        assert!(!state.typeahead_buffer_enabled);
        assert_eq!(state.message, "Buffer Off.");
    }

    #[test]
    fn the_driver_gates_precede_the_sleep_wake_roll() {
        // `combat.md §9`: "Both gates precede the invisibility, sleep-wake and
        // flee checks below, so a skipped dispatch does not run the wake roll
        // either."
        for tag in [NEGATE_TIME_ACTIVE_EFFECT_TAG, QUICKNESS_ACTIVE_EFFECT_TAG] {
            let mut state = combat_ai_turn_state(6, 5);
            state.combat_actors[8].phase_counter = 1;
            state.combat_actors[8].set_status_disabled();
            state.active_effect_tag = Some(tag);
            state.active_effect_counter = 5;
            // Quickness's inclusive `0..1` gate consumes the dispatch on a
            // zero, so pick the seed that draws one.
            state.prng_state = if tag == QUICKNESS_ACTIVE_EFFECT_TAG {
                let mut seed = 0u16;
                loop {
                    let mut probe = seed;
                    if u5_prng_range_u16(&mut probe, 0, 1) == 0 {
                        break seed;
                    }
                    seed = seed.wrapping_add(1);
                }
            } else {
                0x1234
            };

            let mut expected = state.prng_state;
            if tag == QUICKNESS_ACTIVE_EFFECT_TAG {
                // The Quickness gate itself is the one draw the dispatch may
                // spend; the wake roll must not follow it.
                let _ = u5_prng_range_u16(&mut expected, 0, 1);
            }

            let application = state.apply_combat_actor_slot_dispatch(8, 30, false);

            assert!(
                matches!(
                    application,
                    CombatActorSlotDispatchApplication::Slot {
                        action: CombatActorDispatchAction::NegateTimeSkipped
                            | CombatActorDispatchAction::QuicknessSkipped,
                        ..
                    }
                ),
                "{tag} should skip the dispatch, got {application:?}"
            );
            assert_eq!(
                state.prng_state, expected,
                "a skipped dispatch must not spend the `0..16` wake draw"
            );
            assert!(
                state.combat_actors[8].is_status_disabled(),
                "no wake roll ran, so the actor is still asleep"
            );
        }
    }

    #[test]
    fn possess_candidate_acceptance_is_exactly_the_published_five_rejections() {
        // `combat.md §9`: "The drawn slot is accepted only if it is party-side
        // and none of marked-dead, phased/blinked, asleep-or-disabled,
        // hidden/not-yet-revealed, or already controlled is set." The
        // active-player sentinel is not on that list, and `§9`'s own landing
        // step - "the active-player sentinel is cleared to 'none' if the
        // sentinel currently names the possessed character" - is unreachable
        // unless such a target can be drawn.
        let live =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        let mut state = combat_ai_turn_state(6, 5);
        state.combat_actors[0] = live;
        state.active_player = Some(0);

        assert!(
            state.combat_ai_possess_candidate_reaches_resistance_from_roll(0),
            "the sentinel naming the target must not remove it from the draw"
        );

        let candidate = possess_candidate(live, Some(possess_member(b'G', 10)));
        assert!(combat_possess_candidate_reaches_resistance(0, candidate));
        assert_eq!(
            resolve_combat_possess_candidate_slot(&[candidate], 0),
            Some(0)
        );
    }

    #[test]
    fn monster_summon_daemon_plays_the_published_flame_transition() {
        // `combat.md §9`: on success "the acting monster's name and a short
        // summoning line are printed with a sound, and the new actor's linked
        // sprite plays the brief flame transition before settling on the
        // Daemon tile."
        //
        // `audio.md §8.3.1` specifies that transition: "one pass of 256 plots,
        // with no outer repeat", "an input/redraw poll after every eighth
        // completed step - 31 checkpoints, and none after the final step",
        // "flash tile = creature class x 4 + 320", and "the settle tile that
        // replaces it is creature class x 4 + 64".
        let flash = combat_summon_flash_playback(COMBAT_CLASS_DAEMON, 9, 4, (6, 4));

        assert_eq!(flash.actor_slot, 9);
        assert_eq!(flash.active_object_slot, 4);
        assert_eq!(flash.arena_cell, (6, 4));
        assert_eq!(flash.flash_tile, u16::from(COMBAT_CLASS_DAEMON) * 4 + 320);
        assert_eq!(flash.settle_tile, COMBAT_CLASS_DAEMON * 4 + 64);
        assert_eq!(flash.write_order.len(), 256);
        let unique: std::collections::HashSet<_> = flash.write_order.iter().copied().collect();
        assert_eq!(unique.len(), 256, "one pass of 256 plots, no repeats");
        assert_eq!(
            flash.world_tick_after_operations,
            (1..=31u16).map(|step| step * 8).collect::<Vec<_>>(),
            "31 checkpoints, none after the final step"
        );
    }

    #[test]
    fn exhausted_cardinal_fallback_is_not_consumed_when_the_last_draw_repeats_the_first() {
        // `combat.md §9`: "When all four attempts fail, the routine still
        // reports the action as consumed **unless the final draw happened to
        // be the first direction tried**, and the committed displacement in
        // that case is zero."
        let blocked = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        let step_vector = CombatStepVector { dx: 1, dy: 0 };

        assert_eq!(
            resolve_combat_ai_movement(&blocked, 5, 5, step_vector, false, None, true, &[1, 2, 3, 4]),
            CombatAiMovementOutcome::Blocked {
                random_cardinal_attempts: 4,
                action_consumed: true,
            }
        );
        assert_eq!(
            resolve_combat_ai_movement(&blocked, 5, 5, step_vector, false, None, true, &[1, 2, 3, 1]),
            CombatAiMovementOutcome::Blocked {
                random_cardinal_attempts: 4,
                action_consumed: false,
            }
        );
        // A fallback that never spent all four attempts is unaffected by the
        // exception.
        assert!(combat_ai_exhausted_fallback_consumes_action(&[1, 2, 1]));
        assert!(combat_ai_exhausted_fallback_consumes_action(&[]));
    }

    #[test]
    fn repel_undead_is_the_cause_fear_sweep_narrowed_to_the_undead_flag() {
        // `combat.md §9`: "Repel Undead is exactly the same sweep with one
        // extra condition: the actor's class must also carry the undead
        // class-flag bit. It writes the same two values and nothing else."
        //
        // "Exactly the same sweep" is the Cause Fear acceptance test - live,
        // monster-side, not one of the three protected special classes - so a
        // blinked Ghost and an asleep Skeleton are both swept.
        let mut actors = [CombatActorDescriptor::default(); COMBAT_ACTOR_SLOTS];
        let ghost = 6;
        let skeleton = 7;
        let orc = 8;
        let protected = 9;
        actors[ghost] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            23,
            ghost as u8,
            0,
            2,
            2,
        ]);
        actors[skeleton] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40 | COMBAT_ACTOR_FLAG_STATUS_DISABLED,
            33,
            skeleton as u8,
            0,
            3,
            3,
        ]);
        actors[orc] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            32,
            orc as u8,
            0,
            4,
            4,
        ]);
        actors[protected] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            47,
            protected as u8,
            0,
            5,
            5,
        ]);

        let groups = [COMBAT_TARGET_GROUP_PARTY; COMBAT_ACTOR_SLOTS];
        let none_immune = [false; COMBAT_ACTOR_SLOTS];
        let cause_fear = collect_cause_fear_actor_slots(
            &actors,
            &groups,
            COMBAT_TARGET_GROUP_PARTY,
            &none_immune,
        );
        let repel =
            collect_repel_undead_actor_slots(&actors, &groups, COMBAT_TARGET_GROUP_PARTY, &none_immune);

        assert_eq!(cause_fear, vec![ghost, skeleton, orc]);
        assert_eq!(repel, vec![ghost, skeleton]);
        assert!(
            repel.iter().all(|slot| cause_fear.contains(slot)),
            "Repel Undead is a narrowing of the same sweep, never a widening"
        );
        // `catalogs/monster-bestiary.md §6`: Ghost 23 and Skeleton 33 are the
        // two shipped undead rows; the bit lives in the per-class flag word.
        assert!(combat_class_traits(23).unwrap().undead);
        assert!(combat_class_traits(33).unwrap().undead);
        assert!(!combat_class_traits(32).unwrap().undead);
        assert!(combat_class_is_repel_undead_target(23));
        assert!(!combat_class_is_repel_undead_target(32));
    }

    #[test]
    fn the_cast_like_branch_replaces_melee_while_the_effect_prerequisite_is_active() {
        // `combat.md §11`: "One class trait can route an attack into a
        // cast-like ranged/effect branch, rather than ordinary melee, when the
        // combat effect prerequisite state is active. That branch prints the
        // cast/effect narration, reuses the AI direction/effect dispatch, plays
        // the ranged animation, resets the scene state, and consumes the
        // action." `catalogs/monster-bestiary.md §3` marks exactly one shipped
        // class, the Gremlin.
        //
        // Only the routing and the consumed action are pinned here. The
        // narration's wording, the ranged animation, and which scene word is
        // reset are all unpublished, so the branch performs none of those three
        // and this test asserts none of them.
        let gremlin = 25;
        assert!(combat_ranged_effect_stats(gremlin).unwrap().cast_like_branch);

        // Prerequisite clear: the ordinary melee arm.
        assert_eq!(
            resolve_combat_ai_attack_route_with_effect_prerequisite(gremlin, 1, false),
            Some(CombatAiAttackRoute::Melee)
        );
        // Prerequisite active: the cast-like branch replaces it.
        let selector = combat_ranged_effect_stats(gremlin)
            .unwrap()
            .range_effect_selector;
        let payload = combat_ranged_effect_stats(gremlin).unwrap().payload;
        assert_eq!(
            resolve_combat_ai_attack_route_with_effect_prerequisite(gremlin, 1, true),
            Some(CombatAiAttackRoute::CastLikeRangedEffect {
                range_effect_selector: selector,
                payload,
            })
        );
        // A class without the trait is unaffected in either state.
        assert_eq!(
            resolve_combat_ai_attack_route_with_effect_prerequisite(COMBAT_CLASS_GIANT_RAT, 1, true),
            Some(CombatAiAttackRoute::Melee)
        );

        // The dispatcher consumes the action through the branch.
        let mut state = combat_ai_turn_state(6, 5);
        state.combat_actors[8].owner_target_class = gremlin;
        state.combat_effect_prerequisite_active = true;

        let application = state
            .apply_combat_ai_turn_with_inputs(
                8,
                true,
                0,
                false,
                0,
                0,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[1, 2, 3, 4],
                Some(CombatMonsterAttackInputs::default()),
            )
            .unwrap();

        let cast_like = application
            .cast_like_ranged_effect
            .expect("the cast-like branch has a consumer");
        assert_eq!(cast_like.actor_slot, 8);
        assert_eq!(cast_like.target_slot, Some(0));
        assert_eq!(cast_like.range_effect_selector, selector);
        assert_eq!(cast_like.payload, payload);
        assert!(cast_like.action_consumed);
        assert_eq!(application.monster_attack, None);
        assert_eq!(application.movement, None);
        assert_eq!(application.command_key, Some(COMBAT_AI_ATTACK_COMMAND_KEY));
        // The branch does not clear the prerequisite: no document says which
        // scene word "resets the scene state" writes, and clearing this one
        // would silently make the branch one-shot.
        assert!(state.combat_effect_prerequisite_active);
    }

    #[test]
    fn the_swing_cue_runs_downwards_for_a_monster_and_upwards_for_a_party_melee_blow() {
        // `combat.md §11.1`, the "Swing begins" rows of the census:
        //   - "monster, melee and ranged | *nothing* | the swing sweep, played
        //     **before** the roll, running **downwards** (roughly 750 Hz
        //     toward 400 Hz)";
        //   - "party melee | **a newline, unconditionally, before the roll** |
        //     the same swing sweep in the opposite direction, roughly 400 Hz
        //     toward 750 Hz (`audio.md` section 7.4)";
        //   - "party ranged or thrown | *no newline here* | a descending
        //     sweep, roughly 1300 Hz toward 300 Hz, after `Aim! ` and a
        //     confirmed cursor".
        // `§11.1`'s evidence block makes the directions the established part:
        // "only the sweep **directions** were established in this pass, and
        // the monster swing runs opposite to the party's."
        let party_melee = audio::party_melee_attack_swing();
        assert_eq!(party_melee.tone_count(), audio::ATTACK_SWING_UPDATES);
        let rising = party_melee.frequencies();
        assert_eq!(rising[0], audio::ATTACK_SWING_LOW_HZ as u32);
        assert!(
            rising.windows(2).all(|pair| pair[1] > pair[0]),
            "the party melee sweep rises"
        );
        assert!(
            rising
                .iter()
                .all(|hz| *hz < audio::ATTACK_SWING_HIGH_HZ as u32),
            "`audio.md §5.2`: it stops strictly below 750 Hz"
        );
        assert!(party_melee.ends_with_stop());

        let monster_swing = audio::monster_attack_swing();
        assert_eq!(
            monster_swing.tone_count(),
            audio::ATTACK_SWING_UPDATES,
            "`§11.1`: the same swing sweep, in the opposite direction"
        );
        let falling = monster_swing.frequencies();
        assert_eq!(falling[0], audio::ATTACK_SWING_HIGH_HZ as u32);
        assert!(
            falling.windows(2).all(|pair| pair[1] < pair[0]),
            "the monster sweep runs downwards"
        );
        assert!(
            falling
                .iter()
                .all(|hz| *hz > audio::ATTACK_SWING_LOW_HZ as u32),
            "`audio.md §5.2`: it stops strictly above 400 Hz"
        );
        assert!(monster_swing.ends_with_stop());

        let party_ranged_program = audio::party_ranged_attack_swing();
        assert!(party_ranged_program.ends_with_stop());
        let party_ranged = party_ranged_program.frequencies();
        assert_eq!(party_ranged[0], 1300);
        assert!(
            party_ranged.windows(2).all(|pair| pair[1] < pair[0]),
            "the party ranged/thrown sweep descends"
        );
        assert_ne!(
            party_ranged, falling,
            "`§11.1` gives the ranged arm a cue of its own"
        );

        // `audio.md §7.4` keeps the cue "unconditional[], before the to-hit
        // roll", and the miss arm has "no audio call anywhere on it".
        for forced_hit in [true, false] {
            let mut state = combat_ai_turn_state(6, 5);
            state.party[0].hp = 30;
            state.party[0].max_hp = 30;
            let serial = state.sound_effect_serial;

            let application = state
                .resolve_and_apply_combat_monster_attack(8, 0, 7, 0, 0, false, 0, Some(forced_hit))
                .expect("an adjacent monster attack resolves");

            let landed = matches!(
                application.resolution,
                Some(CombatWeaponAttackResolution::Hit { .. })
            );
            assert_eq!(
                landed, forced_hit,
                "forced_hit {forced_hit} produced {:?}",
                application.resolution
            );
            let effects = state.sound_effects_after(serial);
            assert_eq!(
                effects.first(),
                Some(&SoundEffect::MonsterAttackSwing),
                "the monster swing precedes the branch on a forced_hit of {forced_hit}"
            );
            if !forced_hit {
                assert_eq!(
                    effects,
                    vec![SoundEffect::MonsterAttackSwing],
                    "the miss arm adds no audio call of its own"
                );
            }
        }

        // The party side of the same primitive, routed by
        // `resolve_combat_weapon_attack_range_route`. Item 16 is the one
        // catalogue row that reaches both arms: its published range cap is 3,
        // so distance 1 is the melee route, distance 3 the ranged route, and
        // distance 4 no route at all.
        assert_eq!(equipment_weapon_range_cap(16), Some(3));
        for (monster_x, expected) in [
            (6u8, Some(SoundEffect::PartyMeleeAttackSwing)),
            (8, Some(SoundEffect::PartyRangedAttackSwing)),
            (9, None),
        ] {
            let mut state = combat_ai_turn_state(monster_x, 5);
            let serial = state.sound_effect_serial;
            let application = state
                .resolve_and_apply_combat_equipment_weapon_attack(
                    16,
                    0,
                    8,
                    30,
                    10,
                    0,
                    5,
                    Some(false),
                    false,
                )
                .expect("the party attack resolves");
            let route = match application.resolution {
                CombatWeaponAttackResolution::OutOfRange { .. } => None,
                CombatWeaponAttackResolution::Miss { route, .. } => Some(route),
                other => panic!("unexpected resolution {other:?}"),
            };
            assert_eq!(
                route.is_some(),
                expected.is_some(),
                "monster_x {monster_x} routed to {route:?}"
            );
            assert_eq!(
                state.sound_effects_after(serial),
                expected.into_iter().collect::<Vec<_>>(),
                "monster_x {monster_x}: `§11.1` keys the party cue on the arm, \
                 and an attempt that reaches no attack application is silent"
            );
        }

        // `RETRACTIONS.md` R355, quoted in `audio.md §7.4`: when a self-acting
        // actor's ranged shot scatters and "the scattered cell turns out to
        // hold an actor, the ordinary hit chain runs against that actor with
        // its full narration and its own sounds. The ranged arm is silent only
        // when the scatter lands on nobody." `§11.1`'s swing row covers
        // "monster, melee and ranged" with the one downward sweep.
        //
        // The intended target sits at (5,5) and the attacker at (6,5), so a
        // scatter roll of `0` selects offset `(-1,-1)` - cell (4,4) - and a
        // roll of `1` selects `(0,-1)`, cell (5,4).
        let scatter_state = || {
            let mut state = combat_ai_turn_state(6, 5);
            state.party[0].status = b'G';
            state.party[0].hp = 30;
            state.party[0].max_hp = 30;
            state
        };

        // Lands on an actor: the ordinary hit chain, so the swing plays.
        let mut occupied = scatter_state();
        occupied.combat_actors[9] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            COMBAT_CLASS_GIANT_RAT,
            9,
            0,
            4,
            4,
        ]);
        let serial = occupied.sound_effect_serial;
        let application = occupied
            .resolve_and_apply_combat_monster_scattered_attack(8, 0, 7, 0, 0, 0)
            .expect("the scatter arm resolves");
        assert_eq!(
            application.target_slot, 9,
            "the scattered cell holds actor 9"
        );
        assert_eq!(
            occupied.sound_effects_after(serial).first(),
            Some(&SoundEffect::MonsterAttackSwing),
            "the scattered impact runs the ordinary hit chain with its own sounds"
        );

        // Lands on nobody: "the ranged arm is silent".
        let mut empty = scatter_state();
        let serial = empty.sound_effect_serial;
        let application = empty
            .resolve_and_apply_combat_monster_scattered_attack(8, 0, 7, 0, 0, 1)
            .expect("the scatter arm resolves");
        assert_eq!(application.damage_application, None);
        assert!(
            empty.sound_effects_after(serial).is_empty(),
            "a scatter that lands on nobody is silent"
        );
    }

    #[test]
    fn a_party_defender_reads_the_cached_combat_defense_byte_at_offset_0x18() {
        // `combat.md §12`: "For party-member defenders, the damage roll reads
        // the cached combat-defense byte in the character record at offset
        // `+0x18`; factory-seed records carry value `7`. This is not one of the
        // stat bytes earlier in the record - Strength `+0x0C`, Dexterity
        // `+0x0D`, Intelligence `+0x0E`."
        let mut state = combat_ai_turn_state(6, 5);
        state.party_combat_defense = vec![21];
        state.party_strengths = vec![3];
        state.party_intelligence = vec![5];

        assert_eq!(state.party_combat_defense_byte(0), 21);
        assert_eq!(
            state.combat_spell_target_defense_value(0),
            21,
            "the spell damage roll reads the cached byte, not the factory seed"
        );
        assert_ne!(state.combat_spell_target_defense_value(0), 3);
        assert_ne!(state.combat_spell_target_defense_value(0), 5);

        // A slot the roster does not carry falls back to the factory seed.
        assert_eq!(
            state.party_combat_defense_byte(5),
            CHARACTER_DEFENSE_FACTORY_SEED
        );
        assert_eq!(CHARACTER_DEFENSE_FACTORY_SEED, 7);

        // The byte comes off the save image at `+0x18`, not from a constant.
        let mut bytes = vec![0u8; SAVE_ROSTER_OFFSET + 3 * SAVE_CHARACTER_RECORD_LEN];
        for slot in 0..3usize {
            bytes[SAVE_ROSTER_OFFSET
                + slot * SAVE_CHARACTER_RECORD_LEN
                + SAVE_CHARACTER_DEFENSE_BYTE_OFFSET] = 11 + slot as u8;
        }
        assert_eq!(decode_party_combat_defense(&bytes, 3), vec![11, 12, 13]);
        assert_eq!(SAVE_CHARACTER_DEFENSE_BYTE_OFFSET, 0x18);
    }

    #[test]
    fn a_gazer_gaze_takes_the_stoning_branch_against_an_awake_defender() {
        // `combat.md §12`: "Gazer attacks have a separate stoning-style effect
        // against awake defenders, and magic/effect attack tiles can also enter
        // the same poison or stoning-style branches before falling back to
        // ordinary damage."
        //
        // The branch's payload is not published anywhere - no status letter,
        // no HP change, no tile, no message, no sound - and the sentence puts
        // it "before falling back to ordinary damage", so this pins the
        // routing and the awake gate only, and asserts that the ordinary
        // damage path still runs. An ungated payload-free branch that
        // swallowed the attack would claim more than the spec does.
        assert!(combat_class_gaze_stones(COMBAT_CLASS_GAZER));
        assert!(!combat_class_gaze_stones(COMBAT_CLASS_GIANT_RAT));

        let mut awake = combat_ai_turn_state(6, 5);
        awake.combat_actors[8].owner_target_class = COMBAT_CLASS_GAZER;
        awake.party[0].status = b'G';
        awake.party[0].hp = 30;
        awake.party[0].max_hp = 30;

        let application = awake
            .resolve_and_apply_combat_monster_attack(8, 0, 7, 0, 0, false, 0, Some(true))
            .expect("the gaze resolves");
        assert_eq!(
            application.stoning,
            Some(CombatStoningEffectOutcome {
                source: CombatStoningEffectSource::GazerGaze,
                target_slot: 0,
            })
        );
        assert!(
            matches!(
                application.resolution,
                Some(CombatWeaponAttackResolution::Hit { .. })
            ),
            "the recorded branch still falls back to ordinary damage: {:?}",
            application.resolution
        );
        assert!(application.damage_application.is_some());
        assert!(awake.party[0].hp < 30);

        // "against awake defenders": an asleep defender does not enter the
        // branch at all, and takes exactly the same ordinary damage path.
        // `§6.1`: "Combat sleep for non-party targets stores into this
        // bit; party sleep uses the character status byte `'S'` instead", so a
        // party defender's sleep is the roster letter.
        let mut asleep = combat_ai_turn_state(6, 5);
        asleep.combat_actors[8].owner_target_class = COMBAT_CLASS_GAZER;
        asleep.party[0].status = b'S';
        asleep.party[0].hp = 30;
        asleep.party[0].max_hp = 30;

        let application = asleep
            .resolve_and_apply_combat_monster_attack(8, 0, 7, 0, 0, false, 0, Some(true))
            .expect("the gaze resolves");
        assert_eq!(application.stoning, None);
        assert!(
            application.resolution.is_some(),
            "an asleep defender takes the ordinary damage path"
        );
        assert!(asleep.party[0].hp < 30);

        // The awake gate itself, stated directly.
        let live = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        let mut disabled = live;
        disabled.set_status_disabled();
        assert!(combat_stoning_effect_defender_is_awake(live, None));
        assert!(!combat_stoning_effect_defender_is_awake(disabled, None));
    }

    #[test]
    fn the_active_effect_counter_ages_on_the_action_tail_and_not_on_the_round_wrap() {
        // `combat.md §12`: the shared active-effect counter's "other values
        // decrement when the committed non-digit action tail runs" and it "is
        // not the time system's torch/light-spell counter; do not model it as
        // one decrement per minute or per full actor-table sweep."
        let game_dir = std::path::Path::new(".");

        // The round counter's wrap advances the turn clock; that must not age
        // the tag a second time.
        let mut wrap = combat_player_command_state(10, 10);
        wrap.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        wrap.active_effect_counter = 9;
        wrap.combat_round_counter = COMBAT_ROUND_COUNTER_WRAP - 1;
        let tick = wrap.advance_combat_round_counter();
        assert!(tick.wrapped);
        assert_ne!(tick.advance_time_minutes, 0);
        assert_eq!(
            wrap.active_effect_counter, 9,
            "the wrap's turn-clock advance is not an ageing site"
        );

        // One committed non-digit action ages it exactly once.
        let mut action = combat_player_command_state(10, 10);
        action.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
        action.active_effect_counter = 9;
        action.combat_round_counter = COMBAT_ROUND_COUNTER_WRAP - 1;
        handle_play_key_input(&mut action, ' ', "", game_dir).unwrap();
        assert_eq!(
            action.active_effect_counter, 8,
            "one committed action, one decrement - even on the round that wraps"
        );
    }

    #[test]
    fn every_class_in_the_forty_eight_row_space_has_a_flag_word() {
        // `combat.md §13`: the per-class flag word is "Sixteen bits per class"
        // over the same class space as the eight-byte stat record, and "The
        // 48-row stat table boundary is part of the public combat contract:
        // party combat classes, special NPC classes, and monsters share the
        // same eight-byte row shape."
        for class in 0u8..48 {
            assert!(
                combat_class_stats(class).is_some(),
                "class {class} has a stat row"
            );
            assert!(
                combat_class_traits(class).is_some(),
                "class {class} has a flag word"
            );
        }
        assert_eq!(combat_class_traits(48), None);

        // The rows `catalogs/monster-bestiary.md §4` confirms no trait for are
        // all-zero words, not absent ones.
        for class in 1u8..12 {
            let traits = combat_class_traits(class).unwrap();
            assert_eq!(
                traits,
                traits_without_identity(class, traits.name),
                "class {class} carries an all-zero flag word"
            );
        }

        // A class reachable through the `encounters.md §4` ship/pirate family
        // therefore carries both published rows and can still be damaged.
        // `catalogs/monster-bestiary.md §3` publishes no ranged/effect row for
        // it, and nothing published says what such a row would hold, so the
        // engine keeps propagating the absence rather than substituting
        // invented side-table bytes - see the spec question recorded with this
        // change.
        let mut state = combat_ai_turn_state(6, 5);
        state.combat_actors[8].owner_target_class = OUTDOOR_PIRATE_COMBAT_CLASS;
        state.combat_actors[8].hp_or_wound = 20;
        state.party[0].hp = 30;
        state.party[0].max_hp = 30;

        assert!(combat_class_stats(OUTDOOR_PIRATE_COMBAT_CLASS).is_some());
        assert!(combat_class_traits(OUTDOOR_PIRATE_COMBAT_CLASS).is_some());
        assert_eq!(combat_ranged_effect_stats(OUTDOOR_PIRATE_COMBAT_CLASS), None);
        assert!(
            state
                .apply_combat_weapon_damage_to_target(None, 8, 5, false)
                .is_some(),
            "a pirate-family monster can still be damaged"
        );
    }

    #[test]
    fn surface_tile_one_reaches_its_special_only_after_the_low_tile_allowance_die() {
        // `encounters.md §4`: the tile-1 row is "Surface tile 1 **after the
        // low-tile allowance gate**", and the low-tile family runs "an extra
        // allowance die before any bucket selection; a failed die rejects. The
        // die is a draw over the closed interval `[0, 64]`, inclusive, accepted
        // when the result is below sixteen".
        assert_eq!(
            spawn_terrain_branch(0x01, false),
            SpawnTerrainBranch::LowTileAllowance
        );

        let mut rejected = 0usize;
        let mut allowed = 0usize;
        for seed in (0u16..4000).step_by(37) {
            let mut state = world_state(open_world_grid(), 10, 20);
            state.prng_state = seed;

            let mut probe = seed;
            let allowance =
                u5_prng_range_u16(&mut probe, 0, u16::from(SPAWN_LOW_TILE_ALLOWANCE_DRAW_HIGH))
                    as u8;
            let accepts = spawn_low_tile_allowance_accepts(allowance);

            let picked = state.native_world_encounter_type(WorldPlane::Britannia, 0x01, 0);

            if accepts {
                allowed += 1;
                assert!(
                    picked.is_some(),
                    "seed {seed:#06x}: an allowed tile-1 candidate still spawns"
                );
            } else {
                rejected += 1;
                assert_eq!(
                    picked, None,
                    "seed {seed:#06x}: a failed allowance die rejects tile 1"
                );
                assert_eq!(
                    state.prng_state, probe,
                    "seed {seed:#06x}: a rejected candidate spends only the allowance draw"
                );
            }
        }
        assert!(
            rejected > 0 && allowed > 0,
            "both arms of the allowance die must be exercised"
        );
    }

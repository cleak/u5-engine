// `systems/audio.md` trigger-boundary regressions: spells combat.
//
// Each test names the published clause it pins. Add tests here rather
// than to the numbered chunks so the audio work stays reviewable as a
// unit.

    fn spells_combat_audio_state(spell_index: usize) -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 8,
            hp: 30,
            max_hp: 30,
            level: 8,
        }];
        state.party_intelligence = vec![31];
        state.prng_state = 0x1234;
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.active_objects[0] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        state
    }

    fn spells_combat_audio_hostile_target(state: &mut PlayState, class: u8) -> usize {
        let slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[slot] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            class,
            slot as u8,
            0,
            7,
            7,
        ]);
        state.active_objects[slot] = ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: 7,
            y: 7,
            ..ActiveObject::empty()
        };
        slot
    }

    #[test]
    fn committed_combat_spell_pre_effects_use_their_published_shared_variant() {
        // `audio.md §6` spell groupings, plus `audio.md §8.3`: "For combat cursor
        // spells, confirmation plays the spell effect before the
        // coordinate/projectile-impact resolver." Each cast therefore opens with
        // its variant regardless of how the resolver then lands.

        // Conjure: variant 2.
        let conjure = spell_index_from_code("KX").unwrap();
        let mut state = spells_combat_audio_state(conjure);
        let serial = state.sound_effect_serial;
        let _ = state.cast_combat_conjure_spell(0, conjure);
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::SharedVariant { variant: 2 }),
            "Conjure",
        );

        // Dispel Field: variant 4.
        let mut state = spells_combat_audio_state(DISPEL_FIELD_SPELL_INDEX);
        let serial = state.sound_effect_serial;
        let _ = state.cast_combat_dispel_field(0, Some(Direction::North));
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::SharedVariant { variant: 4 }),
            "Dispel Field",
        );

        // Swarm: variant 5.
        let swarm = spell_index_from_code("BIX").unwrap();
        let mut state = spells_combat_audio_state(swarm);
        let serial = state.sound_effect_serial;
        let _ = state.cast_combat_swarm_spell(0, swarm);
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::SharedVariant { variant: 5 }),
            "Swarm",
        );

        // Charm: variant 6.
        let charm = spell_index_from_code("AEX").unwrap();
        let mut state = spells_combat_audio_state(charm);
        let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
        let serial = state.sound_effect_serial;
        let _ = state.cast_combat_charm_spell(0, charm, target);
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::SharedVariant { variant: 6 }),
            "Charm",
        );

        // Polymorph: variant 6.
        let polymorph = spell_index_from_code("BRX").unwrap();
        let mut state = spells_combat_audio_state(polymorph);
        let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
        let serial = state.sound_effect_serial;
        let _ = state.cast_combat_polymorph_spell(0, polymorph, target);
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::SharedVariant { variant: 6 }),
            "Polymorph",
        );

        // Kill: **no** shared variant. `RETRACTIONS.md` withdraws the earlier
        // "variant 6's confirmed uses include Kill/Slay Living": Kill is a
        // circle-7 spell that "plays no dispatcher variant at all" and takes
        // the combat effect template instead - the circle-scaled rumble lead.
        let kill = spell_index_from_code("CX").unwrap();
        assert_eq!(kill, 37);
        let mut state = spells_combat_audio_state(kill);
        let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
        let serial = state.sound_effect_serial;
        let _ = state.cast_active_target_combat_spell(0, kill, CombatSpellDamageKind::Kill, target);
        let kill_effects = state.sound_effects_after(serial);
        assert_eq!(
            kill_effects.first(),
            Some(&SoundEffect::CircleRumbleLead { circle: 7 }),
            "Kill",
        );
        assert!(
            !kill_effects
                .iter()
                .any(|effect| matches!(effect, SoundEffect::SharedVariant { .. })),
            "Kill must reach no shared variant on any path: {kill_effects:?}",
        );

        // Creature clone: variant 7.
        let clone = spell_index_from_code("IQX").unwrap();
        let mut state = spells_combat_audio_state(clone);
        let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
        let serial = state.sound_effect_serial;
        let _ = state.cast_combat_clone_spell(0, clone, target);
        assert_eq!(
            state.sound_effects_after(serial).first(),
            Some(&SoundEffect::SharedVariant { variant: 7 }),
            "creature clone",
        );
    }

    #[test]
    fn combat_spell_failure_tail_follows_the_committed_pre_effect() {
        // `audio.md §8.3`: "Common spell failure tail | After `Failed!`, play the
        // 50-update 800-to-2000 Hz cast-failure glissando." A post-commit failure
        // keeps the pre-effect and appends the tail, in that order.

        // Dispel Field into an empty cell: no field is removed.
        let mut state = spells_combat_audio_state(DISPEL_FIELD_SPELL_INDEX);
        let serial = state.sound_effect_serial;
        assert_eq!(
            state.cast_combat_dispel_field(0, Some(Direction::North)),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Failed!");
        assert_eq!(
            state.sound_effects_after(serial),
            vec![
                SoundEffect::SharedVariant { variant: 4 },
                SoundEffect::CastFailure,
            ]
        );

        // Kill against a protected special class: rejected after the resource gate
        // and the pre-effect, without consuming gameplay randomness.
        let kill = spell_index_from_code("CX").unwrap();
        let mut state = spells_combat_audio_state(kill);
        let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_BLACKTHORN);
        let prng_before = state.prng_state;
        let serial = state.sound_effect_serial;
        assert_eq!(
            state.cast_active_target_combat_spell(0, kill, CombatSpellDamageKind::Kill, target),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Failed!");
        assert_eq!(state.prng_state, prng_before);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![
                SoundEffect::CircleRumbleLead { circle: 7 },
                SoundEffect::CastFailure,
            ],
            "Kill opens with its template lead, not a shared variant",
        );
    }

    #[test]
    fn the_combat_effect_template_spells_reach_no_shared_variant() {
        // `audio.md §6.1` second table: Magic Missile (1), Fireball (13) and
        // Kill (37) "do not reach the dispatcher on any path" and play the
        // combat effect template - the circle-scaled rumble lead, plus "on a
        // resolved effect ... a **descending** glissando, 20 updates from
        // 1300 Hz down toward 350 Hz".
        for (code, kind, circle) in [
            ("GP", CombatSpellDamageKind::MagicMissile, 1u8),
            ("FV", CombatSpellDamageKind::Fireball, 3),
            ("CX", CombatSpellDamageKind::Kill, 7),
        ] {
            let spell = spell_index_from_code(code).unwrap();
            assert_eq!(crate::audio::spell_circle(spell), circle, "{code}");
            let mut state = spells_combat_audio_state(spell);
            let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
            let serial = state.sound_effect_serial;
            let outcome = state.cast_active_target_combat_spell(0, spell, kind, target);
            let effects = state.sound_effects_after(serial);
            assert_eq!(
                effects.first(),
                Some(&SoundEffect::CircleRumbleLead { circle }),
                "{code} opens with its own circle's rumble lead",
            );
            assert!(
                !effects
                    .iter()
                    .any(|effect| matches!(effect, SoundEffect::SharedVariant { .. })),
                "{code} must play no shared variant: {effects:?}",
            );
            if outcome == MoveOutcome::Cast {
                assert!(
                    effects.contains(&SoundEffect::CombatTemplateImpact),
                    "{code} resolved, so the descending impact glissando follows: {effects:?}",
                );
            }
        }
    }

    #[test]
    fn the_mass_target_family_plays_one_bare_circle_rumble_and_no_variant() {
        // `audio.md §6.1`: "No dispatcher call. Instead: one bare random rumble
        // `(800, T, 700)`" with T = 8000 + 1600 x circle. Death Wind (44) and
        // Flame Wind (45) are circle 8, so T = 20800 for both.
        for (code, effect, circle) in [
            ("CGIV", CombatDirectedSpellEffect::DeathWind, 8u8),
            ("FHI", CombatDirectedSpellEffect::FlameWind, 8),
        ] {
            let spell = spell_index_from_code(code).unwrap();
            assert_eq!(crate::audio::spell_circle(spell), circle, "{code}");
            assert_eq!(crate::audio::spell_shared_variant(spell), None, "{code}");
            let mut state = spells_combat_audio_state(spell);
            let _ = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
            let serial = state.sound_effect_serial;
            let _ = state.cast_directed_combat_spell(0, spell, effect, Some(Direction::North));
            let effects = state.sound_effects_after(serial);
            assert_eq!(
                effects.first(),
                Some(&SoundEffect::CircleRumbleLead { circle }),
                "{code}",
            );
            assert!(
                !effects
                    .iter()
                    .any(|effect| matches!(effect, SoundEffect::SharedVariant { .. })),
                "{code} must play no shared variant: {effects:?}",
            );
        }
    }

    #[test]
    fn pre_commit_combat_spell_rejections_stay_silent() {
        // `audio.md §8.3`: "The ordinary spell presentation is committed only after
        // the spell's own input gate accepts. A direction or combat-cursor
        // cancellation that the spell spec places before its sound skips the shared
        // variant."

        // Scene rejection, before the resource gate.
        let charm = spell_index_from_code("AEX").unwrap();
        let mut state = spells_combat_audio_state(charm);
        let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
        state.combat_active = false;
        let serial = state.sound_effect_serial;
        assert_eq!(
            state.cast_combat_charm_spell(0, charm, target),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Not here!");
        assert!(state.sound_effects_after(serial).is_empty());

        // Combat-cursor rejection: the target slot holds no live combatant.
        let kill = spell_index_from_code("CX").unwrap();
        let mut state = spells_combat_audio_state(kill);
        let serial = state.sound_effect_serial;
        assert_eq!(
            state.cast_active_target_combat_spell(
                0,
                kill,
                CombatSpellDamageKind::Kill,
                COMBAT_PARTY_ACTOR_SLOTS,
            ),
            MoveOutcome::Blocked
        );
        assert!(state.message.starts_with("Target?"));
        assert!(state.sound_effects_after(serial).is_empty());

        // Resource-gate rejection: no charge left to spend.
        let conjure = spell_index_from_code("KX").unwrap();
        let mut state = spells_combat_audio_state(conjure);
        state.spell_charges[conjure] = 0;
        let serial = state.sound_effect_serial;
        let _ = state.cast_combat_conjure_spell(0, conjure);
        assert_ne!(state.message, "Failed!");
        assert!(state.sound_effects_after(serial).is_empty());
    }

    #[test]
    fn player_summon_runs_its_envelope_only_on_an_accepted_placement() {
        // `audio.md §8.3` Player Summon: "The committed cast uses its shared spell
        // variant. An accepted placement additionally runs (5, 500, 12000, 1, 2760)
        // before actor finalization." `§6.1` now names Summon (id 43, circle 8):
        // "Unconditional at placement-helper entry, before the eight-try cell
        // probe, so a failed placement still plays it."
        let summon = spell_index_from_code("CKX").unwrap();
        assert_eq!(summon, 43);

        let mut state = spells_combat_audio_state(summon);
        state.prng_state = 0;
        let serial = state.sound_effect_serial;
        let outcome = state.cast_combat_summon_daemon_spell(0, summon);
        assert!(matches!(outcome, MoveOutcome::Cast | MoveOutcome::Blocked));
        assert!(!state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_empty());
        assert_eq!(
            state.sound_effects_after(serial),
            vec![
                SoundEffect::SharedVariant { variant: 8 },
                SoundEffect::PlayerSummon,
            ]
        );

        // A failed placement keeps the variant - it precedes the probe - and
        // takes the common failure tail instead of the envelope.
        let mut state = spells_combat_audio_state(summon);
        state.combat_terrain = [[0x00; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        let serial = state.sound_effect_serial;
        assert_eq!(
            state.cast_combat_summon_daemon_spell(0, summon),
            MoveOutcome::Blocked
        );
        assert_eq!(state.message, "Failed!");
        assert_eq!(
            state.sound_effects_after(serial),
            vec![
                SoundEffect::SharedVariant { variant: 8 },
                SoundEffect::CastFailure,
            ],
            "the variant fires before the eight-try cell probe"
        );
    }

    #[test]
    fn monster_summon_and_possession_emit_their_success_envelopes_after_narration() {
        // `audio.md §8.3`: possession is "After possession narration, run software
        // envelope ..."; summon is "After successful placement and narration, run
        // ..., then perform the summon tile flash." Both are success-only.

        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = COMBAT_CLASS_DAEMON;
        let serial = state.sound_effect_serial;
        let _ = state
            .apply_combat_ai_possess_special_with_inputs(8, 0, false)
            .unwrap();
        assert_eq!(state.message, "Monster possessed party member 1.");
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::Possession]
        );

        // "Resistance skips this success envelope."
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[8].owner_target_class = 28;
        let serial = state.sound_effect_serial;
        let _ = state
            .apply_combat_ai_possess_special_with_inputs(8, 0, true)
            .unwrap();
        assert_eq!(state.message, "Possession resisted.");
        assert!(state.sound_effects_after(serial).is_empty());

        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[4][6] = 0x04;
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        state.combat_actors[actor_slot] = CombatActorDescriptor::from_row([
            99,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DRAGON,
            actor_slot as u8,
            0,
            5,
            5,
        ]);
        state.active_objects[actor_slot] = ActiveObject {
            type_byte: 0xdc,
            tile: 0xdc,
            x: 5,
            y: 5,
            z: -1,
            ..ActiveObject::empty()
        };
        let serial = state.sound_effect_serial;
        let _ = state
            .apply_combat_ai_summon_daemon_special_with_candidates(actor_slot, &[(4, 4), (6, 4)])
            .unwrap();
        assert_eq!(state.message, "Monster summons daemon.");
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::MonsterSummon]
        );

        // "Failed chance, coordinate, legality, or allocation gates are silent."
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.combat_actors[actor_slot] = CombatActorDescriptor::from_row([
            99,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            COMBAT_CLASS_DRAGON,
            actor_slot as u8,
            0,
            5,
            5,
        ]);
        let serial = state.sound_effect_serial;
        assert_eq!(
            state.apply_combat_ai_summon_daemon_special_with_candidates(actor_slot, &[(6, 5)]),
            None
        );
        assert!(state.sound_effects_after(serial).is_empty());
    }

    #[test]
    fn combat_spells_the_spec_never_names_emit_no_shared_variant() {
        // `audio.md §6`'s spell table says "Confirmed groupings include" and never
        // names Magic Missile, Fireball, or the directed winds. Emitting a borrowed
        // variant for them would invent a cue, so only the published failure tail
        // may sound on those routes.

        let missile = spell_index_from_code("GP").unwrap();
        let mut state = spells_combat_audio_state(missile);
        let target = spells_combat_audio_hostile_target(&mut state, COMBAT_CLASS_GIANT_RAT);
        let serial = state.sound_effect_serial;
        let _ = state.cast_active_target_combat_spell(
            0,
            missile,
            CombatSpellDamageKind::MagicMissile,
            target,
        );
        assert!(
            !state
                .sound_effects_after(serial)
                .iter()
                .any(|effect| matches!(effect, SoundEffect::SharedVariant { .. })),
            "Magic Missile has no published shared variant",
        );

        let sleep = spell_index_from_code("IZ").unwrap();
        let mut state = spells_combat_audio_state(sleep);
        let serial = state.sound_effect_serial;
        let _ = state.cast_directed_combat_spell(
            0,
            sleep,
            CombatDirectedSpellEffect::Sleep,
            Some(Direction::North),
        );
        assert!(
            !state
                .sound_effects_after(serial)
                .iter()
                .any(|effect| matches!(effect, SoundEffect::SharedVariant { .. })),
            "Sleep has no published shared variant",
        );
    }

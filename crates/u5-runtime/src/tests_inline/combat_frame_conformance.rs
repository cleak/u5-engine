    // `systems/combat.md` conformance regressions for `combat_frame.rs`.

    fn combat_frame_conformance_state() -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state
            .active_objects
            .resize(COMBAT_ACTOR_SLOTS, ActiveObject::empty());
        state
    }

    #[test]
    fn gazer_death_places_a_live_insect_swarm_at_the_death_cell() {
        // `combat.md §6.3` "The Gazer death spawns a real combatant, not a
        // cosmetic effect": after writing `0x1F` into its own record the Gazer
        // branch calls the ordinary monster-placement primitive with class id
        // 31 and the dying Gazer's arena coordinates and Z plane. The engine
        // previously stamped the marker and stopped, silently dropping a
        // combatant.
        let mut state = combat_frame_conformance_state();
        let gazer_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let gazer_object_slot = 12;
        place_death_side_effect_monster(&mut state, 28, gazer_slot, gazer_object_slot);
        state.active_objects[gazer_object_slot].z = 3;

        state
            .apply_combat_weapon_damage_to_target(
                None,
                gazer_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();

        // The Gazer keeps its own `0x1F` marker and its slot.
        assert!(state.combat_actors[gazer_slot].is_marked_dead());
        assert_eq!(
            state.active_objects[gazer_object_slot].tile,
            COMBAT_GAZER_DEATH_MARKER_TILE
        );

        let swarm_slot = (COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS)
            .find(|slot| {
                state.combat_actors[*slot].owner_target_class == COMBAT_CLASS_INSECT_SWARM
                    && !state.combat_actors[*slot].is_empty()
            })
            .expect("the Gazer death must place a live Insect Swarm combatant");
        let swarm = state.combat_actors[swarm_slot];
        let stats = combat_class_stats(COMBAT_CLASS_INSECT_SWARM).unwrap();
        assert_eq!(stats.max_hp, 5);
        assert_eq!(stats.speed_seed, 30);
        assert_eq!(swarm.hp_or_wound, stats.max_hp);
        assert_eq!((swarm.x, swarm.y), (4, 5));
        assert!(!swarm.is_marked_dead());
        // Hostile faction tag, never the controlled/summoned flags.
        assert_eq!(swarm.flags, COMBAT_ACTOR_FLAG_SELECTABLE_80);
        assert!(!swarm.is_controlled());
        // Base-step is the speed seed under the `[-4, +3]` adjustment, and the
        // phase counter is thirty-six minus that base-step.
        assert!((26..=30).contains(&swarm.base_step));
        assert_eq!(
            swarm.phase_counter,
            COMBAT_PLACEMENT_PHASE_BASE - swarm.base_step
        );

        // A new active-object record with the Insect Swarm sprite run base.
        let swarm_object = state.active_objects[usize::from(swarm.active_object_slot)];
        assert_eq!(swarm_object.type_byte, 0xBC);
        assert_eq!(swarm_object.tile, 0xBC);
        assert_eq!((swarm_object.x, swarm_object.y), (4, 5));
        assert_eq!(swarm_object.z, 3);
    }

    #[test]
    fn gazer_death_spawn_is_skipped_with_no_side_effect_when_the_table_is_full() {
        // `combat.md §6.3`: "The spawn is skipped with no other side effect
        // when the arena has no free descriptor (all thirty-two allocated) or
        // no free active-object record."
        let mut state = combat_frame_conformance_state();
        let gazer_slot = COMBAT_PARTY_ACTOR_SLOTS;
        place_death_side_effect_monster(&mut state, 28, gazer_slot, 12);
        for slot in COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS {
            if slot != gazer_slot {
                state.combat_actors[slot] = CombatActorDescriptor::from_row([
                    10,
                    1,
                    COMBAT_ACTOR_FLAG_SELECTABLE_80,
                    COMBAT_CLASS_GIANT_RAT,
                    slot as u8,
                    0,
                    1,
                    1,
                ]);
            }
        }
        let prng_before = state.prng_state;

        state
            .apply_combat_weapon_damage_to_target(
                None,
                gazer_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();

        assert_eq!(
            state.prng_state, prng_before,
            "a failed allocation must not draw the placement adjustment"
        );
        assert!(
            !state
                .combat_actors
                .iter()
                .any(|actor| actor.owner_target_class == COMBAT_CLASS_INSECT_SWARM),
            "no swarm may be placed when the descriptor table is full"
        );
    }

    #[test]
    fn default_death_drop_rolls_use_the_shared_skewed_one_to_thirty_helper() {
        // `combat.md §6.3` "Both rolls use the same helper": the underlying
        // draw is a uniform `0..60` halved with truncation and a zero result
        // promoted to one, not a `0..99` percentage roll. The engine's earlier
        // `COMBAT_DEFAULT_DEATH_DROP_ROLL_MAX = 99` reading made every
        // non-zero drop cap fire far too rarely.
        let mut state = combat_frame_conformance_state();
        let mut seen_min = u8::MAX;
        let mut seen_max = u8::MIN;
        for seed in 0..512u16 {
            state.prng_state = seed;
            let (first, second) = state.combat_default_death_drop_rolls();
            for roll in [first, second] {
                assert!(
                    (1..=30).contains(&roll),
                    "drop rolls live in 1..30, got {roll}"
                );
                seen_min = seen_min.min(roll);
                seen_max = seen_max.max(roll);
            }
        }
        assert!(
            seen_min <= 3 && seen_max >= 27,
            "the shared helper spans the 1..30 band, saw {seen_min}..{seen_max}"
        );
        assert_eq!(combat_skewed_roll_1_to_30(0), 1);
        assert_eq!(combat_skewed_roll_1_to_30(1), 1);
        assert_eq!(combat_skewed_roll_1_to_30(60), 30);
    }

    #[test]
    fn default_death_first_drop_gate_accepts_a_roll_equal_to_the_cap() {
        // `combat.md §6.3`: "the first roll is less than or equal to the class
        // drop-cap byte" accepts; the second needs to be strictly below it.
        let mut state = combat_frame_conformance_state();
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 9;
        let stats = place_death_side_effect_monster(&mut state, 32, actor_slot, active_object_slot);
        // Orc's drop cap is non-zero, so the boundary is observable.
        assert_ne!(stats.default_drop_cap, 0);
        state.prng_state = seed_for_default_death_gates(stats.default_drop_cap, true, false);

        state
            .apply_combat_weapon_damage_to_target(
                None,
                actor_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();

        assert_eq!(
            state.active_objects[active_object_slot].tile,
            COMBAT_DEFAULT_DEATH_DROP_TILE
        );
        // Byte five stores the class drop-cap value itself, not a random one.
        assert_eq!(
            state.active_objects[active_object_slot].aux1,
            stats.default_drop_cap
        );
    }

    #[test]
    fn vanish_on_death_narrates_the_vanishes_line_before_releasing_the_slot() {
        // `combat.md §6.3` vanish row + `§12`: the branch prints
        // `<name> vanishes!`, writes the `0x16` marker, and releases the slot.
        // The engine previously produced a silent tile swap.
        let mut state = combat_frame_conformance_state();
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 11;
        place_death_side_effect_monster(&mut state, 13, actor_slot, active_object_slot);

        state
            .apply_combat_weapon_damage_to_target(
                None,
                actor_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();

        assert_eq!(state.message, "Wanderer vanishes!");
        assert!(state.combat_actors[actor_slot].is_free_for_allocation());
        assert_eq!(
            state.active_objects[active_object_slot].tile,
            0
        );
    }

    #[test]
    fn split_class_damaged_but_not_killed_divides_into_a_free_slot() {
        // `combat.md §12` "Splitting / replicating monsters": a split-flagged
        // class damaged but not killed copies its class byte into an empty
        // slot and prints `<monster name> divides!`. The resolver existed but
        // had no caller, so slimes and gargoyles never divided.
        let mut state = combat_frame_conformance_state();
        let parent_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let stats = place_death_side_effect_monster(&mut state, 24, parent_slot, 10);
        assert!(combat_class_traits(24).unwrap().splits);
        assert!(stats.max_hp > 2);

        state
            .apply_combat_weapon_damage_to_target(None, parent_slot, 1, false)
            .unwrap();

        assert_eq!(state.message, "Slime divides!");
        assert!(!state.combat_actors[parent_slot].is_marked_dead());
        let child_slot = (COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS)
            .find(|slot| *slot != parent_slot && !state.combat_actors[*slot].is_empty())
            .expect("a damaged slime must divide into a free slot");
        let child = state.combat_actors[child_slot];
        assert_eq!(child.owner_target_class, 24);
        assert_eq!(child.hp_or_wound, stats.max_hp);
        assert_eq!(child.flags, COMBAT_ACTOR_FLAG_SELECTABLE_80);
        assert_eq!(
            (child.x, child.y),
            (
                state.combat_actors[parent_slot].x,
                state.combat_actors[parent_slot].y
            )
        );
    }

    #[test]
    fn a_killed_split_class_does_not_divide() {
        // The resolver's own gate: `applied_damage != 0 && !killed`.
        let mut state = combat_frame_conformance_state();
        let parent_slot = COMBAT_PARTY_ACTOR_SLOTS;
        place_death_side_effect_monster(&mut state, 24, parent_slot, 10);

        state
            .apply_combat_weapon_damage_to_target(
                None,
                parent_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();

        assert_ne!(state.message, "Slime divides!");
        assert!(
            !(COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS)
                .any(|slot| slot != parent_slot && !state.combat_actors[slot].is_empty()),
            "a killed slime must not divide"
        );
    }

    #[test]
    fn round_walker_sends_a_controlled_party_slot_to_the_automatic_driver() {
        // `combat.md §6.1a` "A dispatch input": the walker dispatches through
        // the slot-to-group helper. A party-side actor carrying bit `0x01`
        // groups with the monsters and therefore takes its turn through the
        // automatic actor driver, not the player's prompt. The reading that
        // any slot with the bit set goes to the player command parser is
        // expressly withdrawn.
        let mut state = combat_ai_turn_state(8, 5);
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        assert_eq!(
            state.combat_target_group_for_slot(0),
            COMBAT_TARGET_GROUP_MONSTER
        );

        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            0, 30, false, false, 0, false, 1, 1, &[], None, 0, false, None, true, &[1, 2, 3, 4],
            &[],
        );
        let CombatActorSlotDispatchApplication::Slot { action, .. } = application else {
            panic!("a live party slot should produce a slot dispatch");
        };
        assert_ne!(
            action,
            CombatActorDispatchAction::PlayerReady,
            "a charmed party member must not answer to the player's keyboard"
        );

        // An uncontrolled party slot still reaches the player prompt, and a
        // controlled monster still lands in the party's group.
        let mut clear = combat_ai_turn_state(8, 5);
        let application = clear.apply_combat_actor_slot_dispatch_with_inputs(
            0, 30, false, false, 0, false, 1, 1, &[], None, 0, false, None, true, &[1, 2, 3, 4],
            &[],
        );
        if let CombatActorSlotDispatchApplication::Slot { action, .. } = application {
            assert_eq!(action, CombatActorDispatchAction::PlayerReady);
        } else {
            panic!("party slot should produce a slot dispatch");
        }

        let mut charmed_monster = combat_ai_turn_state(8, 5);
        charmed_monster.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        assert_eq!(
            charmed_monster.combat_target_group_for_slot(8),
            COMBAT_TARGET_GROUP_PARTY
        );
    }

    #[test]
    fn controlled_actor_with_a_non_adjacent_target_takes_no_action() {
        // `combat.md §6.1a` "Readers — the attack driver": when the chosen
        // target is further than straight-line distance one the controlled
        // actor's turn produces no action at all — no ranged fallthrough, no
        // max-range consult, and no step.
        let mut state = combat_ai_turn_state(9, 5);
        state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        let position_before = (state.combat_actors[8].x, state.combat_actors[8].y);

        let turn = state
            .apply_combat_ai_turn_with_inputs(
                8,
                false,
                0,
                false,
                255,
                255,
                &[],
                None,
                0,
                false,
                None,
                true,
                &[1, 2, 3, 4],
                Some(CombatMonsterAttackInputs::default()),
            )
            .expect("a controlled monster still takes a dispatch");

        assert!(turn.attack_route.is_none());
        assert!(turn.monster_attack.is_none());
        assert!(turn.movement.is_none());
        assert!(turn.movement_commit.is_none());
        assert_eq!(
            (state.combat_actors[8].x, state.combat_actors[8].y),
            position_before,
            "a controlled actor with no adjacent target must not step"
        );
    }

    #[test]
    fn controlled_actor_adjacent_attack_skips_the_class_cascade_and_back_link() {
        // `combat.md §6.1a`: adjacent, the strike goes through the shared
        // attack-application primitive; the attacker back-link, the class
        // attack overrides, the poison/status branch and the monster
        // ranged-spell branch are all skipped.
        let mut state = combat_ai_turn_state(5, 6);
        state.combat_actors[8].owner_target_class = COMBAT_CLASS_GIANT_RAT;
        state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        assert!(
            combat_class_traits(COMBAT_CLASS_GIANT_RAT)
                .unwrap()
                .poison_status_attack,
            "the fixture class must have a poison/status branch to skip"
        );
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        }];

        let application = state
            .resolve_and_apply_combat_monster_attack(8, 0, 1, 0, 0, true, 0, Some(true))
            .expect("an adjacent controlled attacker still resolves an attack");

        assert!(
            application.poison_status_outcome.is_none(),
            "the poison/status branch must be skipped for a controlled attacker"
        );
        assert!(application.resolution.is_some());
        assert_eq!(
            state.combat_interference_sources[0], 0,
            "the controlled attack writes no attacker back-link"
        );
    }

    #[test]
    fn controlled_actor_attack_out_of_range_resolves_nothing() {
        let mut state = combat_ai_turn_state(9, 5);
        state.combat_actors[8].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
        assert!(
            state
                .resolve_and_apply_combat_monster_attack(8, 0, 1, 0, 0, false, 0, Some(true))
                .is_none(),
            "a controlled attacker requires straight-line distance exactly one"
        );
    }

    fn combat_charm_state(target_slot: usize, target_flags: u8, target_class: u8) -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.party_intelligence[0] = u8::MAX;
        state.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 6,
            hp: 30,
            max_hp: 30,
            level: 6,
        }];
        let spell_index = spell_index_from_code("AEX").unwrap();
        state.spell_charges[spell_index] = 1;
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            30,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            3,
            3,
        ]);
        state.combat_actors[target_slot] = CombatActorDescriptor::from_row([
            20,
            1,
            target_flags,
            target_class,
            target_slot as u8,
            0,
            5,
            5,
        ]);
        state
    }

    #[test]
    fn charm_names_its_victim_and_suppresses_the_dispatcher_epilogue() {
        // `catalogs/spell-list.md` id 34 + `combat.md §6.1a` Writers #2:
        // Charm "prints `<name> charmed!` and suppresses the shared epilogue".
        // The engine printed the generic `Charm!` instead.
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let mut state =
            combat_charm_state(target_slot, COMBAT_ACTOR_FLAG_SELECTABLE_80, COMBAT_CLASS_DAEMON);

        assert_eq!(
            state
                .cast_spell_from_suffix("1AEX7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert_eq!(state.message, "Daemon charmed!");
        assert!(state.combat_actors[target_slot].is_controlled());
    }

    #[test]
    fn a_second_charm_clears_the_controlled_marker() {
        // `magic.md §8` creature-prompt targeters: "a second successful Charm
        // on the same actor clears it". The shared creature-prompt predicate
        // rejects an actor that already carries the bit, which made the
        // clearing half unreachable.
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let mut state = combat_charm_state(
            target_slot,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED,
            COMBAT_CLASS_DAEMON,
        );

        assert_eq!(
            state
                .cast_spell_from_suffix("1AEX7", std::path::Path::new(""))
                .unwrap(),
            MoveOutcome::Cast
        );

        assert!(
            !state.combat_actors[target_slot].is_controlled(),
            "a second Charm clears the controlled/charmed bit"
        );
        assert_eq!(state.message, "Daemon charmed!");
    }

    #[test]
    fn charm_on_a_party_slot_restores_the_good_status_letter() {
        // `combat.md §6.1a` Writers #2: "When the accepted target is a
        // party-side slot, Charm also writes the Good status letter into that
        // character's roster status byte ... in both toggle directions."
        // `§12` is explicit that the byte is `'G'`, never `'C'`.
        let mut state = combat_charm_state(1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1);
        state.party.push(PartyMember {
            slot: 1,
            class_byte: 1,
            status: CharacterStatus::Poisoned.save_byte(),
            climb_stat: 0,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        });
        // A party-side slot only becomes eligible for the creature prompt once
        // it has been team-toggled onto the monster side, which is exactly the
        // enemy-charmed party member the clear half is for.
        state.combat_actors[1].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;

        state
            .apply_combat_charm_allegiance(1)
            .expect("a live party descriptor accepts the charm toggle");

        assert!(!state.combat_actors[1].is_controlled());
        assert_eq!(state.party[1].status, CharacterStatus::Good.save_byte());
        assert_ne!(state.party[1].status, CharacterStatus::Charmed.save_byte());

        // The other toggle direction writes `'G'` as well.
        state.party[1].status = CharacterStatus::Sleeping.save_byte();
        state.apply_combat_charm_allegiance(1).unwrap();
        assert!(state.combat_actors[1].is_controlled());
        assert_eq!(state.party[1].status, CharacterStatus::Good.save_byte());
    }

    #[test]
    fn charm_prompt_relaxes_only_the_already_marked_test() {
        // The relaxation is scoped to actors that already carry bit `0x01`.
        // Every other creature-prompt gate — empty, dead, hidden,
        // status-disabled, same-group — still refuses.
        let target_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let caster_group = COMBAT_TARGET_GROUP_PARTY;

        let hostile =
            combat_charm_state(target_slot, COMBAT_ACTOR_FLAG_SELECTABLE_80, COMBAT_CLASS_DAEMON);
        assert!(hostile.charm_prompt_target_is_eligible(target_slot, caster_group));

        let marked = combat_charm_state(
            target_slot,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED,
            COMBAT_CLASS_DAEMON,
        );
        assert!(
            marked.charm_prompt_target_is_eligible(target_slot, caster_group),
            "an already-marked actor is re-targetable so the second cast can clear it"
        );

        let dead = combat_charm_state(
            target_slot,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_MARKED_DEAD,
            COMBAT_CLASS_DAEMON,
        );
        assert!(!dead.charm_prompt_target_is_eligible(target_slot, caster_group));

        let hidden = combat_charm_state(
            target_slot,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
            COMBAT_CLASS_DAEMON,
        );
        assert!(!hidden.charm_prompt_target_is_eligible(target_slot, caster_group));

        let asleep = combat_charm_state(
            target_slot,
            COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_STATUS_DISABLED,
            COMBAT_CLASS_DAEMON,
        );
        assert!(!asleep.charm_prompt_target_is_eligible(target_slot, caster_group));

        // An unmarked party-side slot groups with the caster and is refused.
        let allied = combat_charm_state(1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 1);
        assert_eq!(
            allied.combat_target_group_for_slot(1),
            COMBAT_TARGET_GROUP_PARTY
        );
        assert!(!allied.charm_prompt_target_is_eligible(1, caster_group));

        // An empty slot is never eligible.
        let empty = combat_charm_state(target_slot, 0, 0);
        assert!(!empty.charm_prompt_target_is_eligible(COMBAT_ACTOR_SLOTS - 1, caster_group));
    }

    #[test]
    fn combat_placement_base_step_reverts_an_adjustment_past_thirty() {
        // `combat.md §5`: "a base-step of the class speed seed randomised by a
        // uniform `[-4, +3]` adjustment, reverted to the unadjusted seed
        // whenever the adjusted value would exceed thirty".
        assert_eq!(combat_placement_base_step(30, 0), 26);
        assert_eq!(combat_placement_base_step(30, 4), 30);
        assert_eq!(combat_placement_base_step(30, 5), 30);
        assert_eq!(combat_placement_base_step(30, 7), 30);
        assert_eq!(combat_placement_base_step(10, 0), 6);
        assert_eq!(combat_placement_base_step(10, 7), 13);
        assert_eq!(COMBAT_PLACEMENT_PHASE_BASE, 36);
    }

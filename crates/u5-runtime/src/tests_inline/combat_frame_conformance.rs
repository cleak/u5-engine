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

    #[test]
    fn every_combat_class_refreshes_its_phase_counter_above_zero() {
        // `combat.md §7` per-actor body step 5: "The counter is reset to
        // `36 - base_step`." `tick_combat_actor_phase_counter` treats a
        // counter of one or less as Ready, so a class whose refresh
        // resolves to zero acts on *every* table walk forever. The
        // regression this pins is a refresh constant of thirty: the ten
        // classes with a speed seed of thirty (Bat, Mongbat, Guard,
        // Insect Swarm, Wisp, Mimic, the three unique humans and the
        // Shadow Lord) all refreshed to zero, and a sixteen-Bat swarm
        // resolved dozens of attacks per player keystroke.
        assert_eq!(COMBAT_PHASE_REFRESH_CONSTANT, 36);
        assert_eq!(COMBAT_PHASE_REFRESH_CONSTANT, COMBAT_PLACEMENT_PHASE_BASE);
        for class in 0..=47u8 {
            let Some(stats) = combat_class_stats(class) else {
                continue;
            };
            for roll in 0..8u8 {
                let base_step = combat_placement_base_step(stats.speed_seed, roll);
                assert!(
                    base_step <= COMBAT_PLACEMENT_BASE_STEP_MAX,
                    "class {class} ({}) placed at base-step {base_step}",
                    stats.name
                );
                assert_ne!(
                    resolve_combat_phase_refresh_counter(base_step, COMBAT_PHASE_REFRESH_CONSTANT),
                    0,
                    "class {class} ({}) refreshes to zero at base-step {base_step}",
                    stats.name
                );
            }
        }
    }

    #[test]
    fn terrain_placement_seeds_base_step_and_phase_from_the_speed_variation_draw() {
        // `combat.md §5`: the placed descriptor gets "a base-step of the
        // class speed seed randomised by a uniform `[-4, +3]` adjustment,
        // reverted to the unadjusted seed whenever the adjusted value
        // would exceed thirty; a phase counter of thirty-six minus the
        // base-step". Every ordinary placement site used to write a phase
        // counter of zero and the raw speed seed.
        let record = CombatArenaRecord::from_record_bytes(&synthetic_combat_arena_record()).unwrap();
        let trigger = ActiveObject {
            type_byte: 0xc0,
            tile: 0xc0,
            x: 10,
            y: 20,
            z: WorldPlane::Britannia.save_floor(),
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        let setup =
            terrain_combat_setup_from_record_at_arena(WorldPlane::Britannia, trigger, 4, &record)
                .unwrap();
        let stats = setup.base_class.expect("the synthetic trigger has a class");

        // One seed per spawn, walking the whole `[-4, +3]` span.
        let speed_adjust_rolls: Vec<u8> = (0..8u8).collect();
        let instance =
            terrain_combat_instance_from_setup(&setup, 8, None, &[], &speed_adjust_rolls).unwrap();
        assert_eq!(instance.placed_count, 8);

        for spawn in 0..usize::from(instance.placed_count) {
            let actor = instance.actors[COMBAT_PARTY_ACTOR_SLOTS + spawn];
            let expected_base_step =
                combat_placement_base_step(stats.speed_seed, speed_adjust_rolls[spawn]);
            assert_eq!(actor.base_step, expected_base_step, "spawn {spawn}");
            assert_eq!(
                actor.phase_counter,
                COMBAT_PLACEMENT_PHASE_BASE - expected_base_step,
                "spawn {spawn}"
            );
            assert_ne!(actor.phase_counter, 0, "spawn {spawn}");
        }
    }

    #[test]
    fn terrain_placement_takes_one_speed_variation_draw_per_monster_in_placement_order() {
        // `combat.md §5`: "Each ordinary monster placement then consumes
        // one speed-variation draw." The draws interleave with the
        // per-monster companion check, which only early spawn indexes
        // below the `count / 4 + 1` threshold roll at all.
        let mut state = world_state(open_world_grid(), 10, 20);
        state.prng_state = 0x1234;
        let requested_count = 8u8;
        let companion_class = Some(41u8);

        let mut expected_prng = state.prng_state;
        let threshold = terrain_combat_replacement_threshold(requested_count);
        let mut expected_replacement = Vec::new();
        let mut expected_speed = Vec::new();
        for spawn in 0..requested_count {
            expected_replacement.push(if spawn != 0 && spawn < threshold {
                u5_prng_range_u16(
                    &mut expected_prng,
                    0,
                    u16::from(TERRAIN_COMBAT_REPLACEMENT_DENOMINATOR - 1),
                ) as u8
            } else {
                1
            });
            expected_speed.push(u5_prng_range_u16(
                &mut expected_prng,
                u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_LOW),
                u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_HIGH),
            ) as u8);
        }

        let (replacement_rolls, speed_rolls) =
            state.terrain_combat_placement_roll_seeds(requested_count, companion_class);

        assert_eq!(replacement_rolls, expected_replacement);
        assert_eq!(speed_rolls, expected_speed);
        assert_eq!(speed_rolls.len(), usize::from(requested_count));
        assert!(speed_rolls.iter().all(|roll| *roll <= 7));
        assert_eq!(state.prng_state, expected_prng);
    }

    #[test]
    fn dungeon_room_placement_consumes_its_speed_draw_instead_of_discarding_it() {
        // `combat.md §5`: the dungeon-room source scan places actors "in
        // ascending occupied-source order" and each ordinary placement
        // consumes one speed-variation draw. The site previously burned
        // the draw and wrote a phase counter of zero.
        let mut bytes = synthetic_combat_arena_record();
        let source_base =
            DUNGEON_ROOM_SOURCE_ROW * COMBAT_ARENA_ROW_STRIDE + DUNGEON_ROOM_SOURCE_COLUMN;
        for offset in 0..DUNGEON_ROOM_SOURCE_COUNT {
            bytes[source_base + offset] = 0x00;
        }
        bytes[source_base + 1] = 0x44;
        let record = CombatArenaRecord::from_record_bytes(&bytes).unwrap();
        let setup = dungeon_room_combat_setup_from_record(111, &record);

        let mut prng_state = 0x2468u16;
        let instance = dungeon_room_combat_instance_from_setup_with_prng(&setup, 7, &mut prng_state);

        // Mirror the published draw order: the four random-special
        // palette draws first, then this placement's speed variation.
        let mut expected_prng = 0x2468u16;
        let _ = dungeon_room_random_special_setup_ids(setup.scan_sources, &mut expected_prng);
        let expected_roll = u5_prng_range_u16(
            &mut expected_prng,
            u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_LOW),
            u16::from(COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_HIGH),
        ) as u8;
        assert_eq!(prng_state, expected_prng);

        let actor = instance.actors[COMBAT_PARTY_ACTOR_SLOTS];
        assert!(!actor.is_empty(), "the ordinary source must place an actor");
        let stats = combat_class_stats(actor.owner_target_class).unwrap();
        let expected_base_step = combat_placement_base_step(stats.speed_seed, expected_roll);
        assert_eq!(actor.base_step, expected_base_step);
        assert_eq!(
            actor.phase_counter,
            COMBAT_PLACEMENT_PHASE_BASE - expected_base_step
        );
        assert_ne!(actor.phase_counter, 0);
    }

    /// `combat.md` §4.1: the conflict banner "is **unconditional**. The test
    /// that precedes it cannot fail, so every terrain-setup entry prints it."
    /// The camp ambush is "the one entry that reaches setup without passing
    /// through" the world-side terrain-combat entry step, so it "prints the
    /// conflict banner below but **no** group name".
    #[test]
    fn the_camp_ambush_prints_the_conflict_banner_and_no_group_name() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.prng_state = 0x0f0f;
        state.message_transcript.clear();
        state
            .enter_sleep_ambush_combat(SleepAmbushMonster::Bat, 0, std::path::Path::new(""))
            .unwrap();

        let printed: Vec<String> = state
            .message_entries()
            .iter()
            .map(|entry| entry.text.clone())
            .collect();
        assert_eq!(
            printed.iter().filter(|text| **text == combat_banner_line()).count(),
            1,
            "transcript was {printed:?}"
        );
        assert!(
            !printed.iter().any(|text| text == "BATS"),
            "the camp ambush prints no group name: {printed:?}"
        );
    }

    #[test]
    fn sleep_ambush_placement_seeds_base_step_and_phase_like_any_other_placement() {
        // `combat.md §5`: the alternate rest/camp entry modes seat the
        // party and then write monster records through the same
        // placement rule.
        let mut state = world_state(open_world_grid(), 10, 20);
        state.prng_state = 0x0f0f;
        state
            .enter_sleep_ambush_combat(SleepAmbushMonster::Bat, 0, std::path::Path::new(""))
            .unwrap();

        let stats = combat_class_stats(COMBAT_CLASS_BAT).unwrap();
        assert_eq!(stats.speed_seed, 30);
        let placed: Vec<CombatActorDescriptor> = (COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS)
            .map(|slot| state.combat_actors[slot])
            .filter(|actor| !actor.is_empty())
            .collect();
        assert!(!placed.is_empty(), "the ambush must place at least one Bat");
        for actor in placed {
            assert_eq!(actor.owner_target_class, COMBAT_CLASS_BAT);
            assert!((26..=30).contains(&actor.base_step));
            assert_eq!(
                actor.phase_counter,
                COMBAT_PLACEMENT_PHASE_BASE - actor.base_step
            );
            assert_ne!(actor.phase_counter, 0);
        }
    }

    #[test]
    fn party_seating_takes_its_base_step_from_dexterity_and_no_prng_draw() {
        // `combat.md §5` party descriptor seeding: "Base-step | The
        // character's dexterity" and "Phase counter | Thirty-six minus
        // the base-step". §5 charges a speed-variation draw only to
        // *monster* placements, so seating must not touch the PRNG. The
        // class stat table's speed seed - the value this site used to
        // read - is a monster placement input.
        //
        // `combat.md §5.1` settles the floor question: "The seating pass
        // copies the character's dexterity byte into the actor's base step
        // verbatim. There is no minimum, no maximum, no level scaling, no
        // equipment adjustment and no random variation applied on the way."
        // The floor lives in chargen, not combat, and it is a *starting
        // point* for the questionnaire tally rather than a clamp.
        let mut state = world_state(open_world_grid(), 10, 20);
        state.prng_state = 0xbeef;
        state.party.truncate(1);
        state.party[0].class_byte = b'A';
        state.party[0].climb_stat = 3;
        state.party[0].status = b'G';
        let mut bard = state.party[0];
        bard.slot = 1;
        bard.class_byte = b'B';
        bard.climb_stat = 22;
        state.party.push(bard);

        let mut active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        let positions = [(1u8, 1u8), (2, 1), (3, 1), (4, 1), (5, 1), (6, 1)];
        state.populate_combat_party_with_positions(&mut active_objects, &mut actors, 0, &positions);

        assert_eq!(state.prng_state, 0xbeef, "party seating takes no draw");
        for slot in 0..2usize {
            let dexterity = state.party[slot].dexterity();
            assert_eq!(state.party[slot].climb_stat, dexterity);
            assert_eq!(actors[slot].base_step, dexterity, "slot {slot}");
            assert_eq!(
                actors[slot].phase_counter,
                COMBAT_PLACEMENT_PHASE_BASE - dexterity,
                "slot {slot}"
            );
        }
        // The Avatar's class-table speed seed is 25 and the Bard's is 20;
        // neither is what the descriptor now carries.
        assert_ne!(actors[0].base_step, combat_class_stats(3).unwrap().speed_seed);
        assert_ne!(actors[1].base_step, combat_class_stats(1).unwrap().speed_seed);
    }

    #[test]
    fn a_fast_monster_swarm_acts_about_twice_between_player_turns() {
        // `combat.md §7`, closing paragraph: "initiative is *interleaved*
        // by phase counter, so a fast monster might act twice between the
        // player's turns." Sixteen Bats (speed seed 30, refresh 6-10)
        // against a dexterity-22 party member (refresh 14) is the worst
        // shipped case. Before the fix every Bat refreshed to zero and
        // the swarm resolved twenty to forty actions per player turn.
        let mut state = combat_frame_conformance_state();
        state.party.truncate(1);
        state.party[0].climb_stat = 22;
        state.party[0].status = b'G';
        state.party[0].hp = 999;
        state.party[0].max_hp = 999;

        let party_step = state.party[0].dexterity();
        state.active_objects[0] = ActiveObject {
            type_byte: 0x4c,
            tile: 0x4c,
            x: 5,
            y: 10,
            ..ActiveObject::empty()
        };
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            0,
            party_step,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            COMBAT_PLACEMENT_PHASE_BASE - party_step,
            5,
            10,
        ]);

        let bat_stats = combat_class_stats(COMBAT_CLASS_BAT).unwrap();
        const BATS: usize = 16;
        for index in 0..BATS {
            let slot = COMBAT_PARTY_ACTOR_SLOTS + index;
            let x = (index % 8) as u8;
            let y = (index / 8) as u8;
            state.active_objects[slot] = ActiveObject {
                type_byte: 0xb4,
                tile: 0xb4,
                x: usize::from(x),
                y: usize::from(y),
                aux1: bat_stats.max_hp,
                ..ActiveObject::empty()
            };
            state.combat_actors[slot] = combat_placement_descriptor(
                bat_stats,
                slot as u8,
                x,
                y,
                COMBAT_ACTOR_FLAG_SELECTABLE_40,
                (index % 8) as u8,
            );
        }

        const PLAYER_TURNS: usize = 8;
        let mut monster_actions_between_turns = Vec::new();
        let mut since_player_turn = 0usize;
        let mut per_bat_since_player_turn = [0usize; BATS];
        let mut worst_single_bat = 0usize;
        let mut slot = 0usize;
        for _ in 0..4000 {
            if !state.combat_active || monster_actions_between_turns.len() >= PLAYER_TURNS {
                break;
            }
            let application =
                state.apply_combat_round_walk_from_slot(slot, COMBAT_PHASE_REFRESH_CONSTANT, false);
            for entry in &application.applications {
                if let CombatActorSlotDispatchApplication::Slot {
                    slot: acted,
                    phase_tick: Some(CombatActorPhaseTick::Ready { .. }),
                    ..
                } = entry
                {
                    if *acted >= COMBAT_PARTY_ACTOR_SLOTS {
                        since_player_turn += 1;
                        per_bat_since_player_turn[*acted - COMBAT_PARTY_ACTOR_SLOTS] += 1;
                    }
                }
            }
            slot = match application.stop_reason {
                CombatRoundWalkStopReason::EndOfRound => 0,
                _ => application.next_slot,
            };
            if application.stop_reason == CombatRoundWalkStopReason::AwaitingPlayer {
                monster_actions_between_turns.push(since_player_turn);
                since_player_turn = 0;
                worst_single_bat =
                    worst_single_bat.max(per_bat_since_player_turn.iter().copied().max().unwrap());
                per_bat_since_player_turn = [0usize; BATS];
            }
            if application.stop_reason == CombatRoundWalkStopReason::Exit {
                break;
            }
        }

        assert_eq!(
            monster_actions_between_turns.len(),
            PLAYER_TURNS,
            "the player must keep getting turns: {monster_actions_between_turns:?}"
        );
        // "A fast monster might act twice between the player's turns."
        // The fastest Bat here refreshes to six against the party's
        // fourteen, so a single Bat lands two or three actions per player
        // turn depending on where the window falls - never the fourteen
        // or more a refresh of zero produced.
        assert!(
            worst_single_bat <= 3,
            "one Bat acted {worst_single_bat} times inside a single player turn"
        );
        assert!(worst_single_bat >= 2, "initiative must stay interleaved");
        let worst = monster_actions_between_turns
            .iter()
            .copied()
            .max()
            .unwrap_or_default();
        assert!(
            worst <= 3 * BATS,
            "sixteen Bats resolved {worst} actions in one player turn: {monster_actions_between_turns:?}"
        );
        assert!(
            state.party[0].living(),
            "the party must survive eight interleaved turns"
        );
    }

    /// `combat.md §8.1`, the turn banner: "a newline, the actor's name, and -
    /// for a party-side actor - the clause `, armed with ` followed by the
    /// names of that actor's readied items separated by `, `, or `bare hands`
    /// when none qualifies, terminated by a colon."
    #[test]
    fn turn_banner_names_the_actor_and_its_qualifying_readied_items() {
        let mut equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
        assert_eq!(
            combat_turn_banner("Iolo", Some(&equipment)),
            "\nIolo, armed with bare hands:"
        );

        // "Only the helm, weapon-hand and shield-hand slots are scanned, and
        // only items whose per-item weapon-capability entry is non-zero are
        // named. Ordinary helms, ordinary shields and all body armour
        // therefore never appear in the clause."
        let leather_helm = 0u8;
        let plate_mail = 14u8;
        let large_shield = 5u8;
        let spiked_helm = 3u8;
        let dagger = 16u8;
        let spiked_shield = 6u8;
        assert_eq!(equipment_name(usize::from(leather_helm)), "Leather Helm");
        assert_eq!(equipment_name(usize::from(plate_mail)), "Plate Mail");
        assert_eq!(equipment_name(usize::from(large_shield)), "Large Shield");
        assert_eq!(equipment_name(usize::from(spiked_helm)), "Spiked Helm");
        assert_eq!(equipment_name(usize::from(dagger)), "Dagger");
        assert_eq!(equipment_name(usize::from(spiked_shield)), "Spiked Shield");
        equipment[EQUIP_SLOT_HELM] = leather_helm;
        equipment[EQUIP_SLOT_ARMOUR] = plate_mail;
        equipment[EQUIP_SLOT_OFFHAND] = large_shield;
        assert_eq!(
            combat_turn_banner("Iolo", Some(&equipment)),
            "\nIolo, armed with bare hands:"
        );

        // "- while the **spiked helm and spiked shield do**, because they
        // carry a non-zero capability entry." Scan order is helm, weapon
        // hand, shield hand (§8.2).
        equipment[EQUIP_SLOT_HELM] = spiked_helm;
        equipment[EQUIP_SLOT_WEAPON] = dagger;
        equipment[EQUIP_SLOT_OFFHAND] = spiked_shield;
        assert_eq!(
            combat_turn_banner("Iolo", Some(&equipment)),
            "\nIolo, armed with Spiked Helm, Dagger, Spiked Shield:"
        );

        // "A charmed monster acting under player control gets only its name
        // and the colon, with no armament clause."
        assert_eq!(combat_turn_banner("Troll", None), "\nTroll:");
    }

    /// `combat.md §8.1`: the banner is "emitted at the start of every
    /// keyboard-driven combatant's turn, *before any key is read*", and
    /// "appears identically whether the player then presses `A`, a direction
    /// key, `Space`, or anything else".
    #[test]
    fn opening_a_player_turn_prints_the_banner_before_any_key_is_read() {
        let mut state = combat_frame_conformance_state();
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
        state.active_objects[0] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        state.party_equipment[0] = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
        state.message.clear();

        state.ensure_pending_combat_player_turn();
        assert_eq!(state.pending_combat_actor_slot, Some(0));
        let expected = combat_turn_banner(
            &party_name_to_string(state.combat_party_name_for_slot(0).unwrap()).unwrap(),
            Some(&state.party_equipment[0]),
        );
        assert!(
            state.message.ends_with(&expected),
            "banner missing from {:?}",
            state.message
        );
        // "A free re-prompt after a refusal uses the short form and does
        // **not** reprint the banner": reinstating the pending slot without
        // reopening the turn must leave the transcript untouched.
        let after_open = state.message.clone();
        state.pending_combat_actor_slot = Some(0);
        state.ensure_pending_combat_player_turn();
        assert_eq!(state.message, after_open);
        assert_eq!(state.message.matches(expected.as_str()).count(), 1);
    }

    /// `combat.md §5` / `§5.3` step 3a: the surface camp ambush "sets and
    /// forwards the [shuffle] bit ... and draws exactly fifteen uniform
    /// `[0, 15]` draws, taken after seating and before the banner", and "with
    /// `N` monsters the permuted order makes them occupy a random `N`-subset
    /// of the sixteen authored cells in a random order, rather than the first
    /// `N`".
    #[test]
    fn surface_camp_ambush_places_monsters_through_the_fifteen_swap_permutation() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.prng_state = 0;

        // The published fifteen-swap vector from shared PRNG state `0x0000`.
        let mut expected_permutation_state = state.clone();
        expected_permutation_state.prng_state = 0;
        let permutation =
            expected_permutation_state.terrain_combat_placement_slot_permutation();
        assert_eq!(
            permutation,
            [2, 4, 1, 0, 14, 7, 6, 3, 5, 8, 10, 11, 12, 9, 13, 15]
        );

        // Replay the published draw order to predict the monster count: the
        // fifteen placement draws come first and leave the shared state at
        // `0x0cf4`, and only then does the count roll happen. If the route
        // took any other number of placement draws, or took the count roll
        // first, this predicted count would not match the placed count below.
        assert_eq!(expected_permutation_state.prng_state, 0x0cf4);
        let bat_stats = combat_class_stats_for_sprite_byte(sleep_ambush_monster_sprite(
            SleepAmbushMonster::Bat,
        ))
        .unwrap();
        let expected_count = expected_permutation_state
            .roll_terrain_combat_setup_count(bat_stats.default_spawn_count, false);

        state
            .enter_sleep_ambush_combat(SleepAmbushMonster::Bat, 0, std::path::Path::new(""))
            .unwrap();

        let placed: Vec<(u8, u8)> = (COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS)
            .map(|slot| state.combat_actors[slot])
            .filter(|actor| !actor.is_empty())
            .map(|actor| (actor.x, actor.y))
            .collect();
        assert!(!placed.is_empty(), "the ambush must place at least one Bat");
        assert_eq!(placed.len(), usize::from(expected_count));
        // With no `BRIT.CBT` on the empty path the route falls back to the
        // crate-private stand-in grid, so what this asserts is the *ordering*
        // the permutation imposes on whatever sixteen cells the record
        // supplies - not arena record 0's authored cells, which are shipped
        // data this test cannot see.
        let expected: Vec<(u8, u8)> = permutation
            .iter()
            .take(placed.len())
            .map(|slot| SLEEP_AMBUSH_FALLBACK_PLACEMENT_SLOTS[usize::from(*slot)])
            .collect();
        assert_eq!(placed, expected);

        // Identity order would have put the first monster in authored cell 0;
        // the permutation's first entry is cell 2.
        assert_ne!(placed[0], SLEEP_AMBUSH_FALLBACK_PLACEMENT_SLOTS[0]);
    }

    // NOTE: whether the *dungeon* rest interruption also shuffles is an open
    // spec question (see the scope note in `enter_sleep_ambush_combat`). The
    // engine's surface-only gate is a conservative stand-in and is
    // deliberately not pinned by a test here.

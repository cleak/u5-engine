// Conformance tests for spec commit `dd21c58`, which answers
// `cleak/u5-spec#187` (combat.md residuals) and `#188` (visibility.md
// compositor read sources). Nine of the fourteen combat answers are
// reversals: `RETRACTIONS.md` R356-R366.

/// `combat.md §8.1` / `RETRACTIONS.md` R356: the turn banner is "terminated
/// by a colon **and then a newline**", and `§8.2`'s item-name line is "on its
/// own line" with "the colon carr[ying] its own trailing newline", so
/// "`Attack-` breaks to a new line after **both**".
#[test]
fn attack_starts_a_fresh_line_after_the_banner_and_the_item_name_line() {
    let banner = combat_turn_banner("Shamino", None);
    assert!(
        banner.ends_with(":\n"),
        "the banner ends with a colon and a newline, got {banner:?}"
    );
    assert_eq!(COMBAT_TURN_BANNER_TERMINATOR, ":\n");

    let mut equipment = [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT];
    equipment[EQUIP_SLOT_WEAPON] = 26; // Bow
    equipment[EQUIP_SLOT_HELM] = 3; // Spiked Helm
    let attempts = combat_attack_attempts(&equipment);
    let line = combat_attack_item_line(&attempts, 0).expect("two items qualify");
    assert!(
        line.ends_with(":\n"),
        "the item-name line ends with a colon and a newline, got {line:?}"
    );
}

/// `combat.md §8.2`: "**One byte this document had not published: a single
/// leading space.** When two or three items qualify, the Attack walker
/// repositions the cursor and emits one space **before the first attempt's
/// newline**. It sits outside the per-attempt loop, so it appears **once per
/// Attack command**, not once per attempt, and not at all when fewer than two
/// items qualify."
#[test]
fn the_attack_walk_leads_with_one_space_only_when_two_or_three_items_qualify() {
    let mut two = combat_player_command_state(6, 5);
    two.party_equipment = default_party_equipment(1);
    two.party_equipment[0][EQUIP_SLOT_WEAPON] = 26; // Bow
    two.party_equipment[0][EQUIP_SLOT_HELM] = 3; // Spiked Helm
    let walk = two.begin_combat_attack_walk(0, true);
    assert!(
        walk.text.starts_with(" \n"),
        "one leading space before the first attempt's newline, got {:?}",
        walk.text
    );
    // The second attempt is reached by closing the first; the space is not
    // re-emitted, because it "sits outside the per-attempt loop".
    let second = two
        .apply_combat_targeting_cursor_key('\u{1b}')
        .expect("a cursor is open");
    assert!(
        !second.text.starts_with(' '),
        "the space is once per command, not once per attempt, got {:?}",
        second.text
    );

    let mut one = combat_player_command_state(6, 5);
    one.party_equipment = default_party_equipment(1);
    one.party_equipment[0][EQUIP_SLOT_WEAPON] = 26; // Bow
    let single = one.begin_combat_attack_walk(0, true);
    assert!(
        !single.text.starts_with(' '),
        "no leading space with one qualifying item, got {:?}",
        single.text
    );
}

/// `combat.md §7`, "What owns that coordinate" (`RETRACTIONS.md` R357): the
/// cursor "**seeds** them when it opens", "**rewrites** them on every accepted
/// move", leaves them alone on a rejected move, and "**The coordinate itself
/// is never cleared** - after the cursor closes it still holds the last cell
/// the cursor stood on, and only the lowered gate stops it being drawn."
#[test]
fn the_aim_marker_gate_closes_but_its_coordinate_is_never_cleared() {
    let mut state = combat_player_command_state(6, 5);
    state.party_equipment = default_party_equipment(1);
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = 26; // Bow, reach 7
    state.pending_combat_actor_slot = Some(0);
    state.combat_cursor_blink = true;

    assert!(!state.combat_aim_marker_gate);
    assert!(state.begin_combat_attack_walk(0, true).cursor_open);
    assert!(state.combat_aim_marker_gate, "the cursor raises the gate");
    assert_eq!(state.combat_aim_marker_cell, Some((5, 5)));

    let _ = state
        .apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST))
        .expect("a cursor is open");
    assert_eq!(
        state.combat_aim_marker_cell,
        Some((6, 5)),
        "an accepted move rewrites the pair"
    );

    // A rejected move - off the arena edge - leaves the pair exactly as it
    // was: "no message, no beep, nothing moves".
    for _ in 0..8 {
        let _ = state.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST));
    }
    let held = state.combat_aim_marker_cell;
    let _ = state.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_EAST));
    assert_eq!(state.combat_aim_marker_cell, held);

    let done = state
        .apply_combat_targeting_cursor_key('\u{1b}')
        .expect("a cursor is open");
    assert!(!done.cursor_open);
    assert!(!state.combat_aim_marker_gate, "the cursor lowers the gate");
    assert_eq!(
        state.combat_aim_marker_cell, held,
        "the coordinate survives the close"
    );
    assert_eq!(state.combat_secondary_marker(), None);
}

/// `combat.md §8.2`, "The remembered previous target": "**Cleared in exactly
/// one case.** On the ranged/cast arm ... a **confirmed empty cell** on that
/// arm really does wipe the memory. A **cancelled** attempt ... leave[s] the
/// previous value standing. The adjacent-aim arm has no clear at all, so an
/// empty-cell confirm there also leaves the previous value."
#[test]
fn only_a_confirmed_empty_cell_on_the_ranged_arm_clears_the_remembered_target() {
    // Ranged arm (Bow): confirm on an empty cell clears.
    let mut ranged = combat_player_command_state(6, 5);
    ranged.party_equipment = default_party_equipment(1);
    ranged.party_equipment[0][EQUIP_SLOT_WEAPON] = 26; // Bow
    ranged.combat_remembered_targets[0] = Some(8);
    assert!(ranged.begin_combat_attack_walk(0, true).cursor_open);
    // Move two cells north-west of the monster so the confirm lands empty.
    let _ = ranged.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_NORTH));
    let _ = ranged.apply_combat_targeting_cursor_key_with_inputs('\r', None);
    assert_eq!(
        ranged.combat_remembered_targets[0], None,
        "a confirmed empty cell on the ranged arm wipes the memory"
    );

    // Cancel leaves it standing.
    let mut cancelled = combat_player_command_state(6, 5);
    cancelled.party_equipment = default_party_equipment(1);
    cancelled.party_equipment[0][EQUIP_SLOT_WEAPON] = 26; // Bow
    cancelled.combat_remembered_targets[0] = Some(8);
    assert!(cancelled.begin_combat_attack_walk(0, true).cursor_open);
    let _ = cancelled.apply_combat_targeting_cursor_key('\u{1b}');
    assert_eq!(cancelled.combat_remembered_targets[0], Some(8));

    // The adjacent-aim arm has no clear at all.
    let mut melee = combat_player_command_state(6, 5);
    melee.party_equipment = default_party_equipment(1);
    melee.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_EMPTY;
    melee.combat_remembered_targets[0] = Some(8);
    assert!(melee.begin_combat_attack_walk(0, true).cursor_open);
    let _ = melee.apply_combat_targeting_cursor_key(char::from(INPUT_CODE_NORTH));
    let _ = melee.apply_combat_targeting_cursor_key_with_inputs('\r', None);
    assert_eq!(
        melee.combat_remembered_targets[0],
        Some(8),
        "an empty-cell confirm on the adjacent-aim arm leaves the previous value"
    );
}

/// `combat.md §8.2`: the memory "is invalidated for real when the slot index
/// is recycled: placing a new actor into a slot sweeps all thirty-two
/// combatants and resets to the sentinel every remembered target that named
/// it."
#[test]
fn placing_a_new_actor_into_a_slot_sweeps_every_memory_naming_it() {
    // Learn which slot the allocation takes, so the seeded memories name it
    // for certain rather than by luck. Allocation is deterministic for a
    // given table, so the probe and the run below land in the same slot.
    let mut probe = combat_ai_turn_state(8, 5);
    let recycled = probe
        .place_combat_monster_at_arena_cell(COMBAT_CLASS_GIANT_RAT, 2, 2, 0, 0)
        .expect("an empty slot is available")
        .actor_slot;

    let mut state = combat_ai_turn_state(8, 5);
    // Every combatant remembers a different slot, so exactly one seeded
    // memory names the slot about to be recycled...
    for (slot, entry) in state.combat_remembered_targets.iter_mut().enumerate() {
        *entry = Some(slot as u8);
    }
    // ...and two more are pointed at it by hand, because the sweep is over
    // "all thirty-two combatants", not over one entry.
    let bystanders = [0, 1].map(|slot| if slot == recycled { 2 } else { slot });
    for slot in bystanders {
        state.combat_remembered_targets[slot] = Some(recycled as u8);
    }

    let placed = state
        .place_combat_monster_at_arena_cell(COMBAT_CLASS_GIANT_RAT, 2, 2, 0, 0)
        .expect("an empty slot is available");
    assert_eq!(placed.actor_slot, recycled);

    for slot in bystanders {
        assert_eq!(
            state.combat_remembered_targets[slot], None,
            "the memory held by slot {slot} named the recycled index and is swept"
        );
    }
    assert_eq!(
        state.combat_remembered_targets[recycled], None,
        "the recycled slot's own seeded memory named itself and is swept too"
    );
    // The sweep touches nothing else: every other seeded memory survives
    // verbatim, and none of the thirty-two still names the recycled index.
    for (slot, entry) in state.combat_remembered_targets.iter().enumerate() {
        assert_ne!(
            *entry,
            Some(recycled as u8),
            "slot {slot} still names the recycled index"
        );
        if slot != recycled && !bystanders.contains(&slot) {
            assert_eq!(
                *entry,
                Some(slot as u8),
                "slot {slot} named no recycled index and must be untouched"
            );
        }
    }
}

/// `combat.md §7`, "The middle tier's flag is a stats-panel refresh request,
/// not a leave-combat flag" (`RETRACTIONS.md` R358): "Nothing anywhere in the
/// game leaves combat, breaks a loop, returns from a handler, or writes a
/// scene byte on the strength of it", and each mode loop "reads it once at the
/// top of its per-turn entry point and, if it is set, redraws the full party
/// stats panel and clears it".
#[test]
fn the_hazard_tier_raises_a_stats_panel_refresh_and_never_leaves_combat() {
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[0].x = 5;
    state.combat_actors[0].y = 5;
    assert!(!state.party_stats_panel_refresh_pending);

    let application = state
        .apply_combat_arena_field_contact(CombatArenaFieldKind::Fire, 0, 0, 3)
        .expect("the fire tier applies");
    assert!(matches!(
        application.contact_outcome,
        CombatArenaFieldContactOutcome::FireDamage { .. }
    ));
    assert!(
        state.party_stats_panel_refresh_pending,
        "the middle damaging tier requests a stats-panel refresh"
    );

    // Nothing leaves combat on it: with a live party the control is still an
    // ordinary walk.
    assert_eq!(
        state.combat_round_loop_control(false),
        CombatRoundLoopControl::ContinueActorWalk
    );

    // The mode loop's per-turn entry point redraws and clears.
    let redraws_before = state.pending_party_stats_panel_redraws;
    assert!(state.take_party_stats_panel_refresh());
    assert!(!state.party_stats_panel_refresh_pending);
    assert_eq!(
        state.pending_party_stats_panel_redraws,
        redraws_before + 1,
        "the reader redraws the full party stats panel"
    );
    assert!(!state.take_party_stats_panel_refresh(), "and clears it");
}

/// `combat.md §12`, "The Gazer branch is a sleep application, and it replaces
/// ordinary damage" (`RETRACTIONS.md` R359): "no damage roll, no defence roll,
/// no HP change, no experience credit", the status byte becomes `'S'`, the
/// descriptor's disabled bit is set, the active-player sentinel is cleared,
/// and the shared narrator prints `<target> slept!`.
#[test]
fn a_gazer_attack_sleeps_the_defender_instead_of_damaging_it() {
    let mut state = combat_ai_turn_state(6, 5);
    state.combat_actors[8].owner_target_class = COMBAT_CLASS_GAZER;
    state.active_player = Some(0);
    let hp_before = state.party[0].hp;

    let attack = state
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .expect("the Gazer acts");

    assert!(matches!(
        attack.sleep_effect,
        Some(CombatSleepEffectOutcome::PartyMemberSlept { .. })
    ));
    assert!(attack.resolution.is_none(), "no damage roll runs");
    assert!(attack.damage_application.is_none(), "no HP change");
    assert_eq!(state.party[0].hp, hp_before);
    assert_eq!(state.party[0].status, b'S');
    assert!(state.combat_actors[0].is_status_disabled());
    assert_eq!(state.active_player, None);
    assert_eq!(
        crate::input_dispatch::combat_monster_attack_result_message(&state, attack).as_deref(),
        Some("Avatar slept!")
    );

    // "A party defender already marked dead is refused outright, with nothing
    // written."
    let mut dead = combat_ai_turn_state(6, 5);
    dead.combat_actors[8].owner_target_class = COMBAT_CLASS_GAZER;
    dead.party[0].status = b'D';
    dead.party[0].hp = 0;
    let refused = dead
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .expect("the Gazer acts");
    assert_eq!(
        refused.sleep_effect,
        Some(CombatSleepEffectOutcome::RefusedDeadParty)
    );
    assert_eq!(dead.party[0].status, b'D');
}

/// `combat.md §12`: the Gazer arm's entry gate. "When the attacker is a
/// monster of the Gazer class **and the defender is not already asleep**, the
/// resolver applies the asleep state and returns straight to its own
/// epilogue." Already asleep is not a second no-op sleep: the branch is not
/// entered at all, and the attack falls through to the ordinary roller.
#[test]
fn a_gazer_swinging_at_an_already_asleep_defender_falls_through_to_damage() {
    // Party defender: the status letter alone is enough to close the gate.
    let mut party = combat_ai_turn_state(6, 5);
    party.combat_actors[8].owner_target_class = COMBAT_CLASS_GAZER;
    party.party[0].status = b'S';
    let hp_before = party.party[0].hp;

    assert!(party.combat_defender_is_already_asleep(0));
    let attack = party
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .expect("the Gazer acts");

    assert_eq!(attack.sleep_effect, None, "the sleep arm is not entered");
    assert!(
        attack.resolution.is_some(),
        "the attack falls through to the ordinary roller"
    );
    assert!(
        party.party[0].hp < hp_before,
        "an already-asleep defender takes ordinary damage, not a second sleep"
    );
    assert_eq!(party.party[0].status, b'S', "and stays asleep");

    // The descriptor's disabled bit is the other witness - "the whole of the
    // state §12 writes for a non-party defender", and the not-saved half of
    // the party pair - and it closes the gate on either side of the arena.
    let mut disabled = combat_ai_turn_state(6, 5);
    assert!(!disabled.combat_defender_is_already_asleep(0));
    assert!(!disabled.combat_defender_is_already_asleep(8));
    disabled.combat_actors[0].set_status_disabled();
    disabled.combat_actors[8].set_status_disabled();
    assert!(disabled.combat_defender_is_already_asleep(0));
    assert!(disabled.combat_defender_is_already_asleep(8));
    // Upstream of the gate, this engine's shared "active, not dead"
    // precondition already refuses a status-disabled defender outright, so
    // the descriptor witness never has to carry the refusal on its own.
    assert!(!combat_actor_is_active_not_dead(disabled.combat_actors[8]));
}

/// `combat.md §11` (`RETRACTIONS.md` R361), the Gremlin food-theft branch:
/// "Draw one uniform value over zero through three and accept on three of the
/// four ... **This draw is taken before the food test, so it is spent even
/// when the party has no food**", then "Subtract five from the party's food
/// supply, saturating at zero", "print ... `A <monster> stole some food!` on
/// its own row", and "**Consume the attack action and return.**"
#[test]
fn a_landed_gremlin_attack_steals_food_instead_of_dealing_damage() {
    let mut state = combat_ai_turn_state(6, 5);
    state.combat_actors[8].owner_target_class = COMBAT_CLASS_GREMLIN;
    state.food = 12;
    let hp_before = state.party[0].hp;

    // Seed a PRNG state whose next `[0, 3]` draw accepts.
    let mut seed = 0u16;
    loop {
        let mut probe = seed;
        if combat_food_theft_roll_accepts(u5_prng_range_u16(&mut probe, 0, 3) as u8) {
            break;
        }
        seed = seed.wrapping_add(1);
    }
    state.prng_state = seed;
    let sound_before = state.sound_effect_serial;

    let attack = state
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .expect("the Gremlin acts");

    // `combat.md §11.1`'s Food-steal row publishes the cue as "a rising cue
    // roughly 800 Hz toward 2000 Hz". That is `audio.md §6`'s 50-update
    // cast-failure envelope, not the 40-update action snap that starts at
    // 1200 Hz - and `audio.md §11`'s action-snap census enumerates that
    // recipe's sites by name without the theft among them.
    assert_eq!(
        state.sound_effects_after(sound_before),
        vec![SoundEffect::CastFailure]
    );

    assert_eq!(
        attack.food_theft,
        Some(CombatFoodTheftOutcome::Stole {
            food_before: 12,
            food_after: 7,
        })
    );
    assert_eq!(state.food, 7, "five, saturating at zero");
    assert!(attack.resolution.is_none(), "no damage chain runs");
    assert_eq!(state.party[0].hp, hp_before);
    assert_eq!(
        crate::input_dispatch::combat_monster_attack_result_message(&state, attack).as_deref(),
        Some("\nA Gremlin stole some food!\n")
    );

    // "the draw is taken **before** the food test, so it is spent even with an
    // empty larder", and the attack then falls through to ordinary melee.
    let mut hungry = combat_ai_turn_state(6, 5);
    hungry.combat_actors[8].owner_target_class = COMBAT_CLASS_GREMLIN;
    hungry.food = 0;
    hungry.prng_state = seed;
    let prng_before = hungry.prng_state;
    let fell_through = hungry
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .expect("the Gremlin acts");
    assert_eq!(fell_through.food_theft, Some(CombatFoodTheftOutcome::NoFood));
    assert_ne!(
        hungry.prng_state, prng_before,
        "the draw is spent even with an empty larder"
    );
    assert!(
        fell_through.resolution.is_some(),
        "the attack falls through to ordinary melee resolution"
    );
}

/// `combat.md §11`: "the branch sits **after** the to-hit roll, so a Gremlin
/// cannot steal on a miss".
#[test]
fn a_missed_gremlin_attack_never_reaches_the_theft_branch() {
    let mut state = combat_ai_turn_state(6, 5);
    state.combat_actors[8].owner_target_class = COMBAT_CLASS_GREMLIN;
    state.food = 12;

    let attack = state
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(false))
        .expect("the Gremlin acts");

    assert_eq!(attack.food_theft, None);
    assert_eq!(state.food, 12);
}

/// `combat.md §9`, "The teleport arm's odds and draw budget": "**three** draws
/// for a not-surrounded actor whose chance roll accepts, **one** when that
/// roll rejects, **two** when the actor is surrounded", the probe is "two
/// independent uniform draws over the sixteen values zero through fifteen ...
/// accepted only when both land inside the eleven-cell arena span".
#[test]
fn the_teleport_arm_spends_its_published_draw_budget() {
    assert!(combat_teleport_chance_accepts(0));
    assert!(combat_teleport_chance_accepts(1));
    assert!(combat_teleport_chance_accepts(2));
    assert!(!combat_teleport_chance_accepts(3));
    assert!(combat_teleport_probe_accepts(10, 10));
    assert!(!combat_teleport_probe_accepts(11, 0));
    assert!(!combat_teleport_probe_accepts(0, 15));

    // A not-surrounded actor: one chance draw, then two probe draws.
    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].owner_target_class = COMBAT_CLASS_WANDERER;
    let mut expected = state.prng_state;
    let accepted =
        combat_teleport_chance_accepts(u5_prng_range_u16(&mut expected, 0, 3) as u8);
    if accepted {
        let _ = u5_prng_range_u16(&mut expected, 0, 15);
        let _ = u5_prng_range_u16(&mut expected, 0, 15);
    }
    let _ = state.combat_ai_teleport_candidate(8);
    assert_eq!(
        state.prng_state, expected,
        "three draws on acceptance, one on rejection"
    );
}

/// `combat.md §9`: "the arm returns immediately for two classes, the
/// **Reaper** and the **Mimic**, which are immobile by design. They never step
/// and never teleport."
#[test]
fn the_reaper_and_the_mimic_are_refused_by_the_movement_arm() {
    assert!(combat_movement_arm_refuses_class(COMBAT_CLASS_REAPER));
    assert!(combat_movement_arm_refuses_class(COMBAT_CLASS_MIMIC));
    assert!(!combat_movement_arm_refuses_class(COMBAT_CLASS_GIANT_RAT));

    let mut state = combat_ai_turn_state(8, 5);
    state.combat_actors[8].owner_target_class = COMBAT_CLASS_REAPER;
    let before = (state.combat_actors[8].x, state.combat_actors[8].y);
    let application = state
        .apply_combat_ai_turn_with_inputs(
            8, false, 0, false, 1, 1, &[], None, 0, false, None, true, &[4, 1, 3, 2], None,
        )
        .expect("the Reaper's slot dispatches");
    assert!(application.movement.is_none(), "the Reaper never steps");
    assert_eq!((state.combat_actors[8].x, state.combat_actors[8].y), before);
}

/// `combat.md §12` (issue #187 question 13): "a monster class whose attack
/// byte is `99` returns `99` from the roller with the defence roll skipped,
/// and the damage-and-status endpoint ... kills outright." Two shipped classes
/// carry it: Wanderer and Lord British.
#[test]
fn a_monster_attack_byte_of_ninety_nine_is_the_instant_kill_sentinel() {
    for class in [COMBAT_CLASS_WANDERER, COMBAT_CLASS_LORD_BRITISH] {
        assert_eq!(
            combat_class_stats(class).unwrap().attack_value as i16,
            COMBAT_INSTANT_KILL_DAMAGE,
            "class {class} carries the sentinel attack byte"
        );
        assert_eq!(
            resolve_combat_attacker_raw_damage(
                CombatAttackerDamageSource::MonsterFlat {
                    attack_value: combat_class_stats(class).unwrap().attack_value,
                },
                0,
            )
            .unwrap()
            .route,
            CombatWeaponDamageRoute::Special,
            "the monster arm jumps onto the sentinel test"
        );
    }

    let mut state = combat_ai_turn_state(6, 5);
    state.combat_actors[8].owner_target_class = COMBAT_CLASS_WANDERER;
    state.party[0].hp = 30;
    let attack = state
        .resolve_and_apply_combat_monster_attack(8, 0, 0, false, 0, Some(true))
        .expect("the Wanderer acts");
    assert_eq!(state.party[0].hp, 0, "the sentinel kills regardless of HP");
    assert_eq!(state.party[0].status, b'D');
    assert!(matches!(
        attack.resolution,
        Some(CombatWeaponAttackResolution::Special { .. })
    ));
}

/// `combat.md §8.2`, "The to-hit draw is taken fresh inside each attempt", and
/// `§12`: the three always-hit ids "**short-circuit the to-hit roll to an
/// automatic hit with zero draws** on the ordinary (non-cast) arm, so a Glass
/// Sword or Sword of Chaos swing consumes no randomness at all".
#[test]
fn the_three_always_hit_ids_take_no_to_hit_draw() {
    let mut state = combat_player_command_state(6, 5);
    for item_id in EQUIPMENT_ALWAYS_HIT_ITEM_IDS {
        let before = state.prng_state;
        let inputs = state.combat_player_weapon_attack_inputs_for_item(0, Some(item_id));
        assert_eq!(
            state.prng_state, before,
            "item {item_id} must spend no to-hit draw"
        );
        assert_eq!(inputs.forced_hit, Some(true));
    }

    // An ordinary readied item still draws, once per attempt.
    let before = state.prng_state;
    let _ = state.combat_player_weapon_attack_inputs_for_item(0, Some(26));
    assert_ne!(state.prng_state, before);
    let after_first = state.prng_state;
    let _ = state.combat_player_weapon_attack_inputs_for_item(0, Some(26));
    assert_ne!(
        state.prng_state, after_first,
        "the draw is taken fresh inside each attempt"
    );
}

/// `combat.md §5` "Arena-centre special" (`RETRACTIONS.md` R362): the arm is
/// "gated on the centre cell already holding `0xDC`", its auxiliary byte is
/// "three times the current level index plus seven ... with no random draw of
/// any kind", and "**The terrain byte under the converted cell is not left as
/// loaded** ... the step's last act overwrites that centre cell with the
/// room's floor-fill terrain byte".
#[test]
fn the_arena_centre_special_is_gated_draw_free_and_overwrites_its_terrain() {
    let mut state = world_state(open_world_grid(), 10, 20);
    let mut terrain = [[0x04u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    let mut objects = vec![ActiveObject::empty(); OOL_SLOTS];

    // Gated: without the byte there is no conversion and no terrain change.
    let prng_before = state.prng_state;
    assert_eq!(
        state.place_combat_arena_centre_special(&mut terrain, Some(0x04), 2, &mut objects),
        None
    );
    assert_eq!(terrain[5][5], 0x04);

    let (row, column) = COMBAT_ARENA_CENTRE_CELL;
    terrain[row][column] = COMBAT_ARENA_CENTRE_SPECIAL_TILE;
    let record = state
        .place_combat_arena_centre_special(&mut terrain, Some(0x04), 2, &mut objects)
        .expect("a qualifying centre cell converts");
    assert_eq!(
        state.prng_state, prng_before,
        "setup id one's auxiliary-byte rule is draw-free"
    );
    assert_eq!(
        objects[record].aux1,
        2 * 3 + 7,
        "three times the level index plus seven, in the quantity/loot byte"
    );
    assert_eq!(
        objects[record].aux3, COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
        "the stamp creates a world-object row with no combat descriptor"
    );
    assert_eq!(
        terrain[row][column], 0x04,
        "the step overwrites the centre terrain with the room's floor fill"
    );
}

/// `combat.md §5` / `dungeon-mode.md §14.1`: the same step on the production
/// path. "The chest class stamps the byte the combat setup pass tests at the
/// arena centre ... That is the only way that byte ever reaches the centre
/// cell - no shipped arena record carries it - so dungeon-room combat entered
/// while the party stands on a chest cell is the sole live trigger for that
/// step", and the step then "overwrites that centre cell with the room's
/// floor-fill terrain byte".
#[test]
fn entering_dungeon_combat_from_a_chest_cell_converts_and_repaints_the_centre() {
    fn ambush_from_underfoot(cell: u8) -> PlayState {
        let mut state = test_state(open_grid(), 1, 1);
        state.player.facing = Direction::North;
        state.grid[dungeon_cell_index(3, 1, 1)] = cell;
        let object = ActiveObject {
            type_byte: 0,
            tile: 0,
            x: 2,
            y: 1,
            z: 3,
            phase: STEADY_PHASE,
            aux1: 20,
            aux3: DUNGEON_MONSTER_UPPER_DEP3,
        };
        state
            .enter_dungeon_active_monster_combat(3, object)
            .expect("the ambush launches");
        state
    }

    // A chest underfoot: the painter stamps the qualifying byte, the setup
    // pass converts it, and the terrain is repainted with the corridor fill.
    let chest = ambush_from_underfoot(0x40);
    assert_eq!(
        dungeon_cell_class_of(0x40),
        DungeonCellClass::Chest,
        "0x4? is the chest class"
    );
    let (row, column) = COMBAT_ARENA_CENTRE_CELL;
    let stamped = chest
        .active_objects
        .iter()
        .find(|object| {
            object.type_byte == COMBAT_ARENA_CENTRE_SPECIAL_SETUP_ID
                && (usize::from(object.x), usize::from(object.y)) == (column, row)
        })
        .expect("the centre cell converted to a setup-id-one special object");
    assert_eq!(
        stamped.aux1,
        3 * 3 + 7,
        "three times the level index plus seven, in the quantity/loot byte"
    );
    assert_eq!(
        stamped.aux3, COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
        "a world-object row with no combat descriptor"
    );
    assert_eq!(
        chest.combat_terrain[row][column], DUNGEON_AMBUSH_ARENA_FLOOR_TILE,
        "the centre terrain is overwritten with the room's floor fill, not          left holding 0xDC"
    );

    // Any other underfoot class stamps nothing, so the arm stays gated.
    let ladder = ambush_from_underfoot(0x10);
    assert!(
        ladder
            .active_objects
            .iter()
            .all(|object| object.type_byte != COMBAT_ARENA_CENTRE_SPECIAL_SETUP_ID
                || (usize::from(object.x), usize::from(object.y)) != (column, row)),
        "a non-chest underfoot class presents no qualifying centre cell"
    );
    assert_eq!(
        ladder.combat_terrain[row][column],
        DUNGEON_AMBUSH_ARENA_FLOOR_TILE
    );
}

/// `visibility.md §11` (`RETRACTIONS.md` R365): "There is exactly **one**
/// active-object compositor in the shipped game ... and combat enters it",
/// with a fixed list of five skips. The withdrawn text had the post-pass skip
/// combat scenes entirely, which left an arena uncomposited.
#[test]
fn a_combat_frame_composites_its_actors_through_the_shared_post_pass() {
    let mut state = combat_ai_turn_state(8, 5);
    state.visibility_dirty = true;
    state.visibility_buffers_ready = true;
    state.refresh_top_down_visibility_buffers(TopDownRenderArea::Town, VIEWPORT_PLAYER_ROW);

    let monster_grid = state.visibility_grid[visibility_grid_active_index(5, 8).unwrap()];
    let monster_band = state.terrain_band[terrain_band_active_index(5, 8).unwrap()];
    assert_eq!(
        monster_grid, VISIBILITY_USE_COMPANION,
        "the arena's monster slot is composited into the companion band"
    );
    assert_eq!(
        monster_band, 0x90,
        "and its sprite byte lands in the terrain band"
    );

    // Slot zero is walked like any other slot: no slot-zero refresh runs, so
    // the party record's own arena coordinates are what get composited.
    let party_grid = state.visibility_grid[visibility_grid_active_index(5, 5).unwrap()];
    assert_eq!(party_grid, VISIBILITY_USE_COMPANION);
    assert_eq!(
        state.terrain_band[terrain_band_active_index(5, 5).unwrap()],
        0x80
    );
}

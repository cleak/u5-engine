// Regressions for the pass-11 exploratory-playtest findings (p11combat) and
// the u5-spec#302 answers in the combat area.
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// `active-objects.md §3`: class `0xFC` (the Shadow Lord actor) takes the
/// default script A (frames 2, 3, 4) and "the displayed frame is class + n -
/// 1", which is the legal tile byte `0xFF` for frame 4. The animator added
/// `class + n` first and overflowed the byte, so a debug build panicked a few
/// turns after entering Stonegate (S06).
#[test]
fn playtest_s06_shadow_lord_frame_four_reaches_tile_ff_without_overflow() {
    let mut reached = false;
    for seed in 0..64u16 {
        let mut state = test_state(open_grid(), 1, 1);
        state.prng_state = seed.wrapping_mul(977).wrapping_add(3);
        state.active_objects.push(ActiveObject {
            type_byte: 0xfc,
            tile: 0xfe,
            x: 5,
            y: 5,
            z: 0,
            // Script position 2 (script A's `Frame(4)`), delay 0: the next
            // pass is a decision point.
            phase: 0x20,
            aux1: 0,
            aux3: 0,
        });
        state.animate_active_objects();
        let object = state.active_objects[1];
        if object.phase >> 4 == 3 {
            assert_eq!(object.tile, 0xff, "frame 4 of class 0xFC is 0xFC + 4 - 1");
            reached = true;
        } else {
            assert_eq!(object.tile, 0xfe, "a lost coin leaves the frame alone");
        }
    }
    assert!(reached, "some seed passes the eligibility coin");
}

/// A packed arena (`combat.md §5`): roster member 1 is Dead and unseated, so
/// roster member 2 acts from descriptor 1, and that descriptor's turn is open.
fn p11_packed_party_state() -> PlayState {
    let mut state = combat2b_two_member_state();
    state.party[1].status = b'D';
    state.party[1].hp = 0;
    state.party.push(PartyMember {
        slot: 2,
        class_byte: 1,
        status: b'G',
        climb_stat: 0,
        mana: 0,
        // Enough to outlast the bat's swings at the sleeper this round.
        hp: 900,
        max_hp: 999,
        level: 1,
    });
    state.party_experience.push(0);
    state.party_strengths.push(20);
    state.party_intelligence.push(20);
    state.party_equipment = default_party_equipment(3);
    state.combat_actors[1].owner_target_class = 2;
    assert_eq!(state.pending_combat_actor_slot, Some(1));
    state
}

/// `combat.md §8` Z row: "for a party-side actor it opens that character's
/// own sheet"; `inventory.md §6`: combat R binds "the active living combat
/// actor"; `magic.md §5` step 1: "Inside combat, the active-player byte is
/// whichever party-slot the round walker is currently dispatching". With a
/// dead member ahead in the roster the descriptor index named that dead
/// member, so Z showed his sheet and R and C refused (S05).
#[test]
fn playtest_s05_packed_descriptor_binds_z_ready_and_cast_to_its_own_member() {
    let game_dir = std::path::Path::new(".");
    let state = p11_packed_party_state();
    assert_eq!(state.z_stats_initial_party_index(), 2);
    assert_eq!(
        state.shared_acting_member_selection(true),
        ActingMemberSelection::Selected(2),
        "the acting member is the one the dispatched descriptor seats"
    );

    let mut z = p11_packed_party_state();
    handle_play_key_input(&mut z, 'Z', "", game_dir).unwrap();
    assert_eq!(
        z.active_z_stats.as_ref().map(|session| session.selected_party_index),
        Some(2),
        "Z opens Iolo's own sheet, not the dead Shamino's"
    );

    let mut ready = p11_packed_party_state();
    // One carried item, so the picker opens rather than the empty-handed
    // refusal.
    ready.equipment_stock[0] = 1;
    handle_play_key_input(&mut ready, 'R', "", game_dir).unwrap();
    assert_eq!(
        ready
            .active_ready
            .as_ref()
            .and_then(|session| session.selected_party_index),
        Some(2),
        "R opens the Item: picker for the acting member"
    );

    // The cast gate reads the caster's mana and consciousness from the
    // member descriptor 1 seats, so a live caster is charged.
    let mut cast = p11_packed_party_state();
    cast.party[2].mana = 10;
    cast.party[2].level = 4;
    cast.spell_charges[HEAL_SPELL_INDEX] = 1;
    assert_eq!(cast.cast_spell_resource_gate(1, HEAL_SPELL_INDEX, 1), None);
    assert_eq!(cast.party[2].mana, 9, "Iolo pays the mana");
    assert_eq!(cast.spell_charges[HEAL_SPELL_INDEX], 0);
}

/// `catalogs/item-list.md §7.2` Orange in combat: success "marks that combat
/// descriptor asleep/disabled", and the `combat.md §8.1` asleep arm then
/// replaces the sleeper's commands. The potion path set the roster letter on
/// member 2 but wrote the disabled bit on descriptor 2 (empty in a packed
/// arena), so the sleeper kept getting prompts (S05).
#[test]
fn playtest_s05_orange_potion_disables_the_packed_drinkers_descriptor() {
    let game_dir = std::path::Path::new(".");
    let mut state = p11_packed_party_state();
    state.potion_stock[POTION_ORANGE_INDEX] = 1;
    let seed = (1..=u16::MAX)
        .find(|seed| {
            let mut probe = state.clone();
            probe.prng_state = *seed;
            probe.potion_variation_roll() >= POTION_VARIATION_SELECTED_FIRST
        })
        .expect("some seed keeps the selected colour");
    state.prng_state = seed;

    combat2b_use_row(
        &mut state,
        |request| matches!(request, UseItemRequest::Potion { .. }),
        game_dir,
    );

    assert_eq!(state.party[2].status, b'S', "roster member 2 drank it");
    assert!(
        state.combat_actors[1].is_status_disabled(),
        "descriptor 1 seats member 2 and carries the asleep bit"
    );
    assert_eq!(state.party[1].status, b'D');
}

/// Put a Sleep Field marker on the arena cell of combat descriptor `slot`,
/// over plain floor (a swamp cell would take the terrain arm first).
fn p11_place_sleep_marker_under(state: &mut PlayState, slot: usize) {
    let actor = state.combat_actors[slot];
    state.combat_terrain[usize::from(actor.y)][usize::from(actor.x)] = 0x05;
    let free = (1..OOL_SLOTS)
        .find(|index| state.active_objects[*index].is_empty())
        .expect("a free record for the marker");
    state.active_objects[free] = ActiveObject {
        type_byte: COMBAT_FIELD_KIND_SLEEP,
        tile: COMBAT_FIELD_KIND_SLEEP,
        x: usize::from(actor.x),
        y: usize::from(actor.y),
        ..ActiveObject::empty()
    };
}

/// `combat.md §11`, Sleep row (u5-spec#302 Q6, `RETRACTIONS.md` R624):
/// contact "Call[s] the shared sleep routine of Section 12": a party target
/// gets "status `'S'` **whatever the current letter, Poisoned included**,
/// descriptor bit `0x08`, prone marker", and "No message (`slept!` is not
/// printed on this path)". "The hook runs after every dispatch of the actor,
/// including ... the one in which the wake draw succeeded, so an actor still
/// on the marker is put back to sleep straight away". Contact used to write
/// the roster letter alone, so the asleep arm never ran.
#[test]
fn spec302_q6_sleep_field_contact_runs_the_shared_sleep_routine_on_a_party_member() {
    let mut state = combat2b_two_member_state();
    state.party[1].status = b'P';
    state.active_player = Some(1);
    p11_place_sleep_marker_under(&mut state, 1);
    let rows_before = combat2b_rows_text(&state);
    let serial = state.sound_effect_serial;

    assert!(
        state
            .apply_combat_post_dispatch_contact_for_actor_position(1)
            .is_some()
    );

    assert_eq!(state.party[1].status, b'S', "Poisoned is not exempt");
    assert!(state.combat_actors[1].is_status_disabled(), "bit 0x08 is set");
    let object = usize::from(state.combat_actors[1].active_object_slot);
    assert_eq!(state.active_objects[object].tile, COMBAT_POTION_SLEEP_DISPLAY_TILE);
    assert_eq!(state.active_player, None, "the active-player selection is cleared");
    state.flush_message_slot();
    assert_eq!(combat2b_rows_text(&state), rows_before, "contact prints nothing");
    assert!(!state.message.contains("slept"));
    assert!(state.sound_effects_after(serial).is_empty(), "and plays nothing");

    // A successful wake draw restores `G`, and the hook at the end of that
    // same dispatch puts the member straight back to sleep.
    let wake = state
        .apply_combat_sleep_wake_dispatch(1, COMBAT_SLEEP_WAKE_SUCCESS_ROLL)
        .expect("a disabled actor takes the wake dispatch");
    assert!(wake.woke);
    assert_eq!(state.party[1].status, b'G');
    let _ = state.apply_combat_post_dispatch_contact_for_actor_position(1);
    assert_eq!(state.party[1].status, b'S');
    assert!(state.combat_actors[1].is_status_disabled());
}

/// `combat.md §11`/§12 (u5-spec#302 Q6): a non-party target gets "bit
/// `0x08`, the linked record's phase byte set to the freeze sentinel (sprite
/// unchanged)", and the wake helper later "zeroes the whole byte".
#[test]
fn spec302_q6_sleep_field_contact_freezes_a_monsters_animation_until_it_wakes() {
    let mut state = combat2b_two_member_state();
    let monster = (COMBAT_PARTY_ACTOR_SLOTS..state.combat_actors.len())
        .find(|slot| !state.combat_actors[*slot].is_empty())
        .expect("the fixture seats a monster");
    let object = usize::from(state.combat_actors[monster].active_object_slot);
    let tile_before = state.active_objects[object].tile;
    p11_place_sleep_marker_under(&mut state, monster);

    assert!(
        state
            .apply_combat_post_dispatch_contact_for_actor_position(monster)
            .is_some()
    );

    assert!(state.combat_actors[monster].is_status_disabled());
    assert_eq!(state.active_objects[object].phase, STEADY_PHASE, "the freeze sentinel");
    assert_eq!(state.active_objects[object].tile, tile_before, "sprite unchanged");

    let wake = state
        .apply_combat_sleep_wake_dispatch(monster, COMBAT_SLEEP_WAKE_SUCCESS_ROLL)
        .expect("a disabled actor takes the wake dispatch");
    assert!(wake.woke);
    assert_eq!(state.active_objects[object].phase, 0, "the script restarts at step 0");
}

/// `combat.md §8.1` / `audio.md §5.3` (added 2026-10-25, u5-spec#302 Q5):
/// the dragged-under arm's order is "`ARGH!` and its feed, the short rumble -
/// the random rumble `(40, 3000, 500)` ... - then the release draw", and a
/// release prints the regurgitated line, then "a longer rumble plays -
/// `(1, 7000, 600)`". Both rumbles were left silent while unpublished.
#[test]
fn spec302_q5_dragged_under_turns_play_the_argh_and_release_rumbles() {
    let mut seen = [false, false];
    for seed in 0..64u16 {
        let mut state = combat_state_after_the_conflict_banner(6, 5);
        state.combat_actors[0].flags |= COMBAT_ACTOR_FLAG_DRAGGED_UNDER;
        state.combat_actors[0].phase_counter = 1;
        state.combat_actors[0].base_step = 16;
        state.prng_state = seed.wrapping_mul(2_654).wrapping_add(1);
        let serial = state.sound_effect_serial;

        let _ = state.apply_combat_actor_slot_dispatch(0, 30);

        let sounds = state.sound_effects_after(serial);
        let released = !state.combat_actors[0].is_dragged_under();
        seen[usize::from(released)] = true;
        let expected: &[SoundEffect] = if released {
            &[
                SoundEffect::CombatDraggedUnderRumble,
                SoundEffect::CombatDraggedUnderReleaseRumble,
            ]
        } else {
            &[SoundEffect::CombatDraggedUnderRumble]
        };
        let dragged: Vec<SoundEffect> = sounds
            .iter()
            .cloned()
            .filter(|effect| {
                matches!(
                    effect,
                    SoundEffect::CombatDraggedUnderRumble
                        | SoundEffect::CombatDraggedUnderReleaseRumble
                )
            })
            .collect();
        assert_eq!(dragged, expected, "seed {seed}: {sounds:?}");
    }
    assert_eq!(seen, [true, true], "both a hold and a release must be exercised");

    let mut jitter = crate::audio::RumbleJitter::new();
    assert_eq!(
        SoundEffect::CombatDraggedUnderRumble.program(&mut jitter).ops.len(),
        75 + 1,
        "75 updates and the stop"
    );
    assert_eq!(
        SoundEffect::CombatDraggedUnderReleaseRumble.program(&mut jitter).ops.len(),
        7000 + 1,
        "7000 updates and the stop"
    );
}

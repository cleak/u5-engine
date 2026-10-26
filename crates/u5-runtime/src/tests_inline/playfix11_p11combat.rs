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

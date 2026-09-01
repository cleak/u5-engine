// `systems/audio.md` trigger-boundary regressions: spells noncombat.
//
// Each test names the published clause it pins. Add tests here rather
// than to the numbered chunks so the audio work stays reviewable as a
// unit.

/// Give one caster a charge for `spell_index`. The default party member
/// already carries level 8 and mana 8, which covers every circle.
fn arm_spell(state: &mut PlayState, spell_index: usize) {
    state.spell_charges[spell_index] = 1;
    state.party[0].mana = 8;
    state.party[0].level = 8;
}

#[test]
fn ring_vanish_plays_the_action_snap_after_the_mutation_and_narration() {
    // audio.md §8.1 "Ring vanishes": after the accepted mutation and
    // narration, play the 40-update action snap.
    let mut state = test_state(open_grid(), 1, 1);
    state.equipment_stock[EQUIPMENT_ID_RING_REGENERATION] = 1;
    // ready_ring_vanish_roll = (turn + party_index + item_id) & 0x0f.
    state.turn = 4;
    let serial = state.sound_effect_serial;

    assert_eq!(
        state.ready_equipment(InlineReadyRequest {
            party_index: 0,
            item_id: EQUIPMENT_ID_RING_REGENERATION,
        }),
        MoveOutcome::Used
    );

    assert!(
        state.message.ends_with("but it vanished."),
        "expected the vanish narration, got {:?}",
        state.message
    );
    assert_eq!(
        state.party_equipment[0][EQUIP_SLOT_RING], EQUIPMENT_EMPTY,
        "the ring is gone before the snap"
    );
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::ActionSnap]
    );
}

#[test]
fn ordinary_ready_and_its_refusals_stay_silent() {
    // audio.md §8.1: only the vanish arm sounds; §9 keeps generic
    // successful commands silent.
    let mut kept = test_state(open_grid(), 1, 1);
    kept.equipment_stock[EQUIPMENT_ID_RING_REGENERATION] = 1;
    kept.turn = 5;
    let serial = kept.sound_effect_serial;

    assert_eq!(
        kept.ready_equipment(InlineReadyRequest {
            party_index: 0,
            item_id: EQUIPMENT_ID_RING_REGENERATION,
        }),
        MoveOutcome::Used
    );
    assert!(
        !kept.message.ends_with("but it vanished."),
        "this roll keeps the ring: {:?}",
        kept.message
    );
    assert!(kept.sound_effects_after(serial).is_empty());

    // The refusal cascade never reaches the mutation.
    let mut refused = test_state(open_grid(), 1, 1);
    let serial = refused.sound_effect_serial;
    assert_eq!(
        refused.ready_equipment(InlineReadyRequest {
            party_index: 0,
            item_id: EQUIPMENT_ID_RING_REGENERATION,
        }),
        MoveOutcome::Blocked
    );
    assert!(refused.message.starts_with("No carried"));
    assert!(refused.sound_effects_after(serial).is_empty());
}

#[test]
fn vanish_commit_plays_variant_one_then_snaps_after_the_poof() {
    // audio.md §8.3 "Vanish success": variant 1 when direction input
    // commits, then the 40-update action snap after the accepted tile
    // rewrite, `POOF!`, dirtying, and redraw.
    let mut grid = open_grid();
    grid[32 + 2] = 0x90;
    let mut state = test_state(grid, 1, 1);
    arm_spell(&mut state, VANISH_SPELL_INDEX);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state.cast_vanish(0, Some(Direction::East), false),
        MoveOutcome::Cast
    );

    assert_eq!(state.message, "POOF!");
    assert_eq!(state.grid[32 + 2], VANISH_CLEARED_TILE);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 1 },
            SoundEffect::ActionSnap,
        ]
    );
}

#[test]
fn vanish_pass_and_direction_reprompt_stay_silent() {
    // audio.md §8.3: "Pass is silent." The direction prompt has not
    // committed yet either.
    let mut pass = test_state(open_grid(), 1, 1);
    arm_spell(&mut pass, VANISH_SPELL_INDEX);
    let serial = pass.sound_effect_serial;

    assert_eq!(pass.cast_vanish(0, None, true), MoveOutcome::Cast);
    assert_eq!(pass.message, DIRECTION_PROMPT_LABEL_PASS);
    assert!(pass.sound_effects_after(serial).is_empty());

    let mut prompt = test_state(open_grid(), 1, 1);
    arm_spell(&mut prompt, VANISH_SPELL_INDEX);
    let serial = prompt.sound_effect_serial;

    assert_eq!(prompt.cast_vanish(0, None, false), MoveOutcome::Observed);
    assert!(prompt.sound_effects_after(serial).is_empty());

    // A diagonal re-prompts from the already-spent cast and is still silent.
    let serial = prompt.sound_effect_serial;
    assert_eq!(
        prompt.confirm_spent_directed_utility_spell(
            0,
            VANISH_SPELL_INDEX,
            Some(Direction::NorthEast),
            false
        ),
        MoveOutcome::Observed
    );
    assert!(prompt.sound_effects_after(serial).is_empty());
}

#[test]
fn vanish_nonmatching_tile_keeps_variant_one_then_plays_the_failure_tail() {
    // audio.md §8.3: "A nonmatching tile retains the earlier variant-1
    // presentation, then reaches the common failure tail." §8.3 again:
    // "After `Failed!`, play the 50-update ... cast-failure glissando."
    let mut state = test_state(open_grid(), 1, 1);
    arm_spell(&mut state, VANISH_SPELL_INDEX);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state.cast_vanish(0, Some(Direction::East), false),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, "Failed!");
    assert_eq!(
        state.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 1 },
            SoundEffect::CastFailure,
        ]
    );
}

#[test]
fn open_magic_lock_and_unlock_magic_use_their_published_variants() {
    // audio.md §6: "successful Open" is variant 2; "Magic Lock, and
    // successful unlock-door effects" are variant 5. Only Vanish adds the
    // §8.3 action snap.
    for (spell_index, tile, rewrite, variant) in [
        (OPEN_SPELL_INDEX, 0xB9u8, 0xB8u8, 2u8),
        (MAGIC_LOCK_SPELL_INDEX, 0xB8, 0x97, 5),
        (UNLOCK_MAGIC_SPELL_INDEX, 0x97, 0xB8, 5),
    ] {
        let mut grid = open_grid();
        grid[32 + 2] = tile;
        let mut state = test_state(grid, 1, 1);
        arm_spell(&mut state, spell_index);
        let serial = state.sound_effect_serial;

        assert_eq!(
            state.cast_directed_utility_spell(
                0,
                spell_index,
                spell_circle_for(spell_index as u8).unwrap(),
                Some(Direction::East),
                false,
            ),
            MoveOutcome::Cast,
            "spell {spell_index}"
        );

        assert_eq!(state.message, "Success!");
        assert_eq!(state.grid[32 + 2], rewrite);
        assert_eq!(
            state.sound_effects_after(serial),
            vec![SoundEffect::SharedVariant { variant }],
            "spell {spell_index} sounds its variant and no action snap"
        );
    }
}

#[test]
fn committed_party_spells_play_their_published_variant_before_the_effect() {
    // audio.md §6 variant 1: "Awaken, Cure, Heal". Variant 8: "The highest
    // resurrection-mode presentation". audio.md §8.3 puts the pre-effect
    // after the spell's own gate and before the effect, so a post-commit
    // failure sounds the variant first and the failure tail second.
    let mut cure = test_state(open_grid(), 1, 1);
    cure.party[0].status = b'P';
    arm_spell(&mut cure, CURE_SPELL_INDEX);
    let serial = cure.sound_effect_serial;
    assert_eq!(cure.cast_cure(0, 0), MoveOutcome::Cast);
    assert_eq!(
        cure.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 1 }]
    );

    let mut heal = test_state(open_grid(), 1, 1);
    heal.party[0].hp = 1;
    arm_spell(&mut heal, HEAL_SPELL_INDEX);
    let serial = heal.sound_effect_serial;
    assert_eq!(heal.cast_heal(0, 0), MoveOutcome::Cast);
    assert_eq!(
        heal.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 1 }]
    );

    let mut awaken = test_state(open_grid(), 1, 1);
    arm_spell(&mut awaken, AWAKEN_SPELL_INDEX);
    let serial = awaken.sound_effect_serial;
    assert_eq!(awaken.cast_awaken(0), MoveOutcome::Blocked);
    assert_eq!(awaken.message, "Failed!");
    assert_eq!(
        awaken.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 1 },
            SoundEffect::CastFailure,
        ],
        "a no-sleeper Awaken still commits, then fails"
    );

    let mut resurrect = test_state(open_grid(), 1, 1);
    arm_spell(&mut resurrect, RESURRECT_SPELL_INDEX);
    let serial = resurrect.sound_effect_serial;
    assert_eq!(resurrect.cast_resurrect(0, 0), MoveOutcome::Blocked);
    assert_eq!(
        resurrect.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 8 },
            SoundEffect::CastFailure,
        ]
    );
}

#[test]
fn committed_utility_spells_play_their_published_variant() {
    // audio.md §6.1: "the variant is the tier index of the thing being used",
    // and for a spell that index is its circle. Locate (id 9) and Create Food
    // (id 11) are circle 2, Peer (id 39) is circle 7, Great Light (id 12) is
    // circle 3, and Light (id 0) is circle 1. "No spell uses variant 0" -
    // variant 0 belongs to the Light scroll, not to In Lor.
    let mut locate = world_state(open_world_grid(), 10, 20);
    arm_spell(&mut locate, IN_WIS_SPELL_INDEX);
    let serial = locate.sound_effect_serial;
    assert_eq!(locate.cast_locate(0), MoveOutcome::Observed);
    assert_eq!(
        locate.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 2 }]
    );

    let mut food = world_state(open_world_grid(), 10, 20);
    arm_spell(&mut food, CREATE_FOOD_SPELL_INDEX);
    let serial = food.sound_effect_serial;
    assert_eq!(food.cast_create_food(0), MoveOutcome::Cast);
    assert_eq!(
        food.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 2 }]
    );

    let mut peer = world_state(open_world_grid(), 10, 20);
    arm_spell(&mut peer, PEER_SPELL_INDEX);
    let serial = peer.sound_effect_serial;
    assert_eq!(peer.cast_peer(0), MoveOutcome::Observed);
    assert_eq!(
        peer.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 7 }]
    );

    let mut great_light = world_state(open_world_grid(), 10, 20);
    arm_spell(&mut great_light, VAS_LOR_SPELL_INDEX);
    let serial = great_light.sound_effect_serial;
    assert_eq!(
        great_light.cast_light_spell(0, VAS_LOR_SPELL_INDEX, VAS_LOR_COST, 20),
        MoveOutcome::Cast
    );
    assert_eq!(
        great_light.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 3 }]
    );

    let mut light = world_state(open_world_grid(), 10, 20);
    arm_spell(&mut light, IN_LOR_SPELL_INDEX);
    let serial = light.sound_effect_serial;
    assert_eq!(
        light.cast_light_spell(0, IN_LOR_SPELL_INDEX, IN_LOR_COST, 10),
        MoveOutcome::Cast
    );
    assert_eq!(
        light.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 1 }],
        "In Lor is circle 1; variant 0 is the Light scroll's"
    );
}

#[test]
fn committed_dungeon_spells_play_their_published_variant() {
    // audio.md §6: "Dungeon rise/fall and Dispel Field" are variant 4, and
    // Open is variant 2 on its dungeon route too.
    let mut descend = dungeon_state(open_dungeon_record(), 0, 4, 4);
    arm_spell(&mut descend, DES_POR_SPELL_INDEX);
    let serial = descend.sound_effect_serial;
    descend
        .cast_dungeon_level_spell(0, DES_POR_SPELL_INDEX, 1, "Down", Path::new(""))
        .unwrap();
    assert_eq!(
        descend.sound_effects_after(serial)[0],
        SoundEffect::SharedVariant { variant: 4 },
        "the committed cast sounds before the destination test"
    );

    // No field under the target cell: the variant still runs, then the tail.
    let mut dispel = dungeon_state(open_dungeon_record(), 0, 4, 4);
    arm_spell(&mut dispel, DISPEL_FIELD_SPELL_INDEX);
    let serial = dispel.sound_effect_serial;
    assert_eq!(
        dispel.cast_dispel_field(0, Some(Direction::East)),
        MoveOutcome::Blocked
    );
    assert_eq!(dispel.message, "Failed!");
    assert_eq!(
        dispel.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 4 },
            SoundEffect::CastFailure,
        ]
    );

    let mut open = dungeon_state(open_dungeon_record(), 0, 4, 4);
    open.grid[dungeon_cell_index(0, 4, 4)] = 0x40;
    arm_spell(&mut open, OPEN_SPELL_INDEX);
    let serial = open.sound_effect_serial;
    assert_eq!(
        open.cast_open_spell(0, None, false, Path::new("")).unwrap(),
        MoveOutcome::ContainerOpened
    );
    assert_eq!(
        open.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 2 }]
    );
}

/// The tile `noncombat_blink_target` accepts as a landing cell.
const BLINK_LANDING_TILE: u8 = 0x05;

#[test]
fn blink_success_plays_variant_three_and_nothing_else() {
    // audio.md §6 lists "Blink" unqualified, so the variant belongs at the
    // commit; audio.md §9 adds no success chime after it. `open_world_grid`
    // is entirely `BLINK_LANDING_TILE`, so the eastward scan is known to
    // find a destination.
    let mut state = world_state(open_world_grid(), 10, 20);
    assert!(
        state.noncombat_blink_target(Direction::East).is_some(),
        "this world must make the destination search succeed"
    );
    arm_spell(&mut state, BLINK_SPELL_INDEX);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state
            .cast_blink(0, Some(Direction::East), false, Path::new(""))
            .unwrap(),
        MoveOutcome::Cast
    );

    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 3 }]
    );
}

#[test]
fn blink_failure_plays_variant_three_then_the_failure_tail() {
    // audio.md §8.3 puts the committed pre-effect "after the spell's own
    // input gate accepts", so a destination search that then finds nothing
    // still sounds variant 3 first and only then the §8.3 failure tail
    // "After `Failed!`, play the 50-update ... cast-failure glissando".
    let mut grid = open_world_grid();
    for x in 0..WORLD_SIDE {
        // Clear the whole scanned row, so no cell can land the blink.
        grid[world_cell_index(x, 20)] = BLINK_LANDING_TILE + 1;
    }
    let mut state = world_state(grid, 10, 20);
    assert!(
        state.noncombat_blink_target(Direction::East).is_none(),
        "this world must make the destination search fail"
    );
    arm_spell(&mut state, BLINK_SPELL_INDEX);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state
            .cast_blink(0, Some(Direction::East), false, Path::new(""))
            .unwrap(),
        MoveOutcome::Blocked
    );

    assert_eq!(state.message, "Failed!");
    assert_eq!(
        state.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 3 },
            SoundEffect::CastFailure,
        ]
    );
}

#[test]
fn refusals_before_the_committed_cast_stay_silent() {
    // audio.md §8.3: the presentation "is committed only after the spell's
    // own input gate accepts", so every refusal ahead of that gate - the
    // scene refusal, the direction prompt, and the resource gate itself -
    // is silent.
    let mut not_here = test_state(open_grid(), 1, 1);
    arm_spell(&mut not_here, IN_WIS_SPELL_INDEX);
    let serial = not_here.sound_effect_serial;
    assert_eq!(not_here.cast_locate(0), MoveOutcome::Blocked);
    assert_eq!(not_here.message, "Not here!");
    assert!(not_here.sound_effects_after(serial).is_empty());

    let mut no_direction = dungeon_state(open_dungeon_record(), 0, 4, 4);
    arm_spell(&mut no_direction, DISPEL_FIELD_SPELL_INDEX);
    let serial = no_direction.sound_effect_serial;
    assert_eq!(no_direction.cast_dispel_field(0, None), MoveOutcome::Blocked);
    assert!(no_direction.sound_effects_after(serial).is_empty());

    let mut no_charge = test_state(open_grid(), 1, 1);
    no_charge.spell_charges[AWAKEN_SPELL_INDEX] = 0;
    let serial = no_charge.sound_effect_serial;
    assert_eq!(no_charge.cast_awaken(0), MoveOutcome::Blocked);
    assert!(
        no_charge.sound_effects_after(serial).is_empty(),
        "a rejected resource gate never commits the cast"
    );

    let mut out_of_range = test_state(open_grid(), 1, 1);
    arm_spell(&mut out_of_range, CURE_SPELL_INDEX);
    let serial = out_of_range.sound_effect_serial;
    assert_eq!(out_of_range.cast_cure(0, 9), MoveOutcome::Blocked);
    assert!(out_of_range.sound_effects_after(serial).is_empty());
    assert_eq!(
        out_of_range.spell_charges[CURE_SPELL_INDEX], 1,
        "the pre-gate refusal spends nothing"
    );
}

#[test]
fn combat_reveal_and_invisibility_play_their_published_variants() {
    // audio.md §6.1: Reveal is id 23, circle 4; Invisibility is id 36,
    // circle 7. The withdrawn "Reveal/locate" grouping put Reveal at 2, but
    // Locate is id 9, circle 2, and the two do not share a variant.
    let mut reveal = test_state(open_grid(), 1, 1);
    reveal.combat_active = true;
    arm_spell(&mut reveal, REVEAL_SPELL_INDEX);
    let serial = reveal.sound_effect_serial;
    assert_eq!(reveal.cast_reveal(0), MoveOutcome::Cast);
    assert_eq!(
        reveal.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 4 }]
    );

    // Nothing to hide: the committed cast still sounds, then fails.
    let mut invisible = test_state(open_grid(), 1, 1);
    invisible.combat_active = true;
    arm_spell(&mut invisible, INVISIBILITY_SPELL_INDEX);
    let serial = invisible.sound_effect_serial;
    assert_eq!(invisible.cast_invisibility(0), MoveOutcome::Blocked);
    assert_eq!(invisible.message, "Failed!");
    assert_eq!(
        invisible.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 7 },
            SoundEffect::CastFailure,
        ]
    );

    // The pre-gate scene refusal stays silent.
    let mut not_here = test_state(open_grid(), 1, 1);
    arm_spell(&mut not_here, REVEAL_SPELL_INDEX);
    let serial = not_here.sound_effect_serial;
    assert_eq!(not_here.cast_reveal(0), MoveOutcome::Blocked);
    assert_eq!(not_here.message, "Not here!");
    assert!(not_here.sound_effects_after(serial).is_empty());
}

/// A combat frame with one live party caster at (5, 5) on an all-walkable
/// arena, armed for `spell_index`.
fn combat_cursor_state(spell_index: usize) -> PlayState {
    let mut state = world_state(open_world_grid(), 10, 20);
    state.combat_active = true;
    state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([30, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.active_objects[0] = ActiveObject {
        type_byte: 0x80,
        tile: 0x80,
        x: 5,
        y: 5,
        ..ActiveObject::empty()
    };
    arm_spell(&mut state, spell_index);
    state
}

#[test]
fn combat_blink_confirmation_sounds_before_the_coordinate_resolver() {
    // audio.md §8.3: "For combat cursor spells, confirmation plays the spell
    // effect before the coordinate/projectile-impact resolver." The same
    // confirmed coordinate therefore opens with audio.md §6's variant 3
    // whether the resolver then accepts or rejects it.
    let mut accepted = combat_cursor_state(BLINK_SPELL_INDEX);
    let serial = accepted.sound_effect_serial;
    assert_eq!(
        accepted.cast_combat_blink_to_coordinate(0, Some((5, 6))),
        MoveOutcome::Cast
    );
    assert_eq!(
        (accepted.combat_actors[0].x, accepted.combat_actors[0].y),
        (5, 6)
    );
    assert_eq!(
        accepted.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 3 }]
    );

    // The identical confirmed coordinate, now illegal terrain: the resolver
    // rejects it, but the variant has already sounded and the §8.3 failure
    // tail follows `Failed!`.
    let mut rejected = combat_cursor_state(BLINK_SPELL_INDEX);
    rejected.combat_terrain[6][5] = 0x01;
    let serial = rejected.sound_effect_serial;
    assert_eq!(
        rejected.cast_combat_blink_to_coordinate(0, Some((5, 6))),
        MoveOutcome::Blocked
    );
    assert_eq!(rejected.message, "Failed!");
    assert_eq!(
        (rejected.combat_actors[0].x, rejected.combat_actors[0].y),
        (5, 5)
    );
    assert_eq!(
        rejected.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 3 },
            SoundEffect::CastFailure,
        ]
    );
}

#[test]
fn combat_blink_cursor_and_caster_refusals_stay_silent() {
    // audio.md §8.3: "A direction or combat-cursor cancellation that the
    // spell spec places before its sound skips the shared variant." The
    // missing-target prompt, the scene refusal, and the dead-caster refusal
    // all sit ahead of the confirmation, so none of them sound.
    let mut prompt = combat_cursor_state(BLINK_SPELL_INDEX);
    let serial = prompt.sound_effect_serial;
    assert_eq!(
        prompt.cast_combat_blink_to_coordinate(0, None),
        MoveOutcome::Blocked
    );
    assert!(prompt.message.starts_with("Target?"));
    assert!(prompt.sound_effects_after(serial).is_empty());
    assert_eq!(
        prompt.spell_charges[BLINK_SPELL_INDEX], 1,
        "the cursor prompt spends nothing"
    );

    let mut not_here = combat_cursor_state(BLINK_SPELL_INDEX);
    not_here.combat_active = false;
    let serial = not_here.sound_effect_serial;
    assert_eq!(
        not_here.cast_combat_blink_to_coordinate(0, Some((5, 6))),
        MoveOutcome::Blocked
    );
    assert_eq!(not_here.message, "Not here!");
    assert!(not_here.sound_effects_after(serial).is_empty());

    let mut no_caster = combat_cursor_state(BLINK_SPELL_INDEX);
    no_caster.combat_actors[0] = CombatActorDescriptor::default();
    let serial = no_caster.sound_effect_serial;
    assert_eq!(
        no_caster.cast_combat_blink_to_coordinate(0, Some((5, 6))),
        MoveOutcome::Blocked
    );
    assert_eq!(no_caster.message, "Who casts?");
    assert!(no_caster.sound_effects_after(serial).is_empty());
}

#[test]
fn failed_directed_utility_casts_sound_only_the_failure_tail() {
    // audio.md §6 qualifies these rows by success - "successful Open" is
    // variant 2, and variant 5 covers "Magic Lock, and successful
    // unlock-door effects" - and audio.md §8.3's only pre-success spell
    // boundary is Vanish. A failed attempt therefore reaches the §8.3
    // failure tail with no shared variant ahead of it.
    for spell_index in [
        OPEN_SPELL_INDEX,
        MAGIC_LOCK_SPELL_INDEX,
        UNLOCK_MAGIC_SPELL_INDEX,
    ] {
        // A tile the route has no rewrite for, and no chest under it.
        let mut nonmatching = test_state(open_grid(), 1, 1);
        arm_spell(&mut nonmatching, spell_index);
        let serial = nonmatching.sound_effect_serial;
        assert_eq!(
            nonmatching.cast_directed_utility_spell(
                0,
                spell_index,
                spell_circle_for(spell_index as u8).unwrap(),
                Some(Direction::East),
                false,
            ),
            MoveOutcome::Blocked,
            "spell {spell_index}"
        );
        assert_eq!(nonmatching.message, "Failed!");
        assert_eq!(
            nonmatching.sound_effects_after(serial),
            vec![SoundEffect::CastFailure],
            "spell {spell_index} must not sound a success variant on a nonmatching tile"
        );

        // An off-map target never reaches a tile at all.
        let mut off_map = test_state(open_grid(), 0, 1);
        arm_spell(&mut off_map, spell_index);
        let serial = off_map.sound_effect_serial;
        assert_eq!(
            off_map.cast_directed_utility_spell(
                0,
                spell_index,
                spell_circle_for(spell_index as u8).unwrap(),
                Some(Direction::West),
                false,
            ),
            MoveOutcome::Blocked,
            "spell {spell_index}"
        );
        assert_eq!(off_map.message, "Failed!");
        assert_eq!(
            off_map.sound_effects_after(serial),
            vec![SoundEffect::CastFailure],
            "spell {spell_index} must not sound a success variant off the map"
        );
    }
}

#[test]
fn vanish_is_the_only_directed_utility_spell_that_sounds_before_its_tile_test() {
    // audio.md §8.3: "Vanish first runs variant 1 when direction input
    // commits ... A nonmatching tile retains the earlier variant-1
    // presentation, then reaches the common failure tail." No other row of
    // the audio.md §6 table carries that pre-success boundary.
    let mut vanish = test_state(open_grid(), 1, 1);
    arm_spell(&mut vanish, VANISH_SPELL_INDEX);
    let serial = vanish.sound_effect_serial;
    assert_eq!(
        vanish.cast_vanish(0, Some(Direction::East), false),
        MoveOutcome::Blocked
    );
    assert_eq!(
        vanish.sound_effects_after(serial),
        vec![
            SoundEffect::SharedVariant { variant: 1 },
            SoundEffect::CastFailure,
        ]
    );

    let mut open = test_state(open_grid(), 1, 1);
    arm_spell(&mut open, OPEN_SPELL_INDEX);
    let serial = open.sound_effect_serial;
    assert_eq!(
        open.cast_directed_utility_spell(
            0,
            OPEN_SPELL_INDEX,
            spell_circle_for(OPEN_SPELL_INDEX as u8).unwrap(),
            Some(Direction::East),
            false,
        ),
        MoveOutcome::Blocked
    );
    assert_eq!(
        open.sound_effects_after(serial),
        vec![SoundEffect::CastFailure],
        "only Vanish sounds ahead of the tile test"
    );
}

#[test]
fn directed_utility_confirmation_dead_caster_refusal_stays_silent() {
    // The caster can die between the direction prompt and this confirmation
    // re-entry. The resulting `Who casts?` is a bare refusal that never
    // reaches the audio.md §8.3 failure tail, and audio.md §9 gives such a
    // refusal no acknowledgement sound - not even Vanish's commit variant.
    for spell_index in [
        VANISH_SPELL_INDEX,
        OPEN_SPELL_INDEX,
        MAGIC_LOCK_SPELL_INDEX,
        UNLOCK_MAGIC_SPELL_INDEX,
    ] {
        let mut state = test_state(open_grid(), 5, 5);
        state.combat_active = true;
        state.combat_actors[0] = CombatActorDescriptor::default();
        let serial = state.sound_effect_serial;

        assert_eq!(
            state.confirm_spent_directed_utility_spell(
                0,
                spell_index,
                Some(Direction::East),
                false,
            ),
            MoveOutcome::Blocked,
            "spell {spell_index}"
        );
        assert_eq!(state.message, "Who casts?");
        assert!(
            state.sound_effects_after(serial).is_empty(),
            "spell {spell_index} must not sound on a pure refusal"
        );
    }
}

#[test]
fn directed_open_sounds_variant_two_when_it_unlocks_a_chest() {
    // audio.md §6: variant 2 is "successful Open". Unlocking the chest under
    // the aimed cell is the route's other success arm, so it sounds.
    let mut state = test_state(open_grid(), 5, 5);
    state.combat_active = true;
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.active_objects.resize(3, ActiveObject::empty());
    state.active_objects[2] = ActiveObject {
        type_byte: COMBAT_DEFAULT_DEATH_DROP_TILE,
        tile: COMBAT_DEFAULT_DEATH_DROP_TILE,
        x: 6,
        y: 5,
        z: 7,
        aux1: 0xA5,
        ..ActiveObject::empty()
    };
    arm_spell(&mut state, OPEN_SPELL_INDEX);
    let serial = state.sound_effect_serial;

    assert_eq!(
        state.cast_directed_utility_spell(
            0,
            OPEN_SPELL_INDEX,
            OPEN_SPELL_COST,
            Some(Direction::East),
            false,
        ),
        MoveOutcome::Cast
    );

    assert_eq!(state.message, "Success!");
    assert_eq!(state.active_objects[2].aux1, 0x25);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 2 }]
    );
}

#[test]
fn dungeon_open_sounds_variant_two_only_after_the_chest_test() {
    // audio.md §6: variant 2 is "successful Open". The dungeon route's own
    // `Failed!` arm is reached when the aimed cell holds no chest, and
    // audio.md §8.3's only pre-success spell boundary is Vanish, so that
    // failure carries the cast-failure tail alone.
    let mut empty_cell = dungeon_state(open_dungeon_record(), 0, 4, 4);
    arm_spell(&mut empty_cell, OPEN_SPELL_INDEX);
    let serial = empty_cell.sound_effect_serial;
    assert_eq!(
        empty_cell
            .cast_open_spell(0, None, false, Path::new(""))
            .unwrap(),
        MoveOutcome::Blocked
    );
    assert_eq!(empty_cell.message, "Failed!");
    assert_eq!(
        empty_cell.sound_effects_after(serial),
        vec![SoundEffect::CastFailure]
    );

    let mut chest = dungeon_state(open_dungeon_record(), 0, 4, 4);
    chest.grid[dungeon_cell_index(0, 4, 4)] = 0x40;
    arm_spell(&mut chest, OPEN_SPELL_INDEX);
    let serial = chest.sound_effect_serial;
    assert_eq!(
        chest.cast_open_spell(0, None, false, Path::new("")).unwrap(),
        MoveOutcome::ContainerOpened
    );
    assert_eq!(
        chest.sound_effects_after(serial),
        vec![SoundEffect::SharedVariant { variant: 2 }]
    );
}

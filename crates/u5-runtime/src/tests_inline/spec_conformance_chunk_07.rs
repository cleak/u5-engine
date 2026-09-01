// Spec-conformance regressions for `play_state_impl/chunk_07.rs`.
//
// Each test names the published sentence it pins. Where an earlier engine
// reading was carried from a retracted spec revision, the test asserts the
// current text, not the retracted one.

/// `town-mode.md §10`: "Burning family, live tiles `0xBC` and `0x8F`. ...
/// The two ids are the fireplace and molten lava of
/// `catalogs/tile-catalog.md` Section 6; an earlier revision of this bullet
/// called them "the rune/lever family", which is withdrawn."
#[test]
fn town_burning_live_tiles_are_the_fireplace_and_molten_lava() {
    assert!(PlayState::is_town_burning_live_tile(
        PlayState::TOWN_BURNING_FIREPLACE_TILE
    ));
    assert!(PlayState::is_town_burning_live_tile(
        PlayState::TOWN_BURNING_LAVA_TILE
    ));
    assert_eq!(PlayState::TOWN_BURNING_FIREPLACE_TILE, 0xbc);
    assert_eq!(PlayState::TOWN_BURNING_LAVA_TILE, 0x8f);
    // The Stonegate script fills the whole live grid with the same id, which
    // is why its survivors keep burning afterwards.
    assert_eq!(
        PlayState::TOWN_BURNING_LAVA_TILE,
        STONEGATE_TRAPDOOR_GRID_TILE
    );
    // The trapdoor family is a different tile and does not overlap.
    assert!(!PlayState::is_town_burning_live_tile(
        TOWN_TRAPDOOR_LIVE_TILE
    ));
    for tile in [0x00u8, 0x04, 0x10, 0x90, 0x93, 0xff] {
        assert!(
            !PlayState::is_town_burning_live_tile(tile),
            "tile 0x{tile:02X} is not in the burning family",
        );
    }
}

/// `town-mode.md §10`: "Burning family, live tiles `0xBC` and `0x8F`.
/// Rebuild the view, print the stored line `Burning!`, then apply the same
/// independently rolled `1..8` mass damage to every non-Dead slot. These
/// tiles are damage tiles, not cosmetic ones."
#[test]
fn town_burning_underfoot_prints_burning_and_damages_every_non_dead_slot() {
    let dir = debug_game_dir();
    for tile in [
        PlayState::TOWN_BURNING_FIREPLACE_TILE,
        PlayState::TOWN_BURNING_LAVA_TILE,
    ] {
        let mut state = test_state(open_grid(), 5, 5);
        state.grid[5 * 32 + 5] = tile;
        for member in &mut state.party {
            member.max_hp = 200;
            member.hp = 100;
            member.status = b'G';
        }
        state.message.clear();

        state
            .apply_town_post_turn_effects_after_turn(state.turn.wrapping_sub(1), &dir)
            .unwrap();

        assert!(
            state.message.contains(PlayState::TOWN_BURNING_MESSAGE),
            "tile 0x{tile:02X} must print the stored `Burning!` line, got {:?}",
            state.message,
        );
        for (slot, member) in state.party.iter().enumerate() {
            assert!(
                (92..=99).contains(&member.hp),
                "slot {slot} on tile 0x{tile:02X} must take an independent 1..8 hit, got {}",
                member.hp,
            );
        }

        // "every non-Dead slot" — a Dead slot is skipped entirely.
        let mut dead_state = test_state(open_grid(), 5, 5);
        dead_state.grid[5 * 32 + 5] = tile;
        for member in &mut dead_state.party {
            member.max_hp = 200;
            member.hp = 0;
            member.status = CharacterStatus::Dead.save_byte();
        }
        dead_state.message.clear();

        dead_state
            .apply_town_post_turn_effects_after_turn(dead_state.turn.wrapping_sub(1), &dir)
            .unwrap();

        assert!(
            dead_state.message.contains(PlayState::TOWN_BURNING_MESSAGE),
            "the line still prints with an all-Dead party",
        );
        assert!(
            dead_state
                .party
                .iter()
                .all(|member| member.hp == 0 && member.status == CharacterStatus::Dead.save_byte()),
            "Dead slots take no burning damage",
        );
    }
    let _ = fs::remove_dir_all(dir);
}

/// `town-mode.md §10`: the underfoot handler "runs on **every** turn-consuming
/// action, not only on a committed step", so "the same cell keeps working
/// forever" — standing on lava burns again next turn.
#[test]
fn town_burning_underfoot_re_fires_every_consumed_turn() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    state.grid[5 * 32 + 5] = PlayState::TOWN_BURNING_LAVA_TILE;
    for member in &mut state.party {
        member.max_hp = 200;
        member.hp = 100;
        member.status = b'G';
    }

    let first_before = state.party[0].hp;
    state
        .apply_town_post_turn_effects_after_turn(state.turn.wrapping_sub(1), &dir)
        .unwrap();
    let after_one = state.party[0].hp;
    state.message.clear();
    state
        .apply_town_post_turn_effects_after_turn(state.turn.wrapping_sub(1), &dir)
        .unwrap();
    let after_two = state.party[0].hp;

    assert!(after_one < first_before, "the first turn burns");
    assert!(after_two < after_one, "standing still burns again");
    assert!(state.message.contains(PlayState::TOWN_BURNING_MESSAGE));
    let _ = fs::remove_dir_all(dir);
}

/// `town-mode.md §7.1` step 3 fills the live grid with molten lava, and
/// `§10` makes that id a damage tile — so the scripted-death survivors of a
/// later rescue stand on a burning grid.
#[test]
fn stonegate_scripted_death_leaves_the_party_standing_on_a_burning_grid() {
    let mut state = test_state(open_grid(), 4, 4);
    state.area = Area::Town {
        scene: Scene::new(STONEGATE_SCENE_BYTE).unwrap(),
        floor: 0,
    };

    state.apply_stonegate_trapdoor_script(0);

    assert!(
        state
            .grid
            .iter()
            .all(|tile| PlayState::is_town_burning_live_tile(*tile)),
        "every rewritten cell is in the burning family",
    );
}

/// `time.md §7`: "the candidate equals the party's current scene byte, or
/// the candidate equals the value currently stored in **any** of the three
/// slots, including the slot being rerolled and any slot already rewritten
/// earlier in the same pass."
///
/// This replaces the earlier engine reading, which discarded the party-scene
/// argument into `_current` and drew each slot exactly once with no rejection
/// set.
#[test]
fn shadowlord_midnight_reroll_rejects_the_party_scene_and_every_stored_slot() {
    for seed in 0u16..64 {
        let mut state = test_state(open_grid(), 5, 5);
        state.prng_state = seed.wrapping_mul(2477).wrapping_add(1);
        let party_scene = 3u8;
        state.shadowlord_hideouts = [1, 2, 8];
        let previous = state.shadowlord_hideouts;

        let rerolled = state.reroll_shadowlord_hideouts_excluding(Some(party_scene));

        assert_eq!(rerolled, SHADOWLORD_COUNT);
        let after = state.shadowlord_hideouts;
        for slot in 0..SHADOWLORD_COUNT {
            assert!(
                (SHADOWLORD_HIDEOUT_MIN..=SHADOWLORD_HIDEOUT_MAX).contains(&after[slot]),
                "seed {seed} slot {slot} must land on a town scene byte",
            );
            assert_ne!(
                after[slot], party_scene,
                "seed {seed} slot {slot} must not land in the party's own town",
            );
            assert_ne!(
                after[slot], previous[slot],
                "seed {seed}: a living Shadowlord never stays in the same town two days running",
            );
        }
        assert!(
            after[0] != after[1] && after[1] != after[2] && after[0] != after[2],
            "seed {seed}: no two living Shadowlords share a town, got {after:?}",
        );
    }
}

/// `time.md §7`: "Vanquished slots hold `0xFF` and never collide with a
/// `1..8` candidate, so they do not constrain the draw", and "the daily
/// walker skips any slot whose high bit is set, so vanquishing a Shadowlord
/// is sticky across future days."
#[test]
fn shadowlord_midnight_reroll_skips_only_high_bit_slots() {
    let mut state = test_state(open_grid(), 5, 5);
    state.shadowlord_hideouts = [SHADOWLORD_VANQUISHED, 2, SHADOWLORD_VANQUISHED];

    let rerolled = state.reroll_shadowlord_hideouts_excluding(None);

    assert_eq!(rerolled, 1, "only the one living slot is rewritten");
    assert_eq!(state.shadowlord_hideouts[0], SHADOWLORD_VANQUISHED);
    assert_eq!(state.shadowlord_hideouts[2], SHADOWLORD_VANQUISHED);
    assert_ne!(state.shadowlord_hideouts[1], 2);
    assert!(PlayState::shadowlord_slot_is_rerollable(0));
    assert!(PlayState::shadowlord_slot_is_rerollable(8));
    assert!(!PlayState::shadowlord_slot_is_rerollable(
        SHADOWLORD_VANQUISHED
    ));
}

/// `time.md §7`: "A slot value of `0` means "not yet placed". A newly created
/// game starts with all three slots at `0`, so no Shadowlord is anywhere
/// until the first midnight pass assigns hideouts. Implementations should
/// treat `0` as neither "in a town" nor "vanquished": it matches no town
/// scene, and the reroll walker rewrites it on the first day rollover."
///
/// The earlier engine reading gated the walker on the living `1..=8` range,
/// so a factory-zero save was never placed for the life of the game.
#[test]
fn shadowlord_unplaced_zero_slots_are_rerolled_on_the_first_day_rollover() {
    for seed in 0u16..32 {
        let mut state = test_state(open_grid(), 5, 5);
        state.prng_state = seed.wrapping_mul(9091).wrapping_add(7);
        state.shadowlord_hideouts = [0, 0, 0];

        // `0` is neither "in a town" nor "vanquished".
        assert!(!PlayState::shadowlord_slot_is_living(0));
        assert!(!PlayState::shadowlord_slot_is_vanquished(0));
        assert!(!state.all_shadowlords_vanquished());

        let rerolled = state.reroll_shadowlord_hideouts_excluding(None);

        assert_eq!(rerolled, SHADOWLORD_COUNT, "every unplaced slot is placed");
        let after = state.shadowlord_hideouts;
        for slot in 0..SHADOWLORD_COUNT {
            assert!(
                state.shadowlord_alive(slot),
                "seed {seed} slot {slot} is placed in a town, got {after:?}",
            );
        }
        assert!(
            after[0] != after[1] && after[1] != after[2] && after[0] != after[2],
            "seed {seed}: the first pass still yields three distinct towns, got {after:?}",
        );
    }
}

/// `blackthorn.md §3` step 2: "scan the eight shrine ruin flags in shrine
/// order and take the first whose flag is *exactly* clear — never ruined and
/// never restored."
#[test]
fn blackthorn_audience_selects_the_first_exactly_clear_shrine_flag() {
    let mut state = test_state(open_grid(), 5, 5);
    assert_eq!(state.blackthorn_selected_shrine(), Some(0));

    state.shrine_ruin_flags[0] = SAVE_QUEST_TILE_FLAG_HIGH_BIT;
    assert_eq!(state.blackthorn_selected_shrine(), Some(1));

    // A restored shrine has had only its ruin bit cleared, so its byte is
    // non-zero and it is skipped as "never restored" requires.
    state.shrine_ruin_flags[1] = SAVE_QUEST_TILE_FLAG_HIGH_BIT;
    state.shrine_ruin_flags[1] &= !SAVE_QUEST_TILE_FLAG_HIGH_BIT;
    state.shrine_ruin_flags[1] |= 0x01;
    assert_eq!(state.blackthorn_selected_shrine(), Some(2));

    // "If every flag is non-zero the whole audience is abandoned."
    state.shrine_ruin_flags = [SAVE_QUEST_TILE_FLAG_HIGH_BIT; SAVE_SHRINE_RUIN_FLAG_COUNT];
    assert_eq!(state.blackthorn_selected_shrine(), None);
}

/// `blackthorn.md §4`: "A correct answer ruins that shrine and costs five
/// points of moral standing. ... The moral-standing debit is a clamped
/// subtraction of five, floored at zero."
///
/// The §4 withdrawal reverses the earlier "the challenge does not directly
/// adjust numeric karma" reading and the earlier per-member jail-flag
/// reading: the flag set is a shrine's, not a party member's.
#[test]
fn blackthorn_correct_answer_ruins_the_shrine_and_debits_five_standing() {
    let mut state = test_state(open_grid(), 5, 5);
    state.moral_standing = 40;
    state.shrine_ruin_flags[3] = 0;

    assert!(state.apply_blackthorn_correct_answer_consequences(3));

    assert_eq!(
        state.shrine_ruin_flags[3] & SAVE_QUEST_TILE_FLAG_HIGH_BIT,
        SAVE_QUEST_TILE_FLAG_HIGH_BIT,
        "the shrine's durable ruin flag is set",
    );
    assert_eq!(state.moral_standing, 35);
    assert_eq!(PlayState::BLACKTHORN_CORRECT_ANSWER_STANDING_DEBIT, 5);

    // Floored at zero, not wrapped.
    state.moral_standing = 2;
    assert!(state.apply_blackthorn_correct_answer_consequences(4));
    assert_eq!(state.moral_standing, 0);

    assert!(!state.apply_blackthorn_correct_answer_consequences(SAVE_SHRINE_RUIN_FLAG_COUNT));
}

/// `blackthorn.md §8`: "Carried-key count zeroed by the audience cleanup |
/// Durable inventory effect of the capture", and the §8 retraction: the byte
/// once called a Blackthorn conversation signal "is the party's ordinary
/// carried-key counter ... and the audience's cleanup simply zeroes it, so
/// the party leaves the capture without its keys."
#[test]
fn blackthorn_audience_cleanup_zeroes_the_carried_key_count() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
    state.area = Area::Town { scene, floor: 0 };
    state.keys = 9;

    state
        .apply_blackthorn_captive_cell_handoff(&dir, "test")
        .unwrap();

    assert_eq!(
        state.keys, 0,
        "the guards take the party's keys at the audience cleanup",
    );
    let _ = fs::remove_dir_all(dir);
}

/// The same §8 row, reached through the live capture chain rather than the
/// cleanup helper directly: a capture that begins with a non-zero key count
/// ends with none.
#[test]
fn blackthorn_capture_chain_leaves_the_party_without_its_keys() {
    let dir = debug_game_dir();
    let mut state = test_state(open_grid(), 5, 5);
    let scene = Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).unwrap();
    state.area = Area::Town { scene, floor: 0 };
    state.keys = 4;
    state.moral_standing = 60;

    state.begin_blackthorn_audience_capture(&dir).unwrap();
    assert!(state.active_blackthorn.is_some());
    assert_eq!(state.keys, 4, "the keys survive until the cleanup");

    // Shrine zero is Honesty, whose accepted answer is `Ahm`.
    state
        .submit_blackthorn_audience_answer("Ahm", &dir)
        .unwrap();

    assert!(state.active_blackthorn.is_none());
    assert_eq!(state.keys, 0);
    assert_eq!(
        state.shrine_ruin_flags[0] & SAVE_QUEST_TILE_FLAG_HIGH_BIT,
        SAVE_QUEST_TILE_FLAG_HIGH_BIT,
        "the correct answer ruined the interrogated shrine",
    );
    assert_eq!(state.moral_standing, 55);
    let _ = fs::remove_dir_all(dir);
}

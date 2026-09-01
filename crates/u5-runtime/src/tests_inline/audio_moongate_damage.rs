// `systems/audio.md` trigger-boundary regressions: moongate damage.
//
// Each test names the published clause it pins. Add tests here rather
// than to the numbered chunks so the audio work stays reviewable as a
// unit.

#[test]
fn an_accepted_moongate_transit_emits_the_published_transit_envelope() {
    // `audio.md §8.3`: "Moongate transit | During an accepted transit, run
    // `(2, 2000, 30000, 1, 5900)`." The transit is the blocking stage-A /
    // stage-B transition of `overworld.md §9.2`, and one accepted transit
    // records exactly one envelope.
    let mut state = test_state(open_grid(), 4, 4);
    state.natural_moongate_counter = MOONGATE_PHASE_FULL;
    let serial = state.sound_effect_serial;

    let playback = state.play_natural_moongate_transit().unwrap();

    assert!(playback.ran_to_completion);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::MoongateTransit]
    );
    assert_eq!(state.sound_effect_serial, serial + 1);
}

#[test]
fn a_step_that_hands_off_no_moongate_cell_stays_silent() {
    // `audio.md §8.3`: "No destination handoff means no transit envelope."
    // The entry hook runs on every step; a step that reaches no live gate
    // cell returns before the transition and must not sound.
    let dir = Path::new(".");
    let mut state = test_state(open_grid(), 4, 4);
    let serial = state.sound_effect_serial;

    assert_eq!(state.resolve_natural_moongate_entry(dir).unwrap(), None);

    assert!(state.sound_effects_after(serial).is_empty());
    assert_eq!(state.sound_effect_serial, serial);
}

#[test]
fn a_gate_cell_whose_terrain_is_no_longer_live_stays_silent() {
    // Same clause, the second gate in the hook: the party stands on the
    // cell but its terrain has already been rewritten to `5`, so no
    // transition runs and nothing sounds.
    let dir = Path::new(".");
    let idx = world_cell_index(5, 5);
    let mut grid = open_world_grid();
    grid[idx] = NATURAL_MOONGATE_RESTORED_TERRAIN_TILE;
    let mut state = britannia_state(grid, 5, 5);
    state.natural_moongate_counter = MOONGATE_PHASE_FULL;
    let serial = state.sound_effect_serial;

    assert_eq!(state.resolve_natural_moongate_entry(dir).unwrap(), None);

    assert!(state.sound_effects_after(serial).is_empty());
}

#[test]
fn the_shared_damage_presentation_emits_one_rumble_per_damaged_member() {
    // `audio.md §8.2`: "Ordinary damage presentation | The shared damage
    // presentation runs the 160-update 100..2000 Hz rumble." The helper is
    // per-member, so a whole-party pass records one rumble per member it
    // actually damages and none for the members it skips.
    let mut state = test_state(open_grid(), 4, 4);
    let template = state.party[0];
    state.party.push(PartyMember {
        slot: 1,
        status: PARTY_STATUS_DEAD,
        hp: 0,
        ..template
    });
    state.party.push(PartyMember {
        slot: 2,
        ..template
    });
    let serial = state.sound_effect_serial;

    let applied = state.apply_outdoor_impact_party_damage();

    assert_eq!(
        applied.iter().map(|entry| entry.slot).collect::<Vec<_>>(),
        vec![0, 2],
        "the dead slot is skipped by the pass"
    );
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::DamageRumble, SoundEffect::DamageRumble]
    );
    assert_eq!(state.sound_effect_serial, serial + 2);
}

#[test]
fn a_whole_party_damage_pass_with_no_living_member_stays_silent() {
    // The other direction of the same clause: the presentation is what
    // sounds, so a pass that presents nothing sounds nothing.
    let mut state = test_state(open_grid(), 4, 4);
    state.party[0].status = PARTY_STATUS_DEAD;
    state.party[0].hp = 0;
    let serial = state.sound_effect_serial;

    assert!(state.apply_outdoor_impact_party_damage().is_empty());

    assert!(state.sound_effects_after(serial).is_empty());
}

#[test]
fn the_rumble_precedes_the_hit_point_write_it_presents() {
    // `audio.md §8.2` again: "Preserve the caller's own damage/narration
    // order." The helper flashes the member's row before subtracting from
    // the hit-point word, so the cue is recorded ahead of the write and
    // ahead of the death bookkeeping rather than after it.
    let mut state = test_state(open_grid(), 4, 4);
    state.party[0].hp = 3;
    state.active_player = Some(0);
    let serial = state.sound_effect_serial;

    let damage = state.apply_shared_party_damage(0, 9);

    assert!(damage.died);
    assert_eq!(state.party[0].hp, 0);
    assert_eq!(state.active_player, None);
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::DamageRumble],
        "one presentation, one rumble, even though the member also died"
    );
}

#[test]
fn hit_point_writes_outside_the_shared_presentation_stay_silent() {
    // `audio.md §8.2`: "this is not a global sound for every HP write."
    // The world damage-tile pass writes hit points directly instead of
    // going through the shared presentation helper, so it stays silent,
    // and so does the bare record write it uses.
    let mut state = britannia_state(open_world_grid(), 5, 5);
    let serial = state.sound_effect_serial;
    let hp_before = state.party[0].hp;

    let report = state.apply_world_damage_tile(WorldDamageTileEntry {
        plane: WorldPlane::Britannia,
        x: 5,
        y: 5,
        effect: WorldDamageEffect::Lava,
        expected_tile: None,
    });
    assert!(report.contains("took"));
    assert!(state.party[0].hp < hp_before);

    let _ = state.party[0].apply_damage(1);

    assert!(state.sound_effects_after(serial).is_empty());
    assert_eq!(state.sound_effect_serial, serial);
}

#[test]
fn the_transit_envelope_survives_the_destination_warp() {
    // `audio.md §8.3` ties the envelope to the accepted transit, and the
    // transit with a destination handoff is the one that warps. The warp
    // rebuilds the whole state from `PlayOptions` and keeps only the sound
    // history the destination scene load produced, so without the restore
    // in `resolve_natural_moongate_entry` the published cue would reach a
    // frontend on every path *except* the sanctioned one.
    let dir = debug_game_dir();
    let idx = world_cell_index(5, 5);
    let mut grid = open_world_grid();
    grid[idx] = NATURAL_MOONGATE_TERRAIN_TILE;
    let mut state = britannia_state(grid, 5, 5);
    state.clock = GameClock::new(11, 58).unwrap();
    state.natural_moongate_counter = MOONGATE_PHASE_FULL;
    state.set_cached_moon_glyph_slots(Some(1), None);
    state.moonstone_slots[1] = MoonstoneGateSlot {
        scene: 0,
        x: 6,
        y: 7,
        z: WorldPlane::Britannia.save_floor() as u8,
    };
    let serial = state.sound_effect_serial;

    assert_eq!(
        handle_play_key_input(&mut state, 'q', "", &dir).unwrap(),
        PlayInputDisposition::Continue
    );

    assert_eq!((state.player.x, state.player.y), (6, 7));
    assert_eq!(
        state.sound_effects_after(serial),
        vec![SoundEffect::MoongateTransit],
        "the accepted transit is recorded exactly once across the warp"
    );
    let _ = fs::remove_dir_all(dir);
}

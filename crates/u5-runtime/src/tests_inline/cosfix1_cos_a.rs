// Closed-audit cosmetic gaps (u5-spec#302), batch cos-a.

#[test]
fn cos_a_shared_party_damage_flashes_the_damaged_members_row() {
    // `time.md §5`: the shared party-damage path "plays the standard damage
    // feedback — a brief highlight of the affected member's roster row and a
    // short noise burst"; `overworld.md §6.2.4`: it "inverts the member's
    // panel row, plays the shared damage rumble, and inverts the row back".
    let mut state = test_state(open_grid(), 4, 4);
    while state.party.len() < 3 {
        let mut member = state.party[0];
        member.slot = state.party.len() as u8;
        state.party.push(member);
    }
    state.party[2].hp = 30;
    let serial = state.sound_effect_serial;

    let _ = state.apply_shared_party_damage(2, 1);

    let effects = state.sound_effects_with_serials_after(serial);
    assert_eq!(effects.len(), 1);
    assert_eq!(effects[0].1, SoundEffect::DamageRumble);
    assert_eq!(state.stats_row_flash_for_sound_effect(effects[0].0), Some(2));
    assert_eq!(
        state.presentation_stats_row_flash, None,
        "the frontend owns the flash while the rumble sounds"
    );
}

#[test]
fn cos_a_sleep_field_inverts_each_failing_members_row_and_poison_does_not() {
    // `dungeon-mode.md §8.1`: for each member whose sleep save fails, "that
    // member's stats-panel row is inverted (highlighted), then the short
    // status rumble"; the poison field plays "the same short status rumble
    // only, with no row highlight". Dexterity 1 always fails, 31 saves.
    for (field, flashes) in [
        (DungeonFieldEffect::Sleep, vec![Some(0), Some(1)]),
        (DungeonFieldEffect::PoisonGas, vec![None, None]),
    ] {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        let member = |slot, status, climb_stat| PartyMember {
            slot,
            class_byte: b'A',
            status,
            climb_stat,
            mana: 0,
            hp: 20,
            max_hp: 20,
            level: 1,
        };
        state.party = vec![
            member(0, b'G', 1),
            member(1, b'G', 1),
            member(2, b'D', 1),
            member(3, b'G', 31),
        ];
        let serial = state.sound_effect_serial;

        let _ = state.apply_dungeon_field_effect(field);

        let rows: Vec<Option<usize>> = state
            .sound_effects_with_serials_after(serial)
            .into_iter()
            .filter(|(_, effect)| *effect == SoundEffect::StatusFieldRumble)
            .map(|(serial, _)| state.stats_row_flash_for_sound_effect(serial))
            .collect();
        assert_eq!(rows, flashes, "{field:?}");
    }
}

#[test]
fn cos_a_a_flashing_roster_row_paints_inverted() {
    let mut state = test_state(open_grid(), 5, 5);
    while state.party.len() < 2 {
        let mut member = state.party[0];
        member.slot = state.party.len() as u8;
        state.party.push(member);
    }
    state.repaint_stats_panel();
    let paint = |state: &PlayState| {
        let mut system = TextWindowSystem::new();
        configure_play_text_windows(&mut system);
        paint_stats_panel_text_window(&mut system, state, None);
        let window = system.window(STATS_PANEL_TEXT_WINDOW_INDEX).unwrap();
        (0..2u8)
            .map(|row| {
                system
                    .cell(window.top_left_x + 1, window.top_left_y + row)
                    .unwrap()
                    .inverse
            })
            .collect::<Vec<_>>()
    };
    let plain = paint(&state);
    state.presentation_stats_row_flash = Some(1);
    let flashing = paint(&state);
    assert_eq!(flashing[0], plain[0], "other rows keep their style");
    assert_ne!(flashing[1], plain[1], "the flashing row is inverted");
}

#[test]
fn cos_a_a_rumbles_row_flash_survives_a_scene_rebuild() {
    // A scene rebuild carries the outgoing sound history across so a frontend
    // that reads after the transition still hears the rumble; the roster row
    // that rumble highlights (`time.md §5`) is keyed by the same serial and
    // must cross with it, not be dropped or aliased by the destination's own
    // freshly numbered history.
    let dir = debug_game_dir();
    let scene = Scene::new(17).unwrap();
    let mut grid = open_world_grid();
    grid[world_cell_index(10, 20)] = 7;
    let mut state = world_state(grid, 10, 20);
    while state.party.len() < 2 {
        let mut member = state.party[0];
        member.slot = state.party.len() as u8;
        state.party.push(member);
    }
    state.party[1].hp = 30;
    let serial = state.sound_effect_serial;
    let _ = state.apply_shared_party_damage(1, 1);

    state
        .enter_world_target(&dir, WorldPlane::Underworld, PlayTarget::Town(scene), true)
        .unwrap();

    let rows: Vec<(SoundEffect, Option<usize>)> = state
        .sound_effects_with_serials_after(serial)
        .into_iter()
        .map(|(serial, effect)| (effect, state.stats_row_flash_for_sound_effect(serial)))
        .collect();
    assert_eq!(rows.first(), Some(&(SoundEffect::DamageRumble, Some(1))), "{rows:?}");
    assert!(
        rows.iter().skip(1).all(|(_, row)| row.is_none()),
        "{rows:?}"
    );
    let _ = fs::remove_dir_all(dir);
}

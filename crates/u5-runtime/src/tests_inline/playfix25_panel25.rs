// Exploratory playtest S25 (stats panel): u5-spec-head fixes for P25-06 and
// P25-07.

/// P25-06: `stats-panel.md §6` - the gold slot's ship variant applies only
/// "When the scene is not combat-class"; `§2.1` lists "a non-combat scene"
/// among its conditions. A fight that started aboard a frigate kept
/// `Ship:NN` on the counters row for the whole fight.
#[test]
fn playtest_s25_p25_06_combat_aboard_a_frigate_shows_gold_not_the_hull() {
    let mut state = world_state(vec![BRIT_DEEP_WATER_TILE; WORLD_CELLS], 100, 100);
    state.player.transport = TransportState::Ship {
        type_byte: 168,
        tile: 168,
        sails_hoisted: false,
        hull: 77,
        skiffs: 1,
    };
    state.sync_player_object();
    state.food = 63;
    state.gold = 150;
    state.repaint_stats_panel();
    assert_eq!(state.displayed_stats_panel().counter_row, "F:63    Ship:77");

    state.combat_active = true;
    state.repaint_stats_panel();
    assert_eq!(state.displayed_stats_panel().counter_row, "F:63      G:150");

    // The post-combat repaint brings the hull back.
    state.combat_active = false;
    state.repaint_stats_panel();
    assert_eq!(state.displayed_stats_panel().counter_row, "F:63    Ship:77");
}

/// P25-07: `stats-panel.md §4` - current HP is "right-justified in a
/// four-column space-padded field" with no clamp, and `§3` gives the stats
/// window column 39. An edited five-digit HP printed as `9999`.
#[test]
fn playtest_s25_p25_07_party_row_hp_is_not_clamped_to_9999() {
    let mut state = test_state(open_grid(), 1, 1);
    state.party[0].hp = 12345;
    state.party[0].status = b'G';
    state.active_player = None;
    state.repaint_stats_panel();
    let row = state.displayed_stats_panel().rows[0].clone();
    assert!(row.ends_with(" 12345G"), "{row:?}");
    assert_eq!(row.chars().count(), STATS_PANEL_WIDTH + 1, "{row:?}");

    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    paint_stats_panel_text_window(&mut system, &state, state.active_player);
    let digits: Vec<u8> = (34..=38)
        .map(|column| system.cell(column, 1).unwrap().byte)
        .collect();
    assert_eq!(digits, b"12345");
    assert_eq!(system.cell(39, 1).unwrap().byte, b'G');

    // An in-range HP is still the published fifteen-cell row.
    state.party[0].hp = 60;
    state.repaint_stats_panel();
    let row = state.displayed_stats_panel().rows[0].clone();
    assert_eq!(row.chars().count(), STATS_PANEL_WIDTH, "{row:?}");
    assert!(row.ends_with("  60G"), "{row:?}");
}

/// P25-07: `inventory.md §4.7` - the attribute page "re-widens the window's
/// right edge to column 39" and prints HP, HM and Ex through the padded
/// number printer, which `text-output.md` says produces the requested width
/// "or wider, if the value's natural width exceeds the field". The fifth
/// digit of a five-digit value was dropped.
#[test]
fn playtest_s25_p25_07_attribute_page_prints_five_digit_values_in_full() {
    let mut state = test_state(open_grid(), 5, 5);
    state.party = vec![PartyMember {
        slot: 0,
        class_byte: b'A',
        status: b'G',
        climb_stat: 30,
        mana: 7,
        hp: 12345,
        max_hp: 12345,
        level: 8,
    }];
    state.party_strengths = vec![30];
    state.party_intelligence = vec![30];
    state.party_experience = vec![12345];
    let session = ZStatsSession::new(0);
    let rows = state.z_stats_panel_rows(&session);
    assert_eq!(rows[3], "Str=30  HP:12345");
    assert_eq!(rows[4], "Int=30  HM:12345");
    assert_eq!(rows[5], "Dex=30  Ex:12345");

    state.active_z_stats = Some(session);
    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    assert!(crate::stats_panel::paint_z_stats_page_text_window(&mut system, &state));
    for row in 4..=6 {
        assert_eq!(system.cell(38, row).unwrap().byte, b'4', "row {row}");
        assert_eq!(system.cell(39, row).unwrap().byte, b'5', "row {row}");
    }
}

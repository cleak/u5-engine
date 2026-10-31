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

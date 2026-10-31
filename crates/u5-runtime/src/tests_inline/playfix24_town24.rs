// Regressions for the pass-24 exploratory playtest findings (town24).
//
// Each test names the published sentence it pins and the playtest id whose
// reproduction it captures.

/// A guard-demand figure east of the party at `(1, 1)`, as the existing
/// reserved-dialog Talk tests build it.
fn playfix24_guard_demand_state() -> PlayState {
    let mut state = test_state(open_grid(), 1, 1);
    state.player.facing = Direction::East;
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: BLACKTHORN_GUARD_DEMAND_DIALOG_ID,
            schedule: [0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 8, 16, 20],
            name: None,
        },
    ]);
    state
}

/// Playtest P24-08. `town-mode.md §14`: explicit `T` on the reserved
/// guard-demand figure is one of the three routings into the arrest
/// sequence, which "branches on the current location before it prints
/// anything. Inside Lord Blackthorn's Castle ... it plays the Blackthorn
/// audience/capture cinematic"; the palace surrender fallback is withdrawn
/// (R442). The asset-backed Talk handed the arrest no game directory, so a
/// Badge-less explicit Talk asked `Wilt thou come quietly?` instead.
/// Skipped when `U5_TEST_ASSET_DIR` is unset.
#[test]
fn playtest_p24_08_explicit_talk_to_a_badgeless_palace_guard_opens_the_audience() {
    let Some(dir) = crate::test_fixtures::configured_original_asset_dir() else {
        return;
    };
    let mut state = playfix24_guard_demand_state();
    state.area = Area::Town {
        scene: Scene::new(SCENE_LORD_BLACKTHORNS_CASTLE).unwrap(),
        floor: 0,
    };

    state
        .talk_direction_with_game_dir(Direction::East, &dir)
        .expect("palace talk");

    assert!(state.pending_town_arrest.is_none());
    assert!(
        !state.message.contains("Wilt thou come quietly?"),
        "message: {:?}",
        state.message
    );
    assert!(state.blackthorn_capture.is_some());
}

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

/// Playtest P24-09. `town-mode.md §7` step 4: the Negate Time gate "also
/// jumps past post-action cleanup, so no contact, conflict or arrest can
/// begin on such a turn", and §14 hosts the explicit-T refusal's arrest in
/// that cleanup. A refused tribute under Negate Time opened the arrest.
#[test]
fn playtest_p24_09_negate_time_stops_the_explicit_talk_refusal_arrest() {
    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = playfix24_guard_demand_state();
    state.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    state.gold = 150;

    assert_eq!(
        state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
        MoveOutcome::Talked
    );
    assert_eq!(
        state.resolve_blackthorn_guard_demand_input('N', "", None),
        Some(MoveOutcome::Used)
    );
    assert!(state.active_blackthorn_guard_demand.is_none());
    assert!(state.pending_town_arrest.is_none());
    assert!(!state.message.contains("Thou art under arrest!"));
}

/// Playtest P24-09, Quickness arm. `town-mode.md §14`: "Under Quickness the
/// same holds on the alternate turns whose gate skips the walkers and
/// cleanup, so an arrest under Quickness can begin only on a non-skipped
/// turn." The engine arrested on both parities.
#[test]
fn playtest_p24_09_quickness_lets_the_explicit_talk_arrest_begin_only_on_a_run_turn() {
    let dialogue = HashMap::new();
    let raw = HashMap::new();
    let mut state = playfix24_guard_demand_state();
    state.active_effect_tag = Some(QUICKNESS_ACTIVE_EFFECT_TAG);
    state.active_effect_counter = 10;
    state.gold = 150;

    let mut arrested = Vec::new();
    for _ in 0..2 {
        state.pending_town_arrest = None;
        assert_eq!(
            state.talk_facing_with_dialogue_and_keyword_raw(&dialogue, &raw, None),
            MoveOutcome::Talked
        );
        let skipped = state.town_post_action_cleanup_skipped;
        state.resolve_blackthorn_guard_demand_input('N', "", None);
        assert_eq!(state.pending_town_arrest.is_some(), !skipped);
        arrested.push(state.pending_town_arrest.is_some());
    }
    assert_eq!(arrested.iter().filter(|&&arrest| arrest).count(), 1);
}

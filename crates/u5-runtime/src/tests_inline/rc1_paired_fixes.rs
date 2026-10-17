// Regressions for the rc1 full paired sweep of 2026-10-17. Each test names
// the stock capture it reproduces and the published sentence it rests on.

fn blackthorn_audience_fixture_map() -> MiscmapsCutsceneMap {
    let mut tiles = vec![0x44; MISCMAPS_CUTSCENE_ROWS * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS];
    // The audience grid's manacles under the two party actors' start cells.
    for (x, y) in [(3usize, 5usize), (7, 5), (0, 1)] {
        tiles[y * MISCMAPS_CUTSCENE_VISIBLE_COLUMNS + x] = 0x85;
    }
    MiscmapsCutsceneMap {
        record_index: BLACKTHORN_AUDIENCE_CUTSCENE_MAP_RECORD,
        tiles,
    }
}

/// `bt-correct` (artifact bt-correct-20261017-004820): on every audience
/// beat the stock game draws the Avatar at `(3,5)` and the second member at
/// `(7,5)` as the manacled prisoner. `blackthorn.md §6.3` puts slot 0 at
/// `(3,5)` and §5 the second-member actor at `(7,5)`; both cells are the
/// manacles `0x85`, and `visibility.md §8` stamps "one of `0x64..0x67`" for a
/// terrain-aware actor there. The world's slot-zero refresh must not drag
/// the Avatar actor off the cutscene grid to the party's town position.
#[test]
fn rc1_blackthorn_audience_party_actors_draw_as_manacled_prisoners() {
    let mut state = test_state(open_grid(), 12, 20);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.blackthorn_audience_map = Some(blackthorn_audience_fixture_map());
    state.install_blackthorn_audience_actors();

    state.sync_player_object();
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (3, 5),
        "the cutscene owns slot zero while the audience runs"
    );

    let prisoner = 0x164..=0x167usize;
    for slot in [0usize, 1] {
        let tile = state.blackthorn_audience_actor_atlas_tile(slot).unwrap();
        assert!(prisoner.contains(&tile), "slot {slot} drew {tile:#x}");
    }
    // A guard on cobble is stamped unchanged.
    assert_eq!(
        state.blackthorn_audience_actor_atlas_tile(6),
        Some(0x100 + usize::from(BLACKTHORN_GUARD_ACTOR_BYTE))
    );
    // The suppressed tableau draws nothing.
    assert_eq!(state.blackthorn_audience_actor_atlas_tile(8), None);

    // `visibility.md §8.1`: one fresh draw per selecting actor per pass,
    // and the frame follows the drawn variant.
    state.blackthorn_audience_actor_variants = [0; 9];
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..64 {
        state.run_blackthorn_audience_composite_pass();
        seen.insert(state.blackthorn_audience_actor_atlas_tile(0).unwrap());
    }
    assert!(seen.len() > 1, "the prisoner re-rolls its frame: {seen:?}");
    assert!(seen.iter().all(|tile| prisoner.contains(tile)));
}

/// Outside the audience the slot-zero refresh still runs.
#[test]
fn rc1_slot_zero_refresh_still_runs_outside_the_audience() {
    let mut state = test_state(open_grid(), 12, 20);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.install_blackthorn_audience_actors();
    state.sync_player_object();
    assert_eq!(
        (state.active_objects[0].x, state.active_objects[0].y),
        (12, 20)
    );
}

/// Play-test P22, `shops.md §8.C`: the fee question `Wilt thou\npay?" `
/// "accept[s] Y/N only and echo[es] bare `Yes` or `No`" - the declined `No`
/// lands after `pay?" ` on the prompt's own row, like `Yes`, not on a row of
/// its own; the continuation question follows.
#[test]
fn rc1_healer_declined_fee_echoes_no_on_the_prompt_row() {
    use crate::shop_runtime::*;
    use crate::shop_session::ActiveShopSession;
    use crate::shops::Healer;

    let mut state = test_state(open_grid(), 1, 1);
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Confirm {
            service: HealerService::Cure,
            slot: 0,
            cost: 10,
        },
        Healer::TheHealersMission,
    ));
    state.emit_message_line("\n\"I can cure thy poisoned body for 10 gold.\n\nWilt thou\npay?\" ");
    state.message_row_open_mid_line = true;
    handle_play_key_input(&mut state, 'N', "", Path::new("")).unwrap();
    state.flush_message_slot();
    let rows: Vec<&str> = state
        .message_transcript
        .iter()
        .map(|entry| entry.text.as_str())
        .collect();
    let at = rows
        .iter()
        .position(|row| *row == "pay?\" No")
        .unwrap_or_else(|| panic!("`No` is not on the prompt row: {rows:#?}"));
    assert!(!rows.contains(&"No"), "{rows:#?}");
    assert_eq!(
        rows[at + 1],
        "",
        "one blank row before the continuation: {rows:#?}"
    );
    assert_eq!(
        rows[at + 2],
        "\"Is there any other way in which I may",
        "{rows:#?}"
    );
    assert!(
        state.message.ends_with("aid thee?\" "),
        "{:?}",
        state.message
    );
}

/// `audio.md §8.9`, drowning: `Ship sunk!`, the panel refresh, **then** the
/// long descent, then `DROWNING!!!`, then the death loop, whose passes each
/// play their own damage rumble. Stock capture (`sail-heading-echo`, beat
/// `sailing`, 2026-10-17): about four seconds after the sinking the window
/// still ends on `Ship sunk!` with the whole party listed, and the hit points
/// fall over the following samples. The engine printed `DROWNING!!!` and the
/// dead party at once.
#[test]
fn rc1_drowning_holds_ship_sunk_through_the_descent_then_paces_the_loop() {
    let mut state = world_state(open_world_grid(), 5, 5);
    state.party = six_member_party(40);
    state.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 0;
    aboard_frigate(&mut state, 1, 0);
    let full_row = crate::stats_panel::stats_panel_snapshot_of(&state).rows[0].clone();
    let OutdoorImpactAbsorption::ShipDestroyed { drowning, .. } = state.apply_outdoor_impact()
    else {
        panic!("hull one is always destroyed");
    };
    assert!(drowning.len() > 1, "a hardy party survives several passes");
    let last_row = |state: &PlayState| {
        state
            .message_entries()
            .last()
            .map(|entry| entry.text.clone())
            .unwrap_or_default()
    };
    // Through the descent: `Ship sunk!` is the last row and the panel still
    // shows the party whole, although the live roster is already dead.
    let descent = SoundEffect::LongDescent
        .program(&mut crate::audio::RumbleJitter::default())
        .duration(true)
        .as_secs_f64();
    assert!(descent > 6.0);
    assert!(state.advance_staged_narration_to(100.0));
    assert!(state.advance_staged_narration_to(100.0 + descent - 0.1));
    assert_eq!(last_row(&state), SHIP_SUNK_MESSAGE);
    assert_eq!(state.displayed_stats_panel().rows[0], full_row);
    assert!(state.party.iter().all(|member| member.hp == 0));

    // Then `DROWNING!!!`, and the passes repaint the panel one at a time.
    assert!(state.advance_staged_narration_to(100.0 + descent + 0.01));
    assert_eq!(last_row(&state), DROWNING_MESSAGE);
    let mut shown = vec![state.displayed_stats_panel().rows[0].clone()];
    let mut now = 100.0 + descent + 0.01;
    while state.staged_narration_active() {
        now += 0.05;
        state.advance_staged_narration_to(now);
        let row = state.displayed_stats_panel().rows[0].clone();
        if shown.last() != Some(&row) {
            shown.push(row);
        }
    }
    assert!(
        shown.len() > 2,
        "the hit points fall pass by pass: {shown:?}"
    );
    assert!(now - (100.0 + descent) > 0.5, "the loop takes time");
    assert_eq!(state.displayed_stats_panel(), &state.stats_panel);
}

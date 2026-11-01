// u5-spec#302 closed re-audit, `systems/blackthorn.md` (a3-bt).

/// Re-audit blackthorn:7.1:5, `blackthorn.md §7.1` "What paces the fall": an
/// implementation "must not express the fall, or a hold after it, as a
/// BIOS-tick count". The hold is the recipe's own length, not that length
/// rounded up to whole ticks.
#[test]
fn auditfix3_bt_7_1_5_stonegate_hold_is_the_recipe_length_not_ticks() {
    use crate::audio::RumbleJitter;
    let tick = crate::narration::BIOS_TICK_SECS;
    let mut off_tick = 0;
    for members in 1..=6usize {
        let mut state = test_state(open_grid(), 5, 5);
        while state.party.len() < members {
            let companion = audit_g5_companion(state.party.len() as u8);
            state.party.push(companion);
        }
        state.party.truncate(members);
        let serial = state.sound_effect_serial;
        state.apply_stonegate_trapdoor_script_after(0, serial);
        let mut jitter = RumbleJitter::new();
        let sound: f64 = state
            .sound_effects_after(serial)
            .iter()
            .map(|effect| effect.program(&mut jitter).duration(true).as_secs_f64())
            .sum();
        let hold = state.staged_narration.beats().next().expect("hold").delay_secs;
        assert!((hold - sound).abs() < 1e-9, "{members}: {hold} vs {sound}");
        if ((sound / tick) - (sound / tick).round()).abs() > 1e-6 {
            off_tick += 1;
        }
    }
    assert!(off_tick > 0, "some recipe length is not a whole tick count");
}

/// Re-audit blackthorn:5:3, `blackthorn.md §5` execution step 4: "the
/// impact figure is drawn, the 300-update impact rumble plays, and one world
/// tick follows." The tick follows the rumble, ahead of step 5's actor clear
/// and step 6's buffered pendulum write.
#[test]
fn auditfix3_bt_5_3_execution_impact_is_followed_by_one_world_tick() {
    let dir = debug_game_dir();
    lit18_write_merciful_death_record(&dir);
    let mut state = test_state(open_grid(), 5, 5);
    state.party.push(audit_g5_companion(1));
    state.party.push(audit_g5_companion(2));
    state.repaint_stats_panel();
    state.begin_blackthorn_audience_capture(&dir).unwrap();
    while state.active_blackthorn.as_ref().is_some_and(|c| c.current_prompt().is_none()) {
        state.submit_blackthorn_audience_answer("", &dir).unwrap();
    }
    state.submit_blackthorn_audience_answer("Ahm", &dir).unwrap();
    state.advance_staged_narration_to(0.0);
    state.advance_staged_narration_to(crate::narration::bios_ticks_secs(10) + 0.01);
    assert!(
        state.sound_effects_after(0).contains(&SoundEffect::RoughSeasImpactRumble),
        "the impact rumble has played"
    );
    let frame_before = state.animation.frame;
    assert_eq!(state.party.len(), 3, "the blade hold is still running");
    state.flush_staged_narration();
    assert_eq!(state.party.len(), 2, "the hold's end lifts the companion");
    assert_eq!(
        state.animation.frame,
        (frame_before + 1) % crate::animation::STATIC_TILE_ANIMATION_PERIOD_TICKS,
        "exactly one world tick follows the impact rumble"
    );
    let _ = fs::remove_dir_all(dir);
}

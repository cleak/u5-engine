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

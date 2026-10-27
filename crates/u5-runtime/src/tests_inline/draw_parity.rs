// Random-stream parity: which half of each gate's byte passes
// (u5-spec#303 Q5). The published rates were already right; the stream
// position each gate selects was not.
mod draw_parity {
    use super::*;

    /// Seed whose first `random(0, 255)` result equals `value`.
    fn seed_drawing(value: u8) -> u16 {
        (0u16..=u16::MAX)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 255) == u16::from(value)
            })
            .expect("every byte is reachable")
    }

    /// `npc-schedules.md §9.1` stage 1: bit 3 set passes, and a clear bit
    /// "takes no direction draw".
    #[test]
    fn the_wander_coin_tests_bit_three_set() {
        for value in 0..=255u8 {
            assert_eq!(town_npc_wander_coin_passes(value), value & 8 != 0, "{value}");
        }
        // 1 has bit 0 set and bit 3 clear: the low-bit test let it through.
        let mut state = test_state(open_grid(), 5, 5);
        state.prng_state = seed_drawing(1);
        assert!(!state.town_npc_wander_gate_passes());
        // 8 is even with bit 3 set: the low-bit test refused it.
        state.prng_state = seed_drawing(8);
        assert!(state.town_npc_wander_gate_passes());
    }

    /// `active-objects.md §3`: "the snap is taken when the result is below 64",
    /// "the script restarts when the result is below 192", and the coin
    /// "accepted at 128 or above" - whole-byte thresholds, executed on the
    /// original at 0, 63, 64, 255 / 0, 191, 192, 255 / 0, 127, 128, 255.
    #[test]
    fn the_animator_gates_are_whole_byte_thresholds() {
        for (value, snap) in [(0u8, true), (63, true), (64, false), (255, false)] {
            assert_eq!(animation_snap_taken(value), snap, "snap {value}");
        }
        for (value, restart) in [(0u8, true), (191, true), (192, false), (255, false)] {
            assert_eq!(animation_six_step_restarts(value), restart, "six-step {value}");
        }
        for (value, pass) in [(0u8, false), (127, false), (128, true), (255, true)] {
            assert_eq!(animation_eligibility_coin_passes(value), pass, "coin {value}");
        }
        // The values the old low-bit masks decided the other way.
        assert!(!animation_snap_taken(64), "64 % 4 == 0 but is not below 64");
        assert!(animation_snap_taken(1), "1 % 4 != 0 but is below 64");
        assert!(!animation_six_step_restarts(193), "193 % 4 != 0 but is not below 192");
        assert!(animation_six_step_restarts(4), "4 % 4 == 0 but is below 192");
    }

    /// The animator's snap step on a live record: class `0x5C` takes no coin,
    /// so its first draw is the snap's. A draw of 64 must not snap.
    #[test]
    fn a_snap_draw_of_64_advances_instead_of_snapping() {
        let record = ActiveObject {
            type_byte: 0x5c,
            tile: 0x5f,
            x: 3,
            y: 2,
            z: 0,
            phase: 0x30,
            aux1: 0,
            aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
        };
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[4] = record;
        state.prng_state = seed_drawing(64);
        state.animate_active_objects();
        assert_ne!(state.active_objects[4].tile, 0x5c, "64 is not below 64");

        state.active_objects[4] = record;
        state.prng_state = seed_drawing(63);
        state.animate_active_objects();
        assert_eq!(state.active_objects[4].tile, 0x5c, "63 snaps");
    }

    /// `karma.md §7`: the approach walk and the exit are counted world steps
    /// at one one-tick delay each. Re-measured with synced keys
    /// (`shrine-enter-pacing`, 2026-10-26): the original kneels by `t2400`,
    /// where the serial-fitted 72-tick hold kneeled at `t3600`. The mantra
    /// holds are key-anchored too and were re-measured the same way
    /// (`shrine-three-mantras` reaction probe, 2026-10-26): the stock reacts
    /// about 0.35 s after the virtue Return and 0.70 s after each mantra
    /// Return, six and twelve ticks. The Kneel-to-question hold, an interval
    /// between two stock records, was re-measured with a finer sampler the
    /// same day: 0.60 to 0.66 s, eleven ticks for its ten steps.
    #[test]
    fn shrine_walk_and_exit_cost_one_tick_a_step() {
        assert_eq!(SHRINE_KNEEL_HOLD_BIOS_TICKS, SHRINE_KNEEL_WORLD_STEPS);
        assert_eq!(SHRINE_KNEEL_HOLD_BIOS_TICKS, 46);
        assert_eq!(SHRINE_EXIT_PACING_HOLD_BIOS_TICKS, 61);
        for steps in [0u16, 1, 40, 51, 60, 61, 66] {
            assert_eq!(shrine_presentation_hold_bios_ticks(steps), steps);
        }
        // Key to Kneel record: 46 ticks is 2.53 s, inside the stock's
        // (1.8 s, 2.4 s nominal + screenshot cost] window, and well short of
        // the 3.6 s the serial-fitted hold gave.
        let kneel_secs = f64::from(SHRINE_KNEEL_HOLD_BIOS_TICKS) * crate::narration::BIOS_TICK_SECS;
        assert!((2.4..2.7).contains(&kneel_secs), "{kneel_secs}");
        assert_eq!(SHRINE_VIRTUE_QUESTION_HOLD_WORLD_TICKS, 11);
        let question =
            f64::from(SHRINE_VIRTUE_QUESTION_HOLD_WORLD_TICKS) * crate::narration::BIOS_TICK_SECS;
        assert!((0.59..0.66).contains(&question), "{question}");
        assert_eq!(SHRINE_MANTRA_FIRST_HOLD_BIOS_TICKS, SHRINE_MANTRA_FIRST_WORLD_STEPS);
        assert_eq!(SHRINE_MANTRA_FIRST_HOLD_BIOS_TICKS, 6);
        assert_eq!(SHRINE_MANTRA_SUBSEQUENT_HOLD_BIOS_TICKS, SHRINE_MANTRA_SUBSEQUENT_WORLD_STEPS);
        assert_eq!(SHRINE_MANTRA_SUBSEQUENT_HOLD_BIOS_TICKS, 12);
        // The re-measured stock reactions, less a ~55 ms display latency:
        // about 0.35 s and 0.70 s.
        let first = f64::from(SHRINE_MANTRA_FIRST_HOLD_BIOS_TICKS) * crate::narration::BIOS_TICK_SECS;
        let later = f64::from(SHRINE_MANTRA_SUBSEQUENT_HOLD_BIOS_TICKS) * crate::narration::BIOS_TICK_SECS;
        assert!((0.30..0.40).contains(&first), "{first}");
        assert!((0.62..0.75).contains(&later), "{later}");
    }
}

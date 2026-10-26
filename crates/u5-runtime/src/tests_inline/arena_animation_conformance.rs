// Arena animation conformance: the decision-point arm of the per-slot
// animator (`animation.md §4`/`§5`, `active-objects.md §3`/`§8`) as it
// behaves on the records combat setup places.
//
// Scope note: these are animator tests only. `visibility.md §11`'s "the
// post-pass is [called]" on the combat blat-copy branch is **not**
// implemented - see `copy_combat_terrain_to_visibility_buffers` - because
// this engine's arena rasteriser (`render_combat_viewport`) does not consume
// the visibility buffers, so there is nothing here to certify about it.
//
// The cadence numbers quoted here come from a black-box capture of the shipped
// game under DOSBox: an outdoor sixteen-bat arena, 200 PrintWindow samples over
// 7.97 s, one ROI per arena cell. Nineteen of the 121 cells change - the
// sixteen bats, two party sprites, and the third party cell under the blinking
// turn cursor. Each of the eighteen non-cursor cells visits four (a few five)
// tiles, at 1.73 to 2.74 ticks between changes, mean 2.05, against
// `animation.md §2`'s 54.9254 ms step.
mod arena_animation_conformance {
    use super::*;

    /// `active-objects.md §3` gate 3 / `animation.md §4` case 4: the
    /// decision-point arm rewrites the displayed frame one step through the
    /// slot's own family and wraps.
    #[test]
    fn the_decision_point_frame_step_walks_each_family_and_wraps() {
        // A four-frame monster family: the bat, active-object type `0x94`.
        let mut tile = 0x94;
        let mut seen = Vec::new();
        for _ in 0..8 {
            seen.push(tile);
            tile = active_object_next_frame_tile(0x94, tile).expect("the bat has a family");
        }
        assert_eq!(seen, vec![0x94, 0x95, 0x96, 0x97, 0x94, 0x95, 0x96, 0x97]);

        // The same rule from a non-base starting frame.
        assert_eq!(active_object_next_frame_tile(0x94, 0x97), Some(0x94));

        // `combat_class_sprite_byte` is `class * 4 + 0x40`, so the party's own
        // combat sprites are the first four such groups. Capture shows them
        // cycling four frames exactly like a monster.
        for class_byte in [b'M', b'B', b'F', b'A'] {
            let base = combat_party_actor_byte(class_byte);
            assert_eq!(base & 0x03, 0, "{} is a family base", base);
            assert_eq!(active_object_next_frame_tile(base, base), Some(base + 1));
            assert_eq!(active_object_next_frame_tile(base, base + 3), Some(base));
        }

        // The two-frame pairs above `0xC0` toggle.
        assert_eq!(active_object_next_frame_tile(0xC0, 0xC0), Some(0xC1));
        assert_eq!(active_object_next_frame_tile(0xC0, 0xC1), Some(0xC0));

        // `animation.md §4` case 4's "Some slots do nothing": no family, no
        // rewrite - and, in the animator, no draw either.
        assert_eq!(active_object_next_frame_tile(0x20, 0x20), None);
        assert_eq!(active_object_next_frame_tile(PLAYER_TILE, PLAYER_TILE), None);
    }

    /// `animation.md §5` "One family, named": the beggar run `0x6C..0x6F`
    /// steps base -> first -> second -> third, and from the third either
    /// re-bases (one in four) and holds the base six ticks, or rewinds to the
    /// first. The base is never entered from anywhere but the third, and the
    /// type byte reads the base throughout.
    #[test]
    fn the_beggar_family_runs_its_published_script() {
        let mut state = arena_animation_state(&[(3, arena_record(0x6C, 4, 4, 0))]);
        state.prng_state = 0x1234;
        let mut transitions = std::collections::BTreeSet::new();
        let mut base_runs = Vec::new();
        let mut run = 0usize;
        let mut previous = state.active_objects[3].tile;
        let mut entered_base = false;
        for _ in 0..10_000 {
            state.animate_active_objects();
            let tile = state.active_objects[3].tile;
            assert_eq!(state.active_objects[3].type_byte, 0x6C);
            assert!((0x6C..=0x6F).contains(&tile));
            if tile != previous {
                transitions.insert((previous, tile));
                if entered_base {
                    base_runs.push(run);
                }
                entered_base = tile == 0x6C;
                run = 0;
            }
            run += 1;
            previous = tile;
        }
        assert_eq!(
            transitions.into_iter().collect::<Vec<_>>(),
            vec![
                (0x6C, 0x6D),
                (0x6D, 0x6E),
                (0x6E, 0x6F),
                (0x6F, 0x6C),
                (0x6F, 0x6D),
            ]
        );
        // The six-tick hold, then the coin: the base is held at least seven
        // passes (six held, then at least the deciding pass itself).
        assert!(!base_runs.is_empty());
        assert!(
            base_runs.iter().all(|run| *run > usize::from(BEGGAR_BASE_HOLD_TICKS)),
            "base runs {base_runs:?}"
        );
    }

    /// Issue #298 Q29 / `RETRACTIONS.md` R535: the eleven per-class scripts
    /// of `active-objects.md §3`, their class map and the excluded bands.
    #[test]
    fn the_per_class_script_table_is_the_published_one() {
        use AnimationScriptStep::*;
        for class in [0x94u8, 0x9c, 0xa0, 0xc0, 0xc8, 0xd0, 0xd8] {
            assert_eq!(active_object_animation_script(class), Some(ANIMATION_SCRIPT_B));
            assert_eq!(active_object_animation_script(class | 3), Some(ANIMATION_SCRIPT_B));
        }
        assert_eq!(active_object_animation_script(0xf0), Some(ANIMATION_SCRIPT_C));
        assert_eq!(active_object_animation_script(0xb0), Some(ANIMATION_SCRIPT_D));
        assert_eq!(active_object_animation_script(0xa8), Some(ANIMATION_SCRIPT_E));
        assert_eq!(active_object_animation_script(0x70), Some(ANIMATION_SCRIPT_F));
        assert_eq!(active_object_animation_script(0x98), Some(ANIMATION_SCRIPT_G));
        assert_eq!(active_object_animation_script(0xdc), Some(ANIMATION_SCRIPT_H));
        assert_eq!(active_object_animation_script(0xe4), Some(ANIMATION_SCRIPT_H));
        assert_eq!(active_object_animation_script(0xf4), Some(ANIMATION_SCRIPT_I));
        assert_eq!(active_object_animation_script(0x8c), Some(ANIMATION_SCRIPT_J));
        assert_eq!(active_object_animation_script(0xb8), Some(ANIMATION_SCRIPT_K));
        for class in [0x34u8, 0x40, 0x5c, 0x6c, 0x78, 0x80, 0xec, 0xfc] {
            assert_eq!(active_object_animation_script(class), Some(ANIMATION_SCRIPT_A));
        }
        for class in [0x00u8, 0x2c, 0x30, 0x33, 0xb4, 0xb7, 0xe8, 0xeb] {
            assert_eq!(active_object_animation_script(class), None, "{class:#x}");
        }
        assert_eq!(ANIMATION_SCRIPT_A, &[Frame(2), Frame(3), Frame(4), Snap, Restart]);
        assert_eq!(
            ANIMATION_SCRIPT_J,
            &[Frame(2), Pause(2), Frame(3), Pause(2), Frame(4), Pause(2), SixStep, Frame(1), Restart]
        );
        assert_eq!(ANIMATION_SCRIPT_I.last(), Some(&GotoTwo));
        assert!(active_object_animates_unconditionally(0x5c));
        assert!(active_object_animates_unconditionally(0xa9));
        assert!(!active_object_animates_unconditionally(0x94));
    }

    /// Issue #298 Q29: "The mimic and the corpser show frame 1 and then
    /// freeze on their p15 step"; p15 "is the freeze sentinel". The mimic
    /// takes no coin, so the run is deterministic and draw-free.
    #[test]
    fn the_mimic_shows_frame_one_then_freezes_without_drawing() {
        let mut state = arena_animation_state(&[(4, arena_record(0xa8, 3, 2, 0))]);
        state.active_objects[4].tile = 0xab;
        let seed = state.prng_state;
        state.animate_active_objects();
        assert_eq!(state.active_objects[4].tile, 0xa8, "f1");
        assert_eq!(state.active_objects[4].phase, 0x10);
        state.animate_active_objects();
        assert_eq!(state.active_objects[4].phase, 0x20 | STEADY_PHASE, "p15 freezes");
        for _ in 0..20 {
            state.animate_active_objects();
        }
        assert_eq!(state.active_objects[4].tile, 0xa8);
        assert_eq!(state.active_objects[4].phase, 0x20 | STEADY_PHASE);
        assert_eq!(state.prng_state, seed, "the mimic is exempt from the coin");
    }

    /// Issue #298 Q29: "p*n* - set the frame-delay nibble to *n* ... The next
    /// *n* passes only decrement the delay" (the gazer's p7, R535's "value
    /// less 128", not "less eight"); a restart continues in the same pass.
    #[test]
    fn a_pause_step_holds_the_frame_for_its_delay_and_restart_continues() {
        // The gazer runs script D (f1 f2 f3 f4 p7 restart); position 4 is
        // its p7. Pin the eligibility coin high by searching seeds.
        let mut state = arena_animation_state(&[(4, arena_record(0xb0, 3, 2, 0x40))]);
        state.active_objects[4].tile = 0xb3;
        state.prng_state = (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 255) >= 128
            })
            .unwrap();
        state.animate_active_objects();
        assert_eq!(state.active_objects[4].phase, 0x57, "p7 sets the delay to 7");
        assert_eq!(state.active_objects[4].tile, 0xb3, "the frame is unchanged");
        for remaining in (0..7u8).rev() {
            state.animate_active_objects();
            assert_eq!(state.active_objects[4].phase, 0x50 | remaining);
            assert_eq!(state.active_objects[4].tile, 0xb3);
        }
    }

    /// Issue #298 Q29: for class `0x5C` an accepted snap sets the frame to
    /// the type byte and "the position advances instead of pausing"; for
    /// every other class "the delay is set to six and the position is held".
    #[test]
    fn an_accepted_snap_holds_six_ticks_except_for_class_0x5c() {
        let accepted = (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 255) < 64
            })
            .unwrap();
        let mut state = arena_animation_state(&[(4, arena_record(0x5c, 3, 2, 0x30))]);
        state.active_objects[4].tile = 0x5f;
        state.prng_state = accepted;
        state.animate_active_objects();
        assert_eq!(state.active_objects[4].tile, 0x5c);
        assert_eq!(state.active_objects[4].phase, 0x40);

        // A coin-gated class: one coin draw, then the snap draw.
        let seed = (1u16..)
            .find(|seed| {
                let mut probe = *seed;
                u5_prng_range_u16(&mut probe, 0, 255) >= 128
                    && u5_prng_range_u16(&mut probe, 0, 255) < 64
            })
            .unwrap();
        let mut state = arena_animation_state(&[(4, arena_record(0x50, 3, 2, 0x30))]);
        state.active_objects[4].tile = 0x53;
        state.prng_state = seed;
        state.animate_active_objects();
        assert_eq!(state.active_objects[4].tile, 0x50);
        assert_eq!(state.active_objects[4].phase, 0x30 | ANIMATION_SNAP_HOLD_TICKS);
    }

    fn arena_animation_state(records: &[(usize, ActiveObject)]) -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        for (slot, object) in records.iter().copied() {
            state.active_objects[slot] = object;
        }
        state
    }

    fn arena_record(type_byte: u8, x: usize, y: usize, phase: u8) -> ActiveObject {
        ActiveObject {
            type_byte,
            tile: type_byte,
            x,
            y,
            z: 0,
            phase,
            aux1: 0,
            aux3: COMBAT_ACTIVE_OBJECT_NO_DESCRIPTOR,
        }
    }

    /// `animation.md §5`: "one common form is a **fair coin** that decides
    /// whether this slot's animation advances at all this pass."
    ///
    /// The coin is drawn from the shared gameplay stream, which
    /// `visibility.md §8.4` lists as the *first* of the three per-tick
    /// consumers an engine "must reproduce ... in this order": "The
    /// **active-object animator**, first."
    ///
    /// Both arms are pinned here off a known PRNG state, so a future change to
    /// the number of draws per slot is a test failure rather than a silent
    /// stream shift.
    #[test]
    fn a_decision_point_slot_advances_its_frame_only_on_the_coin() {
        // Issue #298 Q29: the coin "is an inclusive `0..255` draw accepted
        // at 128 or above". Find one seed on each side of it.
        let mut heads = None;
        let mut tails = None;
        for seed in 1u16..512 {
            let mut probe = seed;
            let draw = u5_prng_range_u16(&mut probe, 0, 255);
            if draw >= 128 && heads.is_none() {
                heads = Some(seed);
            }
            if draw < 128 && tails.is_none() {
                tails = Some(seed);
            }
        }
        let heads = heads.expect("some seed draws high");
        let tails = tails.expect("some seed draws low");

        // The bat (`0x94`) runs script B, f1 f2 f3 f4 restart; at position
        // 1 the next step is f2.
        for (seed, expected_tile, expected_phase, expected_dirty) in
            [(heads, 0x95u8, 0x20u8, true), (tails, 0x94u8, 0x10u8, false)]
        {
            let mut state = arena_animation_state(&[(4, arena_record(0x94, 3, 2, 0x10))]);
            state.prng_state = seed;
            state.visibility_dirty = false;

            state.animate_active_objects();

            assert_eq!(state.active_objects[4].tile, expected_tile);
            assert_eq!(state.visibility_dirty, expected_dirty);
            // Exactly one draw, whichever way the coin fell: a frame step
            // takes none of its own.
            assert_eq!(state.prng_state, u5_prng_advance_state(seed));
            // The animator never writes a coordinate: `animation.md §5`
            // R316, "**It cannot change the slot's map position.**"
            assert_eq!((state.active_objects[4].x, state.active_objects[4].y), (3, 2));
            // Byte 6's high nibble is the script position, advanced by the
            // frame step that ends the pass.
            assert_eq!(state.active_objects[4].phase, expected_phase);
        }
    }

    /// `active-objects.md §3` gate 1: an all-ones low nibble makes the animator
    /// "bail immediately, **writing nothing**". A slot with no frame family is
    /// `animation.md §4` case 4's "Some slots do nothing". Neither may cost a
    /// draw - `visibility.md §8.1` charges draws only to slots that reach an
    /// arm, and the same discipline applies to the animator's own coin.
    #[test]
    fn steady_empty_and_family_less_slots_cost_the_animator_no_draw() {
        let mut state = arena_animation_state(&[
            (4, arena_record(0x94, 3, 2, STEADY_PHASE)),
            (5, arena_record(0x20, 4, 2, 0)),
        ]);
        state.prng_state = 0x0070;
        state.visibility_dirty = false;

        state.animate_active_objects();

        assert_eq!(state.prng_state, 0x0070, "no slot here reaches the coin");
        assert_eq!(state.active_objects[4].tile, 0x94);
        assert_eq!(state.active_objects[5].tile, 0x20);
        assert!(!state.visibility_dirty);
    }

    /// `active-objects.md §8` early-out 4 (u5-spec#300 Q16, R608): the
    /// animator skips a slot whose frame byte is exactly `0x00`, `0x1D` (a
    /// party combatant's invisibility frame) or `0x1E` (a party combatant's
    /// asleep frame); "No other frame value is exempt". A sleeping party
    /// combatant's record keeps its asleep frame and draws no coin.
    #[test]
    fn spec300_q16_asleep_and_invisible_party_frames_are_parked() {
        let party = combat_party_actor_byte(b'F');
        for parked in [COMBAT_POTION_SLEEP_DISPLAY_TILE, COMBAT_POTION_INVISIBLE_WAKE_DISPLAY_TILE] {
            let mut record = arena_record(party, 4, 4, 0);
            record.tile = parked;
            let mut state = arena_animation_state(&[(1, record)]);
            state.prng_state = 0x1234;
            let prng_before = state.prng_state;
            for _ in 0..64 {
                state.animate_active_objects();
                assert_eq!(state.active_objects[1].tile, parked);
            }
            assert_eq!(state.prng_state, prng_before, "{parked:#04x} drew no coin");
        }
    }
}

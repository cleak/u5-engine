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
        // Find one seed whose next draw is the advancing outcome and one whose
        // next draw is not, so both arms are exercised against the real
        // generator rather than a stub.
        let mut heads = None;
        let mut tails = None;
        for seed in 1u16..512 {
            let mut probe = seed;
            let flip = u5_prng_range_u16(&mut probe, 0, 1);
            if flip == 1 && heads.is_none() {
                heads = Some(seed);
            }
            if flip == 0 && tails.is_none() {
                tails = Some(seed);
            }
        }
        let heads = heads.expect("some seed flips heads");
        let tails = tails.expect("some seed flips tails");

        for (seed, expected_tile, expected_dirty) in
            [(heads, 0x95u8, true), (tails, 0x94u8, false)]
        {
            let mut state = arena_animation_state(&[(4, arena_record(0x94, 3, 2, 0))]);
            state.prng_state = seed;
            state.visibility_dirty = false;

            state.animate_active_objects();

            assert_eq!(state.active_objects[4].tile, expected_tile);
            assert_eq!(state.visibility_dirty, expected_dirty);
            // Exactly one draw, whichever way the coin fell.
            assert_eq!(state.prng_state, u5_prng_advance_state(seed));
            // The animator never writes a coordinate: `animation.md §5`
            // R316, "**It cannot change the slot's map position.**"
            assert_eq!((state.active_objects[4].x, state.active_objects[4].y), (3, 2));
            // Byte 6 is left alone. **This pins an inference, not the spec.**
            // `active-objects.md §3` gate 3 says the decision point "**may**
            // advance the script step and rewrite the byte" and
            // `animation.md §5` says it "**can** reseed the phase counter";
            // neither says when, and this engine never does. The consequence
            // is that a placed combat record sits at a decision point for the
            // whole fight, which is what sets the measured cadence. If the
            // spec later fixes a reseed rule, this assertion is the thing
            // that has to change.
            assert_eq!(state.active_objects[4].phase, 0);
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
}

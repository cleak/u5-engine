    // ------------------------------------------------------------------
    // `systems/animation.md` Section 6 — global tile animation.
    //
    // Provenance for every test in this block: `animation.md §6` at spec
    // HEAD `c00bf63`, corroborated by `catalogs/tile-catalog.md §3.1`
    // and `§4` at the same HEAD.
    //
    // §6 retracts the family list this engine previously implemented:
    // "no water, lava, brazier or torch tile animates through this pass
    // at all." The five families below are the complete list.
    // ------------------------------------------------------------------

    /// `animation.md §6` (spec HEAD `c00bf63`) publishes exactly five
    /// families with these id ranges and cycle lengths:
    ///
    /// | Tile ids | Family | Behaviour |
    /// |---|---|---|
    /// | `0xD4..0xD7` | Waterfall | four-frame, every tick, ungated |
    /// | `0xD8..0xDB` | Fountain | four-frame, every tick, ungated |
    /// | `0x80..0x83` | Pendulum | two-frame toggle in adjacent pairs, bit-0 gate |
    /// | `0xEC..0xEF` | The standard of Britannia | four-frame, bit-0 gate |
    /// | `0xFA..0xFD` | Grandfather clock `0xFA..0xFB`, bellows `0xFC..0xFD` | two-frame toggles, bit-1 gate nested in bit-0 |
    ///
    /// `catalogs/tile-catalog.md §3.1` independently confirms
    /// "waterfall family `0xD4..0xD7`", "the fountain band
    /// `0xD8..0xDB`" and "grandfather clock `0xFA..0xFB`" from the
    /// shipped description table.
    #[test]
    fn static_tile_animation_families_match_the_spec_table() {
        assert_eq!(
            STATIC_TILE_ANIMATION_FAMILIES.len(),
            5,
            "animation.md §6: 'There are exactly five such families'"
        );

        let expected: [(StaticTileAnimationFamily, u8, u8, u8); 5] = [
            (StaticTileAnimationFamily::Waterfall, 0xD4, 4, 4),
            (StaticTileAnimationFamily::Fountain, 0xD8, 4, 4),
            (StaticTileAnimationFamily::Pendulum, 0x80, 4, 2),
            (StaticTileAnimationFamily::StandardOfBritannia, 0xEC, 4, 4),
            (StaticTileAnimationFamily::ClockAndBellows, 0xFA, 4, 2),
        ];

        for (family, first_id, id_count, cycle) in expected {
            let spec = family.spec();
            assert_eq!(spec.family, family);
            assert_eq!(spec.first_id, first_id, "{family:?} first id");
            assert_eq!(spec.id_count, id_count, "{family:?} id count");
            assert_eq!(spec.cycle, cycle, "{family:?} cycle length");

            for offset in 0..id_count {
                let tile = first_id + offset;
                assert_eq!(
                    static_tile_animation_family(tile),
                    Some(family),
                    "tile 0x{tile:02x} must belong to {family:?}"
                );
            }
            // The id just below and just above the published range is
            // not part of the family.
            assert_ne!(
                static_tile_animation_family(first_id - 1),
                Some(family),
                "0x{:02x} is outside {family:?}",
                first_id - 1
            );
            if let Some(above) = first_id.checked_add(id_count) {
                assert_ne!(
                    static_tile_animation_family(above),
                    Some(family),
                    "0x{above:02x} is outside {family:?}"
                );
            }
        }

        // Twenty animated ids total, and nothing else in 0..=255.
        let animated: Vec<u8> = (0u8..=0xFF)
            .filter(|tile| static_tile_animation_family(*tile).is_some())
            .collect();
        assert_eq!(animated.len(), 20, "five families of four adjacent ids");
    }

    /// `animation.md §6`: "advance the waterfall family, advance the
    /// fountain family, then test bit 0 of the shared phase counter. If
    /// bit 0 is clear, the pass skips **everything** that follows —
    /// pendulum, flag and clock/bellows alike — and goes straight to
    /// incrementing the counter. If bit 0 is set, the pendulum and the
    /// flag advance, and only then is bit 1 tested; the clock/bellows
    /// family advances only when both bits are set."
    #[test]
    fn static_tile_animation_gates_are_nested_not_independent() {
        for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
            let pass = static_tile_animation_pass(phase);

            assert!(pass.waterfall, "phase {phase}: waterfall is ungated");
            assert!(pass.fountain, "phase {phase}: fountain is ungated");

            if phase & 0x01 == 0 {
                assert!(
                    !pass.pendulum && !pass.standard_of_britannia && !pass.clock_and_bellows,
                    "phase {phase}: a clear bit 0 must skip pendulum, flag AND clock/bellows"
                );
            } else {
                assert!(pass.pendulum, "phase {phase}: bit 0 set advances pendulum");
                assert!(
                    pass.standard_of_britannia,
                    "phase {phase}: the flag is gated exactly as the pendulum is"
                );
                assert_eq!(
                    pass.clock_and_bellows,
                    phase & 0x02 != 0,
                    "phase {phase}: bit 1 is only read inside the bit-0 gate"
                );
            }
        }

        // The nesting, stated as a truth table over the two bits. Phase
        // 2 has bit 1 set and bit 0 clear: a lone bit-1 test would
        // advance the clock here, and the nested gate must not.
        assert!(
            !static_tile_animation_pass(2).clock_and_bellows,
            "phase 2 (bit 1 set, bit 0 clear) must not advance clock/bellows"
        );
        assert!(
            !static_tile_animation_pass(6).clock_and_bellows,
            "phase 6 (bit 1 set, bit 0 clear) must not advance clock/bellows"
        );
        assert!(static_tile_animation_pass(3).clock_and_bellows);
        assert!(static_tile_animation_pass(7).clock_and_bellows);
    }

    /// `animation.md §6`: "Two families — waterfall and fountain —
    /// advance on every tick. The pendulum and the flag advance on every
    /// second tick... The clock/bellows family advances on every fourth
    /// tick." Net rates 1x / 2x / 4x, measured over one full period.
    #[test]
    fn static_tile_animation_net_rates_are_one_half_and_quarter() {
        let period = u32::from(STATIC_TILE_ANIMATION_PERIOD_TICKS);
        let advances = |family: StaticTileAnimationFamily| {
            (0..STATIC_TILE_ANIMATION_PERIOD_TICKS)
                .filter(|phase| static_tile_animation_pass(*phase).advances(family))
                .count() as u32
        };

        assert_eq!(advances(StaticTileAnimationFamily::Waterfall), period);
        assert_eq!(advances(StaticTileAnimationFamily::Fountain), period);
        assert_eq!(advances(StaticTileAnimationFamily::Pendulum), period / 2);
        assert_eq!(
            advances(StaticTileAnimationFamily::StandardOfBritannia),
            period / 2,
            "the flag advances at half rate, NOT every tick"
        );
        assert_eq!(
            advances(StaticTileAnimationFamily::ClockAndBellows),
            period / 4,
            "quarter rate, as a consequence of bit 0 AND bit 1"
        );
    }

    /// `animation.md §6`: "Each id inside a family owns its own selector
    /// byte, so the four ids of a four-frame family are permanently a
    /// quarter-cycle apart and a wall of waterfall cells does not
    /// flicker in lockstep."
    #[test]
    fn four_frame_family_ids_stay_a_quarter_cycle_apart() {
        for first_id in [0xD4u8, 0xD8, 0xEC] {
            for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
                let clock = AnimationClock::at_static_tile_phase(phase);
                let resolved: Vec<u8> = (0..4)
                    .map(|offset| clock.resolve_static_tile(first_id + offset))
                    .collect();

                // All four ids display four distinct frames at once.
                let mut sorted = resolved.clone();
                sorted.sort_unstable();
                sorted.dedup();
                assert_eq!(
                    sorted.len(),
                    4,
                    "0x{first_id:02x} family at phase {phase}: the four ids must not share a frame, got {resolved:02x?}"
                );

                // And they stay exactly one step apart, in order.
                for offset in 0..4usize {
                    let base = resolved[0];
                    let expected =
                        first_id + ((base - first_id) + offset as u8) % 4;
                    assert_eq!(
                        resolved[offset], expected,
                        "0x{first_id:02x} family at phase {phase}: id +{offset} must be a quarter-cycle past the first"
                    );
                }
            }
        }
    }

    /// `animation.md §6`: the pendulum and the clock/bellows families
    /// are "two-frame toggle[s] in adjacent pairs" — `0x80..0x81` and
    /// `0x82..0x83`; grandfather clock `0xFA..0xFB` and bellows
    /// `0xFC..0xFD`. A pair never leaks a frame into its neighbour.
    #[test]
    fn paired_toggle_families_toggle_inside_their_adjacent_pair() {
        for pair_base in [0x80u8, 0x82, 0xFA, 0xFC] {
            for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
                let clock = AnimationClock::at_static_tile_phase(phase);
                let low = clock.resolve_static_tile(pair_base);
                let high = clock.resolve_static_tile(pair_base + 1);

                assert!(
                    low == pair_base || low == pair_base + 1,
                    "0x{pair_base:02x} must stay inside its pair, got 0x{low:02x}"
                );
                assert!(
                    high == pair_base || high == pair_base + 1,
                    "0x{:02x} must stay inside its pair, got 0x{high:02x}",
                    pair_base + 1
                );
                assert_ne!(
                    low, high,
                    "0x{pair_base:02x} pair at phase {phase}: each id owns its own selector, so the two are always opposite"
                );
            }
        }

        // The pendulum's two pairs are independent runs: 0x82 never
        // displays 0x80 or 0x81.
        for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
            let clock = AnimationClock::at_static_tile_phase(phase);
            assert!(clock.resolve_static_tile(0x82) >= 0x82);
            assert!(clock.resolve_static_tile(0xFC) >= 0xFC);
        }
    }

    /// `animation.md §6` (spec HEAD `c00bf63`) retraction: "**no water,
    /// lava, brazier or torch tile animates through this pass at all.**"
    /// `catalogs/tile-catalog.md §4` withdraws the same list plus a
    /// "wind / gust visuals" row.
    ///
    /// Withdrawn ids checked here: water `0x01..0x03` and swamp `0x04`
    /// (`tile-catalog.md §3`), the wells/brazier/fireplace band
    /// `0x5C..0x5F` (§3 row "92..95"), the fire-effect / poison / sleep
    /// band `0x98..0x9F` (§3 row "152..159"), and molten lava `0x8F`.
    #[test]
    fn withdrawn_water_lava_brazier_and_torch_ids_resolve_to_themselves() {
        let withdrawn: Vec<u8> = (0x01u8..=0x04)
            .chain(0x5C..=0x5F)
            .chain(std::iter::once(0x8F))
            .chain(0x98..=0x9F)
            .collect();

        for tile in withdrawn {
            assert_eq!(
                static_tile_animation_family(tile),
                None,
                "0x{tile:02x} is not a member of any animation.md §6 family"
            );
            for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
                assert_eq!(
                    AnimationClock::at_static_tile_phase(phase).resolve_static_tile(tile),
                    tile,
                    "0x{tile:02x} must resolve to itself at phase {phase}"
                );
            }
        }

        // The moon gate sits directly above the fountain band and is
        // "not animated at all" per `tile-catalog.md §4`.
        for phase in 0..STATIC_TILE_ANIMATION_PERIOD_TICKS {
            assert_eq!(
                AnimationClock::at_static_tile_phase(phase).resolve_static_tile(0xDC),
                0xDC,
                "moon gate 0xDC must not animate through the §6 pass"
            );
        }
    }

    /// `catalogs/tile-catalog.md §4`: "Moongate graphics are **not**
    /// animated at all." The moongate presence counter is not a member
    /// of the `animation.md §6` families and the tile-animation tick
    /// must never advance it.
    #[test]
    fn static_tile_tick_does_not_advance_the_moongate_counter() {
        let mut clock = AnimationClock::default();
        for _ in 0..(STATIC_TILE_ANIMATION_PERIOD_TICKS * 3) {
            clock.tick_static_tiles();
            assert_eq!(
                clock.moongate_frame, 0,
                "tick_static_tiles must leave the moongate presence counter alone"
            );
        }
    }

    /// `animation.md §6`: "The counter is incremented once at the end of
    /// the pass, whichever path was taken." Eight ticks returns every
    /// family's selectors to phase zero, and nothing shorter does.
    #[test]
    fn static_tile_animation_period_is_eight_ticks() {
        assert_eq!(STATIC_TILE_ANIMATION_PERIOD_TICKS, 8);

        let animated: Vec<u8> = (0u8..=0xFF)
            .filter(|tile| static_tile_animation_family(*tile).is_some())
            .collect();
        let phase_zero: Vec<u8> = animated.clone();

        let mut clock = AnimationClock::default();
        for tick in 1..=STATIC_TILE_ANIMATION_PERIOD_TICKS {
            clock.tick_static_tiles();
            let resolved: Vec<u8> = animated
                .iter()
                .map(|tile| clock.resolve_static_tile(*tile))
                .collect();
            if tick == STATIC_TILE_ANIMATION_PERIOD_TICKS {
                assert_eq!(clock.frame, 0, "the counter wraps at the layer's period");
                assert_eq!(
                    resolved, phase_zero,
                    "after {tick} ticks every selector is back at its authored id"
                );
            } else {
                assert_ne!(
                    resolved, phase_zero,
                    "the layer must not return to phase zero after only {tick} ticks"
                );
            }
        }
    }

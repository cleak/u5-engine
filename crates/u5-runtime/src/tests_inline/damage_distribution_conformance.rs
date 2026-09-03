    // `systems/combat.md` §9 conformance for the monster AI's target scan,
    // its movement planner, and the draw budget each dispatch spends. These
    // are the three things that decide *which* party member a swarm's damage
    // lands on, so they are grouped here with the statistical share test the
    // side-by-side capture motivated.

    /// The published seating for the three-member party in the side-by-side
    /// capture: combat slots 0, 1 and 2 at arena `(5,7)`, `(4,8)` and `(6,8)`.
    /// Slots 1 and 2 are mirror images about the column slot 0 stands in, which
    /// is what makes them a clean probe of the §9 tie-break.
    const PUBLISHED_SEATING: [(u8, u8); 3] = [(5, 7), (4, 8), (6, 8)];

    fn seated_party_candidates() -> Vec<CombatTargetCandidateView> {
        let mut candidates = vec![
            CombatTargetCandidateView {
                descriptor: CombatActorDescriptor::empty(),
                group: COMBAT_TARGET_GROUP_MONSTER,
                suppressed: false,
                invisible_or_unrevealed: false,
            };
            COMBAT_ACTOR_SLOTS
        ];
        for (slot, (x, y)) in PUBLISHED_SEATING.into_iter().enumerate() {
            candidates[slot] = CombatTargetCandidateView {
                descriptor: CombatActorDescriptor::from_row([
                    20,
                    12,
                    COMBAT_ACTOR_FLAG_SELECTABLE_80,
                    slot as u8,
                    slot as u8,
                    0,
                    x,
                    y,
                ]),
                group: COMBAT_TARGET_GROUP_PARTY,
                suppressed: false,
                invisible_or_unrevealed: false,
            };
        }
        candidates
    }

    fn place_scanning_bat(
        candidates: &mut [CombatTargetCandidateView],
        slot: usize,
        x: u8,
        y: u8,
    ) {
        candidates[slot] = CombatTargetCandidateView {
            descriptor: CombatActorDescriptor::from_row([
                5,
                5,
                COMBAT_ACTOR_FLAG_SELECTABLE_40,
                COMBAT_CLASS_BAT,
                slot as u8,
                0,
                x,
                y,
            ]),
            group: COMBAT_TARGET_GROUP_MONSTER,
            suppressed: false,
            invisible_or_unrevealed: false,
        };
    }

    /// Independent restatement of the `combat.md §9` outcome: closest by
    /// truncated linear Euclidean distance, and "*the lowest-numbered slot
    /// among candidates of equal distance wins*".
    ///
    /// It is deliberately formulated the opposite way round from the engine's
    /// scan - an **ascending** walk keeping a **strictly** closer candidate -
    /// so agreeing with [`find_combat_ai_target`], which walks descending and
    /// keeps `<=`, is evidence about the rule rather than a transcription of
    /// the implementation.
    fn published_target_rule(from: (u8, u8)) -> usize {
        let mut best: Option<(usize, u8)> = None;
        for (slot, &(x, y)) in PUBLISHED_SEATING.iter().enumerate() {
            let range = combat_arena_range(from.0, from.1, x, y);
            match best {
                Some((_, best_range)) if range >= best_range => {}
                _ => best = Some((slot, range)),
            }
        }
        best.expect("three seated candidates").0
    }

    #[test]
    fn ai_target_scan_gives_a_distance_tie_to_the_lowest_numbered_slot() {
        // `combat.md §9`: "*the lowest-numbered slot among candidates of equal
        // distance wins*, biasing toward party members (low slots) when
        // distances tie." Arena cell (5,10) is two cells from seat 1 at (4,8)
        // and two from seat 2 at (6,8) - a genuine tie - and three from seat 0,
        // so the tie is decided between the two mirrored seats alone and the
        // published rule gives it to seat 1.
        let mut candidates = seated_party_candidates();
        place_scanning_bat(&mut candidates, COMBAT_PARTY_ACTOR_SLOTS, 5, 10);

        let pick = find_combat_ai_target(
            &candidates,
            COMBAT_PARTY_ACTOR_SLOTS,
            COMBAT_TARGET_GROUP_MONSTER,
            false,
        );

        assert_eq!(
            combat_arena_range(5, 10, 4, 8),
            combat_arena_range(5, 10, 6, 8),
            "the probe cell has to be a genuine tie for the two mirrored seats"
        );
        assert_eq!(pick.slot, Some(1));
        assert!(pick.first_five_party_slot_survived);
    }

    #[test]
    fn ai_target_scan_still_prefers_a_strictly_closer_lower_slot() {
        // The tie-break must not become "always the highest slot": a strictly
        // closer candidate wins from any slot. Seat 0 at (5,7) is one cell from
        // (5,6) while both other seats are two.
        let mut candidates = seated_party_candidates();
        place_scanning_bat(&mut candidates, COMBAT_PARTY_ACTOR_SLOTS, 5, 6);

        let pick = find_combat_ai_target(
            &candidates,
            COMBAT_PARTY_ACTOR_SLOTS,
            COMBAT_TARGET_GROUP_MONSTER,
            false,
        );

        assert_eq!(pick.slot, Some(0));
    }

    #[test]
    fn per_seat_target_share_over_the_whole_arena_matches_the_published_rule() {
        // The statistical form of the two cases above, over every cell a
        // monster can occupy in the eleven-by-eleven arena. Two things are
        // asserted: every single pick equals an independent restatement of the
        // §9 rule, and the resulting per-seat share is the one that rule
        // implies for the published seating.
        //
        // Seats 1 and 2 are mirror images about column five, so a rule with no
        // tie-break preference would split the arena between them evenly, and
        // seat 0 would take only the cells strictly closest to it. The §9
        // tie-break instead sends every tie to the lower slot, which is what
        // produces both the dominant seat-0 share and the seat-1-over-seat-2
        // asymmetry asserted below.
        //
        // No claim is made here about the *original's* observed per-seat loss:
        // the paired side-by-side capture (43/8/22 across seats 0/1/2 over
        // twenty keys) is not reproduced by either tie-break direction, and
        // the residual is recorded as an open spec question rather than used
        // as evidence for one of them.
        let mut share = [0usize; 3];
        let mut cells = 0usize;
        for y in 0..COMBAT_ARENA_SIDE as u8 {
            for x in 0..COMBAT_ARENA_SIDE as u8 {
                if PUBLISHED_SEATING.contains(&(x, y)) {
                    continue;
                }
                let mut candidates = seated_party_candidates();
                place_scanning_bat(&mut candidates, COMBAT_PARTY_ACTOR_SLOTS, x, y);
                let pick = find_combat_ai_target(
                    &candidates,
                    COMBAT_PARTY_ACTOR_SLOTS,
                    COMBAT_TARGET_GROUP_MONSTER,
                    false,
                );
                let expected = published_target_rule((x, y));
                assert_eq!(
                    pick.slot,
                    Some(expected),
                    "cell ({x},{y}) diverges from the published §9 rule"
                );
                share[expected] += 1;
                cells += 1;
            }
        }

        assert_eq!(cells, COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE - 3);
        assert_eq!(share.iter().sum::<usize>(), cells);
        // The exact share is a regression lock on the tie-break direction: a
        // highest-slot-wins scan moves every mirrored tie from seat 1 to seat 2
        // and every seat-0 tie away from seat 0, giving [41, 36, 41] instead.
        assert_eq!(share, [74, 24, 20]);
        assert!(
            share[1] > share[2],
            "the §9 tie-break has to favour the lower of the two mirrored seats"
        );
    }

    fn adjacent_bat_dispatch_state() -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_round_loop_prologue_ran = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state
            .active_objects
            .resize(COMBAT_ACTOR_SLOTS, ActiveObject::empty());
        state.party = vec![PartyMember {
            slot: 0,
            class_byte: 1,
            status: b'G',
            climb_stat: 0,
            mana: 0,
            hp: 40,
            max_hp: 60,
            level: 1,
        }];
        state.combat_actors[0] =
            CombatActorDescriptor::from_row([40, 12, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 7]);
        state
    }

    fn place_dispatch_bat(state: &mut PlayState, slot: usize, x: u8, y: u8) {
        state.combat_actors[slot] = CombatActorDescriptor::from_row([
            5,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_40,
            COMBAT_CLASS_BAT,
            slot as u8,
            0,
            x,
            y,
        ]);
        state.active_objects[slot] = ActiveObject {
            type_byte: 0x94,
            tile: 0x94,
            x: usize::from(x),
            y: usize::from(y),
            ..ActiveObject::empty()
        };
    }

    /// How many shared-PRNG draws moved the state from `before` to `after`.
    /// Every draw is one state advance regardless of its range, so the count
    /// is recoverable by replaying advances.
    fn shared_prng_draws_between(before: u16, after: u16) -> usize {
        let mut state = before;
        for count in 0..64 {
            if state == after {
                return count;
            }
            state = u5_prng_advance_state(state);
        }
        panic!("state {after:#06x} is not within 64 draws of {before:#06x}");
    }

    #[test]
    fn an_adjacent_bat_dispatch_spends_only_the_attempt_draws_the_spec_charges_it() {
        // `combat.md §11`: the to-hit roll "happens inside the shared to-hit
        // helper, which is entered once per accepted attempt". `§12` gives the
        // poison gate only to "classes with the poison/status attack flag
        // cluster" - the Bat has none - and `§9` puts the axis-priority coin
        // inside ordinary stepping, which an attacking dispatch never reaches.
        // So the whole dispatch is the to-hit draw plus the defence draw the
        // damage roller takes against a non-zero defence rating.
        let mut state = adjacent_bat_dispatch_state();
        place_dispatch_bat(&mut state, COMBAT_PARTY_ACTOR_SLOTS, 5, 6);
        state.prng_state = 0x1234;

        let before = state.prng_state;
        let application = state
            .apply_combat_ai_turn(COMBAT_PARTY_ACTOR_SLOTS)
            .expect("an adjacent bat dispatches an attack");

        assert_eq!(application.attack_route, Some(CombatAiAttackRoute::Melee));
        assert!(application.monster_attack.is_some());
        assert_eq!(shared_prng_draws_between(before, state.prng_state), 2);
    }

    #[test]
    fn a_stepping_bat_dispatch_spends_only_the_axis_priority_coin() {
        // The same budget from the other side: a bat with an open direct axis
        // reaches ordinary stepping, whose only draw is `§9`'s "randomized axis
        // priority" coin. No to-hit draw is taken for an attack that never
        // happened, and the random-cardinal fallback is not entered.
        let mut state = adjacent_bat_dispatch_state();
        place_dispatch_bat(&mut state, COMBAT_PARTY_ACTOR_SLOTS, 5, 3);
        state.prng_state = 0x1234;

        let before = state.prng_state;
        let application = state
            .apply_combat_ai_turn(COMBAT_PARTY_ACTOR_SLOTS)
            .expect("a distant bat dispatches a step");

        assert_eq!(
            application.attack_route,
            Some(CombatAiAttackRoute::OutOfRange)
        );
        assert!(application.monster_attack.is_none());
        assert_eq!(shared_prng_draws_between(before, state.prng_state), 1);
        assert_eq!(
            (
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].x,
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].y
            ),
            (5, 4)
        );
    }

    #[test]
    fn the_immobile_classes_are_the_reaper_and_the_mimic() {
        // `combat.md §9` "The two classes refused outright": the movement and
        // teleport arm "returns immediately for two classes, the **Reaper** and
        // the **Mimic**, which are immobile by design".
        assert!(combat_ai_class_never_moves(COMBAT_CLASS_REAPER));
        assert!(combat_ai_class_never_moves(COMBAT_CLASS_MIMIC));
        assert!(!combat_ai_class_never_moves(COMBAT_CLASS_BAT));
        assert!(!combat_ai_class_never_moves(COMBAT_CLASS_DAEMON));
    }

    #[test]
    fn an_immobile_class_dispatch_never_steps_and_takes_no_movement_draw() {
        // The frame-level half of the rule. The Mimic is the class that can
        // actually reach the movement arm: its range/effect selector is two, so
        // a target four cells away is out of range and the dispatch falls
        // through to movement. (The Reaper's selector is nine, which covers
        // every cell pair in an eleven-by-eleven arena, so it always resolves
        // an attack instead - the predicate above is what guards it.)
        let mut state = adjacent_bat_dispatch_state();
        place_dispatch_bat(&mut state, COMBAT_PARTY_ACTOR_SLOTS, 5, 3);
        state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].owner_target_class = COMBAT_CLASS_MIMIC;
        state.prng_state = 0x1234;

        let before = state.prng_state;
        let application = state
            .apply_combat_ai_turn(COMBAT_PARTY_ACTOR_SLOTS)
            .expect("an immobile class still dispatches");

        assert_eq!(
            application.movement,
            Some(CombatAiMovementOutcome::Blocked {
                random_cardinal_attempts: 0
            })
        );
        assert_eq!(
            (
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].x,
                state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].y
            ),
            (5, 3)
        );
        assert_eq!(shared_prng_draws_between(before, state.prng_state), 0);
    }

    #[test]
    fn the_teleport_probe_accepts_only_coordinates_inside_the_arena_span() {
        // `combat.md §9`: "two independent uniform draws over the sixteen
        // values zero through fifteen ... accepted only when both land inside
        // the eleven-cell arena span, i.e. at ten or below, so the probe
        // succeeds with probability 121/256".
        let accepted = (0u8..16)
            .flat_map(|x| (0u8..16).map(move |y| (x, y)))
            .filter(|&(x, y)| combat_ai_teleport_probe_cell(x, y).is_some())
            .count();

        assert_eq!(accepted, 121);
        assert_eq!(combat_ai_teleport_probe_cell(10, 10), Some((10, 10)));
        assert_eq!(combat_ai_teleport_probe_cell(11, 0), None);
        assert_eq!(combat_ai_teleport_probe_cell(0, 11), None);
    }

    #[test]
    fn the_teleport_chance_roll_is_a_flat_three_in_four() {
        // `combat.md §9`: "one uniform draw over the four values zero through
        // three, and the arm continues on the three lower values and is
        // abandoned on the maximum".
        let accepted = (0u8..4)
            .filter(|&roll| combat_ai_teleport_chance_accepts(roll))
            .count();
        assert_eq!(accepted, 3);
        assert!(!combat_ai_teleport_chance_accepts(3));
    }

    #[test]
    fn the_surrounded_predicate_reads_the_four_cardinal_neighbours_only() {
        // `combat.md §9`: the encirclement bypass asks "whether all four
        // cardinal neighbours of the actor are blocked". Diagonals are outside
        // the test, and an off-arena neighbour counts as blocked.
        let mut legal = [[true; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        assert!(!combat_ai_cardinal_neighbours_blocked(&legal, 5, 5));

        for (x, y) in [(4usize, 5usize), (6, 5), (5, 4), (5, 6)] {
            legal[y][x] = false;
        }
        assert!(combat_ai_cardinal_neighbours_blocked(&legal, 5, 5));

        // Diagonals stay open and must not change the verdict.
        assert!(legal[4][4] && legal[6][6]);
    }

    /// The three-seat arena the side-by-side capture used.
    ///
    /// The party's cached combat-defense byte is zeroed so that `combat.md
    /// §12`'s "when it is zero it takes no draw at all and subtracts nothing"
    /// applies: every landed Bat blow is then the flat class attack byte, and
    /// the per-seat damage sample below measures target selection rather than
    /// defence-roll noise. Hit points are deep enough that no seat can die
    /// inside a sample run, so all three stay in the candidate set throughout.
    fn seated_party_dispatch_state() -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_round_loop_prologue_ran = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state
            .active_objects
            .resize(COMBAT_ACTOR_SLOTS, ActiveObject::empty());
        state.party = (0..PUBLISHED_SEATING.len())
            .map(|slot| PartyMember {
                slot: slot as u8,
                class_byte: 1,
                status: b'G',
                climb_stat: 0,
                mana: 0,
                hp: 900,
                max_hp: 900,
                level: 1,
            })
            .collect();
        state.party_combat_defense = vec![0; PUBLISHED_SEATING.len()];
        for (slot, (x, y)) in PUBLISHED_SEATING.into_iter().enumerate() {
            state.combat_actors[slot] = CombatActorDescriptor::from_row([
                200,
                12,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                slot as u8,
                slot as u8,
                0,
                x,
                y,
            ]);
            state.active_objects[slot] = ActiveObject {
                type_byte: 0x4c,
                tile: 0x4c,
                x: usize::from(x),
                y: usize::from(y),
                ..ActiveObject::empty()
            };
        }
        state
    }

    /// Seeds swept by the per-seat damage-share sample below.
    const DAMAGE_SHARE_SEEDS: u16 = 1024;
    /// Dispatches given to each seeded Bat: enough for a Bat starting anywhere
    /// in the eleven-by-eleven arena to close and land several blows.
    const DAMAGE_SHARE_DISPATCHES: usize = 12;

    #[test]
    fn per_seat_damage_share_over_many_seeds_matches_the_published_target_rule() {
        // The statistical form of the tie-break contract, in the units the
        // side-by-side capture is measured in: **hit points lost per seat**.
        //
        // Each seed seeds the shared gameplay PRNG, draws the Bat's start cell
        // from that same stream (so the sample of geometries is seeded rather
        // than hand-picked), then runs production `apply_combat_ai_turn`
        // dispatches. Every dispatch that lands damage contributes its applied
        // damage to two columns: the seat the engine actually wounded, and the
        // seat `published_target_rule` names for the cell the Bat dispatched
        // from. Nothing is stubbed - the to-hit roll, the step direction and
        // the random-cardinal fallback all run off the shared PRNG - so a seed
        // whose to-hit draw misses simply contributes nothing to either column.
        //
        // The sweep is deterministic: fixed seeds through a fixed generator.
        let mut measured = [0u32; 3];
        let mut predicted = [0u32; 3];
        let mut landed = 0usize;
        let mut seat_hits = [0usize; 3];

        for seed in 0..DAMAGE_SHARE_SEEDS {
            let mut state = seated_party_dispatch_state();
            state.prng_state = seed.wrapping_mul(0x9e37).wrapping_add(0x1234);
            let x =
                u5_prng_range_u16(&mut state.prng_state, 0, (COMBAT_ARENA_SIDE - 1) as u16) as u8;
            let y =
                u5_prng_range_u16(&mut state.prng_state, 0, (COMBAT_ARENA_SIDE - 1) as u16) as u8;
            if PUBLISHED_SEATING.contains(&(x, y)) {
                continue;
            }
            place_dispatch_bat(&mut state, COMBAT_PARTY_ACTOR_SLOTS, x, y);

            for _ in 0..DAMAGE_SHARE_DISPATCHES {
                let cell = (
                    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].x,
                    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].y,
                );
                let Some(application) = state.apply_combat_ai_turn(COMBAT_PARTY_ACTOR_SLOTS) else {
                    break;
                };
                let Some(CombatWeaponDamageApplication::Party {
                    target_slot,
                    damage,
                }) = application
                    .monster_attack
                    .and_then(|attack| attack.damage_application)
                else {
                    continue;
                };
                if damage.applied_damage == 0 {
                    continue;
                }
                measured[target_slot] += u32::from(damage.applied_damage);
                predicted[published_target_rule(cell)] += u32::from(damage.applied_damage);
                seat_hits[target_slot] += 1;
                landed += 1;
            }
        }

        assert!(
            landed >= 500,
            "the sweep has to land a real damage sample; it landed {landed} blows"
        );
        // Every hit point the engine took off a seat went to the seat the
        // published §9 rule names for the cell the blow was dispatched from.
        assert_eq!(
            measured, predicted,
            "per-seat damage share {measured:?} diverges from the published              §9 rule's {predicted:?} over {landed} landed blows"
        );

        // The published tie-break's measurable consequence, and the reason §9
        // glosses it as "biasing toward party members (low slots)": the
        // per-seat damage share falls monotonically with slot number. Seats 1
        // and 2 are mirror images about seat 0's column, so nothing but the
        // tie-break can separate them. Both orderings invert under a
        // highest-slot-wins scan, which is what makes this the statistical
        // regression lock on the tie-break direction. The sweep currently
        // measures 9102/5610/4614 hit points over 3221 landed blows
        // (1517/935/769 of them), every blow the Bat's flat attack byte of six.
        assert!(
            measured[0] > measured[1] && measured[1] > measured[2],
            "the published rule biases toward low slots, so the per-seat damage              share has to fall with slot number; it was {measured:?} over hits              {seat_hits:?}"
        );
    }

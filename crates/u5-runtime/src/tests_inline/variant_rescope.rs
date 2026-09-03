// `systems/visibility.md` §8/§8.1/§8.3/§8.4 as re-scoped by
// `cleak/u5-spec#182` (spec commit `210aa41`, `RETRACTIONS.md` R329-R333),
// plus `systems/prng.md` §4, `systems/combat.md` §5.3 step 6 and
// `systems/intro.md` §7.

/// `visibility.md §8`, normative: "Those five rows are the **only** rows that
/// reach the selector. **Every other row of the table above — including both
/// chair fall-throughs, the bed, the two ladders, the two facing-only chairs,
/// and the plain pass-through — makes no selection at all.**"
#[test]
fn only_five_terrain_rows_reach_the_variant_selector() {
    // Stocks and manacles are unconditional: no neighbour predicate.
    for neighbour in [None, Some(0x00), Some(0x9A), Some(0x9B), Some(0x9C)] {
        assert_eq!(
            active_object_default_variant_base(0x40, 0x84, neighbour, neighbour),
            Some(0x60),
            "stocks 0x84 select unconditionally"
        );
        assert_eq!(
            active_object_default_variant_base(0x40, 0x85, neighbour, neighbour),
            Some(0x64),
            "manacles 0x85 select unconditionally"
        );
        // `§8.4`: "**Terrain `0x9E` never appears as map terrain and its row
        // is dead in the shipped game.** Only `0x9D` reaches the trapped-soul
        // selection."
        assert_eq!(
            active_object_default_variant_base(0x40, 0x9D, neighbour, neighbour),
            Some(0x3C),
            "the mirror row 0x9D selects unconditionally"
        );
    }

    // "**The accepted set differs per facing, asymmetrically.** The `0x92`
    // chair accepts `0x9A` or `0x9C` on the row below it and rejects `0x9B`;
    // the `0x90` chair accepts `0x9B` or `0x9C` on the row above it and
    // rejects `0x9A`."
    for accepted in [0x9A, 0x9C] {
        assert_eq!(
            active_object_default_variant_base(0x40, 0x92, None, Some(accepted)),
            Some(0x34)
        );
    }
    assert_eq!(
        active_object_default_variant_base(0x40, 0x92, None, Some(0x9B)),
        None,
        "the 0x92 chair rejects 0x9B"
    );
    for accepted in [0x9B, 0x9C] {
        assert_eq!(
            active_object_default_variant_base(0x40, 0x90, Some(accepted), None),
            Some(0x38)
        );
    }
    assert_eq!(
        active_object_default_variant_base(0x40, 0x90, Some(0x9A), None),
        None,
        "the 0x90 chair rejects 0x9A"
    );

    // The predicate is one-sided: each facing reads exactly one neighbouring
    // row, so a laden table on the wrong side never qualifies.
    assert_eq!(
        active_object_default_variant_base(0x40, 0x92, Some(0x9C), None),
        None
    );
    assert_eq!(
        active_object_default_variant_base(0x40, 0x90, None, Some(0x9C)),
        None
    );

    // "**A bare table is not a table for this purpose.** The plain-table ids
    // `0x94..0x96` never qualify, and neither does any other furniture — an
    // end table, a desk, a candelabrum, a harpsichord, or ordinary floor."
    for plain in [0x94u8, 0x95, 0x96] {
        assert_eq!(
            active_object_default_variant_base(0x40, 0x92, None, Some(plain)),
            None,
            "plain table {plain:#04x} below a 0x92 chair"
        );
        assert_eq!(
            active_object_default_variant_base(0x40, 0x90, Some(plain), None),
            None,
            "plain table {plain:#04x} above a 0x90 chair"
        );
    }

    // Every other published row: the two facing-only chairs, the bed, the two
    // ladders, the suppress/direct rows and the plain pass-through.
    for terrain in [0x91u8, 0x93, 0xAB, 0xC8, 0xC9, 0xEC, 0x0A, 0x57, 0x6A, 0x6B, 0x10, 0x9F] {
        assert_eq!(
            active_object_default_variant_base(0x40, terrain, Some(0x9C), Some(0x9C)),
            None,
            "terrain {terrain:#04x} must make no selection"
        );
    }
}

/// The "does this row draw?" predicate and the stamped tile cannot drift
/// apart: a row selects **iff** its stamped tile depends on the variant.
/// `visibility.md §8.1`: "one draw for each actor that ... stands on stocks,
/// manacles, a mirror, or a chair whose neighbouring row on the correct side
/// holds a laden-table id — and zero draws for everything else".
#[test]
fn variant_base_is_exactly_the_set_of_variant_dependent_stamps() {
    let neighbours = [
        None,
        Some(0x00u8),
        Some(0x44),
        Some(0x94),
        Some(0x95),
        Some(0x96),
        Some(0x9A),
        Some(0x9B),
        Some(0x9C),
        Some(0x9D),
    ];
    for tile in 0..=u8::MAX {
        for terrain in 0..=u8::MAX {
            for previous in neighbours {
                for next in neighbours {
                    let selects =
                        active_object_default_variant_base(tile, terrain, previous, next).is_some();
                    let with_zero =
                        active_object_default_composite(tile, terrain, previous, next, 5, 0);
                    let variant_dependent = (1u8..4).any(|variant| {
                        active_object_default_composite(tile, terrain, previous, next, 5, variant)
                            != with_zero
                    });
                    assert_eq!(
                        selects, variant_dependent,
                        "tile {tile:#04x} terrain {terrain:#04x} prev {previous:?} next {next:?}"
                    );
                    if let Some(base) = active_object_default_variant_base(
                        tile, terrain, previous, next,
                    ) {
                        for variant in 0..4u8 {
                            assert_eq!(
                                active_object_default_composite(
                                    tile, terrain, previous, next, 5, variant
                                ),
                                ActiveObjectCompositeResult::Companion(base + variant),
                                "four-entry range from {base:#04x}"
                            );
                        }
                    }
                }
            }
        }
    }
}

/// `visibility.md §8.1`: "The three direct-stamp branches (the two
/// water/companion classes and the single-sprite-family seated branch of
/// Section 8) bypass the compositor entirely and therefore never draw,
/// whatever terrain they are on", and the fogged / already-claimed cell
/// guards are evaluated "before the compositor is invoked, so a skipped actor
/// costs no draw at all".
#[test]
fn direct_stamp_branches_and_cell_guards_never_draw() {
    // A selecting terrain under each direct-stamp branch still takes no draw.
    for (type_byte, frame_byte) in [(0xE8u8, 0x44u8), (0x80, 0x1D), (0x5C, 0x44)] {
        assert!(
            !composite_active_object_slot_draws_variant(
                type_byte,
                frame_byte,
                if type_byte == 0x5C {
                    SINGLE_SPRITE_FAMILY_SEATED_CHAIR_TERRAIN
                } else {
                    VISIBILITY_CLEAR
                },
                0x84,
                None,
                None,
            ),
            "direct-stamp branch ({type_byte:#04x}, {frame_byte:#04x}) must not draw"
        );
    }

    // The two cell-state guards.
    for guard in [VISIBILITY_HIDDEN, VISIBILITY_ALREADY_RENDERED] {
        assert!(!composite_active_object_slot_draws_variant(
            0x80, 0x44, guard, 0x84, None, None
        ));
    }

    // The default helper on a selecting row does draw, for slot zero and for
    // any other slot alike.
    assert!(composite_active_object_slot_draws_variant(
        0x80,
        0x44,
        VISIBILITY_CLEAR,
        0x84,
        None,
        None
    ));

    // `RETRACTIONS.md` R330: the `0x5C` fall-through reaches the default
    // helper "with its frame byte **reduced by eight**", so the row it lands
    // on is decided by the reduced byte. `0x44 - 8 = 0x3C` is not
    // terrain-aware and cannot select; `0x4C - 8 = 0x44` is.
    assert!(!composite_active_object_slot_draws_variant(
        0x5C,
        0x44,
        VISIBILITY_CLEAR,
        0x84,
        None,
        None
    ));
    assert!(composite_active_object_slot_draws_variant(
        0x5C,
        0x4C,
        VISIBILITY_CLEAR,
        0x84,
        None,
        None
    ));

    // A non-qualifying chair takes no draw and stamps the fixed tile — the
    // majority case in the shipped maps (`§8`, `§8.3`).
    assert!(!composite_active_object_slot_draws_variant(
        0x80,
        0x44,
        VISIBILITY_CLEAR,
        0x92,
        None,
        Some(0x95),
    ));
    assert_eq!(
        composite_active_object_slot(false, 0x80, 0x44, VISIBILITY_CLEAR, 0x92, None, Some(0x95), 5, 0),
        ActiveObjectCompositeResult::Companion(0x32)
    );
    // The bed likewise: "**A single fixed tile — not a variant, and no
    // selection is made.**"
    assert!(!composite_active_object_slot_draws_variant(
        0x80,
        0x44,
        VISIBILITY_CLEAR,
        0xAB,
        None,
        None
    ));
}

/// `visibility.md §8.1`, normative: the variant "is drawn **once per composite
/// pass** — not once per placement — and **it is never cached anywhere.**"
/// `§8.3`: the value "changes on about **three passes in four** while the
/// player presses nothing", because "the probability that two draws separated
/// by one to five steps differ is 0.7508".
#[test]
fn the_variant_is_a_fresh_uniform_draw_on_every_pass() {
    let mut state = test_state(open_grid(), 10, 20);

    const SAMPLES: usize = 20_000;
    let mut histogram = [0usize; 4];
    let mut transitions = 0usize;
    let mut previous = state.draw_active_object_composite_variant();
    histogram[usize::from(previous)] += 1;
    for _ in 1..SAMPLES {
        let next = state.draw_active_object_composite_variant();
        assert!(next < 4, "the selector returns a value in 0..3");
        histogram[usize::from(next)] += 1;
        if next != previous {
            transitions += 1;
        }
        previous = next;
    }

    // "the requested span of four divides its output range exactly so the four
    // outcomes are equally likely ... the `0..3` histogram over 200,000 draws
    // is flat to within about 0.7 percent".
    for (entry, count) in histogram.iter().enumerate() {
        let share = *count as f64 / SAMPLES as f64;
        assert!(
            (share - 0.25).abs() < 0.02,
            "entry {entry} share {share} is not uniform"
        );
    }

    // The measured cadence on `cleak/u5-spec#182`'s named-cell recapture was
    // 0.695..0.753 transitions per tick on the six qualifying seats.
    let rate = transitions as f64 / (SAMPLES - 1) as f64;
    assert!(
        (rate - ACTIVE_OBJECT_VARIANT_TRANSITION_PROBABILITY).abs() < 0.02,
        "transition rate {rate} is not the published ~0.75"
    );
    assert!((0.68..=0.78).contains(&rate));
}

/// `visibility.md §8.1`: the composite pass takes "one draw for each actor
/// that (a) survives all of the pass's per-slot skips, (b) is handed to the
/// default helper ... and (c) stands on stocks, manacles, a mirror, or a chair
/// whose neighbouring row on the correct side holds a laden-table id — and
/// zero draws for everything else". Driven through the live compositor.
/// One humanoid NPC sprite byte inside the merging band. `§8.4` names the
/// band ("the humanoid NPC sprite range"); `§8`'s table routes every
/// effective tile at or above `0x80` to the plain stamp before the terrain
/// rows are consulted, so a merging sprite is below it.
const HUMANOID_NPC_TEST_SPRITE: u8 = 0x60;

fn variant_pass_grid(chair: u8, neighbour: u8, chair_x: usize, chair_y: usize) -> Vec<u8> {
    let mut grid = open_grid();
    grid[chair_y * 32 + chair_x] = chair;
    let neighbour_y = if chair == 0x92 { chair_y + 1 } else { chair_y - 1 };
    grid[neighbour_y * 32 + chair_x] = neighbour;
    grid
}

/// One ordinary humanoid NPC on a cell, in a slot that is not slot zero:
/// `§8.4` says "There is nothing avatar-specific anywhere in the merge: an NPC
/// in the humanoid sprite range on a selecting terrain merges by exactly the
/// same rules the party does", and "the merge applies only to the party's own
/// sprite families (on foot, horse, magic carpet, skiff) and to the humanoid
/// NPC sprite range" - the band the default helper treats as terrain-aware
/// without passing it straight through, i.e. below `0x80`.
fn seated_humanoid_npc(x: usize, y: usize) -> ActiveObject {
    ActiveObject {
        type_byte: HUMANOID_NPC_TEST_SPRITE,
        tile: HUMANOID_NPC_TEST_SPRITE,
        x,
        y,
        z: 0,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    }
}

/// Run one live composite pass and report both halves of the `§8.1` contract
/// that must not drift apart: how many draws the pass took off the shared
/// stream, and the byte it left in the companion terrain band under the seat.
///
/// `extra_actors` are pushed *after* the seated NPC, so the compositor's
/// "thirty-one down to zero" walk reaches them **first** - which is how a
/// stamp onto the seat's neighbouring cell gets to land before the seat reads
/// that neighbour.
fn composite_pass_at(
    chair: u8,
    neighbour: u8,
    chair_xy: (usize, usize),
    party_xy: (usize, usize),
    extra_actors: &[ActiveObject],
) -> (u32, u8) {
    let (chair_x, chair_y) = chair_xy;
    let (party_x, party_y) = party_xy;
    let mut state = test_state(
        variant_pass_grid(chair, neighbour, chair_x, chair_y),
        party_x,
        party_y,
    );
    state.active_objects.push(seated_humanoid_npc(chair_x, chair_y));
    state.active_objects.extend_from_slice(extra_actors);
    state.visibility_buffers_ready = false;
    let before = state.prng_state;
    state.refresh_top_down_visibility_buffers(TopDownRenderArea::Town, VIEWPORT_PLAYER_ROW);
    let mut probe = before;
    let mut draws = 0u32;
    while probe != state.prng_state {
        let _ = u5_prng_range_u16(&mut probe, 0, 3);
        draws += 1;
        assert!(draws < 64, "composite pass drew far more than one value");
    }
    let row = chair_y + VIEWPORT_PLAYER_ROW - party_y;
    let col = chair_x + VIEWPORT_PLAYER_COL - party_x;
    let stamped = state.terrain_band[terrain_band_active_index(row, col).unwrap()];
    (draws, stamped)
}

fn composite_pass_draw_count(chair: u8, neighbour: u8) -> u32 {
    // The party stands three cells south of the seat, so the seat and its
    // neighbouring row are both comfortably inside the viewport. The two edge
    // cases the band cannot answer have their own tests below.
    composite_pass_at(chair, neighbour, (10, 20), (10, 23), &[]).0
}

#[test]
fn composite_pass_draws_once_for_a_qualifying_seat_and_never_otherwise() {
    // A 0x92 chair over a laden table selects; the same chair over the plain
    // table 0x95 is a fall-through and must cost nothing.
    assert_eq!(composite_pass_draw_count(0x92, 0x9C), 1);
    assert_eq!(composite_pass_draw_count(0x92, 0x9A), 1);
    assert_eq!(composite_pass_draw_count(0x92, 0x9B), 0);
    assert_eq!(composite_pass_draw_count(0x92, 0x95), 0);
    assert_eq!(composite_pass_draw_count(0x90, 0x9C), 1);
    assert_eq!(composite_pass_draw_count(0x90, 0x9B), 1);
    assert_eq!(composite_pass_draw_count(0x90, 0x9A), 0);
    assert_eq!(composite_pass_draw_count(0x90, 0x95), 0);
}

/// `visibility.md §8`: "Terrain-aware stamps use the live world/combat tile
/// at the object's coordinate, plus one neighbouring row for a few edge
/// shapes."
///
/// The eleven-by-eleven companion terrain band is not that read. A seat on the
/// viewport's own edge row has its neighbouring row *outside* the band
/// entirely, and the party has to be only five cells away for that to happen -
/// ordinary tavern or castle walking. An engine that consults the band there
/// sees no neighbour, takes no draw where `§8.1` charges one ("advances the
/// single global generator when the original does not, and its stream position
/// diverges permanently" - in the under-draw direction), and stamps the fixed
/// fall-through tile instead of a variant.
#[test]
fn a_qualifying_seat_on_a_viewport_edge_row_still_reads_its_neighbour() {
    // A `0x92` seat on the bottom viewport row (party five cells north): the
    // laden table it faces is one row further south, off the band.
    let (draws, stamped) = composite_pass_at(0x92, 0x9C, (10, 20), (10, 15), &[]);
    assert_eq!(draws, 1, "the seat qualifies, so the pass takes one draw");
    assert!(
        (0x34..=0x37).contains(&stamped),
        "expected one of 0x34..0x37, not the fall-through 0x32: {stamped:#04x}"
    );

    // The mirror image: a `0x90` seat on the top viewport row (party five
    // cells south), its laden table one row further north and off the band.
    let (draws, stamped) = composite_pass_at(0x90, 0x9B, (10, 20), (10, 25), &[]);
    assert_eq!(draws, 1);
    assert!(
        (0x38..=0x3B).contains(&stamped),
        "expected one of 0x38..0x3B, not the fall-through 0x30: {stamped:#04x}"
    );

    // The same two seats over furniture their facing rejects still take no
    // draw when the neighbour is off-band - the read is restored, not widened.
    assert_eq!(composite_pass_at(0x92, 0x9B, (10, 20), (10, 15), &[]).0, 0);
    assert_eq!(composite_pass_at(0x90, 0x9A, (10, 20), (10, 25), &[]).0, 0);
}

/// The band is also not the live map *inside* the viewport: `§8` says the
/// compositor's write set includes "the companion sprite band", and it walks
/// "from slot thirty-one down to slot zero", so a stamp made earlier in the
/// same pass overwrites the band cell a later slot would read. An actor
/// standing on the laden table must not erase the table id the seat beside it
/// reads.
#[test]
fn an_earlier_stamp_on_the_neighbouring_cell_does_not_erase_the_terrain() {
    // Slot 2 stands on the laden table at (10, 21) and composites first; slot
    // 1 is the seat at (10, 20) that reads that row afterwards.
    let table_walker = [seated_humanoid_npc(10, 21)];
    let (draws, stamped) = composite_pass_at(0x92, 0x9C, (10, 20), (10, 23), &table_walker);
    assert_eq!(
        draws, 1,
        "the actor on the table stands on no selecting row and takes no draw; \
         the seat still takes its one"
    );
    assert!(
        (0x34..=0x37).contains(&stamped),
        "expected one of 0x34..0x37, not the fall-through 0x32: {stamped:#04x}"
    );

    // And the mirror facing, with the walker on the row above the seat.
    let table_walker = [seated_humanoid_npc(10, 19)];
    let (draws, stamped) = composite_pass_at(0x90, 0x9B, (10, 20), (10, 23), &table_walker);
    assert_eq!(draws, 1);
    assert!(
        (0x38..=0x3B).contains(&stamped),
        "expected one of 0x38..0x3B, not the fall-through 0x30: {stamped:#04x}"
    );
}

/// `visibility.md §8`: "Before the per-slot loop runs, and **only outside
/// combat**, the compositor refreshes slot zero's bytes from the world-state
/// globals ... Note that this refresh writes the party sprite marker into
/// **both** the slot's type byte and its sprite byte, which is why the party
/// can never satisfy the type-byte test of the single-sprite-family seated
/// branch above. In combat the refresh does not run".
#[test]
fn slot_zero_refresh_writes_the_party_marker_into_both_bytes_outside_combat_only() {
    let mut state = test_state(open_grid(), 10, 20);
    state.active_objects[0] = ActiveObject {
        type_byte: 0x5C,
        tile: 0x5C,
        x: 0,
        y: 0,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
    state.sync_player_object();
    let marker = state.player.transport.save_marker();
    assert_eq!(state.active_objects[0].type_byte, marker);
    assert_eq!(state.active_objects[0].tile, marker);
    assert_ne!(
        state.active_objects[0].type_byte, 0x5C,
        "the party can never satisfy the single-sprite-family type-byte test"
    );
    assert_ne!(
        active_object_compositor_branch(marker, marker),
        ActiveObjectCompositorBranch::SingleSpriteFamilySeated
    );

    // In combat the refresh does not run: slot zero is the seated first party
    // member and keeps whatever the combat subsystem left there.
    let mut combat = test_state(open_grid(), 10, 20);
    combat.combat_active = true;
    combat.active_objects[0] = ActiveObject {
        type_byte: 0x5C,
        tile: 0x5C,
        x: 3,
        y: 4,
        z: 0,
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
    combat.sync_player_object();
    assert_eq!(combat.active_objects[0].type_byte, 0x5C);
    assert_eq!(combat.active_objects[0].tile, 0x5C);
}

/// `prng.md §4` / `visibility.md §8.4` (`RETRACTIONS.md` R331, R332): "**Three**
/// per-pass consumers run on the idle path, before any command is entered, and
/// they draw in this order: 1. The **active-object animator** ... 2. The
/// **per-pass wind check**, which draws **once** and returns in the common
/// case ... On the uncommon result it enters a retry loop taking **one further
/// draw at a time**, so its count per invocation is one, two, three, and so on
/// upward ... 3. The **viewport composite pass** ... In an ordinary scene with
/// nobody seated at a laid table this consumer takes **no draws at all**".
#[test]
fn idle_pass_consumers_run_animator_then_wind_then_composite() {
    let mut state = test_state(open_grid(), 10, 20);
    // The wind check is the first modelled consumer of the tick (the animator
    // draws nothing in this engine — see the not-implemented note in
    // `advance_visual_tick`), and it draws at least once.
    let before = state.prng_state;
    state.advance_visual_tick();
    assert_ne!(before, state.prng_state, "the wind check draws every tick");

    // The wind check's retry loop takes single draws, so every integer from
    // one upward is a reachable per-invocation count. Assert the shape rather
    // than a maximum, since "**No maximum exists and an engine must not assume
    // one**".
    let mut counts = std::collections::BTreeSet::new();
    for seed in 1..40_000u32 {
        let seed = seed as u16;
        state.prng_state = seed;
        let _ = state.idle_wind_drift();
        let mut walk = seed;
        let mut draws = 0u32;
        while walk != state.prng_state && draws < 64 {
            walk = u5_prng_advance_state(walk);
            draws += 1;
        }
        // The advance is a permutation of the sixteen-bit state, so a seed
        // that happens to be its own successor is indistinguishable from "no
        // draw" by state comparison alone; skip those rather than mis-count.
        if draws != 0 {
            counts.insert(draws);
        }
    }
    // `RETRACTIONS.md` R332: "The retries are **single draws**, so the
    // per-invocation count is **one, two, three, and so on upward** - every
    // integer from one *is* reachable, and the combat instruction was exactly
    // backwards." A pairs-based retry cannot produce this run of counts.
    for reachable in [1u32, 2, 3, 4] {
        assert!(
            counts.contains(&reachable),
            "a per-invocation count of {reachable} must be reachable, got {counts:?}"
        );
    }

    // The composite adds nothing in an ordinary scene with nobody seated at a
    // laid table.
    assert_eq!(composite_pass_draw_count(0x92, 0x95), 0);
}

/// `intro.md §7` step 7: "**The test is a single byte: `SAVED.GAM` file offset
/// `0x0002` — the first byte of the name field of character record zero — must
/// be non-zero.** Nothing else is examined: not a length, not a checksum, not
/// a party-size field, not any other byte of the name, and not any other field
/// of the record."
#[test]
fn journey_onward_active_save_gate_is_exactly_byte_two() {
    assert_eq!(SAVE_AVATAR_NAME_OFFSET, 0x0002);

    let mut image = vec![0u8; SAVED_GAM_LEN];
    assert!(
        !save_image_has_active_avatar(&image),
        "the shipped starting template has that name field entirely zero"
    );

    // "an implementation that scans the whole nine-byte name field, or that
    // trims spaces before testing, disagrees with the original on a save whose
    // stored name begins with a zero byte but has non-zero bytes after it."
    for offset in 1..9usize {
        let mut leading_zero = image.clone();
        leading_zero[SAVE_AVATAR_NAME_OFFSET + offset] = b'A';
        assert!(
            !save_image_has_active_avatar(&leading_zero),
            "byte {offset} of the name field must not be examined"
        );
    }

    // A space is non-zero, so it passes: the test is not a trim.
    image[SAVE_AVATAR_NAME_OFFSET] = b' ';
    assert!(save_image_has_active_avatar(&image));
    for byte in 1..=u8::MAX {
        image[SAVE_AVATAR_NAME_OFFSET] = byte;
        assert!(save_image_has_active_avatar(&image), "byte {byte:#04x}");
    }

    // No other field of the record is examined: a save whose every other byte
    // is zero still passes on that one byte.
    let mut only_the_gate = vec![0u8; SAVED_GAM_LEN];
    only_the_gate[SAVE_AVATAR_NAME_OFFSET] = b'A';
    assert!(save_image_has_active_avatar(&only_the_gate));
}

/// `visibility.md §8` / `RETRACTIONS.md` R333: the Negate Time code has "**two**
/// producers, not one. ... the **Negate Time spell** handler, which writes the
/// code as an immediate together with the effect's ten-turn duration; the
/// shared **timed-effect setter**, which writes the code from its argument and
/// is passed this code by exactly one of its call sites, the **Negate Time
/// scroll**, with a twenty-turn duration".
///
/// `magic.md §8`'s timed-effect table: "*An Tym* — Negate Time (spell) | `T` |
/// 10" and "Negate Time scroll | `T` | 20".
#[test]
fn negate_time_has_two_producers_with_ten_and_twenty_turn_durations() {
    assert_eq!(NEGATE_TIME_ACTIVE_EFFECT_TAG, b'T');
    assert_eq!(TIME_STOP_DURATION, 10);
    assert_eq!(SCROLL_NEGATE_TIME_DURATION, 20);

    // Both producers install the same code into the same shared register, so
    // both freeze the selector.
    let mut spell = test_state(open_grid(), 10, 20);
    spell.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
    spell.party[0].mana = 8;
    spell.party[0].level = 8;
    assert_eq!(spell.cast_time_stop(0), MoveOutcome::Cast);
    assert_eq!(spell.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
    assert_eq!(spell.active_effect_counter, TIME_STOP_DURATION);
    assert!(spell.negate_time_active());

    let mut scroll = test_state(open_grid(), 10, 20);
    scroll.scroll_stock[SCROLL_NEGATE_TIME_INDEX] = 1;
    assert_eq!(scroll.use_negate_time_scroll(), MoveOutcome::Used);
    assert_eq!(scroll.active_effect_tag, Some(NEGATE_TIME_ACTIVE_EFFECT_TAG));
    // `magic.md §8`: "Command-dispatch cleanup and combat
    // active-player/selection cleanup age nonzero/non-255 countdowns", and the
    // scroll's own turn runs that cleanup, so the observable counter is one
    // below the installed twenty.
    assert_eq!(
        scroll.active_effect_counter + 1,
        SCROLL_NEGATE_TIME_DURATION,
        "the scroll installs the twenty-turn duration, not the spell's ten"
    );
    assert!(
        scroll.active_effect_counter > TIME_STOP_DURATION,
        "an engine that froze the selector only for the spell would use ten here"
    );
    assert!(scroll.negate_time_active());

    // "An engine that freezes the selector only for the spell will animate
    // seated furniture through the scroll's effect" — and it would also draw
    // from the shared stream while the original does not.
    for state in [&mut spell, &mut scroll] {
        let before = state.prng_state;
        for _ in 0..16 {
            assert_eq!(state.draw_active_object_composite_variant(), 0);
        }
        assert_eq!(before, state.prng_state);
    }
}

/// The re-scoped contract, checked against the **shipped** location files
/// rather than a hand-built grid. Cells and neighbour ids are the ones
/// published in the `cleak/u5-spec#182` recapture table; each row names its
/// cell and the terrain on the neighbouring row, which `§8.3` requires of any
/// claim about this arm.
#[test]
fn shipped_location_files_reproduce_the_named_cell_recapture() {
    let Some(dir) = local_clean_assets() else {
        return;
    };

    // Iolo's Hut (17,14) `0x92`: "below `0x95` (plain table)", 0 transitions
    // over 273 ticks. "its only `0x92` chair sits above the plain table
    // `0x95`".
    let hut = load_floor(&dir, Scene::new(13).unwrap(), 0).expect("DWELLING.DAT floor loads");
    assert_eq!(hut[14 * 32 + 17], 0x92);
    assert_eq!(hut[15 * 32 + 17], 0x95);
    assert_eq!(
        active_object_default_variant_base(PLAYER_TILE, hut[14 * 32 + 17], None, Some(hut[15 * 32 + 17])),
        None,
        "Iolo's Hut (17,14) is a fall-through: 0 transitions in 273 ticks"
    );

    let castle = load_floor(&dir, Scene::new(17).unwrap(), 0).expect("CASTLE.DAT floor loads");
    // LB castle (9,24) `0x92`: "below `0x9C`", 191/275 = 0.695 per tick.
    assert_eq!(castle[24 * 32 + 9], 0x92);
    assert_eq!(castle[25 * 32 + 9], 0x9C);
    assert_eq!(
        active_object_default_variant_base(
            PLAYER_TILE,
            castle[24 * 32 + 9],
            Some(castle[23 * 32 + 9]),
            Some(castle[25 * 32 + 9]),
        ),
        Some(0x34)
    );
    // LB castle (11,23) `0x92`: "below `0x44`", 0/275.
    assert_eq!(castle[23 * 32 + 11], 0x92);
    assert_eq!(castle[24 * 32 + 11], 0x44);
    assert_eq!(
        active_object_default_variant_base(
            PLAYER_TILE,
            castle[23 * 32 + 11],
            Some(castle[22 * 32 + 11]),
            Some(castle[24 * 32 + 11]),
        ),
        None
    );
    // LB castle (9,26) `0x90`: "above `0x9C`", 195/275 = 0.709 per tick. The
    // same `0x9C` cell serves the `0x92` seat two rows up.
    assert_eq!(castle[26 * 32 + 9], 0x90);
    assert_eq!(castle[25 * 32 + 9], 0x9C);
    assert_eq!(
        active_object_default_variant_base(
            PLAYER_TILE,
            castle[26 * 32 + 9],
            Some(castle[25 * 32 + 9]),
            Some(castle[27 * 32 + 9]),
        ),
        Some(0x38)
    );

    // Serpent's Hold (7,19) `0x92` below `0x9C` and (7,21) `0x90` above
    // `0x9C`: 204/275 and 207/275.
    let keep = load_floor(&dir, Scene::new(32).unwrap(), 0).expect("KEEP.DAT floor loads");
    assert_eq!(keep[19 * 32 + 7], 0x92);
    assert_eq!(keep[20 * 32 + 7], 0x9C);
    assert_eq!(
        active_object_default_variant_base(PLAYER_TILE, 0x92, None, Some(keep[20 * 32 + 7])),
        Some(0x34)
    );
    assert_eq!(keep[21 * 32 + 7], 0x90);
    assert_eq!(
        active_object_default_variant_base(PLAYER_TILE, 0x90, Some(keep[20 * 32 + 7]), None),
        Some(0x38)
    );
}

/// `visibility.md §8`: "a full census of every chair cell in the four
/// town/dwelling/castle/keep location files, adjudicated against its own
/// neighbouring row, finds **roughly half** of the `0x92` chairs and **about
/// two in five** of the `0x90` chairs qualifying", and `§8.4`: "**Terrain
/// `0x9E` never appears as map terrain and its row is dead in the shipped
/// game.**"
#[test]
fn shipped_chair_census_matches_the_published_proportions() {
    let Some(dir) = local_clean_assets() else {
        return;
    };

    let mut totals = [(0usize, 0usize); 2];
    let mut dead_row_cells = 0usize;
    for stem in ["TOWNE", "DWELLING", "CASTLE", "KEEP"] {
        let bytes = std::fs::read(dir.join(format!("{stem}.DAT"))).expect("location file reads");
        for page in bytes.chunks_exact(1024) {
            for y in 0..32usize {
                for x in 0..32usize {
                    let terrain = page[y * 32 + x];
                    if terrain == 0x9E {
                        dead_row_cells += 1;
                    }
                    let slot = match terrain {
                        0x92 => 0,
                        0x90 => 1,
                        _ => continue,
                    };
                    let previous = (y > 0).then(|| page[(y - 1) * 32 + x]);
                    let next = (y + 1 < 32).then(|| page[(y + 1) * 32 + x]);
                    totals[slot].0 += 1;
                    if active_object_default_variant_base(PLAYER_TILE, terrain, previous, next).is_some() {
                        totals[slot].1 += 1;
                    }
                }
            }
        }
    }

    assert_eq!(dead_row_cells, 0, "0x9E never appears as map terrain");

    let facing_92 = totals[0].1 as f64 / totals[0].0 as f64;
    let facing_90 = totals[1].1 as f64 / totals[1].0 as f64;
    assert!(
        (0.40..=0.60).contains(&facing_92),
        "roughly half of the 0x92 chairs qualify, got {facing_92} of {}",
        totals[0].0
    );
    assert!(
        (0.30..=0.50).contains(&facing_90),
        "about two in five of the 0x90 chairs qualify, got {facing_90} of {}",
        totals[1].0
    );
    // "**a seated actor that never changes tile is the expected result for the
    // majority of seats in the game, not a defect.**"
    let seats = totals[0].0 + totals[1].0;
    let qualifying = totals[0].1 + totals[1].1;
    assert!(
        qualifying * 2 < seats,
        "most seats in the shipped maps are fall-throughs"
    );
}

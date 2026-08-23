//! Shared outdoor ranged-attack contract.
//!
//! `overworld.md §6.2` ("Creature ranged attacks") owns this contract and
//! says so explicitly: "They share one trace procedure and one damage
//! payload, and **this section is the normative owner of both**.
//! `systems/active-objects.md` Section 8 describes the same two reactions
//! from the per-turn walker's side and points here rather than restating
//! the contract."
//!
//! The two outdoor callers live in the outdoor per-turn walker's first
//! phase: the Sea Serpent / Dragon breath attack and the ship-like
//! water-creature / pirate broadside. Three further sites reach the
//! *payload* half without the trace — the sand-trap adjacency reaction,
//! the whirlpool engagement, and the drowning loop of `vehicles.md §6`.
//!
//! This module owns the parts of §6.2 that are pure: the trigger windows,
//! the wrapped-torus deltas they are measured on, the sub-tile line
//! generator and its sampling rule, and the payload's roll bounds and
//! branch predicates. The stateful halves — walking the roster, writing
//! hull condition, running the loss-of-ship ladder — live on `PlayState`
//! because they need the roster, the transport marker and the PRNG.
//!
//! # Exactly two outdoor ranged attackers exist
//!
//! §6.2.1 tabulates them, and §6.2.5 bounds how far that census reaches:
//! the count of creatures that can fire a ranged shot at all is "an
//! exhaustive byte scan for near calls across the shipped executable and
//! every overlay with a published load base", which "does not cover far
//! calls, indirect or computed calls, or table dispatch".
//! [`outdoor_ranged_attacker_figure`] is the closed recognition table that
//! fact becomes in code.
//!
//! # The flight carries no payload
//!
//! [`trace_outdoor_ranged_attack`] walks the line drawing a figure per
//! sampled position and returns only whether the line ran clear. §6.2.2
//! states the polarity positively: "a run that reaches the end of the
//! generated line reports clear, and clear is what both outdoor call
//! sites treat as a hit." Damage is the caller's next step, never the
//! walker's.
//!
//! # Known divergence: the blocking tile-id set
//!
//! §6.2.2 says the obstruction test consults "a fixed per-tile-id
//! passability bitmap in which exactly **46** of the 256 tile ids block",
//! and §6.2.5 names that set as an open gap: "The 46 blocking ids are
//! established as a set of ids; they were not mapped to named terrain."
//! This module therefore takes the predicate from its caller and does not
//! own one. The tree's [`crate::surface_tile_blocks_projectile`] stands in
//! at the two outdoor call sites and blocks considerably more than 46 ids;
//! that is a known divergence from the published count, not a claim to
//! reproduce it.

use crate::transport::{ShipLossFallback, TransportState};
use crate::{VIEWPORT_PLAYER_COL, VIEWPORT_PLAYER_ROW, VIEWPORT_SIDE};

/// Module-local `i32` view of [`crate::WORLD_SIDE`], matching
/// [`crate::directed_step`]'s convention so the torus arithmetic here does
/// not need a cast per call.
const WORLD_SIDE: i32 = crate::WORLD_SIDE as i32;

/// `overworld.md §6.2.1` ranged-attack window, in cells. Both outdoor
/// cases use the same number: the breath attack requires "[w]rapped
/// absolute separation from the party of at most three on **both** axes,
/// inclusive on each axis", and the broadside requires "zero separation on
/// one axis, separation below four on the other".
pub const OUTDOOR_RANGED_ATTACK_RANGE_CELLS: i32 = 3;

/// Signed shortest delta from `from` to `to` on the 256-cell overworld
/// torus, in `-128..=127`.
///
/// Overworld player-relative distances **wrap**: a creature at x=2 and a
/// party at x=254 are four cells apart, not 252. Raw subtraction would put
/// every object near the map seam ~255 cells away and silently disable
/// every window in this module. Ties at exactly half the torus prefer the
/// forward step, matching [`crate::directed_step`]'s `wrapped_step_axis`.
pub fn wrapped_axis_delta(from: u8, to: u8) -> i32 {
    let forward = (to as i32 - from as i32).rem_euclid(WORLD_SIDE);
    if forward <= WORLD_SIDE - forward {
        forward
    } else {
        forward - WORLD_SIDE
    }
}

/// Wrapped `(dx, dy)` from an attacker's cell to the party's cell on the
/// overworld torus. Convenience wrapper over [`wrapped_axis_delta`] so
/// callers cannot accidentally mix a wrapped axis with a raw one.
pub fn wrapped_deltas_to_player(
    attacker_x: u8,
    attacker_y: u8,
    player_x: u8,
    player_y: u8,
) -> (i32, i32) {
    (
        wrapped_axis_delta(attacker_x, player_x),
        wrapped_axis_delta(attacker_y, player_y),
    )
}

/// `overworld.md §6.2.4` presentation index. The section's presentation
/// notes state that "[t]he two outdoor cases share the flight machinery
/// but pass different effect-figure indices, selecting what is drawn at
/// each sampled position". This enum is that index; it selects the figure
/// only and never the line, the payload or the outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutdoorRangedAttackFigure {
    /// The ship's broadside. §6.2.4 attributes "a small solid burst
    /// travelling along the line" to it through "[a]n inherited
    /// description, not re-derived by the verification pass", and §6.2.5
    /// keeps the appearance itself unverified.
    SolidBurst,
    /// The breath attack: "a coloured spark cloud around each sampled
    /// position with no outline", under the same unverified caveat.
    SparkCloud,
}

/// `overworld.md §6.2.1` breath-attack range window: "[w]rapped absolute
/// separation from the party of at most three on **both** axes, inclusive
/// on each axis". Unlike the broadside there is no alignment requirement,
/// so an oblique offset such as `(3, 1)` is inside the window.
///
/// Takes **wrapped** deltas — see [`wrapped_axis_delta`].
pub const fn outdoor_breath_attack_in_range(wrapped_dx: i32, wrapped_dy: i32) -> bool {
    let abs_dx = wrapped_dx.abs();
    let abs_dy = wrapped_dy.abs();
    abs_dx <= OUTDOOR_RANGED_ATTACK_RANGE_CELLS && abs_dy <= OUTDOOR_RANGED_ATTACK_RANGE_CELLS
}

/// `catalogs/monster-bestiary.md`: the anchor of the four-frame
/// active-object sprite run the catalog publishes per bestiary class.
///
/// The catalog publishes the runs class by class rather than the
/// arithmetic. [`bestiary_sprite_run_first_frame`] derives the arithmetic
/// from it, and `bestiary_sprite_run_first_frame_is_linear_in_class`
/// checks it against four independently published anchors: Sea Serpent
/// class 18 → `0x88`, Dragon class 39 → `0xDC`, Sand Trap class 40 →
/// `0xE0`, Troll class 41 → `0xE4`.
pub const BESTIARY_SPRITE_RUN_ANCHOR_CLASS: u8 = 18;
pub const BESTIARY_SPRITE_RUN_ANCHOR_FIRST_FRAME: u8 = 0x88;
pub const BESTIARY_SPRITE_RUN_FRAMES: u8 = 4;

/// First frame of a bestiary class's sprite run, for classes at or above
/// [`BESTIARY_SPRITE_RUN_ANCHOR_CLASS`]. Lower ids are the party sprites
/// and the low bestiary block, which the anchor does not cover.
pub const fn bestiary_sprite_run_first_frame(class: u8) -> Option<u8> {
    if class < BESTIARY_SPRITE_RUN_ANCHOR_CLASS {
        return None;
    }
    let offset =
        (class - BESTIARY_SPRITE_RUN_ANCHOR_CLASS) as u16 * BESTIARY_SPRITE_RUN_FRAMES as u16;
    let frame = BESTIARY_SPRITE_RUN_ANCHOR_FIRST_FRAME as u16 + offset;
    if frame > u8::MAX as u16 {
        None
    } else {
        Some(frame as u8)
    }
}

/// `overworld.md §6.2.1` / `active-objects.md §8` breath attackers, by
/// **exact equality on the unmasked type byte**: the first frame of the
/// Sea Serpent run and the first frame of the Dragon run.
///
/// §6.2.1 makes the exactness contract: "**The breath test is exact
/// equality — not a range, not a masked family.** Sibling animation frames
/// `0x89..0x8B` and `0xDD..0xDF` never enter the breath branch."
pub const OUTDOOR_BREATH_ATTACKER_SEA_SERPENT_FIRST_FRAME: u8 = 0x88;
pub const OUTDOOR_BREATH_ATTACKER_DRAGON_FIRST_FRAME: u8 = 0xDC;

pub const fn outdoor_breath_attacker_class(type_byte: u8) -> bool {
    type_byte == OUTDOOR_BREATH_ATTACKER_SEA_SERPENT_FIRST_FRAME
        || type_byte == OUTDOOR_BREATH_ATTACKER_DRAGON_FIRST_FRAME
}

/// `active-objects.md §8`: the Sand Trap sprite run,
/// `catalogs/monster-bestiary.md` class 40. Its orthogonal-adjacency
/// reaction "reaches the shared impact-absorption stage directly and
/// **silently** — no message of any kind".
///
/// §8 is emphatic that this run is *not* a sea-serpent family: "an earlier
/// revision of this document, of `systems/encounters.md` and of
/// `systems/movement.md` called `0xE0..0xE3` the 'outdoor sea-serpent
/// adjacency family'. **That is withdrawn and was backwards.**"
pub const OUTDOOR_SAND_TRAP_SPRITE_RUN_FIRST: u8 = 0xE0;
pub const OUTDOOR_SAND_TRAP_SPRITE_RUN_LAST: u8 = 0xE3;

pub const fn outdoor_sand_trap_class(type_byte: u8) -> bool {
    matches!(
        type_byte,
        OUTDOOR_SAND_TRAP_SPRITE_RUN_FIRST..=OUTDOOR_SAND_TRAP_SPRITE_RUN_LAST
    )
}

/// `overworld.md §6.2.1` / `active-objects.md §8`: the "[s]hip-like
/// water-creature and pirate frames" that fire a broadside — the
/// `0x2C..=0x2F` facing-frame family.
///
/// §6.2.1: "**The broadside test is a masked family test**, deliberately
/// unlike the one above. The same walker uses both forms a few steps
/// apart. Do not generalise either rule to the other."
pub const fn outdoor_broadside_attacker_class(type_byte: u8) -> bool {
    matches!(type_byte, 0x2C..=0x2F)
}

/// `overworld.md §6.2.1`, as a single closed recognition table: which
/// outdoor ranged attack — if any — a slot's type byte and wrapped offset
/// from the party admit. The one-in-eight gate is *not* applied here; the
/// caller rolls it, because only the breath row has one.
///
/// These two rows are the whole outdoor census.
pub fn outdoor_ranged_attacker_figure(
    type_byte: u8,
    wrapped_dx: i32,
    wrapped_dy: i32,
) -> Option<OutdoorRangedAttackFigure> {
    if outdoor_breath_attacker_class(type_byte)
        && outdoor_breath_attack_in_range(wrapped_dx, wrapped_dy)
    {
        return Some(OutdoorRangedAttackFigure::SparkCloud);
    }
    if outdoor_broadside_attacker_class(type_byte)
        && crate::outdoor_water_creature_attack_aligned(wrapped_dx, wrapped_dy)
    {
        return Some(OutdoorRangedAttackFigure::SolidBurst);
    }
    None
}

/// `active-objects.md §8`: the broadside's announcement, printed "before
/// the shot".
///
/// §8 and `overworld.md §6.2` both require *a* boom message here without
/// fixing its wording, so this reuses the wording the tree already prints
/// for the player's own broadside in
/// [`crate::PlayState::fire_ship_broadside`] rather than introducing a
/// second spelling of the same cue. §6.2.4 is explicit about what this
/// message is *not*: "Neither the generic 'attacked' message nor any melee
/// narration belongs to these paths; that message is the
/// adjacent-engagement case."
pub const OUTDOOR_BROADSIDE_BOOM_MESSAGE: &str = "BOOOM! A broadside is fired at the party.";

/// Per-slot salt mixed into the breath attack's one-in-eight gate roll, so
/// the gate is decorrelated from the step planner's axis-choice roll on the
/// same slot and turn. `overworld.md §6.2.1` fixes the denominator and the
/// closed interval `[0, 7]` but not the generator, and this harness has no
/// cycle-accurate PRNG to reproduce.
pub const OUTDOOR_SERPENT_DRAGON_BREATH_SALT: u8 = 0x5B;

// -------------------------------------------------------------------------
// §6.2.2 — the sub-tile line, and how it is sampled
// -------------------------------------------------------------------------

/// `overworld.md §6.2.2` coordinate space: "There are sixteen sub-tile
/// units per cell; cell `c` owns the closed span of positions `16c + 8`
/// through `16c + 23`, and an endpoint converts to `16c + 16`."
pub const SUBTILE_UNITS_PER_CELL: i32 = 16;
pub const SUBTILE_CELL_SPAN_BASE: i32 = 8;
pub const SUBTILE_ENDPOINT_OFFSET: i32 = 16;

/// §6.2.2: "Positions outside the closed band `[8, 183]` on either axis
/// are off the eleven-by-eleven viewport; reaching one ends the walk and
/// reports a clear line."
pub const SUBTILE_BAND_LOW: i32 = SUBTILE_CELL_SPAN_BASE;
pub const SUBTILE_BAND_HIGH: i32 = SUBTILE_UNITS_PER_CELL * (VIEWPORT_SIDE as i32 - 1)
    + SUBTILE_CELL_SPAN_BASE
    + (SUBTILE_UNITS_PER_CELL - 1);

/// §6.2.2: "An accumulator carries one hundred times the row-per-column
/// slope, truncated toward zero exactly once at setup and then taken
/// positive."
pub const SUBTILE_SLOPE_SCALE: i32 = 100;

/// §6.2.2: "A shot with no column delta substitutes a very large constant
/// in place of the slope."
///
/// The text does not publish the constant, and does not have to: any value
/// exceeding [`SUBTILE_SLOPE_SCALE`] times the row-step count forces every
/// row step into the first iteration, which is the whole of the observable
/// behaviour, and every geometry either outdoor attack can reach is far
/// inside that bound. The value below satisfies it for any line within the
/// viewport; it is not a claim to reproduce a byte.
pub const SUBTILE_VERTICAL_SLOPE: i32 = 0x7FFF;

/// §6.2.2: "On the overworld it visits every thirteenth position, starting
/// with the first." §6.2.5 scopes that number to the overworld: "The
/// sampling interval is smaller in interior scenes, which changes every
/// tested-cell set above."
pub const OUTDOOR_SAMPLE_INTERVAL: usize = 13;

/// Generator step budget. §6.2.5: "The generator carries a step budget,
/// and only the first part of each path buffer is pre-filled with the run
/// terminator. A line long enough to exhaust either is unreachable at
/// breath and broadside ranges and was not analysed. ... treat long lines
/// as undefined and out of contract."
///
/// The longest in-viewport line emits `1 + 160 + 160 + 1` positions, so
/// this bound is unreachable for any line this module is asked to trace;
/// it exists so a malformed call cannot spin.
pub const SUBTILE_RUN_BUDGET: usize = 512;

/// §6.2.2: an endpoint cell converts to sub-tile position `16c + 16`.
pub const fn subtile_endpoint(cell: i32) -> i32 {
    cell * SUBTILE_UNITS_PER_CELL + SUBTILE_ENDPOINT_OFFSET
}

/// §6.2.2: "The inverse conversion, applied to every sampled position,
/// subtracts eight and divides by sixteen truncating toward zero."
pub const fn subtile_to_cell(position: i32) -> i32 {
    (position - SUBTILE_CELL_SPAN_BASE) / SUBTILE_UNITS_PER_CELL
}

/// §6.2.2: whether a sub-tile position is inside the closed band
/// `[8, 183]` that the eleven-by-eleven viewport covers.
pub const fn subtile_in_band(position: i32) -> bool {
    position >= SUBTILE_BAND_LOW && position <= SUBTILE_BAND_HIGH
}

/// `overworld.md §6.2.2` line generator, in sub-tile positions.
///
/// Both endpoints are **viewport cells**, `(column, row)`. The returned
/// run starts at the shooter's endpoint position — "[t]he starting
/// position is itself part of the emitted run" — and consecutive entries
/// "differ on exactly one axis, by one unit".
///
/// The column axis drives and the row axis is accumulated. Four properties
/// §6.2.2 calls contract, "because they are where an implementation that
/// substitutes a textbook line-drawing routine will diverge", fall out of
/// the loop below rather than being special-cased:
///
/// * the accumulator is initialised to the *full* slope, so "the first
///   thing after the start position is a **row** step whenever the shot
///   has any row delta at all";
/// * the row test is strictly greater than zero, so "[a]n accumulator
///   value of exactly zero does **not** step the row";
/// * the inner row advance is a `while`, not an `if`, so "a steep line
///   takes several row steps per column step";
/// * the run "always ends inside the target cell", with at most one
///   sub-tile column overshoot in the direction of travel — which happens
///   only for a shot with no column delta, since the loop always takes one
///   column step per iteration.
pub fn generate_outdoor_ranged_attack_run(
    from_cell: (i32, i32),
    to_cell: (i32, i32),
) -> Vec<(i32, i32)> {
    let mut column = subtile_endpoint(from_cell.0);
    let mut row = subtile_endpoint(from_cell.1);
    let delta_column = subtile_endpoint(to_cell.0) - column;
    let delta_row = subtile_endpoint(to_cell.1) - row;

    // A shot with no column delta still steps the column once, so the
    // direction there is a fixed choice rather than a sign. §6.2.2 bounds
    // that step to "never far enough to leave the target cell", which
    // holds either way: cell `c`'s span is `[16c + 8, 16c + 23]` and the
    // endpoint sits at `16c + 16`, eight units from the near edge and
    // seven from the far one.
    let column_step = if delta_column < 0 { -1 } else { 1 };
    let row_step = delta_row.signum();
    let slope = if delta_column == 0 {
        SUBTILE_VERTICAL_SLOPE
    } else {
        (SUBTILE_SLOPE_SCALE * delta_row / delta_column).abs()
    };

    let mut accumulator = slope;
    let mut rows_left = delta_row.abs();
    let mut columns_left = delta_column.abs();
    let mut run = Vec::with_capacity((2 + rows_left + columns_left) as usize);
    run.push((column, row));

    while run.len() < SUBTILE_RUN_BUDGET {
        while accumulator > 0 && rows_left > 0 {
            row += row_step;
            rows_left -= 1;
            accumulator -= SUBTILE_SLOPE_SCALE;
            run.push((column, row));
        }
        column += column_step;
        run.push((column, row));
        accumulator += slope;
        columns_left = (columns_left - 1).max(0);
        if rows_left == 0 && columns_left == 0 {
            break;
        }
    }
    run
}

/// One sampled position of a traced shot, converted to a viewport cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutdoorRangedAttackSample {
    pub column: i32,
    pub row: i32,
    /// Whether the sampling loop reaches the obstruction test for this
    /// sample. §6.2.2 fixes the order — convert, draw, advance the index,
    /// stop if the run has ended, "**and only then** test the current cell
    /// for obstruction" — so "[t]he last sampled cell is therefore never
    /// obstruction-tested".
    pub tested: bool,
}

/// `overworld.md §6.2.2` sampling walk: the cells the shot draws on, in
/// visit order, each flagged with whether it is obstruction-tested.
///
/// Endpoints are **viewport cells**, `(column, row)`. A sample whose
/// position leaves the closed band `[8, 183]` ends the walk without being
/// drawn or tested, which §6.2.2 reports as a clear line.
pub fn outdoor_ranged_attack_samples(
    from_cell: (i32, i32),
    to_cell: (i32, i32),
) -> Vec<OutdoorRangedAttackSample> {
    let run = generate_outdoor_ranged_attack_run(from_cell, to_cell);
    let mut samples = Vec::new();
    let mut index = 0usize;
    while let Some(&(column, row)) = run.get(index) {
        if !subtile_in_band(column) || !subtile_in_band(row) {
            break;
        }
        index += OUTDOOR_SAMPLE_INTERVAL;
        let ended = index >= run.len();
        samples.push(OutdoorRangedAttackSample {
            column: subtile_to_cell(column),
            row: subtile_to_cell(row),
            tested: !ended,
        });
        if ended {
            break;
        }
    }
    samples
}

/// Outcome of a traced ranged attack, per `overworld.md §6.2.2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutdoorRangedAttackOutcome {
    /// "a run that reaches the end of the generated line reports clear,
    /// and clear is what both outdoor call sites treat as a hit."
    Connects,
    /// "*Blocked* means the shot stops where it stopped and nothing
    /// further happens — no payload, no message, no state change."
    /// Carries the blocking viewport cell so the presentation can stop the
    /// projectile on it.
    Obstructed { column: i32, row: i32 },
}

/// `overworld.md §6.2.2` shared traced line. Both endpoints are
/// **viewport cells**; the party sits at the viewport centre and the
/// attacker at the centre offset by its wrapped separation.
///
/// `blocks` is the caller's per-cell obstruction predicate over the same
/// viewport grid — this module deliberately owns no passability set; see
/// the module header on the 46-id gap.
///
/// The shooter's own cell is exempted by coordinate comparison, not by
/// sample index: §6.2.2 says "[a] blocking cell whose coordinates equal
/// the shooter's own starting cell is ignored and the walk continues —
/// this is a coordinate comparison, not a 'skip the first sample' rule".
///
/// §6.2.3 warns against normalising the direction: "the column axis drives
/// the walk and sampling starts at the shooter's end, so exchanging the
/// endpoints can change which cells are tested. ... **trace from the
/// actual shooter's cell; do not normalise the direction and mirror the
/// result.**" The same routine serves the player's own shot with the
/// endpoints exchanged, which is why it takes both cells rather than an
/// offset.
pub fn trace_outdoor_ranged_attack(
    from_cell: (i32, i32),
    to_cell: (i32, i32),
    mut blocks: impl FnMut(i32, i32) -> bool,
) -> OutdoorRangedAttackOutcome {
    for sample in outdoor_ranged_attack_samples(from_cell, to_cell) {
        if !sample.tested || (sample.column, sample.row) == from_cell {
            continue;
        }
        if blocks(sample.column, sample.row) {
            return OutdoorRangedAttackOutcome::Obstructed {
                column: sample.column,
                row: sample.row,
            };
        }
    }
    OutdoorRangedAttackOutcome::Connects
}

/// The party's viewport cell. §6.2.2's worked example places "the party at
/// (5, 5)" of the eleven-by-eleven viewport.
pub const OUTDOOR_RANGED_ATTACK_PARTY_CELL: (i32, i32) =
    (VIEWPORT_PLAYER_COL as i32, VIEWPORT_PLAYER_ROW as i32);

/// The attacker's viewport cell for a wrapped attacker-to-party offset.
/// The deltas point *from* the attacker *to* the party, so the attacker
/// sits at the centre minus them — §6.2.2's example creature, "three
/// columns east and one row south of the party", has deltas `(-3, -1)` and
/// "sits at viewport cell (8, 6)".
pub const fn outdoor_ranged_attacker_viewport_cell(wrapped_dx: i32, wrapped_dy: i32) -> (i32, i32) {
    (
        OUTDOOR_RANGED_ATTACK_PARTY_CELL.0 - wrapped_dx,
        OUTDOOR_RANGED_ATTACK_PARTY_CELL.1 - wrapped_dy,
    )
}

// -------------------------------------------------------------------------
// §6.2.4 — the damage payload
// -------------------------------------------------------------------------

/// `overworld.md §6.2.4` / `vehicles.md §6`: while the party is aboard a
/// frigate the impact "draw[s] a uniform integer in the **closed interval
/// `[1, 30]`, inclusive on both ends**, and compare[s] it against the
/// ship's hull-condition byte".
pub const OUTDOOR_IMPACT_HULL_ROLL_LOW: u8 = 1;
pub const OUTDOOR_IMPACT_HULL_ROLL_HIGH: u8 = 30;

/// `overworld.md §6.2.4` whole-party pass: each qualifying member "draws
/// its own **fresh, independent** uniform integer in the **closed interval
/// `[1, 8]`, inclusive on both ends**".
pub const OUTDOOR_IMPACT_MEMBER_DAMAGE_LOW: u8 = 1;
pub const OUTDOOR_IMPACT_MEMBER_DAMAGE_HIGH: u8 = 8;

/// `overworld.md §6.2.4`: "The pass's own hard bound is six slots, indices
/// `0..5`."
pub const OUTDOOR_IMPACT_PARTY_PASS_SLOT_BOUND: usize = 6;

/// `formats/saved-gam.md §3.1` dead status letter, character record
/// `+0x0B`.
pub const PARTY_STATUS_DEAD: u8 = b'D';

/// `overworld.md §6.2.4` roster filter, stated exactly as published: "each
/// slot index that is **below the party-size byte** and whose **status
/// byte is not the dead marker**".
///
/// §6.2.5 forbids widening this into a living-letter whitelist: "Whether
/// 'not dead' is equivalent to membership in any particular set of living
/// letters is **not** established ... Implement the inequality, not a
/// living-letter whitelist." A sleeping or poisoned member is therefore
/// damaged, and so is a member holding any letter the engine has never
/// been seen to write.
pub const fn outdoor_impact_damages_member(status: u8) -> bool {
    status != PARTY_STATUS_DEAD
}

/// `overworld.md §6.2.4` frigate branch. Any marker in the hoisted or
/// furled ship families qualifies — "all four headings and both sail
/// states, eight values in total".
pub const fn outdoor_impact_absorbed_by_hull(transport: TransportState) -> bool {
    matches!(transport, TransportState::Ship { .. })
}

/// `overworld.md §6.2.4` / `vehicles.md §6` hull comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutdoorImpactHullOutcome {
    /// "Roll **strictly less than** the hull: subtract the roll from the
    /// hull ... **No party member loses hit points.** The hull cannot fall
    /// to zero or below by this route; the least it can hold afterwards is
    /// one."
    Absorbed { hull_after: u8 },
    /// "Roll **greater than or equal to** the hull: the ship is
    /// destroyed."
    ShipDestroyed,
}

pub const fn outdoor_impact_hull_outcome(roll: u8, hull: u8) -> OutdoorImpactHullOutcome {
    if roll < hull {
        OutdoorImpactHullOutcome::Absorbed {
            hull_after: hull - roll,
        }
    } else {
        OutdoorImpactHullOutcome::ShipDestroyed
    }
}

/// One member's share of the whole-party damage pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OutdoorImpactMemberDamage {
    pub slot: usize,
    /// The member's own fresh draw from the closed interval `[1, 8]`.
    pub roll: u8,
    /// What actually came off, after the clamp at zero.
    pub applied: u16,
    pub hp_after: u16,
    /// Whether this application is the one that killed the member.
    pub died: bool,
}

/// `overworld.md §6.2.4` stage two, as an outcome record. The stage "takes
/// no arguments and branches on exactly one thing: the party's transport
/// marker", and the outcome differs in kind between the two branches
/// rather than only in magnitude.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutdoorImpactAbsorption {
    /// Aboard a frigate, roll below hull: the hull ate it and no party
    /// member lost hit points.
    HullAbsorbed { roll: u8, hull_after: u8 },
    /// Aboard a frigate, roll at or above hull: the ship is gone and the
    /// `vehicles.md §6` loss-of-ship ladder ran.
    ShipDestroyed {
        roll: u8,
        fallback: ShipLossFallback,
        /// Damage taken on the drowning rung, if the ladder reached it.
        /// Each entry is one iteration's whole-party pass.
        drowning: Vec<Vec<OutdoorImpactMemberDamage>>,
    },
    /// Every other transport marker — "foot, horse, carpet, skiff, and the
    /// sprite-suppressed value" — takes the whole-party pass.
    PartyDamaged(Vec<OutdoorImpactMemberDamage>),
}

/// What one resolved outdoor ranged attack did, end to end.
///
/// The flight itself carries no payload — §6.2.2's return is only whether
/// the line ran clear — so `absorption` is `None` for a blocked shot and
/// carries the §6.2.4 result for a clear one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutdoorRangedAttackReport {
    /// Which effect figure the flight draws. Presentation only.
    pub figure: OutdoorRangedAttackFigure,
    /// The attacker's viewport cell, the end the trace starts from.
    pub attacker_cell: (i32, i32),
    pub outcome: OutdoorRangedAttackOutcome,
    pub absorption: Option<OutdoorImpactAbsorption>,
}

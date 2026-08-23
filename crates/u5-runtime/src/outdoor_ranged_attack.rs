//! Shared outdoor ranged-attack contract.
//!
//! `overworld.md §6.2` ("Creature ranged attacks") owns this contract and is
//! explicit that it is *one* procedure with several callers:
//!
//! > Some creatures attack the party at a distance instead of closing with
//! > it. Two outdoor cases exist, and they are resolved by the same
//! > procedure. ... The resolution is identical in both cases. A straight
//! > line is traced from the attacker's cell to the party's cell, drawn as
//! > an animated projectile travelling along that line, and tested cell by
//! > cell for obstructions as it goes. The attacker's own cell never
//! > obstructs its own shot. If the line reaches the party with no
//! > intervening blocker, the attack connects: the world tick runs and
//! > damage is applied to the party at its map coordinates. If an
//! > obstruction is met first, the shot stops there and nothing further
//! > happens.
//!
//! and on symmetry:
//!
//! > The player's own ranged attack uses the identical procedure with the
//! > endpoints exchanged, so **line-of-fire rules are symmetric between the
//! > party and the creatures**. An implementation should share one routine.
//!
//! The two callers live in the outdoor per-turn walker's first phase
//! (`active-objects.md §8`): the Sea Serpent / Dragon breath attack and the
//! ship-like water-creature / pirate broadside. This module owns the parts
//! of the contract the specification fully pins down — the range window,
//! the wrapped-torus deltas the windows are measured on, the presentation
//! index, and the traced line itself. The parts it does not pin down are
//! `require_*` panic seams rather than invented values, each citing
//! `cleak/u5-spec#90`.
//!
//! # Why this trace is new code rather than a reuse
//!
//! Before adding [`trace_outdoor_ranged_attack`] the tree was searched for
//! an existing map-cell line walk. There is none. The nearest three are
//! each something else:
//!
//! - [`crate::PlayState::ship_broadside_target_slot`] walks a cardinal run
//!   of cells on the torus, but it searches for the first *active object*
//!   and never consults terrain, so it cannot answer "is the shot
//!   obstructed".
//! - [`crate::PlayState::town_fire_target`] does test terrain, through
//!   [`crate::surface_tile_blocks_projectile`], but on the bounded 32x32
//!   town grid with no wrapping, and it returns a target classification
//!   rather than an obstruction outcome.
//! - The `draw_line` helpers in the viewport and display-driver paths are
//!   pixel-space rasterizers for drawing, not map-cell walks.
//!
//! So this is one routine where there was none, not a second copy of one.
//! What it does *not* yet do is absorb the player's own broadside.
//! `overworld.md §6.2` says "[t]he player's own ranged attack uses the
//! identical procedure with the endpoints exchanged ... An implementation
//! should share one routine", which means `ship_broadside_target_slot`
//! should eventually resolve through here and gain the terrain-obstruction
//! test it currently lacks. That migration changes player-facing broadside
//! behaviour, so it is deliberately not part of this change.

/// Module-local `i32` view of [`crate::WORLD_SIDE`], matching
/// [`crate::directed_step`]'s convention so the torus arithmetic here does
/// not need a cast per call.
const WORLD_SIDE: i32 = crate::WORLD_SIDE as i32;

/// `overworld.md §6.2` ranged-attack window, in cells. Both outdoor cases
/// use the same number: the breath attack requires the attacker to be
/// "within three cells of the party on **both** axes", and the broadside
/// requires alignment "within three cells" along the shared axis.
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

/// `overworld.md §6.2` presentation index. The section's two presentation
/// notes state that the outdoor cases "share the flight machinery but
/// differ in the figure drawn at each sampled point along the line,
/// selected by a single index the caller supplies". This enum is that
/// index; it selects the figure only and never the line or the outcome.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutdoorRangedAttackFigure {
    /// "the ship's broadside draws a small solid burst travelling along the
    /// line".
    SolidBurst,
    /// "the breath attack paints a coloured spark cloud around each sampled
    /// point with no outline".
    SparkCloud,
}

/// `overworld.md §6.2` breath-attack range window: the Sea Serpent / Dragon
/// case triggers only when the attacker is "[w]ithin three cells of the
/// party on **both** axes". Both axes are tested independently against
/// [`OUTDOOR_RANGED_ATTACK_RANGE_CELLS`]; unlike the broadside there is no
/// alignment requirement, so an oblique offset such as `(3, 1)` is inside
/// the window.
///
/// Takes **wrapped** deltas — see [`wrapped_axis_delta`].
pub const fn outdoor_breath_attack_in_range(wrapped_dx: i32, wrapped_dy: i32) -> bool {
    let abs_dx = wrapped_dx.abs();
    let abs_dy = wrapped_dy.abs();
    abs_dx <= OUTDOOR_RANGED_ATTACK_RANGE_CELLS && abs_dy <= OUTDOOR_RANGED_ATTACK_RANGE_CELLS
}

/// `active-objects.md §8` / `encounters.md` payload table: the Dragon
/// first frame, the one breath-attack attacker class the specification
/// identifies unambiguously.
///
/// `encounters.md`'s payload-family table gives `0xDC..0xDF` as the
/// "Dragon sprite run" and adds that "the first frame also participates in
/// a special outdoor near-range pull/effect path" — that path is §8's
/// breath attack, and the first frame is `0xDC`.
///
/// Note this byte's storage domain. §8 warns that "[t]he `0xDC` comparison
/// ... is made against the moving active-object's type byte, where it is
/// the first Dragon frame. This is a different storage domain from a live
/// terrain byte with the same numeric value." The same numeric value as a
/// *terrain* byte is the moon-gate / local-light family, and is unrelated.
pub const OUTDOOR_BREATH_ATTACKER_DRAGON_FIRST_FRAME: u8 = 0xDC;

/// `active-objects.md §8`: returns `true` for the "[s]hip-like
/// water-creature and pirate frames" that fire a broadside — the
/// `0x2C..=0x2F` facing-frame family, which `encounters.md`'s payload table
/// names "Pirate-ship / water-creature facing frames".
///
/// Any frame in the family qualifies, not only the first: §8 scopes this
/// reaction to "[s]hip-like water-creature and pirate frames" as a group,
/// in contrast with the breath attack, which it scopes to "first-frame
/// hostile classes".
pub const fn outdoor_broadside_attacker_class(type_byte: u8) -> bool {
    matches!(type_byte, 0x2C..=0x2F)
}

/// `active-objects.md §8`: the broadside's announcement, printed "before
/// the shot".
///
/// §8 and `overworld.md §6.2` both require *a* boom message here without
/// fixing its wording, so this reuses the wording the tree already prints
/// for the player's own broadside in
/// [`crate::PlayState::fire_ship_broadside`] rather than introducing a
/// second spelling of the same cue. §6.2 is explicit about what this
/// message is *not*: "Neither the generic 'attacked' message nor any melee
/// narration belongs to these paths; that message is the
/// adjacent-engagement case."
pub const OUTDOOR_BROADSIDE_BOOM_MESSAGE: &str = "BOOOM! A broadside is fired at the party.";

/// Per-slot salt mixed into the breath attack's one-in-eight gate roll, so
/// the gate is decorrelated from the step planner's axis-choice roll on the
/// same slot and turn. `active-objects.md §8` fixes the denominator but not
/// the generator, and this harness has no cycle-accurate PRNG to reproduce.
pub const OUTDOOR_SERPENT_DRAGON_BREATH_SALT: u8 = 0x5B;

/// Outcome of a traced ranged attack, per `overworld.md §6.2`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutdoorRangedAttackOutcome {
    /// "the line reaches the party with no intervening blocker" — the
    /// attack connects. The caller runs the world tick and applies damage.
    Connects,
    /// "an obstruction is met first, the shot stops there and nothing
    /// further happens." Carries the blocking cell so the presentation can
    /// stop the projectile on it.
    Obstructed { x: u8, y: u8 },
}

/// `overworld.md §6.2` shared traced line: walks the cells strictly between
/// the attacker and the party, in order, and reports the first obstruction.
///
/// Two boundary rules come straight from the section. "The attacker's own
/// cell never obstructs its own shot", so the attacker's cell is not
/// tested. The party's own cell is the destination rather than an
/// obstruction — the section's connect condition is "no *intervening*
/// blocker" — so it is not tested either.
///
/// `blocks` is the caller's per-cell obstruction predicate. Because §6.2
/// makes the endpoints exchangeable ("line-of-fire rules are symmetric
/// between the party and the creatures"), the same routine serves a
/// party-to-creature shot by swapping `from` and `to`.
///
/// # Panics
///
/// Panics for an oblique line — one where the two axis deltas are neither
/// equal in magnitude nor is one of them zero. See
/// [`require_outdoor_ranged_attack_rasterization`].
pub fn trace_outdoor_ranged_attack(
    from: (u8, u8),
    to: (u8, u8),
    mut blocks: impl FnMut(u8, u8) -> bool,
) -> OutdoorRangedAttackOutcome {
    let (dx, dy) = wrapped_deltas_to_player(from.0, from.1, to.0, to.1);
    let steps = outdoor_ranged_attack_step_count(dx, dy);
    let (step_x, step_y) = (dx.signum(), dy.signum());

    // Cells strictly between the endpoints: step 0 is the attacker's own
    // cell and step `steps` is the party's cell, and §6.2 excludes both.
    for step in 1..steps {
        let x = (from.0 as i32 + step_x * step).rem_euclid(WORLD_SIDE) as u8;
        let y = (from.1 as i32 + step_y * step).rem_euclid(WORLD_SIDE) as u8;
        if blocks(x, y) {
            return OutdoorRangedAttackOutcome::Obstructed { x, y };
        }
    }
    OutdoorRangedAttackOutcome::Connects
}

/// Number of one-cell steps along the traced line, for the line geometries
/// `overworld.md §6.2` pins down unambiguously.
///
/// A cardinal line (one delta zero) and an exact diagonal line (equal
/// magnitudes) have the same cell sequence under every straight-line
/// rasterization, so they need no rasterization rule to be reproducible.
/// An oblique line does not, and §6.2 does not supply one — see
/// [`require_outdoor_ranged_attack_rasterization`].
fn outdoor_ranged_attack_step_count(wrapped_dx: i32, wrapped_dy: i32) -> i32 {
    let abs_dx = wrapped_dx.abs();
    let abs_dy = wrapped_dy.abs();
    // Cardinal (one delta zero) and exact-diagonal (equal magnitudes) lines
    // all have `max(|dx|, |dy|)` steps; the two cases differ in which cells
    // those steps visit, not in how many there are.
    if abs_dx == 0 || abs_dy == 0 || abs_dx == abs_dy {
        abs_dx.max(abs_dy)
    } else {
        require_outdoor_ranged_attack_rasterization(wrapped_dx, wrapped_dy)
    }
}

/// Spec gap: `overworld.md §6.2` does not specify how an **oblique** ranged
/// attack line is rasterized.
///
/// The section says only that "[a] straight line is traced from the
/// attacker's cell to the party's cell ... and tested cell by cell for
/// obstructions as it goes". For a cardinal or exact-diagonal line that is
/// enough — every straight-line rasterization visits the same cells. The
/// breath attack's window is "within three cells ... on **both** axes" with
/// no alignment requirement, so it admits oblique offsets such as `(3, 1)`,
/// and there the choice of rasterization decides which cells are tested and
/// therefore whether the shot is blocked at all.
///
/// Picking Bresenham (or any other line walk) here would invent the one
/// thing the specification withholds, and it would do it inside an
/// obstruction test whose whole purpose is to decide hits. It refuses
/// instead.
pub fn require_outdoor_ranged_attack_rasterization(wrapped_dx: i32, wrapped_dy: i32) -> ! {
    panic!(
        "outdoor ranged attack from an oblique offset (wrapped dx={wrapped_dx}, dy={wrapped_dy}): \
         overworld.md §6.2 specifies that a straight line is traced and tested cell by cell, but \
         not how an oblique line is rasterized. Cardinal and exact-diagonal lines are \
         rasterization-independent and are handled; an oblique line is not, and choosing a line \
         walk here would invent the cells the obstruction test reads. See cleak/u5-spec#90"
    )
}

/// Spec gap: neither `overworld.md §6.2` nor `active-objects.md §8`
/// specifies the **damage** a connecting outdoor ranged attack deals.
///
/// §6.2 says only "the world tick runs and damage is applied to the party
/// at its map coordinates", and §8 says only that "the same per-turn
/// finishers as other outdoor encounter effects run and damage is applied".
/// Neither gives an amount, a roll, a die range, or a rule for which party
/// members are affected, and `encounters.md` and `combat.md` do not supply
/// one for this path either.
///
/// A damage number invented here would be indistinguishable from a
/// specified one at every call site that reads it, so it refuses instead.
pub fn require_outdoor_ranged_attack_damage(figure: OutdoorRangedAttackFigure) -> ! {
    panic!(
        "a {figure:?} outdoor ranged attack connected, but the damage it deals is unspecified: \
         overworld.md §6.2 says only that \"damage is applied to the party at its map \
         coordinates\" and active-objects.md §8 says only that \"damage is applied\", with no \
         amount, roll, range, or party-member selection rule in either, nor in encounters.md or \
         combat.md. Inventing a damage value is a forbidden fallback. See cleak/u5-spec#90"
    )
}

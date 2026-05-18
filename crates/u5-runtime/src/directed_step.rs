//! Outdoor directed step planner per `active-objects.md` §8.
//!
//! For wrapped overworld coordinates on the 256-cell torus, this module
//! computes the one-cell X and Y steps that would reduce wrapped distance
//! to the player, and exposes a one-bit "axis first" selector. Caller wires
//! the random roll, walkability check, target-cell check, and committer.

// Module-local i32 view of the canonical `crate::WORLD_SIDE` so the
// rem_euclid arithmetic below does not require a usize cast per call.
const WORLD_SIDE: i32 = crate::WORLD_SIDE as i32;

/// Cardinal axis pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

/// One-cell step offsets toward the player along each axis on the 256-cell
/// torus. Each component is `-1`, `0`, or `+1`. A zero component means the
/// actor is already aligned with the player on that axis.
pub fn directed_step_offsets(actor_x: u8, actor_y: u8, player_x: u8, player_y: u8) -> (i8, i8) {
    let dx = wrapped_step_axis(actor_x, player_x);
    let dy = wrapped_step_axis(actor_y, player_y);
    (dx, dy)
}

/// Per-axis one-cell step toward `player` on the 256-cell torus. Returns
/// `0` when already aligned, `+1` when the player is the shorter way
/// forward, and `-1` when the player is the shorter way back. Ties (when
/// both directions are equidistant on the wrapped axis) prefer the
/// forward step.
fn wrapped_step_axis(actor: u8, player: u8) -> i8 {
    if actor == player {
        return 0;
    }
    let forward = (player as i32 - actor as i32).rem_euclid(WORLD_SIDE);
    let backward = (actor as i32 - player as i32).rem_euclid(WORLD_SIDE);
    if forward <= backward { 1 } else { -1 }
}

/// Axis-first selector for the one-bit random roll documented in §8: bit 0
/// of the roll picks X first, bit 1 picks Y first. The caller falls back to
/// the other axis if the chosen direction is blocked.
pub const fn axis_first_choice(rng_bit: u8) -> Axis {
    if rng_bit & 1 == 0 { Axis::X } else { Axis::Y }
}

/// Per-destination-tile chance gate for ordinary outdoor movers per
/// `active-objects.md` §8: returns `Some(d)` when the committer must roll a
/// one-in-`d` chance for the tile, or `None` when the move proceeds
/// immediately once validation accepts the candidate. Returns `None` for
/// tile ids outside `0x04..=0x1F`.
pub const fn terrain_chance_gate_denominator(tile: u8) -> Option<u8> {
    match tile {
        0x04 | 0x06..=0x08 | 0x1E..=0x1F => Some(2),
        0x09..=0x0F => Some(3),
        _ => None,
    }
}

/// Per `active-objects.md` §8: ship-like water-creature frames `0x2C..=0x2F`
/// and the Bat/Daemon/Dragon/Mongbat first-frame type bytes bypass the
/// post-validation terrain chance gate.
pub const fn type_bypasses_terrain_chance_gate(type_byte: u8) -> bool {
    matches!(type_byte, 0x2C..=0x2F | 0x94 | 0xD8 | 0xDC | 0xF0,)
}

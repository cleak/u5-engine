//! Ship broadside helper constants and the depletion-clear rule per
//! `vehicles.md §7`. The shared overworld-`F`-Fire path delegates to
//! the ship-broadside helper; this module exposes the spec's
//! observable constants and the per-hit byte mutation contract.

/// `vehicles.md §7`: returns `true` when a requested fire direction
/// is a legal broadside relative to the ship's facing. Bow/stern
/// shots refuse with "Fire broadsides only!"; only directions
/// perpendicular to the ship's heading are accepted. Both arguments
/// use the published facing convention `0` north, `1` east,
/// `2` south, `3` west.
pub const fn ship_broadside_direction_accepted(ship_facing: u8, fire_direction: u8) -> bool {
    let f = ship_facing & 0x03;
    let d = fire_direction & 0x03;
    // Perpendicular when the low bit (axis selector) differs.
    (f & 0x01) != (d & 0x01)
}

/// `vehicles.md §7`: broadside trace length in cells. The projectile
/// scans up to three cells from the ship in the chosen direction.
pub const SHIP_BROADSIDE_RANGE_CELLS: u8 = 3;

/// `vehicles.md §7`: per-hit damage roll. A successful broadside
/// subtracts a random `1..=20` amount from the target object's
/// active-object byte `+5`.
pub const SHIP_BROADSIDE_DAMAGE_MIN: u8 = 1;
pub const SHIP_BROADSIDE_DAMAGE_MAX: u8 = 20;

/// `vehicles.md §7`: active-object descriptor offset of the
/// hull-condition / depletion byte that broadside damage subtracts
/// from. For ship/frigate targets this byte doubles as hull
/// condition; non-ship targets carry an ordinary family-specific
/// meaning that broadside damage still depletes generically.
pub const SHIP_BROADSIDE_DEPLETION_BYTE_OFFSET: usize = 5;

/// `vehicles.md §7`: per-hit depletion-and-clear rule. Subtracts
/// `damage` from the target's `+5` byte. If the subtraction wraps
/// the byte into the high-bit range (signed underflow), the slot is
/// cleared and the helper returns `None`. Otherwise the byte stays
/// in place with the reduced value, returned as `Some(remaining)`.
pub const fn ship_broadside_apply_damage(byte: u8, damage: u8) -> Option<u8> {
    let next = byte.wrapping_sub(damage);
    if (next & 0x80) != 0 || damage > byte {
        None
    } else {
        Some(next)
    }
}

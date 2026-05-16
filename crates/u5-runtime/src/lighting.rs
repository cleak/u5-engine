//! Lighting helpers per `lighting.md`. Combines the cached ambient value
//! with personal light sources (torch, light spell) and exposes the
//! counter-decay and Ignite-duration rules. The original engine stores
//! ambient on a 2..=50 scale with 51+ as a "do-not-recompute" sentinel.

use crate::*;

/// `lighting.md §4`: raise `ambient` to the brightest active personal-light
/// floor. When both counters are nonzero, the torch floor (18) dominates
/// the spell-light floor (10); zero counters contribute nothing.
pub const fn apply_personal_light(ambient: u8, torch_counter: u8, light_spell_counter: u8) -> u8 {
    let mut value = ambient;
    if torch_counter != 0 && value < TORCH_LIGHT_FLOOR {
        value = TORCH_LIGHT_FLOOR;
    }
    if light_spell_counter != 0 && value < LIGHT_SPELL_FLOOR {
        value = LIGHT_SPELL_FLOOR;
    }
    value
}

/// `lighting.md §6`: dungeon-mode blackout gate. Without either personal
/// light source the corridor view and Look description are suppressed.
pub const fn dungeon_blackout(torch_counter: u8, light_spell_counter: u8) -> bool {
    torch_counter == 0 && light_spell_counter == 0
}

/// `lighting.md §5` per-turn cadence class. The light counter spends
/// one unit per ordinary town/dungeon/combat turn and two units per
/// ordinary overworld turn; longer waits spend the wait's requested
/// increment directly. Mode-zero refreshes recompute ambient lighting
/// only and do not spend counter duration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightDecayCadence {
    /// Town, dungeon, or combat turn — 1 counter unit.
    TownDungeonCombatTurn,
    /// Ordinary overworld turn — 2 counter units.
    OverworldTurn,
    /// Long wait — explicit caller-supplied increment passes through.
    Wait(u8),
    /// Mode-zero refresh — no counter spend.
    ModeZeroRefresh,
}

/// `lighting.md §5`: turn-cadence -> counter-spend mapping. Returns
/// the number of counter units the per-turn cleanup decays by.
pub const fn light_counter_increment(cadence: LightDecayCadence) -> u8 {
    match cadence {
        LightDecayCadence::TownDungeonCombatTurn => 1,
        LightDecayCadence::OverworldTurn => 2,
        LightDecayCadence::Wait(units) => units,
        LightDecayCadence::ModeZeroRefresh => 0,
    }
}

/// `lighting.md §5`: saturating per-turn light counter decrement. The
/// counter is the turn-local light/torch byte; the increment is how many
/// counter units the current turn spends (1 for town/dungeon/combat, 2
/// for ordinary overworld, longer for waits).
pub const fn decay_light_counter(counter: u8, increment: u8) -> u8 {
    if counter > increment {
        counter - increment
    } else {
        0
    }
}

/// `lighting.md §3`: ambient values 51..=255 are the "do-not-recompute"
/// sentinel band. The cleanup routine leaves a cached ambient byte alone
/// when it observes one of these values.
pub const fn ambient_is_sentinel(ambient: u8) -> bool {
    ambient >= DAYLIGHT_SENTINEL_MIN
}

/// `lighting.md §3` overworld special-underfoot blackout markers. The
/// overworld loop's environmental branch forces ambient light to zero
/// while the party stands on the `0xFF` underfoot tile state, unless
/// the opaque `0x0E` state tag exempts the pass.
pub const OVERWORLD_UNDERFOOT_BLACKOUT_TILE: u8 = 0xFF;
pub const OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG: u8 = 0x0E;

/// `lighting.md §3`: returns `true` when the overworld loop's
/// underfoot blackout branch forces ambient to zero. Active when the
/// underfoot tile is the special `0xFF` state and the opaque-state
/// tag is not the `0x0E` exemption.
pub const fn overworld_underfoot_forces_dark(underfoot_tile: u8, opaque_state_tag: u8) -> bool {
    underfoot_tile == OVERWORLD_UNDERFOOT_BLACKOUT_TILE
        && opaque_state_tag != OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG
}

/// `lighting.md §8`: Ignite outside dungeon scenes sets the torch counter
/// to a fixed 240-unit value, overwriting any prior burn.
pub const fn ignite_torch_surface() -> u8 {
    SURFACE_TORCH_DURATION
}

/// `lighting.md §8`: Ignite in dungeon scenes adds a random 112..=127
/// counter unit increment to the current torch counter, capped at 255.
/// `roll_112_to_127` is the caller-supplied uniform `[112, 127]` random
/// roll.
pub const fn ignite_torch_dungeon(current: u8, roll_112_to_127: u8) -> u8 {
    current.saturating_add(roll_112_to_127)
}

/// `lighting.md §8` dungeon Ignite random-increment bounds.
pub const DUNGEON_TORCH_INCREMENT_MIN: u8 = 112;
pub const DUNGEON_TORCH_INCREMENT_MAX: u8 = 127;

/// `lighting.md §8`: *In Lor* (ordinary Light spell) overwrites the
/// light-spell counter with 100 units; *Vas Lor* (Great Light) overwrites
/// it with 255 units. Light spells do not stack with prior spell-light
/// duration.
pub const LIGHT_SPELL_DURATION: u8 = 100;
pub const GREAT_LIGHT_SPELL_DURATION: u8 = 255;

/// `time.md §6` Stage-1 base daylight value computed from hour/minute and
/// scene. Returns the cached ambient base before personal-light floors:
///   - underworld plane or dungeon depth (`z != 0`) → full darkness;
///   - hour < 5 or hour > 19 → full darkness;
///   - hour == 5 → dawn gradient indexed by `minute / 10`;
///   - hour == 19 → dusk gradient indexed by `(59 - minute) / 10`;
///   - otherwise (06..=18 surface) → full daylight.
/// Caller still applies the personal-light floors in [`apply_personal_light`]
/// and the [`ambient_is_sentinel`] skip rule before writing the result.
pub const fn daylight_base_value(hour: u8, minute: u8, underworld: bool, depth_z: u8) -> u8 {
    if underworld || depth_z != 0 {
        return FULL_DARKNESS;
    }
    if hour < 5 || hour > 19 {
        return FULL_DARKNESS;
    }
    if hour == 5 {
        let raw = (minute / 10) as usize;
        let idx = if raw > 5 { 5 } else { raw };
        return DAWN_DUSK_LIGHT[idx];
    }
    if hour == 19 {
        let raw = ((59 - minute) / 10) as usize;
        let idx = if raw > 5 { 5 } else { raw };
        return DAWN_DUSK_LIGHT[idx];
    }
    FULL_DAYLIGHT
}

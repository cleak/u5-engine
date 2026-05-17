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

/// `lighting.md §5`: returns the light-counter spend for `cadence`
/// after the per-turn cleanup applies the same `tag_byte` timing-tag
/// adjustment it applies to the minute counter. `T` suppresses the
/// light-counter write entirely (returns `None`); `Q` halves the
/// spend with the same one-unit floor the minute increment uses;
/// other tag bytes pass the cadence spend through unchanged.
pub const fn light_counter_spend_with_tag(
    cadence: LightDecayCadence,
    tag_byte: u8,
) -> Option<u8> {
    apply_timing_tag_increment(light_counter_increment(cadence), tag_byte)
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

/// `overworld.md §natural-gates`: minimum ambient light at which the
/// per-frame natural-moongate animator may stamp a frame. Below this
/// threshold the animator resets its phase instead of drawing.
/// Equal to [`FULL_DAYLIGHT`]: the animator's "daytime threshold"
/// matches the daytime ambient value the time/lighting cleanup
/// writes during the surface daytime band.
pub const MOONGATE_ANIMATOR_DAYTIME_THRESHOLD: u8 = FULL_DAYLIGHT;

/// `overworld.md §natural-gates`: returns `true` when the per-frame
/// natural-moongate animator may stamp a frame at this ambient
/// light value. Returns `false` for ambient values below the
/// daytime threshold, where the animator resets its phase instead.
pub const fn moongate_animator_render_eligible(ambient_light: u8) -> bool {
    ambient_light >= MOONGATE_ANIMATOR_DAYTIME_THRESHOLD
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

/// `lighting.md §8` dungeon Ignite random-increment bounds. The
/// increment rolls a uniform `[MIN, MAX]` for 16 outcomes (the
/// low nibble of a random byte plus MIN). Anchor MAX to MIN +
/// DUNGEON_CELL_LOW_NIBBLE_MASK so the 16-value width derives
/// from the same low-nibble mask the dungeon-cell parser uses.
pub const DUNGEON_TORCH_INCREMENT_MIN: u8 = 112;
pub const DUNGEON_TORCH_INCREMENT_MAX: u8 =
    DUNGEON_TORCH_INCREMENT_MIN + crate::DUNGEON_CELL_LOW_NIBBLE_MASK;

/// `lighting.md §8`: *In Lor* (ordinary Light spell) overwrites the
/// light-spell counter with 100 units; *Vas Lor* (Great Light) overwrites
/// it with 255 units. Light spells do not stack with prior spell-light
/// duration.
pub const LIGHT_SPELL_DURATION: u8 = 100;
pub const GREAT_LIGHT_SPELL_DURATION: u8 = 255;

/// `lighting.md §3`: hour at which the surface dawn gradient is played
/// (06:00 transition window — hours below this are full darkness).
pub const DAWN_HOUR: u8 = 5;

/// `lighting.md §3`: hour at which the surface dusk gradient is played
/// (hours above this are full darkness).
pub const DUSK_HOUR: u8 = 19;

/// `lighting.md §3`: each dawn/dusk gradient step covers this many
/// minutes. Six ten-minute levels exactly fill the dawn or dusk hour.
pub const DAWN_DUSK_STEP_MINUTES: u8 = 10;

/// `lighting.md §3`: highest index into [`DAWN_DUSK_LIGHT`]. The dawn
/// and dusk paths clamp to this index so a minute byte that has
/// somehow advanced past 59 still selects the last published level
/// rather than indexing out of the table.
pub const DAWN_DUSK_LAST_INDEX: usize = DAWN_DUSK_LIGHT.len() - 1;

/// `lighting.md §3`: last in-hour minute. Dusk reverses the gradient
/// by indexing on `(LAST_IN_HOUR_MINUTE - minute) / DAWN_DUSK_STEP_MINUTES`
/// so 19:00 starts at the brightest gradient level and 19:59 ends at
/// the darkest.
pub const LAST_IN_HOUR_MINUTE: u8 = 59;

/// `time.md §6` Stage-1 base daylight value computed from hour/minute and
/// scene. Returns the cached ambient base before personal-light floors:
///   - underworld plane or dungeon depth (`z != 0`) → full darkness;
///   - hour < DAWN_HOUR or hour > DUSK_HOUR → full darkness;
///   - hour == DAWN_HOUR → dawn gradient indexed by `minute / DAWN_DUSK_STEP_MINUTES`;
///   - hour == DUSK_HOUR → dusk gradient indexed by
///     `(LAST_IN_HOUR_MINUTE - minute) / DAWN_DUSK_STEP_MINUTES`;
///   - otherwise (06..=18 surface) → full daylight.
/// Caller still applies the personal-light floors in [`apply_personal_light`]
/// and the [`ambient_is_sentinel`] skip rule before writing the result.
pub const fn daylight_base_value(hour: u8, minute: u8, underworld: bool, depth_z: u8) -> u8 {
    if underworld || depth_z != 0 {
        return FULL_DARKNESS;
    }
    if hour < DAWN_HOUR || hour > DUSK_HOUR {
        return FULL_DARKNESS;
    }
    if hour == DAWN_HOUR {
        let raw = (minute / DAWN_DUSK_STEP_MINUTES) as usize;
        let idx = if raw > DAWN_DUSK_LAST_INDEX {
            DAWN_DUSK_LAST_INDEX
        } else {
            raw
        };
        return DAWN_DUSK_LIGHT[idx];
    }
    if hour == DUSK_HOUR {
        let raw = ((LAST_IN_HOUR_MINUTE - minute) / DAWN_DUSK_STEP_MINUTES) as usize;
        let idx = if raw > DAWN_DUSK_LAST_INDEX {
            DAWN_DUSK_LAST_INDEX
        } else {
            raw
        };
        return DAWN_DUSK_LIGHT[idx];
    }
    FULL_DAYLIGHT
}

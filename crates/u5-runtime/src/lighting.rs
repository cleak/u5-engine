//! Lighting helpers per `lighting.md`. Combines the cached ambient value
//! with personal light sources (torch, light spell) and exposes the
//! counter-decay and Ignite-duration rules. The original engine stores
//! ambient on a 2..=50 scale with 51+ as a "do-not-recompute" sentinel.

use crate::*;

/// `lighting.md §4`: personal light combines as a maximum, expressed as
/// two ordered floors — ambient first, then the spell raises it to at
/// least [`LIGHT_SPELL_FLOOR`] (18), then the torch raises it to at least
/// [`TORCH_LIGHT_FLOOR`] (10). Neither floor ever *lowers* ambient, so
/// neither does anything in daylight, and the torch floor is a complete
/// no-op while a light spell burns.
///
/// Both counters are read as booleans: a torch with one minute left
/// lights exactly as far as a fresh one.
pub const fn apply_personal_light(ambient: u8, torch_counter: u8, light_spell_counter: u8) -> u8 {
    let mut value = ambient;
    if light_spell_counter != 0 && value < LIGHT_SPELL_FLOOR {
        value = LIGHT_SPELL_FLOOR;
    }
    if torch_counter != 0 && value < TORCH_LIGHT_FLOOR {
        value = TORCH_LIGHT_FLOOR;
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
pub const fn light_counter_spend_with_tag(cadence: LightDecayCadence, tag_byte: u8) -> Option<u8> {
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

// `overworld.md §9` (spec HEAD c00bf63) withdraws the natural-moongate
// "daylight threshold" entirely. The threshold was both inverted and
// misattributed: the scratch block it guarded belongs to the night-time
// rotating light beacon of `visibility.md §12.6`, whose gate runs only
// while ambient is *strictly below* full daylight, and which never holds
// a moongate. Nothing on the moongate path reads ambient light; gate
// presence is decided by the hour alone through
// `crate::moongate::natural_moongate_counter_step`.

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
/// it with 255 units (the largest representable byte — the counter
/// saturates rather than wrapping). Light spells do not stack with
/// prior spell-light duration.
pub const LIGHT_SPELL_DURATION: u8 = 100;
/// `lighting.md §8`: Great Light overwrites the light-spell counter
/// with the largest representable byte value, so the counter saturates
/// at the byte width rather than restating `255` as a bare literal.
/// Anchored to [`u8::MAX`].
pub const GREAT_LIGHT_SPELL_DURATION: u8 = u8::MAX;

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
/// the darkest. Anchored to [`crate::MINUTES_PER_HOUR`] - 1 so the
/// last minute of an hour derives from the published hour length.
pub const LAST_IN_HOUR_MINUTE: u8 = crate::MINUTES_PER_HOUR - 1;

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
    daylight_base_value_for_scene(hour, minute, underworld, depth_z, crate::SCENE_OVERWORLD)
}

/// `lighting.md §3` scene byte pinned to [`FULL_DARKNESS`] at every
/// hour: "One location, scene twenty-five (Ararat), is pinned to the
/// dark value regardless of the hour."
///
/// §10 records that the name attached to the scene number comes from
/// the gazetteer rather than a fresh trace, but "the forced-dark rule
/// for that scene number is firm", so the pin is anchored to the
/// number through [`crate::SCENE_ARARAT`].
pub const FORCED_DARK_SCENE: u8 = crate::SCENE_ARARAT;

/// `lighting.md §3` Stage-1 base daylight value, including the scene
/// forced-dark test.
///
/// "**Scope of the forced-dark tests.** Both tests run *before* the
/// clock is consulted, and there are exactly two of them." The plane /
/// floor test is the `underworld`/`depth_z` pair; the scene test
/// "pins scene twenty-five (Ararat) to 2 at every hour, independently
/// of Z".
///
/// The pin is applied to the *base* value only: §3's torch and
/// light-spell floors "then apply normally on top of a forced-dark
/// result", so a light source still lifts Ararat to its personal-light
/// floor. Callers apply those floors in [`apply_personal_light`] and
/// the [`ambient_is_sentinel`] skip rule before writing the result.
/// `lighting.md §3`: the forced-dark plane/floor test fires on any Z with its
/// high bit set, i.e. any value above 127. Both the Underworld plane and a
/// below-entry town floor carry `0xFF`; dungeon level indices count upward from
/// zero and never reach it.
pub const FORCED_DARK_MIN_DEPTH_Z: u8 = 128;

pub const fn daylight_base_value_for_scene(
    hour: u8,
    minute: u8,
    underworld: bool,
    depth_z: u8,
    scene_byte: u8,
) -> u8 {
    if scene_byte == FORCED_DARK_SCENE {
        return FULL_DARKNESS;
    }
    // `lighting.md §3`: "The plane / floor test is on the party's Z value, read
    // as an unsigned byte: any Z with its high bit set - that is, any value
    // above one hundred twenty-seven - pins the ambient value at 2 for every
    // hour." That selects the Underworld plane and a below-entry town floor,
    // both of which carry Z `0xFF`.
    //
    // It explicitly "does **not** select ordinary dungeon levels: a dungeon
    // level index counts upward from zero at the top of the stack, so it never
    // sets the high bit". This engine tested `depth_z != 0`, which is the
    // earlier wording the section retracts, and which would force darkness on
    // every dungeon level below the first.
    if underworld || depth_z >= FORCED_DARK_MIN_DEPTH_Z {
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

#[cfg(test)]
mod forced_dark_scene_tests {
    use super::*;

    #[test]
    fn ararat_is_pinned_to_full_darkness_at_every_hour() {
        // `lighting.md §3`: "One location, scene twenty-five (Ararat),
        // is pinned to the dark value regardless of the hour", and
        // "**The scene test** pins scene twenty-five (Ararat) to 2 at
        // every hour, independently of Z."
        assert_eq!(FORCED_DARK_SCENE, 25);
        for hour in 0u8..24 {
            for minute in [0u8, 9, 10, 29, 30, 49, 50, 59] {
                assert_eq!(
                    daylight_base_value_for_scene(hour, minute, false, 0, FORCED_DARK_SCENE),
                    FULL_DARKNESS,
                    "{hour}:{minute:02} in scene {FORCED_DARK_SCENE}"
                );
            }
        }
    }

    #[test]
    fn the_scene_test_runs_before_the_clock_and_independently_of_z() {
        // §3: "Both tests run *before* the clock is consulted", and the
        // scene test applies "independently of Z".
        assert_eq!(
            daylight_base_value_for_scene(12, 0, true, 0xFF, FORCED_DARK_SCENE),
            FULL_DARKNESS
        );
        // Noon in an ordinary scene still reaches full daylight, so the
        // pin is scene-specific rather than a blanket dark value.
        assert_eq!(
            daylight_base_value_for_scene(12, 0, false, 0, crate::SCENE_OVERWORLD),
            FULL_DAYLIGHT
        );
        assert_eq!(
            daylight_base_value_for_scene(12, 0, false, 0, FORCED_DARK_SCENE - 1),
            FULL_DAYLIGHT
        );
        assert_eq!(
            daylight_base_value_for_scene(12, 0, false, 0, FORCED_DARK_SCENE + 1),
            FULL_DAYLIGHT
        );
        // The four-argument form is the overworld-scene projection of
        // the five-argument one.
        assert_eq!(
            daylight_base_value(12, 0, false, 0),
            daylight_base_value_for_scene(12, 0, false, 0, crate::SCENE_OVERWORLD)
        );
    }

    #[test]
    fn personal_light_floors_still_apply_on_top_of_the_ararat_pin() {
        // §3: "The torch and light-spell floors of Section 4 then apply
        // normally on top of a forced-dark result", so the pin lands on
        // the base value, not on the final one.
        let base = daylight_base_value_for_scene(12, 0, false, 0, FORCED_DARK_SCENE);
        assert_eq!(base, FULL_DARKNESS);
        assert_eq!(apply_personal_light(base, 1, 0), TORCH_LIGHT_FLOOR);
        assert_eq!(apply_personal_light(base, 0, 1), LIGHT_SPELL_FLOOR);
        assert_eq!(apply_personal_light(base, 0, 0), FULL_DARKNESS);
    }
}

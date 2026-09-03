//! Natural-moongate live-tile refresh helpers per `overworld.md` §9.

use crate::{DAWN_HOUR, DUSK_HOUR, NATURAL_MOONGATE_COUNTER_MAX};

/// `overworld.md §9` shared natural-moongate gate-presence counter
/// outcome for one cleanup pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NaturalMoongateCounterStep {
    /// Night hours `20..=23` and `0..=4`: counter increases toward
    /// [`NATURAL_MOONGATE_COUNTER_MAX`].
    Increase,
    /// Daytime hours `5..=19`: counter decreases toward zero.
    Decrease,
}

/// `overworld.md §9`: classify the cleanup pass for the shared
/// gate-presence counter from the current hour. Hours outside the
/// `lighting.md §3` daylight band `DAWN_HOUR..=DUSK_HOUR` are the
/// night band that grows the counter; daylight hours shrink it.
pub const fn natural_moongate_counter_step(hour: u8) -> NaturalMoongateCounterStep {
    if hour < DAWN_HOUR || hour > DUSK_HOUR {
        NaturalMoongateCounterStep::Increase
    } else {
        NaturalMoongateCounterStep::Decrease
    }
}

/// `overworld.md §9`: advance the shared counter for one cleanup pass.
/// Increases saturate at [`NATURAL_MOONGATE_COUNTER_MAX`]; decreases
/// floor at zero.
pub const fn natural_moongate_advance_counter(counter: u8, hour: u8) -> u8 {
    match natural_moongate_counter_step(hour) {
        NaturalMoongateCounterStep::Increase => {
            if counter >= NATURAL_MOONGATE_COUNTER_MAX {
                NATURAL_MOONGATE_COUNTER_MAX
            } else {
                counter + 1
            }
        }
        NaturalMoongateCounterStep::Decrease => {
            if counter == 0 {
                0
            } else {
                counter - 1
            }
        }
    }
}

/// `overworld.md §9` saved-Moonstone-slot eligibility check for the
/// natural-moongate live-tile refresh. Surface (overworld) eligibility
/// requires `slot_scene == current_scene`, `slot_z == current_z`, and
/// the saved `(slot_x, slot_y)` falling inside the active 32-by-32
/// loaded chunk window. Interior and town-family non-combat scenes use
/// only the scene/Z match (`window` is then `None`).
pub fn natural_moongate_slot_eligible(
    slot_scene: u8,
    slot_z: u8,
    slot_x: u8,
    slot_y: u8,
    current_scene: u8,
    current_z: u8,
    chunk_window: Option<(u8, u8, u8, u8)>,
) -> bool {
    if slot_scene != current_scene || slot_z != current_z {
        return false;
    }
    let Some((x0, y0, w, h)) = chunk_window else {
        return true;
    };
    coordinate_in_wrapping_window(slot_x, x0, w) && coordinate_in_wrapping_window(slot_y, y0, h)
}

const fn coordinate_in_wrapping_window(coordinate: u8, start: u8, len: u8) -> bool {
    len != 0 && coordinate.wrapping_sub(start) < len
}

/// `overworld.md §9`: live-gate entry hook secondary outcome — when the
/// hook clears the `0xDC` cell, an hour-`0` minute-`<10` window
/// dispatches to the shrine/urn meditate overlay; otherwise the cached
/// moon-glyph slot warps the party. Returns `true` for the meditate
/// dispatch.
pub const fn natural_moongate_dispatches_meditate(hour: u8, minute: u8) -> bool {
    hour == 0 && minute < 10
}

/// `overworld.md §9`: pick the cached-moon-glyph slot the live-gate
/// entry hook reads when warping after clearing the `0xDC` cell.
/// Before noon (`hour < 12`) the hook reads the first cached glyph;
/// from noon onward it reads the second. Returns `0` for first glyph,
/// `1` for second.
pub const fn natural_moongate_cached_glyph_slot(hour: u8) -> u8 {
    if hour < 12 { 0 } else { 1 }
}

/// `moons.md §2.2` cache-only "no gate for this moon" encoding.
///
/// This is **not** a glyph-table entry. §2.2 states plainly: "There is
/// no sentinel byte in either table. An implementation that reserves a
/// high-bit value for 'off horizon' is modelling something the tables
/// do not contain; whether a moon is drawn is decided solely by the
/// hour-driven visibility rule." The published day tables below
/// therefore hold nothing but ASCII digits `b'0'..=b'7'`.
///
/// The engine still needs a byte to park in its two-entry glyph cache
/// when a caller explicitly says "no Moonstone slot for this moon", so
/// these two high-bit values remain as an internal cache encoding that
/// [`moonstone_slot_from_glyph_byte`] decodes back to `None`.
pub const TRAMMEL_OFF_HORIZON_SENTINEL: u8 = 0xF0;

/// `moons.md §2.2` cache-only "no gate for this moon" encoding for
/// Felucca. See [`TRAMMEL_OFF_HORIZON_SENTINEL`]; it is not a table
/// entry either.
pub const FELUCCA_OFF_HORIZON_SENTINEL: u8 = 0x80;

/// `moons.md §2.2`: "The glyph identity for each moon is table-driven,
/// **indexed by the calendar day of the month, one through
/// twenty-eight**. It is not indexed by the hour." Twenty-eight
/// entries, every one an ASCII phase digit `b'0'..=b'7'` mapping to
/// Moonstone slot index `0..=7`.
///
/// Trammel repeats every fourteen days, running the full eight-phase
/// cycle twice per month, with the published pattern
/// `0, 1, 1, 2, 2, 3, 3, 4, 5, 5, 6, 6, 7, 7`.
///
/// An earlier revision of `moons.md §2` published twenty-four-entry
/// hour tables carrying off-horizon sentinel bytes; §2.2 retracts both
/// statements ("the tables are not twenty-four-entry hour tables, and
/// they contain no off-horizon sentinel entries at all"). Index this
/// table with `day - 1`.
pub const TRAMMEL_GLYPH_BY_DAY: [u8; MOON_GLYPH_DAYS_PER_MONTH] = [
    b'0', b'1', b'1', b'2', b'2', b'3', b'3', b'4', b'5', b'5', b'6', b'6', b'7', b'7', b'0', b'1',
    b'1', b'2', b'2', b'3', b'3', b'4', b'5', b'5', b'6', b'6', b'7', b'7',
];

/// `moons.md §2.2` day-indexed Felucca glyph table.
///
/// Felucca repeats every nine days with the pattern
/// `0, 0, 1, 2, 3, 4, 5, 6, 7`. Twenty-eight is not a multiple of
/// nine, so the month ends part-way through the fourth repetition and
/// day twenty-eight is `b'0'`, the first entry of the next repetition.
/// The calendar wrap from day twenty-eight back to day one is a real
/// discontinuity the original does not smooth.
pub const FELUCCA_GLYPH_BY_DAY: [u8; MOON_GLYPH_DAYS_PER_MONTH] = [
    b'0', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'0', b'0', b'1', b'2', b'3', b'4', b'5',
    b'6', b'7', b'0', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'0',
];

/// `moons.md §2.2` length of both published glyph tables: the calendar
/// month is twenty-eight days, "one through twenty-eight". "There is
/// no day zero."
pub const MOON_GLYPH_DAYS_PER_MONTH: usize = 28;

/// `moons.md §2.2` cache pair meaning "neither moon selects a
/// Moonstone slot". Built from the two cache-only encodings above, not
/// from any table entry. Callers that cannot fail on an out-of-range
/// day byte park this pair instead of synthesising a phase.
pub const MOON_GLYPH_CACHE_NO_GATE: [u8; 2] =
    [TRAMMEL_OFF_HORIZON_SENTINEL, FELUCCA_OFF_HORIZON_SENTINEL];

/// `moons.md §2.2`: raw cached glyph bytes for a status/moon refresh on
/// the supplied calendar day of the month (`1..=28`).
///
/// "The day index is the saved day-of-month byte, which the per-turn
/// clock keeps in the range one through twenty-eight ... There is no
/// day zero, so an implementation should treat a zero or out-of-range
/// day as a save-data error rather than looking up a twenty-ninth
/// entry." A day outside `1..=28` therefore yields `None` rather than
/// a synthesised sentinel pair.
pub const fn cached_moon_glyph_bytes_for_day(day: u8) -> Option<[u8; 2]> {
    if day == 0 || day as usize > MOON_GLYPH_DAYS_PER_MONTH {
        return None;
    }
    let index = day as usize - 1;
    Some([TRAMMEL_GLYPH_BY_DAY[index], FELUCCA_GLYPH_BY_DAY[index]])
}

/// `moons.md §2.2`: decode a cached glyph byte into a Moonstone slot
/// index (`0..=7`). Returns `None` for the cache-only "no gate"
/// encodings ([`TRAMMEL_OFF_HORIZON_SENTINEL`] /
/// [`FELUCCA_OFF_HORIZON_SENTINEL`]) and for any other unexpected
/// byte. Every byte the published day tables contain decodes to a
/// slot, because those tables hold nothing but `b'0'..=b'7'`.
pub const fn moonstone_slot_from_glyph_byte(byte: u8) -> Option<usize> {
    if byte & 0x80 != 0 {
        return None;
    }
    if byte < b'0' || byte > b'7' {
        return None;
    }
    Some((byte - b'0') as usize)
}

/// `moons.md §2.2`: Trammel Moonstone-slot index for a calendar day of
/// the month (`1..=28`). Returns `None` only for an out-of-range day —
/// Trammel has a phase on every published day.
pub const fn trammel_moonstone_slot_for_day(day: u8) -> Option<usize> {
    if day == 0 || day as usize > MOON_GLYPH_DAYS_PER_MONTH {
        return None;
    }
    moonstone_slot_from_glyph_byte(TRAMMEL_GLYPH_BY_DAY[day as usize - 1])
}

/// `moons.md §2.2`: Felucca Moonstone-slot index for a calendar day of
/// the month (`1..=28`).
pub const fn felucca_moonstone_slot_for_day(day: u8) -> Option<usize> {
    if day == 0 || day as usize > MOON_GLYPH_DAYS_PER_MONTH {
        return None;
    }
    moonstone_slot_from_glyph_byte(FELUCCA_GLYPH_BY_DAY[day as usize - 1])
}

/// `overworld.md §9` live moon-gate terrain byte. Eligible saved
/// Moonstone slots are stamped with this tile while the shared
/// gate-presence counter is nonzero; when the counter wanes to
/// zero, the live cell is restored to
/// [`NATURAL_MOONGATE_UNDERLYING_TILE`]. The overworld live-gate
/// entry hook also keys its enter-portal branch off this byte.
pub const NATURAL_MOONGATE_LIVE_TILE: u8 = 0xDC;

/// `overworld.md §9` underlying terrain byte the natural-moongate
/// refresh restores when the gate-presence counter wanes to zero.
/// Also the byte the live-gate entry hook writes back when the
/// portal animation finishes.
pub const NATURAL_MOONGATE_UNDERLYING_TILE: u8 = 5;

/// `overworld.md §9` fixed narrative gate location: surface plane
/// world coordinate `(233, 235)` for the post-action special-tile
/// branch.
pub const NARRATIVE_GATE_X: u8 = 233;
pub const NARRATIVE_GATE_Y: u8 = 235;

/// `formats/saved-gam.md §7.2` first tile id of the contiguous burial
/// terrain band. The Moonstone bury action accepts any tile in
/// `MOONSTONE_BURIAL_BAND_FIRST..=MOONSTONE_BURIAL_BAND_LAST`.
pub const MOONSTONE_BURIAL_BAND_FIRST: u8 = 4;
/// `formats/saved-gam.md §7.2` last tile id of the contiguous burial
/// terrain band.
pub const MOONSTONE_BURIAL_BAND_LAST: u8 = 10;
/// `formats/saved-gam.md §7.2` first single accepted Moonstone burial
/// tile id outside the contiguous band (sand-style overworld tile).
pub const MOONSTONE_BURIAL_TILE_EXTRA_A: u8 = 44;
/// `formats/saved-gam.md §7.2` second single accepted Moonstone
/// burial tile id outside the contiguous band.
pub const MOONSTONE_BURIAL_TILE_EXTRA_B: u8 = 45;

/// `formats/saved-gam.md §7.2`: Moonstone burying is accepted only
/// outside dungeon/combat scenes and only when the tile under the
/// party is one of these world-tile ids: `4..10`, `44`, or `45`.
pub const fn moonstone_burial_tile_accepted(tile_id: u8) -> bool {
    matches!(
        tile_id,
        MOONSTONE_BURIAL_BAND_FIRST
            ..=MOONSTONE_BURIAL_BAND_LAST
                | MOONSTONE_BURIAL_TILE_EXTRA_A
                | MOONSTONE_BURIAL_TILE_EXTRA_B
    )
}

/// `formats/saved-gam.md §7.2`: invalid Gate Travel target sentinel
/// written into the destination-scene byte of an unused Moonstone slot.
pub const MOONSTONE_GATE_INVALID_SCENE: u8 = 0xFF;

/// `overworld.md §8` / `§8.1`: Britannia `(54, 138)` is **not** the falls
/// trigger. It is the cell the handler tests *after* it has already printed
/// the banner and force-stepped the party two cells south, and the only
/// coordinate on either plane whose landing also writes the underworld plane
/// (`RETRACTIONS.md` R320). The trigger itself is [`is_waterfall_tile`].
pub const SURFACE_CHASM_X: u8 = 54;
pub const SURFACE_CHASM_Y: u8 = 138;
/// Whether a *landing* coordinate opens the plane gate. `§8.1` notes the gate
/// "never tests the plane", which is harmless in stock data because no
/// underworld brink can reach column 54.
pub const fn is_surface_chasm_cell(x: u8, y: u8) -> bool {
    x == SURFACE_CHASM_X && y == SURFACE_CHASM_Y
}

/// `catalogs/tile-catalog.md §3.1`: the waterfall family, a four-frame
/// animated run. `overworld.md §8` makes any of the four the falls trigger,
/// "either south of the party or under it ... on **both** planes"
/// (`RETRACTIONS.md` R320).
pub const WATERFALL_TILE_FIRST: u8 = 0xD4;
pub const WATERFALL_TILE_LAST: u8 = 0xD7;
pub const fn is_waterfall_tile(tile: u8) -> bool {
    tile >= WATERFALL_TILE_FIRST && tile <= WATERFALL_TILE_LAST
}

/// `overworld.md §8` "Whirlpool": the fixed underworld emergence coordinate
/// an outdoor whirlpool engagement forces a non-foot party to.
pub const WHIRLPOOL_UNDERWORLD_EMERGENCE_X: usize = 34;
pub const WHIRLPOOL_UNDERWORLD_EMERGENCE_Y: usize = 18;

/// `overworld.md §8`, forced-movement table, "Surface chasm/falls" row: the
/// handler pushes the party **two cells south**, with one world tick between
/// the two steps.
pub const OVERWORLD_FALLS_FORCED_STEPS_SOUTH: usize = 2;

/// `overworld.md §8` + `RETRACTIONS.md` R321: the per-member fall check draws
/// the shared skewed closed-interval `1..30` roll — a uniform `0..60` halved
/// with truncation and zero promoted to one, the helper `combat.md §9.1`
/// publishes — and applies `1 HP` when the member's Dexterity byte is **less
/// than or equal to** the roll.
///
/// The earlier `0..255` byte with a strictly-greater gate made fall damage
/// nearly impossible, where a Dexterity-20 member is really hit about one
/// time in three.
pub const WORLD_PLANE_FALL_SAVE_RAW_ROLL_LOW: u8 = 0;
pub const WORLD_PLANE_FALL_SAVE_RAW_ROLL_HIGH: u8 = 60;
/// `overworld.md §8`: the flat one point of damage a failed check applies.
pub const WORLD_PLANE_FALL_DAMAGE: u8 = 1;
/// Inclusive, deliberately: this is **not** the outdoor K-Klimb contract,
/// which draws a flat `1..30` and gates strictly
/// (`doors-and-z-transitions.md §12.1`, "Do not share one implementation
/// between the two").
pub const fn world_plane_fall_member_takes_damage(dexterity: u8, roll_1_to_30: u8) -> bool {
    dexterity <= roll_1_to_30
}

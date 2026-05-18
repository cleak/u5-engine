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

/// `catalogs/gazetteer.md §8`: confirmed surface chasm at Britannia
/// `(54, 138)` — stepping onto this cell damages the party, swaps the
/// plane to the Underworld, and reseeds active objects.
pub const SURFACE_CHASM_X: u8 = 54;
pub const SURFACE_CHASM_Y: u8 = 138;
pub const fn is_surface_chasm_cell(x: u8, y: u8) -> bool {
    x == SURFACE_CHASM_X && y == SURFACE_CHASM_Y
}

/// `overworld.md §2`: maximum per-member fall-damage value rolled when
/// the Britannia chasm or whirlpool plane writer transitions a
/// conscious party member to the Underworld. The runtime draws a
/// uniform `1..=WORLD_PLANE_FALL_DAMAGE_MAX` roll for each conscious
/// member.
pub const WORLD_PLANE_FALL_DAMAGE_MAX: u8 = 5;

//! Active-object encode/decode for SAVED.OOL mirroring + write helpers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

/// `active-objects.md §3` field offsets within the eight-byte record.
pub const ACTIVE_OBJECT_FIELD_TYPE: usize = 0;
pub const ACTIVE_OBJECT_FIELD_TILE: usize = 1;
pub const ACTIVE_OBJECT_FIELD_X: usize = 2;
pub const ACTIVE_OBJECT_FIELD_Y: usize = 3;
pub const ACTIVE_OBJECT_FIELD_Z: usize = 4;
pub const ACTIVE_OBJECT_FIELD_DEP1: usize = 5;
pub const ACTIVE_OBJECT_FIELD_PHASE: usize = 6;
pub const ACTIVE_OBJECT_FIELD_DEP3: usize = 7;

/// `active-objects.md §2` per-pass iteration order. The renderer
/// walks slots from `OOL_SLOTS - 1` down to `0` so lower-indexed
/// slots paint on top — guaranteeing the player (slot zero) draws
/// over every other entity in the same cell. The per-tick animator
/// walks slots from `0` up to `OOL_SLOTS - 1`; iteration order there
/// affects only deterministic tie-breaking, not correctness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveObjectPassOrder {
    /// Renderer pass — high index to low (`31..=0`).
    RendererHighToLow,
    /// Per-tick animator pass — low index to high (`0..=31`).
    AnimatorLowToHigh,
}

impl ActiveObjectPassOrder {
    /// Returns the (start, end_inclusive, step_descending) tuple for
    /// the requested pass. `step_descending == true` means iterate
    /// from `start` down to `end_inclusive`.
    pub const fn iteration(self) -> (usize, usize, bool) {
        match self {
            Self::RendererHighToLow => (OOL_SLOTS - 1, 0, true),
            Self::AnimatorLowToHigh => (0, OOL_SLOTS - 1, false),
        }
    }
}

/// `active-objects.md §10` overworld off-screen pruning radius. The
/// per-turn walker frees outdoor active-object slots whose distance
/// from the scroll bases (Manhattan in either axis) is greater than
/// this many cells.
pub const ACTIVE_OBJECT_PRUNE_RADIUS: i32 = 32;

/// `active-objects.md §10`: predicate for the overworld per-turn
/// pruning sweep. Returns `true` when an outdoor slot at
/// `(slot_x, slot_y)` is more than [`ACTIVE_OBJECT_PRUNE_RADIUS`] cells
/// from the scroll base in either axis and should be freed.
pub const fn active_object_should_prune(
    slot_x: i32,
    slot_y: i32,
    scroll_base_x: i32,
    scroll_base_y: i32,
) -> bool {
    let dx = slot_x - scroll_base_x;
    let dy = slot_y - scroll_base_y;
    let abs_dx = if dx < 0 { -dx } else { dx };
    let abs_dy = if dy < 0 { -dy } else { dy };
    abs_dx > ACTIVE_OBJECT_PRUNE_RADIUS || abs_dy > ACTIVE_OBJECT_PRUNE_RADIUS
}

/// `active-objects.md §11` save-image active-object region length.
pub const ACTIVE_OBJECT_SAVE_BYTES: usize = 256;

/// `active-objects.md §8`: animator outcome for one slot's phase
/// counter (low nibble of byte 6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationPhaseStep {
    /// All-ones nibble — slot is steady; the animator skips it.
    Steady,
    /// Mid-cycle. The animator decrements the nibble and writes back
    /// the new value (always `>= 0`); the renderer combines this with
    /// the tile class to pick a frame.
    Decrement(u8),
    /// Cycle ended. The slot is eligible for an AI tick this pass.
    AiEligible,
}

/// All-ones nibble in byte 6 marks "steady, do not animate" per
/// `active-objects.md §8`.
pub const ANIMATION_PHASE_STEADY_NIBBLE: u8 = 0x0F;

/// `active-objects.md §8`: classify the low nibble of an active-object
/// phase byte (byte 6) into the animator's per-tick outcome. Higher
/// bits of the input are masked off; callers may pass either the raw
/// byte or just the nibble.
pub const fn animation_phase_step(phase_byte: u8) -> AnimationPhaseStep {
    let nibble = phase_byte & 0x0F;
    if nibble == ANIMATION_PHASE_STEADY_NIBBLE {
        AnimationPhaseStep::Steady
    } else if nibble == 0 {
        AnimationPhaseStep::AiEligible
    } else {
        AnimationPhaseStep::Decrement(nibble - 1)
    }
}

/// `active-objects.md §4`: deterministic eviction phase a candidate
/// qualifies for, or `None` if the byte-0 / on-screen combination is not
/// a victim in any phase. Phases 1..=5 are the off-screen passes (with
/// phase 1 being the empty-slot path); phases 6..=10 are the
/// any-on-screen passes. Byte 0x00 (empty slot) returns `Some(1)`. Byte
/// 0xB5 is universally protected and returns `None` regardless.
pub const fn active_object_eviction_phase(byte0: u8, off_screen: bool) -> Option<u8> {
    if byte0 == ACTIVE_OBJECT_PROTECTED_TYPE_BYTE {
        return None;
    }
    if byte0 == 0x00 {
        return Some(1);
    }
    if off_screen {
        match byte0 {
            0x01..=0x0F => Some(2),
            0x80..=0xFF => Some(3), // 0xB5 already returned None above.
            0x10..=0x11 => Some(4),
            0x30..=0x7F => Some(5),
            _ => Some(10),
        }
    } else {
        match byte0 {
            0x01..=0x0F => Some(6),
            0x80..=0xFF => Some(7),
            0x10..=0x11 => Some(8),
            0x30..=0x7F => Some(9),
            _ => Some(10),
        }
    }
}

pub fn refresh_saved_ool_mirrors_for_load(game_dir: &Path) -> io::Result<()> {
    let bytes = read_saved_ool_bytes(game_dir)?;
    fs::write(game_dir.join("BRIT.OOL"), &bytes[..OOL_PLANE_LEN])?;
    fs::write(game_dir.join("UNDER.OOL"), &bytes[OOL_PLANE_LEN..])?;
    Ok(())
}

pub fn read_saved_ool_bytes(game_dir: &Path) -> io::Result<Vec<u8>> {
    let path = game_dir.join("SAVED.OOL");
    let bytes = read(&path)?;
    if bytes.len() != SAVED_OOL_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "SAVED.OOL must be {SAVED_OOL_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(bytes)
}

pub fn encode_active_object_table(objects: &[ActiveObject]) -> io::Result<Vec<u8>> {
    if objects.len() > OOL_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "active-object table has {} slots, expected at most {OOL_SLOTS}",
                objects.len()
            ),
        ));
    }
    let mut bytes = vec![0; OOL_PLANE_LEN];
    for (slot, object) in objects.iter().copied().enumerate() {
        write_active_object_record(&mut bytes, slot, object)?;
    }
    Ok(bytes)
}

pub fn encode_ool_plane_objects(objects: &[ActiveObject]) -> io::Result<Vec<u8>> {
    if objects.len() > OOL_SLOTS - 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "world overlay has {} non-player slots, expected at most {}",
                objects.len(),
                OOL_SLOTS - 1
            ),
        ));
    }
    let mut bytes = vec![0; OOL_PLANE_LEN];
    for (index, object) in objects.iter().copied().enumerate() {
        write_active_object_record(&mut bytes, index + 1, object)?;
    }
    Ok(bytes)
}

pub fn write_active_object_record(
    bytes: &mut [u8],
    slot: usize,
    object: ActiveObject,
) -> io::Result<()> {
    if slot >= OOL_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("active-object slot {slot} is outside 0..{}", OOL_SLOTS - 1),
        ));
    }
    let x = u8::try_from(object.x).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object slot {slot} x coordinate {} is outside 0..255",
                object.x
            ),
        )
    })?;
    let y = u8::try_from(object.y).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "active-object slot {slot} y coordinate {} is outside 0..255",
                object.y
            ),
        )
    })?;
    let offset = slot * OOL_RECORD_LEN;
    bytes[offset] = object.type_byte;
    bytes[offset + 1] = object.tile;
    bytes[offset + 2] = x;
    bytes[offset + 3] = y;
    bytes[offset + 4] = object.z as u8;
    bytes[offset + 5] = object.aux1;
    bytes[offset + 6] = object.phase;
    bytes[offset + 7] = object.aux3;
    Ok(())
}

pub fn decode_ool_plane_objects(bytes: &[u8]) -> io::Result<Vec<ActiveObject>> {
    decode_active_object_table(bytes, "OOL plane table")
}

pub fn decode_saved_active_objects(bytes: &[u8]) -> io::Result<Vec<ActiveObject>> {
    let end = SAVE_ACTIVE_OBJECTS_OFFSET + OOL_PLANE_LEN;
    let table = bytes
        .get(SAVE_ACTIVE_OBJECTS_OFFSET..end)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "SAVED.GAM is too short"))?;
    decode_active_object_table(table, "SAVED.GAM active-object table")
}

pub fn decode_active_object_table(bytes: &[u8], label: &str) -> io::Result<Vec<ActiveObject>> {
    if bytes.len() != OOL_PLANE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} must be {OOL_PLANE_LEN} bytes, got {}", bytes.len()),
        ));
    }

    let mut objects = Vec::with_capacity(OOL_SLOTS - 1);
    for (slot, record) in bytes.chunks_exact(OOL_RECORD_LEN).enumerate() {
        let type_byte = record[0];
        if slot == 0 {
            continue;
        }
        objects.push(ActiveObject {
            type_byte,
            tile: record[1],
            x: record[2] as usize,
            y: record[3] as usize,
            z: record[4] as i8,
            phase: record[6],
            aux1: record[5],
            aux3: record[7],
        });
    }
    Ok(objects)
}

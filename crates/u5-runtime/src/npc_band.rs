//! `formats/saved-gam.md §12` - the saved NPC family band at
//! `0x07B4..0x105F`.
//!
//! §12: "The final 2,220 bytes, `0x07B4..0x105F`, contain the current
//! location's live NPC family and several other fields. Preserve this band
//! together with the active-object table. Journey Onward restores it as
//! part of the single 4,192-byte image; it does not reload the location's
//! `.NPC` source records or reconstruct the saved route state. **An
//! active-object-only writer cannot restore a working town cast.**"
//!
//! `RETRACTIONS.md` R341 makes the band durable state rather than scratch:
//! "the entire NPC runtime family - schedule table, per-NPC runtime block,
//! path queues and read pointers, type array, stuck counters - lies
//! **inside the 4192-byte save image**", and a producer that "declines to
//! persist the band at all ... resumes a **completely empty location**".
//!
//! `RETRACTIONS.md` R395 (issue #217) keeps the 1,024-byte world-tile
//! render buffer *out* of the band: "The 1,024-byte location/world tile
//! buffer is outside the 4,192-byte save image." Nothing here writes one.

use std::io;

use crate::constants::*;
use crate::npc_runtime::{
    NPC_PATH_DIR_EAST, NPC_PATH_DIR_NORTH, NPC_PATH_DIR_SOUTH, NPC_PATH_DIR_WEST, RuntimeNpc,
};

/// `formats/saved-gam.md §12.3`: "A town-family scene (`1..32` at file
/// `0x02ED`) reaches the **preserving** entry mode." Only there is the band
/// the location's live cast; a world save (`scene 0`) or a dungeon save
/// (`scene > 32`) must leave it alone so it is not corrupted.
pub const fn scene_byte_is_town_family(scene_byte: u8) -> bool {
    scene_byte >= SAVE_TOWN_FAMILY_SCENE_FIRST && scene_byte <= SAVE_TOWN_FAMILY_SCENE_LAST
}

/// Translate an engine BFS direction code (`npc-schedules.md §8.2`:
/// `1` west, `2` south, `3` east, `4` north) into the saved route
/// direction of `formats/saved-gam.md §12.2` (`1` east, `2` north,
/// `3` west, `4` south). The two encodings are different and an engine
/// that writes its own codes produces a route the original walks
/// backwards.
pub const fn saved_route_direction_from_engine(code: u8) -> Option<u8> {
    match code {
        NPC_PATH_DIR_WEST => Some(SAVE_NPC_BAND_ROUTE_DIR_WEST),
        NPC_PATH_DIR_SOUTH => Some(SAVE_NPC_BAND_ROUTE_DIR_SOUTH),
        NPC_PATH_DIR_EAST => Some(SAVE_NPC_BAND_ROUTE_DIR_EAST),
        NPC_PATH_DIR_NORTH => Some(SAVE_NPC_BAND_ROUTE_DIR_NORTH),
        _ => None,
    }
}

/// Inverse of [`saved_route_direction_from_engine`].
pub const fn engine_route_direction_from_saved(direction: u8) -> Option<u8> {
    match direction {
        SAVE_NPC_BAND_ROUTE_DIR_EAST => Some(NPC_PATH_DIR_EAST),
        SAVE_NPC_BAND_ROUTE_DIR_NORTH => Some(NPC_PATH_DIR_NORTH),
        SAVE_NPC_BAND_ROUTE_DIR_WEST => Some(NPC_PATH_DIR_WEST),
        SAVE_NPC_BAND_ROUTE_DIR_SOUTH => Some(NPC_PATH_DIR_SOUTH),
        _ => None,
    }
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16) {
    bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
}

fn read_u16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

pub const fn band_schedule_offset(slot: usize) -> usize {
    SAVE_NPC_BAND_SCHEDULES_OFFSET + slot * SAVE_NPC_BAND_SCHEDULE_RECORD_LEN
}

pub const fn band_runtime_offset(slot: usize) -> usize {
    SAVE_NPC_BAND_RUNTIME_OFFSET + slot * SAVE_NPC_BAND_RUNTIME_RECORD_LEN
}

pub const fn band_route_offset(slot: usize) -> usize {
    SAVE_NPC_BAND_ROUTES_OFFSET + slot * SAVE_NPC_BAND_ROUTE_BUFFER_LEN
}

pub const fn band_route_cursor_offset(slot: usize) -> usize {
    SAVE_NPC_BAND_ROUTE_CURSORS_OFFSET + slot * 2
}

pub const fn band_type_offset(slot: usize) -> usize {
    SAVE_NPC_BAND_TYPES_OFFSET + slot
}

pub const fn band_stuck_counter_offset(slot: usize) -> usize {
    SAVE_NPC_BAND_STUCK_COUNTERS_OFFSET + slot * 2
}

/// `formats/saved-gam.md §12.2`: run-length encode a live engine move
/// queue into "up to sixteen two-byte runs. The first byte of a run is its
/// remaining step count; the second is a direction".
///
/// The engine holds its queue as a flat list of one-step direction codes
/// with a separate read index, so the *remaining* suffix (`directions`
/// here) is the whole of its live route state. Encoding that suffix from
/// buffer offset zero and setting the cursor to zero reproduces exactly
/// the same remaining walk as the original's "partially consumed count at
/// the cursor" form: §12.2's rule is that "An accepted queued step
/// decrements the run's count", so a count at the cursor is by
/// construction a *remaining* count, which is what the first run below
/// carries. Consumed runs before the cursor hold a count of zero and a
/// cleared direction byte in the original and carry no information.
///
/// Returns the filled 32-byte buffer and the cursor word, `0xFFFF` when
/// there is nothing queued ("`0xFFFF` means inactive").
pub fn encode_route_buffer(
    directions: &[u8],
) -> io::Result<([u8; SAVE_NPC_BAND_ROUTE_BUFFER_LEN], u16)> {
    let mut buffer = [0u8; SAVE_NPC_BAND_ROUTE_BUFFER_LEN];
    let mut run = 0usize;
    let mut index = 0usize;
    while index < directions.len() && run < SAVE_NPC_BAND_ROUTE_MAX_RUNS {
        let code = directions[index];
        let direction = saved_route_direction_from_engine(code).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("NPC move queue holds a non-cardinal direction code {code}"),
            )
        })?;
        // A single run's count is one byte wide, so a longer straight leg
        // spills into the next run.
        let mut count = 0usize;
        while index < directions.len() && directions[index] == code && count < usize::from(u8::MAX)
        {
            count += 1;
            index += 1;
        }
        buffer[run * SAVE_NPC_BAND_ROUTE_RUN_LEN] = count as u8;
        buffer[run * SAVE_NPC_BAND_ROUTE_RUN_LEN + 1] = direction;
        run += 1;
    }
    let cursor = if run == 0 {
        SAVE_NPC_BAND_ROUTE_CURSOR_INACTIVE
    } else {
        0
    };
    Ok((buffer, cursor))
}

/// `formats/saved-gam.md §12.2`, read side. Expands the runs from the
/// cursor onward into the engine's flat direction list.
///
/// "Reaching the end of the buffer or a zero next count makes the cursor
/// inactive", and "A zero count at the cursor also means there is no
/// queued step to execute", so both end the walk. A cursor that is
/// `0xFFFF`, odd, or outside `0..30` yields an empty queue.
pub fn decode_route_buffer(buffer: &[u8], cursor: u16) -> Vec<u8> {
    let mut directions = Vec::new();
    if cursor == SAVE_NPC_BAND_ROUTE_CURSOR_INACTIVE {
        return directions;
    }
    let mut offset = usize::from(cursor);
    if offset % SAVE_NPC_BAND_ROUTE_RUN_LEN != 0 {
        return directions;
    }
    while offset + SAVE_NPC_BAND_ROUTE_RUN_LEN <= buffer.len() {
        let count = buffer[offset];
        if count == 0 {
            break;
        }
        let Some(code) = engine_route_direction_from_saved(buffer[offset + 1]) else {
            break;
        };
        for _ in 0..count {
            directions.push(code);
        }
        offset += SAVE_NPC_BAND_ROUTE_RUN_LEN;
    }
    directions
}

/// Write one occupied slot's six sub-tables.
fn encode_occupied_slot(
    save: &mut [u8],
    npc: &RuntimeNpc,
    active_object_count: usize,
) -> io::Result<()> {
    let slot = npc.slot;
    let schedule = band_schedule_offset(slot);
    // §12.1: "The schedule record has exactly the source `.NPC` shape ...
    // The saved copy is the **live** schedule: pursuit and other
    // interactions can alter it during a visit. Substituting the original
    // asset record would discard those changes."
    save[schedule..schedule + SAVE_NPC_BAND_SCHEDULE_RECORD_LEN].copy_from_slice(&npc.schedule);

    let runtime = band_runtime_offset(slot);
    let to_word = |value: usize, field: &str| -> io::Result<u16> {
        u16::try_from(value).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("NPC slot {slot} {field} is outside word range: {value}"),
            )
        })
    };
    // Word 0. §12.1: "Full 16-bit state ... the high byte is not padding."
    // The engine models the state machine as a byte (`npc-schedules.md §7`
    // values `0..8`), so the high byte is written zero.
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_STATE_OFFSET,
        u16::from(npc.state),
    );
    // Words 2/4. §12.1: "NPC's current map column, including while
    // following a route" / "current map row".
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_X_OFFSET,
        to_word(npc.x, "X")?,
    );
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_Y_OFFSET,
        to_word(npc.y, "Y")?,
    );
    // Word 6. §12.1: "Initialization zero-extends the source byte: a
    // basement floor byte `0xFF` is stored as word `0x00FF`."
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_Z_OFFSET,
        u16::from(npc.z),
    );
    // Word 8. §12.1: "Word copy of the NPC type byte."
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_TYPE_MIRROR_OFFSET,
        u16::from(npc.type_byte),
    );
    // Word 10. §12.1: "Word initialized by zero-extending the source
    // dialogue byte. Preserve live changes ... There is no separate saved
    // 32-byte dialogue array."
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_DIALOGUE_OFFSET,
        u16::from(npc.dialog_id),
    );
    // Word 12. §12.1: "Index into the saved active-object table, or zero
    // when no object is linked. It is an object-table index, not the NPC's
    // roster index." §12.3: "The linked-object words refer to
    // `0x06B4..0x07B3`, whose records must agree with the saved NPC
    // positions and links." The engine's live link is exactly that index
    // and `PlayState::sync_npc_active_object` keeps the record's
    // coordinates equal to the NPC's, so writing both from the same live
    // pair keeps the join. A stale index past the end of the table is
    // written as "no object" rather than as a dangling reference.
    let linked = match npc.active_object {
        Some(index) if index < active_object_count => to_word(index, "linked object")?,
        _ => SAVE_NPC_BAND_RUNTIME_LINKED_OBJECT_NONE,
    };
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_LINKED_OBJECT_OFFSET,
        linked,
    );
    // Word 14. §12.1: "Last reached waypoint index, normally `0..2`."
    write_u16(
        save,
        runtime + SAVE_NPC_BAND_RUNTIME_CACHED_WP_OFFSET,
        to_word(npc.cached_wp, "cached waypoint")?,
    );

    // §12.1 rows 4 and 5, encoded per §12.2.
    let remaining = npc
        .move_queue
        .get(npc.move_queue_pos.min(npc.move_queue.len())..)
        .unwrap_or(&[]);
    let (buffer, cursor) = encode_route_buffer(remaining)?;
    let route = band_route_offset(slot);
    save[route..route + SAVE_NPC_BAND_ROUTE_BUFFER_LEN].copy_from_slice(&buffer);
    write_u16(save, band_route_cursor_offset(slot), cursor);

    // §12.1 row 6: "NPC types: one byte per slot, using `formats/npc.md`
    // Section 6." That byte is also the occupancy flag: "any slot whose
    // `type[n]` is zero is treated as empty and skipped by the schedule
    // processor".
    save[band_type_offset(slot)] = npc.type_byte;
    // §12.1 row 10: "Stuck counters: 32 words, one per NPC."
    write_u16(save, band_stuck_counter_offset(slot), npc.stuck_counter);
    Ok(())
}

/// Retire a slot the live cast does not hold.
///
/// `formats/npc.md §6`: "any slot whose `type[n]` is zero is treated as
/// empty and skipped by the schedule processor. The corresponding schedule
/// record may still hold non-zero bytes (residue from authoring or from
/// save-game state), but the engine does not read them." So clearing the
/// occupancy byte is enough to retire a slot, and the residue is left in
/// place rather than scrubbed.
///
/// The route pair is put in the state `formats/saved-gam.md §12.2` names
/// for a slot with nothing queued - "clears only the first route byte,
/// sets the cursor to `0xFFFF` and the stuck counter to zero" - and the
/// state word is cleared, which is what `npc-schedules.md §4` does for an
/// empty slot: "For empty slots (on-disk type byte zero), only the state
/// word is cleared".
fn retire_stale_slot(save: &mut [u8], slot: usize) {
    save[band_type_offset(slot)] = SAVE_NPC_BAND_TYPE_EMPTY;
    write_u16(
        save,
        band_runtime_offset(slot) + SAVE_NPC_BAND_RUNTIME_STATE_OFFSET,
        0,
    );
    save[band_route_offset(slot)] = 0;
    write_u16(
        save,
        band_route_cursor_offset(slot),
        SAVE_NPC_BAND_ROUTE_CURSOR_INACTIVE,
    );
    write_u16(save, band_stuck_counter_offset(slot), 0);
}

/// Write the live NPC family into the `0x07B4..0x105F` band of a save
/// image built from a template.
///
/// Written: schedules, NPC runtime records, routes, route cursors, types
/// and stuck counters - the "six NPC tables [that] occupy 2,208 bytes"
/// of §12.1.
///
/// Passed through untouched: the four opaque head bytes, the two opaque
/// bytes at `0x101A`, the engagement event kind/index pair (§12.3: "The
/// two event bytes are saved with the slab but are cleared at the next NPC
/// schedule pass"), the three dungeon-mode bytes at `0x105C..0x105E`
/// ("dungeon-mode state, not an NPC field"), and the shipwright byte at
/// `0x105F`, which the caller owns through
/// [`SAVE_PENDING_VEHICLE_CLASS_OFFSET`].
///
/// Slot zero is never touched (§12.1: "slot zero is reserved ... Preserve
/// slot zero and inactive slots when round-tripping"), and neither is any
/// slot the template already marks empty.
pub fn write_npc_band(
    save: &mut [u8],
    npcs: &[RuntimeNpc],
    active_object_count: usize,
) -> io::Result<()> {
    for npc in npcs {
        if npc.slot == SAVE_NPC_BAND_RESERVED_SLOT || npc.slot >= SAVE_NPC_BAND_SLOT_COUNT {
            // §12.1: "slot zero is reserved, and ordinary scheduling walks
            // slots `1..31`". A runtime NPC outside that range has no band
            // row to occupy.
            continue;
        }
        if npc.type_byte == SAVE_NPC_BAND_TYPE_EMPTY {
            continue;
        }
        encode_occupied_slot(save, npc, active_object_count)?;
    }
    // Any slot the template marks occupied but the live cast does not hold
    // belongs to a *different* location: the template is the previous
    // `SAVED.GAM`, and §12.3 makes the band "**per location, not per
    // floor**". Leaving a foreign type byte in place would resurrect that
    // location's cast here, because "the per-tick NPC walker skips every
    // roster slot whose type byte is zero" and would *not* skip this one.
    for slot in 1..SAVE_NPC_BAND_SLOT_COUNT {
        if npcs
            .iter()
            .any(|npc| npc.slot == slot && npc.type_byte != SAVE_NPC_BAND_TYPE_EMPTY)
        {
            continue;
        }
        if save[band_type_offset(slot)] != SAVE_NPC_BAND_TYPE_EMPTY {
            retire_stale_slot(save, slot);
        }
    }
    Ok(())
}

/// Read the band back into runtime NPCs, the inverse of
/// [`write_npc_band`].
///
/// `formats/npc.md §6` / §12.1: a slot is present exactly when its type
/// byte is non-zero, and slot zero is reserved.
pub fn read_npc_band(save: &[u8]) -> Vec<RuntimeNpc> {
    let mut npcs = Vec::new();
    for slot in 1..SAVE_NPC_BAND_SLOT_COUNT {
        let type_byte = save[band_type_offset(slot)];
        if type_byte == SAVE_NPC_BAND_TYPE_EMPTY {
            continue;
        }
        let schedule_offset = band_schedule_offset(slot);
        let mut schedule = [0u8; SAVE_NPC_BAND_SCHEDULE_RECORD_LEN];
        schedule.copy_from_slice(
            &save[schedule_offset..schedule_offset + SAVE_NPC_BAND_SCHEDULE_RECORD_LEN],
        );
        let runtime = band_runtime_offset(slot);
        let route = band_route_offset(slot);
        let cursor = read_u16(save, band_route_cursor_offset(slot));
        let linked = read_u16(save, runtime + SAVE_NPC_BAND_RUNTIME_LINKED_OBJECT_OFFSET);
        npcs.push(RuntimeNpc {
            slot,
            type_byte,
            // §12.1: the dialogue *word* is the roster's dialogue byte
            // zero-extended, so the engine's byte-wide field is its low
            // byte.
            dialog_id: read_u16(save, runtime + SAVE_NPC_BAND_RUNTIME_DIALOGUE_OFFSET) as u8,
            schedule,
            state: read_u16(save, runtime + SAVE_NPC_BAND_RUNTIME_STATE_OFFSET) as u8,
            x: usize::from(read_u16(save, runtime + SAVE_NPC_BAND_RUNTIME_X_OFFSET)),
            y: usize::from(read_u16(save, runtime + SAVE_NPC_BAND_RUNTIME_Y_OFFSET)),
            // §12.1: "a basement floor byte `0xFF` is stored as word
            // `0x00FF`", so the byte is the word's low half.
            z: read_u16(save, runtime + SAVE_NPC_BAND_RUNTIME_Z_OFFSET) as u8,
            cached_wp: usize::from(read_u16(
                save,
                runtime + SAVE_NPC_BAND_RUNTIME_CACHED_WP_OFFSET,
            )),
            move_queue: decode_route_buffer(
                &save[route..route + SAVE_NPC_BAND_ROUTE_BUFFER_LEN],
                cursor,
            ),
            move_queue_pos: 0,
            stuck_counter: read_u16(save, band_stuck_counter_offset(slot)),
            active_object: (linked != SAVE_NPC_BAND_RUNTIME_LINKED_OBJECT_NONE)
                .then_some(usize::from(linked)),
        });
    }
    npcs
}

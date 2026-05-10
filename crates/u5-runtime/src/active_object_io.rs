//! Active-object encode/decode for SAVED.OOL mirroring + write helpers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

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

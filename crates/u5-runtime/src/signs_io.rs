//! Parser for `SIGNS.DAT`: scene-keyed sign records keyed by `(scene, z, y, x)`.
//!
//! Implements `formats/signs-dat.md` §2-§4: a 66-byte little-endian directory
//! of 33 scene-block offsets, variable-size scene blocks of records, and
//! per-record `[scene, z, y, x]` headers followed by NUL-terminated payloads.
//!
//! The decoder produces a printable approximation of the on-disk payload:
//! - `0x00` ends the record;
//! - `0x0D` becomes a newline (the on-disk pause becomes a paragraph break);
//! - `0x29..=0x31` decoration-fragment bytes are emitted as a single ASCII
//!   sentinel character because the resident macro pool is not in the public
//!   spec; consumers that need the original glyph must resolve fragments at
//!   render time;
//! - `0x26` and `0x27` divider bytes both become `'-'`;
//! - other bytes are emitted as their low-seven-bit character.

use std::fs;
use std::io;
use std::path::Path;

const SCENE_DIRECTORY_SLOTS: usize = 33;
const SCENE_DIRECTORY_BYTES: usize = SCENE_DIRECTORY_SLOTS * 2;
const RECORD_HEADER_LEN: usize = 4;
const SIGNS_DAT_FILE: &str = "SIGNS.DAT";

/// `formats/signs-dat.md §2`: scene directory holds 33 little-endian
/// scene-block offsets in the leading 66 bytes of the file.
pub const SIGNS_DAT_SCENE_DIRECTORY_SLOTS: usize = 33;
pub const SIGNS_DAT_SCENE_DIRECTORY_BYTES: usize = 66;
/// `formats/signs-dat.md §3`: each sign record begins with a four-byte
/// `(scene, z, y, x)` header followed by a NUL-terminated payload.
pub const SIGNS_DAT_RECORD_HEADER_LEN: usize = 4;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignRecord {
    pub scene: u8,
    pub z: u8,
    pub y: u8,
    pub x: u8,
    pub body: String,
}

pub fn load_sign_records(game_dir: &Path) -> io::Result<Option<Vec<SignRecord>>> {
    let path = game_dir.join(SIGNS_DAT_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_sign_records(&bytes).map(Some)
}

pub fn parse_sign_records(bytes: &[u8]) -> io::Result<Vec<SignRecord>> {
    if bytes.len() < SCENE_DIRECTORY_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{SIGNS_DAT_FILE} too short: {} bytes, need at least {SCENE_DIRECTORY_BYTES} for the scene directory",
                bytes.len()
            ),
        ));
    }
    let mut records = Vec::new();
    let mut visited = Vec::new();
    for slot in 0..SCENE_DIRECTORY_SLOTS {
        let offset = u16::from_le_bytes([bytes[slot * 2], bytes[slot * 2 + 1]]) as usize;
        if offset == 0 || offset >= bytes.len() || visited.contains(&offset) {
            continue;
        }
        visited.push(offset);
        parse_scene_block(bytes, offset, &mut records);
    }
    Ok(records)
}

fn parse_scene_block(bytes: &[u8], start: usize, out: &mut Vec<SignRecord>) {
    let mut cursor = start;
    while cursor + RECORD_HEADER_LEN <= bytes.len() {
        let header = &bytes[cursor..cursor + RECORD_HEADER_LEN];
        let scene = header[0];
        // The end-of-block sentinel is conventionally a zero scene byte after
        // the last real record; bail when the header looks invalid.
        if scene == 0 {
            return;
        }
        let z = header[1];
        let y = header[2];
        let x = header[3];
        cursor += RECORD_HEADER_LEN;
        let payload_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != 0x00 {
            cursor += 1;
        }
        let payload_end = cursor;
        if cursor < bytes.len() {
            cursor += 1; // skip the NUL terminator
        }
        let body = decode_sign_payload(&bytes[payload_start..payload_end]);
        out.push(SignRecord {
            scene,
            z,
            y,
            x,
            body,
        });
    }
}

pub fn decode_sign_payload(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        let low = byte & 0x7f;
        match low {
            0x00 => break,
            0x0d => out.push('\n'),
            0x26 | 0x27 => out.push('-'),
            0x29..=0x31 => out.push('*'),
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

pub fn find_sign(records: &[SignRecord], scene: u8, z: u8, y: u8, x: u8) -> Option<&SignRecord> {
    records
        .iter()
        .find(|r| r.scene == scene && r.z == z && r.y == y && r.x == x)
}

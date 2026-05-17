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
/// `formats/signs-dat.md §2` published filename for the sign-record file.
pub const SIGNS_DAT_FILE: &str = "SIGNS.DAT";

/// `formats/signs-dat.md §2`: scene directory holds 33 little-
/// endian scene-block offsets in the leading 66 bytes of the
/// file — one slot per addressable scene byte, indexed by the
/// active scene byte. The directory covers the overworld (scene
/// byte 0) and the 32 town-family scenes (1..=SCENE_TOWN_
/// FAMILY_LAST). Anchored to `SCENE_TOWN_FAMILY_LAST as usize +
/// 1` so the directory size derives from the scene partition.
pub const SIGNS_DAT_SCENE_DIRECTORY_SLOTS: usize =
    crate::SCENE_TOWN_FAMILY_LAST as usize + 1;
pub const SIGNS_DAT_SCENE_DIRECTORY_BYTES: usize = SIGNS_DAT_SCENE_DIRECTORY_SLOTS * 2;
/// `formats/signs-dat.md §3`: each sign record begins with a four-byte
/// `(scene, z, y, x)` header followed by a NUL-terminated payload.
pub const SIGNS_DAT_RECORD_HEADER_LEN: usize = 4;

/// `formats/signs-dat.md §3` alias-bridge length. The on-disk alias
/// bridge that lets multiple coordinate headers share one printed
/// body is a separator byte, a zero byte that terminates the
/// scanner's current payload walk, and then another four-byte
/// `[scene, z, y, x]` header — six bytes total. Promote the length
/// so content tools and the bridge-aware scanner can refer to one
/// named value instead of re-deriving the sum at each call site.
pub const SIGNS_DAT_ALIAS_BRIDGE_LEN: usize = 1 + 1 + SIGNS_DAT_RECORD_HEADER_LEN;

/// `formats/signs-dat.md §4` formatter byte vocabulary for a sign
/// payload byte. The formatter classifies each byte by value range
/// and either ends the record, pauses for input, substitutes a
/// macro fragment, emits the shared separator glyph, or prints
/// the low-seven-bit character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignBodyByteKind {
    /// `0x00` — end-of-record terminator.
    EndOfRecord,
    /// `0x0D` — pause for keypress, then resume.
    PauseForKey,
    /// `0x29..=0x31` — index into the small resident macro pool
    /// for framed-sign decoration fragments.
    Macro(u8),
    /// `0x26` or `0x27` — separator glyph used as a decorative
    /// divider in shipped records.
    SeparatorGlyph,
    /// Anything else — print the low-seven-bit character value;
    /// the high bit controls a presentation-mode toggle in the
    /// text-output layer rather than the printed glyph.
    Character(u8),
}

/// `formats/signs-dat.md §4` end-of-record byte. The byte `0x00`
/// terminates the current sign record; the record scanner advances
/// to the next record after seeing it.
pub const SIGN_BODY_END_OF_RECORD: u8 = 0x00;
/// `formats/signs-dat.md §4` pause-for-key byte. The byte `0x0D`
/// pauses sign rendering until the player presses a key and then
/// resumes printing.
pub const SIGN_BODY_PAUSE_FOR_KEY: u8 = 0x0D;
/// `formats/signs-dat.md §4` first byte in the contiguous macro
/// range (`0x29..=0x31`). Macro bytes select NUL-terminated
/// decoration fragments from a small resident pool rather than
/// printing the byte directly.
pub const SIGN_BODY_MACRO_FIRST: u8 = 0x29;
/// `formats/signs-dat.md §4` last byte in the macro range.
pub const SIGN_BODY_MACRO_LAST: u8 = 0x31;
/// `formats/signs-dat.md §4` first separator-glyph byte (`0x26`).
/// Shipped records pair this with `SIGN_BODY_SEPARATOR_GLYPH_B` as
/// a decorative divider; both render the same separator glyph.
pub const SIGN_BODY_SEPARATOR_GLYPH_A: u8 = 0x26;
/// `formats/signs-dat.md §4` second separator-glyph byte (`0x27`).
/// Anchored to SIGN_BODY_SEPARATOR_GLYPH_A + 1 so the paired
/// adjacent bytes have one source of truth.
pub const SIGN_BODY_SEPARATOR_GLYPH_B: u8 = SIGN_BODY_SEPARATOR_GLYPH_A + 1;
/// `formats/signs-dat.md §4` low-seven-bit character mask. Ordinary
/// printable bytes render the low seven bits; the high bit is a
/// presentation-mode toggle owned by the surrounding Look renderer.
pub const SIGN_BODY_CHARACTER_MASK: u8 = 0x7F;

/// `formats/signs-dat.md §4`: classify a single payload byte for
/// the sign-body formatter.
pub const fn sign_body_byte_kind(byte: u8) -> SignBodyByteKind {
    match byte {
        SIGN_BODY_END_OF_RECORD => SignBodyByteKind::EndOfRecord,
        SIGN_BODY_PAUSE_FOR_KEY => SignBodyByteKind::PauseForKey,
        SIGN_BODY_MACRO_FIRST..=SIGN_BODY_MACRO_LAST => SignBodyByteKind::Macro(byte),
        SIGN_BODY_SEPARATOR_GLYPH_A | SIGN_BODY_SEPARATOR_GLYPH_B => {
            SignBodyByteKind::SeparatorGlyph
        }
        other => SignBodyByteKind::Character(other & SIGN_BODY_CHARACTER_MASK),
    }
}

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

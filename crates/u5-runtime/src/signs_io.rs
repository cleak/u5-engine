//! Parser for `SIGNS.DAT`: scene-keyed sign records keyed by `(scene, z, x, y)`.
//!
//! Implements `formats/signs-dat.md` §2-§4: a 66-byte little-endian directory
//! of 33 scene-block offsets, variable-size scene blocks of records, and
//! per-record `[scene, z, x, y]` headers followed by NUL-terminated payloads.
//!
//! The decoder produces a printable approximation of the on-disk payload
//! (`formats/signs-dat.md §4`, corrected 2026-10-25, u5-spec#300 Q13/Q21,
//! `RETRACTIONS.md` R600). The control tests compare the **raw** byte:
//! - `0x00` ends the record;
//! - raw `0x0D` becomes a [`crate::text_hold`] wait-key marker: it "switches
//!   to the Roman font and waits for a key";
//! - raw `0x29..=0x31` decoration-fragment bytes are emitted as a single ASCII
//!   sentinel character because the resident macro pool is not in the public
//!   spec; consumers that need the original glyph must resolve fragments at
//!   render time;
//! - raw `0x26` and `0x27` divider bytes both print the runic `l` bar (`'-'`
//!   in the plain-text view);
//! - every other byte prints its low seven bits, in the runic font when its
//!   high bit is clear and the Roman font when set ([`sign_payload_rendered`]).

use std::io;
use std::path::Path;

use crate::read_optional_disk_file;

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
pub const SIGNS_DAT_SCENE_DIRECTORY_SLOTS: usize = crate::SCENE_TOWN_FAMILY_LAST as usize + 1;
pub const SIGNS_DAT_SCENE_DIRECTORY_BYTES: usize = SIGNS_DAT_SCENE_DIRECTORY_SLOTS * 2;
/// `formats/signs-dat.md §3`: each sign record begins with a four-byte
/// `(scene, z, x, y)` header followed by a NUL-terminated payload.
pub const SIGNS_DAT_RECORD_HEADER_LEN: usize = 4;

/// `formats/signs-dat.md §3` alias-bridge length. The on-disk alias
/// bridge that lets multiple coordinate headers share one printed
/// body is a separator byte, a zero byte that terminates the
/// scanner's current payload walk, and then another four-byte
/// `[scene, z, x, y]` header — six bytes total. Promote the length
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
    /// Anything else — print the low-seven-bit character value, in the
    /// font the byte's high bit selects (clear: runic, set: Roman;
    /// `formats/signs-dat.md §4`, R600).
    Character(u8),
}

/// `formats/signs-dat.md §4` end-of-record byte. The byte `0x00`
/// terminates the current sign record; the record scanner advances
/// to the next record after seeing it.
pub const SIGN_BODY_END_OF_RECORD: u8 = 0x00;
/// `formats/signs-dat.md §4` pause-for-key byte. Only the raw byte `0x0D`
/// pauses sign rendering until the player presses a key; `0x8D` is an
/// ordinary byte that reaches the cell emitter as a carriage return (R600).
pub const SIGN_BODY_PAUSE_FOR_KEY: u8 = 0x0D;
/// `formats/signs-dat.md §4`/§6: an explicit line feed in the authored
/// stream (low seven bits `0x0A`); also the alias-bridge separator (§3).
pub const SIGN_BODY_LINE_FEED: u8 = 0x0A;
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
/// `formats/signs-dat.md §4`: a separator "prints one cell in the runic font
/// at the code point of lowercase `l`, which in that font is a full-width
/// horizontal bar" (u5-spec#300 Q13).
pub const SIGN_SEPARATOR_RUNIC_CODE_POINT: u8 = b'l';
/// `formats/signs-dat.md §4` high bit: set selects the Roman text font, clear
/// selects the runic font (u5-spec#300 Q13, R600).
pub const SIGN_BODY_ROMAN_FONT_BIT: u8 = 0x80;
/// `formats/signs-dat.md §6` "The no-record placard" (added 2026-10-25,
/// u5-spec#300 Q13): "a fixed six-row framed placard from resident text, in
/// the runic font" whose inner rows read "LIVE BY THE / EIGHT LAWS". Each row
/// is sixteen cells "and ends with a line feed carried in the same stored
/// string".
pub const SIGN_NO_RECORD_PLACARD_ROWS: [&str; 6] = [
    "abbbbbbbbbbbbbbc",
    "g              g",
    "g  LIVE@BY@[E  g",
    "g  EIGHT@LAWS  g",
    "g              g",
    "deeeeeeeeeeeeeef",
];
/// `formats/signs-dat.md §4` second separator-glyph byte (`0x27`).
/// Anchored to SIGN_BODY_SEPARATOR_GLYPH_A + 1 so the paired
/// adjacent bytes have one source of truth.
pub const SIGN_BODY_SEPARATOR_GLYPH_B: u8 = SIGN_BODY_SEPARATOR_GLYPH_A + 1;
/// `formats/signs-dat.md §4` low-seven-bit character mask. Ordinary
/// printable bytes render the low seven bits; the high bit selects the font
/// for that byte (R600), and is removed only when printing an ordinary byte.
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
    /// The raw payload bytes behind `body` (alias bridges resolved), kept so
    /// the per-byte font of `formats/signs-dat.md §4` survives decoding.
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RawSignRecord {
    scene: u8,
    z: u8,
    y: u8,
    x: u8,
    payload_start: usize,
    payload_end: usize,
}

pub fn load_sign_records(game_dir: &Path) -> io::Result<Option<Vec<SignRecord>>> {
    let path = game_dir.join(SIGNS_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
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
        if offset == 0 {
            continue;
        }
        if offset < SCENE_DIRECTORY_BYTES || offset >= bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{SIGNS_DAT_FILE} directory slot {slot} has invalid offset {offset}"),
            ));
        }
        if visited.contains(&offset) {
            continue;
        }
        visited.push(offset);
        // `formats/signs-dat.md §5`: "The active scene indexes the
        // `SIGNS.DAT` scene directory" and "[t]he scene block is scanned".
        // Shipped blocks sit back to back with no sentinel between them, so
        // a block ends where the next scene's block begins. Scanning on past
        // it collected each scene-k record once per non-empty slot <= k, and
        // the Look path printed one sign body that many times (playtest J04).
        let end = scene_block_end(bytes, offset);
        parse_scene_block(bytes, slot as u8, offset, end, &mut records)?;
    }
    Ok(records)
}

/// The first nonzero directory offset after `start`, or the end of the file:
/// where the scene block that begins at `start` stops.
fn scene_block_end(bytes: &[u8], start: usize) -> usize {
    (0..SCENE_DIRECTORY_SLOTS)
        .map(|slot| u16::from_le_bytes([bytes[slot * 2], bytes[slot * 2 + 1]]) as usize)
        .filter(|&offset| offset > start)
        .min()
        .unwrap_or(bytes.len())
        .min(bytes.len())
}

fn parse_scene_block(
    bytes: &[u8],
    directory_scene: u8,
    start: usize,
    end: usize,
    out: &mut Vec<SignRecord>,
) -> io::Result<()> {
    let mut cursor = start;
    let mut raw_records = Vec::new();
    while cursor < bytes.len() {
        if cursor >= end || bytes[cursor] == 0 {
            for index in 0..raw_records.len() {
                let raw = &raw_records[index];
                let payload = resolve_sign_body(bytes, &raw_records, index, directory_scene)?;
                out.push(SignRecord {
                    scene: raw.scene,
                    z: raw.z,
                    y: raw.y,
                    x: raw.x,
                    body: decode_sign_payload(&payload),
                    payload,
                });
            }
            return Ok(());
        }
        if cursor + RECORD_HEADER_LEN > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SIGNS_DAT_FILE} scene {directory_scene} block has truncated record header at byte {cursor}"
                ),
            ));
        }
        let header = &bytes[cursor..cursor + RECORD_HEADER_LEN];
        let scene = header[0];
        let z = header[1];
        let x = header[2];
        let y = header[3];
        cursor += RECORD_HEADER_LEN;
        let payload_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != 0x00 {
            cursor += 1;
        }
        let payload_end = cursor;
        if cursor >= bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SIGNS_DAT_FILE} scene {directory_scene} record at byte {} is not NUL-terminated",
                    payload_start - RECORD_HEADER_LEN
                ),
            ));
        }
        cursor += 1; // skip the NUL terminator
        raw_records.push(RawSignRecord {
            scene,
            z,
            y,
            x,
            payload_start,
            payload_end,
        });
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("{SIGNS_DAT_FILE} scene {directory_scene} block has no end sentinel"),
    ))
}

fn resolve_sign_body(
    bytes: &[u8],
    raw_records: &[RawSignRecord],
    index: usize,
    directory_scene: u8,
) -> io::Result<Vec<u8>> {
    let mut resolved_index = index;
    while is_alias_bridge_payload(record_payload(bytes, &raw_records[resolved_index])) {
        let next_index = resolved_index + 1;
        if next_index >= raw_records.len() {
            let raw = &raw_records[index];
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{SIGNS_DAT_FILE} scene {directory_scene} alias bridge at ({}, {}, {}) has no following shared-body record",
                    raw.z, raw.y, raw.x
                ),
            ));
        }
        resolved_index = next_index;
    }
    Ok(record_payload(bytes, &raw_records[resolved_index]).to_vec())
}

fn record_payload<'a>(bytes: &'a [u8], record: &RawSignRecord) -> &'a [u8] {
    &bytes[record.payload_start..record.payload_end]
}

/// `formats/signs-dat.md §3`: the formatter skips a six-byte alias bridge
/// "while it sees the line-separator marker used by the sign stream". Every
/// shipped bridge is a lone line feed (`0x0A`); the paired divider bytes are
/// still accepted for authored fixtures.
fn is_alias_bridge_payload(payload: &[u8]) -> bool {
    payload.len() == 1
        && (payload[0] & SIGN_BODY_CHARACTER_MASK == SIGN_BODY_LINE_FEED
            || matches!(
                payload[0],
                SIGN_BODY_SEPARATOR_GLYPH_A | SIGN_BODY_SEPARATOR_GLYPH_B
            ))
}

pub fn decode_sign_payload(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        // `formats/signs-dat.md §4` (u5-spec#300 Q21, R600): "the control
        // tests then compare the raw byte, high bit included". Masking first
        // turned high-bit `,` `.` `/` `0` `1` (`0xAC..0xB1`) into macro stars
        // and a high-bit `'` into a separator (playtest K01). The plain-text
        // view draws the runic bar as `-`; [`sign_payload_rendered`] carries
        // the fonts.
        match byte {
            SIGN_BODY_END_OF_RECORD => break,
            SIGN_BODY_PAUSE_FOR_KEY => {
                out.push(char::from(crate::text_hold::TEXT_HOLD_WAIT_KEY_MARKER))
            }
            SIGN_BODY_SEPARATOR_GLYPH_A | SIGN_BODY_SEPARATOR_GLYPH_B => out.push('-'),
            SIGN_BODY_MACRO_FIRST..=SIGN_BODY_MACRO_LAST => out.push('*'),
            // Every other byte prints its low seven bits; `0x8D` is a
            // carriage return, not a pause (see [`sign_payload_rendered`]).
            _ => match byte & SIGN_BODY_CHARACTER_MASK {
                SIGN_BODY_LINE_FEED => out.push('\n'),
                ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
                _ => {}
            },
        }
    }
    out
}

/// `formats/signs-dat.md §4` (corrected 2026-10-25, u5-spec#300 Q13 and Q21,
/// `RETRACTIONS.md` R600): decode a payload into display cells, each carrying
/// its font. "The high bit selects the font for that byte" (clear: runic,
/// set: Roman), "the control tests then compare the raw byte, high bit
/// included", and the high bit "is removed only when printing an ordinary
/// byte". So `0xA6`/`0xA7` print the Roman `&`/`'`, `0xA9..0xB1` the Roman
/// characters at `0x29..0x31`, and `0x8D` "reaches the cell emitter as a
/// carriage return (column reset, no glyph), not a pause"; no shipped record
/// carries one, and this string model has no overwrite, so it emits nothing.
pub fn sign_payload_rendered(bytes: &[u8]) -> crate::tlk_runner::TlkRenderedText {
    use crate::tlk_runner::{TlkGlyphFont, TlkRenderedGlyph, TlkRenderedText};
    let mut out = TlkRenderedText::default();
    let mut push = |byte: u8, font: TlkGlyphFont| {
        out.text.push(char::from(byte));
        out.glyphs.push(TlkRenderedGlyph { byte, font });
    };
    for &byte in bytes {
        match byte {
            SIGN_BODY_END_OF_RECORD => break,
            SIGN_BODY_PAUSE_FOR_KEY => push(
                crate::text_hold::TEXT_HOLD_WAIT_KEY_MARKER,
                TlkGlyphFont::Ordinary,
            ),
            SIGN_BODY_SEPARATOR_GLYPH_A | SIGN_BODY_SEPARATOR_GLYPH_B => {
                push(SIGN_SEPARATOR_RUNIC_CODE_POINT, TlkGlyphFont::Runic)
            }
            SIGN_BODY_MACRO_FIRST..=SIGN_BODY_MACRO_LAST => push(b'*', TlkGlyphFont::Ordinary),
            _ => {
                let font = if byte & SIGN_BODY_ROMAN_FONT_BIT != 0 {
                    TlkGlyphFont::Ordinary
                } else {
                    TlkGlyphFont::Runic
                };
                match byte & SIGN_BODY_CHARACTER_MASK {
                    // `text-output.md §5`: a line feed is a row break with a
                    // column reset, drawn in neither font.
                    SIGN_BODY_LINE_FEED => push(b'\n', TlkGlyphFont::Ordinary),
                    ch if (0x20..=0x7e).contains(&ch) => push(ch, font),
                    _ => {}
                }
            }
        }
    }
    out
}

/// `formats/signs-dat.md §2` and §5 step 4 (issue #302 Q3, `RETRACTIONS.md`
/// R623): "The window is 2000 bytes from the absolute offset in the scene's
/// directory entry."
pub const SIGNS_DAT_READ_WINDOW_BYTES: usize = 2000;
/// `formats/signs-dat.md §2`: the window is "first fill[ed] ... with the
/// all-ones byte (255)", which survives only past a short read; §3's only end
/// test is this byte "where the next header would start".
pub const SIGNS_DAT_WINDOW_FILL_BYTE: u8 = 0xFF;

/// The Look path's sign lookup over the raw file, `formats/signs-dat.md §5`
/// (issue #302 Q3, R623).
///
/// - A scene outside the directory, or a zero directory entry, finds nothing
///   (the caller prints the §6 placard; §5 step 3, §7).
/// - The 2000-byte window starts at the scene's directory offset and is
///   scanned record by record across later scenes' blocks; "only the record's
///   `z`, `x` and `y` bytes are compared" and the first match wins (step 6).
/// - A record "can match only if its whole four-byte header lies inside the
///   window". The scan ends at the window's end, at a 255 byte where a header
///   would start, or at a record with no terminator - step 7's clean contract,
///   a deliberate divergence from the original's unbounded scan.
/// - The zero-padding pseudo-record (an all-zero header with an empty body) is
///   ignored, the second deliberate divergence.
/// - A matched record whose text runs past the window is printed in full from
///   the file, as the clean contract allows; an alias bridge resolves to the
///   following record's shared body (§3).
pub fn find_sign_in_window(bytes: &[u8], scene: u8, z: u8, y: u8, x: u8) -> Option<SignRecord> {
    let slot = usize::from(scene);
    if slot >= SCENE_DIRECTORY_SLOTS || bytes.len() < SCENE_DIRECTORY_BYTES {
        return None;
    }
    let offset = usize::from(u16::from_le_bytes([bytes[slot * 2], bytes[slot * 2 + 1]]));
    if offset == 0 || offset >= bytes.len() {
        return None;
    }
    // The window holds file bytes up to `data_end`; past that only the 255
    // prefill a short read leaves behind, which the end test stops at.
    let window_end = offset + SIGNS_DAT_READ_WINDOW_BYTES;
    let data_end = window_end.min(bytes.len());
    let mut cursor = offset;
    loop {
        if cursor + RECORD_HEADER_LEN > window_end {
            return None;
        }
        if cursor >= data_end || bytes[cursor] == SIGNS_DAT_WINDOW_FILL_BYTE {
            return None;
        }
        if cursor + RECORD_HEADER_LEN > data_end {
            // The rest of the header is the short read's 255 prefill.
            return None;
        }
        let header = &bytes[cursor..cursor + RECORD_HEADER_LEN];
        let payload_start = cursor + RECORD_HEADER_LEN;
        let payload_end = payload_start + bytes[payload_start..].iter().position(|&b| b == 0)?;
        let pseudo_record = header.iter().all(|&b| b == 0) && payload_end == payload_start;
        if !pseudo_record && header[1] == z && header[2] == x && header[3] == y {
            let mut body_start = payload_start;
            let mut body_end = payload_end;
            // §3 alias bridge: the bridge's terminator is followed by another
            // header, and the formatter prints the body after it.
            while is_alias_bridge_payload(&bytes[body_start..body_end]) {
                let next_start = body_end + 1 + RECORD_HEADER_LEN;
                if next_start > bytes.len() {
                    return None;
                }
                body_start = next_start;
                body_end = body_start + bytes[body_start..].iter().position(|&b| b == 0)?;
            }
            let payload = bytes[body_start..body_end].to_vec();
            return Some(SignRecord {
                scene: header[0],
                z: header[1],
                y: header[3],
                x: header[2],
                body: decode_sign_payload(&payload),
                payload,
            });
        }
        cursor = payload_end + 1;
    }
}

/// `formats/signs-dat.md §5` step 6 (corrected 2026-10-25, u5-spec#300 Q13,
/// `RETRACTIONS.md` R598): "The first matching record is printed through the
/// sign formatter and the scan stops; a later record with the same coordinate
/// is never shown."
pub fn find_sign(records: &[SignRecord], scene: u8, z: u8, y: u8, x: u8) -> Option<&SignRecord> {
    records
        .iter()
        .find(|r| r.scene == scene && r.z == z && r.y == y && r.x == x)
}

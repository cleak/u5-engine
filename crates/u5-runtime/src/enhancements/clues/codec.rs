//! The `QJRN` payload, version 1 (Phase B design 8.3.7).
//!
//! The foundation container (magic, tag, stamp, length, checksum) wraps
//! it; this module is the payload alone. Version 1 is a list of sections,
//! each `id u8 | len u32 | body`, little-endian. Ids this build does not
//! know, and section 4 (PINS, written by the full map in L1), are skipped
//! by length and written back unchanged.
//!
//! ```text
//! 1 CLUES   count u16; entries of 24 bytes
//! 2 LINKS   count u16; entry u16 | kind u8 | target u16
//! 3 NOTES   count u16; owner_kind u8 | owner u16 | len u16 | UTF-8
//! 4 PINS    carried through
//! 5 MISSES  count u16; scene u8 | slot u8 | at u32 | len u8 | bytes
//! 6 KNOWN   count u16; step u16
//! 7 TEXT    count u16; entry u16 | rows u8 | (len u8 | bytes | runic u16)*
//! ```
//!
//! Only sections 3, 5 and 7 hold variable-length data, all of it the
//! player's own: notes they typed, words they typed, rows they saw. TEXT
//! is written only with `clue_text = kept`, MISSES only with
//! `clue_misses` on ([`EncodeOptions`]).

use super::{
    ClueEntry, ClueLog, DialogKey, EntryFlags, GameMinute, Link, MAX_ENTRIES, MISS_MAX_LEN, Miss,
    Note, NpcKey, RecordAddr, SeenRows, TestBits, TextRow,
};

/// The payload version this build writes. A newer one is read-only.
pub const JOURNAL_PAYLOAD_VERSION: u16 = 1;

/// The payload cap (8.3.7).
pub const MAX_PAYLOAD_LEN: usize = 1 << 20;

/// One CLUES entry on disk (test T8 pins it).
pub const CLUE_ENTRY_LEN: usize = 24;

/// The most bytes a TEXT row holds: the message window's width.
pub const TEXT_ROW_MAX_LEN: usize = 16;

const SECTION_CLUES: u8 = 1;
const SECTION_LINKS: u8 = 2;
const SECTION_NOTES: u8 = 3;
const SECTION_PINS: u8 = 4;
const SECTION_MISSES: u8 = 5;
const SECTION_KNOWN: u8 = 6;
const SECTION_TEXT: u8 = 7;

const KIND_LEADING: u8 = 0;
const KIND_PAIR: u8 = 1;
const KIND_SCOPED: u8 = 2;
const KIND_MISS: u8 = 3;

/// Why a payload did not decode or encode. Any of these on read makes the
/// file `Corrupt`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodecError {
    /// A section or a field runs past the end.
    Truncated,
    /// A section's body has bytes its count does not account for.
    TrailingBytes(u8),
    /// A known section appears twice.
    DuplicateSection(u8),
    /// A field holds a value the format does not allow.
    BadValue(u8),
    /// More than [`MAX_ENTRIES`] entries.
    TooManyEntries,
    /// Over [`MAX_PAYLOAD_LEN`], or a count that does not fit its field.
    TooLarge,
    /// A link, note or text row names an entry that does not exist.
    BadReference(u8),
}

/// What the encoder writes besides the always-written sections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EncodeOptions {
    /// `clue_text = kept` (D8.1): write TEXT.
    pub keep_text: bool,
    /// `clue_misses` on: write MISSES.
    pub keep_misses: bool,
}

/// Encodes `log` as payload version 1.
pub fn encode_payload(log: &ClueLog, options: EncodeOptions) -> Result<Vec<u8>, CodecError> {
    if log.entries.len() > MAX_ENTRIES {
        return Err(CodecError::TooManyEntries);
    }
    let mut out = Vec::new();

    let mut body = count(log.entries.len())?;
    for entry in &log.entries {
        let mut entry = *entry;
        if !options.keep_text {
            // No TEXT section, so no entry claims kept text.
            entry.flags.set(EntryFlags::HAS_TEXT, false);
        }
        body.extend_from_slice(&entry_bytes(&entry));
    }
    section(&mut out, SECTION_CLUES, &body)?;

    if !log.links.is_empty() {
        let mut body = count(log.links.len())?;
        for link in &log.links {
            body.extend_from_slice(&link.entry.to_le_bytes());
            body.push(link.kind);
            body.extend_from_slice(&link.target.to_le_bytes());
        }
        section(&mut out, SECTION_LINKS, &body)?;
    }

    if !log.notes.is_empty() {
        let mut body = count(log.notes.len())?;
        for note in &log.notes {
            body.push(note.owner_kind);
            body.extend_from_slice(&note.owner.to_le_bytes());
            let len = u16::try_from(note.text.len()).map_err(|_| CodecError::TooLarge)?;
            body.extend_from_slice(&len.to_le_bytes());
            body.extend_from_slice(note.text.as_bytes());
        }
        section(&mut out, SECTION_NOTES, &body)?;
    }

    for (id, body) in log.passthrough.iter().filter(|(id, _)| *id == SECTION_PINS) {
        section(&mut out, *id, body)?;
    }

    if options.keep_misses && !log.misses.is_empty() {
        let mut body = count(log.misses.len())?;
        for miss in &log.misses {
            let typed = &miss.typed[..miss.typed.len().min(MISS_MAX_LEN)];
            body.extend_from_slice(&[miss.npc.scene, miss.npc.slot]);
            body.extend_from_slice(&miss.at.0.to_le_bytes());
            body.push(typed.len() as u8);
            body.extend_from_slice(typed);
        }
        section(&mut out, SECTION_MISSES, &body)?;
    }

    if !log.known.is_empty() {
        let mut body = count(log.known.len())?;
        for step in &log.known {
            body.extend_from_slice(&step.to_le_bytes());
        }
        section(&mut out, SECTION_KNOWN, &body)?;
    }

    if options.keep_text && !log.text.is_empty() {
        let mut body = count(log.text.len())?;
        for seen in &log.text {
            body.extend_from_slice(&seen.entry.to_le_bytes());
            body.push(u8::try_from(seen.rows.len()).map_err(|_| CodecError::TooLarge)?);
            for row in &seen.rows {
                let bytes = &row.bytes[..row.bytes.len().min(TEXT_ROW_MAX_LEN)];
                body.push(bytes.len() as u8);
                body.extend_from_slice(bytes);
                body.extend_from_slice(&row.runic.to_le_bytes());
            }
        }
        section(&mut out, SECTION_TEXT, &body)?;
    }

    for (id, body) in log.passthrough.iter().filter(|(id, _)| *id != SECTION_PINS) {
        section(&mut out, *id, body)?;
    }
    Ok(out)
}

/// Decodes a version 1 payload. A payload from a newer version decodes as
/// far as its sections are known; the caller treats it as read-only.
pub fn decode_payload(bytes: &[u8]) -> Result<ClueLog, CodecError> {
    if bytes.len() > MAX_PAYLOAD_LEN {
        return Err(CodecError::TooLarge);
    }
    let mut log = ClueLog::default();
    let mut seen = [false; 8];
    let mut at = Reader::new(bytes);
    while !at.is_empty() {
        let id = at.u8()?;
        let len = at.u32()? as usize;
        let body = at.take(len)?;
        if let Some(slot) = seen
            .get_mut(usize::from(id))
            .filter(|_| id != 0 && id != SECTION_PINS)
        {
            if *slot {
                return Err(CodecError::DuplicateSection(id));
            }
            *slot = true;
        }
        let mut r = Reader::new(body);
        match id {
            SECTION_CLUES => {
                let n = usize::from(r.u16()?);
                if n > MAX_ENTRIES {
                    return Err(CodecError::TooManyEntries);
                }
                for _ in 0..n {
                    log.entries.push(read_entry(&mut r)?);
                }
            }
            SECTION_LINKS => {
                for _ in 0..r.u16()? {
                    log.links.push(Link {
                        entry: r.u16()?,
                        kind: r.u8()?,
                        target: r.u16()?,
                    });
                }
            }
            SECTION_NOTES => {
                for _ in 0..r.u16()? {
                    let owner_kind = r.u8()?;
                    let owner = r.u16()?;
                    let len = usize::from(r.u16()?);
                    let text = std::str::from_utf8(r.take(len)?)
                        .map_err(|_| CodecError::BadValue(id))?
                        .to_string();
                    log.notes.push(Note {
                        owner_kind,
                        owner,
                        text,
                    });
                }
            }
            SECTION_MISSES => {
                for _ in 0..r.u16()? {
                    let npc = NpcKey {
                        scene: r.u8()?,
                        slot: r.u8()?,
                    };
                    let at = GameMinute(r.u32()?);
                    let len = usize::from(r.u8()?);
                    if len > MISS_MAX_LEN {
                        return Err(CodecError::BadValue(id));
                    }
                    let typed = r.take(len)?.to_vec();
                    log.misses.push(Miss { npc, at, typed });
                }
            }
            SECTION_KNOWN => {
                for _ in 0..r.u16()? {
                    log.known.push(r.u16()?);
                }
            }
            SECTION_TEXT => {
                for _ in 0..r.u16()? {
                    let entry = r.u16()?;
                    let mut rows = Vec::new();
                    for _ in 0..r.u8()? {
                        let len = usize::from(r.u8()?);
                        if len > TEXT_ROW_MAX_LEN {
                            return Err(CodecError::BadValue(id));
                        }
                        let bytes = r.take(len)?.to_vec();
                        rows.push(TextRow {
                            bytes,
                            runic: r.u16()?,
                        });
                    }
                    log.text.push(SeenRows { entry, rows });
                }
            }
            _ => {
                log.passthrough.push((id, body.to_vec()));
                continue;
            }
        }
        if !r.is_empty() {
            return Err(CodecError::TrailingBytes(id));
        }
    }
    let entries = log.entries.len();
    let in_range = |entry: u16| usize::from(entry) < entries;
    if !log.links.iter().all(|link| in_range(link.entry)) {
        return Err(CodecError::BadReference(SECTION_LINKS));
    }
    if !log
        .notes
        .iter()
        .filter(|note| note.owner_kind == super::note_owner::CLUE)
        .all(|note| in_range(note.owner))
    {
        return Err(CodecError::BadReference(SECTION_NOTES));
    }
    if !log.text.iter().all(|seen| in_range(seen.entry)) {
        return Err(CodecError::BadReference(SECTION_TEXT));
    }
    Ok(log)
}

fn count(n: usize) -> Result<Vec<u8>, CodecError> {
    let n = u16::try_from(n).map_err(|_| CodecError::TooLarge)?;
    Ok(n.to_le_bytes().to_vec())
}

fn section(out: &mut Vec<u8>, id: u8, body: &[u8]) -> Result<(), CodecError> {
    let len = u32::try_from(body.len()).map_err(|_| CodecError::TooLarge)?;
    if out.len() + 5 + body.len() > MAX_PAYLOAD_LEN {
        return Err(CodecError::TooLarge);
    }
    out.push(id);
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(body);
    Ok(())
}

fn entry_bytes(entry: &ClueEntry) -> [u8; CLUE_ENTRY_LEN] {
    let (kind, a, b) = match entry.record {
        RecordAddr::Leading(n) => (KIND_LEADING, n, 0),
        RecordAddr::Pair(k) => (KIND_PAIR, k, 0),
        RecordAddr::Scoped { decl, arm } => (KIND_SCOPED, decl, arm),
        RecordAddr::Miss => (KIND_MISS, 0, 0),
    };
    let mut out = [0; CLUE_ENTRY_LEN];
    out[..8].copy_from_slice(&[
        entry.npc.scene,
        entry.npc.slot,
        entry.dialog.family,
        entry.dialog.dialog_id,
        kind,
        a,
        b,
        entry.flags.0,
    ]);
    out[8..10].copy_from_slice(&entry.variant.to_le_bytes());
    out[10..12].copy_from_slice(&entry.count.to_le_bytes());
    out[12..16].copy_from_slice(&entry.first.0.to_le_bytes());
    out[16..20].copy_from_slice(&entry.last.0.to_le_bytes());
    out[20..23].copy_from_slice(&[entry.tests.n, entry.tests.pre, entry.tests.post]);
    out
}

fn read_entry(r: &mut Reader<'_>) -> Result<ClueEntry, CodecError> {
    let b = r.take(CLUE_ENTRY_LEN)?;
    let u16_at = |i: usize| u16::from_le_bytes([b[i], b[i + 1]]);
    let u32_at = |i: usize| u32::from_le_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let record = match (b[4], b[5], b[6]) {
        (KIND_LEADING, n @ 0..=4, 0) => RecordAddr::Leading(n),
        (KIND_PAIR, k, 0) => RecordAddr::Pair(k),
        (KIND_SCOPED, decl, arm) => RecordAddr::Scoped { decl, arm },
        (KIND_MISS, 0, 0) => RecordAddr::Miss,
        _ => return Err(CodecError::BadValue(SECTION_CLUES)),
    };
    Ok(ClueEntry {
        npc: NpcKey {
            scene: b[0],
            slot: b[1],
        },
        dialog: DialogKey {
            family: b[2],
            dialog_id: b[3],
        },
        record,
        flags: EntryFlags(b[7]),
        variant: u16_at(8),
        count: u16_at(10),
        first: GameMinute(u32_at(12)),
        last: GameMinute(u32_at(16)),
        tests: TestBits {
            n: b[20],
            pre: b[21],
            post: b[22],
        },
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], CodecError> {
        if self.bytes.len() < n {
            return Err(CodecError::Truncated);
        }
        let (head, tail) = self.bytes.split_at(n);
        self.bytes = tail;
        Ok(head)
    }

    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, CodecError> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, CodecError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
}

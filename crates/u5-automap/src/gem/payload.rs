//! The `GEMK` payload and its attach (Phase B design 10.3.7; build plan
//! M3-07).
//!
//! The payload is what the foundation's `SidecarStore` wraps in its
//! container as `gemmap.u5e`. There is no file I/O here except through
//! the store, which owns the container, the stamp, the atomic write and
//! the `.bad` rename. Integers are little-endian:
//!
//! ```text
//! offset size field
//! 0      2    section count N
//! 2      ..   N sections: tag u8, key 3 bytes, len u32, payload
//! ```
//!
//! | Tag | Key | Payload |
//! |---|---|---|
//! | 1 entered | 0, 0, 0 | n scene bytes |
//! | 2 flags | 0, 0, 0 | shrines, flags, named entrances (3 B) |
//! | 3 gates | 0, 0, 0 | n x (plane, x, y) |
//! | 4 signs | 0, 0, 0 | n x (scene, floor, x, y) |
//! | 5 trades | 0, 0, 0 | n x (scene, trade) |
//! | 6 keepers | 0, 0, 0 | n x (scene, floor, x, y, trade) |
//! | 7 town links | 0, 0, 0 | n x (scene, floor, x, y, dir) |
//! | 8 exits | 0, 0, 0 | n x (scene, side, run start) |
//! | 9 dungeon cells | record, level, 0 | 8 row bytes, as `AMAP` tag 3 |
//! | 10 dungeon marks | record, level, 0 | n x (x, y, kind) |
//! | 11 town-side cells | scene, 0, 0 | 48 x 36 bits (216 B) |
//!
//! Rules (10.3.7):
//! - empty sections are not written, so an empty record is two bytes;
//! - unknown tags are skipped by their length, and an unknown code inside
//!   a known tag (a later trade, side, direction or mark kind) skips that
//!   entry, so a later version stays readable;
//! - a newer payload version is read-only for this build: the record is
//!   used in RAM and the file is never overwritten
//!   ([`AttachOutcome::ReadOnly`]; the store also refuses the write);
//! - the payload holds only scene bytes, coordinates and codes, never
//!   game text: names come from the gazetteer at draw time.
//!
//! Decoding rebuilds the record through [`GemKnowledge::apply`], so a
//! decoded record keeps every invariant the observer's events keep.

use std::fmt;

use u5_runtime::enhancements::{Freshness, SaveStamp, SidecarPayload, SidecarStore};

use super::knowledge::{
    DungeonMark, EdgeSide, ExitRun, FLAG_CODEX, FLAG_UNDERWORLD, GEM_PAYLOAD_VERSION,
    GEM_SIDECAR_TAG, GateCell, GemKnowledge, KnowEvent, LinkDir, MarkKind, SIDE_WINDOW_BYTES,
    SignCell, TownLink, Trade,
};
use crate::memory::{DUNGEON_SIDE_CELLS, DungeonBits};
use crate::section::{SectionError, SectionList, read_sections};

/// Section tags of payload version 1 (10.3.7). Never renumber.
pub mod tag {
    pub const ENTERED: u8 = 1;
    pub const FLAGS: u8 = 2;
    pub const GATES: u8 = 3;
    pub const SIGNS: u8 = 4;
    pub const TRADES: u8 = 5;
    pub const KEEPERS: u8 = 6;
    pub const TOWN_LINKS: u8 = 7;
    pub const EXITS: u8 = 8;
    pub const DUNGEON_CELLS: u8 = 9;
    pub const DUNGEON_MARKS: u8 = 10;
    pub const SIDE_CELLS: u8 = 11;
}

/// A payload this build cannot read. The attach treats it as an empty
/// record; the next save replaces the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadError {
    /// Fewer than two bytes: no section count.
    MissingCount,
    /// A section header or payload runs past the end.
    Section(SectionError),
    /// A known tag whose length is not a whole number of its entries.
    BadLength { tag: u8, len: usize },
    /// Bytes after the last section, in a payload of this build's
    /// version (a newer version may append them).
    TrailingBytes { at: usize },
    /// Payload version 0, which no build writes.
    BadVersion,
}

impl fmt::Display for PayloadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingCount => write!(f, "gem map payload has no section count"),
            Self::Section(err) => write!(f, "gem map payload: {err}"),
            Self::BadLength { tag, len } => {
                write!(f, "gem map section {tag} has a bad length {len}")
            }
            Self::TrailingBytes { at } => write!(f, "gem map payload has extra bytes at {at}"),
            Self::BadVersion => write!(f, "gem map payload version 0"),
        }
    }
}

impl std::error::Error for PayloadError {}

impl From<SectionError> for PayloadError {
    fn from(err: SectionError) -> Self {
        Self::Section(err)
    }
}

/// Encodes `knowledge` as a version-[`GEM_PAYLOAD_VERSION`] payload.
/// Deterministic: sections in tag order, entries in record order.
pub fn encode_payload(knowledge: &GemKnowledge) -> Vec<u8> {
    let mut list = SectionList::new();
    let mut push = |tag: u8, key: [u8; 3], bytes: &[u8]| {
        if !bytes.is_empty() {
            // A finished game is a few kilobytes and at most 64 + 64 + 256
            // + 9 sections: neither limit of the codec can be reached.
            list.push(tag, key, bytes)
                .expect("gem payload sections stay within the codec's limits");
        }
    };
    const ZERO: [u8; 3] = [0; 3];

    let entered: Vec<u8> = knowledge.entered().iter().copied().collect();
    push(tag::ENTERED, ZERO, &entered);

    let flags = knowledge.flag_bytes();
    if flags != [0; 3] {
        push(tag::FLAGS, ZERO, &flags);
    }

    let gates: Vec<u8> = knowledge
        .gates()
        .iter()
        .flat_map(|g| [g.plane, g.x, g.y])
        .collect();
    push(tag::GATES, ZERO, &gates);

    let signs: Vec<u8> = knowledge
        .signs()
        .iter()
        .flat_map(|s| [s.scene, s.floor as u8, s.x, s.y])
        .collect();
    push(tag::SIGNS, ZERO, &signs);

    let trades: Vec<u8> = knowledge
        .trades()
        .iter()
        .flat_map(|(scene, trade)| [*scene, trade.code()])
        .collect();
    push(tag::TRADES, ZERO, &trades);

    let keepers: Vec<u8> = knowledge
        .keepers()
        .iter()
        .flat_map(|k| [k.scene, k.floor as u8, k.x, k.y, k.trade.code()])
        .collect();
    push(tag::KEEPERS, ZERO, &keepers);

    let links: Vec<u8> = knowledge
        .town_links()
        .iter()
        .flat_map(|l| [l.scene, l.floor as u8, l.x, l.y, l.dir as u8])
        .collect();
    push(tag::TOWN_LINKS, ZERO, &links);

    let exits: Vec<u8> = knowledge
        .exits()
        .iter()
        .flat_map(|e| [e.scene, e.side as u8, e.at])
        .collect();
    push(tag::EXITS, ZERO, &exits);

    let cells: &DungeonBits = knowledge.cells();
    for (record, level) in cells.nonempty_levels() {
        push(
            tag::DUNGEON_CELLS,
            [record, level, 0],
            &cells.level_rows(record, level),
        );
    }

    // The marks are ordered by record, level, x, y, kind, so each level's
    // marks are one contiguous run.
    let marks: Vec<&DungeonMark> = knowledge.marks().iter().collect();
    for run in marks.chunk_by(|a, b| (a.record, a.level) == (b.record, b.level)) {
        let bytes: Vec<u8> = run.iter().flat_map(|m| [m.x, m.y, m.kind as u8]).collect();
        push(tag::DUNGEON_MARKS, [run[0].record, run[0].level, 0], &bytes);
    }

    for (scene, bits) in knowledge.sides() {
        if bits.iter().any(|b| *b != 0) {
            push(tag::SIDE_CELLS, [*scene, 0, 0], &bits[..]);
        }
    }

    let mut out = Vec::with_capacity(2 + list.as_bytes().len());
    list.write_counted(&mut out);
    out
}

/// Decodes a payload written with `version`. Unknown tags and unknown
/// codes are skipped; a payload of a newer version may carry bytes after
/// its sections.
pub fn decode_payload(bytes: &[u8], version: u16) -> Result<GemKnowledge, PayloadError> {
    if version == 0 {
        return Err(PayloadError::BadVersion);
    }
    let count = match bytes {
        [lo, hi, ..] => u16::from_le_bytes([*lo, *hi]),
        _ => return Err(PayloadError::MissingCount),
    };
    let body = &bytes[2..];
    let (sections, used) = read_sections(body, count)?;
    if version <= GEM_PAYLOAD_VERSION && used != body.len() {
        return Err(PayloadError::TrailingBytes { at: 2 + used });
    }

    let mut k = GemKnowledge::new();
    for section in sections {
        let payload = section.payload;
        let entries = |size: usize| -> Result<std::slice::ChunksExact<'_, u8>, PayloadError> {
            if payload.len() % size == 0 {
                Ok(payload.chunks_exact(size))
            } else {
                Err(PayloadError::BadLength {
                    tag: section.tag,
                    len: payload.len(),
                })
            }
        };
        let [key0, key1, _] = section.key;
        match section.tag {
            tag::ENTERED => {
                for &scene in payload {
                    k.apply(&KnowEvent::PlaceEntered { scene });
                }
            }
            tag::FLAGS => {
                let &[shrines, flags, named, ..] = payload else {
                    return Err(PayloadError::BadLength {
                        tag: section.tag,
                        len: payload.len(),
                    });
                };
                for bit in 0..8u8 {
                    if shrines & (1 << bit) != 0 {
                        k.apply(&KnowEvent::ShrineEntered { virtue: bit });
                    }
                    if named & (1 << bit) != 0 {
                        k.apply(&KnowEvent::EntranceNamed { record: bit });
                    }
                }
                if flags & FLAG_UNDERWORLD != 0 {
                    k.apply(&KnowEvent::UnderworldVisited);
                }
                if flags & FLAG_CODEX != 0 {
                    k.apply(&KnowEvent::CodexEntered);
                }
            }
            tag::GATES => {
                for e in entries(3)? {
                    k.apply(&KnowEvent::GateUsed(GateCell {
                        plane: e[0],
                        x: e[1],
                        y: e[2],
                    }));
                }
            }
            tag::SIGNS => {
                for e in entries(4)? {
                    k.apply(&KnowEvent::SignRead(SignCell {
                        scene: e[0],
                        floor: e[1] as i8,
                        x: e[2],
                        y: e[3],
                    }));
                }
            }
            tag::TRADES => {
                for e in entries(2)? {
                    if let Some(trade) = Trade::from_code(e[1]) {
                        k.apply(&KnowEvent::ShopOpened {
                            scene: e[0],
                            trade,
                            keeper: None,
                        });
                    }
                }
            }
            tag::KEEPERS => {
                for e in entries(5)? {
                    if let Some(trade) = Trade::from_code(e[4]) {
                        k.apply(&KnowEvent::ShopOpened {
                            scene: e[0],
                            trade,
                            keeper: Some((e[1] as i8, e[2], e[3])),
                        });
                    }
                }
            }
            tag::TOWN_LINKS => {
                for e in entries(5)? {
                    if let Some(dir) = link_dir(e[4]) {
                        k.apply(&KnowEvent::TownLinkUsed(TownLink {
                            scene: e[0],
                            floor: e[1] as i8,
                            x: e[2],
                            y: e[3],
                            dir,
                        }));
                    }
                }
            }
            tag::EXITS => {
                for e in entries(3)? {
                    if let Some(side) = edge_side(e[1]) {
                        k.apply(&KnowEvent::TownExit(ExitRun {
                            scene: e[0],
                            side,
                            at: e[2],
                        }));
                    }
                }
            }
            tag::DUNGEON_CELLS => {
                let rows: [u8; 8] = payload.try_into().map_err(|_| PayloadError::BadLength {
                    tag: section.tag,
                    len: payload.len(),
                })?;
                k.apply(&KnowEvent::DungeonCellsSeen {
                    record: key0,
                    level: key1,
                    rows,
                });
            }
            tag::DUNGEON_MARKS => {
                for e in entries(3)? {
                    let in_level = e[0] < DUNGEON_SIDE_CELLS && e[1] < DUNGEON_SIDE_CELLS;
                    if let (true, Some(kind)) = (in_level, mark_kind(e[2])) {
                        k.apply(&KnowEvent::DungeonMarked(DungeonMark {
                            record: key0,
                            level: key1,
                            x: e[0],
                            y: e[1],
                            kind,
                        }));
                    }
                }
            }
            tag::SIDE_CELLS => {
                if payload.len() != SIDE_WINDOW_BYTES {
                    return Err(PayloadError::BadLength {
                        tag: section.tag,
                        len: payload.len(),
                    });
                }
                for (byte_index, &byte) in payload.iter().enumerate() {
                    for bit in 0..8 {
                        if byte & (1 << bit) != 0 {
                            k.apply(&KnowEvent::SideCellSeen {
                                scene: key0,
                                index: (byte_index * 8 + bit) as u16,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }
    Ok(k)
}

fn link_dir(code: u8) -> Option<LinkDir> {
    match code {
        0 => Some(LinkDir::Up),
        1 => Some(LinkDir::Down),
        _ => None,
    }
}

fn edge_side(code: u8) -> Option<EdgeSide> {
    match code {
        0 => Some(EdgeSide::North),
        1 => Some(EdgeSide::East),
        2 => Some(EdgeSide::South),
        3 => Some(EdgeSide::West),
        _ => None,
    }
}

fn mark_kind(code: u8) -> Option<MarkKind> {
    match code {
        0 => Some(MarkKind::Ladder),
        1 => Some(MarkKind::Fountain),
        2 => Some(MarkKind::Chest),
        3 => Some(MarkKind::Trap),
        4 => Some(MarkKind::SecretDoor),
        _ => None,
    }
}

/// Why the attach starts from an empty record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmptyReason {
    /// No `gemmap.u5e`, or a container from a newer build (kept as is).
    Missing,
    /// The container did not decode; the store renamed it `.bad`.
    Corrupt,
    /// Another character's record; replaced at the next save.
    Foreign,
    /// The container is sound but its payload does not decode; replaced
    /// at the next save.
    Damaged,
}

/// What [`attach`] did with `gemmap.u5e` (10.3.7 "Freshness").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttachOutcome {
    /// `Fresh`: the record of the save on disk.
    Restored,
    /// `SameLineage`: the same Avatar saved elsewhere since; kept, and the
    /// frontend shows the foundation's note.
    RestoredStale,
    /// A newer payload version: used in RAM, never written. `stale` is
    /// true for `SameLineage`.
    ReadOnly { stale: bool },
    /// The record starts empty.
    Empty(EmptyReason),
}

impl AttachOutcome {
    /// Whether the record may be written at the next detected save. Only a
    /// newer build's file is protected; every other file is replaced.
    pub fn may_write(self) -> bool {
        !matches!(self, Self::ReadOnly { .. })
    }

    /// Whether the frontend shows the foundation's `SameLineage` note.
    pub fn shows_stale_note(self) -> bool {
        matches!(self, Self::RestoredStale | Self::ReadOnly { stale: true })
    }
}

/// Reads `gemmap.u5e` through `store` against `now`, the stamp of the save
/// on disk, and maps its freshness to a record and an outcome. A corrupt
/// container is renamed `.bad` by the store itself.
pub fn attach(store: &SidecarStore, now: &SaveStamp) -> (GemKnowledge, AttachOutcome) {
    let (freshness, payload) = store.read(GEM_SIDECAR_TAG, now);
    attach_payload(freshness, payload)
}

/// [`attach`] after the read: the policy alone, without the store.
pub fn attach_payload(
    freshness: Freshness,
    payload: Option<SidecarPayload>,
) -> (GemKnowledge, AttachOutcome) {
    let empty = |reason| (GemKnowledge::new(), AttachOutcome::Empty(reason));
    let stale = match freshness {
        Freshness::Fresh => false,
        Freshness::SameLineage => true,
        Freshness::Foreign => return empty(EmptyReason::Foreign),
        Freshness::Missing => return empty(EmptyReason::Missing),
        Freshness::Corrupt => return empty(EmptyReason::Corrupt),
    };
    let Some(payload) = payload else {
        return empty(EmptyReason::Missing);
    };
    match decode_payload(&payload.bytes, payload.version) {
        Err(_) => empty(EmptyReason::Damaged),
        Ok(knowledge) => {
            let outcome = if payload.version > GEM_PAYLOAD_VERSION {
                AttachOutcome::ReadOnly { stale }
            } else if stale {
                AttachOutcome::RestoredStale
            } else {
                AttachOutcome::Restored
            };
            (knowledge, outcome)
        }
    }
}

#[cfg(test)]
#[path = "payload_tests.rs"]
mod tests;

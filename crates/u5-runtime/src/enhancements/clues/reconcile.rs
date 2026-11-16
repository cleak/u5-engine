//! What a `journal.u5e` read gives the log (Phase B design 8.3.7, 8.2.9).
//!
//! The store ([`SidecarStore::read`](crate::enhancements::SidecarStore::read))
//! classifies the file against the save on disk with the foundation's
//! [`Freshness`] (1.3.10) and renames an undecodable container to `.bad`.
//! [`reconcile`] is the clue log's policy on top, pure:
//!
//! | Freshness | Log | Note |
//! |---|---|---|
//! | `Fresh` | the file's | none |
//! | `SameLineage` (saved while the log was off) | the file's, every entry | 5 s: the gap was not logged |
//! | `Foreign` (other Avatar, or a save older than the file) | empty; replaced at the next save (D8.5) | 5 s |
//! | `Missing` | empty | none |
//! | `Corrupt` | empty | 5 s |
//!
//! A payload newer than [`JOURNAL_PAYLOAD_VERSION`] is read as far as this
//! build knows its sections and is read-only: it is never overwritten or
//! renamed (R3.1). A payload that does not decode is `Corrupt`, and the
//! shell renames it as the store does for a bad container.

use super::ClueLog;
use super::codec::{JOURNAL_PAYLOAD_VERSION, decode_payload};
use crate::enhancements::sidecar::{Freshness, SidecarPayload};

/// How long a reconcile note shows (8.2.9).
pub const NOTE_SECS: u32 = 5;

/// The note a read shows, our own words.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReconcileNote {
    /// `SameLineage`: the game was saved while the log was off.
    GapNotLogged,
    /// `Foreign`: the file belongs to another save.
    Foreign,
    /// The file could not be read.
    Corrupt,
    /// The file is from a newer build and could not be read here.
    Newer,
}

impl ReconcileNote {
    pub fn text(self) -> &'static str {
        match self {
            ReconcileNote::GapNotLogged => {
                "Clues heard while the clue log was off were not logged."
            }
            ReconcileNote::Foreign => "The clue log is from another save. Starting a new one.",
            ReconcileNote::Corrupt => "The clue log could not be read. Starting a new one.",
            ReconcileNote::Newer => "The clue log is from a newer version and is left as it is.",
        }
    }
}

/// The outcome of a read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reconciled {
    pub log: ClueLog,
    pub note: Option<ReconcileNote>,
    /// The file is from a newer payload version: never write over it.
    pub read_only: bool,
    /// The payload did not decode: rename the file to `.bad`.
    pub rename_bad: bool,
}

impl Reconciled {
    fn empty(note: Option<ReconcileNote>) -> Self {
        Self {
            log: ClueLog::default(),
            note,
            read_only: false,
            rename_bad: false,
        }
    }
}

/// The log a read gives, by freshness (module table). The log comes back
/// clean: nothing unsaved, nothing dirty.
pub fn reconcile(freshness: Freshness, payload: Option<&SidecarPayload>) -> Reconciled {
    let kept = |note: Option<ReconcileNote>| {
        // The store returns a payload for every file it could decode.
        let Some(payload) = payload else {
            return Reconciled::empty(None);
        };
        let read_only = payload.version > JOURNAL_PAYLOAD_VERSION;
        match decode_payload(&payload.bytes) {
            Ok(log) => Reconciled {
                log,
                note,
                read_only,
                rename_bad: false,
            },
            Err(_) if read_only => Reconciled {
                read_only,
                ..Reconciled::empty(Some(ReconcileNote::Newer))
            },
            Err(_) => Reconciled {
                rename_bad: true,
                ..Reconciled::empty(Some(ReconcileNote::Corrupt))
            },
        }
    };
    match freshness {
        Freshness::Fresh => kept(None),
        Freshness::SameLineage => kept(Some(ReconcileNote::GapNotLogged)),
        Freshness::Foreign => Reconciled::empty(Some(ReconcileNote::Foreign)),
        Freshness::Missing => Reconciled::empty(None),
        Freshness::Corrupt => Reconciled::empty(Some(ReconcileNote::Corrupt)),
    }
}

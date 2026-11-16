//! The clue edge table (Phase B design 8.3.8; build plan M6-07).
//!
//! **Ids only.** Each row links a dialog, `(family, dialog_id)`, to a
//! tracker step. The rows are hand-written from the source people
//! `catalogs/quest-graph.md` §3-§10 names, each mapped to its dialog
//! through the roster coordinate of `catalogs/npc-roster.md` §3 that its
//! `spec_ref` cites. Names, keywords and answers stay in the spec; nothing
//! here reads `.TLK` text to decide relevance.
//!
//! **Levels.** [`NPC_EDGES`] are Npc-level: any topic the player heard
//! from a named source counts, and only with `journal.tracker_evidence =
//! npc` (a dev option, D8.9). [`RECORD_EDGES`] are Record-level and stay
//! empty until the spec publishes its machine-readable edge file (S8.3);
//! there is no generator yet.
//!
//! **Conflicts (S8.5).** Where the spec names different sources for the
//! same step in different documents, both are linked, each with its own
//! reference and `conflict` set; [`conflicts`] reports them.
//!
//! Sources the roster does not list (a party member) or chains with no
//! tracker card (moongate stones, Stonegate, mystic arms, reagents) have
//! no row.

use crate::enhancements::journal::TrackerEvidence;
use crate::quest_journal::trackers::{ClueEdge, EdgeLevel, EdgeTable, QuestId, QuestKind, StepId};

use super::DialogKey;

/// The `.TLK` families, in scene order (`systems/conversation.md` §3).
pub const TOWNE: u8 = 0;
pub const DWELLING: u8 = 1;
pub const CASTLE: u8 = 2;
pub const KEEP: u8 = 3;

const fn npc(
    family: u8,
    dialog_id: u8,
    kind: QuestKind,
    index: u8,
    step: u8,
    spec_ref: &'static str,
) -> ClueEdge {
    ClueEdge {
        dialog: DialogKey { family, dialog_id },
        record: None,
        step: StepId::new(QuestId::new(kind, index), step),
        level: EdgeLevel::Npc,
        spec_ref,
        conflict: false,
    }
}

const fn conflict(edge: ClueEdge) -> ClueEdge {
    ClueEdge {
        conflict: true,
        ..edge
    }
}

use QuestKind::{
    Companions, Password, Regalia, SandalwoodBox, Shadowlord, Shrine, Tools, WordOfPower,
};

/// Record-level edges `(family, dialog, record) -> step`: none until the
/// spec's edge file (S8.3).
pub const RECORD_EDGES: &[ClueEdge] = &[];

/// Npc-level edges, by quest-graph section. Indices follow the tracker's
/// table: shrines in virtue order, dungeons from Deceit, Shadowlord
/// slots, regalia pieces and tools in `quest_journal::text` order.
#[rustfmt::skip]
pub const NPC_EDGES: &[ClueEdge] = &[
    // §3 Password gates: 0 the Resistance's, 1 the Oppression's.
    npc(TOWNE, 1, Password, 0, 0, "catalogs/quest-graph.md §3; catalogs/npc-roster.md §3 TOWNE:0 slot 3"),
    npc(TOWNE, 17, Password, 0, 0, "catalogs/quest-graph.md §3; catalogs/npc-roster.md §3 TOWNE:3 slot 14"),
    npc(CASTLE, 21, Password, 0, 0, "catalogs/quest-graph.md §3; catalogs/npc-roster.md §3 CASTLE:3 slot 2"),
    npc(CASTLE, 10, Password, 1, 0, "catalogs/quest-graph.md §3; catalogs/npc-roster.md §3 CASTLE:1 slot 5"),
    npc(CASTLE, 7, Password, 1, 0, "catalogs/quest-graph.md §3; catalogs/npc-roster.md §3 CASTLE:0 slot 26"),
    npc(TOWNE, 38, Password, 1, 0, "catalogs/quest-graph.md §3; catalogs/npc-roster.md §3 TOWNE:6 slot 5"),
    // §4 Dungeon words: step 0, learn the word. Doom has no clue source.
    npc(TOWNE, 2, WordOfPower, 0, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 TOWNE:0 slot 4"),
    npc(TOWNE, 12, WordOfPower, 1, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 TOWNE:1 slot 13"),
    npc(TOWNE, 14, WordOfPower, 2, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 TOWNE:2 slot 8"),
    npc(TOWNE, 15, WordOfPower, 2, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 TOWNE:2 slot 9"),
    npc(TOWNE, 22, WordOfPower, 3, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 TOWNE:3 slot 10"),
    npc(TOWNE, 26, WordOfPower, 4, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 TOWNE:4 slot 3"),
    npc(TOWNE, 32, WordOfPower, 5, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 TOWNE:5 slot 5"),
    conflict(npc(CASTLE, 17, WordOfPower, 6, 0, "catalogs/quest-graph.md §4; catalogs/npc-roster.md §3 CASTLE:1 slot 31")),
    conflict(npc(TOWNE, 46, WordOfPower, 6, 0, "catalogs/sage-rumours.md §4 row 14; catalogs/npc-roster.md §3 TOWNE:7 slot 6")),
    // §5 Shadowlords: step 0, learn the name.
    npc(KEEP, 12, Shadowlord, 0, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 KEEP:5 slot 6"),
    npc(CASTLE, 31, Shadowlord, 0, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 CASTLE:6 slot 3"),
    npc(CASTLE, 32, Shadowlord, 0, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 CASTLE:6 slot 4"),
    npc(KEEP, 11, Shadowlord, 0, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 KEEP:5 slot 5"),
    npc(DWELLING, 11, Shadowlord, 1, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 DWELLING:6 slot 1"),
    npc(KEEP, 16, Shadowlord, 1, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 KEEP:6 slot 10"),
    npc(KEEP, 22, Shadowlord, 2, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 KEEP:7 slot 11"),
    npc(KEEP, 25, Shadowlord, 2, 0, "catalogs/quest-graph.md §5; catalogs/npc-roster.md §3 KEEP:7 slot 18"),
    // §6 Royal artifacts: 0 Crown, 1 Sceptre, 2 Amulet.
    npc(TOWNE, 17, Regalia, 0, 0, "catalogs/quest-graph.md §6; catalogs/npc-roster.md §3 TOWNE:3 slot 14"),
    conflict(npc(TOWNE, 18, Regalia, 0, 0, "catalogs/quest-graph.md §6; catalogs/npc-roster.md §3 TOWNE:3 slot 13")),
    conflict(npc(TOWNE, 9, Regalia, 0, 0, "catalogs/sage-rumours.md §4 row 15; catalogs/npc-roster.md §3 TOWNE:1 slot 6")),
    npc(TOWNE, 37, Regalia, 1, 0, "catalogs/quest-graph.md §6; catalogs/npc-roster.md §3 TOWNE:6 slot 4"),
    npc(TOWNE, 36, Regalia, 1, 0, "catalogs/quest-graph.md §6; catalogs/npc-roster.md §3 TOWNE:3 slot 19"),
    npc(KEEP, 2, Regalia, 1, 0, "catalogs/quest-graph.md §6; catalogs/npc-roster.md §3 KEEP:1 slot 1"),
    npc(KEEP, 2, Regalia, 2, 0, "catalogs/quest-graph.md §6; catalogs/npc-roster.md §3 KEEP:1 slot 1"),
    npc(KEEP, 3, Regalia, 2, 0, "catalogs/quest-graph.md §6; catalogs/npc-roster.md §3 KEEP:1 slot 2"),
    // §7 The Sandalwood Box.
    npc(CASTLE, 7, SandalwoodBox, 0, 0, "catalogs/quest-graph.md §7; catalogs/npc-roster.md §3 CASTLE:0 slot 26"),
    // §8 Mantras: step 0, learn the mantra. Honesty and Sacrifice have no
    // source yet (S8.6).
    npc(TOWNE, 6, Shrine, 1, 0, "catalogs/quest-graph.md §8; catalogs/npc-roster.md §3 TOWNE:1 slot 14"),
    npc(TOWNE, 16, Shrine, 2, 0, "catalogs/quest-graph.md §8; catalogs/npc-roster.md §3 TOWNE:2 slot 10"),
    npc(TOWNE, 17, Shrine, 3, 0, "catalogs/quest-graph.md §8; catalogs/npc-roster.md §3 TOWNE:3 slot 14"),
    npc(TOWNE, 34, Shrine, 5, 0, "catalogs/quest-graph.md §8; catalogs/npc-roster.md §3 TOWNE:5 slot 9"),
    npc(TOWNE, 39, Shrine, 6, 0, "catalogs/quest-graph.md §8; catalogs/npc-roster.md §3 TOWNE:6 slot 6"),
    npc(TOWNE, 48, Shrine, 7, 0, "catalogs/quest-graph.md §8; catalogs/npc-roster.md §3 TOWNE:7 slot 8"),
    // §9 Tools: 0 carpet, 1 skull keys, 2 grapple, 3 spyglass, 4 sextant,
    // 7 badge, 8 the Glass Sword.
    npc(CASTLE, 3, Tools, 0, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 CASTLE:0 slot 18"),
    npc(KEEP, 23, Tools, 0, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 KEEP:7 slot 16"),
    npc(CASTLE, 30, Tools, 0, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 CASTLE:5 slot 6"),
    npc(DWELLING, 13, Tools, 0, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 DWELLING:4 slot 4"),
    npc(KEEP, 24, Tools, 1, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 KEEP:7 slot 17"),
    npc(CASTLE, 36, Tools, 2, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 CASTLE:7 slot 8"),
    npc(KEEP, 16, Tools, 2, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 KEEP:6 slot 10"),
    npc(KEEP, 6, Tools, 3, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 KEEP:2 slot 3"),
    npc(KEEP, 4, Tools, 3, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 KEEP:2 slot 1"),
    npc(CASTLE, 35, Tools, 4, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 CASTLE:7 slot 7"),
    npc(DWELLING, 7, Tools, 4, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 DWELLING:2 slot 3"),
    npc(KEEP, 9, Tools, 7, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 KEEP:3 slot 2"),
    npc(TOWNE, 8, Tools, 8, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 TOWNE:1 slot 5"),
    npc(TOWNE, 3, Tools, 8, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 TOWNE:0 slot 5"),
    npc(CASTLE, 37, Tools, 8, 0, "catalogs/quest-graph.md §9; catalogs/npc-roster.md §3 CASTLE:7 slot 9"),
    // §10 Companion join paths.
    npc(TOWNE, 11, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 TOWNE:1 slot 8"),
    npc(TOWNE, 35, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 TOWNE:3 slot 21"),
    npc(TOWNE, 47, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 TOWNE:7 slot 7"),
    npc(KEEP, 1, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 KEEP:0 slot 1"),
    npc(KEEP, 27, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 KEEP:5 slot 19"),
    npc(KEEP, 28, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 KEEP:6 slot 15"),
    npc(KEEP, 30, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 KEEP:1 slot 4"),
    npc(KEEP, 31, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 KEEP:1 slot 5"),
    npc(KEEP, 29, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 KEEP:7 slot 19"),
    npc(CASTLE, 40, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 CASTLE:7 slot 12"),
    npc(KEEP, 32, Companions, 0, 0, "catalogs/quest-graph.md §10; catalogs/npc-roster.md §3 KEEP:6 slot 16"),
];

/// Every edge, Npc-level first. [`crate::quest_journal::trackers`]
/// counts an Npc-level edge only with `tracker_evidence = npc`.
pub fn all_edges() -> impl Iterator<Item = &'static ClueEdge> {
    NPC_EDGES.iter().chain(RECORD_EDGES)
}

/// The table the tracker reads under `evidence`: the Record-level rows
/// (empty until S8.3), plus the Npc-level rows with `npc`.
pub fn edge_table(evidence: TrackerEvidence) -> EdgeTable<'static> {
    match evidence {
        TrackerEvidence::Record => EdgeTable {
            edges: RECORD_EDGES,
        },
        TrackerEvidence::Npc => EdgeTable { edges: NPC_EDGES },
    }
}

/// The steps the spec gives conflicting sources for (S8.5), each with the
/// references of both sides.
pub fn conflicts() -> Vec<(StepId, Vec<&'static str>)> {
    let mut out: Vec<(StepId, Vec<&'static str>)> = Vec::new();
    for edge in all_edges().filter(|edge| edge.conflict) {
        match out.iter_mut().find(|(step, _)| *step == edge.step) {
            Some((_, refs)) => refs.push(edge.spec_ref),
            None => out.push((edge.step, vec![edge.spec_ref])),
        }
    }
    out
}

#[cfg(test)]
#[path = "edges_tests.rs"]
mod tests;

//! Chapter 8's quest trackers, Phase 2a: cards from save evidence (Phase B
//! design 8.2.7, 8.3.9; build plan M6-06).
//!
//! [`QUEST_CHAINS`] lists every chain the tracker knows, each with its
//! steps, the evidence that proves a step ([`Evidence`]) and the spec
//! paths it is built from. [`derive_trackers`] turns a [`QuestSnapshot`]
//! (and, when the clue log is on, the player's own pins and marks) into a
//! [`TrackerView`] at one [`SpoilerTier`].
//!
//! **Shown-when (U17, foundation 1.4.7).** At tiers 1 and 2 a card exists
//! only with save evidence that the player discovered it, or a clue the
//! player pinned to it (or an "I know this" mark); a pinned-only card
//! names no step. Tiers 3 and 4 reach the rest of the table. No line has a
//! denominator below tier 4. Seals show their current state. Knowledge
//! steps (no save evidence exists) show `?`, and at best a hollow tick.
//! The next-step lines come from [`super::text`] by tier, and the tier 4
//! answer exists only behind the 8.3.9 evidence gate.
//!
//! [`derive_trackers`] is monotone in the tier: each card at tier *t* is
//! in the view at tier *t+1*, its steps a subsequence of the steps there
//! and its next lines a prefix; an answer appears only at tier 4.
//!
//! Pure: no file, no RNG, nothing written. The order by recency reads a
//! session-only [`TrackerActivity`] the shell keeps.

use crate::enhancements::clues::{ClueEntry, ClueLog, DialogKey, GameMinute, Link, RecordAddr};
use crate::enhancements::journal::{SpoilerTier, TrackerEvidence};
use crate::shrine_virtue::ShrineVirtue;
use crate::{
    SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT, SHADOWLORD_COUNT, SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
    SPECIAL_ITEM_WOODEN_BOX_INDEX, dungeon_resident_name, dungeon_word_of_power,
    shadowlord_name_for_slot, shrine_mantra_for,
};

use super::text::{self, ToolSource};
use super::{
    CompanionPlace, FIRST_DUNGEON_SCENE, JournalText, Lines, QuestSnapshot, ShadowlordState,
    ShrineCondition, ShrinePhase, StatusMark,
};

// --- Ids. ----------------------------------------------------------------

/// A chain of the tracker table (8.3.9), in the table's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum QuestKind {
    Shrine = 0,
    WordOfPower = 1,
    Shadowlord = 2,
    Shard = 3,
    Regalia = 4,
    SandalwoodBox = 5,
    Tools = 6,
    Companions = 7,
    Doom = 8,
    Password = 9,
}

impl QuestKind {
    pub const ALL: [QuestKind; 10] = [
        QuestKind::Shrine,
        QuestKind::WordOfPower,
        QuestKind::Shadowlord,
        QuestKind::Shard,
        QuestKind::Regalia,
        QuestKind::SandalwoodBox,
        QuestKind::Tools,
        QuestKind::Companions,
        QuestKind::Doom,
        QuestKind::Password,
    ];
}

/// One quest: a chain and its instance (virtue, dungeon record, slot...).
/// Its [`QuestId::code`] is the LINKS `target` of a kind 0 pin (8.3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct QuestId {
    pub kind: QuestKind,
    pub index: u8,
}

impl QuestId {
    pub const fn new(kind: QuestKind, index: u8) -> Self {
        Self { kind, index }
    }

    /// `kind << 4 | index`: one byte.
    pub const fn code(self) -> u16 {
        ((self.kind as u16) << 4) | (self.index as u16 & 0x0F)
    }

    pub fn from_code(code: u16) -> Option<Self> {
        let kind = *QuestKind::ALL.get(usize::from(code >> 4))?;
        Some(Self::new(kind, (code & 0x0F) as u8))
    }
}

/// One step of a quest. Its [`StepId::code`] is the LINKS `target` of a
/// kind 1 pin and a KNOWN entry (8.3.7).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StepId {
    pub quest: QuestId,
    pub step: u8,
}

impl StepId {
    pub const fn new(quest: QuestId, step: u8) -> Self {
        Self { quest, step }
    }

    /// The quest's code in the high byte, the step in the low byte.
    pub const fn code(self) -> u16 {
        (self.quest.code() << 8) | self.step as u16
    }

    pub fn from_code(code: u16) -> Option<Self> {
        Some(Self::new(QuestId::from_code(code >> 8)?, code as u8))
    }
}

// --- The chain table. ----------------------------------------------------

/// A save test on a snapshot and the quest's instance.
pub type SaveTest = fn(&QuestSnapshot, u8) -> bool;

/// What proves a step (8.3.9).
#[derive(Clone, Copy, Debug)]
pub enum Evidence {
    /// The save shows it done.
    Save(SaveTest),
    /// No save evidence exists: `?`, at best a hollow tick.
    Knowledge,
}

/// One step of a chain.
#[derive(Clone, Copy, Debug)]
pub struct StepSpec {
    pub step: u8,
    pub evidence: Evidence,
    /// The step's name, in our own words ([`text`]).
    pub label: &'static str,
    /// The game has already given this step (chapter 5.2.5), so tier 2
    /// may name it before it is done.
    pub given: Option<SaveTest>,
}

/// One chain of the table: `count` quests of one kind.
#[derive(Clone, Copy, Debug)]
pub struct ChainSpec {
    pub kind: QuestKind,
    pub count: u8,
    pub steps: &'static [StepSpec],
    /// The spec paths the row is built from (8.3.9 table).
    pub spec_ref: &'static str,
}

impl ChainSpec {
    pub fn quests(&self) -> impl Iterator<Item = QuestId> + '_ {
        (0..self.count).map(|index| QuestId::new(self.kind, index))
    }
}

const fn step(step: u8, evidence: Evidence, label: &'static str) -> StepSpec {
    StepSpec {
        step,
        evidence,
        label,
        given: None,
    }
}

fn virtue(index: u8) -> ShrineVirtue {
    ShrineVirtue::ALL[usize::from(index) % ShrineVirtue::ALL.len()]
}

fn chanted(snap: &QuestSnapshot, index: u8) -> bool {
    snap.phase(virtue(index)) != ShrinePhase::NotStarted
}

fn codex_read(snap: &QuestSnapshot, index: u8) -> bool {
    matches!(
        snap.phase(virtue(index)),
        ShrinePhase::CodexRead | ShrinePhase::Complete
    )
}

fn ordained_now(snap: &QuestSnapshot, index: u8) -> bool {
    snap.phase(virtue(index)) == ShrinePhase::Ordained
}

fn shrine_complete(snap: &QuestSnapshot, index: u8) -> bool {
    snap.phase(virtue(index)) == ShrinePhase::Complete
}

fn seal_open(snap: &QuestSnapshot, index: u8) -> bool {
    snap.seal_open(usize::from(index))
}

fn vanquished(snap: &QuestSnapshot, index: u8) -> bool {
    snap.shadowlords[usize::from(index)] == ShadowlordState::Vanquished
}

fn shard_held(snap: &QuestSnapshot, index: u8) -> bool {
    snap.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX + usize::from(index)] != 0
}

/// The Crown, the Sceptre and the Amulet: [`text::REGALIA`] without the
/// Sandalwood Box.
fn regalia_index(index: u8) -> usize {
    text::REGALIA
        .iter()
        .filter(|(_, item, _)| *item != SPECIAL_ITEM_WOODEN_BOX_INDEX)
        .nth(usize::from(index))
        .map_or(SPECIAL_ITEM_WOODEN_BOX_INDEX, |(_, item, _)| *item)
}

fn regalia_carried(snap: &QuestSnapshot, index: u8) -> bool {
    snap.special_items[regalia_index(index)] != 0
}

fn box_carried(snap: &QuestSnapshot, _: u8) -> bool {
    snap.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0
}

/// Tool `index` of [`text::TOOLS`], then the Glass Sword.
fn tool_carried(snap: &QuestSnapshot, index: u8) -> bool {
    match text::TOOLS.get(usize::from(index)) {
        Some((_, ToolSource::Special(item))) => snap.special_items[*item] != 0,
        Some((_, ToolSource::ClimbingGear)) => snap.climbing_gear != 0,
        None => snap.glass_sword != 0,
    }
}

fn tool_label(index: u8) -> &'static str {
    text::TOOLS
        .get(usize::from(index))
        .map_or(text::GLASS_SWORD, |(label, _)| label)
}

/// Companions the player has: party members after the Avatar, and the
/// records lodged at an inn (5.2.8, 6).
fn companions(snap: &QuestSnapshot) -> impl Iterator<Item = &super::CompanionFact> {
    snap.companions
        .iter()
        .enumerate()
        .filter(|(at, companion)| match companion.place {
            CompanionPlace::Party => *at > 0,
            CompanionPlace::Lodged { .. } => true,
            CompanionPlace::Executed => false,
        })
        .map(|(_, companion)| companion)
}

fn has_companion(snap: &QuestSnapshot, _: u8) -> bool {
    companions(snap).next().is_some()
}

fn all_vanquished(snap: &QuestSnapshot, _: u8) -> bool {
    snap.shadowlords
        .iter()
        .all(|state| *state == ShadowlordState::Vanquished)
}

const DOOM_RECORD: usize = SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT - 1;

fn doom_open(snap: &QuestSnapshot, _: u8) -> bool {
    snap.seal_open(DOOM_RECORD)
}

/// Doom readiness (8.3.9): the Doom seal open and all three Shadowlords
/// vanquished.
pub fn doom_ready(snap: &QuestSnapshot) -> bool {
    doom_open(snap, 0) && all_vanquished(snap, 0)
}

const SHRINE_STEPS: &[StepSpec] = &[
    step(0, Evidence::Knowledge, text::STEP_LEARN_MANTRA),
    step(1, Evidence::Save(chanted), text::STEP_CHANT),
    StepSpec {
        given: Some(ordained_now),
        ..step(2, Evidence::Save(codex_read), text::STEP_READ_CODEX)
    },
    step(3, Evidence::Save(shrine_complete), text::STEP_RETURN),
];

/// The tracker's chains (8.3.9 table), each with its spec paths (T15).
pub const QUEST_CHAINS: &[ChainSpec] = &[
    ChainSpec {
        kind: QuestKind::Shrine,
        count: 8,
        steps: SHRINE_STEPS,
        spec_ref: "formats/saved-gam.md §9.1; systems/karma.md §10; catalogs/quest-graph.md §8",
    },
    ChainSpec {
        kind: QuestKind::WordOfPower,
        count: 8,
        steps: &[
            step(0, Evidence::Knowledge, text::STEP_LEARN_WORD),
            step(1, Evidence::Save(seal_open), text::STEP_SPEAK_WORD),
        ],
        spec_ref: "formats/saved-gam.md §9.1; catalogs/quest-graph.md §4",
    },
    ChainSpec {
        kind: QuestKind::Shadowlord,
        count: 3,
        steps: &[
            step(0, Evidence::Knowledge, text::STEP_LEARN_NAME),
            step(1, Evidence::Save(vanquished), text::STEP_DESTROY_SHARD),
        ],
        spec_ref: "formats/saved-gam.md §9.1; catalogs/quest-graph.md §5",
    },
    ChainSpec {
        kind: QuestKind::Shard,
        count: 3,
        steps: &[step(0, Evidence::Save(shard_held), text::STEP_CARRY)],
        spec_ref: "formats/saved-gam.md §7; catalogs/quest-graph.md §5",
    },
    ChainSpec {
        kind: QuestKind::Regalia,
        count: 3,
        steps: &[step(0, Evidence::Save(regalia_carried), text::STEP_CARRY)],
        spec_ref: "formats/saved-gam.md §7; catalogs/quest-graph.md §6",
    },
    ChainSpec {
        kind: QuestKind::SandalwoodBox,
        count: 1,
        steps: &[step(0, Evidence::Save(box_carried), text::STEP_CARRY)],
        spec_ref: "formats/saved-gam.md §7; catalogs/quest-graph.md §7",
    },
    ChainSpec {
        kind: QuestKind::Tools,
        count: text::TOOLS.len() as u8 + 1,
        steps: &[step(0, Evidence::Save(tool_carried), text::STEP_CARRY)],
        spec_ref: "formats/saved-gam.md §7; catalogs/quest-graph.md §9",
    },
    ChainSpec {
        kind: QuestKind::Companions,
        count: 1,
        steps: &[step(0, Evidence::Save(has_companion), text::STEP_TRAVEL)],
        spec_ref: "formats/saved-gam.md §3, §4; catalogs/quest-graph.md §10",
    },
    ChainSpec {
        kind: QuestKind::Doom,
        count: 1,
        steps: &[
            step(0, Evidence::Save(all_vanquished), text::ENDGAME_SHADOWLORDS),
            step(1, Evidence::Save(doom_open), text::ENDGAME_DOOM),
        ],
        spec_ref: "formats/saved-gam.md §9.1; catalogs/quest-graph.md §4, §5",
    },
    ChainSpec {
        kind: QuestKind::Password,
        count: 2,
        steps: &[step(0, Evidence::Knowledge, text::STEP_LEARN_PASSWORD)],
        spec_ref: "catalogs/quest-graph.md §3",
    },
];

/// The chain of `kind`.
pub fn chain(kind: QuestKind) -> &'static ChainSpec {
    QUEST_CHAINS
        .iter()
        .find(|chain| chain.kind == kind)
        .unwrap_or(&QUEST_CHAINS[0])
}

// --- Clue edges (filled by M6-07). ----------------------------------------

/// How sure an edge is (8.3.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EdgeLevel {
    /// The record that carries the step (spec edge, S8.1/S8.3).
    Record,
    /// Any answer from a source person the spec names
    /// (`tracker_evidence = npc`).
    Npc,
}

/// One spec edge from a dialog (and record) to a step. Ids only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClueEdge {
    pub dialog: DialogKey,
    /// `None` for an Npc-level edge.
    pub record: Option<RecordAddr>,
    pub step: StepId,
    pub level: EdgeLevel,
    /// The spec documents the edge comes from (T15).
    pub spec_ref: &'static str,
    /// The spec names other sources for the same step elsewhere (S8.5):
    /// both sides are linked, each with its own reference.
    pub conflict: bool,
}

/// The edge table [`derive_trackers`] reads (`enhancements::clues::edges`).
#[derive(Clone, Copy, Debug, Default)]
pub struct EdgeTable<'a> {
    pub edges: &'a [ClueEdge],
}

impl EdgeTable<'_> {
    pub const EMPTY: EdgeTable<'static> = EdgeTable { edges: &[] };
}

/// The chains shown as one summary card (index 0): an edge or a pin to
/// any of their instances counts on that card.
const fn summary(kind: QuestKind) -> bool {
    matches!(
        kind,
        QuestKind::Shard | QuestKind::Regalia | QuestKind::Tools
    )
}

/// The card a quest's edges and pins count on.
pub const fn card_of(quest: QuestId) -> QuestId {
    if summary(quest.kind) {
        QuestId::new(quest.kind, 0)
    } else {
        quest
    }
}

// --- The view. -----------------------------------------------------------

/// A step's tick (8.3.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tick {
    /// Proven by the save.
    Done,
    /// Heard, not proven: a Record-level clue, a pin or "I know this".
    Hollow,
    /// A Knowledge step with nothing yet: `?`.
    Unknown,
    /// A save step not done yet.
    Open,
}

impl Tick {
    /// The box the page draws.
    pub const fn glyph(self) -> &'static str {
        match self {
            Tick::Done => "[x]",
            Tick::Hollow => "[o]",
            Tick::Unknown => "[?]",
            Tick::Open => "[ ]",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepLine {
    pub step: StepId,
    pub label: JournalText,
    pub tick: Tick,
}

/// A tier 4 answer that passed the evidence gate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrackerAnswer {
    /// "Mantra", "Word" or "Flame".
    pub label: &'static str,
    pub secret: &'static str,
    /// The step it answers.
    pub step: StepId,
}

/// One card of the Quests tab.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackerCard {
    pub quest: QuestId,
    pub title: JournalText,
    /// A state box (`[Codex read]`), or a plain value (`Shards held 2`).
    pub boxed: bool,
    pub state: JournalText,
    pub mark: StatusMark,
    /// The short next-step cell, by tier; may be empty.
    pub cell: JournalText,
    /// The steps the tier may name, in chain order.
    pub steps: Vec<StepLine>,
    /// Lines under the steps (a tool or companion list).
    pub notes: Vec<JournalText>,
    /// The next-step lines, lowest tier first.
    pub next: Vec<JournalText>,
    /// Entries the player pinned to the quest or one of its steps.
    pub pinned: u16,
    /// Heard clues the spec's edges link to it (tier 3 and above only).
    pub heard: u16,
    pub answer: Option<TrackerAnswer>,
    /// The latest activity: a save change or a pinned clue.
    pub recency: Option<GameMinute>,
}

impl TrackerCard {
    fn new(quest: QuestId, title: impl Into<JournalText>) -> Self {
        Self {
            quest,
            title: title.into(),
            boxed: true,
            state: JournalText::Static(text::UNKNOWN_STATE),
            mark: StatusMark::BODY,
            cell: JournalText::Static(""),
            steps: Vec::new(),
            notes: Vec::new(),
            next: Vec::new(),
            pinned: 0,
            heard: 0,
            answer: None,
            recency: None,
        }
    }

    /// Every string of the card, for the spoiler and wording tests.
    pub fn strings(&self) -> impl Iterator<Item = &str> {
        [self.title.as_str(), self.state.as_str(), self.cell.as_str()]
            .into_iter()
            .chain(self.steps.iter().map(|step| step.label.as_str()))
            .chain(self.notes.iter().map(JournalText::as_str))
            .chain(self.next.iter().map(JournalText::as_str))
            .chain(self.answer.iter().map(|answer| answer.secret))
    }
}

/// The tracker at one tier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackerView {
    pub tier: SpoilerTier,
    /// Tiers 3 and 4.
    pub spoilers: bool,
    /// Most recent activity first, then the table's (virtue) order.
    pub cards: Vec<TrackerCard>,
    /// The clue log is off: the cards come from the save alone.
    pub clue_log_off: bool,
}

impl TrackerView {
    pub fn card(&self, quest: QuestId) -> Option<&TrackerCard> {
        self.cards.iter().find(|card| card.quest == quest)
    }

    pub fn strings(&self) -> impl Iterator<Item = &str> {
        self.cards.iter().flat_map(TrackerCard::strings)
    }
}

// --- Recency. ------------------------------------------------------------

/// When each quest's save evidence last changed, this session (8.2.7
/// order). Kept by the shell, never saved.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrackerActivity {
    seen: Vec<(QuestId, u32)>,
    stamps: Vec<(QuestId, GameMinute)>,
}

impl TrackerActivity {
    /// Notes the evidence in `snap` at `now`: a quest whose evidence
    /// differs from the last note is stamped. The first note stamps
    /// nothing.
    pub fn note(&mut self, snap: &QuestSnapshot, now: GameMinute) {
        let first = self.seen.is_empty();
        for chain in QUEST_CHAINS {
            for quest in chain.quests() {
                let key = evidence_key(snap, quest);
                match self.seen.iter_mut().find(|(seen, _)| *seen == quest) {
                    Some((_, old)) if *old == key => {}
                    Some((_, old)) => {
                        *old = key;
                        self.stamp(quest, now);
                    }
                    None => {
                        self.seen.push((quest, key));
                        if !first {
                            self.stamp(quest, now);
                        }
                    }
                }
            }
        }
    }

    fn stamp(&mut self, quest: QuestId, now: GameMinute) {
        match self.stamps.iter_mut().find(|(at, _)| *at == quest) {
            Some((_, when)) => *when = now,
            None => self.stamps.push((quest, now)),
        }
    }

    pub fn last(&self, quest: QuestId) -> Option<GameMinute> {
        self.stamps
            .iter()
            .find(|(at, _)| *at == quest)
            .map(|(_, when)| *when)
    }
}

/// The save facts a quest's card is built from, folded into one number.
fn evidence_key(snap: &QuestSnapshot, quest: QuestId) -> u32 {
    let index = quest.index;
    match quest.kind {
        QuestKind::Shrine => {
            let virtue = virtue(index);
            (snap.phase(virtue) as u32) << 8 | u32::from(snap.ruin[virtue.index()])
        }
        QuestKind::WordOfPower => u32::from(snap.seals[usize::from(index)]),
        QuestKind::Shadowlord => snap.shadowlords[usize::from(index)] as u32,
        QuestKind::Shard => u32::from(shard_held(snap, index)),
        QuestKind::Regalia => u32::from(regalia_carried(snap, index)),
        QuestKind::SandalwoodBox => u32::from(box_carried(snap, index)),
        QuestKind::Tools => u32::from(tool_carried(snap, index)),
        QuestKind::Companions => companions(snap).count() as u32,
        QuestKind::Doom => u32::from(doom_ready(snap)),
        QuestKind::Password => 0,
    }
}

// --- Derive. -------------------------------------------------------------

/// What the clue log says about one quest.
#[derive(Default)]
struct ClueFacts {
    /// Distinct entries pinned to the quest or a step.
    pinned: Vec<u16>,
    /// Steps pinned or marked "I know this": a hollow tick.
    hollow: Vec<u8>,
    /// Steps with a pin (gate (c)) or a captured clue (gate (b)).
    gated: Vec<u8>,
    /// Spec-linked heard clues, counted by entry (Record) or source (Npc).
    heard: u16,
    /// The latest hearing of a pinned entry.
    last: Option<GameMinute>,
}

fn clue_facts(
    log: Option<&ClueLog>,
    edges: &EdgeTable<'_>,
    quest: QuestId,
    evidence: TrackerEvidence,
) -> ClueFacts {
    let mut facts = ClueFacts::default();
    let Some(log) = log else {
        return facts;
    };
    for link in log.links() {
        let step = match link.kind {
            0 if link.target == quest.code() => None,
            1 => match StepId::from_code(link.target) {
                Some(step) if step.quest == quest => Some(step.step),
                _ => continue,
            },
            _ => continue,
        };
        if let Some(entry) = log.entries().get(usize::from(link.entry)) {
            if !facts.pinned.contains(&link.entry) {
                facts.pinned.push(link.entry);
            }
            facts.last = facts.last.max(Some(entry.last));
        }
        if let Some(step) = step {
            facts.hollow.push(step);
            facts.gated.push(step);
        }
    }
    for code in log.known() {
        if let Some(step) = StepId::from_code(*code)
            && step.quest == quest
        {
            facts.hollow.push(step.step);
        }
    }
    // Npc-level edges count sources (one dialog once), not records.
    let mut sources: Vec<DialogKey> = Vec::new();
    for edge in edges
        .edges
        .iter()
        .filter(|edge| card_of(edge.step.quest) == quest)
    {
        for entry in log
            .entries()
            .iter()
            .filter(|entry| entry.dialog == edge.dialog && entry.record.is_topic())
        {
            match edge.level {
                EdgeLevel::Record if Some(entry.record) == edge.record => {
                    facts.heard += 1;
                    facts.gated.push(edge.step.step);
                }
                EdgeLevel::Npc if evidence == TrackerEvidence::Npc => {
                    if !sources.contains(&entry.dialog) {
                        sources.push(entry.dialog);
                        facts.heard += 1;
                    }
                    facts.gated.push(edge.step.step);
                }
                _ => {}
            }
        }
    }
    facts
}

/// The tracker at `tier` (8.2.7, 8.3.9). `log` is `None` while the clue
/// log is off; the cards then come from the save alone.
pub fn derive_trackers(
    snap: &QuestSnapshot,
    log: Option<&ClueLog>,
    edges: &EdgeTable<'_>,
    tier: SpoilerTier,
    evidence: TrackerEvidence,
    activity: &TrackerActivity,
) -> TrackerView {
    let mut cards = Vec::new();
    for chain in QUEST_CHAINS {
        for quest in chain.quests() {
            let facts = clue_facts(log, edges, quest, evidence);
            if let Some(mut card) = card(snap, chain, quest, &facts, tier) {
                card.recency = activity.last(quest).max(facts.last);
                cards.push(card);
            }
        }
    }
    // Most recent first; the sort is stable, so ties keep the table's
    // order (shrines in virtue order first).
    cards.sort_by_key(|card| std::cmp::Reverse(card.recency));
    TrackerView {
        tier,
        spoilers: tier.is_spoiler(),
        cards,
        clue_log_off: log.is_none(),
    }
}

fn hints(tier: SpoilerTier) -> bool {
    tier >= SpoilerTier::Hints
}

/// The steps a tier may name: done steps and Knowledge steps a later done
/// step implies (tier 1), steps the game gave (tier 2), Knowledge steps
/// with a hollow tick (with save evidence), and every step at tier 3+.
fn steps(
    snap: &QuestSnapshot,
    chain: &ChainSpec,
    quest: QuestId,
    facts: &ClueFacts,
    tier: SpoilerTier,
    named: bool,
) -> Vec<StepLine> {
    let done: Vec<bool> = chain
        .steps
        .iter()
        .map(|spec| match spec.evidence {
            Evidence::Save(test) => test(snap, quest.index),
            Evidence::Knowledge => false,
        })
        .collect();
    chain
        .steps
        .iter()
        .enumerate()
        .filter_map(|(at, spec)| {
            let later_done = done[at + 1..].iter().any(|done| *done);
            // A pin or a mark at any tier; a spec-linked clue from tier 3.
            let hollow = facts.hollow.contains(&spec.step)
                || (hints(tier) && facts.gated.contains(&spec.step));
            let given = tier >= SpoilerTier::Goals
                && spec.given.is_some_and(|given| given(snap, quest.index));
            let shown = hints(tier)
                || (named
                    && (done[at]
                        || given
                        || matches!(spec.evidence, Evidence::Knowledge) && (later_done || hollow)));
            if !shown {
                return None;
            }
            let tick = match spec.evidence {
                Evidence::Save(_) if done[at] => Tick::Done,
                Evidence::Save(_) => Tick::Open,
                Evidence::Knowledge if hollow => Tick::Hollow,
                Evidence::Knowledge => Tick::Unknown,
            };
            Some(StepLine {
                step: StepId::new(quest, spec.step),
                label: spec.label.into(),
                tick,
            })
        })
        .collect()
}

/// A card that passes the shown-when rules at `tier`, or `None`.
fn card(
    snap: &QuestSnapshot,
    chain: &ChainSpec,
    quest: QuestId,
    facts: &ClueFacts,
    tier: SpoilerTier,
) -> Option<TrackerCard> {
    let index = usize::from(quest.index);
    let pinned = !facts.pinned.is_empty() || !facts.hollow.is_empty();
    let full = tier == SpoilerTier::Full;
    let mut lines = Lines::new();
    let (mut card, evidence) = match quest.kind {
        QuestKind::Shrine => {
            let virtue = virtue(quest.index);
            let phase = snap.phase(virtue);
            let condition = snap.condition(virtue);
            let ruined = condition == ShrineCondition::Ruined;
            let evidence = phase != ShrinePhase::NotStarted || condition != ShrineCondition::Intact;
            if !evidence && !pinned && !hints(tier) {
                return None;
            }
            let mut card = TrackerCard::new(quest, virtue.name());
            if ruined {
                card.state = text::RUINED.into();
                card.mark = StatusMark::ALERT;
            } else if phase != ShrinePhase::NotStarted {
                card.state = text::phase_value(phase, SpoilerTier::Progress).into();
                card.mark = StatusMark::for_phase(phase);
            }
            let spirituality = virtue == ShrineVirtue::Spirituality;
            card.cell = text::shrine_next_cell(phase, ruined, spirituality, tier).into();
            if ruined {
                lines.at(SpoilerTier::Progress, text::RUIN_LINE);
                if spirituality {
                    lines.at(SpoilerTier::Hints, text::RUIN_NO_RESTORE);
                } else {
                    lines.at(SpoilerTier::Hints, text::RUIN_RESTORE_GOAL);
                    lines.at(SpoilerTier::Hints, text::RUIN_RESTORE_HINT);
                }
            } else {
                for (at, line) in text::phase_details(phase) {
                    if *at >= SpoilerTier::Goals {
                        lines.at(*at, *line);
                    }
                }
                if phase == ShrinePhase::NotStarted
                    && let Some(hint) = text::mantra_hint(index)
                {
                    lines.at(SpoilerTier::Hints, hint);
                }
            }
            // Gate (a): the shrine accepted the mantra, or the ruin byte
            // is set (restoring needs it; 5.2.7).
            if full && (evidence || facts.gated.contains(&0)) {
                card.answer = Some(TrackerAnswer {
                    label: text::ANSWER_MANTRA,
                    secret: shrine_mantra_for(virtue),
                    step: StepId::new(quest, 0),
                });
            }
            (card, evidence)
        }
        QuestKind::WordOfPower => {
            let scene = FIRST_DUNGEON_SCENE + quest.index;
            let open = snap.seal_open(index);
            let inside = snap.current_scene == scene;
            let evidence = open || inside;
            if !evidence && !hints(tier) {
                return None;
            }
            let mut card = TrackerCard::new(quest, dungeon_resident_name(scene)?);
            // Seals show their current state (S8.7).
            card.state = if open {
                text::UNSEALED
            } else {
                text::SEALED_STATE
            }
            .into();
            card.mark = if open {
                StatusMark::DONE
            } else {
                StatusMark::NONE
            };
            card.cell = text::word_next_cell(open, tier).into();
            if !open {
                lines.at(SpoilerTier::Hints, text::DUNGEON_WORD_GOAL);
                lines.at(SpoilerTier::Hints, text::dungeon_word_hint(index));
            }
            if full
                && (open || facts.gated.contains(&0))
                && let Some(word) = dungeon_word_of_power(scene)
            {
                card.answer = Some(TrackerAnswer {
                    label: text::ANSWER_WORD,
                    secret: word,
                    step: StepId::new(quest, 0),
                });
            }
            (card, evidence)
        }
        QuestKind::Shadowlord => {
            let won = vanquished(snap, quest.index);
            if !won && !hints(tier) {
                return None;
            }
            let name = text::shadowlord_display_name(shadowlord_name_for_slot(index)?);
            let mut card = TrackerCard::new(quest, name);
            if won {
                card.state = text::VANQUISHED.into();
                card.mark = StatusMark::DONE;
            } else {
                lines.at(SpoilerTier::Hints, text::SHADOWLORD_NAME_HINT);
                lines.at(SpoilerTier::Hints, text::SHADOWLORD_FLAME_HINT);
            }
            if full && (won || facts.gated.contains(&0)) {
                card.answer = Some(TrackerAnswer {
                    label: text::ANSWER_FLAME,
                    secret: text::flame_for(index),
                    step: StepId::new(quest, 0),
                });
            }
            (card, won)
        }
        // One summary line for the three shards.
        QuestKind::Shard => {
            if quest.index != 0 {
                return None;
            }
            let held = (0..SHADOWLORD_COUNT as u8)
                .filter(|slot| shard_held(snap, *slot))
                .count();
            if held == 0 && !full {
                return None;
            }
            let mut card = TrackerCard::new(quest, text::SHARDS_LABEL);
            card.boxed = false;
            card.state = if full {
                text::of_total(held, SHADOWLORD_COUNT).into()
            } else {
                held.to_string().into()
            };
            card.notes = (0..SHADOWLORD_COUNT)
                .filter(|slot| shard_held(snap, *slot as u8))
                .map(|slot| JournalText::Static(text::shard_label(slot)))
                .collect();
            (card, held > 0)
        }
        // One line for the three pieces: carried names only below tier 3.
        QuestKind::Regalia => {
            if quest.index != 0 {
                return None;
            }
            let carried: Vec<&str> = (0..3)
                .filter(|piece| regalia_carried(snap, *piece))
                .map(|piece| regalia_label(piece))
                .collect();
            if carried.is_empty() && !hints(tier) {
                return None;
            }
            let mut card = TrackerCard::new(quest, text::REGALIA_LABEL);
            card.boxed = false;
            card.state = carried.join(" ").into();
            if hints(tier) {
                for piece in 0..3 {
                    if !regalia_carried(snap, piece) {
                        for hint in regalia_hints(piece) {
                            lines.at(SpoilerTier::Hints, *hint);
                        }
                    }
                }
            }
            (card, !carried.is_empty())
        }
        QuestKind::SandalwoodBox => {
            let carried = box_carried(snap, 0);
            if !carried && !hints(tier) {
                return None;
            }
            let label = text::REGALIA
                .iter()
                .find(|(_, item, _)| *item == SPECIAL_ITEM_WOODEN_BOX_INDEX)
                .map_or("", |(label, _, hints)| {
                    if !carried {
                        for hint in *hints {
                            lines.at(SpoilerTier::Hints, *hint);
                        }
                    }
                    label
                });
            let mut card = TrackerCard::new(quest, label);
            card.boxed = false;
            card.state = if carried { text::CARRIED } else { "" }.into();
            (card, carried)
        }
        // One line for every tool: only carried tools are ever named
        // (5.2.8, 5).
        QuestKind::Tools => {
            if quest.index != 0 {
                return None;
            }
            let carried: Vec<JournalText> = (0..chain.count)
                .filter(|tool| tool_carried(snap, *tool))
                .map(|tool| JournalText::Static(tool_label(tool)))
                .collect();
            if carried.is_empty() {
                return None;
            }
            let mut card = TrackerCard::new(quest, text::TOOLS_LABEL);
            card.boxed = false;
            card.state = carried.len().to_string().into();
            card.notes = carried;
            (card, true)
        }
        QuestKind::Companions => {
            let names: Vec<JournalText> = companions(snap)
                .map(|companion| JournalText::Owned(companion.name.clone()))
                .collect();
            if names.is_empty() {
                return None;
            }
            let mut card = TrackerCard::new(quest, text::COMPANIONS_LABEL);
            card.boxed = false;
            card.state = names.len().to_string().into();
            card.notes = names;
            (card, true)
        }
        // Doom readiness: tiers 3 and 4 only, as chapter 5's Endgame.
        QuestKind::Doom => {
            if !hints(tier) {
                return None;
            }
            let ready = doom_ready(snap);
            let mut card = TrackerCard::new(quest, text::DOOM_LABEL);
            card.state = if ready {
                text::DOOM_READY
            } else {
                text::DOOM_NOT_READY
            }
            .into();
            card.mark = if ready {
                StatusMark::DONE
            } else {
                StatusMark::NONE
            };
            lines.at(SpoilerTier::Hints, text::ENDGAME_SHADOWLORDS_LINE);
            (card, ready)
        }
        // Knowledge only: a pin or a mark, else tier 3 and above.
        QuestKind::Password => {
            if !pinned && !hints(tier) {
                return None;
            }
            let card = TrackerCard::new(quest, text::password_label(index));
            lines.at(SpoilerTier::Hints, text::password_hint(index));
            // No save evidence: below tier 3 it names no step.
            (card, false)
        }
    };
    // A pinned-only card names no step below tier 3.
    card.steps = steps(snap, chain, quest, facts, tier, evidence || hints(tier));
    card.next = lines.upto(tier);
    card.pinned = facts.pinned.len() as u16;
    // A spec link is never shown or counted below tier 3 (8.2.7).
    card.heard = if hints(tier) { facts.heard } else { 0 };
    Some(card)
}

// --- Evidence links (M6-07). ---------------------------------------------

/// LINKS kinds (8.3.7): a pin to a quest card, or to one of its steps.
pub const LINK_QUEST: u8 = 0;
pub const LINK_STEP: u8 = 1;

/// What a pin or the Clues tab's Quest filter points at: a card, or one
/// of its steps.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LinkTarget {
    pub quest: QuestId,
    pub step: Option<u8>,
}

impl LinkTarget {
    pub const fn quest(quest: QuestId) -> Self {
        Self { quest, step: None }
    }

    pub const fn step(step: StepId) -> Self {
        Self {
            quest: step.quest,
            step: Some(step.step),
        }
    }

    /// The LINKS row pinning `entry` here.
    pub fn link(self, entry: u16) -> Link {
        match self.step {
            None => Link {
                entry,
                kind: LINK_QUEST,
                target: self.quest.code(),
            },
            Some(step) => Link {
                entry,
                kind: LINK_STEP,
                target: StepId::new(self.quest, step).code(),
            },
        }
    }

    /// Whether `link` pins to this target: a step target takes its own
    /// step's pins; a card target takes the card's and all its steps'.
    pub fn covers(self, link: &Link) -> bool {
        match (link.kind, self.step) {
            (LINK_QUEST, None) => QuestId::from_code(link.target) == Some(self.quest),
            (LINK_STEP, step) => StepId::from_code(link.target).is_some_and(|linked| {
                linked.quest == self.quest && step.is_none_or(|step| step == linked.step)
            }),
            _ => false,
        }
    }

    /// Whether `edge` links to this target.
    fn has_edge(self, edge: &ClueEdge) -> bool {
        card_of(edge.step.quest) == self.quest
            && self.step.is_none_or(|step| step == edge.step.step)
    }
}

/// Whether `entry` is a heard clue `edge` links (Record: the record;
/// Npc: any topic of the source, and only under `npc`).
fn edge_hears(edge: &ClueEdge, entry: &ClueEntry, evidence: TrackerEvidence) -> bool {
    entry.dialog == edge.dialog
        && match edge.level {
            EdgeLevel::Record => Some(entry.record) == edge.record,
            EdgeLevel::Npc => evidence == TrackerEvidence::Npc && entry.record.is_topic(),
        }
}

/// The entries the Clues tab keeps under the Quest filter (8.2.7 "Enter
/// on a step"): the player's pins at every tier, and from tier 3 the
/// heard clues the spec's edges link (8.2.8: a clue is listed under a
/// quest below tier 3 only by a player pin). In log order.
pub fn linked_entries(
    log: &ClueLog,
    edges: &EdgeTable<'_>,
    target: LinkTarget,
    tier: SpoilerTier,
    evidence: TrackerEvidence,
) -> Vec<usize> {
    (0..log.entries().len())
        .filter(|index| {
            let entry = &log.entries()[*index];
            let pinned = log
                .links()
                .iter()
                .any(|link| usize::from(link.entry) == *index && target.covers(link));
            pinned
                || (hints(tier)
                    && edges
                        .edges
                        .iter()
                        .any(|edge| target.has_edge(edge) && edge_hears(edge, entry, evidence)))
        })
        .collect()
}

/// One row of the pin picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PinTarget {
    pub target: LinkTarget,
    pub label: JournalText,
    /// 0 a card, 1 a step under it.
    pub depth: u8,
    /// The entry is pinned here already.
    pub linked: bool,
}

/// The cards the pin picker offers at every tier, card or not (8.2.5
/// "works at any tier"): the shrines, by virtue, and the two passwords by
/// role. They name nothing the tier hides: a pin is how such a card comes
/// to exist below tier 3.
fn always_pinnable() -> impl Iterator<Item = (QuestId, JournalText)> {
    let shrines = ShrineVirtue::ALL.iter().enumerate().map(|(at, virtue)| {
        (
            QuestId::new(QuestKind::Shrine, at as u8),
            virtue.name().into(),
        )
    });
    let passwords = (0..chain(QuestKind::Password).count).map(|at| {
        (
            QuestId::new(QuestKind::Password, at),
            text::password_label(usize::from(at)).into(),
        )
    });
    shrines.chain(passwords)
}

/// The pin picker for `entry` (8.2.5 point 2): the cards the view shows,
/// plus [`always_pinnable`], in the table's order, each followed by the
/// Knowledge steps it names at this tier. Nothing the view hides is
/// offered.
pub fn pin_targets(view: &TrackerView, log: &ClueLog, entry: u16) -> Vec<PinTarget> {
    let mut cards: Vec<(QuestId, JournalText, Vec<&StepLine>)> = view
        .cards
        .iter()
        .map(|card| (card.quest, card.title.clone(), card.steps.iter().collect()))
        .collect();
    for (quest, title) in always_pinnable() {
        if !cards.iter().any(|(shown, _, _)| *shown == quest) {
            cards.push((quest, title, Vec::new()));
        }
    }
    cards.sort_by_key(|(quest, _, _)| *quest);
    let linked = |target: LinkTarget| log.links().contains(&target.link(entry));
    let mut rows = Vec::new();
    for (quest, title, steps) in cards {
        let target = LinkTarget::quest(quest);
        rows.push(PinTarget {
            target,
            label: title,
            depth: 0,
            linked: linked(target),
        });
        let knowledge = |line: &&StepLine| {
            chain(quest.kind).steps.iter().any(|spec| {
                spec.step == line.step.step && matches!(spec.evidence, Evidence::Knowledge)
            })
        };
        for line in steps.into_iter().filter(knowledge) {
            let target = LinkTarget::step(line.step);
            rows.push(PinTarget {
                target,
                label: line.label.clone(),
                depth: 1,
                linked: linked(target),
            });
        }
    }
    rows
}

/// The detail lines of a clue's links (8.2.3 detail pane): "Pinned to:"
/// the cards the view shows that the player pinned it to, and from tier
/// 3 the spec link's level. A card the view hides is never named.
pub fn link_lines(
    log: &ClueLog,
    edges: &EdgeTable<'_>,
    view: &TrackerView,
    index: usize,
    evidence: TrackerEvidence,
) -> Vec<String> {
    let mut lines = Vec::new();
    let Some(entry) = log.entries().get(index) else {
        return lines;
    };
    let mut titles: Vec<&str> = Vec::new();
    for link in log
        .links()
        .iter()
        .filter(|link| usize::from(link.entry) == index)
    {
        let quest = match link.kind {
            LINK_QUEST => QuestId::from_code(link.target),
            LINK_STEP => StepId::from_code(link.target).map(|step| step.quest),
            _ => None,
        };
        if let Some(card) = quest.and_then(|quest| view.card(card_of(quest)))
            && !titles.contains(&card.title.as_str())
        {
            titles.push(card.title.as_str());
        }
    }
    if !titles.is_empty() {
        lines.push(format!("{}{}", text::PINNED_TO, titles.join(", ")));
    }
    if hints(view.tier) {
        let level = edges
            .edges
            .iter()
            .filter(|edge| edge_hears(edge, entry, evidence))
            .map(|edge| edge.level)
            .min_by_key(|level| *level != EdgeLevel::Record);
        match level {
            Some(EdgeLevel::Record) => lines.push(text::LINK_RECORD.to_string()),
            Some(EdgeLevel::Npc) => lines.push(text::LINK_NPC.to_string()),
            None => {}
        }
    }
    lines
}

fn regalia_label(piece: u8) -> &'static str {
    let item = regalia_index(piece);
    text::REGALIA
        .iter()
        .find(|(_, index, _)| *index == item)
        .map_or("", |(label, _, _)| label)
}

fn regalia_hints(piece: u8) -> &'static [&'static str] {
    let item = regalia_index(piece);
    text::REGALIA
        .iter()
        .find(|(_, index, _)| *index == item)
        .map_or(&[], |(_, _, hints)| hints)
}

/// The side panel's tracker lines (8.2.7), never above tier 2: the shrine
/// cards with a goal the game gave, grouped under it, then the shards held
/// and the Glass Sword. Each line fits `columns`.
pub fn panel_card_lines(snap: &QuestSnapshot, columns: usize) -> Vec<String> {
    let view = derive_trackers(
        snap,
        None,
        &EdgeTable::EMPTY,
        SpoilerTier::Goals,
        TrackerEvidence::Record,
        &TrackerActivity::default(),
    );
    let mut lines = Vec::new();
    let codex: Vec<&TrackerCard> = view
        .cards
        .iter()
        .filter(|card| card.quest.kind == QuestKind::Shrine && !card.cell.is_empty())
        .collect();
    if let Some(first) = codex.first() {
        lines.push(format!("{}:", first.cell));
        for card in &codex {
            lines.push(format!(" {}", card.title));
        }
    }
    if let Some(shards) = view.card(QuestId::new(QuestKind::Shard, 0)) {
        lines.push(format!("{} {}", text::SHARDS_LABEL, shards.state));
    }
    if snap.glass_sword != 0 {
        lines.push(text::GLASS_SWORD.to_string());
    }
    for line in &mut lines {
        *line = line.chars().take(columns).collect();
    }
    lines
}

#[cfg(test)]
#[path = "trackers_tests.rs"]
mod tests;

//! The quest journal's read-only progress model (Phase B design 5.3.2;
//! build plan M5-01).
//!
//! Spec: `systems/karma.md` §7, §7.1, §10; `formats/saved-gam.md` §3.1,
//! §3.3, §7, §9.1, §9.2; `catalogs/quest-graph.md` §4-§8;
//! `systems/endgame.md` §2.
//!
//! A [`QuestSnapshot`] copies the few save fields the journal reads, from a
//! [`PlayState`], a [`PlayOptions`] or SAVED.GAM bytes. It reduces
//! sensitive values while it is built: a Shadowlord's hideout town becomes a
//! [`ShadowlordState`], so the town never reaches the model or a renderer
//! (D5.4). [`derive`] turns a snapshot into a [`JournalModel`] at one
//! [`SpoilerTier`].
//!
//! **The discovery filter is in [`derive`]** (U17, foundation 1.4.7). At
//! tiers 1 and 2 a row exists only with save evidence of the player's own
//! action, a section with no row is left out, and no line gives a total of
//! things not found. The eight virtues are the one exception, because
//! character creation names them all. Tiers 3 and 4 are the labelled
//! spoiler tiers; tier 4 answers still need evidence.
//!
//! [`derive`] is monotone in the tier: each row at tier *t* is in the model
//! at tier *t+1*, with its detail lines a prefix of the lines there, and a
//! `secret` appears only at tier 4.
//!
//! Everything here is pure: no random numbers, no files, no `DiskSession`,
//! and nothing writes the state it reads. Nothing in the game calls it; the
//! shell, the offline report and the tests do.

pub mod text;

use std::collections::HashMap;
use std::io;

use crate::enhancements::journal::SpoilerTier;
use crate::shrine_virtue::{ShrineQuestState, ShrineVirtue};
use crate::{
    Area, InnGuestRecord, PlayOptions, PlayState, PlayTarget, SAVE_CHARACTER_NAME_LEN,
    SAVE_SHRINE_RUIN_FLAG_COUNT, SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT, SAVED_GAM_FILENAME,
    SHADOWLORD_COUNT, SHADOWLORD_HIDEOUT_MAX, SHADOWLORD_HIDEOUT_MIN, SHADOWLORD_VANQUISHED,
    SPECIAL_ITEM_COUNT, SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX, SPECIAL_ITEM_WOODEN_BOX_INDEX,
    dungeon_resident_name, dungeon_word_of_power, shadowlord_name_for_slot, shrine_mantra_for,
    town_resident_name,
};

use text::ToolSource;

/// A virtue's shrine quest phase (`systems/karma.md` §10). The engine's
/// existing [`ShrineQuestState`] under the design's name.
pub type ShrinePhase = ShrineQuestState;

/// The scene byte of the first stock dungeon, Deceit; seal and dungeon
/// record `i` is scene `33 + i` (`formats/saved-gam.md` §9.1).
const FIRST_DUNGEON_SCENE: u8 = 33;
/// The seal byte's high bit: the entrance has been opened.
const SEAL_OPEN_BIT: u8 = 0x80;
/// The ruin byte's high bit: the shrine is ruined.
const RUIN_BIT: u8 = 0x80;
/// The last town-family scene byte (`catalogs/gazetteer.md` §5).
const LAST_TOWN_SCENE: u8 = 32;

/// One bit per virtue in the two shrine masks.
pub const fn virtue_bit(virtue: ShrineVirtue) -> u8 {
    1 << (virtue as u8)
}

/// Pure derive: the two mask bytes give the phase (karma.md §10 table).
pub const fn shrine_phase(ordained_mask: u8, codex_mask: u8, virtue: ShrineVirtue) -> ShrinePhase {
    let bit = virtue_bit(virtue);
    ShrineVirtue::shrine_quest_state(ordained_mask & bit != 0, codex_mask & bit != 0)
}

/// A shrine's condition from its ruin byte (`formats/saved-gam.md` §9.1;
/// `systems/karma.md` §7.1: restoration clears only the high bit).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShrineCondition {
    Intact,
    Ruined,
    Restored,
}

pub const fn shrine_condition(ruin_byte: u8) -> ShrineCondition {
    if ruin_byte & RUIN_BIT != 0 {
        ShrineCondition::Ruined
    } else if ruin_byte != 0 {
        ShrineCondition::Restored
    } else {
        ShrineCondition::Intact
    }
}

/// A Shadowlord's slot, reduced: the hideout town is not kept (D5.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ShadowlordState {
    /// Slot 0.
    NotPlaced,
    /// Slot 1..=8. Not evidence that the player has met it.
    AtLarge,
    /// Slot `0xFF`.
    Vanquished,
    /// Any other value, outside `catalogs/quest-graph.md` §5's set.
    Unknown,
}

impl ShadowlordState {
    pub const fn from_slot(byte: u8) -> Self {
        if byte == 0 {
            ShadowlordState::NotPlaced
        } else if byte >= SHADOWLORD_HIDEOUT_MIN && byte <= SHADOWLORD_HIDEOUT_MAX {
            ShadowlordState::AtLarge
        } else if byte == SHADOWLORD_VANQUISHED {
            ShadowlordState::Vanquished
        } else {
            ShadowlordState::Unknown
        }
    }
}

/// Where a companion the journal may name is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CompanionPlace {
    /// Travelling in the party.
    Party,
    /// Lodged at the inn of this scene (the record's inn marker,
    /// `formats/saved-gam.md` §3.3).
    Lodged { scene: u8 },
    /// The marker holds `0x7F`, Blackthorn's execution write
    /// (`systems/blackthorn.md` §5).
    Executed,
}

/// A party member, or a roster record outside the party whose inn marker
/// only the player's own lodging (or the execution) writes. Never an
/// unrecruited record.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CompanionFact {
    pub name: String,
    pub place: CompanionPlace,
}

/// The virtue header's counts (5.2.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct VirtueSummary {
    /// Virtues in phase Complete.
    pub complete: u8,
    /// Virtues Ordained or Codex read.
    pub in_progress: u8,
    /// Shrines whose ruin byte has its high bit set.
    pub ruined: u8,
}

/// The save facts the journal reads, reduced (5.3.2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestSnapshot {
    pub ordained_mask: u8,
    pub codex_mask: u8,
    pub ruin: [u8; SAVE_SHRINE_RUIN_FLAG_COUNT],
    pub seals: [u8; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
    pub shadowlords: [ShadowlordState; SHADOWLORD_COUNT],
    pub special_items: [u8; SPECIAL_ITEM_COUNT],
    pub climbing_gear: u8,
    pub moral_standing: u8,
    /// Party members first, in party order, then the records outside the
    /// party whose inn marker is set, in roster order.
    pub companions: Vec<CompanionFact>,
    /// (town scene byte, count of name-known bits), count above zero only,
    /// by scene byte.
    pub introduced: Vec<(u8, u8)>,
    /// The scene the party is in now (a dungeon row's evidence).
    pub current_scene: u8,
}

/// The fields both in-memory sources share.
struct Source<'a> {
    ordained_mask: u8,
    codex_mask: u8,
    ruin: [u8; SAVE_SHRINE_RUIN_FLAG_COUNT],
    seals: [u8; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
    hideouts: [u8; SHADOWLORD_COUNT],
    special_items: [u8; SPECIAL_ITEM_COUNT],
    climbing_gear: u8,
    moral_standing: u8,
    party_len: usize,
    party_names: &'a [[u8; SAVE_CHARACTER_NAME_LEN]],
    inn_registry: &'a [InnGuestRecord],
    name_known: &'a HashMap<u8, u32>,
    current_scene: u8,
}

impl QuestSnapshot {
    /// The live game's facts. Reads only; the state is not changed.
    pub fn from_play_state(state: &PlayState) -> Self {
        Self::build(Source {
            ordained_mask: state.shrine_ordained_mask,
            codex_mask: state.shrine_codex_mask,
            ruin: state.shrine_ruin_flags,
            seals: state.word_of_power_seal_flags,
            hideouts: state.shadowlord_hideouts,
            special_items: state.special_items,
            climbing_gear: state.climbing_gear,
            moral_standing: state.moral_standing,
            party_len: state.party.len(),
            party_names: &state.party_names,
            inn_registry: &state.inn_registry,
            name_known: &state.talk_branch_flags,
            current_scene: match state.area {
                Area::Town { scene, .. } => scene.byte,
                Area::Dungeon { scene, .. } => scene.byte,
                Area::World { .. } => 0,
            },
        })
    }

    /// The facts of a decoded save, before any scene is loaded.
    pub fn from_play_options(options: &PlayOptions) -> Self {
        Self::build(Source {
            ordained_mask: options.shrine_ordained_mask,
            codex_mask: options.shrine_codex_mask,
            ruin: options.shrine_ruin_flags,
            seals: options.word_of_power_seal_flags,
            hideouts: options.shadowlord_hideouts,
            special_items: options.special_items,
            climbing_gear: options.climbing_gear,
            moral_standing: options.moral_standing,
            party_len: options.party.len(),
            party_names: &options.party_names,
            inn_registry: &options.inn_registry,
            name_known: &options.talk_branch_flags,
            current_scene: match options.target {
                PlayTarget::Town(scene) => scene.byte,
                PlayTarget::Dungeon(scene) => scene.byte,
                PlayTarget::World(_) => 0,
            },
        })
    }

    /// SAVED.GAM bytes only, through the game's own decode
    /// (`play_options_from_save_bytes_named`). It opens no file, writes no
    /// mirror and never reads SAVED.WPS (5.3.3).
    pub fn from_save_bytes(bytes: &[u8]) -> io::Result<Self> {
        let options = crate::play_options_from_save_bytes_named(
            bytes,
            SAVED_GAM_FILENAME,
            "--quest-journal-report",
            false,
        )?;
        Ok(Self::from_play_options(&options))
    }

    fn build(source: Source<'_>) -> Self {
        let mut companions: Vec<CompanionFact> = source
            .party_names
            .iter()
            .take(source.party_len)
            .map(|name| CompanionFact {
                name: decode_name(name),
                place: CompanionPlace::Party,
            })
            .collect();
        companions.extend(source.inn_registry.iter().map(|guest| CompanionFact {
            name: decode_name(&guest.name),
            place: if guest.scene_marker == crate::karma::INTERRED_WHEREABOUTS {
                CompanionPlace::Executed
            } else {
                CompanionPlace::Lodged {
                    scene: guest.scene_marker,
                }
            },
        }));
        let mut introduced: Vec<(u8, u8)> = source
            .name_known
            .iter()
            .filter(|(scene, _)| (1..=LAST_TOWN_SCENE).contains(*scene))
            .map(|(scene, bits)| (*scene, bits.count_ones() as u8))
            .filter(|(_, count)| *count > 0)
            .collect();
        introduced.sort_unstable();
        QuestSnapshot {
            ordained_mask: source.ordained_mask,
            codex_mask: source.codex_mask,
            ruin: source.ruin,
            seals: source.seals,
            shadowlords: source.hideouts.map(ShadowlordState::from_slot),
            special_items: source.special_items,
            climbing_gear: source.climbing_gear,
            moral_standing: source.moral_standing,
            companions,
            introduced,
            current_scene: source.current_scene,
        }
    }

    pub fn phase(&self, virtue: ShrineVirtue) -> ShrinePhase {
        shrine_phase(self.ordained_mask, self.codex_mask, virtue)
    }

    pub fn condition(&self, virtue: ShrineVirtue) -> ShrineCondition {
        shrine_condition(self.ruin[virtue.index()])
    }

    /// Whether seal `record` reads Opened (its high bit).
    pub fn seal_open(&self, record: usize) -> bool {
        self.seals[record] & SEAL_OPEN_BIT != 0
    }

    pub fn virtue_summary(&self) -> VirtueSummary {
        let mut summary = VirtueSummary::default();
        for virtue in ShrineVirtue::ALL {
            match self.phase(virtue) {
                ShrinePhase::Complete => summary.complete += 1,
                ShrinePhase::Ordained | ShrinePhase::CodexRead => summary.in_progress += 1,
                ShrinePhase::NotStarted => {}
            }
            if self.condition(virtue) == ShrineCondition::Ruined {
                summary.ruined += 1;
            }
        }
        summary
    }
}

/// A save name: the bytes before the first NUL, printable ASCII only.
fn decode_name(bytes: &[u8]) -> String {
    let name: String = bytes
        .iter()
        .take_while(|byte| **byte != 0)
        .filter(|byte| byte.is_ascii_graphic() || **byte == b' ')
        .map(|byte| char::from(*byte))
        .collect();
    let name = name.trim();
    if name.is_empty() {
        text::UNNAMED.to_string()
    } else {
        name.to_string()
    }
}

// --- The model. ----------------------------------------------------------

/// The page's sections, in the fixed cycle order (5.2.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SectionId {
    Virtues,
    Dungeons,
    Shadowlords,
    Regalia,
    Items,
    People,
    Endgame,
}

impl SectionId {
    pub const ALL: [SectionId; 7] = [
        SectionId::Virtues,
        SectionId::Dungeons,
        SectionId::Shadowlords,
        SectionId::Regalia,
        SectionId::Items,
        SectionId::People,
        SectionId::Endgame,
    ];

    pub const fn title(self) -> &'static str {
        text::section_title(self)
    }
}

/// A row's identity, the same at every tier (its label may change, as a
/// shard row gains its Shadowlord's name at tier 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowKey {
    Virtue(u8),
    Dungeon(u8),
    Shadowlord(u8),
    /// Regalia by special item index.
    Regalia(u8),
    /// Tools by their place in [`text::TOOLS`].
    Item(u8),
    /// Companions by their place in [`QuestSnapshot::companions`].
    Companion(u8),
    /// The introductions total.
    Introduced,
    /// Introductions in one town, by scene byte.
    IntroducedIn(u8),
    Karma,
    Endgame(u8),
}

/// A marker character with its EGA palette index, so colour is data and is
/// never the only cue (5.2.5).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct StatusMark {
    pub glyph: char,
    pub colour: u8,
}

impl StatusMark {
    /// Not begun, or nothing to report: dark grey `-`.
    pub const NONE: Self = Self::new('-', 8);
    /// Ordained, go and read the Codex: yellow `>`.
    pub const ORDAINED: Self = Self::new('>', 14);
    /// Codex read: light cyan `<`.
    pub const CODEX_READ: Self = Self::new('<', 11);
    /// Complete, opened, vanquished, carried, done: light green `*`.
    pub const DONE: Self = Self::new('*', 10);
    /// Ruined, executed: light red `!`.
    pub const ALERT: Self = Self::new('!', 12);
    /// A restored shrine's small `r`, after the phase marker.
    pub const RESTORED: Self = Self::new('r', 10);
    /// A slot outside the spec's values: greyed `?`.
    pub const UNKNOWN: Self = Self::new('?', 8);
    /// Plain body text (light grey), no marker.
    pub const BODY: Self = Self::new(' ', 7);

    pub const fn new(glyph: char, colour: u8) -> Self {
        Self { glyph, colour }
    }

    pub const fn for_phase(phase: ShrinePhase) -> Self {
        match phase {
            ShrinePhase::NotStarted => Self::NONE,
            ShrinePhase::Ordained => Self::ORDAINED,
            ShrinePhase::CodexRead => Self::CODEX_READ,
            ShrinePhase::Complete => Self::DONE,
        }
    }
}

/// A piece of journal wording: a line of [`text`], or a composed line that
/// holds only names already in the save or in the spec's catalogues.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum JournalText {
    Static(&'static str),
    Owned(String),
}

impl JournalText {
    pub fn as_str(&self) -> &str {
        match self {
            JournalText::Static(text) => text,
            JournalText::Owned(text) => text,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }
}

impl From<&'static str> for JournalText {
    fn from(text: &'static str) -> Self {
        JournalText::Static(text)
    }
}

impl From<String> for JournalText {
    fn from(text: String) -> Self {
        JournalText::Owned(text)
    }
}

impl std::fmt::Display for JournalText {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One row of a section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRow {
    pub key: RowKey,
    pub label: JournalText,
    pub value: JournalText,
    pub mark: StatusMark,
    /// A small mark after `mark`: a restored shrine's `r`.
    pub suffix: Option<StatusMark>,
    /// A second state shown beside the value: a ruined shrine, or a
    /// Shadowlord's shard.
    pub note: Option<(StatusMark, JournalText)>,
    /// The selected row's detail lines, lowest tier first.
    pub detail: Vec<JournalText>,
    /// A tier 4 answer that passed the evidence rule: a mantra, a Word of
    /// Power or a flame. `None` below tier 4.
    pub secret: Option<&'static str>,
}

impl JournalRow {
    fn new(key: RowKey, label: impl Into<JournalText>, value: impl Into<JournalText>) -> Self {
        Self {
            key,
            label: label.into(),
            value: value.into(),
            mark: StatusMark::BODY,
            suffix: None,
            note: None,
            detail: Vec::new(),
            secret: None,
        }
    }

    fn mark(mut self, mark: StatusMark) -> Self {
        self.mark = mark;
        self
    }

    /// Every string of the row, for the spoiler and wording tests.
    pub fn strings(&self) -> impl Iterator<Item = &str> {
        [self.label.as_str(), self.value.as_str()]
            .into_iter()
            .chain(self.note.iter().map(|(_, text)| text.as_str()))
            .chain(self.detail.iter().map(JournalText::as_str))
            .chain(self.secret)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalSection {
    pub id: SectionId,
    pub rows: Vec<JournalRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalModel {
    pub tier: SpoilerTier,
    pub summary: VirtueSummary,
    /// The summary header line (5.2.5).
    pub header: String,
    /// Only sections with a row, in [`SectionId::ALL`] order.
    pub sections: Vec<JournalSection>,
    /// True at tiers 3 and 4: the title row reads SPOILERS.
    pub spoilers: bool,
}

impl JournalModel {
    pub fn section(&self, id: SectionId) -> Option<&JournalSection> {
        self.sections.iter().find(|section| section.id == id)
    }

    /// Every string of the model, for the spoiler and wording tests.
    pub fn strings(&self) -> impl Iterator<Item = &str> {
        std::iter::once(self.header.as_str()).chain(
            self.sections
                .iter()
                .flat_map(|section| section.rows.iter().flat_map(JournalRow::strings)),
        )
    }
}

/// Detail lines tagged with the lowest tier that shows each; kept in tier
/// order, so each tier's lines are a prefix of the next tier's.
struct Lines(Vec<(SpoilerTier, JournalText)>);

impl Lines {
    fn new() -> Self {
        Lines(Vec::new())
    }

    fn at(&mut self, tier: SpoilerTier, line: impl Into<JournalText>) {
        self.0.push((tier, line.into()));
    }

    fn upto(mut self, tier: SpoilerTier) -> Vec<JournalText> {
        self.0.sort_by_key(|(at, _)| *at);
        self.0
            .into_iter()
            .filter(|(at, _)| *at <= tier)
            .map(|(_, line)| line)
            .collect()
    }
}

/// The journal at `tier` (5.2.4-5.2.8), with the discovery filter at tiers
/// 1 and 2.
pub fn derive(snap: &QuestSnapshot, tier: SpoilerTier) -> JournalModel {
    let summary = snap.virtue_summary();
    let sections = [
        (SectionId::Virtues, virtue_rows(snap, tier)),
        (SectionId::Dungeons, dungeon_rows(snap, tier)),
        (SectionId::Shadowlords, shadowlord_rows(snap, tier)),
        (SectionId::Regalia, regalia_rows(snap, tier)),
        (SectionId::Items, item_rows(snap)),
        (SectionId::People, people_rows(snap)),
        (SectionId::Endgame, endgame_rows(snap, tier)),
    ]
    .into_iter()
    .filter(|(_, rows)| !rows.is_empty())
    .map(|(id, rows)| JournalSection { id, rows })
    .collect();
    JournalModel {
        tier,
        summary,
        header: text::summary_line(summary),
        sections,
        spoilers: tier.is_spoiler(),
    }
}

fn full(tier: SpoilerTier) -> bool {
    tier == SpoilerTier::Full
}

fn virtue_rows(snap: &QuestSnapshot, tier: SpoilerTier) -> Vec<JournalRow> {
    ShrineVirtue::ALL
        .into_iter()
        .map(|virtue| {
            let index = virtue.index();
            let phase = snap.phase(virtue);
            let condition = snap.condition(virtue);
            let mut lines = Lines::new();
            // A ruined shrine cannot be meditated at until it is restored,
            // so a virtue not begun there gets the ruin lines only.
            let begin_hints =
                phase != ShrinePhase::NotStarted || condition != ShrineCondition::Ruined;
            if begin_hints {
                for (at, line) in text::phase_details(phase) {
                    lines.at(*at, *line);
                }
                if phase == ShrinePhase::NotStarted
                    && let Some(hint) = text::mantra_hint(index)
                {
                    lines.at(SpoilerTier::Hints, hint);
                }
            }
            match condition {
                ShrineCondition::Intact => {}
                ShrineCondition::Restored => lines.at(SpoilerTier::Progress, text::RESTORED_LINE),
                ShrineCondition::Ruined => {
                    lines.at(SpoilerTier::Progress, text::RUIN_LINE);
                    if virtue == ShrineVirtue::Spirituality {
                        lines.at(SpoilerTier::Hints, text::RUIN_NO_RESTORE);
                    } else {
                        lines.at(SpoilerTier::Hints, text::RUIN_RESTORE_GOAL);
                        lines.at(SpoilerTier::Hints, text::RUIN_RESTORE_HINT);
                        // karma.md §7.1 pairs virtue i with seal i's word;
                        // it is shown only once that seal is open.
                        if snap.seal_open(index)
                            && let Some(word) =
                                dungeon_word_of_power(FIRST_DUNGEON_SCENE + index as u8)
                        {
                            lines.at(SpoilerTier::Full, text::ruin_restore_method(word));
                        }
                    }
                }
            }
            // A mantra is known once the shrine accepted it (any phase but
            // Not begun), or once the ruin byte is non-zero: restoring needs
            // it, and Blackthorn's ruin follows giving it to him (5.2.7).
            let known = phase != ShrinePhase::NotStarted || condition != ShrineCondition::Intact;
            let mut row = JournalRow::new(
                RowKey::Virtue(index as u8),
                virtue.name(),
                text::phase_value(phase, tier),
            )
            .mark(StatusMark::for_phase(phase));
            row.suffix = (condition == ShrineCondition::Restored).then_some(StatusMark::RESTORED);
            row.note = (condition == ShrineCondition::Ruined)
                .then_some((StatusMark::ALERT, JournalText::Static(text::RUINED)));
            row.detail = lines.upto(tier);
            row.secret = (full(tier) && known).then(|| shrine_mantra_for(virtue));
            row
        })
        .collect()
}

fn dungeon_rows(snap: &QuestSnapshot, tier: SpoilerTier) -> Vec<JournalRow> {
    (0..SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT)
        .filter_map(|record| {
            let scene = FIRST_DUNGEON_SCENE + record as u8;
            let opened = snap.seal_open(record);
            let inside = snap.current_scene == scene;
            // Opened proves the player found the entrance and spoke there;
            // being inside proves they found it.
            if tier <= SpoilerTier::Goals && !opened && !inside {
                return None;
            }
            let name = dungeon_resident_name(scene)?;
            let mut lines = Lines::new();
            if opened {
                lines.at(SpoilerTier::Progress, text::DUNGEON_OPENED_LINE);
            } else {
                if inside {
                    lines.at(SpoilerTier::Progress, text::DUNGEON_INSIDE_LINE);
                }
                lines.at(SpoilerTier::Hints, text::DUNGEON_WORD_GOAL);
                lines.at(SpoilerTier::Hints, text::dungeon_word_hint(record));
            }
            let (value, mark) = if opened {
                (text::OPENED, StatusMark::DONE)
            } else {
                (text::SEALED, StatusMark::NONE)
            };
            let mut row = JournalRow::new(RowKey::Dungeon(record as u8), name, value).mark(mark);
            row.detail = lines.upto(tier);
            // Speaking a word again re-seals, so a re-sealed dungeon hides
            // its word: that errs on the side of not spoiling.
            row.secret = if full(tier) && opened {
                dungeon_word_of_power(scene)
            } else {
                None
            };
            Some(row)
        })
        .collect()
}

fn shadowlord_rows(snap: &QuestSnapshot, tier: SpoilerTier) -> Vec<JournalRow> {
    (0..SHADOWLORD_COUNT)
        .filter_map(|slot| {
            let state = snap.shadowlords[slot];
            let vanquished = state == ShadowlordState::Vanquished;
            let shard = snap.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX + slot] != 0;
            // A slot of 1..=8 is not evidence of a meeting; the hideout is
            // not even in the snapshot.
            if tier <= SpoilerTier::Goals && !vanquished && !shard {
                return None;
            }
            let label: JournalText = if vanquished || tier >= SpoilerTier::Hints {
                text::shadowlord_display_name(shadowlord_name_for_slot(slot)?).into()
            } else {
                text::shard_label(slot).into()
            };
            let (value, mark) = match state {
                ShadowlordState::Vanquished => (text::VANQUISHED, StatusMark::DONE),
                ShadowlordState::Unknown if tier >= SpoilerTier::Hints => {
                    (text::UNKNOWN, StatusMark::UNKNOWN)
                }
                _ => ("", StatusMark::NONE),
            };
            let mut lines = Lines::new();
            let note = if shard {
                lines.at(SpoilerTier::Progress, text::SHARD_CARRIED_LINE);
                Some((StatusMark::BODY, JournalText::Static(text::SHARD_CARRIED)))
            } else if vanquished {
                lines.at(SpoilerTier::Progress, text::SHARD_DESTROYED_LINE);
                Some((StatusMark::BODY, JournalText::Static(text::SHARD_DESTROYED)))
            } else {
                None
            };
            if !vanquished {
                lines.at(SpoilerTier::Hints, text::SHADOWLORD_NAME_HINT);
                lines.at(SpoilerTier::Hints, text::SHADOWLORD_FLAME_HINT);
            }
            let mut row = JournalRow::new(RowKey::Shadowlord(slot as u8), label, value).mark(mark);
            row.note = note;
            row.detail = lines.upto(tier);
            row.secret = (full(tier) && vanquished).then(|| text::flame_for(slot));
            Some(row)
        })
        .collect()
}

fn regalia_rows(snap: &QuestSnapshot, tier: SpoilerTier) -> Vec<JournalRow> {
    text::REGALIA
        .iter()
        .filter_map(|(label, index, hints)| {
            let carried = snap.special_items[*index] != 0;
            if !carried && tier <= SpoilerTier::Goals {
                return None;
            }
            let mut lines = Lines::new();
            if !carried {
                for hint in *hints {
                    lines.at(SpoilerTier::Hints, *hint);
                }
            }
            let mut row = if carried {
                JournalRow::new(RowKey::Regalia(*index as u8), *label, text::CARRIED)
                    .mark(StatusMark::DONE)
            } else {
                JournalRow::new(RowKey::Regalia(*index as u8), *label, "").mark(StatusMark::NONE)
            };
            row.detail = lines.upto(tier);
            Some(row)
        })
        .collect()
}

/// Tools are listed only while carried, at every tier: the list itself
/// would name things not found (5.2.8, 5).
fn item_rows(snap: &QuestSnapshot) -> Vec<JournalRow> {
    text::TOOLS
        .iter()
        .enumerate()
        .filter(|(_, (_, source))| match source {
            ToolSource::Special(index) => snap.special_items[*index] != 0,
            ToolSource::ClimbingGear => snap.climbing_gear != 0,
        })
        .map(|(place, (label, _))| {
            JournalRow::new(RowKey::Item(place as u8), *label, text::CARRIED).mark(StatusMark::DONE)
        })
        .collect()
}

fn people_rows(snap: &QuestSnapshot) -> Vec<JournalRow> {
    let mut rows: Vec<JournalRow> = snap
        .companions
        .iter()
        .enumerate()
        .map(|(place, companion)| {
            let key = RowKey::Companion(place as u8);
            let name = companion.name.clone();
            match companion.place {
                CompanionPlace::Party => JournalRow::new(key, name, text::IN_PARTY),
                CompanionPlace::Lodged { scene } => {
                    let town = town_resident_name(scene).unwrap_or(text::AN_INN);
                    let mut row = JournalRow::new(key, name, text::LODGED);
                    row.detail = vec![text::lodged_line(town).into()];
                    row
                }
                CompanionPlace::Executed => {
                    JournalRow::new(key, name, text::EXECUTED).mark(StatusMark::ALERT)
                }
            }
        })
        .collect();
    let total: u32 = snap
        .introduced
        .iter()
        .map(|(_, count)| u32::from(*count))
        .sum();
    if total > 0 {
        rows.push(JournalRow::new(
            RowKey::Introduced,
            text::INTRODUCED_LABEL,
            text::people_count(total),
        ));
        for (scene, count) in &snap.introduced {
            rows.push(JournalRow::new(
                RowKey::IntroducedIn(*scene),
                town_resident_name(*scene).unwrap_or(text::ANOTHER_PLACE),
                count.to_string(),
            ));
        }
    }
    rows.push(JournalRow::new(
        RowKey::Karma,
        text::KARMA_LABEL,
        snap.moral_standing.to_string(),
    ));
    rows
}

/// The readiness checklist of `systems/endgame.md` §2. It exists only at
/// tiers 3 and 4: even unlabelled ticks would give its count (5.2.8, 7).
fn endgame_rows(snap: &QuestSnapshot, tier: SpoilerTier) -> Vec<JournalRow> {
    if tier <= SpoilerTier::Goals {
        return Vec::new();
    }
    let doom = SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT - 1;
    let gates: [(&str, bool, &[&str]); 3] = [
        (
            text::ENDGAME_SHADOWLORDS,
            snap.shadowlords
                .iter()
                .all(|state| *state == ShadowlordState::Vanquished),
            &[text::ENDGAME_SHADOWLORDS_LINE],
        ),
        (
            text::ENDGAME_DOOM,
            snap.seal_open(doom),
            &[text::ENDGAME_DOOM_LINE, text::ENDGAME_AMULET_LINE],
        ),
        (
            text::ENDGAME_BOX,
            snap.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0,
            &[text::ENDGAME_BOX_LINE],
        ),
    ];
    gates
        .into_iter()
        .enumerate()
        .map(|(place, (label, done, lines))| {
            let (value, mark) = if done {
                (text::DONE, StatusMark::DONE)
            } else {
                ("", StatusMark::NONE)
            };
            let mut row = JournalRow::new(RowKey::Endgame(place as u8), label, value).mark(mark);
            row.detail = lines.iter().map(|line| JournalText::Static(line)).collect();
            row
        })
        .collect()
}

#[cfg(test)]
mod tests;

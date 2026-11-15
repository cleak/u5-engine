//! The `[journal]` settings as the engine sees them (Phase B design 5.3.2,
//! 5.4.1; build plan M5-01).
//!
//! The rows are in the catalogue ([`super::catalogue`]): the two toggles
//! `journal.page` and `journal.side_panel`, which are independent, and three
//! valued sub-options. This module holds their word lists, the setting types
//! and the typed view [`ResolvedEnhancements::journal`]. It is plain data:
//! no serde, no Bevy, no paths.
//!
//! Every key is effect class (a). Spoiler tiers 3 and 4 are the labelled
//! spoiler tiers of the no-spoiler rule (foundation 1.4.7): deviation tier
//! and in no preset. The side panel never shows anything above tier 2,
//! whatever the page's tier (5.2.10).

use super::catalogue::Toggle;
use super::resolve::ResolvedEnhancements;

pub const PAGE_KEY: &str = "journal.page";
pub const SPOILER_TIER_KEY: &str = "journal.spoiler_tier";
pub const MANTRAS_KEY: &str = "journal.mantras";
pub const SIDE_PANEL_KEY: &str = "journal.side_panel";
pub const SIDE_PANEL_SIDE_KEY: &str = "journal.side_panel_side";

/// Chapter 5's five `[journal]` keys, in catalogue order of 5.4.1.
pub const JOURNAL_KEYS: &[&str] = &[
    PAGE_KEY,
    SPOILER_TIER_KEY,
    MANTRAS_KEY,
    SIDE_PANEL_KEY,
    SIDE_PANEL_SIDE_KEY,
];

/// The words of `journal.mantras`, in [`MantraDisplay`] order.
pub const MANTRA_WORDS: &[&str] = &["reveal", "never"];

/// The words of `journal.side_panel_side`, in [`JournalPanelSide`] order.
pub const PANEL_SIDE_WORDS: &[&str] = &["auto", "left", "right"];

/// How much the journal tells (5.2.4-5.2.8). Tiers 1 and 2 show only what
/// the player has discovered; 3 and 4 are the labelled spoiler tiers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpoilerTier {
    /// What has been achieved.
    Progress = 1,
    /// Also the goals the game has already given (the default, U8).
    #[default]
    Goals = 2,
    /// Spoiler: hints in our own words.
    Hints = 3,
    /// Spoiler: also the answers the save proves the player has used.
    Full = 4,
}

impl SpoilerTier {
    /// Every tier, lowest first.
    pub const ALL: [SpoilerTier; 4] = [
        SpoilerTier::Progress,
        SpoilerTier::Goals,
        SpoilerTier::Hints,
        SpoilerTier::Full,
    ];

    /// The tier numbered `number` (1..=4).
    pub fn from_number(number: i64) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|tier| i64::from(tier.number()) == number)
    }

    pub const fn number(self) -> u8 {
        self as u8
    }

    /// Tiers 3 and 4: the title row reads `SPOILERS` (1.4.7).
    pub const fn is_spoiler(self) -> bool {
        matches!(self, SpoilerTier::Hints | SpoilerTier::Full)
    }
}

/// `journal.mantras` (5.2.7, D5.3). There is no ungated mode in v1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MantraDisplay {
    /// Masked cells, revealed on select (the default, U8).
    #[default]
    Reveal,
    /// The mantra column stays blank at tier 4.
    Never,
}

impl MantraDisplay {
    pub fn key(self) -> &'static str {
        MANTRA_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [MantraDisplay::Reveal, MantraDisplay::Never]
            .into_iter()
            .find(|display| display.key() == word)
    }
}

/// `journal.side_panel_side` (5.2.10). It lives here because the typed view
/// does, and `u5-runtime` cannot name `u5-bevy`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum JournalPanelSide {
    /// The side the scrollback panel does not use.
    #[default]
    Auto,
    Left,
    Right,
}

impl JournalPanelSide {
    pub fn key(self) -> &'static str {
        PANEL_SIDE_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [
            JournalPanelSide::Auto,
            JournalPanelSide::Left,
            JournalPanelSide::Right,
        ]
        .into_iter()
        .find(|side| side.key() == word)
    }
}

/// The typed view `resolved.journal()` (5.3.2). `page` and `side_panel` are
/// independent. The default is every key at its catalogue default: both
/// off, tier 2, mantras `reveal`, side `auto`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct JournalSettings {
    pub page: bool,
    pub tier: SpoilerTier,
    pub mantras: MantraDisplay,
    pub side_panel: bool,
    pub side: JournalPanelSide,
    /// Chapter 8's keys (8.4.1).
    pub clues: ClueSettings,
}

impl JournalSettings {
    /// Whether any part of the journal is on (the plugin's condition, apart
    /// from the menu overlay's).
    pub fn any_on(&self) -> bool {
        self.page || self.side_panel
    }
}

impl ResolvedEnhancements {
    /// The `[journal]` section as [`JournalSettings`] (5.3.6). A missing or
    /// unexpected value reads as the key's default. A row that has not
    /// shipped has already been resolved to its default (U13), and the
    /// parity lock gives the defaults (5.4.5).
    pub fn journal(&self) -> JournalSettings {
        let values = &self.values;
        JournalSettings {
            page: self.toggles.on(Toggle::JournalPage),
            tier: values
                .int(SPOILER_TIER_KEY)
                .and_then(SpoilerTier::from_number)
                .unwrap_or_default(),
            mantras: values
                .str(MANTRAS_KEY)
                .and_then(MantraDisplay::from_key)
                .unwrap_or_default(),
            side_panel: self.toggles.on(Toggle::JournalSidePanel),
            side: values
                .str(SIDE_PANEL_SIDE_KEY)
                .and_then(JournalPanelSide::from_key)
                .unwrap_or_default(),
            clues: self.clue_settings(),
        }
    }
}

// --- Chapter 8: the clue log and quest trackers (design 8.4.1). ----------

pub const CLUES_KEY: &str = "journal.clues";
pub const CLUE_TEXT_KEY: &str = "journal.clue_text";
pub const CLUE_HIGHLIGHT_KEY: &str = "journal.clue_highlight";
pub const CLUE_HIGHLIGHT_STYLE_KEY: &str = "journal.clue_highlight_style";
pub const CLUE_SCROLLBACK_KEY: &str = "journal.clue_scrollback";
pub const CLUE_TOAST_KEY: &str = "journal.clue_toast";
pub const NOTES_KEY: &str = "journal.notes";
pub const CLUE_MISSES_KEY: &str = "journal.clue_misses";
pub const QUEST_TRACKER_KEY: &str = "journal.quest_tracker";
pub const TRACKER_EVIDENCE_KEY: &str = "journal.tracker_evidence";

/// Chapter 8's ten `[journal]` keys in M6, in the order of 8.4.1.
/// `clue_map_links` is not among them: it arrives with the full map (L1).
pub const CLUE_KEYS: &[&str] = &[
    CLUES_KEY,
    CLUE_TEXT_KEY,
    CLUE_HIGHLIGHT_KEY,
    CLUE_HIGHLIGHT_STYLE_KEY,
    CLUE_SCROLLBACK_KEY,
    CLUE_TOAST_KEY,
    NOTES_KEY,
    CLUE_MISSES_KEY,
    QUEST_TRACKER_KEY,
    TRACKER_EVIDENCE_KEY,
];

/// The words `journal.clue_text` accepts in this build. `kept` (8.3.6) is
/// held back until the user signs D8.1, so the resolver rejects it with a
/// note.
pub const CLUE_TEXT_WORDS: &[&str] = &["off", "live"];

/// The `journal.clue_text` word reserved for D8.1.
pub const CLUE_TEXT_KEPT: &str = "kept";

/// The words of `journal.clue_highlight`, in [`ClueHighlight`] order.
pub const CLUE_HIGHLIGHT_WORDS: &[&str] = &["off", "asked", "asked_elsewhere", "new"];

/// The words of `journal.clue_highlight_style`, in [`HighlightStyle`] order.
pub const HIGHLIGHT_STYLE_WORDS: &[&str] = &["dotted", "tint"];

/// The words of `journal.tracker_evidence`, in [`TrackerEvidence`] order.
pub const TRACKER_EVIDENCE_WORDS: &[&str] = &["record", "npc"];

/// `journal.clue_text` (8.3.6).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ClueText {
    /// No text is shown or kept.
    #[default]
    Off,
    /// Rows seen this session from RAM; static renders after a relaunch.
    Live,
    /// The rows seen are written to the sidecar (D8.1; not reachable from
    /// the resolver in this build).
    Kept,
}

impl ClueText {
    pub fn key(self) -> &'static str {
        match self {
            ClueText::Off => "off",
            ClueText::Live => "live",
            ClueText::Kept => CLUE_TEXT_KEPT,
        }
    }

    /// The value for `word`, among the words this build accepts.
    pub fn from_key(word: &str) -> Option<Self> {
        [ClueText::Off, ClueText::Live]
            .into_iter()
            .find(|text| text.key() == word)
    }
}

/// `journal.clue_highlight` (8.3.10, D8.13).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ClueHighlight {
    #[default]
    Off,
    /// Topics asked of this person.
    Asked,
    /// Also a sparse mark under a word asked of someone else.
    AskedElsewhere,
    /// Deviation tier: tints topics not asked yet.
    New,
}

impl ClueHighlight {
    pub const ALL: [ClueHighlight; 4] = [
        ClueHighlight::Off,
        ClueHighlight::Asked,
        ClueHighlight::AskedElsewhere,
        ClueHighlight::New,
    ];

    pub fn key(self) -> &'static str {
        CLUE_HIGHLIGHT_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|mode| mode.key() == word)
    }
}

/// `journal.clue_highlight_style` (8.3.10), a preference.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HighlightStyle {
    #[default]
    Dotted,
    Tint,
}

impl HighlightStyle {
    pub fn key(self) -> &'static str {
        HIGHLIGHT_STYLE_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [HighlightStyle::Dotted, HighlightStyle::Tint]
            .into_iter()
            .find(|style| style.key() == word)
    }
}

/// `journal.tracker_evidence` (8.3.9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TrackerEvidence {
    /// Only the record that carries the step counts.
    #[default]
    Record,
    /// Deviation tier: any answer from a source person counts.
    Npc,
}

impl TrackerEvidence {
    pub fn key(self) -> &'static str {
        TRACKER_EVIDENCE_WORDS[self as usize]
    }

    pub fn from_key(word: &str) -> Option<Self> {
        [TrackerEvidence::Record, TrackerEvidence::Npc]
            .into_iter()
            .find(|evidence| evidence.key() == word)
    }
}

/// Chapter 8's part of the typed view, `resolved.journal().clues`. The
/// default is every key at its catalogue default: everything off,
/// `dotted`, `record`. A key whose prerequisite is off already resolved
/// to its default (8.4.1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClueSettings {
    pub clues: bool,
    pub text: ClueText,
    pub highlight: ClueHighlight,
    pub highlight_style: HighlightStyle,
    pub scrollback: bool,
    pub toast: bool,
    pub notes: bool,
    pub misses: bool,
    pub quest_tracker: bool,
    pub tracker_evidence: TrackerEvidence,
}

impl ClueSettings {
    /// Whether the clue observer is wanted (8.3.4): the log, or the asked
    /// set highlighting needs.
    pub fn observer_on(&self) -> bool {
        self.clues || self.highlight != ClueHighlight::Off
    }

    /// Whether the observer runs without the log (8.3.4): highlighting on,
    /// `clues` off. No file is touched.
    pub fn session_only(&self) -> bool {
        !self.clues && self.highlight != ClueHighlight::Off
    }

    /// Whether the view must carry the `SPOILERS` label (8.2.8, U12).
    pub fn spoiler_risk(&self) -> bool {
        self.highlight == ClueHighlight::New
            || self.misses
            || self.tracker_evidence == TrackerEvidence::Npc
    }
}

impl ResolvedEnhancements {
    fn clue_settings(&self) -> ClueSettings {
        let values = &self.values;
        let on = |toggle| self.toggles.on(toggle);
        ClueSettings {
            clues: on(Toggle::JournalClues),
            text: values
                .str(CLUE_TEXT_KEY)
                .and_then(ClueText::from_key)
                .unwrap_or_default(),
            highlight: values
                .str(CLUE_HIGHLIGHT_KEY)
                .and_then(ClueHighlight::from_key)
                .unwrap_or_default(),
            highlight_style: values
                .str(CLUE_HIGHLIGHT_STYLE_KEY)
                .and_then(HighlightStyle::from_key)
                .unwrap_or_default(),
            scrollback: on(Toggle::JournalClueScrollback),
            toast: on(Toggle::JournalClueToast),
            notes: on(Toggle::JournalNotes),
            misses: on(Toggle::JournalClueMisses),
            quest_tracker: on(Toggle::JournalQuestTracker),
            tracker_evidence: values
                .str(TRACKER_EVIDENCE_KEY)
                .and_then(TrackerEvidence::from_key)
                .unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod clue_rows_tests;
#[cfg(test)]
mod tests;

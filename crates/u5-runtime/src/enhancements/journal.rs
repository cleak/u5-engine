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
        }
    }
}

#[cfg(test)]
mod tests;

//! Every word the quest journal shows (Phase B design 5.2.8, 5.5 Phase 0).
//!
//! This is the one wording table the user reviews before the page ships
//! (M5-02 produces the report for that review). It is our own wording, in a
//! neutral, modern tone. No line is game text: names come from the save
//! (party names) or from engine tables that already exist (virtue, dungeon,
//! town and Shadowlord names, mantras and Words of Power), and the hint
//! lines are our own summaries of the spec's quest graph
//! (`catalogs/quest-graph.md` §4-§8).
//!
//! The wording is precise on purpose: "introduced", never "met"; "executed
//! by Blackthorn", never "killed" or "interred".

use crate::enhancements::journal::SpoilerTier;
use crate::shrine_virtue::ShrineQuestState;

use super::{SectionId, VirtueSummary};

pub const TITLE: &str = "QUEST JOURNAL";
/// Added to the title row at tiers 3 and 4 (foundation 1.4.7).
pub const SPOILERS: &str = "SPOILERS";
pub const NOTHING_RECORDED: &str = "Nothing recorded yet";

pub const fn section_title(section: SectionId) -> &'static str {
    match section {
        SectionId::Virtues => "Virtues",
        SectionId::Dungeons => "Dungeons",
        SectionId::Shadowlords => "Shadowlords",
        SectionId::Regalia => "Regalia",
        SectionId::Items => "Items",
        SectionId::People => "People",
        SectionId::Endgame => "Endgame",
    }
}

/// The summary header (5.2.5): "N complete, M in progress", plus
/// ", R ruined" when R is above zero. There is no "/8".
pub fn summary_line(summary: VirtueSummary) -> String {
    if summary.complete == 0 && summary.in_progress == 0 && summary.ruined == 0 {
        return NOTHING_RECORDED.to_string();
    }
    let mut line = format!(
        "{} complete, {} in progress",
        summary.complete, summary.in_progress
    );
    if summary.ruined > 0 {
        line.push_str(&format!(", {} ruined", summary.ruined));
    }
    line
}

// --- Virtues (5.2.5-5.2.7). --------------------------------------------

/// A virtue's phase label. Not begun has no words at any tier: its marker
/// says it. Tier 1 names only what was achieved; tier 2 adds the goal the
/// altar has already given; "Return to the shrine" waits for tier 3 until
/// spec request S5.1 is answered (D5.10).
pub const fn phase_value(phase: ShrineQuestState, tier: SpoilerTier) -> &'static str {
    match (phase, tier) {
        (ShrineQuestState::NotStarted, _) => "",
        (ShrineQuestState::Ordained, SpoilerTier::Progress) => "Ordained",
        (ShrineQuestState::Ordained, _) => "Read the Codex",
        (ShrineQuestState::CodexRead, SpoilerTier::Progress | SpoilerTier::Goals) => "Codex read",
        (ShrineQuestState::CodexRead, _) => "Return to the shrine",
        (ShrineQuestState::Complete, _) => "Complete",
    }
}

/// The detail lines of a phase, each with the lowest tier that shows it.
pub const fn phase_details(phase: ShrineQuestState) -> &'static [(SpoilerTier, &'static str)] {
    match phase {
        ShrineQuestState::NotStarted => &[(
            SpoilerTier::Hints,
            "Meditate at its shrine and give its mantra to begin.",
        )],
        ShrineQuestState::Ordained => &[
            (SpoilerTier::Progress, "You were ordained at its shrine."),
            (SpoilerTier::Goals, "The shrine sent you to read the Codex."),
        ],
        ShrineQuestState::CodexRead => &[
            (
                SpoilerTier::Progress,
                "You were ordained, and the Codex has been read.",
            ),
            (SpoilerTier::Hints, "Return to the shrine to complete it."),
        ],
        ShrineQuestState::Complete => {
            &[(SpoilerTier::Progress, "This virtue's quest is complete.")]
        }
    }
}

/// Tier 3's mantra hint for a virtue not begun: who teaches it
/// (`catalogs/quest-graph.md` §8). `None` where the graph names no one.
pub const fn mantra_hint(virtue_index: usize) -> Option<&'static str> {
    match virtue_index {
        1 => Some("Greyson knows its mantra."),
        2 => Some("Thorne knows its mantra."),
        3 => Some("Chamfort knows its mantra."),
        5 => Some("Gruman knows its mantra."),
        6 => Some("Kindor knows its mantra."),
        7 => Some("Wartow knows its mantra."),
        _ => None,
    }
}

pub const RUINED: &str = "Ruined";
/// Tiers 1 and 2 name no cause and no way back (5.2.6).
pub const RUIN_LINE: &str = "The shrine lies in ruin.";
pub const RUIN_RESTORE_GOAL: &str = "Restore it before meditating there.";
pub const RUIN_RESTORE_HINT: &str = "Glinkie knows how a ruined shrine is restored.";
/// Spirituality's coordinate row cannot restore a surface shrine
/// (`systems/karma.md` §7.1), so the journal never offers a method.
pub const RUIN_NO_RESTORE: &str = "No way to restore this shrine is known.";
pub const RESTORED_LINE: &str = "The shrine has been restored.";

/// Tier 4's restoration method, only once the matching seal is open.
pub fn ruin_restore_method(word: &str) -> String {
    format!("Yell {word} beside the shrine, then give the virtue and its mantra three times.")
}

// --- Dungeons (5.2.8, 2). ----------------------------------------------

pub const OPENED: &str = "Opened";
pub const SEALED: &str = "Sealed";
pub const DUNGEON_OPENED_LINE: &str = "Its entrance has been opened.";
pub const DUNGEON_INSIDE_LINE: &str = "The party is inside it now.";
pub const DUNGEON_WORD_GOAL: &str = "Learn its Word of Power.";

/// Tier 3's hint for a dungeon's word: who teaches it
/// (`catalogs/quest-graph.md` §4). Doom has no known teacher.
pub const fn dungeon_word_hint(record: usize) -> &'static str {
    match record {
        0 => "Malifora knows the word.",
        1 => "Annon knows the word.",
        2 => "Trian and Goeth in Jhelom lead to the word.",
        3 => "Felespar knows the word.",
        4 => "Fiona knows the word.",
        5 => "Sindar knows the word.",
        6 => "Hassad knows the word.",
        _ => "No one is known to teach this word.",
    }
}

// --- Shadowlords (5.2.8, 3). -------------------------------------------

pub const VANQUISHED: &str = "Vanquished";
/// A slot outside the spec's value set, at tiers 3 and 4 (5.2.12).
pub const UNKNOWN: &str = "Unknown";
pub const SHARD_CARRIED: &str = "Shard carried";
pub const SHARD_DESTROYED: &str = "Shard destroyed";
pub const SHARD_CARRIED_LINE: &str = "Its shard is carried.";
/// Inferred: destruction clears the shard flag (5.2.8).
pub const SHARD_DESTROYED_LINE: &str = "Its shard has been destroyed.";
pub const SHADOWLORD_FLAME_HINT: &str =
    "A shard is destroyed in an Eternal Flame while its Shadowlord stands there.";
pub const SHADOWLORD_NAME_HINT: &str = "Its name summons it.";

/// A shard's label, by the virtue word it is named for (never the
/// Shadowlord's name, which the shard alone does not prove).
pub const fn shard_label(slot: usize) -> &'static str {
    match slot {
        0 => "Falsehood shard",
        1 => "Hatred shard",
        _ => "Cowardice shard",
    }
}

/// The flame that destroys a shard (`catalogs/quest-graph.md` §5), shown at
/// tier 4 for a vanquished Shadowlord only.
pub const fn flame_for(slot: usize) -> &'static str {
    match slot {
        0 => "Flame of Truth",
        1 => "Flame of Love",
        _ => "Flame of Courage",
    }
}

/// A Shadowlord's name as the destruction line prints it: mixed case from
/// the engine's Yell keywords.
pub fn shadowlord_display_name(keyword: &str) -> String {
    let mut chars = keyword.chars();
    match chars.next() {
        Some(first) => first
            .to_ascii_uppercase()
            .to_string()
            .chars()
            .chain(chars.map(|c| c.to_ascii_lowercase()))
            .collect(),
        None => String::new(),
    }
}

// --- Regalia (5.2.8, 4) and items (5.2.8, 5). --------------------------

pub const CARRIED: &str = "Carried";
pub const REGALIA_GOAL: &str = "Sir Simon asks for the Crown, the Sceptre and the Amulet.";

/// The royal regalia and the Sandalwood Box: (label, special item index,
/// tier 3 hint lines).
pub const REGALIA: &[(&str, usize, &[&str])] = &[
    (
        "Crown",
        crate::SPECIAL_ITEM_CROWN_LB_INDEX,
        &[REGALIA_GOAL, "Landon knows where it is kept."],
    ),
    (
        "Sceptre",
        crate::SPECIAL_ITEM_SCEPTRE_LB_INDEX,
        &[REGALIA_GOAL, "Froed starts the trail to it."],
    ),
    (
        "Amulet",
        crate::SPECIAL_ITEM_AMULET_LB_INDEX,
        &[
            REGALIA_GOAL,
            "Lady Tessa knows where it lies.",
            "It lets the party pass through unholy darkness.",
        ],
    ),
    (
        "Sandalwood Box",
        crate::SPECIAL_ITEM_WOODEN_BOX_INDEX,
        &["Saduj speaks of a hidden box in Lord British's chamber."],
    ),
];

/// A tool row's source: a special item, or the grapple's own counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToolSource {
    Special(usize),
    ClimbingGear,
}

/// The tools of 5.2.8, 5, in the design's order. No goals, no hints.
pub const TOOLS: &[(&str, ToolSource)] = &[
    (
        "Magic carpet",
        ToolSource::Special(crate::SPECIAL_ITEM_MAGIC_CARPET_INDEX),
    ),
    (
        "Skull keys",
        ToolSource::Special(crate::SPECIAL_ITEM_SKULL_KEY_INDEX),
    ),
    ("Grapple", ToolSource::ClimbingGear),
    (
        "Spyglass",
        ToolSource::Special(crate::SPECIAL_ITEM_SPYGLASS_INDEX),
    ),
    (
        "Sextant",
        ToolSource::Special(crate::SPECIAL_ITEM_SEXTANT_INDEX),
    ),
    (
        "Pocket watch",
        ToolSource::Special(crate::SPECIAL_ITEM_POCKET_WATCH_INDEX),
    ),
    (
        "HMS Cape plans",
        ToolSource::Special(crate::SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX),
    ),
    (
        "Black badge",
        ToolSource::Special(crate::SPECIAL_ITEM_BLACK_BADGE_INDEX),
    ),
];

// --- People (5.2.8, 6). ------------------------------------------------

pub const IN_PARTY: &str = "In the party";
pub const LODGED: &str = "Lodged";
pub const EXECUTED: &str = "Executed by Blackthorn";
pub const UNNAMED: &str = "Unnamed";
/// An inn whose scene has no name in the gazetteer.
pub const AN_INN: &str = "an inn";
/// A place with introductions whose scene has no name in the gazetteer.
pub const ANOTHER_PLACE: &str = "Another place";
pub const INTRODUCED_LABEL: &str = "Introduced yourself to";
/// The Avatar's standing, the number Ctrl+K prints.
pub const KARMA_LABEL: &str = "Karma";

pub fn lodged_line(town: &str) -> String {
    format!("Waiting at the inn in {town}.")
}

pub fn people_count(count: u32) -> String {
    if count == 1 {
        "1 person".to_string()
    } else {
        format!("{count} people")
    }
}

// --- Endgame (5.2.8, 7; tiers 3 and 4 only). ---------------------------

pub const DONE: &str = "Done";
pub const ENDGAME_SHADOWLORDS: &str = "All three Shadowlords vanquished";
pub const ENDGAME_DOOM: &str = "The Doom entrance opened";
pub const ENDGAME_BOX: &str = "The Sandalwood Box carried";
pub const ENDGAME_SHADOWLORDS_LINE: &str =
    "Doom admits the party only once all three are vanquished.";
pub const ENDGAME_DOOM_LINE: &str = "The final room lies at the bottom of Doom.";
pub const ENDGAME_AMULET_LINE: &str = "The Amulet helps the party through unholy darkness.";
pub const ENDGAME_BOX_LINE: &str = "The last scene asks for it.";

// --- The offline report (5.2.11). --------------------------------------

/// The report's first words; the tier and its name follow.
pub const REPORT_TITLE: &str = "Quest journal";

/// A tier's short name in the report's first line.
pub const fn tier_name(tier: SpoilerTier) -> &'static str {
    match tier {
        SpoilerTier::Progress => "progress only",
        SpoilerTier::Goals => "named goals",
        SpoilerTier::Hints => "hints",
        SpoilerTier::Full => "full checklist",
    }
}

/// The label of a tier 4 answer, by the section it belongs to.
pub const fn secret_label(section: SectionId) -> &'static str {
    match section {
        SectionId::Virtues => "Mantra",
        SectionId::Dungeons => "Word of Power",
        SectionId::Shadowlords => "Flame",
        _ => "Answer",
    }
}

/// The report's line when SAVED.GAM cannot be decoded (5.2.12).
pub fn cannot_read_save(reason: &str) -> String {
    format!("cannot read SAVED.GAM: {reason}")
}

//! Runtime Z-stats browser state.

use crate::*;

pub const Z_STATS_INVENTORY_PANEL_ROWS: usize = 8;

/// A continuation line of a wrapped Z-stats inventory entry. `inventory.md
/// §4.5`: a label that will not fit "the rest of the display line" moves to
/// the next line, which starts at window column 0, over the frame's left
/// rule, rather than at the list's column 1. The painter strips the marker and
/// places the line there.
pub const Z_STATS_WRAPPED_LINE_MARKER: char = '\u{1}';

pub const READY_PICKER_PANEL_ROWS: usize = 8;
/// `inventory.md §5` step 4 (`RETRACTIONS.md` R444): "PgUp and PgDn move seven
/// displayable items toward the relevant endpoint; the frame has seven
/// interior rows." The engine paged by the panel's eight rows.
pub const READY_PICKER_PAGE_STEP: usize = 7;
pub const USE_PICKER_PANEL_ROWS: usize = 8;
pub const READY_PICKER_ESCAPE_MESSAGE: &str = "Done";
pub const ITEM_PICKER_ESCAPE_MESSAGE: &str = "None!";
/// `inventory.md §4.7` empty-state placeholder, printed when the
/// six-slot equipment block holds nothing readied.
pub const Z_STATS_NONE_READY_PLACEHOLDER: &str = "(None ready)";
/// Index into the nine Z-stats panel rows (screen rows 1..=9) where the Arms
/// page draws [`Z_STATS_NONE_READY_PLACEHOLDER`]: screen row 5. `inventory.md
/// §4.7` publishes the literal but no row; this is a clean runtime
/// observation of the stock game (paired `reseed-falsehood-theft`, beat
/// `p2`, 2026-10-26), two rows below where a readied list starts.
pub const Z_STATS_NONE_READY_PANEL_ROW_INDEX: usize = 4;
/// `inventory.md §4.7` empty-state placeholder, printed when an
/// inventory page's row scanner finds no slot with a non-zero count.
pub const Z_STATS_NONE_OWNED_PLACEHOLDER: &str = "(None owned!)";
/// `inventory.md §4.7` per-page slot counts.
pub const Z_STATS_EQUIPMENT_SLOTS: usize = 6;
pub const Z_STATS_ARMAMENTS_SLOTS: usize = 48;
pub const Z_STATS_SPELLS_SLOTS: usize = 48;

/// `inventory.md` §4.7 "The Spells page rows and their labels" (added
/// 2026-10-25, u5-spec#300 Q11): each spell's label, indexed by spell id, "the
/// rune phrases of `catalogs/spell-list.md` cut to ten cells, stored that way
/// rather than shortened at run time. Some cuts fall mid-word and some keep
/// only initials; use them exactly as given."
pub const Z_STATS_SPELL_LABELS: [&str; Z_STATS_SPELLS_SLOTS] = [
    "In Lor", "Grav Por", "An Zu", "An Nox", "Mani", "An Ylem", "An Sanct", "An Xen Cor",
    "Rel Hur", "In Wis", "Kal Xen", "In Xen Man", "Vas Lor", "Vas Flam", "In Flam Gr",
    "In Nox Gr", "In Zu Grav", "In Por", "An Grav", "In Sanct", "In Sanct G", "Uus Por",
    "Des Por", "Wis Quas", "In Bet Xen", "An Ex Por", "In Ex Por", "Vas Mani", "In Zu",
    "Rel Tym", "In Vas P Y", "Quas An Wi", "In An", "Wis An Yle", "An Xen Ex", "Rel Xen Be",
    "Sanct Lo", "Xen Corp", "In Quas Xe", "In Quas Wi", "In Nox Hur", "In Quas Co",
    "In Mani Co", "Kal Xen Co", "In Vas G C", "In Flam Hu", "Vas Rel Po", "An Tym",
];
pub const Z_STATS_REAGENTS_SLOTS: usize = 8;
pub const Z_STATS_ITEMS_SLOTS: usize = 38;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZStatsPage {
    Stats,
    Equipment,
    /// Not part of the published page sequence (`inventory.md §4.7`
    /// lists six pages and no spell-book page among them), so it is
    /// absent from [`ZStatsPage::ORDERED`] and unreachable by
    /// direction navigation. The variant and its renderer are retained
    /// only until the render side is retired.
    SpellBook,
    Reagents,
    Spells,
    SpecialUse,
    EquipmentStock,
    /// The party-counters screen: `Food`, `Gold`, and the dotted-leader
    /// `Keys`/`Gems`/`Torches` rows of `inventory.md §4.7`'s literal
    /// table.
    ///
    /// §4.7 folds this into "the equipment page", but a capture of the
    /// stock game pages through it as a screen of its own, with its own
    /// `Equipment` border label, immediately after the `Arms` screen.
    /// The engine had the literals and no page to draw them on.
    Counters,
}

impl ZStatsPage {
    /// This flat seven-entry order is the published cycle only for a
    /// one-member party. `inventory.md §4.7` (`RETRACTIONS.md` R457)
    /// withdrew the "universal seven-screen cycle" reading this array
    /// once implemented: for **N** current members the real cycle is
    /// **2N + 5** screens - each member's Attributes/Arms pair in
    /// party-slot order, then the five shared screens below. The
    /// party-aware navigation lives in [`ZStatsSession::cycle_position`],
    /// [`ZStatsSession::seek`] and [`ZStatsSession::cycle_len`]; this
    /// array and [`Self::next`]/[`Self::previous`] are unused by that
    /// path and are kept only as the solo-party special case.
    pub const ORDERED: [Self; 7] = [
        Self::Stats,
        Self::Equipment,
        Self::Counters,
        Self::Reagents,
        Self::Spells,
        Self::SpecialUse,
        Self::EquipmentStock,
    ];

    /// The five screens that follow every member's Attributes/Arms pair.
    ///
    /// `inventory.md` §4.7: "shared Equipment, Reagents, Spells, Items and
    /// Armaments". `Counters` is this engine's name for the shared Equipment
    /// screen, `SpecialUse` for Items and `EquipmentStock` for Armaments.
    pub const SHARED: [Self; 5] = [
        Self::Counters,
        Self::Reagents,
        Self::Spells,
        Self::SpecialUse,
        Self::EquipmentStock,
    ];

    /// `inventory.md §4.6`/`§4.7`: the panel's top border label for
    /// this page, or `None` for the two character-specific pages,
    /// whose border carries no label at all.
    ///
    /// The stored literals are the bare words with their punctuation;
    /// the bracketing end-cap triangles are chrome, not characters.
    /// The Items page shares the `Items:` literal with the U-Use item
    /// browser.
    pub const fn border_label(self) -> Option<&'static str> {
        match self {
            Self::Stats | Self::Equipment | Self::SpellBook => None,
            Self::Counters => Some(Z_STATS_EQUIPMENT_HEADING),
            Self::Reagents => Some("Reagents"),
            Self::Spells => Some("Spells"),
            // The U-Use item browser's label carries a colon
            // (`inventory.md §4.4` writes `Items:` over the picker); the
            // Z-stats page's does not. A capture shows the border reading
            // `Items` here.
            Self::SpecialUse => Some("Items"),
            Self::EquipmentStock => Some("Armaments"),
        }
    }

    /// Debug/transcript name for the page. This is **not** the border
    /// label - see [`Self::border_label`] for the published literals.
    pub const fn title(self) -> &'static str {
        match self {
            Self::Stats => "Stats",
            Self::Equipment => "Equipment",
            Self::SpellBook => "Spell Book",
            Self::Reagents => "Reagents",
            Self::Spells => "Spell Charges",
            Self::SpecialUse => "Special/Use Items",
            Self::EquipmentStock => "Weapons/Armour Stock",
            Self::Counters => "Equipment Counters",
        }
    }

    pub fn next(self) -> Self {
        let index = self.index();
        Self::ORDERED[(index + 1) % Self::ORDERED.len()]
    }

    pub fn previous(self) -> Self {
        let index = self.index();
        Self::ORDERED[(index + Self::ORDERED.len() - 1) % Self::ORDERED.len()]
    }

    fn index(self) -> usize {
        Self::ORDERED
            .iter()
            .position(|page| *page == self)
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ZStatsSession {
    pub selected_party_index: usize,
    pub page: ZStatsPage,
    pub inventory_cursor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadySession {
    pub selected_party_index: Option<usize>,
    pub cursor: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UseSession {
    pub cursor: usize,
    pub pending: Option<UsePendingAction>,
    /// Message-window strip scrolls this picker session's panel repaints
    /// have charged (`cleak/u5-engine#25`; `display-driver-abi.md §9.5`,
    /// `inventory.md §7.1`). Each one lifts the painted `Item:` row while the
    /// message cursor stays at its column, so after `n` of them the prompt
    /// sits `n` rows above the row the completion is written on.
    pub strip_scrolls: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastSession {
    pub caster_index: usize,
    pub buffer: String,
    pub combat_actor_slot: Option<usize>,
    pub combat_had_foe: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CastFollowupSession {
    pub caster_index: usize,
    pub spell_code: String,
    pub kind: CastFollowupKind,
    pub buffer: String,
    pub combat_actor_slot: Option<usize>,
    pub combat_had_foe: bool,
}

/// What a spell handler still needs before it can resolve.
///
/// The inline suffix form (`C1BRX7`) supplies every argument at once; the
/// interactive `C`-Cast prompt supplies only the caster, so a handler that
/// needs a direction, a party member, a moon phase or an arena target
/// records the request here and the cast prompt opens the matching
/// follow-up ([`CastFollowupKind`]).
///
/// This used to be carried in the message slot as a sentence naming the
/// engine's own inline syntax (`Direction? Use C1AG6.`), which the
/// follow-up then string-matched. Nothing published looks like that, and a
/// request the matcher failed to recognise printed it to the player - see
/// `cleak/u5-engine#19`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastArgumentRequest {
    /// Inline callers only: the suffix named no caster. The interactive
    /// prompt always supplies one.
    Caster,
    Direction,
    PartyTarget,
    GatePhase,
    Creature,
    Target,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastFollowupKind {
    Direction {
        pass_allowed: bool,
    },
    PartyTarget,
    GatePhase,
    CombatTarget {
        creature: bool,
    },
    CombatCoordinate {
        x: u8,
        y: u8,
        range_origin: Option<(u8, u8)>,
        max_range: Option<u8>,
    },
    /// `magic.md §8` "Creature-prompt targeters" and
    /// `catalogs/spell-list.md`: `AEX`/`BRX`/`IQX` "use the `Creature:`
    /// target prompt", and `CX` is "a separate `Creature: ` cursor
    /// handler". The dispatcher "prints `Creature: ` and opens the arena
    /// cursor", so the target is a cell the player moves to, not a slot
    /// number typed at a harness prompt.
    CombatCreatureCursor {
        x: u8,
        y: u8,
    },
    /// `magic.md §8`: Magic Missile and Fireball "print the shared aiming
    /// prompt" and use "the combat aiming/projectile path", and Kill
    /// "uses the shared `Aim! ` cursor and projectile path"
    /// (`catalogs/spell-list.md`: `CX` "uses `Aim! ` and the same
    /// attack-wrapper family as Magic Missile and Fireball"). Kill's former
    /// `Creature:` prompt is withdrawn by `RETRACTIONS.md` R446. The target
    /// is therefore a confirmed arena *cell*, not a typed slot number.
    CombatAimCursor {
        x: u8,
        y: u8,
    },
}

/// What one key does inside a spell's arena cursor (`combat.md §8.2`, the
/// shared cursor `magic.md §8` sends the spell cursors through).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CombatSpellCursorKey {
    /// Discarded keys, and Enter/`A` on the caster's own cell: "nothing
    /// happens and the loop reads another key".
    Hold,
    /// Escape, or Space on the caster's own cell.
    Cancel,
    /// Enter, `A` or Space anywhere else.
    Confirm,
    /// One of the eight direction codes.
    Move(crate::InputDirection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RestSession {
    pub phase: RestPhase,
    pub hours: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestPhase {
    Hours,
    WatchYesNo,
    WatchSlot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JimmySession {
    pub direction: Direction,
    /// The roster row the shared picker indicates (`inventory.md §4.3`),
    /// which Return or Space commits.
    pub highlight: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SurfaceChestSession {
    pub x: usize,
    pub y: usize,
    pub verb: SurfaceChestVerb,
    /// The roster row the shared picker indicates (`inventory.md §4.3`),
    /// which Return or Space commits.
    pub highlight: usize,
    /// Present when the shared container picker was opened by the dungeon
    /// chest site. `traps.md §2.1` requires the same interactive picker at
    /// both sites; keeping the pending cell here lets the existing modal
    /// input path resume the dungeon operation after confirmation.
    pub dungeon: Option<DungeonChestSelection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DungeonChestSelection {
    pub scene: DungeonScene,
    pub level: u8,
    pub index: usize,
    pub tile: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceChestVerb {
    Get,
    Open,
    /// `containers.md §9`: the moldy-corpse Search branch uses the same
    /// shared acting-member picker as surface containers.
    Search,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShrineSession {
    pub virtue: ShrineVirtue,
    pub phase: ShrinePhase,
    pub mantra_buffer: String,
    /// The typed answer to the virtue question the shrine opens with.
    pub virtue_buffer: String,
    /// `karma.md §12`: the session asks `\nMantra:` **three** times. This
    /// counts the asks still to come, including the open one.
    pub mantra_asks_remaining: u8,
    /// `karma.md §12` "Unfocused result": set false by a wrong virtue answer
    /// or any wrong mantra. "Only an accepted virtue answer plus all three
    /// accepted mantras reaches the shrine quest state machine."
    pub answers_matched: bool,
}

/// `karma.md §12`: "Ask `\nMantra:` three times".
pub const SHRINE_MANTRA_ASK_COUNT: u8 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShrinePhase {
    /// **Measured** 2026-09-07: `E` on a shrine marker opens with the
    /// virtue question (`MISCMSG.DAT` record `29`) and a `:` input row - the
    /// virtue is *typed*, not derived from which shrine the party is on.
    Virtue,
    Mantra,
    Offering,
    /// `karma.md §12`, Codex-unread arm: record `31` is on screen and the
    /// altar is waiting for a command key before the quest sentence.
    /// **Measured** 2026-09-08 (`qa/paired/shrine-three-mantras.tsv`).
    AltarAnnouncement,
    /// The quest sentence is on screen; the next command key brings record
    /// `33`, the instruction to return when the quest is done.
    AltarQuest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MixSession {
    pub phase: MixPhase,
    pub spell_buffer: String,
    pub spell_index: Option<usize>,
    pub reagent_mask: u8,
    pub reagent_cursor: usize,
    pub quantity_buffer: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MixPhase {
    Spell,
    Reagents,
    Quantity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NewOrderSession {
    pub first: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YellSession {
    pub buffer: String,
}

/// `karma.md §7.1`: the four-field ruined-shrine restoration prompt entered
/// from outdoor Yell. It is deliberately separate from M-Meditate's session.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShrineRestorationSession {
    pub word_index: usize,
    pub target_x: usize,
    pub target_y: usize,
    pub response_index: usize,
    pub all_responses_match: bool,
    pub buffer: String,
}

pub const SHRINE_RESTORATION_INPUT_MAX_LEN: usize = 15;
pub const SHRINE_RESTORATION_VIRTUE_PROMPT: &str = "\nUpon what virtue\ndost thou\nmeditate?\n\n:";
pub const SHRINE_RESTORATION_MANTRA_PROMPT: &str = "\nMantra:";
/// The rows of [`SHRINE_RESTORATION_VIRTUE_PROMPT`] that sit above its answer
/// row; the closing `:` opens the row the virtue is typed on.
pub const SHRINE_RESTORATION_VIRTUE_QUESTION: &str = "\nUpon what virtue\ndost thou\nmeditate?\n\n";
/// The text the virtue answer is typed after, on its own row.
pub const SHRINE_RESTORATION_VIRTUE_ANSWER_PROMPT: &str = ":";
/// [`SHRINE_RESTORATION_MANTRA_PROMPT`] without its leading feed, which only
/// closes the previous answer's row: the text each mantra is typed after.
pub const SHRINE_RESTORATION_MANTRA_ANSWER_PROMPT: &str = "Mantra:";
pub const SHRINE_RESTORATION_SUCCESS_BANNER: &str = "\n\nThe Shrine is\nrestored!\n";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WishingWellSession {
    pub direction: Direction,
    pub coin_accepted: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectionPromptSession {
    pub kind: DirectionPromptKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionPromptKind {
    Attack,
    DungeonLook {
        party_index: Option<usize>,
        drink: Option<bool>,
    },
    SurfaceFountainDrink {
        direction: Direction,
    },
    DungeonSearch {
        party_index: Option<usize>,
    },
    Klimb,
    CombatKlimb {
        actor_slot: usize,
    },
    CombatPush {
        actor_slot: usize,
    },
    CombatSjog {
        actor_slot: usize,
        branch: CombatCommandBranch,
    },
    Fire,
    Get,
    Jimmy,
    Look,
    Open,
    Push,
    Search,
    Talk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonLookFocus {
    Ahead,
    Right,
    Left,
    Here,
}

/// The relative-focus helper reads the four movement keys, not letters:
/// up is ahead, left and right rotate the facing a quarter turn, and down
/// selects the party's own cell (measured 2026-09-07 against the stock
/// game, which ignores `A`/`R`/`L`/`H` and Escape here).
///
/// Besides the direction codes it reads the compass digits, because its only
/// caller is the harness's inline `L`/`S` operand, which is not a keystroke
/// at any prompt (`input.md §5` keeps a typed digit out of the prompts
/// themselves; see [`crate::Direction::from_prompt_key`]).
pub fn dungeon_look_focus_from_key(key: char) -> Option<DungeonLookFocus> {
    let direction = match key {
        '8' | '4' | '6' | '2' => crate::Direction::from_play_key(key),
        _ => crate::Direction::from_prompt_key(key),
    };
    match direction? {
        crate::Direction::North => Some(DungeonLookFocus::Ahead),
        crate::Direction::West => Some(DungeonLookFocus::Left),
        crate::Direction::East => Some(DungeonLookFocus::Right),
        crate::Direction::South => Some(DungeonLookFocus::Here),
        _ => None,
    }
}

/// The interactive `Dir-` chooser's reading of a key: the four cardinal
/// input codes and nothing else.
///
/// The key arrives as the frontend translated it (`input.md §5`; the dungeon
/// exception there - "does not run the digit-to-direction translation at
/// all" - is scoped to the dungeon command reader by u5-spec#299 Q18,
/// `RETRACTIONS.md` R572), and `dungeon-mode.md §12` gives the
/// chooser only Up/Left/Right/Down and Space, every other key "ignored
/// without echo". [`dungeon_look_focus_from_key`] keeps the compass digits
/// for the harness's inline `L`/`S` operands, which are not keystrokes at
/// the chooser (playtest E19).
pub fn dungeon_chooser_focus_from_key(key: char) -> Option<DungeonLookFocus> {
    let byte = u8::try_from(u32::from(key)).ok()?;
    match crate::input_code_direction(byte)? {
        crate::InputDirection::North => Some(DungeonLookFocus::Ahead),
        crate::InputDirection::West => Some(DungeonLookFocus::Left),
        crate::InputDirection::East => Some(DungeonLookFocus::Right),
        crate::InputDirection::South => Some(DungeonLookFocus::Here),
        _ => None,
    }
}

/// The word the chosen focus writes onto the open `Dir-` line.
pub const fn dungeon_look_focus_label(focus: DungeonLookFocus) -> &'static str {
    match focus {
        DungeonLookFocus::Ahead => crate::DUNGEON_DIRECTION_LABEL_AHEAD,
        DungeonLookFocus::Right => crate::DUNGEON_DIRECTION_LABEL_RIGHT,
        DungeonLookFocus::Left => crate::DUNGEON_DIRECTION_LABEL_LEFT,
        DungeonLookFocus::Here => crate::DUNGEON_DIRECTION_LABEL_HERE,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YesNoPromptSession {
    pub kind: YesNoPromptKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YesNoPromptKind {
    DungeonFountainDrink {
        party_index: usize,
        focus: DungeonLookFocus,
    },
    TownExit {
        scene: Scene,
        floor: i8,
    },
    SaveGame,
    ExitToDos,
    /// `overworld.md §7.1` step 3's "`Dost thou pay?`" toll demand. `toll`
    /// is the already-rolled amount (step 4), signed per the spec's "signed
    /// sixteen-bit" arithmetic.
    TrollBridgeToll { toll: i16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsePendingAction {
    /// `inventory.md §4`: the `On who:` prompt is the shared party-member
    /// selector, and there "a digit moves the indicator; it does not commit
    /// [...] Only Return or Space commits the indicated row". `highlight` is
    /// the row the digits and direction keys move.
    PotionTarget {
        index: usize,
        highlight: usize,
    },
    ScrollWindDirection {
        index: usize,
    },
    ScrollResurrectionTarget {
        index: usize,
        highlight: usize,
    },
    /// **Measured** 2026-09-07: the skull key completes its row as
    /// `Item: Skull Key` and then opens the shared `Direction-` prompt
    /// rather than acting on the party's facing.
    SkullKeyDirection,
    /// `inventory.md §7.1` "The Sceptre's fallback": in an arena, when the
    /// nearby scan dissolved nothing, the field-removal helper "shows the
    /// ordinary direction prompt".
    SceptreFieldDirection,
}

impl ReadySession {
    pub const fn new() -> Self {
        Self {
            selected_party_index: None,
            cursor: 0,
        }
    }

    pub const fn with_party(selected_party_index: usize) -> Self {
        Self {
            selected_party_index: Some(selected_party_index),
            cursor: 0,
        }
    }

    pub fn select_party_index(&mut self, party_index: usize) {
        self.selected_party_index = Some(party_index);
        self.cursor = 0;
    }
}

impl UseSession {
    pub const fn new() -> Self {
        Self {
            cursor: 0,
            pending: None,
            strip_scrolls: 0,
        }
    }

    /// The open row the picker's cursor waits on: `Item: ` until a strip
    /// scroll has lifted it, then that prompt's width in spaces, since the
    /// cursor keeps the prompt's column on the row below
    /// (`cleak/u5-engine#25`). The completion and `None!` are written there.
    pub fn item_prompt_literal(&self) -> String {
        if self.strip_scrolls == 0 {
            crate::ITEM_SELECTION_PROMPT.to_string()
        } else {
            " ".repeat(crate::ITEM_SELECTION_PROMPT.chars().count())
        }
    }
}

impl ZStatsSession {
    pub const fn new(selected_party_index: usize) -> Self {
        Self {
            selected_party_index,
            page: ZStatsPage::Stats,
            inventory_cursor: 0,
        }
    }

    /// Where this session sits in the published page cycle.
    ///
    /// `inventory.md` §4.7 (`RETRACTIONS.md` R457): "Attributes and Arms
    /// repeat for every current party slot, followed by shared Equipment,
    /// Reagents, Spells, Items and Armaments. Total 2N+5, including eleven for
    /// three members; seven applies only to one member."
    ///
    /// So positions `0..2N` are the per-member pairs - even is that member's
    /// Attributes, odd its Arms - and the five shared screens follow. The
    /// engine walked a flat seven-entry list, which is the cycle only for a
    /// solo party; with a fuller party it skipped every companion's pages.
    pub fn cycle_position(&self, party_len: usize) -> usize {
        let members = party_len.max(1);
        match self.page {
            ZStatsPage::Stats => self.selected_party_index.min(members - 1) * 2,
            ZStatsPage::Equipment => self.selected_party_index.min(members - 1) * 2 + 1,
            page => {
                let shared = ZStatsPage::SHARED
                    .iter()
                    .position(|candidate| *candidate == page)
                    .unwrap_or(0);
                members * 2 + shared
            }
        }
    }

    pub fn cycle_len(party_len: usize) -> usize {
        party_len.max(1) * 2 + ZStatsPage::SHARED.len()
    }

    fn seek(&mut self, position: usize, party_len: usize) {
        let members = party_len.max(1);
        if position < members * 2 {
            self.selected_party_index = position / 2;
            self.page = if position % 2 == 0 {
                ZStatsPage::Stats
            } else {
                ZStatsPage::Equipment
            };
        } else {
            let shared = (position - members * 2).min(ZStatsPage::SHARED.len() - 1);
            self.page = ZStatsPage::SHARED[shared];
        }
        self.inventory_cursor = 0;
    }

    pub fn move_next_page(&mut self, party_len: usize) {
        let len = Self::cycle_len(party_len);
        let next = (self.cycle_position(party_len) + 1) % len;
        self.seek(next, party_len);
    }

    pub fn move_previous_page(&mut self, party_len: usize) {
        let len = Self::cycle_len(party_len);
        let previous = (self.cycle_position(party_len) + len - 1) % len;
        self.seek(previous, party_len);
    }

    /// `inventory.md` §4 (`RETRACTIONS.md` R458): "A valid member digit always
    /// opens that member's Attributes, from Arms or any shared screen as
    /// well." The engine used to preserve the Attributes/Arms half.
    pub fn select_party_index(&mut self, party_index: usize) {
        self.selected_party_index = party_index;
        self.page = ZStatsPage::Stats;
        self.inventory_cursor = 0;
    }

    /// `inventory.md` §4 (`RETRACTIONS.md` R459): at the initial Z selector
    /// "`0` opens the shared Equipment screen without choosing a member".
    pub fn open_shared_equipment(&mut self) {
        self.page = ZStatsPage::Counters;
        self.inventory_cursor = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZStatsInputAction {
    Exit,
    NextPage,
    PreviousPage,
    InventoryPageNext,
    InventoryPagePrevious,
    SelectParty(usize),
    /// `inventory.md §4`: while browsing, "`0` jumps to shared Equipment".
    SharedEquipment,
    Redraw,
    Discard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadyInputAction {
    Exit,
    Confirm,
    NextItem,
    PreviousItem,
    PageNext,
    PagePrevious,
    /// `inventory.md §5` step 4 (`RETRACTIONS.md` R444): "Home and End select
    /// the first and last displayable items". The engine used to page all
    /// four corner keys by a full eight-row window.
    FirstItem,
    LastItem,
    SelectParty(usize),
    Redraw,
    Discard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseInputAction {
    Exit,
    Confirm,
    NextItem,
    PreviousItem,
    PageNext,
    PagePrevious,
    /// See [`ReadyInputAction::FirstItem`]; the U-Use picker is the same
    /// shared picker (`inventory.md §5`, "The picker is shared
    /// infrastructure").
    FirstItem,
    LastItem,
    Redraw,
    Discard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastInputAction {
    Cancel,
    Complete,
    Backspace,
    Append(char),
    Discard,
}

impl CastSession {
    pub fn new(caster_index: usize) -> Self {
        Self {
            caster_index,
            buffer: String::new(),
            combat_actor_slot: None,
            combat_had_foe: false,
        }
    }

    pub fn for_combat_actor(actor_slot: usize, combat_had_foe: bool) -> Self {
        Self {
            caster_index: actor_slot,
            buffer: String::new(),
            combat_actor_slot: Some(actor_slot),
            combat_had_foe,
        }
    }
}

impl CastFollowupSession {
    pub fn new(
        caster_index: usize,
        spell_code: String,
        kind: CastFollowupKind,
        combat_actor_slot: Option<usize>,
        combat_had_foe: bool,
    ) -> Self {
        Self {
            caster_index,
            spell_code,
            kind,
            buffer: String::new(),
            combat_actor_slot,
            combat_had_foe,
        }
    }
}

impl RestSession {
    pub const fn new() -> Self {
        Self {
            phase: RestPhase::Hours,
            hours: None,
        }
    }
}

impl JimmySession {
    pub const fn new(direction: Direction) -> Self {
        Self {
            direction,
            highlight: 0,
        }
    }
}

impl SurfaceChestSession {
    pub const fn new(x: usize, y: usize, verb: SurfaceChestVerb) -> Self {
        Self {
            x,
            y,
            verb,
            highlight: 0,
            dungeon: None,
        }
    }

    pub const fn new_dungeon(
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        index: usize,
        tile: u8,
    ) -> Self {
        Self {
            x,
            y,
            verb: SurfaceChestVerb::Open,
            highlight: 0,
            dungeon: Some(DungeonChestSelection {
                scene,
                level,
                index,
                tile,
            }),
        }
    }
}

impl SurfaceChestVerb {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Get => "Got",
            Self::Open => "Opened",
            Self::Search => "Searched",
        }
    }
}

impl ShrineSession {
    pub fn new(virtue: ShrineVirtue) -> Self {
        Self {
            virtue,
            phase: ShrinePhase::Mantra,
            mantra_buffer: String::new(),
            virtue_buffer: String::new(),
            mantra_asks_remaining: SHRINE_MANTRA_ASK_COUNT,
            answers_matched: true,
        }
    }

    /// The session an `E` on a shrine marker opens: the virtue question
    /// first, with the coordinate's own virtue only as the fallback the
    /// mantra phase starts from.
    pub fn entering(virtue: ShrineVirtue) -> Self {
        Self {
            virtue,
            phase: ShrinePhase::Virtue,
            mantra_buffer: String::new(),
            virtue_buffer: String::new(),
            mantra_asks_remaining: SHRINE_MANTRA_ASK_COUNT,
            answers_matched: true,
        }
    }
}

impl MixSession {
    pub fn new() -> Self {
        Self {
            phase: MixPhase::Spell,
            spell_buffer: String::new(),
            spell_index: None,
            reagent_mask: 0,
            reagent_cursor: 0,
            quantity_buffer: String::new(),
        }
    }
}

impl NewOrderSession {
    pub const fn new() -> Self {
        Self { first: None }
    }
}

impl YellSession {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }
}

impl ShrineRestorationSession {
    pub fn new(word_index: usize, target_x: usize, target_y: usize) -> Self {
        Self {
            word_index,
            target_x,
            target_y,
            response_index: 0,
            all_responses_match: true,
            buffer: String::new(),
        }
    }

    /// The prompt text the current answer is typed after: the virtue
    /// question's closing `:` for the first read, `Mantra:` for the three
    /// mantra reads (`karma.md §7.1`).
    pub fn answer_prompt(&self) -> &'static str {
        if self.response_index == 0 {
            SHRINE_RESTORATION_VIRTUE_ANSWER_PROMPT
        } else {
            SHRINE_RESTORATION_MANTRA_ANSWER_PROMPT
        }
    }
}

impl WishingWellSession {
    pub const fn new(direction: Direction) -> Self {
        Self {
            direction,
            coin_accepted: false,
        }
    }
}

impl DirectionPromptSession {
    pub const fn new(kind: DirectionPromptKind) -> Self {
        Self { kind }
    }
}

impl YesNoPromptSession {
    pub const fn new(kind: YesNoPromptKind) -> Self {
        Self { kind }
    }
}

pub fn cast_input_action(key: char) -> CastInputAction {
    match key {
        '\u{1b}' => CastInputAction::Cancel,
        '\r' | '\n' | ' ' => CastInputAction::Complete,
        '\u{8}' | '\u{7f}' => CastInputAction::Backspace,
        ch if ch.is_ascii_alphabetic() && !spell_selector_is_ignored(ch as u8) => {
            CastInputAction::Append(ch.to_ascii_uppercase())
        }
        _ => CastInputAction::Discard,
    }
}

pub fn ready_input_action(key: char) -> ReadyInputAction {
    match key {
        '\u{1b}' => ReadyInputAction::Exit,
        '\r' | '\n' | ' ' => ReadyInputAction::Confirm,
        ch if ch as u32 == u32::from(INPUT_CODE_SOUTH) => ReadyInputAction::NextItem,
        ch if ch as u32 == u32::from(INPUT_CODE_NORTH) => ReadyInputAction::PreviousItem,
        // `input.md §4`: the four corner keys arrive as four *distinct*
        // diagonal codes - Home northwest, End southwest, PgUp northeast,
        // PgDn southeast - and `inventory.md §5` gives each its own job.
        ch if ch as u32 == u32::from(INPUT_CODE_NORTHWEST) => ReadyInputAction::FirstItem,
        ch if ch as u32 == u32::from(INPUT_CODE_SOUTHWEST) => ReadyInputAction::LastItem,
        ch if ch as u32 == u32::from(INPUT_CODE_NORTHEAST) => ReadyInputAction::PagePrevious,
        ch if ch as u32 == u32::from(INPUT_CODE_SOUTHEAST) => ReadyInputAction::PageNext,
        // Retain the terminal harness's printable navigation aliases.
        '>' | '+' => ReadyInputAction::NextItem,
        '<' | '-' => ReadyInputAction::PreviousItem,
        ']' | '}' => ReadyInputAction::PageNext,
        '[' | '{' => ReadyInputAction::PagePrevious,
        '1'..='6' => ReadyInputAction::SelectParty((key as u8 - b'1') as usize),
        'R' | 'r' => ReadyInputAction::Redraw,
        _ => ReadyInputAction::Discard,
    }
}

pub fn use_input_action(key: char) -> UseInputAction {
    match key {
        '\u{1b}' => UseInputAction::Exit,
        '\r' | '\n' | ' ' => UseInputAction::Confirm,
        ch if ch as u32 == u32::from(INPUT_CODE_SOUTH) => UseInputAction::NextItem,
        ch if ch as u32 == u32::from(INPUT_CODE_NORTH) => UseInputAction::PreviousItem,
        // `input.md §4`: the four corner keys arrive as four *distinct*
        // diagonal codes - Home northwest, End southwest, PgUp northeast,
        // PgDn southeast - and `inventory.md §5` gives each its own job.
        ch if ch as u32 == u32::from(INPUT_CODE_NORTHWEST) => UseInputAction::FirstItem,
        ch if ch as u32 == u32::from(INPUT_CODE_SOUTHWEST) => UseInputAction::LastItem,
        ch if ch as u32 == u32::from(INPUT_CODE_NORTHEAST) => UseInputAction::PagePrevious,
        ch if ch as u32 == u32::from(INPUT_CODE_SOUTHEAST) => UseInputAction::PageNext,
        // Retain the terminal harness's printable navigation aliases.
        '>' | '+' => UseInputAction::NextItem,
        '<' | '-' => UseInputAction::PreviousItem,
        ']' | '}' => UseInputAction::PageNext,
        '[' | '{' => UseInputAction::PagePrevious,
        'U' | 'u' => UseInputAction::Redraw,
        _ => UseInputAction::Discard,
    }
}

pub fn ready_first_input_key(key: char, suffix: &str) -> char {
    suffix
        .chars()
        .find(|ch| !ch.is_ascii_whitespace())
        .unwrap_or(key)
}

/// Which way one of the four direction keys moves a list indicator.
///
/// `inventory.md §4.3`: "the four direction keys move the indicator."
/// It does not publish which way each key goes; South/East step forward
/// and North/West step back, matching the R-Ready picker's own
/// North/South scan (`§5` step 4) and a capture of the original, where
/// `RIGHT` moves the `Select:` bar from the Avatar down to Shamino.
/// What one key does at the acting-member picker the trap-bearing commands
/// open at their chest sites (Jimmy, Open, Get, Search).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActingMemberPickerKey {
    /// Escape: "Escape prints `None!\n` on the prompt row and aborts"
    /// (`commands.md §5.8`).
    Cancel,
    /// Pick this roster row.
    Pick(usize),
    /// Move the indicated row here; the prompt stays open.
    Move(usize),
    /// Read past: `0`, out-of-range digits and every other key.
    Ignore,
}

/// `commands.md §5.8`'s conditional acting-member exchange runs the shared
/// roster picker (`inventory.md §4.3`) with the chest sites' argument:
///
/// - Escape cancels.
/// - "**A digit moves the indicator; it does not commit.** `1` through `6`,
///   bounded by the party size, reposition the inverted row exactly as the
///   direction keys do and leave the prompt open" (`inventory.md §4.3`,
///   clarified by issue #192: the older "select directly" bullet "was read
///   as a commit"). This picker committed on a digit, unlike the shared
///   selector every other caller uses (playtest F10).
/// - "Only Return or Space commits the indicated row", and "the four
///   direction keys move the indicator". Neither is a cancel.
/// - `0` "commits the explicit 'no one' answer only in the callers that
///   allow it ... other callers ignore `0`", and at the chest sites "the
///   digit `0` the reader would need is simply ignored".
pub fn acting_member_picker_key(key: char, highlight: usize, party_len: usize) -> ActingMemberPickerKey {
    let rows = party_len.min(6);
    match key {
        '\u{1b}' => return ActingMemberPickerKey::Cancel,
        '\r' | '\n' | ' ' if highlight < rows => return ActingMemberPickerKey::Pick(highlight),
        _ => {}
    }
    if let Some(forward) = party_selector_direction_step(key) {
        let last = rows.saturating_sub(1);
        return ActingMemberPickerKey::Move(if forward {
            highlight.saturating_add(1).min(last)
        } else {
            highlight.saturating_sub(1)
        });
    }
    match key.to_digit(10).map(|digit| digit as usize) {
        Some(digit) if (1..=rows).contains(&digit) => ActingMemberPickerKey::Move(digit - 1),
        _ => ActingMemberPickerKey::Ignore,
    }
}

pub fn party_selector_direction_step(key: char) -> Option<bool> {
    let byte = u8::try_from(key as u32).ok()?;
    if byte == INPUT_CODE_SOUTH || byte == INPUT_CODE_EAST {
        return Some(true);
    }
    if byte == INPUT_CODE_NORTH || byte == INPUT_CODE_WEST {
        return Some(false);
    }
    None
}

pub fn z_stats_input_action(key: char) -> ZStatsInputAction {
    // `inventory.md §4`: "Direction-style navigation moves backward or
    // forward through the visible page sequence."
    if let Some(forward) = party_selector_direction_step(key) {
        return if forward {
            ZStatsInputAction::NextPage
        } else {
            ZStatsInputAction::PreviousPage
        };
    }
    match key {
        ' ' | '\u{1b}' => ZStatsInputAction::Exit,
        '>' | '+' | '\r' | '\n' => ZStatsInputAction::NextPage,
        '<' | '-' => ZStatsInputAction::PreviousPage,
        ']' | '}' => ZStatsInputAction::InventoryPageNext,
        '[' | '{' => ZStatsInputAction::InventoryPagePrevious,
        '1'..='6' => ZStatsInputAction::SelectParty((key as u8 - b'1') as usize),
        '0' => ZStatsInputAction::SharedEquipment,
        'Z' | 'z' => ZStatsInputAction::Redraw,
        _ => ZStatsInputAction::Discard,
    }
}

pub fn z_stats_first_input_key(key: char, suffix: &str) -> char {
    suffix
        .chars()
        .find(|ch| !ch.is_ascii_whitespace())
        .unwrap_or(key)
}

/// `magic.md section 11` per-character spell-book class filter. Mage,
/// Druid, and Avatar-class records are full spellcasters; Bard and
/// Tinker records keep the smaller bard-family half-magic book;
/// fighter-family records do not publish a spell-book list. This is
/// display-only: the C-Cast dispatcher still owns resource gates and
/// effect legality.
pub const fn z_stats_spell_book_max_circle(class_byte: u8) -> u8 {
    match class_byte {
        b'A' | b'M' | b'D' => 8,
        b'B' | b'T' => 4,
        _ => 0,
    }
}

pub fn spell_recipe_label(mask: u8) -> String {
    const REAGENTS: [Reagent; REAGENT_COUNT] = [
        Reagent::SulfurAsh,
        Reagent::Ginseng,
        Reagent::Garlic,
        Reagent::SpiderSilk,
        Reagent::BloodMoss,
        Reagent::BlackPearl,
        Reagent::Nightshade,
        Reagent::Mandrake,
    ];
    let names = REAGENTS
        .iter()
        .zip(REAGENT_MASKS.iter())
        .filter_map(|(reagent, reagent_mask)| {
            (mask & *reagent_mask != 0).then(|| reagent.abbreviation())
        })
        .collect::<Vec<_>>();
    if names.is_empty() {
        "no reagents".to_string()
    } else {
        names.join("+")
    }
}

pub fn special_item_name(index: usize) -> &'static str {
    match index {
        SPECIAL_ITEM_MAGIC_CARPET_INDEX => "Magic Carpet",
        SPECIAL_ITEM_SKULL_KEY_INDEX => "Skull Keys",
        2 => "Odd Key",
        SPECIAL_ITEM_AMULET_LB_INDEX => "Amulet of Lord British",
        SPECIAL_ITEM_CROWN_LB_INDEX => "Crown of Lord British",
        SPECIAL_ITEM_SCEPTRE_LB_INDEX => "Sceptre of Lord British",
        SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX => "Shard of Falsehood",
        SPECIAL_ITEM_SHARD_HATRED_INDEX => "Shard of Hatred",
        SPECIAL_ITEM_SHARD_COWARDICE_INDEX => "Shard of Cowardice",
        9 => "Grapple",
        SPECIAL_ITEM_SPYGLASS_INDEX => "Spyglass",
        SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX => "HMS Cape Plans",
        SPECIAL_ITEM_SEXTANT_INDEX => "Sextant",
        SPECIAL_ITEM_POCKET_WATCH_INDEX => "Pocket Watch",
        SPECIAL_ITEM_BLACK_BADGE_INDEX => "Black Badge",
        SPECIAL_ITEM_WOODEN_BOX_INDEX => "Wooden Box",
        _ => "Special Item",
    }
}

/// `inventory.md §4.5`: the decorated potion row prints "the potion's **short
/// colour name**" after the marker, and the 38-name family lists the eight
/// potions as bare colours - `Blue`, `Yellow`, ... The longer
/// [`potion_inventory_name`] form is the Z-stats/message wording.
pub fn potion_short_colour_name(index: usize) -> &'static str {
    match index {
        POTION_BLUE_INDEX => "Blue",
        POTION_YELLOW_INDEX => "Yellow",
        POTION_RED_INDEX => "Red",
        POTION_GREEN_INDEX => "Green",
        POTION_ORANGE_INDEX => "Orange",
        POTION_PURPLE_INDEX => "Purple",
        POTION_BLACK_INDEX => "Black",
        POTION_WHITE_INDEX => "White",
        _ => "Potion",
    }
}

pub fn potion_inventory_name(index: usize) -> &'static str {
    match index {
        POTION_BLUE_INDEX => "Blue Potion",
        POTION_YELLOW_INDEX => "Yellow Potion",
        POTION_RED_INDEX => "Red Potion",
        POTION_GREEN_INDEX => "Green Potion",
        POTION_ORANGE_INDEX => "Orange Potion",
        POTION_PURPLE_INDEX => "Purple Potion",
        POTION_BLACK_INDEX => "Black Potion",
        POTION_WHITE_INDEX => "White Potion",
        _ => "Potion",
    }
}

/// `inventory.md §4`: "The command starts by choosing a character... In
/// combat scenes, Z-stats and R-Ready bind to the currently active living
/// combat actor when that actor maps to a party slot; outside combat they
/// use the normal party-member selector. Escape cancels the selector."
///
/// The selector is a presentation stage as much as an input stage: while
/// it is live the party-roster box's border label reads `Select:` and the
/// candidate roster row is drawn in inverse video.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PartySelectorSession {
    /// Which command opened the selector, so the confirmed slot can be
    /// handed to the right follow-on stage.
    pub target: PartySelectorTarget,
    /// The roster row currently offered, drawn in inverse video.
    pub highlight: usize,
}

impl PartySelectorSession {
    pub const fn new(target: PartySelectorTarget, highlight: usize) -> Self {
        Self { target, highlight }
    }
}

/// The command a live [`PartySelectorSession`] is selecting for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartySelectorTarget {
    ZStats,
    /// `cleak/u5-spec#194` capture: the direction-form Search asks
    /// `Player: ` for the acting member before it searches.
    Search {
        direction: crate::Direction,
    },
    /// `commands.md §6`: N-New Order "prompts for two party members
    /// through the shared party-member selector". `cleak/u5-spec#194`
    /// capture: the prompts read `Swap ` and `with `.
    NewOrder {
        first: Option<usize>,
    },
    /// `magic.md §5` step 1 / `text-output.md §10.6`: with no active player
    /// set, C-Cast runs the `Player: ` active-player prompt first.
    Cast,
    /// `view.md §3`: a surface or town fountain "prompt[s] for the drinking
    /// party member". A capture of the stock game shows that prompt using
    /// this same shared surface - the `Select:` border label and the
    /// inverted roster row - rather than a bespoke one.
    FountainDrink {
        direction: crate::Direction,
    },
    /// `view.md §3` death-vision tile: the crystal sphere "uses the
    /// conditional acting-member selection of `commands.md` Section 5.8",
    /// so two or more eligible members produce `Player: ` and the vision
    /// resumes at the looked-at cell for the chosen member.
    DeathVision {
        x: usize,
        y: usize,
    },
    /// `dungeon-mode.md §8.1`, "Who acts": dungeon L-Look and Search "run
    /// the shared acting-member prompt", which is `inventory.md §4.3`'s one
    /// shared routine - a digit moves the indicator, Return or Space
    /// commits, Escape cancels and `0` is ignored (playtest F10, F11).
    /// On a pick the `Dir-` relative-focus chooser opens.
    DungeonLook {
        drink: Option<bool>,
    },
    DungeonSearch,
    /// `rest-and-camp.md §4`'s camp "member prompt", `Who will stand guard? `.
    /// `inventory.md §4.3`: "Every command that needs a party member ...
    /// shares one selection surface ... This is one shared routine, so the
    /// rule is the same for ... every other caller" - the `Select:` label,
    /// the inverted row, digits that move rather than commit, Return or Space
    /// to commit and Escape to cancel (playtest H08, H36). The keys are read
    /// by the live rest session, which owns the camp that follows.
    CampWatch,
}

impl PartySelectorTarget {
    /// Whether this prompt opens a new block in the message window - and
    /// so spends `text-output.md §10.4`'s leading line feed on a blank row
    /// - or continues the one the command echo already opened.
    ///
    /// The captures disagree per command, so this follows them rather than
    /// a rule: `>New Order` has a blank row under it before `Swap `, and
    /// so does `>Ready...` before its `Player: ` (that one is opened by
    /// [`crate::PlayState::start_ready_equipment`]). `>Z-stats...`,
    /// `>Cast...` and `>Search-<direction>` do not - their `Player: `
    /// sits directly under the echo. N-New Order's second prompt
    /// continues its first: `Swap Shamino` / `with Iolo!` are consecutive
    /// rows. `cleak/u5-engine#5`.
    pub const fn prompt_opens_a_block(self) -> bool {
        matches!(self, Self::NewOrder { first: None })
    }

    /// The message-window prompt the selector opens with.
    /// The word a cancel writes onto the open prompt row. **Measured**
    /// 2026-09-07: cancelling New Order reads `Swap nobody!`, not the
    /// `None!` every other selector answers with.
    pub fn cancel_reply(self) -> &'static str {
        match self {
            Self::NewOrder { .. } => NEW_ORDER_CANCEL_REPLY,
            _ => crate::SELECTION_CANCELLED_LITERAL,
        }
    }

    /// `inventory.md §4.3`: "`0` commits the explicit 'no one' answer only
    /// in the callers that allow it ... other callers ignore `0`." The
    /// dungeon acting-member callers are among the others: `dungeon-mode.md
    /// §8.1` gives that prompt a pick, a re-prompt and a cancel only.
    ///
    /// Of this selector's callers only Z-stats is published as allowing it
    /// (§4.3, R459: "Z uses that answer to open shared Equipment"); the
    /// active-player prompt is the digit command, not this surface. Cast,
    /// Search and New Order used to take `0` as a cancel and print `None!` /
    /// `Swap nobody!`, but `commands.md §5.8` says "the digit `0` ... is
    /// simply ignored" where the answer is not wired, and only Escape cancels
    /// (playtest J28).
    pub const fn ignores_zero(self) -> bool {
        !matches!(self, Self::ZStats)
    }

    pub fn prompt(self) -> &'static str {
        match self {
            Self::NewOrder { first: None } => NEW_ORDER_FIRST_PROMPT,
            Self::NewOrder { first: Some(_) } => NEW_ORDER_SECOND_PROMPT,
            Self::FountainDrink { .. } => crate::FOUNTAIN_DRINK_PROMPT,
            Self::CampWatch => crate::REST_WATCH_MEMBER_PROMPT,
            _ => crate::PARTY_SELECTION_PROMPT,
        }
    }
}

/// `cleak/u5-spec#194` capture: the two N-New Order selector prompts.
pub const NEW_ORDER_FIRST_PROMPT: &str = "Swap ";
pub const NEW_ORDER_SECOND_PROMPT: &str = "with ";
/// `commands.md §6`: picking the leader in either prompt prints the
/// leader's actual name followed by this suffix (`Avatar must lead!` is the
/// example for a leader named Avatar).
pub const NEW_ORDER_LEADER_REFUSAL_SUFFIX: &str = " must lead!";

/// `commands.md §6`: the leader refusal line for a leader named `leader`.
pub fn new_order_leader_refusal(leader: &str) -> String {
    format!("{leader}{NEW_ORDER_LEADER_REFUSAL_SUFFIX}")
}
/// **Measured** 2026-09-07: the New Order selector's own cancel word.
pub const NEW_ORDER_CANCEL_REPLY: &str = "nobody!";

/// `inventory.md §4.7` page-loop sub-prompt literal `\nStatus:_` — the
/// line the Z-stats page loop leaves open in the message window while it
/// waits for a page key.
pub const Z_STATS_STATUS_PROMPT: &str = "Status: ";
/// `inventory.md §4.7`: "Leaving the pages prints `Done\n` in the
/// message window."
pub const Z_STATS_DONE_MESSAGE: &str = "Done";

/// `inventory.md §4.7` attribute-page label literals. Underscores in the
/// published table are literal spaces and `\n` a newline; the labels
/// carry their own interior spacing, which is why nothing here pads.
pub const Z_STATS_LEVEL_LABEL: &str = " Lv-";
pub const Z_STATS_STRENGTH_LABEL: &str = "Str=";
pub const Z_STATS_HP_LABEL: &str = "  HP:";
pub const Z_STATS_INTELLIGENCE_LABEL: &str = "Int=";
/// `inventory.md §4.7` (corrected 2026-10-25, issue #302 Q2, `RETRACTIONS.md`
/// R622): "`__HM:` | **Maximum** hit points (four cells, space-padded)", and
/// `\n\n____Magic:` is followed by "Current magic points, printed on the same
/// line right after the label". The earlier table's magic-point pairing is
/// retracted; the engine had followed a capture of the original that agreed
/// with the correction (playtest pass 10 R03).
pub const Z_STATS_MAX_HP_LABEL: &str = "  HM:";
pub const Z_STATS_DEXTERITY_LABEL: &str = "Dex=";
pub const Z_STATS_EXPERIENCE_LABEL: &str = "  Ex:";
pub const Z_STATS_MAGIC_LABEL: &str = "    Magic:";
/// `inventory.md §4.7` field widths (issue #302 Q2): "magic points take two
/// cells, space-padded (`Magic:_7`, `Magic:25`)", straight after the colon.
pub const Z_STATS_MAGIC_VALUE_CELLS: usize = 2;
/// `inventory.md §4.7` equipment-page heading literal `Arms\n\n`.
pub const Z_STATS_ARMS_HEADING: &str = "Arms";
/// `inventory.md §4.5` moonstone row sentinel: "The word `Moonstone_`,
/// then a single runic letter naming the stone."
pub const Z_STATS_MOONSTONE_LABEL: &str = "Moonstone";
/// `inventory.md §4.7` literal table: the `Equipment` heading, which the
/// stock game uses as the counters screen's border label.
pub const Z_STATS_EQUIPMENT_HEADING: &str = "Equipment";

/// `inventory.md §4.7` (issue #302 Q2, R622): "The level-line indent, in
/// spaces, by class: Avatar 1, Mage 2, Bard 2, Fighter 1, Druid 1, Tinker 1,
/// Paladin 1, Ranger 1, Shepherd 0. It comes from a per-class table in the
/// shipped data, not from the name length." The level line is the text
/// window's first row; it is not centred.
pub const fn z_stats_level_line_indent(class: crate::CharacterClass) -> usize {
    match class {
        crate::CharacterClass::Mage | crate::CharacterClass::Bard => 2,
        crate::CharacterClass::Shepherd => 0,
        crate::CharacterClass::Avatar
        | crate::CharacterClass::Fighter
        | crate::CharacterClass::Druid
        | crate::CharacterClass::Tinker
        | crate::CharacterClass::Paladin
        | crate::CharacterClass::Ranger => 1,
    }
}

/// Width of the attribute page's right-hand numeric column.
///
/// Runtime observation, spec silent: `inventory.md §4.7` calls the page
/// "label-driven, not column-driven" and publishes no field width. A
/// capture of the original's Avatar sheet puts `HP: 60` and `Ex: 150`
/// with their last digit in screen column 38 — the same right edge the
/// roster rows use — so the value after `__HP:` / `__HM:` / `__Ex:` is
/// right-justified in four cells.
pub const Z_STATS_ATTRIBUTE_VALUE_CELLS: usize = 4;
/// Digits the left attribute column prints, zero-padded.
///
/// Measured 2026-09-09 (`qa/paired/zstats-pages.tsv`, page `p1`): the original
/// prints `Int=05`, not `Int=5`.
pub const Z_STATS_ATTRIBUTE_LEFT_DIGITS: usize = 2;

/// The condition line beneath the name on the attribute page.
///
/// `inventory.md §4.7` keys the name off "the five-entry status table keyed
/// by the status letter", and §4.2 publishes that alphabet as `G P D S C`,
/// looked up so that "an out-of-range byte yields no letter rather than a
/// wrong one". A byte outside it - `A` included - therefore prints nothing
/// (playtest P17-08).
///
/// The specification does not publish the five table entries or the
/// `DATA.OVL` record that holds them, so the clean engine prints no status
/// name for any letter until it does (a spec question asks for the table or
/// its record). The row itself, and the centring a status name would get,
/// are unchanged.
pub fn z_stats_health_line(status: u8) -> &'static str {
    let _ = status;
    ""
}

/// Rows of the stats panel a Z-stats page body owns: screen rows 1..=9
/// of window 1 (`inventory.md §4.1`).
pub const Z_STATS_PANEL_ROWS: usize = 9;

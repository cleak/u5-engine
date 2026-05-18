//! Runtime Z-stats browser state.

use crate::*;

pub const Z_STATS_INVENTORY_PANEL_ROWS: usize = 8;
pub const READY_PICKER_PANEL_ROWS: usize = 8;
pub const USE_PICKER_PANEL_ROWS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZStatsPage {
    Stats,
    Equipment,
    SpellBook,
    Reagents,
    Spells,
    SpecialUse,
    EquipmentStock,
}

impl ZStatsPage {
    pub const ORDERED: [Self; 7] = [
        Self::Stats,
        Self::Equipment,
        Self::SpellBook,
        Self::Reagents,
        Self::Spells,
        Self::SpecialUse,
        Self::EquipmentStock,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Stats => "Stats",
            Self::Equipment => "Equipment",
            Self::SpellBook => "Spell Book",
            Self::Reagents => "Reagents",
            Self::Spells => "Spell Charges",
            Self::SpecialUse => "Special/Use Items",
            Self::EquipmentStock => "Weapons/Armour Stock",
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CastFollowupKind {
    Direction { pass_allowed: bool },
    PartyTarget,
    GatePhase,
    CombatTarget { creature: bool },
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
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShrineSession {
    pub virtue: ShrineVirtue,
    pub phase: ShrinePhase,
    pub mantra_buffer: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShrinePhase {
    Mantra,
    Offering,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectionPromptSession {
    pub kind: DirectionPromptKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DirectionPromptKind {
    Attack,
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
    Open,
    Push,
    Search,
    Talk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct YesNoPromptSession {
    pub kind: YesNoPromptKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YesNoPromptKind {
    SaveGame,
    ExitToDos,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsePendingAction {
    PotionTarget { index: usize },
    ScrollWindDirection { index: usize },
    ScrollResurrectionTarget { index: usize },
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

    pub fn move_next_page(&mut self) {
        self.page = self.page.next();
        self.inventory_cursor = 0;
    }

    pub fn move_previous_page(&mut self) {
        self.page = self.page.previous();
        self.inventory_cursor = 0;
    }

    pub fn select_party_index(&mut self, party_index: usize) {
        self.selected_party_index = party_index;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZStatsInputAction {
    Exit,
    NextPage,
    PreviousPage,
    SelectParty(usize),
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
        Self { direction }
    }
}

impl ShrineSession {
    pub fn new(virtue: ShrineVirtue) -> Self {
        Self {
            virtue,
            phase: ShrinePhase::Mantra,
            mantra_buffer: String::new(),
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
        ' ' | '\u{1b}' => ReadyInputAction::Exit,
        '\r' | '\n' => ReadyInputAction::Confirm,
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
        ' ' | '\u{1b}' => UseInputAction::Exit,
        '\r' | '\n' => UseInputAction::Confirm,
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

pub fn z_stats_input_action(key: char) -> ZStatsInputAction {
    match key {
        ' ' | '\u{1b}' => ZStatsInputAction::Exit,
        '>' | '+' | '\r' | '\n' => ZStatsInputAction::NextPage,
        '<' | '-' => ZStatsInputAction::PreviousPage,
        '1'..='6' => ZStatsInputAction::SelectParty((key as u8 - b'1') as usize),
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

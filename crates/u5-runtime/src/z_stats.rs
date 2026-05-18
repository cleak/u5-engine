//! Runtime Z-stats browser state.

use crate::*;

pub const Z_STATS_INVENTORY_PANEL_ROWS: usize = 8;
pub const READY_PICKER_PANEL_ROWS: usize = 8;
pub const USE_PICKER_PANEL_ROWS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZStatsPage {
    Stats,
    Equipment,
    Reagents,
    Spells,
    SpecialUse,
    EquipmentStock,
}

impl ZStatsPage {
    pub const ORDERED: [Self; 6] = [
        Self::Stats,
        Self::Equipment,
        Self::Reagents,
        Self::Spells,
        Self::SpecialUse,
        Self::EquipmentStock,
    ];

    pub const fn title(self) -> &'static str {
        match self {
            Self::Stats => "Stats",
            Self::Equipment => "Equipment",
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

//! Special-rule classifier for the three gated `LOOK2.DAT` hidden
//! treasure records per `hidden-treasures.md` §2.

/// `hidden-treasures.md §2` record indices that have special staging
/// rules beyond the ordinary one-shot found-bitmap protection.
pub const HIDDEN_TREASURE_RECORD_KEY_NPC_GATED: usize = 13;
pub const HIDDEN_TREASURE_RECORD_DAILY_CACHE: usize = 14;
pub const HIDDEN_TREASURE_RECORD_SINGLE_USE_NPC_GATED: usize = 15;

/// `hidden-treasures.md §2` record 13 stage-acceptance gate. Search
/// stages this record only when the party owns at least one
/// ordinary key and the searched cell is not occupied by an NPC.
pub const fn hidden_treasure_record_13_accepts(keys: u8, npc_present: bool) -> bool {
    keys >= 1 && !npc_present
}

/// `hidden-treasures.md §2` record 14 daily-cooldown gate. Search
/// stages the record at most once per in-game day; the saved
/// cooldown cookie holds the last day the record fired (or
/// `FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY` = 0 when never staged).
/// A successful stage stores the current day; subsequent searches
/// the same day are rejected.
pub const fn hidden_treasure_record_14_ready(stored_day: u8, current_day: u8) -> bool {
    stored_day != current_day
}

/// `hidden-treasures.md §2` record 15 stage-acceptance gate. Search
/// stages this record only when its single-use flag is still clear
/// and the searched cell is not occupied by an NPC.
pub const fn hidden_treasure_record_15_accepts(single_use_flag: bool, npc_present: bool) -> bool {
    !single_use_flag && !npc_present
}

/// `hidden-treasures.md §3` distinct pickup classes that appear in
/// the fixed 113-record table. The class drives Get's downstream
/// inventory-add dispatch; the State column is per-record subtype.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasurePickupClass {
    Armour,
    Weapon,
    Scroll,
    RingOfKeys,
    Gem,
    Potion,
    Food,
    Torches,
    Ring,
    MoldyCorpse,
    RottingBody,
    SackOfGold,
    Amulet,
}

/// `hidden-treasures.md §3` co-located underworld stack: records
/// `0..=11` are all staged at the underworld plane at `(233, 233)`,
/// alternating Armour state 15 (even indices) and Weapon state 41
/// (odd indices). The shared coordinate forms a one-shot stack that
/// drains on repeated successful Search at the same point.
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_FIRST: usize = 0;
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_LAST: usize = 11;
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_LEN: usize = 12;
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_FLOOR: u8 = 255;
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_X: u8 = 233;
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_Y: u8 = 233;
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_ARMOUR_STATE: u8 = 15;
pub const HIDDEN_TREASURE_UNDERWORLD_STACK_WEAPON_STATE: u8 = 41;

/// `hidden-treasures.md §3`: returns the pickup class and per-record
/// State byte for any record in the underworld stack. Returns `None`
/// for records outside the `0..=11` stack range.
pub const fn underworld_stack_record(
    record_index: usize,
) -> Option<(HiddenTreasurePickupClass, u8)> {
    if record_index > HIDDEN_TREASURE_UNDERWORLD_STACK_LAST {
        return None;
    }
    if record_index % 2 == 0 {
        Some((
            HiddenTreasurePickupClass::Armour,
            HIDDEN_TREASURE_UNDERWORLD_STACK_ARMOUR_STATE,
        ))
    } else {
        Some((
            HiddenTreasurePickupClass::Weapon,
            HIDDEN_TREASURE_UNDERWORLD_STACK_WEAPON_STATE,
        ))
    }
}

/// `hidden-treasures.md §2` per-record special-rule classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasureRule {
    /// Records 0..=12 and 16..=112 (everything except 13/14/15) — one
    /// shot only, prevented from staging again by the save-backed
    /// found bitmap after a successful Search.
    OneShot,
    /// Record 13 — requires the party to own at least one key and the
    /// searched tile not to be occupied by an NPC.
    KeyAndNpcAbsence,
    /// Record 14 — daily cache: can stage once per in-game day; success
    /// stores the current day as the cooldown cookie.
    DailyCache,
    /// Record 15 — requires its single-use flag to be clear and the
    /// searched tile not to be occupied by an NPC.
    SingleUseAndNpcAbsence,
}

/// `hidden-treasures.md §2`: classify a record's staging rule.
pub const fn hidden_treasure_rule(record_index: usize) -> HiddenTreasureRule {
    match record_index {
        HIDDEN_TREASURE_RECORD_KEY_NPC_GATED => HiddenTreasureRule::KeyAndNpcAbsence,
        HIDDEN_TREASURE_RECORD_DAILY_CACHE => HiddenTreasureRule::DailyCache,
        HIDDEN_TREASURE_RECORD_SINGLE_USE_NPC_GATED => HiddenTreasureRule::SingleUseAndNpcAbsence,
        _ => HiddenTreasureRule::OneShot,
    }
}

/// `hidden-treasures.md §2`: predicate combining the rule check with
/// caller-provided context for record 13 (key + NPC), 14 (daily
/// cooldown), and 15 (single-use flag + NPC). For record 13 the caller
/// passes the party key count; for 14 the cooldown cookie + current
/// day; for 15 the saved single-use flag. Records outside the gated
/// set return `true` because the ordinary one-shot bitmap is owned by
/// the caller, not by this rule.
pub const fn hidden_treasure_can_stage(
    record_index: usize,
    keys: u8,
    tile_has_npc: bool,
    cooldown_day_cookie: u8,
    current_day: u8,
    single_use_flag_clear: bool,
) -> bool {
    match hidden_treasure_rule(record_index) {
        HiddenTreasureRule::OneShot => true,
        HiddenTreasureRule::KeyAndNpcAbsence => keys >= 1 && !tile_has_npc,
        HiddenTreasureRule::DailyCache => cooldown_day_cookie != current_day,
        HiddenTreasureRule::SingleUseAndNpcAbsence => single_use_flag_clear && !tile_has_npc,
    }
}

//! Special-rule classifier for the three gated `LOOK2.DAT` hidden
//! treasure records per `hidden-treasures.md` §2.

/// `hidden-treasures.md §2` record indices that have special staging
/// rules beyond the ordinary one-shot found-bitmap protection.
pub const HIDDEN_TREASURE_RECORD_KEY_NPC_GATED: usize = 13;
pub const HIDDEN_TREASURE_RECORD_DAILY_CACHE: usize = 14;
pub const HIDDEN_TREASURE_RECORD_SINGLE_USE_NPC_GATED: usize = 15;

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
        HIDDEN_TREASURE_RECORD_SINGLE_USE_NPC_GATED => {
            HiddenTreasureRule::SingleUseAndNpcAbsence
        }
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

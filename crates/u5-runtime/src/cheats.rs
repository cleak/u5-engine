//! Cheats and debug tools: the runtime rules (Phase B design 7.3.3, 7.3.8;
//! build plan M8-01).
//!
//! This module holds the engine half of chapter 7:
//!
//! - the limits table, every value cited to the specification and reusing
//!   the engine's own anchored constants, so the cheats and the game can
//!   never disagree;
//! - [`CheatEdit`] and [`apply_cheat_edit`], the quick actions and the
//!   Edit party and stores edits ([`MemberEdit`], [`StoresEdit`],
//!   [`StoredStatus`]), with [`apply_cheat_edits`] for the page's Apply;
//! - [`EngagedLocks`], [`StoreFields`], the `apply_*_lock` functions and
//!   [`apply_locks`];
//! - the slot-0 mirror-coherent stat setters;
//! - [`WatchField`], [`WatchValue`], [`watch_snapshot`] and [`watch_diff`],
//!   the read-only change watch;
//! - [`plan_teleport`] and [`commit_teleport`], the teleport (7.3.6, D7.8,
//!   D7.9; build plan M8-04).
//!
//! Every function here except the teleport is deterministic: no I/O, no
//! randomness, no clock or turn change. The teleport reads the game
//! directory and, like any transit out of the overworld, writes the
//! departing plane's `.OOL` mirror; it changes neither the clock nor the
//! turn. Nothing in the game calls this module; only the cheats
//! plugin in the shell does, and only while a cheat is on (7.4.2). Every
//! label is engine text.

use std::collections::BTreeMap;
use std::io;
use std::path::Path;

use crate::*;

// ---------------------------------------------------------------------------
// The limits table (7.3.3).
// ---------------------------------------------------------------------------

/// Food target and cap: "word-sized party food counter caps at 9999"
/// (`systems/inventory.md §2`).
pub const CHEAT_FOOD_MAX: u16 = PARTY_FOOD_CAP;
/// Gold target and cap: "word-sized party gold counter caps at 9999"
/// (`systems/inventory.md §2`).
pub const CHEAT_GOLD_MAX: u16 = PARTY_GOLD_CAP;
/// Keys, gems, torches, arrows (item 27), quarrels (item 29) and the eight
/// reagents: "byte-sized carried commodity ... stocks commonly cap at 99"
/// (`systems/inventory.md §2`, `formats/saved-gam.md §7`).
pub const CHEAT_BYTE_STOCK_MAX: u8 = PARTY_BYTE_STOCK_CAP;
/// The lowest primary stat an edit may set. An engine choice, so that no
/// Dexterity-divided rule sees zero (D7.15).
pub const CHEAT_STAT_MIN: u8 = 1;
/// The highest primary stat: the stat reward is "increased by one with a
/// cap of 30" (`systems/rest-and-camp.md §7`).
pub const CHEAT_STAT_MAX: u8 = AVATAR_STAT_MAX;
/// Experience credit caps at 9,999 (`systems/stat-arithmetic.md §3`,
/// `systems/combat.md §12`).
pub const CHEAT_EXPERIENCE_MAX: u16 = COMBAT_EXPERIENCE_CAP;
/// The hull a repair sets: the value a delivered frigate starts with
/// (`systems/shops.md §8.7`, `systems/vehicles.md §4`). No cheat ever
/// raises a hull to 100 or more (7.2.4).
pub const CHEAT_HULL_TARGET: u8 = FRIGATE_PURCHASE_HULL;
/// The party's own frigates in the outdoor table, sails up or furled
/// (`systems/active-objects.md §3`). Pirate ships
/// ([`OUTDOOR_PIRATE_TYPE_FIRST`]..=[`OUTDOOR_PIRATE_TYPE_LAST`]) are
/// outside this range and are never touched.
pub const CHEAT_PARTY_FRIGATE_TYPES: std::ops::RangeInclusive<u8> =
    SHIP_TRANSPORT_HOISTED_FIRST..=SHIP_TRANSPORT_FURLED_LAST;

/// One row of the limits table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheatLimit {
    Food,
    Gold,
    ByteStock,
    PrimaryStat,
    Experience,
    Hull,
}

/// Every row of the limits table, in 7.3.3's order.
pub const CHEAT_LIMITS: [CheatLimit; 6] = [
    CheatLimit::Food,
    CheatLimit::Gold,
    CheatLimit::ByteStock,
    CheatLimit::PrimaryStat,
    CheatLimit::Experience,
    CheatLimit::Hull,
];

impl CheatLimit {
    /// The inclusive range an edit may set.
    pub const fn range(self) -> (u32, u32) {
        match self {
            CheatLimit::Food => (0, CHEAT_FOOD_MAX as u32),
            CheatLimit::Gold => (0, CHEAT_GOLD_MAX as u32),
            CheatLimit::ByteStock => (0, CHEAT_BYTE_STOCK_MAX as u32),
            CheatLimit::PrimaryStat => (CHEAT_STAT_MIN as u32, CHEAT_STAT_MAX as u32),
            CheatLimit::Experience => (0, CHEAT_EXPERIENCE_MAX as u32),
            CheatLimit::Hull => (0, CHEAT_HULL_TARGET as u32),
        }
    }

    /// The specification section the limit comes from.
    pub const fn source(self) -> &'static str {
        match self {
            CheatLimit::Food | CheatLimit::Gold | CheatLimit::ByteStock => {
                "systems/inventory.md §2"
            }
            CheatLimit::PrimaryStat => "systems/rest-and-camp.md §7",
            CheatLimit::Experience => "systems/stat-arithmetic.md §3",
            CheatLimit::Hull => "systems/shops.md §8.7",
        }
    }

    /// `Ok` when `value` is in range, else the refusal that names it.
    pub fn check(self, value: u32) -> Result<(), CheatRefusal> {
        let (min, max) = self.range();
        if (min..=max).contains(&value) {
            Ok(())
        } else {
            Err(CheatRefusal::OutOfRange { min, max })
        }
    }
}

/// The level `[L]` recomputes from experience: the camp rule, level 1
/// below 100 XP, then one more for each halving step
/// (`systems/rest-and-camp.md §7`).
pub const fn cheat_level_for_experience(experience: u16) -> u8 {
    level_for_experience(experience as u32)
}

/// Maximum (and current) HP for a recomputed level: `30 * level`
/// (`systems/rest-and-camp.md §7`).
pub const fn cheat_hp_for_level(level: u8) -> u16 {
    lord_british_camp_event_hp_for_level(level)
}

/// The published mana target of a class: Intelligence for Avatar and
/// Mage, half Intelligence rounded down for Bard, none for the others
/// (`systems/rest-and-camp.md §7`, `systems/magic.md §11`).
pub const fn cheat_mana_target(class_byte: u8, intelligence: u8) -> Option<u8> {
    lord_british_camp_refreshed_mana(class_byte, intelligence)
}

// ---------------------------------------------------------------------------
// The edit API (7.3.3).
// ---------------------------------------------------------------------------

/// A quick action (7.2.4), or an Edit party and stores edit (7.2.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheatEdit {
    HealAll,
    CurePoison,
    ResurrectAll,
    MaxGold,
    MaxFood,
    RefillArrows {
        quarrels: bool,
    },
    MaxReagents,
    RepairHull {
        parked: bool,
    },
    /// The changed fields of one travelling member.
    Member {
        slot: usize,
        fields: MemberEdit,
    },
    /// The changed shared counters.
    Stores(StoresEdit),
    /// `[L]`: the camp's level recomputation from experience, with maximum
    /// HP set to `30 * level` (7.2.6).
    RecomputeLevel {
        slot: usize,
    },
}

/// The fields of one member an edit sets; `None` leaves a field exactly as
/// it is, so an untouched out-of-range value is preserved (7.2.6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MemberEdit {
    pub strength: Option<u8>,
    pub dexterity: Option<u8>,
    pub intelligence: Option<u8>,
    pub hp: Option<u16>,
    pub mana: Option<u8>,
    pub experience: Option<u16>,
    pub status: Option<StoredStatus>,
}

impl MemberEdit {
    /// Whether no field is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// A status the editor may choose (7.2.6). Ashes (`A`) is shown and
/// preserved but cannot be chosen, and charmed (`C`) is a display override
/// that is never stored (`formats/saved-gam.md §3.2`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoredStatus {
    Good,
    Poisoned,
    Sleeping,
    Dead,
}

impl StoredStatus {
    /// The choices, in the editor's order.
    pub const ALL: [StoredStatus; 4] = [
        StoredStatus::Good,
        StoredStatus::Poisoned,
        StoredStatus::Sleeping,
        StoredStatus::Dead,
    ];

    /// The stored status letter.
    pub const fn byte(self) -> u8 {
        match self {
            StoredStatus::Good => b'G',
            StoredStatus::Poisoned => b'P',
            StoredStatus::Sleeping => b'S',
            StoredStatus::Dead => b'D',
        }
    }

    /// The choice a stored letter is, if it is one.
    pub fn of_byte(byte: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|status| status.byte() == byte)
    }
}

/// The shared counters an edit sets; `None` leaves a counter as it is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoresEdit {
    pub food: Option<u16>,
    pub gold: Option<u16>,
    pub keys: Option<u8>,
    pub gems: Option<u8>,
    pub torches: Option<u8>,
    /// Item 27.
    pub arrows: Option<u8>,
    /// Item 29.
    pub quarrels: Option<u8>,
    /// In the mixing order of `formats/saved-gam.md §7`.
    pub reagents: [Option<u8>; REAGENT_COUNT],
}

impl StoresEdit {
    /// Whether no counter is set.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }
}

/// Why an edit did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheatRefusal {
    NotAtIdlePrompt,
    InCombat,
    NotAboard,
    NothingToDo,
    OutOfRange { min: u32, max: u32 },
    NoSuchMember,
}

/// The page's result line for each refusal (engine text).
const REFUSED_NOT_NOW: &str = "Not now";
const REFUSED_IN_COMBAT: &str = "Not during combat";
const REFUSED_NOT_ABOARD: &str = "Not aboard a ship";
const REFUSED_NOTHING: &str = "Nothing to change";
const REFUSED_NO_MEMBER: &str = "No such member";

impl CheatRefusal {
    /// The page's result line for this refusal (engine text).
    pub fn message(self) -> String {
        match self {
            CheatRefusal::NotAtIdlePrompt => REFUSED_NOT_NOW.to_string(),
            CheatRefusal::InCombat => REFUSED_IN_COMBAT.to_string(),
            CheatRefusal::NotAboard => REFUSED_NOT_ABOARD.to_string(),
            CheatRefusal::NothingToDo => REFUSED_NOTHING.to_string(),
            CheatRefusal::OutOfRange { min, max } => format!("Range {min}..{max}"),
            CheatRefusal::NoSuchMember => REFUSED_NO_MEMBER.to_string(),
        }
    }
}

/// What an edit changed.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EditReport {
    pub changed: bool,
    pub members_touched: u8,
    /// The stats panel must be repainted (7.3.5).
    pub panel_refresh: bool,
}

/// Whether a stored status is Dead (`D`) or Ashes (`A`), the two a heal
/// never touches (`formats/saved-gam.md §3.2`).
fn dead_or_ashes(status: u8) -> bool {
    matches!(status, b'D' | b'A')
}

/// Raises `value` to `target`; never lowers. Returns whether it changed.
fn raise<T: PartialOrd + Copy>(value: &mut T, target: T) -> bool {
    if *value < target {
        *value = target;
        true
    } else {
        false
    }
}

/// Heals one member as Heal all does: HP to max unless Dead or Ashes, and
/// outside combat `P` and `S` to `G`. Never lowers an HP above max.
fn heal_member(member: &mut PartyMember, in_combat: bool) -> bool {
    if dead_or_ashes(member.status) {
        return false;
    }
    let mut changed = raise(&mut member.hp, member.max_hp);
    if !in_combat && matches!(member.status, b'P' | b'S') {
        member.status = b'G';
        changed = true;
    }
    changed
}

/// Apply one quick action. No I/O, no PRNG draw, no clock or turn change.
///
/// Checks, in order: the idle prompt (either kind), then combat for the
/// edits that write a status byte. An edit that changes nothing returns
/// [`CheatRefusal::NothingToDo`], so the page can say so without marking
/// the game modified.
pub fn apply_cheat_edit(
    state: &mut PlayState,
    edit: &CheatEdit,
) -> Result<EditReport, CheatRefusal> {
    if state.idle_command_prompt().is_none() {
        return Err(CheatRefusal::NotAtIdlePrompt);
    }
    let in_combat = state.combat_active;
    let writes_status = match edit {
        CheatEdit::CurePoison | CheatEdit::ResurrectAll => true,
        CheatEdit::Member { fields, .. } => fields.status.is_some(),
        _ => false,
    };
    if in_combat && writes_status {
        return Err(CheatRefusal::InCombat);
    }
    let mut members_touched = 0u8;
    let changed = match edit {
        CheatEdit::HealAll => {
            for member in &mut state.party {
                if heal_member(member, in_combat) {
                    members_touched += 1;
                }
            }
            let hull = repair_boarded_hull(state);
            members_touched > 0 || hull
        }
        CheatEdit::CurePoison => {
            for member in &mut state.party {
                if member.status == b'P' {
                    member.status = b'G';
                    members_touched += 1;
                }
            }
            members_touched > 0
        }
        CheatEdit::ResurrectAll => {
            // Exactly `D`: Ashes is preserved (D7.7); level, experience
            // and maximum HP are unchanged (D7.4).
            for member in &mut state.party {
                if member.status == b'D' {
                    member.status = b'G';
                    member.hp = member.max_hp;
                    members_touched += 1;
                }
            }
            members_touched > 0
        }
        CheatEdit::MaxGold => raise(&mut state.gold, CHEAT_GOLD_MAX),
        CheatEdit::MaxFood => raise(&mut state.food, CHEAT_FOOD_MAX),
        CheatEdit::RefillArrows { quarrels } => {
            let arrows = raise(
                &mut state.equipment_stock[EQUIPMENT_ID_ARROWS],
                CHEAT_BYTE_STOCK_MAX,
            );
            let quarrels = *quarrels
                && raise(
                    &mut state.equipment_stock[EQUIPMENT_ID_QUARRELS],
                    CHEAT_BYTE_STOCK_MAX,
                );
            arrows || quarrels
        }
        CheatEdit::MaxReagents => state.reagents.iter_mut().fold(false, |changed, count| {
            raise(count, CHEAT_BYTE_STOCK_MAX) | changed
        }),
        CheatEdit::RepairHull { parked } => {
            let aboard = matches!(state.player.transport, TransportState::Ship { .. });
            let boarded = repair_boarded_hull(state);
            let parked_changed = *parked && repair_parked_frigates(state);
            if !aboard && !parked_changed {
                return Err(CheatRefusal::NotAboard);
            }
            boarded || parked_changed
        }
        CheatEdit::Member { slot, fields } => {
            let changed = edit_member(state, *slot, fields)?;
            members_touched = u8::from(changed);
            changed
        }
        CheatEdit::Stores(stores) => edit_stores(state, stores)?,
        CheatEdit::RecomputeLevel { slot } => {
            let changed = recompute_level(state, *slot)?;
            members_touched = u8::from(changed);
            changed
        }
    };
    if !changed {
        return Err(CheatRefusal::NothingToDo);
    }
    Ok(EditReport {
        changed,
        members_touched,
        panel_refresh: true,
    })
}

/// The page's Apply: every edit in order, as one change (7.2.6). The edits
/// run on a copy, so a refusal anywhere leaves `state` exactly as it was;
/// an edit that changes nothing is skipped. When none changes anything the
/// result is [`CheatRefusal::NothingToDo`], so the session stays untainted.
pub fn apply_cheat_edits(
    state: &mut PlayState,
    edits: &[CheatEdit],
) -> Result<EditReport, CheatRefusal> {
    if state.idle_command_prompt().is_none() {
        return Err(CheatRefusal::NotAtIdlePrompt);
    }
    let mut draft = state.clone();
    let mut report = EditReport::default();
    for edit in edits {
        match apply_cheat_edit(&mut draft, edit) {
            Ok(one) => {
                report.changed = true;
                report.panel_refresh = true;
                report.members_touched = report.members_touched.saturating_add(one.members_touched);
            }
            Err(CheatRefusal::NothingToDo) => {}
            Err(refusal) => return Err(refusal),
        }
    }
    if !report.changed {
        return Err(CheatRefusal::NothingToDo);
    }
    *state = draft;
    Ok(report)
}

/// Checks `value` against an inclusive range.
fn check_range(value: u32, min: u32, max: u32) -> Result<(), CheatRefusal> {
    if (min..=max).contains(&value) {
        Ok(())
    } else {
        Err(CheatRefusal::OutOfRange { min, max })
    }
}

/// The range a member's current HP may be set to: `1..=max` while the
/// member lives, none once Dead or Ashes (7.2.6).
pub fn member_hp_range(status: u8, max_hp: u16) -> Option<(u16, u16)> {
    (!dead_or_ashes(status)).then_some((1, max_hp))
}

/// Checks every field of a member edit against the limits table and the
/// member as the edit leaves it, before anything is written.
fn check_member(
    state: &PlayState,
    slot: usize,
    fields: &MemberEdit,
) -> Result<PartyMember, CheatRefusal> {
    let member = *state.party.get(slot).ok_or(CheatRefusal::NoSuchMember)?;
    for stat in [fields.strength, fields.dexterity, fields.intelligence]
        .into_iter()
        .flatten()
    {
        CheatLimit::PrimaryStat.check(u32::from(stat))?;
    }
    if let Some(experience) = fields.experience {
        CheatLimit::Experience.check(u32::from(experience))?;
    }
    if fields.status.is_some() && member.status == b'A' {
        // Ashes is preserved: it cannot be changed, as it cannot be chosen.
        return Err(CheatRefusal::NothingToDo);
    }
    let status = fields.status.map_or(member.status, StoredStatus::byte);
    let none = CheatRefusal::OutOfRange { min: 0, max: 0 };
    if let Some(hp) = fields.hp {
        let (min, max) = member_hp_range(status, member.max_hp).ok_or(none)?;
        check_range(u32::from(hp), u32::from(min), u32::from(max))?;
    }
    if let Some(mana) = fields.mana {
        let intelligence = match fields.intelligence {
            Some(value) => value,
            None => member_stat(state, slot, PrimaryStat::Intelligence).unwrap_or(0),
        };
        let target = cheat_mana_target(member.class_byte, intelligence).ok_or(none)?;
        check_range(u32::from(mana), 0, u32::from(target))?;
    }
    Ok(member)
}

/// Writes one member's edit, every mirror together (7.3.3 step 3). The
/// status goes first: `D` sets HP to 0, and a living status on a member
/// with no HP (a Dead member made Good) sets HP to max, the Resurrect all
/// rule; a typed HP then overrides it.
fn edit_member(
    state: &mut PlayState,
    slot: usize,
    fields: &MemberEdit,
) -> Result<bool, CheatRefusal> {
    let before = check_member(state, slot, fields)?;
    let experience_before = state.party_experience.get(slot).copied();
    let stats_before = [
        PrimaryStat::Strength,
        PrimaryStat::Dexterity,
        PrimaryStat::Intelligence,
    ]
    .map(|stat| member_stat(state, slot, stat));
    for (stat, value) in [
        (PrimaryStat::Strength, fields.strength),
        (PrimaryStat::Dexterity, fields.dexterity),
        (PrimaryStat::Intelligence, fields.intelligence),
    ] {
        if let Some(value) = value {
            set_member_stat(state, slot, stat, value)?;
        }
    }
    if let Some(experience) = fields.experience
        && let Some(cell) = state.party_experience.get_mut(slot)
    {
        *cell = experience;
    }
    let member = &mut state.party[slot];
    if let Some(status) = fields.status {
        member.status = status.byte();
        if status == StoredStatus::Dead {
            member.hp = 0;
        } else if member.hp == 0 {
            member.hp = member.max_hp;
        }
    }
    if let Some(hp) = fields.hp {
        member.hp = hp;
    }
    if let Some(mana) = fields.mana {
        member.mana = mana;
    }
    let stats_after = [
        PrimaryStat::Strength,
        PrimaryStat::Dexterity,
        PrimaryStat::Intelligence,
    ]
    .map(|stat| member_stat(state, slot, stat));
    Ok(state.party[slot] != before
        || stats_after != stats_before
        || state.party_experience.get(slot).copied() != experience_before)
}

/// Writes the changed shared counters, after checking every one.
fn edit_stores(state: &mut PlayState, stores: &StoresEdit) -> Result<bool, CheatRefusal> {
    for word in [stores.food, stores.gold].into_iter().flatten() {
        CheatLimit::Food.check(u32::from(word))?;
    }
    let bytes = [
        stores.keys,
        stores.gems,
        stores.torches,
        stores.arrows,
        stores.quarrels,
    ];
    for byte in bytes.into_iter().chain(stores.reagents).flatten() {
        CheatLimit::ByteStock.check(u32::from(byte))?;
    }
    fn put<T: PartialEq + Copy>(cell: &mut T, value: Option<T>) -> bool {
        match value {
            Some(value) if *cell != value => {
                *cell = value;
                true
            }
            _ => false,
        }
    }
    let mut changed = put(&mut state.food, stores.food);
    changed |= put(&mut state.gold, stores.gold);
    changed |= put(&mut state.keys, stores.keys);
    changed |= put(&mut state.gems, stores.gems);
    changed |= put(&mut state.torches, stores.torches);
    changed |= put(
        &mut state.equipment_stock[EQUIPMENT_ID_ARROWS],
        stores.arrows,
    );
    changed |= put(
        &mut state.equipment_stock[EQUIPMENT_ID_QUARRELS],
        stores.quarrels,
    );
    for (count, value) in state.reagents.iter_mut().zip(stores.reagents) {
        changed |= put(count, value);
    }
    Ok(changed)
}

/// `[L]`: level from experience, maximum HP `30 * level`, and current HP
/// the same for a living member (a Dead or Ashes member keeps 0 HP, so the
/// living test stays consistent). No stat reward is applied (7.2.6).
fn recompute_level(state: &mut PlayState, slot: usize) -> Result<bool, CheatRefusal> {
    let experience = *state
        .party_experience
        .get(slot)
        .ok_or(CheatRefusal::NoSuchMember)?;
    let member = state
        .party
        .get_mut(slot)
        .ok_or(CheatRefusal::NoSuchMember)?;
    let before = *member;
    member.level = cheat_level_for_experience(experience);
    member.max_hp = cheat_hp_for_level(member.level);
    if !dead_or_ashes(member.status) {
        member.hp = member.max_hp;
    }
    Ok(*member != before)
}

/// Raises the boarded frigate's hull to [`CHEAT_HULL_TARGET`], then
/// mirrors the vessel into slot zero (7.3.3 step 4). A hull already at 99
/// or above is never changed.
fn repair_boarded_hull(state: &mut PlayState) -> bool {
    let TransportState::Ship { hull, .. } = &mut state.player.transport else {
        return false;
    };
    if !raise(hull, CHEAT_HULL_TARGET) {
        return false;
    }
    state.sync_player_object();
    true
}

/// Raises every parked party frigate in the current outdoor table to
/// [`CHEAT_HULL_TARGET`] (D7.6). Slot zero is the party itself. Pirate
/// ships are never touched. In combat the table is the arena's, so
/// nothing is parked.
fn repair_parked_frigates(state: &mut PlayState) -> bool {
    if state.combat_active || !matches!(state.area, Area::World { .. }) {
        return false;
    }
    state
        .active_objects
        .iter_mut()
        .skip(1)
        .filter(|object| CHEAT_PARTY_FRIGATE_TYPES.contains(&object.type_byte))
        .fold(false, |changed, object| {
            raise(&mut object.aux1, CHEAT_HULL_TARGET) | changed
        })
}

// ---------------------------------------------------------------------------
// Slot-0 mirror-coherent setters (7.3.3, "Where the state lives").
// ---------------------------------------------------------------------------

/// A primary stat.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryStat {
    Strength,
    Dexterity,
    Intelligence,
}

/// The stat as the save writer will store it: slot zero from
/// `avatar_stats`, the others from the active vectors.
pub fn member_stat(state: &PlayState, slot: usize, stat: PrimaryStat) -> Option<u8> {
    state.party.get(slot)?;
    if slot == 0 {
        return Some(match stat {
            PrimaryStat::Strength => state.avatar_stats.strength,
            PrimaryStat::Dexterity => state.avatar_stats.dexterity,
            PrimaryStat::Intelligence => state.avatar_stats.intelligence,
        });
    }
    match stat {
        PrimaryStat::Strength => state.party_strengths.get(slot).copied(),
        PrimaryStat::Dexterity => Some(state.party[slot].climb_stat),
        PrimaryStat::Intelligence => state.party_intelligence.get(slot).copied(),
    }
}

/// Sets one member's primary stat, keeping every copy in step. For slot
/// zero that is `avatar_stats` with `party_strengths[0]`,
/// `party[0].climb_stat` or `party_intelligence[0]`, the pattern of the
/// camp stat reward. Returns whether anything changed.
pub fn set_member_stat(
    state: &mut PlayState,
    slot: usize,
    stat: PrimaryStat,
    value: u8,
) -> Result<bool, CheatRefusal> {
    if slot >= state.party.len() {
        return Err(CheatRefusal::NoSuchMember);
    }
    CheatLimit::PrimaryStat.check(u32::from(value))?;
    let before = member_stat(state, slot, stat);
    let write = |cell: Option<&mut u8>| {
        if let Some(cell) = cell {
            *cell = value;
        }
    };
    match stat {
        PrimaryStat::Strength => {
            if slot == 0 {
                state.avatar_stats.strength = value;
            }
            write(state.party_strengths.get_mut(slot));
        }
        PrimaryStat::Dexterity => {
            if slot == 0 {
                state.avatar_stats.dexterity = value;
            }
            write(Some(&mut state.party[slot].climb_stat));
        }
        PrimaryStat::Intelligence => {
            if slot == 0 {
                state.avatar_stats.intelligence = value;
            }
            write(state.party_intelligence.get_mut(slot));
        }
    }
    Ok(before != Some(value))
}

// ---------------------------------------------------------------------------
// Locks (7.2.5, 7.3.3).
// ---------------------------------------------------------------------------

/// The locks the player has checked, already masked by
/// `LockPermissions` in the shell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EngagedLocks {
    pub health: bool,
    pub mana: bool,
    pub food: bool,
    pub gold: bool,
    pub keys: bool,
    pub gems: bool,
    pub torches: bool,
    pub arrows: bool,
    /// Item 29 with the arrows (`refill_quarrels`).
    pub quarrels: bool,
    /// One per reagent, in the mixing order of `formats/saved-gam.md §7`.
    pub reagents: [bool; REAGENT_COUNT],
}

impl EngagedLocks {
    /// Whether any lock is engaged.
    pub fn any(&self) -> bool {
        self.health
            || self.mana
            || self.food
            || self.gold
            || self.keys
            || self.gems
            || self.torches
            || self.arrows
            || self.quarrels
            || self.reagents.iter().any(|on| *on)
    }
}

/// Where a lock is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LockContext {
    pub in_combat: bool,
}

/// The shared counters a store lock writes, borrowed from a
/// [`PlayState`] or from plain test values.
#[derive(Debug)]
pub struct StoreFields<'a> {
    pub food: &'a mut u16,
    pub gold: &'a mut u16,
    pub keys: &'a mut u8,
    pub gems: &'a mut u8,
    pub torches: &'a mut u8,
    pub equipment_stock: &'a mut [u8; EQUIPMENT_COUNT],
    pub reagents: &'a mut [u8; REAGENT_COUNT],
}

impl<'a> StoreFields<'a> {
    /// The counters of `state`.
    pub fn of(state: &'a mut PlayState) -> Self {
        Self {
            food: &mut state.food,
            gold: &mut state.gold,
            keys: &mut state.keys,
            gems: &mut state.gems,
            torches: &mut state.torches,
            equipment_stock: &mut state.equipment_stock,
            reagents: &mut state.reagents,
        }
    }
}

/// Raise HP to max for living members; outside combat also clear `P` and
/// `S` to `G` (D7.5). Never resurrects (D7.3), never lowers.
pub fn apply_health_lock(party: &mut [PartyMember], ctx: LockContext) -> bool {
    party.iter_mut().fold(false, |changed, member| {
        heal_member(member, ctx.in_combat) | changed
    })
}

/// Raise MP to the class target (A/M: INT, B: INT/2) for living members.
/// Other classes have no published target and are left alone. Never
/// lowers. `intelligence[0]` is the Avatar's (`avatar_stats`).
pub fn apply_mana_lock(party: &mut [PartyMember], intelligence: &[u8]) -> bool {
    let mut changed = false;
    for (member, int) in party.iter_mut().zip(intelligence) {
        if dead_or_ashes(member.status) {
            continue;
        }
        if let Some(target) = cheat_mana_target(member.class_byte, *int) {
            changed |= raise(&mut member.mana, target);
        }
    }
    changed
}

/// Food, gold, keys, gems, torches, arrows, quarrels and the checked
/// reagents, each raised to its cap.
pub fn apply_store_locks(stores: &mut StoreFields<'_>, locks: &EngagedLocks) -> bool {
    let mut changed = false;
    if locks.food {
        changed |= raise(stores.food, CHEAT_FOOD_MAX);
    }
    if locks.gold {
        changed |= raise(stores.gold, CHEAT_GOLD_MAX);
    }
    if locks.keys {
        changed |= raise(stores.keys, CHEAT_BYTE_STOCK_MAX);
    }
    if locks.gems {
        changed |= raise(stores.gems, CHEAT_BYTE_STOCK_MAX);
    }
    if locks.torches {
        changed |= raise(stores.torches, CHEAT_BYTE_STOCK_MAX);
    }
    if locks.arrows {
        changed |= raise(
            &mut stores.equipment_stock[EQUIPMENT_ID_ARROWS],
            CHEAT_BYTE_STOCK_MAX,
        );
    }
    if locks.arrows && locks.quarrels {
        changed |= raise(
            &mut stores.equipment_stock[EQUIPMENT_ID_QUARRELS],
            CHEAT_BYTE_STOCK_MAX,
        );
    }
    for (count, on) in stores.reagents.iter_mut().zip(locks.reagents) {
        if on {
            changed |= raise(count, CHEAT_BYTE_STOCK_MAX);
        }
    }
    changed
}

/// The intelligence of each travelling member, slot zero from
/// `avatar_stats`, as the camp's class refresh reads it.
fn party_intelligence_values(state: &PlayState) -> Vec<u8> {
    (0..state.party.len())
        .map(|slot| member_stat(state, slot, PrimaryStat::Intelligence).unwrap_or(0))
        .collect()
}

/// Every engaged lock, once; the one entry point the shell calls, once per
/// lock epoch. Does nothing away from an idle prompt.
pub fn apply_locks(state: &mut PlayState, locks: &EngagedLocks) -> bool {
    if state.idle_command_prompt().is_none() {
        return false;
    }
    let ctx = LockContext {
        in_combat: state.combat_active,
    };
    let mut changed = false;
    if locks.health {
        changed |= apply_health_lock(&mut state.party, ctx);
    }
    if locks.mana {
        let intelligence = party_intelligence_values(state);
        changed |= apply_mana_lock(&mut state.party, &intelligence);
    }
    changed |= apply_store_locks(&mut StoreFields::of(state), locks);
    changed
}

// ---------------------------------------------------------------------------
// The change watch (7.3.8).
// ---------------------------------------------------------------------------

/// A named field the watch records. No quest flag, seal, Shadowlord or
/// hideout field exists here (7.2.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WatchField {
    Turn,
    Clock,
    Area,
    Position,
    Facing,
    Transport,
    Hull,
    Wind,
    Food,
    Gold,
    Keys,
    Gems,
    Torches,
    Arrows,
    Reagent(u8),
    MemberStatus(u8),
    MemberHp(u8),
    MemberMaxHp(u8),
    MemberMp(u8),
    MemberStr(u8),
    MemberDex(u8),
    MemberInt(u8),
    MemberXp(u8),
    MemberLevel(u8),
    PartySize,
    ActivePlayer,
    MoralStanding,
    LightCounters,
    ActiveEffect,
    /// Engine group, off by default.
    Prng,
}

/// The Watch page's groups (7.2.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WatchGroup {
    Party,
    Stores,
    Place,
    Time,
    /// PRNG state and the turn counter, for maintainers. Off by default.
    Engine,
}

impl WatchField {
    /// The group the field is shown under.
    pub fn group(self) -> WatchGroup {
        match self {
            WatchField::Turn | WatchField::Prng => WatchGroup::Engine,
            WatchField::Clock | WatchField::LightCounters | WatchField::ActiveEffect => {
                WatchGroup::Time
            }
            WatchField::Area
            | WatchField::Position
            | WatchField::Facing
            | WatchField::Transport
            | WatchField::Hull
            | WatchField::Wind => WatchGroup::Place,
            WatchField::Food
            | WatchField::Gold
            | WatchField::Keys
            | WatchField::Gems
            | WatchField::Torches
            | WatchField::Arrows
            | WatchField::Reagent(_) => WatchGroup::Stores,
            WatchField::MemberStatus(_)
            | WatchField::MemberHp(_)
            | WatchField::MemberMaxHp(_)
            | WatchField::MemberMp(_)
            | WatchField::MemberStr(_)
            | WatchField::MemberDex(_)
            | WatchField::MemberInt(_)
            | WatchField::MemberXp(_)
            | WatchField::MemberLevel(_)
            | WatchField::PartySize
            | WatchField::ActivePlayer
            | WatchField::MoralStanding => WatchGroup::Party,
        }
    }

    /// The field's label on the Watch page (engine text). Members and
    /// reagents count from 1.
    pub fn label(self) -> String {
        let member = |slot: u8, what: &str| format!("Party {} {what}", u32::from(slot) + 1);
        match self {
            WatchField::Reagent(index) => format!("Reagent {}", u32::from(index) + 1),
            WatchField::MemberStatus(slot) => member(slot, "status"),
            WatchField::MemberHp(slot) => member(slot, "HP"),
            WatchField::MemberMaxHp(slot) => member(slot, "max HP"),
            WatchField::MemberMp(slot) => member(slot, "MP"),
            WatchField::MemberStr(slot) => member(slot, "STR"),
            WatchField::MemberDex(slot) => member(slot, "DEX"),
            WatchField::MemberInt(slot) => member(slot, "INT"),
            WatchField::MemberXp(slot) => member(slot, "XP"),
            WatchField::MemberLevel(slot) => member(slot, "level"),
            fixed => fixed.fixed_label().unwrap_or_default().to_string(),
        }
    }

    /// The label of a field that names no slot.
    const fn fixed_label(self) -> Option<&'static str> {
        Some(match self {
            WatchField::Turn => "Turn",
            WatchField::Clock => "Clock",
            WatchField::Area => "Area",
            WatchField::Position => "Position",
            WatchField::Facing => "Facing",
            WatchField::Transport => "Transport",
            WatchField::Hull => "Hull",
            WatchField::Wind => "Wind",
            WatchField::Food => "Food",
            WatchField::Gold => "Gold",
            WatchField::Keys => "Keys",
            WatchField::Gems => "Gems",
            WatchField::Torches => "Torches",
            WatchField::Arrows => "Arrows",
            WatchField::PartySize => "Party size",
            WatchField::ActivePlayer => "Active member",
            WatchField::MoralStanding => "Standing",
            WatchField::LightCounters => "Light",
            WatchField::ActiveEffect => "Effect",
            WatchField::Prng => "Random state",
            _ => return None,
        })
    }
}

/// One recorded value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WatchValue {
    Num(i64),
    Text(String),
}

impl WatchValue {
    /// The value as the Watch page prints it.
    pub fn text(&self) -> String {
        match self {
            WatchValue::Num(number) => number.to_string(),
            WatchValue::Text(text) => text.clone(),
        }
    }

    /// What a field reads as when it is absent from one snapshot.
    pub fn absent() -> Self {
        WatchValue::Text("-".to_string())
    }
}

fn area_text(area: Area) -> String {
    match area {
        Area::World {
            plane: WorldPlane::Britannia,
        } => "Britannia".to_string(),
        Area::World {
            plane: WorldPlane::Underworld,
        } => "Underworld".to_string(),
        Area::Town { scene, floor } => format!("Location {} floor {floor}", scene.byte),
        Area::Dungeon { scene, level } => format!("Dungeon {} level {level}", scene.byte),
    }
}

fn transport_text(transport: TransportState) -> &'static str {
    match transport {
        TransportState::Foot => "on foot",
        TransportState::Horse { .. } => "horse",
        TransportState::Ship {
            sails_hoisted: true,
            ..
        } => "ship, sails up",
        TransportState::Ship { .. } => "ship",
        TransportState::Skiff { .. } => "skiff",
        TransportState::Carpet { .. } => "carpet",
        TransportState::SpriteSuppressed => "none",
    }
}

fn facing_text(facing: Direction) -> &'static str {
    match facing {
        Direction::NorthWest => "north-west",
        Direction::North => "north",
        Direction::NorthEast => "north-east",
        Direction::West => "west",
        Direction::East => "east",
        Direction::SouthWest => "south-west",
        Direction::South => "south",
        Direction::SouthEast => "south-east",
    }
}

fn wind_text(wind: WindState) -> &'static str {
    match wind {
        WindState::Calm => "calm",
        WindState::North => "north",
        WindState::South => "south",
        WindState::East => "east",
        WindState::West => "west",
    }
}

fn status_text(status: u8) -> String {
    if status.is_ascii_uppercase() {
        char::from(status).to_string()
    } else {
        format!("?{status}")
    }
}

/// A pure read of the named fields, in [`WatchField`] order. Takes
/// `&PlayState` only.
pub fn watch_snapshot(state: &PlayState) -> Vec<(WatchField, WatchValue)> {
    use WatchValue::{Num, Text};
    let clock = state.clock;
    let mut fields = vec![
        (WatchField::Turn, Num(state.turn as i64)),
        (
            WatchField::Clock,
            Text(format!(
                "{}-{:02}-{:02} {:02}:{:02}",
                clock.year, clock.month, clock.day, clock.hour, clock.minute
            )),
        ),
        (WatchField::Area, Text(area_text(state.area))),
        (
            WatchField::Position,
            Text(format!("{},{}", state.player.x, state.player.y)),
        ),
        (
            WatchField::Facing,
            Text(facing_text(state.player.facing).to_string()),
        ),
        (
            WatchField::Transport,
            Text(transport_text(state.player.transport).to_string()),
        ),
    ];
    if let TransportState::Ship { hull, .. } = state.player.transport {
        fields.push((WatchField::Hull, Num(i64::from(hull))));
    }
    fields.extend([
        (WatchField::Wind, Text(wind_text(state.wind).to_string())),
        (WatchField::Food, Num(i64::from(state.food))),
        (WatchField::Gold, Num(i64::from(state.gold))),
        (WatchField::Keys, Num(i64::from(state.keys))),
        (WatchField::Gems, Num(i64::from(state.gems))),
        (WatchField::Torches, Num(i64::from(state.torches))),
        (
            WatchField::Arrows,
            Num(i64::from(state.equipment_stock[EQUIPMENT_ID_ARROWS])),
        ),
    ]);
    for (index, count) in state.reagents.iter().enumerate() {
        fields.push((WatchField::Reagent(index as u8), Num(i64::from(*count))));
    }
    let members: Vec<(u8, PartyMember)> = state
        .party
        .iter()
        .take(usize::from(u8::MAX))
        .enumerate()
        .map(|(slot, member)| (slot as u8, *member))
        .collect();
    let stat = |slot: u8, which: PrimaryStat| {
        member_stat(state, usize::from(slot), which)
            .map_or_else(WatchValue::absent, |value| Num(i64::from(value)))
    };
    // Grouped by kind, then by slot, so the list is in `WatchField` order.
    for (slot, member) in &members {
        fields.push((
            WatchField::MemberStatus(*slot),
            Text(status_text(member.status)),
        ));
    }
    for (slot, member) in &members {
        fields.push((WatchField::MemberHp(*slot), Num(i64::from(member.hp))));
    }
    for (slot, member) in &members {
        fields.push((
            WatchField::MemberMaxHp(*slot),
            Num(i64::from(member.max_hp)),
        ));
    }
    for (slot, member) in &members {
        fields.push((WatchField::MemberMp(*slot), Num(i64::from(member.mana))));
    }
    for (slot, _) in &members {
        fields.push((
            WatchField::MemberStr(*slot),
            stat(*slot, PrimaryStat::Strength),
        ));
    }
    for (slot, _) in &members {
        fields.push((
            WatchField::MemberDex(*slot),
            stat(*slot, PrimaryStat::Dexterity),
        ));
    }
    for (slot, _) in &members {
        fields.push((
            WatchField::MemberInt(*slot),
            stat(*slot, PrimaryStat::Intelligence),
        ));
    }
    for (slot, _) in &members {
        let xp = state
            .party_experience
            .get(usize::from(*slot))
            .map_or_else(WatchValue::absent, |xp| Num(i64::from(*xp)));
        fields.push((WatchField::MemberXp(*slot), xp));
    }
    for (slot, member) in &members {
        fields.push((WatchField::MemberLevel(*slot), Num(i64::from(member.level))));
    }
    fields.extend([
        (WatchField::PartySize, Num(state.party.len() as i64)),
        (
            WatchField::ActivePlayer,
            state
                .active_player
                .map_or_else(WatchValue::absent, |slot| Num(slot as i64 + 1)),
        ),
        (
            WatchField::MoralStanding,
            Num(i64::from(state.moral_standing)),
        ),
        (
            WatchField::LightCounters,
            Text(format!(
                "torch {}, spell {}",
                state.torch_counter, state.light_spell_counter
            )),
        ),
        (
            WatchField::ActiveEffect,
            match state.active_effect_tag {
                Some(_) => Text(format!("on, {} left", state.active_effect_counter)),
                None => Text("none".to_string()),
            },
        ),
        (WatchField::Prng, Num(i64::from(state.prng_state))),
    ]);
    fields
}

/// Every field whose value differs between two snapshots, in
/// [`WatchField`] order, as `(field, old, new)`. A field present in only
/// one snapshot reads as [`WatchValue::absent`] in the other.
pub fn watch_diff(
    old: &[(WatchField, WatchValue)],
    new: &[(WatchField, WatchValue)],
) -> Vec<(WatchField, WatchValue, WatchValue)> {
    let mut merged: BTreeMap<WatchField, (WatchValue, WatchValue)> = BTreeMap::new();
    for (field, value) in old {
        merged.insert(*field, (value.clone(), WatchValue::absent()));
    }
    for (field, value) in new {
        merged
            .entry(*field)
            .or_insert_with(|| (WatchValue::absent(), WatchValue::absent()))
            .1 = value.clone();
    }
    merged
        .into_iter()
        .filter(|(_, (old, new))| old != new)
        .map(|(field, (old, new))| (field, old, new))
        .collect()
}

// ---------------------------------------------------------------------------
// Teleport (7.3.6, D7.8, D7.9).
// ---------------------------------------------------------------------------

/// Where a teleport goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TeleportArea {
    World(WorldPlane),
    Location { scene: Scene, floor: i8 },
    Dungeon { scene: DungeonScene, level: u8 },
}

/// A teleport destination cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TeleportTarget {
    pub area: TeleportArea,
    pub x: usize,
    pub y: usize,
}

/// The landing cell [`plan_teleport`] chose, or why there is none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeleportPlan {
    Ready {
        target: TeleportTarget,
        nudged: bool,
    },
    Refused(TeleportRefusal),
}

/// Why a teleport cannot run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TeleportRefusal {
    NotAtIdlePrompt,
    InCombat,
    AboardVessel,
    BadFloor,
    OutOfBounds,
    NoOpenCell,
    LoadFailed(String),
}

/// The Chebyshev radius of the nudge search (D7.9).
pub const TELEPORT_NUDGE_RADIUS: usize = 3;

impl TeleportArea {
    /// The side of the area's square map: the overworld is 256 cells, a
    /// location 32 and a dungeon level 8 (the bounds of
    /// [`gate_travel_destination`]).
    pub fn side(self) -> usize {
        match self {
            TeleportArea::World(_) => WORLD_SIDE,
            TeleportArea::Location { .. } => 32,
            TeleportArea::Dungeon { .. } => DUNGEON_SIDE,
        }
    }

    /// Whether the live state is already in this area.
    pub fn is_current(self, area: Area) -> bool {
        match (self, area) {
            (TeleportArea::World(want), Area::World { plane }) => want == plane,
            (
                TeleportArea::Location { scene, floor },
                Area::Town {
                    scene: here,
                    floor: here_floor,
                },
            ) => scene == here && floor == here_floor,
            (
                TeleportArea::Dungeon { scene, level },
                Area::Dungeon {
                    scene: here,
                    level: here_level,
                },
            ) => scene == here && level == here_level,
            _ => false,
        }
    }

    fn scene_byte_and_z(self) -> (u8, u8) {
        match self {
            TeleportArea::World(WorldPlane::Britannia) => (0, 0),
            // Any nonzero z names the Underworld; this is the byte a save
            // of the plane records.
            TeleportArea::World(WorldPlane::Underworld) => (0, u8::MAX),
            TeleportArea::Location { scene, floor } => (scene.byte, floor as u8),
            TeleportArea::Dungeon { scene, level } => (scene.byte, level),
        }
    }
}

impl TeleportTarget {
    /// The target as a moonstone slot, so the moongate's own validation
    /// checks it (7.3.6 step 2). `None` when a coordinate does not fit a
    /// slot byte.
    fn as_gate_slot(self) -> Option<MoonstoneGateSlot> {
        let (scene, z) = self.area.scene_byte_and_z();
        Some(MoonstoneGateSlot {
            scene,
            x: u8::try_from(self.x).ok()?,
            y: u8::try_from(self.y).ok()?,
            z,
        })
    }
}

/// Whether the party may land on `(x, y)` of `state`'s current area: open
/// ground for its transport with no NPC or object on it; in a dungeon, an
/// ordinary passage cell with no object (7.2.7).
fn teleport_cell_open(state: &PlayState, x: usize, y: usize) -> bool {
    let ground = match state.area {
        Area::Dungeon { level, .. } => {
            dungeon_cell_class_of(state.dungeon_cell(level, x, y)) == DungeonCellClass::Passage
        }
        _ => {
            state.tile_walkable(state.current_area_tile(x, y))
                && state.npc_at_current_floor(x, y).is_none()
        }
    };
    ground && state.object_at_current_floor(x, y).is_none()
}

/// The cells at Chebyshev distance `radius` from `(x, y)` inside a
/// `side`-cell map, rows top to bottom, then columns left to right.
pub fn teleport_ring(x: usize, y: usize, radius: usize, side: usize) -> Vec<(usize, usize)> {
    let (x, y, r) = (x as isize, y as isize, radius as isize);
    let side = side as isize;
    let mut cells = Vec::new();
    for cy in (y - r)..=(y + r) {
        for cx in (x - r)..=(x + r) {
            let ring = (cx - x).abs().max((cy - y).abs()) == r;
            if ring && (0..side).contains(&cx) && (0..side).contains(&cy) {
                cells.push((cx as usize, cy as usize));
            }
        }
    }
    cells
}

/// The target cell if it is open, else the first open cell of rings 1..3.
fn teleport_landing(
    state: &PlayState,
    want: TeleportTarget,
) -> Result<(TeleportTarget, bool), TeleportRefusal> {
    if teleport_cell_open(state, want.x, want.y) {
        return Ok((want, false));
    }
    (1..=TELEPORT_NUDGE_RADIUS)
        .flat_map(|radius| teleport_ring(want.x, want.y, radius, want.area.side()))
        .find(|&(x, y)| teleport_cell_open(state, x, y))
        .map(|(x, y)| (TeleportTarget { x, y, ..want }, true))
        .ok_or(TeleportRefusal::NoOpenCell)
}

/// Read-only: validate a teleport and choose its landing cell (7.3.6).
///
/// A different area is probed on a scratch state built by
/// [`PlayState::load_scene`] from the carried options, which writes no
/// file; the scratch state is dropped.
pub fn plan_teleport(state: &PlayState, game_dir: &Path, want: TeleportTarget) -> TeleportPlan {
    match plan_teleport_inner(state, game_dir, want) {
        Ok((target, nudged)) => TeleportPlan::Ready { target, nudged },
        Err(refusal) => TeleportPlan::Refused(refusal),
    }
}

fn plan_teleport_inner(
    state: &PlayState,
    game_dir: &Path,
    want: TeleportTarget,
) -> Result<(TeleportTarget, bool), TeleportRefusal> {
    if state.idle_command_prompt().is_none() {
        return Err(TeleportRefusal::NotAtIdlePrompt);
    }
    if state.combat_active {
        return Err(TeleportRefusal::InCombat);
    }
    if matches!(
        state.player.transport,
        TransportState::Ship { .. } | TransportState::Skiff { .. }
    ) {
        return Err(TeleportRefusal::AboardVessel);
    }
    if want.x >= want.area.side() || want.y >= want.area.side() {
        return Err(TeleportRefusal::OutOfBounds);
    }
    let slot = want.as_gate_slot().ok_or(TeleportRefusal::OutOfBounds)?;
    if let GateTravelDestination::Invalid(_) | GateTravelDestination::Empty =
        gate_travel_destination(slot)
    {
        return Err(match want.area {
            TeleportArea::Dungeon { .. } => TeleportRefusal::BadFloor,
            _ => TeleportRefusal::OutOfBounds,
        });
    }
    let (target, floor) = match gate_travel_destination_in(game_dir, slot) {
        Ok(GateTravelDestination::Ready { target, floor, .. }) => (target, floor),
        Ok(_) => return Err(TeleportRefusal::BadFloor),
        Err(err) => return Err(TeleportRefusal::LoadFailed(err.to_string())),
    };
    if want.area.is_current(state.area) {
        return teleport_landing(state, want);
    }
    // `load_scene` refuses a start cell its own check finds blocked, so the
    // scratch state is loaded at the first candidate it accepts, in the
    // nudge order (D7.9). The landing cell chosen on it must itself load,
    // or the commit would fail; a cell the load refuses is skipped.
    let load = |cell: (usize, usize)| {
        PlayState::load_scene(game_dir, state.carried_play_options(target, floor, cell))
    };
    let candidates: Vec<(usize, usize)> = std::iter::once((want.x, want.y))
        .chain(
            (1..=TELEPORT_NUDGE_RADIUS)
                .flat_map(|radius| teleport_ring(want.x, want.y, radius, want.area.side())),
        )
        .collect();
    let mut scratch = None;
    for &cell in &candidates {
        match load(cell) {
            Ok(loaded) => {
                scratch = Some((cell, loaded));
                break;
            }
            Err(err) if teleport_start_refused(&err) => {}
            Err(err) => return Err(TeleportRefusal::LoadFailed(err.to_string())),
        }
    }
    let Some((loaded_at, scratch)) = scratch else {
        return Err(TeleportRefusal::NoOpenCell);
    };
    for &(x, y) in &candidates {
        if !teleport_cell_open(&scratch, x, y) {
            continue;
        }
        if (x, y) != loaded_at {
            match load((x, y)) {
                Ok(_) => {}
                Err(err) if teleport_start_refused(&err) => continue,
                Err(err) => return Err(TeleportRefusal::LoadFailed(err.to_string())),
            }
        }
        let nudged = (x, y) != (want.x, want.y);
        return Ok((TeleportTarget { x, y, ..want }, nudged));
    }
    Err(TeleportRefusal::NoOpenCell)
}

/// Whether a scene load failed only because its start cell is blocked or
/// out of range: the start checks report `InvalidInput`, while a missing or
/// damaged file reports another kind.
fn teleport_start_refused(err: &io::Error) -> bool {
    err.kind() == io::ErrorKind::InvalidInput
}

/// The scene load a teleport target names, through the moongate's own
/// slot decoding.
fn teleport_destination(target: TeleportTarget) -> io::Result<(PlayTarget, i8, (usize, usize))> {
    match target.as_gate_slot().map(gate_travel_destination) {
        Some(GateTravelDestination::Ready {
            target,
            floor,
            start,
        }) => Ok((target, floor, start)),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the teleport target is not a destination",
        )),
    }
}

/// Commit a [`TeleportPlan::Ready`] plan. No turn, no epilogue, no
/// narration and no cue (7.2.7).
///
/// Inside one location floor or dungeon level the party is placed where
/// it stands. Anything else, the overworld included, is the gate-travel
/// rebuild without its gate-specific parts: the departing plane's `.OOL`
/// is written, the carried options are loaded, the Underworld fixed
/// objects are placed, a dungeon puts the party on foot, and the sound
/// history, transcript, session settings and presentation flags carry.
pub fn commit_teleport(
    state: &mut PlayState,
    game_dir: &Path,
    plan: &TeleportPlan,
) -> io::Result<()> {
    let TeleportPlan::Ready { target, .. } = *plan else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a refused teleport plan cannot be committed",
        ));
    };
    let (play_target, floor, start) = teleport_destination(target)?;
    let same_area = target.area.is_current(state.area);
    if same_area && !matches!(target.area, TeleportArea::World(_)) {
        state.player.x = target.x;
        state.player.y = target.y;
        state.sync_player_object();
        state.mark_visibility_dirty();
        return Ok(());
    }
    // As a moongate warp within one plane: a gate already live on the
    // landing cell is carried, any other is held back one command.
    let arrival_gate_carried = same_area && {
        let idx = world_cell_index(start.0, start.1);
        state.natural_moongate_live_cells.contains(&idx)
            && state.grid.get(idx).copied() == Some(NATURAL_MOONGATE_TERRAIN_TILE)
    };
    state.cache_current_world_overlay();
    state.write_current_world_plane_object_table(game_dir, true)?;
    let previous_turn = state.turn;
    let options = state.carried_play_options(play_target, floor, start);
    let mut next = PlayState::load_scene(game_dir, options)?;
    next.turn = previous_turn;
    next.world_overlays = state.world_overlays.clone();
    if let PlayTarget::World(plane) = play_target {
        next.place_underworld_fixed_objects(plane);
        next.cache_current_world_overlay();
    }
    if matches!(play_target, PlayTarget::Dungeon(_)) {
        next.force_foot_transport();
    }
    next.sync_player_object();
    next.pending_town_arrest = None;
    next.active_blackthorn = None;
    state.carry_presentation_into(&mut next);
    next.natural_moongate_hook_held_for_warp_arrival = !arrival_gate_carried
        && next
            .current_live_natural_moongate_cell_index()
            .is_some_and(|idx| next.grid.get(idx).copied() == Some(NATURAL_MOONGATE_TERRAIN_TILE));
    *state = next;
    Ok(())
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod teleport_tests;

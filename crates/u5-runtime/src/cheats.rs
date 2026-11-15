//! Cheats and debug tools: the runtime rules (Phase B design 7.3.3, 7.3.8;
//! build plan M8-01).
//!
//! This module holds the engine half of chapter 7:
//!
//! - the limits table, every value cited to the specification and reusing
//!   the engine's own anchored constants, so the cheats and the game can
//!   never disagree;
//! - [`CheatEdit`] and [`apply_cheat_edit`], the quick actions;
//! - [`EngagedLocks`], [`StoreFields`], the `apply_*_lock` functions and
//!   [`apply_locks`];
//! - the slot-0 mirror-coherent stat setters;
//! - [`WatchField`], [`WatchValue`], [`watch_snapshot`] and [`watch_diff`],
//!   the read-only change watch.
//!
//! Every function here is deterministic: no I/O, no randomness, no clock
//! or turn change. Nothing in the game calls this module; only the cheats
//! plugin in the shell does, and only while a cheat is on (7.4.2). Every
//! label is engine text.

use std::collections::BTreeMap;

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

/// A quick action (7.2.4). Edit party and stores adds its variants in
/// M8-03.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CheatEdit {
    HealAll,
    CurePoison,
    ResurrectAll,
    MaxGold,
    MaxFood,
    RefillArrows { quarrels: bool },
    MaxReagents,
    RepairHull { parked: bool },
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

impl CheatRefusal {
    /// The page's result line for this refusal (engine text).
    pub fn message(self) -> String {
        match self {
            CheatRefusal::NotAtIdlePrompt => "Not now".to_string(),
            CheatRefusal::InCombat => "Not during combat".to_string(),
            CheatRefusal::NotAboard => "Not aboard a ship".to_string(),
            CheatRefusal::NothingToDo => "Nothing to change".to_string(),
            CheatRefusal::OutOfRange { min, max } => format!("Range {min}..{max}"),
            CheatRefusal::NoSuchMember => "No such member".to_string(),
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
    if in_combat && matches!(edit, CheatEdit::CurePoison | CheatEdit::ResurrectAll) {
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
            WatchField::Turn => "Turn".to_string(),
            WatchField::Clock => "Clock".to_string(),
            WatchField::Area => "Area".to_string(),
            WatchField::Position => "Position".to_string(),
            WatchField::Facing => "Facing".to_string(),
            WatchField::Transport => "Transport".to_string(),
            WatchField::Hull => "Hull".to_string(),
            WatchField::Wind => "Wind".to_string(),
            WatchField::Food => "Food".to_string(),
            WatchField::Gold => "Gold".to_string(),
            WatchField::Keys => "Keys".to_string(),
            WatchField::Gems => "Gems".to_string(),
            WatchField::Torches => "Torches".to_string(),
            WatchField::Arrows => "Arrows".to_string(),
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
            WatchField::PartySize => "Party size".to_string(),
            WatchField::ActivePlayer => "Active member".to_string(),
            WatchField::MoralStanding => "Standing".to_string(),
            WatchField::LightCounters => "Light".to_string(),
            WatchField::ActiveEffect => "Effect".to_string(),
            WatchField::Prng => "Random state".to_string(),
        }
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

#[cfg(test)]
mod tests;

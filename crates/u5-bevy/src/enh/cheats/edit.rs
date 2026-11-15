//! Edit party and stores (Phase B design 7.2.6; build plan M8-03).
//!
//! The editor works on a draft ([`EditDraft`]): the page's keys change the
//! draft only, and nothing reaches the game until `[A]` Apply, which turns
//! every changed field into runtime edits ([`EditDraft::edits`]) applied by
//! `cheat_apply` in one call (`apply_cheat_edits`). `[U]` Undo all throws
//! the draft away.
//!
//! - Up/Down move between fields, the selected member's first and then the
//!   stores; `1`-`6` pick the member. Only the first party-size records,
//!   the travelling party, are shown (`formats/saved-gam.md §4`).
//! - Left/Right step by one, PgUp/PgDn by ten, clamped at the range ends.
//!   A value already outside its range (an edited or legacy save) is drawn
//!   with a `!` and kept exactly as it is unless that field is edited; a
//!   step from it lands inside the range.
//! - Enter types a value; a typed value outside the range is refused and
//!   the result line shows the range.
//! - Status cycles G, P, S, D (or takes those letters), outside combat
//!   only. Ashes is shown and kept, and can never be chosen.
//! - `[L]` recomputes level from experience with maximum HP `30 * level`.
//!
//! Every label here is engine text.

use u5_runtime::cheats::{
    CHEAT_BYTE_STOCK_MAX, CHEAT_EXPERIENCE_MAX, CHEAT_FOOD_MAX, CHEAT_GOLD_MAX, CHEAT_STAT_MAX,
    CHEAT_STAT_MIN, CheatEdit, CheatRefusal, MemberEdit, PrimaryStat, StoredStatus, StoresEdit,
    cheat_hp_for_level, cheat_level_for_experience, cheat_mana_target, member_hp_range,
    member_stat,
};
use u5_runtime::{EQUIPMENT_ID_ARROWS, EQUIPMENT_ID_QUARRELS, PlayState, REAGENT_COUNT};

// ---------------------------------------------------------------------------
// Engine text.

pub const EDIT_TITLE: &str = "Edit party";
pub const EDIT_FOOTER: &str = "A: apply  U: undo  Esc: back";
/// The main page's entry.
pub const EDIT_ENTRY: &str = "[E] Edit party";
pub const MORE_HEADING: &str = "More:";
pub const TABLE_HEADS: [(u32, &str); 7] = [
    (0, "#"),
    (2, "Name"),
    (11, "Cl"),
    (14, "St"),
    (18, "HP/Max"),
    (25, "Lv"),
    (31, "XP"),
];
pub const HELP_LINES: [&str; 2] = [
    "Up/Down field  Left/Right PgUp/PgDn",
    "Enter type  1-6 member  L level",
];
const APPLIED: &str = "Changes applied.";
const UNDONE: &str = "Changes undone.";
const NOT_IN_COMBAT: &str = "Not during combat.";
const ASHES_KEPT: &str = "Ashes cannot change.";
const LIVING_ONLY: &str = "Only for the living.";
const NO_MANA: &str = "No mana for this class.";
const USE_ARROWS: &str = "Use Left/Right or G P S D.";
const LEVEL_WORD: &str = "Level";
const MAX_HP_WORD: &str = "max HP";
const APPLY_HINT: &str = "A applies.";
const LABEL_STR: &str = "STR";
const LABEL_DEX: &str = "DEX";
const LABEL_INT: &str = "INT";
const LABEL_STATUS: &str = "Status";
const LABEL_HP: &str = "HP";
const LABEL_MP: &str = "MP";
const LABEL_XP: &str = "XP";
const LABEL_FOOD: &str = "Food";
const LABEL_GOLD: &str = "Gold";
const LABEL_KEYS: &str = "Keys";
const LABEL_GEMS: &str = "Gems";
const LABEL_TORCHES: &str = "Torches";
const LABEL_ARROWS: &str = "Arrows";
const LABEL_QUARRELS: &str = "Quarrels";
const LABEL_REAGENT: &str = "Reag";
/// The field and member cursor.
pub const CURSOR: &str = ">";
/// Shown for a value that does not apply (MP of a class with no target).
const NONE_SHOWN: &str = "-";
/// The most members the table shows (a full party).
pub const TABLE_ROWS: usize = 6;

// ---------------------------------------------------------------------------
// The fields.

/// One editable field: the selected member's, then the stores'.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    Str,
    Dex,
    Int,
    Status,
    Hp,
    Mp,
    Xp,
    Food,
    Gold,
    Keys,
    Gems,
    Torches,
    Arrows,
    Quarrels,
    Reagent(u8),
}

/// The fields in Up/Down order.
pub fn field_order() -> Vec<Field> {
    let mut fields = vec![
        Field::Str,
        Field::Dex,
        Field::Int,
        Field::Hp,
        Field::Mp,
        Field::Xp,
        Field::Status,
        Field::Food,
        Field::Gold,
        Field::Keys,
        Field::Gems,
        Field::Torches,
        Field::Arrows,
        Field::Quarrels,
    ];
    fields.extend((0..REAGENT_COUNT as u8).map(Field::Reagent));
    fields
}

impl Field {
    /// Whether it belongs to the selected member.
    pub fn is_member(self) -> bool {
        matches!(
            self,
            Field::Str
                | Field::Dex
                | Field::Int
                | Field::Status
                | Field::Hp
                | Field::Mp
                | Field::Xp
        )
    }

    fn label(self) -> String {
        match self {
            Field::Str => LABEL_STR.to_string(),
            Field::Dex => LABEL_DEX.to_string(),
            Field::Int => LABEL_INT.to_string(),
            Field::Status => LABEL_STATUS.to_string(),
            Field::Hp => LABEL_HP.to_string(),
            Field::Mp => LABEL_MP.to_string(),
            Field::Xp => LABEL_XP.to_string(),
            Field::Food => LABEL_FOOD.to_string(),
            Field::Gold => LABEL_GOLD.to_string(),
            Field::Keys => LABEL_KEYS.to_string(),
            Field::Gems => LABEL_GEMS.to_string(),
            Field::Torches => LABEL_TORCHES.to_string(),
            Field::Arrows => LABEL_ARROWS.to_string(),
            Field::Quarrels => LABEL_QUARRELS.to_string(),
            Field::Reagent(index) => format!("{LABEL_REAGENT} {}", u32::from(index) + 1),
        }
    }
}

// ---------------------------------------------------------------------------
// The values.

/// One travelling member as the editor shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberValues {
    pub name: String,
    pub class_byte: u8,
    pub status: u8,
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
    pub hp: u16,
    pub max_hp: u16,
    pub mana: u8,
    pub experience: u16,
    pub level: u8,
}

impl MemberValues {
    fn of(state: &PlayState, slot: usize) -> Self {
        let member = state.party[slot];
        let stat = |which| member_stat(state, slot, which).unwrap_or(0);
        let name = state
            .party_names
            .get(slot)
            .map(|bytes| {
                bytes
                    .iter()
                    .take_while(|byte| **byte != 0)
                    .map(|byte| {
                        if byte.is_ascii_graphic() {
                            char::from(*byte)
                        } else {
                            ' '
                        }
                    })
                    .take(8)
                    .collect()
            })
            .unwrap_or_default();
        Self {
            name,
            class_byte: member.class_byte,
            status: member.status,
            strength: stat(PrimaryStat::Strength),
            dexterity: stat(PrimaryStat::Dexterity),
            intelligence: stat(PrimaryStat::Intelligence),
            hp: member.hp,
            max_hp: member.max_hp,
            mana: member.mana,
            experience: state.party_experience.get(slot).copied().unwrap_or(0),
            level: member.level,
        }
    }

    fn living(&self) -> bool {
        member_hp_range(self.status, self.max_hp).is_some()
    }

    fn mana_target(&self) -> Option<u8> {
        cheat_mana_target(self.class_byte, self.intelligence)
    }
}

/// The shared counters as the editor shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoreValues {
    pub food: u16,
    pub gold: u16,
    pub keys: u8,
    pub gems: u8,
    pub torches: u8,
    pub arrows: u8,
    pub quarrels: u8,
    pub reagents: [u8; REAGENT_COUNT],
}

impl StoreValues {
    fn of(state: &PlayState) -> Self {
        Self {
            food: state.food,
            gold: state.gold,
            keys: state.keys,
            gems: state.gems,
            torches: state.torches,
            arrows: state.equipment_stock[EQUIPMENT_ID_ARROWS],
            quarrels: state.equipment_stock[EQUIPMENT_ID_QUARRELS],
            reagents: state.reagents,
        }
    }
}

/// `Some(new)` when the draft differs from the original.
fn changed<T: PartialEq + Copy>(original: T, draft: T) -> Option<T> {
    (original != draft).then_some(draft)
}

// ---------------------------------------------------------------------------
// The draft.

/// What a key did, for the page's result line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EditOutcome {
    /// The draft changed (or the cursor moved); nothing to say.
    Quiet,
    /// A line for the result strip.
    Say(String),
    /// Apply: the runtime edits to queue.
    Apply(Vec<CheatEdit>),
    /// Leave the editor for the main page.
    Back,
}

/// The editor's working copy (7.2.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditDraft {
    original: Vec<MemberValues>,
    members: Vec<MemberValues>,
    /// `[L]` was pressed for the member.
    recompute: Vec<bool>,
    original_stores: StoreValues,
    stores: StoreValues,
    in_combat: bool,
    member: usize,
    field: usize,
    /// Digits typed after Enter.
    typing: Option<String>,
}

impl EditDraft {
    /// A fresh draft of `state`: the first party-size records only.
    pub fn of(state: &PlayState) -> Self {
        let members: Vec<MemberValues> = (0..state.party.len().min(TABLE_ROWS))
            .map(|slot| MemberValues::of(state, slot))
            .collect();
        let stores = StoreValues::of(state);
        Self {
            recompute: vec![false; members.len()],
            original: members.clone(),
            members,
            original_stores: stores.clone(),
            stores,
            in_combat: state.combat_active,
            member: 0,
            field: 0,
            typing: None,
        }
    }

    /// The same draft rebuilt from `state` after an Apply, keeping the
    /// cursor.
    pub fn refreshed(&self, state: &PlayState) -> Self {
        let mut fresh = Self::of(state);
        fresh.member = self.member.min(fresh.members.len().saturating_sub(1));
        fresh.field = self.field;
        fresh
    }

    pub fn members(&self) -> &[MemberValues] {
        &self.members
    }

    pub fn stores(&self) -> &StoreValues {
        &self.stores
    }

    pub fn selected_member(&self) -> usize {
        self.member
    }

    pub fn field(&self) -> Field {
        field_order()[self.field]
    }

    pub fn typing(&self) -> Option<&str> {
        self.typing.as_deref()
    }

    /// The inclusive range a field takes now, or why it cannot be edited.
    pub fn range(&self, field: Field) -> Result<(u32, u32), String> {
        let member = self.members.get(self.member);
        let byte = (0, u32::from(CHEAT_BYTE_STOCK_MAX));
        let stat = (u32::from(CHEAT_STAT_MIN), u32::from(CHEAT_STAT_MAX));
        Ok(match field {
            Field::Str | Field::Dex | Field::Int => stat,
            Field::Xp => (0, u32::from(CHEAT_EXPERIENCE_MAX)),
            Field::Hp => {
                let member = member.ok_or_else(String::new)?;
                let (min, max) = member_hp_range(member.status, member.max_hp)
                    .ok_or_else(|| LIVING_ONLY.to_string())?;
                (u32::from(min), u32::from(max))
            }
            Field::Mp => {
                let member = member.ok_or_else(String::new)?;
                let target = member.mana_target().ok_or_else(|| NO_MANA.to_string())?;
                (0, u32::from(target))
            }
            Field::Status => return Err(USE_ARROWS.to_string()),
            Field::Food => (0, u32::from(CHEAT_FOOD_MAX)),
            Field::Gold => (0, u32::from(CHEAT_GOLD_MAX)),
            Field::Keys
            | Field::Gems
            | Field::Torches
            | Field::Arrows
            | Field::Quarrels
            | Field::Reagent(_) => byte,
        })
    }

    /// A field's draft value, for the numeric fields.
    pub fn value(&self, field: Field) -> Option<u32> {
        let member = self.members.get(self.member);
        let stores = &self.stores;
        Some(match field {
            Field::Str => u32::from(member?.strength),
            Field::Dex => u32::from(member?.dexterity),
            Field::Int => u32::from(member?.intelligence),
            Field::Hp => u32::from(member?.hp),
            Field::Mp => u32::from(member?.mana),
            Field::Xp => u32::from(member?.experience),
            Field::Status => return None,
            Field::Food => u32::from(stores.food),
            Field::Gold => u32::from(stores.gold),
            Field::Keys => u32::from(stores.keys),
            Field::Gems => u32::from(stores.gems),
            Field::Torches => u32::from(stores.torches),
            Field::Arrows => u32::from(stores.arrows),
            Field::Quarrels => u32::from(stores.quarrels),
            Field::Reagent(index) => u32::from(stores.reagents[usize::from(index)]),
        })
    }

    /// Whether the draft value is outside the field's range (the `!`
    /// marker). A value that does not apply is never marked.
    pub fn out_of_range(&self, field: Field) -> bool {
        if field == Field::Status {
            return self
                .members
                .get(self.member)
                .is_some_and(|member| out_of_range_status(member.status));
        }
        match (self.value(field), self.range(field)) {
            (Some(value), Ok((min, max))) => !(min..=max).contains(&value),
            (Some(value), Err(_)) if field == Field::Hp => {
                // A Dead member with HP left is inconsistent.
                value > 0
            }
            _ => false,
        }
    }

    /// Whether the draft differs from the game for this field.
    pub fn dirty(&self, field: Field) -> bool {
        if field.is_member() {
            let (Some(draft), Some(original)) = (
                self.members.get(self.member),
                self.original.get(self.member),
            ) else {
                return false;
            };
            return match field {
                Field::Status => draft.status != original.status,
                _ => {
                    let mut was = self.clone();
                    was.members[self.member] = original.clone();
                    self.value(field) != was.value(field)
                }
            };
        }
        let mut was = self.clone();
        was.stores = self.original_stores.clone();
        self.value(field) != was.value(field)
    }

    /// Whether `[L]` is pending for the member at `slot`.
    pub fn recompute_pending(&self, slot: usize) -> bool {
        self.recompute.get(slot).copied().unwrap_or(false)
    }

    fn set(&mut self, field: Field, value: u32) {
        let member = self.member;
        let narrow8 = |value: u32| u8::try_from(value).unwrap_or(u8::MAX);
        let narrow16 = |value: u32| u16::try_from(value).unwrap_or(u16::MAX);
        let stores = &mut self.stores;
        match field {
            Field::Str => self.members[member].strength = narrow8(value),
            Field::Dex => self.members[member].dexterity = narrow8(value),
            Field::Int => self.members[member].intelligence = narrow8(value),
            Field::Hp => self.members[member].hp = narrow16(value),
            Field::Mp => self.members[member].mana = narrow8(value),
            Field::Xp => {
                self.members[member].experience = narrow16(value);
                if self.recompute[member] {
                    self.recompute_draft(member);
                }
            }
            Field::Status => {}
            Field::Food => stores.food = narrow16(value),
            Field::Gold => stores.gold = narrow16(value),
            Field::Keys => stores.keys = narrow8(value),
            Field::Gems => stores.gems = narrow8(value),
            Field::Torches => stores.torches = narrow8(value),
            Field::Arrows => stores.arrows = narrow8(value),
            Field::Quarrels => stores.quarrels = narrow8(value),
            Field::Reagent(index) => stores.reagents[usize::from(index)] = narrow8(value),
        }
    }

    /// Left/Right/PgUp/PgDn: the value moved by `delta`, clamped into the
    /// range (an out-of-range value lands inside it).
    pub fn step(&mut self, delta: i32) -> EditOutcome {
        let field = self.field();
        if field == Field::Status {
            return self.cycle_status(delta.signum());
        }
        if !self.has_owner(field) {
            return EditOutcome::Quiet;
        }
        let (min, max) = match self.range(field) {
            Ok(range) => range,
            Err(why) => return EditOutcome::Say(why),
        };
        let value = self.value(field).unwrap_or(0);
        let next = (i64::from(value) + i64::from(delta)).clamp(i64::from(min), i64::from(max));
        self.set(field, next as u32);
        EditOutcome::Quiet
    }

    /// Whether the field has something to edit: a store always, a
    /// member's field while there is a member.
    fn has_owner(&self, field: Field) -> bool {
        !field.is_member() || self.member < self.members.len()
    }

    /// The status choices, outside combat only; Ashes stays Ashes.
    fn status_guard(&self) -> Result<usize, String> {
        if self.in_combat {
            return Err(NOT_IN_COMBAT.to_string());
        }
        let member = self.members.get(self.member).ok_or_else(String::new)?;
        let original = &self.original[self.member];
        if original.status == b'A' || member.status == b'A' {
            return Err(ASHES_KEPT.to_string());
        }
        Ok(self.member)
    }

    fn cycle_status(&mut self, direction: i32) -> EditOutcome {
        let member = match self.status_guard() {
            Ok(member) => member,
            Err(why) => return EditOutcome::Say(why),
        };
        let all = StoredStatus::ALL;
        let next = match StoredStatus::of_byte(self.members[member].status) {
            Some(now) => {
                let index = all.iter().position(|status| *status == now).unwrap_or(0) as i32;
                all[(index + direction).rem_euclid(all.len() as i32) as usize]
            }
            // A stored letter that is no choice: the first step makes it one.
            None => StoredStatus::Good,
        };
        self.choose_status(member, next);
        EditOutcome::Quiet
    }

    /// `G`, `P`, `S` or `D` typed on the status field.
    pub fn status_key(&mut self, status: StoredStatus) -> EditOutcome {
        match self.status_guard() {
            Ok(member) => {
                self.choose_status(member, status);
                EditOutcome::Quiet
            }
            Err(why) => EditOutcome::Say(why),
        }
    }

    /// The runtime's status rule on the draft: `D` sets HP to 0, a living
    /// status on a member with no HP sets HP to max.
    fn choose_status(&mut self, member: usize, status: StoredStatus) {
        let values = &mut self.members[member];
        values.status = status.byte();
        if status == StoredStatus::Dead {
            values.hp = 0;
        } else if values.hp == 0 {
            values.hp = values.max_hp;
        }
    }

    /// Up/Down.
    pub fn move_field(&mut self, delta: i32) {
        let count = field_order().len() as i32;
        self.field = (self.field as i32 + delta).rem_euclid(count) as usize;
    }

    /// `1`-`6`.
    pub fn pick_member(&mut self, slot: usize) {
        if slot < self.members.len() {
            self.member = slot;
        }
    }

    /// `[L]`: the draft takes the recomputed level now; Apply sends it.
    pub fn recompute_level(&mut self) -> EditOutcome {
        let member = self.member;
        if member >= self.members.len() {
            return EditOutcome::Quiet;
        }
        self.recompute[member] = true;
        self.recompute_draft(member);
        let values = &self.members[member];
        EditOutcome::Say(format!(
            "{LEVEL_WORD} {}, {MAX_HP_WORD} {}. {APPLY_HINT}",
            values.level, values.max_hp
        ))
    }

    fn recompute_draft(&mut self, member: usize) {
        let values = &mut self.members[member];
        values.level = cheat_level_for_experience(values.experience);
        values.max_hp = cheat_hp_for_level(values.level);
        if values.living() {
            values.hp = values.max_hp;
        }
    }

    /// Enter: starts typing on a numeric field, or commits what was typed.
    pub fn enter(&mut self) -> EditOutcome {
        let field = self.field();
        let Some(typed) = self.typing.take() else {
            if !self.has_owner(field) {
                return EditOutcome::Quiet;
            }
            return match self.range(field) {
                Ok(_) => {
                    self.typing = Some(String::new());
                    EditOutcome::Quiet
                }
                Err(why) => EditOutcome::Say(why),
            };
        };
        let Ok((min, max)) = self.range(field) else {
            return EditOutcome::Quiet;
        };
        match typed.parse::<u32>() {
            Ok(value) if (min..=max).contains(&value) => {
                self.set(field, value);
                EditOutcome::Quiet
            }
            _ => EditOutcome::Say(format!(
                "{}.",
                CheatRefusal::OutOfRange { min, max }.message()
            )),
        }
    }

    /// How many digits the field under the cursor takes: one more than
    /// its largest value, so a value one step outside can be typed and
    /// refused.
    fn typing_cap(&self) -> usize {
        let max = self.range(self.field()).map_or(0, |(_, max)| max);
        max.to_string().len() + 1
    }

    /// A digit while typing.
    pub fn type_digit(&mut self, digit: u8) {
        let cap = self.typing_cap();
        if let Some(typed) = self.typing.as_mut()
            && typed.len() < cap
        {
            typed.push(char::from(b'0' + digit));
        }
    }

    pub fn backspace(&mut self) {
        if let Some(typed) = self.typing.as_mut() {
            typed.pop();
        }
    }

    /// Esc while typing: the typed digits are dropped.
    pub fn cancel_typing(&mut self) -> bool {
        self.typing.take().is_some()
    }

    /// `[U]`.
    pub fn undo_all(&mut self) -> EditOutcome {
        self.members = self.original.clone();
        self.stores = self.original_stores.clone();
        self.recompute.iter_mut().for_each(|flag| *flag = false);
        self.typing = None;
        EditOutcome::Say(UNDONE.to_string())
    }

    /// `[A]`: the runtime edits for every changed field, in order. Each
    /// member's fields go first; a pending `[L]` follows them, so it reads
    /// the experience just set, and a typed HP that differs from what `[L]`
    /// leaves follows that. An untouched field is never sent, so a value
    /// outside its range survives (7.2.6). An empty list is the empty Apply.
    pub fn edits(&self) -> Vec<CheatEdit> {
        let mut edits = Vec::new();
        for (slot, (draft, original)) in self.members.iter().zip(&self.original).enumerate() {
            let status = changed(original.status, draft.status).and_then(StoredStatus::of_byte);
            let mut fields = MemberEdit {
                strength: changed(original.strength, draft.strength),
                dexterity: changed(original.dexterity, draft.dexterity),
                intelligence: changed(original.intelligence, draft.intelligence),
                hp: None,
                mana: changed(original.mana, draft.mana),
                experience: changed(original.experience, draft.experience),
                status,
            };
            // The HP the runtime leaves without a typed HP.
            let mut hp = original.hp;
            if let Some(status) = status {
                if status == StoredStatus::Dead {
                    hp = 0;
                } else if hp == 0 {
                    hp = original.max_hp;
                }
            }
            let recompute = self.recompute[slot];
            if recompute && draft.living() {
                hp = draft.max_hp;
            }
            let hp = changed(hp, draft.hp).filter(|_| draft.living());
            if !recompute {
                fields.hp = hp;
            }
            if !fields.is_empty() {
                edits.push(CheatEdit::Member { slot, fields });
            }
            if recompute {
                edits.push(CheatEdit::RecomputeLevel { slot });
                if let Some(hp) = hp {
                    edits.push(CheatEdit::Member {
                        slot,
                        fields: MemberEdit {
                            hp: Some(hp),
                            ..MemberEdit::default()
                        },
                    });
                }
            }
        }
        let (was, now) = (&self.original_stores, &self.stores);
        let mut stores = StoresEdit {
            food: changed(was.food, now.food),
            gold: changed(was.gold, now.gold),
            keys: changed(was.keys, now.keys),
            gems: changed(was.gems, now.gems),
            torches: changed(was.torches, now.torches),
            arrows: changed(was.arrows, now.arrows),
            quarrels: changed(was.quarrels, now.quarrels),
            ..StoresEdit::default()
        };
        for (index, slot) in stores.reagents.iter_mut().enumerate() {
            *slot = changed(was.reagents[index], now.reagents[index]);
        }
        if !stores.is_empty() {
            edits.push(CheatEdit::Stores(stores));
        }
        edits
    }

    /// `[A]`.
    pub fn apply(&mut self) -> EditOutcome {
        self.typing = None;
        EditOutcome::Apply(self.edits())
    }
}

/// A stored status the editor cannot show as a choice or as Ashes.
fn out_of_range_status(status: u8) -> bool {
    StoredStatus::of_byte(status).is_none() && status != b'A'
}

/// The result line after an Apply went through.
pub fn applied_result() -> String {
    APPLIED.to_string()
}

// ---------------------------------------------------------------------------
// Drawing: what the page shows, as cell runs.

/// Ink roles, mapped to EGA indices by the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Text,
    Help,
    Dim,
    Accent,
}

/// One run of text the page draws, relative to the panel: `col` 0 is the
/// box's first cell, where the `>` cursor sits; text starts at 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditRun {
    pub col: u32,
    pub row: u32,
    pub text: String,
    pub ink: Ink,
}

/// The rows of the edit page, relative to the panel's top: the table, the
/// member's fields, the stores and the help.
/// The box's last row stays empty, so no descender meets the rule below.
pub const EDIT_DIVIDERS: [u32; 3] = [8, 12, 18];
const TABLE_HEAD_ROW: u32 = 1;
const TABLE_FIRST_ROW: u32 = 2;
const FIELDS_FIRST_ROW: u32 = 9;
const STORES_FIRST_ROW: u32 = 13;
pub const HELP_FIRST_ROW: u32 = 19;
pub const RESULT_ROW: u32 = 21;
/// The three columns of the member's fields and of the stores, each a
/// `>` cell, the text and a `!` cell.
const COLS: [u32; 3] = [0, 13, 26];
/// Text widths: the first two columns, and the last (which ends at the
/// box's edge).
const COL_WIDTH: usize = 11;
const LAST_COL_WIDTH: usize = 10;

/// The text width of column `column`.
const fn col_width(column: usize) -> usize {
    if column == COLS.len() - 1 {
        LAST_COL_WIDTH
    } else {
        COL_WIDTH
    }
}

/// `label` left, `value` right, in `width` cells.
fn padded(label: &str, value: &str, width: usize) -> String {
    let gap = width.saturating_sub(label.len() + value.len()).max(1);
    format!("{label}{}{value}", " ".repeat(gap))
}

fn status_shown(status: u8) -> String {
    if status.is_ascii_graphic() {
        char::from(status).to_string()
    } else {
        "?".to_string()
    }
}

impl EditDraft {
    /// A field's value as shown (the typed digits while typing).
    fn shown(&self, field: Field) -> String {
        if self.field() == field
            && let Some(typed) = &self.typing
        {
            // The cursor shows while another digit fits.
            return if typed.len() < self.typing_cap() {
                format!("{typed}_")
            } else {
                typed.clone()
            };
        }
        let member = self.members.get(self.member);
        match field {
            Field::Status => member.map_or_else(String::new, |m| status_shown(m.status)),
            Field::Mp if member.is_some_and(|m| m.mana_target().is_none()) => {
                NONE_SHOWN.to_string()
            }
            Field::Food | Field::Gold => format!("{:04}", self.value(field).unwrap_or(0)),
            Field::Hp => member.map_or_else(String::new, |m| format!("{}/{}", m.hp, m.max_hp)),
            Field::Keys
            | Field::Gems
            | Field::Torches
            | Field::Arrows
            | Field::Quarrels
            | Field::Reagent(_) => format!("{:02}", self.value(field).unwrap_or(0)),
            _ => self
                .value(field)
                .map_or_else(String::new, |v| v.to_string()),
        }
    }

    /// Whether a field cannot be edited now (drawn dimmed).
    fn locked(&self, field: Field) -> bool {
        match field {
            Field::Status => self.status_guard().is_err(),
            Field::Hp | Field::Mp => self.range(field).is_err(),
            _ => false,
        }
    }

    fn field_runs(&self, field: Field, col: u32, row: u32, width: usize, runs: &mut Vec<EditRun>) {
        if self.field() == field {
            runs.push(EditRun {
                col,
                row,
                text: CURSOR.to_string(),
                ink: Ink::Text,
            });
        }
        let ink = if self.locked(field) {
            Ink::Dim
        } else if self.dirty(field) {
            Ink::Accent
        } else {
            Ink::Text
        };
        runs.push(EditRun {
            col: col + 1,
            row,
            text: padded(&field.label(), &self.shown(field), width),
            ink,
        });
        if self.out_of_range(field) {
            runs.push(EditRun {
                col: col + 1 + width as u32,
                row,
                text: "!".to_string(),
                ink: Ink::Text,
            });
        }
    }

    /// Every run of the edit page except the result line.
    pub fn runs(&self) -> Vec<EditRun> {
        let mut runs = Vec::new();
        let mut put = |col: u32, row: u32, text: String, ink: Ink| {
            runs.push(EditRun {
                col,
                row,
                text,
                ink,
            });
        };
        for (col, head) in TABLE_HEADS {
            put(1 + col, TABLE_HEAD_ROW, head.to_string(), Ink::Help);
        }
        for (slot, member) in self.members.iter().enumerate() {
            let row = TABLE_FIRST_ROW + slot as u32;
            if slot == self.member {
                put(0, row, CURSOR.to_string(), Ink::Text);
            }
            let dirty = *member != self.original[slot];
            let ink = if dirty { Ink::Accent } else { Ink::Text };
            let line = format!(
                "{} {:<8} {}  {}  {:>7} {:>2}  {:>4}",
                slot + 1,
                member.name,
                status_shown(member.class_byte),
                status_shown(member.status),
                format!("{}/{}", member.hp, member.max_hp),
                member.level,
                member.experience,
            );
            put(1, row, line, ink);
        }
        if let Some(member) = self.members.get(self.member) {
            // Columns: STR DEX INT, HP MP XP, then Status over Level.
            let columns = [
                vec![Field::Str, Field::Dex, Field::Int],
                vec![Field::Hp, Field::Mp, Field::Xp],
                vec![Field::Status],
            ];
            for (column, fields) in columns.iter().enumerate() {
                for (i, field) in fields.iter().enumerate() {
                    let row = FIELDS_FIRST_ROW + i as u32;
                    self.field_runs(*field, COLS[column], row, col_width(column), &mut runs);
                }
            }
            runs.push(EditRun {
                col: COLS[2] + 1,
                row: FIELDS_FIRST_ROW + 1,
                text: padded(LEVEL_WORD, &member.level.to_string(), col_width(2)),
                ink: if self.recompute[self.member] {
                    Ink::Accent
                } else {
                    Ink::Text
                },
            });
        }
        let stores: Vec<Field> = field_order()
            .into_iter()
            .filter(|f| !f.is_member())
            .collect();
        for (i, field) in stores.into_iter().enumerate() {
            let column = i % COLS.len();
            self.field_runs(
                field,
                COLS[column],
                STORES_FIRST_ROW + (i / COLS.len()) as u32,
                col_width(column),
                &mut runs,
            );
        }
        for (i, line) in HELP_LINES.iter().enumerate() {
            runs.push(EditRun {
                col: 1,
                row: HELP_FIRST_ROW + i as u32,
                text: line.to_string(),
                ink: Ink::Help,
            });
        }
        runs
    }
}

#[cfg(test)]
#[path = "edit_tests.rs"]
mod tests;

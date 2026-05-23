//! Terminal endgame state helpers.

use crate::*;

/// `catalogs/quest-graph.md §5` Eternal-Flame principle the
/// destruction of one Shadowlord's shard must use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EternalFlame {
    /// Falsehood's opposed flame.
    Truth,
    /// Hatred's opposed flame.
    Love,
    /// Cowardice's opposed flame.
    Courage,
}

impl EternalFlame {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Truth => "Flame of Truth",
            Self::Love => "Flame of Love",
            Self::Courage => "Flame of Courage",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        let key = key.to_ascii_lowercase();
        let key = key
            .strip_prefix("flame_of_")
            .or_else(|| key.strip_prefix("flame-of-"))
            .or_else(|| key.strip_prefix("flameof"))
            .or_else(|| key.strip_prefix("flame"))
            .unwrap_or(&key);
        match key.trim_matches(['_', '-']) {
            "truth" => Some(Self::Truth),
            "love" => Some(Self::Love),
            "courage" => Some(Self::Courage),
            _ => None,
        }
    }
}

/// `catalogs/quest-graph.md §5`: the Eternal-Flame paired with each
/// Shadowlord (by zero-based slot 0=Falsehood / 1=Hatred / 2=Cowardice).
pub const fn eternal_flame_for_shadowlord(slot: usize) -> Option<EternalFlame> {
    Some(match slot {
        0 => EternalFlame::Truth,
        1 => EternalFlame::Love,
        2 => EternalFlame::Courage,
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §5` typed Shadowlord identity. The three
/// hideout slots are tied to a fixed virtue principle: slot 0 is
/// Falsehood (Fauline), slot 1 is Hatred (Astaroth), slot 2 is
/// Cowardice (Nosfentor). The principle, the name, and the opposed
/// Eternal-Flame are stable across the quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowlordPrinciple {
    /// Slot 0 — Fauline, opposed by the Flame of Truth.
    Falsehood,
    /// Slot 1 — Astaroth, opposed by the Flame of Love.
    Hatred,
    /// Slot 2 — Nosfentor, opposed by the Flame of Courage.
    Cowardice,
}

impl ShadowlordPrinciple {
    /// Slot order published in `catalogs/quest-graph.md §5`.
    pub const ALL: [Self; 3] = [Self::Falsehood, Self::Hatred, Self::Cowardice];

    /// `catalogs/quest-graph.md §5`: hideout slot index `0..=2` the
    /// shipped roster assigns to this principle.
    pub const fn slot(self) -> usize {
        match self {
            Self::Falsehood => 0,
            Self::Hatred => 1,
            Self::Cowardice => 2,
        }
    }

    /// `catalogs/quest-graph.md §5`: opposed Eternal Flame.
    pub const fn eternal_flame(self) -> EternalFlame {
        match self {
            Self::Falsehood => EternalFlame::Truth,
            Self::Hatred => EternalFlame::Love,
            Self::Cowardice => EternalFlame::Courage,
        }
    }
}

/// `catalogs/quest-graph.md §5`: classify a hideout slot index into
/// the typed Shadowlord principle, or `None` for indices outside
/// the published `0..=2` set.
pub const fn shadowlord_principle_for_slot(slot: usize) -> Option<ShadowlordPrinciple> {
    Some(match slot {
        0 => ShadowlordPrinciple::Falsehood,
        1 => ShadowlordPrinciple::Hatred,
        2 => ShadowlordPrinciple::Cowardice,
        _ => return None,
    })
}

/// `catalogs/quest-graph.md §2`: the four main-quest requirements
/// the web-shaped progression converges on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainQuestRequirement {
    /// 1. Recover the royal artifacts: Crown, Sceptre, and Amulet.
    RoyalArtifacts,
    /// 2. Learn and use the eight dungeon Words of Power.
    DungeonWords,
    /// 3. Recover and destroy the three evil shards by using the
    ///    Eternal Flames and the Shadowlords' names.
    ShardsAndShadowlords,
    /// 4. Preserve and use the hidden sandalwood-box object that
    ///    enables Lord British's return.
    SandalwoodBox,
}

impl MainQuestRequirement {
    /// `catalogs/quest-graph.md §2` ordered list of the four
    /// requirements as they appear in the spec.
    pub const ALL: [Self; 4] = [
        Self::RoyalArtifacts,
        Self::DungeonWords,
        Self::ShardsAndShadowlords,
        Self::SandalwoodBox,
    ];
}

/// `catalogs/quest-graph.md §7`: shipped Sandalwood Box pickup —
/// non-speaking object slot 31 in `CASTLE:0` at local (X=18, Y=12, Z=2)
/// with object tag `0x0E` (the pickup runs through the shared
/// item-add path, sets the save-backed box flag).
/// `catalogs/quest-graph.md §7`: the Sandalwood Box ships in
/// Lord British's Castle (`CASTLE:0`, scene byte 17). Anchored
/// to [`crate::SCENE_LORD_BRITISHS_CASTLE`] so the box's scene
/// and the named scene constant share one source of truth.
pub const SANDALWOOD_BOX_PICKUP_SCENE: u8 = crate::SCENE_LORD_BRITISHS_CASTLE;
pub const SANDALWOOD_BOX_PICKUP_X: u8 = 18;
pub const SANDALWOOD_BOX_PICKUP_Y: u8 = 12;
pub const SANDALWOOD_BOX_PICKUP_Z: u8 = 2;
/// `catalogs/quest-graph.md §7` shipped Sandalwood Box pickup —
/// the box lives in the last active-object slot of the
/// 32-record CASTLE:0 active-object table. Anchored to
/// [`crate::OOL_SLOTS`] - 1 so the highest active-object slot
/// index has one source of truth.
pub const SANDALWOOD_BOX_PICKUP_OBJECT_SLOT: usize = crate::OOL_SLOTS - 1;
pub const SANDALWOOD_BOX_PICKUP_TAG: u8 = 0x0E;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndgameState {
    pub first_confirmation: Option<bool>,
    pub final_confirmation: Option<bool>,
    pub outcome: Option<EndgameOutcome>,
    pub certificate: Option<String>,
    pub cinematic: crate::endgame_cinematic::EndgameCinematic,
    pub messages: Option<EndgameMessages>,
    pub final_narrative: Option<EndNarrative>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndgameOutcome {
    Victory,
    MissingBoxOrRefused,
}

pub const ENDGAME_TABLEAU_WIDTH: usize = 11;
pub const ENDGAME_TABLEAU_HEIGHT: usize = 11;
pub const ENDGAME_TABLEAU_PARTY_START: (usize, usize) = (5, 9);
pub const ENDGAME_TABLEAU_LORD_BRITISH_SLOT: usize = 6;
pub const ENDGAME_TABLEAU_SCENE_MARKER_SLOT: usize = OOL_SLOTS - 1;
pub const ENDGAME_TABLEAU_LORD_BRITISH_TYPE: u8 = 0x0e;
pub const ENDGAME_TABLEAU_LORD_BRITISH_ORB_TYPE: u8 = 0x08;
pub const ENDGAME_TABLEAU_SCENE_MARKER_TYPE: u8 = 0x7c;
pub const ENDGAME_TABLEAU_PHASE: u8 = 0;

const ENDGAME_TABLEAU_PARTY_TARGETS: [(usize, usize); SAVE_PARTY_SIZE_MAX as usize] =
    [(5, 5), (4, 6), (6, 6), (3, 7), (5, 7), (7, 7)];
const ENDGAME_TABLEAU_LORD_BRITISH_POS: (usize, usize) = (5, 4);
const ENDGAME_TABLEAU_SCENE_MARKER_START: (usize, usize) = (5, 8);
const ENDGAME_TABLEAU_REFUSAL_SLOT0_TARGET: (usize, usize) = (8, 4);
const ENDGAME_TABLEAU_REFUSAL_SLOT2_TARGET: (usize, usize) = (8, 6);
const ENDGAME_TABLEAU_REFUSAL_MARKER_TARGET: (usize, usize) = (4, 1);
const ENDGAME_TABLEAU_VICTORY_EXIT_TARGET: (usize, usize) = (5, 4);
const ENDGAME_TABLEAU_SETTLE_STEP_CAP: usize = ENDGAME_TABLEAU_WIDTH * ENDGAME_TABLEAU_HEIGHT * 2;
pub const ENDGAME_TABLEAU_JITTER_SLOTS: [usize; 4] = [1, 3, 4, 5];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndgameTableauActorRole {
    PartyMember(u8),
    LordBritish,
    SceneMarker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EndgameTableauActorPlacement {
    pub role: EndgameTableauActorRole,
    pub active_object_slot: usize,
    pub start: (usize, usize),
    pub target: (usize, usize),
    pub tile: u8,
    pub type_byte: u8,
}

impl EndgameState {
    pub fn awaiting_first_confirmation() -> Self {
        Self::awaiting_first_confirmation_with_messages(None)
    }

    pub fn awaiting_first_confirmation_with_messages(messages: Option<EndgameMessages>) -> Self {
        Self {
            first_confirmation: None,
            final_confirmation: None,
            outcome: None,
            certificate: None,
            cinematic: crate::endgame_cinematic::EndgameCinematic::default(),
            messages,
            final_narrative: None,
        }
    }

    pub fn awaiting_final_confirmation(first_confirmation: bool) -> Self {
        Self::awaiting_final_confirmation_with_messages(first_confirmation, None)
    }

    pub fn awaiting_final_confirmation_with_messages(
        first_confirmation: bool,
        messages: Option<EndgameMessages>,
    ) -> Self {
        Self {
            first_confirmation: Some(first_confirmation),
            final_confirmation: None,
            outcome: None,
            certificate: None,
            cinematic: crate::endgame_cinematic::EndgameCinematic::default(),
            messages,
            final_narrative: None,
        }
    }

    pub fn terminal(
        first_confirmation: bool,
        final_confirmation: bool,
        has_sandalwood_box: bool,
        certificate: String,
        messages: Option<EndgameMessages>,
        final_narrative: Option<EndNarrative>,
    ) -> Self {
        let outcome = endgame_outcome(final_confirmation, has_sandalwood_box);
        let cinematic = if outcome == EndgameOutcome::Victory {
            let rite_count = messages
                .as_ref()
                .map(|messages| messages.rite_messages().len().min(u8::MAX as usize) as u8)
                .unwrap_or(0);
            crate::endgame_cinematic::EndgameCinematic::start_with_rite_messages(rite_count)
        } else {
            crate::endgame_cinematic::EndgameCinematic::default()
        };
        Self {
            first_confirmation: Some(first_confirmation),
            final_confirmation: Some(final_confirmation),
            outcome: Some(outcome),
            certificate: (outcome == EndgameOutcome::Victory).then_some(certificate),
            cinematic,
            messages,
            final_narrative: (outcome == EndgameOutcome::Victory)
                .then_some(final_narrative)
                .flatten(),
        }
    }

    /// Advance the post-victory cinematic by one keystroke. Returns
    /// the new step's banner label for caller-side display.
    pub fn advance_cinematic(&mut self) -> &'static str {
        self.cinematic.advance();
        self.cinematic.banner_label()
    }

    /// `true` when the post-victory cinematic has presented every
    /// screen and the engine should remain on the terminal final panel.
    pub fn cinematic_is_finished(&self) -> bool {
        self.cinematic.is_finished()
    }

    pub fn is_terminal(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn first_prompt_text(&self, leader_name: &str) -> String {
        if let Some(messages) = &self.messages {
            let mut lines = Vec::new();
            if let Some(greeting) = messages.initial_greeting() {
                lines.push(format!("{leader_name}: {greeting}"));
            }
            if let Some(prompt) = messages.first_box_prompt() {
                lines.push(prompt.to_string());
            }
            if !lines.is_empty() {
                lines.push("(Y/N)".to_string());
                return lines.join("\n");
            }
        }
        "Endgame: Lord British asks whether thou hast brought his box. (Y/N)".to_string()
    }

    pub fn second_prompt_text(&self, first_answer: bool) -> String {
        if let Some(messages) = &self.messages {
            if let Some(prompt) = messages.second_box_prompt() {
                return format!(
                    "Thou answered {}.\n{prompt}\n(Y/N)",
                    yes_no_word(first_answer)
                );
            }
        }
        "Endgame: Lord British asks again for the sandalwood box. (Y/N)".to_string()
    }

    pub fn refusal_text(&self) -> String {
        self.messages
            .as_ref()
            .and_then(|messages| messages.refusal_branch())
            .map(str::to_string)
            .unwrap_or_else(|| {
                "Endgame: the sandalwood box handoff failed; the ending tableau is terminal."
                    .to_string()
            })
    }

    pub fn current_cinematic_text(&self) -> String {
        match self.cinematic.step {
            crate::endgame_cinematic::EndgameCinematicStep::RiteMessage(index) => self
                .messages
                .as_ref()
                .and_then(|messages| messages.rite_messages().get(index as usize))
                .cloned()
                .unwrap_or_else(|| self.cinematic.banner_label().to_string()),
            crate::endgame_cinematic::EndgameCinematicStep::NarrativeWindow(index) => self
                .final_narrative
                .as_ref()
                .and_then(|narrative| narrative.window_by_number(index.saturating_add(1)))
                .unwrap_or_else(|| self.cinematic.banner_label().to_string()),
            crate::endgame_cinematic::EndgameCinematicStep::Certificate
            | crate::endgame_cinematic::EndgameCinematicStep::Finished => self
                .certificate
                .clone()
                .unwrap_or_else(|| self.cinematic.banner_label().to_string()),
            _ => self.cinematic.banner_label().to_string(),
        }
    }
}

/// `endgame.md §4`: returns `true` when a party slot needs the
/// dead-member restoration pass before the throne-room tableau. The
/// pass announces the restoration, flips the slot back to a Good
/// status, and restores current HP from the stored maximum. Only
/// Dead status triggers it; Ashes remains outside the restoration
/// per the documented per-slot loop, and other statuses keep
/// whatever HP/status they brought into the endgame.
pub const fn endgame_needs_tableau_restoration(status: CharacterStatus) -> bool {
    matches!(status, CharacterStatus::Dead)
}

pub fn endgame_outcome(final_confirmation: bool, has_sandalwood_box: bool) -> EndgameOutcome {
    if final_confirmation && has_sandalwood_box {
        EndgameOutcome::Victory
    } else {
        EndgameOutcome::MissingBoxOrRefused
    }
}

fn yes_no_word(answer: bool) -> &'static str {
    if answer { "yes" } else { "no" }
}

/// `endgame.md §9` certificate elapsed-time baseline. The certificate
/// computes campaign elapsed time by subtracting this fixed date from
/// the saved world clock under the thirteen-month / twenty-eight-day
/// calendar model. Year 139, month 4, day 5 corresponds to the
/// campaign-start date the certificate calls the "beginning of the
/// quest".
pub const ENDGAME_CAMPAIGN_START_YEAR: u16 = crate::CHARGEN_STARTING_YEAR;
pub const ENDGAME_CAMPAIGN_START_MONTH: u8 = crate::CHARGEN_STARTING_MONTH;
pub const ENDGAME_CAMPAIGN_START_DAY: u8 = crate::CHARGEN_STARTING_DAY;

pub fn endgame_elapsed_campaign_time(clock: GameClock) -> (u16, u8, u8) {
    let mut years = clock.year.saturating_sub(ENDGAME_CAMPAIGN_START_YEAR);
    let mut months = clock.month as i16 - ENDGAME_CAMPAIGN_START_MONTH as i16;
    let mut days = clock.day as i16 - ENDGAME_CAMPAIGN_START_DAY as i16;

    if days < 0 {
        days += DAYS_PER_MONTH as i16;
        months -= 1;
    }
    if months < 0 {
        months += MONTHS_PER_YEAR as i16;
        years = years.saturating_sub(1);
    }

    (years, months as u8, days as u8)
}

pub fn endgame_certificate_summary(leader_name: &str, clock: GameClock) -> String {
    let (years, months, days) = endgame_elapsed_campaign_time(clock);
    let day = endgame_ordinal_word(clock.day).unwrap_or_else(|| clock.day.to_string());
    let month = endgame_ordinal_word(clock.month).unwrap_or_else(|| clock.month.to_string());
    let year = endgame_cardinal_word(clock.year);
    format!(
        "Certificate: On the {day} day of the {month} month in the year {year}, {leader_name} restored Lord British, the people, and the land. Quest time: {}. Report this completed quest to Origin.",
        elapsed_time_label(years, months, days)
    )
}

pub fn endgame_ordinal_word(value: u8) -> Option<String> {
    let word = match value {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        11 => "eleventh",
        12 => "twelfth",
        13 => "thirteenth",
        14 => "fourteenth",
        15 => "fifteenth",
        16 => "sixteenth",
        17 => "seventeenth",
        18 => "eighteenth",
        19 => "nineteenth",
        20 => "twentieth",
        21 => "twenty-first",
        22 => "twenty-second",
        23 => "twenty-third",
        24 => "twenty-fourth",
        25 => "twenty-fifth",
        26 => "twenty-sixth",
        27 => "twenty-seventh",
        28 => "twenty-eighth",
        _ => return None,
    };
    Some(word.to_string())
}

pub fn endgame_cardinal_word(value: u16) -> String {
    match value {
        0 => "zero".to_string(),
        1..=19 => small_cardinal_word(value as u8).unwrap().to_string(),
        20..=99 => two_digit_cardinal_word(value as u8),
        100..=999 => {
            let hundreds = value / 100;
            let remainder = value % 100;
            if remainder == 0 {
                format!("{} hundred", endgame_cardinal_word(hundreds))
            } else {
                format!(
                    "{} hundred {}",
                    endgame_cardinal_word(hundreds),
                    endgame_cardinal_word(remainder)
                )
            }
        }
        1000..=9999 => {
            let thousands = value / 1000;
            let remainder = value % 1000;
            if remainder == 0 {
                format!("{} thousand", endgame_cardinal_word(thousands))
            } else {
                format!(
                    "{} thousand {}",
                    endgame_cardinal_word(thousands),
                    endgame_cardinal_word(remainder)
                )
            }
        }
        _ => value.to_string(),
    }
}

fn small_cardinal_word(value: u8) -> Option<&'static str> {
    Some(match value {
        0 => "zero",
        1 => "one",
        2 => "two",
        3 => "three",
        4 => "four",
        5 => "five",
        6 => "six",
        7 => "seven",
        8 => "eight",
        9 => "nine",
        10 => "ten",
        11 => "eleven",
        12 => "twelve",
        13 => "thirteen",
        14 => "fourteen",
        15 => "fifteen",
        16 => "sixteen",
        17 => "seventeen",
        18 => "eighteen",
        19 => "nineteen",
        _ => return None,
    })
}

fn two_digit_cardinal_word(value: u8) -> String {
    let tens = match value / 10 {
        2 => "twenty",
        3 => "thirty",
        4 => "forty",
        5 => "fifty",
        6 => "sixty",
        7 => "seventy",
        8 => "eighty",
        9 => "ninety",
        _ => return small_cardinal_word(value).unwrap_or("zero").to_string(),
    };
    match value % 10 {
        0 => tens.to_string(),
        ones => format!("{tens}-{}", small_cardinal_word(ones).unwrap()),
    }
}

pub fn elapsed_time_label(years: u16, months: u8, days: u8) -> String {
    let mut parts = Vec::new();
    if years != 0 {
        parts.push(format!("{years} {}", plural("year", years)));
    }
    if months != 0 {
        parts.push(format!("{months} {}", plural("month", u16::from(months))));
    }
    if days != 0 {
        parts.push(format!("{days} {}", plural("day", u16::from(days))));
    }
    if parts.is_empty() {
        "0 days".to_string()
    } else {
        parts.join(", ")
    }
}

fn plural(label: &'static str, amount: u16) -> &'static str {
    if amount == 1 {
        label
    } else {
        match label {
            "year" => "years",
            "month" => "months",
            "day" => "days",
            _ => label,
        }
    }
}

/// One-call grid step toward a target per `endgame.md` §7: examine the slot's
/// current `(x, y)` and choose the axis with the greater remaining distance.
/// Ties prefer the X axis. Returns the new `(x, y)`. When already on the
/// target, returns the input unchanged.
pub fn endgame_step_toward_target(
    current: (isize, isize),
    target: (isize, isize),
) -> (isize, isize) {
    let dx = target.0 - current.0;
    let dy = target.1 - current.1;
    if dx == 0 && dy == 0 {
        return current;
    }
    if dx.unsigned_abs() >= dy.unsigned_abs() {
        if dx == 0 {
            (current.0, current.1 + dy.signum())
        } else {
            (current.0 + dx.signum(), current.1)
        }
    } else if dy == 0 {
        (current.0 + dx.signum(), current.1)
    } else {
        (current.0, current.1 + dy.signum())
    }
}

pub const fn endgame_tableau_tile_for_class_byte(class_byte: u8) -> u8 {
    match class_byte {
        b'M' => 0x40,
        b'B' => 0x44,
        b'F' => 0x48,
        b'A' | b'D' | b'T' | b'P' | b'R' | b'S' => 0x4c,
        _ => 0x4c,
    }
}

pub const fn endgame_tableau_party_tile(tile: u8) -> bool {
    matches!(tile, 0x40 | 0x44 | 0x48 | 0x4c)
}

pub fn endgame_tableau_party_placement(
    party_slot: usize,
    class_byte: u8,
) -> Option<EndgameTableauActorPlacement> {
    let target = ENDGAME_TABLEAU_PARTY_TARGETS.get(party_slot).copied()?;
    let tile = endgame_tableau_tile_for_class_byte(class_byte);
    Some(EndgameTableauActorPlacement {
        role: EndgameTableauActorRole::PartyMember(party_slot as u8),
        active_object_slot: party_slot,
        start: ENDGAME_TABLEAU_PARTY_START,
        target,
        tile,
        type_byte: tile,
    })
}

pub fn endgame_tableau_lord_british_placement() -> EndgameTableauActorPlacement {
    EndgameTableauActorPlacement {
        role: EndgameTableauActorRole::LordBritish,
        active_object_slot: ENDGAME_TABLEAU_LORD_BRITISH_SLOT,
        start: ENDGAME_TABLEAU_LORD_BRITISH_POS,
        target: ENDGAME_TABLEAU_LORD_BRITISH_POS,
        tile: ENDGAME_TABLEAU_LORD_BRITISH_TYPE,
        type_byte: ENDGAME_TABLEAU_LORD_BRITISH_TYPE,
    }
}

pub fn endgame_tableau_scene_marker_placement(
    target: (usize, usize),
) -> EndgameTableauActorPlacement {
    EndgameTableauActorPlacement {
        role: EndgameTableauActorRole::SceneMarker,
        active_object_slot: ENDGAME_TABLEAU_SCENE_MARKER_SLOT,
        start: ENDGAME_TABLEAU_SCENE_MARKER_START,
        target,
        tile: ENDGAME_TABLEAU_SCENE_MARKER_TYPE,
        type_byte: ENDGAME_TABLEAU_SCENE_MARKER_TYPE,
    }
}

pub fn endgame_tableau_actor_placements(
    party: &[PartyMember],
) -> Vec<EndgameTableauActorPlacement> {
    let mut placements = party
        .iter()
        .take(SAVE_PARTY_SIZE_MAX as usize)
        .enumerate()
        .filter_map(|(slot, member)| endgame_tableau_party_placement(slot, member.class_byte))
        .collect::<Vec<_>>();
    placements.push(endgame_tableau_scene_marker_placement(
        ENDGAME_TABLEAU_SCENE_MARKER_START,
    ));
    placements
}

pub fn endgame_tableau_cell_walkable(x: usize, y: usize) -> bool {
    x > 0
        && x + 1 < ENDGAME_TABLEAU_WIDTH
        && (3..ENDGAME_TABLEAU_HEIGHT).contains(&y)
        && !(y == 3 && !(3..=7).contains(&x))
}

pub fn endgame_tableau_role_for_slot(
    slot: usize,
    object: ActiveObject,
) -> Option<EndgameTableauActorRole> {
    if object.is_empty() {
        return None;
    }
    if slot < SAVE_PARTY_SIZE_MAX as usize
        && object.type_byte == object.tile
        && endgame_tableau_party_tile(object.type_byte)
    {
        Some(EndgameTableauActorRole::PartyMember(slot as u8))
    } else if slot == ENDGAME_TABLEAU_LORD_BRITISH_SLOT
        && object.type_byte == object.tile
        && matches!(
            object.type_byte,
            ENDGAME_TABLEAU_LORD_BRITISH_TYPE | ENDGAME_TABLEAU_LORD_BRITISH_ORB_TYPE
        )
    {
        Some(EndgameTableauActorRole::LordBritish)
    } else if slot == ENDGAME_TABLEAU_SCENE_MARKER_SLOT
        && object.type_byte == ENDGAME_TABLEAU_SCENE_MARKER_TYPE
        && object.tile == ENDGAME_TABLEAU_SCENE_MARKER_TYPE
    {
        Some(EndgameTableauActorRole::SceneMarker)
    } else {
        None
    }
}

pub fn endgame_tableau_object_from_placement(
    placement: EndgameTableauActorPlacement,
) -> ActiveObject {
    let mut object = ActiveObject::empty();
    write_endgame_tableau_placement(&mut object, placement);
    object
}

fn write_endgame_tableau_placement(
    object: &mut ActiveObject,
    placement: EndgameTableauActorPlacement,
) {
    object.type_byte = placement.type_byte;
    object.tile = placement.tile;
    object.x = placement.start.0;
    object.y = placement.start.1;
    object.phase = ENDGAME_TABLEAU_PHASE;
}

impl PlayState {
    pub fn enter_endgame(&mut self) -> MoveOutcome {
        self.enter_endgame_with_messages(None)
    }

    pub fn enter_endgame_from_game_dir(
        &mut self,
        game_dir: Option<&std::path::Path>,
    ) -> std::io::Result<MoveOutcome> {
        let messages = game_dir.map(require_endgame_messages).transpose()?;
        Ok(self.enter_endgame_with_messages(messages))
    }

    pub fn enter_endgame_with_messages(
        &mut self,
        messages: Option<EndgameMessages>,
    ) -> MoveOutcome {
        self.pending_moongate = None;
        self.combat_active = false;
        self.pending_combat_actor_slot = None;
        self.pending_combat_terrain_trigger_slot = None;
        // endgame.md §10: dead party members are mutated into a present /
        // restored state for the ending tableau, with current health restored
        // from the stored maximum.
        self.restore_party_for_endgame_tableau();
        self.install_endgame_tableau();
        self.settle_endgame_tableau_to_targets();
        self.endgame = Some(EndgameState::awaiting_first_confirmation_with_messages(
            messages,
        ));
        self.message = self
            .endgame
            .as_ref()
            .expect("endgame state was just installed")
            .first_prompt_text(&self.party_leader_name());
        MoveOutcome::EndgameEntered
    }

    pub fn ensure_endgame_messages_loaded(
        &mut self,
        game_dir: &std::path::Path,
    ) -> std::io::Result<()> {
        let Some(state) = self.endgame.as_mut() else {
            return Ok(());
        };
        if state.messages.is_none() {
            state.messages = load_endgame_messages(game_dir)?;
        }
        Ok(())
    }

    /// endgame.md section 10: restore Dead travelling-party members to Good status with
    /// HP equal to their stored maximum. Non-Dead statuses keep their entry
    /// state. This mutation is cinematic only and is not committed to disk.
    pub fn restore_party_for_endgame_tableau(&mut self) {
        for member in &mut self.party {
            if character_status_for_byte(member.status)
                .is_some_and(endgame_needs_tableau_restoration)
            {
                member.status = CharacterStatus::Good.save_byte();
                member.hp = member.max_hp;
            }
        }
    }

    pub fn install_endgame_tableau(&mut self) {
        let placements = endgame_tableau_actor_placements(&self.party);
        self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        for object in &mut self.active_objects {
            object.type_byte = 0;
            object.tile = 0;
        }
        for placement in placements {
            if let Some(slot) = self.active_objects.get_mut(placement.active_object_slot) {
                write_endgame_tableau_placement(slot, placement);
            }
        }
    }

    pub fn advance_endgame_tableau_toward_targets(&mut self) -> bool {
        let placements = endgame_tableau_actor_placements(&self.party);
        let mut moved = false;
        for placement in placements {
            let Some(object) = self.active_objects.get_mut(placement.active_object_slot) else {
                continue;
            };
            if endgame_tableau_role_for_slot(placement.active_object_slot, *object)
                != Some(placement.role)
            {
                continue;
            }
            let current = (object.x as isize, object.y as isize);
            let target = (placement.target.0 as isize, placement.target.1 as isize);
            let next = endgame_step_toward_target(current, target);
            if next != current {
                object.x = next.0 as usize;
                object.y = next.1 as usize;
                moved = true;
            }
        }
        moved
    }

    pub fn settle_endgame_tableau_to_targets(&mut self) -> usize {
        let mut steps = 0;
        while steps < ENDGAME_TABLEAU_SETTLE_STEP_CAP
            && self.advance_endgame_tableau_toward_targets()
        {
            steps += 1;
        }
        steps
    }

    fn step_endgame_tableau_slot_to_target(
        &mut self,
        slot: usize,
        target: (usize, usize),
    ) -> usize {
        let mut steps = 0;
        while steps < ENDGAME_TABLEAU_SETTLE_STEP_CAP {
            let Some(object) = self.active_objects.get_mut(slot) else {
                break;
            };
            if object.is_empty() {
                break;
            }
            let current = (object.x as isize, object.y as isize);
            let target = (target.0 as isize, target.1 as isize);
            let next = endgame_step_toward_target(current, target);
            if next == current {
                break;
            }
            object.x = next.0 as usize;
            object.y = next.1 as usize;
            self.animation.tick_static_tiles();
            steps += 1;
        }
        steps
    }

    fn clear_endgame_tableau_slot_type_tile(&mut self, slot: usize) {
        if let Some(object) = self.active_objects.get_mut(slot) {
            object.type_byte = 0;
            object.tile = 0;
        }
    }

    pub fn prepare_endgame_refusal_tableau(&mut self) {
        if let Some(object) = self.active_objects.get_mut(0) {
            if endgame_tableau_role_for_slot(0, *object)
                == Some(EndgameTableauActorRole::PartyMember(0))
            {
                object.y = object.y.saturating_sub(1);
            }
        }
        loop {
            let moved = self
                .step_endgame_tableau_slot_to_target(2, ENDGAME_TABLEAU_REFUSAL_SLOT2_TARGET)
                + self.step_endgame_tableau_slot_to_target(
                    ENDGAME_TABLEAU_SCENE_MARKER_SLOT,
                    ENDGAME_TABLEAU_REFUSAL_MARKER_TARGET,
                )
                + self.step_endgame_tableau_slot_to_target(0, ENDGAME_TABLEAU_REFUSAL_SLOT0_TARGET);
            if moved == 0 {
                break;
            }
        }
    }

    pub fn prepare_endgame_victory_tableau(&mut self) {
        self.step_endgame_tableau_slot_to_target(0, ENDGAME_TABLEAU_VICTORY_EXIT_TARGET);
        self.step_endgame_tableau_slot_to_target(0, ENDGAME_TABLEAU_PARTY_TARGETS[0]);
        if self.active_objects.len() < OOL_SLOTS {
            self.active_objects.resize(OOL_SLOTS, ActiveObject::empty());
        }
        if let Some(slot) = self
            .active_objects
            .get_mut(ENDGAME_TABLEAU_LORD_BRITISH_SLOT)
        {
            write_endgame_tableau_placement(slot, endgame_tableau_lord_british_placement());
        }
    }

    pub fn complete_endgame_victory_tableau(&mut self) {
        if let Some(lord_british) = self
            .active_objects
            .get_mut(ENDGAME_TABLEAU_LORD_BRITISH_SLOT)
        {
            if endgame_tableau_role_for_slot(ENDGAME_TABLEAU_LORD_BRITISH_SLOT, *lord_british)
                == Some(EndgameTableauActorRole::LordBritish)
            {
                lord_british.type_byte = ENDGAME_TABLEAU_LORD_BRITISH_ORB_TYPE;
                lord_british.tile = ENDGAME_TABLEAU_LORD_BRITISH_ORB_TYPE;
            }
        }
        self.clear_endgame_tableau_slot_type_tile(ENDGAME_TABLEAU_LORD_BRITISH_SLOT);
        self.step_endgame_tableau_slot_to_target(
            ENDGAME_TABLEAU_SCENE_MARKER_SLOT,
            ENDGAME_TABLEAU_VICTORY_EXIT_TARGET,
        );
        self.clear_endgame_tableau_slot_type_tile(ENDGAME_TABLEAU_SCENE_MARKER_SLOT);
        for slot in 0..self.party.len().min(SAVE_PARTY_SIZE_MAX as usize) {
            self.step_endgame_tableau_slot_to_target(slot, ENDGAME_TABLEAU_VICTORY_EXIT_TARGET);
            self.clear_endgame_tableau_slot_type_tile(slot);
        }
    }

    pub fn endgame_tableau_is_settled(&self) -> bool {
        endgame_tableau_actor_placements(&self.party)
            .into_iter()
            .all(|placement| {
                self.active_objects
                    .get(placement.active_object_slot)
                    .copied()
                    .is_some_and(|object| {
                        endgame_tableau_role_for_slot(placement.active_object_slot, object)
                            == Some(placement.role)
                            && (object.x, object.y) == placement.target
                    })
            })
    }

    pub fn advance_endgame_terminal_tableau_jitter(&mut self) -> bool {
        let party_slots = self.party.len().min(SAVE_PARTY_SIZE_MAX as usize);
        let eligible = ENDGAME_TABLEAU_JITTER_SLOTS
            .iter()
            .copied()
            .filter(|slot| *slot < party_slots)
            .filter(|slot| {
                self.active_objects
                    .get(*slot)
                    .copied()
                    .is_some_and(|object| {
                        endgame_tableau_role_for_slot(*slot, object)
                            == Some(EndgameTableauActorRole::PartyMember(*slot as u8))
                    })
            })
            .collect::<Vec<_>>();
        if eligible.is_empty() || u5_prng_range_u16(&mut self.prng_state, 0, 1) != 0 {
            self.animation.tick_static_tiles();
            return false;
        }
        let actor_slot = eligible
            [u5_prng_range_u16(&mut self.prng_state, 0, eligible.len() as u16 - 1) as usize];
        for _ in 0..8 {
            let dir = u5_prng_range_u16(&mut self.prng_state, 0, 3) as u8;
            let (dx, dy) = match dir {
                0 => (1isize, 0isize),
                1 => (-1, 0),
                2 => (0, 1),
                _ => (0, -1),
            };
            let Some(object) = self.active_objects.get(actor_slot).copied() else {
                self.animation.tick_static_tiles();
                return false;
            };
            if object.is_empty() {
                self.animation.tick_static_tiles();
                return false;
            }
            let nx = object.x as isize + dx;
            let ny = object.y as isize + dy;
            if nx < 0 || ny < 0 {
                continue;
            }
            let nx = nx as usize;
            let ny = ny as usize;
            if !endgame_tableau_cell_walkable(nx, ny) {
                continue;
            }
            if let Some(object) = self.active_objects.get_mut(actor_slot) {
                object.x = nx;
                object.y = ny;
            }
            self.animation.tick_static_tiles();
            return true;
        }
        self.animation.tick_static_tiles();
        false
    }

    pub fn resolve_endgame_confirmation(&mut self, answer: bool) -> MoveOutcome {
        self.resolve_endgame_confirmation_with_narrative(answer, None)
    }

    pub fn resolve_endgame_confirmation_from_game_dir(
        &mut self,
        answer: bool,
        game_dir: &std::path::Path,
    ) -> std::io::Result<MoveOutcome> {
        let needs_final_narrative = self.endgame.as_ref().is_some_and(|state| {
            !state.is_terminal()
                && state.first_confirmation.is_some()
                && answer
                && self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0
        });
        let final_narrative = if needs_final_narrative {
            Some(require_end_narrative(game_dir)?)
        } else {
            None
        };
        Ok(self.resolve_endgame_confirmation_with_narrative(answer, final_narrative))
    }

    fn resolve_endgame_confirmation_with_narrative(
        &mut self,
        answer: bool,
        final_narrative: Option<EndNarrative>,
    ) -> MoveOutcome {
        let Some(current) = self.endgame.clone() else {
            self.message = "No endgame confirmation is pending.".to_string();
            return MoveOutcome::Blocked;
        };
        if current.is_terminal() {
            // Victory branch advances the cinematic page-flip on every
            // keystroke until the closer is finished, then keeps the terminal
            // final panel active.
            if matches!(current.outcome, Some(EndgameOutcome::Victory)) {
                if let Some(state) = self.endgame.as_mut() {
                    if state.cinematic_is_finished() {
                        self.message = state.current_cinematic_text();
                    } else {
                        let previous_step = state.cinematic.step;
                        state.advance_cinematic();
                        let next_step = state.cinematic.step;
                        self.message = state.current_cinematic_text();
                        if matches!(
                            (previous_step, next_step),
                            (
                                crate::endgame_cinematic::EndgameCinematicStep::RiteMessage(_),
                                crate::endgame_cinematic::EndgameCinematicStep::ThroneTableau
                            ) | (
                                crate::endgame_cinematic::EndgameCinematicStep::ThroneTableau,
                                crate::endgame_cinematic::EndgameCinematicStep::NarrativeWindow(0)
                            )
                        ) {
                            self.complete_endgame_victory_tableau();
                        }
                    }
                    return MoveOutcome::Observed;
                }
            }
            if matches!(current.outcome, Some(EndgameOutcome::MissingBoxOrRefused)) {
                self.advance_endgame_terminal_tableau_jitter();
            }
            self.message = match current.outcome {
                Some(EndgameOutcome::Victory) => current
                    .certificate
                    .unwrap_or_else(|| "The victory ending is complete.".to_string()),
                Some(EndgameOutcome::MissingBoxOrRefused) => {
                    "Lord British waits with thee in the ending tableau.".to_string()
                }
                None => unreachable!("terminal endgame has an outcome"),
            };
            return MoveOutcome::Observed;
        }

        if current.first_confirmation.is_none() {
            self.endgame = Some(EndgameState::awaiting_final_confirmation_with_messages(
                answer,
                current.messages,
            ));
            self.message = self
                .endgame
                .as_ref()
                .expect("endgame state was just installed")
                .second_prompt_text(answer);
            return MoveOutcome::Used;
        }

        let first = current.first_confirmation.unwrap_or(false);
        let has_box = self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] != 0;
        let certificate = endgame_certificate_summary(&self.party_leader_name(), self.clock);
        let next = EndgameState::terminal(
            first,
            answer,
            has_box,
            certificate.clone(),
            current.messages,
            final_narrative,
        );
        match next.outcome {
            Some(EndgameOutcome::Victory) => self.prepare_endgame_victory_tableau(),
            Some(EndgameOutcome::MissingBoxOrRefused) => self.prepare_endgame_refusal_tableau(),
            None => {}
        }
        self.message = match next.outcome {
            Some(EndgameOutcome::Victory) => {
                if next
                    .messages
                    .as_ref()
                    .is_some_and(|messages| !messages.rite_messages().is_empty())
                {
                    next.current_cinematic_text()
                } else {
                    certificate
                }
            }
            Some(EndgameOutcome::MissingBoxOrRefused) => next.refusal_text(),
            None => unreachable!("terminal endgame has an outcome"),
        };
        self.endgame = Some(next);
        MoveOutcome::EndgameEntered
    }

    pub fn party_leader_name(&self) -> String {
        self.party_names
            .first()
            .and_then(|name| party_name_to_string(name))
            .unwrap_or_else(|| "Avatar".to_string())
    }
}

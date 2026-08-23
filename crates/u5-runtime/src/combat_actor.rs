//! Combat actor descriptor rows.

use crate::*;

/// `active-objects.md §7`: combat actor table is a 32-record
/// view over the active-object table. Anchored to
/// [`crate::OOL_SLOTS`] so the combat-side slot count and the
/// .OOL record count share one source of truth.
pub const COMBAT_ACTOR_SLOTS: usize = crate::OOL_SLOTS;
/// `combat.md §3`: the combat actor table reserves party slots
/// (one per travelling member). Anchored to
/// [`crate::SAVE_PARTY_SIZE_MAX`] so the combat party cap and
/// the save-file roster cap stay one value.
pub const COMBAT_PARTY_ACTOR_SLOTS: usize = crate::SAVE_PARTY_SIZE_MAX as usize;
/// `active-objects.md §7`: combat caps total combatants (party + monsters)
/// at twenty-six. Monster placement runs in slots 1..=25.
pub const COMBAT_MONSTER_SLOT_FIRST: usize = 1;
pub const COMBAT_MONSTER_SLOT_LAST: usize = 25;
/// `active-objects.md §7`: combat caps total combatants at the
/// last monster slot index plus one (slots 0..=25 = 26 records).
/// Anchored to [`COMBAT_MONSTER_SLOT_LAST`] + 1 so resizing the
/// monster band only happens in one place.
pub const COMBAT_MAX_COMBATANTS: usize = COMBAT_MONSTER_SLOT_LAST + 1;
/// `active-objects.md §7`: each combat actor record shares the
/// 8-byte active-object record layout. Anchored to
/// [`crate::OOL_RECORD_LEN`] so the combat-side record stride
/// and the format-side record length share one value.
pub const COMBAT_ACTOR_RECORD_LEN: usize = crate::OOL_RECORD_LEN;

/// `combat.md §12` per-kill raw experience-reward unit. Each monster
/// killed produces "roughly a quarter of max-HP plus one" credited to
/// the killing party member's experience word (capped at 9999 by the
/// caller's standard arithmetic). The unit is also reused by spell-side
/// multi-target callers like Tremor.
pub const fn monster_kill_xp_reward(class_max_hp: u16) -> u16 {
    (class_max_hp / 4).saturating_add(1)
}

/// `combat.md §11` Fire Field per-contact raw-damage roll. The
/// post-step contact hook rolls a uniform `[1, 21]` value before the
/// normal random defense subtraction. Caller passes the raw `0..=20`
/// PRNG seed.
pub const FIRE_FIELD_DAMAGE_MIN: u8 = 1;
pub const FIRE_FIELD_DAMAGE_MAX: u8 = 21;
pub const fn fire_field_raw_damage(roll_seed_0_to_20: u8) -> u8 {
    FIRE_FIELD_DAMAGE_MIN
        + (roll_seed_0_to_20 % (FIRE_FIELD_DAMAGE_MAX - FIRE_FIELD_DAMAGE_MIN + 1))
}

/// `combat.md §11` Energy Field raw damage. Energy contact supplies
/// raw zero to the same damage/value path; the final hit value is
/// produced by the random defense subtraction alone.
pub const ENERGY_FIELD_RAW_DAMAGE: u8 = 0;

/// `combat.md §9` four-bucket wound classification produced by the
/// monster wound-score classifier. The classifier consumes the acting
/// monster's current HP against its class maximum and feeds AI
/// fleeing/morale decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonsterWoundBucket {
    /// HP `< 1/4 max` — always-fleeing critical band.
    Critical,
    /// HP in `[1/4, 1/2)` — morale-check band; fleeing on 252/256.
    Wounded,
    /// HP in `[1/2, 3/4)` — light wounds.
    LightlyWounded,
    /// HP `>= 3/4 max` — healthy.
    Healthy,
}

/// `combat.md §9` morale-check probability over a `0..=255` roll.
/// In the wounded band, fleeing is set on 252 of the 256 possible
/// results.
pub const WOUND_MORALE_FLEE_THRESHOLD: u16 = 252;

/// `combat.md §9` monster wound-score classifier. Returns the
/// four-bucket wound classification.
pub const fn monster_wound_bucket(current_hp: u16, class_max_hp: u16) -> MonsterWoundBucket {
    if class_max_hp == 0 {
        return MonsterWoundBucket::Critical;
    }
    let quarter = class_max_hp / 4;
    let half = class_max_hp / 2;
    let three_quarters = (class_max_hp * 3) / 4;
    if current_hp < quarter {
        MonsterWoundBucket::Critical
    } else if current_hp < half {
        MonsterWoundBucket::Wounded
    } else if current_hp < three_quarters {
        MonsterWoundBucket::LightlyWounded
    } else {
        MonsterWoundBucket::Healthy
    }
}

/// `combat.md §9` morale verdict for the wounded band. Below 1/4 the
/// classifier always sets fleeing; in [1/4, 1/2) it sets fleeing
/// when the morale roll is `< 252` (252 of 256 outcomes); at or above
/// 1/2 it clears fleeing regardless of the roll.
pub const fn monster_wound_sets_fleeing(
    current_hp: u16,
    class_max_hp: u16,
    morale_roll_0_to_255: u8,
) -> bool {
    match monster_wound_bucket(current_hp, class_max_hp) {
        MonsterWoundBucket::Critical => true,
        MonsterWoundBucket::Wounded => (morale_roll_0_to_255 as u16) < WOUND_MORALE_FLEE_THRESHOLD,
        MonsterWoundBucket::LightlyWounded | MonsterWoundBucket::Healthy => false,
    }
}

/// `combat.md §6` byte offsets inside the eight-byte combat actor
/// descriptor. The decoded row order is HP/wound counter, base-step,
/// flags/faction, owner/target/class, active-object back-reference,
/// phase counter, arena X, and arena Y.
pub const COMBAT_ACTOR_HP_OFFSET: usize = 0;
pub const COMBAT_ACTOR_BASE_STEP_OFFSET: usize = 1;
pub const COMBAT_ACTOR_FLAGS_OFFSET: usize = 2;
pub const COMBAT_ACTOR_OWNER_TARGET_CLASS_OFFSET: usize = 3;
pub const COMBAT_ACTOR_BACKREF_OFFSET: usize = 4;
pub const COMBAT_ACTOR_PHASE_OFFSET: usize = 5;
pub const COMBAT_ACTOR_X_OFFSET: usize = 6;
pub const COMBAT_ACTOR_Y_OFFSET: usize = 7;

/// `combat.md §6` typed combat-actor field selector. Pairs with the
/// `COMBAT_ACTOR_*_OFFSET` constants for callers that prefer enum
/// dispatch over raw indexing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatActorField {
    Hp,
    BaseStep,
    Flags,
    OwnerTargetClass,
    Backref,
    Phase,
    ArenaX,
    ArenaY,
}

impl CombatActorField {
    pub const fn offset(self) -> usize {
        match self {
            Self::Hp => COMBAT_ACTOR_HP_OFFSET,
            Self::BaseStep => COMBAT_ACTOR_BASE_STEP_OFFSET,
            Self::Flags => COMBAT_ACTOR_FLAGS_OFFSET,
            Self::OwnerTargetClass => COMBAT_ACTOR_OWNER_TARGET_CLASS_OFFSET,
            Self::Backref => COMBAT_ACTOR_BACKREF_OFFSET,
            Self::Phase => COMBAT_ACTOR_PHASE_OFFSET,
            Self::ArenaX => COMBAT_ACTOR_X_OFFSET,
            Self::ArenaY => COMBAT_ACTOR_Y_OFFSET,
        }
    }
}

/// `combat.md §9` Pass-2 monster class-flag ability bits, tested in
/// fixed order: possess/charm-on-turn first, then blink/phase, then
/// summon-daemon. Variant classes carrying multiple bits attempt
/// possess first.
pub const MONSTER_ABILITY_POSSESS: u16 = 0x0040;
pub const MONSTER_ABILITY_BLINK: u16 = 0x0800;
pub const MONSTER_ABILITY_SUMMON_DAEMON: u16 = 0x0400;

/// `combat.md §9` monster class-flag ability bit tested in turn-pass 2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MonsterAbility {
    /// `0x0040` — possess/charm-on-turn.
    Possess,
    /// `0x0800` — blink/phase (~1-in-8 toggle).
    Blink,
    /// `0x0400` — summon-daemon (~1-in-8 placement attempt).
    SummonDaemon,
}

/// `combat.md §9`: select the monster ability the Pass-2 hook attempts
/// first for the given class flag word. The fixed branch order is
/// possess → blink → summon-daemon, so a class with multiple bits
/// returns possess. Returns `None` when no ability bit is set.
pub const fn first_monster_ability(class_flags: u16) -> Option<MonsterAbility> {
    if class_flags & MONSTER_ABILITY_POSSESS != 0 {
        Some(MonsterAbility::Possess)
    } else if class_flags & MONSTER_ABILITY_BLINK != 0 {
        Some(MonsterAbility::Blink)
    } else if class_flags & MONSTER_ABILITY_SUMMON_DAEMON != 0 {
        Some(MonsterAbility::SummonDaemon)
    } else {
        None
    }
}

/// `combat.md` §6.1 / public issue #6: controlled/player-command gate.
pub const COMBAT_ACTOR_FLAG_CONTROLLED: u8 = 0x01;
/// Back-compatible name for the low controlled/charm bit.
pub const COMBAT_ACTOR_FLAG_TEAM_TOGGLE: u8 = COMBAT_ACTOR_FLAG_CONTROLLED;
/// `combat.md` §6.1 / public issue #7: flee-step inversion bit.
pub const COMBAT_ACTOR_FLAG_FLEEING: u8 = 0x02;
/// `combat.md` §6.2 / public issue #8: asleep/charmed/disabled skip bit.
pub const COMBAT_ACTOR_FLAG_STATUS_DISABLED: u8 = 0x08;
/// `combat.md` §6.2: disabled actors wake only on their own-turn 0..16 roll.
pub const COMBAT_SLEEP_WAKE_ROLL_LOW: u8 = 0;
pub const COMBAT_SLEEP_WAKE_ROLL_HIGH: u8 = 16;
pub const COMBAT_SLEEP_WAKE_SUCCESS_ROLL: u8 = 16;
pub const COMBAT_ACTOR_FLAG_SELECTABLE_80: u8 = 0x80;
pub const COMBAT_ACTOR_FLAG_SELECTABLE_40: u8 = 0x40;
pub const COMBAT_ACTOR_FLAG_MARKED_DEAD: u8 = 0x20;
pub const COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED: u8 = 0x04;
pub const COMBAT_SWARM_JITTER_ROLL_MAX: u8 = 4;
pub const COMBAT_SWARM_JITTER_CENTER_ROLL: u8 = 2;
pub const COMBAT_NO_TARGET_FLEE_MIN_SLOT: usize = 5;
pub const COMBAT_NO_TARGET_FLEE_MAX_SLOT: usize = 25;
pub const COMBAT_NO_TARGET_FLEE_STEP_QUEUE: u8 = 1;
pub const COMBAT_FIELD_REJECTED_ACTIVE_OBJECT_TILE: u8 = 0xf4;
pub const COMBAT_INSTANT_KILL_DAMAGE: i16 = 99;
/// `combat.md §12`: default monster death drop gates use the
/// class drop-cap byte as a percentage over a `0..=99` roll.
pub const COMBAT_DEFAULT_DEATH_DROP_ROLL_MAX: u8 = 99;
/// `combat.md` death-marker table: party-member corpse marker.
pub const COMBAT_PARTY_CORPSE_TILE: u8 = 0x1E;
/// `combat.md` death-marker table: default monster death/drop marker.
/// Drop and no-drop outcomes share this tile; byte five records
/// promoted loot when a drop gate accepts.
pub const COMBAT_DEFAULT_DEATH_MARKER_TILE: u8 = 0x01;
/// Back-compatible name for the default monster death/drop marker.
pub const COMBAT_DEFAULT_DEATH_DROP_TILE: u8 = COMBAT_DEFAULT_DEATH_MARKER_TILE;
/// Back-compatible name for the default monster death/no-drop marker.
pub const COMBAT_DEFAULT_DEATH_NO_DROP_TILE: u8 = COMBAT_DEFAULT_DEATH_MARKER_TILE;
/// `combat.md` death-marker table: vanish-on-death marker.
pub const COMBAT_VANISH_DEATH_MARKER_TILE: u8 = 0x16;
/// `combat.md` death-marker table: Gazer eye-burst marker.
pub const COMBAT_GAZER_DEATH_MARKER_TILE: u8 = 0x1F;
/// `combat.md` death-marker table: Gargoyle lava terrain tile.
pub const COMBAT_GARGOYLE_DEATH_TERRAIN_TILE: u8 = 0x4C;
/// `catalogs/spell-list.md §5` Magic Missile raw damage roll cap.
/// Anchored to [`crate::MAGIC_MISSILE_RAW_DAMAGE_MAX`] so the
/// combat-side roll cap and the spell-list-side raw cap stay one
/// value.
pub const COMBAT_MAGIC_MISSILE_DAMAGE_ROLL_MAX: u8 = crate::MAGIC_MISSILE_RAW_DAMAGE_MAX;
/// `catalogs/spell-list.md §5` Fireball raw damage roll cap.
/// Anchored to [`crate::FIREBALL_RAW_DAMAGE_MAX`] so the
/// combat-side roll cap and the spell-list-side raw cap stay one
/// value.
pub const COMBAT_FIREBALL_DAMAGE_ROLL_MAX: u8 = crate::FIREBALL_RAW_DAMAGE_MAX;
pub const COMBAT_TREMOR_DAMAGE_ROLL_MAX: u8 = 20;
pub const COMBAT_FLAME_WIND_DAMAGE_ROLL_MAX: u8 = 30;
/// `magic.md §8`: Protection's `P` active-effect tag adds this many
/// points to the resident party-member defense helper after equipment
/// defense is summed. The bonus is applied through saturating_add so
/// a defense byte already near `0xFF` does not wrap.
pub const PROTECTION_ACTIVE_EFFECT_DEFENSE_BONUS: u8 = 3;
/// `combat.md §5` per-attacker experience cap. Each monster-kill
/// or spell-cast experience credit clamps at this word-sized
/// counter cap, identical to the gold-counter convention.
/// Anchored to [`crate::PARTY_GOLD_CAP`] so the experience cap
/// and the inventory.md §2 "9999" word-counter cap share one
/// source of truth.
pub const COMBAT_EXPERIENCE_CAP: u16 = crate::PARTY_GOLD_CAP;
pub const COMBAT_TARGET_PICK_COUNTED_PARTY_SLOTS: usize = 5;
pub const COMBAT_ROUND_COUNTER_WRAP: u8 = 10;
pub const COMBAT_ROUND_WRAP_TIME_ADVANCE_MINUTES: u8 = 1;
pub const COMBAT_CLASS_GUARD: u8 = 12;
pub const COMBAT_CLASS_GUARD_SPRITE_BASE: u8 = 0x70;
pub const COMBAT_CLASS_WANDERER: u8 = COMBAT_CLASS_GUARD + 1;
pub const COMBAT_CLASS_BLACKTHORN: u8 = COMBAT_CLASS_WANDERER + 1;
pub const COMBAT_CLASS_LORD_BRITISH: u8 = COMBAT_CLASS_BLACKTHORN + 1;
pub const COMBAT_CLASS_GIANT_RAT: u8 = 20;
pub const COMBAT_CLASS_GIANT_RAT_SPRITE_BASE: u8 = 0x90;
/// `catalogs/monster-bestiary.md §2` consecutive small-monster
/// combat class ids (Giant Rat 20 / Bat 21 / Giant Spider 22).
/// Anchor each successor to the chain.
pub const COMBAT_CLASS_BAT: u8 = COMBAT_CLASS_GIANT_RAT + 1;
pub const COMBAT_CLASS_GIANT_SPIDER: u8 = COMBAT_CLASS_BAT + 1;
pub const COMBAT_CLASS_INSECT_SWARM: u8 = 31;
pub const COMBAT_CLASS_PYTHON: u8 = 34;
pub const COMBAT_CLASS_DAEMON: u8 = 38;
/// `catalogs/monster-bestiary.md §2`: Dragon (39) follows Daemon
/// (38) consecutively. Anchor DRAGON to DAEMON + 1.
pub const COMBAT_CLASS_DRAGON: u8 = COMBAT_CLASS_DAEMON + 1;
pub const COMBAT_CLASS_SHADOW_LORD: u8 = 47;
pub const COMBAT_SUMMONED_ACTOR_FLAGS: u8 =
    COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_CONTROLLED;
/// `magic.md §8` Conjure spell outcome bound — fifteen weighted
/// outcomes. Same fundamental count as
/// [`crate::CONJURE_OUTCOME_COUNT`] in magic.rs; anchored
/// through to that constant so the per-conjure handler and the
/// spec-rooted Conjure-outcome anchor share one source of truth.
pub const CONJURE_ANIMAL_OUTCOME_COUNT: u8 = crate::CONJURE_OUTCOME_COUNT;
/// `combat.md §11` field-kind byte enumeration. Poison, Sleep,
/// Fire, and Energy field kinds occupy four consecutive class
/// bytes 0x33..=0x36. Anchor each successor to the chain.
pub const COMBAT_FIELD_KIND_POISON: u8 = 0x33;
pub const COMBAT_FIELD_KIND_SLEEP: u8 = COMBAT_FIELD_KIND_POISON + 1;
pub const COMBAT_FIELD_KIND_FIRE: u8 = COMBAT_FIELD_KIND_SLEEP + 1;
pub const COMBAT_FIELD_KIND_ENERGY: u8 = COMBAT_FIELD_KIND_FIRE + 1;
pub const COMBAT_FIELD_CURSOR_RANGE: u8 = (COMBAT_ARENA_SIDE - 1) as u8;
pub const COMBAT_ROUND_RESULT_DEFEAT: u8 = 0;
pub const COMBAT_ROUND_RESULT_SUCCESS: u8 = COMBAT_ROUND_RESULT_DEFEAT + 1;
pub const COMBAT_TARGET_GROUP_NEUTRAL: u8 = 0;
pub const COMBAT_TARGET_GROUP_PARTY: u8 = COMBAT_TARGET_GROUP_NEUTRAL + 1;
pub const COMBAT_TARGET_GROUP_MONSTER: u8 = COMBAT_TARGET_GROUP_PARTY + 1;
pub const COMBAT_HIDDEN_ACTIVE_OBJECT_TILE: u8 = 0x00;
pub const COMBAT_ARENA_CENTER_COORDINATE: u8 = (COMBAT_ARENA_SIDE / 2) as u8;

pub const COMBAT_AI_ATTACK_COMMAND_KEY: char = 'A';

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatStepVector {
    pub dx: i8,
    pub dy: i8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatStepDestination {
    pub dx: i8,
    pub dy: i8,
    pub x: i16,
    pub y: i16,
    pub in_bounds: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatAiMovementOutcome {
    Teleport { x: u8, y: u8 },
    Step { direction_code: u8, x: u8, y: u8 },
    Blocked { surrounded: bool },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatLinkedPositionCommitOutcome {
    pub active_object_slot: usize,
    pub actor_position_before: (u8, u8),
    pub actor_position_after: (u8, u8),
    pub active_object_position_before: Option<(usize, usize)>,
    pub active_object_position_after: Option<(usize, usize)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatStepOrAttackOutcome {
    OutOfArena { x: i16, y: i16 },
    Move { x: u8, y: u8 },
    Attack { target_slot: usize },
    BlockedActor { target_slot: usize },
    BlockedWall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatStepOrAttackPrimitiveOutcome {
    InactiveActor,
    OutOfArena {
        x: i16,
        y: i16,
    },
    Moved {
        commit: CombatLinkedPositionCommitOutcome,
    },
    Attack {
        target_slot: usize,
    },
    BlockedActor {
        target_slot: usize,
    },
    BlockedWall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatWeaponDamageRoute {
    NoOrdinaryDamage,
    Damage { raw_damage: i16 },
    Special,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatWeaponAttackRangeRoute {
    Melee,
    Ranged { effect_code: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatWeaponAttackResolution {
    OutOfRange {
        target_range: u8,
        range_cap: u8,
    },
    NoOrdinaryDamage {
        route: CombatWeaponAttackRangeRoute,
    },
    Miss {
        route: CombatWeaponAttackRangeRoute,
        hit_score: i16,
    },
    Hit {
        route: CombatWeaponAttackRangeRoute,
        raw_damage: i16,
    },
    Special {
        route: CombatWeaponAttackRangeRoute,
    },
}

impl CombatStepOrAttackPrimitiveOutcome {
    pub const fn committed_movement(self) -> bool {
        matches!(self, Self::Moved { .. })
    }

    pub const fn blocked(self) -> bool {
        matches!(
            self,
            Self::InactiveActor | Self::BlockedActor { .. } | Self::BlockedWall
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatTargetCandidateView {
    pub descriptor: CombatActorDescriptor,
    pub group: u8,
    pub suppressed: bool,
    pub invisible_or_unrevealed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatTargetPick {
    pub slot: Option<usize>,
    pub first_five_party_slot_survived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatAiTargetResolution {
    ChosenActor {
        slot: usize,
        x: u8,
        y: u8,
    },
    CleanupFallback {
        x: u8,
        y: u8,
    },
    CenterFallback {
        x: u8,
        y: u8,
        critical_hp_flee_slots: Vec<usize>,
    },
    NoUsableTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRoundLoopExit {
    Defeat,
    LeaveCombat,
}

impl CombatRoundLoopExit {
    pub const fn result_code(self) -> u8 {
        match self {
            Self::Defeat => COMBAT_ROUND_RESULT_DEFEAT,
            Self::LeaveCombat => COMBAT_ROUND_RESULT_SUCCESS,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRoundLoopControl {
    ContinueActorWalk,
    StartNextRound,
    Exit(CombatRoundLoopExit),
}

impl CombatRoundLoopControl {
    pub const fn result_code(self) -> Option<u8> {
        match self {
            Self::ContinueActorWalk | Self::StartNextRound => None,
            Self::Exit(exit) => Some(exit.result_code()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatRoundCounterTick {
    pub counter: u8,
    pub wrapped: bool,
    pub redraw_tiles: bool,
    pub advance_time_minutes: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatActorPhaseTick {
    Inactive,
    Waiting {
        counter_before: u8,
        counter_after: u8,
    },
    Ready {
        counter_before: u8,
        refreshed_counter: u8,
    },
}

impl CombatActorPhaseTick {
    pub const fn actor_should_dispatch(self) -> bool {
        matches!(self, Self::Ready { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatPassCommandOutcome {
    pub moves: bool,
    pub attacks: bool,
    pub ends_turn: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatMagicRingPassOutcome {
    pub invisibility_applied: bool,
    pub regeneration_applied: u16,
    pub vanished_ring: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatActivePlayerSelectionOutcome {
    Clear,
    SelectPartySlot(usize),
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatSceneAbortVerb {
    Board,
    Enter,
    Fire,
    HoleUp,
    IgniteTorch,
    Look,
    Mix,
    NewOrder,
    Talk,
    UseItem,
    View,
}

pub const fn combat_scene_abort_verb_prefix(verb: CombatSceneAbortVerb) -> &'static str {
    match verb {
        CombatSceneAbortVerb::Board => "Board",
        CombatSceneAbortVerb::Enter => "Enter",
        CombatSceneAbortVerb::Fire => "Fire",
        CombatSceneAbortVerb::HoleUp => "Hole up",
        CombatSceneAbortVerb::IgniteTorch => "Ignite",
        CombatSceneAbortVerb::Look => "Look",
        CombatSceneAbortVerb::Mix => "Mix",
        CombatSceneAbortVerb::NewOrder => "New order",
        CombatSceneAbortVerb::Talk => "Talk",
        CombatSceneAbortVerb::UseItem => "Use",
        CombatSceneAbortVerb::View => "View",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatCommandBranch {
    Attack,
    CastSpell,
    SceneMessageAbort(CombatSceneAbortVerb),
    DWhatRefusal,
    Get,
    Jimmy,
    Klimb,
    Open,
    Push,
    QuitDefeat,
    Ready,
    Search,
    WWhatRefusal,
    XitCleanup,
    Yell,
    ZStats,
    Pass,
    AbortPrompt,
    ToggleMusic,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatCommandLiveActorGate {
    NotRequired,
    Accepted,
    RejectedDeadOrMissing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatYellCommandOutcome {
    PromptForInput,
    NothingSaid,
    NoEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatCastInterferenceOutcome {
    ContinueToSpellDispatcher,
    Interfered,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatOutOfArenaLeavePresentation {
    EscapeWithFoes,
    OrdinaryCleanup,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatOutOfArenaLeaveOutcome {
    InArena,
    NotCardinalMove,
    RefusedShipStyle,
    RefusedConstrainedDirection {
        required_direction_code: u8,
        attempted_direction_code: u8,
    },
    Accepted {
        direction_code: u8,
        presentation: CombatOutOfArenaLeavePresentation,
        established_direction_code: Option<u8>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveEffectAgeOutcome {
    pub tag: Option<u8>,
    pub counter: u8,
    pub expired: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatWoundScoreBucket {
    UnderOneQuarter,
    OneQuarterToUnderHalf,
    HalfToUnderThreeQuarters,
    ThreeQuartersOrMore,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatWoundMorale {
    pub bucket: CombatWoundScoreBucket,
    pub fleeing: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatMonsterDeathPath {
    DefaultDropCheck,
    SpecialTileTransition,
    Vanish,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatMonsterDamageOutcome {
    pub class: u8,
    pub raw_damage: i16,
    pub applied_damage: u8,
    pub missed: bool,
    pub instant_kill: bool,
    pub killed: bool,
    pub return_value: u8,
    pub death_path: Option<CombatMonsterDeathPath>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatPartyDamageOutcome {
    pub raw_damage: i16,
    pub applied_damage: u16,
    pub missed: bool,
    pub instant_kill: bool,
    pub killed: bool,
    pub status_before: u8,
    pub status_after: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPartySleepOutcome {
    SkippedDeadParty,
    SleptPartyMember { status_before: u8, status_after: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPartyPoisonOutcome {
    PoisonedPartyMember { status_before: u8, status_after: u8 },
    FallbackDamage { raw_damage: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatDefaultDeathMarker {
    NoDrop,
    Drop { loot_byte: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatSplitPlacement {
    pub slot: usize,
    pub class: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatAiAttackRoute {
    OutOfRange,
    Melee,
    RangedEffect {
        range_effect_selector: u8,
        payload: u8,
        scene_resistance: bool,
        cast_like_branch: bool,
        pre_gate_bypass: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatAiSpecialHook {
    Possess,
    Blink,
    SummonDaemon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatPossessCandidateView {
    pub descriptor: CombatActorDescriptor,
    pub member: Option<PartyMember>,
    pub suppressed: bool,
    pub invisible_or_unrevealed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPossessResistanceOutcome {
    Blocked,
    Landed {
        cleared_active_player: bool,
        daemon_clears_self: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatLinkedVisibility {
    Hidden,
    Visible,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatLinkedVisibilityOutcome {
    pub visibility: CombatLinkedVisibility,
    pub actor_flags_before: u8,
    pub actor_flags_after: u8,
    pub visual_tile_before: Option<u8>,
    pub visual_tile_after: Option<u8>,
}

impl CombatLinkedVisibilityOutcome {
    pub fn changed(self) -> bool {
        self.actor_flags_before != self.actor_flags_after
            || self.visual_tile_before != self.visual_tile_after
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPoisonStatusAttackOutcome {
    NotPoisonStatusClass,
    GateRejected,
    PoisonedPartyMember { status_before: u8, status_after: u8 },
    FallbackDamage { raw_damage: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatSpellDamageKind {
    MagicMissile,
    Fireball,
    Kill,
    Tremor,
    DeathWind,
    FlameWind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatDirectedSpellEffect {
    Sleep,
    PoisonWind,
    DeathWind,
    FlameWind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatCreaturePromptSpellEffect {
    Charm,
    Polymorph,
    Clone,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatSpellHandlerFamily {
    ActiveTargetAttack(CombatSpellDamageKind),
    FieldPlacement(CombatArenaFieldKind),
    FieldRemoval,
    DirectedWindCone(CombatDirectedSpellEffect),
    TableWideTremor,
    ActiveEffect { tag: u8, duration: u8 },
    CreaturePromptTargeter(CombatCreaturePromptSpellEffect),
    ActiveCasterInvisibility,
    TableWideFear,
    ConjureAnimal,
    Swarm,
    SummonDaemon,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatCloneAllocation {
    pub actor_slot: usize,
    pub active_object_slot: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatArenaFieldKind {
    Poison,
    Sleep,
    Fire,
    Energy,
}

impl CombatArenaFieldKind {
    pub const fn from_kind_byte(kind: u8) -> Option<Self> {
        match kind {
            COMBAT_FIELD_KIND_POISON => Some(Self::Poison),
            COMBAT_FIELD_KIND_SLEEP => Some(Self::Sleep),
            COMBAT_FIELD_KIND_FIRE => Some(Self::Fire),
            COMBAT_FIELD_KIND_ENERGY => Some(Self::Energy),
            _ => None,
        }
    }

    pub const fn kind_byte(self) -> u8 {
        match self {
            Self::Poison => COMBAT_FIELD_KIND_POISON,
            Self::Sleep => COMBAT_FIELD_KIND_SLEEP,
            Self::Fire => COMBAT_FIELD_KIND_FIRE,
            Self::Energy => COMBAT_FIELD_KIND_ENERGY,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Poison => "Poison",
            Self::Sleep => "Sleep",
            Self::Fire => "Fire",
            Self::Energy => "Energy",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatArenaFieldContactOutcome {
    SkippedCurrentActor,
    PoisonSkippedByLinkedTileClass,
    PoisonedPartyMember { status_before: u8, status_after: u8 },
    PoisonFallbackDamage { raw_damage: u8 },
    SleepSkippedDeadParty,
    SleptPartyMember { status_before: u8, status_after: u8 },
    SleepDisabledNonParty,
    FireDamage { raw_damage: u8 },
    EnergyDamage { raw_damage: u8 },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatActorDescriptor {
    pub hp_or_wound: u8,
    pub base_step: u8,
    pub flags: u8,
    pub owner_target_class: u8,
    pub active_object_slot: u8,
    pub phase_counter: u8,
    pub x: u8,
    pub y: u8,
}

impl CombatActorDescriptor {
    pub const fn from_row(row: [u8; COMBAT_ACTOR_RECORD_LEN]) -> Self {
        Self {
            hp_or_wound: row[0],
            base_step: row[1],
            flags: row[2],
            owner_target_class: row[3],
            active_object_slot: row[4],
            phase_counter: row[5],
            x: row[6],
            y: row[7],
        }
    }

    pub const fn raw_row(self) -> [u8; COMBAT_ACTOR_RECORD_LEN] {
        [
            self.hp_or_wound,
            self.base_step,
            self.flags,
            self.owner_target_class,
            self.active_object_slot,
            self.phase_counter,
            self.x,
            self.y,
        ]
    }

    pub const fn empty() -> Self {
        Self::from_row([0; COMBAT_ACTOR_RECORD_LEN])
    }

    pub const fn for_monster_placement(
        stats: CombatClassStats,
        active_object_slot: u8,
        x: u8,
        y: u8,
        flags: u8,
        phase_counter: u8,
    ) -> Self {
        Self {
            hp_or_wound: stats.max_hp,
            base_step: stats.speed_seed,
            flags,
            owner_target_class: stats.class,
            active_object_slot,
            phase_counter,
            x,
            y,
        }
    }

    pub const fn is_empty(self) -> bool {
        self.raw_row()[0] == 0
            && self.raw_row()[1] == 0
            && self.raw_row()[2] == 0
            && self.raw_row()[3] == 0
            && self.raw_row()[4] == 0
            && self.raw_row()[5] == 0
            && self.raw_row()[6] == 0
            && self.raw_row()[7] == 0
    }

    pub const fn is_marked_dead(self) -> bool {
        self.flags & COMBAT_ACTOR_FLAG_MARKED_DEAD != 0
    }

    pub const fn is_hidden_or_unrevealed(self) -> bool {
        self.flags & COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED != 0
    }

    pub const fn is_status_disabled(self) -> bool {
        self.flags & COMBAT_ACTOR_FLAG_STATUS_DISABLED != 0
    }

    pub const fn is_fleeing(self) -> bool {
        self.flags & COMBAT_ACTOR_FLAG_FLEEING != 0
    }

    pub const fn has_field_lookup_selectable_bit(self) -> bool {
        self.flags & (COMBAT_ACTOR_FLAG_SELECTABLE_80 | COMBAT_ACTOR_FLAG_SELECTABLE_40) != 0
    }

    pub const fn team_toggled(self) -> bool {
        self.flags & COMBAT_ACTOR_FLAG_TEAM_TOGGLE != 0
    }

    pub const fn is_controlled(self) -> bool {
        self.flags & COMBAT_ACTOR_FLAG_CONTROLLED != 0
    }

    pub const fn eligible_for_field_coordinate_lookup(self, linked_active_object_tile: u8) -> bool {
        self.has_field_lookup_selectable_bit()
            && !self.is_marked_dead()
            && !self.is_hidden_or_unrevealed()
            && linked_active_object_tile != COMBAT_FIELD_REJECTED_ACTIVE_OBJECT_TILE
    }

    pub fn clear(&mut self) {
        *self = Self::empty();
    }

    pub fn mark_dead(&mut self) {
        self.flags |= COMBAT_ACTOR_FLAG_MARKED_DEAD;
    }

    pub fn set_status_disabled(&mut self) {
        self.flags |= COMBAT_ACTOR_FLAG_STATUS_DISABLED;
    }

    pub fn clear_status_disabled(&mut self) {
        self.flags &= !COMBAT_ACTOR_FLAG_STATUS_DISABLED;
    }

    pub fn set_fleeing(&mut self, fleeing: bool) {
        if fleeing {
            self.flags |= COMBAT_ACTOR_FLAG_FLEEING;
        } else {
            self.flags &= !COMBAT_ACTOR_FLAG_FLEEING;
        }
    }

    pub fn range_to(self, other: Self) -> u8 {
        combat_arena_range(self.x, self.y, other.x, other.y)
    }

    pub fn apply_monster_damage(
        &mut self,
        raw_damage: i16,
        magical: bool,
    ) -> Option<CombatMonsterDamageOutcome> {
        let stats = combat_class_stats(self.owner_target_class)?;
        let traits = combat_class_traits(self.owner_target_class)?;
        let missed = raw_damage < 0;
        let instant_kill = raw_damage == COMBAT_INSTANT_KILL_DAMAGE;
        let damage = if missed {
            0
        } else if instant_kill {
            self.hp_or_wound
        } else {
            let clamped = raw_damage.clamp(0, u8::MAX as i16) as u8;
            resolve_physical_damage_for_class(self.owner_target_class, clamped, magical)
        };
        let applied_damage = self.hp_or_wound.min(damage);
        self.hp_or_wound -= applied_damage;

        let killed = instant_kill || self.hp_or_wound == 0;
        let death_path = if killed {
            self.hp_or_wound = 0;
            self.mark_dead();
            Some(if traits.vanish_branch {
                CombatMonsterDeathPath::Vanish
            } else if traits.special_death {
                CombatMonsterDeathPath::SpecialTileTransition
            } else {
                CombatMonsterDeathPath::DefaultDropCheck
            })
        } else {
            None
        };
        let return_value = if killed {
            stats.reward_unit()
        } else {
            applied_damage
        };

        Some(CombatMonsterDamageOutcome {
            class: stats.class,
            raw_damage,
            applied_damage,
            missed,
            instant_kill,
            killed,
            return_value,
            death_path,
        })
    }
}

pub fn apply_combat_party_damage(
    member: &mut PartyMember,
    raw_damage: i16,
) -> CombatPartyDamageOutcome {
    let status_before = member.status;
    let missed = raw_damage < 0;
    let instant_kill = raw_damage == COMBAT_INSTANT_KILL_DAMAGE;
    let applied_damage = if missed {
        0
    } else if instant_kill {
        let applied = member.hp;
        member.hp = 0;
        member.status = b'D';
        applied
    } else {
        member.apply_damage(raw_damage.clamp(0, u8::MAX as i16) as u8)
    };

    CombatPartyDamageOutcome {
        raw_damage,
        applied_damage,
        missed,
        instant_kill,
        killed: member.hp == 0,
        status_before,
        status_after: member.status,
    }
}

pub fn apply_combat_sleep_to_party_target(member: &mut PartyMember) -> CombatPartySleepOutcome {
    if member.status == b'D' || member.hp == 0 {
        return CombatPartySleepOutcome::SkippedDeadParty;
    }

    let status_before = member.status;
    member.status = b'S';
    CombatPartySleepOutcome::SleptPartyMember {
        status_before,
        status_after: member.status,
    }
}

pub fn apply_combat_poison_to_party_target(
    member: &mut PartyMember,
    poison_damage_roll: u8,
) -> CombatPartyPoisonOutcome {
    if member.status == b'G' && member.hp > 0 {
        let status_before = member.status;
        member.status = b'P';
        CombatPartyPoisonOutcome::PoisonedPartyMember {
            status_before,
            status_after: member.status,
        }
    } else {
        CombatPartyPoisonOutcome::FallbackDamage {
            raw_damage: combat_field_poison_fallback_damage(poison_damage_roll),
        }
    }
}

pub const fn combat_direction_code_step(direction_code: u8) -> CombatStepVector {
    match direction_code {
        1 => CombatStepVector { dx: -1, dy: 0 },
        2 => CombatStepVector { dx: 1, dy: 0 },
        3 => CombatStepVector { dx: 0, dy: -1 },
        4 => CombatStepVector { dx: 0, dy: 1 },
        _ => CombatStepVector { dx: 0, dy: 0 },
    }
}

pub const fn combat_direction_code_is_cardinal(direction_code: u8) -> bool {
    matches!(direction_code, 1..=4)
}

pub const fn combat_direction_code_for_direction(direction: Direction) -> Option<u8> {
    match direction {
        Direction::West => Some(1),
        Direction::East => Some(2),
        Direction::North => Some(3),
        Direction::South => Some(4),
        Direction::NorthWest
        | Direction::NorthEast
        | Direction::SouthWest
        | Direction::SouthEast => None,
    }
}

pub const fn combat_direction_code_for_step(dx: i8, dy: i8) -> Option<u8> {
    match (dx, dy) {
        (-1, 0) => Some(1),
        (1, 0) => Some(2),
        (0, -1) => Some(3),
        (0, 1) => Some(4),
        _ => None,
    }
}

pub const fn combat_direction_code_ai_command_key(direction_code: u8) -> Option<char> {
    match direction_code {
        1 => Some('W'),
        2 => Some('E'),
        3 => Some('N'),
        4 => Some('S'),
        _ => None,
    }
}

pub const fn resolve_combat_ai_synthesized_command_key(
    target_range: Option<u8>,
    movement_direction_code: Option<u8>,
) -> Option<char> {
    if matches!(target_range, Some(1)) {
        Some(COMBAT_AI_ATTACK_COMMAND_KEY)
    } else {
        match movement_direction_code {
            Some(direction_code) => combat_direction_code_ai_command_key(direction_code),
            None => None,
        }
    }
}

pub const fn combat_arena_coordinate_in_bounds(x: i16, y: i16) -> bool {
    x >= 0 && y >= 0 && x < COMBAT_ARENA_SIDE as i16 && y < COMBAT_ARENA_SIDE as i16
}

pub const fn resolve_combat_step_destination(
    x: u8,
    y: u8,
    direction_code: u8,
) -> CombatStepDestination {
    let step = combat_direction_code_step(direction_code);
    let x = x as i16 + step.dx as i16;
    let y = y as i16 + step.dy as i16;
    CombatStepDestination {
        dx: step.dx,
        dy: step.dy,
        x,
        y,
        in_bounds: combat_arena_coordinate_in_bounds(x, y),
    }
}

pub const fn combat_target_groups_are_hostile(attacker_group: u8, target_group: u8) -> bool {
    attacker_group != COMBAT_TARGET_GROUP_NEUTRAL
        && target_group != COMBAT_TARGET_GROUP_NEUTRAL
        && attacker_group != target_group
}

pub const fn combat_step_or_attack_occupant_is_active(view: CombatTargetCandidateView) -> bool {
    combat_actor_is_active_not_dead(view.descriptor)
        && !view.suppressed
        && !view.invisible_or_unrevealed
}

pub fn resolve_combat_step_or_attack_inner_pass(
    candidates: &[CombatTargetCandidateView],
    moving_slot: usize,
    attacker_group: u8,
    destination: CombatStepDestination,
    destination_walkable: bool,
) -> CombatStepOrAttackOutcome {
    if !destination.in_bounds {
        return CombatStepOrAttackOutcome::OutOfArena {
            x: destination.x,
            y: destination.y,
        };
    }

    let x = destination.x as u8;
    let y = destination.y as u8;
    for (slot, candidate) in candidates
        .iter()
        .copied()
        .enumerate()
        .take(COMBAT_ACTOR_SLOTS)
    {
        if slot == moving_slot
            || candidate.descriptor.x != x
            || candidate.descriptor.y != y
            || !combat_step_or_attack_occupant_is_active(candidate)
        {
            continue;
        }
        return if combat_target_groups_are_hostile(attacker_group, candidate.group) {
            CombatStepOrAttackOutcome::Attack { target_slot: slot }
        } else {
            CombatStepOrAttackOutcome::BlockedActor { target_slot: slot }
        };
    }

    if destination_walkable {
        CombatStepOrAttackOutcome::Move { x, y }
    } else {
        CombatStepOrAttackOutcome::BlockedWall
    }
}

pub fn resolve_combat_step_or_attack_primitive(
    actor: &mut CombatActorDescriptor,
    active_objects: &mut [ActiveObject],
    candidates: &[CombatTargetCandidateView],
    moving_slot: usize,
    attacker_group: u8,
    direction_code: u8,
    destination_walkable: bool,
) -> CombatStepOrAttackPrimitiveOutcome {
    if !combat_actor_is_active_not_dead(*actor) {
        return CombatStepOrAttackPrimitiveOutcome::InactiveActor;
    }

    let destination = resolve_combat_step_destination(actor.x, actor.y, direction_code);
    match resolve_combat_step_or_attack_inner_pass(
        candidates,
        moving_slot,
        attacker_group,
        destination,
        destination_walkable,
    ) {
        CombatStepOrAttackOutcome::OutOfArena { x, y } => {
            CombatStepOrAttackPrimitiveOutcome::OutOfArena { x, y }
        }
        CombatStepOrAttackOutcome::Move { x, y } => {
            match commit_combat_actor_linked_position(actor, active_objects, x, y) {
                Some(commit) => CombatStepOrAttackPrimitiveOutcome::Moved { commit },
                None => CombatStepOrAttackPrimitiveOutcome::InactiveActor,
            }
        }
        CombatStepOrAttackOutcome::Attack { target_slot } => {
            CombatStepOrAttackPrimitiveOutcome::Attack { target_slot }
        }
        CombatStepOrAttackOutcome::BlockedActor { target_slot } => {
            CombatStepOrAttackPrimitiveOutcome::BlockedActor { target_slot }
        }
        CombatStepOrAttackOutcome::BlockedWall => CombatStepOrAttackPrimitiveOutcome::BlockedWall,
    }
}

pub const fn resolve_combat_out_of_arena_leave(
    destination_in_bounds: bool,
    direction_code: u8,
    ship_style_combat: bool,
    constrained_exit: bool,
    established_exit_direction_code: Option<u8>,
    live_foes_remain: bool,
) -> CombatOutOfArenaLeaveOutcome {
    if destination_in_bounds {
        return CombatOutOfArenaLeaveOutcome::InArena;
    }
    if !combat_direction_code_is_cardinal(direction_code) {
        return CombatOutOfArenaLeaveOutcome::NotCardinalMove;
    }
    if ship_style_combat {
        return CombatOutOfArenaLeaveOutcome::RefusedShipStyle;
    }
    if constrained_exit {
        if let Some(required_direction_code) = established_exit_direction_code {
            if required_direction_code != direction_code {
                return CombatOutOfArenaLeaveOutcome::RefusedConstrainedDirection {
                    required_direction_code,
                    attempted_direction_code: direction_code,
                };
            }
        }
    }

    let presentation = if live_foes_remain {
        CombatOutOfArenaLeavePresentation::EscapeWithFoes
    } else {
        CombatOutOfArenaLeavePresentation::OrdinaryCleanup
    };
    let established_direction_code = if constrained_exit {
        Some(match established_exit_direction_code {
            Some(required_direction_code) => required_direction_code,
            None => direction_code,
        })
    } else {
        None
    };

    CombatOutOfArenaLeaveOutcome::Accepted {
        direction_code,
        presentation,
        established_direction_code,
    }
}

pub fn resolve_post_combat_active_party_slot(
    pre_combat_active_slot: Option<usize>,
    party: &[PartyMember],
) -> Option<usize> {
    let slot = pre_combat_active_slot?;
    party
        .get(slot)
        .copied()
        .is_some_and(PartyMember::conscious)
        .then_some(slot)
}

pub const fn combat_spell_damage_roll(roll: u8, max_damage: u8) -> i16 {
    if max_damage == 0 {
        0
    } else {
        1 + (roll % max_damage) as i16
    }
}

pub const fn resolve_combat_weapon_raw_damage(attack_max: u8, roll: u8) -> CombatWeaponDamageRoute {
    match attack_max {
        0 => CombatWeaponDamageRoute::NoOrdinaryDamage,
        1 => CombatWeaponDamageRoute::Damage { raw_damage: 1 },
        99 => CombatWeaponDamageRoute::Special,
        max_damage => CombatWeaponDamageRoute::Damage {
            raw_damage: combat_spell_damage_roll(roll, max_damage),
        },
    }
}

pub fn resolve_combat_equipment_weapon_raw_damage(
    item_id: usize,
    roll: u8,
) -> Option<CombatWeaponDamageRoute> {
    Some(resolve_combat_weapon_raw_damage(
        equipment_attack_max(item_id)?,
        roll,
    ))
}

pub const fn resolve_combat_weapon_attack_range_route(
    target_range: u8,
    range_cap: u8,
    effect_code: u8,
) -> Option<CombatWeaponAttackRangeRoute> {
    if target_range <= 1 {
        Some(CombatWeaponAttackRangeRoute::Melee)
    } else if range_cap != 0 && target_range <= range_cap {
        Some(CombatWeaponAttackRangeRoute::Ranged { effect_code })
    } else {
        None
    }
}

pub const fn resolve_combat_weapon_attack(
    attack_max: u8,
    target_range: u8,
    range_cap: u8,
    effect_code: u8,
    attacker_rating: u8,
    defender_rating: u8,
    hit_roll: u8,
    damage_roll: u8,
    forced_hit: Option<bool>,
) -> CombatWeaponAttackResolution {
    let Some(route) =
        resolve_combat_weapon_attack_range_route(target_range, range_cap, effect_code)
    else {
        return CombatWeaponAttackResolution::OutOfRange {
            target_range,
            range_cap,
        };
    };

    match resolve_combat_weapon_raw_damage(attack_max, damage_roll) {
        CombatWeaponDamageRoute::NoOrdinaryDamage => {
            CombatWeaponAttackResolution::NoOrdinaryDamage { route }
        }
        CombatWeaponDamageRoute::Special => CombatWeaponAttackResolution::Special { route },
        CombatWeaponDamageRoute::Damage { raw_damage } => {
            let hit_score = combat_to_hit_score(attacker_rating, defender_rating);
            let hit = match forced_hit {
                Some(hit) => hit,
                None => hit_score > hit_roll as i16,
            };
            if hit {
                CombatWeaponAttackResolution::Hit { route, raw_damage }
            } else {
                CombatWeaponAttackResolution::Miss { route, hit_score }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn resolve_combat_equipment_weapon_attack(
    item_id: usize,
    target_range: u8,
    attacker_rating: u8,
    defender_rating: u8,
    hit_roll: u8,
    damage_roll: u8,
    forced_hit: Option<bool>,
) -> Option<CombatWeaponAttackResolution> {
    Some(resolve_combat_weapon_attack(
        equipment_attack_max(item_id)?,
        target_range,
        equipment_weapon_range_cap(item_id)?,
        equipment_weapon_effect_code(item_id)?,
        attacker_rating,
        defender_rating,
        hit_roll,
        damage_roll,
        forced_hit,
    ))
}

pub const fn resolve_combat_spell_raw_damage(kind: CombatSpellDamageKind, roll: u8) -> i16 {
    match kind {
        CombatSpellDamageKind::MagicMissile => {
            combat_spell_damage_roll(roll, COMBAT_MAGIC_MISSILE_DAMAGE_ROLL_MAX)
        }
        CombatSpellDamageKind::Fireball => {
            combat_spell_damage_roll(roll, COMBAT_FIREBALL_DAMAGE_ROLL_MAX)
        }
        CombatSpellDamageKind::Kill => COMBAT_INSTANT_KILL_DAMAGE,
        CombatSpellDamageKind::Tremor => {
            combat_spell_damage_roll(roll, COMBAT_TREMOR_DAMAGE_ROLL_MAX)
        }
        CombatSpellDamageKind::DeathWind => COMBAT_INSTANT_KILL_DAMAGE,
        CombatSpellDamageKind::FlameWind => {
            combat_spell_damage_roll(roll, COMBAT_FLAME_WIND_DAMAGE_ROLL_MAX)
        }
    }
}

pub const fn resolve_spell_damage_after_defense(raw_damage: i16, defense_roll: u8) -> i16 {
    if raw_damage == COMBAT_INSTANT_KILL_DAMAGE {
        COMBAT_INSTANT_KILL_DAMAGE
    } else {
        raw_damage - defense_roll as i16
    }
}

pub const fn resolve_active_target_spell_damage(
    kind: CombatSpellDamageKind,
    damage_roll: u8,
    defense_roll: u8,
) -> Option<i16> {
    match kind {
        CombatSpellDamageKind::MagicMissile | CombatSpellDamageKind::Fireball => {
            Some(resolve_spell_damage_after_defense(
                resolve_combat_spell_raw_damage(kind, damage_roll),
                defense_roll,
            ))
        }
        CombatSpellDamageKind::Kill => Some(COMBAT_INSTANT_KILL_DAMAGE),
        CombatSpellDamageKind::Tremor
        | CombatSpellDamageKind::DeathWind
        | CombatSpellDamageKind::FlameWind => None,
    }
}

pub const fn directed_spell_actor_is_eligible(actor: CombatActorDescriptor) -> bool {
    !actor.is_empty()
        && !actor.is_marked_dead()
        && !actor.is_hidden_or_unrevealed()
        && !actor.is_status_disabled()
}

pub const fn combat_actor_is_present_not_dead(actor: CombatActorDescriptor) -> bool {
    !actor.is_empty() && !actor.is_marked_dead() && actor.has_field_lookup_selectable_bit()
}

pub const fn combat_actor_is_active_not_dead(actor: CombatActorDescriptor) -> bool {
    !actor.is_empty()
        && !actor.is_marked_dead()
        && !actor.is_status_disabled()
        && actor.has_field_lookup_selectable_bit()
}

pub const fn combat_actor_is_revealable(actor: CombatActorDescriptor) -> bool {
    !actor.is_empty() && !actor.is_marked_dead() && actor.is_hidden_or_unrevealed()
}

pub fn apply_combat_invisibility(actor: &mut CombatActorDescriptor) -> bool {
    if actor.is_empty() || actor.is_marked_dead() {
        return false;
    }

    let flags_before = actor.flags;
    actor.flags |= COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
    actor.flags != flags_before
}

pub fn clear_combat_invisibility(actor: &mut CombatActorDescriptor) -> bool {
    let flags_before = actor.flags;
    actor.flags &= !COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
    actor.flags != flags_before
}

pub fn set_combat_linked_visibility(
    actor: &mut CombatActorDescriptor,
    active_objects: &mut [ActiveObject],
    hidden: bool,
) -> Option<CombatLinkedVisibilityOutcome> {
    if actor.is_empty() || actor.is_marked_dead() {
        return None;
    }

    let actor_flags_before = actor.flags;
    if hidden {
        actor.flags |= COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
    } else {
        actor.flags &= !COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
    }

    let mut visual_tile_before = None;
    let mut visual_tile_after = None;
    if let Some(object) = active_objects.get_mut(actor.active_object_slot as usize) {
        visual_tile_before = Some(object.tile);
        object.tile = if hidden {
            COMBAT_HIDDEN_ACTIVE_OBJECT_TILE
        } else {
            object.type_byte
        };
        visual_tile_after = Some(object.tile);
    }

    Some(CombatLinkedVisibilityOutcome {
        visibility: if hidden {
            CombatLinkedVisibility::Hidden
        } else {
            CombatLinkedVisibility::Visible
        },
        actor_flags_before,
        actor_flags_after: actor.flags,
        visual_tile_before,
        visual_tile_after,
    })
}

pub fn apply_combat_linked_invisibility(
    actor: &mut CombatActorDescriptor,
    active_objects: &mut [ActiveObject],
) -> Option<CombatLinkedVisibilityOutcome> {
    set_combat_linked_visibility(actor, active_objects, true)
}

pub fn clear_combat_linked_invisibility(
    actor: &mut CombatActorDescriptor,
    active_objects: &mut [ActiveObject],
) -> Option<CombatLinkedVisibilityOutcome> {
    set_combat_linked_visibility(actor, active_objects, false)
}

pub fn toggle_combat_blink_phase(
    actor: &mut CombatActorDescriptor,
    active_objects: &mut [ActiveObject],
) -> Option<CombatLinkedVisibilityOutcome> {
    set_combat_linked_visibility(actor, active_objects, !actor.is_hidden_or_unrevealed())
}

pub fn apply_combat_reveal(actors: &mut [CombatActorDescriptor]) -> usize {
    let mut revealed = 0;
    for actor in actors {
        if combat_actor_is_revealable(*actor) {
            actor.flags &= !COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
            revealed += 1;
        }
    }
    revealed
}

pub const fn tremor_spell_actor_is_damageable(actor: CombatActorDescriptor) -> bool {
    directed_spell_actor_is_eligible(actor)
}

pub fn collect_directed_spell_actor_slots(
    actors: &[CombatActorDescriptor],
    target_cells: &[(u8, u8)],
) -> Vec<usize> {
    let mut slots = Vec::new();
    for (slot, actor) in actors.iter().copied().enumerate() {
        if !directed_spell_actor_is_eligible(actor) {
            continue;
        }
        if target_cells
            .iter()
            .any(|(target_x, target_y)| actor.x == *target_x && actor.y == *target_y)
        {
            slots.push(slot);
        }
    }
    slots
}

/// `magic.md §8` / `catalogs/monster-bestiary.md §6`: the repel-undead
/// player spell targets the published undead/spectral combat classes.
pub const fn combat_class_is_repel_undead_target(class: u8) -> bool {
    matches!(class, 23 | 33)
}

pub fn collect_repel_undead_actor_slots(actors: &[CombatActorDescriptor]) -> Vec<usize> {
    let mut slots = Vec::new();
    for (slot, actor) in actors.iter().copied().enumerate() {
        if directed_spell_actor_is_eligible(actor)
            && combat_class_is_repel_undead_target(actor.owner_target_class)
        {
            slots.push(slot);
        }
    }
    slots
}

pub fn collect_tremor_spell_actor_slots(
    actors: &[CombatActorDescriptor],
    gate_accepts: &[bool],
) -> Vec<usize> {
    let mut slots = Vec::new();
    for (slot, actor) in actors.iter().copied().enumerate() {
        if tremor_spell_actor_is_damageable(actor)
            && gate_accepts.get(slot).copied().unwrap_or(false)
        {
            slots.push(slot);
        }
    }
    slots
}

pub const fn resolve_combat_spell_handler_family(
    spell_index: usize,
) -> Option<CombatSpellHandlerFamily> {
    match spell_index {
        1 => Some(CombatSpellHandlerFamily::ActiveTargetAttack(
            CombatSpellDamageKind::MagicMissile,
        )),
        10 => Some(CombatSpellHandlerFamily::ConjureAnimal),
        13 => Some(CombatSpellHandlerFamily::ActiveTargetAttack(
            CombatSpellDamageKind::Fireball,
        )),
        14 => Some(CombatSpellHandlerFamily::FieldPlacement(
            CombatArenaFieldKind::Fire,
        )),
        15 => Some(CombatSpellHandlerFamily::FieldPlacement(
            CombatArenaFieldKind::Poison,
        )),
        16 => Some(CombatSpellHandlerFamily::FieldPlacement(
            CombatArenaFieldKind::Sleep,
        )),
        18 => Some(CombatSpellHandlerFamily::FieldRemoval),
        19 => Some(CombatSpellHandlerFamily::ActiveEffect {
            tag: PROTECTION_ACTIVE_EFFECT_TAG,
            duration: PROTECTION_ACTIVE_EFFECT_DURATION,
        }),
        20 => Some(CombatSpellHandlerFamily::FieldPlacement(
            CombatArenaFieldKind::Energy,
        )),
        24 => Some(CombatSpellHandlerFamily::Swarm),
        28 => Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::Sleep,
        )),
        29 => Some(CombatSpellHandlerFamily::ActiveEffect {
            tag: QUICKNESS_ACTIVE_EFFECT_TAG,
            duration: QUICKNESS_ACTIVE_EFFECT_DURATION,
        }),
        30 => Some(CombatSpellHandlerFamily::TableWideTremor),
        31 => Some(CombatSpellHandlerFamily::ActiveEffect {
            tag: MASS_CHARM_ACTIVE_EFFECT_TAG,
            duration: MASS_CHARM_ACTIVE_EFFECT_DURATION,
        }),
        32 => Some(CombatSpellHandlerFamily::ActiveEffect {
            tag: NEGATE_MAGIC_ACTIVE_EFFECT_TAG,
            duration: NEGATE_MAGIC_ACTIVE_EFFECT_DURATION,
        }),
        34 => Some(CombatSpellHandlerFamily::CreaturePromptTargeter(
            CombatCreaturePromptSpellEffect::Charm,
        )),
        35 => Some(CombatSpellHandlerFamily::CreaturePromptTargeter(
            CombatCreaturePromptSpellEffect::Polymorph,
        )),
        36 => Some(CombatSpellHandlerFamily::ActiveCasterInvisibility),
        37 => Some(CombatSpellHandlerFamily::ActiveTargetAttack(
            CombatSpellDamageKind::Kill,
        )),
        38 => Some(CombatSpellHandlerFamily::CreaturePromptTargeter(
            CombatCreaturePromptSpellEffect::Clone,
        )),
        40 => Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::PoisonWind,
        )),
        41 => Some(CombatSpellHandlerFamily::TableWideFear),
        43 => Some(CombatSpellHandlerFamily::SummonDaemon),
        44 => Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::DeathWind,
        )),
        45 => Some(CombatSpellHandlerFamily::DirectedWindCone(
            CombatDirectedSpellEffect::FlameWind,
        )),
        _ => None,
    }
}

pub const fn cause_fear_actor_is_live(actor: CombatActorDescriptor) -> bool {
    !actor.is_empty() && !actor.is_marked_dead()
}

pub fn collect_cause_fear_actor_slots(
    actors: &[CombatActorDescriptor],
    groups: &[u8],
    caster_group: u8,
    protected_or_immune: &[bool],
) -> Vec<usize> {
    let mut slots = Vec::new();
    for (slot, actor) in actors.iter().copied().enumerate() {
        if !cause_fear_actor_is_live(actor) {
            continue;
        }
        if groups.get(slot).copied() == Some(caster_group) {
            continue;
        }
        if protected_or_immune.get(slot).copied().unwrap_or(false) {
            continue;
        }
        slots.push(slot);
    }
    slots
}

pub fn combat_has_active_not_dead_non_party_actor(actors: &[CombatActorDescriptor]) -> bool {
    actors.iter().copied().enumerate().any(|(slot, actor)| {
        slot >= COMBAT_PARTY_ACTOR_SLOTS && combat_actor_is_present_not_dead(actor)
    })
}

pub fn resolve_combat_victory(actors: &[CombatActorDescriptor]) -> bool {
    !combat_has_active_not_dead_non_party_actor(actors)
}

pub fn resolve_combat_xit_cleanup_allowed(actors: &[CombatActorDescriptor]) -> bool {
    resolve_combat_victory(actors)
}

pub fn combat_party_slot_can_continue(
    slot: usize,
    actors: &[CombatActorDescriptor],
    party: &[PartyMember],
) -> bool {
    if slot >= COMBAT_PARTY_ACTOR_SLOTS {
        return false;
    }
    let Some(actor) = actors.get(slot).copied() else {
        return false;
    };
    if !combat_actor_is_active_not_dead(actor) {
        return false;
    }
    party.get(slot).copied().is_some_and(PartyMember::conscious)
}

pub fn resolve_combat_defeat(party: &[PartyMember], actors: &[CombatActorDescriptor]) -> bool {
    !(0..COMBAT_PARTY_ACTOR_SLOTS).any(|slot| combat_party_slot_can_continue(slot, actors, party))
}

pub const fn resolve_combat_round_loop_control(
    defeat_flag: bool,
    leave_combat_flag: bool,
    exhausted_slots: bool,
) -> CombatRoundLoopControl {
    if defeat_flag {
        CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
    } else if leave_combat_flag {
        CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
    } else if exhausted_slots {
        CombatRoundLoopControl::StartNextRound
    } else {
        CombatRoundLoopControl::ContinueActorWalk
    }
}

pub const fn resolve_combat_round_counter_tick(counter: u8) -> CombatRoundCounterTick {
    let next = counter as u16 + 1;
    let wrapped = next >= COMBAT_ROUND_COUNTER_WRAP as u16;
    CombatRoundCounterTick {
        counter: (next % COMBAT_ROUND_COUNTER_WRAP as u16) as u8,
        wrapped,
        redraw_tiles: wrapped,
        advance_time_minutes: if wrapped {
            COMBAT_ROUND_WRAP_TIME_ADVANCE_MINUTES
        } else {
            0
        },
    }
}

pub const fn resolve_combat_phase_refresh_counter(base_step: u8, refresh_constant: u8) -> u8 {
    refresh_constant.saturating_sub(base_step)
}

pub fn tick_combat_actor_phase_counter(
    actor: &mut CombatActorDescriptor,
    refresh_constant: u8,
) -> CombatActorPhaseTick {
    if !combat_actor_is_present_not_dead(*actor) {
        return CombatActorPhaseTick::Inactive;
    }

    let counter_before = actor.phase_counter;
    if counter_before > 1 {
        actor.phase_counter -= 1;
        return CombatActorPhaseTick::Waiting {
            counter_before,
            counter_after: actor.phase_counter,
        };
    }

    actor.phase_counter = resolve_combat_phase_refresh_counter(actor.base_step, refresh_constant);
    CombatActorPhaseTick::Ready {
        counter_before,
        refreshed_counter: actor.phase_counter,
    }
}

pub const fn resolve_combat_pass_command() -> CombatPassCommandOutcome {
    CombatPassCommandOutcome {
        moves: false,
        attacks: false,
        ends_turn: true,
    }
}

pub fn combat_ring_regeneration_amount(
    wearer: PartyMember,
    ring_item_id: u8,
    regeneration_roll: u8,
) -> u16 {
    if ring_item_id as usize != EQUIPMENT_ID_RING_REGENERATION
        || !wearer.living()
        || regeneration_roll & 0x07 != 0
    {
        0
    } else if wearer.hp < wearer.max_hp {
        1
    } else {
        0
    }
}

pub const fn combat_magic_ring_vanishes(ring_item_id: u8, vanish_roll: u8) -> bool {
    is_combat_magic_ring_id(ring_item_id) && vanish_roll & 0x0f == 0
}

pub const fn is_combat_magic_ring_id(ring_item_id: u8) -> bool {
    matches!(
        ring_item_id as usize,
        EQUIPMENT_ID_RING_INVISIBILITY | EQUIPMENT_ID_RING_REGENERATION
    )
}

pub const fn resolve_combat_quit_command() -> CombatRoundLoopControl {
    CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
}

pub const fn resolve_combat_active_player_digit(key: char) -> CombatActivePlayerSelectionOutcome {
    match key {
        '0' => CombatActivePlayerSelectionOutcome::Clear,
        '1'..='6' => {
            CombatActivePlayerSelectionOutcome::SelectPartySlot((key as u8 - b'1') as usize)
        }
        _ => CombatActivePlayerSelectionOutcome::Invalid,
    }
}

pub fn resolve_post_combat_active_player_restore(
    pre_combat_active_player: Option<usize>,
    party: &[PartyMember],
) -> Option<usize> {
    let slot = pre_combat_active_player?;
    let member = party.get(slot)?;
    if matches!(member.status, b'D' | b'S') {
        None
    } else {
        Some(slot)
    }
}

pub fn resolve_combat_command_branch(key: char) -> CombatCommandBranch {
    match key.to_ascii_uppercase() {
        'A' => CombatCommandBranch::Attack,
        'B' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Board),
        'C' => CombatCommandBranch::CastSpell,
        'D' => CombatCommandBranch::DWhatRefusal,
        'E' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Enter),
        'F' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Fire),
        'G' => CombatCommandBranch::Get,
        'H' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::HoleUp),
        'I' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::IgniteTorch),
        'J' => CombatCommandBranch::Jimmy,
        'K' => CombatCommandBranch::Klimb,
        'L' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Look),
        'M' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Mix),
        'N' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::NewOrder),
        'O' => CombatCommandBranch::Open,
        'P' => CombatCommandBranch::Push,
        'Q' => CombatCommandBranch::QuitDefeat,
        'R' => CombatCommandBranch::Ready,
        'S' => CombatCommandBranch::Search,
        'T' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::Talk),
        'U' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::UseItem),
        'V' => CombatCommandBranch::SceneMessageAbort(CombatSceneAbortVerb::View),
        'W' => CombatCommandBranch::WWhatRefusal,
        'X' => CombatCommandBranch::XitCleanup,
        'Y' => CombatCommandBranch::Yell,
        'Z' => CombatCommandBranch::ZStats,
        ' ' => CombatCommandBranch::Pass,
        '\u{1b}' => CombatCommandBranch::AbortPrompt,
        '\u{13}' => CombatCommandBranch::ToggleMusic,
        _ => CombatCommandBranch::Invalid,
    }
}

pub const fn combat_command_branch_requires_live_active_actor(branch: CombatCommandBranch) -> bool {
    match branch {
        CombatCommandBranch::Get
        | CombatCommandBranch::Jimmy
        | CombatCommandBranch::Open
        | CombatCommandBranch::Ready
        | CombatCommandBranch::Search => true,
        CombatCommandBranch::Attack
        | CombatCommandBranch::CastSpell
        | CombatCommandBranch::SceneMessageAbort(_)
        | CombatCommandBranch::DWhatRefusal
        | CombatCommandBranch::Klimb
        | CombatCommandBranch::Push
        | CombatCommandBranch::QuitDefeat
        | CombatCommandBranch::WWhatRefusal
        | CombatCommandBranch::XitCleanup
        | CombatCommandBranch::Yell
        | CombatCommandBranch::ZStats
        | CombatCommandBranch::Pass
        | CombatCommandBranch::AbortPrompt
        | CombatCommandBranch::ToggleMusic
        | CombatCommandBranch::Invalid => false,
    }
}

pub const fn resolve_combat_command_live_actor_gate(
    branch: CombatCommandBranch,
    active_actor: Option<CombatActorDescriptor>,
) -> CombatCommandLiveActorGate {
    if !combat_command_branch_requires_live_active_actor(branch) {
        return CombatCommandLiveActorGate::NotRequired;
    }

    match active_actor {
        Some(actor) if combat_actor_is_active_not_dead(actor) => {
            CombatCommandLiveActorGate::Accepted
        }
        Some(_) | None => CombatCommandLiveActorGate::RejectedDeadOrMissing,
    }
}

pub const fn combat_command_branch_published_label(
    branch: CombatCommandBranch,
) -> Option<&'static str> {
    match branch {
        CombatCommandBranch::DWhatRefusal => Some("D-What?"),
        CombatCommandBranch::Get => Some("Get-"),
        CombatCommandBranch::Jimmy => Some("Jimmy-"),
        CombatCommandBranch::Open => Some("Open-"),
        CombatCommandBranch::Push => Some("Push-"),
        CombatCommandBranch::Search => Some("Search-"),
        CombatCommandBranch::WWhatRefusal => Some("W-What?"),
        CombatCommandBranch::SceneMessageAbort(_) => None,
        CombatCommandBranch::Attack
        | CombatCommandBranch::CastSpell
        | CombatCommandBranch::Klimb
        | CombatCommandBranch::QuitDefeat
        | CombatCommandBranch::Ready
        | CombatCommandBranch::XitCleanup
        | CombatCommandBranch::Yell
        | CombatCommandBranch::ZStats
        | CombatCommandBranch::Pass
        | CombatCommandBranch::AbortPrompt
        | CombatCommandBranch::ToggleMusic
        | CombatCommandBranch::Invalid => None,
    }
}

pub const fn combat_command_branch_is_named_multistage(branch: CombatCommandBranch) -> bool {
    match branch {
        CombatCommandBranch::Attack
        | CombatCommandBranch::CastSpell
        | CombatCommandBranch::Get
        | CombatCommandBranch::Jimmy
        | CombatCommandBranch::Klimb
        | CombatCommandBranch::Open
        | CombatCommandBranch::Ready
        | CombatCommandBranch::Search
        | CombatCommandBranch::Yell => true,
        CombatCommandBranch::SceneMessageAbort(_)
        | CombatCommandBranch::DWhatRefusal
        | CombatCommandBranch::Push
        | CombatCommandBranch::QuitDefeat
        | CombatCommandBranch::WWhatRefusal
        | CombatCommandBranch::XitCleanup
        | CombatCommandBranch::ZStats
        | CombatCommandBranch::Pass
        | CombatCommandBranch::AbortPrompt
        | CombatCommandBranch::ToggleMusic
        | CombatCommandBranch::Invalid => false,
    }
}

pub fn resolve_combat_yell_command(word: Option<&str>) -> CombatYellCommandOutcome {
    let Some(word) = word else {
        return CombatYellCommandOutcome::PromptForInput;
    };

    if word.trim().chars().take(30).next().is_none() {
        CombatYellCommandOutcome::NothingSaid
    } else {
        CombatYellCommandOutcome::NoEffect
    }
}

pub const fn combat_cast_interference_target_is_live_visible(
    target: CombatActorDescriptor,
) -> bool {
    combat_actor_is_active_not_dead(target) && !target.is_hidden_or_unrevealed()
}

pub fn resolve_combat_cast_interference(
    caster: CombatActorDescriptor,
    target: Option<CombatActorDescriptor>,
    target_awake: bool,
    negate_time_active: bool,
) -> CombatCastInterferenceOutcome {
    let Some(target) = target else {
        return CombatCastInterferenceOutcome::ContinueToSpellDispatcher;
    };

    if negate_time_active
        || !target_awake
        || !combat_cast_interference_target_is_live_visible(target)
        || caster.range_to(target) != 1
    {
        CombatCastInterferenceOutcome::ContinueToSpellDispatcher
    } else {
        CombatCastInterferenceOutcome::Interfered
    }
}

pub fn apply_cause_fear_critical_hp_setup(
    actors: &mut [CombatActorDescriptor],
    slots: &[usize],
) -> usize {
    let mut applied = 0;
    for slot in slots {
        let Some(actor) = actors.get_mut(*slot) else {
            continue;
        };
        if !cause_fear_actor_is_live(*actor) {
            continue;
        }
        let Some(stats) = combat_class_stats(actor.owner_target_class) else {
            continue;
        };
        actor.hp_or_wound = cause_fear_forced_current_hp(stats.max_hp);
        actor.set_fleeing(true);
        applied += 1;
    }
    applied
}

pub const fn directed_spell_damage_credits_caster(effect: CombatDirectedSpellEffect) -> bool {
    matches!(
        effect,
        CombatDirectedSpellEffect::DeathWind | CombatDirectedSpellEffect::FlameWind
    )
}

pub const fn resolve_directed_spell_raw_damage(
    effect: CombatDirectedSpellEffect,
    roll: u8,
) -> Option<i16> {
    match effect {
        CombatDirectedSpellEffect::Sleep | CombatDirectedSpellEffect::PoisonWind => None,
        CombatDirectedSpellEffect::DeathWind => Some(COMBAT_INSTANT_KILL_DAMAGE),
        CombatDirectedSpellEffect::FlameWind => Some(resolve_combat_spell_raw_damage(
            CombatSpellDamageKind::FlameWind,
            roll,
        )),
    }
}

pub const fn resolve_tremor_spell_raw_damage(roll: u8) -> i16 {
    resolve_combat_spell_raw_damage(CombatSpellDamageKind::Tremor, roll)
}

pub const fn apply_combat_experience_reward(current_experience: u16, reward: u8) -> u16 {
    let added = current_experience.saturating_add(reward as u16);
    if added > COMBAT_EXPERIENCE_CAP {
        COMBAT_EXPERIENCE_CAP
    } else {
        added
    }
}

pub const fn apply_combat_spell_experience_reward(current_experience: u16, reward: u8) -> u16 {
    apply_combat_experience_reward(current_experience, reward)
}

pub const fn cause_fear_forced_current_hp(max_hp: u8) -> u8 {
    if max_hp == 0 { 0 } else { (max_hp - 1) / 4 }
}

pub const fn resolve_conjure_spell_class(selector: u8) -> u8 {
    match selector % CONJURE_ANIMAL_OUTCOME_COUNT {
        0..=5 => COMBAT_CLASS_GIANT_RAT,
        6..=10 => COMBAT_CLASS_GIANT_SPIDER,
        11..=13 => COMBAT_CLASS_BAT,
        _ => COMBAT_CLASS_PYTHON,
    }
}

pub const fn creature_prompt_target_is_eligible(
    actor: CombatActorDescriptor,
    target_group: u8,
    caster_group: u8,
    protected_or_immune: bool,
) -> bool {
    !actor.is_empty()
        && !actor.is_marked_dead()
        && !actor.is_hidden_or_unrevealed()
        && !actor.is_status_disabled()
        && !actor.is_controlled()
        && !protected_or_immune
        && target_group != caster_group
}

pub fn toggle_combat_charm_allegiance(actor: &mut CombatActorDescriptor) -> Option<(u8, u8)> {
    if actor.is_empty() || actor.is_marked_dead() {
        return None;
    }
    let flags_before = actor.flags;
    actor.flags ^= COMBAT_ACTOR_FLAG_TEAM_TOGGLE;
    Some((flags_before, actor.flags))
}

pub fn resolve_summoned_combat_actor_descriptor(
    class: u8,
    active_object_slot: u8,
    x: u8,
    y: u8,
    flags: u8,
    phase_counter: u8,
) -> Option<CombatActorDescriptor> {
    let stats = combat_class_stats(class)?;
    Some(CombatActorDescriptor::for_monster_placement(
        stats,
        active_object_slot,
        x,
        y,
        flags,
        phase_counter,
    ))
}

pub fn resolve_conjure_spell_descriptor(
    selector: u8,
    active_object_slot: u8,
    x: u8,
    y: u8,
    flags: u8,
    phase_counter: u8,
) -> Option<CombatActorDescriptor> {
    resolve_summoned_combat_actor_descriptor(
        resolve_conjure_spell_class(selector),
        active_object_slot,
        x,
        y,
        flags,
        phase_counter,
    )
}

pub fn resolve_swarm_spell_descriptor(
    active_object_slot: u8,
    x: u8,
    y: u8,
    flags: u8,
    phase_counter: u8,
) -> Option<CombatActorDescriptor> {
    resolve_summoned_combat_actor_descriptor(
        COMBAT_CLASS_INSECT_SWARM,
        active_object_slot,
        x,
        y,
        flags,
        phase_counter,
    )
}

pub fn resolve_summon_daemon_spell_descriptor(
    active_object_slot: u8,
    x: u8,
    y: u8,
    flags: u8,
    phase_counter: u8,
) -> Option<CombatActorDescriptor> {
    resolve_summoned_combat_actor_descriptor(
        COMBAT_CLASS_DAEMON,
        active_object_slot,
        x,
        y,
        flags,
        phase_counter,
    )
}

pub fn combat_class_sprite_base(class: u8) -> Option<u8> {
    match class {
        12..=15 => Some(0x70 + (class - 12) * 4),
        16..=41 => Some(0x80 + (class - 16) * 4),
        44..=47 => Some(0xf0 + (class - 44) * 4),
        _ => None,
    }
}

pub fn summoned_active_object_record(class: u8, x: usize, y: usize, z: i8) -> Option<ActiveObject> {
    let sprite = combat_class_sprite_base(class)?;
    Some(ActiveObject {
        type_byte: sprite,
        tile: sprite,
        x,
        y,
        z,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    })
}

pub fn resolve_polymorph_giant_rat_descriptor(
    target: CombatActorDescriptor,
) -> Option<CombatActorDescriptor> {
    let stats = combat_class_stats(COMBAT_CLASS_GIANT_RAT)?;
    Some(CombatActorDescriptor::for_monster_placement(
        stats,
        target.active_object_slot,
        target.x,
        target.y,
        target.flags,
        target.phase_counter,
    ))
}

pub fn polymorph_giant_rat_active_object(target: ActiveObject, x: u8, y: u8) -> ActiveObject {
    ActiveObject {
        type_byte: COMBAT_CLASS_GIANT_RAT_SPRITE_BASE,
        tile: COMBAT_CLASS_GIANT_RAT_SPRITE_BASE,
        x: usize::from(x),
        y: usize::from(y),
        ..target
    }
}

pub fn resolve_clone_spell_allocation(
    actors: &[CombatActorDescriptor],
    active_objects: &[ActiveObject],
) -> Option<CombatCloneAllocation> {
    let actor_slot = actors
        .iter()
        .enumerate()
        .skip(COMBAT_PARTY_ACTOR_SLOTS)
        .find_map(|(slot, descriptor)| descriptor.is_empty().then_some(slot))?;
    let active_object_slot = active_objects
        .iter()
        .enumerate()
        .skip(COMBAT_PARTY_ACTOR_SLOTS)
        .find_map(|(slot, object)| object.is_empty().then_some(slot))?;
    Some(CombatCloneAllocation {
        actor_slot,
        active_object_slot,
    })
}

pub const fn clone_combat_actor_descriptor(
    target: CombatActorDescriptor,
    active_object_slot: u8,
    x: u8,
    y: u8,
) -> CombatActorDescriptor {
    CombatActorDescriptor {
        active_object_slot,
        x,
        y,
        ..target
    }
}

pub fn clone_active_object_record(target: ActiveObject, x: usize, y: usize) -> ActiveObject {
    ActiveObject { x, y, ..target }
}

pub fn resolve_combat_clone_placement_coordinate(
    legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    candidate_coordinates: &[(u8, u8)],
) -> Option<(u8, u8)> {
    candidate_coordinates
        .iter()
        .copied()
        .find(|(x, y)| combat_ai_legal_cell(legal_cells, i16::from(*x), i16::from(*y)))
}

pub fn combat_clone_candidate_coordinates(seed: u8) -> Vec<(u8, u8)> {
    let cell_count = COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE;
    let start = usize::from(seed) % cell_count;
    (0..cell_count)
        .map(|offset| {
            let index = (start + offset) % cell_count;
            (
                (index % COMBAT_ARENA_SIDE) as u8,
                (index / COMBAT_ARENA_SIDE) as u8,
            )
        })
        .collect()
}

pub fn combat_neighbor_candidate_coordinates(
    center_x: u8,
    center_y: u8,
    seed: u8,
) -> Vec<(u8, u8)> {
    const OFFSETS: [(i16, i16); 8] = [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ];
    let start = usize::from(seed) % OFFSETS.len();
    (0..OFFSETS.len())
        .filter_map(|offset| {
            let (dx, dy) = OFFSETS[(start + offset) % OFFSETS.len()];
            let x = i16::from(center_x) + dx;
            let y = i16::from(center_y) + dy;
            combat_arena_coordinate_in_bounds(x, y).then_some((x as u8, y as u8))
        })
        .collect()
}

pub fn combat_swarm_jitter_candidate_coordinate(
    center_x: u8,
    center_y: u8,
    roll_x: u8,
    roll_y: u8,
) -> Option<(u8, u8)> {
    let dx = i16::from(roll_x % (COMBAT_SWARM_JITTER_ROLL_MAX + 1))
        - i16::from(COMBAT_SWARM_JITTER_CENTER_ROLL);
    let dy = i16::from(roll_y % (COMBAT_SWARM_JITTER_ROLL_MAX + 1))
        - i16::from(COMBAT_SWARM_JITTER_CENTER_ROLL);
    let x = i16::from(center_x) + dx;
    let y = i16::from(center_y) + dy;
    combat_arena_coordinate_in_bounds(x, y).then_some((x as u8, y as u8))
}

pub fn combat_ring_candidate_coordinates_around(center_x: i16, center_y: i16) -> Vec<(u8, u8)> {
    const OFFSETS: [(i16, i16); 8] = [
        (0, -1),
        (1, -1),
        (1, 0),
        (1, 1),
        (0, 1),
        (-1, 1),
        (-1, 0),
        (-1, -1),
    ];
    OFFSETS
        .iter()
        .filter_map(|(dx, dy)| {
            let x = center_x + dx;
            let y = center_y + dy;
            combat_arena_coordinate_in_bounds(x, y).then_some((x as u8, y as u8))
        })
        .collect()
}

pub fn combat_ring_candidate_coordinates(center_x: u8, center_y: u8) -> Vec<(u8, u8)> {
    combat_ring_candidate_coordinates_around(i16::from(center_x), i16::from(center_y))
}

pub fn combat_direction_target_coordinate(
    center_x: u8,
    center_y: u8,
    direction: Direction,
) -> Option<(i16, i16)> {
    if !direction.is_cardinal() {
        return None;
    }
    let (dx, dy) = direction.delta();
    Some((
        i16::from(center_x) + dx as i16,
        i16::from(center_y) + dy as i16,
    ))
}

pub fn combat_step_direction_candidate_coordinates(
    center_x: u8,
    center_y: u8,
    step_vector: CombatStepVector,
    seed: u8,
) -> Vec<(u8, u8)> {
    let mut candidates = Vec::new();
    let preferred_x = i16::from(center_x) + i16::from(step_vector.dx);
    let preferred_y = i16::from(center_y) + i16::from(step_vector.dy);
    if (step_vector.dx != 0 || step_vector.dy != 0)
        && combat_arena_coordinate_in_bounds(preferred_x, preferred_y)
    {
        candidates.push((preferred_x as u8, preferred_y as u8));
    }

    for candidate in combat_neighbor_candidate_coordinates(center_x, center_y, seed) {
        if !candidates.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    candidates
}

pub const fn resolve_default_monster_death_marker(
    drop_cap: u8,
    first_gate_accepts: bool,
    second_gate_accepts: bool,
) -> CombatDefaultDeathMarker {
    if first_gate_accepts {
        let loot_byte = if second_gate_accepts {
            drop_cap | 0x80
        } else {
            drop_cap
        };
        CombatDefaultDeathMarker::Drop { loot_byte }
    } else {
        CombatDefaultDeathMarker::NoDrop
    }
}

pub const fn combat_default_death_drop_gate_accepts(drop_cap: u8, roll_0_to_99: u8) -> bool {
    roll_0_to_99 < drop_cap
}

pub fn resolve_default_monster_death_marker_for_class(
    class: u8,
    first_gate_accepts: bool,
    second_gate_accepts: bool,
) -> Option<CombatDefaultDeathMarker> {
    let stats = combat_class_stats(class)?;
    Some(resolve_default_monster_death_marker(
        stats.default_drop_cap,
        first_gate_accepts,
        second_gate_accepts,
    ))
}

pub fn resolve_combat_split_placement(
    class: u8,
    applied_damage: u8,
    killed: bool,
    descriptors: &[CombatActorDescriptor],
    candidate_slots: &[usize],
) -> Option<CombatSplitPlacement> {
    let traits = combat_class_traits(class)?;
    if !traits.splits || applied_damage == 0 || killed {
        return None;
    }

    candidate_slots
        .iter()
        .copied()
        .take(8)
        .find(|slot| {
            *slot < COMBAT_ACTOR_SLOTS
                && descriptors
                    .get(*slot)
                    .is_some_and(|descriptor| descriptor.is_empty())
        })
        .map(|slot| CombatSplitPlacement { slot, class })
}

pub fn resolve_combat_ai_attack_route(class: u8, target_range: u8) -> Option<CombatAiAttackRoute> {
    let ranged = combat_ranged_effect_stats(class)?;
    if target_range > ranged.range_effect_selector {
        return Some(CombatAiAttackRoute::OutOfRange);
    }
    if target_range <= 1 {
        return Some(CombatAiAttackRoute::Melee);
    }
    Some(CombatAiAttackRoute::RangedEffect {
        range_effect_selector: ranged.range_effect_selector,
        payload: ranged.payload,
        scene_resistance: ranged.scene_resistance,
        cast_like_branch: ranged.cast_like_branch,
        pre_gate_bypass: ranged.pre_gate_bypass,
    })
}

pub const fn combat_ai_special_one_in_eight_gate(roll: u8) -> bool {
    roll & 0x07 == 0
}

pub const fn resolve_combat_ai_special_hook_for_traits(
    traits: CombatClassTraits,
    possess_candidate_reaches_resistance: bool,
    blink_roll: u8,
    summon_roll: u8,
    summon_can_place_daemon: bool,
) -> Option<CombatAiSpecialHook> {
    if traits.possess && possess_candidate_reaches_resistance {
        Some(CombatAiSpecialHook::Possess)
    } else if traits.blink && combat_ai_special_one_in_eight_gate(blink_roll) {
        Some(CombatAiSpecialHook::Blink)
    } else if traits.summon_daemon
        && combat_ai_special_one_in_eight_gate(summon_roll)
        && summon_can_place_daemon
    {
        Some(CombatAiSpecialHook::SummonDaemon)
    } else {
        None
    }
}

pub fn resolve_combat_ai_special_hook(
    class: u8,
    possess_candidate_reaches_resistance: bool,
    blink_roll: u8,
    summon_roll: u8,
    summon_can_place_daemon: bool,
) -> Option<CombatAiSpecialHook> {
    let traits = combat_class_traits(class)?;
    resolve_combat_ai_special_hook_for_traits(
        traits,
        possess_candidate_reaches_resistance,
        blink_roll,
        summon_roll,
        summon_can_place_daemon,
    )
}

pub fn combat_possess_candidate_view(
    descriptor: CombatActorDescriptor,
    member: Option<PartyMember>,
    suppressed: bool,
    invisible_or_unrevealed: bool,
) -> CombatPossessCandidateView {
    CombatPossessCandidateView {
        descriptor,
        member,
        suppressed,
        invisible_or_unrevealed,
    }
}

pub fn combat_possess_candidate_reaches_resistance(
    slot: usize,
    candidate: CombatPossessCandidateView,
    active_player: Option<usize>,
) -> bool {
    if slot >= COMBAT_PARTY_ACTOR_SLOTS
        || active_player == Some(slot)
        || candidate.descriptor.is_empty()
        || candidate.descriptor.is_marked_dead()
        || !candidate.descriptor.has_field_lookup_selectable_bit()
        || candidate.descriptor.is_controlled()
        || candidate.descriptor.is_status_disabled()
        || candidate.descriptor.is_hidden_or_unrevealed()
        || candidate.suppressed
        || candidate.invisible_or_unrevealed
    {
        return false;
    }

    candidate
        .member
        .is_some_and(|member| member.living() && matches!(member.status, b'G' | b'P'))
}

pub fn resolve_combat_possess_candidate_slot(
    candidates: &[CombatPossessCandidateView],
    random_slot: usize,
    active_player: Option<usize>,
) -> Option<usize> {
    let candidate = *candidates.get(random_slot)?;
    combat_possess_candidate_reaches_resistance(random_slot, candidate, active_player)
        .then_some(random_slot)
}

pub const fn resolve_combat_possess_resistance_outcome(
    target_slot: usize,
    caster_class: u8,
    active_player: Option<usize>,
    resistance_blocks: bool,
) -> CombatPossessResistanceOutcome {
    if resistance_blocks {
        CombatPossessResistanceOutcome::Blocked
    } else {
        CombatPossessResistanceOutcome::Landed {
            cleared_active_player: matches!(active_player, Some(slot) if slot == target_slot),
            daemon_clears_self: caster_class == COMBAT_CLASS_DAEMON,
        }
    }
}

pub fn resolve_poison_status_attack_for_party_target(
    attacker_class: u8,
    target: &mut PartyMember,
    gate_accepts: bool,
    fallback_raw_damage: u8,
) -> Option<CombatPoisonStatusAttackOutcome> {
    let traits = combat_class_traits(attacker_class)?;
    if !traits.poison_status_attack {
        return Some(CombatPoisonStatusAttackOutcome::NotPoisonStatusClass);
    }
    if !gate_accepts {
        return Some(CombatPoisonStatusAttackOutcome::GateRejected);
    }
    if target.living() && target.status == b'G' {
        let status_before = target.status;
        target.status = b'P';
        return Some(CombatPoisonStatusAttackOutcome::PoisonedPartyMember {
            status_before,
            status_after: target.status,
        });
    }
    Some(CombatPoisonStatusAttackOutcome::FallbackDamage {
        raw_damage: fallback_raw_damage,
    })
}

pub const fn resolve_combat_field_placement_acceptance(
    field: CombatArenaFieldKind,
    callback_accepts: bool,
) -> bool {
    let _ = (field, callback_accepts);
    true
}

pub const fn combat_field_poison_fallback_damage(roll: u8) -> u8 {
    1 + (roll % 20)
}

pub const fn combat_field_fire_raw_damage(roll: u8) -> u8 {
    1 + (roll % 21)
}

pub fn resolve_combat_arena_field_contact_for_party_target(
    field: CombatArenaFieldKind,
    current_active_slot: usize,
    target_slot: usize,
    linked_active_object_tile: u8,
    target: &mut PartyMember,
    poison_damage_roll: u8,
    fire_damage_roll: u8,
) -> CombatArenaFieldContactOutcome {
    if current_active_slot == target_slot {
        return CombatArenaFieldContactOutcome::SkippedCurrentActor;
    }

    match field {
        CombatArenaFieldKind::Poison => {
            if linked_active_object_tile >= 0x80 {
                return CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass;
            }
            match apply_combat_poison_to_party_target(target, poison_damage_roll) {
                CombatPartyPoisonOutcome::PoisonedPartyMember {
                    status_before,
                    status_after,
                } => CombatArenaFieldContactOutcome::PoisonedPartyMember {
                    status_before,
                    status_after,
                },
                CombatPartyPoisonOutcome::FallbackDamage { raw_damage } => {
                    CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage }
                }
            }
        }
        CombatArenaFieldKind::Sleep => match apply_combat_sleep_to_party_target(target) {
            CombatPartySleepOutcome::SkippedDeadParty => {
                CombatArenaFieldContactOutcome::SleepSkippedDeadParty
            }
            CombatPartySleepOutcome::SleptPartyMember {
                status_before,
                status_after,
            } => CombatArenaFieldContactOutcome::SleptPartyMember {
                status_before,
                status_after,
            },
        },
        CombatArenaFieldKind::Fire => CombatArenaFieldContactOutcome::FireDamage {
            raw_damage: combat_field_fire_raw_damage(fire_damage_roll),
        },
        CombatArenaFieldKind::Energy => {
            CombatArenaFieldContactOutcome::EnergyDamage { raw_damage: 0 }
        }
    }
}

pub fn resolve_combat_arena_field_contact_for_non_party_target(
    field: CombatArenaFieldKind,
    current_active_slot: usize,
    target_slot: usize,
    linked_active_object_tile: u8,
    poison_damage_roll: u8,
    fire_damage_roll: u8,
) -> CombatArenaFieldContactOutcome {
    if current_active_slot == target_slot {
        return CombatArenaFieldContactOutcome::SkippedCurrentActor;
    }

    match field {
        CombatArenaFieldKind::Poison => {
            if linked_active_object_tile >= 0x80 {
                CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass
            } else {
                CombatArenaFieldContactOutcome::PoisonFallbackDamage {
                    raw_damage: combat_field_poison_fallback_damage(poison_damage_roll),
                }
            }
        }
        CombatArenaFieldKind::Sleep => CombatArenaFieldContactOutcome::SleepDisabledNonParty,
        CombatArenaFieldKind::Fire => CombatArenaFieldContactOutcome::FireDamage {
            raw_damage: combat_field_fire_raw_damage(fire_damage_roll),
        },
        CombatArenaFieldKind::Energy => {
            CombatArenaFieldContactOutcome::EnergyDamage { raw_damage: 0 }
        }
    }
}

/// `combat.md §11` Amulet/Turning scatter-mode roll threshold. The
/// turnable-attack helper rolls a uniform `[0, 255]` byte; rolls
/// strictly below this threshold force the ranged/effect helper
/// into scatter mode, while rolls at or above it use the ordinary
/// hit-roll path. The threshold is the midpoint of the byte
/// domain, so the scatter outcome fires at exactly 1-in-2 odds.
pub const AMULET_TURNING_SCATTER_THRESHOLD: u8 = 128;

pub const fn resolve_amulet_turning_scatter(
    turnable_attack: bool,
    living_party_target: bool,
    amulet_turning_readied: bool,
    roll: u8,
) -> bool {
    turnable_attack
        && living_party_target
        && amulet_turning_readied
        && roll < AMULET_TURNING_SCATTER_THRESHOLD
}

pub fn resolve_amulet_turning_scatter_for_class(
    attacker_class: u8,
    living_party_target: bool,
    amulet_turning_readied: bool,
    roll: u8,
) -> Option<bool> {
    let traits = combat_class_traits(attacker_class)?;
    Some(resolve_amulet_turning_scatter(
        traits.turnable_attack,
        living_party_target,
        amulet_turning_readied,
        roll,
    ))
}

pub fn resolve_amulet_turning_scatter_for_party_target(
    attacker_class: u8,
    target: PartyMember,
    equipment: &[u8; EQUIPMENT_SLOT_COUNT],
    roll: u8,
) -> Option<bool> {
    resolve_amulet_turning_scatter_for_class(
        attacker_class,
        target.living(),
        is_amulet_turning_readied(equipment),
        roll,
    )
}

pub fn find_combat_actor_at_field_coordinate(
    descriptors: &[CombatActorDescriptor],
    active_objects: &[ActiveObject],
    x: u8,
    y: u8,
) -> Option<usize> {
    find_combat_actor_at_field_coordinate_skipping(descriptors, active_objects, x, y, None)
}

pub fn find_combat_actor_at_field_coordinate_skipping(
    descriptors: &[CombatActorDescriptor],
    active_objects: &[ActiveObject],
    x: u8,
    y: u8,
    skip_slot: Option<usize>,
) -> Option<usize> {
    descriptors
        .iter()
        .enumerate()
        .find(|(slot, descriptor)| {
            if Some(*slot) == skip_slot || descriptor.x != x || descriptor.y != y {
                return false;
            }
            let Some(active_object) = active_objects.get(descriptor.active_object_slot as usize)
            else {
                return false;
            };
            descriptor.eligible_for_field_coordinate_lookup(active_object.tile)
        })
        .map(|(slot, _)| slot)
}

/// `catalogs/item-list.md §5.3` shared combat to-hit score bias.
/// The shared to-hit helper computes the score as
/// `(attacker - defender + COMBAT_TO_HIT_BIAS) / 2` and compares it
/// against a uniform random byte. The +30 bias balances the score
/// so that two equal-rating actors clear the median of `0..=255`.
pub const COMBAT_TO_HIT_BIAS: i16 = 30;

pub const fn combat_to_hit_score(attacker_rating: u8, defender_rating: u8) -> i16 {
    ((attacker_rating as i16 - defender_rating as i16) + COMBAT_TO_HIT_BIAS) / 2
}

pub const fn resolve_combat_hit(attacker_rating: u8, defender_rating: u8, roll: u8) -> bool {
    combat_to_hit_score(attacker_rating, defender_rating) > roll as i16
}

pub const fn resolve_mass_charm_target_group(normal_group: u8, threshold: u8, roll: u8) -> u8 {
    if roll > threshold { 0 } else { normal_group }
}

pub fn party_name_forces_monster_combat_group(name: &[u8]) -> bool {
    name.get(4).copied() == Some(b'j')
}

pub fn resolve_combat_target_group(
    slot: usize,
    party_name: Option<&[u8]>,
    team_toggled: bool,
) -> u8 {
    if slot >= COMBAT_ACTOR_SLOTS {
        return COMBAT_TARGET_GROUP_NEUTRAL;
    }

    if slot < COMBAT_PARTY_ACTOR_SLOTS {
        if party_name.is_some_and(party_name_forces_monster_combat_group) {
            return COMBAT_TARGET_GROUP_MONSTER;
        }
        if team_toggled {
            COMBAT_TARGET_GROUP_MONSTER
        } else {
            COMBAT_TARGET_GROUP_PARTY
        }
    } else if team_toggled {
        COMBAT_TARGET_GROUP_PARTY
    } else {
        COMBAT_TARGET_GROUP_MONSTER
    }
}

pub fn resolve_combat_target_group_for_actor(
    actor: CombatActorDescriptor,
    slot: usize,
    party_name: Option<&[u8]>,
) -> u8 {
    resolve_combat_target_group(slot, party_name, actor.team_toggled())
}

pub fn combat_target_candidate_view_from_descriptor(
    descriptor: CombatActorDescriptor,
    slot: usize,
    party_name: Option<&[u8]>,
    suppressed: bool,
    invisible_or_unrevealed: bool,
) -> CombatTargetCandidateView {
    CombatTargetCandidateView {
        descriptor,
        group: resolve_combat_target_group_for_actor(descriptor, slot, party_name),
        suppressed,
        invisible_or_unrevealed,
    }
}

pub const fn age_active_effect_state(tag: Option<u8>, counter: u8) -> ActiveEffectAgeOutcome {
    if counter == 0 {
        ActiveEffectAgeOutcome {
            tag: None,
            counter: 0,
            expired: false,
        }
    } else if counter == u8::MAX {
        ActiveEffectAgeOutcome {
            tag,
            counter,
            expired: false,
        }
    } else {
        let counter = counter - 1;
        if counter == 0 {
            ActiveEffectAgeOutcome {
                tag: None,
                counter,
                expired: true,
            }
        } else {
            ActiveEffectAgeOutcome {
                tag,
                counter,
                expired: false,
            }
        }
    }
}

pub const fn active_effect_is_active(tag: Option<u8>, counter: u8, expected_tag: u8) -> bool {
    match tag {
        Some(tag) => tag == expected_tag && counter != 0,
        None => false,
    }
}

pub const fn resolve_protection_defense_bonus(
    base_defense: u8,
    tag: Option<u8>,
    counter: u8,
) -> u8 {
    if active_effect_is_active(tag, counter, PROTECTION_ACTIVE_EFFECT_TAG) {
        base_defense.saturating_add(PROTECTION_ACTIVE_EFFECT_DEFENSE_BONUS)
    } else {
        base_defense
    }
}

pub const fn resolve_quickness_dispatch_consumed(tag: Option<u8>, counter: u8, roll: u8) -> bool {
    active_effect_is_active(tag, counter, QUICKNESS_ACTIVE_EFFECT_TAG) && roll == 0
}

pub const fn resolve_negate_magic_absorbs_combat_cast(tag: Option<u8>, counter: u8) -> bool {
    active_effect_is_active(tag, counter, NEGATE_MAGIC_ACTIVE_EFFECT_TAG)
}

pub const fn combat_ai_step_vector(
    from_x: u8,
    from_y: u8,
    target_x: u8,
    target_y: u8,
    fleeing: bool,
) -> CombatStepVector {
    let mut dx = coordinate_step(from_x, target_x);
    let mut dy = coordinate_step(from_y, target_y);
    if fleeing {
        dx = -dx;
        dy = -dy;
    }
    CombatStepVector { dx, dy }
}

pub fn find_combat_ai_target(
    actors: &[CombatTargetCandidateView],
    acting_slot: usize,
    acting_group: u8,
    bypass_suppression_filter: bool,
) -> CombatTargetPick {
    let mut best_slot = None;
    let mut best_range = u8::MAX;
    let mut first_five_party_slot_survived = false;
    let Some(acting_actor) = actors.get(acting_slot) else {
        return CombatTargetPick {
            slot: None,
            first_five_party_slot_survived,
        };
    };
    let scan_len = actors.len().min(COMBAT_ACTOR_SLOTS);

    for slot in (0..scan_len).rev() {
        if slot == acting_slot {
            continue;
        }
        let candidate = actors[slot];
        if candidate.descriptor.is_empty()
            || candidate.descriptor.is_marked_dead()
            || candidate.group == acting_group
            || (!bypass_suppression_filter && candidate.suppressed)
            || candidate.invisible_or_unrevealed
        {
            continue;
        }

        if slot < COMBAT_TARGET_PICK_COUNTED_PARTY_SLOTS {
            first_five_party_slot_survived = true;
        }

        let range = acting_actor.descriptor.range_to(candidate.descriptor);
        if range <= best_range {
            best_range = range;
            best_slot = Some(slot);
        }
    }

    CombatTargetPick {
        slot: best_slot,
        first_five_party_slot_survived,
    }
}

pub const fn combat_ai_center_fallback_target() -> (u8, u8) {
    (
        COMBAT_ARENA_CENTER_COORDINATE,
        COMBAT_ARENA_CENTER_COORDINATE,
    )
}

pub fn apply_combat_ai_center_fallback_markers(actors: &mut [CombatActorDescriptor]) -> Vec<usize> {
    let mut critical_hp_flee_slots = Vec::new();
    let max_slot = actors
        .len()
        .min(COMBAT_NO_TARGET_FLEE_MAX_SLOT + 1)
        .saturating_sub(1);
    if max_slot < COMBAT_NO_TARGET_FLEE_MIN_SLOT {
        return critical_hp_flee_slots;
    }
    for slot in (COMBAT_NO_TARGET_FLEE_MIN_SLOT..=max_slot).rev() {
        let actor = &mut actors[slot];
        if !cause_fear_actor_is_live(*actor) || actor.flags & COMBAT_ACTOR_FLAG_SELECTABLE_40 == 0 {
            continue;
        }
        let Some(stats) = combat_class_stats(actor.owner_target_class) else {
            continue;
        };
        actor.hp_or_wound = cause_fear_forced_current_hp(stats.max_hp);
        actor.phase_counter = COMBAT_NO_TARGET_FLEE_STEP_QUEUE;
        actor.set_fleeing(true);
        critical_hp_flee_slots.push(slot);
    }
    critical_hp_flee_slots
}

pub fn resolve_combat_ai_target_after_scan(
    actors: &mut [CombatActorDescriptor],
    pick: CombatTargetPick,
    cleanup_fallback_target: Option<(u8, u8)>,
) -> CombatAiTargetResolution {
    if let Some(slot) = pick.slot {
        return actors
            .get(slot)
            .copied()
            .filter(|actor| !actor.is_empty() && !actor.is_marked_dead())
            .map(|actor| CombatAiTargetResolution::ChosenActor {
                slot,
                x: actor.x,
                y: actor.y,
            })
            .unwrap_or(CombatAiTargetResolution::NoUsableTarget);
    }

    if pick.first_five_party_slot_survived {
        return CombatAiTargetResolution::NoUsableTarget;
    }

    if let Some((x, y)) = cleanup_fallback_target {
        return CombatAiTargetResolution::CleanupFallback { x, y };
    }

    let (x, y) = combat_ai_center_fallback_target();
    CombatAiTargetResolution::CenterFallback {
        x,
        y,
        critical_hp_flee_slots: apply_combat_ai_center_fallback_markers(actors),
    }
}

pub fn combat_ai_legal_cell(
    legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    x: i16,
    y: i16,
) -> bool {
    combat_arena_coordinate_in_bounds(x, y) && legal_cells[y as usize][x as usize]
}

pub const fn combat_actor_occupies_arena_cell(actor: CombatActorDescriptor, x: u8, y: u8) -> bool {
    combat_actor_is_present_not_dead(actor) && actor.x == x && actor.y == y
}

pub fn build_combat_ai_legal_cell_mask(
    terrain: &[[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    actors: &[CombatActorDescriptor],
    terrain_walkable: impl Fn(u8) -> bool,
) -> [[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE] {
    let mut legal_cells = [[false; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    for y in 0..COMBAT_ARENA_SIDE {
        for x in 0..COMBAT_ARENA_SIDE {
            legal_cells[y][x] = terrain_walkable(terrain[y][x]);
        }
    }

    for actor in actors.iter().copied().take(COMBAT_ACTOR_SLOTS) {
        if !combat_actor_is_present_not_dead(actor) {
            continue;
        }
        let x = actor.x as usize;
        let y = actor.y as usize;
        if x < COMBAT_ARENA_SIDE && y < COMBAT_ARENA_SIDE {
            legal_cells[y][x] = false;
        }
    }

    legal_cells
}

pub fn commit_combat_actor_linked_position(
    actor: &mut CombatActorDescriptor,
    active_objects: &mut [ActiveObject],
    x: u8,
    y: u8,
) -> Option<CombatLinkedPositionCommitOutcome> {
    if actor.is_empty() || actor.is_marked_dead() {
        return None;
    }

    let active_object_slot = actor.active_object_slot as usize;
    let actor_position_before = (actor.x, actor.y);
    actor.x = x;
    actor.y = y;

    let mut active_object_position_before = None;
    let mut active_object_position_after = None;
    if let Some(object) = active_objects.get_mut(active_object_slot) {
        active_object_position_before = Some((object.x, object.y));
        object.x = x as usize;
        object.y = y as usize;
        active_object_position_after = Some((object.x, object.y));
    }

    Some(CombatLinkedPositionCommitOutcome {
        active_object_slot,
        actor_position_before,
        actor_position_after: (actor.x, actor.y),
        active_object_position_before,
        active_object_position_after,
    })
}

pub fn commit_combat_ai_movement_outcome(
    actor: &mut CombatActorDescriptor,
    active_objects: &mut [ActiveObject],
    movement: CombatAiMovementOutcome,
) -> Option<CombatLinkedPositionCommitOutcome> {
    match movement {
        CombatAiMovementOutcome::Teleport { x, y } | CombatAiMovementOutcome::Step { x, y, .. } => {
            commit_combat_actor_linked_position(actor, active_objects, x, y)
        }
        CombatAiMovementOutcome::Blocked { .. } => None,
    }
}

pub fn combat_ai_cardinal_neighbors_surrounded(
    legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    x: u8,
    y: u8,
) -> bool {
    (1..=4).all(|direction_code| {
        let destination = resolve_combat_step_destination(x, y, direction_code);
        !combat_ai_legal_cell(legal_cells, destination.x, destination.y)
    })
}

pub fn resolve_combat_ai_movement(
    legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    actor_x: u8,
    actor_y: u8,
    step_vector: CombatStepVector,
    teleport_capable: bool,
    teleport_candidate: Option<(u8, u8)>,
    horizontal_axis_first: bool,
    random_cardinal_direction_codes: &[u8],
) -> CombatAiMovementOutcome {
    if teleport_capable {
        if let Some((x, y)) = teleport_candidate {
            if combat_ai_legal_cell(legal_cells, x as i16, y as i16) {
                return CombatAiMovementOutcome::Teleport { x, y };
            }
        }
    }

    let surrounded = combat_ai_cardinal_neighbors_surrounded(legal_cells, actor_x, actor_y);
    if surrounded {
        return CombatAiMovementOutcome::Blocked { surrounded };
    }

    let horizontal = combat_direction_code_for_step(step_vector.dx, 0);
    let vertical = combat_direction_code_for_step(0, step_vector.dy);
    let direct = if horizontal_axis_first {
        [horizontal, vertical]
    } else {
        [vertical, horizontal]
    };

    for direction_code in direct.into_iter().flatten() {
        let destination = resolve_combat_step_destination(actor_x, actor_y, direction_code);
        if combat_ai_legal_cell(legal_cells, destination.x, destination.y) {
            return CombatAiMovementOutcome::Step {
                direction_code,
                x: destination.x as u8,
                y: destination.y as u8,
            };
        }
    }

    for direction_code in random_cardinal_direction_codes
        .iter()
        .copied()
        .filter(|direction_code| combat_direction_code_is_cardinal(*direction_code))
    {
        let destination = resolve_combat_step_destination(actor_x, actor_y, direction_code);
        if combat_ai_legal_cell(legal_cells, destination.x, destination.y) {
            return CombatAiMovementOutcome::Step {
                direction_code,
                x: destination.x as u8,
                y: destination.y as u8,
            };
        }
    }

    CombatAiMovementOutcome::Blocked { surrounded }
}

pub fn resolve_combat_wound_morale(
    current_hp: u8,
    max_hp: u8,
    morale_roll: u8,
) -> CombatWoundMorale {
    let bucket = combat_wound_score_bucket(current_hp, max_hp);
    let fleeing = match bucket {
        CombatWoundScoreBucket::UnderOneQuarter => true,
        CombatWoundScoreBucket::OneQuarterToUnderHalf => {
            (morale_roll as u16) < WOUND_MORALE_FLEE_THRESHOLD
        }
        CombatWoundScoreBucket::HalfToUnderThreeQuarters
        | CombatWoundScoreBucket::ThreeQuartersOrMore => false,
    };
    CombatWoundMorale { bucket, fleeing }
}

pub fn resolve_combat_wound_morale_for_class(
    current_hp: u8,
    class: u8,
    morale_roll: u8,
) -> Option<CombatWoundMorale> {
    let stats = combat_class_stats(class)?;
    Some(resolve_combat_wound_morale(
        current_hp,
        stats.max_hp,
        morale_roll,
    ))
}

pub fn combat_wound_score_bucket(current_hp: u8, max_hp: u8) -> CombatWoundScoreBucket {
    if max_hp == 0 {
        return CombatWoundScoreBucket::UnderOneQuarter;
    }
    let hp = current_hp.min(max_hp) as u16;
    let max = max_hp as u16;
    let scaled = hp * 4;
    if scaled < max {
        CombatWoundScoreBucket::UnderOneQuarter
    } else if hp * 2 < max {
        CombatWoundScoreBucket::OneQuarterToUnderHalf
    } else if scaled < max * 3 {
        CombatWoundScoreBucket::HalfToUnderThreeQuarters
    } else {
        CombatWoundScoreBucket::ThreeQuartersOrMore
    }
}

pub fn combat_arena_range(from_x: u8, from_y: u8, to_x: u8, to_y: u8) -> u8 {
    let dx = from_x.abs_diff(to_x) as u16;
    let dy = from_y.abs_diff(to_y) as u16;
    integer_square_root((dx * dx) + (dy * dy)) as u8
}

const fn coordinate_step(from: u8, to: u8) -> i8 {
    if to > from {
        1
    } else if to < from {
        -1
    } else {
        0
    }
}

fn integer_square_root(value: u16) -> u16 {
    let mut root = 0u16;
    while (root + 1) * (root + 1) <= value {
        root += 1;
    }
    root
}

//! Combat enter/exit framing helpers.

use std::io;

use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatFrameSnapshot {
    pub area: Area,
    pub player: Player,
    pub active_objects: Vec<ActiveObject>,
    pub active_player: Option<usize>,
    pub combat_terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub dungeon_room_clear_on_success: Option<PendingDungeonRoomClear>,
    pub enter_endgame_after_successful_combat: bool,
    pub endgame_messages: Option<EndgameMessages>,
    pub endgame_tableau_map: Option<MiscmapsCutsceneMap>,
    /// `combat.md §14`: the Escape refusal chooses `Not here` when the
    /// encounter entry mode has its high bit set.
    pub encounter_mode_high_bit: bool,
    /// `combat.md §6.3`: rest/camp alternate-entry modes 4 and 6 set bit
    /// `0x04`, suppressing the one world tick normally run by the controlled-
    /// party faint sleep helper.
    pub suppress_controlled_faint_sleep_tick: bool,
    /// One-shot out-of-arena exit announcement state sampled by Escape.
    pub exit_announced: bool,
    /// First party-side cardinal edge direction accepted in this combat.
    pub established_exit_direction_code: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingDungeonRoomClear {
    pub scene: DungeonScene,
    pub room_slot: u8,
}

pub const COMBAT_TERRAIN_REVEAL_PIXEL_COUNT: usize = 16 * 16;
pub const COMBAT_TERRAIN_REVEAL_WORLD_TICKS: u8 = 31;

/// Completed `combat.md §6.3` vanish-on-death cell reveal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatTerrainRevealPlayback {
    pub actor_slot: usize,
    pub arena_cell: (u8, u8),
    pub terrain_tile: u8,
    pub pixel_order: Vec<(u8, u8)>,
    pub world_tick_after_operations: Vec<u16>,
}

/// EGA baseline visit order: top-left first, then the maximal-length 8-bit
/// shift-register permutation of every nonzero local pixel coordinate.
pub fn combat_terrain_reveal_pixel_order() -> Vec<(u8, u8)> {
    let mut order = Vec::with_capacity(COMBAT_TERRAIN_REVEAL_PIXEL_COUNT);
    order.push((0, 0));
    let mut state = 1u8;
    for _ in 1..COMBAT_TERRAIN_REVEAL_PIXEL_COUNT {
        order.push((state >> 4, state & 0x0f));
        let discarded_low_bit = state & 1;
        state >>= 1;
        if discarded_low_bit != 0 {
            state ^= 0xb8;
        }
    }
    order
}

pub fn combat_terrain_reveal_playback(
    actor_slot: usize,
    arena_cell: (u8, u8),
    terrain_tile: u8,
) -> CombatTerrainRevealPlayback {
    CombatTerrainRevealPlayback {
        actor_slot,
        arena_cell,
        terrain_tile,
        pixel_order: combat_terrain_reveal_pixel_order(),
        world_tick_after_operations: (1..=COMBAT_TERRAIN_REVEAL_WORLD_TICKS)
            .map(|step| u16::from(step) * 8)
            .collect(),
    }
}

/// `combat.md §5` ambush / camp-attack reveal-slot capacity.
/// Ambush-style and camp-attack arenas can stamp up to eight
/// hidden reveal coordinates; stepping onto one consumes the
/// coordinate and rewrites one or two arena cells with the
/// associated reveal tile when their target coordinates are
/// inside the eleven-by-eleven arena. Coordinates outside the
/// arena are sentinels for "no stamp" rather than map cells.
pub const COMBAT_AMBUSH_REVEAL_SLOTS_MAX: u8 = 8;
pub const COMBAT_AMBUSH_REVEAL_SLOT_COUNT: usize = COMBAT_AMBUSH_REVEAL_SLOTS_MAX as usize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatAmbushRevealRecord {
    pub trigger_x: u8,
    pub trigger_y: u8,
    pub reveal_tile: u8,
    pub target_a_x: u8,
    pub target_a_y: u8,
    pub target_b_x: u8,
    pub target_b_y: u8,
}

impl CombatAmbushRevealRecord {
    pub const fn new(
        trigger_x: u8,
        trigger_y: u8,
        reveal_tile: u8,
        target_a_x: u8,
        target_a_y: u8,
        target_b_x: u8,
        target_b_y: u8,
    ) -> Self {
        Self {
            trigger_x,
            trigger_y,
            reveal_tile,
            target_a_x,
            target_a_y,
            target_b_x,
            target_b_y,
        }
    }

    pub const fn trigger_matches(self, x: u8, y: u8) -> bool {
        self.trigger_x == x && self.trigger_y == y
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatAmbushRevealApplication {
    pub slot: usize,
    pub trigger_x: u8,
    pub trigger_y: u8,
    pub reveal_tile: u8,
    pub stamped_cells: u8,
}

/// `combat.md §14` round-loop exit outcomes. The framer's restore
/// phase runs identically for all three; only the result code the
/// round loop returns to its caller differs. Victory and Escape
/// both return `1`; Defeat returns `0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatExitOutcome {
    /// Every hostile actor has been killed.
    Victory,
    /// The entire party is dead, asleep, or otherwise inactive.
    Defeat,
    /// The party left the arena via the out-of-bounds combat-leave
    /// helper.
    Escape,
}

impl CombatExitOutcome {
    /// `combat.md §14` the result code the combat round loop
    /// returns to the framer's caller. Victory and Escape both use
    /// `1`; Defeat uses `0`.
    pub const fn result_code(self) -> u8 {
        match self {
            Self::Victory | Self::Escape => 0,
            Self::Defeat => 1,
        }
    }
}

/// Combat cursor blink state after a round boundary: which arena cell
/// the blinking active-actor cursor should be drawn on, and where the
/// secondary marker sits when it is inside the arena.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatCursorBlinkReport {
    pub cursor_blink_visible: bool,
    pub cursor_draw_cell: Option<(u8, u8)>,
    pub secondary_marker_cell: Option<(u8, u8)>,
}

/// `combat.md Section 12` split-on-damage placement-attempt cap.
/// When a monster with the split-on-damage class flag is damaged but
/// not killed, combat scans the actor table for an empty slot to copy
/// the parent's class byte into. Up to this many attempts are made before
/// the divide is dropped silently.
pub const COMBAT_SPLIT_PLACEMENT_ATTEMPTS: u8 = 8;

/// `combat.md §12` factory-seed cached combat-defense byte at
/// character-record offset `+0x18`. Applies to fresh save images
/// before any equipment/effect re-cache; the live combat damage
/// roll subtracts a random value driven by this byte.
pub const CHARACTER_DEFENSE_FACTORY_SEED: u8 = 7;

/// Field-marker contact reached from the terrain/effect hook is not owned by
/// a caster slot. Use an out-of-range sentinel so the shared contact helper
/// does not treat the actor stepping onto the marker as the spell's active
/// caster.
pub const COMBAT_FIELD_CONTACT_NO_ACTIVE_SKIP_SLOT: usize = COMBAT_ACTOR_SLOTS;

/// `combat.md §11` step-or-attack direction codes. The world-mode
/// loops and the combat dispatcher share this mapping: `1 = west`,
/// `2 = east`, `3 = north`, `4 = south`. Code `0` (or any value out
/// of `1..=4`) maps to `(0, 0)` and is the "attack in place" case.
pub const COMBAT_DIRECTION_WEST: u8 = 1;
pub const COMBAT_DIRECTION_EAST: u8 = 2;
pub const COMBAT_DIRECTION_NORTH: u8 = 3;
pub const COMBAT_DIRECTION_SOUTH: u8 = 4;

const AMULET_TURNING_SCATTER_OFFSETS: [(i8, i8); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

/// `combat.md §11`: translate a step-or-attack direction code into
/// the `(dx, dy)` unit step the primitive applies. Code zero or any
/// value outside `1..=4` returns `(0, 0)` so the caller treats it as
/// "attack in place".
pub const fn combat_step_direction_delta(code: u8) -> (i8, i8) {
    match code {
        COMBAT_DIRECTION_WEST => (-1, 0),
        COMBAT_DIRECTION_EAST => (1, 0),
        COMBAT_DIRECTION_NORTH => (0, -1),
        COMBAT_DIRECTION_SOUTH => (0, 1),
        _ => (0, 0),
    }
}

/// `combat.md §4` post-combat active-player restore. The framer
/// restores the saved active-player slot only when the pre-combat
/// active player has not died or fallen asleep during the fight; a
/// `'D'` (Dead) or `'S'` (Sleeping) post-combat status keeps the
/// active-player slot cleared so the player must re-select.
pub const fn combat_restore_active_player_slot(
    saved_slot: u8,
    post_combat_status: CharacterStatus,
) -> Option<u8> {
    match post_combat_status {
        CharacterStatus::Dead | CharacterStatus::Sleeping => None,
        _ => Some(saved_slot),
    }
}

pub fn resolve_amulet_turning_scatter_cell(
    target_x: u8,
    target_y: u8,
    attacker_x: u8,
    attacker_y: u8,
    roll: u8,
) -> (i8, i8) {
    let mut index = usize::from(roll & 7);
    loop {
        let (dx, dy) = AMULET_TURNING_SCATTER_OFFSETS[index];
        let x = target_x as i8 + dx;
        let y = target_y as i8 + dy;
        if x != attacker_x as i8 || y != attacker_y as i8 {
            return (x, y);
        }
        index = (index + 1) % AMULET_TURNING_SCATTER_OFFSETS.len();
    }
}

/// The active-player sentinel names a **roster** slot, not a combat
/// descriptor slot: `combat.md §9` says the sentinel "is compared against
/// the target's own owner/character byte", and `§5` makes the descriptor
/// index a packed index that a dead member shifts away from the roster
/// index. Bound it by the saved-game party-size maximum rather than by
/// the combat descriptor count; the two constants share a value, so this
/// is a semantic correction and not a behaviour change.
pub fn decode_active_player_slot(byte: u8, party_size: usize) -> Option<usize> {
    if byte == 0xff {
        return None;
    }
    let slot = usize::from(byte);
    (slot < party_size && slot < SAVE_PARTY_SIZE_MAX as usize).then_some(slot)
}

pub fn encode_active_player_slot(active_player: Option<usize>) -> u8 {
    match active_player {
        Some(slot) if slot < SAVE_PARTY_SIZE_MAX as usize => slot as u8,
        _ => 0xff,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PostCombatTriggerReconcile {
    Cleared,
    BodyRetrieval,
    MissingSlot,
}

/// `active-objects.md §9` post-combat body/retrieval rewrite constants.
/// When the resident terrain-target wrapper sees the restored
/// trigger slot in the water-creature/body family
/// (`WATER_CREATURE_BODY_TYPE_FIRST..=WATER_CREATURE_BODY_TYPE_LAST`)
/// and combat set the exit-message state, it lowers both sprite
/// bytes by [`WATER_CREATURE_BODY_SPRITE_SHIFT`] and stamps the
/// auxiliary state (`aux1 = AUX1`, `aux3 = AUX3`).
pub const WATER_CREATURE_BODY_TYPE_FIRST: u8 = 0x2C;
pub const WATER_CREATURE_BODY_TYPE_LAST: u8 = 0x2F;
pub const WATER_CREATURE_BODY_SPRITE_SHIFT: u8 = 8;
pub const WATER_CREATURE_BODY_AUX1: u8 = 0x63;
pub const WATER_CREATURE_BODY_AUX3: u8 = 0x02;

pub fn reconcile_post_combat_terrain_trigger_slot(
    active_objects: &mut [ActiveObject],
    slot: usize,
    body_retrieval_exit: bool,
) -> PostCombatTriggerReconcile {
    let Some(object) = active_objects.get_mut(slot) else {
        return PostCombatTriggerReconcile::MissingSlot;
    };

    if body_retrieval_exit
        && (WATER_CREATURE_BODY_TYPE_FIRST..=WATER_CREATURE_BODY_TYPE_LAST)
            .contains(&object.type_byte)
    {
        object.type_byte = object
            .type_byte
            .saturating_sub(WATER_CREATURE_BODY_SPRITE_SHIFT);
        object.tile = object.tile.saturating_sub(WATER_CREATURE_BODY_SPRITE_SHIFT);
        object.aux1 = WATER_CREATURE_BODY_AUX1;
        object.aux3 = WATER_CREATURE_BODY_AUX3;
        return PostCombatTriggerReconcile::BodyRetrieval;
    }

    object.type_byte = 0;
    object.tile = 0;
    object.x = 0;
    object.y = 0;
    object.z = 0;
    PostCombatTriggerReconcile::Cleared
}

pub fn combat_exit_requests_body_retrieval_reconcile(
    exit: CombatRoundLoopExit,
    actors: &[CombatActorDescriptor],
) -> bool {
    matches!(exit, CombatRoundLoopExit::Victory)
        && !combat_has_active_not_dead_non_party_actor(actors)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatWeaponDamageApplication {
    Party {
        target_slot: usize,
        damage: CombatPartyDamageOutcome,
    },
    Monster {
        target_slot: usize,
        damage: CombatMonsterDamageOutcome,
        credited_experience: Option<u16>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatWeaponAttackApplication {
    pub resolution: CombatWeaponAttackResolution,
    pub damage_application: Option<CombatWeaponDamageApplication>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatMonsterAttackApplication {
    pub attacker_slot: usize,
    pub target_slot: usize,
    pub poison_status_outcome: Option<CombatPoisonStatusAttackOutcome>,
    pub resolution: Option<CombatWeaponAttackResolution>,
    pub damage_application: Option<CombatWeaponDamageApplication>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatMonsterAttackInputs {
    pub party_defender_rating: u8,
    pub hit_roll: u8,
    pub damage_roll: u8,
    pub poison_gate_accepts: bool,
    pub poison_damage_roll: u8,
    pub forced_hit: Option<bool>,
    pub amulet_turning_scatter_roll: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatPlayerWeaponAttackInputs {
    pub hit_roll: u8,
    pub damage_roll: u8,
    pub forced_hit: Option<bool>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatActiveTargetSpellDamageApplication {
    pub kind: CombatSpellDamageKind,
    pub raw_damage: i16,
    pub damage_application: CombatWeaponDamageApplication,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatTremorSpellDamageApplication {
    pub applications: Vec<CombatTremorSpellSlotDamageApplication>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatTremorSpellSlotDamageApplication {
    pub target_slot: usize,
    pub raw_damage: i16,
    pub damage_application: CombatWeaponDamageApplication,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatDirectedSpellDamageApplication {
    pub effect: CombatDirectedSpellEffect,
    pub applications: Vec<CombatDirectedSpellSlotDamageApplication>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatDirectedSpellSlotDamageApplication {
    pub target_slot: usize,
    pub raw_damage: i16,
    pub damage_application: CombatWeaponDamageApplication,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatDirectedSpellStatusApplication {
    pub effect: CombatDirectedSpellEffect,
    pub applications: Vec<CombatDirectedSpellSlotStatusApplication>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatDirectedSpellSlotStatusApplication {
    PartySleep {
        target_slot: usize,
        outcome: CombatPartySleepOutcome,
    },
    NonPartySleepDisabled {
        target_slot: usize,
    },
    PartyPoison {
        target_slot: usize,
        outcome: CombatPartyPoisonOutcome,
        fallback_damage_application: Option<CombatWeaponDamageApplication>,
    },
    NonPartyPoisonFallbackDamage {
        target_slot: usize,
        raw_damage: i16,
        damage_application: CombatWeaponDamageApplication,
    },
    PoisonGateRejected {
        target_slot: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatArenaFieldContactApplication {
    pub field: CombatArenaFieldKind,
    pub target_slot: usize,
    pub contact_outcome: CombatArenaFieldContactOutcome,
    pub damage_application: Option<CombatWeaponDamageApplication>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPostDispatchContactSource {
    ArenaTerrain { tile: u8 },
    PlacedMarker { active_object_slot: usize },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatPostDispatchContactApplication {
    pub source: CombatPostDispatchContactSource,
    pub field_contact: CombatArenaFieldContactApplication,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatAbsorbableFieldApplication {
    pub actor_slot: usize,
    pub companion_band_index: usize,
    pub marker_byte: u8,
    pub x: u8,
    pub y: u8,
    pub armed_endgame_result: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatArenaFieldPlacementApplication {
    pub field: CombatArenaFieldKind,
    pub target_slot: Option<usize>,
    pub active_object_slot: usize,
    pub x: u8,
    pub y: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatArenaFieldRemovalApplication {
    pub field: CombatArenaFieldKind,
    pub active_object_slot: usize,
    pub x: u8,
    pub y: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatCharmApplication {
    pub target_slot: usize,
    pub flags_before: u8,
    pub flags_after: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatPolymorphApplication {
    pub target_slot: usize,
    pub active_object_slot: usize,
    pub actor_before: CombatActorDescriptor,
    pub actor_after: CombatActorDescriptor,
    pub object_before: ActiveObject,
    pub object_after: ActiveObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatCloneApplication {
    pub target_slot: usize,
    pub actor_slot: usize,
    pub active_object_slot: usize,
    pub x: u8,
    pub y: u8,
    pub actor: CombatActorDescriptor,
    pub active_object: ActiveObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatSummonApplication {
    pub class: u8,
    pub actor_slot: usize,
    pub active_object_slot: usize,
    pub x: u8,
    pub y: u8,
    pub actor: CombatActorDescriptor,
    pub active_object: ActiveObject,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatAiSpecialApplication {
    Possess {
        actor_slot: usize,
        target_slot: usize,
        outcome: CombatPossessResistanceOutcome,
        target_flags_before: u8,
        target_flags_after: u8,
    },
    Blink {
        actor_slot: usize,
        visibility: CombatLinkedVisibilityOutcome,
    },
    SummonDaemon {
        actor_slot: usize,
        summon: CombatSummonApplication,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatAiTurnApplication {
    pub actor_slot: usize,
    pub special: Option<CombatAiSpecialApplication>,
    pub possess_hook_handled: bool,
    pub acting_group: u8,
    pub target: CombatAiTargetResolution,
    pub step_vector: Option<CombatStepVector>,
    pub attack_route: Option<CombatAiAttackRoute>,
    pub monster_attack: Option<CombatMonsterAttackApplication>,
    pub movement: Option<CombatAiMovementOutcome>,
    pub command_key: Option<char>,
    pub movement_commit: Option<CombatLinkedPositionCommitOutcome>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatSleepWakeApplication {
    pub slot: usize,
    pub roll: u8,
    pub woke: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPlayerCommandInput {
    Key(char),
    Direction(u8),
    AttackDirection(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPlayerCommandAction {
    QuicknessSkipped,
    ActivePlayerSelection(CombatActivePlayerSelectionOutcome),
    Pass(CombatPassCommandOutcome),
    PromptForAttackDirection,
    StepOrAttack {
        prompted_attack: bool,
        direction_code: u8,
        outcome: CombatStepOrAttackPrimitiveOutcome,
    },
    InvalidDirection {
        direction_code: u8,
    },
    EscapeCleanup {
        application: CombatEscapeCleanupApplication,
    },
    Branch {
        branch: CombatCommandBranch,
        live_actor_gate: CombatCommandLiveActorGate,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatEscapeCleanupApplication {
    pub decision: CombatEscapeCleanupDecision,
    pub cleared_descriptor_slots: u8,
    pub cleared_active_object_slots: u8,
    pub world_ticks: u8,
    pub rising_glissando: bool,
    pub stats_panel_dirty: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatOutOfArenaLeaveApplication {
    pub outcome: CombatOutOfArenaLeaveOutcome,
    pub cleared_descriptor: bool,
    pub cleared_active_object: bool,
    pub world_ticks: u8,
}

impl CombatEscapeCleanupApplication {
    pub const fn refused(decision: CombatEscapeCleanupDecision) -> Self {
        Self {
            decision,
            cleared_descriptor_slots: 0,
            cleared_active_object_slots: 0,
            world_ticks: 0,
            rising_glissando: false,
            stats_panel_dirty: false,
        }
    }

    pub const fn accepted(cleared_descriptor_slots: u8, cleared_active_object_slots: u8) -> Self {
        Self {
            decision: CombatEscapeCleanupDecision::Accepted,
            cleared_descriptor_slots,
            cleared_active_object_slots,
            world_ticks: cleared_descriptor_slots.saturating_add(cleared_active_object_slots),
            rising_glissando: true,
            stats_panel_dirty: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatPlayerCommandApplication {
    pub actor_slot: usize,
    pub input: CombatPlayerCommandInput,
    pub action: CombatPlayerCommandAction,
    pub weapon_attack: Option<CombatWeaponAttackApplication>,
    pub ring_pass: Option<CombatMagicRingPassOutcome>,
    pub active_effect_age: Option<ActiveEffectAgeOutcome>,
    pub absorbable_contact: Option<CombatAbsorbableFieldApplication>,
    pub post_dispatch_contact: Option<CombatPostDispatchContactApplication>,
    pub out_of_arena_leave: Option<CombatOutOfArenaLeaveApplication>,
    pub victory_announced: bool,
    /// `combat.md §8`'s single re-prompt flag. When set, the same actor
    /// remains pending and has spent no combat action.
    pub reprompt: bool,
    pub control_after: CombatRoundLoopControl,
}

pub const fn combat_player_command_action_defers_maintenance(
    action: &CombatPlayerCommandAction,
) -> bool {
    match action {
        CombatPlayerCommandAction::PromptForAttackDirection => true,
        CombatPlayerCommandAction::Branch { branch, .. } => {
            combat_command_branch_is_named_multistage(*branch)
                || matches!(
                    branch,
                    // Push calls the shared direction-prompt handler directly
                    // rather than belonging to the named Shape-A prompt group,
                    // but the event-driven continuation still has to defer the
                    // parser epilogue until that handler returns. Z-stats
                    // likewise closes through its modal continuation.
                    CombatCommandBranch::Push | CombatCommandBranch::ZStats
                )
        }
        _ => false,
    }
}

pub const fn combat_player_command_action_reprompts(action: &CombatPlayerCommandAction) -> bool {
    match action {
        CombatPlayerCommandAction::QuicknessSkipped => true,
        CombatPlayerCommandAction::ActivePlayerSelection(
            CombatActivePlayerSelectionOutcome::Invalid,
        ) => true,
        CombatPlayerCommandAction::InvalidDirection { .. }
        | CombatPlayerCommandAction::EscapeCleanup {
            application:
                CombatEscapeCleanupApplication {
                    decision:
                        CombatEscapeCleanupDecision::RefusedNotHere
                        | CombatEscapeCleanupDecision::RefusedNotYet,
                    ..
                },
        } => true,
        CombatPlayerCommandAction::StepOrAttack {
            outcome:
                CombatStepOrAttackPrimitiveOutcome::InactiveActor
                | CombatStepOrAttackPrimitiveOutcome::BlockedActor { .. }
                | CombatStepOrAttackPrimitiveOutcome::BlockedWall,
            ..
        } => true,
        CombatPlayerCommandAction::Branch {
            live_actor_gate: CombatCommandLiveActorGate::RejectedDeadOrMissing,
            ..
        } => true,
        CombatPlayerCommandAction::Branch { branch, .. } => matches!(
            branch,
            CombatCommandBranch::SceneMessageAbort(_)
                | CombatCommandBranch::DWhatRefusal
                | CombatCommandBranch::WWhatRefusal
                | CombatCommandBranch::ToggleMusic
                | CombatCommandBranch::Invalid
        ),
        CombatPlayerCommandAction::ActivePlayerSelection(_)
        | CombatPlayerCommandAction::Pass(_)
        | CombatPlayerCommandAction::PromptForAttackDirection
        | CombatPlayerCommandAction::StepOrAttack { .. }
        | CombatPlayerCommandAction::EscapeCleanup {
            application:
                CombatEscapeCleanupApplication {
                    decision: CombatEscapeCleanupDecision::Accepted,
                    ..
                },
        } => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatActorDispatchAction {
    Inactive,
    PartyDeathSweep,
    Waiting,
    StatusDisabledWake {
        wake: CombatSleepWakeApplication,
    },
    PlayerReady,
    /// `combat.md §8`: a self-acting slot whose dispatch the live Quickness
    /// effect consumed. This is the only Quickness gate in the engine.
    QuicknessSkipped,
    /// `magic.md` runtime tag `T`: a self-acting slot whose turn the live
    /// Negate Time effect skipped outright. The party is unaffected - it
    /// is still prompted normally - because this gate sits inside the
    /// automatic actor driver, past the `PlayerReady` arm.
    NegateTimeSkipped,
    MonsterAi {
        ai_turn: Option<CombatAiTurnApplication>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatActorSlotDispatchApplication {
    EndOfRound {
        control: CombatRoundLoopControl,
    },
    Slot {
        slot: usize,
        phase_tick: Option<CombatActorPhaseTick>,
        action: CombatActorDispatchAction,
        control_after: CombatRoundLoopControl,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatRoundWalkStopReason {
    AwaitingPlayer,
    /// A graphical frontend requested one visible automatic action at a time.
    AutomaticAction,
    Exit,
    EndOfRound,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatRoundWalkApplication {
    pub start_slot: usize,
    pub next_slot: usize,
    pub stop_reason: CombatRoundWalkStopReason,
    pub applications: Vec<CombatActorSlotDispatchApplication>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatRoundLoopExitApplication {
    pub exit: CombatRoundLoopExit,
    pub result_code: u8,
    pub restored_snapshot: bool,
}

/// `combat.md §6.3` "Both rolls use the same helper": the first ordinary
/// death-drop roll takes the accepted branch when it is **less than or
/// equal to** the class drop-cap byte. The second roll uses the strict
/// [`combat_default_death_drop_gate_accepts`] form instead. Since the
/// shared `1..30` helper never returns zero, a zero drop cap can never
/// take the accepted branch under either gate.
pub const fn combat_default_death_drop_gate_accepts_inclusive(
    drop_cap: u8,
    roll_1_to_30: u8,
) -> bool {
    roll_1_to_30 <= drop_cap
}

/// `combat.md §5` monster placement: a placed monster's base-step is the
/// class speed seed randomised by a uniform `[-4, +3]` adjustment,
/// reverted to the unadjusted seed whenever the adjusted value would
/// exceed thirty. The published rule only names the upper revert; an
/// adjustment that would drive the value below zero is reverted the same
/// way rather than wrapping.
pub const fn combat_placement_base_step(speed_seed: u8, adjust_roll_0_to_7: u8) -> u8 {
    let adjusted = speed_seed as i16 + (adjust_roll_0_to_7 % 8) as i16 - 4;
    if adjusted > COMBAT_PLACEMENT_BASE_STEP_MAX as i16 || adjusted < 0 {
        speed_seed
    } else {
        adjusted as u8
    }
}

/// `combat.md §5`: the placement base-step ceiling above which the
/// `[-4, +3]` adjustment is reverted.
pub const COMBAT_PLACEMENT_BASE_STEP_MAX: u8 = 30;

/// `combat.md §5`: the inclusive bounds of the one speed-variation draw
/// each ordinary monster placement consumes. Eight outcomes spanning the
/// uniform `[-4, +3]` adjustment.
pub const COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_LOW: u8 = 0;
pub const COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_HIGH: u8 = 7;

/// The speed-variation roll that leaves the class speed seed unchanged
/// (`4 - 4 == 0`). Callers handed a short pre-rolled seed slice - the
/// deterministic gallery and conformance harnesses - fall back to this so
/// a missing seed means "no adjustment" rather than the `-4` end.
pub const COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_NEUTRAL: u8 = 4;

/// `combat.md §5`: a placed monster's phase counter is thirty-six minus
/// its base-step.
pub const COMBAT_PLACEMENT_PHASE_BASE: u8 = 36;

/// `combat.md §7` per-actor body step 5: "The counter is reset to
/// `36 - base_step`." The round walker's refresh constant is the same
/// thirty-six the placement seed in `§5` uses, so both spell it with one
/// value. Every production round-walk call site must pass this constant
/// rather than a bare literal - a smaller constant makes every class
/// whose base-step reaches it refresh to zero and act on every pass.
pub const COMBAT_PHASE_REFRESH_CONSTANT: u8 = COMBAT_PLACEMENT_PHASE_BASE;

/// How many consecutive table walks a driver runs while looking for the
/// next actor that is ready to act. `combat.md §7` restarts the table walk
/// whenever slot 32 is reached with actors still present, so this is only
/// a non-termination guard - but it has to be able to cross the largest
/// phase counter a descriptor can hold, which is
/// [`COMBAT_PLACEMENT_PHASE_BASE`] at a base-step of zero. A bound below
/// that silently drops a slow party member's turn.
pub const COMBAT_ROUND_WALK_DRAIN_LIMIT: usize = COMBAT_PLACEMENT_PHASE_BASE as usize + 1;

/// `combat.md §5` monster placement seeding, shared by every placement
/// site (terrain, dungeon-room, sleep ambush, and the `§6.3` Gazer death
/// spawn). The descriptor takes "the class's maximum HP as its HP/wound
/// counter; a base-step of the class speed seed randomised by a uniform
/// `[-4, +3]` adjustment, reverted to the unadjusted seed whenever the
/// adjusted value would exceed thirty; a phase counter of thirty-six
/// minus the base-step". `speed_adjust_roll_0_to_7` is the one
/// speed-variation draw `§5` charges to each ordinary placement; the
/// caller owns the draw so the shared PRNG order stays visible at the
/// placement site.
pub fn combat_placement_descriptor(
    stats: CombatClassStats,
    active_object_slot: u8,
    x: u8,
    y: u8,
    flags: u8,
    speed_adjust_roll_0_to_7: u8,
) -> CombatActorDescriptor {
    let base_step = combat_placement_base_step(stats.speed_seed, speed_adjust_roll_0_to_7);
    let mut actor = CombatActorDescriptor::for_monster_placement(
        stats,
        active_object_slot,
        x,
        y,
        flags,
        COMBAT_PLACEMENT_PHASE_BASE.saturating_sub(base_step),
    );
    actor.base_step = base_step;
    actor
}

impl PlayState {
    pub fn set_combat_actor_status_disabled(&mut self, slot: usize) -> bool {
        let Some(actor) = self.combat_actors.get_mut(slot) else {
            return false;
        };
        actor.set_status_disabled();
        self.mark_visibility_dirty();
        true
    }

    pub fn combat_sleep_wake_roll(&mut self, slot: usize) -> u8 {
        let _ = slot;
        self.random_range_u8(COMBAT_SLEEP_WAKE_ROLL_LOW, COMBAT_SLEEP_WAKE_ROLL_HIGH)
    }

    pub fn apply_combat_sleep_wake_dispatch(
        &mut self,
        slot: usize,
        roll: u8,
    ) -> Option<CombatSleepWakeApplication> {
        let actor = self.combat_actors.get_mut(slot)?;
        if !actor.is_status_disabled() {
            return None;
        }
        let woke = roll == COMBAT_SLEEP_WAKE_SUCCESS_ROLL;
        if woke {
            actor.clear_status_disabled();
            let hidden = actor.is_hidden_or_unrevealed();
            let active_object_slot = usize::from(actor.active_object_slot);
            // `combat.md §5`: the sleeping character is named by the
            // descriptor's owner/target/class byte, not by the
            // descriptor index.
            if let Some(member) = self
                .combat_roster_slot_for_actor_slot(slot)
                .and_then(|roster_slot| self.party.get_mut(roster_slot))
            {
                if member.status == b'S' {
                    member.status = b'G';
                }
            }
            if let Some(object) = self.active_objects.get_mut(active_object_slot) {
                object.tile = if hidden {
                    COMBAT_POTION_INVISIBLE_WAKE_DISPLAY_TILE
                } else {
                    object.type_byte
                };
            }
            self.mark_visibility_dirty();
        }
        Some(CombatSleepWakeApplication { slot, roll, woke })
    }

    /// `combat.md §5` / `active-objects.md §7`: a seated party member's
    /// descriptor carries "the character's roster slot index" in its
    /// owner/target/class field, and a dead member "is skipped entirely",
    /// so "the remaining members therefore pack into the low descriptor
    /// indexes rather than keeping their roster index". A combat
    /// descriptor index is therefore **not** a roster index, and every
    /// roster-side read reached from a descriptor slot has to go through
    /// this field - the same route
    /// [`Self::apply_combat_magic_ring_pass_to_slot`] already takes.
    ///
    /// `§5` also fixes the free test: "Descriptor slots are considered
    /// free when their flags byte is zero." An index with no seated
    /// descriptor has no roster field to read, so it stands for itself
    /// rather than resolving to roster slot zero.
    pub(crate) fn combat_roster_slot_for_actor_slot(&self, slot: usize) -> Option<usize> {
        if slot >= COMBAT_PARTY_ACTOR_SLOTS {
            return None;
        }
        let roster_slot = match self.combat_actors.get(slot).copied() {
            Some(actor) if actor.flags != 0 => usize::from(actor.owner_target_class),
            _ => slot,
        };
        (roster_slot < COMBAT_PARTY_ACTOR_SLOTS).then_some(roster_slot)
    }

    pub(crate) fn combat_party_name_for_slot(&self, slot: usize) -> Option<&[u8]> {
        let roster_slot = self.combat_roster_slot_for_actor_slot(slot)?;
        self.party_names
            .get(roster_slot)
            .map(|name| name.as_slice())
    }

    /// `combat.md §8.1`, the turn banner. "The actor-and-weapons line is
    /// **not** Attack's announcement. It is the **turn banner**, emitted at
    /// the start of every keyboard-driven combatant's turn, *before any key
    /// is read*: a newline, the actor's name, and - for a party-side actor -
    /// the clause `, armed with ` followed by the names of that actor's
    /// readied items separated by `, `, or `bare hands` when none qualifies,
    /// terminated by a colon."
    ///
    /// "A charmed monster acting under player control gets only its name and
    /// the colon, with no armament clause", which is the non-party arm here.
    ///
    /// "Because the banner precedes the keystroke, it appears identically
    /// whether the player then presses `A`, a direction key, `Space`, or
    /// anything else." What `A` adds on top is `Attack-` and `Aim! `
    /// (`§8.2`), which stay with the Attack branch.
    pub fn combat_turn_banner_for_actor(&self, actor_slot: usize) -> Option<String> {
        if actor_slot < COMBAT_PARTY_ACTOR_SLOTS {
            let roster_slot = self.combat_roster_slot_for_actor_slot(actor_slot)?;
            let name = self
                .combat_party_name_for_slot(actor_slot)
                .and_then(party_name_to_string)?;
            let equipment = self.party_equipment.get(roster_slot).copied();
            return Some(combat_turn_banner(&name, equipment.as_ref()));
        }
        let class = self.combat_actors.get(actor_slot)?.owner_target_class;
        let name = combat_class_stats(class)?.name;
        Some(combat_turn_banner(name, None))
    }

    /// Open a keyboard-driven combatant's turn. `combat.md §8.1` prints the
    /// turn banner here, "before any key is read", so it is appended to the
    /// live transcript when the round walker hands control over rather than
    /// when the keystroke arrives.
    ///
    /// A free re-prompt after a refusal "uses the short form and does **not**
    /// reprint the banner" because the re-prompt branch reinstates
    /// `pending_combat_actor_slot` directly without calling this helper, so
    /// no second banner is ever emitted for the same turn.
    pub(crate) fn open_pending_combat_player_turn(&mut self, slot: Option<usize>) {
        self.pending_combat_actor_slot = slot;
        self.select_active_player_for_pending_combat_actor();
        let banner = slot.and_then(|slot| self.combat_turn_banner_for_actor(slot));
        if let Some(banner) = banner.as_deref() {
            self.message.push_str(banner);
        }
    }

    pub(crate) fn combat_target_group_for_slot(&self, slot: usize) -> u8 {
        self.combat_actors
            .get(slot)
            .copied()
            .map(|actor| {
                resolve_combat_target_group_for_actor(
                    actor,
                    slot,
                    self.combat_party_name_for_slot(slot),
                )
            })
            .unwrap_or(COMBAT_TARGET_GROUP_NEUTRAL)
    }

    /// `magic.md §7`: revalidate a victim's save-backed source at the instant
    /// C-Cast is dispatched. Stale, friendly, hidden, sleeping, dead, distant,
    /// and Negate-Time-suppressed references all fall through to the spell UI.
    pub(crate) fn combat_cast_interference_source_for_slot(
        &self,
        caster_slot: usize,
    ) -> Option<usize> {
        let source_slot = usize::from(*self.combat_interference_sources.get(caster_slot)?);
        let caster = self.combat_actors.get(caster_slot).copied()?;
        let source = self.combat_actors.get(source_slot).copied();
        let source_is_hostile = source.is_some()
            && combat_target_groups_are_hostile(
                self.combat_target_group_for_slot(source_slot),
                self.combat_target_group_for_slot(caster_slot),
            );
        let negate_time_active = active_effect_is_active(
            self.active_effect_tag,
            self.active_effect_counter,
            NEGATE_TIME_ACTIVE_EFFECT_TAG,
        );

        matches!(
            resolve_combat_cast_interference(
                caster,
                source,
                source_is_hostile,
                negate_time_active,
            ),
            CombatCastInterferenceOutcome::Interfered
        )
        .then_some(source_slot)
    }

    pub(crate) fn clear_combat_interference_for_completed_action(&mut self, victim_slot: usize) {
        if let Some(source) = self.combat_interference_sources.get_mut(victim_slot) {
            *source = COMBAT_INTERFERENCE_NO_SOURCE;
        }
    }

    pub(crate) fn combat_target_candidate_view(
        &self,
        descriptor: CombatActorDescriptor,
        slot: usize,
        suppressed: bool,
        invisible_or_unrevealed: bool,
    ) -> CombatTargetCandidateView {
        combat_target_candidate_view_from_descriptor(
            descriptor,
            slot,
            self.combat_party_name_for_slot(slot),
            suppressed,
            invisible_or_unrevealed,
        )
    }

    fn combat_suppression_filter_bypassed_for_class(&self, class: u8) -> bool {
        class == COMBAT_CLASS_SHADOW_LORD
            || matches!(
                self.combat_frame_snapshot.as_ref().map(|snapshot| snapshot.area),
                Some(Area::Dungeon { scene, .. }) if scene.record == DOOM_DUNGEON_RECORD
            )
    }

    fn combat_ai_morale_roll(&mut self, actor_slot: usize) -> u8 {
        let _ = actor_slot;
        self.random_range_u8(0, u8::MAX)
    }

    fn combat_ai_actor_fleeing(&mut self, actor_slot: usize) -> bool {
        let Some(actor) = self.combat_actors.get(actor_slot).copied() else {
            return false;
        };
        if actor_slot < COMBAT_PARTY_ACTOR_SLOTS || !combat_actor_is_active_not_dead(actor) {
            return false;
        }
        let Some(stats) = combat_class_stats(actor.owner_target_class) else {
            return actor.is_fleeing();
        };
        let bucket = combat_wound_score_bucket(actor.hp_or_wound, stats.max_hp);
        let morale_roll = if matches!(bucket, CombatWoundScoreBucket::OneQuarterToUnderHalf) {
            self.combat_ai_morale_roll(actor_slot)
        } else {
            0
        };
        let morale = resolve_combat_wound_morale(actor.hp_or_wound, stats.max_hp, morale_roll);
        self.combat_actors[actor_slot].set_fleeing(morale.fleeing);
        morale.fleeing
    }

    /// `combat.md §7` step 3, the restraint skip: "Read the arena terrain
    /// under the actor and compare it against exactly two tile ids - the
    /// stocks `0x84` and the manacles `0x85`. On a match, skip the slot
    /// entirely. [...] No other terrain participates: water, swamp,
    /// mountains, walls and force fields are all outside the test."
    ///
    /// The same section withdraws the earlier reading this site used to
    /// implement: "Earlier revisions called this step 'skip wall-cell
    /// slots, a defensive guard against bad placement'. That is withdrawn
    /// - it is a restraint guard, and reading it as a walkability guard
    /// freezes actors the original leaves acting." `§7.1` spells out the
    /// case that matters: a land class fought over water is placed by
    /// `§5.4` onto one of arena 15's sixteen authored water cells and
    /// "takes its turn every round on schedule; only its *movement* is
    /// constrained".
    ///
    /// The tile ids are the same two [`JIMMY_STOCKS_TILE`] and
    /// [`JIMMY_MANACLES_TILE`] the `J` Jimmy release path tests, which is
    /// what makes the freeze recoverable (`§7.1`).
    fn combat_actor_stands_on_restraint_arena_cell(&self, actor: CombatActorDescriptor) -> bool {
        let x = actor.x as usize;
        let y = actor.y as usize;
        y < COMBAT_ARENA_SIDE
            && x < COMBAT_ARENA_SIDE
            && crate::jimmy_restraint_tile(self.combat_terrain[y][x])
    }

    /// `catalogs/spell-list.md §4`: the active scene byte for the cast
    /// dispatcher's scene gate.
    ///
    /// `PlayState` has no stored scene byte during combat — it keeps `area`
    /// pointed at the map the fight started from and raises the
    /// `combat_active` flag — so combat is converted back to the published
    /// combat-class byte [`SCENE_COMBAT_TEMPORARY`] (`0xFF`) here. Both
    /// world planes report the single published overworld byte `0`; the
    /// catalog's classification bands do not split Britannia from the
    /// Underworld.
    pub fn current_scene_byte(&self) -> u8 {
        if self.combat_active {
            return SCENE_COMBAT_TEMPORARY;
        }
        match self.area {
            Area::World { .. } => SCENE_OVERWORLD,
            Area::Town { scene, .. } => scene.byte,
            Area::Dungeon { scene, .. } => scene.byte,
        }
    }

    /// `magic.md §9` scene class the cast dispatcher's scene gate tests
    /// against the per-spell allow mask.
    pub fn current_spell_scene_class(&self) -> SpellSceneClass {
        spell_scene_class_for_scene_byte(self.current_scene_byte())
    }

    /// `magic.md §7` gate 5 / `magic.md §9`: does the spell's published
    /// allow mask carry the active scene's bit? The dispatcher rejects with
    /// `Not here!` when it does not.
    pub fn spell_allowed_in_current_cast_context(&self, spell_index: usize) -> bool {
        if spell_index >= SPELL_COUNT {
            return false;
        }
        spell_allowed_in_scene(
            SPELL_SCENE_MASKS[spell_index],
            self.current_spell_scene_class(),
        )
    }

    pub fn combat_arena_field_placement_callback_accepts(
        &mut self,
        caster_index: usize,
        target_slot: usize,
        spell_index: usize,
    ) -> bool {
        let _ = (caster_index, target_slot, spell_index);
        spell_combat_field_kind(spell_index)
            .and_then(CombatArenaFieldKind::from_kind_byte)
            .is_some()
    }

    pub fn combat_spell_damage_roll_for_kind(&mut self, kind: CombatSpellDamageKind) -> u8 {
        let max = match kind {
            CombatSpellDamageKind::MagicMissile => COMBAT_MAGIC_MISSILE_DAMAGE_ROLL_MAX,
            CombatSpellDamageKind::Fireball => COMBAT_FIREBALL_DAMAGE_ROLL_MAX,
            CombatSpellDamageKind::Tremor => COMBAT_TREMOR_DAMAGE_ROLL_MAX,
            CombatSpellDamageKind::FlameWind => COMBAT_FLAME_WIND_DAMAGE_ROLL_MAX,
            CombatSpellDamageKind::Kill | CombatSpellDamageKind::DeathWind => 0,
        };
        self.random_mod_u8(max)
    }

    /// `combat.md §9.1`: party actors use their owner character's persisted
    /// Intelligence; monster actors use their class endurance byte.
    pub fn combat_actor_resistance_rating(&self, slot: usize) -> Option<u8> {
        let actor = self.combat_actors.get(slot).copied()?;
        if slot < COMBAT_PARTY_ACTOR_SLOTS {
            self.party_intelligence
                .get(actor.owner_target_class as usize)
                .copied()
        } else {
            combat_class_stats(actor.owner_target_class).map(|stats| stats.endurance)
        }
    }

    pub fn combat_resistance_raw_roll(&mut self) -> u8 {
        self.random_range_u8(0, 60)
    }

    pub fn combat_resistance_blocks(&mut self, caster_slot: usize, target_slot: usize) -> bool {
        let ratings = self
            .combat_actor_resistance_rating(caster_slot)
            .zip(self.combat_actor_resistance_rating(target_slot));
        let raw_roll = self.combat_resistance_raw_roll();
        ratings.is_none_or(|(caster, target)| {
            combat_resistance_blocks_from_raw_roll(caster, target, raw_roll)
        })
    }

    pub fn combat_target_weight(&self, target_slot: usize) -> Option<u8> {
        let actor = self.combat_actors.get(target_slot).copied()?;
        let negate_time_active = active_effect_is_active(
            self.active_effect_tag,
            self.active_effect_counter,
            NEGATE_TIME_ACTIVE_EFFECT_TAG,
        );
        Some(combat_actor_weight(target_slot, actor, negate_time_active))
    }

    pub fn combat_target_weight_gate_accepts(&mut self, target_slot: usize) -> bool {
        let weight = self.combat_target_weight(target_slot);
        let raw_roll = self.combat_resistance_raw_roll();
        weight
            .is_some_and(|weight| combat_target_weight_gate_accepts_from_raw_roll(weight, raw_roll))
    }

    pub fn combat_spell_target_defense_value(&self, target_slot: usize) -> u8 {
        if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            // `magic.md §8`: Protection occupies and displays the shared
            // timed-effect slot, but its intended defense computation is
            // unreachable and has no mechanical consequence.
            CHARACTER_DEFENSE_FACTORY_SEED
        } else {
            self.combat_actors
                .get(target_slot)
                .and_then(|actor| combat_class_stats(actor.owner_target_class))
                .map(|stats| stats.defense)
                .unwrap_or_default()
        }
    }

    pub fn combat_spell_target_defense_roll(&mut self, target_slot: usize) -> u8 {
        self.random_range_u8(0, self.combat_spell_target_defense_value(target_slot))
    }

    pub fn combat_arena_field_poison_damage_roll(&mut self) -> u8 {
        self.random_mod_u8(20)
    }

    pub fn combat_arena_field_fire_damage_roll(&mut self) -> u8 {
        self.random_mod_u8(21)
    }

    pub fn combat_arena_field_defense_roll(&mut self, target_slot: usize) -> u8 {
        self.combat_spell_target_defense_roll(target_slot)
    }

    pub fn apply_combat_arena_field_placement(
        &mut self,
        field: CombatArenaFieldKind,
        target_x: u8,
        target_y: u8,
        callback_accepts: bool,
    ) -> Option<CombatArenaFieldPlacementApplication> {
        let target_slot = find_combat_actor_at_field_coordinate(
            &self.combat_actors,
            &self.active_objects,
            target_x,
            target_y,
        );
        if !resolve_combat_field_placement_acceptance(field, callback_accepts) {
            return None;
        }

        let active_object_slot = self
            .active_objects
            .iter()
            .position(|object| object.is_empty())?;
        let kind_byte = field.kind_byte();
        self.active_objects[active_object_slot] = ActiveObject {
            type_byte: kind_byte,
            tile: kind_byte,
            x: usize::from(target_x),
            y: usize::from(target_y),
            z: target_slot
                .and_then(|slot| self.combat_actors.get(slot))
                .and_then(|actor| self.active_objects.get(actor.active_object_slot as usize))
                .map(|object| object.z)
                .unwrap_or_default(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        self.mark_visibility_dirty();

        Some(CombatArenaFieldPlacementApplication {
            field,
            target_slot,
            active_object_slot,
            x: target_x,
            y: target_y,
        })
    }

    pub fn combat_field_cursor_cell_in_range(
        &self,
        caster_index: usize,
        target_x: u8,
        target_y: u8,
    ) -> bool {
        let Some(caster) = self.combat_actors.get(caster_index).copied() else {
            return false;
        };
        combat_actor_is_active_not_dead(caster)
            && usize::from(target_x) < COMBAT_ARENA_SIDE
            && usize::from(target_y) < COMBAT_ARENA_SIDE
            && combat_arena_range(caster.x, caster.y, target_x, target_y)
                <= COMBAT_FIELD_CURSOR_RANGE
    }

    pub fn combat_field_cursor_start(&self, caster_index: usize) -> Option<(u8, u8)> {
        let caster = self.combat_actors.get(caster_index).copied()?;
        if !combat_actor_is_active_not_dead(caster) {
            return None;
        }
        if let Some((x, y)) = self.combat_secondary_marker {
            if self.combat_field_cursor_cell_in_range(caster_index, x, y) {
                return Some((x, y));
            }
        }
        (usize::from(caster.x) < COMBAT_ARENA_SIDE && usize::from(caster.y) < COMBAT_ARENA_SIDE)
            .then_some((caster.x, caster.y))
    }

    pub fn resolve_combat_arena_field_impact(
        &self,
        caster_index: usize,
        target: Option<(u8, u8)>,
    ) -> Option<(u8, u8)> {
        let (target_x, target_y) = target?;
        self.combat_field_cursor_cell_in_range(caster_index, target_x, target_y)
            .then_some((target_x, target_y))
    }

    pub fn cast_combat_arena_field_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        field: CombatArenaFieldKind,
        target: Option<(u8, u8)>,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(caster_actor) = self.combat_actors.get(caster_index).copied() else {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        };
        if caster_actor.is_empty() || caster_actor.is_marked_dead() {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        }

        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        self.confirm_spent_combat_arena_field_spell(caster_index, spell_index, field, target)
    }

    pub fn confirm_spent_combat_arena_field_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        field: CombatArenaFieldKind,
        target: Option<(u8, u8)>,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(caster_actor) = self.combat_actors.get(caster_index).copied() else {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        };
        if caster_actor.is_empty() || caster_actor.is_marked_dead() {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        }

        let Some((target_x, target_y)) =
            self.resolve_combat_arena_field_impact(caster_index, target)
        else {
            self.message = "Target? Use C1FGI4,3/C1GIN4,3/C1GIZ4,3/C1GIS4,3.".to_string();
            return MoveOutcome::Blocked;
        };
        let target_slot = find_combat_actor_at_field_coordinate(
            &self.combat_actors,
            &self.active_objects,
            target_x,
            target_y,
        );
        // `audio.md §6.1`: the combat arm of the four field spells is one of
        // the combat effect template's users - "The combat arm plays the combat
        // template instead" - so it plays the circle-scaled rumble lead alone,
        // never the dungeon arm's shared variant. `§11` lists "the combat arm
        // of the four field spells" in the shared sequence's not-produced-by
        // column.
        self.emit_sound_effect(SoundEffect::CircleRumbleLead {
            circle: audio::spell_circle(spell_index),
        });

        let callback_accepts = target_slot
            .map(|slot| {
                self.combat_arena_field_placement_callback_accepts(caster_index, slot, spell_index)
            })
            .unwrap_or(true);
        let applied =
            self.apply_combat_arena_field_placement(field, target_x, target_y, callback_accepts);

        self.advance_turn();
        self.message = if applied.is_some() {
            format!("{} field placed.", field.label())
        } else {
            "Failed!".to_string()
        };
        if applied.is_some() {
            // `audio.md §6.1`: "On a resolved effect it adds a **descending**
            // glissando, 20 updates from 1300 Hz down toward 350 Hz."
            self.emit_sound_effect(SoundEffect::CombatTemplateImpact);
        }
        if applied.is_none() {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
        }
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn find_combat_arena_field_marker(
        &self,
        target_x: u8,
        target_y: u8,
    ) -> Option<(usize, CombatArenaFieldKind)> {
        self.find_combat_arena_field_marker_excluding(target_x, target_y, None)
    }

    pub fn find_combat_arena_field_marker_excluding(
        &self,
        target_x: u8,
        target_y: u8,
        excluded_active_object_slot: Option<usize>,
    ) -> Option<(usize, CombatArenaFieldKind)> {
        self.active_objects
            .iter()
            .take(OOL_SLOTS)
            .enumerate()
            .find_map(|(slot, object)| {
                if excluded_active_object_slot == Some(slot) {
                    return None;
                }
                if object.x != usize::from(target_x) || object.y != usize::from(target_y) {
                    return None;
                }
                CombatArenaFieldKind::from_kind_byte(object.type_byte).map(|field| (slot, field))
            })
    }

    pub fn apply_combat_absorbable_field_contact_for_actor_position(
        &mut self,
        actor_slot: usize,
    ) -> Option<CombatAbsorbableFieldApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_present_not_dead(actor) || actor.y != 2 {
            return None;
        }
        let companion_band_index = terrain_band_active_index(1, usize::from(actor.x))?;
        let marker_byte = self
            .combat_render_actor_byte_at(usize::from(actor.x), 1)
            .unwrap_or(self.combat_terrain[1][usize::from(actor.x)]);
        self.terrain_band[companion_band_index] = marker_byte;
        if !dungeon_room_absorbable_field_family(marker_byte) {
            return None;
        }
        self.active_player = None;
        let armed_endgame_result = self.combat_frame_snapshot.as_mut().is_some_and(|snapshot| {
            let armed = snapshot.endgame_messages.is_some();
            if armed {
                snapshot.enter_endgame_after_successful_combat = true;
            }
            armed
        });
        self.message = "Absorbed!".to_string();
        self.mark_visibility_dirty();
        Some(CombatAbsorbableFieldApplication {
            actor_slot,
            companion_band_index,
            marker_byte,
            x: actor.x,
            y: actor.y,
            armed_endgame_result,
        })
    }

    pub fn apply_combat_arena_field_removal(
        &mut self,
        target_x: u8,
        target_y: u8,
    ) -> Option<CombatArenaFieldRemovalApplication> {
        let (active_object_slot, field) =
            self.find_combat_arena_field_marker(target_x, target_y)?;
        self.free_active_object_slot(active_object_slot);
        self.mark_visibility_dirty();
        Some(CombatArenaFieldRemovalApplication {
            field,
            active_object_slot,
            x: target_x,
            y: target_y,
        })
    }

    pub fn cast_combat_dispel_field(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
    ) -> MoveOutcome {
        if !self.combat_active
            || !self.spell_allowed_in_current_cast_context(DISPEL_FIELD_SPELL_INDEX)
        {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(direction) = direction else {
            self.message = "Direction? Use C1AG6.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Dispel Field requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(caster_actor) = self.combat_actors.get(caster_index).copied() else {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        };
        if caster_actor.is_empty() || caster_actor.is_marked_dead() {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, DISPEL_FIELD_SPELL_INDEX, DISPEL_FIELD_COST)
        {
            return outcome;
        }

        // `audio.md §6`: Dispel Field shares variant 4. `audio.md §8.3`:
        // confirmation plays the spell effect before the coordinate resolver.
        self.emit_sound_effect(SoundEffect::SharedVariant { variant: 4 });

        let (dx, dy) = direction.delta();
        let target_x = caster_actor.x as isize + dx;
        let target_y = caster_actor.y as isize + dy;
        let applied = if (0..COMBAT_ARENA_SIDE as isize).contains(&target_x)
            && (0..COMBAT_ARENA_SIDE as isize).contains(&target_y)
        {
            self.apply_combat_arena_field_removal(target_x as u8, target_y as u8)
        } else {
            None
        };

        self.advance_turn();
        self.message = if let Some(application) = applied {
            format!("Dispelled {} field.", application.field.label())
        } else {
            "Failed!".to_string()
        };
        if applied.is_none() {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
        }
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn apply_combat_polymorph_giant_rat(
        &mut self,
        target_slot: usize,
    ) -> Option<CombatPolymorphApplication> {
        let actor_before = self.combat_actors.get(target_slot).copied()?;
        let actor_after = resolve_polymorph_giant_rat_descriptor(actor_before)?;
        let active_object_slot = usize::from(actor_after.active_object_slot);
        let object_before = self.active_objects.get(active_object_slot).copied()?;
        let object_after =
            polymorph_giant_rat_active_object(object_before, actor_after.x, actor_after.y);

        self.combat_actors[target_slot] = actor_after;
        self.active_objects[active_object_slot] = object_after;
        self.mark_visibility_dirty();

        Some(CombatPolymorphApplication {
            target_slot,
            active_object_slot,
            actor_before,
            actor_after,
            object_before,
            object_after,
        })
    }

    pub fn cast_combat_polymorph_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        target_slot: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let target_actor = self
            .combat_actors
            .get(target_slot)
            .copied()
            .unwrap_or_default();
        let caster_group = self.combat_target_group_for_slot(caster_index);
        let target_group = self.combat_target_group_for_slot(target_slot);
        if !creature_prompt_target_is_eligible(target_actor, target_group, caster_group, false) {
            self.message = "Target? Use C1BRX7 to target a hostile creature.".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6`: Polymorph shares variant 6. `audio.md §8.3`:
        // confirmation plays the spell effect before the target resolver.
        self.emit_sound_effect(SoundEffect::SharedVariant { variant: 6 });

        let applied = self.apply_combat_polymorph_giant_rat(target_slot);
        self.advance_turn();
        self.message = if applied.is_some() {
            "Polymorph!".to_string()
        } else {
            "Failed!".to_string()
        };
        if applied.is_none() {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
        }
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn apply_combat_charm_allegiance(
        &mut self,
        target_slot: usize,
    ) -> Option<CombatCharmApplication> {
        let (flags_before, flags_after) =
            toggle_combat_charm_allegiance(self.combat_actors.get_mut(target_slot)?)?;

        // `combat.md §6.1a` Writers #2 + `catalogs/spell-list.md` id 34: when
        // the accepted target is a party-side slot, Charm also writes the Good
        // status letter into that character's roster status byte and refreshes
        // the stats panel — in both toggle directions, so Charm on a Sleeping
        // or Poisoned party member restores the letter to Good as a side
        // effect. `§12` is explicit that the byte written is `'G'`, never
        // `'C'`: the panel's in-combat `C` is the separate presentation
        // override driven by the descriptor bit.
        if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            if let Some(member) = self.party.get_mut(target_slot) {
                member.status = CharacterStatus::Good.save_byte();
            }
            self.mark_visibility_dirty();
        }

        Some(CombatCharmApplication {
            target_slot,
            flags_before,
            flags_after,
        })
    }

    /// `combat.md §6.1a` + `magic.md §8` creature-prompt targeters: Charm
    /// toggles the controlled/charmed bit, and "a second successful Charm on
    /// the same actor clears it". The shared creature-prompt predicate
    /// rejects any actor already carrying the bit, which would make the
    /// clearing half unreachable, so Charm keeps every other eligibility test
    /// and re-targets an already-marked actor regardless of which group its
    /// toggled descriptor now falls in.
    pub fn charm_prompt_target_is_eligible(&self, target_slot: usize, caster_group: u8) -> bool {
        let Some(actor) = self.combat_actors.get(target_slot).copied() else {
            return false;
        };
        if actor.is_empty()
            || actor.is_marked_dead()
            || actor.is_hidden_or_unrevealed()
            || actor.is_status_disabled()
        {
            return false;
        }
        actor.is_controlled() || self.combat_target_group_for_slot(target_slot) != caster_group
    }

    /// `catalogs/spell-list.md` id 34: Charm names its victim — a party-side
    /// slot by roster name, a monster by class name.
    pub fn combat_charm_target_display_name(&self, target_slot: usize) -> Option<String> {
        if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            return self
                .combat_party_name_for_slot(target_slot)
                .and_then(party_name_to_string);
        }
        self.combat_actors
            .get(target_slot)
            .and_then(|actor| combat_class_stats(actor.owner_target_class))
            .map(|stats| stats.name.to_string())
    }

    pub fn cast_combat_charm_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        target_slot: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let caster_group = self.combat_target_group_for_slot(caster_index);
        if !self.charm_prompt_target_is_eligible(target_slot, caster_group) {
            self.message = "Target? Use C1AEX7 to target a hostile creature.".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6`: Charm shares variant 6. `audio.md §8.3`: confirmation
        // plays the spell effect before the target resolver, so a resisted cast
        // keeps the pre-effect and adds the failure tail.
        self.emit_sound_effect(SoundEffect::SharedVariant { variant: 6 });

        // `catalogs/spell-list.md` id 34: the victim's name has to be read
        // before the toggle, because a party-side target's roster status write
        // happens inside the application.
        let target_name = self.combat_charm_target_display_name(target_slot);
        let applied = if self.combat_resistance_blocks(caster_index, target_slot) {
            None
        } else {
            self.apply_combat_charm_allegiance(target_slot)
        };
        self.advance_turn();
        if applied.is_some() {
            // `combat.md §6.1a` Writers #2 + `catalogs/spell-list.md` id 34:
            // Charm prints its own charmed line and suppresses the
            // dispatcher's success/failure epilogue, so the generic `Charm!`
            // never appears.
            self.message = match target_name {
                Some(name) => format!("{name} charmed!"),
                None => "Charmed!".to_string(),
            };
            MoveOutcome::Cast
        } else {
            self.message = "Failed!".to_string();
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
            MoveOutcome::Blocked
        }
    }

    pub fn apply_combat_clone_to_coordinate(
        &mut self,
        target_slot: usize,
        x: u8,
        y: u8,
    ) -> Option<CombatCloneApplication> {
        let target_actor = self.combat_actors.get(target_slot).copied()?;
        if !combat_actor_is_active_not_dead(target_actor) {
            return None;
        }
        let target_object = self
            .active_objects
            .get(usize::from(target_actor.active_object_slot))
            .copied()?;
        if target_object.is_empty() {
            return None;
        }
        let allocation = resolve_clone_spell_allocation(&self.combat_actors, &self.active_objects)?;
        let actor =
            clone_combat_actor_descriptor(target_actor, allocation.active_object_slot as u8, x, y);
        let active_object =
            clone_active_object_record(target_object, usize::from(x), usize::from(y));

        self.combat_actors[allocation.actor_slot] = actor;
        self.active_objects[allocation.active_object_slot] = active_object;
        self.mark_visibility_dirty();

        Some(CombatCloneApplication {
            target_slot,
            actor_slot: allocation.actor_slot,
            active_object_slot: allocation.active_object_slot,
            x,
            y,
            actor,
            active_object,
        })
    }

    pub fn apply_combat_clone_with_legal_mask(
        &mut self,
        target_slot: usize,
        legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        candidate_coordinates: &[(u8, u8)],
    ) -> Option<CombatCloneApplication> {
        let (x, y) = resolve_combat_clone_placement_coordinate(legal_cells, candidate_coordinates)?;
        self.apply_combat_clone_to_coordinate(target_slot, x, y)
    }

    pub fn combat_arena_placement_seed(&mut self) -> u8 {
        self.random_range_u8(0, (COMBAT_ARENA_SIDE * COMBAT_ARENA_SIDE - 1) as u8)
    }

    pub fn combat_neighbor_placement_seed(&mut self) -> u8 {
        self.random_mod_u8(8)
    }

    pub fn combat_conjure_class_selector(&mut self) -> u8 {
        self.random_range_u8(0, CONJURE_ANIMAL_OUTCOME_COUNT - 1)
    }

    pub fn combat_actor_z(&self, slot: usize) -> i8 {
        self.combat_actors
            .get(slot)
            .and_then(|actor| self.active_objects.get(actor.active_object_slot as usize))
            .map(|object| object.z)
            .unwrap_or_default()
    }

    /// `combat.md §6.3` "Both rolls use the same helper": each ordinary
    /// death-drop gate draws the shared near-uniform `1..30` value — a
    /// uniform `0..60` raw draw halved with truncation, with zero promoted
    /// to one — not a percentage roll.
    pub fn combat_default_death_drop_rolls(&mut self) -> (u8, u8) {
        let first = combat_skewed_roll_1_to_30(self.random_range_u8(0, 60));
        let second = combat_skewed_roll_1_to_30(self.random_range_u8(0, 60));
        (first, second)
    }

    /// `combat.md §5`: "Each ordinary monster placement then consumes one
    /// speed-variation draw." One uniform `0..7` draw off the shared PRNG,
    /// fed to [`combat_placement_base_step`].
    pub fn combat_placement_speed_adjust_roll(&mut self) -> u8 {
        self.random_range_u8(
            COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_LOW,
            COMBAT_PLACEMENT_SPEED_ADJUST_ROLL_HIGH,
        )
    }

    /// `combat.md §5` monster placement in its ordinary place-a-monster
    /// mode: allocate a fresh descriptor and a fresh active-object record
    /// for `class` at the given arena cell and Z plane, seeded with the
    /// class maximum HP, a base-step of the class speed seed adjusted by a
    /// uniform `[-4, +3]`, a phase counter of thirty-six minus that
    /// base-step, the class id, and the caller's faction tag. Returns
    /// `None` — with no other side effect and no adjustment draw — when
    /// either table is full, which is the allocation failure `§6.3`
    /// requires the Gazer death spawn to tolerate.
    pub fn place_combat_monster_at_arena_cell(
        &mut self,
        class: u8,
        x: u8,
        y: u8,
        z: i8,
        actor_flags: u8,
    ) -> Option<CombatSummonApplication> {
        let stats = combat_class_stats(class)?;
        let active_object =
            summoned_active_object_record(class, usize::from(x), usize::from(y), z)?;
        let allocation = resolve_clone_spell_allocation(&self.combat_actors, &self.active_objects)?;
        let actor = combat_placement_descriptor(
            stats,
            allocation.active_object_slot as u8,
            x,
            y,
            actor_flags,
            self.combat_placement_speed_adjust_roll(),
        );

        self.combat_actors[allocation.actor_slot] = actor;
        self.active_objects[allocation.active_object_slot] = active_object;
        self.mark_visibility_dirty();

        Some(CombatSummonApplication {
            class,
            actor_slot: allocation.actor_slot,
            active_object_slot: allocation.active_object_slot,
            x,
            y,
            actor,
            active_object,
        })
    }

    pub fn apply_combat_summon_class_with_legal_mask(
        &mut self,
        class: u8,
        z: i8,
        actor_flags: u8,
        legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        candidate_coordinates: &[(u8, u8)],
    ) -> Option<CombatSummonApplication> {
        let (x, y) = resolve_combat_clone_placement_coordinate(legal_cells, candidate_coordinates)?;
        let allocation = resolve_clone_spell_allocation(&self.combat_actors, &self.active_objects)?;
        let actor = resolve_summoned_combat_actor_descriptor(
            class,
            allocation.active_object_slot as u8,
            x,
            y,
            actor_flags,
            0,
        )?;
        let active_object =
            summoned_active_object_record(class, usize::from(x), usize::from(y), z)?;

        self.combat_actors[allocation.actor_slot] = actor;
        self.active_objects[allocation.active_object_slot] = active_object;
        self.mark_visibility_dirty();

        Some(CombatSummonApplication {
            class,
            actor_slot: allocation.actor_slot,
            active_object_slot: allocation.active_object_slot,
            x,
            y,
            actor,
            active_object,
        })
    }

    pub fn apply_combat_conjure_class_with_random_attempts(
        &mut self,
        class: u8,
        z: i8,
        legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    ) -> Option<CombatSummonApplication> {
        for _ in 0..8 {
            let x = self.random_range_u8(0, 15);
            let y = self.random_range_u8(0, 15);
            if usize::from(x) >= COMBAT_ARENA_SIDE || usize::from(y) >= COMBAT_ARENA_SIDE {
                continue;
            }
            if let Some(application) = self.apply_combat_summon_class_with_legal_mask(
                class,
                z,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                legal_cells,
                &[(x, y)],
            ) {
                return Some(application);
            }
        }
        None
    }

    pub fn apply_combat_summon_daemon_with_random_attempts(
        &mut self,
        z: i8,
        legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    ) -> Option<CombatSummonApplication> {
        for _ in 0..8 {
            let x = self.random_range_u8(0, 15);
            let y = self.random_range_u8(0, 15);
            if usize::from(x) >= COMBAT_ARENA_SIDE || usize::from(y) >= COMBAT_ARENA_SIDE {
                continue;
            }
            if let Some(application) = self.apply_combat_summon_class_with_legal_mask(
                COMBAT_CLASS_DAEMON,
                z,
                COMBAT_ACTOR_FLAG_SELECTABLE_80,
                legal_cells,
                &[(x, y)],
            ) {
                return Some(application);
            }
        }
        None
    }

    pub fn apply_combat_summon_class_around_slot(
        &mut self,
        class: u8,
        center_slot: usize,
        seed: u8,
    ) -> Option<CombatSummonApplication> {
        let center = self.combat_actors.get(center_slot).copied()?;
        if !combat_actor_is_active_not_dead(center) {
            return None;
        }
        let legal_cells = self.combat_legal_cell_mask();
        let candidates = combat_neighbor_candidate_coordinates(center.x, center.y, seed);
        self.apply_combat_summon_class_with_legal_mask(
            class,
            self.combat_actor_z(center_slot),
            COMBAT_SUMMONED_ACTOR_FLAGS,
            &legal_cells,
            &candidates,
        )
    }

    pub fn apply_combat_summon_class_in_ring_around_slot(
        &mut self,
        class: u8,
        center_slot: usize,
    ) -> Option<CombatSummonApplication> {
        let center = self.combat_actors.get(center_slot).copied()?;
        if !combat_actor_is_active_not_dead(center) {
            return None;
        }
        let legal_cells = self.combat_legal_cell_mask();
        let candidates = combat_ring_candidate_coordinates(center.x, center.y);
        self.apply_combat_summon_class_with_legal_mask(
            class,
            self.combat_actor_z(center_slot),
            COMBAT_SUMMONED_ACTOR_FLAGS,
            &legal_cells,
            &candidates,
        )
    }

    pub fn apply_combat_summon_class_around_target_coordinate(
        &mut self,
        class: u8,
        z: i8,
        target_x: i16,
        target_y: i16,
    ) -> Option<CombatSummonApplication> {
        let legal_cells = self.combat_legal_cell_mask();
        let candidates = combat_ring_candidate_coordinates_around(target_x, target_y);
        self.apply_combat_summon_class_with_legal_mask(
            class,
            z,
            COMBAT_SUMMONED_ACTOR_FLAGS,
            &legal_cells,
            &candidates,
        )
    }

    pub fn apply_combat_ai_blink_special(
        &mut self,
        actor_slot: usize,
    ) -> Option<CombatAiSpecialApplication> {
        let visibility = toggle_combat_blink_phase(
            self.combat_actors.get_mut(actor_slot)?,
            &mut self.active_objects,
        )?;
        if visibility.changed() {
            self.mark_visibility_dirty();
        }
        self.message = match visibility.visibility {
            CombatLinkedVisibility::Hidden => "Monster vanishes.".to_string(),
            CombatLinkedVisibility::Visible => "Monster reappears.".to_string(),
        };
        Some(CombatAiSpecialApplication::Blink {
            actor_slot,
            visibility,
        })
    }

    pub fn apply_combat_ai_summon_daemon_special_with_candidates(
        &mut self,
        actor_slot: usize,
        candidate_coordinates: &[(u8, u8)],
    ) -> Option<CombatAiSpecialApplication> {
        if !combat_class_traits(
            self.combat_actors
                .get(actor_slot)
                .copied()?
                .owner_target_class,
        )?
        .summon_daemon
        {
            return None;
        }
        let legal_cells = self.combat_legal_cell_mask();
        let summon = self.apply_combat_summon_class_with_legal_mask(
            COMBAT_CLASS_DAEMON,
            self.combat_actor_z(actor_slot),
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            &legal_cells,
            candidate_coordinates,
        )?;
        self.message = "Monster summons daemon.".to_string();
        // `audio.md §8.3`: after successful placement and narration, run the
        // monster-summon envelope, "then perform the summon tile flash".
        //
        // `§8.3.1` identifies that flash as "the shared single-cell
        // pseudorandom pixel converge" - not the shared full-viewport flash and
        // not a bespoke effect - drawn on the one 16x16 cell the creature
        // appears in, with "no row bias" in combat and "no calibrated delay
        // ... at all". The runtime owns the cue; the converge itself is a
        // driver-side blit, whose plot order this engine already models as
        // `return_to_view::return_to_view_single_cell_write_coordinates` and
        // whose tiles are `combat_class_summon_flash_tile` then
        // `combat_class_sprite_byte`. No runtime channel carries visual
        // presentation events yet, so the flash is not emitted here.
        self.emit_sound_effect(SoundEffect::MonsterSummon);
        Some(CombatAiSpecialApplication::SummonDaemon { actor_slot, summon })
    }

    pub fn apply_combat_ai_possess_special_with_inputs(
        &mut self,
        actor_slot: usize,
        random_target_slot: usize,
        resistance_blocks: bool,
    ) -> Option<CombatAiSpecialApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_active_not_dead(actor) {
            return None;
        }
        if !combat_class_traits(actor.owner_target_class)?.possess {
            return None;
        }

        let candidates = (0..COMBAT_ACTOR_SLOTS)
            .map(|slot| {
                combat_possess_candidate_view(
                    self.combat_actors[slot],
                    // `combat.md §5`: the roster member behind a
                    // descriptor is named by the descriptor's
                    // owner/target/class byte, not by its index.
                    self.combat_roster_slot_for_actor_slot(slot)
                        .and_then(|roster_slot| self.party.get(roster_slot).copied()),
                    false,
                    false,
                )
            })
            .collect::<Vec<_>>();
        let target_slot = resolve_combat_possess_candidate_slot(
            &candidates,
            random_target_slot,
            self.active_player,
        )?;
        let target_flags_before = self.combat_actors[target_slot].flags;
        let outcome = resolve_combat_possess_resistance_outcome(
            // `combat.md §9`: the sentinel is "compared against the
            // target's own owner/character byte", which under `§5`
            // packing is not the descriptor index.
            usize::from(self.combat_actors[target_slot].owner_target_class),
            actor.owner_target_class,
            self.active_player,
            resistance_blocks,
        );

        if let CombatPossessResistanceOutcome::Landed {
            cleared_active_player,
            daemon_clears_self,
        } = outcome
        {
            self.combat_actors[target_slot].flags |= COMBAT_ACTOR_FLAG_TEAM_TOGGLE;
            if cleared_active_player {
                self.active_player = None;
            }
            if daemon_clears_self {
                self.combat_actors[actor_slot].clear();
                let linked_slot = usize::from(actor.active_object_slot);
                if let Some(object) = self.active_objects.get_mut(linked_slot) {
                    *object = ActiveObject::empty();
                }
            }
            self.mark_visibility_dirty();
            self.message = format!("Monster possessed party member {}.", target_slot + 1);
            // `audio.md §8.3`: after possession narration.
            self.emit_sound_effect(SoundEffect::Possession);
        } else {
            self.message = "Possession resisted.".to_string();
        }

        Some(CombatAiSpecialApplication::Possess {
            actor_slot,
            target_slot,
            outcome,
            target_flags_before,
            target_flags_after: self.combat_actors[target_slot].flags,
        })
    }

    fn combat_ai_handled_special_turn(
        actor_slot: usize,
        acting_group: u8,
        special: CombatAiSpecialApplication,
        possess_hook_handled: bool,
    ) -> CombatAiTurnApplication {
        CombatAiTurnApplication {
            actor_slot,
            special: Some(special),
            possess_hook_handled,
            acting_group,
            target: CombatAiTargetResolution::NoUsableTarget,
            step_vector: None,
            attack_route: None,
            monster_attack: None,
            movement: None,
            command_key: None,
            movement_commit: None,
        }
    }

    /// Run one production monster-AI turn using the shared gameplay PRNG.
    ///
    /// `combat.md §9` requires a lazy special-hook cascade. No later branch
    /// or ordinary-AI input is drawn after a handled possess, blink, or summon,
    /// and a summon probe always draws fresh X then Y coordinates.
    pub fn apply_combat_ai_turn(&mut self, actor_slot: usize) -> Option<CombatAiTurnApplication> {
        if !self.combat_active {
            return None;
        }
        let actor = *self.combat_actors.get(actor_slot)?;
        if !combat_actor_is_active_not_dead(actor) {
            return None;
        }

        let class = actor.owner_target_class;
        let acting_group = self.combat_target_group_for_slot(actor_slot);
        let enemy_magic_suppressed =
            negate_magic_aura_active(self.active_effect_tag, self.active_effect_counter);

        // `magic.md §8`: the enemy-side Negate Magic/Crown check at the
        // class-special entry is independent of the later movement and
        // ranged-effect checks. It runs before the special flags or any
        // special-hook PRNG are consulted, and an active aura falls through
        // to ordinary target selection instead of consuming the turn.
        if actor_slot >= COMBAT_PARTY_ACTOR_SLOTS && !enemy_magic_suppressed {
            let traits = combat_class_traits(class)?;
            if traits.possess {
                let target_slot = self.combat_ai_possess_target_slot_roll(actor_slot);
                if self.combat_ai_possess_candidate_reaches_resistance_from_roll(target_slot) {
                    let resistance_blocks =
                        self.combat_ai_possess_resistance_blocks(actor_slot, target_slot);
                    if let Some(special) = self.apply_combat_ai_possess_special_with_inputs(
                        actor_slot,
                        target_slot,
                        resistance_blocks,
                    ) {
                        return Some(Self::combat_ai_handled_special_turn(
                            actor_slot,
                            acting_group,
                            special,
                            true,
                        ));
                    }
                }
            }

            if traits.blink {
                let blink_roll = self.combat_ai_blink_roll(actor_slot);
                if combat_ai_special_one_in_eight_gate(blink_roll) {
                    if let Some(special) = self.apply_combat_ai_blink_special(actor_slot) {
                        return Some(Self::combat_ai_handled_special_turn(
                            actor_slot,
                            acting_group,
                            special,
                            false,
                        ));
                    }
                }
            }

            if traits.summon_daemon {
                let summon_roll = self.combat_ai_summon_roll(actor_slot);
                if combat_ai_special_one_in_eight_gate(summon_roll) {
                    let candidate = self.combat_ai_summon_probe_coordinate(actor_slot);
                    if let Some(special) = self
                        .apply_combat_ai_summon_daemon_special_with_candidates(
                            actor_slot,
                            &[candidate],
                        )
                    {
                        return Some(Self::combat_ai_handled_special_turn(
                            actor_slot,
                            acting_group,
                            special,
                            false,
                        ));
                    }
                }
            }
        }

        let mass_charm_roll = if active_effect_is_active(
            self.active_effect_tag,
            self.active_effect_counter,
            MASS_CHARM_ACTIVE_EFFECT_TAG,
        ) {
            self.combat_ai_mass_charm_roll(actor_slot)
        } else {
            0
        };
        let traits = combat_class_traits(class)?;
        let teleport_candidate = (traits.teleport_capable && !enemy_magic_suppressed)
            .then(|| self.combat_ai_teleport_candidate(actor_slot))
            .flatten();
        let horizontal_axis_first = self.combat_ai_horizontal_axis_first(actor_slot);
        let random_cardinal_direction_codes =
            self.combat_ai_random_cardinal_direction_codes(actor_slot);
        let monster_attack_inputs = self.combat_monster_attack_inputs(actor_slot);

        self.apply_combat_ai_turn_with_inputs(
            actor_slot,
            false,
            0,
            false,
            32,
            32,
            &[],
            None,
            mass_charm_roll,
            false,
            teleport_candidate,
            horizontal_axis_first,
            &random_cardinal_direction_codes,
            Some(monster_attack_inputs),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_combat_ai_turn_with_inputs(
        &mut self,
        actor_slot: usize,
        possess_candidate_reaches_resistance: bool,
        possess_target_slot: usize,
        possess_resistance_blocks: bool,
        blink_roll: u8,
        summon_roll: u8,
        summon_candidate_coordinates: &[(u8, u8)],
        cleanup_fallback_target: Option<(u8, u8)>,
        mass_charm_roll: u8,
        fleeing: bool,
        teleport_candidate: Option<(u8, u8)>,
        horizontal_axis_first: bool,
        random_cardinal_direction_codes: &[u8],
        monster_attack_inputs: Option<CombatMonsterAttackInputs>,
    ) -> Option<CombatAiTurnApplication> {
        if !self.combat_active {
            return None;
        }
        let actor = *self.combat_actors.get(actor_slot)?;
        if !combat_actor_is_active_not_dead(actor) {
            return None;
        }

        let class = actor.owner_target_class;
        let normal_group = self.combat_target_group_for_slot(actor_slot);
        let enemy_magic_suppressed =
            negate_magic_aura_active(self.active_effect_tag, self.active_effect_counter);
        let summon_candidate = summon_candidate_coordinates.first().copied();
        let summon_can_place_daemon = if !enemy_magic_suppressed
            && combat_class_traits(class).is_some_and(|traits| traits.summon_daemon)
        {
            let legal_cells = self.combat_legal_cell_mask();
            summon_candidate.is_some_and(|candidate| {
                resolve_combat_clone_placement_coordinate(&legal_cells, &[candidate]).is_some()
            }) && resolve_clone_spell_allocation(&self.combat_actors, &self.active_objects)
                .is_some()
        } else {
            false
        };

        let special_hook = (actor_slot >= COMBAT_PARTY_ACTOR_SLOTS && !enemy_magic_suppressed)
            .then(|| {
                resolve_combat_ai_special_hook(
                    class,
                    possess_candidate_reaches_resistance,
                    blink_roll,
                    summon_roll,
                    summon_can_place_daemon,
                )
            })
            .flatten();
        match special_hook {
            Some(CombatAiSpecialHook::Possess) => {
                if let Some(special) = self.apply_combat_ai_possess_special_with_inputs(
                    actor_slot,
                    possess_target_slot,
                    possess_resistance_blocks,
                ) {
                    return Some(Self::combat_ai_handled_special_turn(
                        actor_slot,
                        normal_group,
                        special,
                        true,
                    ));
                }
            }
            Some(CombatAiSpecialHook::Blink) => {
                if let Some(special) = self.apply_combat_ai_blink_special(actor_slot) {
                    return Some(Self::combat_ai_handled_special_turn(
                        actor_slot,
                        normal_group,
                        special,
                        false,
                    ));
                }
            }
            Some(CombatAiSpecialHook::SummonDaemon) => {
                if let Some(candidate) = summon_candidate {
                    if let Some(special) = self
                        .apply_combat_ai_summon_daemon_special_with_candidates(
                            actor_slot,
                            &[candidate],
                        )
                    {
                        return Some(Self::combat_ai_handled_special_turn(
                            actor_slot,
                            normal_group,
                            special,
                            false,
                        ));
                    }
                }
            }
            None => {}
        }

        let acting_group = if active_effect_is_active(
            self.active_effect_tag,
            self.active_effect_counter,
            MASS_CHARM_ACTIVE_EFFECT_TAG,
        ) {
            let threshold = if actor_slot < COMBAT_PARTY_ACTOR_SLOTS {
                self.party
                    .get(usize::from(actor.owner_target_class))
                    .map(|member| member.climb_stat)
            } else {
                combat_class_stats(class).map(|stats| stats.mass_charm_threshold())
            };
            threshold
                .map(|threshold| {
                    resolve_mass_charm_target_group(normal_group, threshold, mass_charm_roll)
                })
                .unwrap_or(normal_group)
        } else {
            normal_group
        };

        let candidates = self
            .combat_actors
            .iter()
            .copied()
            .enumerate()
            .map(|(slot, descriptor)| {
                self.combat_target_candidate_view(
                    descriptor,
                    slot,
                    descriptor.is_hidden_or_unrevealed(),
                    false,
                )
            })
            .collect::<Vec<_>>();
        let bypass_suppression_filter = self.combat_suppression_filter_bypassed_for_class(class);
        let pick = find_combat_ai_target(
            &candidates,
            actor_slot,
            acting_group,
            bypass_suppression_filter,
        );
        let target = resolve_combat_ai_target_after_scan(
            &mut self.combat_actors,
            pick,
            cleanup_fallback_target,
        );

        let (target_x, target_y, target_slot) = match target {
            CombatAiTargetResolution::ChosenActor { slot, x, y } => (x, y, Some(slot)),
            CombatAiTargetResolution::CleanupFallback { x, y }
            | CombatAiTargetResolution::CenterFallback { x, y, .. } => (x, y, None),
            CombatAiTargetResolution::NoUsableTarget => {
                return Some(CombatAiTurnApplication {
                    actor_slot,
                    special: None,
                    possess_hook_handled: false,
                    acting_group,
                    target,
                    step_vector: None,
                    attack_route: None,
                    monster_attack: None,
                    movement: None,
                    command_key: None,
                    movement_commit: None,
                });
            }
        };

        let fleeing = fleeing || self.combat_ai_actor_fleeing(actor_slot);
        let actor = self.combat_actors[actor_slot];
        let step_vector = combat_ai_step_vector(actor.x, actor.y, target_x, target_y, fleeing);
        let target_range = target_slot.map(|slot| actor.range_to(self.combat_actors[slot]));
        // `combat.md §6.1a` "Readers — the attack driver": an actor carrying
        // bit `0x01` resolves its attack as a fixed magic strike. The driver
        // still picks a target the normal way and then adds one requirement
        // the ordinary path has no counterpart for — the chosen target must
        // be at straight-line distance exactly one. Further away, the turn
        // produces no action at all: no fallthrough to the ranged branch, no
        // consult of the class maximum-attack-range byte, and no step.
        let controlled_attacker = actor.is_controlled();
        let attack_route = target_range.and_then(|range| {
            if controlled_attacker {
                (range == 1).then_some(CombatAiAttackRoute::Melee)
            } else {
                resolve_combat_ai_attack_route(class, range)
            }
        });
        if controlled_attacker && !matches!(attack_route, Some(CombatAiAttackRoute::Melee)) {
            return Some(CombatAiTurnApplication {
                actor_slot,
                special: None,
                possess_hook_handled: false,
                acting_group,
                target,
                step_vector: Some(step_vector),
                attack_route: None,
                monster_attack: None,
                movement: None,
                command_key: None,
                movement_commit: None,
            });
        }
        if matches!(
            attack_route,
            Some(CombatAiAttackRoute::Melee | CombatAiAttackRoute::RangedEffect { .. })
        ) {
            // `magic.md §8`: only a scene-resistant, non-adjacent effect is
            // aborted here. The outer attack driver still reports the action
            // as handled, so the result is a silent consumed attack with no
            // projectile, hit test, damage, or status path.
            let resistant_ranged_effect_suppressed =
                matches!(
                    attack_route,
                    Some(CombatAiAttackRoute::RangedEffect {
                        scene_resistance: true,
                        ..
                    })
                ) && negate_magic_aura_active(self.active_effect_tag, self.active_effect_counter);
            let monster_attack = (!resistant_ranged_effect_suppressed)
                .then_some(target_slot)
                .flatten()
                .and_then(|target_slot| {
                    monster_attack_inputs.and_then(|inputs| {
                        if matches!(attack_route, Some(CombatAiAttackRoute::RangedEffect { .. }))
                            && self.combat_monster_amulet_turning_scatter_applies(
                                actor_slot,
                                target_slot,
                            )
                        {
                            self.resolve_and_apply_combat_monster_scattered_attack(
                                actor_slot,
                                target_slot,
                                inputs.party_defender_rating,
                                inputs.hit_roll,
                                inputs.damage_roll,
                                inputs.amulet_turning_scatter_roll,
                            )
                        } else {
                            self.resolve_and_apply_combat_monster_attack(
                                actor_slot,
                                target_slot,
                                inputs.party_defender_rating,
                                inputs.hit_roll,
                                inputs.damage_roll,
                                inputs.poison_gate_accepts,
                                inputs.poison_damage_roll,
                                inputs.forced_hit,
                            )
                        }
                    })
                });
            return Some(CombatAiTurnApplication {
                actor_slot,
                special: None,
                possess_hook_handled: false,
                acting_group,
                target,
                step_vector: Some(step_vector),
                attack_route,
                monster_attack,
                movement: None,
                command_key: Some(COMBAT_AI_ATTACK_COMMAND_KEY),
                movement_commit: None,
            });
        }

        let legal_cells = self.combat_legal_cell_mask();
        let traits = combat_class_traits(class)?;
        let movement = resolve_combat_ai_movement(
            &legal_cells,
            actor.x,
            actor.y,
            step_vector,
            traits.teleport_capable
                && !negate_magic_aura_active(self.active_effect_tag, self.active_effect_counter),
            teleport_candidate,
            horizontal_axis_first,
            random_cardinal_direction_codes,
        );
        let movement_commit = commit_combat_ai_movement_outcome(
            &mut self.combat_actors[actor_slot],
            &mut self.active_objects,
            movement,
        );
        if movement_commit.is_some() {
            self.mark_visibility_dirty();
            let _ = self.apply_combat_ambush_reveal_for_actor_position(actor_slot);
        }
        let movement_direction_code = match movement {
            CombatAiMovementOutcome::Step { direction_code, .. } => Some(direction_code),
            CombatAiMovementOutcome::Teleport { .. } | CombatAiMovementOutcome::Blocked { .. } => {
                None
            }
        };
        let command_key = resolve_combat_ai_synthesized_command_key(None, movement_direction_code);

        Some(CombatAiTurnApplication {
            actor_slot,
            special: None,
            possess_hook_handled: false,
            acting_group,
            target,
            step_vector: Some(step_vector),
            attack_route,
            monster_attack: None,
            movement: Some(movement),
            command_key,
            movement_commit,
        })
    }

    pub fn apply_combat_swarm_with_random_attempts(
        &mut self,
        z: i8,
        legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    ) -> Vec<CombatSummonApplication> {
        let mut accepted_coordinate = None;
        for _ in 0..8 {
            let x = self.random_range_u8(0, 15);
            let y = self.random_range_u8(0, 15);
            if usize::from(x) >= COMBAT_ARENA_SIDE || usize::from(y) >= COMBAT_ARENA_SIDE {
                continue;
            }
            if combat_ai_legal_cell(legal_cells, i16::from(x), i16::from(y)) {
                accepted_coordinate = Some((x, y));
                break;
            }
        }

        let Some((accepted_x, accepted_y)) = accepted_coordinate else {
            return Vec::new();
        };
        let mut accepted = Vec::new();
        for _ in 0..4 {
            let Some(application) = self.apply_combat_summon_class_with_legal_mask(
                COMBAT_CLASS_INSECT_SWARM,
                z,
                COMBAT_SUMMONED_ACTOR_FLAGS,
                legal_cells,
                &[(accepted_x, accepted_y)],
            ) else {
                break;
            };
            accepted.push(application);
        }
        accepted
    }

    pub fn cast_combat_conjure_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6`: Conjure shares variant 2. `audio.md §8.3`: confirmation
        // plays the spell effect before the placement resolver.
        self.emit_sound_effect(SoundEffect::SharedVariant { variant: 2 });

        let class = resolve_conjure_spell_class(self.combat_conjure_class_selector());
        let legal_cells = self.combat_legal_cell_mask();
        let applied = self.apply_combat_conjure_class_with_random_attempts(
            class,
            self.combat_actor_z(caster_index),
            &legal_cells,
        );
        self.advance_turn();
        self.message = if applied.is_some() {
            "Success!".to_string()
        } else {
            "Failed!".to_string()
        };
        if applied.is_none() {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
        }
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn cast_combat_swarm_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6`: Swarm shares variant 5. `audio.md §8.3`: confirmation
        // plays the spell effect before the placement resolver.
        self.emit_sound_effect(SoundEffect::SharedVariant { variant: 5 });

        let legal_cells = self.combat_legal_cell_mask();
        let applied = self.apply_combat_swarm_with_random_attempts(
            self.combat_actor_z(caster_index),
            &legal_cells,
        );
        self.advance_turn();
        self.message = if applied.is_empty() {
            "Failed!".to_string()
        } else {
            "Success!".to_string()
        };
        if applied.is_empty() {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
            MoveOutcome::Blocked
        } else {
            MoveOutcome::Cast
        }
    }

    pub fn cast_combat_summon_daemon_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(caster) = self.combat_actors.get(caster_index).copied() else {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        };
        if !combat_actor_is_active_not_dead(caster) {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6.1` id 43: "**Unconditional at placement-helper entry,
        // before the eight-try cell probe**, so a failed placement still plays
        // it." The variant is the spell's circle, 8. `§8.3` then adds the
        // player Summon envelope only on an accepted placement.
        if let Some(variant) = audio::spell_shared_variant(spell_index) {
            self.emit_sound_effect(SoundEffect::SharedVariant { variant });
        }

        let legal_cells = self.combat_legal_cell_mask();
        let applied = self.apply_combat_summon_daemon_with_random_attempts(
            self.combat_actor_z(caster_index),
            &legal_cells,
        );
        self.advance_turn();
        let Some(applied) = applied else {
            self.message = "Failed!".to_string();
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
            return MoveOutcome::Blocked;
        };
        // `audio.md §8.3`: an accepted placement additionally runs the player
        // Summon envelope before actor finalization. The `Oops...` branch is
        // still an accepted placement, so the envelope precedes the self check.
        //
        // `§8.3.1`: "The monster summon and the player Summon spell use an
        // identical construct: play the envelope cue, set the new actor's tile
        // to a placeholder, run the converge on the flash tile, then set the
        // actor's tile to the real creature sprite." The converge half is
        // driver-side; see the note in
        // `apply_combat_ai_summon_daemon_special_with_coordinates`.
        self.emit_sound_effect(SoundEffect::PlayerSummon);
        if self.combat_summon_daemon_self_check_oops(caster_index) {
            self.message = "Oops...".to_string();
            MoveOutcome::Blocked
        } else {
            self.combat_actors[applied.actor_slot].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
            self.message = "Summon Daemon!".to_string();
            MoveOutcome::Cast
        }
    }

    pub fn combat_summon_daemon_self_check_threshold(&self, caster_index: usize) -> u8 {
        self.party_intelligence
            .get(caster_index)
            .copied()
            .or_else(|| {
                self.combat_actors
                    .get(caster_index)
                    .and_then(|actor| combat_class_stats(actor.owner_target_class))
                    .map(|stats| stats.endurance)
            })
            .unwrap_or(self.avatar_stats.intelligence)
    }

    pub fn combat_summon_daemon_self_check_oops(&mut self, caster_index: usize) -> bool {
        let threshold = self.combat_summon_daemon_self_check_threshold(caster_index);
        let raw_roll = self.random_range_u8(0, 60);
        let roll = combat_skewed_roll_1_to_30(raw_roll);
        roll >= threshold
    }

    pub fn cast_combat_clone_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        target_slot: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let target_actor = self
            .combat_actors
            .get(target_slot)
            .copied()
            .unwrap_or_default();
        let caster_group = self.combat_target_group_for_slot(caster_index);
        let target_group = self.combat_target_group_for_slot(target_slot);
        if !creature_prompt_target_is_eligible(target_actor, target_group, caster_group, false) {
            self.message = "Target? Use C1IQX7 to target a hostile creature.".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6`: the creature clone shares variant 7. `audio.md §8.3`:
        // confirmation plays the spell effect before the placement resolver.
        self.emit_sound_effect(SoundEffect::SharedVariant { variant: 7 });

        let legal_cells = self.combat_legal_cell_mask();
        let candidates = combat_clone_candidate_coordinates(self.combat_arena_placement_seed());
        let applied =
            self.apply_combat_clone_with_legal_mask(target_slot, &legal_cells, &candidates);
        self.advance_turn();
        self.message = if applied.is_some() {
            "Clone!".to_string()
        } else {
            "Failed!".to_string()
        };
        if applied.is_none() {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
        }
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn combat_legal_cell_mask(&self) -> [[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE] {
        let mut legal_cells = build_combat_ai_legal_cell_mask(
            &self.combat_terrain,
            &self.combat_actors,
            is_combat_arena_tile_walkable,
        );
        for object in self.active_objects.iter().take(OOL_SLOTS) {
            if object.type_byte != COMBAT_FIELD_KIND_ENERGY {
                continue;
            }
            if object.x < COMBAT_ARENA_SIDE && object.y < COMBAT_ARENA_SIDE {
                legal_cells[object.y][object.x] = false;
            }
        }
        legal_cells
    }

    pub fn enter_combat_frame(
        &mut self,
        active_objects: Vec<ActiveObject>,
        actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    ) -> io::Result<CombatFrameSnapshot> {
        self.enter_combat_frame_with_terrain(active_objects, actors, DEFAULT_COMBAT_ARENA_TERRAIN)
    }

    pub fn enter_combat_frame_with_terrain(
        &mut self,
        active_objects: Vec<ActiveObject>,
        actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    ) -> io::Result<CombatFrameSnapshot> {
        self.enter_combat_frame_with_terrain_and_reveals(
            active_objects,
            actors,
            terrain,
            [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
        )
    }

    pub fn enter_combat_frame_with_terrain_and_reveals(
        &mut self,
        active_objects: Vec<ActiveObject>,
        actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
        terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        reveals: [Option<CombatAmbushRevealRecord>; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
    ) -> io::Result<CombatFrameSnapshot> {
        if active_objects.len() != OOL_SLOTS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "combat frame requires {OOL_SLOTS} active-object records, got {}",
                    active_objects.len()
                ),
            ));
        }

        let snapshot = CombatFrameSnapshot {
            area: self.area,
            player: self.player,
            active_objects: self.active_objects.clone(),
            active_player: self.active_player,
            combat_terrain: self.combat_terrain,
            dungeon_room_clear_on_success: None,
            enter_endgame_after_successful_combat: false,
            endgame_messages: None,
            endgame_tableau_map: None,
            encounter_mode_high_bit: false,
            suppress_controlled_faint_sleep_tick: false,
            exit_announced: false,
            established_exit_direction_code: None,
        };
        self.active_objects = active_objects;
        self.combat_actors = actors;
        self.combat_terrain = terrain;
        self.combat_magic_effects = [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        self.combat_cursor_blink = false;
        self.combat_secondary_marker = None;
        self.combat_ambush_reveals = reveals;
        self.combat_active = true;
        self.combat_action_result = 0;
        self.pending_combat_terrain_reveals.clear();
        self.apply_combat_entry_magic_ring_passes();
        // `visibility.md §12.6`: "combat entry switches the beacon off
        // outright". There is no matching exit trigger; the beacon stays off
        // until a map loader next harvests a source.
        self.light_beacon.switch_off();
        // `visibility.md §12.4`: combat entry rebuilds the shared scratch
        // mask after setup, before combat reuses that storage.
        self.rebuild_surface_local_light_mask();
        self.combat_frame_snapshot = Some(snapshot.clone());
        self.pending_combat_actor_slot = None;
        self.pending_combat_terrain_trigger_slot = None;
        self.next_combat_actor_slot = 0;
        Ok(snapshot)
    }

    pub fn restore_combat_frame(&mut self, snapshot: CombatFrameSnapshot) {
        self.restore_combat_frame_with_trigger_reconcile(snapshot, false);
    }

    fn restore_combat_frame_with_trigger_reconcile(
        &mut self,
        snapshot: CombatFrameSnapshot,
        body_retrieval_exit: bool,
    ) {
        let pending_terrain_trigger = self.pending_combat_terrain_trigger_slot.take();
        self.area = snapshot.area;
        self.player = snapshot.player;
        self.active_objects = snapshot.active_objects;
        self.active_player =
            resolve_post_combat_active_player_restore(snapshot.active_player, &self.party);
        self.combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
        self.combat_terrain = snapshot.combat_terrain;
        self.combat_magic_effects = [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        self.combat_cursor_blink = false;
        self.combat_secondary_marker = None;
        self.combat_ambush_reveals = [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT];
        self.combat_active = false;
        self.combat_action_result = 0;
        self.combat_frame_snapshot = None;
        self.pending_combat_actor_slot = None;
        self.next_combat_actor_slot = 0;
        if let Some(slot) = pending_terrain_trigger {
            reconcile_post_combat_terrain_trigger_slot(
                &mut self.active_objects,
                slot,
                body_retrieval_exit,
            );
        }
        if matches!(self.area, Area::Dungeon { .. }) {
            self.setup_dungeon_active_monster_fresh();
        }
        // `town-mode.md §14`: "On exit the town chain clears the NPC
        // slot, reloads the town map, and re-runs the Shadowlord install
        // pass of Section 13". The slot clear needs no game directory
        // and runs here, with the frame restored; the reload and the
        // re-install are drained at the input boundary.
        if let Some(mut pending) = self.pending_town_conflict.take() {
            if !pending.awaiting_floor_reload {
                if let Ok(scene) = Scene::new(pending.scene_byte) {
                    self.clear_town_conflict_npc_slot(scene, pending.npc_slot, pending.type_byte);
                }
                pending.awaiting_floor_reload = true;
            }
            self.pending_town_conflict = Some(pending);
        }
        // `visibility.md §12.4`: restore the non-combat influence mask after
        // the combat terrain scratch has been released.
        self.rebuild_surface_local_light_mask();
        self.mark_visibility_dirty();
    }

    pub fn apply_combat_round_loop_exit(
        &mut self,
        exit: CombatRoundLoopExit,
    ) -> CombatRoundLoopExitApplication {
        match exit {
            CombatRoundLoopExit::Defeat => self.message = "\nBATTLE IS LOST!".to_string(),
            // `combat.md §14`: there is no separate victory message.
            CombatRoundLoopExit::Victory => self.message.clear(),
            CombatRoundLoopExit::LeaveCombat => {}
        }
        let result_code = exit.result_code();
        let body_retrieval_exit =
            combat_exit_requests_body_retrieval_reconcile(exit, &self.combat_actors)
                || (!matches!(exit, CombatRoundLoopExit::Defeat)
                    && self
                        .combat_frame_snapshot
                        .as_ref()
                        .is_some_and(|snapshot| snapshot.exit_announced)
                    && !combat_has_active_not_dead_non_party_actor(&self.combat_actors));
        let restored_snapshot = if let Some(snapshot) = self.combat_frame_snapshot.take() {
            let enter_endgame_after_restore =
                snapshot.enter_endgame_after_successful_combat && body_retrieval_exit;
            let endgame_messages = snapshot.endgame_messages.clone();
            let endgame_tableau_map = snapshot.endgame_tableau_map.clone();
            let dungeon_room_clear_on_success = snapshot.dungeon_room_clear_on_success;
            self.restore_combat_frame_with_trigger_reconcile(snapshot, body_retrieval_exit);
            if result_code != COMBAT_ROUND_RESULT_DEFEAT {
                if let Some(clear) = dungeon_room_clear_on_success {
                    set_dungeon_room_clear_bit(
                        &mut self.dungeon_room_clear_bitmap,
                        clear.scene,
                        clear.room_slot,
                    );
                    self.mark_visibility_dirty();
                }
            }
            if enter_endgame_after_restore {
                self.enter_endgame_with_resources(endgame_messages, endgame_tableau_map);
            }
            true
        } else {
            self.combat_active = false;
            self.pending_combat_actor_slot = None;
            if let Some(slot) = self.pending_combat_terrain_trigger_slot.take() {
                reconcile_post_combat_terrain_trigger_slot(
                    &mut self.active_objects,
                    slot,
                    body_retrieval_exit,
                );
            }
            self.next_combat_actor_slot = 0;
            self.combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
            self.combat_ambush_reveals = [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT];
            self.mark_visibility_dirty();
            false
        };
        CombatRoundLoopExitApplication {
            exit,
            result_code,
            restored_snapshot,
        }
    }

    pub fn reconcile_post_combat_terrain_trigger_slot(
        &mut self,
        slot: usize,
        body_retrieval_exit: bool,
    ) -> PostCombatTriggerReconcile {
        let outcome = reconcile_post_combat_terrain_trigger_slot(
            &mut self.active_objects,
            slot,
            body_retrieval_exit,
        );
        if !matches!(outcome, PostCombatTriggerReconcile::MissingSlot) {
            self.mark_visibility_dirty();
        }
        outcome
    }

    pub fn apply_combat_party_damage_to_slot(
        &mut self,
        slot: usize,
        raw_damage: i16,
    ) -> Option<CombatPartyDamageOutcome> {
        // `combat.md §5`: the damaged character is the descriptor's
        // owner/target/class byte. With a packed party the descriptor
        // index and the roster index differ, so indexing the roster by
        // the descriptor slot wounds the wrong character.
        let roster_slot = self.combat_roster_slot_for_actor_slot(slot)?;
        let outcome = apply_combat_party_damage(self.party.get_mut(roster_slot)?, raw_damage);
        if outcome.killed {
            if self.active_player == Some(roster_slot) {
                self.active_player = None;
            }
            if let Some(actor) = self.combat_actors.get(slot) {
                if let Some(object) = self
                    .active_objects
                    .get_mut(actor.active_object_slot as usize)
                {
                    object.type_byte = COMBAT_PARTY_CORPSE_TILE;
                    object.tile = COMBAT_PARTY_CORPSE_TILE;
                    object.phase = STEADY_PHASE;
                }
            }
        }
        Some(outcome)
    }

    pub fn credit_combat_party_attacker_experience(
        &mut self,
        attacker_slot: usize,
        reward: u8,
    ) -> Option<u16> {
        // `combat.md §5`: the credited character is the descriptor's
        // owner/target/class byte - "the character's roster slot index" -
        // because a party with a dead member packs into lower descriptor
        // indexes than its roster indexes.
        let roster_slot = self.combat_roster_slot_for_actor_slot(attacker_slot)?;
        if !self.party.get(roster_slot)?.living() {
            return None;
        }

        if self.party_experience.len() < self.party.len() {
            self.party_experience.resize(self.party.len(), 0);
        }
        let experience = self.party_experience.get_mut(roster_slot)?;
        *experience = apply_combat_experience_reward(*experience, reward);
        Some(*experience)
    }

    fn run_combat_terrain_reveal(
        &mut self,
        actor_slot: usize,
        arena_cell: (u8, u8),
        terrain_tile: u8,
    ) {
        let playback = combat_terrain_reveal_playback(actor_slot, arena_cell, terrain_tile);
        for _ in &playback.world_tick_after_operations {
            self.advance_visual_tick();
        }
        self.pending_combat_terrain_reveals.push(playback);
    }

    pub fn take_pending_combat_terrain_reveals(&mut self) -> Vec<CombatTerrainRevealPlayback> {
        std::mem::take(&mut self.pending_combat_terrain_reveals)
    }

    /// Negative-form release used by vanished, incorporeal, terrain-rejected,
    /// and Gargoyle deaths. The descriptor owner/class and linked object's two
    /// trailing auxiliary bytes intentionally remain stale.
    fn release_combat_actor_slot_negative(&mut self, actor_slot: usize) -> bool {
        let Some(actor) = self.combat_actors.get(actor_slot).copied() else {
            return false;
        };
        self.combat_actors[actor_slot].release_preserving_owner_target_class();
        if let Some(object) = self
            .active_objects
            .get_mut(usize::from(actor.active_object_slot))
        {
            object.clear_record_prefix();
        }
        true
    }

    /// `combat.md §6.3`: the vanish tail scans party combat descriptors in
    /// slot order and handles only the first party-side controlled actor.
    fn apply_combat_vanish_party_control_faint_tail(&mut self) -> Option<usize> {
        let actor_slot = (0..COMBAT_PARTY_ACTOR_SLOTS).find(|slot| {
            let flags = self.combat_actors[*slot].flags;
            flags & COMBAT_ACTOR_FLAG_SELECTABLE_80 != 0
                && flags & COMBAT_ACTOR_FLAG_CONTROLLED != 0
        })?;
        let actor = self.combat_actors[actor_slot];
        let roster_slot = usize::from(actor.owner_target_class);
        self.combat_actors[actor_slot].flags &= !COMBAT_ACTOR_FLAG_CONTROLLED;

        let name = self
            .party_names
            .get(roster_slot)
            .and_then(|bytes| party_name_to_string(bytes))
            .unwrap_or_default();
        self.emit_message_line(format!("{name} passes out!"));

        // `combat.md §6.3`: narration completes before this blocking cue;
        // equipment removal and the sleep helper follow only after it ends.
        self.emit_sound_effect(SoundEffect::ControlledPartyFaint);

        if let Some(equipment) = self.party_equipment.get_mut(roster_slot)
            && let Some(item) = equipment
                .iter_mut()
                .find(|item| usize::from(**item) == EQUIPMENT_SWORD_OF_CHAOS)
        {
            *item = EQUIPMENT_EMPTY;
        }

        let slept = self.party.get_mut(roster_slot).is_some_and(|member| {
            matches!(
                apply_combat_sleep_to_party_target(member),
                CombatPartySleepOutcome::SleptPartyMember { .. }
            )
        });
        if slept {
            self.combat_actors[actor_slot].set_status_disabled();
            if let Some(object) = self
                .active_objects
                .get_mut(usize::from(actor.active_object_slot))
            {
                object.tile = COMBAT_POTION_SLEEP_DISPLAY_TILE;
            }
            if self.active_player == Some(roster_slot) {
                self.active_player = None;
            }
            // The normal sleep helper replaces, rather than ORs, the vanish
            // narration bit. This preserves the original duplicate-narration
            // edge documented by §6.3.
            self.combat_action_result = COMBAT_ACTION_RESULT_SLEEP;
            self.mark_visibility_dirty();
            let suppress_tick = self
                .combat_frame_snapshot
                .as_ref()
                .is_some_and(|snapshot| snapshot.suppress_controlled_faint_sleep_tick);
            if !suppress_tick {
                self.advance_visual_tick();
            }
        }
        Some(actor_slot)
    }

    fn apply_combat_monster_death_active_object_effect(
        &mut self,
        target_slot: usize,
        damage: CombatMonsterDamageOutcome,
    ) -> bool {
        let Some(death_path) = damage.death_path else {
            return false;
        };
        let Some(actor) = self.combat_actors.get(target_slot).copied() else {
            return false;
        };
        let active_object_slot = usize::from(actor.active_object_slot);
        let mut changed = false;

        match death_path {
            CombatMonsterDeathPath::Vanish => {
                // `combat.md §6.3`: narration and the result store precede
                // the temporary marker and the blocking cell reveal.
                if let Some(stats) = combat_class_stats(damage.class) {
                    self.emit_message_line(format!("{} vanishes!", stats.name));
                }
                self.combat_action_result = COMBAT_ACTION_RESULT_VANISH_NARRATED;
                if let Some(object) = self.active_objects.get_mut(active_object_slot) {
                    object.type_byte = COMBAT_VANISH_DEATH_MARKER_TILE;
                    object.tile = COMBAT_VANISH_DEATH_MARKER_TILE;
                    object.phase = STEADY_PHASE;
                    object.aux1 = 0;
                }
                let terrain_tile = self
                    .combat_terrain
                    .get(usize::from(actor.y))
                    .and_then(|row| row.get(usize::from(actor.x)))
                    .copied()
                    .unwrap_or(0);
                self.run_combat_terrain_reveal(target_slot, (actor.x, actor.y), terrain_tile);
                self.release_combat_actor_slot_negative(target_slot);
                let _ = self.apply_combat_vanish_party_control_faint_tail();
                changed = true;
            }
            CombatMonsterDeathPath::Incorporeal => {
                // `combat.md §6.3` Incorporeal-class row: tile byte
                // written into active-object bytes 0 and 1 is "none",
                // other writes are "none", slot released "Yes".
                // `§12`: the branch "releases the slot immediately and
                // leaves **no tile marker and no drop at all**", so the
                // linked active-object record must be left exactly as
                // the per-encounter reset produced it — no marker, no
                // byte-5 drop value, and no drop rolls.
                self.release_combat_actor_slot_negative(target_slot);
                changed = true;
            }
            CombatMonsterDeathPath::DefaultDropCheck => {
                let Some(stats) = combat_class_stats(damage.class) else {
                    return changed;
                };
                let terrain = self
                    .combat_terrain
                    .get(usize::from(actor.y))
                    .and_then(|row| row.get(usize::from(actor.x)))
                    .copied()
                    .unwrap_or(0);
                if terrain == 0x87 || terrain < 4 {
                    self.release_combat_actor_slot_negative(target_slot);
                    changed = true;
                    if changed {
                        self.mark_visibility_dirty();
                    }
                    return changed;
                }
                let drop_cap = stats.default_drop_cap;
                let (first_roll, second_roll) = self.combat_default_death_drop_rolls();
                let marker = resolve_default_monster_death_marker(
                    drop_cap,
                    combat_default_death_drop_gate_accepts_inclusive(drop_cap, first_roll),
                    combat_default_death_drop_gate_accepts(drop_cap, second_roll),
                );
                if let Some(object) = self.active_objects.get_mut(active_object_slot) {
                    match marker {
                        CombatDefaultDeathMarker::Drop { loot_byte } => {
                            object.type_byte = COMBAT_DEFAULT_DEATH_DROP_TILE;
                            object.tile = COMBAT_DEFAULT_DEATH_DROP_TILE;
                            object.aux1 = loot_byte;
                        }
                        CombatDefaultDeathMarker::NoDrop => {
                            object.type_byte = COMBAT_DEFAULT_DEATH_NO_DROP_TILE;
                            object.tile = COMBAT_DEFAULT_DEATH_NO_DROP_TILE;
                            object.aux1 = 0;
                        }
                    }
                    object.phase = STEADY_PHASE;
                    changed = true;
                }
            }
            CombatMonsterDeathPath::SpecialTileTransition if damage.class == 28 => {
                let mut death_z = 0;
                if let Some(object) = self.active_objects.get_mut(active_object_slot) {
                    object.type_byte = COMBAT_GAZER_DEATH_MARKER_TILE;
                    object.tile = COMBAT_GAZER_DEATH_MARKER_TILE;
                    object.phase = STEADY_PHASE;
                    object.aux1 = 0;
                    death_z = object.z;
                    changed = true;
                }
                // `combat.md §6.3` "The Gazer death spawns a real combatant":
                // after stamping its own `0x1F` marker the branch calls the
                // ordinary monster-placement primitive with class 31 and the
                // dying Gazer's arena coordinates and Z plane, then redraws.
                // The Gazer keeps its marker and its slot; the swarm is a live
                // hostile actor, not a particle effect. The spawn is skipped
                // with no other side effect when either table is full.
                if self
                    .place_combat_monster_at_arena_cell(
                        COMBAT_CLASS_INSECT_SWARM,
                        actor.x,
                        actor.y,
                        death_z,
                        COMBAT_ACTOR_FLAG_SELECTABLE_80,
                    )
                    .is_some()
                {
                    changed = true;
                }
            }
            CombatMonsterDeathPath::SpecialTileTransition if damage.class == 30 => {
                // `combat.md §6.3` Gargoyle row + "Gargoyle does not fall
                // through to the ordinary path": the branch writes `0x4C` into
                // the arena terrain cell under the actor, writes no tile byte
                // into the active-object record, runs no drop rolls, and goes
                // straight to the slot-clear helper.
                let actor_x = usize::from(actor.x);
                let actor_y = usize::from(actor.y);
                if actor_y < COMBAT_ARENA_SIDE && actor_x < COMBAT_ARENA_SIDE {
                    self.combat_terrain[actor_y][actor_x] = COMBAT_GARGOYLE_DEATH_TERRAIN_TILE;
                }
                self.release_combat_actor_slot_negative(target_slot);
                changed = true;
            }
            CombatMonsterDeathPath::SpecialTileTransition => {}
        }
        if changed {
            self.mark_visibility_dirty();
        }
        changed
    }

    /// `combat.md §12` "Splitting / replicating monsters": a class carrying
    /// the split-on-damage flag that is damaged but **not** killed looks for
    /// an empty slot in the actor table — up to eight attempts — copies the
    /// parent's class byte into it, and prints `<monster name> divides!`.
    /// The child is placed through the ordinary monster-placement primitive
    /// at the parent's own arena cell and Z plane with the hostile tag; the
    /// eight-attempt cap lives in [`resolve_combat_split_placement`].
    pub fn apply_combat_monster_split_placement(
        &mut self,
        target_slot: usize,
        damage: CombatMonsterDamageOutcome,
    ) -> Option<CombatSummonApplication> {
        let parent = self.combat_actors.get(target_slot).copied()?;
        let candidate_slots =
            (COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS).collect::<Vec<usize>>();
        let placement = resolve_combat_split_placement(
            damage.class,
            damage.applied_damage,
            damage.killed,
            &self.combat_actors,
            &candidate_slots,
        )?;
        let z = self
            .active_objects
            .get(usize::from(parent.active_object_slot))
            .map(|object| object.z)
            .unwrap_or_default();
        let child = self.place_combat_monster_at_arena_cell(
            placement.class,
            parent.x,
            parent.y,
            z,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
        )?;
        if let Some(stats) = combat_class_stats(placement.class) {
            self.message = format!("{} divides!", stats.name);
        }
        Some(child)
    }

    pub fn apply_combat_weapon_damage_to_target(
        &mut self,
        attacker_slot: Option<usize>,
        target_slot: usize,
        raw_damage: i16,
        magical: bool,
    ) -> Option<CombatWeaponDamageApplication> {
        if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            let damage = self.apply_combat_party_damage_to_slot(target_slot, raw_damage)?;
            return Some(CombatWeaponDamageApplication::Party {
                target_slot,
                damage,
            });
        }

        let damage = self
            .combat_actors
            .get_mut(target_slot)?
            .apply_monster_damage(raw_damage, magical)?;
        if damage.killed {
            self.apply_combat_monster_death_active_object_effect(target_slot, damage);
        } else {
            self.apply_combat_monster_split_placement(target_slot, damage);
        }
        let credited_experience = if damage.return_value == 0 {
            None
        } else {
            attacker_slot.and_then(|slot| {
                self.credit_combat_party_attacker_experience(slot, damage.return_value)
            })
        };

        Some(CombatWeaponDamageApplication::Monster {
            target_slot,
            damage,
            credited_experience,
        })
    }

    pub fn apply_active_target_combat_spell_damage(
        &mut self,
        caster_slot: Option<usize>,
        target_slot: usize,
        kind: CombatSpellDamageKind,
        damage_roll: u8,
        defense_roll: u8,
    ) -> Option<CombatActiveTargetSpellDamageApplication> {
        let raw_damage = resolve_active_target_spell_damage(kind, damage_roll, defense_roll)?;
        let damage_application =
            self.apply_combat_weapon_damage_to_target(caster_slot, target_slot, raw_damage, true)?;

        Some(CombatActiveTargetSpellDamageApplication {
            kind,
            raw_damage,
            damage_application,
        })
    }

    pub fn cast_active_target_combat_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        kind: CombatSpellDamageKind,
        target_slot: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let target_actor = self.combat_actors.get(target_slot).copied();
        if target_slot >= COMBAT_ACTOR_SLOTS
            || !target_actor.is_some_and(combat_actor_is_active_not_dead)
        {
            self.message = "Target? Use C1GP7 to target a live combat slot.".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6.1`: Magic Missile (id 1), Fireball (id 13) and Kill
        // (id 37) are three of the seven ids that "do not reach the dispatcher
        // on any path" and play the **combat effect template** instead: the
        // circle-scaled rumble lead alone. `RETRACTIONS.md` withdraws the
        // earlier claim that Kill shares variant 6 - "Kill is a circle-7 spell,
        // and it plays no dispatcher variant at all". Death Wind (44) and
        // Flame Wind (45) reach this handler from the mass-target family,
        // whose bare rumble is the same arithmetic at their own circle.
        //
        // Tremor (id 30) is not in either family: `§6.1` lists it among the 41
        // spells at variant = circle, "unconditional at helper entry".
        //
        // `§8.3`: for combat cursor spells, confirmation plays the spell effect
        // before the coordinate/projectile-impact resolver.
        //
        // §6.1 leaves **unresolved** "how many times the circle-scaled rumble
        // fires inside one combat spell resolution" - "once per accepted target
        // cursor is what the code shape suggests, but that is not established".
        // This handler resolves exactly one accepted cursor, so it fires once.
        match audio::spell_shared_variant(spell_index) {
            Some(variant) => self.emit_sound_effect(SoundEffect::SharedVariant { variant }),
            None => self.emit_sound_effect(SoundEffect::CircleRumbleLead {
                circle: audio::spell_circle(spell_index),
            }),
        }

        // Public clean-spec issue #132: protected Kill targets are rejected
        // only after the shared cast/resource and normal pre-effect envelope.
        // They bypass resistance and all target-death/effect work, but the
        // combat action is committed through the ordinary failure return.
        if matches!(kind, CombatSpellDamageKind::Kill)
            && target_actor
                .is_some_and(|actor| combat_class_is_protected_special(actor.owner_target_class))
        {
            self.advance_turn();
            self.message = "Failed!".to_string();
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
            return MoveOutcome::Blocked;
        }

        let resistance_blocked = matches!(kind, CombatSpellDamageKind::Kill)
            && self.combat_resistance_blocks(caster_index, target_slot);
        let raw_roll = self.combat_spell_damage_roll_for_kind(kind);
        let defense_roll = match kind {
            CombatSpellDamageKind::MagicMissile | CombatSpellDamageKind::Fireball => {
                self.combat_spell_target_defense_roll(target_slot)
            }
            CombatSpellDamageKind::Kill => 0,
            CombatSpellDamageKind::Tremor
            | CombatSpellDamageKind::DeathWind
            | CombatSpellDamageKind::FlameWind => 0,
        };
        let applied = if resistance_blocked {
            None
        } else {
            self.apply_active_target_combat_spell_damage(
                Some(caster_index),
                target_slot,
                kind,
                raw_roll,
                defense_roll,
            )
        };

        self.advance_turn();
        let succeeded = applied.is_some();
        self.message = match (kind, succeeded) {
            (CombatSpellDamageKind::MagicMissile, true) => "Magic Missile!".to_string(),
            (CombatSpellDamageKind::Fireball, true) => "Fireball!".to_string(),
            (CombatSpellDamageKind::Kill, true) => "Kill!".to_string(),
            _ => "Failed!".to_string(),
        };
        if succeeded && audio::spell_shared_variant(spell_index).is_none() {
            // `audio.md §6.1` combat effect template: "On a resolved effect it
            // adds a **descending** glissando, 20 updates from 1300 Hz down
            // toward 350 Hz." Only the template spells add it; the shared
            // variant closes with its envelope pair instead. §6.1 also fixes
            // which impact branch runs: "the area-of-effect branch always runs
            // and the projectile branch (a 400-to-750 Hz glissando) never
            // does", so no projectile sweep is emitted here.
            self.emit_sound_effect(SoundEffect::CombatTemplateImpact);
        }
        if !succeeded {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
        }
        if succeeded {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn apply_tremor_combat_spell_damage(
        &mut self,
        caster_slot: Option<usize>,
        gate_accepts: &[bool],
        damage_rolls: &[u8],
    ) -> Option<CombatTremorSpellDamageApplication> {
        let target_slots = collect_tremor_spell_actor_slots(&self.combat_actors, gate_accepts);
        if damage_rolls.len() < target_slots.len() {
            return None;
        }

        let mut applications = Vec::new();
        for (slot, roll) in target_slots
            .iter()
            .copied()
            .zip(damage_rolls.iter().copied())
        {
            let raw_damage = resolve_tremor_spell_raw_damage(roll);
            let damage_application =
                self.apply_combat_weapon_damage_to_target(caster_slot, slot, raw_damage, true)?;
            applications.push(CombatTremorSpellSlotDamageApplication {
                target_slot: slot,
                raw_damage,
                damage_application,
            });
        }

        Some(CombatTremorSpellDamageApplication { applications })
    }

    pub fn cast_tremor_combat_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6.1`: Tremor is id 30, circle 6 - "Unconditional at helper
        // entry". It is one of the 41 dispatcher spells, not a member of the
        // mass-target family, despite sweeping every slot.
        if let Some(variant) = audio::spell_shared_variant(spell_index) {
            self.emit_sound_effect(SoundEffect::SharedVariant { variant });
        }

        let mut applications = Vec::new();
        for slot in 0..self.combat_actors.len() {
            if !tremor_spell_actor_is_damageable(self.combat_actors[slot])
                || !self.combat_target_weight_gate_accepts(slot)
            {
                continue;
            }
            let roll = self.combat_spell_damage_roll_for_kind(CombatSpellDamageKind::Tremor);
            let raw_damage = resolve_tremor_spell_raw_damage(roll);
            if let Some(damage_application) = self.apply_combat_weapon_damage_to_target(
                Some(caster_index),
                slot,
                raw_damage,
                true,
            ) {
                applications.push(CombatTremorSpellSlotDamageApplication {
                    target_slot: slot,
                    raw_damage,
                    damage_application,
                });
            }
        }
        let applied = Some(CombatTremorSpellDamageApplication { applications });

        self.advance_turn();
        self.message = if applied
            .as_ref()
            .is_some_and(|application| !application.applications.is_empty())
        {
            "Tremor!".to_string()
        } else {
            "Tremor found no target.".to_string()
        };
        MoveOutcome::Cast
    }

    pub fn apply_directed_combat_spell_damage(
        &mut self,
        caster_slot: Option<usize>,
        effect: CombatDirectedSpellEffect,
        target_cells: &[(u8, u8)],
        damage_rolls: &[u8],
    ) -> Option<CombatDirectedSpellDamageApplication> {
        resolve_directed_spell_raw_damage(effect, 0)?;

        let target_slots = collect_directed_spell_actor_slots(&self.combat_actors, target_cells);
        if matches!(effect, CombatDirectedSpellEffect::FlameWind)
            && damage_rolls.len() < target_slots.len()
        {
            return None;
        }

        let credit_slot = if directed_spell_damage_credits_caster(effect) {
            caster_slot
        } else {
            None
        };
        let mut applications = Vec::new();
        for (index, slot) in target_slots.iter().copied().enumerate() {
            let roll = damage_rolls.get(index).copied().unwrap_or(0);
            let raw_damage = resolve_directed_spell_raw_damage(effect, roll)?;
            let damage_application =
                self.apply_combat_weapon_damage_to_target(credit_slot, slot, raw_damage, true)?;
            applications.push(CombatDirectedSpellSlotDamageApplication {
                target_slot: slot,
                raw_damage,
                damage_application,
            });
        }

        Some(CombatDirectedSpellDamageApplication {
            effect,
            applications,
        })
    }

    pub fn apply_directed_combat_spell_status(
        &mut self,
        effect: CombatDirectedSpellEffect,
        target_cells: &[(u8, u8)],
        poison_gate_accepts: &[bool],
        poison_damage_rolls: &[u8],
    ) -> Option<CombatDirectedSpellStatusApplication> {
        if !matches!(
            effect,
            CombatDirectedSpellEffect::Sleep | CombatDirectedSpellEffect::PoisonWind
        ) {
            return None;
        }

        let target_slots = collect_directed_spell_actor_slots(&self.combat_actors, target_cells);
        if matches!(effect, CombatDirectedSpellEffect::PoisonWind)
            && (poison_gate_accepts.len() < target_slots.len()
                || poison_damage_rolls.len() < target_slots.len())
        {
            return None;
        }

        let mut applications = Vec::new();
        for (index, slot) in target_slots.iter().copied().enumerate() {
            let application = match effect {
                CombatDirectedSpellEffect::Sleep if slot < COMBAT_PARTY_ACTOR_SLOTS => {
                    let outcome = apply_combat_sleep_to_party_target(self.party.get_mut(slot)?);
                    CombatDirectedSpellSlotStatusApplication::PartySleep {
                        target_slot: slot,
                        outcome,
                    }
                }
                CombatDirectedSpellEffect::Sleep => {
                    self.set_combat_actor_status_disabled(slot);
                    CombatDirectedSpellSlotStatusApplication::NonPartySleepDisabled {
                        target_slot: slot,
                    }
                }
                CombatDirectedSpellEffect::PoisonWind => {
                    if !poison_gate_accepts[index] {
                        CombatDirectedSpellSlotStatusApplication::PoisonGateRejected {
                            target_slot: slot,
                        }
                    } else {
                        let poison_damage_roll = poison_damage_rolls[index];
                        if slot < COMBAT_PARTY_ACTOR_SLOTS {
                            let outcome = apply_combat_poison_to_party_target(
                                self.party.get_mut(slot)?,
                                poison_damage_roll,
                            );
                            let fallback_damage_application = match outcome {
                                CombatPartyPoisonOutcome::FallbackDamage { raw_damage } => {
                                    Some(self.apply_combat_weapon_damage_to_target(
                                        None,
                                        slot,
                                        raw_damage as i16,
                                        true,
                                    )?)
                                }
                                CombatPartyPoisonOutcome::PoisonedPartyMember { .. } => None,
                            };
                            CombatDirectedSpellSlotStatusApplication::PartyPoison {
                                target_slot: slot,
                                outcome,
                                fallback_damage_application,
                            }
                        } else {
                            let raw_damage =
                                combat_field_poison_fallback_damage(poison_damage_roll) as i16;
                            let damage_application = self.apply_combat_weapon_damage_to_target(
                                None, slot, raw_damage, true,
                            )?;
                            CombatDirectedSpellSlotStatusApplication::NonPartyPoisonFallbackDamage {
                                target_slot: slot,
                                raw_damage,
                                damage_application,
                            }
                        }
                    }
                }
                CombatDirectedSpellEffect::DeathWind | CombatDirectedSpellEffect::FlameWind => {
                    return None;
                }
            };
            applications.push(application);
        }

        Some(CombatDirectedSpellStatusApplication {
            effect,
            applications,
        })
    }

    pub fn directed_combat_spell_target_cells(
        &self,
        caster_index: usize,
        direction: Direction,
        _effect: CombatDirectedSpellEffect,
    ) -> Option<Vec<(u8, u8)>> {
        let caster = self.combat_actors.get(caster_index).copied()?;
        if !combat_actor_is_active_not_dead(caster) || !direction.is_cardinal() {
            return None;
        }
        let mut cells = Vec::new();
        let cx = caster.x as i16;
        let cy = caster.y as i16;
        let max = (COMBAT_ARENA_SIDE - 1) as i16;
        let emit = |cells: &mut Vec<(u8, u8)>, x: i16, y: i16| -> bool {
            if (0..=max).contains(&x) && (0..=max).contains(&y) {
                let cell = (x as u8, y as u8);
                if !cells.contains(&cell) {
                    cells.push(cell);
                    return cells.len() == DIRECTED_WIND_MAX_CELLS;
                }
            }
            false
        };
        match direction {
            Direction::West => {
                for d in 1..=cx {
                    let x = cx - d;
                    for y in (cy - d)..=(cy + d) {
                        if emit(&mut cells, x, y) {
                            return Some(cells);
                        }
                    }
                }
            }
            Direction::East => {
                for d in 1..=(max - cx) {
                    let x = cx + d;
                    for y in (cy - d)..=(cy + d) {
                        if emit(&mut cells, x, y) {
                            return Some(cells);
                        }
                    }
                }
            }
            Direction::North => {
                for d in 1..=cy {
                    let y = cy - d;
                    for x in (cx - d)..=(cx + d) {
                        if emit(&mut cells, x, y) {
                            return Some(cells);
                        }
                    }
                }
            }
            Direction::South => {
                for d in 1..=(max - cy) {
                    let y = cy + d;
                    for x in (cx - d)..=(cx + d) {
                        if emit(&mut cells, x, y) {
                            return Some(cells);
                        }
                    }
                }
            }
            _ => return None,
        }
        Some(cells)
    }

    pub fn cast_directed_combat_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        effect: CombatDirectedSpellEffect,
        direction: Option<Direction>,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(direction) = direction else {
            self.message = "Direction? Use C1IZ6/C1HIN6/C1CGIV6/C1FHI6.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Directed spell requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(target_cells) =
            self.directed_combat_spell_target_cells(caster_index, direction, effect)
        else {
            self.message = "Who casts?".to_string();
            return MoveOutcome::Blocked;
        };

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        // `audio.md §6.1` mass-target family (Sleep 28, Poison Wind 40, Death
        // Wind 44, Flame Wind 45): "No dispatcher call. Instead: one bare
        // random rumble `(800, T, 700)`" with `T = 8000 + 1600 x circle`.
        //
        // The 21-by-21 raster that follows is deliberately **not** emitted:
        // §6.1 marks its rate unresolved - "entirely a function of the
        // cell-draw cost in the enclosing loop, which was not cycle-counted"
        // and "cannot be published as a frequency-versus-time curve" - so
        // there is no implementable contract for it yet.
        self.emit_sound_effect(SoundEffect::CircleRumbleLead {
            circle: audio::spell_circle(spell_index),
        });

        let target_slots = collect_directed_spell_actor_slots(&self.combat_actors, &target_cells);
        let applied = match effect {
            CombatDirectedSpellEffect::Sleep => {
                let mut affected = false;
                for slot in target_slots.iter().copied() {
                    if self.combat_resistance_blocks(caster_index, slot) {
                        continue;
                    }
                    if slot < COMBAT_PARTY_ACTOR_SLOTS {
                        if let Some(member) = self.party.get_mut(slot) {
                            let _ = apply_combat_sleep_to_party_target(member);
                            affected = true;
                        }
                    } else {
                        self.set_combat_actor_status_disabled(slot);
                        affected = true;
                    }
                }
                Some(affected)
            }
            CombatDirectedSpellEffect::PoisonWind => {
                let mut affected = false;
                for slot in target_slots.iter().copied() {
                    if !self.combat_target_weight_gate_accepts(slot) {
                        continue;
                    }
                    if slot < COMBAT_PARTY_ACTOR_SLOTS {
                        let needs_damage_roll = self
                            .party
                            .get(slot)
                            .is_some_and(|member| member.status != b'G' || member.hp == 0);
                        let damage_roll = if needs_damage_roll {
                            self.combat_arena_field_poison_damage_roll()
                        } else {
                            0
                        };
                        if let Some(member) = self.party.get_mut(slot) {
                            let outcome = apply_combat_poison_to_party_target(member, damage_roll);
                            if let CombatPartyPoisonOutcome::FallbackDamage { raw_damage } = outcome
                            {
                                let _ = self.apply_combat_weapon_damage_to_target(
                                    None,
                                    slot,
                                    raw_damage as i16,
                                    true,
                                );
                            }
                            affected = true;
                        }
                    } else {
                        let raw_damage = combat_field_poison_fallback_damage(
                            self.combat_arena_field_poison_damage_roll(),
                        ) as i16;
                        let _ =
                            self.apply_combat_weapon_damage_to_target(None, slot, raw_damage, true);
                        affected = true;
                    }
                }
                Some(affected)
            }
            CombatDirectedSpellEffect::DeathWind => {
                let mut affected = false;
                for slot in target_slots.iter().copied() {
                    if self.combat_resistance_blocks(caster_index, slot) {
                        continue;
                    }
                    affected |= self
                        .apply_combat_weapon_damage_to_target(
                            Some(caster_index),
                            slot,
                            COMBAT_INSTANT_KILL_DAMAGE,
                            true,
                        )
                        .is_some();
                }
                Some(affected)
            }
            CombatDirectedSpellEffect::FlameWind => {
                let damage_rolls = target_slots
                    .iter()
                    .map(|_| {
                        self.combat_spell_damage_roll_for_kind(CombatSpellDamageKind::FlameWind)
                    })
                    .collect::<Vec<_>>();
                self.apply_directed_combat_spell_damage(
                    Some(caster_index),
                    effect,
                    &target_cells,
                    &damage_rolls,
                )
                .map(|application| !application.applications.is_empty())
            }
        };

        self.advance_turn();
        self.message = match (effect, applied.unwrap_or(false)) {
            (CombatDirectedSpellEffect::Sleep, true) => "Sleep!".to_string(),
            (CombatDirectedSpellEffect::PoisonWind, true) => "Poison wind!".to_string(),
            (CombatDirectedSpellEffect::DeathWind, true) => "Death wind!".to_string(),
            (CombatDirectedSpellEffect::FlameWind, true) => "Flame wind!".to_string(),
            _ => "Failed!".to_string(),
        };
        if !applied.unwrap_or(false) {
            // `audio.md §8.3`: after `Failed!`, the common spell failure tail.
            self.emit_sound_effect(SoundEffect::CastFailure);
        }
        if applied.unwrap_or(false) {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn cast_repel_undead(&mut self, caster_index: usize) -> MoveOutcome {
        if !self.combat_active
            || !self.spell_allowed_in_current_cast_context(REPEL_UNDEAD_SPELL_INDEX)
        {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, REPEL_UNDEAD_SPELL_INDEX, REPEL_UNDEAD_COST)
        {
            return outcome;
        }

        // `audio.md §6.1`: Repel Undead is id 7, circle 2 - "Unconditional at
        // helper entry", so it sounds even when no undead is present.
        self.emit_sound_effect(SoundEffect::SharedVariant {
            variant: audio::spell_circle(REPEL_UNDEAD_SPELL_INDEX),
        });

        let accepted = collect_repel_undead_actor_slots(&self.combat_actors)
            .into_iter()
            .filter(|slot| !self.combat_resistance_blocks(caster_index, *slot))
            .collect::<Vec<_>>();
        let affected = apply_cause_fear_critical_hp_setup(&mut self.combat_actors, &accepted);

        self.advance_turn();
        self.message = if affected == 0 {
            "Repel Undead found no target.".to_string()
        } else {
            format!("Repel Undead! {affected} undead repelled.")
        };
        MoveOutcome::Cast
    }

    pub fn apply_combat_arena_field_contact(
        &mut self,
        field: CombatArenaFieldKind,
        target_slot: usize,
        poison_damage_roll: u8,
        fire_damage_roll: u8,
    ) -> Option<CombatArenaFieldContactApplication> {
        let actor = self.combat_actors.get(target_slot)?;
        let linked_active_object_tile = self
            .active_objects
            .get(actor.active_object_slot as usize)?
            .tile;

        let contact_outcome = if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            resolve_combat_arena_field_contact_for_party_target(
                field,
                linked_active_object_tile,
                self.party.get_mut(target_slot)?,
                poison_damage_roll,
                fire_damage_roll,
            )?
        } else {
            resolve_combat_arena_field_contact_for_non_party_target(
                field,
                linked_active_object_tile,
                poison_damage_roll,
                fire_damage_roll,
            )?
        };

        if matches!(
            contact_outcome,
            CombatArenaFieldContactOutcome::SleepDisabledNonParty
        ) {
            self.set_combat_actor_status_disabled(target_slot);
        }

        let damage_application = match contact_outcome {
            CombatArenaFieldContactOutcome::PoisonFallbackDamage { raw_damage } => {
                Some(self.apply_combat_weapon_damage_to_target(
                    None,
                    target_slot,
                    raw_damage as i16,
                    true,
                )?)
            }
            CombatArenaFieldContactOutcome::FireDamage { raw_damage } => {
                Some(self.apply_combat_weapon_damage_to_target(
                    None,
                    target_slot,
                    raw_damage as i16,
                    true,
                )?)
            }
            CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass
            | CombatArenaFieldContactOutcome::PoisonedPartyMember { .. }
            | CombatArenaFieldContactOutcome::SleepSkippedDeadParty
            | CombatArenaFieldContactOutcome::SleptPartyMember { .. }
            | CombatArenaFieldContactOutcome::SleepDisabledNonParty => None,
        };

        Some(CombatArenaFieldContactApplication {
            field,
            target_slot,
            contact_outcome,
            damage_application,
        })
    }

    fn apply_combat_selected_field_contact_for_actor_position(
        &mut self,
        actor_slot: usize,
        field: CombatArenaFieldKind,
    ) -> Option<CombatArenaFieldContactApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_present_not_dead(actor) {
            return None;
        }
        if field == CombatArenaFieldKind::Energy {
            return None;
        }
        let linked_tile = self
            .active_objects
            .get(actor.active_object_slot as usize)?
            .tile;
        let poison_damage_roll = if field == CombatArenaFieldKind::Poison
            && linked_tile < 0x80
            // `combat.md §5`: read the roster through the descriptor's
            // owner/target/class byte, not through the descriptor index.
            && (actor_slot >= COMBAT_PARTY_ACTOR_SLOTS
                || self
                    .combat_roster_slot_for_actor_slot(actor_slot)
                    .and_then(|roster_slot| self.party.get(roster_slot))?
                    .status
                    != b'G')
        {
            self.random_range_u8(0, 20)
        } else {
            0
        };
        let fire_damage_roll = if field == CombatArenaFieldKind::Fire {
            self.random_range_u8(0, 10)
        } else {
            0
        };
        let application = self.apply_combat_arena_field_contact(
            field,
            actor_slot,
            poison_damage_roll,
            fire_damage_roll,
        )?;
        self.mark_visibility_dirty();
        Some(application)
    }

    pub fn apply_combat_arena_field_contact_for_actor_position(
        &mut self,
        actor_slot: usize,
    ) -> Option<CombatArenaFieldContactApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        let (_, field) = self.find_combat_arena_field_marker_excluding(
            actor.x,
            actor.y,
            Some(actor.active_object_slot as usize),
        )?;
        self.apply_combat_selected_field_contact_for_actor_position(actor_slot, field)
    }

    pub fn apply_combat_post_dispatch_contact_for_actor_position(
        &mut self,
        actor_slot: usize,
    ) -> Option<CombatPostDispatchContactApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_present_not_dead(actor) {
            return None;
        }
        let terrain = *self
            .combat_terrain
            .get(usize::from(actor.y))?
            .get(usize::from(actor.x))?;
        if let Some(field) = combat_arena_terrain_contact_kind(terrain) {
            let field_contact =
                self.apply_combat_selected_field_contact_for_actor_position(actor_slot, field)?;
            return Some(CombatPostDispatchContactApplication {
                source: CombatPostDispatchContactSource::ArenaTerrain { tile: terrain },
                field_contact,
            });
        }
        let (active_object_slot, field) = self.find_combat_arena_field_marker_excluding(
            actor.x,
            actor.y,
            Some(actor.active_object_slot as usize),
        )?;
        if field == CombatArenaFieldKind::Energy {
            return None;
        }
        let field_contact =
            self.apply_combat_selected_field_contact_for_actor_position(actor_slot, field)?;
        Some(CombatPostDispatchContactApplication {
            source: CombatPostDispatchContactSource::PlacedMarker { active_object_slot },
            field_contact,
        })
    }

    pub fn apply_combat_ambush_reveal_for_actor_position(
        &mut self,
        actor_slot: usize,
    ) -> Option<CombatAmbushRevealApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_active_not_dead(actor) {
            return None;
        }
        let application = apply_combat_ambush_reveal_records(
            &mut self.combat_ambush_reveals,
            &mut self.combat_terrain,
            actor.x,
            actor.y,
        )?;
        self.mark_visibility_dirty();
        Some(application)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_and_apply_combat_equipment_weapon_attack(
        &mut self,
        item_id: usize,
        attacker_slot: usize,
        target_slot: usize,
        attacker_rating: u8,
        defender_rating: u8,
        hit_roll: u8,
        damage_roll: u8,
        forced_hit: Option<bool>,
        magical: bool,
    ) -> Option<CombatWeaponAttackApplication> {
        let attacker = *self.combat_actors.get(attacker_slot)?;
        let target = *self.combat_actors.get(target_slot)?;
        let resolution = resolve_combat_equipment_weapon_attack(
            item_id,
            attacker.range_to(target),
            attacker_rating,
            defender_rating,
            hit_roll,
            damage_roll,
            forced_hit,
        )?;
        let damage_application = match resolution {
            CombatWeaponAttackResolution::Hit { raw_damage, .. } => self
                .apply_combat_weapon_damage_to_target(
                    Some(attacker_slot),
                    target_slot,
                    raw_damage,
                    magical,
                ),
            CombatWeaponAttackResolution::OutOfRange { .. }
            | CombatWeaponAttackResolution::NoOrdinaryDamage { .. }
            | CombatWeaponAttackResolution::Miss { .. }
            | CombatWeaponAttackResolution::Special { .. } => None,
        };

        Some(CombatWeaponAttackApplication {
            resolution,
            damage_application,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn resolve_and_apply_combat_monster_attack(
        &mut self,
        attacker_slot: usize,
        target_slot: usize,
        party_defender_rating: u8,
        hit_roll: u8,
        damage_roll: u8,
        poison_gate_accepts: bool,
        poison_damage_roll: u8,
        forced_hit: Option<bool>,
    ) -> Option<CombatMonsterAttackApplication> {
        if attacker_slot < COMBAT_PARTY_ACTOR_SLOTS || attacker_slot >= COMBAT_ACTOR_SLOTS {
            return None;
        }
        let attacker = *self.combat_actors.get(attacker_slot)?;
        let target = *self.combat_actors.get(target_slot)?;
        if !combat_actor_is_active_not_dead(attacker) || !combat_actor_is_active_not_dead(target) {
            return None;
        }

        let attacker_stats = combat_class_stats(attacker.owner_target_class)?;
        let ranged = combat_ranged_effect_stats(attacker.owner_target_class)?;
        let target_range = attacker.range_to(target);
        // `magic.md §7`: ordinary automatic adjacent attacks install their
        // source before the hit test, so misses and poison-special returns
        // record exactly like ordinary hits. `combat.md §6.1a`: the
        // controlled/charmed actor's fixed magic strike explicitly skips the
        // attacker back-link, so it never writes this map.
        if target_range == 1 && !attacker.is_controlled() {
            self.combat_interference_sources[target_slot] = attacker_slot as u8;
        }
        let defender_rating = if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            party_defender_rating
        } else {
            combat_class_stats(target.owner_target_class)?.defense
        };

        // `combat.md §6.1a` "Readers — the attack driver": a controlled actor's
        // attack is a fixed magic strike resolved by the shared
        // attack-application primitive. It requires straight-line distance
        // exactly one, and the pre-attack animation, the attacker back-link
        // (skipped above), the class-specific attack overrides, the
        // poison/status branch and the monster ranged-spell branch are all
        // skipped. Only the shared to-hit roll, damage feeder and hit/miss
        // narration run. The renderer's fixed magic-strike action-tile id is
        // not published in any current spec document, so no tile marker is
        // stamped here; the damage is fed as magical because the published
        // flavour is a magic strike, not a weapon blow.
        if attacker.is_controlled() {
            if target_range != 1 {
                return None;
            }
            let resolution = resolve_combat_weapon_attack(
                attacker_stats.attack_cap,
                target_range,
                1,
                0,
                attacker_stats.attack_cap,
                defender_rating,
                hit_roll,
                damage_roll,
                forced_hit,
            );
            let damage_application = match resolution {
                CombatWeaponAttackResolution::Hit { raw_damage, .. } => {
                    self.apply_combat_weapon_damage_to_target(None, target_slot, raw_damage, true)
                }
                CombatWeaponAttackResolution::OutOfRange { .. }
                | CombatWeaponAttackResolution::NoOrdinaryDamage { .. }
                | CombatWeaponAttackResolution::Miss { .. }
                | CombatWeaponAttackResolution::Special { .. } => None,
            };
            return Some(CombatMonsterAttackApplication {
                attacker_slot,
                target_slot,
                poison_status_outcome: None,
                resolution: Some(resolution),
                damage_application,
            });
        }

        let mut poison_status_outcome = None;
        if target_range <= 1 {
            let fallback_raw_damage = combat_field_poison_fallback_damage(poison_damage_roll);
            let poison_outcome = if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
                resolve_poison_status_attack_for_party_target(
                    attacker.owner_target_class,
                    self.party.get_mut(target_slot)?,
                    poison_gate_accepts,
                    fallback_raw_damage,
                )?
            } else if combat_class_traits(attacker.owner_target_class)
                .is_some_and(|traits| traits.poison_status_attack)
            {
                if poison_gate_accepts {
                    CombatPoisonStatusAttackOutcome::FallbackDamage {
                        raw_damage: fallback_raw_damage,
                    }
                } else {
                    CombatPoisonStatusAttackOutcome::GateRejected
                }
            } else {
                CombatPoisonStatusAttackOutcome::NotPoisonStatusClass
            };

            match poison_outcome {
                CombatPoisonStatusAttackOutcome::PoisonedPartyMember { .. } => {
                    return Some(CombatMonsterAttackApplication {
                        attacker_slot,
                        target_slot,
                        poison_status_outcome: Some(poison_outcome),
                        resolution: None,
                        damage_application: None,
                    });
                }
                CombatPoisonStatusAttackOutcome::FallbackDamage { raw_damage } => {
                    let damage_application = self.apply_combat_weapon_damage_to_target(
                        None,
                        target_slot,
                        raw_damage as i16,
                        true,
                    );
                    return Some(CombatMonsterAttackApplication {
                        attacker_slot,
                        target_slot,
                        poison_status_outcome: Some(poison_outcome),
                        resolution: None,
                        damage_application,
                    });
                }
                CombatPoisonStatusAttackOutcome::NotPoisonStatusClass
                | CombatPoisonStatusAttackOutcome::GateRejected => {
                    poison_status_outcome = Some(poison_outcome);
                }
            }
        }

        let resolution = resolve_combat_weapon_attack(
            attacker_stats.attack_cap,
            target_range,
            ranged.range_effect_selector,
            ranged.payload,
            attacker_stats.attack_cap,
            defender_rating,
            hit_roll,
            damage_roll,
            forced_hit,
        );
        let damage_application = match resolution {
            CombatWeaponAttackResolution::Hit { raw_damage, .. } => {
                self.apply_combat_weapon_damage_to_target(None, target_slot, raw_damage, false)
            }
            CombatWeaponAttackResolution::OutOfRange { .. }
            | CombatWeaponAttackResolution::NoOrdinaryDamage { .. }
            | CombatWeaponAttackResolution::Miss { .. }
            | CombatWeaponAttackResolution::Special { .. } => None,
        };

        Some(CombatMonsterAttackApplication {
            attacker_slot,
            target_slot,
            poison_status_outcome,
            resolution: Some(resolution),
            damage_application,
        })
    }

    pub fn combat_monster_amulet_turning_scatter_applies(
        &mut self,
        attacker_slot: usize,
        target_slot: usize,
    ) -> bool {
        if target_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(attacker) = self.combat_actors.get(attacker_slot).copied() else {
            return false;
        };
        // `combat.md §5`: the defender's roster record and equipment are
        // reached through the descriptor's owner/target/class byte.
        let Some(roster_slot) = self.combat_roster_slot_for_actor_slot(target_slot) else {
            return false;
        };
        let Some(target) = self.party.get(roster_slot).copied() else {
            return false;
        };
        let Some(equipment) = self.party_equipment.get(roster_slot).copied() else {
            return false;
        };
        let roll = self.combat_monster_amulet_turning_roll(attacker_slot, target_slot);
        resolve_amulet_turning_scatter_for_party_target(
            attacker.owner_target_class,
            target,
            &equipment,
            roll,
        )
        .unwrap_or(false)
    }

    pub fn combat_actor_at_scatter_impact(&self, x: u8, y: u8) -> Option<usize> {
        self.combat_actors
            .iter()
            .copied()
            .enumerate()
            .find(|(_, actor)| {
                actor.x == x
                    && actor.y == y
                    && combat_actor_is_active_not_dead(*actor)
                    && !actor.is_hidden_or_unrevealed()
            })
            .map(|(slot, _)| slot)
    }

    pub fn resolve_and_apply_combat_monster_scattered_attack(
        &mut self,
        attacker_slot: usize,
        intended_target_slot: usize,
        party_defender_rating: u8,
        hit_roll: u8,
        damage_roll: u8,
        scatter_roll: u8,
    ) -> Option<CombatMonsterAttackApplication> {
        if attacker_slot < COMBAT_PARTY_ACTOR_SLOTS || attacker_slot >= COMBAT_ACTOR_SLOTS {
            return None;
        }
        let attacker = *self.combat_actors.get(attacker_slot)?;
        let intended = *self.combat_actors.get(intended_target_slot)?;
        if !combat_actor_is_active_not_dead(attacker) || !combat_actor_is_active_not_dead(intended)
        {
            return None;
        }
        let attacker_stats = combat_class_stats(attacker.owner_target_class)?;
        let ranged = combat_ranged_effect_stats(attacker.owner_target_class)?;
        let (impact_x, impact_y) = resolve_amulet_turning_scatter_cell(
            intended.x,
            intended.y,
            attacker.x,
            attacker.y,
            scatter_roll,
        );
        let route = CombatWeaponAttackRangeRoute::Ranged {
            effect_code: ranged.payload,
        };
        if !(0..COMBAT_ARENA_SIDE as i8).contains(&impact_x)
            || !(0..COMBAT_ARENA_SIDE as i8).contains(&impact_y)
        {
            return Some(CombatMonsterAttackApplication {
                attacker_slot,
                target_slot: intended_target_slot,
                poison_status_outcome: None,
                resolution: Some(CombatWeaponAttackResolution::Miss {
                    route,
                    hit_score: 0,
                }),
                damage_application: None,
            });
        }

        let impact_x = impact_x as u8;
        let impact_y = impact_y as u8;
        let Some(target_slot) = self.combat_actor_at_scatter_impact(impact_x, impact_y) else {
            return Some(CombatMonsterAttackApplication {
                attacker_slot,
                target_slot: intended_target_slot,
                poison_status_outcome: None,
                resolution: Some(CombatWeaponAttackResolution::Miss {
                    route,
                    hit_score: 0,
                }),
                damage_application: None,
            });
        };
        let defender_rating = if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            party_defender_rating
        } else {
            combat_class_stats(self.combat_actors[target_slot].owner_target_class)?.defense
        };
        let impact_range = combat_arena_range(attacker.x, attacker.y, impact_x, impact_y).max(2);
        let resolution = resolve_combat_weapon_attack(
            attacker_stats.attack_cap,
            impact_range,
            ranged.range_effect_selector,
            ranged.payload,
            attacker_stats.attack_cap,
            defender_rating,
            hit_roll,
            damage_roll,
            Some(true),
        );
        let damage_application = match resolution {
            CombatWeaponAttackResolution::Hit { raw_damage, .. } => {
                self.apply_combat_weapon_damage_to_target(None, target_slot, raw_damage, false)
            }
            CombatWeaponAttackResolution::OutOfRange { .. }
            | CombatWeaponAttackResolution::NoOrdinaryDamage { .. }
            | CombatWeaponAttackResolution::Miss { .. }
            | CombatWeaponAttackResolution::Special { .. } => None,
        };

        Some(CombatMonsterAttackApplication {
            attacker_slot,
            target_slot,
            poison_status_outcome: None,
            resolution: Some(resolution),
            damage_application,
        })
    }

    pub fn apply_combat_active_player_digit(
        &mut self,
        key: char,
    ) -> CombatActivePlayerSelectionOutcome {
        match resolve_combat_active_player_digit(key) {
            CombatActivePlayerSelectionOutcome::Clear => {
                self.active_player = None;
                CombatActivePlayerSelectionOutcome::Clear
            }
            CombatActivePlayerSelectionOutcome::SelectPartySlot(slot)
                if slot < self.party.len() && slot < COMBAT_PARTY_ACTOR_SLOTS =>
            {
                self.active_player = Some(slot);
                CombatActivePlayerSelectionOutcome::SelectPartySlot(slot)
            }
            CombatActivePlayerSelectionOutcome::SelectPartySlot(_)
            | CombatActivePlayerSelectionOutcome::Invalid => {
                CombatActivePlayerSelectionOutcome::Invalid
            }
        }
    }

    pub fn combat_destination_walkable_for_direction(
        &self,
        actor_slot: usize,
        direction_code: u8,
    ) -> Option<bool> {
        let actor = *self.combat_actors.get(actor_slot)?;
        let destination = resolve_combat_step_destination(actor.x, actor.y, direction_code);
        if !destination.in_bounds {
            return Some(false);
        }
        if self.active_objects.iter().take(OOL_SLOTS).any(|object| {
            object.type_byte == COMBAT_FIELD_KIND_ENERGY
                && object.x == destination.x as usize
                && object.y == destination.y as usize
        }) {
            return Some(false);
        }
        Some(is_combat_arena_tile_walkable(
            self.combat_terrain[destination.y as usize][destination.x as usize],
        ))
    }

    /// `combat.md §8`: the player's command handler reads only the Negate
    /// Magic tag. There is exactly one Quickness gate and it sits at the head
    /// of the automatic actor driver, so Quickness makes hostiles act about
    /// half as often - it never turns the player's own turn into a coin flip.
    pub fn apply_combat_player_command_with_inputs(
        &mut self,
        actor_slot: usize,
        input: CombatPlayerCommandInput,
    ) -> Option<CombatPlayerCommandApplication> {
        let weapon_attack_inputs = if matches!(
            input,
            CombatPlayerCommandInput::Direction(_) | CombatPlayerCommandInput::AttackDirection(_)
        ) {
            self.combat_player_weapon_attack_inputs(actor_slot)
        } else {
            CombatPlayerWeaponAttackInputs::default()
        };
        self.apply_combat_player_command_with_attack_inputs(actor_slot, input, weapon_attack_inputs)
    }

    pub fn combat_player_weapon_attack_inputs(
        &mut self,
        attacker_slot: usize,
    ) -> CombatPlayerWeaponAttackInputs {
        let _ = attacker_slot;
        CombatPlayerWeaponAttackInputs {
            hit_roll: self.random_range_u8(0, u8::MAX),
            damage_roll: self.random_range_u8(0, u8::MAX),
            forced_hit: None,
        }
    }

    pub fn combat_quickness_dispatch_roll(&mut self, actor_slot: usize) -> u8 {
        let _ = actor_slot;
        if active_effect_is_active(
            self.active_effect_tag,
            self.active_effect_counter,
            QUICKNESS_ACTIVE_EFFECT_TAG,
        ) {
            self.random_mod_u8(2)
        } else {
            1
        }
    }

    pub fn combat_magic_ring_regeneration_roll(&mut self, actor_slot: usize) -> u8 {
        let _ = actor_slot;
        self.random_mod_u8(8)
    }

    pub fn combat_magic_ring_vanish_roll(&mut self, actor_slot: usize) -> u8 {
        let _ = actor_slot;
        self.random_mod_u8(16)
    }

    pub fn apply_visible_combat_magic_ring_pass_to_slot(
        &mut self,
        slot: usize,
    ) -> Option<CombatMagicRingPassOutcome> {
        let acting_actor = self.combat_actors.get(slot).copied()?;
        if acting_actor.flags & COMBAT_ACTOR_FLAG_SELECTABLE_80 == 0
            || acting_actor.is_marked_dead()
        {
            return None;
        }
        let wearer_slot = acting_actor.owner_target_class as usize;
        let ring = *self
            .party_equipment
            .get(wearer_slot)?
            .get(EQUIP_SLOT_RING)?;
        if ring != EQUIPMENT_ID_RING_INVISIBILITY as u8
            && ring != EQUIPMENT_ID_RING_REGENERATION as u8
        {
            return None;
        }

        let mut outcome = CombatMagicRingPassOutcome::default();
        if ring == EQUIPMENT_ID_RING_INVISIBILITY as u8 {
            outcome.invisibility_applied = apply_combat_linked_invisibility(
                &mut self.combat_actors[slot],
                &mut self.active_objects,
            )
            .is_some_and(CombatLinkedVisibilityOutcome::changed);
            if outcome.invisibility_applied {
                self.mark_visibility_dirty();
            }
        } else {
            let eligible_wearers: Vec<usize> = self
                .combat_actors
                .iter()
                .copied()
                .filter(|actor| {
                    actor.flags & COMBAT_ACTOR_FLAG_SELECTABLE_80 != 0 && !actor.is_marked_dead()
                })
                .map(|actor| actor.owner_target_class as usize)
                .filter(|&party_slot| {
                    self.party
                        .get(party_slot)
                        .is_some_and(|member| member.living())
                        && self
                            .party_equipment
                            .get(party_slot)
                            .is_some_and(|equipment| {
                                equipment[EQUIP_SLOT_RING] == EQUIPMENT_ID_RING_REGENERATION as u8
                            })
                })
                .collect();
            for party_slot in eligible_wearers {
                let regeneration_roll = self.combat_magic_ring_regeneration_roll(party_slot);
                if regeneration_roll == 0 {
                    outcome.regeneration_applied = outcome
                        .regeneration_applied
                        .saturating_add(self.party[party_slot].heal_by(1));
                }
            }
        }
        (outcome != CombatMagicRingPassOutcome::default()).then_some(outcome)
    }

    pub fn apply_combat_player_command_with_attack_inputs(
        &mut self,
        actor_slot: usize,
        input: CombatPlayerCommandInput,
        weapon_attack_inputs: CombatPlayerWeaponAttackInputs,
    ) -> Option<CombatPlayerCommandApplication> {
        if !self.combat_active {
            return None;
        }
        let active_actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_active_not_dead(active_actor) {
            return None;
        }
        // `combat.md §6.1a`: the round walker picks the keystroke path
        // through the slot-to-group helper, so a party-side actor
        // carrying the controlled bit runs on the automatic driver.
        // `magic.md §8`: "Nothing routes a summoned creature through the
        // player command parser, and the player never gets to move it."
        if !combat_slot_takes_player_command_path(actor_slot, active_actor) {
            return None;
        }

        let action = match input {
            CombatPlayerCommandInput::Direction(direction_code)
            | CombatPlayerCommandInput::AttackDirection(direction_code) => {
                if !combat_direction_code_is_cardinal(direction_code) {
                    CombatPlayerCommandAction::InvalidDirection { direction_code }
                } else {
                    let attacker_group = self.combat_target_group_for_slot(actor_slot);
                    let destination_walkable =
                        self.combat_destination_walkable_for_direction(actor_slot, direction_code)?;
                    let outcome = self.apply_combat_step_or_attack_primitive(
                        actor_slot,
                        attacker_group,
                        direction_code,
                        destination_walkable,
                    );
                    CombatPlayerCommandAction::StepOrAttack {
                        prompted_attack: matches!(
                            input,
                            CombatPlayerCommandInput::AttackDirection(_)
                        ),
                        direction_code,
                        outcome,
                    }
                }
            }
            CombatPlayerCommandInput::Key(key) => match resolve_combat_active_player_digit(key) {
                CombatActivePlayerSelectionOutcome::Clear
                | CombatActivePlayerSelectionOutcome::SelectPartySlot(_) => {
                    CombatPlayerCommandAction::ActivePlayerSelection(
                        self.apply_combat_active_player_digit(key),
                    )
                }
                CombatActivePlayerSelectionOutcome::Invalid => {
                    match resolve_combat_command_branch(key) {
                        CombatCommandBranch::Pass => {
                            CombatPlayerCommandAction::Pass(resolve_combat_pass_command())
                        }
                        CombatCommandBranch::Attack => {
                            CombatPlayerCommandAction::PromptForAttackDirection
                        }
                        CombatCommandBranch::EscapeCleanup => {
                            CombatPlayerCommandAction::EscapeCleanup {
                                application: self.apply_combat_escape_cleanup(),
                            }
                        }
                        branch => CombatPlayerCommandAction::Branch {
                            branch,
                            live_actor_gate: resolve_combat_command_live_actor_gate(
                                branch,
                                Some(active_actor),
                            ),
                        },
                    }
                }
            },
        };

        // `audio.md §7.4`: the first of combat's two blocked-step sites, "the
        // step-or-attack refusal". §7.4 leaves **unresolved** whether a missed
        // attack roll also beeps - "two private analyses of that query directly
        // contradict each other" - so this stays on the refusal outcomes the
        // contract does name and adds no beep to a resolved swing that misses.
        if matches!(
            action,
            CombatPlayerCommandAction::StepOrAttack {
                outcome: CombatStepOrAttackPrimitiveOutcome::BlockedActor { .. }
                    | CombatStepOrAttackPrimitiveOutcome::BlockedWall,
                ..
            }
        ) {
            self.emit_sound_effect(SoundEffect::BlockedStep);
        }

        // `audio.md §8.8`: the combat command refused as inapplicable. The
        // twelve verbs the combat scene does not implement reach one shared
        // responder, and "this event is the only thing that produces" the
        // 220/150 Hz pair. It is **not** the blocked-step recipe above and
        // "must not be conflated with it" (`§7.4`).
        //
        // Scope is combat scenes only and all of them - "overworld-triggered,
        // town-triggered and dungeon-room combat alike" - which this site gets
        // for free by sitting in the combat command dispatch.
        //
        // The tail varies and the sound does not: "All three arms, and the
        // out-of-range fall-through, play the identical two-tone pair."
        // `combat_command_refusal_sounds` is the published key gate, so
        // `DWhatRefusal`, `WWhatRefusal` and `Invalid` stay silent (`§9`).
        //
        // No turn cost: `combat_player_command_action_reprompts` already
        // returns `true` for `SceneMessageAbort`, so the same combatant is
        // re-prompted and the committed-action tail is skipped.
        if let CombatPlayerCommandAction::Branch {
            branch: CombatCommandBranch::SceneMessageAbort(verb),
            ..
        } = action
            && audio::combat_command_refusal_sounds(combat_scene_abort_verb_key(verb))
        {
            self.emit_sound_effect(SoundEffect::CombatCommandRefused);
        }

        let out_of_arena_leave = match action {
            CombatPlayerCommandAction::StepOrAttack {
                direction_code,
                outcome: CombatStepOrAttackPrimitiveOutcome::OutOfArena { .. },
                ..
            } => Some(self.apply_combat_out_of_arena_leave(actor_slot, direction_code)),
            _ => None,
        };

        // `audio.md §7.4`: the second combat site, "the out-of-arena exit
        // refusal that prints `All must use the same exit!`". The third arm,
        // `Stay with ship!`, is silent (`§9`).
        if audio::combat_out_of_arena_refusal_beeps(matches!(
            out_of_arena_leave.map(|application| application.outcome),
            Some(CombatOutOfArenaLeaveOutcome::RefusedConstrainedDirection { .. })
        )) {
            self.emit_sound_effect(SoundEffect::BlockedStep);
        }

        let mut control_after = match action {
            CombatPlayerCommandAction::EscapeCleanup {
                application:
                    CombatEscapeCleanupApplication {
                        decision: CombatEscapeCleanupDecision::Accepted,
                        ..
                    },
            } => CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat),
            _ => self.combat_round_loop_control(false, false),
        };
        let weapon_attack = self.apply_combat_player_weapon_attack_for_action(
            actor_slot,
            &action,
            weapon_attack_inputs,
        );
        let edge_refused = out_of_arena_leave.is_some_and(|application| {
            !matches!(
                application.outcome,
                CombatOutOfArenaLeaveOutcome::Accepted { .. }
            )
        });
        let reprompt = combat_player_command_action_reprompts(&action) || edge_refused;
        // `combat.md §8` places both hooks in the committed non-digit action
        // tail. Multi-stage commands defer that tail until their continuation
        // closes; free refusals and actor-selection digits bypass it entirely.
        let digit_selection = matches!(action, CombatPlayerCommandAction::ActivePlayerSelection(_));
        let maintenance_deferred = combat_player_command_action_defers_maintenance(&action);
        let (absorbable_contact, post_dispatch_contact, ring_pass, active_effect_age) =
            if reprompt || digit_selection || maintenance_deferred {
                (None, None, None, None)
            } else {
                let absorbable_contact =
                    self.apply_combat_absorbable_field_contact_for_actor_position(actor_slot);
                let post_dispatch_contact =
                    self.apply_combat_post_dispatch_contact_for_actor_position(actor_slot);
                self.clear_combat_interference_for_completed_action(actor_slot);
                (
                    absorbable_contact,
                    post_dispatch_contact,
                    self.apply_visible_combat_magic_ring_pass_to_slot(actor_slot),
                    Some(self.age_active_effect()),
                )
            };
        if post_dispatch_contact.is_some() {
            let leave_combat = matches!(
                control_after,
                CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
            );
            control_after = self.combat_round_loop_control(leave_combat, false);
        }
        if matches!(control_after, CombatRoundLoopControl::ContinueActorWalk) {
            control_after = self.combat_round_loop_control(false, false);
        }
        let victory_announced = !reprompt
            && matches!(control_after, CombatRoundLoopControl::ContinueActorWalk)
            && self.announce_combat_victory_if_needed();

        Some(CombatPlayerCommandApplication {
            actor_slot,
            input,
            action,
            weapon_attack,
            ring_pass,
            active_effect_age,
            absorbable_contact,
            post_dispatch_contact,
            out_of_arena_leave,
            victory_announced,
            reprompt,
            control_after,
        })
    }

    pub fn apply_combat_player_weapon_attack_for_action(
        &mut self,
        actor_slot: usize,
        action: &CombatPlayerCommandAction,
        inputs: CombatPlayerWeaponAttackInputs,
    ) -> Option<CombatWeaponAttackApplication> {
        let target_slot = match action {
            CombatPlayerCommandAction::StepOrAttack {
                outcome: CombatStepOrAttackPrimitiveOutcome::Attack { target_slot },
                ..
            } => *target_slot,
            _ => return None,
        };
        let item_id = self
            .party_equipment
            .get(actor_slot)?
            .get(EQUIP_SLOT_WEAPON)
            .copied()?;
        if item_id == EQUIPMENT_EMPTY {
            return None;
        }
        let attacker_rating = self
            .party_strengths
            .get(actor_slot)
            .copied()
            .unwrap_or(AVATAR_STAT_MAX);
        let defender_rating = if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            7
        } else {
            combat_class_stats(self.combat_actors.get(target_slot)?.owner_target_class)?.defense
        };
        self.resolve_and_apply_combat_equipment_weapon_attack(
            item_id as usize,
            actor_slot,
            target_slot,
            attacker_rating,
            defender_rating,
            inputs.hit_roll,
            inputs.damage_roll,
            inputs.forced_hit,
            false,
        )
    }

    /// Apply one geometric arena-edge attempt. Acceptance releases only the
    /// acting combatant; the immediate side recount decides whether combat
    /// continues, reaches defeat, or has become empty.
    pub fn apply_combat_out_of_arena_leave(
        &mut self,
        actor_slot: usize,
        direction_code: u8,
    ) -> CombatOutOfArenaLeaveApplication {
        let (ship_style_combat, constrained_exit, established_exit_direction_code) = self
            .combat_frame_snapshot
            .as_ref()
            .map(|snapshot| {
                (
                    matches!(snapshot.player.transport, TransportState::Ship { .. }),
                    snapshot.encounter_mode_high_bit,
                    snapshot.established_exit_direction_code,
                )
            })
            .unwrap_or((
                matches!(self.player.transport, TransportState::Ship { .. }),
                false,
                None,
            ));
        let outcome = resolve_combat_out_of_arena_leave(
            false,
            direction_code,
            ship_style_combat,
            constrained_exit,
            established_exit_direction_code,
            combat_has_active_not_dead_non_party_actor(&self.combat_actors),
        );

        let CombatOutOfArenaLeaveOutcome::Accepted {
            established_direction_code,
            ..
        } = outcome
        else {
            return CombatOutOfArenaLeaveApplication {
                outcome,
                cleared_descriptor: false,
                cleared_active_object: false,
                world_ticks: 0,
            };
        };

        if let Some(snapshot) = &mut self.combat_frame_snapshot {
            snapshot.established_exit_direction_code = established_direction_code;
        }
        self.active_player = None;
        let mut cleared_descriptor = false;
        let mut cleared_active_object = false;
        if let Some(actor) = self.combat_actors.get_mut(actor_slot) {
            let active_object_slot = usize::from(actor.active_object_slot);
            if !actor.is_empty() {
                actor.release_preserving_owner_target_class();
                cleared_descriptor = true;
            }
            if let Some(object) = self.active_objects.get_mut(active_object_slot)
                && !object.is_empty()
            {
                *object = ActiveObject::empty();
                cleared_active_object = true;
            }
        }
        self.advance_visual_tick();
        self.mark_visibility_dirty();
        self.emit_sound_effect(SoundEffect::ActionSnap);
        CombatOutOfArenaLeaveApplication {
            outcome,
            cleared_descriptor,
            cleared_active_object,
            world_ticks: 1,
        }
    }

    pub fn announce_combat_victory_if_needed(&mut self) -> bool {
        if combat_has_active_not_dead_non_party_actor(&self.combat_actors)
            || !combat_escape_has_unmarked_party_side_actor(&self.combat_actors)
        {
            return false;
        }
        let Some(snapshot) = &mut self.combat_frame_snapshot else {
            // Detached deterministic command fixtures have no framer snapshot;
            // production combat always does. Still report the observable
            // transition to callers exercising the command primitive directly.
            return true;
        };
        if snapshot.exit_announced {
            return false;
        }
        snapshot.exit_announced = true;
        true
    }

    pub fn combat_escape_cleanup_decision(&self) -> CombatEscapeCleanupDecision {
        let encounter_mode_high_bit = self
            .combat_frame_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.encounter_mode_high_bit);
        let exit_announced = self
            .combat_frame_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.exit_announced);
        resolve_combat_escape_cleanup(&self.combat_actors, encounter_mode_high_bit, exit_announced)
    }

    pub fn apply_combat_escape_cleanup(&mut self) -> CombatEscapeCleanupApplication {
        let decision = self.combat_escape_cleanup_decision();
        if decision != CombatEscapeCleanupDecision::Accepted {
            return CombatEscapeCleanupApplication::refused(decision);
        }

        let mut cleared_descriptor_slots = 0u8;
        for slot in 0..COMBAT_ACTOR_SLOTS {
            if !self.combat_actors[slot].is_empty() {
                self.combat_actors[slot].clear();
                cleared_descriptor_slots = cleared_descriptor_slots.saturating_add(1);
                self.advance_visual_tick();
            }
        }
        let mut cleared_active_object_slots = 0u8;
        for slot in 0..COMBAT_ACTOR_SLOTS.min(self.active_objects.len()) {
            if !self.active_objects[slot].is_empty() {
                self.active_objects[slot] = ActiveObject::empty();
                cleared_active_object_slots = cleared_active_object_slots.saturating_add(1);
                self.advance_visual_tick();
            }
        }
        self.mark_visibility_dirty();
        // `audio.md §7.4`: "The accepted exit arm prints `Escape!` and plays
        // the 40-update action snap." `§11` lists "the accepted combat exit
        // (`Escape!`)" among the action snap's producers, alongside eight
        // further sites that share the recipe - it is the generic snap, not a
        // bespoke escape cue. The caller prints `Escape!` from this accepted
        // application, so the snap follows the cleanup here.
        self.emit_sound_effect(SoundEffect::ActionSnap);
        CombatEscapeCleanupApplication::accepted(
            cleared_descriptor_slots,
            cleared_active_object_slots,
        )
    }

    pub fn combat_round_loop_control(
        &self,
        leave_combat_flag: bool,
        exhausted_slots: bool,
    ) -> CombatRoundLoopControl {
        let foes_remain = combat_has_active_not_dead_non_party_actor(&self.combat_actors);
        let party_remains = combat_escape_has_unmarked_party_side_actor(&self.combat_actors);
        if !party_remains && foes_remain {
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::Defeat)
        } else if !party_remains || leave_combat_flag {
            CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
        } else if exhausted_slots {
            CombatRoundLoopControl::StartNextRound
        } else {
            CombatRoundLoopControl::ContinueActorWalk
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn apply_combat_actor_slot_dispatch_with_inputs(
        &mut self,
        slot: usize,
        refresh_constant: u8,
        leave_combat_flag: bool,
        possess_candidate_reaches_resistance: bool,
        possess_target_slot: usize,
        possess_resistance_blocks: bool,
        blink_roll: u8,
        summon_roll: u8,
        summon_candidate_coordinates: &[(u8, u8)],
        cleanup_fallback_target: Option<(u8, u8)>,
        mass_charm_roll: u8,
        fleeing: bool,
        teleport_candidate: Option<(u8, u8)>,
        horizontal_axis_first: bool,
        random_cardinal_direction_codes: &[u8],
        monster_attack_inputs_by_slot: &[(usize, CombatMonsterAttackInputs)],
    ) -> CombatActorSlotDispatchApplication {
        self.apply_combat_actor_slot_dispatch_internal(
            slot,
            refresh_constant,
            leave_combat_flag,
            possess_candidate_reaches_resistance,
            possess_target_slot,
            possess_resistance_blocks,
            blink_roll,
            summon_roll,
            summon_candidate_coordinates,
            cleanup_fallback_target,
            mass_charm_roll,
            fleeing,
            teleport_candidate,
            horizontal_axis_first,
            random_cardinal_direction_codes,
            monster_attack_inputs_by_slot,
            false,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_combat_actor_slot_dispatch_internal(
        &mut self,
        slot: usize,
        refresh_constant: u8,
        leave_combat_flag: bool,
        possess_candidate_reaches_resistance: bool,
        possess_target_slot: usize,
        possess_resistance_blocks: bool,
        blink_roll: u8,
        summon_roll: u8,
        summon_candidate_coordinates: &[(u8, u8)],
        cleanup_fallback_target: Option<(u8, u8)>,
        mass_charm_roll: u8,
        fleeing: bool,
        teleport_candidate: Option<(u8, u8)>,
        horizontal_axis_first: bool,
        random_cardinal_direction_codes: &[u8],
        monster_attack_inputs_by_slot: &[(usize, CombatMonsterAttackInputs)],
        draw_ai_inputs_from_shared_prng: bool,
    ) -> CombatActorSlotDispatchApplication {
        if slot >= COMBAT_ACTOR_SLOTS {
            return CombatActorSlotDispatchApplication::EndOfRound {
                control: self.combat_round_loop_control(leave_combat_flag, true),
            };
        }

        // `combat.md §6.3`: the combat walker clears the entire global
        // action-result scratch before the next actor dispatch.
        self.combat_action_result = 0;

        let actor = self.combat_actors[slot];
        if !combat_actor_is_present_not_dead(actor) {
            return CombatActorSlotDispatchApplication::Slot {
                slot,
                phase_tick: Some(CombatActorPhaseTick::Inactive),
                action: CombatActorDispatchAction::Inactive,
                control_after: self.combat_round_loop_control(leave_combat_flag, false),
            };
        }

        // `combat.md §7` step 3: the restraint skip runs *before* the
        // phase decrement in step 4, "so a restrained actor's counter
        // never advances and it never takes a turn at all".
        if self.combat_actor_stands_on_restraint_arena_cell(actor) {
            return CombatActorSlotDispatchApplication::Slot {
                slot,
                phase_tick: Some(CombatActorPhaseTick::Inactive),
                action: CombatActorDispatchAction::Inactive,
                control_after: self.combat_round_loop_control(leave_combat_flag, false),
            };
        }

        if slot < COMBAT_PARTY_ACTOR_SLOTS
            && self
                .party
                .get(slot)
                .copied()
                .is_some_and(|member| member.status == b'D' || member.hp == 0)
        {
            self.combat_actors[slot].mark_dead();
            return CombatActorSlotDispatchApplication::Slot {
                slot,
                phase_tick: None,
                action: CombatActorDispatchAction::PartyDeathSweep,
                control_after: self.combat_round_loop_control(leave_combat_flag, false),
            };
        }

        let Some(phase_tick) = self.tick_combat_actor_phase_counter(slot, refresh_constant) else {
            return CombatActorSlotDispatchApplication::Slot {
                slot,
                phase_tick: None,
                action: CombatActorDispatchAction::Inactive,
                control_after: self.combat_round_loop_control(leave_combat_flag, false),
            };
        };
        if !phase_tick.actor_should_dispatch() {
            return CombatActorSlotDispatchApplication::Slot {
                slot,
                phase_tick: Some(phase_tick),
                action: CombatActorDispatchAction::Waiting,
                control_after: self.combat_round_loop_control(leave_combat_flag, false),
            };
        }

        if actor.is_status_disabled() {
            let wake_roll = self.combat_sleep_wake_roll(slot);
            let wake = self
                .apply_combat_sleep_wake_dispatch(slot, wake_roll)
                .expect("status-disabled actor should produce a wake dispatch");
            let _ = self.apply_combat_post_dispatch_contact_for_actor_position(slot);
            self.clear_combat_interference_for_completed_action(slot);
            return CombatActorSlotDispatchApplication::Slot {
                slot,
                phase_tick: Some(phase_tick),
                action: CombatActorDispatchAction::StatusDisabledWake { wake },
                control_after: self.combat_round_loop_control(leave_combat_flag, false),
            };
        }

        // `combat.md §6.1a` Writers #4, "The Sword of Chaos
        // compulsion": "On the player-driven branch, if the slot is
        // party-side and its character has item id 35 (Sword of Chaos)
        // readied in either the weapon-hand or shield-hand slot, the
        // engine sets this bit on that party descriptor, clears the
        // active-player sentinel, and runs the turn through the
        // automatic actor driver instead of reading a command from the
        // player. Any other readied equipment takes the ordinary
        // interactive path and never sets the bit."
        //
        // Setting bit `0x01` here is what performs the redirect: the
        // bit is the slot-to-group helper's team toggle, so the very
        // next group read below sends this party-side slot to the
        // driver rather than to `PlayerReady`. `§6.1a` "Lifetime": the
        // bit "lives only in the combat-instance descriptor table" and
        // "Nothing writes it into the save image", so this is a
        // per-combat descriptor write only.
        //
        // The compulsion rides on the handler's player-driven branch,
        // which `§6.1a` defines as taken "when the active-player
        // sentinel is unset, or when the slot is party-side and its
        // owner/character byte equals the sentinel", so a party slot
        // that is not the one the player is currently commanding does
        // not have the bit stamped on it this dispatch.
        let player_driven_branch = self.active_player.is_none()
            || self.active_player == Some(usize::from(actor.owner_target_class));
        if slot < COMBAT_PARTY_ACTOR_SLOTS
            && player_driven_branch
            && self.combat_target_group_for_slot(slot) == COMBAT_TARGET_GROUP_PARTY
            && self
                .party_equipment
                .get(actor.owner_target_class as usize)
                .is_some_and(|equipment| {
                    equipment_compels_automatic_turn(usize::from(equipment[EQUIP_SLOT_WEAPON]))
                        || equipment_compels_automatic_turn(usize::from(
                            equipment[EQUIP_SLOT_OFFHAND],
                        ))
                })
        {
            self.combat_actors[slot].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
            self.active_player = None;
        }

        // `combat.md §6.1a` "A dispatch input": the walker dispatches through
        // the slot-to-group helper, which returns bit `0x01` itself for a
        // party-side slot and that bit inverted for a monster-side slot. The
        // group ordinarily occupied by seated party members goes to the
        // keystroke/command path and the other group to the automatic actor
        // driver, so a party-side actor carrying the controlled/charmed bit
        // takes its turn through the driver instead of the player's prompt.
        // The earlier reading that any slot carrying the bit is sent to the
        // player command parser is expressly withdrawn.
        let action = if self.combat_target_group_for_slot(slot) == COMBAT_TARGET_GROUP_PARTY {
            CombatActorDispatchAction::PlayerReady
        } else if resolve_negate_time_dispatch_skipped(
            self.active_effect_tag,
            self.active_effect_counter,
        ) {
            // `magic.md` tag `T`: the automatic actor driver returns
            // immediately while Negate Time is live, so every self-acting
            // actor's turn is skipped outright.
            CombatActorDispatchAction::NegateTimeSkipped
        } else if resolve_quickness_dispatch_consumed(
            self.active_effect_tag,
            self.active_effect_counter,
            self.combat_quickness_dispatch_roll(slot),
        ) {
            // `combat.md §8`: the single Quickness gate sits at the head of the
            // automatic actor driver, so a self-acting slot forfeits about half
            // its dispatches while the effect is live.
            CombatActorDispatchAction::QuicknessSkipped
        } else {
            let monster_attack_inputs = monster_attack_inputs_by_slot
                .iter()
                .find_map(|&(input_slot, inputs)| (input_slot == slot).then_some(inputs));
            let ai_turn = if draw_ai_inputs_from_shared_prng {
                self.apply_combat_ai_turn(slot)
            } else {
                self.apply_combat_ai_turn_with_inputs(
                    slot,
                    possess_candidate_reaches_resistance,
                    possess_target_slot,
                    possess_resistance_blocks,
                    blink_roll,
                    summon_roll,
                    summon_candidate_coordinates,
                    cleanup_fallback_target,
                    mass_charm_roll,
                    fleeing,
                    teleport_candidate,
                    horizontal_axis_first,
                    random_cardinal_direction_codes,
                    monster_attack_inputs,
                )
            };
            CombatActorDispatchAction::MonsterAi { ai_turn }
        };

        if !matches!(action, CombatActorDispatchAction::PlayerReady) {
            let _ = self.apply_combat_post_dispatch_contact_for_actor_position(slot);
            self.clear_combat_interference_for_completed_action(slot);
        }

        CombatActorSlotDispatchApplication::Slot {
            slot,
            phase_tick: Some(phase_tick),
            action,
            control_after: self.combat_round_loop_control(leave_combat_flag, false),
        }
    }

    pub fn apply_combat_actor_slot_dispatch(
        &mut self,
        slot: usize,
        refresh_constant: u8,
        leave_combat_flag: bool,
    ) -> CombatActorSlotDispatchApplication {
        // The shared-PRNG mode enters the exact same phase/status/effect gates
        // as deterministic tests, then draws monster-AI inputs only after the
        // slot actually reaches Pass 2.
        self.apply_combat_actor_slot_dispatch_internal(
            slot,
            refresh_constant,
            leave_combat_flag,
            false,
            0,
            false,
            32,
            32,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[],
            &[],
            true,
        )
    }

    /// `combat.md §7`: toggle the combat-overlay blink and report the two
    /// presentation coordinates for the next idle repaint. A dark pass, an
    /// invalid active cell, or a non-player active group suppresses both
    /// overlays. The secondary coordinate is deliberately not range-checked;
    /// the display surface owns clipping.
    pub fn apply_combat_cursor_blink_tick(&mut self) -> CombatCursorBlinkReport {
        let mut report = CombatCursorBlinkReport::default();

        if self.combat_active {
            self.combat_cursor_blink = !self.combat_cursor_blink;
            report.cursor_blink_visible = self.combat_cursor_blink;
            if self.combat_cursor_blink {
                report.cursor_draw_cell = self.combat_cursor_actor_cell();
                if report.cursor_draw_cell.is_some() {
                    report.secondary_marker_cell = self.combat_secondary_marker;
                }
            }
        }

        report
    }

    /// Inverse of [`Self::combat_roster_slot_for_actor_slot`]: find the
    /// party descriptor that `combat.md §5` seated for a given roster
    /// slot. Needed because a packed party puts a character at a lower
    /// descriptor index than its roster index.
    pub(crate) fn combat_party_descriptor_slot_for_roster_slot(
        &self,
        roster_slot: usize,
    ) -> Option<usize> {
        (0..COMBAT_PARTY_ACTOR_SLOTS)
            .find(|slot| self.combat_roster_slot_for_actor_slot(*slot) == Some(roster_slot))
    }

    /// Point the resident active-player selector at the combat actor the
    /// round walk just parked on.
    ///
    /// `combat.md §7` draws the cursor box "around the eligible active
    /// player's arena cell" and `stats-panel.md §4.1` puts the roster marker
    /// on the selected member; both read this one selector. Nothing else
    /// moved it during a fight, so the box and the marker stayed on whichever
    /// member the exploration gate had selected instead of following the
    /// character being prompted - which is the member the original draws the
    /// white box around.
    pub(crate) fn select_active_player_for_pending_combat_actor(&mut self) {
        let Some(slot) = self.pending_combat_actor_slot else {
            return;
        };
        if let Some(roster_slot) = self.combat_roster_slot_for_actor_slot(slot) {
            self.active_player = Some(roster_slot);
        }
    }

    pub(crate) fn combat_cursor_actor_cell(&self) -> Option<(u8, u8)> {
        // The active-player sentinel names a roster slot; the cursor is
        // drawn on that character's descriptor, found through its
        // owner/target/class byte (`combat.md §5`).
        let slot = self.combat_party_descriptor_slot_for_roster_slot(self.active_player?)?;
        let actor = *self.combat_actors.get(slot)?;
        if !combat_actor_is_active_not_dead(actor)
            || self.combat_target_group_for_slot(slot) != COMBAT_TARGET_GROUP_PARTY
        {
            return None;
        }
        let x = usize::from(actor.x);
        let y = usize::from(actor.y);
        (x < COMBAT_ARENA_SIDE && y < COMBAT_ARENA_SIDE).then_some((actor.x, actor.y))
    }

    pub fn apply_combat_round_walk_from_slot_with_inputs(
        &mut self,
        start_slot: usize,
        refresh_constant: u8,
        leave_combat_flag: bool,
        possess_candidate_reaches_resistance: bool,
        possess_target_slot: usize,
        possess_resistance_blocks: bool,
        blink_roll: u8,
        summon_roll: u8,
        summon_candidate_coordinates: &[(u8, u8)],
        cleanup_fallback_target: Option<(u8, u8)>,
        mass_charm_roll: u8,
        fleeing: bool,
        teleport_candidate: Option<(u8, u8)>,
        horizontal_axis_first: bool,
        random_cardinal_direction_codes: &[u8],
        monster_attack_inputs_by_slot: &[(usize, CombatMonsterAttackInputs)],
    ) -> CombatRoundWalkApplication {
        let mut applications = Vec::new();
        let mut slot = start_slot;
        loop {
            let application = self.apply_combat_actor_slot_dispatch_with_inputs(
                slot,
                refresh_constant,
                leave_combat_flag,
                possess_candidate_reaches_resistance,
                possess_target_slot,
                possess_resistance_blocks,
                blink_roll,
                summon_roll,
                summon_candidate_coordinates,
                cleanup_fallback_target,
                mass_charm_roll,
                fleeing,
                teleport_candidate,
                horizontal_axis_first,
                random_cardinal_direction_codes,
                monster_attack_inputs_by_slot,
            );

            match &application {
                CombatActorSlotDispatchApplication::EndOfRound { .. } => {
                    self.apply_combat_cursor_blink_tick();
                    applications.push(application);
                    return CombatRoundWalkApplication {
                        start_slot,
                        next_slot: COMBAT_ACTOR_SLOTS,
                        stop_reason: CombatRoundWalkStopReason::EndOfRound,
                        applications,
                    };
                }
                CombatActorSlotDispatchApplication::Slot {
                    action,
                    control_after,
                    ..
                } => {
                    let stop_reason = if control_after.result_code().is_some() {
                        Some(CombatRoundWalkStopReason::Exit)
                    } else if matches!(action, CombatActorDispatchAction::PlayerReady) {
                        Some(CombatRoundWalkStopReason::AwaitingPlayer)
                    } else {
                        None
                    };
                    applications.push(application);
                    if let Some(stop_reason) = stop_reason {
                        return CombatRoundWalkApplication {
                            start_slot,
                            next_slot: slot.saturating_add(1).min(COMBAT_ACTOR_SLOTS),
                            stop_reason,
                            applications,
                        };
                    }
                }
            }

            slot = slot.saturating_add(1);
        }
    }

    pub fn apply_combat_round_walk_from_slot(
        &mut self,
        start_slot: usize,
        refresh_constant: u8,
        leave_combat_flag: bool,
    ) -> CombatRoundWalkApplication {
        self.apply_combat_round_walk_from_slot_inner(
            start_slot,
            refresh_constant,
            leave_combat_flag,
            false,
        )
    }

    /// Run the actor walk until input, exit, the end of a round, or one
    /// automatic action that needs a visible presentation. The ordinary
    /// blocking driver remains available above for terminal and test callers.
    pub fn apply_combat_round_walk_from_slot_paced(
        &mut self,
        start_slot: usize,
        refresh_constant: u8,
        leave_combat_flag: bool,
    ) -> CombatRoundWalkApplication {
        self.apply_combat_round_walk_from_slot_inner(
            start_slot,
            refresh_constant,
            leave_combat_flag,
            true,
        )
    }

    fn apply_combat_round_walk_from_slot_inner(
        &mut self,
        start_slot: usize,
        refresh_constant: u8,
        leave_combat_flag: bool,
        stop_after_automatic_action: bool,
    ) -> CombatRoundWalkApplication {
        let mut applications = Vec::new();
        let mut slot = start_slot;
        loop {
            let application =
                self.apply_combat_actor_slot_dispatch(slot, refresh_constant, leave_combat_flag);

            match &application {
                CombatActorSlotDispatchApplication::EndOfRound { .. } => {
                    self.apply_combat_cursor_blink_tick();
                    applications.push(application);
                    return CombatRoundWalkApplication {
                        start_slot,
                        next_slot: COMBAT_ACTOR_SLOTS,
                        stop_reason: CombatRoundWalkStopReason::EndOfRound,
                        applications,
                    };
                }
                CombatActorSlotDispatchApplication::Slot {
                    action,
                    control_after,
                    ..
                } => {
                    let stop_reason = if control_after.result_code().is_some() {
                        Some(CombatRoundWalkStopReason::Exit)
                    } else if matches!(action, CombatActorDispatchAction::PlayerReady) {
                        Some(CombatRoundWalkStopReason::AwaitingPlayer)
                    } else if stop_after_automatic_action
                        && matches!(
                            action,
                            CombatActorDispatchAction::PartyDeathSweep
                                | CombatActorDispatchAction::StatusDisabledWake { .. }
                                | CombatActorDispatchAction::QuicknessSkipped
                                | CombatActorDispatchAction::NegateTimeSkipped
                                | CombatActorDispatchAction::MonsterAi { ai_turn: Some(_) }
                        )
                    {
                        Some(CombatRoundWalkStopReason::AutomaticAction)
                    } else {
                        None
                    };
                    applications.push(application);
                    if let Some(stop_reason) = stop_reason {
                        return CombatRoundWalkApplication {
                            start_slot,
                            next_slot: slot.saturating_add(1).min(COMBAT_ACTOR_SLOTS),
                            stop_reason,
                            applications,
                        };
                    }
                }
            }

            slot = slot.saturating_add(1);
        }
    }

    pub fn combat_ai_possess_target_slot_roll(&mut self, actor_slot: usize) -> usize {
        let _ = actor_slot;
        usize::from(self.random_range_u8(0, (COMBAT_ACTOR_SLOTS - 1) as u8))
    }

    pub fn combat_ai_possess_resistance_blocks(
        &mut self,
        actor_slot: usize,
        target_slot: usize,
    ) -> bool {
        self.combat_resistance_blocks(actor_slot, target_slot)
    }

    pub fn combat_ai_possess_candidate_reaches_resistance_from_roll(
        &self,
        target_slot: usize,
    ) -> bool {
        let Some(candidate) = self
            .combat_actors
            .get(target_slot)
            .copied()
            .map(|descriptor| {
                combat_possess_candidate_view(
                    descriptor,
                    // `combat.md §5`: descriptor index is not roster index.
                    self.combat_roster_slot_for_actor_slot(target_slot)
                        .and_then(|roster_slot| self.party.get(roster_slot).copied()),
                    false,
                    false,
                )
            })
        else {
            return false;
        };
        combat_possess_candidate_reaches_resistance(target_slot, candidate, self.active_player)
    }

    pub fn combat_ai_blink_roll(&mut self, actor_slot: usize) -> u8 {
        let _ = actor_slot;
        self.random_range_u8(0, u8::MAX)
    }

    pub fn combat_ai_summon_roll(&mut self, actor_slot: usize) -> u8 {
        let _ = actor_slot;
        self.random_range_u8(0, u8::MAX)
    }

    /// `combat.md §9`: a passed monster-summon gate makes exactly two
    /// fresh shared-PRNG draws, X first and then Y, both in inclusive 0..15.
    pub fn combat_ai_summon_probe_coordinate(&mut self, actor_slot: usize) -> (u8, u8) {
        let _ = actor_slot;
        (self.random_range_u8(0, 15), self.random_range_u8(0, 15))
    }

    pub fn combat_ai_mass_charm_roll(&mut self, actor_slot: usize) -> u8 {
        let _ = actor_slot;
        self.random_range_u8(0, u8::MAX)
    }

    pub fn combat_ai_teleport_candidate(&mut self, actor_slot: usize) -> Option<(u8, u8)> {
        let _ = actor_slot;
        Some((
            self.random_range_u8(0, (COMBAT_ARENA_SIDE - 1) as u8),
            self.random_range_u8(0, (COMBAT_ARENA_SIDE - 1) as u8),
        ))
    }

    pub fn combat_ai_horizontal_axis_first(&mut self, actor_slot: usize) -> bool {
        let _ = actor_slot;
        self.random_mod_u8(2) == 0
    }

    pub fn combat_ai_random_cardinal_direction_codes(&mut self, actor_slot: usize) -> [u8; 4] {
        let _ = actor_slot;
        [
            self.random_range_u8(1, 4),
            self.random_range_u8(1, 4),
            self.random_range_u8(1, 4),
            self.random_range_u8(1, 4),
        ]
    }

    pub fn combat_monster_attack_inputs(
        &mut self,
        attacker_slot: usize,
    ) -> CombatMonsterAttackInputs {
        let _ = attacker_slot;
        CombatMonsterAttackInputs {
            party_defender_rating: 7,
            hit_roll: self.random_range_u8(0, u8::MAX),
            damage_roll: self.random_range_u8(0, u8::MAX),
            poison_gate_accepts: self.random_mod_u8(2) == 0,
            poison_damage_roll: self.random_mod_u8(20),
            forced_hit: None,
            amulet_turning_scatter_roll: self.random_mod_u8(8),
        }
    }

    pub fn combat_monster_amulet_turning_roll(
        &mut self,
        attacker_slot: usize,
        target_slot: usize,
    ) -> u8 {
        let _ = (attacker_slot, target_slot);
        self.random_range_u8(0, u8::MAX)
    }

    pub fn ensure_pending_combat_player_turn(&mut self) -> Option<CombatRoundWalkApplication> {
        if !self.combat_active || self.pending_combat_actor_slot.is_some() {
            return None;
        }

        let mut last_application = None;
        for _ in 0..COMBAT_ROUND_WALK_DRAIN_LIMIT {
            let start_slot = self.next_combat_actor_slot.min(COMBAT_ACTOR_SLOTS);
            let application = self.apply_combat_round_walk_from_slot(
                start_slot,
                COMBAT_PHASE_REFRESH_CONSTANT,
                false,
            );
            self.next_combat_actor_slot = match application.stop_reason {
                CombatRoundWalkStopReason::EndOfRound => 0,
                CombatRoundWalkStopReason::AwaitingPlayer
                | CombatRoundWalkStopReason::AutomaticAction
                | CombatRoundWalkStopReason::Exit => application.next_slot,
            };
            if application.stop_reason == CombatRoundWalkStopReason::AwaitingPlayer {
                self.open_pending_combat_player_turn(ready_player_slot_from_round_walk(
                    &application,
                ));
            }
            let should_stop = !matches!(
                application.stop_reason,
                CombatRoundWalkStopReason::EndOfRound
            ) || self.pending_combat_actor_slot.is_some();
            last_application = Some(application);
            if should_stop {
                break;
            }
        }
        last_application
    }

    pub fn apply_combat_step_or_attack_primitive(
        &mut self,
        moving_slot: usize,
        attacker_group: u8,
        direction_code: u8,
        destination_walkable: bool,
    ) -> CombatStepOrAttackPrimitiveOutcome {
        if moving_slot >= COMBAT_ACTOR_SLOTS {
            return CombatStepOrAttackPrimitiveOutcome::InactiveActor;
        }

        let candidates = self
            .combat_actors
            .iter()
            .copied()
            .enumerate()
            .map(|(slot, descriptor)| {
                self.combat_target_candidate_view(
                    descriptor,
                    slot,
                    false,
                    descriptor.is_hidden_or_unrevealed(),
                )
            })
            .collect::<Vec<_>>();

        let outcome = resolve_combat_step_or_attack_primitive(
            &mut self.combat_actors[moving_slot],
            &mut self.active_objects,
            &candidates,
            moving_slot,
            attacker_group,
            direction_code,
            destination_walkable,
        );
        if outcome.committed_movement() {
            self.mark_visibility_dirty();
            let _ = self.apply_combat_ambush_reveal_for_actor_position(moving_slot);
        }
        outcome
    }

    pub fn tick_combat_actor_phase_counter(
        &mut self,
        slot: usize,
        refresh_constant: u8,
    ) -> Option<CombatActorPhaseTick> {
        let tick = crate::tick_combat_actor_phase_counter(
            self.combat_actors.get_mut(slot)?,
            refresh_constant,
        );
        if tick.actor_should_dispatch() {
            self.advance_combat_round_counter();
        }
        Some(tick)
    }

    pub fn apply_combat_magic_ring_pass_to_slot(
        &mut self,
        slot: usize,
        regeneration_roll: u8,
        vanish_roll: u8,
    ) -> Option<CombatMagicRingPassOutcome> {
        let actor = *self.combat_actors.get(slot)?;
        if actor.flags & COMBAT_ACTOR_FLAG_SELECTABLE_80 == 0 || actor.is_marked_dead() {
            return None;
        }
        let wearer_slot = actor.owner_target_class as usize;
        let wearer = *self.party.get(wearer_slot)?;
        if self.party_equipment.len() < self.party.len() {
            self.party_equipment
                .resize(self.party.len(), [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]);
        }

        let ring = self.party_equipment[wearer_slot][EQUIP_SLOT_RING];
        let mut outcome = CombatMagicRingPassOutcome::default();
        // `combat.md §12`: the encounter-entry destruction roll happens
        // immediately before seating-time ring effects. A vanished ring never
        // reaches invisibility or regeneration.
        if combat_magic_ring_vanishes(ring, vanish_roll) {
            outcome.vanished_ring = Some(ring);
            self.message = "A ring has vanished!".to_string();
            // `audio.md §8.1` terrain-combat-entry path, in its published
            // order: "print `A ring has vanished!`, play the 40-update action
            // snap, then remove the item". The Ready path orders the same
            // three steps print/destroy/tone instead, so the ordering is
            // stated per path and not shared. Both are a 1-in-16 random roll
            // with no player interaction: the earlier "a cancelled
            // confirmation does not [play it]" clause is withdrawn
            // (`RETRACTIONS.md`), because there is no confirmation prompt.
            // `§11` puts both paths on the generic action snap.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.party_equipment[wearer_slot][EQUIP_SLOT_RING] = EQUIPMENT_EMPTY;
            if ring as usize == EQUIPMENT_ID_RING_INVISIBILITY
                && clear_combat_linked_invisibility(
                    &mut self.combat_actors[slot],
                    &mut self.active_objects,
                )
                .is_some_and(CombatLinkedVisibilityOutcome::changed)
            {
                self.mark_visibility_dirty();
            }
            return Some(outcome);
        }

        if ring as usize == EQUIPMENT_ID_RING_INVISIBILITY {
            outcome.invisibility_applied = apply_combat_linked_invisibility(
                &mut self.combat_actors[slot],
                &mut self.active_objects,
            )
            .is_some_and(CombatLinkedVisibilityOutcome::changed);
            if outcome.invisibility_applied {
                self.mark_visibility_dirty();
            }
        }

        let regeneration = combat_ring_regeneration_amount(wearer, ring, regeneration_roll);
        if regeneration != 0 {
            outcome.regeneration_applied = self.party[wearer_slot].heal_by(regeneration);
        }

        Some(outcome)
    }

    /// Run the encounter-entry-only ring destruction and seating hooks in
    /// descriptor order. Action-time maintenance deliberately uses the
    /// separate non-vanishing helper above the command parser.
    fn apply_combat_entry_magic_ring_passes(&mut self) {
        for actor_slot in 0..COMBAT_ACTOR_SLOTS {
            let actor = self.combat_actors[actor_slot];
            if actor.flags & COMBAT_ACTOR_FLAG_SELECTABLE_80 == 0 || actor.is_marked_dead() {
                continue;
            }
            let wearer_slot = actor.owner_target_class as usize;
            let Some(ring) = self
                .party_equipment
                .get(wearer_slot)
                .map(|equipment| equipment[EQUIP_SLOT_RING])
            else {
                continue;
            };
            if ring as usize != EQUIPMENT_ID_RING_INVISIBILITY
                && ring as usize != EQUIPMENT_ID_RING_REGENERATION
            {
                continue;
            }
            let vanish_roll = self.combat_magic_ring_vanish_roll(actor_slot);
            let regeneration_roll =
                if vanish_roll != 0 && ring as usize == EQUIPMENT_ID_RING_REGENERATION {
                    self.combat_magic_ring_regeneration_roll(actor_slot)
                } else {
                    1
                };
            let _ = self.apply_combat_magic_ring_pass_to_slot(
                actor_slot,
                regeneration_roll,
                vanish_roll,
            );
        }
    }

    pub fn advance_combat_round_counter(&mut self) -> CombatRoundCounterTick {
        let tick = resolve_combat_round_counter_tick(self.combat_round_counter);
        self.combat_round_counter = tick.counter;
        if tick.redraw_tiles {
            self.mark_visibility_dirty();
        }
        if tick.advance_time_minutes != 0 {
            self.advance_turn_with_minutes(tick.advance_time_minutes);
        }
        tick
    }
}

pub fn apply_combat_ambush_reveal_records(
    records: &mut [Option<CombatAmbushRevealRecord>; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
    terrain: &mut [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    trigger_x: u8,
    trigger_y: u8,
) -> Option<CombatAmbushRevealApplication> {
    for (slot, entry) in records.iter_mut().enumerate() {
        let Some(record) = *entry else {
            continue;
        };
        if !record.trigger_matches(trigger_x, trigger_y) {
            continue;
        }

        *entry = None;
        let mut stamped_cells = 0;
        for (x, y) in [
            (record.target_a_x, record.target_a_y),
            (record.target_b_x, record.target_b_y),
        ] {
            if let Some((x, y)) = combat_reveal_target_cell(x, y) {
                terrain[y][x] = record.reveal_tile;
                stamped_cells += 1;
            }
        }

        return Some(CombatAmbushRevealApplication {
            slot,
            trigger_x,
            trigger_y,
            reveal_tile: record.reveal_tile,
            stamped_cells,
        });
    }
    None
}

pub const fn combat_reveal_target_cell(x: u8, y: u8) -> Option<(usize, usize)> {
    if (x as usize) < COMBAT_ARENA_SIDE && (y as usize) < COMBAT_ARENA_SIDE {
        Some((x as usize, y as usize))
    } else {
        None
    }
}

pub fn ready_player_slot_from_round_walk(
    application: &CombatRoundWalkApplication,
) -> Option<usize> {
    application
        .applications
        .iter()
        .rev()
        .find_map(|entry| match entry {
            CombatActorSlotDispatchApplication::Slot {
                slot,
                action: CombatActorDispatchAction::PlayerReady,
                ..
            } => Some(*slot),
            _ => None,
        })
}

#[cfg(test)]
mod combat_death_batch_tests {
    use super::*;
    use crate::test_fixtures::{open_world_grid, world_state};

    fn place_incorporeal_test_monster(
        state: &mut PlayState,
        class: u8,
        actor_slot: usize,
        active_object_slot: usize,
    ) -> CombatClassStats {
        let stats = combat_class_stats(class).unwrap();
        state
            .active_objects
            .resize(COMBAT_ACTOR_SLOTS, ActiveObject::empty());
        state.combat_actors[actor_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            active_object_slot as u8,
            4,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        state.active_objects[active_object_slot] = ActiveObject {
            type_byte: 0x90,
            tile: 0x90,
            x: 4,
            y: 5,
            z: 0,
            phase: 7,
            aux1: 0x55,
            aux3: 0,
        };
        stats
    }

    #[test]
    fn incorporeal_death_releases_the_slot_without_a_marker_or_a_drop() {
        // `combat.md §6.3` Incorporeal-class row: tile byte written into
        // active-object bytes 0 and 1 is "none", other writes are
        // "none", and the slot IS released. `§12`: the branch "releases
        // the slot immediately and leaves **no tile marker and no drop
        // at all**". Before this landed, a Ghost fell through to the
        // default drop check and stamped a corpse/chest marker.
        let mut state = world_state(open_world_grid(), 10, 20);
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let active_object_slot = 10;
        let stats = place_incorporeal_test_monster(&mut state, 23, actor_slot, active_object_slot);
        let marker_before = state.active_objects[active_object_slot];
        let prng_before = state.prng_state;

        let application = state
            .apply_combat_weapon_damage_to_target(
                None,
                actor_slot,
                COMBAT_INSTANT_KILL_DAMAGE,
                true,
            )
            .unwrap();

        let CombatWeaponDamageApplication::Monster { damage, .. } = application else {
            panic!("a monster slot should produce a monster damage application");
        };
        assert_eq!(damage.death_path, Some(CombatMonsterDeathPath::Incorporeal));
        assert_eq!(
            damage.return_value,
            stats.reward_unit(),
            "the reward unit is still computed on the incorporeal branch"
        );
        assert!(
            state.combat_actors[actor_slot].is_free_for_allocation(),
            "the incorporeal branch releases its descriptor slot"
        );
        assert_eq!(
            state.combat_actors[actor_slot].owner_target_class, stats.class,
            "negative release preserves descriptor byte 3"
        );
        assert_eq!(
            state.active_objects[active_object_slot],
            ActiveObject {
                phase: marker_before.phase,
                aux3: marker_before.aux3,
                ..ActiveObject::empty()
            },
            "negative release clears bytes 0..5 and preserves bytes 6..7"
        );
        assert_eq!(
            state.prng_state, prng_before,
            "the incorporeal branch runs no drop rolls"
        );
    }

    #[test]
    fn vanish_reveal_visits_every_pixel_once_and_ticks_after_each_eight_except_last() {
        let playback = combat_terrain_reveal_playback(6, (4, 5), 0x42);
        assert_eq!(playback.pixel_order.len(), 256);
        assert_eq!(playback.pixel_order[0], (0, 0));
        assert_eq!(playback.pixel_order[1], (0, 1));
        assert_eq!(
            playback
                .pixel_order
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            256
        );
        assert_eq!(
            playback.world_tick_after_operations,
            (8..=248).step_by(8).collect::<Vec<_>>()
        );
    }

    fn sword_of_chaos_dispatch_state() -> PlayState {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.active_objects = vec![ActiveObject::empty(); OOL_SLOTS];
        state.active_objects[0] = ActiveObject {
            type_byte: 0x80,
            tile: 0x80,
            x: 5,
            y: 5,
            ..ActiveObject::empty()
        };
        state.combat_actors[0] = CombatActorDescriptor::from_row([
            20,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            0,
            0,
            5,
            5,
        ]);
        state.active_player = Some(0);
        state
    }

    fn dispatch_party_slot_zero(state: &mut PlayState) -> CombatActorDispatchAction {
        let application = state.apply_combat_actor_slot_dispatch_with_inputs(
            0,
            30,
            false,
            false,
            0,
            false,
            1,
            1,
            &[],
            None,
            0,
            false,
            None,
            true,
            &[1, 2, 3, 4],
            &[],
        );
        let CombatActorSlotDispatchApplication::Slot { action, .. } = application else {
            panic!("a live party slot should produce a slot dispatch");
        };
        action
    }

    #[test]
    fn a_readied_sword_of_chaos_compels_the_automatic_turn() {
        // `combat.md §6.1a` Writers #4, "The Sword of Chaos
        // compulsion": "if the slot is party-side and its character has
        // item id 35 (Sword of Chaos) readied in either the weapon-hand
        // or shield-hand slot, the engine sets this bit on that party
        // descriptor, clears the active-player sentinel, and runs the
        // turn through the automatic actor driver instead of reading a
        // command from the player."
        for hand in [EQUIP_SLOT_WEAPON, EQUIP_SLOT_OFFHAND] {
            let mut state = sword_of_chaos_dispatch_state();
            state.party_equipment[0][hand] = EQUIPMENT_SWORD_OF_CHAOS as u8;

            let action = dispatch_party_slot_zero(&mut state);

            assert_ne!(
                action,
                CombatActorDispatchAction::PlayerReady,
                "hand slot {hand}: the compelled turn runs on the automatic driver"
            );
            assert!(
                state.combat_actors[0].is_controlled(),
                "hand slot {hand}: bit 0x01 is set on the party descriptor"
            );
            assert_eq!(
                state.active_player, None,
                "hand slot {hand}: the active-player sentinel is cleared"
            );
        }
    }

    #[test]
    fn the_compulsion_only_fires_on_the_handlers_player_driven_branch() {
        // `§6.1a`: "The command-path handler (Section 8) takes its
        // player-driven branch for a slot when the active-player
        // sentinel is unset, or when the slot is party-side and its
        // owner/character byte equals the sentinel", and the
        // compulsion is described as happening "On the player-driven
        // branch".
        let mut other_is_active = sword_of_chaos_dispatch_state();
        other_is_active.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_SWORD_OF_CHAOS as u8;
        other_is_active.active_player = Some(3);
        assert_eq!(
            dispatch_party_slot_zero(&mut other_is_active),
            CombatActorDispatchAction::PlayerReady
        );
        assert!(!other_is_active.combat_actors[0].is_controlled());
        assert_eq!(other_is_active.active_player, Some(3));

        // An unset sentinel is the other accepted form of that branch.
        let mut sentinel_unset = sword_of_chaos_dispatch_state();
        sentinel_unset.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_SWORD_OF_CHAOS as u8;
        sentinel_unset.active_player = None;
        assert_ne!(
            dispatch_party_slot_zero(&mut sentinel_unset),
            CombatActorDispatchAction::PlayerReady
        );
        assert!(sentinel_unset.combat_actors[0].is_controlled());
    }

    #[test]
    fn any_other_readied_weapon_keeps_the_ordinary_interactive_path() {
        // `§6.1a`: "Any other readied equipment takes the ordinary
        // interactive path and never sets the bit."
        for item_id in [EQUIPMENT_EMPTY as usize, EQUIPMENT_SWORD_OF_CHAOS - 1] {
            let mut state = sword_of_chaos_dispatch_state();
            state.party_equipment[0][EQUIP_SLOT_WEAPON] = item_id as u8;
            state.party_equipment[0][EQUIP_SLOT_OFFHAND] = item_id as u8;

            let action = dispatch_party_slot_zero(&mut state);

            assert_eq!(
                action,
                CombatActorDispatchAction::PlayerReady,
                "item {item_id} must not compel an automatic turn"
            );
            assert!(!state.combat_actors[0].is_controlled(), "item {item_id}");
            assert_eq!(state.active_player, Some(0), "item {item_id}");
        }
    }
}

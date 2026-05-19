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
    pub enter_endgame_after_successful_combat: bool,
    pub endgame_messages: Option<EndgameMessages>,
}

/// `combat.md §5` ambush / camp-attack reveal-slot capacity.
/// Ambush-style and camp-attack arenas can stamp up to eight
/// hidden reveal coordinates; stepping onto one consumes the
/// coordinate and rewrites one or two arena cells with the
/// associated reveal tile when their target coordinates are
/// inside the eleven-by-eleven arena. Coordinates outside the
/// arena are sentinels for "no stamp" rather than map cells.
pub const COMBAT_AMBUSH_REVEAL_SLOTS_MAX: u8 = 8;

/// `combat.md §14` round-loop exit outcomes. The framer's restore
/// phase runs identically for all three; only the result code the
/// round loop returns to its caller differs. Victory and Escape
/// both return `1`; Defeat returns `0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatExitOutcome {
    /// Every hostile actor has been killed.
    Victory,
    /// The entire party is dead, asleep, or otherwise inactive
    /// (also reached intentionally by combat `Q`).
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
            Self::Victory | Self::Escape => 1,
            Self::Defeat => 0,
        }
    }
}

/// `combat.md §12` split-on-damage placement-attempt cap. When a
/// monster with the split-on-damage class flag is damaged but not
/// killed, combat scans the actor table for an empty slot to copy
/// the parent's class byte into. Up to this many attempts are made
/// before the divide is dropped silently.
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

pub fn decode_active_player_slot(byte: u8, party_size: usize) -> Option<usize> {
    if byte == 0xff {
        return None;
    }
    let slot = usize::from(byte);
    (slot < party_size && slot < COMBAT_PARTY_ACTOR_SLOTS).then_some(slot)
}

pub fn encode_active_player_slot(active_player: Option<usize>) -> u8 {
    match active_player {
        Some(slot) if slot < COMBAT_PARTY_ACTOR_SLOTS => slot as u8,
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
    matches!(exit, CombatRoundLoopExit::LeaveCombat)
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
pub struct CombatArenaFieldPlacementApplication {
    pub field: CombatArenaFieldKind,
    pub target_slot: usize,
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
    QuitDefeat,
    XitCleanup {
        allowed: bool,
    },
    Branch {
        branch: CombatCommandBranch,
        live_actor_gate: CombatCommandLiveActorGate,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatPlayerCommandApplication {
    pub actor_slot: usize,
    pub input: CombatPlayerCommandInput,
    pub action: CombatPlayerCommandAction,
    pub weapon_attack: Option<CombatWeaponAttackApplication>,
    pub ring_pass: Option<CombatMagicRingPassOutcome>,
    pub control_after: CombatRoundLoopControl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CombatActorDispatchAction {
    Inactive,
    PartyDeathSweep,
    Waiting,
    PlayerReady,
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

impl PlayState {
    pub(crate) fn combat_party_name_for_slot(&self, slot: usize) -> Option<&[u8]> {
        (slot < COMBAT_PARTY_ACTOR_SLOTS)
            .then(|| self.party_names.get(slot).map(|name| name.as_slice()))
            .flatten()
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

    fn combat_ai_morale_roll(&self, actor_slot: usize) -> u8 {
        (self.turn as u8)
            .wrapping_add((actor_slot as u8).wrapping_mul(29))
            .wrapping_add(self.combat_round_counter)
    }

    fn combat_ai_actor_fleeing(&mut self, actor_slot: usize) -> bool {
        let Some(actor) = self.combat_actors.get(actor_slot).copied() else {
            return false;
        };
        if actor_slot < COMBAT_PARTY_ACTOR_SLOTS || !combat_actor_is_active_not_dead(actor) {
            return false;
        }
        let Some(morale) = resolve_combat_wound_morale_for_class(
            actor.hp_or_wound,
            actor.owner_target_class,
            self.combat_ai_morale_roll(actor_slot),
        ) else {
            return actor.is_fleeing();
        };
        self.combat_actors[actor_slot].set_fleeing(morale.fleeing);
        morale.fleeing
    }

    fn combat_actor_stands_on_walkable_arena_cell(&self, actor: CombatActorDescriptor) -> bool {
        if !self.combat_terrain.iter().flatten().any(|tile| *tile != 0) {
            return true;
        }
        let x = actor.x as usize;
        let y = actor.y as usize;
        y < COMBAT_ARENA_SIDE
            && x < COMBAT_ARENA_SIDE
            && is_probe_walkable(self.combat_terrain[y][x])
    }

    pub fn spell_allowed_in_current_cast_context(&self, spell_index: usize) -> bool {
        if spell_index >= SPELL_COUNT {
            return false;
        }
        if self.combat_active {
            SPELL_SCENE_MASKS[spell_index] & SPELL_SCENE_COMBAT != 0
        } else {
            spell_allowed_in_area(spell_index, self.area)
        }
    }

    pub fn combat_arena_field_placement_callback_accepts(
        &mut self,
        caster_index: usize,
        target_slot: usize,
        spell_index: usize,
    ) -> bool {
        let _ = (caster_index, target_slot, spell_index);
        self.random_mod_u8(2) == 0
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

    pub fn combat_spell_target_defense_value(&self, target_slot: usize) -> u8 {
        if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            resolve_protection_defense_bonus(
                CHARACTER_DEFENSE_FACTORY_SEED,
                self.active_effect_tag,
                self.active_effect_counter,
            )
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
        )?;
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
            z: self
                .active_objects
                .get(self.combat_actors.get(target_slot)?.active_object_slot as usize)
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

    pub fn cast_combat_arena_field_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        field: CombatArenaFieldKind,
        direction: Option<Direction>,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(direction) = direction else {
            self.message = "Direction? Use C1FGI6/C1GIN6/C1GIZ6/C1GIS6.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Field placement requires a cardinal direction.".to_string();
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
        let (dx, dy) = direction.delta();
        let target_x = caster_actor.x as isize + dx;
        let target_y = caster_actor.y as isize + dy;
        if !(0..COMBAT_ARENA_SIDE as isize).contains(&target_x)
            || !(0..COMBAT_ARENA_SIDE as isize).contains(&target_y)
        {
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        let target_x = target_x as u8;
        let target_y = target_y as u8;
        let target_slot = find_combat_actor_at_field_coordinate(
            &self.combat_actors,
            &self.active_objects,
            target_x,
            target_y,
        );
        let callback_accepts = target_slot.is_some_and(|slot| {
            self.combat_arena_field_placement_callback_accepts(caster_index, slot, spell_index)
        });
        let applied =
            self.apply_combat_arena_field_placement(field, target_x, target_y, callback_accepts);
        if let Some(placement) = applied {
            let poison_damage_roll = self.combat_arena_field_poison_damage_roll();
            let fire_damage_roll = self.combat_arena_field_fire_damage_roll();
            let defense_roll = self.combat_arena_field_defense_roll(placement.target_slot);
            let _ = self.apply_combat_arena_field_contact(
                field,
                caster_index,
                placement.target_slot,
                poison_damage_roll,
                fire_damage_roll,
                defense_roll,
            );
        }

        self.advance_turn();
        self.message = if applied.is_some() {
            format!("{} field placed.", field.label())
        } else {
            "Failed!".to_string()
        };
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
        self.active_objects
            .iter()
            .enumerate()
            .find_map(|(slot, object)| {
                if object.x != usize::from(target_x) || object.y != usize::from(target_y) {
                    return None;
                }
                CombatArenaFieldKind::from_kind_byte(object.type_byte).map(|field| (slot, field))
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

        let applied = self.apply_combat_polymorph_giant_rat(target_slot);
        self.advance_turn();
        self.message = if applied.is_some() {
            "Polymorph!".to_string()
        } else {
            "Failed!".to_string()
        };
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

        Some(CombatCharmApplication {
            target_slot,
            flags_before,
            flags_after,
        })
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

        let target_actor = self
            .combat_actors
            .get(target_slot)
            .copied()
            .unwrap_or_default();
        let caster_group = self.combat_target_group_for_slot(caster_index);
        let target_group = self.combat_target_group_for_slot(target_slot);
        if !creature_prompt_target_is_eligible(target_actor, target_group, caster_group, false) {
            self.message = "Target? Use C1AEX7 to target a hostile creature.".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        let applied = self.apply_combat_charm_allegiance(target_slot);
        self.advance_turn();
        self.message = if applied.is_some() {
            "Charm!".to_string()
        } else {
            "Failed!".to_string()
        };
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
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

    pub fn apply_combat_summon_class_with_legal_mask(
        &mut self,
        class: u8,
        z: i8,
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
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
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
            &legal_cells,
            candidate_coordinates,
        )?;
        self.message = "Monster summons daemon.".to_string();
        Some(CombatAiSpecialApplication::SummonDaemon { actor_slot, summon })
    }

    pub fn apply_combat_ai_summon_daemon_special(
        &mut self,
        actor_slot: usize,
        seed: u8,
    ) -> Option<CombatAiSpecialApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_active_not_dead(actor) {
            return None;
        }
        let candidates = combat_neighbor_candidate_coordinates(actor.x, actor.y, seed);
        self.apply_combat_ai_summon_daemon_special_with_candidates(actor_slot, &candidates)
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
                    self.party.get(slot).copied(),
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
            target_slot,
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
        if !self.combat_active || actor_slot < COMBAT_PARTY_ACTOR_SLOTS {
            return None;
        }
        let actor = *self.combat_actors.get(actor_slot)?;
        if !combat_actor_is_active_not_dead(actor) {
            return None;
        }

        let class = actor.owner_target_class;
        let mut special = None;
        let mut possess_hook_handled = false;
        let mut summon_hook_pending = false;
        let summon_can_place_daemon = if combat_class_traits(class)
            .is_some_and(|traits| traits.summon_daemon)
        {
            let legal_cells = self.combat_legal_cell_mask();
            resolve_combat_clone_placement_coordinate(&legal_cells, summon_candidate_coordinates)
                .is_some()
                && resolve_clone_spell_allocation(&self.combat_actors, &self.active_objects)
                    .is_some()
        } else {
            false
        };

        match resolve_combat_ai_special_hook(
            class,
            possess_candidate_reaches_resistance,
            blink_roll,
            summon_roll,
            summon_can_place_daemon,
        ) {
            Some(CombatAiSpecialHook::Possess) => {
                special = self.apply_combat_ai_possess_special_with_inputs(
                    actor_slot,
                    possess_target_slot,
                    possess_resistance_blocks,
                );
                possess_hook_handled = special.is_some();
            }
            Some(CombatAiSpecialHook::Blink) => {
                special = self.apply_combat_ai_blink_special(actor_slot);
            }
            Some(CombatAiSpecialHook::SummonDaemon) => {
                summon_hook_pending = true;
            }
            None => {}
        }

        let normal_group = self.combat_target_group_for_slot(actor_slot);
        let acting_group = if active_effect_is_active(
            self.active_effect_tag,
            self.active_effect_counter,
            MASS_CHARM_ACTIVE_EFFECT_TAG,
        ) {
            combat_class_stats(class)
                .map(|stats| {
                    resolve_mass_charm_target_group(
                        normal_group,
                        stats.mass_charm_threshold(),
                        mass_charm_roll,
                    )
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
                    special,
                    possess_hook_handled,
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
        if summon_hook_pending {
            let mut directional_candidates = combat_step_direction_candidate_coordinates(
                actor.x,
                actor.y,
                step_vector,
                self.combat_ai_summon_roll(actor_slot),
            );
            for candidate in summon_candidate_coordinates.iter().copied() {
                if !directional_candidates.contains(&candidate) {
                    directional_candidates.push(candidate);
                }
            }
            special = self.apply_combat_ai_summon_daemon_special_with_candidates(
                actor_slot,
                &directional_candidates,
            );
        }
        let target_range = target_slot.map(|slot| actor.range_to(self.combat_actors[slot]));
        let attack_route =
            target_range.and_then(|range| resolve_combat_ai_attack_route(class, range));
        if matches!(
            attack_route,
            Some(CombatAiAttackRoute::Melee | CombatAiAttackRoute::RangedEffect { .. })
        ) {
            let monster_attack = target_slot.and_then(|target_slot| {
                monster_attack_inputs.and_then(|inputs| {
                    if matches!(attack_route, Some(CombatAiAttackRoute::RangedEffect { .. }))
                        && self
                            .combat_monster_amulet_turning_scatter_applies(actor_slot, target_slot)
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
                special,
                possess_hook_handled,
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
            traits.teleport_capable,
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
            let _ = self.apply_combat_arena_field_contact_for_actor_position(actor_slot);
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
            special,
            possess_hook_handled,
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

    pub fn apply_combat_swarm_with_legal_mask(
        &mut self,
        z: i8,
        legal_cells: &[[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        candidate_coordinates: &[(u8, u8)],
    ) -> Vec<CombatSummonApplication> {
        let mut accepted = Vec::new();
        let mut remaining_legal = *legal_cells;
        for &(x, y) in candidate_coordinates {
            if accepted.len() >= 8 {
                break;
            }
            if !combat_ai_legal_cell(&remaining_legal, i16::from(x), i16::from(y)) {
                continue;
            }
            let Some(application) = self.apply_combat_summon_class_with_legal_mask(
                COMBAT_CLASS_INSECT_SWARM,
                z,
                &remaining_legal,
                &[(x, y)],
            ) else {
                break;
            };
            remaining_legal[usize::from(y)][usize::from(x)] = false;
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

        let placement_seed = self.combat_neighbor_placement_seed();
        let class = resolve_conjure_spell_class(self.combat_conjure_class_selector());
        let applied =
            self.apply_combat_summon_class_around_slot(class, caster_index, placement_seed);
        self.advance_turn();
        self.message = if applied.is_some() {
            "Success!".to_string()
        } else {
            "Failed!".to_string()
        };
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

        let seed = self.combat_arena_placement_seed();
        let legal_cells = self.combat_legal_cell_mask();
        let candidates = combat_clone_candidate_coordinates(seed);
        let applied = self.apply_combat_swarm_with_legal_mask(
            self.combat_actor_z(caster_index),
            &legal_cells,
            &candidates,
        );
        self.advance_turn();
        self.message = if applied.is_empty() {
            "Failed!".to_string()
        } else {
            "Success!".to_string()
        };
        if applied.is_empty() {
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

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        let seed = self.combat_neighbor_placement_seed();
        let applied =
            self.apply_combat_summon_class_around_slot(COMBAT_CLASS_DAEMON, caster_index, seed);
        self.advance_turn();
        self.message = if applied.is_some() {
            "Summon Daemon!".to_string()
        } else {
            "Failed!".to_string()
        };
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
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
        if applied.is_some() {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn combat_legal_cell_mask(&self) -> [[bool; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE] {
        build_combat_ai_legal_cell_mask(
            &self.combat_terrain,
            &self.combat_actors,
            is_probe_walkable,
        )
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
            enter_endgame_after_successful_combat: false,
            endgame_messages: None,
        };
        self.active_objects = active_objects;
        self.combat_actors = actors;
        self.combat_terrain = terrain;
        self.combat_active = true;
        self.combat_frame_snapshot = Some(snapshot.clone());
        self.pending_combat_actor_slot = None;
        self.pending_combat_terrain_trigger_slot = None;
        self.next_combat_actor_slot = 0;
        self.combat_potion_presentation = None;
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
        self.combat_active = false;
        self.combat_frame_snapshot = None;
        self.pending_combat_actor_slot = None;
        self.next_combat_actor_slot = 0;
        self.combat_potion_presentation = None;
        if let Some(slot) = pending_terrain_trigger {
            reconcile_post_combat_terrain_trigger_slot(
                &mut self.active_objects,
                slot,
                body_retrieval_exit,
            );
        }
        self.mark_visibility_dirty();
    }

    pub fn apply_combat_round_loop_exit(
        &mut self,
        exit: CombatRoundLoopExit,
    ) -> CombatRoundLoopExitApplication {
        let result_code = exit.result_code();
        let body_retrieval_exit =
            combat_exit_requests_body_retrieval_reconcile(exit, &self.combat_actors);
        let restored_snapshot = if let Some(snapshot) = self.combat_frame_snapshot.take() {
            let enter_endgame_after_restore =
                snapshot.enter_endgame_after_successful_combat && body_retrieval_exit;
            let endgame_messages = snapshot.endgame_messages.clone();
            self.restore_combat_frame_with_trigger_reconcile(snapshot, body_retrieval_exit);
            if enter_endgame_after_restore {
                self.enter_endgame_with_messages(endgame_messages);
            }
            true
        } else {
            self.combat_active = false;
            self.pending_combat_actor_slot = None;
            self.combat_potion_presentation = None;
            if let Some(slot) = self.pending_combat_terrain_trigger_slot.take() {
                reconcile_post_combat_terrain_trigger_slot(
                    &mut self.active_objects,
                    slot,
                    body_retrieval_exit,
                );
            }
            self.next_combat_actor_slot = 0;
            self.combat_actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
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
        let outcome = apply_combat_party_damage(self.party.get_mut(slot)?, raw_damage);
        if outcome.killed && self.active_player == Some(slot) {
            self.active_player = None;
        }
        Some(outcome)
    }

    pub fn credit_combat_party_attacker_experience(
        &mut self,
        attacker_slot: usize,
        reward: u8,
    ) -> Option<u16> {
        if attacker_slot >= COMBAT_PARTY_ACTOR_SLOTS || !self.party.get(attacker_slot)?.living() {
            return None;
        }

        if self.party_experience.len() < self.party.len() {
            self.party_experience.resize(self.party.len(), 0);
        }
        let experience = self.party_experience.get_mut(attacker_slot)?;
        *experience = apply_combat_experience_reward(*experience, reward);
        Some(*experience)
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
        if target_slot >= COMBAT_ACTOR_SLOTS
            || !self
                .combat_actors
                .get(target_slot)
                .copied()
                .is_some_and(combat_actor_is_active_not_dead)
        {
            self.message = "Target? Use C1GP7 to target a live combat slot.".to_string();
            return MoveOutcome::Blocked;
        }

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

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
        let applied = self.apply_active_target_combat_spell_damage(
            Some(caster_index),
            target_slot,
            kind,
            raw_roll,
            defense_roll,
        );

        self.advance_turn();
        let succeeded = applied.is_some();
        self.message = match (kind, succeeded) {
            (CombatSpellDamageKind::MagicMissile, true) => "Magic Missile!".to_string(),
            (CombatSpellDamageKind::Fireball, true) => "Fireball!".to_string(),
            (CombatSpellDamageKind::Kill, true) => "Kill!".to_string(),
            _ => "Failed!".to_string(),
        };
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

        let gate_accepts = [true; COMBAT_ACTOR_SLOTS];
        let target_slots = collect_tremor_spell_actor_slots(&self.combat_actors, &gate_accepts);
        let damage_rolls = target_slots
            .iter()
            .copied()
            .map(|_| self.combat_spell_damage_roll_for_kind(CombatSpellDamageKind::Tremor))
            .collect::<Vec<_>>();
        let applied =
            self.apply_tremor_combat_spell_damage(Some(caster_index), &gate_accepts, &damage_rolls);

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
                    if let Some(actor) = self.combat_actors.get_mut(slot) {
                        actor.set_status_disabled();
                    }
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
        target_slot: usize,
        effect: CombatDirectedSpellEffect,
    ) -> Option<Vec<(u8, u8)>> {
        let caster = self.combat_actors.get(caster_index).copied()?;
        let target = self.combat_actors.get(target_slot).copied()?;
        if !combat_actor_is_active_not_dead(caster) || !directed_spell_actor_is_eligible(target) {
            return None;
        }
        if matches!(effect, CombatDirectedSpellEffect::Sleep) {
            return Some(vec![(target.x, target.y)]);
        }

        let delta_x = target.x as i16 - caster.x as i16;
        let delta_y = target.y as i16 - caster.y as i16;
        let (forward_x, forward_y) = if delta_x.abs() >= delta_y.abs() {
            (delta_x.signum(), 0)
        } else {
            (0, delta_y.signum())
        };
        if forward_x == 0 && forward_y == 0 {
            return Some(vec![(caster.x, caster.y)]);
        }
        let (side_x, side_y) = (-forward_y, forward_x);
        let mut cells = Vec::new();
        for distance in 1..COMBAT_ARENA_SIDE as i16 {
            let lateral_radius = (distance / 2).min(2);
            for lateral in -lateral_radius..=lateral_radius {
                let x = caster.x as i16 + forward_x * distance + side_x * lateral;
                let y = caster.y as i16 + forward_y * distance + side_y * lateral;
                if (0..COMBAT_ARENA_SIDE as i16).contains(&x)
                    && (0..COMBAT_ARENA_SIDE as i16).contains(&y)
                {
                    let cell = (x as u8, y as u8);
                    if !cells.contains(&cell) {
                        cells.push(cell);
                        if cells.len() == DIRECTED_TARGET_WALK_MAX_CELLS {
                            return Some(cells);
                        }
                    }
                }
            }
        }
        Some(cells)
    }

    pub fn cast_directed_combat_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        effect: CombatDirectedSpellEffect,
        target_slot: usize,
    ) -> MoveOutcome {
        if !self.combat_active || !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if target_slot >= COMBAT_ACTOR_SLOTS
            || !self
                .combat_actors
                .get(target_slot)
                .copied()
                .is_some_and(directed_spell_actor_is_eligible)
        {
            self.message = "Target? Use C1IZ7 to target a live visible combat slot.".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(target_cells) =
            self.directed_combat_spell_target_cells(caster_index, target_slot, effect)
        else {
            self.message = "Target? Use C1IZ7 to target a live visible combat slot.".to_string();
            return MoveOutcome::Blocked;
        };

        let mana_cost = (spell_index / 6 + 1) as u8;
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        let target_slots = collect_directed_spell_actor_slots(&self.combat_actors, &target_cells);
        let applied = match effect {
            CombatDirectedSpellEffect::Sleep => self
                .apply_directed_combat_spell_status(effect, &target_cells, &[], &[])
                .map(|application| !application.applications.is_empty()),
            CombatDirectedSpellEffect::PoisonWind => {
                let poison_rolls = target_slots
                    .iter()
                    .copied()
                    .map(|_| self.combat_arena_field_poison_damage_roll())
                    .collect::<Vec<_>>();
                let gate_accepts = poison_rolls
                    .iter()
                    .copied()
                    .map(|roll| roll & 1 == 0)
                    .collect::<Vec<_>>();
                self.apply_directed_combat_spell_status(
                    effect,
                    &target_cells,
                    &gate_accepts,
                    &poison_rolls,
                )
                .map(|application| !application.applications.is_empty())
            }
            CombatDirectedSpellEffect::DeathWind | CombatDirectedSpellEffect::FlameWind => {
                let damage_rolls = match effect {
                    CombatDirectedSpellEffect::FlameWind => target_slots
                        .iter()
                        .map(|_| {
                            self.combat_spell_damage_roll_for_kind(CombatSpellDamageKind::FlameWind)
                        })
                        .collect::<Vec<_>>(),
                    CombatDirectedSpellEffect::DeathWind => Vec::new(),
                    CombatDirectedSpellEffect::Sleep | CombatDirectedSpellEffect::PoisonWind => {
                        Vec::new()
                    }
                };
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

        let target_slots = collect_repel_undead_actor_slots(&self.combat_actors);
        let mut affected = 0usize;
        for slot in target_slots {
            if self
                .apply_combat_weapon_damage_to_target(
                    Some(caster_index),
                    slot,
                    COMBAT_INSTANT_KILL_DAMAGE,
                    true,
                )
                .is_some()
            {
                affected += 1;
            }
        }

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
        current_active_slot: usize,
        target_slot: usize,
        poison_damage_roll: u8,
        fire_damage_roll: u8,
        defense_roll: u8,
    ) -> Option<CombatArenaFieldContactApplication> {
        let contact_outcome = if current_active_slot == target_slot {
            CombatArenaFieldContactOutcome::SkippedCurrentActor
        } else {
            let actor = self.combat_actors.get(target_slot)?;
            let linked_active_object_tile = self
                .active_objects
                .get(actor.active_object_slot as usize)?
                .tile;

            if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
                resolve_combat_arena_field_contact_for_party_target(
                    field,
                    current_active_slot,
                    target_slot,
                    linked_active_object_tile,
                    self.party.get_mut(target_slot)?,
                    poison_damage_roll,
                    fire_damage_roll,
                )
            } else {
                resolve_combat_arena_field_contact_for_non_party_target(
                    field,
                    current_active_slot,
                    target_slot,
                    linked_active_object_tile,
                    poison_damage_roll,
                    fire_damage_roll,
                )
            }
        };

        if matches!(
            contact_outcome,
            CombatArenaFieldContactOutcome::SleepDisabledNonParty
        ) {
            if let Some(actor) = self.combat_actors.get_mut(target_slot) {
                actor.set_status_disabled();
            }
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
                let defended_damage =
                    resolve_spell_damage_after_defense(raw_damage as i16, defense_roll);
                Some(self.apply_combat_weapon_damage_to_target(
                    None,
                    target_slot,
                    defended_damage,
                    true,
                )?)
            }
            CombatArenaFieldContactOutcome::EnergyDamage { raw_damage } => {
                Some(self.apply_combat_weapon_damage_to_target(
                    None,
                    target_slot,
                    raw_damage as i16,
                    true,
                )?)
            }
            CombatArenaFieldContactOutcome::SkippedCurrentActor
            | CombatArenaFieldContactOutcome::PoisonSkippedByLinkedTileClass
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

    pub fn apply_combat_arena_field_contact_for_actor_position(
        &mut self,
        actor_slot: usize,
    ) -> Option<CombatArenaFieldContactApplication> {
        let actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_active_not_dead(actor) {
            return None;
        }
        let (_, field) = self.find_combat_arena_field_marker(actor.x, actor.y)?;
        let poison_damage_roll = self.combat_arena_field_poison_damage_roll();
        let fire_damage_roll = self.combat_arena_field_fire_damage_roll();
        let defense_roll = self.combat_arena_field_defense_roll(actor_slot);
        let application = self.apply_combat_arena_field_contact(
            field,
            COMBAT_FIELD_CONTACT_NO_ACTIVE_SKIP_SLOT,
            actor_slot,
            poison_damage_roll,
            fire_damage_roll,
            defense_roll,
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
        let defender_rating = if target_slot < COMBAT_PARTY_ACTOR_SLOTS {
            party_defender_rating
        } else {
            combat_class_stats(target.owner_target_class)?.defense
        };

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
        &self,
        attacker_slot: usize,
        target_slot: usize,
    ) -> bool {
        if target_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(attacker) = self.combat_actors.get(attacker_slot).copied() else {
            return false;
        };
        let Some(target) = self.party.get(target_slot).copied() else {
            return false;
        };
        let Some(equipment) = self.party_equipment.get(target_slot) else {
            return false;
        };
        let roll = self.combat_monster_amulet_turning_roll(attacker_slot, target_slot);
        resolve_amulet_turning_scatter_for_party_target(
            attacker.owner_target_class,
            target,
            equipment,
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
        Some(is_probe_walkable(
            self.combat_terrain[destination.y as usize][destination.x as usize],
        ))
    }

    pub fn apply_combat_player_command_with_inputs(
        &mut self,
        actor_slot: usize,
        input: CombatPlayerCommandInput,
        quickness_roll: u8,
    ) -> Option<CombatPlayerCommandApplication> {
        let weapon_attack_inputs = self.combat_player_weapon_attack_inputs(actor_slot);
        self.apply_combat_player_command_with_attack_inputs(
            actor_slot,
            input,
            quickness_roll,
            weapon_attack_inputs,
        )
    }

    pub fn combat_player_weapon_attack_inputs(
        &self,
        attacker_slot: usize,
    ) -> CombatPlayerWeaponAttackInputs {
        CombatPlayerWeaponAttackInputs {
            hit_roll: (self.turn as u8).wrapping_add(attacker_slot as u8),
            damage_roll: (self.turn as u8).wrapping_add((attacker_slot as u8).wrapping_mul(3)),
            forced_hit: None,
        }
    }

    pub fn combat_quickness_dispatch_roll(&self, actor_slot: usize) -> u8 {
        (self.turn as u8).wrapping_add(actor_slot as u8) & 1
    }

    pub fn combat_magic_ring_regeneration_roll(&self, actor_slot: usize) -> u8 {
        (self.turn as u8).wrapping_add((actor_slot as u8).wrapping_mul(5)) & 0x07
    }

    pub fn combat_magic_ring_vanish_roll(&self, actor_slot: usize) -> u8 {
        (self.turn as u8)
            .wrapping_add((actor_slot as u8).wrapping_mul(13))
            .wrapping_add(1)
            & 0x0f
    }

    pub fn apply_visible_combat_magic_ring_pass_to_slot(
        &mut self,
        slot: usize,
    ) -> Option<CombatMagicRingPassOutcome> {
        let outcome = self.apply_combat_magic_ring_pass_to_slot(
            slot,
            self.combat_magic_ring_regeneration_roll(slot),
            self.combat_magic_ring_vanish_roll(slot),
        )?;
        (outcome != CombatMagicRingPassOutcome::default()).then_some(outcome)
    }

    pub fn apply_combat_player_command_with_attack_inputs(
        &mut self,
        actor_slot: usize,
        input: CombatPlayerCommandInput,
        quickness_roll: u8,
        weapon_attack_inputs: CombatPlayerWeaponAttackInputs,
    ) -> Option<CombatPlayerCommandApplication> {
        if !self.combat_active || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS {
            return None;
        }
        let active_actor = self.combat_actors.get(actor_slot).copied()?;
        if !combat_actor_is_active_not_dead(active_actor) {
            return None;
        }

        if resolve_quickness_dispatch_consumed(
            self.active_effect_tag,
            self.active_effect_counter,
            quickness_roll,
        ) {
            let ring_pass = self.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
            return Some(CombatPlayerCommandApplication {
                actor_slot,
                input,
                action: CombatPlayerCommandAction::QuicknessSkipped,
                weapon_attack: None,
                ring_pass,
                control_after: self.combat_round_loop_control(false, false),
            });
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
                        CombatCommandBranch::QuitDefeat => CombatPlayerCommandAction::QuitDefeat,
                        CombatCommandBranch::XitCleanup => CombatPlayerCommandAction::XitCleanup {
                            allowed: self.combat_xit_cleanup_allowed(),
                        },
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

        let mut control_after = match action {
            CombatPlayerCommandAction::QuitDefeat => resolve_combat_quit_command(),
            CombatPlayerCommandAction::XitCleanup { allowed: true } => {
                CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
            }
            CombatPlayerCommandAction::StepOrAttack {
                direction_code,
                outcome: CombatStepOrAttackPrimitiveOutcome::OutOfArena { .. },
                ..
            } if matches!(
                resolve_combat_out_of_arena_leave(
                    false,
                    direction_code,
                    false,
                    false,
                    None,
                    combat_has_active_not_dead_non_party_actor(&self.combat_actors),
                ),
                CombatOutOfArenaLeaveOutcome::Accepted { .. }
            ) =>
            {
                CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat)
            }
            _ => self.combat_round_loop_control(false, false),
        };
        let weapon_attack = self.apply_combat_player_weapon_attack_for_action(
            actor_slot,
            &action,
            weapon_attack_inputs,
        );
        let ring_pass = self.apply_visible_combat_magic_ring_pass_to_slot(actor_slot);
        if matches!(control_after, CombatRoundLoopControl::ContinueActorWalk)
            && resolve_combat_victory(&self.combat_actors)
        {
            control_after = CombatRoundLoopControl::Exit(CombatRoundLoopExit::LeaveCombat);
        }

        Some(CombatPlayerCommandApplication {
            actor_slot,
            input,
            action,
            weapon_attack,
            ring_pass,
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

    pub fn combat_xit_cleanup_allowed(&self) -> bool {
        resolve_combat_xit_cleanup_allowed(&self.combat_actors)
    }

    pub fn combat_round_loop_control(
        &self,
        leave_combat_flag: bool,
        exhausted_slots: bool,
    ) -> CombatRoundLoopControl {
        resolve_combat_round_loop_control(
            resolve_combat_defeat(&self.party, &self.combat_actors),
            leave_combat_flag || resolve_combat_victory(&self.combat_actors),
            exhausted_slots,
        )
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
        if slot >= COMBAT_ACTOR_SLOTS {
            return CombatActorSlotDispatchApplication::EndOfRound {
                control: self.combat_round_loop_control(leave_combat_flag, true),
            };
        }

        let actor = self.combat_actors[slot];
        if !combat_actor_is_active_not_dead(actor) {
            return CombatActorSlotDispatchApplication::Slot {
                slot,
                phase_tick: Some(CombatActorPhaseTick::Inactive),
                action: CombatActorDispatchAction::Inactive,
                control_after: self.combat_round_loop_control(leave_combat_flag, false),
            };
        }

        if !self.combat_actor_stands_on_walkable_arena_cell(actor) {
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

        let action = if slot < COMBAT_PARTY_ACTOR_SLOTS {
            CombatActorDispatchAction::PlayerReady
        } else {
            let monster_attack_inputs = monster_attack_inputs_by_slot
                .iter()
                .find_map(|&(input_slot, inputs)| (input_slot == slot).then_some(inputs));
            CombatActorDispatchAction::MonsterAi {
                ai_turn: self.apply_combat_ai_turn_with_inputs(
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
                ),
            }
        };

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
        if slot >= COMBAT_ACTOR_SLOTS {
            return CombatActorSlotDispatchApplication::EndOfRound {
                control: self.combat_round_loop_control(leave_combat_flag, true),
            };
        }

        let possess_target_slot = self.combat_ai_possess_target_slot_roll(slot);
        let possess_candidate_reaches_resistance =
            self.combat_ai_possess_candidate_reaches_resistance_from_roll(possess_target_slot);
        let possess_resistance_blocks =
            self.combat_ai_possess_resistance_blocks(slot, possess_target_slot);
        let summon_candidate_coordinates = self.combat_ai_summon_candidate_coordinates(slot);
        let random_cardinal_direction_codes = self.combat_ai_random_cardinal_direction_codes(slot);
        let monster_attack_inputs = self.combat_monster_attack_inputs(slot);
        let monster_attack_inputs_by_slot = [(slot, monster_attack_inputs)];

        self.apply_combat_actor_slot_dispatch_with_inputs(
            slot,
            refresh_constant,
            leave_combat_flag,
            possess_candidate_reaches_resistance,
            possess_target_slot,
            possess_resistance_blocks,
            self.combat_ai_blink_roll(slot),
            self.combat_ai_summon_roll(slot),
            &summon_candidate_coordinates,
            None,
            self.combat_ai_mass_charm_roll(slot),
            false,
            None,
            self.combat_ai_horizontal_axis_first(slot),
            &random_cardinal_direction_codes,
            &monster_attack_inputs_by_slot,
        )
    }

    #[allow(clippy::too_many_arguments)]
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
        let mut applications = Vec::new();
        let mut slot = start_slot;
        loop {
            let application =
                self.apply_combat_actor_slot_dispatch(slot, refresh_constant, leave_combat_flag);

            match &application {
                CombatActorSlotDispatchApplication::EndOfRound { .. } => {
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

    pub fn combat_ai_possess_target_slot_roll(&self, actor_slot: usize) -> usize {
        (self.turn as usize)
            .wrapping_add(actor_slot)
            .wrapping_add(usize::from(self.combat_round_counter))
            % COMBAT_ACTOR_SLOTS
    }

    pub fn combat_ai_possess_resistance_blocks(
        &self,
        actor_slot: usize,
        target_slot: usize,
    ) -> bool {
        (self.turn as u8)
            .wrapping_add(actor_slot as u8)
            .wrapping_add(target_slot as u8)
            & 1
            != 0
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
                    self.party.get(target_slot).copied(),
                    false,
                    false,
                )
            })
        else {
            return false;
        };
        combat_possess_candidate_reaches_resistance(target_slot, candidate, self.active_player)
    }

    pub fn combat_ai_blink_roll(&self, actor_slot: usize) -> u8 {
        (self.turn as u8)
            .wrapping_add((actor_slot as u8).wrapping_mul(3))
            .wrapping_add(self.combat_round_counter)
    }

    pub fn combat_ai_summon_roll(&self, actor_slot: usize) -> u8 {
        (self.turn as u8)
            .wrapping_add((actor_slot as u8).wrapping_mul(5))
            .wrapping_add(self.combat_round_counter)
    }

    pub fn combat_ai_summon_candidate_coordinates(&self, actor_slot: usize) -> Vec<(u8, u8)> {
        let Some(actor) = self.combat_actors.get(actor_slot).copied() else {
            return Vec::new();
        };
        combat_neighbor_candidate_coordinates(
            actor.x,
            actor.y,
            self.combat_ai_summon_roll(actor_slot),
        )
    }

    pub fn combat_ai_mass_charm_roll(&self, actor_slot: usize) -> u8 {
        (self.turn as u8)
            .wrapping_add((actor_slot as u8).wrapping_mul(7))
            .wrapping_add(self.combat_round_counter)
    }

    pub fn combat_ai_horizontal_axis_first(&self, actor_slot: usize) -> bool {
        (self.turn as u8)
            .wrapping_add(actor_slot as u8)
            .wrapping_add(self.combat_round_counter)
            & 1
            == 0
    }

    pub fn combat_ai_random_cardinal_direction_codes(&self, actor_slot: usize) -> [u8; 4] {
        let base = [1, 2, 3, 4];
        let start = usize::from(
            (self.turn as u8)
                .wrapping_add(actor_slot as u8)
                .wrapping_add(self.combat_round_counter)
                & 3,
        );
        [
            base[start],
            base[(start + 1) % base.len()],
            base[(start + 2) % base.len()],
            base[(start + 3) % base.len()],
        ]
    }

    pub fn combat_monster_attack_inputs(&self, attacker_slot: usize) -> CombatMonsterAttackInputs {
        CombatMonsterAttackInputs {
            party_defender_rating: 7,
            hit_roll: (self.turn as u8).wrapping_add(attacker_slot as u8),
            damage_roll: (self.turn as u8).wrapping_add((attacker_slot as u8).wrapping_mul(3)),
            poison_gate_accepts: (self.turn as u8)
                .wrapping_add((attacker_slot as u8).wrapping_mul(11))
                & 1
                == 0,
            poison_damage_roll: (self.turn as u8)
                .wrapping_add((attacker_slot as u8).wrapping_mul(13)),
            forced_hit: None,
            amulet_turning_scatter_roll: (self.turn as u8)
                .wrapping_add((attacker_slot as u8).wrapping_mul(23)),
        }
    }

    pub fn combat_monster_amulet_turning_roll(
        &self,
        attacker_slot: usize,
        target_slot: usize,
    ) -> u8 {
        (self.turn as u8)
            .wrapping_add((attacker_slot as u8).wrapping_mul(7))
            .wrapping_add((target_slot as u8).wrapping_mul(19))
    }

    pub fn ensure_pending_combat_player_turn(&mut self) -> Option<CombatRoundWalkApplication> {
        if !self.combat_active || self.pending_combat_actor_slot.is_some() {
            return None;
        }

        let mut last_application = None;
        for _ in 0..COMBAT_ACTOR_SLOTS {
            let start_slot = self.next_combat_actor_slot.min(COMBAT_ACTOR_SLOTS);
            let application = self.apply_combat_round_walk_from_slot(start_slot, 30, false);
            self.next_combat_actor_slot = match application.stop_reason {
                CombatRoundWalkStopReason::EndOfRound => 0,
                CombatRoundWalkStopReason::AwaitingPlayer | CombatRoundWalkStopReason::Exit => {
                    application.next_slot
                }
            };
            if application.stop_reason == CombatRoundWalkStopReason::AwaitingPlayer {
                self.pending_combat_actor_slot = ready_player_slot_from_round_walk(&application);
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
            let _ = self.apply_combat_arena_field_contact_for_actor_position(moving_slot);
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
        if slot >= COMBAT_PARTY_ACTOR_SLOTS {
            return None;
        }
        let wearer = *self.party.get(slot)?;
        self.combat_actors.get(slot)?;
        if self.party_equipment.len() < self.party.len() {
            self.party_equipment
                .resize(self.party.len(), [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]);
        }

        let ring = self.party_equipment[slot][EQUIP_SLOT_RING];
        let mut outcome = CombatMagicRingPassOutcome::default();
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
            outcome.regeneration_applied = self.party[slot].heal_by(regeneration);
        }

        if combat_magic_ring_vanishes(ring, vanish_roll) {
            self.party_equipment[slot][EQUIP_SLOT_RING] = EQUIPMENT_EMPTY;
            outcome.vanished_ring = Some(ring);
            self.message = format!("{} vanished.", equipment_name(ring as usize));
            if ring as usize == EQUIPMENT_ID_RING_INVISIBILITY
                && clear_combat_linked_invisibility(
                    &mut self.combat_actors[slot],
                    &mut self.active_objects,
                )
                .is_some_and(CombatLinkedVisibilityOutcome::changed)
            {
                self.mark_visibility_dirty();
            }
        }

        Some(outcome)
    }

    pub fn advance_combat_round_counter(&mut self) -> CombatRoundCounterTick {
        let tick = resolve_combat_round_counter_tick(self.combat_round_counter);
        self.combat_round_counter = tick.counter;
        if tick.redraw_tiles {
            self.mark_visibility_dirty();
        }
        tick
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

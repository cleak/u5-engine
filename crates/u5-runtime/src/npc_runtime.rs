//! Runtime NPC + door tracker + location markers.

use crate::*;

/// Per `npc-schedules.md §7`: NPC schedule state-machine values `0..=8`.
/// Empty / idle / in-plane move / replaying queue / descend toward target /
/// ascend toward target / climb up off-floor / climb down off-floor /
/// parked off-floor.
pub const NPC_STATE_EMPTY: u8 = 0;
pub const NPC_STATE_IDLE: u8 = 1;
pub const NPC_STATE_INPLANE_MOVE: u8 = 2;
pub const NPC_STATE_REPLAY_QUEUE: u8 = 3;
pub const NPC_STATE_DESCEND_TOWARD_TARGET: u8 = 4;
pub const NPC_STATE_ASCEND_TOWARD_TARGET: u8 = 5;
pub const NPC_STATE_CLIMB_UP_OFF_FLOOR: u8 = 6;
pub const NPC_STATE_CLIMB_DOWN_OFF_FLOOR: u8 = 7;
pub const NPC_STATE_PARKED_OFF_FLOOR: u8 = 8;

/// `npc-schedules.md §7` typed enumeration of the NPC schedule
/// state-machine byte. The dispatcher reads a raw `u8` from the
/// runtime block; this enum lets callers exhaustively match the
/// nine published states without juggling `NPC_STATE_*` integer
/// constants. State byte values outside `0..=8` are not produced by
/// the engine's own writers (initialisation writes 1/0, the
/// boundary trigger writes 1/2/4/5/6/7/8, the pathfinder-success
/// path writes 3, and the world-mutation primitive writes 1); this
/// classifier returns `None` for those out-of-band values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcScheduleState {
    /// `0` — slot is empty; the walker skips before reading state.
    Empty,
    /// `1` — at the active waypoint; boundary trigger may upgrade.
    Idle,
    /// `2` — both NPC and target on the player's floor; probe and
    /// step.
    InPlaneMove,
    /// `3` — replaying a cached path produced by the pathfinder;
    /// pop the next direction byte and apply it.
    ReplayQueue,
    /// `4` — NPC's floor is above the displayed floor and the
    /// waypoint is on it; search the displayed floor for an
    /// ascend-link cell (`0xC8`), route to it, then surface there.
    DescendTowardTarget,
    /// `5` — NPC's floor is below the displayed floor and the
    /// waypoint is on it; search the displayed floor for a
    /// descend-link cell (`0xC9`), route to it, then surface there.
    AscendTowardTarget,
    /// `6` — NPC is on the displayed floor and the waypoint is
    /// above; ask the gate whether it already stands on an ascend
    /// link (`0xC8`) or stairway, else route toward the nearest.
    ClimbUpOffFloor,
    /// `7` — mirror of state 6 for a target below.
    ClimbDownOffFloor,
    /// `8` — neither end of the move is on the player's floor;
    /// the walker has no movement arm for this state.
    ParkedOffFloor,
}

/// `npc-schedules.md §7`: classify a raw state byte. Returns `None`
/// for byte values outside the published `0..=8` set so callers can
/// distinguish a missing slot from corrupted runtime state.
pub const fn npc_schedule_state_classify(state: u8) -> Option<NpcScheduleState> {
    Some(match state {
        NPC_STATE_EMPTY => NpcScheduleState::Empty,
        NPC_STATE_IDLE => NpcScheduleState::Idle,
        NPC_STATE_INPLANE_MOVE => NpcScheduleState::InPlaneMove,
        NPC_STATE_REPLAY_QUEUE => NpcScheduleState::ReplayQueue,
        NPC_STATE_DESCEND_TOWARD_TARGET => NpcScheduleState::DescendTowardTarget,
        NPC_STATE_ASCEND_TOWARD_TARGET => NpcScheduleState::AscendTowardTarget,
        NPC_STATE_CLIMB_UP_OFF_FLOOR => NpcScheduleState::ClimbUpOffFloor,
        NPC_STATE_CLIMB_DOWN_OFF_FLOOR => NpcScheduleState::ClimbDownOffFloor,
        NPC_STATE_PARKED_OFF_FLOOR => NpcScheduleState::ParkedOffFloor,
        _ => return None,
    })
}

impl NpcScheduleState {
    /// `npc-schedules.md §7`: returns the raw state byte the
    /// engine's writers use for this state.
    pub const fn save_byte(self) -> u8 {
        match self {
            Self::Empty => NPC_STATE_EMPTY,
            Self::Idle => NPC_STATE_IDLE,
            Self::InPlaneMove => NPC_STATE_INPLANE_MOVE,
            Self::ReplayQueue => NPC_STATE_REPLAY_QUEUE,
            Self::DescendTowardTarget => NPC_STATE_DESCEND_TOWARD_TARGET,
            Self::AscendTowardTarget => NPC_STATE_ASCEND_TOWARD_TARGET,
            Self::ClimbUpOffFloor => NPC_STATE_CLIMB_UP_OFF_FLOOR,
            Self::ClimbDownOffFloor => NPC_STATE_CLIMB_DOWN_OFF_FLOOR,
            Self::ParkedOffFloor => NPC_STATE_PARKED_OFF_FLOOR,
        }
    }

    /// `npc-schedules.md §7`: returns `true` for the five "probe and
    /// step" states — InPlaneMove, DescendTowardTarget,
    /// AscendTowardTarget, ClimbUpOffFloor, and ClimbDownOffFloor.
    /// These are the states whose tick body probes cardinal
    /// directions and may invoke the flood-fill pathfinder; Idle,
    /// ReplayQueue, Empty, and ParkedOffFloor take other code paths.
    pub const fn is_probe_and_step(self) -> bool {
        matches!(
            self,
            Self::InPlaneMove
                | Self::DescendTowardTarget
                | Self::AscendTowardTarget
                | Self::ClimbUpOffFloor
                | Self::ClimbDownOffFloor,
        )
    }

    /// `npc-schedules.md §7`: returns `true` for states that produce
    /// a visible per-tick action (probe-and-step states plus the
    /// queue-replay state). The walker skips the other states (Empty
    /// is skipped before the state byte is even read; Idle and
    /// ParkedOffFloor have no movement dispatch arm).
    pub const fn produces_visible_step(self) -> bool {
        self.is_probe_and_step() || matches!(self, Self::ReplayQueue)
    }
}

/// `npc-schedules.md §7`: what one per-slot walker step reports back to
/// the pass. The pass needs three answers, not two, because the
/// queue-drain re-entry into state 6/7 "also ends the tick... every slot
/// after the one that triggered it is skipped for that tick", and it is
/// the only path in the walker that leaves the per-slot loop early.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcScheduleStepOutcome {
    /// The slot took no observable action this tick.
    Stalled,
    /// The slot moved, or its sprite link changed; the pass marks the
    /// view dirty once at the end.
    Moved,
    /// The queue drained while the NPC was still in state 3 and the
    /// walker re-entered state 6/7. The pass stops iterating slots.
    EndTick,
}

impl NpcScheduleStepOutcome {
    /// Lift the ordinary "did this slot produce a visible change" bool
    /// into the three-valued outcome.
    pub const fn from_moved(moved: bool) -> Self {
        if moved { Self::Moved } else { Self::Stalled }
    }
}

/// `npc-schedules.md §6` floor-classification mapper. The boundary
/// trigger compares the NPC's current floor, the new waypoint's
/// floor, and the location's current floor and chooses the new
/// state-machine value:
///
/// - both on map -> in-plane move (2)
/// - NPC on map, target above -> climb-up off-floor (6)
/// - NPC on map, target below -> climb-down off-floor (7)
/// - NPC above, target on map -> descend toward target (4)
/// - NPC below, target on map -> ascend toward target (5)
/// - neither on map -> parked off-floor (8)
///
/// The floor index grows upward and the ordering test is signed, so
/// the basement byte `0xFF` orders below `0x00`. This is an alias of
/// [`schedule_floor_state`]; both names resolve to one classifier so
/// the two can never drift apart.
pub const fn npc_schedule_state_for_floor_transition(
    npc_z: u8,
    target_z: u8,
    map_current_floor: u8,
) -> u8 {
    schedule_floor_state(npc_z, target_z, map_current_floor)
}
/// `npc-schedules.md §8.1` pathfinder workspace shape. The flood-fill
/// pathfinder operates on a 32x32 byte scratch grid (1,024 bytes
/// total) keyed by `(row, col)` in row-major order. The workspace
/// is rebuilt from scratch on every pathfinding call — it carries
/// no incremental state between ticks. The workspace mirrors the
/// town grid; anchor to [`crate::TOWN_GRID_SIDE`] so the
/// pathfinder workspace and the town grid share one value.
pub const NPC_PATHFIND_WORKSPACE_SIDE: usize = crate::TOWN_GRID_SIDE;
pub const NPC_PATHFIND_WORKSPACE_LEN: usize =
    NPC_PATHFIND_WORKSPACE_SIDE * NPC_PATHFIND_WORKSPACE_SIDE;

/// `npc-schedules.md §7`: returns `true` when the NPC's state byte
/// represents a parked/off-floor or empty slot — the per-tick
/// walker has no movement dispatch arm for these states. Empty (0)
/// and parked (8) both qualify; every other state byte is a live
/// movement state the dispatcher acts on.
pub const fn npc_state_off_floor_or_empty(state: u8) -> bool {
    matches!(state, NPC_STATE_EMPTY | NPC_STATE_PARKED_OFF_FLOOR)
}

/// `formats/npc.md §6` published type-byte classes. The type byte
/// at `+0x200..+0x21F` doubles as the slot's occupancy flag and the
/// NPC's sprite/tile class. Three values are special-cased by the
/// engine; every other non-zero byte is an ordinary sprite-class
/// value derived by adding the byte to the NPC sprite page.
pub const NPC_TYPE_EMPTY: u8 = 0x00;
pub const NPC_TYPE_DEFAULT_HUMAN_SPRITE: u8 = 0x01;
pub const NPC_TYPE_SHADOWLORD_ACTOR: u8 = SHADOWLORD_ACTOR_TILE;

/// `formats/npc.md §6`: classify a roster type byte. Combines the
/// occupancy flag (zero = empty) and the three published sprite-class
/// special cases (`0x01` default human, `0xFC` Shadow Lord actor)
/// with the catch-all "ordinary derived sprite" path used for every
/// other non-zero value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcTypeByteClass {
    /// `0` — empty slot. The schedule processor skips the slot.
    Empty,
    /// `1` — occupied slot rendered with the default human/person
    /// sprite instead of the ordinary derived-sprite path.
    DefaultHumanSprite,
    /// `0xFC` — resident or summoned Shadow Lord actor.
    ShadowlordActor,
    /// Any other non-zero value — ordinary derived sprite class.
    OrdinarySpriteClass,
}

/// `formats/npc.md §6`: classify a roster type byte into its engine
/// contract category.
pub const fn npc_type_byte_class(byte: u8) -> NpcTypeByteClass {
    match byte {
        NPC_TYPE_EMPTY => NpcTypeByteClass::Empty,
        NPC_TYPE_DEFAULT_HUMAN_SPRITE => NpcTypeByteClass::DefaultHumanSprite,
        NPC_TYPE_SHADOWLORD_ACTOR => NpcTypeByteClass::ShadowlordActor,
        _ => NpcTypeByteClass::OrdinarySpriteClass,
    }
}

/// `formats/npc.md §6`: returns `true` when the schedule processor
/// should treat the slot as occupied. Any non-zero type byte counts
/// as occupied — the special sprite-class sentinels are still
/// occupied slots.
pub const fn npc_type_byte_occupied(byte: u8) -> bool {
    byte != NPC_TYPE_EMPTY
}

/// `catalogs/npc-roster.md §4`: dialog-id `0` is the ordinary
/// no-dialogue value (the engine prints the funny-look stub when the
/// player Talks to such an NPC).
pub const NPC_DIALOG_ID_NONE: u8 = 0;
// `formats/npc.md §7`: "Dialog index `1` is **not** reserved. It
// addresses an ordinary authored blob like any other id, and exactly
// one occupied roster slot in each of the four class files carries
// it: `TOWNE:0` slot 3, `DWELLING:0` slot 1, `CASTLE:0` slot 13, and
// `KEEP:0` slot 1." The withdrawn `NPC_DIALOG_ID_TLK_SENTINEL` used
// to sit here; id `1` now classifies as an ordinary blob id.
/// `catalogs/npc-roster.md §4`: high dialog ids `129..=136` and `255`
/// are observed in the shipped roster but do not resolve to real
/// `.TLK` records; they likely mark guards, generic role actors,
/// hostile actors, or non-speaking schedule participants.
pub const NPC_DIALOG_ID_HIGH_FIRST: u8 = 129;
pub const NPC_DIALOG_ID_HIGH_LAST: u8 = 136;
pub const NPC_DIALOG_ID_HIGH_FALLBACK: u8 = 255;

/// `npc-schedules.md Section 11`: active-object visual tile used when
/// the hidden-NPC bitmask suppresses only presentation. The active
/// object keeps its nonzero NPC type byte so collision, scheduling,
/// and Talk linkage remain live while the rendered tile is transparent.
///
/// The transparent value is the one reserved actor byte, not zero:
/// `catalogs/tile-catalog.md §3.1` says "the sole reserved actor byte
/// is `0x16`, which means 'draw nothing'". Actor byte `0x00` is an
/// ordinary drawable actor id (atlas tile `256`), so storing zero here
/// painted real artwork over every hidden NPC.
pub const NPC_HIDDEN_SPRITE_TILE: u8 = crate::ACTOR_TILE_TRANSPARENT_BYTE;

/// `formats/npc.md §6` / `catalogs/npc-roster.md §4`: the sprite the
/// sprite-link helper forces for roster tag `0x01`, the "default
/// human/person" sentinel, "instead of using the tag as a direct
/// sprite class".
///
/// **The spec does not publish this tile's numeric id.**
/// `npc-schedules.md §11` only says the tile is "a single hard-coded
/// 'person' tile". This engine uses the villager class `0x50`, which
/// `catalogs/npc-roster.md §4` publishes as `a villager` / "Generic
/// adult townsperson; the most common named-NPC sprite class" - the
/// only generic-person sprite class the catalog names. Three shipped
/// roster slots carry tag `0x01` (`CASTLE:0` slots 23, 24 and 25, all
/// static, dialogue-less basement actors); if the spec later publishes
/// the real id, only this constant changes.
pub const NPC_DEFAULT_PERSON_SPRITE_TILE: u8 = 0x50;

/// `npc-schedules.md §11`: shipped hidden-sprite mask. "The mask table
/// is indexed by the **one-based public scene byte itself**, not by a
/// zero-based scene ordinal", and "the shipped DOS data sets
/// hidden-sprite bits in only four scenes":
///
/// | Public scene | Location | Hidden roster slots |
/// |---:|---|---|
/// | 4 | Yew | 15, 17 (two of the three rodent-class actors) |
/// | 5 | Minoc | 1 (Tactus) |
/// | 28 | Windemere | 3..=9 (the keep's rodent group) |
/// | 29 | Stonegate | 5..=8 (the four bat-class actors) |
///
/// The zero-based reading — Moonglow/Minoc/Trinsic/Stonegate/Lycaeum —
/// is retracted by §11: every row of it sat one scene late, and it
/// wrongly hid quest-critical speakers such as Zachariah, Malik and
/// Lady Janell. "No shipped scene hides a talkable named NPC except
/// Minoc's single row." Hidden slots still participate in scheduling,
/// collision and Talk; only the active-object tile is replaced by
/// [`NPC_HIDDEN_SPRITE_TILE`].
pub const fn npc_hidden_sprite_slot(scene_byte: u8, slot: usize) -> bool {
    match scene_byte {
        SCENE_YEW => matches!(slot, 15 | 17),
        SCENE_MINOC => slot == 1,
        SCENE_WINDEMERE => matches!(slot, 3..=9),
        SCENE_STONEGATE => matches!(slot, 5..=8),
        _ => false,
    }
}

/// `catalogs/npc-roster.md §4`: classify a dialog-id byte into the
/// engine's `.TLK`-resolution category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcDialogIdKind {
    /// `0` — no dialogue; Talk produces the funny-look stub.
    NoDialogue,
    /// `1..=128` and any other id below the high-special band — an
    /// ordinary `.TLK` blob lookup key. `formats/npc.md §7`: "Dialog
    /// index `1` is **not** reserved. It addresses an ordinary
    /// authored blob like any other id".
    OrdinaryBlobId,
    /// `129..=136` and `255` — high/special non-resolving ids the
    /// shipped roster uses for guards, hostiles, and similar
    /// non-speaking participants.
    HighSpecial,
}

/// `catalogs/npc-roster.md §4`: classify a dialog-id byte.
pub const fn npc_dialog_id_kind(byte: u8) -> NpcDialogIdKind {
    match byte {
        NPC_DIALOG_ID_NONE => NpcDialogIdKind::NoDialogue,
        NPC_DIALOG_ID_HIGH_FIRST..=NPC_DIALOG_ID_HIGH_LAST => NpcDialogIdKind::HighSpecial,
        NPC_DIALOG_ID_HIGH_FALLBACK => NpcDialogIdKind::HighSpecial,
        _ => NpcDialogIdKind::OrdinaryBlobId,
    }
}

/// `active-objects.md §6`: per-NPC runtime descriptor stride is
/// sixteen bytes (pursuit target, pathfinding state, active
/// waypoint, linked-slot index into the active-object table).
pub const NPC_RUNTIME_DESCRIPTOR_BYTES: usize = 16;

/// `active-objects.md §6` action the world-mutation helper takes
/// when a schedule step crosses (or stays on) the player's floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcLinkAction {
    /// Off → on player's floor: allocate a new slot, fill the
    /// record, set linked-slot.
    Allocate,
    /// On → on player's floor: update the existing slot's
    /// coordinates.
    UpdateCoordinates,
    /// On → off player's floor: free the slot (zero the type byte),
    /// clear linked-slot.
    Free,
    /// Off → off player's floor: no active-object slot action;
    /// logical state only.
    NoAction,
}

/// `active-objects.md §6`: classify the world-mutation helper's
/// action from the NPC's old and new floor compared against the
/// player's current floor.
pub const fn npc_link_action(
    old_on_player_floor: bool,
    new_on_player_floor: bool,
) -> NpcLinkAction {
    match (old_on_player_floor, new_on_player_floor) {
        (false, true) => NpcLinkAction::Allocate,
        (true, true) => NpcLinkAction::UpdateCoordinates,
        (true, false) => NpcLinkAction::Free,
        (false, false) => NpcLinkAction::NoAction,
    }
}

/// `formats/npc.md §7` Talk-entry shop-trigger family for high
/// dialog-index values `0x81..=0x88`. These are not `.TLK` blob ids;
/// they identify which shop the active scene's resident shop tables
/// dispatch to when the player initiates Talk against the NPC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcShopTrigger {
    /// `0x81` — weaponsmith / armourer.
    WeaponsmithOrArmourer,
    /// `0x82` — tavern, meal counter, or sage.
    TavernOrSage,
    /// `0x83` — horse trader.
    HorseTrader,
    /// `0x84` — ship broker / shipwright.
    ShipwrightOrBroker,
    /// `0x85` — herbalist / reagent shop.
    Herbalist,
    /// `0x86` — guild shop.
    Guild,
    /// `0x87` — healer / sanctum.
    HealerOrSanctum,
    /// `0x88` — innkeeper.
    Innkeeper,
}

/// `formats/npc.md §7`: classify a high dialog-index byte
/// (`0x81..=0x88`) as a Talk-entry shop trigger. Returns `None` for
/// ordinary speaking NPC blob ids and any value outside the published
/// shop range.
pub const fn npc_shop_trigger(dialog_index: u8) -> Option<NpcShopTrigger> {
    Some(match dialog_index {
        0x81 => NpcShopTrigger::WeaponsmithOrArmourer,
        0x82 => NpcShopTrigger::TavernOrSage,
        0x83 => NpcShopTrigger::HorseTrader,
        0x84 => NpcShopTrigger::ShipwrightOrBroker,
        0x85 => NpcShopTrigger::Herbalist,
        0x86 => NpcShopTrigger::Guild,
        0x87 => NpcShopTrigger::HealerOrSanctum,
        0x88 => NpcShopTrigger::Innkeeper,
        _ => return None,
    })
}

/// `formats/npc.md §5.3` per-waypoint AI behaviour selector. Values
/// `0..=7` are the shipped behaviour families; values above `7` fall
/// through to the no-action/default case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcAiBehavior {
    /// `0` — stationary at the selected waypoint.
    Stationary,
    /// `1` — random wander, bounded to a small radius around the
    /// waypoint.
    BoundedWander,
    /// `2` — random wander without the radius bound.
    UnboundedWander,
    /// `3` — retreat from the player.
    Retreating,
    /// `4` — approach and attack when close enough.
    ApproachAndAttack,
    /// `5` — randomized unconditional chase with an attack event; not used
    /// by shipped roster data.
    ReservedEngage,
    /// `6` — guard or blocking event path.
    GuardOrBlock,
    /// `7` — randomized chase/engage path.
    RandomChase,
}

impl NpcAiBehavior {
    /// `npc-schedules.md §9`: returns `true` for AI behaviours that
    /// can raise the town-mode attack event when the NPC reaches an
    /// adjacent cell to the player. Only the approach-and-attack and
    /// the two chase/engage paths use this event; guard/block uses
    /// its own non-attack event instead.
    pub const fn raises_attack_event(self) -> bool {
        matches!(
            self,
            Self::ApproachAndAttack | Self::ReservedEngage | Self::RandomChase
        )
    }

    /// `npc-schedules.md §9`: returns `true` for AI behaviours that
    /// raise the non-attack guard event instead of the attack event
    /// when adjacent. Only the guard/block family takes this path.
    pub const fn raises_guard_event(self) -> bool {
        matches!(self, Self::GuardOrBlock)
    }

    /// `npc-schedules.md §9`: returns `true` for the two random-wander
    /// behaviours. Bounded wander rejects steps that would leave a
    /// small radius around the waypoint; unbounded wander skips that
    /// gate.
    pub const fn is_wander(self) -> bool {
        matches!(self, Self::BoundedWander | Self::UnboundedWander)
    }
}

/// `formats/npc.md §5.3`: classify the per-waypoint AI byte. Returns
/// `None` for values above `7`, which the spec maps to the
/// no-action/default case the dispatcher takes.
pub const fn npc_ai_behavior(byte: u8) -> Option<NpcAiBehavior> {
    Some(match byte {
        0 => NpcAiBehavior::Stationary,
        1 => NpcAiBehavior::BoundedWander,
        2 => NpcAiBehavior::UnboundedWander,
        3 => NpcAiBehavior::Retreating,
        4 => NpcAiBehavior::ApproachAndAttack,
        5 => NpcAiBehavior::ReservedEngage,
        6 => NpcAiBehavior::GuardOrBlock,
        7 => NpcAiBehavior::RandomChase,
        _ => return None,
    })
}

/// `npc-schedules.md §8.2` BFS direction codes (1 = west, 2 = south,
/// 3 = east, 4 = north). The seed direction stamped in the start cell's
/// high nibble is `4` (north). The high nibble of any visited cell is
/// the *inbound* direction; the route-reverse pass steps opposite each
/// stored direction from the goal back to the start.
pub const NPC_PATH_DIR_WEST: u8 = 1;
pub const NPC_PATH_DIR_SOUTH: u8 = 2;
pub const NPC_PATH_DIR_EAST: u8 = 3;
pub const NPC_PATH_DIR_NORTH: u8 = 4;

/// `npc-schedules.md §4`: returns `true` when the per-NPC stuck
/// counter has exceeded the published replan threshold and the
/// walker must reset the move queue / counter pair on the next pass.
pub const fn npc_stuck_counter_forces_replan(counter: u16) -> bool {
    counter > NPC_STUCK_REPLAN_THRESHOLD
}

/// `npc-schedules.md §8.2`: coordinate effect of one BFS direction
/// code (`(dx, dy)` to add to the current cell). Returns `(0, 0)` for
/// any code outside `1..=4`.
pub const fn npc_path_direction_offset(code: u8) -> (i8, i8) {
    match code {
        NPC_PATH_DIR_WEST => (-1, 0),
        NPC_PATH_DIR_SOUTH => (0, 1),
        NPC_PATH_DIR_EAST => (1, 0),
        NPC_PATH_DIR_NORTH => (0, -1),
        _ => (0, 0),
    }
}

/// `npc-schedules.md §8.4`: the route-reverse step takes the *opposite*
/// of each stored inbound-direction code. Returns the opposite-direction
/// code, or `None` for any input outside `1..=4`.
pub const fn npc_path_direction_opposite(code: u8) -> Option<u8> {
    Some(match code {
        NPC_PATH_DIR_WEST => NPC_PATH_DIR_EAST,
        NPC_PATH_DIR_EAST => NPC_PATH_DIR_WEST,
        NPC_PATH_DIR_NORTH => NPC_PATH_DIR_SOUTH,
        NPC_PATH_DIR_SOUTH => NPC_PATH_DIR_NORTH,
        _ => return None,
    })
}

/// `npc-schedules.md §8.4` BFS queue capacity (small circular FIFO of
/// `(x, y)` byte pairs).
pub const NPC_PATHFIND_QUEUE_CAPACITY: usize = 32;

/// `npc-schedules.md §8.2`: bit shift used to encode an inbound direction
/// code into the high nibble of a workspace cell (`direction << 4`).
/// The low nibble of the same byte carries the workspace marker (open,
/// goal sentinel) before the cell has been visited.
pub const NPC_PATHFIND_DIRECTION_SHIFT: u8 = 4;

/// `npc-schedules.md §8.5`: encoded high-nibble start-cell seed used by
/// the workspace builder. The seed is `NPC_PATH_DIR_NORTH << NPC_PATHFIND_DIRECTION_SHIFT`
/// (`0x40`); BFS reads this as the start cell's already-visited inbound
/// direction.
pub const NPC_PATHFIND_START_SEED: u8 = NPC_PATH_DIR_NORTH << NPC_PATHFIND_DIRECTION_SHIFT;

/// `npc-schedules.md §8.2`: encode an inbound direction code as the
/// high-nibble visit stamp written into a workspace cell. Returns
/// `direction << NPC_PATHFIND_DIRECTION_SHIFT`. The low nibble is left
/// to the workspace marker so capture-then-write order in BFS can read
/// the original cell's goal sentinel before the visit stamp overwrites
/// it.
pub const fn npc_pathfind_visit_stamp(direction: u8) -> u8 {
    direction << NPC_PATHFIND_DIRECTION_SHIFT
}

/// `npc-schedules.md §8.5` paired floor-link marker tile bytes used by
/// the tile-ID variant of the pathfinder. The same byte pair is the
/// town-mode marker family `NPC_FLOOR_LINK_TILE_A` / `_B`; anchor
/// these aliases to that promoted source so the schedule-walker and
/// the town-mode floor link share one definition.
pub const NPC_FLOOR_LINK_TILE_C8: u8 = crate::NPC_FLOOR_LINK_TILE_A;
pub const NPC_FLOOR_LINK_TILE_C9: u8 = crate::NPC_FLOOR_LINK_TILE_B;

/// `npc-schedules.md §6`: "The floor index grows upward... The ordering
/// test that separates 'above' from 'below' is a **signed eight-bit**
/// comparison: `0xFF` orders below `0x00`, not above it." Returns `true`
/// when floor byte `floor` is above floor byte `other`. The separate
/// equality test ("is this floor the displayed floor?") stays a plain
/// byte match, so no conversion is needed there.
pub const fn npc_floor_is_above(floor: u8, other: u8) -> bool {
    (floor as i8) > (other as i8)
}

/// `npc-schedules.md §6`: classify a real boundary transition into the
/// movement state byte the per-tick walker switches on, given the NPC's
/// current floor, the new waypoint's floor, and the location's current
/// floor. Returns the state byte from the floor-classification table:
///   - equal/equal → 2 (in-plane move)
///   - equal/below → 7 (NPC on this floor; target downstairs)
///   - equal/above → 6 (NPC on this floor; target upstairs)
///   - below/equal → 5 (NPC downstairs; surfaces at a descend link)
///   - above/equal → 4 (NPC upstairs; surfaces at an ascend link)
///   - neither/neither → 8 (parked off-floor / replan needed)
///
/// "Above" and "below" are the signed comparison of
/// [`npc_floor_is_above`], so the basement floor byte `0xFF` orders
/// *below* `0x00` and a basement NPC lands in state 5 rather than state
/// 4. Caller still applies the already-on-waypoint short-circuit and
/// only invokes the classifier when a real transition has been detected.
pub const fn schedule_floor_state(npc_floor: u8, target_floor: u8, map_floor: u8) -> u8 {
    let npc_eq = npc_floor == map_floor;
    let target_eq = target_floor == map_floor;
    if npc_eq && target_eq {
        NPC_STATE_INPLANE_MOVE
    } else if npc_eq && npc_floor_is_above(target_floor, map_floor) {
        NPC_STATE_CLIMB_UP_OFF_FLOOR
    } else if npc_eq && npc_floor_is_above(map_floor, target_floor) {
        NPC_STATE_CLIMB_DOWN_OFF_FLOOR
    } else if target_eq && npc_floor_is_above(npc_floor, map_floor) {
        NPC_STATE_DESCEND_TOWARD_TARGET
    } else if target_eq && npc_floor_is_above(map_floor, npc_floor) {
        NPC_STATE_ASCEND_TOWARD_TARGET
    } else {
        NPC_STATE_PARKED_OFF_FLOOR
    }
}

/// `npc-schedules.md §8.5` ("Which marker a state selects"): *the walker
/// hunts the link that points toward whichever floor is not the displayed
/// one.* The live tile grid only ever holds the displayed floor, so the
/// search always runs there: an "other" floor above the displayed floor
/// selects the ascend link `0xC8`, and one below selects the descend link
/// `0xC9`. States 6/7 pass the active waypoint's floor as `other_floor`;
/// states 4/5 pass the NPC's own off-screen floor.
pub const fn npc_floor_link_marker_toward(displayed_floor: u8, other_floor: u8) -> u8 {
    if npc_floor_is_above(other_floor, displayed_floor) {
        NPC_FLOOR_LINK_TILE_C8
    } else {
        NPC_FLOOR_LINK_TILE_C9
    }
}

/// `npc-schedules.md §8.5` ("Stairway acceptance"), on-floor half. The
/// states 6/7 gate reads the live tile under the NPC's own cell and
/// accepts the direction-matching link marker or a stairway-family tile.
/// The on-floor gate is the deliberately *wider* of the two acceptance
/// tests: besides the visible stairway family `0xC4..=0xC7` it also
/// treats `0xCC..=0xCF` as stairway-like.
pub const fn npc_floor_link_gate_accepts(tile: u8, marker: u8) -> bool {
    tile == marker
        || (crate::TOWN_STAIR_TILE_FIRST <= tile && tile <= crate::TOWN_STAIR_TILE_LAST)
        || matches!(tile, 0xCC..=0xCF)
}

/// `npc-schedules.md §8.5` ("Stairway acceptance"), off-floor half. The
/// states 4/5 arrival test re-reads the live tile at the link cell the
/// search returned and accepts the state's own marker "or any tile in the
/// stairway family `0xC4..0xC7`" — and nothing else. The two tests are
/// intentionally not identical; `0xCC..=0xCF` is gate-only.
pub const fn npc_floor_link_arrival_accepts(tile: u8, marker: u8) -> bool {
    tile == marker || (crate::TOWN_STAIR_TILE_FIRST <= tile && tile <= crate::TOWN_STAIR_TILE_LAST)
}

/// Per `npc-schedules.md §4`: stuck counter threshold for forced replan.
/// When the counter exceeds this value the move queue is reset to inactive
/// and a fresh route is requested on a later tick.
pub const NPC_STUCK_REPLAN_THRESHOLD: u16 = 3;

/// Per `npc-schedules.md §10` movement constraint: dynamic-obstacle scan
/// radius. An occupied active-object cell is reported as blocked only when
/// the occupant is within Manhattan distance less than this value from the
/// NPC's runtime destination. Cells outside this radius are treated as
/// walkable by the pathfinding workspace.
pub const NPC_DYNAMIC_OBSTACLE_MANHATTAN_RADIUS: usize = 4;

/// `npc-schedules.md §10`: returns `true` when an occupied active-
/// object cell falls within the dynamic-obstacle scan radius around
/// the NPC's runtime destination. Occupants strictly inside the
/// radius (Manhattan distance `<` the radius) are reported as
/// blocked; occupants at or beyond the radius are treated as
/// walkable by the pathfinding workspace.
pub const fn npc_dynamic_obstacle_blocks(
    occupant_x: i32,
    occupant_y: i32,
    destination_x: i32,
    destination_y: i32,
) -> bool {
    let dx = if occupant_x > destination_x {
        occupant_x - destination_x
    } else {
        destination_x - occupant_x
    };
    let dy = if occupant_y > destination_y {
        occupant_y - destination_y
    } else {
        destination_y - occupant_y
    };
    (dx + dy) < NPC_DYNAMIC_OBSTACLE_MANHATTAN_RADIUS as i32
}

/// `npc-schedules.md §10` ("Tile passability"): dedicated NPC pathfinding
/// bitmap, intentionally separate from player/vehicle terrain passability.
/// "A set bit marks the tile id as an **obstacle** for NPC pathfinding; a
/// clear bit marks it open." The ranges below are the published obstacle
/// list; everything else is open, including the two unlocked door ids
/// `0xB8`/`0xBA`, the chair family `0x90..=0x93`, the stairway family
/// `0xC4..=0xC7`, and both floor links `0xC8`/`0xC9`. The locked doors
/// `0xB9`/`0xBB` stay obstacles, which is the spec's own confirmation that
/// this is the right way round.
pub const fn npc_path_tile_obstacle(tile: u8) -> bool {
    matches!(
        tile,
        0x01..=0x03
            | 0x0C..=0x0D
            | 0x10..=0x1C
            | 0x27..=0x2B
            | 0x2E..=0x3F
            | 0x41..=0x43
            | 0x46
            | 0x4A..=0x69
            | 0x6C..=0x86
            | 0x88..=0x8F
            | 0x94..=0xA9
            | 0xAB..=0xB7
            | 0xB9
            | 0xBB..=0xC3
            | 0xCA..=0xFF
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeNpc {
    pub slot: usize,
    pub type_byte: u8,
    pub dialog_id: u8,
    pub schedule: [u8; 16],
    pub state: u8,
    pub x: usize,
    pub y: usize,
    pub z: u8,
    pub cached_wp: usize,
    pub move_queue: Vec<u8>,
    pub move_queue_pos: usize,
    pub stuck_counter: u16,
    pub active_object: Option<usize>,
}

impl RuntimeNpc {
    pub fn from_slot(slot: &NpcSlot, hour: u8) -> Self {
        let wp = waypoint_for_hour(&slot.schedule, hour);
        Self {
            slot: slot.slot,
            type_byte: slot.type_byte,
            dialog_id: slot.dialog_id,
            schedule: slot.schedule,
            state: NPC_STATE_IDLE,
            x: slot.schedule[NPC_SCHEDULE_X_OFFSET + wp] as usize,
            y: slot.schedule[NPC_SCHEDULE_Y_OFFSET + wp] as usize,
            z: slot.schedule[NPC_SCHEDULE_Z_OFFSET + wp],
            cached_wp: wp,
            move_queue: Vec::new(),
            move_queue_pos: 0,
            stuck_counter: 0,
            active_object: None,
        }
    }

    /// `town-mode.md §13`: build the resident Shadowlord's stationary
    /// three-waypoint schedule at its fixed town coordinate on floor zero.
    pub fn from_resident_shadowlord(slot: usize, x: usize, y: usize, hour: u8) -> Self {
        let mut schedule = [0u8; NPC_SCHEDULE_RECORD_LEN];
        for wp in 0..NPC_SCHEDULE_WAYPOINT_COUNT {
            schedule[NPC_SCHEDULE_X_OFFSET + wp] = x as u8;
            schedule[NPC_SCHEDULE_Y_OFFSET + wp] = y as u8;
            schedule[NPC_SCHEDULE_Z_OFFSET + wp] = 0;
        }
        let cached_wp = waypoint_for_hour(&schedule, hour);
        Self {
            slot,
            type_byte: SHADOWLORD_ACTOR_TILE,
            dialog_id: NPC_DIALOG_ID_NONE,
            schedule,
            state: NPC_STATE_IDLE,
            x,
            y,
            z: 0,
            cached_wp,
            move_queue: Vec::new(),
            move_queue_pos: 0,
            stuck_counter: 0,
            active_object: None,
        }
    }

    pub fn waypoint_position(&self, wp: usize) -> (usize, usize, u8) {
        (
            self.schedule[NPC_SCHEDULE_X_OFFSET + wp] as usize,
            self.schedule[NPC_SCHEDULE_Y_OFFSET + wp] as usize,
            self.schedule[NPC_SCHEDULE_Z_OFFSET + wp],
        )
    }

    pub fn schedule_time_boundaries(&self) -> [u8; NPC_SCHEDULE_TIME_BOUNDARY_COUNT] {
        [
            self.schedule[NPC_SCHEDULE_TIME_OFFSET],
            self.schedule[NPC_SCHEDULE_TIME_OFFSET + 1],
            self.schedule[NPC_SCHEDULE_TIME_OFFSET + 2],
            self.schedule[NPC_SCHEDULE_TIME_OFFSET + 3],
        ]
    }

    /// `town-mode.md §§13-14`: whether any of the four schedule
    /// boundaries participates in the forced-flight/entry predicates.
    pub fn has_nonzero_schedule_time_boundary(&self) -> bool {
        self.schedule_time_boundaries()
            .into_iter()
            .any(|time| time != 0)
    }

    /// Destructively install the published pursuit schedule while preserving
    /// all waypoint coordinates and the dialogue index.
    pub fn force_town_pursuit(&mut self) {
        let ai = if self.type_byte < TOWN_NPC_FORCED_PURSUIT_NEAR_TYPE_CUTOFF {
            TOWN_NPC_FORCED_PURSUIT_NEAR_AI
        } else {
            TOWN_NPC_FORCED_PURSUIT_RANDOM_AI
        };
        self.schedule[NPC_SCHEDULE_AI_OFFSET..NPC_SCHEDULE_AI_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT]
            .fill(ai);
        self.schedule
            [NPC_SCHEDULE_TIME_OFFSET..NPC_SCHEDULE_TIME_OFFSET + NPC_SCHEDULE_TIME_BOUNDARY_COUNT]
            .fill(0);
        self.reset_move_queue();
    }

    /// Attempt the destructive forced-flight rewrite. Rejection leaves every
    /// byte untouched; acceptance preserves time boundaries and waypoints.
    pub fn force_town_flight(&mut self) -> bool {
        if !(TOWN_NPC_ORDINARY_TYPE_FIRST..=TOWN_NPC_ORDINARY_TYPE_LAST).contains(&self.type_byte)
            || (self.dialog_id != TOWN_NPC_BRUSHOFF_DIALOG_ID
                && !self.has_nonzero_schedule_time_boundary())
        {
            return false;
        }
        self.schedule[NPC_SCHEDULE_AI_OFFSET..NPC_SCHEDULE_AI_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT]
            .fill(TOWN_NPC_FORCED_FLIGHT_AI);
        self.dialog_id = TOWN_NPC_COWERING_DIALOG_ID;
        self.reset_move_queue();
        true
    }

    pub fn reset_move_queue(&mut self) {
        self.move_queue.clear();
        self.move_queue_pos = 0;
        self.stuck_counter = 0;
    }

    pub fn set_idle(&mut self) {
        self.state = NPC_STATE_IDLE;
    }

    pub fn set_settled_at_waypoint(&mut self, waypoint: usize) {
        self.cached_wp = waypoint;
        self.state = NPC_STATE_IDLE;
        self.reset_move_queue();
    }

    pub fn pop_move_queue_direction(&mut self) -> Option<u8> {
        let code = self.peek_move_queue_direction()?;
        self.advance_move_queue_direction();
        Some(code)
    }

    pub fn peek_move_queue_direction(&self) -> Option<u8> {
        self.move_queue.get(self.move_queue_pos).copied()
    }

    pub fn advance_move_queue_direction(&mut self) {
        if self.move_queue_pos >= self.move_queue.len() {
            return;
        }
        self.move_queue_pos += 1;
        if self.move_queue_pos >= self.move_queue.len() {
            self.move_queue.clear();
            self.move_queue_pos = 0;
        }
    }

    pub fn set_move_queue(&mut self, route: Vec<u8>) {
        self.move_queue = route;
        self.move_queue_pos = 0;
        self.state = NPC_STATE_REPLAY_QUEUE;
        self.stuck_counter = 0;
    }

    pub fn note_failed_progress(&mut self) {
        self.stuck_counter = self.stuck_counter.saturating_add(1);
        if npc_stuck_counter_forces_replan(self.stuck_counter) {
            self.move_queue.clear();
            self.move_queue_pos = 0;
            self.stuck_counter = 0;
            if self.state == NPC_STATE_REPLAY_QUEUE {
                self.state = NPC_STATE_IDLE;
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DoorTracker {
    pub previous_tile: u8,
    pub x: usize,
    pub y: usize,
    pub turns_remaining: u8,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocationNpcStartMarkers {
    pub npc_markers: Vec<(usize, usize)>,
}

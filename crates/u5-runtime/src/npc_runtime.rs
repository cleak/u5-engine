//! Runtime NPC + door tracker + location markers.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::Path;

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
    /// `4` — NPC is upstairs of the target; steer toward a
    /// down-stairway on this floor.
    DescendTowardTarget,
    /// `5` — NPC is downstairs of the target; steer toward an
    /// up-stairway on this floor.
    AscendTowardTarget,
    /// `6` — NPC is on this floor and target is above; steer
    /// toward an up-stairway. Floor change happens via state 4/5.
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
}

/// `npc-schedules.md §6` floor-classification mapper. The boundary
/// trigger compares the NPC's current floor, the new waypoint's
/// floor, and the location's current floor and chooses the new
/// state-machine value:
///
/// - both on map -> in-plane move (2)
/// - NPC on map, target above -> climb-up off-floor (6)
/// - NPC on map, target below -> climb-down off-floor (7)
/// - NPC above, target on map -> ascend toward target (5)
/// - NPC below, target on map -> descend toward target (4)
/// - neither on map -> parked off-floor (8)
///
/// "Below" means floor index numerically greater than the map's
/// current floor; "above" means numerically less.
pub const fn npc_schedule_state_for_floor_transition(
    npc_z: u8,
    target_z: u8,
    map_current_floor: u8,
) -> u8 {
    let npc_on_map = npc_z == map_current_floor;
    let target_on_map = target_z == map_current_floor;
    if npc_on_map && target_on_map {
        return NPC_STATE_INPLANE_MOVE;
    }
    if npc_on_map && target_z < map_current_floor {
        return NPC_STATE_CLIMB_UP_OFF_FLOOR;
    }
    if npc_on_map && target_z > map_current_floor {
        return NPC_STATE_CLIMB_DOWN_OFF_FLOOR;
    }
    if target_on_map && npc_z < map_current_floor {
        return NPC_STATE_ASCEND_TOWARD_TARGET;
    }
    if target_on_map && npc_z > map_current_floor {
        return NPC_STATE_DESCEND_TOWARD_TARGET;
    }
    NPC_STATE_PARKED_OFF_FLOOR
}
/// `npc-schedules.md §8.1` pathfinder workspace shape. The flood-fill
/// pathfinder operates on a 32x32 byte scratch grid (1,024 bytes
/// total) keyed by `(row, col)` in row-major order. The workspace
/// is rebuilt from scratch on every pathfinding call — it carries
/// no incremental state between ticks.
pub const NPC_PATHFIND_WORKSPACE_SIDE: usize = 32;
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

/// `formats/npc.md §6` published type-byte sentinels. The type byte
/// at `+0x200..+0x21F` doubles as the slot's occupancy flag and the
/// NPC's sprite/tile class. Three values are special-cased by the
/// engine; every other non-zero byte is an ordinary sprite-class
/// value derived by adding the byte to the NPC sprite page.
pub const NPC_TYPE_EMPTY: u8 = 0x00;
pub const NPC_TYPE_DEFAULT_HUMAN_SPRITE: u8 = 0x01;
pub const NPC_TYPE_RUNTIME_PLAYER_MIRROR: u8 = 0xFC;

/// `formats/npc.md §6`: classify a roster type byte. Combines the
/// occupancy flag (zero = empty) and the three published sprite-class
/// special cases (`0x01` default human, `0xFC` runtime player mirror)
/// with the catch-all "ordinary derived sprite" path used for every
/// other non-zero value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcTypeByteClass {
    /// `0` — empty slot. The schedule processor skips the slot.
    Empty,
    /// `1` — occupied slot rendered with the default human/person
    /// sprite instead of the ordinary derived-sprite path.
    DefaultHumanSprite,
    /// `0xFC` — runtime player-mirror marker written when the
    /// town-mode player is attached to an NPC slot.
    RuntimePlayerMirror,
    /// Any other non-zero value — ordinary derived sprite class.
    OrdinarySpriteClass,
}

/// `formats/npc.md §6`: classify a roster type byte into its engine
/// contract category.
pub const fn npc_type_byte_class(byte: u8) -> NpcTypeByteClass {
    match byte {
        NPC_TYPE_EMPTY => NpcTypeByteClass::Empty,
        NPC_TYPE_DEFAULT_HUMAN_SPRITE => NpcTypeByteClass::DefaultHumanSprite,
        NPC_TYPE_RUNTIME_PLAYER_MIRROR => NpcTypeByteClass::RuntimePlayerMirror,
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
/// `catalogs/npc-roster.md §4`: dialog-id `1` is the universal `.TLK`
/// header sentinel — no shipped roster slot carries this id.
pub const NPC_DIALOG_ID_TLK_SENTINEL: u8 = 1;
/// `catalogs/npc-roster.md §4`: high dialog ids `129..=136` and `255`
/// are observed in the shipped roster but do not resolve to real
/// `.TLK` records; they likely mark guards, generic role actors,
/// hostile actors, or non-speaking schedule participants.
pub const NPC_DIALOG_ID_HIGH_FIRST: u8 = 129;
pub const NPC_DIALOG_ID_HIGH_LAST: u8 = 136;
pub const NPC_DIALOG_ID_HIGH_FALLBACK: u8 = 255;

/// `catalogs/npc-roster.md §4`: classify a dialog-id byte into the
/// engine's `.TLK`-resolution category.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcDialogIdKind {
    /// `0` — no dialogue; Talk produces the funny-look stub.
    NoDialogue,
    /// `1` — universal `.TLK` sentinel; no live roster slot uses it.
    TlkHeaderSentinel,
    /// `2..=128` and any other id below the high-special band — an
    /// ordinary `.TLK` blob lookup key.
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
        NPC_DIALOG_ID_TLK_SENTINEL => NpcDialogIdKind::TlkHeaderSentinel,
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
    /// `3` — follow or shadow the player while maintaining distance.
    FollowAtDistance,
    /// `4` — approach and attack when close enough.
    ApproachAndAttack,
    /// `5` — reserved engage/chase path; present in the dispatcher but
    /// not used by shipped roster data.
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
        3 => NpcAiBehavior::FollowAtDistance,
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

/// `npc-schedules.md §8.5` paired floor-link marker tile bytes used by
/// the tile-ID variant of the pathfinder.
pub const NPC_FLOOR_LINK_TILE_C8: u8 = 0xC8;
pub const NPC_FLOOR_LINK_TILE_C9: u8 = 0xC9;

/// `npc-schedules.md §6`: classify a real boundary transition into the
/// movement state byte the per-tick walker switches on, given the NPC's
/// current floor, the new waypoint's floor, and the location's current
/// floor. Returns the state byte from the floor-classification table:
///   - both equal → 2 (in-plane move)
///   - equal/below → 7 (climb-down off this floor)
///   - equal/above → 6 (climb-up off this floor)
///   - below/equal → 5 (ascend toward target floor)
///   - above/equal → 4 (descend toward target floor)
///   - neither/neither → 8 (parked off-floor / replan needed)
/// "Below" means a floor index numerically greater than the map's
/// current floor; "above" means numerically smaller. Caller still
/// applies the already-on-waypoint short-circuit and only invokes the
/// classifier when a real transition has been detected.
pub const fn schedule_floor_state(
    npc_floor: u8,
    target_floor: u8,
    map_floor: u8,
) -> u8 {
    let npc_eq = npc_floor == map_floor;
    let target_eq = target_floor == map_floor;
    if npc_eq && target_eq {
        NPC_STATE_INPLANE_MOVE
    } else if npc_eq && target_floor > map_floor {
        NPC_STATE_CLIMB_DOWN_OFF_FLOOR
    } else if npc_eq && target_floor < map_floor {
        NPC_STATE_CLIMB_UP_OFF_FLOOR
    } else if target_eq && npc_floor > map_floor {
        NPC_STATE_ASCEND_TOWARD_TARGET
    } else if target_eq && npc_floor < map_floor {
        NPC_STATE_DESCEND_TOWARD_TARGET
    } else {
        NPC_STATE_PARKED_OFF_FLOOR
    }
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeNpc {
    pub slot: usize,
    pub type_byte: u8,
    pub dialog_id: u8,
    pub schedule: [u8; 16],
    pub x: usize,
    pub y: usize,
    pub z: u8,
    pub cached_wp: usize,
    pub active_object: Option<usize>,
    pub player_phantom: bool,
}

impl RuntimeNpc {
    pub fn from_slot(slot: &NpcSlot, hour: u8) -> Self {
        let wp = waypoint_for_hour(&slot.schedule, hour);
        Self {
            slot: slot.slot,
            type_byte: slot.type_byte,
            dialog_id: slot.dialog_id,
            schedule: slot.schedule,
            x: slot.schedule[3 + wp] as usize,
            y: slot.schedule[6 + wp] as usize,
            z: slot.schedule[9 + wp],
            cached_wp: wp,
            active_object: None,
            player_phantom: false,
        }
    }

    pub fn from_player_phantom(x: usize, y: usize, z: u8, hour: u8) -> Self {
        let mut schedule = [0u8; 16];
        for wp in 0..3 {
            schedule[3 + wp] = x as u8;
            schedule[6 + wp] = y as u8;
            schedule[9 + wp] = z;
        }
        let cached_wp = waypoint_for_hour(&schedule, hour);
        Self {
            slot: PLAYER_NPC_SLOT,
            type_byte: PLAYER_NPC_SENTINEL_TYPE,
            dialog_id: PLAYER_NPC_DIALOG_ID,
            schedule,
            x,
            y,
            z,
            cached_wp,
            active_object: None,
            player_phantom: true,
        }
    }

    pub fn is_player_phantom(&self) -> bool {
        self.player_phantom
    }

    pub fn sync_player_phantom_floor(&mut self, floor: u8, hour: u8) {
        self.z = floor;
        for wp in 0..3 {
            self.schedule[9 + wp] = floor;
        }
        self.cached_wp = waypoint_for_hour(&self.schedule, hour);
    }

    pub fn waypoint_position(&self, wp: usize) -> (usize, usize, u8) {
        (
            self.schedule[3 + wp] as usize,
            self.schedule[6 + wp] as usize,
            self.schedule[9 + wp],
        )
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
pub struct LocationMarkers {
    pub npc_markers: Vec<(usize, usize)>,
    pub spawn_markers: Vec<(usize, usize)>,
}

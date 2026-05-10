//! PlayOptions + small action enums + moonstone gate helpers + initial overlay cache.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlayOptions {
    pub target: PlayTarget,
    pub floor: i8,
    pub start: Option<(usize, usize)>,
    pub clock: GameClock,
    pub food: u16,
    pub gold: u16,
    pub keys: u8,
    pub gems: u8,
    pub climbing_gear: u8,
    pub party: Vec<PartyMember>,
    pub spell_charges: [u8; SPELL_COUNT],
    pub reagents: [u8; REAGENT_COUNT],
    pub moonstone_slots: [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT],
    pub shrine_ordained_mask: u8,
    pub shrine_codex_mask: u8,
    pub shrine_standing: [u8; VIRTUE_COUNT],
    pub avatar_stats: AvatarStats,
    pub torches: u8,
    pub torch_counter: u8,
    pub light_spell_counter: u8,
    pub wind: WindState,
    pub wind_save_byte: u8,
    pub timing_status: TimingStatusTag,
    pub time_stop_counter: u8,
    pub active_effect_tag: Option<u8>,
    pub active_effect_counter: u8,
    pub transport: TransportState,
    pub pending_vehicle: Option<PendingVehicleAcquisition>,
    pub initial_britannia_overlay: Option<Vec<ActiveObject>>,
    pub debug_enter: Option<PlayTarget>,
    pub saved_active_objects: Option<Vec<ActiveObject>>,
    pub save_template_source: SaveTemplateSource,
}

impl Default for PlayOptions {
    fn default() -> Self {
        Self {
            target: PlayTarget::Town(
                Scene::new(0x11).expect("default Lord British castle scene is valid"),
            ),
            floor: 0,
            start: None,
            clock: GameClock::default(),
            food: DEFAULT_FOOD_STOCK,
            gold: DEFAULT_GOLD_STOCK,
            keys: DEFAULT_KEY_STOCK,
            gems: DEFAULT_GEM_STOCK,
            climbing_gear: DEFAULT_CLIMBING_GEAR,
            party: default_party(),
            spell_charges: [0; SPELL_COUNT],
            reagents: DEFAULT_REAGENTS,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            shrine_standing: [0; VIRTUE_COUNT],
            avatar_stats: AvatarStats::default(),
            torches: DEFAULT_TORCH_STOCK,
            torch_counter: 0,
            light_spell_counter: 0,
            wind: WindState::default(),
            wind_save_byte: 0,
            timing_status: TimingStatusTag::default(),
            time_stop_counter: 0,
            active_effect_tag: None,
            active_effect_counter: 0,
            transport: TransportState::Foot,
            pending_vehicle: None,
            initial_britannia_overlay: None,
            debug_enter: None,
            saved_active_objects: None,
            save_template_source: SaveTemplateSource::PreferSavedGame,
        }
    }
}

pub fn initial_world_overlay_cache(options: &PlayOptions) -> WorldOverlayCache {
    let mut overlays = WorldOverlayCache::default();
    if let Some(objects) = options.initial_britannia_overlay.clone() {
        overlays.set(WorldPlane::Britannia, objects);
    }
    overlays
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GateTravelDestination {
    Ready {
        target: PlayTarget,
        floor: i8,
        start: (usize, usize),
    },
    Empty,
    Invalid(String),
}

pub fn gate_travel_destination(slot: MoonstoneGateSlot) -> GateTravelDestination {
    if !slot.is_valid() {
        return GateTravelDestination::Empty;
    }

    let x = slot.x as usize;
    let y = slot.y as usize;
    match slot.scene {
        0 => {
            let plane = WorldPlane::from_save_z(slot.z);
            GateTravelDestination::Ready {
                target: PlayTarget::World(plane),
                floor: plane.save_floor(),
                start: (x, y),
            }
        }
        1..=32 => {
            if x >= 32 || y >= 32 {
                return GateTravelDestination::Invalid(format!(
                    "town position must be inside 0..31, got ({x}, {y})"
                ));
            }
            match Scene::new(slot.scene) {
                Ok(scene) => GateTravelDestination::Ready {
                    target: PlayTarget::Town(scene),
                    floor: slot.z as i8,
                    start: (x, y),
                },
                Err(err) => GateTravelDestination::Invalid(err.to_string()),
            }
        }
        33..=40 => {
            if slot.z > 7 {
                return GateTravelDestination::Invalid(format!(
                    "dungeon level must be inside 0..7, got {}",
                    slot.z
                ));
            }
            if x >= DUNGEON_SIDE || y >= DUNGEON_SIDE {
                return GateTravelDestination::Invalid(format!(
                    "dungeon position must be inside 0..7, got ({x}, {y})"
                ));
            }
            match DungeonScene::new(slot.scene) {
                Ok(scene) => GateTravelDestination::Ready {
                    target: PlayTarget::Dungeon(scene),
                    floor: slot.z as i8,
                    start: (x, y),
                },
                Err(err) => GateTravelDestination::Invalid(err.to_string()),
            }
        }
        scene => GateTravelDestination::Invalid(format!("unsupported scene {scene}")),
    }
}

pub fn moonstone_slot_matches_world(
    slot: MoonstoneGateSlot,
    plane: WorldPlane,
    x: usize,
    y: usize,
) -> bool {
    slot.scene == 0
        && WorldPlane::from_save_z(slot.z) == plane
        && slot.x as usize == x
        && slot.y as usize == y
}

pub fn moonstone_slot_matches_town(
    slot: MoonstoneGateSlot,
    scene: Scene,
    floor: i8,
    x: usize,
    y: usize,
) -> bool {
    slot.scene == scene.byte
        && slot.z as i8 == floor
        && slot.x as usize == x
        && slot.y as usize == y
}

pub fn moonstone_bury_tile_allowed(tile: u8) -> bool {
    matches!(tile, 4..=10 | 44 | 45)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoveOutcome {
    Moved,
    Blocked,
    Boarded,
    ExitedVehicle,
    SailToggled,
    SailStalled,
    Fired,
    Pushed,
    Rested,
    Talked,
    Ignited,
    Cast,
    DoorOpened,
    ContainerOpened,
    Got,
    Used,
    LockTried,
    Observed,
    Searched,
    IdleTick,
    PromptDeclined,
    Passed,
    Saved,
    Transition(AreaTransition),
}

impl MoveOutcome {
    pub fn is_transition(self) -> bool {
        matches!(self, Self::Transition(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseItemRequest {
    Torch,
    Gem,
    Key,
    Moonstone(usize),
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaTransition {
    ChangedFloor {
        scene: Scene,
        floor: i8,
    },
    ChangedDungeonLevel {
        scene: DungeonScene,
        level: u8,
    },
    ChangedWorldPlane {
        from: WorldPlane,
        to: WorldPlane,
    },
    MoongateTeleported {
        from: WorldPlane,
        to: WorldPlane,
    },
    GateTraveled {
        target: PlayTarget,
    },
    EnteredLocation(Scene),
    EnteredDungeon(DungeonScene),
    ExitedLocation(Scene),
    ExitedDungeon(DungeonScene),
    ExitedDungeonToWorldPlane {
        scene: DungeonScene,
        plane: WorldPlane,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClimbIntent {
    Up,
    Down,
}

#[derive(Debug)]
pub struct NpcSlot {
    pub slot: usize,
    pub type_byte: u8,
    pub dialog_id: u8,
    pub schedule: [u8; 16],
    pub name: Option<String>,
}

#[derive(Debug)]
pub struct MapStats {
    pub scene: Scene,
    pub floor: usize,
    pub npc_markers: Vec<(usize, usize)>,
    pub spawn_markers: Vec<(usize, usize)>,
    pub door_count: usize,
    pub stair_count: usize,
    pub render_hash: u64,
    pub class_histogram: HashMap<&'static str, usize>,
}

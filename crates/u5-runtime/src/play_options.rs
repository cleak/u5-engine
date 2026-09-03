//! PlayOptions + small action enums + moonstone gate helpers + initial overlay cache.

use std::collections::HashMap;

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
    pub special_items: [u8; SPECIAL_ITEM_COUNT],
    pub party: Vec<PartyMember>,
    pub party_names: Vec<[u8; SAVE_CHARACTER_NAME_LEN]>,
    pub party_experience: Vec<u16>,
    pub party_stay_counters: Vec<u8>,
    pub party_strengths: Vec<u8>,
    pub party_intelligence: Vec<u8>,
    pub party_equipment: Vec<[u8; EQUIPMENT_SLOT_COUNT]>,
    pub party_roster: Vec<PartyRosterRecord>,
    pub equipment_stock: [u8; EQUIPMENT_COUNT],
    pub spell_charges: [u8; SPELL_COUNT],
    pub scroll_stock: [u8; SCROLL_COUNT],
    pub potion_stock: [u8; POTION_COUNT],
    pub reagents: [u8; REAGENT_COUNT],
    pub rare_reagent_harvest_days: [u8; RARE_REAGENT_HARVEST_POINT_COUNT],
    pub fixed_hidden_treasure_found: [u8; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
    pub fixed_hidden_treasure_daily_day: u8,
    pub dungeon_room_clear_bitmap: [u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
    pub saved_dungeon_working_buffer: Option<Vec<u8>>,
    pub moonstone_slots: [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT],
    pub shadowlord_hideouts: [u8; SHADOWLORD_COUNT],
    pub removed_town_npc_flags: HashMap<u8, u32>,
    pub talk_branch_flags: HashMap<u8, u32>,
    pub shrine_ordained_mask: u8,
    pub shrine_codex_mask: u8,
    pub word_of_power_seal_flags: [u8; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
    pub shrine_ruin_flags: [u8; SAVE_SHRINE_RUIN_FLAG_COUNT],
    pub moral_standing: u8,
    pub toll_progress: u8,
    /// `formats/saved-gam.md §5`: saved pre-cascade hour snapshot
    /// (`0x02DA`) restored from the save image.
    pub cleanup_previous_hour: u8,
    /// `formats/saved-gam.md §5` / `time.md §11` (spec `0170809`): the
    /// twelve-hour value at `0x02DE`, which the ambient-audio tick reads
    /// "as a count of remaining loud repeats". Restored verbatim from the
    /// save image.
    pub twelve_hour_audio_repeats: u8,
    /// `formats/saved-gam.md §5.1` (spec `0170809`): the cached Trammel
    /// and Felucca moon-phase digits at `0x02DF`/`0x02E0`, restored
    /// verbatim. They are "the sole input to natural-gate destination
    /// selection" (`RETRACTIONS.md` R339).
    pub cached_moon_glyph_bytes: [u8; 2],
    /// `formats/saved-gam.md §10` / `time.md §6` (spec `0170809`): the
    /// cached ambient light level at `0x02FF`. A stored value of 51 or
    /// higher "makes the recompute skip entirely", so the byte has to be
    /// restored rather than reseeded.
    pub ambient_light: u8,
    /// `overworld.md §9.1` (spec HEAD c00bf63): the shared
    /// natural-moongate gate-presence counter, restored from
    /// `SAVED.GAM` offset `0x02E1`. Persistent world state - it
    /// survives turns, mode changes, scene changes and save/load.
    pub natural_moongate_counter: u8,
    /// `animation.md §9`/`§12`: the driver-side animation layer's state,
    /// which "lives in the asset buffer for the whole program run" and is
    /// **not** reset by a scene change or a save load. It is not saved
    /// state, so nothing in `SAVED.GAM` sets it; it rides here purely so
    /// that a `PlayState` rebuilt for a new area inherits the phases the
    /// previous one had reached instead of snapping every fountain, water
    /// tile, banner and clock back to phase zero. [`Default`] is
    /// [`AnimationAssetBuffer::AT_BOOT`], which is the correct value for a
    /// program that is only now starting (`§6.1`).
    pub animation_asset_buffer: AnimationAssetBuffer,
    pub avatar_stats: AvatarStats,
    pub torches: u8,
    pub torch_counter: u8,
    pub light_spell_counter: u8,
    pub wind: WindState,
    pub wind_save_byte: u8,
    pub time_stop_counter: u8,
    pub active_effect_tag: Option<u8>,
    pub active_effect_counter: u8,
    pub fortunes_of_war: u8,
    /// `rest-and-camp.md §5` persisted camp cooldown counter, restored
    /// from `SAVED.GAM` offset `0x02E6`.
    pub camp_cooldown: u8,
    /// `rest-and-camp.md §5` / `formats/saved-gam.md §10` persisted
    /// month cookie at `0x02E7`. The apparition draw writes it; no
    /// shipped consumer reads it.
    pub camp_month_cookie: u8,
    pub active_player: Option<usize>,
    pub combat_round_counter: u8,
    pub combat_interference_sources: [u8; COMBAT_ACTOR_SLOTS],
    pub transport: TransportState,
    pub facing: Option<Direction>,
    pub door_tracker: Option<DoorTracker>,
    pub pending_vehicle: Option<PendingVehicleAcquisition>,
    pub pending_vehicle_save: PendingVehicleSaveState,
    pub inn_registry: Vec<InnGuestRecord>,
    pub initial_britannia_overlay: Option<Vec<ActiveObject>>,
    pub debug_enter: Option<PlayTarget>,
    pub saved_active_objects: Option<Vec<ActiveObject>>,
    pub town_npc_mutations: Vec<TownNpcMutation>,
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
            special_items: [0; SPECIAL_ITEM_COUNT],
            party: default_party(),
            party_names: default_party_names(1),
            party_experience: default_party_experience(1),
            party_stay_counters: default_party_stay_counters(1),
            party_strengths: default_party_strengths(1),
            party_intelligence: default_party_intelligence(1),
            party_equipment: default_party_equipment(1),
            party_roster: default_party_roster(1),
            equipment_stock: [0; EQUIPMENT_COUNT],
            spell_charges: [0; SPELL_COUNT],
            scroll_stock: [0; SCROLL_COUNT],
            potion_stock: [0; POTION_COUNT],
            reagents: DEFAULT_REAGENTS,
            rare_reagent_harvest_days: [RARE_REAGENT_HARVEST_UNSEEN_DAY;
                RARE_REAGENT_HARVEST_POINT_COUNT],
            fixed_hidden_treasure_found: [0; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
            fixed_hidden_treasure_daily_day: FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY,
            dungeon_room_clear_bitmap: [0; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
            saved_dungeon_working_buffer: None,
            moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
            shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
            removed_town_npc_flags: HashMap::new(),
            talk_branch_flags: HashMap::new(),
            shrine_ordained_mask: 0,
            shrine_codex_mask: 0,
            word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
            shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
            moral_standing: 0,
            toll_progress: 0,
            cleanup_previous_hour: 0,
            twelve_hour_audio_repeats: 0,
            // `formats/saved-gam.md §5.1`: "Factory seed: both bytes are
            // zero, and the first scene entry replaces them with the pair
            // for day five of the shipped start date." The default is that
            // seed, not a synthesised phase.
            cached_moon_glyph_bytes: [0, 0],
            ambient_light: 0,
            natural_moongate_counter: 0,
            animation_asset_buffer: AnimationAssetBuffer::AT_BOOT,
            avatar_stats: AvatarStats::default(),
            torches: DEFAULT_TORCH_STOCK,
            torch_counter: 0,
            light_spell_counter: 0,
            wind: WindState::default(),
            wind_save_byte: 0,
            time_stop_counter: 0,
            active_effect_tag: None,
            active_effect_counter: 0,
            fortunes_of_war: 0,
            camp_cooldown: 0,
            camp_month_cookie: 0,
            active_player: None,
            combat_round_counter: 0,
            combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
            transport: TransportState::Foot,
            facing: None,
            door_tracker: None,
            pending_vehicle: None,
            pending_vehicle_save: PendingVehicleSaveState::default(),
            inn_registry: Vec::new(),
            initial_britannia_overlay: None,
            debug_enter: None,
            saved_active_objects: None,
            town_npc_mutations: Vec::new(),
            save_template_source: SaveTemplateSource::PreferSavedGame,
        }
    }
}

impl PlayOptions {
    /// Derive timing from the one authoritative shared effect slot.
    pub const fn active_effect_timing_status(&self) -> TimingStatusTag {
        TimingStatusTag::from_save_byte(match self.active_effect_tag {
            Some(tag) if self.active_effect_counter != 0 => tag,
            _ => 0,
        })
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
    EndgameEntered,
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
    WoodenBox,
    HmsCapePlans,
    CrownOfLordBritish,
    AmuletOfLordBritish,
    Sceptre,
    BlackBadge,
    Spyglass,
    Scroll {
        index: usize,
        direction: Option<Direction>,
        target: Option<usize>,
    },
    Potion {
        index: usize,
        target: Option<usize>,
    },
    MagicCarpet,
    SkullKey,
    Sextant,
    PocketWatch,
    ShadowlordShard(usize),
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
    /// `visibility.md §12.6` indoor beacon sources for this floor. These
    /// were reported as "spawn markers" until `formats/location-dat.md §6`
    /// withdrew that reading of `0x2A`.
    pub beacon_sources: [Option<(u8, u8)>; 2],
    pub door_count: usize,
    pub stair_count: usize,
    pub render_hash: u64,
    pub class_histogram: HashMap<&'static str, usize>,
}

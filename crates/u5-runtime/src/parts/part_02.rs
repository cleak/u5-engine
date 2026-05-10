



#[derive(Clone, Debug)]
pub struct PlayState {
    pub area: Area,
    pub player: Player,
    pub active_objects: Vec<ActiveObject>,
    pub npcs: Vec<RuntimeNpc>,
    pub door_tracker: Option<DoorTracker>,
    pub opened_town_doors: Vec<(u8, i8, usize, usize)>,
    pub revealed_town_secret_doors: Vec<(u8, i8, usize, usize)>,
    pub passability: Option<TilePassability>,
    pub moongates: Vec<MoongateEntry>,
    pub grid: Vec<u8>,
    pub clock: GameClock,
    pub animation: AnimationClock,
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
    pub ambient_light: u8,
    pub visibility_dirty: bool,
    pub wind: WindState,
    pub wind_save_byte: u8,
    pub timing_status: TimingStatusTag,
    pub time_stop_counter: u8,
    pub active_effect_tag: Option<u8>,
    pub active_effect_counter: u8,
    pub sail_cadence: u8,
    pub sail_stall_pending: bool,
    pub turn: u64,
    pub message: String,
    pub debug_enter: Option<PlayTarget>,
    pub return_world: Option<WorldReturn>,
    pub world_overlays: WorldOverlayCache,
    pub save_template_source: SaveTemplateSource,
    pub typeahead_buffer_enabled: bool,
    pub pending_moongate: Option<MoongateEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WorldOverlayCache {
    pub britannia: Option<Vec<ActiveObject>>,
    pub underworld: Option<Vec<ActiveObject>>,
}

impl WorldOverlayCache {
    pub fn get(&self, plane: WorldPlane) -> Option<Vec<ActiveObject>> {
        match plane {
            WorldPlane::Britannia => self.britannia.clone(),
            WorldPlane::Underworld => self.underworld.clone(),
        }
    }

    pub fn set(&mut self, plane: WorldPlane, objects: Vec<ActiveObject>) {
        match plane {
            WorldPlane::Britannia => self.britannia = Some(objects),
            WorldPlane::Underworld => self.underworld = Some(objects),
        }
    }
}

#[derive(Clone, Debug)]
pub struct WorldReturn {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub transport: TransportState,
    pub timing_status: TimingStatusTag,
    pub sail_cadence: u8,
    pub sail_stall_pending: bool,
    pub grid: Vec<u8>,
    pub active_objects: Vec<ActiveObject>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldLocationEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub target: PlayTarget,
    pub town_entry_y: Option<usize>,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShrineEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub virtue: ShrineVirtue,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldPlaneTransitionEntry {
    pub from_plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub to_plane: WorldPlane,
    pub to_x: usize,
    pub to_y: usize,
    pub expected_tile: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGetTileEntry {
    pub plane: WorldPlane,
    pub x: usize,
    pub y: usize,
    pub replacement_tile: u8,
    pub expected_tile: Option<u8>,
    pub grant: Option<ObjectPickupGrant>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectPickupKind {
    Food,
    Gold,
    Keys,
    Gems,
    Torches,
}

impl ObjectPickupKind {
    pub fn from_key(key: &str) -> Option<Self> {
        match key.to_ascii_lowercase().as_str() {
            "food" | "ration" | "rations" => Some(Self::Food),
            "gold" | "coin" | "coins" => Some(Self::Gold),
            "key" | "keys" => Some(Self::Keys),
            "gem" | "gems" => Some(Self::Gems),
            "torch" | "torches" => Some(Self::Torches),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Food => "food",
            Self::Gold => "gold",
            Self::Keys => "keys",
            Self::Gems => "gems",
            Self::Torches => "torches",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectPickupGrant {
    pub kind: ObjectPickupKind,
    pub amount: u8,
}


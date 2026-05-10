impl PartyMember {
    pub fn living(self) -> bool {
        self.hp > 0 && !matches!(self.status, b'D' | b'A')
    }

    pub fn conscious(self) -> bool {
        self.living() && self.status != b'S'
    }

    pub fn apply_damage(&mut self, damage: u8) -> u16 {
        let damage = damage as u16;
        let applied = self.hp.min(damage);
        self.hp -= applied;
        if self.hp == 0 {
            self.status = b'D';
        }
        applied
    }

    pub fn heal_by(&mut self, hp: u16) -> u16 {
        let applied = self.max_hp.saturating_sub(self.hp).min(hp);
        self.hp += applied;
        applied
    }

    pub fn recover_mana_by(&mut self, mana: u8) -> u8 {
        let applied = REST_MANA_CAP.saturating_sub(self.mana).min(mana);
        self.mana += applied;
        applied
    }

    pub fn heal_to_max(&mut self) -> (u16, u16) {
        let before = self.hp;
        self.hp = self.max_hp;
        (before, self.hp)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MoonstoneGateSlot {
    pub scene: u8,
    pub x: u8,
    pub y: u8,
    pub z: u8,
}

impl MoonstoneGateSlot {
    pub const fn invalid() -> Self {
        Self {
            scene: MOONSTONE_INVALID_SCENE,
            x: 0,
            y: 0,
            z: 0,
        }
    }

    pub fn is_valid(self) -> bool {
        self.scene != MOONSTONE_INVALID_SCENE
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AvatarStats {
    pub strength: u8,
    pub dexterity: u8,
    pub intelligence: u8,
}

impl AvatarStats {
    pub fn capped_seed() -> Self {
        Self {
            strength: AVATAR_STAT_MAX,
            dexterity: AVATAR_STAT_MAX,
            intelligence: AVATAR_STAT_MAX,
        }
    }

    pub fn increase_strength(&mut self) -> bool {
        increase_capped_stat(&mut self.strength)
    }

    pub fn increase_dexterity(&mut self) -> bool {
        increase_capped_stat(&mut self.dexterity)
    }

    pub fn increase_intelligence(&mut self) -> bool {
        increase_capped_stat(&mut self.intelligence)
    }
}

pub fn increase_capped_stat(stat: &mut u8) -> bool {
    if *stat >= AVATAR_STAT_MAX {
        false
    } else {
        *stat += 1;
        true
    }
}

impl Default for AvatarStats {
    fn default() -> Self {
        Self::capped_seed()
    }
}

pub fn default_party() -> Vec<PartyMember> {
    vec![PartyMember {
        slot: 0,
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 8,
        hp: DEFAULT_PARTY_HP,
        max_hp: DEFAULT_PARTY_MAX_HP,
        level: 8,
    }]
}

pub fn party_status_name(status: u8) -> &'static str {
    match status {
        b'G' => "good",
        b'P' => "poisoned",
        b'S' => "asleep",
        b'D' => "dead",
        b'A' => "ashes",
        _ => "status-tagged",
    }
}

pub fn party_member_unavailable_message(party_len: usize) -> String {
    format!(
        "Party has {} member{}.",
        party_len,
        if party_len == 1 { "" } else { "s" }
    )
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TransportState {
    #[default]
    Foot,
    Horse {
        type_byte: u8,
        tile: u8,
    },
    Ship {
        type_byte: u8,
        tile: u8,
        sails_hoisted: bool,
        hull: u8,
        skiffs: u8,
    },
    Skiff {
        type_byte: u8,
        tile: u8,
    },
    Carpet {
        type_byte: u8,
        tile: u8,
    },
    Balloon {
        type_byte: u8,
        tile: u8,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PendingVehicleAcquisition {
    Frigate { x: usize, y: usize, skiffs: u8 },
    Skiff { x: usize, y: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoardVehicleCandidate {
    pub slot: usize,
    pub transport: TransportState,
    pub blocked_by_occupant: bool,
}

impl PendingVehicleAcquisition {
    pub fn active_object(self, z: i8) -> ActiveObject {
        match self {
            Self::Frigate { x, y, skiffs } => ActiveObject {
                type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
                tile: FIRST_PLAYABLE_FRIGATE_TILE,
                x,
                y,
                z,
                phase: STEADY_PHASE,
                aux1: FIRST_PLAYABLE_FULL_SHIP_HULL,
                aux3: skiffs,
            },
            Self::Skiff { x, y } => ActiveObject {
                type_byte: FIRST_PLAYABLE_SKIFF_TILE,
                tile: FIRST_PLAYABLE_SKIFF_TILE,
                x,
                y,
                z,
                phase: STEADY_PHASE,
                aux1: 0,
                aux3: 0,
            },
        }
    }
}

impl TransportState {
    pub fn is_foot(self) -> bool {
        matches!(self, Self::Foot)
    }

    pub fn is_horse(self) -> bool {
        matches!(self, Self::Horse { .. })
    }

    pub fn is_ship_under_sail(self) -> bool {
        matches!(
            self,
            Self::Ship {
                sails_hoisted: true,
                ..
            }
        )
    }

    pub fn is_balloon(self) -> bool {
        matches!(self, Self::Balloon { .. })
    }

    pub fn avatar_tile(self) -> u8 {
        match self {
            Self::Foot => PLAYER_TILE,
            Self::Horse { tile, .. }
            | Self::Ship { tile, .. }
            | Self::Skiff { tile, .. }
            | Self::Carpet { tile, .. }
            | Self::Balloon { tile, .. } => tile,
        }
    }

    pub fn kind_name(self) -> &'static str {
        match self {
            Self::Foot => "foot",
            Self::Horse { .. } => "horse",
            Self::Ship { .. } => "ship",
            Self::Skiff { .. } => "skiff",
            Self::Carpet { .. } => "carpet",
            Self::Balloon { .. } => "balloon",
        }
    }

    pub fn status_label(self) -> String {
        match self {
            Self::Foot => "foot".to_string(),
            Self::Horse { tile, .. } => format!("horse tile {tile}"),
            Self::Ship {
                tile,
                sails_hoisted,
                hull,
                skiffs,
                ..
            } => format!(
                "ship tile {tile} sails={} hull={hull} skiffs={skiffs}",
                if sails_hoisted { "hoisted" } else { "furled" }
            ),
            Self::Skiff { tile, .. } => format!("skiff tile {tile}"),
            Self::Carpet { tile, .. } => format!("magic carpet tile {tile}"),
            Self::Balloon { tile, .. } => format!("balloon tile {tile}"),
        }
    }

    pub fn can_board(self, target: Self) -> bool {
        match target {
            Self::Ship { .. } => {
                matches!(self, Self::Foot | Self::Ship { .. } | Self::Skiff { .. })
            }
            Self::Horse { .. } | Self::Skiff { .. } | Self::Carpet { .. } => self.is_foot(),
            Self::Balloon { .. } => false,
            Self::Foot => false,
        }
    }

    pub fn save_marker(self) -> u8 {
        match self {
            Self::Foot => FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER,
            Self::Horse { tile, .. }
            | Self::Ship { tile, .. }
            | Self::Skiff { tile, .. }
            | Self::Carpet { tile, .. } => tile,
            Self::Balloon { .. } => FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER,
        }
    }

    pub fn parked_object(self, x: usize, y: usize, z: i8) -> Option<ActiveObject> {
        let (type_byte, tile, aux1, aux3) = match self {
            Self::Foot => return None,
            Self::Horse {
                type_byte, tile, ..
            }
            | Self::Skiff {
                type_byte, tile, ..
            }
            | Self::Carpet {
                type_byte, tile, ..
            }
            | Self::Balloon {
                type_byte, tile, ..
            } => (type_byte, tile, 0, 0),
            Self::Ship {
                type_byte,
                tile,
                hull,
                skiffs,
                ..
            } => (type_byte, tile, hull, skiffs),
        };
        Some(ActiveObject {
            type_byte,
            tile,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1,
            aux3,
        })
    }

    pub fn append_ship_auxiliary_warnings(self, message: &mut String) {
        if let Self::Ship { hull: 0, .. } = self {
            message.push(' ');
            message.push_str(SHIP_BADLY_DAMAGED_WARNING);
        }
        if let Self::Ship { skiffs: 0, .. } = self {
            message.push(' ');
            message.push_str(SHIP_NO_SKIFFS_WARNING);
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TimingStatusTag {
    #[default]
    Normal,
    HalfTime,
    NoMinuteLight,
}

impl TimingStatusTag {
    pub fn from_save_byte(byte: u8) -> Self {
        match byte {
            b'Q' => Self::HalfTime,
            b'T' => Self::NoMinuteLight,
            _ => Self::Normal,
        }
    }

    pub fn for_transport(transport: TransportState) -> Self {
        if matches!(transport, TransportState::Skiff { .. }) {
            Self::HalfTime
        } else {
            Self::Normal
        }
    }

    pub fn effective_minutes(self, base: u8) -> u8 {
        match self {
            Self::Normal => base,
            Self::HalfTime if base == 0 => 0,
            Self::HalfTime => (base / 2).max(1),
            Self::NoMinuteLight => 0,
        }
    }

    pub fn save_byte(self) -> u8 {
        match self {
            Self::Normal => 0,
            Self::HalfTime => b'Q',
            Self::NoMinuteLight => b'T',
        }
    }

    pub fn status_label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::HalfTime => "half-time",
            Self::NoMinuteLight => "no-minute-light",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveTemplateSource {
    PreferSavedGame,
    SavedGame,
    InitGame,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DungeonFieldEffect {
    Sleep,
    PoisonGas,
    Fire,
    Electric,
    Energy,
}

impl DungeonFieldEffect {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sleep => "sleep field",
            Self::PoisonGas => "poison gas field",
            Self::Fire => "wall of fire",
            Self::Electric => "electric field",
            Self::Energy => "energy field",
        }
    }

    pub fn status(self) -> Option<u8> {
        match self {
            Self::Sleep => Some(b'S'),
            Self::PoisonGas => Some(b'P'),
            Self::Fire | Self::Electric | Self::Energy => None,
        }
    }

    pub fn is_damage_field(self) -> bool {
        matches!(self, Self::Fire | Self::Electric)
    }

    pub fn damage_seed_bias(self) -> u8 {
        match self {
            Self::Fire => 19,
            Self::Electric => 29,
            Self::Sleep | Self::PoisonGas | Self::Energy => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimationClock {
    pub frame: u8,
    pub moongate_frame: u8,
}

impl AnimationClock {
    pub fn tick_static_tiles(&mut self) {
        self.frame = self.frame.wrapping_add(1) & 3;
    }

    pub fn tick_moongate(&mut self) {
        self.moongate_frame = self.moongate_frame.wrapping_add(1) & (MOONGATE_ANIMATION_FRAMES - 1);
    }

    pub fn resolve_static_tile(self, tile: u8) -> u8 {
        static_tile_animation_family_base(tile).map_or(tile, |base| base + self.frame)
    }

    pub fn resolve_moongate_tile(self) -> u8 {
        MOONGATE_TILE_BASE + self.moongate_frame
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveObject {
    pub type_byte: u8,
    pub tile: u8,
    pub x: usize,
    pub y: usize,
    pub z: i8,
    pub phase: u8,
    pub aux1: u8,
    pub aux3: u8,
}

impl ActiveObject {
    pub fn empty() -> Self {
        Self {
            type_byte: 0,
            tile: 0,
            x: 0,
            y: 0,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        }
    }

    pub fn moonstone_pickup(slot_index: usize, x: usize, y: usize, z: i8) -> Self {
        Self {
            type_byte: FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE,
            tile: FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1: slot_index as u8,
            aux3: MOONSTONE_PICKUP_AUX3,
        }
    }

    pub fn free(&mut self) {
        self.type_byte = 0;
    }

    pub fn tick_phase(&mut self) -> PhaseTick {
        let low = self.phase & 0x0f;
        if low == STEADY_PHASE {
            PhaseTick::Steady
        } else if low > 0 {
            self.phase = (self.phase & 0xf0) | (low - 1);
            PhaseTick::Countdown
        } else {
            PhaseTick::DecisionPoint
        }
    }

    pub fn is_player(self) -> bool {
        self.type_byte == PLAYER_TILE
    }

    pub fn is_player_phantom(self) -> bool {
        self.type_byte == PLAYER_NPC_SENTINEL_TYPE
    }

    pub fn is_empty(self) -> bool {
        self.type_byte == 0
    }

    pub fn moonstone_slot_index(self) -> Option<usize> {
        let slot_index = self.aux1 as usize;
        (self.type_byte == FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE
            && self.tile == FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE
            && self.aux3 == MOONSTONE_PICKUP_AUX3
            && slot_index < MOONSTONE_SLOT_COUNT)
            .then_some(slot_index)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhaseTick {
    Steady,
    Countdown,
    DecisionPoint,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActiveShipWind {
    None,
    Stalled,
    Drifted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameClock {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
}

impl GameClock {
    pub fn new(hour: u8, minute: u8) -> io::Result<Self> {
        Self::with_date(
            PLAY_START_YEAR,
            PLAY_START_MONTH,
            PLAY_START_DAY,
            hour,
            minute,
        )
    }

    pub fn with_date(year: u16, month: u8, day: u8, hour: u8, minute: u8) -> io::Result<Self> {
        if !(1..=13).contains(&month) || !(1..=28).contains(&day) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid Britannian date year {year}, month {month}, day {day}"),
            ));
        }
        if hour > 23 || minute > 59 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid clock time {hour:02}:{minute:02}"),
            ));
        }
        Ok(Self {
            year,
            month,
            day,
            hour,
            minute,
        })
    }

    pub fn advance_minutes(&mut self, minutes: u8) {
        let total = self.minute as u16 + minutes as u16;
        self.minute = (total % 60) as u8;
        for _ in 0..(total / 60) {
            self.advance_hour();
        }
    }

    pub fn display_hour(self) -> u8 {
        match self.hour {
            0 => 12,
            1..=12 => self.hour,
            _ => self.hour - 12,
        }
    }

    pub fn am_pm_suffix(self) -> &'static str {
        if self.hour < 12 { "A.M." } else { "P.M." }
    }

    pub fn advance_hour(&mut self) {
        self.hour += 1;
        if self.hour >= 24 {
            self.hour = 0;
            self.advance_day();
        }
    }

    pub fn advance_day(&mut self) {
        self.day += 1;
        if self.day > 28 {
            self.day = 1;
            self.month += 1;
            if self.month > 13 {
                self.month = 1;
                self.year = self.year.saturating_add(1);
            }
        }
    }
}

impl Default for GameClock {
    fn default() -> Self {
        Self {
            year: PLAY_START_YEAR,
            month: PLAY_START_MONTH,
            day: PLAY_START_DAY,
            hour: PLAY_START_HOUR,
            minute: 0,
        }
    }
}

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


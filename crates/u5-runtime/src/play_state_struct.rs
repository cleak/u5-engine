//! The PlayState struct and overlay caches (impl blocks live in parts/play_state_impl/).

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

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
    pub natural_moongate_counter: u8,
    pub cached_moon_glyph_slots: [Option<usize>; 2],
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
    pub equipment_stock: [u8; EQUIPMENT_COUNT],
    pub spell_charges: [u8; SPELL_COUNT],
    pub scroll_stock: [u8; SCROLL_COUNT],
    pub potion_stock: [u8; POTION_COUNT],
    pub reagents: [u8; REAGENT_COUNT],
    pub rare_reagent_harvest_days: [u8; RARE_REAGENT_HARVEST_POINT_COUNT],
    pub fixed_hidden_treasure_found: [u8; FIXED_HIDDEN_TREASURE_FOUND_BYTES],
    pub fixed_hidden_treasure_daily_day: u8,
    pub dungeon_room_clear_bitmap: [u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
    pub moonstone_slots: [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT],
    pub shadowlord_hideouts: [u8; SHADOWLORD_COUNT],
    pub shrine_ordained_mask: u8,
    pub shrine_codex_mask: u8,
    pub shrine_standing: [u8; VIRTUE_COUNT],
    pub moral_standing: u8,
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
    pub fortunes_of_war: u8,
    pub active_player: Option<usize>,
    pub combat_round_counter: u8,
    pub combat_active: bool,
    pub combat_frame_snapshot: Option<CombatFrameSnapshot>,
    pub pending_combat_actor_slot: Option<usize>,
    pub pending_combat_terrain_trigger_slot: Option<usize>,
    pub next_combat_actor_slot: usize,
    pub combat_terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub combat_actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
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
    pub pending_town_arrest: Option<TownArrestPrompt>,
    pub endgame: Option<EndgameState>,
    pub active_blackthorn: Option<crate::blackthorn_session::BlackthornChallenge>,
    pub blackthorn_jailed_party_slots: Vec<u8>,
    pub active_shop: Option<crate::shop_session::ActiveShopSession>,
    pub active_conversation: Option<Box<crate::conversation_session::ConversationSession>>,
    pub active_z_stats: Option<crate::z_stats::ZStatsSession>,
    pub active_ready: Option<crate::z_stats::ReadySession>,
    pub active_use: Option<crate::z_stats::UseSession>,
    pub pickpocketed_npcs: Vec<(u8, i8, usize)>,
    pub removed_town_npcs: Vec<(u8, i8, usize)>,
    pub town_npc_alarm_states: Vec<TownNpcAlarmMarker>,
    pub talk_branch_flags: HashMap<u8, u32>,
    pub inn_registry: Vec<InnGuestRecord>,
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
    pub pending_vehicle: Option<PendingVehicleAcquisition>,
}

//! The PlayState struct and overlay caches (impl blocks live in parts/play_state_impl/).

use std::collections::HashMap;

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
    pub grid: Vec<u8>,
    pub world_live_chunks: Option<WorldLiveChunkBuffer>,
    pub clock: GameClock,
    pub prng_state: u16,
    pub animation: AnimationClock,
    pub natural_moongate_counter: u8,
    pub natural_moongate_live_cells: Vec<usize>,
    /// `overworld.md §9.2` (spec HEAD `c00bf63`): what the last blocking
    /// moongate transit spent, or `None` if none has run in this session.
    ///
    /// Presentation bookkeeping, not saved state: the transit is blocking
    /// and unskippable, so this is only ever a record of a sequence that
    /// already finished, never a resumable position in one.
    pub last_natural_moongate_transit: Option<MoongateTransitPlayback>,
    pub cached_moon_glyph_bytes: [u8; 2],
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
    pub fixed_hidden_treasure_single_use_cookie: u8,
    pub dungeon_room_clear_bitmap: [u8; SAVE_DUNGEON_ROOM_CLEAR_BITMAP_LEN],
    pub moonstone_slots: [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT],
    pub shadowlord_hideouts: [u8; SHADOWLORD_COUNT],
    pub quest_progress_word: u16,
    pub shrine_ordained_mask: u8,
    pub shrine_codex_mask: u8,
    pub moral_standing: u8,
    pub toll_progress: u8,
    pub avatar_stats: AvatarStats,
    pub torches: u8,
    pub torch_counter: u8,
    pub light_spell_counter: u8,
    pub ambient_light: u8,
    /// `visibility.md §12.6`: the night-time rotating beacon's resident
    /// scratch block — up to two harvested source positions plus the
    /// beam's current bearing. Map loaders own the positions; the
    /// per-turn pass owns the bearing.
    pub light_beacon: LightBeaconState,
    /// `visibility.md §12.6` beam stencils, read from the shipped
    /// `DATA.OVL` at the offset `formats/tiles.md §5.1.1` publishes.
    ///
    /// Not optional. A state that reached this point has a table, because
    /// [`crate::load_beacon_bearing_stencils`] fails loudly rather than
    /// handing back an absent one — `§5.1.1` asks for exactly that, since
    /// a silently dark beacon and a missing table look identical in play.
    pub beacon_bearing_stencils: BeaconBearingStencils,
    pub visibility_dirty: bool,
    pub visibility_grid: [u8; VISIBILITY_GRID_LEN],
    pub terrain_band: [u8; TERRAIN_BAND_LEN],
    pub visibility_buffers_ready: bool,
    pub world_underfoot_blackout_latched: bool,
    pub wind: WindState,
    pub wind_save_byte: u8,
    pub timing_status: TimingStatusTag,
    pub time_stop_counter: u8,
    pub active_effect_tag: Option<u8>,
    pub active_effect_counter: u8,
    pub fortunes_of_war: u8,
    /// `rest-and-camp.md §5` camp cooldown counter. Armed at
    /// [`crate::COMPLETED_LONG_CAMP_COOLDOWN_HOURS`] whenever a camp
    /// completes and reduced by one, floored at zero, at every hour
    /// rollover; the completed-camp recovery walk runs only while it
    /// reads zero.
    ///
    /// **Not save-backed, deliberately.** The counter's fourteen-hour
    /// lifetime sits well inside the save window, so it *should*
    /// persist — but `formats/saved-gam.md` publishes no offset for it,
    /// and every byte in the band it would live in is either claimed by
    /// another system or covered by that document's instruction to
    /// "preserve adjacent unnamed bytes". Choosing a byte ourselves
    /// would overwrite one the original owns, which is a worse failure
    /// than the one this leaves: saving and reloading inside the
    /// fourteen-hour window clears the cooldown, so a save/load lets a
    /// second camp recover. Persist it here the moment the spec
    /// publishes an offset.
    pub camp_cooldown: u8,
    pub active_player: Option<usize>,
    pub combat_round_counter: u8,
    pub combat_active: bool,
    pub combat_frame_snapshot: Option<CombatFrameSnapshot>,
    pub pending_combat_actor_slot: Option<usize>,
    pub pending_combat_terrain_trigger_slot: Option<usize>,
    pub next_combat_actor_slot: usize,
    pub combat_terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub combat_magic_effects: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub combat_cursor_blink: bool,
    pub combat_secondary_marker: Option<(u8, u8)>,
    pub combat_ambush_reveals: [Option<CombatAmbushRevealRecord>; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
    pub combat_actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    pub sail_cadence: u8,
    pub sail_stall_pending: bool,
    pub turn: u64,
    pub message: String,
    /// `text-output.md §2` + `commands.md §5`: the scrolling message
    /// window is a transcript, not a single line. Each command turn opens
    /// one entry with the command's verb echo, and the handler's own
    /// prompt or result either continues that entry or starts the next
    /// one. Renderers read it through [`PlayState::message_entries`];
    /// `message` above remains the newest handler text for the many
    /// call sites and tests that assert on it directly.
    pub(crate) message_transcript: Vec<MessageEntry>,
    /// Bumped on every transcript push so callers can tell whether a
    /// dispatch already recorded its own output.
    pub(crate) message_transcript_revision: u64,
    /// The verb echo opened for the command currently being dispatched,
    /// if any, plus the message text that stood when it was opened.
    pub(crate) pending_command_echo: Option<PendingCommandEcho>,
    pub pending_hourly_status_message: Option<String>,
    pub debug_enter: Option<PlayTarget>,
    pub return_world: Option<WorldReturn>,
    pub world_overlays: WorldOverlayCache,
    pub save_template_source: SaveTemplateSource,
    pub typeahead_buffer_enabled: bool,
    pub music_enabled: bool,
    pub pending_town_arrest: Option<TownArrestPrompt>,
    pub endgame: Option<EndgameState>,
    pub active_blackthorn: Option<crate::blackthorn_session::BlackthornChallenge>,
    pub blackthorn_audience_map: Option<MiscmapsCutsceneMap>,
    pub blackthorn_story: BlackthornStoryState,
    pub active_shop: Option<crate::shop_session::ActiveShopSession>,
    pub common_word_dictionary: Option<crate::common_words_io::CommonWordDictionary>,
    pub active_conversation: Option<Box<crate::conversation_session::ConversationSession>>,
    pub active_conversation_join_candidate: Option<String>,
    pub active_z_stats: Option<crate::z_stats::ZStatsSession>,
    /// `inventory.md §4`: the outside-combat party-member selector that
    /// Z-stats opens before it binds a character. While it is live the
    /// roster box carries the `Select:` border label and its candidate
    /// row is drawn in inverse video; see
    /// [`PlayState::selector_highlight`] and
    /// [`PlayState::roster_box_label`].
    pub active_party_selector: Option<crate::z_stats::PartySelectorSession>,
    pub active_ready: Option<crate::z_stats::ReadySession>,
    pub active_use: Option<crate::z_stats::UseSession>,
    pub active_cast: Option<crate::z_stats::CastSession>,
    pub active_cast_followup: Option<crate::z_stats::CastFollowupSession>,
    pub active_rest: Option<crate::z_stats::RestSession>,
    pub active_jimmy: Option<crate::z_stats::JimmySession>,
    pub active_surface_chest: Option<crate::z_stats::SurfaceChestSession>,
    pub active_shrine: Option<crate::z_stats::ShrineSession>,
    pub active_mix: Option<crate::z_stats::MixSession>,
    pub active_new_order: Option<crate::z_stats::NewOrderSession>,
    pub active_yell: Option<crate::z_stats::YellSession>,
    pub active_wishing_well: Option<crate::z_stats::WishingWellSession>,
    pub active_view_overlay: Option<ViewOverlay>,
    pub white_potion_sweep: Option<WhitePotionSweep>,
    pub combat_potion_presentation: Option<CombatPotionPresentation>,
    pub active_direction_prompt: Option<crate::z_stats::DirectionPromptSession>,
    pub active_yes_no_prompt: Option<crate::z_stats::YesNoPromptSession>,
    pub pickpocketed_npcs: Vec<(u8, i8, usize)>,
    pub removed_town_npcs: Vec<(u8, i8, usize)>,
    pub town_npc_alarm_states: Vec<TownNpcAlarmMarker>,
    pub talk_branch_flags: HashMap<u8, u32>,
    pub conversation_resource_signals: [u8; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT],
    pub conversation_signal_flags: [u8; TLK_GENERIC_SIGNAL_COUNT],
    pub conversation_signal_bank_a: [u8; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
    pub conversation_signal_bank_b: [u8; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
    pub inn_registry: Vec<InnGuestRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewOverlay {
    pub title: String,
    pub text_map: String,
    pub kind: ViewOverlayKind,
    pub mode: ViewOverlayMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewOverlayKind {
    Surface,
    BritanniaChunkMap,
    Dungeon { level: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewOverlayMode {
    GemView,
    PeerSpell,
    XRaySpell,
    SurfaceLook,
    BritanniaOverview,
}

impl ViewOverlayMode {
    pub const fn uses_alternate_view_bank(self) -> bool {
        matches!(self, Self::GemView | Self::PeerSpell)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhitePotionSweep {
    pub frames_remaining: u8,
    pub radius: u8,
    pub center_x: usize,
    pub center_y: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CombatPotionPresentation {
    pub kind: CombatPotionPresentationKind,
    pub actor_slot: usize,
    pub active_object_slot: usize,
    pub frames_remaining: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CombatPotionPresentationKind {
    Sleep,
    Poof,
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

/// One line of the scrolling message-window transcript.
///
/// `commands.md §5` requires each command block to print a resident verb
/// prefix before its handler runs; the original renders those lines with
/// a leading `>` glyph and continuation lines without one. `is_command_echo`
/// carries that distinction to the renderer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageEntry {
    pub text: String,
    pub is_command_echo: bool,
}

/// Bookkeeping for a verb echo that has been written to the transcript
/// but whose handler has not finished yet.
#[derive(Clone, Debug)]
pub(crate) struct PendingCommandEcho {
    pub(crate) echo: CommandEcho,
    /// `PlayState::message` as it stood when the echo was opened, so the
    /// commit step can tell whether the handler wrote anything at all.
    pub(crate) message_at_entry: String,
}

/// How many transcript entries are retained. The original scrolls its
/// twelve-row message window; keeping a few screens of history lets a
/// renderer scroll back without the buffer growing without bound over a
/// long session.
pub const MESSAGE_TRANSCRIPT_CAPACITY: usize = 64;

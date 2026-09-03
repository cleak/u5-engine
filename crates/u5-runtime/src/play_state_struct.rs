//! The PlayState struct and overlay caches (impl blocks live in parts/play_state_impl/).

use std::collections::HashMap;

use crate::*;

/// Frontend-facing sound boundaries published by the clean specification.
///
/// The runtime records only the effect identity and a monotonically changing
/// serial; each frontend owns synthesis and playback. The effect vocabulary is
/// `crate::audio::SoundEffect`, whose variants are exactly the confirmed
/// trigger inventory of `systems/audio.md`.
pub use crate::audio::SoundEffect;

/// `audio.md §2` keeps one serial speaker, so a frontend only ever needs the
/// most recent boundaries; this bounds the non-saved history.
///
/// The bound has to clear the longest single blocking sequence the engine can
/// emit between two frontend drains, because eviction drops the oldest entry
/// while the serial keeps counting — a lost cue looks exactly like a cue that
/// never fired. The worst published case is the `town-mode.md §7.1` Stonegate
/// scripted death: the ordinary trapdoor prefix rolls `1..8` damage against
/// every party slot (one `§8.2` damage rumble each), then the script emits its
/// descent sweep and one further rumble per slot as it kills them. With a full
/// party that is fifteen boundaries from one keystroke, so sixteen left no
/// margin at all.
pub const SOUND_EFFECT_HISTORY_CAPACITY: usize = 64;

#[derive(Clone, Debug)]
pub struct PlayState {
    pub area: Area,
    pub player: Player,
    pub active_objects: Vec<ActiveObject>,
    pub npcs: Vec<RuntimeNpc>,
    pub door_tracker: Option<DoorTracker>,
    /// Runtime-only latch: the pending auto-close in
    /// [`Self::door_tracker`] has already fired. The four save bytes at
    /// `0x03A9..0x03AC` stay resident afterwards (the original never clears
    /// them), so this flag — not a cleared block — is what keeps the close
    /// from firing a second time when the countdown wraps.
    pub door_tracker_closed: bool,
    pub opened_town_doors: Vec<(u8, i8, usize, usize)>,
    pub revealed_town_secret_doors: Vec<(u8, i8, usize, usize)>,
    pub passability: Option<TilePassability>,
    pub grid: Vec<u8>,
    pub world_live_chunks: Option<WorldLiveChunkBuffer>,
    pub clock: GameClock,
    /// `time.md §5`: "The pass keeps its own previous-hour snapshot and
    /// compares it with the current hour." The party status/provision pass
    /// runs once per turn-consuming action, so it cannot share the per-turn
    /// cleanup's local snapshot — only its food/starvation branch is gated on
    /// the hour changing, and the camp loop advances the clock without
    /// entering the pass at all.
    pub status_pass_previous_hour: u8,
    /// `formats/saved-gam.md §5` / `time.md §2`: the per-turn cleanup's
    /// pre-cascade hour snapshot at save offset `0x02DA`, "taken at the
    /// start of every cleanup pass" and compared against the post-cascade
    /// hour. Distinct from [`Self::status_pass_previous_hour`], which is the
    /// status/provision pass's own snapshot (`time.md §5`).
    pub cleanup_previous_hour: u8,
    /// `formats/saved-gam.md §5` / `time.md §11` (spec `0170809`): save
    /// byte `0x02DE`. "On a cleanup call whose snapshot at `0x02DA`
    /// disagrees with the hour at `0x02D9`, the byte takes the twelve-hour
    /// form of the hour ... That is the whole write rule; there is no
    /// second writer." Its only consumer is the ambient-audio tick, which
    /// "reads the byte as a count of remaining loud repeats ... and
    /// decrements it toward zero on **two of every eight** of its own
    /// calls". `RETRACTIONS.md` R338 withdraws the old "12-hour display"
    /// reading: nothing in the shipped game renders it.
    pub twelve_hour_audio_repeats: u8,
    /// Free-running sub-tick behind the two-in-eight decrement above.
    /// `time.md §11`: it uses "a small free-running sub-tick counter that
    /// is not part of the save image", so this field is deliberately not
    /// persisted.
    pub ambient_audio_sub_tick: u8,
    /// `dungeon-mode.md §15`: the dungeon loop charges one minute at the
    /// head of every iteration, ungated on whether the command consumed a
    /// turn. This flag records that the iteration's minute is already spent so
    /// a turn-consuming dungeon handler's own `advance_turn` bumps the action
    /// counter and runs its epilogues without charging the clock twice.
    pub dungeon_loop_minute_charged: bool,
    pub prng_state: u16,
    pub animation: AnimationClock,
    /// The one global water-surface scroll counter.
    ///
    /// Runtime observation, `cleak/u5-spec#179`: every water tile on
    /// screen advances on the same tick, so this is one counter for the
    /// whole map rather than a per-cell phase. It is deliberately
    /// separate from [`AnimationClock`], which owns the published
    /// `animation.md §6` family pass and nothing else — see
    /// [`crate::water_scroll`].
    pub water_scroll: WaterScrollClock,
    /// The driver-side fire animator's accumulated state.
    ///
    /// `animation.md §12.4`: the fire fixtures animate by a cumulative
    /// masked-noise XOR with no frame set to enumerate, so unlike
    /// [`WaterScrollClock`] this cannot be a phase counter. It rides the
    /// same driver pass on the same tick - see [`crate::fire_flicker`].
    pub fire_flicker: FireFlickerClock,
    /// `dungeon-mode.md §6.7`: shared three-frame fountain-water phase,
    /// advanced once per point-blank corridor paint.
    pub dungeon_fountain_frame: u8,
    pub natural_moongate_counter: u8,
    pub natural_moongate_live_cells: Vec<usize>,
    /// `overworld.md §9.2` (spec HEAD `c00bf63`): what the last blocking
    /// moongate transit spent, or `None` if none has run in this session.
    ///
    /// Presentation bookkeeping, not saved state: the transit is blocking
    /// and unskippable, so this is only ever a record of a sequence that
    /// already finished, never a resumable position in one.
    pub last_natural_moongate_transit: Option<MoongateTransitPlayback>,
    /// Completed blocking map-viewport dissolves waiting for a frontend to
    /// present them. This is transient presentation state, never save data.
    pub pending_map_viewport_dissolves: Vec<MapViewportDissolvePlayback>,
    /// Completed Blackthorn rescue tableau between its two rectangle
    /// dissolves. Transient presentation state, never serialized.
    pub pending_blackthorn_rescue_playbacks: Vec<BlackthornRescuePlayback>,
    /// Completed vanish-on-death single-cell reveals waiting for the frontend.
    /// The runtime has already exhausted all 256 pixel operations and 31
    /// world ticks; this transient record preserves their exact order.
    pub pending_combat_terrain_reveals: Vec<CombatTerrainRevealPlayback>,
    /// `catalogs/item-list.md §7.2`: a selected-bottle flash waiting for the
    /// frontend to present it. This is transient presentation state, never
    /// resumable or saved gameplay state.
    pub pending_potion_flash: Option<PotionFlashPlayback>,
    /// Completed Stonegate trapdoor tableau waiting for a frontend. This is a
    /// blocking, non-resumable presentation record and is never save data.
    pub pending_stonegate_trapdoor_playback: Option<StonegateTrapdoorPlayback>,
    /// `town-mode.md §7`/§10: an ordinary one-minute town turn advances the
    /// clock first, then runs tile effects, and only then enters the shared
    /// party status/provision pass. This flag carries that pass from the
    /// clock routine to the trailing edge of the town underfoot handler.
    /// Transient within one town action and never serialized.
    pub pending_town_status_provision_pass: bool,
    /// `town-mode.md §7`: the NPC scheduler follows the underfoot/status pass
    /// and the slot-zero coordinate copy. This flag carries that scheduler
    /// call across the I/O-bearing underfoot handler. Stonegate uses the same
    /// tail after its script-owned object-table clear. Never serialized.
    pub pending_town_npc_schedule_pass: bool,
    /// Whether the deferred town tail also owes the ordinary active-object
    /// animator/free-roaming pass requested by the clock caller. This remains
    /// separate because some low-level time calls deliberately suppress that
    /// pass while still owing the NPC scheduler. Never serialized.
    pub pending_town_active_object_pass: bool,
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
    /// `combat.md §12`: the cached combat-defense byte the damage roller
    /// reads for a party defender, at character-record offset `+0x18`,
    /// one entry per roster slot.
    pub party_combat_defense: Vec<u8>,
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
    pub moonstone_slots: [MoonstoneGateSlot; MOONSTONE_SLOT_COUNT],
    pub shadowlord_hideouts: [u8; SHADOWLORD_COUNT],
    /// `town-mode.md §13` entry-local selector naming which Shadowlord is
    /// resident in the current town. Reset before every entry comparison;
    /// transient runtime state, not part of the save image.
    pub resident_shadowlord: Option<usize>,
    /// `commands.md §11` transient identity handshake for a Shadowlord
    /// summoned by name. The actor record itself carries only the shared
    /// `0xFC` identity bytes.
    pub summoned_shadowlord: Option<usize>,
    pub removed_town_npc_flags: HashMap<u8, u32>,
    pub shrine_ordained_mask: u8,
    pub shrine_codex_mask: u8,
    pub word_of_power_seal_flags: [u8; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
    pub shrine_ruin_flags: [u8; SAVE_SHRINE_RUIN_FLAG_COUNT],
    pub moral_standing: u8,
    /// Visit-local tavern drunken-command counter. It is armed to 25 by
    /// accepting a fourth secondary drink and is cleared on town entry/exit.
    /// This is transient runtime state and is never serialized.
    pub town_drunkenness_counter: u8,
    /// Number of successful secondary-drink purchases in the current tavern
    /// visit. Reset whenever Talk opens a new tavern session.
    pub tavern_secondary_drink_count: u8,
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
    /// `visibility.md §12.4`: persistent 32x32 influence mask produced by
    /// the local-light refresh pass. It is rebuilt only by the published
    /// Moonstone-refresh and combat-boundary triggers; ordinary visibility
    /// carves consume this cached state without rescanning sources.
    pub local_light_mask: [bool; TOWN_GRID_BYTES],
    pub visibility_dirty: bool,
    pub visibility_grid: [u8; VISIBILITY_GRID_LEN],
    pub terrain_band: [u8; TERRAIN_BAND_LEN],
    pub visibility_buffers_ready: bool,
    pub world_underfoot_blackout_latched: bool,
    pub wind: WindState,
    pub wind_save_byte: u8,
    pub time_stop_counter: u8,
    pub active_effect_tag: Option<u8>,
    pub active_effect_counter: u8,
    pub fortunes_of_war: u8,
    /// `rest-and-camp.md §5` camp cooldown counter. Armed at
    /// [`crate::COMPLETED_LONG_CAMP_COOLDOWN_HOURS`] whenever a camp
    /// completes and reduced by one, floored at zero, at every hour
    /// rollover; the completed-camp recovery walk runs only while it
    /// reads zero.
    /// Persisted at `SAVED.GAM` offset `0x02E6`; save/reload therefore
    /// cannot clear the recovery window.
    pub camp_cooldown: u8,
    /// `rest-and-camp.md §5`: the successful apparition draw copies
    /// the current calendar month into `SAVED.GAM` offset `0x02E7`.
    /// There is no shipped reader, so preserve this cookie without
    /// deriving any additional behaviour from it.
    pub camp_month_cookie: u8,
    pub active_player: Option<usize>,
    pub combat_round_counter: u8,
    /// `combat.md §6.3`: global combat action-result/narration scratch.
    /// It is reset before actor dispatch and has no resumable combat lifetime.
    pub combat_action_result: u8,
    /// `magic.md §7`: save-backed per-victim source slots used by the C-Cast
    /// interference gate. This state intentionally survives combat boundaries.
    pub combat_interference_sources: [u8; COMBAT_ACTOR_SLOTS],
    pub combat_active: bool,
    /// Frontend presentation policy, never serialized. Graphical frontends
    /// set this so the automatic actor walk stops after each visible action;
    /// batch-oriented callers retain the blocking walk by default.
    pub pace_combat_presentations: bool,
    pub combat_frame_snapshot: Option<CombatFrameSnapshot>,
    pub pending_combat_actor_slot: Option<usize>,
    pub pending_combat_terrain_trigger_slot: Option<usize>,
    /// `town-mode.md §14`: the town NPC-conflict chain's carry-over.
    /// A-Attack on a town actor enters the ordinary terrain arena, and
    /// "On exit the town chain clears the NPC slot, reloads the town
    /// map, and re-runs the Shadowlord install pass of Section 13".
    pub pending_town_conflict: Option<PendingTownConflict>,
    /// High-to-low outdoor reaction slots staged by the I/O-free active-object
    /// walker. Lower entries survive a terrain-combat frame and resume when
    /// that frame returns to the world.
    pub pending_outdoor_reaction_slots: Vec<usize>,
    pub next_combat_actor_slot: usize,
    pub combat_terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub combat_magic_effects: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub combat_cursor_blink: bool,
    /// `combat.md §7`/`§5.3` step 8: has this encounter's round-loop entry
    /// prologue already run? The prologue "runs once per entry into the
    /// round loop, and the loop is entered once per encounter: the sweep
    /// restart jumps back past the prologue" (`RETRACTIONS.md` R308).
    pub combat_round_loop_prologue_ran: bool,
    /// `combat.md §7`, "Loop-entry prologue": the bundle includes a
    /// "per-slot scratch state reset". `§7` names the scratch only by that
    /// phrase - it publishes no reader, no writer and no field layout for
    /// it - so this engine models it as one opaque byte per actor slot,
    /// zeroed by the prologue. Nothing else reads or writes it; the hedge is
    /// recorded rather than filled in with an invented meaning.
    pub combat_round_slot_scratch: [u8; COMBAT_ACTOR_SLOTS],
    /// `combat.md §7`, "Loop-entry prologue": the bundle ends by "clearing
    /// the 'any spell cast this round' flag". As with the scratch above, the
    /// specification publishes the *clear* and nothing else - no reader and
    /// no writer anywhere in `combat.md` or `magic.md` - so the flag exists
    /// here with exactly the published lifetime and no invented consumer.
    pub combat_spell_cast_this_round: bool,
    /// `combat.md §4` restore phase: "If the resident tile-restoration flag
    /// is set when the round loop returns, clear that flag and invoke the
    /// display driver's tile-graphics save/restore/mutation entry with mode
    /// value `1` before the ordinary world redraw." The setter is the
    /// dungeon room painter's two-way ladder cell (`dungeon-mode.md §14.1`),
    /// which is outside combat's contract: "combat owns only the
    /// sampling/clear/call ordering, while the setter provenance and
    /// tile-asset mutation details belong to the dungeon and driver specs."
    /// Transient presentation handoff, never serialized.
    pub tile_restoration_pending: bool,
    /// Driver tile-graphics restores the combat framer sampled out of
    /// [`Self::tile_restoration_pending`] and owes a frontend. The runtime
    /// has no display driver of its own, so it records the request in the
    /// published order - ahead of the restore phase's world redraw - and a
    /// frontend drains it by issuing
    /// [`crate::EgaDisplayOperation::RestoreLoadedTileGraphics`].
    pub pending_driver_tile_graphics_restores: usize,
    pub combat_secondary_marker: Option<(u8, u8)>,
    pub combat_ambush_reveals: [Option<CombatAmbushRevealRecord>; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
    pub combat_actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    pub sail_cadence: u8,
    pub sail_stall_pending: bool,
    /// Exact queued shipwright-delivery bytes from `SAVED.GAM`. The packed
    /// class is cleared only when world setup successfully delivers it.
    pub pending_vehicle_save: PendingVehicleSaveState,
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
    /// `text-output.md §11`: the original "has no message slot to
    /// overwrite" — text is a stream into a windowed grid, so a turn
    /// that produces two lines shows both. `message` above cannot
    /// report its own writes, so this shadows the value most recently
    /// appended to `message_transcript`: any later value differing from
    /// it is an emission the transcript has not seen, and
    /// [`PlayState::flush_message_slot`] appends it before the slot can
    /// be overwritten again.
    pub(crate) message_flushed: String,
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
    /// Non-saved presentation event. The serial lets a frontend distinguish a
    /// new occurrence from a redraw of the same message or state.
    pub sound_effect_serial: u64,
    pub(crate) sound_effect_history: Vec<(u64, SoundEffect)>,
    /// `town-mode.md §13`: how many leading notes of the thirteen-note
    /// harpsichord tune have been played. Non-saved runtime state; the
    /// section is explicit that leaving the chair does not clear it, so only
    /// a wrong note or a completed tune resets it.
    pub(crate) harpsichord_progress: usize,
    pub active_blackthorn_guard_demand: Option<ActiveBlackthornGuardDemand>,
    pub pending_town_arrest: Option<TownArrestPrompt>,
    pub endgame: Option<EndgameState>,
    pub active_blackthorn: Option<crate::blackthorn_session::BlackthornChallenge>,
    pub blackthorn_audience_map: Option<MiscmapsCutsceneMap>,
    pub active_shop: Option<crate::shop_session::ActiveShopSession>,
    pub common_word_dictionary: Option<crate::common_words_io::CommonWordDictionary>,
    pub active_conversation: Option<Box<crate::conversation_session::ConversationSession>>,
    /// NPC roster slot captured when the active conversation opens. The
    /// opening acquaintance test addresses this slot in the scene's TALK bitset.
    pub active_conversation_npc_slot: Option<usize>,
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
    pub active_shrine_restoration: Option<crate::z_stats::ShrineRestorationSession>,
    pub active_wishing_well: Option<crate::z_stats::WishingWellSession>,
    pub active_view_overlay: Option<ViewOverlay>,
    pub visibility_sweep: Option<VisibilitySweep>,
    /// `overworld.md §8.1`: the two scripted swallow presentations replace or
    /// suppress the party marker for part of their run - the falls chain
    /// "hides" it across the damage pass, and the whirlpool swaps in the
    /// whirlpool sprite before the long descent - and both restore it before
    /// the state commit. `vehicles.md §2` marker `0x00` is the
    /// sprite-suppressed party, which is what "hidden" means here.
    ///
    /// This is presentation only: it never reaches the durable transport
    /// marker, and [`PlayState::sync_player_object`] re-applies it after the
    /// ordinary slot-zero refresh so a tick inside the presentation cannot
    /// undo it.
    pub party_marker_tile_override: Option<u8>,
    pub active_direction_prompt: Option<crate::z_stats::DirectionPromptSession>,
    pub active_yes_no_prompt: Option<crate::z_stats::YesNoPromptSession>,
    pub town_npc_mutations: Vec<TownNpcMutation>,
    pub talk_branch_flags: HashMap<u8, u32>,
    pub conversation_signal_flags: [u8; TLK_GENERIC_SIGNAL_COUNT],
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
    Sky(SkyOverlayState),
    Dungeon { level: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewOverlayMode {
    GemView,
    PeerSpell,
    XRaySpell,
    SurfaceLook,
    SkyView,
}

impl ViewOverlayMode {
    /// `view.md §4`: the 32x32 LOOKOBJ overlay classes `0xA`/`0xB`/`0xC`/
    /// `0xD`/`0xF` publish modal normal-vs-peer source families, so the
    /// **surface** overlay does pick an alternate pen family in the gem and
    /// peer-spell modes.
    ///
    /// This is a surface-only distinction. `view.md §6.3` and
    /// `dungeon-mode.md §12.4` withdraw the matching dungeon reading: "the
    /// value they were reading is the **display-adapter identifier**, not a
    /// peer-spell flag ... V-View has no peer-spell branch of its own." The
    /// dungeon minimap painters must therefore not call this.
    pub const fn uses_alternate_surface_view_bank(self) -> bool {
        matches!(self, Self::GemView | Self::PeerSpell)
    }
}

/// The shared spell/potion visibility sweep: the body of the White potion
/// (`catalogs/item-list.md §7.2`) and of the sixth-circle X-Ray spell
/// *Wis An Ylem* (`systems/magic.md §8` utility effects) — "the two are the
/// only callers of it".
///
/// The sweep calls the visibility producer once with the negative
/// no-line-of-sight sentinel in the light argument, so the field it freezes is
/// the producer's full-fill branch: all 121 cells of the eleven-by-eleven
/// window, straight from the map, with no distance test, no propagation
/// frontier and no blocker rule (`systems/visibility.md §3`/`§4`, corrected by
/// R327). It then holds that unchanged field through twenty repaints.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibilitySweep {
    pub frames_remaining: u8,
    pub pause_bios_ticks_per_frame: u8,
    pub center_x: usize,
    pub center_y: usize,
    /// The producer runs exactly once. Terrain, objects, and animated tiles
    /// are recomposited for each frame through this frozen visibility field.
    pub visible_cells: [bool; VIEWPORT_SIDE * VIEWPORT_SIDE],
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
    /// Fixed-font choice for each byte in `text`. Plain engine messages use
    /// the ordinary font; TLK `0x8E` spans retain their runic selection here.
    pub glyphs: Vec<TlkRenderedGlyph>,
    pub is_command_echo: bool,
    /// Center this output line in the sixteen-cell message window. Cursor
    /// centering is presentation state, not ASCII padding in `text`.
    pub centered: bool,
    /// Preserve this empty output row instead of treating it as an empty slot.
    pub explicit_blank: bool,
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

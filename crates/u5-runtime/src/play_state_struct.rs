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
    /// `formats/saved-gam.md §5` / `time.md §2`: the per-turn cleanup's
    /// pre-cascade hour snapshot at save offset `0x02DA`, "taken at the
    /// start of every cleanup pass" and compared against the post-cascade
    /// hour. `time.md §5` (u5-spec#298 Q27, `RETRACTIONS.md` R548): the party
    /// upkeep pass "compares and then rewrites the shared pre-cascade hour
    /// snapshot, save byte `0x02DA`" and "has no snapshot of its own" - "an
    /// implementation must keep one byte for both uses".
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
    /// Dungeon loop iterations begun in this scene (one per
    /// [`PlayState::advance_dungeon_loop_minute`]). `dungeon-mode.md §10`'s
    /// inner loop paints the first-person view once per iteration, in its
    /// render-and-poll step; this counter is what lets a frontend that
    /// repaints every frame still render "once" per iteration.
    pub dungeon_loop_iteration: u32,
    /// `dungeon-mode.md §6.4`'s flavour-3 flourish draw for the iteration it
    /// was taken in, and whether it lit the skeleton's eyes.
    pub dungeon_flourish_draw: Option<(u32, bool)>,
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
    /// The last map-view frame this state painted. `active-objects.md §12`:
    /// a cell whose actor carries the draw-nothing byte `0x16` "keeps
    /// whatever was already painted there rather than showing terrain", so
    /// the renderer copies that cell from here instead of leaving the fresh
    /// buffer's black (playtest Q06).
    pub top_down_previous_viewport: Option<TileViewport>,
    /// `dungeon-mode.md §6.7`: shared three-frame fountain-water phase,
    /// advanced once per point-blank corridor paint.
    pub dungeon_fountain_frame: u8,
    /// `dungeon-mode.md §14` (u5-spec#298 Q33): the wandering monster's
    /// stored **previous** cell, the second of its two stored positions.
    /// Fresh setup (`§4.1`) writes the placed X/Y to both; an accepted
    /// step moves the current cell here. Where the original keeps this
    /// across a save is not published, so `None` (a resumed game) reads
    /// as the monster's current cell, the value setup would have written.
    pub dungeon_monster_previous_cell: Option<(usize, usize)>,
    /// `audio.md §7.4` (u5-spec#300 Q32): the footstep rumble's current
    /// intensity and its parity ("just emitted"). A committed step plays 15
    /// and 11 and leaves 7 armed; later dungeon ticks alternate silent and
    /// emitting, so the second plays the 7 and the fourth the 3.
    pub dungeon_footstep_intensity: u8,
    pub dungeon_footstep_just_emitted: bool,
    /// `dungeon-mode.md §14.1` (u5-spec#298 Q1): a post-combat code 5 or 6
    /// that leaves the level stack takes the surface exit of `§13.2`, which
    /// needs the game directory; the framer's restore owes it to the input
    /// boundary as (scene, level the edge was reached at).
    pub pending_dungeon_surface_exit: Option<(DungeonScene, u8)>,
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
    /// `magic.md §8`: the message window frozen while a spell's whole-view
    /// flash plays. See [`MessageWindowFlashHold`]. Transient presentation
    /// state, never saved.
    pub message_window_flash_hold: MessageWindowFlashHold,
    /// `animation.md §13.2` (R639): the world tick's gate byte is restored
    /// from the save (`0x02EB`), and "A first step is bare only when the
    /// loaded file holds zero in the byte". Set by [`PlayState::load_scene`]
    /// from that byte; the first world step clears it and advances nothing.
    pub session_start_world_step_frozen: bool,
    /// `timing.md §8.2`: "On the overworld the input helper performs one
    /// world step when it is entered" - with no wait of its own (R371) - and
    /// `stats-panel.md §2.3` places the overworld panel drain "in the shared
    /// input helper, after that helper's world-tick call and before the
    /// prompt newline and the key read". Set when the overworld loop is about
    /// to re-enter that helper: at load and after every dispatched key. A
    /// frontend with a timer-paced idle pump consumes it through
    /// [`PlayState::run_overworld_input_helper_entry_step`]; the terminal
    /// shell runs its own entry pass per key read instead. Transient, never
    /// saved.
    pub overworld_input_helper_entry_owed: bool,
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
    /// Whether the deferred town tail also owes the per-slot **animator**
    /// pass. It is a separate flag from the walker pass above because
    /// `npc-schedules.md §5`'s three effect gates skip "**both** town walkers
    /// — the object walker that moves loose horse-family objects and this
    /// schedule processor" and the animator is neither: it "moves no actor"
    /// (`RETRACTIONS.md` R316) and `npc-schedules.md §12` runs it
    /// "independently of the scheduler, once per world tick rather than once
    /// per player turn" (`RETRACTIONS.md` R373 withdrew the older "each
    /// render frame" wording; §12 also answers issue #189 that the cadence is
    /// **descriptive**). Never serialized.
    pub pending_town_active_object_animate_pass: bool,
    /// `overworld.md §5` step 6h: the outdoor object epilogue (encounter
    /// roll, then animate/move/prune) runs after the narrative gate, the
    /// status pass and the underfoot chain. The clock routine takes the
    /// cadence-gate decision; this flag carries the walker it owes to that
    /// epilogue. Never serialized.
    pub pending_world_object_walker_pass: bool,
    /// `npc-schedules.md §5` gate 1 / `encounters.md §2.1` gate 3: "While
    /// the party's transport marker is one of the four values `0x12..0x15`,
    /// a stored parity bit flips each turn and the loop skips **both** town
    /// walkers on the turns where it comes up set". The bit is stored, not
    /// derived from the turn counter, because "an early return leaves the
    /// later gates' parity bits un-flipped" - so whether it advances on a
    /// given turn depends on which mode's gate order reached it. Never
    /// serialized.
    pub transport_walker_gate_parity: bool,
    /// `npc-schedules.md §5` gate 3 / `encounters.md §2.1` gate 2: the
    /// Quickness parity bit, stored for the same reason as
    /// [`Self::transport_walker_gate_parity`]. Never serialized.
    pub quickness_walker_gate_parity: bool,
    /// `encounters.md §2.1`: the three effect gates "sit ahead of the
    /// encounter probe *and* ahead of the outdoor creature walker, so a gate
    /// that fires costs the turn its probe as well as its creature movement".
    /// The gates are evaluated once, in the per-turn clock routine, and the
    /// outdoor post-action epilogue reads that one decision from here instead
    /// of re-testing them - a second test would flip the stored parity bits
    /// twice in one turn. Never serialized.
    pub world_walkers_ran_this_turn: bool,
    /// `timing.md §8.2`: which half of the under-sail auto-advance route the
    /// next idle-wait pass owes. The route is "one world step followed by one
    /// one-tick wait - before ... when sails are set, performing a bare
    /// cursor poll instead", so a full pass is two ticks with the world step
    /// on the first of them. Cleared whenever the route does not apply.
    /// Never serialized.
    pub under_sail_wait_cursor_poll_pending: bool,
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
    /// `commands.md §11` / `formats/saved-gam.md §9.1` `0x0325`: the id of
    /// the Shadowlord last summoned by name. Only a successful Yell writes
    /// it and nothing clears it (u5-spec#300 Q3, R585), so it may name a
    /// Shadowlord no longer present. The actor record itself carries only
    /// the shared `0xFC` identity bytes.
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
    /// A correct Blackthorn answer has been given and the companion's
    /// "merciful death" is owed: the execution's blade hold is staged and
    /// the roster edit lands when it ends.
    ///
    /// `blackthorn.md §5` (u5-spec#298 Q31): "on a correct answer the
    /// companion leaves the stats panel at the end of the blade-and-impact
    /// hold", which "is not a key wait"; the key that follows belongs to the
    /// §4.2 exit beat.
    pub(crate) pending_blackthorn_merciful_death: bool,
    /// The execution's quiet 10-tick pause is staged and its swell and
    /// impact rumble (`blackthorn.md §5` steps 3-4) are still to sound, at
    /// the end of that pause.
    pub(crate) pending_blackthorn_execution_swell: bool,
    /// The fourth-wrong-answer execution's victim, lifted at the end of the
    /// blade hold, before the sliced page (u5-spec#298 Q31).
    pub(crate) pending_blackthorn_execution_victim: Option<usize>,
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
    /// `combat.md §11.1`: the target slot whose flee classifier (and its
    /// `§9` morale draw) the single direction-keyed swing already ran at hit
    /// time, ahead of the committed-action tail. The result narrator consumes
    /// it instead of classifying the same hit a second time.
    pub combat_wound_classified_at_hit: Option<usize>,
    /// `combat.md §11.1`: the `<monster> divides!` line the damage handler
    /// printed, held until the shared result narrator places it - after the
    /// hit newline and before the result line ("Order, stated once" steps
    /// 4 and 5). Any line no narrator claims is printed at the next actor
    /// dispatch.
    pub combat_pending_divide_line: Option<String>,
    /// `combat.md §11.1`: result lines the per-turn standing hazard's shared
    /// narrator produced, by acting slot, held until that actor's own output
    /// (its command echo, its attack narration) has printed.
    pub combat_pending_hazard_lines: Vec<(usize, String)>,
    /// `magic.md §7`: save-backed per-victim source slots used by the C-Cast
    /// interference gate. This state intentionally survives combat boundaries.
    pub combat_interference_sources: [u8; COMBAT_ACTOR_SLOTS],
    pub combat_active: bool,
    /// Frontend presentation policy, never serialized. Graphical frontends
    /// set this so the automatic actor walk stops after each visible action;
    /// batch-oriented callers retain the blocking walk by default.
    pub pace_combat_presentations: bool,
    /// Frontend input policy, never serialized. The terminal harness keeps
    /// its `<` / `>` climb and `.` idle-tick shortcuts, which play scripts
    /// rely on. They are not commands of the game - `commands.md §5.2` makes
    /// `K` the only climb verb and answers any unmapped key `What?` - so a
    /// frontend that delivers real keystrokes clears this, and those bytes
    /// then take that refusal (playtest H18).
    pub harness_shortcut_keys: bool,
    pub combat_frame_snapshot: Option<CombatFrameSnapshot>,
    pub pending_combat_actor_slot: Option<usize>,
    /// Whether the message window's cursor is standing part-way along a row
    /// that already carries text.
    ///
    /// `text-output.md §10.4`: "the per-cell emitter treats the line-feed
    /// byte as a combined carriage return and line feed: it advances the row
    /// *and* returns the column to the window's left edge", so a print that
    /// opens with a line feed lands on the next row and "the next cycle's
    /// leading line feed advances again - producing exactly one blank row"
    /// when the cursor was already at column 0. Combat prints line by line
    /// as it produces them (`combat.md §11.1`'s order, whose every step
    /// begins with a newline), so the arena transcript needs to know which
    /// of the two cases it is in. Never serialized: it is presentation
    /// position, not saved state.
    pub combat_transcript_row_open: bool,
    /// Whether the arena prompt now waiting for a key is standing on the
    /// row a `combat.md §8.1` turn banner opened.
    ///
    /// §8.1 spends the marker row's line feed in the banner - the turn
    /// handler "emits the line feed itself, unconditionally, between
    /// printing the banner and reading the command byte" - so a prompt that
    /// banner opened must not also take `text-output.md §10.4`'s derived
    /// blank row. A **free re-prompt after a refusal** is the case that
    /// separates the two: §8.1 says it "uses the short form and does
    /// **not** reprint the banner", so no line feed was spent for it and
    /// its marker row takes §10.4's blank like any world-loop prompt.
    /// Never serialized: presentation position, not saved state.
    pub combat_prompt_row_opened_by_banner: bool,
    /// `view.md §3`, the death-vision tile: the accepted branch prints
    /// `Strange vision!\n` and the view it paints "is modal: the next
    /// accepted key dismisses it, is consumed by the view, and triggers the
    /// ordinary viewport redraw rather than another command". The literal's
    /// own feed opens the row that key is read on, so `text-output.md §10.4`
    /// spends no second one - the same shape as the arena banner and the arms
    /// shop welcome. True only while that modal view is up.
    pub view_overlay_row_opened_by_result: bool,
    /// `view.md §4`: idle polls the modal local view has spent waiting for
    /// its key; the party-marker box is XOR-drawn on every fourth one.
    pub view_overlay_idle_polls: u32,
    /// `karma.md §12`: the meditation's ask is `\nMantra:` - a **leading**
    /// line feed - so the label's row is still open when an empty Return
    /// closes the interaction. `text-output.md §10.4`'s next-cycle feed is
    /// what closes it, and the marker row follows on the next row rather than
    /// under a blank. One-shot: set when that Return closes the prompt,
    /// consumed by the next command echo.
    pub surface_command_row_follows_history: bool,
    /// `text-output.md §10.2`'s overworld prompt gate, stored inverted: set
    /// means the one-byte flag is **clear**, so the next overworld prompt
    /// emits neither its leading line feed nor its end-cap marker. "It is
    /// cleared by any direction taken while the sails are **hoisted**, but a
    /// frigate facing change (either sail state) and an under-sail refusal
    /// (`BREAKING UP!`, `COLLISION!`, `Docked!`) set it again within the same
    /// command ... A furled ship, like a party on foot, never touches the
    /// flag." The helper re-arms it as soon as it has tested it, and "the
    /// overworld's per-entry initialiser sets the flag **unconditionally**"
    /// (`RETRACTIONS.md` R497, amended by R526, u5-spec#297).
    pub overworld_prompt_pair_suppressed: bool,
    /// `town-mode.md §7.1`: a firing drunkenness draw prints `Hic!` and a
    /// line break on the row of the command whose key was read, and the
    /// substituted cardinal step's echo follows on the next row with no
    /// prompt pair of its own (playtest J18). Set by the hiccup, consumed by
    /// that step's [`Self::begin_command_echo`].
    pub town_hiccup_echo_continues: bool,
    /// The stats-panel text window's own, persistent inverse-video bit
    /// (`inventory.md §4.4` "The highlight can leak into the panel",
    /// u5-spec#296; `text-output.md` control bytes: the `0xFD` toggle "flips
    /// both the cache and the active window's own inverse bit ... an inverse
    /// state left set in a window survives the reset helper"). Each picker
    /// list pass flips it once per emitted toggle; it is set after a pass
    /// whose highlight sits on a two-line entry's first or continuation line.
    /// While set, every panel repaint draws inverted. "Turns, movement and
    /// messages do not clear it ... It flips back only when a later picker
    /// session leaves odd parity again."
    pub stats_panel_window_inverse: bool,
    /// `formats/saved-gam.md §6`, `0x02EE`: "Combat entry uses it to save
    /// the pre-combat scene for later restore". `None` until this process
    /// enters a fight, so the save template's byte round-trips untouched;
    /// afterwards the save writes the scene the last fight was entered from
    /// (playtest N24).
    pub saved_scene_scratch: Option<u8>,
    /// `formats/saved-gam.md §4`: "Byte-compatible readers must preserve
    /// unrecognized marker values", and `save-load.md §4.2`: the load path
    /// "normalises no restored byte". The raw `0x02D6` marker a save was
    /// loaded with, paired with the transport the engine decoded from it.
    /// While the party's transport is still that decoded value the raw byte
    /// is what slot zero and the save carry (`0x1D`, `0x00`, or an unknown
    /// value); the first transport change drops it for good (playtest N25).
    pub preserved_transport_marker: Option<(u8, TransportState)>,
    /// Narration this presentation is printing on a timer rather than all at
    /// once. `main-loop.md §9`: "presentations, cutscene beats and paced turn
    /// loops call [the world tick] directly", so a handler with waits between
    /// its records is holding the loop. See [`crate::narration`].
    pub staged_narration: crate::narration::StagedNarration,
    /// Pages of an already-composed message still waiting behind a pause
    /// or key wait (`conversation.md §7.3`, `formats/signs-dat.md §4`).
    /// See [`crate::text_hold`].
    pub pending_text_hold: Option<crate::text_hold::PendingTextHold>,
    /// Where `blackthorn.md §7`'s rescue cinematic has got to, if one is
    /// running. Its narration is staged; this is the part the queue cannot
    /// carry - the one blocking key read in the middle, and the durable
    /// handoff at the end.
    pub pending_blackthorn_rescue: Option<crate::blackthorn::BlackthornRescuePhase>,
    /// `blackthorn.md §4.2`'s audience exit beat is running, and the
    /// captive-cell handoff is owed when its hold ends.
    pub pending_blackthorn_audience_exit: bool,
    /// `karma.md §12`'s third mantra answer is in its own twelve-world-tick
    /// hold (`cleak/u5-engine#23`). Unlike the first two asks, nothing
    /// prints when the hold clears - each of the three closing branches runs
    /// its own fallible `MISCMSG` lookup, so the session moves here rather
    /// than staying in `active_shrine` while `staged_narration` carries the
    /// wait, and [`PlayState::step_shrine_third_mantra_resolution`] runs the
    /// branch once the hold drains.
    pub pending_shrine_third_mantra: Option<crate::z_stats::ShrineSession>,
    /// The interrogation ended on a *punishing* wrong-answer branch.
    ///
    /// `cleak/u5-spec#268`: "The punishing wrong-answer branches never run
    /// the beat, and that is why only they reach §6.3's conditional
    /// Blackthorn-tableau cleanup: the beat clears the tableau slot that
    /// cleanup's test reads." The correct-answer branch with two or more
    /// nondead members does run it, after the merciful-death execution.
    pub blackthorn_audience_punished: bool,
    /// Whether the last emitted line left the cursor part-way along its own
    /// row rather than opening a fresh one.
    ///
    /// `text-output.md §10.4`: a line feed "advances the row *and* returns
    /// the column", so a string that ends in one leaves the cursor at column
    /// 0 of a new row and a string that does not leaves it inline. While a
    /// held page or a staged presentation is on screen - neither of which
    /// runs a turn cycle - that distinction is the whole difference between
    /// what the two draw:
    ///
    ///   - inline: the cursor sits one cell past the text and no extra row
    ///     exists. `blackthorn.md §4.1`'s records 9 and 10 "end with a
    ///     closing quote plus exactly one trailing space", and the original
    ///     draws the barber pole after it (`bt-password/passed`).
    ///   - fresh row: the opened row is drawn, empty, and carries no cursor.
    ///     `karma.md §12`'s approach record ends with two feeds, and the
    ///     original holds two blank rows under it (`shrine-enter/early`);
    ///     `blackthorn.md §5`'s execution helper "finishes with one line
    ///     feed" and the original draws no cursor after `falls!`
    ///     (`bt-escalate/ask5`).
    ///
    /// All three were measured 2026-09-12 and disagreed with each other under
    /// any rule that did not look at the trailing byte.
    pub message_row_open_mid_line: bool,
    /// Armed by a producer that is about to write a row and leave it open;
    /// consumed by the next transcript entry, which carries it as
    /// [`MessageEntry::leaves_row_open`].
    ///
    /// One-shot, but consumed at *entry creation* rather than at the next
    /// command echo, so it cannot drift onto a later producer the way
    /// `surface_command_row_follows_history` can. `cleak/u5-engine#36`.
    pub pending_entry_leaves_row_open: bool,
    /// Keys pressed while a cinematic hold owned the loop.
    ///
    /// `cleak/u5-spec#268` on `blackthorn.md §4.2`: the beat's animation
    /// "consumes no input - the script language has no input command, the
    /// repeated pause polls no keyboard, and there is no abort path - so it
    /// cannot be skipped". Not polling the keyboard is not the same as
    /// discarding what is typed: the keys sit in the BIOS buffer and the next
    /// reader takes them. Measured 2026-09-12 (`bt-audience/ask3`), where the
    /// original acknowledges the record with the `m` of a typed `mul`, holds
    /// through the beat, and then reads the `u` as U-Use - this engine had
    /// dropped it.
    pub keys_buffered_during_hold: std::collections::VecDeque<(char, String)>,
    /// The roster panel as it was at the last refresh. See
    /// [`crate::stats_panel::StatsPanelSnapshot`] for why the panel is not
    /// painted from live state.
    pub stats_panel: crate::stats_panel::StatsPanelSnapshot,
    /// A panel body a staged presentation keeps on screen in place of
    /// [`Self::stats_panel`] while its beats run - the drowning loop's
    /// per-pass repaints (`audio.md §8.9`), which the engine applies at once
    /// but the original paints one damage pass at a time.
    pub stats_panel_hold: Option<crate::stats_panel::StatsPanelSnapshot>,
    /// What the U-Use picker's passes have painted into the panel window,
    /// residue included (`inventory.md §4.4`); kept after the picker closes
    /// for its window-column-15 cells, which the roster repaint never reaches.
    pub use_picker_panel: Option<crate::stats_panel::PickerPanelCells>,
    /// Panel bodies still owed to [`Self::stats_panel_hold`], one per staged
    /// beat that carries a repaint, in release order.
    pub staged_stats_panels: std::collections::VecDeque<crate::stats_panel::StatsPanelSnapshot>,
    /// `stats-panel.md §2.2`'s one-byte panel refresh request: "a plain
    /// boolean ... written only 'set' or 'clear', ... no second value, no
    /// bitmask, no priority and no partial-refresh encoding". Raised by a
    /// site that changed a displayed number and chose not to repaint, and
    /// consumed at one of §2.3's four command-prompt heads.
    pub stats_panel_refresh_requested: bool,
    /// How the command now running will leave the panel.
    ///
    /// `stats-panel.md §2.2` chooses per call site, and the census puts
    /// seventy-four sites on the immediate side against twenty-three
    /// deferred - so [`crate::stats_panel::PanelRefreshMode::Immediate`] is
    /// the default and only the exceptions annotate themselves. Reset at the
    /// head of each command.
    pub stats_panel_command_refresh: crate::stats_panel::PanelRefreshMode,
    /// The panel body as it stood when the current command began.
    ///
    /// A site refreshes the panel *because it changed a displayed number*.
    /// A command that changes none of them paints nothing and raises
    /// nothing - which is why, in `cleak/u5-spec#267`'s measured case, the
    /// wishing well's stale gold "survives the whole scenario and every
    /// following prompt until something unrelated repaints the panel".
    ///
    /// Comparing against this at the end of the command is what distinguishes
    /// the two, without having to classify all ~148 counter writes by hand.
    pub stats_panel_command_baseline: crate::stats_panel::StatsPanelSnapshot,
    /// `combat.md §8.2`: the live `A`-Attack attempt walk and its open
    /// targeting cursor. `A` "opens a second, separate input read, and
    /// it is not a one-shot direction key but an **interactive targeting
    /// cursor**", entered "once per readied weapon, not once per Attack
    /// command". Never serialized: combat cannot be saved mid-fight.
    /// See [`crate::CastArgumentRequest`]: what the last spell dispatch
    /// still needs, for the cast prompt's follow-up to open.
    pub pending_cast_argument: Option<CastArgumentRequest>,
    /// `magic.md §5` (issue #277): "Steps 4 to 6 precede the handler, so a
    /// handler's own prompt [...] is reached with the charge - and [...] the
    /// mana - already spent." Set while a dispatch runs whose charge, mana
    /// and level gates have already been settled, so the handler's own
    /// [`PlayState::cast_spell_resource_gate`] call does not charge twice.
    pub cast_resources_spent: bool,
    pub active_combat_targeting: Option<CombatTargetingCursorSession>,
    /// `combat.md §8.2`: the targeting cursor "starts on **the attacker's**
    /// remembered previous target when that target is still a valid, live,
    /// visible actor within the maximum range, and on the attacker's own
    /// cell otherwise." The remembered target is a property of the attacker,
    /// so it is held per actor slot rather than once for the arena. Never
    /// serialized: combat cannot be saved mid-fight (`§15`).
    pub combat_remembered_targets: [Option<u8>; COMBAT_ACTOR_SLOTS],
    pub pending_combat_terrain_trigger_slot: Option<usize>,
    /// `town-mode.md §14`: the town NPC-conflict chain's carry-over.
    /// A-Attack on a town actor enters the ordinary terrain arena, and
    /// "On exit the town chain clears the NPC slot, reloads the town
    /// map, and re-runs the Shadowlord install pass of Section 13".
    pub pending_town_conflict: Option<PendingTownConflict>,
    /// `npc-schedules.md §9.2`: the engagement event kind/index pair. "The
    /// schedule pass clears the shared event kind and NPC index", an
    /// engagement records its producer, and "the last producer visited
    /// supplies the event that town mode handles".
    pub pending_town_npc_contact: Option<TownNpcContactEvent>,
    /// High-to-low outdoor reaction slots staged by the I/O-free active-object
    /// walker. Lower entries survive a terrain-combat frame and resume when
    /// that frame returns to the world.
    pub pending_outdoor_reaction_slots: Vec<usize>,
    /// `movement.md §8.1` / `active-objects.md §8` (u5-spec#305 Q4): the rest
    /// of a difficult-terrain step whose extra object-update call opened a
    /// terrain combat. The original's fight returns to that call, so its
    /// remaining extra calls, the terrain charge, the ordinary turn and the
    /// consumed-action tail still run once the combat frame returns.
    pub pending_difficult_terrain_step: Option<PendingDifficultTerrainStep>,
    pub next_combat_actor_slot: usize,
    pub combat_terrain: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub combat_magic_effects: [[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    pub combat_cursor_blink: bool,
    /// `combat.md §7`/`§5.3` step 8: has this encounter's round-loop entry
    /// prologue already run? The prologue "runs once per entry into the
    /// round loop, and the loop is entered once per encounter: the sweep
    /// restart jumps back past the prologue" (`RETRACTIONS.md` R308).
    pub combat_round_loop_prologue_ran: bool,
    /// `combat.md §4` restore phase: "If the resident tile-restoration flag
    /// is set when the round loop returns, clear that flag and invoke the
    /// display driver's tile-graphics save/restore/mutation entry with mode
    /// value `1` before the ordinary world redraw." The setter is the
    /// dungeon room painter's two-way ladder cell (`dungeon-mode.md §14.1`),
    /// which is outside combat's contract: "combat owns only the
    /// sampling/clear/call ordering, while the setter provenance and
    /// tile-asset mutation details belong to the dungeon and driver specs."
    /// Transient presentation handoff, never serialized.
    ///
    /// `combat.md §4` now settles the call's operand set and the flag's
    /// setter census, and both confirm this model: "**The operand set is the
    /// mode value and nothing else.** The framer's wrapper writes the
    /// dispatch index into the driver call cell, places the mode value in
    /// the driver's single argument register, and makes the call. It sets no
    /// other register, and it passes no coordinate, no rectangle, no handle
    /// and no pointer to a saved pair ... An implementation with no driver
    /// handle and no saved pair to pass is not missing an operand - there is
    /// none to pass." The setter is "the dungeon room painter's
    /// two-way-ladder cell", which "issues the matching **save** in the same
    /// breath, through the same driver entry with mode value `0`" - so the
    /// pair is a driver-private save-and-substitute, `display-driver.md`.
    ///
    /// "**No Klimb ordering constraint**" (`combat.md §4`, `RETRACTIONS.md`
    /// R471): the K-Klimb handler does not read this flag.
    pub tile_restoration_pending: bool,
    /// Driver tile-graphics saves (`0x6C` mode `0`) the dungeon room
    /// painter's two-way-ladder cell issued and owes a frontend, in the same
    /// drained-request model as the restores below; a frontend issues
    /// [`crate::EgaDisplayOperation::SaveLoadedTileGraphics`] for each one.
    pub pending_driver_tile_graphics_saves: usize,
    /// Driver tile-graphics restores the combat framer sampled out of
    /// [`Self::tile_restoration_pending`] and owes a frontend. The runtime
    /// has no display driver of its own, so it records the request in the
    /// published order - ahead of the restore phase's world redraw - and a
    /// frontend drains it by issuing
    /// [`crate::EgaDisplayOperation::RestoreLoadedTileGraphics`].
    pub pending_driver_tile_graphics_restores: usize,
    /// `combat.md §7`, "The middle tier's flag is a stats-panel refresh
    /// request, not a leave-combat flag" (`RETRACTIONS.md` R358).
    ///
    /// "It is a one-bit 'the party stats panel is stale' request with a
    /// single consumer shape: each of the four world/combat mode loops -
    /// combat, dungeon, outdoor and town - reads it once at the top of its
    /// per-turn entry point and, if it is set, redraws the full party stats
    /// panel and clears it. That is the whole contract ... The request is
    /// raised in twenty-three places across the game, every one of them a
    /// routine that has just changed a number the panel shows - spell mana,
    /// food, gold, HP from a hazard, or the active-player marker - so an
    /// engine should model it as a shared display latch owned by the stats
    /// panel, not as combat state and certainly not as a contact-record
    /// field."
    ///
    /// Transient display state, never serialized.
    pub party_stats_panel_refresh_pending: bool,
    /// `combat.md §7`, "What owns that coordinate": the aim marker's arena
    /// X/Y pair. The `§8.2` targeting cursor is its only writer - it
    /// "**seeds** them when it opens", "**rewrites** them on every accepted
    /// move", leaves them alone on a rejected move, and "**The coordinate
    /// itself is never cleared**".
    ///
    /// `None` is *unwritten*, not a cell: `§7` contemplates exactly that
    /// state ("an engine that leaves the coordinate unwritten draws no aim
    /// marker at all") and publishes no initial value, so the engine does
    /// not invent one. Modelling the unwritten state as the literal `(0, 0)`
    /// would be an invented seed - `(0, 0)` is a real arena cell that passes
    /// [`PlayState::combat_field_cursor_cell_in_range`] for every live
    /// caster - and would make `§8.2`'s published "attacker's own cell
    /// otherwise" fallback unreachable until a cursor had opened once.
    pub combat_aim_marker_cell: Option<(u8, u8)>,
    /// `combat.md §7`, "What owns that coordinate": the marker's separate
    /// **gate** byte. "The cursor raises it as its first act and lowers it
    /// as its last, so the marker is on screen **if and only if an arena
    /// targeting cursor is currently open** ... The round loop additionally
    /// clears it as part of the per-action scratch reset before each acting
    /// slot, which is belt and braces rather than a second mechanism."
    pub combat_aim_marker_gate: bool,
    pub combat_ambush_reveals: [Option<CombatAmbushRevealRecord>; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
    pub combat_actors: [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    /// `weather.md §5`'s **sailing counter**: the count of wait passes the
    /// hoisted-sail ship has already paid against the wind/heading table's
    /// threshold. Cleared by a heading change, by the release, and by a wind
    /// change (`§5.1`, "**A fourth reset of the sailing counter.**").
    ///
    /// There is deliberately no second `sail_stall_pending`-style flag beside
    /// it. `weather.md §6` says the sailing refusal state is "short-lived" and
    /// that "The next pass command can report that the ship is stalled by the
    /// wind, then clears that state", and `§5.1`'s third clear states the Pass
    /// condition exactly: "Only while the outdoor scene is current and the
    /// cache is non-zero; it prints the stalled-sailing line first, then
    /// clears." The cache **is** the state Pass reports on, so an extra flag
    /// would be a second source of truth with no reader.
    pub sail_cadence: u8,
    /// `weather.md §5`/`§5.1` and `overworld.md §5` step 3: the hoisted-sail
    /// ship's **cached sail direction**. "Once the requested direction matches
    /// the cached heading, the input helper can synthesize repeated movement
    /// commands from the cached direction until the cache is cleared or
    /// replaced", and at the loop's input step, "Under sail on the wind-driven
    /// cadence, this step does not enter the command wait: the input helper
    /// runs its own auto-advance loop and returns the cached sail direction"
    /// (`RETRACTIONS.md` R372 withdrew the older "does not read the keyboard
    /// at all"; the loop polls one key per pass and swallows the
    /// same-direction one).
    ///
    /// `weather.md §5.1` publishes the lifecycle as **one setter and four
    /// clears**, and the rule is marker-conditioned rather than a command
    /// list. The setter is a movement command taken while the marker is a
    /// hoisted frigate, and it runs only when the heading differs. The clears
    /// are: entering outdoor mode; the under-sail step refusal (one decision
    /// point with three narration arms - breaking up, docking, collision); the
    /// Pass command; and the post-command marker guard, which clears the cache
    /// after every outdoor command handler unless the marker is still a
    /// hoisted frigate. That last one is what covers Furl, board, X-it,
    /// mounting a horse or a carpet - "An engine that implements per-command
    /// clears will agree with the original on the commands it thought of and
    /// disagree on the ones it did not."
    ///
    /// `None` means there is nothing to synthesize, so the loop reads a
    /// command as usual. Never serialized.
    pub sail_cached_direction: Option<Direction>,
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
    /// `text-output.md §3`: clearing a text window blanks its rectangle
    /// without moving the cursor, so output after a clear resumes on the
    /// row it had reached. This is that row, as an offset from the
    /// window's top. `cleak/u5-engine#11`.
    pub(crate) message_window_top_offset: u8,
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
    /// `endgame.md §3` step 2's full redraw, owed until the first entry
    /// frame is pumped. See `advance_endgame_entry_presentation`.
    pub endgame_entry_redraw_pending: bool,
    /// `LOOK2.DAT` descriptions, cached at scene load.
    ///
    /// Look loads this per call from its own `game_dir`. Talk needs the
    /// same descriptions - `conversation.md §2` step 5's non-speaker
    /// refusal is composed from the NPC's Look description, measured as
    /// `The guard offers no response!` against `Thou dost see / a guard`
    /// (`cleak/u5-spec#198`) - but the Talk path carries no `game_dir`,
    /// so the table is cached here at scene load instead of threaded
    /// through every Talk signature. `None` when the scene was built
    /// without assets, as most unit-test fixtures are.
    pub look_table: Option<crate::LookTable>,
    /// Harness-only diagnostics: NPC slot indices, sprite type bytes,
    /// cell coordinates, alarm-sweep counts.
    ///
    /// These used to be written straight into [`Self::message`], which
    /// put internal identifiers in front of the player and, worse, hid
    /// whatever the original prints on those rows - a paired capture
    /// could not measure the real narration because ours had already
    /// overwritten it (`cleak/u5-engine#8`). They live here instead, so
    /// the acceptance suites can still assert on them while the message
    /// window carries only published text.
    pub diagnostics: Vec<String>,
    /// How many diagnostics [`Self::push_diagnostic`] has ever recorded.
    ///
    /// `diagnostics` is a bounded window that drops its oldest entry, so its
    /// length cannot tell a frontend which entries are new. A frontend that
    /// forwards diagnostics to its own log keeps the serial it last saw and
    /// asks [`Self::diagnostics_since`] for the rest.
    pub diagnostics_serial: u64,
    /// The final room as it stood when the Avatar was absorbed, with the
    /// absorbed actor gone, held for display through the measured
    /// absorption hold ([`crate::ENDGAME_ABSORPTION_HOLD_BIOS_TICKS`]).
    ///
    /// `endgame.md §3` rebuilds the table as the tableau at entry, but the
    /// screen does not show it until the setup pass draws: the original
    /// keeps the arena on screen, Avatar gone, for the whole hold
    /// (`doom-final-room` timelapses). The live state has already left the
    /// arena by then, so the frame it shows is kept here and released when
    /// the tableau setup's first frame is pumped. Display state only.
    pub endgame_absorption_arena: Option<Box<PlayState>>,
    pub active_blackthorn: Option<crate::blackthorn_session::BlackthornChallenge>,
    pub blackthorn_audience_map: Option<MiscmapsCutsceneMap>,
    /// The two viewport ribbon gaps (top, bottom) as they stood when the
    /// current fight's arena was framed. Display state only.
    ///
    /// `moons.md §2.2`/`§3` and `weather.md §2.1` keep combat outside the
    /// sky-strip and wind-banner renderers, so nothing repaints either gap
    /// during a fight. Stock captures show what that leaves: the pre-combat
    /// strip and banner, or a dungeon room's level and facing labels, stay
    /// on screen for the whole fight (`combat-town-attack`,
    /// `combat-escape-ordinary`, `combat-dungeon-room`, `overworld-night-walk`
    /// stock frames, 2026-10-25) - the "skips rather than erases" case, the
    /// same as `dungeon-mode.md §13`'s labels outliving the mode that drew
    /// them. Frozen here at combat entry; not live. `moons.md §3` now
    /// publishes it (u5-spec#300 Q35, `RETRACTIONS.md` R619).
    pub combat_entry_chrome: Option<(ChromeGap, ChromeGap)>,
    /// `moons.md §3` (u5-spec#300 Q35, R619): the surface-overworld gaps
    /// left frozen after a fight; see [`crate::PostCombatRibbonHold`].
    pub post_combat_ribbon_hold: Option<PostCombatRibbonHold>,
    /// `commands.md §3`: "`A` overworld | Always \"no action\", including
    /// the successful attack that enters combat", and `combat.md §4`: the
    /// exit's clock call has "a **zero** increment ... so the only time a
    /// fight adds is its round-wrap minutes". Set when an overworld A-Attack
    /// opens the arena, so the round loop's exit owes no outdoor turn for it
    /// (p10 R11); consumed by that exit.
    pub combat_exit_skips_world_turn: bool,
    /// `karma.md §7` "Entry pacing": the shrine presentation's own display.
    ///
    /// Meditation does not happen on the overworld map. The presentation
    /// "copies the whole active-object table aside and clears every slot's
    /// **type** byte", so "the party and every nearby actor are invisible
    /// while their records survive intact", raises the scene byte to its
    /// suspended sentinel, and loads "the shrine's eleven-by-eleven display
    /// grid - record `1` of `MISCMAPS.DAT`". This engine used to leave the
    /// overworld on screen for the whole meditation; measured 2026-09-13 with
    /// `qa/tools/viewport_audit.py`, that was 35% of the viewport's pixels
    /// across four beats of `qa/paired/shrine-three-mantras.tsv`, and the
    /// paired harness never saw it because it compared only text.
    /// `town-mode.md §7.1` step 1's direct black viewport fill, held for as
    /// long as the original holds it: from the Stonegate trapdoor script
    /// until the rescue hands off. `blackthorn.md §7` step 3 dissolves to the
    /// same black and step 9 draws the party on it.
    pub cutscene_black_viewport: bool,
    /// `blackthorn.md §3`'s capture narration while its timeline is staged
    /// (issue #304 Q6); `None` outside it.
    pub blackthorn_capture: Option<crate::blackthorn::BlackthornCaptureNarration>,
    /// How many of `blackthorn.md §7` step 4's three cell reveals are on
    /// screen: the two Guardians, then the spectral figure.
    pub blackthorn_rescue_reveals_shown: u8,
    /// The rescue's crowned spectral figure (`blackthorn.md §7.1` step 5:
    /// actor byte `0x74` at `(5,2)`) as an animated record, live from the
    /// verdict's key wait. `endgame.md §4` (issue #298 Q43): it selects
    /// script A and "likewise animates during its verdict key wait". Empty
    /// while no animated figure is on screen.
    pub blackthorn_rescue_spectral_actor: ActiveObject,
    /// The BIOS tick the rescue figure's animator last ran on; `None` until
    /// the first look anchors it.
    pub blackthorn_rescue_animation_tick: Option<u64>,
    /// The last `visibility.md §8` variant drawn for each audience actor slot
    /// (`0..=8`) standing on a selecting terrain row - the cutscene's party
    /// actors start on the manacles `0x85` and are drawn as the prisoner set.
    pub blackthorn_audience_actor_variants: [u8; 9],
    /// The BIOS tick the audience's composite pass last ran on.
    pub blackthorn_audience_composite_tick: Option<u64>,
    pub shrine_presentation_map: Option<MiscmapsCutsceneMap>,
    /// `karma.md §7` "Entry pacing": "The only route from a world step to the
    /// text printer is the ambient wind-and-location row, and that row
    /// returns immediately while the scene byte is at or above the
    /// ordinary-scene threshold - which the presentation's suspended sentinel
    /// is ... no row draw ... no panel repaint occurs anywhere in the entry".
    /// The wind drift may still store a new wind (`weather.md §2.1`), but the
    /// ribbon gaps keep what the scene drew at entry until the presentation
    /// ends. Captured when a shrine or Codex presentation opens (p10 R06).
    pub presentation_entry_chrome: Option<(ChromeGap, ChromeGap)>,
    /// `karma.md §7`'s approach walk, while the shrine presentation is up.
    pub shrine_approach_walk: Option<crate::shrine_presentation::ShrineApproachWalk>,
    /// `karma.md §7.2`: the offering and Codex turn-in arms XOR-invert the
    /// whole gameplay viewport once and never restore it; the counted steps
    /// after the swell repaint over it. Set from the inversion until the
    /// swell's hold releases.
    pub presentation_viewport_inverted: bool,
    /// `rest-and-camp.md §7.1` step 4 (u5-spec#299 Q5): a camp-apparition
    /// flash is on screen - the whole gameplay viewport XOR-inverted in the
    /// accent pen (UI colour slot 1; palette index 15 on the sixteen-colour
    /// drivers) while that living member's long sting sounds
    /// ([`crate::audio::SoundEffect::holds_viewport_inversion`]). Set and
    /// cleared by the frontend that owns the speaker voice, since only it
    /// knows when the envelope ends; the world-tick pause after the sting
    /// repaints the viewport, which is when it clears.
    pub presentation_camp_flash_inverted: bool,
    pub active_shop: Option<crate::shop_session::ActiveShopSession>,
    /// `shops.md §8.1`: the arms post-item prompt closes with `sir?`/`milady?`
    /// once a transaction has completed in this visit and with `then?` before
    /// then, and "a decline or carry-cap refusal uses this same prompt,
    /// retaining whether an earlier purchase completed". That is a property of
    /// the visit rather than of the menu state, so it lives beside the session
    /// and is cleared whenever a new shop session opens.
    pub arms_transaction_completed: bool,
    pub common_word_dictionary: Option<crate::common_words_io::CommonWordDictionary>,
    pub active_conversation: Option<Box<crate::conversation_session::ConversationSession>>,
    /// NPC roster slot captured when the active conversation opens. The
    /// opening acquaintance test addresses this slot in the scene's TALK bitset.
    pub active_conversation_npc_slot: Option<usize>,
    /// `shops.md §2` "Contact and turn accounting": set while the shared
    /// Talk dispatcher runs for an automatic conversation-contact event,
    /// which "does not create an additional Talk command or schedule tick".
    pub talk_entered_by_contact: bool,
    /// `town-mode.md §7` step 4 / `shops.md §2`: an explicit Talk that
    /// opened a shop owes its turn epilogue - clock minute, underfoot and
    /// status pass, both walkers - to the moment the shop arm returns, not to
    /// the moment it opens (playtest J16). Set when the shop opens; the
    /// shop-key dispatcher spends it when the session closes.
    pub pending_talk_shop_turn: bool,
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
    /// `inventory.md §5` timing contract and `commands.md §4`: outside
    /// combat R-Ready and Z-stats report the default "acted" status, which
    /// the mode loop consumes *after* the handler returns - "Once in the
    /// post-action mode epilogue". The charge is owed from the moment the
    /// command is dispatched and is paid when its last prompt closes, so no
    /// world turn (clock, active objects, encounters) runs while `Player: `
    /// or the picker is still waiting (playtest F23).
    pub exploration_acted_charge_owed: bool,
    /// `commands.md §3`: a dungeon letter that neither forwards its
    /// handler's value nor is a published "no action" producer reports the
    /// default "acted", and the dungeon loop runs its post-action pass on any
    /// non-zero status (`dungeon-mode.md §4` step 6). The action counter
    /// before such a letter was dispatched, held until its last prompt has
    /// closed; the command epilogue then charges the turn if the handler did
    /// not (playtest J08).
    pub dungeon_acted_turn_before: Option<u64>,
    pub active_use: Option<crate::z_stats::UseSession>,
    pub active_cast: Option<crate::z_stats::CastSession>,
    pub active_cast_followup: Option<crate::z_stats::CastFollowupSession>,
    pub active_rest: Option<crate::z_stats::RestSession>,
    pub active_jimmy: Option<crate::z_stats::JimmySession>,
    pub active_surface_chest: Option<crate::z_stats::SurfaceChestSession>,
    pub active_shrine: Option<crate::z_stats::ShrineSession>,
    /// The open Codex reader's key position.
    ///
    /// `karma.md §8`'s presentation is not instant. Measured 2026-09-09
    /// (`qa/paired/codex-enter.tsv`): after E-Enter the original draws its
    /// first record and then waits - sixteen seconds with no key advanced it
    /// no further - and the next three keys step it through the remaining
    /// records without reaching the command parser.
    ///
    /// `karma.md §8.2`: the number of the advance key the open reader waits
    /// for next (`1..=9` in that section's table), or `0` when no reader is
    /// open. Each key prints its own record; nothing is printed ahead.
    pub codex_presentation_steps: u8,
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

/// What a difficult-terrain step still owes after a terrain combat opened
/// inside one of its extra object-update calls; see
/// [`PlayState::pending_difficult_terrain_step`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PendingDifficultTerrainStep {
    pub plane: WorldPlane,
    pub step: WorldDifficultTerrainStep,
    /// Extra calls not yet started. The interrupted call's own proximity
    /// pause is still owed ahead of them.
    pub remaining_calls: u8,
    pub transition: Option<WorldPlaneTransitionEntry>,
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

// `view.md §4`, corrected 2026-10-12 (`cleak/u5-spec#283`; `RETRACTIONS.md`
// R517): the local-view overlay classes `0xA`/`0xB`/`0xC`/`0xD`/`0xF` do
// **not** pick an alternate pen in gem/peer-spell mode. The modal arm reads
// the display-adapter identifier, the same word `view.md §6.3` and
// `dungeon-mode.md §12.4` already withdrew the matching dungeon "peer tint
// branch" reading for - and on EGA/Tandy, the v1 target, it is constant:
// View, Peer, X-Ray and Look all draw the same colours. The surface-only
// `uses_alternate_surface_view_bank` method this impl block used to carry
// is withdrawn along with that reading.

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
    pub grid: Vec<u8>,
    pub active_objects: Vec<ActiveObject>,
    pub pending_vehicle: Option<PendingVehicleAcquisition>,
}

/// `magic.md §8` (issue #231 captures, 2026-10-16): "while the spell's
/// whole-view flash effect plays, the message window holds only the cast,
/// spell-name and spell rows; then `Success!` and a newline print, followed
/// by the next actor's prompt". The flash is `audio.md §6`'s shared variant
/// sequence - the viewport inversion around the envelope pair - and it is
/// synchronous, so nothing a cast prints after it can be on screen before it
/// ends.
///
/// The cast's handler still composes its whole result in one call, so rather
/// than restaging every spell's text, the hold freezes what the window *shows*:
/// the transcript (and any unflushed slot text) as it stood when the flash
/// started, until a staged pure hold of the flash's own length releases. The
/// staged hold is also what makes the loop buffer keys for that time, as the
/// original's blocking flash does. One mechanism for every spell whose result
/// follows its flash.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageWindowFlashHold {
    /// A C-Cast dispatch is running, so a shared variant is a spell's flash
    /// (scrolls and potions reach the same sound through their own paths).
    pub(crate) casting: bool,
    /// The transcript as the window showed it when the flash began.
    pub(crate) entries: Option<Vec<MessageEntry>>,
    /// The slot text the window was showing on top of `entries`, if any.
    pub(crate) slot: Option<String>,
    /// Staged flash holds not yet released; the snapshot is dropped when the
    /// last one releases.
    pub(crate) pending_beats: u16,
}

impl MessageWindowFlashHold {
    pub fn is_active(&self) -> bool {
        self.entries.is_some()
    }
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
    /// Set when this entry is a blank row a *producer* wrote, from a line
    /// feed embedded in its own text, rather than the blank that closes a
    /// command turn. See [`crate::MessageLineKind::ProducerBlank`].
    pub producer_blank: bool,
    /// Fixed-font choice for each byte in `text`. Plain engine messages use
    /// the ordinary font; TLK `0x8E` spans retain their runic selection here.
    pub glyphs: Vec<TlkRenderedGlyph>,
    pub is_command_echo: bool,
    /// Whether this echo was written onto a marker row some earlier print's
    /// line feed had already opened.
    ///
    /// `text-output.md §10.2` runs the world loops' echo cycle as "1. Emit a
    /// line feed into the message window. 2. Draw the right-pointing bracket
    /// end-cap ... 3. Read the key", and §10.4 derives the blank row between
    /// command turns from step 1. `combat.md §8.1` moves that line feed:
    /// the arena's turn handler "emits the line feed itself, unconditionally,
    /// between printing the banner and reading the command byte", so the
    /// marker is drawn on the row the banner's own newline opened and no
    /// second line feed - and therefore no derived blank row - precedes it.
    pub continues_open_row: bool,
    /// Whether this echo's prompt emitted neither the leading line feed nor
    /// the end-cap marker: `text-output.md §10.2`'s overworld gate after a
    /// direction taken aboard a ship. The echo starts in column 24 on the row
    /// directly beneath the history, with no derived blank above it.
    pub prompt_pair_suppressed: bool,
    /// Whether this entry leaves its row open: written without a closing
    /// line feed, so the next cycle's leading feed closes the row rather
    /// than deriving `text-output.md §10.4`'s blank under it.
    ///
    /// Carried to [`crate::MessageLogLine::row_left_open`], which the layout
    /// reads from the **last** line only - so an entry appended afterwards
    /// ends the suppression by construction, with no flag to go stale.
    /// `cleak/u5-engine#36`.
    pub leaves_row_open: bool,
    /// Center this output line in the sixteen-cell message window. Cursor
    /// centering is presentation state, not ASCII padding in `text`.
    pub centered: bool,
    /// Preserve this empty output row instead of treating it as an empty slot.
    pub explicit_blank: bool,
    /// The open row a strip scroll left under a U-Use `Item: ` prompt: the
    /// message cursor stays at the prompt's column while the painted rows
    /// rise (`cleak/u5-engine#25`, `display-driver-abi.md §9.5`), so the
    /// row starts with that many spaces and the completion is appended to
    /// it. While it is still only spaces it must survive renderers that
    /// drop blank-text entries, because the cursor is drawn on it.
    pub prompt_column_row: bool,
    /// The marker row an overworld prompt pair drew before a shipboard
    /// direction that echoes nothing: a hoisted key that keeps the facing,
    /// or a synthesised sailing step the input helper released.
    ///
    /// `text-output.md §10.2`: "Each mode's shared input helper emits the
    /// pair before waiting for a key", and `vehicles.md §6` (u5-spec#297):
    /// "A key that keeps the facing prints nothing under hoisted sail". So
    /// the row holds the end cap and the cursor in column 25, and whatever
    /// the step prints next (`BREAKING UP!`, `COLLISION!`) is written on it.
    /// Measured 2026-10-16 (`qa/paired/sail-calm.tsv`, beat `heading`): the
    /// stock game reads `>Yell HOIST!` / blank / `>BREAKING UP!`.
    pub bare_marker: bool,
    /// This output starts a row of its own even after a
    /// [`Self::bare_marker`] row, rather than being written on it.
    ///
    /// The bare marker row takes the step's own refusal (`BREAKING UP!`),
    /// but not an encounter the step ran into. Measured 2026-10-16
    /// (`qa/paired/sail-heading-echo.tsv`, beat `sailing-more`, stock runs
    /// `204248` and `222947`): under hoisted sail the original reads
    /// `Hull weak!` / blank / the bare `>` marker row / `Attacked!`, the
    /// encounter line on the row below the marker.
    pub starts_fresh_row: bool,
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

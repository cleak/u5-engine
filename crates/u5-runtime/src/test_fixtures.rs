//! Synthetic fixtures used by both u5-runtime's internal tests and
//! u5-tui's integration tests. Public API so it can be reached from
//! other crates' test targets.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::*;

pub fn open_grid() -> Vec<u8> {
    vec![16; 1024]
}

pub fn open_dungeon_record() -> Vec<u8> {
    vec![0; DUNGEON_RECORD_LEN]
}

pub fn open_world_grid() -> Vec<u8> {
    vec![5; WORLD_CELLS]
}

pub fn synthetic_tile_atlas(depth: TileGraphicsDepth) -> TileAtlas {
    let pixel_limit = depth.pixel_limit();
    let mut pixels = Vec::with_capacity(TILE_ATLAS_PIXEL_LEN);
    for tile in 0..TILE_ATLAS_TILE_COUNT {
        pixels.extend(std::iter::repeat((tile as u8) % pixel_limit).take(TILE_ATLAS_TILE_PIXELS));
    }
    TileAtlas {
        depth,
        pixels,
        dungeon_billboards: None,
        dungeon_sprites: None,
    }
}

pub fn test_state(grid: Vec<u8>, x: usize, y: usize) -> PlayState {
    let scene = Scene::new(0x11).unwrap();
    let mut state = PlayState {
        area: Area::Town { scene, floor: 0 },
        player: Player {
            x,
            y,
            facing: Direction::South,
            transport: TransportState::Foot,
        },
        active_objects: vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: 0,
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        npcs: Vec::new(),
        door_tracker: None,
        door_tracker_closed: false,
        opened_town_doors: Vec::new(),
        revealed_town_secret_doors: Vec::new(),
        passability: None,
        grid,
        world_live_chunks: None,
        clock: GameClock::default(),
        status_pass_previous_hour: GameClock::default().hour,
        cleanup_previous_hour: GameClock::default().hour,
        dungeon_loop_minute_charged: false,
        prng_state: DEFAULT_PRNG_STATE,
        animation: AnimationClock::default(),
        water_scroll: WaterScrollClock::default(),
        fire_flicker: FireFlickerClock::default(),
        dungeon_fountain_frame: 0,
        natural_moongate_counter: 0,
        last_natural_moongate_transit: None,
        pending_map_viewport_dissolves: Vec::new(),
        pending_blackthorn_rescue_playbacks: Vec::new(),
        pending_combat_terrain_reveals: Vec::new(),
        pending_potion_flash: None,
        pending_stonegate_trapdoor_playback: None,
        pending_town_status_provision_pass: false,
        pending_town_npc_schedule_pass: false,
        pending_town_active_object_pass: false,
        natural_moongate_live_cells: Vec::new(),
        cached_moon_glyph_bytes: cached_moon_glyph_bytes_for_day(PLAY_START_DAY)
            .unwrap_or(MOON_GLYPH_CACHE_NO_GATE),
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
        moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
        shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
        resident_shadowlord: None,
        summoned_shadowlord: None,
        removed_town_npc_flags: HashMap::new(),
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
        moral_standing: 0,
        town_drunkenness_counter: 0,
        tavern_secondary_drink_count: 0,
        toll_progress: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        ambient_light: FULL_DAYLIGHT,
        light_beacon: LightBeaconState::new(),
        beacon_bearing_stencils: synthetic_beacon_bearing_stencils(),
        local_light_mask: [false; TOWN_GRID_BYTES],
        visibility_dirty: false,
        visibility_grid: [0; VISIBILITY_GRID_LEN],
        terrain_band: [0; TERRAIN_BAND_LEN],
        visibility_buffers_ready: false,
        world_underfoot_blackout_latched: false,
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
        combat_action_result: 0,
        combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
        combat_active: false,
        pace_combat_presentations: false,
        combat_frame_snapshot: None,
        pending_combat_actor_slot: None,
        pending_combat_terrain_trigger_slot: None,
        pending_town_conflict: None,
        pending_outdoor_reaction_slots: Vec::new(),
        next_combat_actor_slot: 0,
        combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
        combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        combat_cursor_blink: false,
        combat_secondary_marker: None,
        combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
        combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        sail_cadence: 0,
        sail_stall_pending: false,
        pending_vehicle_save: PendingVehicleSaveState::default(),
        turn: 0,
        message: String::new(),
        message_transcript: Vec::new(),
        message_transcript_revision: 0,
        message_flushed: String::new(),
        pending_command_echo: None,
        pending_hourly_status_message: None,
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        music_enabled: true,
        sound_effect_serial: 0,
        sound_effect_history: Vec::new(),
        harpsichord_progress: 0,
        active_blackthorn_guard_demand: None,
        pending_town_arrest: None,
        endgame: None,
        active_blackthorn: None,
        blackthorn_audience_map: None,
        active_shop: None,
        common_word_dictionary: None,
        active_conversation: None,
        active_conversation_npc_slot: None,
        active_conversation_join_candidate: None,
        active_z_stats: None,
        active_party_selector: None,
        active_ready: None,
        active_use: None,
        active_cast: None,
        active_cast_followup: None,
        active_rest: None,
        active_jimmy: None,
        active_surface_chest: None,
        active_shrine: None,
        active_mix: None,
        active_new_order: None,
        active_yell: None,
        active_shrine_restoration: None,
        active_wishing_well: None,
        active_view_overlay: None,
        white_potion_sweep: None,
        active_direction_prompt: None,
        active_yes_no_prompt: None,
        town_npc_mutations: Vec::new(),
        talk_branch_flags: HashMap::new(),
        conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
        inn_registry: Vec::new(),
    };
    state.rebuild_surface_local_light_mask();
    state.visibility_dirty = false;
    state
}

pub fn dungeon_state(grid: Vec<u8>, level: u8, x: usize, y: usize) -> PlayState {
    let scene = DungeonScene::new(33).unwrap();
    let mut state = PlayState {
        area: Area::Dungeon { scene, level },
        player: Player {
            x,
            y,
            facing: Direction::East,
            transport: TransportState::Foot,
        },
        active_objects: vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: level as i8,
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        npcs: Vec::new(),
        door_tracker: None,
        door_tracker_closed: false,
        opened_town_doors: Vec::new(),
        revealed_town_secret_doors: Vec::new(),
        passability: None,
        grid,
        world_live_chunks: None,
        clock: GameClock::default(),
        status_pass_previous_hour: GameClock::default().hour,
        cleanup_previous_hour: GameClock::default().hour,
        dungeon_loop_minute_charged: false,
        prng_state: DEFAULT_PRNG_STATE,
        animation: AnimationClock::default(),
        water_scroll: WaterScrollClock::default(),
        fire_flicker: FireFlickerClock::default(),
        dungeon_fountain_frame: 0,
        natural_moongate_counter: 0,
        last_natural_moongate_transit: None,
        pending_map_viewport_dissolves: Vec::new(),
        pending_blackthorn_rescue_playbacks: Vec::new(),
        pending_combat_terrain_reveals: Vec::new(),
        pending_potion_flash: None,
        pending_stonegate_trapdoor_playback: None,
        pending_town_status_provision_pass: false,
        pending_town_npc_schedule_pass: false,
        pending_town_active_object_pass: false,
        natural_moongate_live_cells: Vec::new(),
        cached_moon_glyph_bytes: cached_moon_glyph_bytes_for_day(PLAY_START_DAY)
            .unwrap_or(MOON_GLYPH_CACHE_NO_GATE),
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
        moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
        shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
        resident_shadowlord: None,
        summoned_shadowlord: None,
        removed_town_npc_flags: HashMap::new(),
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
        moral_standing: 0,
        town_drunkenness_counter: 0,
        tavern_secondary_drink_count: 0,
        toll_progress: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        ambient_light: 0,
        light_beacon: LightBeaconState::new(),
        beacon_bearing_stencils: synthetic_beacon_bearing_stencils(),
        local_light_mask: [false; TOWN_GRID_BYTES],
        visibility_dirty: false,
        visibility_grid: [0; VISIBILITY_GRID_LEN],
        terrain_band: [0; TERRAIN_BAND_LEN],
        visibility_buffers_ready: false,
        world_underfoot_blackout_latched: false,
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
        combat_action_result: 0,
        combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
        combat_active: false,
        pace_combat_presentations: false,
        combat_frame_snapshot: None,
        pending_combat_actor_slot: None,
        pending_combat_terrain_trigger_slot: None,
        pending_town_conflict: None,
        pending_outdoor_reaction_slots: Vec::new(),
        next_combat_actor_slot: 0,
        combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
        combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        combat_cursor_blink: false,
        combat_secondary_marker: None,
        combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
        combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        sail_cadence: 0,
        sail_stall_pending: false,
        pending_vehicle_save: PendingVehicleSaveState::default(),
        turn: 0,
        message: String::new(),
        message_transcript: Vec::new(),
        message_transcript_revision: 0,
        message_flushed: String::new(),
        pending_command_echo: None,
        pending_hourly_status_message: None,
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        music_enabled: true,
        sound_effect_serial: 0,
        sound_effect_history: Vec::new(),
        harpsichord_progress: 0,
        active_blackthorn_guard_demand: None,
        pending_town_arrest: None,
        endgame: None,
        active_blackthorn: None,
        blackthorn_audience_map: None,
        active_shop: None,
        common_word_dictionary: None,
        active_conversation: None,
        active_conversation_npc_slot: None,
        active_conversation_join_candidate: None,
        active_z_stats: None,
        active_party_selector: None,
        active_ready: None,
        active_use: None,
        active_cast: None,
        active_cast_followup: None,
        active_rest: None,
        active_jimmy: None,
        active_surface_chest: None,
        active_shrine: None,
        active_mix: None,
        active_new_order: None,
        active_yell: None,
        active_shrine_restoration: None,
        active_wishing_well: None,
        active_view_overlay: None,
        white_potion_sweep: None,
        active_direction_prompt: None,
        active_yes_no_prompt: None,
        town_npc_mutations: Vec::new(),
        talk_branch_flags: HashMap::new(),
        conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
        inn_registry: Vec::new(),
    };
    state.rebuild_surface_local_light_mask();
    state.visibility_dirty = false;
    state
}

pub fn world_state(grid: Vec<u8>, x: usize, y: usize) -> PlayState {
    let world_live_chunks =
        WorldLiveChunkBuffer::from_full_grid(WorldPlane::Underworld, &grid, x, y, |_| {
            LiveChunkSubstitutionPolicy::NONE
        })
        .ok();
    let mut state = PlayState {
        area: Area::World {
            plane: WorldPlane::Underworld,
        },
        player: Player {
            x,
            y,
            facing: Direction::South,
            transport: TransportState::Foot,
        },
        active_objects: vec![ActiveObject {
            type_byte: PLAYER_TILE,
            tile: PLAYER_TILE,
            x,
            y,
            z: WorldPlane::Underworld.save_floor(),
            phase: PLAYER_ACTIVE_OBJECT_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        npcs: Vec::new(),
        door_tracker: None,
        door_tracker_closed: false,
        opened_town_doors: Vec::new(),
        revealed_town_secret_doors: Vec::new(),
        passability: None,
        grid,
        world_live_chunks,
        clock: GameClock::default(),
        status_pass_previous_hour: GameClock::default().hour,
        cleanup_previous_hour: GameClock::default().hour,
        dungeon_loop_minute_charged: false,
        prng_state: DEFAULT_PRNG_STATE,
        animation: AnimationClock::default(),
        water_scroll: WaterScrollClock::default(),
        fire_flicker: FireFlickerClock::default(),
        dungeon_fountain_frame: 0,
        natural_moongate_counter: 0,
        last_natural_moongate_transit: None,
        pending_map_viewport_dissolves: Vec::new(),
        pending_blackthorn_rescue_playbacks: Vec::new(),
        pending_combat_terrain_reveals: Vec::new(),
        pending_potion_flash: None,
        pending_stonegate_trapdoor_playback: None,
        pending_town_status_provision_pass: false,
        pending_town_npc_schedule_pass: false,
        pending_town_active_object_pass: false,
        natural_moongate_live_cells: Vec::new(),
        cached_moon_glyph_bytes: cached_moon_glyph_bytes_for_day(PLAY_START_DAY)
            .unwrap_or(MOON_GLYPH_CACHE_NO_GATE),
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
        moonstone_slots: [MoonstoneGateSlot::invalid(); MOONSTONE_SLOT_COUNT],
        shadowlord_hideouts: DEFAULT_SHADOWLORD_HIDEOUTS,
        resident_shadowlord: None,
        summoned_shadowlord: None,
        removed_town_npc_flags: HashMap::new(),
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        word_of_power_seal_flags: [0; SAVE_WORD_OF_POWER_SEAL_FLAG_COUNT],
        shrine_ruin_flags: [0; SAVE_SHRINE_RUIN_FLAG_COUNT],
        moral_standing: 0,
        town_drunkenness_counter: 0,
        tavern_secondary_drink_count: 0,
        toll_progress: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        ambient_light: FULL_DAYLIGHT,
        light_beacon: LightBeaconState::new(),
        beacon_bearing_stencils: synthetic_beacon_bearing_stencils(),
        local_light_mask: [false; TOWN_GRID_BYTES],
        visibility_dirty: false,
        visibility_grid: [0; VISIBILITY_GRID_LEN],
        terrain_band: [0; TERRAIN_BAND_LEN],
        visibility_buffers_ready: false,
        world_underfoot_blackout_latched: false,
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
        combat_action_result: 0,
        combat_interference_sources: [0; COMBAT_ACTOR_SLOTS],
        combat_active: false,
        pace_combat_presentations: false,
        combat_frame_snapshot: None,
        pending_combat_actor_slot: None,
        pending_combat_terrain_trigger_slot: None,
        pending_town_conflict: None,
        pending_outdoor_reaction_slots: Vec::new(),
        next_combat_actor_slot: 0,
        combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
        combat_magic_effects: [[0; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        combat_cursor_blink: false,
        combat_secondary_marker: None,
        combat_ambush_reveals: [None; COMBAT_AMBUSH_REVEAL_SLOT_COUNT],
        combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        sail_cadence: 0,
        sail_stall_pending: false,
        pending_vehicle_save: PendingVehicleSaveState::default(),
        turn: 0,
        message: String::new(),
        message_transcript: Vec::new(),
        message_transcript_revision: 0,
        message_flushed: String::new(),
        pending_command_echo: None,
        pending_hourly_status_message: None,
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        music_enabled: true,
        sound_effect_serial: 0,
        sound_effect_history: Vec::new(),
        harpsichord_progress: 0,
        active_blackthorn_guard_demand: None,
        pending_town_arrest: None,
        endgame: None,
        active_blackthorn: None,
        blackthorn_audience_map: None,
        active_shop: None,
        common_word_dictionary: None,
        active_conversation: None,
        active_conversation_npc_slot: None,
        active_conversation_join_candidate: None,
        active_z_stats: None,
        active_party_selector: None,
        active_ready: None,
        active_use: None,
        active_cast: None,
        active_cast_followup: None,
        active_rest: None,
        active_jimmy: None,
        active_surface_chest: None,
        active_shrine: None,
        active_mix: None,
        active_new_order: None,
        active_yell: None,
        active_shrine_restoration: None,
        active_wishing_well: None,
        active_view_overlay: None,
        white_potion_sweep: None,
        active_direction_prompt: None,
        active_yes_no_prompt: None,
        town_npc_mutations: Vec::new(),
        talk_branch_flags: HashMap::new(),
        conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
        inn_registry: Vec::new(),
    };
    state.rebuild_surface_local_light_mask();
    state.visibility_dirty = false;
    state
}

/// Refuse to use the player's pristine asset install as a *write*
/// target.
///
/// `CLAUDE.md`'s clean-room rules let the engine read local original
/// game assets at runtime, but never modify them. Several test helpers
/// and harness paths take a "game dir" that they both read from and
/// write into (installed test assets, committed saves, seeded data
/// files). If such a path is ever handed [`crate::DEFAULT_GAME_DIR`],
/// it silently corrupts the reference install - which has happened:
/// `SAVED.GAM` was replaced by a played save and `TITLE.BIT` /
/// `BRITISH.BIT` were rewritten as re-encoded test variants.
///
/// Call this from any helper whose `dir` argument is a write
/// destination. Comparison is on the canonicalised path where
/// possible so `C:/Games/U5-Clean`, `C:\Games\U5-Clean` and
/// symlinked or `..`-relative spellings are all caught; it falls back
/// to a case-insensitive separator-normalised compare when the path
/// does not exist yet.
pub fn assert_writable_game_dir(dir: &Path, context: &str) {
    fn normalize(path: &Path) -> String {
        path.canonicalize()
            .unwrap_or_else(|_| path.to_path_buf())
            .to_string_lossy()
            .replace('\\', "/")
            .trim_end_matches('/')
            .to_ascii_lowercase()
    }

    let pristine = Path::new(crate::DEFAULT_GAME_DIR);
    assert!(
        normalize(dir) != normalize(pristine),
        "{context} would write into the pristine local asset install at {}. \
         Copy the assets to a temp directory and pass that instead - the \
         original game files are read-only clean-room inputs and must never \
         be modified (got {dir:?}).",
        crate::DEFAULT_GAME_DIR
    );
}

/// Clear a file's read-only bit, if it has one.
///
/// The pristine asset install is kept read-only so nothing can modify
/// it by accident. On Windows `fs::copy` propagates that attribute to
/// the destination, so a fixture copied out of it lands read-only and
/// the very next write into the *temp* copy fails with "Access is
/// denied" - a failure that looks like a permissions bug but is really
/// attribute inheritance.
pub fn clear_readonly(path: &Path) -> std::io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    if permissions.readonly() {
        #[allow(clippy::permissions_set_readonly_false)]
        permissions.set_readonly(false);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

/// `fs::copy` that leaves the destination writable. Use this whenever
/// copying out of a game-asset directory into a scratch directory the
/// caller intends to write to.
pub fn copy_asset_writable(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::copy(source, destination)?;
    clear_readonly(destination)
}

#[cfg(test)]
mod pristine_game_dir_guard_tests {
    use super::*;

    #[test]
    fn temp_dirs_are_accepted() {
        assert_writable_game_dir(&debug_game_dir(), "unit test");
    }

    #[test]
    #[should_panic(expected = "pristine local asset install")]
    fn the_pristine_install_is_rejected() {
        assert_writable_game_dir(Path::new(crate::DEFAULT_GAME_DIR), "unit test");
    }

    #[test]
    #[should_panic(expected = "pristine local asset install")]
    fn alternate_spellings_of_the_pristine_install_are_rejected() {
        assert_writable_game_dir(Path::new("C:/Games/u5-clean/"), "unit test");
    }
}

pub fn debug_game_dir() -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("u5-engine-test-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    // `formats/location-dat.md` §3: every class file is exactly sixteen
    // 1024-byte pages. The synthetic file is uniform, but it has to be
    // full length or scenes whose published base page sits high in the
    // file (Lord Blackthorn's Castle enters on page 6) cannot load.
    fs::write(dir.join("CASTLE.DAT"), vec![16; 16 * TOWN_GRID_BYTES]).unwrap();
    // The synthetic CASTLE.DAT is a few uniform pages, not the shipped
    // file, so the published `formats/location-dat.md` §4.1 base page for
    // CASTLE:0 (page 1, with a basement below it and three floors above)
    // does not describe it. Pin the fixture to page 0 so tests that write
    // their own page contents keep addressing them as `floor == page`.
    // Tests that exercise the published table itself must use a directory
    // without this file.
    fs::write(dir.join(LOCATION_FLOOR_TABLE_FILE), "CASTLE:0 0\n").unwrap();
    fs::write(dir.join("CASTLE.NPC"), vec![0; 2304]).unwrap();
    fs::write(dir.join("CASTLE.TLK"), [0, 0]).unwrap();
    fs::write(dir.join("DUNGEON.DAT"), vec![0; DUNGEON_DAT_LEN]).unwrap();
    fs::write(dir.join("UNDER.DAT"), vec![5; UNDER_DAT_LEN]).unwrap();
    // `systems/intro.md §3` step 2: the intro pre-flourish phase
    // loads IBM.CH and RUNES.CH into the resident font-slot table
    // before the title flourish. The terminal harness depends on
    // these files existing in the game directory; populate them
    // here with well-formed 1024-byte glyph tables of solid 0xFF
    // bytes so every glyph renders as a visible 8x8 block. A
    // blank (all-zero) fixture would silently pass the size check
    // but produce empty rendered pixels, breaking downstream
    // tests that assert on visible chargen / menu output.
    fs::write(
        dir.join(crate::IBM_CH_FILE),
        vec![0xffu8; crate::CH_FONT_LEN],
    )
    .unwrap();
    fs::write(
        dir.join(crate::RUNES_CH_FILE),
        vec![0xffu8; crate::CH_FONT_LEN],
    )
    .unwrap();
    write_britannia_world_files(&dir, 5);
    dir
}

pub fn saved_game_seed_bytes(scene: u8, z: u8, x: u8, y: u8) -> Vec<u8> {
    let mut bytes = vec![0; SAVED_GAM_LEN];
    bytes[SAVE_SCENE_OFFSET] = scene;
    bytes[SAVE_Z_OFFSET] = z;
    bytes[SAVE_X_OFFSET] = x;
    bytes[SAVE_Y_OFFSET] = y;
    write_u16_at(&mut bytes, SAVE_FOOD_STOCK_OFFSET, DEFAULT_FOOD_STOCK);
    write_u16_at(&mut bytes, SAVE_GOLD_STOCK_OFFSET, DEFAULT_GOLD_STOCK);
    bytes[SAVE_KEY_STOCK_OFFSET] = DEFAULT_KEY_STOCK;
    bytes[SAVE_GEM_STOCK_OFFSET] = DEFAULT_GEM_STOCK;
    bytes[SAVE_TORCH_STOCK_OFFSET] = DEFAULT_TORCH_STOCK;
    bytes[SAVE_CLIMBING_GEAR_OFFSET] = DEFAULT_CLIMBING_GEAR;
    bytes[SAVE_ACTIVE_PLAYER_OFFSET] = 0xff;
    bytes[SAVE_SPECIAL_ITEM_OFFSET..SAVE_SPECIAL_ITEM_OFFSET + SPECIAL_ITEM_COUNT]
        .copy_from_slice(&[0; SPECIAL_ITEM_COUNT]);
    bytes[SAVE_FIXED_HIDDEN_TREASURE_DAILY_COOKIE_OFFSET] = FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY;
    bytes[SAVE_SHADOWLORD_HIDEOUTS_OFFSET..SAVE_SHADOWLORD_HIDEOUTS_OFFSET + SHADOWLORD_COUNT]
        .copy_from_slice(&DEFAULT_SHADOWLORD_HIDEOUTS);
    encode_reagent_stock(&mut bytes, DEFAULT_REAGENTS);
    write_saved_clock(&mut bytes, GameClock::new(8, 35).unwrap());
    bytes
}

pub fn write_saved_clock(bytes: &mut [u8], clock: GameClock) {
    write_u16_at(bytes, SAVE_YEAR_OFFSET, clock.year);
    bytes[SAVE_MONTH_OFFSET] = clock.month;
    bytes[SAVE_DAY_OFFSET] = clock.day;
    bytes[SAVE_HOUR_OFFSET] = clock.hour;
    bytes[SAVE_MINUTE_OFFSET] = clock.minute;
    bytes[SAVE_AMPM_DISPLAY_OFFSET] = clock.display_hour();
}

pub fn ool_plane_with_object(slot: usize, object: ActiveObject) -> Vec<u8> {
    let mut bytes = vec![0; OOL_PLANE_LEN];
    write_ool_object(&mut bytes, slot, object);
    bytes
}

/// Install the two per-plane mirror files required by the Q-save
/// staging contract. Keep this opt-in so tests for missing/corrupt
/// mirror failures can continue to start from [`debug_game_dir`].
pub fn write_empty_ool_mirrors(game_dir: &Path) {
    fs::write(game_dir.join(BRIT_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
    fs::write(game_dir.join(UNDER_OOL_FILENAME), vec![0; OOL_PLANE_LEN]).unwrap();
}

pub fn write_ool_object(bytes: &mut [u8], slot: usize, object: ActiveObject) {
    assert!(slot < OOL_SLOTS);
    let offset = slot * OOL_RECORD_LEN;
    bytes[offset] = object.type_byte;
    bytes[offset + 1] = object.tile;
    bytes[offset + 2] = object.x as u8;
    bytes[offset + 3] = object.y as u8;
    bytes[offset + 4] = if object.z < 0 { 0xff } else { object.z as u8 };
    bytes[offset + 5] = object.aux1;
    bytes[offset + 6] = object.phase;
    bytes[offset + 7] = object.aux3;
}

pub fn synthetic_britannia_chunk_index() -> [u8; WORLD_CHUNK_COUNT] {
    let mut table = [BRIT_WATER_SENTINEL; WORLD_CHUNK_COUNT];
    for (entry, slot) in table.iter_mut().take(BRIT_STORED_CHUNKS).enumerate() {
        *slot = entry as u8;
    }
    table
}

pub fn write_britannia_world_files(dir: &Path, tile: u8) {
    fs::write(dir.join("DATA.OVL"), synthetic_data_ovl()).unwrap();
    fs::write(dir.join("BRIT.DAT"), vec![tile; BRIT_DAT_LEN]).unwrap();
}

/// A synthetic `DATA.OVL` carrying both resident tables the engine reads
/// out of the overlay: the Britannia chunk index, and the
/// `visibility.md §12.6` beacon bearing stencils at the offset
/// `formats/tiles.md §5.1.1` publishes.
///
/// The stencils have to be here because
/// [`crate::load_beacon_bearing_stencils`] no longer answers "no table" —
/// `§5.1.1` requires it to fail loudly instead, so every game directory a
/// `PlayState` is built from needs a readable table. The geometry is
/// synthetic (see [`synthetic_beacon_stencil_table`]); the shipped
/// offsets stay in the shipped file.
pub fn synthetic_data_ovl() -> Vec<u8> {
    let mut data = vec![42; 32];
    data.extend_from_slice(&synthetic_britannia_chunk_index());
    data.extend_from_slice(&[42; 32]);
    // Zero padding out to the published stencil offset. Zeros cannot
    // form a second Britannia chunk-index candidate (the validator
    // rejects a repeated non-sentinel entry), so the chunk-index search
    // stays unambiguous.
    data.resize(BEACON_STENCIL_TABLE_OFFSET, 0);
    data.extend_from_slice(&synthetic_beacon_stencil_table());
    data.resize(CAMP_NO_EFFECT_MESSAGE_OFFSET + 16, 0);
    data[CAMP_SUCCESS_MESSAGE_OFFSET..CAMP_SUCCESS_MESSAGE_OFFSET + 9]
        .copy_from_slice(b"RESTED!\n\0");
    data[CAMP_NO_EFFECT_MESSAGE_OFFSET..CAMP_NO_EFFECT_MESSAGE_OFFSET + 12]
        .copy_from_slice(b"NO EFFECT!\n\0");
    data
}

/// Sixteen bearing-stencil records matching every published structural
/// rule of `formats/tiles.md §5.1.1` without reproducing the shipped
/// geometry.
///
/// Each record takes the cells nearest its own heading, in
/// Chebyshev-distance order, up to the published per-class cell count —
/// fifteen on a cardinal, eleven on a diagonal, nine on a halfway bearing
/// — and pads the rest of the sixteen pairs with `(0, 0)`.
pub fn synthetic_beacon_stencil_table() -> Vec<u8> {
    let mut bytes = vec![0u8; BEACON_STENCIL_TABLE_BYTES];
    for index in 0..BEACON_BEARING_COUNT as usize {
        let reach = i8::try_from(BEACON_BEAM_MAX_REACH).unwrap();
        let mut cells: Vec<(i8, i8)> = (-reach..=reach)
            .flat_map(|dy| (-reach..=reach).map(move |dx| (dx, dy)))
            .filter(|&(dx, dy)| {
                (dx, dy) != (0, 0)
                    && crate::light_beacon::beacon_offset_matches_bearing(dx, dy, index)
            })
            .collect();
        cells.sort_by_key(|&(dx, dy)| (dx.unsigned_abs().max(dy.unsigned_abs()), dx, dy));
        cells.truncate(beacon_record_cell_count(index));
        assert_eq!(
            cells.len(),
            beacon_record_cell_count(index),
            "record {index} cannot reach its published cell count"
        );
        let start = index * BEACON_STENCIL_RECORD_BYTES;
        for (slot, (dx, dy)) in cells.into_iter().enumerate() {
            bytes[start + slot * 2] = dx as u8;
            bytes[start + slot * 2 + 1] = dy as u8;
        }
    }
    bytes
}

/// The synthetic stencils as a parsed table, for `PlayState` fixtures
/// built without a game directory.
pub fn synthetic_beacon_bearing_stencils() -> BeaconBearingStencils {
    parse_beacon_bearing_stencils(&synthetic_beacon_stencil_table())
        .expect("the synthetic stencil table must satisfy the published record shape")
}

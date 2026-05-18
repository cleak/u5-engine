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
    TileAtlas { depth, pixels }
}

pub fn test_state(grid: Vec<u8>, x: usize, y: usize) -> PlayState {
    let scene = Scene::new(0x11).unwrap();
    PlayState {
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
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        npcs: Vec::new(),
        door_tracker: None,
        opened_town_doors: Vec::new(),
        revealed_town_secret_doors: Vec::new(),
        passability: None,
        moongates: Vec::new(),
        grid,
        clock: GameClock::default(),
        animation: AnimationClock::default(),
        natural_moongate_counter: 0,
        natural_moongate_live_cells: Vec::new(),
        cached_moon_glyph_slots: [None, None],
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
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        shrine_standing: [0; VIRTUE_COUNT],
        moral_standing: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        ambient_light: 0,
        visibility_dirty: false,
        wind: WindState::default(),
        wind_save_byte: 0,
        timing_status: TimingStatusTag::default(),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        fortunes_of_war: 0,
        active_player: None,
        combat_round_counter: 0,
        combat_active: false,
        combat_frame_snapshot: None,
        pending_combat_actor_slot: None,
        pending_combat_terrain_trigger_slot: None,
        next_combat_actor_slot: 0,
        combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
        combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        sail_cadence: 0,
        sail_stall_pending: false,
        turn: 0,
        message: String::new(),
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        music_enabled: true,
        pending_moongate: None,
        pending_town_arrest: None,
        endgame: None,
        active_blackthorn: None,
        blackthorn_jailed_party_slots: Vec::new(),
        active_shop: None,
        active_conversation: None,
        active_z_stats: None,
        active_ready: None,
        active_use: None,
        active_cast: None,
        active_cast_followup: None,
        active_rest: None,
        active_jimmy: None,
        active_shrine: None,
        active_mix: None,
        active_new_order: None,
        active_yell: None,
        active_view_overlay: None,
        active_direction_prompt: None,
        active_yes_no_prompt: None,
        pickpocketed_npcs: Vec::new(),
        removed_town_npcs: Vec::new(),
        town_npc_alarm_states: Vec::new(),
        talk_branch_flags: HashMap::new(),
        conversation_resource_signals: [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT],
        conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
        conversation_signal_bank_a: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
        conversation_signal_bank_b: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
        inn_registry: Vec::new(),
    }
}

pub fn dungeon_state(grid: Vec<u8>, level: u8, x: usize, y: usize) -> PlayState {
    let scene = DungeonScene::new(33).unwrap();
    PlayState {
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
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        npcs: Vec::new(),
        door_tracker: None,
        opened_town_doors: Vec::new(),
        revealed_town_secret_doors: Vec::new(),
        passability: None,
        moongates: Vec::new(),
        grid,
        clock: GameClock::default(),
        animation: AnimationClock::default(),
        natural_moongate_counter: 0,
        natural_moongate_live_cells: Vec::new(),
        cached_moon_glyph_slots: [None, None],
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
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        shrine_standing: [0; VIRTUE_COUNT],
        moral_standing: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        ambient_light: 0,
        visibility_dirty: false,
        wind: WindState::default(),
        wind_save_byte: 0,
        timing_status: TimingStatusTag::default(),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        fortunes_of_war: 0,
        active_player: None,
        combat_round_counter: 0,
        combat_active: false,
        combat_frame_snapshot: None,
        pending_combat_actor_slot: None,
        pending_combat_terrain_trigger_slot: None,
        next_combat_actor_slot: 0,
        combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
        combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        sail_cadence: 0,
        sail_stall_pending: false,
        turn: 0,
        message: String::new(),
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        music_enabled: true,
        pending_moongate: None,
        pending_town_arrest: None,
        endgame: None,
        active_blackthorn: None,
        blackthorn_jailed_party_slots: Vec::new(),
        active_shop: None,
        active_conversation: None,
        active_z_stats: None,
        active_ready: None,
        active_use: None,
        active_cast: None,
        active_cast_followup: None,
        active_rest: None,
        active_jimmy: None,
        active_shrine: None,
        active_mix: None,
        active_new_order: None,
        active_yell: None,
        active_view_overlay: None,
        active_direction_prompt: None,
        active_yes_no_prompt: None,
        pickpocketed_npcs: Vec::new(),
        removed_town_npcs: Vec::new(),
        town_npc_alarm_states: Vec::new(),
        talk_branch_flags: HashMap::new(),
        conversation_resource_signals: [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT],
        conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
        conversation_signal_bank_a: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
        conversation_signal_bank_b: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
        inn_registry: Vec::new(),
    }
}

pub fn world_state(grid: Vec<u8>, x: usize, y: usize) -> PlayState {
    PlayState {
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
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        }],
        npcs: Vec::new(),
        door_tracker: None,
        opened_town_doors: Vec::new(),
        revealed_town_secret_doors: Vec::new(),
        passability: None,
        moongates: Vec::new(),
        grid,
        clock: GameClock::default(),
        animation: AnimationClock::default(),
        natural_moongate_counter: 0,
        natural_moongate_live_cells: Vec::new(),
        cached_moon_glyph_slots: [None, None],
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
        shrine_ordained_mask: 0,
        shrine_codex_mask: 0,
        shrine_standing: [0; VIRTUE_COUNT],
        moral_standing: 0,
        avatar_stats: AvatarStats::default(),
        torches: DEFAULT_TORCH_STOCK,
        torch_counter: 0,
        light_spell_counter: 0,
        ambient_light: 0,
        visibility_dirty: false,
        wind: WindState::default(),
        wind_save_byte: 0,
        timing_status: TimingStatusTag::default(),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        fortunes_of_war: 0,
        active_player: None,
        combat_round_counter: 0,
        combat_active: false,
        combat_frame_snapshot: None,
        pending_combat_actor_slot: None,
        pending_combat_terrain_trigger_slot: None,
        next_combat_actor_slot: 0,
        combat_terrain: DEFAULT_COMBAT_ARENA_TERRAIN,
        combat_actors: [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS],
        sail_cadence: 0,
        sail_stall_pending: false,
        turn: 0,
        message: String::new(),
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        music_enabled: true,
        pending_moongate: None,
        pending_town_arrest: None,
        endgame: None,
        active_blackthorn: None,
        blackthorn_jailed_party_slots: Vec::new(),
        active_shop: None,
        active_conversation: None,
        active_z_stats: None,
        active_ready: None,
        active_use: None,
        active_cast: None,
        active_cast_followup: None,
        active_rest: None,
        active_jimmy: None,
        active_shrine: None,
        active_mix: None,
        active_new_order: None,
        active_yell: None,
        active_view_overlay: None,
        active_direction_prompt: None,
        active_yes_no_prompt: None,
        pickpocketed_npcs: Vec::new(),
        removed_town_npcs: Vec::new(),
        town_npc_alarm_states: Vec::new(),
        talk_branch_flags: HashMap::new(),
        conversation_resource_signals: [0; CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT],
        conversation_signal_flags: [0; TLK_GENERIC_SIGNAL_COUNT],
        conversation_signal_bank_a: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
        conversation_signal_bank_b: [0; CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT],
        inn_registry: Vec::new(),
    }
}

pub fn debug_game_dir() -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("u5-engine-test-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("CASTLE.DAT"), vec![16; 4096]).unwrap();
    fs::write(dir.join("CASTLE.NPC"), vec![0; 2304]).unwrap();
    fs::write(dir.join("CASTLE.TLK"), [1, 0, 0, 0]).unwrap();
    fs::write(dir.join("DUNGEON.DAT"), vec![0; DUNGEON_DAT_LEN]).unwrap();
    fs::write(dir.join("UNDER.DAT"), vec![5; UNDER_DAT_LEN]).unwrap();
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
    let table = synthetic_britannia_chunk_index();
    let mut data = vec![42; 32];
    data.extend_from_slice(&table);
    data.extend_from_slice(&[42; 32]);
    fs::write(dir.join("DATA.OVL"), data).unwrap();
    fs::write(dir.join("BRIT.DAT"), vec![tile; BRIT_DAT_LEN]).unwrap();
}

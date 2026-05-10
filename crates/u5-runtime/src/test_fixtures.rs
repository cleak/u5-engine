//! Synthetic fixtures used by both u5-runtime's internal tests and
//! u5-tui's integration tests. Public API so it can be reached from
//! other crates' test targets.

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
        ambient_light: 0,
        visibility_dirty: false,
        wind: WindState::default(),
        wind_save_byte: 0,
        timing_status: TimingStatusTag::default(),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        sail_cadence: 0,
        sail_stall_pending: false,
        turn: 0,
        message: String::new(),
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        pending_moongate: None,
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
        ambient_light: 0,
        visibility_dirty: false,
        wind: WindState::default(),
        wind_save_byte: 0,
        timing_status: TimingStatusTag::default(),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        sail_cadence: 0,
        sail_stall_pending: false,
        turn: 0,
        message: String::new(),
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        pending_moongate: None,
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
        ambient_light: 0,
        visibility_dirty: false,
        wind: WindState::default(),
        wind_save_byte: 0,
        timing_status: TimingStatusTag::default(),
        time_stop_counter: 0,
        active_effect_tag: None,
        active_effect_counter: 0,
        sail_cadence: 0,
        sail_stall_pending: false,
        turn: 0,
        message: String::new(),
        debug_enter: None,
        return_world: None,
        world_overlays: WorldOverlayCache::default(),
        save_template_source: SaveTemplateSource::PreferSavedGame,
        typeahead_buffer_enabled: false,
        pending_moongate: None,
    }
}

pub fn debug_game_dir() -> PathBuf {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "u5-engine-test-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("CASTLE.DAT"), vec![16; 1024]).unwrap();
    fs::write(dir.join("CASTLE.NPC"), vec![0; 576]).unwrap();
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

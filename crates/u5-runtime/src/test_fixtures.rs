//! Synthetic fixtures used by both u5-runtime's internal tests and
//! u5-tui's integration tests. Public API so it can be reached from
//! other crates' test targets.

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

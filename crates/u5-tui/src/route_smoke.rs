//! Route-level smoke suite for local clean assets.
//!
//! These cases intentionally exercise public harness routes and sidecar-backed
//! transitions without asserting copyrighted text content.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use u5_runtime::{
    ActiveObject, Area, ArmsShop, BLACKTHORN_CAPTIVE_CELL_SCENE, BLACKTHORN_RESCUE_HANDOFF_SCENE,
    BLINK_COST, BLINK_SPELL_INDEX, COMBAT_PARTY_ACTOR_SLOTS, CREATE_FOOD_COST,
    CREATE_FOOD_MAX_GRANT, CREATE_FOOD_SPELL_INDEX, DEATH_VISION_OBJECT_CLASS, DEFAULT_FOOD_STOCK,
    DUNGEON_AMBUSH_ARENA_FLOOR_TILE, Direction, DungeonScene, EQUIP_SLOT_RING, EQUIP_SLOT_WEAPON,
    EQUIPMENT_EMPTY, EQUIPMENT_ID_ARROWS, EQUIPMENT_ID_BOW, EQUIPMENT_ID_RING_REGENERATION,
    FIRST_PLAYABLE_FRIGATE_TILE, FIRST_PLAYABLE_FULL_SHIP_HULL,
    FIRST_PLAYABLE_HOURLY_POISON_DAMAGE, GATE_TRAVEL_COST, GATE_TRAVEL_SPELL_INDEX, GameClock,
    GuildShop, HORSE_PARKED_FIRST, HOURLY_STARVATION_DAMAGE_MAX, HOURLY_STARVATION_DAMAGE_MIN,
    Healer, Herbalist, IN_LOR_SPELL_INDEX, Inn, MoonstoneGateSlot, NATURAL_MOONGATE_TERRAIN_TILE,
    NpcSlot, PEER_COST, PEER_SPELL_INDEX, PartyMember, PlayOptions, PlayState, PlayTarget,
    REAGENT_SULFUR_ASH, SCENE_EMPATH_ABBEY, SCENE_JHELOM, SCENE_MOONGLOW, SCENE_SERPENTS_HOLD,
    SCENE_STONEGATE, SCENE_THE_LYCAEUM, SHADOWLORD_COWARDICE_INDEX, SHADOWLORD_FALSEHOOD_INDEX,
    SHADOWLORD_HATRED_INDEX, SHADOWLORD_HIDEOUT_VANQUISHED, SHADOWLORD_OBJECT_TILE_BASE,
    SHADOWLORD_VANQUISHED, SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX, SPECIAL_ITEM_MAGIC_CARPET_INDEX,
    SPECIAL_ITEM_OWNED_VALUE, SPECIAL_ITEM_POCKET_WATCH_INDEX, SPECIAL_ITEM_SCEPTRE_LB_INDEX,
    SPECIAL_ITEM_SEXTANT_INDEX, SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
    SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX, SPECIAL_ITEM_SHARD_HATRED_INDEX,
    SPECIAL_ITEM_SPYGLASS_INDEX, SPECIAL_ITEM_WOODEN_BOX_INDEX, STEADY_PHASE, SURFACE_CHASM_X,
    SURFACE_CHASM_Y, Scene, Shipwright, Stable, TALK_NO_RESPONSE_MESSAGE, TALK_SLEEPING_MESSAGE,
    TALK_STATUS_TILE_PRAYING, TALK_STATUS_TILE_SLEEPING, TAVERN_AFFORDABILITY_REFUSAL_BARK,
    TOWN_GAS_DOORWAY_RANGE_MAX, TOWN_GRID_SIDE, TOWN_POISON_GAS_LIVE_TILE, Tavern,
    TileGraphicsDepth, TransportState, WORD_OF_POWER_SEAL_XOR, WORLD_SIDE, WindState,
    WordOfPowerSeal, WorldPlane, WorldReturn, X_RAY_COST, X_RAY_SPELL_INDEX,
    default_party_equipment, default_party_experience, default_party_intelligence,
    default_party_names, default_party_stay_counters, dungeon_cell_index, inn_base_room_rate,
    load_tile_atlas, shop_intelligence_adjusted_price,
    shop_runtime::{
        ArmsShopState, GuildShopState, HealerShopState, HorseTraderState, InnkeeperState,
        ReagentShopState, SageState, ShipBrokerState, TavernState,
    },
    shop_session::ActiveShopSession,
    stable_horse_price, u5_prng_range_u16, word_of_power_seal_for_word, world_cell_index,
};

use crate::{
    play_script_state_line, raster_diagnostic_line, raster_frame_kind, replay_play_script_commands,
};

const VIEWPORT_RADIUS: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteSmokeExpectation {
    World(WorldPlane),
    Town(Scene),
    Dungeon(DungeonScene),
}

impl RouteSmokeExpectation {
    fn matches(self, state: &PlayState) -> bool {
        match (self, state.area) {
            (Self::World(expected), Area::World { plane }) => expected == plane,
            (Self::Town(expected), Area::Town { scene, .. }) => expected == scene,
            (Self::Dungeon(expected), Area::Dungeon { scene, .. }) => expected == scene,
            _ => false,
        }
    }

    fn label(self) -> String {
        match self {
            Self::World(plane) => plane.key().to_string(),
            Self::Town(scene) => scene.key(),
            Self::Dungeon(scene) => scene.key(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RouteSmokeCase {
    pub name: &'static str,
    pub options: PlayOptions,
    pub script: &'static [&'static str],
    pub expected: RouteSmokeExpectation,
    pub min_turn: u64,
    pub expected_frame_kind: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSmokeReport {
    pub name: String,
    pub commands_run: usize,
    pub final_state_line: String,
    pub final_raster_line: String,
}

pub fn route_smoke_cases() -> Vec<RouteSmokeCase> {
    let castle = Scene::new(0x11).expect("castle scene is valid");
    let blackthorn_captive_scene =
        Scene::new(BLACKTHORN_CAPTIVE_CELL_SCENE).expect("Blackthorn captive scene is valid");
    let blackthorn_rescue_scene =
        Scene::new(BLACKTHORN_RESCUE_HANDOFF_SCENE).expect("Blackthorn rescue scene is valid");
    let shadowlord_town = Scene::new(SCENE_MOONGLOW).expect("Shadowlord hideout town is valid");
    let stonegate = Scene::new(SCENE_STONEGATE).expect("Stonegate scene is valid");
    let dungeon = DungeonScene::new(0x21).expect("dungeon scene is valid");
    let doom = DungeonScene::new(0x28).expect("doom dungeon scene is valid");

    let world = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };

    let underworld = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        ..PlayOptions::default()
    };

    let fallax_seal =
        word_of_power_seal_for_word("FALLAX").expect("FALLAX Word-of-Power seal row is public");
    let veramocor_seal = word_of_power_seal_for_word("VERAMOCOR")
        .expect("VERAMOCOR Word-of-Power seal row is public");
    let britannia_word_of_power = PlayOptions {
        target: PlayTarget::World(fallax_seal.plane),
        ..PlayOptions::default()
    };
    let doom_word_of_power = PlayOptions {
        target: PlayTarget::World(veramocor_seal.plane),
        ..PlayOptions::default()
    };

    let fixed_hidden_single_use = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((79, 64)),
        facing: Some(Direction::East),
        ..PlayOptions::default()
    };

    let minoc = Scene::new(0x05).expect("Minoc scene is valid");
    let fixed_hidden_daily = PlayOptions {
        target: PlayTarget::Town(minoc),
        clock: GameClock::new(5, 0).expect("05:00 is a valid game-clock time"),
        ..PlayOptions::default()
    };

    let fixed_hidden_underworld_stack = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        ..PlayOptions::default()
    };

    let blackthorn_fixed_hidden_key_cache = PlayOptions {
        target: PlayTarget::Town(Scene::new(18).expect("Blackthorn castle scene is valid")),
        floor: -1,
        ..PlayOptions::default()
    };

    let mut world_move = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    world_move.start = Some((62, 124));

    let world_to_castle = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        debug_enter: Some(PlayTarget::Town(castle)),
        ..PlayOptions::default()
    };

    let world_to_dungeon = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        debug_enter: Some(PlayTarget::Dungeon(dungeon)),
        ..PlayOptions::default()
    };

    let underworld_to_castle = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        debug_enter: Some(PlayTarget::Town(castle)),
        ..PlayOptions::default()
    };

    let ship_transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 2,
    };
    let ship_xit = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        ..PlayOptions::default()
    };
    let ship_sail = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        wind: WindState::East,
        wind_save_byte: WindState::East.save_byte(),
        ..PlayOptions::default()
    };

    let dungeon_options = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        ..PlayOptions::default()
    };

    let mut britannia_view = world.clone();
    britannia_view.gems = 1;

    let mut britannia_spyglass = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        clock: GameClock::new(20, 0).expect("20:00 is a valid game-clock time"),
        ..PlayOptions::default()
    };
    britannia_spyglass.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;

    let mut britannia_utility_use = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        clock: GameClock::new(20, 0).expect("20:00 is a valid game-clock time"),
        ..PlayOptions::default()
    };
    britannia_utility_use.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    britannia_utility_use.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    britannia_utility_use.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] = 1;

    let mut hms_cape_plans = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        ..PlayOptions::default()
    };
    hms_cape_plans.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;

    let mut create_food = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        food: DEFAULT_FOOD_STOCK,
        ..PlayOptions::default()
    };
    create_food.spell_charges[CREATE_FOOD_SPELL_INDEX] = 1;
    create_food.party[0].mana = CREATE_FOOD_COST;
    create_food.party[0].level = CREATE_FOOD_COST;

    let mut blink_east = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((62, 124)),
        ..PlayOptions::default()
    };
    blink_east.spell_charges[BLINK_SPELL_INDEX] = 1;
    blink_east.party[0].mana = BLINK_COST;
    blink_east.party[0].level = BLINK_COST;

    let hourly_provision_poison = PlayOptions {
        target: PlayTarget::Town(castle),
        clock: GameClock::with_date(139, 4, 5, 5, 59).expect("05:59 is a valid game-clock time"),
        food: 10,
        party: vec![
            route_party_member(0, b'A', b'G', 12, 20),
            route_party_member(1, b'F', b'P', 12, 20),
            route_party_member(2, b'M', b'S', 12, 20),
            route_party_member(3, b'D', b'D', 0, 20),
            route_party_member(4, b'B', b'A', 0, 20),
        ],
        ..PlayOptions::default()
    };

    let hourly_poison_starvation = PlayOptions {
        target: PlayTarget::Town(castle),
        clock: GameClock::with_date(139, 4, 5, 8, 59).expect("08:59 is a valid game-clock time"),
        food: 0,
        party: vec![
            route_party_member(0, b'A', b'P', 20, 20),
            route_party_member(1, b'F', b'G', 20, 20),
            route_party_member(2, b'M', b'D', 0, 20),
        ],
        ..PlayOptions::default()
    };

    let mut hourly_ring_regeneration = PlayOptions {
        target: PlayTarget::Town(castle),
        clock: GameClock::with_date(139, 4, 5, 7, 59).expect("07:59 is a valid game-clock time"),
        food: 99,
        party: vec![route_party_member(0, b'A', b'G', 19, 20)],
        party_equipment: default_party_equipment(1),
        ..PlayOptions::default()
    };
    hourly_ring_regeneration.party_equipment[0][EQUIP_SLOT_RING] =
        EQUIPMENT_ID_RING_REGENERATION as u8;

    let town_poison_gas = PlayOptions {
        target: PlayTarget::Town(castle),
        party: vec![
            route_party_member(0, b'A', b'P', 10, 20),
            route_party_member(1, b'F', b'G', 10, 20),
            route_party_member(2, b'M', b'P', 10, 20),
        ],
        ..PlayOptions::default()
    };

    let mut command_workflows = PlayOptions {
        target: PlayTarget::Town(castle),
        party: vec![
            route_party_member(0, b'A', b'G', 20, 20),
            route_party_member(1, b'F', b'G', 20, 20),
            route_party_member(2, b'M', b'G', 20, 20),
        ],
        party_names: default_party_names(3),
        party_experience: default_party_experience(3),
        party_stay_counters: default_party_stay_counters(3),
        party_strengths: vec![50; 3],
        party_intelligence: default_party_intelligence(3),
        party_equipment: default_party_equipment(3),
        ..PlayOptions::default()
    };
    command_workflows.reagents[REAGENT_SULFUR_ASH] = 2;
    command_workflows.equipment_stock[EQUIPMENT_ID_BOW] = 1;
    command_workflows.equipment_stock[EQUIPMENT_ID_ARROWS] = 5;

    let board_horse = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        facing: Some(Direction::East),
        ..PlayOptions::default()
    };

    let mut gate_travel_to_underworld = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    seed_gate_travel_resources(&mut gate_travel_to_underworld);
    gate_travel_to_underworld.moonstone_slots[0] = MoonstoneGateSlot {
        scene: 0,
        x: 231,
        y: 5,
        z: WorldPlane::Underworld.save_floor() as u8,
    };

    let mut gate_travel_to_castle = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    seed_gate_travel_resources(&mut gate_travel_to_castle);
    gate_travel_to_castle.moonstone_slots[1] = MoonstoneGateSlot {
        scene: castle.byte,
        x: 7,
        y: 0,
        z: 0,
    };

    let mut gate_travel_invalid_slot = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    seed_gate_travel_resources(&mut gate_travel_invalid_slot);

    let mut gate_travel_shipboard_refusal = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        ..PlayOptions::default()
    };
    seed_gate_travel_resources(&mut gate_travel_shipboard_refusal);
    gate_travel_shipboard_refusal.moonstone_slots[1] = gate_travel_to_castle.moonstone_slots[1];

    let mut natural_moongate_trammel = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((62, 124)),
        clock: GameClock::new(1, 0).expect("01:00 is a valid game-clock time"),
        ..PlayOptions::default()
    };
    natural_moongate_trammel.moonstone_slots[0] = gate_travel_to_underworld.moonstone_slots[0];

    let natural_moongate_empty_slot = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((62, 124)),
        clock: GameClock::new(1, 0).expect("01:00 is a valid game-clock time"),
        ..PlayOptions::default()
    };

    let chasm_fall = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((SURFACE_CHASM_X as usize, SURFACE_CHASM_Y as usize - 1)),
        facing: Some(Direction::South),
        ..PlayOptions::default()
    };

    let mut wooden_box = PlayOptions::default();
    wooden_box.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;

    let mut shadowlord_town_entry = PlayOptions {
        target: PlayTarget::Town(shadowlord_town),
        ..PlayOptions::default()
    };
    shadowlord_town_entry.shadowlord_hideouts = [
        SCENE_MOONGLOW,
        SHADOWLORD_HIDEOUT_VANQUISHED,
        SHADOWLORD_HIDEOUT_VANQUISHED,
    ];

    let mut shadowlord_town_yell = shadowlord_town_entry.clone();
    shadowlord_town_yell.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = SCENE_MOONGLOW;

    let mut lycaeum_shard_falsehood = PlayOptions {
        target: PlayTarget::Town(Scene::new(SCENE_THE_LYCAEUM).expect("Lycaeum scene is valid")),
        floor: 2,
        ..PlayOptions::default()
    };
    lycaeum_shard_falsehood.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX] =
        SPECIAL_ITEM_OWNED_VALUE;
    lycaeum_shard_falsehood.shadowlord_hideouts[SHADOWLORD_FALSEHOOD_INDEX] = SCENE_MOONGLOW;

    let mut empath_shard_hatred = PlayOptions {
        target: PlayTarget::Town(Scene::new(SCENE_EMPATH_ABBEY).expect("Empath Abbey is valid")),
        floor: 1,
        ..PlayOptions::default()
    };
    empath_shard_hatred.special_items[SPECIAL_ITEM_SHARD_HATRED_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    empath_shard_hatred.shadowlord_hideouts[SHADOWLORD_HATRED_INDEX] = SCENE_MOONGLOW;

    let mut serpents_shard_cowardice = PlayOptions {
        target: PlayTarget::Town(Scene::new(SCENE_SERPENTS_HOLD).expect("Serpent's Hold is valid")),
        floor: -1,
        ..PlayOptions::default()
    };
    serpents_shard_cowardice.special_items[SPECIAL_ITEM_SHARD_COWARDICE_INDEX] =
        SPECIAL_ITEM_OWNED_VALUE;
    serpents_shard_cowardice.shadowlord_hideouts[SHADOWLORD_COWARDICE_INDEX] = SCENE_MOONGLOW;

    let mut stonegate_entry = PlayOptions {
        target: PlayTarget::Town(stonegate),
        ..PlayOptions::default()
    };
    stonegate_entry.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    stonegate_entry.shadowlord_hideouts =
        [SCENE_MOONGLOW, SHADOWLORD_HIDEOUT_VANQUISHED, SCENE_JHELOM];

    let mut castle_view = PlayOptions::default();
    castle_view.gems = 1;

    let mut dungeon_view = dungeon_options.clone();
    dungeon_view.gems = 1;

    let mut peer_view = PlayOptions::default();
    peer_view.spell_charges[PEER_SPELL_INDEX] = 1;
    peer_view.party[0].mana = PEER_COST + 1;
    peer_view.party[0].level = PEER_COST;

    let mut x_ray_view = PlayOptions::default();
    x_ray_view.spell_charges[X_RAY_SPELL_INDEX] = 1;
    x_ray_view.party[0].mana = X_RAY_COST + 1;
    x_ray_view.party[0].level = X_RAY_COST;

    let surface_fountain = PlayOptions::default();
    let yew_poster_scene = Scene::new(4).expect("Yew scene is valid");
    let yew_wanted_poster = PlayOptions {
        target: PlayTarget::Town(yew_poster_scene),
        ..PlayOptions::default()
    };
    let wishing_well_scene = Scene::new(0x16).expect("Buccaneer's Den scene is valid");
    let wishing_well = PlayOptions {
        target: PlayTarget::Town(wishing_well_scene),
        gold: 5,
        ..PlayOptions::default()
    };
    let mut death_vision = PlayOptions::default();
    death_vision.party_intelligence[0] = 30;

    let mut light_decay = PlayOptions::default();
    light_decay.light_spell_counter = 2;

    let dungeon_rest_no_direct_recovery = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        clock: GameClock::new(8, 0).expect("08:00 is a valid game-clock time"),
        party: vec![
            route_party_member(0, b'A', b'G', 5, 20),
            route_party_member(1, b'F', b'S', 3, 20),
            route_party_member(2, b'M', b'D', 0, 20),
        ],
        ..PlayOptions::default()
    };

    let doom_options = PlayOptions {
        target: PlayTarget::Dungeon(doom),
        floor: 0,
        ..PlayOptions::default()
    };

    vec![
        RouteSmokeCase {
            name: "castle-pass-and-idle",
            options: PlayOptions::default(),
            script: &["empty", "idle:2"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-move-pass-idle",
            options: world_move.clone(),
            script: &["d", "empty", "idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-look-pass",
            options: world.clone(),
            script: &["l6", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-view-overlay",
            options: britannia_view,
            script: &["v"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "britannia-spyglass-chunk-map",
            options: britannia_spyglass,
            script: &["USP"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "britannia-utility-use-items",
            options: britannia_utility_use,
            script: &["UW", "US", "UC"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 3,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-hms-cape-plans-use",
            options: hms_cape_plans,
            script: &["UP"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-broadside-fire-route",
            options: ship_xit.clone(),
            script: &["F6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-create-food-cast",
            options: create_food,
            script: &["C1IMX"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-blink-east-ray",
            options: blink_east,
            script: &["C1IP6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-hourly-provision-poison-pass",
            options: hourly_provision_poison,
            script: &["empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-hourly-poison-starvation-pass",
            options: hourly_poison_starvation,
            script: &["empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-hourly-ring-regeneration-pass",
            options: hourly_ring_regeneration,
            script: &["empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-poison-gas-step",
            options: town_poison_gas,
            script: &["d"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-mix-ready-order-route",
            options: command_workflows,
            script: &["MIL/0x80/1", "R1/26", "R1/26", "N23"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-board-horse-route",
            options: board_horse,
            script: &["B"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "gate-travel-world-to-underworld",
            options: gate_travel_to_underworld,
            script: &["C1PRV1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "gate-travel-world-to-castle",
            options: gate_travel_to_castle,
            script: &["C1PRV2"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "gate-travel-invalid-slot-refusal",
            options: gate_travel_invalid_slot,
            script: &["C1PRV4"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "gate-travel-shipboard-refusal",
            options: gate_travel_shipboard_refusal,
            script: &["C1PRV2"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "natural-moongate-trammel-gate-travel",
            options: natural_moongate_trammel,
            script: &["idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "natural-moongate-empty-slot-clears-live-tile",
            options: natural_moongate_empty_slot,
            script: &["idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-chasm-fall-to-underworld",
            options: chasm_fall,
            script: &["s"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-hole-up-rest",
            options: world.clone(),
            script: &["H1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 3,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-save-refusal",
            options: world.clone(),
            script: &["Q", "N"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-dispatcher-refusals",
            options: world_move,
            script: &["B", "D", "F", "T6", "W", "X"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-fixed-hidden-single-use-search-get",
            options: fixed_hidden_single_use,
            script: &["S6", "G6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "underworld-pass-and-idle",
            options: underworld.clone(),
            script: &["empty", "idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "underworld-fixed-hidden-stack-search-get-search",
            options: fixed_hidden_underworld_stack,
            script: &["S6", "G6", "S6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 3,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "blackthorn-fixed-hidden-zero-key-search",
            options: blackthorn_fixed_hidden_key_cache,
            script: &["S6"],
            expected: RouteSmokeExpectation::Town(
                Scene::new(18).expect("Blackthorn castle scene is valid"),
            ),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-z-stats-modal",
            options: PlayOptions::default(),
            script: &["Z", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-wooden-box-use",
            options: wooden_box,
            script: &["UB"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "endgame-missing-box-confirmation",
            options: PlayOptions::default(),
            script: &["Y", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "endgame-box-victory-confirmation",
            options: PlayOptions::default(),
            script: &["Y", "Y", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "blackthorn-audience-correct",
            options: PlayOptions::default(),
            script: &["Ahm"],
            expected: RouteSmokeExpectation::Town(blackthorn_captive_scene),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "blackthorn-audience-wrong",
            options: PlayOptions::default(),
            script: &["wrong"],
            expected: RouteSmokeExpectation::Town(blackthorn_captive_scene),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "blackthorn-rescue-refuge",
            options: PlayOptions::default(),
            script: &["empty"],
            expected: RouteSmokeExpectation::Town(blackthorn_rescue_scene),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "virtue-town-shadowlord-entry",
            options: shadowlord_town_entry,
            script: &[],
            expected: RouteSmokeExpectation::Town(shadowlord_town),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "virtue-town-shadowlord-yell",
            options: shadowlord_town_yell,
            script: &["YFAULINEI"],
            expected: RouteSmokeExpectation::Town(shadowlord_town),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "lycaeum-shard-falsehood-vanquish",
            options: lycaeum_shard_falsehood,
            script: &["UF"],
            expected: RouteSmokeExpectation::Town(
                Scene::new(SCENE_THE_LYCAEUM).expect("Lycaeum scene is valid"),
            ),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "empath-shard-hatred-vanquish",
            options: empath_shard_hatred,
            script: &["UH"],
            expected: RouteSmokeExpectation::Town(
                Scene::new(SCENE_EMPATH_ABBEY).expect("Empath Abbey is valid"),
            ),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "serpents-hold-shard-cowardice-vanquish",
            options: serpents_shard_cowardice,
            script: &["UCW"],
            expected: RouteSmokeExpectation::Town(
                Scene::new(SCENE_SERPENTS_HOLD).expect("Serpent's Hold is valid"),
            ),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "stonegate-shadowlord-entry",
            options: stonegate_entry,
            script: &[],
            expected: RouteSmokeExpectation::Town(stonegate),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-word-of-power-seal-opens",
            options: britannia_word_of_power,
            script: &["YFALLAX"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "underworld-doom-word-of-power-seal-opens",
            options: doom_word_of_power,
            script: &["YVERAMOCOR"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-look-pass",
            options: PlayOptions::default(),
            script: &["l6", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "minoc-fixed-hidden-daily-search-get-repeat",
            options: fixed_hidden_daily,
            script: &["S6", "G6", "S6"],
            expected: RouteSmokeExpectation::Town(minoc),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-view-overlay",
            options: castle_view,
            script: &["v"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "castle-peer-overlay",
            options: peer_view,
            script: &["C1QWI"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "castle-x-ray-overlay",
            options: x_ray_view,
            script: &["C1AWY"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "castle-surface-fountain-look",
            options: surface_fountain,
            script: &["l6", "1"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "yew-wanted-poster-look",
            options: yew_wanted_poster,
            script: &["l6"],
            expected: RouteSmokeExpectation::Town(yew_poster_scene),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "buccaneers-den-wishing-well-horse",
            options: wishing_well.clone(),
            script: &["l6", "Y", "Horse"],
            expected: RouteSmokeExpectation::Town(wishing_well_scene),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "buccaneers-den-wishing-well-ferrari-grants-horse",
            options: wishing_well,
            script: &["l6", "Y", "Ferrari"],
            expected: RouteSmokeExpectation::Town(wishing_well_scene),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-death-vision-look",
            options: death_vision,
            script: &["l6", "1"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "castle-talk-status-sleeping-refusal",
            options: PlayOptions::default(),
            script: &["T6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-talk-status-praying-refusal",
            options: PlayOptions::default(),
            script: &["T6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-light-decay-route",
            options: light_decay,
            script: &["empty", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-save-refusal",
            options: PlayOptions::default(),
            script: &["Q", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-dispatcher-refusals",
            options: PlayOptions::default(),
            script: &["B", "D", "E", "F6", "M", "W", "X"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-party-overlay-routes",
            options: PlayOptions::default(),
            script: &["C1IL", "I", "N12", "R"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-native-stair-up-route",
            options: PlayOptions::default(),
            script: &["d"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-native-stair-down-route",
            options: PlayOptions {
                floor: 1,
                ..PlayOptions::default()
            },
            script: &["d"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-native-stair-cross-route",
            options: PlayOptions::default(),
            script: &["w"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-local-buy-sell-route",
            options: PlayOptions::default(),
            script: &["B", "A", "N", "S", "1", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-healer-heal-decline-route",
            options: PlayOptions::default(),
            script: &["Y", "H", "1", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-inn-rest-decline-route",
            options: PlayOptions::default(),
            script: &["R", "N", "P"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-inn-rest-accept-public-rate",
            options: PlayOptions::default(),
            script: &["R", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-reagent-buy-route",
            options: PlayOptions::default(),
            script: &["A", "1", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-drink-and-food-route",
            options: PlayOptions::default(),
            script: &["Y", "M", "P", "1", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-horse-trader-decline-route",
            options: PlayOptions::default(),
            script: &["B", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-horse-trader-horse-and-rider-buy",
            options: PlayOptions::default(),
            script: &["B", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-horse-trader-stablehouse-buy",
            options: PlayOptions::default(),
            script: &["B", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-horse-trader-wishing-well-buy",
            options: PlayOptions::default(),
            script: &["B", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-shipwright-quote-decline-route",
            options: world.clone(),
            script: &["F", "N"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-guild-buy-route",
            options: PlayOptions::default(),
            script: &["A", "1", "D"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-sage-topic-miss-route",
            options: PlayOptions::default(),
            script: &["MANTRA", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-sage-topic-paid-success-route",
            options: PlayOptions::default(),
            script: &["HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-sage-topic-short-funds-route",
            options: PlayOptions::default(),
            script: &["COMP", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-castle",
            options: world_to_castle.clone(),
            script: &["e", "empty", "idle:1"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-castle-return-world",
            options: world_to_castle,
            script: &["e", "w", "idle:1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-castle-from-underworld",
            options: underworld_to_castle,
            script: &["e", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-xit-launches-skiff",
            options: ship_xit,
            script: &["X", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-hoist-and-sail-east",
            options: ship_sail,
            script: &["Y", "d", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 3,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-dungeon",
            options: world_to_dungeon,
            script: &["e", "Q", "N"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-exit-refusal",
            options: dungeon_options.clone(),
            script: &["Q", "N", "idle:1"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-view-overlay",
            options: dungeon_view,
            script: &["v"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "dungeon-hole-up-rest",
            options: dungeon_options.clone(),
            script: &["H1"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 3,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-hole-up-no-direct-recovery",
            options: dungeon_rest_no_direct_recovery,
            script: &["H1"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 3,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-ignite-torch-route",
            options: dungeon_options.clone(),
            script: &["I"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-turn-and-blocked-step",
            options: dungeon_options.clone(),
            script: &["w", "a", "d", "s"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 2,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-heavy-door-variant-block",
            options: dungeon_options.clone(),
            script: &["."],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-ladder-down-up-route",
            options: dungeon_options.clone(),
            script: &[">", "<"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 2,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-surface-exit-return-world",
            options: dungeon_options.clone(),
            script: &["K"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "dungeon-attack-direction-route",
            options: dungeon_options.clone(),
            script: &["A", "6"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-active-monster-attack-ambush",
            options: dungeon_options.clone(),
            script: &["A"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "dungeon-active-monster-contact-ambush",
            options: dungeon_options.clone(),
            script: &["empty"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "dungeon-search-focus-route",
            options: dungeon_options.clone(),
            script: &["S6"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-sjog-underfoot-routes",
            options: dungeon_options.clone(),
            script: &["G", "J", "O"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-refusal-letter-routes",
            options: dungeon_options.clone(),
            script: &["B", "E", "F", "P", "X", "T"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-exit-confirm",
            options: dungeon_options,
            script: &["Q", "Y"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 0,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "doom-room-combat-trigger",
            options: doom_options.clone(),
            script: &["empty"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-pass-round",
            options: doom_options.clone(),
            script: &["empty", "empty"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-select-player-clear",
            options: doom_options.clone(),
            script: &["empty", "0"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-select-player-one",
            options: doom_options.clone(),
            script: &["empty", "1"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-select-player-six",
            options: doom_options.clone(),
            script: &["empty", "6"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-escape-abort",
            options: doom_options.clone(),
            script: &["empty", "\x1b"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-music-toggle",
            options: doom_options.clone(),
            script: &["empty", "ctrl-s"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-direct-step-east",
            options: doom_options.clone(),
            script: &["empty", "d"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-use-refusal",
            options: doom_options.clone(),
            script: &["empty", "U"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-d-refusal",
            options: doom_options.clone(),
            script: &["empty", "D"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-w-refusal",
            options: doom_options.clone(),
            script: &["empty", "W"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-board-refusal",
            options: doom_options.clone(),
            script: &["empty", "B"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-enter-refusal",
            options: doom_options.clone(),
            script: &["empty", "E"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-fire-refusal",
            options: doom_options.clone(),
            script: &["empty", "F"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-hole-up-refusal",
            options: doom_options.clone(),
            script: &["empty", "H"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-ignite-refusal",
            options: doom_options.clone(),
            script: &["empty", "I"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-mix-refusal",
            options: doom_options.clone(),
            script: &["empty", "M"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-new-order-refusal",
            options: doom_options.clone(),
            script: &["empty", "N"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-talk-refusal",
            options: doom_options.clone(),
            script: &["empty", "T"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-view-label-only",
            options: doom_options.clone(),
            script: &["empty", "V"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-look-label-only",
            options: doom_options.clone(),
            script: &["empty", "L"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-attack-direction",
            options: doom_options.clone(),
            script: &["empty", "A6"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-cast-refusal",
            options: doom_options.clone(),
            script: &["empty", "C1IL"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-get-direction",
            options: doom_options.clone(),
            script: &["empty", "G6"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-jimmy-direction",
            options: doom_options.clone(),
            script: &["empty", "J6"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-open-direction",
            options: doom_options.clone(),
            script: &["empty", "O6"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-push-direction",
            options: doom_options.clone(),
            script: &["empty", "P6"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-klimb-direction",
            options: doom_options.clone(),
            script: &["empty", "K6"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-ready-prompt",
            options: doom_options.clone(),
            script: &["empty", "R"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-z-stats",
            options: doom_options.clone(),
            script: &["empty", "Z"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-yell-word",
            options: doom_options.clone(),
            script: &["empty", "YFALLAX"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-xit-foes-remain",
            options: doom_options.clone(),
            script: &["empty", "X"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-search-prompt",
            options: doom_options,
            script: &["empty", "S"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "britannia-extended-exploration",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: &[
                "d", "d", "s", "s", "a", "a", "w", "w", "l6", "empty", "Z", "empty",
            ],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 8,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-extended-walk-and-rest",
            options: PlayOptions::default(),
            script: &["s", "s", "a", "a", "w", "d", "Z", "empty", "l6", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "dungeon-extended-turn-and-search",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                ..PlayOptions::default()
            },
            script: &["a", "a", "d", "w", "s", "w", "S6", "G", "empty"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 4,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-multi-round-pass",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                ..PlayOptions::default()
            },
            script: &["empty", "empty", "empty", "empty", "empty"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 4,
            expected_frame_kind: "combat viewport",
        },
    ]
}

fn seed_gate_travel_resources(options: &mut PlayOptions) {
    options.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    if let Some(caster) = options.party.first_mut() {
        caster.mana = GATE_TRAVEL_COST + 1;
        caster.level = GATE_TRAVEL_COST;
    }
}

pub fn run_route_smoke(game_dir: &Path, raster_depth: TileGraphicsDepth) -> io::Result<()> {
    let cases = route_smoke_cases();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    println!("Route smoke: {} case(s).", cases.len());
    for case in &cases {
        let report = run_route_smoke_case(game_dir, &atlas, case)?;
        println!(
            "route-smoke {}: {} command(s), {}",
            report.name, report.commands_run, report.final_state_line
        );
        println!("{}", report.final_raster_line);
    }
    println!("Route smoke: all cases passed.");
    Ok(())
}

pub fn run_route_smoke_case(
    game_dir: &Path,
    atlas: &u5_runtime::TileAtlas,
    case: &RouteSmokeCase,
) -> io::Result<RouteSmokeReport> {
    let route_game_dir = prepare_route_smoke_case_game_dir(case.name)?;
    let command_game_dir = route_game_dir.as_deref().unwrap_or(game_dir);
    let mut state = PlayState::load_scene(game_dir, case.options.clone())?;
    apply_route_smoke_case_setup(&mut state, case.name, game_dir)?;
    let commands = case
        .script
        .iter()
        .map(|command| (*command).to_string())
        .collect::<Vec<_>>();
    let mut commands_run = 0;

    let initial_raster = raster_diagnostic_line(&mut state, VIEWPORT_RADIUS, atlas)?;
    require_raster_available(case, &initial_raster)?;

    let result =
        replay_play_script_commands(&mut state, command_game_dir, &commands, |state, _, _| {
            commands_run += 1;
            let raster = raster_diagnostic_line(state, VIEWPORT_RADIUS, atlas)?;
            require_raster_hash(case, &raster)
        });
    if let Some(dir) = &route_game_dir {
        let _ = fs::remove_dir_all(dir);
    }
    result?;

    if !case.expected.matches(&state) {
        return Err(io::Error::other(format!(
            "route smoke `{}` ended in `{}`; expected {}",
            case.name,
            state.current_area_label(),
            case.expected.label()
        )));
    }
    if state.turn < case.min_turn {
        return Err(io::Error::other(format!(
            "route smoke `{}` ended at turn {}; expected at least {}",
            case.name, state.turn, case.min_turn
        )));
    }
    let final_frame_kind = raster_frame_kind(&state);
    let frame_kind_matches = final_frame_kind == case.expected_frame_kind
        || (case.name == "castle-death-vision-look" && final_frame_kind == "tile viewport");
    if !frame_kind_matches {
        return Err(io::Error::other(format!(
            "route smoke `{}` ended with `{}`; expected `{}`",
            case.name, final_frame_kind, case.expected_frame_kind
        )));
    }
    validate_route_smoke_case_state(&state, case.name)?;

    let final_raster_line = raster_diagnostic_line(&mut state, VIEWPORT_RADIUS, atlas)?;
    require_raster_hash(case, &final_raster_line)?;
    Ok(RouteSmokeReport {
        name: case.name.to_string(),
        commands_run,
        final_state_line: play_script_state_line(&state),
        final_raster_line,
    })
}

fn prepare_route_smoke_case_game_dir(case_name: &str) -> io::Result<Option<PathBuf>> {
    if case_name != "castle-poison-gas-step" {
        return Ok(None);
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "u5-route-smoke-{case_name}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&dir)?;
    Ok(Some(dir))
}

fn apply_route_smoke_case_setup(
    state: &mut PlayState,
    case_name: &str,
    game_dir: &Path,
) -> io::Result<()> {
    match case_name {
        "endgame-missing-box-confirmation" => {
            state.enter_endgame();
        }
        "endgame-box-victory-confirmation" => {
            state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
            state.enter_endgame();
        }
        "blackthorn-audience-correct" | "blackthorn-audience-wrong" => {
            state.begin_blackthorn_audience_capture(game_dir)?;
        }
        "blackthorn-rescue-refuge" => {
            state.apply_blackthorn_rescue_refuge(game_dir)?;
        }
        "lycaeum-shard-falsehood-vanquish" => {
            seed_shadowlord_shard_route(state, SHADOWLORD_FALSEHOOD_INDEX, 15, 9);
        }
        "empath-shard-hatred-vanquish" => {
            seed_shadowlord_shard_route(state, SHADOWLORD_HATRED_INDEX, 15, 3);
        }
        "serpents-hold-shard-cowardice-vanquish" => {
            seed_shadowlord_shard_route(state, SHADOWLORD_COWARDICE_INDEX, 15, 16);
        }
        "underworld-fixed-hidden-stack-search-get-search" => {
            state.player.x = 232;
            state.player.y = 233;
            state.player.facing = Direction::East;
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "blackthorn-fixed-hidden-zero-key-search" => {
            state.player.x = 5;
            state.player.y = 8;
            state.player.facing = Direction::East;
            state.keys = 0;
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "minoc-fixed-hidden-daily-search-get-repeat" => {
            state.player.x = 1;
            state.player.y = 2;
            state.player.facing = Direction::East;
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "natural-moongate-trammel-gate-travel" | "natural-moongate-empty-slot-clears-live-tile" => {
            let idx = state.player.y * WORLD_SIDE + state.player.x;
            if let Some(tile) = state.grid.get_mut(idx) {
                *tile = NATURAL_MOONGATE_TERRAIN_TILE;
            }
            state.natural_moongate_live_cells = vec![idx];
            state.set_cached_moon_glyph_slots(Some(0), None);
            state.mark_visibility_dirty();
        }
        "castle-hourly-poison-starvation-pass" => {
            state.prng_state = 0x3456;
        }
        "castle-hourly-ring-regeneration-pass" => {
            state.prng_state = ring_regeneration_first_heal_seed();
        }
        "castle-poison-gas-step" => {
            seed_town_poison_gas_route(state);
        }
        "britannia-board-horse-route" => {
            seed_world_board_horse_route(state);
        }
        "castle-surface-fountain-look" => {
            stamp_town_route_look_tile(state, 0xD8);
        }
        "yew-wanted-poster-look" => {
            seed_yew_wanted_poster_route(state);
        }
        "buccaneers-den-wishing-well-horse"
        | "buccaneers-den-wishing-well-ferrari-grants-horse" => {
            stamp_town_route_look_tile(state, 0xA1);
        }
        "castle-death-vision-look" => {
            stamp_town_route_look_tile(state, 0x00);
            state.active_objects.push(ActiveObject {
                type_byte: DEATH_VISION_OBJECT_CLASS,
                tile: DEATH_VISION_OBJECT_CLASS,
                x: state.player.x + 1,
                y: state.player.y,
                z: state.current_floor().unwrap_or(0),
                phase: 0,
                aux1: 0,
                aux3: 0,
            });
        }
        "castle-talk-status-sleeping-refusal" => {
            seed_town_talk_status_tile_route(state, TALK_STATUS_TILE_SLEEPING);
        }
        "castle-talk-status-praying-refusal" => {
            seed_town_talk_status_tile_route(state, TALK_STATUS_TILE_PRAYING);
        }
        "castle-native-stair-up-route" => {
            seed_town_native_stair_route(state, Direction::East, 0xC5);
        }
        "castle-native-stair-down-route" => {
            seed_town_native_stair_route(state, Direction::East, 0xC7);
        }
        "castle-native-stair-cross-route" => {
            seed_town_native_stair_route(state, Direction::North, 0xC5);
        }
        "britannia-word-of-power-seal-opens" | "underworld-doom-word-of-power-seal-opens" => {
            stamp_word_of_power_seal_route(state, case_name);
        }
        "dungeon-ladder-down-up-route" => {
            let current = dungeon_cell_index(0, state.player.x, state.player.y);
            let below = dungeon_cell_index(1, state.player.x, state.player.y);
            if let Some(cell) = state.grid.get_mut(current) {
                *cell = 0x30;
            }
            if let Some(cell) = state.grid.get_mut(below) {
                *cell = 0x30;
            }
            state.mark_visibility_dirty();
        }
        "dungeon-heavy-door-variant-block" => {
            state.player.x = 1;
            state.player.y = 1;
            state.player.facing = Direction::East;
            let target = dungeon_cell_index(0, 2, 1);
            if let Some(cell) = state.grid.get_mut(target) {
                *cell = 0xE0;
            }
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "dungeon-surface-exit-return-world" => {
            let current = dungeon_cell_index(0, state.player.x, state.player.y);
            if let Some(cell) = state.grid.get_mut(current) {
                *cell = 0x60;
            }
            state.return_world = Some(WorldReturn {
                plane: WorldPlane::Britannia,
                x: 62,
                y: 124,
                transport: TransportState::Foot,
                timing_status: state.timing_status,
                sail_cadence: state.sail_cadence,
                sail_stall_pending: state.sail_stall_pending,
                grid: vec![0; WORLD_SIDE * WORLD_SIDE],
                active_objects: Vec::new(),
                pending_vehicle: None,
            });
            state.mark_visibility_dirty();
        }
        "dungeon-active-monster-attack-ambush" => {
            seed_dungeon_active_monster_route(state, STEADY_PHASE);
        }
        "dungeon-active-monster-contact-ambush" => {
            seed_dungeon_active_monster_route(state, 0x20);
        }
        "shop-arms-local-buy-sell-route" => {
            state.gold = 999;
            if let Some(intelligence) = state.party_intelligence.first_mut() {
                *intelligence = 20;
            }
            state.active_shop = Some(ActiveShopSession::ArmsLocal(
                ArmsShopState::Greeting,
                ArmsShop::IolosBows,
            ));
        }
        "shop-healer-heal-decline-route" => {
            state.gold = 999;
            state.active_shop = Some(ActiveShopSession::Healer(
                HealerShopState::Greeting,
                Healer::TheHealersMission,
            ));
        }
        "shop-inn-rest-decline-route" | "shop-inn-rest-accept-public-rate" => {
            state.gold = 999;
            if case_name == "shop-inn-rest-accept-public-rate" {
                if let Some(member) = state.party.first_mut() {
                    member.class_byte = b'A';
                    member.status = b'G';
                    member.hp = 10;
                    member.max_hp = 30;
                    member.mana = 0;
                }
                if let Some(intelligence) = state.party_intelligence.first_mut() {
                    *intelligence = 24;
                }
            }
            state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::for_inn(
                Inn::TheWayfarerInn,
            )));
        }
        "shop-reagent-buy-route" => {
            state.gold = 999;
            state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::for_herbalist(
                Herbalist::TheHerbalist,
            )));
        }
        "shop-tavern-drink-and-food-route" => {
            state.gold = 999;
            state.food = 0;
            state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
                Tavern::TheSwordAndKeg,
            )));
        }
        "shop-horse-trader-decline-route"
        | "shop-horse-trader-horse-and-rider-buy"
        | "shop-horse-trader-stablehouse-buy"
        | "shop-horse-trader-wishing-well-buy" => {
            let stable = horse_trader_route_stable(case_name);
            seed_town_horse_trader_route(state);
            state.gold = 999;
            state.active_shop = Some(ActiveShopSession::HorseTrader(
                HorseTraderState::for_stable(stable),
            ));
        }
        "shop-shipwright-quote-decline-route" => {
            state.gold = 999;
            state.return_world = Some(WorldReturn {
                plane: WorldPlane::Britannia,
                x: state.player.x,
                y: state.player.y,
                transport: state.player.transport,
                timing_status: state.timing_status,
                sail_cadence: state.sail_cadence,
                sail_stall_pending: state.sail_stall_pending,
                grid: state.grid.clone(),
                active_objects: state.active_objects.clone(),
                pending_vehicle: None,
            });
            state.active_shop = Some(ActiveShopSession::ShipBroker(
                ShipBrokerState::for_shipwright(Shipwright::IslandShipwrights),
            ));
        }
        "shop-guild-buy-route" => {
            state.gold = 999;
            state.active_shop = Some(ActiveShopSession::Guild(GuildShopState::for_shop(
                GuildShop::TheGuild,
            )));
        }
        "shop-sage-topic-miss-route" => {
            state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
        }
        "shop-sage-topic-paid-success-route" => {
            state.gold = 100;
            state.prng_state = 0x3456;
            state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
        }
        "shop-sage-topic-short-funds-route" => {
            state.gold = 49;
            state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
        }
        _ => {}
    }
    Ok(())
}

fn seed_shadowlord_shard_route(state: &mut PlayState, index: usize, x: usize, y: usize) {
    state.player.x = x;
    state.player.y = y;
    state.player.facing = Direction::South;
    state.sync_player_object();
    let tile = SHADOWLORD_OBJECT_TILE_BASE + index as u8;
    let floor = state.current_floor().unwrap_or(0);
    state.active_objects.push(ActiveObject {
        type_byte: tile,
        tile,
        x,
        y: y.saturating_sub(1),
        z: floor,
        phase: STEADY_PHASE,
        aux1: index as u8,
        aux3: state.shadowlord_hideouts.get(index).copied().unwrap_or(0),
    });
    state.mark_visibility_dirty();
}

fn seed_dungeon_active_monster_route(state: &mut PlayState, phase: u8) {
    state.player.x = 1;
    state.player.y = 1;
    state.player.facing = Direction::East;
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 0xC0,
        tile: 0xC0,
        x: 2,
        y: 1,
        z: state.current_floor().unwrap_or(0),
        phase,
        aux1: 0,
        aux3: 0,
    });
    state.mark_visibility_dirty();
}

fn word_of_power_seal_for_case(case_name: &str) -> Option<WordOfPowerSeal> {
    match case_name {
        "britannia-word-of-power-seal-opens" => word_of_power_seal_for_word("FALLAX"),
        "underworld-doom-word-of-power-seal-opens" => word_of_power_seal_for_word("VERAMOCOR"),
        _ => None,
    }
}

fn stamp_word_of_power_seal_route(state: &mut PlayState, case_name: &str) {
    if let Some(seal) = word_of_power_seal_for_case(case_name) {
        if let Area::World { plane } = state.area {
            if plane == seal.plane {
                state.player.x = seal.x;
                state.player.y = seal.y;
                state.sync_player_object();
                let idx = world_cell_index(seal.x, seal.y);
                if let Some(cell) = state.grid.get_mut(idx) {
                    *cell = seal.closed_tile;
                }
                state.mark_visibility_dirty();
            }
        }
    }
}

fn seed_town_talk_status_tile_route(state: &mut PlayState, status_tile: u8) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = Direction::East;
    state.sync_player_object();

    let mut schedule = [0u8; 16];
    schedule[3..6].copy_from_slice(&[16, 16, 16]);
    schedule[6..9].copy_from_slice(&[15, 15, 15]);
    schedule[12..16].copy_from_slice(&[0, 8, 16, 20]);
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot: 1,
            type_byte: 1,
            dialog_id: 0x84,
            schedule,
            name: None,
        },
    ]);
    if let Some(slot) = state.npcs.first().and_then(|npc| npc.active_object) {
        if let Some(object) = state.active_objects.get_mut(slot) {
            object.type_byte = 1;
            object.tile = status_tile;
        }
    }
    state.mark_visibility_dirty();
}

fn seed_town_native_stair_route(state: &mut PlayState, facing: Direction, stair_tile: u8) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = facing;
    let (dx, dy) = facing.delta();
    let target_x = (state.player.x as isize + dx) as usize;
    let target_y = (state.player.y as isize + dy) as usize;
    let target_idx = target_y * TOWN_GRID_SIDE + target_x;
    if let Some(cell) = state.grid.get_mut(target_idx) {
        *cell = stair_tile;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn stamp_town_route_look_tile(state: &mut PlayState, tile: u8) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = Direction::East;
    let target_idx = state.player.y * TOWN_GRID_SIDE + state.player.x + 1;
    if let Some(cell) = state.grid.get_mut(target_idx) {
        *cell = tile;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_yew_wanted_poster_route(state: &mut PlayState) {
    state.player.x = 16;
    state.player.y = 21;
    state.player.facing = Direction::East;
    let floor = state.current_floor().unwrap_or(0);
    let target_x = 17;
    let target_y = 21;
    state.active_objects.retain(|object| {
        object.is_empty() || object.x != target_x || object.y != target_y || object.z != floor
    });
    state.active_objects.push(ActiveObject {
        type_byte: 0xA0,
        tile: 0xA0,
        x: target_x,
        y: target_y,
        z: floor,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_town_poison_gas_route(state: &mut PlayState) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = Direction::East;
    let floor = state.current_floor().unwrap_or(0);
    let target_x = state.player.x + 1;
    let target_y = state.player.y;
    let target_idx = target_y * TOWN_GRID_SIDE + target_x;
    if let Some(cell) = state.grid.get_mut(target_idx) {
        *cell = TOWN_POISON_GAS_LIVE_TILE;
    }
    for object in &mut state.active_objects {
        if !object.is_empty() && object.x == target_x && object.y == target_y && object.z == floor {
            *object = ActiveObject::empty();
        }
    }
    if let Some(member) = state.party.get_mut(1) {
        member.climb_stat = 0;
    }
    state.prng_state = poison_gas_first_poison_seed();
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_town_horse_trader_route(state: &mut PlayState) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = Direction::South;
    let target_idx = (state.player.y + 1) * TOWN_GRID_SIDE + state.player.x;
    if let Some(cell) = state.grid.get_mut(target_idx) {
        *cell = 0x05;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn horse_trader_route_stable(case_name: &str) -> Stable {
    match case_name {
        "shop-horse-trader-stablehouse-buy" => Stable::TheStablehouse,
        "shop-horse-trader-wishing-well-buy" => Stable::WishingWellHorses,
        _ => Stable::HorseAndRider,
    }
}

fn seed_world_board_horse_route(state: &mut PlayState) {
    state.player.x = 62;
    state.player.y = 124;
    state.player.facing = Direction::East;
    state.player.transport = TransportState::Foot;
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 63,
        y: 124,
        z: WorldPlane::Britannia.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    state.mark_visibility_dirty();
}

fn poison_gas_first_poison_seed() -> u16 {
    for candidate in 0..=u16::MAX {
        let mut state = candidate;
        if u5_prng_range_u16(&mut state, 0, TOWN_GAS_DOORWAY_RANGE_MAX) > 0 {
            return candidate;
        }
    }
    unreachable!("PRNG range cycle must hit a poison roll")
}

fn ring_regeneration_first_heal_seed() -> u16 {
    for candidate in 0..=u16::MAX {
        let mut state = candidate;
        if u5_prng_range_u16(&mut state, 0, 7) == 0 {
            return candidate;
        }
    }
    unreachable!("PRNG range cycle must hit a ring regeneration roll")
}

fn route_party_member(slot: u8, class_byte: u8, status: u8, hp: u16, max_hp: u16) -> PartyMember {
    PartyMember {
        slot,
        class_byte,
        status,
        climb_stat: 30,
        mana: 8,
        hp,
        max_hp,
        level: 8,
    }
}

fn validate_route_smoke_case_state(state: &PlayState, case_name: &str) -> io::Result<()> {
    match case_name {
        "britannia-create-food-cast" => {
            let max_expected = DEFAULT_FOOD_STOCK.saturating_add(CREATE_FOOD_MAX_GRANT);
            if !(DEFAULT_FOOD_STOCK..=max_expected).contains(&state.food)
                || state.spell_charges[CREATE_FOOD_SPELL_INDEX] != 0
                || !state.message.contains("Created")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply bounded Create Food result"
                )));
            }
        }
        "britannia-blink-east-ray" => {
            if state.player.x <= 62
                || state.player.y != 124
                || state.spell_charges[BLINK_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || !state.message.contains("Blinked East")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the public Blink ray rule"
                )));
            }
        }
        "castle-hourly-provision-poison-pass" => {
            if state.clock.hour != 6
                || state.food != 8
                || state.party.get(0).is_none_or(|member| member.hp != 12)
                || state
                    .party
                    .get(1)
                    .is_none_or(|member| member.hp != 11 || member.status != b'P')
                || state.party.get(2).is_none_or(|member| member.hp != 12)
                || state.message.contains("Starving!")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply meal-hour provisions plus poison"
                )));
            }
        }
        "castle-hourly-poison-starvation-pass" => {
            let mut expected_prng = 0x3456;
            let poisoned_starvation = u5_prng_range_u16(
                &mut expected_prng,
                HOURLY_STARVATION_DAMAGE_MIN,
                HOURLY_STARVATION_DAMAGE_MAX,
            );
            let good_starvation = u5_prng_range_u16(
                &mut expected_prng,
                HOURLY_STARVATION_DAMAGE_MIN,
                HOURLY_STARVATION_DAMAGE_MAX,
            );
            let poisoned_hp =
                20u16 - u16::from(FIRST_PLAYABLE_HOURLY_POISON_DAMAGE) - poisoned_starvation;
            let good_hp = 20u16 - good_starvation;
            if state.clock.hour != 9
                || state.food != 0
                || state.prng_state != expected_prng
                || state
                    .party
                    .get(0)
                    .is_none_or(|member| member.hp != poisoned_hp || member.status != b'P')
                || state
                    .party
                    .get(1)
                    .is_none_or(|member| member.hp != good_hp || member.status != b'G')
                || state
                    .party
                    .get(2)
                    .is_none_or(|member| member.hp != 0 || member.status != b'D')
                || !state.message.contains("Starving! starvation damage")
                || !state
                    .message
                    .contains(&format!("party slot 0 took {poisoned_starvation} HP"))
                || !state
                    .message
                    .contains(&format!("party slot 1 took {good_starvation} HP"))
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply public poison-before-starvation rolls"
                )));
            }
        }
        "castle-hourly-ring-regeneration-pass" => {
            let mut expected_prng = ring_regeneration_first_heal_seed();
            let roll = u5_prng_range_u16(&mut expected_prng, 0, 7);
            if roll != 0
                || state.clock.hour != 8
                || state.prng_state != expected_prng
                || state.party.first().is_none_or(|member| {
                    member.status != b'G' || member.hp != member.max_hp || member.mana != 8
                })
                || state.party_equipment.first().is_none_or(|equipment| {
                    equipment[EQUIP_SLOT_RING] != EQUIPMENT_ID_RING_REGENERATION as u8
                })
                || !state.message.contains("Pass")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply hourly Ring of Regeneration"
                )));
            }
        }
        "castle-poison-gas-step" => {
            let mut expected_prng = poison_gas_first_poison_seed();
            let roll = u5_prng_range_u16(&mut expected_prng, 0, TOWN_GAS_DOORWAY_RANGE_MAX);
            if roll == 0
                || state.prng_state != expected_prng
                || state.player.x != 16
                || state.player.y != 15
                || state
                    .party
                    .get(0)
                    .is_none_or(|member| member.status != b'P')
                || state
                    .party
                    .get(1)
                    .is_none_or(|member| member.status != b'P')
                || state
                    .party
                    .get(2)
                    .is_none_or(|member| member.status != b'P')
                || !state
                    .message
                    .contains("poison gas doorway: poisoned party slot 1")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply public #51 poison-gas roll semantics"
                )));
            }
        }
        "blackthorn-fixed-hidden-zero-key-search" => {
            if state.keys != 0
                || !state.fixed_hidden_treasure_found(13)
                || !state
                    .active_objects
                    .iter()
                    .any(|object| object.fixed_hidden_treasure_record() == Some(13))
                || !state.message.contains("Found ring of keys")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the fixed hidden zero-key cache rule"
                )));
            }
        }
        "castle-mix-ready-order-route" => {
            if state.spell_charges[IN_LOR_SPELL_INDEX] != 1
                || state.reagents[REAGENT_SULFUR_ASH] != 1
                || state.equipment_stock[EQUIPMENT_ID_BOW] != 1
                || state
                    .party_equipment
                    .first()
                    .is_none_or(|equipment| equipment[EQUIP_SLOT_WEAPON] != EQUIPMENT_EMPTY)
                || state.party.get(1).is_none_or(|member| member.slot != 2)
                || state.party.get(2).is_none_or(|member| member.slot != 1)
                || !state.message.contains("party slots 2 and 3 swapped")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete mix, Ready, and New Order workflow"
                )));
            }
        }
        "britannia-board-horse-route" => {
            if !matches!(state.player.transport, TransportState::Horse { .. })
                || state.player.x != 62
                || state.player.y != 124
                || !state.message.contains("horse")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not board the adjacent horse"
                )));
            }
        }
        "ship-broadside-fire-route" => {
            if !matches!(state.player.transport, TransportState::Ship { .. })
                || !state.message.contains("BOOOM! Ship broadside fired")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not fire the ship broadside"
                )));
            }
        }
        "castle-surface-fountain-look" => {
            if !state.message.contains("fountain") || !state.message.contains("feels refreshed") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the fountain Look flow"
                )));
            }
        }
        "yew-wanted-poster-look" => {
            if !state.message.contains("Wanted Poster") || !state.message.contains("Avatar") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not render the hard-coded Yew wanted poster"
                )));
            }
        }
        "buccaneers-den-wishing-well-horse" => {
            if state.gold != 4
                || !state.message.contains("horse appears")
                || !state
                    .active_objects
                    .iter()
                    .any(|object| object.type_byte == HORSE_PARKED_FIRST)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not consume a coin and spawn a horse"
                )));
            }
        }
        "buccaneers-den-wishing-well-ferrari-grants-horse" => {
            if state.gold != 4
                || !state.message.contains("horse appears")
                || !state
                    .active_objects
                    .iter()
                    .any(|object| object.type_byte == HORSE_PARKED_FIRST)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not map Ferrari to the horse-family grant"
                )));
            }
        }
        "castle-death-vision-look" => {
            if !state.message.contains("Strange vision") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the death-vision Look flow"
                )));
            }
        }
        "castle-talk-status-sleeping-refusal" => {
            if state.message != TALK_SLEEPING_MESSAGE || state.active_shop.is_some() {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the sleeping Talk refusal"
                )));
            }
        }
        "castle-talk-status-praying-refusal" => {
            if state.message != TALK_NO_RESPONSE_MESSAGE || state.active_shop.is_some() {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the praying Talk refusal"
                )));
            }
        }
        "castle-native-stair-up-route" => {
            if state.current_floor() != Some(1)
                || state.player.x != 16
                || state.player.y != 15
                || !state.message.contains("floor 1")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the native walk-on stair up transition"
                )));
            }
        }
        "castle-native-stair-down-route" => {
            if state.current_floor() != Some(0)
                || state.player.x != 16
                || state.player.y != 15
                || !state.message.contains("floor 0")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the native walk-on stair down transition"
                )));
            }
        }
        "castle-native-stair-cross-route" => {
            if state.current_floor() != Some(0)
                || state.player.x != 15
                || state.player.y != 14
                || !state.message.contains("Moved to")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not treat side-crossing a native stair as ordinary movement"
                )));
            }
        }
        "britannia-word-of-power-seal-opens" | "underworld-doom-word-of-power-seal-opens" => {
            let Some(seal) = word_of_power_seal_for_case(case_name) else {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` has no Word-of-Power seal row"
                )));
            };
            let idx = world_cell_index(seal.x, seal.y);
            if state.grid.get(idx).copied() != Some(seal.closed_tile ^ WORD_OF_POWER_SEAL_XOR)
                || state.player.x != seal.x
                || state.player.y != seal.y
                || !state.message.contains("The seal opens.")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not open the public Word-of-Power seal"
                )));
            }
        }
        "lycaeum-shard-falsehood-vanquish" => {
            validate_shadowlord_shard_route(
                state,
                case_name,
                SHADOWLORD_FALSEHOOD_INDEX,
                SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
                "Falsehood vanquished",
            )?;
        }
        "empath-shard-hatred-vanquish" => {
            validate_shadowlord_shard_route(
                state,
                case_name,
                SHADOWLORD_HATRED_INDEX,
                SPECIAL_ITEM_SHARD_HATRED_INDEX,
                "Hatred vanquished",
            )?;
        }
        "serpents-hold-shard-cowardice-vanquish" => {
            validate_shadowlord_shard_route(
                state,
                case_name,
                SHADOWLORD_COWARDICE_INDEX,
                SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
                "Cowardice vanquished",
            )?;
        }
        "shop-inn-rest-accept-public-rate" => {
            let raw = inn_base_room_rate(Inn::TheWayfarerInn) * state.party.len() as u16;
            let speaker_intelligence = state.party_intelligence.first().copied().unwrap_or(0);
            let expected_gold = 999 - shop_intelligence_adjusted_price(raw, speaker_intelligence);
            let inn_recovery_applied = state.party.first().is_some_and(|member| {
                member.hp == member.max_hp && member.mana == 24 && member.status == b'G'
            });
            if state.gold != expected_gold
                || !inn_recovery_applied
                || !state.message.contains("Rested 8 hours at the inn")
                || !state.message.contains("recovered 20 HP and 24 MP")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the public inn-rest outcome"
                )));
            }
        }
        "shop-sage-topic-paid-success-route" => {
            if state.gold != 50
                || state.active_shop.is_some()
                || !state.message.contains("Malik")
                || !state.message.contains("Moonglow")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not debit and render the public sage topic"
                )));
            }
        }
        "shop-sage-topic-short-funds-route" => {
            if state.gold != 49
                || state.active_shop.is_none()
                || state.message != TAVERN_AFFORDABILITY_REFUSAL_BARK
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve gold on sage short funds"
                )));
            }
        }
        "shop-horse-trader-horse-and-rider-buy"
        | "shop-horse-trader-stablehouse-buy"
        | "shop-horse-trader-wishing-well-buy" => {
            let stable = horse_trader_route_stable(case_name);
            let raw = stable_horse_price(stable);
            let speaker_intelligence = state.party_intelligence.first().copied().unwrap_or(0);
            let expected_gold = 999 - shop_intelligence_adjusted_price(raw, speaker_intelligence);
            let horse = state
                .active_objects
                .iter()
                .find(|object| object.type_byte == HORSE_PARKED_FIRST);
            let boardable = state.boardable_vehicle_slot_at(15, 16).is_some();
            if state.gold != expected_gold
                || state.active_shop.is_some()
                || horse.is_none_or(|object| object.x != 15 || object.y != 16)
                || !boardable
                || !state.message.contains("Thy horse awaits outside")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the public horse-trader sale"
                )));
            }
        }
        "castle-light-decay-route" => {
            if state.light_spell_counter != 0 || !state.message.contains("Pass") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not age light counters through turns"
                )));
            }
        }
        "dungeon-hole-up-no-direct-recovery" => {
            if state.clock.hour != 9
                || state.party.get(0).is_none_or(|member| {
                    member.status != b'G' || member.hp != 5 || member.mana != 8
                })
                || state.party.get(1).is_none_or(|member| {
                    member.status != b'G' || member.hp != 3 || member.mana != 8
                })
                || state
                    .party
                    .get(2)
                    .is_none_or(|member| member.status != b'D' || member.hp != 0)
                || !state.message.contains("recovered 0 HP and 0 MP")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve no-direct-recovery rest behavior"
                )));
            }
        }
        "dungeon-ignite-torch-route" => {
            if state.torch_counter == 0 || !state.message.contains("Ignited a torch") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not light a dungeon torch"
                )));
            }
        }
        "dungeon-ladder-down-up-route" => {
            if state.current_floor() != Some(0) || !state.message.contains("level 0") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the down/up ladder chain"
                )));
            }
        }
        "dungeon-heavy-door-variant-block" => {
            if state.player.x != 1
                || state.player.y != 1
                || state.turn != 0
                || state.message != "Blocked!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not block the public 0xE? heavy-door variant"
                )));
            }
        }
        "dungeon-surface-exit-return-world" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Britannia
                }
            ) || state.player.transport != TransportState::Foot
                || state.player.x != 62
                || state.player.y != 124
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not restore the saved overworld return"
                )));
            }
        }
        "dungeon-active-monster-attack-ambush" | "dungeon-active-monster-contact-ambush" => {
            if !state.combat_active
                || !state.message.contains("entered dungeon combat")
                || state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].tile != 0xC0
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].x != 6
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].y != 5
                || !state.combat_terrain.iter().all(|row| {
                    row.iter()
                        .all(|tile| *tile == DUNGEON_AMBUSH_ARENA_FLOOR_TILE)
                })
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not enter the public #21 dungeon ambush frame"
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_shadowlord_shard_route(
    state: &PlayState,
    case_name: &str,
    shadowlord_index: usize,
    item_index: usize,
    message_fragment: &str,
) -> io::Result<()> {
    let shadowlord_tile = SHADOWLORD_OBJECT_TILE_BASE + shadowlord_index as u8;
    if state.shadowlord_hideouts.get(shadowlord_index).copied() != Some(SHADOWLORD_VANQUISHED)
        || state.special_items.get(item_index).copied() != Some(0)
        || state
            .active_objects
            .iter()
            .any(|object| object.type_byte == shadowlord_tile && object.tile == shadowlord_tile)
        || !state.message.contains(message_fragment)
    {
        return Err(io::Error::other(format!(
            "route smoke `{case_name}` did not complete native shard destruction"
        )));
    }
    Ok(())
}

fn require_raster_hash(case: &RouteSmokeCase, raster: &str) -> io::Result<()> {
    if case.name == "castle-death-vision-look"
        && (raster.contains("tile viewport") || raster.contains("view overlay"))
        && raster.contains(" hash ")
    {
        return Ok(());
    }
    if !raster.contains(case.expected_frame_kind) || !raster.contains(" hash ") {
        return Err(io::Error::other(format!(
            "route smoke `{}` produced weak raster diagnostic: {raster}",
            case.name
        )));
    }
    Ok(())
}

fn require_raster_available(case: &RouteSmokeCase, raster: &str) -> io::Result<()> {
    if !raster.contains(" hash ") {
        return Err(io::Error::other(format!(
            "route smoke `{}` produced weak initial raster diagnostic: {raster}",
            case.name
        )));
    }
    Ok(())
}

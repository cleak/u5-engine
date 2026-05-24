//! Route-level smoke suite for local clean assets.
//!
//! These cases intentionally exercise public harness routes and sidecar-backed
//! transitions without asserting copyrighted text content.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use u5_runtime::{
    AWAKEN_COST, AWAKEN_SPELL_INDEX, ActiveObject, Area, ArmsShop, BLACKTHORN_CAPTIVE_CELL_SCENE,
    BLACKTHORN_RESCUE_HANDOFF_SCENE, BLINK_COST, BLINK_SPELL_INDEX,
    COMBAT_ACTOR_FLAG_SELECTABLE_80, COMBAT_ACTOR_SLOTS, COMBAT_ARENA_SIDE, COMBAT_CLASS_GIANT_RAT,
    COMBAT_PARTY_ACTOR_SLOTS, CREATE_FOOD_COST, CREATE_FOOD_MAX_GRANT, CREATE_FOOD_SPELL_INDEX,
    CURE_COST, CURE_SPELL_INDEX, CombatActorDescriptor, CombatArenaFieldKind,
    DEATH_VISION_OBJECT_CLASS, DEATH_WIND_COST, DEATH_WIND_SPELL_INDEX, DEFAULT_FOOD_STOCK,
    DES_POR_SPELL_INDEX, DISPEL_FIELD_COST, DISPEL_FIELD_SPELL_INDEX,
    DUNGEON_AMBUSH_ARENA_FLOOR_TILE, DUNGEON_LEVEL_SPELL_COST, Direction, DungeonScene,
    ENERGY_FIELD_COST, ENERGY_FIELD_SPELL_INDEX, EQUIP_SLOT_RING, EQUIP_SLOT_WEAPON,
    EQUIPMENT_EMPTY, EQUIPMENT_ID_ARROWS, EQUIPMENT_ID_BOW, EQUIPMENT_ID_RING_REGENERATION,
    EndgameOutcome, FIELD_SPELL_COST, FIRE_FIELD_SPELL_INDEX, FIRST_PLAYABLE_FRIGATE_TILE,
    FIRST_PLAYABLE_FULL_SHIP_HULL, FIRST_PLAYABLE_HOURLY_POISON_DAMAGE, FLAME_WIND_COST,
    FLAME_WIND_SPELL_INDEX, GATE_TRAVEL_COST, GATE_TRAVEL_SPELL_INDEX, GREAT_HEAL_COST,
    GREAT_HEAL_SPELL_INDEX, GameClock, GuildShop, HEAL_COST, HEAL_SPELL_INDEX, HORSE_PARKED_FIRST,
    HOURLY_STARVATION_DAMAGE_MAX, HOURLY_STARVATION_DAMAGE_MIN, Healer, Herbalist,
    IN_LOR_SPELL_INDEX, IN_WIS_COST, IN_WIS_SPELL_INDEX, Inn, MoonstoneGateSlot,
    NATURAL_MOONGATE_TERRAIN_TILE, NEGATE_MAGIC_COST, NEGATE_MAGIC_SPELL_INDEX,
    NEGATE_TIME_ACTIVE_EFFECT_TAG, NpcSlot, OPEN_SPELL_COST, OPEN_SPELL_INDEX, PEER_COST,
    PEER_SPELL_INDEX, POISON_FIELD_SPELL_INDEX, POISON_WIND_COST, POISON_WIND_SPELL_INDEX,
    PROTECTION_COST, PROTECTION_SPELL_INDEX, PartyMember, PendingVehicleAcquisition, PlayOptions,
    PlayState, PlayTarget, QUICKNESS_COST, QUICKNESS_SPELL_INDEX, REAGENT_SULFUR_ASH,
    RESURRECT_COST, RESURRECT_SPELL_INDEX, SCENE_EMPATH_ABBEY, SCENE_JHELOM, SCENE_MOONGLOW,
    SCENE_SERPENTS_HOLD, SCENE_STONEGATE, SCENE_THE_LYCAEUM, SHADOWLORD_COWARDICE_INDEX,
    SHADOWLORD_FALSEHOOD_INDEX, SHADOWLORD_HATRED_INDEX, SHADOWLORD_HIDEOUT_VANQUISHED,
    SHADOWLORD_OBJECT_TILE_BASE, SHADOWLORD_VANQUISHED, SLEEP_COST, SLEEP_FIELD_SPELL_INDEX,
    SLEEP_SPELL_INDEX, SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX, SPECIAL_ITEM_MAGIC_CARPET_INDEX,
    SPECIAL_ITEM_OWNED_VALUE, SPECIAL_ITEM_POCKET_WATCH_INDEX, SPECIAL_ITEM_SCEPTRE_LB_INDEX,
    SPECIAL_ITEM_SEXTANT_INDEX, SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
    SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX, SPECIAL_ITEM_SHARD_HATRED_INDEX,
    SPECIAL_ITEM_SPYGLASS_INDEX, SPECIAL_ITEM_WOODEN_BOX_INDEX, STEADY_PHASE, SURFACE_CHASM_X,
    SURFACE_CHASM_Y, Scene, Shipwright, ShipwrightPurchaseKind, Stable, TALK_NO_RESPONSE_MESSAGE,
    TALK_SLEEPING_MESSAGE, TALK_STATUS_TILE_PRAYING, TALK_STATUS_TILE_SLEEPING,
    TAVERN_AFFORDABILITY_REFUSAL_BARK, TIME_STOP_COST, TIME_STOP_DURATION, TIME_STOP_SPELL_INDEX,
    TOWN_GAS_DOORWAY_RANGE_MAX, TOWN_GRID_SIDE, TOWN_POISON_GAS_LIVE_TILE, Tavern,
    TileGraphicsDepth, TransportState, UUS_POR_SPELL_INDEX, VAS_LOR_COST, VAS_LOR_SPELL_INDEX,
    WORD_OF_POWER_SEAL_XOR, WORLD_SIDE, WindState, WordOfPowerSeal, WorldPlane, WorldReturn,
    X_RAY_COST, X_RAY_SPELL_INDEX, combat_class_stats, default_party_equipment,
    default_party_experience, default_party_intelligence, default_party_names,
    default_party_stay_counters, dungeon_cell_index, inn_base_room_rate, load_tile_atlas,
    shipwright_delivery_coordinate, shipwright_price, shop_intelligence_adjusted_price,
    shop_runtime::{
        ArmsShopState, GuildShopState, HealerShopState, HorseTraderState, InnkeeperState,
        ReagentShopState, SageState, ShipBrokerState, TavernState,
    },
    shop_session::ActiveShopSession,
    spell_index_from_code, spell_mp_cost, stable_horse_price, summoned_active_object_record,
    u5_prng_range_u16, word_of_power_seal_for_word, world_cell_index,
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
    Endgame(EndgameOutcome),
}

impl RouteSmokeExpectation {
    fn matches(self, state: &PlayState) -> bool {
        match (self, state.area) {
            (Self::World(expected), Area::World { plane }) => expected == plane,
            (Self::Town(expected), Area::Town { scene, .. }) => expected == scene,
            (Self::Dungeon(expected), Area::Dungeon { scene, .. }) => expected == scene,
            (Self::Endgame(expected), _) => {
                state.endgame.as_ref().and_then(|endgame| endgame.outcome) == Some(expected)
            }
            _ => false,
        }
    }

    fn label(self) -> String {
        match self {
            Self::World(plane) => plane.key().to_string(),
            Self::Town(scene) => scene.key(),
            Self::Dungeon(scene) => scene.key(),
            Self::Endgame(EndgameOutcome::Victory) => "endgame victory".to_string(),
            Self::Endgame(EndgameOutcome::MissingBoxOrRefused) => {
                "endgame missing-box/refusal".to_string()
            }
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

    let mut locate = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((62, 124)),
        ..PlayOptions::default()
    };
    locate.spell_charges[IN_WIS_SPELL_INDEX] = 1;
    locate.party[0].mana = IN_WIS_COST;
    locate.party[0].level = IN_WIS_COST;

    let mut light_open = PlayOptions::default();
    light_open.spell_charges[VAS_LOR_SPELL_INDEX] = 1;
    light_open.spell_charges[OPEN_SPELL_INDEX] = 1;
    light_open.party[0].mana = VAS_LOR_COST + OPEN_SPELL_COST;
    light_open.party[0].level = VAS_LOR_COST.max(OPEN_SPELL_COST);

    let mut restore_spells = PlayOptions {
        party: vec![
            route_party_member(0, b'A', b'G', 20, 20),
            route_party_member(1, b'F', b'S', 8, 24),
            route_party_member(2, b'M', b'P', 6, 30),
            route_party_member(3, b'B', b'D', 0, 19),
        ],
        party_names: default_party_names(4),
        party_experience: vec![0, 0, 0, 350],
        party_stay_counters: default_party_stay_counters(4),
        party_strengths: vec![30; 4],
        party_intelligence: default_party_intelligence(4),
        party_equipment: default_party_equipment(4),
        moral_standing: 99,
        ..PlayOptions::default()
    };
    restore_spells.party[0].mana =
        AWAKEN_COST + CURE_COST + HEAL_COST + GREAT_HEAL_COST + RESURRECT_COST;
    restore_spells.party[0].level = RESURRECT_COST;
    restore_spells.spell_charges[AWAKEN_SPELL_INDEX] = 1;
    restore_spells.spell_charges[CURE_SPELL_INDEX] = 1;
    restore_spells.spell_charges[HEAL_SPELL_INDEX] = 1;
    restore_spells.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;
    restore_spells.spell_charges[RESURRECT_SPELL_INDEX] = 1;

    let mut active_effect_spells = PlayOptions::default();
    active_effect_spells.spell_charges[PROTECTION_SPELL_INDEX] = 1;
    active_effect_spells.spell_charges[QUICKNESS_SPELL_INDEX] = 1;
    active_effect_spells.spell_charges[NEGATE_MAGIC_SPELL_INDEX] = 1;
    active_effect_spells.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
    active_effect_spells.party[0].mana =
        PROTECTION_COST + QUICKNESS_COST + NEGATE_MAGIC_COST + TIME_STOP_COST;
    active_effect_spells.party[0].level = TIME_STOP_COST;

    let mut dungeon_level_spells = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 1,
        ..PlayOptions::default()
    };
    dungeon_level_spells.spell_charges[UUS_POR_SPELL_INDEX] = 1;
    dungeon_level_spells.spell_charges[DES_POR_SPELL_INDEX] = 1;
    dungeon_level_spells.party[0].mana = DUNGEON_LEVEL_SPELL_COST * 2;
    dungeon_level_spells.party[0].level = DUNGEON_LEVEL_SPELL_COST;

    let mut dungeon_field_cycle = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        facing: Some(Direction::East),
        ..PlayOptions::default()
    };
    dungeon_field_cycle.spell_charges[FIRE_FIELD_SPELL_INDEX] = 1;
    dungeon_field_cycle.spell_charges[POISON_FIELD_SPELL_INDEX] = 1;
    dungeon_field_cycle.spell_charges[SLEEP_FIELD_SPELL_INDEX] = 1;
    dungeon_field_cycle.spell_charges[ENERGY_FIELD_SPELL_INDEX] = 1;
    dungeon_field_cycle.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 4;
    dungeon_field_cycle.party[0].mana =
        FIELD_SPELL_COST * 3 + ENERGY_FIELD_COST + DISPEL_FIELD_COST * 4;
    dungeon_field_cycle.party[0].level = ENERGY_FIELD_COST.max(DISPEL_FIELD_COST);

    let mut dungeon_open_chest = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        ..PlayOptions::default()
    };
    dungeon_open_chest.spell_charges[OPEN_SPELL_INDEX] = 1;
    dungeon_open_chest.party[0].mana = OPEN_SPELL_COST;
    dungeon_open_chest.party[0].level = OPEN_SPELL_COST;

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
            name: "britannia-locate-cast",
            options: locate,
            script: &["C1IW"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-light-open-spell-route",
            options: light_open,
            script: &["C1LV", "C1AS6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-restore-spell-suite",
            options: restore_spells,
            script: &["C1AZ", "C1AN3", "C1M3", "C1MV3", "C1CIM4"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 5,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-active-effect-spell-suite",
            options: active_effect_spells,
            script: &["C1IS", "C1RT", "C1AI", "C1AT"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 4,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "dungeon-level-up-down-spells",
            options: dungeon_level_spells,
            script: &["C1PU", "C1DP"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 2,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-field-cycle-spells",
            options: dungeon_field_cycle,
            script: &[
                "C1FGI6", "C1AG6", "C1GIN6", "C1AG6", "C1GIZ6", "C1AG6", "C1GIS6", "C1AG6",
            ],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 8,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-open-chest-spell",
            options: dungeon_open_chest,
            script: &["C1AS"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "dungeon first-person viewport",
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
            expected: RouteSmokeExpectation::Endgame(EndgameOutcome::MissingBoxOrRefused),
            min_turn: 0,
            expected_frame_kind: "endgame tableau",
        },
        RouteSmokeCase {
            name: "endgame-missing-box-terminal-jitter",
            options: PlayOptions::default(),
            script: &["Y", "Y", "empty", "empty", "empty", "empty"],
            expected: RouteSmokeExpectation::Endgame(EndgameOutcome::MissingBoxOrRefused),
            min_turn: 0,
            expected_frame_kind: "endgame tableau",
        },
        RouteSmokeCase {
            name: "endgame-box-victory-confirmation",
            options: PlayOptions::default(),
            script: &["Y", "Y", "empty"],
            expected: RouteSmokeExpectation::Endgame(EndgameOutcome::Victory),
            min_turn: 0,
            expected_frame_kind: "endgame tableau",
        },
        RouteSmokeCase {
            name: "endgame-box-full-victory-cinematic",
            options: PlayOptions::default(),
            script: &[
                "Y", "Y", "empty", "empty", "empty", "empty", "empty", "empty", "empty", "empty",
                "empty", "empty", "empty", "empty", "empty", "empty", "empty", "empty", "empty",
                "empty", "empty", "empty", "empty", "empty", "empty", "empty", "empty", "empty",
                "empty", "empty", "empty", "empty", "empty", "empty", "empty", "empty", "empty",
                "empty", "empty", "empty", "empty", "empty", "empty", "empty",
            ],
            expected: RouteSmokeExpectation::Endgame(EndgameOutcome::Victory),
            min_turn: 0,
            expected_frame_kind: "endgame tableau",
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
            script: &["Y", "M", "R", "1", "N"],
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
            name: "shop-shipwright-island-frigate-buy",
            options: world.clone(),
            script: &["F", "Y"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-shipwright-crows-nest-skiff-buy",
            options: world.clone(),
            script: &["S", "Y"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-shipwright-oaken-oar-frigate-buy",
            options: world.clone(),
            script: &["F", "Y"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-shipwright-rusty-bucket-skiff-buy",
            options: world.clone(),
            script: &["S", "Y"],
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
            name: "dungeon-heavy-door-variant-pass-through",
            options: dungeon_options.clone(),
            script: &["."],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
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
            name: "combat-directed-sleep-cone",
            options: world.clone(),
            script: &["C1IZ6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-directed-poison-wind-cone",
            options: world.clone(),
            script: &["C1HIN6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-directed-death-wind-cone",
            options: world.clone(),
            script: &["C1CGIV6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-directed-flame-wind-cone",
            options: world.clone(),
            script: &["C1FHI6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-fire-marker-placement",
            options: world.clone(),
            script: &["C1FGI6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-poison-marker-placement",
            options: world.clone(),
            script: &["C1GIN6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-sleep-marker-placement",
            options: world.clone(),
            script: &["C1GIZ6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-energy-marker-placement",
            options: world.clone(),
            script: &["C1GIS6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-magic-missile-target",
            options: world.clone(),
            script: &["C1GP7"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-tremor-targets",
            options: world.clone(),
            script: &["C1IPVY"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-repel-undead-targets",
            options: world.clone(),
            script: &["C1ACX"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-charm-target",
            options: world.clone(),
            script: &["C1AEX7"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-polymorph-target",
            options: world.clone(),
            script: &["C1BRX7"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-clone-target",
            options: world.clone(),
            script: &["C1IQX7"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-conjure-animal",
            options: world.clone(),
            script: &["C1KX"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-swarm-summon",
            options: world.clone(),
            script: &["C1BIX"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-summon-daemon-ring",
            options: world.clone(),
            script: &["C1CKX6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
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
            "route smoke `{}` ended at turn {}; expected at least {}; message `{}`",
            case.name, state.turn, case.min_turn, state.message
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
        "endgame-missing-box-confirmation" | "endgame-missing-box-terminal-jitter" => {
            state.enter_endgame_from_game_dir(Some(game_dir))?;
        }
        "endgame-box-victory-confirmation" | "endgame-box-full-victory-cinematic" => {
            state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
            state.enter_endgame_from_game_dir(Some(game_dir))?;
        }
        "blackthorn-audience-correct" | "blackthorn-audience-wrong" => {
            state.begin_blackthorn_audience_capture(game_dir)?;
        }
        "blackthorn-rescue-refuge" => {
            state.apply_blackthorn_rescue_refuge(game_dir)?;
        }
        "castle-light-open-spell-route" => {
            state.player.x = 1;
            state.player.y = 1;
            state.player.facing = Direction::East;
            let target = state.player.y * TOWN_GRID_SIDE + state.player.x + 1;
            if let Some(cell) = state.grid.get_mut(target) {
                *cell = 0x97;
            }
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "combat-directed-sleep-cone" => {
            seed_directed_wind_combat_route(
                state,
                SLEEP_SPELL_INDEX,
                SLEEP_COST,
                2,
                Some(1),
                false,
            )?;
        }
        "combat-directed-poison-wind-cone" => {
            seed_directed_wind_combat_route(
                state,
                POISON_WIND_SPELL_INDEX,
                POISON_WIND_COST,
                3,
                Some(2),
                false,
            )?;
            state.prng_state = poison_wind_first_accept_seed();
        }
        "combat-directed-death-wind-cone" => {
            seed_directed_wind_combat_route(
                state,
                DEATH_WIND_SPELL_INDEX,
                DEATH_WIND_COST,
                2,
                Some(1),
                true,
            )?;
        }
        "combat-directed-flame-wind-cone" => {
            seed_directed_wind_combat_route(
                state,
                FLAME_WIND_SPELL_INDEX,
                FLAME_WIND_COST,
                1,
                None,
                true,
            )?;
        }
        "combat-field-fire-marker-placement"
        | "combat-field-poison-marker-placement"
        | "combat-field-sleep-marker-placement"
        | "combat-field-energy-marker-placement" => {
            let (spell_index, cost, _) = combat_field_route_spell(case_name);
            seed_combat_field_route(state, spell_index, cost)?;
        }
        "combat-magic-missile-target"
        | "combat-tremor-targets"
        | "combat-repel-undead-targets"
        | "combat-charm-target"
        | "combat-polymorph-target"
        | "combat-clone-target"
        | "combat-conjure-animal"
        | "combat-swarm-summon"
        | "combat-summon-daemon-ring" => {
            seed_combat_spell_route(state, combat_spell_route_code(case_name))?;
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
        "dungeon-heavy-door-variant-pass-through" => {
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
        "dungeon-field-cycle-spells" => {
            state.player.x = 1;
            state.player.y = 1;
            state.player.facing = Direction::East;
            let target = dungeon_cell_index(0, 2, 1);
            if let Some(cell) = state.grid.get_mut(target) {
                *cell = 0x00;
            }
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "dungeon-open-chest-spell" => {
            state.player.x = 1;
            state.player.y = 1;
            let current = dungeon_cell_index(0, state.player.x, state.player.y);
            if let Some(cell) = state.grid.get_mut(current) {
                *cell = 0x40;
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
            if let Some(member) = state.party.first_mut() {
                member.status = b'G';
                member.hp = 3;
                member.max_hp = member.max_hp.max(30);
            }
            state.active_shop = Some(ActiveShopSession::Healer(
                HealerShopState::Greeting,
                Healer::WoundsOfHonour,
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
        "shop-shipwright-quote-decline-route"
        | "shop-shipwright-island-frigate-buy"
        | "shop-shipwright-crows-nest-skiff-buy"
        | "shop-shipwright-oaken-oar-frigate-buy"
        | "shop-shipwright-rusty-bucket-skiff-buy" => {
            let shipwright = shipwright_route_shop(case_name);
            state.gold = 999;
            state.return_world = Some(WorldReturn {
                plane: WorldPlane::Britannia,
                x: 1,
                y: 2,
                transport: state.player.transport,
                timing_status: state.timing_status,
                sail_cadence: state.sail_cadence,
                sail_stall_pending: state.sail_stall_pending,
                grid: state.grid.clone(),
                active_objects: state.active_objects.clone(),
                pending_vehicle: None,
            });
            state.active_shop = Some(ActiveShopSession::ShipBroker(
                ShipBrokerState::for_shipwright(shipwright),
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

fn shipwright_route_shop(case_name: &str) -> Shipwright {
    match case_name {
        "shop-shipwright-crows-nest-skiff-buy" => Shipwright::TheCrowsNest,
        "shop-shipwright-oaken-oar-frigate-buy" => Shipwright::TheOakenOar,
        "shop-shipwright-rusty-bucket-skiff-buy" => Shipwright::TheRustyBucket,
        _ => Shipwright::IslandShipwrights,
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

fn poison_wind_first_accept_seed() -> u16 {
    for candidate in 0..=u16::MAX {
        let mut state = candidate;
        if u5_prng_range_u16(&mut state, 0, 19) & 1 == 0 {
            return candidate;
        }
    }
    unreachable!("PRNG range cycle must hit a Poison Wind acceptance roll")
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

fn route_combat_active_object(tile: u8, x: usize, y: usize, z: i8) -> ActiveObject {
    ActiveObject {
        type_byte: tile,
        tile,
        x,
        y,
        z,
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    }
}

fn seed_directed_wind_combat_route(
    state: &mut PlayState,
    spell_index: usize,
    cost: u8,
    party_count: usize,
    target_party_slot: Option<usize>,
    include_monster_target: bool,
) -> io::Result<()> {
    state.party = (0..party_count)
        .map(|slot| route_party_member(slot as u8, b'A', b'G', 12, 20))
        .collect();
    state.party_names = default_party_names(party_count);
    state.party_experience = default_party_experience(party_count);
    state.party_stay_counters = default_party_stay_counters(party_count);
    state.party_strengths = vec![30; party_count];
    state.party_intelligence = default_party_intelligence(party_count);
    state.party_equipment = default_party_equipment(party_count);
    if let Some(caster) = state.party.first_mut() {
        caster.mana = cost;
        caster.level = cost;
    }
    state.active_player = Some(0);
    state.spell_charges[spell_index] = 1;

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([12, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    if let Some(target_slot) = target_party_slot {
        actors[target_slot] = CombatActorDescriptor::from_row([
            12,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            target_slot as u8,
            0,
            6,
            5,
        ]);
    }

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    for slot in 0..party_count {
        let x = if Some(slot) == target_party_slot {
            6
        } else {
            5
        };
        active_objects[slot] = route_combat_active_object(0x4c, x, 5, 0);
    }

    if include_monster_target {
        let stats = combat_class_stats(COMBAT_CLASS_GIANT_RAT)
            .ok_or_else(|| io::Error::other("giant rat combat stats are unavailable"))?;
        let monster_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let monster_x = if target_party_slot.is_some() { 7 } else { 6 };
        actors[monster_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            monster_slot as u8,
            monster_x,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        active_objects[monster_slot] =
            summoned_active_object_record(COMBAT_CLASS_GIANT_RAT, monster_x as usize, 5, 0)
                .ok_or_else(|| io::Error::other("giant rat active object is unavailable"))?;

        let reserve_slot = monster_slot + 1;
        actors[reserve_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            reserve_slot as u8,
            3,
            5,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        active_objects[reserve_slot] =
            summoned_active_object_record(COMBAT_CLASS_GIANT_RAT, 3, 5, 0).ok_or_else(|| {
                io::Error::other("reserve giant rat active object is unavailable")
            })?;
    }

    state.enter_combat_frame(active_objects, actors)?;
    Ok(())
}

fn combat_field_route_spell(case_name: &str) -> (usize, u8, CombatArenaFieldKind) {
    match case_name {
        "combat-field-poison-marker-placement" => (
            POISON_FIELD_SPELL_INDEX,
            FIELD_SPELL_COST,
            CombatArenaFieldKind::Poison,
        ),
        "combat-field-sleep-marker-placement" => (
            SLEEP_FIELD_SPELL_INDEX,
            FIELD_SPELL_COST,
            CombatArenaFieldKind::Sleep,
        ),
        "combat-field-energy-marker-placement" => (
            ENERGY_FIELD_SPELL_INDEX,
            ENERGY_FIELD_COST,
            CombatArenaFieldKind::Energy,
        ),
        _ => (
            FIRE_FIELD_SPELL_INDEX,
            FIELD_SPELL_COST,
            CombatArenaFieldKind::Fire,
        ),
    }
}

fn seed_combat_field_route(state: &mut PlayState, spell_index: usize, cost: u8) -> io::Result<()> {
    state.party = vec![route_party_member(0, b'A', b'G', 20, 20)];
    state.party_names = default_party_names(1);
    state.party_experience = default_party_experience(1);
    state.party_stay_counters = default_party_stay_counters(1);
    state.party_strengths = vec![30];
    state.party_intelligence = default_party_intelligence(1);
    state.party_equipment = default_party_equipment(1);
    if let Some(caster) = state.party.first_mut() {
        caster.mana = cost;
        caster.level = cost;
    }
    state.active_player = Some(0);
    state.spell_charges[spell_index] = 1;

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    active_objects[0] = route_combat_active_object(0x4c, 5, 5, 0);
    state.enter_combat_frame(active_objects, actors)?;
    Ok(())
}

fn combat_spell_route_code(case_name: &str) -> &'static str {
    match case_name {
        "combat-tremor-targets" => "IPVY",
        "combat-repel-undead-targets" => "ACX",
        "combat-charm-target" => "AEX",
        "combat-polymorph-target" => "BRX",
        "combat-clone-target" => "IQX",
        "combat-conjure-animal" => "KX",
        "combat-swarm-summon" => "BIX",
        "combat-summon-daemon-ring" => "CKX",
        _ => "GP",
    }
}

fn seed_combat_spell_route(state: &mut PlayState, code: &str) -> io::Result<()> {
    let spell_index = spell_index_from_code(code)
        .ok_or_else(|| io::Error::other(format!("unknown combat spell code `{code}`")))?;
    let cost = spell_mp_cost(spell_index)
        .ok_or_else(|| io::Error::other(format!("unknown combat spell cost for `{code}`")))?;

    state.party = vec![route_party_member(0, b'A', b'G', 99, 99)];
    state.party_names = default_party_names(1);
    state.party_experience = default_party_experience(1);
    state.party_stay_counters = default_party_stay_counters(1);
    state.party_strengths = vec![30];
    state.party_intelligence = default_party_intelligence(1);
    state.party_equipment = default_party_equipment(1);
    if let Some(caster) = state.party.first_mut() {
        caster.mana = cost;
        caster.level = cost;
    }
    state.active_player = Some(0);
    state.spell_charges[spell_index] = 1;
    let mut combat_terrain = if matches!(code, "IQX" | "KX" | "BIX" | "CKX") {
        [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE]
    } else {
        [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE]
    };
    match code {
        "IQX" => {
            combat_terrain[1][8] = 0x04;
            combat_terrain[5][5] = 0x04;
            combat_terrain[5][6] = 0x04;
        }
        "KX" => {
            combat_terrain[0][7] = 0x04;
            combat_terrain[5][5] = 0x04;
        }
        "BIX" => {
            combat_terrain[5][5] = 0x04;
            combat_terrain[4][5] = 0x04;
            combat_terrain[4][6] = 0x04;
        }
        "CKX" => {
            combat_terrain[5][5] = 0x04;
            combat_terrain[4][6] = 0x04;
        }
        _ => {}
    }
    state.prng_state = match code {
        "GP" => first_nonzero_prng_roll_seed(15),
        "IPVY" => first_nonzero_prng_roll_seed(19),
        _ => 0,
    };

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([99, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    active_objects[0] = route_combat_active_object(0x4c, 5, 5, 0);

    match code {
        "ACX" => {
            seed_combat_route_monster(&mut actors, &mut active_objects, 23, 6, 4, 5)?;
            seed_combat_route_monster(&mut actors, &mut active_objects, 33, 7, 5, 4)?;
            seed_combat_route_monster(&mut actors, &mut active_objects, 32, 8, 6, 5)?;
        }
        "KX" | "BIX" | "CKX" => {}
        _ => {
            let class = if matches!(code, "BRX" | "IPVY") {
                39
            } else {
                COMBAT_CLASS_GIANT_RAT
            };
            seed_combat_route_monster(&mut actors, &mut active_objects, class, 6, 6, 5)?;
        }
    }

    state.enter_combat_frame_with_terrain(active_objects, actors, combat_terrain)?;
    Ok(())
}

fn seed_combat_route_monster(
    actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    active_objects: &mut [ActiveObject],
    class: u8,
    slot: usize,
    x: u8,
    y: u8,
) -> io::Result<()> {
    let stats = combat_class_stats(class).ok_or_else(|| {
        io::Error::other(format!("combat stats for class {class} are unavailable"))
    })?;
    let active_object_slot = slot as u8;
    actors[slot] = CombatActorDescriptor::for_monster_placement(
        stats,
        active_object_slot,
        x,
        y,
        COMBAT_ACTOR_FLAG_SELECTABLE_80,
        0,
    );
    active_objects[slot] = summoned_active_object_record(class, usize::from(x), usize::from(y), 0)
        .ok_or_else(|| {
            io::Error::other(format!(
                "active-object record for combat class {class} is unavailable"
            ))
        })?;
    Ok(())
}

fn first_nonzero_prng_roll_seed(max: u16) -> u16 {
    for candidate in 0..=u16::MAX {
        let mut state = candidate;
        if u5_prng_range_u16(&mut state, 0, max) > 0 {
            return candidate;
        }
    }
    0
}

fn validate_combat_spell_route_state(state: &PlayState, case_name: &str) -> io::Result<()> {
    let code = combat_spell_route_code(case_name);
    let spell_index = spell_index_from_code(code)
        .ok_or_else(|| io::Error::other(format!("unknown combat spell code `{code}`")))?;
    if !state.combat_active
        || state.spell_charges[spell_index] != 0
        || state.party.first().is_none_or(|member| member.mana != 0)
        || state.turn < 1
    {
        return Err(io::Error::other(format!(
            "route smoke `{case_name}` did not spend combat spell resources"
        )));
    }

    match case_name {
        "combat-magic-missile-target" => {
            if !state.message.starts_with("Magic Missile!") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the targeted Magic Missile spell; message `{}`",
                    state.message
                )));
            }
        }
        "combat-tremor-targets" => {
            if !state.message.starts_with("Tremor!")
                || state
                    .party
                    .first()
                    .is_none_or(|member| member.hp >= member.max_hp)
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].hp_or_wound
                    >= combat_class_stats(39)
                        .map(|stats| stats.max_hp)
                        .unwrap_or(u8::MAX)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply Tremor to party and monster slots"
                )));
            }
        }
        "combat-repel-undead-targets" => {
            if !state
                .message
                .starts_with("Repel Undead! 2 undead repelled.")
                || !state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_marked_dead()
                || !state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_marked_dead()
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2].is_marked_dead()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not repel only undead combat actors"
                )));
            }
        }
        "combat-charm-target" => {
            if !state.message.starts_with("Charm!") {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not charm the targeted combat actor"
                )));
            }
        }
        "combat-polymorph-target" => {
            if !state.message.starts_with("Polymorph!")
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].owner_target_class
                    != COMBAT_CLASS_GIANT_RAT
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not polymorph the hostile target"
                )));
            }
        }
        "combat-clone-target" => {
            if !state.message.starts_with("Clone!")
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_empty()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not place a cloned combat actor; message `{}`, slot 7 {:?}",
                    state.message,
                    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1]
                )));
            }
        }
        "combat-conjure-animal" | "combat-swarm-summon" => {
            if !state.message.starts_with("Success!")
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_empty()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not place summoned combat actors"
                )));
            }
        }
        "combat-summon-daemon-ring" => {
            if !state.message.starts_with("Summon Daemon!")
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_empty()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not place the daemon around the target cell"
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_route_smoke_case_state(state: &PlayState, case_name: &str) -> io::Result<()> {
    match case_name {
        "endgame-missing-box-confirmation" | "endgame-missing-box-terminal-jitter" => {
            let outcome = state.endgame.as_ref().and_then(|endgame| endgame.outcome);
            if outcome != Some(EndgameOutcome::MissingBoxOrRefused)
                || state
                    .endgame
                    .as_ref()
                    .is_none_or(|endgame| !endgame.is_terminal())
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not remain in the missing-box endgame tableau"
                )));
            }
        }
        "endgame-box-victory-confirmation" => {
            let Some(endgame) = state.endgame.as_ref() else {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not enter the victory endgame"
                )));
            };
            if endgame.outcome != Some(EndgameOutcome::Victory)
                || endgame.cinematic_is_finished()
                || endgame.certificate.is_none()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not enter the active victory cinematic"
                )));
            }
        }
        "endgame-box-full-victory-cinematic" => {
            let Some(endgame) = state.endgame.as_ref() else {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not enter the victory endgame"
                )));
            };
            let party_slots_cleared = state
                .active_objects
                .iter()
                .take(state.party.len().min(6))
                .all(|object| object.is_empty());
            let cinematic_slots_cleared = state
                .active_objects
                .get(6)
                .is_none_or(|object| object.is_empty())
                && state
                    .active_objects
                    .get(31)
                    .is_none_or(|object| object.is_empty());
            if endgame.outcome != Some(EndgameOutcome::Victory)
                || !endgame.cinematic_is_finished()
                || endgame.certificate.is_none()
                || !party_slots_cleared
                || !cinematic_slots_cleared
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not finish the victory cinematic and clear tableau actors (step={:?}, finished={}, certificate={}, party_slots_cleared={}, cinematic_slots_cleared={})",
                    endgame.cinematic.step,
                    endgame.cinematic_is_finished(),
                    endgame.certificate.is_some(),
                    party_slots_cleared,
                    cinematic_slots_cleared
                )));
            }
        }
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
        "britannia-locate-cast" => {
            if state.player.x != 62
                || state.player.y != 124
                || state.spell_charges[IN_WIS_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state.message != "Locate: H'M,D'O\""
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the public Locate sextant output"
                )));
            }
        }
        "castle-light-open-spell-route" => {
            let target = state.player.y * TOWN_GRID_SIDE + state.player.x + 1;
            if state.spell_charges[VAS_LOR_SPELL_INDEX] != 0
                || state.spell_charges[OPEN_SPELL_INDEX] != 0
                || state.light_spell_counter == 0
                || state.grid.get(target).copied() != Some(0xb8)
                || state.message != "Opened!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not light the scene and open the stamped door"
                )));
            }
        }
        "castle-restore-spell-suite" => {
            if state.spell_charges[AWAKEN_SPELL_INDEX] != 0
                || state.spell_charges[CURE_SPELL_INDEX] != 0
                || state.spell_charges[HEAL_SPELL_INDEX] != 0
                || state.spell_charges[GREAT_HEAL_SPELL_INDEX] != 0
                || state.spell_charges[RESURRECT_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state
                    .party
                    .get(1)
                    .is_none_or(|member| member.status != b'G' || member.hp != 8)
                || state
                    .party
                    .get(2)
                    .is_none_or(|member| member.status != b'G' || member.hp != member.max_hp)
                || state.party.get(3).is_none_or(|member| {
                    member.status != b'G' || member.hp != 1 || member.max_hp == 0
                })
                || !state.message.starts_with("Resurrected party member 4")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the restore spell suite"
                )));
            }
        }
        "castle-active-effect-spell-suite" => {
            if state.spell_charges[PROTECTION_SPELL_INDEX] != 0
                || state.spell_charges[QUICKNESS_SPELL_INDEX] != 0
                || state.spell_charges[NEGATE_MAGIC_SPELL_INDEX] != 0
                || state.spell_charges[TIME_STOP_SPELL_INDEX] != 0
                || state.active_effect_tag != Some(NEGATE_TIME_ACTIVE_EFFECT_TAG)
                || state.active_effect_counter != TIME_STOP_DURATION
                || state.message != "Negate time!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the active-effect spell sequence"
                )));
            }
        }
        "combat-directed-sleep-cone" => {
            if !state.combat_active
                || state.spell_charges[SLEEP_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state
                    .party
                    .get(1)
                    .is_none_or(|member| member.status != b'S')
                || state.message != "Sleep!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the directed Sleep cone"
                )));
            }
        }
        "combat-directed-poison-wind-cone" => {
            if !state.combat_active
                || state.spell_charges[POISON_WIND_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state
                    .party
                    .get(2)
                    .is_none_or(|member| member.status != b'P')
                || state.message != "Poison wind!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the directed Poison Wind cone"
                )));
            }
        }
        "combat-directed-death-wind-cone" => {
            let stats = combat_class_stats(COMBAT_CLASS_GIANT_RAT)
                .ok_or_else(|| io::Error::other("giant rat combat stats are unavailable"))?;
            if !state.combat_active
                || state.spell_charges[DEATH_WIND_SPELL_INDEX] != 0
                || state
                    .party
                    .first()
                    .is_none_or(|member| member.status != b'G')
                || state
                    .party
                    .get(1)
                    .is_none_or(|member| member.status != b'D')
                || !state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_marked_dead()
                || state
                    .party_experience
                    .first()
                    .is_none_or(|xp| *xp != u16::from(stats.reward_unit()))
                || !state.message.starts_with("Death wind!")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the directed Death Wind cone"
                )));
            }
        }
        "combat-directed-flame-wind-cone" => {
            if !state.combat_active
                || state.spell_charges[FLAME_WIND_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || !state.message.starts_with("Flame wind!")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the directed Flame Wind cone"
                )));
            }
        }
        "combat-field-fire-marker-placement"
        | "combat-field-poison-marker-placement"
        | "combat-field-sleep-marker-placement"
        | "combat-field-energy-marker-placement" => {
            let (spell_index, _, field) = combat_field_route_spell(case_name);
            let marker = state.find_combat_arena_field_marker(6, 5);
            if !state.combat_active
                || state.spell_charges[spell_index] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || marker.is_none_or(|(_, placed)| placed != field)
                || !state
                    .message
                    .contains(&format!("{} field placed.", field.label()))
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not materialize the public combat field marker"
                )));
            }
        }
        "combat-magic-missile-target"
        | "combat-tremor-targets"
        | "combat-repel-undead-targets"
        | "combat-charm-target"
        | "combat-polymorph-target"
        | "combat-clone-target"
        | "combat-conjure-animal"
        | "combat-swarm-summon"
        | "combat-summon-daemon-ring" => {
            validate_combat_spell_route_state(state, case_name)?;
        }
        "dungeon-level-up-down-spells" => {
            if !matches!(state.area, Area::Dungeon { level: 1, .. })
                || state
                    .active_objects
                    .first()
                    .is_none_or(|object| object.z != 1)
                || state.spell_charges[UUS_POR_SPELL_INDEX] != 0
                || state.spell_charges[DES_POR_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || !state.message.contains("Down!")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not move dungeon levels up then down"
                )));
            }
        }
        "dungeon-field-cycle-spells" => {
            let target = dungeon_cell_index(0, 2, 1);
            if state.grid.get(target).copied() != Some(0x00)
                || state.spell_charges[FIRE_FIELD_SPELL_INDEX] != 0
                || state.spell_charges[POISON_FIELD_SPELL_INDEX] != 0
                || state.spell_charges[SLEEP_FIELD_SPELL_INDEX] != 0
                || state.spell_charges[ENERGY_FIELD_SPELL_INDEX] != 0
                || state.spell_charges[DISPEL_FIELD_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || !state.message.contains("Dispelled electric field")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not cycle dungeon field placement and dispel"
                )));
            }
        }
        "dungeon-open-chest-spell" => {
            let current = dungeon_cell_index(0, state.player.x, state.player.y);
            if state.grid.get(current).copied() != Some(0x70)
                || state.spell_charges[OPEN_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || !state.message.contains("Safely opened dungeon chest")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not open the dungeon chest by spell"
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
                || !state.message.contains("is poisoned!")
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
            if !state.message.contains("Wanted:") || !state.message.contains("Dead or Alive") {
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
        "shop-arms-local-buy-sell-route" => {
            if state.gold != 999
                || state.active_shop.is_some()
                || state.equipment_stock[EQUIPMENT_ID_BOW] != 0
                || !state.message.contains("Farewell")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not exercise arms buy/sell decline without mutation"
                )));
            }
        }
        "shop-healer-heal-decline-route" => {
            if state.gold != 999
                || state.active_shop.is_none()
                || state
                    .party
                    .first()
                    .is_none_or(|member| member.hp == member.max_hp)
                || !state.message.contains("Declined Heal")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not quote and decline healer treatment"
                )));
            }
        }
        "shop-inn-rest-decline-route" => {
            if state.gold != 999
                || state.active_shop.is_none()
                || !state.message.contains("No one here is from thy party")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not decline inn rest and stay in the inn menu"
                )));
            }
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
        "shop-reagent-buy-route" => {
            if state.gold >= 999
                || state.active_shop.is_some()
                || state.reagents.iter().all(|count| *count == 0)
                || !state.message.contains("Farewell")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not buy reagent stock and exit"
                )));
            }
        }
        "shop-tavern-drink-and-food-route" => {
            if state.gold >= 999
                || state.food == 0
                || state.active_shop.is_some()
                || !state.message.contains("Farewell")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not serve a drink round and provisions"
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
                || state.active_shop.is_some()
                || state.message != TAVERN_AFFORDABILITY_REFUSAL_BARK
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve gold and exit on sage short funds"
                )));
            }
        }
        "shop-horse-trader-decline-route" => {
            if state.gold != 999
                || state.active_shop.is_none()
                || state
                    .active_objects
                    .iter()
                    .any(|object| object.type_byte == HORSE_PARKED_FIRST)
                || !state.message.contains("As you wish")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not quote and decline the horse trader"
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
        "shop-shipwright-quote-decline-route" => {
            if state.gold != 999
                || state
                    .return_world
                    .as_ref()
                    .is_none_or(|world| world.pending_vehicle.is_some())
                || state.active_shop.is_none()
                || !state.message.contains("As you wish")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not quote and decline shipwright purchase"
                )));
            }
        }
        "shop-shipwright-island-frigate-buy"
        | "shop-shipwright-crows-nest-skiff-buy"
        | "shop-shipwright-oaken-oar-frigate-buy"
        | "shop-shipwright-rusty-bucket-skiff-buy" => {
            let shipwright = shipwright_route_shop(case_name);
            let (x, y) = shipwright_delivery_coordinate(shipwright);
            let expected_pending = if case_name.contains("skiff") {
                PendingVehicleAcquisition::Skiff { x, y }
            } else {
                PendingVehicleAcquisition::Frigate { x, y, skiffs: 2 }
            };
            let expected_gold = 999
                - match case_name.contains("skiff") {
                    true => shipwright_price(shipwright, ShipwrightPurchaseKind::Skiff),
                    false => shipwright_price(shipwright, ShipwrightPurchaseKind::Frigate),
                };
            if state.gold != expected_gold
                || state
                    .return_world
                    .as_ref()
                    .is_none_or(|world| world.pending_vehicle != Some(expected_pending))
                || !state.message.contains("Delivery is queued")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not queue delivery at the published shipwright coordinate"
                )));
            }
        }
        "shop-guild-buy-route" => {
            if state.gold >= 999
                || state.keys == 0
                || state.active_shop.is_some()
                || !state.message.contains("Farewell")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not buy guild stock and exit"
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
        "dungeon-heavy-door-variant-pass-through" => {
            if state.player.x != 2
                || state.player.y != 1
                || state.turn != 1
                || !state.message.contains("underfoot heavy-door variant")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not pass through the public 0xE? heavy-door variant"
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

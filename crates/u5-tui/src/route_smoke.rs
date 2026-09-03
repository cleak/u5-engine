//! Route-level smoke suite for local clean assets.
//!
//! These cases intentionally exercise public harness routes and sidecar-backed
//! transitions without asserting copyrighted text content.
//!
//! # Case names are matched in more than one place
//!
//! A case name is a bare string, and it is keyed on by several `match
//! case_name` blocks in this file - notably the *setup* block
//! ([`apply_route_smoke_case_setup`]) and the *validation* block
//! ([`validate_route_smoke_case_state`]). The same name legitimately
//! appears in both, so searching for `"some-case" =>` finds more than one
//! anchor and it is easy to add an arm to the wrong block. An arm added to
//! the block that already has one for that name is dead code: the case
//! then "passes" while doing nothing, which is the worst possible
//! outcome for a smoke harness.
//!
//! Two guards make that loud instead of silent:
//!
//! * `deny(unreachable_patterns)` below turns a duplicate arm into a build
//!   error rather than a warning nobody reads.
//! * [`tests::every_case_name_arm_names_a_real_route_smoke_case`] rejects
//!   an arm keyed on a name no case carries, which is the other way an arm
//!   can never fire.

// A second arm for a case name that a `match case_name` block already
// handles can never run. That is exactly how a validation arm added to the
// setup block disappears, so it must not compile.
#![deny(unreachable_patterns)]

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use u5_runtime::{
    AWAKEN_COST, AWAKEN_SPELL_INDEX, ActiveObject, Area, ArmsShop, BLACKTHORN_CAPTIVE_CELL_SCENE,
    BLACKTHORN_RESCUE_HANDOFF_SCENE, BLINK_COST, BLINK_SPELL_INDEX, BRIT_DAT_FILENAME,
    BRIT_OOL_FILENAME, CODEX_URN_TABLE_FILE, COMBAT_ACTOR_FLAG_FLEEING,
    COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED, COMBAT_ACTOR_FLAG_SELECTABLE_80, COMBAT_ACTOR_SLOTS,
    COMBAT_ARENA_SIDE, COMBAT_CLASS_GIANT_RAT, COMBAT_CLASS_SHADOW_LORD,
    COMBAT_DEFAULT_DEATH_DROP_TILE, COMBAT_GARGOYLE_DEATH_TERRAIN_TILE,
    COMBAT_GAZER_DEATH_MARKER_TILE, COMBAT_PARTY_ACTOR_SLOTS, COMBAT_PLACEMENT_PHASE_BASE,
    COMPLETED_LONG_CAMP_COOLDOWN_HOURS, CREATE_FOOD_COST, CREATE_FOOD_MAX_GRANT,
    CREATE_FOOD_SPELL_INDEX, CURE_COST, CURE_SPELL_INDEX, CombatActorDescriptor,
    CombatArenaFieldKind, DEATH_VISION_LOOK_TILE, DEATH_WIND_COST, DEATH_WIND_SPELL_INDEX,
    DEFAULT_FOOD_STOCK, DEFAULT_KEY_STOCK, DES_POR_SPELL_INDEX, DISPEL_FIELD_COST,
    DISPEL_FIELD_SPELL_INDEX, DUNGEON_AMBUSH_ARENA_FLOOR_TILE, DUNGEON_AMBUSH_PARTY_ENTRY_X,
    DUNGEON_AMBUSH_PARTY_ENTRY_Y, DUNGEON_LEVEL_SPELL_COST, DUNGEON_MONSTER_COMBAT_CLASSES,
    DUNGEON_ROOM_SOURCE_COUNT, Direction, DungeonScene, ENERGY_FIELD_COST,
    ENERGY_FIELD_SPELL_INDEX, EQUIP_SLOT_RING, EQUIP_SLOT_WEAPON, EQUIPMENT_EMPTY,
    EQUIPMENT_ID_ARROWS, EQUIPMENT_ID_BOW, EQUIPMENT_ID_RING_REGENERATION, EndgameOutcome,
    FIELD_SPELL_COST, FIRE_FIELD_SPELL_INDEX, FIRST_PLAYABLE_FRIGATE_TILE,
    FIRST_PLAYABLE_FULL_SHIP_HULL, FIRST_PLAYABLE_HOURLY_POISON_DAMAGE, FLAME_WIND_COST,
    FLAME_WIND_SPELL_INDEX, GATE_TRAVEL_COST, GATE_TRAVEL_SPELL_INDEX, GREAT_HEAL_COST,
    GREAT_HEAL_SPELL_INDEX, GameClock, GuildShop, HARPSICHORD_FLOOR,
    HARPSICHORD_PASSAGE_CELLS_NORTH, HARPSICHORD_PASSAGE_CLEARED_TILE, HARPSICHORD_TILE, HEAL_COST,
    HEAL_SPELL_INDEX, HORSE_PARKED_FIRST, HOURLY_STARVATION_DAMAGE_MAX,
    HOURLY_STARVATION_DAMAGE_MIN, Healer, Herbalist, IN_LOR_COST, IN_LOR_SPELL_INDEX, IN_WIS_COST,
    IN_WIS_SPELL_INDEX, INN_REST_WAKE_HOUR, Inn, JIMMY_MANACLES_TILE, JIMMY_RELEASE_AI_MODE,
    JIMMY_STOCKS_TILE, MAGIC_LOCK_COST, MAGIC_LOCK_SPELL_INDEX, MASS_CHARM_ACTIVE_EFFECT_DURATION,
    MASS_CHARM_ACTIVE_EFFECT_TAG, MORAL_STANDING_MAX, MoonstoneGateSlot, NARRATIVE_GATE_X,
    NARRATIVE_GATE_Y, NATURAL_MOONGATE_RESTORED_TERRAIN_TILE, NATURAL_MOONGATE_TERRAIN_TILE,
    NEGATE_MAGIC_COST, NEGATE_MAGIC_SPELL_INDEX, NEGATE_TIME_ACTIVE_EFFECT_TAG, NPC_DIALOG_ID_NONE,
    NPC_SCHEDULE_AI_OFFSET, NPC_SCHEDULE_WAYPOINT_COUNT, NPC_SCHEDULE_X_OFFSET,
    NPC_SCHEDULE_Y_OFFSET, NpcSlot, OOL_RECORD_LEN, OOL_SLOTS, OPEN_SPELL_COST, OPEN_SPELL_INDEX,
    OUTDOOR_BROADSIDE_BOOM_MESSAGE, OUTDOOR_IMPACT_HULL_ROLL_HIGH, PEER_COST, PEER_SPELL_INDEX,
    POISON_FIELD_SPELL_INDEX, POISON_WIND_COST, POISON_WIND_SPELL_INDEX, PROTECTION_COST,
    PROTECTION_SPELL_INDEX, PartyMember, PendingVehicleAcquisition, PlayOptions, PlayState,
    PlayTarget, QUICKNESS_COST, QUICKNESS_SPELL_INDEX, REAGENT_SULFUR_ASH, REL_HUR_COST,
    REL_HUR_SPELL_INDEX, RESURRECT_COST, RESURRECT_SPELL_INDEX, SAVE_QUEST_TILE_FLAG_HIGH_BIT,
    SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, SAVED_OOL_LEN, SCENE_EMPATH_ABBEY, SCENE_JHELOM,
    SCENE_MOONGLOW, SCENE_SERPENTS_HOLD, SCENE_STONEGATE, SCENE_THE_LYCAEUM,
    SHADOWLORD_COWARDICE_INDEX, SHADOWLORD_FALSEHOOD_INDEX, SHADOWLORD_HATRED_INDEX,
    SHADOWLORD_HIDEOUT_VANQUISHED, SHADOWLORD_VANQUISHED, SHIP_NO_SKIFFS_WARNING,
    SHRINE_ALTAR_TILE_FIRST, SHRINE_RESTORATION_SUCCESS_BANNER, SLEEP_COST,
    SLEEP_FIELD_SPELL_INDEX, SLEEP_SPELL_INDEX, SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX,
    SPECIAL_ITEM_MAGIC_CARPET_INDEX, SPECIAL_ITEM_OWNED_VALUE, SPECIAL_ITEM_POCKET_WATCH_INDEX,
    SPECIAL_ITEM_SCEPTRE_LB_INDEX, SPECIAL_ITEM_SEXTANT_INDEX, SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
    SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX, SPECIAL_ITEM_SHARD_HATRED_INDEX,
    SPECIAL_ITEM_SPYGLASS_INDEX, SPECIAL_ITEM_WOODEN_BOX_INDEX, STEADY_PHASE, SURFACE_CHASM_X,
    SURFACE_CHASM_Y, Scene, Shipwright, ShipwrightPurchaseKind, ShrineVirtue, Stable,
    TALK_NO_RESPONSE_MESSAGE, TALK_SLEEPING_MESSAGE, TALK_STATUS_TILE_PRAYING,
    TALK_STATUS_TILE_SLEEPING, TAVERN_AFFORDABILITY_REFUSAL_BARK, TIME_STOP_COST,
    TIME_STOP_DURATION, TIME_STOP_SPELL_INDEX, TOWN_ARREST_JAIL_FLOOR, TOWN_ARREST_JAIL_SCENE,
    TOWN_ARREST_JAIL_X, TOWN_ARREST_JAIL_Y, TOWN_DOOR_MAGIC_PLAIN_TILE, TOWN_GAS_DOORWAY_RANGE_MAX,
    TOWN_GRID_SIDE, TOWN_POISON_GAS_LIVE_TILE, TOWN_TRAPDOOR_LIVE_TILE,
    TRANSPORT_MARKER_SHIP_FURLED_FIRST, Tavern, TileGraphicsDepth, TransportState,
    UNDER_OOL_FILENAME, UNLOCK_MAGIC_COST, UNLOCK_MAGIC_SPELL_INDEX, UUS_POR_SPELL_INDEX,
    VANISH_CLEARED_TILE, VANISH_COST, VANISH_SPELL_INDEX, VAS_LOR_COST, VAS_LOR_SPELL_INDEX,
    WHIRLPOOL_EMERGENCE_X, WHIRLPOOL_EMERGENCE_Y, WORD_OF_POWER_SEALED_TILE, WORD_OF_POWER_SEALS,
    WORLD_RUINED_SHRINE_TILE, WORLD_SHRINE_COORDINATES, WORLD_SHRINE_TILE, WORLD_SIDE, WindState,
    WordOfPowerSeal, WorldPlane, WorldReturn, X_RAY_COST, X_RAY_SPELL_INDEX,
    YELL_NOTHING_SAID_MESSAGE, YELL_SAILS_HOISTED_MESSAGE, combat_class_sprite_byte,
    combat_class_stats, combat_monster_placement_flags, default_party_equipment,
    default_party_experience, default_party_intelligence, default_party_names,
    default_party_roster, default_party_stay_counters, default_party_strengths,
    dungeon_ambush_source_rows, dungeon_cell_index, dungeon_room_entry_seed_for_direction,
    hash_palette_indices, inn_base_room_rate, load_camp_result_messages,
    load_play_options_from_save, load_tile_atlas, published_world_location_entries,
    shipwright_delivery_coordinate, shipwright_price, shop_intelligence_adjusted_price,
    shop_runtime::{
        ArmsShopState, GuildShopState, HealerShopState, HorseTraderState, InnkeeperState,
        ReagentShopState, SageState, ShipBrokerState, TavernState,
    },
    shop_session::ActiveShopSession,
    spell_index_from_code, spell_mp_cost, stable_horse_price, summoned_active_object_record,
    u5_prng_advance_state, u5_prng_range_u16, waypoint_for_hour, word_of_power_seal_for_word,
    world_cell_index,
};

use crate::{
    complete_headless_blocking_presentations, play_script_command_label, play_script_state_line,
    raster_diagnostic_line, raster_frame_kind, replay_play_script_commands,
};

const VIEWPORT_RADIUS: usize = 5;

/// `town-mode.md §13` harpsichord placement in Lord British's Castle, read
/// off the shipped `CASTLE.DAT` floor `+2` grid rather than assumed: the
/// instrument tile `0x8D` sits at (17, 18), the chair the party plays from is
/// the cell immediately north of it, and the cell five squares north of the
/// instrument in the same column is a `catalogs/tile-catalog.md` wall variant
/// with ordinary cobble already behind it. These routes exist to catch the
/// shipped map drifting away from that description.
const HARPSICHORD_ROUTE_X: usize = 17;
const HARPSICHORD_ROUTE_Y: usize = 18;
/// The chair immediately north of the instrument: the only cell the position-
/// only arming test accepts.
const HARPSICHORD_ROUTE_CHAIR_Y: usize = HARPSICHORD_ROUTE_Y - 1;
/// The wall cell a finished tune rewrites to cobble.
const HARPSICHORD_ROUTE_PASSAGE_Y: usize = HARPSICHORD_ROUTE_Y - HARPSICHORD_PASSAGE_CELLS_NORTH;
/// `catalogs/tile-catalog.md`: wall variants are `0x4D..0x4F`, and the shipped
/// passage cell is `0x4F`. Asserting the *starting* byte is what makes the
/// "opened" assertion meaningful.
const HARPSICHORD_ROUTE_WALL_TILE: u8 = 0x4F;
/// Cobble two cells north of the chair, used by the away-from-the-chair route.
const HARPSICHORD_ROUTE_OFF_CHAIR_Y: usize = 15;
/// The floor `+2` ascend link, four cardinal steps from the chair. Klimbing up
/// and straight back down is the round trip that proves the passage rewrite
/// never reached the on-disk floor.
const HARPSICHORD_ROUTE_KLIMB_X: usize = 15;
const HARPSICHORD_ROUTE_KLIMB_Y: usize = 15;

/// The reload checkpoint sits one command past the whole tune.
const HARPSICHORD_RELOAD_CHECKPOINTS: &[usize] = &[HARPSICHORD_TUNE_SCRIPT.len()];

/// `town-mode.md §13` the thirteen-note tune, as script keystrokes.
const HARPSICHORD_TUNE_SCRIPT: [&str; 13] = [
    "6", "7", "8", "9", "8", "7", "8", "7", "6", "7", "6", "5", "3",
];

/// Full victory route: the 40-tick rite lead-in, six rite acknowledgements,
/// one Orb presentation, one Orb acknowledgement, 40 automatic tableau
/// frames, and eight ending acknowledgements through the terminal hold.
const ENDGAME_FULL_VICTORY_CINEMATIC_SCRIPT: [&str; 98] = {
    let mut script = ["empty"; 98];
    script[0] = "Y";
    script[1] = "Y";
    let mut index = 2;
    while index < 42 {
        script[index] = "endgame:frame";
        index += 1;
    }
    script[48] = "endgame:frame";
    index = 50;
    while index < 90 {
        script[index] = "endgame:frame";
        index += 1;
    }
    script
};

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
    pub final_frame_kind: String,
    pub final_width: usize,
    pub final_height: usize,
    pub final_hash: u64,
    pub final_nonblack_pixels: usize,
    pub frames: Vec<RouteSmokeFrameReport>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSmokeFrameReport {
    pub label: String,
    pub frame_kind: String,
    pub width: usize,
    pub height: usize,
    pub hash: u64,
    pub nonblack_pixels: usize,
    pub metadata: Vec<String>,
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
    let ship_xit_no_skiffs = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: TransportState::Ship {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
            skiffs: 0,
        },
        ..PlayOptions::default()
    };
    let ship_sail = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        wind: WindState::East,
        wind_save_byte: WindState::East.save_byte(),
        ..PlayOptions::default()
    };
    let ship_yell_town = PlayOptions {
        target: PlayTarget::Town(castle),
        ..PlayOptions::default()
    };
    let ship_yell_dungeon = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        torch_counter: 9,
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

    // `catalogs/item-list.md` Spyglass row: the scene gate admits a
    // town-class scene, and the night window is the Sextant's
    // `19..=23` / `0..=5`. Hour 19 in a town is the case the previous
    // implementation refused twice over — once on its overworld-only
    // scene test and once on the town-*lighting* window, which starts an
    // hour later. The overworld case above runs at hour 20, inside both
    // windows, so it never covered either bug.
    let mut town_spyglass_night_edge = PlayOptions {
        target: PlayTarget::Town(minoc),
        clock: GameClock::new(19, 0).expect("19:00 is a valid game-clock time"),
        ..PlayOptions::default()
    };
    town_spyglass_night_edge.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;

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

    let mut rel_hur = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        wind: WindState::Calm,
        wind_save_byte: WindState::Calm.save_byte(),
        ..PlayOptions::default()
    };
    rel_hur.spell_charges[REL_HUR_SPELL_INDEX] = 1;
    rel_hur.party[0].mana = REL_HUR_COST;
    rel_hur.party[0].level = REL_HUR_COST;

    let mut light_open = PlayOptions::default();
    light_open.spell_charges[IN_LOR_SPELL_INDEX] = 1;
    light_open.spell_charges[VAS_LOR_SPELL_INDEX] = 1;
    light_open.spell_charges[OPEN_SPELL_INDEX] = 1;
    light_open.party[0].mana = IN_LOR_COST + VAS_LOR_COST + OPEN_SPELL_COST;
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

    let town_jimmy_no_roll = PlayOptions {
        target: PlayTarget::Town(castle),
        start: Some((1, 1)),
        facing: Some(Direction::East),
        keys: DEFAULT_KEY_STOCK,
        party: vec![route_party_member(0, b'A', b'G', 20, 20)],
        ..PlayOptions::default()
    };
    let town_jimmy_release = PlayOptions {
        target: PlayTarget::Town(castle),
        start: Some((1, 1)),
        facing: Some(Direction::East),
        keys: DEFAULT_KEY_STOCK,
        moral_standing: 98,
        party: vec![route_party_member(0, b'A', b'G', 20, 20)],
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
        // `RETRACTIONS.md` R320: `(54, 138)` is the landing cell, not a
        // brink. The party starts on Britannia's one gate-reaching brink,
        // `(54, 136)`, with the waterfall family in the cell south of it; the
        // handler then force-steps two cells south onto the gate.
        start: Some((SURFACE_CHASM_X as usize, SURFACE_CHASM_Y as usize - 2)),
        facing: Some(Direction::South),
        // Britannia's one gate-reaching brink is river tile `0x60`, which no
        // party reaches on foot, so this route arrives by skiff.
        transport: TransportState::Skiff {
            type_byte: u5_runtime::SKIFF_PARKED_FIRST,
            tile: u5_runtime::SKIFF_PARKED_FIRST,
        },
        ..PlayOptions::default()
    };

    let mut whirlpool_forced_underworld = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((0, 0)),
        transport: ship_transport,
        ..PlayOptions::default()
    };
    whirlpool_forced_underworld.saved_active_objects = Some(vec![ActiveObject {
        type_byte: 0xEC,
        tile: 0xEC,
        x: 1,
        y: 0,
        z: WorldPlane::Britannia.save_floor(),
        phase: 0x80,
        aux1: 0,
        aux3: 0,
    }]);

    let narrative_gate_unordained_refusal = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((NARRATIVE_GATE_X as usize, NARRATIVE_GATE_Y as usize)),
        ..PlayOptions::default()
    };

    let mut narrative_gate_ordained_passage = narrative_gate_unordained_refusal.clone();
    narrative_gate_ordained_passage.shrine_ordained_mask = 0b0000_0001;

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
    let dungeon_long_camp_recovery = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        clock: GameClock::new(8, 0).expect("08:00 is a valid game-clock time"),
        food: 99,
        ..PlayOptions::default()
    };

    // `rest-and-camp.md §5`: a second camp begun inside fourteen game
    // hours of the previous one recovers nothing. Two six-hour camps
    // back to back leave the cooldown at eight when the second walk is
    // reached, so this route drives the gate the engine had no field for.
    // `rest-and-camp.md §5`: a camp begun inside fourteen game hours of
    // the previous one recovers nothing. The window cannot be reached by
    // camping twice in a route — the wilderness rest loop's sleep-ambush
    // roll interrupts long before a second six-hour camp completes — so
    // the counter is seeded directly. The options are otherwise identical
    // to `dungeon-long-camp-recovery`, and the gate draws no randomness,
    // so the two routes take the same path up to the recovery walk and
    // diverge only in whether it runs.
    let mut dungeon_camp_inside_cooldown = dungeon_long_camp_recovery.clone();
    dungeon_camp_inside_cooldown.camp_cooldown = COMPLETED_LONG_CAMP_COOLDOWN_HOURS;

    let doom_options = PlayOptions {
        target: PlayTarget::Dungeon(doom),
        floor: 0,
        ..PlayOptions::default()
    };

    let mut cases = vec![
        RouteSmokeCase {
            name: "castle-pass-and-idle",
            options: PlayOptions::default(),
            script: &["empty", "idle:2"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-canonical-ool-exit",
            options: PlayOptions::default(),
            script: &["s", "s", "Y"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
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
            name: "britannia-spyglass-night-sky",
            options: britannia_spyglass,
            script: &["USP"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "view overlay",
        },
        RouteSmokeCase {
            name: "town-spyglass-night-window-edge",
            options: town_spyglass_night_edge,
            script: &["USP"],
            expected: RouteSmokeExpectation::Town(minoc),
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
            name: "britannia-rel-hur-east",
            options: rel_hur,
            script: &["C1HR6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-light-open-spell-route",
            options: light_open,
            script: &["C1IL", "C1LV", "C1AS6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 3,
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
            name: "dungeon-open-chest-command",
            options: dungeon_options.clone(),
            script: &["O"],
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
            name: "castle-jimmy-magic-lock-no-picker",
            options: town_jimmy_no_roll.clone(),
            script: &["J", "6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-jimmy-empty-restraint-no-picker",
            options: town_jimmy_no_roll,
            script: &["J", "6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-jimmy-prisoner-release",
            options: town_jimmy_release.clone(),
            script: &["J", "6", "1"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "reload-castle-jimmy-prisoner-release",
            options: town_jimmy_release,
            script: &["J", "6", "1", "empty"],
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
            options: board_horse.clone(),
            script: &["B"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "reload-boarded-horse-pass",
            options: board_horse,
            script: &["B", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "gate-travel-world-to-underworld",
            options: gate_travel_to_underworld.clone(),
            script: &["C1PRV1"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "reload-gate-travel-underworld-pass",
            options: gate_travel_to_underworld,
            script: &["C1PRV1", "empty"],
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
            options: chasm_fall.clone(),
            // The chain runs off the post-action pass, so any turn-consuming
            // command on the brink reaches it; stepping south is refused by
            // the waterfall tile itself.
            script: &["empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "reload-chasm-underworld-pass",
            options: chasm_fall,
            script: &["empty", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-whirlpool-forced-underworld",
            options: whirlpool_forced_underworld,
            script: &[],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-pirate-broadside-damages-the-party",
            options: world.clone(),
            script: &["empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-pirate-broadside-spends-ship-hull",
            options: world.clone(),
            script: &["empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-fixed-narrative-gate-unordained-refusal",
            options: narrative_gate_unordained_refusal,
            script: &["empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "britannia-fixed-narrative-gate-ordained-passage",
            options: narrative_gate_ordained_passage,
            script: &["empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
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
            options: fixed_hidden_underworld_stack.clone(),
            script: &["S6", "G6", "S6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 3,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "reload-underworld-fixed-hidden-stack-search-get-search",
            options: fixed_hidden_underworld_stack,
            script: &["S6", "G6", "S6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Underworld),
            min_turn: 1,
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
            script: &ENDGAME_FULL_VICTORY_CINEMATIC_SCRIPT,
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
            name: "britannia-defeat-persists-ool-before-rescue",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["q"],
            expected: RouteSmokeExpectation::Town(blackthorn_rescue_scene),
            min_turn: 0,
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
            name: "stonegate-trapdoor-rescue",
            options: PlayOptions {
                target: PlayTarget::Town(stonegate),
                ..PlayOptions::default()
            },
            script: &["empty", "q"],
            expected: RouteSmokeExpectation::Town(blackthorn_rescue_scene),
            min_turn: 1,
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
            name: "britannia-empty-yell-is-acted",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["Y", "empty"],
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
            name: "britannia-ruined-honesty-shrine-restoration",
            options: world.clone(),
            script: &["YFALLAX", "Honesty", "Ahm", "Ahm", "Ahm"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
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
            options: fixed_hidden_daily.clone(),
            script: &["S6", "G6", "S6"],
            expected: RouteSmokeExpectation::Town(minoc),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "reload-minoc-fixed-hidden-daily-search-get-repeat",
            options: fixed_hidden_daily,
            script: &["S6", "G6", "S6"],
            expected: RouteSmokeExpectation::Town(minoc),
            min_turn: 0,
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
        // `systems/magic.md §8`: X-Ray (*Wis An Ylem*) is the second caller of
        // the shared visibility sweep, and `catalogs/item-list.md §7.2` says
        // that branch "does not ... enter the modal View overlay" (R327). The
        // frame it leaves is therefore the ordinary eleven-by-eleven raster
        // with every cell revealed, not the 32x32 class map.
        RouteSmokeCase {
            name: "castle-x-ray-sweep",
            options: x_ray_view,
            script: &["C1AWY"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
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
            name: "castle-town-attack-death-mask-npc",
            options: PlayOptions::default(),
            script: &["A6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            // `town-mode.md §14`: A-Attack at a town NPC **does** call the
            // combat framer and swap to a `.CBT` arena. The reading where it
            // stayed inside town mode is withdrawn, so the frame this route
            // ends on is the combat raster, not the town tile viewport.
            name: "castle-town-attack-guard-alarm",
            options: PlayOptions::default(),
            script: &["A6"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "castle-town-hostile-adjacent-alarm",
            options: PlayOptions::default(),
            script: &["empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-town-guard-arrest-refusal",
            options: PlayOptions::default(),
            script: &["empty", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "castle-town-guard-arrest-surrender-yew",
            options: PlayOptions::default(),
            script: &["empty", "Y"],
            expected: RouteSmokeExpectation::Town(yew_poster_scene),
            min_turn: 1,
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
            name: "castle-talk-ordinary-keyword-route",
            options: PlayOptions::default(),
            script: &["T", "6", "NAME"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
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
            script: &["B", "A", "N", "S", "empty", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-iolos-bows-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-naughty-nomaans-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-arms-of-justice-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-darkwatch-armoury-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-paladins-protectorate-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-north-star-armoury-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-buccaneers-booty-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-shattered-shield-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-siege-crafters-buy-first",
            options: PlayOptions::default(),
            script: &["B", "A", "Y", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-iolos-bows-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-naughty-nomaans-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-arms-of-justice-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-darkwatch-armoury-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-paladins-protectorate-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-north-star-armoury-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-buccaneers-booty-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-shattered-shield-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-arms-siege-crafters-terminator-refusal",
            options: PlayOptions::default(),
            script: &["B", "H", "\x1b"],
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
            script: &["Y", "M", "Y", "R", "1", "N"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-honest-meal-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "A", "Y", "C", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-wayfarer-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "A", "Y", "C", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-sword-and-keg-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "A", "Y", "C", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-slaughtered-lamb-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "R", "Y", "H", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-humble-palate-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "S", "Y", "A", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-blue-boar-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "C", "Y", "T", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-cats-lair-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "A", "Y", "C", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-fallen-virgin-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "R", "Y", "H", "HONE", "Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "shop-tavern-folley-tap-lore-route",
            options: PlayOptions::default(),
            script: &["Y", "A", "Y", "C", "HONE", "Y"],
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
            name: "reload-horse-trader-horse-and-rider-buy-pass",
            options: PlayOptions::default(),
            script: &["B", "Y", "empty"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
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
            // Public #94 enters every town-family scene at (15, 30).
            // Cross the south edge from that fixed cell.
            script: &["e", "s", "s", "Y"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "debug-enter-castle-from-underworld",
            options: underworld_to_castle,
            script: &["e", "empty"],
            expected: RouteSmokeExpectation::Town(Scene::new(25).expect("Ararat scene is valid")),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-xit-launches-skiff",
            options: ship_xit.clone(),
            script: &["X", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 2,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-xit-no-skiffs-refusal",
            options: ship_xit_no_skiffs,
            script: &["X"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "reload-ship-xit-skiff-pass",
            options: ship_xit,
            script: &["X", "empty"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
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
            name: "ship-yell-toggles-town-band",
            options: ship_yell_town,
            script: &["Y"],
            expected: RouteSmokeExpectation::Town(castle),
            min_turn: 1,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "ship-yell-toggles-dungeon-band",
            options: ship_yell_dungeon,
            script: &["Y"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "dungeon first-person viewport",
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
            name: "dungeon-long-camp-recovery",
            options: dungeon_long_camp_recovery,
            script: &["H6/4"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 18,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-camp-inside-cooldown-window",
            options: dungeon_camp_inside_cooldown,
            script: &["H6/4"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 18,
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
            name: "reload-dungeon-ladder-down-up-route",
            options: dungeon_options.clone(),
            script: &[">", "<"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
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
            name: "reload-dungeon-surface-exit-return-world",
            options: dungeon_options.clone(),
            script: &["K", "empty"],
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
            name: "dungeon-jimmy-no-keys-commits-action",
            options: dungeon_options.clone(),
            script: &["J", "1"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-jimmy-no-lock-commits-action",
            options: dungeon_options.clone(),
            script: &["J", "1"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-jimmy-cancel-commits-action",
            options: dungeon_options.clone(),
            script: &["J", "\x1b"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
            expected_frame_kind: "dungeon first-person viewport",
        },
        RouteSmokeCase {
            name: "dungeon-jimmy-success-clears-trap-subtype",
            options: dungeon_options.clone(),
            script: &["J", "1"],
            expected: RouteSmokeExpectation::Dungeon(dungeon),
            min_turn: 1,
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
            name: "terrain-combat-party-entry",
            options: world.clone(),
            script: &[],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "dungeon-room-party-entry",
            options: doom_options.clone(),
            script: &[],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 0,
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
            name: "doom-combat-escape-not-yet",
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
            name: "doom-combat-use-picker",
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
            name: "combat-field-fire-marker-placement",
            options: world.clone(),
            script: &["C1FGI6,5"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-poison-marker-placement",
            options: world.clone(),
            script: &["C1GIN6,5"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-sleep-marker-placement",
            options: world.clone(),
            script: &["C1GIZ6,5"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-energy-marker-placement",
            options: world.clone(),
            script: &["C1GIS6,5"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-dispel-fire-marker",
            options: world.clone(),
            script: &["C1AG6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-field-dispel-empty-refusal",
            options: world.clone(),
            script: &["C1AG6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-utility-vanish-tile",
            options: world.clone(),
            script: &["C1AY6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-utility-open-tile",
            options: world.clone(),
            script: &["C1AS6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-utility-magic-lock-tile",
            options: world.clone(),
            script: &["C1AEP6"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-utility-unlock-magic-tile",
            options: world.clone(),
            script: &["C1EIP6"],
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
            name: "combat-fireball-target",
            options: world.clone(),
            script: &["C1FV7"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-reveal-hidden-target",
            options: world.clone(),
            script: &["C1QW"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-invisibility-caster",
            options: world.clone(),
            script: &["C1LS"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-cause-fear-target",
            options: world.clone(),
            script: &["C1CIQ"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-mass-charm-effect",
            options: world.clone(),
            script: &["C1AQW"],
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
            name: "combat-kill-gazer-eye-burst",
            options: world.clone(),
            script: &["C1CX7"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-kill-gargoyle-lava-marker",
            options: world.clone(),
            script: &["C1CX7"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "combat-kill-shadowlord-protected-rejection",
            options: world.clone(),
            script: &["C1CX7"],
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
            name: "doom-combat-xit-refusal",
            options: doom_options.clone(),
            script: &["empty", "X"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "doom-combat-quit-refusal",
            options: doom_options.clone(),
            script: &["q"],
            expected: RouteSmokeExpectation::Dungeon(doom),
            min_turn: 0,
            expected_frame_kind: "combat viewport",
        },
        RouteSmokeCase {
            name: "terrain-combat-escape-announced-cleanup",
            options: world.clone(),
            script: &["\x1b"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        },
        RouteSmokeCase {
            name: "terrain-combat-out-of-arena-leave",
            options: world.clone(),
            script: &["d"],
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
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
            script: &["w", "w", "a", "a", "s", "d", "Z", "empty", "l6", "empty"],
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
    ];
    append_directed_wind_route_smoke_cases(&mut cases, world.clone());
    append_asset_backed_conversation_route_smoke_cases(&mut cases);
    append_shrine_route_smoke_cases(&mut cases);
    append_public_location_route_smoke_cases(&mut cases);
    append_harpsichord_route_smoke_cases(&mut cases, castle);
    cases
}

/// `town-mode.md §13` + `commands.md §3` harpsichord routes, replayed against
/// the shipped Lord British's Castle floor `+2` map.
///
/// The unit tests in `u5-runtime` stamp the instrument into a synthetic grid,
/// so they cannot notice the tile moving on the real map. These routes can:
/// every one of them starts the party on the shipped chair coordinate and
/// asserts the shipped wall byte before it asserts the rewrite.
fn append_harpsichord_route_smoke_cases(cases: &mut Vec<RouteSmokeCase>, castle: Scene) {
    let seated = PlayOptions {
        target: PlayTarget::Town(castle),
        floor: HARPSICHORD_FLOOR,
        start: Some((HARPSICHORD_ROUTE_X, HARPSICHORD_ROUTE_CHAIR_Y)),
        ..PlayOptions::default()
    };
    let off_chair = PlayOptions {
        target: PlayTarget::Town(castle),
        floor: HARPSICHORD_FLOOR,
        start: Some((HARPSICHORD_ROUTE_X, HARPSICHORD_ROUTE_OFF_CHAIR_Y)),
        ..PlayOptions::default()
    };

    // The whole tune, keyed from the chair.
    let full_tune: &'static [&'static str] =
        Box::leak(HARPSICHORD_TUNE_SCRIPT.to_vec().into_boxed_slice());
    // Twelve notes: one short of completion, so the wall must still stand.
    let short_tune: &'static [&'static str] =
        Box::leak(HARPSICHORD_TUNE_SCRIPT[..12].to_vec().into_boxed_slice());
    // Ten correct notes, a stray `8`, then the tune from note three - the
    // continuation only completes if the stray left the player three notes in
    // rather than starting them over.
    let resync_after_ten: &'static [&'static str] = Box::leak(
        [
            &HARPSICHORD_TUNE_SCRIPT[..10],
            &["8"][..],
            &HARPSICHORD_TUNE_SCRIPT[3..],
        ]
        .concat()
        .into_boxed_slice(),
    );
    // Eleven correct notes, a stray `7`, then the tune from note two.
    let resync_after_eleven: &'static [&'static str] = Box::leak(
        [
            &HARPSICHORD_TUNE_SCRIPT[..11],
            &["7"][..],
            &HARPSICHORD_TUNE_SCRIPT[2..],
        ]
        .concat()
        .into_boxed_slice(),
    );
    // The whole tune, then four cardinal steps to the floor `+2` ascend link
    // and a klimb up and straight back down. Movement uses letter keys because
    // the digit keys belong to the instrument while the party is still seated.
    let tune_then_floor_round_trip: &'static [&'static str] = Box::leak(
        [
            &HARPSICHORD_TUNE_SCRIPT[..],
            &["w", "a", "a", "w", "K", "K"][..],
        ]
        .concat()
        .into_boxed_slice(),
    );
    // The whole tune, then one command that carries the save/reload checkpoint.
    let tune_then_reload: &'static [&'static str] = Box::leak(
        [&HARPSICHORD_TUNE_SCRIPT[..], &["empty"][..]]
            .concat()
            .into_boxed_slice(),
    );

    for (name, options, script, min_turn) in [
        (
            "castle-harpsichord-tune-opens-passage",
            seated.clone(),
            full_tune,
            0u64,
        ),
        (
            "castle-harpsichord-digits-consume-no-turn",
            seated.clone(),
            short_tune,
            0,
        ),
        (
            "castle-harpsichord-resync-after-ten-notes-and-stray-eight",
            seated.clone(),
            resync_after_ten,
            0,
        ),
        (
            "castle-harpsichord-resync-after-eleven-notes-and-stray-seven",
            seated.clone(),
            resync_after_eleven,
            0,
        ),
        (
            "castle-harpsichord-digit-is-an-ordinary-command-off-the-chair",
            off_chair,
            &["8", "5"][..],
            1,
        ),
        (
            "castle-harpsichord-passage-lost-on-floor-round-trip",
            seated.clone(),
            tune_then_floor_round_trip,
            2,
        ),
        (
            "reload-castle-harpsichord-passage-lost-on-reload",
            seated,
            tune_then_reload,
            0,
        ),
    ] {
        cases.push(RouteSmokeCase {
            name,
            options,
            script,
            expected: RouteSmokeExpectation::Town(castle),
            min_turn,
            expected_frame_kind: "tile viewport",
        });
    }
}

fn append_directed_wind_route_smoke_cases(cases: &mut Vec<RouteSmokeCase>, world: PlayOptions) {
    for (name, script) in [
        ("combat-directed-sleep-cone", &["C1IZ6"][..]),
        ("combat-directed-sleep-cone-north", &["C1IZ8"][..]),
        ("combat-directed-sleep-cone-east", &["C1IZ6"][..]),
        ("combat-directed-sleep-cone-south", &["C1IZ2"][..]),
        ("combat-directed-sleep-cone-west", &["C1IZ4"][..]),
        ("combat-directed-poison-wind-cone", &["C1HIN6"][..]),
        ("combat-directed-poison-wind-cone-north", &["C1HIN8"][..]),
        ("combat-directed-poison-wind-cone-east", &["C1HIN6"][..]),
        ("combat-directed-poison-wind-cone-south", &["C1HIN2"][..]),
        ("combat-directed-poison-wind-cone-west", &["C1HIN4"][..]),
        ("combat-directed-death-wind-cone", &["C1CGIV6"][..]),
        ("combat-directed-death-wind-cone-north", &["C1CGIV8"][..]),
        ("combat-directed-death-wind-cone-east", &["C1CGIV6"][..]),
        ("combat-directed-death-wind-cone-south", &["C1CGIV2"][..]),
        ("combat-directed-death-wind-cone-west", &["C1CGIV4"][..]),
        ("combat-directed-flame-wind-cone", &["C1FHI6"][..]),
        ("combat-directed-flame-wind-cone-north", &["C1FHI8"][..]),
        ("combat-directed-flame-wind-cone-east", &["C1FHI6"][..]),
        ("combat-directed-flame-wind-cone-south", &["C1FHI2"][..]),
        ("combat-directed-flame-wind-cone-west", &["C1FHI4"][..]),
    ] {
        cases.push(RouteSmokeCase {
            name,
            options: world.clone(),
            script,
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 1,
            expected_frame_kind: "combat viewport",
        });
    }
}

fn append_asset_backed_conversation_route_smoke_cases(cases: &mut Vec<RouteSmokeCase>) {
    for (family, scene_range) in [
        ("towne", 1u8..=8u8),
        ("dwelling", 9u8..=16u8),
        ("castle", 17u8..=24u8),
        ("keep", 25u8..=32u8),
    ] {
        let representative_scene = *scene_range.start();
        for scene_byte in scene_range {
            let scene = Scene::new(scene_byte).expect("representative TLK family scene is valid");
            for (kind, command) in [
                ("reserved-name", "NAME"),
                ("reserved-job", "JOB"),
                ("reserved-work", "WORK"),
                ("reserved-bye", "BYE"),
                ("reserved-thank", "THANK"),
                ("ordinary-no-match", "XYZZY"),
            ] {
                let name: &'static str = if scene_byte == representative_scene {
                    Box::leak(format!("talk-{family}-{kind}").into_boxed_str())
                } else {
                    Box::leak(format!("talk-{family}-{scene_byte:02}-{kind}").into_boxed_str())
                };
                let command: &'static str = Box::leak(command.to_string().into_boxed_str());
                let script: &'static [&'static str] =
                    Box::leak(vec!["T", "6", command].into_boxed_slice());
                cases.push(RouteSmokeCase {
                    name,
                    options: PlayOptions {
                        target: PlayTarget::Town(scene),
                        shadowlord_hideouts: [SHADOWLORD_VANQUISHED; 3],
                        ..PlayOptions::default()
                    },
                    script,
                    expected: RouteSmokeExpectation::Town(scene),
                    min_turn: 1,
                    expected_frame_kind: "tile viewport",
                });
            }
        }
    }
}

fn append_shrine_route_smoke_cases(cases: &mut Vec<RouteSmokeCase>) {
    for virtue in ShrineVirtue::ALL {
        let name: &'static str = Box::leak(
            format!(
                "shrine-native-{}-meditation",
                virtue.name().to_ascii_lowercase()
            )
            .into_boxed_str(),
        );
        let command: &'static str = Box::leak(format!("M{}", virtue.mantra()).into_boxed_str());
        let script: &'static [&'static str] = Box::leak(vec![command].into_boxed_slice());
        cases.push(RouteSmokeCase {
            name,
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script,
            expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
            min_turn: 0,
            expected_frame_kind: "tile viewport",
        });
    }
    cases.push(RouteSmokeCase {
        name: "codex-urn-honesty-read",
        options: PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            ..PlayOptions::default()
        },
        script: &["M"],
        expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
        min_turn: 0,
        expected_frame_kind: "tile viewport",
    });
    cases.push(RouteSmokeCase {
        name: "shrine-honesty-codex-turn-in",
        options: PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            ..PlayOptions::default()
        },
        script: &["MAhm"],
        expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
        min_turn: 0,
        expected_frame_kind: "tile viewport",
    });
    cases.push(RouteSmokeCase {
        name: "shrine-compassion-completed-offering",
        options: PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            gold: 500,
            ..PlayOptions::default()
        },
        script: &["MMu/1"],
        expected: RouteSmokeExpectation::World(WorldPlane::Britannia),
        min_turn: 0,
        expected_frame_kind: "tile viewport",
    });
}

fn append_public_location_route_smoke_cases(cases: &mut Vec<RouteSmokeCase>) {
    for (index, entry) in published_world_location_entries().into_iter().enumerate() {
        let name: &'static str =
            Box::leak(format!("stock-location-enter-{:02}", index + 1).into_boxed_str());
        let mut options = PlayOptions {
            target: PlayTarget::World(entry.plane),
            ..PlayOptions::default()
        };
        if matches!(entry.target, PlayTarget::Dungeon(scene) if scene.record == 7) {
            options.shadowlord_hideouts = [SHADOWLORD_VANQUISHED; 3];
        }
        let (expected, expected_frame_kind) = match entry.target {
            PlayTarget::Town(scene) => (RouteSmokeExpectation::Town(scene), "tile viewport"),
            PlayTarget::Dungeon(scene) => (
                RouteSmokeExpectation::Dungeon(scene),
                "dungeon first-person viewport",
            ),
            PlayTarget::World(_) => continue,
        };
        cases.push(RouteSmokeCase {
            name,
            options,
            script: &["e"],
            expected,
            min_turn: 0,
            expected_frame_kind,
        });
    }
}

fn seed_gate_travel_resources(options: &mut PlayOptions) {
    options.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    if let Some(caster) = options.party.first_mut() {
        caster.mana = GATE_TRAVEL_COST + 1;
        caster.level = GATE_TRAVEL_COST;
    }
}

pub fn run_route_smoke(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    manifest_path: Option<&Path>,
) -> io::Result<()> {
    let cases = route_smoke_cases();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    let baseline_brit_ool = fs::read(game_dir.join(BRIT_OOL_FILENAME))?;
    let baseline_under_ool = fs::read(game_dir.join(UNDER_OOL_FILENAME))?;
    println!("Route smoke: {} case(s).", cases.len());
    let mut reports = Vec::with_capacity(cases.len());
    for case in &cases {
        fs::write(game_dir.join(BRIT_OOL_FILENAME), &baseline_brit_ool)?;
        fs::write(game_dir.join(UNDER_OOL_FILENAME), &baseline_under_ool)?;
        let report = run_route_smoke_case(game_dir, &atlas, case)?;
        println!(
            "route-smoke {}: {} command(s), {}",
            report.name, report.commands_run, report.final_state_line
        );
        println!("{}", report.final_raster_line);
        reports.push(report);
    }
    fs::write(game_dir.join(BRIT_OOL_FILENAME), &baseline_brit_ool)?;
    fs::write(game_dir.join(UNDER_OOL_FILENAME), &baseline_under_ool)?;
    if let Some(path) = manifest_path {
        write_route_smoke_manifest(path, &reports)?;
        println!("Saved route smoke manifest: {}.", path.display());
    }
    println!("Route smoke: all cases passed.");
    Ok(())
}

pub fn run_route_smoke_case(
    game_dir: &Path,
    atlas: &u5_runtime::TileAtlas,
    case: &RouteSmokeCase,
) -> io::Result<RouteSmokeReport> {
    let route_game_dir = prepare_route_smoke_case_game_dir(game_dir, case.name)?;
    let reload_save_dir = prepare_route_smoke_reload_save_dir(game_dir, case.name)?;
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
    let initial_metadata = vec![
        "phase=initial".to_string(),
        format!("commands {}", case.script.len()),
        play_script_state_line(&state),
    ];
    let mut frames = vec![capture_route_smoke_frame(
        &mut state,
        atlas,
        &format!("route-{}-00-initial", case.name),
        initial_metadata,
    )?];

    let reload_checkpoints = route_reload_checkpoints(case.name);
    let result = replay_play_script_commands(
        &mut state,
        command_game_dir,
        &commands,
        |state, index, command| {
            commands_run += 1;
            if reload_checkpoints.contains(&(index + 1)) {
                let Some(save_dir) = reload_save_dir.as_deref() else {
                    return Err(io::Error::other(format!(
                        "route smoke `{}` has reload checkpoints but no temp save dir",
                        case.name
                    )));
                };
                reload_route_smoke_state_from_checkpoint(state, game_dir, save_dir)?;
            }
            let raster = raster_diagnostic_line(state, VIEWPORT_RADIUS, atlas)?;
            require_raster_hash(case, &raster)?;
            let metadata = vec![
                "phase=step".to_string(),
                format!("step {}", index + 1),
                format!("input={}", sanitize_manifest_field(command)),
                play_script_state_line(state),
            ];
            frames.push(capture_route_smoke_frame(
                state,
                atlas,
                &format!(
                    "route-{}-{:02}-{}",
                    case.name,
                    index + 1,
                    sanitize_route_label_fragment(&play_script_command_label(command))
                ),
                metadata,
            )?);
            Ok(())
        },
    );
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
    validate_route_smoke_case_state(&state, case.name, command_game_dir)?;

    let final_raster_line = raster_diagnostic_line(&mut state, VIEWPORT_RADIUS, atlas)?;
    require_raster_hash(case, &final_raster_line)?;
    let final_viewport = state
        .render_top_down_frame(VIEWPORT_RADIUS, atlas)?
        .ok_or_else(|| {
            io::Error::other(format!("route smoke `{}` has no final raster", case.name))
        })?;
    let final_nonblack_pixels = final_viewport
        .pixels
        .iter()
        .filter(|pixel| **pixel != 0)
        .count();
    frames.push(RouteSmokeFrameReport {
        label: format!("route-{}", case.name),
        frame_kind: final_frame_kind.to_string(),
        width: final_viewport.width,
        height: final_viewport.height,
        hash: hash_palette_indices(&final_viewport.pixels),
        nonblack_pixels: final_nonblack_pixels,
        metadata: vec![
            "phase=final".to_string(),
            format!("commands {}", commands_run),
            play_script_state_line(&state),
        ],
    });
    let report = RouteSmokeReport {
        name: case.name.to_string(),
        commands_run,
        final_state_line: play_script_state_line(&state),
        final_raster_line,
        final_frame_kind: final_frame_kind.to_string(),
        final_width: final_viewport.width,
        final_height: final_viewport.height,
        final_hash: hash_palette_indices(&final_viewport.pixels),
        final_nonblack_pixels,
        frames,
    };
    if let Some(dir) = &route_game_dir {
        let _ = fs::remove_dir_all(dir);
    }
    if let Some(dir) = &reload_save_dir {
        let _ = fs::remove_dir_all(dir);
    }
    Ok(report)
}

pub fn write_route_smoke_manifest(path: &Path, reports: &[RouteSmokeReport]) -> io::Result<()> {
    let mut manifest = String::new();
    manifest.push_str("# Ultima V route smoke manifest\n");
    manifest.push_str(
        "# Sanitized: contains route labels, command counts, dimensions, frame hashes, and state hashes only.\n",
    );
    manifest.push_str(&format!("coverage\ttotal-routes\t{}\n", reports.len()));
    let total_frames: usize = reports.iter().map(|report| report.frames.len()).sum();
    manifest.push_str(&format!("coverage\ttotal-route-frames\t{total_frames}\n"));
    manifest.push_str("# label\tdimensions\tframe-kind\thash\tnonblack\treview-metadata\n");
    for report in reports {
        for frame in &report.frames {
            manifest.push_str(&format!(
                "{}\t{}x{}\t{}\thash {:016x}\tnonblack {}\t{}\n",
                sanitize_manifest_field(&frame.label),
                frame.width,
                frame.height,
                sanitize_manifest_field(&frame.frame_kind),
                frame.hash,
                frame.nonblack_pixels,
                frame
                    .metadata
                    .iter()
                    .map(|value| sanitize_manifest_field(value))
                    .collect::<Vec<_>>()
                    .join("\t"),
            ));
        }
    }
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, manifest)
}

fn sanitize_manifest_field(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

fn sanitize_route_label_fragment(value: &str) -> String {
    let mut label = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            label.push(ch.to_ascii_lowercase());
        } else if ch == '_' || ch == '-' {
            label.push(ch);
        } else if !label.ends_with('_') {
            label.push('_');
        }
    }
    let trimmed = label.trim_matches('_');
    if trimmed.is_empty() {
        "empty".to_string()
    } else {
        trimmed.to_string()
    }
}

fn capture_route_smoke_frame(
    state: &mut PlayState,
    atlas: &u5_runtime::TileAtlas,
    label: &str,
    metadata: Vec<String>,
) -> io::Result<RouteSmokeFrameReport> {
    complete_headless_blocking_presentations(state, Some(atlas))?;
    let frame_kind = raster_frame_kind(state).to_string();
    let viewport = state
        .render_top_down_frame(VIEWPORT_RADIUS, atlas)?
        .ok_or_else(|| {
            io::Error::other(format!(
                "route smoke frame `{label}` has no renderable viewport"
            ))
        })?;
    let nonblack_pixels = viewport.pixels.iter().filter(|pixel| **pixel != 0).count();
    let report = RouteSmokeFrameReport {
        label: label.to_string(),
        frame_kind,
        width: viewport.width,
        height: viewport.height,
        hash: hash_palette_indices(&viewport.pixels),
        nonblack_pixels,
        metadata,
    };
    // Dissolve records describe blocking calls that have already finished.
    // The captured caller-composed frame is this frontend's acknowledgement.
    let _ = state.take_pending_map_viewport_dissolves();
    let _ = state.take_pending_blackthorn_rescue_playbacks();
    let _ = state.take_pending_stonegate_trapdoor_playback();
    Ok(report)
}

fn prepare_route_smoke_case_game_dir(
    game_dir: &Path,
    case_name: &str,
) -> io::Result<Option<PathBuf>> {
    if case_name != "castle-poison-gas-step"
        && case_name != "codex-urn-honesty-read"
        && case_name != "britannia-defeat-persists-ool-before-rescue"
    {
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
    if case_name == "britannia-defeat-persists-ool-before-rescue" {
        for file_name in [BRIT_DAT_FILENAME, "CASTLE.DAT", "CASTLE.NPC", "CASTLE.TLK"] {
            u5_runtime::test_fixtures::copy_asset_writable(
                &game_dir.join(file_name),
                &dir.join(file_name),
            )?;
        }
        let karma = game_dir.join("KARMA.DAT");
        if karma.exists() {
            u5_runtime::test_fixtures::copy_asset_writable(&karma, &dir.join("KARMA.DAT"))?;
        }
    }
    if case_name == "codex-urn-honesty-read" {
        fs::write(
            dir.join(CODEX_URN_TABLE_FILE),
            format!("BRITANNIA 62 124 {SHRINE_ALTAR_TILE_FIRST}\n"),
        )?;
    }
    Ok(Some(dir))
}

fn prepare_route_smoke_reload_save_dir(
    game_dir: &Path,
    case_name: &str,
) -> io::Result<Option<PathBuf>> {
    if route_reload_checkpoints(case_name).is_empty() {
        return Ok(None);
    }
    let dir = route_smoke_temp_dir(case_name, "reload")?;
    seed_route_smoke_save_files(game_dir, &dir)?;
    Ok(Some(dir))
}

fn route_smoke_temp_dir(case_name: &str, label: &str) -> io::Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "u5-route-smoke-{case_name}-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn seed_route_smoke_save_files(game_dir: &Path, save_dir: &Path) -> io::Result<()> {
    if copy_route_smoke_save_file(game_dir, save_dir, SAVED_GAM_FILENAME, SAVED_GAM_FILENAME)
        .is_err()
    {
        copy_route_smoke_save_file(game_dir, save_dir, "INIT.GAM", SAVED_GAM_FILENAME)?;
    }
    if copy_route_smoke_save_file(game_dir, save_dir, SAVED_OOL_FILENAME, SAVED_OOL_FILENAME)
        .is_err()
    {
        if game_dir.join("INIT.OOL").exists() {
            let init_ool = fs::read(game_dir.join("INIT.OOL"))?;
            let mut saved_ool = vec![0; SAVED_OOL_LEN];
            let copied_len = init_ool.len().min(saved_ool.len() / 2);
            saved_ool[..copied_len].copy_from_slice(&init_ool[..copied_len]);
            fs::write(save_dir.join(SAVED_OOL_FILENAME), saved_ool)?;
        } else {
            fs::write(save_dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN])?;
        }
    }
    // Reload cases exercise the real save handler, whose published staging
    // input is the two per-plane mirrors rather than the combined save file.
    let saved_ool = fs::read(save_dir.join(SAVED_OOL_FILENAME))?;
    u5_runtime::write_saved_ool_mirrors(save_dir, &saved_ool)?;
    Ok(())
}

fn copy_route_smoke_save_file(
    game_dir: &Path,
    save_dir: &Path,
    source_name: &str,
    destination_name: &str,
) -> io::Result<()> {
    // The reload checkpoints write into `save_dir`, so the seeded copy
    // must not inherit the pristine install's read-only attribute.
    u5_runtime::test_fixtures::copy_asset_writable(
        &game_dir.join(source_name),
        &save_dir.join(destination_name),
    )
}

fn route_reload_checkpoints(case_name: &str) -> &'static [usize] {
    match case_name {
        "reload-gate-travel-underworld-pass"
        | "reload-chasm-underworld-pass"
        | "reload-boarded-horse-pass"
        | "reload-ship-xit-skiff-pass"
        | "reload-dungeon-ladder-down-up-route"
        | "reload-dungeon-surface-exit-return-world" => &[1],
        "reload-underworld-fixed-hidden-stack-search-get-search"
        | "reload-minoc-fixed-hidden-daily-search-get-repeat"
        | "reload-horse-trader-horse-and-rider-buy-pass" => &[2],
        "reload-castle-jimmy-prisoner-release" => &[3],
        // The reload fires after the thirteenth note, so the checkpoint sits
        // on the command that has just opened the passage.
        "reload-castle-harpsichord-passage-lost-on-reload" => HARPSICHORD_RELOAD_CHECKPOINTS,
        _ => &[],
    }
}

fn reload_route_smoke_state_from_checkpoint(
    state: &mut PlayState,
    game_dir: &Path,
    save_dir: &Path,
) -> io::Result<()> {
    state.save_game_command(save_dir, Some(true))?;
    let options = load_play_options_from_save(save_dir)?;
    *state = PlayState::load_scene(game_dir, options)?;
    Ok(())
}

fn apply_route_smoke_case_setup(
    state: &mut PlayState,
    case_name: &str,
    game_dir: &Path,
) -> io::Result<()> {
    // Route smoke is a fixed-input regression suite, so it must not inherit
    // the host-clock seed `prng.md` gives a fresh scene: `britannia-hole-up-rest`
    // took the sleep-ambush branch on roughly one run in four with it. Pinning
    // happens before the per-case setup below, so a case that needs a
    // particular stream still sets its own. Production seeding is untouched.
    state.prng_state = route_smoke_prng_seed();
    if let Some(index) = route_smoke_public_location_index(case_name) {
        seed_public_location_route_position(state, index)?;
    }
    // Legacy route labels retained for manifest stability. They now exercise
    // real published E-Enter rows; debug target bypass is no longer valid.
    if let Some(index) = match case_name {
        "debug-enter-castle" | "debug-enter-castle-return-world" => Some(16),
        "debug-enter-castle-from-underworld" => Some(24),
        "debug-enter-dungeon" => Some(32),
        _ => None,
    } {
        seed_public_location_route_position(state, index)?;
    }

    match case_name {
        "ship-yell-toggles-town-band" | "ship-yell-toggles-dungeon-band" => {
            state.player.transport = TransportState::Ship {
                type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
                tile: FIRST_PLAYABLE_FRIGATE_TILE,
                sails_hoisted: false,
                hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
                skiffs: 2,
            };
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
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
        "britannia-defeat-persists-ool-before-rescue" => {
            state
                .active_objects
                .resize(OOL_SLOTS, ActiveObject::empty());
            state.active_objects[31] = ActiveObject {
                type_byte: 0x71,
                tile: 0x72,
                x: 73,
                y: 74,
                z: WorldPlane::Britannia.save_floor(),
                phase: 0x76,
                aux1: 0x75,
                aux3: 0x77,
            };
            for member in &mut state.party {
                member.status = b'D';
                member.hp = 0;
            }
        }
        "stonegate-trapdoor-rescue" => {
            let index = state
                .grid
                .iter()
                .position(|tile| *tile == TOWN_TRAPDOOR_LIVE_TILE)
                .ok_or_else(|| {
                    io::Error::other(
                        "Stonegate runtime floor contains no live trapdoor tile for route smoke",
                    )
                })?;
            state.player.x = index % TOWN_GRID_SIDE;
            state.player.y = index / TOWN_GRID_SIDE;
            state.force_foot_transport();
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "castle-light-open-spell-route" => {
            state.player.x = 1;
            state.player.y = 1;
            state.player.facing = Direction::East;
            let target = state.player.y * TOWN_GRID_SIDE + state.player.x + 1;
            if let Some(cell) = state.grid.get_mut(target) {
                *cell = 0xB9;
            }
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        case_name if directed_wind_route_config(case_name).is_some() => {
            let (
                spell_index,
                cost,
                party_count,
                target_party_slot,
                include_monster_target,
                direction,
            ) = directed_wind_route_config(case_name).expect("directed wind route config exists");
            seed_directed_wind_combat_route(
                state,
                spell_index,
                cost,
                party_count,
                target_party_slot,
                include_monster_target,
                direction,
            )?;
            if spell_index == POISON_WIND_SPELL_INDEX {
                state.prng_state = poison_wind_first_accept_seed();
            } else if spell_index == FLAME_WIND_SPELL_INDEX {
                // Keep the reserve monster's automatic post-cast turn
                // deterministic. A host-clock seed can otherwise make it
                // leave combat before the route captures its combat raster.
                state.prng_state = 0;
            }
        }
        "combat-field-fire-marker-placement"
        | "combat-field-poison-marker-placement"
        | "combat-field-sleep-marker-placement"
        | "combat-field-energy-marker-placement" => {
            let (spell_index, cost, _) = combat_field_route_spell(case_name);
            seed_combat_field_route(state, spell_index, cost)?;
        }
        "combat-field-dispel-fire-marker" => {
            seed_combat_field_dispel_route(state, Some(CombatArenaFieldKind::Fire))?;
        }
        "combat-field-dispel-empty-refusal" => {
            seed_combat_field_dispel_route(state, None)?;
        }
        "combat-utility-vanish-tile"
        | "combat-utility-open-tile"
        | "combat-utility-magic-lock-tile"
        | "combat-utility-unlock-magic-tile" => {
            let (spell_index, cost, source_tile, _, _) = combat_utility_route_spell(case_name);
            seed_combat_utility_tile_route(state, spell_index, cost, source_tile)?;
        }
        "combat-kill-gazer-eye-burst" => {
            seed_combat_special_death_route(state, 28)?;
        }
        "combat-kill-gargoyle-lava-marker" => {
            seed_combat_special_death_route(state, 30)?;
        }
        "combat-kill-shadowlord-protected-rejection" => {
            seed_combat_special_death_route(state, 47)?;
            seed_route_combat_pending_party_actor(state);
        }
        "terrain-combat-party-entry" => {
            seed_terrain_combat_party_entry_route(state, game_dir)?;
        }
        "doom-combat-quit-refusal" => {
            seed_dungeon_room_party_entry_route(state, game_dir)?;
            seed_route_combat_pending_party_actor(state);
        }
        "terrain-combat-escape-announced-cleanup" => {
            seed_terrain_combat_party_entry_route(state, game_dir)?;
            clear_route_combat_non_party_actors(state);
            seed_route_combat_pending_party_actor(state);
            if let Some(snapshot) = state.combat_frame_snapshot.as_mut() {
                snapshot.exit_announced = true;
            }
        }
        "terrain-combat-out-of-arena-leave" => {
            seed_terrain_combat_party_entry_route(state, game_dir)?;
            seed_route_combat_party_actor_at_east_edge(state);
        }
        "dungeon-room-party-entry" => {
            seed_dungeon_room_party_entry_route(state, game_dir)?;
        }
        "combat-magic-missile-target"
        | "combat-fireball-target"
        | "combat-reveal-hidden-target"
        | "combat-invisibility-caster"
        | "combat-cause-fear-target"
        | "combat-mass-charm-effect"
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
        "underworld-fixed-hidden-stack-search-get-search"
        | "reload-underworld-fixed-hidden-stack-search-get-search" => {
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
        "minoc-fixed-hidden-daily-search-get-repeat"
        | "reload-minoc-fixed-hidden-daily-search-get-repeat" => {
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
        "britannia-pirate-broadside-damages-the-party" => {
            seed_outdoor_ranged_attack_route(state, None);
        }
        "britannia-pirate-broadside-spends-ship-hull" => {
            // `vehicles.md §6`: a hull above the `[1, 30]` roll ceiling can
            // never be destroyed, so this arm always lands on the absorbed
            // branch and never on the loss-of-ship ladder.
            seed_outdoor_ranged_attack_route(state, Some(OUTDOOR_IMPACT_HULL_ROLL_HIGH + 1));
        }
        "britannia-whirlpool-forced-underworld" => {
            if state
                .apply_world_whirlpool_engagement(game_dir, WorldPlane::Britannia)?
                .is_none()
            {
                return Err(io::Error::other(
                    "seeded whirlpool route did not find adjacent whirlpool object",
                ));
            }
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
        "castle-jimmy-magic-lock-no-picker" => {
            seed_town_jimmy_unoccupied_target(state, TOWN_DOOR_MAGIC_PLAIN_TILE);
            state.party[0].climb_stat = 0;
            state.prng_state = 0x3456;
        }
        "castle-jimmy-empty-restraint-no-picker" => {
            seed_town_jimmy_unoccupied_target(state, JIMMY_STOCKS_TILE);
            state.party[0].climb_stat = 0;
            state.prng_state = 0x3456;
        }
        "castle-jimmy-prisoner-release" | "reload-castle-jimmy-prisoner-release" => {
            seed_town_jimmy_prisoner_route(state);
        }
        "britannia-board-horse-route" | "reload-boarded-horse-pass" => {
            seed_world_board_horse_route(state);
        }
        "castle-surface-fountain-look" => {
            stamp_town_route_look_tile(state, 0xD8);
        }
        "yew-wanted-poster-look" => {
            seed_yew_wanted_poster_route(state);
        }
        "castle-town-attack-death-mask-npc" => {
            seed_town_attack_death_mask_npc_route(state);
        }
        "castle-town-attack-guard-alarm" => {
            seed_town_attack_guard_alarm_route(state);
        }
        "castle-town-hostile-adjacent-alarm" => {
            seed_town_hostile_adjacent_alarm_route(state);
        }
        "castle-town-guard-arrest-refusal" | "castle-town-guard-arrest-surrender-yew" => {
            seed_town_guard_arrest_route(state);
        }
        "buccaneers-den-wishing-well-horse"
        | "buccaneers-den-wishing-well-ferrari-grants-horse" => {
            stamp_town_route_look_tile(state, 0xA1);
        }
        "castle-death-vision-look" => {
            // `view.md §3` entry-dispatch row 2 tests the live
            // terrain-layer byte, not an active-object descriptor.
            stamp_town_route_look_tile(state, DEATH_VISION_LOOK_TILE);
        }
        "castle-talk-status-sleeping-refusal" => {
            seed_town_talk_status_tile_route(state, TALK_STATUS_TILE_SLEEPING);
        }
        "castle-talk-status-praying-refusal" => {
            seed_town_talk_status_tile_route(state, TALK_STATUS_TILE_PRAYING);
        }
        "castle-talk-ordinary-keyword-route" => {
            seed_town_ordinary_talk_route(state);
        }
        _ if asset_backed_conversation_route_family(case_name).is_some() => {
            seed_town_ordinary_talk_route(state);
        }
        _ if shrine_route_virtue(case_name).is_some() => {
            let virtue = shrine_route_virtue(case_name).expect("shrine route virtue is known");
            seed_world_shrine_route(state, virtue);
        }
        "codex-urn-honesty-read" => {
            seed_world_shrine_route(state, ShrineVirtue::Honesty);
            state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
            state.shrine_codex_mask = 0;
        }
        "shrine-honesty-codex-turn-in" => {
            seed_world_shrine_route(state, ShrineVirtue::Honesty);
            state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
            state.shrine_codex_mask = ShrineVirtue::Honesty.bit();
            state.moral_standing = 10;
        }
        "shrine-compassion-completed-offering" => {
            seed_world_shrine_route(state, ShrineVirtue::Compassion);
            state.shrine_ordained_mask = 0;
            state.shrine_codex_mask = ShrineVirtue::Compassion.bit();
            state.moral_standing = 10;
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
        "britannia-ruined-honesty-shrine-restoration" => {
            stamp_ruined_honesty_shrine_route(state);
        }
        "dungeon-ladder-down-up-route" | "reload-dungeon-ladder-down-up-route" => {
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
        "dungeon-long-camp-recovery" | "dungeon-camp-inside-cooldown-window" => {
            seed_long_camp_recovery_route(state);
        }
        "dungeon-hole-up-rest" | "dungeon-hole-up-no-direct-recovery" => {
            // These routes assert the completed-rest branch, so they must not
            // inherit the host-clock PRNG seed and intermittently enter the
            // separately covered sleep-ambush branch.
            state.prng_state = long_camp_no_ambush_seed();
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
        "dungeon-open-chest-spell" | "dungeon-open-chest-command" => {
            state.player.x = 1;
            state.player.y = 1;
            let current = dungeon_cell_index(0, state.player.x, state.player.y);
            if let Some(cell) = state.grid.get_mut(current) {
                *cell = 0x4b;
            }
            state.sync_player_object();
            state.mark_visibility_dirty();
        }
        "dungeon-surface-exit-return-world" | "reload-dungeon-surface-exit-return-world" => {
            // `dungeon-mode.md` §13: the climb-out route is an up ladder on
            // level zero, reaching the shared exit contract of §13.2. The
            // plain pit `0x60` this seed used is an ordinary descent; the
            // claim that it bypassed the level step is withdrawn.
            let current = dungeon_cell_index(0, state.player.x, state.player.y);
            if let Some(cell) = state.grid.get_mut(current) {
                *cell = 0x10;
            }
            // Deliberately seed a conflicting cached coordinate. The native
            // surface-reset contract must ignore it and use Deceit's public
            // world-location row `(240, 73)`.
            state.return_world = Some(WorldReturn {
                plane: WorldPlane::Britannia,
                x: 62,
                y: 124,
                transport: TransportState::Foot,
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
        "dungeon-jimmy-no-keys-commits-action"
        | "dungeon-jimmy-no-lock-commits-action"
        | "dungeon-jimmy-cancel-commits-action"
        | "dungeon-jimmy-success-clears-trap-subtype" => {
            let index = dungeon_cell_index(
                state.current_floor().unwrap_or(0) as u8,
                state.player.x,
                state.player.y,
            );
            state.grid[index] = match case_name {
                "dungeon-jimmy-no-lock-commits-action" => 0x00,
                _ => 0x4b,
            };
            state.keys = if case_name == "dungeon-jimmy-no-keys-commits-action" {
                0
            } else {
                2
            };
            state.party[0].climb_stat = 30;
            state.prng_state = 0x1234;
            state.mark_visibility_dirty();
        }
        "shop-arms-local-buy-sell-route" => {
            seed_route_arms_shop(state, ArmsShop::IolosBows, 999);
            state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
        }
        _ if arms_route_shop(case_name).is_some() => {
            let shop = arms_route_shop(case_name).expect("arms route shop is known");
            seed_route_arms_shop(state, shop, 9999);
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
        "shop-tavern-honest-meal-lore-route"
        | "shop-tavern-wayfarer-lore-route"
        | "shop-tavern-sword-and-keg-lore-route"
        | "shop-tavern-slaughtered-lamb-lore-route"
        | "shop-tavern-humble-palate-lore-route"
        | "shop-tavern-blue-boar-lore-route"
        | "shop-tavern-cats-lair-lore-route"
        | "shop-tavern-fallen-virgin-lore-route"
        | "shop-tavern-folley-tap-lore-route" => {
            state.gold = 999;
            state.prng_state = 0x3456;
            state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
                tavern_lore_route_tavern(case_name),
            )));
        }
        "shop-horse-trader-decline-route"
        | "shop-horse-trader-horse-and-rider-buy"
        | "reload-horse-trader-horse-and-rider-buy-pass"
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
    // This route starts at the public destruction instant, with the named
    // encounter on the Eternal Flame one cell north. The separate Yell route
    // pins the initial two-cells-north placement.
    let z = state.current_floor().expect("shard route is in a keep");
    let object = state
        .shadowlord_name_encounter_object(index, x, y.saturating_sub(1), z)
        .expect("shard route uses a valid Shadowlord index");
    state
        .allocate_highest_empty_active_object_slot(object)
        .expect("shard route has room for the summoned Shadowlord");
    state.summoned_shadowlord = Some(index);
    state.mark_visibility_dirty();
}

fn tavern_lore_route_tavern(case_name: &str) -> Tavern {
    match case_name {
        "shop-tavern-honest-meal-lore-route" => Tavern::TheHonestMeal,
        "shop-tavern-wayfarer-lore-route" => Tavern::TheWayfarerTavern,
        "shop-tavern-sword-and-keg-lore-route" => Tavern::TheSwordAndKeg,
        "shop-tavern-slaughtered-lamb-lore-route" => Tavern::TheSlaughteredLamb,
        "shop-tavern-humble-palate-lore-route" => Tavern::TheHumblePalate,
        "shop-tavern-blue-boar-lore-route" => Tavern::TheBlueBoarTavern,
        "shop-tavern-cats-lair-lore-route" => Tavern::TheCatsLair,
        "shop-tavern-fallen-virgin-lore-route" => Tavern::TheFallenVirgin,
        "shop-tavern-folley-tap-lore-route" => Tavern::TheFolleyTap,
        _ => Tavern::TheSwordAndKeg,
    }
}

fn seed_dungeon_active_monster_route(state: &mut PlayState, phase: u8) {
    state.player.x = 1;
    state.player.y = 1;
    state.player.facing = Direction::East;
    state.sync_player_object();
    state.active_objects.push(ActiveObject {
        type_byte: 0,
        tile: 0,
        x: 2,
        y: 1,
        z: state.current_floor().unwrap_or(0),
        phase,
        aux1: DUNGEON_MONSTER_COMBAT_CLASSES[0],
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
                state.player.x = (seal.x + 1) % WORLD_SIDE;
                state.player.y = seal.y;
                state.sync_player_object();
                let idx = world_cell_index(seal.x, seal.y);
                if let Some(cell) = state.grid.get_mut(idx) {
                    *cell = WORD_OF_POWER_SEALED_TILE;
                }
                state.word_of_power_seal_flags.fill(0);
                let _ = state.refresh_world_live_chunks_for_current_area();
                state.mark_visibility_dirty();
            }
        }
    }
}

fn stamp_ruined_honesty_shrine_route(state: &mut PlayState) {
    let (x, y) = WORLD_SHRINE_COORDINATES[0];
    state.player.x = (x + 1) % WORLD_SIDE;
    state.player.y = y;
    state.sync_player_object();
    state.grid[world_cell_index(x, y)] = WORLD_RUINED_SHRINE_TILE;
    state.shrine_ruin_flags[0] = 0x85;
    state.word_of_power_seal_flags[0] = 0x27;
    let _ = state.refresh_world_live_chunks_for_current_area();
    state.mark_visibility_dirty();
}

fn seed_route_arms_shop(state: &mut PlayState, shop: ArmsShop, gold: u16) {
    state.gold = gold;
    if let Some(intelligence) = state.party_intelligence.first_mut() {
        *intelligence = 20;
    }
    state.equipment_stock.fill(0);
    state.active_shop = Some(ActiveShopSession::ArmsLocal(ArmsShopState::Greeting, shop));
}

fn arms_route_shop(case_name: &str) -> Option<ArmsShop> {
    let shop = if case_name.contains("iolos-bows") {
        ArmsShop::IolosBows
    } else if case_name.contains("naughty-nomaans") {
        ArmsShop::NaughtyNomaans
    } else if case_name.contains("arms-of-justice") {
        ArmsShop::ArmsOfJustice
    } else if case_name.contains("darkwatch-armoury") {
        ArmsShop::DarkwatchArmoury
    } else if case_name.contains("paladins-protectorate") {
        ArmsShop::ThePaladinsProtectorate
    } else if case_name.contains("north-star-armoury") {
        ArmsShop::NorthStarArmoury
    } else if case_name.contains("buccaneers-booty") {
        ArmsShop::BuccaneersBooty
    } else if case_name.contains("shattered-shield") {
        ArmsShop::TheShatteredShield
    } else if case_name.contains("siege-crafters") {
        ArmsShop::SiegeCrafters
    } else {
        return None;
    };
    Some(shop)
}

fn is_arms_buy_first_route(case_name: &str) -> bool {
    case_name.starts_with("shop-arms-") && case_name.ends_with("-buy-first")
}

fn is_arms_terminator_refusal_route(case_name: &str) -> bool {
    case_name.starts_with("shop-arms-") && case_name.ends_with("-terminator-refusal")
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
    // `conversation.md`: the sleeping/praying Talk refusal is decided by the
    // live MAP tile the NPC stands on — a bed or an altar — not by the NPC's
    // renderer sprite frame. The engine read the sprite byte until the
    // conformance audit corrected it, so this fixture used to seed the sprite
    // and would now seed a status the gate never looks at. Write the map cell
    // the NPC's schedule puts it on, and leave the sprite an ordinary NPC.
    let npc_x = 16usize;
    let npc_y = 15usize;
    state.grid[npc_y * 32 + npc_x] = status_tile;
    if let Some(slot) = state.npcs.first().and_then(|npc| npc.active_object) {
        if let Some(object) = state.active_objects.get_mut(slot) {
            object.type_byte = 1;
        }
    }
    state.mark_visibility_dirty();
}

fn seed_town_ordinary_talk_route(state: &mut PlayState) {
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
            dialog_id: 2,
            schedule,
            name: None,
        },
    ]);
    state.mark_visibility_dirty();
}

/// Seeds the `overworld.md §6.2` outdoor ranged attack the ordinary routes
/// never reach: a pirate ship three cells east of the party, on a cleared
/// line, so the per-turn walker's broadside fires and the §6.2.4 payload
/// runs for real.
///
/// This exists because no natural route brings a hostile ship or a dragon
/// into range -- the subsystem's absence survived the whole suite. Passing
/// a hull puts the party aboard a frigate, which is the payload's other
/// branch.
fn seed_outdoor_ranged_attack_route(state: &mut PlayState, hull: Option<u8>) {
    let Area::World { plane } = state.area else {
        return;
    };
    let (px, py) = (state.player.x, state.player.y);

    // `overworld.md §6.2.2`: a broadside three cells out tests two cells,
    // one and two steps along the fire axis. Clear both so the line runs
    // clear and the payload actually runs.
    for step in 1..=2 {
        let index = world_cell_index(px + step, py);
        if let Some(cell) = state.grid.get_mut(index) {
            *cell = OUTDOOR_RANGED_ATTACK_ROUTE_CLEAR_TILE;
        }
    }

    for member in state.party.iter_mut() {
        member.status = b'G';
        member.hp = member.max_hp;
    }

    if let Some(hull) = hull {
        state.player.transport = TransportState::Ship {
            type_byte: TRANSPORT_MARKER_SHIP_FURLED_FIRST,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull,
            skiffs: 2,
        };
    }

    // `overworld.md §6.2.1`: the broadside row is a masked family test on
    // `0x2C..0x2F` and has no gate roll -- "it fires whenever the geometry
    // holds" -- so this reaches the payload deterministically.
    state.active_objects.push(ActiveObject {
        type_byte: OUTDOOR_RANGED_ATTACK_ROUTE_PIRATE_FRAME,
        tile: OUTDOOR_RANGED_ATTACK_ROUTE_PIRATE_FRAME,
        x: px + 3,
        y: py,
        z: plane.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    state.sync_player_object();
    state.mark_visibility_dirty();
}

/// Open low terrain, well outside `surface_tile_blocks_projectile`'s
/// blocking bands, so the seeded line is clear whatever the shipped map
/// holds there.
const OUTDOOR_RANGED_ATTACK_ROUTE_CLEAR_TILE: u8 = 0x03;

/// `encounters.md §4` pirate-ship / water-creature facing frames.
const OUTDOOR_RANGED_ATTACK_ROUTE_PIRATE_FRAME: u8 = 0x2C;

fn seed_world_shrine_route(state: &mut PlayState, virtue: ShrineVirtue) {
    state.area = Area::World {
        plane: WorldPlane::Britannia,
    };
    state.player.x = 62;
    state.player.y = 124;
    state.player.facing = Direction::East;
    let tile = SHRINE_ALTAR_TILE_FIRST + virtue.index() as u8;
    let idx = world_cell_index(state.player.x, state.player.y);
    if let Some(cell) = state.grid.get_mut(idx) {
        *cell = tile;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn shrine_route_virtue(case_name: &str) -> Option<ShrineVirtue> {
    let key = case_name
        .strip_prefix("shrine-native-")?
        .strip_suffix("-meditation")?;
    ShrineVirtue::from_key(key)
}

fn asset_backed_conversation_route_family(case_name: &str) -> Option<&str> {
    let rest = case_name.strip_prefix("talk-")?;
    let (family, kind) = rest.split_once('-')?;
    matches!(family, "towne" | "dwelling" | "castle" | "keep")
        .then_some(kind)
        .map(|_| family)
}

fn asset_backed_conversation_route_exits(case_name: &str) -> bool {
    case_name.ends_with("-reserved-bye") || case_name.ends_with("-reserved-thank")
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

fn seed_town_route_scheduled_npc(
    state: &mut PlayState,
    slot: usize,
    type_byte: u8,
    npc_x: usize,
    npc_y: usize,
    ai: u8,
) {
    state.player.x = npc_x.saturating_sub(1);
    state.player.y = npc_y;
    state.player.facing = Direction::East;
    state.clock = GameClock::new(8, 0).expect("route clock is valid");
    state.load_scheduled_npcs(&[
        NpcSlot {
            slot: 0,
            type_byte: 0,
            dialog_id: 0,
            schedule: [0; 16],
            name: None,
        },
        NpcSlot {
            slot,
            type_byte,
            dialog_id: 0,
            schedule: [
                ai,
                ai,
                ai,
                npc_x as u8,
                npc_x as u8,
                npc_x as u8,
                npc_y as u8,
                npc_y as u8,
                npc_y as u8,
                0,
                0,
                0,
                0,
                8,
                16,
                20,
            ],
            name: None,
        },
    ]);
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_town_attack_death_mask_npc_route(state: &mut PlayState) {
    seed_town_route_scheduled_npc(state, 1, 0x0E, 2, 1, 0);
}

fn seed_town_attack_guard_alarm_route(state: &mut PlayState) {
    seed_town_route_scheduled_npc(state, 1, 0x70, 2, 1, 0);
}

fn seed_town_hostile_adjacent_alarm_route(state: &mut PlayState) {
    seed_town_route_scheduled_npc(state, 1, 0x50, 6, 5, 4);
}

fn seed_town_guard_arrest_route(state: &mut PlayState) {
    seed_town_route_scheduled_npc(state, 2, 0x70, 6, 5, 6);
}

fn seed_town_jimmy_unoccupied_target(state: &mut PlayState, tile: u8) {
    state.player.x = 1;
    state.player.y = 1;
    state.player.facing = Direction::East;
    let floor = state.current_floor().unwrap_or(0);
    let target_x = 2;
    let target_y = 1;
    // Dropping the roster must drop the sprites the roster owned. An
    // orphaned NPC record keeps its roster tag, and `town-mode.md §16`
    // only skips *linked* NPC sprite classes from the free-roaming
    // walker - so leaving a castle stable horse (`catalogs/npc-roster.md
    // §4` tags `10`/`11`) behind here turns this Jimmy fixture into a
    // horse-wander fixture that spends PRNG draws the route is trying to
    // prove the Jimmy path never makes.
    for npc in &state.npcs {
        if let Some(slot) = npc.active_object {
            if let Some(object) = state.active_objects.get_mut(slot) {
                *object = ActiveObject::empty();
            }
        }
    }
    state.npcs.clear();
    for object in &mut state.active_objects {
        if !object.is_empty() && object.x == target_x && object.y == target_y && object.z == floor {
            *object = ActiveObject::empty();
        }
    }
    state.grid[target_y * TOWN_GRID_SIDE + target_x] = tile;
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_town_jimmy_prisoner_route(state: &mut PlayState) {
    seed_town_jimmy_unoccupied_target(state, JIMMY_MANACLES_TILE);
    seed_town_route_scheduled_npc(state, 1, 0x0E, 2, 1, 0);
    if let Some(npc) = state.npcs.iter_mut().find(|npc| npc.slot == 1) {
        npc.dialog_id = 2;
    }
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
    state
        .active_objects
        .resize(OOL_SLOTS, ActiveObject::empty());
    state.active_objects[1] = ActiveObject {
        type_byte: HORSE_PARKED_FIRST,
        tile: HORSE_PARKED_FIRST,
        x: 63,
        y: 124,
        z: WorldPlane::Britannia.save_floor(),
        phase: 0,
        aux1: 0,
        aux3: 0,
    };
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

fn seed_long_camp_recovery_route(state: &mut PlayState) {
    state.party = vec![
        route_party_member(0, b'A', b'G', 1, 2),
        route_party_member(1, b'M', b'G', 4, 10),
        route_party_member(2, b'B', b'G', 5, 6),
        route_party_member(3, b'F', b'G', 5, 20),
        route_party_member(4, b'A', b'P', 20, 20),
        route_party_member(5, b'M', b'D', 0, 20),
    ];
    for (member, mana) in state.party.iter_mut().zip([0, 1, 2, 3, 4, 5]) {
        member.mana = mana;
    }
    state.avatar_stats.intelligence = 22;
    state.party_names = default_party_names(6);
    state.party_experience = default_party_experience(6);
    state.party_stay_counters = default_party_stay_counters(6);
    state.party_strengths = default_party_strengths(6);
    state.party_intelligence = vec![22, 24, 20, 18, 12, 8];
    state.party_equipment = default_party_equipment(6);
    state.party_roster = default_party_roster(6);
    state.prng_state = long_camp_no_ambush_seed();
}

/// The fixed stream every route-smoke case starts from.
///
/// Reuses the ambush-free camp seed, which is the strictest requirement any
/// route places on the stream: eighteen consecutive `0..63` draws that never
/// roll zero covers both the wilderness and the dungeon rest routes.
/// Did the case's last turn come from the `Pass` key?
///
/// `commands.md §8.1` gives the pass an echo and no result line, so the
/// evidence is the transcript entry the dispatcher opened, not the message
/// slot the handler leaves empty.
fn route_state_echoed_a_pass(state: &PlayState) -> bool {
    state
        .message_entries()
        .iter()
        .any(|entry| entry.is_command_echo && entry.text == "Pass")
}

fn route_smoke_prng_seed() -> u16 {
    long_camp_no_ambush_seed()
}

fn long_camp_no_ambush_seed() -> u16 {
    for candidate in 0..=u16::MAX {
        let mut state = candidate;
        let mut safe = true;
        for _ in 0..18 {
            if u5_prng_range_u16(&mut state, 0, 63) == 0 {
                safe = false;
                break;
            }
        }
        if safe {
            return candidate;
        }
    }
    unreachable!("PRNG range cycle must contain an uninterrupted six-hour camp seed")
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

fn directed_wind_route_config(
    case_name: &str,
) -> Option<(usize, u8, usize, Option<usize>, bool, Direction)> {
    let direction = if case_name.ends_with("-north") {
        Direction::North
    } else if case_name.ends_with("-south") {
        Direction::South
    } else if case_name.ends_with("-west") {
        Direction::West
    } else {
        Direction::East
    };

    let base_name = case_name
        .strip_suffix("-north")
        .or_else(|| case_name.strip_suffix("-east"))
        .or_else(|| case_name.strip_suffix("-south"))
        .or_else(|| case_name.strip_suffix("-west"))
        .unwrap_or(case_name);

    match base_name {
        "combat-directed-sleep-cone" => {
            Some((SLEEP_SPELL_INDEX, SLEEP_COST, 2, Some(1), false, direction))
        }
        "combat-directed-poison-wind-cone" => Some((
            POISON_WIND_SPELL_INDEX,
            POISON_WIND_COST,
            3,
            Some(2),
            false,
            direction,
        )),
        "combat-directed-death-wind-cone" => Some((
            DEATH_WIND_SPELL_INDEX,
            DEATH_WIND_COST,
            2,
            Some(1),
            true,
            direction,
        )),
        "combat-directed-flame-wind-cone" => Some((
            FLAME_WIND_SPELL_INDEX,
            FLAME_WIND_COST,
            1,
            None,
            true,
            direction,
        )),
        _ => None,
    }
}

fn directed_route_coordinate_from_caster(direction: Direction, distance: i16) -> (u8, u8) {
    let (dx, dy) = direction.delta();
    (
        (5 + dx as i16 * distance) as u8,
        (5 + dy as i16 * distance) as u8,
    )
}

fn directed_route_reserve_coordinate(direction: Direction) -> (u8, u8) {
    match direction {
        Direction::North => (5, 7),
        Direction::South => (5, 3),
        Direction::East => (3, 5),
        Direction::West => (7, 5),
        _ => (3, 5),
    }
}

fn seed_directed_wind_combat_route(
    state: &mut PlayState,
    spell_index: usize,
    cost: u8,
    party_count: usize,
    target_party_slot: Option<usize>,
    include_monster_target: bool,
    direction: Direction,
) -> io::Result<()> {
    state.party = (0..party_count)
        .map(|slot| route_party_member(slot as u8, b'A', b'G', 12, 20))
        .collect();
    state.party_names = default_party_names(party_count);
    state.party_experience = default_party_experience(party_count);
    state.party_stay_counters = default_party_stay_counters(party_count);
    state.party_strengths = vec![30; party_count];
    state.party_intelligence = default_party_intelligence(party_count);
    if let Some(caster_rating) = state.party_intelligence.first_mut() {
        // Shared-resistance wind routes deterministically exercise their
        // accepted branch; runtime unit tests cover resisted boundaries.
        *caster_rating = u8::MAX;
    }
    for target_rating in state.party_intelligence.iter_mut().skip(1) {
        *target_rating = 0;
    }
    state.party_equipment = default_party_equipment(party_count);
    if let Some(caster) = state.party.first_mut() {
        caster.mana = cost;
        caster.level = cost;
        // The surviving reserve monster may receive its automatic action
        // after a lethal wind cast. Keep the route's caster alive so this
        // fixture observes the spell result in combat instead of sometimes
        // falling through to the defeat/rescue path.
        caster.hp = 99;
        caster.max_hp = 99;
    }
    state.active_player = Some(0);
    state.spell_charges[spell_index] = 1;

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([99, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    if let Some(target_slot) = target_party_slot {
        let (target_x, target_y) = directed_route_coordinate_from_caster(direction, 1);
        actors[target_slot] = CombatActorDescriptor::from_row([
            12,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            target_slot as u8,
            target_slot as u8,
            0,
            target_x,
            target_y,
        ]);
    }

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    for slot in 0..party_count {
        let (x, y) = if Some(slot) == target_party_slot {
            directed_route_coordinate_from_caster(direction, 1)
        } else {
            (5, 5)
        };
        active_objects[slot] = route_combat_active_object(0x4c, usize::from(x), usize::from(y), 0);
    }

    if include_monster_target {
        let stats = combat_class_stats(COMBAT_CLASS_GIANT_RAT)
            .ok_or_else(|| io::Error::other("giant rat combat stats are unavailable"))?;
        let monster_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let monster_distance = if target_party_slot.is_some() { 2 } else { 1 };
        let (monster_x, monster_y) =
            directed_route_coordinate_from_caster(direction, monster_distance);
        // `combat.md §6.1`: "Monster and object descriptors never carry"
        // the party-side bit `0x80`; placement stamps the hostile tag.
        actors[monster_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            monster_slot as u8,
            monster_x,
            monster_y,
            combat_monster_placement_flags(COMBAT_CLASS_GIANT_RAT),
            0,
        );
        active_objects[monster_slot] = summoned_active_object_record(
            COMBAT_CLASS_GIANT_RAT,
            monster_x as usize,
            monster_y as usize,
            0,
        )
        .ok_or_else(|| io::Error::other("giant rat active object is unavailable"))?;

        let reserve_slot = monster_slot + 1;
        let (reserve_x, reserve_y) = directed_route_reserve_coordinate(direction);
        actors[reserve_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            reserve_slot as u8,
            reserve_x,
            reserve_y,
            combat_monster_placement_flags(COMBAT_CLASS_GIANT_RAT),
            0,
        );
        active_objects[reserve_slot] = summoned_active_object_record(
            COMBAT_CLASS_GIANT_RAT,
            reserve_x as usize,
            reserve_y as usize,
            0,
        )
        .ok_or_else(|| io::Error::other("reserve giant rat active object is unavailable"))?;
    }

    // `combat.md §6.3`: ordinary death markers are rejected on terrain
    // bytes below four. Seed accepted arena terrain so the directed-wind
    // routes exercise their intended death/status branch rather than the
    // negative release form.
    state.enter_combat_frame_with_terrain(
        active_objects,
        actors,
        [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    )?;
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

fn seed_combat_field_dispel_route(
    state: &mut PlayState,
    field: Option<CombatArenaFieldKind>,
) -> io::Result<()> {
    state.party = vec![route_party_member(0, b'A', b'G', 20, 20)];
    state.party_names = default_party_names(1);
    state.party_experience = default_party_experience(1);
    state.party_stay_counters = default_party_stay_counters(1);
    state.party_strengths = vec![30];
    state.party_intelligence = default_party_intelligence(1);
    state.party_equipment = default_party_equipment(1);
    if let Some(caster) = state.party.first_mut() {
        caster.mana = DISPEL_FIELD_COST;
        caster.level = DISPEL_FIELD_COST;
    }
    state.active_player = Some(0);
    state.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 1;

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    active_objects[0] = route_combat_active_object(0x4c, 5, 5, 0);
    if let Some(field) = field {
        active_objects[6] = route_combat_active_object(field.kind_byte(), 6, 5, 0);
    }
    state.enter_combat_frame(active_objects, actors)?;
    Ok(())
}

fn combat_utility_route_spell(case_name: &str) -> (usize, u8, u8, u8, &'static str) {
    match case_name {
        "combat-utility-vanish-tile" => (
            VANISH_SPELL_INDEX,
            VANISH_COST,
            0x90,
            VANISH_CLEARED_TILE,
            "POOF!",
        ),
        "combat-utility-magic-lock-tile" => (
            MAGIC_LOCK_SPELL_INDEX,
            MAGIC_LOCK_COST,
            0xB8,
            0x97,
            "Success!",
        ),
        "combat-utility-unlock-magic-tile" => (
            UNLOCK_MAGIC_SPELL_INDEX,
            UNLOCK_MAGIC_COST,
            0x97,
            0xB8,
            "Success!",
        ),
        _ => (OPEN_SPELL_INDEX, OPEN_SPELL_COST, 0xB9, 0xB8, "Success!"),
    }
}

fn seed_combat_utility_tile_route(
    state: &mut PlayState,
    spell_index: usize,
    cost: u8,
    source_tile: u8,
) -> io::Result<()> {
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
    active_objects[1] = route_combat_active_object(0x50, 6, 5, 0);
    state.enter_combat_frame(active_objects, actors)?;
    state.combat_terrain[5][6] = source_tile;
    Ok(())
}

fn combat_spell_route_code(case_name: &str) -> &'static str {
    match case_name {
        "combat-fireball-target" => "FV",
        "combat-reveal-hidden-target" => "QW",
        "combat-invisibility-caster" => "LS",
        "combat-cause-fear-target" => "CIQ",
        "combat-mass-charm-effect" => "AQW",
        "combat-tremor-targets" => "IPVY",
        "combat-repel-undead-targets" => "ACX",
        "combat-charm-target" => "AEX",
        "combat-polymorph-target" => "BRX",
        "combat-clone-target" => "IQX",
        "combat-conjure-animal" => "KX",
        "combat-swarm-summon" => "BIX",
        "combat-summon-daemon-ring" => "CKX",
        "combat-kill-gazer-eye-burst"
        | "combat-kill-gargoyle-lava-marker"
        | "combat-kill-shadowlord-protected-rejection" => "CX",
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
    if matches!(code, "ACX" | "AEX" | "CIQ" | "CKX") {
        // Shared-resistance route cases exercise the accepted branch
        // deterministically; the formula itself is pinned in runtime tests.
        state.party_intelligence[0] = u8::MAX;
    }
    state.party_equipment = default_party_equipment(1);
    if let Some(caster) = state.party.first_mut() {
        caster.mana = cost;
        caster.level = cost;
    }
    state.active_player = Some(0);
    state.spell_charges[spell_index] = 1;
    let mut combat_terrain = if code == "ACX" {
        // Grass keeps the 1-HP post-Repel checkpoint free of the combat
        // swamp-contact damage associated with tile 0x04.
        [[0x05; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE]
    } else if code == "IQX" {
        [[0x0c; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE]
    } else if matches!(code, "KX" | "BIX" | "CKX") {
        [[0x05; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE]
    } else {
        [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE]
    };
    match code {
        "IQX" => {
            combat_terrain[1][8] = 0x04;
            combat_terrain[5][5] = 0x04;
            combat_terrain[5][6] = 0x04;
        }
        _ => {}
    }
    state.prng_state = match code {
        "GP" => first_nonzero_prng_roll_seed(15),
        "FV" => first_nonzero_prng_roll_seed(29),
        "IPVY" => first_nonzero_prng_roll_seed(19),
        "ACX" => 0x1234,
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
            // Keep the post-cast route checkpoint ahead of later flee/attack
            // turns so it validates Repel's immediate state transition.
            for actor in &mut actors[COMBAT_PARTY_ACTOR_SLOTS..=COMBAT_PARTY_ACTOR_SLOTS + 2] {
                actor.base_step = 1;
            }
        }
        "KX" | "BIX" | "CKX" => {}
        _ => {
            let class = if matches!(code, "BRX" | "FV" | "IPVY") {
                39
            } else {
                COMBAT_CLASS_GIANT_RAT
            };
            seed_combat_route_monster(&mut actors, &mut active_objects, class, 6, 6, 5)?;
            if code == "IPVY" {
                // Tremor's skewed roll is always at least one, so weight one
                // deterministically exercises both target applications.
                actors[6].base_step = 1;
            }
            if code == "FV" {
                // Same reason as the Repel case above ("Keep the post-cast
                // route checkpoint ahead of later flee/attack turns"): this
                // route exists to reach the targeted Fireball's own state,
                // and with the `combat.md §6.1` placement tag now stamped
                // correctly the class-39 monster is a full-weight hostile
                // whose post-cast turns out-damage the single route party
                // member before the checkpoint is read. Combat weight one
                // keeps the checkpoint reachable. Route observation, not a
                // published rule.
                actors[6].base_step = 1;
            }
            if code == "QW" {
                actors[6].flags |= COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
            }
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
    // `combat.md §6.1`: "Monster and object descriptors never carry"
    // the party-side bit `0x80`; placement stamps the hostile tag.
    actors[slot] = CombatActorDescriptor::for_monster_placement(
        stats,
        active_object_slot,
        x,
        y,
        combat_monster_placement_flags(class),
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

fn seed_combat_special_death_route(state: &mut PlayState, class: u8) -> io::Result<()> {
    let spell_index = spell_index_from_code("CX")
        .ok_or_else(|| io::Error::other("Kill spell code is unavailable"))?;
    let cost = spell_mp_cost(spell_index)
        .ok_or_else(|| io::Error::other("Kill spell cost is unavailable"))?;

    state.party = vec![route_party_member(0, b'A', b'G', 99, 99)];
    state.party_names = default_party_names(1);
    state.party_experience = default_party_experience(1);
    state.party_stay_counters = default_party_stay_counters(1);
    state.party_strengths = vec![30];
    state.party_intelligence = default_party_intelligence(1);
    state.party_intelligence[0] = u8::MAX;
    state.party_equipment = default_party_equipment(1);
    if let Some(caster) = state.party.first_mut() {
        caster.mana = cost;
        caster.level = cost;
    }
    state.active_player = Some(0);
    state.spell_charges[spell_index] = 1;

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([99, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    active_objects[0] = route_combat_active_object(0x4c, 5, 5, 0);
    seed_combat_route_monster(
        &mut actors,
        &mut active_objects,
        class,
        COMBAT_PARTY_ACTOR_SLOTS,
        6,
        5,
    )?;
    seed_combat_route_monster(
        &mut actors,
        &mut active_objects,
        COMBAT_CLASS_GIANT_RAT,
        COMBAT_PARTY_ACTOR_SLOTS + 1,
        8,
        5,
    )?;

    state.enter_combat_frame_with_terrain(
        active_objects,
        actors,
        [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    )?;
    Ok(())
}

fn seed_combat_entry_party(state: &mut PlayState) {
    state.party = vec![
        route_party_member(0, b'A', b'G', 30, 30),
        route_party_member(1, b'F', b'G', 30, 30),
    ];
    state.party_names = default_party_names(2);
    state.party_experience = default_party_experience(2);
    state.party_stay_counters = default_party_stay_counters(2);
    state.party_strengths = default_party_strengths(2);
    state.party_intelligence = default_party_intelligence(2);
    state.party_equipment = default_party_equipment(2);
    state.party_roster = default_party_roster(2);
}

fn seed_terrain_combat_party_entry_route(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    seed_combat_entry_party(state);
    let trigger = ActiveObject {
        type_byte: 0x50,
        tile: 0xc0,
        x: state.player.x,
        y: state.player.y,
        z: WorldPlane::Britannia.save_floor(),
        phase: STEADY_PHASE,
        aux1: 0,
        aux3: 0,
    };
    state.enter_terrain_combat_from_world_object(game_dir, WorldPlane::Britannia, 1, trigger)?;
    Ok(())
}

fn seed_dungeon_room_party_entry_route(state: &mut PlayState, game_dir: &Path) -> io::Result<()> {
    seed_combat_entry_party(state);
    state.enter_dungeon_room_combat(
        game_dir,
        DungeonScene::new(0x28).expect("Doom dungeon scene is valid"),
        7,
        15,
        111,
        dungeon_room_entry_seed_for_direction(Direction::South),
        true,
        false,
    )?;
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
    if case_name == "combat-kill-shadowlord-protected-rejection" {
        let target = state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS];
        if !state.combat_active
            || state.spell_charges[spell_index] != 0
            || state.party.first().is_none_or(|member| member.mana != 0)
            || state.turn < 1
            || target.owner_target_class != COMBAT_CLASS_SHADOW_LORD
            || target.is_marked_dead()
            || state.active_cast_followup.is_some()
            || !state.message.contains("Failed!")
        {
            return Err(io::Error::other(format!(
                "route smoke `{case_name}` did not commit the protected Shadow Lord rejection after spending cast resources"
            )));
        }
        return Ok(());
    }
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
        "combat-fireball-target" => {
            if !state.message.starts_with("Fireball!")
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].hp_or_wound
                    >= combat_class_stats(39)
                        .map(|stats| stats.max_hp)
                        .unwrap_or(u8::MAX)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not damage the targeted actor with Fireball; message `{}`",
                    state.message
                )));
            }
        }
        "combat-reveal-hidden-target" => {
            if !state.message.starts_with("Revealed 1 combat actor")
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_hidden_or_unrevealed()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not reveal the hidden combat actor; message `{}`",
                    state.message
                )));
            }
        }
        "combat-invisibility-caster" => {
            if !state.message.starts_with("Invisibility!")
                || !state.combat_actors[0].is_hidden_or_unrevealed()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not hide the active caster; message `{}`",
                    state.message
                )));
            }
        }
        "combat-cause-fear-target" => {
            if !state
                .message
                .starts_with("Cause Fear affected 1 combat actor")
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].flags & COMBAT_ACTOR_FLAG_FLEEING
                    == 0
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not mark the hostile actor as fleeing; message `{}`",
                    state.message
                )));
            }
        }
        "combat-mass-charm-effect" => {
            // How far the shared effect slot has aged when the scripted
            // route stops is a property of the route, not of any published
            // rule, so it is re-derived by running the route rather than
            // read out of the spec. It moved in this change because the
            // `combat.md §6.1` placement tags a few lines above now stamp
            // `0x40` instead of `0x80` on the spawned monsters, which
            // changes how many script steps the fight survives - not
            // because of Mass Charm's own target-picker override, which
            // §16.1 says "does not" affect side counting either way. The
            // expectation is pinned exactly rather than as a range.
            const MASS_CHARM_ROUTE_AGE_STEPS: u8 = 4;
            if !state.message.starts_with("Mass charm!")
                || state.active_effect_tag != Some(MASS_CHARM_ACTIVE_EFFECT_TAG)
                || state.active_effect_counter
                    != MASS_CHARM_ACTIVE_EFFECT_DURATION.saturating_sub(MASS_CHARM_ROUTE_AGE_STEPS)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not retain the post-action Mass Charm effect; tag {:?}, counter {}, message `{}`",
                    state.active_effect_tag, state.active_effect_counter, state.message
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
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].hp_or_wound != 1
                || !state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_fleeing()
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].hp_or_wound != 1
                || !state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1].is_fleeing()
                || state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2].is_marked_dead()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not repel only undead combat actors: message={:?}, ghost={:?}, skeleton={:?}, orc={:?}, xp={:?}",
                    state.message,
                    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS],
                    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 1],
                    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS + 2],
                    state.party_experience,
                )));
            }
        }
        "combat-charm-target" => {
            // `catalogs/spell-list.md` id 34: Charm prints `<name> charmed!`
            // and suppresses the shared epilogue.
            if !state.message.starts_with("Giant Rat charmed!") {
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
        "combat-kill-gazer-eye-burst" => {
            if !state.message.starts_with("Kill!")
                || !state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_marked_dead()
                || state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].tile
                    != COMBAT_GAZER_DEATH_MARKER_TILE
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not materialize the Gazer eye-burst death marker (message `{}`, actor {:?}, tile 0x{:02x})",
                    state.message,
                    state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS],
                    state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].tile
                )));
            }
        }
        "combat-kill-gargoyle-lava-marker" => {
            // `combat.md §6.3`: the Gargoyle branch writes the lava terrain
            // byte, writes **no** tile byte into the active-object record,
            // runs no drop rolls, and releases the slot. The earlier reading
            // that it fell through to the ordinary drop check is withdrawn.
            if !state.message.starts_with("Kill!")
                || !state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS].is_empty()
                || state.combat_terrain[5][6] != COMBAT_GARGOYLE_DEATH_TERRAIN_TILE
                || state.active_objects[COMBAT_PARTY_ACTOR_SLOTS].tile
                    == COMBAT_DEFAULT_DEATH_DROP_TILE
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the Gargoyle lava death transition"
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

/// The shipped floor `+2` cells every harpsichord route depends on.
///
/// `town-mode.md §13` describes the instrument, the chair immediately north of
/// it, and the wall five cells north in the same column. If the shipped map
/// ever stops matching that description, the rewrite assertions below would
/// quietly become vacuous - so each route checks the map's own shape first and
/// fails with the bytes it actually found.
fn validate_harpsichord_route_map(state: &PlayState, case_name: &str) -> io::Result<()> {
    if !matches!(
        state.area,
        Area::Town {
            floor: HARPSICHORD_FLOOR,
            ..
        }
    ) {
        return Err(io::Error::other(format!(
            "route smoke `{case_name}` left Lord British's Castle floor {HARPSICHORD_FLOOR}: {}",
            state.current_area_label()
        )));
    }
    let instrument = state.grid[HARPSICHORD_ROUTE_Y * TOWN_GRID_SIDE + HARPSICHORD_ROUTE_X];
    let chair = state.grid[HARPSICHORD_ROUTE_CHAIR_Y * TOWN_GRID_SIDE + HARPSICHORD_ROUTE_X];
    // `catalogs/tile-catalog.md`: the four-facing chair family is `0x90..0x93`.
    if instrument != HARPSICHORD_TILE || !(0x90..=0x93).contains(&chair) {
        return Err(io::Error::other(format!(
            "route smoke `{case_name}`: shipped floor {HARPSICHORD_FLOOR} no longer matches town-mode.md §13;              ({HARPSICHORD_ROUTE_X}, {HARPSICHORD_ROUTE_Y}) is {instrument:#04x} (expected the harpsichord {HARPSICHORD_TILE:#04x})              and the cell north of it is {chair:#04x} (expected a chair 0x90..0x93)"
        )));
    }
    Ok(())
}

/// The live byte of the cell a finished tune opens.
fn harpsichord_route_passage_tile(state: &PlayState) -> u8 {
    state.grid[HARPSICHORD_ROUTE_PASSAGE_Y * TOWN_GRID_SIDE + HARPSICHORD_ROUTE_X]
}

/// `commands.md §3`: the town digit handler at the instrument is the only
/// producer of status `3` anywhere in the game, so no keystroke on any of
/// these routes may advance the turn counter or the clock.
fn validate_harpsichord_route_consumed_no_time(
    state: &PlayState,
    case_name: &str,
) -> io::Result<()> {
    let start_clock = PlayOptions::default().clock;
    if state.turn != 0
        || state.clock != start_clock
        || state.player.x != HARPSICHORD_ROUTE_X
        || state.player.y != HARPSICHORD_ROUTE_CHAIR_Y
    {
        return Err(io::Error::other(format!(
            "route smoke `{case_name}` advanced the world while playing the instrument: turn={}, clock={:?} (expected {start_clock:?}), party at ({}, {})",
            state.turn, state.clock, state.player.x, state.player.y
        )));
    }
    Ok(())
}

/// How many draws separate two states of the shared generator, walking
/// forward from `from` and giving up after `max_draws`.
///
/// `npc-schedules.md §9.1` "Random-stream consumption": "A wander-eligible NPC
/// consumes one draw from the shared generator on a turn where the gate fails
/// and two on a turn where it passes, so the number of draws a turn consumes
/// depends on how many NPCs were eligible." A town route therefore cannot pin
/// the post-turn stream position to a single recomputed value the way an
/// overworld route can - but it can still pin it to a *recomputed* position:
/// the state the published head draws leave behind, advanced by a draw count
/// inside the bound that same sentence gives.
fn prng_draws_between(from: u16, to: u16, max_draws: usize) -> Option<usize> {
    let mut probe = from;
    for draws in 0..=max_draws {
        if probe == to {
            return Some(draws);
        }
        probe = u5_prng_advance_state(probe);
    }
    None
}

/// The published bound on the draws one town turn's schedule pass may add
/// after the turn's own rolls: at most two per NPC in the roster
/// (`npc-schedules.md §9.1`, "one draw ... on a turn where the gate fails and
/// two on a turn where it passes"), and the schedule processor gives "every
/// NPC ... one chance to act per tick" (§5).
fn town_schedule_pass_draw_bound(state: &PlayState) -> usize {
    2 * state.npcs.len()
}

fn validate_route_smoke_case_state(
    state: &PlayState,
    case_name: &str,
    game_dir: &Path,
) -> io::Result<()> {
    if let Some(index) = route_smoke_public_location_index(case_name) {
        let Some(entry) = published_world_location_entries().into_iter().nth(index) else {
            return Err(io::Error::other(format!(
                "route smoke `{case_name}` does not map to a published location row"
            )));
        };
        match entry.target {
            PlayTarget::Town(_) => {
                if state.return_world.is_some() {
                    return Err(io::Error::other(format!(
                        "route smoke `{case_name}` retained a forbidden town return snapshot"
                    )));
                }
                let file_name = match entry.plane {
                    WorldPlane::Britannia => BRIT_OOL_FILENAME,
                    WorldPlane::Underworld => UNDER_OOL_FILENAME,
                };
                let table = fs::read(game_dir.join(file_name))?;
                if table.get(2).copied() != Some(entry.x as u8)
                    || table.get(3).copied() != Some(entry.y as u8)
                    || table.get(4).copied() != Some(entry.plane.save_floor() as u8)
                {
                    return Err(io::Error::other(format!(
                        "route smoke `{case_name}` did not persist slot zero to the canonical {} mirror",
                        entry.plane.key()
                    )));
                }
            }
            PlayTarget::Dungeon(scene) => {
                let Some(return_world) = state.return_world.as_ref() else {
                    return Err(io::Error::other(format!(
                        "route smoke `{case_name}` did not cache its dungeon return checkpoint"
                    )));
                };
                if return_world.plane != entry.plane
                    || return_world.x != entry.x
                    || return_world.y != entry.y
                {
                    return Err(io::Error::other(format!(
                        "route smoke `{case_name}` saved return ({}, {}, {}) instead of ({}, {}, {})",
                        return_world.plane.key(),
                        return_world.x,
                        return_world.y,
                        entry.plane.key(),
                        entry.x,
                        entry.y
                    )));
                }
                let expected_level = if entry.plane == WorldPlane::Underworld && scene.record != 7 {
                    7
                } else {
                    0
                };
                match state.area {
                    Area::Dungeon { level, .. } if level == expected_level => {}
                    _ => {
                        return Err(io::Error::other(format!(
                            "route smoke `{case_name}` did not enter the expected dungeon level {expected_level}"
                        )));
                    }
                }
            }
            PlayTarget::World(_) => unreachable!("published table excludes world targets"),
        }
        return Ok(());
    }

    match case_name {
        "castle-canonical-ool-exit" => {
            if state.area
                != (Area::World {
                    plane: WorldPlane::Britannia,
                })
                || (state.player.x, state.player.y) != (86, 107)
                || state.return_world.is_some()
                || !state.message.contains("Exit to")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the fixed-coordinate canonical-OOL town exit"
                )));
            }
        }
        "britannia-defeat-persists-ool-before-rescue" => {
            let rescue_scene = Scene::new(BLACKTHORN_RESCUE_HANDOFF_SCENE)?;
            let bytes = fs::read(game_dir.join(BRIT_OOL_FILENAME))?;
            let offset = 31 * OOL_RECORD_LEN;
            let expected = [0x71, 0x72, 73, 74, 0, 0x75, 0x76, 0x77];
            if state.area
                != (Area::Town {
                    scene: rescue_scene,
                    floor: 0,
                })
                || state.turn != 0
                || state.party.is_empty()
                || state
                    .party
                    .iter()
                    .any(|member| member.status != b'G' || member.hp != member.max_hp.max(1))
                || state.player.transport != TransportState::Foot
                || bytes.get(offset..offset + OOL_RECORD_LEN) != Some(expected.as_slice())
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not persist the complete live Britannia OOL table before the shared rescue"
                )));
            }
        }
        "stonegate-trapdoor-rescue" => {
            if state.turn != 1
                || state.party.is_empty()
                || state
                    .party
                    .iter()
                    .any(|member| member.status != b'G' || member.hp != member.max_hp.max(1))
                || state.player.transport != TransportState::Foot
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete one trapdoor turn followed by the shared rescue"
                )));
            }
        }
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
            // `cleak/u5-spec#82` published the certificate wording, so
            // the victory ending now runs to its end: rite beats,
            // tableau exit, the §7.1 fade, all six `END.DAT` windows,
            // the certificate and the elapsed-time report, finishing in
            // the terminal hold §9.5 describes.
            if endgame.outcome != Some(EndgameOutcome::Victory)
                || !endgame.cinematic_is_finished()
                || endgame.certificate.is_none()
                || !party_slots_cleared
                || !cinematic_slots_cleared
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not run the victory ending to its terminal hold and clear tableau actors (step={:?}, certificate={}, party_slots_cleared={}, cinematic_slots_cleared={})",
                    endgame.cinematic.step,
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
                || state.message != "Locate:\nH'M\", D'O\"\n"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the public Locate sextant output"
                )));
            }
        }
        "britannia-rel-hur-east" => {
            if state.wind != WindState::East
                || state.wind_save_byte != WindState::East.save_byte()
                || state.spell_charges[REL_HUR_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state.message != "Wind change! Calm Winds -> East Winds."
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the public Rel Hur wind mapping"
                )));
            }
        }
        "castle-light-open-spell-route" => {
            let target = state.player.y * TOWN_GRID_SIDE + state.player.x + 1;
            if state.spell_charges[IN_LOR_SPELL_INDEX] != 0
                || state.spell_charges[VAS_LOR_SPELL_INDEX] != 0
                || state.spell_charges[OPEN_SPELL_INDEX] != 0
                || state.light_spell_counter == 0
                || state.grid.get(target).copied() != Some(0xb8)
                || state.message != "Success!"
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
        case_name if directed_wind_route_config(case_name).is_some() => {
            let (spell_index, _, _, target_party_slot, include_monster_target, _) =
                directed_wind_route_config(case_name).expect("directed wind route config exists");
            if !state.combat_active
                || state.spell_charges[spell_index] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not spend directed wind resources"
                )));
            }

            match spell_index {
                SLEEP_SPELL_INDEX => {
                    let target_slot = target_party_slot
                        .ok_or_else(|| io::Error::other("Sleep route missing target slot"))?;
                    if state
                        .party
                        .get(target_slot)
                        .is_none_or(|member| member.status != b'S')
                        || state.message != "Sleep!"
                    {
                        return Err(io::Error::other(format!(
                            "route smoke `{case_name}` did not apply the directed Sleep cone"
                        )));
                    }
                }
                POISON_WIND_SPELL_INDEX => {
                    let target_slot = target_party_slot
                        .ok_or_else(|| io::Error::other("Poison Wind route missing target slot"))?;
                    if state
                        .party
                        .get(target_slot)
                        .is_none_or(|member| member.status != b'P')
                        || state.message != "Poison wind!"
                    {
                        return Err(io::Error::other(format!(
                            "route smoke `{case_name}` did not apply the directed Poison Wind cone"
                        )));
                    }
                }
                DEATH_WIND_SPELL_INDEX => {
                    let stats = combat_class_stats(COMBAT_CLASS_GIANT_RAT).ok_or_else(|| {
                        io::Error::other("giant rat combat stats are unavailable")
                    })?;
                    let target_slot = target_party_slot
                        .ok_or_else(|| io::Error::other("Death Wind route missing target slot"))?;
                    if state
                        .party
                        .get(target_slot)
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
                FLAME_WIND_SPELL_INDEX => {
                    if !include_monster_target || !state.message.starts_with("Flame wind!") {
                        return Err(io::Error::other(format!(
                            "route smoke `{case_name}` did not apply the directed Flame Wind cone"
                        )));
                    }
                }
                _ => unreachable!("directed route config only yields directed wind spell ids"),
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
        "combat-field-dispel-fire-marker" => {
            if !state.combat_active
                || state.spell_charges[DISPEL_FIELD_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state.find_combat_arena_field_marker(6, 5).is_some()
                || state.message != "Dispelled Fire field."
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not remove the public combat field marker"
                )));
            }
        }
        "combat-field-dispel-empty-refusal" => {
            if !state.combat_active
                || state.spell_charges[DISPEL_FIELD_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state.message != "Failed!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not spend resources and fail on a missing combat field"
                )));
            }
        }
        "combat-utility-vanish-tile"
        | "combat-utility-open-tile"
        | "combat-utility-magic-lock-tile"
        | "combat-utility-unlock-magic-tile" => {
            let (spell_index, _, _, expected_tile, expected_message) =
                combat_utility_route_spell(case_name);
            if !state.combat_active
                || state.spell_charges[spell_index] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || state.message != expected_message
                || state
                    .active_objects
                    .get(1)
                    .is_none_or(|object| object.tile != 0x50 || object.x != 6 || object.y != 5)
                || state.combat_terrain[5][6] != expected_tile
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the published combat utility live-tile rewrite"
                )));
            }
        }
        "combat-magic-missile-target"
        | "combat-fireball-target"
        | "combat-reveal-hidden-target"
        | "combat-invisibility-caster"
        | "combat-cause-fear-target"
        | "combat-mass-charm-effect"
        | "combat-tremor-targets"
        | "combat-repel-undead-targets"
        | "combat-charm-target"
        | "combat-polymorph-target"
        | "combat-clone-target"
        | "combat-conjure-animal"
        | "combat-swarm-summon"
        | "combat-summon-daemon-ring"
        | "combat-kill-gazer-eye-burst"
        | "combat-kill-gargoyle-lava-marker"
        | "combat-kill-shadowlord-protected-rejection" => {
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
            if state.grid.get(current).copied() != Some(0x78)
                || state.spell_charges[OPEN_SPELL_INDEX] != 0
                || state.party.first().is_none_or(|member| member.mana != 0)
                || !state.message.contains("Safely opened dungeon chest")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not open the dungeon chest by spell"
                )));
            }
        }
        "dungeon-open-chest-command" => {
            let current = dungeon_cell_index(0, state.player.x, state.player.y);
            if state.grid.get(current).copied() != Some(0x78)
                || !state.message.contains("Opened dungeon chest")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not clear dungeon chest trap/subtype bits while preserving the visit marker"
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
                    "route smoke `{case_name}` did not apply meal-hour provisions plus poison (clock {:02}:{:02}, food {}, party {:?}, message `{}`)",
                    state.clock.hour,
                    state.clock.minute,
                    state.food,
                    state
                        .party
                        .iter()
                        .map(|member| (member.hp, member.status))
                        .collect::<Vec<_>>(),
                    state.message,
                )));
            }
        }
        "castle-hourly-poison-starvation-pass" => {
            let mut expected_prng: u16 = 0x3456;
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
            // The two starvation draws come off the head of the stream,
            // because the shared status/provision pass runs ahead of the two
            // town walkers in the turn tail; the damage equalities below pin
            // both values and their order exactly. What follows them is the
            // town schedule pass, whose draw count `npc-schedules.md §9.1`
            // makes roster-dependent, so the end position is pinned as the
            // recomputed post-starvation state advanced by a draw count
            // inside that section's published per-NPC bound - not as "the
            // stream moved".
            let schedule_pass_draws = prng_draws_between(
                expected_prng,
                state.prng_state,
                town_schedule_pass_draw_bound(state),
            );
            if state.clock.hour != 9
                || state.food != 0
                || schedule_pass_draws.is_none()
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
            // As in the poison/starvation case: the ring's own draw is the
            // first off the seeded stream and the recovery equalities below
            // pin its value, and the town schedule pass that follows it draws
            // a roster-dependent number of times (`npc-schedules.md §9.1`).
            // The end position is therefore pinned as the recomputed
            // post-ring state advanced by a bounded draw count.
            let schedule_pass_draws = prng_draws_between(
                expected_prng,
                state.prng_state,
                town_schedule_pass_draw_bound(state),
            );
            if roll != 0
                || state.clock.hour != 8
                || schedule_pass_draws.is_none()
                || state.party.first().is_none_or(|member| {
                    member.status != b'G' || member.hp != member.max_hp || member.mana != 8
                })
                || state.party_equipment.first().is_none_or(|equipment| {
                    equipment[EQUIP_SLOT_RING] != EQUIPMENT_ID_RING_REGENERATION as u8
                })
                || !route_state_echoed_a_pass(state)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply hourly Ring of Regeneration"
                )));
            }
        }
        "castle-poison-gas-step" => {
            let mut expected_prng = poison_gas_first_poison_seed();
            let roll = u5_prng_range_u16(&mut expected_prng, 0, TOWN_GAS_DOORWAY_RANGE_MAX);
            // As above: the gas roll is the first draw off the seeded stream
            // and its value is pinned by `roll` and the poisoned status
            // below; the town schedule pass that follows draws a
            // roster-dependent number of times (`npc-schedules.md §9.1`), so
            // the end position is pinned as the recomputed post-gas state
            // advanced by a bounded draw count.
            let schedule_pass_draws = prng_draws_between(
                expected_prng,
                state.prng_state,
                town_schedule_pass_draw_bound(state),
            );
            if roll == 0
                || schedule_pass_draws.is_none()
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
        "castle-jimmy-magic-lock-no-picker" => {
            if state.turn != 1
                || state.keys != DEFAULT_KEY_STOCK - 1
                || state.prng_state != 0x3456
                || state.active_jimmy.is_some()
                || state.grid[TOWN_GRID_SIDE + 2] != TOWN_DOOR_MAGIC_PLAIN_TILE
                || state.message != "Key broke!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not take the promptless magic-lock key-break path"
                )));
            }
        }
        "castle-jimmy-empty-restraint-no-picker" => {
            if state.turn != 1
                || state.keys != DEFAULT_KEY_STOCK
                || state.prng_state != 0x3456
                || state.active_jimmy.is_some()
                || state.grid[TOWN_GRID_SIDE + 2] != JIMMY_STOCKS_TILE
                || state.message != "No one is there!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not reject the empty restraint before picker and PRNG"
                )));
            }
        }
        "castle-harpsichord-tune-opens-passage"
        | "castle-harpsichord-resync-after-ten-notes-and-stray-eight"
        | "castle-harpsichord-resync-after-eleven-notes-and-stray-seven" => {
            validate_harpsichord_route_map(state, case_name)?;
            validate_harpsichord_route_consumed_no_time(state, case_name)?;
            // `town-mode.md §13` requires the completion to rewrite the wall
            // cell and mark the view dirty, and `harpsichord.rs`'s unit tests
            // pin both at the moment of the rewrite. This route asserts the
            // durable half only: the route harness renders after every step,
            // and a render legitimately consumes the dirty flag, so requiring
            // it to survive to the end of a thirteen-keystroke replay would
            // pin the harness rather than the contract.
            let passage = harpsichord_route_passage_tile(state);
            if passage != HARPSICHORD_PASSAGE_CLEARED_TILE || state.harpsichord_progress() != 0 {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not open the passage five cells north of the harpsichord:                      ({HARPSICHORD_ROUTE_X}, {HARPSICHORD_ROUTE_PASSAGE_Y}) is {passage:#04x}                      (expected cobble {HARPSICHORD_PASSAGE_CLEARED_TILE:#04x}), progress={}, view dirty={}",
                    state.harpsichord_progress(),
                    state.visibility_dirty,
                )));
            }
        }
        "castle-harpsichord-digits-consume-no-turn" => {
            validate_harpsichord_route_map(state, case_name)?;
            validate_harpsichord_route_consumed_no_time(state, case_name)?;
            // Twelve of thirteen notes: the counter has advanced and the wall
            // has not, which is what makes the no-turn assertion meaningful
            // rather than a route that simply did nothing.
            let passage = harpsichord_route_passage_tile(state);
            if passage != HARPSICHORD_ROUTE_WALL_TILE
                || state.harpsichord_progress() != HARPSICHORD_TUNE_SCRIPT.len() - 1
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` expected twelve notes counted behind an intact wall:                      passage tile {passage:#04x} (expected {HARPSICHORD_ROUTE_WALL_TILE:#04x}), progress={}",
                    state.harpsichord_progress(),
                )));
            }
        }
        "castle-harpsichord-digit-is-an-ordinary-command-off-the-chair" => {
            validate_harpsichord_route_map(state, case_name)?;
            // `8` stepped north and `5` reached the ordinary dispatcher's
            // refusal, exactly as both did before the instrument existed.
            let passage = harpsichord_route_passage_tile(state);
            if state.turn != 1
                || state.player.x != HARPSICHORD_ROUTE_X
                || state.player.y != HARPSICHORD_ROUTE_OFF_CHAIR_Y - 1
                || state.harpsichord_progress() != 0
                || passage != HARPSICHORD_ROUTE_WALL_TILE
                || state.message != "What?"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not forward its digits to the ordinary dispatcher:                      turn={}, party at ({}, {}), progress={}, passage tile {passage:#04x}, message {:?}",
                    state.turn,
                    state.player.x,
                    state.player.y,
                    state.harpsichord_progress(),
                    state.message,
                )));
            }
        }
        "castle-harpsichord-passage-lost-on-floor-round-trip"
        | "reload-castle-harpsichord-passage-lost-on-reload" => {
            validate_harpsichord_route_map(state, case_name)?;
            // `town-mode.md §13`: the rewrite is a live tile-buffer edit, not
            // a saved map change, so the floor comes back with its wall.
            let passage = harpsichord_route_passage_tile(state);
            if passage != HARPSICHORD_ROUTE_WALL_TILE {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` persisted the harpsichord passage across a floor reload:                      ({HARPSICHORD_ROUTE_X}, {HARPSICHORD_ROUTE_PASSAGE_Y}) is {passage:#04x}, expected the wall {HARPSICHORD_ROUTE_WALL_TILE:#04x}"
                )));
            }
        }
        "castle-jimmy-prisoner-release" => {
            let released = state.npcs.iter().find(|npc| npc.slot == 1);
            let released_snapshot = released.map(|npc| {
                (
                    npc.dialog_id,
                    npc.schedule[NPC_SCHEDULE_AI_OFFSET
                        ..NPC_SCHEDULE_AI_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT]
                        .to_vec(),
                )
            });
            if state.turn != 1
                || state.keys != DEFAULT_KEY_STOCK
                || state.moral_standing != MORAL_STANDING_MAX
                || state.removed_town_npc_flags.get(&17).copied().unwrap_or(0) & 0b10 == 0
                || state.grid[TOWN_GRID_SIDE + 2] != JIMMY_MANACLES_TILE
                || released.is_none_or(|npc| {
                    npc.dialog_id != NPC_DIALOG_ID_NONE
                        || npc.schedule[NPC_SCHEDULE_AI_OFFSET
                            ..NPC_SCHEDULE_AI_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT]
                            .iter()
                            .any(|mode| *mode != JIMMY_RELEASE_AI_MODE)
                })
                || state.active_jimmy.is_some()
                || state.message != "I thank thee!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the live prisoner release mutation and reward: turn={}, keys={}, moral={}, mask={:#010x}, tile={:#04x}, npc={released_snapshot:?}, active_jimmy={}, message={:?}",
                    state.turn,
                    state.keys,
                    state.moral_standing,
                    state.removed_town_npc_flags.get(&17).copied().unwrap_or(0),
                    state.grid[TOWN_GRID_SIDE + 2],
                    state.active_jimmy.is_some(),
                    state.message,
                )));
            }
        }
        "reload-castle-jimmy-prisoner-release" => {
            if state.turn != 1
                || state.keys != DEFAULT_KEY_STOCK
                || state.moral_standing != MORAL_STANDING_MAX
                || state.removed_town_npc_flags.get(&17).copied().unwrap_or(0) & 0b10 == 0
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve the native prisoner removal mask and reward across save/reload"
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
        "reload-boarded-horse-pass" => {
            if !matches!(state.player.transport, TransportState::Horse { .. })
                || state.player.x != 62
                || state.player.y != 124
                || state.active_objects.first().is_none_or(|object| {
                    object.x != state.player.x
                        || object.y != state.player.y
                        || object.z != WorldPlane::Britannia.save_floor()
                })
                || !route_state_echoed_a_pass(state)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve boarded horse state across save/reload"
                )));
            }
        }
        "reload-gate-travel-underworld-pass" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Underworld
                }
            ) || state.player.x != 231
                || state.player.y != 5
                || state.active_objects.first().is_none_or(|object| {
                    object.x != state.player.x
                        || object.y != state.player.y
                        || object.z != WorldPlane::Underworld.save_floor()
                })
                || !route_state_echoed_a_pass(state)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve Gate Travel destination across save/reload"
                )));
            }
        }
        "natural-moongate-trammel-gate-travel" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Underworld
                }
            ) || state.player.x != 231
                || state.player.y != 5
                || state.turn != 0
                || !state.message.contains("Gate Travel phase 1")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not consume the cached natural moongate slot"
                )));
            }
        }
        "natural-moongate-empty-slot-clears-live-tile" => {
            let idx = 124 * WORLD_SIDE + 62;
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Britannia
                }
            ) || state.player.x != 62
                || state.player.y != 124
                || state.turn != 0
                || state.grid.get(idx).copied() != Some(NATURAL_MOONGATE_RESTORED_TERRAIN_TILE)
                || !state.natural_moongate_live_cells.is_empty()
                || !state
                    .message
                    .contains("Natural moongate phase 1 is not set")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not clear the empty natural moongate slot"
                )));
            }
        }
        "britannia-chasm-fall-to-underworld" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Underworld
                }
            ) || state.player.x != SURFACE_CHASM_X as usize
                || state.player.y != SURFACE_CHASM_Y as usize
                || state.active_objects.first().is_none_or(|object| {
                    object.x != SURFACE_CHASM_X as usize
                        || object.y != SURFACE_CHASM_Y as usize
                        || object.z != WorldPlane::Underworld.save_floor()
                })
                || !state.message.contains("underworld!!")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not land on the published chasm transition"
                )));
            }
        }
        "reload-chasm-underworld-pass" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Underworld
                }
            ) || state.player.x != SURFACE_CHASM_X as usize
                || state.player.y != SURFACE_CHASM_Y as usize
                || state.active_objects.first().is_none_or(|object| {
                    object.x != SURFACE_CHASM_X as usize
                        || object.y != SURFACE_CHASM_Y as usize
                        || object.z != WorldPlane::Underworld.save_floor()
                })
                || !route_state_echoed_a_pass(state)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve chasm Underworld landing across save/reload"
                )));
            }
        }
        "britannia-whirlpool-forced-underworld" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Underworld
                }
            ) || state.player.x != WHIRLPOOL_EMERGENCE_X as usize
                || state.player.y != WHIRLPOOL_EMERGENCE_Y as usize
                || state.active_objects.first().is_none_or(|object| {
                    object.x != WHIRLPOOL_EMERGENCE_X as usize
                        || object.y != WHIRLPOOL_EMERGENCE_Y as usize
                        || object.z != WorldPlane::Underworld.save_floor()
                })
                // `overworld.md §8.1`: the whirlpool banner is the first and
                // only text on the path, and the coordinate narration is gone.
                || !state
                    .message_entries()
                    .iter()
                    .any(|entry| entry.text == "WHIRLPOOL!")
                || state.message.contains("Sucked into the underworld")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the published whirlpool transition"
                )));
            }
        }
        "britannia-fixed-narrative-gate-unordained-refusal" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Britannia
                }
            ) || state.player.x != NARRATIVE_GATE_X as usize
                || state.player.y != NARRATIVE_GATE_Y as usize + 1
                || state.active_objects.first().is_none_or(|object| {
                    object.x != state.player.x
                        || object.y != state.player.y
                        || object.z != WorldPlane::Britannia.save_floor()
                })
                || !state
                    .message
                    .ends_with("Thou art not upon a Sacred Quest!\nPassage denied!\n")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the unordained narrative gate branch"
                )));
            }
        }
        "britannia-fixed-narrative-gate-ordained-passage" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Britannia
                }
            ) || state.player.x != NARRATIVE_GATE_X as usize
                || state.player.y != NARRATIVE_GATE_Y as usize
                || state.shrine_ordained_mask == 0
                || !state.message.ends_with("Pass, Seeker!\n")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply the ordained passage branch"
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
        "castle-town-attack-death-mask-npc" => {
            if state.removed_town_npc_flags.get(&17).copied().unwrap_or(0) & 0b10 == 0
                || state.combat_active
                || !state.message.contains("target removed")
                || !state.npcs.is_empty()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not remove the attackable town NPC through the death-mask path"
                )));
            }
        }
        "castle-town-attack-guard-alarm" => {
            // `town-mode.md §14`: the attack enters the terrain arena *and*
            // raises the alarm. This used to assert `!state.combat_active`,
            // which pinned the withdrawn "stays inside town mode" reading; the
            // alarm assertions below are the part that always belonged here.
            //
            // The retraction publishes two more discriminators, and without
            // them this case would only prove that *some* fight started.
            // `encounters.md §7`: "The arena selector then resolves to the
            // cobble arena for ordinary town ground, and the terrain setup's
            // town-style override forces the monster count to one unless the
            // target's class is Guard (whose stat row carries the sentinel
            // count eight)." §5's arena table gives cobble as arena 8, and
            // this route's NPC is `town-mode.md §15`'s exact type `0x70`
            // Guard - so the requested count must be the sentinel eight
            // rather than the town-style one.
            let npc_schedule_swept = state
                .npcs
                .iter()
                .find(|npc| npc.slot == 1)
                .is_some_and(|npc| npc.schedule[..3] == [7, 7, 7]);
            let placed_monsters = state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS..]
                .iter()
                .filter(|actor| !actor.is_empty())
                .count();
            let is_cobble_arena = state
                .combat_terrain
                .iter()
                .flatten()
                .all(|tile| *tile == 0x44);
            if !state.combat_active
                || !npc_schedule_swept
                || placed_monsters != 8
                || !is_cobble_arena
                || state.message != u5_runtime::combat_banner_line()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not raise the town alarm and enter the cobble \
                     arena after attacking a Guard-class town NPC: combat_active={} \
                     npc_slot_1_schedule_swept={npc_schedule_swept} \
                     placed_monsters={placed_monsters} is_cobble_arena={is_cobble_arena} msg={:?}",
                    state.combat_active, state.message,
                )));
            }
        }
        "castle-town-hostile-adjacent-alarm" => {
            if state.combat_active
                || !state.message.contains("Hostile NPC slot 1")
                || !state.message.contains("alarm raised")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not handle the adjacent hostile NPC through alarm cleanup"
                )));
            }
        }
        "castle-town-guard-arrest-refusal" => {
            if state.pending_town_arrest.is_some()
                || state
                    .npcs
                    .iter()
                    .find(|npc| npc.slot == 2)
                    .is_none_or(|npc| npc.schedule[..3] != [7, 7, 7])
                || !state.message.contains("Refused surrender")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not refuse the guard arrest prompt into alarm cleanup"
                )));
            }
        }
        "castle-town-guard-arrest-surrender-yew" => {
            let yew_scene = Scene::new(TOWN_ARREST_JAIL_SCENE).expect("Yew scene is valid");
            if state.pending_town_arrest.is_some()
                || !matches!(state.area, Area::Town { scene, floor }
                    if scene == yew_scene && floor == TOWN_ARREST_JAIL_FLOOR as i8)
                || state.player.x != TOWN_ARREST_JAIL_X as usize
                || state.player.y != TOWN_ARREST_JAIL_Y as usize
                || !matches!(state.player.transport, TransportState::Foot)
                || state.clock.hour != 8
                || !state.message.contains("Surrendered to the guards")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not surrender into the public Yew jail wakeup path"
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
            if !state.message.contains("Strange vision") && !state.message.contains("Death vision")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the death-vision Look flow; message `{}`",
                    state.message
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
        "castle-talk-ordinary-keyword-route" => {
            if state.active_conversation.is_none()
                || state.active_shop.is_some()
                || state.message.is_empty()
                || state.message.contains("[w")
                || state.message.contains("Dialogue id")
                || state.message.contains("funny look")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not keep an ordinary asset-backed Talk session active"
                )));
            }
        }
        _ if asset_backed_conversation_route_family(case_name).is_some() => {
            let exits = asset_backed_conversation_route_exits(case_name);
            if (!exits && state.active_conversation.is_none())
                || (exits && state.active_conversation.is_some())
                || state.active_shop.is_some()
                || state.message.is_empty()
                || state.message.contains("[w")
                || state.message.contains("Dialogue id")
                || state.message.contains("funny look")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the asset-backed TLK conversation path"
                )));
            }
        }
        _ if shrine_route_virtue(case_name).is_some() => {
            let virtue = shrine_route_virtue(case_name).expect("shrine route virtue is known");
            if state.shrine_ordained_mask & virtue.bit() == 0
                || state.shrine_codex_mask != 0
                || !state.message.contains("ordained")
                || !state.message.contains(virtue.name())
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete native shrine meditation for {}",
                    virtue.name()
                )));
            }
        }
        "codex-urn-honesty-read" => {
            if state.shrine_codex_mask & ShrineVirtue::Honesty.bit() == 0
                || !state.message.contains("Read Codex page for Honesty")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not stamp the Codex-read bit"
                )));
            }
        }
        "shrine-honesty-codex-turn-in" => {
            if state.shrine_ordained_mask & ShrineVirtue::Honesty.bit() != 0
                || state.shrine_codex_mask & ShrineVirtue::Honesty.bit() == 0
                || state.moral_standing != 13
                || !state.message.contains("Completed the Shrine of Honesty")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not clear ordained state and apply the Codex turn-in"
                )));
            }
        }
        "shrine-compassion-completed-offering" => {
            if state.gold != 400
                || state.shrine_codex_mask & ShrineVirtue::Compassion.bit() == 0
                || state.moral_standing != 11
                || !state.message.contains("Offered 100 gold")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the completed-shrine offering"
                )));
            }
        }
        "castle-native-stair-up-route" => {
            if state.current_floor() != Some(1)
                || state.player.x != 16
                || state.player.y != 15
                || state.message != "Up!"
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
                || state.message != "Down!"
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
                // `text-output.md §10.2`: an accepted step is complete at its
                // own direction echo and prints no result line, so "ordinary
                // movement" here is the arrival with an empty result slot.
                || !state.message.is_empty()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not treat side-crossing a native stair as ordinary movement; floor {:?} at ({}, {}) message {:?}",
                    state.current_floor(),
                    state.player.x,
                    state.player.y,
                    state.message
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
            let word_index = WORD_OF_POWER_SEALS
                .iter()
                .position(|candidate| candidate.word == seal.word)
                .expect("route seal belongs to fixed word table");
            if state.grid.get(idx).copied() != Some(seal.unsealed_tile)
                || state.player.x != (seal.x + 1) % WORLD_SIDE
                || state.player.y != seal.y
                || state.word_of_power_seal_flags[word_index] & SAVE_QUEST_TILE_FLAG_HIGH_BIT == 0
                || !state.message.contains("The seal opens.")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not open the public Word-of-Power seal"
                )));
            }
        }
        "britannia-empty-yell-is-acted" => {
            if state.turn != 1
                || state.active_yell.is_some()
                || state.message != YELL_NOTHING_SAID_MESSAGE
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not commit the empty prompted Yell"
                )));
            }
        }
        "britannia-ruined-honesty-shrine-restoration" => {
            let (x, y) = WORLD_SHRINE_COORDINATES[0];
            if state.grid[world_cell_index(x, y)] != WORLD_SHRINE_TILE
                || state.shrine_ruin_flags[0] != 0x05
                || state.word_of_power_seal_flags[0] != 0x27
                || state.active_shrine_restoration.is_some()
                || !state.message.contains(SHRINE_RESTORATION_SUCCESS_BANNER)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the ruined-shrine restoration"
                )));
            }
        }
        "lycaeum-shard-falsehood-vanquish" => {
            validate_shadowlord_shard_route(
                state,
                case_name,
                SHADOWLORD_FALSEHOOD_INDEX,
                SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
                // `quest-graph.md §5` publishes the Shadowlord name; the
                // sentence around it is not published, so pin the name.
                "FAULINEI is vanquished!",
            )?;
        }
        "empath-shard-hatred-vanquish" => {
            validate_shadowlord_shard_route(
                state,
                case_name,
                SHADOWLORD_HATRED_INDEX,
                SPECIAL_ITEM_SHARD_HATRED_INDEX,
                "ASTAROTH is vanquished!",
            )?;
        }
        "serpents-hold-shard-cowardice-vanquish" => {
            validate_shadowlord_shard_route(
                state,
                case_name,
                SHADOWLORD_COWARDICE_INDEX,
                SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
                "NOSFENTOR is vanquished!",
            )?;
        }
        "shop-arms-local-buy-sell-route" => {
            if state.gold != 999
                || !matches!(
                    state.active_shop,
                    Some(ActiveShopSession::ArmsLocal(
                        ArmsShopState::SellPickItem(_),
                        ArmsShop::IolosBows
                    ))
                )
                || state.equipment_stock[EQUIPMENT_ID_BOW] != 1
                || !state.message.starts_with("No\n")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not exercise arms buy/sell browser declines without mutation"
                )));
            }
        }
        _ if is_arms_buy_first_route(case_name) => {
            let shop = arms_route_shop(case_name).ok_or_else(|| {
                io::Error::other(format!(
                    "route smoke `{case_name}` has no arms shop mapping"
                ))
            })?;
            let item = shop.stock_table().item_ids[0] as usize;
            if state.gold >= 9999
                || state.active_shop.is_some()
                || state.equipment_stock.get(item).copied() != Some(1)
                || !state.message.contains("Farewell")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not buy the first published arms-shop stock item and exit"
                )));
            }
        }
        _ if is_arms_terminator_refusal_route(case_name) => {
            if state.gold != 9999
                || state.active_shop.is_some()
                || state.equipment_stock.iter().any(|count| *count != 0)
                || !state.message.contains("Farewell")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not reject the arms-shop terminator letter without mutation"
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
            // `shops.md §8.4`: "The clock is then run forward in paced steps
            // until the hour byte reads **six** - the rest always ends at
            // 06:00, whatever hour it began at". The fixed eight-hour advance
            // this route used to pin is withdrawn with the rest of the
            // "pure presentation" reading of the `R` action.
            let scheduled_npcs_are_on_their_06_00_waypoints = state.npcs.iter().all(|npc| {
                let wp = waypoint_for_hour(&npc.schedule, state.clock.hour);
                (npc.x, npc.y)
                    == (
                        npc.schedule[NPC_SCHEDULE_X_OFFSET + wp] as usize,
                        npc.schedule[NPC_SCHEDULE_Y_OFFSET + wp] as usize,
                    )
            });
            if state.gold != expected_gold
                || !inn_recovery_applied
                || state.clock.hour != INN_REST_WAKE_HOUR
                || !state.message.contains("hours at the inn for")
                || !state.message.contains("recovered 20 HP and 24 MP")
                || !scheduled_npcs_are_on_their_06_00_waypoints
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
            if state.gold >= 999 || state.food == 0 || state.active_shop.is_some() {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not serve a drink round and provisions"
                )));
            }
        }
        "shop-tavern-honest-meal-lore-route"
        | "shop-tavern-wayfarer-lore-route"
        | "shop-tavern-sword-and-keg-lore-route"
        | "shop-tavern-slaughtered-lamb-lore-route"
        | "shop-tavern-humble-palate-lore-route"
        | "shop-tavern-blue-boar-lore-route"
        | "shop-tavern-cats-lair-lore-route"
        | "shop-tavern-fallen-virgin-lore-route"
        | "shop-tavern-folley-tap-lore-route" => {
            if state.gold >= 999
                || state.active_shop.is_some()
                || !state.message.contains("Malik")
                || !state.message.contains("Moonglow")
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not route the public tavern lore selector into a paid sage topic"
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
            let expected_message =
                u5_runtime::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir)
                    .ok()
                    .and_then(|renderer| renderer.render_sage_short_funds_record(None).ok())
                    .unwrap_or_else(|| TAVERN_AFFORDABILITY_REFUSAL_BARK.to_string());
            if state.gold != 49 || state.active_shop.is_some() || state.message != expected_message
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve gold and exit on sage short funds"
                )));
            }
        }
        "shop-horse-trader-decline-route" => {
            // Scoped to the delivery cell. `CASTLE:0` roster slots 15, 16
            // and 17 are the castle's own stable horses and carry the
            // shipped horse tags `0x10`/`0x11` (`catalogs/npc-roster.md
            // §4`), so a table-wide "no parked horse anywhere" test says
            // nothing about this sale - it only used to hold because every
            // NPC sprite byte was being clamped to one monster tile.
            if state.gold != 999
                || state.active_shop.is_none()
                || state.active_objects.iter().any(|object| {
                    object.type_byte == HORSE_PARKED_FIRST && (object.x, object.y) == (15, 16)
                })
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
            // The castle's own stable horses share this object byte, so the
            // sale's horse is identified by its delivery cell.
            let horse = state.active_objects.iter().find(|object| {
                object.type_byte == HORSE_PARKED_FIRST && (object.x, object.y) == (15, 16)
            });
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
        "reload-horse-trader-horse-and-rider-buy-pass" => {
            let stable = horse_trader_route_stable("shop-horse-trader-horse-and-rider-buy");
            let raw = stable_horse_price(stable);
            let speaker_intelligence = state.party_intelligence.first().copied().unwrap_or(0);
            let expected_gold = 999 - shop_intelligence_adjusted_price(raw, speaker_intelligence);
            // The post-reload Pass runs the town free-roaming object walker,
            // so the delivered horse may take one legal step. Its identity
            // and boardability, not its pre-turn sale coordinate, are the
            // durable reload contract - but it still has to be the horse
            // this sale delivered rather than one of the castle's own
            // stable horses, which carry the same object byte.
            let horse = state.active_objects.iter().find(|object| {
                object.type_byte == HORSE_PARKED_FIRST
                    && object.x.abs_diff(15) + object.y.abs_diff(16) <= 1
            });
            let boardable = horse.is_some_and(|object| {
                state
                    .boardable_vehicle_slot_at(object.x, object.y)
                    .is_some()
            });
            if state.gold != expected_gold
                || state.active_shop.is_some()
                || horse.is_none()
                || !boardable
                || !route_state_echoed_a_pass(state)
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve horse-trader delivery across save/reload: gold={}/{expected_gold}, shop={}, horse={horse:?}, boardable={boardable}, message={:?}",
                    state.gold,
                    state.active_shop.is_some(),
                    state.message
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
                PendingVehicleAcquisition::Skiff { x, y, aux3: 0 }
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
            if state.light_spell_counter != 0 || !route_state_echoed_a_pass(state) {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not age light counters through turns"
                )));
            }
        }
        "dungeon-hole-up-no-direct-recovery" => {
            let camp_messages = load_camp_result_messages(game_dir)?;
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
                || state.message != camp_messages.success
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve no-direct-recovery rest behavior:                      hour={} p0={:?}/{}/{} p1={:?}/{}/{} p2={:?}/{} msg={:?}",
                    state.clock.hour,
                    state.party.first().map(|m| m.status as char),
                    state.party.first().map(|m| m.hp).unwrap_or(0),
                    state.party.first().map(|m| m.mana).unwrap_or(0),
                    state.party.get(1).map(|m| m.status as char),
                    state.party.get(1).map(|m| m.hp).unwrap_or(0),
                    state.party.get(1).map(|m| m.mana).unwrap_or(0),
                    state.party.get(2).map(|m| m.status as char),
                    state.party.get(2).map(|m| m.hp).unwrap_or(0),
                    state.message,
                )));
            }
        }
        "dungeon-long-camp-recovery" => {
            let camp_messages = load_camp_result_messages(game_dir)?;
            if state.clock.hour != 14
                || state.active_rest.is_some()
                || state.party.get(0).is_none_or(|member| {
                    member.status != b'G' || member.hp != 2 || member.mana != 22
                })
                || state.party.get(1).is_none_or(|member| {
                    member.status != b'G' || !(5..=10).contains(&member.hp) || member.mana != 24
                })
                || state.party.get(2).is_none_or(|member| {
                    member.status != b'G' || member.hp != 6 || member.mana != 10
                })
                || state.party.get(3).is_none_or(|member| {
                    member.status != b'G' || member.hp != 5 || member.mana != 3
                })
                // `rest-and-camp.md §5`: a member "already poisoned at entry"
                // fails the recovery guard, so slot 4 gets neither the `1..63`
                // hit points nor the class-keyed magic-point write, and
                // "Poisoned members keep Poisoned status; rest does not cure
                // poison." Its hit points are also *unchanged*: the camp
                // elapse loop "never enters the shared party status/provision
                // pass, so while a camp is elapsing no poison damage is taken,
                // no provisions are spent, and no starvation damage is
                // applied, regardless of how many hours the camp covers. Only
                // the town-bed loop runs that pass." An earlier form of this
                // assertion subtracted one point per camped hour, which is the
                // per-hour poison reading `time.md §5` withdraws.
                || state
                    .party
                    .get(4)
                    .is_none_or(|member| member.status != b'P' || member.hp != 20 || member.mana != 4)
                || state.party.get(5).is_none_or(|member| {
                    member.status != b'D' || member.hp != 0 || member.mana != 5
                })
                || state.message != camp_messages.success
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not apply public #47 completed long-camp                      recovery: hour={} active_rest={} msg_matches={} party={:?} msg={:?}",
                    state.clock.hour,
                    state.active_rest.is_some(),
                    state.message == camp_messages.success,
                    state
                        .party
                        .iter()
                        .map(|m| (m.status as char, m.hp, m.mana))
                        .collect::<Vec<_>>(),
                    state.message,
                )));
            }
        }
        // `rest-and-camp.md §5`: with the cooldown counter armed, the
        // same camp that recovers above must recover nothing. The
        // discriminators are the three magic-point rows the recovery walk
        // assigns (22 / 24 / 10) and the Avatar's hit points, which the
        // walk caps at a maximum of two.
        "dungeon-camp-inside-cooldown-window" => {
            let camp_messages = load_camp_result_messages(game_dir)?;
            if state.clock.hour != 14
                || state.active_rest.is_some()
                || state.party.first().is_none_or(|member| {
                    member.status != b'G' || member.hp != 1 || member.mana != 0
                })
                || state.party.get(1).is_none_or(|member| {
                    member.status != b'G' || member.hp != 4 || member.mana != 1
                })
                || state.party.get(2).is_none_or(|member| {
                    member.status != b'G' || member.hp != 5 || member.mana != 2
                })
                || state.party.get(3).is_none_or(|member| {
                    member.status != b'G' || member.hp != 5 || member.mana != 3
                })
                || state.message != camp_messages.no_effect
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` recovered inside the published camp cooldown window"
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
        "dungeon-ladder-down-up-route" | "reload-dungeon-ladder-down-up-route" => {
            // `dungeon-mode.md` 8.1: an accepted climb narrates only the
            // direction word - "Applying a climb prints `Up!` or `Down!`
            // **first**, before any test" - so the level is read from the
            // state, not from a message the original never prints.
            if state.current_floor() != Some(0) || state.message != u5_runtime::DUNGEON_KLIMB_UP {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not complete the down/up ladder chain"
                )));
            }
        }
        "dungeon-heavy-door-variant-pass-through" => {
            // The step itself prints nothing (`text-output.md §10.2`), so the
            // pass-through is read from where the party now stands: on the
            // seeded `0xE?` variant cell, one turn later.
            let underfoot = state
                .grid
                .get(dungeon_cell_index(0, state.player.x, state.player.y))
                .copied();
            if state.player.x != 2
                || state.player.y != 1
                || state.turn != 1
                || !state.message.is_empty()
                || !underfoot.is_some_and(|tile| (0xE0..=0xEF).contains(&tile))
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not pass through the public 0xE? heavy-door variant"
                )));
            }
        }
        "dungeon-surface-exit-return-world" | "reload-dungeon-surface-exit-return-world" => {
            if !matches!(
                state.area,
                Area::World {
                    plane: WorldPlane::Britannia
                }
            ) || state.player.transport != TransportState::Foot
                || state.player.x != 240
                || state.player.y != 73
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not use Deceit's published Britannia exit coordinate"
                )));
            }
        }
        "ship-xit-launches-skiff" => {
            let parked_ship = state.active_objects.iter().skip(1).find(|object| {
                object.type_byte == FIRST_PLAYABLE_FRIGATE_TILE
                    && object.x == state.player.x
                    && object.y == state.player.y
                    && object.z == WorldPlane::Britannia.save_floor()
            });
            if state.turn != 2
                || !matches!(state.player.transport, TransportState::Skiff { .. })
                || parked_ship.is_none_or(|object| {
                    object.aux1 != FIRST_PLAYABLE_FULL_SHIP_HULL || object.aux3 != 1
                })
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not launch a skiff while parking the decremented ship hull"
                )));
            }
        }
        "ship-xit-no-skiffs-refusal" => {
            if state.turn != 0
                || !matches!(
                    state.player.transport,
                    TransportState::Ship {
                        sails_hoisted: false,
                        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
                        skiffs: 0,
                        ..
                    }
                )
                || state.message != SHIP_NO_SKIFFS_WARNING
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve the furled ship on the published no-skiffs refusal"
                )));
            }
        }
        "ship-yell-toggles-town-band" | "ship-yell-toggles-dungeon-band" => {
            let expected_scene = if case_name == "ship-yell-toggles-town-band" {
                0x11
            } else {
                0x21
            };
            if state.current_scene_byte() != expected_scene
                || state.turn != 1
                || !state.player.transport.is_ship_under_sail()
                || state.message != YELL_SAILS_HOISTED_MESSAGE
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not take the published low-scene-byte sail shortcut"
                )));
            }
        }
        "reload-ship-xit-skiff-pass" => {
            if !matches!(state.player.transport, TransportState::Skiff { .. })
                || state.active_objects.first().is_none_or(|object| {
                    object.x != state.player.x
                        || object.y != state.player.y
                        || object.z != WorldPlane::Britannia.save_floor()
                })
                || state.message != "Rough seas!"
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not preserve launched skiff transport or run the published deep-water epilogue across save/reload"
                )));
            }
        }
        // `dungeon-mode.md §14.1`: the wandering-monster launch "is **not**
        // the room-trigger `DUNGEON.CBT` path. No arena record is read from
        // disk on this path." The arena is synthesised, the party takes the
        // facing-selected entry row, and each nonzero source is "placed on the
        // ordinary path, recovering the same class the painter encoded".
        //
        // Both routes seed the party facing East, so the party entry row is
        // row two and the monster sources are the facing-east sixteen. The
        // exact source slot is *not* fixed: the painter writes `count` copies
        // of the source byte "into the first `count` permuted slots" of "a
        // shuffled permutation of the sixteen slot indices", so this asserts
        // membership in the published source set rather than one coordinate.
        // An earlier form of this case pinned the monster to (6,5) and read
        // the class out of the combat active-object's DEP1 byte; (6,5) is not
        // a source coordinate under any facing, and the room-combat placement
        // path carries the class in the object's tile byte, not DEP1.
        "dungeon-active-monster-attack-ambush" | "dungeon-active-monster-contact-ambush" => {
            let facing_seed = dungeon_room_entry_seed_for_direction(Direction::East);
            let (source_x, source_y) = dungeon_ambush_source_rows(facing_seed)
                .expect("east is one of the four published ambush facings");
            let party_row = usize::from(facing_seed);
            let monster_actor = state.combat_actors[COMBAT_PARTY_ACTOR_SLOTS];
            // Descriptors are scanned from index six, but the monster's
            // active-object RECORD comes from the lowest record the
            // seated party left free, so the two indexes diverge as
            // soon as the party is not exactly six strong. Follow the
            // descriptor's link byte, which `active-objects.md` section
            // 7 calls authoritative in both directions, rather than
            // assuming record index == descriptor index.
            let monster_object =
                state.active_objects[usize::from(monster_actor.active_object_slot)];
            let expected_tile = combat_class_sprite_byte(DUNGEON_MONSTER_COMBAT_CLASSES[0]);
            let monster_on_published_source = (0..DUNGEON_ROOM_SOURCE_COUNT)
                .any(|slot| source_x[slot] == monster_actor.x && source_y[slot] == monster_actor.y);
            let party_slots = state.party.len().min(COMBAT_PARTY_ACTOR_SLOTS);
            let party_on_entry_row = state
                .combat_actors
                .iter()
                .take(party_slots)
                .enumerate()
                .all(|(slot, actor)| {
                    actor.x == DUNGEON_AMBUSH_PARTY_ENTRY_X[party_row][slot]
                        && actor.y == DUNGEON_AMBUSH_PARTY_ENTRY_Y[party_row][slot]
                });
            if !state.combat_active
                || !state.message.contains("entered dungeon combat")
                || monster_object.tile != expected_tile
                || !monster_on_published_source
                || !party_on_entry_row
                || !state.combat_terrain.iter().all(|row| {
                    row.iter()
                        .all(|tile| *tile == DUNGEON_AMBUSH_ARENA_FLOOR_TILE)
                })
            {
                let bad_terrain = state
                    .combat_terrain
                    .iter()
                    .flat_map(|row| row.iter())
                    .filter(|tile| **tile != DUNGEON_AMBUSH_ARENA_FLOOR_TILE)
                    .count();
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not enter the public #21 dungeon ambush frame: \
                     combat_active={} msg={:?} monster_tile={:#04x} (want {expected_tile:#04x}) \
                     monster_actor=({},{}) on_published_east_source={monster_on_published_source} \
                     party_on_entry_row_{party_row}={party_on_entry_row} party_at={:?} \
                     non_floor_terrain_cells={bad_terrain}",
                    state.combat_active,
                    state.message,
                    monster_object.tile,
                    monster_actor.x,
                    monster_actor.y,
                    state
                        .combat_actors
                        .iter()
                        .take(party_slots)
                        .map(|actor| (actor.x, actor.y))
                        .collect::<Vec<_>>(),
                )));
            }
        }
        "dungeon-jimmy-no-keys-commits-action"
        | "dungeon-jimmy-no-lock-commits-action"
        | "dungeon-jimmy-cancel-commits-action"
        | "dungeon-jimmy-success-clears-trap-subtype" => {
            let index = dungeon_cell_index(
                state.current_floor().unwrap_or(0) as u8,
                state.player.x,
                state.player.y,
            );
            let expected = match case_name {
                "dungeon-jimmy-no-keys-commits-action" => {
                    state.keys == 0
                        && state.grid[index] == 0x4b
                        && state.prng_state == 0x1234
                        && state.message == "No keys!"
                }
                "dungeon-jimmy-no-lock-commits-action" => {
                    state.keys == 2
                        && state.grid[index] == 0x00
                        && state.prng_state == 0x1234
                        && state.message == "No lock!"
                }
                "dungeon-jimmy-cancel-commits-action" => {
                    state.keys == 2
                        && state.grid[index] == 0x4b
                        && state.prng_state == 0x1234
                        && state.message == "None!"
                }
                "dungeon-jimmy-success-clears-trap-subtype" => {
                    state.keys == 2
                        && state.grid[index] == 0x78
                        && state.prng_state == u5_prng_advance_state(0x1234)
                        && state.message == "Unlocked!"
                }
                _ => unreachable!("grouped Jimmy route names are exhaustive"),
            };
            if state.turn != 1 || state.active_jimmy.is_some() || !expected {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` missed the committed dungeon Jimmy outcome (turn {}, keys {}, cell {:#04x}, prng {:#06x}, prompt {}, message `{}`)",
                    state.turn,
                    state.keys,
                    state.grid[index],
                    state.prng_state,
                    state.active_jimmy.is_some(),
                    state.message
                )));
            }
        }
        "terrain-combat-party-entry" | "dungeon-room-party-entry" | "doom-room-combat-trigger" => {
            validate_combat_party_descriptor_links(state, case_name)?;
        }
        "britannia-pirate-broadside-damages-the-party" => {
            // `text-output.md §11`: "A turn that produces an epilogue
            // announcement *and* a command result shows the announcement
            // first, then the result beneath it." The walker announces the
            // broadside inside the turn epilogue and the command handler
            // then assigns its own result line, so this route is the
            // end-to-end check that the announcement still reaches the
            // player. It previously could not be asserted at all: the
            // announcement lived in the single message slot the handler
            // overwrote, and only the payload's effect on the roster was
            // durable evidence that the shot had happened.
            if !state
                .message_entries()
                .iter()
                .any(|entry| entry.text.contains(OUTDOOR_BROADSIDE_BOOM_MESSAGE))
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` lost the broadside announcement before the player could see it"
                )));
            }
            // `overworld.md §6.2.4`: on foot "[e]very qualifying member is
            // damaged", so the whole party is below full.
            let untouched = state
                .party
                .iter()
                .filter(|member| member.hp == member.max_hp)
                .count();
            if untouched != 0 {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` left {untouched} party member(s) at full hit points"
                )));
            }
        }
        "britannia-pirate-broadside-spends-ship-hull" => {
            // `vehicles.md §6`: "**The hull absorbs the impact entirely: no
            // party member loses hit points while the ship survives.**"
            let TransportState::Ship { hull, .. } = state.player.transport else {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not stay aboard the frigate"
                )));
            };
            if hull >= OUTDOOR_IMPACT_HULL_ROLL_HIGH + 1 || hull == 0 {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` left hull at {hull}"
                )));
            }
            if state.party.iter().any(|member| member.hp != member.max_hp) {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` damaged a party member while the ship survived"
                )));
            }
        }
        "doom-combat-quit-refusal" => {
            if !state.combat_active
                || !state.message.contains("Quit-Not here")
                || state.combat_frame_snapshot.is_none()
                || state.pending_combat_actor_slot.is_none()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not refuse Quit and re-prompt the same combat actor"
                )));
            }
        }
        "terrain-combat-escape-announced-cleanup" => {
            if state.combat_active
                || state.message != "Escape!"
                || state.combat_frame_snapshot.is_some()
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not run announced Escape cleanup (combat_active={}, snapshot={}, message `{}`)",
                    state.combat_active,
                    state.combat_frame_snapshot.is_some(),
                    state.message
                )));
            }
        }
        "terrain-combat-out-of-arena-leave" => {
            if !state.combat_active
                || !state.message.contains("Escape!")
                || state.combat_frame_snapshot.is_none()
                || !state.combat_actors[0].is_empty()
                || state
                    .active_objects
                    .first()
                    .is_some_and(|object| !object.is_empty())
            {
                return Err(io::Error::other(format!(
                    "route smoke `{case_name}` did not release only the acting combatant on an out-of-arena leave (combat_active={}, snapshot={}, message `{}`)",
                    state.combat_active,
                    state.combat_frame_snapshot.is_some(),
                    state.message
                )));
            }
        }
        _ => {}
    }
    Ok(())
}

fn seed_public_location_route_position(state: &mut PlayState, index: usize) -> io::Result<()> {
    let Some(entry) = published_world_location_entries().into_iter().nth(index) else {
        return Err(io::Error::other(format!(
            "published location route index {index} is out of range"
        )));
    };
    state.area = Area::World { plane: entry.plane };
    state.player.x = entry.x;
    state.player.y = entry.y;
    if let Some(tile) = entry.expected_tile {
        state.grid[world_cell_index(entry.x, entry.y)] = tile;
    }
    if let Some(object) = state.active_objects.get_mut(0) {
        object.z = entry.plane.save_floor();
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
    Ok(())
}

fn route_smoke_public_location_index(case_name: &str) -> Option<usize> {
    let suffix = case_name.strip_prefix("stock-location-enter-")?;
    let row = suffix.parse::<usize>().ok()?;
    (1..=published_world_location_entries().len())
        .contains(&row)
        .then_some(row - 1)
}

fn clear_route_combat_non_party_actors(state: &mut PlayState) {
    for slot in COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS {
        state.combat_actors[slot].clear();
        if let Some(object) = state.active_objects.get_mut(slot) {
            *object = ActiveObject::empty();
        }
    }
}

fn seed_route_combat_party_actor_at_east_edge(state: &mut PlayState) {
    seed_route_combat_pending_party_actor(state);
    if let Some(actor) = state.combat_actors.get_mut(0) {
        actor.x = (COMBAT_ARENA_SIDE - 1) as u8;
        actor.y = 5;
    }
    if let Some(object) = state.active_objects.get_mut(0) {
        object.x = COMBAT_ARENA_SIDE - 1;
        object.y = 5;
    }
}

fn seed_route_combat_pending_party_actor(state: &mut PlayState) {
    state.active_player = Some(0);
    state.pending_combat_actor_slot = Some(0);
}

fn validate_combat_party_descriptor_links(state: &PlayState, case_name: &str) -> io::Result<()> {
    if !state.combat_active {
        return Err(io::Error::other(format!(
            "route smoke `{case_name}` did not enter combat"
        )));
    }

    for slot in 0..state.party.len().min(COMBAT_PARTY_ACTOR_SLOTS) {
        if !state.party[slot].conscious() {
            continue;
        }
        let actor = state.combat_actors[slot];
        let Some(object) = state.active_objects.get(slot).copied() else {
            return Err(io::Error::other(format!(
                "route smoke `{case_name}` did not seed party active-object slot {slot}"
            )));
        };
        // `combat.md §5` party descriptor seeding: base-step is the
        // character's dexterity and the phase counter is thirty-six
        // minus it. The class stat table's speed seed is a monster
        // placement input and has no part in party seating.
        let expected_step = state.party[slot].dexterity();
        let expected_phase = COMBAT_PLACEMENT_PHASE_BASE.saturating_sub(expected_step);
        if actor.owner_target_class != slot as u8
            || actor.active_object_slot != slot as u8
            || actor.base_step != expected_step
            || actor.phase_counter != expected_phase
            || actor.x != object.x as u8
            || actor.y != object.y as u8
            || !actor.has_field_lookup_selectable_bit()
        {
            return Err(io::Error::other(format!(
                "route smoke `{case_name}` did not seed party slot {slot} with the public combat descriptor link"
            )));
        }
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
    if state.shadowlord_hideouts.get(shadowlord_index).copied() != Some(SHADOWLORD_VANQUISHED)
        || state.special_items.get(item_index).copied() != Some(0)
        || state
            .active_objects
            .iter()
            .copied()
            .skip(1)
            .any(PlayState::is_shadowlord_actor)
        || state.summoned_shadowlord.is_some()
        || !state.message.contains(message_fragment)
    {
        return Err(io::Error::other(format!(
            "route smoke `{case_name}` did not complete native shard destruction: \
             hideout={:?}, item={:?}, player=({}, {}, {:?}), actors={:?}, summoned={:?}, message={:?}",
            state.shadowlord_hideouts.get(shadowlord_index),
            state.special_items.get(item_index),
            state.player.x,
            state.player.y,
            state.current_floor(),
            state
                .active_objects
                .iter()
                .copied()
                .skip(1)
                .filter(|object| PlayState::is_shadowlord_actor(*object))
                .map(|object| (object.x, object.y, object.z))
                .collect::<Vec<_>>(),
            state.summoned_shadowlord,
            state.message,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashMap, HashSet};

    const SOURCE: &str = include_str!("route_smoke.rs");

    /// One `match case_name { .. }` block: the 1-based line it opens on and
    /// the case-name literals its arms are keyed on, in source order.
    struct CaseNameMatchBlock {
        line: usize,
        literals: Vec<String>,
    }

    /// Walk this file's own source for every `match case_name` block.
    ///
    /// The scan leans on rustfmt's layout rather than on brace counting,
    /// which string literals containing braces would throw off: the block
    /// ends at the first line that is exactly the `match`'s own indent
    /// followed by `}`, and an arm pattern is a line at exactly one
    /// indent step deeper that starts with a string literal or `| `.
    fn case_name_match_blocks() -> Vec<CaseNameMatchBlock> {
        let lines: Vec<&str> = SOURCE.lines().collect();
        let mut blocks = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            if line.trim() != "match case_name {" {
                continue;
            }
            let indent = line.len() - line.trim_start().len();
            let closing = format!("{}}}", " ".repeat(indent));
            let arm_indent = indent + 4;
            let mut literals = Vec::new();
            for probe in &lines[index + 1..] {
                if *probe == closing {
                    break;
                }
                if probe.len() - probe.trim_start().len() != arm_indent {
                    continue;
                }
                let trimmed = probe.trim_start();
                if !(trimmed.starts_with('"') || trimmed.starts_with("| \"")) {
                    continue;
                }
                // Arm patterns only: stop at the fat arrow so a guard or an
                // arm body on the same line contributes nothing.
                let pattern = trimmed.split("=>").next().unwrap_or(trimmed);
                literals.extend(pattern.split('"').skip(1).step_by(2).map(str::to_string));
            }
            blocks.push(CaseNameMatchBlock {
                line: index + 1,
                literals,
            });
        }
        blocks
    }

    #[test]
    fn source_scan_finds_the_setup_and_validation_case_name_blocks() {
        // Guard the scanner itself: if rustfmt or a refactor moves these
        // blocks out from under the heuristic above, the two tests below
        // would quietly stop checking anything.
        let blocks = case_name_match_blocks();
        assert!(
            blocks.len() >= 2,
            "expected several `match case_name` blocks, found {}",
            blocks.len()
        );
        let biggest = blocks
            .iter()
            .map(|block| block.literals.len())
            .max()
            .unwrap_or_default();
        assert!(
            biggest > 50,
            "the setup/validation blocks key on many cases; largest block found keys on {biggest}"
        );
    }

    #[test]
    fn no_case_name_is_matched_twice_inside_one_block() {
        // `deny(unreachable_patterns)` already makes this a build error.
        // The test states the invariant in the harness's own terms and
        // still holds if the lint is ever relaxed.
        for block in case_name_match_blocks() {
            let mut seen = HashSet::new();
            for literal in &block.literals {
                assert!(
                    seen.insert(literal.clone()),
                    "`match case_name` at line {} keys on `{literal}` twice;                      the second arm can never run",
                    block.line
                );
            }
        }
    }

    #[test]
    fn every_case_name_arm_names_a_real_route_smoke_case() {
        // An arm keyed on a name no case carries never fires, so the case
        // it was meant to cover passes without being set up or validated.
        let cases = route_smoke_cases();
        let names: HashSet<&str> = cases.iter().map(|case| case.name).collect();
        let mut orphans: HashMap<String, usize> = HashMap::new();
        for block in case_name_match_blocks() {
            for literal in &block.literals {
                if !names.contains(literal.as_str()) {
                    orphans.insert(literal.clone(), block.line);
                }
            }
        }
        assert!(
            orphans.is_empty(),
            "route-smoke `match case_name` arms key on names no case carries: {orphans:?}"
        );
    }
}

#[cfg(test)]
mod scratch_run {
    use super::*;
    use u5_runtime::DEFAULT_GAME_DIR;

    #[test]
    fn scratch_run_harpsichord_routes() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join("CASTLE.DAT").exists() {
            println!("no assets");
            return;
        }
        let atlas = load_tile_atlas(game_dir, TileGraphicsDepth::Ega16).unwrap();
        let mut failures = 0;
        for case in route_smoke_cases()
            .iter()
            .filter(|case| case.name.contains("harpsichord"))
        {
            match run_route_smoke_case(game_dir, &atlas, case) {
                Ok(report) => println!("OK   {}: {}", report.name, report.final_state_line),
                Err(err) => {
                    failures += 1;
                    println!("FAIL {}: {err}", case.name);
                }
            }
        }
        assert_eq!(failures, 0, "harpsichord routes failed");
    }
}

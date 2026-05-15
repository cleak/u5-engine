//! Engine-wide constants: filenames, save offsets, tile sentinels,
//! world dimensions, spell tables, defaults.

pub const DEFAULT_GAME_DIR: &str = r"C:\Games\U5-Clean";
pub const REPORT_PATH: &str = "reports/lb-throne-room-slice.txt";
pub const WORLD_LOCATION_TABLE_FILE: &str = "world_locations.tsv";
pub const WORLD_PLANE_TRANSITION_TABLE_FILE: &str = "world_plane_transitions.tsv";
pub const WORLD_GET_TILE_TABLE_FILE: &str = "world_get_tiles.tsv";
pub const OBJECT_PICKUP_TABLE_FILE: &str = "object_pickups.tsv";
pub const WORLD_WATERFALL_TABLE_FILE: &str = "world_waterfalls.tsv";
pub const WORLD_DAMAGE_TILE_TABLE_FILE: &str = "world_damage_tiles.tsv";
pub const WORLD_ENCOUNTER_TABLE_FILE: &str = "world_encounters.tsv";
pub const SHRINE_TABLE_FILE: &str = "shrines.tsv";
pub const DUNGEON_DEEPER_TRANSITION_TABLE_FILE: &str = "dungeon_deeper_transitions.tsv";
pub const DUNGEON_TELEPORT_TABLE_FILE: &str = "dungeon_teleports.tsv";
pub const DUNGEON_WIND_TILE_TABLE_FILE: &str = "dungeon_wind_tiles.tsv";
pub const DUNGEON_EXIT_TILE_TABLE_FILE: &str = "dungeon_exit_tiles.tsv";
pub const DUNGEON_DOOR_TABLE_FILE: &str = "dungeon_doors.tsv";
pub const DUNGEON_CHEST_TABLE_FILE: &str = "dungeon_chests.tsv";
pub const SECRET_DOOR_TABLE_FILE: &str = "secret_doors.tsv";
pub const TOWN_FIRE_SOURCE_TABLE_FILE: &str = "town_fire_sources.tsv";
pub const TOWN_PUSHABLE_TABLE_FILE: &str = "town_pushables.tsv";
pub const TOWN_GET_TILE_TABLE_FILE: &str = "town_get_tiles.tsv";
pub const TOWN_REST_BED_TABLE_FILE: &str = "town_rest_beds.tsv";
pub const TOWN_STAIR_TABLE_FILE: &str = "town_stairs.tsv";
pub const TOWN_TRAP_DOOR_TABLE_FILE: &str = "town_trap_doors.tsv";
pub const TOWN_EXIT_TILE_TABLE_FILE: &str = "town_exit_tiles.tsv";
pub const TOWN_LOCK_TABLE_FILE: &str = "town_locks.tsv";
pub const BLINK_TARGET_TABLE_FILE: &str = "blink_targets.tsv";
pub const MOONGATE_TABLE_FILE: &str = "moongates.tsv";
pub const LOCATION_FLOOR_TABLE_FILE: &str = "location_floor_pages.tsv";
pub const LOCATION_ENTRY_Y_TABLE_FILE: &str = "location_entry_y.tsv";
pub const TILE_PASSABILITY_FILE: &str = "tile_passability.bin";
pub const LOOK2_DAT_FILE: &str = "LOOK2.DAT";
pub const KARMA_DAT_FILE: &str = "KARMA.DAT";
pub const TILES_EGA_FILE: &str = "TILES.16";
pub const TILES_CGA_FILE: &str = "TILES.4";
#[cfg(test)]
pub const TITLE_BIT_FILE: &str = "TITLE.BIT";
#[cfg(test)]
pub const BRITISH_BIT_FILE: &str = "BRITISH.BIT";
#[cfg(test)]
pub const WD_BIT_FILE: &str = "WD.BIT";
#[cfg(test)]
pub const IBM_CH_FILE: &str = "IBM.CH";
#[cfg(test)]
pub const RUNES_CH_FILE: &str = "RUNES.CH";
#[cfg(test)]
pub const IBM_HCS_FILE: &str = "IBM.HCS";
#[cfg(test)]
pub const RUNES_HCS_FILE: &str = "RUNES.HCS";
#[cfg(test)]
pub const PROPORT_PCS_FILE: &str = "PROPORT.PCS";
pub const TILE_PASSABILITY_LEN: usize = 32;
pub const LOOK2_TILE_COUNT: usize = 512;
pub const LOOK2_TABLE_LEN: usize = LOOK2_TILE_COUNT * 2;
pub const TILE_ATLAS_TILE_COUNT: usize = 512;
pub const TILE_ATLAS_SIDE: usize = 16;
pub const TILE_ATLAS_TILE_PIXELS: usize = TILE_ATLAS_SIDE * TILE_ATLAS_SIDE;
pub const TILE_ATLAS_PIXEL_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_TILE_PIXELS;
pub const TILE_ATLAS_EGA_TILE_STRIDE: usize = TILE_ATLAS_TILE_PIXELS / 2;
pub const TILE_ATLAS_CGA_TILE_STRIDE: usize = TILE_ATLAS_TILE_PIXELS / 4;
pub const TILE_ATLAS_EGA_BODY_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_EGA_TILE_STRIDE;
pub const TILE_ATLAS_CGA_BODY_LEN: usize = TILE_ATLAS_TILE_COUNT * TILE_ATLAS_CGA_TILE_STRIDE;
pub const LZW_CLEAR_CODE: u16 = 256;
pub const LZW_END_CODE: u16 = 257;
pub const LZW_FIRST_USER_CODE: u16 = 258;
pub const LZW_MAX_CODES: u16 = 4096;
pub const LZW_INITIAL_CODE_SIZE: u8 = 9;
pub const LZW_MAX_CODE_SIZE: u8 = 12;
#[cfg(test)]
pub const SINGLE_IMAGE_BIT_FORMAT_MARKER: u16 = 1;
#[cfg(test)]
pub const SINGLE_IMAGE_BIT_MODE_MARKER: u16 = 4;
#[cfg(test)]
pub const FIXED_FONT_GLYPH_COUNT: usize = 128;
#[cfg(test)]
pub const CH_FONT_CELL_WIDTH: usize = 8;
#[cfg(test)]
pub const CH_FONT_CELL_HEIGHT: usize = 8;
#[cfg(test)]
pub const HCS_FONT_CELL_WIDTH: usize = 16;
#[cfg(test)]
pub const HCS_FONT_CELL_HEIGHT: usize = 12;
#[cfg(test)]
pub const PCS_FIRST_CODE: u8 = 0x20;
#[cfg(test)]
pub const PCS_GLYPH_BITMAP_WIDTH: usize = 8;
#[cfg(test)]
pub const PCS_GLYPH_HEIGHT: usize = 11;
#[cfg(test)]
pub const PCS_GLYPH_BLOCK_LEN: usize = 1 + PCS_GLYPH_HEIGHT;
pub const PLAY_SCRIPT_MAX_IDLE_TICKS: usize = 1024;
pub const KARMA_RECORD_COUNT: usize = 6;
pub const PLAY_IGNORED_INPUT_KEY: char = '\u{1e}';
pub const PLAY_TYPEAHEAD_TOGGLE_KEY: char = '\u{1f}';
pub const TRAP_NON_COMBAT_EFFECT_TABLE: [u8; 8] = [0, 0, 0, 1, 1, 2, 2, 3];
pub const TRAP_ACID_DAMAGE_MAX: u8 = 30;
pub const TRAP_BOMB_DAMAGE_MAX: u8 = 8;
// Sentinel value the active-object table uses to mark the player slot
// (slot zero). Per `u5-spec/catalogs/tile-catalog.md` Section 14, this
// is "the player avatar sprite sentinel value 0xFC referenced in the
// town-entry handler" -- a marker, NOT the actual sprite to render.
// `PLAYER_SPRITE_TILE` below is what the renderer should display.
pub const PLAYER_TILE: u8 = 0xfc;

// The actual avatar sprite tile id in the EGA atlas. The character
// sprites live in the upper half of the 9-bit tile space (256..=511);
// tile 0x144 is the south-facing on-foot avatar walking frame. LOOK2.DAT
// labels lower-half 0xFC as "a bellows" which is why a literal blit of
// PLAYER_TILE shows a blacksmith's bellows on the map.
pub const PLAYER_SPRITE_TILE: usize = 0x144;

// Moongate is a single static sprite at tile id 0xDC per LOOK2.DAT
// ("a moon gate!"). Earlier guesses at 0x80 and 0xD4 picked the wrong
// tiles (food/banquet and a waterfall animation respectively).
pub const MOONGATE_TILE_BASE: u8 = 0xDC;
pub const MOONGATE_ANIMATION_FRAMES: u8 = 1;
pub const NATURAL_MOONGATE_TERRAIN_TILE: u8 = 0xDC;
pub const NATURAL_MOONGATE_RESTORED_TERRAIN_TILE: u8 = 5;
pub const NATURAL_MOONGATE_COUNTER_MAX: u8 = 16;
pub const STEADY_PHASE: u8 = 0x0f;
pub const PLAY_START_YEAR: u16 = 139;
pub const PLAY_START_MONTH: u8 = 4;
pub const PLAY_START_DAY: u8 = 5;
pub const PLAY_START_HOUR: u8 = 12;
pub const SAVED_GAM_LEN: usize = 4192;
pub const SAVE_FOOD_STOCK_OFFSET: usize = 0x0202;
pub const SAVE_GOLD_STOCK_OFFSET: usize = 0x0204;
pub const SAVE_KEY_STOCK_OFFSET: usize = 0x0206;
pub const SAVE_GEM_STOCK_OFFSET: usize = 0x0207;
pub const SAVE_TORCH_STOCK_OFFSET: usize = 0x0208;
pub const SAVE_CLIMBING_GEAR_OFFSET: usize = 0x0209;
pub const SAVE_SPECIAL_ITEM_OFFSET: usize = 0x020a;
pub const SAVE_EQUIPMENT_STOCK_OFFSET: usize = 0x021a;
pub const SAVE_SPELL_CHARGES_OFFSET: usize = 0x024a;
pub const SAVE_SCROLL_STOCK_OFFSET: usize = 0x027a;
pub const SAVE_POTION_STOCK_OFFSET: usize = 0x0282;
pub const SAVE_MOONSTONE_X_OFFSET: usize = 0x028a;
pub const SAVE_MOONSTONE_Y_OFFSET: usize = 0x0292;
pub const SAVE_MOONSTONE_SCENE_OFFSET: usize = 0x029a;
pub const SAVE_MOONSTONE_Z_OFFSET: usize = 0x02a2;
pub const SAVE_REAGENTS_OFFSET: usize = 0x02aa;
pub const SAVE_YEAR_OFFSET: usize = 0x02ce;
pub const SAVE_TIMING_STATUS_TAG_OFFSET: usize = 0x02d4;
pub const SAVE_ACTIVE_PLAYER_OFFSET: usize = 0x02d5;
pub const SAVE_TRANSPORT_MARKER_OFFSET: usize = 0x02d6;
pub const SAVE_MONTH_OFFSET: usize = 0x02d7;
pub const SAVE_DAY_OFFSET: usize = 0x02d8;
pub const SAVE_HOUR_OFFSET: usize = 0x02d9;
pub const SAVE_MINUTE_OFFSET: usize = 0x02db;
pub const SAVE_COMBAT_ROUND_COUNTER_OFFSET: usize = 0x02dc;
pub const SAVE_AMPM_DISPLAY_OFFSET: usize = 0x02de;
pub const SAVE_MORAL_STANDING_OFFSET: usize = 0x02e2;
pub const SAVE_WIND_OFFSET: usize = 0x02ec;
pub const SAVE_SCENE_OFFSET: usize = 0x02ed;
pub const SAVE_Z_OFFSET: usize = 0x02ef;
pub const SAVE_X_OFFSET: usize = 0x02f0;
pub const SAVE_Y_OFFSET: usize = 0x02f1;
pub const SAVE_LIGHT_SPELL_COUNTER_OFFSET: usize = 0x0300;
pub const SAVE_TORCH_COUNTER_OFFSET: usize = 0x0301;
pub const SAVE_SHRINE_ORDAINED_MASK_OFFSET: usize = 0x0326;
pub const SAVE_SHRINE_CODEX_MASK_OFFSET: usize = 0x0328;
pub const SAVE_FORTUNES_OF_WAR_OFFSET: usize = 0x03b3;
pub const SAVE_AVATAR_NAME_OFFSET: usize = 0x0002;
pub const SAVE_AVATAR_NAME_LEN: usize = 9;
pub const SAVE_ACTIVE_OBJECTS_OFFSET: usize = 0x06b4;
pub const SAVE_PARTY_SIZE_OFFSET: usize = 0x02b5;
pub const SAVE_ROSTER_OFFSET: usize = 0x0002;
pub const SAVE_INN_REGISTRY_OFFSET: usize = 0x0021;
pub const SAVE_INN_REGISTRY_COUNT: usize = 16;
pub const SAVE_CHARACTER_RECORD_LEN: usize = 32;
pub const SAVE_CHARACTER_NAME_LEN: usize = 9;
pub const SAVE_CHARACTER_GENDER_OFFSET: usize = 0x09;
pub const SAVE_GENDER_MALE_BYTE: u8 = 0x0b;
pub const SAVE_GENDER_FEMALE_BYTE: u8 = 0x0c;
pub const SAVE_CHARACTER_CLASS_OFFSET: usize = 0x0a;
pub const SAVE_CHARACTER_STATUS_OFFSET: usize = 0x0b;
pub const SAVE_CHARACTER_STR_OFFSET: usize = 0x0c;
pub const SAVE_CHARACTER_DEX_OFFSET: usize = 0x0d;
pub const SAVE_CHARACTER_INT_OFFSET: usize = 0x0e;
pub const SAVE_CHARACTER_MANA_OFFSET: usize = 0x0f;
pub const SAVE_CHARACTER_HP_OFFSET: usize = 0x10;
pub const SAVE_CHARACTER_MAX_HP_OFFSET: usize = 0x12;
pub const SAVE_CHARACTER_EXPERIENCE_OFFSET: usize = 0x14;
pub const SAVE_CHARACTER_LEVEL_OFFSET: usize = 0x16;
pub const SAVE_CHARACTER_STAY_COUNTER_OFFSET: usize = 0x17;
pub const SAVE_CHARACTER_EQUIPMENT_OFFSET: usize = 0x19;
pub const SPELL_COUNT: usize = 48;
pub const EQUIPMENT_COUNT: usize = 48;
pub const SCROLL_COUNT: usize = 8;
pub const POTION_COUNT: usize = 8;
pub const POTION_BLUE_INDEX: usize = 0;
pub const POTION_YELLOW_INDEX: usize = 1;
pub const POTION_RED_INDEX: usize = 2;
pub const POTION_GREEN_INDEX: usize = 3;
pub const POTION_ORANGE_INDEX: usize = 4;
pub const POTION_PURPLE_INDEX: usize = 5;
pub const POTION_BLACK_INDEX: usize = 6;
pub const POTION_WHITE_INDEX: usize = 7;
pub const SHADOWLORD_COUNT: usize = 3;
pub const SHADOWLORD_FALSEHOOD_INDEX: usize = 0;
pub const SHADOWLORD_HATRED_INDEX: usize = 1;
pub const SHADOWLORD_COWARDICE_INDEX: usize = 2;
pub const SHADOWLORD_HIDEOUT_MIN: u8 = 1;
pub const SHADOWLORD_HIDEOUT_MAX: u8 = 8;
pub const SHADOWLORD_VANQUISHED: u8 = 0xff;
pub const DEFAULT_SHADOWLORD_HIDEOUTS: [u8; SHADOWLORD_COUNT] = [1, 2, 3];
pub const SHADOWLORD_OBJECT_TILE_BASE: u8 = 0xfd;
pub const SCROLL_LIGHT_INDEX: usize = 0;
pub const SCROLL_WIND_CHANGE_INDEX: usize = 1;
pub const SCROLL_PROTECTION_INDEX: usize = 2;
pub const SCROLL_NEGATE_MAGIC_INDEX: usize = 3;
pub const SCROLL_VIEW_INDEX: usize = 4;
pub const SCROLL_SUMMON_DAEMON_INDEX: usize = 5;
pub const SCROLL_RESURRECTION_INDEX: usize = 6;
pub const SCROLL_NEGATE_TIME_INDEX: usize = 7;
pub const SCROLL_LIGHT_DURATION: u8 = 240;
pub const SCROLL_PROTECTION_DURATION: u8 = 100;
pub const SCROLL_NEGATE_MAGIC_DURATION: u8 = 20;
pub const SCROLL_NEGATE_TIME_DURATION: u8 = 20;
pub const SPECIAL_ITEM_COUNT: usize = 16;
pub const SPECIAL_ITEM_MAGIC_CARPET_INDEX: usize = 0x00;
pub const SPECIAL_ITEM_SKULL_KEY_INDEX: usize = 0x01;
pub const SPECIAL_ITEM_AMULET_LB_INDEX: usize = 0x03;
pub const SPECIAL_ITEM_CROWN_LB_INDEX: usize = 0x04;
pub const SPECIAL_ITEM_SCEPTRE_LB_INDEX: usize = 0x05;
pub const SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX: usize = 0x06;
pub const SPECIAL_ITEM_SHARD_HATRED_INDEX: usize = 0x07;
pub const SPECIAL_ITEM_SHARD_COWARDICE_INDEX: usize = 0x08;
pub const SPECIAL_ITEM_SPYGLASS_INDEX: usize = 0x0a;
pub const SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX: usize = 0x0b;
pub const SPECIAL_ITEM_SEXTANT_INDEX: usize = 0x0c;
pub const SPECIAL_ITEM_POCKET_WATCH_INDEX: usize = 0x0d;
pub const SPECIAL_ITEM_BLACK_BADGE_INDEX: usize = 0x0e;
pub const SPECIAL_ITEM_WOODEN_BOX_INDEX: usize = 0x0f;
pub const SPECIAL_ITEM_OWNED_VALUE: u8 = 1;
pub const SPECIAL_ITEM_WORN_VALUE: u8 = 2;
pub const LORD_BLACKTHORN_CASTLE_SCENE_BYTE: u8 = 18;
pub const STONEGATE_SCENE_BYTE: u8 = 29;
pub const DOOM_DUNGEON_RECORD: usize = 7;
pub const DOOM_FINAL_ROOM_LEVEL: u8 = 7;
pub const DOOM_FINAL_ROOM_X: usize = 5;
pub const DOOM_FINAL_ROOM_Y: usize = 7;
pub const DOOM_FINAL_ROOM_SLOT: u8 = 15;
pub const EQUIPMENT_SLOT_COUNT: usize = 6;
pub const EQUIPMENT_EMPTY: u8 = 0xff;
pub const EQUIPMENT_STOCK_CAP: u8 = 99;
pub const EQUIP_SLOT_HELM: usize = 0;
pub const EQUIP_SLOT_ARMOUR: usize = 1;
pub const EQUIP_SLOT_WEAPON: usize = 2;
pub const EQUIP_SLOT_OFFHAND: usize = 3;
pub const EQUIP_SLOT_RING: usize = 4;
pub const EQUIP_SLOT_AMULET: usize = 5;
pub const EQUIPMENT_TAG_AMMO: u8 = 0x00;
pub const EQUIPMENT_TAG_RING: u8 = 0x02;
pub const EQUIPMENT_TAG_AMULET: u8 = 0x04;
pub const EQUIPMENT_TAG_ONE_HAND: u8 = 0x20;
pub const EQUIPMENT_TAG_TWO_HAND: u8 = 0x30;
pub const EQUIPMENT_TAG_ARMOUR: u8 = 0x40;
pub const EQUIPMENT_TAG_HELM: u8 = 0x80;
pub const EQUIPMENT_ID_BOW: usize = 26;
pub const EQUIPMENT_ID_ARROWS: usize = 27;
pub const EQUIPMENT_ID_CROSSBOW: usize = 28;
pub const EQUIPMENT_ID_QUARRELS: usize = 29;
pub const EQUIPMENT_ID_MAGIC_BOW: usize = 36;
pub const EQUIPMENT_ID_RING_INVISIBILITY: usize = 42;
pub const EQUIPMENT_ID_RING_REGENERATION: usize = 44;
pub const EQUIPMENT_ID_AMULET_TURNING: usize = 45;
pub const REAGENT_COUNT: usize = 8;
pub const VIRTUE_COUNT: usize = 8;
pub const SHRINE_STANDING_MAX: u8 = 99;
pub const MORAL_STANDING_MAX: u8 = 99;
pub const AVATAR_STAT_MAX: u8 = 30;
pub const REAGENT_SULFUR_ASH: usize = 0;
pub const REAGENT_GINSENG: usize = 1;
pub const REAGENT_GARLIC: usize = 2;
pub const REAGENT_SPIDER_SILK: usize = 3;
pub const REAGENT_BLOOD_MOSS: usize = 4;
pub const REAGENT_BLACK_PEARL: usize = 5;
pub const REAGENT_NIGHTSHADE: usize = 6;
pub const REAGENT_MANDRAKE: usize = 7;
pub const RARE_REAGENT_HARVEST_POINT_COUNT: usize = 3;
pub const RARE_REAGENT_HARVEST_UNSEEN_DAY: u8 = 0;
pub const FIXED_HIDDEN_TREASURE_COUNT: usize = 113;
pub const FIXED_HIDDEN_TREASURE_FOUND_BYTES: usize = 15;
pub const FIXED_HIDDEN_TREASURE_DAILY_UNSEEN_DAY: u8 = 0;
pub const FIXED_HIDDEN_TREASURE_OBJECT_TILE: u8 = 0x1f;
pub const FIXED_HIDDEN_TREASURE_OBJECT_AUX3: u8 = 0xa5;
pub const REAGENT_MASKS: [u8; REAGENT_COUNT] = [0x80, 0x40, 0x20, 0x10, 0x08, 0x04, 0x02, 0x01];
pub const REAGENT_SAVE_ORDER: [usize; REAGENT_COUNT] = [
    REAGENT_BLACK_PEARL,
    REAGENT_BLOOD_MOSS,
    REAGENT_GARLIC,
    REAGENT_GINSENG,
    REAGENT_MANDRAKE,
    REAGENT_NIGHTSHADE,
    REAGENT_SPIDER_SILK,
    REAGENT_SULFUR_ASH,
];
pub const DEFAULT_REAGENTS: [u8; REAGENT_COUNT] = [0, 6, 7, 0, 6, 4, 3, 0];
pub const IN_LOR_SPELL_INDEX: usize = 0;
pub const IN_LOR_COST: u8 = 1;
pub const IN_LOR_LIGHT_DURATION: u8 = 100;
pub const AWAKEN_SPELL_INDEX: usize = 2;
pub const AWAKEN_COST: u8 = 1;
pub const CURE_SPELL_INDEX: usize = 3;
pub const CURE_COST: u8 = 1;
pub const HEAL_SPELL_INDEX: usize = 4;
pub const HEAL_COST: u8 = 1;
pub const HEAL_RAW_ROLL_MAX: u8 = 60;
pub const VANISH_SPELL_INDEX: usize = 5;
pub const VANISH_COST: u8 = 1;
pub const OPEN_SPELL_INDEX: usize = 6;
pub const OPEN_SPELL_COST: u8 = 2;
pub const REL_HUR_SPELL_INDEX: usize = 8;
pub const REL_HUR_COST: u8 = 2;
pub const IN_WIS_SPELL_INDEX: usize = 9;
pub const IN_WIS_COST: u8 = 2;
pub const CREATE_FOOD_SPELL_INDEX: usize = 11;
pub const CREATE_FOOD_COST: u8 = 2;
pub const CREATE_FOOD_AMOUNT: u16 = 100;
pub const VAS_LOR_SPELL_INDEX: usize = 12;
pub const VAS_LOR_COST: u8 = 3;
pub const VAS_LOR_LIGHT_DURATION: u8 = 255;
pub const FIRE_FIELD_SPELL_INDEX: usize = 14;
pub const POISON_FIELD_SPELL_INDEX: usize = 15;
pub const SLEEP_FIELD_SPELL_INDEX: usize = 16;
pub const FIELD_SPELL_COST: u8 = 3;
pub const BLINK_SPELL_INDEX: usize = 17;
pub const BLINK_COST: u8 = 3;
pub const DISPEL_FIELD_SPELL_INDEX: usize = 18;
pub const DISPEL_FIELD_COST: u8 = 4;
pub const PROTECTION_SPELL_INDEX: usize = 19;
pub const PROTECTION_COST: u8 = 4;
pub const PROTECTION_ACTIVE_EFFECT_TAG: u8 = b'P';
pub const PROTECTION_ACTIVE_EFFECT_DURATION: u8 = 20;
pub const UUS_POR_SPELL_INDEX: usize = 21;
pub const DES_POR_SPELL_INDEX: usize = 22;
pub const DUNGEON_LEVEL_SPELL_COST: u8 = 4;
pub const REVEAL_SPELL_INDEX: usize = 23;
pub const REVEAL_COST: u8 = 4;
pub const ENERGY_FIELD_SPELL_INDEX: usize = 20;
pub const ENERGY_FIELD_COST: u8 = 4;
pub const MAGIC_LOCK_SPELL_INDEX: usize = 25;
pub const MAGIC_LOCK_COST: u8 = 5;
pub const UNLOCK_MAGIC_SPELL_INDEX: usize = 26;
pub const UNLOCK_MAGIC_COST: u8 = 5;
pub const GREAT_HEAL_SPELL_INDEX: usize = 27;
pub const GREAT_HEAL_COST: u8 = 5;
pub const QUICKNESS_SPELL_INDEX: usize = 29;
pub const QUICKNESS_COST: u8 = 5;
pub const QUICKNESS_ACTIVE_EFFECT_TAG: u8 = b'Q';
pub const QUICKNESS_ACTIVE_EFFECT_DURATION: u8 = 30;
pub const MASS_CHARM_SPELL_INDEX: usize = 31;
pub const MASS_CHARM_COST: u8 = 6;
pub const MASS_CHARM_ACTIVE_EFFECT_TAG: u8 = b'C';
pub const MASS_CHARM_ACTIVE_EFFECT_DURATION: u8 = 20;
pub const NEGATE_MAGIC_SPELL_INDEX: usize = 32;
pub const NEGATE_MAGIC_COST: u8 = 6;
pub const NEGATE_MAGIC_ACTIVE_EFFECT_TAG: u8 = b'N';
pub const NEGATE_MAGIC_ACTIVE_EFFECT_DURATION: u8 = 10;
pub const X_RAY_SPELL_INDEX: usize = 33;
pub const X_RAY_COST: u8 = 6;
pub const INVISIBILITY_SPELL_INDEX: usize = 36;
pub const INVISIBILITY_COST: u8 = 7;
pub const CAUSE_FEAR_SPELL_INDEX: usize = 41;
pub const CAUSE_FEAR_COST: u8 = 7;
pub const PEER_SPELL_INDEX: usize = 39;
pub const PEER_COST: u8 = 7;
pub const RESURRECT_SPELL_INDEX: usize = 42;
pub const RESURRECT_COST: u8 = 8;
pub const SUMMON_DAEMON_SPELL_INDEX: usize = 43;
pub const GATE_TRAVEL_SPELL_INDEX: usize = 46;
pub const GATE_TRAVEL_COST: u8 = 8;
pub const TIME_STOP_SPELL_INDEX: usize = 47;
pub const TIME_STOP_COST: u8 = 8;
pub const TIME_STOP_DURATION: u8 = 10;
pub const NEGATE_TIME_ACTIVE_EFFECT_TAG: u8 = b'T';

// Combat-side raw damage caps for single-target damage spells per
// `catalogs/spell-list.md` §5. The instant-kill sentinel itself lives in
// `combat_actor` because the damage helpers there compare it as `i16`.
pub const MAGIC_MISSILE_SPELL_INDEX: usize = 1;
pub const MAGIC_MISSILE_RAW_DAMAGE_MAX: u8 = 16;
pub const FIREBALL_SPELL_INDEX: usize = 13;
pub const FIREBALL_RAW_DAMAGE_MAX: u8 = 30;
pub const KILL_SPELL_INDEX: usize = 37;
/// Fire-Field per-actor raw damage roll cap per `combat.md` §11. Energy
/// Field supplies raw zero to the same damage path; that case has no cap.
pub const FIRE_FIELD_RAW_DAMAGE_MAX: u8 = 21;

/// Inclusive town/world door tile-id range per `catalogs/tile-catalog.md` §6:
/// indices `96..=103` are the door family used by O-Open / J-Jimmy / magic
/// Open. Open variants written by the O command live in this range alongside
/// the closed forms.
pub const TOWN_DOOR_TILE_FIRST: u8 = 96;
pub const TOWN_DOOR_TILE_LAST: u8 = 103;
/// Inclusive town stair tile-id range per `catalogs/tile-catalog.md` §6:
/// `0xC4..=0xC7` is the facing-sensitive stairway family whose low two bits
/// encode movement-wrapper-normalised facing.
pub const TOWN_STAIR_TILE_FIRST: u8 = 0xC4;
pub const TOWN_STAIR_TILE_LAST: u8 = 0xC7;
/// Town chair trigger tile per `catalogs/tile-catalog.md` §6.
pub const TOWN_CHAIR_TILE: u8 = 0x8C;
/// NPC floor-link marker tiles consumed by the schedule pathfinder per
/// `catalogs/tile-catalog.md` §6.
pub const NPC_FLOOR_LINK_TILE_A: u8 = 0xC8;
pub const NPC_FLOOR_LINK_TILE_B: u8 = 0xC9;
pub const SPELL_CODES: [&str; SPELL_COUNT] = [
    "IL", "GP", "AZ", "AN", "M", "AY", "AS", "ACX", "HR", "IW", "KX", "IMX", "LV", "FV", "FGI",
    "GIN", "GIZ", "IP", "AG", "IS", "GIS", "PU", "DP", "QW", "BIX", "AEP", "EIP", "MV", "IZ", "RT",
    "IPVY", "AQW", "AI", "AWY", "AEX", "BRX", "LS", "CX", "IQX", "IQW", "HIN", "CIQ", "CIM", "CKX",
    "CGIV", "FHI", "PRV", "AT",
];
pub const SPELL_RECIPE_MASKS: [u8; SPELL_COUNT] = [
    0x80, 0x84, 0x60, 0x60, 0x50, 0x28, 0x88, 0xa0, 0x88, 0x02, 0x11, 0x61, 0x81, 0x84, 0x94, 0x16,
    0x54, 0x18, 0x84, 0xe0, 0x15, 0x18, 0x18, 0x12, 0x98, 0xa8, 0x88, 0x51, 0x52, 0x89, 0x89, 0x03,
    0xa1, 0x81, 0x16, 0x93, 0x0b, 0x06, 0xd9, 0x03, 0x8a, 0x23, 0xf9, 0x39, 0x83, 0x89, 0x85, 0x29,
];
pub const SPELL_SCENE_DUNGEON: u8 = 0x01;
pub const SPELL_SCENE_COMBAT: u8 = 0x02;
pub const SPELL_SCENE_INDOOR: u8 = 0x04;
pub const SPELL_SCENE_OVERWORLD: u8 = 0x08;
pub const SPELL_SCENE_MASKS: [u8; SPELL_COUNT] = [
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_INDOOR,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON,
    SPELL_SCENE_DUNGEON,
    SPELL_SCENE_DUNGEON,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_INDOOR,
    SPELL_SCENE_COMBAT | SPELL_SCENE_INDOOR,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_COMBAT,
    SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
    SPELL_SCENE_COMBAT | SPELL_SCENE_DUNGEON | SPELL_SCENE_INDOOR | SPELL_SCENE_OVERWORLD,
];
pub const MOONSTONE_SLOT_COUNT: usize = 8;
pub const MOONSTONE_INVALID_SCENE: u8 = 0xff;
pub const FIRST_PLAYABLE_MOONSTONE_PICKUP_TILE: u8 = 10;
pub const MOONSTONE_PICKUP_AUX3: u8 = b'M';
pub const DEFAULT_FOOD_STOCK: u16 = 63;
pub const DEFAULT_GOLD_STOCK: u16 = 150;
pub const DEFAULT_KEY_STOCK: u8 = 2;
pub const DEFAULT_GEM_STOCK: u8 = 0;
pub const DEFAULT_CLIMBING_GEAR: u8 = 0;
pub const DEFAULT_CLIMB_STAT: u8 = 30;
pub const SHIP_BADLY_DAMAGED_WARNING: &str = "DANGER: SHIP BADLY DAMAGED!";
pub const SHIP_NO_SKIFFS_WARNING: &str = "WARNING: NO SKIFFS ON BOARD!";
pub const FIRST_PLAYABLE_FOOT_TRANSPORT_MARKER: u8 = 28;
pub const FIRST_PLAYABLE_FULL_SHIP_HULL: u8 = 77;
pub const FIRST_PLAYABLE_HORSE_TILE: u8 = 160;
pub const FIRST_PLAYABLE_FRIGATE_TILE: u8 = 168;
pub const FIRST_PLAYABLE_SKIFF_TILE: u8 = 176;
pub const FIRST_PLAYABLE_MAGIC_CARPET_TILE: u8 = 184;
pub const FIRST_PLAYABLE_BALLOON_TILE: u8 = 188;
pub const DEFAULT_PARTY_HP: u16 = 60;
pub const DEFAULT_PARTY_MAX_HP: u16 = 150;
pub const REST_WATCH_TICKS_PER_HOUR: u8 = 3;
pub const REST_WATCH_MINUTES_PER_TICK: u8 = 20;
pub const TOWN_REST_TICKS_PER_HOUR: u8 = 6;
pub const TOWN_REST_MINUTES_PER_TICK: u8 = 10;
pub const TOWN_REST_INITIAL_SCHEDULE_BURST_TICKS: u8 = 16;
pub const REST_MANA_CAP: u8 = 99;
pub const DEFAULT_TORCH_STOCK: u8 = 4;
pub const SURFACE_TORCH_DURATION: u8 = 240;
pub const DUNGEON_TORCH_DURATION_MIN: u8 = 112;
pub const FULL_DAYLIGHT: u8 = 50;
pub const FULL_DARKNESS: u8 = 2;
pub const DAYLIGHT_SENTINEL_MIN: u8 = 51;
pub const TORCH_LIGHT_FLOOR: u8 = 18;
pub const LIGHT_SPELL_FLOOR: u8 = 10;
pub const DAWN_DUSK_LIGHT: [u8; 6] = [2, 5, 10, 20, 34, 49];
pub const OOL_RECORD_LEN: usize = 8;
pub const OOL_SLOTS: usize = 32;
pub const OOL_PLANE_LEN: usize = OOL_RECORD_LEN * OOL_SLOTS;
pub const SAVED_OOL_LEN: usize = OOL_PLANE_LEN * 2;
pub const DUNGEON_DAT_LEN: usize = 4096;
pub const DUNGEON_RECORD_LEN: usize = 512;
pub const DUNGEON_LEVEL_LEN: usize = 64;
pub const DUNGEON_SIDE: usize = 8;
pub const DUNGEON_VIEW_DEPTH: usize = 4;
pub const DUNGEON_GEM_VIEW_RADIUS: isize = 5;
pub const WORLD_SIDE: usize = 256;
pub const WORLD_CELLS: usize = WORLD_SIDE * WORLD_SIDE;
pub const UNDER_DAT_LEN: usize = WORLD_CELLS;
pub const BRIT_DAT_LEN: usize = 52_480;
pub const CHUNK_SIDE: usize = 16;
pub const CHUNK_BYTES: usize = CHUNK_SIDE * CHUNK_SIDE;
pub const WORLD_CHUNKS_PER_SIDE: usize = WORLD_SIDE / CHUNK_SIDE;
pub const WORLD_CHUNK_COUNT: usize = WORLD_CHUNKS_PER_SIDE * WORLD_CHUNKS_PER_SIDE;
pub const BRIT_STORED_CHUNKS: usize = BRIT_DAT_LEN / CHUNK_BYTES;
pub const BRIT_WATER_SENTINEL: u8 = 0xff;
pub const BRIT_DEEP_WATER_TILE: u8 = 1;
pub const NPC_PATH_QUEUE_LIMIT: usize = 32;
pub const ACTIVE_OBJECT_NEIGHBORHOOD_RADIUS: usize = 32;
pub const PLAYER_NPC_SLOT: usize = OOL_SLOTS - 1;
pub const PLAYER_NPC_SENTINEL_TYPE: u8 = 0x7f;
pub const PLAYER_NPC_DIALOG_ID: u8 = 0;
pub const LOCATION_MARKER_CLEANUP_TILE: u8 = 16;

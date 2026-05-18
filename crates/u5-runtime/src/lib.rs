//! Game runtime for the Ultima V clean-room implementation.
//!
//! This crate owns the simulation, parsers, and rules. It has no UI
//! dependencies. UI shells (`u5-tui`, `u5-bevy`) consume its public API.

pub mod active_object_io;
pub mod animation;
pub mod blackthorn;
pub mod blackthorn_session;
pub mod boot;
pub mod character_record;
pub mod chargen;
pub mod clock;
pub mod codex_challenge;
pub mod combat_actor;
pub mod combat_arena;
pub mod combat_driver;
pub mod combat_frame;
pub mod combat_scenario;
pub mod combat_setup;
pub mod combat_stats;
pub mod commands;
pub mod constants;
pub mod containers;
pub mod conversation_session;
pub mod directed_step;
pub mod direction;
pub mod dungeon_tables;
pub mod dungeon_tables_io;
pub mod dungeon_tables_io_movement;
pub mod end_io;
pub mod endgame;
pub mod endgame_cinematic;
pub mod endmsg_io;
pub mod equipment;
pub mod fonts_io;
pub mod graphics;
pub mod graphics_io;
pub mod hidden_treasures;
pub mod inline_parsers;
pub mod input_codes;
pub mod input_dispatch;
pub mod intro;
pub mod intro_menu;
pub mod jimmy;
pub mod karma;
pub mod lighting;
pub mod lord_british_camp;
pub mod lzw;
pub mod magic;
pub mod main_loop;
pub mod map_decoders;
pub mod map_io;
pub mod menu_dispatch;
pub mod misc_tables;
pub mod misc_tables_io;
pub mod miscmsg_io;
pub mod moongate;
pub mod npc_runtime;
pub mod party;
pub mod play_options;
pub mod play_state_impl;
pub mod play_state_struct;
pub mod predicates;
pub mod prng;
pub mod pth;
pub mod quest_flags;
pub mod question_io;
pub mod report;
pub mod rest_camp;
pub mod return_to_view;
pub mod save_load;
pub mod scene;
pub mod ship_broadside;
pub mod shop_runtime;
pub mod shop_session;
pub mod shoppe_bark;
pub mod shoppe_records;
pub mod shops;
pub mod shrine_virtue;
pub mod signs_io;
pub mod start_validation;
pub mod stat_arithmetic;
pub mod stats_panel;
pub mod story_io;
pub mod test_fixtures;
pub mod text_wrap;
pub mod tile_classes;
pub mod tile_helpers;
pub mod timing;
pub mod tlk_control_codes;
pub mod tlk_runner;
pub mod town_mode;
pub mod town_tables;
pub mod town_tables_io;
pub mod town_tables_io_movement;
pub mod transport;
pub mod traps;
pub mod u4_transfer;
pub mod u4_transfer_session;
pub mod view_classes;
pub mod visibility;
pub mod wind;
pub mod world_tables;
pub mod world_tables_io;
pub mod world_tables_io_get_pickup;
pub mod world_tables_io_locations;
pub mod z_stats;

pub use active_object_io::*;
pub use animation::{ActiveObject, ActiveShipWind, AnimationClock, PhaseTick};
pub use blackthorn::{
    BLACKTHORN_CAPTIVE_CELL_SCENE, BLACKTHORN_CAPTIVE_CELL_X, BLACKTHORN_CAPTIVE_CELL_Y,
    BLACKTHORN_CHALLENGE_INPUT_LIMIT, BLACKTHORN_CHALLENGE_PROMPT_COUNT,
    BLACKTHORN_CHALLENGE_PROMPT_TABLE, BLACKTHORN_FAILURE_VICTIM_SLOT,
    BLACKTHORN_RESCUE_HANDOFF_SCENE, BLACKTHORN_RESCUE_HANDOFF_X, BLACKTHORN_RESCUE_HANDOFF_Y,
    BLACKTHORN_RESCUE_STANDING_FLOOR, BlackthornCutsceneActor, BlackthornEntryFamily,
    KARMA_DAT_BAND_WIDTH, KarmaDatTier, blackthorn_challenge_answer_matches,
    blackthorn_challenge_prompt, blackthorn_cutscene_actor, blackthorn_rescue_post_print_standing,
    blackthorn_rescue_verdict_record, karma_dat_tier, lord_british_camp_verdict_record,
};
pub use boot::{
    DATA_OVL_FILENAME, DRIVER_SELECTOR_CGA, DRIVER_SELECTOR_EGA, DRIVER_SELECTOR_HERCULES,
    DRIVER_SELECTOR_TANDY, DisplayDriverFamily, GraphicsCapability, INTRO_OVL_FILENAME,
    MachineClass, TANDY_LOW_MEMORY_THRESHOLD_KB, ULTIMA_EXE_FILENAME,
    parse_explicit_driver_selector, resolve_driver_family, tandy_low_memory_downgrades,
};
pub use character_record::{
    CharacterClass, CharacterStatus, RestDurationInput, SAVE_CHARACTER_DEFENSE_BYTE_OFFSET,
    SAVE_CHARACTER_DEXTERITY_OFFSET, SAVE_CHARACTER_EXPERIENCE_OFFSET,
    SAVE_CHARACTER_HP_CURRENT_OFFSET, SAVE_CHARACTER_HP_MAX_OFFSET,
    SAVE_CHARACTER_INTELLIGENCE_OFFSET, SAVE_CHARACTER_LEVEL_OFFSET,
    SAVE_CHARACTER_MAGIC_POINTS_OFFSET, SAVE_CHARACTER_MONTH_COUNTER_OFFSET,
    SAVE_CHARACTER_NAME_LEN_BYTES, SAVE_CHARACTER_NAME_OFFSET, SAVE_CHARACTER_RECORD_LEN,
    SAVE_CHARACTER_STRENGTH_OFFSET, character_class_for_byte, character_status_for_byte,
    rest_cleanup_transitions_to_good, rest_duration_input, rest_with_watch_participates,
    rest_with_watch_recovers_hp, save_character_field_offset, sleep_ambush_restored_status,
    town_rest_temp_sleep_marked,
};
pub use chargen::*;
pub use clock::{
    CHARACTER_MONTH_COUNTER_CAP, CLEANUP_INCREMENT_RECOMPUTE, DAYS_PER_MONTH, DAYS_PER_YEAR,
    GameClock, HOURS_PER_DAY, MINUTES_PER_HOUR, MINUTES_PER_INDOOR_TURN, MINUTES_PER_OUTDOOR_TURN,
    MONTHS_PER_YEAR, PROVISION_DECREMENT_HOURS, SHADOWLORD_HIDEOUT_FIRST, SHADOWLORD_HIDEOUT_LAST,
    SHADOWLORD_HIDEOUT_VANQUISHED, SHADOWLORD_NAME_ASTAROTH, SHADOWLORD_NAME_FAULINEI,
    SHADOWLORD_NAME_NOSFENTOR, SHOP_TIME_OF_DAY_AFTERNOON_END_HOUR,
    SHOP_TIME_OF_DAY_MORNING_END_HOUR, SKY_STRIP_CELL_COUNT,
    SKY_STRIP_FELUCCA_AFTERNOON_BAND_FIRST, SKY_STRIP_FELUCCA_AFTERNOON_BAND_LAST,
    SKY_STRIP_FELUCCA_AFTERNOON_OFFSET, SKY_STRIP_FELUCCA_MORNING_BAND_FIRST,
    SKY_STRIP_FELUCCA_MORNING_BAND_LAST, SKY_STRIP_FELUCCA_MORNING_OFFSET,
    SKY_STRIP_FIXED_HOUR_BAND_FIRST, SKY_STRIP_FIXED_HOUR_BAND_LAST, SKY_STRIP_FIXED_HOUR_OFFSET,
    SKY_STRIP_RENDER_ORDER, SKY_STRIP_TRAMMEL_MORNING_BAND_FIRST,
    SKY_STRIP_TRAMMEL_MORNING_BAND_LAST, SKY_STRIP_TRAMMEL_MORNING_OFFSET,
    SKY_STRIP_TRAMMEL_NIGHT_BAND_FIRST, SKY_STRIP_TRAMMEL_NIGHT_BAND_LAST,
    SKY_STRIP_TRAMMEL_NIGHT_OFFSET, SkyStripMarker, TIMING_TAG_NEGATE_TIME, TIMING_TAG_QUICKNESS,
    TOWN_ARREST_CLEANUP_INCREMENT_MINUTES, TOWN_ARREST_RELEASE_HOUR,
    TOWN_REST_CLEANUP_INCREMENT_MINUTES, age_character_month_counter, all_shadowlords_vanquished,
    apply_timing_tag_increment, display_hour_12h, is_provision_decrement_hour,
    shadowlord_hideout_is_live, shadowlord_hideout_is_vanquished, shadowlord_name_for_slot,
    shadowlord_slot_for_name, shop_time_of_day_word, sky_strip_composed_cells,
    sky_strip_marker_position, sky_strip_renders, town_arrest_release_loop_done,
};
pub use combat_actor::*;
pub use combat_arena::*;
pub use combat_frame::*;
pub use combat_setup::*;
pub use combat_stats::*;
pub use commands::{
    BRITANNIA_CHUNK_MAP_COLUMNS, BRITANNIA_CHUNK_MAP_LOOK_TRIGGER_TILE, BRITANNIA_CHUNK_MAP_ROWS,
    Command, LOCAL_VIEW_CELL_PIXEL_SCALE, LOCAL_VIEW_OVERLAY_SIDE, LocalViewClass, NewOrderOutcome,
    PUSHABLE_CANNON_FLOOR_STAMP, PUSHABLE_GENERIC_FLOOR_STAMP, PushableTileFamily,
    ViewCommandOutcome, WISHING_WELL_WISH_KEYWORDS, YELL_INPUT_MAX_LEN, YELL_NOTHING_SAID_MESSAGE,
    YELL_SAILS_FURLED_MESSAGE, YELL_SAILS_HOISTED_MESSAGE, YellInputContext, command_for_letter,
    local_view_class_for_tile, new_order_outcome, new_order_swap_accepted, pushable_facing_index,
    pushable_oriented_tile, pushable_tile_family, town_fountain_drink_accepts,
    view_command_outcome, wishing_well_wish_accepted,
};
pub use constants::*;
pub use containers::{
    CHEST_CONTENT_ROLL_DIE, CHEST_PRIMARY_POOL_ROW_COUNT, CHEST_PRIMARY_POOL_THRESHOLDS,
    CHEST_SECONDARY_POOL_ATTEMPT_BIAS, CHEST_SECONDARY_POOL_ATTEMPT_DIVISOR,
    CHEST_SECONDARY_POOL_ROW_COUNT, CHEST_SECONDARY_POOL_THRESHOLDS, DUNGEON_CHEST_FOOD_MAX,
    DUNGEON_CHEST_GOLD_DEPTH_MULTIPLIER, DUNGEON_CHEST_ROW_GATE_BIAS,
    DUNGEON_CHEST_ROW_GATE_DEPTH_MULTIPLIER, DUNGEON_CHEST_ROWS, DUNGEON_CHEST_SMALL_MAX,
    DUNGEON_CHEST_SUBTYPE_MAX, DungeonBombSearchOutcome, DungeonChestReward, DungeonChestRow,
    DungeonChestSearchOutcome, DungeonChestTrapTier, INVENTORY_ADD_AMMO_UNITS,
    INVENTORY_ADD_EQUIPMENT_UNITS, InventoryAddClass, RARE_REAGENT_HARVEST_HOUR,
    RARE_REAGENT_HARVEST_MINUTE_BOUND, RARE_REAGENT_HARVEST_QUANTITY_MAX,
    RARE_REAGENT_HARVEST_QUANTITY_MIN, RARE_REAGENT_HARVEST_QUANTITY_SPAN, SearchLocationPrefix,
    SearchTrapVisibility, TABLE_FOOD_TILE_A, TABLE_FOOD_TILE_B, TOWN_CHEST_OPEN_KARMA_DEBIT,
    chest_primary_pool_row_succeeds, chest_secondary_pool_attempts, chest_secondary_pool_threshold,
    dungeon_bomb_search_outcome, dungeon_chest_gold_is_zero_width, dungeon_chest_gold_upper,
    dungeon_chest_row_awarded, dungeon_chest_row_gate_max, dungeon_chest_search_outcome,
    dungeon_chest_trap_tier, equipment_grant_quantity, inventory_add_class,
    inventory_add_class_cap, inventory_add_equipment_units, rare_reagent_harvest_hour_accepted,
    rare_reagent_harvest_quantity, search_location_prefix, search_trap_detection_threshold,
    search_trap_visibility, table_food_get_resulting_tile, town_chest_open_standing,
};
pub use directed_step::{
    Axis, axis_first_choice, directed_step_offsets, terrain_chance_gate_denominator,
    type_bypasses_terrain_chance_gate,
};
pub use direction::Direction;
pub use dungeon_tables::*;
pub use dungeon_tables_io::*;
pub use dungeon_tables_io_movement::*;
pub use end_io::{
    END_DAT_FILE, END_DAT_LEN, END_DAT_WINDOW_COUNT, END_PARAGRAPH_START_MARKER,
    END_SOFT_BREAK_MARKER, EndNarrative, EndNarrativeGroup, EndNarrativeWindow, decode_end_window,
    end_narrative_window, load_end_narrative, parse_end_narrative, require_end_narrative,
};
pub use endgame::*;
pub use endmsg_io::{
    ENDMSG_DAT_LEN, ENDMSG_DAT_RECORDS, EndgameMessages, load_endgame_messages,
    parse_endgame_messages, require_endgame_messages,
};
pub use equipment::*;
#[cfg(test)]
pub use fonts_io::*;
pub use graphics::*;
pub use graphics_io::*;
pub use hidden_treasures::{
    HIDDEN_TREASURE_RECORD_DAILY_CACHE, HIDDEN_TREASURE_RECORD_KEY_NPC_GATED,
    HIDDEN_TREASURE_RECORD_SINGLE_USE_NPC_GATED, HIDDEN_TREASURE_UNDERWORLD_STACK_ARMOUR_STATE,
    HIDDEN_TREASURE_UNDERWORLD_STACK_FIRST, HIDDEN_TREASURE_UNDERWORLD_STACK_FLOOR,
    HIDDEN_TREASURE_UNDERWORLD_STACK_LAST, HIDDEN_TREASURE_UNDERWORLD_STACK_LEN,
    HIDDEN_TREASURE_UNDERWORLD_STACK_WEAPON_STATE, HIDDEN_TREASURE_UNDERWORLD_STACK_X,
    HIDDEN_TREASURE_UNDERWORLD_STACK_Y, HiddenTreasurePickupClass, HiddenTreasureRule,
    hidden_treasure_can_stage, hidden_treasure_record_13_accepts, hidden_treasure_record_14_ready,
    hidden_treasure_record_15_accepts, hidden_treasure_rule, underworld_stack_record,
};
pub use inline_parsers::*;
pub use input_codes::{
    CURSOR_BLINK_BASE_GLYPH, CURSOR_BLINK_MODULUS, CardinalPromptAction,
    DIRECTION_PROMPT_LABEL_EAST, DIRECTION_PROMPT_LABEL_NORTH, DIRECTION_PROMPT_LABEL_PASS,
    DIRECTION_PROMPT_LABEL_SOUTH, DIRECTION_PROMPT_LABEL_WEST, FreeTextInputAction,
    INPUT_CODE_CARDINAL_FIRST, INPUT_CODE_CARDINAL_LAST, INPUT_CODE_DIAGONAL_FIRST,
    INPUT_CODE_DIAGONAL_LAST, INPUT_CODE_EAST, INPUT_CODE_F1, INPUT_CODE_F10,
    INPUT_CODE_FUNCTION_FIRST, INPUT_CODE_FUNCTION_LAST, INPUT_CODE_NORTH, INPUT_CODE_NORTHEAST,
    INPUT_CODE_NORTHWEST, INPUT_CODE_SOUTH, INPUT_CODE_SOUTHEAST, INPUT_CODE_SOUTHWEST,
    INPUT_CODE_WEST, InputByteClass, InputDirection, NumericPromptAction,
    PartyTargetSelectorAction, PartyTargetSelectorResult, SPELL_DIRECTION_PROMPT_PREFIX,
    cardinal_direction_prompt_action, direction_prompt_label, free_text_input_action,
    input_byte_class, input_case_fold, input_code_direction, input_function_key_index,
    input_prompt_mode_active, numeric_prompt_action, numeric_prompt_apply,
    party_target_selector_action, party_target_selector_result,
};
pub use input_dispatch::{PlayInputDisposition, handle_play_key_input};
pub use intro::{
    BRITISH_PTH_PEN_ORIGINS, IntroMenuAction, MISCMAPS_CUTSCENE_RECORD_BYTES,
    MISCMAPS_CUTSCENE_RECORD_COUNT, MISCMAPS_CUTSCENE_ROW_STRIDE, MISCMAPS_CUTSCENE_ROWS,
    MISCMAPS_CUTSCENE_SECTION_BYTES, MISCMAPS_CUTSCENE_SECTION_OFFSET,
    MISCMAPS_CUTSCENE_VISIBLE_COLUMNS, MISCMAPS_DAT_FILE, MISCMAPS_RTV_COMMAND_SECTION_OFFSET,
    MISCMAPS_RTV_STRIP_ROW_STRIDE, MISCMAPS_RTV_STRIP_SECTION_BYTES,
    MISCMAPS_RTV_STRIP_SECTION_OFFSET, RTV_COMMAND_COUNT, RTV_COMMAND_STREAM_BYTES,
    RTV_STRIP_COLUMNS, RTV_STRIP_COUNT, RTV_STRIP_ROWS, TITLE_BIT_INITIAL_PLACEMENTS,
    TITLE_BIT_REMAINING_PLACEMENTS, TITLE_LOWER_BAND_CLEAR_Y, TITLE_SURFACE_HEIGHT,
    TITLE_SURFACE_WIDTH, TITLE_TICK_FRAME_COUNT, TITLE_TICK_FRAME_HEIGHT, TITLE_TICK_FRAME_WIDTH,
    TITLE_TICK_FRAME_X, TITLE_TICK_FRAME_Y, TitleBitAsset, TitleBitPlacement, intro_menu_action,
    title_tick_next_frame,
};
pub use jimmy::{
    DOOR_AUTO_CLOSE_TURNS, DoorAutoCloseTick, JIMMY_CHEST_THRESHOLD_BIAS,
    JIMMY_CHEST_THRESHOLD_DIVISOR, JIMMY_DOOR_DIE_HIGH, JIMMY_DOOR_DIE_LOW,
    JIMMY_DUNGEON_CHEST_DEPTH_MULTIPLIER, JIMMY_NPC_PICKPOCKET_KARMA_REWARD, JIMMY_OBJECT_DIE_HIGH,
    JIMMY_OBJECT_DIE_LOW, MAGIC_UNLOCK_CLOSED_WOODEN_A, MAGIC_UNLOCK_CLOSED_WOODEN_B,
    MAGIC_UNLOCK_OPEN_WOODEN_A, MAGIC_UNLOCK_OPEN_WOODEN_B, OUTDOOR_KLIMB_FALL_DAMAGE_MAX,
    OUTDOOR_KLIMB_FALL_DAMAGE_MIN, OUTDOOR_KLIMB_FALL_DIE_HIGH, OUTDOOR_KLIMB_FALL_DIE_LOW,
    OverworldKlimbEntryGate, door_auto_close_tick, dungeon_chest_jimmy_succeeds,
    dungeon_chest_jimmy_threshold, jimmy_door_succeeds, magic_unlock_door_rewrite,
    object_chest_jimmy_succeeds, object_chest_jimmy_threshold, outdoor_klimb_member_falls,
    overworld_klimb_entry_gate,
};
pub use karma::{
    CODEX_TURNIN_STAT_CAP, CODEX_TURNIN_STAT_INCREMENT, KARMA_CROP_OR_TABLE_FOOD_DEBIT,
    KARMA_HELPED_NPC_THANK_YOU_GAIN, KARMA_TOLL_MILESTONE_GAIN,
    KARMA_TOLL_MILESTONE_ZERO_GOLD_BONUS, KarmaAction, RESURRECTION_PENALTY_PERCENT_DIVISOR,
    RESURRECTION_PENALTY_SKIP_THRESHOLD, SHRINE_MANTRA_INPUT_LIMIT, apply_karma_action,
    codex_turnin_stat_reward, resurrection_penalty_skipped, resurrection_scaled_xp,
    shrine_mantra_for,
};
pub use lighting::{
    DAWN_DUSK_LAST_INDEX, DAWN_DUSK_STEP_MINUTES, DAWN_HOUR, DUNGEON_TORCH_INCREMENT_MAX,
    DUNGEON_TORCH_INCREMENT_MIN, DUSK_HOUR, GREAT_LIGHT_SPELL_DURATION, LAST_IN_HOUR_MINUTE,
    LIGHT_SPELL_DURATION, LightDecayCadence, MOONGATE_ANIMATOR_DAYTIME_THRESHOLD,
    OVERWORLD_UNDERFOOT_BLACKOUT_EXEMPT_TAG, OVERWORLD_UNDERFOOT_BLACKOUT_TILE,
    ambient_is_sentinel, apply_personal_light, daylight_base_value, decay_light_counter,
    dungeon_blackout, ignite_torch_dungeon, ignite_torch_surface, light_counter_increment,
    light_counter_spend_with_tag, moongate_animator_render_eligible,
    overworld_underfoot_forces_dark,
};
pub use lord_british_camp::{
    LORD_BRITISH_CAMP_EVENT_ROLL_BOUND, LORD_BRITISH_CAMP_EVENT_THRESHOLD,
    LORD_BRITISH_CAMP_STAT_REWARD_CAP, LordBritishCampStatReward, level_for_experience,
    lord_british_camp_event_hp_for_level, lord_british_camp_event_triggered,
    lord_british_camp_refreshed_mana, lord_british_camp_stat_reward,
};
pub use lzw::*;
pub use magic::{
    ActiveEffectTag, COMBAT_FIELD_KIND_ENERGY, COMBAT_FIELD_KIND_FIRE, COMBAT_FIELD_KIND_POISON,
    COMBAT_FIELD_KIND_SLEEP, CONJURE_OUTCOME_COUNT, CastGateOutcome, ConjureSummon,
    DIRECTED_WIND_MAX_CELLS, DirectedWindSpell, FieldSpellKind, MMIX_COMBAT_REFUSAL_MESSAGE,
    MMIX_EMPTY_SELECTION_MESSAGE, MMIX_INSUFFICIENT_REAGENTS_MESSAGE, MMIX_MIXING_MESSAGE,
    MMIX_NO_REAGENTS_OWNED_MESSAGE, MMIX_QUANTITY_PROMPT_DIGITS, MMIX_QUANTITY_PROMPT_MAX,
    MMIX_QUANTITY_PROMPT_MESSAGE, MMIX_SPELL_PROMPT_MESSAGE, RUNE_SYLLABLE_VOCABULARY,
    SPELL_SCENE_BIT_COMBAT, SPELL_SCENE_BIT_DUNGEON, SPELL_SCENE_BIT_INDOOR,
    SPELL_SCENE_BIT_OVERWORLD, SPELL_SELECTOR_IGNORED_LETTERS, SPELL_SELECTOR_MAX_LEN,
    SpellSceneClass, active_effect_tag_for_byte, cast_dispatcher_gate, combat_interference_blocks,
    conjure_summon_for_roll, field_spell_kind_for_dungeon_byte, heal_spell_amount_from_raw_roll_u8,
    is_resident_rune_syllable, sextant_coordinate_letters, spell_allowed_in_scene,
    spell_charge_add_capped, spell_circle_for, spell_combat_field_kind, spell_common_name,
    spell_field_placement_byte, spell_indoor_absorbs, spell_mana_cost, spell_min_caster_level,
    spell_rune_name, spell_selector_is_ignored,
};
pub use main_loop::{
    CommandDispatchStatus, DUNGEON_DOOM_SCENE_BYTE, DUNGEON_ENTRY_SURFACE_X,
    DUNGEON_ENTRY_SURFACE_Y, DUNGEON_ENTRY_SURFACE_Z, DUNGEON_ENTRY_UNDERWORLD_X,
    DUNGEON_ENTRY_UNDERWORLD_Y, DUNGEON_ENTRY_UNDERWORLD_Z, DUNGEON_FACING_EAST,
    DUNGEON_FACING_NORTH, DUNGEON_FACING_SOUTH, DUNGEON_FACING_WEST, DungeonEntrySeed,
    DungeonMovementAction, OuterLoopFlags, SCENE_COMBAT_TEMPORARY, SCENE_DUNGEON_FAMILY_FIRST,
    SCENE_DUNGEON_FAMILY_LAST, SCENE_DUNGEON_NAMED_FIRST, SCENE_DUNGEON_NAMED_LAST,
    SCENE_INTRO_FIRST, SCENE_INTRO_LAST, SCENE_OVERWORLD, SCENE_TOWN_FAMILY_FIRST,
    SCENE_TOWN_FAMILY_LAST, SceneRoute, WorldTickPath, dungeon_entry_seed,
    dungeon_facing_back_delta, dungeon_facing_forward_delta, dungeon_facing_left_delta,
    dungeon_facing_right_delta, dungeon_facing_turn_around, dungeon_facing_turn_left,
    dungeon_facing_turn_right, dungeon_movement_action, dungeon_record_index,
    dungeon_resident_name, dungeon_scene_for_word_of_power, dungeon_word_of_power,
    mode_minute_increment, save_scene_byte_normalised, scene_route, world_tick_path,
};
pub use map_decoders::*;
pub use map_io::*;
pub use misc_tables::*;
pub use misc_tables_io::*;
pub use miscmsg_io::{
    MISCMSG_DAT_FILE, MISCMSG_DAT_LEN, MISCMSG_DAT_RECORDS, MISCMSG_URN_CODEX_RANGE, MiscMessages,
    MiscMsgFamily, TileGlyphDigraph, load_misc_messages, miscmsg_family, parse_misc_messages,
    render_miscmsg_tile_glyph_text, tile_glyph_digraph,
};
pub use moongate::{
    MOONSTONE_BURIAL_BAND_FIRST, MOONSTONE_BURIAL_BAND_LAST, MOONSTONE_BURIAL_TILE_EXTRA_A,
    MOONSTONE_BURIAL_TILE_EXTRA_B, MOONSTONE_GATE_INVALID_SCENE, NARRATIVE_GATE_X,
    NARRATIVE_GATE_Y, NATURAL_MOONGATE_LIVE_TILE, NATURAL_MOONGATE_UNDERLYING_TILE,
    NaturalMoongateCounterStep, SURFACE_CHASM_X, SURFACE_CHASM_Y, WORLD_PLANE_FALL_DAMAGE_MAX,
    is_surface_chasm_cell, moonstone_burial_tile_accepted, natural_moongate_advance_counter,
    natural_moongate_cached_glyph_slot, natural_moongate_counter_step,
    natural_moongate_dispatches_meditate, natural_moongate_slot_eligible,
};
pub use npc_runtime::{
    DoorTracker, LocationMarkers, NPC_DIALOG_ID_HIGH_FALLBACK, NPC_DIALOG_ID_HIGH_FIRST,
    NPC_DIALOG_ID_HIGH_LAST, NPC_DIALOG_ID_NONE, NPC_DIALOG_ID_TLK_SENTINEL,
    NPC_DYNAMIC_OBSTACLE_MANHATTAN_RADIUS, NPC_FLOOR_LINK_TILE_C8, NPC_FLOOR_LINK_TILE_C9,
    NPC_HIDDEN_SPRITE_TILE, NPC_PATH_DIR_EAST, NPC_PATH_DIR_NORTH, NPC_PATH_DIR_SOUTH,
    NPC_PATH_DIR_WEST, NPC_PATHFIND_DIRECTION_SHIFT, NPC_PATHFIND_QUEUE_CAPACITY,
    NPC_PATHFIND_START_SEED, NPC_PATHFIND_WORKSPACE_LEN, NPC_PATHFIND_WORKSPACE_SIDE,
    NPC_RUNTIME_DESCRIPTOR_BYTES, NPC_STATE_ASCEND_TOWARD_TARGET, NPC_STATE_CLIMB_DOWN_OFF_FLOOR,
    NPC_STATE_CLIMB_UP_OFF_FLOOR, NPC_STATE_DESCEND_TOWARD_TARGET, NPC_STATE_EMPTY, NPC_STATE_IDLE,
    NPC_STATE_INPLANE_MOVE, NPC_STATE_PARKED_OFF_FLOOR, NPC_STATE_REPLAY_QUEUE,
    NPC_STUCK_REPLAN_THRESHOLD, NPC_TYPE_DEFAULT_HUMAN_SPRITE, NPC_TYPE_EMPTY,
    NPC_TYPE_RUNTIME_PLAYER_MIRROR, NpcAiBehavior, NpcDialogIdKind, NpcLinkAction,
    NpcScheduleState, NpcShopTrigger, NpcTypeByteClass, RuntimeNpc, npc_ai_behavior,
    npc_dialog_id_kind, npc_dynamic_obstacle_blocks, npc_hidden_sprite_slot, npc_link_action,
    npc_path_direction_offset, npc_path_direction_opposite, npc_path_tile_open,
    npc_pathfind_visit_stamp, npc_schedule_state_classify, npc_schedule_state_for_floor_transition,
    npc_shop_trigger, npc_state_off_floor_or_empty, npc_stuck_counter_forces_replan,
    npc_type_byte_class, npc_type_byte_occupied, schedule_floor_state,
};
pub use party::{
    Area, AvatarStats, LEVEL_FROM_EXPERIENCE_BASE_DIVISOR, LEVEL_FROM_EXPERIENCE_HALVING_STEP,
    MoonstoneGateSlot, POTION_VARIATION_FORCED_ORANGE_ROLL, POTION_VARIATION_RANDOM_INDEX_MASK,
    POTION_VARIATION_ROLL_MASK, POTION_VARIATION_SELECTED_THRESHOLD, PartyMember, Player,
    RESURRECTION_MAX_HP_PER_LEVEL, RESURRECTION_REBUILT_CURRENT_HP, class_refreshed_mana,
    default_party, default_party_experience, default_party_intelligence, default_party_names,
    default_party_stay_counters, heal_spell_amount_from_raw_roll, increase_capped_stat,
    party_member_unavailable_message, party_name_to_string, party_status_name,
    potion_effect_index_after_variation, potion_label, recompute_level_from_experience,
    resurrection_adjusted_experience, resurrection_max_hp_for_level,
};
pub use play_options::*;
pub use play_state_struct::{
    PlayState, ViewOverlay, ViewOverlayKind, WorldOverlayCache, WorldReturn,
};
pub use predicates::*;
pub use prng::*;
pub use pth::{
    BRITISH_PTH_LEN, BRITISH_PTH_SEGMENT_COUNT, PTH_BYTE_SIGN_X, PTH_BYTE_SIGN_Y,
    PTH_NIBBLE_MAGNITUDE_MASK, PTH_PEN_DOWN_MAX_MAGNITUDE, PenStroke, pth_decode_byte,
};
pub use quest_flags::{
    CONVERSATION_CLEANUP_GOLD_DEBIT_MAX, CONVERSATION_CLEANUP_GOLD_DEBIT_MIN,
    CONVERSATION_CLEANUP_SENTINEL_ALLOW, ConversationCleanupReconciliation,
    ConversationLetterAction, ConversationPassword, QUEST_GRAPH_NODE_CLASSES, QuestGraphNodeClass,
    conversation_cleanup_gold_debit_amount, conversation_cleanup_reconciliation,
    conversation_cleanup_runs_warning, conversation_letter_action, conversation_password,
    tlk_scene_branch_is_set, tlk_scene_branch_mask, tlk_scene_branch_set,
};
pub use question_io::{
    QUESTION_DAT_DILEMMA_COUNT, QUESTION_DAT_FILE, QUESTION_DAT_FIRST_DILEMMA_RECORD,
    QUESTION_DAT_LEN, QUESTION_DAT_RECORDS, QUESTION_PARAGRAPH_START_MARKER,
    QUESTION_SOFT_BREAK_MARKER, QuestionRecords, load_question_records, parse_question_records,
    question_dat_dilemma_record_for_pair,
};
pub use report::run_report;
pub use return_to_view::{
    RTV_ACTOR_SLOTS, RTV_CLOSE_EFFECT_FINAL_TILE, RTV_EFFECT_SENTINEL_TILE,
    RTV_OPEN_EFFECT_FINAL_TILE, RTV_PREVIEW_CELLS, RTV_PREVIEW_SIDE, RTV_STRIP_RECORD_BYTES,
    RTV_STRIP_TILE_COUNT, RTV_STRIP_VISIBLE_COLUMNS, RTV_STRIP_VISIBLE_ROWS,
    RTV_TEMPORARY_ACTOR_TILE, ReturnToViewActor, ReturnToViewAssets, ReturnToViewCommand,
    ReturnToViewControl, ReturnToViewMapStrips, ReturnToViewPreviewReport, ReturnToViewPreviewRun,
    ReturnToViewPreviewState, ReturnToViewScript, load_return_to_view_assets,
    load_return_to_view_map_strips, load_return_to_view_script, parse_return_to_view_commands,
    parse_return_to_view_map_strips, parse_return_to_view_map_strips_file,
    parse_return_to_view_script_file, render_return_to_view_preview_viewport,
    return_to_view_command_histogram, return_to_view_command_name,
    run_return_to_view_preview_state_until_restart, run_return_to_view_preview_until_restart,
    summarize_return_to_view_preview, summarize_return_to_view_script,
};
pub use save_load::*;
pub use scene::{
    DungeonPresentationFlavour, DungeonScene, FIRST_DUNGEON_SCENE_BYTE, Family,
    LAST_DUNGEON_SCENE_BYTE, PlayTarget, Scene, WorldPlane,
};
pub use ship_broadside::{
    SHIP_BROADSIDE_DAMAGE_MAX, SHIP_BROADSIDE_DAMAGE_MIN, SHIP_BROADSIDE_DEPLETION_BYTE_OFFSET,
    SHIP_BROADSIDE_RANGE_CELLS, ship_broadside_apply_damage, ship_broadside_direction_accepted,
};
pub use shops::*;
pub use shrine_virtue::{
    CodexUrnReadOutcome, ShrineMeditationOutcome, ShrineQuestState, ShrineVirtue,
    all_virtues_complete, read_codex_urn, shrine_meditation_outcome,
};
pub use signs_io::{
    SIGN_BODY_CHARACTER_MASK, SIGN_BODY_END_OF_RECORD, SIGN_BODY_MACRO_FIRST, SIGN_BODY_MACRO_LAST,
    SIGN_BODY_PAUSE_FOR_KEY, SIGN_BODY_SEPARATOR_GLYPH_A, SIGN_BODY_SEPARATOR_GLYPH_B,
    SIGNS_DAT_ALIAS_BRIDGE_LEN, SIGNS_DAT_FILE, SIGNS_DAT_RECORD_HEADER_LEN,
    SIGNS_DAT_SCENE_DIRECTORY_BYTES, SIGNS_DAT_SCENE_DIRECTORY_SLOTS, SignBodyByteKind, SignRecord,
    decode_sign_payload, find_sign, load_sign_records, parse_sign_records, sign_body_byte_kind,
};
pub use start_validation::*;
pub use stat_arithmetic::{capped_add_u8, capped_add_word, floor_sub_u8, floor_sub_word};
pub use stats_panel::{
    STATS_PANEL_PARTY_ROWS, STATS_PANEL_WIDTH, render_stats_panel,
    stats_panel_active_cursor_visible,
};
pub use story_io::{
    INTRO_AUTO_OPENING_STEP, INTRO_INLINE_DOORWAY_STEP, INTRO_STEP_0_TRANSITION_STRIPS,
    INTRO_STEP_1_EXTRA_ART_X, INTRO_STEP_1_EXTRA_ART_Y, INTRO_STEP_1_EXTRA_SUBIMAGE,
    INTRO_STEP_1_RECT_TRANSITION, INTRO_STEP_6_EXTRA_ART_X, INTRO_STEP_6_EXTRA_ART_Y,
    INTRO_STEP_6_EXTRA_SUBIMAGE, INTRO_STEP_7_TRANSITION_STRIPS, INTRO_STEP_14_TRANSITION_STRIPS,
    INTRO_STORY_STEP_COUNT, INTRO_STORY6_SECONDARY_PASS_STEPS, INTRO_STORY6_SECONDARY_Y_DELTA,
    INTRO_TRANSITION_STRIP_STEPS, IntroStoryArtPlacement, STORY_DAT_FILE, STORY_DAT_LEN,
    STORY_DAT_RECORDS, STORY_HARD_NEWLINE_MARKER, STORY_PARAGRAPH_START_MARKER,
    STORY_RECORD_END_MARKER, STORY_SOFT_BREAK_MARKER, StoryRecords, StoryTextMarker,
    intro_step_has_story6_secondary_pass, intro_step_has_transition_strip,
    intro_step_transition_strips, intro_story_art_file_for_step,
    intro_story_art_placement_for_step, intro_story_step_waits_for_input,
    intro_story6_secondary_subimage, load_story_records, parse_story_records, story_text_marker,
};
pub use text_wrap::{
    EmitterByteKind, ParagraphByteKind, ProportionalRendererByteKind, TEXT_COLOR_BACKGROUND_SHIFT,
    TEXT_COLOR_FOREGROUND_MASK, TEXT_CTRL_CENTRE_OFF, TEXT_CTRL_CENTRE_ON, TEXT_CTRL_CLEAR_WINDOW,
    TEXT_CTRL_INVERSE_TOGGLE, TEXT_CTRL_RANGE_FIRST, TEXT_CTRL_RANGE_LAST,
    TEXT_CTRL_UNDERLINE_TOGGLE, TEXT_SCREEN_COLUMNS, TEXT_SCREEN_ROWS, TEXT_WINDOW_COUNT,
    TEXT_WINDOW_DEFAULT_ACTIVE_INDEX, TEXT_WINDOW_DEFAULT_BACKGROUND,
    TEXT_WINDOW_DEFAULT_FOREGROUND, TextControlByte, WRAP_MIN_LINE_BUFFER, WrapByteKind,
    WrappedLine, paragraph_byte_kind, proportional_renderer_byte_kind, text_color_background,
    text_color_foreground, text_control_byte, text_emitter_byte_kind,
    text_window_centred_start_column, text_window_clamp_rectangle, text_window_default_color_byte,
    text_window_inner_width, wrap_byte_kind, wrap_text,
};
pub use tile_classes::{
    TILE_BARRIER_FIRST, TILE_BARRIER_LAST, TILE_DECORATION_FIRST, TILE_DECORATION_LAST,
    TILE_DOOR_FIRST, TILE_DOOR_LAST, TILE_FURNITURE_FIRST, TILE_FURNITURE_LAST, TILE_NPC_FIRST,
    TILE_NPC_LAST, TILE_PATH_FIRST, TILE_PATH_LAST, TILE_SPECIAL_FIRST, TILE_SPECIAL_LAST,
    TILE_TERRAIN_FIRST, TILE_TERRAIN_LAST, TILE_VEHICLE_ART_FIRST, TILE_VEHICLE_ART_LAST,
    TILE_VEHICLE_FIRST, TILE_VEHICLE_LAST, TILE_WALL_FIRST, TILE_WALL_LAST, TILE_WATER_FIRST,
    TILE_WATER_LAST, TileAnimationFamily, TileClass, TileSuperCategory, coarse_tile_class,
    tile_animation_cycle_length, tile_animation_family, tile_super_category,
};
pub use tile_helpers::*;
pub use timing::{DungeonFieldEffect, SaveTemplateSource, TimingStatusTag};
pub use tlk_control_codes::{
    CASTLE_TLK_NPCS, COMMON_WORD_DICTIONARY_ENTRIES, COMMON_WORD_DICTIONARY_NUL_SENTINELS,
    CONVERSATION_CLEANUP_RESOURCE_SIGNAL_COUNT, CONVERSATION_CLEANUP_SECONDARY_SIGNAL_COUNT,
    CONVERSATION_SHARED_NO_SLOT_SENTINEL, DWELLING_TLK_NPCS, KEEP_TLK_NPCS,
    QUEST_PASSWORD_OPPRESSION, QUEST_PASSWORD_RESISTANCE, RESERVED_KEYWORD_FUNCTIONAL_COUNT,
    RESERVED_KEYWORD_REBUKE_COUNT, RESERVED_KEYWORD_TABLE_ENTRIES, ReservedKeywordEffect,
    SHOPPE_PHRASE_TOKEN_FIRST, SHOPPE_PHRASE_TOKEN_LAST, TALK_NO_RESPONSE_MESSAGE,
    TALK_NOBODY_HERE_MESSAGE, TALK_SLEEPING_MESSAGE, TLK_BLOB_FIXED_WINDOW,
    TLK_CODE_ACTION_DISPATCH, TLK_CODE_ASK_PARTY_NAME, TLK_CODE_ASK_WHO, TLK_CODE_CURSE_CHECK,
    TLK_CODE_END_OF_RESPONSE, TLK_CODE_END_STREAM, TLK_CODE_GOLD_PAYMENT,
    TLK_CODE_GOTO_LABEL_FIRST, TLK_CODE_GOTO_LABEL_LAST, TLK_CODE_IF_ELSE, TLK_CODE_IF_ELSE_ALT,
    TLK_CODE_LABEL_RECORD, TLK_CODE_LITERAL_NEWLINE, TLK_CODE_PANEL_NEWLINE, TLK_CODE_PAUSE,
    TLK_CODE_PRINT_AVATAR_NAME, TLK_CODE_PROTECT_RUN, TLK_CODE_SET_FLAG, TLK_CODE_WAIT_KEY,
    TLK_CONTROL_CODE_FIRST, TLK_CONTROL_CODE_LAST, TLK_DICTIONARY_TOKEN_FIRST,
    TLK_DICTIONARY_TOKEN_LAST, TLK_DOUBLE_QUOTE_ENCODED, TLK_EMPTY_INPUT_BYE_MESSAGE,
    TLK_GENERIC_SIGNAL_COUNT, TLK_GOLD_PAYMENT_ARGUMENT_BYTES, TLK_HEADER_ENTRY_LEN,
    TLK_HEADER_FIXED_READ, TLK_IF_ELSE_ALT_ARGUMENT_BYTES, TLK_INPUT_MAX_LEN, TLK_KEYWORD_PROMPT,
    TLK_LABEL_BYTE_COUNT, TLK_LABEL_FIRST, TLK_LABEL_LAST, TLK_LEADING_ENTRY_COUNT,
    TLK_NO_KEYWORD_MATCH_MESSAGE, TLK_ONE_BYTE_INTRODUCER_ARGUMENT_BYTES, TLK_PRINTABLE_TEXT_FIRST,
    TLK_PRINTABLE_TEXT_LAST, TLK_SENTINEL_NPC_ID, TLK_TEXT_XOR_MASK, TOWNE_TLK_NPCS, TalkRefusal,
    TlkActionDispatchVerb, TlkByteKind, TlkByteRunnerClass, TlkFileClass, TlkLeadingEntry,
    TlkPlayerInputKind, TlkPrintMaskState, classify_tlk_byte,
    conversation_cleanup_gold_debit_from_seed, is_tlk_label_byte, reserved_keyword_effect,
    shoppe_dictionary_index, talk_liveness_refusal, tlk_action_dispatch_is_signal_flag,
    tlk_action_dispatch_verb, tlk_ask_party_name_match, tlk_byte_runner_class, tlk_class_for_scene,
    tlk_dictionary_index, tlk_gold_payment_amount, tlk_if_else_alt_branches,
    tlk_introducer_argument_count, tlk_keyword_matches, tlk_label_index, tlk_leading_entry_index,
    tlk_player_input_kind,
};
pub use town_mode::{
    CASTLE_NPC_FILENAME, CASTLE_TLK_FILENAME, DWELLING_NPC_FILENAME, DWELLING_TLK_FILENAME,
    KEEP_NPC_FILENAME, KEEP_TLK_FILENAME, LOCATION_DAT_BLOCK_LEN, LOCATION_DAT_BLOCKS_PER_FILE,
    LOCATION_DAT_FILE_LEN, LOCATION_DAT_FLOOR_PAGE_LEN, LOCATION_DAT_FLOOR_PAGES_PER_BLOCK,
    NPC_DIALOG_ARRAY_LEN, NPC_DIALOG_ARRAY_OFFSET, NPC_EFFECTIVE_SLOTS_PER_SUB_MAP,
    NPC_FILE_CLASS_COUNT, NPC_FILE_LEN, NPC_SCHEDULE_AI_OFFSET, NPC_SCHEDULE_ARRAY_LEN,
    NPC_SCHEDULE_RECORD_LEN, NPC_SCHEDULE_TIME_BOUNDARY_COUNT, NPC_SCHEDULE_TIME_OFFSET,
    NPC_SCHEDULE_WAYPOINT_COUNT, NPC_SCHEDULE_X_OFFSET, NPC_SCHEDULE_Y_OFFSET,
    NPC_SCHEDULE_Z_OFFSET, NPC_SENTINEL_SLOT, NPC_SLOTS_PER_SUB_MAP, NPC_SUB_MAP_LEN,
    NPC_SUB_MAPS_PER_FILE, NPC_TYPE_ARRAY_LEN, NPC_TYPE_ARRAY_OFFSET, NPC_WORLD_ROSTER_MAX,
    SCENE_ARARAT, SCENE_BORDERMARCH, SCENE_BRITAIN, SCENE_BUCCANEERS_DEN, SCENE_COVE,
    SCENE_EAST_BRITANNY, SCENE_EMPATH_ABBEY, SCENE_FARTHING, SCENE_FOGSBANE, SCENE_GREYHAVEN,
    SCENE_IOLOS_HUT, SCENE_JHELOM, SCENE_LORD_BLACKTHORNS_CASTLE, SCENE_LORD_BRITISHS_CASTLE,
    SCENE_MINOC, SCENE_MOONGLOW, SCENE_NEW_MAGINCIA, SCENE_NORTH_BRITANNY, SCENE_PAWS,
    SCENE_SERPENTS_HOLD, SCENE_SKARA_BRAE, SCENE_STONEGATE, SCENE_STORMCROW, SCENE_THE_LYCAEUM,
    SCENE_TRINSIC, SCENE_WAVEGUIDE, SCENE_WEST_BRITANNY, SCENE_WINDEMERE, SCENE_YEW,
    TOWN_ARREST_JAIL_FLOOR, TOWN_ARREST_JAIL_SCENE, TOWN_ARREST_JAIL_X, TOWN_ARREST_JAIL_Y,
    TOWN_DAWN_DUSK_GATE_CLOSED_TILE, TOWN_DAWN_DUSK_GATE_MARKER_TILE,
    TOWN_DAWN_DUSK_GATE_OPEN_TILE, TOWN_DAWN_DUSK_GATE_TOGGLE_XOR, TOWN_EXIT_THRESHOLD_TILE,
    TOWN_EXIT_UNDERWORLD_SCENE, TOWN_GRID_BYTES, TOWN_GRID_SIDE, TOWN_NIGHT_BAND_DAWN_HOUR,
    TOWN_NIGHT_BAND_DUSK_HOUR, TOWN_NPC_BLOCK_BYTES, TOWN_NPC_ROSTER_SLOTS, TOWN_STAIR_TILE_FIRST,
    TOWN_STAIR_TILE_LAST, TOWN_TILE_DASH_MARKER, TOWN_TILE_NPC_START_A, TOWN_TILE_NPC_START_B,
    TOWN_TILE_PERIOD_MARKER, TOWN_TILE_SPAWN_ASTERISK, TOWNE_NPC_FILENAME, TOWNE_TLK_FILENAME,
    TownArrestPrompt, TownLocationClass, TownNpcAlarmMarker, TownNpcAlarmState,
    TownNpcAttackResolution, TownStairIntent, TownTileMarker, WORLD_LOCATION_TABLE_DUNGEON_ROWS,
    WORLD_LOCATION_TABLE_TOTAL_ROWS, WORLD_LOCATION_TABLE_TOWN_ROWS, location_dat_filename,
    npc_dialog_index_offset, npc_roster_filename, npc_schedule_hour_at_boundary,
    npc_schedule_record_offset, npc_schedule_waypoint_for_hour, npc_sub_map_offset,
    npc_tlk_filename, npc_type_byte_offset, town_dawn_dusk_gate_pass_fires_at_hour,
    town_dawn_dusk_gate_toggle, town_dawn_dusk_substitution_active, town_entry_is_jail_wakeup,
    town_exit_lands_underworld, town_floor_offset, town_location_class,
    town_npc_activation_mask_eligible, town_npc_attack_resolution, town_npc_type_guard_like,
    town_per_class_index, town_resident_name, town_stair_intent, town_tile_marker,
    world_location_table_scene_for_row,
};
pub use town_tables::*;
pub use town_tables_io::*;
pub use town_tables_io_movement::*;
pub use transport::{
    BoardVehicleCandidate, BoardableFamily, CARPET_BOARDING_EAST_MARKER,
    CARPET_BOARDING_NORTH_MARKER, CARPET_MOUNTED, CARPET_PARKED, FRIGATE_PURCHASE_HULL,
    FRIGATE_PURCHASE_SKIFFS, HORSE_MOUNTED_FIRST, HORSE_MOUNTED_LAST, HORSE_PARKED_FIRST,
    HORSE_PARKED_LAST, PendingVehicleAcquisition, SHIP_BOARDING_HULL_WARNING_THRESHOLD,
    SHIP_PARKED_FIRST, SHIP_PARKED_LAST, SKIFF_PARKED_FIRST, SKIFF_PARKED_LAST, TransportState,
    boardable_family, mount_horse_marker, ship_boarding_precondition_accepts,
    ship_boarding_stows_carpet, ship_boarding_warns, vehicle_exit_object_support,
};
pub use traps::*;
pub use u4_transfer::*;
pub use view_classes::{fc_sprite_proximity_mask_hits, tile_view_class};
pub use visibility::{
    ActiveObjectCompositeResult, ActiveObjectCompositorBranch, FOG_REFINE_SQUARED_THRESHOLD,
    LOCAL_LIGHT_MASK_SIDE, LightRadiusBranch, TERRAIN_BAND_ROW_STRIDE,
    VEHICLE_AVATAR_UNDERLAY_MARKER, VIEWPORT_CENTER, VIEWPORT_MAX_INDEX, VIEWPORT_PLAYER_COL,
    VIEWPORT_PLAYER_ROW, VIEWPORT_ROW_STRIDE, VIEWPORT_SIDE, VISIBILITY_ALREADY_RENDERED,
    VISIBILITY_CARVE_NEIGHBOR_ORDER, VISIBILITY_CLEAR, VISIBILITY_DIM_PERIPHERY, VISIBILITY_HIDDEN,
    VISIBILITY_USE_COMPANION, VisibilityMarker, active_object_composite,
    active_object_compositor_branch, active_object_compositor_variant,
    active_object_default_composite, active_object_default_tile_is_terrain_aware,
    fog_refine_folded_coord, fog_refine_inside_clear_core, fog_refine_squared_distance,
    is_local_light_source_tile, light_radius_branch, visibility_cheap_path_needs_refill,
    visibility_in_radius, visibility_marker,
};
pub use wind::{
    ACTIVE_SHIP_CADENCE_EVERY_TURN, ACTIVE_SHIP_CADENCE_INTO_WIND, ACTIVE_SHIP_CADENCE_WITH_WIND,
    PLAYER_SAIL_WAIT_TICKS_INTO_WIND, PLAYER_SAIL_WAIT_TICKS_PERPENDICULAR,
    PLAYER_SAIL_WAIT_TICKS_WITH_WIND, WIND_DRIFT_CALM_ACCEPT_MIN, WIND_DRIFT_CANDIDATE_MODULUS,
    WIND_DRIFT_OUTER_ROLL_MASK, WindSetterOutcome, WindState, wind_setter_outcome,
};
pub use world_tables::*;
pub use world_tables_io::*;
pub use world_tables_io_get_pickup::*;
pub use world_tables_io_locations::*;
pub use z_stats::*;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::fs;
    use std::io;
    use std::path::Path;

    use super::*;
    include!("tests_inline/chunk_01.rs");
    include!("tests_inline/chunk_02.rs");
    include!("tests_inline/chunk_03.rs");
    include!("tests_inline/chunk_04.rs");
    include!("tests_inline/chunk_05.rs");
    include!("tests_inline/chunk_06.rs");
    include!("tests_inline/chunk_07.rs");
    include!("tests_inline/chunk_08.rs");
    include!("tests_inline/chunk_09.rs");
    include!("tests_inline/chunk_10.rs");
    include!("tests_inline/chunk_11.rs");
    include!("tests_inline/chunk_12.rs");
    include!("tests_inline/chunk_13.rs");
    include!("tests_inline/chunk_14.rs");
    include!("tests_inline/chunk_15.rs");
    include!("tests_inline/chunk_16.rs");
    include!("tests_inline/chunk_17.rs");
    include!("tests_inline/chunk_18.rs");
    include!("tests_inline/chunk_19.rs");
    include!("tests_inline/chunk_20.rs");
    include!("tests_inline/chunk_21.rs");
    include!("tests_inline/chunk_22.rs");
    include!("tests_inline/chunk_23.rs");
}

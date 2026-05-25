//! Bevy-backed visual harness. The gameplay source of truth stays in
//! [`PlayState`]; this module only owns the window, the CPU framebuffer, and
//! the Bevy texture handle. Input dispatch reuses the terminal-mode handler so
//! movement, doors, transitions, and other supported behavior come along for
//! free.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use image::{ImageBuffer, Rgba};

use u5_runtime::{
    AWAKEN_COST, AWAKEN_SPELL_INDEX, ActiveObject, ArmsShop, BLINK_COST, BLINK_SPELL_INDEX,
    BRIT_CBT_RECORDS, BRITISH_PTH_PEN_ORIGINS, BritishPth, CBT_PLACEMENT_SLOT_COUNT,
    CGA_PALETTE_RGB, CH_CELL_SIDE, CODEX_URN_TABLE_FILE, COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED,
    COMBAT_ACTOR_FLAG_SELECTABLE_80, COMBAT_ACTOR_SLOTS, COMBAT_ARENA_SIDE, COMBAT_CLASS_GIANT_RAT,
    COMBAT_DEFAULT_DEATH_DROP_TILE, COMBAT_FIELD_KIND_ENERGY, COMBAT_FIELD_KIND_FIRE,
    COMBAT_FIELD_KIND_POISON, COMBAT_FIELD_KIND_SLEEP, COMBAT_GARGOYLE_DEATH_TERRAIN_TILE,
    COMBAT_GAZER_DEATH_MARKER_TILE, COMBAT_PARTY_ACTOR_SLOTS, COMBAT_PARTY_CORPSE_TILE,
    COMBAT_VANISH_DEATH_MARKER_TILE, CREATE_FOOD_COST, CREATE_FOOD_SPELL_INDEX, CURE_COST,
    CURE_SPELL_INDEX, ChargenSession, ChargenSessionResult, ChargenSessionStep,
    CombatActorDescriptor, DEATH_VISION_OBJECT_CLASS, DEATH_WIND_COST, DEATH_WIND_SPELL_INDEX,
    DEFAULT_CLIMB_STAT, DEFAULT_FOOD_STOCK, DES_POR_SPELL_INDEX, DISPEL_FIELD_COST,
    DISPEL_FIELD_SPELL_INDEX, DUNGEON_CBT_RECORDS, DUNGEON_LEVEL_SPELL_COST, Direction,
    DiskIoHandlerPhase, DungeonRoomCombatSetup, DungeonScene, EGA_PALETTE_RGB,
    ENDGAME_TABLEAU_HEIGHT, ENDGAME_TABLEAU_WIDTH, ENERGY_FIELD_COST, ENERGY_FIELD_SPELL_INDEX,
    EQUIP_SLOT_RING, EQUIP_SLOT_WEAPON, EQUIPMENT_EMPTY, EQUIPMENT_ID_ARROWS, EQUIPMENT_ID_BOW,
    EQUIPMENT_ID_RING_REGENERATION, FIELD_SPELL_COST, FIRE_FIELD_SPELL_INDEX,
    FIRST_PLAYABLE_FRIGATE_TILE, FIRST_PLAYABLE_FULL_SHIP_HULL, FLAME_WIND_COST,
    FLAME_WIND_SPELL_INDEX, FixedCellFont, GATE_TRAVEL_COST, GATE_TRAVEL_SPELL_INDEX,
    GREAT_HEAL_COST, GREAT_HEAL_SPELL_INDEX, GameClock, GraphicImage, GuildShop, HEAL_COST,
    HEAL_SPELL_INDEX, HORSE_PARKED_FIRST, Healer, Herbalist, IN_LOR_COST, IN_LOR_SPELL_INDEX,
    IN_WIS_COST, IN_WIS_SPELL_INDEX, INTRO_INLINE_DOORWAY_STEP, INTRO_START_MENU_REVEAL_RECT,
    INTRO_STEP_1_EXTRA_ART_X, INTRO_STEP_1_EXTRA_ART_Y, INTRO_STEP_1_EXTRA_SUBIMAGE,
    INTRO_STEP_1_RECT_TRANSITION, INTRO_STEP_6_EXTRA_ART_X, INTRO_STEP_6_EXTRA_ART_Y,
    INTRO_STEP_6_EXTRA_SUBIMAGE, INTRO_STORY_STEP_COUNT, INTRO_STORY6_SECONDARY_Y_DELTA, Inn,
    IntroStoryArtPlacement, MAGIC_LOCK_COST, MAGIC_LOCK_SPELL_INDEX, MAIN_TEXT_WINDOW_INDEX,
    MISCMAPS_DAT_FILE, MISCMAPS_RTV_COMMAND_SECTION_OFFSET, MISCMAPS_RTV_STRIP_SECTION_BYTES,
    MISCMAPS_RTV_STRIP_SECTION_OFFSET, MonochromeBitmap, MoonstoneGateSlot, NARRATIVE_GATE_X,
    NARRATIVE_GATE_Y, NATURAL_MOONGATE_TERRAIN_TILE, NEGATE_MAGIC_COST, NEGATE_MAGIC_SPELL_INDEX,
    NpcSlot, OOL_SLOTS, OPEN_SPELL_COST, OPEN_SPELL_INDEX, PCS_GLYPH_HEIGHT, PEER_COST,
    PEER_SPELL_INDEX, PLAY_MUSIC_TOGGLE_KEY, PLAYER_SPRITE_TILE, PLAYER_TILE,
    POISON_FIELD_SPELL_INDEX, POISON_WIND_COST, POISON_WIND_SPELL_INDEX, PROMPT_TEXT_WINDOW_INDEX,
    PROTECTION_COST, PROTECTION_SPELL_INDEX, PartyMember, PlayInputDisposition, PlayOptions,
    PlayState, PlayTarget, ProportionalFont, ProportionalWidthTable, QUICKNESS_COST,
    QUICKNESS_SPELL_INDEX, REAGENT_COUNT, REAGENT_SULFUR_ASH, REL_HUR_COST, REL_HUR_SPELL_INDEX,
    RESURRECT_COST, RESURRECT_SPELL_INDEX, RTV_COMMAND_STREAM_BYTES, RectColumnSweepTransition,
    ReturnToViewFrameKind, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME, SAVED_OOL_LEN,
    SCENE_EMPATH_ABBEY, SCENE_JHELOM, SCENE_MOONGLOW, SCENE_SERPENTS_HOLD, SCENE_STONEGATE,
    SCENE_THE_LYCAEUM, SHADOWLORD_COWARDICE_INDEX, SHADOWLORD_FALSEHOOD_INDEX,
    SHADOWLORD_HATRED_INDEX, SHADOWLORD_HIDEOUT_VANQUISHED, SHADOWLORD_OBJECT_TILE_BASE,
    SHADOWLORD_VANQUISHED, SHRINE_ALTAR_TILE_FIRST, SLEEP_COST, SLEEP_FIELD_SPELL_INDEX,
    SLEEP_SPELL_INDEX, SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX, SPECIAL_ITEM_MAGIC_CARPET_INDEX,
    SPECIAL_ITEM_OWNED_VALUE, SPECIAL_ITEM_POCKET_WATCH_INDEX, SPECIAL_ITEM_SCEPTRE_LB_INDEX,
    SPECIAL_ITEM_SEXTANT_INDEX, SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
    SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX, SPECIAL_ITEM_SHARD_HATRED_INDEX,
    SPECIAL_ITEM_SPYGLASS_INDEX, SPECIAL_ITEM_WOODEN_BOX_INDEX, STATS_PANEL_TEXT_BOTTOM,
    STATS_PANEL_TEXT_LEFT, STATS_PANEL_TEXT_RIGHT, STATS_PANEL_TEXT_WINDOW_INDEX, STEADY_PHASE,
    SURFACE_CHASM_X, SURFACE_CHASM_Y, Scene, Shipwright, ShrineVirtue, Stable, StoryRecords,
    TALK_SHOP_TEXT_WINDOW_INDEX, TALK_STATUS_TILE_PRAYING, TALK_STATUS_TILE_SLEEPING,
    TERRAIN_COMBAT_PARTY_POSITIONS, TEXT_SCREEN_ROWS, TEXT_WINDOW_RENDER_HEIGHT,
    TEXT_WINDOW_RENDER_WIDTH, TILE_ATLAS_SIDE, TIME_STOP_COST, TIME_STOP_SPELL_INDEX,
    TITLE_BIT_INITIAL_PLACEMENTS, TITLE_BIT_REMAINING_PLACEMENTS, TITLE_LOWER_BAND_CLEAR_Y,
    TITLE_SURFACE_HEIGHT, TITLE_SURFACE_WIDTH, TITLE_TICK_FRAME_HEIGHT, TITLE_TICK_FRAME_WIDTH,
    TITLE_TICK_FRAME_X, TITLE_TICK_FRAME_Y, TLK_TEXT_XOR_MASK, TOWN_GAS_DOORWAY_RANGE_MAX,
    TOWN_GRID_SIDE, TOWN_POISON_GAS_LIVE_TILE, Tavern, TerrainCombatSetup, TextWindowSystem,
    TileAtlas, TileGraphicsDepth, TileViewport, TitleBitAsset, TitleBitImages, TitleBitPlacement,
    TransportState, U4TransferOverrides, U4TransferSource, UNLOCK_MAGIC_COST,
    UNLOCK_MAGIC_SPELL_INDEX, UUS_POR_SPELL_INDEX, VANISH_COST, VANISH_SPELL_INDEX, VAS_LOR_COST,
    VAS_LOR_SPELL_INDEX, ViewOverlayMode, WORLD_SIDE, WindState, WorldPlane, WorldReturn,
    X_RAY_COST, X_RAY_SPELL_INDEX, blit_tile_id_to_viewport, combat_actor_is_active_not_dead,
    combat_class_stats, commit_chargen_save, commit_u4_transfer_save,
    configure_talk_shop_text_window,
    conversation_session::ConversationSession,
    default_party_equipment, default_party_experience, default_party_intelligence,
    default_party_names, default_party_roster, default_party_stay_counters, disk_io_error_message,
    dungeon_cell_index, dungeon_room_combat_instance_from_setup,
    dungeon_room_combat_setup_from_record_for_entry, dungeon_room_entry_seed_for_direction,
    endgame_tableau_role_for_slot, handle_play_key_input, hash_bytes, input_case_fold,
    input_function_key_code, input_keypad_digit_direction_code,
    intro_menu::{IntroSubflow, IntroSubflowResult},
    intro_step_has_story6_secondary_pass, intro_step_transition_strips,
    intro_story_art_file_for_step, intro_story_art_placement_for_step,
    intro_story_step_waits_for_input, intro_story6_secondary_subimage, load_brit_cbt,
    load_british_bit, load_british_pth, load_dungeon_cbt, load_graphic_image_directory,
    load_ibm_ch_font, load_legacy_proportional_font, load_play_options_from_save,
    load_question_records, load_return_to_view_assets, load_story_records, load_tile_atlas,
    load_title_bit,
    menu_dispatch::{UnifiedMenuDispatch, UnifiedMenuStep},
    paint_inn_pickup_register_text_window, paint_message_text_window,
    paint_prompt_text_window_with_cursor, paint_stats_panel_text_window,
    paint_talk_shop_text_window, published_world_location_entries,
    rasterize_proportional_paragraph, read_u4_transfer_source_from_party_sav,
    render_play_text_window_system, render_return_to_view_playback_frame_viewport,
    render_text_panel_rgba, render_text_window_rgba, return_to_view_fixed_wipe_rectangles,
    run_return_to_view_playback_until_restart,
    shop_runtime::{
        ArmsShopState, GuildShopState, HealerShopState, HorseTraderState, InnkeeperState,
        ReagentShopState, SageState, ShipBrokerState, TavernState,
    },
    shop_session::ActiveShopSession,
    spell_index_from_code, spell_mp_cost, stats_panel_active_cursor_visible,
    summarize_return_to_view_preview, summarize_return_to_view_script,
    summoned_active_object_record, terrain_combat_instance_from_setup,
    terrain_combat_raw_replacement_tile_for_arena, terrain_combat_setup_from_record,
    terrain_combat_tile_for_spawn_index, title_tick_flame_palette_index, title_tick_next_frame,
    town_resident_name,
    u4_transfer_session::{U4TransferPreview, u4_transfer_preview_from_u4_values},
    u5_prng_range_u16, word_of_power_seal_for_word,
};

const VIEWPORT_RADIUS: usize = 5;
const VIEWPORT_CELLS: usize = VIEWPORT_RADIUS * 2 + 1;
const VIEWPORT_SIZE_PX: u32 = (VIEWPORT_CELLS * TILE_ATLAS_SIDE) as u32;
const DISPLAY_SCALE: f32 = 3.0;
const SURFACE_VIEW_CLASS_GALLERY_TILES: [u8; 17] = [
    0x00, 0x05, 0x09, 0x70, 0x1D, 0x10, 0x0D, 0x0C, 0x0B, 0x06, 0x60, 0xD4, 0x01, 0x04, 0xE0, 0xD8,
    0x20,
];

const READY_HINT: &str =
    "Arrows/keypad: move. Shift+A attacks, Shift+S searches. Ctrl+S music. Esc quit.";
const INTRO_FRAMEBUFFER_WIDTH: u32 = TEXT_WINDOW_RENDER_WIDTH as u32;
const INTRO_FRAMEBUFFER_HEIGHT: u32 = TEXT_WINDOW_RENDER_HEIGHT as u32;
const INTRO_DISPLAY_SCALE: f32 = 2.5;
const RETURN_TO_VIEW_CAPTION_Y: usize = 4;
const RETURN_TO_VIEW_CAPTION_HEIGHT: usize = 14;
const RETURN_TO_VIEW_PREVIEW_Y: usize = 18;
const RETURN_TO_VIEW_FIXED_WIPE_RGBA: [u8; 4] = [0xff, 0x55, 0xff, 0xff];
const INTRO_ANIMATION_TICK_INTERVAL_SECS: f32 = 1.0 / 18.2;
const PROPORTIONAL_TEXT_LINE_HEIGHT: usize = PCS_GLYPH_HEIGHT + 2;
const INTRO_STORY_TEXT_X: usize = 10;
const INTRO_STORY_TEXT_Y: usize = 138;
const INTRO_STORY_TEXT_WIDTH: usize = 300;
const CHARGEN_PROPORTIONAL_TEXT_X: usize = 16;
const CHARGEN_PROPORTIONAL_TEXT_Y: usize = 34;
const CHARGEN_PROPORTIONAL_TEXT_WIDTH: usize = 288;
const CHARGEN_QUESTION_TEXT_X: usize = 8;
const CHARGEN_QUESTION_TEXT_Y: usize = 150;
const CHARGEN_QUESTION_TEXT_WIDTH: usize = 304;
const CHARGEN_RESULT_TEXT_X: usize = 16;
const CHARGEN_RESULT_TEXT_Y: usize = 24;
const CHARGEN_RESULT_TEXT_WIDTH: usize = 292;
const PROMPT_CURSOR_GLYPH: u8 = 4;

#[derive(Clone, Copy)]
struct ImagePanelSpec {
    stem: &'static str,
    subimage: u8,
    top_left_x: usize,
    top_left_y: usize,
    width: usize,
    height: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IntroDisplayBuffer {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl IntroDisplayBuffer {
    fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; width.saturating_mul(height)],
        }
    }

    fn clear(&mut self, color: u8) {
        self.pixels.fill(color & 0x0f);
    }

    fn clear_rect_inclusive(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: u8) {
        if self.width == 0 || self.height == 0 {
            return;
        }
        let x0 = x0.min(self.width - 1);
        let x1 = x1.min(self.width - 1);
        let y0 = y0.min(self.height - 1);
        let y1 = y1.min(self.height - 1);
        if x0 > x1 || y0 > y1 {
            return;
        }
        for y in y0..=y1 {
            let start = y * self.width + x0;
            let end = y * self.width + x1 + 1;
            self.pixels[start..end].fill(color & 0x0f);
        }
    }

    fn blit_rgba(
        &mut self,
        src: &[u8],
        src_width: usize,
        src_height: usize,
        dst_x: usize,
        dst_y: usize,
    ) {
        for row in 0..src_height {
            let y = dst_y + row;
            if y >= self.height {
                break;
            }
            let cols = src_width.min(self.width.saturating_sub(dst_x));
            for col in 0..cols {
                let src_offset = (row * src_width + col) * 4;
                let Some(rgba) = src.get(src_offset..src_offset + 4) else {
                    continue;
                };
                self.pixels[y * self.width + dst_x + col] = ega_palette_index_from_rgba(rgba);
            }
        }
    }

    fn draw_title_tick(&mut self, frame: u8) {
        let start_x = TITLE_TICK_FRAME_X as usize;
        let start_y = TITLE_TICK_FRAME_Y as usize;
        let end_x = start_x
            .saturating_add(TITLE_TICK_FRAME_WIDTH as usize)
            .min(self.width);
        let end_y = start_y
            .saturating_add(TITLE_TICK_FRAME_HEIGHT as usize)
            .min(self.height);

        for y in start_y..end_y {
            let local_y = y - start_y;
            for x in start_x..end_x {
                let local_x = x - start_x;
                self.pixels[y * self.width + x] =
                    title_tick_flame_palette_index(local_x, local_y, frame).unwrap_or(0);
            }
        }
    }

    fn copy_revealed_columns_from(
        &mut self,
        source: &IntroDisplayBuffer,
        transition: RectColumnSweepTransition,
    ) {
        if self.width != source.width || self.height != source.height {
            return;
        }
        let Some((start_x, end_x)) = transition.revealed_columns() else {
            return;
        };
        let (rect_x0, rect_y0, rect_x1, rect_y1) = transition.rect;
        if self.width == 0 || self.height == 0 {
            return;
        }
        let y0 = usize::from(rect_y0).min(self.height - 1);
        let y1 = usize::from(rect_y1).min(self.height - 1);
        let x0 = usize::from(rect_x0).min(self.width - 1);
        let x1 = usize::from(rect_x1).min(self.width - 1);
        let revealed_start = usize::from(start_x).min(self.width - 1);
        let revealed_end = usize::from(end_x).min(self.width - 1);
        if x0 > x1 || y0 > y1 {
            return;
        }

        for y in y0..=y1 {
            for x in x0..=x1 {
                if x < revealed_start || x > revealed_end {
                    continue;
                }
                let index = y * self.width + x;
                self.pixels[index] = source.pixels[index];
            }
        }
    }

    fn from_rgba(width: usize, height: usize, rgba: &[u8]) -> Self {
        let mut buffer = Self::new(width, height);
        for (index, pixel) in rgba.chunks_exact(4).take(width * height).enumerate() {
            buffer.pixels[index] = ega_palette_index_from_rgba(pixel);
        }
        buffer
    }

    fn to_rgba(&self) -> Vec<u8> {
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for palette_index in &self.pixels {
            let rgb = EGA_PALETTE_RGB[usize::from(*palette_index & 0x0f)];
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
        rgba
    }
}

const STARTSC_PANEL_SPECS: [ImagePanelSpec; 3] = [
    ImagePanelSpec {
        stem: "STARTSC",
        subimage: 0,
        top_left_x: 0,
        top_left_y: 0,
        width: 16,
        height: 137,
    },
    ImagePanelSpec {
        stem: "STARTSC",
        subimage: 1,
        top_left_x: 16,
        top_left_y: 0,
        width: 288,
        height: 137,
    },
    ImagePanelSpec {
        stem: "STARTSC",
        subimage: 2,
        top_left_x: 304,
        top_left_y: 0,
        width: 16,
        height: 137,
    },
];

const INTRO_MENU_LABELS: [(IntroSubflow, usize, usize, &str); 6] = [
    (IntroSubflow::JourneyOnward, 12, 17, " Journey Onward "),
    (IntroSubflow::CharacterCreation, 9, 18, " Create New Char. "),
    (IntroSubflow::UltimaIvTransfer, 8, 19, " Transfer from U4 "),
    (IntroSubflow::StorySlides, 9, 20, " Ultima V Intro. "),
    (IntroSubflow::Acknowledgements, 11, 21, " Acknowledgements "),
    (IntroSubflow::ReturnToView, 10, 22, " Return to View "),
];
const INTRO_MENU_IDLE_RETURN_TO_VIEW_TICKS: u16 = 200;

const CREATE_OPENING_PANEL: ImagePanelSpec = ImagePanelSpec {
    stem: "CREATE",
    subimage: 0,
    top_left_x: 0,
    top_left_y: 96,
    width: 168,
    height: 96,
};
const CREATE_QUESTION_BACKING_LEFT: ImagePanelSpec = ImagePanelSpec {
    stem: "CREATE",
    subimage: 1,
    top_left_x: 16,
    top_left_y: 0,
    width: 120,
    height: 148,
};
const CREATE_QUESTION_BACKING_RIGHT: ImagePanelSpec = ImagePanelSpec {
    stem: "CREATE",
    subimage: 1,
    top_left_x: 200,
    top_left_y: 0,
    width: 120,
    height: 148,
};
const CREATE_RESULT_PANEL: ImagePanelSpec = ImagePanelSpec {
    stem: "CREATE",
    subimage: 10,
    top_left_x: 168,
    top_left_y: 100,
    width: 152,
    height: 100,
};

const CREATE_VIRTUE_PANEL_SPECS: [ImagePanelSpec; 8] = [
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 2,
        top_left_x: 40,
        top_left_y: 5,
        width: 51,
        height: 67,
    },
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 3,
        top_left_x: 48,
        top_left_y: 7,
        width: 43,
        height: 67,
    },
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 4,
        top_left_x: 48,
        top_left_y: 4,
        width: 34,
        height: 69,
    },
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 5,
        top_left_x: 40,
        top_left_y: 10,
        width: 55,
        height: 58,
    },
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 6,
        top_left_x: 40,
        top_left_y: 8,
        width: 48,
        height: 61,
    },
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 7,
        top_left_x: 48,
        top_left_y: 0,
        width: 42,
        height: 64,
    },
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 8,
        top_left_x: 40,
        top_left_y: 5,
        width: 50,
        height: 65,
    },
    ImagePanelSpec {
        stem: "CREATE",
        subimage: 9,
        top_left_x: 48,
        top_left_y: 6,
        width: 42,
        height: 65,
    },
];

pub fn run_visual_loop(
    game_dir: &Path,
    options: PlayOptions,
    raster_depth: TileGraphicsDepth,
) -> std::io::Result<()> {
    let state = PlayState::load_scene(game_dir, options)?;
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    let text_font = load_ibm_ch_font(game_dir)?;
    let bootstrap = Bootstrap {
        game_dir: game_dir.to_path_buf(),
        state,
        atlas,
        text_font,
    };

    let display_w = VISUAL_PLAY_FRAME_WIDTH as f32 * DISPLAY_SCALE;
    let display_h = VISUAL_PLAY_FRAME_HEIGHT as f32 * DISPLAY_SCALE;

    // Headless screenshot driver: when U5_BEVY_SCREENSHOT is set, the harness
    // waits a few frames (so the swapchain has a real image), takes a
    // screenshot via Bevy's `Screenshot` component, then exits. Lets us
    // verify end-to-end Bevy rendering without an interactive desktop.
    let screenshot_path: Option<PathBuf> =
        std::env::var("U5_BEVY_SCREENSHOT").ok().map(PathBuf::from);
    let screenshot_delay: u32 = std::env::var("U5_BEVY_SCREENSHOT_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    // Optional pre-screenshot keystrokes (single chars), e.g.
    // `U5_BEVY_PRESS=dddss` to step east 3 then south 2 before the shot.
    let preset_keys: Vec<char> = std::env::var("U5_BEVY_PRESS")
        .ok()
        .map(|s| s.chars().collect())
        .unwrap_or_default();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ultima V".into(),
                resolution: (display_w, display_h).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(PendingBootstrap(Mutex::new(Some(bootstrap))))
        .insert_resource(ScreenshotConfig {
            path: screenshot_path,
            frame_delay: screenshot_delay,
            preset_keys,
        })
        .insert_resource(ScreenshotState::default())
        .add_systems(Startup, setup)
        .insert_resource(AnimationPump::default())
        .add_systems(
            Update,
            (drive_visual, animate_static_tiles, screenshot_system),
        )
        .run();

    Ok(())
}

pub fn run_visual_intro_loop(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
) -> std::io::Result<()> {
    let launch_result = Arc::new(Mutex::new(None));
    run_visual_intro_menu_app(game_dir.to_path_buf(), raster_depth, launch_result.clone());
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualFrameReport {
    pub label: String,
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub frame_kind: &'static str,
    pub byte_hash: u64,
    pub nonblack_pixels: usize,
}

pub fn run_visual_frame_suite(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out_dir: &Path,
) -> io::Result<()> {
    let reports = visual_frame_suite(game_dir, raster_depth, out_dir)?;
    for report in &reports {
        println!(
            "visual-suite {}: {}x{} {} hash {:016x} nonblack {} -> {}",
            report.label,
            report.width,
            report.height,
            report.frame_kind,
            report.byte_hash,
            report.nonblack_pixels,
            report.path.display()
        );
    }
    println!(
        "Saved Bevy visual frame suite: {} PNG(s) plus manifest at {}.",
        reports.len(),
        out_dir.join("manifest.txt").display()
    );
    Ok(())
}

pub fn run_visual_route_suite(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out_dir: &Path,
) -> io::Result<()> {
    let reports = visual_route_suite(game_dir, raster_depth, out_dir)?;
    for report in &reports {
        println!(
            "visual-route {}: {}x{} {} hash {:016x} nonblack {} -> {}",
            report.label,
            report.width,
            report.height,
            report.frame_kind,
            report.byte_hash,
            report.nonblack_pixels,
            report.path.display()
        );
    }
    println!(
        "Saved Bevy visual route suite: {} PNG(s) plus manifest at {}.",
        reports.len(),
        out_dir.join("manifest.txt").display()
    );
    Ok(())
}

pub fn visual_frame_suite(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out_dir: &Path,
) -> io::Result<Vec<VisualFrameReport>> {
    std::fs::create_dir_all(out_dir)?;
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    let font = load_ibm_ch_font(game_dir)?;
    let mut reports = Vec::new();

    for case in visual_gameplay_frame_cases() {
        let mut state = PlayState::load_scene(game_dir, case.options)?;
        if case.synthetic_combat {
            seed_visual_suite_combat(&mut state);
        }
        if let Some(configure) = case.configure {
            configure(&mut state);
        }
        if case.label == "endgame-status" {
            state.enter_endgame_from_game_dir(Some(game_dir))?;
        }
        if let Some(inputs) = case.inputs {
            for (key, suffix) in inputs {
                handle_play_key_input(&mut state, *key, suffix, game_dir)?;
            }
        }
        reports.push(write_visual_play_report(
            out_dir,
            case.label,
            case.frame_kind,
            &mut state,
            &atlas,
            &font,
        )?);
    }
    push_visual_combat_gallery_reports(game_dir, out_dir, &atlas, &font, &mut reports)?;
    push_visual_surface_view_class_gallery_reports(game_dir, out_dir, &atlas, &font, &mut reports)?;

    reports.push(write_visual_intro_report(
        out_dir,
        "intro-menu",
        "intro menu",
        VisualIntroPanel::Menu,
        game_dir,
        raster_depth,
    )?);
    reports.push(write_visual_intro_report_with_title_dismissed(
        out_dir,
        "intro-finished-menu",
        "intro finished menu",
        game_dir,
        raster_depth,
    )?);
    if let Some(records) = load_story_records(game_dir)? {
        reports.push(write_visual_intro_report(
            out_dir,
            "intro-story-art",
            "intro story art",
            VisualIntroPanel::Story {
                records,
                step: 7,
                transition: None,
            },
            game_dir,
            raster_depth,
        )?);
    }
    let preview = visual_return_to_view_summary(game_dir, raster_depth);
    reports.push(write_visual_intro_report(
        out_dir,
        "intro-return-to-view",
        "intro return-to-view",
        VisualIntroPanel::ReturnToView {
            summary: preview.summary,
            preview_frames_rgba: preview.frames_rgba,
            frame_metadata: preview.frame_metadata,
            preview_frame_index: 0,
            preview_width: preview.width,
            preview_height: preview.height,
        },
        game_dir,
        raster_depth,
    )?);

    for report in &reports {
        if report.nonblack_pixels == 0 {
            return Err(io::Error::other(format!(
                "visual frame suite `{}` produced an all-black PNG",
                report.label
            )));
        }
    }
    write_visual_frame_suite_manifest(out_dir, &reports)?;
    Ok(reports)
}

pub fn visual_route_suite(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out_dir: &Path,
) -> io::Result<Vec<VisualFrameReport>> {
    std::fs::create_dir_all(out_dir)?;
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    let font = load_ibm_ch_font(game_dir)?;
    let mut reports = Vec::new();

    for case in visual_route_suite_cases() {
        let route_game_dir = prepare_visual_route_case_game_dir(case.label)?;
        let reload_save_dir = prepare_visual_route_reload_save_dir(game_dir, case.label)?;
        let reload_checkpoints = visual_route_reload_checkpoints(case.label);
        let command_game_dir = route_game_dir.as_deref().unwrap_or(game_dir);
        let mut state = PlayState::load_scene(game_dir, case.options)?;
        if let Some(configure) = case.configure {
            configure(&mut state);
        }
        apply_visual_route_initial_setup(&mut state, case.label, game_dir)?;
        let initial = write_visual_play_report(
            out_dir,
            &visual_route_step_label(case.label, 0, "initial"),
            case.frame_kind,
            &mut state,
            &atlas,
            &font,
        )?;
        let mut previous_hash = initial.byte_hash;
        reports.push(initial);

        for (index, command) in case.script.iter().enumerate() {
            apply_visual_route_command(&mut state, command, command_game_dir)?;
            if reload_checkpoints.contains(&(index + 1)) {
                let Some(save_dir) = reload_save_dir.as_deref() else {
                    return Err(io::Error::other(format!(
                        "visual route suite `{}` has reload checkpoints but no temp save dir",
                        case.label
                    )));
                };
                reload_visual_route_state_from_checkpoint(&mut state, game_dir, save_dir)?;
            }
            let report = write_visual_play_report(
                out_dir,
                &visual_route_step_label(case.label, index + 1, command),
                case.frame_kind,
                &mut state,
                &atlas,
                &font,
            )?;
            if report.byte_hash == previous_hash
                && !visual_route_allows_unchanged_step(case.label, index + 1)
            {
                return Err(io::Error::other(format!(
                    "visual route suite `{}` command `{}` did not change the frame",
                    case.label, command
                )));
            }
            previous_hash = report.byte_hash;
            reports.push(report);
        }
        if let Some(dir) = &reload_save_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
        if let Some(dir) = &route_game_dir {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
    push_visual_key_route_reports(game_dir, out_dir, &atlas, &font, &mut reports)?;

    for report in &reports {
        if report.nonblack_pixels == 0 {
            return Err(io::Error::other(format!(
                "visual route suite `{}` produced an all-black PNG",
                report.label
            )));
        }
    }
    write_visual_frame_suite_manifest(out_dir, &reports)?;
    Ok(reports)
}

fn apply_visual_route_initial_setup(
    state: &mut PlayState,
    label: &str,
    game_dir: &Path,
) -> io::Result<()> {
    if let Some(index) = visual_route_public_location_index(label) {
        let Some(entry) = published_world_location_entries().into_iter().nth(index) else {
            return Err(io::Error::other(format!(
                "visual route `{label}` does not map to a published location row"
            )));
        };
        state.area = u5_runtime::Area::World { plane: entry.plane };
        state.player.x = entry.x;
        state.player.y = entry.y;
        if let Some(object) = state.active_objects.get_mut(0) {
            object.z = entry.plane.save_floor();
        }
        state.sync_player_object();
        state.mark_visibility_dirty();
        return Ok(());
    }
    if let Some(virtue) = visual_route_shrine_virtue(label) {
        seed_visual_route_shrine(state, virtue);
        return Ok(());
    }
    match label {
        "route-endgame-missing-box-terminal-jitter"
        | "route-endgame-missing-box-confirmation"
        | "route-endgame-box-victory-confirmation"
        | "route-endgame-box-full-victory-cinematic"
        | "route-endgame-class-tableau-restoration" => {
            state.enter_endgame_from_game_dir(Some(game_dir))?;
        }
        "route-codex-urn-honesty-read" => {
            seed_visual_route_shrine(state, ShrineVirtue::Honesty);
            state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
            state.shrine_codex_mask = 0;
        }
        "route-shrine-honesty-codex-turn-in" => {
            seed_visual_route_shrine(state, ShrineVirtue::Honesty);
            state.shrine_ordained_mask = ShrineVirtue::Honesty.bit();
            state.shrine_codex_mask = ShrineVirtue::Honesty.bit();
            state.moral_standing = 10;
        }
        "route-shrine-compassion-completed-offering" => {
            seed_visual_route_shrine(state, ShrineVirtue::Compassion);
            state.shrine_ordained_mask = 0;
            state.shrine_codex_mask = ShrineVirtue::Compassion.bit();
            state.moral_standing = 10;
        }
        _ => {}
    }
    Ok(())
}

fn push_visual_key_route_reports(
    game_dir: &Path,
    out_dir: &Path,
    atlas: &TileAtlas,
    font: &FixedCellFont,
    reports: &mut Vec<VisualFrameReport>,
) -> io::Result<()> {
    for case in visual_key_route_suite_cases() {
        let mut state = PlayState::load_scene(game_dir, case.options)?;
        if let Some(configure) = case.configure {
            configure(&mut state);
        }
        let mut input_line = String::new();
        let mut prompt_cursor_visible = visual_line_prompt_active(&state);
        let initial = write_visual_play_report_with_input(
            out_dir,
            &visual_route_step_label(case.label, 0, "initial"),
            case.frame_kind,
            &mut state,
            atlas,
            font,
            &input_line,
            prompt_cursor_visible,
        )?;
        let mut previous_hash = initial.byte_hash;
        reports.push(initial);

        for (index, step) in case.steps.iter().enumerate() {
            apply_visual_key_route_step(&mut state, &mut input_line, *step, game_dir)?;
            prompt_cursor_visible = visual_line_prompt_active(&state);
            let report = write_visual_play_report_with_input(
                out_dir,
                &visual_key_route_step_label(case.label, index + 1, step),
                case.frame_kind,
                &mut state,
                atlas,
                font,
                &input_line,
                prompt_cursor_visible,
            )?;
            if report.byte_hash == previous_hash {
                return Err(io::Error::other(format!(
                    "visual key route suite `{}` step `{}` did not change the frame",
                    case.label, step.label
                )));
            }
            previous_hash = report.byte_hash;
            reports.push(report);
        }
    }
    Ok(())
}

fn apply_visual_key_route_step(
    state: &mut PlayState,
    input_line: &mut String,
    step: VisualKeyStep,
    game_dir: &Path,
) -> io::Result<()> {
    if visual_line_prompt_active(state) {
        match handle_visual_line_key(
            state,
            input_line,
            step.key,
            step.shift,
            step.control,
            game_dir,
        )? {
            Some(PlayInputDisposition::Quit) => {
                return Err(io::Error::other(format!(
                    "visual key route step `{}` requested quit",
                    step.label
                )));
            }
            Some(PlayInputDisposition::Continue) | None => return Ok(()),
        }
    }
    if step.key == KeyCode::Escape && should_escape_quit_visual(state) {
        return Err(io::Error::other(
            "visual key route Escape would quit outside a prompt",
        ));
    }
    let Some(ch) = key_code_to_char(step.key, step.shift, step.control) else {
        return Ok(());
    };
    match handle_play_key_input(state, ch, "", game_dir)? {
        PlayInputDisposition::Quit => Err(io::Error::other(format!(
            "visual key route step `{}` requested quit",
            step.label
        ))),
        PlayInputDisposition::Continue => Ok(()),
    }
}

fn visual_key_route_step_label(route_label: &str, step: usize, key: &VisualKeyStep) -> String {
    visual_route_step_label(route_label, step, key.label)
}

fn seed_visual_key_route_conversation(state: &mut PlayState) {
    fn enc(text: &str) -> Vec<u8> {
        text.bytes().map(|byte| byte ^ TLK_TEXT_XOR_MASK).collect()
    }
    let raw = vec![
        enc("Ada"),
        enc("a quiet smith"),
        enc("Greetings, traveller."),
        enc("I mend gear."),
        enc("Farewell."),
    ];
    let decoded = vec![
        "Ada".to_string(),
        "a quiet smith".to_string(),
        "Greetings, traveller.".to_string(),
        "I mend gear.".to_string(),
        "Farewell.".to_string(),
    ];
    state.active_conversation = Some(Box::new(ConversationSession::new(raw, decoded)));
    state.advance_active_conversation_greeting();
}

fn seed_visual_key_route_shrine(state: &mut PlayState) {
    seed_visual_route_shrine(state, ShrineVirtue::Honesty);
}

fn seed_visual_key_route_reagent_shop(state: &mut PlayState) {
    state.gold = 100;
    state.reagents = [0; REAGENT_COUNT];
    state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::for_herbalist(
        Herbalist::Mysticism,
    )));
}

fn seed_visual_key_route_ready_picker(state: &mut PlayState) {
    state.party_strengths = vec![50];
    state.party_equipment = default_party_equipment(1);
    state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
    state.equipment_stock[EQUIPMENT_ID_ARROWS] = 5;
}

fn seed_visual_key_route_use_picker(state: &mut PlayState) {
    state.clock = GameClock::with_date(139, 1, 1, 13, 0).expect("visual route clock is valid");
    state.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] = 1;
}

fn seed_visual_key_route_mix_prompt(state: &mut PlayState) {
    state.reagents = [0; REAGENT_COUNT];
    state.reagents[REAGENT_SULFUR_ASH] = 2;
}

fn seed_visual_key_route_rest_watch(state: &mut PlayState) {
    state.clock = GameClock::new(8, 0).expect("visual route clock is valid");
    state.party.push(PartyMember {
        slot: 1,
        class_byte: b'B',
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 0,
        hp: 8,
        max_hp: 12,
        level: 8,
    });
}

fn seed_visual_key_route_new_order(state: &mut PlayState) {
    state.party.push(PartyMember {
        slot: 1,
        class_byte: b'F',
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 0,
        hp: 20,
        max_hp: 20,
        level: 1,
    });
    state.party.push(PartyMember {
        slot: 2,
        class_byte: b'M',
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 4,
        hp: 18,
        max_hp: 18,
        level: 2,
    });
    state.party_names = vec![*b"AVATAR\0\0\0", *b"IOLO\0\0\0\0\0", *b"MARIA\0\0\0\0"];
}

fn visual_route_public_location_index(label: &str) -> Option<usize> {
    let suffix = label.strip_prefix("route-stock-location-enter-")?;
    let row = suffix.parse::<usize>().ok()?;
    (1..=published_world_location_entries().len())
        .contains(&row)
        .then_some(row - 1)
}

fn seed_visual_route_shrine(state: &mut PlayState, virtue: ShrineVirtue) {
    state.area = u5_runtime::Area::World {
        plane: WorldPlane::Britannia,
    };
    state.player.x = 62;
    state.player.y = 124;
    state.player.facing = Direction::East;
    let tile = SHRINE_ALTAR_TILE_FIRST + virtue.index() as u8;
    let idx = u5_runtime::world_cell_index(state.player.x, state.player.y);
    if let Some(cell) = state.grid.get_mut(idx) {
        *cell = tile;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn visual_route_shrine_virtue(label: &str) -> Option<ShrineVirtue> {
    let key = label
        .strip_prefix("route-shrine-native-")?
        .strip_suffix("-meditation")?;
    ShrineVirtue::from_key(key)
}

fn prepare_visual_route_case_game_dir(case_label: &str) -> io::Result<Option<PathBuf>> {
    if case_label != "route-codex-urn-honesty-read" {
        return Ok(None);
    }
    let dir = visual_route_temp_dir(case_label, "case")?;
    std::fs::write(
        dir.join(CODEX_URN_TABLE_FILE),
        format!("BRITANNIA 62 124 {SHRINE_ALTAR_TILE_FIRST}\n"),
    )?;
    Ok(Some(dir))
}

fn prepare_visual_route_reload_save_dir(
    game_dir: &Path,
    case_label: &str,
) -> io::Result<Option<PathBuf>> {
    if visual_route_reload_checkpoints(case_label).is_empty() {
        return Ok(None);
    }
    let dir = visual_route_temp_dir(case_label, "reload")?;
    seed_visual_route_save_files(game_dir, &dir)?;
    Ok(Some(dir))
}

fn visual_route_temp_dir(case_label: &str, label: &str) -> io::Result<PathBuf> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "u5-visual-route-{case_label}-{label}-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn seed_visual_route_save_files(game_dir: &Path, save_dir: &Path) -> io::Result<()> {
    if copy_visual_route_save_file(game_dir, save_dir, SAVED_GAM_FILENAME, SAVED_GAM_FILENAME)
        .is_err()
    {
        copy_visual_route_save_file(game_dir, save_dir, "INIT.GAM", SAVED_GAM_FILENAME)?;
    }
    if copy_visual_route_save_file(game_dir, save_dir, SAVED_OOL_FILENAME, SAVED_OOL_FILENAME)
        .is_err()
    {
        if game_dir.join("INIT.OOL").exists() {
            copy_visual_route_save_file(game_dir, save_dir, "INIT.OOL", SAVED_OOL_FILENAME)?;
        } else {
            std::fs::write(save_dir.join(SAVED_OOL_FILENAME), vec![0; SAVED_OOL_LEN])?;
        }
    }
    Ok(())
}

fn copy_visual_route_save_file(
    game_dir: &Path,
    save_dir: &Path,
    source_name: &str,
    destination_name: &str,
) -> io::Result<()> {
    std::fs::copy(game_dir.join(source_name), save_dir.join(destination_name)).map(|_| ())
}

fn visual_route_reload_checkpoints(case_label: &str) -> &'static [usize] {
    match case_label {
        "route-reload-boarded-horse-pass"
        | "route-reload-gate-travel-underworld-pass"
        | "route-reload-chasm-underworld-pass"
        | "route-reload-ship-xit-skiff-pass"
        | "route-reload-dungeon-ladder-down-up"
        | "route-reload-dungeon-ladder-down-up-route"
        | "route-reload-dungeon-surface-exit-return-world" => &[1],
        "route-reload-underworld-fixed-hidden-stack-search-get-search"
        | "route-reload-horse-trader-horse-and-rider-buy-pass" => &[2],
        _ => &[],
    }
}

fn reload_visual_route_state_from_checkpoint(
    state: &mut PlayState,
    game_dir: &Path,
    save_dir: &Path,
) -> io::Result<()> {
    state.save_game_command(save_dir, Some(true))?;
    let options = load_play_options_from_save(save_dir)?;
    *state = PlayState::load_scene(game_dir, options)?;
    Ok(())
}

struct VisualGameplayFrameCase {
    label: &'static str,
    frame_kind: &'static str,
    options: PlayOptions,
    inputs: Option<&'static [(char, &'static str)]>,
    configure: Option<fn(&mut PlayState)>,
    synthetic_combat: bool,
}

fn visual_gameplay_frame_cases() -> Vec<VisualGameplayFrameCase> {
    vec![
        VisualGameplayFrameCase {
            label: "world-play",
            frame_kind: "visual world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "world-after-step",
            frame_kind: "visual world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            inputs: Some(&[('d', ""), (' ', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "town-play",
            frame_kind: "visual town frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "dungeon-play",
            frame_kind: "visual dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(
                    DungeonScene::new(0x21).expect("dungeon scene is valid"),
                ),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            inputs: None,
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "dungeon-dark",
            frame_kind: "visual dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(
                    DungeonScene::new(0x21).expect("dungeon scene is valid"),
                ),
                floor: 0,
                torch_counter: 0,
                light_spell_counter: 0,
                ..PlayOptions::default()
            },
            inputs: None,
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "world-save-confirmation-prompt",
            frame_kind: "visual world prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            inputs: Some(&[('Q', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "world-hole-up-watch-prompt",
            frame_kind: "visual world prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            inputs: Some(&[('H', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "world-use-item-prompt",
            frame_kind: "visual world prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            inputs: Some(&[('U', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "castle-cast-party-prompt",
            frame_kind: "visual town prompt frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: Some(&[('C', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "castle-mix-reagent-prompt",
            frame_kind: "visual town prompt frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: Some(&[('M', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "castle-ready-party-prompt",
            frame_kind: "visual town prompt frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: Some(&[('R', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "castle-talk-keyword-prompt",
            frame_kind: "visual town conversation frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: Some(&[('T', ""), ('6', "")]),
            configure: Some(seed_visual_route_talk_ordinary_keyword),
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "dungeon-search-direction-prompt",
            frame_kind: "visual dungeon prompt frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(
                    DungeonScene::new(0x21).expect("dungeon scene is valid"),
                ),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            inputs: Some(&[('S', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "dungeon-open-direction-prompt",
            frame_kind: "visual dungeon prompt frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(
                    DungeonScene::new(0x21).expect("dungeon scene is valid"),
                ),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            inputs: Some(&[('O', "")]),
            configure: None,
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "combat-play",
            frame_kind: "visual combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: None,
            synthetic_combat: true,
        },
        VisualGameplayFrameCase {
            label: "combat-status-highlight",
            frame_kind: "visual combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: Some(seed_visual_suite_combat_status_highlight),
            synthetic_combat: true,
        },
        VisualGameplayFrameCase {
            label: "combat-attack-direction-prompt",
            frame_kind: "visual combat prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            inputs: Some(&[('A', "")]),
            configure: None,
            synthetic_combat: true,
        },
        VisualGameplayFrameCase {
            label: "combat-cast-party-prompt",
            frame_kind: "visual combat prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            inputs: Some(&[('C', "")]),
            configure: None,
            synthetic_combat: true,
        },
        VisualGameplayFrameCase {
            label: "combat-ready-party-prompt",
            frame_kind: "visual combat prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            inputs: Some(&[('R', "")]),
            configure: None,
            synthetic_combat: true,
        },
        VisualGameplayFrameCase {
            label: "combat-search-direction-prompt",
            frame_kind: "visual combat prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            inputs: Some(&[('S', "")]),
            configure: None,
            synthetic_combat: true,
        },
        VisualGameplayFrameCase {
            label: "surface-view-overlay",
            frame_kind: "visual view overlay frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: Some(|state| {
                state.gems = 1;
                state.view_gem();
            }),
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "dungeon-view-overlay",
            frame_kind: "visual view overlay frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(
                    DungeonScene::new(0x21).expect("dungeon scene is valid"),
                ),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            inputs: None,
            configure: Some(|state| {
                state.gems = 1;
                state.view_gem();
            }),
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "britannia-chunk-map-overlay",
            frame_kind: "visual view overlay frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: Some(|state| {
                state.clock = GameClock::new(20, 0).expect("20:00 is a valid game-clock time");
                state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
                state.use_spyglass();
            }),
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "peer-view-overlay",
            frame_kind: "visual view overlay frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: Some(|state| {
                state.activate_peer_view_overlay();
            }),
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "x-ray-view-overlay",
            frame_kind: "visual view overlay frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: Some(|state| {
                state.activate_x_ray_view_overlay();
            }),
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "z-stats-modal",
            frame_kind: "visual status modal frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            inputs: None,
            configure: Some(|state| {
                state.z_stats();
            }),
            synthetic_combat: false,
        },
        VisualGameplayFrameCase {
            label: "endgame-status",
            frame_kind: "visual endgame status frame",
            options: PlayOptions::default(),
            inputs: None,
            configure: None,
            synthetic_combat: false,
        },
    ]
}

fn seed_visual_suite_combat(state: &mut PlayState) {
    state.combat_active = true;
    state.combat_terrain = [[5; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.combat_terrain[0][0] = 12;
    state.combat_terrain[5][5] = 4;
    state.combat_terrain[6][5] = 1;
}

fn seed_visual_suite_combat_status_highlight(state: &mut PlayState) {
    state.pending_combat_actor_slot = Some(0);
    state.active_player = Some(0);
    state.combat_actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);
    state.message = "Combat status highlight".to_string();
}

fn seed_surface_view_class_gallery(state: &mut PlayState, mode: ViewOverlayMode) {
    state.player.x = TOWN_GRID_SIDE / 2;
    state.player.y = TOWN_GRID_SIDE / 2;
    state.grid = vec![0; TOWN_GRID_SIDE * TOWN_GRID_SIDE];
    for (index, tile) in SURFACE_VIEW_CLASS_GALLERY_TILES.iter().enumerate() {
        state.grid[4 * TOWN_GRID_SIDE + 4 + index] = *tile;
    }
    state.active_view_overlay = None;
    state.sync_player_object();
    state.mark_visibility_dirty();
    match mode {
        ViewOverlayMode::GemView => {
            state.gems = 1;
            state.view_gem();
        }
        ViewOverlayMode::PeerSpell => {
            state.activate_peer_view_overlay();
        }
        ViewOverlayMode::XRaySpell => {
            state.activate_x_ray_view_overlay();
        }
        ViewOverlayMode::SurfaceLook | ViewOverlayMode::BritanniaOverview => {
            unreachable!("surface view class gallery uses local surface-view modes")
        }
    }
}

fn push_visual_surface_view_class_gallery_reports(
    game_dir: &Path,
    out_dir: &Path,
    atlas: &TileAtlas,
    font: &FixedCellFont,
    reports: &mut Vec<VisualFrameReport>,
) -> io::Result<()> {
    for (label, mode) in [
        ("surface-view-class-gallery", ViewOverlayMode::GemView),
        ("peer-view-class-gallery", ViewOverlayMode::PeerSpell),
        ("x-ray-view-class-gallery", ViewOverlayMode::XRaySpell),
    ] {
        let mut state = PlayState::load_scene(
            game_dir,
            PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
        )?;
        seed_surface_view_class_gallery(&mut state, mode);
        reports.push(write_visual_play_report(
            out_dir,
            label,
            "visual surface view class gallery frame",
            &mut state,
            atlas,
            font,
        )?);
    }
    Ok(())
}

fn push_visual_combat_gallery_reports(
    game_dir: &Path,
    out_dir: &Path,
    atlas: &TileAtlas,
    font: &FixedCellFont,
    reports: &mut Vec<VisualFrameReport>,
) -> io::Result<()> {
    let bank = load_brit_cbt(game_dir)?;
    for arena_index in 0..BRIT_CBT_RECORDS {
        let record = bank.record(arena_index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("BRIT.CBT has no outdoor arena record {arena_index}"),
            )
        })?;
        let type_byte = 0x40 + (arena_index as u8) * 4;
        let trigger = ActiveObject {
            type_byte,
            tile: 0xc0,
            x: 0,
            y: 0,
            z: WorldPlane::Britannia.save_floor(),
            phase: STEADY_PHASE,
            aux1: 0,
            aux3: 0,
        };
        let setup = terrain_combat_setup_from_record(WorldPlane::Britannia, trigger, record)?;
        let replacement_tile = terrain_combat_raw_replacement_tile_for_arena(arena_index);
        let replacement_rolls = vec![0; 16];
        let mut instance =
            terrain_combat_instance_from_setup(&setup, 16, replacement_tile, &replacement_rolls)?;

        let mut state = PlayState::load_scene(
            game_dir,
            PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
        )?;
        state.populate_combat_party_at_placement_slots(
            &mut instance.active_objects,
            &mut instance.actors,
            trigger.z,
            &setup.placement_slots,
            usize::from(instance.placed_count),
        );
        state.enter_combat_frame_with_terrain(
            instance.active_objects,
            instance.actors,
            setup.terrain,
        )?;
        validate_visual_outdoor_combat_gallery_state(arena_index, &setup, &state)?;
        reports.push(write_visual_play_report(
            out_dir,
            &format!("combat-arena-{arena_index:02}"),
            "combat outdoor arena replacement gallery",
            &mut state,
            atlas,
            font,
        )?);
    }

    let dungeon_bank = load_dungeon_cbt(game_dir)?;
    for arena_index in 0..DUNGEON_CBT_RECORDS {
        let record = dungeon_bank.record(arena_index).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("DUNGEON.CBT has no dungeon-room arena record {arena_index}"),
            )
        })?;
        let setup = dungeon_room_combat_setup_from_record_for_entry(arena_index, record, 0, false);
        let mut instance = dungeon_room_combat_instance_from_setup(&setup, 0);
        let placed_count = instance.placed_count;
        let mut state = PlayState::load_scene(
            game_dir,
            PlayOptions {
                target: PlayTarget::Dungeon(DungeonScene::new(0x21).expect("Deceit is valid")),
                ..PlayOptions::default()
            },
        )?;
        seed_visual_combat_gallery_party(&mut state);
        state.populate_dungeon_room_combat_party(
            &mut instance.active_objects,
            &mut instance.actors,
            0,
            &setup.party_positions,
        );
        state.enter_combat_frame_with_terrain(
            instance.active_objects,
            instance.actors,
            setup.terrain,
        )?;
        validate_visual_dungeon_combat_gallery_state(arena_index, &setup, placed_count, &state)?;
        reports.push(write_visual_play_report(
            out_dir,
            &format!("dungeon-combat-arena-{arena_index:03}"),
            "combat dungeon-room arena gallery",
            &mut state,
            atlas,
            font,
        )?);
    }

    let mut marker_state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            ..PlayOptions::default()
        },
    )?;
    seed_visual_combat_marker_gallery(&mut marker_state)?;
    reports.push(write_visual_play_report(
        out_dir,
        "combat-marker-gallery",
        "combat death and marker gallery",
        &mut marker_state,
        atlas,
        font,
    )?);
    Ok(())
}

fn seed_visual_combat_gallery_party(state: &mut PlayState) {
    state.party = (0..COMBAT_PARTY_ACTOR_SLOTS)
        .map(|slot| route_visual_party_member(slot as u8, b'A', b'G', 30, 30))
        .collect();
    state.party_names = default_party_names(COMBAT_PARTY_ACTOR_SLOTS);
    state.party_experience = default_party_experience(COMBAT_PARTY_ACTOR_SLOTS);
    state.party_stay_counters = default_party_stay_counters(COMBAT_PARTY_ACTOR_SLOTS);
    state.party_strengths = vec![30; COMBAT_PARTY_ACTOR_SLOTS];
    state.party_intelligence = default_party_intelligence(COMBAT_PARTY_ACTOR_SLOTS);
    state.party_equipment = default_party_equipment(COMBAT_PARTY_ACTOR_SLOTS);
    state.party_roster = default_party_roster(COMBAT_PARTY_ACTOR_SLOTS);
}

fn validate_visual_outdoor_combat_gallery_state(
    arena_index: usize,
    setup: &TerrainCombatSetup,
    state: &PlayState,
) -> io::Result<()> {
    validate_visual_combat_gallery_base(
        state,
        &setup.terrain,
        &format!("outdoor arena {arena_index:02}"),
    )?;
    if setup.arena_index != arena_index || setup.placement_slots.len() != CBT_PLACEMENT_SLOT_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("outdoor arena {arena_index:02} setup metadata mismatch"),
        ));
    }
    validate_visual_combat_party_slots(
        state,
        WorldPlane::Britannia.save_floor(),
        &TERRAIN_COMBAT_PARTY_POSITIONS,
        &format!("outdoor arena {arena_index:02} party"),
    )?;

    let replacement_tile = terrain_combat_raw_replacement_tile_for_arena(arena_index);
    for spawn_index in 0..16 {
        let actor_slot = COMBAT_PARTY_ACTOR_SLOTS + spawn_index;
        let Some(placement) = setup.placement_slots.get(spawn_index).copied() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("outdoor arena {arena_index:02} missing placement {spawn_index}"),
            ));
        };
        if placement.slot != spawn_index
            || usize::from(placement.x) >= COMBAT_ARENA_SIDE
            || usize::from(placement.y) >= COMBAT_ARENA_SIDE
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "outdoor arena {arena_index:02} placement {spawn_index} is invalid at ({},{})",
                    placement.x, placement.y
                ),
            ));
        }
        let expected_tile = terrain_combat_tile_for_spawn_index(
            spawn_index as u8,
            16,
            setup.base_tile,
            replacement_tile,
            0,
        );
        validate_visual_combat_actor_slot(
            state,
            actor_slot,
            expected_tile,
            usize::from(placement.x),
            usize::from(placement.y),
            WorldPlane::Britannia.save_floor(),
            &format!("outdoor arena {arena_index:02} spawn {spawn_index}"),
        )?;
    }
    Ok(())
}

fn validate_visual_dungeon_combat_gallery_state(
    arena_index: usize,
    setup: &DungeonRoomCombatSetup,
    placed_count: u8,
    state: &PlayState,
) -> io::Result<()> {
    validate_visual_combat_gallery_base(
        state,
        &setup.terrain,
        &format!("dungeon arena {arena_index:03}"),
    )?;
    if setup.arena_index != arena_index || setup.placement_slots.len() != CBT_PLACEMENT_SLOT_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("dungeon arena {arena_index:03} setup metadata mismatch"),
        ));
    }
    validate_visual_combat_party_slots(
        state,
        0,
        &setup.party_positions,
        &format!("dungeon arena {arena_index:03} party"),
    )?;
    for (slot, (x, y)) in setup.party_positions.iter().copied().enumerate() {
        if usize::from(x) >= COMBAT_ARENA_SIDE || usize::from(y) >= COMBAT_ARENA_SIDE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("dungeon arena {arena_index:03} party slot {slot} is outside arena"),
            ));
        }
    }

    let placed_count = usize::from(placed_count);
    if placed_count > COMBAT_ACTOR_SLOTS - COMBAT_PARTY_ACTOR_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("dungeon arena {arena_index:03} placed too many sources: {placed_count}"),
        ));
    }
    for source_index in 0..placed_count {
        let slot = COMBAT_PARTY_ACTOR_SLOTS + source_index;
        let Some(object) = state.active_objects.get(slot).copied() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("dungeon arena {arena_index:03} missing active-object slot {slot}"),
            ));
        };
        if object.is_empty()
            || object.x >= COMBAT_ARENA_SIDE
            || object.y >= COMBAT_ARENA_SIDE
            || object.z != 0
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "dungeon arena {arena_index:03} source {source_index} invalid object at ({},{},{})",
                    object.x, object.y, object.z
                ),
            ));
        }
        let actor = state.combat_actors.get(slot).copied().unwrap_or_default();
        if !actor.is_empty() {
            validate_visual_combat_actor_link(
                state,
                slot,
                object.tile,
                object.x,
                object.y,
                0,
                &format!("dungeon arena {arena_index:03} source {source_index}"),
            )?;
        }
    }
    Ok(())
}

fn validate_visual_combat_gallery_base(
    state: &PlayState,
    expected_terrain: &[[u8; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
    label: &str,
) -> io::Result<()> {
    if !state.combat_active || state.combat_terrain != *expected_terrain {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} visual combat gallery did not preserve combat terrain"),
        ));
    }
    if state.active_objects.len() < COMBAT_ACTOR_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} visual combat gallery has too few active objects"),
        ));
    }
    Ok(())
}

fn validate_visual_combat_party_slots(
    state: &PlayState,
    z: i8,
    positions: &[(u8, u8); COMBAT_PARTY_ACTOR_SLOTS],
    label: &str,
) -> io::Result<()> {
    for (slot, (x, y)) in positions.iter().copied().enumerate() {
        if !state
            .party
            .get(slot)
            .copied()
            .is_some_and(PartyMember::conscious)
        {
            continue;
        }
        validate_visual_combat_actor_link(
            state,
            slot,
            PLAYER_TILE,
            usize::from(x),
            usize::from(y),
            z,
            &format!("{label} slot {slot}"),
        )?;
    }
    Ok(())
}

fn validate_visual_combat_actor_slot(
    state: &PlayState,
    slot: usize,
    expected_tile: u8,
    x: usize,
    y: usize,
    z: i8,
    label: &str,
) -> io::Result<()> {
    validate_visual_combat_actor_link(state, slot, expected_tile, x, y, z, label)?;
    let actor = state.combat_actors.get(slot).copied().unwrap_or_default();
    if !combat_actor_is_active_not_dead(actor) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} combat actor is not active"),
        ));
    }
    Ok(())
}

fn validate_visual_combat_actor_link(
    state: &PlayState,
    slot: usize,
    expected_tile: u8,
    x: usize,
    y: usize,
    z: i8,
    label: &str,
) -> io::Result<()> {
    let Some(object) = state.active_objects.get(slot).copied() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{label} missing active-object slot {slot}"),
        ));
    };
    let actor = state.combat_actors.get(slot).copied().unwrap_or_default();
    if object.tile != expected_tile
        || object.type_byte != expected_tile
        || object.x != x
        || object.y != y
        || object.z != z
        || actor.active_object_slot as usize != slot
        || usize::from(actor.x) != x
        || usize::from(actor.y) != y
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{label} expected tile 0x{expected_tile:02x} at ({x},{y},{z}) linked to actor slot {slot}, got object tile 0x{:02x} type 0x{:02x} at ({},{},{}) and actor ({},{}) -> object {}",
                object.tile,
                object.type_byte,
                object.x,
                object.y,
                object.z,
                actor.x,
                actor.y,
                actor.active_object_slot
            ),
        ));
    }
    Ok(())
}

fn seed_visual_combat_marker_gallery(state: &mut PlayState) -> io::Result<()> {
    seed_visual_combat_gallery_party(state);
    state.active_player = Some(0);

    let mut terrain = [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    terrain[6][6] = COMBAT_GARGOYLE_DEATH_TERRAIN_TILE;

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([20, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 8]);

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    active_objects[0] = visual_route_combat_active_object(0x4c, 5, 8, 0);
    active_objects[1] = visual_route_combat_active_object(COMBAT_PARTY_CORPSE_TILE, 1, 2, 0);
    active_objects[2] = visual_route_combat_active_object(COMBAT_DEFAULT_DEATH_DROP_TILE, 2, 2, 0);
    active_objects[3] = visual_route_combat_active_object(COMBAT_VANISH_DEATH_MARKER_TILE, 3, 2, 0);
    active_objects[4] = visual_route_combat_active_object(COMBAT_GAZER_DEATH_MARKER_TILE, 4, 2, 0);
    active_objects[5] = visual_route_combat_active_object(COMBAT_DEFAULT_DEATH_DROP_TILE, 6, 6, 0);
    active_objects[6] = visual_route_combat_active_object(COMBAT_FIELD_KIND_POISON, 7, 3, 0);
    active_objects[7] = visual_route_combat_active_object(COMBAT_FIELD_KIND_SLEEP, 8, 3, 0);
    active_objects[8] = visual_route_combat_active_object(COMBAT_FIELD_KIND_FIRE, 7, 4, 0);
    active_objects[9] = visual_route_combat_active_object(COMBAT_FIELD_KIND_ENERGY, 8, 4, 0);

    state.enter_combat_frame_with_terrain(active_objects, actors, terrain)?;
    state.combat_cursor_blink = true;
    state.combat_secondary_marker = Some((3, 4));
    state.message = "Combat marker gallery".to_string();
    validate_visual_combat_marker_gallery_state(state)?;
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CombatMarkerGalleryCell {
    slot: usize,
    tile: u8,
    x: usize,
    y: usize,
    label: &'static str,
}

const COMBAT_MARKER_GALLERY_CELLS: [CombatMarkerGalleryCell; 10] = [
    CombatMarkerGalleryCell {
        slot: 0,
        tile: 0x4c,
        x: 5,
        y: 8,
        label: "controlled-party",
    },
    CombatMarkerGalleryCell {
        slot: 1,
        tile: COMBAT_PARTY_CORPSE_TILE,
        x: 1,
        y: 2,
        label: "party-corpse",
    },
    CombatMarkerGalleryCell {
        slot: 2,
        tile: COMBAT_DEFAULT_DEATH_DROP_TILE,
        x: 2,
        y: 2,
        label: "default-drop",
    },
    CombatMarkerGalleryCell {
        slot: 3,
        tile: COMBAT_VANISH_DEATH_MARKER_TILE,
        x: 3,
        y: 2,
        label: "vanish",
    },
    CombatMarkerGalleryCell {
        slot: 4,
        tile: COMBAT_GAZER_DEATH_MARKER_TILE,
        x: 4,
        y: 2,
        label: "gazer",
    },
    CombatMarkerGalleryCell {
        slot: 5,
        tile: COMBAT_DEFAULT_DEATH_DROP_TILE,
        x: 6,
        y: 6,
        label: "gargoyle-terrain-drop",
    },
    CombatMarkerGalleryCell {
        slot: 6,
        tile: COMBAT_FIELD_KIND_POISON,
        x: 7,
        y: 3,
        label: "poison-field",
    },
    CombatMarkerGalleryCell {
        slot: 7,
        tile: COMBAT_FIELD_KIND_SLEEP,
        x: 8,
        y: 3,
        label: "sleep-field",
    },
    CombatMarkerGalleryCell {
        slot: 8,
        tile: COMBAT_FIELD_KIND_FIRE,
        x: 7,
        y: 4,
        label: "fire-field",
    },
    CombatMarkerGalleryCell {
        slot: 9,
        tile: COMBAT_FIELD_KIND_ENERGY,
        x: 8,
        y: 4,
        label: "energy-field",
    },
];

fn validate_visual_combat_marker_gallery_state(state: &PlayState) -> io::Result<()> {
    if !state.combat_active {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "combat marker gallery did not enter combat",
        ));
    }
    if state.active_player != Some(0) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "combat marker gallery active player is not slot 0",
        ));
    }
    if !state.combat_cursor_blink {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "combat marker gallery cursor blink is not visible",
        ));
    }
    if state.combat_secondary_marker != Some((3, 4)) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "combat marker gallery secondary marker is not at (3,4)",
        ));
    }
    if state.combat_terrain[6][6] != COMBAT_GARGOYLE_DEATH_TERRAIN_TILE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "combat marker gallery missing Gargoyle death terrain cell",
        ));
    }
    for cell in COMBAT_MARKER_GALLERY_CELLS {
        let Some(object) = state.active_objects.get(cell.slot) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("combat marker gallery missing slot {}", cell.slot),
            ));
        };
        if object.tile != cell.tile || object.x != cell.x || object.y != cell.y {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "combat marker gallery {} slot {} expected tile 0x{:02x} at ({},{}), got tile 0x{:02x} at ({},{})",
                    cell.label,
                    cell.slot,
                    cell.tile,
                    cell.x,
                    cell.y,
                    object.tile,
                    object.x,
                    object.y
                ),
            ));
        }
    }
    Ok(())
}

struct VisualRouteSuiteCase {
    label: &'static str,
    frame_kind: &'static str,
    options: PlayOptions,
    script: &'static [&'static str],
    configure: Option<fn(&mut PlayState)>,
}

#[derive(Clone)]
struct VisualKeyRouteSuiteCase {
    label: &'static str,
    frame_kind: &'static str,
    options: PlayOptions,
    steps: &'static [VisualKeyStep],
    configure: Option<fn(&mut PlayState)>,
}

#[derive(Clone, Copy)]
struct VisualKeyStep {
    label: &'static str,
    key: KeyCode,
    shift: bool,
    control: bool,
}

impl VisualKeyStep {
    const fn key(label: &'static str, key: KeyCode) -> Self {
        Self {
            label,
            key,
            shift: false,
            control: false,
        }
    }

    const fn control(label: &'static str, key: KeyCode) -> Self {
        Self {
            label,
            key,
            shift: false,
            control: true,
        }
    }
}

const VISUAL_KEY_WORLD_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_d", KeyCode::KeyD),
    VisualKeyStep::key("space", KeyCode::Space),
    VisualKeyStep::control("ctrl_s", KeyCode::KeyS),
];
const VISUAL_KEY_SAVE_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_q", KeyCode::KeyQ),
    VisualKeyStep::key("key_n", KeyCode::KeyN),
];
const VISUAL_KEY_TALK_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_j", KeyCode::KeyJ),
    VisualKeyStep::key("key_o", KeyCode::KeyO),
    VisualKeyStep::key("key_b", KeyCode::KeyB),
    VisualKeyStep::key("backspace", KeyCode::Backspace),
    VisualKeyStep::key("key_b", KeyCode::KeyB),
    VisualKeyStep::key("enter", KeyCode::Enter),
];
const VISUAL_KEY_SHRINE_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_m", KeyCode::KeyM),
    VisualKeyStep::key("key_a", KeyCode::KeyA),
    VisualKeyStep::key("key_h", KeyCode::KeyH),
    VisualKeyStep::key("key_m", KeyCode::KeyM),
    VisualKeyStep::key("enter", KeyCode::Enter),
];
const VISUAL_KEY_SHOP_QUANTITY_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_a", KeyCode::KeyA),
    VisualKeyStep::key("digit_1", KeyCode::Digit1),
    VisualKeyStep::key("digit_2", KeyCode::Digit2),
    VisualKeyStep::key("backspace", KeyCode::Backspace),
    VisualKeyStep::key("digit_2", KeyCode::Digit2),
    VisualKeyStep::key("enter", KeyCode::Enter),
];
const VISUAL_KEY_ESCAPE_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_m", KeyCode::KeyM),
    VisualKeyStep::key("key_a", KeyCode::KeyA),
    VisualKeyStep::key("key_h", KeyCode::KeyH),
    VisualKeyStep::key("escape", KeyCode::Escape),
];
const VISUAL_KEY_DIRECTION_PROMPT_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_l", KeyCode::KeyL),
    VisualKeyStep::key("key_d", KeyCode::KeyD),
    VisualKeyStep::key("key_a", KeyCode::KeyA),
    VisualKeyStep::key("key_d", KeyCode::KeyD),
    VisualKeyStep::key("key_g", KeyCode::KeyG),
    VisualKeyStep::key("key_d", KeyCode::KeyD),
    VisualKeyStep::key("key_o", KeyCode::KeyO),
    VisualKeyStep::key("key_d", KeyCode::KeyD),
    VisualKeyStep::key("key_p", KeyCode::KeyP),
    VisualKeyStep::key("key_d", KeyCode::KeyD),
    VisualKeyStep::key("key_s", KeyCode::KeyS),
    VisualKeyStep::key("key_d", KeyCode::KeyD),
];
const VISUAL_KEY_YELL_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_y", KeyCode::KeyY),
    VisualKeyStep::key("key_f", KeyCode::KeyF),
    VisualKeyStep::key("key_a", KeyCode::KeyA),
    VisualKeyStep::key("key_l", KeyCode::KeyL),
    VisualKeyStep::key("key_l", KeyCode::KeyL),
    VisualKeyStep::key("key_a", KeyCode::KeyA),
    VisualKeyStep::key("backspace", KeyCode::Backspace),
    VisualKeyStep::key("key_x", KeyCode::KeyX),
    VisualKeyStep::key("enter", KeyCode::Enter),
];
const VISUAL_KEY_READY_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_r", KeyCode::KeyR),
    VisualKeyStep::key("digit_1", KeyCode::Digit1),
    VisualKeyStep::key("enter", KeyCode::Enter),
    VisualKeyStep::key("enter", KeyCode::Enter),
    VisualKeyStep::key("space", KeyCode::Space),
];
const VISUAL_KEY_STATS_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_z", KeyCode::KeyZ),
    VisualKeyStep::key("space", KeyCode::Space),
];
const VISUAL_KEY_USE_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_u", KeyCode::KeyU),
    VisualKeyStep::key("enter", KeyCode::Enter),
];
const VISUAL_KEY_MIX_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_m", KeyCode::KeyM),
    VisualKeyStep::key("escape", KeyCode::Escape),
];
const VISUAL_KEY_REST_WATCH_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_h", KeyCode::KeyH),
    VisualKeyStep::key("digit_1", KeyCode::Digit1),
    VisualKeyStep::key("key_y", KeyCode::KeyY),
    VisualKeyStep::key("digit_2", KeyCode::Digit2),
];
const VISUAL_KEY_NEW_ORDER_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_n", KeyCode::KeyN),
    VisualKeyStep::key("digit_2", KeyCode::Digit2),
    VisualKeyStep::key("digit_3", KeyCode::Digit3),
];
const VISUAL_KEY_DUNGEON_CONTROL_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("key_w", KeyCode::KeyW),
    VisualKeyStep::key("key_a", KeyCode::KeyA),
    VisualKeyStep::key("key_d", KeyCode::KeyD),
    VisualKeyStep::key("key_s", KeyCode::KeyS),
];
const VISUAL_KEY_DIRECTIONAL_KEY_STEPS: &[VisualKeyStep] = &[
    VisualKeyStep::key("arrow_right", KeyCode::ArrowRight),
    VisualKeyStep::key("arrow_down", KeyCode::ArrowDown),
    VisualKeyStep::key("numpad_4", KeyCode::Numpad4),
    VisualKeyStep::key("home", KeyCode::Home),
];

fn visual_key_route_suite_cases() -> Vec<VisualKeyRouteSuiteCase> {
    vec![
        VisualKeyRouteSuiteCase {
            label: "route-key-world-movement-pass-music",
            frame_kind: "visual key route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_WORLD_STEPS,
            configure: None,
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-save-refusal",
            frame_kind: "visual key route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_SAVE_STEPS,
            configure: None,
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-talk-keyword-buffer",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions::default(),
            steps: VISUAL_KEY_TALK_STEPS,
            configure: Some(seed_visual_key_route_conversation),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-shrine-mantra-buffer",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_SHRINE_STEPS,
            configure: Some(seed_visual_key_route_shrine),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-shop-quantity-buffer",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions::default(),
            steps: VISUAL_KEY_SHOP_QUANTITY_STEPS,
            configure: Some(seed_visual_key_route_reagent_shop),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-prompt-escape-cancel",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_ESCAPE_STEPS,
            configure: Some(seed_visual_key_route_shrine),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-world-direction-prompts",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_DIRECTION_PROMPT_STEPS,
            configure: None,
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-yell-buffer",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_YELL_STEPS,
            configure: None,
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-ready-picker",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions::default(),
            steps: VISUAL_KEY_READY_STEPS,
            configure: Some(seed_visual_key_route_ready_picker),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-z-stats-picker",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions::default(),
            steps: VISUAL_KEY_STATS_STEPS,
            configure: None,
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-use-picker",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions::default(),
            steps: VISUAL_KEY_USE_STEPS,
            configure: Some(seed_visual_key_route_use_picker),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-mix-prompt",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions::default(),
            steps: VISUAL_KEY_MIX_STEPS,
            configure: Some(seed_visual_key_route_mix_prompt),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-rest-watch-prompt",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_REST_WATCH_STEPS,
            configure: Some(seed_visual_key_route_rest_watch),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-new-order-picker",
            frame_kind: "visual key route prompt frame",
            options: PlayOptions::default(),
            steps: VISUAL_KEY_NEW_ORDER_STEPS,
            configure: Some(seed_visual_key_route_new_order),
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-dungeon-controls",
            frame_kind: "visual key route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(
                    DungeonScene::new(0x21).expect("dungeon scene is valid"),
                ),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_DUNGEON_CONTROL_STEPS,
            configure: None,
        },
        VisualKeyRouteSuiteCase {
            label: "route-key-directional-keys",
            frame_kind: "visual key route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            steps: VISUAL_KEY_DIRECTIONAL_KEY_STEPS,
            configure: None,
        },
    ]
}

fn visual_route_suite_cases() -> Vec<VisualRouteSuiteCase> {
    let castle = Scene::new(0x11).expect("castle scene is valid");
    let dungeon = DungeonScene::new(0x21).expect("dungeon scene is valid");
    let doom = DungeonScene::new(0x28).expect("doom dungeon scene is valid");
    let ship_transport = TransportState::Ship {
        type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
        tile: FIRST_PLAYABLE_FRIGATE_TILE,
        sails_hoisted: false,
        hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
        skiffs: 2,
    };
    let world = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    let underworld = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        ..PlayOptions::default()
    };
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
    let mut castle_light_decay = PlayOptions::default();
    castle_light_decay.light_spell_counter = 2;
    let shadowlord_town = Scene::new(SCENE_MOONGLOW).expect("Shadowlord hideout town is valid");
    let stonegate = Scene::new(SCENE_STONEGATE).expect("Stonegate scene is valid");
    let mut gate_travel_to_underworld = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    seed_visual_route_gate_travel_resources(&mut gate_travel_to_underworld);
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
    seed_visual_route_gate_travel_resources(&mut gate_travel_to_castle);
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
    seed_visual_route_gate_travel_resources(&mut gate_travel_invalid_slot);
    let mut gate_travel_shipboard_refusal = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        transport: ship_transport,
        ..PlayOptions::default()
    };
    seed_visual_route_gate_travel_resources(&mut gate_travel_shipboard_refusal);
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
    let narrative_gate_open = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((NARRATIVE_GATE_X as usize, NARRATIVE_GATE_Y as usize)),
        ..PlayOptions::default()
    };
    let mut narrative_gate_ordained_block = narrative_gate_open.clone();
    narrative_gate_ordained_block.shrine_ordained_mask = 0b0000_0001;
    let fixed_hidden_single_use = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        start: Some((79, 64)),
        facing: Some(Direction::East),
        ..PlayOptions::default()
    };
    let fixed_hidden_underworld_stack = PlayOptions {
        target: PlayTarget::World(WorldPlane::Underworld),
        ..PlayOptions::default()
    };
    let minoc = Scene::new(0x05).expect("Minoc scene is valid");
    let fixed_hidden_daily = PlayOptions {
        target: PlayTarget::Town(minoc),
        clock: GameClock::new(5, 0).expect("05:00 is a valid game-clock time"),
        ..PlayOptions::default()
    };
    let blackthorn_fixed_hidden_key_cache = PlayOptions {
        target: PlayTarget::Town(Scene::new(18).expect("Blackthorn castle scene is valid")),
        floor: -1,
        ..PlayOptions::default()
    };
    let mut wooden_box = PlayOptions::default();
    wooden_box.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
    let hourly_provision_poison = PlayOptions {
        target: PlayTarget::Town(castle),
        clock: GameClock::with_date(139, 4, 5, 5, 59).expect("05:59 is valid"),
        food: 10,
        party: vec![
            route_visual_party_member(0, b'A', b'G', 12, 20),
            route_visual_party_member(1, b'F', b'P', 12, 20),
            route_visual_party_member(2, b'M', b'S', 12, 20),
            route_visual_party_member(3, b'D', b'D', 0, 20),
            route_visual_party_member(4, b'B', b'A', 0, 20),
        ],
        ..PlayOptions::default()
    };
    let hourly_poison_starvation = PlayOptions {
        target: PlayTarget::Town(castle),
        clock: GameClock::with_date(139, 4, 5, 8, 59).expect("08:59 is valid"),
        food: 0,
        party: vec![
            route_visual_party_member(0, b'A', b'P', 20, 20),
            route_visual_party_member(1, b'F', b'G', 20, 20),
            route_visual_party_member(2, b'M', b'D', 0, 20),
        ],
        ..PlayOptions::default()
    };
    let mut hourly_ring_regeneration = PlayOptions {
        target: PlayTarget::Town(castle),
        clock: GameClock::with_date(139, 4, 5, 7, 59).expect("07:59 is valid"),
        food: 99,
        party: vec![route_visual_party_member(0, b'A', b'G', 19, 20)],
        party_equipment: default_party_equipment(1),
        ..PlayOptions::default()
    };
    hourly_ring_regeneration.party_equipment[0][EQUIP_SLOT_RING] =
        EQUIPMENT_ID_RING_REGENERATION as u8;
    let dungeon_rest_no_direct_recovery = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        clock: GameClock::new(8, 0).expect("08:00 is valid"),
        torch_counter: 9,
        party: vec![
            route_visual_party_member(0, b'A', b'G', 5, 20),
            route_visual_party_member(1, b'F', b'S', 3, 20),
            route_visual_party_member(2, b'M', b'D', 0, 20),
        ],
        ..PlayOptions::default()
    };
    let dungeon_long_camp_recovery = PlayOptions {
        target: PlayTarget::Dungeon(dungeon),
        floor: 0,
        clock: GameClock::new(8, 0).expect("08:00 is valid"),
        food: 99,
        torch_counter: 9,
        ..PlayOptions::default()
    };
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
        target: PlayTarget::Town(
            Scene::new(SCENE_SERPENTS_HOLD).expect("Serpent's Hold scene is valid"),
        ),
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
    let fallax_seal =
        word_of_power_seal_for_word("FALLAX").expect("FALLAX Word-of-Power seal row is public");
    let veramocor_seal = word_of_power_seal_for_word("VERAMOCOR")
        .expect("VERAMOCOR Word-of-Power seal row is public");
    let mut cases = vec![
        VisualRouteSuiteCase {
            label: "route-world-movement",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: &["d", "."],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-move-pass-idle",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: &["d", "."],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-pass-and-idle",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-town-status-modal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Z"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-z-stats-modal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Z"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-town-view-overlay",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["v", "."],
            configure: Some(|state| {
                state.gems = 1;
            }),
        },
        VisualRouteSuiteCase {
            label: "route-world-view-overlay",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["v", "."],
            configure: Some(|state| {
                state.gems = 1;
            }),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-view-overlay",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["v", "."],
            configure: Some(|state| {
                state.gems = 1;
            }),
        },
        VisualRouteSuiteCase {
            label: "route-castle-view-overlay",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["v", "."],
            configure: Some(|state| {
                state.gems = 1;
            }),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-view-overlay",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["v", "."],
            configure: Some(|state| {
                state.gems = 1;
            }),
        },
        VisualRouteSuiteCase {
            label: "route-castle-peer-overlay",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1IQW", "."],
            configure: Some(seed_visual_route_peer_spell),
        },
        VisualRouteSuiteCase {
            label: "route-castle-x-ray-overlay",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1AWY", "."],
            configure: Some(seed_visual_route_x_ray_spell),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-look",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["l6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-look-pass",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["l6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-look-pass",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["l6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-spyglass-chunk-map",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                clock: GameClock::new(20, 0).expect("20:00 is a valid game-clock time"),
                ..PlayOptions::default()
            },
            script: &["USP"],
            configure: Some(|state| {
                state.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
            }),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-utility-use-items",
            frame_kind: "visual route world frame",
            options: britannia_utility_use,
            script: &["UW", "US", "UC"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-ship-hms-cape-plans-use",
            frame_kind: "visual route world frame",
            options: hms_cape_plans,
            script: &["UP"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-create-food-cast",
            frame_kind: "visual route world frame",
            options: create_food,
            script: &["C1IMX"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-gate-travel-world-to-underworld",
            frame_kind: "visual route world frame",
            options: gate_travel_to_underworld.clone(),
            script: &["C1PRV1"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-reload-gate-travel-underworld-pass",
            frame_kind: "visual route world frame",
            options: gate_travel_to_underworld,
            script: &["C1PRV1", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-gate-travel-world-to-castle",
            frame_kind: "visual route town frame",
            options: gate_travel_to_castle,
            script: &["C1PRV2"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-gate-travel-invalid-slot-refusal",
            frame_kind: "visual route world frame",
            options: gate_travel_invalid_slot,
            script: &["C1PRV4"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-gate-travel-shipboard-refusal",
            frame_kind: "visual route world frame",
            options: gate_travel_shipboard_refusal,
            script: &["C1PRV2"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-natural-moongate-trammel-gate-travel",
            frame_kind: "visual route world frame",
            options: natural_moongate_trammel,
            script: &["idle:1"],
            configure: Some(seed_visual_route_natural_moongate),
        },
        VisualRouteSuiteCase {
            label: "route-natural-moongate-empty-slot-clears-live-tile",
            frame_kind: "visual route world frame",
            options: natural_moongate_empty_slot,
            script: &["idle:1"],
            configure: Some(seed_visual_route_natural_moongate),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-chasm-fall-to-underworld",
            frame_kind: "visual route world frame",
            options: chasm_fall.clone(),
            script: &["s"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-reload-chasm-underworld-pass",
            frame_kind: "visual route world frame",
            options: chasm_fall,
            script: &["s", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-whirlpool-forced-underworld",
            frame_kind: "visual route world frame",
            options: whirlpool_forced_underworld,
            script: &["setup:whirlpool-engagement"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-fixed-narrative-gate-open-south-step",
            frame_kind: "visual route world frame",
            options: narrative_gate_open,
            script: &["empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-fixed-narrative-gate-ordained-block",
            frame_kind: "visual route world frame",
            options: narrative_gate_ordained_block,
            script: &["empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-hole-up-rest",
            frame_kind: "visual route world frame",
            options: world.clone(),
            script: &["H1"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-save-refusal",
            frame_kind: "visual route world frame",
            options: world.clone(),
            script: &["Q", "N"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-dispatcher-refusals",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: &["B"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-fixed-hidden-single-use-search-get",
            frame_kind: "visual route world frame",
            options: fixed_hidden_single_use,
            script: &["S6", "G6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-underworld-pass-and-idle",
            frame_kind: "visual route world frame",
            options: underworld,
            script: &["empty", "idle:1"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-underworld-fixed-hidden-stack-search-get-search",
            frame_kind: "visual route world frame",
            options: fixed_hidden_underworld_stack.clone(),
            script: &["S6", "G6", "S6"],
            configure: Some(seed_visual_route_underworld_fixed_hidden_stack),
        },
        VisualRouteSuiteCase {
            label: "route-reload-underworld-fixed-hidden-stack-search-get-search",
            frame_kind: "visual route world frame",
            options: fixed_hidden_underworld_stack,
            script: &["S6", "G6", "S6"],
            configure: Some(seed_visual_route_underworld_fixed_hidden_stack),
        },
        VisualRouteSuiteCase {
            label: "route-blackthorn-fixed-hidden-zero-key-search",
            frame_kind: "visual route town frame",
            options: blackthorn_fixed_hidden_key_cache,
            script: &["S6"],
            configure: Some(seed_visual_route_blackthorn_fixed_hidden),
        },
        VisualRouteSuiteCase {
            label: "route-minoc-fixed-hidden-daily-search-get-repeat",
            frame_kind: "visual route town frame",
            options: fixed_hidden_daily.clone(),
            script: &["S6", "G6", "S6"],
            configure: Some(seed_visual_route_minoc_fixed_hidden_daily),
        },
        VisualRouteSuiteCase {
            label: "route-reload-minoc-fixed-hidden-daily-search-get-repeat",
            frame_kind: "visual route town frame",
            options: fixed_hidden_daily,
            script: &["S6", "G6", "S6"],
            configure: Some(seed_visual_route_minoc_fixed_hidden_daily),
        },
        VisualRouteSuiteCase {
            label: "route-castle-wooden-box-use",
            frame_kind: "visual route town frame",
            options: wooden_box,
            script: &["UB"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-save-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Q", "N"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-dispatcher-board-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-dispatcher-refusals",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-dispatcher-fire-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["F6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-command-workflow-overlays",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["MIL/0x80/1", "R1/26", "R1/26", "N23"],
            configure: Some(seed_visual_route_command_workflows),
        },
        VisualRouteSuiteCase {
            label: "route-castle-hourly-provision-poison-pass",
            frame_kind: "visual route town frame",
            options: hourly_provision_poison,
            script: &["empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-hourly-poison-starvation-pass",
            frame_kind: "visual route town frame",
            options: hourly_poison_starvation,
            script: &["empty"],
            configure: Some(seed_visual_route_hourly_poison_starvation),
        },
        VisualRouteSuiteCase {
            label: "route-castle-hourly-ring-regeneration-pass",
            frame_kind: "visual route town frame",
            options: hourly_ring_regeneration,
            script: &["empty"],
            configure: Some(seed_visual_route_hourly_ring_regeneration),
        },
        VisualRouteSuiteCase {
            label: "route-castle-talk-status-sleeping-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["T6"],
            configure: Some(seed_visual_route_talk_status_sleeping),
        },
        VisualRouteSuiteCase {
            label: "route-castle-talk-status-praying-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["T6"],
            configure: Some(seed_visual_route_talk_status_praying),
        },
        VisualRouteSuiteCase {
            label: "route-castle-native-stair-up-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["d"],
            configure: Some(seed_visual_route_native_stair_up),
        },
        VisualRouteSuiteCase {
            label: "route-castle-native-stair-down-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                floor: 1,
                ..PlayOptions::default()
            },
            script: &["d"],
            configure: Some(seed_visual_route_native_stair_down),
        },
        VisualRouteSuiteCase {
            label: "route-castle-native-stair-cross-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["w"],
            configure: Some(seed_visual_route_native_stair_cross),
        },
        VisualRouteSuiteCase {
            label: "route-debug-enter-castle",
            frame_kind: "visual route town frame",
            options: world_to_castle.clone(),
            script: &["e", "empty", "idle:1"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-debug-enter-castle-return-world",
            frame_kind: "visual route world frame",
            options: world_to_castle,
            script: &["e", "w", "idle:1"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-debug-enter-castle-from-underworld",
            frame_kind: "visual route town frame",
            options: underworld_to_castle,
            script: &["e", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-world-board-horse",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                facing: Some(Direction::East),
                ..PlayOptions::default()
            },
            script: &["B"],
            configure: Some(seed_visual_route_board_horse),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-board-horse-route",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                facing: Some(Direction::East),
                ..PlayOptions::default()
            },
            script: &["B"],
            configure: Some(seed_visual_route_board_horse),
        },
        VisualRouteSuiteCase {
            label: "route-reload-boarded-horse-pass",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                facing: Some(Direction::East),
                ..PlayOptions::default()
            },
            script: &["B", "empty"],
            configure: Some(seed_visual_route_board_horse),
        },
        VisualRouteSuiteCase {
            label: "route-ship-xit-launches-skiff",
            frame_kind: "visual route world frame",
            options: ship_xit.clone(),
            script: &["X", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-reload-ship-xit-skiff-pass",
            frame_kind: "visual route world frame",
            options: ship_xit,
            script: &["X", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-ship-hoist-and-sail-east",
            frame_kind: "visual route world frame",
            options: ship_sail,
            script: &["Y", "d", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-ship-broadside-fire",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                transport: ship_transport,
                ..PlayOptions::default()
            },
            script: &["F6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-ship-broadside-fire-route",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                transport: ship_transport,
                ..PlayOptions::default()
            },
            script: &["F6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-movement-search",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["w", "a", "S6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-search-focus-route",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["S6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-attack-direction-route",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["A", "6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-hole-up-rest",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["H1"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-hole-up-no-direct-recovery",
            frame_kind: "visual route dungeon frame",
            options: dungeon_rest_no_direct_recovery,
            script: &["H1"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-long-camp-recovery",
            frame_kind: "visual route dungeon frame",
            options: dungeon_long_camp_recovery,
            script: &["H6/4"],
            configure: Some(seed_visual_route_long_camp_recovery),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-heavy-door-variant-pass-through",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["."],
            configure: Some(seed_visual_route_dungeon_heavy_door_variant),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-ladder-down-up-route",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &[">", "<"],
            configure: Some(seed_visual_route_dungeon_ladder),
        },
        VisualRouteSuiteCase {
            label: "route-reload-dungeon-ladder-down-up",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &[">", "<"],
            configure: Some(seed_visual_route_dungeon_ladder),
        },
        VisualRouteSuiteCase {
            label: "route-reload-dungeon-ladder-down-up-route",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &[">", "<"],
            configure: Some(seed_visual_route_dungeon_ladder),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-surface-exit-return-world",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["K"],
            configure: Some(seed_visual_route_dungeon_surface_exit),
        },
        VisualRouteSuiteCase {
            label: "route-reload-dungeon-surface-exit-return-world",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["K", "empty"],
            configure: Some(seed_visual_route_dungeon_surface_exit),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-active-monster-attack-ambush",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["A"],
            configure: Some(seed_visual_route_dungeon_active_monster_attack),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-active-monster-contact-ambush",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["empty"],
            configure: Some(seed_visual_route_dungeon_active_monster_contact),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-ignite-torch",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 0,
                light_spell_counter: 0,
                ..PlayOptions::default()
            },
            script: &["I"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-sjog-underfoot-get",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["G"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-sjog-underfoot-jimmy",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["J"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-sjog-underfoot-open",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["O"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-debug-enter-dungeon",
            frame_kind: "visual route dungeon frame",
            options: world_to_dungeon,
            script: &["e", "Q", "N"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-exit-refusal",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["Q", "N"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-exit-confirm",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["Q", "Y"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-refusal-board",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["B"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-refusal-fire",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["F"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-local-buy-sell",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "N", "S", "1", "N"],
            configure: Some(seed_visual_route_arms_local),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-local-buy-sell-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "N", "S", "1", "N"],
            configure: Some(seed_visual_route_arms_local),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-iolos-bows-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_iolos_bows),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-naughty-nomaans-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_naughty_nomaans),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-arms-of-justice-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_arms_of_justice),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-darkwatch-armoury-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_darkwatch_armoury),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-paladins-protectorate-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_paladins_protectorate),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-north-star-armoury-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_north_star_armoury),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-buccaneers-booty-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_buccaneers_booty),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-shattered-shield-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_shattered_shield),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-siege-crafters-buy-first",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "A", "Y", "\x1b"],
            configure: Some(seed_visual_route_arms_siege_crafters),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-iolos-bows-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_iolos_bows),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-naughty-nomaans-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_naughty_nomaans),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-arms-of-justice-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_arms_of_justice),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-darkwatch-armoury-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_darkwatch_armoury),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-paladins-protectorate-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_paladins_protectorate),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-north-star-armoury-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_north_star_armoury),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-buccaneers-booty-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_buccaneers_booty),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-shattered-shield-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_shattered_shield),
        },
        VisualRouteSuiteCase {
            label: "route-shop-arms-siege-crafters-terminator-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "H", "\x1b"],
            configure: Some(seed_visual_route_arms_siege_crafters),
        },
        VisualRouteSuiteCase {
            label: "route-shop-healer-heal-decline",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "H", "1", "N"],
            configure: Some(seed_visual_route_healer),
        },
        VisualRouteSuiteCase {
            label: "route-shop-healer-heal-decline-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "H", "1", "N"],
            configure: Some(seed_visual_route_healer),
        },
        VisualRouteSuiteCase {
            label: "route-shop-healer-cure-accept",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "C", "1", "Y"],
            configure: Some(seed_visual_route_healer_cure),
        },
        VisualRouteSuiteCase {
            label: "route-shop-healer-heal-accept",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "H", "1", "Y"],
            configure: Some(seed_visual_route_healer_heal),
        },
        VisualRouteSuiteCase {
            label: "route-shop-healer-resurrect-accept",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "R", "1", "Y"],
            configure: Some(seed_visual_route_healer_resurrect),
        },
        VisualRouteSuiteCase {
            label: "route-shop-inn-rest-decline",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["R", "N", "P"],
            configure: Some(seed_visual_route_inn_rest_decline),
        },
        VisualRouteSuiteCase {
            label: "route-shop-inn-rest-decline-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["R", "N", "P"],
            configure: Some(seed_visual_route_inn_rest_decline),
        },
        VisualRouteSuiteCase {
            label: "route-shop-reagent-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["A", "1", "N"],
            configure: Some(seed_visual_route_reagent),
        },
        VisualRouteSuiteCase {
            label: "route-shop-reagent-buy-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["A", "1", "N"],
            configure: Some(seed_visual_route_reagent),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-drink-and-food",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "M", "R", "1", "N"],
            configure: Some(seed_visual_route_tavern),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-drink-and-food-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "M", "R", "1", "N"],
            configure: Some(seed_visual_route_tavern),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-honest-meal-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "A", "C", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_honest_meal_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-wayfarer-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "A", "C", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_wayfarer_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-sword-and-keg-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "A", "C", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_sword_and_keg_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-slaughtered-lamb-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "R", "H", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_slaughtered_lamb_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-humble-palate-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "S", "A", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_humble_palate_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-blue-boar-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "C", "T", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_blue_boar_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-cats-lair-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "A", "C", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_cats_lair_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-fallen-virgin-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "R", "H", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_fallen_virgin_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-tavern-folley-tap-lore",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["Y", "A", "C", "HONE", "Y"],
            configure: Some(seed_visual_route_tavern_folley_tap_lore),
        },
        VisualRouteSuiteCase {
            label: "route-shop-horse-trader-decline",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "N"],
            configure: Some(seed_visual_route_horse_trader_decline),
        },
        VisualRouteSuiteCase {
            label: "route-shop-horse-trader-decline-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "N"],
            configure: Some(seed_visual_route_horse_trader_decline),
        },
        VisualRouteSuiteCase {
            label: "route-shop-horse-trader-no-marker-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "Y"],
            configure: Some(seed_visual_route_horse_trader_no_marker),
        },
        VisualRouteSuiteCase {
            label: "route-shop-shipwright-quote-decline",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["F", "N"],
            configure: Some(seed_visual_route_shipwright),
        },
        VisualRouteSuiteCase {
            label: "route-shop-shipwright-quote-decline-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["F", "N"],
            configure: Some(seed_visual_route_shipwright),
        },
        VisualRouteSuiteCase {
            label: "route-shop-shipwright-frigate-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["F", "Y"],
            configure: Some(seed_visual_route_shipwright),
        },
        VisualRouteSuiteCase {
            label: "route-shop-shipwright-island-frigate-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["F", "Y"],
            configure: Some(seed_visual_route_shipwright_island),
        },
        VisualRouteSuiteCase {
            label: "route-shop-shipwright-crows-nest-skiff-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["S", "Y"],
            configure: Some(seed_visual_route_shipwright_crows_nest),
        },
        VisualRouteSuiteCase {
            label: "route-shop-shipwright-oaken-oar-frigate-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["F", "Y"],
            configure: Some(seed_visual_route_shipwright_oaken_oar),
        },
        VisualRouteSuiteCase {
            label: "route-shop-shipwright-rusty-bucket-skiff-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["S", "Y"],
            configure: Some(seed_visual_route_shipwright_rusty_bucket),
        },
        VisualRouteSuiteCase {
            label: "route-shop-guild-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["A", "1", "D"],
            configure: Some(seed_visual_route_guild),
        },
        VisualRouteSuiteCase {
            label: "route-shop-guild-buy-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["A", "1", "D"],
            configure: Some(seed_visual_route_guild),
        },
        VisualRouteSuiteCase {
            label: "route-shop-sage-topic-miss",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["MANTRA"],
            configure: Some(|state| {
                state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
            }),
        },
        VisualRouteSuiteCase {
            label: "route-shop-sage-topic-miss-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["MANTRA"],
            configure: Some(|state| {
                state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
            }),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-blink-east-ray",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: &["C1IP6"],
            configure: Some(seed_visual_route_blink),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-locate-cast",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: &["C1IW"],
            configure: Some(seed_visual_route_locate),
        },
        VisualRouteSuiteCase {
            label: "route-britannia-rel-hur-east",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                wind: WindState::Calm,
                wind_save_byte: WindState::Calm.save_byte(),
                ..PlayOptions::default()
            },
            script: &["C1HR6"],
            configure: Some(seed_visual_route_rel_hur),
        },
        VisualRouteSuiteCase {
            label: "route-castle-in-lor-spell",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1IL"],
            configure: Some(seed_visual_route_in_lor),
        },
        VisualRouteSuiteCase {
            label: "route-castle-light-open-spell",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1LV", "C1AS6"],
            configure: Some(seed_visual_route_light_open),
        },
        VisualRouteSuiteCase {
            label: "route-castle-light-open-spell-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1LV", "C1AS6"],
            configure: Some(seed_visual_route_light_open),
        },
        VisualRouteSuiteCase {
            label: "route-castle-light-decay-route",
            frame_kind: "visual route town frame",
            options: castle_light_decay,
            script: &["empty", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-restore-spell-suite",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1AZ", "C1AN3", "C1M3", "C1MV3", "C1CIM4"],
            configure: Some(seed_visual_route_restore_spells),
        },
        VisualRouteSuiteCase {
            label: "route-castle-active-effect-spell-suite",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1IS", "C1RT", "C1AI", "C1AT"],
            configure: Some(seed_visual_route_active_effect_spells),
        },
        VisualRouteSuiteCase {
            label: "route-combat-directed-sleep-cone",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1IZ6"],
            configure: Some(seed_visual_route_directed_sleep),
        },
        VisualRouteSuiteCase {
            label: "route-combat-directed-poison-wind-cone",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1HIN6"],
            configure: Some(seed_visual_route_directed_poison_wind),
        },
        VisualRouteSuiteCase {
            label: "route-combat-directed-death-wind-cone",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1CGIV6"],
            configure: Some(seed_visual_route_directed_death_wind),
        },
        VisualRouteSuiteCase {
            label: "route-combat-directed-flame-wind-cone",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1FHI6"],
            configure: Some(seed_visual_route_directed_flame_wind),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-fire-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1FGI6"],
            configure: Some(seed_visual_route_combat_fire_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-fire-marker-placement",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1FGI6"],
            configure: Some(seed_visual_route_combat_fire_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-poison-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1GIN6"],
            configure: Some(seed_visual_route_combat_poison_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-poison-marker-placement",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1GIN6"],
            configure: Some(seed_visual_route_combat_poison_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-sleep-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1GIZ6"],
            configure: Some(seed_visual_route_combat_sleep_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-sleep-marker-placement",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1GIZ6"],
            configure: Some(seed_visual_route_combat_sleep_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-energy-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1GIS6"],
            configure: Some(seed_visual_route_combat_energy_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-energy-marker-placement",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1GIS6"],
            configure: Some(seed_visual_route_combat_energy_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-dispel-field-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AG6"],
            configure: Some(seed_visual_route_combat_dispel_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-dispel-fire-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AG6"],
            configure: Some(seed_visual_route_combat_dispel_field),
        },
        VisualRouteSuiteCase {
            label: "route-combat-field-dispel-empty-refusal",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AG6"],
            configure: Some(seed_visual_route_combat_dispel_empty),
        },
        VisualRouteSuiteCase {
            label: "route-combat-utility-vanish-failure",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AY"],
            configure: Some(seed_visual_route_combat_utility_vanish),
        },
        VisualRouteSuiteCase {
            label: "route-combat-utility-open-failure",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AS"],
            configure: Some(seed_visual_route_combat_utility_open),
        },
        VisualRouteSuiteCase {
            label: "route-combat-utility-magic-lock-failure",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AEP"],
            configure: Some(seed_visual_route_combat_utility_magic_lock),
        },
        VisualRouteSuiteCase {
            label: "route-combat-utility-unlock-magic-failure",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1EIP"],
            configure: Some(seed_visual_route_combat_utility_unlock_magic),
        },
        VisualRouteSuiteCase {
            label: "route-combat-magic-missile-target",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1GP7"],
            configure: Some(seed_visual_route_combat_magic_missile),
        },
        VisualRouteSuiteCase {
            label: "route-combat-fireball-target",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1FV7"],
            configure: Some(seed_visual_route_combat_fireball),
        },
        VisualRouteSuiteCase {
            label: "route-combat-reveal-hidden-target",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1QW"],
            configure: Some(seed_visual_route_combat_reveal),
        },
        VisualRouteSuiteCase {
            label: "route-combat-invisibility-caster",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1LS"],
            configure: Some(seed_visual_route_combat_invisibility),
        },
        VisualRouteSuiteCase {
            label: "route-combat-cause-fear-target",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1CIQ"],
            configure: Some(seed_visual_route_combat_cause_fear),
        },
        VisualRouteSuiteCase {
            label: "route-combat-mass-charm-effect",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AQW"],
            configure: Some(seed_visual_route_combat_mass_charm),
        },
        VisualRouteSuiteCase {
            label: "route-combat-tremor-targets",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1IPVY"],
            configure: Some(seed_visual_route_combat_tremor),
        },
        VisualRouteSuiteCase {
            label: "route-combat-repel-undead-targets",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1ACX"],
            configure: Some(seed_visual_route_combat_repel_undead),
        },
        VisualRouteSuiteCase {
            label: "route-combat-charm-target",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1AEX7"],
            configure: Some(seed_visual_route_combat_charm),
        },
        VisualRouteSuiteCase {
            label: "route-combat-polymorph-target",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1BRX7"],
            configure: Some(seed_visual_route_combat_polymorph),
        },
        VisualRouteSuiteCase {
            label: "route-combat-clone-target",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1IQX7"],
            configure: Some(seed_visual_route_combat_clone),
        },
        VisualRouteSuiteCase {
            label: "route-combat-conjure-animal",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1KX"],
            configure: Some(seed_visual_route_combat_conjure),
        },
        VisualRouteSuiteCase {
            label: "route-combat-swarm-summon",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1BIX"],
            configure: Some(seed_visual_route_combat_swarm),
        },
        VisualRouteSuiteCase {
            label: "route-combat-summon-daemon-ring",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1CKX6"],
            configure: Some(seed_visual_route_combat_summon_daemon),
        },
        VisualRouteSuiteCase {
            label: "route-combat-kill-gazer-eye-burst",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1CX7"],
            configure: Some(seed_visual_route_combat_kill_gazer),
        },
        VisualRouteSuiteCase {
            label: "route-combat-kill-gargoyle-lava-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1CX7"],
            configure: Some(seed_visual_route_combat_kill_gargoyle),
        },
        VisualRouteSuiteCase {
            label: "route-combat-kill-shadowlord-vanish-marker",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["C1CX7"],
            configure: Some(seed_visual_route_combat_kill_shadowlord),
        },
        VisualRouteSuiteCase {
            label: "route-terrain-combat-party-entry",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["setup:terrain-combat-party-entry"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-terrain-combat-xit-no-foes-clean-exit",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["setup:terrain-combat-no-foes", "X"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-terrain-combat-out-of-arena-leave",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: &["setup:terrain-combat-east-edge", "d"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-room-party-entry",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                ..PlayOptions::default()
            },
            script: &["setup:dungeon-room-party-entry"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-level-up-down-spells",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 1,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["C1PU", "C1DP"],
            configure: Some(seed_visual_route_dungeon_level_spells),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-field-cycle-spells",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                facing: Some(Direction::East),
                ..PlayOptions::default()
            },
            script: &[
                "C1FGI6", "C1AG6", "C1GIN6", "C1AG6", "C1GIZ6", "C1AG6", "C1GIS6", "C1AG6",
            ],
            configure: Some(seed_visual_route_dungeon_field_cycle),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-open-chest-spell",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["C1AS"],
            configure: Some(seed_visual_route_dungeon_open_chest),
        },
        VisualRouteSuiteCase {
            label: "route-castle-poison-gas-step",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["d"],
            configure: Some(seed_visual_route_poison_gas),
        },
        VisualRouteSuiteCase {
            label: "route-shop-inn-rest-accept",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["R", "Y"],
            configure: Some(seed_visual_route_inn_rest_accept),
        },
        VisualRouteSuiteCase {
            label: "route-shop-inn-rest-accept-public-rate",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["R", "Y"],
            configure: Some(seed_visual_route_inn_rest_accept),
        },
        VisualRouteSuiteCase {
            label: "route-shop-horse-trader-horse-and-rider-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "Y"],
            configure: Some(seed_visual_route_horse_trader_horse_and_rider),
        },
        VisualRouteSuiteCase {
            label: "route-reload-horse-trader-horse-and-rider-buy-pass",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "Y", "empty"],
            configure: Some(seed_visual_route_horse_trader_horse_and_rider),
        },
        VisualRouteSuiteCase {
            label: "route-shop-horse-trader-stablehouse-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "Y"],
            configure: Some(seed_visual_route_horse_trader_stablehouse),
        },
        VisualRouteSuiteCase {
            label: "route-shop-horse-trader-wishing-well-buy",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["B", "Y"],
            configure: Some(seed_visual_route_horse_trader_wishing_well),
        },
        VisualRouteSuiteCase {
            label: "route-shop-sage-topic-paid-success",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["HONE", "Y"],
            configure: Some(seed_visual_route_sage_paid),
        },
        VisualRouteSuiteCase {
            label: "route-shop-sage-topic-paid-success-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["HONE", "Y"],
            configure: Some(seed_visual_route_sage_paid),
        },
        VisualRouteSuiteCase {
            label: "route-shop-sage-topic-short-funds",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["COMP", "Y"],
            configure: Some(seed_visual_route_sage_short_funds),
        },
        VisualRouteSuiteCase {
            label: "route-shop-sage-topic-short-funds-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["COMP", "Y"],
            configure: Some(seed_visual_route_sage_short_funds),
        },
        VisualRouteSuiteCase {
            label: "route-castle-fountain-look",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["l6", "1"],
            configure: Some(seed_visual_route_fountain),
        },
        VisualRouteSuiteCase {
            label: "route-castle-surface-fountain-look",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["l6", "1"],
            configure: Some(seed_visual_route_fountain),
        },
        VisualRouteSuiteCase {
            label: "route-yew-wanted-poster-look",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(4).expect("Yew scene is valid")),
                ..PlayOptions::default()
            },
            script: &["l6"],
            configure: Some(seed_visual_route_yew_wanted_poster),
        },
        VisualRouteSuiteCase {
            label: "route-castle-town-attack-death-mask-npc",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["A6"],
            configure: Some(seed_visual_route_town_attack_death_mask_npc),
        },
        VisualRouteSuiteCase {
            label: "route-castle-town-attack-guard-alarm",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["A6"],
            configure: Some(seed_visual_route_town_attack_guard_alarm),
        },
        VisualRouteSuiteCase {
            label: "route-castle-town-hostile-adjacent-alarm",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["empty"],
            configure: Some(seed_visual_route_town_hostile_adjacent_alarm),
        },
        VisualRouteSuiteCase {
            label: "route-castle-town-guard-arrest-refusal",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["empty", "N"],
            configure: Some(seed_visual_route_town_guard_arrest),
        },
        VisualRouteSuiteCase {
            label: "route-castle-town-guard-arrest-surrender-yew",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["empty", "Y"],
            configure: Some(seed_visual_route_town_guard_arrest),
        },
        VisualRouteSuiteCase {
            label: "route-buccaneers-den-wishing-well",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x18).expect("Buccaneer's Den scene is valid")),
                ..PlayOptions::default()
            },
            script: &["l6", "Y", "Horse"],
            configure: Some(seed_visual_route_wishing_well),
        },
        VisualRouteSuiteCase {
            label: "route-buccaneers-den-wishing-well-horse",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x18).expect("Buccaneer's Den scene is valid")),
                ..PlayOptions::default()
            },
            script: &["l6", "Y", "Horse"],
            configure: Some(seed_visual_route_wishing_well),
        },
        VisualRouteSuiteCase {
            label: "route-buccaneers-den-wishing-well-ferrari-grants-horse",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x18).expect("Buccaneer's Den scene is valid")),
                ..PlayOptions::default()
            },
            script: &["l6", "Y", "Ferrari"],
            configure: Some(seed_visual_route_wishing_well),
        },
        VisualRouteSuiteCase {
            label: "route-castle-death-vision-look",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["l6", "1"],
            configure: Some(seed_visual_route_death_vision),
        },
        VisualRouteSuiteCase {
            label: "route-blackthorn-audience-correct",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["setup:blackthorn-audience", "Ahm"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-blackthorn-audience-wrong",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["setup:blackthorn-audience", "wrong"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-blackthorn-rescue-refuge",
            frame_kind: "visual route town frame",
            options: PlayOptions::default(),
            script: &["setup:blackthorn-rescue", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-virtue-town-shadowlord-entry",
            frame_kind: "visual route town frame",
            options: shadowlord_town_entry,
            script: &[],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-virtue-town-shadowlord-yell",
            frame_kind: "visual route town frame",
            options: shadowlord_town_yell,
            script: &["YFAULINEI"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-lycaeum-shard-falsehood-vanquish",
            frame_kind: "visual route town frame",
            options: lycaeum_shard_falsehood,
            script: &["UF"],
            configure: Some(seed_visual_route_falsehood_shard),
        },
        VisualRouteSuiteCase {
            label: "route-empath-shard-hatred-vanquish",
            frame_kind: "visual route town frame",
            options: empath_shard_hatred,
            script: &["UH"],
            configure: Some(seed_visual_route_hatred_shard),
        },
        VisualRouteSuiteCase {
            label: "route-serpents-hold-shard-cowardice-vanquish",
            frame_kind: "visual route town frame",
            options: serpents_shard_cowardice,
            script: &["UCW"],
            configure: Some(seed_visual_route_cowardice_shard),
        },
        VisualRouteSuiteCase {
            label: "route-stonegate-shadowlord-entry",
            frame_kind: "visual route town frame",
            options: stonegate_entry,
            script: &[],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-word-of-power-seal-opens",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(fallax_seal.plane),
                ..PlayOptions::default()
            },
            script: &["YFALLAX"],
            configure: Some(seed_visual_route_britannia_word_of_power),
        },
        VisualRouteSuiteCase {
            label: "route-underworld-doom-word-of-power-seal-opens",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(veramocor_seal.plane),
                ..PlayOptions::default()
            },
            script: &["YVERAMOCOR"],
            configure: Some(seed_visual_route_underworld_word_of_power),
        },
        VisualRouteSuiteCase {
            label: "route-endgame-missing-box-terminal-jitter",
            frame_kind: "visual route endgame frame",
            options: PlayOptions::default(),
            script: &["Y", "Y", ""],
            configure: Some(seed_visual_route_endgame_missing_box),
        },
        VisualRouteSuiteCase {
            label: "route-endgame-missing-box-confirmation",
            frame_kind: "visual route endgame frame",
            options: PlayOptions::default(),
            script: &["Y", "Y"],
            configure: Some(seed_visual_route_endgame_missing_box),
        },
        VisualRouteSuiteCase {
            label: "route-endgame-box-victory-confirmation",
            frame_kind: "visual route endgame frame",
            options: PlayOptions::default(),
            script: &["Y", "Y"],
            configure: Some(seed_visual_route_endgame_victory),
        },
        VisualRouteSuiteCase {
            label: "route-endgame-box-full-victory-cinematic",
            frame_kind: "visual route endgame frame",
            options: PlayOptions::default(),
            script: &[
                "Y", "Y", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "", "",
            ],
            configure: Some(seed_visual_route_endgame_victory),
        },
        VisualRouteSuiteCase {
            label: "route-endgame-class-tableau-restoration",
            frame_kind: "visual route endgame frame",
            options: PlayOptions::default(),
            script: &["Y"],
            configure: Some(seed_visual_route_endgame_class_tableau),
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-trigger",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &[""],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-pass",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["", ""],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-attack",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["", "A6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-board-refusal",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["", "B"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-z-stats",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["", "Z"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-search-prompt",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["", "S"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-britannia-extended-exploration",
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: &[
                "d", "d", "s", "s", "a", "a", "w", "w", "l6", "empty", "Z", "empty",
            ],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-extended-walk-and-save",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["s", "s", "d", "d", "w", "w", "Q", "N", "Z"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-castle-extended-walk-and-rest",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["s"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-extended-turn-and-search",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["a", "a", "d", "w", "s", "w", "d", "a", "S6"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-multi-round-pass",
            frame_kind: "visual route combat frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["empty", "empty", "empty", "empty", "empty"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-doom-combat-quit-defeat",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(doom),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["setup:dungeon-room-party-entry", "q"],
            configure: None,
        },
    ];
    cases.extend([
        VisualRouteSuiteCase {
            label: "route-castle-mix-ready-order-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["MIL/0x80/1", "R1/26", "R1/26", "N23"],
            configure: Some(seed_visual_route_command_workflows),
        },
        VisualRouteSuiteCase {
            label: "route-castle-party-overlay-routes",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["C1IL", "I", "N12", "R"],
            configure: Some(seed_visual_route_party_overlay_workflows),
        },
        VisualRouteSuiteCase {
            label: "route-castle-talk-ordinary-keyword-route",
            frame_kind: "visual route town frame",
            options: PlayOptions {
                target: PlayTarget::Town(castle),
                ..PlayOptions::default()
            },
            script: &["T", "6", "NAME"],
            configure: Some(seed_visual_route_talk_ordinary_keyword),
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-ignite-torch-route",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 0,
                ..PlayOptions::default()
            },
            script: &["I"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-turn-and-blocked-step",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["w", "a", "d", "s"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-sjog-underfoot-routes",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["G", "J"],
            configure: None,
        },
        VisualRouteSuiteCase {
            label: "route-dungeon-refusal-letter-routes",
            frame_kind: "visual route dungeon frame",
            options: PlayOptions {
                target: PlayTarget::Dungeon(dungeon),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: &["B"],
            configure: None,
        },
        visual_doom_combat_case("route-doom-room-combat-trigger", doom, &[""]),
        visual_doom_combat_case("route-doom-combat-pass-round", doom, &["", ""]),
        visual_doom_combat_case("route-doom-combat-select-player-clear", doom, &["", "0"]),
        visual_doom_combat_case("route-doom-combat-select-player-one", doom, &["", "1"]),
        visual_doom_combat_case("route-doom-combat-select-player-six", doom, &["", "6"]),
        visual_doom_combat_case("route-doom-combat-direct-step-east", doom, &["", "d"]),
        visual_doom_combat_case("route-doom-combat-d-refusal", doom, &["", "D"]),
        visual_doom_combat_case("route-doom-combat-w-refusal", doom, &["", "W"]),
        visual_doom_combat_case("route-doom-combat-view-label-only", doom, &["", "V"]),
        visual_doom_combat_case("route-doom-combat-look-label-only", doom, &["", "L"]),
        visual_doom_combat_case("route-doom-combat-attack-direction", doom, &["", "A6"]),
        visual_doom_combat_case("route-doom-combat-escape-abort", doom, &["", "\x1b"]),
        visual_doom_combat_case("route-doom-combat-music-toggle", doom, &["", "\u{13}"]),
        visual_doom_combat_case("route-doom-combat-select-clear", doom, &["", "0"]),
        visual_doom_combat_case("route-doom-combat-select-one", doom, &["", "1"]),
        visual_doom_combat_case("route-doom-combat-select-six", doom, &["", "6"]),
        visual_doom_combat_case("route-doom-combat-step-east", doom, &["", "d"]),
        visual_doom_combat_case("route-doom-combat-use-refusal", doom, &["", "U"]),
        visual_doom_combat_case("route-doom-combat-drop-refusal", doom, &["", "D"]),
        visual_doom_combat_case("route-doom-combat-wear-refusal", doom, &["", "W"]),
        visual_doom_combat_case("route-doom-combat-enter-refusal", doom, &["", "E"]),
        visual_doom_combat_case("route-doom-combat-fire-refusal", doom, &["", "F"]),
        visual_doom_combat_case("route-doom-combat-hole-up-refusal", doom, &["", "H"]),
        visual_doom_combat_case("route-doom-combat-ignite-refusal", doom, &["", "I"]),
        visual_doom_combat_case("route-doom-combat-mix-refusal", doom, &["", "M"]),
        visual_doom_combat_case("route-doom-combat-new-order-refusal", doom, &["", "N"]),
        visual_doom_combat_case("route-doom-combat-talk-refusal", doom, &["", "T"]),
        visual_doom_combat_case("route-doom-combat-view-label", doom, &["", "V"]),
        visual_doom_combat_case("route-doom-combat-look-label", doom, &["", "L"]),
        visual_doom_combat_case("route-doom-combat-cast-refusal", doom, &["", "C1IL"]),
        visual_doom_combat_case("route-doom-combat-get-direction", doom, &["", "G6"]),
        visual_doom_combat_case("route-doom-combat-jimmy-direction", doom, &["", "J6"]),
        visual_doom_combat_case("route-doom-combat-open-direction", doom, &["", "O6"]),
        visual_doom_combat_case("route-doom-combat-push-direction", doom, &["", "P6"]),
        visual_doom_combat_case("route-doom-combat-klimb-direction", doom, &["", "K6"]),
        visual_doom_combat_case("route-doom-combat-ready-prompt", doom, &["", "R"]),
        visual_doom_combat_case("route-doom-combat-yell-word", doom, &["", "YFALLAX"]),
        visual_doom_combat_case("route-doom-combat-xit-foes-remain", doom, &["", "X"]),
    ]);
    append_directed_wind_visual_route_cases(&mut cases);
    append_asset_backed_conversation_visual_route_cases(&mut cases);
    append_shrine_visual_route_cases(&mut cases);
    append_public_location_visual_route_cases(&mut cases);
    cases
}

fn append_directed_wind_visual_route_cases(cases: &mut Vec<VisualRouteSuiteCase>) {
    let world = PlayOptions {
        target: PlayTarget::World(WorldPlane::Britannia),
        ..PlayOptions::default()
    };
    for (label, script, configure) in [
        (
            "route-combat-directed-sleep-cone-north",
            &["C1IZ8"][..],
            seed_visual_route_directed_sleep_north as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-sleep-cone-east",
            &["C1IZ6"][..],
            seed_visual_route_directed_sleep_east as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-sleep-cone-south",
            &["C1IZ2"][..],
            seed_visual_route_directed_sleep_south as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-sleep-cone-west",
            &["C1IZ4"][..],
            seed_visual_route_directed_sleep_west as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-poison-wind-cone-north",
            &["C1HIN8"][..],
            seed_visual_route_directed_poison_wind_north as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-poison-wind-cone-east",
            &["C1HIN6"][..],
            seed_visual_route_directed_poison_wind_east as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-poison-wind-cone-south",
            &["C1HIN2"][..],
            seed_visual_route_directed_poison_wind_south as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-poison-wind-cone-west",
            &["C1HIN4"][..],
            seed_visual_route_directed_poison_wind_west as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-death-wind-cone-north",
            &["C1CGIV8"][..],
            seed_visual_route_directed_death_wind_north as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-death-wind-cone-east",
            &["C1CGIV6"][..],
            seed_visual_route_directed_death_wind_east as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-death-wind-cone-south",
            &["C1CGIV2"][..],
            seed_visual_route_directed_death_wind_south as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-death-wind-cone-west",
            &["C1CGIV4"][..],
            seed_visual_route_directed_death_wind_west as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-flame-wind-cone-north",
            &["C1FHI8"][..],
            seed_visual_route_directed_flame_wind_north as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-flame-wind-cone-east",
            &["C1FHI6"][..],
            seed_visual_route_directed_flame_wind_east as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-flame-wind-cone-south",
            &["C1FHI2"][..],
            seed_visual_route_directed_flame_wind_south as fn(&mut PlayState),
        ),
        (
            "route-combat-directed-flame-wind-cone-west",
            &["C1FHI4"][..],
            seed_visual_route_directed_flame_wind_west as fn(&mut PlayState),
        ),
    ] {
        cases.push(VisualRouteSuiteCase {
            label,
            frame_kind: "visual route combat frame",
            options: world.clone(),
            script,
            configure: Some(configure),
        });
    }
}

fn append_asset_backed_conversation_visual_route_cases(cases: &mut Vec<VisualRouteSuiteCase>) {
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
            ] {
                let label: &'static str = if scene_byte == representative_scene {
                    Box::leak(format!("route-talk-{family}-{kind}").into_boxed_str())
                } else {
                    Box::leak(
                        format!("route-talk-{family}-{scene_byte:02}-{kind}").into_boxed_str(),
                    )
                };
                let command: &'static str = Box::leak(command.to_string().into_boxed_str());
                let script: &'static [&'static str] =
                    Box::leak(vec!["T", "6", command].into_boxed_slice());
                cases.push(VisualRouteSuiteCase {
                    label,
                    frame_kind: "visual route town frame",
                    options: PlayOptions {
                        target: PlayTarget::Town(scene),
                        ..PlayOptions::default()
                    },
                    script,
                    configure: Some(seed_visual_route_talk_ordinary_keyword),
                });
            }
        }
    }
}

fn append_shrine_visual_route_cases(cases: &mut Vec<VisualRouteSuiteCase>) {
    for virtue in ShrineVirtue::ALL {
        let label: &'static str = Box::leak(
            format!(
                "route-shrine-native-{}-meditation",
                virtue.name().to_ascii_lowercase()
            )
            .into_boxed_str(),
        );
        let command: &'static str = Box::leak(format!("M{}", virtue.mantra()).into_boxed_str());
        let script: &'static [&'static str] = Box::leak(vec![command].into_boxed_slice());
        cases.push(VisualRouteSuiteCase {
            label,
            frame_kind: "visual route world frame",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script,
            configure: None,
        });
    }
    cases.push(VisualRouteSuiteCase {
        label: "route-codex-urn-honesty-read",
        frame_kind: "visual route world frame",
        options: PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            ..PlayOptions::default()
        },
        script: &["M"],
        configure: None,
    });
    cases.push(VisualRouteSuiteCase {
        label: "route-shrine-honesty-codex-turn-in",
        frame_kind: "visual route world frame",
        options: PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            ..PlayOptions::default()
        },
        script: &["MAhm"],
        configure: None,
    });
    cases.push(VisualRouteSuiteCase {
        label: "route-shrine-compassion-completed-offering",
        frame_kind: "visual route world frame",
        options: PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            gold: 500,
            ..PlayOptions::default()
        },
        script: &["MMu/1"],
        configure: None,
    });
}

fn append_public_location_visual_route_cases(cases: &mut Vec<VisualRouteSuiteCase>) {
    for (index, entry) in published_world_location_entries().into_iter().enumerate() {
        let label: &'static str =
            Box::leak(format!("route-stock-location-enter-{:02}", index + 1).into_boxed_str());
        let mut options = PlayOptions {
            target: PlayTarget::World(entry.plane),
            ..PlayOptions::default()
        };
        if matches!(entry.target, PlayTarget::Dungeon(scene) if scene.record == 7) {
            options.shadowlord_hideouts = [SHADOWLORD_VANQUISHED; 3];
        }
        let frame_kind = match entry.target {
            PlayTarget::Town(_) => "visual route town frame",
            PlayTarget::Dungeon(_) => "visual route dungeon frame",
            PlayTarget::World(_) => continue,
        };
        cases.push(VisualRouteSuiteCase {
            label,
            frame_kind,
            options,
            script: &["e"],
            configure: None,
        });
    }
}

fn visual_doom_combat_case(
    label: &'static str,
    doom: DungeonScene,
    script: &'static [&'static str],
) -> VisualRouteSuiteCase {
    VisualRouteSuiteCase {
        label,
        frame_kind: "visual route combat frame",
        options: PlayOptions {
            target: PlayTarget::Dungeon(doom),
            floor: 0,
            torch_counter: 9,
            ..PlayOptions::default()
        },
        script,
        configure: None,
    }
}

fn seed_visual_route_gate_travel_resources(options: &mut PlayOptions) {
    options.spell_charges[GATE_TRAVEL_SPELL_INDEX] = 1;
    if let Some(caster) = options.party.first_mut() {
        caster.mana = GATE_TRAVEL_COST + 1;
        caster.level = GATE_TRAVEL_COST;
    }
}

fn seed_visual_route_natural_moongate(state: &mut PlayState) {
    let idx = state.player.y * WORLD_SIDE + state.player.x;
    if let Some(tile) = state.grid.get_mut(idx) {
        *tile = NATURAL_MOONGATE_TERRAIN_TILE;
    }
    state.natural_moongate_live_cells = vec![idx];
    state.set_cached_moon_glyph_slots(Some(0), None);
    state.mark_visibility_dirty();
}

fn seed_visual_route_underworld_fixed_hidden_stack(state: &mut PlayState) {
    state.player.x = 232;
    state.player.y = 233;
    state.player.facing = Direction::East;
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_blackthorn_fixed_hidden(state: &mut PlayState) {
    state.player.x = 5;
    state.player.y = 8;
    state.player.facing = Direction::East;
    state.keys = 0;
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_minoc_fixed_hidden_daily(state: &mut PlayState) {
    state.player.x = 1;
    state.player.y = 2;
    state.player.facing = Direction::East;
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_hourly_poison_starvation(state: &mut PlayState) {
    state.prng_state = 0x3456;
}

fn seed_visual_route_hourly_ring_regeneration(state: &mut PlayState) {
    state.prng_state = ring_regeneration_first_heal_seed();
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

fn seed_visual_route_talk_status_sleeping(state: &mut PlayState) {
    seed_visual_route_talk_status_tile(state, TALK_STATUS_TILE_SLEEPING);
}

fn seed_visual_route_talk_status_praying(state: &mut PlayState) {
    seed_visual_route_talk_status_tile(state, TALK_STATUS_TILE_PRAYING);
}

fn seed_visual_route_talk_status_tile(state: &mut PlayState, status_tile: u8) {
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
    if let Some(slot) = state.npcs.first().and_then(|npc| npc.active_object)
        && let Some(object) = state.active_objects.get_mut(slot)
    {
        object.type_byte = 1;
        object.tile = status_tile;
    }
    state.mark_visibility_dirty();
}

fn seed_visual_route_talk_ordinary_keyword(state: &mut PlayState) {
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

fn seed_visual_route_native_stair_up(state: &mut PlayState) {
    seed_visual_route_town_native_stair(state, Direction::East, 0xC5);
}

fn seed_visual_route_native_stair_down(state: &mut PlayState) {
    seed_visual_route_town_native_stair(state, Direction::East, 0xC7);
}

fn seed_visual_route_native_stair_cross(state: &mut PlayState) {
    seed_visual_route_town_native_stair(state, Direction::North, 0xC5);
}

fn seed_visual_route_town_native_stair(state: &mut PlayState, facing: Direction, stair_tile: u8) {
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

fn seed_visual_route_dungeon_active_monster_attack(state: &mut PlayState) {
    seed_visual_route_dungeon_active_monster(state, STEADY_PHASE);
}

fn seed_visual_route_dungeon_active_monster_contact(state: &mut PlayState) {
    seed_visual_route_dungeon_active_monster(state, 0x20);
}

fn seed_visual_route_dungeon_active_monster(state: &mut PlayState, phase: u8) {
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

fn seed_visual_route_command_workflows(state: &mut PlayState) {
    state.party = vec![
        route_visual_party_member(0, b'A', b'G', 20, 20),
        route_visual_party_member(1, b'F', b'G', 20, 20),
        route_visual_party_member(2, b'M', b'G', 20, 20),
    ];
    state.party_names = default_party_names(3);
    state.party_experience = default_party_experience(3);
    state.party_stay_counters = default_party_stay_counters(3);
    state.party_strengths = vec![50; 3];
    state.party_intelligence = default_party_intelligence(3);
    state.party_equipment = default_party_equipment(3);
    state.reagents[REAGENT_SULFUR_ASH] = 2;
    state.equipment_stock[EQUIPMENT_ID_BOW] = 1;
    state.equipment_stock[EQUIPMENT_ID_ARROWS] = 5;
    state.party_equipment[0][EQUIP_SLOT_WEAPON] = EQUIPMENT_EMPTY;
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_party_overlay_workflows(state: &mut PlayState) {
    seed_visual_route_command_workflows(state);
    state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = caster.mana.max(IN_LOR_COST);
        caster.level = caster.level.max(IN_LOR_COST);
    }
}

fn seed_visual_route_board_horse(state: &mut PlayState) {
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

fn seed_visual_route_blink(state: &mut PlayState) {
    state.player.x = 62;
    state.player.y = 124;
    state.player.facing = Direction::East;
    state.spell_charges[BLINK_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = BLINK_COST;
        caster.level = BLINK_COST;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_locate(state: &mut PlayState) {
    state.player.x = 62;
    state.player.y = 124;
    state.spell_charges[IN_WIS_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = IN_WIS_COST;
        caster.level = IN_WIS_COST;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_rel_hur(state: &mut PlayState) {
    state.wind = WindState::Calm;
    state.wind_save_byte = WindState::Calm.save_byte();
    state.spell_charges[REL_HUR_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = REL_HUR_COST;
        caster.level = REL_HUR_COST;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_in_lor(state: &mut PlayState) {
    state.spell_charges[IN_LOR_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = IN_LOR_COST;
        caster.level = IN_LOR_COST;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_light_open(state: &mut PlayState) {
    state.player.x = 1;
    state.player.y = 1;
    state.player.facing = Direction::East;
    let target = state.player.y * TOWN_GRID_SIDE + state.player.x + 1;
    if let Some(cell) = state.grid.get_mut(target) {
        *cell = 0x97;
    }
    state.spell_charges[VAS_LOR_SPELL_INDEX] = 1;
    state.spell_charges[OPEN_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = VAS_LOR_COST + OPEN_SPELL_COST;
        caster.level = VAS_LOR_COST.max(OPEN_SPELL_COST);
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn route_visual_party_member(
    slot: u8,
    class_byte: u8,
    status: u8,
    hp: u16,
    max_hp: u16,
) -> PartyMember {
    PartyMember {
        slot,
        class_byte,
        status,
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 0,
        hp,
        max_hp,
        level: 1,
    }
}

fn seed_visual_route_long_camp_recovery(state: &mut PlayState) {
    state.party = vec![
        route_visual_party_member(0, b'A', b'G', 1, 2),
        route_visual_party_member(1, b'M', b'G', 4, 10),
        route_visual_party_member(2, b'B', b'G', 5, 6),
        route_visual_party_member(3, b'F', b'G', 5, 20),
        route_visual_party_member(4, b'A', b'P', 20, 20),
        route_visual_party_member(5, b'M', b'D', 0, 20),
    ];
    for (member, mana) in state.party.iter_mut().zip([0, 1, 2, 3, 4, 5]) {
        member.mana = mana;
        member.level = 8;
    }
    state.avatar_stats.intelligence = 22;
    state.party_names = default_party_names(6);
    state.party_experience = default_party_experience(6);
    state.party_stay_counters = default_party_stay_counters(6);
    state.party_strengths = vec![30; 6];
    state.party_intelligence = vec![22, 24, 20, 18, 12, 8];
    state.party_equipment = default_party_equipment(6);
    state.party_roster = default_party_roster(6);
    state.prng_state = visual_long_camp_no_ambush_seed();
}

fn visual_long_camp_no_ambush_seed() -> u16 {
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

fn seed_visual_route_combat_entry_party(state: &mut PlayState) {
    state.party = vec![
        route_visual_party_member(0, b'A', b'G', 30, 30),
        route_visual_party_member(1, b'F', b'G', 30, 30),
    ];
    state.party_names = default_party_names(2);
    state.party_experience = vec![0, 0];
    state.party_stay_counters = default_party_stay_counters(2);
    state.party_strengths = vec![30; 2];
    state.party_intelligence = default_party_intelligence(2);
    state.party_equipment = default_party_equipment(2);
}

fn seed_visual_route_restore_spells(state: &mut PlayState) {
    state.party = vec![
        route_visual_party_member(0, b'A', b'G', 20, 20),
        route_visual_party_member(1, b'F', b'S', 8, 24),
        route_visual_party_member(2, b'M', b'P', 6, 30),
        route_visual_party_member(3, b'B', b'D', 0, 19),
    ];
    state.party_names = default_party_names(4);
    state.party_experience = vec![0, 0, 0, 350];
    state.party_stay_counters = default_party_stay_counters(4);
    state.party_strengths = vec![30; 4];
    state.party_intelligence = default_party_intelligence(4);
    state.party_equipment = default_party_equipment(4);
    state.moral_standing = 99;
    state.party[0].mana = AWAKEN_COST + CURE_COST + HEAL_COST + GREAT_HEAL_COST + RESURRECT_COST;
    state.party[0].level = RESURRECT_COST;
    state.spell_charges[AWAKEN_SPELL_INDEX] = 1;
    state.spell_charges[CURE_SPELL_INDEX] = 1;
    state.spell_charges[HEAL_SPELL_INDEX] = 1;
    state.spell_charges[GREAT_HEAL_SPELL_INDEX] = 1;
    state.spell_charges[RESURRECT_SPELL_INDEX] = 1;
}

fn seed_visual_route_active_effect_spells(state: &mut PlayState) {
    state.spell_charges[PROTECTION_SPELL_INDEX] = 1;
    state.spell_charges[QUICKNESS_SPELL_INDEX] = 1;
    state.spell_charges[NEGATE_MAGIC_SPELL_INDEX] = 1;
    state.spell_charges[TIME_STOP_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = PROTECTION_COST + QUICKNESS_COST + NEGATE_MAGIC_COST + TIME_STOP_COST;
        caster.level = TIME_STOP_COST;
    }
}

fn visual_route_combat_active_object(tile: u8, x: usize, y: usize, z: i8) -> ActiveObject {
    ActiveObject {
        type_byte: tile,
        tile,
        x,
        y,
        z,
        phase: 0,
        aux1: 0,
        aux3: 0,
    }
}

fn seed_visual_route_directed_wind_combat(
    state: &mut PlayState,
    spell_index: usize,
    cost: u8,
    party_count: usize,
    target_party_slot: Option<usize>,
    include_monster_target: bool,
    direction: Direction,
) {
    state.party = (0..party_count)
        .map(|slot| route_visual_party_member(slot as u8, b'A', b'G', 12, 20))
        .collect();
    state.party_names = default_party_names(party_count);
    state.party_experience = vec![0; party_count];
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
        let (target_x, target_y) = visual_directed_route_coordinate(direction, 1);
        actors[target_slot] = CombatActorDescriptor::from_row([
            12,
            1,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
            target_slot as u8,
            0,
            target_x,
            target_y,
        ]);
    }

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    for slot in 0..party_count {
        let (x, y) = if Some(slot) == target_party_slot {
            visual_directed_route_coordinate(direction, 1)
        } else {
            (5, 5)
        };
        active_objects[slot] =
            visual_route_combat_active_object(0x4c, usize::from(x), usize::from(y), 0);
    }

    if include_monster_target {
        let stats =
            combat_class_stats(COMBAT_CLASS_GIANT_RAT).expect("giant rat combat stats exist");
        let monster_slot = COMBAT_PARTY_ACTOR_SLOTS;
        let monster_distance = if target_party_slot.is_some() { 2 } else { 1 };
        let (monster_x, monster_y) = visual_directed_route_coordinate(direction, monster_distance);
        actors[monster_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            monster_slot as u8,
            monster_x,
            monster_y,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        active_objects[monster_slot] = summoned_active_object_record(
            COMBAT_CLASS_GIANT_RAT,
            monster_x as usize,
            monster_y as usize,
            0,
        )
        .expect("giant rat active object exists");

        let reserve_slot = monster_slot + 1;
        let (reserve_x, reserve_y) = visual_directed_route_reserve_coordinate(direction);
        actors[reserve_slot] = CombatActorDescriptor::for_monster_placement(
            stats,
            reserve_slot as u8,
            reserve_x,
            reserve_y,
            COMBAT_ACTOR_FLAG_SELECTABLE_80,
            0,
        );
        active_objects[reserve_slot] = summoned_active_object_record(
            COMBAT_CLASS_GIANT_RAT,
            reserve_x as usize,
            reserve_y as usize,
            0,
        )
        .expect("reserve giant rat active object exists");
    }

    state
        .enter_combat_frame(active_objects, actors)
        .expect("visual route directed wind combat frame should seed");
}

fn visual_directed_route_coordinate(direction: Direction, distance: i16) -> (u8, u8) {
    let (dx, dy) = direction.delta();
    (
        (5 + dx as i16 * distance) as u8,
        (5 + dy as i16 * distance) as u8,
    )
}

fn visual_directed_route_reserve_coordinate(direction: Direction) -> (u8, u8) {
    match direction {
        Direction::North => (5, 7),
        Direction::South => (5, 3),
        Direction::East => (3, 5),
        Direction::West => (7, 5),
        _ => (3, 5),
    }
}

fn seed_visual_route_directed_sleep(state: &mut PlayState) {
    seed_visual_route_directed_sleep_east(state);
}

fn seed_visual_route_directed_sleep_north(state: &mut PlayState) {
    seed_visual_route_directed_wind_combat(
        state,
        SLEEP_SPELL_INDEX,
        SLEEP_COST,
        2,
        Some(1),
        false,
        Direction::North,
    );
}

fn seed_visual_route_directed_sleep_east(state: &mut PlayState) {
    seed_visual_route_directed_wind_combat(
        state,
        SLEEP_SPELL_INDEX,
        SLEEP_COST,
        2,
        Some(1),
        false,
        Direction::East,
    );
}

fn seed_visual_route_directed_sleep_south(state: &mut PlayState) {
    seed_visual_route_directed_wind_combat(
        state,
        SLEEP_SPELL_INDEX,
        SLEEP_COST,
        2,
        Some(1),
        false,
        Direction::South,
    );
}

fn seed_visual_route_directed_sleep_west(state: &mut PlayState) {
    seed_visual_route_directed_wind_combat(
        state,
        SLEEP_SPELL_INDEX,
        SLEEP_COST,
        2,
        Some(1),
        false,
        Direction::West,
    );
}

fn seed_visual_route_directed_poison_wind(state: &mut PlayState) {
    seed_visual_route_directed_poison_wind_east(state);
}

fn seed_visual_route_directed_poison_wind_north(state: &mut PlayState) {
    seed_visual_route_directed_poison_wind_for_direction(state, Direction::North);
}

fn seed_visual_route_directed_poison_wind_east(state: &mut PlayState) {
    seed_visual_route_directed_poison_wind_for_direction(state, Direction::East);
}

fn seed_visual_route_directed_poison_wind_south(state: &mut PlayState) {
    seed_visual_route_directed_poison_wind_for_direction(state, Direction::South);
}

fn seed_visual_route_directed_poison_wind_west(state: &mut PlayState) {
    seed_visual_route_directed_poison_wind_for_direction(state, Direction::West);
}

fn seed_visual_route_directed_poison_wind_for_direction(
    state: &mut PlayState,
    direction: Direction,
) {
    seed_visual_route_directed_wind_combat(
        state,
        POISON_WIND_SPELL_INDEX,
        POISON_WIND_COST,
        3,
        Some(2),
        false,
        direction,
    );
    state.prng_state = poison_wind_first_accept_seed();
}

fn seed_visual_route_directed_death_wind(state: &mut PlayState) {
    seed_visual_route_directed_death_wind_east(state);
}

fn seed_visual_route_directed_death_wind_north(state: &mut PlayState) {
    seed_visual_route_directed_death_wind_for_direction(state, Direction::North);
}

fn seed_visual_route_directed_death_wind_east(state: &mut PlayState) {
    seed_visual_route_directed_death_wind_for_direction(state, Direction::East);
}

fn seed_visual_route_directed_death_wind_south(state: &mut PlayState) {
    seed_visual_route_directed_death_wind_for_direction(state, Direction::South);
}

fn seed_visual_route_directed_death_wind_west(state: &mut PlayState) {
    seed_visual_route_directed_death_wind_for_direction(state, Direction::West);
}

fn seed_visual_route_directed_death_wind_for_direction(
    state: &mut PlayState,
    direction: Direction,
) {
    seed_visual_route_directed_wind_combat(
        state,
        DEATH_WIND_SPELL_INDEX,
        DEATH_WIND_COST,
        2,
        Some(1),
        true,
        direction,
    );
}

fn seed_visual_route_directed_flame_wind(state: &mut PlayState) {
    seed_visual_route_directed_flame_wind_east(state);
}

fn seed_visual_route_directed_flame_wind_north(state: &mut PlayState) {
    seed_visual_route_directed_flame_wind_for_direction(state, Direction::North);
}

fn seed_visual_route_directed_flame_wind_east(state: &mut PlayState) {
    seed_visual_route_directed_flame_wind_for_direction(state, Direction::East);
}

fn seed_visual_route_directed_flame_wind_south(state: &mut PlayState) {
    seed_visual_route_directed_flame_wind_for_direction(state, Direction::South);
}

fn seed_visual_route_directed_flame_wind_west(state: &mut PlayState) {
    seed_visual_route_directed_flame_wind_for_direction(state, Direction::West);
}

fn seed_visual_route_directed_flame_wind_for_direction(
    state: &mut PlayState,
    direction: Direction,
) {
    seed_visual_route_directed_wind_combat(
        state,
        FLAME_WIND_SPELL_INDEX,
        FLAME_WIND_COST,
        1,
        None,
        true,
        direction,
    );
}

fn seed_visual_route_combat_field(state: &mut PlayState, spell_index: usize, cost: u8) {
    state.party = vec![route_visual_party_member(0, b'A', b'G', 20, 20)];
    state.party_names = default_party_names(1);
    state.party_experience = vec![0];
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
    active_objects[0] = visual_route_combat_active_object(0x4c, 5, 5, 0);
    state
        .enter_combat_frame(active_objects, actors)
        .expect("visual route combat field frame should seed");
}

fn seed_visual_route_combat_fire_field(state: &mut PlayState) {
    seed_visual_route_combat_field(state, FIRE_FIELD_SPELL_INDEX, FIELD_SPELL_COST);
}

fn seed_visual_route_combat_poison_field(state: &mut PlayState) {
    seed_visual_route_combat_field(state, POISON_FIELD_SPELL_INDEX, FIELD_SPELL_COST);
}

fn seed_visual_route_combat_sleep_field(state: &mut PlayState) {
    seed_visual_route_combat_field(state, SLEEP_FIELD_SPELL_INDEX, FIELD_SPELL_COST);
}

fn seed_visual_route_combat_energy_field(state: &mut PlayState) {
    seed_visual_route_combat_field(state, ENERGY_FIELD_SPELL_INDEX, ENERGY_FIELD_COST);
}

fn seed_visual_route_combat_dispel_field(state: &mut PlayState) {
    seed_visual_route_combat_dispel_route(state, true);
}

fn seed_visual_route_combat_dispel_empty(state: &mut PlayState) {
    seed_visual_route_combat_dispel_route(state, false);
}

fn seed_visual_route_combat_dispel_route(state: &mut PlayState, place_field: bool) {
    state.party = vec![route_visual_party_member(0, b'A', b'G', 20, 20)];
    state.party_names = default_party_names(1);
    state.party_experience = vec![0];
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
    active_objects[0] = visual_route_combat_active_object(0x4c, 5, 5, 0);
    if place_field {
        active_objects[6] = visual_route_combat_active_object(COMBAT_FIELD_KIND_FIRE, 6, 5, 0);
    }
    state
        .enter_combat_frame(active_objects, actors)
        .expect("visual route combat field dispel frame should seed");
}

fn seed_visual_route_combat_utility_failure(state: &mut PlayState, spell_index: usize, cost: u8) {
    state.party = vec![route_visual_party_member(0, b'A', b'G', 20, 20)];
    state.party_names = default_party_names(1);
    state.party_experience = vec![0];
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
    active_objects[0] = visual_route_combat_active_object(0x4c, 5, 5, 0);
    active_objects[1] = visual_route_combat_active_object(0x50, 6, 5, 0);
    state
        .enter_combat_frame(active_objects, actors)
        .expect("visual route combat utility frame should seed");
    state.combat_terrain[5][6] = 0x97;
}

fn seed_visual_route_combat_utility_vanish(state: &mut PlayState) {
    seed_visual_route_combat_utility_failure(state, VANISH_SPELL_INDEX, VANISH_COST);
}

fn seed_visual_route_combat_utility_open(state: &mut PlayState) {
    seed_visual_route_combat_utility_failure(state, OPEN_SPELL_INDEX, OPEN_SPELL_COST);
}

fn seed_visual_route_combat_utility_magic_lock(state: &mut PlayState) {
    seed_visual_route_combat_utility_failure(state, MAGIC_LOCK_SPELL_INDEX, MAGIC_LOCK_COST);
}

fn seed_visual_route_combat_utility_unlock_magic(state: &mut PlayState) {
    seed_visual_route_combat_utility_failure(state, UNLOCK_MAGIC_SPELL_INDEX, UNLOCK_MAGIC_COST);
}

fn seed_visual_route_combat_magic_missile(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "GP");
}

fn seed_visual_route_combat_fireball(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "FV");
}

fn seed_visual_route_combat_reveal(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "QW");
}

fn seed_visual_route_combat_invisibility(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "LS");
}

fn seed_visual_route_combat_cause_fear(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "CIQ");
}

fn seed_visual_route_combat_mass_charm(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "AQW");
}

fn seed_visual_route_combat_tremor(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "IPVY");
}

fn seed_visual_route_combat_repel_undead(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "ACX");
}

fn seed_visual_route_combat_charm(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "AEX");
}

fn seed_visual_route_combat_polymorph(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "BRX");
}

fn seed_visual_route_combat_clone(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "IQX");
}

fn seed_visual_route_combat_conjure(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "KX");
}

fn seed_visual_route_combat_swarm(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "BIX");
}

fn seed_visual_route_combat_summon_daemon(state: &mut PlayState) {
    seed_visual_route_combat_spell(state, "CKX");
}

fn seed_visual_route_combat_kill_gazer(state: &mut PlayState) {
    seed_visual_route_combat_special_death(state, 28);
}

fn seed_visual_route_combat_kill_gargoyle(state: &mut PlayState) {
    seed_visual_route_combat_special_death(state, 30);
}

fn seed_visual_route_combat_kill_shadowlord(state: &mut PlayState) {
    seed_visual_route_combat_special_death(state, 47);
}

fn seed_visual_route_combat_spell(state: &mut PlayState, code: &str) {
    let spell_index = spell_index_from_code(code).expect("visual route combat spell code is valid");
    let cost = spell_mp_cost(spell_index).expect("visual route combat spell cost is valid");

    state.party = vec![route_visual_party_member(0, b'A', b'G', 99, 99)];
    state.party_names = default_party_names(1);
    state.party_experience = vec![0];
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
        "FV" => first_nonzero_prng_roll_seed(29),
        "IPVY" => first_nonzero_prng_roll_seed(19),
        _ => 0,
    };

    let mut actors = [CombatActorDescriptor::empty(); COMBAT_ACTOR_SLOTS];
    actors[0] =
        CombatActorDescriptor::from_row([99, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    active_objects[0] = visual_route_combat_active_object(0x4c, 5, 5, 0);

    match code {
        "ACX" => {
            seed_visual_route_combat_monster(&mut actors, &mut active_objects, 23, 6, 4, 5);
            seed_visual_route_combat_monster(&mut actors, &mut active_objects, 33, 7, 5, 4);
            seed_visual_route_combat_monster(&mut actors, &mut active_objects, 32, 8, 6, 5);
        }
        "KX" | "BIX" | "CKX" => {}
        _ => {
            let class = if matches!(code, "BRX" | "FV" | "IPVY") {
                39
            } else {
                COMBAT_CLASS_GIANT_RAT
            };
            seed_visual_route_combat_monster(&mut actors, &mut active_objects, class, 6, 6, 5);
            if code == "QW" {
                actors[6].flags |= COMBAT_ACTOR_FLAG_HIDDEN_OR_UNREVEALED;
            }
        }
    }

    state
        .enter_combat_frame_with_terrain(active_objects, actors, combat_terrain)
        .expect("visual route combat spell frame should seed");
}

fn seed_visual_route_combat_special_death(state: &mut PlayState, class: u8) {
    let spell_index = spell_index_from_code("CX").expect("visual route Kill spell code is valid");
    let cost = spell_mp_cost(spell_index).expect("visual route Kill spell cost is valid");

    state.party = vec![route_visual_party_member(0, b'A', b'G', 99, 99)];
    state.party_names = default_party_names(1);
    state.party_experience = vec![0];
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
        CombatActorDescriptor::from_row([99, 1, COMBAT_ACTOR_FLAG_SELECTABLE_80, 0, 0, 0, 5, 5]);

    let mut active_objects = vec![ActiveObject::empty(); COMBAT_ACTOR_SLOTS];
    active_objects[0] = visual_route_combat_active_object(0x4c, 5, 5, 0);
    seed_visual_route_combat_monster(
        &mut actors,
        &mut active_objects,
        class,
        COMBAT_PARTY_ACTOR_SLOTS,
        6,
        5,
    );
    seed_visual_route_combat_monster(
        &mut actors,
        &mut active_objects,
        COMBAT_CLASS_GIANT_RAT,
        COMBAT_PARTY_ACTOR_SLOTS + 1,
        8,
        5,
    );

    state
        .enter_combat_frame_with_terrain(
            active_objects,
            actors,
            [[0x04; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE],
        )
        .expect("visual route combat special death frame should seed");
}

fn seed_visual_route_combat_monster(
    actors: &mut [CombatActorDescriptor; COMBAT_ACTOR_SLOTS],
    active_objects: &mut [ActiveObject],
    class: u8,
    slot: usize,
    x: u8,
    y: u8,
) {
    let stats = combat_class_stats(class).expect("visual route combat monster stats exist");
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
        .expect("visual route combat monster active object exists");
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

fn poison_wind_first_accept_seed() -> u16 {
    for candidate in 0..=u16::MAX {
        let mut state = candidate;
        if u5_prng_range_u16(&mut state, 0, 19) & 1 == 0 {
            return candidate;
        }
    }
    unreachable!("PRNG range cycle must hit a Poison Wind acceptance roll")
}

fn seed_visual_route_dungeon_level_spells(state: &mut PlayState) {
    state.spell_charges[UUS_POR_SPELL_INDEX] = 1;
    state.spell_charges[DES_POR_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = DUNGEON_LEVEL_SPELL_COST * 2;
        caster.level = DUNGEON_LEVEL_SPELL_COST;
    }
}

fn seed_visual_route_dungeon_field_cycle(state: &mut PlayState) {
    state.player.x = 1;
    state.player.y = 1;
    state.player.facing = Direction::East;
    let target = dungeon_cell_index(0, 2, 1);
    if let Some(cell) = state.grid.get_mut(target) {
        *cell = 0x00;
    }
    state.spell_charges[FIRE_FIELD_SPELL_INDEX] = 1;
    state.spell_charges[POISON_FIELD_SPELL_INDEX] = 1;
    state.spell_charges[SLEEP_FIELD_SPELL_INDEX] = 1;
    state.spell_charges[ENERGY_FIELD_SPELL_INDEX] = 1;
    state.spell_charges[DISPEL_FIELD_SPELL_INDEX] = 4;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = FIELD_SPELL_COST * 3 + ENERGY_FIELD_COST + DISPEL_FIELD_COST * 4;
        caster.level = ENERGY_FIELD_COST.max(DISPEL_FIELD_COST);
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_dungeon_open_chest(state: &mut PlayState) {
    state.player.x = 1;
    state.player.y = 1;
    let current = dungeon_cell_index(0, state.player.x, state.player.y);
    if let Some(cell) = state.grid.get_mut(current) {
        *cell = 0x40;
    }
    state.spell_charges[OPEN_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = OPEN_SPELL_COST;
        caster.level = OPEN_SPELL_COST;
    }
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_peer_spell(state: &mut PlayState) {
    state.spell_charges[PEER_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = PEER_COST + 1;
        caster.level = PEER_COST;
    }
}

fn seed_visual_route_x_ray_spell(state: &mut PlayState) {
    state.spell_charges[X_RAY_SPELL_INDEX] = 1;
    if let Some(caster) = state.party.first_mut() {
        caster.mana = X_RAY_COST + 1;
        caster.level = X_RAY_COST;
    }
}

fn seed_visual_route_poison_gas(state: &mut PlayState) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = Direction::East;
    let target_x = state.player.x + 1;
    let target_y = state.player.y;
    let target_idx = target_y * TOWN_GRID_SIDE + target_x;
    if let Some(cell) = state.grid.get_mut(target_idx) {
        *cell = TOWN_POISON_GAS_LIVE_TILE;
    }
    state.prng_state = poison_gas_first_poison_seed();
    state.sync_player_object();
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

fn seed_visual_route_arms_local(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::IolosBows, 999);
}

fn seed_visual_route_arms_shop(state: &mut PlayState, shop: ArmsShop, gold: u16) {
    state.gold = gold;
    if let Some(intelligence) = state.party_intelligence.first_mut() {
        *intelligence = 20;
    }
    state.equipment_stock.fill(0);
    state.active_shop = Some(ActiveShopSession::ArmsLocal(ArmsShopState::Greeting, shop));
}

fn seed_visual_route_arms_iolos_bows(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::IolosBows, 9999);
}

fn seed_visual_route_arms_naughty_nomaans(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::NaughtyNomaans, 9999);
}

fn seed_visual_route_arms_arms_of_justice(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::ArmsOfJustice, 9999);
}

fn seed_visual_route_arms_darkwatch_armoury(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::DarkwatchArmoury, 9999);
}

fn seed_visual_route_arms_paladins_protectorate(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::ThePaladinsProtectorate, 9999);
}

fn seed_visual_route_arms_north_star_armoury(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::NorthStarArmoury, 9999);
}

fn seed_visual_route_arms_buccaneers_booty(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::BuccaneersBooty, 9999);
}

fn seed_visual_route_arms_shattered_shield(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::TheShatteredShield, 9999);
}

fn seed_visual_route_arms_siege_crafters(state: &mut PlayState) {
    seed_visual_route_arms_shop(state, ArmsShop::SiegeCrafters, 9999);
}

fn seed_visual_route_healer(state: &mut PlayState) {
    seed_visual_route_healer_member(state, b'G', 3, 30);
}

fn seed_visual_route_healer_cure(state: &mut PlayState) {
    seed_visual_route_healer_member(state, b'P', 3, 30);
}

fn seed_visual_route_healer_heal(state: &mut PlayState) {
    seed_visual_route_healer_member(state, b'G', 3, 30);
}

fn seed_visual_route_healer_resurrect(state: &mut PlayState) {
    seed_visual_route_healer_member(state, b'D', 0, 30);
}

fn seed_visual_route_healer_member(state: &mut PlayState, status: u8, hp: u16, max_hp: u16) {
    state.gold = 999;
    if let Some(member) = state.party.first_mut() {
        member.status = status;
        member.hp = hp;
        member.max_hp = member.max_hp.max(max_hp);
    }
    state.active_shop = Some(ActiveShopSession::Healer(
        HealerShopState::Greeting,
        Healer::WoundsOfHonour,
    ));
}

fn seed_visual_route_inn_rest_decline(state: &mut PlayState) {
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::for_inn(
        Inn::TheWayfarerInn,
    )));
}

fn seed_visual_route_inn_rest_accept(state: &mut PlayState) {
    state.gold = 999;
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
    state.active_shop = Some(ActiveShopSession::Innkeeper(InnkeeperState::for_inn(
        Inn::TheWayfarerInn,
    )));
}

fn seed_visual_route_reagent(state: &mut PlayState) {
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::for_herbalist(
        Herbalist::TheHerbalist,
    )));
}

fn seed_visual_route_tavern(state: &mut PlayState) {
    state.gold = 999;
    state.food = 0;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(
        Tavern::TheSwordAndKeg,
    )));
}

fn seed_visual_route_tavern_lore(state: &mut PlayState, tavern: Tavern) {
    state.gold = 999;
    state.prng_state = 0x3456;
    state.active_shop = Some(ActiveShopSession::Tavern(TavernState::for_tavern(tavern)));
}

fn seed_visual_route_tavern_honest_meal_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheHonestMeal);
}

fn seed_visual_route_tavern_wayfarer_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheWayfarerTavern);
}

fn seed_visual_route_tavern_sword_and_keg_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheSwordAndKeg);
}

fn seed_visual_route_tavern_slaughtered_lamb_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheSlaughteredLamb);
}

fn seed_visual_route_tavern_humble_palate_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheHumblePalate);
}

fn seed_visual_route_tavern_blue_boar_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheBlueBoarTavern);
}

fn seed_visual_route_tavern_cats_lair_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheCatsLair);
}

fn seed_visual_route_tavern_fallen_virgin_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheFallenVirgin);
}

fn seed_visual_route_tavern_folley_tap_lore(state: &mut PlayState) {
    seed_visual_route_tavern_lore(state, Tavern::TheFolleyTap);
}

fn seed_visual_route_horse_trader(state: &mut PlayState, stable: Stable) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = Direction::South;
    let target_idx = (state.player.y + 1) * TOWN_GRID_SIDE + state.player.x;
    if let Some(cell) = state.grid.get_mut(target_idx) {
        *cell = 0x05;
    }
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::HorseTrader(
        HorseTraderState::for_stable(stable),
    ));
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_horse_trader_decline(state: &mut PlayState) {
    seed_visual_route_horse_trader(state, Stable::HorseAndRider);
}

fn seed_visual_route_horse_trader_no_marker(state: &mut PlayState) {
    state.player.x = 15;
    state.player.y = 15;
    state.player.facing = Direction::South;
    for (x, y) in [(15, 16), (15, 14), (16, 15), (14, 15)] {
        let idx = y * TOWN_GRID_SIDE + x;
        if let Some(cell) = state.grid.get_mut(idx) {
            *cell = 0x00;
        }
    }
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::HorseTrader(
        HorseTraderState::for_stable(Stable::HorseAndRider),
    ));
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_horse_trader_horse_and_rider(state: &mut PlayState) {
    seed_visual_route_horse_trader(state, Stable::HorseAndRider);
}

fn seed_visual_route_horse_trader_stablehouse(state: &mut PlayState) {
    seed_visual_route_horse_trader(state, Stable::TheStablehouse);
}

fn seed_visual_route_horse_trader_wishing_well(state: &mut PlayState) {
    seed_visual_route_horse_trader(state, Stable::WishingWellHorses);
}

fn seed_visual_route_sage_paid(state: &mut PlayState) {
    state.gold = 100;
    state.prng_state = 0x3456;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
}

fn seed_visual_route_sage_short_funds(state: &mut PlayState) {
    state.gold = 49;
    state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));
}

fn seed_visual_route_shipwright(state: &mut PlayState) {
    seed_visual_route_shipwright_shop(state, Shipwright::IslandShipwrights);
}

fn seed_visual_route_shipwright_island(state: &mut PlayState) {
    seed_visual_route_shipwright_shop(state, Shipwright::IslandShipwrights);
}

fn seed_visual_route_shipwright_crows_nest(state: &mut PlayState) {
    seed_visual_route_shipwright_shop(state, Shipwright::TheCrowsNest);
}

fn seed_visual_route_shipwright_oaken_oar(state: &mut PlayState) {
    seed_visual_route_shipwright_shop(state, Shipwright::TheOakenOar);
}

fn seed_visual_route_shipwright_rusty_bucket(state: &mut PlayState) {
    seed_visual_route_shipwright_shop(state, Shipwright::TheRustyBucket);
}

fn seed_visual_route_shipwright_shop(state: &mut PlayState, shipwright: Shipwright) {
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

fn seed_visual_route_guild(state: &mut PlayState) {
    state.gold = 999;
    state.active_shop = Some(ActiveShopSession::Guild(GuildShopState::for_shop(
        GuildShop::TheGuild,
    )));
}

fn seed_visual_route_dungeon_heavy_door_variant(state: &mut PlayState) {
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

fn seed_visual_route_dungeon_ladder(state: &mut PlayState) {
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

fn seed_visual_route_dungeon_surface_exit(state: &mut PlayState) {
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

fn stamp_visual_route_look_tile(state: &mut PlayState, tile: u8) {
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

fn seed_visual_route_fountain(state: &mut PlayState) {
    stamp_visual_route_look_tile(state, 0xD8);
}

fn seed_visual_route_yew_wanted_poster(state: &mut PlayState) {
    state.player.x = 16;
    state.player.y = 21;
    state.player.facing = Direction::East;
    let floor = state.current_floor().unwrap_or(0);
    state.active_objects.push(ActiveObject {
        type_byte: 0xA0,
        tile: 0xA0,
        x: 17,
        y: 21,
        z: floor,
        phase: 0,
        aux1: 0,
        aux3: 0,
    });
    state.sync_player_object();
    state.mark_visibility_dirty();
}

fn seed_visual_route_town_scheduled_npc(
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
    state.clock = GameClock::new(8, 0).expect("visual route clock is valid");
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

fn seed_visual_route_town_attack_death_mask_npc(state: &mut PlayState) {
    seed_visual_route_town_scheduled_npc(state, 1, 0x0E, 2, 1, 0);
}

fn seed_visual_route_town_attack_guard_alarm(state: &mut PlayState) {
    seed_visual_route_town_scheduled_npc(state, 1, 0x70, 2, 1, 0);
}

fn seed_visual_route_town_hostile_adjacent_alarm(state: &mut PlayState) {
    seed_visual_route_town_scheduled_npc(state, 1, 0x50, 6, 5, 4);
}

fn seed_visual_route_town_guard_arrest(state: &mut PlayState) {
    seed_visual_route_town_scheduled_npc(state, 2, 0x70, 6, 5, 6);
}

fn seed_visual_route_wishing_well(state: &mut PlayState) {
    stamp_visual_route_look_tile(state, 0xA1);
}

fn seed_visual_route_death_vision(state: &mut PlayState) {
    stamp_visual_route_look_tile(state, 0x00);
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

fn seed_visual_route_shadowlord_shard(state: &mut PlayState, index: usize, x: usize, y: usize) {
    state.player.x = x;
    state.player.y = y;
    state.player.facing = Direction::South;
    state.sync_player_object();
    let tile = SHADOWLORD_OBJECT_TILE_BASE + index as u8;
    state.active_objects.push(ActiveObject {
        type_byte: tile,
        tile,
        x,
        y: y.saturating_sub(1),
        z: state.current_floor().unwrap_or(0),
        phase: STEADY_PHASE,
        aux1: index as u8,
        aux3: state.shadowlord_hideouts.get(index).copied().unwrap_or(0),
    });
    state.mark_visibility_dirty();
}

fn seed_visual_route_falsehood_shard(state: &mut PlayState) {
    seed_visual_route_shadowlord_shard(state, SHADOWLORD_FALSEHOOD_INDEX, 15, 9);
}

fn seed_visual_route_hatred_shard(state: &mut PlayState) {
    seed_visual_route_shadowlord_shard(state, SHADOWLORD_HATRED_INDEX, 15, 3);
}

fn seed_visual_route_cowardice_shard(state: &mut PlayState) {
    seed_visual_route_shadowlord_shard(state, SHADOWLORD_COWARDICE_INDEX, 15, 16);
}

fn seed_visual_route_word_of_power(state: &mut PlayState, word: &str) {
    if let Some(seal) = word_of_power_seal_for_word(word) {
        if matches!(state.area, u5_runtime::Area::World { plane } if plane == seal.plane) {
            state.player.x = seal.x;
            state.player.y = seal.y;
            state.sync_player_object();
            let idx = seal.y * WORLD_SIDE + seal.x;
            if let Some(cell) = state.grid.get_mut(idx) {
                *cell = seal.closed_tile;
            }
            state.mark_visibility_dirty();
        }
    }
}

fn seed_visual_route_britannia_word_of_power(state: &mut PlayState) {
    seed_visual_route_word_of_power(state, "FALLAX");
}

fn seed_visual_route_underworld_word_of_power(state: &mut PlayState) {
    seed_visual_route_word_of_power(state, "VERAMOCOR");
}

fn seed_visual_route_endgame_missing_box(state: &mut PlayState) {
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 0;
}

fn seed_visual_route_endgame_victory(state: &mut PlayState) {
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
}

fn seed_visual_route_endgame_class_tableau(state: &mut PlayState) {
    let template = state.party.first().copied().unwrap_or(PartyMember {
        slot: 0,
        class_byte: b'A',
        status: b'G',
        climb_stat: DEFAULT_CLIMB_STAT,
        mana: 0,
        hp: 30,
        max_hp: 30,
        level: 1,
    });
    let classes = [b'A', b'M', b'B', b'F', b'D', b'R'];
    state.party = classes
        .iter()
        .enumerate()
        .map(|(slot, class)| PartyMember {
            slot: slot as u8,
            class_byte: *class,
            status: if slot == 4 { b'D' } else { b'G' },
            hp: if slot == 4 { 0 } else { template.hp.max(1) },
            max_hp: template.max_hp.max(30) + slot as u16,
            ..template
        })
        .collect();
    state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = 0;
}

fn apply_visual_route_command(
    state: &mut PlayState,
    command: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let command = command.trim();
    let lower = command.to_ascii_lowercase();
    if lower == "setup:blackthorn-audience" {
        state.begin_blackthorn_audience_capture(game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if lower == "setup:blackthorn-rescue" {
        state.apply_blackthorn_rescue_refuge(game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if lower == "setup:whirlpool-engagement" {
        if state
            .apply_world_whirlpool_engagement(game_dir, WorldPlane::Britannia)?
            .is_none()
        {
            return Err(io::Error::other(
                "seeded visual route did not find adjacent whirlpool object",
            ));
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if lower == "setup:terrain-combat-party-entry" {
        seed_visual_route_terrain_combat_party_entry(state, game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if lower == "setup:terrain-combat-no-foes" {
        seed_visual_route_terrain_combat_party_entry(state, game_dir)?;
        for slot in COMBAT_PARTY_ACTOR_SLOTS..COMBAT_ACTOR_SLOTS {
            state.combat_actors[slot].clear();
            if let Some(object) = state.active_objects.get_mut(slot) {
                *object = ActiveObject::empty();
            }
        }
        state.active_player = Some(0);
        state.pending_combat_actor_slot = Some(0);
        return Ok(PlayInputDisposition::Continue);
    }
    if lower == "setup:terrain-combat-east-edge" {
        seed_visual_route_terrain_combat_party_entry(state, game_dir)?;
        state.active_player = Some(0);
        state.pending_combat_actor_slot = Some(0);
        if let Some(actor) = state.combat_actors.get_mut(0) {
            actor.x = (COMBAT_ARENA_SIDE - 1) as u8;
            actor.y = 5;
        }
        if let Some(object) = state.active_objects.get_mut(0) {
            object.x = COMBAT_ARENA_SIDE - 1;
            object.y = 5;
        }
        return Ok(PlayInputDisposition::Continue);
    }
    if lower == "setup:dungeon-room-party-entry" {
        seed_visual_route_combat_entry_party(state);
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
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(lower.as_str(), "empty" | "pass") {
        handle_play_key_input(state, '\n', "", game_dir)?;
        return Ok(PlayInputDisposition::Continue);
    }
    if matches!(lower.as_str(), "idle" | "tick" | "ticks") {
        return handle_play_key_input(state, '.', "", game_dir);
    }
    if let Some(value) = lower
        .strip_prefix("idle:")
        .or_else(|| lower.strip_prefix("tick:"))
    {
        let count = value.parse::<usize>().map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("visual route idle command `{command}` has invalid tick count: {err}"),
            )
        })?;
        if count == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("visual route idle command `{command}` must request at least one tick"),
            ));
        }
        for _ in 0..count {
            if handle_play_key_input(state, '.', "", game_dir)? == PlayInputDisposition::Quit {
                return Ok(PlayInputDisposition::Quit);
            }
        }
        return Ok(PlayInputDisposition::Continue);
    }
    let mut chars = command.chars();
    let Some(key) = chars.next() else {
        return handle_play_key_input(state, '\n', "", game_dir);
    };
    handle_play_key_input(state, key, chars.as_str(), game_dir)
}

fn seed_visual_route_terrain_combat_party_entry(
    state: &mut PlayState,
    game_dir: &Path,
) -> io::Result<()> {
    seed_visual_route_combat_entry_party(state);
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

fn visual_route_step_label(route_label: &str, step: usize, command: &str) -> String {
    let mut command_label = String::with_capacity(command.len().max(5));
    for ch in command.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            command_label.push(ch.to_ascii_lowercase());
        } else if ch == '.' {
            command_label.push_str("idle");
        } else {
            command_label.push('_');
        }
    }
    if command_label.is_empty() {
        command_label.push_str("empty");
    }
    format!("{route_label}-{step:02}-{command_label}")
}

fn visual_route_allows_unchanged_step(route_label: &str, step: usize) -> bool {
    (route_label == "route-endgame-box-full-victory-cinematic" && (3..=18).contains(&step))
        || (route_label == "route-doom-combat-multi-round-pass" && (2..=5).contains(&step))
        || (route_label == "route-castle-light-decay-route" && (1..=2).contains(&step))
        || (route_label.starts_with("route-shop-arms-")
            && route_label.ends_with("-terminator-refusal")
            && (1..=3).contains(&step))
}

fn run_visual_intro_menu_app(
    game_dir: PathBuf,
    raster_depth: TileGraphicsDepth,
    launch_result: Arc<Mutex<Option<PlayOptions>>>,
) {
    let screenshot_path: Option<PathBuf> =
        std::env::var("U5_BEVY_SCREENSHOT").ok().map(PathBuf::from);
    let screenshot_delay: u32 = std::env::var("U5_BEVY_SCREENSHOT_DELAY")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    let preset_keys: Vec<char> = std::env::var("U5_BEVY_PRESS")
        .ok()
        .map(|s| s.chars().collect())
        .unwrap_or_default();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Ultima V Intro".into(),
                resolution: (820.0, 620.0).into(),
                resizable: true,
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(VisualIntroState {
            game_dir,
            raster_depth,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::Menu,
            launch_result,
            image_handle: None,
        })
        .insert_resource(VisualIntroAnimationPump::default())
        .insert_resource(ScreenshotConfig {
            path: screenshot_path,
            frame_delay: screenshot_delay,
            preset_keys,
        })
        .insert_resource(ScreenshotState::default())
        .add_systems(Startup, setup_intro)
        .add_systems(
            Update,
            (
                drive_visual_intro,
                animate_visual_intro_title_effects,
                screenshot_system,
            ),
        )
        .run();
}

#[derive(Resource)]
struct ScreenshotConfig {
    path: Option<PathBuf>,
    frame_delay: u32,
    preset_keys: Vec<char>,
}

#[derive(Resource, Default)]
struct ScreenshotState {
    frames_elapsed: u32,
    preset_keys_applied: bool,
    taken: bool,
    frames_after_shot: u32,
}

fn screenshot_system(
    mut commands: Commands,
    config: Res<ScreenshotConfig>,
    mut state: ResMut<ScreenshotState>,
    visual: Option<ResMut<VisualState>>,
    intro: Option<ResMut<VisualIntroState>>,
    mut images: ResMut<Assets<Image>>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(path) = config.path.clone() else {
        return;
    };
    state.frames_elapsed += 1;

    // Apply preset keystrokes directly to PlayState (bypasses the keyboard
    // system) before the screenshot delay finishes counting down.
    if !state.preset_keys_applied && !config.preset_keys.is_empty() {
        if let Some(mut visual) = visual {
            let game_dir = visual.game_dir.clone();
            for ch in &config.preset_keys {
                let _ = handle_play_key_input(&mut visual.state, *ch, "", &game_dir);
            }
            // Re-render the framebuffer to reflect the new state.
            let v: &mut VisualState = visual.as_mut();
            let rgba =
                render_visual_play_frame_with_input(&mut v.state, &v.atlas, &v.text_font, "", "");
            if let Some(image) = images.get_mut(&v.image_handle) {
                image.data = Some(rgba);
            }
            state.preset_keys_applied = true;
        } else if let Some(mut intro) = intro {
            let mut handled = false;
            for ch in &config.preset_keys {
                handled |= step_visual_intro(&mut intro, *ch);
            }
            if handled {
                let rgba = render_intro_frame(&mut intro);
                if let Some(handle) = intro.image_handle.as_ref() {
                    if let Some(image) = images.get_mut(handle) {
                        image.data = Some(rgba);
                    }
                }
            }
            state.preset_keys_applied = true;
        }
    }

    if !state.taken && state.frames_elapsed >= config.frame_delay {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
        state.taken = true;
    }
    if state.taken {
        state.frames_after_shot += 1;
        // Give the encoder a few frames to flush the PNG to disk.
        if state.frames_after_shot >= 30 {
            exit.write(AppExit::Success);
        }
    }
}

struct Bootstrap {
    game_dir: PathBuf,
    state: PlayState,
    atlas: TileAtlas,
    text_font: FixedCellFont,
}

#[derive(Resource)]
struct PendingBootstrap(Mutex<Option<Bootstrap>>);

#[derive(Resource)]
struct VisualState {
    game_dir: PathBuf,
    state: PlayState,
    atlas: TileAtlas,
    image_handle: Handle<Image>,
    text_font: FixedCellFont,
    input_line: String,
    prompt_cursor_visible: bool,
}

#[derive(Resource)]
struct VisualIntroState {
    game_dir: PathBuf,
    raster_depth: TileGraphicsDepth,
    dispatch: UnifiedMenuDispatch,
    title_flourish_step: usize,
    title_flourish_complete: bool,
    title_signature_progress: usize,
    title_signature_complete: bool,
    title_tick_frame: u8,
    title_tick_visible_frame: u8,
    start_menu_reveal: Option<RectColumnSweepTransition>,
    start_menu_reveal_backing: Option<Vec<u8>>,
    modal_backing: Option<Vec<u8>>,
    menu_idle_ticks: u16,
    message_waiting_for_key: bool,
    message: String,
    panel: VisualIntroPanel,
    launch_result: Arc<Mutex<Option<PlayOptions>>>,
    image_handle: Option<Handle<Image>>,
}

#[derive(Debug, Default)]
enum VisualIntroPanel {
    #[default]
    Menu,
    CharacterCreation {
        session: ChargenSession,
        input_line: String,
    },
    U4Transfer {
        source: U4TransferSource,
        preview: U4TransferPreview,
        overrides: U4TransferOverrides,
        stage: VisualU4TransferStage,
        input_line: String,
    },
    Story {
        records: StoryRecords,
        step: usize,
        transition: Option<RectColumnSweepTransition>,
    },
    Acknowledgements,
    ReturnToView {
        summary: String,
        preview_frames_rgba: Vec<Vec<u8>>,
        frame_metadata: Vec<VisualReturnToViewFrameMeta>,
        preview_frame_index: usize,
        preview_width: usize,
        preview_height: usize,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct VisualReturnToViewPreview {
    summary: String,
    frames_rgba: Vec<Vec<u8>>,
    frame_metadata: Vec<VisualReturnToViewFrameMeta>,
    width: usize,
    height: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct VisualReturnToViewFrameMeta {
    command_index: usize,
    elapsed_title_ticks: u32,
    kind: ReturnToViewFrameKind,
    caption: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisualU4TransferStage {
    ConfirmName,
    ReplacementName,
    ConfirmGender,
    ReplacementGender,
    ConfirmCommit,
}

/// Drives the static-tile animator (water cycle) at a fixed wall-clock
/// cadence so the world looks alive even when the player isn't moving.
/// Original U5 advances frames on every render tick; we use ~3 Hz which
/// roughly matches the EGA waterfall pacing the user sees in DOSBox.
#[derive(Resource)]
struct AnimationPump {
    accumulator: f32,
    interval: f32,
}

impl Default for AnimationPump {
    fn default() -> Self {
        Self {
            accumulator: 0.0,
            interval: 0.33,
        }
    }
}

#[derive(Resource)]
struct VisualIntroAnimationPump {
    accumulator: f32,
    interval: f32,
}

impl Default for VisualIntroAnimationPump {
    fn default() -> Self {
        Self {
            accumulator: 0.0,
            interval: INTRO_ANIMATION_TICK_INTERVAL_SECS,
        }
    }
}

fn animate_static_tiles(
    time: Res<Time>,
    mut pump: ResMut<AnimationPump>,
    visual: Option<ResMut<VisualState>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut visual) = visual else {
        return;
    };
    pump.accumulator += time.delta_secs();
    let mut advanced = false;
    while pump.accumulator >= pump.interval {
        pump.accumulator -= pump.interval;
        let mut prompt_cursor_visible = visual.prompt_cursor_visible;
        advanced |= advance_visual_wait_frame(&mut visual.state, &mut prompt_cursor_visible);
        visual.prompt_cursor_visible = prompt_cursor_visible;
    }
    if !advanced {
        return;
    }
    let v: &mut VisualState = visual.as_mut();
    let input_line = v.input_line.clone();
    let rgba = render_visual_play_frame_with_input_and_cursor(
        &mut v.state,
        &v.atlas,
        &v.text_font,
        &input_line,
        "",
        v.prompt_cursor_visible,
    );
    if let Some(image) = images.get_mut(&v.image_handle) {
        image.data = Some(rgba);
    }
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    pending: Res<PendingBootstrap>,
) {
    let bootstrap = pending
        .0
        .lock()
        .expect("visual bootstrap lock poisoned")
        .take()
        .expect("visual bootstrap missing");
    let Bootstrap {
        game_dir,
        mut state,
        atlas,
        text_font,
    } = bootstrap;

    let rgba = render_visual_play_frame_with_input(&mut state, &atlas, &text_font, "", READY_HINT);
    let mut image = Image::new(
        Extent3d {
            width: VISUAL_PLAY_FRAME_WIDTH,
            height: VISUAL_PLAY_FRAME_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);
    let display_width = VISUAL_PLAY_FRAME_WIDTH as f32 * DISPLAY_SCALE;
    let display_height = VISUAL_PLAY_FRAME_HEIGHT as f32 * DISPLAY_SCALE;

    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            image: image_handle.clone(),
            custom_size: Some(Vec2::new(display_width, display_height)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));

    commands.insert_resource(VisualState {
        game_dir,
        state,
        atlas,
        image_handle,
        text_font,
        input_line: String::new(),
        prompt_cursor_visible: false,
    });
    commands.remove_resource::<PendingBootstrap>();
}

fn setup_intro(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut intro: ResMut<VisualIntroState>,
) {
    commands.spawn(Camera2d);
    let rgba = render_intro_frame(&mut intro);
    let mut image = Image::new(
        Extent3d {
            width: INTRO_FRAMEBUFFER_WIDTH,
            height: INTRO_FRAMEBUFFER_HEIGHT,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);
    intro.image_handle = Some(image_handle.clone());

    commands.spawn((
        Sprite {
            image: image_handle,
            custom_size: Some(Vec2::new(
                INTRO_FRAMEBUFFER_WIDTH as f32 * INTRO_DISPLAY_SCALE,
                INTRO_FRAMEBUFFER_HEIGHT as f32 * INTRO_DISPLAY_SCALE,
            )),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
}

fn drive_visual_intro(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    intro: Option<ResMut<VisualIntroState>>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<&mut Sprite>,
    mut windows: Query<&mut Window>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(mut intro) = intro else {
        return;
    };
    if visual_intro_start_menu_reveal_active(&intro) {
        return;
    }
    let mut handled = false;
    if keyboard.just_pressed(KeyCode::Escape) {
        if matches!(intro.panel, VisualIntroPanel::ReturnToView { .. }) {
            if visual_intro_return_to_view_complete(&intro.panel)
                && step_visual_intro_panel(&mut intro, '\x1b')
            {
                handled = true;
            }
        } else if cancel_visual_intro_panel(&mut intro) {
            handled = true;
        } else {
            exit.write(AppExit::Success);
            return;
        }
    }

    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let control_pressed =
        keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    for key in keyboard.get_just_pressed() {
        if *key == KeyCode::Escape {
            continue;
        }
        let Some(ch) = key_code_to_char(*key, shift_pressed, control_pressed) else {
            continue;
        };
        if step_visual_intro(&mut intro, ch) {
            handled = true;
        }
    }
    if handled {
        let rgba = render_intro_frame(&mut intro);
        if let Some(handle) = intro.image_handle.as_ref() {
            if let Some(image) = images.get_mut(handle) {
                image.data = Some(rgba);
            }
        }
    }
    let launch_options = intro
        .launch_result
        .lock()
        .expect("visual intro launch lock poisoned")
        .take();
    if let Some(options) = launch_options {
        transition_visual_intro_to_gameplay(
            &mut commands,
            &mut intro,
            &mut images,
            &mut sprites,
            &mut windows,
            options,
        );
    }
}

fn transition_visual_intro_to_gameplay(
    commands: &mut Commands,
    intro: &mut VisualIntroState,
    images: &mut Assets<Image>,
    sprites: &mut Query<&mut Sprite>,
    windows: &mut Query<&mut Window>,
    options: PlayOptions,
) {
    let game_dir = intro.game_dir.clone();
    let launch = PlayState::load_scene(&game_dir, options).and_then(|state| {
        let atlas = load_tile_atlas(&game_dir, intro.raster_depth)?;
        let text_font = load_ibm_ch_font(&game_dir)?;
        Ok((state, atlas, text_font))
    });
    let (mut state, atlas, text_font) = match launch {
        Ok(launch) => launch,
        Err(err) => {
            intro.message = format!("Journey Onward failed: {err}");
            intro.message_waiting_for_key = true;
            intro.menu_idle_ticks = 0;
            intro
                .dispatch
                .complete_subflow(IntroSubflow::JourneyOnward, IntroSubflowResult::Cancelled);
            return;
        }
    };
    let Some(image_handle) = intro.image_handle.clone() else {
        intro.message = "Journey Onward failed: missing visual framebuffer.".to_string();
        intro.message_waiting_for_key = true;
        intro.menu_idle_ticks = 0;
        intro
            .dispatch
            .complete_subflow(IntroSubflow::JourneyOnward, IntroSubflowResult::Cancelled);
        return;
    };
    intro.image_handle = None;
    let rgba = render_visual_play_frame_with_input(&mut state, &atlas, &text_font, "", READY_HINT);
    if let Some(image) = images.get_mut(&image_handle) {
        image.data = Some(rgba);
    }
    for mut sprite in sprites.iter_mut() {
        sprite.custom_size = Some(Vec2::new(
            VISUAL_PLAY_FRAME_WIDTH as f32 * DISPLAY_SCALE,
            VISUAL_PLAY_FRAME_HEIGHT as f32 * DISPLAY_SCALE,
        ));
    }
    if let Ok(mut window) = windows.single_mut() {
        window.title = "Ultima V".to_string();
        window.resolution.set(
            VISUAL_PLAY_FRAME_WIDTH as f32 * DISPLAY_SCALE,
            VISUAL_PLAY_FRAME_HEIGHT as f32 * DISPLAY_SCALE,
        );
    }
    commands.insert_resource(VisualState {
        game_dir,
        state,
        atlas,
        image_handle,
        text_font,
        input_line: String::new(),
        prompt_cursor_visible: false,
    });
    commands.remove_resource::<VisualIntroState>();
}

fn animate_visual_intro_title_effects(
    time: Res<Time>,
    mut pump: ResMut<VisualIntroAnimationPump>,
    intro: Option<ResMut<VisualIntroState>>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(mut intro) = intro else {
        return;
    };

    pump.accumulator += time.delta_secs();
    let mut advanced = false;
    if pump.accumulator >= pump.interval {
        pump.accumulator -= pump.interval;
        if pump.accumulator >= pump.interval {
            pump.accumulator = 0.0;
        }
        advanced = advance_visual_intro_animation_tick(&mut intro);
    }
    if !advanced {
        return;
    }

    let rgba = render_intro_frame(&mut intro);
    if let Some(handle) = intro.image_handle.as_ref() {
        if let Some(image) = images.get_mut(handle) {
            image.data = Some(rgba);
        }
    }
}

fn advance_visual_intro_animation_tick(intro: &mut VisualIntroState) -> bool {
    const SIGNATURE_STEPS_PER_TICK: usize = 24;

    if matches!(intro.panel, VisualIntroPanel::Menu) {
        let mut advanced = false;
        let title_phase = matches!(intro.dispatch.tick_title(), UnifiedMenuStep::PresentTitle);

        if !title_phase && !intro.message_waiting_for_key {
            clear_carry_visual_intro_title_tick(intro);
            advanced = true;
            advanced |= advance_visual_intro_start_menu_reveal(intro);
        }

        advanced |= advance_visual_intro_finished_menu_idle(intro);

        if title_phase && !intro.title_flourish_complete {
            if intro.title_flourish_step + 1 >= intro_title_flourish_total_steps() {
                intro.title_flourish_complete = true;
            } else {
                intro.title_flourish_step += 1;
            }
            advanced = true;
        } else if title_phase && !intro.title_signature_complete {
            let Ok(signature) = load_british_pth(&intro.game_dir) else {
                intro.title_signature_complete = true;
                return true;
            };
            let total_steps = british_signature_step_count(&signature);
            if total_steps == 0 {
                intro.title_signature_complete = true;
                return true;
            }

            intro.title_signature_progress =
                (intro.title_signature_progress + SIGNATURE_STEPS_PER_TICK).min(total_steps);
            if intro.title_signature_progress >= total_steps {
                intro.title_signature_progress = 0;
                intro.title_signature_complete = true;
            }
            advanced = true;
        }
        return advanced;
    } else {
        let mut title_tick_frame = intro.title_tick_frame;
        let title_tick_visible_frame = intro.title_tick_frame;
        if !advance_visual_intro_panel_animation(&mut intro.panel, &mut title_tick_frame) {
            return false;
        }
        intro.title_tick_frame = title_tick_frame;
        intro.title_tick_visible_frame = title_tick_visible_frame;
        true
    }
}

fn clear_carry_visual_intro_title_tick(intro: &mut VisualIntroState) {
    intro.title_tick_visible_frame = intro.title_tick_frame;
    intro.title_tick_frame = title_tick_next_frame(intro.title_tick_frame);
}

fn advance_visual_intro_panel_animation(
    panel: &mut VisualIntroPanel,
    title_tick_frame: &mut u8,
) -> bool {
    advance_visual_intro_story_auto_step(panel)
        || advance_visual_intro_story_wipe(panel, title_tick_frame)
        || advance_visual_intro_return_to_view(panel, title_tick_frame)
}

fn advance_visual_intro_story_auto_step(panel: &mut VisualIntroPanel) -> bool {
    let VisualIntroPanel::Story {
        step, transition, ..
    } = panel
    else {
        return false;
    };
    if *step + 1 >= INTRO_STORY_STEP_COUNT || intro_story_step_waits_for_input(*step) {
        return false;
    }
    *step = (*step).saturating_add(1);
    *transition = None;
    true
}

fn advance_visual_intro_start_menu_reveal(intro: &mut VisualIntroState) -> bool {
    let Some(reveal) = intro.start_menu_reveal.as_mut() else {
        return false;
    };
    if reveal.advance_title_tick() {
        intro.start_menu_reveal = None;
        intro.start_menu_reveal_backing = None;
    }
    true
}

fn advance_visual_intro_story_wipe(
    panel: &mut VisualIntroPanel,
    title_tick_frame: &mut u8,
) -> bool {
    let VisualIntroPanel::Story {
        step, transition, ..
    } = panel
    else {
        return false;
    };
    if *step != 1 {
        return false;
    }
    let Some(active_transition) = transition.as_mut() else {
        return false;
    };

    *title_tick_frame = title_tick_next_frame(*title_tick_frame);
    if active_transition.advance_title_tick() {
        *step = (*step).saturating_add(1);
        *transition = None;
    }
    true
}

fn advance_visual_intro_return_to_view(
    panel: &mut VisualIntroPanel,
    title_tick_frame: &mut u8,
) -> bool {
    let VisualIntroPanel::ReturnToView {
        preview_frames_rgba,
        preview_frame_index,
        ..
    } = panel
    else {
        return false;
    };
    let next_index = preview_frame_index.saturating_add(1);
    if next_index >= preview_frames_rgba.len() {
        return false;
    }
    *preview_frame_index = next_index;
    *title_tick_frame = title_tick_next_frame(*title_tick_frame);
    true
}

fn visual_intro_return_to_view_complete(panel: &VisualIntroPanel) -> bool {
    let VisualIntroPanel::ReturnToView {
        preview_frames_rgba,
        preview_frame_index,
        ..
    } = panel
    else {
        return false;
    };
    preview_frame_index.saturating_add(1) >= preview_frames_rgba.len()
}

fn advance_visual_intro_finished_menu_idle(intro: &mut VisualIntroState) -> bool {
    if !matches!(intro.panel, VisualIntroPanel::Menu)
        || intro.message_waiting_for_key
        || visual_intro_start_menu_reveal_active(intro)
        || matches!(intro.dispatch.tick_title(), UnifiedMenuStep::PresentTitle)
    {
        return false;
    }
    intro.menu_idle_ticks = intro.menu_idle_ticks.saturating_add(1);
    if intro.menu_idle_ticks < INTRO_MENU_IDLE_RETURN_TO_VIEW_TICKS {
        return false;
    }
    intro.menu_idle_ticks = 0;
    if matches!(
        intro.dispatch.submit_menu_key(b'R'),
        UnifiedMenuStep::EnteredSubflow(IntroSubflow::ReturnToView)
    ) {
        resolve_visual_intro_subflow(intro, IntroSubflow::ReturnToView);
        return true;
    }
    false
}

fn step_visual_intro(intro: &mut VisualIntroState, ch: char) -> bool {
    if visual_intro_start_menu_reveal_active(intro) {
        return false;
    }
    if !matches!(intro.panel, VisualIntroPanel::Menu) {
        return step_visual_intro_panel(intro, ch);
    }

    if matches!(intro.dispatch.tick_title(), UnifiedMenuStep::PresentTitle) {
        intro.dispatch.dismiss_title();
        intro.title_flourish_step = 0;
        intro.title_flourish_complete = true;
        intro.title_signature_progress = 0;
        intro.title_signature_complete = true;
        intro.title_tick_frame = 0;
        intro.title_tick_visible_frame = 0;
        clear_carry_visual_intro_title_tick(intro);
        intro.menu_idle_ticks = 0;
        intro.start_menu_reveal = None;
        intro.start_menu_reveal_backing = None;
        intro.modal_backing = None;
        intro.message_waiting_for_key = false;
        if ch.eq_ignore_ascii_case(&'J') {
            return resolve_visual_intro_subflow(intro, IntroSubflow::JourneyOnward);
        }
        intro.message.clear();
        return true;
    }

    if intro.message_waiting_for_key {
        intro.message_waiting_for_key = false;
        intro.message.clear();
        clear_carry_visual_intro_title_tick(intro);
        intro.menu_idle_ticks = 0;
        return true;
    }

    intro.menu_idle_ticks = 0;
    let key = if ch == '\r' { b'\r' } else { ch as u8 };
    match intro.dispatch.submit_menu_key(key) {
        UnifiedMenuStep::EnteredSubflow(subflow) => resolve_visual_intro_subflow(intro, subflow),
        UnifiedMenuStep::Ignored => true,
        UnifiedMenuStep::PresentMenu | UnifiedMenuStep::ReturnedToMenu => true,
        UnifiedMenuStep::LaunchGameplay => true,
        UnifiedMenuStep::PresentTitle
        | UnifiedMenuStep::CodexAdvanced(_)
        | UnifiedMenuStep::CodexCompleted
        | UnifiedMenuStep::BlackthornAdvanced
        | UnifiedMenuStep::BlackthornEnded { .. }
        | UnifiedMenuStep::U4Stepped => false,
    }
}

enum VisualIntroPanelOutcome {
    Stay,
    ReturnToMenu {
        subflow: IntroSubflow,
        result: IntroSubflowResult,
        message: String,
    },
    CommitChargen(ChargenSessionResult),
    CommitU4Transfer {
        source: U4TransferSource,
        overrides: U4TransferOverrides,
    },
}

fn step_visual_intro_panel(intro: &mut VisualIntroState, ch: char) -> bool {
    let outcome = match &mut intro.panel {
        VisualIntroPanel::Menu => return false,
        VisualIntroPanel::CharacterCreation {
            session,
            input_line,
        } => step_visual_chargen_panel(session, input_line, ch),
        VisualIntroPanel::U4Transfer {
            source,
            overrides,
            stage,
            input_line,
            ..
        } => step_visual_u4_transfer_panel(source, overrides, stage, input_line, ch),
        VisualIntroPanel::Story {
            step, transition, ..
        } => {
            if *step == 1 {
                if transition.is_none() {
                    *transition =
                        Some(RectColumnSweepTransition::new(INTRO_STEP_1_RECT_TRANSITION));
                }
                VisualIntroPanelOutcome::Stay
            } else if *step + 1 < INTRO_STORY_STEP_COUNT {
                *step += 1;
                VisualIntroPanelOutcome::Stay
            } else {
                VisualIntroPanelOutcome::ReturnToMenu {
                    subflow: IntroSubflow::StorySlides,
                    result: IntroSubflowResult::ReturnedToMenu,
                    message: "Ultima V Introduction complete.".to_string(),
                }
            }
        }
        VisualIntroPanel::Acknowledgements => VisualIntroPanelOutcome::ReturnToMenu {
            subflow: IntroSubflow::Acknowledgements,
            result: IntroSubflowResult::ReturnedToMenu,
            message: "Acknowledgements complete.".to_string(),
        },
        VisualIntroPanel::ReturnToView {
            preview_frames_rgba,
            preview_frame_index,
            ..
        } => {
            if preview_frame_index.saturating_add(1) >= preview_frames_rgba.len() {
                VisualIntroPanelOutcome::ReturnToMenu {
                    subflow: IntroSubflow::ReturnToView,
                    result: IntroSubflowResult::ReturnedToMenu,
                    message: "Return-to-View preview complete.".to_string(),
                }
            } else {
                VisualIntroPanelOutcome::Stay
            }
        }
    };

    match outcome {
        VisualIntroPanelOutcome::Stay => {}
        VisualIntroPanelOutcome::ReturnToMenu {
            subflow,
            result,
            message,
        } => {
            let reveal_backing = render_intro_frame(intro);
            intro.panel = VisualIntroPanel::Menu;
            intro.dispatch.complete_subflow(subflow, result);
            intro.start_menu_reveal =
                Some(RectColumnSweepTransition::new(INTRO_START_MENU_REVEAL_RECT));
            intro.start_menu_reveal_backing = Some(reveal_backing);
            intro.modal_backing = None;
            intro.menu_idle_ticks = 0;
            intro.message_waiting_for_key = false;
            intro.message = message;
        }
        VisualIntroPanelOutcome::CommitChargen(result) => {
            match commit_chargen_save(
                &intro.game_dir,
                &result.entered_name,
                result.male,
                result.tournament.stats,
            ) {
                Ok(avatar) => {
                    intro.panel = VisualIntroPanel::Menu;
                    intro.dispatch.complete_subflow(
                        IntroSubflow::CharacterCreation,
                        IntroSubflowResult::SaveReady,
                    );
                    intro.menu_idle_ticks = 0;
                    intro.message_waiting_for_key = false;
                    intro.message = format!(
                        "Created {}. Choose Journey Onward to load the new save.",
                        display_name_bytes(&avatar.name)
                    );
                }
                Err(err) => {
                    intro.panel = VisualIntroPanel::Menu;
                    intro.dispatch.complete_subflow(
                        IntroSubflow::CharacterCreation,
                        IntroSubflowResult::Cancelled,
                    );
                    intro.menu_idle_ticks = 0;
                    intro.message_waiting_for_key = false;
                    intro.message = format!("Character creation failed: {err}");
                }
            }
        }
        VisualIntroPanelOutcome::CommitU4Transfer { source, overrides } => {
            match commit_u4_transfer_save(&intro.game_dir, &source, Some(&overrides)) {
                Ok(avatar) => {
                    intro.panel = VisualIntroPanel::Menu;
                    intro.dispatch.complete_subflow(
                        IntroSubflow::UltimaIvTransfer,
                        IntroSubflowResult::SaveReady,
                    );
                    intro.menu_idle_ticks = 0;
                    intro.message_waiting_for_key = false;
                    intro.message = format!(
                        "Transferred {}. Choose Journey Onward to load the new save.",
                        display_name_bytes(&avatar.name)
                    );
                }
                Err(err) => {
                    intro.panel = VisualIntroPanel::Menu;
                    intro.dispatch.complete_subflow(
                        IntroSubflow::UltimaIvTransfer,
                        IntroSubflowResult::Cancelled,
                    );
                    intro.menu_idle_ticks = 0;
                    intro.message_waiting_for_key = false;
                    intro.message = format!("Transfer failed: {err}");
                }
            }
        }
    }
    true
}

fn cancel_visual_intro_panel(intro: &mut VisualIntroState) -> bool {
    let Some((subflow, result, message)) = (match intro.panel {
        VisualIntroPanel::Menu => None,
        VisualIntroPanel::CharacterCreation { .. } => Some((
            IntroSubflow::CharacterCreation,
            IntroSubflowResult::Cancelled,
            "Character creation cancelled; returning to the intro menu.",
        )),
        VisualIntroPanel::U4Transfer { .. } => Some((
            IntroSubflow::UltimaIvTransfer,
            IntroSubflowResult::Cancelled,
            "Transfer cancelled; returning to the intro menu.",
        )),
        VisualIntroPanel::Story { .. } => Some((
            IntroSubflow::StorySlides,
            IntroSubflowResult::ReturnedToMenu,
            "Ultima V Introduction cancelled; returning to the intro menu.",
        )),
        VisualIntroPanel::Acknowledgements => Some((
            IntroSubflow::Acknowledgements,
            IntroSubflowResult::ReturnedToMenu,
            "Acknowledgements cancelled; returning to the intro menu.",
        )),
        VisualIntroPanel::ReturnToView { .. } => None,
    }) else {
        return false;
    };

    let reveal_backing = render_intro_frame(intro);
    intro.panel = VisualIntroPanel::Menu;
    intro.dispatch.complete_subflow(subflow, result);
    intro.start_menu_reveal = Some(RectColumnSweepTransition::new(INTRO_START_MENU_REVEAL_RECT));
    intro.start_menu_reveal_backing = Some(reveal_backing);
    intro.modal_backing = None;
    intro.menu_idle_ticks = 0;
    intro.message_waiting_for_key = false;
    intro.message = message.to_string();
    true
}

fn step_visual_chargen_panel(
    session: &mut ChargenSession,
    input_line: &mut String,
    ch: char,
) -> VisualIntroPanelOutcome {
    match session.current_step() {
        ChargenSessionStep::PromptName => match ch {
            '\r' | '\n' => {
                let submitted = std::mem::take(input_line);
                match session.submit_name(&submitted) {
                    ChargenSessionStep::Aborted => VisualIntroPanelOutcome::ReturnToMenu {
                        subflow: IntroSubflow::CharacterCreation,
                        result: IntroSubflowResult::Cancelled,
                        message: "Character creation aborted; returning to the intro menu."
                            .to_string(),
                    },
                    _ => VisualIntroPanelOutcome::Stay,
                }
            }
            '\u{8}' => {
                input_line.pop();
                VisualIntroPanelOutcome::Stay
            }
            _ if ch.is_ascii_graphic() || ch == ' ' => {
                if input_line.len() < u5_runtime::CHARGEN_NAME_INPUT_LIMIT {
                    input_line.push(ch);
                }
                VisualIntroPanelOutcome::Stay
            }
            _ => VisualIntroPanelOutcome::Stay,
        },
        ChargenSessionStep::PromptGender => {
            session.submit_gender_key(ch as u8);
            VisualIntroPanelOutcome::Stay
        }
        ChargenSessionStep::PresentIntro { .. } => match session.advance_intro() {
            ChargenSessionStep::Completed(result) => VisualIntroPanelOutcome::CommitChargen(result),
            _ => VisualIntroPanelOutcome::Stay,
        },
        ChargenSessionStep::PresentQuestion(_) => {
            session.submit_answer_key(ch as u8);
            match session.current_step() {
                ChargenSessionStep::Completed(result) => {
                    VisualIntroPanelOutcome::CommitChargen(result)
                }
                _ => VisualIntroPanelOutcome::Stay,
            }
        }
        ChargenSessionStep::Completed(result) => VisualIntroPanelOutcome::CommitChargen(result),
        ChargenSessionStep::Aborted => VisualIntroPanelOutcome::ReturnToMenu {
            subflow: IntroSubflow::CharacterCreation,
            result: IntroSubflowResult::Cancelled,
            message: "Character creation aborted; returning to the intro menu.".to_string(),
        },
        ChargenSessionStep::Ignored => VisualIntroPanelOutcome::Stay,
    }
}

fn step_visual_u4_transfer_panel(
    source: &U4TransferSource,
    overrides: &mut U4TransferOverrides,
    stage: &mut VisualU4TransferStage,
    input_line: &mut String,
    ch: char,
) -> VisualIntroPanelOutcome {
    match *stage {
        VisualU4TransferStage::ConfirmName => match yes_no_key(ch) {
            Some(true) => {
                *stage = VisualU4TransferStage::ConfirmGender;
                VisualIntroPanelOutcome::Stay
            }
            Some(false) => {
                input_line.clear();
                *stage = VisualU4TransferStage::ReplacementName;
                VisualIntroPanelOutcome::Stay
            }
            None => VisualIntroPanelOutcome::Stay,
        },
        VisualU4TransferStage::ReplacementName => match ch {
            '\r' | '\n' => {
                if !input_line.trim().is_empty() {
                    overrides.name = Some(input_line.trim().as_bytes().to_vec());
                    input_line.clear();
                    *stage = VisualU4TransferStage::ConfirmGender;
                }
                VisualIntroPanelOutcome::Stay
            }
            '\u{8}' => {
                input_line.pop();
                VisualIntroPanelOutcome::Stay
            }
            _ if ch.is_ascii_graphic() || ch == ' ' => {
                if input_line.len() < u5_runtime::CHARGEN_NAME_INPUT_LIMIT {
                    input_line.push(ch);
                }
                VisualIntroPanelOutcome::Stay
            }
            _ => VisualIntroPanelOutcome::Stay,
        },
        VisualU4TransferStage::ConfirmGender => match yes_no_key(ch) {
            Some(true) => {
                *stage = VisualU4TransferStage::ConfirmCommit;
                VisualIntroPanelOutcome::Stay
            }
            Some(false) => {
                *stage = VisualU4TransferStage::ReplacementGender;
                VisualIntroPanelOutcome::Stay
            }
            None => VisualIntroPanelOutcome::Stay,
        },
        VisualU4TransferStage::ReplacementGender => match ch.to_ascii_uppercase() {
            'M' => {
                overrides.male = Some(true);
                *stage = VisualU4TransferStage::ConfirmCommit;
                VisualIntroPanelOutcome::Stay
            }
            'F' => {
                overrides.male = Some(false);
                *stage = VisualU4TransferStage::ConfirmCommit;
                VisualIntroPanelOutcome::Stay
            }
            _ => VisualIntroPanelOutcome::Stay,
        },
        VisualU4TransferStage::ConfirmCommit => match yes_no_key(ch) {
            Some(true) => VisualIntroPanelOutcome::CommitU4Transfer {
                source: source.clone(),
                overrides: overrides.clone(),
            },
            Some(false) => VisualIntroPanelOutcome::ReturnToMenu {
                subflow: IntroSubflow::UltimaIvTransfer,
                result: IntroSubflowResult::Cancelled,
                message: "Transfer aborted; returning to the intro menu.".to_string(),
            },
            None => VisualIntroPanelOutcome::Stay,
        },
    }
}

fn yes_no_key(ch: char) -> Option<bool> {
    match ch.to_ascii_uppercase() {
        'Y' => Some(true),
        'N' => Some(false),
        _ => None,
    }
}

fn resolve_visual_intro_subflow(intro: &mut VisualIntroState, subflow: IntroSubflow) -> bool {
    intro.menu_idle_ticks = 0;
    intro.message_waiting_for_key = false;
    intro.start_menu_reveal = None;
    intro.start_menu_reveal_backing = None;
    if !matches!(subflow, IntroSubflow::ReturnToView) {
        intro.modal_backing = None;
    }
    match subflow {
        IntroSubflow::JourneyOnward => match load_play_options_from_save(&intro.game_dir) {
            Ok(options) => {
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::SaveReady);
                *intro
                    .launch_result
                    .lock()
                    .expect("visual intro launch lock poisoned") = Some(options);
            }
            Err(err) => {
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                intro.message = visual_intro_load_error_message(&err);
                intro.message_waiting_for_key = true;
            }
        },
        IntroSubflow::CharacterCreation => match load_question_records(&intro.game_dir) {
            Ok(Some(records)) => {
                match ChargenSession::new(records.records, visual_chargen_rng_pool()) {
                    Ok(session) => {
                        intro.panel = VisualIntroPanel::CharacterCreation {
                            session,
                            input_line: String::new(),
                        };
                        intro.message_waiting_for_key = false;
                        intro.message.clear();
                    }
                    Err(err) => {
                        intro.panel = VisualIntroPanel::Menu;
                        intro
                            .dispatch
                            .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                        intro.message_waiting_for_key = false;
                        intro.message = format!("QUESTION.DAT could not start chargen: {err}");
                    }
                }
            }
            Ok(None) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                intro.message_waiting_for_key = false;
                intro.message =
                    "QUESTION.DAT is required for visual character creation.".to_string();
            }
            Err(err) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                intro.message_waiting_for_key = false;
                intro.message = format!("QUESTION.DAT could not be loaded: {err}");
            }
        },
        IntroSubflow::UltimaIvTransfer => {
            match read_u4_transfer_source_from_party_sav(&intro.game_dir) {
                Ok(source) => {
                    let preview = u4_transfer_preview_from_u4_values(
                        display_name_bytes(&source.name),
                        source.class_index,
                        source.strength,
                        source.dexterity,
                        source.intelligence,
                        0,
                    );
                    intro.panel = VisualIntroPanel::U4Transfer {
                        source,
                        preview,
                        overrides: U4TransferOverrides {
                            name: None,
                            male: None,
                        },
                        stage: VisualU4TransferStage::ConfirmName,
                        input_line: String::new(),
                    };
                    intro.message_waiting_for_key = false;
                    intro.message.clear();
                }
                Err(err) => {
                    intro.panel = VisualIntroPanel::Menu;
                    intro
                        .dispatch
                        .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                    intro.message_waiting_for_key = false;
                    intro.message = format!("Transfer source rejected: {err}");
                }
            }
        }
        IntroSubflow::StorySlides => match load_story_records(&intro.game_dir) {
            Ok(Some(records)) => {
                intro.panel = VisualIntroPanel::Story {
                    records,
                    step: 0,
                    transition: None,
                };
                intro.message_waiting_for_key = false;
                intro.message.clear();
            }
            Ok(None) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
                intro.message_waiting_for_key = false;
                intro.message = "STORY.DAT is missing; returning to the intro menu.".to_string();
            }
            Err(err) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
                intro.message_waiting_for_key = false;
                intro.message = format!("STORY.DAT could not be loaded: {err}");
            }
        },
        IntroSubflow::Acknowledgements => {
            intro.panel = VisualIntroPanel::Acknowledgements;
            intro.message_waiting_for_key = false;
            intro.message.clear();
        }
        IntroSubflow::ReturnToView => {
            intro.modal_backing = Some(render_intro_frame(intro));
            let preview = visual_return_to_view_summary(&intro.game_dir, intro.raster_depth);
            intro.panel = VisualIntroPanel::ReturnToView {
                summary: preview.summary,
                preview_frames_rgba: preview.frames_rgba,
                frame_metadata: preview.frame_metadata,
                preview_frame_index: 0,
                preview_width: preview.width,
                preview_height: preview.height,
            };
            intro.message_waiting_for_key = false;
            intro.message.clear();
        }
    }
    true
}

fn drive_visual(
    keyboard: Res<ButtonInput<KeyCode>>,
    visual: Option<ResMut<VisualState>>,
    mut images: ResMut<Assets<Image>>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(mut visual) = visual else {
        return;
    };
    if keyboard.just_pressed(KeyCode::Escape) && should_escape_quit_visual(&visual.state) {
        exit.write(AppExit::Success);
        return;
    }
    let mut handled = false;
    let shift_pressed =
        keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let control_pressed =
        keyboard.pressed(KeyCode::ControlLeft) || keyboard.pressed(KeyCode::ControlRight);
    for key in keyboard.get_just_pressed() {
        if visual_line_prompt_active(&visual.state) {
            let game_dir = visual.game_dir.clone();
            let v: &mut VisualState = visual.as_mut();
            let result = handle_visual_line_key(
                &mut v.state,
                &mut v.input_line,
                *key,
                shift_pressed,
                control_pressed,
                &game_dir,
            );
            match result {
                Ok(Some(PlayInputDisposition::Quit)) => {
                    exit.write(AppExit::Success);
                    return;
                }
                Ok(Some(PlayInputDisposition::Continue)) => {
                    handled = true;
                    continue;
                }
                Ok(None) => continue,
                Err(err) => {
                    visual.state.message = format!("Input error: {err}");
                    handled = true;
                    continue;
                }
            }
        }
        let Some(ch) = key_code_to_char(*key, shift_pressed, control_pressed) else {
            continue;
        };
        let game_dir = visual.game_dir.clone();
        match handle_play_key_input(&mut visual.state, ch, "", &game_dir) {
            Ok(PlayInputDisposition::Quit) => {
                exit.write(AppExit::Success);
                return;
            }
            Ok(PlayInputDisposition::Continue) => handled = true,
            Err(err) => {
                visual.state.message = format!("Input error: {err}");
                handled = true;
            }
        }
    }
    if !handled {
        return;
    }

    let v: &mut VisualState = visual.as_mut();
    v.prompt_cursor_visible = visual_line_prompt_active(&v.state);
    let input_line = v.input_line.clone();
    let rgba = render_visual_play_frame_with_input_and_cursor(
        &mut v.state,
        &v.atlas,
        &v.text_font,
        &input_line,
        "",
        v.prompt_cursor_visible,
    );
    if let Some(image) = images.get_mut(&v.image_handle) {
        image.data = Some(rgba);
    }
}

fn visual_intro_load_error_message(err: &io::Error) -> String {
    disk_io_error_message(DiskIoHandlerPhase::ReadPrompt, SAVED_GAM_FILENAME, err)
}

fn summarize_intro(intro: &mut VisualIntroState) -> String {
    match &intro.panel {
        VisualIntroPanel::Menu => {}
        VisualIntroPanel::CharacterCreation {
            session,
            input_line,
        } => {
            return summarize_visual_chargen(session, input_line);
        }
        VisualIntroPanel::U4Transfer {
            source,
            preview,
            overrides,
            stage,
            input_line,
        } => {
            return summarize_visual_u4_transfer(source, preview, overrides, *stage, input_line);
        }
        VisualIntroPanel::Story { records, step, .. } => {
            return summarize_intro_story(records, *step);
        }
        VisualIntroPanel::Acknowledgements => {
            return u5_runtime::ACKNOWLEDGEMENTS_LINES
                .iter()
                .map(|line| (*line).to_string())
                .collect::<Vec<_>>()
                .join("\n");
        }
        VisualIntroPanel::ReturnToView {
            summary,
            preview_frames_rgba,
            frame_metadata,
            preview_frame_index,
            ..
        } => {
            let frame_line = if preview_frames_rgba.is_empty() {
                "No rendered playback frames are available.".to_string()
            } else {
                format!(
                    "Playback frame {} of {}.",
                    preview_frame_index.saturating_add(1),
                    preview_frames_rgba.len()
                )
            };
            let frame_detail = frame_metadata
                .get(*preview_frame_index)
                .map(|meta| {
                    let caption = meta.caption.unwrap_or("No active map-strip caption");
                    format!(
                        "{}; command {}; title tick {}; caption: {}.",
                        visual_return_to_view_frame_kind_label(meta.kind),
                        meta.command_index,
                        meta.elapsed_title_ticks,
                        caption
                    )
                })
                .unwrap_or_else(|| "No playback metadata for this frame.".to_string());
            return [
                "Return to View".to_string(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                String::new(),
                summary.clone(),
                String::new(),
                frame_line,
                frame_detail,
                "Press any key after playback completes.".to_string(),
            ]
            .join("\n");
        }
    }

    if matches!(intro.dispatch.tick_title(), UnifiedMenuStep::PresentTitle) {
        return "Ultima V\n\nPress any key for the main menu\nPress J to journey onward"
            .to_string();
    }

    let mut lines = vec![
        "Ultima V".to_string(),
        String::new(),
        "J  Journey Onward".to_string(),
        "C  Create New Character".to_string(),
        "T  Transfer from Ultima IV".to_string(),
        "U  Ultima V Introduction".to_string(),
        "A  Acknowledgements".to_string(),
        "R  Return to View".to_string(),
        String::new(),
        "Esc quits visual intro".to_string(),
    ];
    if !intro.message.is_empty() {
        lines.push(String::new());
        lines.push(intro.message.clone());
    }
    lines.join("\n")
}

fn summarize_visual_chargen(session: &ChargenSession, input_line: &str) -> String {
    match session.current_step() {
        ChargenSessionStep::PromptName => [
            "Create New Character".to_string(),
            String::new(),
            "By what name shalt thou be known?".to_string(),
            format!("> {input_line}"),
            "Esc cancels to the intro menu.".to_string(),
        ]
        .join("\n"),
        ChargenSessionStep::PromptGender => [
            "Create New Character".to_string(),
            String::new(),
            "Art thou Male or Female?".to_string(),
            "Press M or F.".to_string(),
        ]
        .join("\n"),
        ChargenSessionStep::PresentIntro { text, .. } => [
            "Create New Character".to_string(),
            String::new(),
            text,
            String::new(),
            "Press any key to continue.".to_string(),
        ]
        .join("\n"),
        ChargenSessionStep::PresentQuestion(question) => [
            "Create New Character".to_string(),
            format!(
                "Question {} of {} (round {})",
                question.question_index + 1,
                u5_runtime::CHARGEN_QUESTION_COUNT,
                question.round
            ),
            String::new(),
            question.text,
            String::new(),
            format!(
                "A: {}    B: {}",
                question.option_a.name(),
                question.option_b.name()
            ),
            "Choose A or B.".to_string(),
        ]
        .join("\n"),
        ChargenSessionStep::Completed(result) => [
            "Create New Character".to_string(),
            String::new(),
            format!("Writing save for {}.", display_name_bytes(&result.name)),
        ]
        .join("\n"),
        ChargenSessionStep::Aborted => "Character creation aborted.".to_string(),
        ChargenSessionStep::Ignored => "Character creation is waiting.".to_string(),
    }
}

fn summarize_visual_u4_transfer(
    source: &U4TransferSource,
    preview: &U4TransferPreview,
    overrides: &U4TransferOverrides,
    stage: VisualU4TransferStage,
    input_line: &str,
) -> String {
    let selected_name = overrides
        .name
        .as_deref()
        .map(display_name_bytes)
        .unwrap_or_else(|| preview.name.clone());
    let selected_gender = overrides.male.unwrap_or(source.male);
    let mut lines = vec![
        "Transfer from Ultima IV".to_string(),
        String::new(),
        format!(
            "Preview: {} class {}, {}, STR {}, DEX {}, INT {}, XP {}.",
            selected_name,
            preview.class_index,
            if selected_gender { "male" } else { "female" },
            preview.strength,
            preview.dexterity,
            preview.intelligence,
            source.experience / 10
        ),
        String::new(),
    ];
    match stage {
        VisualU4TransferStage::ConfirmName => {
            lines.push(format!("Use imported name {}? Press Y or N.", preview.name));
        }
        VisualU4TransferStage::ReplacementName => {
            lines.push("Replacement name:".to_string());
            lines.push(format!("> {input_line}"));
        }
        VisualU4TransferStage::ConfirmGender => {
            lines.push(format!(
                "Use imported gender {}? Press Y or N.",
                if source.male { "M" } else { "F" }
            ));
        }
        VisualU4TransferStage::ReplacementGender => {
            lines.push("Replacement gender: press M or F.".to_string());
        }
        VisualU4TransferStage::ConfirmCommit => {
            lines.push("Commit transfer save? Press Y or N.".to_string());
        }
    }
    lines.push(String::new());
    lines.push("Esc cancels to the intro menu.".to_string());
    lines.join("\n")
}

fn summarize_intro_story(records: &StoryRecords, step: usize) -> String {
    let mut lines = vec![
        "Ultima V Introduction".to_string(),
        format!("Story step {} of {}", step + 1, INTRO_STORY_STEP_COUNT),
    ];
    if let Some(file) = intro_story_art_file_for_step(step) {
        if let Some(placement) = intro_story_art_placement_for_step(step) {
            lines.push(format_story_art_line(file, placement));
        }
    }
    if let Some(strips) = intro_step_transition_strips(step) {
        lines.push(format!(
            "Transition strips: #{}, ({}, {}); #{}, ({}, {}).",
            strips[0].0, strips[0].1, strips[0].2, strips[1].0, strips[1].1, strips[1].2
        ));
    }
    if step == INTRO_INLINE_DOORWAY_STEP {
        lines.push("Inline doorway transition text.".to_string());
    } else {
        let record_index = if step < INTRO_INLINE_DOORWAY_STEP {
            step
        } else {
            step - 1
        };
        if let Some(text) = records.record(record_index) {
            lines.push(String::new());
            lines.push(text.to_string());
        } else {
            lines.push(format!("Missing STORY.DAT record {record_index}."));
        }
    }
    if intro_step_has_story6_secondary_pass(step) {
        if let Some(subimage) = intro_story6_secondary_subimage(step) {
            lines.push(format!("Secondary STORY6.16 subimage {subimage}."));
        }
    }
    lines.push(String::new());
    if intro_story_step_waits_for_input(step) {
        lines.push("Press any key for the next story step.".to_string());
    } else {
        lines.push("Opening transition step; press any key to continue.".to_string());
    }
    lines.join("\n")
}

fn visual_return_to_view_frame_kind_label(kind: ReturnToViewFrameKind) -> &'static str {
    match kind {
        ReturnToViewFrameKind::StripReveal { .. } => "Map strip reveal",
        ReturnToViewFrameKind::PreviewTick => "Preview title tick",
        ReturnToViewFrameKind::CellEffectStep { .. } => "Local cell-effect step",
        ReturnToViewFrameKind::CellEffectFinalTick { .. } => "Local cell-effect final tick",
        ReturnToViewFrameKind::TemporaryActorDraw => "Temporary actor draw",
        ReturnToViewFrameKind::TemporaryActorDrawOverBacking => "Temporary actor backing draw",
        ReturnToViewFrameKind::FixedWipeRectangle { .. } => "Fixed wipe rectangle",
        ReturnToViewFrameKind::FixedWipeActorDraw => "Fixed wipe actor draw",
        ReturnToViewFrameKind::FixedWait { .. } => "Fixed wait tick",
        ReturnToViewFrameKind::FixedWipeTrailingTick { .. } => "Fixed wipe trailing tick",
        ReturnToViewFrameKind::MoveActorTick => "Actor movement tick",
    }
}

fn format_story_art_line(file: &str, placement: IntroStoryArtPlacement) -> String {
    format!(
        "Art {file} subimage {} at ({}, {}).",
        placement.subimage, placement.top_left_x, placement.top_left_y
    )
}

fn render_intro_frame(intro: &mut VisualIntroState) -> Vec<u8> {
    if matches!(intro.panel, VisualIntroPanel::ReturnToView { .. }) {
        return render_return_to_view_intro_frame(intro);
    }
    if matches!(intro.panel, VisualIntroPanel::Story { .. }) {
        return render_story_intro_frame(intro);
    }
    if matches!(intro.panel, VisualIntroPanel::CharacterCreation { .. }) {
        return render_chargen_intro_frame(intro);
    }

    let summary = summarize_intro(intro);
    let menu_panel = visual_intro_title_surface_visible(intro);
    let title_phase =
        menu_panel && matches!(intro.dispatch.tick_title(), UnifiedMenuStep::PresentTitle);
    let mut rgba =
        vec![0; (INTRO_FRAMEBUFFER_WIDTH as usize) * (INTRO_FRAMEBUFFER_HEIGHT as usize) * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
    }
    if menu_panel {
        let title_flourish_step =
            (title_phase && !intro.title_flourish_complete).then_some(intro.title_flourish_step);
        let signature_progress = (title_phase && !intro.title_signature_complete)
            .then_some(intro.title_signature_progress);
        let mut drew_title = false;
        let panel_rgba = if title_phase {
            visual_intro_title_art_rgba(&intro.game_dir, title_flourish_step, signature_progress)
        } else {
            visual_intro_start_menu_rgba(
                &intro.game_dir,
                intro.raster_depth,
                intro.title_tick_visible_frame,
                intro.dispatch.intro.cached_selection,
            )
            .or_else(|| visual_intro_title_art_rgba(&intro.game_dir, None, None))
        };
        if let Some(title_rgba) = panel_rgba {
            blit_rgba(
                &mut rgba,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                INTRO_FRAMEBUFFER_HEIGHT as usize,
                &title_rgba,
                TITLE_SURFACE_WIDTH as usize,
                TITLE_SURFACE_HEIGHT as usize,
                0,
                0,
            );
            drew_title = true;
        }
        if !drew_title {
            rgba = render_text_panel_rgba(
                &summary,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                INTRO_FRAMEBUFFER_HEIGHT as usize,
            )
            .unwrap_or(rgba);
        } else if !title_phase {
            overlay_intro_menu_message_rgba(&mut rgba, &intro.game_dir, &intro.message);
        }
    } else {
        rgba = render_text_panel_rgba(
            &summary,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
        )
        .unwrap_or(rgba);
    }
    if let VisualIntroPanel::Story {
        step, transition, ..
    } = &intro.panel
    {
        for draw in visual_intro_story_art_draws_rgba(
            &intro.game_dir,
            intro.raster_depth,
            *step,
            *transition,
        ) {
            blit_rgba(
                &mut rgba,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                INTRO_FRAMEBUFFER_HEIGHT as usize,
                &draw.rgba,
                draw.width,
                draw.height,
                usize::from(draw.top_left_x),
                usize::from(draw.top_left_y),
            );
        }
    }
    if let VisualIntroPanel::ReturnToView {
        preview_frames_rgba,
        preview_frame_index,
        preview_width,
        preview_height,
        ..
    } = &intro.panel
    {
        if let Some(preview_rgba) = preview_frames_rgba.get(*preview_frame_index) {
            let x = ((INTRO_FRAMEBUFFER_WIDTH as usize).saturating_sub(*preview_width)) / 2;
            blit_rgba(
                &mut rgba,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                INTRO_FRAMEBUFFER_HEIGHT as usize,
                preview_rgba,
                *preview_width,
                *preview_height,
                x,
                RETURN_TO_VIEW_PREVIEW_Y,
            );
        }
    }
    if let Some(reveal) = intro.start_menu_reveal {
        let source_buffer = IntroDisplayBuffer::from_rgba(
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
            &rgba,
        );
        let backing_rgba = intro
            .start_menu_reveal_backing
            .clone()
            .unwrap_or_else(|| visual_intro_final_title_backing_rgba(intro));
        let mut backing_buffer = IntroDisplayBuffer::from_rgba(
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
            &backing_rgba,
        );
        backing_buffer.copy_revealed_columns_from(&source_buffer, reveal);
        rgba = backing_buffer.to_rgba();
    }
    rgba
}

fn visual_intro_final_title_backing_rgba(intro: &VisualIntroState) -> Vec<u8> {
    let mut buffer = IntroDisplayBuffer::new(
        INTRO_FRAMEBUFFER_WIDTH as usize,
        INTRO_FRAMEBUFFER_HEIGHT as usize,
    );
    buffer.clear(0);
    if let Some(title_rgba) = visual_intro_title_art_rgba(&intro.game_dir, None, None) {
        buffer.blit_rgba(
            &title_rgba,
            TITLE_SURFACE_WIDTH as usize,
            TITLE_SURFACE_HEIGHT as usize,
            0,
            0,
        );
    }
    buffer.to_rgba()
}

fn visual_intro_start_menu_rgba(
    game_dir: &Path,
    depth: TileGraphicsDepth,
    title_tick_frame: u8,
    highlighted: Option<IntroSubflow>,
) -> Option<Vec<u8>> {
    let mut buffer = IntroDisplayBuffer::new(
        INTRO_FRAMEBUFFER_WIDTH as usize,
        INTRO_FRAMEBUFFER_HEIGHT as usize,
    );
    buffer.clear(0);
    blit_image_panel_specs_intro_buffer(&mut buffer, game_dir, depth, &STARTSC_PANEL_SPECS)?;
    buffer.clear_rect_inclusive(
        0,
        136,
        INTRO_FRAMEBUFFER_WIDTH as usize - 1,
        INTRO_FRAMEBUFFER_HEIGHT as usize - 1,
        0,
    );
    let font = load_ibm_ch_font(game_dir).ok()?;
    draw_intro_menu_labels_intro_buffer(&mut buffer, &font, highlighted);
    buffer.draw_title_tick(title_tick_frame);
    Some(buffer.to_rgba())
}

#[cfg(test)]
fn draw_intro_menu_labels_rgba(
    rgba: &mut [u8],
    font: &FixedCellFont,
    highlighted: Option<IntroSubflow>,
) {
    for (subflow, col, row, label) in INTRO_MENU_LABELS {
        overlay_fixed_cell_text_rgba(
            rgba,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
            font,
            label,
            col,
            row,
            highlighted == Some(subflow),
        );
    }
}

fn draw_intro_menu_labels_intro_buffer(
    buffer: &mut IntroDisplayBuffer,
    font: &FixedCellFont,
    highlighted: Option<IntroSubflow>,
) {
    for (subflow, col, row, label) in INTRO_MENU_LABELS {
        overlay_fixed_cell_text_intro_buffer(
            buffer,
            font,
            label,
            col,
            row,
            highlighted == Some(subflow),
        );
    }
}

fn render_story_intro_frame(intro: &mut VisualIntroState) -> Vec<u8> {
    let mut rgba =
        vec![0; (INTRO_FRAMEBUFFER_WIDTH as usize) * (INTRO_FRAMEBUFFER_HEIGHT as usize) * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
    }

    let VisualIntroPanel::Story {
        records,
        step,
        transition,
    } = &intro.panel
    else {
        return rgba;
    };
    for draw in
        visual_intro_story_art_draws_rgba(&intro.game_dir, intro.raster_depth, *step, *transition)
    {
        blit_rgba(
            &mut rgba,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
            &draw.rgba,
            draw.width,
            draw.height,
            usize::from(draw.top_left_x),
            usize::from(draw.top_left_y),
        );
    }

    if let Some(text) = visual_intro_story_text(records, *step) {
        if overlay_proportional_text_from_assets_rgba(
            &mut rgba,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
            &intro.game_dir,
            text,
            ProportionalTextPlacement {
                x: INTRO_STORY_TEXT_X,
                y: INTRO_STORY_TEXT_Y,
                width: INTRO_STORY_TEXT_WIDTH,
                line_height: PROPORTIONAL_TEXT_LINE_HEIGHT,
                color: [0xff, 0xff, 0xff, 0xff],
                shadow: true,
            },
        )
        .is_ok()
        {
            return rgba;
        }
    }

    let summary = summarize_intro(intro);
    overlay_nonblack_text_panel_rgba(
        &mut rgba,
        INTRO_FRAMEBUFFER_WIDTH as usize,
        INTRO_FRAMEBUFFER_HEIGHT as usize,
        &summary,
    );
    rgba
}

fn render_chargen_intro_frame(intro: &mut VisualIntroState) -> Vec<u8> {
    let mut rgba =
        vec![0; (INTRO_FRAMEBUFFER_WIDTH as usize) * (INTRO_FRAMEBUFFER_HEIGHT as usize) * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
    }

    let VisualIntroPanel::CharacterCreation {
        session,
        input_line,
    } = &intro.panel
    else {
        return rgba;
    };

    if !render_chargen_intro_graphics(&mut rgba, intro, session, input_line) {
        let summary = summarize_visual_chargen(session, input_line);
        rgba = render_text_panel_rgba(
            &summary,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
        )
        .unwrap_or(rgba);
    }
    rgba
}

fn render_chargen_intro_graphics(
    rgba: &mut [u8],
    intro: &VisualIntroState,
    session: &ChargenSession,
    input_line: &str,
) -> bool {
    let width = INTRO_FRAMEBUFFER_WIDTH as usize;
    let height = INTRO_FRAMEBUFFER_HEIGHT as usize;
    let Ok(font) = load_ibm_ch_font(&intro.game_dir) else {
        return false;
    };
    match session.current_step() {
        ChargenSessionStep::PromptName => {
            overlay_fixed_cell_text_rgba(
                rgba,
                width,
                height,
                &font,
                "By what name shalt thou be known?",
                3,
                17,
                false,
            );
            overlay_fixed_cell_text_rgba(rgba, width, height, &font, input_line, 14, 19, false);
            true
        }
        ChargenSessionStep::PromptGender => {
            overlay_fixed_cell_text_rgba(
                rgba,
                width,
                height,
                &font,
                "Art thou Male or Female?",
                8,
                21,
                false,
            );
            true
        }
        ChargenSessionStep::PresentIntro { record, text } => {
            let panel = if record == 0 {
                CREATE_OPENING_PANEL
            } else {
                CREATE_RESULT_PANEL
            };
            if blit_image_panel_specs_rgba(
                rgba,
                width,
                height,
                &intro.game_dir,
                intro.raster_depth,
                &[panel],
            )
            .is_none()
            {
                return false;
            }
            let placement = if record == 0 {
                ProportionalTextPlacement {
                    x: CHARGEN_PROPORTIONAL_TEXT_X,
                    y: CHARGEN_PROPORTIONAL_TEXT_Y,
                    width: CHARGEN_PROPORTIONAL_TEXT_WIDTH,
                    line_height: PROPORTIONAL_TEXT_LINE_HEIGHT,
                    color: [0xff, 0xff, 0xff, 0xff],
                    shadow: true,
                }
            } else {
                ProportionalTextPlacement {
                    x: CHARGEN_RESULT_TEXT_X,
                    y: CHARGEN_RESULT_TEXT_Y,
                    width: CHARGEN_RESULT_TEXT_WIDTH,
                    line_height: PROPORTIONAL_TEXT_LINE_HEIGHT,
                    color: [0xff, 0xff, 0xff, 0xff],
                    shadow: true,
                }
            };
            overlay_proportional_text_from_assets_rgba(
                rgba,
                width,
                height,
                &intro.game_dir,
                &text,
                placement,
            )
            .is_ok()
        }
        ChargenSessionStep::PresentQuestion(question) => {
            let option_a = create_virtue_panel_spec(question.option_a, 0);
            let option_b = create_virtue_panel_spec(question.option_b, 184);
            if blit_image_panel_specs_rgba(
                rgba,
                width,
                height,
                &intro.game_dir,
                intro.raster_depth,
                &[
                    CREATE_QUESTION_BACKING_LEFT,
                    CREATE_QUESTION_BACKING_RIGHT,
                    option_a,
                    option_b,
                ],
            )
            .is_none()
            {
                return false;
            }
            overlay_fixed_cell_text_rgba(rgba, width, height, &font, "A", 3, 2, false);
            overlay_fixed_cell_text_rgba(rgba, width, height, &font, "B", 26, 2, false);
            overlay_proportional_text_from_assets_rgba(
                rgba,
                width,
                height,
                &intro.game_dir,
                &question.text,
                ProportionalTextPlacement {
                    x: CHARGEN_QUESTION_TEXT_X,
                    y: CHARGEN_QUESTION_TEXT_Y,
                    width: CHARGEN_QUESTION_TEXT_WIDTH,
                    line_height: PROPORTIONAL_TEXT_LINE_HEIGHT,
                    color: [0xff, 0xff, 0xff, 0xff],
                    shadow: true,
                },
            )
            .is_ok()
        }
        ChargenSessionStep::Completed(result) => {
            let _ = blit_image_panel_specs_rgba(
                rgba,
                width,
                height,
                &intro.game_dir,
                intro.raster_depth,
                &[CREATE_RESULT_PANEL],
            );
            overlay_fixed_cell_text_rgba(
                rgba,
                width,
                height,
                &font,
                &format!("Writing save for {}.", display_name_bytes(&result.name)),
                3,
                21,
                false,
            );
            true
        }
        ChargenSessionStep::Aborted | ChargenSessionStep::Ignored => false,
    }
}

fn create_virtue_panel_spec(virtue: ShrineVirtue, x_offset: usize) -> ImagePanelSpec {
    let mut spec = CREATE_VIRTUE_PANEL_SPECS[virtue.index()];
    spec.top_left_x = spec.top_left_x.saturating_add(x_offset);
    spec
}

fn visual_intro_story_text(records: &StoryRecords, step: usize) -> Option<&str> {
    if step == INTRO_INLINE_DOORWAY_STEP {
        return None;
    }
    let record_index = if step < INTRO_INLINE_DOORWAY_STEP {
        step
    } else {
        step - 1
    };
    records.record(record_index)
}

fn render_return_to_view_intro_frame(intro: &VisualIntroState) -> Vec<u8> {
    let mut rgba = intro.modal_backing.clone().unwrap_or_else(|| {
        render_text_panel_rgba(
            "Return to View",
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
        )
        .unwrap_or_else(|_| visual_intro_final_title_backing_rgba(intro))
    });
    draw_title_tick_overlay_rgba(
        &mut rgba,
        INTRO_FRAMEBUFFER_WIDTH as usize,
        INTRO_FRAMEBUFFER_HEIGHT as usize,
        intro.title_tick_visible_frame,
    );

    let VisualIntroPanel::ReturnToView {
        preview_frames_rgba,
        frame_metadata,
        preview_frame_index,
        preview_width,
        preview_height,
        ..
    } = &intro.panel
    else {
        return rgba;
    };

    let current_meta = frame_metadata.get(*preview_frame_index);
    let caption = current_meta
        .and_then(|meta| meta.caption)
        .unwrap_or("Return to View");
    overlay_centered_text_band_rgba(
        &mut rgba,
        INTRO_FRAMEBUFFER_WIDTH as usize,
        INTRO_FRAMEBUFFER_HEIGHT as usize,
        caption,
        RETURN_TO_VIEW_CAPTION_Y,
        RETURN_TO_VIEW_CAPTION_HEIGHT,
    );

    if let Some(preview_rgba) = preview_frames_rgba.get(*preview_frame_index) {
        let x = ((INTRO_FRAMEBUFFER_WIDTH as usize).saturating_sub(*preview_width)) / 2;
        blit_rgba(
            &mut rgba,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
            preview_rgba,
            *preview_width,
            *preview_height,
            x,
            RETURN_TO_VIEW_PREVIEW_Y,
        );
    }
    if let Some(ReturnToViewFrameKind::FixedWipeRectangle { step }) =
        current_meta.map(|meta| meta.kind)
    {
        if let Some(rects) = return_to_view_fixed_wipe_rectangles(step) {
            let [((x0, y0), (x1, y1)), ((x2, y2), (x3, y3))] = rects;
            fill_rgba_rect_inclusive(
                &mut rgba,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                INTRO_FRAMEBUFFER_HEIGHT as usize,
                usize::from(x0),
                usize::from(y0),
                usize::from(x1),
                usize::from(y1),
                RETURN_TO_VIEW_FIXED_WIPE_RGBA,
            );
            fill_rgba_rect_inclusive(
                &mut rgba,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                INTRO_FRAMEBUFFER_HEIGHT as usize,
                usize::from(x2),
                usize::from(y2),
                usize::from(x3),
                usize::from(y3),
                RETURN_TO_VIEW_FIXED_WIPE_RGBA,
            );
        }
    }
    rgba
}

fn visual_intro_title_surface_visible(intro: &VisualIntroState) -> bool {
    matches!(intro.panel, VisualIntroPanel::Menu)
}

fn visual_intro_start_menu_reveal_active(intro: &VisualIntroState) -> bool {
    matches!(intro.panel, VisualIntroPanel::Menu) && intro.start_menu_reveal.is_some()
}

fn visual_intro_title_art_rgba(
    game_dir: &Path,
    flourish_step: Option<usize>,
    signature_progress: Option<usize>,
) -> Option<Vec<u8>> {
    let title = load_title_bit(game_dir).ok()?;
    let british = load_british_bit(game_dir).ok()?;
    let mut rgba = compose_intro_title_art_rgba(
        &title,
        &british,
        if let Some(step) = flourish_step {
            IntroTitleCompositionPhase::Flourish { step }
        } else {
            IntroTitleCompositionPhase::Signature {
                completed_signature: signature_progress.is_none(),
            }
        },
    );
    if let Some(progress) = signature_progress.filter(|progress| *progress > 0) {
        let signature = load_british_pth(game_dir).ok()?;
        draw_british_signature_rgba(
            &mut rgba,
            TITLE_SURFACE_WIDTH as usize,
            TITLE_SURFACE_HEIGHT as usize,
            &signature,
            progress,
        );
    }
    Some(rgba)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IntroTitleCompositionPhase {
    Flourish { step: usize },
    Signature { completed_signature: bool },
}

const TITLE_FLOURISH_ROW_REVEAL_GROUPS: &[&[&[u8]]] = &[
    &[&[], &[], &[], &[1], &[], &[], &[], &[], &[0, 2], &[]],
    &[
        &[],
        &[],
        &[1, 5],
        &[],
        &[],
        &[2, 4],
        &[],
        &[],
        &[3],
        &[],
        &[0, 6],
        &[],
    ],
    &[
        &[],
        &[],
        &[2, 8],
        &[3, 7],
        &[1, 9],
        &[4, 6],
        &[5],
        &[0, 10],
        &[],
    ],
    &[
        &[],
        &[4, 15],
        &[1, 7, 12, 18],
        &[5, 14],
        &[2, 8, 11, 17],
        &[3, 6, 13, 16],
        &[9, 10],
        &[0, 19],
        &[],
    ],
    &[
        &[],
        &[7, 24],
        &[2, 12, 19, 29],
        &[3, 8, 13, 18, 23, 28],
        &[1, 6, 11, 20, 25, 30],
        &[4, 9, 14, 17, 22, 27],
        &[5, 10, 15, 16, 21, 26],
        &[0, 31],
        &[],
    ],
    &[
        &[],
        &[4, 11, 18, 26, 33, 40],
        &[1, 8, 15, 19, 36, 43],
        &[6, 13, 20, 24, 31, 38],
        &[3, 10, 17, 22, 27, 34, 41],
        &[2, 5, 9, 12, 16, 19, 25, 28, 32, 35, 39, 42],
        &[7, 14, 21, 23, 30, 37],
        &[0, 44],
        &[],
    ],
    &[
        &[],
        &[28, 23, 18, 13, 8, 3, 32, 37, 42, 47, 52, 57],
        &[26, 21, 16, 11, 6, 1, 34, 39, 44, 49, 54, 59],
        &[29, 24, 19, 14, 9, 4, 31, 36, 41, 46, 51, 56],
        &[27, 22, 17, 12, 7, 2, 33, 38, 43, 48, 53, 58],
        &[25, 15, 5, 35, 45, 55],
        &[30, 40, 50, 20, 10],
        &[0, 60],
        &[],
    ],
];

fn intro_title_flourish_total_steps() -> usize {
    TITLE_FLOURISH_ROW_REVEAL_GROUPS
        .iter()
        .map(|groups| groups.len())
        .sum()
}

fn intro_title_flourish_frame_for_step(step: usize) -> Option<(usize, usize)> {
    let mut remaining = step;
    for (slot, groups) in TITLE_FLOURISH_ROW_REVEAL_GROUPS.iter().enumerate() {
        if remaining < groups.len() {
            return Some((slot, remaining));
        }
        remaining -= groups.len();
    }
    TITLE_FLOURISH_ROW_REVEAL_GROUPS
        .last()
        .and_then(|groups| groups.len().checked_sub(1))
        .map(|last_group| (TITLE_FLOURISH_ROW_REVEAL_GROUPS.len() - 1, last_group))
}

fn compose_intro_title_art_rgba(
    title: &TitleBitImages,
    british: &MonochromeBitmap,
    phase: IntroTitleCompositionPhase,
) -> Vec<u8> {
    let width = TITLE_SURFACE_WIDTH as usize;
    let height = TITLE_SURFACE_HEIGHT as usize;
    let mut rgba = vec![0; width * height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
    }

    if let IntroTitleCompositionPhase::Flourish { step } = phase {
        blit_intro_title_flourish_step_rgba(&mut rgba, width, height, title, step);
        return rgba;
    }

    if let Some(placement) = TITLE_BIT_INITIAL_PLACEMENTS
        .iter()
        .copied()
        .find(|placement| placement.slot == 6)
    {
        blit_intro_title_placement_rgba_with_rgb(
            &mut rgba,
            width,
            height,
            title,
            british,
            placement,
            EGA_PALETTE_RGB[9],
        );
    }
    clear_rgba_band(&mut rgba, width, height, TITLE_LOWER_BAND_CLEAR_Y as usize);
    for placement in TITLE_BIT_REMAINING_PLACEMENTS
        .iter()
        .copied()
        .filter(|placement| {
            matches!(placement.asset, TitleBitAsset::Title) && matches!(placement.slot, 7 | 8)
        })
    {
        blit_intro_title_placement_rgba(&mut rgba, width, height, title, british, placement);
    }
    if matches!(
        phase,
        IntroTitleCompositionPhase::Signature {
            completed_signature: true
        }
    ) {
        for placement in TITLE_BIT_REMAINING_PLACEMENTS
            .iter()
            .copied()
            .filter(|placement| {
                matches!(placement.asset, TitleBitAsset::British)
                    || (matches!(placement.asset, TitleBitAsset::Title) && placement.slot == 9)
            })
        {
            blit_intro_title_placement_rgba(&mut rgba, width, height, title, british, placement);
        }
    }

    rgba
}

fn clear_rgba_band(rgba: &mut [u8], width: usize, height: usize, start_y: usize) {
    if start_y >= height {
        return;
    }
    for y in start_y..height {
        for x in 0..width {
            let offset = (y * width + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
        }
    }
}

fn blit_intro_title_flourish_step_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    title: &TitleBitImages,
    step: usize,
) {
    let Some((slot, group_index)) = intro_title_flourish_frame_for_step(step) else {
        return;
    };
    let Some(placement) = TITLE_BIT_INITIAL_PLACEMENTS
        .iter()
        .copied()
        .find(|placement| usize::from(placement.slot) == slot)
    else {
        return;
    };
    let Some(groups) = TITLE_FLOURISH_ROW_REVEAL_GROUPS.get(slot) else {
        return;
    };
    let Some(src) = title.blocks.get(slot) else {
        return;
    };
    let draw_height = usize::from(placement.height).min(src.height);
    let draw_width = usize::from(placement.width).min(src.width);
    let rgb = EGA_PALETTE_RGB[9];
    for group in groups.iter().take(group_index + 1) {
        for row in group.iter().copied().map(usize::from) {
            if row >= draw_height {
                continue;
            }
            blit_intro_title_row_rgba(
                dst,
                dst_width,
                dst_height,
                src,
                usize::from(placement.top_left_x),
                usize::from(placement.top_left_y),
                draw_width,
                row,
                rgb,
            );
        }
    }
}

fn blit_intro_title_row_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    src: &MonochromeBitmap,
    base_x: usize,
    base_y: usize,
    draw_width: usize,
    source_y: usize,
    foreground_rgb: [u8; 3],
) {
    let target_y = base_y + source_y;
    if target_y >= dst_height {
        return;
    }
    for x in 0..draw_width {
        let target_x = base_x + x;
        if target_x >= dst_width {
            break;
        }
        let source_pixel = src.pixels[source_y * src.width + x];
        let rgb = if source_pixel == 0 {
            [0x00, 0x00, 0x00]
        } else {
            foreground_rgb
        };
        let offset = (target_y * dst_width + target_x) * 4;
        dst[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
    }
}

fn blit_intro_title_placement_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    title: &TitleBitImages,
    british: &MonochromeBitmap,
    placement: TitleBitPlacement,
) {
    blit_intro_title_placement_rgba_with_rgb(
        dst,
        dst_width,
        dst_height,
        title,
        british,
        placement,
        EGA_PALETTE_RGB[15],
    );
}

fn blit_intro_title_placement_rgba_with_rgb(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    title: &TitleBitImages,
    british: &MonochromeBitmap,
    placement: TitleBitPlacement,
    foreground_rgb: [u8; 3],
) {
    let src = match placement.asset {
        TitleBitAsset::Title => title.blocks.get(usize::from(placement.slot)),
        TitleBitAsset::British => (placement.slot == 0).then_some(british),
    };
    let Some(src) = src else {
        return;
    };

    let draw_width = usize::from(placement.width).min(src.width);
    let draw_height = usize::from(placement.height).min(src.height);
    let base_x = usize::from(placement.top_left_x);
    let base_y = usize::from(placement.top_left_y);
    for y in 0..draw_height {
        let target_y = base_y + y;
        if target_y >= dst_height {
            break;
        }
        for x in 0..draw_width {
            let target_x = base_x + x;
            if target_x >= dst_width {
                break;
            }
            let source_pixel = src.pixels[y * src.width + x];
            let rgb = if source_pixel == 0 {
                [0x00, 0x00, 0x00]
            } else {
                foreground_rgb
            };
            let offset = (target_y * dst_width + target_x) * 4;
            dst[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
    }
}

fn draw_british_signature_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    signature: &BritishPth,
    max_steps: usize,
) {
    let mut remaining = max_steps;
    for (segment_index, origin) in BRITISH_PTH_PEN_ORIGINS.iter().enumerate() {
        let Some(segment) = signature.segment(segment_index) else {
            continue;
        };
        let mut x = i16::from(origin.0);
        let mut y = i16::from(origin.1);
        for stroke in segment {
            if remaining == 0 {
                return;
            }
            x += i16::from(stroke.dx);
            y += i16::from(stroke.dy);
            if stroke.pen_down {
                paint_signature_pixel_rgba(dst, dst_width, dst_height, x, y);
            }
            remaining -= 1;
        }
    }
}

fn british_signature_step_count(signature: &BritishPth) -> usize {
    signature.segments.iter().map(Vec::len).sum()
}

fn draw_title_tick_overlay_rgba(dst: &mut [u8], dst_width: usize, dst_height: usize, frame: u8) {
    // `cleak/u5-spec#65`: the clean replacement is a deterministic,
    // opaque four-frame overlay in the public title-tick rectangle.
    let start_x = TITLE_TICK_FRAME_X as usize;
    let start_y = TITLE_TICK_FRAME_Y as usize;
    let end_x = start_x
        .saturating_add(TITLE_TICK_FRAME_WIDTH as usize)
        .min(dst_width);
    let end_y = start_y
        .saturating_add(TITLE_TICK_FRAME_HEIGHT as usize)
        .min(dst_height);

    for y in start_y..end_y {
        let local_y = y - start_y;
        for x in start_x..end_x {
            let local_x = x - start_x;
            let offset = (y * dst_width + x) * 4;
            let palette_index =
                title_tick_flame_palette_index(local_x, local_y, frame).unwrap_or(0);
            let rgb = EGA_PALETTE_RGB[usize::from(palette_index)];
            dst[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
    }
}

fn paint_signature_pixel_rgba(dst: &mut [u8], dst_width: usize, dst_height: usize, x: i16, y: i16) {
    let Ok(x) = usize::try_from(x) else {
        return;
    };
    let Ok(y) = usize::try_from(y) else {
        return;
    };
    if x >= dst_width || y >= dst_height {
        return;
    }
    let rgb = EGA_PALETTE_RGB[15];
    let offset = (y * dst_width + x) * 4;
    dst[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct IntroStoryDrawSpec {
    stem: &'static str,
    subimage: u8,
    top_left_x: u16,
    top_left_y: u16,
    clip_width: Option<u16>,
    clip_height: Option<u16>,
}

struct IntroStoryDrawRgba {
    rgba: Vec<u8>,
    width: usize,
    height: usize,
    top_left_x: u16,
    top_left_y: u16,
}

fn visual_intro_story_draw_specs(step: usize) -> Vec<IntroStoryDrawSpec> {
    let mut specs = Vec::new();
    if let Some(strips) = intro_step_transition_strips(step) {
        for (subimage, top_left_x, top_left_y) in strips {
            specs.push(IntroStoryDrawSpec {
                stem: "TEXT",
                subimage,
                top_left_x,
                top_left_y,
                clip_width: None,
                clip_height: None,
            });
        }
    }

    if let Some(file) = intro_story_art_file_for_step(step) {
        if let Some(placement) = intro_story_art_placement_for_step(step) {
            specs.push(IntroStoryDrawSpec {
                stem: intro_story_stem(file),
                subimage: placement.subimage,
                top_left_x: placement.top_left_x,
                top_left_y: placement.top_left_y,
                clip_width: None,
                clip_height: None,
            });
        }
    }

    match step {
        1 => specs.push(IntroStoryDrawSpec {
            stem: "STORY1",
            subimage: INTRO_STEP_1_EXTRA_SUBIMAGE,
            top_left_x: INTRO_STEP_1_EXTRA_ART_X,
            top_left_y: INTRO_STEP_1_EXTRA_ART_Y,
            clip_width: None,
            clip_height: None,
        }),
        INTRO_INLINE_DOORWAY_STEP => specs.push(IntroStoryDrawSpec {
            stem: "STORY2",
            subimage: INTRO_STEP_6_EXTRA_SUBIMAGE,
            top_left_x: INTRO_STEP_6_EXTRA_ART_X,
            top_left_y: INTRO_STEP_6_EXTRA_ART_Y,
            clip_width: None,
            clip_height: None,
        }),
        _ => {
            if intro_step_has_story6_secondary_pass(step) {
                if let Some(primary) = intro_story_art_placement_for_step(step) {
                    if let Some(subimage) = intro_story6_secondary_subimage(step) {
                        specs.push(IntroStoryDrawSpec {
                            stem: "STORY6",
                            subimage,
                            top_left_x: primary.top_left_x,
                            top_left_y: primary
                                .top_left_y
                                .saturating_add(INTRO_STORY6_SECONDARY_Y_DELTA),
                            clip_width: None,
                            clip_height: None,
                        });
                    }
                }
            }
        }
    }

    specs
}

fn visual_intro_story_draw_specs_for_active_panel(
    step: usize,
    transition: Option<RectColumnSweepTransition>,
) -> Vec<IntroStoryDrawSpec> {
    let mut specs = visual_intro_story_draw_specs(step);
    if step != 1 {
        return specs;
    }

    let Some(transition) = transition else {
        specs.retain(|spec| {
            !(spec.stem == "STORY1"
                && spec.subimage == INTRO_STEP_1_EXTRA_SUBIMAGE
                && spec.top_left_x == INTRO_STEP_1_EXTRA_ART_X
                && spec.top_left_y == INTRO_STEP_1_EXTRA_ART_Y)
        });
        return specs;
    };

    if let Some((start_x, end_x)) = transition.revealed_columns() {
        let (_rect_x0, rect_y0, _rect_x1, rect_y1) = transition.rect;
        let clip_width = end_x.saturating_sub(start_x).saturating_add(1);
        let clip_height = rect_y1.saturating_sub(rect_y0).saturating_add(1);
        for spec in &mut specs {
            if spec.stem == "STORY1"
                && spec.subimage == INTRO_STEP_1_EXTRA_SUBIMAGE
                && spec.top_left_x == INTRO_STEP_1_EXTRA_ART_X
                && spec.top_left_y == INTRO_STEP_1_EXTRA_ART_Y
            {
                spec.clip_width = Some(clip_width);
                spec.clip_height = Some(clip_height);
            }
        }
    }
    specs
}

fn visual_intro_story_art_draws_rgba(
    game_dir: &Path,
    depth: TileGraphicsDepth,
    step: usize,
    transition: Option<RectColumnSweepTransition>,
) -> Vec<IntroStoryDrawRgba> {
    visual_intro_story_draw_specs_for_active_panel(step, transition)
        .into_iter()
        .filter_map(|spec| {
            let directory = load_graphic_image_directory(game_dir, spec.stem, depth).ok()?;
            let image = directory.images.get(usize::from(spec.subimage))?.as_ref()?;
            let width = spec
                .clip_width
                .map(usize::from)
                .unwrap_or(image.width)
                .min(image.width);
            let height = spec
                .clip_height
                .map(usize::from)
                .unwrap_or(image.height)
                .min(image.height);
            let rgba = if spec.clip_width.is_some() || spec.clip_height.is_some() {
                graphic_image_to_rgba_clipped(image, depth, width, height)
            } else {
                graphic_image_to_rgba(image, depth)
            };
            Some(IntroStoryDrawRgba {
                rgba,
                width,
                height,
                top_left_x: spec.top_left_x,
                top_left_y: spec.top_left_y,
            })
        })
        .collect()
}

fn intro_story_stem(file: &'static str) -> &'static str {
    match file {
        "STORY1.16" => "STORY1",
        "STORY2.16" => "STORY2",
        "STORY3.16" => "STORY3",
        "STORY4.16" => "STORY4",
        "STORY5.16" => "STORY5",
        "STORY6.16" => "STORY6",
        other => other,
    }
}

fn graphic_image_to_rgba(image: &GraphicImage, depth: TileGraphicsDepth) -> Vec<u8> {
    graphic_image_to_rgba_clipped(image, depth, image.width, image.height)
}

fn graphic_image_to_rgba_clipped(
    image: &GraphicImage,
    depth: TileGraphicsDepth,
    width: usize,
    height: usize,
) -> Vec<u8> {
    let palette: &[[u8; 3]] = match depth {
        TileGraphicsDepth::Ega16 => &EGA_PALETTE_RGB,
        TileGraphicsDepth::Cga4 => &CGA_PALETTE_RGB,
    };
    let limit = palette.len();
    let width = width.min(image.width);
    let height = height.min(image.height);
    let mut rgba = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let row_start = row * image.width;
        for pixel in &image.pixels[row_start..row_start + width] {
            let rgb = palette[usize::from(*pixel) % limit];
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
    }
    rgba
}

fn blit_image_panel_specs_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    game_dir: &Path,
    depth: TileGraphicsDepth,
    specs: &[ImagePanelSpec],
) -> Option<()> {
    for spec in specs {
        let directory = load_graphic_image_directory(game_dir, spec.stem, depth).ok()?;
        let image = directory.images.get(usize::from(spec.subimage))?.as_ref()?;
        let width = spec.width.min(image.width);
        let height = spec.height.min(image.height);
        let rgba = graphic_image_to_rgba_clipped(image, depth, width, height);
        blit_rgba(
            dst,
            dst_width,
            dst_height,
            &rgba,
            width,
            height,
            spec.top_left_x,
            spec.top_left_y,
        );
    }
    Some(())
}

fn blit_image_panel_specs_intro_buffer(
    dst: &mut IntroDisplayBuffer,
    game_dir: &Path,
    depth: TileGraphicsDepth,
    specs: &[ImagePanelSpec],
) -> Option<()> {
    for spec in specs {
        let directory = load_graphic_image_directory(game_dir, spec.stem, depth).ok()?;
        let image = directory.images.get(usize::from(spec.subimage))?.as_ref()?;
        let width = spec.width.min(image.width);
        let height = spec.height.min(image.height);
        let rgba = graphic_image_to_rgba_clipped(image, depth, width, height);
        dst.blit_rgba(&rgba, width, height, spec.top_left_x, spec.top_left_y);
    }
    Some(())
}

fn ega_palette_index_from_rgba(rgba: &[u8]) -> u8 {
    if rgba.len() < 3 {
        return 0;
    }
    let mut best_index = 0u8;
    let mut best_distance = u32::MAX;
    for (index, rgb) in EGA_PALETTE_RGB.iter().enumerate() {
        let dr = i32::from(rgba[0]) - i32::from(rgb[0]);
        let dg = i32::from(rgba[1]) - i32::from(rgb[1]);
        let db = i32::from(rgba[2]) - i32::from(rgb[2]);
        let distance = (dr * dr + dg * dg + db * db) as u32;
        if distance < best_distance {
            best_distance = distance;
            best_index = index as u8;
        }
    }
    best_index
}

fn blit_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    src: &[u8],
    src_width: usize,
    src_height: usize,
    dst_x: usize,
    dst_y: usize,
) {
    for row in 0..src_height {
        let y = dst_y + row;
        if y >= dst_height {
            break;
        }
        let src_row = row * src_width * 4;
        let dst_row = (y * dst_width + dst_x) * 4;
        let cols = src_width.min(dst_width.saturating_sub(dst_x));
        let bytes = cols * 4;
        if let (Some(src_slice), Some(dst_slice)) = (
            src.get(src_row..src_row + bytes),
            dst.get_mut(dst_row..dst_row + bytes),
        ) {
            dst_slice.copy_from_slice(src_slice);
        }
    }
}

fn overlay_fixed_cell_text_intro_buffer(
    dst: &mut IntroDisplayBuffer,
    font: &FixedCellFont,
    text: &str,
    cell_x: usize,
    cell_y: usize,
    inverse: bool,
) {
    for (index, byte) in text.bytes().enumerate() {
        let px = cell_x.saturating_add(index).saturating_mul(CH_CELL_SIDE);
        let py = cell_y.saturating_mul(CH_CELL_SIDE);
        if px >= dst.width || py >= dst.height {
            break;
        }
        let code = byte & 0x7f;
        for glyph_y in 0..CH_CELL_SIDE {
            let target_y = py + glyph_y;
            if target_y >= dst.height {
                break;
            }
            let mut row_bits = font.glyph_row(code, glyph_y).unwrap_or(0);
            if inverse {
                row_bits = !row_bits;
            }
            for glyph_x in 0..CH_CELL_SIDE {
                let target_x = px + glyph_x;
                if target_x >= dst.width {
                    break;
                }
                dst.pixels[target_y * dst.width + target_x] =
                    if row_bits & (1 << (7 - glyph_x)) != 0 {
                        0x0f
                    } else {
                        0x00
                    };
            }
        }
    }
}

fn overlay_fixed_cell_text_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    font: &FixedCellFont,
    text: &str,
    cell_x: usize,
    cell_y: usize,
    inverse: bool,
) {
    let foreground = EGA_PALETTE_RGB[15];
    let background = EGA_PALETTE_RGB[0];
    for (index, byte) in text.bytes().enumerate() {
        let px = cell_x.saturating_add(index).saturating_mul(CH_CELL_SIDE);
        let py = cell_y.saturating_mul(CH_CELL_SIDE);
        if px >= dst_width || py >= dst_height {
            break;
        }
        let code = byte & 0x7f;
        for glyph_y in 0..CH_CELL_SIDE {
            let target_y = py + glyph_y;
            if target_y >= dst_height {
                break;
            }
            let mut row_bits = font.glyph_row(code, glyph_y).unwrap_or(0);
            if inverse {
                row_bits = !row_bits;
            }
            for glyph_x in 0..CH_CELL_SIDE {
                let target_x = px + glyph_x;
                if target_x >= dst_width {
                    break;
                }
                let rgb = if row_bits & (1 << (7 - glyph_x)) != 0 {
                    foreground
                } else {
                    background
                };
                let offset = (target_y * dst_width + target_x) * 4;
                if let Some(pixel) = dst.get_mut(offset..offset + 4) {
                    pixel.copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
                }
            }
        }
    }
}

fn overlay_intro_menu_message_rgba(dst: &mut [u8], game_dir: &Path, message: &str) {
    if message.is_empty() {
        return;
    }
    let Ok(font) = load_ibm_ch_font(game_dir) else {
        overlay_nonblack_text_panel_rgba(
            dst,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
            message,
        );
        return;
    };
    overlay_fixed_cell_text_rgba(
        dst,
        INTRO_FRAMEBUFFER_WIDTH as usize,
        INTRO_FRAMEBUFFER_HEIGHT as usize,
        &font,
        message,
        1,
        24,
        false,
    );
}

fn fill_rgba_rect_inclusive(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    color: [u8; 4],
) {
    if dst_width == 0 || dst_height == 0 {
        return;
    }
    let min_x = x0.min(x1).min(dst_width - 1);
    let max_x = x0.max(x1).min(dst_width - 1);
    let min_y = y0.min(y1).min(dst_height - 1);
    let max_y = y0.max(y1).min(dst_height - 1);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let offset = (y * dst_width + x) * 4;
            if let Some(pixel) = dst.get_mut(offset..offset + 4) {
                pixel.copy_from_slice(&color);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ProportionalTextPlacement {
    x: usize,
    y: usize,
    width: usize,
    line_height: usize,
    color: [u8; 4],
    shadow: bool,
}

fn overlay_proportional_text_from_assets_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    game_dir: &Path,
    text: &str,
    placement: ProportionalTextPlacement,
) -> io::Result<()> {
    let font = load_legacy_proportional_font(game_dir)?;
    overlay_proportional_text_rgba(dst, dst_width, dst_height, &font, text, placement)
}

fn overlay_proportional_text_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    font: &ProportionalFont,
    text: &str,
    placement: ProportionalTextPlacement,
) -> io::Result<()> {
    let widths = visual_proportional_width_table(font);
    let bitmap = rasterize_proportional_paragraph(
        font,
        &widths,
        text.as_bytes(),
        placement.width,
        placement.line_height,
    )?;
    if placement.shadow {
        overlay_monochrome_bitmap_rgba(
            dst,
            dst_width,
            dst_height,
            &bitmap,
            placement.x.saturating_add(1),
            placement.y.saturating_add(1),
            [0x00, 0x00, 0x00, 0xff],
        );
    }
    overlay_monochrome_bitmap_rgba(
        dst,
        dst_width,
        dst_height,
        &bitmap,
        placement.x,
        placement.y,
        placement.color,
    );
    Ok(())
}

fn visual_proportional_width_table(font: &ProportionalFont) -> ProportionalWidthTable {
    let mut widths = ProportionalWidthTable::from_font_advances(font);
    if widths.widths[usize::from(b' ')] == 0 {
        widths.widths[usize::from(b' ')] = 4;
    }
    for byte in b'A'..=b'Z' {
        if widths.widths[usize::from(byte)] == 0 {
            widths.widths[usize::from(byte)] = 6;
        }
    }
    for byte in b'a'..=b'z' {
        if widths.widths[usize::from(byte)] == 0 {
            widths.widths[usize::from(byte)] = 6;
        }
    }
    widths
}

fn overlay_monochrome_bitmap_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    bitmap: &MonochromeBitmap,
    x: usize,
    y: usize,
    color: [u8; 4],
) {
    for row in 0..bitmap.height {
        let target_y = y + row;
        if target_y >= dst_height {
            break;
        }
        for col in 0..bitmap.width {
            if bitmap.pixels[row * bitmap.width + col] == 0 {
                continue;
            }
            let target_x = x + col;
            if target_x >= dst_width {
                break;
            }
            let offset = (target_y * dst_width + target_x) * 4;
            if let Some(pixel) = dst.get_mut(offset..offset + 4) {
                pixel.copy_from_slice(&color);
            }
        }
    }
}

fn overlay_nonblack_text_panel_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    text: &str,
) {
    let Ok(text_rgba) = render_text_panel_rgba(text, dst_width, dst_height) else {
        return;
    };
    let text_pixels: Vec<(usize, [u8; 4])> = text_rgba
        .chunks_exact(4)
        .enumerate()
        .filter_map(|(index, pixel)| {
            (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
                .then_some((index, [pixel[0], pixel[1], pixel[2], pixel[3]]))
        })
        .collect();

    for (index, _) in &text_pixels {
        let x = index % dst_width;
        let y = index / dst_width;
        let shadow_x = x + 1;
        let shadow_y = y + 1;
        if shadow_x < dst_width && shadow_y < dst_height {
            let offset = (shadow_y * dst_width + shadow_x) * 4;
            if let Some(pixel) = dst.get_mut(offset..offset + 4) {
                pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
            }
        }
    }

    for (index, pixel) in text_pixels {
        let offset = index * 4;
        if let Some(dst_pixel) = dst.get_mut(offset..offset + 4) {
            dst_pixel.copy_from_slice(&pixel);
        }
    }
}

fn overlay_centered_text_band_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    text: &str,
    y: usize,
    band_height: usize,
) {
    if y >= dst_height || band_height == 0 {
        return;
    }
    let glyph_advance = 4usize;
    let max_cols = dst_width / glyph_advance;
    let text_cols = text.chars().count().min(max_cols);
    let pad_cols = max_cols.saturating_sub(text_cols) / 2;
    let centered = format!("{}{}", " ".repeat(pad_cols), text);
    let Ok(text_rgba) = render_text_panel_rgba(&centered, dst_width, band_height) else {
        return;
    };

    let band_height = band_height.min(dst_height - y);
    for row in 0..band_height {
        for x in 0..dst_width {
            let src_offset = (row * dst_width + x) * 4;
            let Some(pixel) = text_rgba.get(src_offset..src_offset + 4) else {
                continue;
            };
            if pixel[0] == 0 && pixel[1] == 0 && pixel[2] == 0 {
                continue;
            }
            let dst_offset = ((y + row) * dst_width + x) * 4;
            if let Some(dst_pixel) = dst.get_mut(dst_offset..dst_offset + 4) {
                dst_pixel.copy_from_slice(pixel);
            }
        }
    }
}

fn write_visual_play_report(
    out_dir: &Path,
    label: &str,
    frame_kind: &'static str,
    state: &mut PlayState,
    atlas: &TileAtlas,
    font: &FixedCellFont,
) -> io::Result<VisualFrameReport> {
    let rgba = render_visual_play_frame(state, atlas, font);
    write_visual_report(
        out_dir,
        label,
        VISUAL_PLAY_FRAME_WIDTH,
        VISUAL_PLAY_FRAME_HEIGHT,
        frame_kind,
        rgba,
    )
}

fn write_visual_play_report_with_input(
    out_dir: &Path,
    label: &str,
    frame_kind: &'static str,
    state: &mut PlayState,
    atlas: &TileAtlas,
    font: &FixedCellFont,
    input_line: &str,
    prompt_cursor_visible: bool,
) -> io::Result<VisualFrameReport> {
    let rgba = render_visual_play_frame_with_input_and_cursor(
        state,
        atlas,
        font,
        input_line,
        READY_HINT,
        prompt_cursor_visible,
    );
    write_visual_report(
        out_dir,
        label,
        VISUAL_PLAY_FRAME_WIDTH,
        VISUAL_PLAY_FRAME_HEIGHT,
        frame_kind,
        rgba,
    )
}

fn write_visual_intro_report(
    out_dir: &Path,
    label: &str,
    frame_kind: &'static str,
    panel: VisualIntroPanel,
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
) -> io::Result<VisualFrameReport> {
    write_visual_intro_report_inner(
        out_dir,
        label,
        frame_kind,
        panel,
        game_dir,
        raster_depth,
        false,
    )
}

fn write_visual_intro_report_with_title_dismissed(
    out_dir: &Path,
    label: &str,
    frame_kind: &'static str,
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
) -> io::Result<VisualFrameReport> {
    write_visual_intro_report_inner(
        out_dir,
        label,
        frame_kind,
        VisualIntroPanel::Menu,
        game_dir,
        raster_depth,
        true,
    )
}

fn write_visual_intro_report_inner(
    out_dir: &Path,
    label: &str,
    frame_kind: &'static str,
    panel: VisualIntroPanel,
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    title_dismissed: bool,
) -> io::Result<VisualFrameReport> {
    let static_title = matches!(panel, VisualIntroPanel::Menu) && !title_dismissed;
    let mut intro = VisualIntroState {
        game_dir: game_dir.to_path_buf(),
        raster_depth,
        dispatch: UnifiedMenuDispatch::new(),
        title_flourish_step: if static_title {
            intro_title_flourish_total_steps()
        } else {
            0
        },
        title_flourish_complete: static_title || title_dismissed,
        title_signature_progress: 0,
        title_signature_complete: static_title,
        title_tick_frame: 0,
        title_tick_visible_frame: 0,
        start_menu_reveal: None,
        start_menu_reveal_backing: None,
        modal_backing: None,
        menu_idle_ticks: 0,
        message_waiting_for_key: false,
        message: String::new(),
        panel,
        launch_result: Arc::new(Mutex::new(None)),
        image_handle: None,
    };
    if title_dismissed {
        intro.dispatch.dismiss_title();
    }
    let rgba = render_intro_frame(&mut intro);
    write_visual_report(
        out_dir,
        label,
        INTRO_FRAMEBUFFER_WIDTH,
        INTRO_FRAMEBUFFER_HEIGHT,
        frame_kind,
        rgba,
    )
}

const VISUAL_PLAY_FRAME_WIDTH: u32 = TEXT_WINDOW_RENDER_WIDTH as u32;
const VISUAL_PLAY_FRAME_HEIGHT: u32 = TEXT_WINDOW_RENDER_HEIGHT as u32;
const VISUAL_MAIN_TEXT_TOP: u8 = 22;
const VISUAL_MAIN_TEXT_RIGHT: u8 = 22;
const VISUAL_OVERLAY_SIDE_PANEL_X: usize = STATS_PANEL_TEXT_LEFT as usize * 8;
const VISUAL_OVERLAY_SIDE_PANEL_Y: usize = 0;

fn render_visual_play_frame(
    state: &mut PlayState,
    atlas: &TileAtlas,
    font: &FixedCellFont,
) -> Vec<u8> {
    render_visual_play_frame_with_input(state, atlas, font, "", READY_HINT)
}

fn render_visual_play_frame_with_input(
    state: &mut PlayState,
    atlas: &TileAtlas,
    font: &FixedCellFont,
    input_line: &str,
    fallback: &str,
) -> Vec<u8> {
    render_visual_play_frame_with_input_and_cursor(state, atlas, font, input_line, fallback, false)
}

fn render_visual_play_frame_with_input_and_cursor(
    state: &mut PlayState,
    atlas: &TileAtlas,
    font: &FixedCellFont,
    input_line: &str,
    fallback: &str,
    prompt_cursor_visible: bool,
) -> Vec<u8> {
    if state.endgame.is_some() {
        return render_endgame_framebuffer(state, atlas, input_line, fallback, font);
    }

    let width = VISUAL_PLAY_FRAME_WIDTH as usize;
    let height = VISUAL_PLAY_FRAME_HEIGHT as usize;
    let mut rgba = render_integrated_status_framebuffer(
        state,
        input_line,
        fallback,
        font,
        prompt_cursor_visible,
    );

    let viewport = render_base_framebuffer(state, atlas);
    blit_rgba(
        &mut rgba,
        width,
        height,
        &viewport,
        VIEWPORT_SIZE_PX as usize,
        VIEWPORT_SIZE_PX as usize,
        0,
        0,
    );
    blit_active_view_overlay_rgba(&mut rgba, width, height, state, atlas.depth);
    rgba
}

fn render_endgame_framebuffer(
    state: &mut PlayState,
    atlas: &TileAtlas,
    input_line: &str,
    fallback: &str,
    font: &FixedCellFont,
) -> Vec<u8> {
    let width = VISUAL_PLAY_FRAME_WIDTH as usize;
    let height = VISUAL_PLAY_FRAME_HEIGHT as usize;
    let mut rgba = render_status_framebuffer(state, input_line, fallback, font);
    if endgame_frame_should_show_tableau(state)
        && let Ok(viewport) = render_endgame_tableau_viewport(state, atlas)
    {
        blit_rgba(
            &mut rgba,
            width,
            height,
            &viewport.to_rgba(),
            viewport.width,
            viewport.height,
            0,
            0,
        );
    }
    rgba
}

fn endgame_frame_should_show_tableau(state: &PlayState) -> bool {
    let Some(endgame) = state.endgame.as_ref() else {
        return false;
    };
    if matches!(
        endgame.outcome,
        Some(u5_runtime::EndgameOutcome::MissingBoxOrRefused)
    ) {
        return true;
    }
    if !matches!(endgame.outcome, Some(u5_runtime::EndgameOutcome::Victory)) {
        return true;
    }
    matches!(
        endgame.cinematic.step,
        u5_runtime::endgame_cinematic::EndgameCinematicStep::RiteMessage(_)
            | u5_runtime::endgame_cinematic::EndgameCinematicStep::ThroneTableau
    )
}

fn render_endgame_tableau_viewport(
    state: &PlayState,
    atlas: &TileAtlas,
) -> io::Result<TileViewport> {
    let width = ENDGAME_TABLEAU_WIDTH * TILE_ATLAS_SIDE;
    let height = ENDGAME_TABLEAU_HEIGHT * TILE_ATLAS_SIDE;
    let mut viewport = TileViewport {
        depth: atlas.depth,
        cells_wide: ENDGAME_TABLEAU_WIDTH,
        cells_high: ENDGAME_TABLEAU_HEIGHT,
        width,
        height,
        pixels: vec![0; width * height],
    };

    for y in 0..ENDGAME_TABLEAU_HEIGHT {
        for x in 0..ENDGAME_TABLEAU_WIDTH {
            let tile = state.grid.get(y * TOWN_GRID_SIDE + x).copied().unwrap_or(0);
            blit_tile_id_to_viewport(&mut viewport, atlas, usize::from(tile), x, y)?;
        }
    }

    for (slot, object) in state.active_objects.iter().copied().enumerate().rev() {
        if endgame_tableau_role_for_slot(slot, object).is_none() {
            continue;
        }
        if object.x >= ENDGAME_TABLEAU_WIDTH || object.y >= ENDGAME_TABLEAU_HEIGHT {
            continue;
        }
        let tile = if object.tile == u5_runtime::PLAYER_TILE {
            PLAYER_SPRITE_TILE
        } else {
            usize::from(object.tile)
        };
        blit_tile_id_to_viewport(&mut viewport, atlas, tile, object.x, object.y)?;
    }
    Ok(viewport)
}

fn write_visual_report(
    out_dir: &Path,
    label: &str,
    width: u32,
    height: u32,
    frame_kind: &'static str,
    rgba: Vec<u8>,
) -> io::Result<VisualFrameReport> {
    let byte_hash = hash_bytes(&rgba);
    let nonblack_pixels = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count();
    let path = out_dir.join(format!("{label}.png"));
    write_rgba_png(&path, width, height, rgba)?;
    Ok(VisualFrameReport {
        label: label.to_string(),
        path,
        width,
        height,
        frame_kind,
        byte_hash,
        nonblack_pixels,
    })
}

fn write_visual_frame_suite_manifest(
    out_dir: &Path,
    reports: &[VisualFrameReport],
) -> io::Result<()> {
    let mut manifest = String::new();
    manifest.push_str("# Ultima V Bevy visual frame suite manifest\n");
    manifest.push_str(
        "# Sanitized: contains dimensions, frame kind, hashes, and clean review metadata only.\n",
    );
    manifest.push_str(&format!("coverage\ttotal-frames\t{}\n", reports.len()));
    for coverage in visual_review_coverage_reports(reports) {
        if let Some(expected) = coverage.expected {
            if coverage.actual != expected {
                return Err(io::Error::other(format!(
                    "visual review family `{}` wrote {} frame(s), expected {}",
                    coverage.label, coverage.actual, expected
                )));
            }
            manifest.push_str(&format!(
                "coverage\t{}\t{}/{}\t{}\n",
                coverage.label, coverage.actual, expected, coverage.note
            ));
        } else {
            manifest.push_str(&format!(
                "coverage\t{}\t{}\t{}\n",
                coverage.label, coverage.actual, coverage.note
            ));
        }
    }
    manifest.push_str("# label\tdimensions\tframe-kind\thash\tnonblack\treview-metadata\n");
    for report in reports {
        manifest.push_str(&format!(
            "{}\t{}x{}\t{}\thash {:016x}\tnonblack {}\t{}\n",
            report.label,
            report.width,
            report.height,
            report.frame_kind,
            report.byte_hash,
            report.nonblack_pixels,
            visual_review_metadata(report)
        ));
    }
    std::fs::write(out_dir.join("manifest.txt"), manifest)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VisualReviewCoverage {
    label: &'static str,
    actual: usize,
    expected: Option<usize>,
    note: &'static str,
}

fn visual_review_coverage_reports(reports: &[VisualFrameReport]) -> Vec<VisualReviewCoverage> {
    let mut coverage = Vec::new();
    let outdoor_combat = reports
        .iter()
        .filter(|report| report.label.starts_with("combat-arena-"))
        .count();
    if outdoor_combat > 0 {
        coverage.push(VisualReviewCoverage {
            label: "combat-outdoor-arena-gallery",
            actual: outdoor_combat,
            expected: Some(BRIT_CBT_RECORDS),
            note: "source=BRIT.CBT replacement-gallery",
        });
    }

    let dungeon_combat = reports
        .iter()
        .filter(|report| report.label.starts_with("dungeon-combat-arena-"))
        .count();
    if dungeon_combat > 0 {
        coverage.push(VisualReviewCoverage {
            label: "combat-dungeon-room-gallery",
            actual: dungeon_combat,
            expected: Some(DUNGEON_CBT_RECORDS),
            note: "source=DUNGEON.CBT source-scan-disabled",
        });
    }

    let surface_view_class = reports
        .iter()
        .filter(|report| {
            matches!(
                report.label.as_str(),
                "surface-view-class-gallery"
                    | "peer-view-class-gallery"
                    | "x-ray-view-class-gallery"
            )
        })
        .count();
    if surface_view_class > 0 {
        coverage.push(VisualReviewCoverage {
            label: "surface-view-class-gallery",
            actual: surface_view_class,
            expected: Some(3),
            note: "modes=View,Peer,X-Ray",
        });
    }

    let route_steps = reports
        .iter()
        .filter(|report| report.label.starts_with("route-"))
        .count();
    if route_steps > 0 {
        coverage.push(VisualReviewCoverage {
            label: "visual-route-steps",
            actual: route_steps,
            expected: None,
            note: "per-step Bevy route replay frames",
        });
    }

    let key_route_steps = reports
        .iter()
        .filter(|report| report.label.starts_with("route-key-"))
        .count();
    if key_route_steps > 0 {
        coverage.push(VisualReviewCoverage {
            label: "visual-key-route-steps",
            actual: key_route_steps,
            expected: None,
            note: "real-key Bevy input route frames",
        });
    }

    let combat_route_steps = reports
        .iter()
        .filter(|report| {
            report.label.starts_with("route-combat-")
                || report.label.starts_with("route-doom-combat-")
        })
        .count();
    if combat_route_steps > 0 {
        coverage.push(VisualReviewCoverage {
            label: "visual-route-combat-steps",
            actual: combat_route_steps,
            expected: None,
            note: "combat and Doom combat route frames",
        });
    }

    coverage
}

fn visual_review_metadata(report: &VisualFrameReport) -> String {
    if let Some(index) = parse_visual_index(&report.label, "combat-arena-", 2) {
        let replacement = terrain_combat_raw_replacement_tile_for_arena(index)
            .map(|tile| format!("0x{tile:02x}"))
            .unwrap_or_else(|| "none".to_string());
        return format!(
            "file={}.png review=gallery/combat/outdoor source=BRIT.CBT arena={index:02} replacement_tile={replacement}",
            report.label
        );
    }
    if let Some(index) = parse_visual_index(&report.label, "dungeon-combat-arena-", 3) {
        return format!(
            "file={}.png review=gallery/combat/dungeon-room source=DUNGEON.CBT arena={index:03} source_scan=disabled",
            report.label
        );
    }
    if matches!(
        report.label.as_str(),
        "surface-view-class-gallery" | "peer-view-class-gallery" | "x-ray-view-class-gallery"
    ) {
        return format!(
            "file={}.png review=gallery/surface-view-class mode={}",
            report.label,
            report.label.trim_end_matches("-class-gallery")
        );
    }
    if report.label == "combat-marker-gallery" {
        return format!(
            "file={}.png review=gallery/combat/markers markers=party-corpse,default-drop,vanish,gazer,gargoyle,poison,sleep,fire,energy cursor=slot0 secondary=(3,4)",
            report.label
        );
    }
    if let Some((route, step, input)) = visual_route_step_metadata(&report.label) {
        return format!(
            "file={}.png review=route-step route={route} step={step} input={input}",
            report.label
        );
    }
    format!("file={}.png review=single-frame", report.label)
}

fn parse_visual_index(label: &str, prefix: &str, digits: usize) -> Option<usize> {
    let suffix = label.strip_prefix(prefix)?;
    if suffix.len() != digits || !suffix.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    suffix.parse().ok()
}

fn visual_route_step_metadata(label: &str) -> Option<(&str, &str, &str)> {
    if !label.starts_with("route-") {
        return None;
    }
    let (prefix, input) = label.rsplit_once('-')?;
    let (route, step) = prefix.rsplit_once('-')?;
    if step.len() != 2 || !step.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    Some((route, step, input))
}

fn write_rgba_png(out: &Path, width: u32, height: u32, rgba: Vec<u8>) -> io::Result<()> {
    let image: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, rgba)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "framebuffer size did not match visual frame dimensions",
            )
        })?;
    if let Some(parent) = out.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    image
        .save(out)
        .map_err(|err| io::Error::other(format!("failed to save {}: {err}", out.display())))?;
    Ok(())
}

fn visual_return_to_view_summary(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
) -> VisualReturnToViewPreview {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    match std::fs::metadata(&path) {
        Ok(metadata) => {
            let header = format!(
                "{} found ({} bytes). Return-to-View strips start at byte {}, span {} bytes; command stream starts at byte {} and spans {} bytes.",
                MISCMAPS_DAT_FILE,
                metadata.len(),
                MISCMAPS_RTV_STRIP_SECTION_OFFSET,
                MISCMAPS_RTV_STRIP_SECTION_BYTES,
                MISCMAPS_RTV_COMMAND_SECTION_OFFSET,
                RTV_COMMAND_STREAM_BYTES
            );
            match load_return_to_view_assets(game_dir) {
                Ok(Some(assets)) => {
                    let script_summary = summarize_return_to_view_script(&assets.script);
                    let playback_result = run_return_to_view_playback_until_restart(
                        &assets.strips,
                        &assets.script,
                        4096,
                    );
                    let frames = load_tile_atlas(game_dir, raster_depth).and_then(|atlas| {
                        let playback = playback_result?;
                        let metadata = playback
                            .frames
                            .iter()
                            .map(|frame| VisualReturnToViewFrameMeta {
                                command_index: frame.command_index,
                                elapsed_title_ticks: frame.elapsed_title_ticks,
                                kind: frame.kind,
                                caption: frame.state.current_caption,
                            })
                            .collect::<Vec<_>>();
                        let rendered_frames = playback
                            .frames
                            .iter()
                            .map(|frame| {
                                render_return_to_view_playback_frame_viewport(frame, &atlas, 0).map(
                                    |viewport| {
                                        (viewport.to_rgba(), viewport.width, viewport.height)
                                    },
                                )
                            })
                            .collect::<io::Result<Vec<_>>>()?;
                        Ok((rendered_frames, metadata))
                    });
                    match (
                        summarize_return_to_view_preview(&assets.strips, &assets.script),
                        frames,
                    ) {
                        (Ok(preview_summary), Ok((rendered_frames, frame_metadata))) => {
                            let (width, height) = rendered_frames
                                .first()
                                .map(|(_, width, height)| (*width, *height))
                                .unwrap_or((0, 0));
                            let frames_rgba = rendered_frames
                                .into_iter()
                                .map(|(rgba, _, _)| rgba)
                                .collect::<Vec<_>>();
                            VisualReturnToViewPreview {
                                summary: format!(
                                    "{header} {script_summary} {preview_summary} Rendered {} playback frame(s).",
                                    frames_rgba.len()
                                ),
                                frames_rgba,
                                frame_metadata,
                                width,
                                height,
                            }
                        }
                        (Ok(preview_summary), Err(err)) => VisualReturnToViewPreview {
                            summary: format!(
                                "{header} {script_summary} {preview_summary} Render error: {err}"
                            ),
                            ..Default::default()
                        },
                        (Err(err), _) => VisualReturnToViewPreview {
                            summary: format!("{header} {script_summary} Dry-run error: {err}"),
                            ..Default::default()
                        },
                    }
                }
                Ok(None) => VisualReturnToViewPreview {
                    summary: format!("{MISCMAPS_DAT_FILE} is missing; preview cannot run."),
                    ..Default::default()
                },
                Err(err) => VisualReturnToViewPreview {
                    summary: format!("{header} Script error: {err}"),
                    ..Default::default()
                },
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => VisualReturnToViewPreview {
            summary: format!("{MISCMAPS_DAT_FILE} is missing; preview cannot run."),
            ..Default::default()
        },
        Err(err) => VisualReturnToViewPreview {
            summary: format!("Return-to-View preview error: {err}"),
            ..Default::default()
        },
    }
}

fn visual_chargen_rng_pool() -> Vec<u8> {
    let offset = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as u64)
        .unwrap_or(0)
        .to_le_bytes()[0]
        & 0x07;
    (0u8..128).map(|byte| byte.wrapping_add(offset)).collect()
}

fn display_name_bytes(name: &[u8]) -> String {
    let end = name
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(name.len());
    String::from_utf8_lossy(&name[..end]).trim_end().to_string()
}

#[cfg(test)]
fn render_framebuffer(state: &mut PlayState, atlas: &TileAtlas) -> Vec<u8> {
    match state.render_top_down_frame(VIEWPORT_RADIUS, atlas) {
        Ok(Some(viewport)) => {
            let rgba = viewport.to_rgba();
            if viewport.width as u32 == VIEWPORT_SIZE_PX
                && viewport.height as u32 == VIEWPORT_SIZE_PX
            {
                rgba
            } else {
                center_rgba_on_viewport(rgba, viewport.width, viewport.height)
            }
        }
        _ => render_text_panel_rgba(
            &state.render_text_view(VIEWPORT_RADIUS),
            VIEWPORT_SIZE_PX as usize,
            VIEWPORT_SIZE_PX as usize,
        )
        .unwrap_or_else(|_| vec![0; (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4]),
    }
}

fn render_base_framebuffer(state: &mut PlayState, atlas: &TileAtlas) -> Vec<u8> {
    match state.render_top_down_base_frame(VIEWPORT_RADIUS, atlas) {
        Ok(Some(viewport)) => {
            let rgba = viewport.to_rgba();
            if viewport.width as u32 == VIEWPORT_SIZE_PX
                && viewport.height as u32 == VIEWPORT_SIZE_PX
            {
                rgba
            } else {
                center_rgba_on_viewport(rgba, viewport.width, viewport.height)
            }
        }
        _ => render_text_panel_rgba(
            &state.render_text_view(VIEWPORT_RADIUS),
            VIEWPORT_SIZE_PX as usize,
            VIEWPORT_SIZE_PX as usize,
        )
        .unwrap_or_else(|_| vec![0; (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4]),
    }
}

fn blit_active_view_overlay_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    state: &PlayState,
    depth: TileGraphicsDepth,
) {
    let Some(overlay) = state.render_active_view_overlay(depth) else {
        return;
    };
    let rgba = overlay.to_rgba();
    blit_rgba(
        dst,
        dst_width,
        dst_height,
        &rgba,
        overlay.width,
        overlay.height,
        VISUAL_OVERLAY_SIDE_PANEL_X,
        VISUAL_OVERLAY_SIDE_PANEL_Y,
    );
}

fn center_rgba_on_viewport(src: Vec<u8>, src_width: usize, src_height: usize) -> Vec<u8> {
    let dst_width = VIEWPORT_SIZE_PX as usize;
    let dst_height = VIEWPORT_SIZE_PX as usize;
    let mut dst = vec![0; dst_width * dst_height * 4];
    for pixel in dst.chunks_exact_mut(4) {
        pixel[3] = 0xff;
    }
    let copy_width = src_width.min(dst_width);
    let copy_height = src_height.min(dst_height);
    let src_x = src_width.saturating_sub(copy_width) / 2;
    let src_y = src_height.saturating_sub(copy_height) / 2;
    let dst_x = dst_width.saturating_sub(copy_width) / 2;
    let dst_y = dst_height.saturating_sub(copy_height) / 2;
    for row in 0..copy_height {
        let src_row = ((src_y + row) * src_width + src_x) * 4;
        let dst_row = ((dst_y + row) * dst_width + dst_x) * 4;
        let bytes = copy_width * 4;
        if let (Some(src_slice), Some(dst_slice)) = (
            src.get(src_row..src_row + bytes),
            dst.get_mut(dst_row..dst_row + bytes),
        ) {
            dst_slice.copy_from_slice(src_slice);
        }
    }
    dst
}

#[allow(dead_code)]
fn render_status_framebuffer(
    state: &mut PlayState,
    input_line: &str,
    fallback: &str,
    font: &FixedCellFont,
) -> Vec<u8> {
    let active_cursor = state.active_player;
    let mut display_state = state.clone();
    display_state.refresh_cached_moon_glyphs();
    if display_state.message.is_empty() {
        display_state.message = fallback.to_string();
    } else {
        display_state.message = visual_display_message(&display_state);
    }
    let input_echo = if visual_line_prompt_active(&display_state)
        && !(display_state.active_shop.is_some()
            && input_line.is_empty()
            && !display_state.message.trim().is_empty())
    {
        Some(input_line)
    } else {
        None
    };
    let system = render_play_text_window_system(&display_state, active_cursor, input_echo);
    if stats_panel_active_cursor_visible(state, active_cursor) {
        state.active_player = None;
    }
    let mut rgba = render_text_window_rgba(&system, font)
        .unwrap_or_else(|_| vec![0; TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4]);
    apply_endgame_certificate_rect_operation_mask(&mut rgba, &display_state);
    rgba
}

fn render_integrated_status_framebuffer(
    state: &mut PlayState,
    input_line: &str,
    fallback: &str,
    font: &FixedCellFont,
    prompt_cursor_visible: bool,
) -> Vec<u8> {
    let active_cursor = state.active_player;
    let mut display_state = state.clone();
    display_state.refresh_cached_moon_glyphs();
    if display_state.message.is_empty() {
        display_state.message = fallback.to_string();
    } else {
        display_state.message = visual_display_message(&display_state);
    }
    let input_echo = if visual_line_prompt_active(&display_state)
        && !(display_state.active_shop.is_some()
            && input_line.is_empty()
            && !display_state.message.trim().is_empty())
    {
        Some(input_line)
    } else {
        None
    };
    let mut system = TextWindowSystem::new();
    system.set_window_rect(
        MAIN_TEXT_WINDOW_INDEX,
        0,
        VISUAL_MAIN_TEXT_TOP,
        VISUAL_MAIN_TEXT_RIGHT,
        TEXT_SCREEN_ROWS - 1,
    );
    system.set_window_rect(
        STATS_PANEL_TEXT_WINDOW_INDEX,
        STATS_PANEL_TEXT_LEFT,
        0,
        STATS_PANEL_TEXT_RIGHT,
        STATS_PANEL_TEXT_BOTTOM,
    );
    system.set_window_rect(
        PROMPT_TEXT_WINDOW_INDEX,
        0,
        TEXT_SCREEN_ROWS - 2,
        VISUAL_MAIN_TEXT_RIGHT,
        TEXT_SCREEN_ROWS - 1,
    );
    system.set_active_window(MAIN_TEXT_WINDOW_INDEX);
    let message = display_state
        .active_shop
        .as_ref()
        .map(|shop| shop.modal_text(&display_state.message))
        .unwrap_or_else(|| display_state.message.clone());
    if display_state.active_shop.is_some() {
        configure_talk_shop_text_window(&mut system);
        system.set_window_rect(
            TALK_SHOP_TEXT_WINDOW_INDEX,
            0,
            VISUAL_MAIN_TEXT_TOP,
            VISUAL_MAIN_TEXT_RIGHT,
            TEXT_SCREEN_ROWS - 1,
        );
        paint_talk_shop_text_window(&mut system, &message);
    } else {
        paint_message_text_window(&mut system, &message);
    }
    paint_stats_panel_text_window(&mut system, &display_state, active_cursor);
    if display_state.active_shop.is_some() {
        paint_inn_pickup_register_text_window(&mut system, &display_state);
    }
    if let Some(input_echo) = input_echo {
        let cursor_glyph = prompt_cursor_visible.then_some(PROMPT_CURSOR_GLYPH);
        paint_prompt_text_window_with_cursor(&mut system, input_echo, cursor_glyph);
    }
    if display_state.active_shop.is_some() {
        system.set_active_window(TALK_SHOP_TEXT_WINDOW_INDEX);
    } else {
        system.set_active_window(MAIN_TEXT_WINDOW_INDEX);
    }
    if stats_panel_active_cursor_visible(state, active_cursor) {
        state.active_player = None;
    }
    let mut rgba = render_text_window_rgba(&system, font)
        .unwrap_or_else(|_| vec![0; TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4]);
    apply_endgame_certificate_rect_operation_mask(&mut rgba, &display_state);
    rgba
}

fn visual_display_message(state: &PlayState) -> String {
    match state.area {
        u5_runtime::Area::Town { scene, .. } => town_resident_name(scene.byte)
            .map(|name| state.message.replacen(&scene.key(), name, 1))
            .unwrap_or_else(|| state.message.clone()),
        u5_runtime::Area::Dungeon { scene, .. } => {
            state.message.replacen(&scene.key(), scene.name(), 1)
        }
        u5_runtime::Area::World { .. } => state.message.clone(),
    }
}

fn apply_endgame_certificate_rect_operation_mask(rgba: &mut [u8], state: &PlayState) {
    let Some(rect) = state
        .endgame
        .as_ref()
        .and_then(|endgame| endgame.cinematic.certificate_rect_operation)
    else {
        return;
    };
    let (x0, y0, x1, y1) = rect;
    fill_rgba_rect_inclusive(
        rgba,
        TEXT_WINDOW_RENDER_WIDTH,
        TEXT_WINDOW_RENDER_HEIGHT,
        usize::from(x0),
        usize::from(y0),
        usize::from(x1),
        usize::from(y1),
        [0x00, 0x00, 0x00, 0xff],
    );
}

#[cfg(test)]
fn apply_rect_column_sweep_reveal_rgba(
    destination: &mut [u8],
    source: &[u8],
    width: usize,
    height: usize,
    transition: RectColumnSweepTransition,
) {
    if destination.len() != source.len() {
        return;
    }
    let Some((start_x, end_x)) = transition.revealed_columns() else {
        return;
    };
    let (rect_x0, rect_y0, rect_x1, rect_y1) = transition.rect;
    let y0 = usize::from(rect_y0).min(height);
    let y1 = usize::from(rect_y1).min(height.saturating_sub(1));
    let x0 = usize::from(rect_x0).min(width);
    let x1 = usize::from(rect_x1).min(width.saturating_sub(1));
    let revealed_start = usize::from(start_x).min(width);
    let revealed_end = usize::from(end_x).min(width.saturating_sub(1));

    if x0 > x1 || y0 > y1 {
        return;
    }

    for y in y0..=y1 {
        for x in x0..=x1 {
            if x < revealed_start || x > revealed_end {
                continue;
            }
            let offset = (y * width + x) * 4;
            if let Some(src_pixel) = source.get(offset..offset + 4)
                && let Some(dst_pixel) = destination.get_mut(offset..offset + 4)
            {
                dst_pixel.copy_from_slice(src_pixel);
            }
        }
    }
}

#[cfg(test)]
fn summarize(state: &mut PlayState, fallback: &str, input_line: &str) -> String {
    let dungeon_note = if matches!(state.area, u5_runtime::Area::Dungeon { .. }) {
        " [Dungeon first-person panel]"
    } else {
        ""
    };
    let msg = if state.message.is_empty() {
        fallback.to_string()
    } else {
        state.message.clone()
    };
    let mut summary = format!(
        "{} ({}, {}) facing {} - turn {} - music {}{}\n{}",
        state.current_area_label(),
        state.player.x,
        state.player.y,
        u5_runtime::Direction::name(state.player.facing),
        state.turn,
        if state.music_enabled { "on" } else { "off" },
        dungeon_note,
        msg
    );
    summary.push('\n');
    let input_echo = visual_line_prompt_active(state).then_some(input_line);
    summary.push_str(&state.render_text_window_frame(input_echo));
    summary
}

fn visual_line_prompt_active(state: &PlayState) -> bool {
    state.active_conversation.is_some()
        || state.active_blackthorn.is_some()
        || state.active_shrine.is_some()
        || state.active_yell.is_some()
        || state
            .active_wishing_well
            .as_ref()
            .is_some_and(|session| session.coin_accepted)
        || matches!(
            state.active_shop.as_ref(),
            Some(
                ActiveShopSession::Sage(SageState::Prompt { .. })
                    | ActiveShopSession::Tavern(TavernState::PickProvisionQuantity { .. })
                    | ActiveShopSession::Reagent(ReagentShopState::PickQuantity { .. })
                    | ActiveShopSession::Guild(GuildShopState::PickQuantity { .. })
            )
        )
}

fn visual_modal_prompt_active(state: &PlayState) -> bool {
    visual_line_prompt_active(state)
        || state.active_z_stats.is_some()
        || state.active_ready.is_some()
        || state.active_use.is_some()
        || state.active_cast.is_some()
        || state.active_cast_followup.is_some()
        || state.active_rest.is_some()
        || state.active_jimmy.is_some()
        || state.active_surface_chest.is_some()
        || state.active_mix.is_some()
        || state.active_new_order.is_some()
        || state.active_wishing_well.is_some()
        || state.active_direction_prompt.is_some()
        || state.active_yes_no_prompt.is_some()
        || state.active_shop.is_some()
        || state.pending_moongate.is_some()
        || state.pending_town_arrest.is_some()
        || state.endgame.is_some()
}

fn visual_idle_tick(state: &mut PlayState) -> bool {
    if visual_modal_prompt_active(state) {
        return false;
    }
    state.advance_visual_tick();
    true
}

fn advance_visual_wait_frame(state: &mut PlayState, prompt_cursor_visible: &mut bool) -> bool {
    if visual_line_prompt_active(state) {
        *prompt_cursor_visible = !*prompt_cursor_visible;
        true
    } else if advance_visual_endgame_frame_operation(state) {
        *prompt_cursor_visible = false;
        true
    } else {
        *prompt_cursor_visible = false;
        visual_idle_tick(state)
    }
}

fn advance_visual_endgame_frame_operation(state: &mut PlayState) -> bool {
    state
        .endgame
        .as_mut()
        .is_some_and(|endgame| endgame.advance_cinematic_frame_operation())
}

fn should_escape_quit_visual(state: &PlayState) -> bool {
    !visual_modal_prompt_active(state)
}

fn handle_visual_line_key(
    state: &mut PlayState,
    input_line: &mut String,
    key: KeyCode,
    shift_pressed: bool,
    control_pressed: bool,
    game_dir: &Path,
) -> std::io::Result<Option<PlayInputDisposition>> {
    let Some(byte) = key_code_to_line_input_byte(key, shift_pressed, control_pressed) else {
        return Ok(None);
    };
    match u5_runtime::free_text_input_action(byte) {
        u5_runtime::FreeTextInputAction::Cancel => {
            input_line.clear();
            handle_play_key_input(state, '\u{1b}', "", game_dir).map(Some)
        }
        u5_runtime::FreeTextInputAction::Submit => {
            let submitted = std::mem::take(input_line);
            let mut chars = submitted.chars();
            let (key, suffix) = match chars.next() {
                Some(first) => {
                    let mut suffix = chars.collect::<String>();
                    if state.active_shrine.is_some() {
                        suffix.push('\n');
                    }
                    (first, suffix)
                }
                None => ('\n', String::new()),
            };
            handle_play_key_input(state, key, &suffix, game_dir).map(Some)
        }
        u5_runtime::FreeTextInputAction::Backspace => {
            input_line.pop();
            Ok(Some(PlayInputDisposition::Continue))
        }
        u5_runtime::FreeTextInputAction::Append(byte) => {
            input_line.push(char::from(byte));
            Ok(Some(PlayInputDisposition::Continue))
        }
        u5_runtime::FreeTextInputAction::Discard => Ok(None),
    }
}

fn key_code_to_line_input_byte(
    key: KeyCode,
    shift_pressed: bool,
    control_pressed: bool,
) -> Option<u8> {
    use KeyCode::*;
    if control_pressed {
        return None;
    }
    match key {
        KeyA => return Some(line_letter_for_shift(b'a', shift_pressed)),
        KeyB => return Some(line_letter_for_shift(b'b', shift_pressed)),
        KeyC => return Some(line_letter_for_shift(b'c', shift_pressed)),
        KeyD => return Some(line_letter_for_shift(b'd', shift_pressed)),
        KeyE => return Some(line_letter_for_shift(b'e', shift_pressed)),
        KeyF => return Some(line_letter_for_shift(b'f', shift_pressed)),
        KeyG => return Some(line_letter_for_shift(b'g', shift_pressed)),
        KeyH => return Some(line_letter_for_shift(b'h', shift_pressed)),
        KeyI => return Some(line_letter_for_shift(b'i', shift_pressed)),
        KeyJ => return Some(line_letter_for_shift(b'j', shift_pressed)),
        KeyK => return Some(line_letter_for_shift(b'k', shift_pressed)),
        KeyL => return Some(line_letter_for_shift(b'l', shift_pressed)),
        KeyM => return Some(line_letter_for_shift(b'm', shift_pressed)),
        KeyN => return Some(line_letter_for_shift(b'n', shift_pressed)),
        KeyO => return Some(line_letter_for_shift(b'o', shift_pressed)),
        KeyP => return Some(line_letter_for_shift(b'p', shift_pressed)),
        KeyQ => return Some(line_letter_for_shift(b'q', shift_pressed)),
        KeyR => return Some(line_letter_for_shift(b'r', shift_pressed)),
        KeyS => return Some(line_letter_for_shift(b's', shift_pressed)),
        KeyT => return Some(line_letter_for_shift(b't', shift_pressed)),
        KeyU => return Some(line_letter_for_shift(b'u', shift_pressed)),
        KeyV => return Some(line_letter_for_shift(b'v', shift_pressed)),
        KeyW => return Some(line_letter_for_shift(b'w', shift_pressed)),
        KeyX => return Some(line_letter_for_shift(b'x', shift_pressed)),
        KeyY => return Some(line_letter_for_shift(b'y', shift_pressed)),
        KeyZ => return Some(line_letter_for_shift(b'z', shift_pressed)),
        _ => {}
    };
    key_code_to_input_byte(key, shift_pressed, false)
}

fn line_letter_for_shift(lower: u8, shift_pressed: bool) -> u8 {
    if shift_pressed {
        lower.to_ascii_uppercase()
    } else {
        lower
    }
}

fn key_code_to_char(key: KeyCode, shift_pressed: bool, control_pressed: bool) -> Option<char> {
    key_code_to_input_byte(key, shift_pressed, control_pressed).map(char::from)
}

fn key_code_to_input_byte(key: KeyCode, shift_pressed: bool, control_pressed: bool) -> Option<u8> {
    use KeyCode::*;
    if control_pressed {
        return match key {
            KeyS => Some(PLAY_MUSIC_TOGGLE_KEY as u8),
            _ => None,
        };
    }

    let byte = match key {
        Escape => 0x1B,
        Enter | NumpadEnter => 0x0D,
        Backspace | NumpadBackspace => 0x08,
        ArrowUp => u5_runtime::INPUT_CODE_NORTH,
        ArrowDown => u5_runtime::INPUT_CODE_SOUTH,
        ArrowLeft => u5_runtime::INPUT_CODE_WEST,
        ArrowRight => u5_runtime::INPUT_CODE_EAST,
        Home => u5_runtime::INPUT_CODE_NORTHWEST,
        PageUp => u5_runtime::INPUT_CODE_NORTHEAST,
        End => u5_runtime::INPUT_CODE_SOUTHWEST,
        PageDown => u5_runtime::INPUT_CODE_SOUTHEAST,
        Numpad1 => u5_runtime::INPUT_CODE_SOUTHWEST,
        Numpad2 => u5_runtime::INPUT_CODE_SOUTH,
        Numpad3 => u5_runtime::INPUT_CODE_SOUTHEAST,
        Numpad4 => u5_runtime::INPUT_CODE_WEST,
        Numpad6 => u5_runtime::INPUT_CODE_EAST,
        Numpad7 => u5_runtime::INPUT_CODE_NORTHWEST,
        Numpad8 => u5_runtime::INPUT_CODE_NORTH,
        Numpad9 => u5_runtime::INPUT_CODE_NORTHEAST,
        F1 => input_function_key_code(1)?,
        F2 => input_function_key_code(2)?,
        F3 => input_function_key_code(3)?,
        F4 => input_function_key_code(4)?,
        F5 => input_function_key_code(5)?,
        F6 => input_function_key_code(6)?,
        F7 => input_function_key_code(7)?,
        F8 => input_function_key_code(8)?,
        F9 => input_function_key_code(9)?,
        F10 => input_function_key_code(10)?,
        Digit1 if shift_pressed => input_keypad_digit_direction_code(1)?,
        Digit2 if shift_pressed => input_keypad_digit_direction_code(2)?,
        Digit3 if shift_pressed => input_keypad_digit_direction_code(3)?,
        Digit4 if shift_pressed => input_keypad_digit_direction_code(4)?,
        Digit6 if shift_pressed => input_keypad_digit_direction_code(6)?,
        Digit7 if shift_pressed => input_keypad_digit_direction_code(7)?,
        Digit8 if shift_pressed => input_keypad_digit_direction_code(8)?,
        Digit9 if shift_pressed => input_keypad_digit_direction_code(9)?,
        Digit0 | Numpad0 => b'0',
        Digit1 => b'1',
        Digit2 => b'2',
        Digit3 => b'3',
        Digit4 => b'4',
        Digit5 | Numpad5 => b'5',
        Digit6 => b'6',
        Digit7 => b'7',
        Digit8 => b'8',
        Digit9 => b'9',
        Space => b' ',
        BracketLeft => {
            if shift_pressed {
                b'{'
            } else {
                b'['
            }
        }
        BracketRight => {
            if shift_pressed {
                b'}'
            } else {
                b']'
            }
        }
        Equal | NumpadAdd if shift_pressed => b'+',
        Equal => b'=',
        Minus if shift_pressed => b'_',
        Minus | NumpadSubtract => b'-',
        NumpadAdd => b'+',
        Comma => {
            if shift_pressed {
                b'<'
            } else {
                b','
            }
        }
        Period => {
            if shift_pressed {
                b'>'
            } else {
                b'.'
            }
        }
        KeyA => b'A',
        KeyB => b'B',
        KeyC => b'C',
        KeyD => b'D',
        KeyE => b'E',
        KeyF => b'F',
        KeyG => b'G',
        KeyH => b'H',
        KeyI => b'I',
        KeyJ => b'J',
        KeyK => b'K',
        KeyL => b'L',
        KeyM => b'M',
        KeyN => b'N',
        KeyO => b'O',
        KeyP => b'P',
        KeyQ => b'Q',
        KeyR => b'R',
        KeyS => b'S',
        KeyT => b'T',
        KeyU => b'U',
        KeyV => b'V',
        KeyW => b'W',
        KeyX => b'X',
        KeyY => b'Y',
        KeyZ => b'Z',
        _ => return None,
    };
    Some(input_case_fold(byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use u5_runtime::blackthorn_session::BlackthornChallenge;
    use u5_runtime::conversation_session::ConversationSession;
    use u5_runtime::shop_runtime::{
        ArmsShopState, GuildShopState, ReagentShopState, SageState, TavernState,
    };
    use u5_runtime::shop_session::ActiveShopSession;
    use u5_runtime::test_fixtures::{
        debug_game_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
        saved_game_seed_bytes, synthetic_tile_atlas, test_state, world_state,
    };
    use u5_runtime::tlk_control_codes::TLK_TEXT_XOR_MASK;
    use u5_runtime::{
        Area, ArmsShop, BRIT_OOL_FILENAME, CH_CELL_SIDE, CH_FONT_LEN, COMBAT_ARENA_SIDE,
        DEFAULT_GAME_DIR, Direction, EGA_PALETTE_RGB, GuildShop, Herbalist, IBM_CH_FILE,
        INIT_GAM_FILENAME, INIT_OOL_FILENAME, OOL_PLANE_LEN, PenStroke, ProportionalGlyph,
        REAGENT_COUNT, REAGENT_SPIDER_SILK, SAVE_CHARACTER_DEX_OFFSET,
        SAVE_CHARACTER_GENDER_OFFSET, SAVE_CHARACTER_INT_OFFSET, SAVE_CHARACTER_NAME_LEN,
        SAVE_CHARACTER_STR_OFFSET, SAVE_ROSTER_OFFSET, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME,
        SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST, SHRINE_TABLE_FILE, STORY_DAT_FILE, ShrineVirtue,
        SurfaceChestVerb, TILES_EGA_FILE, Tavern, TileGraphicsDepth,
        U4_TRANSFER_U5_SEED_GAM_FILENAME, U4TransferSource, WorldPlane, dungeon_cell_index,
        parse_ch_font, world_cell_index, wrap_text_panel_lines,
    };

    fn enc_tlk_text(text: &str) -> Vec<u8> {
        text.bytes().map(|b| b ^ TLK_TEXT_XOR_MASK).collect()
    }

    fn temp_output_dir(name: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("u5-bevy-frame-suite-{name}-{nonce}"))
    }

    fn install_test_conversation(state: &mut PlayState) {
        let raw = vec![
            enc_tlk_text("Ada"),
            enc_tlk_text("a quiet smith"),
            enc_tlk_text("Greetings, traveller."),
            enc_tlk_text("I mend gear."),
            enc_tlk_text("Farewell."),
        ];
        let decoded = vec![
            "Ada".to_string(),
            "a quiet smith".to_string(),
            "Greetings, traveller.".to_string(),
            "I mend gear.".to_string(),
            "Farewell.".to_string(),
        ];
        state.active_conversation = Some(Box::new(ConversationSession::new(raw, decoded)));
        state.advance_active_conversation_greeting();
    }

    fn assert_viewport_rgba_frame(rgba: &[u8]) {
        assert_eq!(
            rgba.len(),
            (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4
        );
        assert!(rgba.chunks_exact(4).all(|pixel| pixel[3] == 0xff));
    }

    fn assert_nonblack_rgba(rgba: &[u8]) {
        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        );
    }

    fn rgba_pixel(rgba: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
        let offset = (y * width + x) * 4;
        [
            rgba[offset],
            rgba[offset + 1],
            rgba[offset + 2],
            rgba[offset + 3],
        ]
    }

    fn ega_rgba(index: usize) -> [u8; 4] {
        [
            EGA_PALETTE_RGB[index][0],
            EGA_PALETTE_RGB[index][1],
            EGA_PALETTE_RGB[index][2],
            0xff,
        ]
    }

    fn proportional_test_font() -> ProportionalFont {
        let glyph = |advance_width: u8, lit_width: usize| ProportionalGlyph {
            advance_width,
            bitmap: MonochromeBitmap {
                width: PCS_GLYPH_HEIGHT.min(8),
                height: PCS_GLYPH_HEIGHT,
                pixels: (0..PCS_GLYPH_HEIGHT)
                    .flat_map(|row| {
                        (0..PCS_GLYPH_HEIGHT.min(8))
                            .map(move |col| u8::from(row < 3 && col < lit_width))
                    })
                    .collect(),
            },
        };
        let mut glyphs = vec![glyph(0, 0); usize::from(b'z' - b' ' + 1)];
        for byte in b'A'..=b'Z' {
            glyphs[usize::from(byte - b' ')] = glyph(5, 3);
        }
        for byte in b'a'..=b'z' {
            glyphs[usize::from(byte - b' ')] = glyph(5, 3);
        }
        glyphs[usize::from(b' ' - b' ')] = glyph(0, 0);
        ProportionalFont {
            first_code: b' ',
            glyphs,
        }
    }

    #[test]
    fn proportional_intro_overlay_draws_glyphs_and_shadow() {
        let font = proportional_test_font();
        let mut rgba = vec![0; 40 * 30 * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0x11, 0x11, 0x11, 0xff]);
        }

        overlay_proportional_text_rgba(
            &mut rgba,
            40,
            30,
            &font,
            "AB",
            ProportionalTextPlacement {
                x: 4,
                y: 5,
                width: 24,
                line_height: PROPORTIONAL_TEXT_LINE_HEIGHT,
                color: [0xee, 0xdd, 0xcc, 0xff],
                shadow: true,
            },
        )
        .unwrap();

        assert_eq!(rgba_pixel(&rgba, 40, 4, 5), [0xee, 0xdd, 0xcc, 0xff]);
        assert_eq!(rgba_pixel(&rgba, 40, 5, 6), [0xee, 0xdd, 0xcc, 0xff]);
        assert_eq!(rgba_pixel(&rgba, 40, 7, 8), [0x00, 0x00, 0x00, 0xff]);
    }

    #[test]
    fn world_framebuffer_renders_top_down_rgba() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let rgba = render_framebuffer(&mut state, &atlas);

        assert_viewport_rgba_frame(&rgba);
        assert_nonblack_rgba(&rgba);
    }

    #[test]
    fn town_framebuffer_renders_top_down_rgba() {
        let mut grid = open_grid();
        grid[5 * 32 + 5] = 5;
        let mut state = test_state(grid, 5, 5);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let rgba = render_framebuffer(&mut state, &atlas);

        assert_viewport_rgba_frame(&rgba);
        assert_nonblack_rgba(&rgba);
    }

    #[test]
    fn combat_framebuffer_renders_arena_rgba() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.combat_active = true;
        state.combat_terrain = [[5; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
        state.combat_terrain[0][0] = 12;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let rgba = render_framebuffer(&mut state, &atlas);

        assert_viewport_rgba_frame(&rgba);
        assert_nonblack_rgba(&rgba);
    }

    #[test]
    fn combat_marker_gallery_seed_matches_visual_review_contract() {
        let mut state = world_state(open_world_grid(), 10, 20);

        seed_visual_combat_marker_gallery(&mut state).unwrap();

        validate_visual_combat_marker_gallery_state(&state).unwrap();
        assert_eq!(COMBAT_MARKER_GALLERY_CELLS.len(), 10);
    }

    #[test]
    fn visual_combat_marker_gallery_frame_preserves_marker_and_cursor_pixels() {
        let mut state = world_state(open_world_grid(), 10, 20);
        seed_visual_combat_marker_gallery(&mut state).unwrap();
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();

        let rgba = render_visual_play_frame(&mut state, &atlas, &font);
        let width = VISUAL_PLAY_FRAME_WIDTH as usize;
        let cell_center = |x: usize, y: usize| {
            (
                x * TILE_ATLAS_SIDE + TILE_ATLAS_SIDE / 2,
                y * TILE_ATLAS_SIDE + TILE_ATLAS_SIDE / 2,
            )
        };

        let (x, y) = cell_center(1, 2);
        assert_eq!(
            rgba_pixel(&rgba, width, x, y),
            ega_rgba(usize::from(COMBAT_PARTY_CORPSE_TILE) % 16)
        );
        let (x, y) = cell_center(7, 3);
        assert_eq!(
            rgba_pixel(&rgba, width, x, y),
            ega_rgba(usize::from(COMBAT_FIELD_KIND_POISON) % 16)
        );
        let (x, y) = cell_center(3, 4);
        assert_eq!(rgba_pixel(&rgba, width, x, y), ega_rgba(11));
        assert_eq!(
            rgba_pixel(&rgba, width, 5 * TILE_ATLAS_SIDE, 8 * TILE_ATLAS_SIDE),
            ega_rgba(14)
        );
    }

    #[test]
    fn dungeon_framebuffer_renders_first_person_raster_when_lit() {
        let mut grid = open_dungeon_record();
        grid[dungeon_cell_index(0, 2, 1)] = 0x40;
        grid[dungeon_cell_index(0, 2, 0)] = 0xb0;
        let mut state = dungeon_state(grid, 0, 1, 1);
        state.player.facing = Direction::East;
        state.torch_counter = 9;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let rgba = render_framebuffer(&mut state, &atlas);

        assert_eq!(
            rgba.len(),
            (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4
        );
        assert!(rgba.chunks_exact(4).any(|pixel| pixel
            == [
                EGA_PALETTE_RGB[15][0],
                EGA_PALETTE_RGB[15][1],
                EGA_PALETTE_RGB[15][2],
                0xff
            ]));
        assert!(rgba.chunks_exact(4).any(|pixel| pixel
            == [
                EGA_PALETTE_RGB[8][0],
                EGA_PALETTE_RGB[8][1],
                EGA_PALETTE_RGB[8][2],
                0xff
            ]));
    }

    #[test]
    fn dungeon_framebuffer_stays_black_without_light() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.player.facing = Direction::East;
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let rgba = render_framebuffer(&mut state, &atlas);

        assert_eq!(
            rgba.len(),
            (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4
        );
        assert!(
            rgba.chunks_exact(4)
                .all(|pixel| pixel == [0x00, 0x00, 0x00, 0xff])
        );
    }

    #[test]
    fn text_panel_wrapper_preserves_short_lines_and_wraps_long_status() {
        let lines =
            wrap_text_panel_lines("DUNGEON:0 LEVEL 0\nA VERY LONG DUNGEON STATUS LINE", 12, 6);

        assert_eq!(lines[0], "DUNGEON:0");
        assert_eq!(lines[1], "LEVEL 0");
        assert!(lines.iter().any(|line| line == "A VERY LONG"));
        assert!(lines.iter().any(|line| line == "DUNGEON"));
    }

    #[test]
    fn status_framebuffer_uses_fixed_cell_text_surface() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.active_player = Some(0);

        let rgba = render_status_framebuffer(&mut state, "", READY_HINT, &font);

        assert_eq!(
            rgba.len(),
            TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4
        );
        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| pixel == [0xff, 0xff, 0xff, 0xff])
        );
        assert_eq!(state.active_player, None);
    }

    #[test]
    fn status_framebuffer_refreshes_moon_glyphs_before_rendering() {
        let mut font_bytes = vec![0x00; CH_FONT_LEN];
        for row in 0..CH_CELL_SIDE {
            font_bytes[usize::from(b'6') * CH_CELL_SIDE + row] = 0xff;
        }
        let font = parse_ch_font(&font_bytes, IBM_CH_FILE).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.clock = GameClock::with_date(12, 5, 18, 17, 0).unwrap();
        state.set_cached_moon_glyph_bytes(b'0', b'0');

        let rgba = render_status_framebuffer(&mut state, "", READY_HINT, &font);

        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| pixel == [0xff, 0xff, 0xff, 0xff]),
            "Felucca hour-17 glyph `6` should be refreshed from the clock before Bevy renders"
        );
    }

    #[test]
    fn intro_framebuffer_matches_public_display_surface() {
        assert_eq!(INTRO_FRAMEBUFFER_WIDTH, 320);
        assert_eq!(INTRO_FRAMEBUFFER_HEIGHT, 200);
        assert_eq!(INTRO_FRAMEBUFFER_WIDTH, VISUAL_PLAY_FRAME_WIDTH);
        assert_eq!(INTRO_FRAMEBUFFER_HEIGHT, VISUAL_PLAY_FRAME_HEIGHT);
    }

    #[test]
    fn intro_menu_frame_renders_nonblank_rgba() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: intro_title_flourish_total_steps(),
            title_flourish_complete: true,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: "Intro menu smoke".to_string(),
            panel: VisualIntroPanel::Menu,
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        let frame = render_intro_frame(&mut intro);

        assert_eq!(
            frame.len(),
            (INTRO_FRAMEBUFFER_WIDTH as usize) * (INTRO_FRAMEBUFFER_HEIGHT as usize) * 4
        );
        assert!(frame.chunks_exact(4).all(|pixel| pixel[3] == 0xff));
        assert_nonblack_rgba(&frame);
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn intro_start_menu_reveal_blocks_input_until_full_startsc_rect_completes() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: Some(RectColumnSweepTransition::new(INTRO_START_MENU_REVEAL_RECT)),
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::Menu,
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };
        assert_eq!(INTRO_START_MENU_REVEAL_RECT, (0, 0, 319, 100));
        assert!(visual_intro_start_menu_reveal_active(&intro));

        let total_ticks =
            u5_runtime::intro_rect_transition_tick_count(INTRO_START_MENU_REVEAL_RECT);
        for expected_tick in 1..total_ticks {
            assert!(advance_visual_intro_start_menu_reveal(&mut intro));
            assert_eq!(
                intro.start_menu_reveal.map(|reveal| reveal.tick),
                Some(expected_tick)
            );
            assert!(visual_intro_start_menu_reveal_active(&intro));
        }

        assert!(advance_visual_intro_start_menu_reveal(&mut intro));
        assert_eq!(intro.start_menu_reveal, None);
        assert!(!visual_intro_start_menu_reveal_active(&intro));
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn intro_story_draw_specs_include_transition_and_secondary_art() {
        assert_eq!(
            visual_intro_story_draw_specs(7),
            vec![
                IntroStoryDrawSpec {
                    stem: "TEXT",
                    subimage: 0,
                    top_left_x: 232,
                    top_left_y: 26,
                    clip_width: None,
                    clip_height: None,
                },
                IntroStoryDrawSpec {
                    stem: "TEXT",
                    subimage: 2,
                    top_left_x: 200,
                    top_left_y: 54,
                    clip_width: None,
                    clip_height: None,
                },
                IntroStoryDrawSpec {
                    stem: "STORY3",
                    subimage: 0,
                    top_left_x: 0,
                    top_left_y: 0,
                    clip_width: None,
                    clip_height: None,
                },
            ]
        );

        assert!(
            visual_intro_story_draw_specs(1).contains(&IntroStoryDrawSpec {
                stem: "STORY1",
                subimage: INTRO_STEP_1_EXTRA_SUBIMAGE,
                top_left_x: INTRO_STEP_1_EXTRA_ART_X,
                top_left_y: INTRO_STEP_1_EXTRA_ART_Y,
                clip_width: None,
                clip_height: None,
            })
        );
        assert!(
            visual_intro_story_draw_specs(INTRO_INLINE_DOORWAY_STEP).contains(
                &IntroStoryDrawSpec {
                    stem: "STORY2",
                    subimage: INTRO_STEP_6_EXTRA_SUBIMAGE,
                    top_left_x: INTRO_STEP_6_EXTRA_ART_X,
                    top_left_y: INTRO_STEP_6_EXTRA_ART_Y,
                    clip_width: None,
                    clip_height: None,
                }
            )
        );
        assert!(
            visual_intro_story_draw_specs(15).contains(&IntroStoryDrawSpec {
                stem: "STORY6",
                subimage: 3,
                top_left_x: 176,
                top_left_y: 55,
                clip_width: None,
                clip_height: None,
            })
        );
    }

    #[test]
    fn intro_story_step_one_extra_art_is_column_wiped_after_keypress() {
        let hidden = visual_intro_story_draw_specs_for_active_panel(1, None);
        assert!(!hidden.iter().any(|spec| {
            spec.stem == "STORY1"
                && spec.subimage == INTRO_STEP_1_EXTRA_SUBIMAGE
                && spec.top_left_x == INTRO_STEP_1_EXTRA_ART_X
                && spec.top_left_y == INTRO_STEP_1_EXTRA_ART_Y
        }));

        let tick0 = visual_intro_story_draw_specs_for_active_panel(
            1,
            Some(RectColumnSweepTransition::new(INTRO_STEP_1_RECT_TRANSITION)),
        );
        let extra = tick0
            .iter()
            .find(|spec| spec.stem == "STORY1" && spec.subimage == INTRO_STEP_1_EXTRA_SUBIMAGE)
            .unwrap();
        assert_eq!(extra.clip_width, Some(1));
        assert_eq!(extra.clip_height, Some(35));

        let tick35 = visual_intro_story_draw_specs_for_active_panel(
            1,
            Some(RectColumnSweepTransition {
                rect: INTRO_STEP_1_RECT_TRANSITION,
                tick: 35,
            }),
        );
        let extra = tick35
            .iter()
            .find(|spec| spec.stem == "STORY1" && spec.subimage == INTRO_STEP_1_EXTRA_SUBIMAGE)
            .unwrap();
        assert_eq!(extra.clip_width, Some(36));
        assert_eq!(extra.clip_height, Some(35));
    }

    #[test]
    fn intro_story_step_one_key_starts_wipe_before_advancing_step() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: intro_title_flourish_total_steps(),
            title_flourish_complete: true,
            title_signature_progress: 0,
            title_signature_complete: true,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::Story {
                records: StoryRecords {
                    records: (0..20).map(|i| format!("Story record {i}")).collect(),
                },
                step: 1,
                transition: None,
            },
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        assert!(step_visual_intro_panel(&mut intro, ' '));

        match &intro.panel {
            VisualIntroPanel::Story {
                step, transition, ..
            } => {
                assert_eq!(*step, 1);
                assert_eq!(
                    *transition,
                    Some(RectColumnSweepTransition::new(INTRO_STEP_1_RECT_TRANSITION))
                );
            }
            _ => panic!("story panel should remain active"),
        }
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn intro_story_step_zero_auto_advances_without_keypress() {
        let mut panel = VisualIntroPanel::Story {
            records: StoryRecords {
                records: (0..20).map(|i| format!("Story record {i}")).collect(),
            },
            step: 0,
            transition: None,
        };

        assert!(advance_visual_intro_story_auto_step(&mut panel));
        match panel {
            VisualIntroPanel::Story {
                step, transition, ..
            } => {
                assert_eq!(step, 1);
                assert_eq!(transition, None);
            }
            _ => panic!("story panel should remain active"),
        }
    }

    #[test]
    fn intro_story_step_one_wipe_advances_on_title_ticks_then_enters_step_two() {
        let mut panel = VisualIntroPanel::Story {
            records: StoryRecords {
                records: (0..20).map(|i| format!("Story record {i}")).collect(),
            },
            step: 1,
            transition: Some(RectColumnSweepTransition {
                rect: INTRO_STEP_1_RECT_TRANSITION,
                tick: 34,
            }),
        };
        let mut title_tick_frame = 0;

        assert!(advance_visual_intro_story_wipe(
            &mut panel,
            &mut title_tick_frame
        ));
        match &panel {
            VisualIntroPanel::Story {
                step, transition, ..
            } => {
                assert_eq!(*step, 1);
                assert_eq!(
                    *transition,
                    Some(RectColumnSweepTransition {
                        rect: INTRO_STEP_1_RECT_TRANSITION,
                        tick: 35,
                    })
                );
            }
            _ => panic!("story panel should remain active"),
        }

        assert!(advance_visual_intro_story_wipe(
            &mut panel,
            &mut title_tick_frame
        ));
        match panel {
            VisualIntroPanel::Story {
                step, transition, ..
            } => {
                assert_eq!(step, 2);
                assert_eq!(transition, None);
            }
            _ => panic!("story panel should remain active"),
        }
    }

    #[test]
    fn finished_intro_menu_keeps_title_surface_and_overlays_menu_text() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::Menu,
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };
        assert!(visual_intro_title_surface_visible(&intro));
        assert!(matches!(
            intro.dispatch.tick_title(),
            UnifiedMenuStep::PresentTitle
        ));

        intro.dispatch.dismiss_title();
        assert!(visual_intro_title_surface_visible(&intro));
        assert!(!matches!(
            intro.dispatch.tick_title(),
            UnifiedMenuStep::PresentTitle
        ));

        let mut frame = vec![0; (INTRO_FRAMEBUFFER_WIDTH as usize) * 16 * 4];
        for pixel in frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0x22, 0x22, 0x22, 0xff]);
        }
        overlay_nonblack_text_panel_rgba(
            &mut frame,
            INTRO_FRAMEBUFFER_WIDTH as usize,
            16,
            "J  Journey Onward",
        );
        assert!(frame.chunks_exact(4).any(|pixel| {
            pixel[3] == 0xff
                && (pixel[0] != 0x22 || pixel[1] != 0x22 || pixel[2] != 0x22)
                && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        }));
        assert!(
            frame
                .chunks_exact(4)
                .any(|pixel| pixel == [0x22, 0x22, 0x22, 0xff])
        );
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn visual_intro_menu_invalid_key_is_silent() {
        let mut intro = visual_intro_state_with_panel(debug_game_dir(), VisualIntroPanel::Menu);

        assert!(step_visual_intro(&mut intro, 'x'));

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.is_empty());
        assert!(!intro.message_waiting_for_key);
        assert_eq!(intro.menu_idle_ticks, 0);
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn intro_title_dismiss_runs_startsc_clear_carry_tick() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: intro_title_flourish_total_steps(),
            title_flourish_complete: true,
            title_signature_progress: 0,
            title_signature_complete: true,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::Menu,
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        assert!(step_visual_intro(&mut intro, 'x'));

        assert_eq!(intro.title_tick_visible_frame, 0);
        assert_eq!(intro.title_tick_frame, title_tick_next_frame(0));
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn intro_menu_idle_draws_current_title_tick_then_advances_counter() {
        let mut intro = visual_intro_state_with_panel(debug_game_dir(), VisualIntroPanel::Menu);
        intro.title_tick_frame = 2;
        intro.title_tick_visible_frame = 0;

        assert!(advance_visual_intro_animation_tick(&mut intro));

        assert_eq!(intro.title_tick_visible_frame, 2);
        assert_eq!(intro.title_tick_frame, title_tick_next_frame(2));
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn intro_menu_cached_selection_renders_inverse_highlight() {
        let font = parse_ch_font(&vec![0x00; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let mut frame =
            vec![0; (INTRO_FRAMEBUFFER_WIDTH as usize) * (INTRO_FRAMEBUFFER_HEIGHT as usize) * 4];
        for pixel in frame.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
        }

        draw_intro_menu_labels_rgba(&mut frame, &font, Some(IntroSubflow::Acknowledgements));

        assert_eq!(
            rgba_pixel(
                &frame,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                11 * CH_CELL_SIDE,
                21 * CH_CELL_SIDE
            ),
            [0xff, 0xff, 0xff, 0xff]
        );
        assert_eq!(
            rgba_pixel(
                &frame,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                12 * CH_CELL_SIDE,
                17 * CH_CELL_SIDE
            ),
            [0x00, 0x00, 0x00, 0xff]
        );
    }

    #[test]
    fn visual_intro_menu_message_wait_consumes_next_key_without_dispatch() {
        let mut intro = visual_intro_state_with_panel(debug_game_dir(), VisualIntroPanel::Menu);
        intro.message = "No active game".to_string();
        intro.message_waiting_for_key = true;
        intro.title_tick_frame = 2;
        intro.title_tick_visible_frame = 0;
        intro.menu_idle_ticks = 57;

        assert!(step_visual_intro(&mut intro, 'j'));

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.is_empty());
        assert!(!intro.message_waiting_for_key);
        assert_eq!(intro.menu_idle_ticks, 0);
        assert_eq!(intro.title_tick_visible_frame, 2);
        assert_eq!(intro.title_tick_frame, title_tick_next_frame(2));
        assert!(intro.launch_result.lock().unwrap().is_none());
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn visual_intro_menu_idle_timeout_enters_return_to_view() {
        let mut intro = visual_intro_state_with_panel(debug_game_dir(), VisualIntroPanel::Menu);
        intro.menu_idle_ticks = INTRO_MENU_IDLE_RETURN_TO_VIEW_TICKS - 1;

        assert!(advance_visual_intro_finished_menu_idle(&mut intro));

        assert!(matches!(intro.panel, VisualIntroPanel::ReturnToView { .. }));
        assert_eq!(intro.menu_idle_ticks, 0);
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn intro_title_art_composition_clears_lower_band_then_draws_remaining_slots() {
        let blank = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![0],
        };
        let mut blocks = vec![blank; 10];
        blocks[6] = MonochromeBitmap {
            width: 1,
            height: 61,
            pixels: vec![1; 61],
        };
        blocks[7] = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };
        blocks[8] = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };
        blocks[9] = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };
        let title = TitleBitImages { blocks };
        let british = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };

        let rgba = compose_intro_title_art_rgba(
            &title,
            &british,
            IntroTitleCompositionPhase::Signature {
                completed_signature: true,
            },
        );
        let width = TITLE_SURFACE_WIDTH as usize;
        let flourish_rgb = EGA_PALETTE_RGB[9];

        assert_eq!(rgba.len(), width * (TITLE_SURFACE_HEIGHT as usize) * 4);
        assert_eq!(
            rgba_pixel(&rgba, width, 20, 46),
            [flourish_rgb[0], flourish_rgb[1], flourish_rgb[2], 0xff]
        );
        assert_eq!(rgba_pixel(&rgba, width, 20, 118), [0, 0, 0, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 20, 140), [0, 0, 0, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 108, 140), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 152, 0), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 24, 66), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 104, 160), [0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn intro_title_art_defers_completed_signature_overlays_until_path_finishes() {
        let blank = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![0],
        };
        let mut blocks = vec![blank; 10];
        blocks[7] = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };
        blocks[8] = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };
        blocks[9] = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };
        let title = TitleBitImages { blocks };
        let british = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![1],
        };

        let rgba = compose_intro_title_art_rgba(
            &title,
            &british,
            IntroTitleCompositionPhase::Signature {
                completed_signature: false,
            },
        );
        let width = TITLE_SURFACE_WIDTH as usize;

        assert_eq!(rgba_pixel(&rgba, width, 108, 140), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 152, 0), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 24, 66), [0, 0, 0, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 104, 160), [0, 0, 0, 0xff]);
    }

    #[test]
    fn intro_title_flourish_replaces_prior_slots_and_reveals_rows() {
        let blank = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![0],
        };
        let mut blocks = vec![blank; 10];
        blocks[0] = MonochromeBitmap {
            width: 24,
            height: 3,
            pixels: vec![1; 24 * 3],
        };
        blocks[1] = MonochromeBitmap {
            width: 40,
            height: 7,
            pixels: vec![1; 40 * 7],
        };
        let title = TitleBitImages { blocks };
        let british = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![0],
        };

        let rgba = compose_intro_title_art_rgba(
            &title,
            &british,
            IntroTitleCompositionPhase::Flourish { step: 12 },
        );
        let width = TITLE_SURFACE_WIDTH as usize;
        let flourish_rgb = EGA_PALETTE_RGB[9];

        assert_eq!(rgba_pixel(&rgba, width, 148, 75), [0, 0, 0, 0xff]);
        assert_eq!(
            rgba_pixel(&rgba, width, 140, 73),
            [flourish_rgb[0], flourish_rgb[1], flourish_rgb[2], 0xff]
        );
        assert_eq!(rgba_pixel(&rgba, width, 140, 72), [0, 0, 0, 0xff]);
    }

    #[test]
    fn british_signature_renderer_paints_pen_down_steps_from_spec_origins() {
        let mut rgba =
            vec![0; (TITLE_SURFACE_WIDTH as usize) * (TITLE_SURFACE_HEIGHT as usize) * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
        }
        let signature = BritishPth {
            segments: vec![
                vec![
                    PenStroke {
                        dx: 1,
                        dy: 0,
                        pen_down: true,
                    },
                    PenStroke {
                        dx: 5,
                        dy: 0,
                        pen_down: false,
                    },
                    PenStroke {
                        dx: 0,
                        dy: 1,
                        pen_down: true,
                    },
                ],
                vec![PenStroke {
                    dx: 0,
                    dy: 1,
                    pen_down: true,
                }],
                vec![PenStroke {
                    dx: -1,
                    dy: 0,
                    pen_down: true,
                }],
                vec![PenStroke {
                    dx: 1,
                    dy: -1,
                    pen_down: false,
                }],
            ],
        };

        draw_british_signature_rgba(
            &mut rgba,
            TITLE_SURFACE_WIDTH as usize,
            TITLE_SURFACE_HEIGHT as usize,
            &signature,
            usize::MAX,
        );
        let width = TITLE_SURFACE_WIDTH as usize;

        assert_eq!(rgba_pixel(&rgba, width, 69, 44), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 74, 44), [0, 0, 0, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 74, 45), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 94, 65), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 77, 143), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 106, 166), [0, 0, 0, 0xff]);
    }

    #[test]
    fn title_tick_overlay_stays_inside_spec_strip_and_overwrites_title_pixels() {
        let width = TITLE_SURFACE_WIDTH as usize;
        let height = TITLE_SURFACE_HEIGHT as usize;
        let mut frame0 = vec![0; width * height * 4];
        for pixel in frame0.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
        }
        let preserved_x = 16usize;
        let preserved_y = TITLE_TICK_FRAME_Y as usize + 2;
        let preserved_offset = (preserved_y * width + preserved_x) * 4;
        frame0[preserved_offset..preserved_offset + 4].copy_from_slice(&[0xff, 0xff, 0xff, 0xff]);
        let mut frame1 = frame0.clone();

        draw_title_tick_overlay_rgba(&mut frame0, width, height, 0);
        draw_title_tick_overlay_rgba(&mut frame1, width, height, 1);

        assert_eq!(
            rgba_pixel(&frame0, width, preserved_x, preserved_y),
            [0, 0, 0, 0xff]
        );
        assert!(frame0.chunks_exact(4).enumerate().any(|(index, pixel)| {
            let x = index % width;
            let y = index / width;
            y >= TITLE_TICK_FRAME_Y as usize
                && y < (TITLE_TICK_FRAME_Y + TITLE_TICK_FRAME_HEIGHT) as usize
                && x < TITLE_TICK_FRAME_WIDTH as usize
                && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        }));
        assert!(
            frame0
                .chunks_exact(4)
                .enumerate()
                .filter(|(index, _)| {
                    let y = index / width;
                    y < TITLE_TICK_FRAME_Y as usize
                        || y >= (TITLE_TICK_FRAME_Y + TITLE_TICK_FRAME_HEIGHT) as usize
                })
                .all(|(_, pixel)| pixel == [0x00, 0x00, 0x00, 0xff])
        );
        assert_ne!(frame0, frame1);
    }

    #[test]
    fn intro_display_title_tick_overwrites_full_spec_strip() {
        let mut buffer = IntroDisplayBuffer::new(
            INTRO_FRAMEBUFFER_WIDTH as usize,
            INTRO_FRAMEBUFFER_HEIGHT as usize,
        );
        buffer.clear(0x03);

        buffer.draw_title_tick(0);

        assert_eq!(
            buffer.pixels[(TITLE_TICK_FRAME_Y as usize - 1) * buffer.width + 54],
            0x03
        );
        assert_eq!(
            buffer.pixels[TITLE_TICK_FRAME_Y as usize * buffer.width + 54],
            0x00
        );
        assert_eq!(
            buffer.pixels[(TITLE_TICK_FRAME_Y as usize + 20) * buffer.width + 54],
            title_tick_flame_palette_index(54, 20, 0).unwrap()
        );
        assert_eq!(
            buffer.pixels[(TITLE_TICK_FRAME_Y as usize + 20) * buffer.width + 120],
            0x00
        );
    }

    #[test]
    fn title_tick_flame_stripe_uses_published_palette_cycle() {
        // `cleak/u5-spec#65`: the clean replacement keeps the
        // independently-authored flame silhouette and treats non-flame
        // pixels as opaque black inside the title-tick rectangle.
        assert_eq!(title_tick_flame_palette_index(54, 8, 0), None);
        assert_eq!(title_tick_flame_palette_index(54, 20, 0), Some(0x0E));
        assert_eq!(title_tick_flame_palette_index(54, 40, 0), Some(0x06));
        assert_eq!(title_tick_flame_palette_index(160, 20, 1), Some(0x0C));
        assert_eq!(title_tick_flame_palette_index(160, 40, 1), Some(0x04));
        assert_eq!(title_tick_flame_palette_index(266, 20, 2), Some(0x0E));
        assert_eq!(title_tick_flame_palette_index(266, 40, 3), Some(0x06));
        assert_eq!(title_tick_flame_palette_index(120, 20, 0), None);
    }

    #[test]
    fn title_tick_overlay_draws_dense_wavy_flame_band() {
        let width = TITLE_SURFACE_WIDTH as usize;
        let height = TITLE_SURFACE_HEIGHT as usize;
        let mut rgba = vec![0; width * height * 4];
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
        }

        draw_title_tick_overlay_rgba(&mut rgba, width, height, 0);

        let lit_in_band = rgba
            .chunks_exact(4)
            .enumerate()
            .filter(|(index, pixel)| {
                let y = index / width;
                y >= TITLE_TICK_FRAME_Y as usize
                    && y < (TITLE_TICK_FRAME_Y + TITLE_TICK_FRAME_HEIGHT) as usize
                    && (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
            })
            .count();
        assert!(
            lit_in_band > 2_000,
            "procedural flame stripe should be a dense band, got {lit_in_band} lit pixels"
        );
        assert_eq!(
            rgba_pixel(&rgba, width, 54, TITLE_TICK_FRAME_Y as usize + 20),
            [0xff, 0xff, 0x55, 0xff]
        );
        assert_eq!(
            rgba_pixel(&rgba, width, 54, TITLE_TICK_FRAME_Y as usize + 40),
            [0xaa, 0x55, 0x00, 0xff]
        );
    }

    #[test]
    fn endgame_status_framebuffer_renders_modal_surface() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.enter_endgame();

        let rgba = render_status_framebuffer(&mut state, "", "", &font);

        assert_eq!(
            rgba.len(),
            TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4
        );
        assert_nonblack_rgba(&rgba);
        assert!(state.endgame.is_some());
    }

    #[test]
    fn rect_column_sweep_reveal_copies_columns_from_source() {
        let width = 8;
        let height = 4;
        let mut destination = vec![0x11; width * height * 4];
        let source = vec![0xee; width * height * 4];
        let transition = RectColumnSweepTransition {
            rect: (2, 1, 6, 2),
            tick: 1,
        };

        apply_rect_column_sweep_reveal_rgba(&mut destination, &source, width, height, transition);

        assert_eq!(
            rgba_pixel(&destination, width, 2, 1),
            [0xee, 0xee, 0xee, 0xee]
        );
        assert_eq!(
            rgba_pixel(&destination, width, 3, 2),
            [0xee, 0xee, 0xee, 0xee]
        );
        assert_eq!(
            rgba_pixel(&destination, width, 4, 1),
            [0x11, 0x11, 0x11, 0x11]
        );
        assert_eq!(
            rgba_pixel(&destination, width, 6, 2),
            [0x11, 0x11, 0x11, 0x11]
        );
        assert_eq!(
            rgba_pixel(&destination, width, 7, 1),
            [0x11, 0x11, 0x11, 0x11]
        );
        assert_eq!(
            rgba_pixel(&destination, width, 4, 3),
            [0x11, 0x11, 0x11, 0x11]
        );
    }

    #[test]
    fn visual_wait_frame_consumes_endgame_certificate_rect_operation_before_certificate() {
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.endgame = Some(u5_runtime::EndgameState::terminal(
            true,
            true,
            true,
            "Certificate".to_string(),
            None,
            None,
        ));
        let endgame = state.endgame.as_mut().unwrap();
        for _ in 0..(1 + u5_runtime::endgame_cinematic::ENDGAME_NARRATIVE_WINDOW_COUNT) {
            endgame.cinematic.advance();
        }
        assert_eq!(
            endgame.cinematic.step,
            u5_runtime::endgame_cinematic::EndgameCinematicStep::CertificateRectOperation
        );
        assert_eq!(
            endgame.cinematic.certificate_rect_operation,
            Some(u5_runtime::endgame_cinematic::ENDGAME_CERTIFICATE_RECT_OPERATION)
        );

        let mut prompt_cursor_visible = true;
        assert!(advance_visual_wait_frame(
            &mut state,
            &mut prompt_cursor_visible
        ));

        assert!(!prompt_cursor_visible);
        assert_eq!(
            state.endgame.as_ref().map(|endgame| endgame.cinematic.step),
            Some(u5_runtime::endgame_cinematic::EndgameCinematicStep::Certificate)
        );
        assert_eq!(
            state
                .endgame
                .as_ref()
                .and_then(|endgame| endgame.cinematic.certificate_rect_operation),
            None
        );
    }

    #[test]
    fn visual_play_frame_composes_endgame_tableau_over_modal_surface() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.enter_endgame();

        let mut expected_state = state.clone();
        let expected = render_status_framebuffer(&mut expected_state, "", READY_HINT, &font);
        let rgba = render_visual_play_frame(&mut state, &atlas, &font);

        assert_ne!(rgba, expected);
        assert_nonblack_rgba(&rgba);
        assert_eq!(
            rgba.len(),
            (VISUAL_PLAY_FRAME_WIDTH as usize) * (VISUAL_PLAY_FRAME_HEIGHT as usize) * 4
        );
    }

    #[test]
    fn visual_play_frame_stops_tableau_overlay_after_victory_throne() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] = SPECIAL_ITEM_OWNED_VALUE;
        state.enter_endgame();
        state.resolve_endgame_confirmation(true);
        state.resolve_endgame_confirmation(true);
        for _ in 0..(ENDGAME_TABLEAU_WIDTH * ENDGAME_TABLEAU_HEIGHT * 2) {
            state.resolve_endgame_confirmation(true);
            if matches!(
                state.endgame.as_ref().map(|endgame| endgame.cinematic.step),
                Some(u5_runtime::endgame_cinematic::EndgameCinematicStep::NarrativeWindow(0))
            ) {
                break;
            }
        }
        assert!(matches!(
            state.endgame.as_ref().map(|endgame| endgame.cinematic.step),
            Some(u5_runtime::endgame_cinematic::EndgameCinematicStep::NarrativeWindow(0))
        ));

        let mut expected_state = state.clone();
        let expected = render_status_framebuffer(&mut expected_state, "", READY_HINT, &font);
        let rgba = render_visual_play_frame(&mut state, &atlas, &font);

        assert_eq!(rgba, expected);
    }

    #[test]
    fn endgame_tableau_viewport_uses_loaded_grid_and_low_slot_top_order() {
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.grid = vec![0; TOWN_GRID_SIDE * TOWN_GRID_SIDE];
        state.grid[0] = 0x21;
        state
            .active_objects
            .resize(u5_runtime::OOL_SLOTS, ActiveObject::empty());
        state.active_objects[31] = ActiveObject {
            type_byte: 0x7c,
            tile: 0x7c,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[6] = ActiveObject {
            type_byte: 0x0e,
            tile: 0x0e,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };
        state.active_objects[0] = ActiveObject {
            type_byte: 0x44,
            tile: 0x44,
            x: 5,
            y: 5,
            z: 0,
            phase: 0,
            aux1: 0,
            aux3: 0,
        };

        let viewport = render_endgame_tableau_viewport(&state, &atlas).unwrap();

        assert_eq!(viewport.pixels[0], 0x21 % 16);
        let overlap_pixel = 5 * TILE_ATLAS_SIDE * viewport.width + 5 * TILE_ATLAS_SIDE;
        assert_eq!(viewport.pixels[overlap_pixel], 0x44 % 16);
    }

    #[test]
    fn visual_play_frame_composes_viewport_and_status_surface() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);

        let rgba = render_visual_play_frame(&mut state, &atlas, &font);

        assert_eq!(
            rgba.len(),
            (VISUAL_PLAY_FRAME_WIDTH as usize) * (VISUAL_PLAY_FRAME_HEIGHT as usize) * 4
        );
        assert!(rgba.chunks_exact(4).all(|pixel| pixel[3] == 0xff));
        assert_nonblack_rgba(&rgba);
    }

    #[test]
    fn visual_display_message_uses_named_starting_location() {
        let mut state = test_state(open_grid(), 15, 15);
        state.area = u5_runtime::Area::Town {
            scene: Scene::new(u5_runtime::CHARGEN_STARTING_SCENE).unwrap(),
            floor: 0,
        };
        state.message = "Entered DWELLING:4 at (15, 15).".to_string();

        let message = visual_display_message(&state);

        assert_eq!(message, "Entered Iolo's Hut at (15, 15).");
    }

    #[test]
    fn visual_play_frame_blits_view_overlay_into_side_panel_over_base_viewport() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        state.gems = 1;
        state.view_gem();
        assert!(state.active_view_overlay.is_some());
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let overlay = state
            .render_active_view_overlay(TileGraphicsDepth::Ega16)
            .unwrap();
        let overlay_rgba = overlay.to_rgba();
        let (overlay_index, expected_overlay_pixel) = overlay_rgba
            .chunks_exact(4)
            .enumerate()
            .find_map(|(index, pixel)| {
                (pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
                    .then_some((index, [pixel[0], pixel[1], pixel[2], pixel[3]]))
            })
            .expect("overlay should contain nonblack pixels");
        let overlay_sample_x = overlay_index % overlay.width;
        let overlay_sample_y = overlay_index / overlay.width;

        let rgba = render_visual_play_frame(&mut state, &atlas, &font);
        let width = VISUAL_PLAY_FRAME_WIDTH as usize;
        let base_pixel = rgba_pixel(
            &rgba,
            width,
            VIEWPORT_SIZE_PX as usize / 2,
            VIEWPORT_SIZE_PX as usize / 2,
        );
        let overlay_pixel = rgba_pixel(
            &rgba,
            width,
            VISUAL_OVERLAY_SIDE_PANEL_X + overlay_sample_x,
            VISUAL_OVERLAY_SIDE_PANEL_Y + overlay_sample_y,
        );

        assert_ne!(base_pixel, [0x00, 0x00, 0x00, 0xff]);
        assert_eq!(overlay_pixel, expected_overlay_pixel);
    }

    #[test]
    fn centered_overlay_framebuffer_preserves_fixed_bevy_texture_size() {
        let mut src = vec![0; 2 * 2 * 4];
        for pixel in src.chunks_exact_mut(4) {
            pixel.copy_from_slice(&[0xaa, 0xbb, 0xcc, 0xff]);
        }

        let rgba = center_rgba_on_viewport(src, 2, 2);

        assert_eq!(
            rgba.len(),
            (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4
        );
        assert!(rgba.chunks_exact(4).all(|pixel| pixel[3] == 0xff));
        assert!(
            rgba.chunks_exact(4)
                .any(|pixel| pixel == [0xaa, 0xbb, 0xcc, 0xff])
        );
    }

    #[test]
    fn visual_frame_suite_local_clean_writes_pngs_and_manifest_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join("CASTLE.DAT").exists()
            || !game_dir.join(TILES_EGA_FILE).exists()
            || !game_dir.join(IBM_CH_FILE).exists()
        {
            return;
        }

        let dir = temp_output_dir("suite");
        let reports = visual_frame_suite(game_dir, TileGraphicsDepth::Ega16, &dir).unwrap();

        let has_story = game_dir.join(STORY_DAT_FILE).exists();
        assert_eq!(reports.len(), if has_story { 163 } else { 162 });
        for report in &reports {
            assert!(report.path.exists());
            assert!(report.nonblack_pixels > 0);
        }
        for label in [
            "world-play",
            "world-after-step",
            "town-play",
            "dungeon-play",
            "dungeon-dark",
            "world-save-confirmation-prompt",
            "world-hole-up-watch-prompt",
            "world-use-item-prompt",
            "castle-cast-party-prompt",
            "castle-mix-reagent-prompt",
            "castle-ready-party-prompt",
            "castle-talk-keyword-prompt",
            "dungeon-search-direction-prompt",
            "dungeon-open-direction-prompt",
            "combat-play",
            "combat-status-highlight",
            "combat-attack-direction-prompt",
            "combat-cast-party-prompt",
            "combat-ready-party-prompt",
            "combat-search-direction-prompt",
            "surface-view-overlay",
            "dungeon-view-overlay",
            "britannia-chunk-map-overlay",
            "peer-view-overlay",
            "x-ray-view-overlay",
            "surface-view-class-gallery",
            "peer-view-class-gallery",
            "x-ray-view-class-gallery",
            "z-stats-modal",
            "endgame-status",
            "combat-marker-gallery",
        ] {
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected visual gameplay report");
            assert_eq!(report.width, VISUAL_PLAY_FRAME_WIDTH);
            assert_eq!(report.height, VISUAL_PLAY_FRAME_HEIGHT);
        }
        for arena_index in 0..BRIT_CBT_RECORDS {
            let label = format!("combat-arena-{arena_index:02}");
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected outdoor combat arena gallery report");
            assert_eq!(report.width, VISUAL_PLAY_FRAME_WIDTH);
            assert_eq!(report.height, VISUAL_PLAY_FRAME_HEIGHT);
        }
        for arena_index in 0..DUNGEON_CBT_RECORDS {
            let label = format!("dungeon-combat-arena-{arena_index:03}");
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected dungeon combat arena gallery report");
            assert_eq!(report.width, VISUAL_PLAY_FRAME_WIDTH);
            assert_eq!(report.height, VISUAL_PLAY_FRAME_HEIGHT);
        }
        let intro_labels: &[&str] = if has_story {
            &[
                "intro-menu",
                "intro-finished-menu",
                "intro-story-art",
                "intro-return-to-view",
            ]
        } else {
            &["intro-menu", "intro-finished-menu", "intro-return-to-view"]
        };
        for label in intro_labels {
            let report = reports
                .iter()
                .find(|report| report.label == *label)
                .expect("expected visual intro report");
            assert_eq!(report.width, INTRO_FRAMEBUFFER_WIDTH);
            assert_eq!(report.height, INTRO_FRAMEBUFFER_HEIGHT);
        }
        let manifest = fs::read_to_string(dir.join("manifest.txt")).unwrap();
        assert!(manifest.contains("world-play"));
        assert!(manifest.contains("world-after-step"));
        assert!(manifest.contains("town-play"));
        assert!(manifest.contains("dungeon-play"));
        assert!(manifest.contains("dungeon-dark"));
        assert!(manifest.contains("world-save-confirmation-prompt"));
        assert!(manifest.contains("world-hole-up-watch-prompt"));
        assert!(manifest.contains("world-use-item-prompt"));
        assert!(manifest.contains("castle-cast-party-prompt"));
        assert!(manifest.contains("castle-mix-reagent-prompt"));
        assert!(manifest.contains("castle-ready-party-prompt"));
        assert!(manifest.contains("castle-talk-keyword-prompt"));
        assert!(manifest.contains("dungeon-search-direction-prompt"));
        assert!(manifest.contains("dungeon-open-direction-prompt"));
        assert!(manifest.contains("combat-play"));
        assert!(manifest.contains("combat-status-highlight"));
        assert!(manifest.contains("combat-attack-direction-prompt"));
        assert!(manifest.contains("combat-cast-party-prompt"));
        assert!(manifest.contains("combat-ready-party-prompt"));
        assert!(manifest.contains("combat-search-direction-prompt"));
        assert!(manifest.contains("surface-view-overlay"));
        assert!(manifest.contains("dungeon-view-overlay"));
        assert!(manifest.contains("britannia-chunk-map-overlay"));
        assert!(manifest.contains("peer-view-overlay"));
        assert!(manifest.contains("x-ray-view-overlay"));
        assert!(manifest.contains("surface-view-class-gallery"));
        assert!(manifest.contains("peer-view-class-gallery"));
        assert!(manifest.contains("x-ray-view-class-gallery"));
        assert!(manifest.contains("z-stats-modal"));
        assert!(manifest.contains("endgame-status"));
        assert!(manifest.contains("combat-arena-00"));
        assert!(manifest.contains("combat-arena-15"));
        assert!(manifest.contains("dungeon-combat-arena-000"));
        assert!(manifest.contains("dungeon-combat-arena-111"));
        assert!(manifest.contains("intro-menu\t320x200\tintro menu"));
        assert!(manifest.contains("intro-finished-menu\t320x200\tintro finished menu"));
        assert!(manifest.contains("intro-return-to-view\t320x200\tintro return-to-view"));
        assert!(manifest.contains("coverage\ttotal-frames\t"));
        assert!(manifest.contains("coverage\tcombat-outdoor-arena-gallery\t16/16"));
        assert!(manifest.contains("coverage\tcombat-dungeon-room-gallery\t112/112"));
        assert!(manifest.contains("coverage\tsurface-view-class-gallery\t3/3"));
        assert!(
            manifest.contains("combat-arena-00\t320x200\tcombat outdoor arena replacement gallery")
        );
        assert!(manifest.contains(
            "review=gallery/combat/outdoor source=BRIT.CBT arena=00 replacement_tile=0x"
        ));
        assert!(manifest.contains(
            "review=gallery/combat/dungeon-room source=DUNGEON.CBT arena=000 source_scan=disabled"
        ));
        assert!(manifest.contains(
            "surface-view-class-gallery\t320x200\tvisual surface view class gallery frame"
        ));
        assert!(manifest.contains("review=gallery/surface-view-class mode=surface-view"));
        assert!(manifest.contains("combat-marker-gallery"));
        assert!(manifest.contains("review=gallery/combat/markers"));
        assert!(manifest.contains("cursor=slot0 secondary=(3,4)"));
        assert!(manifest.contains("intro-menu"));
        assert!(manifest.contains("intro-finished-menu"));
        if has_story {
            assert!(manifest.contains("intro-story-art"));
        }
        assert!(manifest.contains("intro-return-to-view"));
        assert!(!manifest.contains("Avatar"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_route_suite_cases_cover_multi_step_play_routes() {
        let cases = visual_route_suite_cases();

        assert_eq!(cases.len(), 518);
        assert!(cases.iter().all(|case| {
            !case.script.is_empty()
                || matches!(
                    case.label,
                    "route-virtue-town-shadowlord-entry" | "route-stonegate-shadowlord-entry"
                )
        }));
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-world-movement")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-town-status-modal")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-town-view-overlay")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-world-view-overlay")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-view-overlay")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-peer-overlay")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-x-ray-overlay")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-britannia-look")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-britannia-spyglass-chunk-map")
        );
        for label in [
            "route-britannia-move-pass-idle",
            "route-castle-pass-and-idle",
            "route-castle-z-stats-modal",
            "route-britannia-view-overlay",
            "route-castle-view-overlay",
            "route-britannia-look-pass",
            "route-castle-look-pass",
            "route-britannia-utility-use-items",
            "route-ship-hms-cape-plans-use",
            "route-britannia-create-food-cast",
            "route-gate-travel-world-to-underworld",
            "route-reload-gate-travel-underworld-pass",
            "route-gate-travel-world-to-castle",
            "route-gate-travel-invalid-slot-refusal",
            "route-gate-travel-shipboard-refusal",
            "route-natural-moongate-trammel-gate-travel",
            "route-natural-moongate-empty-slot-clears-live-tile",
            "route-britannia-chasm-fall-to-underworld",
            "route-reload-chasm-underworld-pass",
            "route-britannia-whirlpool-forced-underworld",
            "route-britannia-fixed-narrative-gate-open-south-step",
            "route-britannia-fixed-narrative-gate-ordained-block",
            "route-britannia-hole-up-rest",
            "route-britannia-save-refusal",
            "route-britannia-dispatcher-refusals",
            "route-britannia-fixed-hidden-single-use-search-get",
            "route-underworld-pass-and-idle",
            "route-underworld-fixed-hidden-stack-search-get-search",
            "route-reload-underworld-fixed-hidden-stack-search-get-search",
            "route-blackthorn-fixed-hidden-zero-key-search",
            "route-minoc-fixed-hidden-daily-search-get-repeat",
            "route-reload-minoc-fixed-hidden-daily-search-get-repeat",
            "route-castle-wooden-box-use",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-save-refusal")
        );
        for label in [
            "route-castle-dispatcher-board-refusal",
            "route-castle-dispatcher-refusals",
            "route-castle-dispatcher-fire-refusal",
            "route-castle-command-workflow-overlays",
            "route-castle-mix-ready-order-route",
            "route-castle-party-overlay-routes",
            "route-castle-talk-ordinary-keyword-route",
            "route-castle-hourly-provision-poison-pass",
            "route-castle-hourly-poison-starvation-pass",
            "route-castle-hourly-ring-regeneration-pass",
            "route-castle-talk-status-sleeping-refusal",
            "route-castle-talk-status-praying-refusal",
            "route-castle-native-stair-up-route",
            "route-castle-native-stair-down-route",
            "route-castle-native-stair-cross-route",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        for label in [
            "route-talk-towne-reserved-name",
            "route-talk-towne-08-reserved-thank",
            "route-talk-dwelling-reserved-job",
            "route-talk-dwelling-16-reserved-bye",
            "route-talk-castle-reserved-thank",
            "route-talk-castle-24-reserved-work",
            "route-talk-keep-reserved-work",
            "route-talk-keep-32-reserved-name",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        for label in [
            "route-debug-enter-castle",
            "route-debug-enter-castle-return-world",
            "route-debug-enter-castle-from-underworld",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-world-board-horse")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-britannia-board-horse-route")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-reload-boarded-horse-pass")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-ship-xit-launches-skiff")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-reload-ship-xit-skiff-pass")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-ship-hoist-and-sail-east")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-ship-broadside-fire")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-ship-broadside-fire-route")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-movement-search")
        );
        for label in [
            "route-dungeon-search-focus-route",
            "route-dungeon-attack-direction-route",
            "route-dungeon-hole-up-rest",
            "route-dungeon-hole-up-no-direct-recovery",
            "route-dungeon-long-camp-recovery",
            "route-dungeon-ladder-down-up-route",
            "route-dungeon-surface-exit-return-world",
            "route-dungeon-active-monster-attack-ambush",
            "route-dungeon-active-monster-contact-ambush",
            "route-dungeon-exit-confirm",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-heavy-door-variant-pass-through")
        );
        for label in [
            "route-reload-dungeon-ladder-down-up",
            "route-reload-dungeon-ladder-down-up-route",
            "route-reload-dungeon-surface-exit-return-world",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-ignite-torch")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-ignite-torch-route")
        );
        for label in [
            "route-dungeon-turn-and-blocked-step",
            "route-dungeon-sjog-underfoot-routes",
            "route-dungeon-refusal-letter-routes",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        for label in [
            "route-dungeon-sjog-underfoot-get",
            "route-dungeon-sjog-underfoot-jimmy",
            "route-dungeon-sjog-underfoot-open",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-debug-enter-dungeon")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-exit-refusal")
        );
        for label in ["route-dungeon-refusal-board", "route-dungeon-refusal-fire"] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-sage-topic-miss")
        );
        for label in [
            "route-shop-arms-local-buy-sell",
            "route-shop-arms-iolos-bows-buy-first",
            "route-shop-arms-naughty-nomaans-buy-first",
            "route-shop-arms-arms-of-justice-buy-first",
            "route-shop-arms-darkwatch-armoury-buy-first",
            "route-shop-arms-paladins-protectorate-buy-first",
            "route-shop-arms-north-star-armoury-buy-first",
            "route-shop-arms-buccaneers-booty-buy-first",
            "route-shop-arms-shattered-shield-buy-first",
            "route-shop-arms-siege-crafters-buy-first",
            "route-shop-arms-iolos-bows-terminator-refusal",
            "route-shop-arms-naughty-nomaans-terminator-refusal",
            "route-shop-arms-arms-of-justice-terminator-refusal",
            "route-shop-arms-darkwatch-armoury-terminator-refusal",
            "route-shop-arms-paladins-protectorate-terminator-refusal",
            "route-shop-arms-north-star-armoury-terminator-refusal",
            "route-shop-arms-buccaneers-booty-terminator-refusal",
            "route-shop-arms-shattered-shield-terminator-refusal",
            "route-shop-arms-siege-crafters-terminator-refusal",
            "route-shop-healer-heal-decline",
            "route-shop-healer-heal-decline-route",
            "route-shop-healer-cure-accept",
            "route-shop-healer-heal-accept",
            "route-shop-healer-resurrect-accept",
            "route-shop-inn-rest-decline",
            "route-shop-inn-rest-decline-route",
            "route-shop-reagent-buy",
            "route-shop-reagent-buy-route",
            "route-shop-tavern-drink-and-food",
            "route-shop-tavern-drink-and-food-route",
            "route-shop-tavern-honest-meal-lore",
            "route-shop-tavern-wayfarer-lore",
            "route-shop-tavern-sword-and-keg-lore",
            "route-shop-tavern-slaughtered-lamb-lore",
            "route-shop-tavern-humble-palate-lore",
            "route-shop-tavern-blue-boar-lore",
            "route-shop-tavern-cats-lair-lore",
            "route-shop-tavern-fallen-virgin-lore",
            "route-shop-tavern-folley-tap-lore",
            "route-shop-horse-trader-decline",
            "route-shop-horse-trader-decline-route",
            "route-shop-horse-trader-no-marker-refusal",
            "route-shop-shipwright-quote-decline",
            "route-shop-shipwright-quote-decline-route",
            "route-shop-shipwright-frigate-buy",
            "route-shop-shipwright-island-frigate-buy",
            "route-shop-shipwright-crows-nest-skiff-buy",
            "route-shop-shipwright-oaken-oar-frigate-buy",
            "route-shop-shipwright-rusty-bucket-skiff-buy",
            "route-shop-guild-buy",
            "route-shop-guild-buy-route",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-britannia-blink-east-ray")
        );
        for label in [
            "route-britannia-locate-cast",
            "route-britannia-rel-hur-east",
            "route-castle-in-lor-spell",
            "route-castle-light-open-spell",
            "route-castle-light-open-spell-route",
            "route-castle-light-decay-route",
            "route-castle-restore-spell-suite",
            "route-castle-active-effect-spell-suite",
            "route-combat-directed-sleep-cone",
            "route-combat-directed-poison-wind-cone",
            "route-combat-directed-death-wind-cone",
            "route-combat-directed-flame-wind-cone",
            "route-combat-directed-sleep-cone-north",
            "route-combat-directed-sleep-cone-east",
            "route-combat-directed-sleep-cone-south",
            "route-combat-directed-sleep-cone-west",
            "route-combat-directed-poison-wind-cone-north",
            "route-combat-directed-poison-wind-cone-east",
            "route-combat-directed-poison-wind-cone-south",
            "route-combat-directed-poison-wind-cone-west",
            "route-combat-directed-death-wind-cone-north",
            "route-combat-directed-death-wind-cone-east",
            "route-combat-directed-death-wind-cone-south",
            "route-combat-directed-death-wind-cone-west",
            "route-combat-directed-flame-wind-cone-north",
            "route-combat-directed-flame-wind-cone-east",
            "route-combat-directed-flame-wind-cone-south",
            "route-combat-directed-flame-wind-cone-west",
            "route-combat-field-fire-marker",
            "route-combat-field-fire-marker-placement",
            "route-combat-field-poison-marker",
            "route-combat-field-poison-marker-placement",
            "route-combat-field-sleep-marker",
            "route-combat-field-sleep-marker-placement",
            "route-combat-field-energy-marker",
            "route-combat-field-energy-marker-placement",
            "route-combat-dispel-field-marker",
            "route-combat-field-dispel-fire-marker",
            "route-combat-field-dispel-empty-refusal",
            "route-combat-magic-missile-target",
            "route-combat-fireball-target",
            "route-combat-reveal-hidden-target",
            "route-combat-invisibility-caster",
            "route-combat-cause-fear-target",
            "route-combat-mass-charm-effect",
            "route-combat-tremor-targets",
            "route-combat-repel-undead-targets",
            "route-combat-charm-target",
            "route-combat-polymorph-target",
            "route-combat-clone-target",
            "route-combat-conjure-animal",
            "route-combat-swarm-summon",
            "route-combat-summon-daemon-ring",
            "route-combat-kill-gazer-eye-burst",
            "route-combat-kill-gargoyle-lava-marker",
            "route-combat-kill-shadowlord-vanish-marker",
            "route-terrain-combat-party-entry",
            "route-terrain-combat-xit-no-foes-clean-exit",
            "route-terrain-combat-out-of-arena-leave",
            "route-dungeon-room-party-entry",
            "route-dungeon-level-up-down-spells",
            "route-dungeon-field-cycle-spells",
            "route-dungeon-open-chest-spell",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-poison-gas-step")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-inn-rest-accept")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-inn-rest-accept-public-rate")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-horse-trader-horse-and-rider-buy")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-reload-horse-trader-horse-and-rider-buy-pass")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-horse-trader-stablehouse-buy")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-horse-trader-wishing-well-buy")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-sage-topic-paid-success")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-sage-topic-paid-success-route")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-sage-topic-short-funds")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-sage-topic-short-funds-route")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-fountain-look")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-surface-fountain-look")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-yew-wanted-poster-look")
        );
        for label in [
            "route-castle-town-attack-death-mask-npc",
            "route-castle-town-attack-guard-alarm",
            "route-castle-town-hostile-adjacent-alarm",
            "route-castle-town-guard-arrest-refusal",
            "route-castle-town-guard-arrest-surrender-yew",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-buccaneers-den-wishing-well")
        );
        for label in [
            "route-buccaneers-den-wishing-well-horse",
            "route-buccaneers-den-wishing-well-ferrari-grants-horse",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-death-vision-look")
        );
        for label in [
            "route-blackthorn-audience-correct",
            "route-blackthorn-audience-wrong",
            "route-blackthorn-rescue-refuge",
            "route-virtue-town-shadowlord-entry",
            "route-virtue-town-shadowlord-yell",
            "route-lycaeum-shard-falsehood-vanquish",
            "route-empath-shard-hatred-vanquish",
            "route-serpents-hold-shard-cowardice-vanquish",
            "route-stonegate-shadowlord-entry",
            "route-britannia-word-of-power-seal-opens",
            "route-underworld-doom-word-of-power-seal-opens",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-endgame-missing-box-terminal-jitter")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-endgame-missing-box-confirmation")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-endgame-box-victory-confirmation")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-endgame-box-full-victory-cinematic")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-trigger")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-room-combat-trigger")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-pass")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-pass-round")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-attack")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-attack-direction")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-board-refusal")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-z-stats")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-search-prompt")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-cast-refusal")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-ready-prompt")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-yell-word")
        );
        for label in [
            "route-britannia-extended-exploration",
            "route-castle-extended-walk-and-save",
            "route-castle-extended-walk-and-rest",
            "route-dungeon-extended-turn-and-search",
            "route-doom-combat-multi-round-pass",
            "route-endgame-class-tableau-restoration",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        for row in 1..=published_world_location_entries().len() {
            let label = format!("route-stock-location-enter-{row:02}");
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        for label in [
            "route-shrine-native-honesty-meditation",
            "route-shrine-native-humility-meditation",
            "route-codex-urn-honesty-read",
            "route-shrine-honesty-codex-turn-in",
            "route-shrine-compassion-completed-offering",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert_eq!(
            visual_route_step_label("route-world-movement", 2, "."),
            "route-world-movement-02-idle"
        );
        assert_eq!(
            visual_route_step_label("route-dungeon-movement-search", 3, "S6"),
            "route-dungeon-movement-search-03-s6"
        );
        assert_eq!(
            visual_route_step_label("route-doom-combat-trigger", 1, ""),
            "route-doom-combat-trigger-01-empty"
        );
        assert_eq!(
            visual_route_step_label("route-britannia-blink-east-ray", 1, "C1IP6"),
            "route-britannia-blink-east-ray-01-c1ip6"
        );
        assert_eq!(
            visual_route_step_label("route-britannia-locate-cast", 1, "C1IW"),
            "route-britannia-locate-cast-01-c1iw"
        );
        assert_eq!(
            visual_route_step_label(
                "route-britannia-whirlpool-forced-underworld",
                1,
                "setup:whirlpool-engagement"
            ),
            "route-britannia-whirlpool-forced-underworld-01-setup_whirlpool-engagement"
        );
        assert_eq!(
            visual_route_step_label("route-dungeon-field-cycle-spells", 8, "C1AG6"),
            "route-dungeon-field-cycle-spells-08-c1ag6"
        );
        assert_eq!(
            visual_route_step_label("route-combat-directed-death-wind-cone", 1, "C1CGIV6"),
            "route-combat-directed-death-wind-cone-01-c1cgiv6"
        );
        assert_eq!(
            visual_route_step_label("route-shop-horse-trader-stablehouse-buy", 2, "Y"),
            "route-shop-horse-trader-stablehouse-buy-02-y"
        );
        assert_eq!(
            visual_route_step_label("route-reload-dungeon-ladder-down-up", 2, "<"),
            "route-reload-dungeon-ladder-down-up-02-_"
        );
        assert_eq!(
            visual_route_step_label("route-shop-shipwright-crows-nest-skiff-buy", 2, "Y"),
            "route-shop-shipwright-crows-nest-skiff-buy-02-y"
        );
        assert_eq!(
            visual_route_step_label("route-endgame-missing-box-terminal-jitter", 3, ""),
            "route-endgame-missing-box-terminal-jitter-03-empty"
        );
        assert_eq!(
            visual_route_step_label("route-britannia-extended-exploration", 10, "empty"),
            "route-britannia-extended-exploration-10-empty"
        );
    }

    #[test]
    fn visual_route_suite_scripts_cover_every_published_spell_code() {
        let cases = visual_route_suite_cases();
        let mut covered = [false; u5_runtime::SPELL_COUNT];

        for command in cases.iter().flat_map(|case| case.script.iter().copied()) {
            let Some(suffix) = command.strip_prefix('C') else {
                continue;
            };
            let code = u5_runtime::inline_parsers::inline_spell_code(suffix);
            if let Some(index) = spell_index_from_code(&code) {
                covered[index] = true;
            }
        }

        for (index, code) in u5_runtime::SPELL_CODES.iter().enumerate() {
            assert!(covered[index], "visual route scripts should cover {code}");
        }
    }

    #[test]
    fn visual_key_route_suite_cases_cover_real_keyboard_prompts() {
        let cases = visual_key_route_suite_cases();

        assert_eq!(cases.len(), 16);
        for label in [
            "route-key-world-movement-pass-music",
            "route-key-save-refusal",
            "route-key-talk-keyword-buffer",
            "route-key-shrine-mantra-buffer",
            "route-key-shop-quantity-buffer",
            "route-key-prompt-escape-cancel",
            "route-key-world-direction-prompts",
            "route-key-yell-buffer",
            "route-key-ready-picker",
            "route-key-z-stats-picker",
            "route-key-use-picker",
            "route-key-mix-prompt",
            "route-key-rest-watch-prompt",
            "route-key-new-order-picker",
            "route-key-dungeon-controls",
            "route-key-directional-keys",
        ] {
            assert!(cases.iter().any(|case| case.label == label), "{label}");
        }
        assert!(
            cases
                .iter()
                .any(|case| case.steps.iter().any(|step| step.control))
        );
        assert!(
            cases
                .iter()
                .any(|case| case.steps.iter().any(|step| step.key == KeyCode::Backspace))
        );
        assert!(
            cases
                .iter()
                .any(|case| case.steps.iter().any(|step| step.key == KeyCode::Escape))
        );
        assert!(cases.iter().any(|case| {
            case.frame_kind == "visual key route dungeon frame"
                && case.steps.iter().any(|step| step.key == KeyCode::KeyW)
        }));
        assert!(cases.iter().any(|case| {
            case.label == "route-key-directional-keys"
                && case
                    .steps
                    .iter()
                    .any(|step| step.key == KeyCode::ArrowRight)
                && case.steps.iter().any(|step| step.key == KeyCode::Numpad4)
        }));
    }

    #[test]
    fn visual_key_route_step_buffers_echo_before_submit() {
        let mut state = test_state(open_grid(), 1, 1);
        install_test_conversation(&mut state);
        let mut input_line = String::new();

        apply_visual_key_route_step(
            &mut state,
            &mut input_line,
            VisualKeyStep::key("key-j", KeyCode::KeyJ),
            Path::new(""),
        )
        .unwrap();
        apply_visual_key_route_step(
            &mut state,
            &mut input_line,
            VisualKeyStep::key("key-o", KeyCode::KeyO),
            Path::new(""),
        )
        .unwrap();
        apply_visual_key_route_step(
            &mut state,
            &mut input_line,
            VisualKeyStep::key("key-x", KeyCode::KeyX),
            Path::new(""),
        )
        .unwrap();
        apply_visual_key_route_step(
            &mut state,
            &mut input_line,
            VisualKeyStep::key("backspace", KeyCode::Backspace),
            Path::new(""),
        )
        .unwrap();

        assert_eq!(input_line, "jo");
        assert!(!state.message.contains("mend"));

        apply_visual_key_route_step(
            &mut state,
            &mut input_line,
            VisualKeyStep::key("key-b", KeyCode::KeyB),
            Path::new(""),
        )
        .unwrap();
        apply_visual_key_route_step(
            &mut state,
            &mut input_line,
            VisualKeyStep::key("enter", KeyCode::Enter),
            Path::new(""),
        )
        .unwrap();

        assert!(input_line.is_empty());
        assert!(state.message.contains("mend"));
    }

    #[test]
    fn visual_route_suite_local_clean_writes_per_step_pngs_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join("CASTLE.DAT").exists()
            || !game_dir.join(TILES_EGA_FILE).exists()
            || !game_dir.join(IBM_CH_FILE).exists()
            || !game_dir.join("ENDMSG.DAT").exists()
            || !game_dir.join("END.DAT").exists()
        {
            return;
        }

        let dir = temp_output_dir("routes");
        let reports = visual_route_suite(game_dir, TileGraphicsDepth::Ega16, &dir).unwrap();

        assert_eq!(reports.len(), 1780);
        for report in &reports {
            assert!(report.path.exists());
            assert_eq!(report.width, VISUAL_PLAY_FRAME_WIDTH);
            assert_eq!(report.height, VISUAL_PLAY_FRAME_HEIGHT);
            assert!(report.nonblack_pixels > 0);
        }
        let manifest = fs::read_to_string(dir.join("manifest.txt")).unwrap();
        assert!(manifest.contains("coverage\tvisual-route-steps\t1780"));
        assert!(manifest.contains("coverage\tvisual-key-route-steps\t89"));
        assert!(manifest.contains("coverage\tvisual-route-combat-steps\t"));
        assert!(manifest.contains("route-world-movement-01-d\t320x200\t"));
        assert!(manifest.contains("review=route-step route=route-world-movement step=01 input=d"));
        assert!(manifest.contains("route-key-world-movement-pass-music-03-ctrl_s"));
        assert!(manifest.contains("route-key-save-refusal-02-key_n"));
        assert!(manifest.contains("route-key-talk-keyword-buffer-04-backspace"));
        assert!(manifest.contains("route-key-talk-keyword-buffer-06-enter"));
        assert!(manifest.contains("route-key-shrine-mantra-buffer-05-enter"));
        assert!(manifest.contains("route-key-shop-quantity-buffer-06-enter"));
        assert!(manifest.contains("route-key-prompt-escape-cancel-04-escape"));
        assert!(manifest.contains("route-key-world-direction-prompts-12-key_d"));
        assert!(manifest.contains("route-key-yell-buffer-09-enter"));
        assert!(manifest.contains("route-key-ready-picker-05-space"));
        assert!(manifest.contains("route-key-z-stats-picker-02-space"));
        assert!(manifest.contains("route-key-use-picker-02-enter"));
        assert!(manifest.contains("route-key-mix-prompt-02-escape"));
        assert!(manifest.contains("route-key-rest-watch-prompt-04-digit_2"));
        assert!(manifest.contains("route-key-new-order-picker-03-digit_3"));
        assert!(
            manifest.contains(
                "review=route-step route=route-key-talk-keyword-buffer step=03 input=key_b"
            )
        );
        assert!(manifest.contains("route-world-movement-00-initial"));
        assert!(manifest.contains("route-world-movement-01-d"));
        assert!(manifest.contains("route-britannia-move-pass-idle-02-idle"));
        assert!(manifest.contains("route-castle-pass-and-idle-01-empty"));
        assert!(manifest.contains("route-town-status-modal-01-z"));
        assert!(manifest.contains("route-castle-z-stats-modal-01-z"));
        assert!(manifest.contains("route-town-view-overlay-01-v"));
        assert!(manifest.contains("route-town-view-overlay-02-idle"));
        assert!(manifest.contains("route-world-view-overlay-01-v"));
        assert!(manifest.contains("route-world-view-overlay-02-idle"));
        assert!(manifest.contains("route-britannia-view-overlay-02-idle"));
        assert!(manifest.contains("route-castle-view-overlay-02-idle"));
        assert!(manifest.contains("route-dungeon-view-overlay-01-v"));
        assert!(manifest.contains("route-dungeon-view-overlay-02-idle"));
        assert!(manifest.contains("route-castle-peer-overlay-01-c1iqw"));
        assert!(manifest.contains("route-castle-peer-overlay-02-idle"));
        assert!(manifest.contains("route-castle-x-ray-overlay-01-c1awy"));
        assert!(manifest.contains("route-castle-x-ray-overlay-02-idle"));
        assert!(manifest.contains("route-britannia-look-01-l6"));
        assert!(manifest.contains("route-britannia-look-pass-01-l6"));
        assert!(manifest.contains("route-castle-look-pass-01-l6"));
        assert!(manifest.contains("route-britannia-spyglass-chunk-map-01-usp"));
        assert!(manifest.contains("route-britannia-utility-use-items-03-uc"));
        assert!(manifest.contains("route-ship-hms-cape-plans-use-01-up"));
        assert!(manifest.contains("route-britannia-create-food-cast-01-c1imx"));
        assert!(manifest.contains("route-gate-travel-world-to-underworld-01-c1prv1"));
        assert!(manifest.contains("route-reload-gate-travel-underworld-pass-02-empty"));
        assert!(manifest.contains("route-gate-travel-world-to-castle-01-c1prv2"));
        assert!(manifest.contains("route-gate-travel-invalid-slot-refusal-01-c1prv4"));
        assert!(manifest.contains("route-stock-location-enter-01-01-e"));
        assert!(manifest.contains("route-stock-location-enter-40-01-e"));
        assert!(manifest.contains("route-shrine-native-honesty-meditation-01-mahm"));
        assert!(manifest.contains("route-shrine-native-humility-meditation-01-mlum"));
        assert!(manifest.contains("route-codex-urn-honesty-read-01-m"));
        assert!(manifest.contains("route-shrine-honesty-codex-turn-in-01-mahm"));
        assert!(manifest.contains("route-shrine-compassion-completed-offering-01-mmu_1"));
        assert!(manifest.contains("route-gate-travel-shipboard-refusal-01-c1prv2"));
        assert!(manifest.contains("route-natural-moongate-trammel-gate-travel-01-idle_1"));
        assert!(manifest.contains("route-natural-moongate-empty-slot-clears-live-tile-01-idle_1"));
        assert!(manifest.contains("route-britannia-chasm-fall-to-underworld-01-s"));
        assert!(manifest.contains("route-reload-chasm-underworld-pass-02-empty"));
        assert!(
            manifest.contains(
                "route-britannia-whirlpool-forced-underworld-01-setup_whirlpool-engagement"
            )
        );
        assert!(manifest.contains("route-britannia-fixed-narrative-gate-open-south-step-01-empty"));
        assert!(manifest.contains("route-britannia-fixed-narrative-gate-ordained-block-01-empty"));
        assert!(manifest.contains("route-britannia-hole-up-rest-01-h1"));
        assert!(manifest.contains("route-britannia-save-refusal-02-n"));
        assert!(manifest.contains("route-britannia-dispatcher-refusals-01-b"));
        assert!(manifest.contains("route-britannia-fixed-hidden-single-use-search-get-02-g6"));
        assert!(manifest.contains("route-underworld-pass-and-idle-02-idle_1"));
        assert!(manifest.contains("route-underworld-fixed-hidden-stack-search-get-search-03-s6"));
        assert!(
            manifest.contains("route-reload-underworld-fixed-hidden-stack-search-get-search-03-s6")
        );
        assert!(manifest.contains("route-blackthorn-fixed-hidden-zero-key-search-01-s6"));
        assert!(manifest.contains("route-minoc-fixed-hidden-daily-search-get-repeat-03-s6"));
        assert!(manifest.contains("route-reload-minoc-fixed-hidden-daily-search-get-repeat-03-s6"));
        assert!(manifest.contains("route-castle-wooden-box-use-01-ub"));
        assert!(manifest.contains("route-castle-save-refusal-02-n"));
        assert!(manifest.contains("route-castle-dispatcher-board-refusal-01-b"));
        assert!(manifest.contains("route-castle-dispatcher-refusals-01-b"));
        assert!(manifest.contains("route-castle-dispatcher-fire-refusal-01-f6"));
        assert!(manifest.contains("route-castle-command-workflow-overlays-04-n23"));
        assert!(manifest.contains("route-castle-mix-ready-order-route-04-n23"));
        assert!(manifest.contains("route-castle-party-overlay-routes-04-r"));
        assert!(manifest.contains("route-castle-hourly-provision-poison-pass-01-empty"));
        assert!(manifest.contains("route-castle-hourly-poison-starvation-pass-01-empty"));
        assert!(manifest.contains("route-castle-hourly-ring-regeneration-pass-01-empty"));
        assert!(manifest.contains("route-castle-talk-status-sleeping-refusal-01-t6"));
        assert!(manifest.contains("route-castle-talk-status-praying-refusal-01-t6"));
        assert!(manifest.contains("route-castle-talk-ordinary-keyword-route-03-name"));
        assert!(manifest.contains("route-talk-towne-reserved-name-03-name"));
        assert!(manifest.contains("route-talk-towne-08-reserved-thank-03-thank"));
        assert!(manifest.contains("route-talk-dwelling-reserved-job-03-job"));
        assert!(manifest.contains("route-talk-dwelling-16-reserved-bye-03-bye"));
        assert!(manifest.contains("route-talk-castle-reserved-thank-03-thank"));
        assert!(manifest.contains("route-talk-castle-24-reserved-work-03-work"));
        assert!(manifest.contains("route-talk-keep-reserved-work-03-work"));
        assert!(manifest.contains("route-talk-keep-32-reserved-name-03-name"));
        assert!(manifest.contains("route-castle-native-stair-up-route-01-d"));
        assert!(manifest.contains("route-castle-native-stair-down-route-01-d"));
        assert!(manifest.contains("route-castle-native-stair-cross-route-01-w"));
        assert!(manifest.contains("route-debug-enter-castle-03-idle_1"));
        assert!(manifest.contains("route-debug-enter-castle-return-world-02-w"));
        assert!(manifest.contains("route-debug-enter-castle-from-underworld-02-empty"));
        assert!(manifest.contains("route-world-board-horse-01-b"));
        assert!(manifest.contains("route-britannia-board-horse-route-01-b"));
        assert!(manifest.contains("route-reload-boarded-horse-pass-02-empty"));
        assert!(manifest.contains("route-ship-xit-launches-skiff-01-x"));
        assert!(manifest.contains("route-reload-ship-xit-skiff-pass-02-empty"));
        assert!(manifest.contains("route-ship-hoist-and-sail-east-02-d"));
        assert!(manifest.contains("route-ship-broadside-fire-01-f6"));
        assert!(manifest.contains("route-ship-broadside-fire-route-01-f6"));
        assert!(manifest.contains("route-dungeon-movement-search-03-s6"));
        assert!(manifest.contains("route-dungeon-search-focus-route-01-s6"));
        assert!(manifest.contains("route-dungeon-attack-direction-route-02-6"));
        assert!(manifest.contains("route-dungeon-hole-up-rest-01-h1"));
        assert!(manifest.contains("route-dungeon-hole-up-no-direct-recovery-01-h1"));
        assert!(manifest.contains("route-dungeon-long-camp-recovery-01-h6_4"));
        assert!(manifest.contains("route-dungeon-heavy-door-variant-pass-through-01-idle"));
        assert!(manifest.contains("route-dungeon-ladder-down-up-route-02-_"));
        assert!(manifest.contains(&visual_route_step_label(
            "route-reload-dungeon-ladder-down-up",
            2,
            "<"
        )));
        assert!(manifest.contains(&visual_route_step_label(
            "route-reload-dungeon-ladder-down-up-route",
            2,
            "<"
        )));
        assert!(manifest.contains("route-dungeon-surface-exit-return-world-01-k"));
        assert!(manifest.contains("route-reload-dungeon-surface-exit-return-world-02-empty"));
        assert!(manifest.contains("route-dungeon-active-monster-attack-ambush-01-a"));
        assert!(manifest.contains("route-dungeon-active-monster-contact-ambush-01-empty"));
        assert!(manifest.contains("route-dungeon-ignite-torch-01-i"));
        assert!(manifest.contains("route-dungeon-ignite-torch-route-01-i"));
        assert!(manifest.contains("route-dungeon-turn-and-blocked-step-04-s"));
        assert!(manifest.contains("route-dungeon-sjog-underfoot-get-01-g"));
        assert!(manifest.contains("route-dungeon-sjog-underfoot-jimmy-01-j"));
        assert!(manifest.contains("route-dungeon-sjog-underfoot-open-01-o"));
        assert!(manifest.contains("route-dungeon-sjog-underfoot-routes-02-j"));
        assert!(manifest.contains("route-dungeon-refusal-letter-routes-01-b"));
        assert!(manifest.contains("route-debug-enter-dungeon-03-n"));
        assert!(manifest.contains("route-dungeon-exit-refusal-02-n"));
        assert!(manifest.contains("route-dungeon-exit-confirm-02-y"));
        assert!(manifest.contains("route-dungeon-refusal-board-01-b"));
        assert!(manifest.contains("route-dungeon-refusal-fire-01-f"));
        assert!(manifest.contains("route-shop-arms-local-buy-sell-06-n"));
        assert!(manifest.contains("route-shop-arms-local-buy-sell-route-06-n"));
        assert!(manifest.contains("route-shop-arms-iolos-bows-terminator-refusal-03-_"));
        assert!(manifest.contains("route-shop-arms-siege-crafters-terminator-refusal-03-_"));
        assert!(manifest.contains("route-shop-healer-heal-decline-04-n"));
        assert!(manifest.contains("route-shop-healer-heal-decline-route-04-n"));
        assert!(manifest.contains("route-shop-healer-cure-accept-04-y"));
        assert!(manifest.contains("route-shop-healer-heal-accept-04-y"));
        assert!(manifest.contains("route-shop-healer-resurrect-accept-04-y"));
        assert!(manifest.contains("route-shop-inn-rest-decline-03-p"));
        assert!(manifest.contains("route-shop-inn-rest-decline-route-03-p"));
        assert!(manifest.contains("route-shop-reagent-buy-03-n"));
        assert!(manifest.contains("route-shop-reagent-buy-route-03-n"));
        assert!(manifest.contains("route-shop-tavern-drink-and-food-05-n"));
        assert!(manifest.contains("route-shop-tavern-drink-and-food-route-05-n"));
        assert!(manifest.contains("route-shop-tavern-honest-meal-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-wayfarer-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-sword-and-keg-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-slaughtered-lamb-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-humble-palate-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-blue-boar-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-cats-lair-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-fallen-virgin-lore-05-y"));
        assert!(manifest.contains("route-shop-tavern-folley-tap-lore-05-y"));
        assert!(manifest.contains("route-shop-horse-trader-decline-02-n"));
        assert!(manifest.contains("route-shop-horse-trader-decline-route-02-n"));
        assert!(manifest.contains("route-shop-horse-trader-no-marker-refusal-02-y"));
        assert!(manifest.contains("route-shop-shipwright-quote-decline-02-n"));
        assert!(manifest.contains("route-shop-shipwright-quote-decline-route-02-n"));
        assert!(manifest.contains("route-shop-shipwright-frigate-buy-02-y"));
        assert!(manifest.contains("route-shop-shipwright-island-frigate-buy-02-y"));
        assert!(manifest.contains("route-shop-shipwright-crows-nest-skiff-buy-02-y"));
        assert!(manifest.contains("route-shop-shipwright-oaken-oar-frigate-buy-02-y"));
        assert!(manifest.contains("route-shop-shipwright-rusty-bucket-skiff-buy-02-y"));
        assert!(manifest.contains("route-shop-guild-buy-03-d"));
        assert!(manifest.contains("route-shop-guild-buy-route-03-d"));
        assert!(manifest.contains("route-shop-sage-topic-miss-01-mantra"));
        assert!(manifest.contains("route-shop-sage-topic-miss-route-01-mantra"));
        assert!(manifest.contains("route-britannia-blink-east-ray-01-c1ip6"));
        assert!(manifest.contains("route-britannia-locate-cast-01-c1iw"));
        assert!(manifest.contains("route-britannia-rel-hur-east-01-c1hr6"));
        assert!(manifest.contains("route-castle-in-lor-spell-01-c1il"));
        assert!(manifest.contains("route-castle-light-open-spell-02-c1as6"));
        assert!(manifest.contains("route-castle-light-open-spell-route-02-c1as6"));
        assert!(manifest.contains("route-castle-light-decay-route-02-empty"));
        assert!(manifest.contains("route-castle-restore-spell-suite-05-c1cim4"));
        assert!(manifest.contains("route-castle-active-effect-spell-suite-04-c1at"));
        assert!(manifest.contains("route-combat-directed-sleep-cone-01-c1iz6"));
        assert!(manifest.contains("route-combat-directed-poison-wind-cone-01-c1hin6"));
        assert!(manifest.contains("route-combat-directed-death-wind-cone-01-c1cgiv6"));
        assert!(manifest.contains("route-combat-directed-flame-wind-cone-01-c1fhi6"));
        assert!(manifest.contains("route-combat-directed-sleep-cone-north-01-c1iz8"));
        assert!(manifest.contains("route-combat-directed-poison-wind-cone-west-01-c1hin4"));
        assert!(manifest.contains("route-combat-directed-death-wind-cone-south-01-c1cgiv2"));
        assert!(manifest.contains("route-combat-directed-flame-wind-cone-east-01-c1fhi6"));
        assert!(manifest.contains("route-combat-field-fire-marker-01-c1fgi6"));
        assert!(manifest.contains("route-combat-field-fire-marker-placement-01-c1fgi6"));
        assert!(manifest.contains("route-combat-field-poison-marker-01-c1gin6"));
        assert!(manifest.contains("route-combat-field-poison-marker-placement-01-c1gin6"));
        assert!(manifest.contains("route-combat-field-sleep-marker-01-c1giz6"));
        assert!(manifest.contains("route-combat-field-sleep-marker-placement-01-c1giz6"));
        assert!(manifest.contains("route-combat-field-energy-marker-01-c1gis6"));
        assert!(manifest.contains("route-combat-field-energy-marker-placement-01-c1gis6"));
        assert!(manifest.contains("route-combat-dispel-field-marker-01-c1ag6"));
        assert!(manifest.contains("route-combat-field-dispel-fire-marker-01-c1ag6"));
        assert!(manifest.contains("route-combat-field-dispel-empty-refusal-01-c1ag6"));
        assert!(manifest.contains("route-combat-magic-missile-target-01-c1gp7"));
        assert!(manifest.contains("route-combat-fireball-target-01-c1fv7"));
        assert!(manifest.contains("route-combat-reveal-hidden-target-01-c1qw"));
        assert!(manifest.contains("route-combat-invisibility-caster-01-c1ls"));
        assert!(manifest.contains("route-combat-cause-fear-target-01-c1ciq"));
        assert!(manifest.contains("route-combat-mass-charm-effect-01-c1aqw"));
        assert!(manifest.contains("route-combat-tremor-targets-01-c1ipvy"));
        assert!(manifest.contains("route-combat-repel-undead-targets-01-c1acx"));
        assert!(manifest.contains("route-combat-charm-target-01-c1aex7"));
        assert!(manifest.contains("route-combat-polymorph-target-01-c1brx7"));
        assert!(manifest.contains("route-combat-clone-target-01-c1iqx7"));
        assert!(manifest.contains("route-combat-conjure-animal-01-c1kx"));
        assert!(manifest.contains("route-combat-swarm-summon-01-c1bix"));
        assert!(manifest.contains("route-combat-summon-daemon-ring-01-c1ckx6"));
        assert!(manifest.contains("route-combat-kill-gazer-eye-burst-01-c1cx7"));
        assert!(manifest.contains("route-combat-kill-gargoyle-lava-marker-01-c1cx7"));
        assert!(manifest.contains("route-combat-kill-shadowlord-vanish-marker-01-c1cx7"));
        assert!(
            manifest
                .contains("route-terrain-combat-party-entry-01-setup_terrain-combat-party-entry")
        );
        assert!(manifest.contains("route-terrain-combat-xit-no-foes-clean-exit-02-x"));
        assert!(manifest.contains("route-terrain-combat-out-of-arena-leave-02-d"));
        assert!(
            manifest.contains("route-dungeon-room-party-entry-01-setup_dungeon-room-party-entry")
        );
        assert!(manifest.contains("route-dungeon-level-up-down-spells-02-c1dp"));
        assert!(manifest.contains("route-dungeon-field-cycle-spells-08-c1ag6"));
        assert!(manifest.contains("route-dungeon-open-chest-spell-01-c1as"));
        assert!(manifest.contains("route-castle-poison-gas-step-01-d"));
        assert!(manifest.contains("route-shop-inn-rest-accept-02-y"));
        assert!(manifest.contains("route-shop-inn-rest-accept-public-rate-02-y"));
        assert!(manifest.contains("route-shop-horse-trader-horse-and-rider-buy-02-y"));
        assert!(manifest.contains("route-reload-horse-trader-horse-and-rider-buy-pass-03-empty"));
        assert!(manifest.contains("route-shop-horse-trader-stablehouse-buy-02-y"));
        assert!(manifest.contains("route-shop-horse-trader-wishing-well-buy-02-y"));
        assert!(manifest.contains("route-shop-sage-topic-paid-success-02-y"));
        assert!(manifest.contains("route-shop-sage-topic-short-funds-02-y"));
        assert!(manifest.contains("route-shop-sage-topic-paid-success-route-02-y"));
        assert!(manifest.contains("route-shop-sage-topic-short-funds-route-02-y"));
        assert!(manifest.contains("route-castle-fountain-look-02-1"));
        assert!(manifest.contains("route-castle-surface-fountain-look-02-1"));
        assert!(manifest.contains("route-yew-wanted-poster-look-01-l6"));
        assert!(manifest.contains("route-castle-town-attack-death-mask-npc-01-a6"));
        assert!(manifest.contains("route-castle-town-attack-guard-alarm-01-a6"));
        assert!(manifest.contains("route-castle-town-hostile-adjacent-alarm-01-empty"));
        assert!(manifest.contains("route-castle-town-guard-arrest-refusal-02-n"));
        assert!(manifest.contains("route-castle-town-guard-arrest-surrender-yew-02-y"));
        assert!(manifest.contains("route-buccaneers-den-wishing-well-03-horse"));
        assert!(manifest.contains("route-buccaneers-den-wishing-well-horse-03-horse"));
        assert!(
            manifest.contains("route-buccaneers-den-wishing-well-ferrari-grants-horse-03-ferrari")
        );
        assert!(manifest.contains("route-castle-death-vision-look-02-1"));
        assert!(manifest.contains("route-blackthorn-audience-correct-02-ahm"));
        assert!(manifest.contains("route-blackthorn-audience-wrong-02-wrong"));
        assert!(manifest.contains("route-blackthorn-rescue-refuge-02-empty"));
        assert!(manifest.contains("route-virtue-town-shadowlord-entry-00-initial"));
        assert!(manifest.contains("route-virtue-town-shadowlord-yell-01-yfaulinei"));
        assert!(manifest.contains("route-lycaeum-shard-falsehood-vanquish-01-uf"));
        assert!(manifest.contains("route-empath-shard-hatred-vanquish-01-uh"));
        assert!(manifest.contains("route-serpents-hold-shard-cowardice-vanquish-01-ucw"));
        assert!(manifest.contains("route-stonegate-shadowlord-entry-00-initial"));
        assert!(manifest.contains("route-britannia-word-of-power-seal-opens-01-yfallax"));
        assert!(manifest.contains("route-underworld-doom-word-of-power-seal-opens-01-yveramocor"));
        assert!(manifest.contains("route-endgame-missing-box-terminal-jitter-03-empty"));
        assert!(manifest.contains("route-endgame-missing-box-confirmation-02-y"));
        assert!(manifest.contains("route-endgame-box-victory-confirmation-02-y"));
        assert!(manifest.contains("route-endgame-box-full-victory-cinematic-18-empty"));
        assert!(manifest.contains("route-doom-combat-trigger-01-empty"));
        assert!(manifest.contains("route-doom-room-combat-trigger-01-empty"));
        assert!(manifest.contains("route-doom-combat-pass-02-empty"));
        assert!(manifest.contains("route-doom-combat-pass-round-02-empty"));
        assert!(manifest.contains("route-doom-combat-attack-02-a6"));
        assert!(manifest.contains("route-doom-combat-attack-direction-02-a6"));
        assert!(manifest.contains("route-doom-combat-board-refusal-02-b"));
        assert!(manifest.contains("route-doom-combat-z-stats-02-z"));
        assert!(manifest.contains("route-doom-combat-search-prompt-02-s"));
        assert!(manifest.contains("route-doom-combat-select-clear-02-0"));
        assert!(manifest.contains("route-doom-combat-select-player-clear-02-0"));
        assert!(manifest.contains("route-doom-combat-select-player-one-02-1"));
        assert!(manifest.contains("route-doom-combat-select-player-six-02-6"));
        assert!(manifest.contains("route-doom-combat-escape-abort-02-_"));
        assert!(manifest.contains("route-doom-combat-music-toggle-02-_"));
        assert!(manifest.contains("route-doom-combat-step-east-02-d"));
        assert!(manifest.contains("route-doom-combat-direct-step-east-02-d"));
        assert!(manifest.contains("route-doom-combat-d-refusal-02-d"));
        assert!(manifest.contains("route-doom-combat-w-refusal-02-w"));
        assert!(manifest.contains("route-doom-combat-view-label-only-02-v"));
        assert!(manifest.contains("route-doom-combat-look-label-only-02-l"));
        assert!(manifest.contains("route-doom-combat-cast-refusal-02-c1il"));
        assert!(manifest.contains("route-doom-combat-get-direction-02-g6"));
        assert!(manifest.contains("route-doom-combat-push-direction-02-p6"));
        assert!(manifest.contains("route-doom-combat-ready-prompt-02-r"));
        assert!(manifest.contains("route-doom-combat-yell-word-02-yfallax"));
        assert!(manifest.contains("route-doom-combat-xit-foes-remain-02-x"));
        assert!(manifest.contains("route-doom-combat-quit-defeat-02-q"));
        assert!(manifest.contains("route-endgame-class-tableau-restoration-01-y"));
        assert!(manifest.contains("route-britannia-extended-exploration-12-empty"));
        assert!(manifest.contains("route-castle-extended-walk-and-save-09-z"));
        assert!(manifest.contains("route-castle-extended-walk-and-rest-01-s"));
        assert!(manifest.contains("route-dungeon-extended-turn-and-search-09-s6"));
        assert!(manifest.contains("route-doom-combat-multi-round-pass-05-empty"));
        assert!(!manifest.contains("Avatar"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_key_map_emits_spec_input_bytes_for_commands_and_movement() {
        assert_eq!(key_code_to_char(KeyCode::KeyW, false, false), Some('W'));
        assert_eq!(key_code_to_char(KeyCode::KeyA, false, false), Some('A'));
        assert_eq!(key_code_to_char(KeyCode::KeyS, false, false), Some('S'));
        assert_eq!(key_code_to_char(KeyCode::KeyD, false, false), Some('D'));
        assert_eq!(key_code_to_char(KeyCode::KeyA, true, false), Some('A'));
        assert_eq!(key_code_to_char(KeyCode::KeyS, true, false), Some('S'));
        assert_eq!(
            key_code_to_char(KeyCode::KeyS, false, true),
            Some(PLAY_MUSIC_TOGGLE_KEY)
        );
        assert_eq!(key_code_to_char(KeyCode::KeyA, false, true), None);
        assert_eq!(key_code_to_char(KeyCode::KeyQ, false, false), Some('Q'));
        assert_eq!(key_code_to_char(KeyCode::KeyU, false, false), Some('U'));
        assert_eq!(key_code_to_char(KeyCode::Digit2, false, false), Some('2'));
        assert_eq!(
            key_code_to_input_byte(KeyCode::ArrowUp, false, false),
            Some(u5_runtime::INPUT_CODE_NORTH)
        );
        assert_eq!(
            key_code_to_input_byte(KeyCode::Numpad4, false, false),
            Some(u5_runtime::INPUT_CODE_WEST)
        );
        assert_eq!(
            key_code_to_input_byte(KeyCode::Home, false, false),
            Some(u5_runtime::INPUT_CODE_NORTHWEST)
        );
        assert_eq!(
            key_code_to_input_byte(KeyCode::PageDown, false, false),
            Some(u5_runtime::INPUT_CODE_SOUTHEAST)
        );
        assert_eq!(
            key_code_to_input_byte(KeyCode::Digit8, true, false),
            Some(u5_runtime::INPUT_CODE_NORTH)
        );
        assert_eq!(
            key_code_to_input_byte(KeyCode::Digit1, true, false),
            Some(u5_runtime::INPUT_CODE_SOUTHWEST)
        );
        assert_eq!(
            key_code_to_input_byte(KeyCode::F1, false, false),
            Some(u5_runtime::INPUT_CODE_F1)
        );
        assert_eq!(
            key_code_to_input_byte(KeyCode::F10, false, false),
            Some(u5_runtime::INPUT_CODE_F10)
        );
    }

    #[test]
    fn visual_key_map_emits_modal_prompt_controls() {
        assert_eq!(key_code_to_char(KeyCode::Enter, false, false), Some('\r'));
        assert_eq!(
            key_code_to_char(KeyCode::NumpadEnter, false, false),
            Some('\r')
        );
        assert_eq!(
            key_code_to_char(KeyCode::Backspace, false, false),
            Some('\u{8}')
        );
        assert_eq!(
            key_code_to_char(KeyCode::NumpadBackspace, false, false),
            Some('\u{8}')
        );
        assert_eq!(
            key_code_to_char(KeyCode::Escape, false, false),
            Some('\u{1b}')
        );
        assert_eq!(
            key_code_to_char(KeyCode::BracketLeft, false, false),
            Some('[')
        );
        assert_eq!(
            key_code_to_char(KeyCode::BracketRight, true, false),
            Some('}')
        );
        assert_eq!(key_code_to_char(KeyCode::Equal, true, false), Some('+'));
        assert_eq!(key_code_to_char(KeyCode::Minus, false, false), Some('-'));
        assert_eq!(
            key_code_to_char(KeyCode::NumpadAdd, false, false),
            Some('+')
        );
        assert_eq!(
            key_code_to_char(KeyCode::NumpadSubtract, true, false),
            Some('-')
        );
    }

    #[test]
    fn visual_escape_quits_only_when_no_gameplay_prompt_is_active() {
        let mut state = test_state(open_grid(), 1, 1);
        assert!(should_escape_quit_visual(&state));

        state.start_cast_spell_prompt();
        assert!(state.active_cast.is_some());
        assert!(!should_escape_quit_visual(&state));

        let mut chest = test_state(open_grid(), 1, 1);
        chest.start_surface_object_chest_prompt(2, 1, SurfaceChestVerb::Open);
        assert!(chest.active_surface_chest.is_some());
        assert!(!should_escape_quit_visual(&chest));
    }

    #[test]
    fn visual_idle_tick_advances_runtime_wait_tick_without_game_time() {
        let mut state = world_state(open_world_grid(), 10, 20);
        state.message = "Ready.".to_string();
        let clock_before = state.clock;

        assert!(visual_idle_tick(&mut state));

        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, clock_before);
        assert_eq!(state.animation.frame, 1);
        assert_eq!(state.message, "Ready.");
    }

    #[test]
    fn visual_idle_tick_suppresses_world_tick_during_modal_prompt() {
        let mut state = world_state(open_world_grid(), 10, 20);
        let _ = state.start_wishing_well_prompt(Direction::East);
        let clock_before = state.clock;

        assert!(!visual_idle_tick(&mut state));

        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, clock_before);
        assert_eq!(state.animation.frame, 0);
        assert_eq!(state.message, "Wishing well: toss a coin? (Y/N)");
    }

    #[test]
    fn visual_wait_frame_blinks_line_prompt_without_world_tick() {
        let mut state = world_state(open_world_grid(), 10, 20);
        install_test_conversation(&mut state);
        let clock_before = state.clock;
        let mut prompt_cursor_visible = false;

        assert!(advance_visual_wait_frame(
            &mut state,
            &mut prompt_cursor_visible
        ));
        assert!(prompt_cursor_visible);
        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, clock_before);
        assert_eq!(state.animation.frame, 0);

        assert!(advance_visual_wait_frame(
            &mut state,
            &mut prompt_cursor_visible
        ));
        assert!(!prompt_cursor_visible);
        assert_eq!(state.animation.frame, 0);
    }

    #[test]
    fn visual_prompt_cursor_changes_fixed_cell_frame_only_when_visible() {
        let mut state = test_state(open_grid(), 1, 1);
        install_test_conversation(&mut state);
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();

        let hidden =
            render_integrated_status_framebuffer(&mut state.clone(), "job", "", &font, false);
        let visible =
            render_integrated_status_framebuffer(&mut state.clone(), "job", "", &font, true);

        assert_ne!(hash_bytes(&hidden), hash_bytes(&visible));
    }

    #[test]
    fn visual_cast_prompt_receives_backspace_from_key_map() {
        let mut state = test_state(open_grid(), 1, 1);
        state.start_cast_spell_prompt();

        for key in [KeyCode::KeyI, KeyCode::KeyN, KeyCode::Backspace] {
            let ch = key_code_to_char(key, false, false).unwrap();
            handle_play_key_input(&mut state, ch, "", Path::new("")).unwrap();
        }

        assert_eq!(state.active_cast.as_ref().unwrap().buffer, "I");
        assert!(state.message.contains("Spell name: I"));
    }

    #[test]
    fn visual_status_reports_music_toggle_state() {
        let mut state = test_state(open_grid(), 1, 1);

        assert!(summarize(&mut state, "", "").contains("music on"));
        handle_play_key_input(&mut state, PLAY_MUSIC_TOGGLE_KEY, "", Path::new("")).unwrap();

        assert!(summarize(&mut state, "", "").contains("music off"));
    }

    #[test]
    fn visual_line_input_buffers_conversation_keyword_until_enter() {
        let mut state = test_state(open_grid(), 1, 1);
        install_test_conversation(&mut state);
        let mut input_line = String::new();

        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::KeyJ,
            false,
            false,
            Path::new(""),
        )
        .unwrap();
        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::KeyO,
            false,
            false,
            Path::new(""),
        )
        .unwrap();
        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::KeyB,
            false,
            false,
            Path::new(""),
        )
        .unwrap();

        assert_eq!(input_line, "job");
        assert!(state.active_conversation.is_some());
        assert!(!state.message.contains("mend"));

        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Enter,
            false,
            false,
            Path::new(""),
        )
        .unwrap();

        assert!(input_line.is_empty());
        assert!(state.message.contains("mend"));
    }

    #[test]
    fn visual_line_input_supports_backspace_and_status_echo() {
        let mut state = test_state(open_grid(), 1, 1);
        install_test_conversation(&mut state);
        let mut input_line = String::new();

        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::KeyJ,
            false,
            false,
            Path::new(""),
        )
        .unwrap();
        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::KeyX,
            false,
            false,
            Path::new(""),
        )
        .unwrap();
        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Backspace,
            false,
            false,
            Path::new(""),
        )
        .unwrap();

        assert_eq!(input_line, "j");
        let summary = summarize(&mut state, "", &input_line);
        assert!(summary.contains("\n> j"));
    }

    #[test]
    fn visual_line_input_ignores_control_shortcuts() {
        let mut state = test_state(open_grid(), 1, 1);
        install_test_conversation(&mut state);
        let mut input_line = String::new();

        let result = handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::KeyS,
            false,
            true,
            Path::new(""),
        )
        .unwrap();

        assert_eq!(result, None);
        assert!(input_line.is_empty());
        assert!(state.music_enabled);
        assert!(state.active_conversation.is_some());
    }

    #[test]
    fn visual_line_input_discards_direction_and_function_bytes() {
        let mut state = test_state(open_grid(), 1, 1);
        install_test_conversation(&mut state);
        let mut input_line = String::new();

        for key in [
            KeyCode::ArrowUp,
            KeyCode::Numpad1,
            KeyCode::Digit8,
            KeyCode::F1,
        ] {
            let shift = key == KeyCode::Digit8;
            let result = handle_visual_line_key(
                &mut state,
                &mut input_line,
                key,
                shift,
                false,
                Path::new(""),
            )
            .unwrap();
            assert_eq!(result, None);
        }

        assert!(input_line.is_empty());
        assert!(state.active_conversation.is_some());
    }

    #[test]
    fn visual_line_input_buffers_shop_quantity_until_enter() {
        let mut state = test_state(open_grid(), 1, 1);
        state.gold = 100;
        state.reagents = [0; REAGENT_COUNT];
        state.active_shop = Some(ActiveShopSession::Reagent(ReagentShopState::for_herbalist(
            Herbalist::Mysticism,
        )));
        handle_play_key_input(&mut state, 'A', "", Path::new("")).unwrap();
        assert!(matches!(
            state.active_shop.as_ref(),
            Some(ActiveShopSession::Reagent(
                ReagentShopState::PickQuantity { .. }
            ))
        ));
        assert!(visual_line_prompt_active(&state));

        let mut input_line = String::new();
        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Digit1,
            false,
            false,
            Path::new(""),
        )
        .unwrap();
        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Digit2,
            false,
            false,
            Path::new(""),
        )
        .unwrap();

        assert_eq!(input_line, "12");
        assert_eq!(state.gold, 100);
        assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 0);

        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Enter,
            false,
            false,
            Path::new(""),
        )
        .unwrap();

        assert!(input_line.is_empty());
        assert_eq!(state.gold, 28);
        assert_eq!(state.reagents[REAGENT_SPIDER_SILK], 12);
        assert!(state.message.contains("72 gold"));
    }

    #[test]
    fn visual_line_input_buffers_blackthorn_answer_until_enter() {
        let dir = debug_game_dir();
        let mut state = test_state(open_grid(), 1, 1);
        let mut challenge = BlackthornChallenge::new();
        challenge.begin();
        state.active_blackthorn = Some(challenge);

        let mut input_line = String::new();
        for key in [KeyCode::KeyA, KeyCode::KeyH, KeyCode::KeyM] {
            handle_visual_line_key(&mut state, &mut input_line, key, false, false, &dir).unwrap();
        }

        assert_eq!(input_line, "ahm");
        assert!(state.active_blackthorn.is_some());
        assert!(!state.blackthorn_story.is_party_slot_jailed(0));

        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Enter,
            false,
            false,
            &dir,
        )
        .unwrap();

        assert!(input_line.is_empty());
        assert!(state.active_blackthorn.is_none());
        assert!(state.blackthorn_story.is_party_slot_jailed(0));
        assert!(
            state
                .message
                .contains("Returned to Blackthorn's captive cell")
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_line_prompt_active_covers_quantity_shop_states() {
        let mut tavern = test_state(open_grid(), 1, 1);
        tavern.active_shop = Some(ActiveShopSession::Tavern(
            TavernState::PickProvisionQuantity {
                tavern: Tavern::TheWayfarerTavern,
                unit_price: 15,
                continuation_ready: false,
            },
        ));
        assert!(visual_line_prompt_active(&tavern));

        let mut guild = test_state(open_grid(), 1, 1);
        guild.active_shop = Some(ActiveShopSession::Guild(GuildShopState::PickQuantity {
            shop: GuildShop::TheDen,
            commodity: u5_runtime::GuildCommodity::Keys,
            unit_price: 190,
        }));
        assert!(visual_line_prompt_active(&guild));
    }

    #[test]
    fn visual_summary_includes_active_shop_modal_text() {
        let mut state = test_state(open_grid(), 1, 1);
        state.message = "Mace costs 42 gold.".to_string();
        state.active_shop = Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::BuyConfirm {
                item: 1,
                quoted_price: 42,
                quote_record_id: SHOPPE_RECORDS_ARMS_DESCRIPTIONS_FIRST + 1,
            },
            ArmsShop::IolosBows.stock_table(),
        ));

        let summary = summarize(&mut state, "", "");

        assert!(summary.contains("Iolo"), "{summary}");
        assert!(summary.contains("Item 1 costs 42 gold"), "{summary}");
        assert!(summary.contains("Mace costs 42 gold."), "{summary}");
        assert!(summary.contains("STATS"), "{summary}");
    }

    #[test]
    fn visual_shop_line_prompt_empty_echo_preserves_outcome_text() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let mut state = test_state(open_grid(), 1, 1);
        state.active_shop = Some(ActiveShopSession::Sage(SageState::default()));

        let before =
            render_integrated_status_framebuffer(&mut state.clone(), "", READY_HINT, &font, false);
        handle_play_key_input(&mut state, 'M', "ANTRA", Path::new("")).unwrap();
        let after =
            render_integrated_status_framebuffer(&mut state.clone(), "", READY_HINT, &font, false);

        assert_ne!(hash_bytes(&before), hash_bytes(&after));
    }

    #[test]
    fn visual_line_input_buffers_shrine_mantra_until_enter() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = world_state(grid, 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };

        handle_play_key_input(&mut state, 'M', "", &dir).unwrap();
        assert!(visual_line_prompt_active(&state));

        let mut input_line = String::new();
        for key in [KeyCode::KeyA, KeyCode::KeyH, KeyCode::KeyM] {
            handle_visual_line_key(&mut state, &mut input_line, key, false, false, &dir).unwrap();
        }
        assert_eq!(input_line, "ahm");
        assert_eq!(state.shrine_ordained_mask, 0);

        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Enter,
            false,
            false,
            &dir,
        )
        .unwrap();

        assert!(input_line.is_empty());
        assert_eq!(state.shrine_ordained_mask, ShrineVirtue::Honesty.bit());
        assert!(state.message.contains("ordained"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_line_input_escape_cancels_shrine_prompt() {
        let dir = debug_game_dir();
        fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 10 20 HONESTY 136\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = 136;
        let mut state = world_state(grid, 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        handle_play_key_input(&mut state, 'M', "", &dir).unwrap();
        let mut input_line = "ahm".to_string();

        handle_visual_line_key(
            &mut state,
            &mut input_line,
            KeyCode::Escape,
            false,
            false,
            &dir,
        )
        .unwrap();

        assert!(input_line.is_empty());
        assert!(state.active_shrine.is_none());
        assert!(state.message.contains("None"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_intro_summary_switches_from_title_to_menu() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::Menu,
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        let title = summarize_intro(&mut intro);
        assert!(title.contains("Press any key"));
        assert!(!title.contains("Create New Character"));

        intro.dispatch.dismiss_title();
        intro.message = "Choose a path.".to_string();
        let menu = summarize_intro(&mut intro);
        assert!(menu.contains("Journey Onward"));
        assert!(menu.contains("Create New Character"));
        assert!(menu.contains("Choose a path."));
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn visual_intro_load_error_message_surfaces_disk_prompt_boundary() {
        let message =
            visual_intro_load_error_message(&io::Error::new(io::ErrorKind::NotFound, "missing"));

        assert!(message.contains("Disk read failed for SAVED.GAM"));
        assert!(message.contains("mounted game/save directory"));
    }

    #[test]
    fn visual_return_to_view_summary_reports_miscmap_shape() {
        let dir = debug_game_dir();
        fs::write(dir.join(MISCMAPS_DAT_FILE), vec![0u8; 128]).unwrap();

        let preview = visual_return_to_view_summary(&dir, TileGraphicsDepth::Ega16);

        assert!(preview.summary.contains(MISCMAPS_DAT_FILE));
        assert!(preview.summary.contains("128 bytes"));
        assert!(preview.summary.contains("Return-to-View strips"));
        assert!(preview.frames_rgba.is_empty());
        assert_eq!(preview.width, 0);
        assert_eq!(preview.height, 0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_intro_story_summary_uses_step_art_and_story_record() {
        let records = StoryRecords {
            records: (0..20).map(|i| format!("Story record {i}")).collect(),
        };

        let summary = summarize_intro_story(&records, 7);

        assert!(summary.contains("Story step 8 of 21"));
        assert!(summary.contains("Art STORY3.16 subimage 0 at (0, 0)."));
        assert!(summary.contains("Transition strips"));
        assert!(summary.contains("Story record 6"));
        assert!(summary.contains("Press any key"));
    }

    #[test]
    fn visual_intro_story_art_helpers_use_spec_file_stem_and_palette() {
        assert_eq!(intro_story_stem("STORY3.16"), "STORY3");
        assert_eq!(intro_story_stem("STORY3"), "STORY3");
        let image = GraphicImage {
            width: 2,
            height: 1,
            pixels: vec![0, 15],
        };

        let rgba = graphic_image_to_rgba(&image, TileGraphicsDepth::Ega16);

        assert_eq!(&rgba[..4], &[0x00, 0x00, 0x00, 0xff]);
        assert_eq!(&rgba[4..8], &[0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn visual_intro_story_panel_pages_back_to_menu_after_final_step() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::Story {
                records: StoryRecords {
                    records: (0..20).map(|i| format!("Story record {i}")).collect(),
                },
                step: INTRO_STORY_STEP_COUNT - 1,
                transition: None,
            },
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };
        intro.dispatch.dismiss_title();
        intro.dispatch.submit_menu_key(b'U');

        assert!(step_visual_intro_panel(&mut intro, ' '));

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.contains("Introduction complete"));
        assert!(matches!(
            intro.dispatch.submit_menu_key(b'A'),
            UnifiedMenuStep::EnteredSubflow(IntroSubflow::Acknowledgements)
        ));
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    fn chargen_records() -> Vec<String> {
        (0..30)
            .map(|index| format!("Questionnaire record {index}"))
            .collect()
    }

    fn visual_intro_state_with_panel(
        dir: std::path::PathBuf,
        panel: VisualIntroPanel,
    ) -> VisualIntroState {
        let mut dispatch = UnifiedMenuDispatch::new();
        dispatch.dismiss_title();
        VisualIntroState {
            game_dir: dir,
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch,
            title_flourish_step: intro_title_flourish_total_steps(),
            title_flourish_complete: true,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel,
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        }
    }

    #[test]
    fn visual_intro_character_creation_writes_save_and_returns_to_menu() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(INIT_GAM_FILENAME),
            saved_game_seed_bytes(13, 0, 15, 15),
        )
        .unwrap();
        fs::write(dir.join(INIT_OOL_FILENAME), vec![0x44; OOL_PLANE_LEN]).unwrap();
        let session = ChargenSession::new(chargen_records(), (0u8..=127).collect()).unwrap();
        let mut intro = visual_intro_state_with_panel(
            dir.clone(),
            VisualIntroPanel::CharacterCreation {
                session,
                input_line: String::new(),
            },
        );

        for ch in "Avatar".chars() {
            step_visual_intro_panel(&mut intro, ch);
        }
        step_visual_intro_panel(&mut intro, '\r');
        step_visual_intro_panel(&mut intro, 'M');
        step_visual_intro_panel(&mut intro, ' ');
        step_visual_intro_panel(&mut intro, ' ');
        for _ in 0..7 {
            step_visual_intro_panel(&mut intro, 'A');
        }
        step_visual_intro_panel(&mut intro, ' ');

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.contains("Created Avatar"));
        let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(
            &saved[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + SAVE_CHARACTER_NAME_LEN - 1],
            b"Avatar\0\0"
        );
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_GENDER_OFFSET],
            0x0b
        );
        assert_eq!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_STR_OFFSET], 20);
        assert!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_DEX_OFFSET] > 0);
        assert!(saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_INT_OFFSET] > 0);
        assert_eq!(
            fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap(),
            [vec![0u8; OOL_PLANE_LEN], vec![0x44; OOL_PLANE_LEN]].concat()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_intro_character_creation_escape_returns_to_menu_without_save() {
        let dir = debug_game_dir();
        let session = ChargenSession::new(chargen_records(), (0u8..=127).collect()).unwrap();
        let mut intro = visual_intro_state_with_panel(
            dir.clone(),
            VisualIntroPanel::CharacterCreation {
                session,
                input_line: "Avatar".to_string(),
            },
        );
        intro.dispatch.submit_menu_key(b'C');

        assert!(cancel_visual_intro_panel(&mut intro));

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.contains("Character creation cancelled"));
        assert!(!dir.join(SAVED_GAM_FILENAME).exists());
        assert!(matches!(
            intro.dispatch.submit_menu_key(b'A'),
            UnifiedMenuStep::EnteredSubflow(IntroSubflow::Acknowledgements)
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_intro_u4_transfer_accepts_overrides_and_writes_save() {
        let dir = debug_game_dir();
        fs::write(
            dir.join(U4_TRANSFER_U5_SEED_GAM_FILENAME),
            saved_game_seed_bytes(0, 0, 10, 20),
        )
        .unwrap();
        fs::write(dir.join(BRIT_OOL_FILENAME), vec![0x55; OOL_PLANE_LEN]).unwrap();
        let source = U4TransferSource {
            name: b"OLDNAME\0\0".to_vec(),
            male: true,
            class_index: 6,
            strength: 35,
            dexterity: 20,
            intelligence: 22,
            experience: 1500,
        };
        let preview = u4_transfer_preview_from_u4_values(
            display_name_bytes(&source.name),
            source.class_index,
            source.strength,
            source.dexterity,
            source.intelligence,
            0,
        );
        let mut intro = visual_intro_state_with_panel(
            dir.clone(),
            VisualIntroPanel::U4Transfer {
                source,
                preview,
                overrides: U4TransferOverrides {
                    name: None,
                    male: None,
                },
                stage: VisualU4TransferStage::ConfirmName,
                input_line: String::new(),
            },
        );

        step_visual_intro_panel(&mut intro, 'N');
        for ch in "New".chars() {
            step_visual_intro_panel(&mut intro, ch);
        }
        step_visual_intro_panel(&mut intro, '\r');
        step_visual_intro_panel(&mut intro, 'N');
        step_visual_intro_panel(&mut intro, 'F');
        step_visual_intro_panel(&mut intro, 'Y');

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.contains("Transferred New"));
        let saved = fs::read(dir.join(SAVED_GAM_FILENAME)).unwrap();
        assert_eq!(
            &saved[SAVE_ROSTER_OFFSET..SAVE_ROSTER_OFFSET + SAVE_CHARACTER_NAME_LEN - 1],
            b"New\0\0\0\0\0"
        );
        assert_eq!(
            saved[SAVE_ROSTER_OFFSET + SAVE_CHARACTER_GENDER_OFFSET],
            0x0c
        );
        assert_eq!(
            fs::read(dir.join(SAVED_OOL_FILENAME)).unwrap(),
            [vec![0u8; OOL_PLANE_LEN], vec![0x55; OOL_PLANE_LEN]].concat()
        );
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn visual_intro_u4_transfer_escape_returns_to_menu_without_save() {
        let dir = debug_game_dir();
        let source = U4TransferSource {
            name: b"OLDNAME\0\0".to_vec(),
            male: true,
            class_index: 6,
            strength: 35,
            dexterity: 20,
            intelligence: 22,
            experience: 1500,
        };
        let preview = u4_transfer_preview_from_u4_values(
            display_name_bytes(&source.name),
            source.class_index,
            source.strength,
            source.dexterity,
            source.intelligence,
            0,
        );
        let mut intro = visual_intro_state_with_panel(
            dir.clone(),
            VisualIntroPanel::U4Transfer {
                source,
                preview,
                overrides: U4TransferOverrides {
                    name: Some(b"New".to_vec()),
                    male: None,
                },
                stage: VisualU4TransferStage::ConfirmGender,
                input_line: String::new(),
            },
        );
        intro.dispatch.submit_menu_key(b'T');

        assert!(cancel_visual_intro_panel(&mut intro));

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.contains("Transfer cancelled"));
        assert!(!dir.join(SAVED_GAM_FILENAME).exists());
        assert!(matches!(
            intro.dispatch.submit_menu_key(b'A'),
            UnifiedMenuStep::EnteredSubflow(IntroSubflow::Acknowledgements)
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn return_to_view_intro_input_waits_until_final_frame() {
        let dir = debug_game_dir();
        let mut intro = visual_intro_state_with_panel(
            dir.clone(),
            VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_frames_rgba: vec![
                    vec![0x00, 0x00, 0x00, 0xff],
                    vec![0xff, 0xff, 0xff, 0xff],
                ],
                frame_metadata: vec![
                    VisualReturnToViewFrameMeta {
                        command_index: 0,
                        elapsed_title_ticks: 1,
                        kind: ReturnToViewFrameKind::PreviewTick,
                        caption: Some("The Castle of Lord British"),
                    },
                    VisualReturnToViewFrameMeta {
                        command_index: 1,
                        elapsed_title_ticks: 2,
                        kind: ReturnToViewFrameKind::FixedWait { tick: 0 },
                        caption: Some("The Keep of Lord Blackthorn"),
                    },
                ],
                preview_frame_index: 0,
                preview_width: 1,
                preview_height: 1,
            },
        );
        intro.dispatch.submit_menu_key(b'R');

        assert!(step_visual_intro_panel(&mut intro, 'x'));
        assert!(matches!(
            intro.panel,
            VisualIntroPanel::ReturnToView {
                preview_frame_index: 0,
                ..
            }
        ));
        assert!(intro.message.is_empty());

        assert!(advance_visual_intro_panel_animation(
            &mut intro.panel,
            &mut intro.title_tick_frame
        ));
        assert!(step_visual_intro_panel(&mut intro, 'x'));

        assert!(matches!(intro.panel, VisualIntroPanel::Menu));
        assert!(intro.message.contains("Return-to-View preview complete"));
        assert!(matches!(
            intro.dispatch.submit_menu_key(b'A'),
            UnifiedMenuStep::EnteredSubflow(IntroSubflow::Acknowledgements)
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn return_to_view_intro_escape_cancel_is_not_available() {
        let dir = debug_game_dir();
        let mut intro = visual_intro_state_with_panel(
            dir.clone(),
            VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_frames_rgba: vec![vec![0x00, 0x00, 0x00, 0xff]],
                frame_metadata: vec![VisualReturnToViewFrameMeta {
                    command_index: 0,
                    elapsed_title_ticks: 1,
                    kind: ReturnToViewFrameKind::PreviewTick,
                    caption: Some("The Castle of Lord British"),
                }],
                preview_frame_index: 0,
                preview_width: 1,
                preview_height: 1,
            },
        );
        intro.dispatch.submit_menu_key(b'R');

        assert!(!cancel_visual_intro_panel(&mut intro));
        assert!(matches!(intro.panel, VisualIntroPanel::ReturnToView { .. }));
        assert!(intro.message.is_empty());
        assert!(matches!(
            intro.dispatch.submit_menu_key(b'A'),
            UnifiedMenuStep::Ignored
        ));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn return_to_view_intro_frame_overlays_preview_rgba() {
        let preview_rgba = vec![
            0xff, 0x00, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff,
        ];
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_frames_rgba: vec![preview_rgba],
                frame_metadata: vec![VisualReturnToViewFrameMeta {
                    command_index: 6,
                    elapsed_title_ticks: 12,
                    kind: ReturnToViewFrameKind::PreviewTick,
                    caption: Some("The Castle of Lord British"),
                }],
                preview_frame_index: 0,
                preview_width: 2,
                preview_height: 2,
            },
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        let frame = render_intro_frame(&mut intro);
        let x = ((INTRO_FRAMEBUFFER_WIDTH as usize) - 2) / 2;
        let offset = ((RETURN_TO_VIEW_PREVIEW_Y * INTRO_FRAMEBUFFER_WIDTH as usize) + x) * 4;

        assert_eq!(&frame[offset..offset + 4], &[0xff, 0x00, 0x00, 0xff]);
        let caption_start = RETURN_TO_VIEW_CAPTION_Y * INTRO_FRAMEBUFFER_WIDTH as usize * 4;
        let caption_end =
            caption_start + RETURN_TO_VIEW_CAPTION_HEIGHT * INTRO_FRAMEBUFFER_WIDTH as usize * 4;
        assert!(
            frame[caption_start..caption_end]
                .chunks_exact(4)
                .any(|pixel| { pixel == [0x55, 0xff, 0xff, 0xff] })
        );
        assert!(
            frame[..caption_start]
                .chunks_exact(4)
                .any(|pixel| { pixel != [0x00, 0x00, 0x00, 0xff] }),
            "Return-to-View should preserve or synthesize a visible intro backing surface"
        );
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn return_to_view_intro_frame_ticks_over_preserved_backing() {
        let mut intro = visual_intro_state_with_panel(
            debug_game_dir(),
            VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_frames_rgba: vec![vec![0x00, 0x00, 0x00, 0xff]],
                frame_metadata: vec![VisualReturnToViewFrameMeta {
                    command_index: 6,
                    elapsed_title_ticks: 12,
                    kind: ReturnToViewFrameKind::PreviewTick,
                    caption: Some("The Castle of Lord British"),
                }],
                preview_frame_index: 0,
                preview_width: 1,
                preview_height: 1,
            },
        );
        intro.modal_backing = Some(vec![
            0x00;
            (INTRO_FRAMEBUFFER_WIDTH as usize)
                * (INTRO_FRAMEBUFFER_HEIGHT as usize)
                * 4
        ]);
        intro.title_tick_visible_frame = 0;

        let frame = render_intro_frame(&mut intro);

        assert_eq!(
            rgba_pixel(
                &frame,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                54,
                TITLE_TICK_FRAME_Y as usize + 20
            ),
            [0xff, 0xff, 0x55, 0xff]
        );
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn return_to_view_intro_frame_draws_fixed_wipe_rectangles() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_frames_rgba: vec![vec![0x00, 0x00, 0x00, 0xff]],
                frame_metadata: vec![VisualReturnToViewFrameMeta {
                    command_index: 11,
                    elapsed_title_ticks: 1,
                    kind: ReturnToViewFrameKind::FixedWipeRectangle { step: 0 },
                    caption: Some("The Summoning"),
                }],
                preview_frame_index: 0,
                preview_width: 1,
                preview_height: 1,
            },
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        let frame = render_intro_frame(&mut intro);
        let pixel = |x: usize, y: usize| -> &[u8] {
            let offset = (y * INTRO_FRAMEBUFFER_WIDTH as usize + x) * 4;
            &frame[offset..offset + 4]
        };

        assert_eq!(pixel(128, 152), RETURN_TO_VIEW_FIXED_WIPE_RGBA);
        assert_eq!(pixel(137, 156), RETURN_TO_VIEW_FIXED_WIPE_RGBA);
        assert_ne!(pixel(127, 152), RETURN_TO_VIEW_FIXED_WIPE_RGBA);
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn return_to_view_intro_animation_advances_preview_frames_until_final() {
        let mut panel = VisualIntroPanel::ReturnToView {
            summary: "Preview".to_string(),
            preview_frames_rgba: vec![vec![0x00, 0x00, 0x00, 0xff], vec![0xff, 0xff, 0xff, 0xff]],
            frame_metadata: vec![
                VisualReturnToViewFrameMeta {
                    command_index: 0,
                    elapsed_title_ticks: 1,
                    kind: ReturnToViewFrameKind::PreviewTick,
                    caption: Some("The Castle of Lord British"),
                },
                VisualReturnToViewFrameMeta {
                    command_index: 1,
                    elapsed_title_ticks: 2,
                    kind: ReturnToViewFrameKind::FixedWait { tick: 0 },
                    caption: Some("The Keep of Lord Blackthorn"),
                },
            ],
            preview_frame_index: 0,
            preview_width: 1,
            preview_height: 1,
        };
        let mut title_tick_frame = 0;

        assert!(advance_visual_intro_panel_animation(
            &mut panel,
            &mut title_tick_frame
        ));
        assert_eq!(title_tick_frame, title_tick_next_frame(0));
        assert!(matches!(
            panel,
            VisualIntroPanel::ReturnToView {
                preview_frame_index: 1,
                ..
            }
        ));

        assert!(!advance_visual_intro_panel_animation(
            &mut panel,
            &mut title_tick_frame
        ));
        assert!(matches!(
            panel,
            VisualIntroPanel::ReturnToView {
                preview_frame_index: 1,
                ..
            }
        ));
    }

    #[test]
    fn return_to_view_intro_summary_reports_current_caption_and_frame_kind() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_flourish_step: 0,
            title_flourish_complete: false,
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            title_tick_visible_frame: 0,
            start_menu_reveal: None,
            start_menu_reveal_backing: None,
            modal_backing: None,
            menu_idle_ticks: 0,
            message_waiting_for_key: false,
            message: String::new(),
            panel: VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_frames_rgba: vec![
                    vec![0x00, 0x00, 0x00, 0xff],
                    vec![0xff, 0xff, 0xff, 0xff],
                ],
                frame_metadata: vec![
                    VisualReturnToViewFrameMeta {
                        command_index: 2,
                        elapsed_title_ticks: 9,
                        kind: ReturnToViewFrameKind::CellEffectStep { step: 4 },
                        caption: Some("The Castle of Lord British"),
                    },
                    VisualReturnToViewFrameMeta {
                        command_index: 3,
                        elapsed_title_ticks: 10,
                        kind: ReturnToViewFrameKind::TemporaryActorDraw,
                        caption: Some("The Keep of Lord Blackthorn"),
                    },
                ],
                preview_frame_index: 1,
                preview_width: 1,
                preview_height: 1,
            },
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        let summary = summarize_intro(&mut intro);
        assert!(summary.contains("Playback frame 2 of 2"));
        assert!(summary.contains("Temporary actor draw"));
        assert!(summary.contains("command 3"));
        assert!(summary.contains("title tick 10"));
        assert!(summary.contains("The Keep of Lord Blackthorn"));
        let _ = fs::remove_dir_all(&intro.game_dir);
    }
}

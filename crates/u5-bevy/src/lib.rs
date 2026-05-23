//! Bevy-backed visual harness. The gameplay source of truth stays in
//! [`PlayState`]; this module only owns the window, the CPU framebuffer, and
//! the Bevy texture handle. Input dispatch reuses the terminal-mode handler so
//! movement, doors, transitions, and other supported behavior come along for
//! free.

use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use image::{ImageBuffer, Rgba};

use u5_runtime::{
    ActiveObject, BLINK_COST, BLINK_SPELL_INDEX, BRITISH_PTH_PEN_ORIGINS, BritishPth,
    CGA_PALETTE_RGB, COMBAT_ARENA_SIDE, ChargenSession, ChargenSessionResult, ChargenSessionStep,
    DEATH_VISION_OBJECT_CLASS, Direction, DungeonScene, EGA_PALETTE_RGB,
    FIRST_PLAYABLE_FRIGATE_TILE, FIRST_PLAYABLE_FULL_SHIP_HULL, FixedCellFont, GameClock,
    GraphicImage, HORSE_PARKED_FIRST, INTRO_INLINE_DOORWAY_STEP, INTRO_STEP_1_EXTRA_ART_X,
    INTRO_STEP_1_EXTRA_ART_Y, INTRO_STEP_1_EXTRA_SUBIMAGE, INTRO_STEP_1_RECT_TRANSITION,
    INTRO_STEP_6_EXTRA_ART_X, INTRO_STEP_6_EXTRA_ART_Y, INTRO_STEP_6_EXTRA_SUBIMAGE,
    INTRO_STORY_STEP_COUNT, INTRO_STORY6_SECONDARY_Y_DELTA, Inn, IntroStoryArtPlacement,
    MAIN_TEXT_WINDOW_INDEX, MISCMAPS_DAT_FILE, MISCMAPS_RTV_COMMAND_SECTION_OFFSET,
    MISCMAPS_RTV_STRIP_SECTION_BYTES, MISCMAPS_RTV_STRIP_SECTION_OFFSET, MonochromeBitmap,
    PLAY_MUSIC_TOGGLE_KEY, PROMPT_TEXT_WINDOW_INDEX, PlayInputDisposition, PlayOptions, PlayState,
    PlayTarget, RTV_COMMAND_STREAM_BYTES, RectColumnSweepTransition, SPECIAL_ITEM_OWNED_VALUE,
    SPECIAL_ITEM_SPYGLASS_INDEX, STATS_PANEL_TEXT_BOTTOM, STATS_PANEL_TEXT_LEFT,
    STATS_PANEL_TEXT_RIGHT, STATS_PANEL_TEXT_WINDOW_INDEX, Scene, Stable, StoryRecords,
    TEXT_SCREEN_ROWS, TEXT_WINDOW_RENDER_HEIGHT, TEXT_WINDOW_RENDER_WIDTH, TILE_ATLAS_SIDE,
    TITLE_BIT_INITIAL_PLACEMENTS, TITLE_BIT_REMAINING_PLACEMENTS, TITLE_LOWER_BAND_CLEAR_Y,
    TITLE_SURFACE_HEIGHT, TITLE_SURFACE_WIDTH, TITLE_TICK_FRAME_HEIGHT, TITLE_TICK_FRAME_WIDTH,
    TITLE_TICK_FRAME_X, TITLE_TICK_FRAME_Y, TOWN_GAS_DOORWAY_RANGE_MAX, TOWN_GRID_SIDE,
    TOWN_POISON_GAS_LIVE_TILE, TextWindowSystem, TileAtlas, TileGraphicsDepth, TitleBitAsset,
    TitleBitImages, TitleBitPlacement, TransportState, U4TransferOverrides, U4TransferSource,
    WorldPlane, commit_chargen_save, commit_u4_transfer_save, dungeon_cell_index,
    handle_play_key_input, hash_bytes, input_case_fold, input_function_key_code,
    input_keypad_digit_direction_code,
    intro_menu::{IntroSubflow, IntroSubflowResult},
    intro_step_has_story6_secondary_pass, intro_step_transition_strips,
    intro_story_art_file_for_step, intro_story_art_placement_for_step,
    intro_story_step_waits_for_input, intro_story6_secondary_subimage, load_british_bit,
    load_british_pth, load_graphic_image_directory, load_ibm_ch_font, load_play_options_from_save,
    load_question_records, load_return_to_view_assets, load_story_records, load_tile_atlas,
    load_title_bit,
    menu_dispatch::{UnifiedMenuDispatch, UnifiedMenuStep},
    paint_message_text_window, paint_prompt_text_window_with_cursor, paint_stats_panel_text_window,
    read_u4_transfer_source_from_party_sav, render_play_text_window_system,
    render_return_to_view_playback_frame_viewport, render_text_panel_rgba, render_text_window_rgba,
    run_return_to_view_playback_until_restart,
    shop_runtime::{
        GuildShopState, HorseTraderState, InnkeeperState, ReagentShopState, SageState, TavernState,
    },
    shop_session::ActiveShopSession,
    stats_panel_active_cursor_visible, summarize_return_to_view_preview,
    summarize_return_to_view_script, title_tick_next_frame, title_tick_palette_indices,
    u4_transfer_session::{U4TransferPreview, u4_transfer_preview_from_u4_values},
    u5_prng_range_u16,
};

const VIEWPORT_RADIUS: usize = 5;
const VIEWPORT_CELLS: usize = VIEWPORT_RADIUS * 2 + 1;
const VIEWPORT_SIZE_PX: u32 = (VIEWPORT_CELLS * TILE_ATLAS_SIDE) as u32;
const DISPLAY_SCALE: f32 = 3.0;

const READY_HINT: &str =
    "Arrows/keypad: move. Shift+A attacks, Shift+S searches. Ctrl+S music. Esc quit.";
const INTRO_FRAMEBUFFER_WIDTH: u32 = 320;
const INTRO_FRAMEBUFFER_HEIGHT: u32 = 220;
const INTRO_DISPLAY_SCALE: f32 = 2.5;
const PROMPT_CURSOR_GLYPH: u8 = 4;

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
    let options = launch_result
        .lock()
        .expect("visual intro launch lock poisoned")
        .take();
    if let Some(options) = options {
        run_visual_loop(game_dir, options, raster_depth)?;
    }
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
        if let Some(inputs) = case.inputs {
            for (key, suffix) in inputs {
                handle_play_key_input(&mut state, *key, suffix, game_dir)?;
            }
        }
        if case.synthetic_combat {
            seed_visual_suite_combat(&mut state);
        }
        if let Some(configure) = case.configure {
            configure(&mut state);
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
        let mut state = PlayState::load_scene(game_dir, case.options)?;
        if let Some(configure) = case.configure {
            configure(&mut state);
        }
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
            apply_visual_route_command(&mut state, command, game_dir)?;
            let report = write_visual_play_report(
                out_dir,
                &visual_route_step_label(case.label, index + 1, command),
                case.frame_kind,
                &mut state,
                &atlas,
                &font,
            )?;
            if report.byte_hash == previous_hash {
                return Err(io::Error::other(format!(
                    "visual route suite `{}` command `{}` did not change the frame",
                    case.label, command
                )));
            }
            previous_hash = report.byte_hash;
            reports.push(report);
        }
    }

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
            configure: Some(|state| {
                state.enter_endgame();
            }),
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

struct VisualRouteSuiteCase {
    label: &'static str,
    frame_kind: &'static str,
    options: PlayOptions,
    script: &'static [&'static str],
    configure: Option<fn(&mut PlayState)>,
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
    vec![
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
            label: "route-dungeon-heavy-door-variant-block",
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
    ]
}

fn seed_visual_route_board_horse(state: &mut PlayState) {
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

fn apply_visual_route_command(
    state: &mut PlayState,
    command: &str,
    game_dir: &Path,
) -> io::Result<PlayInputDisposition> {
    let command = command.trim();
    let mut chars = command.chars();
    let Some(key) = chars.next() else {
        return handle_play_key_input(state, '\n', "", game_dir);
    };
    handle_play_key_input(state, key, chars.as_str(), game_dir)
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
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            message: String::new(),
            panel: VisualIntroPanel::Menu,
            launch_result,
            image_handle: None,
        })
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
                handled |= step_visual_intro(&mut intro, *ch, &mut exit);
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
    title_signature_progress: usize,
    title_signature_complete: bool,
    title_tick_frame: u8,
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
        preview_frame_index: usize,
        preview_width: usize,
        preview_height: usize,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct VisualReturnToViewPreview {
    summary: String,
    frames_rgba: Vec<Vec<u8>>,
    width: usize,
    height: usize,
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
    keyboard: Res<ButtonInput<KeyCode>>,
    intro: Option<ResMut<VisualIntroState>>,
    mut images: ResMut<Assets<Image>>,
    mut exit: EventWriter<AppExit>,
) {
    let Some(mut intro) = intro else {
        return;
    };
    let mut handled = false;
    if keyboard.just_pressed(KeyCode::Escape) {
        if cancel_visual_intro_panel(&mut intro) {
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
        if step_visual_intro(&mut intro, ch, &mut exit) {
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
}

fn animate_visual_intro_title_effects(
    intro: Option<ResMut<VisualIntroState>>,
    mut images: ResMut<Assets<Image>>,
) {
    const SIGNATURE_STEPS_PER_FRAME: usize = 24;

    let Some(mut intro) = intro else {
        return;
    };

    if matches!(intro.panel, VisualIntroPanel::Menu) {
        let title_phase = matches!(intro.dispatch.tick_title(), UnifiedMenuStep::PresentTitle);

        intro.title_tick_frame = title_tick_next_frame(intro.title_tick_frame);

        if title_phase && !intro.title_signature_complete {
            let Ok(signature) = load_british_pth(&intro.game_dir) else {
                intro.title_signature_complete = true;
                return;
            };
            let total_steps = british_signature_step_count(&signature);
            if total_steps == 0 {
                intro.title_signature_complete = true;
                return;
            }

            intro.title_signature_progress =
                (intro.title_signature_progress + SIGNATURE_STEPS_PER_FRAME).min(total_steps);
            if intro.title_signature_progress >= total_steps {
                intro.title_signature_progress = 0;
                intro.title_signature_complete = true;
            }
        }
    } else {
        let mut title_tick_frame = intro.title_tick_frame;
        if !advance_visual_intro_panel_animation(&mut intro.panel, &mut title_tick_frame) {
            return;
        }
        intro.title_tick_frame = title_tick_frame;
    }

    let rgba = render_intro_frame(&mut intro);
    if let Some(handle) = intro.image_handle.as_ref() {
        if let Some(image) = images.get_mut(handle) {
            image.data = Some(rgba);
        }
    }
}

fn advance_visual_intro_panel_animation(
    panel: &mut VisualIntroPanel,
    title_tick_frame: &mut u8,
) -> bool {
    advance_visual_intro_story_wipe(panel, title_tick_frame)
        || advance_visual_intro_return_to_view(panel, title_tick_frame)
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

fn step_visual_intro(
    intro: &mut VisualIntroState,
    ch: char,
    exit: &mut EventWriter<AppExit>,
) -> bool {
    if !matches!(intro.panel, VisualIntroPanel::Menu) {
        return step_visual_intro_panel(intro, ch);
    }

    if matches!(intro.dispatch.tick_title(), UnifiedMenuStep::PresentTitle) {
        intro.dispatch.dismiss_title();
        intro.title_signature_progress = 0;
        intro.title_signature_complete = true;
        intro.title_tick_frame = 0;
        if ch.eq_ignore_ascii_case(&'J') {
            return resolve_visual_intro_subflow(intro, IntroSubflow::JourneyOnward, exit);
        }
        intro.message.clear();
        return true;
    }

    let key = if ch == '\r' { b'\r' } else { ch as u8 };
    match intro.dispatch.submit_menu_key(key) {
        UnifiedMenuStep::EnteredSubflow(subflow) => {
            resolve_visual_intro_subflow(intro, subflow, exit)
        }
        UnifiedMenuStep::Ignored => {
            intro.message = "Choose J, C, T, U, A, R, or Enter to repeat.".to_string();
            true
        }
        UnifiedMenuStep::PresentMenu | UnifiedMenuStep::ReturnedToMenu => true,
        UnifiedMenuStep::LaunchGameplay => {
            exit.write(AppExit::Success);
            true
        }
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
        VisualIntroPanel::ReturnToView { .. } => VisualIntroPanelOutcome::ReturnToMenu {
            subflow: IntroSubflow::ReturnToView,
            result: IntroSubflowResult::ReturnedToMenu,
            message: "Return-to-View preview complete.".to_string(),
        },
    };

    match outcome {
        VisualIntroPanelOutcome::Stay => {}
        VisualIntroPanelOutcome::ReturnToMenu {
            subflow,
            result,
            message,
        } => {
            intro.panel = VisualIntroPanel::Menu;
            intro.dispatch.complete_subflow(subflow, result);
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
        VisualIntroPanel::ReturnToView { .. } => Some((
            IntroSubflow::ReturnToView,
            IntroSubflowResult::ReturnedToMenu,
            "Return-to-View preview cancelled; returning to the intro menu.",
        )),
    }) else {
        return false;
    };

    intro.panel = VisualIntroPanel::Menu;
    intro.dispatch.complete_subflow(subflow, result);
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
        ChargenSessionStep::PresentIntro { .. } => {
            session.advance_intro();
            VisualIntroPanelOutcome::Stay
        }
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

fn resolve_visual_intro_subflow(
    intro: &mut VisualIntroState,
    subflow: IntroSubflow,
    exit: &mut EventWriter<AppExit>,
) -> bool {
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
                exit.write(AppExit::Success);
            }
            Err(err) => {
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                intro.message = format!("No loadable saved game: {err}");
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
                        intro.message.clear();
                    }
                    Err(err) => {
                        intro.panel = VisualIntroPanel::Menu;
                        intro
                            .dispatch
                            .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                        intro.message = format!("QUESTION.DAT could not start chargen: {err}");
                    }
                }
            }
            Ok(None) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::Cancelled);
                intro.message =
                    "QUESTION.DAT is required for visual character creation.".to_string();
            }
            Err(err) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::Cancelled);
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
                    intro.message.clear();
                }
                Err(err) => {
                    intro.panel = VisualIntroPanel::Menu;
                    intro
                        .dispatch
                        .complete_subflow(subflow, IntroSubflowResult::Cancelled);
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
                intro.message.clear();
            }
            Ok(None) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
                intro.message = "STORY.DAT is missing; returning to the intro menu.".to_string();
            }
            Err(err) => {
                intro.panel = VisualIntroPanel::Menu;
                intro
                    .dispatch
                    .complete_subflow(subflow, IntroSubflowResult::ReturnedToMenu);
                intro.message = format!("STORY.DAT could not be loaded: {err}");
            }
        },
        IntroSubflow::Acknowledgements => {
            intro.panel = VisualIntroPanel::Acknowledgements;
            intro.message.clear();
        }
        IntroSubflow::ReturnToView => {
            let preview = visual_return_to_view_summary(&intro.game_dir, intro.raster_depth);
            intro.panel = VisualIntroPanel::ReturnToView {
                summary: preview.summary,
                preview_frames_rgba: preview.frames_rgba,
                preview_frame_index: 0,
                preview_width: preview.width,
                preview_height: preview.height,
            };
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
                "Press any key to return to the intro menu.".to_string(),
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

fn format_story_art_line(file: &str, placement: IntroStoryArtPlacement) -> String {
    format!(
        "Art {file} subimage {} at ({}, {}).",
        placement.subimage, placement.top_left_x, placement.top_left_y
    )
}

fn render_intro_frame(intro: &mut VisualIntroState) -> Vec<u8> {
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
        let signature_progress = (title_phase && !intro.title_signature_complete)
            .then_some(intro.title_signature_progress);
        let mut drew_title = false;
        if let Some(title_rgba) =
            visual_intro_title_art_rgba(&intro.game_dir, signature_progress, intro.title_tick_frame)
        {
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
            overlay_nonblack_text_panel_rgba(
                &mut rgba,
                INTRO_FRAMEBUFFER_WIDTH as usize,
                INTRO_FRAMEBUFFER_HEIGHT as usize,
                &summary,
            );
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
                18,
            );
        }
    }
    rgba
}

fn visual_intro_title_surface_visible(intro: &VisualIntroState) -> bool {
    matches!(intro.panel, VisualIntroPanel::Menu)
}

fn visual_intro_title_art_rgba(
    game_dir: &Path,
    signature_progress: Option<usize>,
    title_tick_frame: u8,
) -> Option<Vec<u8>> {
    let title = load_title_bit(game_dir).ok()?;
    let british = load_british_bit(game_dir).ok()?;
    let mut rgba = compose_intro_title_art_rgba(&title, &british);
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
    draw_title_tick_overlay_rgba(
        &mut rgba,
        TITLE_SURFACE_WIDTH as usize,
        TITLE_SURFACE_HEIGHT as usize,
        title_tick_frame,
    );
    Some(rgba)
}

fn compose_intro_title_art_rgba(title: &TitleBitImages, british: &MonochromeBitmap) -> Vec<u8> {
    let width = TITLE_SURFACE_WIDTH as usize;
    let height = TITLE_SURFACE_HEIGHT as usize;
    let mut rgba = vec![0; width * height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
    }

    for placement in &TITLE_BIT_INITIAL_PLACEMENTS {
        blit_intro_title_placement_rgba(&mut rgba, width, height, title, british, *placement);
    }
    clear_rgba_band(&mut rgba, width, height, TITLE_LOWER_BAND_CLEAR_Y as usize);
    for placement in &TITLE_BIT_REMAINING_PLACEMENTS {
        blit_intro_title_placement_rgba(&mut rgba, width, height, title, british, *placement);
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

fn blit_intro_title_placement_rgba(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    title: &TitleBitImages,
    british: &MonochromeBitmap,
    placement: TitleBitPlacement,
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
                EGA_PALETTE_RGB[15]
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
    // `cleak/u5-spec#52`: the published title-tick effect is a
    // palette-cycled flame stripe over the band. This is an
    // independently-authored silhouette that follows the public
    // rectangle, four-frame color cycle, and "three upward-tapering
    // flames" visual contract without copying the original driver
    // pixel pattern.
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
            if dst[offset] != 0 || dst[offset + 1] != 0 || dst[offset + 2] != 0 {
                continue;
            }
            let Some(palette_index) = title_tick_flame_palette_index(local_x, local_y, frame)
            else {
                continue;
            };
            let rgb = EGA_PALETTE_RGB[usize::from(palette_index)];
            dst[offset..offset + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
    }
}

fn title_tick_flame_palette_index(local_x: usize, local_y: usize, frame: u8) -> Option<u8> {
    let band_width = TITLE_TICK_FRAME_WIDTH as usize;
    let band_height = TITLE_TICK_FRAME_HEIGHT as usize;
    if local_x >= band_width || local_y >= band_height {
        return None;
    }

    let flame_height = 34usize;
    let flame_top = band_height.saturating_sub(flame_height);
    if local_y < flame_top {
        return None;
    }

    let from_base = band_height - 1 - local_y;
    let frame = usize::from(frame % 4);
    let mut inside = false;
    for center in [54isize, 160, 266] {
        let wave = ((local_y * 3 + frame * 5) % 11) as isize - 5;
        let taper = from_base * 34 / flame_height;
        let half_width = 42usize.saturating_sub(taper).max(5);
        let dx = (local_x as isize - (center + wave)).unsigned_abs();
        if dx <= half_width {
            inside = true;
            break;
        }

        // Add a narrow upper tongue so the stripe reads as flame rather than
        // as three static wedges.
        if from_base > 16 {
            let tongue_center = center + ((frame as isize - 1) * 5);
            let tongue_width = 10usize.saturating_sub((from_base - 16) / 2).max(3);
            if (local_x as isize - tongue_center).unsigned_abs() <= tongue_width {
                inside = true;
                break;
            }
        }
    }
    if !inside {
        return None;
    }

    let (bright, dim) = title_tick_palette_indices(frame as u8);
    if local_y < band_height / 2 {
        Some(bright)
    } else {
        Some(dim)
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
    let mut intro = VisualIntroState {
        game_dir: game_dir.to_path_buf(),
        raster_depth,
        dispatch: UnifiedMenuDispatch::new(),
        title_signature_progress: 0,
        title_signature_complete: false,
        title_tick_frame: 0,
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
        return render_status_framebuffer(state, input_line, fallback, font);
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

    let viewport = render_framebuffer(state, atlas);
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
    rgba
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
    manifest.push_str("# Sanitized: contains dimensions, frame kind, and hashes only.\n");
    for report in reports {
        manifest.push_str(&format!(
            "{}\t{}x{}\t{}\thash {:016x}\tnonblack {}\n",
            report.label,
            report.width,
            report.height,
            report.frame_kind,
            report.byte_hash,
            report.nonblack_pixels
        ));
    }
    std::fs::write(out_dir.join("manifest.txt"), manifest)
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
                    let playback = run_return_to_view_playback_until_restart(
                        &assets.strips,
                        &assets.script,
                        4096,
                    );
                    let frames = load_tile_atlas(game_dir, raster_depth).and_then(|atlas| {
                        let playback = playback?;
                        playback
                            .frames
                            .iter()
                            .map(|frame| {
                                render_return_to_view_playback_frame_viewport(frame, &atlas, 0).map(
                                    |viewport| {
                                        (viewport.to_rgba(), viewport.width, viewport.height)
                                    },
                                )
                            })
                            .collect::<io::Result<Vec<_>>>()
                    });
                    match (
                        summarize_return_to_view_preview(&assets.strips, &assets.script),
                        frames,
                    ) {
                        (Ok(preview_summary), Ok(rendered_frames)) => {
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
    if display_state.message.is_empty() {
        display_state.message = fallback.to_string();
    }
    let input_echo = visual_line_prompt_active(&display_state).then_some(input_line);
    let system = render_play_text_window_system(&display_state, active_cursor, input_echo);
    if stats_panel_active_cursor_visible(state, active_cursor) {
        state.active_player = None;
    }
    let mut rgba = render_text_window_rgba(&system, font)
        .unwrap_or_else(|_| vec![0; TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4]);
    apply_endgame_page_transition_mask(&mut rgba, &display_state);
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
    if display_state.message.is_empty() {
        display_state.message = fallback.to_string();
    }
    let input_echo = visual_line_prompt_active(&display_state).then_some(input_line);
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
    paint_message_text_window(&mut system, &display_state.message);
    paint_stats_panel_text_window(&mut system, &display_state, active_cursor);
    if let Some(input_echo) = input_echo {
        let cursor_glyph = prompt_cursor_visible.then_some(PROMPT_CURSOR_GLYPH);
        paint_prompt_text_window_with_cursor(&mut system, input_echo, cursor_glyph);
    }
    system.set_active_window(MAIN_TEXT_WINDOW_INDEX);
    if stats_panel_active_cursor_visible(state, active_cursor) {
        state.active_player = None;
    }
    let mut rgba = render_text_window_rgba(&system, font)
        .unwrap_or_else(|_| vec![0; TEXT_WINDOW_RENDER_WIDTH * TEXT_WINDOW_RENDER_HEIGHT * 4]);
    apply_endgame_page_transition_mask(&mut rgba, &display_state);
    rgba
}

fn apply_endgame_page_transition_mask(rgba: &mut [u8], state: &PlayState) {
    let Some(transition) = state
        .endgame
        .as_ref()
        .and_then(|endgame| endgame.cinematic.page_transition)
    else {
        return;
    };
    apply_rect_column_sweep_mask_rgba(
        rgba,
        TEXT_WINDOW_RENDER_WIDTH,
        TEXT_WINDOW_RENDER_HEIGHT,
        transition,
    );
}

fn apply_rect_column_sweep_mask_rgba(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    transition: RectColumnSweepTransition,
) {
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
            if x >= revealed_start && x <= revealed_end {
                continue;
            }
            let offset = (y * width + x) * 4;
            if let Some(pixel) = rgba.get_mut(offset..offset + 4) {
                pixel.copy_from_slice(&[0x00, 0x00, 0x00, 0xff]);
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
    let _ = state.idle_tick();
    true
}

fn advance_visual_wait_frame(state: &mut PlayState, prompt_cursor_visible: &mut bool) -> bool {
    if visual_line_prompt_active(state) {
        *prompt_cursor_visible = !*prompt_cursor_visible;
        true
    } else if advance_visual_endgame_page_transition(state) {
        *prompt_cursor_visible = false;
        true
    } else {
        *prompt_cursor_visible = false;
        visual_idle_tick(state)
    }
}

fn advance_visual_endgame_page_transition(state: &mut PlayState) -> bool {
    state
        .endgame
        .as_mut()
        .is_some_and(|endgame| endgame.cinematic.advance_page_transition_title_tick())
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
    use u5_runtime::shop_runtime::{GuildShopState, ReagentShopState, TavernState};
    use u5_runtime::shop_session::ActiveShopSession;
    use u5_runtime::test_fixtures::{
        debug_game_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
        saved_game_seed_bytes, synthetic_tile_atlas, test_state, world_state,
    };
    use u5_runtime::tlk_control_codes::TLK_TEXT_XOR_MASK;
    use u5_runtime::{
        Area, BRIT_OOL_FILENAME, CH_FONT_LEN, COMBAT_ARENA_SIDE, DEFAULT_GAME_DIR, Direction,
        EGA_PALETTE_RGB, GuildShop, Herbalist, IBM_CH_FILE, INIT_GAM_FILENAME, INIT_OOL_FILENAME,
        OOL_PLANE_LEN, PenStroke, REAGENT_COUNT, REAGENT_SPIDER_SILK, SAVE_CHARACTER_DEX_OFFSET,
        SAVE_CHARACTER_GENDER_OFFSET, SAVE_CHARACTER_INT_OFFSET, SAVE_CHARACTER_NAME_LEN,
        SAVE_CHARACTER_STR_OFFSET, SAVE_ROSTER_OFFSET, SAVED_GAM_FILENAME, SAVED_OOL_FILENAME,
        SHRINE_TABLE_FILE, STORY_DAT_FILE, ShrineVirtue, SurfaceChestVerb, TILES_EGA_FILE, Tavern,
        TileGraphicsDepth, U4_TRANSFER_U5_SEED_GAM_FILENAME, U4TransferSource, WorldPlane,
        dungeon_cell_index, parse_ch_font, world_cell_index, wrap_text_panel_lines,
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
    fn intro_menu_frame_renders_nonblank_rgba() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
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
            title_signature_progress: 0,
            title_signature_complete: true,
            title_tick_frame: 0,
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
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
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
    fn intro_title_art_composition_clears_lower_band_then_draws_remaining_slots() {
        let blank = MonochromeBitmap {
            width: 1,
            height: 1,
            pixels: vec![0],
        };
        let mut blocks = vec![blank; 10];
        blocks[6] = MonochromeBitmap {
            width: 1,
            height: 24,
            pixels: vec![1; 24],
        };
        blocks[7] = MonochromeBitmap {
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

        let rgba = compose_intro_title_art_rgba(&title, &british);
        let width = TITLE_SURFACE_WIDTH as usize;

        assert_eq!(rgba.len(), width * (TITLE_SURFACE_HEIGHT as usize) * 4);
        assert_eq!(rgba_pixel(&rgba, width, 20, 139), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 20, 140), [0, 0, 0, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 108, 140), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 24, 66), [0xff, 0xff, 0xff, 0xff]);
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
    fn title_tick_overlay_stays_inside_spec_strip_and_preserves_title_pixels() {
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
            [0xff, 0xff, 0xff, 0xff]
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
    fn title_tick_flame_stripe_uses_published_palette_cycle() {
        // `cleak/u5-spec#52`: the clean replacement is a three-flame
        // upward-tapering stripe. It uses the published bright/dim
        // palette pairs while leaving pixels outside the silhouette alone.
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
    fn endgame_page_transition_mask_reveals_columns_from_left_edge() {
        let width = 8;
        let height = 4;
        let mut rgba = vec![0xff; width * height * 4];
        let transition = RectColumnSweepTransition {
            rect: (2, 1, 6, 2),
            tick: 1,
        };

        apply_rect_column_sweep_mask_rgba(&mut rgba, width, height, transition);

        assert_eq!(rgba_pixel(&rgba, width, 2, 1), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 3, 2), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 4, 1), [0x00, 0x00, 0x00, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 6, 2), [0x00, 0x00, 0x00, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 7, 1), [0xff, 0xff, 0xff, 0xff]);
        assert_eq!(rgba_pixel(&rgba, width, 4, 3), [0xff, 0xff, 0xff, 0xff]);
    }

    #[test]
    fn visual_wait_frame_advances_endgame_page_transition_during_modal_hold() {
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
        endgame.cinematic.advance();
        assert_eq!(
            endgame
                .cinematic
                .page_transition
                .map(|transition| transition.tick),
            Some(0)
        );

        let mut prompt_cursor_visible = true;
        assert!(advance_visual_wait_frame(
            &mut state,
            &mut prompt_cursor_visible
        ));

        assert!(!prompt_cursor_visible);
        assert_eq!(
            state
                .endgame
                .as_ref()
                .and_then(|endgame| endgame.cinematic.page_transition)
                .map(|transition| transition.tick),
            Some(1)
        );
    }

    #[test]
    fn visual_play_frame_uses_full_endgame_surface_without_viewport_blit() {
        let font = parse_ch_font(&vec![0xff; CH_FONT_LEN], IBM_CH_FILE).unwrap();
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.enter_endgame();

        let mut expected_state = state.clone();
        let expected = render_status_framebuffer(&mut expected_state, "", READY_HINT, &font);
        let rgba = render_visual_play_frame(&mut state, &atlas, &font);

        assert_eq!(rgba, expected);
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
        assert_eq!(reports.len(), if has_story { 17 } else { 16 });
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
            "combat-play",
            "surface-view-overlay",
            "dungeon-view-overlay",
            "britannia-chunk-map-overlay",
            "peer-view-overlay",
            "x-ray-view-overlay",
            "z-stats-modal",
            "endgame-status",
        ] {
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected visual gameplay report");
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
        assert!(manifest.contains("combat-play"));
        assert!(manifest.contains("surface-view-overlay"));
        assert!(manifest.contains("dungeon-view-overlay"));
        assert!(manifest.contains("britannia-chunk-map-overlay"));
        assert!(manifest.contains("peer-view-overlay"));
        assert!(manifest.contains("x-ray-view-overlay"));
        assert!(manifest.contains("z-stats-modal"));
        assert!(manifest.contains("endgame-status"));
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

        assert_eq!(cases.len(), 31);
        assert!(cases.iter().all(|case| !case.script.is_empty()));
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
                .any(|case| case.label == "route-britannia-look")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-britannia-spyglass-chunk-map")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-save-refusal")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-world-board-horse")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-ship-broadside-fire")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-movement-search")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-heavy-door-variant-block")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-ignite-torch")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-dungeon-exit-refusal")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-shop-sage-topic-miss")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-britannia-blink-east-ray")
        );
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
                .any(|case| case.label == "route-shop-horse-trader-horse-and-rider-buy")
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
                .any(|case| case.label == "route-shop-sage-topic-short-funds")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-fountain-look")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-yew-wanted-poster-look")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-buccaneers-den-wishing-well")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-castle-death-vision-look")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-trigger")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-pass")
        );
        assert!(
            cases
                .iter()
                .any(|case| case.label == "route-doom-combat-attack")
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
            visual_route_step_label("route-shop-horse-trader-stablehouse-buy", 2, "Y"),
            "route-shop-horse-trader-stablehouse-buy-02-y"
        );
    }

    #[test]
    fn visual_route_suite_local_clean_writes_per_step_pngs_when_present() {
        let game_dir = Path::new(DEFAULT_GAME_DIR);
        if !game_dir.join("CASTLE.DAT").exists()
            || !game_dir.join(TILES_EGA_FILE).exists()
            || !game_dir.join(IBM_CH_FILE).exists()
        {
            return;
        }

        let dir = temp_output_dir("routes");
        let reports = visual_route_suite(game_dir, TileGraphicsDepth::Ega16, &dir).unwrap();

        assert_eq!(reports.len(), 83);
        for report in &reports {
            assert!(report.path.exists());
            assert_eq!(report.width, VISUAL_PLAY_FRAME_WIDTH);
            assert_eq!(report.height, VISUAL_PLAY_FRAME_HEIGHT);
            assert!(report.nonblack_pixels > 0);
        }
        let manifest = fs::read_to_string(dir.join("manifest.txt")).unwrap();
        assert!(manifest.contains("route-world-movement-00-initial"));
        assert!(manifest.contains("route-world-movement-01-d"));
        assert!(manifest.contains("route-town-status-modal-01-z"));
        assert!(manifest.contains("route-town-view-overlay-01-v"));
        assert!(manifest.contains("route-britannia-look-01-l6"));
        assert!(manifest.contains("route-britannia-spyglass-chunk-map-01-usp"));
        assert!(manifest.contains("route-castle-save-refusal-02-n"));
        assert!(manifest.contains("route-world-board-horse-01-b"));
        assert!(manifest.contains("route-ship-broadside-fire-01-f6"));
        assert!(manifest.contains("route-dungeon-movement-search-03-s6"));
        assert!(manifest.contains("route-dungeon-heavy-door-variant-block-01-idle"));
        assert!(manifest.contains("route-dungeon-ignite-torch-01-i"));
        assert!(manifest.contains("route-dungeon-exit-refusal-02-n"));
        assert!(manifest.contains("route-shop-sage-topic-miss-01-mantra"));
        assert!(manifest.contains("route-britannia-blink-east-ray-01-c1ip6"));
        assert!(manifest.contains("route-castle-poison-gas-step-01-d"));
        assert!(manifest.contains("route-shop-inn-rest-accept-02-y"));
        assert!(manifest.contains("route-shop-horse-trader-horse-and-rider-buy-02-y"));
        assert!(manifest.contains("route-shop-horse-trader-stablehouse-buy-02-y"));
        assert!(manifest.contains("route-shop-horse-trader-wishing-well-buy-02-y"));
        assert!(manifest.contains("route-shop-sage-topic-paid-success-02-y"));
        assert!(manifest.contains("route-shop-sage-topic-short-funds-02-y"));
        assert!(manifest.contains("route-castle-fountain-look-02-1"));
        assert!(manifest.contains("route-yew-wanted-poster-look-01-l6"));
        assert!(manifest.contains("route-buccaneers-den-wishing-well-03-horse"));
        assert!(manifest.contains("route-castle-death-vision-look-02-1"));
        assert!(manifest.contains("route-doom-combat-trigger-01-empty"));
        assert!(manifest.contains("route-doom-combat-pass-02-empty"));
        assert!(manifest.contains("route-doom-combat-attack-02-a6"));
        assert!(manifest.contains("route-doom-combat-board-refusal-02-b"));
        assert!(manifest.contains("route-doom-combat-z-stats-02-z"));
        assert!(manifest.contains("route-doom-combat-search-prompt-02-s"));
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
        let clock_before = state.clock;

        assert!(visual_idle_tick(&mut state));

        assert_eq!(state.turn, 0);
        assert_eq!(state.clock, clock_before);
        assert_eq!(state.animation.frame, 1);
        assert!(state.message.contains("Idle animation tick."));
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
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
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
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
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
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
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
    fn return_to_view_intro_frame_overlays_preview_rgba() {
        let preview_rgba = vec![
            0xff, 0x00, 0x00, 0xff, 0x00, 0xff, 0x00, 0xff, 0x00, 0x00, 0xff, 0xff, 0xff, 0xff,
            0xff, 0xff,
        ];
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            title_signature_progress: 0,
            title_signature_complete: false,
            title_tick_frame: 0,
            message: String::new(),
            panel: VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_frames_rgba: vec![preview_rgba],
                preview_frame_index: 0,
                preview_width: 2,
                preview_height: 2,
            },
            launch_result: Arc::new(Mutex::new(None)),
            image_handle: None,
        };

        let frame = render_intro_frame(&mut intro);
        let x = ((INTRO_FRAMEBUFFER_WIDTH as usize) - 2) / 2;
        let offset = ((18 * INTRO_FRAMEBUFFER_WIDTH as usize) + x) * 4;

        assert_eq!(&frame[offset..offset + 4], &[0xff, 0x00, 0x00, 0xff]);
        let _ = fs::remove_dir_all(&intro.game_dir);
    }

    #[test]
    fn return_to_view_intro_animation_advances_preview_frames_until_final() {
        let mut panel = VisualIntroPanel::ReturnToView {
            summary: "Preview".to_string(),
            preview_frames_rgba: vec![vec![0x00, 0x00, 0x00, 0xff], vec![0xff, 0xff, 0xff, 0xff]],
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
}

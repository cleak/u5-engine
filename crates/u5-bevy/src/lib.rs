//! Bevy-backed visual harness. The gameplay source of truth stays in
//! [`PlayState`]; this module only owns the window, the CPU framebuffer, and
//! the Bevy texture handle. Input dispatch reuses the terminal-mode handler so
//! movement, doors, transitions, and other supported behavior come along for
//! free.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::sprite::Anchor;
use bevy::text::TextBounds;

use u5_runtime::{
    Area, ChargenSession, ChargenSessionResult, ChargenSessionStep, Direction,
    INTRO_INLINE_DOORWAY_STEP, INTRO_STORY_STEP_COUNT, IntroStoryArtPlacement, MISCMAPS_DAT_FILE,
    MISCMAPS_RTV_COMMAND_SECTION_OFFSET, MISCMAPS_RTV_STRIP_SECTION_BYTES,
    MISCMAPS_RTV_STRIP_SECTION_OFFSET, PLAY_MUSIC_TOGGLE_KEY, PlayInputDisposition, PlayOptions,
    PlayState, RTV_COMMAND_STREAM_BYTES, StoryRecords, TILE_ATLAS_SIDE, TileAtlas,
    TileGraphicsDepth, U4TransferOverrides, U4TransferSource, commit_chargen_save,
    commit_u4_transfer_save, handle_play_key_input,
    intro_menu::{IntroSubflow, IntroSubflowResult},
    intro_step_has_story6_secondary_pass, intro_step_transition_strips,
    intro_story_art_file_for_step, intro_story_art_placement_for_step,
    intro_story_step_waits_for_input, intro_story6_secondary_subimage, load_play_options_from_save,
    load_question_records, load_return_to_view_assets, load_story_records, load_tile_atlas,
    menu_dispatch::{UnifiedMenuDispatch, UnifiedMenuStep},
    read_u4_transfer_source_from_party_sav, render_return_to_view_preview_viewport,
    render_text_panel_rgba,
    shop_runtime::{GuildShopState, ReagentShopState, SageState, TavernState},
    shop_session::ActiveShopSession,
    summarize_return_to_view_preview, summarize_return_to_view_script,
    u4_transfer_session::{U4TransferPreview, u4_transfer_preview_from_u4_values},
};

const VIEWPORT_RADIUS: usize = 5;
const VIEWPORT_CELLS: usize = VIEWPORT_RADIUS * 2 + 1;
const VIEWPORT_SIZE_PX: u32 = (VIEWPORT_CELLS * TILE_ATLAS_SIDE) as u32;
const DISPLAY_SCALE: f32 = 3.0;
const STATUS_PANEL_HEIGHT: f32 = 260.0;
const STATUS_PANEL_PADDING: f32 = 8.0;
const STATUS_FONT_SIZE: f32 = 9.0;

const READY_HINT: &str =
    "WASD/arrows: move. Shift+A attacks, Shift+S searches. Ctrl+S music. Esc quit.";
const INTRO_FRAMEBUFFER_WIDTH: u32 = 320;
const INTRO_FRAMEBUFFER_HEIGHT: u32 = 220;
const INTRO_DISPLAY_SCALE: f32 = 2.5;

pub fn run_visual_loop(
    game_dir: &Path,
    options: PlayOptions,
    raster_depth: TileGraphicsDepth,
) -> std::io::Result<()> {
    let state = PlayState::load_scene(game_dir, options)?;
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    let bootstrap = Bootstrap {
        game_dir: game_dir.to_path_buf(),
        state,
        atlas,
    };

    let display_w = VIEWPORT_SIZE_PX as f32 * DISPLAY_SCALE;
    let display_h = display_w + STATUS_PANEL_HEIGHT;

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
        .add_systems(Update, (drive_visual_intro, screenshot_system))
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
    mut text_query: Query<&mut Text2d, With<StatusText>>,
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
            let rgba = render_framebuffer(&mut v.state, &v.atlas);
            if let Some(image) = images.get_mut(&v.image_handle) {
                image.data = Some(rgba);
            }
            if let Ok(mut text) = text_query.single_mut() {
                text.0 = summarize(&mut v.state, "", &v.input_line);
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
}

#[derive(Resource)]
struct PendingBootstrap(Mutex<Option<Bootstrap>>);

#[derive(Resource)]
struct VisualState {
    game_dir: PathBuf,
    state: PlayState,
    atlas: TileAtlas,
    image_handle: Handle<Image>,
    input_line: String,
}

#[derive(Component)]
struct StatusText;

#[derive(Resource)]
struct VisualIntroState {
    game_dir: PathBuf,
    raster_depth: TileGraphicsDepth,
    dispatch: UnifiedMenuDispatch,
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
    },
    Acknowledgements,
    ReturnToView {
        summary: String,
        preview_rgba: Option<Vec<u8>>,
        preview_width: usize,
        preview_height: usize,
    },
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
        visual.state.animation.tick_static_tiles();
        advanced = true;
    }
    if !advanced || !visual.state.viewport_has_animated_tiles(VIEWPORT_RADIUS) {
        // Tick still consumed (state advances even when nothing visible
        // animates so re-entering a water scene picks up at the right
        // phase), but skip the framebuffer re-blit.
        return;
    }
    let v: &mut VisualState = visual.as_mut();
    let rgba = render_framebuffer(&mut v.state, &v.atlas);
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
    } = bootstrap;

    let rgba = render_framebuffer(&mut state, &atlas);
    let mut image = Image::new(
        Extent3d {
            width: VIEWPORT_SIZE_PX,
            height: VIEWPORT_SIZE_PX,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        rgba,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    );
    image.sampler = ImageSampler::nearest();
    let image_handle = images.add(image);

    let display_size = VIEWPORT_SIZE_PX as f32 * DISPLAY_SCALE;

    commands.spawn(Camera2d);
    commands.spawn((
        Sprite {
            image: image_handle.clone(),
            custom_size: Some(Vec2::splat(display_size)),
            ..default()
        },
        Transform::from_xyz(0.0, STATUS_PANEL_HEIGHT * 0.5, 0.0),
    ));
    commands.spawn((
        Text2d::new(summarize(&mut state, READY_HINT, "")),
        TextFont {
            font_size: STATUS_FONT_SIZE,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(JustifyText::Center),
        TextBounds::new(
            display_size - STATUS_PANEL_PADDING * 2.0,
            STATUS_PANEL_HEIGHT - STATUS_PANEL_PADDING * 2.0,
        ),
        Anchor::TopCenter,
        Transform::from_xyz(
            0.0,
            -display_size * 0.5 + STATUS_PANEL_HEIGHT * 0.5 - STATUS_PANEL_PADDING,
            0.0,
        ),
        StatusText,
    ));

    commands.insert_resource(VisualState {
        game_dir,
        state,
        atlas,
        image_handle,
        input_line: String::new(),
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
        VisualIntroPanel::Story { step, .. } => {
            if *step + 1 < INTRO_STORY_STEP_COUNT {
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
                intro.panel = VisualIntroPanel::Story { records, step: 0 };
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
            let (summary, preview_rgba, preview_width, preview_height) =
                visual_return_to_view_summary(&intro.game_dir, intro.raster_depth);
            intro.panel = VisualIntroPanel::ReturnToView {
                summary,
                preview_rgba,
                preview_width,
                preview_height,
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
    mut text_query: Query<&mut Text2d, With<StatusText>>,
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
    let rgba = render_framebuffer(&mut v.state, &v.atlas);
    if let Some(image) = images.get_mut(&v.image_handle) {
        image.data = Some(rgba);
    }
    if let Ok(mut text) = text_query.single_mut() {
        let summary = summarize(&mut v.state, "", &v.input_line);
        text.0 = summary;
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
        VisualIntroPanel::Story { records, step } => {
            return summarize_intro_story(records, *step);
        }
        VisualIntroPanel::Acknowledgements => {
            return [
                "Acknowledgements".to_string(),
                String::new(),
                "This intro submenu is self-contained and returns to the main menu.".to_string(),
                "The clean specification does not transcribe the exact acknowledgement text."
                    .to_string(),
                String::new(),
                "Press any key to return to the intro menu.".to_string(),
            ]
            .join("\n");
        }
        VisualIntroPanel::ReturnToView { summary, .. } => {
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
                "The preview above is rendered from the dry-run Return-to-View state.".to_string(),
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
    let mut rgba = render_text_panel_rgba(
        &summarize_intro(intro),
        INTRO_FRAMEBUFFER_WIDTH as usize,
        INTRO_FRAMEBUFFER_HEIGHT as usize,
    )
    .unwrap_or_else(|_| {
        vec![0; (INTRO_FRAMEBUFFER_WIDTH as usize) * (INTRO_FRAMEBUFFER_HEIGHT as usize) * 4]
    });
    if let VisualIntroPanel::ReturnToView {
        preview_rgba: Some(preview_rgba),
        preview_width,
        preview_height,
        ..
    } = &intro.panel
    {
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

fn visual_return_to_view_summary(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
) -> (String, Option<Vec<u8>>, usize, usize) {
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
                    let preview = load_tile_atlas(game_dir, raster_depth).and_then(|atlas| {
                        render_return_to_view_preview_viewport(
                            &assets.strips,
                            &assets.script,
                            &atlas,
                        )
                    });
                    match (
                        summarize_return_to_view_preview(&assets.strips, &assets.script),
                        preview,
                    ) {
                        (Ok(preview_summary), Ok((viewport, _report))) => (
                            format!("{header} {script_summary} {preview_summary}"),
                            Some(viewport.to_rgba()),
                            viewport.width,
                            viewport.height,
                        ),
                        (Ok(preview_summary), Err(err)) => (
                            format!(
                                "{header} {script_summary} {preview_summary} Render error: {err}"
                            ),
                            None,
                            0,
                            0,
                        ),
                        (Err(err), _) => (
                            format!("{header} {script_summary} Dry-run error: {err}"),
                            None,
                            0,
                            0,
                        ),
                    }
                }
                Ok(None) => (
                    format!("{MISCMAPS_DAT_FILE} is missing; preview cannot run."),
                    None,
                    0,
                    0,
                ),
                Err(err) => (format!("{header} Script error: {err}"), None, 0, 0),
            }
        }
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => (
            format!("{MISCMAPS_DAT_FILE} is missing; preview cannot run."),
            None,
            0,
            0,
        ),
        Err(err) => (format!("Return-to-View preview error: {err}"), None, 0, 0),
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
        Ok(Some(viewport)) => viewport.to_rgba(),
        _ => render_text_panel_rgba(
            &state.render_text_view(VIEWPORT_RADIUS),
            VIEWPORT_SIZE_PX as usize,
            VIEWPORT_SIZE_PX as usize,
        )
        .unwrap_or_else(|_| vec![0; (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4]),
    }
}

fn summarize(state: &mut PlayState, fallback: &str, input_line: &str) -> String {
    let dungeon_note = if matches!(state.area, Area::Dungeon { .. }) {
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
        Direction::name(state.player.facing),
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
        || state.active_direction_prompt.is_some()
        || state.active_yes_no_prompt.is_some()
        || state.active_shop.is_some()
        || state.pending_moongate.is_some()
        || state.pending_town_arrest.is_some()
        || state.endgame.is_some()
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
    use KeyCode::*;
    if control_pressed {
        return Ok(None);
    }
    match key {
        Escape => {
            input_line.clear();
            handle_play_key_input(state, '\u{1b}', "", game_dir).map(Some)
        }
        Enter | NumpadEnter => {
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
        Backspace => {
            input_line.pop();
            Ok(Some(PlayInputDisposition::Continue))
        }
        _ => {
            if let Some(ch) = key_code_to_line_char(key, shift_pressed) {
                input_line.push(ch);
                Ok(Some(PlayInputDisposition::Continue))
            } else {
                Ok(None)
            }
        }
    }
}

fn key_code_to_line_char(key: KeyCode, shift_pressed: bool) -> Option<char> {
    use KeyCode::*;
    let ch = match key {
        KeyA => letter_for_shift('a', shift_pressed),
        KeyB => letter_for_shift('b', shift_pressed),
        KeyC => letter_for_shift('c', shift_pressed),
        KeyD => letter_for_shift('d', shift_pressed),
        KeyE => letter_for_shift('e', shift_pressed),
        KeyF => letter_for_shift('f', shift_pressed),
        KeyG => letter_for_shift('g', shift_pressed),
        KeyH => letter_for_shift('h', shift_pressed),
        KeyI => letter_for_shift('i', shift_pressed),
        KeyJ => letter_for_shift('j', shift_pressed),
        KeyK => letter_for_shift('k', shift_pressed),
        KeyL => letter_for_shift('l', shift_pressed),
        KeyM => letter_for_shift('m', shift_pressed),
        KeyN => letter_for_shift('n', shift_pressed),
        KeyO => letter_for_shift('o', shift_pressed),
        KeyP => letter_for_shift('p', shift_pressed),
        KeyQ => letter_for_shift('q', shift_pressed),
        KeyR => letter_for_shift('r', shift_pressed),
        KeyS => letter_for_shift('s', shift_pressed),
        KeyT => letter_for_shift('t', shift_pressed),
        KeyU => letter_for_shift('u', shift_pressed),
        KeyV => letter_for_shift('v', shift_pressed),
        KeyW => letter_for_shift('w', shift_pressed),
        KeyX => letter_for_shift('x', shift_pressed),
        KeyY => letter_for_shift('y', shift_pressed),
        KeyZ => letter_for_shift('z', shift_pressed),
        Digit0 | Numpad0 => '0',
        Digit1 | Numpad1 => '1',
        Digit2 | Numpad2 => '2',
        Digit3 | Numpad3 => '3',
        Digit4 | Numpad4 => '4',
        Digit5 | Numpad5 => '5',
        Digit6 | Numpad6 => '6',
        Digit7 | Numpad7 => '7',
        Digit8 | Numpad8 => '8',
        Digit9 | Numpad9 => '9',
        Space => ' ',
        Minus => {
            if shift_pressed {
                '_'
            } else {
                '-'
            }
        }
        Equal => {
            if shift_pressed {
                '+'
            } else {
                '='
            }
        }
        BracketLeft => {
            if shift_pressed {
                '{'
            } else {
                '['
            }
        }
        BracketRight => {
            if shift_pressed {
                '}'
            } else {
                ']'
            }
        }
        Comma => {
            if shift_pressed {
                '<'
            } else {
                ','
            }
        }
        Period => {
            if shift_pressed {
                '>'
            } else {
                '.'
            }
        }
        _ => return None,
    };
    Some(ch)
}

fn letter_for_shift(lower: char, shift_pressed: bool) -> char {
    if shift_pressed {
        lower.to_ascii_uppercase()
    } else {
        lower
    }
}

fn key_code_to_char(key: KeyCode, shift_pressed: bool, control_pressed: bool) -> Option<char> {
    use KeyCode::*;
    if control_pressed {
        return match key {
            KeyS => Some(PLAY_MUSIC_TOGGLE_KEY),
            _ => None,
        };
    }

    if shift_pressed {
        let ch = match key {
            KeyA => 'A',
            KeyB => 'B',
            KeyC => 'C',
            KeyD => 'D',
            KeyE => 'E',
            KeyF => 'F',
            KeyG => 'G',
            KeyH => 'H',
            KeyI => 'I',
            KeyJ => 'J',
            KeyK => 'K',
            KeyL => 'L',
            KeyM => 'M',
            KeyN => 'N',
            KeyO => 'O',
            KeyP => 'P',
            KeyQ => 'Q',
            KeyR => 'R',
            KeyS => 'S',
            KeyT => 'T',
            KeyU => 'U',
            KeyV => 'V',
            KeyW => 'W',
            KeyX => 'X',
            KeyY => 'Y',
            KeyZ => 'Z',
            BracketLeft => '{',
            BracketRight => '}',
            Equal | NumpadAdd => '+',
            Minus => '_',
            NumpadSubtract => '-',
            Comma => '<',
            Period => '>',
            _ => return None,
        };
        return Some(ch);
    }

    let ch = match key {
        Escape => '\u{1b}',
        Enter | NumpadEnter => '\r',
        Backspace | NumpadBackspace => '\u{8}',
        KeyW | ArrowUp | Numpad8 => 'w',
        KeyA | ArrowLeft | Numpad4 => 'a',
        KeyS | ArrowDown | Numpad2 => 's',
        KeyD | ArrowRight | Numpad6 => 'd',
        Numpad7 => 'y',
        Numpad9 => 'u',
        Numpad1 => 'b',
        Numpad3 => 'n',
        Digit0 | Numpad0 => '0',
        Digit1 => '1',
        Digit2 => '2',
        Digit3 => '3',
        Digit4 => '4',
        Digit5 => '5',
        Digit6 => '6',
        Digit7 => '7',
        Digit8 => '8',
        Digit9 => '9',
        BracketLeft => '[',
        BracketRight => ']',
        Equal => '=',
        Minus | NumpadSubtract => '-',
        NumpadAdd => '+',
        KeyB => 'B',
        KeyC => 'C',
        KeyE => 'E',
        KeyF => 'F',
        KeyG => 'G',
        KeyH => 'H',
        KeyI => 'I',
        KeyJ => 'J',
        KeyK => 'K',
        KeyL => 'L',
        KeyM => 'M',
        KeyN => 'N',
        KeyO => 'O',
        KeyP => 'P',
        KeyQ => 'Q',
        KeyR => 'R',
        KeyT => 'T',
        KeyU => 'U',
        KeyV => 'V',
        KeyX => 'X',
        KeyY => 'Y',
        KeyZ => 'Z',
        Space => ' ',
        Comma => '<',
        Period => '>',
        _ => return None,
    };
    Some(ch)
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
        Area, BRIT_OOL_FILENAME, Direction, EGA_PALETTE_RGB, GuildShop, Herbalist,
        INIT_GAM_FILENAME, INIT_OOL_FILENAME, OOL_PLANE_LEN, REAGENT_COUNT, REAGENT_SPIDER_SILK,
        SAVE_CHARACTER_DEX_OFFSET, SAVE_CHARACTER_GENDER_OFFSET, SAVE_CHARACTER_INT_OFFSET,
        SAVE_CHARACTER_NAME_LEN, SAVE_CHARACTER_STR_OFFSET, SAVE_ROSTER_OFFSET, SAVED_GAM_FILENAME,
        SAVED_OOL_FILENAME, SHRINE_TABLE_FILE, ShrineVirtue, SurfaceChestVerb, Tavern,
        TileGraphicsDepth, U4_TRANSFER_U5_SEED_GAM_FILENAME, U4TransferSource, WorldPlane,
        dungeon_cell_index, world_cell_index, wrap_text_panel_lines,
    };

    fn enc_tlk_text(text: &str) -> Vec<u8> {
        text.bytes().map(|b| b ^ TLK_TEXT_XOR_MASK).collect()
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
    fn visual_key_map_keeps_wasd_movement_and_shift_command_conflicts() {
        assert_eq!(key_code_to_char(KeyCode::KeyW, false, false), Some('w'));
        assert_eq!(key_code_to_char(KeyCode::KeyA, false, false), Some('a'));
        assert_eq!(key_code_to_char(KeyCode::KeyS, false, false), Some('s'));
        assert_eq!(key_code_to_char(KeyCode::KeyD, false, false), Some('d'));
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

        let (summary, preview_rgba, preview_width, preview_height) =
            visual_return_to_view_summary(&dir, TileGraphicsDepth::Ega16);

        assert!(summary.contains(MISCMAPS_DAT_FILE));
        assert!(summary.contains("128 bytes"));
        assert!(summary.contains("Return-to-View strips"));
        assert!(preview_rgba.is_none());
        assert_eq!(preview_width, 0);
        assert_eq!(preview_height, 0);
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
    fn visual_intro_story_panel_pages_back_to_menu_after_final_step() {
        let mut intro = VisualIntroState {
            game_dir: debug_game_dir(),
            raster_depth: TileGraphicsDepth::Ega16,
            dispatch: UnifiedMenuDispatch::new(),
            message: String::new(),
            panel: VisualIntroPanel::Story {
                records: StoryRecords {
                    records: (0..20).map(|i| format!("Story record {i}")).collect(),
                },
                step: INTRO_STORY_STEP_COUNT - 1,
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
            message: String::new(),
            panel: VisualIntroPanel::ReturnToView {
                summary: "Preview".to_string(),
                preview_rgba: Some(preview_rgba),
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
}

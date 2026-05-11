//! Bevy-backed visual harness. The gameplay source of truth stays in
//! [`PlayState`]; this module only owns the window, the CPU framebuffer, and
//! the Bevy texture handle. Input dispatch reuses the terminal-mode handler so
//! movement, doors, transitions, and other supported behavior come along for
//! free.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

use u5_runtime::{
    Area, Direction, PlayInputDisposition, PlayOptions, PlayState, TILE_ATLAS_SIDE, TileAtlas,
    TileGraphicsDepth, handle_play_key_input, load_tile_atlas,
};

const VIEWPORT_RADIUS: usize = 5;
const VIEWPORT_CELLS: usize = VIEWPORT_RADIUS * 2 + 1;
const VIEWPORT_SIZE_PX: u32 = (VIEWPORT_CELLS * TILE_ATLAS_SIDE) as u32;
const DISPLAY_SCALE: f32 = 3.0;
const STATUS_PANEL_HEIGHT: f32 = 96.0;

const READY_HINT: &str = "WASD/arrows: move. E enter, O open, K klimb, Space pass, Z stats, Esc quit. < / > climb floors.";

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
    let screenshot_path: Option<PathBuf> = std::env::var("U5_BEVY_SCREENSHOT")
        .ok()
        .map(PathBuf::from);
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
                title: "Ultima V — first-playable visual slice".into(),
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
        .add_systems(Update, (drive_visual, screenshot_system))
        .run();

    Ok(())
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
                text.0 = summarize(&v.state, "");
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
}

#[derive(Component)]
struct StatusText;

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
        Text2d::new(summarize(&state, READY_HINT)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(JustifyText::Center),
        Transform::from_xyz(0.0, -display_size * 0.5, 0.0),
        StatusText,
    ));

    commands.insert_resource(VisualState {
        game_dir,
        state,
        atlas,
        image_handle,
    });
    commands.remove_resource::<PendingBootstrap>();
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
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
        return;
    }
    let mut handled = false;
    for key in keyboard.get_just_pressed() {
        let Some(ch) = key_code_to_char(*key) else {
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
        let summary = summarize(&v.state, "");
        text.0 = summary;
    }
}

fn render_framebuffer(state: &mut PlayState, atlas: &TileAtlas) -> Vec<u8> {
    match state.render_top_down_frame(VIEWPORT_RADIUS, atlas) {
        Ok(Some(viewport)) => viewport.to_rgba(),
        _ => placeholder_rgba(),
    }
}

fn placeholder_rgba() -> Vec<u8> {
    let pixel_count = (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize);
    let mut bytes = Vec::with_capacity(pixel_count * 4);
    for _ in 0..pixel_count {
        bytes.extend_from_slice(&[0x10, 0x10, 0x14, 0xff]);
    }
    bytes
}

fn summarize(state: &PlayState, fallback: &str) -> String {
    let dungeon_note = if matches!(state.area, Area::Dungeon { .. }) {
        " [Dungeon view is text-only in this slice.]"
    } else {
        ""
    };
    let msg = if state.message.is_empty() {
        fallback.to_string()
    } else {
        state.message.clone()
    };
    format!(
        "{} ({}, {}) facing {} — turn {}{}\n{}",
        state.current_area_label(),
        state.player.x,
        state.player.y,
        Direction::name(state.player.facing),
        state.turn,
        dungeon_note,
        msg
    )
}

fn key_code_to_char(key: KeyCode) -> Option<char> {
    use KeyCode::*;
    let ch = match key {
        KeyW | ArrowUp | Numpad8 => 'w',
        KeyA | ArrowLeft | Numpad4 => 'a',
        KeyS | ArrowDown | Numpad2 => 's',
        KeyD | ArrowRight | Numpad6 => 'd',
        Numpad7 => 'y',
        Numpad9 => 'u',
        Numpad1 => 'b',
        Numpad3 => 'n',
        KeyE => 'e',
        KeyO => 'o',
        KeyK => 'k',
        KeyZ => 'Z',
        Space => ' ',
        Comma => '<',
        Period => '>',
        _ => return None,
    };
    Some(ch)
}

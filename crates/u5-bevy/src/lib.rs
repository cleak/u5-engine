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
use bevy::text::TextBounds;

use u5_runtime::{
    Area, Direction, PlayInputDisposition, PlayOptions, PlayState, TILE_ATLAS_SIDE, TileAtlas,
    TileGraphicsDepth, handle_play_key_input, load_tile_atlas, render_text_panel_rgba,
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
        Text2d::new(summarize(&state, READY_HINT)),
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        TextLayout::new_with_justify(JustifyText::Center),
        TextBounds::new_horizontal(display_size - 16.0),
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
        _ => render_text_panel_rgba(
            &state.render_text_view(VIEWPORT_RADIUS),
            VIEWPORT_SIZE_PX as usize,
            VIEWPORT_SIZE_PX as usize,
        )
        .unwrap_or_else(|_| vec![0; (VIEWPORT_SIZE_PX as usize) * (VIEWPORT_SIZE_PX as usize) * 4]),
    }
}

fn summarize(state: &PlayState, fallback: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;
    use u5_runtime::test_fixtures::{dungeon_state, open_dungeon_record, synthetic_tile_atlas};
    use u5_runtime::{
        Direction, EGA_PALETTE_RGB, TileGraphicsDepth, dungeon_cell_index, wrap_text_panel_lines,
    };

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
}

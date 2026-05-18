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
use bevy::sprite::Anchor;
use bevy::text::TextBounds;

use u5_runtime::{
    Area, Direction, PLAY_MUSIC_TOGGLE_KEY, PlayInputDisposition, PlayOptions, PlayState,
    TILE_ATLAS_SIDE, TileAtlas, TileGraphicsDepth, handle_play_key_input, load_tile_atlas,
    render_text_panel_rgba, shop_runtime::SageState, shop_session::ActiveShopSession,
};

const VIEWPORT_RADIUS: usize = 5;
const VIEWPORT_CELLS: usize = VIEWPORT_RADIUS * 2 + 1;
const VIEWPORT_SIZE_PX: u32 = (VIEWPORT_CELLS * TILE_ATLAS_SIDE) as u32;
const DISPLAY_SCALE: f32 = 3.0;
const STATUS_PANEL_HEIGHT: f32 = 260.0;
const STATUS_PANEL_PADDING: f32 = 8.0;
const STATUS_FONT_SIZE: f32 = 14.0;

const READY_HINT: &str =
    "WASD/arrows: move. Shift+A attacks, Shift+S searches. Ctrl+S music. Esc quit.";

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
                text.0 = summarize(&mut v.state, "", &v.input_line);
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
    summary.push_str(&state.render_stats_panel_frame());
    if visual_line_prompt_active(state) {
        summary.push_str("\n> ");
        summary.push_str(input_line);
    }
    summary
}

fn visual_line_prompt_active(state: &PlayState) -> bool {
    state.active_conversation.is_some()
        || state.active_blackthorn.is_some()
        || state.active_shrine.is_some()
        || state.active_yell.is_some()
        || matches!(
            state.active_shop.as_ref(),
            Some(ActiveShopSession::Sage(SageState::Prompt { .. }))
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
    use u5_runtime::conversation_session::ConversationSession;
    use u5_runtime::test_fixtures::{
        debug_game_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid,
        synthetic_tile_atlas, test_state, world_state,
    };
    use u5_runtime::tlk_control_codes::TLK_TEXT_XOR_MASK;
    use u5_runtime::{
        Area, Direction, EGA_PALETTE_RGB, SHRINE_TABLE_FILE, ShrineVirtue, TileGraphicsDepth,
        WorldPlane, dungeon_cell_index, world_cell_index, wrap_text_panel_lines,
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
        assert!(summary.ends_with("\n> j"));
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
}

//! `--save-frame` driver: load PlayState, optionally replay a script,
//! render the current viewport, write it to a PNG, and exit.
//!
//! Useful for verifying visual output without a desktop session.

use std::io;
use std::path::Path;

use image::{ImageBuffer, Rgba};
use u5_runtime::{
    CHROME_RULE_INDEX, COMBAT_ARENA_SIDE, ChromeFonts, ChromePalette, DungeonScene, FixedCellFont,
    MESSAGE_WINDOW_RIGHT, PlayOptions, PlayState, PlayTarget, STATS_PANEL_TEXT_LEFT, Scene,
    TEXT_WINDOW_RENDER_HEIGHT, TEXT_WINDOW_RENDER_WIDTH, TILE_ATLAS_SIDE, TOWN_GRID_SIDE,
    LOCAL_VIEW_OVERLAY_ORIGIN_X, LOCAL_VIEW_OVERLAY_ORIGIN_Y, TextWindowSystem, TileGraphicsDepth,
    VIEWPORT_ORIGIN_X, VIEWPORT_ORIGIN_Y, ViewOverlayMode,
    WorldPlane, configure_play_text_windows, gameplay_chrome_content, hash_bytes, load_ibm_ch_font,
    load_runes_ch_font, load_tile_atlas, message_log_from_entries, paint_fixed_cell_glyph,
    paint_gameplay_frame_chrome, paint_message_line_cap, paint_stats_panel_text_window,
    prompt_cursor_glyph, render_text_panel_rgba,
};

use crate::{
    complete_headless_blocking_presentations, raster_frame_kind, replay_play_script_commands,
};

const VIEWPORT_RADIUS: usize = 5;
const VIEWPORT_CELLS: usize = VIEWPORT_RADIUS * 2 + 1;
const VIEWPORT_SIZE_PX: usize = VIEWPORT_CELLS * TILE_ATLAS_SIDE;
const OVERLAY_SIDE_PANEL_X: usize = STATS_PANEL_TEXT_LEFT as usize * 8;
const OVERLAY_SIDE_PANEL_Y: usize = 0;
const SURFACE_VIEW_CLASS_GALLERY_TILES: [u8; 17] = [
    0x00, 0x05, 0x09, 0x70, 0x1D, 0x10, 0x0D, 0x0C, 0x0B, 0x06, 0x60, 0xD4, 0x01, 0x04, 0xE0, 0xD8,
    0x20,
];

pub fn run_save_frame(
    game_dir: &Path,
    options: PlayOptions,
    raster_depth: TileGraphicsDepth,
    script: Option<Vec<String>>,
    out: &Path,
) -> io::Result<()> {
    let report = save_frame_capture(game_dir, options, raster_depth, script, out)?;
    println!(
        "Saved {}x{} {} to {} (player at ({}, {}) facing {}, turn {})",
        report.width,
        report.height,
        report.frame_kind,
        out.display(),
        report.player_x,
        report.player_y,
        report.facing,
        report.turn,
    );
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedFrameReport {
    pub label: String,
    pub path: std::path::PathBuf,
    pub width: u32,
    pub height: u32,
    pub frame_kind: &'static str,
    pub player_x: usize,
    pub player_y: usize,
    pub facing: &'static str,
    pub turn: u64,
    pub byte_hash: u64,
    pub nonblack_pixels: usize,
}

pub fn run_save_frame_suite(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out_dir: &Path,
) -> io::Result<()> {
    let reports = save_frame_suite(game_dir, raster_depth, out_dir)?;
    for report in &reports {
        println!(
            "suite {}: {}x{} {} hash {:016x} nonblack {} -> {}",
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
        "Saved frame suite: {} PNG(s) plus manifest at {}.",
        reports.len(),
        out_dir.join("manifest.txt").display()
    );
    Ok(())
}

pub fn save_frame_suite(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out_dir: &Path,
) -> io::Result<Vec<SavedFrameReport>> {
    let cases = save_frame_suite_cases();
    u5_runtime::test_fixtures::assert_writable_game_dir(out_dir, "save frame suite output");
    std::fs::create_dir_all(out_dir)?;
    let mut reports = Vec::with_capacity(cases.len());
    for case in cases {
        let out = out_dir.join(format!("{}.png", case.label));
        let report = save_frame_capture(
            game_dir,
            case.options,
            raster_depth,
            case.script
                .map(|commands| commands.iter().map(|cmd| (*cmd).to_string()).collect()),
            &out,
        )?;
        if report.nonblack_pixels == 0 {
            return Err(io::Error::other(format!(
                "frame suite `{}` produced an all-black PNG",
                case.label
            )));
        }
        reports.push(report);
    }
    for report in [
        save_frame_suite_combat(game_dir, raster_depth, &out_dir.join("combat.png"))?,
        save_frame_suite_surface_view(game_dir, raster_depth, &out_dir.join("surface-view.png"))?,
        save_frame_suite_dungeon_view(game_dir, raster_depth, &out_dir.join("dungeon-view.png"))?,
        save_frame_suite_peer_view(game_dir, raster_depth, &out_dir.join("peer-view.png"))?,
        save_frame_suite_x_ray_view(game_dir, raster_depth, &out_dir.join("x-ray-view.png"))?,
        save_frame_suite_surface_view_class_gallery(
            game_dir,
            raster_depth,
            ViewOverlayMode::GemView,
            &out_dir.join("surface-view-class-gallery.png"),
        )?,
        save_frame_suite_surface_view_class_gallery(
            game_dir,
            raster_depth,
            ViewOverlayMode::PeerSpell,
            &out_dir.join("peer-view-class-gallery.png"),
        )?,
        save_frame_suite_surface_view_class_gallery(
            game_dir,
            raster_depth,
            ViewOverlayMode::XRaySpell,
            &out_dir.join("x-ray-view-class-gallery.png"),
        )?,
        save_frame_suite_intro_menu(&out_dir.join("intro-menu.png"))?,
        save_frame_suite_status_window(game_dir, &out_dir.join("status-window.png"))?,
        save_frame_suite_z_stats(game_dir, &out_dir.join("z-stats-modal.png"))?,
        save_frame_suite_endgame(game_dir, &out_dir.join("endgame-status.png"))?,
    ] {
        if report.nonblack_pixels == 0 {
            return Err(io::Error::other(format!(
                "frame suite `{}` produced an all-black PNG",
                report.label
            )));
        }
        reports.push(report);
    }
    write_frame_suite_manifest(out_dir, &reports)?;
    Ok(reports)
}

#[derive(Clone)]
struct SaveFrameSuiteCase {
    label: &'static str,
    options: PlayOptions,
    script: Option<&'static [&'static str]>,
}

fn save_frame_suite_cases() -> Vec<SaveFrameSuiteCase> {
    vec![
        SaveFrameSuiteCase {
            label: "britannia",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                ..PlayOptions::default()
            },
            script: None,
        },
        SaveFrameSuiteCase {
            label: "britannia-step",
            options: PlayOptions {
                target: PlayTarget::World(WorldPlane::Britannia),
                start: Some((62, 124)),
                ..PlayOptions::default()
            },
            script: Some(&["d", "empty", "idle:1"]),
        },
        SaveFrameSuiteCase {
            label: "castle",
            options: PlayOptions {
                target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
                ..PlayOptions::default()
            },
            script: None,
        },
        SaveFrameSuiteCase {
            label: "dungeon",
            options: PlayOptions {
                target: PlayTarget::Dungeon(
                    DungeonScene::new(0x21).expect("dungeon scene is valid"),
                ),
                floor: 0,
                torch_counter: 9,
                ..PlayOptions::default()
            },
            script: Some(&["idle:1"]),
        },
    ]
}

fn save_frame_capture(
    game_dir: &Path,
    options: PlayOptions,
    raster_depth: TileGraphicsDepth,
    script: Option<Vec<String>>,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(game_dir, options)?;
    let atlas = load_tile_atlas(game_dir, raster_depth)?;

    if let Some(commands) = script {
        replay_play_script_commands(&mut state, game_dir, &commands, |_, _, _| Ok(()))?;
    }
    complete_headless_blocking_presentations(&mut state, Some(&atlas))?;

    save_frame_capture_state(state, &atlas, out)
}

fn save_frame_capture_state(
    state: PlayState,
    atlas: &u5_runtime::TileAtlas,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = state;
    let (width, height, rgba, frame_kind) = if state.active_view_overlay.is_some() {
        let mut rgba = render_text_panel_rgba(
            &state.render_text_window_frame(None),
            TEXT_WINDOW_RENDER_WIDTH,
            TEXT_WINDOW_RENDER_HEIGHT,
        )?;
        if let Some(base) = state.render_top_down_base_frame(VIEWPORT_RADIUS, atlas)? {
            blit_rgba(
                &mut rgba,
                TEXT_WINDOW_RENDER_WIDTH,
                TEXT_WINDOW_RENDER_HEIGHT,
                &base.to_rgba(),
                base.width,
                base.height,
                0,
                0,
            );
        }
        if let Some(overlay) = state.render_active_view_overlay(atlas.depth) {
            let (origin_x, origin_y) = match state.active_view_overlay.as_ref().map(|o| o.kind) {
                Some(
                    u5_runtime::ViewOverlayKind::Dungeon { .. }
                    | u5_runtime::ViewOverlayKind::Sky(_),
                ) => (VIEWPORT_ORIGIN_X, VIEWPORT_ORIGIN_Y),
                _ => (OVERLAY_SIDE_PANEL_X, OVERLAY_SIDE_PANEL_Y),
            };
            blit_rgba(
                &mut rgba,
                TEXT_WINDOW_RENDER_WIDTH,
                TEXT_WINDOW_RENDER_HEIGHT,
                &overlay.to_rgba(),
                overlay.width,
                overlay.height,
                origin_x,
                origin_y,
            );
        }
        (
            TEXT_WINDOW_RENDER_WIDTH as u32,
            TEXT_WINDOW_RENDER_HEIGHT as u32,
            rgba,
            "composed view overlay",
        )
    } else if state.combat_active {
        let mut rgba = render_text_panel_rgba(
            &state.render_text_window_frame(None),
            TEXT_WINDOW_RENDER_WIDTH,
            TEXT_WINDOW_RENDER_HEIGHT,
        )?;
        if let Some(combat) = state.render_top_down_base_frame(VIEWPORT_RADIUS, atlas)? {
            blit_rgba(
                &mut rgba,
                TEXT_WINDOW_RENDER_WIDTH,
                TEXT_WINDOW_RENDER_HEIGHT,
                &combat.to_rgba(),
                combat.width,
                combat.height,
                0,
                0,
            );
        }
        (
            TEXT_WINDOW_RENDER_WIDTH as u32,
            TEXT_WINDOW_RENDER_HEIGHT as u32,
            rgba,
            "composed combat frame",
        )
    } else if let Some(viewport) = save_screen_viewport(&mut state, atlas)? {
        // The viewport `--save-screen` composes: `endgame.md §3.1`'s tableau
        // rather than the dungeon the party left (playtest P15-07).
        (
            viewport.width as u32,
            viewport.height as u32,
            viewport.to_rgba(),
            raster_frame_kind(&state),
        )
    } else {
        (
            VIEWPORT_SIZE_PX as u32,
            VIEWPORT_SIZE_PX as u32,
            render_text_panel_rgba(
                &state.render_text_view(VIEWPORT_RADIUS),
                VIEWPORT_SIZE_PX,
                VIEWPORT_SIZE_PX,
            )?,
            "text panel",
        )
    };

    let byte_hash = hash_bytes(&rgba);
    let nonblack_pixels = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count();
    write_rgba_png(out, width, height, rgba)?;
    Ok(SavedFrameReport {
        label: out
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("frame")
            .to_string(),
        path: out.to_path_buf(),
        width,
        height,
        frame_kind,
        player_x: state.player.x,
        player_y: state.player.y,
        facing: u5_runtime::Direction::name(state.player.facing),
        turn: state.turn,
        byte_hash,
        nonblack_pixels,
    })
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

fn save_frame_suite_combat(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            start: Some((62, 124)),
            ..PlayOptions::default()
        },
    )?;
    state.combat_active = true;
    state.combat_terrain = [[5; COMBAT_ARENA_SIDE]; COMBAT_ARENA_SIDE];
    state.combat_terrain[0][0] = 12;
    state.combat_terrain[5][5] = 4;
    state.combat_terrain[6][5] = 1;
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    save_frame_capture_state(state, &atlas, out)
}

fn save_frame_suite_surface_view(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::World(WorldPlane::Britannia),
            ..PlayOptions::default()
        },
    )?;
    state.gems = 1;
    state.view_gem();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    save_frame_capture_state(state, &atlas, out)
}

fn save_frame_suite_dungeon_view(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::Dungeon(DungeonScene::new(0x21).expect("dungeon scene is valid")),
            floor: 0,
            torch_counter: 9,
            ..PlayOptions::default()
        },
    )?;
    state.gems = 1;
    state.view_gem();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    save_frame_capture_state(state, &atlas, out)
}

fn save_frame_suite_peer_view(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
            ..PlayOptions::default()
        },
    )?;
    state.activate_peer_view_overlay();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    save_frame_capture_state(state, &atlas, out)
}

fn save_frame_suite_x_ray_view(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
            ..PlayOptions::default()
        },
    )?;
    state.activate_x_ray_view_overlay();
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    save_frame_capture_state(state, &atlas, out)
}

fn save_frame_suite_surface_view_class_gallery(
    game_dir: &Path,
    raster_depth: TileGraphicsDepth,
    mode: ViewOverlayMode,
    out: &Path,
) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
            ..PlayOptions::default()
        },
    )?;
    seed_surface_view_class_gallery(&mut state, mode);
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    save_frame_capture_state(state, &atlas, out)
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
        ViewOverlayMode::SurfaceLook | ViewOverlayMode::SkyView => {
            unreachable!("surface view class gallery uses local surface-view modes")
        }
    }
}

fn save_frame_suite_intro_menu(out: &Path) -> io::Result<SavedFrameReport> {
    let text = [
        "Ultima V",
        "",
        "Intro Menu",
    ]
    .into_iter()
    .map(str::to_string)
    .chain(crate::intro_loop::intro_menu_text_rows())
    .chain(["", "Selection:"].into_iter().map(str::to_string))
    .collect::<Vec<_>>()
    .join("\n");
    save_text_window_report(out, "intro text window", &text, None)
}

fn save_frame_suite_status_window(game_dir: &Path, out: &Path) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(game_dir, PlayOptions::default())?;
    // A frame-suite fixture: this text exists to be rendered.
    state.message = "Status frame suite checkpoint.".to_string(); // audit: not a player-facing line
    let text = state.render_text_window_frame(None);
    save_text_window_report(out, "text window", &text, Some(&state))
}

fn save_frame_suite_z_stats(game_dir: &Path, out: &Path) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(
        game_dir,
        PlayOptions {
            target: PlayTarget::Town(Scene::new(0x11).expect("castle scene is valid")),
            ..PlayOptions::default()
        },
    )?;
    state.z_stats();
    let text = state.render_text_window_frame(None);
    save_text_window_report(out, "z-stats text window", &text, Some(&state))
}

fn save_frame_suite_endgame(game_dir: &Path, out: &Path) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(game_dir, PlayOptions::default())?;
    state.enter_endgame_from_game_dir(Some(game_dir))?;
    let text = state.render_text_window_frame(None);
    save_text_window_report(out, "text window", &text, Some(&state))
}

fn save_text_window_report(
    out: &Path,
    frame_kind: &'static str,
    text: &str,
    state: Option<&PlayState>,
) -> io::Result<SavedFrameReport> {
    let rgba = render_text_panel_rgba(text, TEXT_WINDOW_RENDER_WIDTH, TEXT_WINDOW_RENDER_HEIGHT)?;
    let byte_hash = hash_bytes(&rgba);
    let nonblack_pixels = rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0)
        .count();
    write_rgba_png(
        out,
        TEXT_WINDOW_RENDER_WIDTH as u32,
        TEXT_WINDOW_RENDER_HEIGHT as u32,
        rgba,
    )?;
    Ok(SavedFrameReport {
        label: out
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("frame")
            .to_string(),
        path: out.to_path_buf(),
        width: TEXT_WINDOW_RENDER_WIDTH as u32,
        height: TEXT_WINDOW_RENDER_HEIGHT as u32,
        frame_kind,
        player_x: state.map_or(0, |state| state.player.x),
        player_y: state.map_or(0, |state| state.player.y),
        facing: state.map_or("-", |state| {
            u5_runtime::Direction::name(state.player.facing)
        }),
        turn: state.map_or(0, |state| state.turn),
        byte_hash,
        nonblack_pixels,
    })
}

fn write_frame_suite_manifest(out_dir: &Path, reports: &[SavedFrameReport]) -> io::Result<()> {
    let mut manifest = String::new();
    manifest.push_str("# Ultima V frame suite manifest\n");
    manifest.push_str("# Sanitized: contains dimensions, frame kind, position, and hashes only.\n");
    for report in reports {
        manifest.push_str(&format!(
            "{}\t{}x{}\t{}\tturn {}\tat ({}, {}) facing {}\thash {:016x}\tnonblack {}\n",
            report.label,
            report.width,
            report.height,
            report.frame_kind,
            report.turn,
            report.player_x,
            report.player_y,
            report.facing,
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
                "framebuffer size did not match viewport dimensions",
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

/// The open prompt row `--save-screen` hands the layout: the state's own
/// prompt literal, or - as the Bevy shell does - the log's last row when that
/// row already owns the cursor, as the tavern's provision quantity question
/// does. Without the fallback this oracle drew a `>` command row under
/// `thou like?" 3` that the shell does not (playtest F49).
pub(crate) fn save_screen_open_prompt(
    state: &PlayState,
    log: &u5_runtime::GameplayMessageLog,
    flash_hold: bool,
) -> Option<String> {
    if flash_hold {
        return None;
    }
    state.open_prompt_line().or_else(|| {
        if !u5_runtime::open_row_owns_the_cursor(state) {
            return None;
        }
        log.lines()
            .last()
            .filter(|line| !line.text.is_empty())
            .map(|line| format!("{}{}", line.text, " ".repeat(line.trailing_spaces as usize)))
    })
}

/// The `--save-screen` live-row classification. It mirrors the Bevy shell's
/// message-window assembly, so a key wait that is not a command (the
/// endgame's greeting page) takes no end cap here either (playtest E66).
pub(crate) fn save_screen_live_row_kind(
    state: &PlayState,
    spell_echo_present: bool,
) -> u5_runtime::LiveRowKind {
    if spell_echo_present
        || u5_runtime::sage_topic_row_is_continuation(state)
        || u5_runtime::shop_pause_row_is_continuation(state)
        || u5_runtime::selector_prompt_row_is_continuation(state)
        || u5_runtime::dungeon_fountain_prompt_row_is_continuation(state)
        || state.defeated_party_row_is_continuation()
        || u5_runtime::overworld_prompt_pair_is_suppressed(state)
    {
        u5_runtime::LiveRowKind::Continuation
    } else if u5_runtime::endgame_page_row_is_key_wait(state) {
        u5_runtime::LiveRowKind::KeyWaitRow
    } else if u5_runtime::blackthorn_exit_key_wait_on_feed_row(state) {
        // `blackthorn.md §4.2` / §5 (u5-spec#299 Q9): the exit beat's key
        // wait after record `5`, two rows below the record's last row.
        u5_runtime::LiveRowKind::KeyWaitOnFeedRow
    } else {
        u5_runtime::LiveRowKind::CommandRow
    }
}

/// Where `--save-screen` paints the input cursor. A surface that reads no
/// key draws none - the Bevy shell's `message_window_cursor_suppressed`
/// test. Without it the fallback put the cursor after the endgame's
/// `"I see...` during its forty-tick pause, which reads no key
/// (`endgame.md §5.1`, playtest E66).
pub(crate) fn save_screen_cursor_cell(
    state: &PlayState,
    layout: &u5_runtime::MessageWindowLayout,
) -> Option<(u8, u8)> {
    if state.message_window_cursor_suppressed() {
        return None;
    }
    layout.inline_cursor.or_else(|| {
        layout.rows.last().map(|live| {
            (
                (live.column + live.glyphs.len().min(15) as u8).min(MESSAGE_WINDOW_RIGHT),
                live.row,
            )
        })
    })
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use u5_runtime::TILES_EGA_FILE;

    fn temp_output_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("u5-save-frame-{name}-{nonce}"))
    }

    /// Playtest J25: `inventory.md §4.4` (u5-spec#299 Q12) - under the leaked
    /// inverse the effect glyph comes out black on a light cell and each cap
    /// keeps its black triangle, so the slot cells are laid opaque, black
    /// pixels included, exactly as the Bevy shell lays them.
    #[test]
    fn save_screen_lays_the_leaked_effect_slot_cells_opaque() {
        use u5_runtime::test_fixtures::{open_grid, test_state};
        let mut state = test_state(open_grid(), 1, 1);
        state.active_effect_tag = Some(b'P');
        state.repaint_stats_panel();
        state.stats_panel_window_inverse = true;
        let slot = u5_runtime::stats_panel_opaque_effect_cell(&state).expect("leaked slot");
        let mut cells = vec![slot];
        cells.extend(u5_runtime::stats_panel_opaque_effect_cap_cells(&state));
        assert_eq!(cells.len(), 3);

        let (width, height) = (TEXT_WINDOW_RENDER_WIDTH, TEXT_WINDOW_RENDER_HEIGHT);
        let chrome = [0u8, 0, 170, 0xff];
        let mut rgba: Vec<u8> = chrome.repeat(width * height);
        // The text raster: each slot cell is white with one black pixel - the
        // glyph's stroke under the inverse.
        let mut text = vec![0u8; width * height * 4];
        let cell = u5_runtime::CH_CELL_SIDE;
        for &(column, row) in &cells {
            for y in 0..cell {
                for x in 0..cell {
                    let at =
                        ((usize::from(row) * cell + y) * width + usize::from(column) * cell + x)
                            * 4;
                    let pixel = if (x, y) == (3, 3) {
                        [0, 0, 0, 0xff]
                    } else {
                        [0xff; 4]
                    };
                    text[at..at + 4].copy_from_slice(&pixel);
                }
            }
        }
        overlay_panel_text_window_rgba(&mut rgba, &text, width, height, &state);
        for &(column, row) in &cells {
            let at = ((usize::from(row) * cell + 3) * width + usize::from(column) * cell + 3) * 4;
            assert_eq!(
                &rgba[at..at + 4],
                &[0, 0, 0, 0xff],
                "cell ({column}, {row}) keeps its black"
            );
        }

        // Without the leak the cells are not opaque: black lets chrome show.
        state.stats_panel_window_inverse = false;
        let mut plain: Vec<u8> = chrome.repeat(width * height);
        overlay_panel_text_window_rgba(&mut plain, &text, width, height, &state);
        let (column, row) = slot;
        let at = ((usize::from(row) * cell + 3) * width + usize::from(column) * cell + 3) * 4;
        assert_eq!(&plain[at..at + 4], &chrome);
    }

    /// Playtest E66: the `--save-screen` compositor follows the Bevy shell's
    /// message-window rules. `endgame.md §5.1`'s greeting page waits for a key
    /// that is not a command (a bare cursor, no end cap), and the waiting
    /// branch's `"I see...` lead-in is followed by a forty-tick pause that
    /// reads no key, so no cursor is drawn there.
    #[test]
    fn save_screen_follows_the_shell_on_endgame_key_waits_and_pauses() {
        use u5_runtime::test_fixtures::{debug_game_dir, dungeon_state, open_dungeon_record};
        let records: [&str; 11] = [
            "\nLord British says:\n\n\"Well met,\n",
            "\"Didst thou bring my box?\"\n\nYou reply: ",
            "\"The box!\"\n\n\"Didst thou bring it?\"\n\nYou reply: ",
            "Rite 1",
            "Rite 2\n\n",
            "Rite 3\n\n",
            "Rite 4\n\n",
            "Rite 5\n\n",
            "Rite 6\n\n",
            "Rite 7\n",
            "\n\"Pull up a chair.\"\n\n\"A while.\"\n",
        ];
        let mut bytes = Vec::new();
        for record in records {
            bytes.extend_from_slice(record.as_bytes());
            bytes.push(0);
        }
        let dir = debug_game_dir();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party_names = vec![*b"Avatar\0\0\0"];
        state.enter_endgame_with_messages(Some(
            u5_runtime::parse_endgame_messages(&bytes).unwrap(),
        ));
        state.endgame_entry_redraw_pending = true;
        for _ in 0..500 {
            if !state.advance_endgame_display_frame() {
                break;
            }
        }

        // The greeting page's key wait.
        state.flush_staged_narration();
        assert_eq!(
            save_screen_live_row_kind(&state, false),
            u5_runtime::LiveRowKind::KeyWaitRow
        );

        for key in ['y', 'n', 'n'] {
            u5_runtime::handle_play_key_input(&mut state, key, "", &dir).unwrap();
        }
        assert!(state.message.contains("I see..."), "{:?}", state.message);
        let layout = u5_runtime::MessageWindowLayout {
            rows: Vec::new(),
            inline_cursor: Some((30, 20)),
        };
        assert_eq!(save_screen_cursor_cell(&state, &layout), None);
        let _ = fs::remove_dir_all(dir);
    }

    /// Playtest K02. `view.md §4`: the local 32-by-32 View raster's cells
    /// "occupy the inclusive screen rectangle (32,32)..(159,159)" with a
    /// 24-pixel margin on every side of the viewport. `--save-screen` drew it
    /// flush at the viewport origin (8,8).
    #[test]
    fn save_screen_centres_the_local_view_overlay_in_the_viewport() {
        use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
        let mut state = test_state(open_grid(), 5, 5);
        state.gems = 1;
        state.view_gem();
        assert!(save_screen_local_view_overlay_active(&state));
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let overlay = save_screen_viewport(&mut state, &atlas).unwrap().unwrap();
        assert_eq!((overlay.width, overlay.height), (128, 128));

        let (width, height) = (TEXT_WINDOW_RENDER_WIDTH, TEXT_WINDOW_RENDER_HEIGHT);
        let mut rgba = [9u8, 9, 9, 0xff].repeat(width * height);
        blit_save_screen_viewport(&mut rgba, width, height, &overlay, true);

        let pixel = |x: usize, y: usize| &rgba[(y * width + x) * 4..(y * width + x) * 4 + 4];
        let source = overlay.to_rgba();
        let src = |x: usize, y: usize| &source[(y * 128 + x) * 4..(y * 128 + x) * 4 + 4];
        let lit = (0..128 * 128)
            .map(|index| (index % 128, index / 128))
            .find(|&(x, y)| src(x, y) != [0, 0, 0, 0xff])
            .expect("the overlay draws something");
        for (x, y) in [(0, 0), (127, 127), lit] {
            assert_eq!(pixel(32 + x, 32 + y), src(x, y), "overlay cell at ({x}, {y})");
        }
        // The 24-pixel margin is cleared, not left with what was beneath.
        assert_eq!(pixel(8, 8), &[0, 0, 0, 0xff]);
        assert_eq!(pixel(183, 183), &[0, 0, 0, 0xff]);
        // Outside the viewport the screen is untouched.
        assert_eq!(pixel(7, 7), &[9, 9, 9, 0xff]);
    }

    /// Playtest G32: `endgame.md §3.1` - "The throne-room tableau is the
    /// ordinary eleven-by-eleven world viewport". Entered from the Doom
    /// final room, the `--save-screen` viewport kept drawing the dungeon the
    /// party left; it now draws the tableau, as the visual shell does.
    #[test]
    fn save_screen_draws_the_endgame_tableau_not_the_dungeon_left_behind() {
        use u5_runtime::endgame_tableau_frame::{
            endgame_frame_shows_tableau, render_endgame_tableau_viewport,
        };
        use u5_runtime::test_fixtures::{dungeon_state, open_dungeon_record, synthetic_tile_atlas};
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party_names = vec![*b"Avatar\0\0\0"];
        state.enter_endgame_with_messages(None);
        state.endgame_entry_redraw_pending = true;
        for _ in 0..500 {
            if !state.advance_endgame_display_frame() {
                break;
            }
        }
        assert!(endgame_frame_shows_tableau(&state));
        assert!(state.endgame_absorption_arena_view().is_none());
        let atlas = synthetic_tile_atlas(u5_runtime::TileGraphicsDepth::Ega16);

        let drawn = save_screen_viewport(&mut state, &atlas)
            .unwrap()
            .expect("the tableau fills the viewport");
        let tableau = render_endgame_tableau_viewport(&state, &atlas).unwrap();
        assert_eq!((drawn.cells_wide, drawn.cells_high), (11, 11));
        assert!(drawn.pixels == tableau.pixels);
    }

    /// Playtest P15-07: `endgame.md §3.1` - the tableau "is the ordinary
    /// eleven-by-eleven world viewport". `--save-frame` labelled its capture
    /// "endgame tableau" but drew the Doom corridor the party left.
    #[test]
    fn save_frame_draws_the_endgame_tableau_not_the_dungeon_left_behind() {
        use u5_runtime::endgame_tableau_frame::render_endgame_tableau_viewport;
        use u5_runtime::test_fixtures::{dungeon_state, open_dungeon_record, synthetic_tile_atlas};
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        state.party_names = vec![*b"Avatar\0\0\0"];
        state.enter_endgame_with_messages(None);
        state.endgame_entry_redraw_pending = true;
        for _ in 0..500 {
            if !state.advance_endgame_display_frame() {
                break;
            }
        }
        let atlas = synthetic_tile_atlas(u5_runtime::TileGraphicsDepth::Ega16);
        let tableau = render_endgame_tableau_viewport(&state, &atlas).unwrap();
        let dir = temp_output_dir("p15-07");
        let report = save_frame_capture_state(state, &atlas, &dir.join("frame.png")).unwrap();
        assert_eq!(report.frame_kind, "endgame tableau");
        assert_eq!((report.width, report.height), (176, 176));
        assert_eq!(report.byte_hash, hash_bytes(&tableau.to_rgba()));
        let _ = fs::remove_dir_all(dir);
    }

    /// Playtest P15-08: `karma.md §8` - the Codex presentation "shows the
    /// eleven-by-eleven Codex map from `MISCMAPS.DAT`, record 2". The
    /// `--save-screen` and `--save-frame` viewport kept drawing the surface
    /// around `(233, 233)` while the reader waited.
    #[test]
    fn save_screen_draws_the_codex_presentation_map_not_the_surface() {
        use u5_runtime::shrine_presentation_frame::render_shrine_presentation_viewport;
        use u5_runtime::test_fixtures::{open_grid, synthetic_tile_atlas, test_state};
        let mut state = test_state(open_grid(), 5, 5);
        let atlas = synthetic_tile_atlas(TileGraphicsDepth::Ega16);
        let surface = save_screen_viewport(&mut state, &atlas).unwrap().unwrap();
        let mut tiles = vec![0x44u8; 121];
        tiles[2 * 11 + 5] = 0x41;
        state.shrine_presentation_map = Some(u5_runtime::MiscmapsCutsceneMap {
            record_index: 2,
            tiles,
        });
        let codex = render_shrine_presentation_viewport(&state, &atlas)
            .unwrap()
            .unwrap();
        let drawn = save_screen_viewport(&mut state, &atlas).unwrap().unwrap();
        assert_eq!((drawn.cells_wide, drawn.cells_high), (11, 11));
        assert!(drawn.pixels == codex.pixels);
        assert!(drawn.pixels != surface.pixels);

        let dir = temp_output_dir("p15-08");
        let report = save_frame_capture_state(state, &atlas, &dir.join("frame.png")).unwrap();
        assert_eq!(report.byte_hash, hash_bytes(&codex.to_rgba()));
        let _ = fs::remove_dir_all(dir);
    }

    /// Playtest F49: the tavern's provision quantity question keeps the
    /// cursor on its own row (`text-output.md §10.6`), so the oracle places
    /// no live command row under it - the Bevy shell's rule.
    #[test]
    fn save_screen_keeps_the_tavern_quantity_on_its_question_row() {
        use u5_runtime::shop_runtime::TavernState;
        use u5_runtime::shop_session::ActiveShopSession;
        use u5_runtime::test_fixtures::{open_grid, test_state};

        let mut state = test_state(open_grid(), 1, 1);
        state.active_shop = Some(ActiveShopSession::Tavern(
            TavernState::PickProvisionQuantity {
                tavern: u5_runtime::Tavern::TheHonestMeal,
                unit_price: 10,
                continuation_ready: false,
                entry: Default::default(),
            },
        ));
        state.emit_message_line("\n\nHow many wouldst\nthou like?\" 3");
        let log = u5_runtime::message_log_from_entries(state.displayed_message_entries(), |text| {
            (!text.trim().is_empty()).then(|| text.to_string())
        });
        let open_prompt = save_screen_open_prompt(&state, &log, false);
        assert_eq!(open_prompt.as_deref(), Some("thou like?\" 3"));
        let live_row = (!state.message_window_live_row_suppressed()).then_some("");
        let layout = u5_runtime::layout_message_window_with_continuation(
            &log,
            live_row,
            open_prompt.as_deref(),
            false,
            save_screen_live_row_kind(&state, false),
        );
        let last = layout.rows.last().expect("rows");
        assert_eq!(last.text.trim_end(), "thou like?\" 3", "{:?}", layout.rows);
        assert!(!last.prefixed);
    }

    /// Playtest F51: `shops.md §8.1` puts the sell browser's rows at
    /// window-local `(1, 1)..(1, 4)` of the framed side panel, and the
    /// `--save-screen` oracle has to paint them as the Bevy shell does.
    #[test]
    fn save_screen_paints_the_arms_sell_browser_rows() {
        use u5_runtime::shop_runtime::{ArmsSellBrowser, ArmsShopState};
        use u5_runtime::shop_session::ActiveShopSession;
        use u5_runtime::test_fixtures::{open_grid, test_state};

        let mut state = test_state(open_grid(), 1, 1);
        state.equipment_stock = [0; u5_runtime::EQUIPMENT_COUNT];
        state.equipment_stock[0] = 3;
        state.equipment_stock[16] = 2;
        let browser = ArmsSellBrowser::new(&state.equipment_stock).unwrap();
        state.active_shop = Some(ActiveShopSession::ArmsStocked(
            ArmsShopState::SellPickItem(browser),
            u5_runtime::ArmsShop::IolosBows.stock_table(),
        ));
        let mut system = TextWindowSystem::new();
        configure_play_text_windows(&mut system);
        paint_save_screen_panel_windows(&mut system, &state, None);
        let row = |local_row: u8| -> String {
            (1..14)
                .map(|local_column| {
                    system
                        .cell(
                            u5_runtime::ARMS_SELL_BROWSER_LEFT + local_column,
                            u5_runtime::ARMS_SELL_BROWSER_TOP + local_row,
                        )
                        .map_or(' ', |cell| cell.byte as char)
                })
                .collect()
        };
        assert_eq!(row(1).trim_end(), " 3-Leath Helm");
        assert_eq!(row(2).trim_end(), " 2-Dagger");
    }

    #[test]
    fn write_rgba_png_creates_parent_dirs_and_round_trips_pixels() {
        let dir = temp_output_dir("round-trip");
        let out = dir.join("nested").join("frame.png");
        let rgba = vec![0, 0, 0, 255, 255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];

        write_rgba_png(&out, 2, 2, rgba.clone()).unwrap();

        let image = image::open(&out).unwrap().to_rgba8();
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 2);
        assert_eq!(image.into_raw(), rgba);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn write_rgba_png_rejects_bad_buffer_size() {
        let dir = temp_output_dir("bad-buffer");
        let out = dir.join("frame.png");
        let err = write_rgba_png(&out, 2, 2, vec![0; 15]).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(!out.exists());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn save_frame_suite_local_clean_writes_pngs_and_manifest_when_present() {
        let Some(game_dir) = u5_runtime::test_fixtures::configured_original_asset_dir() else {
            return;
        };
        let game_dir = game_dir.as_path();
        if !game_dir.join("CASTLE.DAT").exists() || !game_dir.join(TILES_EGA_FILE).exists() {
            return;
        }

        let dir = temp_output_dir("suite");
        let reports = save_frame_suite(game_dir, TileGraphicsDepth::Ega16, &dir).unwrap();

        assert_eq!(reports.len(), 16);
        for report in &reports {
            assert!(report.path.exists());
            assert!(report.nonblack_pixels > 0);
        }
        for label in ["britannia", "britannia-step", "castle", "dungeon"] {
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected viewport report");
            assert_eq!(report.width, 176);
            assert_eq!(report.height, 176);
        }
        let combat = reports
            .iter()
            .find(|report| report.label == "combat")
            .expect("expected combat frame report");
        assert_eq!(combat.width, TEXT_WINDOW_RENDER_WIDTH as u32);
        assert_eq!(combat.height, TEXT_WINDOW_RENDER_HEIGHT as u32);
        assert_eq!(combat.frame_kind, "composed combat frame");
        for label in ["surface-view", "peer-view", "x-ray-view"] {
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected surface view overlay report");
            assert_eq!(report.width, TEXT_WINDOW_RENDER_WIDTH as u32);
            assert_eq!(report.height, TEXT_WINDOW_RENDER_HEIGHT as u32);
            assert_eq!(report.frame_kind, "composed view overlay");
        }
        for label in [
            "surface-view-class-gallery",
            "peer-view-class-gallery",
            "x-ray-view-class-gallery",
        ] {
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected surface view class gallery report");
            assert_eq!(report.width, TEXT_WINDOW_RENDER_WIDTH as u32);
            assert_eq!(report.height, TEXT_WINDOW_RENDER_HEIGHT as u32);
            assert_eq!(report.frame_kind, "composed view overlay");
        }
        let dungeon_view = reports
            .iter()
            .find(|report| report.label == "dungeon-view")
            .expect("expected dungeon view overlay report");
        assert_eq!(dungeon_view.width, TEXT_WINDOW_RENDER_WIDTH as u32);
        assert_eq!(dungeon_view.height, TEXT_WINDOW_RENDER_HEIGHT as u32);
        assert_eq!(dungeon_view.frame_kind, "composed view overlay");
        let endgame = reports
            .iter()
            .find(|report| report.label == "endgame-status")
            .expect("expected endgame text-window report");
        assert_eq!(endgame.width, TEXT_WINDOW_RENDER_WIDTH as u32);
        assert_eq!(endgame.height, TEXT_WINDOW_RENDER_HEIGHT as u32);
        for label in ["intro-menu", "status-window", "z-stats-modal"] {
            let report = reports
                .iter()
                .find(|report| report.label == label)
                .expect("expected text-window report");
            assert_eq!(report.width, TEXT_WINDOW_RENDER_WIDTH as u32);
            assert_eq!(report.height, TEXT_WINDOW_RENDER_HEIGHT as u32);
        }
        let manifest = fs::read_to_string(dir.join("manifest.txt")).unwrap();
        assert!(manifest.contains("britannia"));
        assert!(manifest.contains("castle"));
        assert!(manifest.contains("dungeon"));
        assert!(manifest.contains("combat"));
        assert!(manifest.contains("surface-view"));
        assert!(manifest.contains("dungeon-view"));
        assert!(manifest.contains("peer-view"));
        assert!(manifest.contains("x-ray-view"));
        assert!(manifest.contains("surface-view-class-gallery"));
        assert!(manifest.contains("peer-view-class-gallery"));
        assert!(manifest.contains("x-ray-view-class-gallery"));
        assert!(manifest.contains("intro-menu"));
        assert!(manifest.contains("status-window"));
        assert!(manifest.contains("z-stats-modal"));
        assert!(manifest.contains("endgame-status"));
        assert!(!manifest.contains("Avatar"));
        let _ = fs::remove_dir_all(dir);
    }
}

/// `--save-screen`: compose the whole 320x200 gameplay screen headlessly.
///
/// Border chrome first, then the text-window surface (stats panel)
/// composited skipping black, then the message window's rows with
/// their ribbon end-cap prefixes, then the tile viewport at its
/// measured origin. Shares every constant and painter with the Bevy
/// compositor via `u5_runtime::gameplay_chrome`.
pub fn run_save_screen(
    game_dir: &Path,
    options: PlayOptions,
    raster_depth: TileGraphicsDepth,
    script: Option<Vec<String>>,
    out: &Path,
) -> io::Result<()> {
    let mut state = PlayState::load_scene(game_dir, options)?;
    let atlas = load_tile_atlas(game_dir, raster_depth)?;
    if let Some(commands) = script {
        replay_play_script_commands(&mut state, game_dir, &commands, |_, _, _| Ok(()))?;
    }
    complete_headless_blocking_presentations(&mut state, Some(&atlas))?;
    let ibm = load_ibm_ch_font(game_dir)?;
    let runes = load_runes_ch_font(game_dir)?;
    let rgba = compose_gameplay_screen(&mut state, game_dir, &atlas, &ibm, &runes)?;
    write_rgba_png(
        out,
        TEXT_WINDOW_RENDER_WIDTH as u32,
        TEXT_WINDOW_RENDER_HEIGHT as u32,
        rgba,
    )?;
    println!(
        "Saved {}x{} composed gameplay screen to {} (player at ({}, {}) facing {}, turn {})",
        TEXT_WINDOW_RENDER_WIDTH,
        TEXT_WINDOW_RENDER_HEIGHT,
        out.display(),
        state.player.x,
        state.player.y,
        u5_runtime::Direction::name(state.player.facing),
        state.turn,
    );
    Ok(())
}

/// The side-panel text windows, painted in the Bevy shell's order: the
/// stats panel, then - while a shop is up - its two framed side panels, the
/// arms sell browser and the inn guest register (`shops.md §8.A`). This
/// oracle painted the stats panel alone, so `--save-screen` showed the
/// ordinary roster where the sell browser's item rows belong (playtest F51).
fn paint_save_screen_panel_windows(
    system: &mut TextWindowSystem,
    state: &PlayState,
    active_cursor: Option<usize>,
) {
    paint_stats_panel_text_window(system, state, active_cursor);
    if state.active_shop.is_some() {
        u5_runtime::paint_arms_sell_browser_text_window(system, state);
        u5_runtime::paint_inn_pickup_register_text_window(system, state);
    }
}

use u5_runtime::endgame_narrative_frame::{
    compose_endgame_narrative_window_rgba, endgame_narrative_window_on_screen,
};

/// Compose the full gameplay screen into a fresh RGBA buffer.
///
/// `endgame.md §8.3`: while a final narrative window is up the screen is
/// that window's full page instead, taken from the same composer the
/// visual shell's page agrees with.
pub fn compose_gameplay_screen(
    state: &mut PlayState,
    game_dir: &Path,
    atlas: &u5_runtime::TileAtlas,
    ibm: &FixedCellFont,
    runes: &FixedCellFont,
) -> io::Result<Vec<u8>> {
    let width = TEXT_WINDOW_RENDER_WIDTH;
    let height = TEXT_WINDOW_RENDER_HEIGHT;
    // `endgame.md §8.3`: a final narrative window is a hard cut onto the
    // full-screen window, composed exactly as the visual shell composes it -
    // none of the gameplay layout below survives onto it (playtest F58).
    if let Some(window) = endgame_narrative_window_on_screen(state) {
        return compose_endgame_narrative_window_rgba(state, game_dir, window);
    }
    // `endgame.md §8.4`/`§9`: after the sixth window the parchment page and
    // the certificate printed over it are the whole screen (P15-06).
    if u5_runtime::endgame_certificate_frame::endgame_certificate_on_screen(state) {
        return u5_runtime::endgame_certificate_frame::compose_endgame_certificate_rgba(
            state, game_dir, ibm, runes,
        );
    }
    let mut rgba = vec![0u8; width * height * 4];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[0, 0, 0, 0xff]);
    }

    state.refresh_cached_moon_glyphs();
    let content = gameplay_chrome_content(state);
    paint_gameplay_frame_chrome(
        &mut rgba,
        width,
        height,
        &content,
        ChromeFonts { ibm, runes },
        ChromePalette::EGA,
    );

    // Only the stats panel goes through the text-window pipeline. The
    // message window is painted directly below, because each echoed
    // line carries the two-colour ribbon end cap that the 1-bit
    // fixed-cell text path cannot express.
    let active_cursor = state.active_player;
    let mut system = TextWindowSystem::new();
    configure_play_text_windows(&mut system);
    paint_save_screen_panel_windows(&mut system, state, active_cursor);
    // `inventory.md §4.5`: a decorated picker row, the Z-stats moonstone row
    // and `magic.md §8`'s sextant coordinate row all mark individual cells
    // runic, and this composer was drawing every cell from `IBM.CH` - the
    // no-runes overload silently falls back to the text font, so the runic
    // cells rendered as whatever `IBM.CH` holds at that code.
    let text = u5_runtime::render_text_window_rgba_with_runes(&system, ibm, Some(runes))?;
    overlay_panel_text_window_rgba(&mut rgba, &text, width, height, state);

    // Keep this in step with the Bevy shell's own message-window
    // assembly: this path exists so `--save-screen` is a native-resolution
    // oracle for the same screen, and it is only an oracle while the two
    // agree. Both drop blank slot text, both append the slot only when
    // the transcript has not recorded it yet, and both suppress the live
    // row for a prompt that is waiting on a key.
    fn keep(text: &str) -> Option<String> {
        (!text.trim().is_empty()).then(|| text.to_string())
    }
    // `magic.md §8`: a spell flash freezes the window on its pre-flash rows.
    let mut log = message_log_from_entries(state.displayed_message_entries(), keep);
    log.set_top_offset(usize::from(state.message_window_top_offset()));
    if let Some(text) = state.displayed_unflushed_message().and_then(keep) {
        log.push_output(&text);
    }
    let flash_hold = state.message_window_flash_hold_active();
    // `text-output.md §10.6`: a prompt awaiting a key keeps its own line
    // open and carries the input cursor inline, so the headless
    // composition places it exactly where the Bevy renderer does.
    let open_prompt = save_screen_open_prompt(state, &log, flash_hold);
    // The spell-name colon line is a live row served from the cast/mix
    // session rather than from the log, so the oracle has to ask for it
    // the same way the shell does. There is no typed-input buffer here,
    // which is the one live-row source this path cannot mirror.
    let spell_echo = state.typed_prompt_echo().filter(|_| !flash_hold);
    let live_row = if state.message_window_live_row_suppressed() {
        None
    } else {
        Some(spell_echo.as_deref().unwrap_or(""))
    };
    let live_row_kind = save_screen_live_row_kind(state, spell_echo.is_some());
    let layout = u5_runtime::layout_message_window_with_continuation(
        &log,
        live_row,
        open_prompt.as_deref(),
        u5_runtime::combat_prompt_row_follows_history(state),
        live_row_kind,
    );
    let cursor_cell = save_screen_cursor_cell(state, &layout);
    for row in layout.rows {
        if row.prefixed {
            paint_message_line_cap(&mut rgba, width, height, ibm, row.row, ChromePalette::EGA);
        }
        for (offset, glyph) in row.glyphs.iter().enumerate() {
            let font = match glyph.font {
                u5_runtime::TlkGlyphFont::Ordinary => ibm,
                u5_runtime::TlkGlyphFont::Runic => runes,
            };
            paint_fixed_cell_glyph(
                &mut rgba,
                width,
                height,
                font,
                glyph.byte,
                row.column.saturating_add(offset as u8),
                row.row,
                CHROME_RULE_INDEX,
            );
        }
    }

    if let Some((column, row)) = cursor_cell {
        paint_fixed_cell_glyph(
            &mut rgba,
            width,
            height,
            ibm,
            prompt_cursor_glyph(0),
            column,
            row,
            CHROME_RULE_INDEX,
        );
    }

    let local_view_overlay = save_screen_local_view_overlay_active(state);
    if let Some(viewport) = save_screen_viewport(state, atlas)? {
        blit_save_screen_viewport(&mut rgba, width, height, &viewport, local_view_overlay);
    }
    Ok(rgba)
}

/// Lay the viewport raster into the composed screen.
///
/// `view.md §4`: the local View/Peer/X-Ray raster is "centered inside the
/// main gameplay viewport" at `anchor = 32 + cell * 4`, covering
/// `(32,32)..(159,159)` - "The formula is authoritative". The Bevy shell
/// clears the viewport and blits there; this path put the 128-pixel raster
/// flush at the viewport's own origin (playtest K02).
fn blit_save_screen_viewport(
    rgba: &mut [u8],
    width: usize,
    height: usize,
    viewport: &u5_runtime::TileViewport,
    local_view_overlay: bool,
) {
    let (origin_x, origin_y) = if local_view_overlay {
        clear_save_screen_viewport(rgba, width, height);
        (LOCAL_VIEW_OVERLAY_ORIGIN_X, LOCAL_VIEW_OVERLAY_ORIGIN_Y)
    } else {
        (VIEWPORT_ORIGIN_X, VIEWPORT_ORIGIN_Y)
    };
    blit_rgba(
        rgba,
        width,
        height,
        &viewport.to_rgba(),
        viewport.width,
        viewport.height,
        origin_x,
        origin_y,
    );
}

/// Whether the viewport [`save_screen_viewport`] returns is the local
/// 32-by-32 View/Peer/X-Ray raster rather than a full-viewport frame. The
/// dungeon V-View and the telescope sky replace the whole viewport
/// (`view.md §6`, `§4.2.2`), as in the Bevy compositor's same test.
fn save_screen_local_view_overlay_active(state: &PlayState) -> bool {
    use u5_runtime::endgame_tableau_frame::endgame_frame_shows_tableau;
    !endgame_frame_shows_tableau(state)
        && state.active_view_overlay.as_ref().is_some_and(|overlay| {
            !matches!(
                overlay.kind,
                u5_runtime::ViewOverlayKind::Dungeon { .. } | u5_runtime::ViewOverlayKind::Sky(_)
            )
        })
}

/// Black out the inclusive `(8,8)..(183,183)` viewport compose rectangle,
/// as the Bevy shell does before blitting a local view overlay.
fn clear_save_screen_viewport(rgba: &mut [u8], width: usize, height: usize) {
    let side = VIEWPORT_SIZE_PX;
    for y in VIEWPORT_ORIGIN_Y..(VIEWPORT_ORIGIN_Y + side).min(height) {
        for x in VIEWPORT_ORIGIN_X..(VIEWPORT_ORIGIN_X + side).min(width) {
            let offset = (y * width + x) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
    }
}

/// Lay the panel text-window raster over the chrome.
///
/// Mirrors the Bevy shell's opaque cells (`render_visual_play_frame_over_viewport`):
/// the picker's panel cells, and under the leaked inverse the timed-effect
/// slot's glyph and caps, are whole text cells whose black pixels cover the
/// chrome beneath (`inventory.md §4.4`, u5-spec#299 Q12). Every other cell
/// lays only its non-black pixels. Overlaying only the non-black pixels
/// everywhere drew the inverted `P` as a solid white cell and the caps
/// without their black triangles (playtest J25).
fn overlay_panel_text_window_rgba(
    rgba: &mut [u8],
    text: &[u8],
    width: usize,
    height: usize,
    state: &PlayState,
) {
    let opaque_cells: Vec<(u8, u8)> = u5_runtime::picker_panel_opaque_cells(state)
        .into_iter()
        .chain(u5_runtime::stats_panel_opaque_effect_cell(state))
        .chain(u5_runtime::stats_panel_opaque_effect_cap_cells(state))
        .collect();
    let cell = u5_runtime::CH_CELL_SIDE;
    // The chrome a residue cell leaves to the band
    // (`picker_residue_cell_chrome_columns`), saved before the text lands.
    let kept_chrome: Vec<(usize, [u8; 4])> = opaque_cells
        .iter()
        .flat_map(|&(column, row)| {
            let (x0, y0) = (usize::from(column) * cell, usize::from(row) * cell);
            let keep = u5_runtime::picker_residue_cell_chrome_columns(state, column, row);
            (y0..(y0 + cell).min(height))
                .flat_map(move |y| (x0..(x0 + keep).min(width)).map(move |x| (y * width + x) * 4))
        })
        .map(|at| (at, [rgba[at], rgba[at + 1], rgba[at + 2], rgba[at + 3]]))
        .collect();
    for (dst, src) in rgba.chunks_exact_mut(4).zip(text.chunks_exact(4)) {
        if src[0] == 0 && src[1] == 0 && src[2] == 0 {
            continue;
        }
        dst.copy_from_slice(src);
    }
    for (column, row) in opaque_cells {
        let (x0, y0) = (usize::from(column) * cell, usize::from(row) * cell);
        for y in y0..(y0 + cell).min(height) {
            let start = (y * width + x0) * 4;
            let end = (y * width + (x0 + cell).min(width)) * 4;
            rgba[start..end].copy_from_slice(&text[start..end]);
        }
    }
    for (at, pixel) in kept_chrome {
        rgba[at..at + 4].copy_from_slice(&pixel);
    }
}

/// The raster that fills the gameplay viewport, chosen as the visual shell
/// chooses it. `endgame.md §3.1`: "The throne-room tableau is the ordinary
/// eleven-by-eleven world viewport", so during the endgame's tableau phase
/// it replaces the dungeon the party left (playtest G32); through the
/// absorption hold the held final room stands in for it.
fn save_screen_viewport(
    state: &mut PlayState,
    atlas: &u5_runtime::TileAtlas,
) -> io::Result<Option<u5_runtime::TileViewport>> {
    use u5_runtime::endgame_tableau_frame::{
        endgame_frame_shows_tableau, render_endgame_tableau_viewport,
    };
    if !endgame_frame_shows_tableau(state) {
        // `karma.md §7`/`§8`: a shrine or Codex presentation shows its own
        // `MISCMAPS.DAT` grid, not the surface view (playtest P15-08).
        if let Some(viewport) =
            u5_runtime::shrine_presentation_frame::render_shrine_presentation_viewport(
                state, atlas,
            )?
        {
            return Ok(Some(viewport));
        }
        return state.render_top_down_frame(VIEWPORT_RADIUS, atlas);
    }
    if state.endgame_absorption_arena_view().is_some()
        && let Some(mut arena) = state.endgame_absorption_arena.take()
    {
        let viewport = arena.render_top_down_frame(VIEWPORT_RADIUS, atlas);
        state.endgame_absorption_arena = Some(arena);
        return viewport;
    }
    render_endgame_tableau_viewport(state, atlas).map(Some)
}

//! `--save-frame` driver: load PlayState, optionally replay a script,
//! render the current viewport, write it to a PNG, and exit.
//!
//! Useful for verifying visual output without a desktop session.

use std::io;
use std::path::Path;

use image::{ImageBuffer, Rgba};
use u5_runtime::{
    CHROME_RULE_INDEX, COMBAT_ARENA_SIDE, ChromeFonts, ChromePalette, DungeonScene, FixedCellFont,
    PlayOptions, PlayState, PlayTarget, STATS_PANEL_TEXT_LEFT, Scene, TEXT_WINDOW_RENDER_HEIGHT,
    TEXT_WINDOW_RENDER_WIDTH, TILE_ATLAS_SIDE, TOWN_GRID_SIDE, TextWindowSystem, TileGraphicsDepth,
    VIEWPORT_ORIGIN_X, VIEWPORT_ORIGIN_Y, ViewOverlayMode, WorldPlane, configure_play_text_windows,
    gameplay_chrome_content, hash_bytes, layout_message_window, load_ibm_ch_font,
    load_runes_ch_font, load_tile_atlas, message_log_from_entries, paint_fixed_cell_glyph,
    paint_gameplay_frame_chrome, paint_message_line_cap, paint_stats_panel_text_window,
    render_text_panel_rgba, render_text_window_rgba,
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
    } else if let Some(viewport) = state.render_top_down_frame(VIEWPORT_RADIUS, &atlas)? {
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
        "  J  Journey Onward",
        "  C  Create New Character",
        "  T  Transfer from Ultima IV",
        "  U  Ultima V Introduction",
        "  A  Acknowledgements",
        "  R  Return to View",
        "",
        "Selection:",
    ]
    .join("\n");
    save_text_window_report(out, "intro text window", &text, None)
}

fn save_frame_suite_status_window(game_dir: &Path, out: &Path) -> io::Result<SavedFrameReport> {
    let mut state = PlayState::load_scene(game_dir, PlayOptions::default())?;
    state.message = "Status frame suite checkpoint.".to_string();
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

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use u5_runtime::{DEFAULT_GAME_DIR, TILES_EGA_FILE};

    fn temp_output_dir(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("u5-save-frame-{name}-{nonce}"))
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
        let game_dir = Path::new(DEFAULT_GAME_DIR);
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
    let rgba = compose_gameplay_screen(&mut state, &atlas, &ibm, &runes)?;
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

/// Compose the full gameplay screen into a fresh RGBA buffer.
pub fn compose_gameplay_screen(
    state: &mut PlayState,
    atlas: &u5_runtime::TileAtlas,
    ibm: &FixedCellFont,
    runes: &FixedCellFont,
) -> io::Result<Vec<u8>> {
    let width = TEXT_WINDOW_RENDER_WIDTH;
    let height = TEXT_WINDOW_RENDER_HEIGHT;
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
    paint_stats_panel_text_window(&mut system, state, active_cursor);
    let text = render_text_window_rgba(&system, ibm)?;
    for (dst, src) in rgba.chunks_exact_mut(4).zip(text.chunks_exact(4)) {
        if src[0] == 0 && src[1] == 0 && src[2] == 0 {
            continue;
        }
        dst.copy_from_slice(src);
    }

    let mut log = message_log_from_entries(state.message_entries(), |text| Some(text.to_string()));
    if !state.message.trim().is_empty()
        && !state
            .message_entries()
            .last()
            .is_some_and(|entry| entry.text == state.message)
    {
        log.push_output(&state.message);
    }
    for row in layout_message_window(&log, Some("")).rows {
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

    if let Some(viewport) = state.render_top_down_frame(VIEWPORT_RADIUS, atlas)? {
        blit_rgba(
            &mut rgba,
            width,
            height,
            &viewport.to_rgba(),
            viewport.width,
            viewport.height,
            VIEWPORT_ORIGIN_X,
            VIEWPORT_ORIGIN_Y,
        );
    }
    Ok(rgba)
}

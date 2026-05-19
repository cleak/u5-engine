//! `--save-frame` driver: load PlayState, optionally replay a script,
//! render the current viewport, write it to a PNG, and exit.
//!
//! Useful for verifying visual output without a desktop session.

use std::io;
use std::path::Path;

use image::{ImageBuffer, Rgba};
use u5_runtime::{
    DungeonScene, PlayOptions, PlayState, PlayTarget, Scene, TILE_ATLAS_SIDE, TileGraphicsDepth,
    WorldPlane, hash_bytes, load_tile_atlas, render_text_panel_rgba,
};

use crate::{raster_frame_kind, replay_play_script_commands};

const VIEWPORT_RADIUS: usize = 5;

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

    let cells = VIEWPORT_RADIUS * 2 + 1;
    let fallback_width = cells * TILE_ATLAS_SIDE;
    let (width, height, rgba, frame_kind) =
        if let Some(viewport) = state.render_top_down_frame(VIEWPORT_RADIUS, &atlas)? {
            (
                viewport.width as u32,
                viewport.height as u32,
                viewport.to_rgba(),
                raster_frame_kind(&state),
            )
        } else {
            (
                fallback_width as u32,
                fallback_width as u32,
                render_text_panel_rgba(
                    &state.render_text_view(VIEWPORT_RADIUS),
                    fallback_width,
                    fallback_width,
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

        assert_eq!(reports.len(), 4);
        for report in &reports {
            assert!(report.path.exists());
            assert_eq!(report.width, 176);
            assert_eq!(report.height, 176);
            assert!(report.nonblack_pixels > 0);
        }
        let manifest = fs::read_to_string(dir.join("manifest.txt")).unwrap();
        assert!(manifest.contains("britannia"));
        assert!(manifest.contains("castle"));
        assert!(manifest.contains("dungeon"));
        assert!(!manifest.contains("Avatar"));
        let _ = fs::remove_dir_all(dir);
    }
}

//! `--save-frame` driver: load PlayState, optionally replay a script,
//! render the current viewport, write it to a PNG, and exit.
//!
//! Useful for verifying visual output without a desktop session.

use std::io;
use std::path::Path;

use image::{ImageBuffer, Rgba};
use u5_runtime::{
    PlayOptions, PlayState, TILE_ATLAS_SIDE, TileGraphicsDepth, load_tile_atlas,
    render_text_panel_rgba,
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

    write_rgba_png(out, width, height, rgba)?;
    println!(
        "Saved {}x{} {frame_kind} to {} (player at ({}, {}) facing {}, turn {})",
        width,
        height,
        out.display(),
        state.player.x,
        state.player.y,
        u5_runtime::Direction::name(state.player.facing),
        state.turn,
    );
    Ok(())
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
}

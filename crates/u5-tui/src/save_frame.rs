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

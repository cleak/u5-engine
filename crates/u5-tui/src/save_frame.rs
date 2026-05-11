//! `--save-frame` driver: load PlayState, optionally replay a script,
//! render the top-down viewport, write it to a PNG, and exit.
//!
//! Useful for verifying visual output without a desktop session.

use std::io;
use std::path::Path;

use image::{ImageBuffer, Rgba};
use u5_runtime::{PlayOptions, PlayState, TileGraphicsDepth, load_tile_atlas};

use crate::handle_play_script_command;

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
        for command in commands {
            handle_play_script_command(&mut state, &command, game_dir)?;
        }
    }

    let Some(viewport) = state.render_top_down_frame(VIEWPORT_RADIUS, &atlas)? else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "current scene has no top-down viewport (dungeon mode is text-only)",
        ));
    };

    let width = viewport.width as u32;
    let height = viewport.height as u32;
    let rgba = viewport.to_rgba();
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
        "Saved {}x{} viewport to {} (player at ({}, {}) facing {}, turn {})",
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

//! `karma.md §7`/`§8`: the shrine and Codex presentation raster, shared by
//! every frontend that draws the gameplay viewport.
//!
//! While the presentation is up the viewport is the presentation's own
//! eleven-by-eleven `MISCMAPS.DAT` grid - record `1` for a shrine, record
//! `2` for the Codex - not the overworld. The visual shell has drawn it this
//! way since `cleak/u5-engine#30`; the headless `--save-screen` and
//! `--save-frame` composers kept drawing the surface view (playtest P15-08).
//! Both now take the raster from here.

use std::io;

use crate::{
    MISCMAPS_CUTSCENE_ROWS, MISCMAPS_CUTSCENE_VISIBLE_COLUMNS, POTION_FLASH_PALETTE_XOR_MASK,
    PlayState, TILE_ATLAS_SIDE, TileAtlas, TileGraphicsDepth, TileViewport,
    blit_tile_id_to_viewport,
};

/// `karma.md §7` "Entry pacing": while the shrine presentation is up, the
/// gameplay viewport is the shrine's own eleven-by-eleven grid and not the
/// overworld map. The presentation clears every active-object slot's type
/// byte for its duration, so no actor is drawn over it, including the party.
///
/// The approach walk is drawn over it. §7's "The two poses, exactly" table
/// gives the walking pose as the party's own on-foot sprite "stamped at
/// `(5, 10)` on frame 5, stepping to `(5, 9)`, `(5, 8)`, `(5, 7)` and
/// `(5, 6)`", and the kneeling pose at that last cell. Frames 1-4 draw "no
/// avatar and no other actor at all", which is the bare backdrop.
/// `cleak/u5-engine#30`; the tiles come from `cleak/u5-spec#273`.
pub fn render_shrine_presentation_viewport(
    state: &PlayState,
    atlas: &TileAtlas,
) -> io::Result<Option<TileViewport>> {
    let Some(map) = state.shrine_presentation_map.as_ref() else {
        return Ok(None);
    };
    let cells = MISCMAPS_CUTSCENE_VISIBLE_COLUMNS;
    let rows = MISCMAPS_CUTSCENE_ROWS;
    let width = cells * TILE_ATLAS_SIDE;
    let height = rows * TILE_ATLAS_SIDE;
    let mut viewport = TileViewport {
        depth: atlas.depth,
        cells_wide: cells,
        cells_high: rows,
        width,
        height,
        pixels: vec![0; width * height],
    };
    for y in 0..rows {
        for x in 0..cells {
            let tile = map.tile(x, y).unwrap_or(0);
            blit_tile_id_to_viewport(&mut viewport, atlas, usize::from(tile), x, y)?;
        }
    }
    if let Some((tile, column, row)) = state
        .shrine_approach_walk
        .as_ref()
        .and_then(|walk| walk.pose_tile())
    {
        blit_tile_id_to_viewport(&mut viewport, atlas, usize::from(tile), column, row)?;
    }
    // `karma.md §7.2`: the offering and Codex turn-in arms invert "the whole
    // viewport" with one exclusive-or fill that is never undone; the counted
    // steps after the swell repaint over it. The same inversion as the
    // potion flash (`audio.md §6`), every colour bit flipped.
    if state.presentation_viewport_inverted {
        let mask = match viewport.depth {
            TileGraphicsDepth::Ega16 => POTION_FLASH_PALETTE_XOR_MASK,
            TileGraphicsDepth::Cga4 => 0x03,
        };
        for pixel in &mut viewport.pixels {
            *pixel ^= mask;
        }
    }
    Ok(Some(viewport))
}

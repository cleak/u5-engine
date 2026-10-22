//! `endgame.md §3.1`: the throne-room tableau raster, shared by every
//! frontend that draws the gameplay viewport.
//!
//! "The throne-room tableau is the ordinary eleven-by-eleven world
//! viewport", drawn from the arena terrain slot with actor coordinates that
//! are viewport cells directly. The visual shell has drawn it this way since
//! `cleak/u5-spec#82`; the headless `--save-screen` composer kept drawing the
//! dungeon the party left (playtest G32). Both now take the raster from here.

use std::io;

use crate::endgame_cinematic::EndgameCinematicStep;
use crate::moongate_phase::{
    MOONGATE_PHASE_ENDGAME_GROUND_TILE, MOONGATE_PHASE_SCRATCH_TILE, MoongatePhaseDraw,
    moongate_phase_draw, moongate_phase_gate_tile, with_moongate_phase_scratch_tile,
};
use crate::{
    ENDGAME_GATE_CELL, ENDGAME_TABLEAU_HEIGHT, ENDGAME_TABLEAU_WIDTH, EndgameOutcome,
    NATURAL_MOONGATE_TERRAIN_TILE, PlayState, TILE_ATLAS_SIDE, TOWN_GRID_SIDE, TileAtlas,
    TileViewport, blit_tile_id_to_viewport, blit_tile_pixels_to_viewport,
    endgame_tableau_role_for_slot,
};

/// Whether the endgame's current frame puts the throne-room tableau (or the
/// held final room before it) in the gameplay viewport, rather than a
/// full-screen page.
pub fn endgame_frame_shows_tableau(state: &PlayState) -> bool {
    let Some(endgame) = state.endgame.as_ref() else {
        return false;
    };
    if matches!(endgame.outcome, Some(EndgameOutcome::MissingBoxOrRefused)) {
        return true;
    }
    if !matches!(endgame.outcome, Some(EndgameOutcome::Victory)) {
        return true;
    }
    matches!(
        endgame.cinematic.step,
        EndgameCinematicStep::RiteMessage(_) | EndgameCinematicStep::ThroneTableau
    )
}

/// Draw the eleven-by-eleven throne-room tableau: terrain from the scene
/// grid with `display-driver-abi.md §10`'s endgame remap, then the tableau
/// actors at their viewport cells.
pub fn render_endgame_tableau_viewport(
    state: &PlayState,
    atlas: &TileAtlas,
) -> io::Result<TileViewport> {
    let width = ENDGAME_TABLEAU_WIDTH * TILE_ATLAS_SIDE;
    let height = ENDGAME_TABLEAU_HEIGHT * TILE_ATLAS_SIDE;
    let mut viewport = TileViewport {
        depth: atlas.depth,
        cells_wide: ENDGAME_TABLEAU_WIDTH,
        cells_high: ENDGAME_TABLEAU_HEIGHT,
        width,
        height,
        pixels: vec![0; width * height],
    };

    for y in 0..ENDGAME_TABLEAU_HEIGHT {
        for x in 0..ENDGAME_TABLEAU_WIDTH {
            let tile = state.grid.get(y * TOWN_GRID_SIDE + x).copied().unwrap_or(0);
            if (x, y) == ENDGAME_GATE_CELL && tile == NATURAL_MOONGATE_TERRAIN_TILE {
                let rows = match moongate_phase_draw(state.natural_moongate_counter) {
                    MoongatePhaseDraw::WholeGate => None,
                    MoongatePhaseDraw::Ground => Some(0),
                    MoongatePhaseDraw::Composed { rows } => Some(rows),
                };
                if let Some(rows) = rows {
                    let missing = |tile: usize| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("endgame gate renderer is missing tile {tile}"),
                        )
                    };
                    let ground = atlas
                        .tile_pixels(usize::from(MOONGATE_PHASE_ENDGAME_GROUND_TILE))
                        .ok_or_else(|| missing(usize::from(MOONGATE_PHASE_ENDGAME_GROUND_TILE)))?
                        .to_vec();
                    let gate_tile = usize::from(moongate_phase_gate_tile());
                    let gate = atlas
                        .tile_pixels(gate_tile)
                        .ok_or_else(|| missing(gate_tile))?
                        .to_vec();
                    let mut scratch = atlas
                        .tile_pixels(MOONGATE_PHASE_SCRATCH_TILE)
                        .ok_or_else(|| missing(MOONGATE_PHASE_SCRATCH_TILE))?
                        .to_vec();
                    with_moongate_phase_scratch_tile(
                        &mut scratch,
                        &ground,
                        &gate,
                        rows,
                        |composed| blit_tile_pixels_to_viewport(&mut viewport, composed, x, y),
                    )??;
                    // Both halves of the composition - floor `0x44` and gate
                    // `0xDC` - are in the remapped set.
                    remap_endgame_tableau_cell(
                        &mut viewport,
                        usize::from(NATURAL_MOONGATE_TERRAIN_TILE),
                        x,
                        y,
                    );
                    continue;
                }
            }
            blit_tile_id_to_viewport(&mut viewport, atlas, usize::from(tile), x, y)?;
            remap_endgame_tableau_cell(&mut viewport, usize::from(tile), x, y);
        }
    }

    for (slot, object) in state.active_objects.iter().copied().enumerate().rev() {
        if endgame_tableau_role_for_slot(slot, object).is_none() {
            continue;
        }
        if object.x >= ENDGAME_TABLEAU_WIDTH || object.y >= ENDGAME_TABLEAU_HEIGHT {
            continue;
        }
        // `active-objects.md §12` / `catalogs/tile-catalog.md §3.1`,
        // published in answer to cleak/u5-spec#82: an active-object
        // record carries an *actor byte*, not a tile id. The renderer
        // adds 256, so actor bytes index the upper, actor half of the
        // 512-tile space - which is why actor `0x44` is the Bard sprite
        // (tile 324) and not the floor tile whose terrain index is also
        // `0x44`. `0x16` is the reserved draw-nothing value.
        //
        // The byte drawn is the compositor's, not the slot's: the default
        // helper is terrain-aware for actors at or above `0x40`, so a king on
        // the bed or a leader on a chair draws as the occupied fixture
        // (`endgame_tableau_actor_drawn_byte`).
        let Some(byte) = crate::endgame_tableau_actor_drawn_byte(&state.grid, object) else {
            continue;
        };
        let Some(tile) = crate::actor_tile_for_byte(byte) else {
            continue;
        };
        blit_tile_id_to_viewport(&mut viewport, atlas, tile, object.x, object.y)?;
    }
    Ok(viewport)
}

/// `display-driver-abi.md §10`'s endgame remap, applied to one drawn cell.
///
/// The mode rewrites twenty-two tiles of the loaded tileset in place, so a
/// renderer that keeps its atlas pristine applies the same map to the
/// pixels those tiles just wrote. "Walls, ground outside that set, and
/// actor sprites are untouched, which is why a captured final room shows
/// green floor and furniture inside an unchanged stone border."
/// `cleak/u5-engine#7`.
fn remap_endgame_tableau_cell(
    viewport: &mut TileViewport,
    tile: usize,
    cell_x: usize,
    cell_y: usize,
) {
    if !crate::endgame_tileset_remap_applies(tile) {
        return;
    }
    let dst_x = cell_x * TILE_ATLAS_SIDE;
    let dst_y = cell_y * TILE_ATLAS_SIDE;
    for row in 0..TILE_ATLAS_SIDE {
        let start = (dst_y + row) * viewport.width + dst_x;
        crate::remap_endgame_tileset_pixels(&mut viewport.pixels[start..start + TILE_ATLAS_SIDE]);
    }
}

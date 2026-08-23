//! `overworld.md §9.1` (spec HEAD `c00bf63`): gate-presence phase and how
//! a moon-gate cell is drawn.
//!
//! A moon-gate cell is **not** drawn as a plain tile most of the time. The
//! renderer special-cases live terrain byte `0xDC` against the shared
//! gate-presence counter, and that counter behaves as a sixteen-step
//! position, not as an on/off flag:
//!
//! | Presence counter | What the cell draws as |
//! |---|---|
//! | `0` | Not a gate. The refresh has already restored the cell to terrain `5`. |
//! | `1..15` | A composed transition frame: the ground tile, with its bottom *N* pixel rows replaced by the top *N* pixel rows of the moon-gate tile. |
//! | `16` | The whole moon-gate tile `0xDC`, drawn through the ordinary tile path. |
//!
//! Read as an animation, phase *N* is "the gate has risen *N* of sixteen
//! pixel rows out of the ground". Counting the phase up makes the gate
//! rise; counting it down makes it sink. Sixteen is the fully open gate
//! and the only phase at which the authored artwork is shown intact.
//!
//! Three contract properties follow:
//!
//! - The composition is **per-cell but the phase is global**: every
//!   visible moon-gate cell composes at the same phase, so a view holding
//!   more than one gate shows them rising and sinking in lockstep. There
//!   is no per-gate phase.
//! - The ground half is **scene-dependent**: ordinary play uses terrain
//!   `5` (grass, what the daytime pass restores); the endgame scene
//!   substitutes tile `0x44`, its throne-room floor, which is why the
//!   endgame gate rises out of flagstones.
//! - The composed frame is written into a **dedicated scratch tile, id
//!   `0x116`**, saved and restored around every composition so its
//!   shipped artwork survives. `0x116` must not be treated as a stable
//!   authored tile while a gate is on screen; the same id doubles as the
//!   party-vanishing sprite of `overworld.md §9.2`.
//!
//! There is **no moongate animator**. `overworld.md §9` retracts the
//! per-render-frame animator in full, and this module deliberately
//! carries no frame ring, no per-frame phase and no ambient-light gate.
//!
//! **Provenance note on the catalog.** `catalogs/tile-catalog.md` briefly
//! contradicted itself here: its §4 said moongates are "not animated at
//! all" and that the renderer "paints it like any other tile", while §11
//! of the same document described this composed-frame model. Both are
//! corrected at spec HEAD `38b0231`; §4 now records that the first
//! withdrawal "overshot", taking the real *composition* with the invented
//! animator. Only the animator was withdrawn. The narrow true claim is
//! that no authored frames exist and no animator advances the phase - so
//! drawing `0xDC` unconditionally is wrong in a different way than the
//! withdrawn animator was. This module implements the composition.

use std::io;

use crate::{
    ENDGAME_TABLEAU_WALKABLE_TILE, NATURAL_MOONGATE_COUNTER_MAX,
    NATURAL_MOONGATE_RESTORED_TERRAIN_TILE, NATURAL_MOONGATE_TERRAIN_TILE, TILE_ATLAS_SIDE,
    TILE_ATLAS_TILE_PIXELS,
};

/// `overworld.md §9.1`: the dedicated scratch tile the composed
/// transition frame is written into. Also the party-vanishing sprite of
/// `overworld.md §9.2`, which is precisely why the slot is saved and
/// restored around every composition rather than owned outright.
pub const MOONGATE_PHASE_SCRATCH_TILE: usize = 0x116;

/// `overworld.md §9.1`: the fully open gate. Sixteen pixel rows, one per
/// phase step, and the same saturation point the once-per-turn refresh
/// counts up to.
pub const MOONGATE_PHASE_FULL: u8 = NATURAL_MOONGATE_COUNTER_MAX;

/// `overworld.md §9.1`: the ordinary-play ground plate - grass, terrain
/// `5`, the same tile the daytime pass restores when a gate closes.
pub const MOONGATE_PHASE_GROUND_TILE: u8 = NATURAL_MOONGATE_RESTORED_TERRAIN_TILE;

/// `overworld.md §9.1`: the endgame scene's substituted ground plate -
/// the throne-room floor, so the endgame gate rises out of flagstones.
pub const MOONGATE_PHASE_ENDGAME_GROUND_TILE: u8 = ENDGAME_TABLEAU_WALKABLE_TILE;

/// `overworld.md §9.1`: how the renderer must draw a live `0xDC` cell at
/// the current shared gate-presence counter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MoongatePhaseDraw {
    /// Counter `0`. Not a gate; the refresh has already restored the cell
    /// to terrain `5`. Drawing the ground plate is the degenerate `N = 0`
    /// case of the composition below - zero gate rows are shown - so the
    /// two branches agree at the boundary.
    Ground,
    /// Counter `1..=15`: a composed transition frame whose bottom `rows`
    /// pixel rows come from the top `rows` pixel rows of the moon-gate
    /// tile.
    Composed { rows: u8 },
    /// Counter `16`: the whole moon-gate tile through the ordinary tile
    /// path, the only phase showing the authored artwork intact.
    WholeGate,
}

/// `overworld.md §9.1`: resolve the shared gate-presence counter into a
/// draw instruction. The counter is global, so every visible gate cell
/// resolves to the same instruction on the same turn.
pub const fn moongate_phase_draw(counter: u8) -> MoongatePhaseDraw {
    if counter == 0 {
        MoongatePhaseDraw::Ground
    } else if counter >= MOONGATE_PHASE_FULL {
        MoongatePhaseDraw::WholeGate
    } else {
        MoongatePhaseDraw::Composed { rows: counter }
    }
}

/// `overworld.md §9.1`: the ground half of the composed frame is
/// scene-dependent - grass in ordinary play, the throne-room floor in the
/// endgame scene.
pub const fn moongate_phase_ground_tile(endgame_scene: bool) -> u8 {
    if endgame_scene {
        MOONGATE_PHASE_ENDGAME_GROUND_TILE
    } else {
        MOONGATE_PHASE_GROUND_TILE
    }
}

/// `overworld.md §9.1`: the whole moon-gate tile, drawn at phase sixteen
/// and the source of the top rows at every partial phase.
pub const fn moongate_phase_gate_tile() -> u8 {
    NATURAL_MOONGATE_TERRAIN_TILE
}

/// `overworld.md §9.1`: compose one transition frame into `scratch`.
///
/// `scratch` is the pixel slot of tile [`MOONGATE_PHASE_SCRATCH_TILE`].
/// The bottom `rows` pixel rows of the ground tile are replaced by the
/// **top** `rows` pixel rows of the moon-gate tile; the remaining top
/// `16 - rows` rows stay ground. `rows` may be `0` (pure ground) through
/// [`MOONGATE_PHASE_FULL`] (pure gate); both endpoints agree with the
/// non-composed branches of [`moongate_phase_draw`].
pub fn compose_moongate_phase_frame(
    scratch: &mut [u8],
    ground: &[u8],
    gate: &[u8],
    rows: u8,
) -> io::Result<()> {
    for (label, slice) in [
        ("scratch", &*scratch),
        ("ground", ground),
        ("moon-gate", gate),
    ] {
        if slice.len() != TILE_ATLAS_TILE_PIXELS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "overworld.md §9.1 gate-phase composition needs a {TILE_ATLAS_TILE_PIXELS}-pixel {label} tile, got {}",
                    slice.len()
                ),
            ));
        }
    }
    if rows > MOONGATE_PHASE_FULL {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "overworld.md §9.1 gate-presence phase is a sixteen-step position; \
                 phase {rows} is out of range 0..={MOONGATE_PHASE_FULL}"
            ),
        ));
    }

    let rows = rows as usize;
    let ground_rows = TILE_ATLAS_SIDE - rows;
    for row in 0..TILE_ATLAS_SIDE {
        let dst = row * TILE_ATLAS_SIDE;
        let src = if row < ground_rows {
            // Untouched ground: the gate has not risen this far yet.
            row * TILE_ATLAS_SIDE
        } else {
            // Bottom `rows` rows of the frame take the TOP `rows` rows of
            // the moon-gate tile - the gate rising out of the ground.
            (row - ground_rows) * TILE_ATLAS_SIDE
        };
        let source = if row < ground_rows { ground } else { gate };
        scratch[dst..dst + TILE_ATLAS_SIDE].copy_from_slice(&source[src..src + TILE_ATLAS_SIDE]);
    }
    Ok(())
}

/// `overworld.md §9.1`: run one composition through the dedicated scratch
/// tile slot, **saving and restoring** the slot around it so the shipped
/// artwork of tile [`MOONGATE_PHASE_SCRATCH_TILE`] - which `§9.2` also
/// uses as the party-vanishing sprite - survives every gate frame.
///
/// The composed pixels are handed to `draw` while they are live in the
/// slot; the slot is restored before this returns, on both the success
/// and the failure path.
pub fn with_moongate_phase_scratch_tile<R>(
    scratch: &mut [u8],
    ground: &[u8],
    gate: &[u8],
    rows: u8,
    draw: impl FnOnce(&[u8]) -> R,
) -> io::Result<R> {
    let saved = scratch.to_vec();
    let composed = compose_moongate_phase_frame(scratch, ground, gate, rows);
    let outcome = composed.map(|()| draw(scratch));
    scratch.copy_from_slice(&saved);
    outcome
}

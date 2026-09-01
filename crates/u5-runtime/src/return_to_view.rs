//! Return-to-View preview script parser.
//!
//! `formats/location-dat.md` section 11 defines the final 655 bytes of
//! `MISCMAPS.DAT` as a compact intro-local bytecode. This module validates the
//! stream shape and exposes command summaries for frontends that do not yet
//! render the full cinematic.

use std::io;
use std::path::Path;

use crate::audio::SoundEffect;
use crate::{AnimationClock, STATIC_TILE_ANIMATION_PERIOD_TICKS};
use crate::{
    MISCMAPS_DAT_FILE, MISCMAPS_RTV_COMMAND_SECTION_OFFSET, MISCMAPS_RTV_STRIP_ROW_STRIDE,
    MISCMAPS_RTV_STRIP_SECTION_BYTES, MISCMAPS_RTV_STRIP_SECTION_OFFSET, RTV_COMMAND_COUNT,
    RTV_COMMAND_STREAM_BYTES, RTV_STRIP_COUNT, TILE_ATLAS_SIDE, TileAtlas, TileViewport,
    read_optional_disk_file,
};

/// `cleak/u5-spec#54` (2026-08-22 resolution, spec head `8192d67`).
///
/// The published orientation was transposed: each `MISCMAPS.DAT`
/// Return-to-View record is **four 32-byte rows**, of which the first
/// nineteen bytes carry tile data and the trailing thirteen are padding.
/// The preview is nineteen cells across by four down.
pub const RTV_STRIP_VISIBLE_COLUMNS: usize = 19;
pub const RTV_STRIP_VISIBLE_ROWS: usize = 4;
/// One 32-byte source row per preview row.
pub const RTV_STRIP_SOURCE_ROWS: usize = RTV_STRIP_VISIBLE_ROWS;
/// Tile bytes carried by each source row before the padding.
pub const RTV_STRIP_SOURCE_COLUMNS: usize = RTV_STRIP_VISIBLE_COLUMNS;
pub const RTV_STRIP_RECORD_BYTES: usize = MISCMAPS_RTV_STRIP_ROW_STRIDE * RTV_STRIP_SOURCE_ROWS;
pub const RTV_STRIP_TILE_COUNT: usize = RTV_STRIP_VISIBLE_COLUMNS * RTV_STRIP_VISIBLE_ROWS;
/// Every preview plane — terrain, overlay and backing — is 19x4.
pub const RTV_PREVIEW_CELLS: usize = RTV_STRIP_TILE_COUNT;
pub const RTV_ACTOR_SLOTS: usize = 32;

/// `#54`: "the viewport's pixel origin is normally `(8, 8)`; the preview
/// raises it to `(8, 16)` on entry and the intro restores `(8, 8)` on
/// exit. That single value is the *only* thing written at preview entry".
pub const RTV_VIEWPORT_ORIGIN_X: usize = 8;
pub const RTV_VIEWPORT_ORIGIN_Y_NORMAL: usize = 8;
pub const RTV_VIEWPORT_ORIGIN_Y_PREVIEW: usize = 16;
/// `#54`: the preview's cells sit seven screen tile rows below the
/// helper grid origin, so cell `(x, y)` lands at
/// `(8 + 16x, 16 + 16(y + 7))` and the strip covers the inclusive
/// rectangle `(8, 128)..(311, 191)` = `304 x 64`.
pub const RTV_SCREEN_ROW_OFFSET: usize = 7;
pub const RTV_PREVIEW_PIXEL_X: usize = RTV_VIEWPORT_ORIGIN_X;
pub const RTV_PREVIEW_PIXEL_Y: usize =
    RTV_VIEWPORT_ORIGIN_Y_PREVIEW + TILE_ATLAS_SIDE * RTV_SCREEN_ROW_OFFSET;
pub const RTV_PREVIEW_PIXEL_WIDTH: usize = RTV_STRIP_VISIBLE_COLUMNS * TILE_ATLAS_SIDE;
pub const RTV_PREVIEW_PIXEL_HEIGHT: usize = RTV_STRIP_VISIBLE_ROWS * TILE_ATLAS_SIDE;

/// `#54` reveal: "the cursor starts on column 9 alone and widens by one
/// column on each side on every **second** preview tick", reaching the
/// full strip after eighteen preview ticks.
pub const RTV_REVEAL_CENTRE_COLUMN: usize = 9;
pub const RTV_REVEAL_TICKS_PER_STEP: u32 = 2;
pub const RTV_REVEAL_FULL_EXPOSURE_TICKS: u32 = 18;

/// `#54`: an overlay byte selects tile index `256 + byte`.
pub const RTV_OVERLAY_TILE_BASE: usize = 256;
/// `#54` reserved plane values meaning "another helper owns this cell
/// this frame"; the ordinary repaint skips them.
pub const RTV_OVERLAY_HELPER_OWNED: u8 = 0x16;
pub const RTV_TERRAIN_HELPER_OWNED: u8 = 0xfe;
/// Retained spelling of the two reserved values under their older names.
pub const RTV_EFFECT_SENTINEL_TILE: u8 = RTV_TERRAIN_HELPER_OWNED;
pub const RTV_TEMPORARY_ACTOR_TILE: u8 = RTV_OVERLAY_HELPER_OWNED;

pub const RTV_OPEN_EFFECT_FINAL_TILE: u8 = 0xdc;
pub const RTV_CLOSE_EFFECT_FINAL_TILE: u8 = 0x05;
pub const RTV_ACTOR_TRANSPARENT_PIXEL: u8 = 0;
/// `#54`: the local cell effect is the driver's animated-terrain shimmer
/// entry, stepped `1..15` to open and `15..1` to close, with two ticks
/// run at the command's tail.
pub const RTV_CELL_EFFECT_STEPS: u8 = 15;
pub const RTV_CELL_EFFECT_FINAL_TICKS: u8 = 2;
/// `u5-spec#117`: carry-set single-cell convergence writes one complete
/// 16x16 cell, checking input through a full preview tick after every eight
/// writes except the final group.
pub const RTV_SINGLE_CELL_WRITES: u16 = 256;
pub const RTV_SINGLE_CELL_WRITES_PER_CHECKPOINT: u16 = 8;
pub const RTV_SINGLE_CELL_CHECKPOINTS: u8 = 31;
pub const RTV_SINGLE_CELL_GALOIS_TAP: u8 = 0xb8;
/// `#54`: the `0x0B` wipe pairs are `n = 0..4`, and the command runs
/// three ticks at its tail. There is **no** fixed eight-tick wait: that
/// was retracted, and the `0x0B` percussive speaker effect is not a
/// timed pause.
pub const RTV_FIXED_WIPE_STEPS: u8 = 5;
pub const RTV_FIXED_WIPE_TRAILING_TICKS: u8 = 3;
pub const RTV_FIXED_WIPE_TOTAL_TICKS: u8 = RTV_FIXED_WIPE_STEPS + RTV_FIXED_WIPE_TRAILING_TICKS;
/// `#54`: command `0x0D` runs seven ticks at its tail.
pub const RTV_MOVE_ACTOR_AND_TICK_TICKS: u8 = 7;
/// `#54`: the fixed-wipe rectangles are drawn in user-interface colour
/// slot 1 and are absolute framebuffer pixel rectangles, deliberately
/// not cell-aligned.
pub const RTV_FIXED_WIPE_COLOUR_SLOT: u8 = 1;

pub const RTV_STRIP_CAPTIONS: [&str; RTV_STRIP_COUNT] = [
    "The Summoning",
    "The Journey",
    "The Arrival",
    "The Welcoming",
];
/// `#54` / `systems/intro.md §12.1`: the chapter caption is fixed-cell
/// text on text row 24 in the window's bottom border, starting at column
/// `18 - floor(len / 2)`.
pub const RTV_CAPTION_TEXT_ROW: usize = 24;
pub const RTV_CAPTION_CENTRE_COLUMN: usize = 18;

/// `#54`: "every preview tick polls the keyboard once, and any pending
/// key aborts the preview immediately", restoring the saved title/menu
/// image. There is no uninterruptible phase and no ESC special case.
pub const RTV_WAIT_EXITS_ON_KEYPRESS: bool = true;

/// `audio.md §8.6` / `systems/intro.md §12`: the strip index also selects the
/// preview's ambient sound. Strips `0` ("The Summoning") and `1` ("The
/// Journey") are silent; strip `2` ("The Arrival") emits a random-pitch
/// percussive effect on every preview tick; strip `3` ("The Welcoming") emits
/// a two-tone chime on an eight-tick cycle.
pub const RTV_PERCUSSIVE_SOUND_STRIP: u8 = 2;
pub const RTV_CHIME_SOUND_STRIP: u8 = 3;
/// `audio.md §8.6`: strip 3 sounds "at local phase 0" and "at phase 4" of an
/// eight-tick cycle.
pub const RTV_CHIME_CYCLE_TICKS: u32 = 8;

/// The cue one scheduled preview tick of `strip` carries.
///
/// `ticks_since_strip_load` is the module's existing chapter-local tick
/// counter, which the first painted tick of a chapter reaches as `1`. The
/// eight-tick chime cycle is therefore anchored on that first tick: phase
/// `(ticks_since_strip_load - 1) % 8`. `audio.md §8.6` publishes the two
/// sounding phases but not what the cycle counts from; anchoring it on the
/// strip load is the only chapter-local origin the published data gives.
///
/// Only phases 0 and 4 are emitted. `SoundEffect::ReturnToViewStrip3` lowers
/// the other six to a stop-only program, so emitting them would be harmless,
/// but `None` states "this tick has no cue" directly.
pub fn return_to_view_tick_sound(
    strip: Option<u8>,
    ticks_since_strip_load: u32,
) -> Option<SoundEffect> {
    if ticks_since_strip_load == 0 {
        return None;
    }
    match strip? {
        RTV_PERCUSSIVE_SOUND_STRIP => Some(SoundEffect::ReturnToViewStrip2),
        RTV_CHIME_SOUND_STRIP => {
            let phase = ((ticks_since_strip_load - 1) % RTV_CHIME_CYCLE_TICKS) as u8;
            crate::audio::return_to_view_strip3_frequency(phase)
                .map(|_| SoundEffect::ReturnToViewStrip3 { phase })
        }
        _ => None,
    }
}

/// `#54` / `systems/intro.md §12.1`: the caption's start column for a
/// caption of `len` cells.
pub const fn return_to_view_caption_start_column(len: usize) -> usize {
    RTV_CAPTION_CENTRE_COLUMN - len / 2
}

/// `formats/location-dat.md §11`: Return-to-View draws its cells and its
/// actors seven screen tile rows below the script-local Y. `#54`
/// confirms the `+ 7` is on the row axis, the `0..3` axis, and that
/// script coordinates never leave `x = 0..18` / `y = 0..3`, so nothing
/// is ever clipped.
pub const fn return_to_view_actor_screen_y(actor_y: u8) -> u8 {
    assert!(
        actor_y <= u8::MAX - RTV_SCREEN_ROW_OFFSET as u8,
        "Return-to-View actor screen Y overflows"
    );
    actor_y + RTV_SCREEN_ROW_OFFSET as u8
}

/// `#54`: the pixel rectangle a preview cell occupies on the intro
/// framebuffer, as `(x, y, width, height)`.
pub const fn return_to_view_cell_pixel_rect(x: usize, y: usize) -> (usize, usize, usize, usize) {
    (
        RTV_VIEWPORT_ORIGIN_X + TILE_ATLAS_SIDE * x,
        RTV_VIEWPORT_ORIGIN_Y_PREVIEW + TILE_ATLAS_SIDE * (y + RTV_SCREEN_ROW_OFFSET),
        TILE_ATLAS_SIDE,
        TILE_ATLAS_SIDE,
    )
}

/// `#54` strip reveal: the inclusive column span the repaint cursor
/// covers after `preview_ticks` preview ticks have elapsed.
///
/// Ticks 1-2 paint column 9 alone; every second tick thereafter widens
/// the span by one column on each side, so the whole strip is exposed
/// from tick 18 onwards. Tick 0 — before the first preview tick of the
/// chapter — paints nothing.
pub fn return_to_view_revealed_columns(preview_ticks: u32) -> Option<(usize, usize)> {
    if preview_ticks == 0 {
        return None;
    }
    let steps = (preview_ticks - 1) / RTV_REVEAL_TICKS_PER_STEP;
    let steps = usize::try_from(steps).unwrap_or(usize::MAX);
    let first = RTV_REVEAL_CENTRE_COLUMN.saturating_sub(steps);
    let last = RTV_REVEAL_CENTRE_COLUMN
        .saturating_add(steps)
        .min(RTV_STRIP_VISIBLE_COLUMNS - 1);
    Some((first, last))
}

/// `#54`: "a non-zero terrain byte is not a tile id: it indexes the
/// engine's animated-tile frame table (the same table the world renderer
/// cycles), and the table's current entry is the tile actually drawn".
///
/// The table's contents are not published separately, so the byte is
/// resolved through the same world-renderer animation families the
/// gameplay view uses ([`crate::static_tile_animation_family`],
/// `animation.md §6`). A bespoke `0x80..=0x87` / `0xD8..=0xDB` table
/// that once lived here was retracted by `#54` as fabricated; the ids
/// that overlap it now arrive only through the published §6 families.
pub fn return_to_view_terrain_tile_for_frame(terrain: u8, animation_frame: u8) -> u8 {
    AnimationClock::at_static_tile_phase(animation_frame).resolve_static_tile(terrain)
}

/// What the ordinary per-cell repaint draws for one preview cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnToViewCellSource {
    /// `#54`: reserved `0xFE` terrain / `0x16` overlay — another helper
    /// owns this cell this frame and the ordinary repaint skips it.
    HelperOwned,
    /// Non-zero terrain byte, resolved through the animated-tile table.
    Terrain(u8),
    /// Zero terrain byte: the overlay byte selects tile `256 + byte`.
    Overlay(u8),
}

/// `#54`: "a cell draws from its terrain byte when that byte is
/// non-zero, and from its overlay byte otherwise".
pub const fn return_to_view_cell_source(terrain: u8, overlay: u8) -> ReturnToViewCellSource {
    if terrain == RTV_TERRAIN_HELPER_OWNED || overlay == RTV_OVERLAY_HELPER_OWNED {
        return ReturnToViewCellSource::HelperOwned;
    }
    if terrain != 0 {
        return ReturnToViewCellSource::Terrain(terrain);
    }
    ReturnToViewCellSource::Overlay(overlay)
}

/// Resolve one preview cell to an index into the 512-entry tile atlas,
/// or `None` when a helper owns the cell.
pub fn return_to_view_cell_tile_index(
    terrain: u8,
    overlay: u8,
    animation_frame: u8,
) -> Option<usize> {
    match return_to_view_cell_source(terrain, overlay) {
        ReturnToViewCellSource::HelperOwned => None,
        ReturnToViewCellSource::Terrain(terrain) => Some(usize::from(
            return_to_view_terrain_tile_for_frame(terrain, animation_frame),
        )),
        ReturnToViewCellSource::Overlay(overlay) => {
            Some(RTV_OVERLAY_TILE_BASE + usize::from(overlay))
        }
    }
}

pub fn return_to_view_caption_for_strip(strip: u8) -> Option<&'static str> {
    RTV_STRIP_CAPTIONS.get(usize::from(strip)).copied()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewAssets {
    pub strips: ReturnToViewMapStrips,
    pub script: ReturnToViewScript,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewMapStrips {
    pub strips: [[u8; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
}

impl ReturnToViewMapStrips {
    pub fn strip(&self, strip: u8) -> Option<&[u8; RTV_STRIP_TILE_COUNT]> {
        self.strips.get(usize::from(strip))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewScript {
    pub commands: Vec<ReturnToViewCommand>,
}

impl ReturnToViewScript {
    pub fn opcode_count(&self, opcode: u8) -> usize {
        self.commands
            .iter()
            .filter(|command| command.opcode() == opcode)
            .count()
    }

    pub fn no_op_count(&self) -> usize {
        self.commands
            .iter()
            .filter(|command| matches!(command, ReturnToViewCommand::NoOp { .. }))
            .count()
    }

    pub fn known_command_count(&self) -> usize {
        let no_ops = self.no_op_count();
        assert!(
            no_ops <= self.commands.len(),
            "Return-to-View no-op count {no_ops} exceeds parsed command count {}",
            self.commands.len()
        );
        self.commands.len() - no_ops
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnToViewCommand {
    SetActor {
        slot: u8,
        tile: u8,
        x: u8,
        y: u8,
    },
    HideActor {
        slot: u8,
    },
    MoveActor {
        slot: u8,
        direction: u8,
    },
    RunPreviewTick {
        ticks: u8,
    },
    OpenCellEffect {
        x: u8,
        y: u8,
    },
    CloseCellEffect,
    LoadMapStrip {
        strip: u8,
    },
    TemporaryActorDraw {
        slot: u8,
    },
    TemporaryActorDrawOverBacking {
        slot: u8,
    },
    RestartStream,
    SetMapCell {
        tile: u8,
        x: u8,
        y: u8,
    },
    FixedWipeAndActorDraw {
        reserved0: u8,
        reserved1: u8,
        slot: u8,
    },
    ClearActors,
    MoveActorAndTick {
        slot: u8,
        direction: u8,
    },
    LoopStart {
        count: u8,
    },
    LoopEnd,
    NoOp {
        opcode: u8,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReturnToViewActor {
    pub tile0: u8,
    pub tile1: u8,
    pub x: u8,
    pub y: u8,
    pub drawable: bool,
}

/// `cleak/u5-spec#54`: the preview keeps three `19 x 4` planes.
///
/// * `terrain` — non-zero bytes index the animated-tile frame table.
/// * `overlay` — selects tile `256 + byte` when terrain is zero.
/// * `backing` — the terrain snapshot a moving actor restores behind it.
///
/// The reserved values `0xFE` (terrain) and `0x16` (overlay) mean
/// "another helper owns this cell this frame"; the ordinary repaint
/// skips them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewPreviewState {
    pub terrain: [u8; RTV_PREVIEW_CELLS],
    pub overlay: [u8; RTV_PREVIEW_CELLS],
    pub backing: [u8; RTV_PREVIEW_CELLS],
    pub actors: [ReturnToViewActor; RTV_ACTOR_SLOTS],
    pub current_strip: Option<u8>,
    pub current_caption: Option<&'static str>,
    pub loop_count: u8,
    pub loop_start_command: Option<usize>,
    pub cached_effect_cell: Option<(u8, u8)>,
    pub total_ticks: u32,
    /// `#54` strip reveal: preview ticks elapsed since the current strip
    /// was loaded. The repaint cursor is derived from this, and `0x06`
    /// resets it — a strip load followed by a short tick run is still
    /// part-way revealed when the next beat starts, which is the
    /// original behaviour and must not be "fixed" by revealing eagerly.
    pub ticks_since_strip_load: u32,
    pub temporary_actor_draws: u32,
    pub fixed_wipes: u32,
    pub cell_effect_steps: u32,
    pub fixed_wipe_rectangle_steps: u32,
}

impl Default for ReturnToViewPreviewState {
    fn default() -> Self {
        Self {
            terrain: [0; RTV_PREVIEW_CELLS],
            overlay: [0; RTV_PREVIEW_CELLS],
            backing: [0; RTV_PREVIEW_CELLS],
            actors: [ReturnToViewActor::default(); RTV_ACTOR_SLOTS],
            current_strip: None,
            current_caption: None,
            loop_count: 0,
            loop_start_command: None,
            cached_effect_cell: None,
            total_ticks: 0,
            ticks_since_strip_load: 0,
            temporary_actor_draws: 0,
            fixed_wipes: 0,
            cell_effect_steps: 0,
            fixed_wipe_rectangle_steps: 0,
        }
    }
}

impl ReturnToViewPreviewState {
    pub fn drawable_actor_count(&self) -> usize {
        self.actors.iter().filter(|actor| actor.drawable).count()
    }

    /// The terrain byte at a preview cell, or `None` when the cell is
    /// outside the `19 x 4` strip.
    pub fn cell(&self, x: u8, y: u8) -> Option<u8> {
        preview_cell_index(x, y).map(|index| self.terrain[index])
    }

    pub fn overlay_cell(&self, x: u8, y: u8) -> Option<u8> {
        preview_cell_index(x, y).map(|index| self.overlay[index])
    }

    /// `#54`: the inclusive column span the repaint cursor currently
    /// covers, or `None` before the first preview tick of the chapter.
    pub fn revealed_columns(&self) -> Option<(usize, usize)> {
        return_to_view_revealed_columns(self.ticks_since_strip_load)
    }

    /// Stamp a playback frame's tick counters. `strip_load_tick` is the
    /// value `total_ticks` held when the current strip was loaded, so
    /// the reveal cursor advances correctly across a chapter.
    fn set_playback_tick(&mut self, elapsed_title_ticks: u32, strip_load_tick: u32) {
        self.total_ticks = elapsed_title_ticks;
        self.ticks_since_strip_load = elapsed_title_ticks.saturating_sub(strip_load_tick);
    }

    /// Advance the shared preview tick: the reveal cursor widens with
    /// elapsed preview ticks, so every tick-consuming command feeds it.
    fn advance_preview_ticks(&mut self, ticks: u32) {
        add_return_to_view_counter(
            &mut self.total_ticks,
            ticks,
            "Return-to-View total tick counter overflowed",
        );
        add_return_to_view_counter(
            &mut self.ticks_since_strip_load,
            ticks,
            "Return-to-View strip reveal tick counter overflowed",
        );
    }
}

impl ReturnToViewPreviewState {
    pub fn apply_command(
        &mut self,
        strips: &ReturnToViewMapStrips,
        command_index: usize,
        command: ReturnToViewCommand,
    ) -> io::Result<ReturnToViewControl> {
        match command {
            ReturnToViewCommand::SetActor { slot, tile, x, y } => {
                let slot = rtv_slot_index(slot)?;
                let _ = preview_cell_index_checked(x, y)?;
                self.actors[slot] = ReturnToViewActor {
                    tile0: tile,
                    tile1: tile,
                    x,
                    y,
                    drawable: true,
                };
            }
            ReturnToViewCommand::HideActor { slot } => {
                let slot = rtv_slot_index(slot)?;
                self.restore_actor_backing(slot)?;
                self.actors[slot] = ReturnToViewActor::default();
            }
            ReturnToViewCommand::MoveActor { slot, direction } => {
                self.move_actor(slot, direction)?;
            }
            ReturnToViewCommand::RunPreviewTick { ticks } => {
                // `#54`: `0x03` runs exactly the number of ticks in its
                // own argument byte. The retracted fixed eight-tick wait
                // is gone.
                self.advance_preview_ticks(u32::from(ticks));
            }
            ReturnToViewCommand::OpenCellEffect { x, y } => {
                // `#54`: script coordinates are plane coordinates
                // (`x = 0..18`, `y = 0..3`); the `+ 7` screen row offset
                // belongs to the renderer, not to the buffer index.
                self.open_cell_effect(x, y)?;
            }
            ReturnToViewCommand::CloseCellEffect => {
                let Some((x, y)) = self.cached_effect_cell else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Return-to-View close-cell effect has no cached open-cell coordinate",
                    ));
                };
                let index = preview_cell_index_checked(x, y)?;
                // `#54`: the shimmer owns the cell while it steps
                // `15..1`, then the caller writes the close tile.
                self.terrain[index] = RTV_TERRAIN_HELPER_OWNED;
                self.terrain[index] = RTV_CLOSE_EFFECT_FINAL_TILE;
                self.backing[index] = RTV_CLOSE_EFFECT_FINAL_TILE;
                add_return_to_view_counter(
                    &mut self.cell_effect_steps,
                    u32::from(RTV_CELL_EFFECT_STEPS),
                    "Return-to-View cell-effect step counter overflowed",
                );
                self.advance_preview_ticks(u32::from(
                    RTV_CELL_EFFECT_STEPS + RTV_CELL_EFFECT_FINAL_TICKS,
                ));
            }
            ReturnToViewCommand::LoadMapStrip { strip } => {
                self.load_map_strip(strips, strip)?;
            }
            ReturnToViewCommand::TemporaryActorDraw { slot }
            | ReturnToViewCommand::TemporaryActorDrawOverBacking { slot } => {
                let slot = rtv_slot_index(slot)?;
                let actor = self.actors[slot];
                let _ = preview_cell_index_checked(actor.x, actor.y)?;
                add_return_to_view_counter(
                    &mut self.temporary_actor_draws,
                    1,
                    "Return-to-View temporary actor draw counter overflowed",
                );
                self.advance_preview_ticks(u32::from(RTV_SINGLE_CELL_CHECKPOINTS));
            }
            ReturnToViewCommand::RestartStream => {
                return Ok(ReturnToViewControl::Restart);
            }
            ReturnToViewCommand::SetMapCell { tile, x, y } => {
                let index = preview_cell_index_checked(x, y)?;
                self.terrain[index] = tile;
                self.backing[index] = tile;
            }
            ReturnToViewCommand::FixedWipeAndActorDraw { slot, .. } => {
                let slot = rtv_slot_index(slot)?;
                let actor = self.actors[slot];
                let _ = preview_cell_index_checked(actor.x, actor.y)?;
                add_return_to_view_counter(
                    &mut self.fixed_wipes,
                    1,
                    "Return-to-View fixed wipe counter overflowed",
                );
                add_return_to_view_counter(
                    &mut self.fixed_wipe_rectangle_steps,
                    u32::from(RTV_FIXED_WIPE_STEPS),
                    "Return-to-View fixed wipe rectangle counter overflowed",
                );
                // `#54`: `0x0B` runs three ticks at its tail. The
                // retracted eight-tick wait is gone, and the command's
                // percussive speaker effect is not a timed pause.
                self.advance_preview_ticks(u32::from(RTV_FIXED_WIPE_TOTAL_TICKS));
            }
            ReturnToViewCommand::ClearActors => {
                self.actors = [ReturnToViewActor::default(); RTV_ACTOR_SLOTS];
            }
            ReturnToViewCommand::MoveActorAndTick { slot, direction } => {
                self.move_actor(slot, direction)?;
                // `#54`: `0x0D` runs seven ticks at its tail.
                self.advance_preview_ticks(u32::from(RTV_MOVE_ACTOR_AND_TICK_TICKS));
            }
            ReturnToViewCommand::LoopStart { count } => {
                self.loop_count = count;
                self.loop_start_command = Some(command_index + 1);
            }
            ReturnToViewCommand::LoopEnd => {
                if self.loop_count > 0 {
                    self.loop_count -= 1;
                    if self.loop_count > 0 {
                        let Some(target) = self.loop_start_command else {
                            return Err(io::Error::new(
                                io::ErrorKind::InvalidData,
                                "Return-to-View loop end has no saved loop start",
                            ));
                        };
                        return Ok(ReturnToViewControl::JumpTo(target));
                    }
                }
            }
            ReturnToViewCommand::NoOp { .. } => {}
        }
        Ok(ReturnToViewControl::Continue)
    }

    fn load_map_strip(&mut self, strips: &ReturnToViewMapStrips, strip: u8) -> io::Result<()> {
        let source = strips.strip(strip).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Return-to-View map strip {strip} is out of range"),
            )
        })?;
        // `#54`: `0x06` fills the planes completely and immediately —
        // the reveal is a repaint-cursor effect, not incremental buffer
        // population — and the cursor restarts at the centre column.
        self.terrain = *source;
        self.backing = *source;
        self.overlay = [0; RTV_PREVIEW_CELLS];
        self.current_strip = Some(strip);
        self.current_caption = return_to_view_caption_for_strip(strip);
        self.ticks_since_strip_load = 0;
        Ok(())
    }

    fn restore_actor_backing(&mut self, slot: usize) -> io::Result<()> {
        let actor = self.actors[slot];
        if !actor.drawable {
            return Ok(());
        }
        let index = preview_cell_index_checked(actor.x, actor.y)?;
        self.terrain[index] = self.backing[index];
        Ok(())
    }

    fn move_actor(&mut self, slot: u8, direction: u8) -> io::Result<()> {
        let slot = rtv_slot_index(slot)?;
        if !self.actors[slot].drawable {
            // The shipped script moves slot 0 at eleven points where no
            // actor is live in that slot (verified against the shipped
            // `MISCMAPS.DAT`: every such move is on an unset slot, and no
            // *placed* actor ever steps off the strip, which is what
            // `cleak/u5-spec#54` means by "script coordinates never leave
            // `x = 0..18` / `y = 0..3`"). An inactive actor-table entry
            // has nothing on screen to move, so the step is a no-op.
            return Ok(());
        }
        self.restore_actor_backing(slot)?;
        let actor = &mut self.actors[slot];
        let (x, y) = rtv_step_coordinate(actor.x, actor.y, direction)?;
        actor.x = x;
        actor.y = y;
        Ok(())
    }

    fn open_cell_effect(&mut self, x: u8, y: u8) -> io::Result<()> {
        let index = preview_cell_index_checked(x, y)?;
        self.cached_effect_cell = Some((x, y));
        // `#54`: the shimmer owns the cell for the duration of its
        // `1..15` steps, then the caller writes `0xDC`.
        self.terrain[index] = RTV_TERRAIN_HELPER_OWNED;
        self.terrain[index] = RTV_OPEN_EFFECT_FINAL_TILE;
        self.backing[index] = RTV_OPEN_EFFECT_FINAL_TILE;
        add_return_to_view_counter(
            &mut self.cell_effect_steps,
            u32::from(RTV_CELL_EFFECT_STEPS),
            "Return-to-View cell-effect step counter overflowed",
        );
        self.advance_preview_ticks(u32::from(
            RTV_CELL_EFFECT_STEPS + RTV_CELL_EFFECT_FINAL_TICKS,
        ));
        Ok(())
    }
}

fn add_return_to_view_counter(counter: &mut u32, amount: u32, message: &'static str) {
    *counter = counter.checked_add(amount).expect(message);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnToViewControl {
    Continue,
    JumpTo(usize),
    Restart,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewPreviewReport {
    pub applied_commands: usize,
    pub restart_seen: bool,
    pub max_commands_reached: bool,
    pub current_strip: Option<u8>,
    pub current_caption: Option<&'static str>,
    pub drawable_actor_count: usize,
    pub total_ticks: u32,
    pub temporary_actor_draws: u32,
    pub fixed_wipes: u32,
    pub cell_effect_steps: u32,
    pub fixed_wipe_rectangle_steps: u32,
    pub cached_effect_cell: Option<(u8, u8)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewPreviewRun {
    pub state: ReturnToViewPreviewState,
    pub report: ReturnToViewPreviewReport,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnToViewFrameKind {
    PreviewTick,
    CellEffectStep { step: u8 },
    CellEffectFinalTick { tick: u8 },
    TemporaryActorDraw { completed_writes: u16 },
    TemporaryActorDrawOverBacking { completed_writes: u16 },
    FixedWipeRectangle { step: u8 },
    FixedWipeActorDraw,
    FixedWait { tick: u8 },
    FixedWipeTrailingTick { tick: u8 },
    MoveActorTick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnToViewActorDrawSource {
    OverlayTile,
    BackingTerrainTile,
    CurrentActorTile,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnToViewActorDrawControl {
    OriginalActorTile(u8),
    BackingMapTile(u8),
    Zero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReturnToViewActorDraw {
    pub slot: u8,
    pub tile: u8,
    pub x: u8,
    pub y: u8,
    pub screen_y: u8,
    pub source: ReturnToViewActorDrawSource,
    pub control: ReturnToViewActorDrawControl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewPlaybackFrame {
    pub command_index: usize,
    pub elapsed_title_ticks: u32,
    pub kind: ReturnToViewFrameKind,
    pub state: ReturnToViewPreviewState,
    pub actor_draw: Option<ReturnToViewActorDraw>,
    /// `audio.md §8.6`: the ambient cue this scheduled preview tick carries.
    /// `None` on strips 0 and 1, on strip 3's six silent phases, and on the
    /// fixed-wipe actor draw, which shares its predecessor's tick rather than
    /// scheduling one of its own.
    pub sound: Option<SoundEffect>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewPlayback {
    pub frames: Vec<ReturnToViewPlaybackFrame>,
    pub run: ReturnToViewPreviewRun,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnToViewActorDrawControlSource {
    OriginalActorTile,
    BackingMapTile,
}

impl ReturnToViewCommand {
    pub const fn opcode(self) -> u8 {
        match self {
            ReturnToViewCommand::SetActor { .. } => 0x00,
            ReturnToViewCommand::HideActor { .. } => 0x01,
            ReturnToViewCommand::MoveActor { .. } => 0x02,
            ReturnToViewCommand::RunPreviewTick { .. } => 0x03,
            ReturnToViewCommand::OpenCellEffect { .. } => 0x04,
            ReturnToViewCommand::CloseCellEffect => 0x05,
            ReturnToViewCommand::LoadMapStrip { .. } => 0x06,
            ReturnToViewCommand::TemporaryActorDraw { .. } => 0x07,
            ReturnToViewCommand::TemporaryActorDrawOverBacking { .. } => 0x08,
            ReturnToViewCommand::RestartStream => 0x09,
            ReturnToViewCommand::SetMapCell { .. } => 0x0a,
            ReturnToViewCommand::FixedWipeAndActorDraw { .. } => 0x0b,
            ReturnToViewCommand::ClearActors => 0x0c,
            ReturnToViewCommand::MoveActorAndTick { .. } => 0x0d,
            ReturnToViewCommand::LoopStart { .. } => 0x0e,
            ReturnToViewCommand::LoopEnd => 0x0f,
            ReturnToViewCommand::NoOp { opcode } => opcode,
        }
    }
}

pub fn load_return_to_view_script(game_dir: &Path) -> io::Result<Option<ReturnToViewScript>> {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    parse_return_to_view_script_file(&bytes).map(Some)
}

pub fn load_return_to_view_assets(game_dir: &Path) -> io::Result<Option<ReturnToViewAssets>> {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    Ok(Some(ReturnToViewAssets {
        strips: parse_return_to_view_map_strips_file(&bytes)?,
        script: parse_return_to_view_script_file(&bytes)?,
    }))
}

pub fn load_return_to_view_map_strips(
    game_dir: &Path,
) -> io::Result<Option<ReturnToViewMapStrips>> {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    let Some(bytes) = read_optional_disk_file(&path)? else {
        return Ok(None);
    };
    parse_return_to_view_map_strips_file(&bytes).map(Some)
}

pub fn parse_return_to_view_map_strips_file(bytes: &[u8]) -> io::Result<ReturnToViewMapStrips> {
    let strip_end = MISCMAPS_RTV_STRIP_SECTION_OFFSET + MISCMAPS_RTV_STRIP_SECTION_BYTES;
    if bytes.len() < strip_end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{MISCMAPS_DAT_FILE}: expected at least {strip_end} bytes for Return-to-View map strips, found {}",
                bytes.len()
            ),
        ));
    }
    parse_return_to_view_map_strips(
        &bytes[MISCMAPS_RTV_STRIP_SECTION_OFFSET
            ..MISCMAPS_RTV_STRIP_SECTION_OFFSET + MISCMAPS_RTV_STRIP_SECTION_BYTES],
    )
}

pub fn parse_return_to_view_map_strips(bytes: &[u8]) -> io::Result<ReturnToViewMapStrips> {
    if bytes.len() != MISCMAPS_RTV_STRIP_SECTION_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Return-to-View map strip section must be {MISCMAPS_RTV_STRIP_SECTION_BYTES} bytes, found {}",
                bytes.len()
            ),
        ));
    }
    // `#54` retraction 1: each record is four 32-byte *rows*, of which
    // the first nineteen bytes carry tile data. The previously published
    // "four 32-byte columns" reading was a transposition and produced an
    // impossible 4-wide by 19-tall preview.
    let mut strips = [[0u8; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT];
    for (strip_index, strip) in strips.iter_mut().enumerate() {
        let base = strip_index * RTV_STRIP_RECORD_BYTES;
        for source_row in 0..RTV_STRIP_SOURCE_ROWS {
            let row_start = base + source_row * MISCMAPS_RTV_STRIP_ROW_STRIDE;
            let source = &bytes[row_start..row_start + RTV_STRIP_SOURCE_COLUMNS];
            for (source_col, tile) in source.iter().copied().enumerate() {
                strip[source_row * RTV_STRIP_VISIBLE_COLUMNS + source_col] = tile;
            }
        }
    }
    Ok(ReturnToViewMapStrips { strips })
}

pub fn parse_return_to_view_script_file(bytes: &[u8]) -> io::Result<ReturnToViewScript> {
    let stream_end = MISCMAPS_RTV_COMMAND_SECTION_OFFSET + RTV_COMMAND_STREAM_BYTES;
    if bytes.len() < stream_end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{MISCMAPS_DAT_FILE}: expected at least {stream_end} bytes for Return-to-View stream, found {}",
                bytes.len()
            ),
        ));
    }
    parse_return_to_view_commands(
        &bytes[MISCMAPS_RTV_COMMAND_SECTION_OFFSET
            ..MISCMAPS_RTV_COMMAND_SECTION_OFFSET + RTV_COMMAND_STREAM_BYTES],
    )
}

pub fn parse_return_to_view_commands(stream: &[u8]) -> io::Result<ReturnToViewScript> {
    if stream.len() != RTV_COMMAND_STREAM_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Return-to-View command stream must be {RTV_COMMAND_STREAM_BYTES} bytes, found {}",
                stream.len()
            ),
        ));
    }

    let mut offset = 0;
    let mut commands = Vec::new();
    while offset < stream.len() {
        let command_offset = offset;
        let opcode = stream[offset];
        offset += 1;
        let command = match opcode {
            0x00 => {
                let args = read_rtv_args::<4>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::SetActor {
                    slot: args[0],
                    tile: args[1],
                    x: args[2],
                    y: args[3],
                }
            }
            0x01 => {
                let args = read_rtv_args::<1>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::HideActor { slot: args[0] }
            }
            0x02 => {
                let args = read_rtv_args::<2>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::MoveActor {
                    slot: args[0],
                    direction: args[1],
                }
            }
            0x03 => {
                let args = read_rtv_args::<1>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::RunPreviewTick { ticks: args[0] }
            }
            0x04 => {
                let args = read_rtv_args::<2>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::OpenCellEffect {
                    x: args[0],
                    y: args[1],
                }
            }
            0x05 => ReturnToViewCommand::CloseCellEffect,
            0x06 => {
                let args = read_rtv_args::<1>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::LoadMapStrip { strip: args[0] }
            }
            0x07 => {
                let args = read_rtv_args::<1>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::TemporaryActorDraw { slot: args[0] }
            }
            0x08 => {
                let args = read_rtv_args::<1>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::TemporaryActorDrawOverBacking { slot: args[0] }
            }
            0x09 => ReturnToViewCommand::RestartStream,
            0x0a => {
                let args = read_rtv_args::<3>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::SetMapCell {
                    tile: args[0],
                    x: args[1],
                    y: args[2],
                }
            }
            0x0b => {
                let args = read_rtv_args::<3>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::FixedWipeAndActorDraw {
                    reserved0: args[0],
                    reserved1: args[1],
                    slot: args[2],
                }
            }
            0x0c => ReturnToViewCommand::ClearActors,
            0x0d => {
                let args = read_rtv_args::<2>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::MoveActorAndTick {
                    slot: args[0],
                    direction: args[1],
                }
            }
            0x0e => {
                let args = read_rtv_args::<1>(stream, &mut offset, command_offset, opcode)?;
                ReturnToViewCommand::LoopStart { count: args[0] }
            }
            0x0f => ReturnToViewCommand::LoopEnd,
            _ => ReturnToViewCommand::NoOp { opcode },
        };
        commands.push(command);
    }
    Ok(ReturnToViewScript { commands })
}

fn read_rtv_args<const N: usize>(
    stream: &[u8],
    offset: &mut usize,
    command_offset: usize,
    opcode: u8,
) -> io::Result<[u8; N]> {
    if *offset + N > stream.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Return-to-View command 0x{opcode:02x} at byte {command_offset} requires {N} argument byte(s)"
            ),
        ));
    }
    let mut args = [0u8; N];
    args.copy_from_slice(&stream[*offset..*offset + N]);
    *offset += N;
    Ok(args)
}

pub const fn return_to_view_command_name(opcode: u8) -> &'static str {
    match opcode {
        0x00 => "set actor",
        0x01 => "hide actor",
        0x02 => "move actor",
        0x03 => "run preview tick",
        0x04 => "open cell effect",
        0x05 => "close cell effect",
        0x06 => "load map strip",
        0x07 => "temporary actor draw",
        0x08 => "temporary actor draw over backing",
        0x09 => "restart stream",
        0x0a => "set map cell",
        0x0b => "fixed wipe and actor draw",
        0x0c => "clear actors",
        0x0d => "move actor and tick",
        0x0e => "loop start",
        0x0f => "loop end",
        _ => "one-byte no-op",
    }
}

pub fn return_to_view_command_histogram(
    script: &ReturnToViewScript,
) -> [(u8, usize); RTV_COMMAND_COUNT] {
    let mut counts = [(0u8, 0usize); RTV_COMMAND_COUNT];
    let mut opcode = 0u8;
    while usize::from(opcode) < RTV_COMMAND_COUNT {
        counts[usize::from(opcode)] = (opcode, script.opcode_count(opcode));
        opcode += 1;
    }
    counts
}

pub fn summarize_return_to_view_script(script: &ReturnToViewScript) -> String {
    format!(
        "{} parsed command(s): {} known, {} high-opcode no-op(s). Loads {} map strip(s), sets {} actor(s), moves/ticks {} actor step(s), runs {} preview tick command(s), uses {} loop marker(s), restarts {} time(s).",
        script.commands.len(),
        script.known_command_count(),
        script.no_op_count(),
        script.opcode_count(0x06),
        script.opcode_count(0x00),
        script.opcode_count(0x02) + script.opcode_count(0x0d),
        script.opcode_count(0x03),
        script.opcode_count(0x0e) + script.opcode_count(0x0f),
        script.opcode_count(0x09)
    )
}

pub fn run_return_to_view_preview_until_restart(
    strips: &ReturnToViewMapStrips,
    script: &ReturnToViewScript,
    max_commands: usize,
) -> io::Result<ReturnToViewPreviewReport> {
    Ok(run_return_to_view_preview_state_until_restart(strips, script, max_commands)?.report)
}

pub fn run_return_to_view_preview_state_until_restart(
    strips: &ReturnToViewMapStrips,
    script: &ReturnToViewScript,
    max_commands: usize,
) -> io::Result<ReturnToViewPreviewRun> {
    let mut state = ReturnToViewPreviewState::default();
    let mut applied_commands = 0usize;
    let mut pc = 0usize;
    let mut restart_seen = false;
    while pc < script.commands.len() && applied_commands < max_commands {
        let command = script.commands[pc];
        match state.apply_command(strips, pc, command)? {
            ReturnToViewControl::Continue => pc += 1,
            ReturnToViewControl::JumpTo(target) => {
                if target >= script.commands.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Return-to-View loop target command {target} is out of range"),
                    ));
                }
                pc = target;
            }
            ReturnToViewControl::Restart => {
                restart_seen = true;
                applied_commands += 1;
                break;
            }
        }
        applied_commands += 1;
    }
    let report = ReturnToViewPreviewReport {
        applied_commands,
        restart_seen,
        max_commands_reached: !restart_seen
            && pc < script.commands.len()
            && applied_commands >= max_commands,
        current_strip: state.current_strip,
        current_caption: state.current_caption,
        drawable_actor_count: state.drawable_actor_count(),
        total_ticks: state.total_ticks,
        temporary_actor_draws: state.temporary_actor_draws,
        fixed_wipes: state.fixed_wipes,
        cell_effect_steps: state.cell_effect_steps,
        fixed_wipe_rectangle_steps: state.fixed_wipe_rectangle_steps,
        cached_effect_cell: state.cached_effect_cell,
    };
    Ok(ReturnToViewPreviewRun { state, report })
}

pub fn run_return_to_view_playback_until_restart(
    strips: &ReturnToViewMapStrips,
    script: &ReturnToViewScript,
    max_commands: usize,
) -> io::Result<ReturnToViewPlayback> {
    let mut state = ReturnToViewPreviewState::default();
    let mut frames = Vec::new();
    let mut applied_commands = 0usize;
    let mut pc = 0usize;
    let mut restart_seen = false;
    // `#54` strip reveal: the repaint cursor is measured from the tick
    // at which the current strip was loaded.
    let mut strip_load_tick = 0u32;
    while pc < script.commands.len() && applied_commands < max_commands {
        let command_index = pc;
        let command = script.commands[pc];
        let before_ticks = state.total_ticks;
        let before_state = state.clone();
        match state.apply_command(strips, command_index, command)? {
            ReturnToViewControl::Continue => pc += 1,
            ReturnToViewControl::JumpTo(target) => {
                if target >= script.commands.len() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("Return-to-View loop target command {target} is out of range"),
                    ));
                }
                pc = target;
            }
            ReturnToViewControl::Restart => {
                restart_seen = true;
                applied_commands += 1;
                break;
            }
        }
        if matches!(command, ReturnToViewCommand::LoadMapStrip { .. }) {
            strip_load_tick = state.total_ticks;
        }
        append_return_to_view_playback_frames(
            &mut frames,
            command_index,
            command,
            before_ticks,
            strip_load_tick,
            &before_state,
            &state,
        );
        applied_commands += 1;
    }
    let report = ReturnToViewPreviewReport {
        applied_commands,
        restart_seen,
        max_commands_reached: !restart_seen
            && pc < script.commands.len()
            && applied_commands >= max_commands,
        current_strip: state.current_strip,
        current_caption: state.current_caption,
        drawable_actor_count: state.drawable_actor_count(),
        total_ticks: state.total_ticks,
        temporary_actor_draws: state.temporary_actor_draws,
        fixed_wipes: state.fixed_wipes,
        cell_effect_steps: state.cell_effect_steps,
        fixed_wipe_rectangle_steps: state.fixed_wipe_rectangle_steps,
        cached_effect_cell: state.cached_effect_cell,
    };
    Ok(ReturnToViewPlayback {
        frames,
        run: ReturnToViewPreviewRun { state, report },
    })
}

fn append_return_to_view_playback_frames(
    frames: &mut Vec<ReturnToViewPlaybackFrame>,
    command_index: usize,
    command: ReturnToViewCommand,
    before_ticks: u32,
    strip_load_tick: u32,
    before_state: &ReturnToViewPreviewState,
    state: &ReturnToViewPreviewState,
) {
    match command {
        ReturnToViewCommand::RunPreviewTick { ticks } => {
            for tick in 1..=ticks {
                push_return_to_view_frame(
                    frames,
                    command_index,
                    before_ticks + u32::from(tick),
                    ReturnToViewFrameKind::PreviewTick,
                    state,
                    strip_load_tick,
                );
            }
        }
        ReturnToViewCommand::LoadMapStrip { .. } => {}
        ReturnToViewCommand::OpenCellEffect { x, y } => {
            // `#54`: `y` is a plane row (`0..3`); the `+ 7` screen row
            // offset belongs to the renderer.
            let screen_y = y;
            for step in 1..=RTV_CELL_EFFECT_STEPS {
                push_return_to_view_cell_effect_frame(
                    frames,
                    command_index,
                    before_ticks + u32::from(step),
                    ReturnToViewFrameKind::CellEffectStep { step },
                    before_state,
                    x,
                    screen_y,
                    RTV_EFFECT_SENTINEL_TILE,
                    strip_load_tick,
                );
            }
            for tick in 0..RTV_CELL_EFFECT_FINAL_TICKS {
                push_return_to_view_cell_effect_frame(
                    frames,
                    command_index,
                    before_ticks + u32::from(RTV_CELL_EFFECT_STEPS) + u32::from(tick) + 1,
                    ReturnToViewFrameKind::CellEffectFinalTick { tick },
                    state,
                    x,
                    screen_y,
                    RTV_OPEN_EFFECT_FINAL_TILE,
                    strip_load_tick,
                );
            }
        }
        ReturnToViewCommand::CloseCellEffect => {
            let (x, y) = before_state
                .cached_effect_cell
                .expect("Return-to-View close-cell playback has no cached open-cell coordinate");
            for offset in 0..RTV_CELL_EFFECT_STEPS {
                let step = RTV_CELL_EFFECT_STEPS - offset;
                push_return_to_view_cell_effect_frame(
                    frames,
                    command_index,
                    before_ticks + u32::from(offset) + 1,
                    ReturnToViewFrameKind::CellEffectStep { step },
                    before_state,
                    x,
                    y,
                    RTV_EFFECT_SENTINEL_TILE,
                    strip_load_tick,
                );
            }
            for tick in 0..RTV_CELL_EFFECT_FINAL_TICKS {
                push_return_to_view_cell_effect_frame(
                    frames,
                    command_index,
                    before_ticks + u32::from(RTV_CELL_EFFECT_STEPS) + u32::from(tick) + 1,
                    ReturnToViewFrameKind::CellEffectFinalTick { tick },
                    state,
                    x,
                    y,
                    RTV_CLOSE_EFFECT_FINAL_TILE,
                    strip_load_tick,
                );
            }
        }
        ReturnToViewCommand::TemporaryActorDraw { slot } => {
            push_return_to_view_actor_draw_frames(
                frames,
                command_index,
                before_ticks,
                before_state,
                slot,
                ReturnToViewActorDrawControlSource::OriginalActorTile,
                strip_load_tick,
            );
        }
        ReturnToViewCommand::TemporaryActorDrawOverBacking { slot } => {
            push_return_to_view_actor_draw_frames(
                frames,
                command_index,
                before_ticks,
                before_state,
                slot,
                ReturnToViewActorDrawControlSource::BackingMapTile,
                strip_load_tick,
            );
        }
        ReturnToViewCommand::FixedWipeAndActorDraw { slot, .. } => {
            let mut elapsed = before_ticks;
            for step in 0..RTV_FIXED_WIPE_STEPS {
                elapsed += 1;
                push_return_to_view_frame(
                    frames,
                    command_index,
                    elapsed,
                    ReturnToViewFrameKind::FixedWipeRectangle { step },
                    state,
                    strip_load_tick,
                );
            }
            push_return_to_view_fixed_actor_draw_frame(frames, command_index, elapsed, state, slot);
            // `#54`: no fixed eight-tick wait — three tail ticks only.
            for tick in 0..RTV_FIXED_WIPE_TRAILING_TICKS {
                elapsed += 1;
                push_return_to_view_frame(
                    frames,
                    command_index,
                    elapsed,
                    ReturnToViewFrameKind::FixedWipeTrailingTick { tick },
                    state,
                    strip_load_tick,
                );
            }
        }
        ReturnToViewCommand::MoveActorAndTick { .. } => {
            // `#54`: `0x0D` runs seven ticks at its tail.
            for tick in 1..=u32::from(RTV_MOVE_ACTOR_AND_TICK_TICKS) {
                push_return_to_view_frame(
                    frames,
                    command_index,
                    before_ticks + tick,
                    ReturnToViewFrameKind::MoveActorTick,
                    state,
                    strip_load_tick,
                );
            }
        }
        _ => {}
    }
}

fn push_return_to_view_cell_effect_frame(
    frames: &mut Vec<ReturnToViewPlaybackFrame>,
    command_index: usize,
    elapsed_title_ticks: u32,
    kind: ReturnToViewFrameKind,
    state: &ReturnToViewPreviewState,
    x: u8,
    y: u8,
    tile: u8,
    strip_load_tick: u32,
) {
    let mut state = state.clone();
    let index = preview_cell_index_checked(x, y)
        .expect("Return-to-View cell-effect playback coordinate is outside preview buffer");
    state.terrain[index] = tile;
    state.backing[index] = tile;
    state.cached_effect_cell = Some((x, y));
    state.set_playback_tick(elapsed_title_ticks, strip_load_tick);
    // audio.md §8.6: a cell-effect raster step is a scheduled preview tick and
    // carries the current strip's ambient cue.
    let sound = return_to_view_tick_sound(state.current_strip, state.ticks_since_strip_load);
    frames.push(ReturnToViewPlaybackFrame {
        command_index,
        elapsed_title_ticks,
        kind,
        state,
        actor_draw: None,
        sound,
    });
}

fn push_return_to_view_frame(
    frames: &mut Vec<ReturnToViewPlaybackFrame>,
    command_index: usize,
    elapsed_title_ticks: u32,
    kind: ReturnToViewFrameKind,
    state: &ReturnToViewPreviewState,
    strip_load_tick: u32,
) {
    let mut state = state.clone();
    state.set_playback_tick(elapsed_title_ticks, strip_load_tick);
    // audio.md §8.6: every scheduled preview tick — preview, fixed-wipe
    // rectangle, trailing and move-actor alike — carries the strip's cue.
    let sound = return_to_view_tick_sound(state.current_strip, state.ticks_since_strip_load);
    frames.push(ReturnToViewPlaybackFrame {
        command_index,
        elapsed_title_ticks,
        kind,
        state,
        actor_draw: None,
        sound,
    });
}

fn push_return_to_view_actor_draw_frames(
    frames: &mut Vec<ReturnToViewPlaybackFrame>,
    command_index: usize,
    before_ticks: u32,
    state: &ReturnToViewPreviewState,
    slot: u8,
    control_source: ReturnToViewActorDrawControlSource,
    strip_load_tick: u32,
) {
    let slot_index = usize::from(slot);
    let actor = *state
        .actors
        .get(slot_index)
        .expect("Return-to-View actor draw slot is outside actor table");
    let original_tile = actor.tile0;
    let backing_index = preview_cell_index_checked(actor.x, actor.y)
        .expect("Return-to-View actor draw coordinate is outside preview buffer");
    let backing_tile = state.backing[backing_index];
    for group in 1..=RTV_SINGLE_CELL_WRITES / RTV_SINGLE_CELL_WRITES_PER_CHECKPOINT {
        let completed_writes = group * RTV_SINGLE_CELL_WRITES_PER_CHECKPOINT;
        let elapsed_title_ticks =
            before_ticks + u32::from(group.min(u16::from(RTV_SINGLE_CELL_CHECKPOINTS)));
        let mut frame_state = state.clone();
        frame_state.actors[slot_index].tile0 = RTV_TEMPORARY_ACTOR_TILE;
        frame_state.actors[slot_index].tile1 = RTV_TEMPORARY_ACTOR_TILE;
        frame_state.terrain[backing_index] = 0;
        frame_state.overlay[backing_index] = RTV_TEMPORARY_ACTOR_TILE;
        frame_state.set_playback_tick(elapsed_title_ticks, strip_load_tick);
        // `u5-spec#117`: input is checked through a full preview tick after
        // every eight writes *except the final group*, so group 32 shares
        // group 31's tick and schedules none of its own. audio.md §8.6 ties
        // the cue to the scheduled tick, so the final group is silent.
        let sound = if group <= u16::from(RTV_SINGLE_CELL_CHECKPOINTS) {
            return_to_view_tick_sound(
                frame_state.current_strip,
                frame_state.ticks_since_strip_load,
            )
        } else {
            None
        };
        let (kind, source, tile, control) = match control_source {
            ReturnToViewActorDrawControlSource::OriginalActorTile => (
                ReturnToViewFrameKind::TemporaryActorDraw { completed_writes },
                ReturnToViewActorDrawSource::OverlayTile,
                original_tile,
                ReturnToViewActorDrawControl::OriginalActorTile(original_tile),
            ),
            ReturnToViewActorDrawControlSource::BackingMapTile => (
                ReturnToViewFrameKind::TemporaryActorDrawOverBacking { completed_writes },
                ReturnToViewActorDrawSource::BackingTerrainTile,
                backing_tile,
                ReturnToViewActorDrawControl::BackingMapTile(backing_tile),
            ),
        };
        frames.push(ReturnToViewPlaybackFrame {
            command_index,
            elapsed_title_ticks,
            kind,
            state: frame_state,
            actor_draw: Some(ReturnToViewActorDraw {
                slot,
                tile,
                x: actor.x,
                y: actor.y,
                screen_y: return_to_view_actor_screen_y(actor.y),
                source,
                control,
            }),
            sound,
        });
    }
}

fn push_return_to_view_fixed_actor_draw_frame(
    frames: &mut Vec<ReturnToViewPlaybackFrame>,
    command_index: usize,
    elapsed_title_ticks: u32,
    state: &ReturnToViewPreviewState,
    slot: u8,
) {
    let slot_index = usize::from(slot);
    let actor = *state
        .actors
        .get(slot_index)
        .expect("Return-to-View fixed actor draw slot is outside actor table");
    let _ = preview_cell_index_checked(actor.x, actor.y)
        .expect("Return-to-View fixed actor draw coordinate is outside preview buffer");
    frames.push(ReturnToViewPlaybackFrame {
        command_index,
        elapsed_title_ticks,
        kind: ReturnToViewFrameKind::FixedWipeActorDraw,
        state: state.clone(),
        actor_draw: Some(ReturnToViewActorDraw {
            slot,
            tile: actor.tile0,
            x: actor.x,
            y: actor.y,
            screen_y: return_to_view_actor_screen_y(actor.y),
            source: ReturnToViewActorDrawSource::CurrentActorTile,
            control: ReturnToViewActorDrawControl::Zero,
        }),
        // audio.md §8.6: this draw reuses the wipe's last tick rather than
        // scheduling one, so it carries no ambient cue of its own.
        sound: None,
    });
}

pub fn summarize_return_to_view_preview(
    strips: &ReturnToViewMapStrips,
    script: &ReturnToViewScript,
) -> io::Result<String> {
    let playback = run_return_to_view_playback_until_restart(strips, script, 4096)?;
    let report = playback.run.report;
    let end = if report.restart_seen {
        "reaches the stream restart"
    } else if report.max_commands_reached {
        "hit the dry-run command cap"
    } else {
        "reaches end of stream"
    };
    Ok(format!(
        "Dry run {end} after {} applied command(s); emits {} playback frame(s); current strip {:?}, {} drawable actor(s), {} scheduled tick(s), {} cell-effect step(s), {} temporary draw(s), {} fixed wipe(s), {} fixed-wipe rectangle step(s).",
        report.applied_commands,
        playback.frames.len(),
        report.current_strip,
        report.drawable_actor_count,
        report.total_ticks,
        report.cell_effect_steps,
        report.temporary_actor_draws,
        report.fixed_wipes,
        report.fixed_wipe_rectangle_steps
    ))
}

pub fn render_return_to_view_preview_viewport(
    strips: &ReturnToViewMapStrips,
    script: &ReturnToViewScript,
    atlas: &TileAtlas,
) -> io::Result<(TileViewport, ReturnToViewPreviewReport)> {
    render_return_to_view_preview_viewport_at_title_tick(strips, script, atlas, 0)
}

pub fn render_return_to_view_preview_viewport_at_title_tick(
    strips: &ReturnToViewMapStrips,
    script: &ReturnToViewScript,
    atlas: &TileAtlas,
    starting_title_tick: u32,
) -> io::Result<(TileViewport, ReturnToViewPreviewReport)> {
    let run = run_return_to_view_preview_state_until_restart(strips, script, 4096)?;
    let viewport = render_return_to_view_state_viewport(
        &run.state,
        atlas,
        return_to_view_animation_frame(starting_title_tick, run.report.total_ticks),
        None,
    )?;
    Ok((viewport, run.report))
}

pub fn render_return_to_view_playback_frame_viewport(
    frame: &ReturnToViewPlaybackFrame,
    atlas: &TileAtlas,
    starting_title_tick: u32,
) -> io::Result<TileViewport> {
    render_return_to_view_playback_frame_over(frame, atlas, starting_title_tick, None)
}

/// `#54`: painting is cell-granular over preserved backing, so a
/// playback frame is rendered on top of the previous frame's raster.
pub fn render_return_to_view_playback_frame_over(
    frame: &ReturnToViewPlaybackFrame,
    atlas: &TileAtlas,
    starting_title_tick: u32,
    already_painted: Option<&TileViewport>,
) -> io::Result<TileViewport> {
    let mut viewport = render_return_to_view_state_viewport(
        &frame.state,
        atlas,
        return_to_view_animation_frame(starting_title_tick, frame.elapsed_title_ticks),
        already_painted,
    )?;
    match frame.kind {
        ReturnToViewFrameKind::CellEffectStep { step } => {
            let (x, y) = frame.state.cached_effect_cell.ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Return-to-View cell-effect frame has no cached target cell",
                )
            })?;
            blit_return_to_view_cell_effect_raster(
                &mut viewport,
                atlas,
                usize::from(x),
                usize::from(y),
                step,
            )?;
        }
        ReturnToViewFrameKind::TemporaryActorDraw { completed_writes }
        | ReturnToViewFrameKind::TemporaryActorDrawOverBacking { completed_writes } => {
            let actor_draw = frame.actor_draw.as_ref().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Return-to-View convergence frame has no actor-draw metadata",
                )
            })?;
            blit_return_to_view_single_cell_prefix(
                &mut viewport,
                atlas,
                actor_draw,
                completed_writes,
            )?;
        }
        _ => {}
    }
    Ok(viewport)
}

/// `#54` preview tick: every tick advances the animated-tile frame
/// table, so the frame selector is the elapsed tick count.
pub fn return_to_view_animation_frame(starting_title_tick: u32, elapsed_ticks: u32) -> u8 {
    let total = starting_title_tick
        .checked_add(elapsed_ticks)
        .expect("Return-to-View animation frame counter overflowed");
    (total % u32::from(STATIC_TILE_ANIMATION_PERIOD_TICKS)) as u8
}

/// Render one preview state into the published `304 x 64` strip raster.
///
/// `cleak/u5-spec#54`: the strip is nineteen `16 x 16` `TILES` cells
/// across by four down, drawn through the same viewport tile entry the
/// world view uses — there is no miniature raster and no scaled resident
/// path. The caller blits the result at [`RTV_PREVIEW_PIXEL_X`],
/// [`RTV_PREVIEW_PIXEL_Y`].
///
/// Painting is cell-granular over preserved backing: only the cells
/// inside the reveal cursor's column span are painted, cells a helper
/// owns are skipped, and there is no clear and no full repaint.
/// `already_painted` carries the previous frame's raster so untouched
/// cells keep whatever is already on screen.
fn render_return_to_view_state_viewport(
    state: &ReturnToViewPreviewState,
    atlas: &TileAtlas,
    animation_frame: u8,
    already_painted: Option<&TileViewport>,
) -> io::Result<TileViewport> {
    let width = RTV_PREVIEW_PIXEL_WIDTH;
    let height = RTV_PREVIEW_PIXEL_HEIGHT;
    let mut viewport = match already_painted {
        Some(previous) => {
            if previous.width != width || previous.height != height {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "Return-to-View backing raster is {}x{}, expected {width}x{height}",
                        previous.width, previous.height
                    ),
                ));
            }
            previous.clone()
        }
        None => TileViewport {
            depth: atlas.depth,
            cells_wide: RTV_STRIP_VISIBLE_COLUMNS,
            cells_high: RTV_STRIP_VISIBLE_ROWS,
            width,
            height,
            pixels: vec![0; width * height],
        },
    };

    let Some((first_column, last_column)) = state.revealed_columns() else {
        // Before the chapter's first preview tick the cursor paints
        // nothing; the previous contents stand.
        return Ok(viewport);
    };

    for cell_y in 0..RTV_STRIP_VISIBLE_ROWS {
        for cell_x in first_column..=last_column {
            let index = cell_y * RTV_STRIP_VISIBLE_COLUMNS + cell_x;
            let Some(tile) = return_to_view_cell_tile_index(
                state.terrain[index],
                state.overlay[index],
                animation_frame,
            ) else {
                // `#54`: another helper owns this cell this frame.
                continue;
            };
            blit_tile_index_to_viewport(&mut viewport, atlas, tile, cell_x, cell_y)?;
        }
    }

    // Drawable actors are the helper that owns their cells; they paint
    // on top with the sprite's transparent pixel left alone.
    for actor in state.actors.iter().filter(|actor| {
        actor.drawable
            && actor.tile0 != RTV_TEMPORARY_ACTOR_TILE
            && actor.tile1 != RTV_TEMPORARY_ACTOR_TILE
    }) {
        let x = usize::from(actor.x);
        let y = usize::from(actor.y);
        if x >= RTV_STRIP_VISIBLE_COLUMNS || y >= RTV_STRIP_VISIBLE_ROWS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Return-to-View actor at preview cell ({x}, {y}) falls outside the {RTV_STRIP_VISIBLE_COLUMNS}x{RTV_STRIP_VISIBLE_ROWS} strip"
                ),
            ));
        }
        if x < first_column || x > last_column {
            continue;
        }
        blit_return_to_view_actor_to_viewport(
            &mut viewport,
            atlas,
            RTV_OVERLAY_TILE_BASE + usize::from(actor.tile0),
            x,
            y,
        )?;
    }
    Ok(viewport)
}

/// `u5-spec#117`: exact carry-set `0x66` permutation. The corner is written
/// first, followed by every nonzero eight-bit Galois state using tap `0xB8`.
pub fn return_to_view_single_cell_write_coordinates() -> [(u8, u8); 256] {
    let mut coordinates = [(0u8, 0u8); 256];
    let mut state = 1u8;
    let mut write = 1usize;
    while write < coordinates.len() {
        coordinates[write] = (state >> 4, state & 0x0f);
        let old_low_bit = state & 1;
        state >>= 1;
        if old_low_bit != 0 {
            state ^= RTV_SINGLE_CELL_GALOIS_TAP;
        }
        write += 1;
    }
    debug_assert_eq!(state, 1);
    coordinates
}

fn blit_return_to_view_cell_effect_raster(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    cell_x: usize,
    cell_y: usize,
    step: u8,
) -> io::Result<()> {
    if !(1..=RTV_CELL_EFFECT_STEPS).contains(&step) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Return-to-View cell-effect step {step} is outside 1..=15"),
        ));
    }
    let base = atlas
        .tile_pixels(usize::from(RTV_CLOSE_EFFECT_FINAL_TILE))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "tile atlas is missing base tile 0x05",
            )
        })?;
    let portal = atlas
        .tile_pixels(usize::from(RTV_OPEN_EFFECT_FINAL_TILE))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "tile atlas is missing portal tile 0xDC",
            )
        })?;
    let split = TILE_ATLAS_SIDE - usize::from(step);
    let dst_x = cell_x * TILE_ATLAS_SIDE;
    let dst_y = cell_y * TILE_ATLAS_SIDE;
    for y in 0..TILE_ATLAS_SIDE {
        let (source, source_y) = if y < split {
            (base, y)
        } else {
            (portal, y - split)
        };
        let source_start = source_y * TILE_ATLAS_SIDE;
        let destination_start = (dst_y + y) * viewport.width + dst_x;
        viewport.pixels[destination_start..destination_start + TILE_ATLAS_SIDE]
            .copy_from_slice(&source[source_start..source_start + TILE_ATLAS_SIDE]);
    }
    Ok(())
}

fn blit_return_to_view_single_cell_prefix(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    actor_draw: &ReturnToViewActorDraw,
    completed_writes: u16,
) -> io::Result<()> {
    if completed_writes == 0 || completed_writes > RTV_SINGLE_CELL_WRITES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "Return-to-View convergence prefix {completed_writes} is outside 1..={RTV_SINGLE_CELL_WRITES}"
            ),
        ));
    }
    let tile = match actor_draw.source {
        ReturnToViewActorDrawSource::OverlayTile => {
            RTV_OVERLAY_TILE_BASE + usize::from(actor_draw.tile)
        }
        ReturnToViewActorDrawSource::BackingTerrainTile => usize::from(actor_draw.tile),
        ReturnToViewActorDrawSource::CurrentActorTile => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "fixed actor draw cannot drive single-cell convergence",
            ));
        }
    };
    let source = atlas.tile_pixels(tile).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tile atlas is missing convergence source tile {tile}"),
        )
    })?;
    let destination_x = usize::from(actor_draw.x) * TILE_ATLAS_SIDE;
    let destination_y = usize::from(actor_draw.y) * TILE_ATLAS_SIDE;
    for &(x, y) in return_to_view_single_cell_write_coordinates()
        .iter()
        .take(usize::from(completed_writes))
    {
        let x = usize::from(x);
        let y = usize::from(y);
        viewport.pixels[(destination_y + y) * viewport.width + destination_x + x] =
            source[y * TILE_ATLAS_SIDE + x];
    }
    Ok(())
}

/// Blit one 512-entry atlas tile index into a preview cell.
fn blit_tile_index_to_viewport(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    tile: usize,
    cell_x: usize,
    cell_y: usize,
) -> io::Result<()> {
    let tile_pixels = atlas.tile_pixels(tile).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tile atlas is missing tile {tile}"),
        )
    })?;
    let dst_x = cell_x * TILE_ATLAS_SIDE;
    let dst_y = cell_y * TILE_ATLAS_SIDE;
    for row in 0..TILE_ATLAS_SIDE {
        let dst_start = (dst_y + row) * viewport.width + dst_x;
        let src_start = row * TILE_ATLAS_SIDE;
        viewport.pixels[dst_start..dst_start + TILE_ATLAS_SIDE]
            .copy_from_slice(&tile_pixels[src_start..src_start + TILE_ATLAS_SIDE]);
    }
    Ok(())
}

pub const fn return_to_view_fixed_wipe_rectangles(
    step: u8,
) -> Option<[((u16, u16), (u16, u16)); 2]> {
    if step >= RTV_FIXED_WIPE_STEPS {
        return None;
    }
    let x0 = 128 + 9 * step as u16;
    let y0 = 152 + 3 * step as u16;
    let x1 = 137 + 9 * step as u16;
    let y1 = 155 + 3 * step as u16;
    Some([((x0, y0), (x1, y1)), ((x0, y0 + 1), (x1, y1 + 1))])
}

fn blit_return_to_view_actor_to_viewport(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    tile: usize,
    cell_x: usize,
    cell_y: usize,
) -> io::Result<()> {
    let tile_pixels = atlas.tile_pixels(tile).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tile atlas is missing tile {tile}"),
        )
    })?;
    let dst_x = cell_x * TILE_ATLAS_SIDE;
    let dst_y = cell_y * TILE_ATLAS_SIDE;
    for row in 0..TILE_ATLAS_SIDE {
        let dst_start = (dst_y + row) * viewport.width + dst_x;
        let src_start = row * TILE_ATLAS_SIDE;
        for col in 0..TILE_ATLAS_SIDE {
            let pixel = tile_pixels[src_start + col];
            if pixel != RTV_ACTOR_TRANSPARENT_PIXEL {
                viewport.pixels[dst_start + col] = pixel;
            }
        }
    }
    Ok(())
}

fn rtv_slot_index(slot: u8) -> io::Result<usize> {
    let slot = usize::from(slot);
    if slot >= RTV_ACTOR_SLOTS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Return-to-View actor slot {slot} is out of range"),
        ));
    }
    Ok(slot)
}

fn preview_cell_index(x: u8, y: u8) -> Option<usize> {
    let x = usize::from(x);
    let y = usize::from(y);
    if x >= RTV_STRIP_VISIBLE_COLUMNS || y >= RTV_STRIP_VISIBLE_ROWS {
        return None;
    }
    Some(y * RTV_STRIP_VISIBLE_COLUMNS + x)
}

fn preview_cell_index_checked(x: u8, y: u8) -> io::Result<usize> {
    preview_cell_index(x, y).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Return-to-View coordinate ({x}, {y}) is outside the {RTV_STRIP_VISIBLE_COLUMNS}x{RTV_STRIP_VISIBLE_ROWS} preview strip"
            ),
        )
    })
}

fn rtv_step_coordinate(x: u8, y: u8, direction: u8) -> io::Result<(u8, u8)> {
    let (dx, dy) = match direction {
        0 => (0i16, -1i16),
        1 => (1, 0),
        2 => (0, 1),
        3 => (-1, 0),
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Return-to-View direction {direction} is out of range"),
            ));
        }
    };
    // `#54`: script coordinates never leave `x = 0..18` / `y = 0..3`,
    // so a step that would leave the strip is a data fault, not
    // something to wrap or clip.
    let nx = i16::from(x) + dx;
    let ny = i16::from(y) + dy;
    if !(0..RTV_STRIP_VISIBLE_COLUMNS as i16).contains(&nx)
        || !(0..RTV_STRIP_VISIBLE_ROWS as i16).contains(&ny)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "Return-to-View actor step to ({nx}, {ny}) leaves the {RTV_STRIP_VISIBLE_COLUMNS}x{RTV_STRIP_VISIBLE_ROWS} preview strip"
            ),
        ));
    }
    Ok((nx as u8, ny as u8))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn padded_stream(prefix: &[u8]) -> Vec<u8> {
        let mut stream = prefix.to_vec();
        stream.resize(RTV_COMMAND_STREAM_BYTES, 0x09);
        stream
    }

    #[test]
    fn parse_return_to_view_commands_decodes_fixed_width_opcodes() {
        let stream = padded_stream(&[
            0x00, 1, 2, 3, 4, 0x01, 5, 0x02, 6, 1, 0x03, 7, 0x04, 8, 9, 0x05, 0x06, 2, 0x07, 3,
            0x08, 4, 0x09, 0x0a, 10, 11, 12, 0x0b, 0xaa, 0xbb, 13, 0x0c, 0x0d, 14, 3, 0x0e, 4,
            0x0f,
        ]);

        let script = parse_return_to_view_commands(&stream).unwrap();

        assert_eq!(
            script.commands[0],
            ReturnToViewCommand::SetActor {
                slot: 1,
                tile: 2,
                x: 3,
                y: 4
            }
        );
        assert_eq!(
            script.commands[11],
            ReturnToViewCommand::FixedWipeAndActorDraw {
                reserved0: 0xaa,
                reserved1: 0xbb,
                slot: 13
            }
        );
        assert_eq!(script.opcode_count(0x09), RTV_COMMAND_STREAM_BYTES - 38 + 1);
    }

    #[test]
    fn parse_return_to_view_commands_treats_high_opcodes_as_noops() {
        let stream = padded_stream(&[0xf0, 0x09]);

        let script = parse_return_to_view_commands(&stream).unwrap();

        assert_eq!(
            script.commands[0],
            ReturnToViewCommand::NoOp { opcode: 0xf0 }
        );
        assert_eq!(script.no_op_count(), 1);
        assert_eq!(script.known_command_count(), script.commands.len() - 1);
    }

    #[test]
    fn parse_return_to_view_commands_rejects_truncated_argument() {
        let mut stream = vec![0x09; RTV_COMMAND_STREAM_BYTES];
        stream[RTV_COMMAND_STREAM_BYTES - 2] = 0x00;
        stream[RTV_COMMAND_STREAM_BYTES - 1] = 1;

        let err = parse_return_to_view_commands(&stream).unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("requires 4 argument"));
    }

    #[test]
    fn parse_return_to_view_script_file_reads_published_section() {
        let mut file = vec![0u8; MISCMAPS_RTV_COMMAND_SECTION_OFFSET];
        file.extend(padded_stream(&[0x06, 2, 0x09]));

        let script = parse_return_to_view_script_file(&file).unwrap();

        assert_eq!(
            script.commands[0],
            ReturnToViewCommand::LoadMapStrip { strip: 2 }
        );
        assert_eq!(script.opcode_count(0x06), 1);
        assert_eq!(script.opcode_count(0x09), RTV_COMMAND_STREAM_BYTES - 2);
    }

    #[test]
    fn parse_return_to_view_map_strips_extracts_visible_cells_and_skips_padding() {
        // `cleak/u5-spec#54` retraction 1: four 32-byte *rows* per
        // record, nineteen tile bytes each, thirteen bytes of padding.
        let mut bytes = vec![0xee; MISCMAPS_RTV_STRIP_SECTION_BYTES];
        for strip in 0..RTV_STRIP_COUNT {
            for source_row in 0..RTV_STRIP_SOURCE_ROWS {
                let row_start =
                    strip * RTV_STRIP_RECORD_BYTES + source_row * MISCMAPS_RTV_STRIP_ROW_STRIDE;
                for source_col in 0..RTV_STRIP_SOURCE_COLUMNS {
                    bytes[row_start + source_col] =
                        (strip * 100 + source_row * 19 + source_col) as u8;
                }
            }
        }

        let strips = parse_return_to_view_map_strips(&bytes).unwrap();

        assert_eq!(strips.strips[0].len(), 19 * 4);
        assert_eq!(strips.strips[0][0], 0);
        assert_eq!(strips.strips[0][18], 18);
        assert_eq!(strips.strips[0][19], 19);
        assert_eq!(strips.strips[0][75], 75);
        assert_eq!(strips.strips[1][0], 100);
        // The thirteen padding bytes of each source row never reach the
        // preview.
        assert!(!strips.strips[0].contains(&0xee));
    }

    #[test]
    fn return_to_view_state_loads_strip_and_applies_actor_map_effects() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[2][0] = 0x11;
        strips.strips[2][1] = 0x12;
        let mut state = ReturnToViewPreviewState::default();

        state
            .apply_command(&strips, 0, ReturnToViewCommand::LoadMapStrip { strip: 2 })
            .unwrap();
        state
            .apply_command(
                &strips,
                1,
                ReturnToViewCommand::SetActor {
                    slot: 3,
                    tile: 0x44,
                    x: 1,
                    y: 0,
                },
            )
            .unwrap();
        state
            .apply_command(
                &strips,
                2,
                ReturnToViewCommand::MoveActor {
                    slot: 3,
                    direction: 1,
                },
            )
            .unwrap();
        state
            .apply_command(
                &strips,
                3,
                ReturnToViewCommand::OpenCellEffect { x: 2, y: 2 },
            )
            .unwrap();
        state
            .apply_command(&strips, 4, ReturnToViewCommand::CloseCellEffect)
            .unwrap();

        assert_eq!(state.current_strip, Some(2));
        assert_eq!(state.cell(0, 0), Some(0x11));
        assert_eq!(state.cell(1, 0), Some(0x12));
        assert_eq!(state.actors[3].x, 2);
        assert_eq!(state.actors[3].tile0, 0x44);
        assert_eq!(state.cell(2, 2), Some(RTV_CLOSE_EFFECT_FINAL_TILE));
        assert_eq!(state.total_ticks, 34);
        assert_eq!(state.cell_effect_steps, 30);
    }

    #[test]
    fn run_return_to_view_preview_follows_loop_until_restart() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][0] = 7;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::LoopStart { count: 3 },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::LoopEnd,
                ReturnToViewCommand::RestartStream,
            ],
        };

        let report = run_return_to_view_preview_until_restart(&strips, &script, 32).unwrap();

        assert!(report.restart_seen);
        assert_eq!(report.applied_commands, 9);
        assert_eq!(report.current_strip, Some(0));
        assert_eq!(report.total_ticks, 6);
        assert!(!report.max_commands_reached);
    }

    #[test]
    fn fixed_wipe_schedule_matches_public_return_to_view_helper() {
        let strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        let mut state = ReturnToViewPreviewState::default();
        state
            .apply_command(
                &strips,
                0,
                ReturnToViewCommand::SetActor {
                    slot: 2,
                    tile: 0x44,
                    x: 1,
                    y: 1,
                },
            )
            .unwrap();
        state
            .apply_command(
                &strips,
                1,
                ReturnToViewCommand::FixedWipeAndActorDraw {
                    reserved0: 0,
                    reserved1: 0,
                    slot: 2,
                },
            )
            .unwrap();

        // `cleak/u5-spec#54` retraction 2: the eight-tick fixed wait was
        // fabricated. `0x0B` runs five wipe steps plus three tail ticks.
        assert_eq!(RTV_FIXED_WIPE_TOTAL_TICKS, 8);
        assert_eq!(state.fixed_wipes, 1);
        assert_eq!(state.fixed_wipe_rectangle_steps, 5);
        assert_eq!(state.total_ticks, 8);
        assert_eq!(
            return_to_view_fixed_wipe_rectangles(0),
            Some([((128, 152), (137, 155)), ((128, 153), (137, 156))])
        );
        assert_eq!(
            return_to_view_fixed_wipe_rectangles(4),
            Some([((164, 164), (173, 167)), ((164, 165), (173, 168))])
        );
        assert_eq!(return_to_view_fixed_wipe_rectangles(5), None);
    }

    #[test]
    fn return_to_view_actor_movement_off_the_strip_is_a_data_fault() {
        // `cleak/u5-spec#54`: "script coordinates never leave
        // `x = 0..18` / `y = 0..3`, so nothing is clipped and no
        // clipping rule is needed". A step off the edge is therefore a
        // corrupt-script fault rather than a wrap.
        let strips = ReturnToViewMapStrips {
            strips: [[0x44; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        let mut state = ReturnToViewPreviewState::default();
        state
            .apply_command(
                &strips,
                0,
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 1,
                    x: 0,
                    y: 0,
                },
            )
            .unwrap();

        let err = state
            .apply_command(
                &strips,
                1,
                ReturnToViewCommand::MoveActor {
                    slot: 0,
                    direction: 3,
                },
            )
            .expect_err("a westward step off column 0 must fail");
        assert!(
            err.to_string().contains("leaves the 19x4 preview strip"),
            "unexpected error: {err}"
        );
        assert_eq!(state.actors[0].x, 0);
        assert_eq!(state.actors[0].y, 0);
    }

    #[test]
    fn return_to_view_terrain_resolves_through_the_world_animation_table() {
        // `cleak/u5-spec#54` retraction 4: this helper has no bespoke
        // family table of its own. A non-zero terrain byte indexes the
        // same animated-tile frame table the world renderer cycles, so
        // the `animation.md §6` families (spec HEAD `c00bf63`) — and
        // only those — move here.

        // Water is not an animated family; the earlier three-frame water
        // cycle asserted here is withdrawn by §6.
        assert_eq!(return_to_view_terrain_tile_for_frame(0x01, 0), 0x01);
        assert_eq!(return_to_view_terrain_tile_for_frame(0x01, 2), 0x01);
        assert_eq!(return_to_view_terrain_tile_for_frame(0x03, 4), 0x03);
        // Grass stays grass at every phase.
        assert_eq!(return_to_view_terrain_tile_for_frame(0x05, 9), 0x05);

        // Waterfall `0xD4..0xD7`: ungated, so the selector has advanced
        // once per elapsed phase.
        assert_eq!(return_to_view_terrain_tile_for_frame(0xD4, 0), 0xD4);
        assert_eq!(return_to_view_terrain_tile_for_frame(0xD4, 1), 0xD5);
        assert_eq!(return_to_view_terrain_tile_for_frame(0xD4, 6), 0xD6);
        // Fountain `0xD8..0xDB`, also ungated; the id keeps its own
        // quarter-cycle offset.
        assert_eq!(return_to_view_terrain_tile_for_frame(0xD8, 5), 0xD9);
        assert_eq!(return_to_view_terrain_tile_for_frame(0xDA, 5), 0xDB);
        // Pendulum `0x80..0x83`: half rate behind the bit-0 gate, so
        // three elapsed phases (0, 1, 2) are one advance.
        assert_eq!(return_to_view_terrain_tile_for_frame(0x80, 3), 0x81);
        assert_eq!(return_to_view_terrain_tile_for_frame(0x80, 2), 0x81);
        assert_eq!(return_to_view_terrain_tile_for_frame(0x80, 1), 0x80);
    }

    #[test]
    fn return_to_view_load_strip_is_single_untimed_script_action() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        for row in 0..RTV_STRIP_VISIBLE_ROWS {
            for col in 0..RTV_STRIP_VISIBLE_COLUMNS {
                strips.strips[0][row * RTV_STRIP_VISIBLE_COLUMNS + col] =
                    10 + row as u8 + col as u8;
            }
        }
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::RestartStream,
            ],
        };

        let playback = run_return_to_view_playback_until_restart(&strips, &script, 32).unwrap();

        assert!(playback.frames.is_empty());
        assert_eq!(playback.run.report.total_ticks, 0);
        assert_eq!(playback.run.state.terrain[0], 10);
        assert_eq!(
            playback.run.state.terrain[RTV_STRIP_VISIBLE_COLUMNS + 9],
            10 + 1 + 9
        );
        assert_eq!(
            playback.run.state.terrain[3 * RTV_STRIP_VISIBLE_COLUMNS + 18],
            10 + 3 + 18
        );
    }

    #[test]
    fn return_to_view_cell_effect_playback_stages_sentinel_then_final_tile() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][1 * RTV_STRIP_VISIBLE_COLUMNS + 2] = 0x33;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::OpenCellEffect { x: 2, y: 1 },
                ReturnToViewCommand::CloseCellEffect,
                ReturnToViewCommand::RestartStream,
            ],
        };

        let playback = run_return_to_view_playback_until_restart(&strips, &script, 96).unwrap();
        let effect_index = preview_cell_index(2, 1).unwrap();

        let open_steps = playback
            .frames
            .iter()
            .filter_map(|frame| {
                (frame.command_index == 1)
                    .then_some(frame)
                    .and_then(|frame| {
                        if let ReturnToViewFrameKind::CellEffectStep { step } = frame.kind {
                            Some((step, frame))
                        } else {
                            None
                        }
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            open_steps.iter().map(|(step, _)| *step).collect::<Vec<_>>(),
            (1..=RTV_CELL_EFFECT_STEPS).collect::<Vec<_>>()
        );
        assert_eq!(
            open_steps[0].1.state.terrain[effect_index],
            RTV_EFFECT_SENTINEL_TILE
        );

        let open_final = playback
            .frames
            .iter()
            .find(|frame| {
                frame.command_index == 1
                    && matches!(
                        frame.kind,
                        ReturnToViewFrameKind::CellEffectFinalTick { tick: 0 }
                    )
            })
            .expect("open effect final tick");
        assert_eq!(
            open_final.state.terrain[effect_index],
            RTV_OPEN_EFFECT_FINAL_TILE
        );

        let close_steps = playback
            .frames
            .iter()
            .filter_map(|frame| {
                (frame.command_index == 2)
                    .then_some(frame)
                    .and_then(|frame| {
                        if let ReturnToViewFrameKind::CellEffectStep { step } = frame.kind {
                            Some((step, frame))
                        } else {
                            None
                        }
                    })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            close_steps
                .iter()
                .map(|(step, _)| *step)
                .collect::<Vec<_>>(),
            (1..=RTV_CELL_EFFECT_STEPS).rev().collect::<Vec<_>>()
        );
        assert_eq!(
            close_steps[0].1.state.terrain[effect_index],
            RTV_EFFECT_SENTINEL_TILE
        );

        let close_final = playback
            .frames
            .iter()
            .find(|frame| {
                frame.command_index == 2
                    && matches!(
                        frame.kind,
                        ReturnToViewFrameKind::CellEffectFinalTick { tick: 0 }
                    )
            })
            .expect("close effect final tick");
        assert_eq!(
            close_final.state.terrain[effect_index],
            RTV_CLOSE_EFFECT_FINAL_TILE
        );
        assert_eq!(
            playback.run.state.terrain[effect_index],
            RTV_CLOSE_EFFECT_FINAL_TILE
        );
    }

    #[test]
    fn return_to_view_playback_expands_timed_commands_into_title_tick_frames() {
        let strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 3,
                    x: 1,
                    y: 1,
                },
                ReturnToViewCommand::TemporaryActorDraw { slot: 0 },
                ReturnToViewCommand::FixedWipeAndActorDraw {
                    reserved0: 0,
                    reserved1: 0,
                    slot: 0,
                },
                ReturnToViewCommand::OpenCellEffect { x: 2, y: 2 },
                ReturnToViewCommand::MoveActorAndTick {
                    slot: 0,
                    direction: 1,
                },
                ReturnToViewCommand::RestartStream,
            ],
        };

        let playback = run_return_to_view_playback_until_restart(&strips, &script, 64).unwrap();

        assert!(playback.run.report.restart_seen);
        // 2 ordinary ticks + 31 convergence checkpoints + 8 for the wipe
        // (five steps, three tail ticks) + 17 for the cell effect + 7 for
        // `0x0D` = 65.
        assert_eq!(playback.run.report.total_ticks, 65);
        assert_eq!(
            playback
                .frames
                .iter()
                .filter(|frame| frame.kind == ReturnToViewFrameKind::PreviewTick)
                .count(),
            2
        );
        assert_eq!(
            playback
                .frames
                .iter()
                .filter(|frame| matches!(
                    frame.kind,
                    ReturnToViewFrameKind::FixedWipeRectangle { .. }
                ))
                .count(),
            RTV_FIXED_WIPE_STEPS as usize
        );
        assert_eq!(
            playback
                .frames
                .iter()
                .filter(|frame| frame.kind == ReturnToViewFrameKind::FixedWipeActorDraw)
                .count(),
            1
        );
        // `#54` retraction 2: there is no fixed wait phase any more.
        assert_eq!(
            playback
                .frames
                .iter()
                .filter(|frame| matches!(frame.kind, ReturnToViewFrameKind::FixedWait { .. }))
                .count(),
            0
        );
        assert_eq!(
            playback
                .frames
                .iter()
                .filter(|frame| matches!(frame.kind, ReturnToViewFrameKind::CellEffectStep { .. }))
                .count(),
            RTV_CELL_EFFECT_STEPS as usize
        );
        assert_eq!(
            playback
                .frames
                .last()
                .map(|frame| frame.elapsed_title_ticks),
            Some(65)
        );
        assert_eq!(playback.run.state.actors[0].x, 2);

        let temporary = playback
            .frames
            .iter()
            .find(|frame| {
                matches!(
                    frame.kind,
                    ReturnToViewFrameKind::TemporaryActorDraw {
                        completed_writes: 8
                    }
                )
            })
            .expect("temporary actor draw frame");
        assert_eq!(temporary.actor_draw.as_ref().unwrap().tile, 3);
        assert_eq!(
            temporary.actor_draw.as_ref().unwrap().source,
            ReturnToViewActorDrawSource::OverlayTile
        );
        assert_eq!(temporary.actor_draw.as_ref().unwrap().screen_y, 8);
        assert_eq!(
            temporary.actor_draw.as_ref().unwrap().control,
            ReturnToViewActorDrawControl::OriginalActorTile(3)
        );
        assert_eq!(temporary.state.actors[0].tile0, RTV_TEMPORARY_ACTOR_TILE);
        assert_eq!(temporary.state.terrain[1 + RTV_STRIP_VISIBLE_COLUMNS], 0);
        assert_eq!(
            temporary.state.overlay[1 + RTV_STRIP_VISIBLE_COLUMNS],
            RTV_TEMPORARY_ACTOR_TILE
        );
        let convergence_frames = playback
            .frames
            .iter()
            .filter_map(|frame| match frame.kind {
                ReturnToViewFrameKind::TemporaryActorDraw { completed_writes } => {
                    Some((completed_writes, frame.elapsed_title_ticks))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(convergence_frames.len(), 32);
        assert_eq!(convergence_frames[0], (8, 3));
        assert_eq!(convergence_frames[30], (248, 33));
        assert_eq!(convergence_frames[31], (256, 33));

        let fixed_actor = playback
            .frames
            .iter()
            .find(|frame| frame.kind == ReturnToViewFrameKind::FixedWipeActorDraw)
            .expect("fixed wipe actor draw frame");
        assert_eq!(fixed_actor.actor_draw.as_ref().unwrap().tile, 3);
        assert_eq!(fixed_actor.actor_draw.as_ref().unwrap().screen_y, 8);
        assert_eq!(
            fixed_actor.actor_draw.as_ref().unwrap().control,
            ReturnToViewActorDrawControl::Zero
        );
    }

    #[test]
    fn return_to_view_actor_draw_over_backing_reports_backing_control_tile() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][0] = 0x55;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 0x44,
                    x: 0,
                    y: 0,
                },
                ReturnToViewCommand::TemporaryActorDrawOverBacking { slot: 0 },
                ReturnToViewCommand::RestartStream,
            ],
        };

        let playback = run_return_to_view_playback_until_restart(&strips, &script, 16).unwrap();
        let frame = playback
            .frames
            .iter()
            .find(|frame| {
                matches!(
                    frame.kind,
                    ReturnToViewFrameKind::TemporaryActorDrawOverBacking {
                        completed_writes: 256
                    }
                )
            })
            .expect("temporary actor backing frame");

        assert_eq!(
            frame.actor_draw.as_ref().unwrap().control,
            ReturnToViewActorDrawControl::BackingMapTile(0x55)
        );
        assert_eq!(
            frame.actor_draw.as_ref().unwrap().screen_y,
            return_to_view_actor_screen_y(0)
        );
        assert_eq!(playback.run.state.actors[0].tile0, 0x44);
        assert_eq!(playback.run.report.total_ticks, 31);
    }

    /// A full 512-entry atlas where tile `n` is painted solid with the
    /// low nibble of `n`, so a rendered pixel names the tile it came
    /// from. `#54` needs the sprite half (`256..511`) as well as the map
    /// half, because an overlay byte selects tile `256 + byte`.
    fn rtv_test_atlas() -> TileAtlas {
        let mut pixels = Vec::with_capacity(crate::TILE_ATLAS_PIXEL_LEN);
        for tile in 0..crate::TILE_ATLAS_TILE_COUNT {
            pixels.extend(std::iter::repeat_n(
                (tile % 16) as u8,
                TILE_ATLAS_SIDE * TILE_ATLAS_SIDE,
            ));
        }
        TileAtlas {
            depth: crate::TileGraphicsDepth::Ega16,
            pixels,
            dungeon_billboards: None,
            dungeon_sprites: None,
        }
    }

    /// A strip whose terrain plane is entirely non-zero, matching the
    /// shipped records (all four have zero empty cells).
    fn rtv_filled_strips() -> ReturnToViewMapStrips {
        ReturnToViewMapStrips {
            strips: [[0x44; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        }
    }

    #[test]
    fn return_to_view_reveal_cursor_widens_one_column_each_side_every_second_tick() {
        // `cleak/u5-spec#54` strip reveal: the cursor starts on column 9
        // alone and widens by one column on each side on every second
        // preview tick, so the strip is fully exposed after eighteen.
        assert_eq!(return_to_view_revealed_columns(0), None);
        assert_eq!(return_to_view_revealed_columns(1), Some((9, 9)));
        assert_eq!(return_to_view_revealed_columns(2), Some((9, 9)));
        assert_eq!(return_to_view_revealed_columns(3), Some((8, 10)));
        assert_eq!(return_to_view_revealed_columns(4), Some((8, 10)));
        assert_eq!(return_to_view_revealed_columns(5), Some((7, 11)));
        assert_eq!(return_to_view_revealed_columns(17), Some((1, 17)));
        assert_eq!(return_to_view_revealed_columns(18), Some((1, 17)));
        assert_eq!(
            return_to_view_revealed_columns(RTV_REVEAL_FULL_EXPOSURE_TICKS + 1),
            Some((0, RTV_STRIP_VISIBLE_COLUMNS - 1))
        );
        assert_eq!(
            return_to_view_revealed_columns(1_000),
            Some((0, RTV_STRIP_VISIBLE_COLUMNS - 1))
        );
    }

    #[test]
    fn return_to_view_preview_geometry_matches_the_published_rectangle() {
        // `#54`: cell `(x, y)` lands at `(8 + 16x, 16 + 16(y + 7))`, so
        // the strip covers inclusive `(8, 128)..(311, 191)` = 304 x 64.
        assert_eq!(RTV_PREVIEW_PIXEL_X, 8);
        assert_eq!(RTV_PREVIEW_PIXEL_Y, 128);
        assert_eq!(RTV_PREVIEW_PIXEL_WIDTH, 304);
        assert_eq!(RTV_PREVIEW_PIXEL_HEIGHT, 64);
        assert_eq!(RTV_PREVIEW_PIXEL_X + RTV_PREVIEW_PIXEL_WIDTH - 1, 311);
        assert_eq!(RTV_PREVIEW_PIXEL_Y + RTV_PREVIEW_PIXEL_HEIGHT - 1, 191);
        assert_eq!(return_to_view_cell_pixel_rect(0, 0), (8, 128, 16, 16));
        assert_eq!(return_to_view_cell_pixel_rect(18, 3), (296, 176, 16, 16));
        // The preview raises the viewport origin and the intro restores it.
        assert_eq!(RTV_VIEWPORT_ORIGIN_Y_NORMAL, 8);
        assert_eq!(RTV_VIEWPORT_ORIGIN_Y_PREVIEW, 16);
    }

    #[test]
    fn return_to_view_cell_source_prefers_terrain_and_skips_helper_owned_cells() {
        // `#54`: terrain wins when non-zero; otherwise the overlay byte
        // selects tile `256 + byte`; the reserved values mean another
        // helper owns the cell and the ordinary repaint skips it.
        assert_eq!(
            return_to_view_cell_source(0x44, 0),
            ReturnToViewCellSource::Terrain(0x44)
        );
        assert_eq!(
            return_to_view_cell_source(0, 0x1f),
            ReturnToViewCellSource::Overlay(0x1f)
        );
        assert_eq!(
            return_to_view_cell_source(RTV_TERRAIN_HELPER_OWNED, 0),
            ReturnToViewCellSource::HelperOwned
        );
        assert_eq!(
            return_to_view_cell_source(0x44, RTV_OVERLAY_HELPER_OWNED),
            ReturnToViewCellSource::HelperOwned
        );
        assert_eq!(return_to_view_cell_tile_index(0x44, 0, 0), Some(0x44));
        assert_eq!(return_to_view_cell_tile_index(0, 0x1f, 0), Some(256 + 0x1f));
        assert_eq!(
            return_to_view_cell_tile_index(RTV_TERRAIN_HELPER_OWNED, 0, 0),
            None
        );
    }

    #[test]
    fn render_return_to_view_preview_resolves_map_cells_at_elapsed_title_tick() {
        // Only the revealed span paints, so the animated cell sits on
        // the reveal cursor's opening column.
        let mut strips = rtv_filled_strips();
        // `animation.md §6` (spec HEAD `c00bf63`): water is not an
        // animated family, so the cell that proves the elapsed title
        // tick reaches the renderer must be a real family member.
        // `0xD4` heads the ungated waterfall family, and the synthetic
        // atlas paints tile `n` as pixel `n % 16`.
        strips.strips[0][RTV_REVEAL_CENTRE_COLUMN] = 0xD4;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let atlas = rtv_test_atlas();

        let (viewport, report) =
            render_return_to_view_preview_viewport_at_title_tick(&strips, &script, &atlas, 1)
                .unwrap();

        assert_eq!(report.total_ticks, 2);
        // Phase 1 + 2 = 3, and the waterfall family advances every tick,
        // so the cell shows `0xD7` -> pixel `0xD7 % 16` = 7.
        let x = RTV_REVEAL_CENTRE_COLUMN * TILE_ATLAS_SIDE;
        assert_eq!(viewport.pixel(x, 0), Some(0xD7 % 16));
        // Columns outside the cursor span are untouched.
        assert_eq!(viewport.pixel(0, 0), Some(0));
    }

    #[test]
    fn render_return_to_view_playback_frame_uses_frame_title_tick() {
        let mut strips = rtv_filled_strips();
        // See above: `0xD4` is the waterfall family's first id, and the
        // synthetic atlas paints tile `n` as pixel `n % 16`.
        strips.strips[0][RTV_REVEAL_CENTRE_COLUMN] = 0xD4;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let atlas = rtv_test_atlas();

        let playback = run_return_to_view_playback_until_restart(&strips, &script, 32).unwrap();
        let preview_frames = playback
            .frames
            .iter()
            .filter(|frame| frame.kind == ReturnToViewFrameKind::PreviewTick)
            .collect::<Vec<_>>();
        let first =
            render_return_to_view_playback_frame_viewport(preview_frames[0], &atlas, 0).unwrap();
        let second =
            render_return_to_view_playback_frame_viewport(preview_frames[1], &atlas, 0).unwrap();

        let x = RTV_REVEAL_CENTRE_COLUMN * TILE_ATLAS_SIDE;
        assert_eq!(preview_frames[0].elapsed_title_ticks, 1);
        assert_eq!(first.pixel(x, 0), Some(0xD5 % 16));
        assert_eq!(preview_frames[1].elapsed_title_ticks, 2);
        assert_eq!(second.pixel(x, 0), Some(0xD6 % 16));
    }

    #[test]
    fn render_return_to_view_preview_viewport_blits_visible_strip_and_actor() {
        let mut strips = rtv_filled_strips();
        strips.strips[0][RTV_REVEAL_CENTRE_COLUMN] = 0x21;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 3,
                    x: RTV_REVEAL_CENTRE_COLUMN as u8,
                    y: 1,
                },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let atlas = rtv_test_atlas();

        let (viewport, report) =
            render_return_to_view_preview_viewport(&strips, &script, &atlas).unwrap();

        assert_eq!(viewport.cells_wide, RTV_STRIP_VISIBLE_COLUMNS);
        assert_eq!(viewport.cells_high, RTV_STRIP_VISIBLE_ROWS);
        assert_eq!(viewport.width, RTV_PREVIEW_PIXEL_WIDTH);
        assert_eq!(viewport.height, RTV_PREVIEW_PIXEL_HEIGHT);
        let x = RTV_REVEAL_CENTRE_COLUMN * TILE_ATLAS_SIDE;
        assert_eq!(viewport.pixel(x, 0), Some(0x21 % 16));
        // `#54`: the actor sits on its own plane row — the `+ 7` screen
        // offset is applied when the strip is blitted, not here — and
        // its sprite comes from the `256 + byte` half of the atlas.
        assert_eq!(
            viewport.pixel(x, TILE_ATLAS_SIDE),
            Some(((RTV_OVERLAY_TILE_BASE + 3) % 16) as u8)
        );
        assert_eq!(report.drawable_actor_count, 1);
        assert!(report.restart_seen);
    }

    #[test]
    fn return_to_view_actor_placement_outside_the_strip_is_rejected() {
        // `#54`: script coordinates never leave `x = 0..18` / `y = 0..3`,
        // so an out-of-strip actor is a data fault, not something to
        // clip silently.
        let strips = rtv_filled_strips();
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 1,
                    x: RTV_STRIP_VISIBLE_COLUMNS as u8,
                    y: 0,
                },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let atlas = rtv_test_atlas();

        let err = render_return_to_view_preview_viewport(&strips, &script, &atlas)
            .expect_err("off-strip actor placement must fail instead of clipping");

        assert!(
            err.to_string().contains("outside the 19x4 preview strip"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn render_return_to_view_preview_actor_zero_pixels_leave_map_visible() {
        let mut strips = rtv_filled_strips();
        strips.strips[0][RTV_REVEAL_CENTRE_COLUMN] = 0x05;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 3,
                    x: RTV_REVEAL_CENTRE_COLUMN as u8,
                    y: 0,
                },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::RestartStream,
            ],
        };
        // Sprite `256 + 3` is transparent except for its first pixel, so
        // the map tile underneath must survive everywhere else.
        let mut pixels = Vec::with_capacity(crate::TILE_ATLAS_PIXEL_LEN);
        for tile in 0..crate::TILE_ATLAS_TILE_COUNT {
            if tile == RTV_OVERLAY_TILE_BASE + 3 {
                let mut sprite =
                    vec![RTV_ACTOR_TRANSPARENT_PIXEL; TILE_ATLAS_SIDE * TILE_ATLAS_SIDE];
                sprite[0] = 7;
                pixels.extend(sprite);
            } else {
                pixels.extend(std::iter::repeat_n(5u8, TILE_ATLAS_SIDE * TILE_ATLAS_SIDE));
            }
        }
        let atlas = TileAtlas {
            depth: crate::TileGraphicsDepth::Ega16,
            pixels,
            dungeon_billboards: None,
            dungeon_sprites: None,
        };

        let (viewport, _report) =
            render_return_to_view_preview_viewport(&strips, &script, &atlas).unwrap();

        let x = RTV_REVEAL_CENTRE_COLUMN * TILE_ATLAS_SIDE;
        assert_eq!(viewport.pixel(x, 0), Some(7));
        assert_eq!(viewport.pixel(x + 1, 0), Some(5));
    }

    #[test]
    fn single_cell_write_order_is_the_exact_corner_plus_b8_permutation() {
        let coordinates = return_to_view_single_cell_write_coordinates();
        assert_eq!(coordinates[0], (0, 0));
        assert_eq!(
            &coordinates[1..=8],
            &[
                (0, 1),
                (11, 8),
                (5, 12),
                (2, 14),
                (1, 7),
                (11, 3),
                (14, 1),
                (12, 8)
            ]
        );
        assert_eq!(
            &coordinates[248..],
            &[
                (7, 1),
                (8, 0),
                (4, 0),
                (2, 0),
                (1, 0),
                (0, 8),
                (0, 4),
                (0, 2)
            ]
        );
        let mut seen = [false; 256];
        for &(x, y) in &coordinates {
            assert!(x < 16 && y < 16);
            let index = usize::from(x) * 16 + usize::from(y);
            assert!(!seen[index], "coordinate ({x},{y}) visited twice");
            seen[index] = true;
        }
        assert!(seen.into_iter().all(|value| value));
    }

    #[test]
    fn cell_effect_raster_splices_portal_rows_into_base_and_writes_zero_opaquely() {
        let mut atlas = rtv_test_atlas();
        let tile_start = |tile: usize| tile * TILE_ATLAS_SIDE * TILE_ATLAS_SIDE;
        for y in 0..TILE_ATLAS_SIDE {
            let base = tile_start(usize::from(RTV_CLOSE_EFFECT_FINAL_TILE)) + y * TILE_ATLAS_SIDE;
            atlas.pixels[base..base + TILE_ATLAS_SIDE].fill(y as u8);
            let portal = tile_start(usize::from(RTV_OPEN_EFFECT_FINAL_TILE)) + y * TILE_ATLAS_SIDE;
            atlas.pixels[portal..portal + TILE_ATLAS_SIDE].fill((15 - y) as u8);
        }
        // Portal row zero's first pixel proves index zero overwrites rather
        // than acting as transparency.
        atlas.pixels[tile_start(usize::from(RTV_OPEN_EFFECT_FINAL_TILE))] = 0;
        let mut viewport = TileViewport {
            depth: atlas.depth,
            cells_wide: RTV_STRIP_VISIBLE_COLUMNS,
            cells_high: RTV_STRIP_VISIBLE_ROWS,
            width: RTV_PREVIEW_PIXEL_WIDTH,
            height: RTV_PREVIEW_PIXEL_HEIGHT,
            pixels: vec![9; RTV_PREVIEW_PIXEL_WIDTH * RTV_PREVIEW_PIXEL_HEIGHT],
        };

        blit_return_to_view_cell_effect_raster(&mut viewport, &atlas, 2, 1, 1).unwrap();
        let origin_x = 2 * TILE_ATLAS_SIDE;
        let origin_y = TILE_ATLAS_SIDE;
        assert_eq!(viewport.pixel(origin_x, origin_y + 14), Some(14));
        assert_eq!(viewport.pixel(origin_x, origin_y + 15), Some(0));
        assert_eq!(viewport.pixel(origin_x + 1, origin_y + 15), Some(15));

        blit_return_to_view_cell_effect_raster(&mut viewport, &atlas, 2, 1, 15).unwrap();
        assert_eq!(viewport.pixel(origin_x, origin_y), Some(0));
        assert_eq!(viewport.pixel(origin_x, origin_y + 1), Some(0));
        assert_eq!(viewport.pixel(origin_x + 1, origin_y + 1), Some(15));
        assert_eq!(viewport.pixel(origin_x + 1, origin_y + 15), Some(1));
    }

    #[test]
    fn temporary_actor_convergence_renders_each_exact_opaque_prefix() {
        let mut strips = rtv_filled_strips();
        strips.strips[0][RTV_REVEAL_CENTRE_COLUMN] = 0x05;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 3,
                    x: RTV_REVEAL_CENTRE_COLUMN as u8,
                    y: 0,
                },
                ReturnToViewCommand::TemporaryActorDraw { slot: 0 },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let mut atlas = rtv_test_atlas();
        let source_tile = RTV_OVERLAY_TILE_BASE + 3;
        let source_start = source_tile * TILE_ATLAS_SIDE * TILE_ATLAS_SIDE;
        for index in 0..TILE_ATLAS_SIDE * TILE_ATLAS_SIDE {
            atlas.pixels[source_start + index] = (index % 16) as u8;
        }
        let playback = run_return_to_view_playback_until_restart(&strips, &script, 16).unwrap();
        let frames = playback
            .frames
            .iter()
            .filter(|frame| matches!(frame.kind, ReturnToViewFrameKind::TemporaryActorDraw { .. }))
            .collect::<Vec<_>>();
        assert_eq!(frames.len(), 32);
        let previous = TileViewport {
            depth: atlas.depth,
            cells_wide: RTV_STRIP_VISIBLE_COLUMNS,
            cells_high: RTV_STRIP_VISIBLE_ROWS,
            width: RTV_PREVIEW_PIXEL_WIDTH,
            height: RTV_PREVIEW_PIXEL_HEIGHT,
            pixels: vec![9; RTV_PREVIEW_PIXEL_WIDTH * RTV_PREVIEW_PIXEL_HEIGHT],
        };
        let first =
            render_return_to_view_playback_frame_over(frames[0], &atlas, 0, Some(&previous))
                .unwrap();
        let cell_x = RTV_REVEAL_CENTRE_COLUMN * TILE_ATLAS_SIDE;
        let order = return_to_view_single_cell_write_coordinates();
        for &(x, y) in &order[..8] {
            let expected = atlas.pixels[source_start + usize::from(y) * 16 + usize::from(x)];
            assert_eq!(
                first.pixel(cell_x + usize::from(x), usize::from(y)),
                Some(expected)
            );
        }
        let (untouched_x, untouched_y) = order[8];
        assert_eq!(
            first.pixel(cell_x + usize::from(untouched_x), usize::from(untouched_y)),
            Some(9)
        );
        // Corner source index is zero, proving convergence writes zero
        // opaquely over the prior value 9.
        assert_eq!(first.pixel(cell_x, 0), Some(0));

        let final_frame =
            render_return_to_view_playback_frame_over(frames[31], &atlas, 0, Some(&previous))
                .unwrap();
        for y in 0..TILE_ATLAS_SIDE {
            for x in 0..TILE_ATLAS_SIDE {
                assert_eq!(
                    final_frame.pixel(cell_x + x, y),
                    Some(atlas.pixels[source_start + y * TILE_ATLAS_SIDE + x])
                );
            }
        }
    }
}

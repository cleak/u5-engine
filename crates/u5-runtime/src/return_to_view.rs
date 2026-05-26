//! Return-to-View preview script parser.
//!
//! `formats/location-dat.md` section 11 defines the final 655 bytes of
//! `MISCMAPS.DAT` as a compact intro-local bytecode. This module validates the
//! stream shape and exposes command summaries for frontends that do not yet
//! render the full cinematic.

use std::io;
use std::path::Path;

use crate::{
    MISCMAPS_DAT_FILE, MISCMAPS_RTV_COMMAND_SECTION_OFFSET, MISCMAPS_RTV_STRIP_ROW_STRIDE,
    MISCMAPS_RTV_STRIP_SECTION_BYTES, MISCMAPS_RTV_STRIP_SECTION_OFFSET, MOONGATE_ANIMATION_FRAMES,
    MOONGATE_TILE_BASE, RTV_COMMAND_COUNT, RTV_COMMAND_STREAM_BYTES, RTV_STRIP_COUNT,
    TILE_ATLAS_SIDE, TileAtlas, TileViewport, blit_tile_to_viewport, read_optional_disk_file,
};

pub const RTV_PREVIEW_SIDE: usize = 32;
pub const RTV_PREVIEW_CELLS: usize = RTV_PREVIEW_SIDE * RTV_PREVIEW_SIDE;
pub const RTV_ACTOR_SLOTS: usize = 32;
pub const RTV_STRIP_SOURCE_COLUMNS: usize = 4;
pub const RTV_STRIP_SOURCE_ROWS: usize = 19;
pub const RTV_STRIP_VISIBLE_COLUMNS: usize = 4;
pub const RTV_STRIP_VISIBLE_ROWS: usize = 19;
pub const RTV_STRIP_RECORD_BYTES: usize = MISCMAPS_RTV_STRIP_ROW_STRIDE * RTV_STRIP_SOURCE_COLUMNS;
pub const RTV_STRIP_TILE_COUNT: usize = RTV_STRIP_VISIBLE_COLUMNS * RTV_STRIP_VISIBLE_ROWS;
pub const RTV_EFFECT_SENTINEL_TILE: u8 = 0xfe;
pub const RTV_OPEN_EFFECT_FINAL_TILE: u8 = 0xdc;
pub const RTV_CLOSE_EFFECT_FINAL_TILE: u8 = 0x05;
pub const RTV_TEMPORARY_ACTOR_TILE: u8 = 0x16;
pub const RTV_ACTOR_TRANSPARENT_PIXEL: u8 = 0;
pub const RTV_CELL_EFFECT_STEPS: u8 = 15;
pub const RTV_CELL_EFFECT_FINAL_TICKS: u8 = 2;
pub const RTV_FIXED_WIPE_STEPS: u8 = 5;
pub const RTV_FIXED_WIPE_TRAILING_TICKS: u8 = 3;
pub const RTV_STRIP_CAPTIONS: [&str; RTV_STRIP_COUNT] = [
    "The Summoning",
    "The Journey",
    "The Arrival",
    "The Welcoming",
];

/// `cleak/u5-spec#54` published Return-to-View wait duration. Per
/// the spec answer, the helper's `WAIT` beat inserts an eight-title-tick
/// fixed pause (~1.1 seconds at 30 fps) during which animated tiles
/// continue cycling at the global title-tick cadence. The
/// [`ReturnToViewCommand::RunPreviewTick`] opcode (`0x03`) carries
/// the per-call tick count in the shipped byte stream; verify that
/// `MISCMAPS.DAT` uses this exact argument when emitting a `WAIT`.
pub const RTV_WAIT_FIXED_TICKS: u8 = 8;
pub const RTV_FIXED_WIPE_TOTAL_TICKS: u8 =
    RTV_FIXED_WIPE_STEPS + RTV_WAIT_FIXED_TICKS + RTV_FIXED_WIPE_TRAILING_TICKS;

/// `systems/intro.md section 12`: the Return-to-View preview exits on any
/// keypress observed by a wait/tick path, then restores the preserved
/// title/menu surface.
pub const RTV_WAIT_EXITS_ON_KEYPRESS: bool = true;

/// `formats/location-dat.md` Return-to-View commands draw special actors in
/// the visible text/map screen seven tile rows below the script-local actor Y.
pub const fn return_to_view_actor_screen_y(actor_y: u8) -> u8 {
    assert!(
        actor_y <= u8::MAX - 7,
        "Return-to-View actor screen Y overflows"
    );
    actor_y + 7
}

/// Resolve a Return-to-View map-cell tile through the title-screen
/// animation selector published in `cleak/u5-spec#54`.
///
/// This is intentionally narrower than the gameplay static-tile
/// animator. Return-to-View uses its own title-loop cadence and includes
/// special cinematic/effect tiles that are not ordinary overworld water
/// animation classes.
pub const fn return_to_view_tile_for_title_tick(tile: u8, title_tick: u32) -> u8 {
    let phase4 = (title_tick % 4) as u8;
    match tile {
        0x80..=0x87 => (tile & 0xfc) + phase4,
        0xD8..=0xDB => 0xD8 + phase4,
        0xDC if MOONGATE_ANIMATION_FRAMES > 1 => {
            MOONGATE_TILE_BASE + ((title_tick % MOONGATE_ANIMATION_FRAMES as u32) as u8)
        }
        _ => tile,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnToViewPreviewState {
    pub visible: [u8; RTV_PREVIEW_CELLS],
    pub backing: [u8; RTV_PREVIEW_CELLS],
    pub actors: [ReturnToViewActor; RTV_ACTOR_SLOTS],
    pub current_strip: Option<u8>,
    pub current_caption: Option<&'static str>,
    pub loop_count: u8,
    pub loop_start_command: Option<usize>,
    pub cached_effect_cell: Option<(u8, u8)>,
    pub total_ticks: u32,
    pub temporary_actor_draws: u32,
    pub fixed_wipes: u32,
    pub cell_effect_steps: u32,
    pub fixed_wipe_rectangle_steps: u32,
    pub fixed_wait_ticks: u32,
}

impl Default for ReturnToViewPreviewState {
    fn default() -> Self {
        Self {
            visible: [0; RTV_PREVIEW_CELLS],
            backing: [0; RTV_PREVIEW_CELLS],
            actors: [ReturnToViewActor::default(); RTV_ACTOR_SLOTS],
            current_strip: None,
            current_caption: None,
            loop_count: 0,
            loop_start_command: None,
            cached_effect_cell: None,
            total_ticks: 0,
            temporary_actor_draws: 0,
            fixed_wipes: 0,
            cell_effect_steps: 0,
            fixed_wipe_rectangle_steps: 0,
            fixed_wait_ticks: 0,
        }
    }
}

impl ReturnToViewPreviewState {
    pub fn drawable_actor_count(&self) -> usize {
        self.actors.iter().filter(|actor| actor.drawable).count()
    }

    pub fn cell(&self, x: u8, y: u8) -> Option<u8> {
        preview_cell_index(x, y).map(|index| self.visible[index])
    }

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
                add_return_to_view_counter(
                    &mut self.total_ticks,
                    u32::from(ticks),
                    "Return-to-View total tick counter overflowed",
                );
            }
            ReturnToViewCommand::OpenCellEffect { x, y } => {
                let screen_y = y.checked_add(7).ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Return-to-View open-cell effect Y overflows screen row offset",
                    )
                })?;
                self.open_cell_effect(x, screen_y)?;
            }
            ReturnToViewCommand::CloseCellEffect => {
                let Some((x, y)) = self.cached_effect_cell else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Return-to-View close-cell effect has no cached open-cell coordinate",
                    ));
                };
                let index = preview_cell_index_checked(x, y)?;
                self.visible[index] = RTV_EFFECT_SENTINEL_TILE;
                self.backing[index] = RTV_EFFECT_SENTINEL_TILE;
                self.visible[index] = RTV_CLOSE_EFFECT_FINAL_TILE;
                self.backing[index] = RTV_CLOSE_EFFECT_FINAL_TILE;
                add_return_to_view_counter(
                    &mut self.cell_effect_steps,
                    u32::from(RTV_CELL_EFFECT_STEPS),
                    "Return-to-View cell-effect step counter overflowed",
                );
                add_return_to_view_counter(
                    &mut self.total_ticks,
                    u32::from(RTV_CELL_EFFECT_STEPS + RTV_CELL_EFFECT_FINAL_TICKS),
                    "Return-to-View total tick counter overflowed",
                );
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
            }
            ReturnToViewCommand::RestartStream => {
                return Ok(ReturnToViewControl::Restart);
            }
            ReturnToViewCommand::SetMapCell { tile, x, y } => {
                let index = preview_cell_index_checked(x, y)?;
                self.visible[index] = tile;
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
                add_return_to_view_counter(
                    &mut self.fixed_wait_ticks,
                    u32::from(RTV_WAIT_FIXED_TICKS),
                    "Return-to-View fixed wait tick counter overflowed",
                );
                add_return_to_view_counter(
                    &mut self.total_ticks,
                    u32::from(RTV_FIXED_WIPE_TOTAL_TICKS),
                    "Return-to-View total tick counter overflowed",
                );
            }
            ReturnToViewCommand::ClearActors => {
                self.actors = [ReturnToViewActor::default(); RTV_ACTOR_SLOTS];
            }
            ReturnToViewCommand::MoveActorAndTick { slot, direction } => {
                self.move_actor(slot, direction)?;
                add_return_to_view_counter(
                    &mut self.total_ticks,
                    1,
                    "Return-to-View total tick counter overflowed",
                );
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
        self.visible = [0; RTV_PREVIEW_CELLS];
        self.backing = [0; RTV_PREVIEW_CELLS];
        for row in 0..RTV_STRIP_VISIBLE_ROWS {
            for col in 0..RTV_STRIP_VISIBLE_COLUMNS {
                let tile = source[row * RTV_STRIP_VISIBLE_COLUMNS + col];
                let index = row * RTV_PREVIEW_SIDE + col;
                self.visible[index] = tile;
                self.backing[index] = tile;
            }
        }
        self.current_strip = Some(strip);
        self.current_caption = return_to_view_caption_for_strip(strip);
        Ok(())
    }

    fn restore_actor_backing(&mut self, slot: usize) -> io::Result<()> {
        let actor = self.actors[slot];
        let index = preview_cell_index_checked(actor.x, actor.y)?;
        self.visible[index] = self.backing[index];
        Ok(())
    }

    fn move_actor(&mut self, slot: u8, direction: u8) -> io::Result<()> {
        let slot = rtv_slot_index(slot)?;
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
        self.visible[index] = RTV_EFFECT_SENTINEL_TILE;
        self.backing[index] = RTV_EFFECT_SENTINEL_TILE;
        self.visible[index] = RTV_OPEN_EFFECT_FINAL_TILE;
        self.backing[index] = RTV_OPEN_EFFECT_FINAL_TILE;
        add_return_to_view_counter(
            &mut self.cell_effect_steps,
            u32::from(RTV_CELL_EFFECT_STEPS),
            "Return-to-View cell-effect step counter overflowed",
        );
        add_return_to_view_counter(
            &mut self.total_ticks,
            u32::from(RTV_CELL_EFFECT_STEPS + RTV_CELL_EFFECT_FINAL_TICKS),
            "Return-to-View total tick counter overflowed",
        );
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
    pub fixed_wait_ticks: u32,
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
    TemporaryActorDraw,
    TemporaryActorDrawOverBacking,
    FixedWipeRectangle { step: u8 },
    FixedWipeActorDraw,
    FixedWait { tick: u8 },
    FixedWipeTrailingTick { tick: u8 },
    MoveActorTick,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnToViewActorDrawSource {
    TemporaryActorTile,
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
    let mut strips = [[0u8; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT];
    for (strip_index, strip) in strips.iter_mut().enumerate() {
        let base = strip_index * RTV_STRIP_RECORD_BYTES;
        for source_col in 0..RTV_STRIP_SOURCE_COLUMNS {
            let column_start = base + source_col * MISCMAPS_RTV_STRIP_ROW_STRIDE;
            let source_column = &bytes[column_start..column_start + RTV_STRIP_SOURCE_ROWS];
            for (source_row, tile) in source_column.iter().copied().enumerate() {
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
        fixed_wait_ticks: state.fixed_wait_ticks,
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
        append_return_to_view_playback_frames(
            &mut frames,
            command_index,
            command,
            before_ticks,
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
        fixed_wait_ticks: state.fixed_wait_ticks,
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
                );
            }
        }
        ReturnToViewCommand::LoadMapStrip { .. } => {}
        ReturnToViewCommand::OpenCellEffect { x, y } => {
            let screen_y = y
                .checked_add(7)
                .expect("Return-to-View open-cell playback Y overflows screen row offset");
            for step in 0..RTV_CELL_EFFECT_STEPS {
                push_return_to_view_cell_effect_frame(
                    frames,
                    command_index,
                    before_ticks + u32::from(step) + 1,
                    ReturnToViewFrameKind::CellEffectStep { step },
                    before_state,
                    x,
                    screen_y,
                    RTV_EFFECT_SENTINEL_TILE,
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
                );
            }
        }
        ReturnToViewCommand::CloseCellEffect => {
            let (x, y) = before_state
                .cached_effect_cell
                .expect("Return-to-View close-cell playback has no cached open-cell coordinate");
            for step in 0..RTV_CELL_EFFECT_STEPS {
                push_return_to_view_cell_effect_frame(
                    frames,
                    command_index,
                    before_ticks + u32::from(step) + 1,
                    ReturnToViewFrameKind::CellEffectStep { step },
                    before_state,
                    x,
                    y,
                    RTV_EFFECT_SENTINEL_TILE,
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
                );
            }
        }
        ReturnToViewCommand::TemporaryActorDraw { slot } => {
            push_return_to_view_actor_draw_frame(
                frames,
                command_index,
                before_ticks,
                ReturnToViewFrameKind::TemporaryActorDraw,
                state,
                slot,
                ReturnToViewActorDrawControlSource::OriginalActorTile,
            );
        }
        ReturnToViewCommand::TemporaryActorDrawOverBacking { slot } => {
            push_return_to_view_actor_draw_frame(
                frames,
                command_index,
                before_ticks,
                ReturnToViewFrameKind::TemporaryActorDrawOverBacking,
                state,
                slot,
                ReturnToViewActorDrawControlSource::BackingMapTile,
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
                );
            }
            push_return_to_view_fixed_actor_draw_frame(frames, command_index, elapsed, state, slot);
            for tick in 0..RTV_WAIT_FIXED_TICKS {
                elapsed += 1;
                push_return_to_view_frame(
                    frames,
                    command_index,
                    elapsed,
                    ReturnToViewFrameKind::FixedWait { tick },
                    state,
                );
            }
            for tick in 0..RTV_FIXED_WIPE_TRAILING_TICKS {
                elapsed += 1;
                push_return_to_view_frame(
                    frames,
                    command_index,
                    elapsed,
                    ReturnToViewFrameKind::FixedWipeTrailingTick { tick },
                    state,
                );
            }
        }
        ReturnToViewCommand::MoveActorAndTick { .. } => {
            push_return_to_view_frame(
                frames,
                command_index,
                before_ticks + 1,
                ReturnToViewFrameKind::MoveActorTick,
                state,
            );
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
) {
    let mut state = state.clone();
    let index = preview_cell_index_checked(x, y)
        .expect("Return-to-View cell-effect playback coordinate is outside preview buffer");
    state.visible[index] = tile;
    state.backing[index] = tile;
    state.total_ticks = elapsed_title_ticks;
    frames.push(ReturnToViewPlaybackFrame {
        command_index,
        elapsed_title_ticks,
        kind,
        state,
        actor_draw: None,
    });
}

fn push_return_to_view_frame(
    frames: &mut Vec<ReturnToViewPlaybackFrame>,
    command_index: usize,
    elapsed_title_ticks: u32,
    kind: ReturnToViewFrameKind,
    state: &ReturnToViewPreviewState,
) {
    let mut state = state.clone();
    state.total_ticks = elapsed_title_ticks;
    frames.push(ReturnToViewPlaybackFrame {
        command_index,
        elapsed_title_ticks,
        kind,
        state,
        actor_draw: None,
    });
}

fn push_return_to_view_actor_draw_frame(
    frames: &mut Vec<ReturnToViewPlaybackFrame>,
    command_index: usize,
    elapsed_title_ticks: u32,
    kind: ReturnToViewFrameKind,
    state: &ReturnToViewPreviewState,
    slot: u8,
    control_source: ReturnToViewActorDrawControlSource,
) {
    let mut state = state.clone();
    let slot_index = usize::from(slot);
    let actor = *state
        .actors
        .get(slot_index)
        .expect("Return-to-View actor draw slot is outside actor table");
    let original_tile = actor.tile0;
    let backing_index = preview_cell_index_checked(actor.x, actor.y)
        .expect("Return-to-View actor draw coordinate is outside preview buffer");
    let backing_tile = state.backing[backing_index];
    state.actors[slot_index].tile0 = RTV_TEMPORARY_ACTOR_TILE;
    state.actors[slot_index].tile1 = RTV_TEMPORARY_ACTOR_TILE;
    frames.push(ReturnToViewPlaybackFrame {
        command_index,
        elapsed_title_ticks,
        kind,
        state,
        actor_draw: Some(ReturnToViewActorDraw {
            slot,
            tile: RTV_TEMPORARY_ACTOR_TILE,
            x: actor.x,
            y: actor.y,
            screen_y: return_to_view_actor_screen_y(actor.y),
            source: ReturnToViewActorDrawSource::TemporaryActorTile,
            control: match control_source {
                ReturnToViewActorDrawControlSource::OriginalActorTile => {
                    ReturnToViewActorDrawControl::OriginalActorTile(original_tile)
                }
                ReturnToViewActorDrawControlSource::BackingMapTile => {
                    ReturnToViewActorDrawControl::BackingMapTile(backing_tile)
                }
            },
        }),
    });
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
        "Dry run {end} after {} applied command(s); emits {} playback frame(s); current strip {:?}, {} drawable actor(s), {} scheduled tick(s), {} cell-effect step(s), {} temporary draw(s), {} fixed wipe(s), {} fixed-wipe rectangle step(s), {} fixed-wait tick(s).",
        report.applied_commands,
        playback.frames.len(),
        report.current_strip,
        report.drawable_actor_count,
        report.total_ticks,
        report.cell_effect_steps,
        report.temporary_actor_draws,
        report.fixed_wipes,
        report.fixed_wipe_rectangle_steps,
        report.fixed_wait_ticks
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
        starting_title_tick
            .checked_add(run.report.total_ticks)
            .expect("Return-to-View render title tick overflowed"),
    )?;
    Ok((viewport, run.report))
}

pub fn render_return_to_view_playback_frame_viewport(
    frame: &ReturnToViewPlaybackFrame,
    atlas: &TileAtlas,
    starting_title_tick: u32,
) -> io::Result<TileViewport> {
    render_return_to_view_state_viewport(
        &frame.state,
        atlas,
        starting_title_tick
            .checked_add(frame.elapsed_title_ticks)
            .expect("Return-to-View playback title tick overflowed"),
    )
}

fn render_return_to_view_state_viewport(
    state: &ReturnToViewPreviewState,
    atlas: &TileAtlas,
    render_title_tick: u32,
) -> io::Result<TileViewport> {
    let width = RTV_STRIP_VISIBLE_COLUMNS
        .checked_mul(TILE_ATLAS_SIDE)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "RTV viewport width overflows")
        })?;
    let height = RTV_STRIP_VISIBLE_ROWS
        .checked_mul(TILE_ATLAS_SIDE)
        .ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "RTV viewport height overflows")
        })?;
    let pixel_count = width.checked_mul(height).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "RTV viewport pixel count overflows",
        )
    })?;
    let mut viewport = TileViewport {
        depth: atlas.depth,
        cells_wide: RTV_STRIP_VISIBLE_COLUMNS,
        cells_high: RTV_STRIP_VISIBLE_ROWS,
        width,
        height,
        pixels: vec![0; pixel_count],
    };
    for cell_y in 0..RTV_STRIP_VISIBLE_ROWS {
        for cell_x in 0..RTV_STRIP_VISIBLE_COLUMNS {
            let tile = state.visible[cell_y * RTV_PREVIEW_SIDE + cell_x];
            let tile = return_to_view_tile_for_title_tick(tile, render_title_tick);
            blit_tile_to_viewport(&mut viewport, atlas, tile, cell_x, cell_y)?;
        }
    }
    for actor in state.actors.iter().filter(|actor| actor.drawable) {
        let x = usize::from(actor.x);
        let y = usize::from(return_to_view_actor_screen_y(actor.y));
        if x < RTV_STRIP_VISIBLE_COLUMNS && y < RTV_STRIP_VISIBLE_ROWS {
            blit_return_to_view_actor_to_viewport(&mut viewport, atlas, actor.tile0, x, y)?;
        }
    }
    Ok(viewport)
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
    tile: u8,
    cell_x: usize,
    cell_y: usize,
) -> io::Result<()> {
    let tile_pixels = atlas.tile_pixels(tile as usize).ok_or_else(|| {
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
    if x >= RTV_PREVIEW_SIDE || y >= RTV_PREVIEW_SIDE {
        return None;
    }
    Some(y * RTV_PREVIEW_SIDE + x)
}

fn preview_cell_index_checked(x: u8, y: u8) -> io::Result<usize> {
    preview_cell_index(x, y).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Return-to-View coordinate ({x}, {y}) is outside the 32x32 preview buffer"),
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
    let side = RTV_PREVIEW_SIDE as i16;
    let nx = (i16::from(x) + dx).rem_euclid(side);
    let ny = (i16::from(y) + dy).rem_euclid(side);
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
        let mut bytes = vec![0xee; MISCMAPS_RTV_STRIP_SECTION_BYTES];
        for strip in 0..RTV_STRIP_COUNT {
            for source_col in 0..RTV_STRIP_SOURCE_COLUMNS {
                let column_start =
                    strip * RTV_STRIP_RECORD_BYTES + source_col * MISCMAPS_RTV_STRIP_ROW_STRIDE;
                for source_row in 0..RTV_STRIP_SOURCE_ROWS {
                    bytes[column_start + source_row] =
                        (strip * 100 + source_col * 19 + source_row) as u8;
                }
            }
        }

        let strips = parse_return_to_view_map_strips(&bytes).unwrap();

        assert_eq!(strips.strips[0][0], 0);
        assert_eq!(strips.strips[0][1], 19);
        assert_eq!(strips.strips[0][4], 1);
        assert_eq!(strips.strips[0][19], 61);
        assert_eq!(strips.strips[0][75], 75);
        assert_eq!(strips.strips[1][0], 100);
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
        assert_eq!(state.cell(2, 9), Some(RTV_CLOSE_EFFECT_FINAL_TILE));
        assert_eq!(state.total_ticks, 34);
        assert_eq!(state.cell_effect_steps, 30);
        assert_eq!(state.fixed_wait_ticks, 0);
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

        assert_eq!(RTV_FIXED_WIPE_TOTAL_TICKS, 16);
        assert_eq!(state.fixed_wipes, 1);
        assert_eq!(state.fixed_wipe_rectangle_steps, 5);
        assert_eq!(state.fixed_wait_ticks, 8);
        assert_eq!(state.total_ticks, 16);
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
    fn return_to_view_actor_movement_wraps_preview_buffer_edges() {
        let strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
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

        state
            .apply_command(
                &strips,
                1,
                ReturnToViewCommand::MoveActor {
                    slot: 0,
                    direction: 3,
                },
            )
            .unwrap();
        state
            .apply_command(
                &strips,
                2,
                ReturnToViewCommand::MoveActor {
                    slot: 0,
                    direction: 0,
                },
            )
            .unwrap();

        assert_eq!(state.actors[0].x, 31);
        assert_eq!(state.actors[0].y, 31);
    }

    #[test]
    fn return_to_view_tile_animation_uses_title_tick_families() {
        assert_eq!(return_to_view_tile_for_title_tick(0x80, 0), 0x80);
        assert_eq!(return_to_view_tile_for_title_tick(0x80, 3), 0x83);
        assert_eq!(return_to_view_tile_for_title_tick(0x84, 2), 0x86);
        assert_eq!(return_to_view_tile_for_title_tick(0xD8, 5), 0xD9);
        assert_eq!(return_to_view_tile_for_title_tick(0xDC, 9), 0xDC);
        assert_eq!(return_to_view_tile_for_title_tick(0x05, 9), 0x05);
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
        assert_eq!(playback.run.state.visible[0], 10);
        assert_eq!(playback.run.state.visible[9 * RTV_PREVIEW_SIDE], 19);
        assert_eq!(playback.run.state.visible[18 * RTV_PREVIEW_SIDE], 28);
    }

    #[test]
    fn return_to_view_cell_effect_playback_stages_sentinel_then_final_tile() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][8 * RTV_STRIP_VISIBLE_COLUMNS + 2] = 0x33;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::OpenCellEffect { x: 2, y: 1 },
                ReturnToViewCommand::CloseCellEffect,
                ReturnToViewCommand::RestartStream,
            ],
        };

        let playback = run_return_to_view_playback_until_restart(&strips, &script, 96).unwrap();
        let effect_index = preview_cell_index(2, 8).unwrap();

        let open_step = playback
            .frames
            .iter()
            .find(|frame| {
                frame.command_index == 1
                    && matches!(
                        frame.kind,
                        ReturnToViewFrameKind::CellEffectStep { step: 0 }
                    )
            })
            .expect("open effect first step");
        assert_eq!(
            open_step.state.visible[effect_index],
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
            open_final.state.visible[effect_index],
            RTV_OPEN_EFFECT_FINAL_TILE
        );

        let close_step = playback
            .frames
            .iter()
            .find(|frame| {
                frame.command_index == 2
                    && matches!(
                        frame.kind,
                        ReturnToViewFrameKind::CellEffectStep { step: 0 }
                    )
            })
            .expect("close effect first step");
        assert_eq!(
            close_step.state.visible[effect_index],
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
            close_final.state.visible[effect_index],
            RTV_CLOSE_EFFECT_FINAL_TILE
        );
        assert_eq!(
            playback.run.state.visible[effect_index],
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
        assert_eq!(playback.run.report.total_ticks, 36);
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
        assert_eq!(
            playback
                .frames
                .iter()
                .filter(|frame| matches!(frame.kind, ReturnToViewFrameKind::FixedWait { .. }))
                .count(),
            RTV_WAIT_FIXED_TICKS as usize
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
            Some(36)
        );
        assert_eq!(playback.run.state.actors[0].x, 2);

        let temporary = playback
            .frames
            .iter()
            .find(|frame| frame.kind == ReturnToViewFrameKind::TemporaryActorDraw)
            .expect("temporary actor draw frame");
        assert_eq!(
            temporary.actor_draw.as_ref().unwrap().tile,
            RTV_TEMPORARY_ACTOR_TILE
        );
        assert_eq!(temporary.actor_draw.as_ref().unwrap().screen_y, 8);
        assert_eq!(
            temporary.actor_draw.as_ref().unwrap().control,
            ReturnToViewActorDrawControl::OriginalActorTile(3)
        );
        assert_eq!(temporary.state.actors[0].tile0, RTV_TEMPORARY_ACTOR_TILE);

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
            .find(|frame| frame.kind == ReturnToViewFrameKind::TemporaryActorDrawOverBacking)
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
    }

    #[test]
    fn render_return_to_view_preview_resolves_map_cells_at_elapsed_title_tick() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][0] = 0xD8;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let mut pixels = Vec::new();
        for tile in 0..=0xDBusize {
            pixels.extend(std::iter::repeat_n(
                (tile % 16) as u8,
                TILE_ATLAS_SIDE * TILE_ATLAS_SIDE,
            ));
        }
        let atlas = TileAtlas {
            depth: crate::TileGraphicsDepth::Ega16,
            pixels,
        };

        let (viewport, report) =
            render_return_to_view_preview_viewport_at_title_tick(&strips, &script, &atlas, 1)
                .unwrap();

        assert_eq!(report.total_ticks, 2);
        assert_eq!(viewport.pixel(0, 0), Some(0xDB % 16));
    }

    #[test]
    fn render_return_to_view_playback_frame_uses_frame_title_tick() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][0] = 0xD8;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::RunPreviewTick { ticks: 2 },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let mut pixels = Vec::new();
        for tile in 0..=0xDBusize {
            pixels.extend(std::iter::repeat_n(
                (tile % 16) as u8,
                TILE_ATLAS_SIDE * TILE_ATLAS_SIDE,
            ));
        }
        let atlas = TileAtlas {
            depth: crate::TileGraphicsDepth::Ega16,
            pixels,
        };

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

        assert_eq!(preview_frames[0].elapsed_title_ticks, 1);
        assert_eq!(first.pixel(0, 0), Some(0xD9 % 16));
        assert_eq!(preview_frames[1].elapsed_title_ticks, 2);
        assert_eq!(second.pixel(0, 0), Some(0xDA % 16));
    }

    #[test]
    fn render_return_to_view_preview_viewport_blits_visible_strip_and_actor() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][0] = 1;
        strips.strips[0][1] = 2;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 3,
                    x: 1,
                    y: 0,
                },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let mut pixels = Vec::new();
        for tile in 0..4 {
            pixels.extend(std::iter::repeat_n(tile, TILE_ATLAS_SIDE * TILE_ATLAS_SIDE));
        }
        let atlas = TileAtlas {
            depth: crate::TileGraphicsDepth::Ega16,
            pixels,
        };

        let (viewport, report) =
            render_return_to_view_preview_viewport(&strips, &script, &atlas).unwrap();

        assert_eq!(viewport.cells_wide, RTV_STRIP_VISIBLE_COLUMNS);
        assert_eq!(viewport.cells_high, RTV_STRIP_VISIBLE_ROWS);
        assert_eq!(viewport.pixel(0, 0), Some(1));
        assert_eq!(viewport.pixel(TILE_ATLAS_SIDE, 0), Some(2));
        assert_eq!(
            viewport.pixel(TILE_ATLAS_SIDE, 7 * TILE_ATLAS_SIDE),
            Some(3)
        );
        assert_eq!(report.drawable_actor_count, 1);
        assert!(report.restart_seen);
    }

    #[test]
    fn render_return_to_view_preview_actor_zero_pixels_leave_map_visible() {
        let mut strips = ReturnToViewMapStrips {
            strips: [[0; RTV_STRIP_TILE_COUNT]; RTV_STRIP_COUNT],
        };
        strips.strips[0][0] = 1;
        strips.strips[0][7 * RTV_STRIP_VISIBLE_COLUMNS] = 1;
        let script = ReturnToViewScript {
            commands: vec![
                ReturnToViewCommand::LoadMapStrip { strip: 0 },
                ReturnToViewCommand::SetActor {
                    slot: 0,
                    tile: 3,
                    x: 0,
                    y: 0,
                },
                ReturnToViewCommand::RestartStream,
            ],
        };
        let mut pixels = Vec::new();
        pixels.extend(std::iter::repeat_n(0, TILE_ATLAS_SIDE * TILE_ATLAS_SIDE));
        pixels.extend(std::iter::repeat_n(5, TILE_ATLAS_SIDE * TILE_ATLAS_SIDE));
        pixels.extend(std::iter::repeat_n(2, TILE_ATLAS_SIDE * TILE_ATLAS_SIDE));
        let mut actor = vec![RTV_ACTOR_TRANSPARENT_PIXEL; TILE_ATLAS_SIDE * TILE_ATLAS_SIDE];
        actor[0] = 7;
        pixels.extend(actor);
        let atlas = TileAtlas {
            depth: crate::TileGraphicsDepth::Ega16,
            pixels,
        };

        let (viewport, _report) =
            render_return_to_view_preview_viewport(&strips, &script, &atlas).unwrap();

        assert_eq!(viewport.pixel(0, 0), Some(5));
        assert_eq!(viewport.pixel(0, 7 * TILE_ATLAS_SIDE), Some(7));
        assert_eq!(viewport.pixel(1, 7 * TILE_ATLAS_SIDE), Some(5));
    }
}

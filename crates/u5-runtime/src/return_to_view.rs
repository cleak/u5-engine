//! Return-to-View preview script parser.
//!
//! `formats/location-dat.md` section 11 defines the final 655 bytes of
//! `MISCMAPS.DAT` as a compact intro-local bytecode. This module validates the
//! stream shape and exposes command summaries for frontends that do not yet
//! render the full cinematic.

use std::fs;
use std::io;
use std::path::Path;

use crate::{
    MISCMAPS_DAT_FILE, MISCMAPS_RTV_COMMAND_SECTION_OFFSET, MISCMAPS_RTV_STRIP_ROW_STRIDE,
    MISCMAPS_RTV_STRIP_SECTION_BYTES, MISCMAPS_RTV_STRIP_SECTION_OFFSET, RTV_COMMAND_COUNT,
    RTV_COMMAND_STREAM_BYTES, RTV_STRIP_COUNT,
};

pub const RTV_PREVIEW_SIDE: usize = 32;
pub const RTV_PREVIEW_CELLS: usize = RTV_PREVIEW_SIDE * RTV_PREVIEW_SIDE;
pub const RTV_ACTOR_SLOTS: usize = 32;
pub const RTV_STRIP_VISIBLE_COLUMNS: usize = 19;
pub const RTV_STRIP_VISIBLE_ROWS: usize = 4;
pub const RTV_STRIP_RECORD_BYTES: usize = MISCMAPS_RTV_STRIP_ROW_STRIDE * RTV_STRIP_VISIBLE_ROWS;
pub const RTV_STRIP_TILE_COUNT: usize = RTV_STRIP_VISIBLE_COLUMNS * RTV_STRIP_VISIBLE_ROWS;
pub const RTV_EFFECT_SENTINEL_TILE: u8 = 0xfe;
pub const RTV_OPEN_EFFECT_FINAL_TILE: u8 = 0xdc;
pub const RTV_CLOSE_EFFECT_FINAL_TILE: u8 = 0x05;
pub const RTV_TEMPORARY_ACTOR_TILE: u8 = 0x16;

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
        self.commands.len().saturating_sub(self.no_op_count())
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
    pub loop_count: u8,
    pub loop_start_command: Option<usize>,
    pub cached_effect_cell: Option<(u8, u8)>,
    pub total_ticks: u32,
    pub temporary_actor_draws: u32,
    pub fixed_wipes: u32,
}

impl Default for ReturnToViewPreviewState {
    fn default() -> Self {
        Self {
            visible: [0; RTV_PREVIEW_CELLS],
            backing: [0; RTV_PREVIEW_CELLS],
            actors: [ReturnToViewActor::default(); RTV_ACTOR_SLOTS],
            current_strip: None,
            loop_count: 0,
            loop_start_command: None,
            cached_effect_cell: None,
            total_ticks: 0,
            temporary_actor_draws: 0,
            fixed_wipes: 0,
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
                self.total_ticks = self.total_ticks.saturating_add(u32::from(ticks));
            }
            ReturnToViewCommand::OpenCellEffect { x, y } => {
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
                self.visible[index] = RTV_EFFECT_SENTINEL_TILE;
                self.backing[index] = RTV_EFFECT_SENTINEL_TILE;
                self.visible[index] = RTV_CLOSE_EFFECT_FINAL_TILE;
                self.backing[index] = RTV_CLOSE_EFFECT_FINAL_TILE;
                self.total_ticks = self.total_ticks.saturating_add(17);
            }
            ReturnToViewCommand::LoadMapStrip { strip } => {
                self.load_map_strip(strips, strip)?;
            }
            ReturnToViewCommand::TemporaryActorDraw { slot }
            | ReturnToViewCommand::TemporaryActorDrawOverBacking { slot } => {
                let slot = rtv_slot_index(slot)?;
                let actor = self.actors[slot];
                let _ = preview_cell_index_checked(actor.x, actor.y)?;
                self.temporary_actor_draws = self.temporary_actor_draws.saturating_add(1);
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
                self.fixed_wipes = self.fixed_wipes.saturating_add(1);
                self.total_ticks = self.total_ticks.saturating_add(8);
            }
            ReturnToViewCommand::ClearActors => {
                self.actors = [ReturnToViewActor::default(); RTV_ACTOR_SLOTS];
            }
            ReturnToViewCommand::MoveActorAndTick { slot, direction } => {
                self.move_actor(slot, direction)?;
                self.total_ticks = self.total_ticks.saturating_add(1);
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
        self.total_ticks = self.total_ticks.saturating_add(17);
        Ok(())
    }
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
    pub drawable_actor_count: usize,
    pub total_ticks: u32,
    pub temporary_actor_draws: u32,
    pub fixed_wipes: u32,
    pub cached_effect_cell: Option<(u8, u8)>,
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
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_return_to_view_script_file(&bytes).map(Some)
}

pub fn load_return_to_view_assets(game_dir: &Path) -> io::Result<Option<ReturnToViewAssets>> {
    let path = game_dir.join(MISCMAPS_DAT_FILE);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
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
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
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
        for row in 0..RTV_STRIP_VISIBLE_ROWS {
            let row_start = base + row * MISCMAPS_RTV_STRIP_ROW_STRIDE;
            let source = &bytes[row_start..row_start + RTV_STRIP_VISIBLE_COLUMNS];
            let target =
                &mut strip[row * RTV_STRIP_VISIBLE_COLUMNS..(row + 1) * RTV_STRIP_VISIBLE_COLUMNS];
            target.copy_from_slice(source);
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
    Ok(ReturnToViewPreviewReport {
        applied_commands,
        restart_seen,
        max_commands_reached: !restart_seen
            && pc < script.commands.len()
            && applied_commands >= max_commands,
        current_strip: state.current_strip,
        drawable_actor_count: state.drawable_actor_count(),
        total_ticks: state.total_ticks,
        temporary_actor_draws: state.temporary_actor_draws,
        fixed_wipes: state.fixed_wipes,
        cached_effect_cell: state.cached_effect_cell,
    })
}

pub fn summarize_return_to_view_preview(
    strips: &ReturnToViewMapStrips,
    script: &ReturnToViewScript,
) -> io::Result<String> {
    let report = run_return_to_view_preview_until_restart(strips, script, 4096)?;
    let end = if report.restart_seen {
        "reaches the stream restart"
    } else if report.max_commands_reached {
        "hit the dry-run command cap"
    } else {
        "reaches end of stream"
    };
    Ok(format!(
        "Dry run {end} after {} applied command(s); current strip {:?}, {} drawable actor(s), {} scheduled tick(s), {} temporary draw(s), {} fixed wipe(s).",
        report.applied_commands,
        report.current_strip,
        report.drawable_actor_count,
        report.total_ticks,
        report.temporary_actor_draws,
        report.fixed_wipes
    ))
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
            for row in 0..RTV_STRIP_VISIBLE_ROWS {
                let row_start =
                    strip * RTV_STRIP_RECORD_BYTES + row * MISCMAPS_RTV_STRIP_ROW_STRIDE;
                for col in 0..RTV_STRIP_VISIBLE_COLUMNS {
                    bytes[row_start + col] = (strip * 40 + row * 19 + col) as u8;
                }
            }
        }

        let strips = parse_return_to_view_map_strips(&bytes).unwrap();

        assert_eq!(strips.strips[0][0], 0);
        assert_eq!(strips.strips[0][18], 18);
        assert_eq!(strips.strips[0][19], 19);
        assert_eq!(strips.strips[1][0], 40);
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
                ReturnToViewCommand::OpenCellEffect { x: 4, y: 2 },
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
        assert_eq!(state.cell(4, 2), Some(RTV_CLOSE_EFFECT_FINAL_TILE));
        assert_eq!(state.total_ticks, 34);
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
}

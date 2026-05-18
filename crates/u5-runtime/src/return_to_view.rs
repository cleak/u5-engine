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
    MISCMAPS_DAT_FILE, MISCMAPS_RTV_COMMAND_SECTION_OFFSET, RTV_COMMAND_COUNT,
    RTV_COMMAND_STREAM_BYTES,
};

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
}

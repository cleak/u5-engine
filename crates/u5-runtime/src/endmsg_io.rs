//! Parser for `ENDMSG.DAT`: eleven NUL-terminated dialogue records consumed by
//! the endgame Lord British box-delivery sequence and the refusal/missing-box
//! branch. Spec: `formats/endmsg-dat.md` §2-§4.

use std::fs;
use std::io;
use std::path::Path;

const ENDMSG_DAT_FILE: &str = "ENDMSG.DAT";
const EXPECTED_RECORD_COUNT: usize = 11;

/// `formats/endmsg-dat.md §2`: shipped DOS file size in bytes.
pub const ENDMSG_DAT_LEN: usize = 786;
/// `formats/endmsg-dat.md §2`: number of NUL-terminated dialogue
/// records the file holds.
pub const ENDMSG_DAT_RECORDS: usize = 11;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndgameMessages {
    pub records: Vec<String>,
}

impl EndgameMessages {
    pub fn initial_greeting(&self) -> Option<&str> {
        self.records.first().map(String::as_str)
    }

    pub fn first_box_prompt(&self) -> Option<&str> {
        self.records.get(1).map(String::as_str)
    }

    pub fn second_box_prompt(&self) -> Option<&str> {
        self.records.get(2).map(String::as_str)
    }

    pub fn rite_messages(&self) -> &[String] {
        let start = 3.min(self.records.len());
        let end = self.records.len().saturating_sub(1).max(start);
        &self.records[start..end]
    }

    pub fn refusal_branch(&self) -> Option<&str> {
        self.records.last().map(String::as_str)
    }
}

pub fn load_endgame_messages(game_dir: &Path) -> io::Result<Option<EndgameMessages>> {
    let path = game_dir.join(ENDMSG_DAT_FILE);
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
    parse_endgame_messages(&bytes).map(Some)
}

pub fn require_endgame_messages(game_dir: &Path) -> io::Result<EndgameMessages> {
    load_endgame_messages(game_dir)?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "{}: required endgame dialogue resource is missing",
                game_dir.join(ENDMSG_DAT_FILE).display()
            ),
        )
    })
}

pub fn parse_endgame_messages(bytes: &[u8]) -> io::Result<EndgameMessages> {
    let mut records = Vec::with_capacity(EXPECTED_RECORD_COUNT);
    let mut start = 0;
    for record_index in 0..EXPECTED_RECORD_COUNT {
        let (record, next_start) = read_endgame_record(bytes, start, record_index)?;
        records.push(record);
        start = next_start;
    }
    validate_endgame_tail(bytes, start)?;
    Ok(EndgameMessages { records })
}

fn read_endgame_record(
    bytes: &[u8],
    start: usize,
    record_index: usize,
) -> io::Result<(String, usize)> {
    let end = bytes[start..]
        .iter()
        .position(|&b| b == 0x00)
        .map(|offset| start + offset);
    let Some(end) = end else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{ENDMSG_DAT_FILE}: record {record_index} is not NUL-terminated"),
        ));
    };
    if end == start {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{ENDMSG_DAT_FILE}: record {record_index} is empty"),
        ));
    }
    let record = decode_endgame_record(record_index, &bytes[start..end])?;
    Ok((record, end + 1))
}

fn validate_endgame_tail(bytes: &[u8], mut start: usize) -> io::Result<()> {
    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|&b| b == 0x00)
            .map(|offset| start + offset);
        let Some(end) = end else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{ENDMSG_DAT_FILE}: extra record starting at byte {start} is not NUL-terminated"
                ),
            ));
        };
        if end > start {
            decode_endgame_record(EXPECTED_RECORD_COUNT, &bytes[start..end])?;
        }
        start = end + 1;
    }
    Ok(())
}

fn decode_endgame_record(record_index: usize, bytes: &[u8]) -> io::Result<String> {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x0a | 0x0d => out.push('\n'),
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{ENDMSG_DAT_FILE}: record {record_index} has unsupported byte 0x{byte:02x}"
                    ),
                ));
            }
        }
    }
    if out.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{ENDMSG_DAT_FILE}: record {record_index} decodes to empty text"),
        ));
    }
    Ok(out)
}

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

pub fn parse_endgame_messages(bytes: &[u8]) -> io::Result<EndgameMessages> {
    let mut records = Vec::with_capacity(EXPECTED_RECORD_COUNT);
    let mut start = 0;
    while start < bytes.len() {
        let end = bytes[start..]
            .iter()
            .position(|&b| b == 0x00)
            .map(|offset| start + offset);
        let Some(end) = end else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{ENDMSG_DAT_FILE}: unterminated record starting at byte {start}"),
            ));
        };
        records.push(decode_endgame_record(&bytes[start..end]));
        start = end + 1;
    }
    if records.len() < EXPECTED_RECORD_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{ENDMSG_DAT_FILE}: expected at least {EXPECTED_RECORD_COUNT} records, found {}",
                records.len()
            ),
        ));
    }
    Ok(EndgameMessages { records })
}

fn decode_endgame_record(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x0a | 0x0d => out.push('\n'),
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

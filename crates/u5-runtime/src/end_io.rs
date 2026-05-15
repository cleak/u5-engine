//! Loader/decoder for `END.DAT`: ~3,698 bytes of narrative text used by the
//! endgame's six fixed final-presentation windows. Spec:
//! `formats/end-dat.md` §2-§4.
//!
//! The on-disk asset has no in-file table. The consumer (the endgame's
//! final-presentation helper) supplies a file-relative seek window. This
//! module exposes the decoded full text plus a `decode_end_window` helper
//! that strips the proportional-text layout markers (`{` page/paragraph,
//! `_` soft hyphen) for any byte slice the caller selects.

use std::fs;
use std::io;
use std::path::Path;

const END_DAT_FILE: &str = "END.DAT";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndNarrative {
    pub raw: Vec<u8>,
}

impl EndNarrative {
    pub fn full_text(&self) -> String {
        decode_end_window(&self.raw)
    }

    pub fn window(&self, start: usize, end: usize) -> Option<String> {
        if end > self.raw.len() || start > end {
            return None;
        }
        Some(decode_end_window(&self.raw[start..end]))
    }
}

pub fn load_end_narrative(game_dir: &Path) -> io::Result<Option<EndNarrative>> {
    let path = game_dir.join(END_DAT_FILE);
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
    Ok(Some(EndNarrative { raw: bytes }))
}

pub fn decode_end_window(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &byte in bytes {
        match byte {
            0x00 => break,
            0x0a | 0x0d => out.push('\n'),
            // formats/end-dat.md §3: `{` is a page/paragraph marker and `_`
            // is a soft hyphen; both are layout hints, not visible glyphs.
            b'{' | b'_' => {}
            ch if (0x20..=0x7e).contains(&ch) => out.push(ch as char),
            _ => {}
        }
    }
    out
}

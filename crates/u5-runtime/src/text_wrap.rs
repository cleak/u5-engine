//! Word-wrap helper for the fixed-cell text-window printer per
//! `systems/text-output.md` §6.
//!
//! Splits a NUL/LF/CR-terminated input stream into wrapped lines that fit a
//! window of `window_width` cells. The first emitted line uses
//! `window_width - cursor_x_at_entry` cells of available width to honour text
//! already on the current row; subsequent lines use the full window width.
//!
//! Break bytes (space / LF / CR / NUL) only act as wrap candidates as
//! described in the spec. NUL stops reading. LF/CR flush immediately and pass
//! through as a hard newline. Visible bytes append to the assembled buffer.
//! Words longer than the window overflow per the original behaviour.

/// `text-output.md §6` byte classification consumed by the wrap-aware
/// printer. A `Break` byte is space/LF/CR/NUL; a `Visible` byte is any
/// other low-ASCII printable; a `Control` byte is anything the per-cell
/// emitter handles (style toggles) and which passes through unchanged
/// without interrupting the wrap state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapByteKind {
    Break,
    Visible,
    Control,
}

/// `text-output.md §6`: classify one source byte for the wrap-aware
/// printer's break/visible/control decision.
pub const fn wrap_byte_kind(byte: u8) -> WrapByteKind {
    match byte {
        0x00 | b'\n' | b'\r' | b' ' => WrapByteKind::Break,
        // Low-ASCII printable range minus the space already covered above
        0x21..=0x7E => WrapByteKind::Visible,
        _ => WrapByteKind::Control,
    }
}

/// `text-output.md §6` minimum line buffer width — the original
/// implementation sizes the assembled-line buffer for at least 64
/// characters.
pub const WRAP_MIN_LINE_BUFFER: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WrappedLine<'a> {
    pub text: &'a str,
    /// True when the source supplied a hard newline (LF/CR) immediately after
    /// this line; used by the per-cell emitter to advance the cursor and
    /// reset the wrap state.
    pub hard_break: bool,
}

/// Wrap `source` into lines that fit `window_width`, treating
/// `cursor_x_at_entry` as text already present on the first row. Stops at
/// the first NUL byte. The returned lines do not include the trailing space
/// or hard-break byte.
pub fn wrap_text(source: &str, window_width: usize, cursor_x_at_entry: usize) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut lines: Vec<String> = Vec::new();
    let mut buffer = String::new();
    let mut last_break: Option<usize> = None;
    let first_line_width = window_width.saturating_sub(cursor_x_at_entry);
    let mut emitted_any = false;

    let line_width =
        |emitted_any: bool| if emitted_any { window_width } else { first_line_width };

    for &byte in bytes {
        match byte {
            0x00 => break,
            b' ' => {
                if buffer.len() >= line_width(emitted_any) {
                    if let Some(break_at) = last_break {
                        let surplus = buffer.split_off(break_at);
                        let trimmed = buffer.trim_end_matches(' ').to_string();
                        lines.push(trimmed);
                        emitted_any = true;
                        buffer = surplus.trim_start_matches(' ').to_string();
                        last_break = None;
                        // The current space caused the wrap; re-evaluate whether
                        // it still fits on the new line so it can serve as the
                        // next break candidate.
                        if buffer.len() < line_width(emitted_any) {
                            buffer.push(' ');
                            last_break = Some(buffer.len() - 1);
                        }
                    } else {
                        // No earlier break point: the buffer is one giant
                        // word that fully filled the line. Emit it as-is and
                        // start the next line empty. The trigger space is
                        // consumed silently.
                        let trimmed = buffer.trim_end_matches(' ').to_string();
                        lines.push(trimmed);
                        emitted_any = true;
                        buffer.clear();
                        last_break = None;
                    }
                } else {
                    buffer.push(' ');
                    last_break = Some(buffer.len() - 1);
                }
            }
            0x0a | 0x0d => {
                let trimmed = buffer.trim_end_matches(' ').to_string();
                lines.push(trimmed);
                emitted_any = true;
                buffer.clear();
                last_break = None;
            }
            ch if (0x20..=0x7e).contains(&ch) => {
                buffer.push(ch as char);
            }
            _ => {
                // Control bytes pass through unchanged per §6.
                buffer.push(byte as char);
            }
        }
    }
    if !buffer.is_empty() {
        let trimmed = buffer.trim_end_matches(' ').to_string();
        lines.push(trimmed);
    }
    lines
}

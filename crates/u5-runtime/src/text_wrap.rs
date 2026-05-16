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

/// `text-output.md §2` fixed text-window count. The system maintains
/// exactly four window descriptors; addressing a fifth window is
/// silently ignored.
pub const TEXT_WINDOW_COUNT: usize = 4;

/// `text-output.md §4` cell-grid extent in columns and rows. The
/// top-left cell is `(0, 0)` and the bottom-right is `(39, 24)`.
pub const TEXT_SCREEN_COLUMNS: u8 = 40;
pub const TEXT_SCREEN_ROWS: u8 = 25;

/// `text-output.md §4` text-window inner width in cells:
/// `bottom_right_x - top_left_x`. The trailing column is excluded —
/// a window whose corners are columns 6 and 33 has width 27 (27
/// characters fit before wrapping is forced). The wrap-aware
/// printer's word-break logic and the centring helper both consume
/// this figure. Returns 0 when the corners are inverted (callers
/// should normalise the descriptor before calling).
pub const fn text_window_inner_width(top_left_x: u8, bottom_right_x: u8) -> u8 {
    if bottom_right_x > top_left_x {
        bottom_right_x - top_left_x
    } else {
        0
    }
}

/// `text-output.md §5` centred-line starting column. When the
/// active window's centre flag is set, the wrap-aware printer
/// repositions the cursor to `(width - characters_in_line) / 2`
/// before emitting. Returns 0 when the line is wider than the
/// window (centring becomes a no-op rather than producing a
/// negative column).
pub const fn text_window_centred_start_column(window_width: u8, line_chars: u8) -> u8 {
    if line_chars >= window_width {
        0
    } else {
        (window_width - line_chars) / 2
    }
}

/// `text-output.md §9` rectangle-setter normaliser. The setter
/// clamps each X to `0..=39` and each Y to `0..=24`, then swaps
/// the X pair if `supplied_left > supplied_right` and the Y pair
/// if `supplied_top > supplied_bottom`. Returns
/// `(top_left_x, top_left_y, bottom_right_x, bottom_right_y)` with
/// the rectangle invariant `top_left_x <= bottom_right_x` and
/// `top_left_y <= bottom_right_y` enforced. Out-of-range window
/// indices are the caller's silent no-op.
pub const fn text_window_clamp_rectangle(
    supplied_x1: u8,
    supplied_y1: u8,
    supplied_x2: u8,
    supplied_y2: u8,
) -> (u8, u8, u8, u8) {
    let max_x = TEXT_SCREEN_COLUMNS - 1;
    let max_y = TEXT_SCREEN_ROWS - 1;
    let x1 = if supplied_x1 > max_x { max_x } else { supplied_x1 };
    let x2 = if supplied_x2 > max_x { max_x } else { supplied_x2 };
    let y1 = if supplied_y1 > max_y { max_y } else { supplied_y1 };
    let y2 = if supplied_y2 > max_y { max_y } else { supplied_y2 };
    let (left, right) = if x1 > x2 { (x2, x1) } else { (x1, x2) };
    let (top, bottom) = if y1 > y2 { (y2, y1) } else { (y1, y2) };
    (left, top, right, bottom)
}

/// `text-output.md §9` boot-time text-window defaults. After
/// `Window descriptor defaults`, all four windows have:
/// - rectangle `(0, 0)..=(39, 24)` (full 40-by-25 screen);
/// - cursor `(0, 0)` (window-local);
/// - colour foreground 15 (bright white) on background 0 (black);
/// - all style flags cleared.
/// The active window is index 0.
pub const TEXT_WINDOW_DEFAULT_FOREGROUND: u8 = 15;
pub const TEXT_WINDOW_DEFAULT_BACKGROUND: u8 = 0;
pub const TEXT_WINDOW_DEFAULT_ACTIVE_INDEX: u8 = 0;

/// `text-output.md §9`: the packed colour byte produced by the
/// boot defaults (low nibble fg, high nibble bg).
pub const fn text_window_default_color_byte() -> u8 {
    (TEXT_WINDOW_DEFAULT_BACKGROUND << 4) | TEXT_WINDOW_DEFAULT_FOREGROUND
}

/// `text-output.md §3` extended text-control bytes that mutate the
/// active window's cached style without rendering as glyphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextControlByte {
    /// `0xFB` — clear centre-output flag.
    CentreOff,
    /// `0xFC` — set centre-output flag.
    CentreOn,
    /// `0xFD` — toggle inverse video.
    InverseToggle,
    /// `0xFE` — toggle underline.
    UnderlineToggle,
    /// `0xFF` — clear the active text window's rectangle through the
    /// display-driver fill path.
    ClearWindow,
}

/// `text-output.md §5` per-cell emitter action classified from one
/// byte. The emitter's three behaviour families are: render a low
/// printable byte as a glyph; treat LF/CR as cursor moves without a
/// glyph; consume a confirmed extended control byte through the
/// style/clear path. Other bytes have no public glyph meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmitterByteKind {
    /// Low-bit-clear printable byte rendered as a glyph at the
    /// active window's cursor; cursor advances one cell after.
    Glyph(u8),
    /// `0x0A` line-feed — cursor steps down one row, may scroll.
    LineFeed,
    /// `0x0D` carriage-return — cursor returns to the window's
    /// left edge without changing the row.
    CarriageReturn,
    /// One of the published extended control bytes (Section 3).
    Control(TextControlByte),
    /// High-bit byte outside the confirmed control range. The
    /// emitter does not render or move the cursor.
    Other,
}

/// `text-output.md §5`: classify a single byte handed to the per-
/// cell emitter. Caller has already done any case folding; this
/// helper does no further translation.
pub const fn text_emitter_byte_kind(byte: u8) -> EmitterByteKind {
    match byte {
        0x0A => EmitterByteKind::LineFeed,
        0x0D => EmitterByteKind::CarriageReturn,
        0x20..=0x7E => EmitterByteKind::Glyph(byte),
        0xFB..=0xFF => match text_control_byte(byte) {
            Some(c) => EmitterByteKind::Control(c),
            None => EmitterByteKind::Other,
        },
        _ => EmitterByteKind::Other,
    }
}

/// `text-output.md §3`: classify a high-bit byte against the spec's
/// extended-control table. Returns `None` for bytes that are not one
/// of the five confirmed control values; callers handle those through
/// the per-cell emitter's ordinary code-byte path.
pub const fn text_control_byte(byte: u8) -> Option<TextControlByte> {
    Some(match byte {
        0xFB => TextControlByte::CentreOff,
        0xFC => TextControlByte::CentreOn,
        0xFD => TextControlByte::InverseToggle,
        0xFE => TextControlByte::UnderlineToggle,
        0xFF => TextControlByte::ClearWindow,
        _ => return None,
    })
}

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

/// `formats/font-pcs.md §4` paragraph-renderer byte classification for
/// the proportional-font intro/chargen layout. The renderer's special
/// bytes are space (word-wrap opportunity), newline (forced break),
/// underscore (soft hyphen marker), and `{` (paragraph/page marker
/// owned by the surrounding caller flow). NUL terminates the buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParagraphByteKind {
    /// `0x00` — NUL terminator stops paragraph reading.
    EndOfStream,
    /// Space — word-wrap opportunity; the renderer can break here.
    SpaceBreak,
    /// LF or CR — forced line break.
    HardBreak,
    /// `_` — soft-hyphen marker; the renderer wraps at it but does not
    /// emit a glyph unless the line wraps here.
    SoftHyphen,
    /// `{` — paragraph/page marker handled by the caller flow rather
    /// than by the renderer itself.
    PageMarker,
    /// Any other printable byte — measured against the width table and
    /// rendered through the font segment.
    Glyph,
}

/// `formats/font-pcs.md §4`: classify one source byte for the
/// proportional paragraph renderer.
pub const fn paragraph_byte_kind(byte: u8) -> ParagraphByteKind {
    match byte {
        0x00 => ParagraphByteKind::EndOfStream,
        b' ' => ParagraphByteKind::SpaceBreak,
        b'\n' | b'\r' => ParagraphByteKind::HardBreak,
        b'_' => ParagraphByteKind::SoftHyphen,
        b'{' => ParagraphByteKind::PageMarker,
        _ => ParagraphByteKind::Glyph,
    }
}

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

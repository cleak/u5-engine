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
    let x1 = if supplied_x1 > max_x {
        max_x
    } else {
        supplied_x1
    };
    let x2 = if supplied_x2 > max_x {
        max_x
    } else {
        supplied_x2
    };
    let y1 = if supplied_y1 > max_y {
        max_y
    } else {
        supplied_y1
    };
    let y2 = if supplied_y2 > max_y {
        max_y
    } else {
        supplied_y2
    };
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
pub const TEXT_WINDOW_FLAG_UNDERLINE: u8 = 0x01;
pub const TEXT_WINDOW_FLAG_CENTRE: u8 = 0x02;
pub const TEXT_WINDOW_FLAG_INVERSE: u8 = 0x04;

/// `text-output.md §3` text-window packed colour-byte layout. The
/// active window's colour attribute carries the foreground palette
/// index in its low nibble and the background palette index in its
/// high nibble. Both nibbles are four bits wide; specific palette
/// entries are a driver concern.
pub const TEXT_COLOR_FOREGROUND_MASK: u8 = 0x0F;
pub const TEXT_COLOR_BACKGROUND_SHIFT: u32 = 4;

/// `text-output.md §3`: extract the foreground palette index from a
/// packed text-window colour byte.
pub const fn text_color_foreground(packed: u8) -> u8 {
    packed & TEXT_COLOR_FOREGROUND_MASK
}

/// `text-output.md §3`: extract the background palette index from a
/// packed text-window colour byte.
pub const fn text_color_background(packed: u8) -> u8 {
    packed >> TEXT_COLOR_BACKGROUND_SHIFT
}

/// `text-output.md §9`: the packed colour byte produced by the
/// boot defaults (low nibble fg, high nibble bg).
pub const fn text_window_default_color_byte() -> u8 {
    (TEXT_WINDOW_DEFAULT_BACKGROUND << TEXT_COLOR_BACKGROUND_SHIFT) | TEXT_WINDOW_DEFAULT_FOREGROUND
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextCell {
    pub byte: u8,
    pub color: u8,
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextWindowDescriptor {
    pub top_left_x: u8,
    pub top_left_y: u8,
    pub bottom_right_x: u8,
    pub bottom_right_y: u8,
    pub cursor_x: u8,
    pub cursor_y: u8,
    pub color: u8,
    pub flags: u8,
}

impl Default for TextWindowDescriptor {
    fn default() -> Self {
        Self {
            top_left_x: 0,
            top_left_y: 0,
            bottom_right_x: TEXT_SCREEN_COLUMNS - 1,
            bottom_right_y: TEXT_SCREEN_ROWS - 1,
            cursor_x: 0,
            cursor_y: 0,
            color: text_window_default_color_byte(),
            flags: 0,
        }
    }
}

impl TextWindowDescriptor {
    pub const fn inner_width(self) -> u8 {
        text_window_inner_width(self.top_left_x, self.bottom_right_x)
    }

    pub const fn height(self) -> u8 {
        self.bottom_right_y - self.top_left_y + 1
    }

    pub const fn absolute_cursor(self) -> Option<(u8, u8)> {
        let x = self.top_left_x.saturating_add(self.cursor_x);
        let y = self.top_left_y.saturating_add(self.cursor_y);
        if x < TEXT_SCREEN_COLUMNS && y < TEXT_SCREEN_ROWS {
            Some((x, y))
        } else {
            None
        }
    }

    pub const fn centre_enabled(self) -> bool {
        self.flags & TEXT_WINDOW_FLAG_CENTRE != 0
    }

    pub const fn underline_enabled(self) -> bool {
        self.flags & TEXT_WINDOW_FLAG_UNDERLINE != 0
    }

    pub const fn inverse_enabled(self) -> bool {
        self.flags & TEXT_WINDOW_FLAG_INVERSE != 0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextWindowSystem {
    windows: [TextWindowDescriptor; TEXT_WINDOW_COUNT],
    active_window: usize,
    cells: Vec<Option<TextCell>>,
    cursor_advance_enabled: bool,
}

impl Default for TextWindowSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl TextWindowSystem {
    pub fn new() -> Self {
        Self {
            windows: [TextWindowDescriptor::default(); TEXT_WINDOW_COUNT],
            active_window: TEXT_WINDOW_DEFAULT_ACTIVE_INDEX as usize,
            cells: vec![None; TEXT_SCREEN_COLUMNS as usize * TEXT_SCREEN_ROWS as usize],
            cursor_advance_enabled: true,
        }
    }

    pub fn active_window_index(&self) -> usize {
        self.active_window
    }

    pub fn window(&self, index: usize) -> Option<TextWindowDescriptor> {
        self.windows.get(index).copied()
    }

    pub fn active_window(&self) -> TextWindowDescriptor {
        self.windows[self.active_window]
    }

    pub fn set_active_window(&mut self, index: usize) {
        if index < TEXT_WINDOW_COUNT {
            self.active_window = index;
        }
    }

    pub fn set_window_rect(&mut self, index: usize, x1: u8, y1: u8, x2: u8, y2: u8) {
        let Some(window) = self.windows.get_mut(index) else {
            return;
        };
        let (left, top, right, bottom) = text_window_clamp_rectangle(x1, y1, x2, y2);
        window.top_left_x = left;
        window.top_left_y = top;
        window.bottom_right_x = right;
        window.bottom_right_y = bottom;
    }

    pub fn set_active_color(&mut self, color: u8) {
        self.windows[self.active_window].color = color;
    }

    pub fn set_active_flags(&mut self, flags: u8) {
        self.windows[self.active_window].flags = flags;
    }

    pub fn clear_active_flags(&mut self) {
        self.windows[self.active_window].flags = 0;
    }

    pub fn active_cursor(&self) -> (u8, u8) {
        let window = self.active_window();
        (window.cursor_x, window.cursor_y)
    }

    pub fn set_active_cursor(&mut self, x: u8, y: u8) {
        let window = self.active_window();
        let absolute_x = window.top_left_x.saturating_add(x);
        let absolute_y = window.top_left_y.saturating_add(y);
        if absolute_x < TEXT_SCREEN_COLUMNS && absolute_y < TEXT_SCREEN_ROWS {
            let window = &mut self.windows[self.active_window];
            window.cursor_x = x;
            window.cursor_y = y;
        }
    }

    pub fn cell(&self, x: u8, y: u8) -> Option<TextCell> {
        if x >= TEXT_SCREEN_COLUMNS || y >= TEXT_SCREEN_ROWS {
            return None;
        }
        self.cells[usize::from(y) * TEXT_SCREEN_COLUMNS as usize + usize::from(x)]
    }

    pub fn screen_rows(&self, fill: u8) -> Vec<String> {
        self.region_rows(0, 0, TEXT_SCREEN_COLUMNS - 1, TEXT_SCREEN_ROWS - 1, fill)
    }

    pub fn region_rows(&self, x1: u8, y1: u8, x2: u8, y2: u8, fill: u8) -> Vec<String> {
        let (left, top, right, bottom) = text_window_clamp_rectangle(x1, y1, x2, y2);
        let mut rows = Vec::with_capacity(usize::from(bottom - top + 1));
        for y in top..=bottom {
            let mut row = String::with_capacity(usize::from(right - left + 1));
            for x in left..=right {
                row.push(char::from(
                    self.cell(x, y).map(|cell| cell.byte).unwrap_or(fill),
                ));
            }
            rows.push(row);
        }
        rows
    }

    pub fn emit_byte(&mut self, byte: u8) {
        match text_emitter_byte_kind(byte) {
            EmitterByteKind::Glyph(glyph) => self.emit_glyph(glyph),
            EmitterByteKind::LineFeed => self.line_feed(),
            EmitterByteKind::CarriageReturn => self.carriage_return(),
            EmitterByteKind::Control(control) => self.apply_control(control),
            EmitterByteKind::Other => {}
        }
    }

    pub fn print_wrapped_string(&mut self, source: &str) {
        if source.is_empty() {
            return;
        }
        let window = self.active_window();
        let width = usize::from(window.inner_width()).max(1);
        let lines = wrap_text(source, width, usize::from(window.cursor_x));
        let last = lines.len().saturating_sub(1);
        for (index, line) in lines.into_iter().enumerate() {
            if self.active_window().centre_enabled() {
                let start = text_window_centred_start_column(
                    self.active_window().inner_width(),
                    line.len().min(u8::MAX as usize) as u8,
                );
                self.set_active_cursor(start, self.active_window().cursor_y);
            }
            let before_y = self.active_window().cursor_y;
            for byte in line.bytes() {
                self.emit_byte(byte);
            }
            if index != last && self.active_window().cursor_y == before_y {
                self.emit_byte(b'\r');
                self.emit_byte(b'\n');
            }
        }
    }

    pub fn print_number(&mut self, value: i16, width: usize, pad: u8) {
        let rendered = format_signed_number(value, width.min(39), char::from(pad));
        self.print_wrapped_string(&rendered);
    }

    pub fn erase_typed_spaces(&mut self, count: usize) {
        let saved = self.active_cursor();
        let old_gate = self.cursor_advance_enabled;
        self.cursor_advance_enabled = false;
        for offset in 0..count {
            let x = saved.0.saturating_add(offset.min(u8::MAX as usize) as u8);
            self.set_active_cursor(x, saved.1);
            self.emit_byte(b' ');
        }
        self.cursor_advance_enabled = old_gate;
        self.set_active_cursor(saved.0, saved.1);
    }

    pub fn paint_cursor_glyph(&mut self, glyph: u8) {
        let saved = self.active_cursor();
        let old_gate = self.cursor_advance_enabled;
        self.cursor_advance_enabled = false;
        self.emit_glyph(glyph);
        self.cursor_advance_enabled = old_gate;
        self.set_active_cursor(saved.0, saved.1);
    }

    fn emit_glyph(&mut self, glyph: u8) {
        let window = self.active_window();
        if let Some((x, y)) = window.absolute_cursor() {
            let index = usize::from(y) * TEXT_SCREEN_COLUMNS as usize + usize::from(x);
            self.cells[index] = Some(TextCell {
                byte: glyph,
                color: window.color,
                underline: window.underline_enabled(),
                inverse: window.inverse_enabled(),
            });
        }
        if self.cursor_advance_enabled {
            self.advance_cursor_after_glyph();
        }
    }

    fn advance_cursor_after_glyph(&mut self) {
        let printable_width = self.active_window().inner_width().max(1);
        let window = &mut self.windows[self.active_window];
        window.cursor_x = window.cursor_x.saturating_add(1);
        if window.cursor_x >= printable_width {
            window.cursor_x = 0;
            self.line_feed();
        }
    }

    fn line_feed(&mut self) {
        let height = self.active_window().height();
        let window = &mut self.windows[self.active_window];
        window.cursor_y = window.cursor_y.saturating_add(1);
        if window.cursor_y >= height {
            window.cursor_y = height.saturating_sub(1);
            self.scroll_active_window();
        }
    }

    fn carriage_return(&mut self) {
        self.windows[self.active_window].cursor_x = 0;
    }

    fn apply_control(&mut self, control: TextControlByte) {
        let window = &mut self.windows[self.active_window];
        match control {
            TextControlByte::CentreOff => window.flags &= !TEXT_WINDOW_FLAG_CENTRE,
            TextControlByte::CentreOn => window.flags |= TEXT_WINDOW_FLAG_CENTRE,
            TextControlByte::InverseToggle => window.flags ^= TEXT_WINDOW_FLAG_INVERSE,
            TextControlByte::UnderlineToggle => window.flags ^= TEXT_WINDOW_FLAG_UNDERLINE,
            TextControlByte::ClearWindow => self.clear_active_window_cells(),
        }
    }

    fn clear_active_window_cells(&mut self) {
        let window = self.active_window();
        for y in window.top_left_y..=window.bottom_right_y {
            for x in window.top_left_x..=window.bottom_right_x {
                let index = usize::from(y) * TEXT_SCREEN_COLUMNS as usize + usize::from(x);
                self.cells[index] = None;
            }
        }
    }

    fn scroll_active_window(&mut self) {
        let window = self.active_window();
        for y in window.top_left_y..window.bottom_right_y {
            for x in window.top_left_x..=window.bottom_right_x {
                let dst = usize::from(y) * TEXT_SCREEN_COLUMNS as usize + usize::from(x);
                let src = usize::from(y + 1) * TEXT_SCREEN_COLUMNS as usize + usize::from(x);
                self.cells[dst] = self.cells[src];
            }
        }
        for x in window.top_left_x..=window.bottom_right_x {
            let index =
                usize::from(window.bottom_right_y) * TEXT_SCREEN_COLUMNS as usize + usize::from(x);
            self.cells[index] = None;
        }
    }
}

pub fn format_signed_number(value: i16, width: usize, pad: char) -> String {
    let value_text = value.to_string();
    if value_text.len() >= width {
        value_text
    } else {
        let mut rendered = String::with_capacity(width);
        for _ in 0..width - value_text.len() {
            rendered.push(pad);
        }
        rendered.push_str(&value_text);
        rendered
    }
}

/// `text-output.md §3` extended text-control byte values. The per-
/// cell emitter consumes these high-bit bytes through the
/// style/clear path rather than rendering them as glyphs.
pub const TEXT_CTRL_CENTRE_OFF: u8 = 0xFB;
pub const TEXT_CTRL_CENTRE_ON: u8 = 0xFC;
pub const TEXT_CTRL_INVERSE_TOGGLE: u8 = 0xFD;
pub const TEXT_CTRL_UNDERLINE_TOGGLE: u8 = 0xFE;
pub const TEXT_CTRL_CLEAR_WINDOW: u8 = 0xFF;
/// `text-output.md §5` low end of the extended text-control range
/// the per-cell emitter probes. Bytes below this value with the high
/// bit set are routed through the ordinary code-byte path with no
/// public glyph meaning.
pub const TEXT_CTRL_RANGE_FIRST: u8 = TEXT_CTRL_CENTRE_OFF;
/// `text-output.md §5` high end of the extended text-control range
/// (`0xFF`).
pub const TEXT_CTRL_RANGE_LAST: u8 = TEXT_CTRL_CLEAR_WINDOW;

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
        TEXT_CTRL_RANGE_FIRST..=TEXT_CTRL_RANGE_LAST => match text_control_byte(byte) {
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
        TEXT_CTRL_CENTRE_OFF => TextControlByte::CentreOff,
        TEXT_CTRL_CENTRE_ON => TextControlByte::CentreOn,
        TEXT_CTRL_INVERSE_TOGGLE => TextControlByte::InverseToggle,
        TEXT_CTRL_UNDERLINE_TOGGLE => TextControlByte::UnderlineToggle,
        TEXT_CTRL_CLEAR_WINDOW => TextControlByte::ClearWindow,
        _ => return None,
    })
}

/// `text-output.md §8` proportional-renderer byte vocabulary.
/// The FONT-overlay proportional renderer (used by intro slides,
/// chargen prompts, and other proportional-text screens) consumes
/// text byte-by-byte until NUL.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProportionalRendererByteKind {
    /// Ordinary visible byte — draw one proportional glyph at the
    /// current pixel cursor and advance by the glyph's width.
    Glyph(u8),
    /// `' '` (space) — legal word-break candidate. The renderer
    /// looks ahead to decide whether the next word fits before the
    /// right edge and may break here instead of drawing the space
    /// past the edge.
    WordBreakSpace,
    /// `'\n'` (line feed) — hard newline.
    HardNewline,
    /// `'_'` (underscore) — soft hyphen / syllable marker. Emits
    /// no glyph but is a legal in-word break point.
    SoftBreak,
    /// `'{'` (left brace) — paragraph-start / page marker. Emits no
    /// glyph; the renderer does not itself wait for input. The
    /// caller's record loop supplies any pause.
    ParagraphStart,
    /// `\0` (NUL) — end of the text buffer. The renderer stops
    /// consuming bytes here.
    EndOfRecord,
}

/// `text-output.md §8`: classify one byte for the proportional
/// renderer. Caller has already loaded the NUL-terminated text
/// record into the working buffer.
pub const fn proportional_renderer_byte_kind(byte: u8) -> ProportionalRendererByteKind {
    match byte {
        0 => ProportionalRendererByteKind::EndOfRecord,
        b' ' => ProportionalRendererByteKind::WordBreakSpace,
        b'\n' => ProportionalRendererByteKind::HardNewline,
        b'_' => ProportionalRendererByteKind::SoftBreak,
        b'{' => ProportionalRendererByteKind::ParagraphStart,
        other => ProportionalRendererByteKind::Glyph(other),
    }
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

    let line_width = |emitted_any: bool| {
        if emitted_any {
            window_width
        } else {
            first_line_width
        }
    };

    for &byte in bytes {
        match byte {
            0x00 => break,
            b' ' => {
                // `text-output.md §5`: a space is pure soft-break
                // bookkeeping. The width test lives on the visible-byte
                // path below, so that the *next* character — not the next
                // space — is what forces the back-up to this break.
                buffer.push(' ');
                last_break = Some(buffer.len() - 1);
            }
            0x0a | 0x0d => {
                let trimmed = buffer.trim_end_matches(' ').to_string();
                lines.push(trimmed);
                emitted_any = true;
                buffer.clear();
                last_break = None;
            }
            ch if (0x20..=0x7e).contains(&ch) => {
                // `text-output.md §5`: "When the next character would carry
                // the line past the window's right edge, the printer backs
                // up to the most recent soft break, emits everything up to
                // that break, and begins a new line with the remainder."
                // The test must run before the push, on every visible byte,
                // or a trailing word that crosses the edge survives to the
                // final flush and is hard-split by the per-cell emitter.
                if !buffer.is_empty() && buffer.len() + 1 > line_width(emitted_any) {
                    match last_break {
                        Some(break_at) => {
                            let surplus = buffer.split_off(break_at);
                            let trimmed = buffer.trim_end_matches(' ').to_string();
                            lines.push(trimmed);
                            emitted_any = true;
                            buffer = surplus.trim_start_matches(' ').to_string();
                            last_break = None;
                        }
                        None => {
                            // A single word wider than the window. §6 calls
                            // this degenerate and allows a stricter
                            // behaviour than the original overflow: emit the
                            // filled line as-is and restart.
                            lines.push(buffer.clone());
                            emitted_any = true;
                            buffer.clear();
                        }
                    }
                }
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

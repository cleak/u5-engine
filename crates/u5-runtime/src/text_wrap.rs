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
//! Nothing is ever written past `bottom_right_x`. A chunk that fills the
//! remaining row with no break byte to retreat to is kept whole, preceded
//! by a line feed if the cursor is not already at the left edge, and
//! printed from column 0; the next chunk continues on that row where it
//! stopped, so a word too long for the *remainder* of a row lands whole on
//! the next one while a word longer than a full row is hard-broken into
//! successive row-filling pieces (`RETRACTIONS.md` R346, R349).

/// `text-output.md §2` fixed text-window count. The system maintains
/// exactly four window descriptors; addressing a fifth window is
/// silently ignored.
pub const TEXT_WINDOW_COUNT: usize = 4;

/// `text-output.md §4` cell-grid extent in columns and rows. The
/// top-left cell is `(0, 0)` and the bottom-right is `(39, 24)`.
pub const TEXT_SCREEN_COLUMNS: u8 = 40;
pub const TEXT_SCREEN_ROWS: u8 = 25;

/// `text-output.md §4` text-window **inclusive-endpoint budget**:
/// `bottom_right_x - top_left_x`. This is the *last legal
/// window-local column*, which the printer's wrap and centring
/// arithmetic genuinely carry in that form — it is **one less than
/// the number of cells a row holds and must never be used as a
/// character count**. Use [`text_window_capacity`] for the count.
/// Returns 0 when the corners are inverted (callers should normalise
/// the descriptor before calling).
///
/// *(Corrected: an earlier revision called this the window's width,
/// said it "does not include the trailing column", and gave the 6..33
/// window twenty-seven characters before a forced wrap. That is
/// withdrawn — twenty-eight fit. `RETRACTIONS.md` R344.)*
pub const fn text_window_inner_width(top_left_x: u8, bottom_right_x: u8) -> u8 {
    if bottom_right_x > top_left_x {
        bottom_right_x - top_left_x
    } else {
        0
    }
}

/// `text-output.md §4` text-window **capacity** in cells:
/// "A window's **capacity** in cells is `bottom_right_x - top_left_x
/// + 1`: both corner columns are inclusive, so the trailing column is
/// usable and a glyph is written into it normally. A window whose
/// corners are columns 6 and 33 therefore holds **twenty-eight**
/// characters on one row, and the twenty-ninth is what forces the
/// wrap."
///
/// This is the figure the wrap-aware printer accepts on a row and the
/// figure the per-cell emitter writes before wrapping — §6: "The
/// number of characters the printer will actually accept on the row
/// is that value **plus one** ... which is exactly the number of
/// cells the per-cell emitter accepts before wrapping, so the two
/// primitives agree and neither carries an off-by-one."
pub const fn text_window_capacity(top_left_x: u8, bottom_right_x: u8) -> u8 {
    text_window_inner_width(top_left_x, bottom_right_x) + 1
}

/// `text-output.md §5` centred-line starting column for a line
/// emitted with the cursor at the window's left edge:
/// `(columns_in_window - characters_in_line) / 2`, truncating.
///
/// `window_width` here is the window's **column count**,
/// `bottom_right_x - top_left_x + 1` — *not* the wrap width returned
/// by [`text_window_inner_width`]. The spec is explicit that an
/// implementation which drops the plus one and centres against
/// `bottom_right_x - top_left_x` agrees on odd-length lines but
/// shifts every even-length line one whole cell left. Use
/// [`TextWindowDescriptor::column_count`] to obtain it.
///
/// Returns 0 when the line is wider than the window (centring
/// becomes a no-op rather than producing a negative column).
pub const fn text_window_centred_start_column(window_width: u8, line_chars: u8) -> u8 {
    if line_chars >= window_width {
        0
    } else {
        (window_width - line_chars) / 2
    }
}

/// `text-output.md §5` centred-line starting column in the printer's
/// exact published form, which also handles a mid-row cursor.
///
/// The computation works from two quantities: the columns still
/// *available* on the current row, `(bottom_right_x - top_left_x)
/// - cursor_x` as the printer was entered, and the **index of the
/// last character** of the line about to be emitted, one less than
/// its character count. The starting column is
/// `(available - last_character_index) / 2`, truncated.
///
/// With the cursor at the window's left edge this reduces to
/// `(columns_in_window - characters_in_line) / 2`, i.e.
/// [`text_window_centred_start_column`] fed the window's column
/// count. Returns 0 when the line cannot fit in what remains of the
/// row, rather than producing a negative column.
pub const fn text_window_centred_start_column_from_cursor(
    inner_width: u8,
    cursor_x: u8,
    line_chars: u8,
) -> u8 {
    if line_chars == 0 {
        return 0;
    }
    let available = inner_width.saturating_sub(cursor_x);
    let last_character_index = line_chars - 1;
    if last_character_index >= available {
        0
    } else {
        (available - last_character_index) / 2
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

    /// `text-output.md §4`/§5 `columns_in_window`, i.e. the row's
    /// **capacity**: `bottom_right_x - top_left_x + 1`. This is the
    /// figure the centre branch measures against, the number of
    /// characters the wrap-aware printer accepts on a row, and the
    /// number of cells the per-cell emitter writes before it wraps —
    /// one more than the inclusive-endpoint budget returned by
    /// [`Self::inner_width`] (`RETRACTIONS.md` R344, R345).
    pub const fn column_count(self) -> u8 {
        text_window_capacity(self.top_left_x, self.bottom_right_x)
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

    /// Clear one window-local row in the retained cell surface without
    /// changing the descriptor or its cursor. This is a modern compositor
    /// helper used when rebuilding the live prompt line into gameplay window
    /// 2; it is not a resident text control and does not model a fourth window.
    pub fn clear_active_row(&mut self, row: u8) {
        let window = self.active_window();
        if row >= window.height() {
            return;
        }
        let y = window.top_left_y + row;
        let start = usize::from(y) * TEXT_SCREEN_COLUMNS as usize + usize::from(window.top_left_x);
        let end =
            usize::from(y) * TEXT_SCREEN_COLUMNS as usize + usize::from(window.bottom_right_x) + 1;
        self.cells[start..end].fill(None);
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

    /// Emit one fixed-font glyph code without routing it through the text
    /// stream's printable/control classifier. Display-owned badges use resident
    /// `IBM.CH` codes below ASCII space (for example the arms page indicator's
    /// `0x01`/`0x02` caps), which are glyphs in that context rather than text
    /// controls.
    pub fn emit_fixed_glyph(&mut self, glyph: u8) {
        self.emit_glyph(glyph);
    }

    pub fn print_wrapped_string(&mut self, source: &str) {
        if source.is_empty() {
            return;
        }
        let window = self.active_window();
        // `text-output.md §6` (`RETRACTIONS.md` R345): the row accepts
        // the window's *capacity*, `bottom_right_x - top_left_x + 1`,
        // not the inclusive-endpoint budget. "An implementation that
        // treats the available-width figure directly as a character
        // count loses the last column of every row."
        let width = usize::from(window.column_count()).max(1);
        // `text-output.md §5`: the centre branch measures the columns
        // still available on the row *as the printer was entered*.
        let entry_cursor_x = window.cursor_x;
        let chunks = wrap_text_chunks(source, width, usize::from(window.cursor_x));
        let last = chunks.len().saturating_sub(1);
        for (index, chunk) in chunks.into_iter().enumerate() {
            // `text-output.md §6` (`RETRACTIONS.md` R346), the row-filling
            // arm: the chunk "is kept whole, preceded by a line feed if the
            // cursor is not already at the left edge, and printed from
            // column 0; the next chunk continues on that row where it
            // stopped." Nothing is ever written past `bottom_right_x`, so
            // the truncated piece is never printed in place on the row it
            // overflowed. R349's worked case - eight columns left of
            // sixteen and `Underworld!` - lands whole on the next row by
            // this mechanism rather than by moving the word down.
            if chunk.row_filling && self.active_window().cursor_x != 0 {
                self.emit_byte(b'\n');
            }
            if self.active_window().centre_enabled() {
                let start = text_window_centred_start_column_from_cursor(
                    self.active_window().inner_width(),
                    entry_cursor_x,
                    chunk.text.len().min(u8::MAX as usize) as u8,
                );
                self.set_active_cursor(start, self.active_window().cursor_y);
            }
            let before_y = self.active_window().cursor_y;
            for byte in chunk.text.bytes() {
                self.emit_byte(byte);
            }
            // A row-filling chunk does **not** end its row: "the following
            // chunk then continues on that same row at the column where the
            // first one stopped" (§6). Only a chunk that ended on a break
            // byte or on a soft wrap gets the row advance.
            if index != last && !chunk.row_filling && self.active_window().cursor_y == before_y {
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
        // `text-output.md §5`: the emitter wraps only when the advance
        // "would carry the cursor past `bottom_right_x`" - the cell at
        // the right column is written normally, so a row takes the
        // window's full capacity before wrapping (`RETRACTIONS.md` R344).
        let printable_width = self.active_window().column_count().max(1);
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
        // Resident LF is a combined carriage return + line feed.
        window.cursor_x = 0;
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
        // `text-output.md §10.5` / `display-driver-abi.md §9.5`: the
        // one-row scroll copies from one cell row below the window as well;
        // it does not blank the vacated bottom row. Subsequent output normally
        // covers that row immediately. A full-screen window has no source row
        // below it, so its last row is left as-is until overwritten.
        for y in window.top_left_y..=window.bottom_right_y {
            let Some(source_y) = y.checked_add(1).filter(|source| *source < TEXT_SCREEN_ROWS)
            else {
                continue;
            };
            for x in window.top_left_x..=window.bottom_right_x {
                let dst = usize::from(y) * TEXT_SCREEN_COLUMNS as usize + usize::from(x);
                let src = usize::from(source_y) * TEXT_SCREEN_COLUMNS as usize + usize::from(x);
                self.cells[dst] = self.cells[src];
            }
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
        // `text-output.md §5`: every high-bit-clear byte except LF/CR is
        // passed to the active fixed-cell font. That includes NUL and the
        // low pictogram band; NUL terminates the higher-level string printer,
        // but is still a glyph when handed directly to this primitive.
        0x00..=0x7F => EmitterByteKind::Glyph(byte),
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

/// One chunk the wrap collector gathered, with the flag
/// `systems/text-output.md` Section 6 needs in order to place it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WrapChunk {
    /// The characters of the chunk, in order, without the trailing space or
    /// hard-break byte that ended it.
    pub text: String,
    /// True when this chunk filled the row the collector was measuring
    /// against and carried **no** break byte to retreat to.
    ///
    /// `text-output.md §6` (`RETRACTIONS.md` R346): "When the collected chunk
    /// fills the remaining row and contains no break byte to retreat to, the
    /// printer keeps the whole chunk, first emits a line feed if the cursor
    /// is not already at the window's left edge, and prints the chunk from
    /// column 0 of the fresh row; the following chunk then continues on that
    /// same row at the column where the first one stopped."
    pub row_filling: bool,
}

/// Wrap `source` into lines that fit `window_width`, treating
/// `cursor_x_at_entry` as text already present on the first row. Stops at
/// the first NUL byte. The returned lines do not include the trailing space
/// or hard-break byte.
///
/// This is the text-only view of [`wrap_text_chunks`]; a caller that has to
/// place the chunks on the screen needs the chunk form, because the
/// row-filling arm of Section 6 does not start a new row after itself.
pub fn wrap_text(source: &str, window_width: usize, cursor_x_at_entry: usize) -> Vec<String> {
    wrap_text_chunks(source, window_width, cursor_x_at_entry)
        .into_iter()
        .map(|chunk| chunk.text)
        .collect()
}

/// The wrap collector of `text-output.md` Section 6, returning each chunk
/// together with whether it took the row-filling arm of `RETRACTIONS.md`
/// R346.
pub fn wrap_text_chunks(
    source: &str,
    window_width: usize,
    cursor_x_at_entry: usize,
) -> Vec<WrapChunk> {
    let bytes = source.as_bytes();
    let mut lines: Vec<WrapChunk> = Vec::new();
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
                lines.push(WrapChunk {
                    text: trimmed,
                    row_filling: false,
                });
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
                            lines.push(WrapChunk {
                                text: trimmed,
                                row_filling: false,
                            });
                            emitted_any = true;
                            buffer = surplus.trim_start_matches(' ').to_string();
                            last_break = None;
                        }
                        None => {
                            // The row-filling arm. `text-output.md §6`
                            // (`RETRACTIONS.md` R346): "A word longer than
                            // the row is **not** allowed to overflow the
                            // right edge. The collector never gathers more
                            // characters than the row can still hold" - so
                            // the chunk is exactly the buffer as collected -
                            // and the printer "keeps the whole chunk, first
                            // emits a line feed if the cursor is not already
                            // at the window's left edge, and prints the chunk
                            // from column 0 of the fresh row; the following
                            // chunk then continues on that same row at the
                            // column where the first one stopped."
                            //
                            // The chunk text is the same either way; the flag
                            // is what [`TextWindowSystem::print_wrapped_string`]
                            // needs to place it, because this arm does *not*
                            // start a new row after itself.
                            lines.push(WrapChunk {
                                text: buffer.clone(),
                                row_filling: true,
                            });
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
        lines.push(WrapChunk {
            text: trimmed,
            row_filling: false,
        });
    }
    lines
}

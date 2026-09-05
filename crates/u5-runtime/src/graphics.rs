//! Tile atlas, viewport, palettes, and image/font formats (TileGraphicsDepth, TileAtlas, TileViewport, GraphicImage*, MonochromeBitmap, TitleBitImages, FixedFont, ProportionalFont).

use std::io;

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileGraphicsDepth {
    Ega16,
    Cga4,
}

impl TileGraphicsDepth {
    pub fn from_key(value: &str) -> io::Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "e" | "ega" | "ega16" | "16" | "t" | "tandy" | "tandy1000" | "t1k" => Ok(Self::Ega16),
            "c" | "cga" | "cga4" | "4" => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "CGA raster output is outside the v1 clean recreation target",
            )),
            "h" | "hercules" | "her" | "herc" => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Hercules raster output is outside the v1 clean recreation target",
            )),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("raster depth must be ega or tandy, got `{value}`"),
            )),
        }
    }

    pub fn file_name(self) -> &'static str {
        match self {
            Self::Ega16 => TILES_EGA_FILE,
            Self::Cga4 => TILES_CGA_FILE,
        }
    }

    pub fn body_len(self) -> usize {
        match self {
            Self::Ega16 => TILE_ATLAS_EGA_BODY_LEN,
            Self::Cga4 => TILE_ATLAS_CGA_BODY_LEN,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ega16 => "EGA tile atlas",
            Self::Cga4 => "CGA tile atlas",
        }
    }

    pub fn file_suffix(self) -> &'static str {
        match self {
            Self::Ega16 => "16",
            Self::Cga4 => "4",
        }
    }

    pub fn pixel_limit(self) -> u8 {
        match self {
            Self::Ega16 => 16,
            Self::Cga4 => 4,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileAtlas {
    pub depth: TileGraphicsDepth,
    pub pixels: Vec<u8>,
    /// The three dungeon corridor billboard banks, when the game
    /// directory ships them.
    ///
    /// They live on the atlas because it is exactly the right scope -
    /// the graphics resources for one game directory at one depth,
    /// loaded once and passed by reference to every renderer. Putting
    /// them on `PlayState` would copy megabytes per frame, since the
    /// compositor clones the state twice per frame; putting them in a
    /// process global made rendering depend on whether some earlier
    /// caller happened to load a real game directory.
    pub dungeon_billboards: Option<crate::dungeon_view::DungeonBillboardBanks>,
    /// Masked first-person object and wandering-monster sprite banks.
    pub dungeon_sprites: Option<crate::dungeon_view::DungeonSpriteBanks>,
}

impl TileAtlas {
    pub fn tile_pixels(&self, tile: usize) -> Option<&[u8]> {
        let start = tile.checked_mul(TILE_ATLAS_TILE_PIXELS)?;
        let end = start.checked_add(TILE_ATLAS_TILE_PIXELS)?;
        self.pixels.get(start..end)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileViewport {
    pub depth: TileGraphicsDepth,
    pub cells_wide: usize,
    pub cells_high: usize,
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl TileViewport {
    #[cfg(test)]
    pub fn pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels.get(y * self.width + x).copied()
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let palette: &[[u8; 3]] = match self.depth {
            TileGraphicsDepth::Ega16 => &EGA_PALETTE_RGB,
            TileGraphicsDepth::Cga4 => &CGA_PALETTE_RGB,
        };
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        let limit = palette.len();
        for &index in &self.pixels {
            let rgb = palette[(index as usize) % limit];
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 0xff]);
        }
        rgba
    }
}

/// Index six: the only palette slot whose value has ever been in
/// dispute. It is settled now; see [`EGA_PALETTE_RGB`].
pub const SHIPPED_PALETTE_DEVIATING_INDEX: usize = 6;
/// Dark yellow: what the six-bit register value `0x06` denotes, and
/// what an enhanced display resolves it to.
pub const SHIPPED_PALETTE_DARK_YELLOW: [u8; 3] = [0xaa, 0xaa, 0x00];
/// Brown: what the firmware default `0x14` denotes, and also what a
/// 200-line display resolves `0x06` to, via the correction modelled by
/// [`MonitorModel::Period200Line`].
pub const STOCK_EGA_BROWN: [u8; 3] = [0xaa, 0x55, 0x00];

/// The sixteen six-bit values the shipped program loads into the
/// adapter's palette registers.
///
/// The firmware mode set installs its own defaults first; the program
/// then overwrites all sixteen from a table of its own that is the
/// stock set in fifteen entries and differs only here at index six,
/// where firmware writes `0x14` (brown) and the program writes `0x06`.
/// It is deliberately undoing the firmware's brown special case.
///
/// This is the register state, not a colour. What it *looks like*
/// depends on the display, which is [`MonitorModel`]'s job.
pub const SHIPPED_PALETTE_REGISTERS: [u8; 16] = [
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x38, 0x39, 0x3a, 0x3b, 0x3c, 0x3d, 0x3e, 0x3f,
];

/// Which display the renderer is modelling.
///
/// This exists because "what colour is attribute six" has two correct
/// answers and they differ by hardware, not by palette. Keeping the
/// register table and the display model separate is what stops the
/// question being re-litigated as a palette edit, which is how it went
/// wrong twice before.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitorModel {
    /// An enhanced display, taking each register value at face value.
    /// `0x06` is dark yellow. This is the specification's stated v1
    /// baseline colour space.
    Enhanced,
    /// A period display driven at 200 lines, whose correction circuit
    /// darkens green for the one value `0x06`, rendering it brown.
    ///
    /// This is not folklore; it is reproduced under control. A test
    /// program of ours that writes `0x06` into palette register *two*
    /// and `0x02` into register *six* renders band two brown and band
    /// six green, so the correction keys off the value and not the
    /// register index. A write of `0x3f` to register six renders white,
    /// which is what rules out the register writes simply being
    /// ignored.
    Period200Line,
}

/// Resolve one six-bit register value to RGB under a display model.
pub const fn resolve_palette_register(value: u8, model: MonitorModel) -> [u8; 3] {
    // The 200-line correction applies to exactly one value.
    if matches!(model, MonitorModel::Period200Line) && value == 0x06 {
        return STOCK_EGA_BROWN;
    }
    // Bits 2..0 are the primary (two thirds) intensities and bits 5..3
    // the secondary (one third) ones, one pair per channel.
    [
        channel_level(value, 0x04, 0x20),
        channel_level(value, 0x02, 0x10),
        channel_level(value, 0x01, 0x08),
    ]
}

const fn channel_level(value: u8, primary: u8, secondary: u8) -> u8 {
    let mut level = 0;
    if value & primary != 0 {
        level += 0xaa;
    }
    if value & secondary != 0 {
        level += 0x55;
    }
    level
}

/// The game's sixteen-entry palette, resolved for [`DISPLAY_MODEL`].
///
/// The v1 baseline is the enhanced-display colour space, so index six renders
/// dark yellow. A frontend may deliberately select the period-monitor model
/// and obtain brown from the same `0x06` register value; changing the register
/// itself to `0x14` would conflate those two layers.
///
/// `formats/tiles.md` section 7 and `systems/display-driver-mode.md`
/// section 5.2 state that the program overwrites the firmware palette
/// from its own table, stock in fifteen entries, substituting `0x06`
/// for the firmware's `0x14` at index six. That is correct.
///
/// Two separate pieces of evidence appeared to contradict it, and both
/// were wrong for different reasons:
///
/// - The specification's own `capture/ultima_000.png` has brown at
///   palette entry six. Its `PLTE` is padded to 256 entries with only
///   0..15 used, and those sixteen are a textbook-exact stock set, so
///   the capture pipeline wrote a canonical palette rather than reading
///   the live registers. It is evidence about a screenshot tool.
/// - Live capture of the shipped game under EGA emulation shows brown
///   and zero dark yellow. That one is real, but it is the *display*,
///   not the register: emulating the same program with a control that
///   writes `0x06` into register two and `0x02` into register six
///   renders band two brown and band six green. The correction keys off
///   the value `0x06`, wherever it is stored. See [`MonitorModel`].
///
/// So both observations are consistent with the register holding
/// `0x06`, and neither is evidence against the spec. The general
/// lesson, which cost two reverts: a value that looks like a known
/// hardware default is evidence of a misread *and* evidence of a
/// deliberate override, and only the shipped bytes tell them apart.
///
/// This is the single v1 palette the whole renderer shares - tiles, text,
/// chrome and every reverse index lookup - so index six lands everywhere at
/// once: bridges, tables, beds, desert, doors, and every other wooden or
/// earthen tone in the game.
///
/// Nothing reprograms the palette after mode setup, the intro included.
/// Anything that looks like recolouring is either a draw under a
/// restricted plane write mask, landing pixels at a different index, or
/// an effect that mutates loaded asset data. Do not add palette-change
/// modelling here.
/// The display model selected for the v1 rendering baseline.
///
/// `display-driver-mode.md §5.2` explicitly chooses the enhanced-display
/// colour space for v1. Period-monitor emulation remains available through
/// [`resolve_palette_register`] but is not the default renderer contract.
pub const DISPLAY_MODEL: MonitorModel = MonitorModel::Enhanced;

pub const EGA_PALETTE_RGB: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xaa],
    [0x00, 0xaa, 0x00],
    [0x00, 0xaa, 0xaa],
    [0xaa, 0x00, 0x00],
    [0xaa, 0x00, 0xaa],
    // Register `0x06`, resolved under the v1 enhanced-display model.
    SHIPPED_PALETTE_DARK_YELLOW,
    [0xaa, 0xaa, 0xaa],
    [0x55, 0x55, 0x55],
    [0x55, 0x55, 0xff],
    [0x55, 0xff, 0x55],
    [0x55, 0xff, 0xff],
    [0xff, 0x55, 0x55],
    [0xff, 0x55, 0xff],
    [0xff, 0xff, 0x55],
    [0xff, 0xff, 0xff],
];

pub const CGA_PALETTE_RGB: [[u8; 3]; 4] = [
    [0x00, 0x00, 0x00],
    [0x55, 0xff, 0xff],
    [0xff, 0x55, 0xff],
    [0xff, 0xff, 0xff],
];

pub const TEXT_PANEL_BG_RGBA: [u8; 4] = [0x00, 0x00, 0x00, 0xff];
pub const TEXT_PANEL_HEADER_RGBA: [u8; 4] = [0x55, 0xff, 0xff, 0xff];
pub const TEXT_PANEL_BODY_RGBA: [u8; 4] = [0xaa, 0xaa, 0xaa, 0xff];
const TEXT_GLYPH_WIDTH: usize = 3;
const TEXT_GLYPH_HEIGHT: usize = 5;
const TEXT_GLYPH_ADVANCE: usize = 4;
const TEXT_LINE_HEIGHT: usize = 6;
pub const TEXT_WINDOW_RENDER_WIDTH: usize = TEXT_SCREEN_COLUMNS as usize * CH_CELL_SIDE;
pub const TEXT_WINDOW_RENDER_HEIGHT: usize = TEXT_SCREEN_ROWS as usize * CH_CELL_SIDE;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedCellFont {
    bytes: Vec<u8>,
}

impl FixedCellFont {
    pub fn glyph_row(&self, code: u8, row: usize) -> Option<u8> {
        if usize::from(code) >= CH_GLYPH_COUNT || row >= CH_CELL_SIDE {
            return None;
        }
        self.bytes
            .get(usize::from(code) * CH_CELL_SIDE + row)
            .copied()
    }
}

pub fn load_ibm_ch_font(game_dir: &std::path::Path) -> io::Result<FixedCellFont> {
    parse_ch_font(&read_disk_file(&game_dir.join(IBM_CH_FILE))?, IBM_CH_FILE)
}

/// `formats/font-ch.md §1`: the rune alphabet shares `IBM.CH`'s
/// code-point order and 8x8 cell geometry; only the file differs. The
/// gameplay sky strip draws its moon phases and hour marker from it.
pub fn load_runes_ch_font(game_dir: &std::path::Path) -> io::Result<FixedCellFont> {
    parse_ch_font(
        &read_disk_file(&game_dir.join(RUNES_CH_FILE))?,
        RUNES_CH_FILE,
    )
}

pub fn parse_ch_font(bytes: &[u8], resource_name: &str) -> io::Result<FixedCellFont> {
    if bytes.len() != CH_FONT_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} fixed font must be {CH_FONT_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    Ok(FixedCellFont {
        bytes: bytes.to_vec(),
    })
}

pub fn render_text_window_rgba(
    system: &TextWindowSystem,
    font: &FixedCellFont,
) -> io::Result<Vec<u8>> {
    render_text_window_rgba_with_runes(system, font, None)
}

/// [`render_text_window_rgba`] with the runic alphabet available.
///
/// A cell whose [`TextCell::runic`] flag is set draws from `runes`
/// instead of `font` (`inventory.md §4.5`: "the renderer switches fonts
/// for that one cell and switches back"). Passing `None` falls back to
/// the text font, which is what the terminal transcripts want.
pub fn render_text_window_rgba_with_runes(
    system: &TextWindowSystem,
    font: &FixedCellFont,
    runes: Option<&FixedCellFont>,
) -> io::Result<Vec<u8>> {
    let pixel_count = TEXT_WINDOW_RENDER_WIDTH
        .checked_mul(TEXT_WINDOW_RENDER_HEIGHT)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "text-window pixel count overflows",
            )
        })?;
    let byte_count = pixel_count.checked_mul(4).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "text-window byte count overflows",
        )
    })?;
    let mut rgba = vec![0; byte_count];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&TEXT_PANEL_BG_RGBA);
    }

    for cell_y in 0..TEXT_SCREEN_ROWS {
        for cell_x in 0..TEXT_SCREEN_COLUMNS {
            let Some(cell) = system.cell(cell_x, cell_y) else {
                continue;
            };
            let foreground = EGA_PALETTE_RGB[usize::from(text_color_foreground(cell.color))];
            let background = EGA_PALETTE_RGB[usize::from(text_color_background(cell.color))];
            for glyph_y in 0..CH_CELL_SIDE {
                let cell_font = if cell.runic {
                    runes.unwrap_or(font)
                } else {
                    font
                };
                let mut row_bits =
                    cell_font
                        .glyph_row(cell.byte & 0x7f, glyph_y)
                        .ok_or_else(|| {
                            io::Error::new(
                                io::ErrorKind::InvalidData,
                                format!(
                                    "fixed font glyph {} is missing row {glyph_y}",
                                    cell.byte & 0x7f
                                ),
                            )
                        })?;
                if cell.underline && glyph_y + 1 == CH_CELL_SIDE {
                    row_bits = 0xff;
                }
                if cell.inverse {
                    row_bits = !row_bits;
                }
                for glyph_x in 0..CH_CELL_SIDE {
                    let color = if row_bits & (1 << (7 - glyph_x)) != 0 {
                        foreground
                    } else {
                        background
                    };
                    let px = usize::from(cell_x) * CH_CELL_SIDE + glyph_x;
                    let py = usize::from(cell_y) * CH_CELL_SIDE + glyph_y;
                    let offset = (py * TEXT_WINDOW_RENDER_WIDTH + px) * 4;
                    rgba[offset..offset + 4].copy_from_slice(&[color[0], color[1], color[2], 0xff]);
                }
            }
        }
    }
    Ok(rgba)
}

pub fn render_text_panel_rgba(text: &str, width: usize, height: usize) -> io::Result<Vec<u8>> {
    let pixel_count = width.checked_mul(height).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "text panel pixel count overflows",
        )
    })?;
    let byte_count = pixel_count.checked_mul(4).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "text panel byte count overflows",
        )
    })?;
    let mut bytes = vec![0; byte_count];
    for pixel in bytes.chunks_exact_mut(4) {
        pixel.copy_from_slice(&TEXT_PANEL_BG_RGBA);
    }

    let max_cols = width / TEXT_GLYPH_ADVANCE;
    let max_lines = height / TEXT_LINE_HEIGHT;
    let lines = wrap_text_panel_lines(text, max_cols, max_lines);
    for (line_index, line) in lines.iter().enumerate() {
        let color = if line_index == 0 {
            TEXT_PANEL_HEADER_RGBA
        } else {
            TEXT_PANEL_BODY_RGBA
        };
        let y = line_index * TEXT_LINE_HEIGHT;
        for (col, ch) in line.chars().take(max_cols).enumerate() {
            draw_text_panel_glyph(&mut bytes, width, col * TEXT_GLYPH_ADVANCE, y, ch, color);
        }
    }
    Ok(bytes)
}

pub fn wrap_text_panel_lines(text: &str, max_cols: usize, max_lines: usize) -> Vec<String> {
    if max_cols == 0 || max_lines == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    for raw in text.lines() {
        if lines.len() >= max_lines {
            break;
        }
        let normalized = raw.trim().to_ascii_uppercase();
        if normalized.is_empty() {
            lines.push(String::new());
            continue;
        }
        let mut current = String::new();
        for word in normalized.split_whitespace() {
            let mut pending = word;
            while pending.len() > max_cols {
                if !current.is_empty() {
                    lines.push(current);
                    current = String::new();
                    if lines.len() >= max_lines {
                        return lines;
                    }
                }
                lines.push(pending[..max_cols].to_string());
                pending = &pending[max_cols..];
                if lines.len() >= max_lines {
                    return lines;
                }
            }
            let separator = usize::from(!current.is_empty());
            if current.len() + separator + pending.len() > max_cols {
                lines.push(current);
                current = String::new();
                if lines.len() >= max_lines {
                    return lines;
                }
            }
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(pending);
        }
        if lines.len() >= max_lines {
            break;
        }
        lines.push(current);
    }
    lines
}

fn draw_text_panel_glyph(
    bytes: &mut [u8],
    width: usize,
    x: usize,
    y: usize,
    ch: char,
    color: [u8; 4],
) {
    if ch == ' ' {
        return;
    }
    for (row, bits) in compact_text_panel_glyph(ch).iter().enumerate() {
        for col in 0..TEXT_GLYPH_WIDTH {
            if bits & (1 << (TEXT_GLYPH_WIDTH - 1 - col)) == 0 {
                continue;
            }
            let px = x + col;
            let py = y + row;
            let offset = (py * width + px) * 4;
            if let Some(pixel) = bytes.get_mut(offset..offset + 4) {
                pixel.copy_from_slice(&color);
            }
        }
    }
}

fn compact_text_panel_glyph(ch: char) -> [u8; TEXT_GLYPH_HEIGHT] {
    match ch {
        'A' => [0b010, 0b101, 0b111, 0b101, 0b101],
        'B' => [0b110, 0b101, 0b110, 0b101, 0b110],
        'C' => [0b011, 0b100, 0b100, 0b100, 0b011],
        'D' => [0b110, 0b101, 0b101, 0b101, 0b110],
        'E' => [0b111, 0b100, 0b110, 0b100, 0b111],
        'F' => [0b111, 0b100, 0b110, 0b100, 0b100],
        'G' => [0b011, 0b100, 0b101, 0b101, 0b011],
        'H' => [0b101, 0b101, 0b111, 0b101, 0b101],
        'I' => [0b111, 0b010, 0b010, 0b010, 0b111],
        'J' => [0b001, 0b001, 0b001, 0b101, 0b010],
        'K' => [0b101, 0b101, 0b110, 0b101, 0b101],
        'L' => [0b100, 0b100, 0b100, 0b100, 0b111],
        'M' => [0b101, 0b111, 0b111, 0b101, 0b101],
        'N' => [0b101, 0b111, 0b111, 0b111, 0b101],
        'O' => [0b010, 0b101, 0b101, 0b101, 0b010],
        'P' => [0b110, 0b101, 0b110, 0b100, 0b100],
        'Q' => [0b010, 0b101, 0b101, 0b111, 0b011],
        'R' => [0b110, 0b101, 0b110, 0b101, 0b101],
        'S' => [0b011, 0b100, 0b010, 0b001, 0b110],
        'T' => [0b111, 0b010, 0b010, 0b010, 0b010],
        'U' => [0b101, 0b101, 0b101, 0b101, 0b111],
        'V' => [0b101, 0b101, 0b101, 0b101, 0b010],
        'W' => [0b101, 0b101, 0b111, 0b111, 0b101],
        'X' => [0b101, 0b101, 0b010, 0b101, 0b101],
        'Y' => [0b101, 0b101, 0b010, 0b010, 0b010],
        'Z' => [0b111, 0b001, 0b010, 0b100, 0b111],
        '0' => [0b111, 0b101, 0b101, 0b101, 0b111],
        '1' => [0b010, 0b110, 0b010, 0b010, 0b111],
        '2' => [0b110, 0b001, 0b010, 0b100, 0b111],
        '3' => [0b110, 0b001, 0b010, 0b001, 0b110],
        '4' => [0b101, 0b101, 0b111, 0b001, 0b001],
        '5' => [0b111, 0b100, 0b110, 0b001, 0b110],
        '6' => [0b011, 0b100, 0b110, 0b101, 0b010],
        '7' => [0b111, 0b001, 0b010, 0b010, 0b010],
        '8' => [0b010, 0b101, 0b010, 0b101, 0b010],
        '9' => [0b010, 0b101, 0b011, 0b001, 0b110],
        ':' => [0b000, 0b010, 0b000, 0b010, 0b000],
        ';' => [0b000, 0b010, 0b000, 0b010, 0b100],
        '.' => [0b000, 0b000, 0b000, 0b000, 0b010],
        ',' => [0b000, 0b000, 0b000, 0b010, 0b100],
        '-' => [0b000, 0b000, 0b111, 0b000, 0b000],
        '_' => [0b000, 0b000, 0b000, 0b000, 0b111],
        '/' => [0b001, 0b001, 0b010, 0b100, 0b100],
        '\\' => [0b100, 0b100, 0b010, 0b001, 0b001],
        '(' => [0b001, 0b010, 0b010, 0b010, 0b001],
        ')' => [0b100, 0b010, 0b010, 0b010, 0b100],
        '[' => [0b011, 0b010, 0b010, 0b010, 0b011],
        ']' => [0b110, 0b010, 0b010, 0b010, 0b110],
        '<' => [0b001, 0b010, 0b100, 0b010, 0b001],
        '>' => [0b100, 0b010, 0b001, 0b010, 0b100],
        '@' => [0b111, 0b101, 0b111, 0b100, 0b011],
        '&' => [0b010, 0b101, 0b010, 0b101, 0b011],
        '\'' => [0b010, 0b010, 0b000, 0b000, 0b000],
        '"' => [0b101, 0b101, 0b000, 0b000, 0b000],
        '!' => [0b010, 0b010, 0b010, 0b000, 0b010],
        '?' => [0b110, 0b001, 0b010, 0b000, 0b010],
        _ => [0b111, 0b001, 0b010, 0b000, 0b010],
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopDownRenderArea {
    Town,
    World(WorldPlane),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicImageDirectory {
    pub depth: TileGraphicsDepth,
    pub images: Vec<Option<GraphicImage>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicSprite {
    pub image: GraphicImage,
    pub transparent_mask: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicSpriteSheet {
    pub depth: TileGraphicsDepth,
    pub sprites: Vec<Option<GraphicSprite>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonochromeBitmap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

impl MonochromeBitmap {
    pub fn pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels.get(y * self.width + x).copied()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TitleBitImages {
    pub blocks: Vec<MonochromeBitmap>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextCellStyle {
    pub underline: bool,
    pub inverse: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedFont {
    pub cell_width: usize,
    pub cell_height: usize,
    pub glyphs: Vec<MonochromeBitmap>,
}

impl FixedFont {
    pub fn glyph(&self, code: u8) -> Option<&MonochromeBitmap> {
        self.glyphs.get(code as usize)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalGlyph {
    pub advance_width: u8,
    pub bitmap: MonochromeBitmap,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalFontResource {
    pub strips: Vec<MonochromeBitmap>,
}

impl ProportionalFontResource {
    pub fn strip(&self, slot: usize) -> Option<&MonochromeBitmap> {
        self.strips.get(slot)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalFont {
    pub first_code: u8,
    pub glyphs: Vec<ProportionalGlyph>,
}

impl ProportionalFont {
    pub fn glyph_for_code(&self, code: u8) -> Option<&ProportionalGlyph> {
        code.checked_sub(self.first_code)
            .and_then(|slot| self.glyphs.get(slot as usize))
    }
}

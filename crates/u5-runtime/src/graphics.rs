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

pub const EGA_PALETTE_RGB: [[u8; 3]; 16] = [
    [0x00, 0x00, 0x00],
    [0x00, 0x00, 0xaa],
    [0x00, 0xaa, 0x00],
    [0x00, 0xaa, 0xaa],
    [0xaa, 0x00, 0x00],
    [0xaa, 0x00, 0xaa],
    [0xaa, 0x55, 0x00],
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
                let mut row_bits = font.glyph_row(cell.byte & 0x7f, glyph_y).ok_or_else(|| {
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

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicSprite {
    pub image: GraphicImage,
    pub transparent_mask: Vec<u8>,
}

#[cfg(test)]
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

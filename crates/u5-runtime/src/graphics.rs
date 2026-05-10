//! Tile atlas, viewport, palettes, and image/font formats (TileGraphicsDepth, TileAtlas, TileViewport, GraphicImage*, MonochromeBitmap, TitleBitImages, FixedFont, ProportionalFont).

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileGraphicsDepth {
    Ega16,
    Cga4,
}

impl TileGraphicsDepth {
    pub fn from_key(value: &str) -> io::Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "e" | "ega" | "ega16" | "16" => Ok(Self::Ega16),
            "c" | "cga" | "cga4" | "4" => Ok(Self::Cga4),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("raster depth must be ega or cga, got `{value}`"),
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

    #[cfg(test)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TopDownRenderArea {
    Town,
    World(WorldPlane),
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicImage {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[cfg(test)]
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

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonochromeBitmap {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u8>,
}

#[cfg(test)]
impl MonochromeBitmap {
    pub fn pixel(&self, x: usize, y: usize) -> Option<u8> {
        if x >= self.width || y >= self.height {
            return None;
        }
        self.pixels.get(y * self.width + x).copied()
    }
}

#[cfg(test)]
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

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FixedFont {
    pub cell_width: usize,
    pub cell_height: usize,
    pub glyphs: Vec<MonochromeBitmap>,
}

#[cfg(test)]
impl FixedFont {
    pub fn glyph(&self, code: u8) -> Option<&MonochromeBitmap> {
        self.glyphs.get(code as usize)
    }
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalGlyph {
    pub advance_width: u8,
    pub bitmap: MonochromeBitmap,
}

#[cfg(test)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalFont {
    pub first_code: u8,
    pub glyphs: Vec<ProportionalGlyph>,
}

#[cfg(test)]
impl ProportionalFont {
    pub fn glyph_for_code(&self, code: u8) -> Option<&ProportionalGlyph> {
        code.checked_sub(self.first_code)
            .and_then(|slot| self.glyphs.get(slot as usize))
    }
}

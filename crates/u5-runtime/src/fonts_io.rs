//! Loaders/parsers for fixed and proportional fonts plus monochrome bitmaps (BIT/CH/HCS/PCS).

use std::{io, path::Path};

use crate::*;

pub fn load_title_bit(game_dir: &Path) -> io::Result<TitleBitImages> {
    parse_title_bit(&read_disk_file(&game_dir.join(TITLE_BIT_FILE))?)
}

pub fn parse_title_bit(bytes: &[u8]) -> io::Result<TitleBitImages> {
    parse_sparse_bit_images(bytes, TITLE_BIT_FILE).or_else(|raw_err| {
        let body = decode_lzw_envelope(bytes, TITLE_BIT_FILE).map_err(|lzw_err| {
            io::Error::new(
                lzw_err.kind(),
                format!(
                    "{TITLE_BIT_FILE} is neither a sparse strip resource ({raw_err}) nor a legacy LZW-wrapped bitmap directory ({lzw_err})"
                ),
            )
        })?;
        parse_title_bit_body(&body, TITLE_BIT_FILE)
    })
}

pub fn parse_sparse_bit_images(bytes: &[u8], resource_name: &str) -> io::Result<TitleBitImages> {
    Ok(TitleBitImages {
        blocks: parse_sparse_strip_resource(bytes, resource_name)?,
    })
}

pub fn parse_title_bit_body(body: &[u8], resource_name: &str) -> io::Result<TitleBitImages> {
    if body.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap directory is shorter than its count word"),
        ));
    }
    let count = u16_at(body, 0) as usize;
    let header_len = 2usize
        .checked_add(count.checked_mul(2).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} bitmap directory count overflows"),
            )
        })?)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} bitmap directory header overflows"),
            )
        })?;
    if header_len > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap directory header exceeds body length"),
        ));
    }

    let mut blocks = Vec::with_capacity(count);
    for slot in 0..count {
        let offset = u16_at(body, 2 + slot * 2) as usize;
        if offset == 0 || offset < header_len || offset >= body.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} bitmap slot {slot} has invalid offset {offset}"),
            ));
        }
        blocks.push(parse_monochrome_bitmap_block(body, offset, resource_name)?);
    }
    Ok(TitleBitImages { blocks })
}

pub fn load_british_bit(game_dir: &Path) -> io::Result<MonochromeBitmap> {
    parse_british_bit(&read_disk_file(&game_dir.join(BRITISH_BIT_FILE))?)
}

pub fn parse_british_bit(bytes: &[u8]) -> io::Result<MonochromeBitmap> {
    parse_single_sparse_bit_image(bytes, BRITISH_BIT_FILE).or_else(|raw_err| {
        let body = decode_lzw_envelope(bytes, BRITISH_BIT_FILE).map_err(|lzw_err| {
            io::Error::new(
                lzw_err.kind(),
                format!(
                    "{BRITISH_BIT_FILE} is neither a sparse strip resource ({raw_err}) nor a legacy LZW-wrapped bitmap ({lzw_err})"
                ),
            )
        })?;
        parse_single_image_bit_body(&body, BRITISH_BIT_FILE)
    })
}

pub fn load_wd_bit(game_dir: &Path) -> io::Result<MonochromeBitmap> {
    parse_wd_bit(&read_disk_file(&game_dir.join(WD_BIT_FILE))?)
}

pub fn parse_wd_bit(bytes: &[u8]) -> io::Result<MonochromeBitmap> {
    parse_single_sparse_bit_image(bytes, WD_BIT_FILE)
}

pub fn parse_single_sparse_bit_image(
    bytes: &[u8],
    resource_name: &str,
) -> io::Result<MonochromeBitmap> {
    let mut strips = parse_sparse_strip_resource(bytes, resource_name)?;
    if strips.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} sparse strip resource must contain exactly one populated strip, got {}",
                strips.len()
            ),
        ));
    }
    Ok(strips.remove(0))
}

pub fn parse_sparse_strip_resource(
    bytes: &[u8],
    resource_name: &str,
) -> io::Result<Vec<MonochromeBitmap>> {
    if bytes.len() < BIT_ENTRY_COUNT_WORD_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sparse strip resource is shorter than its count word"),
        ));
    }
    let entry_count = u16_at(bytes, 0) as usize;
    let mut strips = Vec::new();
    for slot in 0..entry_count {
        let entry_offset = BIT_ENTRY_COUNT_WORD_LEN
            .checked_add(
                slot.checked_mul(BIT_POINTER_TABLE_ENTRY_LEN)
                    .ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!(
                                "{resource_name} sparse strip table slot {slot} offset overflows"
                            ),
                        )
                    })?,
            )
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{resource_name} sparse strip table slot {slot} offset overflows"),
                )
            })?;
        if entry_offset >= bytes.len() {
            break;
        }
        if entry_offset + BIT_POINTER_TABLE_ENTRY_LEN > bytes.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sparse strip table slot {slot} is truncated"),
            ));
        }
        let pointer = u16_at(bytes, entry_offset);
        if pointer == BIT_STRIP_POINTER_NONE {
            continue;
        }
        strips.push(parse_sparse_strip_body(
            bytes,
            pointer as usize,
            resource_name,
            slot,
        )?);
    }
    if strips.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sparse strip resource has no populated strips"),
        ));
    }
    Ok(strips)
}

pub fn parse_sparse_strip_body(
    bytes: &[u8],
    offset: usize,
    resource_name: &str,
    slot: usize,
) -> io::Result<MonochromeBitmap> {
    if offset + BIT_STRIP_HEADER_LEN > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sparse strip slot {slot} header at {offset} is truncated"),
        ));
    }
    parse_monochrome_bitmap_payload(bytes, offset, resource_name)
}

pub fn parse_single_image_bit_body(
    body: &[u8],
    resource_name: &str,
) -> io::Result<MonochromeBitmap> {
    if body.len() < 8 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} single-image bitmap is shorter than its header"),
        ));
    }
    let format_marker = u16_at(body, 0);
    let mode_marker = u16_at(body, 2);
    if format_marker != SINGLE_IMAGE_BIT_FORMAT_MARKER
        || mode_marker != SINGLE_IMAGE_BIT_MODE_MARKER
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} bitmap markers must be {SINGLE_IMAGE_BIT_FORMAT_MARKER}/{SINGLE_IMAGE_BIT_MODE_MARKER}, got {format_marker}/{mode_marker}"
            ),
        ));
    }
    let bitmap = parse_monochrome_bitmap_payload(body, 4, resource_name)?;
    let expected_len = 8usize
        .checked_add(monochrome_bitmap_payload_len(bitmap.width, bitmap.height)?)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} single-image bitmap length overflows"),
            )
        })?;
    if body.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} single-image bitmap must be {expected_len} bytes, got {}",
                body.len()
            ),
        ));
    }
    Ok(bitmap)
}

pub fn parse_monochrome_bitmap_block(
    body: &[u8],
    offset: usize,
    resource_name: &str,
) -> io::Result<MonochromeBitmap> {
    if offset + 4 > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap block at {offset} is shorter than its header"),
        ));
    }
    parse_monochrome_bitmap_payload(body, offset, resource_name)
}

pub fn parse_monochrome_bitmap_payload(
    body: &[u8],
    offset: usize,
    resource_name: &str,
) -> io::Result<MonochromeBitmap> {
    let width = u16_at(body, offset) as usize;
    let height = u16_at(body, offset + 2) as usize;
    if width == 0 || height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap at {offset} has zero dimensions"),
        ));
    }
    let pixel_count = width.checked_mul(height).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap at {offset} pixel count overflows"),
        )
    })?;
    let payload_len = monochrome_bitmap_payload_len(width, height)?;
    let payload_start = offset.checked_add(4).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap payload offset overflows"),
        )
    })?;
    let payload_end = payload_start.checked_add(payload_len).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap payload end overflows"),
        )
    })?;
    if payload_end > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} bitmap at {offset} exceeds body length"),
        ));
    }
    Ok(MonochromeBitmap {
        width,
        height,
        pixels: unpack_monochrome_bits(&body[payload_start..payload_end], pixel_count),
    })
}

pub fn monochrome_bitmap_payload_len(width: usize, height: usize) -> io::Result<usize> {
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_add(7))
        .map(|bits| bits / 8)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "monochrome bitmap payload length overflows",
            )
        })
}

pub fn unpack_monochrome_bits(bytes: &[u8], pixel_count: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(pixel_count);
    for pixel in 0..pixel_count {
        let byte = bytes[pixel / 8];
        pixels.push((byte >> (7 - (pixel % 8))) & 1);
    }
    pixels
}

pub fn load_ch_font(game_dir: &Path, file_name: &str) -> io::Result<FixedFont> {
    parse_fixed_font_body(
        &read_disk_file(&game_dir.join(file_name))?,
        file_name,
        CH_FONT_CELL_WIDTH,
        CH_FONT_CELL_HEIGHT,
    )
}

pub fn load_hcs_font(game_dir: &Path, file_name: &str) -> io::Result<FixedFont> {
    parse_fixed_font_body(
        &read_disk_file(&game_dir.join(file_name))?,
        file_name,
        HCS_FONT_CELL_WIDTH,
        HCS_FONT_CELL_HEIGHT,
    )
}

pub fn parse_fixed_font_body(
    body: &[u8],
    resource_name: &str,
    cell_width: usize,
    cell_height: usize,
) -> io::Result<FixedFont> {
    if cell_width == 0 || cell_height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} fixed font has zero cell dimension"),
        ));
    }
    let row_stride = monochrome_row_stride(cell_width)?;
    let glyph_len = row_stride.checked_mul(cell_height).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} fixed font glyph length overflows"),
        )
    })?;
    let expected_len = glyph_len
        .checked_mul(FIXED_FONT_GLYPH_COUNT)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} fixed font length overflows"),
            )
        })?;
    if body.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} fixed font must be {expected_len} bytes, got {}",
                body.len()
            ),
        ));
    }

    let pixel_count = cell_width.checked_mul(cell_height).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} fixed font cell pixel count overflows"),
        )
    })?;
    let mut glyphs = Vec::with_capacity(FIXED_FONT_GLYPH_COUNT);
    for slot in 0..FIXED_FONT_GLYPH_COUNT {
        let start = slot * glyph_len;
        glyphs.push(MonochromeBitmap {
            width: cell_width,
            height: cell_height,
            pixels: unpack_monochrome_rows(
                &body[start..start + glyph_len],
                cell_width,
                cell_height,
                row_stride,
                pixel_count,
            ),
        });
    }
    Ok(FixedFont {
        cell_width,
        cell_height,
        glyphs,
    })
}

pub fn load_proportional_font(game_dir: &Path) -> io::Result<ProportionalFont> {
    parse_proportional_font(&read_disk_file(&game_dir.join(PROPORT_PCS_FILE))?)
}

pub fn parse_proportional_font(bytes: &[u8]) -> io::Result<ProportionalFont> {
    let body = decode_lzw_envelope(bytes, PROPORT_PCS_FILE)?;
    parse_proportional_font_body(&body, PROPORT_PCS_FILE)
}

pub fn load_proportional_font_resource(game_dir: &Path) -> io::Result<ProportionalFontResource> {
    parse_proportional_font_resource(&read_disk_file(&game_dir.join(PROPORT_PCS_FILE))?)
}

pub fn parse_proportional_font_resource(bytes: &[u8]) -> io::Result<ProportionalFontResource> {
    parse_sparse_proportional_font_resource(bytes).or_else(|sparse_err| {
        let body = decode_lzw_envelope(bytes, PROPORT_PCS_FILE).map_err(|lzw_err| {
            io::Error::new(
                lzw_err.kind(),
                format!(
                    "{PROPORT_PCS_FILE} is neither a sparse strip resource ({sparse_err}) nor a legacy LZW-wrapped proportional font ({lzw_err})"
                ),
            )
        })?;
        parse_sparse_proportional_font_resource(&body).or_else(|body_sparse_err| {
            parse_proportional_font_body(&body, PROPORT_PCS_FILE)
                .map(legacy_proportional_font_as_resource)
                .map_err(|legacy_err| {
                    io::Error::new(
                        legacy_err.kind(),
                        format!(
                            "{PROPORT_PCS_FILE} decoded body is neither a sparse strip resource ({body_sparse_err}) nor the legacy glyph-directory shape ({legacy_err})"
                        ),
                    )
                })
        })
    })
}

pub fn parse_sparse_proportional_font_resource(
    bytes: &[u8],
) -> io::Result<ProportionalFontResource> {
    Ok(ProportionalFontResource {
        strips: parse_sparse_strip_resource(bytes, PROPORT_PCS_FILE)?,
    })
}

fn legacy_proportional_font_as_resource(font: ProportionalFont) -> ProportionalFontResource {
    ProportionalFontResource {
        strips: font.glyphs.into_iter().map(|glyph| glyph.bitmap).collect(),
    }
}

pub fn parse_proportional_font_body(
    body: &[u8],
    resource_name: &str,
) -> io::Result<ProportionalFont> {
    if body.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} proportional font is shorter than its count word"),
        ));
    }
    let count = u16_at(body, 0) as usize;
    if count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} proportional font has no glyphs"),
        ));
    }
    let header_len = 2usize
        .checked_add(count.checked_mul(2).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} proportional font count overflows"),
            )
        })?)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} proportional font header overflows"),
            )
        })?;
    if header_len > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} proportional font header exceeds body length"),
        ));
    }

    let mut glyphs = Vec::with_capacity(count);
    for slot in 0..count {
        let offset = u16_at(body, 2 + slot * 2) as usize;
        let end = offset.checked_add(PCS_GLYPH_BLOCK_LEN).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} glyph slot {slot} length overflows"),
            )
        })?;
        if offset < header_len || end > body.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} glyph slot {slot} has invalid offset {offset}"),
            ));
        }
        let advance_width = body[offset];
        if advance_width as usize > PCS_GLYPH_BITMAP_WIDTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{resource_name} glyph slot {slot} advance width {advance_width} exceeds bitmap width"
                ),
            ));
        }
        glyphs.push(ProportionalGlyph {
            advance_width,
            bitmap: MonochromeBitmap {
                width: PCS_GLYPH_BITMAP_WIDTH,
                height: PCS_GLYPH_HEIGHT,
                pixels: unpack_monochrome_rows(
                    &body[offset + 1..end],
                    PCS_GLYPH_BITMAP_WIDTH,
                    PCS_GLYPH_HEIGHT,
                    1,
                    PCS_GLYPH_BITMAP_WIDTH * PCS_GLYPH_HEIGHT,
                ),
            },
        });
    }
    Ok(ProportionalFont {
        first_code: PCS_FIRST_CODE,
        glyphs,
    })
}

pub fn monochrome_row_stride(width: usize) -> io::Result<usize> {
    width.checked_add(7).map(|bits| bits / 8).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "monochrome row stride overflows",
        )
    })
}

pub fn unpack_monochrome_rows(
    bytes: &[u8],
    width: usize,
    height: usize,
    row_stride: usize,
    pixel_count: usize,
) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(pixel_count);
    for row in 0..height {
        let row_bytes = &bytes[row * row_stride..(row + 1) * row_stride];
        for x in 0..width {
            let byte = row_bytes[x / 8];
            pixels.push((byte >> (7 - (x % 8))) & 1);
        }
    }
    pixels
}

#[cfg(test)]
pub fn prepare_fixed_text_cell(
    font: &FixedFont,
    code: u8,
    style: TextCellStyle,
) -> io::Result<MonochromeBitmap> {
    if code as usize >= FIXED_FONT_GLYPH_COUNT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("fixed font code {code} is outside 0..127"),
        ));
    }
    let mut glyph = font
        .glyph(code)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("fixed font code {code} is missing"),
            )
        })?
        .clone();
    if style.underline && glyph.height > 0 {
        let row_start = (glyph.height - 1) * glyph.width;
        for pixel in &mut glyph.pixels[row_start..row_start + glyph.width] {
            *pixel = 1;
        }
    }
    if style.inverse {
        for pixel in &mut glyph.pixels {
            *pixel ^= 1;
        }
    }
    Ok(glyph)
}

#[cfg(test)]
pub fn rasterize_fixed_text_line(
    font: &FixedFont,
    text: &[u8],
    style: TextCellStyle,
) -> io::Result<MonochromeBitmap> {
    let width = font.cell_width.checked_mul(text.len()).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "fixed text line width overflows",
        )
    })?;
    let pixel_count = width.checked_mul(font.cell_height).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "fixed text line pixel count overflows",
        )
    })?;
    let mut pixels = vec![0; pixel_count];
    for (cell, code) in text.iter().enumerate() {
        let glyph = prepare_fixed_text_cell(font, *code, style)?;
        blit_monochrome_bitmap(
            &mut pixels,
            width,
            font.cell_height,
            &glyph,
            cell * font.cell_width,
            0,
            glyph.width,
        );
    }
    Ok(MonochromeBitmap {
        width,
        height: font.cell_height,
        pixels,
    })
}

pub fn measure_proportional_text(font: &ProportionalFont, text: &[u8]) -> io::Result<usize> {
    let mut width = 0usize;
    for code in text {
        let glyph = font.glyph_for_code(*code).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("proportional font code {code} is outside the loaded range"),
            )
        })?;
        width = width
            .checked_add(glyph.advance_width as usize)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "proportional text width overflows",
                )
            })?;
    }
    Ok(width)
}

pub const PROPORTIONAL_WIDTH_TABLE_LEN: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalWidthTable {
    pub widths: [u8; PROPORTIONAL_WIDTH_TABLE_LEN],
}

impl ProportionalWidthTable {
    pub const fn new(widths: [u8; PROPORTIONAL_WIDTH_TABLE_LEN]) -> Self {
        Self { widths }
    }

    pub fn from_font_advances(font: &ProportionalFont) -> Self {
        let mut widths = [0u8; PROPORTIONAL_WIDTH_TABLE_LEN];
        for (slot, glyph) in font.glyphs.iter().enumerate() {
            let Some(code) = usize::from(font.first_code).checked_add(slot) else {
                break;
            };
            if code >= widths.len() {
                break;
            }
            widths[code] = glyph.advance_width;
        }
        Self { widths }
    }

    pub fn width_for_byte(&self, code: u8) -> io::Result<usize> {
        self.widths
            .get(usize::from(code))
            .copied()
            .map(usize::from)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("proportional width-table code {code} is outside 0..127"),
                )
            })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProportionalParagraphLine {
    pub bytes: Vec<u8>,
    pub width: usize,
    pub hard_break: bool,
}

pub fn measure_proportional_text_with_widths(
    widths: &ProportionalWidthTable,
    text: &[u8],
) -> io::Result<usize> {
    let mut width = 0usize;
    for code in text {
        width = width
            .checked_add(widths.width_for_byte(*code)?)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "proportional text width overflows",
                )
            })?;
    }
    Ok(width)
}

pub fn layout_proportional_paragraph(
    widths: &ProportionalWidthTable,
    text: &[u8],
    max_width: usize,
) -> io::Result<Vec<ProportionalParagraphLine>> {
    if max_width == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "proportional paragraph width must be nonzero",
        ));
    }

    let mut lines = Vec::new();
    let mut line = Vec::new();
    let mut line_width = 0usize;
    let mut word = Vec::new();
    let mut pending_space = false;

    for byte in text {
        match paragraph_byte_kind(*byte) {
            ParagraphByteKind::EndOfStream => break,
            ParagraphByteKind::Glyph | ParagraphByteKind::SoftHyphen => word.push(*byte),
            ParagraphByteKind::SpaceBreak => {
                append_proportional_word(
                    widths,
                    max_width,
                    &mut lines,
                    &mut line,
                    &mut line_width,
                    &word,
                    pending_space,
                )?;
                word.clear();
                pending_space = !line.is_empty();
            }
            ParagraphByteKind::HardBreak => {
                append_proportional_word(
                    widths,
                    max_width,
                    &mut lines,
                    &mut line,
                    &mut line_width,
                    &word,
                    pending_space,
                )?;
                word.clear();
                push_proportional_line(&mut lines, &mut line, &mut line_width, true);
                pending_space = false;
            }
            ParagraphByteKind::PageMarker => {
                append_proportional_word(
                    widths,
                    max_width,
                    &mut lines,
                    &mut line,
                    &mut line_width,
                    &word,
                    pending_space,
                )?;
                word.clear();
                if !line.is_empty() {
                    push_proportional_line(&mut lines, &mut line, &mut line_width, true);
                }
                pending_space = false;
            }
        }
    }

    append_proportional_word(
        widths,
        max_width,
        &mut lines,
        &mut line,
        &mut line_width,
        &word,
        pending_space,
    )?;
    if !line.is_empty() {
        push_proportional_line(&mut lines, &mut line, &mut line_width, false);
    }
    Ok(lines)
}

pub fn rasterize_proportional_text_line_with_widths(
    font: &ProportionalFont,
    widths: &ProportionalWidthTable,
    text: &[u8],
) -> io::Result<MonochromeBitmap> {
    let width = measure_proportional_text_with_widths(widths, text)?;
    let pixel_count = width.checked_mul(PCS_GLYPH_HEIGHT).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "proportional text pixel count overflows",
        )
    })?;
    let mut pixels = vec![0; pixel_count];
    let mut cursor_x = 0usize;
    for code in text {
        let glyph = font.glyph_for_code(*code).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("proportional font code {code} is outside the loaded range"),
            )
        })?;
        let advance = widths.width_for_byte(*code)?;
        if advance > 0 {
            blit_monochrome_bitmap(
                &mut pixels,
                width,
                PCS_GLYPH_HEIGHT,
                &glyph.bitmap,
                cursor_x,
                0,
                advance.min(glyph.bitmap.width),
            );
        }
        cursor_x = cursor_x.checked_add(advance).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "proportional text cursor overflows",
            )
        })?;
    }
    Ok(MonochromeBitmap {
        width,
        height: PCS_GLYPH_HEIGHT,
        pixels,
    })
}

pub fn rasterize_proportional_paragraph(
    font: &ProportionalFont,
    widths: &ProportionalWidthTable,
    text: &[u8],
    max_width: usize,
    line_height: usize,
) -> io::Result<MonochromeBitmap> {
    if line_height < PCS_GLYPH_HEIGHT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "proportional paragraph line height must fit glyph height",
        ));
    }
    let lines = layout_proportional_paragraph(widths, text, max_width)?;
    let height = line_height
        .checked_mul(lines.len().max(1))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "paragraph height overflows"))?;
    let pixel_count = max_width
        .checked_mul(height)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "paragraph pixels overflow"))?;
    let mut pixels = vec![0; pixel_count];
    for (row, line) in lines.iter().enumerate() {
        let bitmap = rasterize_proportional_text_line_with_widths(font, widths, &line.bytes)?;
        blit_monochrome_bitmap(
            &mut pixels,
            max_width,
            height,
            &bitmap,
            0,
            row * line_height,
            bitmap.width,
        );
    }
    Ok(MonochromeBitmap {
        width: max_width,
        height,
        pixels,
    })
}

fn append_proportional_word(
    widths: &ProportionalWidthTable,
    max_width: usize,
    lines: &mut Vec<ProportionalParagraphLine>,
    line: &mut Vec<u8>,
    line_width: &mut usize,
    word: &[u8],
    pending_space: bool,
) -> io::Result<()> {
    if word.is_empty() {
        return Ok(());
    }
    let visible = word
        .iter()
        .copied()
        .filter(|byte| !matches!(paragraph_byte_kind(*byte), ParagraphByteKind::SoftHyphen))
        .collect::<Vec<_>>();
    if visible.is_empty() {
        return Ok(());
    }

    let space_width = if pending_space && !line.is_empty() {
        widths.width_for_byte(b' ')?
    } else {
        0
    };
    let visible_width = measure_proportional_text_with_widths(widths, &visible)?;
    if !line.is_empty()
        && line_width
            .checked_add(space_width)
            .and_then(|value| value.checked_add(visible_width))
            .is_some_and(|width| width <= max_width)
    {
        if space_width > 0 {
            line.push(b' ');
            *line_width += space_width;
        }
        append_proportional_bytes(widths, line, line_width, &visible)?;
        return Ok(());
    }
    if line.is_empty() && visible_width <= max_width {
        append_proportional_bytes(widths, line, line_width, &visible)?;
        return Ok(());
    }
    if !line.is_empty() {
        push_proportional_line(lines, line, line_width, false);
    }

    let mut segment = Vec::new();
    for byte in word {
        if matches!(paragraph_byte_kind(*byte), ParagraphByteKind::SoftHyphen) {
            append_proportional_word_segment(widths, max_width, lines, line, line_width, &segment)?;
            segment.clear();
        } else {
            segment.push(*byte);
        }
    }
    append_proportional_word_segment(widths, max_width, lines, line, line_width, &segment)
}

fn append_proportional_word_segment(
    widths: &ProportionalWidthTable,
    max_width: usize,
    lines: &mut Vec<ProportionalParagraphLine>,
    line: &mut Vec<u8>,
    line_width: &mut usize,
    segment: &[u8],
) -> io::Result<()> {
    if segment.is_empty() {
        return Ok(());
    }
    let segment_width = measure_proportional_text_with_widths(widths, segment)?;
    if !line.is_empty()
        && line_width
            .checked_add(segment_width)
            .is_some_and(|width| width > max_width)
    {
        push_proportional_line(lines, line, line_width, false);
    }
    if segment_width <= max_width {
        append_proportional_bytes(widths, line, line_width, segment)?;
        return Ok(());
    }
    for byte in segment {
        let width = widths.width_for_byte(*byte)?;
        if !line.is_empty()
            && line_width
                .checked_add(width)
                .is_some_and(|value| value > max_width)
        {
            push_proportional_line(lines, line, line_width, false);
        }
        append_proportional_bytes(widths, line, line_width, &[*byte])?;
    }
    Ok(())
}

fn append_proportional_bytes(
    widths: &ProportionalWidthTable,
    line: &mut Vec<u8>,
    line_width: &mut usize,
    bytes: &[u8],
) -> io::Result<()> {
    for byte in bytes {
        line.push(*byte);
        *line_width = line_width
            .checked_add(widths.width_for_byte(*byte)?)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "proportional line width overflows",
                )
            })?;
    }
    Ok(())
}

fn push_proportional_line(
    lines: &mut Vec<ProportionalParagraphLine>,
    line: &mut Vec<u8>,
    line_width: &mut usize,
    hard_break: bool,
) {
    lines.push(ProportionalParagraphLine {
        bytes: std::mem::take(line),
        width: *line_width,
        hard_break,
    });
    *line_width = 0;
}

pub fn rasterize_proportional_text_line(
    font: &ProportionalFont,
    text: &[u8],
) -> io::Result<MonochromeBitmap> {
    let width = measure_proportional_text(font, text)?;
    let pixel_count = width.checked_mul(PCS_GLYPH_HEIGHT).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "proportional text pixel count overflows",
        )
    })?;
    let mut pixels = vec![0; pixel_count];
    let mut cursor_x = 0usize;
    for code in text {
        let glyph = font.glyph_for_code(*code).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("proportional font code {code} is outside the loaded range"),
            )
        })?;
        let visible_width = glyph.advance_width as usize;
        if visible_width > 0 {
            blit_monochrome_bitmap(
                &mut pixels,
                width,
                PCS_GLYPH_HEIGHT,
                &glyph.bitmap,
                cursor_x,
                0,
                visible_width,
            );
        }
        cursor_x += visible_width;
    }
    Ok(MonochromeBitmap {
        width,
        height: PCS_GLYPH_HEIGHT,
        pixels,
    })
}

pub fn blit_monochrome_bitmap(
    dst: &mut [u8],
    dst_width: usize,
    dst_height: usize,
    src: &MonochromeBitmap,
    dst_x: usize,
    dst_y: usize,
    visible_width: usize,
) {
    let copy_width = visible_width.min(src.width);
    for y in 0..src.height {
        let target_y = dst_y + y;
        if target_y >= dst_height {
            break;
        }
        for x in 0..copy_width {
            let target_x = dst_x + x;
            if target_x >= dst_width {
                break;
            }
            dst[target_y * dst_width + target_x] = src.pixels[y * src.width + x];
        }
    }
}

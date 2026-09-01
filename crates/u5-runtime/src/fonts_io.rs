//! Loaders/parsers for fixed and proportional fonts plus monochrome bitmaps (BIT/CH/HCS/PCS).

use std::{io, path::Path};

use crate::*;

// ---------------------------------------------------------------------
// `formats/bit.md`: the standalone one-bit-per-pixel bitmap family.
//
// §1: "A `.BIT` file is a small list of one-bit-per-pixel sub-images.
// Three of the four shipped files carry that list inside the same LZW
// envelope the paired `.16`/`.4` graphics archives use
// (`formats/lzw.md`); the fourth stores the list raw. Nothing in the
// family is a display-driver 'sparse strip' table, and the leading
// value is never an entry count for such a table."
//
// `formats/lzw.md §1`: "This envelope applies to the paired `.16` and
// `.4` graphics archive family and to the standalone bitmap family:
// `TITLE.BIT`, `BRITISH.BIT`, and `PROPORT.PCS`. The one documented
// exception is `WD.BIT`, which stores its payload raw with no
// envelope. Earlier revisions of this document excluded the whole
// `.BIT` and `.PCS` family from the envelope; that exclusion was
// wrong."
//
// `formats/bit.md §5`: "There is no driver dispatch entry that
// decompresses a `.BIT` file ... The dispatch slot that earlier
// revisions of this document assigned the decode role (`0x42`) belongs
// to the packed-to-planar preparation step for the `.16`/`.4` archives
// and never touches this family."
// ---------------------------------------------------------------------

pub fn load_title_bit(game_dir: &Path) -> io::Result<TitleBitImages> {
    parse_title_bit(&read_disk_file(&game_dir.join(TITLE_BIT_FILE))?)
}

/// `formats/bit.md §4.1`: `TITLE.BIT` holds ten sub-images, carried in
/// the shared LZW envelope.
pub fn parse_title_bit(bytes: &[u8]) -> io::Result<TitleBitImages> {
    Ok(TitleBitImages {
        blocks: parse_bit_family_resource(bytes, TITLE_BIT_FILE)?,
    })
}

/// Alias kept for callers that read the whole resource off disk. The
/// file has exactly one reading, so this is [`parse_title_bit`].
pub fn parse_title_bit_loaded_resource(bytes: &[u8]) -> io::Result<TitleBitImages> {
    parse_title_bit(bytes)
}

/// Parse an already-decoded image (envelope removed) as the
/// `formats/bit.md §3` sub-image list.
pub fn parse_title_bit_body(body: &[u8], resource_name: &str) -> io::Result<TitleBitImages> {
    Ok(TitleBitImages {
        blocks: parse_bit_sub_image_list(body, resource_name)?,
    })
}

pub fn load_british_bit(game_dir: &Path) -> io::Result<MonochromeBitmap> {
    parse_british_bit(&read_disk_file(&game_dir.join(BRITISH_BIT_FILE))?)
}

/// `formats/bit.md §4.2`: "`BRITISH.BIT` holds a single 272x62
/// sub-image", carried in the shared LZW envelope.
pub fn parse_british_bit(bytes: &[u8]) -> io::Result<MonochromeBitmap> {
    single_bit_sub_image(
        parse_bit_family_resource(bytes, BRITISH_BIT_FILE)?,
        BRITISH_BIT_FILE,
    )
}

/// Alias kept for callers that read the whole resource off disk.
pub fn parse_british_bit_loaded_resource(bytes: &[u8]) -> io::Result<MonochromeBitmap> {
    parse_british_bit(bytes)
}

pub fn load_wd_bit(game_dir: &Path) -> io::Result<MonochromeBitmap> {
    parse_wd_bit(&read_disk_file(&game_dir.join(WD_BIT_FILE))?)
}

/// `formats/bit.md §4.3`: "`WD.BIT` is the one shipped member stored
/// without the LZW envelope. Parsed directly it is a one-sub-image
/// resource: count `1`, single offset `4`, then a 288x49 image whose
/// rows occupy 36 bytes each." The earlier sparse-strip-table reading
/// of those four leading words is withdrawn: "the four leading words
/// are simply the count, the single offset, the width, and the height."
pub fn parse_wd_bit(bytes: &[u8]) -> io::Result<MonochromeBitmap> {
    single_bit_sub_image(parse_bit_family_resource(bytes, WD_BIT_FILE)?, WD_BIT_FILE)
}

/// Parse an already-decoded single-sub-image resource body.
pub fn parse_single_bit_sub_image(
    body: &[u8],
    resource_name: &str,
) -> io::Result<MonochromeBitmap> {
    single_bit_sub_image(
        parse_bit_sub_image_list(body, resource_name)?,
        resource_name,
    )
}

fn single_bit_sub_image(
    mut images: Vec<MonochromeBitmap>,
    resource_name: &str,
) -> io::Result<MonochromeBitmap> {
    if images.len() != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} must hold exactly one sub-image, got {}",
                images.len()
            ),
        ));
    }
    Ok(images.remove(0))
}

/// `formats/bit.md §2.1`, "Deciding whether a file is enveloped":
///
/// 1. "Try to parse the file directly as the sub-image list of Section
///    3, starting at byte 0."
/// 2. "If the walk stays inside the file and consumes it exactly to the
///    last byte, the file is stored raw. This succeeds only for
///    `WD.BIT`."
/// 3. "Otherwise treat the first four bytes as the LZW decoded length,
///    decode the remainder per `formats/lzw.md`, and parse the decoded
///    image as the sub-image list. The decoded byte count must equal
///    the declared length and the code stream must end with a proper
///    end code."
///
/// "There is no known 'pre-decoded' packaging variant of these files.
/// Earlier guidance in this document described one; that guidance was
/// mistaken and has been removed." This is therefore a structural
/// classification, not a fallback for a second packaging: the raw walk
/// is exact, so at most one of the two shapes can hold, and "the
/// structural test exists so that a validator can reject a corrupt or
/// substituted file rather than mis-parsing it."
pub fn parse_bit_family_resource(
    bytes: &[u8],
    resource_name: &str,
) -> io::Result<Vec<MonochromeBitmap>> {
    match parse_bit_sub_image_list(bytes, resource_name) {
        Ok(images) => Ok(images),
        Err(raw_err) => {
            let body = decode_lzw_envelope(bytes, resource_name).map_err(|envelope_err| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "{resource_name} is not a raw sub-image list ({raw_err}) and its LZW envelope does not decode ({envelope_err})"
                    ),
                )
            })?;
            parse_bit_sub_image_list(&body, resource_name)
        }
    }
}

/// `formats/bit.md §3`: "After the envelope is removed (or immediately,
/// for a raw file), the image is a directory followed by contiguous
/// sub-images" — a two-byte sub-image count, `count * 2` bytes of
/// two-byte offsets measured from the start of the decoded image, then
/// the sub-images "stored back to back, in offset order".
///
/// §6: "There are no sparse or skipped entries and no over-allocated
/// table; every entry in the directory names a real sub-image."
///
/// §3 also names the invariant that "pins the whole reading":
/// "consecutive offsets differ by exactly
/// `4 + max(1, ceil(width / 8)) * height` of the earlier record — the
/// four header bytes plus its row data, with nothing between records. A
/// candidate parse that satisfies that relation for every adjacent pair
/// and consumes the image exactly is the correct one." That is what
/// makes the §2.1 raw-versus-enveloped test exact, so it is enforced
/// here rather than left to a validator.
pub fn parse_bit_sub_image_list(
    body: &[u8],
    resource_name: &str,
) -> io::Result<Vec<MonochromeBitmap>> {
    if body.len() < BIT_SUB_IMAGE_COUNT_WORD_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sub-image directory is shorter than its count word"),
        ));
    }
    let count = u16_at(body, 0) as usize;
    if count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sub-image directory declares no sub-images"),
        ));
    }
    let header_len = count
        .checked_mul(BIT_OFFSET_TABLE_ENTRY_LEN)
        .and_then(|table| table.checked_add(BIT_SUB_IMAGE_COUNT_WORD_LEN))
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sub-image directory header overflows"),
            )
        })?;
    if header_len > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sub-image offset table exceeds the image"),
        ));
    }

    let mut images = Vec::with_capacity(count);
    let mut expected_offset = header_len;
    for slot in 0..count {
        let offset = u16_at(
            body,
            BIT_SUB_IMAGE_COUNT_WORD_LEN + slot * BIT_OFFSET_TABLE_ENTRY_LEN,
        ) as usize;
        // "The first offset in the table always equals `2 + count * 2`",
        // and each later record starts where the previous one ended.
        if offset != expected_offset {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{resource_name} sub-image {slot} starts at {offset}, but the contiguous layout puts it at {expected_offset}"
                ),
            ));
        }
        if offset + BIT_SUB_IMAGE_HEADER_LEN > body.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sub-image {slot} header at {offset} is truncated"),
            ));
        }
        let width = u16_at(body, offset) as usize;
        let height = u16_at(body, offset + BIT_SUB_IMAGE_WIDTH_WORD_LEN) as usize;
        let row_stride = bit_sub_image_row_stride(width)?;
        let record_len = row_stride
            .checked_mul(height)
            .and_then(|rows| rows.checked_add(BIT_SUB_IMAGE_HEADER_LEN))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{resource_name} sub-image {slot} record length overflows"),
                )
            })?;
        let record_end = offset.checked_add(record_len).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sub-image {slot} record end overflows"),
            )
        })?;
        if record_end > body.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sub-image {slot} row data exceeds the image"),
            ));
        }
        let rows_start = offset + BIT_SUB_IMAGE_HEADER_LEN;
        images.push(MonochromeBitmap {
            width,
            height,
            pixels: unpack_monochrome_rows(
                &body[rows_start..record_end],
                width,
                height,
                row_stride,
                width * height,
            ),
        });
        expected_offset = record_end;
    }
    // "the last sub-image ends exactly at the end of the decoded image"
    if expected_offset != body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} sub-images end at {expected_offset}, not at the image end {}",
                body.len()
            ),
        ));
    }
    Ok(images)
}

/// `formats/bit.md §3`: "The row stride is `max(1, ceil(width / 8))`
/// bytes. ... The `max(1, ...)` clause covers the one shipped record
/// whose width is zero — glyph index 0 of `PROPORT.PCS`, the space —
/// which still reserves one byte per row, all of it padding."
pub fn bit_sub_image_row_stride(width: usize) -> io::Result<usize> {
    Ok(monochrome_row_stride(width)?.max(1))
}

// `formats/bit.md §3`: the earlier bitmap helpers here sized a record's row
// data as `(width * height + 7) / 8` and unpacked it as one unbroken bit run.
// That is only ever right when the width is a multiple of eight, and §3 is
// explicit that it is not the layout: "The row stride is
// `max(1, ceil(width / 8))` bytes ... Each row starts on a byte boundary, so a
// width that is not a multiple of eight leaves padding bits at the end of the
// row; those padding bits are not pixels." They had no callers left once
// `parse_bit_sub_image_list` became the one reading, so they are gone rather
// than left as a plausible-looking hook. Use `bit_sub_image_row_stride` and
// `unpack_monochrome_rows`.

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

/// Loads the glyph directory a proportional paragraph renderer needs.
///
/// `formats/font-pcs.md §1`: "It uses exactly the container documented
/// in `formats/bit.md`: the shared LZW envelope of `formats/lzw.md`
/// wrapping a one-bit-per-pixel sub-image list. Earlier revisions of
/// this document described a 'driver-compressed sparse strip resource'
/// and told readers not to feed the file to the LZW decoder. That was
/// wrong in both directions and has been replaced."
///
/// `formats/font-pcs.md §3`: "Take the first four bytes as the
/// little-endian decoded length and decode the remainder with the
/// shared LZW decoder ... The shipped file is 802 bytes on disk and
/// declares, and produces, 1276 decoded bytes."
pub fn parse_proportional_font(bytes: &[u8]) -> io::Result<ProportionalFont> {
    match decode_lzw_envelope(bytes, PROPORT_PCS_FILE) {
        Ok(body) => parse_proportional_font_body(&body, PROPORT_PCS_FILE),
        // `formats/bit.md §2.1` allows a member of this family to be stored
        // raw — `WD.BIT` is — and the envelope check is exact, so this is the
        // remaining classification rather than a second packaging variant.
        Err(envelope_err) => parse_proportional_font_body(bytes, PROPORT_PCS_FILE).map_err(
            |raw_err| {
                io::Error::new(
                    raw_err.kind(),
                    format!(
                        "{PROPORT_PCS_FILE} is neither an LZW-enveloped glyph directory ({envelope_err}) nor a raw one ({raw_err})"
                    ),
                )
            },
        ),
    }
}

pub fn load_proportional_font_resource(game_dir: &Path) -> io::Result<ProportionalFontResource> {
    parse_proportional_font_resource(&read_disk_file(&game_dir.join(PROPORT_PCS_FILE))?)
}

/// `formats/font-pcs.md §7`: a strict loader must "Treat the first four
/// bytes as the LZW decoded length, not as a resource entry count, and
/// require the decoded byte count to match it", then "Require the
/// decoded image's sub-image count, offset table, and record sizes to
/// satisfy the checks in `formats/bit.md` Section 6".
///
/// The shipped file is enveloped, so the §2.1 classification in
/// [`parse_bit_family_resource`] takes the envelope branch; the raw
/// branch stays available for a replacement font stored the way
/// `WD.BIT` is.
pub fn parse_proportional_font_resource(bytes: &[u8]) -> io::Result<ProportionalFontResource> {
    Ok(ProportionalFontResource {
        strips: parse_bit_family_resource(bytes, PROPORT_PCS_FILE)?,
    })
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
        let ink_width = u16_at(body, offset) as usize;
        let glyph_height = u16_at(body, offset + 2) as usize;
        if ink_width > PCS_GLYPH_BITMAP_WIDTH {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{resource_name} glyph slot {slot} ink width {ink_width} exceeds bitmap width"
                ),
            ));
        }
        if glyph_height != PCS_GLYPH_HEIGHT {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{resource_name} glyph slot {slot} height {glyph_height} is not the proportional cell height {PCS_GLYPH_HEIGHT}"
                ),
            ));
        }
        glyphs.push(ProportionalGlyph {
            advance_width: ink_width as u8,
            bitmap: MonochromeBitmap {
                width: PCS_GLYPH_BITMAP_WIDTH,
                height: PCS_GLYPH_HEIGHT,
                pixels: unpack_monochrome_rows(
                    &body[offset + PCS_GLYPH_BLOCK_HEADER_LEN..end],
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

/// The resident 128-entry proportional advance table
/// (`formats/font-pcs.md` section 4.1, published in answer to
/// `cleak/u5-spec#70`).
///
/// Entries `0x20..=0x7A` are byte-identical to the per-glyph widths stored in
/// `PROPORT.PCS` itself, so [`proportional_width_table_from_font`] rebuilds
/// this constant from any loaded font;
/// `fonts_proportional_width_table_matches_shipped_font` asserts that equality
/// against the local asset set. Entries `0x7B..=0x7F` are zero.
///
/// The 32 entries below `0x20` are **not** width data - that part of the
/// resident image overlaps unrelated resident text - and they are unreachable,
/// because the renderer handles every byte at or below `0x20` as a space and
/// only ever draws bytes above `0x20`. They are listed as zero and must not be
/// given invented values.
///
/// Two entries are present but never consulted for their face value: space
/// (`0x20`) is zero because the space advance is layout-descriptor state, not
/// font metrics, and `{` (`0x7B`) is zero because the renderer intercepts it
/// and measures a flat 15 pixels. `_` (`0x5F`) holds 8 but is intercepted as
/// the soft-hyphen marker and measured as zero; what the renderer uses is the
/// hyphen's own entry (`0x2D`, 3).
///
/// These are widths, not advances: a drawn glyph advances the pen by its entry
/// plus [`PCS_GLYPH_ADVANCE_GAP`] (`font-pcs.md` section 4.2).
pub const PROPORTIONAL_WIDTH_TABLE: ProportionalWidthTable = ProportionalWidthTable::new([
    // 0x00..0x1F: not width data, unreachable, deliberately unpublished.
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, //
    // 0x20 ' '..0x2F '/'
    0, 2, 6, 7, 7, 7, 7, 3, 4, 4, 5, 6, 3, 3, 2, 7, //
    // 0x30 '0'..0x3F '?'
    6, 5, 6, 6, 6, 6, 6, 6, 6, 6, 2, 3, 5, 5, 5, 6, //
    // 0x40 '@'..0x4F 'O'
    7, 7, 6, 6, 6, 6, 6, 6, 6, 4, 7, 7, 6, 7, 6, 6, //
    // 0x50 'P'..0x5F '_'
    7, 6, 7, 6, 6, 6, 6, 7, 6, 6, 6, 3, 7, 3, 7, 8, //
    // 0x60 '`'..0x6F 'o'
    3, 5, 5, 4, 5, 5, 5, 5, 5, 2, 3, 5, 2, 7, 6, 5, //
    // 0x70 'p'..0x7A 'z', then 0x7B..0x7F.
    5, 5, 4, 4, 4, 5, 5, 7, 5, 5, 4, 0, 0, 0, 0, 0,
]);

/// Rebuilds [`PROPORTIONAL_WIDTH_TABLE`] from a loaded `PROPORT.PCS` glyph
/// directory. `formats/font-pcs.md` section 4.1 states the two are identical
/// over `0x20..=0x7A`, so either source reproduces the original exactly.
pub fn proportional_width_table_from_font(font: &ProportionalFont) -> ProportionalWidthTable {
    let mut widths = [0u8; PROPORTIONAL_WIDTH_TABLE_LEN];
    for (slot, glyph) in font.glyphs.iter().enumerate() {
        let Some(code) = u8::try_from(slot)
            .ok()
            .and_then(|slot| font.first_code.checked_add(slot))
        else {
            break;
        };
        if usize::from(code) >= PROPORTIONAL_WIDTH_TABLE_LEN {
            break;
        }
        widths[usize::from(code)] = glyph.advance_width;
    }
    ProportionalWidthTable::new(widths)
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

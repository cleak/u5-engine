#[cfg(test)]
pub fn load_british_bit(game_dir: &Path) -> io::Result<MonochromeBitmap> {
    parse_british_bit(&read(&game_dir.join(BRITISH_BIT_FILE))?)
}

#[cfg(test)]
pub fn parse_british_bit(bytes: &[u8]) -> io::Result<MonochromeBitmap> {
    let body = decode_lzw_envelope(bytes, BRITISH_BIT_FILE)?;
    parse_single_image_bit_body(&body, BRITISH_BIT_FILE)
}

#[cfg(test)]
pub fn load_wd_bit(game_dir: &Path) -> io::Result<MonochromeBitmap> {
    parse_wd_bit(&read(&game_dir.join(WD_BIT_FILE))?)
}

#[cfg(test)]
pub fn parse_wd_bit(bytes: &[u8]) -> io::Result<MonochromeBitmap> {
    parse_single_image_bit_body(bytes, WD_BIT_FILE)
}

#[cfg(test)]
pub fn parse_single_image_bit_body(body: &[u8], resource_name: &str) -> io::Result<MonochromeBitmap> {
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
pub fn unpack_monochrome_bits(bytes: &[u8], pixel_count: usize) -> Vec<u8> {
    let mut pixels = Vec::with_capacity(pixel_count);
    for pixel in 0..pixel_count {
        let byte = bytes[pixel / 8];
        pixels.push((byte >> (7 - (pixel % 8))) & 1);
    }
    pixels
}

#[cfg(test)]
pub fn load_ch_font(game_dir: &Path, file_name: &str) -> io::Result<FixedFont> {
    parse_fixed_font_body(
        &read(&game_dir.join(file_name))?,
        file_name,
        CH_FONT_CELL_WIDTH,
        CH_FONT_CELL_HEIGHT,
    )
}

#[cfg(test)]
pub fn load_hcs_font(game_dir: &Path, file_name: &str) -> io::Result<FixedFont> {
    parse_fixed_font_body(
        &read(&game_dir.join(file_name))?,
        file_name,
        HCS_FONT_CELL_WIDTH,
        HCS_FONT_CELL_HEIGHT,
    )
}

#[cfg(test)]
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

#[cfg(test)]
pub fn load_proportional_font(game_dir: &Path) -> io::Result<ProportionalFont> {
    parse_proportional_font(&read(&game_dir.join(PROPORT_PCS_FILE))?)
}

#[cfg(test)]
pub fn parse_proportional_font(bytes: &[u8]) -> io::Result<ProportionalFont> {
    let body = decode_lzw_envelope(bytes, PROPORT_PCS_FILE)?;
    parse_proportional_font_body(&body, PROPORT_PCS_FILE)
}

#[cfg(test)]
pub fn parse_proportional_font_body(body: &[u8], resource_name: &str) -> io::Result<ProportionalFont> {
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

#[cfg(test)]
pub fn monochrome_row_stride(width: usize) -> io::Result<usize> {
    width.checked_add(7).map(|bits| bits / 8).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "monochrome row stride overflows",
        )
    })
}

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
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

pub fn parse_tlk(path: &Path) -> io::Result<HashMap<u16, Vec<String>>> {
    let bytes = read(path)?;
    parse_tlk_bytes(&bytes)
}

pub fn parse_tlk_bytes(bytes: &[u8]) -> io::Result<HashMap<u16, Vec<String>>> {
    if bytes.len() < 4 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "short TLK"));
    }
    let count = u16_at(&bytes, 0) as usize;
    let mut entries = Vec::new();
    for k in 1..count {
        let off = u16_at(&bytes, 4 * k) as usize;
        let id = u16_at(&bytes, 4 * k + 2);
        entries.push((id, off));
    }
    entries.sort_by_key(|(_, off)| *off);
    let mut out = HashMap::new();
    for (idx, (id, off)) in entries.iter().enumerate() {
        let end = entries
            .get(idx + 1)
            .map(|(_, next)| *next)
            .unwrap_or(bytes.len());
        if *off >= bytes.len() || *off >= end {
            continue;
        }
        let mut fields = Vec::new();
        let mut pos = *off;
        while pos < end && fields.len() < 40 {
            let (field, next) = decode_tlk_field(&bytes, pos, end);
            fields.push(field);
            pos = next;
            if pos == end {
                break;
            }
        }
        out.insert(*id, fields);
    }
    Ok(out)
}

pub fn decode_tlk_field(bytes: &[u8], mut pos: usize, end: usize) -> (String, usize) {
    let mut s = String::new();
    while pos < end {
        let b = bytes[pos];
        pos += 1;
        if b == 0 {
            break;
        }
        match b {
            0x85 => pos = (pos + 3).min(end),
            0x86 | 0x8c => pos = (pos + 1).min(end),
            0xfe => pos = (pos + 2).min(end),
            0xa0..=0xfd => s.push((b ^ 0x80) as char),
            0x01..=0x9d => s.push(' '),
            _ => {}
        }
    }
    (compact(&s), pos)
}

pub fn non_empty_talk_keyword(keyword: &str) -> Option<&str> {
    let keyword = keyword.trim();
    (!keyword.is_empty()).then_some(keyword)
}

pub fn talk_keyword_response<'a>(fields: &'a [String], keyword: &str) -> Option<&'a str> {
    if talk_keyword_matches("JOB", keyword) {
        return fields.get(3).map(String::as_str);
    }
    if talk_keyword_matches("BYE", keyword) {
        return fields.get(4).map(String::as_str);
    }

    fields
        .get(5..)
        .unwrap_or_default()
        .chunks_exact(2)
        .find_map(|pair| talk_keyword_matches(&pair[0], keyword).then_some(pair[1].as_str()))
}

pub fn talk_keyword_matches(stored_keyword: &str, input: &str) -> bool {
    let stored = talk_keyword_compare_text(stored_keyword.trim());
    if stored.is_empty() {
        return false;
    }
    let input = talk_keyword_compare_text(input.trim_start());
    input.starts_with(&stored)
        && input
            .as_bytes()
            .get(stored.len())
            .is_none_or(|byte| *byte == b' ')
}

pub fn talk_keyword_compare_text(value: &str) -> String {
    value
        .bytes()
        .map(|byte| (byte & 0x7f).to_ascii_uppercase() as char)
        .collect()
}

pub fn parse_npc_block(
    game_dir: &Path,
    scene: Scene,
    tlk: &HashMap<u16, Vec<String>>,
) -> io::Result<Vec<NpcSlot>> {
    let bytes = read(&game_dir.join(format!("{}.NPC", scene.family.stem())))?;
    let base = scene.block * 576;
    if base + 576 > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "short NPC block",
        ));
    }
    let mut slots = Vec::new();
    for slot in 0..32 {
        let mut schedule = [0u8; 16];
        schedule.copy_from_slice(&bytes[base + slot * 16..base + slot * 16 + 16]);
        let type_byte = bytes[base + 512 + slot];
        let dialog_id = bytes[base + 544 + slot];
        let name = tlk
            .get(&(dialog_id as u16))
            .and_then(|fields| fields.first())
            .filter(|name| !name.is_empty())
            .cloned();
        slots.push(NpcSlot {
            slot,
            type_byte,
            dialog_id,
            schedule,
            name,
        });
    }
    Ok(slots)
}

pub fn load_floor(game_dir: &Path, scene: Scene, floor: i8) -> io::Result<Vec<u8>> {
    let bytes = read(&game_dir.join(format!("{}.DAT", scene.family.stem())))?;
    let page = resolve_location_floor_page(game_dir, scene, floor)?;
    let start = page * 1024;
    if start + 1024 > bytes.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}.DAT is too short for {} floor {} page {}",
                scene.family.stem(),
                scene.key(),
                floor,
                page
            ),
        ));
    }
    Ok(bytes[start..start + 1024].to_vec())
}

pub fn load_town_runtime_floor(
    game_dir: &Path,
    scene: Scene,
    floor: i8,
    hour: u8,
) -> io::Result<Vec<u8>> {
    let mut grid = load_floor(game_dir, scene, floor)?;
    normalize_town_runtime_floor(&mut grid, hour);
    Ok(grid)
}

pub fn normalize_town_runtime_floor(grid: &mut [u8], hour: u8) {
    scrub_location_entry_markers(grid);
    if is_town_night_hour(hour) {
        apply_dawn_dusk_substitution(grid);
    }
}

pub fn resolve_location_floor_page(game_dir: &Path, scene: Scene, floor: i8) -> io::Result<usize> {
    let base_page = load_location_floor_entries(game_dir)?
        .and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.scene == scene)
                .map(|entry| entry.base_page)
        })
        .unwrap_or_else(|| scene.block * 2);
    let page = base_page as i16 + floor as i16;
    if !(0..16).contains(&page) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "{} maps {} floor {} to page {}, outside 0..15",
                LOCATION_FLOOR_TABLE_FILE,
                scene.key(),
                floor,
                page
            ),
        ));
    }
    Ok(page as usize)
}

pub fn load_dungeon_record(game_dir: &Path, scene: DungeonScene) -> io::Result<Vec<u8>> {
    let bytes = read(&game_dir.join("DUNGEON.DAT"))?;
    if bytes.len() != DUNGEON_DAT_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "DUNGEON.DAT must be {DUNGEON_DAT_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    let start = scene.record * DUNGEON_RECORD_LEN;
    Ok(bytes[start..start + DUNGEON_RECORD_LEN].to_vec())
}

pub fn load_world_map(game_dir: &Path, plane: WorldPlane) -> io::Result<Vec<u8>> {
    let bytes = read(&game_dir.join(plane.file_name()))?;
    match plane {
        WorldPlane::Underworld => decode_world_map_bytes(plane, &bytes),
        WorldPlane::Britannia => {
            let data = read(&game_dir.join("DATA.OVL"))?;
            let chunk_index = find_britannia_chunk_index(&data)?;
            decode_britannia_map_bytes(&bytes, &chunk_index)
        }
    }
}

pub fn load_world_location_entries(game_dir: &Path) -> io::Result<Option<Vec<WorldLocationEntry>>> {
    let path = game_dir.join(WORLD_LOCATION_TABLE_FILE);
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(io::Error::new(
                err.kind(),
                format!("{}: {err}", path.display()),
            ));
        }
    };
    parse_world_location_entries(&text).map(Some)
}


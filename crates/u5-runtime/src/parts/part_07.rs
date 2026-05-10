pub fn parse_transport_arg(value: &str) -> io::Result<TransportState> {
    match value.trim().to_ascii_lowercase().as_str() {
        "foot" => Ok(TransportState::Foot),
        "horse" => Ok(TransportState::Horse {
            type_byte: 160,
            tile: 160,
        }),
        "ship" | "frigate" => Ok(TransportState::Ship {
            type_byte: FIRST_PLAYABLE_FRIGATE_TILE,
            tile: FIRST_PLAYABLE_FRIGATE_TILE,
            sails_hoisted: false,
            hull: FIRST_PLAYABLE_FULL_SHIP_HULL,
            skiffs: 2,
        }),
        "skiff" => Ok(TransportState::Skiff {
            type_byte: FIRST_PLAYABLE_SKIFF_TILE,
            tile: FIRST_PLAYABLE_SKIFF_TILE,
        }),
        "carpet" | "magic-carpet" => Ok(TransportState::Carpet {
            type_byte: 184,
            tile: 184,
        }),
        "balloon" => Ok(TransportState::Balloon {
            type_byte: FIRST_PLAYABLE_BALLOON_TILE,
            tile: FIRST_PLAYABLE_BALLOON_TILE,
        }),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown transport `{value}`; expected foot|horse|ship|skiff|carpet|balloon"),
        )),
    }
}

pub fn parse_time_arg(value: &str) -> io::Result<GameClock> {
    let (hour, minute) = if let Some((hour, minute)) = value.split_once(':') {
        (hour, minute)
    } else {
        (value, "0")
    };
    let hour = hour.trim().parse::<u8>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid time hour `{hour}`: {err}"),
        )
    })?;
    let minute = minute.trim().parse::<u8>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid time minute `{minute}`: {err}"),
        )
    })?;
    GameClock::new(hour, minute)
}

pub fn validate_start(
    grid: &[u8],
    pos: (usize, usize),
    passability: Option<&TilePassability>,
) -> io::Result<()> {
    if pos.0 >= 32 || pos.1 >= 32 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "start coordinate must be inside 0..31, got ({}, {})",
                pos.0, pos.1
            ),
        ));
    }
    let tile = grid[pos.1 * 32 + pos.0];
    if !is_tile_walkable(tile, passability) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                tile_class(tile)
            ),
        ));
    }
    Ok(())
}

pub fn validate_dungeon_start(
    grid: &[u8],
    scene: DungeonScene,
    level: u8,
    pos: (usize, usize),
) -> io::Result<()> {
    if pos.0 >= DUNGEON_SIDE || pos.1 >= DUNGEON_SIDE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "dungeon coordinate must be inside 0..7, got ({}, {})",
                pos.0, pos.1
            ),
        ));
    }
    let tile = grid[dungeon_cell_index(level, pos.0, pos.1)];
    if !is_dungeon_walkable(tile) && !is_public_dungeon_reaction_seed(scene, level, pos, tile) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "dungeon start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                dungeon_cell_class(tile)
            ),
        ));
    }
    Ok(())
}

pub fn is_public_dungeon_reaction_seed(
    scene: DungeonScene,
    level: u8,
    pos: (usize, usize),
    tile: u8,
) -> bool {
    let is_surface_seed = level == 0 && pos == (1, 1);
    let is_underworld_seed = scene.record != 7 && level == 7 && pos == (7, 7);
    (is_surface_seed || is_underworld_seed) && matches!(tile >> 4, 0x0a | 0x0f)
}

pub fn validate_world_start_for_transport(
    grid: &[u8],
    pos: (usize, usize),
    plane: WorldPlane,
    passability: Option<&TilePassability>,
    transport: TransportState,
    damage_tiles: &[WorldDamageTileEntry],
) -> io::Result<()> {
    if pos.0 >= WORLD_SIDE || pos.1 >= WORLD_SIDE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "world coordinate must be inside 0..255, got ({}, {})",
                pos.0, pos.1
            ),
        ));
    }
    let tile = grid[world_cell_index(pos.0, pos.1)];
    if let Some(entry) = world_damage_tile_entry_at(damage_tiles, plane, pos.0, pos.1, tile) {
        if entry.effect.allows_transport(transport) {
            return Ok(());
        }
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "world start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                entry.effect.label()
            ),
        ));
    }
    if !is_tile_walkable_for_transport(tile, passability, transport) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "world start coordinate ({}, {}) is blocked by {}",
                pos.0,
                pos.1,
                tile_class(tile)
            ),
        ));
    }
    Ok(())
}

pub fn pass_fail(value: bool) -> &'static str {
    if value { "PASS" } else { "FAIL" }
}

pub fn read(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path).map_err(|err| io::Error::new(err.kind(), format!("{}: {err}", path.display())))
}

pub fn load_tile_passability(game_dir: &Path) -> io::Result<Option<TilePassability>> {
    let path = game_dir.join(TILE_PASSABILITY_FILE);
    match fs::read(&path) {
        Ok(bytes) => TilePassability::from_bytes(&bytes).map(Some),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(io::Error::new(
            err.kind(),
            format!("{}: {err}", path.display()),
        )),
    }
}

pub fn load_look_table(game_dir: &Path) -> io::Result<LookTable> {
    parse_look2_dat(&read(&game_dir.join(LOOK2_DAT_FILE))?)
}

pub fn parse_look2_dat(bytes: &[u8]) -> io::Result<LookTable> {
    if bytes.len() < LOOK2_TABLE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{LOOK2_DAT_FILE} must be at least {LOOK2_TABLE_LEN} bytes, got {}",
                bytes.len()
            ),
        ));
    }
    let meaningful_len = if bytes.last() == Some(&0x1a) {
        bytes.len() - 1
    } else {
        bytes.len()
    };
    if meaningful_len < LOOK2_TABLE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{LOOK2_DAT_FILE} has no string pool"),
        ));
    }

    let mut descriptions = Vec::with_capacity(LOOK2_TILE_COUNT);
    for tile in 0..LOOK2_TILE_COUNT {
        let offset = u16_at(bytes, tile * 2) as usize;
        if offset < LOOK2_TABLE_LEN || offset >= meaningful_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOOK2_DAT_FILE} tile {tile} has invalid string offset {offset}"),
            ));
        }
        let raw = &bytes[offset..meaningful_len];
        let Some(end) = raw.iter().position(|byte| *byte == 0) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOOK2_DAT_FILE} tile {tile} string is not NUL-terminated"),
            ));
        };
        let raw = &raw[..end];
        if !raw.iter().all(u8::is_ascii) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{LOOK2_DAT_FILE} tile {tile} string is not plain ASCII"),
            ));
        }
        let description = std::str::from_utf8(raw)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?
            .to_string();
        descriptions.push(description);
    }
    Ok(LookTable { descriptions })
}

pub struct LzwBitReader<'a> {
    pub bytes: &'a [u8],
    pub bit_pos: usize,
}

impl<'a> LzwBitReader<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, bit_pos: 0 }
    }

    pub fn read_code(&mut self, code_size: u8) -> Option<u16> {
        let bit_count = code_size as usize;
        if self.bit_pos + bit_count > self.bytes.len() * 8 {
            return None;
        }

        let mut code = 0u16;
        for bit_offset in 0..bit_count {
            let source_bit = self.bit_pos + bit_offset;
            let bit = (self.bytes[source_bit / 8] >> (source_bit % 8)) & 1;
            code |= (bit as u16) << bit_offset;
        }
        self.bit_pos += bit_count;
        Some(code)
    }
}

pub fn reset_lzw_dictionary(dictionary: &mut Vec<Vec<u8>>) {
    dictionary.clear();
    for byte in 0..=255u16 {
        dictionary.push(vec![byte as u8]);
    }
    dictionary.push(Vec::new());
    dictionary.push(Vec::new());
}

pub fn decode_lzw_envelope(bytes: &[u8], resource_name: &str) -> io::Result<Vec<u8>> {
    if bytes.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} LZW envelope is shorter than its length header"),
        ));
    }
    let expected_len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    decode_gif_lzw_payload(&bytes[4..], expected_len, resource_name)
}

pub fn decode_gif_lzw_payload(
    payload: &[u8],
    expected_len: usize,
    resource_name: &str,
) -> io::Result<Vec<u8>> {
    let mut reader = LzwBitReader::new(payload);
    let mut dictionary = Vec::with_capacity(LZW_MAX_CODES as usize);
    reset_lzw_dictionary(&mut dictionary);

    let mut code_size = LZW_INITIAL_CODE_SIZE;
    let mut next_code = LZW_FIRST_USER_CODE;
    let mut previous: Option<Vec<u8>> = None;
    let mut output = Vec::with_capacity(expected_len);
    let mut saw_end = false;

    loop {
        let Some(code) = reader.read_code(code_size) else {
            break;
        };

        match code {
            LZW_CLEAR_CODE => {
                reset_lzw_dictionary(&mut dictionary);
                code_size = LZW_INITIAL_CODE_SIZE;
                next_code = LZW_FIRST_USER_CODE;
                previous = None;
                continue;
            }
            LZW_END_CODE => {
                saw_end = true;
                break;
            }
            _ => {}
        }

        let entry = if code == next_code {
            let Some(previous_entry) = previous.as_ref() else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("{resource_name} LZW stream used KwKwK before a previous entry"),
                ));
            };
            let mut entry = previous_entry.clone();
            entry.push(previous_entry[0]);
            entry
        } else if code < next_code
            && (code as usize) < dictionary.len()
            && !dictionary[code as usize].is_empty()
        {
            dictionary[code as usize].clone()
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} LZW stream referenced invalid code {code}"),
            ));
        };

        output.extend_from_slice(&entry);
        if output.len() > expected_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} LZW output exceeded declared length {expected_len}"),
            ));
        }

        if let Some(previous_entry) = previous.as_ref() {
            if next_code < LZW_MAX_CODES {
                let mut dictionary_entry = previous_entry.clone();
                dictionary_entry.push(entry[0]);
                dictionary.push(dictionary_entry);
                next_code += 1;
                if next_code == (1u16 << code_size) && code_size < LZW_MAX_CODE_SIZE {
                    code_size += 1;
                }
            }
        }

        previous = Some(entry);
    }

    if !saw_end {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} LZW payload ended before the end code"),
        ));
    }
    if output.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} LZW declared length {expected_len}, decoded {} bytes",
                output.len()
            ),
        ));
    }

    Ok(output)
}

pub fn load_tile_atlas(game_dir: &Path, depth: TileGraphicsDepth) -> io::Result<TileAtlas> {
    let file_name = depth.file_name();
    parse_tile_atlas(&read(&game_dir.join(file_name))?, depth, file_name)
}

pub fn parse_tile_atlas(
    bytes: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<TileAtlas> {
    let body = decode_lzw_envelope(bytes, resource_name)?;
    unpack_tile_atlas_body(&body, depth, resource_name)
}

pub fn unpack_tile_atlas_body(
    body: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<TileAtlas> {
    let expected_len = depth.body_len();
    if body.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} {} body must contain exactly {expected_len} bytes, got {}",
                depth.label(),
                body.len()
            ),
        ));
    }

    let mut pixels = Vec::with_capacity(TILE_ATLAS_PIXEL_LEN);
    match depth {
        TileGraphicsDepth::Ega16 => {
            for byte in body {
                pixels.push(byte >> 4);
                pixels.push(byte & 0x0f);
            }
        }
        TileGraphicsDepth::Cga4 => {
            for byte in body {
                pixels.push((byte >> 6) & 0x03);
                pixels.push((byte >> 4) & 0x03);
                pixels.push((byte >> 2) & 0x03);
                pixels.push(byte & 0x03);
            }
        }
    }

    Ok(TileAtlas { depth, pixels })
}

pub fn blit_tile_to_viewport(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    tile: u8,
    cell_x: usize,
    cell_y: usize,
) -> io::Result<()> {
    let tile_pixels = atlas.tile_pixels(tile as usize).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tile atlas is missing tile {tile}"),
        )
    })?;
    let dst_x = cell_x * TILE_ATLAS_SIDE;
    let dst_y = cell_y * TILE_ATLAS_SIDE;
    for row in 0..TILE_ATLAS_SIDE {
        let dst_start = (dst_y + row) * viewport.width + dst_x;
        let src_start = row * TILE_ATLAS_SIDE;
        viewport.pixels[dst_start..dst_start + TILE_ATLAS_SIDE]
            .copy_from_slice(&tile_pixels[src_start..src_start + TILE_ATLAS_SIDE]);
    }
    Ok(())
}

#[cfg(test)]
pub fn tile_graphics_file_name(stem: &str, depth: TileGraphicsDepth) -> String {
    format!("{stem}.{}", depth.file_suffix())
}

#[cfg(test)]
pub fn load_graphic_image_directory(
    game_dir: &Path,
    stem: &str,
    depth: TileGraphicsDepth,
) -> io::Result<GraphicImageDirectory> {
    let file_name = tile_graphics_file_name(stem, depth);
    parse_graphic_image_directory(&read(&game_dir.join(&file_name))?, depth, &file_name)
}

#[cfg(test)]
pub fn parse_graphic_image_directory(
    bytes: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicImageDirectory> {
    let body = decode_lzw_envelope(bytes, resource_name)?;
    parse_graphic_image_directory_body(&body, depth, resource_name)
}

#[cfg(test)]
pub fn parse_graphic_image_directory_body(
    body: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicImageDirectory> {
    if body.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image directory is shorter than its count word"),
        ));
    }
    let count = u16_at(body, 0) as usize;
    let header_len = 2usize
        .checked_add(count.checked_mul(4).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} image directory count overflows"),
            )
        })?)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} image directory header overflows"),
            )
        })?;
    if header_len > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image directory header exceeds body length"),
        ));
    }

    let mut images = Vec::with_capacity(count);
    for slot in 0..count {
        let offset = u32_at(body, 2 + slot * 4) as usize;
        if offset == 0 {
            images.push(None);
            continue;
        }
        if offset < header_len || offset >= body.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} image slot {slot} has invalid offset {offset}"),
            ));
        }
        images.push(Some(parse_graphic_image_block(
            body,
            offset,
            depth,
            resource_name,
        )?));
    }

    Ok(GraphicImageDirectory { depth, images })
}

#[cfg(test)]
pub fn load_graphic_sprite_sheet(
    game_dir: &Path,
    stem: &str,
    depth: TileGraphicsDepth,
) -> io::Result<GraphicSpriteSheet> {
    let file_name = tile_graphics_file_name(stem, depth);
    parse_graphic_sprite_sheet(&read(&game_dir.join(&file_name))?, depth, &file_name)
}

#[cfg(test)]
pub fn parse_graphic_sprite_sheet(
    bytes: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicSpriteSheet> {
    let body = decode_lzw_envelope(bytes, resource_name)?;
    parse_graphic_sprite_sheet_body(&body, depth, resource_name)
}

#[cfg(test)]
pub fn parse_graphic_sprite_sheet_body(
    body: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicSpriteSheet> {
    if body.len() < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite sheet is shorter than its slot-count word"),
        ));
    }
    let slot_count = u16_at(body, 0) as usize;
    if slot_count % 2 != 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite sheet slot count must be even, got {slot_count}"),
        ));
    }
    let header_len = 2usize
        .checked_add(slot_count.checked_mul(2).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sprite sheet slot count overflows"),
            )
        })?)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sprite sheet header overflows"),
            )
        })?;
    if header_len > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite sheet header exceeds body length"),
        ));
    }

    let mut sprites = Vec::with_capacity(slot_count / 2);
    for sprite_index in 0..slot_count / 2 {
        let image_slot = sprite_index * 2;
        let mask_slot = image_slot + 1;
        let image_offset = u16_at(body, 2 + image_slot * 2) as usize;
        let mask_offset = u16_at(body, 2 + mask_slot * 2) as usize;
        if image_offset == 0 && mask_offset == 0 {
            sprites.push(None);
            continue;
        }
        if image_offset == 0 || mask_offset == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sprite {sprite_index} has only one populated slot"),
            ));
        }
        if image_offset < header_len || image_offset >= body.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{resource_name} sprite {sprite_index} has invalid image offset {image_offset}"
                ),
            ));
        }
        if mask_offset < header_len || mask_offset >= body.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{resource_name} sprite {sprite_index} has invalid mask offset {mask_offset}"
                ),
            ));
        }

        let image = parse_graphic_image_block(body, image_offset, depth, resource_name)?;
        let transparent_mask = parse_graphic_mask_block(
            body,
            mask_offset,
            image.width,
            image.height,
            resource_name,
            sprite_index,
        )?;
        sprites.push(Some(GraphicSprite {
            image,
            transparent_mask,
        }));
    }

    Ok(GraphicSpriteSheet { depth, sprites })
}

#[cfg(test)]
pub fn parse_graphic_image_block(
    body: &[u8],
    offset: usize,
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicImage> {
    let header_end = offset.checked_add(4).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image header offset overflows"),
        )
    })?;
    if header_end > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image block at {offset} is shorter than its header"),
        ));
    }
    let width = u16_at(body, offset) as usize;
    let height = u16_at(body, offset + 2) as usize;
    if width == 0 || height == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image block at {offset} has zero dimensions"),
        ));
    }
    let row_stride = graphic_image_row_stride(width, depth)?;
    let row_bytes = height.checked_mul(row_stride).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image block at {offset} row bytes overflow"),
        )
    })?;
    let data_end = header_end.checked_add(row_bytes).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image block at {offset} data end overflows"),
        )
    })?;
    if data_end > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image block at {offset} exceeds body length"),
        ));
    }
    let pixels = unpack_graphic_pixels(&body[header_end..data_end], width, height, depth)?;
    Ok(GraphicImage {
        width,
        height,
        pixels,
    })
}

#[cfg(test)]
pub fn parse_graphic_mask_block(
    body: &[u8],
    offset: usize,
    expected_width: usize,
    expected_height: usize,
    resource_name: &str,
    sprite_index: usize,
) -> io::Result<Vec<u8>> {
    let header_end = offset.checked_add(4).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite {sprite_index} mask header offset overflows"),
        )
    })?;
    if header_end > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite {sprite_index} mask is shorter than its header"),
        ));
    }
    let width = u16_at(body, offset) as usize;
    let height = u16_at(body, offset + 2) as usize;
    if width != expected_width || height != expected_height {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{resource_name} sprite {sprite_index} mask dimensions {width}x{height} do not match image {expected_width}x{expected_height}"
            ),
        ));
    }
    let pixel_count = width.checked_mul(height).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite {sprite_index} mask pixel count overflows"),
        )
    })?;
    let byte_count = pixel_count
        .checked_add(7)
        .map(|value| value / 8)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sprite {sprite_index} mask byte count overflows"),
            )
        })?;
    let data_end = header_end.checked_add(byte_count).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite {sprite_index} mask data end overflows"),
        )
    })?;
    if data_end > body.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} sprite {sprite_index} mask exceeds body length"),
        ));
    }

    let mut mask = Vec::with_capacity(pixel_count);
    for pixel in 0..pixel_count {
        let byte = body[header_end + pixel / 8];
        let bit = (byte >> (7 - (pixel % 8))) & 1;
        mask.push(bit);
    }
    Ok(mask)
}

#[cfg(test)]
pub fn graphic_image_row_stride(width: usize, depth: TileGraphicsDepth) -> io::Result<usize> {
    match depth {
        TileGraphicsDepth::Ega16 => width
            .checked_add(7)
            .map(|value| (value / 8) * 4)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "EGA row stride overflows")),
        TileGraphicsDepth::Cga4 => width
            .checked_add(3)
            .map(|value| value / 4)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "CGA row stride overflows")),
    }
}

#[cfg(test)]
pub fn unpack_graphic_pixels(
    rows: &[u8],
    width: usize,
    height: usize,
    depth: TileGraphicsDepth,
) -> io::Result<Vec<u8>> {
    let row_stride = graphic_image_row_stride(width, depth)?;
    let expected_len = height.checked_mul(row_stride).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "graphic image row byte count overflows",
        )
    })?;
    if rows.len() != expected_len {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "graphic image rows must contain {expected_len} bytes, got {}",
                rows.len()
            ),
        ));
    }
    let pixel_count = width.checked_mul(height).ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "graphic pixel count overflows")
    })?;
    let mut pixels = Vec::with_capacity(pixel_count);
    for row in 0..height {
        let row_bytes = &rows[row * row_stride..(row + 1) * row_stride];
        match depth {
            TileGraphicsDepth::Ega16 => {
                for x in 0..width {
                    let byte = row_bytes[x / 2];
                    pixels.push(if x % 2 == 0 { byte >> 4 } else { byte & 0x0f });
                }
            }
            TileGraphicsDepth::Cga4 => {
                for x in 0..width {
                    let byte = row_bytes[x / 4];
                    let shift = 6 - (x % 4) * 2;
                    pixels.push((byte >> shift) & 0x03);
                }
            }
        }
    }
    Ok(pixels)
}

#[cfg(test)]
pub fn load_title_bit(game_dir: &Path) -> io::Result<TitleBitImages> {
    parse_title_bit(&read(&game_dir.join(TITLE_BIT_FILE))?)
}

#[cfg(test)]
pub fn parse_title_bit(bytes: &[u8]) -> io::Result<TitleBitImages> {
    let body = decode_lzw_envelope(bytes, TITLE_BIT_FILE)?;
    parse_title_bit_body(&body, TITLE_BIT_FILE)
}

#[cfg(test)]
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


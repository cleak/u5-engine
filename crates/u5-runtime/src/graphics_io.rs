//! LZW codec, tile-atlas loading, GraphicImage* parsing, sprite-sheet parsing, monochrome-bitmap parsing.

use std::io;
use std::path::Path;

use crate::*;

pub fn load_tile_atlas(game_dir: &Path, depth: TileGraphicsDepth) -> io::Result<TileAtlas> {
    // `dungeon-mode.md §6.2`: the corridor billboard banks come from the
    // same directory at the same depth, so they load with the atlas.
    //
    // A fixture directory may deliberately omit all three corridor files.
    // A partial or malformed shipped set is an error rather than a silently
    // blank first-person view.
    let file_name = depth.file_name();
    let mut atlas = parse_tile_atlas(&read(&game_dir.join(file_name))?, depth, file_name)?;
    atlas.dungeon_billboards =
        crate::dungeon_view::load_optional_dungeon_billboard_banks(game_dir, depth)?;
    atlas.dungeon_sprites =
        crate::dungeon_view::load_optional_dungeon_sprite_banks(game_dir, depth)?;
    Ok(atlas)
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

    Ok(TileAtlas {
        depth,
        pixels,
        dungeon_billboards: None,
        dungeon_sprites: None,
    })
}

/// Opaque blit for tile ids in the lower 0..=255 map-cell range. Per
/// the visibility-spec active-object compositor, sprite tiles draw with
/// their full 16x16 cell including the black bounding-box pixels, so
/// this is also the path used for the avatar / NPCs / monsters once a
/// caller has resolved the tile id to a u8.
pub fn blit_tile_to_viewport(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    tile: u8,
    cell_x: usize,
    cell_y: usize,
) -> io::Result<()> {
    blit_tile_id_to_viewport(viewport, atlas, tile as usize, cell_x, cell_y)
}

/// Opaque blit accepting the full 9-bit tile id range. Active-object
/// records can address upper-half sprite ids (NPCs, monsters, vehicles,
/// the avatar) per the visibility/active-object specs; those don't fit
/// in a u8.
pub fn blit_tile_id_to_viewport(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    tile: usize,
    cell_x: usize,
    cell_y: usize,
) -> io::Result<()> {
    let tile_pixels = atlas.tile_pixels(tile).ok_or_else(|| {
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

/// Terrain blit that runs the display driver's water animator.
///
/// `cleak/u5-spec#179` (01:48, as corrected at 02:07), interim contract
/// pending the spec commit. Two stages, both off one counter and neither
/// part of the `animation.md §6` tile-id selector pass:
///
/// * the rotated ids — the three water ids and lava — take a whole-tile
///   vertical rotation of their own art;
/// * the composite destinations are rebuilt from the rotated shoals tile
///   through a mask tile out of the same shipped atlas.
///
/// Every other tile takes the ordinary path untouched. See
/// [`crate::water_scroll`] for the mechanism and its provenance.
pub fn blit_terrain_tile_to_viewport(
    viewport: &mut TileViewport,
    atlas: &TileAtlas,
    tile: usize,
    cell_x: usize,
    cell_y: usize,
    water_scroll: crate::WaterScrollClock,
) -> io::Result<()> {
    let missing = |tile: usize| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tile atlas is missing tile {tile}"),
        )
    };
    let not_one_tile = |tile: usize| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("tile {tile} is not one atlas tile of pixels"),
        )
    };
    let shift = water_scroll.row_shift();
    let Ok(terrain) = u8::try_from(tile) else {
        // An actor-bank id. Sprites are not touched by either stage.
        return blit_tile_id_to_viewport(viewport, atlas, tile, cell_x, cell_y);
    };

    // Stage two. Composited even at phase zero: the authored destination's
    // water pixels are start-up art, not a frame of the cycle, so the live
    // frame replaces them on every tick including the first.
    if let Some((mask_tile, mask_inverted)) = crate::water_composite_mask(terrain) {
        let source_id = usize::from(crate::WATER_COMPOSITE_SOURCE_TILE);
        let mask_id = usize::from(mask_tile);
        let dest = atlas.tile_pixels(tile).ok_or_else(|| missing(tile))?;
        let mask = atlas.tile_pixels(mask_id).ok_or_else(|| missing(mask_id))?;
        let source = atlas
            .tile_pixels(source_id)
            .ok_or_else(|| missing(source_id))?;
        // The rotated source frame does not advance between destinations,
        // so every composited id shows the phase the rotated tiles show.
        let rotated =
            crate::rotate_tile_rows_down(source, shift).ok_or_else(|| not_one_tile(source_id))?;
        // The mask is the mask tile's intensity plane, one boolean per pixel.
        let intensity_bit = crate::composite_mask_intensity_bit(atlas.depth.pixel_limit());
        let composed =
            crate::composite_tile_pixels(dest, mask, &rotated, mask_inverted, intensity_bit)
                .ok_or_else(|| not_one_tile(tile))?;
        return blit_tile_pixels_to_viewport(viewport, &composed, cell_x, cell_y);
    }

    // Stage one.
    if !crate::water_pass_rotates_tile(terrain) || shift == 0 {
        return blit_tile_id_to_viewport(viewport, atlas, tile, cell_x, cell_y);
    }
    let source = atlas.tile_pixels(tile).ok_or_else(|| missing(tile))?;
    let rotated = crate::rotate_tile_rows_down(source, shift).ok_or_else(|| not_one_tile(tile))?;
    blit_tile_pixels_to_viewport(viewport, &rotated, cell_x, cell_y)
}

/// Opaque blit of an already-composed tile-sized pixel block, for frames
/// that are built at draw time rather than read from the atlas.
///
/// `overworld.md §9.1` (spec HEAD `c00bf63`) needs this for the moon-gate
/// transition frame, which is composed into the dedicated scratch tile
/// `0x116` and drawn from there.
pub fn blit_tile_pixels_to_viewport(
    viewport: &mut TileViewport,
    tile_pixels: &[u8],
    cell_x: usize,
    cell_y: usize,
) -> io::Result<()> {
    if tile_pixels.len() != TILE_ATLAS_TILE_PIXELS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "composed tile needs {TILE_ATLAS_TILE_PIXELS} pixels, got {}",
                tile_pixels.len()
            ),
        ));
    }
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

pub fn tile_graphics_file_name(stem: &str, depth: TileGraphicsDepth) -> String {
    format!("{stem}.{}", depth.file_suffix())
}

pub fn load_graphic_image_directory(
    game_dir: &Path,
    stem: &str,
    depth: TileGraphicsDepth,
) -> io::Result<GraphicImageDirectory> {
    let file_name = tile_graphics_file_name(stem, depth);
    parse_graphic_image_directory(&read(&game_dir.join(&file_name))?, depth, &file_name)
}

pub fn parse_graphic_image_directory(
    bytes: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicImageDirectory> {
    let body = decode_lzw_envelope(bytes, resource_name)?;
    parse_graphic_image_directory_body(&body, depth, resource_name)
}

pub fn parse_graphic_image_directory_body(
    body: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicImageDirectory> {
    if body.len() < TILE_IMAGE_DIRECTORY_COUNT_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{resource_name} image directory is shorter than its count word"),
        ));
    }
    let count = u16_at(body, 0) as usize;
    let header_len = TILE_IMAGE_DIRECTORY_COUNT_BYTES
        .checked_add(
            count
                .checked_mul(TILE_IMAGE_DIRECTORY_OFFSET_BYTES)
                .ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("{resource_name} image directory count overflows"),
                    )
                })?,
        )
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
        let offset = u32_at(
            body,
            TILE_IMAGE_DIRECTORY_COUNT_BYTES + slot * TILE_IMAGE_DIRECTORY_OFFSET_BYTES,
        ) as usize;
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

pub fn load_graphic_sprite_sheet(
    game_dir: &Path,
    stem: &str,
    depth: TileGraphicsDepth,
) -> io::Result<GraphicSpriteSheet> {
    let file_name = tile_graphics_file_name(stem, depth);
    parse_graphic_sprite_sheet(&read(&game_dir.join(&file_name))?, depth, &file_name)
}

pub fn parse_graphic_sprite_sheet(
    bytes: &[u8],
    depth: TileGraphicsDepth,
    resource_name: &str,
) -> io::Result<GraphicSpriteSheet> {
    let body = decode_lzw_envelope(bytes, resource_name)?;
    parse_graphic_sprite_sheet_body(&body, depth, resource_name)
}

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
    // `formats/tiles.md §5.3` (spec `9807eb4`): the leading word is the
    // sprite count. Each sprite owns two 16-bit offsets, image then mask.
    // The old parser treated this as an offset count and consequently loaded
    // only half of every ITEMS/MON sheet.
    let sprite_count = u16_at(body, 0) as usize;
    let header_len = 2usize
        .checked_add(sprite_count.checked_mul(4).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("{resource_name} sprite sheet sprite count overflows"),
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

    let mut sprites = Vec::with_capacity(sprite_count);
    for sprite_index in 0..sprite_count {
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

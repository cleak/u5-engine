//! Outdoor world live 2-by-2 chunk buffer.
//!
//! The public overworld spec describes a 1 KiB live buffer made of four
//! 16-by-16 chunks. This module owns that projection separately from the
//! engine's legacy full decoded world grid.

use std::io;

use crate::*;

pub const WORLD_LIVE_CHUNK_QUADRANTS: [(usize, usize); OVERWORLD_CHUNK_BUFFER_CHUNKS] =
    [(0, 0), (1, 0), (0, 1), (1, 1)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldChunkDescriptor {
    pub plane: WorldPlane,
    pub logical_slot: usize,
    pub file_index: Option<u8>,
    pub all_water: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldLiveChunkBuffer {
    pub plane: WorldPlane,
    pub scroll_base: (usize, usize),
    pub chunks: [u8; OVERWORLD_CHUNK_BUFFER_BYTES],
    pub descriptors: [WorldChunkDescriptor; OVERWORLD_CHUNK_BUFFER_CHUNKS],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldChunkShuffle {
    pub buffer: WorldLiveChunkBuffer,
    pub copied_quadrants: [bool; OVERWORLD_CHUNK_BUFFER_CHUNKS],
}

impl WorldLiveChunkBuffer {
    pub fn from_full_grid<F>(
        plane: WorldPlane,
        grid: &[u8],
        player_x: usize,
        player_y: usize,
        mut chunk_classifier_accepts: F,
    ) -> io::Result<Self>
    where
        F: FnMut(WorldChunkDescriptor) -> bool,
    {
        if grid.len() != WORLD_CELLS {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("world grid must be {WORLD_CELLS} bytes, got {}", grid.len()),
            ));
        }
        let scroll_base = world_scroll_base(player_x, player_y);
        let mut buffer = Self::blank(plane, scroll_base);
        for quadrant in 0..OVERWORLD_CHUNK_BUFFER_CHUNKS {
            let descriptor = live_quadrant_descriptor(plane, scroll_base, quadrant, None, false);
            let substitute_19 = chunk_classifier_accepts(descriptor);
            buffer.descriptors[quadrant] = descriptor;
            copy_live_chunk_from_full_grid(
                grid,
                &mut buffer.chunks,
                scroll_base,
                quadrant,
                substitute_19,
            );
        }
        Ok(buffer)
    }

    pub fn from_underworld_bytes<F>(
        bytes: &[u8],
        player_x: usize,
        player_y: usize,
        mut chunk_classifier_accepts: F,
    ) -> io::Result<Self>
    where
        F: FnMut(WorldChunkDescriptor) -> bool,
    {
        if bytes.len() != UNDER_DAT_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{UNDER_DAT_FILENAME} must be {UNDER_DAT_LEN} bytes, got {}",
                    bytes.len()
                ),
            ));
        }
        let scroll_base = world_scroll_base(player_x, player_y);
        let mut buffer = Self::blank(WorldPlane::Underworld, scroll_base);
        for quadrant in 0..OVERWORLD_CHUNK_BUFFER_CHUNKS {
            let descriptor = live_quadrant_descriptor(
                WorldPlane::Underworld,
                scroll_base,
                quadrant,
                None,
                false,
            );
            let substitute_19 = chunk_classifier_accepts(descriptor);
            buffer.descriptors[quadrant] = descriptor;
            copy_underworld_live_chunk(
                bytes,
                &mut buffer.chunks,
                scroll_base,
                quadrant,
                substitute_19,
            );
        }
        Ok(buffer)
    }

    pub fn from_britannia_bytes<F>(
        brit_bytes: &[u8],
        chunk_index: &[u8],
        player_x: usize,
        player_y: usize,
        mut chunk_classifier_accepts: F,
    ) -> io::Result<Self>
    where
        F: FnMut(WorldChunkDescriptor) -> bool,
    {
        if brit_bytes.len() != BRIT_DAT_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{BRIT_DAT_FILENAME} must be {BRIT_DAT_LEN} bytes, got {}",
                    brit_bytes.len()
                ),
            ));
        }
        validate_britannia_chunk_index(chunk_index)?;
        let scroll_base = world_scroll_base(player_x, player_y);
        let mut buffer = Self::blank(WorldPlane::Britannia, scroll_base);
        for quadrant in 0..OVERWORLD_CHUNK_BUFFER_CHUNKS {
            let slot = live_quadrant_logical_slot(scroll_base, quadrant);
            let table_entry = chunk_index[slot];
            let all_water = table_entry == BRIT_WATER_SENTINEL;
            let file_index = (!all_water).then_some(table_entry);
            let descriptor = live_quadrant_descriptor(
                WorldPlane::Britannia,
                scroll_base,
                quadrant,
                file_index,
                all_water,
            );
            let substitute_19 = chunk_classifier_accepts(descriptor);
            buffer.descriptors[quadrant] = descriptor;
            copy_britannia_live_chunk(
                brit_bytes,
                &mut buffer.chunks,
                quadrant,
                file_index,
                substitute_19,
            );
        }
        Ok(buffer)
    }

    pub fn tile_at(&self, world_x: usize, world_y: usize) -> u8 {
        self.chunks[live_buffer_index(self.scroll_base, world_x, world_y)]
    }

    pub fn contains_world_tile(&self, world_x: usize, world_y: usize) -> bool {
        world_scroll_axis_offset(self.scroll_base.0, world_x) < OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE
            && world_scroll_axis_offset(self.scroll_base.1, world_y)
                < OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE
    }

    pub fn quadrant_chunk_origin(&self, quadrant: usize) -> (usize, usize) {
        live_quadrant_chunk_origin(self.scroll_base, quadrant)
    }

    pub fn shuffled_to_scroll_base(&self, new_scroll_base: (usize, usize)) -> WorldChunkShuffle {
        let mut next = Self::blank(self.plane, new_scroll_base);
        let mut copied_quadrants = [false; OVERWORLD_CHUNK_BUFFER_CHUNKS];
        for new_quadrant in 0..OVERWORLD_CHUNK_BUFFER_CHUNKS {
            let new_origin = live_quadrant_chunk_origin(new_scroll_base, new_quadrant);
            if let Some(old_quadrant) = (0..OVERWORLD_CHUNK_BUFFER_CHUNKS)
                .find(|&old| live_quadrant_chunk_origin(self.scroll_base, old) == new_origin)
            {
                let new_start = new_quadrant * CHUNK_BYTES;
                let old_start = old_quadrant * CHUNK_BYTES;
                next.chunks[new_start..new_start + CHUNK_BYTES]
                    .copy_from_slice(&self.chunks[old_start..old_start + CHUNK_BYTES]);
                next.descriptors[new_quadrant] = self.descriptors[old_quadrant];
                copied_quadrants[new_quadrant] = true;
            }
        }
        WorldChunkShuffle {
            buffer: next,
            copied_quadrants,
        }
    }

    pub fn shuffled_to_party_position(
        &self,
        player_x: usize,
        player_y: usize,
    ) -> WorldChunkShuffle {
        self.shuffled_to_scroll_base(world_scroll_base(player_x, player_y))
    }

    fn blank(plane: WorldPlane, scroll_base: (usize, usize)) -> Self {
        let empty_descriptor = WorldChunkDescriptor {
            plane,
            logical_slot: 0,
            file_index: None,
            all_water: false,
        };
        Self {
            plane,
            scroll_base,
            chunks: [0; OVERWORLD_CHUNK_BUFFER_BYTES],
            descriptors: [empty_descriptor; OVERWORLD_CHUNK_BUFFER_CHUNKS],
        }
    }
}

pub fn load_world_live_chunk_buffer<F>(
    game_dir: &std::path::Path,
    plane: WorldPlane,
    player_x: usize,
    player_y: usize,
    chunk_classifier_accepts: F,
) -> io::Result<WorldLiveChunkBuffer>
where
    F: FnMut(WorldChunkDescriptor) -> bool,
{
    let bytes = read(&game_dir.join(plane.file_name()))?;
    match plane {
        WorldPlane::Underworld => WorldLiveChunkBuffer::from_underworld_bytes(
            &bytes,
            player_x,
            player_y,
            chunk_classifier_accepts,
        ),
        WorldPlane::Britannia => {
            let data = read(&game_dir.join(DATA_OVL_FILENAME))?;
            let chunk_index = find_britannia_chunk_index(&data)?;
            WorldLiveChunkBuffer::from_britannia_bytes(
                &bytes,
                &chunk_index,
                player_x,
                player_y,
                chunk_classifier_accepts,
            )
        }
    }
}

pub fn live_buffer_index(scroll_base: (usize, usize), world_x: usize, world_y: usize) -> usize {
    let local_x =
        world_scroll_axis_offset(scroll_base.0, world_x) & (OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE - 1);
    let local_y =
        world_scroll_axis_offset(scroll_base.1, world_y) & (OVERWORLD_CHUNK_BUFFER_WINDOW_SIDE - 1);
    let quadrant = live_local_quadrant(local_x, local_y);
    quadrant * CHUNK_BYTES + (local_y % CHUNK_SIDE) * CHUNK_SIDE + (local_x % CHUNK_SIDE)
}

pub fn live_local_quadrant(local_x: usize, local_y: usize) -> usize {
    (usize::from(local_y >= CHUNK_SIDE) * OVERWORLD_CHUNK_BUFFER_GRID_SIDE)
        + usize::from(local_x >= CHUNK_SIDE)
}

pub fn live_quadrant_chunk_origin(scroll_base: (usize, usize), quadrant: usize) -> (usize, usize) {
    let (qx, qy) = WORLD_LIVE_CHUNK_QUADRANTS[quadrant];
    (
        (scroll_base.0 + qx * CHUNK_SIDE) % WORLD_SIDE,
        (scroll_base.1 + qy * CHUNK_SIDE) % WORLD_SIDE,
    )
}

pub fn live_quadrant_logical_slot(scroll_base: (usize, usize), quadrant: usize) -> usize {
    let (x, y) = live_quadrant_chunk_origin(scroll_base, quadrant);
    (y / CHUNK_SIDE) * WORLD_CHUNKS_PER_SIDE + (x / CHUNK_SIDE)
}

fn live_quadrant_descriptor(
    plane: WorldPlane,
    scroll_base: (usize, usize),
    quadrant: usize,
    file_index: Option<u8>,
    all_water: bool,
) -> WorldChunkDescriptor {
    WorldChunkDescriptor {
        plane,
        logical_slot: live_quadrant_logical_slot(scroll_base, quadrant),
        file_index,
        all_water,
    }
}

fn copy_live_chunk_from_full_grid(
    grid: &[u8],
    out: &mut [u8; OVERWORLD_CHUNK_BUFFER_BYTES],
    scroll_base: (usize, usize),
    quadrant: usize,
    substitute_19: bool,
) {
    let (origin_x, origin_y) = live_quadrant_chunk_origin(scroll_base, quadrant);
    let dst_start = quadrant * CHUNK_BYTES;
    for local_y in 0..CHUNK_SIDE {
        for local_x in 0..CHUNK_SIDE {
            let wx = (origin_x + local_x) % WORLD_SIDE;
            let wy = (origin_y + local_y) % WORLD_SIDE;
            let tile = grid[world_cell_index(wx, wy)];
            out[dst_start + local_y * CHUNK_SIDE + local_x] =
                live_chunk_substituted_tile(tile, substitute_19);
        }
    }
}

fn copy_underworld_live_chunk(
    bytes: &[u8],
    out: &mut [u8; OVERWORLD_CHUNK_BUFFER_BYTES],
    scroll_base: (usize, usize),
    quadrant: usize,
    substitute_19: bool,
) {
    let (origin_x, origin_y) = live_quadrant_chunk_origin(scroll_base, quadrant);
    let dst_start = quadrant * CHUNK_BYTES;
    for local_y in 0..CHUNK_SIDE {
        for local_x in 0..CHUNK_SIDE {
            let wx = (origin_x + local_x) % WORLD_SIDE;
            let wy = (origin_y + local_y) % WORLD_SIDE;
            let src = under_file_offset(wx as u8, wy as u8);
            out[dst_start + local_y * CHUNK_SIDE + local_x] =
                live_chunk_substituted_tile(bytes[src], substitute_19);
        }
    }
}

fn copy_britannia_live_chunk(
    brit_bytes: &[u8],
    out: &mut [u8; OVERWORLD_CHUNK_BUFFER_BYTES],
    quadrant: usize,
    file_index: Option<u8>,
    substitute_19: bool,
) {
    let dst_start = quadrant * CHUNK_BYTES;
    if let Some(file_index) = file_index {
        let src_start = file_index as usize * CHUNK_BYTES;
        for offset in 0..CHUNK_BYTES {
            out[dst_start + offset] =
                live_chunk_substituted_tile(brit_bytes[src_start + offset], substitute_19);
        }
    } else {
        out[dst_start..dst_start + CHUNK_BYTES].fill(BRIT_DEEP_WATER_TILE);
    }
}

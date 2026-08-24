//! World live-chunk-buffer integration helpers.

use std::io;

use crate::*;

impl PlayState {
    pub fn rebuild_world_live_chunks_from_grid(&mut self, plane: WorldPlane) -> io::Result<()> {
        self.world_live_chunks = Some(WorldLiveChunkBuffer::from_full_grid(
            plane,
            &self.grid,
            self.player.x,
            self.player.y,
            |_| LiveChunkSubstitutionPolicy::NONE,
        )?);
        // `visibility.md §12.6`: the beacon's outdoor source is harvested by
        // the chunk loader, not by the light pass — "the chunk loader scans
        // each freshly loaded thirty-two-by-thirty-two window for the
        // lighthouse tile". The Underworld map holds no lighthouse, so this
        // records the "no beacon" sentinel there every time.
        self.harvest_outdoor_light_beacon();
        Ok(())
    }

    pub fn refresh_world_live_chunks_for_current_area(&mut self) -> io::Result<()> {
        if let Area::World { plane } = self.area {
            self.rebuild_world_live_chunks_from_grid(plane)
        } else {
            self.world_live_chunks = None;
            // Off the outdoor map there is no window to scan; the location
            // and combat paths own the beacon's sources instead
            // (`visibility.md §12.6`).
            self.light_beacon.sources = [None; BEACON_SOURCE_SLOTS];
            Ok(())
        }
    }

    pub fn world_live_tile_at(&self, x: usize, y: usize) -> u8 {
        if let Some(buffer) = &self.world_live_chunks {
            if matches!(self.area, Area::World { plane } if plane == buffer.plane)
                && buffer.contains_world_tile(x, y)
            {
                return buffer.tile_at(x, y);
            }
        }
        self.grid[world_cell_index(x, y)]
    }
}

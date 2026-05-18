
use crate::*;

impl PlayState {
    pub fn npc_can_step(&self, npc_index: usize, x: usize, y: usize, floor: u8) -> bool {
        let wp = waypoint_for_hour(&self.npcs[npc_index].schedule, self.clock.hour);
        let (dest_x, dest_y, _) = self.npcs[npc_index].waypoint_position(wp);
        self.npc_can_step_toward(npc_index, x, y, floor, dest_x, dest_y)
    }

    pub fn npc_can_step_toward(
        &self,
        npc_index: usize,
        x: usize,
        y: usize,
        floor: u8,
        destination_x: usize,
        destination_y: usize,
    ) -> bool {
        if x >= 32 || y >= 32 {
            return false;
        }
        let coordinate_goal = (x, y) == (destination_x, destination_y);
        if !coordinate_goal && !npc_path_tile_open(self.grid[y * 32 + x]) {
            return false;
        }

        if (x, y) == (self.player.x, self.player.y) {
            return false;
        }

        let own_active_object = self.npcs[npc_index].active_object;
        !self
            .active_objects
            .iter()
            .enumerate()
            .any(|(slot, object)| {
                Some(slot) != own_active_object
                    && !object.is_empty()
                    && object.x == x
                    && object.y == y
                    && object.z == floor as i8
                    && npc_dynamic_obstacle_blocks(
                        object.x as i32,
                        object.y as i32,
                        destination_x as i32,
                        destination_y as i32,
                    )
            })
    }
}

impl PlayState {
    pub fn npc_can_step(&self, npc_index: usize, x: usize, y: usize, floor: u8) -> bool {
        if x >= 32
            || y >= 32
            || !is_tile_walkable_for_transport(
                self.grid[y * 32 + x],
                self.passability.as_ref(),
                TransportState::Foot,
            )
        {
            return false;
        }
        if (x, y) == (self.player.x, self.player.y) {
            return false;
        }
        !self
            .npcs
            .iter()
            .enumerate()
            .any(|(index, npc)| index != npc_index && npc.x == x && npc.y == y && npc.z == floor)
    }
}

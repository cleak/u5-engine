use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorationTurnGateOutcome {
    Ready { member_index: usize },
    Slept { transition: Option<MoveOutcome> },
    Rescued { transition: MoveOutcome },
}

impl PlayState {
    pub fn party_capability(&self) -> PartyCapability {
        party_capability(&self.party)
    }

    /// Run the shared exploration-loop gate immediately before a command is
    /// accepted. Frontends call this only between commands, never while a
    /// prompt or blocking presentation owns input.
    pub fn apply_exploration_turn_gate(
        &mut self,
        game_dir: &std::path::Path,
    ) -> std::io::Result<ExplorationTurnGateOutcome> {
        match self.party_capability() {
            PartyCapability::CanAct { member_index } => {
                self.active_player = Some(member_index);
                Ok(ExplorationTurnGateOutcome::Ready { member_index })
            }
            PartyCapability::Sleeping => {
                let turn_before = self.turn;
                if matches!(self.area, Area::World { .. }) {
                    // `overworld.md` capability gate (spec HEAD d3863ef):
                    // the no-input sleeping branch is an ordinary consumed
                    // world turn, but its clock increment is two minutes.
                    self.advance_turn_with_minutes(2);
                } else {
                    self.advance_turn();
                }
                self.message = PARTY_SLEEP_LINE.to_string();
                let transition = match self.area {
                    Area::Town { .. } => {
                        self.apply_town_post_turn_effects_after_turn(turn_before, game_dir)?
                    }
                    // The same clarified contract requires the full ordinary
                    // no-input tail: environment/transport hooks, party
                    // status/provisions, encounter probe, and the normally
                    // gated active-object walk/prune. `advance_turn` owns the
                    // time/status/object portion; this helper owns the
                    // I/O-bearing underfoot and encounter portion.
                    Area::World { .. } => {
                        self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?
                    }
                    // `dungeon-mode.md §4`: sleeping repeats without the
                    // dungeon post-action helper; the next loop head still
                    // pays the ordinary one-minute cleanup.
                    Area::Dungeon { .. } => {
                        self.append_pending_hourly_status_message();
                        None
                    }
                };
                Ok(ExplorationTurnGateOutcome::Slept { transition })
            }
            PartyCapability::Defeated => {
                match self.area {
                    Area::World { .. } => {
                        self.write_world_defeat_active_object_table(game_dir)?;
                    }
                    Area::Dungeon { .. } => {
                        // `dungeon-mode.md` capability gate (spec HEAD
                        // d3863ef): graphics teardown is transient and draws
                        // nothing. This engine keeps the ordinary tile atlas
                        // resident and owns no dungeon-only corridor/item or
                        // monster-bank references, so every required resource
                        // postcondition already holds here. In particular, do
                        // not mutate map, party, position, objects, clock, or
                        // PRNG before the immediate rescue below.
                    }
                    Area::Town { .. } => {}
                }
                let transition = self.apply_blackthorn_rescue_refuge(game_dir)?;
                Ok(ExplorationTurnGateOutcome::Rescued { transition })
            }
        }
    }

    /// `overworld.md` capability gate (spec HEAD d3863ef): defeat writes the
    /// complete live 32-record table, including slot zero, to the mirror for
    /// the current plane after requesting Britannia gameplay resources. This
    /// is deliberately separate from the ordinary animate/prune epilogue and
    /// takes `&self` so the write cannot classify, reorder, or mutate records.
    pub fn write_world_defeat_active_object_table(
        &self,
        game_dir: &std::path::Path,
    ) -> std::io::Result<()> {
        let Area::World { plane } = self.area else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "world defeat object persistence requires world mode",
            ));
        };
        let mut disk_session = DiskPromptSession::single_directory();
        disk_session.request_operation(DiskOperationFamily::GameplayResources);
        // BRIT.DAT availability is the gameplay-disk readiness probe even in
        // the underworld; plane selection applies only to the OOL destination.
        let _ = read_disk_file(&game_dir.join(BRIT_DAT_FILENAME))?;
        let bytes = encode_active_object_table(&self.active_objects)?;
        let file_name = match plane {
            WorldPlane::Britannia => BRIT_OOL_FILENAME,
            WorldPlane::Underworld => UNDER_OOL_FILENAME,
        };
        write_disk_file(&game_dir.join(file_name), bytes)?;
        Ok(())
    }

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
        // npc-schedules.md §10: the waypoint-match rule reports the NPC's
        // own active waypoint open regardless of tile id; otherwise a set
        // bit in the NPC tile set marks the cell as an obstacle.
        let coordinate_goal = (x, y) == (destination_x, destination_y);
        if !coordinate_goal && npc_path_tile_obstacle(self.grid[y * 32 + x]) {
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

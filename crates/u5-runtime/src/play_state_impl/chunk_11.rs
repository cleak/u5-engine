use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorationTurnGateOutcome {
    Ready {
        member_index: usize,
    },
    Slept {
        transition: Option<MoveOutcome>,
    },
    /// A rescue cinematic is running. `blackthorn.md §7` puts its handoff at
    /// the *end*, after nine beats and a blocking key, so the transition
    /// arrives from `step_blackthorn_rescue` rather than from this gate and
    /// is `None` here.
    Rescued {
        transition: Option<MoveOutcome>,
    },
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
        // `stats-panel.md §2.3`: the deferred request is consumed at "the
        // head of that loop's command prompt, not at the end of a turn" -
        // test, refresh if set, clear. This gate is that position for the
        // three exploration modes.
        self.drain_stats_panel_refresh();
        // `karma.md §7`: the shrine display outlives the meditation handler -
        // the exit pacing "re-stamps the walking pose, spends one world step"
        // and walks the avatar back down before the mode loop's prompt. So it
        // is released once the session has ended *and* that staged hold has
        // run out, not on the handler's return.
        // The Codex reader (§8.2) holds the same display through its key
        // waits, so an open reader keeps it too.
        if self.active_shrine.is_none()
            && self.codex_presentation_steps == 0
            && !self.staged_narration_active()
        {
            self.shrine_presentation_map = None;
            self.shrine_approach_walk = None;
        }
        // A rescue cinematic is already running for this defeat. The roster
        // stays Defeated until its restoration loop (`blackthorn.md §7` step
        // 22), so without this the gate would start a second rescue on every
        // frame of the first one.
        if self.pending_blackthorn_rescue.is_some() {
            return Ok(ExplorationTurnGateOutcome::Rescued { transition: None });
        }
        match self.party_capability() {
            PartyCapability::CanAct { member_index } => {
                // The gate reports who can act; it does not *select* anybody.
                // `stats-panel.md §4.1`/§11: the active-player marker is drawn
                // "while a member is selected", the selector is persistent and
                // is "cleared only by an explicit selection change or by the
                // dead/sleeping rule", and no published contract has an
                // ordinary turn set it. Writing it here put the roster arrow
                // permanently on the first able member - a marker the original
                // does not show in ordinary overworld play.
                Ok(ExplorationTurnGateOutcome::Ready { member_index })
            }
            PartyCapability::Sleeping => {
                let turn_before = self.turn;
                match self.area {
                    // `overworld.md` capability gate (spec HEAD d3863ef):
                    // the no-input sleeping branch is an ordinary consumed
                    // world turn, but its clock increment is two minutes.
                    Area::World { .. } => self.advance_turn_with_minutes(2),
                    Area::Town { .. } => self.advance_turn(),
                    // `dungeon-mode.md §4` step 7: the asleep result "re-runs
                    // the check without reading a command, repeating for as
                    // long as that result holds", so the loop head that owns
                    // the indoor clock advance is never reached and nothing
                    // in this arm can wake anybody.
                    //
                    // **Measured** 2026-10-19, pending `cleak/u5-spec#299`
                    // Q23 (`qa/paired/asleep-dungeon-clock.tsv`): the
                    // asleep-dungeon party loaded at 23:55 still shows
                    // `4-5-139` after 150 s of continuous passes, so the day
                    // rollover's panel repaint (`stats-panel.md §2.4`) never
                    // ran - the passes advance no time. With no clock there is
                    // no hour, turn count or upkeep roll to wake on, and
                    // `asleep-dungeon` confirms none fires: the stock party
                    // stays `S` through five minutes and twelve Space presses.
                    // An all-asleep dungeon party is a softlock in the
                    // original too.
                    Area::Dungeon { .. } => {}
                }
                self.emit_party_sleep_line();
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
                    // dungeon post-action helper, and (above) without the
                    // loop head's clock.
                    Area::Dungeon { .. } => None,
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
                // The transition is the cinematic's *last* step, not its
                // first: `blackthorn.md §7` runs nine narrative beats and a
                // blocking key read before the handoff. `step_blackthorn_rescue`
                // reports it when it gets there.
                let _ = self.apply_blackthorn_rescue_refuge(game_dir)?;
                Ok(ExplorationTurnGateOutcome::Rescued { transition: None })
            }
        }
    }

    /// Whether an exploration loop is sitting in its all-asleep arm: every
    /// member unable to act, at least one Sleeping, no combat arena and no
    /// rescue running. That arm reads no command (`main-loop.md §6`).
    pub fn party_sleeping_without_input(&self) -> bool {
        !self.combat_active
            && self.pending_blackthorn_rescue.is_none()
            && self.party_capability() == PartyCapability::Sleeping
    }

    /// Print one pass's sleep line as a row of its own.
    ///
    /// `main-loop.md §6` has town and overworld emit the prompt's
    /// line-feed-and-marker pair before the sleep line, "so a slept-through
    /// turn advances the message window exactly as a polled turn would", and
    /// `dungeon-mode.md §4` step 7 has the dungeon emit its own line feed
    /// before `Zzzzzz...\n` on every repeat.
    ///
    /// **Measured** 2026-10-17, pending `cleak/u5-spec#299` Q23
    /// (`qa/paired/asleep-dungeon.tsv`, artifact
    /// `asleep-dungeon-20261017-221915`): with every member Sleeping in a
    /// dungeon the stock game prints a fresh `>Zzzzzz...` row - end-cap
    /// marker on it, one blank row between repeats - on every pass, and the
    /// window keeps scrolling for the whole five-minute capture. This engine
    /// assigned the same literal to the message slot each pass, which the
    /// slot flush skips as already printed, so it showed a single
    /// `Zzzzzz...` under a live command row and looked frozen (`F14`,
    /// `cleak/u5-engine#47`). Each pass is its own emission now.
    pub(crate) fn emit_party_sleep_line(&mut self) {
        // Anything the previous turn left in the slot prints above this row.
        self.flush_message_slot();
        self.emit_command_echo_line(PARTY_SLEEP_LINE);
        // The literal ends with its own line feed (`Zzzzzz...\n`), and the
        // pass then waits with the cursor on the fresh row that feed opened.
        // Measured 2026-10-20 (`asleep-dungeon`, idle beats): the stock
        // window between passes ends `Zzzzzz...` on the second-to-last row
        // with the bottom row blank; this engine left the sleep line on the
        // bottom row. The row is the blank the next pass's leading feed
        // would otherwise derive, so the next command row adds none.
        self.push_explicit_blank_message_entry();
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

    /// `npc-schedules.md §10` "The per-step gate": "is the cell occupied by
    /// another active object. An occupied cell is refused even where the
    /// terrain allows it." ... "the per-step gate applies no cutoff, so a
    /// route planned around a far-off actor can still be refused a step when
    /// that actor has since moved into the way."
    pub fn npc_can_step_toward(
        &self,
        npc_index: usize,
        x: usize,
        y: usize,
        floor: u8,
        destination_x: usize,
        destination_y: usize,
    ) -> bool {
        self.npc_cell_open(npc_index, x, y, floor, destination_x, destination_y, false)
    }

    /// `npc-schedules.md §8.3` phase 4 (issue #298 Q22, `RETRACTIONS.md`
    /// R536): the pathfinding workspace's dynamic-obstacle overlay "walks the
    /// active-object table from record 31 down to record 1 and, for each
    /// record whose type byte is above `0x3F` and whose Manhattan distance
    /// from the NPC's current (x, y) is strictly less than four, stamps the
    /// object's cell as an obstacle ... The player's position is then
    /// included through the same distance rule." The destination arguments
    /// feed only the §10 waypoint-match rule.
    pub fn npc_workspace_cell_open(
        &self,
        npc_index: usize,
        x: usize,
        y: usize,
        floor: u8,
        destination_x: usize,
        destination_y: usize,
    ) -> bool {
        self.npc_cell_open(npc_index, x, y, floor, destination_x, destination_y, true)
    }

    #[allow(clippy::too_many_arguments)]
    fn npc_cell_open(
        &self,
        npc_index: usize,
        x: usize,
        y: usize,
        floor: u8,
        destination_x: usize,
        destination_y: usize,
        destination_cutoff: bool,
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

        let origin = (self.npcs[npc_index].x as i32, self.npcs[npc_index].y as i32);
        if (x, y) == (self.player.x, self.player.y)
            && (!destination_cutoff
                || npc_dynamic_obstacle_blocks(x as i32, y as i32, origin.0, origin.1))
        {
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
                    && (!destination_cutoff
                        || npc_workspace_overlay_stamps(slot, object.type_byte)
                            && npc_dynamic_obstacle_blocks(
                                object.x as i32,
                                object.y as i32,
                                origin.0,
                                origin.1,
                            ))
            })
    }
}

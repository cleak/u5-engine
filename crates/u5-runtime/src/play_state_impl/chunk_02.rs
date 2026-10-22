use std::io;
use std::path::Path;

use crate::*;

/// `karma.md §8.1` / `audio.md §8.4`: the completed Codex read runs "three
/// shared viewport flash/rumble effects" before page-turn record `40`.
const CODEX_COMPLETION_MAJOR_FLASHES: usize = 3;

impl PlayState {
    pub fn step_non_town(
        &mut self,
        direction: Direction,
        nx: isize,
        ny: isize,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Dungeon { scene, level } => {
                self.step_dungeon(direction, nx, ny, scene, level, game_dir)
            }
            Area::World { plane } => self.step_world(direction, nx, ny, plane, game_dir),
            Area::Town { .. } => unreachable!("town step handled before non-town dispatch"),
        }
    }

    #[cfg(test)]
    pub fn handle_dungeon_key(&mut self, key: char, game_dir: &Path) -> io::Result<bool> {
        self.handle_dungeon_key_with_inline(key, game_dir, None, None, None, None, None)
    }

    /// `commands.md §5`: every dungeon command echoes its resident verb
    /// prefix before the handler prompts or refuses. The echo is opened
    /// speculatively — a key this mode does not handle rolls it back —
    /// and the handler's own prompt or result is folded into it on the
    /// way out. See [`PlayState::begin_command_echo`].
    pub fn handle_dungeon_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_rest: impl Into<InlineRestRequest>,
        inline_drink: Option<bool>,
        inline_party_index: Option<usize>,
        inline_use_request: Option<UseItemRequest>,
        inline_look_focus: Option<DungeonLookFocus>,
    ) -> io::Result<bool> {
        if !matches!(self.area, Area::Dungeon { .. }) {
            return Ok(false);
        }
        if let Some(echo) = dungeon_command_echo(key) {
            self.begin_command_echo(echo);
        }
        let handled = self.handle_dungeon_key_with_inline_inner(
            key,
            game_dir,
            inline_rest,
            inline_drink,
            inline_party_index,
            inline_use_request,
            inline_look_focus,
        );
        match &handled {
            Ok(true) => {
                self.commit_command_echo();
            }
            _ => self.abort_command_echo(),
        }
        handled
    }

    /// `dungeon-mode.md §10` (u5-spec#298 Q28, R530): X on foot underground
    /// prints `X-it what?\n` and, by static reading, reports **acted**, so
    /// the dungeon post-action pass runs. X-it has no working scene refusal.
    fn dungeon_exit_vehicle_command(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        let on_foot = self.player.transport.is_foot();
        let outcome = self.exit_vehicle_with_game_dir(Some(game_dir))?;
        if on_foot {
            self.advance_turn();
        }
        Ok(outcome)
    }

    fn handle_dungeon_key_with_inline_inner(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_rest: impl Into<InlineRestRequest>,
        inline_drink: Option<bool>,
        inline_party_index: Option<usize>,
        inline_use_request: Option<UseItemRequest>,
        inline_look_focus: Option<DungeonLookFocus>,
    ) -> io::Result<bool> {
        let inline_rest = inline_rest.into();
        // `dungeon-mode.md §15`: "The single call site sits at the head of
        // each iteration, ahead of the render-and-poll step and the command
        // dispatch, so a command the dispatcher reports as \"no action\" …
        // still costs a minute underground." The room-trigger resolution below
        // is part of the iteration and therefore sits behind it.
        self.advance_dungeon_loop_minute();
        if self
            .resolve_current_dungeon_room_trigger(Some(game_dir))?
            .is_some()
        {
            return Ok(true);
        }
        // `§10`: "Only the dungeon post-action pass is gated on the status
        // word." The snapshot is taken after the loop-head minute so the pass
        // still keys on whether the command itself consumed a turn.
        let turn_before = self.turn;
        macro_rules! handled {
            ($outcome:expr) => {{
                let outcome = $outcome;
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
                return Ok(true);
            }};
            () => {{
                self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
                return Ok(true);
            }};
        }

        if key == 'S' {
            let outcome = if let Some(focus) = inline_look_focus {
                self.search_dungeon_focus_with_game_dir(focus, game_dir)?
            } else {
                self.start_dungeon_search_prompt()
            };
            handled!(outcome);
        }
        if key == 'A' {
            handled!(self.attack_command_with_game_dir(None, Some(game_dir))?);
        }
        if matches!(key, 'D' | 'W') {
            self.message = unassigned_refusal_echo(key as u8).to_string();
            handled!();
        }
        if let Some(direction) = high_byte_direction_from_key(key) {
            match direction {
                Direction::North => {
                    handled!(self.step_with_game_dir(self.player.facing, Some(game_dir))?);
                }
                Direction::South => {
                    let facing = self.player.facing;
                    let outcome = if let Some(direction) = facing.opposite_cardinal() {
                        self.step_dungeon_back_with_game_dir(direction, Some(game_dir))?
                    } else {
                        self.message =
                            "Dungeon back-step requires a cardinal facing direction.".to_string();
                        MoveOutcome::Blocked
                    };
                    self.player.facing = facing;
                    handled!(outcome);
                }
                Direction::West => {
                    handled!(self.turn_dungeon(false));
                }
                Direction::East => {
                    handled!(self.turn_dungeon(true));
                }
                Direction::NorthWest
                | Direction::NorthEast
                | Direction::SouthWest
                | Direction::SouthEast => {
                    self.message =
                        "Dungeon movement supports forward, back, and turns only.".to_string();
                    handled!();
                }
            }
        }
        // `dungeon-mode.md §4` step 3 and `§10`: digits go to the
        // solo-member selector, "always reporting \"no action\"
        // afterwards". They are never movement underground: only the four
        // cardinal codes, Enter and `.` move the party.
        if key.is_ascii_digit() {
            let _ = self.set_active_player_command(key);
            handled!();
        }
        match key.to_ascii_lowercase() {
            'w' | '.' | '\r' | '\n' => {
                handled!(self.step_with_game_dir(self.player.facing, Some(game_dir))?);
            }
            's' => {
                let facing = self.player.facing;
                let outcome = if let Some(direction) = facing.opposite_cardinal() {
                    self.step_dungeon_back_with_game_dir(direction, Some(game_dir))?
                } else {
                    self.message =
                        "Dungeon back-step requires a cardinal facing direction.".to_string();
                    MoveOutcome::Blocked
                };
                self.player.facing = facing;
                handled!(outcome);
            }
            'a' => {
                handled!(self.turn_dungeon(false));
            }
            'd' => {
                handled!(self.turn_dungeon(true));
            }
            // `dungeon-mode.md §13.1`: every climbing arm reports "acted",
            // so the post-action pass reads the cell the climb landed on.
            'k' | '<' | '>' => {
                let outcome = match key {
                    '<' => self.climb(game_dir, ClimbIntent::Up)?,
                    '>' => self.climb(game_dir, ClimbIntent::Down)?,
                    _ => self.klimb_command(game_dir)?,
                };
                self.apply_post_turn_effects_after_dungeon_level_change(turn_before, game_dir, outcome)?;
                return Ok(true);
            }
            'l' => {
                let outcome = if let Some(focus) = inline_look_focus {
                    self.look_dungeon_with_focus(inline_drink, inline_party_index, focus)
                } else {
                    self.start_dungeon_look_prompt(inline_party_index, inline_drink)
                };
                handled!(outcome);
            }
            'g' => {
                let Area::Dungeon { scene, level } = self.area else {
                    unreachable!("dungeon key handler is gated to dungeon scenes");
                };
                handled!(self.get_dungeon_underfoot_with_game_dir(Some(game_dir), scene, level)?);
            }
            'i' => {
                handled!(self.ignite_torch());
            }
            'o' => {
                handled!(self.open_facing_with_game_dir(Some(game_dir))?);
            }
            'v' => {
                handled!(self.view_gem());
            }
            't' => {
                self.message = "Funny, no response!".to_string();
                handled!();
            }
            // `vehicles.md §4.1` (issue #299 Q25): "Dungeon scene |
            // `\nNot here!\n` | `Board`, then `Not here!`".
            'b' => {
                self.message = BOARD_NOT_HERE_REFUSAL.to_string();
                self.message_row_open_mid_line = false;
                handled!();
            }
            // `commands.md §7` / `active-objects.md §10`: X-it reaches its
            // ordinary handler from a dungeon scene; the party is on foot
            // underground, so it refuses with the shared `X-it what?`
            // (`commands.md §5.8`). The dungeon-specific refusal never prints.
            'x' => {
                handled!(self.dungeon_exit_vehicle_command(game_dir)?);
            }
            // `commands.md §5.3`: "`Enter ` is an operand-follows echo; with
            // nothing to enter the line completes with `what?`". The original
            // does that underground too - measured 2026-09-13
            // (`qa/paired/combat-enter-refusal.tsv`, beats `enter` and
            // `enter-again`): it reads `Enter what?` where this engine
            // completed the same open echo with `Not here!`.
            // `dungeon-mode.md §10` files `E` under the letters that "print
            // \"What?\" or a stock refusal" without saying which, so the
            // shared handler - whose non-`World` arm is written for exactly
            // this case - is what answers it.
            'e' => {
                let _ = self.enter_current_location(game_dir)?;
                handled!();
            }
            'f' => {
                self.message = "What?".to_string();
                handled!();
            }
            'p' => {
                // `commands.md §8.1` row K: the dungeon echo is the
                // complete `Push` line, followed by this exact refusal. The
                // handler performs no direction poll or door cleanup.
                self.message = PUSH_NOT_HERE_REFUSAL.to_string();
                handled!(MoveOutcome::Blocked);
            }
            // `commands.md` Section 4, the `Q` row: "Save game. Routes to the
            // save-game handler, which prompts whether to save. On `N`, it
            // returns without writing. On `Y`, it writes the save files,
            // acknowledges completion, and returns to the caller. This letter
            // is not the DOS-terminate path by itself." `dungeon-mode.md`
            // Section 10 says the same of the dungeon in particular: "`Q` is
            // the ordinary save-game route; the "Exit to DOS?" prompt is a
            // Control binding in the mode-local table, not a letter."
            //
            // Section 3 files `Q` under "no action", so it costs no turn here
            // either; the `Quit:` echo of Section 5.2 comes from the shared
            // dungeon echo table.
            'q' => {
                if inline_drink.is_some() {
                    let _ = self.save_game_command(game_dir, inline_drink)?;
                } else {
                    let _ = self.start_save_game_prompt();
                }
                Ok(true)
            }
            'h' => {
                handled!(self.hole_up_command(game_dir, inline_rest)?);
            }
            ' ' => {
                self.pass_turn_with_game_dir(Some(game_dir))?;
                Ok(true)
            }
            'c' => {
                handled!(self.start_cast_spell_prompt());
            }
            'm' => {
                handled!(self.start_mix_reagents_prompt());
            }
            'n' => {
                handled!(self.start_new_order_prompt());
            }
            'r' => {
                handled!(self.start_ready_equipment());
            }
            'u' => {
                let outcome = if inline_use_request.is_some() {
                    let outcome = self.use_item_command(inline_use_request, Some(game_dir))?;
                    self.ensure_use_action_turn(turn_before);
                    outcome
                } else {
                    self.start_use_item()
                };
                handled!(outcome);
            }
            'y' => {
                handled!(self.start_yell_prompt());
            }
            'z' => {
                handled!(self.z_stats_command());
            }
            'j' => {
                handled!(self.jimmy_facing_with_game_dir(Some(game_dir))?);
            }
            _ => Ok(false),
        }
    }

    fn step_dungeon_back_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let Area::Dungeon { scene, level } = self.area else {
            return self.step_with_game_dir(direction, game_dir);
        };
        let (dx, dy) = direction.delta();
        let nx = self.player.x as isize + dx;
        let ny = self.player.y as isize + dy;
        self.step_dungeon_back(direction, nx, ny, scene, level, game_dir)
    }

    /// `vehicles.md §6` "Movement Announcements And Facing Frames": the
    /// movement-key echo carries the vehicle prefix. Returns `None` for keys
    /// that are not movement keys in [`top_down_command_echo`]'s order.
    pub fn vehicle_prefixed_movement_echo(&self, key: char) -> Option<CommandEcho> {
        if key == ' ' || key.is_ascii_uppercase() {
            return None;
        }
        let direction = Direction::from_play_key(key)?;
        let prefix = match self.player.transport {
            TransportState::Horse { .. } => MovementEchoPrefix::Ride,
            TransportState::Carpet { .. } => MovementEchoPrefix::Fly,
            TransportState::Skiff { .. } => MovementEchoPrefix::Row,
            // `vehicles.md §6` (u5-spec#297, `RETRACTIONS.md` R524/R525): on
            // the overworld a frigate key that changes the ship's *facing* -
            // not the cached sail heading - prints `Head <Dir>`, in either
            // sail state; town-family scenes never prefix a frigate. A
            // hoisted key that keeps the facing prints nothing at all
            // ([`Self::frigate_movement_key_prints_nothing`]).
            TransportState::Ship { .. }
                if direction.is_cardinal()
                    && self
                        .overworld_frigate_facing()
                        .is_some_and(|facing| facing != direction) =>
            {
                MovementEchoPrefix::Head
            }
            _ => MovementEchoPrefix::None,
        };
        vehicle_movement_echo(direction, prefix)
    }

    /// `commands.md §5`: every overworld and town-family command echoes
    /// its resident verb prefix before the handler prompts or refuses.
    /// See [`PlayState::begin_command_echo`].
    pub fn handle_top_down_key_with_inline(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_direction: Option<Direction>,
        inline_rest: impl Into<InlineRestRequest>,
        inline_yes_no: Option<bool>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        if matches!(self.area, Area::Dungeon { .. }) {
            return Ok(false);
        }
        if self.frigate_movement_key_prints_nothing(key) {
            // No echo, but the prompt pair before the key still drew its
            // marker row (`text-output.md §10.2`).
            self.open_overworld_bare_marker_row();
        } else if let Some(echo) = top_down_command_echo(key) {
            let echo = self.vehicle_prefixed_movement_echo(key).unwrap_or(echo);
            self.begin_command_echo(echo);
        }
        let handled = self.handle_top_down_key_with_inline_inner(
            key,
            game_dir,
            inline_direction,
            inline_rest,
            inline_yes_no,
            inline_use_request,
        );
        match &handled {
            Ok(true) => {
                self.commit_command_echo();
            }
            _ => self.abort_command_echo(),
        }
        handled
    }

    /// `commands.md §3`/§7: the inline-direction `G`/`J`/`O`/`S` forms report
    /// "acted" on every exit, exactly as their prompted forms do through
    /// [`Self::charge_tile_command_acted_exit`].
    ///
    /// The verb echo is committed first. The charge advances the clock, and
    /// the per-turn work behind it flushes the message slot as a line of its
    /// own; with the echo still pending, that flush printed the handler's
    /// result (`Nothing to open!`) on a row under `Open-` instead of folding
    /// it onto the echoed verb row the way [`Self::commit_command_echo`]
    /// does. Committing first keeps the result on the verb row, and the
    /// caller's own later commit then finds nothing pending.
    fn charge_inline_tile_command_acted_exit(&mut self, turn_before: u64) {
        if self.turn == turn_before {
            self.commit_command_echo();
        }
        self.charge_tile_command_acted_exit(turn_before);
    }

    fn handle_top_down_key_with_inline_inner(
        &mut self,
        key: char,
        game_dir: &Path,
        inline_direction: Option<Direction>,
        inline_rest: impl Into<InlineRestRequest>,
        inline_yes_no: Option<bool>,
        inline_use_request: Option<UseItemRequest>,
    ) -> io::Result<bool> {
        let inline_rest = inline_rest.into();
        let turn_before = self.turn;
        macro_rules! handled {
            ($outcome:expr) => {{
                let outcome = $outcome;
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
                return Ok(true);
            }};
            () => {{
                self.apply_top_down_post_turn_effects_after_turn(turn_before, game_dir)?;
                return Ok(true);
            }};
        }

        if key == ' ' {
            self.pass_turn_with_game_dir(Some(game_dir))?;
            return Ok(true);
        }

        if key.is_ascii_uppercase() {
            match key {
                'A' => {
                    if let Some(direction) = inline_direction {
                        handled!(
                            self.attack_command_with_game_dir(Some(direction), Some(game_dir))?
                        );
                    } else {
                        handled!(self.start_attack_direction_prompt());
                    }
                }
                'B' => {
                    handled!(self.board_vehicle());
                }
                'C' => {
                    handled!(self.start_cast_spell_prompt());
                }
                'D' | 'W' => {
                    self.message = unassigned_refusal_echo(key as u8).to_string();
                    handled!();
                }
                'E' => {
                    handled!(self.enter_current_location(game_dir)?);
                }
                'F' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.fire_command(Some(direction), game_dir)?);
                    } else if matches!(
                        (self.area, self.player.transport),
                        (Area::World { .. }, TransportState::Ship { .. })
                    ) {
                        handled!(self.start_fire_direction_prompt());
                    } else {
                        handled!(self.fire_command(None, game_dir)?);
                    }
                }
                'G' => {
                    if let Some(direction) = inline_direction {
                        let outcome = self.get_direction_with_game_dir(direction, game_dir)?;
                        self.charge_inline_tile_command_acted_exit(turn_before);
                        handled!(outcome);
                    } else if matches!(self.area, Area::Dungeon { .. }) {
                        handled!(self.get_facing_with_game_dir(game_dir)?);
                    } else {
                        handled!(self.start_get_direction_prompt());
                    }
                }
                'H' => {
                    handled!(self.hole_up_command(game_dir, inline_rest)?);
                }
                'I' => {
                    handled!(self.ignite_torch());
                }
                'J' => {
                    if let Some(direction) = inline_direction {
                        let outcome = self.jimmy_direction_with_game_dir_and_member(
                            direction,
                            Some(game_dir),
                            None,
                        )?;
                        self.charge_inline_tile_command_acted_exit(turn_before);
                        handled!(outcome);
                    } else {
                        handled!(self.start_jimmy_direction_prompt());
                    }
                }
                'K' => {
                    handled!(self.klimb_command(game_dir)?);
                }
                'L' => {
                    if let Some(direction) = inline_direction {
                        // The inline form skips the prompt, so the direction
                        // word has to be written onto the open `Look-` line
                        // here the way the prompt's own poll does it.
                        let _ = self.complete_open_direction_echo("Look-", direction.name());
                        handled!(self.look_direction_with_game_dir(direction, game_dir)?);
                    } else {
                        handled!(self.start_look_direction_prompt());
                    }
                }
                'M' => {
                    // Measured 2026-09-07 (`qa/paired/shrine-flow.tsv` seed):
                    // `M` on a shrine marker tile answers `Mix Reagents` and
                    // `For what spell?`, not a mantra prompt. Meditation is
                    // reached by *entering* the shrine with `E`; the shrine
                    // handler is still dispatched internally by the natural
                    // moongate of `magic.md §9.2`.
                    // `karma.md §8` (`RETRACTIONS.md` R451): "`M` remains Mix
                    // Reagents at this location. The earlier statement that
                    // the M command owns Codex entry is retracted." Codex
                    // entry is E-Enter on tile `0x11`.
                    handled!(self.start_mix_reagents_prompt());
                }
                'N' => {
                    handled!(self.start_new_order_prompt());
                }
                'O' => {
                    if let Some(direction) = inline_direction {
                        let outcome = self.open_direction_with_game_dir(direction, Some(game_dir))?;
                        self.charge_inline_tile_command_acted_exit(turn_before);
                        handled!(outcome);
                    } else {
                        handled!(self.start_open_direction_prompt());
                    }
                }
                'P' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.push_direction_with_game_dir(direction, game_dir)?);
                    } else if matches!(self.area, Area::World { .. } | Area::Town { .. }) {
                        handled!(self.start_push_direction_prompt());
                    } else {
                        handled!(self.push_facing_with_game_dir(game_dir)?);
                    }
                }
                'Q' => {
                    if inline_yes_no.is_some() {
                        handled!(self.save_game_command(game_dir, inline_yes_no)?);
                    } else {
                        handled!(self.start_save_game_prompt());
                    }
                }
                'R' => {
                    handled!(self.start_ready_equipment());
                }
                'S' => {
                    if let Some(direction) = inline_direction {
                        let outcome = self.search_direction_with_game_dir(direction, game_dir)?;
                        self.charge_inline_tile_command_acted_exit(turn_before);
                        handled!(outcome);
                    } else if matches!(self.area, Area::Dungeon { .. }) {
                        handled!(self.search_facing_with_game_dir(game_dir)?);
                    } else {
                        handled!(self.start_search_direction_prompt());
                    }
                }
                'T' => {
                    if let Some(direction) = inline_direction {
                        handled!(self.talk_direction_with_game_dir(direction, game_dir)?);
                    } else {
                        handled!(self.start_talk_direction_prompt());
                    }
                }
                'U' => {
                    let outcome = if inline_use_request.is_some() {
                        let outcome = self.use_item_command(inline_use_request, Some(game_dir))?;
                        self.ensure_use_action_turn(turn_before);
                        outcome
                    } else {
                        self.start_use_item()
                    };
                    handled!(outcome);
                }
                'V' => {
                    handled!(self.view_gem());
                }
                'X' => {
                    handled!(self.dungeon_exit_vehicle_command(game_dir)?);
                }
                'Y' => {
                    handled!(self.start_yell_prompt());
                }
                'Z' => {
                    handled!(self.z_stats_command());
                }
                _ => {}
            }
        }

        // `commands.md §5.2`, the `0` row, and `input.md §5`: a plain
        // unshifted digit is the Set Active Player command, not a
        // direction - the numpad promotion that turns `4` into west
        // needs the Shift/NumLock flag
        // ([`crate::input_typed_digit_direction_code`]), and the codes it
        // produces are the high pseudo-codes below, not ASCII digits.
        // Measured on the original: `1` answers `Avatar`, `0` answers
        // `None!` and `9` answers `Invalid!`, each on the row under the
        // `Set Active Plr:` echo.
        if key.is_ascii_digit() {
            handled!(self.set_active_player_command(key));
        }

        // `movement.md §2`: "World and town movement consume the four
        // cardinal directions only. Diagonal codes exist in the input
        // vocabulary, but the only consumer that treats one as a movement is
        // the combat targeting cursor; elsewhere they page full-screen lists
        // or are refused. No mode steps diagonally." A diagonal key therefore
        // falls through to the resident A-Z command dispatcher below.
        if let Some(direction) =
            Direction::from_play_key(key).filter(|direction| direction.is_cardinal())
        {
            let town_mode = matches!(self.area, Area::Town { .. });
            let outcome = self.step_with_game_dir(direction, Some(game_dir))?;
            // World movement already owns its landing effects inside
            // `step_world`; town movement reaches the shared town epilogue
            // here so native underfoot reactions run after the consumed step.
            if town_mode {
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
            }
            return Ok(true);
        }

        let outcome = match key.to_ascii_lowercase() {
            'e' => self.enter_current_location(game_dir)?,
            'o' => {
                if let Some(direction) = inline_direction {
                    self.open_direction_with_game_dir(direction, Some(game_dir))?
                } else if matches!(self.area, Area::Dungeon { .. }) {
                    self.open_facing_with_game_dir(Some(game_dir))?
                } else {
                    // `doors-and-z-transitions.md §4`: "The non-dungeon
                    // variant prompts for a direction and probes the tile in
                    // front" - the overworld included.
                    self.start_open_direction_prompt()
                }
            }
            'l' => {
                if let Some(direction) = inline_direction {
                    let _ = self.complete_open_direction_echo("Look-", direction.name());
                    self.look_direction_with_game_dir(direction, game_dir)?
                } else {
                    self.look_facing_with_game_dir(game_dir)?
                }
            }
            'v' => self.view_gem(),
            'i' => self.ignite_torch(),
            'h' => self.hole_up_command(game_dir, inline_rest)?,
            'f' => {
                if let Some(direction) = inline_direction {
                    self.fire_command(Some(direction), game_dir)?
                } else if matches!(
                    (self.area, self.player.transport),
                    (Area::World { .. }, TransportState::Ship { .. })
                ) {
                    self.start_fire_direction_prompt()
                } else {
                    self.fire_command(None, game_dir)?
                }
            }
            'p' => {
                if let Some(direction) = inline_direction {
                    self.push_direction_with_game_dir(direction, game_dir)?
                } else if matches!(self.area, Area::World { .. } | Area::Town { .. }) {
                    self.start_push_direction_prompt()
                } else {
                    self.push_facing_with_game_dir(game_dir)?
                }
            }
            'g' => {
                if let Some(direction) = inline_direction {
                    self.get_direction_with_game_dir(direction, game_dir)?
                } else if matches!(self.area, Area::Dungeon { .. }) {
                    self.get_facing_with_game_dir(game_dir)?
                } else {
                    self.start_get_direction_prompt()
                }
            }
            't' => {
                if matches!(self.area, Area::Town { .. }) {
                    self.start_talk_direction_prompt()
                } else {
                    self.talk_facing_with_game_dir(game_dir)?
                }
            }
            'j' => self.jimmy_facing_with_game_dir(Some(game_dir))?,
            'k' => self.klimb_command(game_dir)?,
            'x' => self.exit_vehicle_with_game_dir(Some(game_dir))?,
            // See the `M` arm above: `M` is Mix Reagents everywhere.
            'm' => self.start_mix_reagents_prompt(),
            'z' => self.z_stats_command(),
            'c' => self.start_cast_spell_prompt(),
            // `commands.md §2`: "Lowercase letters should already have been
            // folded to uppercase by the input system." `b` is no longer
            // claimed by the movement layer above — `movement.md §2`: "No
            // mode steps diagonally" — so it reaches the same `B` Board
            // handler its uppercase twin does.
            'b' => self.board_vehicle(),
            'n' => self.start_new_order_prompt(),
            'r' => self.start_ready_equipment(),
            'u' => {
                if inline_use_request.is_some() {
                    let outcome = self.use_item_command(inline_use_request, Some(game_dir))?;
                    self.ensure_use_action_turn(turn_before);
                    outcome
                } else {
                    self.start_use_item()
                }
            }
            'y' => self.start_yell_prompt(),
            '<' => self.climb(game_dir, ClimbIntent::Up)?,
            '>' => self.climb(game_dir, ClimbIntent::Down)?,
            '.' => self.idle_tick(),
            _ => return Ok(false),
        };
        self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        Ok(true)
    }

    pub fn cast_spell_from_suffix(
        &mut self,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        // A cast follow-up re-enters here with the answer appended and the
        // resources already settled by the first entry; anything else
        // starts unpaid. The flag never outlives one dispatch.
        let already_spent = std::mem::take(&mut self.cast_resources_spent);
        // `magic.md §8`: a shared variant played inside the dispatch is the
        // spell's whole-view flash ([`MessageWindowFlashHold`]).
        let was_casting = std::mem::replace(&mut self.message_window_flash_hold.casting, true);
        let outcome = self.dispatch_cast_spell_from_suffix(suffix, game_dir, already_spent);
        self.message_window_flash_hold.casting = was_casting;
        self.cast_resources_spent = false;
        outcome
    }

    fn dispatch_cast_spell_from_suffix(
        &mut self,
        suffix: &str,
        game_dir: &Path,
        already_spent: bool,
    ) -> io::Result<MoveOutcome> {
        let spell_code = inline_spell_code(suffix);
        if spell_code.is_empty() {
            self.message = cast_prompt_message();
            return Ok(MoveOutcome::Blocked);
        }
        let spell_index = spell_index_from_code(&spell_code);
        if spell_index.is_some()
            && parse_inline_party_index(suffix).is_some()
            && self.current_scene_absorbs_casts()
        {
            self.message = SCENE_ABSORBED_CAST_MESSAGE.to_string();
            // `audio.md §6.1`: an absorbing scene "prints `Absorbed!\n`
            // **and plays a manual envelope cue**". The engine printed the
            // line and played nothing.
            self.emit_sound_effect(SoundEffect::MagicAbsorbed);
            return Ok(MoveOutcome::Blocked);
        }
        if spell_index.is_some()
            && parse_inline_party_index(suffix).is_some()
            && self.combat_active
            && resolve_negate_magic_absorbs_combat_cast(
                self.active_effect_tag,
                self.active_effect_counter,
            )
        {
            // `audio.md §6.1` publishes it, which answers the note this
            // site used to carry: "Combat Negate Magic absorption prints
            // `Cast...\nAbsorbed!\n` before invoking that same manual
            // envelope: phase increment 9800, idle count 1, 28000
            // iterations, initial comparison 1000 and comparison
            // increment 2."
            //
            // The engine printed `Magic absorbed!` - the label R462
            // withdrew from the castle gate, copied here when that gate
            // still used it - and played nothing. `§6.1` also settles the
            // tail: "this path has no failure glissando or `Failed!`
            // tail. It completes the action without opening spell
            // selection or spending a charge or MP."
            self.message =
                format!("{COMBAT_CAST_COMMAND_LABEL}\n{SCENE_ABSORBED_CAST_MESSAGE}\n");
            self.emit_sound_effect(SoundEffect::MagicAbsorbed);
            return Ok(MoveOutcome::Blocked);
        }
        // `magic.md §5` step 3: the context gate runs before the handler,
        // so `Not here!` precedes any handler-specific direction/target
        // prompt and costs neither a charge nor a turn. Blink used to be
        // exempted here and re-tested inside its own handlers; that let the
        // dungeon/town "Direction?" prompt appear for a spell whose
        // published mask is `C/O` (`catalogs/spell-list.md §5`, id 17).
        if let (Some(spell_index), Some(caster_index)) =
            (spell_index, parse_inline_party_index(suffix))
            && self.party.get(caster_index).is_some()
            && !self.spell_allowed_in_current_cast_context(spell_index)
        {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        // `magic.md §5` **step 4** is the charges gate - "If the counter is
        // zero, the dispatcher prints `None mixed!` and aborts" - and **step
        // 7** is "dispatch to the effect handler". The target and direction
        // prompts belong to the handler, so the charge counter is read
        // before any of them, exactly as the scene gate above is.
        //
        // Measured 2026-09-13 (`qa/paired/magic-refusal-vocabulary.tsv`,
        // beat `uncharged`): casting Vas Mani with no charge, the original
        // answers `None mixed!` under the committed `:VAS MANI` row, and
        // this engine opened `On who: ` and asked for a target first. The
        // gate it eventually reached - `cast_spell_resource_gate`, whose own
        // ordering is right - sat behind the prompt.
        //
        // `magic.md §5` (clarified 2026-10-12, issue #277): "Steps 4 to 6
        // precede the handler, so a handler's own prompt [...] is reached
        // with the charge - and, for any spell that passed the mana gate,
        // the mana - already spent. Cancelling the prompt returns neither."
        // So the whole charge/mana/level gate runs here, once, and the
        // handler's own gate call is told it has been paid. A follow-up
        // answer re-enters with `already_spent` and is not charged again.
        if let (Some(spell_index), Some(caster_index)) =
            (spell_index, parse_inline_party_index(suffix))
            && self.party.get(caster_index).is_some()
            // Step 1, "resolve the acting member (silent abort)", precedes
            // the gates: a combat caster with no live arena actor is left
            // to the handler's own `Who casts?` refusal, spending nothing.
            && (!self.combat_active
                || self
                    .combat_actors
                    .get(caster_index)
                    .copied()
                    .is_some_and(combat_actor_is_active_not_dead))
        {
            if !already_spent {
                if self.spell_charges[spell_index] == 0 {
                    self.emit_cast_gate_refusal(crate::magic::CastGateOutcome::NoneMixed);
                    return Ok(MoveOutcome::Blocked);
                }
                if let Some(circle) = spell_circle_for(spell_index as u8)
                    && let Some(outcome) =
                        self.cast_spell_resource_gate(caster_index, spell_index, circle)
                {
                    return Ok(outcome);
                }
            }
            self.cast_resources_spent = true;
        }
        match spell_code.as_str() {
            "AG" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_dispel_field(caster_index, parse_inline_cardinal_direction(suffix)))
            }
            "AEP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_magic_lock(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                    game_dir,
                )
            }
            "AN" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::PartyTarget);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_cure(caster_index, target_index))
            }
            "AS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_open_spell(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                    game_dir,
                )
            }
            "ACX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_repel_undead(caster_index))
            }
            "AT" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_time_stop(caster_index))
            }
            "AY" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_vanish(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                ))
            }
            "AWY" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_x_ray(caster_index))
            }
            "AZ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                // Measured 2026-09-07: An Zu asks `On who: ` like its
                // sibling An Nox; it does not pick the first sleeper.
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::PartyTarget);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_awaken(caster_index, target_index))
            }
            "KX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_conjure_spell(caster_index, spell_index.unwrap()))
            }
            "AEX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Creature);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_charm_spell(caster_index, spell_index.unwrap(), target_slot))
            }
            "BRX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Creature);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_polymorph_spell(
                    caster_index,
                    spell_index.unwrap(),
                    target_slot,
                ))
            }
            "BIX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_swarm_spell(caster_index, spell_index.unwrap()))
            }
            "IQX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Creature);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_clone_spell(caster_index, spell_index.unwrap(), target_slot))
            }
            "CX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Target);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_target_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatSpellDamageKind::Kill,
                    target_slot,
                ))
            }
            "DP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_dungeon_level_spell(
                    caster_index,
                    DES_POR_SPELL_INDEX,
                    1,
                    "Down",
                    game_dir,
                )
            }
            "FV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Target);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_target_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatSpellDamageKind::Fireball,
                    target_slot,
                ))
            }
            "FGI" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        FIRE_FIELD_SPELL_INDEX,
                        FIELD_SPELL_COST,
                        CombatArenaFieldKind::Fire,
                        parse_inline_combat_spell_coordinate(suffix, "FGI"),
                    ));
                }
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    FIRE_FIELD_SPELL_INDEX,
                    FIELD_SPELL_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x82,
                    0x8a,
                    "Fire field",
                ))
            }
            "GIN" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        POISON_FIELD_SPELL_INDEX,
                        FIELD_SPELL_COST,
                        CombatArenaFieldKind::Poison,
                        parse_inline_combat_spell_coordinate(suffix, "GIN"),
                    ));
                }
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    POISON_FIELD_SPELL_INDEX,
                    FIELD_SPELL_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x81,
                    0x89,
                    "Poison field",
                ))
            }
            "GIZ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        SLEEP_FIELD_SPELL_INDEX,
                        FIELD_SPELL_COST,
                        CombatArenaFieldKind::Sleep,
                        parse_inline_combat_spell_coordinate(suffix, "GIZ"),
                    ));
                }
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    SLEEP_FIELD_SPELL_INDEX,
                    FIELD_SPELL_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x80,
                    0x88,
                    "Sleep field",
                ))
            }
            "GP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(target_slot) = parse_inline_combat_actor_slot(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Target);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_target_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatSpellDamageKind::MagicMissile,
                    target_slot,
                ))
            }
            "GIS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if self.combat_active {
                    return Ok(self.cast_combat_arena_field_spell(
                        caster_index,
                        ENERGY_FIELD_SPELL_INDEX,
                        ENERGY_FIELD_COST,
                        CombatArenaFieldKind::Energy,
                        parse_inline_combat_spell_coordinate(suffix, "GIS"),
                    ));
                }
                Ok(self.cast_dungeon_field_spell(
                    caster_index,
                    ENERGY_FIELD_SPELL_INDEX,
                    ENERGY_FIELD_COST,
                    parse_inline_cardinal_direction(suffix),
                    0x83,
                    0x8b,
                    "Energy field",
                ))
            }
            "IL" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_light_spell(
                    caster_index,
                    IN_LOR_SPELL_INDEX,
                    IN_LOR_COST,
                    IN_LOR_LIGHT_DURATION,
                ))
            }
            "IS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    PROTECTION_SPELL_INDEX,
                    PROTECTION_COST,
                    PROTECTION_ACTIVE_EFFECT_TAG,
                    PROTECTION_ACTIVE_EFFECT_DURATION,
                ))
            }
            "IMX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_create_food(caster_index))
            }
            "IP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if self.combat_active {
                    // magic.md §8 (u5-spec#298 Q3): combat Blink never
                    // prompts; it runs seven shared random arena probes.
                    return Ok(self.cast_combat_blink(caster_index));
                }
                self.cast_blink(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix)
                        && parse_inline_cardinal_direction(suffix).is_none(),
                    game_dir,
                )
            }
            "IPVY" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_tremor_combat_spell(caster_index, spell_index.unwrap()))
            }
            "IZ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Direction);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::Sleep,
                    Some(direction),
                ))
            }
            "IQW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_peer(caster_index))
            }
            "IW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_locate(caster_index))
            }
            "LS" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_invisibility(caster_index))
            }
            "HIN" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Direction);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::PoisonWind,
                    Some(direction),
                ))
            }
            "HR" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                let direction = parse_inline_cardinal_direction(suffix);
                Ok(self.cast_rel_hur(
                    caster_index,
                    direction,
                    direction.is_none() && suffix.ends_with(' '),
                ))
            }
            "LV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_light_spell(
                    caster_index,
                    VAS_LOR_SPELL_INDEX,
                    VAS_LOR_COST,
                    VAS_LOR_LIGHT_DURATION,
                ))
            }
            "M" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::PartyTarget);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_heal(caster_index, target_index))
            }
            "MV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::PartyTarget);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_great_heal(caster_index, target_index))
            }
            "EIP" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_unlock_magic(
                    caster_index,
                    parse_inline_cardinal_direction(suffix),
                    inline_explicit_pass(suffix),
                    game_dir,
                )
            }
            "CIM" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                let Some(target_index) = parse_inline_target_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::PartyTarget);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_resurrect(caster_index, target_index))
            }
            "CKX" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_combat_summon_daemon_spell(caster_index, spell_index.unwrap()))
            }
            "CGIV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Direction);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::DeathWind,
                    Some(direction),
                ))
            }
            "CIQ" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_cause_fear(caster_index))
            }
            "FHI" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(direction) = parse_inline_cardinal_direction(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Direction);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_directed_combat_spell(
                    caster_index,
                    spell_index.unwrap(),
                    CombatDirectedSpellEffect::FlameWind,
                    Some(direction),
                ))
            }
            "PU" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_dungeon_level_spell(caster_index, UUS_POR_SPELL_INDEX, -1, "Up", game_dir)
            }
            "PRV" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                // `magic.md §8` handler-family map: Gate Travel "[r]efuses
                // while the party is aboard a frigate or a skiff **before**
                // printing anything, so that transcript is a bare `Failed!`
                // with no `To phase:_` row".
                if self.party_is_shipboard_for_gate_travel() {
                    return Ok(self.fail_committed_gate_travel());
                }
                let Some(slot_index) = parse_inline_gate_phase_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::GatePhase);
                    return Ok(MoveOutcome::Blocked);
                };
                self.cast_gate_travel(caster_index, slot_index, game_dir)
            }
            "QW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_reveal(caster_index))
            }
            "RT" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    QUICKNESS_SPELL_INDEX,
                    QUICKNESS_COST,
                    QUICKNESS_ACTIVE_EFFECT_TAG,
                    QUICKNESS_ACTIVE_EFFECT_DURATION,
                ))
            }
            "AQW" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    MASS_CHARM_SPELL_INDEX,
                    MASS_CHARM_COST,
                    MASS_CHARM_ACTIVE_EFFECT_TAG,
                    MASS_CHARM_ACTIVE_EFFECT_DURATION,
                ))
            }
            "AI" => {
                let Some(caster_index) = parse_inline_party_index(suffix) else {
                    self.request_cast_argument(CastArgumentRequest::Caster);
                    return Ok(MoveOutcome::Blocked);
                };
                Ok(self.cast_active_effect_spell(
                    caster_index,
                    NEGATE_MAGIC_SPELL_INDEX,
                    NEGATE_MAGIC_COST,
                    NEGATE_MAGIC_ACTIVE_EFFECT_TAG,
                    NEGATE_MAGIC_ACTIVE_EFFECT_DURATION,
                ))
            }
            _ => {
                if let Some(spell_index) = spell_index {
                    if !self.spell_allowed_in_current_cast_context(spell_index) {
                        self.message = "Not here!".to_string();
                        return Ok(MoveOutcome::Blocked);
                    }
                }
                self.message = "No effect!".to_string();
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn current_scene_absorbs_casts(&self) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        if scene.byte == STONEGATE_SCENE_BYTE {
            return true;
        }
        scene.byte == LORD_BLACKTHORN_CASTLE_SCENE_BYTE
            && self.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] == 0
    }

    pub fn mix_reagents_from_suffix(&mut self, suffix: &str) -> MoveOutcome {
        if self.reagents.iter().all(|count| *count == 0) {
            self.message = MMIX_NO_REAGENTS_OWNED_MESSAGE.to_string();
            return MoveOutcome::Blocked;
        }
        let request = match parse_inline_mix_request(suffix) {
            Ok(Some(request)) => request,
            Ok(None) => {
                self.message = mix_prompt_message();
                return MoveOutcome::PromptDeclined;
            }
            Err(err) => {
                self.message = format!("{err}");
                return MoveOutcome::Blocked;
            }
        };
        // `magic.md §6` step 4 (published 2026-09-13 for
        // `cleak/u5-spec#272`): "A zero quantity, typed or as a bare Return,
        // prints nothing further and is tested **before** the empty-selection
        // test, so `Nothing to mix!` needs both an empty selection and a
        // positive quantity."
        //
        // The ordering here was already right; the text was not. This arm
        // printed `None!`, which the section's older "Zero cancels through
        // the cleanup path" wording was read as licensing. Nothing is
        // printed.
        if request.amount == 0 {
            self.message.clear();
            return MoveOutcome::PromptDeclined;
        }
        if request.reagent_mask == 0 {
            self.message = MMIX_EMPTY_SELECTION_MESSAGE.to_string();
            return MoveOutcome::Blocked;
        }
        for index in selected_reagent_indices(request.reagent_mask) {
            if self.reagents[index] < request.amount {
                self.message = MMIX_INSUFFICIENT_REAGENTS_MESSAGE.to_string();
                return MoveOutcome::Blocked;
            }
        }

        // `magic.md §6` Step 6: "The handler prints `Mixing...`, pauses
        // briefly, then subtracts the requested quantity from each selected
        // raw reagent counter." The line precedes the Step-7 recipe
        // comparison, so it is emitted on the success, wrong-recipe and
        // unknown-spell outcomes alike. The pause itself is presentation
        // only — "Neither branch advances the clock" — and this crate has no
        // frontend delay channel to drive, so only the printed beat is
        // modelled here; mixing still costs no game time.
        self.message = MMIX_MIXING_MESSAGE.to_string();
        for index in selected_reagent_indices(request.reagent_mask) {
            self.reagents[index] -= request.amount;
        }
        match request.spell_index {
            Some(spell_index) if request.reagent_mask == SPELL_RECIPE_MASKS[spell_index] => {
                let before = self.spell_charges[spell_index];
                self.spell_charges[spell_index] = self.spell_charges[spell_index]
                    .saturating_add(request.amount)
                    .min(99);
                let _gained = self.spell_charges[spell_index].saturating_sub(before);
                // `magic.md §6` step 7 says only that the handler "prints the
                // completion message" and does not publish the literal.
                // **Measured**, not transcribed from an asset: the stock game's
                // mixer answers `Done!` on its own row after `Mixing...`
                // (`qa/paired/magic-mix-and-cast.tsv`, and `cleak/u5-spec#221`
                // asks for it to be published). What was here before -
                // `Mixed 1 IL charge; stock is 1.` - was the engine's own
                // composition, and it reported the charge counter the original
                // keeps to itself.
                // The capture also fixes the spacing: `Mixing...` follows
                // the `How much? 1` row immediately, and `Done!` opens its
                // own block one blank row below it.
                self.message = format!("{MMIX_MIXING_MESSAGE}\n\n{MMIX_COMPLETION_MESSAGE}");
                MoveOutcome::Cast
            }
            Some(spell_index) => {
                // `magic.md §6` step 7 gives the wrong-recipe branch no line
                // of its own: "the selected reagents are already spent and no
                // spell charges are added. The wrong-mix branch then emits a
                // line break, scans the travelling party for the first member
                // whose status is Good or Poisoned, and invokes the shared
                // trap-effect resolver". A line break and the resolver's own
                // text, and nothing else.
                //
                // `Mixed wrong reagents for {code}; no spell charges added.`
                // stood here - this engine's own composition, and one that
                // told the player which recipe they had missed. It is the
                // same class of invented text `1828d278` cleared off nine
                // other paths.
                let _ = spell_index;
                self.message = self.wrong_mix_trap_message();
                MoveOutcome::Blocked
            }
            None => {
                // An unrecognised spell code reaches the same branch and, for
                // the same reason, contributes no line of its own.
                self.message = self.wrong_mix_trap_message();
                MoveOutcome::Blocked
            }
        }
    }

    /// `magic.md §6` Step 6/7: the `Mixing...` beat is printed before the
    /// recipe comparison, so it stays on the wrong-mix line too, ahead of
    /// the trap resolver's own text.
    pub fn wrong_mix_trap_message(&mut self) -> String {
        // A normally dispatched Mix always finds a Good-or-Poisoned member.
        // For a forced/corrupt state, use an out-of-range value only as the
        // resolver's deterministic selector seed. Acid and Poison already
        // bounds-check that value and therefore become safe no-ops, while the
        // party-wide Bomb and Gas families retain their published effects.
        let target_slot = self.mixer_trap_target_slot().unwrap_or(usize::MAX);
        // `Mixing...` is printed *before* the resolver runs, because §6 step
        // 6 prints it before step 7 compares the recipe and the resolver
        // prints its own word through `emit_message_line`. Returning it
        // instead would assign it to the slot after the resolver had already
        // pushed `ACID!` to the transcript, inverting the two.
        //
        // The handler already staged that same beat in the slot (step 6
        // prints it once on every outcome), and `emit_message_line` flushes
        // the slot before pushing - so the staged copy is dropped here and
        // this emission is the only `Mixing...` row (playtest G06).
        if self.message == MMIX_MIXING_MESSAGE {
            self.message.clear();
        }
        self.emit_message_line(format!("{MMIX_MIXING_MESSAGE}\n"));
        // What the resolver *returns* is an engine note naming the slot and
        // the damage; `finish_open_dungeon_chest` routes that to
        // `diagnostics` and so does this. It was being appended to the
        // mixer's player-facing line.
        let trap = self.apply_shared_trap_effect_to_slot(target_slot);
        self.push_diagnostic(trap);
        // "emits a line break, [then] ... invokes the shared trap-effect
        // resolver" - the break is the branch's whole contribution, and the
        // resolver's own line is the last thing printed.
        String::new()
    }

    /// `traps.md §4` (M-Mix): the mixer supplies its own victim slot and
    /// does **not** use the §2.1 acting-member selection. Before calling
    /// the trap-effect resolver it refreshes its target to the **first**
    /// travelling member currently marked Good or Poisoned.
    ///
    /// `cleak/u5-spec#174` closes the forced-call edge: no normally dispatched
    /// Mix can miss this scan. A forced call with no match leaves an unrelated
    /// transient word stale in the original; a safe port must not turn that
    /// into an arbitrary roster fallback. `None` preserves that distinction
    /// and lets the caller make single-target Acid/Poison safe no-ops while
    /// retaining party-wide Bomb/Gas.
    ///
    /// The two container call sites do **not** share this helper: §2.1
    /// publishes a real selection rule for them, implemented in
    /// [`Self::shared_acting_member_selection`].
    pub fn mixer_trap_target_slot(&self) -> Option<usize> {
        self.party
            .iter()
            .position(|member| matches!(member.status, b'G' | b'P'))
    }

    /// `traps.md §2.1`: the shared acting-member selection, in priority
    /// order.
    ///
    /// 1. **During a combat-class scene**, the party slot bound to the
    ///    combatant whose turn is in progress, chosen silently: no prompt,
    ///    no status test.
    /// 2. **Otherwise, when a single active character is set**, that
    ///    character, returned directly and silently, with **no status
    ///    re-check** — so a member who has become Asleep or Charmed since
    ///    the hint was set can still be the trap victim.
    /// 3. **Otherwise**, the Good-or-Poisoned scan of
    ///    [`acting_member_scan`].
    ///
    /// The scoping matters and was nearly published wrong: "a party with
    /// no able-bodied member can never spring a container trap" holds
    /// **only** outside a combat-class scene and **only** with no active
    /// character set. Both override branches skip the status test
    /// entirely, so neither is covered by that guarantee.
    ///
    /// `allow_combat_override` selects whether branch 1 is reachable. The
    /// `O` Open dispatcher routes only a narrow band of dungeon scenes to
    /// the dungeon chest handler and every other scene — combat-class
    /// scenes included — to the surface/town handler, so per §4 the combat
    /// override can fire at the surface/town container site and can
    /// **never** fire at the dungeon chest site.
    ///
    /// §2.1 records that the original range-checks neither the combatant
    /// index nor its "is a party member" flag, and that a port which does
    /// range-check is safe against every reachable case published. This
    /// range-checks.
    pub fn shared_acting_member_selection(
        &self,
        allow_combat_override: bool,
    ) -> ActingMemberSelection {
        if allow_combat_override && self.combat_active {
            if let Some(slot) = self
                .pending_combat_actor_slot
                .filter(|slot| *slot < self.party.len() && *slot < COMBAT_PARTY_ACTOR_SLOTS)
            {
                return ActingMemberSelection::Selected(slot);
            }
        }
        if let Some(slot) = self.active_player.filter(|slot| *slot < self.party.len()) {
            return ActingMemberSelection::Selected(slot);
        }
        let statuses: Vec<u8> = self
            .party
            .iter()
            .take(COMBAT_PARTY_ACTOR_SLOTS)
            .map(|member| member.status)
            .collect();
        acting_member_scan(&statuses)
    }

    /// `traps.md §2.1`/§4: the surface/town container site's acting-member
    /// selection. The Open dispatcher sends combat-class scenes here, so
    /// the combat override is reachable at this site.
    pub fn surface_container_acting_member(&self) -> ActingMemberSelection {
        self.shared_acting_member_selection(true)
    }

    /// `traps.md §2.1`/§4: the dungeon chest site's acting-member
    /// selection. Only the narrow dungeon-scene band reaches the dungeon
    /// chest handler, and combat-class scene values sit far above that
    /// band, so the combat override can never fire here.
    pub fn dungeon_container_acting_member(&self) -> ActingMemberSelection {
        self.shared_acting_member_selection(false)
    }

    /// `traps.md §2-3`: dispatch the resolved effect family. The family
    /// classification lives in [`shared_trap_effect_family_from_index`]
    /// so the resolver and the published-predicate helpers cannot drift
    /// apart; this match is exhaustive over [`TrapEffect`] rather than
    /// falling through a catch-all arm.
    pub fn apply_shared_trap_effect_to_slot(&mut self, triggering_slot: usize) -> String {
        self.apply_shared_trap_effect_to_slot_on_row(triggering_slot, false)
    }

    /// The same resolver, with its word written onto the row the caller's
    /// verb echo left open.
    ///
    /// `dungeon-mode.md §8.1`: the dungeon chest site "prints no trap
    /// notice of its own", and the `Open-` prefix leaves its row open, so
    /// the resolver's word completes it. Measured 2026-09-19
    /// (`qa/paired/dungeon-chest.tsv`, beat `opened`): the original reads
    /// `Open-BOMB!` on one row, where this engine put the word on a row of
    /// its own beneath the prefix.
    pub fn apply_shared_trap_effect_to_slot_continuing_row(
        &mut self,
        triggering_slot: usize,
    ) -> String {
        self.apply_shared_trap_effect_to_slot_on_row(triggering_slot, true)
    }

    fn apply_shared_trap_effect_to_slot_on_row(
        &mut self,
        triggering_slot: usize,
        continues_row: bool,
    ) -> String {
        self.emit_sound_effect(SoundEffect::TrapRumble);
        // `traps.md §3`: the resolver "prints exactly one of `ACID!`,
        // `POISON!`, `BOMB!` or `GAS!` **before** its effect", and
        // `dungeon-mode.md §8.1` notes the dungeon chest site adds no notice
        // of its own, so the word lands directly after that command's prefix.
        let family = self.shared_trap_effect_family();
        if continues_row {
            self.emit_message_line_continuing_row(trap_effect_message(family));
        } else {
            self.emit_message_line(trap_effect_message(family));
        }
        match family {
            TrapEffect::Acid => self.apply_acid_trap_effect(triggering_slot),
            TrapEffect::Poison => self.apply_poison_trap_effect(triggering_slot),
            TrapEffect::Bomb => self.apply_bomb_trap_effect(triggering_slot),
            TrapEffect::Gas => self.apply_gas_trap_effect(),
        }
    }

    /// `traps.md §2` step 2: draw the effect id from the shared PRNG.
    /// "Outside combat this selection rolls uniformly over an eight-entry
    /// resident lookup table. During combat-class scenes the resolver
    /// bypasses that lookup and chooses uniformly from the first two effect
    /// ids only." The table order is the engine's; §3 publishes only the
    /// per-id outcome counts.
    pub fn shared_trap_effect_id(&mut self) -> u8 {
        let index = self.shared_trap_effect_index_roll();
        shared_trap_effect_id_from_index(index, self.combat_active)
    }

    /// `traps.md §3`: the effect family selected for this trigger, using
    /// the same combat/non-combat split as [`Self::shared_trap_effect_id`].
    /// Consumes one PRNG draw.
    pub fn shared_trap_effect_family(&mut self) -> TrapEffect {
        let index = self.shared_trap_effect_index_roll();
        shared_trap_effect_family_from_index(index, self.combat_active)
    }

    fn shared_trap_effect_index_roll(&mut self) -> u8 {
        let high = if self.combat_active {
            1
        } else {
            (TRAP_NON_COMBAT_EFFECT_TABLE.len() - 1) as u8
        };
        self.random_range_u8(0, high)
    }

    pub fn apply_acid_trap_effect(&mut self, triggering_slot: usize) -> String {
        // `traps.md §3` effect id 0: "The roll is an inclusive `0..60`
        // roll halved with truncation and floored to one - the same
        // shape `systems/magic.md` publishes for Mani - so it is **not**
        // uniform over `1..30`". The raw roll is a shared-PRNG draw.
        let raw = self.random_range_u8(0, TRAP_ACID_RAW_ROLL_MAX);
        let damage = shared_trap_acid_damage_from_index(raw);
        let Some(member) = self.party.get_mut(triggering_slot) else {
            return format!(
                "Acid trap found no party member in slot {}.",
                triggering_slot.saturating_add(1)
            );
        };

        let applied = member.apply_damage(damage);
        if member.hp == 0 && self.active_player == Some(triggering_slot) {
            self.active_player = None;
        }
        format!(
            "Acid trap hit party member {} for {applied} HP.",
            triggering_slot.saturating_add(1)
        )
    }

    /// `traps.md §3` effect id 1: apply Poisoned status to the triggering
    /// slot through the shared poison helper. A member already marked
    /// Dead is skipped and left Dead.
    pub fn apply_poison_trap_effect(&mut self, triggering_slot: usize) -> String {
        if self.apply_trap_poison_status_to_slot(triggering_slot) {
            format!(
                "Poison trap poisoned party member {}.",
                triggering_slot.saturating_add(1)
            )
        } else {
            format!(
                "Poison trap had no effect on party member {}.",
                triggering_slot.saturating_add(1)
            )
        }
    }

    /// `traps.md §3` effect id 2: roll an inclusive `1..8` damage separately
    /// for each in-party member of the six-slot band "that is not marked Dead
    /// - the only status excluded is Dead". The sweep applies exactly the two
    /// gates the poison helper applies: an unsigned party-count check, then a
    /// Dead skip. Ashes (`'A'`) is a distinct value from Dead and falls
    /// through, so [`PartyMember::living`] is deliberately *not* the predicate
    /// here.
    pub fn apply_bomb_trap_effect(&mut self, _triggering_slot: usize) -> String {
        let mut affected = 0usize;
        let mut total_applied = 0u16;
        let limit = self.party.len().min(COMBAT_PARTY_ACTOR_SLOTS);
        for target_slot in 0..limit {
            if !crate::traps::trap_party_sweep_accepts_status_byte(self.party[target_slot].status) {
                continue;
            }
            // `traps.md §3` effect id 2: an independent inclusive `1..8`
            // shared-PRNG draw per accepted member, in slot order.
            let damage = self.random_range_u8(1, TRAP_BOMB_DAMAGE_MAX);
            let applied = self.party[target_slot].apply_damage(damage);
            if applied != 0 {
                affected += 1;
                total_applied = total_applied.saturating_add(applied);
            }
            if self.party[target_slot].hp == 0 && self.active_player == Some(target_slot) {
                self.active_player = None;
            }
        }
        format!("Bomb trap dealt {total_applied} HP across {affected} party member(s).")
    }

    /// `traps.md §3` effect id 3: apply Poisoned status across the
    /// six-slot party band through the same helper as effect id 1.
    /// Empty roster positions and Dead members are skipped by the helper.
    pub fn apply_gas_trap_effect(&mut self) -> String {
        let mut poisoned = 0usize;
        for slot in 0..COMBAT_PARTY_ACTOR_SLOTS {
            if self.apply_trap_poison_status_to_slot(slot) {
                poisoned += 1;
            }
        }
        format!("Gas trap poisoned {poisoned} party member(s).")
    }

    /// `traps.md §3`: the shared trap poison-status primitive used by
    /// effect ids 1 and 3. A slot outside the current party count is
    /// ignored and a member already marked Dead is skipped and left
    /// Dead; any other in-party member is rewritten to Poisoned.
    /// Nothing else changes — no hit points, maxima, or magic points,
    /// and no relation to the resurrection spell path.
    pub fn apply_trap_poison_status_to_slot(&mut self, slot: usize) -> bool {
        let Some(member) = self.party.get_mut(slot) else {
            return false;
        };
        if !trap_poison_accepts_status_byte(member.status) {
            return false;
        }
        member.status = TRAP_POISON_STATUS_BYTE;
        true
    }

    pub fn shrine_prompt_at_current_position(&self, game_dir: &Path) -> io::Result<Option<String>> {
        Ok(self
            .current_shrine_entry(game_dir)?
            .map(|entry| shrine_prompt_message(entry.virtue)))
    }

    pub fn start_shrine_prompt_at_current_position(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(entry) = self.current_shrine_entry(game_dir)? else {
            return Ok(None);
        };
        self.active_shrine = Some(ShrineSession::new(entry.virtue));
        self.message = self.render_active_shrine();
        Ok(Some(MoveOutcome::Observed))
    }

    pub fn render_active_shrine(&self) -> String {
        self.active_shrine
            .as_ref()
            .map(|session| self.render_shrine_session(session))
            .unwrap_or_else(|| "Mantra?".to_string())
    }

    fn render_shrine_session(&self, session: &ShrineSession) -> String {
        match session.phase {
            // Measured: the question and the mantra label are logged rows;
            // what the player types edits the row beneath (Virtue) or the
            // label's own row (Mantra). Both are served live by
            // [`Self::shrine_prompt_echo`], so the slot only mirrors them.
            ShrinePhase::Virtue | ShrinePhase::Mantra => self
                .shrine_prompt_echo_for(session)
                .unwrap_or_else(String::new),
            // `karma.md §12` "Completed-quest offering": the question is
            // `MISCMSG.DAT` record 34, logged when the phase opens, and what
            // the player types is a single digit echoed onto the row that
            // record's trailing space leaves open. Nothing is live-edited
            // here, so the slot mirrors nothing.
            ShrinePhase::Offering | ShrinePhase::AltarAnnouncement | ShrinePhase::AltarQuest => {
                String::new()
            }
        }
    }

    /// The shrine prompt's live row: the row the player is typing into.
    ///
    /// `text-output.md §10.6`: "the visible layout is a log whose final line
    /// is being edited". Measured 2026-09-07: the virtue answer is typed on a
    /// `:` row below the question, and the mantra is typed onto the
    /// `Mantra:` label's own row.
    pub fn shrine_prompt_echo(&self) -> Option<String> {
        // The session exists from the `E` keypress so the state is right, but
        // `karma.md §12` does not put its question on screen until the staged
        // entry reaches it - and the answer row belongs to the question.
        // Measured 2026-09-12 (`qa/paired/shrine-enter.tsv`, beats `early`
        // and `mid`): the original's row under the approach record is empty
        // where this engine had already drawn the `:`.
        if self.staged_narration_active() {
            return None;
        }
        self.active_shrine
            .as_ref()
            .and_then(|session| self.shrine_prompt_echo_for(session))
    }

    fn shrine_prompt_echo_for(&self, session: &ShrineSession) -> Option<String> {
        match session.phase {
            ShrinePhase::Virtue => Some(format!(":{}", session.virtue_buffer)),
            ShrinePhase::Mantra => Some(format!("{SHRINE_MANTRA_PROMPT}{}", session.mantra_buffer)),
            ShrinePhase::Offering | ShrinePhase::AltarAnnouncement | ShrinePhase::AltarQuest => {
                None
            }
        }
    }

    /// Mirror the live row into the compatibility slot without logging it.
    pub fn adopt_shrine_prompt_row(&mut self) {
        let row = self.render_active_shrine();
        self.adopt_flushed_message(row);
    }

    pub fn step_active_shrine(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(mut session) = self.active_shrine.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            match session.phase {
                ShrinePhase::Virtue => match ch {
                    // `karma.md §7`: "Escape erases the whole current
                    // answer and keeps the same prompt open." It never
                    // cancels the meditation.
                    '\u{1b}' => {
                        session.virtue_buffer.clear();
                    }
                    '\r' | '\n' => {
                        // `karma.md §12`: "A blank virtue answer or any blank
                        // mantra ends the interaction immediately without the
                        // unfocused-result record."
                        if session.virtue_buffer.trim().is_empty() {
                            // §7 "Exit pacing" runs after every arm.
                            self.begin_shrine_presentation_exit(0);
                            return Ok(Some(MoveOutcome::PromptDeclined));
                        }
                        // "The location has already selected the shrine's
                        // virtue; typing another virtue does not redirect the
                        // meditation." A wrong answer is not a re-prompt
                        // either - it carries through to the unfocused result
                        // after the third mantra, which is what the engine's
                        // former re-open of the question got wrong.
                        session.answers_matched &= shrine_answer_matches(
                            session.virtue_buffer.trim(),
                            session.virtue.four_letter_key(),
                        );
                        session.phase = ShrinePhase::Mantra;
                        // Measured: the answered `:` row stays on screen
                        // with a blank row under it, and the mantra label
                        // opens below as the next row to type into.
                        self.emit_message_line(format!(":{}\n\n", session.virtue_buffer));
                        // `karma.md §12`: "wait six world ticks" before the
                        // first `Mantra:` row opens. `shrine_prompt_echo`
                        // already suppresses the live Mantra row while
                        // staged narration is pending, so a pure hold here
                        // is enough - nothing new needs printing once it
                        // clears.
                        self.staged_narration.push_ticks_counting_world_steps(
                            SHRINE_MANTRA_FIRST_HOLD_BIOS_TICKS,
                            SHRINE_MANTRA_FIRST_WORLD_STEPS,
                            String::new(),
                        );
                    }
                    '\u{8}' | '\u{7f}' => {
                        session.virtue_buffer.pop();
                    }
                    ch if !ch.is_control() && session.virtue_buffer.len() < 12 => {
                        session.virtue_buffer.push(ch);
                    }
                    _ => {}
                },
                ShrinePhase::Mantra => {
                    // Measured 2026-09-07 (`qa/paired/shrine-flow.tsv`):
                    // Escape at the mantra row changes nothing at all - the
                    // window is byte-identical before and after it - so the
                    // key is simply not read here. An empty Return is what
                    // closes the prompt.
                    match ch {
                        '\r' | '\n' => {
                            return self.complete_active_shrine_mantra(session);
                        }
                        '\u{8}' | '\u{7f}' => {
                            session.mantra_buffer.pop();
                        }
                        ch if !ch.is_control()
                            && session.mantra_buffer.len() < SHRINE_MANTRA_INPUT_LIMIT =>
                        {
                            session.mantra_buffer.push(ch);
                        }
                        _ => {}
                    }
                }
                // **Measured** 2026-09-08 (`qa/paired/shrine-three-mantras.tsv`)
                // and `karma.md §12`: the altar announcement waits for "a
                // command key", then prints the quest sentence, then waits
                // again before the closing instruction. Any key advances it.
                ShrinePhase::AltarAnnouncement => {
                    // Measured 2026-09-12: one blank row between
                    // `ordained!` and the quest sentence, not two. The
                    // announcement's own trailing newline has already opened
                    // the row this record starts on, so its first newline is
                    // spent - the same split the announcement itself takes
                    // from the mantra row above it.
                    // The quest record's own trailing feed opens the row the
                    // next key is read on, so no `text-output.md §10.4` blank
                    // is spent under it. Measured 2026-09-12
                    // (`qa/paired/shrine-three-mantras.tsv`, beats `after`
                    // and `after2`).
                    self.surface_command_row_follows_history = true;
                    self.emit_shrine_misc_record_after_open_row(
                        game_dir,
                        MISCMSG_SHRINE_QUEST_SENTENCE,
                        true,
                    )?;
                    // "then the virtue's record `12` through `19` in the
                    // virtue order above, then a closing double quote and one
                    // newline".
                    self.emit_shrine_misc_record(
                        game_dir,
                        *crate::miscmsg_io::MISCMSG_VIRTUE_FAILING_RANGE.start()
                            + session.virtue.index(),
                    )?;
                    self.emit_message_line_continuing_row("\"\n");
                    session.phase = ShrinePhase::AltarQuest;
                    self.active_shrine = Some(session);
                    return Ok(None);
                }
                ShrinePhase::AltarQuest => {
                    self.emit_shrine_misc_record(game_dir, MISCMSG_SHRINE_RETURN_INSTRUCTION)?;
                    // "Finish with the shrine's sound sequence and ten world
                    // ticks." `karma.md §7.2` fills in what that sequence is
                    // on this arm: seven envelope runs from a fixed parameter
                    // list, "with no pause, no text and no input between
                    // rows", about 1.42 s audible. It "costs no timer tick
                    // and no repaint", so it sits beside the hold below
                    // rather than inside it.
                    //
                    // Not implemented from §7.2: this arm needs none of the
                    // viewport inversion, which the section gives to the
                    // offering and turn-in arms only.
                    self.emit_sound_effect(SoundEffect::ShrineOrdinationChime);
                    // The handler's ten closing world steps and the
                    // presentation's exit pacing are one real hold rather
                    // than ten instant animator steps. Measured 2026-09-12
                    // (`qa/paired/shrine-three-mantras.tsv`, beat `after2`):
                    // the original is still inside it - no command row on
                    // screen - where this engine had already returned to
                    // play. An empty beat prints nothing and only waits.
                    self.begin_shrine_presentation_exit(SHRINE_HANDLER_CLOSING_WORLD_STEPS);
                    return Ok(Some(MoveOutcome::Observed));
                }
                ShrinePhase::Offering => {
                    // `karma.md §12`: "This is a single-digit chooser, not a
                    // typed amount: accept and echo one digit `0` through
                    // `9`. Other keys, including Escape and Return, silently
                    // keep the chooser open without repeating its question."
                    let Some(digit) = ch.to_digit(10).and_then(|digit| u8::try_from(digit).ok())
                    else {
                        continue;
                    };
                    // "`0` | After echoing the digit, append ` gp\n` and end
                    // the interaction without payment or the ten-tick result
                    // pause."
                    if digit == 0 {
                        self.emit_message_line_continuing_row("0 gp\n");
                        self.begin_shrine_presentation_exit(0);
                        return Ok(Some(MoveOutcome::PromptDeclined));
                    }
                    // "`1` through `9` | After echoing the digit, append
                    // `00 gp\n\n`, so the displayed amount is in gold pieces.
                    // Check affordability after this echo."
                    self.emit_message_line_continuing_row(format!("{digit}00 gp\n\n"));
                    let cost = ShrineVirtue::shrine_offering_cost(digit).unwrap_or(0);
                    if self.gold < cost {
                        // "Insufficient gold | Print record `35` ... then
                        // repeat record `34` and the single-digit chooser. No
                        // payment or standing increase occurs."
                        self.emit_shrine_misc_record(
                            game_dir,
                            MISCMSG_SHRINE_OFFERING_INSUFFICIENT_GOLD,
                        )?;
                        self.emit_shrine_misc_record(game_dir, MISCMSG_SHRINE_OFFERING_PROMPT)?;
                        self.active_shrine = Some(session);
                        return Ok(None);
                    }
                    self.apply_shrine_offering_payment(cost, digit);
                    return Ok(Some(MoveOutcome::Observed));
                }
            }
        }
        self.active_shrine = Some(session);
        self.adopt_shrine_prompt_row();
        Ok(None)
    }

    fn complete_active_shrine_mantra(
        &mut self,
        mut session: ShrineSession,
    ) -> io::Result<Option<MoveOutcome>> {
        // Submitting retires the live row, so log what was typed before the
        // outcome runs - the same rule the cast prompt follows.
        self.emit_message_line(format!("{SHRINE_MANTRA_PROMPT}{}", session.mantra_buffer));
        if session.mantra_buffer.is_empty() {
            // Measured: Return on an empty mantra row closes the prompt and
            // hands the keyboard back to the world loop, printing nothing of
            // its own - the next command's echo follows on the next row.
            //
            // On the *next* row, not under a blank. `karma.md §12`'s ask is
            // `\nMantra:`, a leading feed, so the label's row was still open
            // when this Return arrived and `text-output.md §10.4`'s
            // next-cycle feed is what closes it. Measured 2026-09-12
            // (`qa/paired/shrine-mantra-close.tsv`, beat `closed`, reading
            // `offset+1`): the original's marker row sits directly under
            // `Mantra:` and this engine spent a blank between them.
            self.surface_command_row_follows_history = true;
            self.begin_shrine_presentation_exit(0);
            return Ok(Some(MoveOutcome::PromptDeclined));
        }
        // `karma.md §12`: "Each mantra answer uses that same matcher against
        // the shrine's expected mantra. A wrong nonblank answer does not end
        // the prompts early."
        session.answers_matched &=
            shrine_answer_matches(&session.mantra_buffer, session.virtue.mantra());
        session.mantra_asks_remaining = session.mantra_asks_remaining.saturating_sub(1);
        if session.mantra_asks_remaining > 0 {
            // One of the first two asks. §12: "The first two nonblank mantra
            // submissions likewise have no success/refusal message of their
            // own: their silence does not indicate that ordination has
            // occurred." A fresh `Mantra:` row simply opens beneath.
            session.mantra_buffer.clear();
            // §12: "waiting twelve world ticks after each nonblank answer."
            // Same pure-hold shape as the first ask's six-tick wait above.
            self.staged_narration.push_ticks_counting_world_steps(
                SHRINE_MANTRA_SUBSEQUENT_HOLD_BIOS_TICKS,
                SHRINE_MANTRA_SUBSEQUENT_WORLD_STEPS,
                String::new(),
            );
            self.active_shrine = Some(session);
            self.adopt_shrine_prompt_row();
            return Ok(None);
        }
        // `karma.md §12`'s "Unfocused result" row times this third mantra's
        // own twelve-tick wait as "the third mantra's twelve world steps",
        // reached before whichever of the three branches below prints.
        // Unlike the first two asks, nothing here is a pure hold followed by
        // a live row the existing staged-narration gate already handles for
        // free - each branch runs its own fallible MISCMSG lookup once the
        // wait ends. The session moves to `pending_shrine_third_mantra`
        // rather than staying in `active_shrine`, the same shape
        // `pending_blackthorn_audience_exit` uses for a hold whose end runs
        // a handoff instead of printing queued text; a frontend's per-frame
        // pump (or a headless shell's straight-through runner) calls
        // [`Self::step_shrine_third_mantra_resolution`] once the hold drains.
        self.staged_narration.push_ticks_counting_world_steps(
            SHRINE_MANTRA_SUBSEQUENT_HOLD_BIOS_TICKS,
            SHRINE_MANTRA_SUBSEQUENT_WORLD_STEPS,
            String::new(),
        );
        self.pending_shrine_third_mantra = Some(session);
        Ok(Some(MoveOutcome::Observed))
    }

    /// Run `karma.md §12`'s third-mantra closing branch once its
    /// twelve-tick hold has drained. A no-op while the hold is still
    /// running, or when nothing is pending.
    pub fn step_shrine_third_mantra_resolution(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.pending_shrine_third_mantra.is_none() || self.staged_narration_active() {
            return Ok(None);
        }
        let mut session = self.pending_shrine_third_mantra.take().expect("checked above");
        if !session.answers_matched {
            // "Unfocused result | If the virtue answer or any of the three
            // mantra answers was wrong, render record `30` after the third
            // nonblank mantra, then return without quest progress."
            self.emit_shrine_misc_record_after_open_row(
                game_dir,
                MISCMSG_SHRINE_UNFOCUSED_RESULT,
                true,
            )?;
            // §7: the wrong-answer arm takes its early exit "ahead of both
            // the stamp and that ten-tick hold", so only the exit pacing runs.
            self.begin_shrine_presentation_exit(0);
            return Ok(Some(MoveOutcome::PromptDeclined));
        }
        let bit = session.virtue.bit();
        let ordained = self.shrine_ordained_mask & bit != 0;
        let codex = self.shrine_codex_mask & bit != 0;
        if !ordained && codex {
            // `karma.md §12`: the completed-quest arm "Runs the ordinary
            // offering path", whose question is record 34.
            session.phase = ShrinePhase::Offering;
            self.emit_shrine_misc_record(game_dir, MISCMSG_SHRINE_OFFERING_PROMPT)?;
            self.active_shrine = Some(session);
            return Ok(None);
        }
        if !codex {
            // `karma.md §12`, the first two quest-state rows: with the
            // Codex unread the meditation "Sets the ordained bit. No gold
            // prompt, stat increase, or standing increase is applied."
            //
            // Measured 2026-09-07 (`qa/paired/shrine-flow.tsv`): nothing
            // prints - the engine used to invent a `Meditated at the
            // Shrine of ...` line here. What the capture showed after the
            // accepted mantra was a *fresh* `Mantra:` row, which is §12's
            // second ask of three, not a fourth.
            //
            // §12: "Set/retain ordination before record `31`, the
            // altar's quest announcement. Restore the standing Avatar
            // pose and wait for a command key."
            self.shrine_ordained_mask |= bit;
            // §7 "What 'restore the standing Avatar pose' means": the walking
            // pose is re-stamped at `(5, 6)` "between the quest-bit write and
            // record `31`", so the avatar stands through both key waits and
            // the closing ten steps.
            if let Some(walk) = self.shrine_approach_walk.as_mut() {
                walk.stand();
            }
            // Measured 2026-09-12 (`qa/paired/shrine-three-mantras.tsv`,
            // beat `mantra3`): the announcement prints with the cursor
            // still at the end of the accepted `Mantra:AHM` row, so the
            // record's first newline closes that row and only its second
            // leaves a blank - the same split record `30` takes below. The
            // engine closed the row itself and then spent both, showing
            // two blanks where the original shows one.
            self.emit_shrine_misc_record_after_open_row(
                game_dir,
                MISCMSG_SHRINE_ALTAR_ANNOUNCEMENT,
                true,
            )?;
            session.phase = ShrinePhase::AltarAnnouncement;
            self.active_shrine = Some(session);
            return Ok(None);
        }
        // Ordained and Codex read: the turn-in arm. The mantra has already
        // been accepted by the substring matcher above, so nothing here
        // re-reads the typed text.
        self.complete_shrine_codex_turn_in(session.virtue, game_dir)?;
        Ok(Some(MoveOutcome::Observed))
    }

    /// `karma.md §7`, quest-state row "Ordained and Codex read", and §7.2's
    /// turn-in beat: "Clear ordination before record `36`, the congratulatory
    /// response. Play the turn-in beat of Section 7.2 - one unrestored
    /// viewport inversion, then the 920-step envelope swell, then the shared
    /// flash/rumble described below - and then award standing and the
    /// applicable Avatar attributes. Each applicable attribute prints
    /// `Strength +1\n`, `Dexterity +1\n`, or `Intelligence +1\n`, in that
    /// order ... Finish with ten world ticks."
    fn complete_shrine_codex_turn_in(
        &mut self,
        virtue: ShrineVirtue,
        game_dir: &Path,
    ) -> io::Result<()> {
        let bit = virtue.bit();
        self.shrine_ordained_mask &= !bit;
        // Printed while the accepted `Mantra:` row is still open, like
        // records `30` and `31`.
        self.emit_shrine_misc_record_after_open_row(
            game_dir,
            MISCMSG_SHRINE_TURN_IN_CONGRATULATION,
            true,
        )?;
        // §7.2: "sets the drawing colour and then issues a single rectangle
        // fill over the gameplay viewport with the driver's exclusive-or mode
        // selected", once and never undone.
        self.presentation_viewport_inverted = true;
        // The swell: 920 envelope runs at phase increment 3100, about 5.93 s.
        self.emit_sound_effect(SoundEffect::ShrineTurnInSwell);
        // "Only the turn-in arm then runs the shared flash-and-rumble helper
        // ... **after** the swell rather than instead of it."
        self.emit_major_flash();
        // karma.md §3-4: shared moral-standing selector +3 on Codex turn-in,
        // with Humility receiving another +3 after the stat step.
        let mut moral_gained =
            self.add_moral_standing(ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE);
        let stat_notes = self.apply_shrine_stat_reward(virtue);
        if virtue == ShrineVirtue::Humility {
            let humility_moral =
                self.add_moral_standing(ShrineVirtue::SHRINE_CODEX_TURN_IN_MORAL_INCREASE);
            moral_gained = moral_gained.saturating_add(humility_moral);
        }
        self.diagnostics.push(format!(
            "shrine of {} completed; moral +{} to {}",
            virtue.name(),
            moral_gained,
            self.moral_standing
        ));
        // §7.2: "On that arm the attribute lines print after the whole sound,
        // not during it." The swell costs no world step, so its hold carries
        // none; the inversion stands until it ends.
        let lines: String = stat_notes.iter().map(|line| format!("{line}\n")).collect();
        self.staged_narration
            .push_ticks(SHRINE_TURN_IN_SWELL_HOLD_BIOS_TICKS, lines);
        self.begin_shrine_presentation_exit(SHRINE_HANDLER_CLOSING_WORLD_STEPS);
        Ok(())
    }

    /// `karma.md §7` "Exit pacing": the presentation's exit after the
    /// handler returns, on every arm. `handler_steps` is the handler's own
    /// closing wait that precedes it - ten world steps on the completion
    /// arms, none on the blank, wrong-answer and zero-offering arms, which
    /// "take their early exit ahead of both the stamp and that ten-tick
    /// hold".
    pub(crate) fn begin_shrine_presentation_exit(&mut self, handler_steps: u16) {
        let arm = self
            .shrine_approach_walk
            .as_ref()
            .map(|walk| walk.arm())
            .unwrap_or_default();
        if let Some(walk) = self.shrine_approach_walk.as_mut() {
            walk.begin_exit(handler_steps);
        }
        let steps = handler_steps + arm.exit_world_steps();
        self.staged_narration.push_ticks_counting_world_steps(
            shrine_presentation_hold_bios_ticks(steps),
            steps,
            String::new(),
        );
    }

    /// Run the third-mantra resolution straight through, for shells with no
    /// pump. Mirrors [`Self::run_blackthorn_audience_exit_to_handoff`].
    pub fn run_shrine_third_mantra_resolution_to_completion(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        if self.pending_shrine_third_mantra.is_none() {
            return Ok(None);
        }
        self.flush_staged_narration();
        self.step_shrine_third_mantra_resolution(game_dir)
    }

    /// `karma.md §8`: run the Codex presentation after E-Enter has echoed
    /// `Enter the Shrine of the Codex!`.
    ///
    /// The presentation "suspends the ordinary active objects and shows the
    /// eleven-by-eleven Codex map from `MISCMAPS.DAT`, record 2", runs its
    /// opening world step, prints its own approach record `46` (§8.1: "Every
    /// Codex presentation begins with `MISCMSG.DAT` record `46`"), and walks
    /// the avatar up seven rows: "**twelve** animation frames, and **sixty**
    /// world steps ... between the Codex's own approach record and the
    /// reader". The reader then waits on a key before each later record
    /// (§8.2), which [`Self::step_codex_presentation`] serves one key at a
    /// time; nothing is printed ahead.
    ///
    /// §8 (u5-spec#298 Q5, `RETRACTIONS.md` R545): when any roster record in
    /// slots 1 through 15 holds whereabouts `0x7F`, the interred-party page
    /// "is emitted **inside** the walk, between the frame that first draws
    /// the avatar and the stepping frames" - after frame 5, the first walking
    /// pose (`shrine_presentation.rs`), and before frame 6. Not implemented:
    /// the "one temporary active object per matching record ... with actor
    /// tile `0x03`", whose "per-count positions are not published".
    ///
    /// Measured 2026-09-09 (`qa/paired/codex-enter.tsv`) against the original,
    /// seeded straight onto Britannia `(233, 233)`: record `46`, then `37`,
    /// `38` and the page, each behind its own key.
    pub fn read_codex_urn_after_entry(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.shrine_presentation_map =
            load_miscmaps_cutscene_map(game_dir, CODEX_PRESENTATION_CUTSCENE_MAP_RECORD)?;
        let mut walk = crate::shrine_presentation::ShrineApproachWalk::codex();
        walk.seed_animator(self.prng_state);
        self.shrine_approach_walk = Some(walk);
        // §7's frame shape, shared by §8: build the display, run one world
        // step, print the approach record, then walk.
        self.advance_presentation_world_step();
        self.emit_shrine_misc_record(game_dir, MISCMSG_CODEX_ENTRY_NARRATION)?;
        // "There is no forty-sixth repaint step ... so the Codex interval is
        // a flat sixty."
        let steps = crate::shrine_presentation::PresentationArm::Codex.walk_world_steps();
        match codex_interred_party_page(&self.codex_interred_names()) {
            Some(page) => {
                // The page has "no key wait and no pause of its own": it
                // rides on the end of the frames up to the first avatar
                // frame, and the stepping frames follow.
                let first_pose_steps = u16::from(
                    crate::shrine_presentation::SHRINE_APPROACH_WALK_FIRST_POSE_FRAME,
                ) * u16::from(crate::shrine_presentation::SHRINE_APPROACH_WALK_STEPS_PER_FRAME);
                self.staged_narration.push_ticks_counting_world_steps(
                    shrine_presentation_hold_bios_ticks(first_pose_steps),
                    first_pose_steps,
                    page,
                );
                let rest = steps - first_pose_steps;
                self.staged_narration.push_ticks_counting_world_steps(
                    shrine_presentation_hold_bios_ticks(rest),
                    rest,
                    String::new(),
                );
            }
            None => self.staged_narration.push_ticks_counting_world_steps(
                shrine_presentation_hold_bios_ticks(steps),
                steps,
                String::new(),
            ),
        }
        // §8.2: "The counts begin after the initial approach record `46`."
        self.codex_presentation_steps = 1;
        Ok(MoveOutcome::Observed)
    }

    /// `karma.md §8` (u5-spec#298 Q5): the names on the interred-party page.
    /// "The scan covers roster slots 1..15 (every record but the Avatar's)
    /// whether travelling or not, testing for whereabouts value `0x7F`".
    /// A record's whereabouts byte is its byte `0x1F`, which this engine
    /// holds as the marker of inn-registry slot `k` (`formats/saved-gam.md
    /// §3.1`); the name is roster record `k`'s own.
    /// The matching records' bytes from the name field onward, for
    /// [`codex_interred_party_page`]'s run-on copy (`karma.md §8`). A slot
    /// with no roster record contributes its guest name field alone.
    pub fn codex_interred_names(&self) -> Vec<Vec<u8>> {
        INTERRED_SCAN_SLOTS
            .filter_map(|slot| {
                let guest = self.inn_registry.iter().find(|guest| {
                    usize::from(guest.registry_slot) == slot
                        && guest.scene_marker == INTERRED_WHEREABOUTS
                })?;
                Some(
                    self.party_roster
                        .get(slot)
                        .map_or(guest.name.to_vec(), |record| {
                            record.leading_record_bytes().to_vec()
                        }),
                )
            })
            .collect()
    }

    /// Serve one advance key owed to an open Codex reader, `karma.md §8.2`:
    /// "Each accepted key advances the presentation; its command value is
    /// discarded and produces no ordinary command echo or world command."
    ///
    /// Returns `true` when the key belonged to the presentation and must not
    /// reach the command parser.
    pub fn step_codex_presentation(&mut self, game_dir: &Path) -> io::Result<bool> {
        let key = self.codex_presentation_steps;
        if key == 0 {
            return Ok(false);
        }
        let mut finished = false;
        match key {
            // "1 | Print shared record `37`." / "2 | Print shared record `38`."
            1 => self.emit_shrine_misc_record(game_dir, MISCMSG_CODEX_PAGE_OPENED)?,
            2 => self.emit_shrine_misc_record(game_dir, MISCMSG_CODEX_READS_PREFIX)?,
            // "3 | With no ordained virtue, print `39` and finish. Otherwise
            // stamp the selected virtue and print its quoted aphorism
            // `20`-`27`."
            3 => match read_codex_urn(self.shrine_ordained_mask, &mut self.shrine_codex_mask) {
                CodexUrnReadOutcome::NoOrdained => {
                    self.emit_shrine_misc_record(game_dir, MISCMSG_CODEX_NO_QUEST_PAGE)?;
                    finished = true;
                }
                CodexUrnReadOutcome::Stamped { virtue, .. } => {
                    // §8.1: "The aphorism is enclosed by an opening double
                    // quote and a closing double quote followed by two
                    // newlines, in addition to the record's own authored
                    // content."
                    let index = *MISCMSG_VIRTUE_APHORISM_RANGE.start() + virtue.index();
                    if let Some(record) = load_misc_messages(game_dir)?
                        .and_then(|messages| messages.record(index).map(str::to_owned))
                    {
                        self.emit_message_line(format!(
                            "\"{}\"\n\n",
                            record.replace('\r', "\n")
                        ));
                    }
                }
            },
            // "4, ordained path only | If the updated read mask is
            // incomplete, finish. Otherwise perform the three flash/rumble
            // effects and print page-turn record `40`."
            4 => {
                if self.shrine_codex_mask != 0xFF {
                    finished = true;
                } else {
                    // §8.1: "Each of those three cycles sets a drawing colour
                    // and inverts the gameplay viewport before running the
                    // shared effect, and no restoring inversion follows them."
                    // `audio.md §8.4`: each draws its gameplay-PRNG bands
                    // whether or not sound is audible (audit A029).
                    for _ in 0..CODEX_COMPLETION_MAJOR_FLASHES {
                        self.presentation_viewport_inverted = !self.presentation_viewport_inverted;
                        self.emit_major_flash();
                    }
                    self.emit_shrine_misc_record(game_dir, crate::miscmsg_io::MISCMSG_CODEX_PAGE_TURN)?;
                }
            }
            // "5 | Print the ordinary-font read lead-in, then runic record
            // `41`." / 6-8: runic `42`-`44`.
            5..=8 => {
                if key == 5 {
                    self.emit_message_line(crate::miscmsg_io::CODEX_COMPLETION_READ_LEAD_IN);
                }
                let index = *crate::miscmsg_io::MISCMSG_CODEX_COMPLETION_PAGES.start()
                    + usize::from(key - 5);
                self.emit_codex_runic_page(game_dir, index)?;
            }
            // "9 | Finish and allow the enclosing presentation to restore the
            // prior scene."
            _ => finished = true,
        }
        if finished {
            self.codex_presentation_steps = 0;
            // §8: the Codex exit "is the shrine's - the same block, reached
            // unconditionally after either handler returns".
            self.begin_shrine_presentation_exit(0);
        } else {
            self.codex_presentation_steps = key + 1;
        }
        Ok(true)
    }

    /// One of the Codex's runic pages, `karma.md §8.2`: "Records `41`-`44`
    /// use the ordinary message window with the runic font selected for each
    /// record; the normal font is restored before each following key wait.
    /// Preserve the stored glyphs and separators". The glyph stream goes to
    /// the runic font as stored - `@`, `[`, `]` and `_` are runic cells
    /// (`formats/miscmsg-dat.md §4`), not digraphs to expand.
    fn emit_codex_runic_page(&mut self, game_dir: &Path, index: usize) -> io::Result<()> {
        let Some(record) = load_misc_messages(game_dir)?
            .and_then(|messages| messages.record(index).map(|record| record.replace('\r', "\n")))
        else {
            return Ok(());
        };
        // Same row rules as `emit_message_line` (a leading feed closes an
        // open row; an embedded feed is a producer blank, not the derived
        // turn blank), but each cell keeps the runic font (audit A010
        // review).
        self.flush_message_slot();
        let row_open = self.message_row_open_mid_line;
        let mut segments = record.split('\n').peekable();
        let mut first = true;
        while let Some(row) = segments.next() {
            let is_last = segments.peek().is_none();
            let closes_open_row = first && row_open;
            first = false;
            if row.is_empty() {
                if !is_last && !closes_open_row {
                    self.push_producer_blank_message_entry();
                }
                continue;
            }
            self.push_runic_message_entry(row);
        }
        // Set explicitly for this caller: the page decides whether the
        // printer left its row open.
        self.message_row_open_mid_line = !record.ends_with('\n');
        self.adopt_flushed_message(record.clone());
        Ok(())
    }

    pub fn codex_urn_text_for_virtue(
        &self,
        game_dir: &Path,
        virtue: ShrineVirtue,
    ) -> io::Result<Option<String>> {
        let Some(messages) = load_misc_messages(game_dir)? else {
            return Ok(None);
        };
        Ok(messages
            .urn_codex_for_virtue_index(virtue.index())
            .map(render_miscmsg_tile_glyph_text))
    }

    /// Log one `MISCMSG.DAT` record, keeping "their spaces and line breaks"
    /// exactly as `karma.md §12` requires. A profile without the file logs
    /// nothing rather than substituting engine prose.
    fn emit_shrine_misc_record(&mut self, game_dir: &Path, index: usize) -> io::Result<()> {
        self.emit_shrine_misc_record_after_open_row(game_dir, index, false)
    }

    /// As [`Self::emit_shrine_misc_record`], but for a record printed while a
    /// typed prompt row is still open.
    ///
    /// `karma.md §12` renders record `30` "after the third nonblank mantra",
    /// and that record is `\n\nThine thoughts are unfocused.\n`. The cursor is
    /// sitting at the end of the `Mantra:AHM` row when it prints, so the
    /// record's *first* newline is what closes that row and only the second
    /// leaves a blank - the same split `combat.md §8.1` makes between the
    /// banner's newline and the turn loop's.
    ///
    /// Measured (`qa/paired/shrine-three-mantras.tsv`, beat `mantra3`): the
    /// original shows one blank row between the last mantra and the result;
    /// this engine closed the row itself and then spent both newlines, so it
    /// showed two.
    ///
    /// The record is passed through whole. Stripping its first newline was
    /// this function's original job, back when a leading feed always blanked
    /// a row: with the row open the record's two feeds would have produced
    /// two blanks, so one was removed. `425f69bf` moved that rule into
    /// `push_message_transcript_lines` itself - "a leading feed closes an
    /// open row rather than blanking one" - and the strip became a second
    /// deduction of the same feed, leaving no blank at all.
    ///
    /// Measured again 2026-09-18 (`shrine-unfocused/mantra3`, reading
    /// `scroll`): the original keeps its blank row between the last
    /// `Mantra:AHM` and `Thine thoughts are unfocused.` and this engine ran
    /// them together. `row_open` is now only a statement of the row state,
    /// which is what the transcript layer needs to apply the rule.
    fn emit_shrine_misc_record_after_open_row(
        &mut self,
        game_dir: &Path,
        index: usize,
        row_open: bool,
    ) -> io::Result<()> {
        if let Some(text) = load_misc_messages(game_dir)?.and_then(|messages| {
            messages
                .record(index)
                .map(|record| record.replace('\r', "\n"))
        }) {
            if row_open {
                self.open_message_row();
            }
            self.emit_message_line(text);
        }
        Ok(())
    }

    /// `karma.md §12`: "Affordable nonzero offering | Deduct the displayed
    /// gold amount, refresh the stats display and award the digit's standing
    /// increase. Print `ALAKAZAM` using the runic font, restore the normal
    /// font and append `!\n`. Play the local viewport/sound effect and finish
    /// with ten world ticks."
    fn apply_shrine_offering_payment(&mut self, cost: u16, digit: u8) {
        self.gold -= cost;
        // `karma.md §3-4`: the offered digit is added to the shared
        // moral-standing selector, clamped at ninety-nine.
        let _ = self.add_moral_standing(digit);
        self.push_runic_message_entry("ALAKAZAM");
        // The runic word leaves its row open for the normal-font `!`; the
        // row state is stated here because the push above does not set it.
        self.open_message_row();
        self.emit_message_line_continuing_row("!\n");
        self.adopt_flushed_message("ALAKAZAM!");
        // `karma.md §7.2` names §12's "local viewport/sound effect" on this
        // arm: the envelope generator run 920 times in one sweep at a fixed
        // pitch, "one slow swell and decay", about 7.91 s.
        //
        // The swell is preceded by "a single rectangle fill over the
        // gameplay viewport with the driver's exclusive-or mode selected",
        // once and never undone: "the world steps that follow simply repaint
        // over it".
        self.presentation_viewport_inverted = true;
        self.emit_sound_effect(SoundEffect::ShrineOfferingSwell);
        // The swell blocks for its own length and costs no world step; then
        // the ten-tick result pause and the exit pacing, as counted steps.
        self.staged_narration
            .push_ticks(SHRINE_OFFERING_SWELL_HOLD_BIOS_TICKS, String::new());
        self.begin_shrine_presentation_exit(SHRINE_HANDLER_CLOSING_WORLD_STEPS);
    }

    /// `karma.md §7`: meditation matches the party position against the
    /// single eight-row shrine coordinate table. "The match has one
    /// deliberate fall-through that an implementation must reproduce.
    /// Spirituality's row is a `(0, 0)` sentinel rather than a real position
    /// … so no party can ever match it by position. When the scan of all
    /// eight rows finds no match, the handler resolves the meditation to
    /// **Spirituality**. Meditating at a shrine that is not one of the seven
    /// mapped ones *is* the test for Spirituality, and \"no row matched\" must
    /// not be treated as an error."
    ///
    /// `Ok(None)` is reserved for "this `M` press is not a meditation at
    /// all" — wrong plane, or not standing on a shrine altar tile — so `M`
    /// still routes to Mix Reagents on ordinary terrain.
    /// `karma.md §7`'s coordinate scan alone, for a route into the kneel
    /// overlay that has no shrine tile under the party (the
    /// natural-moongate midnight hook of `overworld.md §9.2`): the sidecar
    /// table when one is present, else the published rows, and
    /// Spirituality when no row matches.
    pub fn shrine_virtue_by_coordinates(&self, game_dir: &Path) -> io::Result<ShrineVirtue> {
        let (x, y) = (self.player.x, self.player.y);
        if let Area::World { plane } = self.area
            && let Some(entries) = load_shrine_entries(game_dir)?
        {
            return Ok(entries
                .into_iter()
                .find(|entry| entry.plane == plane && entry.x == x && entry.y == y)
                .map_or(ShrineVirtue::Spirituality, |entry| entry.virtue));
        }
        Ok(published_shrine_entries()
            .into_iter()
            .find(|entry| entry.x == x && entry.y == y)
            .map_or(ShrineVirtue::Spirituality, |entry| entry.virtue))
    }

    pub fn current_shrine_entry(&self, game_dir: &Path) -> io::Result<Option<ShrineEntry>> {
        let Area::World { plane } = self.area else {
            return Ok(None);
        };
        if plane != WorldPlane::Britannia {
            return Ok(None);
        }
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        if let Some(entries) = load_shrine_entries(game_dir)? {
            if let Some(entry) = entries.into_iter().find(|entry| {
                entry.plane == plane
                    && entry.x == self.player.x
                    && entry.y == self.player.y
                    && entry
                        .expected_tile
                        .map_or(true, |expected| expected == tile)
            }) {
                return Ok(Some(entry));
            }
            // §7 fall-through: the scan of the coordinate table found no row,
            // which resolves the meditation to Spirituality rather than
            // failing. Gated on the shrine tiles so an ordinary surface tile
            // keeps routing `M` to Mix Reagents.
            if tile == SHRINE_MARKER_TILE || shrine_virtue_for_altar_tile(tile).is_some() {
                return Ok(Some(ShrineEntry {
                    plane,
                    x: self.player.x,
                    y: self.player.y,
                    virtue: ShrineVirtue::Spirituality,
                    expected_tile: Some(tile),
                }));
            }
            return Ok(None);
        }
        // `catalogs/gazetteer.md` §7: "The ordinary entry gate is the live map
        // tile `0x19`, not the coordinate list alone" - and that one table
        // "serves both virtue-identification roles". So the tile decides
        // *whether* this is a shrine and the coordinate row decides *which*
        // virtue, with §7's sentinel row as the fall-through: "a meditation
        // attempt whose position matches none of the seven mapped shrines
        // resolves to **Spirituality**".
        //
        // My 2026-09-07 reading - that Honesty's (233, 66) and Humility's
        // (231, 216) are not shrines in the shipped map - was wrong, and
        // `cleak/u5-spec#239` is closed with a fresh decode confirming all
        // seven cells plus "successful E-Enter at Honesty". Measured again
        // 2026-09-08 (`qa/paired/shrine-three-mantras.tsv`): the stock game
        // enters the *Honesty* shrine at (233, 66) and ordains its quest.
        // Gating the lookup on the interior altar-tile family, as this engine
        // did, left every surface shrine resolving to Spirituality.
        if tile != SHRINE_MARKER_TILE && shrine_virtue_for_altar_tile(tile).is_none() {
            return Ok(None);
        }
        let virtue = published_shrine_entries()
            .into_iter()
            .find(|entry| entry.x == self.player.x && entry.y == self.player.y)
            .map(|entry| entry.virtue)
            .or_else(|| shrine_virtue_for_altar_tile(tile))
            .unwrap_or(ShrineVirtue::Spirituality);
        Ok(Some(ShrineEntry {
            plane,
            x: self.player.x,
            y: self.player.y,
            virtue,
            expected_tile: Some(tile),
        }))
    }

    pub fn apply_shrine_stat_reward(&mut self, virtue: ShrineVirtue) -> Vec<String> {
        let mut notes = Vec::new();
        match virtue {
            ShrineVirtue::Honesty => self.add_avatar_intelligence_reward(&mut notes),
            ShrineVirtue::Compassion => self.add_avatar_dexterity_reward(&mut notes),
            ShrineVirtue::Valor => self.add_avatar_strength_reward(&mut notes),
            ShrineVirtue::Justice => {
                self.add_avatar_dexterity_reward(&mut notes);
                self.add_avatar_intelligence_reward(&mut notes);
            }
            ShrineVirtue::Sacrifice => {
                self.add_avatar_strength_reward(&mut notes);
                self.add_avatar_dexterity_reward(&mut notes);
            }
            ShrineVirtue::Honor => {
                self.add_avatar_strength_reward(&mut notes);
                self.add_avatar_intelligence_reward(&mut notes);
            }
            ShrineVirtue::Spirituality => {
                self.add_avatar_strength_reward(&mut notes);
                self.add_avatar_dexterity_reward(&mut notes);
                self.add_avatar_intelligence_reward(&mut notes);
            }
            ShrineVirtue::Humility => {}
        }
        notes
    }

    // `karma.md §12`: the awarded attribute's line "still prints when the
    // attribute is already at its cap", so the push is unconditional and only
    // the stat write is capped. The engine used to gate the line on the
    // increase succeeding, and to abbreviate it as `STR +1`.
    pub fn add_avatar_strength_reward(&mut self, notes: &mut Vec<String>) {
        let _ = self.avatar_stats.increase_strength();
        notes.push(crate::commands::SHRINE_REWARD_STRENGTH_LINE.to_string());
    }

    pub fn add_avatar_dexterity_reward(&mut self, notes: &mut Vec<String>) {
        if self.avatar_stats.increase_dexterity() {
            self.sync_avatar_dexterity_to_party();
        }
        notes.push(crate::commands::SHRINE_REWARD_DEXTERITY_LINE.to_string());
    }

    pub fn add_avatar_intelligence_reward(&mut self, notes: &mut Vec<String>) {
        let _ = self.avatar_stats.increase_intelligence();
        notes.push(crate::commands::SHRINE_REWARD_INTELLIGENCE_LINE.to_string());
    }

    pub fn sync_avatar_dexterity_to_party(&mut self) {
        if let Some(member) = self.party.iter_mut().find(|member| member.slot == 0) {
            member.climb_stat = self.avatar_stats.dexterity;
        }
    }
}

pub(crate) fn high_byte_direction_from_key(key: char) -> Option<Direction> {
    let scalar = key as u32;
    if scalar > u8::MAX as u32 {
        return None;
    }
    match input_code_direction(scalar as u8)? {
        InputDirection::North => Some(Direction::North),
        InputDirection::South => Some(Direction::South),
        InputDirection::East => Some(Direction::East),
        InputDirection::West => Some(Direction::West),
        InputDirection::Northwest => Some(Direction::NorthWest),
        InputDirection::Northeast => Some(Direction::NorthEast),
        InputDirection::Southwest => Some(Direction::SouthWest),
        InputDirection::Southeast => Some(Direction::SouthEast),
    }
}

#[cfg(test)]
mod movement_magic_karma_traps_spec_tests {
    use crate::test_fixtures::{
        debug_game_dir, dungeon_state, open_dungeon_record, open_grid, open_world_grid, test_state,
        world_state,
    };
    use crate::*;

    /// `movement.md §2`: "World and town movement consume the four cardinal
    /// directions only. Diagonal codes exist in the input vocabulary, but the
    /// only consumer that treats one as a movement is the combat targeting
    /// cursor … No mode steps diagonally."
    #[test]
    fn town_movement_refuses_every_diagonal_direction() {
        for direction in [
            Direction::NorthWest,
            Direction::NorthEast,
            Direction::SouthWest,
            Direction::SouthEast,
        ] {
            let mut state = test_state(open_grid(), 10, 10);
            let facing_before = state.player.facing;
            let transport_before = state.player.transport;

            assert_eq!(state.step(direction), MoveOutcome::Blocked);

            assert_eq!((state.player.x, state.player.y), (10, 10));
            assert_eq!(state.player.facing, facing_before);
            assert_eq!(state.player.transport, transport_before);
            assert_eq!(state.turn, 0);
        }
    }

    /// `movement.md §2`: the same rule on the overworld plane.
    #[test]
    fn world_movement_refuses_every_diagonal_direction() {
        for direction in [
            Direction::NorthWest,
            Direction::NorthEast,
            Direction::SouthWest,
            Direction::SouthEast,
        ] {
            let mut state = world_state(open_world_grid(), 40, 40);
            let facing_before = state.player.facing;

            assert_eq!(state.step(direction), MoveOutcome::Blocked);

            assert_eq!((state.player.x, state.player.y), (40, 40));
            assert_eq!(state.player.facing, facing_before);
            assert_eq!(state.turn, 0);
        }
    }

    /// `movement.md §2`: a cardinal step is untouched by the diagonal gate.
    #[test]
    fn town_movement_still_accepts_a_cardinal_step() {
        let mut state = test_state(open_grid(), 10, 10);

        assert_eq!(state.step(Direction::East), MoveOutcome::Moved);

        assert_eq!((state.player.x, state.player.y), (11, 10));
        assert_eq!(state.player.facing, Direction::East);
    }

    /// `magic.md §6` Step 6: "The handler prints `Mixing...`, pauses briefly,
    /// then subtracts the requested quantity from each selected raw reagent
    /// counter." Step 7 compares the recipe only afterwards, so the line is
    /// emitted on the success outcome too.
    #[test]
    fn mmix_prints_the_mixing_beat_before_the_success_line() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 5;

        assert_eq!(
            state.mix_reagents_from_suffix("IL/0x80/1"),
            MoveOutcome::Cast
        );

        // The capture spaces the two: `Mixing...`, a blank row, `Done!`.
        let mut lines = state.message.lines();
        assert_eq!(lines.next(), Some(MMIX_MIXING_MESSAGE));
        assert_eq!(lines.next(), Some(""), "the completion line opens a block");
        assert_eq!(lines.next(), Some(MMIX_COMPLETION_MESSAGE));
        assert_eq!(lines.next(), None);
        assert_eq!(state.turn, 0, "magic.md §6: mixing costs no game time");
    }

    /// `magic.md §6` Step 6: "Because the line precedes the recipe
    /// comparison, it is emitted on both the success and the
    /// wrong-recipe/trap outcomes."
    #[test]
    fn mmix_prints_the_mixing_beat_before_the_wrong_recipe_trap() {
        let mut state = test_state(open_grid(), 1, 1);
        state.reagents = [0; REAGENT_COUNT];
        state.reagents[REAGENT_SULFUR_ASH] = 1;
        state.reagents[REAGENT_BLOOD_MOSS] = 1;

        assert_eq!(
            state.mix_reagents_from_suffix("AS/0x80/1"),
            MoveOutcome::Blocked
        );

        // `§6` step 6 prints `Mixing...` before step 7 compares the recipe,
        // so it is on the wrong-recipe path too. What follows it is the trap
        // resolver's own text and nothing between: step 7 gives the branch
        // "a line break" and no line of its own.
        //
        // Both are transcript rows rather than the message slot. They have
        // to be: the resolver prints its own published word through
        // `emit_message_line`, so a `Mixing...` *returned* to the slot would
        // be assigned after `ACID!` had already been pushed and the two
        // would come out in the wrong order.
        let rows: Vec<String> = state
            .message_entries()
            .iter()
            .rev()
            .take(4)
            .map(|entry| entry.text.clone())
            .filter(|text| !text.is_empty())
            .collect();
        let mixing = rows
            .iter()
            .position(|text| text == MMIX_MIXING_MESSAGE)
            .unwrap_or_else(|| panic!("`Mixing...` is not in the transcript tail: {rows:?}"));
        // `rows` is reversed, so the trap word sits *before* `Mixing...`.
        assert!(
            mixing > 0,
            "the trap line should follow `Mixing...` directly in {rows:?}",
        );
        let trap_line = &rows[mixing - 1];
        assert!(
            matches!(trap_line.as_str(), "ACID!" | "POISON!" | "BOMB!" | "GAS!"),
            "`traps.md §3` prints exactly one of the four words, got {trap_line:?}",
        );
        assert_eq!(state.reagents[REAGENT_SULFUR_ASH], 0);
    }

    /// `traps.md §3` effect id 2 (Bomb): the sweep rolls damage "for each
    /// in-party member of the six-slot band that is not marked Dead - the
    /// only status excluded is Dead". Ashes (`'A'`) is a distinct value and
    /// must fall through into the sweep.
    #[test]
    fn bomb_trap_damages_an_ashes_member_and_skips_only_dead() {
        let mut state = test_state(open_grid(), 1, 1);
        while state.party.len() < 3 {
            let mut extra = state.party[0];
            extra.slot = state.party.len() as u8;
            state.party.push(extra);
        }
        state.party[0].status = b'G';
        state.party[1].status = b'A';
        state.party[2].status = b'D';
        for index in 0..3 {
            state.party[index].hp = DEFAULT_PARTY_HP;
        }

        let report = state.apply_bomb_trap_effect(0);

        assert!(
            state.party[1].hp < DEFAULT_PARTY_HP,
            "Ashes member must take bomb damage; report was {report:?}"
        );
        assert_eq!(
            state.party[2].hp, DEFAULT_PARTY_HP,
            "Dead is the only status the bomb sweep skips"
        );
    }

    /// `karma.md §7`: "When the scan of all eight rows finds no match, the
    /// handler resolves the meditation to **Spirituality**. Meditating at a
    /// shrine that is not one of the seven mapped ones *is* the test for
    /// Spirituality, and \"no row matched\" must not be treated as an error."
    #[test]
    fn unmapped_shrine_altar_meditation_falls_through_to_spirituality() {
        let dir = debug_game_dir();
        std::fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 3 4 HONESTY\n").unwrap();
        let mut grid = open_world_grid();
        grid[world_cell_index(10, 20)] = SHRINE_ALTAR_TILE_FIRST;
        let mut state = world_state(grid, 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };

        let entry = state
            .current_shrine_entry(&dir)
            .unwrap()
            .expect("an altar tile off the coordinate table is still a meditation");
        assert_eq!(entry.virtue, ShrineVirtue::Spirituality);

        let _ = std::fs::remove_dir_all(dir);
    }

    /// `karma.md §7`: the fall-through is gated on the shrine altar family, so
    /// an ordinary surface tile still routes `M` to Mix Reagents rather than
    /// silently becoming a meditation.
    #[test]
    fn ordinary_surface_tile_is_not_a_meditation() {
        let dir = debug_game_dir();
        std::fs::write(dir.join(SHRINE_TABLE_FILE), "BRITANNIA 3 4 HONESTY\n").unwrap();
        let mut state = world_state(open_world_grid(), 10, 20);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };

        assert!(state.current_shrine_entry(&dir).unwrap().is_none());

        let _ = std::fs::remove_dir_all(dir);
    }

    /// `dungeon-mode.md §10` (u5-spec#298 Q28, R530): on foot in a dungeon X
    /// prints `X-it what?` and reports acted, so the post-action pass runs
    /// and the action counter moves; the loop-head minute is not billed twice.
    #[test]
    fn dungeon_x_on_foot_prints_xit_what_and_reports_acted() {
        let dir = debug_game_dir();
        for key in ['X', 'x'] {
            let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
            let minute_before = state.clock.minute;

            assert!(state.handle_dungeon_key(key, &dir).unwrap());

            assert_eq!(
                state.message_transcript.last().map(|entry| entry.text.as_str()),
                Some(format!("X-it {XIT_ON_FOOT_REFUSAL}").as_str())
            );
            assert_eq!(state.turn, 1, "`{key}` reports acted");
            assert_eq!(state.clock.minute, minute_before + 1);
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// `dungeon-mode.md §15`: "it does **not** gate that call on whether the
    /// command consumed a turn … so a command the dispatcher reports as \"no
    /// action\" (a digit solo-select, a refused Push, the typeahead toggle)
    /// still costs a minute underground. Only the dungeon post-action pass is
    /// gated on the status word."
    #[test]
    fn refused_dungeon_commands_still_cost_a_minute_but_no_action_turn() {
        let dir = debug_game_dir();
        // `X` is not here: on foot it reports acted (u5-spec#298 Q28).
        for key in ['B', 'E', 'F', 'P'] {
            let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
            let minute_before = state.clock.minute;

            assert!(state.handle_dungeon_key(key, &dir).unwrap());

            assert_eq!(
                state.turn, 0,
                "`{key}` reports no action, so the action counter must not move"
            );
            assert_eq!(
                state.clock.minute,
                minute_before + 1,
                "`{key}` must still cost the loop-head minute"
            );
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    /// `dungeon-mode.md §15`: "Dungeon turns advance the world clock at the
    /// indoor rate: one minute per loop iteration." The loop-head minute is
    /// charged once per iteration, so a turn-consuming dungeon command must
    /// not be billed twice for the same iteration.
    ///
    /// The probe is a forward step rather than `A` Attack: `commands.md §3`
    /// forwards the dungeon attack status verbatim and its row reads "`A`
    /// dungeon | Always \"no action\"", so `A` never bumps the action
    /// counter underground.
    #[test]
    fn a_turn_consuming_dungeon_command_is_charged_exactly_one_minute() {
        let dir = debug_game_dir();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        let minute_before = state.clock.minute;

        assert!(state.handle_dungeon_key('w', &dir).unwrap());

        assert_eq!(state.turn, 1);
        assert_eq!(state.clock.minute, minute_before + 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// `commands.md §3` forwarded-route table: "`A` dungeon | Always \"no
    /// action\"." Combined with `dungeon-mode.md §15` — "a command the
    /// dispatcher reports as \"no action\" … still costs a minute
    /// underground" — the dungeon attack spends the iteration's minute
    /// without spending an action.
    #[test]
    fn dungeon_attack_reports_no_action_but_still_costs_the_loop_head_minute() {
        let dir = debug_game_dir();
        let mut state = dungeon_state(open_dungeon_record(), 0, 1, 1);
        let minute_before = state.clock.minute;

        assert!(state.handle_dungeon_key('A', &dir).unwrap());

        assert_eq!(state.turn, 0);
        assert_eq!(state.clock.minute, minute_before + 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod codex_interred_scan_spec_tests {
    use crate::test_fixtures::{open_grid, test_state};
    use crate::*;

    fn guest(registry_slot: u8, scene_marker: u8, state: &PlayState) -> InnGuestRecord {
        InnGuestRecord {
            registry_slot,
            scene_marker,
            name: [0; SAVE_CHARACTER_NAME_LEN],
            member: state.party[0],
            strength: 10,
            intelligence: 10,
            experience: 0,
            equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
            stay_counter: 0,
        }
    }

    /// `karma.md §8` (u5-spec#298 Q5, `RETRACTIONS.md` R545): "The scan
    /// covers roster slots 1..15 (every record but the Avatar's) whether
    /// travelling or not, testing for whereabouts value `0x7F`".
    #[test]
    fn scan_covers_slots_one_to_fifteen_for_0x7f_only() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_roster = state.synced_party_roster();
        let template = state.party_roster[0].clone();
        state.party_roster.resize(SAVE_ROSTER_SLOT_COUNT, template);
        for (slot, text) in [(0, b"AVATAR".as_slice()), (3, b"Dupre"), (9, b"Katrina")] {
            let mut name = [0; SAVE_CHARACTER_NAME_LEN];
            name[..text.len()].copy_from_slice(text);
            state.party_roster[slot].name = name;
        }
        state.inn_registry = vec![
            guest(0, INTERRED_WHEREABOUTS, &state),
            guest(9, INTERRED_WHEREABOUTS, &state),
            guest(5, 0x02, &state),
            guest(3, INTERRED_WHEREABOUTS, &state),
        ];
        let names = state.codex_interred_names();
        assert_eq!(names.len(), 2, "the Avatar's slot 0 is outside the scan");
        assert_eq!(&names[0][..5], b"Dupre");
        assert_eq!(&names[1][..7], b"Katrina");
    }

    /// Playtest E65: `karma.md §8` - "The name is copied up to its first
    /// zero byte with no nine-character limit, so a name that fills all nine
    /// bytes runs on into the record bytes after the name field until a zero
    /// byte is met."
    #[test]
    fn a_nine_byte_name_runs_on_into_the_record_bytes() {
        let mut state = test_state(open_grid(), 1, 1);
        state.party_roster = state.synced_party_roster();
        let template = state.party_roster[0].clone();
        state.party_roster.resize(SAVE_ROSTER_SLOT_COUNT, template);
        let record = &mut state.party_roster[1];
        record.name = *b"abcdefghi";
        record.gender = SAVE_GENDER_FEMALE_BYTE;
        record.member.class_byte = b'F';
        record.member.status = b'G';
        record.strength = 0x41;
        record.member.climb_stat = 0x42;
        record.intelligence = 0x43;
        record.member.mana = 0;
        state.inn_registry = vec![guest(1, INTERRED_WHEREABOUTS, &state)];

        let page = codex_interred_party_page(&state.codex_interred_names()).unwrap();

        let run_on = format!(
            "ABCDEFGHI{}FGABC\n",
            char::from(SAVE_GENDER_FEMALE_BYTE)
        );
        assert!(page.ends_with(&run_on), "{page:?}");
    }
}

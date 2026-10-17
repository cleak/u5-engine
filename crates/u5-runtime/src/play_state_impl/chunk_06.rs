use std::io;
use std::path::Path;

use crate::*;

impl PlayState {
    /// The outdoor K-Klimb entry gates. The target cell is not read here:
    /// `RETRACTIONS.md` R470 withdrew the facing probe, so the command's own
    /// gate is the direction prompt this opens.
    pub fn climb_outdoors(
        &mut self,
        _game_dir: &Path,
        _plane: WorldPlane,
    ) -> io::Result<MoveOutcome> {
        // `doors-and-z-transitions.md §13`: "the overworld handler's value is
        // not forwarded at all, so its gear and on-foot refusals still report
        // 'acted' and still cost the outdoor turn" (`commands.md §3`).
        match overworld_klimb_entry_gate(self.climbing_gear != 0, self.player.transport.is_foot()) {
            OverworldKlimbEntryGate::NoGrapple => {
                self.advance_turn();
                self.message = "With what?".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            OverworldKlimbEntryGate::NotOnFoot => {
                self.advance_turn();
                self.message = "On foot!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            OverworldKlimbEntryGate::Proceed => {}
        }

        // `doors-and-z-transitions.md §9`, corrected 2026-09-12 for issue #262
        // (`RETRACTIONS.md` R470): "There is no facing to probe: that gate
        // **is** the direction prompt, it echoes the chosen direction's name on
        // the prefix row, and the pass key cancels the command outright." Both
        // gates above run first - the gear refusal "ends the command
        // **without ever asking for a direction**".
        Ok(self.start_klimb_direction_prompt())
    }

    pub fn climb_outdoors_direction(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let nx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ny = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        let tile = self.grid[world_cell_index(nx, ny)];

        // `doors-and-z-transitions.md §9`, as answered on `cleak/u5-spec#265`:
        // "After the direction prompt the handler reads the **map tile** at
        // the party's cell plus the chosen step and goes straight into the two
        // id compares." Two whole branches this engine had are removed by that
        // sentence:
        //
        //   - the object test. "There is none ... It never scans the active
        //     object table, so an object standing on a `0x0C` cell does not
        //     stop the climb and an object on any other cell changes nothing
        //     about the refusal."
        //   - the world-damage and walkability tests. "Water, lava and every
        //     other non-mountain id are simply 'not `0x0C`' and print `Not
        //     climbable!`. No walkability, terrain-damage or sidecar
        //     predicate is consulted anywhere in the handler."
        //
        // "The only things read before the id tests are the Grapple flag, the
        // transport byte and the direction" - the two gates the caller has
        // already run.
        //
        // The order is fixed: `0x0D` first, then `0x0C`.
        if tile == OVERWORLD_KLIMB_PEAKS_TILE {
            // "`0x0D` (the tile catalog's *peaks*) is the blocked variant and
            // prints `Impassable!`." Not forwarded, so it costs the turn
            // (§13).
            self.advance_turn();
            self.message = "Impassable!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if tile != OVERWORLD_KLIMB_MOUNTAINS_TILE {
            // "`0x0C` (*mountains*) is the one climbable identity. Every other
            // id prints `Not climbable!`." Not forwarded either (§13).
            self.advance_turn();
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let _ = (game_dir, plane);

        let (_checked, falls) = self.apply_outdoor_climb_fall_checks();
        self.player.x = nx;
        self.player.y = ny;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        let fall_report = falls.join("\n");
        // `doors-and-z-transitions.md §9`: the outdoor climb's only published
        // output is `Fell!` for a member whose Dexterity roll failed - "the
        // successful path calls the resident climb/move helper", printing
        // nothing of its own. The line this replaced named the destination
        // coordinate and the plane, neither of which the original prints
        // (`commands.md §8.1`).
        self.message = fall_report;
        if let Some(entry) = self.world_plane_transition_at(game_dir, plane, nx, ny)? {
            let to_plane = entry.to_plane;
            let climb_message = self.message.clone();
            self.apply_world_plane_transition(game_dir, entry)?;
            let transition_message = self.message.clone();
            // A clean climb prints nothing, so the separator only belongs
            // here when a `Fell!` line precedes the transition.
            self.message = if climb_message.is_empty() {
                transition_message
            } else {
                format!("{climb_message} {transition_message}")
            };
            return Ok(MoveOutcome::Transition(AreaTransition::ChangedWorldPlane {
                from: plane,
                to: to_plane,
            }));
        }
        Ok(MoveOutcome::Moved)
    }

    pub fn apply_outdoor_climb_fall_checks(&mut self) -> (usize, Vec<String>) {
        let mut checked = 0;
        let mut falls = Vec::new();
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            checked += 1;
            let roll = self.outdoor_climb_stat_roll();
            if outdoor_klimb_member_falls(self.party[index].climb_stat, roll) {
                let damage = self.outdoor_climb_damage_roll();
                let _applied = self.party[index].apply_damage(damage);
                // `doors-and-z-transitions.md §9`: the printed line is the bare
                // `Fell!`. The slot number and HP arithmetic that used to
                // follow it were the engine's own; the panel is where the
                // player reads the damage (`stats-panel.md §4`).
                falls.push(OUTDOOR_CLIMB_FALL_REFUSAL.to_string());
            }
        }
        (checked, falls)
    }

    pub fn outdoor_climb_stat_roll(&mut self) -> u8 {
        self.random_range_u8(1, 30)
    }

    pub fn outdoor_climb_damage_roll(&mut self) -> u8 {
        self.random_range_u8(1, 5)
    }

    /// The frigate's facing on the overworld, read from the transport
    /// marker's low two bits (`vehicles.md §2`); `None` off the overworld or
    /// off a frigate. `vehicles.md §6`: "No prefix in town-family scenes."
    pub fn overworld_frigate_facing(&self) -> Option<Direction> {
        if !matches!(self.area, Area::World { .. })
            || !matches!(self.player.transport, TransportState::Ship { .. })
        {
            return None;
        }
        transport_marker_facing(self.player.transport.save_marker())
    }

    /// `vehicles.md §6` / `commands.md §5.2` (u5-spec#297, `RETRACTIONS.md`
    /// R524): "Under hoisted sail a key that keeps the facing prints
    /// nothing" - no compass word, pressed or synthesised. Such a key opens
    /// no verb echo at all.
    pub fn frigate_movement_key_prints_nothing(&self, key: char) -> bool {
        if key == ' ' || key.is_ascii_uppercase() || !self.player.transport.is_ship_under_sail() {
            return false;
        }
        let Some(direction) = Direction::from_play_key(key) else {
            return false;
        };
        direction.is_cardinal() && self.overworld_frigate_facing() == Some(direction)
    }

    /// `vehicles.md §6` (u5-spec#297): on the overworld "a key that changes
    /// the facing prints `Head ` plus the direction word and a line feed (for
    /// example `Head West`), turns the ship in place without moving it, and
    /// consumes a full turn, in calm wind too", in either sail state. The
    /// `Head <Dir>` row is the verb echo
    /// ([`Self::vehicle_prefixed_movement_echo`]); this is the handler's half.
    ///
    /// "**Hull warning.** Immediately after a `Head <Dir>` line, if the hull
    /// condition is below 50 (0 to 49), the game prints `Hull weak!`. It
    /// prints on every frigate facing change in either sail state, and on
    /// nothing else."
    ///
    /// Under hoisted sail the same command is still a movement command, so
    /// `weather.md §5.1`'s one setter stores the heading in the cache and
    /// zeroes the counter when it differs. The caller has already written the
    /// new facing into the marker.
    pub(crate) fn turn_overworld_frigate_in_place(&mut self, direction: Direction) -> MoveOutcome {
        let TransportState::Ship {
            sails_hoisted,
            hull,
            ..
        } = self.player.transport
        else {
            unreachable!("only a frigate turns in place");
        };
        if sails_hoisted {
            if self.sail_cached_direction != Some(direction) {
                self.sail_cached_direction = Some(direction);
                self.sail_cadence = 0;
            }
            self.advance_sailing_wait_turn();
        } else {
            self.advance_turn();
        }
        // `text-output.md §10.2` (R526): the facing change sets the prompt
        // flag again, so a course change is followed by an ordinary pair.
        self.rearm_overworld_prompt_pair_in_command();
        if hull < FRIGATE_HULL_WEAK_THRESHOLD {
            self.message = FRIGATE_HULL_WEAK_LINE.to_string();
            self.message_row_open_mid_line = false;
        } else {
            self.message.clear();
        }
        MoveOutcome::SailTurned
    }

    pub fn resolve_sailed_ship_wind_gate(&mut self, direction: Direction) -> Option<MoveOutcome> {
        if !self.player.transport.is_ship_under_sail() {
            return None;
        }
        if !direction.is_cardinal() {
            // Measured: a diagonal under sail is the resident `What?`
            // refusal, not a line of its own.
            self.message = unassigned_refusal_echo(0).to_string();
            return Some(MoveOutcome::Blocked);
        }
        // `weather.md §5.1`, "**The one setter.**": "A movement command taken
        // while the party's transport marker is a hoisted frigate
        // (`0x20`-`0x23`) stores the requested heading into the cache." Two
        // published details: "The stored value is always a real movement
        // direction and is **never zero**, so **a new heading replaces the
        // cache rather than clearing it.** An engine that clears first and
        // then re-sets exposes a momentarily empty cache the original never
        // has." And: "The store runs only when the requested heading
        // **differs** from the cached one, and it is the same step that
        // zeroes the sailing counter. Repeating the heading already cached
        // changes neither the cache nor the counter, and in particular does
        // not restart the wait."
        //
        // This is where the cache is written, so it covers the player's own
        // command and the auto-advance route's synthesized repeat alike - and
        // the synthesized repeat always matches the cache, which is exactly
        // why it must not reset the wait counter.
        //
        // `weather.md §5` (corrected 2026-10-16, u5-spec#297, `RETRACTIONS.md`
        // R525): "Whether the ship turns or moves is decided against the
        // ship's **facing**, not the cache." A key that changes the facing
        // never reaches this gate - [`Self::turn_overworld_frigate_in_place`]
        // takes it in `step_with_game_dir` - so the facing is unchanged here.
        // "If the facing is unchanged, the step is attempted on that same
        // command in any non-calm wind, even upwind, and in calm wind the
        // command is consumed silently. That same-command step is reachable
        // only while the cache is empty - just after hoisting, after an
        // under-sail refusal, or after Sheets in irons - because in steady
        // sailing the cache equals the facing and a pressed key equal to it is
        // swallowed by the input helper's auto-advance loop", where the
        // cadence table below decides the move. The engine used to turn
        // without moving on any key that differed from the cache (R525's
        // withdrawn text), which made a just-hoisted ship spend a turn on a
        // key equal to its facing.
        if self.sail_cached_direction != Some(direction) {
            self.sail_cached_direction = Some(direction);
            self.sail_cadence = 0;
            if self.wind.player_sail_wait_ticks(direction).is_none() {
                self.advance_sailing_wait_turn();
                self.message.clear();
                return Some(MoveOutcome::SailStalled);
            }
            return None;
        }

        let Some(wait_ticks) = self.wind.player_sail_wait_ticks(direction) else {
            self.sail_cadence = 0;
            self.advance_sailing_wait_turn();
            // Measured: the stalled attempt itself is silent; the line
            // lands on the Pass that follows.
            self.message.clear();
            return Some(MoveOutcome::SailStalled);
        };

        if self.sail_cadence >= wait_ticks {
            self.sail_cadence = 0;
            return None;
        }

        self.sail_cadence = self.sail_cadence.saturating_add(1);
        self.advance_sailing_wait_turn();
        // `weather.md §5` itemises a wait pass - cleanup/redraw, clock,
        // outdoor epilogue, world step, counter, key poll - and no line of
        // its own. This arm used to print an invented
        // `Sails stalled by <wind> wind while heading <dir>.`, which no spec
        // text carries. `sail-heading-echo` (2026-10-16) showed it filling
        // three rows of the window under an upwind heading on this engine's
        // side; the stock side of that run had a crosswind and released at
        // once, so the silent wait is the spec's, not a capture of one.
        self.message.clear();
        Some(MoveOutcome::SailStalled)
    }

    // `vehicles.md §11` "Balloon boundary": "Settled, not merely
    // untraced. Balloon sprites are catalog assets only... Do not invent
    // boarding, landing, or wind-driven balloon movement." The
    // `resolve_balloon_wind_step` helper that used to live here invented
    // exactly that (wind-driven drift plus a calm-wind stall message) and
    // is removed with the family.

    #[cfg(test)]
    pub fn open_facing(&mut self) -> MoveOutcome {
        self.open_facing_with_game_dir(None)
            .expect("open without a game dir cannot load sidecar metadata")
    }

    pub fn open_facing_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.open_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn open_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let (scene, floor) = match self.area {
            Area::Town { scene, floor } => (scene, floor),
            Area::Dungeon { scene, level } => {
                return self.open_dungeon_underfoot(game_dir, scene, level);
            }
            Area::World { .. } => {
                let (dx, dy) = direction.delta();
                let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                if self.surface_object_chest_slot_at(tx, ty).is_some() {
                    return Ok(self.begin_surface_object_chest_interaction(
                        tx,
                        ty,
                        SurfaceChestVerb::Open,
                    ));
                }
                // `doors-and-z-transitions.md §4`: "Every resolved
                // non-dungeon Open attempt returns the dispatcher's ordinary
                // committed-action status", the generic refusal included.
                self.advance_turn();
                self.message = "Nothing to open!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
        };
        self.tick_door_tracker();
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.advance_turn_without_door_tick();
            self.message = "Nothing to open!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        // `doors-and-z-transitions.md §2`: "Open alone treats exact tile
        // `0xAF` as already open". A door Open cleared is the ordinary `0x44`
        // floor byte and falls through the cascade like any other cell.
        if tile == TOWN_OPEN_ALREADY_OPEN_TILE {
            self.advance_turn_without_door_tick();
            self.message = "It's open!".to_string();
            return Ok(MoveOutcome::DoorOpened);
        }
        if tile == TOWN_OPEN_TOO_HEAVY_TILE {
            self.advance_turn_without_door_tick();
            self.message = "Too heavy!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        // `RETRACTIONS.md` R448 withdraws R412's "revealed town/dwelling
        // secret doors respond like ordinary unlocked doors":
        // `doors-and-z-transitions.md` §8 now says the reveal "obeys the
        // resulting lock state" - the ground/upper-floor `0xB9` is an
        // ordinary *locked* door and the below-ground `0xB8` an ordinary
        // closed one - so the reveal gets no special case at all.
        if jimmy_locked_door_rewrite(tile).is_some()
            || jimmy_magic_locked_door(tile)
            || self
                .town_lock_at(game_dir, scene, floor, tx, ty, tile)?
                .is_some()
        {
            // `doors-and-z-transitions.md §4`: the locked refusal commits the
            // action like every other resolved non-dungeon Open.
            self.advance_turn_without_door_tick();
            self.message = "Locked!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if self.surface_object_chest_slot_at(tx, ty).is_some() {
            return Ok(self.begin_surface_object_chest_interaction(tx, ty, SurfaceChestVerb::Open));
        }
        if !openable_town_door(tile) {
            self.advance_turn_without_door_tick();
            self.message = "Nothing to open!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.grid[idx] = TOWN_DOOR_CLEARED_TILE;
        self.record_open_town_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn_without_door_tick();
        // R448: the below-ground reveal "accepts O-Open, prints `Opened!` and
        // clears the cell through the ordinary door-opening path", auto-close
        // tracker included.
        self.door_tracker = Some(DoorTracker {
            previous_tile: tile,
            x: tx,
            y: ty,
            turns_remaining: DOOR_AUTO_CLOSE_TURNS,
        });
        self.door_tracker_closed = false;
        self.message = "Opened!".to_string();
        Ok(MoveOutcome::DoorOpened)
    }

    pub fn open_dungeon_underfoot(
        &mut self,
        _game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<MoveOutcome> {
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        // `dungeon-mode.md §8.1`, "Who acts": "Dungeon L-Look, Search, Jimmy
        // and Open all run the shared acting-member prompt before anything
        // else", so the cell is classified only after the member is chosen
        // (playtest E12) - by [`Self::finish_open_dungeon_chest`], which also
        // answers the already-open and `What?` cells.
        Ok(self.open_dungeon_chest(scene, level, self.player.x, self.player.y, idx, tile))
    }

    pub fn start_jimmy_party_prompt(&mut self, direction: Direction) -> MoveOutcome {
        self.active_jimmy = Some(JimmySession::new(direction));
        self.message = self.render_active_jimmy();
        MoveOutcome::Observed
    }

    pub fn render_active_jimmy(&self) -> String {
        PARTY_SELECTION_PROMPT.to_string()
    }

    pub fn step_active_jimmy(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(session) = self.active_jimmy.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            if matches!(ch, '\u{1b}' | ' ' | '0' | '\r' | '\n') {
                self.advance_turn();
                self.message = "None!".to_string();
                return Ok(Some(MoveOutcome::PromptDeclined));
            }
            let Some(digit) = ch
                .to_digit(10)
                .and_then(|digit| usize::try_from(digit).ok())
            else {
                continue;
            };
            if !(1..=self.party.len().min(6)).contains(&digit) {
                continue;
            }
            // `dungeon-mode.md §8.1`, "Who acts": the shared acting-member
            // prompt "prints `Player: ` otherwise, echoing the chosen
            // member's name" - as Open, Look and Search already do
            // (playtest E13).
            if matches!(self.area, Area::Dungeon { .. }) {
                self.record_container_picker_name_echo(digit - 1);
            }
            let outcome = self.jimmy_direction_with_game_dir_and_member(
                session.direction,
                Some(game_dir),
                Some(digit - 1),
            )?;
            return Ok(Some(outcome));
        }
        self.active_jimmy = Some(session);
        self.message = self.render_active_jimmy();
        Ok(None)
    }

    /// `traps.md §2.1` / `containers.md`: the surface/town container site
    /// chooses its acting member through the shared acting-member
    /// selection, *before* testing whether the container is trapped. A
    /// prompt is only **one** of that selection's three outcomes: a
    /// combat-class scene and a set active character both bypass it
    /// silently, and a party with exactly one Good-or-Poisoned member
    /// selects that member silently, with no prompt and no name echo.
    /// When nobody qualifies the command reports it and aborts, before
    /// any trap can fire.
    pub fn begin_surface_object_chest_interaction(
        &mut self,
        x: usize,
        y: usize,
        verb: SurfaceChestVerb,
    ) -> MoveOutcome {
        match self.surface_container_acting_member() {
            ActingMemberSelection::Selected(member_index) => self
                .consume_surface_object_chest_at(x, y, member_index, verb.label())
                .unwrap_or_else(|| {
                    self.message = "Nothing to open!".to_string();
                    MoveOutcome::Blocked
                }),
            ActingMemberSelection::Prompt => self.start_surface_object_chest_prompt(x, y, verb),
            ActingMemberSelection::NoneAble => {
                // The shared acting-member answer when nobody is eligible
                // (`dungeon-mode.md §11`).
                self.message = DUNGEON_ACTING_MEMBER_NONE.to_string();
                MoveOutcome::Blocked
            }
        }
    }

    /// `traps.md §2.1` branch 3, two-or-more case: open the interactive
    /// picker. Reached only from
    /// [`Self::begin_surface_object_chest_interaction`] in ordinary play.
    pub fn start_surface_object_chest_prompt(
        &mut self,
        x: usize,
        y: usize,
        verb: SurfaceChestVerb,
    ) -> MoveOutcome {
        self.active_surface_chest = Some(SurfaceChestSession::new(x, y, verb));
        self.message = self.render_active_surface_chest();
        MoveOutcome::Observed
    }

    pub fn render_active_surface_chest(&self) -> String {
        PARTY_SELECTION_PROMPT.to_string()
    }

    pub fn step_active_surface_chest(
        &mut self,
        key: char,
        suffix: &str,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(session) = self.active_surface_chest.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            if matches!(ch, '\u{1b}' | ' ' | '0' | '\r' | '\n') {
                self.message = "None!".to_string();
                return Ok(Some(MoveOutcome::PromptDeclined));
            }
            let Some(digit) = ch
                .to_digit(10)
                .and_then(|digit| usize::try_from(digit).ok())
            else {
                continue;
            };
            if !(1..=self.party.len().min(6)).contains(&digit) {
                continue;
            }
            let member_index = digit - 1;
            if !self.surface_chest_member_available(member_index) {
                // `traps.md §2.1`: a confirmed pick that is not
                // Good-or-Poisoned is rejected with the short "disabled"
                // notice and **the prompt repeats** - it does not abort.
                // `dungeon-mode.md §11` publishes that notice.
                self.message = DUNGEON_ACTING_MEMBER_DISABLED.to_string();
                self.active_surface_chest = Some(session);
                return Ok(None);
            }
            // `traps.md §2.1`: only a prompted choice echoes the selected
            // member's name. The silent single-match and active-member
            // branches deliberately bypass this helper.
            self.record_container_picker_name_echo(member_index);
            let outcome = if let Some(dungeon) = session.dungeon {
                self.finish_open_dungeon_chest(
                    dungeon.scene,
                    dungeon.level,
                    session.x,
                    session.y,
                    dungeon.index,
                    dungeon.tile,
                    member_index,
                )
            } else if session.verb == SurfaceChestVerb::Search {
                self.finish_moldy_corpse_search_at(session.x, session.y, member_index)
                    .or_else(|| {
                        self.finish_surface_object_trap_search_at(
                            session.x,
                            session.y,
                            member_index,
                        )
                    })
                    .unwrap_or_else(|| {
                        // `commands.md §5.8`: the Search command's own
                        // empty result, not a second wording for it.
                        self.message = SEARCH_NOTHING_FOUND.to_string();
                        MoveOutcome::Blocked
                    })
            } else {
                self.consume_surface_object_chest_at(
                    session.x,
                    session.y,
                    member_index,
                    session.verb.label(),
                )
                .unwrap_or_else(|| {
                    self.message = "Nothing to open!".to_string();
                    MoveOutcome::Blocked
                })
            };
            return Ok(Some(outcome));
        }
        self.active_surface_chest = Some(session);
        self.message = self.render_active_surface_chest();
        Ok(None)
    }

    /// `traps.md §2.1` branch 3: the picker accepts exactly the statuses
    /// the scan accepts - Good or Poisoned. `conscious()` is a wider gate
    /// (it admits Charmed, among others) and was the wrong test here.
    pub fn surface_chest_member_available(&self, member_index: usize) -> bool {
        self.party
            .get(member_index)
            .is_some_and(|member| acting_member_status_eligible(member.status))
    }

    fn record_container_picker_name_echo(&mut self, member_index: usize) {
        let Some(name) = self
            .party_names
            .get(member_index)
            .and_then(|name| party_name_to_string(name))
        else {
            // Saved parties validate every in-party name before play. A
            // synthetic fixture may omit one; do not invent a replacement
            // line for a name the runtime was never given.
            return;
        };
        self.message = name;
        self.flush_message_slot();
    }

    #[cfg(test)]
    pub fn jimmy_facing(&mut self) -> MoveOutcome {
        self.jimmy_facing_with_game_dir_and_member(None, Some(0))
            .expect("jimmy without a game dir cannot load sidecar metadata")
    }

    pub fn jimmy_facing_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_facing_with_game_dir_and_member(game_dir, Some(0))
    }

    /// Measured 2026-09-07: the skull key acts on a *prompted* direction,
    /// not on the party's facing.
    pub fn use_skull_key_direction(
        &mut self,
        game_dir: Option<&Path>,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let facing = self.player.facing;
        self.player.facing = direction;
        // The picker spent the key when the row was accepted, so this is the
        // lock attempt alone.
        let outcome = self.use_skull_key_dispatch(game_dir);
        self.player.facing = facing;
        outcome
    }

    /// `inventory.md §7`: "Skull Key | **Decrements the skull-key/special-key
    /// counter, then** asks for a cardinal target and runs the lock helper".
    ///
    /// The counter is spent when the row is accepted, before the direction is
    /// asked for - not when the lock is tried. Measured
    /// (`qa/paired/use-specials.tsv`, beat `amuletrow`): the original's picker
    /// has already lost its `Skull Keys` row by the next beat, and this
    /// engine's still listed it because the spend waited for the direction.
    /// A target outside the map also skipped the spend entirely.
    pub fn spend_skull_key(&mut self) {
        self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] =
            self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX].saturating_sub(1);
    }

    pub fn use_skull_key(&mut self, game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        if self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        }
        // `inventory.md §7.1`: "One skull key is already spent before the
        // prompt or the dungeon refusal."
        self.spend_skull_key();
        self.use_skull_key_dispatch(game_dir)
    }

    /// `inventory.md §7`: "Dungeon exploration refuses through this path";
    /// a dungeon-room combat is an arena and runs the helper.
    pub fn skull_key_refused_here(&self) -> bool {
        matches!(self.area, Area::Dungeon { .. }) && !self.combat_active
    }

    /// The lock attempt itself, with the counter already spent, on the cell
    /// in the party's (prompted) facing.
    ///
    /// `inventory.md §7.1` "Skull Key targeting": "A selected target of tile
    /// `0x97` becomes `0xB8`, and `0x98` becomes `0xBA`; either success
    /// dirties the map and adds no result sentence. Other target tiles print
    /// `Failed!\n` with the ordinary Use failure sound, not Jimmy's `No
    /// lock!`." "Combat runs this same directed unlock helper" on the arena
    /// cell beside the acting combatant.
    pub fn use_skull_key_dispatch(&mut self, _game_dir: Option<&Path>) -> io::Result<MoveOutcome> {
        if self.skull_key_refused_here() {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let (dx, dy) = self.player.facing.delta();
        let target = if self.combat_active {
            self.pending_combat_actor_slot
                .and_then(|slot| self.combat_actors.get(slot).copied())
                .and_then(|actor| {
                    let tx = isize::from(actor.x) + dx;
                    let ty = isize::from(actor.y) + dy;
                    ((0..COMBAT_ARENA_SIDE as isize).contains(&tx)
                        && (0..COMBAT_ARENA_SIDE as isize).contains(&ty))
                    .then_some((tx as usize, ty as usize))
                })
        } else {
            match self.area {
                Area::World { .. } => Some((
                    (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize,
                    (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize,
                )),
                Area::Town { .. } => {
                    let tx = self.player.x as isize + dx;
                    let ty = self.player.y as isize + dy;
                    ((0..32).contains(&tx) && (0..32).contains(&ty))
                        .then_some((tx as usize, ty as usize))
                }
                Area::Dungeon { .. } => None,
            }
        };
        self.advance_turn();
        let rewritten = target.and_then(|(tx, ty)| {
            let tile = if self.combat_active {
                self.combat_terrain[ty][tx]
            } else {
                self.current_area_tile(tx, ty)
            };
            let unlocked = skull_key_unlock_rewrite(tile)?;
            if self.combat_active {
                self.combat_terrain[ty][tx] = unlocked;
            } else {
                match self.area {
                    Area::World { .. } => {
                        self.grid[world_cell_index(tx, ty)] = unlocked;
                        let _ = self.refresh_world_live_chunks_for_current_area();
                    }
                    Area::Town { .. } => self.grid[ty * 32 + tx] = unlocked,
                    Area::Dungeon { .. } => return None,
                }
            }
            Some(unlocked)
        });
        if rewritten.is_some() {
            self.mark_visibility_dirty();
            self.message.clear();
            return Ok(MoveOutcome::LockTried);
        }
        self.message = USE_SKULL_KEY_FAILED.to_string();
        self.emit_sound_effect(SoundEffect::CastFailure);
        Ok(MoveOutcome::LockTried)
    }

    pub fn jimmy_facing_with_game_dir_and_member(
        &mut self,
        game_dir: Option<&Path>,
        member_index: Option<usize>,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_direction_with_game_dir_and_member(self.player.facing, game_dir, member_index)
    }

    pub fn jimmy_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_direction_with_game_dir_and_member(direction, game_dir, Some(0))
    }

    pub fn jimmy_direction_with_game_dir_and_member(
        &mut self,
        direction: Direction,
        game_dir: Option<&Path>,
        member_index: Option<usize>,
    ) -> io::Result<MoveOutcome> {
        if let Area::Dungeon { scene, level } = self.area {
            // `dungeon-mode.md §11`: "Dungeon L-Look, Search, Jimmy and
            // Open all run the shared" acting-member selection, which
            // picks silently when only one member can act and prompts
            // only when the choice is real. This arm opened the picker
            // unconditionally: measured 2026-09-19
            // (`qa/paired/dungeon-chest-jimmy.tsv`, beat `jimmy`), where
            // the original answers a one-member party with `Jimmy-` /
            // `Chest unlocked` and this engine printed `Jimmy-Player:`.
            let member_index = match member_index {
                Some(index) => Some(index),
                None => match self.dungeon_container_acting_member() {
                    ActingMemberSelection::Selected(slot) => Some(slot),
                    ActingMemberSelection::Prompt => None,
                    ActingMemberSelection::NoneAble => {
                        self.advance_turn();
                        self.message.clear(); // audit: not a player-facing line
                        return Ok(MoveOutcome::Blocked);
                    }
                },
            };
            let Some(member_index) = member_index else {
                return Ok(self.start_jimmy_party_prompt(direction));
            };
            let Some(member_index) = self.resolve_jimmy_member_index(Some(member_index)) else {
                self.advance_turn();
                return Ok(MoveOutcome::PromptDeclined);
            };
            return self.jimmy_dungeon_underfoot(game_dir, scene, level, member_index);
        }
        if self.keys == 0 {
            self.advance_turn();
            self.message = "No keys!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        match self.area {
            Area::Town { scene, floor } => {
                self.jimmy_town_preflight(game_dir, scene, floor, member_index, direction)
            }
            Area::World { .. } => {
                let (dx, dy) = direction.delta();
                let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                if self.surface_object_chest_slot_at(tx, ty).is_none() {
                    self.advance_turn();
                    self.message = "No lock!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                let Some(member_index) = member_index else {
                    return Ok(self.start_jimmy_party_prompt(direction));
                };
                let Some(member_index) = self.resolve_jimmy_member_index(Some(member_index)) else {
                    self.advance_turn();
                    return Ok(MoveOutcome::PromptDeclined);
                };
                Ok(self
                    .jimmy_surface_object_chest_at(tx, ty, member_index)
                    .expect("world Jimmy preflight found the target container"))
            }
            Area::Dungeon { .. } => unreachable!("dungeon Jimmy returns before key preflight"),
        }
    }

    fn jimmy_town_preflight(
        &mut self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
        member_index: Option<usize>,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.advance_turn();
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let tile = self.grid[ty * 32 + tx];
        // `RETRACTIONS.md` R448 withdraws R412's "revealed town/dwelling
        // secret doors respond like ordinary unlocked doors":
        // `doors-and-z-transitions.md` §8 now says the reveal "obeys the
        // resulting lock state" - the ground/upper-floor `0xB9` is an
        // ordinary *locked* door and the below-ground `0xB8` an ordinary
        // closed one - so the reveal gets no special case at all.
        let sidecar_lock = self.town_lock_at(game_dir, scene, floor, tx, ty, tile)?;

        if jimmy_magic_locked_door(tile)
            || sidecar_lock.is_some_and(|entry| entry.kind == TownLockKind::Magic)
        {
            self.advance_turn();
            self.message = "Key broke!".to_string();
            // audio.md §8.1 Jimmy key breaks: failure only — the break line,
            // then the 40-update action snap, then the key decrement.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
            return Ok(MoveOutcome::LockTried);
        }

        if jimmy_restraint_tile(tile) {
            if self.object_slot_at_current_floor(tx, ty).is_none() {
                self.advance_turn();
                self.message = "No one is there!".to_string();
                return Ok(MoveOutcome::LockTried);
            }
        } else if sidecar_lock.is_none()
            && jimmy_locked_door_rewrite(tile).is_none()
            && self.surface_object_chest_slot_at(tx, ty).is_none()
        {
            self.advance_turn();
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let Some(member_index) = member_index else {
            return Ok(self.start_jimmy_party_prompt(direction));
        };
        let Some(member_index) = self.resolve_jimmy_member_index(Some(member_index)) else {
            self.advance_turn();
            return Ok(MoveOutcome::PromptDeclined);
        };
        self.jimmy_town_direction(game_dir, scene, floor, member_index, direction)
    }

    pub fn resolve_jimmy_member_index(&mut self, member_index: Option<usize>) -> Option<usize> {
        let Some(member_index) = member_index else {
            self.message = PARTY_SELECTION_PROMPT.to_string();
            return None;
        };
        if !self
            .party
            .get(member_index)
            .is_some_and(|member| member.conscious())
        {
            // Out-of-range member: the interactive pickers bound the
            // index, so only the inline harness form reaches this. The
            // original has no line for a state it cannot enter.
            self.message.clear();
            return None;
        }
        Some(member_index)
    }

    pub fn jimmy_town_facing(
        &mut self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
        member_index: usize,
    ) -> io::Result<MoveOutcome> {
        self.jimmy_town_direction(game_dir, scene, floor, member_index, self.player.facing)
    }

    pub fn jimmy_town_direction(
        &mut self,
        game_dir: Option<&Path>,
        scene: Scene,
        floor: i8,
        member_index: usize,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = "No lock!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        // `RETRACTIONS.md` R448 withdraws R412's "revealed town/dwelling
        // secret doors respond like ordinary unlocked doors":
        // `doors-and-z-transitions.md` §8 now says the reveal "obeys the
        // resulting lock state" - the ground/upper-floor `0xB9` is an
        // ordinary *locked* door and the below-ground `0xB8` an ordinary
        // closed one - so the reveal gets no special case at all.
        if jimmy_magic_locked_door(tile) {
            self.advance_turn();
            self.message = "Key broke!".to_string();
            // audio.md §8.1 Jimmy key breaks: failure only — the break line,
            // then the 40-update action snap, then the key decrement.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
            return Ok(MoveOutcome::LockTried);
        }
        if let Some(entry) = self.town_lock_at(game_dir, scene, floor, tx, ty, tile)? {
            if entry.kind == TownLockKind::Magic {
                self.advance_turn();
                self.message = "Key broke!".to_string();
                // audio.md §8.1 Jimmy key breaks: failure only — the break line,
                // then the 40-update action snap, then the key decrement.
                self.emit_sound_effect(SoundEffect::ActionSnap);
                self.keys = self.keys.saturating_sub(1);
                return Ok(MoveOutcome::LockTried);
            }
            if !self.jimmy_lock_pick_succeeds(member_index) {
                self.advance_turn();
                self.message = "Key broke!".to_string();
                // audio.md §8.1 Jimmy key breaks: failure only — the break line,
                // then the 40-update action snap, then the key decrement.
                self.emit_sound_effect(SoundEffect::ActionSnap);
                self.keys = self.keys.saturating_sub(1);
                return Ok(MoveOutcome::LockTried);
            }
            self.grid[idx] = entry.unlocked_tile;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = "Unlocked!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        if let Some(unlocked_tile) = Self::visible_jimmy_unlock_tile(tile) {
            if !self.jimmy_lock_pick_succeeds(member_index) {
                self.advance_turn();
                self.message = "Key broke!".to_string();
                // audio.md §8.1 Jimmy key breaks: failure only — the break line,
                // then the 40-update action snap, then the key decrement.
                self.emit_sound_effect(SoundEffect::ActionSnap);
                self.keys = self.keys.saturating_sub(1);
                return Ok(MoveOutcome::LockTried);
            }
            self.grid[idx] = unlocked_tile;
            self.mark_visibility_dirty();
            self.advance_turn();
            self.message = "Unlocked!".to_string();
            return Ok(MoveOutcome::LockTried);
        }
        if jimmy_restraint_tile(tile) {
            return Ok(self.jimmy_town_restraint_at(scene, tx, ty, member_index));
        }
        if let Some(outcome) = self.jimmy_surface_object_chest_at(tx, ty, member_index) {
            return Ok(outcome);
        }
        self.advance_turn();
        self.message = "No lock!".to_string();
        Ok(MoveOutcome::Blocked)
    }

    fn jimmy_town_restraint_at(
        &mut self,
        scene: Scene,
        x: usize,
        y: usize,
        member_index: usize,
    ) -> MoveOutcome {
        let Some(object_slot) = self
            .object_slot_at_current_floor(x, y)
            .map(|(slot, _)| slot)
        else {
            self.advance_turn();
            self.message = "No one is there!".to_string();
            return MoveOutcome::LockTried;
        };
        if !self.jimmy_lock_pick_succeeds(member_index) {
            self.advance_turn();
            self.message = "Key broke!".to_string();
            // audio.md §8.1 Jimmy key breaks: failure only — the break line,
            // then the 40-update action snap, then the key decrement.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
            return MoveOutcome::LockTried;
        }
        let Some(npc_index) = self
            .npcs
            .iter()
            .position(|npc| npc.active_object == Some(object_slot))
        else {
            self.advance_turn();
            self.message.clear();
            return MoveOutcome::LockTried;
        };

        let npc_slot = self.npcs[npc_index].slot;
        let type_byte = self.npcs[npc_index].type_byte;
        self.npcs[npc_index].dialog_id = NPC_DIALOG_ID_NONE;
        let already_removed = self
            .removed_town_npc_flags
            .get(&scene.byte)
            .is_some_and(|mask| {
                1u32.checked_shl(npc_slot as u32)
                    .is_some_and(|bit| *mask & bit != 0)
            });
        if already_removed {
            self.message.clear();
        } else {
            self.npcs[npc_index].schedule
                [NPC_SCHEDULE_AI_OFFSET..NPC_SCHEDULE_AI_OFFSET + NPC_SCHEDULE_WAYPOINT_COUNT]
                .fill(JIMMY_RELEASE_AI_MODE);
            self.add_moral_standing(JIMMY_PRISONER_RELEASE_KARMA_REWARD);
            if town_npc_activation_mask_eligible(type_byte) {
                self.mark_removed_town_npc_once(scene, npc_slot);
            }
            self.message = "I thank thee!".to_string();
        }
        self.advance_turn();
        MoveOutcome::LockTried
    }

    pub fn surface_object_chest_slot_at(
        &self,
        x: usize,
        y: usize,
    ) -> Option<(usize, ActiveObject)> {
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| {
                (self.object_occupies(object, x, y)
                    && Self::surface_object_chest_stat(object).is_some())
                .then_some((slot, object))
            })
    }

    pub fn surface_object_chest_stat(object: ActiveObject) -> Option<u8> {
        let is_furniture_chest = (TILE_FURNITURE_FIRST..=TILE_FURNITURE_LAST)
            .contains(&object.type_byte)
            || (TILE_FURNITURE_FIRST..=TILE_FURNITURE_LAST).contains(&object.tile);
        (is_furniture_chest && object.aux1 != 0).then_some(object.aux1)
    }

    pub fn jimmy_surface_object_chest_at(
        &mut self,
        x: usize,
        y: usize,
        member_index: usize,
    ) -> Option<MoveOutcome> {
        let (slot, object) = self.surface_object_chest_slot_at(x, y)?;
        let stat = Self::surface_object_chest_stat(object)?;
        let Some(threshold) =
            object_chest_jimmy_threshold(stat, self.party[member_index].climb_stat)
        else {
            self.advance_turn();
            self.message = "Key broke!".to_string();
            // audio.md §8.1 Jimmy key breaks: failure only — the break line,
            // then the 40-update action snap, then the key decrement.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
            return Some(MoveOutcome::LockTried);
        };
        let roll = self.surface_object_chest_jimmy_roll();
        if object_chest_jimmy_succeeds(threshold, roll) {
            if let Some(object) = self.active_objects.get_mut(slot) {
                object.aux1 &= 0x7f;
            }
            self.advance_turn();
            self.message = "Unlocked!".to_string();
        } else {
            self.advance_turn();
            self.message = "Key broke!".to_string();
            // audio.md §8.1 Jimmy key breaks: failure only — the break line,
            // then the 40-update action snap, then the key decrement.
            self.emit_sound_effect(SoundEffect::ActionSnap);
            self.keys = self.keys.saturating_sub(1);
        }
        Some(MoveOutcome::LockTried)
    }

    pub fn surface_object_chest_jimmy_roll(&mut self) -> u8 {
        self.random_range_u8(JIMMY_OBJECT_DIE_LOW, JIMMY_OBJECT_DIE_HIGH)
    }

    /// `containers.md` / `traps.md §4`: `member_index` is the acting member
    /// already chosen by [`Self::begin_surface_object_chest_interaction`]
    /// before this site tests the trap bit — the selection is not made
    /// here, and there is no slot-0 fallback.
    ///
    /// Two orderings below are load-bearing. The flag/class byte is copied
    /// to `stat` **first**, so the trap still fires on this open; the
    /// matched object record is then cleared outright — its kind, its
    /// position, and the byte carrying the trap flag and content class —
    /// so a later Open of the same square matches no container at all. The
    /// record lives inside the persisted save image, so the clear is
    /// durable: **a trapped surface or town container cannot spring a
    /// second time.** `traps.md` §4 published that as an open gap and has
    /// since withdrawn it; do not reintroduce the uncertainty.
    pub fn consume_surface_object_chest_at(
        &mut self,
        x: usize,
        y: usize,
        member_index: usize,
        verb: &str,
    ) -> Option<MoveOutcome> {
        let (slot, object) = self.surface_object_chest_slot_at(x, y)?;
        let stat = Self::surface_object_chest_stat(object)?;
        let chest_class = stat & 0x7f;
        let trapped = stat & 0x80 != 0;
        // `containers.md §4`: "The clear happens **after** the helper has
        // taken its own local copy of the container's flag/class byte and
        // **before** it tests that copy for the trap condition." The freed
        // slot is then available to the per-row staging below.
        self.clear_consumed_active_object_slot(slot);
        // `containers.md §4.1`/`traps.md §4`: "print `Trapped!` plus one
        // line feed - no leading feed, no member name, no trap-kind word -
        // and then run the shared trap resolver, which prints its own
        // word." Published 2026-10-12 (issue #292); this engine printed
        // neither notice and invented its own content sentence instead.
        if trapped {
            self.emit_message_line(CHEST_TRANSCRIPT_TRAPPED_LINE);
        }
        let trap_note = trapped.then(|| self.apply_shared_trap_effect_to_slot(member_index));
        // `traps.md §4`: "The surface/town site alone performs a
        // combat-scene cleanup after the resolver returns: if the chosen
        // member is then Dead, it (a) finds that member's live combatant
        // record and **sets a marker bit on it**, (b) **stamps a fixed
        // non-zero marker value into the leading bytes of the world-object
        // entry that record points at**, and (c) clears the active-character
        // hint when that hint named the member. The dungeon chest site has
        // no such step."
        //
        // §4's fourth difference pins the gate: the `O` Open dispatcher
        // "routes every other scene - combat-class scenes included - to the
        // surface/town handler", so this cleanup is reachable only while a
        // combat-class scene is live.
        if trap_note.is_some() && self.combat_active {
            self.apply_surface_container_trap_combat_cleanup(member_index);
        }
        let content_note = self.generate_surface_object_chest_content(x, y, chest_class);
        self.rewrite_surface_object_chest_cell(x, y);
        if matches!(self.area, Area::Town { .. }) {
            self.moral_standing = town_chest_open_standing(self.moral_standing);
        } else if matches!(self.area, Area::World { .. }) {
            self.cache_current_world_overlay();
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        // `commands.md §8.1`: a command "never prints tile ids, coordinates,
        // active-object slot numbers, terrain-class names", so the verb and
        // the cell go to diagnostics rather than the message window.
        //
        // The resolver already printed the player's line -
        // `traps.md §3`'s `ACID!`, `POISON!`, `BOMB!` or `GAS!` - through
        // `emit_message_line`, with `Trapped!` printed ahead of it above.
        // What `apply_shared_trap_effect_to_slot` *returns* is an engine
        // note naming the slot and the damage, which the dungeon chest
        // sibling in `finish_open_dungeon_chest` already routes to
        // `diagnostics` with the same reasoning.
        match trap_note {
            Some(trap) => self.push_diagnostic(format!(
                "{verb} object chest at ({x}, {y}); {trap}"
            )),
            None => self.push_diagnostic(format!("{verb} object chest at ({x}, {y}).")),
        }
        self.message = content_note;
        Some(MoveOutcome::ContainerOpened)
    }

    /// `traps.md §4` surface/town container combat-scene cleanup. See the
    /// three-step citation at the call site in
    /// [`Self::consume_surface_object_chest_at`]. Deliberately **not**
    /// reachable from `finish_open_dungeon_chest`: "The dungeon chest site
    /// has no such step."
    ///
    /// Two earlier readings are withdrawn by §4 and must not be
    /// reintroduced: the combatant record is "**not** removed, cleared, or
    /// freed - it stays in place with one additional bit set", and the
    /// world-object entry is "**not** blanked: a specific constant is
    /// written into it".
    ///
    /// That constant is closed in §4: "the value is **decimal thirty**,
    /// written as two separate byte stores of the same value into **both
    /// leading bytes** of the record, and it is a **corpse**" - and "a port
    /// should write the value **as a corpse-class object**, not as an opaque
    /// stamp and not as a tile", which is why this uses the combat
    /// party-corpse *object* id rather than routing through any terrain/tile
    /// classifier.
    pub fn apply_surface_container_trap_combat_cleanup(&mut self, member_index: usize) {
        if !self
            .party
            .get(member_index)
            .is_some_and(|member| member.hp == 0)
        {
            return;
        }
        if let Some(actor) = self.combat_actors.get_mut(member_index) {
            // (a) marker bit only; the record keeps its slot and contents.
            actor.mark_dead();
            let object_slot = actor.active_object_slot as usize;
            // (b) decimal thirty into both leading bytes of the linked
            // world-object entry, as a corpse-class object id.
            if let Some(object) = self.active_objects.get_mut(object_slot) {
                object.type_byte = COMBAT_PARTY_CORPSE_TILE;
                object.tile = COMBAT_PARTY_CORPSE_TILE;
            }
        }
        // (c) clear the active-character hint when it named this member.
        if self.active_player == Some(member_index) {
            self.active_player = None;
        }
    }

    pub fn rewrite_surface_object_chest_cell(&mut self, x: usize, y: usize) {
        match self.area {
            Area::Town { .. } => {
                if x < 32 && y < 32 {
                    self.grid[y * 32 + x] = LOCATION_MARKER_CLEANUP_TILE;
                }
            }
            Area::World { .. } => {
                if x < WORLD_SIDE && y < WORLD_SIDE {
                    self.grid[world_cell_index(x, y)] = LOCATION_MARKER_CLEANUP_TILE;
                    let _ = self.refresh_world_live_chunks_for_current_area();
                }
            }
            Area::Dungeon { .. } => {}
        }
    }

    /// `containers.md §4.1`'s surface/town chest transcript: one fixed
    /// line per successful grant naming only the family - never a count,
    /// colour, rune word or equipment name - prefixed with a `Found:`
    /// heading before the first line, or `Chest empty!` if nothing
    /// granted.
    ///
    /// `containers.md §4`: "Every successful row from either pool is
    /// committed the same way: the engine acquires a free active-object
    /// slot, writes the result family and the rolled quantity into it, and
    /// names the granted object." Nothing is credited here; the staged
    /// floor object is a later `G` Get's to transfer through the ordinary
    /// class dispatcher. A row whose slot cannot be acquired is dropped
    /// and prints nothing.
    ///
    /// Every draw comes from the shared gameplay PRNG in the published
    /// order (`§4.1` "Draw-order details"): an eligible primary row draws
    /// its `1..30` gate, a passing row draws its quantity (Gold and the
    /// Chest marker first take a discarded `1..90` / `1..10` draw), and
    /// the quantity draw precedes slot allocation. An equipment index
    /// whose threshold exceeds the class is skipped without its `1..30`
    /// draw.
    pub fn generate_surface_object_chest_content(
        &mut self,
        x: usize,
        y: usize,
        chest_class: u8,
    ) -> String {
        let mut parts: Vec<&'static str> = Vec::new();
        for (row, threshold) in CHEST_PRIMARY_POOL_THRESHOLDS.iter().copied().enumerate() {
            // "A row is eligible only when its threshold is less than or
            // equal to the chest class. Eligible rows then roll `1..30`".
            if threshold > chest_class {
                continue;
            }
            let roll = self.random_range_u8(1, CHEST_CONTENT_ROLL_DIE);
            if !chest_primary_pool_row_succeeds(chest_class, threshold, roll) {
                continue;
            }
            let (class_code, value, line) = match row {
                0 => (INVENTORY_ADD_CLASS_CODE_FOOD, self.random_range_u8(1, 2), CHEST_TRANSCRIPT_FOOD_LINE),
                1 => (INVENTORY_ADD_CLASS_CODE_TORCH, self.random_range_u8(1, 2), CHEST_TRANSCRIPT_TORCHES_LINE),
                2 => (INVENTORY_ADD_CLASS_CODE_GEM, self.random_range_u8(1, 2), CHEST_TRANSCRIPT_GEMS_LINE),
                3 => (INVENTORY_ADD_CLASS_CODE_KEY, self.random_range_u8(1, 2), CHEST_TRANSCRIPT_KEYS_LINE),
                4 => (INVENTORY_ADD_CLASS_CODE_SCROLL, self.random_range_u8(1, 8) - 1, CHEST_TRANSCRIPT_SCROLL_LINE),
                5 => (INVENTORY_ADD_CLASS_CODE_POTION, self.random_range_u8(1, 8) - 1, CHEST_TRANSCRIPT_POTION_LINE),
                6 => {
                    let _discarded = self.random_range_u8(1, CHEST_GOLD_DISCARDED_DRAW_MAX);
                    let upper = chest_class.saturating_mul(3);
                    (INVENTORY_ADD_CLASS_CODE_GOLD, self.random_range_u8(1, upper), CHEST_TRANSCRIPT_GOLD_LINE)
                }
                7 => {
                    // `§4` row 8, "Roll `1..chest_class` as the staged
                    // context value", staged as a container-class record
                    // (`§8` `0x01`), which a later Get answers with
                    // `Open_it_first!`.
                    let _discarded = self.random_range_u8(1, CHEST_MARKER_DISCARDED_DRAW_MAX);
                    (INVENTORY_ADD_CLASS_CODE_CONTAINER, self.random_range_u8(1, chest_class), CHEST_TRANSCRIPT_MARKER_LINE)
                }
                _ => continue,
            };
            if self.stage_surface_chest_grant(x, y, class_code, value) {
                parts.push(line);
            }
        }

        let attempts = chest_secondary_pool_attempts(chest_class);
        for _ in 0..attempts {
            let row = usize::from(self.random_range_u8(0, (CHEST_SECONDARY_POOL_ROW_COUNT - 1) as u8));
            let Some(threshold) = chest_secondary_pool_threshold(row) else {
                continue;
            };
            if threshold > chest_class {
                continue;
            }
            let roll = self.random_range_u8(1, CHEST_CONTENT_ROLL_DIE);
            if chest_primary_pool_row_succeeds(chest_class, threshold, roll)
                && self.stage_surface_chest_grant(
                    x,
                    y,
                    chest_secondary_pool_family_id(row),
                    row as u8,
                )
            {
                parts.push(chest_secondary_pool_transcript_line(row));
            }
        }

        if parts.is_empty() {
            CHEST_TRANSCRIPT_EMPTY_LINE.to_string()
        } else {
            format!("{CHEST_TRANSCRIPT_FOUND_HEADING}{}", parts.concat())
        }
    }

    /// `containers.md §4` per-row commit: acquire a free active-object
    /// slot with the shipped downward scan - index zero is never handed
    /// out, and a full table yields no slot - and write the family's
    /// class byte and the rolled value into it at the chest's coordinate.
    /// Returns `false` when the row is dropped.
    ///
    /// `containers.md §4.1` (u5-spec#298 Q18): "both tile bytes of the new
    /// active-object record hold the family id, so it renders as the
    /// actor-half tile at atlas index `0x100 + id`"; "The quantity or
    /// subtype rides in the record's fifth byte"; "The record's last byte is
    /// cleared outside combat and set to `0x20` when the scene byte is `0x80`
    /// or above."
    pub fn stage_surface_chest_grant(
        &mut self,
        x: usize,
        y: usize,
        class_code: u8,
        value: u8,
    ) -> bool {
        let z = self.current_floor().unwrap_or(0);
        self.allocate_highest_empty_active_object_slot(ActiveObject {
            type_byte: class_code,
            tile: class_code,
            x,
            y,
            z,
            phase: STEADY_PHASE,
            aux1: value,
            aux3: if self.combat_active {
                CHEST_GRANT_COMBAT_LAST_BYTE
            } else {
                0
            },
        })
        .is_some()
    }

    pub fn add_moral_standing(&mut self, amount: u8) -> u8 {
        let before = self.moral_standing;
        self.moral_standing = self
            .moral_standing
            .saturating_add(amount)
            .min(MORAL_STANDING_MAX);
        self.moral_standing - before
    }

    pub fn jimmy_dungeon_underfoot(
        &mut self,
        _game_dir: Option<&Path>,
        _scene: DungeonScene,
        level: u8,
        member_index: usize,
    ) -> io::Result<MoveOutcome> {
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        // `dungeon-mode.md §8.1`'s chest-command table: Jimmy's results are
        // five published literals, each with "a bare `\n` first, so the
        // result always starts on a new row". They had no reader - this
        // handler rendered the engine's own prose, and three of the five
        // were different words: `Unlocked!` for `Chest unlocked`,
        // `It's open!` for `Already open!`, and `No lock!` for `What?`.
        Ok(match tile >> 4 {
            0x4 => {
                if self.keys == 0 {
                    self.advance_turn();
                    self.message = DUNGEON_CHEST_JIMMY_NO_KEYS.to_string();
                    return Ok(MoveOutcome::Blocked);
                }
                if Self::is_plain_closed_dungeon_chest(tile) {
                    self.advance_turn();
                    self.message = DUNGEON_CHEST_JIMMY_KEY_BROKE.to_string();
                    // audio.md §8.1 Jimmy key breaks: failure only — the break line,
                    // then the 40-update action snap, then the key decrement.
                    self.emit_sound_effect(SoundEffect::ActionSnap);
                    self.keys = self.keys.saturating_sub(1);
                    return Ok(MoveOutcome::LockTried);
                }

                let dexterity = self
                    .party
                    .get(member_index)
                    .map(|member| member.climb_stat)
                    .unwrap_or_default();
                let threshold = Self::dungeon_chest_pick_threshold(level, dexterity);
                let roll = self.random_range_u8(JIMMY_OBJECT_DIE_LOW, JIMMY_OBJECT_DIE_HIGH);
                if !dungeon_chest_jimmy_succeeds(threshold, roll) {
                    self.advance_turn();
                    self.message = DUNGEON_CHEST_JIMMY_KEY_BROKE.to_string();
                    // audio.md §8.1 Jimmy key breaks: failure only — the break line,
                    // then the 40-update action snap, then the key decrement.
                    self.emit_sound_effect(SoundEffect::ActionSnap);
                    self.keys = self.keys.saturating_sub(1);
                    return Ok(MoveOutcome::LockTried);
                }

                // `§8`: the successful pick leaves a *closed* chest whose
                // lock/trap sub-type is cleared - "a successful dungeon
                // Jimmy clears those bits, and the same O-Open then prints
                // exactly what an untrapped chest prints". Rewriting to the
                // open class here skipped Open altogether.
                self.grid[idx] = dungeon_jimmy_chest_rewrite(tile);
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = DUNGEON_CHEST_JIMMY_UNLOCKED.to_string();
                MoveOutcome::LockTried
            }
            0x7 => {
                self.advance_turn();
                self.message = DUNGEON_CHEST_JIMMY_ALREADY_OPEN.to_string();
                MoveOutcome::LockTried
            }
            _ => {
                self.advance_turn();
                self.message = DUNGEON_CHEST_JIMMY_WHAT.to_string();
                MoveOutcome::Blocked
            }
        })
    }

    pub fn jimmy_lock_pick_succeeds(&mut self, member_index: usize) -> bool {
        let dexterity = self
            .party
            .get(member_index)
            .map(|member| member.climb_stat)
            .unwrap_or_default();
        let roll = self.jimmy_lock_pick_roll();
        jimmy_door_succeeds(dexterity, roll)
    }

    pub fn jimmy_lock_pick_roll(&mut self) -> u8 {
        self.random_range_u8(JIMMY_DOOR_DIE_LOW, JIMMY_DOOR_DIE_HIGH)
    }

    pub fn random_range_u8(&mut self, low: u8, high: u8) -> u8 {
        u5_prng_range_u16(&mut self.prng_state, u16::from(low), u16::from(high)) as u8
    }

    pub fn random_mod_u8(&mut self, modulus: u8) -> u8 {
        if modulus == 0 {
            0
        } else {
            self.random_range_u8(0, modulus - 1)
        }
    }

    pub fn visible_jimmy_unlock_tile(tile: u8) -> Option<u8> {
        jimmy_locked_door_rewrite(tile)
    }

    #[cfg(test)]
    pub fn get_dungeon_underfoot(&mut self, scene: DungeonScene, level: u8) -> MoveOutcome {
        self.get_dungeon_underfoot_with_contents(None, scene, level)
    }

    pub fn get_dungeon_underfoot_with_game_dir(
        &mut self,
        game_dir: Option<&Path>,
        scene: DungeonScene,
        level: u8,
    ) -> io::Result<MoveOutcome> {
        let chest_entries = game_dir
            .map(load_dungeon_chest_content_entries)
            .transpose()?
            .flatten();
        Ok(self.get_dungeon_underfoot_with_contents(chest_entries.as_deref(), scene, level))
    }

    pub fn get_dungeon_underfoot_with_contents(
        &mut self,
        chest_entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
    ) -> MoveOutcome {
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        // `commands.md §5.3`: "The dispatcher emits `Get-` only outside the
        // dungeon band; the dungeon handler opens with its own `Get\n`,
        // unconditionally, before it looks at anything." The dungeon arm of
        // the shared echo table is that `Get` row, so the handler contributes
        // only what follows it - the same split dungeon Push uses.
        self.message = String::new();
        match tile >> 4 {
            0x4 => {
                // `dungeon-mode.md §13`: the published dungeon-chest
                // Get refusal, not a paraphrase of it.
                self.message.push_str(DUNGEON_CHEST_GET_MUST_OPEN_FIRST);
                MoveOutcome::Blocked
            }
            0x7 => self.consume_dungeon_chest(
                chest_entries,
                scene,
                level,
                self.player.x,
                self.player.y,
                idx,
                tile,
                "Got",
            ),
            _ => {
                // `commands.md §5.8`: "Get on a non-chest prints
                // `Get\nNot here!\n`" - not the surface cascade's refusal.
                self.message.push_str(DUNGEON_CHEST_GET_NOT_HERE);
                MoveOutcome::Blocked
            }
        }
    }

    pub fn get_object_pickup_at(
        &mut self,
        game_dir: &Path,
        target: PlayTarget,
        floor: i8,
        x: usize,
        y: usize,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(entries) = load_object_pickup_entries(game_dir)? else {
            return Ok(None);
        };
        let hit = self
            .active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| {
                if !self.object_occupies(object, x, y) {
                    return None;
                }
                let entry = entries
                    .iter()
                    .copied()
                    .find(|entry| object_pickup_matches(*entry, target, floor, x, y, object))?;
                Some((slot, object.tile, entry))
            });
        let Some((slot, tile, entry)) = hit else {
            return Ok(None);
        };

        self.clear_consumed_active_object_slot(slot);
        self.apply_object_pickup(entry.kind, entry.amount);
        self.cache_current_world_overlay();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.diagnostics.push(format!(
            "got {} {} from active-object tile {tile} at ({x}, {y}) in {} floor {}",
            entry.amount,
            entry.kind.label(),
            target.key(),
            floor
        ));
        // `commands.md §5.8` "Successful Get, Search and eat results": the same
        // published rows the native class dispatcher prints, since a clean
        // sidecar row stands in for the record's own class byte.
        self.message = format!("\n{}", entry.kind.pickup_result_line(entry.amount));
        Ok(Some(MoveOutcome::Got))
    }

    /// `containers.md §5` trap/detail narrator for a container-class
    /// object slot. The assessing member comes from the shared
    /// acting-member selection (`traps.md §2.1`), exactly as the corpse
    /// branch's does: a prompt resumes through
    /// [`Self::step_active_surface_chest`], and nobody able aborts with
    /// `None!` before any roll.
    pub fn search_surface_object_trap_at(&mut self, x: usize, y: usize) -> Option<MoveOutcome> {
        self.surface_object_chest_slot_at(x, y)?;
        Some(match self.shared_acting_member_selection(true) {
            ActingMemberSelection::Selected(member_index) => self
                .finish_surface_object_trap_search_at(x, y, member_index)
                .unwrap_or(MoveOutcome::Blocked),
            ActingMemberSelection::Prompt => {
                self.start_surface_object_chest_prompt(x, y, SurfaceChestVerb::Search)
            }
            ActingMemberSelection::NoneAble => {
                self.message = DUNGEON_ACTING_MEMBER_NONE.to_string();
                MoveOutcome::Blocked
            }
        })
    }

    /// `containers.md §5`: "Search combines that difficulty with the acting
    /// member's **Intelligence** ... and a uniform `1..30` roll", drawn from
    /// the shared gameplay PRNG.
    pub fn finish_surface_object_trap_search_at(
        &mut self,
        x: usize,
        y: usize,
        member_index: usize,
    ) -> Option<MoveOutcome> {
        let (_slot, object) = self.surface_object_chest_slot_at(x, y)?;
        let stat = Self::surface_object_chest_stat(object)?;
        let member_intelligence = self
            .party_intelligence
            .get(member_index)
            .copied()
            .unwrap_or_default();
        let trappable = stat & 0x80 != 0;
        let difficulty = stat & 0x7f;
        let threshold =
            search_trap_detection_threshold(trappable, difficulty, member_intelligence);
        let roll = self.random_range_u8(1, JIMMY_OBJECT_DIE_HIGH);
        let detection_bit = u16::from(roll) >= threshold;
        let visibility = search_trap_visibility(trappable, difficulty, detection_bit);

        self.advance_turn();
        self.diagnostics.push(format!(
            "searched active-object tile {} at ({x}, {y}); {}",
            object.tile,
            surface_search_trap_visibility_label(visibility)
        ));
        // `commands.md §5.8`: "Container object at the cell | `[blank]` then
        // `Thou dost find` then one of `no trap!`, `a simple trap!`,
        // `a complex trap!`, `a trap!`". `RETRACTIONS.md` R477 adds that this
        // branch "contains no inventory grant at all", and that the assessed
        // state "is wrong in both directions when the Intelligence roll
        // fails" - the roll above is that assessment, not the truth.
        self.message = format!(
            "{SEARCH_PREAMBLE}{}",
            surface_search_trap_visibility_line(visibility)
        );
        Some(MoveOutcome::Searched)
    }

    pub fn get_native_object_pickup_at(&mut self, x: usize, y: usize) -> Option<MoveOutcome> {
        let (slot, object) = self
            .active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .find(|(_, object)| {
                // `RETRACTIONS.md` R476: "There is one byte. The record's
                // inventory class byte is both the acceptance test and the
                // grant dispatcher's selector". The separate visual filter
                // this used to apply first is withdrawn, so the handler takes
                // the record at the cell and lets its class decide - including
                // deciding to refuse.
                self.object_occupies(*object, x, y)
            })?;

        let Some(grant) = native_object_pickup_grant(object) else {
            // `commands.md §5.8`: "Two class values are guards rather than
            // grants. A container record prints `Open_it_first!\n` and changes
            // nothing at all: no inventory, no slot clear, no dirty marking, no
            // turn sentinel. A record whose class byte is zero reaches the
            // class dispatcher's default arm and prints the ordinary
            // `Nothing_to_get!\n`, also changing nothing."
            self.message = match inventory_add_class(object.type_byte) {
                InventoryAddClass::MustOpenFirst => GET_MUST_OPEN_FIRST_REFUSAL.to_string(),
                _ => GET_NOTHING_REFUSAL.to_string(),
            };
            return Some(MoveOutcome::Blocked);
        };

        self.clear_consumed_active_object_slot(slot);
        self.apply_object_pickup(grant.kind, grant.amount);
        self.cache_current_world_overlay();
        self.mark_visibility_dirty();
        self.advance_turn();
        // `commands.md §5.8` "Successful Get, Search and eat results",
        // published for `cleak/u5-spec#262`. The leading feed "is emitted
        // before the handler looks at the target cell, so it precedes every
        // line in this group, the refusal included".
        let _ = (x, y);
        self.message = format!("\n{}", grant.kind.pickup_result_line(grant.amount));
        Some(MoveOutcome::Got)
    }

    /// `stats-panel.md §2.2.1` row 21: "Any successful grant from Search,
    /// Get, chest opening or hidden-treasure collection ... one shared
    /// tail" is deferred; row 22 names the crop case (reached through
    /// [`Self::apply_get_tile_grant`]) the same way. This is that tail -
    /// every caller is one of those four families, and a refusal never
    /// reaches it at all.
    pub fn apply_object_pickup(&mut self, kind: ObjectPickupKind, amount: u8) {
        self.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Deferred;
        match kind {
            ObjectPickupKind::Food => {
                self.food = self
                    .food
                    .saturating_add(u16::from(amount))
                    .min(PARTY_FOOD_CAP)
            }
            ObjectPickupKind::Gold => {
                self.gold = self
                    .gold
                    .saturating_add(u16::from(amount))
                    .min(PARTY_GOLD_CAP)
            }
            ObjectPickupKind::Keys => {
                self.keys = self.keys.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP)
            }
            ObjectPickupKind::Gems => {
                self.gems = self.gems.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP)
            }
            ObjectPickupKind::Torches => {
                self.torches = self
                    .torches
                    .saturating_add(amount)
                    .min(PARTY_BYTE_STOCK_CAP)
            }
            ObjectPickupKind::SkullKeys => {
                let slot = &mut self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX];
                *slot = slot.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
            }
            ObjectPickupKind::Potion(index) => {
                if let Some(stock) = self.potion_stock.get_mut(index) {
                    *stock = stock.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
                }
            }
            ObjectPickupKind::Scroll(index) => {
                if let Some(stock) = self.scroll_stock.get_mut(index) {
                    *stock = stock.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
                }
            }
            ObjectPickupKind::Equipment(index) => {
                if let Some(stock) = self.equipment_stock.get_mut(index) {
                    let units = inventory_add_equipment_units(index).saturating_mul(amount);
                    *stock = stock.saturating_add(units).min(PARTY_BYTE_STOCK_CAP);
                }
            }
            ObjectPickupKind::Moonstone(index) => {
                if index < MOONSTONE_SLOT_COUNT {
                    self.moonstone_slots[index] = MoonstoneGateSlot::invalid();
                }
            }
            ObjectPickupKind::MagicCarpet => {
                let slot = &mut self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX];
                *slot = slot.saturating_add(amount).min(PARTY_BYTE_STOCK_CAP);
            }
            ObjectPickupKind::HmsCapePlans => {
                self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] = self.special_items
                    [SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX]
                    .max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::SandalwoodBox => {
                self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] =
                    self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::CrownOfLordBritish => {
                self.special_items[SPECIAL_ITEM_CROWN_LB_INDEX] =
                    self.special_items[SPECIAL_ITEM_CROWN_LB_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::SceptreOfLordBritish => {
                self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] =
                    self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::AmuletOfLordBritish => {
                self.special_items[SPECIAL_ITEM_AMULET_LB_INDEX] =
                    self.special_items[SPECIAL_ITEM_AMULET_LB_INDEX].max(SPECIAL_ITEM_OWNED_VALUE);
            }
            ObjectPickupKind::ShadowlordShard(index) => {
                if index < SHADOWLORD_COUNT {
                    self.special_items[SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX + index] =
                        SPECIAL_ITEM_OWNED_VALUE;
                }
            }
        }
    }

    pub fn apply_get_tile_grant(&mut self, grant: ObjectPickupGrant) {
        self.apply_object_pickup(grant.kind, grant.amount);
        if matches!(grant.kind, ObjectPickupKind::Food) {
            self.debit_crop_or_table_food_moral();
        }
    }

    pub fn debit_crop_or_table_food_moral(&mut self) -> u8 {
        let before = self.moral_standing;
        self.moral_standing = self
            .moral_standing
            .saturating_sub(KARMA_CROP_OR_TABLE_FOOD_DEBIT);
        before - self.moral_standing
    }

    pub fn get_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.get_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn get_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::World { plane } => self.get_world_direction(game_dir, plane, direction),
            Area::Town { scene, floor } => {
                self.get_town_direction(game_dir, scene, floor, direction)
            }
            Area::Dungeon { scene, level } => {
                self.get_dungeon_underfoot_with_game_dir(Some(game_dir), scene, level)
            }
        }
    }

    pub fn get_world_facing(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<MoveOutcome> {
        self.get_world_direction(game_dir, plane, self.player.facing)
    }

    pub fn get_world_direction(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if let Some(outcome) = self.get_moonstone_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_fixed_hidden_treasure_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_object_pickup_at(
            game_dir,
            PlayTarget::World(plane),
            plane.save_floor(),
            tx,
            ty,
        )? {
            return Ok(outcome);
        }
        // The surface object chest is its own handler, reached before the
        // class dispatcher: a chest record's class is the one the dispatcher
        // would refuse with `Open it first!`, and the tile-class chest this
        // finds is not that record.
        if self.surface_object_chest_slot_at(tx, ty).is_some() {
            return Ok(self.begin_surface_object_chest_interaction(tx, ty, SurfaceChestVerb::Get));
        }
        if let Some(outcome) = self.get_native_object_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if self.world_object_at(tx, ty).is_some() {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let idx = world_cell_index(tx, ty);
        let tile = self.grid[idx];
        if let Some(outcome) = self.get_tile_consumable(idx, tile) {
            self.rebuild_world_live_chunks_from_grid(plane)?;
            return Ok(outcome);
        }
        let Some(entries) = load_world_get_tile_entries(game_dir)? else {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(entry) = entries
            .iter()
            .find(|entry| world_get_tile_matches(**entry, plane, tx, ty, tile))
            .copied()
        else {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.grid[idx] = entry.replacement_tile;
        self.rebuild_world_live_chunks_from_grid(plane)?;
        if let Some(grant) = entry.grant {
            self.apply_get_tile_grant(grant);
        }
        self.mark_visibility_dirty();
        self.advance_turn();
        self.diagnostics.push(tile_get_message(
            format!("Got world tile {tile} at ({tx}, {ty}) on {}", plane.key()),
            entry.replacement_tile,
            entry.grant,
        ));
        self.message = published_get_tile_line(tile)
            .unwrap_or_default()
            .to_string();
        self.apply_borrowed_lit_fixture_light(tile);
        // audio.md §8.1 borrowed fixed object: after the live tile is
        // rewritten and the borrowing line is printed, play the 40-update
        // action snap. A Food grant is the crop branch — it takes the
        // karma.md §4 crop/table-food debit, and audio.md §8.1 forbids
        // reusing the borrowing sound for crop pickup.
        if !matches!(
            entry.grant.map(|grant| grant.kind),
            Some(ObjectPickupKind::Food)
        ) {
            self.emit_sound_effect(SoundEffect::ActionSnap);
        }
        Ok(MoveOutcome::Got)
    }

    /// `containers.md §7` tile consumables. Both are tile-class rules that
    /// apply at any coordinate, from any approach direction, ahead of the
    /// QA-only per-coordinate Get-tile sidecar:
    ///
    /// - "Crop cells `0x2D` ... The cell changes to the plowed-field tile
    ///   `0x2C`, the line `Crops_picked!\n` is printed, one unit is credited
    ///   to the shared party food word with the 9999 cap, the shared
    ///   moral-standing selector is debited when nonzero, the party-dirty bit
    ///   is set, and the turn sentinel is set."
    /// - "The two **wall-torch** tiles `0xB0` and `0xB1`, and no other cell
    ///   ... The cell is rewritten to the plain floor tile `0x44` ..., the
    ///   line `Borrowed!\n` is printed, the theft sound plays, and the
    ///   party's torch counter is set to 100 counter units".
    pub fn get_tile_consumable(&mut self, idx: usize, tile: u8) -> Option<MoveOutcome> {
        match tile {
            GET_TILE_CROPS_SOURCE => {
                self.grid[idx] = GET_TILE_CROPS_REPLACEMENT;
                self.apply_get_tile_grant(ObjectPickupGrant {
                    kind: ObjectPickupKind::Food,
                    amount: 1,
                });
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = GET_TILE_CROPS_PICKED_LINE.to_string();
                Some(MoveOutcome::Got)
            }
            GET_TILE_WALL_TORCH_SOURCE_FIRST | GET_TILE_WALL_TORCH_SOURCE_SECOND => {
                self.grid[idx] = GET_TILE_WALL_TORCH_REPLACEMENT;
                self.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Deferred;
                self.mark_visibility_dirty();
                self.advance_turn();
                self.message = GET_TILE_BORROWED_LINE.to_string();
                self.apply_borrowed_lit_fixture_light(tile);
                // audio.md §8.1 borrowed fixed object: the 40-update action
                // snap after the line.
                self.emit_sound_effect(SoundEffect::ActionSnap);
                Some(MoveOutcome::Got)
            }
            _ => None,
        }
    }

    /// `lighting.md §8`: "The G-Get "borrow" branch, which lifts a lit
    /// fixture out of a town or castle cell, sets the torch counter to 100
    /// counter units and consumes no carried torch". `containers.md §7`
    /// adds the rest of the branch's contract: "the party's torch counter
    /// is set to 100 counter units - borrowing a lit fixture is a light
    /// source, not an inventory item, and it consumes no carried torch",
    /// and "The traced branch does not debit the shared moral-standing
    /// selector". Accordingly this touches neither `self.torches` nor
    /// `self.moral_standing`.
    ///
    /// `lighting.md §8` also fixes the writer census: with I-Ignite and the
    /// Blackthorn restoration these are "the only three torch-counter
    /// writers besides decay", so the value comes from the shared
    /// [`BORROWED_FIXTURE_TORCH_DURATION`] anchor rather than a local
    /// literal.
    ///
    /// `containers.md §7` (corrected 2026-09-12, `RETRACTIONS.md` R478):
    /// "The two **wall-torch** tiles `0xB0` and `0xB1`, and no other cell,
    /// can be 'borrowed'." The earlier keying on the whole local-light
    /// source set is withdrawn; any other source tile leaves both counters
    /// alone.
    pub fn apply_borrowed_lit_fixture_light(&mut self, source_tile: u8) {
        if !matches!(
            source_tile,
            GET_TILE_WALL_TORCH_SOURCE_FIRST | GET_TILE_WALL_TORCH_SOURCE_SECOND
        ) {
            return;
        }
        self.torch_counter = BORROWED_FIXTURE_TORCH_DURATION;
        // `lighting.md §4`: a non-zero torch counter floors the ambient
        // value, so the recompute has to run on the same turn the counter
        // is written - exactly as the I-Ignite writers do.
        self.recompute_daylight();
        // Unpublished (`cleak/u5-spec#262`). The comment here used to admit
        // "Engine-authored narration", which is the thing to remove rather
        // than to label: the observable effect belongs in `diagnostics`.
        self.diagnostics.push(format!(
            "borrowed a lit fixture; light counter is {BORROWED_FIXTURE_TORCH_DURATION}"
        ));
    }

    pub fn get_town_facing(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
    ) -> io::Result<MoveOutcome> {
        self.get_town_direction(game_dir, scene, floor, self.player.facing)
    }

    pub fn get_town_direction(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        direction: Direction,
    ) -> io::Result<MoveOutcome> {
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        if let Some(outcome) = self.get_moonstone_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_fixed_hidden_treasure_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if let Some(outcome) =
            self.get_object_pickup_at(game_dir, PlayTarget::Town(scene), floor, tx, ty)?
        {
            return Ok(outcome);
        }
        if self.surface_object_chest_slot_at(tx, ty).is_some() {
            return Ok(self.begin_surface_object_chest_interaction(tx, ty, SurfaceChestVerb::Get));
        }
        if let Some(outcome) = self.get_native_object_pickup_at(tx, ty) {
            return Ok(outcome);
        }
        if self.blocking_object_at(tx, ty).is_some() {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        if let Some(outcome) = self.get_town_table_food(idx, tx, ty, tile, dx, dy) {
            return Ok(outcome);
        }
        if let Some(outcome) = self.get_tile_consumable(idx, tile) {
            return Ok(outcome);
        }
        let Some(entries) = load_town_get_tile_entries(game_dir)? else {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(entry) = entries
            .iter()
            .find(|entry| town_get_tile_matches(**entry, scene, floor, tx, ty, tile))
            .copied()
        else {
            self.message = GET_NOTHING_REFUSAL.to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.grid[idx] = entry.replacement_tile;
        if let Some(grant) = entry.grant {
            self.apply_get_tile_grant(grant);
        }
        if self
            .door_tracker
            .is_some_and(|tracker| tracker.x == tx && tracker.y == ty)
        {
            self.door_tracker = None;
            self.door_tracker_closed = false;
        }
        self.forget_open_town_door(scene, floor, tx, ty);
        self.forget_revealed_town_secret_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.diagnostics.push(tile_get_message(
            format!("Got tile {tile} at ({tx}, {ty})"),
            entry.replacement_tile,
            entry.grant,
        ));
        self.message = published_get_tile_line(tile)
            .unwrap_or_default()
            .to_string();
        self.apply_borrowed_lit_fixture_light(tile);
        // audio.md §8.1 borrowed fixed object: after the live tile is
        // rewritten and the borrowing line is printed, play the 40-update
        // action snap. A Food grant is the crop branch — it takes the
        // karma.md §4 crop/table-food debit, and audio.md §8.1 forbids
        // reusing the borrowing sound for crop pickup.
        if !matches!(
            entry.grant.map(|grant| grant.kind),
            Some(ObjectPickupKind::Food)
        ) {
            self.emit_sound_effect(SoundEffect::ActionSnap);
        }
        Ok(MoveOutcome::Got)
    }

    pub fn get_town_table_food(
        &mut self,
        idx: usize,
        x: usize,
        y: usize,
        tile: u8,
        dx: isize,
        dy: isize,
    ) -> Option<MoveOutcome> {
        // `commands.md §5.8` "Eating and borrowing are Get cases, not
        // movement cases", published for `cleak/u5-spec#262`:
        //
        // | Laden table `0x9A`, reached one step south | `Mmmmm...!` | `0x95` |
        // | Laden table `0x9B`, reached one step north | `Mmmmm...!` | `0x95` |
        // | Laden table `0x9C`, reached one step north or south | `Mmmmm...!` | `0x9A` or `0x9B`, keeping the food the party did not take |
        // | Laden table `0x9A`/`0x9B` from any other direction, or `0x9C` from east or west | `Can't reach plate!` | unchanged |
        let replacement = match (tile, dx, dy) {
            (0x9a, 0, 1) => 0x95,
            (0x9b, 0, -1) => 0x95,
            (0x9c, 0, -1) => 0x9a,
            (0x9c, 0, 1) => 0x9b,
            (0x9a | 0x9b | 0x9c, _, _) => {
                self.message = GET_TABLE_CANT_REACH_PLATE_LINE.to_string();
                return Some(MoveOutcome::Blocked);
            }
            _ => return None,
        };

        self.grid[idx] = replacement;
        self.food = self.food.saturating_add(1).min(PARTY_FOOD_CAP);
        self.debit_crop_or_table_food_moral();
        self.mark_visibility_dirty();
        // "The three eat arms and the crop arm share a grant tail - one food
        // with the 9999 cap, the party-dirty bit, and the moral-standing debit
        // when that byte is non-zero - but **not** the turn sentinel: only the
        // crop arm and the `0x9A` arm set it."
        if tile == 0x9a {
            self.advance_turn();
        }
        self.diagnostics.push(format!(
            "ate food from table tile 0x{tile:02X} at ({x}, {y}); replaced with tile \
             0x{replacement:02X}; added 1 food"
        ));
        self.message = GET_TABLE_EATEN_LINE.to_string();
        Some(MoveOutcome::Got)
    }

    pub fn search_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.search_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn search_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let entries = load_secret_door_entries(game_dir)?.unwrap_or_default();
        let chest_entries = load_dungeon_chest_content_entries(game_dir)?;
        Ok(self.search_direction_secret(direction, &entries, chest_entries.as_deref()))
    }

    pub fn search_facing_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        chest_entries: Option<&[DungeonChestContentEntry]>,
    ) -> MoveOutcome {
        self.search_direction_secret(self.player.facing, entries, chest_entries)
    }

    pub fn search_direction_secret(
        &mut self,
        direction: Direction,
        entries: &[SecretDoorEntry],
        chest_entries: Option<&[DungeonChestContentEntry]>,
    ) -> MoveOutcome {
        match self.area {
            Area::Town { scene, floor } => {
                self.search_town_secret_direction(entries, scene, floor, direction)
            }
            Area::Dungeon { scene, level } => {
                self.search_dungeon_secret(entries, chest_entries, scene, level)
            }
            Area::World { plane } => self.search_world_moonstone_direction(plane, direction),
        }
    }

    pub fn search_world_moonstone(&mut self, plane: WorldPlane) -> MoveOutcome {
        self.search_world_moonstone_direction(plane, self.player.facing)
    }

    pub fn search_world_moonstone_direction(
        &mut self,
        plane: WorldPlane,
        direction: Direction,
    ) -> MoveOutcome {
        let (dx, dy) = direction.delta();
        let tx = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
        let ty = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
        if let Some(outcome) = self.search_active_object_treasure_marker_at(tx, ty) {
            return outcome;
        }
        if let Some(outcome) = self.search_surface_object_trap_at(tx, ty) {
            return outcome;
        }
        let target_tile = self.grid.get(ty * WORLD_SIDE + tx).copied();
        let skip_moonstone_scan = target_tile == Some(0xdc);
        if !skip_moonstone_scan {
            if let Some(outcome) = self.search_moonstone_pickup_at(tx, ty, |slot| {
                moonstone_slot_matches_world(slot, plane, tx, ty)
            }) {
                return outcome;
            }
        }
        if let Some(outcome) = self.search_rare_reagent_at(plane, tx, ty) {
            return outcome;
        }
        if let Some(outcome) = self.search_fixed_hidden_treasure_at(
            HiddenTreasureTarget::World(plane),
            plane.save_floor(),
            tx,
            ty,
        ) {
            return outcome;
        }
        if skip_moonstone_scan {
            self.diagnostics
                .push("searched a generic find marker; no Moonstone scan ran".to_string());
            self.message = SEARCH_NOTHING_FOUND.to_string();
            return MoveOutcome::Blocked;
        }
        // `commands.md §5.8` (u5-spec#298 Q4): the furniture prefix "applies
        // to overworld (both planes)" Search as well.
        self.message = search_nothing_found_for_tile(target_tile.unwrap_or_default());
        MoveOutcome::Blocked
    }

    pub fn search_town_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        scene: Scene,
        floor: i8,
    ) -> MoveOutcome {
        self.search_town_secret_direction(entries, scene, floor, self.player.facing)
    }

    pub fn search_town_secret_direction(
        &mut self,
        entries: &[SecretDoorEntry],
        scene: Scene,
        floor: i8,
        direction: Direction,
    ) -> MoveOutcome {
        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.message = SEARCH_NOTHING_FOUND.to_string();
            return MoveOutcome::Blocked;
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        if let Some(outcome) = self.search_active_object_treasure_marker_at(tx, ty) {
            return outcome;
        }
        if let Some(outcome) = self.search_surface_object_trap_at(tx, ty) {
            return outcome;
        }
        // `doors-and-z-transitions.md §8` with `RETRACTIONS.md` R412: "In
        // towns and dwellings, the authored hidden-door terrain is tile `0x4E`
        // in the location map itself. The normal direction and acting-member
        // selection lead to a direct terrain test; no per-map object flag is
        // required." The engine required a sidecar coordinate row - the
        // withdrawn object-table model - so no shipped hidden door in any town
        // could be found at all.
        if tile == TOWN_HIDDEN_DOOR_TILE {
            // "changes the target tile to the ordinary **locked** door `0xB9` on
            // ground/above-ground floors (floor byte below `128`), or `0xB8`
            // on below-ground floors (floor byte at least `128`)". This
            // engine's floor is the signed form of that byte.
            let reveal_tile = if floor >= 0 {
                TOWN_HIDDEN_DOOR_REVEAL_ABOVE_GROUND
            } else {
                TOWN_HIDDEN_DOOR_REVEAL_BELOW_GROUND
            };
            self.grid[idx] = reveal_tile;
            self.forget_open_town_door(scene, floor, tx, ty);
            self.record_revealed_town_secret_door(scene, floor, tx, ty);
            self.mark_visibility_dirty();
            self.advance_turn();
            // "On reveal, Search prints `\nThou dost find\na hidden door!\n`"
            // - which also answers `cleak/u5-spec#236`, filed when this arm
            // printed nothing because the line was unpublished.
            self.emit_message_line(TOWN_SEARCH_HIDDEN_DOOR_LINE);
            return MoveOutcome::Searched;
        }
        let reveal_tile = entries.iter().find_map(|entry| match *entry {
            SecretDoorEntry::Town {
                scene: entry_scene,
                floor: entry_floor,
                x,
                y,
                reveal_tile,
                expected_tile,
            } if entry_scene == scene
                && entry_floor == floor
                && x == tx
                && y == ty
                && expected_tile.map_or(true, |expected| expected == tile) =>
            {
                Some(reveal_tile)
            }
            _ => None,
        });
        let Some(reveal_tile) = reveal_tile else {
            if tile == 0xdc {
                if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                    HiddenTreasureTarget::Town(scene.byte),
                    floor,
                    tx,
                    ty,
                ) {
                    return outcome;
                }
                // `commands.md §5.8`: a Search that finds nothing prints the
                // treasure scan's own miss, the lower-case `nothing of note.`,
                // under whichever preamble the cell's prefix selects. Skipping
                // the Moonstone scan is engine detail, not a row.
                self.diagnostics
                    .push("searched a generic find marker; no Moonstone scan ran".to_string());
                self.message = search_nothing_found_for_tile(tile);
                return MoveOutcome::Blocked;
            }
            let miss_message = search_nothing_found_for_tile(tile);
            if let Some(outcome) = self.search_moonstone_pickup_at(tx, ty, |slot| {
                moonstone_slot_matches_town(slot, scene, floor, tx, ty)
            }) {
                return outcome;
            }
            if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                HiddenTreasureTarget::Town(scene.byte),
                floor,
                tx,
                ty,
            ) {
                return outcome;
            }
            self.message = miss_message;
            return MoveOutcome::Blocked;
        };
        if !(24..=63).contains(&tile) {
            if tile == 0xdc {
                if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                    HiddenTreasureTarget::Town(scene.byte),
                    floor,
                    tx,
                    ty,
                ) {
                    return outcome;
                }
                // `commands.md §5.8`: a Search that finds nothing prints the
                // treasure scan's own miss, the lower-case `nothing of note.`,
                // under whichever preamble the cell's prefix selects. Skipping
                // the Moonstone scan is engine detail, not a row.
                self.diagnostics
                    .push("searched a generic find marker; no Moonstone scan ran".to_string());
                self.message = search_nothing_found_for_tile(tile);
                return MoveOutcome::Blocked;
            }
            let miss_message = search_nothing_found_for_tile(tile);
            if let Some(outcome) = self.search_moonstone_pickup_at(tx, ty, |slot| {
                moonstone_slot_matches_town(slot, scene, floor, tx, ty)
            }) {
                return outcome;
            }
            if let Some(outcome) = self.search_fixed_hidden_treasure_at(
                HiddenTreasureTarget::Town(scene.byte),
                floor,
                tx,
                ty,
            ) {
                return outcome;
            }
            self.message = miss_message;
            return MoveOutcome::Blocked;
        }

        self.grid[idx] = reveal_tile;
        self.forget_open_town_door(scene, floor, tx, ty);
        self.record_revealed_town_secret_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn();
        // Same rule as the Sceptre above: coordinates never reach the message
        // window (`commands.md §8.1`). What a town Search *does* print when it
        // uncovers a secret door is unpublished - `commands.md` §5 gives only
        // the `Thou dost find` / `nothing of note.` failure shape - so the
        // player sees nothing here rather than an invented sentence
        // (cleak/u5-spec#236, which also asks which object-table flag marks
        // one; until that is answered this arm is unreachable in shipped
        // content anyway).
        self.push_diagnostic(format!("Revealed secret door at ({tx}, {ty})."));
        self.message.clear();
        MoveOutcome::Searched
    }

    /// `commands.md §5.8` (u5-spec#298 Q4): the Search preamble the searched
    /// cell's live tile selects. "The classification reads only the searched
    /// cell's live tile and never tests the scene", so the overworld, the
    /// town family and the combat arena all route through it; the dungeon
    /// "is diverted before the prefix choice to its own routine".
    pub fn search_preamble_at(&self, x: usize, y: usize) -> String {
        let tile = if self.combat_active {
            self.combat_terrain.get(y).and_then(|row| row.get(x)).copied()
        } else {
            match self.area {
                Area::World { .. } => self.grid.get(y * WORLD_SIDE + x).copied(),
                Area::Town { .. } => self.grid.get(y * 32 + x).copied(),
                Area::Dungeon { .. } => None,
            }
        };
        tile.map_or_else(|| SEARCH_PREAMBLE.to_string(), search_preamble_for_tile)
    }

    pub fn search_moonstone_at<F>(
        &mut self,
        x: usize,
        y: usize,
        matches_slot: F,
        miss_message: &str,
    ) -> MoveOutcome
    where
        F: Fn(MoonstoneGateSlot) -> bool,
    {
        if let Some(outcome) = self.search_moonstone_pickup_at(x, y, matches_slot) {
            return outcome;
        }

        self.message = miss_message.to_string();
        MoveOutcome::Blocked
    }

    pub fn search_moonstone_pickup_at<F>(
        &mut self,
        x: usize,
        y: usize,
        matches_slot: F,
    ) -> Option<MoveOutcome>
    where
        F: Fn(MoonstoneGateSlot) -> bool,
    {
        let Some(slot_index) = self
            .moonstone_slots
            .iter()
            .copied()
            .enumerate()
            .rev()
            .find_map(|(slot_index, slot)| {
                (slot.is_valid() && matches_slot(slot)).then_some(slot_index)
            })
        else {
            return None;
        };

        if self.moonstone_pickup_exists(slot_index) {
            // `commands.md §5.8`: "searching the same cell again before
            // collecting prints `nothing_of_note.\n`, because the duplicate
            // guard sees the staged object and creates no second one."
            self.diagnostics.push(format!(
                "Moonstone phase {} is already surfaced as a strange rock.",
                slot_index + 1
            ));
            self.message = format!(
                "{}{SEARCH_NOTHING_OF_NOTE_ROW}",
                self.search_preamble_at(x, y)
            );
            return Some(MoveOutcome::Blocked);
        }

        let Some(z) = self.current_floor() else {
            return None;
        };
        let pickup = ActiveObject::moonstone_pickup(slot_index, x, y, z);
        if self.allocate_active_object_slot(pickup).is_none() {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        }

        self.mark_visibility_dirty();
        self.advance_turn();
        // `commands.md §5.8`: "Buried Moonstone | `[blank]` then
        // `Thou dost find` then `a strange rock!`, staging the stone on the
        // cell". "Search alone grants nothing - it only stages the rock".
        self.diagnostics.push(format!(
            "Found a strange rock for Moonstone phase {}.",
            slot_index + 1
        ));
        self.message = format!("{}{SEARCH_STRANGE_ROCK_LINE}", self.search_preamble_at(x, y));
        Some(MoveOutcome::Searched)
    }

    pub fn search_rare_reagent_at(
        &mut self,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        if plane != WorldPlane::Britannia || self.clock.hour != 0 {
            return None;
        }
        let point = RARE_REAGENT_HARVEST_POINTS
            .iter()
            .find(|point| point.x == x && point.y == y)?;
        if self.rare_reagent_harvest_days[point.index] == self.clock.day {
            return None;
        }

        let amount = self.rare_reagent_harvest_amount(point.index);
        self.reagents[point.reagent_index] = self.reagents[point.reagent_index]
            .saturating_add(amount)
            .min(99);
        self.rare_reagent_harvest_days[point.index] = self.clock.day;
        self.advance_turn();
        // `commands.md §5.8`: "Rare reagent at the harvest hour | `[blank]`
        // then `Thou dost find` then the rolled count, then `_sprigs_of` then
        // `mandrake root!` or `nightshade!`".
        self.diagnostics
            .push(format!("Found {amount} sprigs of {}.", point.label));
        self.message = format!(
            "{}{amount}{SEARCH_REAGENT_SPRIGS_FRAGMENT}{}",
            self.search_preamble_at(x, y),
            point.line
        );
        Some(MoveOutcome::Searched)
    }

    pub fn rare_reagent_harvest_amount(&mut self, _point_index: usize) -> u8 {
        self.random_range_u8(
            RARE_REAGENT_HARVEST_QUANTITY_MIN,
            RARE_REAGENT_HARVEST_QUANTITY_MAX,
        )
    }

    pub fn search_fixed_hidden_treasure_at(
        &mut self,
        target: HiddenTreasureTarget,
        floor: i8,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        let mut matching_entries = FIXED_HIDDEN_TREASURES.iter().copied().filter(|entry| {
            entry.target == target && entry.floor == floor && entry.x == x && entry.y == y
        });
        let Some(entry) = matching_entries.find(|entry| {
            if self.fixed_hidden_treasure_pickup_exists(entry.record) {
                true
            } else {
                self.fixed_hidden_treasure_rule_allows(*entry, x, y)
            }
        }) else {
            return None;
        };
        if self.fixed_hidden_treasure_pickup_exists(entry.record) {
            // A second Search of a cell whose pickup is already on the floor.
            // `commands.md §5.8` gives the shape for the Moonstone's identical
            // duplicate guard - "searching the same cell again before
            // collecting prints `nothing_of_note.\n`, because the duplicate
            // guard sees the staged object and creates no second one" - and
            // the treasure scan's own miss is that same lower-case row.
            self.push_diagnostic(format!(
                "{} is already surfaced here.",
                entry.pickup.label()
            ));
            self.message = format!(
                "{}{SEARCH_NOTHING_OF_NOTE_ROW}",
                self.search_preamble_at(x, y)
            );
            return Some(MoveOutcome::Blocked);
        }
        let pickup = ActiveObject::fixed_hidden_treasure_pickup(entry.record, x, y, floor);
        if self.allocate_active_object_slot(pickup).is_none() {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        }
        self.mark_fixed_hidden_treasure_found(entry);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.diagnostics
            .push(format!("Found {}.", entry.pickup.label()));
        // `commands.md §5.8`: "Fixed hidden treasure | `[blank]` then
        // `Thou dost find` then the staged object's name from the found-object
        // vocabulary in `systems/hidden-treasures.md`".
        self.message = format!("{}{}", self.search_preamble_at(x, y), entry.pickup.narrated_row());
        Some(MoveOutcome::Searched)
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_len() -> usize {
        FIXED_HIDDEN_TREASURES.len()
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_records_are_sequential() -> bool {
        FIXED_HIDDEN_TREASURES
            .iter()
            .enumerate()
            .all(|(record, entry)| entry.record == record)
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_fingerprint() -> u64 {
        const FNV_OFFSET: u64 = 0xcbf29ce484222325;
        const FNV_PRIME: u64 = 0x100000001b3;

        let mut hash = FNV_OFFSET;
        for entry in FIXED_HIDDEN_TREASURES {
            for value in [
                entry.record as u64,
                fixed_hidden_treasure_target_code(entry.target),
                fixed_hidden_treasure_floor_code(entry.floor),
                entry.x as u64,
                entry.y as u64,
                fixed_hidden_treasure_pickup_code(entry.pickup),
                entry.state as u64,
                fixed_hidden_treasure_rule_code(entry.rule),
            ] {
                hash ^= value;
                hash = hash.wrapping_mul(FNV_PRIME);
            }
        }
        hash
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_pickup_counts() -> [usize; HIDDEN_TREASURE_PICKUP_CLASS_COUNT]
    {
        let mut counts = [0usize; HIDDEN_TREASURE_PICKUP_CLASS_COUNT];
        for entry in FIXED_HIDDEN_TREASURES {
            counts[fixed_hidden_treasure_pickup_code(entry.pickup) as usize] += 1;
        }
        counts
    }

    #[cfg(test)]
    pub fn fixed_hidden_treasure_table_rule_counts() -> [usize; HIDDEN_TREASURE_RULE_COUNT] {
        let mut counts = [0usize; HIDDEN_TREASURE_RULE_COUNT];
        for entry in FIXED_HIDDEN_TREASURES {
            counts[fixed_hidden_treasure_rule_code(entry.rule) as usize] += 1;
        }
        counts
    }

    pub fn fixed_hidden_treasure_rule_allows(
        &self,
        entry: FixedHiddenTreasureEntry,
        x: usize,
        y: usize,
    ) -> bool {
        match entry.rule {
            HiddenTreasureRule::OneShot => !self.fixed_hidden_treasure_found(entry.record),
            // `hidden-treasures.md §2` record 13: the only gate is the Keys
            // counter plus the NPC test; no found-bitmap bit is read or set,
            // so "spending every key opens it again".
            HiddenTreasureRule::KeyNpcGated => {
                self.keys == 0 && !self.fixed_hidden_treasure_target_has_npc(x, y)
            }
            HiddenTreasureRule::Daily => self.fixed_hidden_treasure_daily_day != self.clock.day,
            // `hidden-treasures.md §2` record 15: the gate field is "the
            // equipment-inventory counter for the item it grants" — record
            // 15 grants the Glass Sword, equipment item id 39. "Stages only
            // when that counter is zero and the searched tile is not occupied
            // by an NPC. The skip condition is therefore \"counter non-zero
            // **or** an NPC is present\"." The record has no persistence field
            // of its own: "an implementation must alias the existing field
            // rather than allocate a private cookie", and "An engine that
            // models the gate as a separate flag the scan never sets will
            // hand out an unlimited supply of Glass Swords."
            HiddenTreasureRule::SingleUseNpcGated => {
                self.equipment_stock
                    .get(entry.state as usize)
                    .is_some_and(|carried| *carried == 0)
                    && !self.fixed_hidden_treasure_target_has_npc(x, y)
            }
        }
    }

    pub fn mark_fixed_hidden_treasure_found(&mut self, entry: FixedHiddenTreasureEntry) {
        match entry.rule {
            HiddenTreasureRule::Daily => self.fixed_hidden_treasure_daily_day = self.clock.day,
            // `hidden-treasures.md §2` record 15: "The scan never writes the
            // counter; the ordinary inventory grant for the item does, and
            // that is exactly what makes the record single-use." The aliased
            // field is `equipment_stock[entry.state]`, which the Get
            // inventory transfer increments.
            HiddenTreasureRule::SingleUseNpcGated => {}
            // `hidden-treasures.md §2`: record 13 sets no found-bitmap bit.
            HiddenTreasureRule::KeyNpcGated => {}
            HiddenTreasureRule::OneShot => self.set_fixed_hidden_treasure_found(entry.record),
        }
    }

    pub fn fixed_hidden_treasure_found(&self, record: usize) -> bool {
        let byte = record / 8;
        let bit = record % 8;
        self.fixed_hidden_treasure_found
            .get(byte)
            .is_some_and(|value| value & (1 << bit) != 0)
    }

    pub fn set_fixed_hidden_treasure_found(&mut self, record: usize) {
        let byte = record / 8;
        let bit = record % 8;
        if let Some(value) = self.fixed_hidden_treasure_found.get_mut(byte) {
            *value |= 1 << bit;
        }
    }

    pub fn fixed_hidden_treasure_target_has_npc(&self, x: usize, y: usize) -> bool {
        let Some(floor) = self.current_floor() else {
            return false;
        };
        self.npcs
            .iter()
            .any(|npc| npc.x == x && npc.y == y && npc.z == floor as u8)
    }

    pub fn fixed_hidden_treasure_pickup_exists(&self, record: usize) -> bool {
        self.active_objects
            .iter()
            .any(|object| object.fixed_hidden_treasure_record() == Some(record))
    }

    pub fn fixed_hidden_treasure_pickup_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(object_slot, object)| {
                if self.object_occupies(*object, x, y) {
                    object
                        .fixed_hidden_treasure_record()
                        .map(|record| (object_slot, record))
                } else {
                    None
                }
            })
    }

    pub fn active_object_treasure_marker_at(&self, x: usize, y: usize) -> Option<(usize, usize)> {
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .rev()
            .find_map(|(object_slot, object)| {
                if !self.object_occupies(object, x, y)
                    || object.type_byte != FIXED_HIDDEN_TREASURE_OBJECT_TILE
                {
                    return None;
                }
                object
                    .fixed_hidden_treasure_record()
                    .or_else(|| {
                        // Native fixed-treasure class records carry their
                        // actual pickup visual in byte 1 and the record in
                        // aux1. A live moldy corpse instead has `0x1F` in both
                        // leading bytes and is handled by the sibling lookup.
                        let record = object.aux1 as usize;
                        (object.tile != FIXED_HIDDEN_TREASURE_OBJECT_TILE
                            && record < FIXED_HIDDEN_TREASURE_COUNT)
                            .then_some(record)
                    })
                    .map(|record| (object_slot, record))
            })
    }

    /// `containers.md §9`: locate either a native moldy-corpse object or a
    /// fixed-hidden-treasure staging record whose published pickup class is a
    /// moldy corpse. The `aux3` distinction is essential: every fixed treasure
    /// uses the same `0x1F` visual, while combat-created moldy corpses carry no
    /// fixed-record marker.
    pub fn moldy_corpse_search_slot_at(&self, x: usize, y: usize) -> Option<usize> {
        self.active_objects
            .iter()
            .copied()
            .enumerate()
            .skip(1)
            .rev()
            .find_map(|(object_slot, object)| {
                if !self.object_occupies(object, x, y) {
                    return None;
                }
                let native_moldy = object.type_byte == FIXED_HIDDEN_TREASURE_OBJECT_TILE
                    && object.tile == FIXED_HIDDEN_TREASURE_OBJECT_TILE
                    && object.fixed_hidden_treasure_record().is_none();
                let staged_record = object.fixed_hidden_treasure_record().or_else(|| {
                    let record = object.aux1 as usize;
                    (object.type_byte == FIXED_HIDDEN_TREASURE_OBJECT_TILE
                        && object.tile != FIXED_HIDDEN_TREASURE_OBJECT_TILE
                        && record < FIXED_HIDDEN_TREASURE_COUNT)
                        .then_some(record)
                });
                let staged_moldy = staged_record.is_some_and(|record| {
                    FIXED_HIDDEN_TREASURES
                        .iter()
                        .any(|entry| entry.record == record && corpse_search_pickup(entry.pickup))
                });
                (native_moldy || staged_moldy).then_some(object_slot)
            })
    }

    pub fn search_active_object_treasure_marker_at(
        &mut self,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        if self.moldy_corpse_search_slot_at(x, y).is_some() {
            return Some(match self.shared_acting_member_selection(true) {
                ActingMemberSelection::Selected(member_index) => self
                    .finish_moldy_corpse_search_at(x, y, member_index)
                    .unwrap_or(MoveOutcome::Blocked),
                ActingMemberSelection::Prompt => {
                    self.start_surface_object_chest_prompt(x, y, SurfaceChestVerb::Search)
                }
                ActingMemberSelection::NoneAble => {
                    // Not a state a keystroke can produce; see the other guards.
                    self.message.clear();
                    MoveOutcome::Blocked
                }
            });
        }
        let (object_slot, record) = self.active_object_treasure_marker_at(x, y)?;
        let Some(entry) = FIXED_HIDDEN_TREASURES
            .iter()
            .find(|entry| entry.record == record)
            .copied()
        else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        };

        // `containers.md §9` / cleak/u5-spec#175: only a surface/town Search
        // of a moldy corpse enters the special branch. It chooses the Plague
        // victim through the shared acting-member selection before consuming
        // any corpse PRNG draw. A multi-member party reuses the established
        // picker and resumes this coordinate after confirmation.
        self.clear_consumed_active_object_slot(object_slot);
        let grant = self.apply_fixed_hidden_treasure_pickup(entry.pickup, entry.state);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Found {}{}.", entry.pickup.label(), grant);
        Some(MoveOutcome::Searched)
    }

    fn finish_moldy_corpse_search_at(
        &mut self,
        x: usize,
        y: usize,
        searching_member: usize,
    ) -> Option<MoveOutcome> {
        let object_slot = self.moldy_corpse_search_slot_at(x, y)?;
        self.resolve_corpse_search(object_slot, searching_member);
        self.mark_visibility_dirty();
        self.advance_turn();
        Some(MoveOutcome::Searched)
    }

    pub fn get_fixed_hidden_treasure_pickup_at(
        &mut self,
        x: usize,
        y: usize,
    ) -> Option<MoveOutcome> {
        let (object_slot, record) = self.fixed_hidden_treasure_pickup_at(x, y)?;
        let Some(entry) = FIXED_HIDDEN_TREASURES
            .iter()
            .find(|entry| entry.record == record)
            .copied()
        else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        };
        // `containers.md §9`: Get never enters the moldy-corpse Search
        // branch. It follows the ordinary object-table consumption path.
        self.clear_consumed_active_object_slot(object_slot);
        let grant = self.apply_fixed_hidden_treasure_pickup(entry.pickup, entry.state);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Got {}{}.", entry.pickup.label(), grant);
        Some(MoveOutcome::Got)
    }

    /// `containers.md §9` "Corpse searches" + "Corpse-search odds".
    ///
    /// "A corpse search splits on a single roll before anything is narrated,
    /// and the fate of the corpse slot follows that split, not the
    /// narration." The split is rolled first, here, ahead of every message
    /// decision.
    ///
    /// "The majority arm -- seven outcomes in eight -- **clears the corpse
    /// slot first**, and only then rolls for what the search is said to have
    /// turned up. Plague is one rare result inside that already-cleared arm,
    /// and the several "nothing"-style results are the rest of it, so plague
    /// and every "nothing"-style narration alike leave the slot cleared and
    /// stage neither food nor gold. Only the minority arm -- one outcome in
    /// eight -- leaves the slot in place and rewrites it into a later
    /// food/gold pickup; the eventual pickup then follows the ordinary
    /// object-table grant rule above."
    ///
    /// `RETRACTIONS.md` R200 reverses the older reading (plague clearing the
    /// slot, every non-plague search staging a pickup); do not reinstate it.
    ///
    /// Every roll below is over a **closed** range - "both bounds are
    /// attainable - which is the detail most likely to be mis-implemented as
    /// an exclusive upper bound" - so each uses
    /// [`PlayState::random_range_u8`], which is inclusive at both ends.
    ///
    /// `cleak/u5-spec#175` publishes the previously missing staged quantity,
    /// exact selector tree, transcript, Plague victim, sound, and repaint
    /// boundaries.
    pub fn resolve_corpse_search(&mut self, slot: usize, searching_member: usize) {
        // The arm split, before any narration: "1 in 8 takes the minority
        // arm and stages a pickup; the other 7 in 8 clear the slot."
        let arm = self.random_range_u8(0, CORPSE_SEARCH_ARM_DENOMINATOR - 1);
        if arm == 0 {
            // Minority arm: the slot is *not* cleared. "Inside the minority
            // arm: 1 in 4 food, 3 in 4 gold."
            let food = self.random_range_u8(0, CORPSE_SEARCH_FOOD_DENOMINATOR - 1) == 0;
            let class = if food {
                INVENTORY_ADD_CLASS_CODE_FOOD
            } else {
                INVENTORY_ADD_CLASS_CODE_GOLD
            };
            let quantity = self.random_range_u8(1, 3);
            if let Some(object) = self.active_objects.get_mut(slot) {
                // Rewrite in place into a later food/gold pickup routed
                // through the ordinary `containers.md §8` inventory-add
                // class dispatcher (`0x02` Gold / `0x0F` Food), so the
                // eventual transfer is owned by the ordinary object-table
                // grant path and not by this helper. The pickup visual is
                // left as the body art already in the slot, which the §3
                // Get visual filter already accepts.
                object.type_byte = class;
                object.aux1 = quantity;
                // Drop the fixed-hidden-treasure record marker so the slot
                // no longer re-enters this corpse path.
                object.aux3 = 0;
            }
            self.message = format!("Thou dost find\n{}!\n", if food { "food" } else { "gold" });
            self.mark_visibility_dirty();
            return;
        }

        // Majority arm: clear the corpse slot FIRST, then narrate.
        self.clear_consumed_active_object_slot(slot);
        if self.random_range_u8(0, CORPSE_SEARCH_PLAGUE_DENOMINATOR - 1)
            == CORPSE_SEARCH_PLAGUE_VALUE
        {
            // The line is visible before the blocking rumble; status changes
            // only after the effect returns.
            self.message = "Thou dost find\nPlague!\n".to_string();
            self.emit_sound_effect(SoundEffect::CorpsePlagueRumble);
            if let Some(member) = self.party.get_mut(searching_member) {
                member.status = b'P';
            }
            // `stats-panel.md §2.2.1` row 20: the plague arm defers the
            // panel; "its food and gold arms request nothing" - those stage
            // an object for a later Get, which is already deferred on its
            // own account through `apply_object_pickup`.
            self.stats_panel_command_refresh = crate::stats_panel::PanelRefreshMode::Deferred;
            self.mark_visibility_dirty();
            return;
        }
        let upper_bound = self.random_range_u8(0, 3);
        let selector = self.random_range_u8(0, upper_bound);
        self.message = format!(
            "Thou dost find\n{}\n",
            CORPSE_SEARCH_NOTHING_MESSAGES[usize::from(selector)]
        );
    }

    pub fn apply_fixed_hidden_treasure_pickup(
        &mut self,
        pickup: HiddenTreasurePickup,
        state: u8,
    ) -> String {
        match pickup {
            HiddenTreasurePickup::Food => {
                self.food = self
                    .food
                    .saturating_add(u16::from(state))
                    .min(PARTY_FOOD_CAP);
                format!("; added {state} food")
            }
            HiddenTreasurePickup::SackOfGold => {
                self.gold = self
                    .gold
                    .saturating_add(u16::from(state))
                    .min(SHOP_GOLD_CAP);
                format!("; added {state} gold")
            }
            HiddenTreasurePickup::RingOfKeys => {
                self.keys = self.keys.saturating_add(state).min(99);
                format!("; added {state} keys")
            }
            HiddenTreasurePickup::Gem => {
                self.gems = self.gems.saturating_add(state).min(99);
                format!("; added {state} gems")
            }
            HiddenTreasurePickup::Torches => {
                self.torches = self.torches.saturating_add(state).min(99);
                format!("; added {state} torches")
            }
            HiddenTreasurePickup::Potion => {
                let subtype = (state as usize) & 7;
                self.potion_stock[subtype] = self.potion_stock[subtype].saturating_add(1).min(99);
                format!("; added 1 {} potion", potion_label(subtype))
            }
            HiddenTreasurePickup::Scroll => {
                let subtype = (state as usize) & 7;
                self.scroll_stock[subtype] = self.scroll_stock[subtype].saturating_add(1).min(99);
                format!("; added 1 scroll subtype {subtype}")
            }
            HiddenTreasurePickup::Armour
            | HiddenTreasurePickup::Weapon
            | HiddenTreasurePickup::Ring
            | HiddenTreasurePickup::Amulet => {
                let item = state as usize;
                if item < EQUIPMENT_COUNT {
                    let amount = inventory_add_equipment_units(item);
                    self.equipment_stock[item] =
                        self.equipment_stock[item].saturating_add(amount).min(99);
                    format!("; added equipment id {item}")
                } else {
                    "; no compatible inventory slot".to_string()
                }
            }
            // Corpse/body classes are not ordinary inventory grants. Only a
            // Search of MoldyCorpse is intercepted by the special branch.
            HiddenTreasurePickup::MoldyCorpse | HiddenTreasurePickup::RottingBody => {
                "; no inventory grant".to_string()
            }
        }
    }

    pub fn search_dungeon_secret(
        &mut self,
        entries: &[SecretDoorEntry],
        _chest_entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
    ) -> MoveOutcome {
        self.search_dungeon_secret_focus(
            entries,
            _chest_entries,
            scene,
            level,
            DungeonLookFocus::Ahead,
            None,
        )
    }

    pub fn search_dungeon_focus_with_game_dir(
        &mut self,
        focus: DungeonLookFocus,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        self.search_dungeon_focus_with_game_dir_as(focus, game_dir, None)
    }

    /// As [`Self::search_dungeon_focus_with_game_dir`], with the party index
    /// the shared acting-member prompt chose (`dungeon-mode.md` Section 8.1,
    /// "Who acts"). `None` falls back to the member the prompt would pick
    /// without asking.
    pub fn search_dungeon_focus_with_game_dir_as(
        &mut self,
        focus: DungeonLookFocus,
        game_dir: &Path,
        actor: Option<usize>,
    ) -> io::Result<MoveOutcome> {
        let Area::Dungeon { scene, level } = self.area else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        };
        let entries = load_secret_door_entries(game_dir)?.unwrap_or_default();
        let chest_entries = load_dungeon_chest_content_entries(game_dir)?;
        Ok(self.search_dungeon_secret_focus(
            &entries,
            chest_entries.as_deref(),
            scene,
            level,
            focus,
            actor,
        ))
    }

    pub fn search_dungeon_secret_focus(
        &mut self,
        entries: &[SecretDoorEntry],
        _chest_entries: Option<&[DungeonChestContentEntry]>,
        scene: DungeonScene,
        level: u8,
        focus: DungeonLookFocus,
        actor: Option<usize>,
    ) -> MoveOutcome {
        if !self.has_personal_light() {
            // `RETRACTIONS.md` R323: the unlit Search refusal is a *find*
            // line that breaks after the colon, with a leading blank row -
            // not a "too dark" message, of which "no ... literal exists
            // anywhere in the resident data image".
            self.message = DUNGEON_SEARCH_DARKNESS_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }
        // `dungeon-mode.md` Section 8.1: "Search prints `You find:`
        // unconditionally, then exactly one outcome line."
        // `RETRACTIONS.md` R324: `Nothing of note.` is one of those
        // *outcomes*, not the preamble.
        self.emit_message_line(DUNGEON_SEARCH_PREAMBLE);
        let (tx, ty) = self.dungeon_look_focus_coord(focus);
        let idx = dungeon_cell_index(level, tx, ty);
        let tile = self.grid[idx];
        if let Some(reveal_cell) = entries.iter().find_map(|entry| match *entry {
            SecretDoorEntry::Dungeon {
                scene: entry_scene,
                level: entry_level,
                x,
                y,
                reveal_cell,
                expected_cell,
            } if entry_scene == scene
                && entry_level == level
                && x == tx
                && y == ty
                && expected_cell.map_or(true, |expected| expected == tile) =>
            {
                Some(reveal_cell)
            }
            _ => None,
        }) {
            self.grid[idx] = reveal_cell;
            self.mark_visibility_dirty();
            self.advance_turn();
            // Section 8.1: the found-door outcome line, shared with the
            // `0xD?` wall branch below (`RETRACTIONS.md` R322).
            self.message = DUNGEON_SEARCH_HIDDEN_DOOR.to_string();
            return MoveOutcome::Searched;
        }

        match tile >> 4 {
            0x4 => self.search_dungeon_chest(scene, level, tx, ty, tile, actor),
            _ if tile == 0x60 => self.search_dungeon_pit(scene, level, tx, ty),
            _ if dungeon_search_secret_pit_reveal(tile, level).is_some() => {
                self.search_dungeon_secret_pit(scene, level, tx, ty, idx, tile)
            }
            _ if is_dungeon_bomb_trap(tile) => {
                self.search_dungeon_bomb_trap(scene, level, tx, ty, idx, tile, actor)
            }
            _ if dungeon_search_wall_rewrite(tile, scene.presentation_flavour()).is_some() => {
                self.search_dungeon_wall_rewrite(scene, level, tx, ty, idx, tile)
            }
            _ => self.search_dungeon_feature(scene, level, tx, ty, tile),
        }
    }

    pub fn search_dungeon_pit(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
    ) -> MoveOutcome {
        self.advance_turn();
        // `dungeon-mode.md` Section 8, `0x60` row: "`Nothing hidden` /
        // `in the pit.` - the line break is in the data. No state change."
        let _ = (scene, level, x, y);
        self.message = DUNGEON_SEARCH_NOTHING_IN_PIT.to_string();
        MoveOutcome::Searched
    }

    pub fn search_dungeon_secret_pit(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
    ) -> MoveOutcome {
        let Some(reveal) = dungeon_search_secret_pit_reveal(tile, level) else {
            return self.search_dungeon_feature(scene, level, x, y, tile);
        };
        // `RETRACTIONS.md` R322: Search on exact `0x61` reports `A pit!`,
        // **not** a found secret door - that line belongs to the `0xD?` wall
        // branch. Every mechanical half of the row re-derived unchanged.
        self.message = DUNGEON_SEARCH_A_PIT.to_string();
        self.grid[idx] = 0x60;
        if matches!(
            reveal,
            DungeonSearchSecretPitReveal::RewriteAndStampLevelBelow
        ) {
            let below_idx = dungeon_cell_index(level + 1, x, y);
            self.grid[below_idx] |= DUNGEON_RUNTIME_VARIANT_BIT;
        }
        self.mark_visibility_dirty();
        self.run_map_viewport_dissolve(MapViewportDissolveSource::DungeonSearchReveal {
            scene,
            level,
            x: x as u8,
            y: y as u8,
            original_cell: tile,
            revealed_cell: 0x60,
        });
        self.advance_turn();
        MoveOutcome::Searched
    }

    pub fn search_dungeon_bomb_trap(
        &mut self,
        _scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
        actor: Option<usize>,
    ) -> MoveOutcome {
        let _ = (x, y, tile);
        // `dungeon-mode.md` Section 8, `0x62` row: roll `1..30` against
        // `(2 * Z - member Dexterity + 30) / 2`, the member being the one
        // the acting-member prompt chose; the roll is a shared-PRNG draw.
        let dexterity = self.dungeon_search_actor_dexterity(actor);
        let threshold = Self::dungeon_chest_pick_threshold(level, dexterity);
        let roll = self.random_range_u8(1, 30);
        match dungeon_bomb_search_outcome(threshold, roll) {
            DungeonBombSearchOutcome::NothingOnPit => {
                self.advance_turn();
                // `RETRACTIONS.md` R324: "a `0x62` roll at or below the
                // threshold" takes the `Nothing of note.` **outcome**.
                self.message = DUNGEON_SEARCH_NOTHING_OF_NOTE.to_string();
            }
            DungeonBombSearchOutcome::SpringBomb => {
                self.grid[idx] = 0x00;
                self.mark_visibility_dirty();
                self.advance_turn();
                // `dungeon-mode.md` Section 8, `0x62` row.
                self.message = DUNGEON_SEARCH_A_BOMB_TRAP.to_string();
            }
        }
        MoveOutcome::Searched
    }

    pub fn search_dungeon_wall_rewrite(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        idx: usize,
        tile: u8,
    ) -> MoveOutcome {
        match dungeon_search_wall_rewrite(tile, scene.presentation_flavour()) {
            Some(DungeonSearchWallRewrite::NarrateOnly) => {
                self.search_dungeon_feature(scene, level, x, y, tile)
            }
            Some(DungeonSearchWallRewrite::ToFlavourFind(reveal_cell)) => {
                // `dungeon-mode.md` Section 8: the flavour-three `0xC?` branch
                // "print[s] that nothing is hidden on the skeleton and then
                // that it crumbles away". Section 8.1 gives the two-line
                // result exactly.
                self.emit_message_line(DUNGEON_SEARCH_NOTHING_ON_SKELETON);
                self.message = DUNGEON_SEARCH_SKELETON_CRUMBLES.to_string();
                self.grid[idx] = reveal_cell;
                self.mark_visibility_dirty();
                self.run_map_viewport_dissolve(MapViewportDissolveSource::DungeonSearchReveal {
                    scene,
                    level,
                    x: x as u8,
                    y: y as u8,
                    original_cell: tile,
                    revealed_cell: reveal_cell,
                });
                self.advance_turn();
                MoveOutcome::Searched
            }
            Some(DungeonSearchWallRewrite::ToHiddenWallReveal(reveal_cell)) => {
                // `RETRACTIONS.md` R322: `A hidden door!` is the `0xD?` wall
                // branch's line.
                self.message = DUNGEON_SEARCH_HIDDEN_DOOR.to_string();
                self.grid[idx] = reveal_cell;
                self.mark_visibility_dirty();
                self.run_map_viewport_dissolve(MapViewportDissolveSource::DungeonSearchReveal {
                    scene,
                    level,
                    x: x as u8,
                    y: y as u8,
                    original_cell: tile,
                    revealed_cell: reveal_cell,
                });
                self.advance_turn();
                MoveOutcome::Searched
            }
            None => self.search_dungeon_feature(scene, level, x, y, tile),
        }
    }

    pub fn search_dungeon_feature(
        &mut self,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> MoveOutcome {
        self.advance_turn();
        // `dungeon-mode.md` Section 8.1 "Search outcomes". Classes with no
        // published outcome literal - the energy fields, which "reuse the
        // L-Look wording of this section" - keep the descriptive fallback
        // rather than having a line invented for them.
        let _ = (scene, level, x, y);
        self.message = match dungeon_search_outcome_line(tile, scene.presentation_flavour()) {
            Some(line) => line.to_string(),
            // Section 8, pit-family table: "Other `0x6?` values | No extra
            // Search-specific handling beyond the preamble."
            None if tile >> 4 == 0x6 => String::new(),
            None => format!("{}.", dungeon_search_description(tile)),
        };
        MoveOutcome::Searched
    }
}

fn native_object_pickup_grant(object: ActiveObject) -> Option<ObjectPickupGrant> {
    let subtype = object.aux1;
    let grant = match inventory_add_class(object.type_byte) {
        InventoryAddClass::MustOpenFirst | InventoryAddClass::NothingToGet => return None,
        InventoryAddClass::Gold => ObjectPickupGrant {
            kind: ObjectPickupKind::Gold,
            amount: subtype,
        },
        InventoryAddClass::Potion => ObjectPickupGrant {
            kind: ObjectPickupKind::Potion((subtype & SCROLL_GRANT_LABEL_MASK) as usize),
            amount: 1,
        },
        InventoryAddClass::ScrollOrPlans => {
            if subtype & !SCROLL_GRANT_LABEL_MASK != 0 {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::HmsCapePlans,
                    amount: 1,
                }
            } else {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::Scroll(scroll_grant_label_id(subtype) as usize),
                    amount: 1,
                }
            }
        }
        InventoryAddClass::Equipment => ObjectPickupGrant {
            kind: ObjectPickupKind::Equipment(subtype as usize),
            amount: 1,
        },
        InventoryAddClass::Key => {
            if subtype & 0x80 != 0 {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::SkullKeys,
                    amount: subtype & 0x7f,
                }
            } else {
                ObjectPickupGrant {
                    kind: ObjectPickupKind::Keys,
                    amount: subtype,
                }
            }
        }
        InventoryAddClass::Gem => ObjectPickupGrant {
            kind: ObjectPickupKind::Gems,
            amount: subtype,
        },
        InventoryAddClass::Torch => ObjectPickupGrant {
            kind: ObjectPickupKind::Torches,
            amount: subtype,
        },
        InventoryAddClass::SandalwoodBox => ObjectPickupGrant {
            kind: ObjectPickupKind::SandalwoodBox,
            amount: 1,
        },
        InventoryAddClass::Food => ObjectPickupGrant {
            kind: ObjectPickupKind::Food,
            amount: subtype,
        },
        InventoryAddClass::Moonstone => ObjectPickupGrant {
            kind: ObjectPickupKind::Moonstone(subtype as usize),
            amount: 1,
        },
        InventoryAddClass::MagicCarpet => ObjectPickupGrant {
            kind: ObjectPickupKind::MagicCarpet,
            amount: 1,
        },
        InventoryAddClass::ShadowlordShard => ObjectPickupGrant {
            kind: ObjectPickupKind::ShadowlordShard(subtype as usize),
            amount: 1,
        },
        InventoryAddClass::CrownOfLordBritish => ObjectPickupGrant {
            kind: ObjectPickupKind::CrownOfLordBritish,
            amount: 1,
        },
        InventoryAddClass::SceptreOfLordBritish => ObjectPickupGrant {
            kind: ObjectPickupKind::SceptreOfLordBritish,
            amount: 1,
        },
        InventoryAddClass::AmuletOfLordBritish => ObjectPickupGrant {
            kind: ObjectPickupKind::AmuletOfLordBritish,
            amount: 1,
        },
    };
    Some(grant)
}

fn surface_search_trap_visibility_label(visibility: SearchTrapVisibility) -> &'static str {
    match visibility {
        SearchTrapVisibility::NoTrap => "no trap",
        SearchTrapVisibility::SimpleTrap => "simple trap",
        SearchTrapVisibility::ComplexTrap => "complex trap",
        SearchTrapVisibility::GenericTrap => "trap",
    }
}

/// `commands.md §5.8`: the four published rows the container-object branch
/// prints under the shared preamble. Distinct from the diagnostic label above,
/// which names the same assessment for the engine's own record.
fn surface_search_trap_visibility_line(visibility: SearchTrapVisibility) -> &'static str {
    match visibility {
        SearchTrapVisibility::NoTrap => SEARCH_TRAP_NONE_LINE,
        SearchTrapVisibility::SimpleTrap => SEARCH_TRAP_SIMPLE_LINE,
        SearchTrapVisibility::ComplexTrap => SEARCH_TRAP_COMPLEX_LINE,
        SearchTrapVisibility::GenericTrap => SEARCH_TRAP_GENERIC_LINE,
    }
}

#[derive(Clone, Copy)]
pub struct RareReagentHarvestPoint {
    pub index: usize,
    pub x: usize,
    pub y: usize,
    pub reagent_index: usize,
    /// The engine-side name used in `diagnostics`.
    pub label: &'static str,
    /// `commands.md §5.8`: the published row this harvest prints after the
    /// count and the `_sprigs_of` fragment.
    pub line: &'static str,
}

pub const RARE_REAGENT_HARVEST_POINTS: [RareReagentHarvestPoint; RARE_REAGENT_HARVEST_POINT_COUNT] = [
    RareReagentHarvestPoint {
        index: 0,
        x: 182,
        y: 54,
        reagent_index: REAGENT_MANDRAKE,
        label: "Mandrake Root",
        line: SEARCH_REAGENT_MANDRAKE_LINE,
    },
    RareReagentHarvestPoint {
        index: 1,
        x: 97,
        y: 165,
        reagent_index: REAGENT_MANDRAKE,
        label: "Mandrake Root",
        line: SEARCH_REAGENT_MANDRAKE_LINE,
    },
    RareReagentHarvestPoint {
        index: 2,
        x: 44,
        y: 137,
        reagent_index: REAGENT_NIGHTSHADE,
        label: "Nightshade",
        line: SEARCH_REAGENT_NIGHTSHADE_LINE,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasureTarget {
    World(WorldPlane),
    Town(u8),
}

/// `containers.md §9` "Corpse-search odds": "**1 in 8** takes the minority
/// arm and stages a pickup; the other **7 in 8** clear the slot." The range
/// is closed, so the roll is an inclusive `0..=7`.
pub const CORPSE_SEARCH_ARM_DENOMINATOR: u8 = 8;

/// `containers.md §9` "Corpse-search odds": "Inside the minority arm:
/// **1 in 4 food, 3 in 4 gold.**" An earlier revision published one third
/// against two thirds; that figure is withdrawn.
pub const CORPSE_SEARCH_FOOD_DENOMINATOR: u8 = 4;

/// `containers.md §9` "Corpse-search odds": "Inside the cleared arm: Plague
/// on **1 in 32**." An earlier revision published one in thirty-one; that
/// figure is withdrawn as an inclusive/exclusive off-by-one.
pub const CORPSE_SEARCH_PLAGUE_DENOMINATOR: u8 = 32;
/// `containers.md §9`: exactly this value, not zero, selects Plague.
pub const CORPSE_SEARCH_PLAGUE_VALUE: u8 = 19;

/// `inventory.md §7.1` Skull Key target rewrites: `0x97` becomes `0xB8`
/// and `0x98` becomes `0xBA`; every other tile fails.
pub const SKULL_KEY_TARGET_FIRST: u8 = 0x97;
pub const SKULL_KEY_TARGET_FIRST_UNLOCKED: u8 = 0xb8;
pub const SKULL_KEY_TARGET_SECOND: u8 = 0x98;
pub const SKULL_KEY_TARGET_SECOND_UNLOCKED: u8 = 0xba;

pub const fn skull_key_unlock_rewrite(tile: u8) -> Option<u8> {
    match tile {
        SKULL_KEY_TARGET_FIRST => Some(SKULL_KEY_TARGET_FIRST_UNLOCKED),
        SKULL_KEY_TARGET_SECOND => Some(SKULL_KEY_TARGET_SECOND_UNLOCKED),
        _ => None,
    }
}

/// `containers.md §8` inventory-add class codes for the two families the
/// corpse minority arm can stage: "`0x02` | Gold" and "`0x0F` | Food /
/// grain".
pub const INVENTORY_ADD_CLASS_CODE_GOLD: u8 = 0x02;
pub const INVENTORY_ADD_CLASS_CODE_FOOD: u8 = 0x0f;

/// `containers.md §8` class codes for the remaining surface/town chest
/// families that `§4` stages into an active-object slot.
pub const INVENTORY_ADD_CLASS_CODE_CONTAINER: u8 = 0x01;
pub const INVENTORY_ADD_CLASS_CODE_POTION: u8 = 0x03;
pub const INVENTORY_ADD_CLASS_CODE_SCROLL: u8 = 0x04;
pub const INVENTORY_ADD_CLASS_CODE_KEY: u8 = 0x07;
pub const INVENTORY_ADD_CLASS_CODE_GEM: u8 = 0x08;
pub const INVENTORY_ADD_CLASS_CODE_TORCH: u8 = 0x0d;
/// `§8` lists `0x05`, `0x06` and `0x09..0x0C` as one Equipment arm that
/// adds stock "by the supplied equipment id". `containers.md §4.1`
/// (u5-spec#298 Q18) now publishes which family id each grant stages -
/// weapon 5, shield 6, helm 9, ring 10, armour 11, amulet 12 - and the
/// secondary pool stages through [`chest_secondary_pool_family_id`]; this
/// is the helm's.
pub const INVENTORY_ADD_CLASS_CODE_EQUIPMENT: u8 = 0x09;
pub const INVENTORY_ADD_CLASS_CODE_WEAPON: u8 = 0x05;
pub const INVENTORY_ADD_CLASS_CODE_SHIELD: u8 = 0x06;
pub const INVENTORY_ADD_CLASS_CODE_HELM: u8 = 0x09;
pub const INVENTORY_ADD_CLASS_CODE_RING: u8 = 0x0a;
pub const INVENTORY_ADD_CLASS_CODE_ARMOUR: u8 = 0x0b;
pub const INVENTORY_ADD_CLASS_CODE_AMULET: u8 = 0x0c;

/// `containers.md §4.1` (u5-spec#298 Q18): the staged record's last byte
/// "set to `0x20` when the scene byte is `0x80` or above" (combat).
pub const CHEST_GRANT_COMBAT_LAST_BYTE: u8 = 0x20;

/// `containers.md §4.1` (u5-spec#298 Q18): the family id - and so the
/// drawn tile `0x100 + id` - of a secondary-pool equipment row, grouped as
/// [`chest_secondary_pool_transcript_line`] groups the names.
pub const fn chest_secondary_pool_family_id(row: usize) -> u8 {
    match row {
        0..=3 => INVENTORY_ADD_CLASS_CODE_HELM,
        4..=8 => INVENTORY_ADD_CLASS_CODE_SHIELD,
        9..=15 => INVENTORY_ADD_CLASS_CODE_ARMOUR,
        16..=41 => INVENTORY_ADD_CLASS_CODE_WEAPON,
        42..=44 => INVENTORY_ADD_CLASS_CODE_RING,
        _ => INVENTORY_ADD_CLASS_CODE_AMULET,
    }
}

/// `containers.md §4.1`: the Gold and Chest-marker rows' first quantity
/// draws, `1..90` and `1..10`, are discarded.
pub const CHEST_GOLD_DISCARDED_DRAW_MAX: u8 = 90;
pub const CHEST_MARKER_DISCARDED_DRAW_MAX: u8 = 10;

/// `containers.md §9` exact ordinary narration selector table. The selector
/// is an inclusive draw `0..upper`, after an inclusive `upper` draw `0..3`.
const CORPSE_SEARCH_NOTHING_MESSAGES: [&str; 4] = ["nothing!", "worms!", "guts!", "a bloody pulp!"];

/// `containers.md §9`: the sole pickup class whose slot fate is
/// decided by [`PlayState::resolve_corpse_search`] rather than by the
/// ordinary "clear the consumed object-table slot" rule. A rotting body does
/// not enter this branch.
pub const fn corpse_search_pickup(pickup: HiddenTreasurePickup) -> bool {
    matches!(pickup, HiddenTreasurePickup::MoldyCorpse)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasurePickup {
    Armour,
    Weapon,
    Scroll,
    Potion,
    Gem,
    Food,
    Torches,
    Ring,
    Amulet,
    RingOfKeys,
    SackOfGold,
    MoldyCorpse,
    RottingBody,
}

#[cfg(test)]
const HIDDEN_TREASURE_PICKUP_CLASS_COUNT: usize = 13;

#[cfg(test)]
fn fixed_hidden_treasure_pickup_code(pickup: HiddenTreasurePickup) -> u64 {
    match pickup {
        HiddenTreasurePickup::Armour => 0,
        HiddenTreasurePickup::Weapon => 1,
        HiddenTreasurePickup::Scroll => 2,
        HiddenTreasurePickup::Potion => 3,
        HiddenTreasurePickup::Gem => 4,
        HiddenTreasurePickup::Food => 5,
        HiddenTreasurePickup::Torches => 6,
        HiddenTreasurePickup::Ring => 7,
        HiddenTreasurePickup::Amulet => 8,
        HiddenTreasurePickup::RingOfKeys => 9,
        HiddenTreasurePickup::SackOfGold => 10,
        HiddenTreasurePickup::MoldyCorpse => 11,
        HiddenTreasurePickup::RottingBody => 12,
    }
}

impl HiddenTreasurePickup {
    /// `hidden-treasures.md §2.1` "What a found record prints": "A staged
    /// record is narrated with one line from a fixed seventeen-name
    /// vocabulary, selected by the record's pickup class, on the row after the
    /// Search preamble of `systems/commands.md` Section 5.8."
    ///
    /// These thirteen are "the whole reachable set". The vocabulary's other
    /// four - `a chest!`, `a shield!`, `a helm!` and its own copy of
    /// `a strange rock!` - have no shipped record, and a class outside the
    /// vocabulary prints the capitalised [`SEARCH_FOUND_NOTHING_OF_NOTE`].
    pub fn narrated_row(self) -> &'static str {
        match self {
            Self::Armour => "some armour!",
            Self::Weapon => "a weapon!",
            Self::Scroll => "a scroll!",
            Self::Potion => "a potion!",
            Self::Gem => "a gem!",
            Self::Food => "some food!",
            Self::Torches => "some torches!",
            Self::Ring => "a ring!",
            Self::Amulet => "an amulet!",
            Self::RingOfKeys => "a ring of keys!",
            Self::SackOfGold => "a sack of gold!",
            Self::MoldyCorpse => "a moldy corpse!",
            Self::RottingBody => "a rotting body!",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Armour => "armour",
            Self::Weapon => "weapon",
            Self::Scroll => "scroll",
            Self::Potion => "potion",
            Self::Gem => "gem",
            Self::Food => "food",
            Self::Torches => "torches",
            Self::Ring => "ring",
            Self::Amulet => "amulet",
            Self::RingOfKeys => "ring of keys",
            Self::SackOfGold => "sack of gold",
            Self::MoldyCorpse => "moldy corpse",
            Self::RottingBody => "rotting body",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HiddenTreasureRule {
    OneShot,
    KeyNpcGated,
    Daily,
    SingleUseNpcGated,
}

#[cfg(test)]
const HIDDEN_TREASURE_RULE_COUNT: usize = 4;

#[cfg(test)]
fn fixed_hidden_treasure_rule_code(rule: HiddenTreasureRule) -> u64 {
    match rule {
        HiddenTreasureRule::OneShot => 0,
        HiddenTreasureRule::KeyNpcGated => 1,
        HiddenTreasureRule::Daily => 2,
        HiddenTreasureRule::SingleUseNpcGated => 3,
    }
}

#[cfg(test)]
fn fixed_hidden_treasure_target_code(target: HiddenTreasureTarget) -> u64 {
    match target {
        HiddenTreasureTarget::World(WorldPlane::Britannia) => 0,
        HiddenTreasureTarget::World(WorldPlane::Underworld) => 255,
        HiddenTreasureTarget::Town(scene) => 1000 + u64::from(scene),
    }
}

#[cfg(test)]
fn fixed_hidden_treasure_floor_code(floor: i8) -> u64 {
    if floor == -1 { 255 } else { floor as u8 as u64 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FixedHiddenTreasureEntry {
    pub record: usize,
    pub target: HiddenTreasureTarget,
    pub floor: i8,
    pub x: usize,
    pub y: usize,
    pub pickup: HiddenTreasurePickup,
    pub state: u8,
    pub rule: HiddenTreasureRule,
}

const fn ht(
    record: usize,
    target: HiddenTreasureTarget,
    floor: i8,
    x: usize,
    y: usize,
    pickup: HiddenTreasurePickup,
    state: u8,
    rule: HiddenTreasureRule,
) -> FixedHiddenTreasureEntry {
    FixedHiddenTreasureEntry {
        record,
        target,
        floor,
        x,
        y,
        pickup,
        state,
        rule,
    }
}

pub const FIXED_HIDDEN_TREASURES: &[FixedHiddenTreasureEntry] = &[
    ht(
        0,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        1,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        2,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        3,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        4,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        5,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        6,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        7,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        8,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        9,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        10,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Armour,
        15,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        11,
        HiddenTreasureTarget::World(WorldPlane::Underworld),
        -1,
        233,
        233,
        HiddenTreasurePickup::Weapon,
        41,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        12,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        15,
        HiddenTreasurePickup::Scroll,
        255,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        13,
        HiddenTreasureTarget::Town(18),
        -1,
        6,
        8,
        HiddenTreasurePickup::RingOfKeys,
        9,
        HiddenTreasureRule::KeyNpcGated,
    ),
    ht(
        14,
        HiddenTreasureTarget::Town(5),
        0,
        2,
        2,
        HiddenTreasurePickup::RingOfKeys,
        133,
        HiddenTreasureRule::Daily,
    ),
    ht(
        15,
        HiddenTreasureTarget::World(WorldPlane::Britannia),
        0,
        80,
        64,
        HiddenTreasurePickup::Weapon,
        39,
        HiddenTreasureRule::SingleUseNpcGated,
    ),
    ht(
        16,
        HiddenTreasureTarget::Town(18),
        1,
        6,
        7,
        HiddenTreasurePickup::Weapon,
        35,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        17,
        HiddenTreasureTarget::Town(18),
        1,
        6,
        23,
        HiddenTreasurePickup::Weapon,
        40,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        18,
        HiddenTreasureTarget::Town(1),
        0,
        5,
        8,
        HiddenTreasurePickup::Gem,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        19,
        HiddenTreasureTarget::Town(1),
        0,
        6,
        25,
        HiddenTreasurePickup::Armour,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        20,
        HiddenTreasureTarget::Town(1),
        0,
        8,
        25,
        HiddenTreasurePickup::Armour,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        21,
        HiddenTreasureTarget::Town(1),
        0,
        5,
        23,
        HiddenTreasurePickup::Potion,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        22,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        13,
        HiddenTreasurePickup::Potion,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        23,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        14,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        24,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        16,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        25,
        HiddenTreasureTarget::Town(1),
        0,
        13,
        17,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        26,
        HiddenTreasureTarget::Town(1),
        1,
        19,
        24,
        HiddenTreasurePickup::Food,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        27,
        HiddenTreasureTarget::Town(1),
        0,
        3,
        27,
        HiddenTreasurePickup::Torches,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        28,
        HiddenTreasureTarget::Town(1),
        0,
        29,
        27,
        HiddenTreasurePickup::Ring,
        42,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        29,
        HiddenTreasureTarget::Town(2),
        0,
        1,
        2,
        HiddenTreasurePickup::Food,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        30,
        HiddenTreasureTarget::Town(2),
        0,
        26,
        6,
        HiddenTreasurePickup::Weapon,
        20,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        31,
        HiddenTreasureTarget::Town(2),
        1,
        6,
        24,
        HiddenTreasurePickup::Ring,
        43,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        32,
        HiddenTreasureTarget::Town(3),
        0,
        16,
        21,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        33,
        HiddenTreasureTarget::Town(3),
        0,
        10,
        20,
        HiddenTreasurePickup::Food,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        34,
        HiddenTreasureTarget::Town(3),
        0,
        1,
        29,
        HiddenTreasurePickup::Food,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        35,
        HiddenTreasureTarget::Town(3),
        0,
        23,
        30,
        HiddenTreasurePickup::Weapon,
        38,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        36,
        HiddenTreasureTarget::Town(3),
        0,
        29,
        1,
        HiddenTreasurePickup::Torches,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        37,
        HiddenTreasureTarget::Town(4),
        0,
        11,
        29,
        HiddenTreasurePickup::Weapon,
        18,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        38,
        HiddenTreasureTarget::Town(4),
        0,
        26,
        22,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        39,
        HiddenTreasureTarget::Town(4),
        0,
        26,
        1,
        HiddenTreasurePickup::Scroll,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        40,
        HiddenTreasureTarget::Town(4),
        0,
        2,
        13,
        HiddenTreasurePickup::MoldyCorpse,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        41,
        HiddenTreasureTarget::Town(4),
        0,
        2,
        14,
        HiddenTreasurePickup::MoldyCorpse,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        42,
        HiddenTreasureTarget::Town(4),
        0,
        4,
        14,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        43,
        HiddenTreasureTarget::Town(4),
        0,
        3,
        16,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        44,
        HiddenTreasureTarget::Town(4),
        0,
        2,
        18,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        45,
        HiddenTreasureTarget::Town(4),
        0,
        3,
        16,
        HiddenTreasurePickup::Weapon,
        21,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        46,
        HiddenTreasureTarget::Town(4),
        -1,
        22,
        27,
        HiddenTreasurePickup::Weapon,
        37,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        47,
        HiddenTreasureTarget::Town(4),
        -1,
        22,
        20,
        HiddenTreasurePickup::RingOfKeys,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        48,
        HiddenTreasureTarget::Town(5),
        0,
        8,
        27,
        HiddenTreasurePickup::SackOfGold,
        99,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        49,
        HiddenTreasureTarget::Town(5),
        1,
        11,
        13,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        50,
        HiddenTreasureTarget::Town(5),
        1,
        11,
        12,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        51,
        HiddenTreasureTarget::Town(5),
        1,
        21,
        8,
        HiddenTreasurePickup::Potion,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        52,
        HiddenTreasureTarget::Town(5),
        1,
        23,
        8,
        HiddenTreasurePickup::Potion,
        2,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        53,
        HiddenTreasureTarget::Town(5),
        1,
        23,
        7,
        HiddenTreasurePickup::Scroll,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        54,
        HiddenTreasureTarget::Town(6),
        1,
        6,
        24,
        HiddenTreasurePickup::Gem,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        55,
        HiddenTreasureTarget::Town(7),
        0,
        2,
        5,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        56,
        HiddenTreasureTarget::Town(7),
        0,
        7,
        6,
        HiddenTreasurePickup::Potion,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        57,
        HiddenTreasureTarget::Town(8),
        1,
        18,
        21,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        58,
        HiddenTreasureTarget::Town(8),
        1,
        21,
        25,
        HiddenTreasurePickup::RingOfKeys,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        59,
        HiddenTreasureTarget::Town(9),
        0,
        12,
        10,
        HiddenTreasurePickup::Torches,
        9,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        60,
        HiddenTreasureTarget::Town(11),
        0,
        15,
        21,
        HiddenTreasurePickup::Potion,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        61,
        HiddenTreasureTarget::Town(11),
        0,
        9,
        14,
        HiddenTreasurePickup::Gem,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        62,
        HiddenTreasureTarget::Town(11),
        0,
        12,
        16,
        HiddenTreasurePickup::SackOfGold,
        50,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        63,
        HiddenTreasureTarget::Town(13),
        0,
        2,
        24,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        64,
        HiddenTreasureTarget::Town(14),
        0,
        12,
        16,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        65,
        HiddenTreasureTarget::Town(14),
        0,
        16,
        14,
        HiddenTreasurePickup::Amulet,
        45,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        66,
        HiddenTreasureTarget::Town(14),
        0,
        12,
        17,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        67,
        HiddenTreasureTarget::Town(14),
        0,
        12,
        14,
        HiddenTreasurePickup::Potion,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        68,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        20,
        HiddenTreasurePickup::Armour,
        10,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        69,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        21,
        HiddenTreasurePickup::Armour,
        11,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        70,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        22,
        HiddenTreasurePickup::Armour,
        9,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        71,
        HiddenTreasureTarget::Town(17),
        0,
        7,
        23,
        HiddenTreasurePickup::Armour,
        12,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        72,
        HiddenTreasureTarget::Town(17),
        1,
        13,
        21,
        HiddenTreasurePickup::Weapon,
        30,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        73,
        HiddenTreasureTarget::Town(17),
        -1,
        18,
        7,
        HiddenTreasurePickup::RingOfKeys,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        74,
        HiddenTreasureTarget::Town(17),
        -1,
        23,
        20,
        HiddenTreasurePickup::Ring,
        44,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        75,
        HiddenTreasureTarget::Town(18),
        1,
        18,
        17,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        76,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        13,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        77,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        14,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        78,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        16,
        HiddenTreasurePickup::Scroll,
        2,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        79,
        HiddenTreasureTarget::Town(18),
        2,
        6,
        17,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        80,
        HiddenTreasureTarget::Town(18),
        2,
        7,
        19,
        HiddenTreasurePickup::Ring,
        43,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        81,
        HiddenTreasureTarget::Town(19),
        0,
        2,
        3,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        82,
        HiddenTreasureTarget::Town(19),
        0,
        7,
        5,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        83,
        HiddenTreasureTarget::Town(19),
        0,
        7,
        7,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        84,
        HiddenTreasureTarget::Town(19),
        0,
        2,
        7,
        HiddenTreasurePickup::RottingBody,
        0,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        85,
        HiddenTreasureTarget::Town(19),
        0,
        7,
        5,
        HiddenTreasurePickup::Scroll,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        86,
        HiddenTreasureTarget::Town(19),
        0,
        2,
        3,
        HiddenTreasurePickup::Ring,
        44,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        87,
        HiddenTreasureTarget::Town(20),
        0,
        25,
        18,
        HiddenTreasurePickup::Gem,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        88,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        13,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        89,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        14,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        90,
        HiddenTreasureTarget::Town(21),
        0,
        2,
        16,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        91,
        HiddenTreasureTarget::Town(22),
        0,
        13,
        13,
        HiddenTreasurePickup::Ring,
        42,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        92,
        HiddenTreasureTarget::Town(22),
        0,
        12,
        3,
        HiddenTreasurePickup::RingOfKeys,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        93,
        HiddenTreasureTarget::Town(23),
        0,
        1,
        15,
        HiddenTreasurePickup::Amulet,
        47,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        94,
        HiddenTreasureTarget::Town(24),
        0,
        22,
        19,
        HiddenTreasurePickup::RingOfKeys,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        95,
        HiddenTreasureTarget::Town(24),
        0,
        22,
        19,
        HiddenTreasurePickup::Potion,
        3,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        96,
        HiddenTreasureTarget::Town(25),
        0,
        16,
        25,
        HiddenTreasurePickup::Scroll,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        97,
        HiddenTreasureTarget::Town(26),
        0,
        4,
        11,
        HiddenTreasurePickup::Amulet,
        46,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        98,
        HiddenTreasureTarget::Town(27),
        0,
        17,
        11,
        HiddenTreasurePickup::Potion,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        99,
        HiddenTreasureTarget::Town(27),
        0,
        17,
        10,
        HiddenTreasurePickup::Potion,
        4,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        100,
        HiddenTreasureTarget::Town(28),
        0,
        22,
        19,
        HiddenTreasurePickup::Scroll,
        5,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        101,
        HiddenTreasureTarget::Town(30),
        2,
        9,
        23,
        HiddenTreasurePickup::Ring,
        42,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        102,
        HiddenTreasureTarget::Town(30),
        2,
        7,
        23,
        HiddenTreasurePickup::Amulet,
        46,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        103,
        HiddenTreasureTarget::Town(30),
        2,
        7,
        20,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        104,
        HiddenTreasureTarget::Town(30),
        1,
        19,
        22,
        HiddenTreasurePickup::Weapon,
        18,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        105,
        HiddenTreasureTarget::Town(30),
        1,
        17,
        22,
        HiddenTreasurePickup::Amulet,
        46,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        106,
        HiddenTreasureTarget::Town(31),
        0,
        3,
        6,
        HiddenTreasurePickup::Potion,
        1,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        107,
        HiddenTreasureTarget::Town(31),
        0,
        7,
        19,
        HiddenTreasurePickup::Food,
        20,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        108,
        HiddenTreasureTarget::Town(32),
        1,
        21,
        8,
        HiddenTreasurePickup::Potion,
        7,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        109,
        HiddenTreasureTarget::Town(1),
        1,
        24,
        19,
        HiddenTreasurePickup::Food,
        16,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        110,
        HiddenTreasureTarget::Town(1),
        1,
        24,
        20,
        HiddenTreasurePickup::Food,
        13,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        111,
        HiddenTreasureTarget::Town(17),
        2,
        12,
        12,
        HiddenTreasurePickup::Potion,
        6,
        HiddenTreasureRule::OneShot,
    ),
    ht(
        112,
        HiddenTreasureTarget::Town(17),
        2,
        12,
        12,
        HiddenTreasurePickup::Scroll,
        7,
        HiddenTreasureRule::OneShot,
    ),
];

#[cfg(test)]
mod hidden_treasure_record_15_spec_tests {
    use super::{FIXED_HIDDEN_TREASURES, FixedHiddenTreasureEntry, HiddenTreasurePickup};
    use crate::test_fixtures::{open_world_grid, world_state};
    use crate::*;

    fn record_15() -> FixedHiddenTreasureEntry {
        *FIXED_HIDDEN_TREASURES
            .iter()
            .find(|entry| entry.record == 15)
            .expect("record 15 is published in hidden-treasures.md §3")
    }

    /// `hidden-treasures.md §2` record 15: "Record 15's granted item is the
    /// Glass Sword (equipment item id `39` …), and its gate is that same
    /// item's carried counter."
    #[test]
    fn record_15_grants_the_glass_sword_equipment_id() {
        let entry = record_15();
        assert_eq!(entry.pickup, HiddenTreasurePickup::Weapon);
        assert_eq!(entry.state as usize, EQUIPMENT_ID_GLASS_SWORD);
        assert_eq!(EQUIPMENT_NAMES[EQUIPMENT_ID_GLASS_SWORD], "Glass Sword");
    }

    /// `hidden-treasures.md §2` record 15: "Stages only when that counter is
    /// zero … An engine that models the gate as a separate flag the scan never
    /// sets will hand out an unlimited supply of Glass Swords."
    #[test]
    fn record_15_closes_once_the_party_carries_a_glass_sword() {
        let entry = record_15();
        let mut state = world_state(open_world_grid(), entry.x, entry.y);
        state.area = Area::World {
            plane: WorldPlane::Britannia,
        };
        state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] = 0;

        assert!(state.fixed_hidden_treasure_rule_allows(entry, entry.x, entry.y));

        state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] = 1;
        assert!(
            !state.fixed_hidden_treasure_rule_allows(entry, entry.x, entry.y),
            "a carried Glass Sword must close record 15"
        );

        // "a party that discards or loses its Glass Sword makes the record
        // available again - that is original behaviour, not a defect."
        state.equipment_stock[EQUIPMENT_ID_GLASS_SWORD] = 0;
        assert!(state.fixed_hidden_treasure_rule_allows(entry, entry.x, entry.y));
    }

    /// `hidden-treasures.md §2` record 13: "None of the three sets a
    /// found-bitmap bit … spending every key opens it again."
    #[test]
    fn record_13_sets_no_found_bit_and_reopens_when_keys_are_spent() {
        let entry = *FIXED_HIDDEN_TREASURES
            .iter()
            .find(|entry| entry.record == 13)
            .expect("record 13 is published in hidden-treasures.md §3");
        let mut state = world_state(open_world_grid(), entry.x, entry.y);
        state.keys = 0;
        assert!(state.fixed_hidden_treasure_rule_allows(entry, entry.x, entry.y));

        state.mark_fixed_hidden_treasure_found(entry);
        assert!(!state.fixed_hidden_treasure_found(entry.record));

        state.keys = 1;
        assert!(!state.fixed_hidden_treasure_rule_allows(entry, entry.x, entry.y));
        state.keys = 0;
        assert!(state.fixed_hidden_treasure_rule_allows(entry, entry.x, entry.y));

        // Even a stale bit (e.g. from an older save) does not close it.
        state.set_fixed_hidden_treasure_found(entry.record);
        assert!(state.fixed_hidden_treasure_rule_allows(entry, entry.x, entry.y));
    }
}

#[cfg(test)]
mod furniture_search_prefix_spec_tests {
    use crate::test_fixtures::{open_grid, open_world_grid, test_state, world_state};
    use crate::*;

    /// `commands.md §5.8` (u5-spec#298 Q4, `RETRACTIONS.md` R544): "Each
    /// piece is the noun, one line feed, then `t` ..., so with the shared
    /// tail `hou_dost_find\n` the location phrase and a lower-case `thou dost
    /// find` are two separate rows, after the ordinary leading line feed."
    #[test]
    fn town_search_of_a_stump_prints_the_location_row_then_lower_case_thou() {
        let mut grid = open_grid();
        grid[2 * 32 + 1] = 0x2B;
        let mut state = test_state(grid, 1, 1);
        let Area::Town { scene, floor } = state.area else {
            unreachable!()
        };
        state.search_town_secret_direction(&[], scene, floor, Direction::South);
        assert_eq!(
            state.message,
            "\nIn the stump\nthou dost find\nnothing of note.\n"
        );
    }

    #[test]
    fn every_furniture_piece_splits_into_phrase_and_thou_rows() {
        assert_eq!(
            search_preamble_for_tile(0xA1),
            "\nNear the well\nthou dost find\n"
        );
        assert_eq!(
            search_preamble_for_tile(0xAB),
            "\nUnder the bed\nthou dost find\n"
        );
        assert_eq!(
            search_preamble_for_tile(0x5A),
            "\nOn the shelf\nthou dost find\n"
        );
        // "Every other tile, the cask included, uses the two-byte default
        // prefix ... the ordinary `\nThou_dost_find\n`."
        assert_eq!(search_preamble_for_tile(0x01), SEARCH_PREAMBLE);
        assert_eq!(search_nothing_found_for_tile(0x01), SEARCH_NOTHING_FOUND);
    }

    /// "The prefix depends only on the live tile, so it applies to overworld
    /// (both planes), town-family and combat Search".
    #[test]
    fn overworld_search_of_a_stump_takes_the_prefix_too() {
        let mut grid = open_world_grid();
        let (x, y) = (10, 10);
        grid[(y + 1) * WORLD_SIDE + x] = 0x2B;
        let mut state = world_state(grid, x, y);
        state.search_world_moonstone_direction(WorldPlane::Britannia, Direction::South);
        assert_eq!(
            state.message,
            "\nIn the stump\nthou dost find\nnothing of note.\n"
        );
    }
}

#[cfg(test)]
mod staged_chest_loot_tile_spec_tests {
    use super::*;
    use crate::test_fixtures::{open_grid, test_state};

    /// `containers.md §4.1` (u5-spec#298 Q18): "both tile bytes of the new
    /// active-object record hold the family id ... weapon 5 (261), shield 6
    /// (262) ... helm 9 (265), ring 10 (266), armour 11 (267), amulet 12
    /// (268)". The engine staged every equipment row as `0x09`, the helm.
    #[test]
    fn secondary_pool_rows_stage_their_own_family_tile() {
        for (row, family) in [
            (0, 9),
            (3, 9),
            (4, 6),
            (9, 11),
            (16, 5),
            (41, 5),
            (42, 10),
            (45, 12),
        ] {
            assert_eq!(chest_secondary_pool_family_id(row), family, "row {row}");
        }
    }

    /// "The record's last byte is cleared outside combat and set to `0x20`
    /// when the scene byte is `0x80` or above"; the quantity rides in the
    /// fifth byte.
    #[test]
    fn staged_grant_carries_family_tile_and_combat_last_byte() {
        let mut state = test_state(open_grid(), 1, 1);
        assert!(state.stage_surface_chest_grant(3, 4, INVENTORY_ADD_CLASS_CODE_RING, 43));
        let staged = state
            .active_objects
            .iter()
            .rev()
            .find(|object| !object.is_empty() && object.x == 3 && object.y == 4)
            .copied()
            .expect("staged");
        assert_eq!((staged.type_byte, staged.tile), (0x0a, 0x0a));
        assert_eq!(staged.aux1, 43);
        assert_eq!(staged.aux3, 0);

        state.combat_active = true;
        assert!(state.stage_surface_chest_grant(5, 6, INVENTORY_ADD_CLASS_CODE_GOLD, 7));
        let staged = state
            .active_objects
            .iter()
            .find(|object| !object.is_empty() && object.x == 5 && object.y == 6)
            .copied()
            .expect("staged");
        assert_eq!(staged.aux3, CHEST_GRANT_COMBAT_LAST_BYTE);
    }
}

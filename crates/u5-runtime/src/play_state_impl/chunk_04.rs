use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::Path;

use crate::*;

const SURFACE_LOOK_VISIBILITY_RADIUS: usize = 5;

#[derive(Clone, Debug)]
pub struct UseItemPickerRow {
    pub label: String,
    /// `inventory.md §4.5`: the two-cell quantity; `None` is the
    /// "no quantity" marker that prints only the name.
    pub quantity: Option<u8>,
    /// `inventory.md §4.5` (`RETRACTIONS.md` R403): which presentation family
    /// decorates this row's name. "The two markers decorate all eight scrolls
    /// and all eight potions, respectively"; the Sceptre, Skull Keys and the
    /// other named artifacts carry none.
    pub decoration: crate::stats_panel::PanelPickerDecoration,
    pub(crate) request: UseItemRequest,
}

/// `commands.md §5` + observation: the U-Use handler's message-window
/// prompt, printed on the line after the `Use item` verb echo.
pub const USE_ITEM_PROMPT_MESSAGE: &str = ITEM_SELECTION_PROMPT;

/// `shops.md §2`: "If the party's transport marker is either of the two horse
/// values, the dispatcher prints a fixed two-line refusal and returns without
/// entering any shop". The two lines are published verbatim; the horse-trader
/// trigger (`0x83`) is exempt. This is a fixed merchant refusal, not a
/// per-shop-role line.
pub const SHOP_MOUNTED_REFUSAL: &str = "A merchant says:\n\"GET THAT HORSE OUT OF HERE!\"";

/// `shops.md §2`: the fixed refusal a shop trigger prints when its keeper is
/// not open for business, "with these line breaks" and "a newline before the
/// preamble and after the closing quote".
///
/// Measured 2026-09-07 (`qa/paired/nb-stable.tsv`).
pub const SHOP_CLOSED_REFUSAL: &str =
    "\nA merchant says:\n\"Come see me at\nmy shoppe, when\nit's open!\"\n";

/// `shops.md §8.0`: the **vendor** column of the two resident name tables that
/// are indexed by the shop-instance row. The row is the index of the active
/// scene byte inside the shop kind's own scene list, so a scene-byte lookup
/// per kind resolves the same row the spec's tables are keyed by.
///
/// "Two resident name tables are indexed by the same row ...: the shop's
/// display name, which fills the `#` substitution, and the vendor's name,
/// which fills the `$` substitution and the `says <shopkeeper>.` /
/// `yells <shopkeeper>.` attribution tails. ... Neither name is read from the
/// NPC roster or the conversation blob, so the shopkeeper an implementation
/// names in shop text is a property of the location, not of the NPC the player
/// happened to talk to."
///
/// The shipped horse-trader table holds a fourth row for scene `30` whose
/// vendor is `Simplon`; the spec records that no `0x83` trigger exists for
/// scene `30`, so the row is unreachable and is deliberately not listed here.
/// Returns `None` when the kind's table does not list the active scene — the
/// spec's error case, where a clean implementation "should reject the trigger
/// and leave the conversation alone".
pub const fn shop_vendor_name_for_scene(dialog_id: u8, scene_byte: u8) -> Option<&'static str> {
    let table: &[(u8, &'static str)] = match dialog_id {
        // Arms shops (9 rows).
        0x81 => &[
            (2, "Gwenneth"),
            (3, "Nomaan"),
            (4, "Ronan"),
            (5, "Shenstone"),
            (6, "Paul"),
            (17, "Max"),
            (24, "Kitiara"),
            (26, "Steve"),
            (32, "Thol"),
        ],
        // Taverns / meal counters (9 rows).
        0x82 => &[
            (1, "Sam"),
            (2, "Tika"),
            (3, "Nicole"),
            (4, "Duclas"),
            (8, "Felicity"),
            (19, "Jaymes"),
            (22, "Dr. Cat"),
            (24, "Nikki"),
            (30, "Rob"),
        ],
        // Horse traders (3 reachable rows).
        0x83 => &[(6, "Hettar"), (20, "Theoan"), (22, "Ferru")],
        // Shipwrights (4 rows).
        0x84 => &[
            (3, "Bantral"),
            (5, "Captain Blyth"),
            (21, "Master Hawkins"),
            (24, "Jones"),
        ],
        // Reagent vendors (5 rows).
        0x85 => &[
            (1, "Nilrem"),
            (4, "Madam Pendra"),
            (7, "Toama"),
            (23, "Enlor"),
            (30, "Virden"),
        ],
        // Guildmasters (3 rows).
        0x86 => &[(8, "Braunam"), (22, "Danfits"), (24, "Daem")],
        // Healers / sanctums (7 rows).
        0x87 => &[
            (5, "Regina"),
            (6, "Leila"),
            (7, "Temptious"),
            (21, "Milan"),
            (23, "Jessica"),
            (30, "Faye"),
            (31, "Jessip"),
        ],
        // Inns (6 rows).
        0x88 => &[
            (2, "Donya"),
            (3, "Gremnor"),
            (7, "Rogi"),
            (20, "Terbor"),
            (22, "Lorien"),
            (24, "Ransack"),
        ],
        _ => return None,
    };
    let mut index = 0;
    while index < table.len() {
        if table[index].0 == scene_byte {
            return Some(table[index].1);
        }
        index += 1;
    }
    None
}

impl PlayState {
    /// Apply the primary tavern round's north-first table-setting rewrite.
    /// At either vertical edge the out-of-range lookup uses the town-grid
    /// southeast fallback cell, matching the shared location-grid accessor.
    pub fn rewrite_tavern_round_table_setting(&mut self) -> bool {
        let north_index = self
            .player
            .y
            .checked_sub(1)
            .map(|y| y * TOWN_GRID_SIDE + self.player.x)
            .unwrap_or(TOWN_GRID_BYTES - 1);
        if self.grid[north_index] == TAVERN_BARE_TABLE_SETTING_TILE {
            self.grid[north_index] = TAVERN_NORTH_FOOD_SETTING_TILE;
            self.mark_visibility_dirty();
            return true;
        }

        let south_index = self
            .player
            .y
            .checked_add(1)
            .filter(|y| *y < TOWN_GRID_SIDE)
            .map(|y| y * TOWN_GRID_SIDE + self.player.x)
            .unwrap_or(TOWN_GRID_BYTES - 1);
        if self.grid[south_index] == TAVERN_BARE_TABLE_SETTING_TILE {
            self.grid[south_index] = TAVERN_SOUTH_FOOD_SETTING_TILE;
            self.mark_visibility_dirty();
            return true;
        }

        false
    }

    pub fn cast_rel_hur(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
        pass: bool,
    ) -> MoveOutcome {
        if !matches!(self.area, Area::World { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if direction.is_none() && !pass {
            self.request_cast_argument(CastArgumentRequest::Direction);
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, REL_HUR_SPELL_INDEX, REL_HUR_COST)
        {
            return outcome;
        }

        if pass {
            self.advance_turn();
            // Measured 2026-09-07: Space completes the scroll's own
            // `Direction-` row with the shared `Pass` word and prints
            // nothing else.
            let _ = self.complete_open_direction_echo(
                SPELL_DIRECTION_PROMPT_PREFIX,
                DIRECTION_PROMPT_LABEL_PASS,
            );
            self.message.clear();
            return MoveOutcome::Cast;
        }

        let previous = self.wind.status_message();
        let next = WindState::rel_hur_target(direction.expect("direction checked above"))
            .expect("inline Rel Hur parser returns cardinal directions only");
        self.apply_wind_state(next);
        self.advance_turn();
        self.message = format!(
            "Wind change! {} -> {}.",
            previous,
            self.wind_status_message()
        );
        MoveOutcome::Cast
    }

    /// `audio.md §7.3` accepted wind change cast as the **spell** `Rel Hur`:
    /// play variant 2, then commit and announce the new wind.
    pub fn apply_wind_state(&mut self, wind: WindState) -> bool {
        self.apply_wind_state_from_caller(wind, Some(audio::WindChangeCaller::Spell))
    }

    /// `audio.md §7.3` accepted wind change reached through the **scroll**
    /// (scroll index 1): the same sequence at variant 1.
    ///
    /// "The scroll variant disagrees with the corresponding spell's variant in
    /// six of the eight cases: ... Wind Change 1 against 2."
    pub fn apply_wind_state_from_scroll(&mut self, wind: WindState) -> bool {
        self.apply_wind_state_from_caller(wind, Some(audio::WindChangeCaller::Scroll))
    }

    /// Commit a wind transition **without** the `audio.md §7.3` cue.
    ///
    /// `§7.3`: "The sound belongs to the spell and scroll handlers, never to
    /// the setter"; the autonomous drift "has no sound call, no wrapper, and
    /// no ambient hook", and `§11` lists it in the wind-change sequence's
    /// explicitly-not-produced-by column. The drift therefore commits through
    /// this entry point.
    pub fn apply_wind_state_without_sound(&mut self, wind: WindState) -> bool {
        self.apply_wind_state_from_caller(wind, None)
    }

    fn apply_wind_state_from_caller(
        &mut self,
        wind: WindState,
        caller: Option<audio::WindChangeCaller>,
    ) -> bool {
        if self.wind == WindState::Calm && wind == WindState::Calm {
            return false;
        }
        // `audio.md §7.3`: "**The variant is chosen by the caller tag, not by
        // the wind.**" The old wind and the requested compass direction do not
        // participate, so requesting the already-active direction still sounds.
        // The earlier previous-wind matrix is withdrawn (`RETRACTIONS.md`).
        let variant =
            caller.and_then(|caller| audio::wind_change_variant(caller, wind == WindState::Calm));
        if let Some(variant) = variant {
            self.emit_sound_effect(SoundEffect::SharedVariant { variant });
        }
        let changed = self.wind != wind;
        self.wind = wind;
        self.wind_save_byte = wind.save_byte();
        // `weather.md §5.1`, "**A fourth reset of the sailing counter.**":
        // "The routine that stores a new wind state clears the counter
        // immediately afterwards, and both are skipped when the same routine
        // is called only to re-announce the wind already in force. So a ship
        // one pass into a two-pass upwind wait restarts that wait when the
        // wind shifts."
        if changed {
            self.sail_cadence = 0;
        }
        changed
    }

    pub fn wind_status_message(&self) -> &'static str {
        wind_status_message_from_state_and_save_byte(self.wind, self.wind_save_byte)
    }

    pub fn cast_gate_travel(
        &mut self,
        caster_index: usize,
        slot_index: usize,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, GATE_TRAVEL_SPELL_INDEX, GATE_TRAVEL_COST)
        {
            return Ok(outcome);
        }

        // `magic.md §5` steps 4-7: the dispatcher spends the premixed charge
        // and debits mana *before* it computes the dispatch index and calls
        // the effect handler, and `magic.md §8` puts the shipboard test inside
        // the handler — "Gate Travel ... requires the party not to be
        // shipboard, prompts `To phase:`". So a shipboard attempt is a
        // committed cast: the charge and the eight magic points are already
        // gone and nothing is refunded, exactly as with the Doom refusal in
        // `cast_dungeon_level_spell`. Only the scene gate (inside
        // `cast_spell_resource_gate`) spends nothing.
        //
        // `magic.md §5` step 8: "ordinary failure appends `Failed!` and a
        // newline with the failure sound". The shipboard test is *inside* the
        // handler - §8 "requires the party not to be shipboard" - so it is an
        // ordinary handler failure and the dispatcher's own epilogue narrates
        // it. The engine-voice sentence that used to stand here was invented.
        if matches!(self.player.transport, TransportState::Ship { .. }) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            self.emit_sound_effect(SoundEffect::CastFailure);
            return Ok(MoveOutcome::Blocked);
        }

        // `audio.md §6.1` id 46: the variant (circle 8) plays "only after the
        // player types a digit `1`..`8` at the moongate prompt; any other key
        // is silent". The shipboard refusal above never reaches that prompt.
        if let Some(variant) = audio::spell_shared_variant(GATE_TRAVEL_SPELL_INDEX) {
            self.emit_sound_effect(SoundEffect::SharedVariant { variant });
        }

        let phase = slot_index + 1;
        let slot = self.moonstone_slots[slot_index];
        self.advance_turn();
        match gate_travel_destination(slot) {
            GateTravelDestination::Ready {
                target,
                floor,
                start,
            } => {
                self.apply_gate_travel(game_dir, phase, target, floor, start)?;
                Ok(MoveOutcome::Transition(AreaTransition::GateTraveled {
                    target,
                }))
            }
            // `magic.md §8`: "an invalid scene sentinel makes the helper
            // return failure and the cast does not teleport" - and §5 step 8
            // turns a handler failure into `Failed!` with the failure sound.
            // Neither the empty slot nor the invalid one has a line of its
            // own; the two sentences that used to stand here named an
            // internal slot number the original never shows.
            GateTravelDestination::Empty | GateTravelDestination::Invalid(_) => {
                let _ = phase;
                self.message = "Failed!".to_string();
                self.emit_sound_effect(SoundEffect::CastFailure);
                Ok(MoveOutcome::Blocked)
            }
        }
    }

    pub fn use_item_command(
        &mut self,
        request: Option<UseItemRequest>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        Ok(match request {
            Some(UseItemRequest::WoodenBox) => self.use_wooden_box(),
            Some(UseItemRequest::HmsCapePlans) => self.use_hms_cape_plans(),
            Some(UseItemRequest::CrownOfLordBritish) => self.use_worn_regalia(
                SPECIAL_ITEM_CROWN_LB_INDEX,
                CROWN_LB_ACTIVE_EFFECT_TAG,
                "Crown",
                USE_CROWN_WORN,
                USE_REGALIA_REMOVED,
            ),
            Some(UseItemRequest::AmuletOfLordBritish) => self.use_worn_regalia(
                SPECIAL_ITEM_AMULET_LB_INDEX,
                AMULET_LB_ACTIVE_EFFECT_TAG,
                "Amulet",
                USE_AMULET_WORN,
                USE_REGALIA_REMOVED,
            ),
            Some(UseItemRequest::Sceptre) => self.use_sceptre_of_lord_british(),
            Some(UseItemRequest::BlackBadge) => self.use_worn_regalia(
                SPECIAL_ITEM_BLACK_BADGE_INDEX,
                BLACK_BADGE_ACTIVE_EFFECT_TAG,
                "Black Badge",
                USE_BLACK_BADGE_WORN,
                USE_REGALIA_REMOVED,
            ),
            Some(UseItemRequest::Spyglass) => self.use_spyglass(),
            Some(UseItemRequest::Scroll {
                index,
                direction,
                target,
            }) => self.use_scroll(index, direction, target),
            Some(UseItemRequest::Potion { index, target }) => self.use_potion(index, target),
            Some(UseItemRequest::MagicCarpet) => self.use_magic_carpet(),
            Some(UseItemRequest::SkullKey) => self.use_skull_key(game_dir)?,
            Some(UseItemRequest::Sextant) => self.use_sextant(),
            Some(UseItemRequest::PocketWatch) => self.use_pocket_watch(),
            Some(UseItemRequest::ShadowlordShard(index)) => {
                self.use_shadowlord_shard(index, game_dir)?
            }
            Some(UseItemRequest::Moonstone(slot_index)) => {
                self.use_moonstone_phase(Some(slot_index))
            }
            Some(UseItemRequest::Invalid) | None => {
                self.message = use_prompt_message();
                MoveOutcome::Blocked
            }
        })
    }

    pub fn start_use_item(&mut self) -> MoveOutcome {
        let rows = self.use_item_picker_rows();
        if rows.is_empty() {
            self.message = USE_NO_USABLE_ITEMS_REFUSAL.to_string();
            // inventory.md §7: U-Use returns the normal action result even
            // when item-specific dispatch is never reached.
            self.advance_turn();
            return MoveOutcome::Blocked;
        }
        self.active_use = Some(UseSession::new());
        // `text-output.md §10.4`: the prompt follows the `Use item` echo,
        // so its leading feed buys a blank row. A capture reads
        // `>Use item`, a blank row, then `Item: None!` on one row.
        // `cleak/u5-engine#5`.
        self.open_prompt_block();
        self.message = self.render_active_use();
        MoveOutcome::Observed
    }

    pub fn render_active_use(&self) -> String {
        self.active_use
            .as_ref()
            .map(|session| self.render_use_session(session))
            .unwrap_or_else(use_prompt_message)
    }

    pub fn render_use_session(&self, session: &UseSession) -> String {
        if let Some(pending) = session.pending {
            return self.render_pending_use_action(pending);
        }
        if self.use_item_picker_rows().is_empty() {
            return USE_NO_USABLE_ITEMS_REFUSAL.to_string();
        }
        // `inventory.md §4.4`: U-Use prints `Item:_` into the message
        // window and runs the picker in the panel; the rows are read from
        // `crate::active_panel_picker` by the panel painter.
        USE_ITEM_PROMPT_MESSAGE.to_string()
    }

    pub(crate) fn use_picker_cursor(&self, session: &UseSession) -> usize {
        session
            .cursor
            .min(self.use_item_picker_rows().len().saturating_sub(1))
    }

    pub fn step_active_use(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<bool> {
        let Some(mut session) = self.active_use.take() else {
            return Ok(false);
        };
        let key = ready_first_input_key(key, suffix);
        if let Some(pending) = session.pending {
            return self.step_pending_use_action(session, pending, key, suffix, game_dir);
        }
        match use_input_action(key) {
            UseInputAction::Exit => {
                let turn_before = self.turn;
                // The reply lands on the prompt's own row: `Item: None!`.
                self.commit_prompt_reply(ITEM_SELECTION_PROMPT, ITEM_PICKER_ESCAPE_MESSAGE);
                self.ensure_use_action_turn(turn_before);
                self.apply_post_turn_effects_after_outcome(
                    turn_before,
                    game_dir,
                    MoveOutcome::Used,
                )?;
            }
            UseInputAction::NextItem => {
                self.move_use_cursor(&mut session, 1);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            UseInputAction::PreviousItem => {
                self.move_use_cursor(&mut session, -1);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            UseInputAction::PageNext => {
                self.move_use_cursor(&mut session, READY_PICKER_PAGE_STEP as isize);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            UseInputAction::PagePrevious => {
                self.move_use_cursor(&mut session, -(READY_PICKER_PAGE_STEP as isize));
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            // `inventory.md §4.7`: "Home selects the first item; End selects
            // the last." The shared picker's rules are §5's, so the U-Use
            // list takes them too.
            UseInputAction::FirstItem => {
                self.select_use_endpoint(&mut session, false);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            UseInputAction::LastItem => {
                self.select_use_endpoint(&mut session, true);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            UseInputAction::Confirm => {
                let turn_before = self.turn;
                let Some(row) = self.use_selected_item(&session) else {
                    self.message = USE_NO_USABLE_ITEMS_REFUSAL.to_string();
                    self.ensure_use_action_turn(turn_before);
                    self.apply_post_turn_effects_after_outcome(
                        turn_before,
                        game_dir,
                        MoveOutcome::Blocked,
                    )?;
                    return Ok(true);
                };
                // The accepted row completes the open `Item: ` line before
                // the handler runs; see [`Self::use_item_echo_family`].
                let echoed = Self::use_item_echo_family(row.request).inspect(|family| {
                    self.commit_prompt_reply(ITEM_SELECTION_PROMPT, family);
                });
                if let Some(pending) = pending_action_for_use_request(row.request) {
                    // The potion and scroll rows debit their stock when the
                    // row is accepted, before the argument is asked for, and
                    // `inventory.md §7` puts the skull key in the same place:
                    // it "**Decrements the skull-key/special-key counter,
                    // then** asks for a cardinal target". Its lock attempt is
                    // what waits for the direction, not its cost.
                    if matches!(pending, UsePendingAction::SkullKeyDirection) {
                        // Except in a dungeon, where §7 refuses instead and
                        // the key is kept.
                        if !matches!(self.area, Area::Dungeon { .. }) {
                            self.spend_skull_key();
                        }
                    } else {
                        let _ = self.use_item_command(Some(row.request), Some(game_dir))?;
                    }
                    session.pending = Some(pending);
                    // Measured: a scroll announces its effect on a row of
                    // its own, under a blank one, and *then* asks for its
                    // argument. A potion asks straight away.
                    self.message = match Self::use_pending_action_lead_line(pending) {
                        Some(lead) => {
                            format!("\n{lead}\n{}", self.render_pending_use_action(pending))
                        }
                        None => self.render_use_session(&session),
                    };
                    self.active_use = Some(session);
                    return Ok(true);
                }
                let outcome = self.use_item_command(Some(row.request), Some(game_dir))?;
                // `text-output.md §10.4`: the handler's own line prints
                // under a completed row, so its leading feed buys a blank
                // row. Measured: `>Use item`, blank, `Item: Scroll`,
                // blank, `Protection!`.
                // Measured: a line that opens with a space *continues* the
                // completed `Item: ` row instead of opening a new one -
                // `Item: Moonstone cannot be buried here!` is one wrapped
                // sentence, where the pocket watch's line starts its own row
                // under a blank one.
                // A handler that recorded its own rows - the sextant writes a
                // runic one - has nothing left in the slot to place, and
                // touching it here would flush a second, font-flattened copy.
                if !self.message_slot_needs_flush() {
                } else if echoed.is_some() && self.message.starts_with(' ') {
                    // The continuation joins the completed `Item: ` row, so
                    // the window wraps the whole sentence together:
                    // `Item: Moonstone` / `cannot be buried` / `here!`.
                    let continuation = std::mem::take(&mut self.message);
                    self.emit_message_line_continuing_row(continuation);
                } else if echoed.is_some()
                    && !self.message.is_empty()
                    && !self.message.starts_with('\n')
                {
                    self.message.insert(0, '\n');
                }
                self.ensure_use_action_turn(turn_before);
                self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
            }
            UseInputAction::Redraw | UseInputAction::Discard => {
                self.normalize_use_cursor(&mut session);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
        }
        Ok(true)
    }

    fn step_pending_use_action(
        &mut self,
        mut session: UseSession,
        pending: UsePendingAction,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<bool> {
        // Measured 2026-09-07: Escape completes the open prompt row with the
        // universal `None!` - `On who: None!` - and Space *accepts* the
        // highlighted member rather than closing, exactly as it does in the
        // item picker itself. A direction prompt takes the shared `Pass`
        // word instead (the wind scroll reads `Direction-Pass`).
        if key == '\u{1b}' {
            let turn_before = self.turn;
            let prompt = self.render_pending_use_action(pending);
            self.commit_prompt_reply(&prompt, SELECTION_CANCELLED_LITERAL);
            self.ensure_use_action_turn(turn_before);
            self.apply_post_turn_effects_after_outcome(turn_before, game_dir, MoveOutcome::Used)?;
            return Ok(true);
        }
        if key == ' '
            && matches!(
                pending,
                UsePendingAction::SkullKeyDirection | UsePendingAction::ScrollWindDirection { .. }
            )
        {
            let turn_before = self.turn;
            self.commit_prompt_reply(SPELL_DIRECTION_PROMPT_PREFIX, DIRECTION_PROMPT_LABEL_PASS);
            self.ensure_use_action_turn(turn_before);
            self.apply_post_turn_effects_after_outcome(turn_before, game_dir, MoveOutcome::Used)?;
            return Ok(true);
        }

        let turn_before = self.turn;
        let outcome = match pending {
            UsePendingAction::SkullKeyDirection => {
                if let Some(direction) = pending_use_cardinal_direction(key, suffix) {
                    // Measured: the chosen cardinal completes the open
                    // `Direction-` row, then the key is tried on that cell.
                    self.commit_prompt_reply(SPELL_DIRECTION_PROMPT_PREFIX, direction.name());
                    self.use_skull_key_direction(Some(game_dir), direction)?
                } else {
                    session.pending = Some(pending);
                    self.message = self.render_use_session(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
            UsePendingAction::PotionTarget { index, highlight } => {
                match self.step_use_target_selector(key, suffix, highlight) {
                    UseTargetStep::Moved(next) => {
                        session.pending = Some(UsePendingAction::PotionTarget {
                            index,
                            highlight: next,
                        });
                        self.message = self.render_use_session(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Waiting => {
                        session.pending = Some(pending);
                        self.message = self.render_use_session(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Committed(target) => {
                        // The answer completes the prompt's own row.
                        let name = self.party_member_display_name(target);
                        self.commit_prompt_reply(USE_POTION_TARGET_PROMPT, &name);
                        self.use_potion_consumed_target(index, target)
                    }
                }
            }
            UsePendingAction::ScrollWindDirection { .. } => {
                if let Some(direction) = pending_use_cardinal_direction(key, suffix) {
                    // The cardinal completes the open `Direction-` row.
                    self.commit_prompt_reply(SPELL_DIRECTION_PROMPT_PREFIX, direction.name());
                    self.use_wind_change_scroll(Some(direction))
                } else {
                    session.pending = Some(pending);
                    self.message = self.render_use_session(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
            UsePendingAction::ScrollResurrectionTarget { index, highlight } => {
                match self.step_use_target_selector(key, suffix, highlight) {
                    UseTargetStep::Moved(next) => {
                        session.pending = Some(UsePendingAction::ScrollResurrectionTarget {
                            index,
                            highlight: next,
                        });
                        self.message = self.render_use_session(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Waiting => {
                        session.pending = Some(pending);
                        self.message = self.render_use_session(&session);
                        self.active_use = Some(session);
                        return Ok(true);
                    }
                    UseTargetStep::Committed(target) => {
                        let name = self.party_member_display_name(target);
                        self.commit_prompt_reply(USE_POTION_TARGET_PROMPT, &name);
                        self.use_resurrection_scroll_consumed_target(index, target)
                    }
                }
            }
        };
        self.ensure_use_action_turn(turn_before);
        self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        Ok(true)
    }

    /// `inventory.md §7`: the U-Use caller returns the normal action result
    /// regardless of the selected handler's success/refusal result. Existing
    /// handlers that already commit their effect's turn remain authoritative;
    /// otherwise the command layer supplies the one ordinary turn here.
    pub(crate) fn ensure_use_action_turn(&mut self, turn_before: u64) {
        if self.turn == turn_before {
            self.advance_turn();
        }
    }

    /// The family word an accepted U-Use row writes onto the `Item: `
    /// line. See [`USE_ITEM_ECHO_SCROLL`]: measured for scrolls and
    /// potions only, so every other item completes nothing.
    fn use_item_echo_family(request: UseItemRequest) -> Option<&'static str> {
        match request {
            UseItemRequest::Scroll { .. } => Some(USE_ITEM_ECHO_SCROLL),
            UseItemRequest::Potion { .. } => Some(USE_ITEM_ECHO_POTION),
            UseItemRequest::SkullKey => Some(USE_ITEM_ECHO_SKULL_KEY),
            UseItemRequest::MagicCarpet => Some(USE_ITEM_ECHO_CARPET),
            UseItemRequest::AmuletOfLordBritish => Some(USE_ITEM_ECHO_AMULET),
            UseItemRequest::CrownOfLordBritish => Some(USE_ITEM_ECHO_CROWN),
            UseItemRequest::Sceptre => Some(USE_ITEM_ECHO_SCEPTRE),
            UseItemRequest::ShadowlordShard(_) => Some(USE_ITEM_ECHO_GEM_SHARD),
            UseItemRequest::Spyglass => Some(USE_ITEM_ECHO_SPYGLASS),
            UseItemRequest::HmsCapePlans => Some(USE_ITEM_ECHO_PLANS),
            UseItemRequest::Sextant => Some(USE_ITEM_ECHO_SEXTANT),
            UseItemRequest::PocketWatch => Some(USE_ITEM_ECHO_WATCH),
            UseItemRequest::BlackBadge => Some(USE_ITEM_ECHO_BADGE),
            UseItemRequest::WoodenBox => Some(USE_ITEM_ECHO_BOX),
            UseItemRequest::Moonstone(_) => Some(USE_ITEM_ECHO_MOONSTONE),
            _ => None,
        }
    }

    /// Measured: the two scroll prompts announce the scroll's effect before
    /// they ask for their argument - `Wind change!` then `Direction-`, and
    /// `Resurrection!` then `On who: `. The potion prompt has no such line.
    fn use_pending_action_lead_line(pending: UsePendingAction) -> Option<&'static str> {
        match pending {
            UsePendingAction::PotionTarget { .. } => None,
            UsePendingAction::ScrollWindDirection { .. } => Some(SCROLL_WIND_CHANGE_RESULT),
            UsePendingAction::ScrollResurrectionTarget { .. } => Some(SCROLL_RESURRECTION_RESULT),
            UsePendingAction::SkullKeyDirection => None,
        }
    }

    fn render_pending_use_action(&self, pending: UsePendingAction) -> String {
        match pending {
            // Measured: the potion's target prompt is
            // [`USE_POTION_TARGET_PROMPT`], and the chosen member's name
            // completes its row. The two scroll prompts below are not
            // measured and keep the harness text for now
            // (`cleak/u5-spec#225`).
            UsePendingAction::PotionTarget { index, .. } => {
                let _ = index;
                USE_POTION_TARGET_PROMPT.to_string()
            }
            UsePendingAction::ScrollWindDirection { index } => {
                let _ = index;
                SPELL_DIRECTION_PROMPT_PREFIX.to_string()
            }
            UsePendingAction::ScrollResurrectionTarget { index, .. } => {
                let _ = index;
                USE_POTION_TARGET_PROMPT.to_string()
            }
            UsePendingAction::SkullKeyDirection => SPELL_DIRECTION_PROMPT_PREFIX.to_string(),
        }
    }

    fn use_potion_consumed_target(&mut self, index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            // Out-of-range member: the interactive pickers bound the
            // index, so only the inline harness form reaches this. The
            // original has no line for a state it cannot enter.
            self.message.clear();
            return MoveOutcome::Blocked;
        }

        let variation_roll = self.potion_variation_roll(index, target_index);
        let random_roll = self.potion_random_effect_roll(index, target_index);
        let effect_index = potion_effect_index_after_variation(index, variation_roll, random_roll);
        self.use_potion_with_effect(index, target_index, effect_index)
    }

    fn use_resurrection_scroll_consumed_target(
        &mut self,
        index: usize,
        target_index: usize,
    ) -> MoveOutcome {
        if index != SCROLL_RESURRECTION_INDEX {
            self.message = "No effect!".to_string();
            return MoveOutcome::Blocked;
        }
        self.use_resurrection_scroll(Some(target_index))
    }

    fn use_selected_item(&self, session: &UseSession) -> Option<UseItemPickerRow> {
        let rows = self.use_item_picker_rows();
        rows.get(session.cursor.min(rows.len().saturating_sub(1)))
            .cloned()
    }

    fn normalize_use_cursor(&self, session: &mut UseSession) {
        let row_count = self.use_item_picker_rows().len();
        if row_count == 0 {
            session.cursor = 0;
        } else if session.cursor >= row_count {
            session.cursor = row_count - 1;
        }
    }

    fn move_use_cursor(&self, session: &mut UseSession, delta: isize) {
        let row_count = self.use_item_picker_rows().len();
        if row_count == 0 {
            session.cursor = 0;
            return;
        }
        let next = session.cursor as isize + delta;
        session.cursor = next.clamp(0, row_count as isize - 1) as usize;
    }

    /// `inventory.md §4.7`: Home and End select the first and last rows
    /// rather than paging toward them.
    fn select_use_endpoint(&self, session: &mut UseSession, last: bool) {
        let row_count = self.use_item_picker_rows().len();
        session.cursor = if row_count == 0 {
            0
        } else if last {
            row_count - 1
        } else {
            0
        };
    }

    /// Rows of the U-Use item picker, in the order the original lists
    /// them.
    ///
    /// `inventory.md §7` says only that "a selected row dispatches by the
    /// handler's use-item enumeration"; the enumeration's order is not
    /// published. Two independent clean sources agree on it, so it is
    /// reproduced here rather than invented:
    ///
    /// - the order of §7's own "Confirmed U-Use families" table - spell
    ///   scrolls, potions, Magic Carpet, Skull Key, regalia, shards,
    ///   Moonstones, Spyglass, HMS Cape plans, Sextant, Pocket Watch,
    ///   Sandalwood Box; and
    /// - the published special-item indices, which run in that same
    ///   sequence (carpet `0x00`, skull key `0x01`, amulet `0x03`, crown
    ///   `0x04`, sceptre `0x05`, shards `0x06..=0x08`, spyglass `0x0A`,
    ///   plans `0x0B`, sextant `0x0C`, watch `0x0D`, badge `0x0E`, box
    ///   `0x0F`).
    ///
    /// A paired capture of the original's picker confirms the one point
    /// both sources constrain and the engine used to get wrong: a carried
    /// scroll and potion are listed *above* the Pocket Watch, not below
    /// it. The engine previously emitted every special item first.
    /// `cleak/u5-spec#196` asks for the enumeration to be published
    /// outright.
    /// The rows the U-Use picker offers, in order. `pub` so the
    /// `use_rows` probe can print the row indexes a paired scenario has to
    /// count keypresses to reach.
    pub fn use_item_picker_rows(&self) -> Vec<UseItemPickerRow> {
        let mut rows = Vec::new();

        for (index, count) in self.scroll_stock.iter().copied().enumerate() {
            if count > 0 {
                rows.push(UseItemPickerRow {
                    // §4.5's scroll row is the marker, the plus and "the
                    // scroll's compact rune label" - not the word `Scroll`,
                    // which the decoration's own glyph stands for.
                    label: scroll_label(index).to_string(),
                    quantity: Some(count),
                    decoration: crate::stats_panel::PanelPickerDecoration::Scroll,
                    request: UseItemRequest::Scroll {
                        index,
                        direction: None,
                        target: None,
                    },
                });
            }
        }

        for (index, count) in self.potion_stock.iter().copied().enumerate() {
            if count > 0 {
                rows.push(UseItemPickerRow {
                    label: crate::z_stats::potion_short_colour_name(index).to_string(),
                    quantity: Some(count),
                    decoration: crate::stats_panel::PanelPickerDecoration::Potion,
                    request: UseItemRequest::Potion {
                        index,
                        target: None,
                    },
                });
            }
        }

        self.push_counted_use_row(
            &mut rows,
            SPECIAL_ITEM_MAGIC_CARPET_INDEX,
            // Measured (`qa/paired/use-specials.tsv`, beat `skullkeysrow`):
            // the original's row reads ` 1 Magic Crpt`. The name field is ten
            // cells, and the original abbreviates rather than truncating -
            // truncation of `Magic Carpet` gives `Magic Carp`, which is what
            // this engine printed.
            "Magic Crpt",
            UseItemRequest::MagicCarpet,
        );

        self.push_counted_use_row(
            &mut rows,
            SPECIAL_ITEM_SKULL_KEY_INDEX,
            "Skull Keys",
            UseItemRequest::SkullKey,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_AMULET_LB_INDEX,
            // **Measured** 2026-09-08 (`qa/paired/use-picker.tsv`): the
            // original's `Items:` panel lists these three by their short
            // names - ` 1 Amulet`, ` 1 Crown`, ` 1 Sceptre` - in a
            // thirteen-column field. The engine's long forms would render
            // truncated, which is not the same text.
            "Amulet",
            UseItemRequest::AmuletOfLordBritish,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_CROWN_LB_INDEX,
            "Crown",
            UseItemRequest::CrownOfLordBritish,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SCEPTRE_LB_INDEX,
            "Sceptre",
            UseItemRequest::Sceptre,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
            // `inventory.md §4.5`'s complete 38-entry label table, published
            // 2026-09-10 in answer to cleak/u5-spec#255: the shards are
            // `Shard/Falsehd`, `Shard/Hatred` and `Shard/Cowrdce`, and the
            // section adds "Use these labels verbatim, including
            // abbreviations and the singular `Plan`". The row renderer
            // "prints each authored label and then pads short rows; it does
            // not abbreviate or truncate a longer name at runtime", so the
            // engine's long forms were not merely rendered short - they were
            // the wrong strings.
            "Shard/Falsehd",
            UseItemRequest::ShadowlordShard(SHADOWLORD_FALSEHOOD_INDEX),
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_HATRED_INDEX,
            "Shard/Hatred",
            UseItemRequest::ShadowlordShard(SHADOWLORD_HATRED_INDEX),
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
            "Shard/Cowrdce",
            UseItemRequest::ShadowlordShard(SHADOWLORD_COWARDICE_INDEX),
        );

        if self.current_moonstone_bury_context().is_some() {
            for index in 0..MOONSTONE_SLOT_COUNT {
                // `inventory.md §7`: the handler "opens an item picker
                // over usable carried stock", and the Moonstone row
                // "records the current valid location into the matching
                // saved Moonstone slot", while "Search/Get recovery later
                // invalidates the slot". A slot that still holds a valid
                // location is therefore a stone lying buried in the world,
                // not one in the pack, so only an invalidated slot has a
                // stone to bury. A capture of the original's U-Use picker
                // at game start - every stone still buried - shows no
                // Moonstone row at all, which is what this gate
                // reproduces; the engine used to offer all eight.
                if self.moonstone_slots[index].is_valid() {
                    continue;
                }
                rows.push(UseItemPickerRow {
                    // §4.5: "`Moonstone` followed by a space in the text
                    // font, then one runic phase glyph" - the phase is the
                    // glyph, not a spelled-out number.
                    label: Z_STATS_MOONSTONE_LABEL.to_string(),
                    quantity: None,
                    decoration: crate::stats_panel::PanelPickerDecoration::Moonstone {
                        phase: index as u8,
                    },
                    request: UseItemRequest::Moonstone(index),
                });
            }
        }

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SPYGLASS_INDEX,
            "Spyglass",
            UseItemRequest::Spyglass,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX,
            "HMS Cape Plan",
            UseItemRequest::HmsCapePlans,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SEXTANT_INDEX,
            "Sextant",
            UseItemRequest::Sextant,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_POCKET_WATCH_INDEX,
            "Pocket Watch",
            UseItemRequest::PocketWatch,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_BLACK_BADGE_INDEX,
            "Black Badge",
            UseItemRequest::BlackBadge,
        );

        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_WOODEN_BOX_INDEX,
            "Wooden Box",
            UseItemRequest::WoodenBox,
        );

        rows
    }

    fn push_counted_use_row(
        &self,
        rows: &mut Vec<UseItemPickerRow>,
        special_item_index: usize,
        label: &str,
        request: UseItemRequest,
    ) {
        let count = self.special_items[special_item_index];
        if count > 0 {
            rows.push(UseItemPickerRow {
                label: label.to_string(),
                decoration: crate::stats_panel::PanelPickerDecoration::None,
                quantity: Some(count),
                request,
            });
        }
    }

    fn push_owned_use_row(
        &self,
        rows: &mut Vec<UseItemPickerRow>,
        special_item_index: usize,
        label: &str,
        request: UseItemRequest,
    ) {
        let value = self.special_items[special_item_index];
        if value > 0 {
            rows.push(UseItemPickerRow {
                label: label.to_string(),
                decoration: crate::stats_panel::PanelPickerDecoration::None,
                // `inventory.md §4.5` "Which U-Use rows omit quantity":
                // quantity suppression is independent of name decoration,
                // and it is driven by the *saved value*, not by the item's
                // identity. "In the picker stock, value 255 is the
                // no-quantity marker; zero means absent from U-Use, and
                // ordinary positive quantities produce the counted layout."
                //
                // `RETRACTIONS.md` R456 settles the case that produced the
                // earlier reading here. Normal acquisition gives the
                // Amulet, Crown and Sceptre the no-quantity value, so their
                // rows omit both cells; a *saved* quantity-one value still
                // yields the counted row. The measurement this engine took
                // (`qa/paired/use-specials.tsv`, beat `skullkeysrow`,
                // printing ` 1 Amulet`) came from a save holding one, so it
                // remains correct - it was generalised too far.
                quantity: (value != crate::commands::USE_PICKER_NO_QUANTITY).then_some(value),
                request,
            });
        }
    }

    pub fn use_wooden_box(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        self.message = USE_WOODEN_BOX_PROMPT.to_string();
        MoveOutcome::PromptDeclined
    }

    /// `quest-graph.md §5` "Presentation order": the shard-use handler is a
    /// five-phase theatrical sequence, not a chain of diagnostic gates.
    ///
    /// 1. It "first prints a heading naming the shard family and a line
    ///    describing the party holding the evil shard aloft, completed by the
    ///    shard's own virtue word (Falsehood, Hatred, or Cowardice). This
    ///    happens before any gate is evaluated."
    /// 2. "It then plays a rising pitch sweep, followed by a falling one,
    ///    again unconditionally."
    /// 3. "Only the **position** gate produces the shared no-effect result."
    /// 4. "Once the position matches, it pauses, prints a line describing the
    ///    shard being cast into the Eternal Flame completed by the opposed
    ///    principle's word (Truth, Love, or Courage), and pauses again —
    ///    **before** testing whether a Shadowlord is on the flame and whether
    ///    the handshake matches."
    /// 5. "If either of those two gates fails, the handler simply returns. It
    ///    prints no refusal line."
    ///
    /// The spec names the two divergences to avoid explicitly: "evaluating the
    /// gates before any output ... and printing a refusal for the
    /// actor/handshake failures (in the original those are silent)". Both were
    /// present here and are corrected below.
    ///
    /// Boundary: the exact heading, aloft, cast and destruction strings are
    /// not published — only their order, their completing words, and which of
    /// them are unconditional — so the literals below are engine voice. The
    /// rising/falling pitch sweep of phase 2 has no published `audio.md §8`
    /// row and no [`SoundEffect`] variant, so it is not emitted rather than
    /// borrowing a neighbouring cue; the `§8.4` destruction flash below is
    /// published and is preserved at its own boundary.
    pub fn use_shadowlord_shard(
        &mut self,
        index: usize,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let Some(item_index) = shadowlord_shard_special_item_index(index) else {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        };
        let name = special_item_name(item_index);
        // Not a gate: the U-Use picker only offers a carried shard, so this is
        // the picker precondition and it keeps its own refusal.
        if self.special_items[item_index] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return Ok(MoveOutcome::Blocked);
        }

        // Phase 1: unconditional heading plus the aloft line, completed by the
        // shard's own virtue word.
        let virtue = Self::shadowlord_title_for_index(index).unwrap_or("Shadowlord");
        let _ = virtue;
        // `inventory.md` §7.1: "The shard continuation is `Thou dost hold
        // above thee the evil Shard of_` followed by `Falsehood...`,
        // `Hatred...` or `Cowardice...`" - the shard's own word carries an
        // ellipsis. Measured 2026-09-11 (`use-specials/sceptre`): the
        // original's row reads `Falsehood...` where this engine read
        // `Falsehood`.
        self.message = format!("{USE_SHARD_ALOFT_PREFIX}{name}...");

        // Phase 3: the position gate is the only one that speaks. The
        // published destruction rows (Lycaeum floor 2 (15,9); Empath Abbey
        // floor 1 (15,3); Serpent's Hold floor 0xFF (15,16)) already carry
        // their opposed flame, so "am I on this shard's row?" is one test and
        // it produces the shared no-effect result on any mismatch.
        let required_flame = eternal_flame_for_shadowlord(index).expect("valid shard index");
        let flame_entry = if let Some(game_dir) = game_dir {
            self.eternal_flame_at_current_position(game_dir)?
        } else {
            self.published_eternal_flame_at_current_position()
        };
        let on_position = flame_entry.is_some_and(|entry| entry.flame == required_flame);
        if !on_position {
            // §7.1: "A wrong destruction position adds `\n\nNo effect!\n`" -
            // two line feeds, so a blank row stands between the aloft line and
            // the refusal. This engine spent one.
            self.message.push_str("\n\nNo effect!\n");
            return Ok(MoveOutcome::Blocked);
        }

        // Phase 4: the cast-into-the-flame line, completed by the opposed
        // principle's word, printed *before* the remaining two gates.
        // §7.1: "At the matching position, the next text is
        // `\n\n...and cast it into the Flame of_` followed by `Truth!\n`,
        // `Love!\n` or `Courage!\n`."
        self.message.push_str(&format!(
            "\n\n...and cast it into the {}!\n",
            required_flame.label()
        ));

        // Phase 5: the Shadowlord-on-flame and handshake gates are silent.
        // `self.message` is left as the cast line and nothing new is printed.
        // A shard whose Shadowlord slot is already vanquished can have no live
        // matching encounter, so it belongs with these silent tests rather
        // than with the position gate.
        if !self.shadowlord_alive(index) || !self.matching_shadowlord_name_encounter_north(index) {
            return Ok(MoveOutcome::Blocked);
        }

        self.special_items[item_index] = 0;
        self.vanquish_shadowlord(index);
        // `audio.md §8.4`: Shadowlord destruction shares the turbulent
        // full-viewport flash. It runs at the destruction boundary — after the
        // shard is consumed and the hideout slot flips to vanquished, before
        // the encounter clearing, redraw, and turn advance — and it draws all
        // 1,856 gameplay-PRNG bands there whether or not sound is audible.
        self.emit_major_flash();
        let cleared = self.clear_shadowlord_name_encounters(index);
        self.mark_visibility_dirty();
        self.advance_turn();
        // Phase 6 closes with a line naming the destroyed Shadowlord.
        self.message.push_str(&format!(
            "\n{} is vanquished! Cleared {cleared} encounter(s).",
            shadowlord_name_for_slot(index).unwrap_or("The Shadowlord")
        ));
        Ok(MoveOutcome::Used)
    }

    pub fn eternal_flame_at_current_position(
        &self,
        game_dir: &Path,
    ) -> io::Result<Option<EternalFlameEntry>> {
        if let Some(entry) = self.published_eternal_flame_at_current_position() {
            return Ok(Some(entry));
        }
        let Some(entries) = load_eternal_flame_entries(game_dir)? else {
            return Ok(None);
        };
        Ok(entries
            .into_iter()
            .find(|entry| self.eternal_flame_entry_matches(*entry)))
    }

    pub fn published_eternal_flame_at_current_position(&self) -> Option<EternalFlameEntry> {
        let (target, floor, x, y) = self.current_blink_context();
        const PUBLISHED_FLAMES: [EternalFlameEntry; 3] = [
            EternalFlameEntry {
                target: PlayTarget::Town(Scene {
                    byte: SCENE_THE_LYCAEUM,
                    family: Family::Keep,
                    block: 5,
                }),
                floor: 2,
                x: 15,
                y: 9,
                flame: EternalFlame::Truth,
                expected_tile: None,
            },
            EternalFlameEntry {
                target: PlayTarget::Town(Scene {
                    byte: SCENE_EMPATH_ABBEY,
                    family: Family::Keep,
                    block: 6,
                }),
                floor: 1,
                x: 15,
                y: 3,
                flame: EternalFlame::Love,
                expected_tile: None,
            },
            EternalFlameEntry {
                target: PlayTarget::Town(Scene {
                    byte: SCENE_SERPENTS_HOLD,
                    family: Family::Keep,
                    block: 7,
                }),
                floor: -1,
                x: 15,
                y: 16,
                flame: EternalFlame::Courage,
                expected_tile: None,
            },
        ];
        PUBLISHED_FLAMES.into_iter().find(|entry| {
            entry.target == target && entry.floor == floor && entry.x == x && entry.y == y
        })
    }

    pub fn eternal_flame_entry_matches(&self, entry: EternalFlameEntry) -> bool {
        let (target, floor, x, y) = self.current_blink_context();
        let flame_tile = self.current_area_tile(entry.x, entry.y);
        entry.target == target
            && entry.floor == floor
            && entry.x == x
            && entry.y == y
            && entry
                .expected_tile
                .map_or(matches!(flame_tile, 0x76..=0x77), |expected| {
                    expected == flame_tile
                })
    }

    pub fn clear_shadowlord_name_encounters(&mut self, index: usize) -> usize {
        if self.summoned_shadowlord != Some(index) {
            return 0;
        }
        let Some(floor) = self.current_floor() else {
            return 0;
        };
        let mut cleared = 0;
        for object in self.active_objects.iter_mut().skip(1) {
            if object.z == floor && Self::is_shadowlord_actor(*object) {
                object.free();
                cleared += 1;
            }
        }
        self.summoned_shadowlord = None;
        cleared
    }

    pub fn use_worn_regalia(
        &mut self,
        special_item_index: usize,
        effect_tag: u8,
        missing_label: &str,
        wear_message: &str,
        remove_message: &str,
    ) -> MoveOutcome {
        if self.special_items[special_item_index] == 0 {
            self.message = format!("No {missing_label}!");
            return MoveOutcome::Blocked;
        }

        self.message = if self.active_effect_tag == Some(effect_tag) {
            self.clear_active_effect_slot();
            remove_message.to_string()
        } else {
            self.active_effect_tag = Some(effect_tag);
            self.active_effect_counter = PERMANENT_ACTIVE_EFFECT_DURATION;
            wear_message.to_string()
        };
        self.mark_visibility_dirty();
        self.advance_turn();
        MoveOutcome::Used
    }

    pub fn use_sceptre_of_lord_british(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let dissolved = self.dissolve_sceptre_barriers_near_party();
        if dissolved == 0 {
            self.message = format!("{USE_SCEPTRE_WIELDED}\nNo effect!");
            return MoveOutcome::Blocked;
        }

        self.mark_visibility_dirty();
        self.advance_turn();
        // `commands.md §8.1`: a command "never prints tile ids, coordinates,
        // active-object slot numbers, terrain-class names" - a count of
        // dissolved cells is the same kind of leak, so it goes to the harness
        // and the player sees only the published wielding line.
        self.push_diagnostic(format!("Sceptre dissolved {dissolved} barrier cell(s)."));
        self.message = USE_SCEPTRE_WIELDED.to_string();
        MoveOutcome::Used
    }

    pub fn dissolve_sceptre_barriers_near_party(&mut self) -> usize {
        let mut dissolved = 0;
        let x = self.player.x as isize;
        let y = self.player.y as isize;
        for dy in -1..=1 {
            for dx in -1..=1 {
                let tx = x + dx;
                let ty = y + dy;
                if self.dissolve_sceptre_barrier_at(tx, ty) {
                    dissolved += 1;
                }
            }
        }
        dissolved
    }

    fn dissolve_sceptre_barrier_at(&mut self, x: isize, y: isize) -> bool {
        let Some(index) = self.top_down_grid_index(x, y) else {
            return false;
        };
        if !(SCEPTRE_BARRIER_TILE_FIRST..=SCEPTRE_BARRIER_TILE_LAST).contains(&self.grid[index]) {
            return false;
        }
        self.grid[index] = SCEPTRE_BARRIER_DISSOLVED_TILE;
        true
    }

    fn top_down_grid_index(&self, x: isize, y: isize) -> Option<usize> {
        match self.area {
            Area::World { .. } => {
                if !(0..WORLD_SIDE as isize).contains(&x) || !(0..WORLD_SIDE as isize).contains(&y)
                {
                    return None;
                }
                Some(world_cell_index(x as usize, y as usize))
            }
            Area::Town { .. } => {
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    return None;
                }
                Some(y as usize * 32 + x as usize)
            }
            Area::Dungeon { .. } => None,
        }
    }

    pub fn use_hms_cape_plans(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if !matches!(self.player.transport, TransportState::Ship { .. }) {
            self.message = USE_PLANS_SHIPBOARD_ONLY_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] =
            self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX].max(2);
        self.advance_turn();
        // Measured 2026-09-07 aboard a frigate: the line ends in an
        // exclamation mark, like the rest of the U-Use results.
        self.message = USE_PLANS_RIGGED_LINE.to_string();
        MoveOutcome::Used
    }

    pub fn ship_rigging_active(&self) -> bool {
        self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] > 1
    }

    pub fn advance_sailing_wait_turn(&mut self) {
        if self.ship_rigging_active() {
            let advance_active_objects = self.turn % 2 == 1;
            self.advance_turn_with_minutes_and_active_objects(1, advance_active_objects);
        } else {
            self.advance_turn();
        }
    }

    pub fn use_magic_carpet(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        // `inventory.md §7.1` (`cleak/u5-spec#251`) publishes the whole gate
        // order, and it is not the carpet movement predicate the engine was
        // reusing: "Activating a carried carpet uses its own terrain rule: it
        // accepts map tile ids `0x00..0x0B` and `0x0D..0xFF`, and rejects only
        // mountains `0x0C`. Neither the on-foot nor the carpet movement
        // predicate is consulted." Chairs `0x90..0x93` therefore permit
        // boarding even though carpet *movement* rejects them, which is what
        // a capture at Iolo's hut shows.
        //
        // Gate 1: "Scene ids `0x21..0xFF` produce `Not here!\n` without a
        // terrain lookup." That band is every dungeon and the temporary
        // combat scene; `0x00..0x20` is the overworld and the town family.
        if self.current_scene_byte() > CARPET_BOARDING_LAST_ELIGIBLE_SCENE {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        // Gate 2: "In an eligible scene, mountains produce that same refusal
        // **before transport is considered**. Thus a ship marker on mountains
        // receives `Not here!\n`."
        let tile = self.current_area_tile(self.player.x, self.player.y);
        if tile == CARPET_BOARDING_REJECTED_TILE {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        // Gate 3: "exact transport marker `0x1C` permits boarding; ship
        // markers `0x20..0x27` produce `X-it ship first!\n`, and every other
        // marker produces `Only on foot!\n`." Foot persists exactly `0x1C`.
        if !self.player.transport.is_foot() {
            self.message = if matches!(self.player.transport, TransportState::Ship { .. }) {
                USE_MAGIC_CARPET_XIT_FIRST.to_string()
            } else {
                USE_MAGIC_CARPET_ONLY_ON_FOOT.to_string()
            };
            return MoveOutcome::Blocked;
        }

        // "Success prints `Boarded!\n`, selects one of the two carpet frames
        // with equal probability" - markers `0x14` and `0x15`, which are the
        // first two tiles of the playable carpet band.
        let frame =
            FIRST_PLAYABLE_MAGIC_CARPET_TILE + u5_prng_range_u16(&mut self.prng_state, 0, 1) as u8;
        let transport = TransportState::Carpet {
            type_byte: frame,
            tile: frame,
        };

        self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] =
            self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX].saturating_sub(1);
        self.player.transport = transport;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = USE_MAGIC_CARPET_BOARDED.to_string();
        MoveOutcome::Boarded
    }

    pub fn use_sextant(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        // `catalogs/item-list.md` Sextant row / `inventory.md §7`: three
        // conditions, of which the plane is tested first and
        // short-circuits. The Underworld is the outdoor world scene on the
        // other plane, so it fails here and takes the same refusal an
        // indoor scene takes — no Underworld-specific message, and it
        // never reaches the night test.
        let outdoors = match self.area {
            Area::World { plane } => {
                sextant_outdoor_position(plane.plane_byte(), self.current_scene_byte())
            }
            Area::Town { .. } | Area::Dungeon { .. } => false,
        };
        if !outdoors {
            // Measured 2026-09-07 indoors: `Only outdoors!`.
            self.message = USE_SEXTANT_ONLY_OUTDOORS.to_string();
            return MoveOutcome::Blocked;
        }
        // The Sextant's night window is `19..=23` / `0..=5`, which is not
        // the town-lighting window `is_town_night_hour` carries.
        if !sextant_night_hour(self.clock.hour) {
            self.message = USE_SEXTANT_DAYTIME_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        self.advance_turn();
        // `magic.md §8` / `inventory.md §7`: the Sextant shares Locate's
        // coordinate printer, so both callers go through the one helper.
        // The label is followed by the printer's leading newline, then Y
        // and X each carry their own closing quote separated by
        // comma-space, then a further line break.
        // Measured 2026-09-07: the label prints in the text font and the
        // coordinate row in the runic one. Under `magic.md` §8's published
        // rule the runic cells decode to exactly this pair, so the values
        // were right all along and only the font was wrong
        // (`cleak/u5-spec#237`).
        let pair = sextant_coordinate_pair_line(self.player.y as u8, self.player.x as u8);
        self.emit_message_line(USE_SEXTANT_READING_LABEL);
        for row in pair.split('\n').filter(|row| !row.is_empty()) {
            self.push_runic_message_entry(row);
        }
        self.adopt_flushed_message(format!("{USE_SEXTANT_READING_LABEL}{pair}"));
        MoveOutcome::Used
    }

    pub fn use_pocket_watch(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        let display_hour = self.clock.display_hour();
        let minute = self.clock.minute;
        let suffix = self.clock.am_pm_suffix();
        self.advance_turn();
        self.message = format!("{USE_POCKET_WATCH_PREFIX}{display_hour}:{minute:02} {suffix}.");
        MoveOutcome::Used
    }

    /// `catalogs/item-list.md` Spyglass row / `inventory.md §7`: the
    /// world-plane byte the Sextant/Spyglass plane test reads. On the
    /// outdoor scenes it is the active plane. Inside a town or dungeon
    /// the party is still standing on one of the two planes, and the
    /// return snapshot is what remembers which; with no snapshot the
    /// surface plane is the only reachable answer, because a game that
    /// has never crossed to the Underworld has never left it.
    pub fn current_world_plane_byte(&self) -> u8 {
        match self.area {
            Area::World { plane } => plane.plane_byte(),
            Area::Town { .. } | Area::Dungeon { .. } => self
                .return_world
                .as_ref()
                .map_or(WorldPlane::Britannia, |snapshot| snapshot.plane)
                .plane_byte(),
        }
    }

    pub fn use_spyglass(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        // `catalogs/item-list.md` Spyglass row: three conditions, of
        // which the plane and scene pair is the "not here" refusal and
        // the hour is the no-stars refusal. The scene gate admits the
        // outdoor world scene *or a town-class scene* — broader than the
        // Sextant's — and the night window is the Sextant's `19..=23` /
        // `0..=5`, not the town-lighting window this handler used to
        // read, which disagrees at hours 5 and 19.
        if !spyglass_position_admits(self.current_world_plane_byte(), self.current_scene_byte()) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if !sextant_night_hour(self.clock.hour) {
            self.message = USE_SPYGLASS_NO_STARS.to_string();
            return MoveOutcome::Blocked;
        }

        // `inventory.md §7.1` utility results: "Spyglass accepted | `Looking...\n`,
        // then the sky view". The engine's own sentence named the item and
        // described the act.
        self.activate_night_sky_overlay(Some("Looking..."));
        MoveOutcome::Observed
    }

    /// `systems/view.md §4.2.1`: a telescope shows the sun during
    /// hours 6 through 17, selects the first healthy/poisoned member
    /// when necessary, and applies one point through the shared damage
    /// path. All other hours enter the night-sky overlay.
    pub fn look_through_telescope(&mut self) {
        if !sky_view_is_daylight(self.clock.hour) {
            self.activate_night_sky_overlay(None);
            return;
        }

        self.active_view_overlay = None;
        if self.active_player.is_none() {
            self.active_player = self
                .party
                .iter()
                .position(|member| matches!(member.status, b'G' | b'P'));
        }
        if let Some(slot) = self.active_player.filter(|slot| *slot < self.party.len()) {
            self.apply_shared_party_damage(slot, 1);
        }
        self.message = "the sun!".to_string();
    }

    /// `systems/view.md §4.2.2`: capture the eighty PRNG-selected stars
    /// once when the modal view opens, hide the ordinary visibility
    /// window, and leave it dirty so closing the overlay restores the
    /// world view.
    pub fn activate_night_sky_overlay(&mut self, message_prefix: Option<&str>) {
        let stars =
            std::array::from_fn(|_| (self.random_range_u8(9, 182), self.random_range_u8(9, 172)));
        let sky = SkyOverlayState {
            stars,
            body_columns: sky_body_columns(self.clock),
        };
        self.mark_visibility_dirty();
        for row in 0..VIEWPORT_SIDE {
            for col in 0..VIEWPORT_SIDE {
                self.visibility_grid[visibility_grid_active_index(row, col).unwrap()] =
                    VISIBILITY_HIDDEN;
            }
        }
        let text_map = sky_text_map(&sky, self.shadowlord_hideouts);
        self.active_view_overlay = Some(ViewOverlay {
            title: "the night sky!".to_string(),
            text_map,
            kind: ViewOverlayKind::Sky(sky),
            mode: ViewOverlayMode::SkyView,
        });
        self.message = match message_prefix {
            Some(prefix) => format!("{prefix}\nthe night sky! "),
            None => "the night sky! ".to_string(),
        };
    }

    pub fn use_scroll(
        &mut self,
        index: usize,
        direction: Option<Direction>,
        target: Option<usize>,
    ) -> MoveOutcome {
        if index >= SCROLL_COUNT || self.scroll_stock[index] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        self.scroll_stock[index] = self.scroll_stock[index].saturating_sub(1);

        match index {
            SCROLL_LIGHT_INDEX => {
                self.light_spell_counter = SCROLL_LIGHT_DURATION;
                self.recompute_daylight();
                // `audio.md §6.1`: "Sets a torch radius, then calls the
                // dispatcher directly." The scroll supplies its scroll index,
                // so this is variant 0 - the one variant no spell reaches, and
                // emphatically not In Lor's variant 1.
                self.emit_scroll_shared_variant(SCROLL_LIGHT_INDEX);
                self.advance_turn();
                self.message = "Light!".to_string();
                MoveOutcome::Used
            }
            SCROLL_WIND_CHANGE_INDEX => self.use_wind_change_scroll(direction),
            SCROLL_PROTECTION_INDEX => {
                // `audio.md §6.1`: "Through the scene-flag helper", at the
                // scroll's own index 2 - not the Protection spell's variant 4.
                self.emit_scroll_shared_variant(SCROLL_PROTECTION_INDEX);
                self.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
                self.active_effect_counter = SCROLL_PROTECTION_DURATION;
                self.advance_turn();
                self.message = "Protection!".to_string();
                MoveOutcome::Used
            }
            SCROLL_NEGATE_MAGIC_INDEX => {
                // `audio.md §6.1`: scene-flag helper at scroll index 3, where
                // the Negate Magic spell is variant 6.
                self.emit_scroll_shared_variant(SCROLL_NEGATE_MAGIC_INDEX);
                self.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
                self.active_effect_counter = SCROLL_NEGATE_MAGIC_DURATION;
                self.advance_turn();
                self.message = "Negate magic!".to_string();
                MoveOutcome::Used
            }
            SCROLL_VIEW_INDEX => {
                if self.combat_active {
                    // `audio.md §6.1`: "Refused with `Not here!` and **no
                    // sound** outside the permitted scene class." `§9` lists
                    // this refusal among the explicit silence boundaries.
                    self.message = "Not here!".to_string();
                    return MoveOutcome::Blocked;
                }
                // `audio.md §6.1`: "Dispatcher, then the look helper" - and
                // "Both look helpers are silent", so the variant is all this
                // scroll plays.
                self.emit_scroll_shared_variant(SCROLL_VIEW_INDEX);
                self.advance_turn();
                let _ = self.activate_peer_view_overlay();
                self.message = "View!".to_string();
                MoveOutcome::Observed
            }
            SCROLL_SUMMON_DAEMON_INDEX => {
                if !self.combat_active {
                    // `audio.md §6.1`: "only in the permitted scene class, else
                    // `Not here!` and silence" (§9).
                    self.message = "Not here!".to_string();
                    return MoveOutcome::Blocked;
                }
                let Some(caster) = self.combat_actors.first().copied() else {
                    // An arena with no actors cannot take a command.
                    self.message.clear();
                    return MoveOutcome::Blocked;
                };
                if !combat_actor_is_active_not_dead(caster) {
                    // Not a state a keystroke can produce; see the other guards.
                    self.message.clear();
                    return MoveOutcome::Blocked;
                }
                // `audio.md §6.1`: "Through the placement helper", which
                // sounds unconditionally at entry, before the eight-try cell
                // probe. Variant 5 is the scroll index; the Summon Daemon
                // *spell* is variant 8.
                self.emit_scroll_shared_variant(SCROLL_SUMMON_DAEMON_INDEX);
                let legal_cells = self.combat_legal_cell_mask();
                let applied = self.apply_combat_summon_daemon_with_random_attempts(
                    self.combat_actor_z(0),
                    &legal_cells,
                );
                self.advance_turn();
                let Some(applied) = applied else {
                    self.message = "Failed!".to_string();
                    return MoveOutcome::Blocked;
                };
                if self.combat_summon_daemon_self_check_oops(0) {
                    self.message = "Oops...".to_string();
                    MoveOutcome::Blocked
                } else {
                    self.combat_actors[applied.actor_slot].flags |= COMBAT_ACTOR_FLAG_CONTROLLED;
                    self.message = "Summon Daemon!".to_string();
                    MoveOutcome::Used
                }
            }
            SCROLL_RESURRECTION_INDEX => self.use_resurrection_scroll(target),
            SCROLL_NEGATE_TIME_INDEX => self.use_negate_time_scroll(),
            _ => {
                self.message = "No effect!".to_string();
                MoveOutcome::Blocked
            }
        }
    }

    pub fn use_wind_change_scroll(&mut self, direction: Option<Direction>) -> MoveOutcome {
        let Some(direction) = direction else {
            self.request_cast_argument(CastArgumentRequest::Direction);
            return MoveOutcome::Blocked;
        };
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let previous = self.wind.status_message();
        let next = WindState::rel_hur_target(direction)
            .expect("inline wind scroll parser returns cardinal directions only");
        // `audio.md §7.3`: the scroll carries its own caller tag and plays
        // variant 1, not the spell's variant 2.
        self.apply_wind_state_from_scroll(next);
        self.advance_turn();
        // Measured: the effect line is printed when the prompt opens, and
        // the accepted direction closes the exchange with nothing further.
        let _ = previous;
        self.message.clear();
        MoveOutcome::Used
    }

    pub fn use_resurrection_scroll(&mut self, target: Option<usize>) -> MoveOutcome {
        if self.combat_active {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(target_index) = target else {
            self.request_cast_argument(CastArgumentRequest::PartyTarget);
            return MoveOutcome::Blocked;
        };
        if target_index >= self.party.len() {
            // Out-of-range member: the interactive pickers bound the
            // index, so only the inline harness form reaches this. The
            // original has no line for a state it cannot enter.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        if self.party[target_index].status != b'D' {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        // `audio.md §6.1`: "Through the resurrect helper; only in the permitted
        // scene class and only if the target is dead." Variant 6 is the scroll
        // index; the Resurrect spell is variant 8.
        self.emit_scroll_shared_variant(SCROLL_RESURRECTION_INDEX);

        let _ = self
            .resurrect_party_member_to_hp(target_index, 1)
            .expect("target status checked before scroll resurrection");
        self.advance_turn();
        // Measured: the raising itself prints nothing; `Resurrection!` was
        // printed when the target prompt opened.
        self.message.clear();
        MoveOutcome::Used
    }

    pub fn use_negate_time_scroll(&mut self) -> MoveOutcome {
        if matches!(
            self.area,
            Area::Town { scene, .. } if scene.byte == STONEGATE_SCENE_BYTE
        ) || matches!(
            self.area,
            Area::Dungeon { scene, .. } if scene.byte == 40
        ) {
            self.advance_turn();
            self.message = "No effect!".to_string();
            // `audio.md §6.1`: "In two specific scenes it instead prints
            // `No effect!` and plays the 50-update cast-failure glissando."
            self.emit_sound_effect(SoundEffect::CastFailure);
            return MoveOutcome::Blocked;
        }

        // `audio.md §6.1`: "Through the scene-flag helper" at scroll index 7.
        self.emit_scroll_shared_variant(SCROLL_NEGATE_TIME_INDEX);
        self.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        self.active_effect_counter = SCROLL_NEGATE_TIME_DURATION;
        self.advance_turn();
        self.message = "Negate time!".to_string();
        MoveOutcome::Used
    }

    pub fn use_potion(&mut self, index: usize, target: Option<usize>) -> MoveOutcome {
        if index >= POTION_COUNT || self.potion_stock[index] == 0 {
            // The U-Use picker lists only what the party carries, so no
            // shipped path reaches this branch: it is the inline harness
            // form's guard, and the original has no line for it.
            self.message.clear();
            return MoveOutcome::Blocked;
        }
        self.potion_stock[index] = self.potion_stock[index].saturating_sub(1);

        let Some(target_index) = target else {
            // Measured: the potion asks `On who: ` and completes the row
            // with the chosen member's name.
            self.request_cast_argument(CastArgumentRequest::PartyTarget);
            self.message = USE_POTION_TARGET_PROMPT.to_string();
            return MoveOutcome::Blocked;
        };
        if target_index >= self.party.len() {
            // Out-of-range member: the interactive pickers bound the
            // index, so only the inline harness form reaches this. The
            // original has no line for a state it cannot enter.
            self.message.clear();
            return MoveOutcome::Blocked;
        }

        let variation_roll = self.potion_variation_roll(index, target_index);
        let random_roll = self.potion_random_effect_roll(index, target_index);
        let effect_index = potion_effect_index_after_variation(index, variation_roll, random_roll);
        self.use_potion_with_effect(index, target_index, effect_index)
    }

    pub fn use_potion_with_effect(
        &mut self,
        selected_index: usize,
        target_index: usize,
        effect_index: usize,
    ) -> MoveOutcome {
        // `catalogs/item-list.md §7.2`: the accepted selected bottle owns a
        // blocking full-playfield flash before the later variation-selected
        // gameplay effect. The frontend consumes this pending event before it
        // displays the resulting gameplay state.
        self.pending_potion_flash = potion_flash_playback(selected_index);
        // `audio.md §7.2`: the shared presentation begins only once the
        // bottle is decremented (`use_potion`) and a party-member target is
        // accepted, and "the selected bottle id, not the later variation roll,
        // chooses variant 0 through 7 from Section 6". The published flash
        // playback already keys on the selected bottle — its rumble target is
        // `8000 + 1600 * selected_index` and its sweep length
        // `10000 + 4000 * selected_index`, which are exactly §6's target and
        // iteration columns for variant `selected_index` — so the bottle id is
        // the variant, with no remapping. An out-of-range bottle yields no
        // playback and no cue.
        if let Some(playback) = self.pending_potion_flash {
            self.emit_sound_effect(SoundEffect::SharedVariant {
                variant: playback.selected_index as u8,
            });
        }
        match effect_index {
            POTION_BLUE_INDEX => {
                if self.party[target_index].status == b'S' && self.party[target_index].hp > 0 {
                    self.party[target_index].status = b'G';
                    if self.clear_combat_party_sleep_presentation(target_index) {
                        self.mark_visibility_dirty();
                    }
                    self.advance_turn();
                    // Measured: waking a sleeping member prints no line at
                    // all - only the status letter changes.
                    self.message.clear();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_YELLOW_INDEX => {
                if !self.party[target_index].living() {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    return MoveOutcome::Blocked;
                }
                let amount = self.potion_heal_amount(selected_index, target_index);
                let _ = self.party[target_index].heal_by(amount);
                self.advance_turn();
                self.message = POTION_RESULT_HEALED.to_string();
                MoveOutcome::Used
            }
            POTION_RED_INDEX => {
                if self.party[target_index].status == b'P' {
                    self.party[target_index].status = b'G';
                    self.advance_turn();
                    self.message = POTION_RESULT_POISON_CURED.to_string();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_GREEN_INDEX => {
                if self.party[target_index].status == b'G' && self.party[target_index].hp > 0 {
                    self.party[target_index].status = b'P';
                    self.advance_turn();
                    self.message = POTION_RESULT_POISONED.to_string();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_ORANGE_INDEX => {
                if self.party[target_index].status == b'G' && self.party[target_index].hp > 0 {
                    if self.combat_active {
                        if matches!(
                            apply_combat_sleep_to_party_target(&mut self.party[target_index]),
                            CombatPartySleepOutcome::SleptPartyMember { .. }
                        ) && self.apply_combat_party_sleep_presentation(target_index)
                        {
                            self.mark_visibility_dirty();
                        }
                    } else {
                        self.party[target_index].status = b'S';
                    }
                    self.advance_turn();
                    self.message = POTION_RESULT_SLEPT.to_string();
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = POTION_RESULT_FAILED.to_string();
                    MoveOutcome::Blocked
                }
            }
            POTION_PURPLE_INDEX => {
                if !self.combat_active {
                    self.advance_turn();
                    self.message = POTION_RESULT_NO_NOTICEABLE_EFFECT.to_string();
                    return MoveOutcome::Blocked;
                }
                let applied = self.apply_combat_potion_poof_presentation(target_index);
                if applied {
                    self.mark_visibility_dirty();
                }
                self.advance_turn();
                // The combat-only presentations are not measured; the
                // engine prints nothing rather than inventing a sentence.
                self.message.clear();
                if applied {
                    MoveOutcome::Used
                } else {
                    MoveOutcome::Blocked
                }
            }
            POTION_BLACK_INDEX => {
                if !self.combat_active {
                    self.advance_turn();
                    self.message = POTION_RESULT_NO_NOTICEABLE_EFFECT.to_string();
                    return MoveOutcome::Blocked;
                }
                self.advance_turn();
                let applied = self.apply_combat_party_invisibility_potion(target_index);
                self.message.clear();
                if applied {
                    MoveOutcome::Used
                } else {
                    MoveOutcome::Blocked
                }
            }
            POTION_WHITE_INDEX => {
                if matches!(self.area, Area::Dungeon { .. }) || self.combat_active {
                    self.advance_turn();
                    self.message = POTION_RESULT_NO_NOTICEABLE_EFFECT.to_string();
                    return MoveOutcome::Blocked;
                }
                self.start_visibility_sweep();
                self.advance_turn();
                // Measured: the white potion's repaint prints no line.
                self.message.clear();
                MoveOutcome::Observed
            }
            _ => {
                self.advance_turn();
                self.message = POTION_RESULT_FAILED.to_string();
                MoveOutcome::Blocked
            }
        }
    }

    pub fn potion_variation_roll(&self, selected_index: usize, target_index: usize) -> u8 {
        (self.turn as u8)
            .wrapping_add((selected_index as u8).wrapping_mul(13))
            .wrapping_add((target_index as u8).wrapping_mul(29))
            .wrapping_add((self.player.x as u8).wrapping_mul(3))
            .wrapping_add((self.player.y as u8).wrapping_mul(5))
            .wrapping_add(self.clock.hour)
            & 0x0f
    }

    pub fn potion_random_effect_roll(&self, selected_index: usize, target_index: usize) -> u8 {
        (self.turn as u8)
            .rotate_left(1)
            .wrapping_add((selected_index as u8).wrapping_mul(37))
            .wrapping_add((target_index as u8).wrapping_mul(11))
            .wrapping_add(self.clock.minute)
    }

    pub fn potion_heal_amount(&self, selected_index: usize, target_index: usize) -> u16 {
        let raw_roll = (self.turn as u8)
            .wrapping_add((selected_index as u8).wrapping_mul(9))
            .wrapping_add((target_index as u8).wrapping_mul(17))
            .wrapping_add((self.player.x as u8).wrapping_mul(3))
            .wrapping_add((self.player.y as u8).wrapping_mul(5))
            % (HEAL_RAW_ROLL_MAX + 1);
        heal_spell_amount_from_raw_roll(raw_roll)
    }

    pub fn clear_combat_party_sleep_presentation(&mut self, target_index: usize) -> bool {
        if !self.combat_active || target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get_mut(target_index) else {
            return false;
        };
        if !actor.is_status_disabled() {
            return false;
        }
        actor.clear_status_disabled();
        // `catalogs/item-list.md` "Orange combat sleep presentation": wake
        // "selects the retained base/type tile as the display tile, except
        // that an actor still under the combat invisibility state uses tile
        // `0x1D`". Invisibility "rides on bit `0x10`, the phase/blink bit"
        // (`RETRACTIONS.md` R380); bit `0x04` is the dragged-under state.
        let hidden = actor.is_phase_suppressed();
        let active_object_slot = usize::from(actor.active_object_slot);
        if let Some(object) = self.active_objects.get_mut(active_object_slot) {
            object.tile = if hidden {
                COMBAT_POTION_INVISIBLE_WAKE_DISPLAY_TILE
            } else {
                object.type_byte
            };
        }
        true
    }

    pub fn apply_combat_party_sleep_presentation(&mut self, target_index: usize) -> bool {
        if !self.combat_active || target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get_mut(target_index) else {
            return false;
        };
        if actor.is_empty() || actor.is_marked_dead() {
            return false;
        }
        let active_object_slot = usize::from(actor.active_object_slot);
        let Some(object) = self.active_objects.get_mut(active_object_slot) else {
            return false;
        };
        actor.set_status_disabled();
        object.tile = COMBAT_POTION_SLEEP_DISPLAY_TILE;
        true
    }

    pub fn apply_combat_potion_poof_presentation(&mut self, target_index: usize) -> bool {
        if !self.combat_active || target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get(target_index).copied() else {
            return false;
        };
        if actor.is_empty() || actor.is_marked_dead() {
            return false;
        }
        let active_object_slot = usize::from(actor.active_object_slot);
        if active_object_slot >= self.active_objects.len() {
            return false;
        }
        let object = &mut self.active_objects[active_object_slot];
        object.type_byte = COMBAT_POTION_POOF_TILE;
        object.tile = COMBAT_POTION_POOF_TILE;
        true
    }

    /// `catalogs/item-list.md §7.2` (White visibility repaint sequence) and
    /// `systems/magic.md §8` (X-Ray, *Wis An Ylem*): the shared spell/potion
    /// visibility sweep. It "invokes the ordinary visibility producer exactly
    /// once, centred on the party's local viewport position — and it invokes
    /// it in the producer's **no-line-of-sight mode**, by passing the negative
    /// sentinel in the light argument". Every one of the 121 cells of the
    /// eleven-by-eleven window is refilled straight from the map: no distance
    /// test, no propagation frontier, no blocker rule. "A wall does not stop
    /// the reveal, and a cell in the far corner is revealed exactly as readily
    /// as the party's own."
    ///
    /// **R318.** The withdrawn text had this call pass the value `32` as an
    /// inclusive squared-Euclidean gate admitting 101 of the 121 cells, with a
    /// blocker inside the gate visible but stopping propagation past itself.
    /// That argument is never read by the producer.
    ///
    /// "The normal map reader supplies tiles, so overworld coordinate wrapping
    /// and named-location bounds remain exactly their ordinary rules; White
    /// adds no scan, clipping, or wrapping rule of its own" — the window-bound
    /// clipping stays in the render grid, which is the map reader here.
    pub fn start_visibility_sweep(&mut self) {
        let wrap_world = matches!(self.area, Area::World { .. });
        let visible = self.surface_visibility_produce(
            self.player.x as isize,
            self.player.y as isize,
            VIEWPORT_PLAYER_ROW,
            VISIBILITY_NO_LINE_OF_SIGHT_LIGHT,
            wrap_world,
        );
        let mut visible_cells = [false; VIEWPORT_SIDE * VIEWPORT_SIDE];
        visible_cells.copy_from_slice(&visible);
        self.visibility_sweep = Some(VisibilitySweep {
            frames_remaining: POTION_WHITE_SWEEP_FRAMES,
            pause_bios_ticks_per_frame: POTION_WHITE_SWEEP_BIOS_TICKS_PER_FRAME,
            center_x: self.player.x,
            center_y: self.player.y,
            visible_cells,
        });
    }

    /// Take the selected-bottle flash that must be shown before the resulting
    /// potion effect is repainted.
    pub fn take_pending_potion_flash(&mut self) -> Option<PotionFlashPlayback> {
        self.pending_potion_flash.take()
    }

    pub fn apply_combat_party_invisibility_potion(&mut self, target_index: usize) -> bool {
        if target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get_mut(target_index) else {
            return false;
        };
        apply_combat_linked_invisibility(actor, &mut self.active_objects)
            .is_some_and(|outcome| outcome.actor_flags_before != outcome.actor_flags_after)
    }

    pub fn use_moonstone_phase(&mut self, slot_index: Option<usize>) -> MoveOutcome {
        let Some(slot_index) = slot_index else {
            self.message = use_prompt_message();
            return MoveOutcome::Blocked;
        };
        let Some((scene, z, tile, label)) = self.current_moonstone_bury_context() else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        if !moonstone_bury_tile_allowed(tile) {
            let _ = tile;
            self.message = MOONSTONE_BURY_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        let removed_pickup = self.clear_moonstone_pickups(slot_index);
        self.moonstone_slots[slot_index] = MoonstoneGateSlot {
            scene,
            x: self.player.x as u8,
            y: self.player.y as u8,
            z,
        };
        if removed_pickup {
            self.mark_visibility_dirty();
        }
        self.advance_turn();
        self.message = format!(
            "Buried Moonstone phase {} at {label} ({}, {}).",
            slot_index + 1,
            self.player.x,
            self.player.y
        );
        MoveOutcome::Used
    }

    pub fn current_moonstone_bury_context(&self) -> Option<(u8, u8, u8, String)> {
        match self.area {
            Area::World { plane } => Some((
                0,
                plane.save_floor() as u8,
                self.grid[world_cell_index(self.player.x, self.player.y)],
                plane.key().to_string(),
            )),
            Area::Town { scene, floor } => Some((
                scene.byte,
                floor as u8,
                self.grid[self.player.y * 32 + self.player.x],
                scene.key(),
            )),
            Area::Dungeon { .. } => None,
        }
    }

    /// `magic.md §5` steps 1 and 3-6 / `magic.md §7`: the live C-Cast
    /// dispatcher gate. Returns `Some(outcome)` when the cast was refused
    /// and the caller must stop, `None` when the spell handler should run.
    ///
    /// The gate decision itself is [`cast_dispatcher_gate`], the crate's one
    /// implementation of the spec's ordering; this method supplies the state
    /// it reads and applies the resource debits its outcome describes:
    ///
    /// - **Step 1, active-player resolve.** A missing or unconscious caster
    ///   aborts before any gate.
    /// - **Gate 5, scene.** `magic.md §7`: "the scene gate runs before
    ///   charge consumption, so `Not here!` does not spend a charge", and
    ///   `magic.md §5` step 3: "Time is *not* consumed on this rejection".
    ///   So `Not here!` debits nothing and does not advance the turn.
    /// - **Gate 6, charges.** `None mixed!` when the counter is zero;
    ///   otherwise the charge is spent immediately.
    /// - **Gate 7, mana.** `M.P. too low!` with the charge already gone.
    /// - **Gate 8, level.** `M.P. too low!` with charge *and* mana gone.
    ///
    /// `mana_cost` is the spell's circle: `catalogs/spell-list.md §1`
    /// publishes `mana_cost(id) = circle(id)` and
    /// `minimum_level(id) = circle(id)`, so the mana gate and the level gate
    /// compare against the same number. The circle is re-derived from
    /// `spell_index` here rather than trusted from the caller, so the level
    /// gate is a level-vs-circle test by construction.
    pub fn cast_spell_resource_gate(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
    ) -> Option<MoveOutcome> {
        let Some(caster) = self.party.get(caster_index).copied() else {
            // Out-of-range caster: the `Player: ` prompt bounds the index.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        };
        if !caster.conscious() {
            // Not a state a keystroke can produce; see the other guards.
            self.message.clear();
            return Some(MoveOutcome::Blocked);
        }
        let Some(circle) = spell_circle_for(spell_index as u8) else {
            self.message = "No effect!".to_string();
            return Some(MoveOutcome::Blocked);
        };
        debug_assert_eq!(
            mana_cost, circle,
            "cast_spell_resource_gate: caller passed mana cost {mana_cost} for spell \
             {spell_index}, but catalogs/spell-list.md §1 publishes mana_cost(id) = \
             circle(id) = {circle}",
        );

        let outcome = cast_dispatcher_gate(
            self.spell_allowed_in_current_cast_context(spell_index),
            self.spell_charges[spell_index],
            caster.mana,
            caster.level,
            circle,
        );
        if outcome.consumed_charge() {
            self.spell_charges[spell_index] = self.spell_charges[spell_index].saturating_sub(1);
        }
        if outcome.consumed_mana() {
            self.party[caster_index].mana = self.party[caster_index].mana.saturating_sub(circle);
        }
        if outcome.plays_failure_glissando() {
            // `magic.md §7` result table: the glissando follows the
            // rejection text; `None mixed!` is silent.
            self.emit_sound_effect(crate::audio::SoundEffect::CastFailure);
        }
        match outcome {
            CastGateOutcome::Cast => None,
            // `magic.md §5` step 3: the scene rejection costs no time. The
            // charges rejection keeps this crate's existing no-turn
            // behaviour; the spec does not settle whether `None mixed!`
            // advances the clock.
            CastGateOutcome::NotHere | CastGateOutcome::NoneMixed => {
                self.message = outcome.message().to_string();
                Some(MoveOutcome::Blocked)
            }
            CastGateOutcome::ManaTooLowChargeOnly | CastGateOutcome::LevelTooLowChargeAndMana => {
                self.message = outcome.message().to_string();
                self.advance_turn();
                Some(MoveOutcome::Blocked)
            }
        }
    }

    pub fn apply_gate_travel(
        &mut self,
        game_dir: &Path,
        phase: usize,
        target: PlayTarget,
        floor: i8,
        start: (usize, usize),
    ) -> io::Result<()> {
        let prior_sound_serial = self.sound_effect_serial;
        self.cache_current_world_overlay();
        let previous_turn = self.turn;
        let mut options = PlayOptions {
            target,
            floor,
            start: Some(start),
            clock: self.clock,
            food: self.food,
            gold: self.gold,
            keys: self.keys,
            gems: self.gems,
            climbing_gear: self.climbing_gear,
            special_items: self.special_items,
            party: self.party.clone(),
            party_names: self.party_names.clone(),
            party_experience: self.party_experience.clone(),
            party_stay_counters: self.party_stay_counters.clone(),
            party_strengths: self.party_strengths.clone(),
            party_combat_defense: self.party_combat_defense.clone(),
            party_intelligence: self.party_intelligence.clone(),
            party_equipment: self.party_equipment.clone(),
            party_roster: self.synced_party_roster(),
            equipment_stock: self.equipment_stock,
            spell_charges: self.spell_charges,
            scroll_stock: self.scroll_stock,
            potion_stock: self.potion_stock,
            reagents: self.reagents,
            rare_reagent_harvest_days: self.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: self.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: self.fixed_hidden_treasure_daily_day,
            dungeon_room_clear_bitmap: self.dungeon_room_clear_bitmap,
            saved_dungeon_working_buffer: None,
            moonstone_slots: self.moonstone_slots,
            shadowlord_hideouts: self.shadowlord_hideouts,
            removed_town_npc_flags: self.removed_town_npc_flags.clone(),
            talk_branch_flags: self.talk_branch_flags.clone(),
            shrine_ordained_mask: self.shrine_ordained_mask,
            shrine_codex_mask: self.shrine_codex_mask,
            word_of_power_seal_flags: self.word_of_power_seal_flags,
            shrine_ruin_flags: self.shrine_ruin_flags,
            moral_standing: self.moral_standing,
            toll_progress: self.toll_progress,
            cleanup_previous_hour: self.cleanup_previous_hour,
            twelve_hour_audio_repeats: self.twelve_hour_audio_repeats,
            cached_moon_glyph_bytes: self.cached_moon_glyph_bytes,
            ambient_light: self.ambient_light,
            // `overworld.md §9.1` (spec HEAD c00bf63): the
            // gate-presence counter survives scene changes.
            natural_moongate_counter: self.natural_moongate_counter,
            // `animation.md §9`/`§12.1`: the driver-side animation layer is
            // never reset and "survives scene changes, save loads, and
            // everything else short of reloading the asset". Carry the live
            // phases into the rebuilt state so water, fountains, banners and
            // clocks do not snap back to phase zero on area entry.
            animation_asset_buffer: self.animation_asset_buffer(),
            avatar_stats: self.avatar_stats,
            torches: self.torches,
            torch_counter: self.torch_counter,
            light_spell_counter: self.light_spell_counter,
            wind: self.wind,
            wind_save_byte: self.wind_save_byte,
            time_stop_counter: self.time_stop_counter,
            active_effect_tag: self.active_effect_tag,
            active_effect_counter: self.active_effect_counter,
            fortunes_of_war: self.fortunes_of_war,
            camp_cooldown: self.camp_cooldown,
            camp_month_cookie: self.camp_month_cookie,
            active_player: self.active_player,
            combat_round_counter: self.combat_round_counter,
            combat_interference_sources: self.combat_interference_sources,
            transport: TransportState::Foot,
            facing: Some(self.player.facing),
            door_tracker: None,
            pending_vehicle: None,
            pending_vehicle_save: self.pending_vehicle_save,
            inn_registry: self.inn_registry.clone(),
            initial_britannia_overlay: self.world_overlays.get(WorldPlane::Britannia),
            debug_enter: self.debug_enter,
            saved_active_objects: None,
            town_npc_mutations: self.town_npc_mutations.clone(),
            save_template_source: self.save_template_source,
        };
        if let PlayTarget::World(plane) = target {
            options.saved_active_objects = self.world_overlays.get(plane);
        }

        let mut next = Self::load_scene(game_dir, options)?;
        next.turn = previous_turn;
        next.world_overlays = self.world_overlays.clone();
        if matches!(target, PlayTarget::World(_)) {
            next.cache_current_world_overlay();
        }
        next.force_foot_transport();
        next.sync_player_object();
        next.pending_town_arrest = None;
        next.active_blackthorn = None;
        next.message = format!(
            "Gate Travel phase {phase} -> {} at ({}, {}).",
            target.key(),
            start.0,
            start.1
        );
        // `audio.md §2` keeps one serial speaker, and the frontend reads it
        // through a monotonic serial. A scene rebuild constructs `next` from
        // scratch, so its history is numbered from 1 and collides with this
        // state's early serials. Carry the outgoing history across and
        // re-serialize the destination's own entry effects on top of it:
        // clearing here used to drop every cue emitted before the transition
        // — the transit envelope among them — while the serial kept counting,
        // so the frontend advanced past serials that had no history entry and
        // the cue was silently lost.
        let entry_effects = next.sound_effects_after(0);
        next.sound_effect_serial = prior_sound_serial;
        next.sound_effect_history = std::mem::take(&mut self.sound_effect_history);
        for effect in entry_effects {
            next.emit_sound_effect(effect);
        }
        // A scene rebuild constructs `next` from scratch, so every frontend
        // presentation flag defaults off. `pace_combat_presentations` is set
        // once by the graphical shell at bootstrap: dropping it here left a
        // fight entered after any location or gate transition resolving a
        // whole sixteen-actor round inside one host frame, with the paced
        // presentation path silently unreachable for the rest of the session.
        next.pace_combat_presentations = self.pace_combat_presentations;
        *self = next;
        Ok(())
    }

    pub fn turn_dungeon(&mut self, clockwise: bool) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        let next = if clockwise {
            self.player.facing.turn_right_cardinal()
        } else {
            self.player.facing.turn_left_cardinal()
        };
        let Some(next) = next else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        self.player.facing = next;
        self.advance_turn();
        // `dungeon-mode.md §14`: a turn prints its verb and nothing else -
        // "`Turn left\n` / `Turn right\n`" - and the handler itself only
        // "decrement[s] the facing byte ... Status row updates; the
        // first-person view is repainted on the next loop iteration". The
        // verb echo is emitted by the input path, so this handler adds no
        // line of its own; it used to compose a `Turned to face East on
        // DEC (Deceit) level 0.` sentence that the original never prints,
        // and which a paired capture showed landing twice.
        // `cleak/u5-engine#5`.
        let _ = (scene, level);
        self.message.clear();
        MoveOutcome::Moved
    }

    pub fn look_dungeon(&mut self) -> MoveOutcome {
        self.look_dungeon_with_drink(None, None)
    }

    pub fn look_dungeon_with_drink(
        &mut self,
        drink: Option<bool>,
        party_index: Option<usize>,
    ) -> MoveOutcome {
        self.look_dungeon_with_focus(drink, party_index, DungeonLookFocus::Ahead)
    }

    pub fn look_dungeon_with_focus(
        &mut self,
        drink: Option<bool>,
        party_index: Option<usize>,
        focus: DungeonLookFocus,
    ) -> MoveOutcome {
        let Area::Dungeon { level, .. } = self.area else {
            // An internal invariant, not a player-facing path: the original
            // prints nothing here, so neither does this. `u5-engine#19`.
            self.message.clear();
            return MoveOutcome::Blocked;
        };
        if !self.has_personal_light() {
            // `RETRACTIONS.md` R323: the stored line breaks after the colon.
            self.message = DUNGEON_LOOK_DARKNESS_REFUSAL.to_string();
            return MoveOutcome::Observed;
        }
        let (x, y) = self.dungeon_look_focus_coord(focus);

        let tile = self.dungeon_cell(level, x, y);
        let description = dungeon_look_description(tile);
        if (tile >> 4) == 0x5 {
            // `dungeon-mode.md §12` step 6: the fountain class prints the
            // ordinary look line and then runs the drink question, whose
            // literals are the published `Will you drink?` / `No.` /
            // `Yes.  Gulp!` group rather than a composed sentence.
            let look_line = format!("{DUNGEON_LOOK_PREAMBLE}{description}.\n");
            self.message = match drink {
                None => {
                    let member_index = party_index.unwrap_or(0);
                    self.emit_message_line(look_line);
                    return self.start_dungeon_fountain_drink_prompt(member_index, focus);
                }
                Some(false) => {
                    format!("{look_line}{DUNGEON_FOUNTAIN_DRINK_PROMPT}{DUNGEON_FOUNTAIN_DECLINED}")
                }
                Some(true) => {
                    let member_index = party_index.unwrap_or(0);
                    let effect = self
                        .apply_dungeon_fountain_effect(member_index, tile)
                        .unwrap_or_default();
                    format!(
                        "{look_line}{DUNGEON_FOUNTAIN_DRINK_PROMPT}{DUNGEON_FOUNTAIN_ACCEPTED}{effect}"
                    )
                }
            };
            return if drink == Some(false) {
                MoveOutcome::PromptDeclined
            } else {
                MoveOutcome::Observed
            };
        }

        self.message = format!("{DUNGEON_LOOK_PREAMBLE}{description}.");
        MoveOutcome::Observed
    }

    pub fn dungeon_look_focus_coord(&self, focus: DungeonLookFocus) -> (usize, usize) {
        let direction = match focus {
            DungeonLookFocus::Ahead => Some(self.player.facing),
            DungeonLookFocus::Right => self.player.facing.turn_right_cardinal(),
            DungeonLookFocus::Left => self.player.facing.turn_left_cardinal(),
            DungeonLookFocus::Here => None,
        };
        let (dx, dy) = direction.map(Direction::delta).unwrap_or((0, 0));
        (
            (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize,
            (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize,
        )
    }

    pub fn apply_dungeon_fountain_effect(
        &mut self,
        member_index: usize,
        tile: u8,
    ) -> Option<String> {
        let subtype = tile & 0x0f;
        match subtype {
            0 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let before = member.status;
                member.status = b'G';
                let _ = (slot, before);
                Some(DUNGEON_FOUNTAIN_CURED.to_string())
            }
            1 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let (before, after) = member.heal_to_max();
                let _ = (slot, before, after);
                Some(DUNGEON_FOUNTAIN_HEALED.to_string())
            }
            2 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                member.status = b'P';
                let _ = slot;
                Some(DUNGEON_FOUNTAIN_POISONED.to_string())
            }
            _ => {
                let damage = self.dungeon_fountain_damage_roll();
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let applied = member.apply_damage(damage);
                let _ = (slot, applied);
                Some(DUNGEON_FOUNTAIN_BAD_TASTE.to_string())
            }
        }
    }

    pub fn view_gem(&mut self) -> MoveOutcome {
        if self.gems == 0 {
            self.message = VIEW_NO_GEM_REFUSAL.to_string();
            return MoveOutcome::Blocked;
        }

        match self.area {
            Area::Dungeon { scene, level } => {
                self.gems = self.gems.saturating_sub(1);
                let title = format!(
                    "Dungeon view of {} ({}) level {} ({} gem(s) remain; 22x22 flood map)",
                    scene.key(),
                    scene.name(),
                    dungeon_display_level(level),
                    self.gems
                );
                let text_map = self.dungeon_vision_map(level);
                self.active_view_overlay = Some(ViewOverlay {
                    title: title.clone(),
                    text_map: text_map.clone(),
                    kind: ViewOverlayKind::Dungeon { level },
                    mode: ViewOverlayMode::GemView,
                });
                self.message.clear();
                MoveOutcome::Observed
            }
            Area::Town { scene, floor } => {
                self.gems = self.gems.saturating_sub(1);
                let title = format!(
                    "Gem view of {} floor {} ({} gem(s) remain; 32x32 class map)",
                    scene.key(),
                    floor,
                    self.gems
                );
                let text_map = self.surface_view_map();
                self.active_view_overlay = Some(ViewOverlay {
                    title: title.clone(),
                    text_map: text_map.clone(),
                    kind: ViewOverlayKind::Surface,
                    mode: ViewOverlayMode::GemView,
                });
                self.message.clear();
                MoveOutcome::Observed
            }
            Area::World { plane } => {
                self.gems = self.gems.saturating_sub(1);
                let title = format!(
                    "Gem view of {} at ({}, {}) ({} gem(s) remain; 32x32 class map)",
                    plane.key(),
                    self.player.x,
                    self.player.y,
                    self.gems
                );
                let text_map = self.surface_view_map();
                self.active_view_overlay = Some(ViewOverlay {
                    title: title.clone(),
                    text_map: text_map.clone(),
                    kind: ViewOverlayKind::Surface,
                    mode: ViewOverlayMode::GemView,
                });
                self.message.clear();
                MoveOutcome::Observed
            }
        }
    }

    pub fn clear_active_view_overlay(&mut self) {
        self.active_view_overlay = None;
        self.message.clear();
    }

    pub fn render_active_view_overlay(&self, depth: TileGraphicsDepth) -> Option<TileViewport> {
        let overlay = self.active_view_overlay.as_ref()?;
        match overlay.kind {
            ViewOverlayKind::Surface => {
                Some(self.render_surface_view_overlay_for_mode(depth, overlay.mode))
            }
            ViewOverlayKind::Sky(sky) => {
                Some(render_sky_overlay(depth, &sky, self.shadowlord_hideouts))
            }
            ViewOverlayKind::Dungeon { level } => {
                Some(self.render_dungeon_view_overlay_for_mode(level, depth, overlay.mode))
            }
        }
    }

    pub fn render_surface_view_overlay(&self, depth: TileGraphicsDepth) -> TileViewport {
        self.render_surface_view_overlay_for_mode(depth, ViewOverlayMode::GemView)
    }

    pub fn render_surface_view_overlay_for_mode(
        &self,
        depth: TileGraphicsDepth,
        mode: ViewOverlayMode,
    ) -> TileViewport {
        let cells = LOCAL_VIEW_OVERLAY_SIDE;
        let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let width = cells * scale;
        let mut viewport = TileViewport {
            depth,
            cells_wide: cells,
            cells_high: cells,
            width,
            height: width,
            pixels: vec![0; width * width],
        };
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let tile = self.surface_view_tile_at(cell_x, cell_y);
                let class = surface_view_class(tile);
                draw_surface_view_cell(
                    &mut viewport,
                    cell_x,
                    cell_y,
                    scale,
                    class,
                    tile,
                    false,
                    mode,
                );
            }
        }
        draw_surface_view_cell(&mut viewport, cells / 2, cells / 2, scale, 0, 0, true, mode);
        viewport
    }

    pub fn render_surface_view_class_cell_for_mode(
        depth: TileGraphicsDepth,
        class: u8,
        tile: u8,
        player_marker: bool,
        mode: ViewOverlayMode,
    ) -> TileViewport {
        let scale = LOCAL_VIEW_CELL_PIXEL_SCALE;
        let mut viewport = TileViewport {
            depth,
            cells_wide: 1,
            cells_high: 1,
            width: scale,
            height: scale,
            pixels: vec![0; scale * scale],
        };
        draw_surface_view_cell(&mut viewport, 0, 0, scale, class, tile, player_marker, mode);
        viewport
    }

    fn surface_view_tile_at(&self, cell_x: usize, cell_y: usize) -> u8 {
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let side = LOCAL_VIEW_OVERLAY_SIDE as isize;
        let x = px - side / 2 + cell_x as isize;
        let y = py - side / 2 + cell_y as isize;
        match self.area {
            Area::Town { .. } => {
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    return 0;
                }
                if let Some(object) = self.object_at_current_floor(x as usize, y as usize) {
                    object.tile
                } else {
                    let tile = self.grid[y as usize * 32 + x as usize];
                    self.animation.resolve_static_tile(tile)
                }
            }
            Area::World { .. } => {
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                if let Some(object) = self.world_object_at(wx, wy) {
                    object.tile
                } else {
                    let tile = self.grid[world_cell_index(wx, wy)];
                    self.animation.resolve_static_tile(tile)
                }
            }
            Area::Dungeon { .. } => 0,
        }
    }

    pub fn render_dungeon_view_overlay(&self, level: u8, depth: TileGraphicsDepth) -> TileViewport {
        self.render_dungeon_view_overlay_for_mode(level, depth, ViewOverlayMode::GemView)
    }

    /// `view.md §6.3`: "the value being read is the display adapter
    /// identifier, not a peer-spell flag. The dungeon map renderer has no
    /// peer-spell branch." The mode is accepted so callers can stay uniform
    /// with the surface overlay, but every mode paints the same dungeon map
    /// for a given adapter `depth`.
    pub fn render_dungeon_view_overlay_for_mode(
        &self,
        level: u8,
        depth: TileGraphicsDepth,
        _mode: ViewOverlayMode,
    ) -> TileViewport {
        let cells = DUNGEON_GEM_VIEW_GRID_SIDE;
        let scale = DUNGEON_GEM_VIEW_CELL_PIXELS;
        let width = cells * scale;
        let mut viewport = TileViewport {
            depth,
            cells_wide: cells,
            cells_high: cells,
            width,
            height: width,
            pixels: vec![0; width * width],
        };
        let glyphs = self.dungeon_vision_glyphs(level);
        for cell_y in 0..cells {
            for cell_x in 0..cells {
                let index = cell_y * cells + cell_x;
                draw_dungeon_view_glyph(&mut viewport, cell_x, cell_y, scale, glyphs[index]);
            }
        }
        viewport
    }

    /// See [`PlayState::render_dungeon_view_overlay_for_mode`]: the dungeon
    /// map renderer has no peer-spell branch, so `_mode` does not select a
    /// pen.
    pub fn render_dungeon_view_glyph_cell_for_mode(
        depth: TileGraphicsDepth,
        glyph: Option<DungeonMinimapGlyph>,
        _mode: ViewOverlayMode,
    ) -> TileViewport {
        let scale = DUNGEON_GEM_VIEW_CELL_PIXELS;
        let mut viewport = TileViewport {
            depth,
            cells_wide: 1,
            cells_high: 1,
            width: scale,
            height: scale,
            pixels: vec![0; scale * scale],
        };
        draw_dungeon_view_glyph(&mut viewport, 0, 0, scale, glyph);
        viewport
    }

    pub fn dungeon_vision_glyphs(&self, level: u8) -> Vec<Option<DungeonMinimapGlyph>> {
        let side = DUNGEON_GEM_VIEW_GRID_SIDE;
        let (party_cell_x, party_cell_y) = DUNGEON_GEM_VIEW_PARTY_CELL;
        // The grid side is even, so the party cell is not a centre:
        // offsets run `-party_cell` through `side - party_cell - 1`,
        // i.e. eleven cells left/above and ten right/below.
        let min_dx = -(party_cell_x as isize);
        let max_dx = (side - party_cell_x - 1) as isize;
        let min_dy = -(party_cell_y as isize);
        let max_dy = (side - party_cell_y - 1) as isize;
        let mut visible = vec![false; side * side];
        let mut queue = VecDeque::with_capacity(DUNGEON_GEM_VIEW_FRONTIER_CAPACITY);

        let party_index = party_cell_y * side + party_cell_x;
        visible[party_index] = true;
        queue.push_back((0isize, 0isize));

        while let Some((sx, sy)) = queue.pop_front() {
            let world_x = (self.player.x as isize + sx).rem_euclid(DUNGEON_SIDE as isize) as usize;
            let world_y = (self.player.y as isize + sy).rem_euclid(DUNGEON_SIDE as isize) as usize;
            if (sx != 0 || sy != 0)
                && !dungeon_minimap_expands(self.dungeon_cell(level, world_x, world_y))
            {
                continue;
            }

            for dy in -1..=1 {
                for dx in -1..=1 {
                    if dx == 0 && dy == 0 {
                        continue;
                    }
                    let next_x = sx + dx;
                    let next_y = sy + dy;
                    if next_x < min_dx || next_x > max_dx || next_y < min_dy || next_y > max_dy {
                        continue;
                    }
                    let scratch_x = (next_x + party_cell_x as isize) as usize;
                    let scratch_y = (next_y + party_cell_y as isize) as usize;
                    let index = scratch_y * side + scratch_x;
                    if !visible[index] {
                        visible[index] = true;
                        queue.push_back((next_x, next_y));
                        debug_assert!(
                            queue.len() <= DUNGEON_GEM_VIEW_FRONTIER_CAPACITY,
                            "dungeon-mode.md §12.2 bounds the V-View flood frontier at                              {DUNGEON_GEM_VIEW_FRONTIER_CAPACITY} pending cells;                              reached {}",
                            queue.len()
                        );
                    }
                }
            }
        }

        let mut glyphs = vec![None; side * side];
        glyphs[party_index] = Some(DUNGEON_MINIMAP_PARTY_GLYPH);
        for scratch_y in 0..side {
            for scratch_x in 0..side {
                let index = scratch_y * side + scratch_x;
                if !visible[index] || index == party_index {
                    continue;
                }
                let dx = scratch_x as isize - party_cell_x as isize;
                let dy = scratch_y as isize - party_cell_y as isize;
                let world_x =
                    (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
                let world_y =
                    (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
                glyphs[index] = dungeon_minimap_glyph(self.dungeon_cell(level, world_x, world_y));
            }
        }
        glyphs
    }

    pub fn dungeon_vision_map(&self, level: u8) -> String {
        let side = DUNGEON_GEM_VIEW_GRID_SIDE;
        let glyphs = self.dungeon_vision_glyphs(level);
        let mut out = String::new();
        for scratch_y in 0..side {
            for scratch_x in 0..side {
                let index = scratch_y * side + scratch_x;
                out.push(render_dungeon_minimap_glyph_code(glyphs[index]));
            }
            out.push('\n');
        }
        out
    }

    pub fn dungeon_forward_view(&self, level: u8) -> String {
        let mut out = String::from("First-person dungeon view:\n");
        out.push_str(&format!(
            "0: here {}\n",
            self.describe_dungeon_offset(level, 0, 0)
        ));

        let Some(left) = self.player.facing.turn_left_cardinal() else {
            out.push_str("view requires a cardinal facing direction\n");
            return out;
        };
        let Some(right) = self.player.facing.turn_right_cardinal() else {
            out.push_str("view requires a cardinal facing direction\n");
            return out;
        };

        let (fdx, fdy) = self.player.facing.delta();
        let (ldx, ldy) = left.delta();
        let (rdx, rdy) = right.delta();
        let mut obscured = false;
        for band in 1..=DUNGEON_VIEW_DEPTH {
            if obscured {
                out.push_str(&format!("{band}: obscured by front wall\n"));
                continue;
            }

            let band = band as isize;
            let ahead_dx = fdx * band;
            let ahead_dy = fdy * band;
            out.push_str(&format!(
                "{band}: ahead {}; left {}; right {}\n",
                self.describe_dungeon_offset(level, ahead_dx, ahead_dy),
                self.describe_dungeon_offset(level, ahead_dx + ldx, ahead_dy + ldy),
                self.describe_dungeon_offset(level, ahead_dx + rdx, ahead_dy + rdy)
            ));
            obscured = self.dungeon_offset_blocks_view(level, ahead_dx, ahead_dy);
        }

        out
    }

    pub fn describe_dungeon_offset(&self, level: u8, dx: isize, dy: isize) -> String {
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y) {
            "the dungeon boundary".to_string()
        } else {
            dungeon_look_description(self.dungeon_cell(level, x as usize, y as usize)).to_string()
        }
    }

    pub fn dungeon_offset_blocks_view(&self, level: u8, dx: isize, dy: isize) -> bool {
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&x) || !(0..DUNGEON_SIDE as isize).contains(&y) {
            return true;
        }

        !is_dungeon_walkable(self.dungeon_cell(level, x as usize, y as usize))
    }

    pub fn surface_view_map(&self) -> String {
        let mut out = String::new();
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        let side = 32isize;
        let origin_x = px - side / 2;
        let origin_y = py - side / 2;
        match self.area {
            Area::Town { .. } => {
                for y in origin_y..origin_y + side {
                    for x in origin_x..origin_x + side {
                        if x == px && y == py {
                            out.push('@');
                        } else if (0..32).contains(&x) && (0..32).contains(&y) {
                            if let Some(object) =
                                self.object_at_current_floor(x as usize, y as usize)
                            {
                                out.push(render_surface_view_class(surface_view_class(
                                    object.tile,
                                )));
                            } else {
                                let tile = self.grid[y as usize * 32 + x as usize];
                                let tile = self.animation.resolve_static_tile(tile);
                                out.push(render_surface_view_class(surface_view_class(tile)));
                            }
                        } else {
                            out.push(' ');
                        }
                    }
                    out.push('\n');
                }
            }
            Area::World { .. } => {
                for y in origin_y..origin_y + side {
                    for x in origin_x..origin_x + side {
                        let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                        let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                        if wx == self.player.x && wy == self.player.y {
                            out.push('@');
                        } else if let Some(object) = self.world_object_at(wx, wy) {
                            out.push(render_surface_view_class(surface_view_class(object.tile)));
                        } else {
                            let tile = self.grid[world_cell_index(wx, wy)];
                            let tile = self.animation.resolve_static_tile(tile);
                            out.push(render_surface_view_class(surface_view_class(tile)));
                        }
                    }
                    out.push('\n');
                }
            }
            Area::Dungeon { .. } => {}
        }
        out
    }

    #[cfg(test)]
    pub fn look_facing(&mut self) -> MoveOutcome {
        self.look_facing_with_table(None)
    }

    pub fn look_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.look_direction_with_game_dir(self.player.facing, game_dir)
    }

    pub fn look_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let look_table = load_look_table(game_dir)?;
        self.look_direction_with_resources(direction, Some(&look_table), Some(game_dir))
    }

    #[cfg(test)]
    pub fn look_facing_with_table(&mut self, look_table: Option<&LookTable>) -> MoveOutcome {
        self.look_direction_with_resources(self.player.facing, look_table, None)
            .expect("look without a game dir cannot perform file-backed look context")
    }

    #[cfg(test)]
    pub fn look_direction_with_table(
        &mut self,
        direction: Direction,
        look_table: Option<&LookTable>,
    ) -> MoveOutcome {
        self.look_direction_with_resources(direction, look_table, None)
            .expect("look without a game dir cannot perform file-backed look context")
    }

    pub fn look_facing_with_resources(
        &mut self,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        self.look_direction_with_resources(self.player.facing, look_table, game_dir)
    }

    pub fn look_direction_with_resources(
        &mut self,
        direction: Direction,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Dungeon { .. } => Ok(self.look_dungeon()),
            Area::Town { .. } => {
                let (dx, dy) = direction.delta();
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    // The border ring is the location's exit trigger, so a
                    // look cannot be aimed off the grid from inside it.
                    self.message.clear();
                    return Ok(MoveOutcome::Observed);
                }
                if !self.surface_look_target_visible(x, y) {
                    self.message.clear();
                    return Ok(MoveOutcome::Observed);
                }
                let x = x as usize;
                let y = y as usize;
                // `view.md §3` entry-dispatch row 2: "Live tile `0x29`
                // (the crystal-sphere tile)" is tested immediately after
                // the row-1 visibility gate and before the row-3 shared
                // preamble and the row-4 per-map object lookup, because
                // "the vision case is decided before anything is
                // printed". The tested byte is the **live terrain
                // layer**, never an active-object descriptor.
                if death_vision_look_tile(self.grid[y * 32 + x]) {
                    return Ok(self.apply_death_vision_look(x, y));
                }
                if let Some(object) = self.blocking_object_at(x, y) {
                    if sign_or_wanted_poster_object_class(object.type_byte) {
                        self.message = self
                            .sign_message_at_for_current_area(game_dir, y as u8, x as u8)?
                            .unwrap_or_else(|| "Sign:\n".to_string());
                        return Ok(MoveOutcome::Observed);
                    }
                    // `cleak/u5-spec#194` (black-box): the stock game prints
                    // `Thou dost see` and the description on the next row.
                    self.message = if look_table.is_some() {
                        format!(
                            "{LOOK_RESULT_PREFIX}\n{}",
                            self.look_object_description(object.tile, look_table)
                        )
                    } else {
                        format!("{LOOK_RESULT_PREFIX}\nan actor tile {}", object.tile)
                    };
                    return Ok(MoveOutcome::Observed);
                }
                if let Area::Town { scene, floor } = self.area {
                    if let Some(sign) =
                        self.sign_message_at(game_dir, scene.byte, floor as u8, y as u8, x as u8)?
                    {
                        self.message = sign;
                        return Ok(MoveOutcome::Observed);
                    }
                }
                let tile = self.grid[y * 32 + x];
                // `view.md §3` terrain-description path row 2: live tile
                // `0x59` is a **telescope** and enters the §4.2 sky renderer
                // instead of printing any description text. The row sits
                // above the wishing well (row 3) and the fountains (row 4),
                // and the table is not scene-scoped: "Only three telescopes
                // are placed in shipped data, all indoors: in Moonglow, in
                // Skara Brae, and in West Britanny", so the town-family arm
                // is the only arm that can ever reach it in ordinary play.
                if tile == TELESCOPE_LOOK_TRIGGER_TILE {
                    self.look_through_telescope();
                    return Ok(MoveOutcome::Observed);
                }
                if surface_wishing_well_look_tile(tile) {
                    // Measured: the well takes the ordinary `Thou dost see`
                    // preamble and then its **own** description, not the
                    // `LOOK2.DAT` record for the tile. See
                    // [`WISHING_WELL_LOOK_DESCRIPTION`].
                    return Ok(
                        self.start_wishing_well_prompt(direction, WISHING_WELL_LOOK_DESCRIPTION)
                    );
                }
                if surface_town_fountain_look_tile(tile) {
                    return Ok(self.start_surface_fountain_drink_prompt(direction));
                }
                self.message = format!(
                    "{LOOK_RESULT_PREFIX}\n{}",
                    self.look_description(tile, look_table)
                );
                Ok(MoveOutcome::Observed)
            }
            Area::World { plane } => {
                let (dx, dy) = direction.delta();
                let raw_x = self.player.x as isize + dx;
                let raw_y = self.player.y as isize + dy;
                if !self.surface_look_target_visible(raw_x, raw_y) {
                    self.message.clear();
                    return Ok(MoveOutcome::Observed);
                }
                let x = raw_x.rem_euclid(WORLD_SIDE as isize) as usize;
                let y = raw_y.rem_euclid(WORLD_SIDE as isize) as usize;
                // `view.md §3` entry-dispatch row 2 — see the town arm
                // above. The live terrain byte is tested ahead of the
                // per-map object row.
                if death_vision_look_tile(self.grid[world_cell_index(x, y)]) {
                    return Ok(self.apply_death_vision_look(x, y));
                }
                if let Some(object) = self.world_object_at(x, y) {
                    if sign_or_wanted_poster_object_class(object.type_byte) {
                        self.message = self
                            .sign_message_at_for_current_area(game_dir, y as u8, x as u8)?
                            .unwrap_or_else(|| "Sign:\n".to_string());
                        return Ok(MoveOutcome::Observed);
                    }
                    self.message = if look_table.is_some() {
                        format!(
                            "{LOOK_RESULT_PREFIX}\n{}",
                            self.look_object_description(object.tile, look_table)
                        )
                    } else {
                        format!("{LOOK_RESULT_PREFIX}\nan object tile {}", object.tile)
                    };
                    return Ok(MoveOutcome::Observed);
                }
                let tile = self.grid[world_cell_index(x, y)];
                if tile == TELESCOPE_LOOK_TRIGGER_TILE {
                    self.look_through_telescope();
                    return Ok(MoveOutcome::Observed);
                }
                if let Some(sign) = self.sign_message_at(
                    game_dir,
                    SCENE_OVERWORLD,
                    plane.save_floor() as u8,
                    y as u8,
                    x as u8,
                )? {
                    self.message = sign;
                    return Ok(MoveOutcome::Observed);
                }
                if surface_wishing_well_look_tile(tile) {
                    return Ok(
                        self.start_wishing_well_prompt(direction, WISHING_WELL_LOOK_DESCRIPTION)
                    );
                }
                if surface_town_fountain_look_tile(tile) {
                    return Ok(self.start_surface_fountain_drink_prompt(direction));
                }
                let description =
                    self.look_description_for_world_tile(tile, look_table, game_dir, plane, x, y)?;
                // `view.md §3`: "the shared \"thou dost see\" preamble is
                // printed here" with the description on the next row, and
                // `commands.md §8.1` has the look family never print "tile
                // ids, coordinates, active-object slot numbers, terrain-class
                // names". The town and dungeon arms above already do this; the
                // overworld arm was still emitting the terminal harness's
                // inline form, `You see: mountains at (240, 72) on
                // BRITANNIA.`, where a paired capture of Deceit's exit reads
                // `Thou dost see` / `mountains` (`cleak/u5-engine#5`).
                self.message = format!("{LOOK_RESULT_PREFIX}\n{description}");
                Ok(MoveOutcome::Observed)
            }
        }
    }

    pub fn look_surface_fountain_with_drinker(
        &mut self,
        direction: Direction,
        member_index: usize,
    ) -> MoveOutcome {
        let Some(tile) = self.surface_look_target_tile(direction) else {
            // An internal invariant: the drink prompt only opens on a
            // fountain the look already matched, and the picker bounds the
            // member index, so no shipped path reaches this. The original
            // prints nothing here, so neither does this.
            self.message.clear();
            return MoveOutcome::Observed;
        };
        if !surface_town_fountain_look_tile(tile) {
            self.message.clear();
            return MoveOutcome::Observed;
        }

        let Some(member) = self.party.get(member_index).copied() else {
            self.message.clear();
            return MoveOutcome::Observed;
        };
        let Some(status) = character_status_for_byte(member.status) else {
            self.message.clear();
            return MoveOutcome::Observed;
        };
        if town_fountain_drink_accepts(status) {
            // The prompt carries no trailing space, so the reply opens its
            // own line rather than completing the prompt's.
            self.message = FOUNTAIN_DRINK_REFRESHED.to_string();
        } else {
            // `view.md §3`, since `cleak/u5-spec#197` published the
            // literals: "A Dead or Asleep member prints
            // `Incapacitated!` and a blank row". The engine printed
            // nothing here while the line was unpublished.
            self.message = FOUNTAIN_DRINK_INCAPACITATED.to_string();
        }
        MoveOutcome::Observed
    }

    pub fn resolve_wishing_well_wish(
        &mut self,
        direction: Direction,
        typed_wish: &str,
    ) -> MoveOutcome {
        let Some((x, y)) = self.surface_look_target_position(direction) else {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        };
        let tile = self.surface_tile_at(x, y);
        if !surface_wishing_well_look_tile(tile) {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        }
        let Some(wish) = wishing_well_wish(typed_wish) else {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        };
        let grant_scene = match self.area {
            Area::Town { scene, .. } => wishing_well_grant_scene(scene.byte),
            _ => false,
        };
        if !grant_scene {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        }
        let _ = wish;
        let grant_x = self.player.x.saturating_add(1);
        let grant_y = self.player.y;
        if grant_x >= 32 || self.object_at_current_floor(grant_x, grant_y).is_some() {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        }
        let Some(z) = self.current_floor() else {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        };
        if self
            .allocate_active_object_slot(horse_purchase_active_object(grant_x, grant_y, z))
            .is_none()
        {
            self.message = format!(
                "{}\n{WISHING_WELL_NO_EFFECT_LINE}",
                typed_wish.to_ascii_uppercase()
            );
            return MoveOutcome::Observed;
        }

        self.mark_visibility_dirty();
        // Measured: the typed wish echoes upper-cased on its own row and
        // the grant prints [`WISHING_WELL_GRANT_LINE`] under it.
        self.message = format!(
            "{}\n{WISHING_WELL_GRANT_LINE}",
            typed_wish.to_ascii_uppercase()
        );
        MoveOutcome::Observed
    }

    /// `view.md §3` entry-dispatch row 2. Measured 2026-09-07: the command
    /// takes no party-member prompt - it rolls for the active player, and
    /// prints one of two one-line results.
    pub fn apply_death_vision_look(&mut self, x: usize, y: usize) -> MoveOutcome {
        let member_index = match self.shared_acting_member_selection(false) {
            ActingMemberSelection::Selected(slot) => slot,
            _ => 0,
        };
        self.apply_death_vision_look_for_member(x, y, member_index)
    }

    pub fn apply_death_vision_look_for_member(
        &mut self,
        x: usize,
        y: usize,
        member_index: usize,
    ) -> MoveOutcome {
        let intelligence = self
            .party_intelligence
            .get(member_index)
            .copied()
            .unwrap_or(self.avatar_stats.intelligence);
        let roll = self.random_range_u8(DEATH_VISION_ROLL_LOW, DEATH_VISION_ROLL_HIGH);
        if intelligence > roll {
            let text_map = self.surface_view_map();
            self.active_view_overlay = Some(ViewOverlay {
                title: String::new(),
                text_map,
                kind: ViewOverlayKind::Surface,
                mode: ViewOverlayMode::SurfaceLook,
            });
            self.message = DEATH_VISION_STRANGE_LINE.to_string();
        } else {
            let _ = (x, y);
            self.active_view_overlay = None;
            self.message = DEATH_VISION_DEATH_LINE.to_string();
        }
        MoveOutcome::Observed
    }

    fn surface_look_target_tile(&self, direction: Direction) -> Option<u8> {
        let (x, y) = self.surface_look_target_position(direction)?;
        Some(self.surface_tile_at(x, y))
    }

    fn surface_look_target_position(&self, direction: Direction) -> Option<(usize, usize)> {
        match self.area {
            Area::Town { .. } => self.adjacent_position(direction),
            Area::World { .. } => {
                let (dx, dy) = direction.delta();
                let x = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let y = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                Some((x, y))
            }
            Area::Dungeon { .. } => None,
        }
    }

    fn surface_look_target_visible(&self, x: isize, y: isize) -> bool {
        let px = self.player.x as isize;
        let py = self.player.y as isize;
        match self.area {
            Area::Town { .. } => {
                if self.surface_visibility_pitch_dark() {
                    return false;
                }
                self.town_cell_visible_with_light_threshold(
                    px,
                    py,
                    x,
                    y,
                    SURFACE_LOOK_VISIBILITY_RADIUS,
                    self.surface_visibility_light_threshold(),
                )
            }
            Area::World { .. } => {
                if self.world_visibility_pitch_dark() {
                    return false;
                }
                self.world_cell_visible_with_light_threshold(
                    px,
                    py,
                    x,
                    y,
                    SURFACE_LOOK_VISIBILITY_RADIUS,
                    self.world_visibility_light_threshold(),
                )
            }
            Area::Dungeon { .. } => false,
        }
    }

    fn surface_tile_at(&self, x: usize, y: usize) -> u8 {
        match self.area {
            Area::Town { .. } => self.grid[y * 32 + x],
            Area::World { .. } => self.grid[world_cell_index(x, y)],
            Area::Dungeon { .. } => unreachable!("surface helper is not used in dungeon mode"),
        }
    }

    pub fn look_description(&self, tile: u8, look_table: Option<&LookTable>) -> String {
        let base = look_table
            .and_then(|table| {
                table.description(tile as usize).filter(|description| {
                    !description.is_empty() && !table.is_sentinel(description)
                })
            })
            .map(str::to_string)
            .unwrap_or_else(|| tile_class(tile).to_string());

        if matches!(tile, 0xfa | 0xfb) {
            format!(
                "{base} ({}:{:02} {})",
                self.clock.display_hour(),
                self.clock.minute,
                self.clock.am_pm_suffix()
            )
        } else if tile == ETERNAL_FLAME_LOOK_TILE {
            // `view.md §3` terrain-description row 5b: "Live tile
            // `0xDE` | Append a virtue word chosen by the current
            // scene: scene `30` appends Truth, scene `31` appends Love,
            // scene `32` appends Courage. In any other scene the base
            // description is printed with no appended word." The row is
            // an appender row even when it appends nothing, so it
            // returns here rather than falling through.
            let scene_byte = match self.area {
                Area::Town { scene, .. } => scene.byte,
                _ => SCENE_OVERWORLD,
            };
            match eternal_flame_word_for_scene(scene_byte) {
                Some(word) => format!("{base} {word}"),
                None => base,
            }
        } else if let Some(virtue) = shrine_virtue_for_altar_tile(tile) {
            format!("{base} (Shrine of {})", virtue.name())
        } else {
            base
        }
    }

    pub fn look_object_description(&self, object_id: u8, look_table: Option<&LookTable>) -> String {
        look_table
            .and_then(|table| {
                table
                    .description(LOOK2_DAT_TERRAIN_ENTRIES + object_id as usize)
                    .filter(|description| {
                        !description.is_empty() && !table.is_sentinel(description)
                    })
            })
            .map(str::to_string)
            .unwrap_or_else(|| tile_class(object_id).to_string())
    }

    pub fn sign_message_at(
        &self,
        game_dir: Option<&Path>,
        scene: u8,
        z: u8,
        y: u8,
        x: u8,
    ) -> io::Result<Option<String>> {
        if let Some(message) = self.yew_wanted_poster_message(scene, z, y, x) {
            return Ok(Some(message));
        }
        let Some(game_dir) = game_dir else {
            return Ok(None);
        };
        let Some(records) = load_sign_records(game_dir)? else {
            return Ok(None);
        };
        let bodies = crate::signs_io::matching_sign_bodies(&records, scene, z, y, x);
        if bodies.is_empty() {
            return Ok(None);
        }
        Ok(Some(format!("Sign:\n{}", bodies.join("\n"))))
    }

    fn yew_wanted_poster_message(&self, scene: u8, z: u8, y: u8, x: u8) -> Option<String> {
        if !(scene == 4 && z == 0 && x == 17 && y == 21) {
            return None;
        }
        Some(yew_wanted_poster_rows(&self.party_names).join("\n"))
    }

    pub fn sign_message_at_for_current_area(
        &self,
        game_dir: Option<&Path>,
        y: u8,
        x: u8,
    ) -> io::Result<Option<String>> {
        match self.area {
            Area::Town { scene, floor } => {
                self.sign_message_at(game_dir, scene.byte, floor as u8, y, x)
            }
            Area::World { plane } => {
                self.sign_message_at(game_dir, SCENE_OVERWORLD, plane.save_floor() as u8, y, x)
            }
            Area::Dungeon { .. } => Ok(None),
        }
    }

    pub fn look_description_for_world_tile(
        &self,
        tile: u8,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<String> {
        let base = self.look_description(tile, look_table);
        if tile != 0xdf {
            if let Some(virtue) = self.world_shrine_virtue_at(game_dir, plane, x, y, tile)? {
                let shrine_name = format!("Shrine of {}", virtue.name());
                if base.contains(&shrine_name) {
                    return Ok(base);
                }
                return Ok(format!("{base} ({shrine_name})"));
            }
            return Ok(base);
        }
        let Some(name) = self.world_dungeon_name_at(game_dir, plane, x, y, tile)? else {
            return Ok(base);
        };
        Ok(format!("{base} ({name})"))
    }

    pub fn world_shrine_virtue_at(
        &self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<ShrineVirtue>> {
        if plane != WorldPlane::Britannia {
            return Ok(None);
        }
        if let Some(game_dir) = game_dir {
            if let Some(entries) = load_shrine_entries(game_dir)? {
                if let Some(entry) = entries.into_iter().find(|entry| {
                    entry.plane == plane
                        && entry.x == x
                        && entry.y == y
                        && entry
                            .expected_tile
                            .map_or(true, |expected| expected == tile)
                }) {
                    return Ok(Some(entry.virtue));
                }
            }
        }
        Ok(shrine_virtue_for_altar_tile(tile))
    }

    pub fn world_dungeon_name_at(
        &self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<&'static str>> {
        let Some(game_dir) = game_dir else {
            return Ok(None);
        };
        Ok(effective_world_location_entries(game_dir)?
            .into_iter()
            .find_map(|entry| {
                if entry.plane == plane
                    && entry.x == x
                    && entry.y == y
                    && entry
                        .expected_tile
                        .map_or(true, |expected| expected == tile)
                {
                    match entry.target {
                        PlayTarget::Dungeon(scene) => Some(scene.name()),
                        _ => None,
                    }
                } else {
                    None
                }
            }))
    }

    pub fn talk_facing_with_game_dir(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        self.talk_facing_with_game_dir_and_keyword(game_dir, None)
    }

    pub fn talk_facing_with_game_dir_and_keyword(
        &mut self,
        game_dir: &Path,
        keyword: Option<&str>,
    ) -> io::Result<MoveOutcome> {
        self.talk_direction_with_game_dir_and_keyword(self.player.facing, game_dir, keyword)
    }

    /// `shops.md §2` open-for-business gate: both the keeper's **cached
    /// reached** waypoint and the waypoint its schedule selects for the
    /// current hour must be waypoint 1.
    ///
    /// A slot this engine cannot resolve - no runtime NPC at the cell, which
    /// the harness fixtures produce - is treated as open, so unit fixtures
    /// that seat a trigger without a schedule keep working.
    fn shop_keeper_is_open_for_business(&self, x: usize, y: usize) -> bool {
        const SHOP_WORKING_WAYPOINT: usize = 1;
        let Some(npc) = self.npc_at_current_floor(x, y) else {
            return true;
        };
        npc.cached_wp == SHOP_WORKING_WAYPOINT
            && waypoint_for_hour(&npc.schedule, self.clock.hour) == SHOP_WORKING_WAYPOINT
    }

    pub fn talk_direction_with_game_dir(
        &mut self,
        direction: Direction,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        self.talk_direction_with_game_dir_and_keyword(direction, game_dir, None)
    }

    pub fn talk_direction_with_game_dir_and_keyword(
        &mut self,
        direction: Direction,
        game_dir: &Path,
        keyword: Option<&str>,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, .. } = self.area else {
            self.message = "Funny, no response!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if self.talk_liveness_blocked() {
            return Ok(self.consume_ordinary_town_talk());
        }
        if let Some((dialog_id, target_x, target_y)) = self.talk_target_in_direction(direction)
            && matches!(
                dialog_id,
                TOWN_NPC_COWERING_DIALOG_ID | TOWN_NPC_BRUSHOFF_DIALOG_ID
            )
        {
            if let Some(refusal) =
                talk_status_tile_refusal(self.talk_status_tile_at(target_x, target_y))
            {
                self.message = format!("\n{refusal}");
                return Ok(self.consume_ordinary_town_talk());
            }
            return Ok(self
                .talk_alarm_sentinel_at(dialog_id, target_x, target_y)
                .expect("published alarm sentinel was preclassified"));
        }
        let dialogue = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let raw_blob = parse_tlk_raw(&game_dir.join(format!("{}.TLK", scene.family.stem())))
            .unwrap_or_default();
        self.common_word_dictionary = load_common_word_dictionary_optional(game_dir)?;
        let shoppe_renderer =
            crate::shoppe_bark::ShoppeTextRenderer::load_from_game_dir(game_dir).ok();
        Ok(self.talk_direction_with_dialogue_and_keyword_raw_inner(
            direction,
            &dialogue,
            &raw_blob,
            keyword,
            shoppe_renderer.as_ref(),
        ))
    }

    pub fn facing_talk_target(&self) -> Option<(u8, usize, usize)> {
        self.talk_target_in_direction(self.player.facing)
    }

    /// `conversation.md §2` step 4: the byte the Talk status-tile gate
    /// compares against `0x9D` (mirror) and `0xAB` (bed).
    ///
    /// "The test object is a map tile, not an NPC sprite. Both ids are
    /// furniture ids in the terrain-description domain of `LOOK2.DAT`, and the
    /// byte comes from the same live-map tile query that movement and Look
    /// use. The gate fires because the NPC's schedule has parked it on a bed
    /// cell or a mirror cell, not because the NPC has a distinct sleeping or
    /// praying appearance. An implementation that stores a per-NPC 'asleep'
    /// flag and tests that instead will diverge."
    ///
    /// The cell is the *resolved* cell — the faced cell, or the cell one step
    /// further along the same direction when the faced cell was talk-through —
    /// which is exactly what [`PlayState::talk_target_in_direction`] returns.
    pub fn talk_status_tile_at(&self, x: usize, y: usize) -> u8 {
        self.grid[y * 32 + x]
    }

    /// The Look description of whatever Talk resolved onto, if the
    /// cached `LOOK2.DAT` table is present. Shares
    /// [`Self::look_object_description`] with the Look command so the two
    /// cannot drift.
    pub fn talk_target_description(&self, x: usize, y: usize) -> Option<String> {
        let table = self.look_table.as_ref()?;
        let object = self.blocking_object_at(x, y)?;
        Some(self.look_object_description(object.tile, Some(table)))
    }

    pub fn talk_target_in_direction(&self, direction: Direction) -> Option<(u8, usize, usize)> {
        let (dx, dy) = direction.delta();
        let x = self.player.x as isize + dx;
        let y = self.player.y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            return None;
        }

        let x = x as usize;
        let y = y as usize;
        if let Some(npc) = self.npc_at_current_floor(x, y) {
            return Some((npc.dialog_id, x, y));
        }
        if !is_talk_through_tile(self.grid[y * 32 + x]) {
            return None;
        }

        let x = x as isize + dx;
        let y = y as isize + dy;
        if !(0..32).contains(&x) || !(0..32).contains(&y) {
            return None;
        }
        let x = x as usize;
        let y = y as usize;
        self.npc_at_current_floor(x, y)
            .map(|npc| (npc.dialog_id, x, y))
    }

    #[cfg(test)]
    pub fn talk_facing_with_dialogue(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
    ) -> MoveOutcome {
        self.talk_facing_with_dialogue_and_keyword(dialogue, None)
    }

    pub fn talk_facing_with_dialogue_and_keyword(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword(self.player.facing, dialogue, keyword)
    }

    /// The legacy dialogue-map entry point. Production talk goes through the
    /// conversation runner (`talk_facing_with_game_dir`); nothing outside the
    /// tests reaches this chain, so the lines it composes are harness output
    /// rather than text the game prints.
    pub fn talk_direction_with_dialogue_and_keyword(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword_inner(direction, dialogue, keyword)
    }

    pub fn talk_direction_with_dialogue_and_keyword_inner(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        if !matches!(self.area, Area::Town { .. }) {
            self.message = "Funny, no response!".to_string();
            return MoveOutcome::Blocked;
        }
        if self.talk_liveness_blocked() {
            return self.consume_ordinary_town_talk();
        }

        let Some((dialog_id, target_x, target_y)) = self.talk_target_in_direction(direction) else {
            self.message = crate::TALK_NOBODY_HERE_LINE.to_string();
            return self.consume_ordinary_town_talk();
        };
        // `conversation.md §2` step 4 status-tile filter: the mirror
        // (`0x9D`) and bed (`0xAB`) gate reads the **live map tile occupying
        // the resolved cell**, not a per-NPC sprite or asleep value.
        if let Some(refusal) =
            talk_status_tile_refusal(self.talk_status_tile_at(target_x, target_y))
        {
            self.message = format!("\n{refusal}");
            return self.consume_ordinary_town_talk();
        }
        if let Some(outcome) = self.talk_alarm_sentinel_at(dialog_id, target_x, target_y) {
            return outcome;
        }

        if let Some((_role, _family)) = talk_shop_trigger(dialog_id) {
            // `shops.md §2` open-for-business gate: "Both the NPC's cached
            // reached waypoint and the waypoint selected by the current world
            // hour must be **waypoint 1**." The cache records arrival, so a
            // keeper who has not reached the working waypoint refuses even
            // after opening time, and one still there after closing refuses
            // too. It applies to all eight triggers, horse traders included,
            // and takes precedence over the transport refusal below.
            //
            // Measured 2026-09-07 at North Britanny's stable
            // (`qa/paired/nb-stable.tsv`), which refuses at 13:00 with
            // exactly the published lines. This engine used to open every
            // shop whose trigger byte it saw (`cleak/u5-engine#15`,
            // `cleak/u5-spec#214`).
            if !self.shop_keeper_is_open_for_business(target_x, target_y) {
                self.message = SHOP_CLOSED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            if self.player.transport.is_horse() && dialog_id != 0x83 {
                self.message = SHOP_MOUNTED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            let scene_byte = match self.area {
                Area::Town { scene, .. } => Some(scene.byte),
                _ => None,
            };
            if let Some(session) =
                crate::shop_session::shop_session_for_talk_context(dialog_id, scene_byte)
            {
                if matches!(&session, crate::shop_session::ActiveShopSession::Tavern(_)) {
                    self.tavern_secondary_drink_count = 0;
                }
                // `shops.md §8.1`: the post-item prompt's suffix reports
                // whether a transaction has completed "in this visit", so the
                // flag is per-visit and resets as the session opens.
                self.arms_transaction_completed = false;
                if matches!(
                    &session,
                    crate::shop_session::ActiveShopSession::Innkeeper(_)
                ) {
                    self.clear_active_effect_slot();
                }
                self.advance_turn();
                // Both Talk entries go through one greeting builder now. This
                // one is the dialogue-map path and carries no game directory,
                // so it takes the builder's no-assets fallback; the
                // asset-backed path renders the `SHOPPE.DAT` record.
                let message =
                    self.format_talk_shop_opening_message(dialog_id, &session, None, None);
                self.active_shop = Some(session);
                self.message = message;
                return MoveOutcome::Talked;
            }
        }
        if dialog_id == BLACKTHORN_GUARD_DEMAND_DIALOG_ID {
            return match self.begin_blackthorn_guard_demand(target_x, target_y, true, None) {
                Ok(outcome) => outcome,
                Err(_) => MoveOutcome::Blocked,
            };
        }
        if matches!(
            npc_dialog_id_kind(dialog_id),
            NpcDialogIdKind::NoDialogue | NpcDialogIdKind::HighSpecial
        ) {
            // `conversation.md §2`'s guard gate: the guard sprite answers
            // the stored `The guard offers no response!` literal and every
            // other non-speaker answers the bare `No response!`. The line "is
            // not composed from the NPC's Look description", which is what
            // this engine used to do off a single guard capture.
            self.message = crate::talk_non_speaker_refusal_for_sprite(
                self.npc_at_current_floor(target_x, target_y)
                    .map(|npc| npc.type_byte),
            );
            return self.consume_ordinary_town_talk();
        }

        let Some(fields) = dialogue.get(&(dialog_id as u16)) else {
            self.message = format!("Dialogue id {dialog_id} is unresolved for this scene."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        };
        if fields.len() < 3 {
            self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        }

        let name = fields
            .first()
            .filter(|name| !name.is_empty())
            .map(String::as_str)
            .unwrap_or("someone");
        let description = fields
            .get(1)
            .filter(|description| !description.is_empty())
            .map(String::as_str)
            .unwrap_or("no description");
        let greeting = fields
            .get(2)
            .filter(|greeting| !greeting.is_empty())
            .map(String::as_str)
            .unwrap_or("...");

        self.advance_turn();
        if let Some(keyword) = keyword.and_then(non_empty_talk_keyword) {
            if fields.len() < 5 {
                self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
                return MoveOutcome::Talked;
            }
            let response = talk_keyword_response(fields, keyword)
                .filter(|response| !response.is_empty())
                .unwrap_or(TLK_NO_KEYWORD_MATCH_MESSAGE);
            let (response, actions) = talk_response_text_and_actions(response);
            self.apply_talk_action_grants(&actions);
            // The same shape the production path prints: the reply alone.
            let _ = name;
            self.message = response.to_string();
        } else {
            let (greeting, actions) = talk_response_text_and_actions(greeting);
            self.apply_talk_action_grants(&actions);
            self.message = format!("{description}\n\n{greeting}\n\nYour interest?");
        }
        MoveOutcome::Talked
    }

    /// Talk dispatch wrapper that uses the byte-runner against the raw
    /// blob bytes when they are available, falling back to the existing
    /// string-based path otherwise. The richer renderer expands engine
    /// control bytes (printable text, action dispatch, IF/ELSE, GOTO
    /// labels) per `systems/conversation.md` §7 and merges active-scene
    /// branch-flag effects emitted by the stream.
    pub fn talk_facing_with_dialogue_and_keyword_raw(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword_raw(
            self.player.facing,
            dialogue,
            raw_blob,
            keyword,
        )
    }

    pub fn talk_direction_with_dialogue_and_keyword_raw(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
        keyword: Option<&str>,
    ) -> MoveOutcome {
        self.talk_direction_with_dialogue_and_keyword_raw_inner(
            direction, dialogue, raw_blob, keyword, None,
        )
    }

    fn talk_direction_with_dialogue_and_keyword_raw_inner(
        &mut self,
        direction: Direction,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
        keyword: Option<&str>,
        shoppe_renderer: Option<&crate::shoppe_bark::ShoppeTextRenderer>,
    ) -> MoveOutcome {
        if !matches!(self.area, Area::Town { .. }) {
            self.message = "Funny, no response!".to_string();
            return MoveOutcome::Blocked;
        }
        if self.talk_liveness_blocked() {
            return self.consume_ordinary_town_talk();
        }

        let Some((dialog_id, target_x, target_y)) = self.talk_target_in_direction(direction) else {
            self.message = crate::TALK_NOBODY_HERE_LINE.to_string();
            return self.consume_ordinary_town_talk();
        };
        let conversation_npc_slot = self
            .npc_at_current_floor(target_x, target_y)
            .map(|npc| npc.slot);

        // `conversation.md §2` step 4: the status-tile gate reads the live map
        // tile at the resolved cell, the same query movement and Look use.
        if let Some(refusal) =
            talk_status_tile_refusal(self.talk_status_tile_at(target_x, target_y))
        {
            self.message = format!("\n{refusal}");
            return self.consume_ordinary_town_talk();
        }
        if let Some(outcome) = self.talk_alarm_sentinel_at(dialog_id, target_x, target_y) {
            return outcome;
        }

        if let Some((_role, family)) = talk_shop_trigger(dialog_id) {
            // Same `shops.md §2` gate as the direction path above; this is the
            // keyword-carrying entry the visual shell uses, and leaving it out
            // is why the first fix looked like it had not taken.
            if !self.shop_keeper_is_open_for_business(target_x, target_y) {
                self.message = SHOP_CLOSED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            if self.player.transport.is_horse() && dialog_id != 0x83 {
                self.message = SHOP_MOUNTED_REFUSAL.to_string();
                return self.consume_ordinary_town_talk();
            }
            let scene_byte = match self.area {
                Area::Town { scene, .. } => Some(scene.byte),
                _ => None,
            };
            if let Some(session) =
                crate::shop_session::shop_session_for_talk_context(dialog_id, scene_byte)
            {
                if matches!(&session, crate::shop_session::ActiveShopSession::Tavern(_)) {
                    self.tavern_secondary_drink_count = 0;
                }
                // `shops.md §8.1`: the post-item prompt's suffix reports
                // whether a transaction has completed "in this visit", so the
                // flag is per-visit and resets as the session opens.
                self.arms_transaction_completed = false;
                if matches!(
                    &session,
                    crate::shop_session::ActiveShopSession::Innkeeper(_)
                ) {
                    self.clear_active_effect_slot();
                }
                self.advance_turn();
                let message = self.format_talk_shop_opening_message(
                    dialog_id,
                    &session,
                    Some(family),
                    shoppe_renderer,
                );
                self.active_shop = Some(session);
                self.message = message;
                return MoveOutcome::Talked;
            }
        }
        if dialog_id == BLACKTHORN_GUARD_DEMAND_DIALOG_ID {
            return match self.begin_blackthorn_guard_demand(target_x, target_y, true, None) {
                Ok(outcome) => outcome,
                Err(_) => MoveOutcome::Blocked,
            };
        }
        if matches!(
            npc_dialog_id_kind(dialog_id),
            NpcDialogIdKind::NoDialogue | NpcDialogIdKind::HighSpecial
        ) {
            // `conversation.md §2`'s guard gate: the guard sprite answers
            // the stored `The guard offers no response!` literal and every
            // other non-speaker answers the bare `No response!`. The line "is
            // not composed from the NPC's Look description", which is what
            // this engine used to do off a single guard capture.
            self.message = crate::talk_non_speaker_refusal_for_sprite(
                self.npc_at_current_floor(target_x, target_y)
                    .map(|npc| npc.type_byte),
            );
            return self.consume_ordinary_town_talk();
        }

        let Some(fields) = dialogue.get(&(dialog_id as u16)) else {
            self.message = format!("Dialogue id {dialog_id} is unresolved for this scene."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        };
        if fields.len() < 3 {
            self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
            return MoveOutcome::Blocked;
        }
        let raw_fields = raw_blob.get(&(dialog_id as u16));
        let keyword = keyword.and_then(non_empty_talk_keyword);

        let name = fields
            .first()
            .filter(|name| !name.is_empty())
            .map(String::as_str)
            .unwrap_or("someone");
        let description = fields
            .get(1)
            .filter(|description| !description.is_empty())
            .map(String::as_str)
            .unwrap_or("no description");

        self.advance_turn();

        let scene_for_flags = match self.area {
            Area::Town { scene, .. } => Some(scene),
            _ => None,
        };

        if keyword.is_none() {
            if let Some(raw_fields) = raw_fields {
                let session = crate::conversation_session::ConversationSession::new(
                    raw_fields.clone(),
                    fields.clone(),
                );
                self.active_conversation = Some(Box::new(session));
                self.active_conversation_npc_slot = conversation_npc_slot;
                // `conversation.md §7.6` / §10: the branch-flag bit `0x8C`
                // tests and `0x88` sets is the speaking NPC's roster slot,
                // supplied by the engine and never by the script.
                let npc_slot = self.active_conversation_npc_slot.map(|slot| slot as u8);
                if let Some(session) = self.active_conversation.as_mut() {
                    session.set_npc_slot(npc_slot);
                }
                let (rendered, suspended) = self.active_conversation_opening_rendered();
                let opening = conversation_opening_rendered(
                    &rendered,
                    self.open_conversation_prompt(suspended),
                );
                self.emit_tlk_message(opening);
                return MoveOutcome::Talked;
            }
        }

        // Compose the inputs for the byte-runner once per call; the
        // avatar name is the first party member's name (or "Avatar"
        // when the roster has not been seeded yet).
        let avatar_name = self
            .party_names
            .first()
            .map(|name| {
                let trimmed: Vec<u8> = name.iter().take_while(|b| **b != 0).copied().collect();
                String::from_utf8_lossy(&trimmed).into_owned()
            })
            .unwrap_or_else(|| "Avatar".to_string());
        let branch_flags = scene_for_flags
            .map(|scene| self.talk_branch_slot_for_scene(scene))
            .unwrap_or(0);
        let dictionary_owned = self.common_word_dictionary.clone();
        let dictionary_refs = common_word_dictionary_refs_or_published(dictionary_owned.as_ref());
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: Some(&dictionary_refs),
            curse_seen: false,
            gold_available: Some(self.gold),
            npc_slot: self.active_conversation_npc_slot.map(|slot| slot as u8),
            ask_who_response: 0,
            yield_on_pause: false,
            yield_on_ask: false,
        };

        // Resolve which field(s) to run through the byte runner.
        let run_field = |idx: usize| -> Option<crate::tlk_runner::TlkRunOutput> {
            raw_fields
                .and_then(|raw| raw.get(idx))
                .map(|bytes| crate::tlk_runner::run_tlk_stream(bytes, &inputs))
        };

        let mut applied_grants: Vec<crate::tlk_control_codes::TlkActionDispatchVerb> = Vec::new();
        let mut applied_payments: Vec<crate::conversation_session::ConversationGoldPayment> =
            Vec::new();
        let mut applied_signal_flags: Vec<u8> = Vec::new();
        let mut applied_flags: u32 = 0;
        let mut applied_standing: Option<u8> = None;

        if let Some(keyword) = keyword {
            if fields.len() < 5 {
                self.message = format!("Dialogue id {dialog_id} has no complete talk envelope."); // audit: not a player-facing line
                return MoveOutcome::Talked;
            }
            let response_field_index = resolve_keyword_response_field_index(fields, keyword);
            let response_text = if let Some(idx) = response_field_index {
                if let Some(output) = run_field(idx) {
                    applied_grants.extend(output.action_grants.iter().copied());
                    applied_signal_flags.extend(output.signal_flags.iter().copied());
                    applied_payments.extend(output.events.iter().filter_map(|event| match event {
                        crate::tlk_runner::TlkRunEvent::GoldPayment { amount, accepted } => {
                            Some(crate::conversation_session::ConversationGoldPayment {
                                amount: *amount,
                                accepted: *accepted,
                            })
                        }
                        _ => None,
                    }));
                    applied_flags |= output.branch_flags_set;
                    applied_standing = output.moral_standing.or(applied_standing);
                    if output.text.is_empty() {
                        TLK_NO_KEYWORD_MATCH_MESSAGE.to_string()
                    } else {
                        output.text
                    }
                } else {
                    talk_keyword_response(fields, keyword)
                        .filter(|response| !response.is_empty())
                        .unwrap_or(TLK_NO_KEYWORD_MATCH_MESSAGE)
                        .to_string()
                }
            } else {
                TLK_NO_KEYWORD_MATCH_MESSAGE.to_string()
            };
            let (legacy_text, legacy_actions) = talk_response_text_and_actions(&response_text);
            self.apply_tlk_action_grants(&applied_grants);
            self.apply_tlk_gold_payments(&applied_payments);
            self.apply_tlk_moral_standing(applied_standing);
            self.record_tlk_signal_flags(&applied_signal_flags);
            self.apply_talk_action_grants(&legacy_actions);
            if let Some(scene) = scene_for_flags {
                self.merge_talk_branch_flags(scene, applied_flags);
            }
            // Measured (`qa/paired/castle-talk.tsv`): a reply is the NPC's
            // own text and nothing else - the sibling branch below already
            // prints it that way, and the original never names the speaker
            // on the reply row.
            let _ = name;
            self.message = legacy_text;
        } else {
            let greeting_text = if let Some(output) = run_field(2) {
                applied_grants.extend(output.action_grants.iter().copied());
                applied_signal_flags.extend(output.signal_flags.iter().copied());
                applied_payments.extend(output.events.iter().filter_map(|event| match event {
                    crate::tlk_runner::TlkRunEvent::GoldPayment { amount, accepted } => {
                        Some(crate::conversation_session::ConversationGoldPayment {
                            amount: *amount,
                            accepted: *accepted,
                        })
                    }
                    _ => None,
                }));
                applied_flags |= output.branch_flags_set;
                applied_standing = output.moral_standing.or(applied_standing);
                if output.text.is_empty() {
                    "...".to_string()
                } else {
                    output.text
                }
            } else {
                fields
                    .get(2)
                    .filter(|greeting| !greeting.is_empty())
                    .map(String::clone)
                    .unwrap_or_else(|| "...".to_string())
            };
            let (legacy_text, legacy_actions) = talk_response_text_and_actions(&greeting_text);
            self.apply_tlk_action_grants(&applied_grants);
            self.apply_tlk_gold_payments(&applied_payments);
            self.apply_tlk_moral_standing(applied_standing);
            self.record_tlk_signal_flags(&applied_signal_flags);
            self.apply_talk_action_grants(&legacy_actions);
            if let Some(scene) = scene_for_flags {
                self.merge_talk_branch_flags(scene, applied_flags);
            }
            self.message = format!("Talked to {name}: {description}. {legacy_text} Your interest?");
        }
        MoveOutcome::Talked
    }

    fn format_talk_shop_opening_message(
        &mut self,
        dialog_id: u8,
        session: &crate::shop_session::ActiveShopSession,
        family: Option<&str>,
        shoppe_renderer: Option<&crate::shoppe_bark::ShoppeTextRenderer>,
    ) -> String {
        if crate::shoppe_records::talk_entry_uses_shared_preamble(dialog_id) {
            if let Some(renderer) = shoppe_renderer {
                let ordinal = self.random_range_u8(0, 3);
                if let Some(record_id) = crate::shoppe_records::shared_shop_bark_record(
                    dialog_id,
                    crate::shoppe_records::SharedShopBarkKind::Preamble,
                    ordinal,
                ) {
                    // `shops.md §4.1` / `§8.0`: `#` takes the shop's display
                    // name and `$` takes the *vendor's* name. They are two
                    // different resident tables sharing one shop-instance
                    // row, so the shop label must not stand in for the
                    // shopkeeper.
                    let label = session.shop_label();
                    let scene_byte = match self.area {
                        Area::Town { scene, .. } => Some(scene.byte),
                        _ => None,
                    };
                    let vendor = scene_byte
                        .and_then(|scene| shop_vendor_name_for_scene(dialog_id, scene))
                        .unwrap_or(label);
                    let ctx = crate::shoppe_bark::ShoppeBarkContext {
                        vendor_name: vendor,
                        shop_name: label,
                        hour: self.clock.hour,
                        ..Default::default()
                    };
                    if let Ok(rendered) = renderer.render_record(record_id, &ctx) {
                        // `shops.md §8.B`: "**All seven non-arms entries**
                        // first emit an opening double quote, render one
                        // record from their shared entry row, then place the
                        // input continuation according to the resulting
                        // window-local cursor column ... The record supplies
                        // the greeting/question and its closing quote."
                        //
                        // The engine printed the record bare: Cove's healer
                        // opened `I bid thee welcome` where the stock game
                        // opened `"I bid thee welcome`, and its answer row
                        // never appeared at all.
                        // `shops.md §8.0`: the Talk dispatcher "first emits
                        // the same conversation entry newline used for all
                        // Talk dispatches" before handing control to the shop
                        // overlay, so the greeting opens a row below the
                        // command echo the way a conversation reply does.
                        let greeting = format!("\n\"{rendered}");
                        let continuation = crate::shoppe_records::shop_entry_input_continuation(
                            crate::wrapped_final_row_columns(&greeting),
                        );
                        let rendered = format!("{greeting}{continuation}");
                        // Measured 2026-09-07/08 at four shops - the Paws
                        // tavern, the North Britanny inn, Cove's healer and
                        // Cove's herbalist (`qa/paired/paws-tavern.tsv`,
                        // `nb-inn.tsv`, `cove-healer.tsv`,
                        // `cove-herbalist.tsv`): entry prints the greeting
                        // record and nothing else, and the shop then waits on
                        // a `:` row where `Y` echoes as `:Yes`. The engine
                        // used to append an invented key summary
                        // (`Rest (R), Leave (L), Pick up (P), or Space.`).
                        return rendered;
                    }
                }
            }
        }

        // `shops.md §8.B`, "Arms entry, in order": arms does not use the
        // shared entry greeting - it opens on its own resident welcome line,
        // "`\"Good @, and welcome to #!\"\n`, with token expansion". The
        // attribution and greeting follow the stage-2 pause, in
        // `handle_arms_shop_key_input`.
        //
        // Without this the arms entry fell through to the no-assets fallback
        // below and printed `Weaponsmith / Armourer is now open.` in ordinary
        // play, which is how it read at Britain's weaponsmith with a full
        // asset directory loaded.
        if dialog_id == crate::shoppe_records::SHOP_DIALOG_ID_ARMS {
            return crate::input_dispatch::arms_welcome_line(self.clock.hour, session.shop_label());
        }

        // `shops.md §8` gives every other shop kind a `SHOPPE.DAT` entry
        // greeting; this is the fallback for when that record cannot be
        // rendered. It used to append `Dispatch family: {family}.`, which
        // is engine-internal routing vocabulary with no counterpart in the
        // original - a paired capture of the Iolo's Bows counter showed it
        // on screen. The routing family is still taken as an argument
        // because callers pass it for other purposes; it is simply never
        // printed.
        let _ = family;
        // Only reached when `SHOPPE.DAT` cannot be read at all - unit
        // fixtures with no assets. The original always has a record here.
        let label = session.shop_label();
        format!("{label} is now open.")
    }

    /// Apply the byte-runner's recorded [`TlkActionDispatchVerb`] grants
    /// to the live runtime counters per `conversation.md §7.6`.
    /// `conversation.md §7.4` / `karma.md §4`: assign the shared
    /// moral-standing selector after a TLK stream ran `0x89`
    /// STANDING-UP or `0x8A` STANDING-DOWN. The runner already applied
    /// the published capped-add / capped-subtract writers, so this is an
    /// assignment; `None` means the stream ran neither code and the
    /// selector is untouched.
    pub fn apply_tlk_moral_standing(&mut self, standing: Option<u8>) {
        if let Some(standing) = standing {
            self.moral_standing = standing;
        }
    }

    pub fn apply_tlk_action_grants(
        &mut self,
        grants: &[crate::tlk_control_codes::TlkActionDispatchVerb],
    ) {
        use crate::tlk_control_codes::TlkActionDispatchVerb;
        for grant in grants {
            match grant {
                TlkActionDispatchVerb::RaiseFood => {
                    self.food = self.food.saturating_add(1).min(PARTY_FOOD_CAP);
                }
                TlkActionDispatchVerb::RaiseGold => {
                    self.gold = self.gold.saturating_add(1).min(PARTY_GOLD_CAP);
                }
                TlkActionDispatchVerb::RaiseKeys => {
                    self.keys = self.keys.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::RaiseGems => {
                    self.gems = self.gems.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::RaiseTorches => {
                    self.torches = self.torches.saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::SetGrappleGate => {
                    self.climbing_gear = 1;
                }
                TlkActionDispatchVerb::RaiseCarpets => {
                    let slot = &mut self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX];
                    *slot = (*slot).saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
                TlkActionDispatchVerb::SetSextantCarried => {
                    self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE;
                }
                TlkActionDispatchVerb::SetSpyglassCarried => {
                    self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE;
                }
                TlkActionDispatchVerb::SetBlackBadgeCarried => {
                    self.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE;
                }
                TlkActionDispatchVerb::RaiseSkullKeys => {
                    let slot = &mut self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX];
                    *slot = (*slot).saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
            }
        }
    }

    /// Apply accepted conversation gold payments emitted by TLK `0x85`.
    ///
    /// Per `conversation.md §7.6` and `karma.md §4.1`, affordability
    /// alone accepts a payment and debits the gold. The turn-aged saved
    /// cooldown is only tested/reset when the speaking live NPC belongs
    /// to the one qualifying actor class. A qualifying threshold payment
    /// raises moral standing by one, plus two when the post-debit purse
    /// is empty. Payments never increment the cooldown.
    pub fn apply_tlk_gold_payments(
        &mut self,
        payments: &[crate::conversation_session::ConversationGoldPayment],
    ) {
        let qualifying_speaker = self
            .active_conversation_npc_slot
            .and_then(|npc_slot| self.npcs.iter().find(|npc| npc.slot == npc_slot))
            .filter(|npc| {
                npc.active_object
                    .and_then(|slot| self.active_objects.get(slot))
                    .is_some_and(|object| !object.is_empty())
            })
            .is_some_and(|npc| npc.type_byte == TLK_GOLD_PAYMENT_KARMA_SPEAKER_CLASS);
        for payment in payments {
            if !payment.accepted || self.gold < payment.amount {
                continue;
            }
            self.gold -= payment.amount;
            if qualifying_speaker && self.toll_progress >= TOLL_PROGRESS_MILESTONE {
                self.toll_progress = 0;
                self.moral_standing = apply_karma_action(
                    self.moral_standing,
                    KarmaAction::TollMilestone {
                        left_party_with_zero_gold: self.gold == 0,
                    },
                );
            }
        }
    }

    /// Merge a byte-runner-produced set-flag mask into the active scene's
    /// branch-flag slot.
    pub fn merge_talk_branch_flags(&mut self, scene: Scene, mask: u32) {
        if mask == 0 {
            return;
        }
        let slot = self.talk_branch_flags.entry(scene.byte).or_insert(0);
        *slot |= mask;
    }

    /// Render the raw TLK Description entry for a conversation opening
    /// and apply any byte-runner side effects it emits. The normal
    /// keyword session owns the Greeting and later responses; the
    /// Description preamble runs once before the session is installed.
    pub fn render_raw_conversation_description(
        &mut self,
        raw_fields: &[Vec<u8>],
        decoded_fields: &[String],
    ) -> String {
        let fallback = decoded_fields
            .get(1)
            .filter(|description| !description.is_empty())
            .map(String::as_str)
            .unwrap_or("no description");
        let Some(bytes) = raw_fields.get(1) else {
            return fallback.to_string();
        };

        let avatar_name = self
            .party_names
            .first()
            .map(|name| {
                let trimmed: Vec<u8> = name.iter().take_while(|b| **b != 0).copied().collect();
                String::from_utf8_lossy(&trimmed).into_owned()
            })
            .unwrap_or_else(|| "Avatar".to_string());
        let branch_flags = match self.area {
            Area::Town { scene, .. } => self.talk_branch_slot_for_scene(scene),
            _ => 0,
        };
        let dictionary_owned = self.common_word_dictionary.clone();
        let dictionary_refs = common_word_dictionary_refs_or_published(dictionary_owned.as_ref());
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: Some(&dictionary_refs),
            curse_seen: false,
            gold_available: Some(self.gold),
            npc_slot: self.active_conversation_npc_slot.map(|slot| slot as u8),
            ask_who_response: 0,
            yield_on_pause: false,
            yield_on_ask: false,
        };
        let output = crate::tlk_runner::run_tlk_stream(bytes, &inputs);
        self.apply_tlk_action_grants(&output.action_grants);
        let gold_payments: Vec<_> = output
            .events
            .iter()
            .filter_map(|event| match event {
                crate::tlk_runner::TlkRunEvent::GoldPayment { amount, accepted } => {
                    Some(crate::conversation_session::ConversationGoldPayment {
                        amount: *amount,
                        accepted: *accepted,
                    })
                }
                _ => None,
            })
            .collect();
        self.apply_tlk_gold_payments(&gold_payments);
        self.apply_tlk_moral_standing(output.moral_standing);
        self.record_tlk_signal_flags(&output.signal_flags);
        if let Area::Town { scene, .. } = self.area {
            self.merge_talk_branch_flags(scene, output.branch_flags_set);
        }
        if output.text.is_empty() {
            fallback.to_string()
        } else {
            output.text
        }
    }

    /// Open a multi-turn conversation session for the facing NPC.
    /// Returns the rendered greeting text. Returns `None` when there
    /// is no NPC, the NPC is a shop trigger, or the area is not a
    /// town-class scene.
    pub fn open_conversation_session(
        &mut self,
        dialogue: &HashMap<u16, Vec<String>>,
        raw_blob: &HashMap<u16, Vec<Vec<u8>>>,
    ) -> Option<String> {
        if !matches!(self.area, Area::Town { .. }) {
            return None;
        }
        if self.talk_liveness_blocked() {
            return None;
        }
        let (dialog_id, target_x, target_y) = self.facing_talk_target()?;
        if matches!(
            dialog_id,
            TOWN_NPC_COWERING_DIALOG_ID | TOWN_NPC_BRUSHOFF_DIALOG_ID
        ) {
            let text = if dialog_id == TOWN_NPC_BRUSHOFF_DIALOG_ID {
                let target_slot = self
                    .npc_at_current_floor(target_x, target_y)
                    .map(|npc| npc.slot);
                if let Some(index) = self
                    .npcs
                    .iter()
                    .position(|npc| Some(npc.slot) == target_slot)
                {
                    let _ = self.npcs[index].force_town_flight();
                    self.record_town_npc_mutation(index);
                }
                TOWN_NPC_BRUSHOFF_RESPONSE
            } else {
                TOWN_NPC_COWERING_RESPONSE
            };
            self.active_conversation = None;
            self.active_conversation_npc_slot = None;
            self.message = text.to_string();
            return Some(text.to_string());
        }
        if dialog_id == 0 || talk_shop_trigger(dialog_id).is_some() {
            return None;
        }
        let fields = dialogue.get(&(dialog_id as u16))?;
        let raw = raw_blob.get(&(dialog_id as u16))?;
        let session =
            crate::conversation_session::ConversationSession::new(raw.clone(), fields.clone());
        self.active_conversation = Some(Box::new(session));
        self.active_conversation_npc_slot = self
            .npc_at_current_floor(target_x, target_y)
            .map(|npc| npc.slot);
        // `conversation.md §7.6` / §10: the branch-flag bit `0x8C`
        // tests and `0x88` sets is the speaking NPC's roster slot,
        // supplied by the engine and never by the script.
        let npc_slot = self.active_conversation_npc_slot.map(|slot| slot as u8);
        if let Some(session) = self.active_conversation.as_mut() {
            session.set_npc_slot(npc_slot);
        }
        let (rendered, suspended) = self.active_conversation_opening_rendered();
        let opening =
            conversation_opening_rendered(&rendered, self.open_conversation_prompt(suspended));
        let text = opening.text.clone();
        self.emit_tlk_message(opening);
        Some(text)
    }

    /// Render the active conversation's greeting and put it in
    /// `state.message`. Returns the rendered text.
    pub fn advance_active_conversation_greeting(&mut self) -> String {
        let (rendered, suspended) = self.active_conversation_opening_rendered();
        let opening =
            conversation_opening_rendered(&rendered, self.open_conversation_prompt(suspended));
        let text = opening.text.clone();
        self.emit_tlk_message(opening);
        text
    }

    /// Which prompt closes the opening: `§7`'s ASK-WHO name prompt when
    /// the Description entry suspended on its `0x88`, `§6`'s keyword
    /// prompt otherwise.
    fn open_conversation_prompt(&self, suspended: bool) -> &'static str {
        if suspended {
            crate::TLK_ASK_WHO_PROMPT
        } else {
            TLK_KEYWORD_PROMPT
        }
    }

    fn active_conversation_opening_rendered(
        &mut self,
    ) -> (crate::tlk_runner::TlkRenderedText, bool) {
        self.active_conversation_opening_rendered_with_seed(None)
    }

    /// Testable form of the conversation opener. A supplied seed is installed
    /// only for a stranger; acquainted NPCs do not sample or mutate the PRNG.
    pub fn active_conversation_greeting_rendered_with_host_seed(
        &mut self,
        host_seed: u16,
    ) -> crate::tlk_runner::TlkRenderedText {
        self.active_conversation_opening_rendered_with_seed(Some(host_seed))
            .0
    }

    /// Run `§9`'s opening: the Description entry, then - unless that
    /// suspended on its own `0x88` - the greeting or stranger branch.
    /// Returns the rendered opening and whether it suspended.
    fn active_conversation_opening_rendered_with_seed(
        &mut self,
        stranger_host_seed: Option<u16>,
    ) -> (crate::tlk_runner::TlkRenderedText, bool) {
        let avatar_name = self
            .party_names
            .first()
            .map(|name| {
                let trimmed: Vec<u8> = name.iter().take_while(|b| **b != 0).copied().collect();
                String::from_utf8_lossy(&trimmed).into_owned()
            })
            .unwrap_or_else(|| "Avatar".to_string());
        let branch_flags = match self.area {
            Area::Town { scene, .. } => self.talk_branch_slot_for_scene(scene),
            _ => 0,
        };
        let party_name_bytes: Vec<Vec<u8>> = self
            .party_names
            .iter()
            .take(self.party.len())
            .map(|name| name.iter().take_while(|b| **b != 0).copied().collect())
            .collect();
        let party_member_names: Vec<&[u8]> = party_name_bytes.iter().map(Vec::as_slice).collect();
        let dictionary_owned = self.common_word_dictionary.clone();
        let dictionary_refs = common_word_dictionary_refs_or_published(dictionary_owned.as_ref());
        let ctx = crate::conversation_session::ConversationContext {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: Some(&dictionary_refs),
            gold_available: Some(self.gold),
            party_member_names: &party_member_names,
        };
        let mut rendered = crate::tlk_runner::TlkRenderedText::default();
        // §9 step 2 first: the Description entry is a byte stream, and a
        // shipped NPC can put the whole opening in it and leave the
        // Greeting entry empty (`cleak/u5-spec#198`). If it suspends on
        // its own `0x88` the greeting must not run on top of it.
        let mut description = crate::tlk_runner::TlkRenderedText::default();
        let mut suspended = false;
        if let Some(session) = self.active_conversation.as_mut() {
            let output = session.present_description(&ctx);
            description = output.rendered_text();
            suspended = session.opening_suspended();
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_gold_payments(&output.gold_payments);
            self.apply_tlk_moral_standing(output.moral_standing);
            self.record_tlk_signal_flags(&output.signal_flags);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        if suspended {
            return (description, true);
        }
        let knows_party = self
            .active_conversation_npc_slot
            .and_then(|slot| u8::try_from(slot).ok())
            .is_none_or(|slot| talk_branch_flag_is_set(branch_flags, slot));
        let stranger_introduces = if knows_party {
            false
        } else {
            self.prng_state = stranger_host_seed.unwrap_or_else(host_clock_prng_seed_now);
            self.random_range_u8(0, 1) != 0
        };
        if let Some(session) = self.active_conversation.as_mut() {
            let output = if knows_party {
                session.present_greeting(&ctx)
            } else {
                session.present_stranger_opening(&ctx, stranger_introduces)
            };
            rendered = output.rendered_text();
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_gold_payments(&output.gold_payments);
            self.apply_tlk_moral_standing(output.moral_standing);
            self.record_tlk_signal_flags(&output.signal_flags);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        let mut opening = description;
        if !rendered.text.trim().is_empty() {
            // `conversation.md §9` step 3: both arms speak inside the quote
            // wrapper - the known-NPC arm "opens the quote wrapper and runs
            // the Greeting entry", and the stranger arm "prints the `I am
            // called` lead-in and runs the **Name** entry ... then closes
            // the quote". The description ahead of it is narration and
            // stays bare. A paired capture of a walked-in castle
            // conversation reads `You see a pretty young girl.`, a blank
            // row, then `"I am called Ava"`. `cleak/u5-engine#5`.
            let trimmed = rendered.trimmed();
            opening.push_plain("\n\n");
            if trimmed.text.trim_start().starts_with('"') {
                opening.push_rendered(&trimmed);
            } else {
                opening.push_plain("\"");
                opening.push_rendered(&trimmed);
                opening.push_plain("\"");
            }
        }
        (opening, false)
    }

    /// Submit one typed keyword line to the active conversation.
    /// Returns the rendered response text and a flag indicating
    /// whether the session has ended.
    pub fn submit_active_conversation_keyword(&mut self, line: &str) -> (String, bool) {
        let join_keyword = line.trim().eq_ignore_ascii_case("join");
        let join_candidate = self
            .active_conversation
            .as_ref()
            .and_then(|session| session.npc_name());
        let avatar_name = self
            .party_names
            .first()
            .map(|name| {
                let trimmed: Vec<u8> = name.iter().take_while(|b| **b != 0).copied().collect();
                String::from_utf8_lossy(&trimmed).into_owned()
            })
            .unwrap_or_else(|| "Avatar".to_string());
        let branch_flags = match self.area {
            Area::Town { scene, .. } => self.talk_branch_slot_for_scene(scene),
            _ => 0,
        };
        let party_name_bytes: Vec<Vec<u8>> = self
            .party_names
            .iter()
            .take(self.party.len())
            .map(|name| name.iter().take_while(|b| **b != 0).copied().collect())
            .collect();
        let party_member_names: Vec<&[u8]> = party_name_bytes.iter().map(Vec::as_slice).collect();
        let dictionary_owned = self.common_word_dictionary.clone();
        let dictionary_refs = common_word_dictionary_refs_or_published(dictionary_owned.as_ref());
        let ctx = crate::conversation_session::ConversationContext {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: Some(&dictionary_refs),
            gold_available: Some(self.gold),
            party_member_names: &party_member_names,
        };
        let mut text = String::new();
        let mut rendered = crate::tlk_runner::TlkRenderedText::default();
        let mut ended = false;
        let mut asked_party_name = None;
        let mut ask_party_name_prompted = false;
        // The line was typed onto the prompt's own `:` row, and the
        // original keeps it there. Commit it before the response is
        // emitted, so the transcript reads `:JOB` and then the answer.
        let open_prompt = self.open_prompt_line();
        if let Some(prompt) = open_prompt.as_deref() {
            // §6's capture shows the keyword echoed in capitals.
            self.commit_typed_prompt_line(prompt, &line.trim().to_ascii_uppercase());
        }
        if let Some(session) = self.active_conversation.as_mut() {
            let output = session.submit_keyword(line, &ctx);
            text = output.text.clone();
            rendered = output.rendered_text();
            ended = output.ended;
            asked_party_name = output.asked_party_name;
            ask_party_name_prompted = session.awaiting_ask_party_name();
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_gold_payments(&output.gold_payments);
            self.apply_tlk_moral_standing(output.moral_standing);
            self.record_tlk_signal_flags(&output.signal_flags);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        let join_prompted_for_roster_companion = ask_party_name_prompted
            && join_candidate
                .as_deref()
                .is_some_and(|candidate| self.conversation_join_candidate_available(candidate));
        if join_keyword || join_prompted_for_roster_companion {
            self.active_conversation_join_candidate = join_candidate;
        }
        if let Some(answer_slot) = asked_party_name {
            if let Some(candidate) = self.active_conversation_join_candidate.take() {
                if let Some(join_text) =
                    self.apply_conversation_join_candidate(&candidate, answer_slot)
                {
                    if !text.is_empty() {
                        text.push(' ');
                        rendered.push_plain(" ");
                    }
                    text.push_str(&join_text);
                    rendered.push_plain(&join_text);
                }
            }
        } else if !join_keyword && !join_prompted_for_roster_companion {
            self.active_conversation_join_candidate = None;
        }
        if ended {
            if let Some(session) = self.active_conversation.as_mut() {
                session.acknowledge_close();
            }
            self.active_conversation = None;
            self.active_conversation_npc_slot = None;
            if let Some(cleanup) = self.run_final_conversation_cleanup() {
                if !text.is_empty() {
                    text.push(' ');
                    rendered.push_plain(" ");
                }
                text.push_str(&cleanup);
                rendered.push_plain(&cleanup);
            }
        }
        debug_assert_eq!(rendered.text, text);
        self.emit_tlk_message(rendered);
        (text, ended)
    }

    /// Return a save-roster view with the live active-party prefix
    /// overlaid onto the preserved inactive records.
    pub fn synced_party_roster(&self) -> Vec<PartyRosterRecord> {
        let mut roster = if self.party_roster.is_empty() {
            party_roster_from_active(
                &self.party,
                &self.party_names,
                &self.party_experience,
                &self.party_stay_counters,
                &self.party_strengths,
                &self.party_intelligence,
                &self.party_equipment,
            )
        } else {
            self.party_roster.clone()
        };

        if roster.len() < self.party.len() {
            roster.resize_with(self.party.len(), || PartyRosterRecord {
                member: PartyMember {
                    slot: 0,
                    class_byte: b'A',
                    status: b'G',
                    climb_stat: DEFAULT_CLIMB_STAT,
                    mana: 0,
                    hp: DEFAULT_PARTY_HP,
                    max_hp: DEFAULT_PARTY_MAX_HP,
                    level: 1,
                },
                name: [0; SAVE_CHARACTER_NAME_LEN],
                // `formats/saved-gam.md §3.1` publishes only two gender
                // byte values (`0x0B` male, `0x0C` female) and says nothing
                // about a record the engine synthesises with no save byte
                // behind it. Defaulting to the male value is an unpublished
                // engine choice, not a spec contract; `systems/shops.md
                // §8.1` puts it on the "otherwise" branch of the arms tail
                // either way. A record that came from a save keeps its own
                // byte via the re-sync arm below.
                gender: SAVE_GENDER_MALE_BYTE,
                experience: 0,
                stay_counter: 0,
                strength: AVATAR_STAT_MAX,
                intelligence: AVATAR_STAT_MAX,
                equipment: [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT],
            });
        }

        for index in 0..self.party.len() {
            if self.party_names.get(index).is_none() && roster.get(index).is_some() {
                continue;
            }
            let mut member = self.party[index];
            member.slot = index as u8;
            let previous = roster.get(index).cloned();
            roster[index] = PartyRosterRecord {
                member,
                // `formats/saved-gam.md §3.1`: the gender byte has no parallel
                // active-party vector to be overlaid from, so it is carried by
                // member identity rather than by slot index — the inn's
                // leave/pick-up helpers and New Order shift `party_names` and
                // friends without shifting `party_roster`, and a slot-indexed
                // carry would hand a departed member's byte to whoever moved
                // up into the slot. See `party_roster_carried_gender`.
                gender: crate::party::party_roster_carried_gender(
                    &self.party_roster,
                    index,
                    self.party_names.get(index),
                ),
                name: self
                    .party_names
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.name))
                    .unwrap_or([0; SAVE_CHARACTER_NAME_LEN]),
                experience: self
                    .party_experience
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.experience))
                    .unwrap_or(0),
                stay_counter: self
                    .party_stay_counters
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.stay_counter))
                    .unwrap_or(0),
                strength: self
                    .party_strengths
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.strength))
                    .unwrap_or_else(|| {
                        if index == 0 {
                            self.avatar_stats.strength
                        } else {
                            AVATAR_STAT_MAX
                        }
                    }),
                intelligence: self
                    .party_intelligence
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.intelligence))
                    .unwrap_or_else(|| {
                        if index == 0 {
                            self.avatar_stats.intelligence
                        } else {
                            AVATAR_STAT_MAX
                        }
                    }),
                equipment: self
                    .party_equipment
                    .get(index)
                    .copied()
                    .or_else(|| previous.as_ref().map(|record| record.equipment))
                    .unwrap_or([EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]),
            };
        }
        roster
    }

    pub fn sync_active_party_from_roster_len(&mut self, active_len: usize) {
        let active_len = active_len
            .min(SAVE_PARTY_SIZE_MAX as usize)
            .min(self.party_roster.len());
        self.party.clear();
        self.party_names.clear();
        self.party_experience.clear();
        self.party_stay_counters.clear();
        self.party_strengths.clear();
        self.party_intelligence.clear();
        self.party_equipment.clear();

        for (index, record) in self.party_roster.iter().take(active_len).enumerate() {
            let mut member = record.member;
            member.slot = index as u8;
            self.party.push(member);
            self.party_names.push(record.name);
            self.party_experience.push(record.experience);
            self.party_stay_counters.push(record.stay_counter);
            self.party_strengths.push(record.strength);
            self.party_intelligence.push(record.intelligence);
            self.party_equipment.push(record.equipment);
        }

        if let Some(avatar) = self.party_roster.first() {
            self.avatar_stats = AvatarStats {
                strength: avatar.strength,
                dexterity: avatar.member.climb_stat,
                intelligence: avatar.intelligence,
            };
        }
        if self.active_player.is_some_and(|index| index >= active_len) {
            self.active_player = None;
        }
    }

    /// `conversation.md §7.6` (table row `0x84` RECRUIT-SPEAKER): "The engine
    /// takes the speaker's *own* name from the Name entry of the loaded blob,
    /// and matches its opening characters — case-insensitively, with bit 7
    /// stripped — against the reserve portion of the sixteen-slot character
    /// roster, **scanned from the last slot downwards**. On a match the matched
    /// roster record is swapped into the active-party insertion slot, that
    /// record's inn-lodging marker is cleared, and the party-size byte is
    /// incremented; the engine then removes the NPC from the live scene."
    ///
    /// Only the reserve portion (`active_len ..` up to the sixteen-slot roster
    /// bound) is eligible, and the scan runs downwards from the last slot, so a
    /// duplicate name resolves to the *highest* reserve slot rather than the
    /// first one encountered.
    ///
    /// Boundary: `conversation.md §7.6` also says "If no reserve record matches
    /// the speaker's name the engine prints its no-match diagnostic", but the
    /// text of that diagnostic is not published for this path (§6 step 7's
    /// `I cannot help thee with that.` is the *keyword*-scan diagnostic), so no
    /// line is emitted here rather than inventing one.
    ///
    /// The `asked_party_name` swap arm below is withdrawn behaviour — §7.6 now
    /// refuses at the cap and ejects nobody, and
    /// `ConversationSession::absorb_recruit_speaker` already refuses before
    /// reaching this helper — but it is still pinned by a test outside this
    /// file, so it is left unreachable-from-play rather than deleted here.
    pub fn apply_conversation_join_candidate(
        &mut self,
        candidate_name: &str,
        asked_party_name: u8,
    ) -> Option<String> {
        let active_len = self.party.len().min(SAVE_PARTY_SIZE_MAX as usize);
        self.party_roster = self.synced_party_roster();
        let reserve_end = self.party_roster.len().min(SAVE_ROSTER_SLOT_COUNT);
        let target_index = (active_len..reserve_end)
            .rev()
            .find(|index| party_roster_name_matches(&self.party_roster[*index], candidate_name))?;

        let joined_name = party_name_to_string(&self.party_roster[target_index].name)
            .unwrap_or_else(|| candidate_name.trim().to_string());
        if active_len < SAVE_PARTY_SIZE_MAX as usize {
            let joining = self.party_roster.remove(target_index);
            let joining_name = joining.name;
            self.party_roster.insert(active_len, joining);
            // §7.6: "that record's inn-lodging marker is cleared". This engine
            // keeps the shifted inn-guest view of `formats/saved-gam.md` §9 as
            // its own registry, so the marker to clear is the scene marker of
            // the guest slot holding this character — the same clear that
            // `systems/shops.md` §8.4 pickup performs when a lodged companion
            // returns to the active roster.
            self.clear_inn_lodging_marker_for_name(&joining_name);
            self.sync_active_party_from_roster_len(active_len + 1);
            // §7.6: "the engine then removes the NPC from the live scene."
            self.remove_recruited_speaker_from_scene();
            return Some(format!("{joined_name} joined."));
        }

        let replace_index = usize::from(asked_party_name).checked_sub(1)?;
        if replace_index == 0 || replace_index >= active_len {
            return None;
        }

        let joining = self.party_roster.remove(target_index);
        let leaving = self.party_roster.remove(replace_index);
        let leaving_name = party_name_to_string(&leaving.name)
            .unwrap_or_else(|| format!("party member {}", replace_index.saturating_add(1)));
        self.party_roster.insert(replace_index, joining);
        self.party_roster.insert(active_len, leaving);
        if self.active_player == Some(replace_index) {
            self.active_player = None;
        }
        self.sync_active_party_from_roster_len(active_len);
        Some(format!("{joined_name} joined; {leaving_name} left."))
    }

    pub fn conversation_join_candidate_available(&self, candidate_name: &str) -> bool {
        self.party_roster
            .iter()
            .any(|record| party_roster_name_matches(record, candidate_name))
    }

    /// `conversation.md §7.6`: a `0x84` recruit clears the joined record's
    /// inn-lodging marker. `formats/saved-gam.md` §9 describes the inn-guest
    /// registry as a shifted view over the same sixteen character records whose
    /// "leading byte of each registry slot is an inn-scene marker"; clearing it
    /// to zero is the same de-lodging write `systems/shops.md` §8.4 pickup
    /// performs ("clears the returned slot's marker to zero after moving the
    /// guest back into the active roster"). Nothing else about the guest slot
    /// is touched here.
    fn clear_inn_lodging_marker_for_name(&mut self, name: &[u8; SAVE_CHARACTER_NAME_LEN]) {
        for guest in self.inn_registry.iter_mut() {
            if guest.name == *name {
                guest.scene_marker = 0;
            }
        }
    }

    /// `conversation.md §7.6`: "the engine then removes the NPC from the live
    /// scene." Live removal only — §7.6 says nothing about recording the slot
    /// in the per-scene permanent removal mask of `town-mode.md` §4, so the
    /// slot is freed without marking it permanently gone.
    fn remove_recruited_speaker_from_scene(&mut self) {
        let Some(npc_slot) = self.active_conversation_npc_slot else {
            return;
        };
        let Some(index) = self.npcs.iter().position(|npc| npc.slot == npc_slot) else {
            return;
        };
        if let Some(object_slot) = self.npcs[index].active_object {
            self.free_active_object_slot(object_slot);
        }
        self.npcs.remove(index);
        self.mark_visibility_dirty();
    }

    /// Return the active scene's shared conversation cleanup sentinel.
    pub fn shared_town_conversation_sentinel(&self) -> u8 {
        if !matches!(self.area, Area::Town { .. }) {
            return CONVERSATION_SHARED_NO_SLOT_SENTINEL;
        }
        self.resident_shadowlord
            .map(|slot| slot as u8)
            .unwrap_or(CONVERSATION_SHARED_NO_SLOT_SENTINEL)
    }

    /// Apply numeric TLK `0x86` action-dispatch arguments to the generic
    /// one-conversation signal band. The public contract increments each
    /// selected slot by one through the capped byte helper.
    pub fn record_tlk_signal_flags(&mut self, flags: &[u8]) {
        for flag in flags {
            let index = usize::from(*flag);
            if let Some(slot) = self.conversation_signal_flags.get_mut(index) {
                *slot = slot.saturating_add(1).min(TLK_GENERIC_SIGNAL_CAP);
            }
        }
    }

    /// Run the Shadowlord of Falsehood's post-conversation theft.
    pub fn run_final_conversation_cleanup(&mut self) -> Option<String> {
        self.run_final_conversation_cleanup_with_seed(host_clock_prng_seed_now())
    }

    /// Deterministic-seed form of [`Self::run_final_conversation_cleanup`].
    /// The production caller samples the host clock after the warning line and
    /// sound; tests can inject the same resulting 12-bit seed directly.
    pub fn run_final_conversation_cleanup_with_seed(&mut self, host_seed: u16) -> Option<String> {
        if self.resident_shadowlord != Some(SHADOWLORD_FALSEHOOD_INDEX) {
            return None;
        }
        self.emit_sound_effect(SoundEffect::ActionSnap);

        // `prng.md §3`: this sample precedes inventory inspection even when
        // the selected branch below is a deterministic high-to-low scan.
        self.prng_state = host_seed;
        if self.keys != 0 || self.gems != 0 || self.torches != 0 {
            loop {
                let stock = match self.random_range_u8(0, 2) {
                    0 => &mut self.keys,
                    1 => &mut self.gems,
                    _ => &mut self.torches,
                };
                if *stock != 0 {
                    *stock -= 1;
                    break;
                }
            }
        } else if decrement_stock_high_to_low(&mut self.equipment_stock).is_some() {
        } else if decrement_stock_high_to_low(&mut self.scroll_stock).is_some() {
        } else if decrement_stock_high_to_low(&mut self.potion_stock).is_some() {
        } else {
            let debit = u16::from(self.random_range_u8(
                CONVERSATION_CLEANUP_GOLD_DEBIT_MIN,
                CONVERSATION_CLEANUP_GOLD_DEBIT_MAX,
            ));
            self.gold = self.gold.saturating_sub(debit);
        }
        self.mark_visibility_dirty();
        Some("Stolen goods.".to_string())
    }

    pub fn apply_talk_action_grants(&mut self, actions: &[char]) {
        for action in actions {
            match *action {
                'F' => self.climbing_gear = 1,
                'H' => {
                    self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
                }
                'I' => {
                    self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
                }
                'J' => {
                    self.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] =
                        SPECIAL_ITEM_TLK_CARRIED_FLAG_VALUE
                }
                _ => {}
            }
        }
    }

    /// `commands.md §3` / `conversation.md §2`: every town `T` result
    /// except the guard-demand arrest discriminator is an ordinary
    /// acted command. A missing target, status-tile refusal, funny-look
    /// stub, or liveness refusal therefore still consumes the town's
    /// one-minute turn and schedule pass.
    fn consume_ordinary_town_talk(&mut self) -> MoveOutcome {
        self.advance_turn();
        MoveOutcome::Blocked
    }

    fn talk_alarm_sentinel_at(
        &mut self,
        dialog_id: u8,
        target_x: usize,
        target_y: usize,
    ) -> Option<MoveOutcome> {
        let response = match dialog_id {
            TOWN_NPC_COWERING_DIALOG_ID => TOWN_NPC_COWERING_RESPONSE,
            TOWN_NPC_BRUSHOFF_DIALOG_ID => {
                let target_slot = self
                    .npc_at_current_floor(target_x, target_y)
                    .map(|npc| npc.slot);
                if let Some(index) = self
                    .npcs
                    .iter()
                    .position(|npc| Some(npc.slot) == target_slot)
                {
                    let _ = self.npcs[index].force_town_flight();
                    self.record_town_npc_mutation(index);
                }
                TOWN_NPC_BRUSHOFF_RESPONSE
            }
            _ => return None,
        };
        self.advance_turn();
        self.message = response.to_string();
        Some(MoveOutcome::Talked)
    }

    pub(crate) fn begin_blackthorn_guard_demand(
        &mut self,
        target_x: usize,
        target_y: usize,
        consume_turn: bool,
        game_dir: Option<&std::path::Path>,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            return Ok(self.consume_ordinary_town_talk());
        };
        let npc_slot = self
            .npc_at_current_floor(target_x, target_y)
            .map_or(0, |npc| npc.slot);
        let arrest = TownArrestPrompt {
            scene_byte: scene.byte,
            floor,
            npc_slot,
        };
        let living = self
            .party
            .iter()
            .filter(|member| party_member_counts_as_living(member.status))
            .count()
            .min(u16::MAX as usize) as u16;
        let badge_active = self.active_effect_tag == Some(BLACK_BADGE_ACTIVE_EFFECT_TAG)
            && self.active_effect_counter != 0;

        if consume_turn {
            self.advance_turn();
        }
        match begin_blackthorn_guard_demand(scene.byte, badge_active, living) {
            BlackthornGuardDemandStart::Prompt(prompt) => {
                self.active_blackthorn_guard_demand =
                    Some(ActiveBlackthornGuardDemand { prompt, arrest });
                self.message = prompt.message();
                Ok(MoveOutcome::Talked)
            }
            BlackthornGuardDemandStart::Refused => {
                // Measured 2026-09-08 at Minoc's gate
                // (`qa/paired/minoc-refuse.tsv`): a refused demand goes
                // straight into the published arrest exchange - `"Thou art
                // under arrest!"`, a blank row, `"Wilt thou come quietly?"`
                // and a `:` row - with no sentence of the engine's own in
                // between. `blackthorn.md` §7a says exactly this; the engine
                // was printing its own summary instead.
                //
                // §7a also says "Palace contact without the worn Badge
                // returns failure before printing a demand at all. The
                // caller then owns arrest or capture under `town-mode.md`
                // Section 14", and §14's arrest branches on location before
                // printing - so inside Blackthorn's castle this refusal is
                // the audience, not the challenge. Caught at `bt-audience`'s
                // `ask1` beat by `qa/tools/paired_compare.py`
                // (`cleak/u5-engine#21`): the stock side was already asking
                // the first mantra where this engine printed `"Thou art
                // under arrest!"`.
                self.message.clear();
                self.open_town_arrest(arrest, game_dir)
            }
        }
    }

    pub fn resolve_blackthorn_guard_demand_input(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: Option<&std::path::Path>,
    ) -> Option<MoveOutcome> {
        let active = self.active_blackthorn_guard_demand?;
        let mut input = String::new();
        input.push(key);
        input.push_str(suffix);
        // `blackthorn.md §7a` branch 2: the `:` answer row "echoes `Yes`
        // or `No!`". The password branch echoes the typed word instead
        // and is handled by the free-text path.
        if !matches!(active.prompt, BlackthornGuardDemandPrompt::PalacePassword) {
            match key.to_ascii_lowercase() {
                'y' => self.commit_prompt_reply(":", TOWN_ARREST_YES_REPLY),
                'n' => self.commit_prompt_reply(":", TOWN_ARREST_NO_REPLY),
                _ => {}
            }
        }
        match resolve_blackthorn_guard_demand(active.prompt, &input, self.gold) {
            BlackthornGuardDemandResolution::AwaitingInput => {
                self.message = active.prompt.message();
                Some(MoveOutcome::PromptDeclined)
            }
            BlackthornGuardDemandResolution::PaidOrPassed { gold } => {
                self.gold = gold;
                self.active_blackthorn_guard_demand = None;
                // `blackthorn.md §7a`: branch 1's match "answers `"Pass,
                // friend!"`" - that one is published. Branches 2 and 3
                // publish no acknowledgement at all; §7a says only that
                // "paid/passed is the ordinary outcome", so the engine
                // prints nothing rather than inventing one, the same way
                // it does for the unpublished arrest and guards'
                // challenges. It previously invented `Thy charitable gift
                // is accepted.` and `Thy tribute is accepted.`
                self.message = match active.prompt {
                    BlackthornGuardDemandPrompt::PalacePassword => "\"Pass, friend!\"".to_string(),
                    BlackthornGuardDemandPrompt::MinocCharity
                    | BlackthornGuardDemandPrompt::Tribute { .. } => String::new(),
                };
                Some(MoveOutcome::Talked)
            }
            BlackthornGuardDemandResolution::Refused { gold } => {
                self.gold = gold;
                self.active_blackthorn_guard_demand = None;
                // Same §7a / §14 hand-off as the immediate refusal above.
                self.message.clear();
                Some(self.open_town_arrest(active.arrest, game_dir).ok()?)
            }
        }
    }

    pub fn talk_liveness_blocked(&self) -> bool {
        let active_asleep = self
            .active_player
            .and_then(|index| self.party.get(index))
            .is_some_and(|member| member.status == b'S');
        talk_liveness_refusal(
            self.combat_active,
            active_asleep,
            self.food == 0,
            self.active_conversation.is_some(),
        )
        .is_some()
    }

    pub fn talk_branch_slot_for_scene(&self, scene: Scene) -> u32 {
        self.talk_branch_flags
            .get(&scene.byte)
            .copied()
            .unwrap_or(0)
    }

    pub fn talk_branch_flag_is_set_for_scene(&self, scene: Scene, bit_index: u8) -> bool {
        talk_branch_flag_is_set(self.talk_branch_slot_for_scene(scene), bit_index)
    }

    pub fn set_talk_branch_flag_for_scene(&mut self, scene: Scene, bit_index: u8) -> bool {
        if talk_branch_flag_mask(bit_index) == 0 {
            return false;
        }
        let slot = self.talk_branch_flags.entry(scene.byte).or_insert(0);
        set_talk_branch_flag(slot, bit_index)
    }

    pub fn active_talk_branch_flag_is_set(&self, bit_index: u8) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        self.talk_branch_flag_is_set_for_scene(scene, bit_index)
    }

    pub fn set_active_talk_branch_flag(&mut self, bit_index: u8) -> bool {
        let Area::Town { scene, .. } = self.area else {
            return false;
        };
        self.set_talk_branch_flag_for_scene(scene, bit_index)
    }
}

pub fn yew_wanted_poster_rows(names: &[[u8; SAVE_CHARACTER_NAME_LEN]]) -> Vec<String> {
    let mut rows = Vec::with_capacity(9);
    rows.push("abbbbbbbbbbbbbc".to_string());
    rows.push("g   Wanted:   g".to_string());
    rows.push("g             g".to_string());
    for slot in 0..3 {
        rows.push(yew_wanted_poster_name_row(names.get(slot)));
    }
    rows.push("g             g".to_string());
    rows.push("gDead or Aliveg".to_string());
    rows.push("deeeeeeeeeeeeeef".to_string());
    rows
}

fn yew_wanted_poster_name_row(name: Option<&[u8; SAVE_CHARACTER_NAME_LEN]>) -> String {
    let mut row = [b' '; 15];
    row[0] = b'g';
    row[14] = b'g';
    if let Some(name) = name.and_then(|name| party_name_to_string(name)) {
        let start = 7usize.saturating_sub(name.len() / 2);
        for (offset, byte) in name.bytes().take(13usize.saturating_sub(start)).enumerate() {
            row[start + offset] = byte;
        }
    }
    String::from_utf8(row.to_vec()).expect("poster rows are ASCII")
}

/// `conversation.md §7.6`: the `0x84` RECRUIT-SPEAKER compare matches the
/// speaker's "opening characters — case-insensitively, with bit 7 stripped —
/// against the reserve portion of the sixteen-slot character roster". §11's
/// derivation note names it as "the case-insensitive bit-7-stripping
/// string-equality routine used by the JOIN-name compare", which is the same
/// routine §6 step 5 uses for keywords: "The compare strips bit 7 from both
/// sides ... and folds both sides to upper case. A match requires the keyword
/// to end cleanly and the typed input either to end at the same point or to
/// have a literal space there; there is no substring search or fuzzy matching."
///
/// Here the speaker's name plays the keyword's role: it supplies the opening
/// characters, and the roster record must either end at that point or carry a
/// literal space there. So `LORD` matches a roster `LORD BRITISH` but `GWEN`
/// does not match `GWENNO`.
fn party_roster_name_matches(record: &PartyRosterRecord, needle: &str) -> bool {
    let speaker = conversation_name_compare_bytes(needle.as_bytes());
    if speaker.is_empty() {
        return false;
    }
    let roster = conversation_name_compare_bytes(&record.name);
    if roster.len() < speaker.len() || roster[..speaker.len()] != speaker[..] {
        return false;
    }
    matches!(roster.get(speaker.len()), None | Some(b' '))
}

/// Normalise one side of the §7.6 JOIN-name compare: stop at the record's NUL
/// terminator (obfuscated or plain), strip bit 7, fold to upper case, and drop
/// surrounding padding spaces.
fn conversation_name_compare_bytes(raw: &[u8]) -> Vec<u8> {
    let end = raw
        .iter()
        .position(|byte| byte & 0x7f == 0)
        .unwrap_or(raw.len());
    let folded: Vec<u8> = raw[..end]
        .iter()
        .map(|byte| (byte & 0x7f).to_ascii_uppercase())
        .collect();
    let start = folded
        .iter()
        .position(|byte| *byte != b' ')
        .unwrap_or(folded.len());
    let stop = folded
        .iter()
        .rposition(|byte| *byte != b' ')
        .map_or(start, |index| index + 1);
    folded[start..stop].to_vec()
}

pub fn decrement_stock_high_to_low(stock: &mut [u8]) -> Option<usize> {
    for index in (0..stock.len()).rev() {
        if stock[index] != 0 {
            stock[index] -= 1;
            return Some(index);
        }
    }
    None
}

pub fn scroll_label(index: usize) -> &'static str {
    SCROLL_SPELL_LABELS.get(index).copied().unwrap_or("Unknown")
}

fn pending_action_for_use_request(request: UseItemRequest) -> Option<UsePendingAction> {
    match request {
        UseItemRequest::Potion {
            index,
            target: None,
        } => Some(UsePendingAction::PotionTarget {
            index,
            highlight: 0,
        }),
        UseItemRequest::Scroll {
            index: SCROLL_WIND_CHANGE_INDEX,
            direction: None,
            ..
        } => Some(UsePendingAction::ScrollWindDirection {
            index: SCROLL_WIND_CHANGE_INDEX,
        }),
        UseItemRequest::Scroll {
            index: SCROLL_RESURRECTION_INDEX,
            target: None,
            ..
        } => Some(UsePendingAction::ScrollResurrectionTarget {
            highlight: 0,
            index: SCROLL_RESURRECTION_INDEX,
        }),
        // Measured: the skull key asks `Direction-` before it acts.
        UseItemRequest::SkullKey => Some(UsePendingAction::SkullKeyDirection),
        _ => None,
    }
}

fn shadowlord_shard_special_item_index(index: usize) -> Option<usize> {
    match index {
        SHADOWLORD_FALSEHOOD_INDEX => Some(SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX),
        SHADOWLORD_HATRED_INDEX => Some(SPECIAL_ITEM_SHARD_HATRED_INDEX),
        SHADOWLORD_COWARDICE_INDEX => Some(SPECIAL_ITEM_SHARD_COWARDICE_INDEX),
        _ => None,
    }
}

impl PlayState {
    /// Step the shared `On who:` party-member prompt by one key.
    ///
    /// See [`UseTargetStep`] for the published rule this follows.
    pub(crate) fn step_use_target_selector(
        &self,
        key: char,
        suffix: &str,
        highlight: usize,
    ) -> UseTargetStep {
        let last = self.party.len().saturating_sub(1);
        // Return or Space commits the indicated row.
        if matches!(key, '\r' | '\n' | ' ') {
            return UseTargetStep::Committed(highlight.min(last));
        }
        // The four direction keys move the indicator.
        if let Some(forward) = party_selector_direction_step(key) {
            let next = if forward {
                highlight.saturating_add(1).min(last)
            } else {
                highlight.saturating_sub(1)
            };
            return UseTargetStep::Moved(next);
        }
        // A digit repositions it, bounded by the party size; an out-of-range
        // digit leaves the indicator where it is.
        if let Some(target) = pending_use_party_target(key, suffix) {
            if target < self.party.len() {
                return UseTargetStep::Moved(target);
            }
            return UseTargetStep::Moved(highlight);
        }
        UseTargetStep::Waiting
    }
}

fn pending_use_party_target(key: char, suffix: &str) -> Option<usize> {
    let mut value = String::new();
    value.push(key);
    value.push_str(suffix);
    parse_inline_party_index(&value)
}

fn pending_use_cardinal_direction(key: char, suffix: &str) -> Option<Direction> {
    std::iter::once(key)
        .chain(suffix.chars())
        .find_map(Direction::from_play_key)
        .filter(|direction| direction.opposite_cardinal().is_some())
}

fn draw_surface_view_cell(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    class: u8,
    tile: u8,
    player_marker: bool,
    mode: ViewOverlayMode,
) {
    if player_marker {
        for offset in 0..scale {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, offset, scale / 2, 15);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, offset, 15);
        }
        return;
    }

    let color = surface_view_class_color(class, mode);
    match class {
        0x00 => {}
        0x01 => draw_surface_view_sparse_checker(viewport, cell_x, cell_y, scale, color),
        0x02 | 0x0F => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        0x03 => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        0x04 => {
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale - 1, color);
        }
        0x05 => {
            for y in (scale / 2).saturating_sub(1)..=(scale / 2) {
                for x in 1..scale.saturating_sub(1) {
                    set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
                }
            }
        }
        0x06 => draw_view_overlay_box(viewport, cell_x, cell_y, scale, color),
        0x07 => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        0x08 => {
            for y in 0..scale {
                for x in 0..scale {
                    if (x < scale / 2 && y < scale / 2) || (x >= scale / 2 && y >= scale / 2) {
                        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
                    }
                }
            }
        }
        0x09 => {
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 2, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, 2, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 3, color);
        }
        0x0A => {
            draw_surface_view_water_corners(viewport, cell_x, cell_y, scale, tile, mode);
        }
        0x0B => {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 0, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 2, color);
        }
        0x0C => {
            // `view.md §4`: deep water is not the old table-mapped no-op.
            // It draws exactly one modal-terrain micro-blit at `(2,2)`.
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 2, color);
        }
        0x0D => {
            let fixed = surface_view_class_color(0x02, mode);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, 0, fixed);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 3, 1, fixed);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 2, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 3, color);
        }
        0x0E => {
            for y in 0..scale {
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, y, color);
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, y, color);
            }
        }
        0x10 => draw_surface_view_road(viewport, cell_x, cell_y, scale, color, tile, mode),
        0x5A => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
        _ => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
    }
}

/// `view.md §6.3`: "Earlier revisions of this section described a magic
/// peer-view tint branch inside the dungeon map renderer, and an alternate
/// tinted tile source for some wall classes. Both are withdrawn: the value
/// being read is the display adapter identifier, not a peer-spell flag. The
/// dungeon map renderer has no peer-spell branch."
///
/// So this painter takes no [`ViewOverlayMode`]: V-View, the gem map, the
/// peer spell and the X-Ray spell all paint the identical dungeon map. The
/// only thing the original varied here is the display adapter, which this
/// engine models as [`TileGraphicsDepth`] on the viewport, and only the
/// high-colour (EGA/Tandy) pens are in the v1 target.
fn draw_dungeon_view_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    glyph: Option<DungeonMinimapGlyph>,
) {
    let Some(glyph) = glyph else {
        return;
    };
    let door_color = dungeon_view_door_color();
    let wall_color = dungeon_view_wall_color();
    let highlight = DUNGEON_VIEW_HIGHLIGHT_COLOR;
    // `dungeon-mode.md §12.3`: two classes "are not font characters at
    // all but small vector drawings". Their geometry is §12.5's and is
    // drawn directly rather than through a glyph index.
    let glyph = match glyph {
        DungeonMinimapGlyph::Fountain => {
            draw_dungeon_fountain_glyph(viewport, cell_x, cell_y, scale);
            return;
        }
        DungeonMinimapGlyph::EnergyField => {
            draw_dungeon_energy_field_glyph(viewport, cell_x, cell_y, scale);
            return;
        }
        DungeonMinimapGlyph::Font { index, .. } => index,
    };
    match glyph {
        // §12.4 party marker: arrowhead glyph 0x60 at the centre cell.
        // The player-marker branch of `draw_surface_view_cell` returns
        // before any mode-dependent colour lookup, so the mode passed
        // here is inert; the dungeon map has no peer-spell branch.
        0x60 => draw_surface_view_cell(
            viewport,
            cell_x,
            cell_y,
            scale,
            0,
            0,
            true,
            ViewOverlayMode::GemView,
        ),
        0x18 => draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale / 2, 7),
        0x2e => draw_dungeon_ladder_glyph(viewport, cell_x, cell_y, scale, true, false, highlight),
        0x2d => draw_dungeon_ladder_glyph(viewport, cell_x, cell_y, scale, false, true, highlight),
        0x2f => draw_dungeon_ladder_glyph(viewport, cell_x, cell_y, scale, true, true, highlight),
        0x70 => {
            fill_view_overlay_cell(viewport, cell_x, cell_y, scale, 6);
            draw_view_overlay_box(viewport, cell_x, cell_y, scale, highlight);
        }
        // §12.4 exact byte 0x68: the up-and-down arrow, and the only
        // published owner of glyph 0x12. The fountain class used to
        // collide with it here.
        0x12 => draw_dungeon_up_and_down_arrow_glyph(viewport, cell_x, cell_y, scale, 7),
        0x19 => draw_dungeon_pit_glyph(viewport, cell_x, cell_y, scale, highlight),
        0x71 => draw_dungeon_trap_glyph(viewport, cell_x, cell_y, scale, 12),
        0x72 => draw_dungeon_trap_glyph(viewport, cell_x, cell_y, scale, 14),
        0x73 => draw_dungeon_door_glyph(viewport, cell_x, cell_y, scale, door_color),
        0x74 => draw_view_overlay_box(viewport, cell_x, cell_y, scale, wall_color),
        0x75 => {
            draw_view_overlay_box(viewport, cell_x, cell_y, scale, wall_color);
            draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, highlight);
        }
        0x76 => {
            draw_view_overlay_box(
                viewport,
                cell_x,
                cell_y,
                scale,
                dungeon_view_extra_wall_color(),
            );
            set_view_overlay_pixel(
                viewport,
                cell_x,
                cell_y,
                scale,
                scale / 2,
                scale / 2,
                highlight,
            );
        }
        0x77 => {
            draw_dungeon_door_glyph(viewport, cell_x, cell_y, scale, door_color);
            draw_view_overlay_box(viewport, cell_x, cell_y, scale, door_color);
        }
        0x7f => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, wall_color),
        _ => set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, 7),
    }
}

/// Diagnostic ASCII stand-in for one minimap cell. The letters are this
/// engine's own text rendering of the map, not published art; what the
/// published table fixes is that each class gets its **own** output, so
/// the fountain and the energy field need codes of their own rather than
/// borrowing the arrow glyphs they used to collide with.
fn render_dungeon_minimap_glyph_code(glyph: Option<DungeonMinimapGlyph>) -> char {
    let Some(glyph) = glyph else {
        return ' ';
    };
    match glyph {
        DungeonMinimapGlyph::Fountain => 'f',
        DungeonMinimapGlyph::EnergyField => '=',
        DungeonMinimapGlyph::Font { index, .. } => match index {
            0x60 => '@',
            0x18 => '.',
            0x2e => '<',
            0x2d => '>',
            0x2f => 'H',
            0x70 => '$',
            0x12 => 'I',
            0x19 => 'o',
            0x71 => 'v',
            0x72 => '!',
            0x73 => '+',
            0x74 | 0x75 | 0x76 | 0x7f => '#',
            0x77 => '+',
            _ => '?',
        },
    }
}

/// `dungeon-mode.md §12.5`: the fountain basin is drawn in "the bright
/// foreground pen", which is also the pen the ladder, pit, chest-box and
/// wall-centre highlights use.
const DUNGEON_VIEW_HIGHLIGHT_COLOR: u8 = 15;

/// The dungeon minimap pens below take no [`ViewOverlayMode`]. `view.md
/// §6.3` and `dungeon-mode.md §12.4` both withdraw the peer-spell tint
/// branch that used to select them: "the value they were reading is the
/// **display-adapter identifier**, not a peer-spell flag ... V-View has no
/// peer-spell branch of its own." Any surviving variation belongs to the
/// display adapter ([`TileGraphicsDepth`]), and CGA/Hercules raster output
/// is outside the v1 clean-recreation target, so one high-colour pen set
/// serves every mode.
fn dungeon_view_door_color() -> u8 {
    11
}

fn dungeon_view_wall_color() -> u8 {
    8
}

/// `dungeon-mode.md §12.4`: the `0xD?` arch wall is "drawn with a
/// background pen; which pen depends on the display adapter" - the adapter,
/// not the view mode.
fn dungeon_view_extra_wall_color() -> u8 {
    13
}

fn draw_dungeon_ladder_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    up: bool,
    down: bool,
    color: u8,
) {
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, color);
    if up {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
    }
    if down {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale - 1, color);
    }
    if up && down {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale / 2, color);
    }
}

/// `dungeon-mode.md §12.5` fountain vector drawing. The basin strokes go
/// down first in the bright foreground pen, then the pen switches to a
/// brighter blue for the jet and the four spray dots. All ranges are
/// inclusive and relative to the cell's pixel origin.
///
/// There is no view-mode branch: `view.md §6.3` withdraws the peer-view
/// tint that used to pick a second basin/jet pair here.
fn draw_dungeon_fountain_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
) {
    let basin = DUNGEON_VIEW_HIGHLIGHT_COLOR;
    let jet = 9;
    let mut plot = |x: usize, y: usize, color: u8| {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    };
    // Basin: lower lip, middle lip, then the two feet.
    for x in 1..=6 {
        plot(x, 4, basin);
    }
    for x in 2..=5 {
        plot(x, 5, basin);
    }
    for x in 1..=2 {
        plot(x, 6, basin);
    }
    for x in 5..=6 {
        plot(x, 6, basin);
    }
    // Jet and spray.
    plot(2, 1, jet);
    plot(5, 1, jet);
    plot(1, 2, jet);
    plot(6, 2, jet);
    for x in 3..=4 {
        plot(x, 2, jet);
        plot(x, 3, jet);
    }
}

/// `dungeon-mode.md §12.5` energy-field vector drawing: eight full-width
/// horizontal runs covering **all eight** rows of the cell, in four
/// two-row colour bands. Each band's pen is a
/// `display-driver.md §2` user-interface colour-table slot biased into
/// the bright half of the palette — band A slot 4, band B slot 0, band C
/// slot 2, band D slot 3 — resolved through the same table the rest of
/// the interface uses so the low-colour drivers inherit their own values.
/// The drawing reads no sub-type, so all four field flavours look
/// identical on the map.
fn draw_dungeon_energy_field_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
) {
    let high_colour = viewport.depth.pixel_limit() > 4;
    for (band, slot) in DUNGEON_ENERGY_FIELD_BAND_SLOTS.iter().enumerate() {
        let color = crate::display_driver::ui_colour_slot_bright(*slot, high_colour);
        for row in 0..2 {
            let y = band * 2 + row;
            for x in 1..=6 {
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
            }
        }
    }
}

/// `dungeon-mode.md §12.5`: the user-interface colour-table slot each of
/// the energy field's four two-row bands takes its pen from, in band
/// order A, B, C, D.
const DUNGEON_ENERGY_FIELD_BAND_SLOTS: [usize; 4] = [4, 0, 2, 3];

/// `dungeon-mode.md §12.4` exact byte `0x68`: the up-and-down-arrow text
/// glyph `0x12`. Drawn as a vertical shaft with a head at each end.
fn draw_dungeon_up_and_down_arrow_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, color);
    let mid = scale / 2;
    for offset in 1..=1 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, mid - offset, offset, color);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, mid + offset, offset, color);
        set_view_overlay_pixel(
            viewport,
            cell_x,
            cell_y,
            scale,
            mid - offset,
            scale - 1 - offset,
            color,
        );
        set_view_overlay_pixel(
            viewport,
            cell_x,
            cell_y,
            scale,
            mid + offset,
            scale - 1 - offset,
            color,
        );
    }
}

fn draw_dungeon_pit_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_box(viewport, cell_x, cell_y, scale, color);
    set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, color);
}

fn draw_dungeon_trap_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_diagonals(viewport, cell_x, cell_y, scale, color);
    set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, color);
}

fn draw_dungeon_door_glyph(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale / 2, color);
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, (scale / 2) + 1, color);
    draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale / 2, color);
}

fn surface_view_class_color(class: u8, mode: ViewOverlayMode) -> u8 {
    if mode.uses_alternate_surface_view_bank() {
        match class {
            0x0A => return 3,
            0x0B => return 11,
            0x0C => return 11,
            0x0D => return 3,
            _ => {}
        }
    }
    match class {
        0x01 => 7,
        0x02 => 2,
        0x03 => 3,
        0x04 => 14,
        0x05 => 15,
        0x06 => 8,
        0x07 => 6,
        0x08 => 5,
        0x09 => 10,
        0x0A => 11,
        0x0B => 13,
        0x0C => 13,
        0x0D => 12,
        0x0E => 9,
        0x0F => 4,
        0x10 => 1,
        0x5A => 6,
        _ => 7,
    }
}

fn fill_view_overlay_cell(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    for y in 0..scale {
        for x in 0..scale {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
        }
    }
}

fn draw_view_overlay_box(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
    draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale - 1, color);
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, 0, color);
    draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale - 1, color);
}

fn draw_view_overlay_diagonals(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    for offset in 0..scale {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, offset, offset, color);
        set_view_overlay_pixel(
            viewport,
            cell_x,
            cell_y,
            scale,
            scale - 1 - offset,
            offset,
            color,
        );
    }
}

fn draw_surface_view_sparse_checker(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
) {
    for (x, y) in [(1, 0), (1, 2), (3, 1), (3, 3)] {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

fn surface_view_river_tile(tile: u8) -> bool {
    matches!(tile, 0x60..=0x69 | 0x6C..=0x6F)
}

fn draw_surface_view_water_corners(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    tile: u8,
    mode: ViewOverlayMode,
) {
    let modal = surface_view_class_color(0x0A, mode);
    let secondary = surface_view_class_color(0x02, mode);
    let shoreline_mask = tile & 0x0F;
    for (bit, x, y) in [(0x01, 1, 0), (0x02, 3, 1), (0x04, 1, 2), (0x08, 3, 3)] {
        let color = if surface_view_river_tile(tile) && shoreline_mask & bit == 0 {
            secondary
        } else {
            modal
        };
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

const fn surface_view_road_connection_mask(tile: u8) -> u8 {
    match tile {
        0x20 => 0x01 | 0x04, // north-south
        0x21 => 0x02 | 0x08, // east-west
        0x22 => 0x01 | 0x02, // north-east
        0x23 => 0x02 | 0x04, // east-south
        0x24 => 0x04 | 0x08, // south-west
        0x25 => 0x08 | 0x01, // west-north
        0x26 => 0x0F,
        _ => 0,
    }
}

fn draw_surface_view_road(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
    tile: u8,
    mode: ViewOverlayMode,
) {
    // `view.md §4`: the road opens with the class-1 secondary checker,
    // then lays down the frame-fill centre body and connection stubs.
    draw_surface_view_sparse_checker(
        viewport,
        cell_x,
        cell_y,
        scale,
        surface_view_class_color(0x01, mode),
    );
    let fill = surface_view_class_color(0x03, mode);
    for y in 1..=2 {
        for x in 1..=2 {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, fill);
        }
    }

    let mask = surface_view_road_connection_mask(tile);
    if mask & 0x1 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, 0, color);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 0, color);
    }
    if mask & 0x2 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 3, 1, color);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 3, 2, color);
    }
    if mask & 0x4 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 1, 3, color);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 2, 3, color);
    }
    if mask & 0x8 != 0 {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 1, color);
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 2, color);
    }

    let blank_notch = match tile {
        0x22 => Some((1, 2)),
        0x23 => Some((1, 1)),
        0x24 => Some((2, 1)),
        0x25 => Some((2, 2)),
        _ => None,
    };
    if let Some((x, y)) = blank_notch {
        // The final stamp is the blank source over the centre quarter
        // diagonally opposite the elbow, not a coloured orientation mark.
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, 0);
    }
}

fn draw_view_overlay_hline(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    y: usize,
    color: u8,
) {
    for x in 0..scale {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

fn draw_view_overlay_vline(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    x: usize,
    color: u8,
) {
    for y in 0..scale {
        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
    }
}

fn set_view_overlay_pixel(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    local_x: usize,
    local_y: usize,
    color: u8,
) {
    let x = cell_x * scale + local_x;
    let y = cell_y * scale + local_y;
    if x < viewport.width && y < viewport.height {
        let limit = viewport.depth.pixel_limit();
        viewport.pixels[y * viewport.width + x] = color % limit;
    }
}

fn conversation_opening_rendered(
    opening: &crate::tlk_runner::TlkRenderedText,
    prompt: &str,
) -> crate::tlk_runner::TlkRenderedText {
    // `text-output.md §10.4`: the opening prints under the
    // `Talk-<direction>` echo, which completed its own row, so its leading
    // line feed leaves one blank row between them - the shape the Look
    // result and the Talk refusals already carry. A paired capture of a
    // walked-in castle conversation reads `>Talk-North`, a blank row,
    // then `You see a pretty young girl.` (`cleak/u5-engine#5`).
    let mut rendered = crate::tlk_runner::TlkRenderedText::plain("\n");
    rendered.push_plain(TLK_OPENING_DESCRIPTION_PREFIX);
    rendered.push_rendered(&opening.trimmed());
    // A blank row separates the opening from the prompt that closes it,
    // the same shape §6's response framing carries. Two line feeds,
    // because `text-output.md §5`'s feed is a combined CR+LF.
    while !rendered.text.ends_with("\n\n") {
        rendered.push_plain("\n");
    }
    rendered.push_plain(prompt);
    rendered
}

#[cfg(test)]
mod shop_vendor_name_tests {
    use super::shop_vendor_name_for_scene;

    /// `shops.md §8.0` publishes 46 reachable vendor names across the eight
    /// per-kind tables. The vendor column is a *second* resident table sharing
    /// the shop-instance row with the display-name column, so it must never be
    /// derived from the shop's own name.
    #[test]
    fn published_vendor_names_cover_all_forty_six_reachable_shop_rows() {
        let published: [(u8, u8, &str); 46] = [
            (0x81, 2, "Gwenneth"),
            (0x81, 3, "Nomaan"),
            (0x81, 4, "Ronan"),
            (0x81, 5, "Shenstone"),
            (0x81, 6, "Paul"),
            (0x81, 17, "Max"),
            (0x81, 24, "Kitiara"),
            (0x81, 26, "Steve"),
            (0x81, 32, "Thol"),
            (0x82, 1, "Sam"),
            (0x82, 2, "Tika"),
            (0x82, 3, "Nicole"),
            (0x82, 4, "Duclas"),
            (0x82, 8, "Felicity"),
            (0x82, 19, "Jaymes"),
            (0x82, 22, "Dr. Cat"),
            (0x82, 24, "Nikki"),
            (0x82, 30, "Rob"),
            (0x83, 6, "Hettar"),
            (0x83, 20, "Theoan"),
            (0x83, 22, "Ferru"),
            (0x84, 3, "Bantral"),
            (0x84, 5, "Captain Blyth"),
            (0x84, 21, "Master Hawkins"),
            (0x84, 24, "Jones"),
            (0x85, 1, "Nilrem"),
            (0x85, 4, "Madam Pendra"),
            (0x85, 7, "Toama"),
            (0x85, 23, "Enlor"),
            (0x85, 30, "Virden"),
            (0x86, 8, "Braunam"),
            (0x86, 22, "Danfits"),
            (0x86, 24, "Daem"),
            (0x87, 5, "Regina"),
            (0x87, 6, "Leila"),
            (0x87, 7, "Temptious"),
            (0x87, 21, "Milan"),
            (0x87, 23, "Jessica"),
            (0x87, 30, "Faye"),
            (0x87, 31, "Jessip"),
            (0x88, 2, "Donya"),
            (0x88, 3, "Gremnor"),
            (0x88, 7, "Rogi"),
            (0x88, 20, "Terbor"),
            (0x88, 22, "Lorien"),
            (0x88, 24, "Ransack"),
        ];

        for (dialog_id, scene, vendor) in published {
            assert_eq!(
                shop_vendor_name_for_scene(dialog_id, scene),
                Some(vendor),
                "shops.md §8.0 vendor for trigger {dialog_id:#04x} scene {scene}"
            );
        }
    }

    #[test]
    fn horse_trader_scene_thirty_row_stays_unreachable() {
        // `shops.md §8.0`: the shipped horse-trader table holds a fourth row
        // for The Lycaeum whose vendor is Simplon, but "no `0x83` trigger
        // exists anywhere in the shipped rosters for scene `30`, so the row is
        // unreachable in ordinary play ... implementations should publish and
        // reach the three rows above and must not treat scene `30` as a fourth
        // stable."
        assert_eq!(shop_vendor_name_for_scene(0x83, 30), None);
    }

    #[test]
    fn a_scene_absent_from_a_kinds_table_resolves_to_no_vendor() {
        // `shops.md §8.0`: the shipped search leaves the row one past the end
        // and reads a neighbouring kind's data; "a clean implementation should
        // reject the trigger and leave the conversation alone", so this
        // lookup must not fall back to row zero.
        assert_eq!(shop_vendor_name_for_scene(0x81, 1), None);
        assert_eq!(shop_vendor_name_for_scene(0x88, 32), None);
        assert_eq!(shop_vendor_name_for_scene(0x89, 2), None);
    }
}

/// What one key did at the shared `On who:` party-member prompt.
///
/// `inventory.md §4`: "**A digit moves the indicator; it does not commit.**
/// `1` through `6`, bounded by the party size, reposition the inverted row
/// exactly as the direction keys do and leave the prompt open. Only Return or
/// Space commits the indicated row, Escape cancels" - and "This is one shared
/// routine, so the rule is the same for Z-stats, R-Ready, New Order, the
/// fountain, Search and every other caller." The `On who:` prompt the potion
/// and the Resurrection scroll share is such a caller.
///
/// This engine committed on the digit instead, so it consumed one key fewer
/// than the original and the scripted Return that should have committed fell
/// through to the world loop and echoed `What`. Measured 2026-09-10
/// (`use-potions/red`), where the original shows no such line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UseTargetStep {
    /// A digit or direction key repositioned the indicator.
    Moved(usize),
    /// An unrecognised key; the prompt stays open and nothing is redrawn.
    Waiting,
    /// Return or Space committed the indicated row.
    Committed(usize),
}

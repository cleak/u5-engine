use std::collections::{HashMap, VecDeque};
use std::io;
use std::path::Path;

use crate::*;

#[derive(Clone, Debug)]
struct UseItemPickerRow {
    label: String,
    detail: String,
    request: UseItemRequest,
}

impl PlayState {
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
            self.message = "Direction? Use C1HR8/C1HR6/C1HR2/C1HR4, or C1HR<space>.".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, REL_HUR_SPELL_INDEX, REL_HUR_COST)
        {
            return outcome;
        }

        if pass {
            self.advance_turn();
            self.message = "Wind change! Pass.".to_string();
            return MoveOutcome::Cast;
        }

        let previous = self.wind;
        let next = WindState::rel_hur_target(direction.expect("direction checked above"))
            .expect("inline Rel Hur parser returns cardinal directions only");
        self.apply_wind_state(next);
        self.advance_turn();
        self.message = format!(
            "Wind change! {} -> {}.",
            previous.status_message(),
            self.wind.status_message()
        );
        MoveOutcome::Cast
    }

    pub fn apply_wind_state(&mut self, wind: WindState) -> bool {
        if self.wind == WindState::Calm && wind == WindState::Calm {
            return false;
        }
        let changed = self.wind != wind;
        self.wind = wind;
        self.wind_save_byte = wind.save_byte();
        self.sail_cadence = 0;
        self.sail_stall_pending = false;
        changed
    }

    pub fn cast_gate_travel(
        &mut self,
        caster_index: usize,
        slot_index: usize,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        if matches!(self.player.transport, TransportState::Ship { .. }) {
            self.message = "Cannot Gate Travel shipboard.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, GATE_TRAVEL_SPELL_INDEX, GATE_TRAVEL_COST)
        {
            return Ok(outcome);
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
            GateTravelDestination::Empty => {
                self.message = format!("Gate Travel phase {phase} is not set.");
                Ok(MoveOutcome::Blocked)
            }
            GateTravelDestination::Invalid(reason) => {
                self.message = format!("Gate Travel phase {phase} is invalid: {reason}.");
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
                "Crown",
                "Wearing Crown.",
                "Removed Crown.",
            ),
            Some(UseItemRequest::AmuletOfLordBritish) => self.use_worn_regalia(
                SPECIAL_ITEM_AMULET_LB_INDEX,
                "Amulet",
                "Wearing Amulet.",
                "Removed Amulet.",
            ),
            Some(UseItemRequest::Sceptre) => self.use_sceptre_of_lord_british(),
            Some(UseItemRequest::BlackBadge) => self.use_worn_regalia(
                SPECIAL_ITEM_BLACK_BADGE_INDEX,
                "Black Badge",
                "Wearing Black Badge.",
                "Removed Black Badge.",
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
            Some(UseItemRequest::ShadowlordShard(index)) => self.use_shadowlord_shard(index),
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
            self.message = "No usable items.".to_string();
            return MoveOutcome::Blocked;
        }
        self.active_use = Some(UseSession::new());
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
        let rows = self.use_item_picker_rows();
        if rows.is_empty() {
            return "No usable items.".to_string();
        }

        let cursor = session.cursor.min(rows.len() - 1);
        let panel_start = (cursor / USE_PICKER_PANEL_ROWS) * USE_PICKER_PANEL_ROWS;
        let mut lines =
            vec!["Use: Enter activates; </> move; [] page; Space/Esc exits.".to_string()];
        for (index, row) in rows
            .iter()
            .enumerate()
            .skip(panel_start)
            .take(USE_PICKER_PANEL_ROWS)
        {
            let marker = if index == cursor { ">" } else { " " };
            lines.push(format!(
                "{marker} {:02}: {} ({})",
                index + 1,
                row.label,
                row.detail
            ));
        }
        if rows.len() > panel_start + USE_PICKER_PANEL_ROWS {
            lines.push(format!(
                "... {} more",
                rows.len() - panel_start - USE_PICKER_PANEL_ROWS
            ));
        }
        lines.join("\n")
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
                self.message = "Use closed.".to_string();
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
                self.move_use_cursor(&mut session, USE_PICKER_PANEL_ROWS as isize);
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            UseInputAction::PagePrevious => {
                self.move_use_cursor(&mut session, -(USE_PICKER_PANEL_ROWS as isize));
                self.message = self.render_use_session(&session);
                self.active_use = Some(session);
            }
            UseInputAction::Confirm => {
                let Some(row) = self.use_selected_item(&session) else {
                    self.message = "No usable items.".to_string();
                    return Ok(true);
                };
                if let Some(pending) = pending_action_for_use_request(row.request) {
                    let _ = self.use_item_command(Some(row.request), Some(game_dir))?;
                    session.pending = Some(pending);
                    self.message = self.render_use_session(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
                let turn_before = self.turn;
                let outcome = self.use_item_command(Some(row.request), Some(game_dir))?;
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
        if matches!(use_input_action(key), UseInputAction::Exit) {
            self.message = "Use closed.".to_string();
            return Ok(true);
        }

        let turn_before = self.turn;
        let outcome = match pending {
            UsePendingAction::PotionTarget { index } => {
                if let Some(target) = pending_use_party_target(key, suffix) {
                    if target < self.party.len() {
                        self.use_potion_consumed_target(index, target)
                    } else {
                        self.message = party_member_unavailable_message(self.party.len());
                        MoveOutcome::Blocked
                    }
                } else {
                    session.pending = Some(pending);
                    self.message = self.render_use_session(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
            UsePendingAction::ScrollWindDirection { .. } => {
                if let Some(direction) = pending_use_cardinal_direction(key, suffix) {
                    self.use_wind_change_scroll(Some(direction))
                } else {
                    session.pending = Some(pending);
                    self.message = self.render_use_session(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
            UsePendingAction::ScrollResurrectionTarget { index } => {
                if let Some(target) = pending_use_party_target(key, suffix) {
                    self.use_resurrection_scroll_consumed_target(index, target)
                } else {
                    session.pending = Some(pending);
                    self.message = self.render_use_session(&session);
                    self.active_use = Some(session);
                    return Ok(true);
                }
            }
        };
        self.apply_post_turn_effects_after_outcome(turn_before, game_dir, outcome)?;
        Ok(true)
    }

    fn render_pending_use_action(&self, pending: UsePendingAction) -> String {
        match pending {
            UsePendingAction::PotionTarget { index } => format!(
                "Use {}: choose party member (1-{}) or Space/Esc to exit.",
                potion_inventory_name(index),
                self.party.len().min(6)
            ),
            UsePendingAction::ScrollWindDirection { index } => format!(
                "Use Scroll {}: choose direction (8/6/2/4) or Space/Esc to exit.",
                scroll_label(index)
            ),
            UsePendingAction::ScrollResurrectionTarget { index } => format!(
                "Use Scroll {}: choose party member (1-{}) or Space/Esc to exit.",
                scroll_label(index),
                self.party.len().min(6)
            ),
        }
    }

    fn use_potion_consumed_target(&mut self, index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
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

    fn use_item_picker_rows(&self) -> Vec<UseItemPickerRow> {
        let mut rows = Vec::new();

        self.push_counted_use_row(
            &mut rows,
            SPECIAL_ITEM_MAGIC_CARPET_INDEX,
            "Magic Carpet",
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
            "Amulet of Lord British",
            UseItemRequest::AmuletOfLordBritish,
        );
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_CROWN_LB_INDEX,
            "Crown of Lord British",
            UseItemRequest::CrownOfLordBritish,
        );
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SCEPTRE_LB_INDEX,
            "Sceptre of Lord British",
            UseItemRequest::Sceptre,
        );
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SPYGLASS_INDEX,
            "Spyglass",
            UseItemRequest::Spyglass,
        );
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX,
            "HMS Cape Plans",
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
            SPECIAL_ITEM_SHARD_FALSEHOOD_INDEX,
            "Shard of Falsehood",
            UseItemRequest::ShadowlordShard(SHADOWLORD_FALSEHOOD_INDEX),
        );
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_HATRED_INDEX,
            "Shard of Hatred",
            UseItemRequest::ShadowlordShard(SHADOWLORD_HATRED_INDEX),
        );
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_SHARD_COWARDICE_INDEX,
            "Shard of Cowardice",
            UseItemRequest::ShadowlordShard(SHADOWLORD_COWARDICE_INDEX),
        );
        self.push_owned_use_row(
            &mut rows,
            SPECIAL_ITEM_WOODEN_BOX_INDEX,
            "Wooden Box",
            UseItemRequest::WoodenBox,
        );

        for (index, count) in self.scroll_stock.iter().copied().enumerate() {
            if count > 0 {
                rows.push(UseItemPickerRow {
                    label: format!("Scroll {}", scroll_label(index)),
                    detail: format!("stock {count}"),
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
                    label: potion_inventory_name(index).to_string(),
                    detail: format!("stock {count}"),
                    request: UseItemRequest::Potion {
                        index,
                        target: None,
                    },
                });
            }
        }
        if self.current_moonstone_bury_context().is_some() {
            for index in 0..MOONSTONE_SLOT_COUNT {
                rows.push(UseItemPickerRow {
                    label: format!("Moonstone phase {}", index + 1),
                    detail: if self.moonstone_slots[index].is_valid() {
                        "buried slot".to_string()
                    } else {
                        "carried".to_string()
                    },
                    request: UseItemRequest::Moonstone(index),
                });
            }
        }

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
                detail: format!("stock {count}"),
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
                detail: if value == SPECIAL_ITEM_WORN_VALUE {
                    "worn".to_string()
                } else {
                    "carried".to_string()
                },
                request,
            });
        }
    }

    pub fn use_wooden_box(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_WOODEN_BOX_INDEX] == 0 {
            self.message = "No Wooden Box!".to_string();
            return MoveOutcome::Blocked;
        }
        self.message = "Wooden Box: How use it?".to_string();
        MoveOutcome::PromptDeclined
    }

    pub fn use_shadowlord_shard(&mut self, index: usize) -> MoveOutcome {
        let Some(item_index) = shadowlord_shard_special_item_index(index) else {
            self.message = "No such Shard.".to_string();
            return MoveOutcome::Blocked;
        };
        let name = special_item_name(item_index);
        if self.special_items[item_index] == 0 {
            self.message = format!("No {name}!");
            return MoveOutcome::Blocked;
        }
        if !self.shadowlord_alive(index) {
            self.message = format!("{name}: matching Shadowlord is already vanquished.");
            return MoveOutcome::Blocked;
        }
        if !self.shadowlord_name_encounter_present(index) {
            self.message = format!("{name}: no matching Shadowlord is nearby.");
            return MoveOutcome::Blocked;
        }

        self.message = format!("{name}: no matching Eternal Flame is here.");
        MoveOutcome::Blocked
    }

    pub fn use_worn_regalia(
        &mut self,
        special_item_index: usize,
        missing_label: &str,
        wear_message: &str,
        remove_message: &str,
    ) -> MoveOutcome {
        if self.special_items[special_item_index] == 0 {
            self.message = format!("No {missing_label}!");
            return MoveOutcome::Blocked;
        }

        let was_worn = self.special_items[special_item_index] == SPECIAL_ITEM_WORN_VALUE;
        for index in [
            SPECIAL_ITEM_AMULET_LB_INDEX,
            SPECIAL_ITEM_CROWN_LB_INDEX,
            SPECIAL_ITEM_BLACK_BADGE_INDEX,
        ] {
            if self.special_items[index] == SPECIAL_ITEM_WORN_VALUE {
                self.special_items[index] = SPECIAL_ITEM_OWNED_VALUE;
            }
        }

        self.message = if was_worn {
            remove_message.to_string()
        } else {
            self.special_items[special_item_index] = SPECIAL_ITEM_WORN_VALUE;
            wear_message.to_string()
        };
        self.mark_visibility_dirty();
        self.advance_turn();
        MoveOutcome::Used
    }

    pub fn use_sceptre_of_lord_british(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SCEPTRE_LB_INDEX] == 0 {
            self.message = "No Sceptre!".to_string();
            return MoveOutcome::Blocked;
        }
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let dissolved = self.dissolve_sceptre_barriers_near_party();
        if dissolved == 0 {
            self.message = "Wielded Sceptre: No effect.".to_string();
            return MoveOutcome::Blocked;
        }

        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Wielded Sceptre: dissolved {dissolved} barrier cell(s).");
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
            self.message = "No HMS Cape Plans!".to_string();
            return MoveOutcome::Blocked;
        }
        if !matches!(self.player.transport, TransportState::Ship { .. }) {
            self.message = "Not aboard ship!".to_string();
            return MoveOutcome::Blocked;
        }

        self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX] =
            self.special_items[SPECIAL_ITEM_HMS_CAPE_PLANS_INDEX].max(2);
        self.advance_turn();
        self.message = "Ship rigged for double speed.".to_string();
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
            self.message = "No Magic Carpet!".to_string();
            return MoveOutcome::Blocked;
        }
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if !self.player.transport.is_foot() {
            self.message = "On foot.".to_string();
            return MoveOutcome::Blocked;
        }

        let tile = self.current_area_tile(self.player.x, self.player.y);
        let transport = TransportState::Carpet {
            type_byte: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
            tile: FIRST_PLAYABLE_MAGIC_CARPET_TILE,
        };
        if !is_tile_walkable_for_transport(tile, self.passability.as_ref(), transport) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX] =
            self.special_items[SPECIAL_ITEM_MAGIC_CARPET_INDEX].saturating_sub(1);
        self.player.transport = transport;
        self.timing_status = TimingStatusTag::for_transport(transport);
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = "Boarded carpet.".to_string();
        MoveOutcome::Boarded
    }

    pub fn use_sextant(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] == 0 {
            self.message = "No Sextant!".to_string();
            return MoveOutcome::Blocked;
        }
        if !matches!(self.area, Area::World { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if !is_town_night_hour(self.clock.hour) {
            self.message = "Cannot see the stars!".to_string();
            return MoveOutcome::Blocked;
        }

        let y = sextant_coordinate(self.player.y);
        let x = sextant_coordinate(self.player.x);
        self.advance_turn();
        // magic.md §8 / inventory.md §7: shared sextant printer is Y first,
        // then a comma and the X-coordinate, with a trailing double quote.
        self.message = format!("Sextant: {y},{x}\"");
        MoveOutcome::Used
    }

    pub fn use_pocket_watch(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_POCKET_WATCH_INDEX] == 0 {
            self.message = "No Pocket Watch!".to_string();
            return MoveOutcome::Blocked;
        }
        self.advance_turn();
        self.message = format!(
            "Pocket Watch: {} {}",
            self.clock.display_hour(),
            self.clock.am_pm_suffix()
        );
        MoveOutcome::Used
    }

    pub fn use_spyglass(&mut self) -> MoveOutcome {
        if self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] == 0 {
            self.message = "No Spyglass!".to_string();
            return MoveOutcome::Blocked;
        }
        if !matches!(
            self.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        ) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if !is_town_night_hour(self.clock.hour) {
            self.message = "Cannot see the stars!".to_string();
            return MoveOutcome::Blocked;
        }

        self.message = format!(
            "Spyglass: Looking at the stars over Britannia.\n{}",
            self.britannia_chunk_overview_map()
        );
        MoveOutcome::Observed
    }

    pub fn britannia_chunk_overview_map(&self) -> String {
        let mut out = String::new();
        let current_chunk_x = self.player.x / CHUNK_SIDE;
        let current_chunk_y = self.player.y / CHUNK_SIDE;
        for row in 0..8 {
            for col in 0..22 {
                let chunk_x =
                    (current_chunk_x + WORLD_CHUNKS_PER_SIDE + col - 11) % WORLD_CHUNKS_PER_SIDE;
                let chunk_y =
                    (current_chunk_y + WORLD_CHUNKS_PER_SIDE + row - 4) % WORLD_CHUNKS_PER_SIDE;
                if chunk_x == current_chunk_x && chunk_y == current_chunk_y {
                    out.push('+');
                    continue;
                }
                let sample_x = chunk_x * CHUNK_SIDE + CHUNK_SIDE / 2;
                let sample_y = chunk_y * CHUNK_SIDE + CHUNK_SIDE / 2;
                let tile = self.grid[world_cell_index(sample_x, sample_y)];
                let tile = self.animation.resolve_static_tile(tile);
                out.push(render_surface_view_class(surface_view_class(tile)));
            }
            out.push('\n');
        }
        out
    }

    pub fn use_scroll(
        &mut self,
        index: usize,
        direction: Option<Direction>,
        target: Option<usize>,
    ) -> MoveOutcome {
        let label = scroll_label(index);
        if index >= SCROLL_COUNT || self.scroll_stock[index] == 0 {
            self.message = format!("No {label} scroll!");
            return MoveOutcome::Blocked;
        }
        self.scroll_stock[index] = self.scroll_stock[index].saturating_sub(1);

        match index {
            SCROLL_LIGHT_INDEX => {
                self.light_spell_counter = SCROLL_LIGHT_DURATION;
                self.recompute_daylight();
                self.advance_turn();
                self.message = "Light!".to_string();
                MoveOutcome::Used
            }
            SCROLL_WIND_CHANGE_INDEX => self.use_wind_change_scroll(direction),
            SCROLL_PROTECTION_INDEX => {
                self.active_effect_tag = Some(PROTECTION_ACTIVE_EFFECT_TAG);
                self.active_effect_counter = SCROLL_PROTECTION_DURATION;
                self.advance_turn();
                self.message = "Protection!".to_string();
                MoveOutcome::Used
            }
            SCROLL_NEGATE_MAGIC_INDEX => {
                self.active_effect_tag = Some(NEGATE_MAGIC_ACTIVE_EFFECT_TAG);
                self.active_effect_counter = SCROLL_NEGATE_MAGIC_DURATION;
                self.advance_turn();
                self.message = "Negate magic!".to_string();
                MoveOutcome::Used
            }
            SCROLL_VIEW_INDEX => {
                if self.combat_active {
                    self.message = "Not here!".to_string();
                    return MoveOutcome::Blocked;
                }
                self.advance_turn();
                let view_message = self.activate_peer_view_overlay();
                self.message = format!("View!\n{view_message}");
                MoveOutcome::Observed
            }
            SCROLL_SUMMON_DAEMON_INDEX => {
                if !self.combat_active {
                    self.message = "Not here!".to_string();
                    return MoveOutcome::Blocked;
                }
                let seed = self.combat_summon_placement_seed(0, SUMMON_DAEMON_SPELL_INDEX);
                let applied =
                    self.apply_combat_summon_class_around_slot(COMBAT_CLASS_DAEMON, 0, seed);
                self.advance_turn();
                self.message = if applied.is_some() {
                    "Summon Daemon!".to_string()
                } else {
                    "Failed!".to_string()
                };
                if applied.is_some() {
                    MoveOutcome::Used
                } else {
                    MoveOutcome::Blocked
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
            self.message = "Direction? Use UHR8/UHR6/UHR2/UHR4.".to_string();
            return MoveOutcome::Blocked;
        };
        if matches!(self.area, Area::Dungeon { .. }) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }

        let previous = self.wind;
        let next = WindState::rel_hur_target(direction)
            .expect("inline wind scroll parser returns cardinal directions only");
        self.apply_wind_state(next);
        self.advance_turn();
        self.message = format!(
            "Wind change! {} -> {}.",
            previous.status_message(),
            self.wind.status_message()
        );
        MoveOutcome::Used
    }

    pub fn use_resurrection_scroll(&mut self, target: Option<usize>) -> MoveOutcome {
        if self.combat_active {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        let Some(target_index) = target else {
            self.message = "Whom? Use UCIM2 to resurrect party member 2.".to_string();
            return MoveOutcome::Blocked;
        };
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if self.party[target_index].status != b'D' {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let max_hp = self
            .resurrect_party_member_to_hp(target_index, 1)
            .expect("target status checked before scroll resurrection");
        self.advance_turn();
        self.message = format!(
            "Resurrection! party member {} (1/{max_hp}).",
            target_index + 1
        );
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
            return MoveOutcome::Blocked;
        }

        self.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        self.active_effect_counter = SCROLL_NEGATE_TIME_DURATION;
        self.advance_turn();
        self.message = "Negate time!".to_string();
        MoveOutcome::Used
    }

    pub fn use_potion(&mut self, index: usize, target: Option<usize>) -> MoveOutcome {
        let label = potion_label(index);
        if index >= POTION_COUNT || self.potion_stock[index] == 0 {
            self.message = format!("No {label} potion!");
            return MoveOutcome::Blocked;
        }
        self.potion_stock[index] = self.potion_stock[index].saturating_sub(1);

        let Some(target_index) = target else {
            self.message = format!(
                "Who? Use U{}1 for party member 1.",
                label.to_ascii_uppercase()
            );
            return MoveOutcome::Blocked;
        };
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
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
        let selected_label = potion_label(selected_index);
        let effect_label = potion_label(effect_index);
        let prefix = if selected_index == effect_index {
            format!("{selected_label} potion")
        } else {
            format!("{selected_label} potion ({effect_label} effect)")
        };

        match effect_index {
            POTION_BLUE_INDEX => {
                if self.party[target_index].status == b'S' && self.party[target_index].hp > 0 {
                    self.party[target_index].status = b'G';
                    if self.clear_combat_party_sleep_presentation(target_index) {
                        self.mark_visibility_dirty();
                    }
                    self.advance_turn();
                    self.message = format!("{prefix}: Awakened party member {}.", target_index + 1);
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = format!("{prefix}: No effect.");
                    MoveOutcome::Blocked
                }
            }
            POTION_YELLOW_INDEX => {
                if !self.party[target_index].living() {
                    self.advance_turn();
                    self.message = format!("{prefix}: No effect.");
                    return MoveOutcome::Blocked;
                }
                let amount = self.potion_heal_amount(selected_index, target_index);
                let healed = self.party[target_index].heal_by(amount);
                let hp = self.party[target_index].hp;
                let max_hp = self.party[target_index].max_hp;
                self.advance_turn();
                self.message = format!(
                    "{prefix}: Healed party member {} for {healed} HP ({hp}/{max_hp}).",
                    target_index + 1
                );
                MoveOutcome::Used
            }
            POTION_RED_INDEX => {
                if self.party[target_index].status == b'P' {
                    self.party[target_index].status = b'G';
                    self.advance_turn();
                    self.message = format!("{prefix}: Cured party member {}.", target_index + 1);
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = format!("{prefix}: No effect.");
                    MoveOutcome::Blocked
                }
            }
            POTION_GREEN_INDEX => {
                if self.party[target_index].status == b'G' && self.party[target_index].hp > 0 {
                    self.party[target_index].status = b'P';
                    self.advance_turn();
                    self.message = format!("{prefix}: Poisoned party member {}.", target_index + 1);
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = format!("{prefix}: No effect.");
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
                    self.message = format!("{prefix}: Slept party member {}.", target_index + 1);
                    MoveOutcome::Used
                } else {
                    self.advance_turn();
                    self.message = format!("{prefix}: No effect.");
                    MoveOutcome::Blocked
                }
            }
            POTION_PURPLE_INDEX => {
                if !self.combat_active {
                    self.advance_turn();
                    self.message = format!("{prefix}: No noticeable effect.");
                    return MoveOutcome::Blocked;
                }
                let applied = self.apply_combat_potion_poof_presentation(target_index);
                if applied {
                    self.mark_visibility_dirty();
                }
                self.advance_turn();
                self.message = if applied {
                    format!("{prefix}: Poof!")
                } else {
                    format!("{prefix}: No effect.")
                };
                if applied {
                    MoveOutcome::Used
                } else {
                    MoveOutcome::Blocked
                }
            }
            POTION_BLACK_INDEX => {
                if !self.combat_active {
                    self.advance_turn();
                    self.message = format!("{prefix}: No noticeable effect.");
                    return MoveOutcome::Blocked;
                }
                self.advance_turn();
                let applied = self.apply_combat_party_invisibility_potion(target_index);
                self.message = if applied {
                    format!("{prefix}: Invisible party member {}.", target_index + 1)
                } else {
                    format!("{prefix}: No effect.")
                };
                if applied {
                    MoveOutcome::Used
                } else {
                    MoveOutcome::Blocked
                }
            }
            POTION_WHITE_INDEX => {
                if matches!(self.area, Area::Dungeon { .. }) || self.combat_active {
                    self.advance_turn();
                    self.message = format!("{prefix}: No noticeable effect.");
                    return MoveOutcome::Blocked;
                }
                self.start_white_potion_sweep();
                self.advance_turn();
                self.message = format!("{prefix}: Visibility sweep.");
                MoveOutcome::Observed
            }
            _ => {
                self.advance_turn();
                self.message = "unknown potion: No effect.".to_string();
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
        let Some(presentation) = self.combat_potion_presentation else {
            return false;
        };
        if presentation.kind == CombatPotionPresentationKind::Sleep
            && presentation.actor_slot == target_index
        {
            self.combat_potion_presentation = None;
            true
        } else {
            false
        }
    }

    pub fn apply_combat_party_sleep_presentation(&mut self, target_index: usize) -> bool {
        self.set_combat_potion_presentation(
            target_index,
            CombatPotionPresentationKind::Sleep,
            COMBAT_POTION_SLEEP_PRESENTATION_FRAMES,
        )
    }

    pub fn apply_combat_potion_poof_presentation(&mut self, target_index: usize) -> bool {
        self.set_combat_potion_presentation(
            target_index,
            CombatPotionPresentationKind::Poof,
            COMBAT_POTION_POOF_PRESENTATION_FRAMES,
        )
    }

    fn set_combat_potion_presentation(
        &mut self,
        target_index: usize,
        kind: CombatPotionPresentationKind,
        frames_remaining: u8,
    ) -> bool {
        if !self.combat_active || target_index >= COMBAT_PARTY_ACTOR_SLOTS {
            return false;
        }
        let Some(actor) = self.combat_actors.get(target_index).copied() else {
            return false;
        };
        if !combat_actor_is_active_not_dead(actor) {
            return false;
        }
        let active_object_slot = usize::from(actor.active_object_slot);
        if active_object_slot >= self.active_objects.len() {
            return false;
        }
        self.combat_potion_presentation = Some(CombatPotionPresentation {
            kind,
            actor_slot: target_index,
            active_object_slot,
            frames_remaining,
        });
        true
    }

    pub fn start_white_potion_sweep(&mut self) {
        self.white_potion_sweep = Some(WhitePotionSweep {
            frames_remaining: POTION_WHITE_SWEEP_FRAMES,
            radius: POTION_WHITE_SWEEP_RADIUS,
            center_x: self.player.x,
            center_y: self.player.y,
        });
        self.mark_visibility_dirty();
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
            self.message = format!("Cannot bury Moonstone on tile {tile}.");
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

    pub fn cast_spell_resource_gate(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
    ) -> Option<MoveOutcome> {
        let Some(caster) = self.party.get(caster_index).copied() else {
            self.message = "Nobody can cast!".to_string();
            return Some(MoveOutcome::Blocked);
        };
        if !caster.conscious() {
            self.message = "Nobody can cast!".to_string();
            return Some(MoveOutcome::Blocked);
        }
        if self.spell_charges[spell_index] == 0 {
            self.message = "None mixed!".to_string();
            return Some(MoveOutcome::Blocked);
        }

        self.spell_charges[spell_index] = self.spell_charges[spell_index].saturating_sub(1);
        if self.party[caster_index].mana < mana_cost {
            self.message = "M.P. too low!".to_string();
            self.advance_turn();
            return Some(MoveOutcome::Blocked);
        }
        self.party[caster_index].mana -= mana_cost;
        if self.party[caster_index].level < mana_cost {
            self.message = "M.P. too low!".to_string();
            self.advance_turn();
            return Some(MoveOutcome::Blocked);
        }

        None
    }

    pub fn apply_gate_travel(
        &mut self,
        game_dir: &Path,
        phase: usize,
        target: PlayTarget,
        floor: i8,
        start: (usize, usize),
    ) -> io::Result<()> {
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
            party_intelligence: self.party_intelligence.clone(),
            party_equipment: self.party_equipment.clone(),
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
            shrine_ordained_mask: self.shrine_ordained_mask,
            shrine_codex_mask: self.shrine_codex_mask,
            shrine_standing: self.shrine_standing,
            moral_standing: self.moral_standing,
            avatar_stats: self.avatar_stats,
            torches: self.torches,
            torch_counter: self.torch_counter,
            light_spell_counter: self.light_spell_counter,
            wind: self.wind,
            wind_save_byte: self.wind_save_byte,
            timing_status: TimingStatusTag::Normal,
            time_stop_counter: self.time_stop_counter,
            active_effect_tag: self.active_effect_tag,
            active_effect_counter: self.active_effect_counter,
            fortunes_of_war: self.fortunes_of_war,
            active_player: self.active_player,
            combat_round_counter: self.combat_round_counter,
            transport: TransportState::Foot,
            facing: Some(self.player.facing),
            pending_vehicle: None,
            inn_registry: self.inn_registry.clone(),
            initial_britannia_overlay: self.world_overlays.get(WorldPlane::Britannia),
            debug_enter: self.debug_enter,
            saved_active_objects: None,
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
        next.pending_moongate = None;
        next.pending_town_arrest = None;
        next.active_blackthorn = None;
        next.message = format!(
            "Gate Travel phase {phase} -> {} at ({}, {}).",
            target.key(),
            start.0,
            start.1
        );
        *self = next;
        Ok(())
    }

    pub fn turn_dungeon(&mut self, clockwise: bool) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Turn is only meaningful in dungeon mode.".to_string();
            return MoveOutcome::Blocked;
        };
        let next = if clockwise {
            self.player.facing.turn_right_cardinal()
        } else {
            self.player.facing.turn_left_cardinal()
        };
        let Some(next) = next else {
            self.message = "Dungeon turn requires a cardinal facing direction.".to_string();
            return MoveOutcome::Blocked;
        };
        self.player.facing = next;
        self.advance_turn();
        self.message = format!(
            "Turned to face {} on {} ({}) level {level}.",
            next.name(),
            scene.key(),
            scene.name()
        );
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
            self.message = "Look is only implemented for dungeon mode in this slice.".to_string();
            return MoveOutcome::Blocked;
        };
        if !self.has_personal_light() {
            self.message = "You see: darkness.".to_string();
            return MoveOutcome::Observed;
        }
        let (x, y) = self.dungeon_look_focus_coord(focus);

        let tile = self.dungeon_cell(level, x, y);
        let description = dungeon_look_description(tile);
        if (tile >> 4) == 0x5 {
            self.message = match drink {
                None => {
                    let member_index = party_index.unwrap_or(0);
                    return self.start_dungeon_fountain_drink_prompt(member_index, focus);
                }
                Some(false) => "You see: a fountain. Will you drink? No.".to_string(),
                Some(true) => {
                    let member_index = party_index.unwrap_or(0);
                    match self.apply_dungeon_fountain_effect(member_index, tile) {
                        Some(report) => {
                            format!("You see: a fountain. Will you drink? Yes. {report}")
                        }
                        None => format!(
                            "You see: a fountain. Will you drink? Yes, but party member {} is unavailable.",
                            member_index + 1
                        ),
                    }
                }
            };
            return if drink == Some(false) {
                MoveOutcome::PromptDeclined
            } else {
                MoveOutcome::Observed
            };
        }

        self.message = format!("You see: {description}.");
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
                Some(format!(
                    "Cured! slot {slot} status {} -> good",
                    party_status_name(before)
                ))
            }
            1 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let (before, after) = member.heal_to_max();
                Some(format!("Healed! slot {slot} HP {before}->{after}"))
            }
            2 => {
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                member.status = b'P';
                Some(format!("Poisoned! slot {slot} is poisoned"))
            }
            _ => {
                let damage = self.dungeon_fountain_damage_roll(member_index, tile);
                let member = self.party.get_mut(member_index)?;
                let slot = member.slot;
                let applied = member.apply_damage(damage);
                Some(format!(
                    "Bad taste. slot {slot} took {applied} HP ({} HP left)",
                    member.hp
                ))
            }
        }
    }

    pub fn view_gem(&mut self) -> MoveOutcome {
        if self.gems == 0 {
            self.message = "No gems!".to_string();
            return MoveOutcome::Blocked;
        }

        match self.area {
            Area::Dungeon { scene, level } => {
                self.gems = self.gems.saturating_sub(1);
                let title = format!(
                    "Dungeon view of {} ({}) level {} ({} gem(s) remain; centered flood map)",
                    scene.key(),
                    scene.name(),
                    level,
                    self.gems
                );
                let text_map = self.dungeon_vision_map(level);
                self.active_view_overlay = Some(ViewOverlay {
                    title: title.clone(),
                    text_map: text_map.clone(),
                    kind: ViewOverlayKind::Dungeon { level },
                });
                self.message = format!("{title}:\n{text_map}");
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
                });
                self.message = format!("{title}:\n{text_map}");
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
                });
                self.message = format!("{title}:\n{text_map}");
                MoveOutcome::Observed
            }
        }
    }

    pub fn clear_active_view_overlay(&mut self) {
        self.active_view_overlay = None;
        self.message = "View closed.".to_string();
    }

    pub fn render_active_view_overlay(&self, depth: TileGraphicsDepth) -> Option<TileViewport> {
        match self.active_view_overlay.as_ref()?.kind {
            ViewOverlayKind::Surface => Some(self.render_surface_view_overlay(depth)),
            ViewOverlayKind::Dungeon { level } => {
                Some(self.render_dungeon_view_overlay(level, depth))
            }
        }
    }

    pub fn render_surface_view_overlay(&self, depth: TileGraphicsDepth) -> TileViewport {
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
                draw_surface_view_cell(&mut viewport, cell_x, cell_y, scale, class, tile, false);
            }
        }
        draw_surface_view_cell(&mut viewport, cells / 2, cells / 2, scale, 0, 0, true);
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
            Area::World { plane } => {
                let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                if let Some(object) = self.world_object_at(wx, wy) {
                    object.tile
                } else if self.visible_moongate_at(plane, wx, wy) {
                    self.animation.resolve_moongate_tile()
                } else {
                    let tile = self.grid[world_cell_index(wx, wy)];
                    self.animation.resolve_static_tile(tile)
                }
            }
            Area::Dungeon { .. } => 0,
        }
    }

    pub fn render_dungeon_view_overlay(&self, level: u8, depth: TileGraphicsDepth) -> TileViewport {
        let radius = DUNGEON_GEM_VIEW_RADIUS;
        let cells = (radius * 2 + 1) as usize;
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
        let map = self.dungeon_vision_map(level);
        for (cell_y, row) in map.lines().enumerate() {
            for (cell_x, glyph) in row.chars().enumerate() {
                draw_dungeon_view_cell(&mut viewport, cell_x, cell_y, scale, glyph);
            }
        }
        viewport
    }

    pub fn dungeon_vision_map(&self, level: u8) -> String {
        let radius = DUNGEON_GEM_VIEW_RADIUS;
        let side = (radius * 2 + 1) as usize;
        let center = radius as usize;
        let mut visible = vec![false; side * side];
        let mut queue = VecDeque::new();

        let center_index = center * side + center;
        visible[center_index] = true;
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
                    if next_x < -radius || next_x > radius || next_y < -radius || next_y > radius {
                        continue;
                    }
                    let scratch_x = (next_x + radius) as usize;
                    let scratch_y = (next_y + radius) as usize;
                    let index = scratch_y * side + scratch_x;
                    if !visible[index] {
                        visible[index] = true;
                        queue.push_back((next_x, next_y));
                    }
                }
            }
        }

        let mut out = String::new();
        for scratch_y in 0..side {
            for scratch_x in 0..side {
                let index = scratch_y * side + scratch_x;
                if scratch_x == center && scratch_y == center {
                    out.push('@');
                } else if visible[index] {
                    let dx = scratch_x as isize - radius;
                    let dy = scratch_y as isize - radius;
                    let world_x =
                        (self.player.x as isize + dx).rem_euclid(DUNGEON_SIDE as isize) as usize;
                    let world_y =
                        (self.player.y as isize + dy).rem_euclid(DUNGEON_SIDE as isize) as usize;
                    out.push(render_dungeon_glyph(
                        self.dungeon_cell(level, world_x, world_y),
                    ));
                } else {
                    out.push(' ');
                }
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
            Area::World { plane } => {
                for y in origin_y..origin_y + side {
                    for x in origin_x..origin_x + side {
                        let wx = x.rem_euclid(WORLD_SIDE as isize) as usize;
                        let wy = y.rem_euclid(WORLD_SIDE as isize) as usize;
                        if wx == self.player.x && wy == self.player.y {
                            out.push('@');
                        } else if let Some(object) = self.world_object_at(wx, wy) {
                            out.push(render_surface_view_class(surface_view_class(object.tile)));
                        } else if self.visible_moongate_at(plane, wx, wy) {
                            out.push(render_surface_view_class(surface_view_class(
                                self.animation.resolve_moongate_tile(),
                            )));
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
        let look_table = load_look_table(game_dir)?;
        self.look_facing_with_resources(Some(&look_table), Some(game_dir))
    }

    #[cfg(test)]
    pub fn look_facing_with_table(&mut self, look_table: Option<&LookTable>) -> MoveOutcome {
        self.look_facing_with_resources(look_table, None)
            .expect("look without a game dir cannot perform file-backed look context")
    }

    pub fn look_facing_with_resources(
        &mut self,
        look_table: Option<&LookTable>,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Dungeon { .. } => Ok(self.look_dungeon()),
            Area::Town { .. } => {
                let (dx, dy) = self.player.facing.delta();
                let x = self.player.x as isize + dx;
                let y = self.player.y as isize + dy;
                if !(0..32).contains(&x) || !(0..32).contains(&y) {
                    self.message = "You see: the location boundary.".to_string();
                    return Ok(MoveOutcome::Observed);
                }
                let x = x as usize;
                let y = y as usize;
                if let Some(object) = self.blocking_object_at(x, y) {
                    self.message = if look_table.is_some() {
                        format!(
                            "You see: {} at ({x}, {y}).",
                            self.look_object_description(object.tile, look_table)
                        )
                    } else {
                        format!("You see: an actor tile {} at ({x}, {y}).", object.tile)
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
                self.message = format!(
                    "You see: {} at ({x}, {y}).",
                    self.look_description(tile, look_table)
                );
                Ok(MoveOutcome::Observed)
            }
            Area::World { plane } => {
                let (dx, dy) = self.player.facing.delta();
                let x = (self.player.x as isize + dx).rem_euclid(WORLD_SIDE as isize) as usize;
                let y = (self.player.y as isize + dy).rem_euclid(WORLD_SIDE as isize) as usize;
                if let Some(object) = self.world_object_at(x, y) {
                    self.message = if look_table.is_some() {
                        format!(
                            "You see: {} at ({x}, {y}).",
                            self.look_object_description(object.tile, look_table)
                        )
                    } else {
                        format!("You see: an object tile {} at ({x}, {y}).", object.tile)
                    };
                    return Ok(MoveOutcome::Observed);
                }
                let tile = self.grid[world_cell_index(x, y)];
                if tile == BRITANNIA_CHUNK_MAP_LOOK_TRIGGER_TILE {
                    self.message = format!(
                        "Britannia overview from Look at ({x}, {y}) on {}:\n{}",
                        plane.key(),
                        self.britannia_chunk_overview_map()
                    );
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
                let description =
                    self.look_description_for_world_tile(tile, look_table, game_dir, plane, x, y)?;
                self.message =
                    format!("You see: {} at ({x}, {y}) on {}.", description, plane.key());
                Ok(MoveOutcome::Observed)
            }
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
            return Ok(base);
        }
        let Some(name) = self.world_dungeon_name_at(game_dir, plane, x, y, tile)? else {
            return Ok(base);
        };
        Ok(format!("{base} ({name})"))
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
        Ok(load_world_location_entries(game_dir)?.and_then(|entries| {
            entries.into_iter().find_map(|entry| {
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
            })
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
            return Ok(MoveOutcome::Blocked);
        }
        let dialogue = parse_tlk(&game_dir.join(format!("{}.TLK", scene.family.stem())))?;
        let raw_blob = parse_tlk_raw(&game_dir.join(format!("{}.TLK", scene.family.stem())))
            .unwrap_or_default();
        Ok(self
            .talk_direction_with_dialogue_and_keyword_raw(direction, &dialogue, &raw_blob, keyword))
    }

    pub fn facing_talk_target(&self) -> Option<(u8, usize, usize)> {
        self.talk_target_in_direction(self.player.facing)
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

    pub fn talk_direction_with_dialogue_and_keyword(
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
            return MoveOutcome::Blocked;
        }

        let Some((dialog_id, _, _)) = self.talk_target_in_direction(direction) else {
            self.message = TALK_NOBODY_HERE_MESSAGE.to_string();
            return MoveOutcome::Blocked;
        };

        if let Some((role, _family)) = talk_shop_trigger(dialog_id) {
            if self.player.transport.is_horse() && dialog_id != 0x83 {
                self.message =
                    format!("{role} refuses thee on horseback; dismount before commerce.");
                return MoveOutcome::Blocked;
            }
            let scene_byte = match self.area {
                Area::Town { scene, .. } => Some(scene.byte),
                _ => None,
            };
            if let Some(session) =
                crate::shop_session::shop_session_for_talk_context(dialog_id, scene_byte)
            {
                self.advance_turn();
                let label = session.shop_label().to_string();
                let prompt = session.opening_prompt().to_string();
                self.active_shop = Some(session);
                self.message = format!("{label} is now open. {prompt}");
                return MoveOutcome::Talked;
            }
        }
        if dialog_id == 0 {
            self.message = "They give thee a funny look.".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(fields) = dialogue.get(&(dialog_id as u16)) else {
            self.message = format!("Dialogue id {dialog_id} is unresolved for this scene.");
            return MoveOutcome::Blocked;
        };
        if fields.len() < 3 {
            self.message = format!("Dialogue id {dialog_id} has no complete talk envelope.");
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
                self.message = format!("Dialogue id {dialog_id} has no complete talk envelope.");
                return MoveOutcome::Talked;
            }
            let response = talk_keyword_response(fields, keyword)
                .filter(|response| !response.is_empty())
                .unwrap_or(TLK_NO_KEYWORD_MATCH_MESSAGE);
            let (response, actions) = talk_response_text_and_actions(response);
            self.apply_talk_action_grants(&actions);
            self.message = format!("Talked to {name}: {response}");
        } else {
            let (greeting, actions) = talk_response_text_and_actions(greeting);
            self.apply_talk_action_grants(&actions);
            self.message = format!("Talked to {name}: {description}. {greeting} Your interest?");
        }
        MoveOutcome::Talked
    }

    /// Talk dispatch wrapper that uses the byte-runner against the raw
    /// blob bytes when they are available, falling back to the existing
    /// string-based path otherwise. The richer renderer expands engine
    /// control bytes (printable text, action dispatch, IF/ELSE, GOTO
    /// labels) per `systems/conversation.md` §7 and updates active-scene
    /// branch flags so subsequent visits see the same set-flag state.
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
        if !matches!(self.area, Area::Town { .. }) {
            self.message = "Funny, no response!".to_string();
            return MoveOutcome::Blocked;
        }
        if self.talk_liveness_blocked() {
            return MoveOutcome::Blocked;
        }

        let Some((dialog_id, _, _)) = self.talk_target_in_direction(direction) else {
            self.message = TALK_NOBODY_HERE_MESSAGE.to_string();
            return MoveOutcome::Blocked;
        };

        if let Some((role, family)) = talk_shop_trigger(dialog_id) {
            if self.player.transport.is_horse() && dialog_id != 0x83 {
                self.message =
                    format!("{role} refuses thee on horseback; dismount before commerce.");
                return MoveOutcome::Blocked;
            }
            let scene_byte = match self.area {
                Area::Town { scene, .. } => Some(scene.byte),
                _ => None,
            };
            if let Some(session) =
                crate::shop_session::shop_session_for_talk_context(dialog_id, scene_byte)
            {
                self.advance_turn();
                let label = session.shop_label().to_string();
                let prompt = session.opening_prompt().to_string();
                self.active_shop = Some(session);
                self.message = format!("{label} is now open. {prompt} Dispatch family: {family}.");
                return MoveOutcome::Talked;
            }
        }
        if dialog_id == 0 {
            self.message = "They give thee a funny look.".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(fields) = dialogue.get(&(dialog_id as u16)) else {
            self.message = format!("Dialogue id {dialog_id} is unresolved for this scene.");
            return MoveOutcome::Blocked;
        };
        if fields.len() < 3 {
            self.message = format!("Dialogue id {dialog_id} has no complete talk envelope.");
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
                let description_text = self.render_raw_conversation_description(raw_fields, fields);
                let session = crate::conversation_session::ConversationSession::new(
                    raw_fields.clone(),
                    fields.clone(),
                );
                self.active_conversation = Some(Box::new(session));
                let greeting_text = self.advance_active_conversation_greeting();
                let prompt = TLK_KEYWORD_PROMPT
                    .lines()
                    .next()
                    .unwrap_or("Your interest?");
                self.message =
                    format!("Talked to {name}: {description_text}. {greeting_text} {prompt}");
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
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: None,
            curse_seen: false,
            gold_payment_accepted: true,
            gold_available: Some(self.gold),
            ask_party_name_response: 0,
            ask_who_response: 0,
            yield_on_pause: false,
            yield_on_ask: false,
            yield_on_gold_payment: false,
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

        if let Some(keyword) = keyword {
            if fields.len() < 5 {
                self.message = format!("Dialogue id {dialog_id} has no complete talk envelope.");
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
            self.record_tlk_signal_flags(&applied_signal_flags);
            self.apply_talk_action_grants(&legacy_actions);
            if let Some(scene) = scene_for_flags {
                self.merge_talk_branch_flags(scene, applied_flags);
            }
            self.message = format!("Talked to {name}: {legacy_text}");
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
            self.record_tlk_signal_flags(&applied_signal_flags);
            self.apply_talk_action_grants(&legacy_actions);
            if let Some(scene) = scene_for_flags {
                self.merge_talk_branch_flags(scene, applied_flags);
            }
            self.message = format!("Talked to {name}: {description}. {legacy_text} Your interest?");
        }
        MoveOutcome::Talked
    }

    /// Apply the byte-runner's recorded [`TlkActionDispatchVerb`] grants
    /// to the live runtime counters per `conversation.md §7.6`.
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
                    self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1;
                }
                TlkActionDispatchVerb::SetSpyglassCarried => {
                    self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 1;
                }
                TlkActionDispatchVerb::SetBlackBadgeCarried => {
                    self.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] = 1;
                }
                TlkActionDispatchVerb::RaiseSkullKeys => {
                    let slot = &mut self.special_items[SPECIAL_ITEM_SKULL_KEY_INDEX];
                    *slot = (*slot).saturating_add(1).min(PARTY_BYTE_STOCK_CAP);
                }
            }
        }
    }

    /// Apply accepted conversation gold payments emitted by TLK `0x85`.
    pub fn apply_tlk_gold_payments(
        &mut self,
        payments: &[crate::conversation_session::ConversationGoldPayment],
    ) {
        for payment in payments {
            if payment.accepted && self.gold >= payment.amount {
                self.gold -= payment.amount;
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
        let inputs = crate::tlk_runner::TlkRunInputs {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: None,
            curse_seen: false,
            gold_payment_accepted: true,
            gold_available: Some(self.gold),
            ask_party_name_response: 0,
            ask_who_response: 0,
            yield_on_pause: false,
            yield_on_ask: false,
            yield_on_gold_payment: false,
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
        let (dialog_id, _, _) = self.facing_talk_target()?;
        if dialog_id == 0 || talk_shop_trigger(dialog_id).is_some() {
            return None;
        }
        let fields = dialogue.get(&(dialog_id as u16))?;
        let raw = raw_blob.get(&(dialog_id as u16))?;
        let session =
            crate::conversation_session::ConversationSession::new(raw.clone(), fields.clone());
        self.active_conversation = Some(Box::new(session));
        // Run the greeting now so the caller sees the opening line.
        Some(self.advance_active_conversation_greeting())
    }

    /// Render the active conversation's greeting and put it in
    /// `state.message`. Returns the rendered text.
    pub fn advance_active_conversation_greeting(&mut self) -> String {
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
        let ctx = crate::conversation_session::ConversationContext {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: None,
            gold_payment_accepted: true,
            gold_available: Some(self.gold),
            party_member_names: &party_member_names,
        };
        let mut text = String::new();
        if let Some(session) = self.active_conversation.as_mut() {
            let output = session.present_greeting(&ctx);
            text = output.text.clone();
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_gold_payments(&output.gold_payments);
            self.record_tlk_signal_flags(&output.signal_flags);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        self.message = text.clone();
        text
    }

    /// Submit one typed keyword line to the active conversation.
    /// Returns the rendered response text and a flag indicating
    /// whether the session has ended.
    pub fn submit_active_conversation_keyword(&mut self, line: &str) -> (String, bool) {
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
        let ctx = crate::conversation_session::ConversationContext {
            avatar_name: &avatar_name,
            branch_flags,
            moral_standing: self.moral_standing,
            dictionary: None,
            gold_payment_accepted: true,
            gold_available: Some(self.gold),
            party_member_names: &party_member_names,
        };
        let mut text = String::new();
        let mut ended = false;
        if let Some(session) = self.active_conversation.as_mut() {
            let output = session.submit_keyword(line, &ctx);
            text = output.text.clone();
            ended = output.ended;
            self.apply_tlk_action_grants(&output.action_grants);
            self.apply_tlk_gold_payments(&output.gold_payments);
            self.record_tlk_signal_flags(&output.signal_flags);
            if let Area::Town { scene, .. } = self.area {
                self.merge_talk_branch_flags(scene, output.branch_flags_set);
            }
        }
        if ended {
            if let Some(session) = self.active_conversation.as_mut() {
                session.acknowledge_close();
            }
            self.active_conversation = None;
            if let Some(cleanup) = self.run_final_conversation_cleanup() {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&cleanup);
            }
        }
        self.message = text.clone();
        (text, ended)
    }

    /// Shared town/conversation sentinel per `quest-flags.md §5` and
    /// `shops.md §6.2`. Town setup exposes either a tracked
    /// Shadowlord slot index (`0..=2`) for the active scene or the
    /// no-slot marker. Cleanup and shop surcharge readers only use
    /// the zero-versus-nonzero predicate.
    pub fn shared_town_conversation_sentinel(&self) -> u8 {
        let Area::Town { scene, .. } = self.area else {
            return CONVERSATION_SHARED_NO_SLOT_SENTINEL;
        };
        self.shadowlord_hideouts
            .iter()
            .copied()
            .enumerate()
            .find_map(|(slot, hideout)| (hideout == scene.byte).then_some(slot as u8))
            .unwrap_or(CONVERSATION_SHARED_NO_SLOT_SENTINEL)
    }

    /// Record numeric TLK `0x86` action-dispatch arguments into the
    /// generic one-conversation signal band. The public contract says
    /// these writes use a nonzero marker rather than a durable bit.
    pub fn record_tlk_signal_flags(&mut self, flags: &[u8]) {
        for flag in flags {
            let index = usize::from(*flag);
            if let Some(slot) = self.conversation_signal_flags.get_mut(index) {
                *slot = 1;
            }
        }
    }

    /// Run final conversation cleanup after Bye/empty-input close.
    /// Returns presentation text for the status line when the zero
    /// sentinel allowed cleanup; returns `None` when the sentinel
    /// suppresses the pass.
    pub fn run_final_conversation_cleanup(&mut self) -> Option<String> {
        if self.shared_town_conversation_sentinel() != 0 {
            return None;
        }

        let reseed = self.conversation_cleanup_reseed();
        if let Some(index) =
            decrement_random_resource_signal(&mut self.conversation_resource_signals, reseed)
        {
            return Some(format!(
                "Stolen-action warning. Conversation resource signal {} reconciled.",
                index + 1
            ));
        }
        if let Some(index) = decrement_signal_high_to_low(&mut self.conversation_signal_flags) {
            return Some(format!(
                "Stolen-action warning. Conversation signal {index} reconciled."
            ));
        }
        if let Some(index) = decrement_signal_high_to_low(&mut self.conversation_signal_bank_a) {
            return Some(format!(
                "Stolen-action warning. Conversation side signal A{} reconciled.",
                index + 1
            ));
        }
        if let Some(index) = decrement_signal_high_to_low(&mut self.conversation_signal_bank_b) {
            return Some(format!(
                "Stolen-action warning. Conversation side signal B{} reconciled.",
                index + 1
            ));
        }

        let debit = conversation_cleanup_gold_debit_from_seed(reseed);
        let before = self.gold;
        self.gold = self.gold.saturating_sub(debit);
        let paid = before - self.gold;
        Some(format!("Stolen-action warning. Gold -{paid}."))
    }

    pub fn conversation_cleanup_reseed(&self) -> u8 {
        (self.turn as u8).wrapping_mul(29)
            ^ self.clock.month.wrapping_mul(31)
            ^ self.clock.day.wrapping_mul(7)
            ^ self.clock.hour.wrapping_mul(11)
            ^ self.clock.minute.wrapping_mul(13)
            ^ (self.player.x as u8).wrapping_mul(17)
            ^ (self.player.y as u8).wrapping_mul(19)
            ^ (self.gold as u8).wrapping_mul(23)
    }

    pub fn apply_talk_action_grants(&mut self, actions: &[char]) {
        for action in actions {
            match *action {
                'F' => self.climbing_gear = 1,
                'H' => self.special_items[SPECIAL_ITEM_SEXTANT_INDEX] = 1,
                'I' => self.special_items[SPECIAL_ITEM_SPYGLASS_INDEX] = 1,
                'J' => self.special_items[SPECIAL_ITEM_BLACK_BADGE_INDEX] = 1,
                _ => {}
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

pub fn decrement_signal_high_to_low(signals: &mut [u8]) -> Option<usize> {
    for index in (0..signals.len()).rev() {
        if signals[index] != 0 {
            signals[index] = signals[index].saturating_sub(1);
            return Some(index);
        }
    }
    None
}

pub fn decrement_random_resource_signal(signals: &mut [u8; 3], seed: u8) -> Option<usize> {
    if signals.iter().all(|value| *value == 0) {
        return None;
    }
    for attempt in 0..signals.len() {
        let index = usize::from(seed.wrapping_add(attempt as u8) % signals.len() as u8);
        if signals[index] != 0 {
            signals[index] = signals[index].saturating_sub(1);
            return Some(index);
        }
    }
    None
}

pub fn sextant_coordinate(coordinate: usize) -> String {
    let value = (coordinate & 0xff) as u8;
    let high = b'A' + ((value >> 4) & 0x0f);
    let low = b'A' + (value & 0x0f);
    format!("{}'{}", high as char, low as char)
}

pub fn scroll_label(index: usize) -> &'static str {
    SCROLL_SPELL_LABELS.get(index).copied().unwrap_or("Unknown")
}

fn pending_action_for_use_request(request: UseItemRequest) -> Option<UsePendingAction> {
    match request {
        UseItemRequest::Potion {
            index,
            target: None,
        } => Some(UsePendingAction::PotionTarget { index }),
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
            index: SCROLL_RESURRECTION_INDEX,
        }),
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
) {
    if player_marker {
        for offset in 0..scale {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, offset, scale / 2, 15);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, offset, 15);
        }
        return;
    }

    let color = surface_view_class_color(class);
    match class {
        0x00 | 0x0C => {}
        0x01 => {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 0, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale - 1, 0, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, scale - 1, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale - 1, scale - 1, color);
        }
        0x02 | 0x03 | 0x0D | 0x0F => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
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
        0x07 | 0x0B => draw_view_overlay_diagonals(viewport, cell_x, cell_y, scale, color),
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
            draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 1.min(scale - 1), color);
            for y in scale / 2..scale {
                for x in (y + cell_x) % 2..scale {
                    if x % 2 == 0 {
                        set_view_overlay_pixel(viewport, cell_x, cell_y, scale, x, y, color);
                    }
                }
            }
        }
        0x0A => {
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, 0, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale - 1, 0, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, 0, scale - 1, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale - 1, scale - 1, color);
            set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, color);
        }
        0x0E => {
            for y in 0..scale {
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, y, color);
                set_view_overlay_pixel(viewport, cell_x, cell_y, scale, (scale / 2) + 1, y, color);
            }
        }
        0x10 => draw_view_overlay_fence(viewport, cell_x, cell_y, scale, color, tile),
        _ => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, color),
    }
}

fn draw_dungeon_view_cell(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    glyph: char,
) {
    match glyph {
        '@' => draw_surface_view_cell(viewport, cell_x, cell_y, scale, 0, 0, true),
        '#' => draw_view_overlay_box(viewport, cell_x, cell_y, scale, 8),
        '>' | '<' => draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale / 2, 14),
        '$' => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, 6),
        '+' => draw_view_overlay_diagonals(viewport, cell_x, cell_y, scale, 11),
        '~' => draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 1.min(scale - 1), 9),
        '*' => fill_view_overlay_cell(viewport, cell_x, cell_y, scale, 12),
        ' ' => {}
        _ => set_view_overlay_pixel(viewport, cell_x, cell_y, scale, scale / 2, scale / 2, 7),
    }
}

fn surface_view_class_color(class: u8) -> u8 {
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
        0x0D => 12,
        0x0E => 9,
        0x0F => 4,
        0x10 => 1,
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

fn draw_view_overlay_fence(
    viewport: &mut TileViewport,
    cell_x: usize,
    cell_y: usize,
    scale: usize,
    color: u8,
    tile: u8,
) {
    let mask = tile & 0x0f;
    if mask == 0 || mask & 0x1 != 0 {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, 0, color);
    }
    if mask == 0 || mask & 0x2 != 0 {
        draw_view_overlay_vline(viewport, cell_x, cell_y, scale, scale - 1, color);
    }
    if mask == 0 || mask & 0x4 != 0 {
        draw_view_overlay_hline(viewport, cell_x, cell_y, scale, scale - 1, color);
    }
    if mask == 0 || mask & 0x8 != 0 {
        draw_view_overlay_vline(viewport, cell_x, cell_y, scale, 0, color);
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

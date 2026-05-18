use std::io;
use std::path::Path;

use crate::play_state_impl::chunk_04::sextant_coordinate;
use crate::*;

impl PlayState {
    pub fn z_stats(&mut self) -> MoveOutcome {
        let selected = self.z_stats_initial_party_index();
        self.z_stats_for_party(selected)
    }

    pub fn z_stats_for_party(&mut self, selected: usize) -> MoveOutcome {
        if self.party.is_empty() {
            self.message = "No party members are available.".to_string();
            return MoveOutcome::Blocked;
        }
        let selected = selected.min(self.party.len() - 1);
        self.active_z_stats = Some(ZStatsSession::new(selected));
        self.message = self.render_active_z_stats();
        MoveOutcome::Observed
    }

    pub fn z_stats_initial_party_index(&self) -> usize {
        if self.combat_active {
            if let Some(slot) = self.pending_combat_actor_slot {
                if slot < self.party.len()
                    && slot < COMBAT_PARTY_ACTOR_SLOTS
                    && self
                        .party
                        .get(slot)
                        .copied()
                        .is_some_and(PartyMember::living)
                {
                    return slot;
                }
            }
        }
        self.active_player
            .filter(|slot| *slot < self.party.len())
            .unwrap_or(0)
    }

    pub fn render_active_z_stats(&self) -> String {
        self.active_z_stats
            .as_ref()
            .map(|session| self.render_z_stats_session(session))
            .unwrap_or_else(|| self.z_stats_message())
    }

    pub fn start_cast_spell_prompt(&mut self) -> MoveOutcome {
        if self.party.is_empty() {
            self.message = "No party members are available.".to_string();
            return MoveOutcome::Blocked;
        }
        let caster_index = self
            .active_player
            .filter(|slot| *slot < self.party.len())
            .unwrap_or(0);
        self.active_cast = Some(CastSession::new(caster_index));
        self.message = self.render_active_cast();
        MoveOutcome::Observed
    }

    pub fn start_combat_cast_spell_prompt(
        &mut self,
        actor_slot: usize,
        combat_had_foe: bool,
    ) -> MoveOutcome {
        if !self.combat_active
            || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS
            || actor_slot >= self.party.len()
            || !self
                .combat_actors
                .get(actor_slot)
                .copied()
                .is_some_and(combat_actor_is_active_not_dead)
        {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        }
        self.active_cast = Some(CastSession::for_combat_actor(actor_slot, combat_had_foe));
        self.message = self.render_active_cast();
        MoveOutcome::Observed
    }

    pub fn render_active_cast(&self) -> String {
        self.active_cast
            .as_ref()
            .map(|session| self.render_cast_session(session))
            .unwrap_or_else(cast_prompt_message)
    }

    pub fn render_cast_session(&self, session: &CastSession) -> String {
        let prompt = if session.buffer.is_empty() {
            "Spell name: _".to_string()
        } else {
            format!("Spell name: {}", session.buffer)
        };
        format!(
            "Cast: party member {}. {prompt}\nType selector letters; Enter/Space casts; Backspace erases; Esc cancels.",
            session.caster_index + 1
        )
    }

    pub fn step_active_cast(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<Option<(MoveOutcome, Option<(usize, bool)>)>> {
        let Some(mut session) = self.active_cast.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            match cast_input_action(ch) {
                CastInputAction::Cancel => {
                    self.message = "None!".to_string();
                    return Ok(None);
                }
                CastInputAction::Complete => {
                    if session.buffer.is_empty() {
                        self.message = "None!".to_string();
                        return Ok(None);
                    }
                    let suffix = format!("{}{}", session.caster_index + 1, session.buffer);
                    let combat = session
                        .combat_actor_slot
                        .map(|slot| (slot, session.combat_had_foe));
                    let outcome = self.cast_spell_from_suffix(&suffix, game_dir)?;
                    return Ok(Some((outcome, combat)));
                }
                CastInputAction::Backspace => {
                    session.buffer.pop();
                }
                CastInputAction::Append(ch) => {
                    if session.buffer.len() < SPELL_SELECTOR_MAX_LEN {
                        session.buffer.push(ch);
                    }
                    if session.buffer.len() >= SPELL_SELECTOR_MAX_LEN {
                        let suffix = format!("{}{}", session.caster_index + 1, session.buffer);
                        let combat = session
                            .combat_actor_slot
                            .map(|slot| (slot, session.combat_had_foe));
                        let outcome = self.cast_spell_from_suffix(&suffix, game_dir)?;
                        return Ok(Some((outcome, combat)));
                    }
                }
                CastInputAction::Discard => {}
            }
        }
        self.message = self.render_cast_session(&session);
        self.active_cast = Some(session);
        Ok(None)
    }

    pub fn start_mix_reagents_prompt(&mut self) -> MoveOutcome {
        if self.reagents.iter().all(|count| *count == 0) {
            self.message = MMIX_NO_REAGENTS_OWNED_MESSAGE.to_string();
            return MoveOutcome::Blocked;
        }
        self.active_mix = Some(MixSession::new());
        self.message = self.render_active_mix();
        MoveOutcome::Observed
    }

    pub fn render_active_mix(&self) -> String {
        self.active_mix
            .as_ref()
            .map(|session| self.render_mix_session(session))
            .unwrap_or_else(mix_prompt_message)
    }

    pub fn render_mix_session(&self, session: &MixSession) -> String {
        match session.phase {
            MixPhase::Spell => {
                let spell = if session.spell_buffer.is_empty() {
                    "_".to_string()
                } else {
                    session.spell_buffer.clone()
                };
                format!(
                    "{MMIX_SPELL_PROMPT_MESSAGE} {spell}\nType selector letters; Enter accepts; Esc cancels."
                )
            }
            MixPhase::Reagents => {
                let mut lines =
                    vec!["Mix reagents: Enter/Space toggles, M accepts, Esc cancels.".to_string()];
                let visible = self.mix_visible_reagents();
                for (row, index) in visible.iter().copied().enumerate() {
                    let reagent = REAGENT_VENDOR_ORDER[index];
                    let marker = if row == session.reagent_cursor {
                        ">"
                    } else {
                        " "
                    };
                    let selected = if session.reagent_mask & REAGENT_MASKS[index] != 0 {
                        "*"
                    } else {
                        " "
                    };
                    lines.push(format!(
                        "{marker}{selected} {}. {} ({})",
                        row + 1,
                        reagent.abbreviation(),
                        self.reagents[index]
                    ));
                }
                if visible.is_empty() {
                    lines.push(MMIX_NO_REAGENTS_OWNED_MESSAGE.to_string());
                }
                lines.join("\n")
            }
            MixPhase::Quantity => {
                let quantity = if session.quantity_buffer.is_empty() {
                    "_".to_string()
                } else {
                    session.quantity_buffer.clone()
                };
                format!(
                    "{MMIX_QUANTITY_PROMPT_MESSAGE} {quantity}\nEnter accepts; Backspace erases; Esc cancels."
                )
            }
        }
    }

    pub fn step_active_mix(&mut self, key: char, suffix: &str) -> Option<MoveOutcome> {
        let Some(mut session) = self.active_mix.take() else {
            return None;
        };
        let had_suffix = !suffix.is_empty();
        for ch in std::iter::once(key).chain(suffix.chars()) {
            if let Some(outcome) = self.step_mix_session_char(&mut session, ch) {
                return Some(outcome);
            }
        }
        if had_suffix {
            if session.phase == MixPhase::Spell && !session.spell_buffer.is_empty() {
                self.accept_mix_spell(&mut session);
            } else if session.phase == MixPhase::Quantity && !session.quantity_buffer.is_empty() {
                if let Some(outcome) = self.complete_mix_session(&mut session) {
                    return Some(outcome);
                }
            }
        }
        self.message = self.render_mix_session(&session);
        self.active_mix = Some(session);
        None
    }

    fn step_mix_session_char(&mut self, session: &mut MixSession, ch: char) -> Option<MoveOutcome> {
        match session.phase {
            MixPhase::Spell => match cast_input_action(ch) {
                CastInputAction::Cancel => {
                    self.message = "None!".to_string();
                    Some(MoveOutcome::PromptDeclined)
                }
                CastInputAction::Complete => {
                    if session.spell_buffer.is_empty() {
                        self.message = "None!".to_string();
                        Some(MoveOutcome::PromptDeclined)
                    } else {
                        self.accept_mix_spell(session);
                        None
                    }
                }
                CastInputAction::Backspace => {
                    session.spell_buffer.pop();
                    None
                }
                CastInputAction::Append(ch) => {
                    if session.spell_buffer.len() < SPELL_SELECTOR_MAX_LEN {
                        session.spell_buffer.push(ch);
                    }
                    if session.spell_buffer.len() >= SPELL_SELECTOR_MAX_LEN {
                        self.accept_mix_spell(session);
                    }
                    None
                }
                CastInputAction::Discard => None,
            },
            MixPhase::Reagents => match ch {
                '\u{1b}' => {
                    self.message = "None!".to_string();
                    Some(MoveOutcome::PromptDeclined)
                }
                '\r' | '\n' | ' ' => {
                    self.toggle_mix_reagent(session);
                    None
                }
                'M' | 'm' => {
                    session.phase = MixPhase::Quantity;
                    session.quantity_buffer.clear();
                    None
                }
                '>' | '+' | '2' | 'j' | 'J' => {
                    self.move_mix_reagent_cursor(session, 1);
                    None
                }
                '<' | '-' | '8' | 'k' | 'K' => {
                    self.move_mix_reagent_cursor(session, -1);
                    None
                }
                '1'..='8' => {
                    let row = (ch as u8 - b'1') as usize;
                    self.toggle_mix_reagent_row(session, row);
                    None
                }
                _ => None,
            },
            MixPhase::Quantity => match ch {
                '\u{1b}' => {
                    self.message = "None!".to_string();
                    Some(MoveOutcome::PromptDeclined)
                }
                '\r' | '\n' | ' ' => self.complete_mix_session(session),
                '\u{8}' | '\u{7f}' => {
                    session.quantity_buffer.pop();
                    None
                }
                ch if ch.is_ascii_digit()
                    && session.quantity_buffer.len() < MMIX_QUANTITY_PROMPT_DIGITS =>
                {
                    session.quantity_buffer.push(ch);
                    if session.quantity_buffer.len() >= MMIX_QUANTITY_PROMPT_DIGITS {
                        return self.complete_mix_session(session);
                    }
                    None
                }
                _ => None,
            },
        }
    }

    fn accept_mix_spell(&mut self, session: &mut MixSession) {
        let code = inline_spell_code(&session.spell_buffer);
        session.spell_buffer = code.clone();
        session.spell_index = spell_index_from_code(&code);
        session.phase = MixPhase::Reagents;
        session.reagent_cursor = 0;
    }

    fn complete_mix_session(&mut self, session: &mut MixSession) -> Option<MoveOutcome> {
        let amount = session.quantity_buffer.parse::<u8>().unwrap_or(0);
        if amount > 0 {
            for index in selected_reagent_indices(session.reagent_mask) {
                if self.reagents[index] < amount {
                    session.quantity_buffer.clear();
                    self.message = format!(
                        "{}\n{}",
                        MMIX_INSUFFICIENT_REAGENTS_MESSAGE,
                        self.render_mix_session(session)
                    );
                    self.active_mix = Some(session.clone());
                    return Some(MoveOutcome::Blocked);
                }
            }
        }
        let suffix = format!(
            "{}/{}/{}",
            session.spell_buffer, session.reagent_mask, amount
        );
        Some(self.mix_reagents_from_suffix(&suffix))
    }

    fn mix_visible_reagents(&self) -> Vec<usize> {
        (0..REAGENT_COUNT)
            .filter(|index| self.reagents[*index] > 0)
            .collect()
    }

    fn toggle_mix_reagent(&self, session: &mut MixSession) {
        let visible = self.mix_visible_reagents();
        if let Some(index) = visible.get(session.reagent_cursor).copied() {
            session.reagent_mask ^= REAGENT_MASKS[index];
        }
    }

    fn toggle_mix_reagent_row(&self, session: &mut MixSession, row: usize) {
        let visible = self.mix_visible_reagents();
        if let Some(index) = visible.get(row).copied() {
            session.reagent_cursor = row;
            session.reagent_mask ^= REAGENT_MASKS[index];
        }
    }

    fn move_mix_reagent_cursor(&self, session: &mut MixSession, delta: isize) {
        let visible = self.mix_visible_reagents();
        if visible.is_empty() {
            session.reagent_cursor = 0;
            return;
        }
        let len = visible.len() as isize;
        let current = session.reagent_cursor.min(visible.len() - 1) as isize;
        session.reagent_cursor = (current + delta).rem_euclid(len) as usize;
    }

    pub fn start_new_order_prompt(&mut self) -> MoveOutcome {
        self.active_new_order = Some(NewOrderSession::new());
        self.message = self.render_active_new_order();
        MoveOutcome::Observed
    }

    pub fn render_active_new_order(&self) -> String {
        self.active_new_order
            .as_ref()
            .map(|session| self.render_new_order_session(session))
            .unwrap_or_else(new_order_prompt_message)
    }

    pub fn render_new_order_session(&self, session: &NewOrderSession) -> String {
        match session.first {
            Some(first) => format!(
                "New order: first party member {}. Choose second member (1-{}) or Space/Esc to exit.",
                first + 1,
                self.party.len().min(6)
            ),
            None => format!(
                "New order: choose first member (1-{}) or Space/Esc to exit.",
                self.party.len().min(6)
            ),
        }
    }

    pub fn step_active_new_order(&mut self, key: char, suffix: &str) -> Option<MoveOutcome> {
        let Some(mut session) = self.active_new_order.take() else {
            return None;
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            match ch {
                '\u{1b}' | ' ' => {
                    self.message = "None!".to_string();
                    return Some(MoveOutcome::PromptDeclined);
                }
                '1'..='6' => {
                    let selected = (ch as u8 - b'1') as usize;
                    if let Some(first) = session.first {
                        let suffix = format!("{}{}", first + 1, selected + 1);
                        return Some(self.new_order_from_suffix(&suffix));
                    }
                    session.first = Some(selected);
                }
                _ => {}
            }
        }
        self.message = self.render_new_order_session(&session);
        self.active_new_order = Some(session);
        None
    }

    pub fn start_yell_prompt(&mut self) -> MoveOutcome {
        if matches!(self.player.transport, TransportState::Ship { .. }) {
            return self.yell_command(None);
        }
        self.active_yell = Some(YellSession::new());
        self.message = self.render_active_yell();
        MoveOutcome::Observed
    }

    pub fn render_active_yell(&self) -> String {
        self.active_yell
            .as_ref()
            .map(|session| self.render_yell_session(session))
            .unwrap_or_else(yell_prompt_message)
    }

    pub fn render_yell_session(&self, session: &YellSession) -> String {
        let word = if session.buffer.is_empty() {
            "_".to_string()
        } else {
            session.buffer.clone()
        };
        format!("Yell what? {word}\nType a word and press Enter; Esc cancels.")
    }

    pub fn step_active_yell(&mut self, key: char, suffix: &str) -> Option<MoveOutcome> {
        let Some(mut session) = self.active_yell.take() else {
            return None;
        };
        if key == '\u{1b}' {
            self.message = "None!".to_string();
            return Some(MoveOutcome::PromptDeclined);
        }
        let mut line = String::new();
        if !matches!(key, '\r' | '\n') {
            line.push(key);
        }
        line.push_str(suffix);
        if line.is_empty() && matches!(key, '\r' | '\n') {
            return Some(self.yell_command(Some("")));
        }
        if !line.is_empty() {
            session.buffer.push_str(&line);
            return Some(self.yell_command(Some(&session.buffer)));
        }
        self.message = self.render_yell_session(&session);
        self.active_yell = Some(session);
        None
    }

    pub fn render_stats_panel_view(&self) -> String {
        render_stats_panel(self, self.active_player)
    }

    pub fn render_stats_panel_frame(&mut self) -> String {
        let active_cursor = self.active_player;
        let panel = render_stats_panel(self, active_cursor);
        if stats_panel_active_cursor_visible(self, active_cursor) {
            self.active_player = None;
        }
        panel
    }

    pub fn render_z_stats_session(&self, session: &ZStatsSession) -> String {
        let mut lines = vec![format!(
            "Z-stats: {} page, party member {} of {}.",
            session.page.title(),
            session.selected_party_index + 1,
            self.party.len()
        )];
        lines.push(self.z_stats_navigation_hint());
        match session.page {
            ZStatsPage::Stats => self.render_z_stats_character_page(session, &mut lines),
            ZStatsPage::Equipment => self.render_z_stats_equipment_page(session, &mut lines),
            ZStatsPage::SpellBook => self.render_z_stats_spell_book_page(session, &mut lines),
            ZStatsPage::Reagents => self.render_z_stats_reagent_page(&mut lines),
            ZStatsPage::Spells => self.render_z_stats_spell_page(&mut lines),
            ZStatsPage::SpecialUse => self.render_z_stats_special_use_page(&mut lines),
            ZStatsPage::EquipmentStock => self.render_z_stats_equipment_stock_page(&mut lines),
        }
        lines.join("\n")
    }

    pub fn step_active_z_stats(&mut self, key: char, suffix: &str) -> bool {
        let Some(mut session) = self.active_z_stats.take() else {
            return false;
        };
        let key = z_stats_first_input_key(key, suffix);
        match z_stats_input_action(key) {
            ZStatsInputAction::Exit => {
                self.message = "Z-stats closed.".to_string();
            }
            ZStatsInputAction::NextPage => {
                session.move_next_page();
                self.message = self.render_z_stats_session(&session);
                self.active_z_stats = Some(session);
            }
            ZStatsInputAction::PreviousPage => {
                session.move_previous_page();
                self.message = self.render_z_stats_session(&session);
                self.active_z_stats = Some(session);
            }
            ZStatsInputAction::SelectParty(index) => {
                if index < self.party.len() {
                    session.select_party_index(index);
                    self.message = self.render_z_stats_session(&session);
                } else {
                    self.message = format!(
                        "Party has {} member{}.\n{}",
                        self.party.len(),
                        if self.party.len() == 1 { "" } else { "s" },
                        self.render_z_stats_session(&session)
                    );
                }
                self.active_z_stats = Some(session);
            }
            ZStatsInputAction::Redraw | ZStatsInputAction::Discard => {
                self.message = self.render_z_stats_session(&session);
                self.active_z_stats = Some(session);
            }
        }
        true
    }

    pub fn start_ready_equipment(&mut self) -> MoveOutcome {
        if self.party.is_empty() {
            self.message = "No party members are available.".to_string();
            return MoveOutcome::Blocked;
        }
        self.active_ready = Some(ReadySession::new());
        self.message = self.render_active_ready();
        MoveOutcome::Observed
    }

    pub fn start_ready_equipment_for_party(&mut self, party_index: usize) -> MoveOutcome {
        if party_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if !self.party[party_index].living() {
            self.message = format!("Party member {} is unavailable.", party_index + 1);
            return MoveOutcome::Blocked;
        }

        let mut session = ReadySession::with_party(party_index);
        self.normalize_ready_cursor(&mut session);
        self.active_ready = Some(session);
        self.message = self.render_active_ready();
        MoveOutcome::Observed
    }

    pub fn start_combat_ready_equipment(&mut self, actor_slot: usize) -> MoveOutcome {
        if !self.combat_active
            || actor_slot >= COMBAT_PARTY_ACTOR_SLOTS
            || actor_slot >= self.party.len()
            || !self
                .combat_actors
                .get(actor_slot)
                .copied()
                .is_some_and(combat_actor_is_active_not_dead)
        {
            self.message = "No active combatant.".to_string();
            return MoveOutcome::Blocked;
        }
        self.start_ready_equipment_for_party(actor_slot)
    }

    pub fn render_active_ready(&self) -> String {
        self.active_ready
            .as_ref()
            .map(|session| self.render_ready_session(session))
            .unwrap_or_else(ready_prompt_message)
    }

    pub fn render_ready_session(&self, session: &ReadySession) -> String {
        let Some(party_index) = session.selected_party_index else {
            return format!(
                "Ready: choose party member (1-{}) or Space/Esc to exit.",
                self.party.len().min(6)
            );
        };
        if party_index >= self.party.len() {
            return format!(
                "Ready: party has {} member{}. Choose 1-{} or Space/Esc to exit.",
                self.party.len(),
                if self.party.len() == 1 { "" } else { "s" },
                self.party.len().min(6)
            );
        }

        let mut lines = vec![format!(
            "Ready: party member {}. Enter equips/unequips; </> move; [] page; 1-6 party; Space/Esc exits.",
            party_index + 1
        )];
        let visible = self.ready_visible_items_for_party(party_index);
        if visible.is_empty() {
            lines.push("Nothing to ready.".to_string());
            return lines.join("\n");
        }

        let cursor = visible
            .iter()
            .copied()
            .find(|item| *item >= session.cursor)
            .or_else(|| visible.first().copied())
            .unwrap_or(0);
        let cursor_pos = visible.iter().position(|item| *item == cursor).unwrap_or(0);
        let panel_start = (cursor_pos / READY_PICKER_PANEL_ROWS) * READY_PICKER_PANEL_ROWS;
        for item_id in visible
            .iter()
            .copied()
            .skip(panel_start)
            .take(READY_PICKER_PANEL_ROWS)
        {
            let marker = if item_id == cursor { ">" } else { " " };
            let stock = self.equipment_stock[item_id];
            let readied = self
                .party_equipment
                .get(party_index)
                .is_some_and(|equipment| character_has_readied(equipment, item_id as u8));
            let state = match (stock, readied) {
                (0, true) => "readied".to_string(),
                (count, true) => format!("stock {count}, readied"),
                (count, false) => format!("stock {count}"),
            };
            lines.push(format!(
                "{marker} {item_id:02}: {} ({state})",
                equipment_name(item_id)
            ));
        }
        if visible.len() > panel_start + READY_PICKER_PANEL_ROWS {
            lines.push(format!(
                "... {} more",
                visible.len() - panel_start - READY_PICKER_PANEL_ROWS
            ));
        }
        lines.join("\n")
    }

    pub fn step_active_ready(&mut self, key: char, suffix: &str) -> bool {
        let Some(mut session) = self.active_ready.take() else {
            return false;
        };
        let key = ready_first_input_key(key, suffix);
        let action = ready_input_action(key);
        if matches!(action, ReadyInputAction::Exit) {
            self.message = "Ready closed.".to_string();
            return true;
        }

        if session.selected_party_index.is_none() {
            match action {
                ReadyInputAction::SelectParty(index) => {
                    if self.ready_select_party_for_session(&mut session, index) {
                        self.message = self.render_ready_session(&session);
                    }
                    self.active_ready = Some(session);
                }
                ReadyInputAction::Redraw | ReadyInputAction::Discard => {
                    self.message = self.render_ready_session(&session);
                    self.active_ready = Some(session);
                }
                ReadyInputAction::Confirm
                | ReadyInputAction::NextItem
                | ReadyInputAction::PreviousItem
                | ReadyInputAction::PageNext
                | ReadyInputAction::PagePrevious
                | ReadyInputAction::Exit => {
                    self.message = self.render_ready_session(&session);
                    self.active_ready = Some(session);
                }
            }
            return true;
        }

        match action {
            ReadyInputAction::SelectParty(index) => {
                if self.ready_select_party_for_session(&mut session, index) {
                    self.message = self.render_ready_session(&session);
                }
                self.active_ready = Some(session);
            }
            ReadyInputAction::NextItem => {
                self.move_ready_cursor(&mut session, 1);
                self.message = self.render_ready_session(&session);
                self.active_ready = Some(session);
            }
            ReadyInputAction::PreviousItem => {
                self.move_ready_cursor(&mut session, -1);
                self.message = self.render_ready_session(&session);
                self.active_ready = Some(session);
            }
            ReadyInputAction::PageNext => {
                self.move_ready_cursor(&mut session, READY_PICKER_PANEL_ROWS as isize);
                self.message = self.render_ready_session(&session);
                self.active_ready = Some(session);
            }
            ReadyInputAction::PagePrevious => {
                self.move_ready_cursor(&mut session, -(READY_PICKER_PANEL_ROWS as isize));
                self.message = self.render_ready_session(&session);
                self.active_ready = Some(session);
            }
            ReadyInputAction::Confirm => {
                let Some(party_index) = session.selected_party_index else {
                    self.message = self.render_ready_session(&session);
                    self.active_ready = Some(session);
                    return true;
                };
                let Some(item_id) = self.ready_selected_item(&session) else {
                    self.message = "Nothing to ready.".to_string();
                    self.active_ready = Some(session);
                    return true;
                };
                let _ = self.ready_equipment(InlineReadyRequest {
                    party_index,
                    item_id,
                });
                let outcome_message = self.message.clone();
                self.normalize_ready_cursor(&mut session);
                self.message =
                    format!("{outcome_message}\n{}", self.render_ready_session(&session));
                self.active_ready = Some(session);
            }
            ReadyInputAction::Redraw | ReadyInputAction::Discard => {
                self.message = self.render_ready_session(&session);
                self.active_ready = Some(session);
            }
            ReadyInputAction::Exit => unreachable!(),
        }
        true
    }

    fn ready_select_party_for_session(&mut self, session: &mut ReadySession, index: usize) -> bool {
        if index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return false;
        }
        if !self.party[index].living() {
            self.message = format!("Party member {} is unavailable.", index + 1);
            return false;
        }
        session.select_party_index(index);
        self.normalize_ready_cursor(session);
        true
    }

    fn ready_visible_items_for_party(&self, party_index: usize) -> Vec<usize> {
        let equipment = self.party_equipment.get(party_index);
        (0..EQUIPMENT_COUNT)
            .filter(|item_id| {
                self.equipment_stock[*item_id] > 0
                    || equipment.is_some_and(|block| character_has_readied(block, *item_id as u8))
            })
            .collect()
    }

    fn ready_selected_item(&self, session: &ReadySession) -> Option<usize> {
        let party_index = session.selected_party_index?;
        self.ready_visible_items_for_party(party_index)
            .into_iter()
            .find(|item| *item >= session.cursor)
            .or_else(|| {
                self.ready_visible_items_for_party(party_index)
                    .into_iter()
                    .next()
            })
    }

    fn normalize_ready_cursor(&self, session: &mut ReadySession) {
        let Some(item_id) = self.ready_selected_item(session) else {
            session.cursor = 0;
            return;
        };
        session.cursor = item_id;
    }

    fn move_ready_cursor(&self, session: &mut ReadySession, delta: isize) {
        let Some(party_index) = session.selected_party_index else {
            return;
        };
        let visible = self.ready_visible_items_for_party(party_index);
        if visible.is_empty() {
            session.cursor = 0;
            return;
        }
        let current = visible
            .iter()
            .position(|item| *item == session.cursor)
            .unwrap_or(0);
        let len = visible.len() as isize;
        let next = (current as isize + delta).rem_euclid(len) as usize;
        session.cursor = visible[next];
    }

    fn z_stats_navigation_hint(&self) -> String {
        "Use </> for pages, 1-6 for party, Space/Esc to exit.".to_string()
    }

    fn render_z_stats_character_page(&self, session: &ZStatsSession, lines: &mut Vec<String>) {
        let Some(member) = self.party.get(session.selected_party_index).copied() else {
            lines.push("No party member selected.".to_string());
            return;
        };
        let name = self.party_member_display_name(session.selected_party_index);
        let class = character_class_for_byte(member.class_byte)
            .map(CharacterClass::display_name)
            .unwrap_or("Unknown");
        let status = party_status_name(member.status);
        let strength = self
            .party_strengths
            .get(session.selected_party_index)
            .copied()
            .unwrap_or(self.avatar_stats.strength);
        let dexterity = member.climb_stat;
        let intellect = self
            .party_intelligence
            .get(session.selected_party_index)
            .copied()
            .unwrap_or(self.avatar_stats.intelligence);
        let experience = self
            .party_experience
            .get(session.selected_party_index)
            .copied()
            .unwrap_or(0);
        lines.push(format!("Name: {name}"));
        lines.push(format!("Class: {class}"));
        lines.push(format!("Status: {status}"));
        lines.push(format!("Level: {}", member.level));
        lines.push(format!(
            "STR {strength:>2} DEX {dexterity:>2} INT {intellect:>2}"
        ));
        lines.push(format!(
            "HP {}/{} MP {} XP {}",
            member.hp, member.max_hp, member.mana, experience
        ));
    }

    fn render_z_stats_equipment_page(&self, session: &ZStatsSession, lines: &mut Vec<String>) {
        let Some(equipment) = self.party_equipment.get(session.selected_party_index) else {
            lines.push("Nothing equipped.".to_string());
            return;
        };
        let mut count = 0;
        for (slot, item) in equipment.iter().copied().enumerate() {
            if item == EQUIPMENT_EMPTY {
                continue;
            }
            count += 1;
            lines.push(format!(
                "{}: {}",
                slot_name(slot),
                equipment_name(item as usize)
            ));
        }
        if count == 0 {
            lines.push("Nothing equipped.".to_string());
        }
    }

    fn render_z_stats_spell_book_page(&self, session: &ZStatsSession, lines: &mut Vec<String>) {
        let Some(member) = self.party.get(session.selected_party_index).copied() else {
            lines.push("No party member selected.".to_string());
            return;
        };
        let max_circle = z_stats_spell_book_max_circle(member.class_byte);
        if max_circle == 0 {
            lines.push("No spell access.".to_string());
            return;
        }

        let visible_circle = max_circle.min(member.level).min(8);
        if visible_circle == 0 {
            lines.push("No spell access at current level.".to_string());
            return;
        }

        let rows = (0..SPELL_COUNT)
            .filter(|index| {
                spell_circle_for(*index as u8).is_some_and(|circle| circle <= visible_circle)
            })
            .map(|index| {
                let circle = spell_circle_for(index as u8).unwrap_or(0);
                let code = SPELL_CODES[index];
                let rune = spell_rune_name(index).unwrap_or("Unknown");
                let name = spell_common_name(index).unwrap_or("Unknown Spell");
                let recipe = spell_recipe_label(SPELL_RECIPE_MASKS[index]);
                format!("C{circle} {code:<4} {rune} / {name} / {recipe}")
            })
            .collect::<Vec<_>>();
        append_inventory_rows(lines, rows);
    }

    fn render_z_stats_reagent_page(&self, lines: &mut Vec<String>) {
        const REAGENTS: [Reagent; REAGENT_COUNT] = [
            Reagent::SulfurAsh,
            Reagent::Ginseng,
            Reagent::Garlic,
            Reagent::SpiderSilk,
            Reagent::BloodMoss,
            Reagent::BlackPearl,
            Reagent::Nightshade,
            Reagent::Mandrake,
        ];
        let rows = REAGENTS
            .iter()
            .filter_map(|reagent| {
                let count = self.reagents[reagent.inventory_index()];
                (count > 0).then(|| format!("{}: {count}", reagent.display_name()))
            })
            .collect::<Vec<_>>();
        append_inventory_rows(lines, rows);
    }

    fn render_z_stats_spell_page(&self, lines: &mut Vec<String>) {
        let rows = self
            .spell_charges
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, count)| {
                (count > 0).then(|| {
                    let name = spell_common_name(index).unwrap_or("Unknown Spell");
                    format!("{} {}: {count}", SPELL_CODES[index], name)
                })
            })
            .collect::<Vec<_>>();
        append_inventory_rows(lines, rows);
    }

    fn render_z_stats_special_use_page(&self, lines: &mut Vec<String>) {
        let mut rows = Vec::new();
        if self.keys > 0 {
            rows.push(format!("Keys: {}", self.keys));
        }
        if self.gems > 0 {
            rows.push(format!("Gems: {}", self.gems));
        }
        if self.torches > 0 {
            rows.push(format!("Torches: {}", self.torches));
        }
        if self.climbing_gear > 0 {
            rows.push(format!("Grapple: {}", self.climbing_gear));
        }
        for (index, count) in self.special_items.iter().copied().enumerate() {
            if count > 0 {
                rows.push(format!("{}: {count}", special_item_name(index)));
            }
        }
        for (index, count) in self.scroll_stock.iter().copied().enumerate() {
            if count > 0 {
                let label = SCROLL_SPELL_LABELS.get(index).copied().unwrap_or("Unknown");
                rows.push(format!("Scroll {label}: {count}"));
            }
        }
        for (index, count) in self.potion_stock.iter().copied().enumerate() {
            if count > 0 {
                rows.push(format!("{}: {count}", potion_inventory_name(index)));
            }
        }
        append_inventory_rows(lines, rows);
    }

    fn render_z_stats_equipment_stock_page(&self, lines: &mut Vec<String>) {
        let rows = self
            .equipment_stock
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, count)| {
                (count > 0).then(|| format!("{}: {count}", equipment_name(index)))
            })
            .collect::<Vec<_>>();
        append_inventory_rows(lines, rows);
    }

    fn party_member_display_name(&self, index: usize) -> String {
        self.party_names
            .get(index)
            .and_then(|name| party_name_to_string(name))
            .unwrap_or_else(|| format!("Party member {}", index + 1))
    }

    pub fn z_stats_message(&self) -> String {
        let area = self.area_status_label();
        let reagents_total: u16 = self.reagents.iter().map(|count| *count as u16).sum();
        let spells = self.spell_stock_summary();
        let party = self.party_status_summary();
        let equipment = equipment_stock_summary(&self.equipment_stock);
        let effect = self.active_effect_status();
        format!(
            "Z-stats: {area} at ({}, {}), facing {}, date Y{} M{} D{} {:02}:{:02}, turn {}; transport {}; wind {}; typeahead {}; timing {}; light torch={} spell={} ambient={} time-stop={} effect={}; inventory food={} gold={} keys={} gems={} torches={} climbing={} reagents={}; equipment {}; spells {}; party {}.",
            self.player.x,
            self.player.y,
            self.player.facing.name(),
            self.clock.year,
            self.clock.month,
            self.clock.day,
            self.clock.hour,
            self.clock.minute,
            self.turn,
            self.player.transport.status_label(),
            self.wind.status_message(),
            self.typeahead_status_label(),
            self.timing_status.status_label(),
            self.torch_counter,
            self.light_spell_counter,
            self.ambient_light,
            self.time_stop_counter,
            effect,
            self.food,
            self.gold,
            self.keys,
            self.gems,
            self.torches,
            self.climbing_gear,
            reagents_total,
            equipment,
            spells,
            party
        )
    }

    pub fn active_effect_status(&self) -> String {
        match (self.active_effect_tag, self.active_effect_counter) {
            (Some(tag), counter) if counter > 0 => {
                format!("{}/{}", char::from(tag), counter)
            }
            _ => "none".to_string(),
        }
    }

    pub fn toggle_typeahead_buffer(&mut self) {
        self.typeahead_buffer_enabled = !self.typeahead_buffer_enabled;
        self.message = if self.typeahead_buffer_enabled {
            "Buffer On."
        } else {
            "Buffer Off."
        }
        .to_string();
    }

    pub fn exit_to_dos_prompt(&mut self, confirm: Option<bool>) -> PlayInputDisposition {
        match confirm {
            None => {
                self.message = "Exit to DOS? Use QY to exit or QN to cancel.".to_string();
                PlayInputDisposition::Continue
            }
            Some(false) => {
                self.message = "No.".to_string();
                PlayInputDisposition::Continue
            }
            Some(true) => {
                self.message = "Yes. Exiting to DOS.".to_string();
                PlayInputDisposition::Quit
            }
        }
    }

    pub fn typeahead_status_label(&self) -> &'static str {
        if self.typeahead_buffer_enabled {
            "on"
        } else {
            "off"
        }
    }

    pub fn area_status_label(&self) -> String {
        match self.area {
            Area::Town { scene, floor } => format!("{} floor {floor}", scene.key()),
            Area::Dungeon { scene, level } => format!("{} level {level}", scene.key()),
            Area::World { plane } => plane.key().to_string(),
        }
    }

    pub fn spell_stock_summary(&self) -> String {
        let stock = self
            .spell_charges
            .iter()
            .enumerate()
            .filter(|(_, charges)| **charges > 0)
            .map(|(index, charges)| format!("{}={charges}", SPELL_CODES[index]))
            .collect::<Vec<_>>();
        if stock.is_empty() {
            "none".to_string()
        } else {
            stock.join(", ")
        }
    }

    pub fn party_status_summary(&self) -> String {
        let party = self
            .party
            .iter()
            .enumerate()
            .map(|(index, member)| {
                let strength = self
                    .party_strengths
                    .get(index)
                    .copied()
                    .unwrap_or(self.avatar_stats.strength);
                let equipment = self
                    .party_equipment
                    .get(index)
                    .map(readied_equipment_summary)
                    .unwrap_or_else(|| "none".to_string());
                format!(
                    "P{}:slot{} {} STR {} HP {}/{} MP {} L{} equip [{}]",
                    index + 1,
                    member.slot,
                    party_status_name(member.status),
                    strength,
                    member.hp,
                    member.max_hp,
                    member.mana,
                    member.level,
                    equipment
                )
            })
            .collect::<Vec<_>>();
        if party.is_empty() {
            "none".to_string()
        } else {
            party.join("; ")
        }
    }

    pub fn new_order_from_suffix(&mut self, suffix: &str) -> MoveOutcome {
        let Some((first, second)) = parse_inline_party_swap(suffix) else {
            self.message = new_order_prompt_message();
            return MoveOutcome::PromptDeclined;
        };
        let party_len = self.party.len();
        if first >= party_len || second >= party_len {
            self.message = format!(
                "Party has {} member{}.",
                party_len,
                if party_len == 1 { "" } else { "s" }
            );
            return MoveOutcome::Blocked;
        }
        // commands.md §6: if either selected slot is slot zero, the command
        // refuses because the leader must remain first, and it returns
        // without consuming a turn.
        if first == 0 || second == 0 {
            self.message = "The leader must remain first.".to_string();
            return MoveOutcome::Blocked;
        }
        // commands.md §6: picking the same nonzero slot twice is accepted as
        // a behavioural no-op, but the turn is still consumed.
        if first == second {
            self.advance_turn();
            self.message = format!("New order: party slot {} unchanged.", first + 1);
            return MoveOutcome::Used;
        }

        self.party.swap(first, second);
        if first < self.party_names.len() && second < self.party_names.len() {
            self.party_names.swap(first, second);
        }
        if first < self.party_stay_counters.len() && second < self.party_stay_counters.len() {
            self.party_stay_counters.swap(first, second);
        }
        if first < self.party_strengths.len() && second < self.party_strengths.len() {
            self.party_strengths.swap(first, second);
        }
        if first < self.party_equipment.len() && second < self.party_equipment.len() {
            self.party_equipment.swap(first, second);
        }
        // commands.md §6: the command marks the turn as consumed after the
        // exchange.
        self.advance_turn();
        self.message = format!(
            "New order: party slots {} and {} swapped.",
            first + 1,
            second + 1
        );
        MoveOutcome::Used
    }

    pub fn ready_equipment_from_suffix(&mut self, suffix: &str) -> MoveOutcome {
        let request = match parse_inline_ready_request(suffix) {
            Ok(Some(request)) => request,
            Ok(None) => {
                self.message = ready_prompt_message();
                return MoveOutcome::PromptDeclined;
            }
            Err(err) => {
                self.message = format!("{err}");
                return MoveOutcome::Blocked;
            }
        };
        self.ready_equipment(request)
    }

    pub fn ready_equipment(&mut self, request: InlineReadyRequest) -> MoveOutcome {
        let party_len = self.party.len();
        if request.party_index >= party_len {
            self.message = party_member_unavailable_message(party_len);
            return MoveOutcome::Blocked;
        }
        if !self.party[request.party_index].living() {
            self.message = format!("Party member {} is unavailable.", request.party_index + 1);
            return MoveOutcome::Blocked;
        }
        if self.party_equipment.len() < party_len {
            self.party_equipment
                .resize(party_len, [EQUIPMENT_EMPTY; EQUIPMENT_SLOT_COUNT]);
        }
        if self.party_strengths.len() < party_len {
            self.party_strengths
                .resize(party_len, self.avatar_stats.strength);
        }

        let item_id = request.item_id;
        let name = equipment_name(item_id);
        if let Some(slot) = self.party_equipment[request.party_index]
            .iter()
            .position(|item| *item as usize == item_id)
        {
            self.party_equipment[request.party_index][slot] = EQUIPMENT_EMPTY;
            self.equipment_stock[item_id] = self.equipment_stock[item_id]
                .saturating_add(1)
                .min(EQUIPMENT_STOCK_CAP);
            self.message = format!(
                "Unequipped {name} from party member {}; stock is {}.",
                request.party_index + 1,
                self.equipment_stock[item_id]
            );
            if self.combat_active
                && item_id == EQUIPMENT_ID_RING_INVISIBILITY
                && request.party_index < COMBAT_PARTY_ACTOR_SLOTS
                && clear_combat_linked_invisibility(
                    &mut self.combat_actors[request.party_index],
                    &mut self.active_objects,
                )
                .is_some_and(CombatLinkedVisibilityOutcome::changed)
            {
                self.mark_visibility_dirty();
            }
            return MoveOutcome::Used;
        }
        if self.equipment_stock[item_id] == 0 {
            self.message = format!("No carried {name} to ready.");
            return MoveOutcome::Blocked;
        }
        if matches!(item_id, EQUIPMENT_ID_ARROWS | EQUIPMENT_ID_QUARRELS) {
            self.message = format!("{name} are ammunition, not readied equipment.");
            return MoveOutcome::Blocked;
        }
        if matches!(item_id, EQUIPMENT_ID_BOW | EQUIPMENT_ID_MAGIC_BOW)
            && self.equipment_stock[EQUIPMENT_ID_ARROWS] == 0
        {
            self.message = "No arrows for that weapon.".to_string();
            return MoveOutcome::Blocked;
        }
        if item_id == EQUIPMENT_ID_CROSSBOW && self.equipment_stock[EQUIPMENT_ID_QUARRELS] == 0 {
            self.message = "No quarrels for that weapon.".to_string();
            return MoveOutcome::Blocked;
        }

        let Some(slot) = self.ready_target_slot(item_id) else {
            self.message = format!("{name} cannot be readied.");
            return MoveOutcome::Blocked;
        };
        if self.party_equipment[request.party_index][slot] != EQUIPMENT_EMPTY {
            self.message = format!("Remove current {} first.", slot_name(slot));
            return MoveOutcome::Blocked;
        }
        if EQUIPMENT_CLASS_TAGS[item_id] == EQUIPMENT_TAG_TWO_HAND
            && self.party_equipment[request.party_index][EQUIP_SLOT_OFFHAND] != EQUIPMENT_EMPTY
        {
            self.message = "Both hands must be free.".to_string();
            return MoveOutcome::Blocked;
        }
        if slot == EQUIP_SLOT_OFFHAND {
            let weapon = self.party_equipment[request.party_index][EQUIP_SLOT_WEAPON];
            if weapon != EQUIPMENT_EMPTY
                && EQUIPMENT_CLASS_TAGS[weapon as usize] == EQUIPMENT_TAG_TWO_HAND
            {
                self.message = "Weapon hand holds a two-handed item.".to_string();
                return MoveOutcome::Blocked;
            }
        }

        let current_burden = ready_burden(&self.party_equipment[request.party_index]);
        let next_burden = current_burden.saturating_add(EQUIPMENT_READY_BURDENS[item_id]);
        let strength = self.party_strengths[request.party_index];
        if next_burden > strength {
            self.message = format!(
                "Party member {} is not strong enough for {name} ({next_burden}>{strength}).",
                request.party_index + 1
            );
            return MoveOutcome::Blocked;
        }

        self.party_equipment[request.party_index][slot] = item_id as u8;
        self.equipment_stock[item_id] = self.equipment_stock[item_id].saturating_sub(1);
        if is_magic_vanish_ring(item_id)
            && self.ready_ring_vanish_roll(request.party_index, item_id) == 0
        {
            self.party_equipment[request.party_index][slot] = EQUIPMENT_EMPTY;
            self.message = format!(
                "Readied {name} for party member {}, but it vanished.",
                request.party_index + 1
            );
        } else {
            self.message = format!(
                "Readied {name} for party member {} in {}; stock is {}.",
                request.party_index + 1,
                slot_name(slot),
                self.equipment_stock[item_id]
            );
        }
        MoveOutcome::Used
    }

    pub fn ready_target_slot(&self, item_id: usize) -> Option<usize> {
        match EQUIPMENT_CLASS_TAGS.get(item_id).copied()? {
            EQUIPMENT_TAG_HELM => Some(EQUIP_SLOT_HELM),
            EQUIPMENT_TAG_ARMOUR => Some(EQUIP_SLOT_ARMOUR),
            EQUIPMENT_TAG_RING => Some(EQUIP_SLOT_RING),
            EQUIPMENT_TAG_AMULET => Some(EQUIP_SLOT_AMULET),
            EQUIPMENT_TAG_TWO_HAND => Some(EQUIP_SLOT_WEAPON),
            EQUIPMENT_TAG_ONE_HAND if is_shield_item(item_id) => Some(EQUIP_SLOT_OFFHAND),
            EQUIPMENT_TAG_ONE_HAND => Some(EQUIP_SLOT_WEAPON),
            EQUIPMENT_TAG_AMMO => None,
            _ => None,
        }
    }

    pub fn ready_ring_vanish_roll(&self, party_index: usize, item_id: usize) -> u8 {
        (self.turn as u8)
            .wrapping_add(party_index as u8)
            .wrapping_add(item_id as u8)
            & 0x0f
    }

    pub fn cast_light_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        duration: u8,
    ) -> MoveOutcome {
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        self.advance_turn();
        self.light_spell_counter = duration;
        self.recompute_daylight();
        self.message = "Light!".to_string();
        MoveOutcome::Cast
    }

    pub fn cast_vanish(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
    ) -> MoveOutcome {
        let Area::Town { .. } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        let Some(direction) = direction else {
            self.message = "Direction? Use C1AY8/C1AY6/C1AY2/C1AY4.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Vanish requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, VANISH_SPELL_INDEX, VANISH_COST)
        {
            return outcome;
        }

        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let Some(slot) = self.vanishable_object_slot_at_current_floor(tx, ty) else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        };

        let object = self.active_objects[slot];
        self.free_active_object_slot(slot);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Vanished object tile {} at ({tx}, {ty}).", object.tile);
        MoveOutcome::Cast
    }

    pub fn vanishable_object_slot_at_current_floor(&self, x: usize, y: usize) -> Option<usize> {
        self.active_objects
            .iter()
            .enumerate()
            .skip(1)
            .find_map(|(slot, object)| {
                let object = *object;
                if !self.object_occupies(object, x, y)
                    || object.moonstone_slot_index().is_some()
                    || transport_from_vehicle_object(
                        object.type_byte,
                        object.tile,
                        object.aux1,
                        object.aux3,
                    )
                    .is_some()
                    || (192..=255).contains(&object.type_byte)
                    || (192..=255).contains(&object.tile)
                {
                    return None;
                }
                (64..=159).contains(&object.tile).then_some(slot)
            })
    }

    pub fn cast_active_effect_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        tag: u8,
        duration: u8,
        label: &str,
    ) -> MoveOutcome {
        if !self.spell_allowed_in_current_cast_context(spell_index) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        self.advance_turn();
        self.active_effect_tag = Some(tag);
        self.active_effect_counter = duration;
        self.message = format!("{label}!");
        MoveOutcome::Cast
    }

    pub fn cast_reveal(&mut self, caster_index: usize) -> MoveOutcome {
        if !self.combat_active {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, REVEAL_SPELL_INDEX, REVEAL_COST)
        {
            return outcome;
        }

        let revealed = apply_combat_reveal(&mut self.combat_actors);
        if revealed != 0 {
            self.mark_visibility_dirty();
        }
        self.advance_turn();
        self.message = if revealed == 0 {
            "Reveal found nothing.".to_string()
        } else {
            format!("Revealed {revealed} combat actor(s).")
        };
        MoveOutcome::Cast
    }

    pub fn cast_invisibility(&mut self, caster_index: usize) -> MoveOutcome {
        if !self.combat_active {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, INVISIBILITY_SPELL_INDEX, INVISIBILITY_COST)
        {
            return outcome;
        }

        let eligible = caster_index < COMBAT_PARTY_ACTOR_SLOTS;
        self.advance_turn();
        let applied = eligible
            && apply_combat_linked_invisibility(
                &mut self.combat_actors[caster_index],
                &mut self.active_objects,
            )
            .is_some_and(CombatLinkedVisibilityOutcome::changed);
        if applied {
            self.mark_visibility_dirty();
        }
        self.message = if applied {
            "Invisibility!".to_string()
        } else {
            "Failed!".to_string()
        };
        if applied {
            MoveOutcome::Cast
        } else {
            MoveOutcome::Blocked
        }
    }

    pub fn cast_cause_fear(&mut self, caster_index: usize) -> MoveOutcome {
        if !self.combat_active {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, CAUSE_FEAR_SPELL_INDEX, CAUSE_FEAR_COST)
        {
            return outcome;
        }

        let mut groups = [0u8; COMBAT_ACTOR_SLOTS];
        for (slot, group) in groups.iter_mut().enumerate() {
            *group = resolve_combat_target_group_for_actor(self.combat_actors[slot], slot, None);
        }
        let protected_or_immune = [false; COMBAT_ACTOR_SLOTS];
        let caster_group = groups.get(caster_index).copied().unwrap_or(1);
        let targets = collect_cause_fear_actor_slots(
            &self.combat_actors,
            &groups,
            caster_group,
            &protected_or_immune,
        );
        let affected = apply_cause_fear_critical_hp_setup(&mut self.combat_actors, &targets);

        self.advance_turn();
        self.message = if affected == 0 {
            "Cause Fear found no target.".to_string()
        } else {
            format!("Cause Fear affected {affected} combat actor(s).")
        };
        MoveOutcome::Cast
    }

    pub fn cast_awaken(&mut self, caster_index: usize) -> MoveOutcome {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, AWAKEN_SPELL_INDEX, AWAKEN_COST)
        {
            return outcome;
        }

        let Some(target_index) = self.party.iter().position(|member| member.status == b'S') else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        };

        self.party[target_index].status = b'G';
        self.advance_turn();
        self.message = format!("Awakened party member {}.", target_index + 1);
        MoveOutcome::Cast
    }

    pub fn cast_cure(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, CURE_SPELL_INDEX, CURE_COST)
        {
            return outcome;
        }

        if self.party[target_index].status != b'P' {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        self.party[target_index].status = b'G';
        self.advance_turn();
        self.message = format!("Cured party member {}.", target_index + 1);
        MoveOutcome::Cast
    }

    pub fn cast_heal(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, HEAL_SPELL_INDEX, HEAL_COST)
        {
            return outcome;
        }

        if !self.party[target_index].living() {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let amount = self.heal_spell_amount(caster_index, target_index);
        let healed = self.party[target_index].heal_by(amount);
        let hp = self.party[target_index].hp;
        let max_hp = self.party[target_index].max_hp;
        self.advance_turn();
        self.message = format!(
            "Healed party member {} for {healed} HP ({hp}/{max_hp}).",
            target_index + 1
        );
        MoveOutcome::Cast
    }

    pub fn heal_spell_raw_roll(&self, caster_index: usize, target_index: usize) -> u8 {
        self.turn
            .wrapping_add((caster_index as u64).wrapping_mul(17))
            .wrapping_add((target_index as u64).wrapping_mul(14))
            .wrapping_add((self.player.x as u64).wrapping_mul(3))
            .wrapping_add((self.player.y as u64).wrapping_mul(5))
            .wrapping_rem(u64::from(HEAL_RAW_ROLL_MAX) + 1) as u8
    }

    pub fn heal_spell_amount(&self, caster_index: usize, target_index: usize) -> u16 {
        heal_spell_amount_from_raw_roll(self.heal_spell_raw_roll(caster_index, target_index))
    }

    pub fn resurrect_party_member_to_hp(
        &mut self,
        target_index: usize,
        hp_after: u16,
    ) -> Option<u16> {
        if target_index >= self.party.len() || self.party[target_index].status != b'D' {
            return None;
        }

        self.normalize_party_progress_vectors();
        let experience = resurrection_adjusted_experience(
            self.party_experience[target_index],
            self.moral_standing,
        );
        self.party_experience[target_index] = experience;
        let level = recompute_level_from_experience(experience);
        let max_hp = u16::from(level) * RESURRECTION_MAX_HP_PER_LEVEL;
        let intelligence = if target_index == 0 {
            self.avatar_stats.intelligence
        } else {
            self.party_intelligence[target_index]
        };
        let mana = class_refreshed_mana(self.party[target_index].class_byte, intelligence)
            .unwrap_or(self.party[target_index].mana);
        self.party[target_index].status = b'G';
        self.party[target_index].mana = mana;
        self.party[target_index].level = level;
        self.party[target_index].hp = hp_after.min(max_hp);
        self.party[target_index].max_hp = max_hp;
        Some(max_hp)
    }

    pub fn cast_great_heal(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, GREAT_HEAL_SPELL_INDEX, GREAT_HEAL_COST)
        {
            return outcome;
        }

        if !self.party[target_index].living() {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }
        // magic.md §8: Great Heal also fails during the dungeon combat-active
        // substate.
        if matches!(self.area, Area::Dungeon { .. }) && self.combat_active {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let before = self.party[target_index].hp;
        let (_, hp) = self.party[target_index].heal_to_max();
        let healed = hp.saturating_sub(before);
        let max_hp = self.party[target_index].max_hp;
        self.advance_turn();
        self.message = format!(
            "Great healed party member {} for {healed} HP ({hp}/{max_hp}).",
            target_index + 1
        );
        MoveOutcome::Cast
    }

    pub fn cast_resurrect(&mut self, caster_index: usize, target_index: usize) -> MoveOutcome {
        if target_index >= self.party.len() {
            self.message = party_member_unavailable_message(self.party.len());
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, RESURRECT_SPELL_INDEX, RESURRECT_COST)
        {
            return outcome;
        }

        if self.party[target_index].status != b'D' {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let max_hp = self
            .resurrect_party_member_to_hp(target_index, 1)
            .expect("target status checked before spell resurrection");
        self.advance_turn();
        self.message = format!(
            "Resurrected party member {} (1/{max_hp}).",
            target_index + 1
        );
        MoveOutcome::Cast
    }

    pub fn cast_locate(&mut self, caster_index: usize) -> MoveOutcome {
        let Area::World { .. } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, IN_WIS_SPELL_INDEX, IN_WIS_COST)
        {
            return outcome;
        }

        let y = sextant_coordinate(self.player.y);
        let x = sextant_coordinate(self.player.x);
        self.advance_turn();
        // magic.md §8: the sextant-style printer prints Y first, then a
        // comma and the X-coordinate, with a trailing double-quote character.
        self.message = format!("Locate: {y},{x}\"");
        MoveOutcome::Observed
    }

    pub fn cast_peer(&mut self, caster_index: usize) -> MoveOutcome {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, PEER_SPELL_INDEX, PEER_COST)
        {
            return outcome;
        }

        self.advance_turn();
        self.message = self.peer_view_message();
        MoveOutcome::Observed
    }

    pub fn cast_x_ray(&mut self, caster_index: usize) -> MoveOutcome {
        if !spell_allowed_in_area(X_RAY_SPELL_INDEX, self.area) {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, X_RAY_SPELL_INDEX, X_RAY_COST)
        {
            return outcome;
        }

        self.advance_turn();
        self.message = self.x_ray_view_message();
        MoveOutcome::Observed
    }

    pub fn cast_create_food(&mut self, caster_index: usize) -> MoveOutcome {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, CREATE_FOOD_SPELL_INDEX, CREATE_FOOD_COST)
        {
            return outcome;
        }

        let before = self.food;
        self.food = self.food.saturating_add(CREATE_FOOD_AMOUNT);
        let created = self.food.saturating_sub(before);
        self.advance_turn();
        self.message = format!("Created {created} food; stock is {}.", self.food);
        MoveOutcome::Cast
    }

    pub fn peer_view_message(&self) -> String {
        match self.area {
            Area::Dungeon { scene, level } => format!(
                "Peer view of {} ({}) level {} (spell; centered flood map):\n{}",
                scene.key(),
                scene.name(),
                level,
                self.dungeon_vision_map(level)
            ),
            Area::Town { scene, floor } => format!(
                "Peer view of {} floor {} (spell; 32x32 class map):\n{}",
                scene.key(),
                floor,
                self.surface_view_map()
            ),
            Area::World { plane } => format!(
                "Peer view of {} at ({}, {}) (spell; 32x32 class map):\n{}",
                plane.key(),
                self.player.x,
                self.player.y,
                self.surface_view_map()
            ),
        }
    }

    pub fn x_ray_view_message(&self) -> String {
        match self.area {
            Area::Town { scene, floor } => format!(
                "X-Ray view of {} floor {} (spell; 32x32 class map):\n{}",
                scene.key(),
                floor,
                self.surface_view_map()
            ),
            Area::World { plane } => format!(
                "X-Ray view of {} at ({}, {}) (spell; 32x32 class map):\n{}",
                plane.key(),
                self.player.x,
                self.player.y,
                self.surface_view_map()
            ),
            Area::Dungeon { .. } => "Not here!".to_string(),
        }
    }

    pub fn cast_open_spell(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
        _game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        if !matches!(self.area, Area::Dungeon { .. }) {
            let Some(direction) = direction else {
                self.message = "Direction? Use C1AS8/C1AS6/C1AS2/C1AS4.".to_string();
                return Ok(MoveOutcome::Blocked);
            };
            if !direction.is_cardinal() {
                self.message = "Open requires a cardinal direction.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, OPEN_SPELL_INDEX, OPEN_SPELL_COST)
        {
            return Ok(outcome);
        }

        let Area::Dungeon { scene, level } = self.area else {
            return Ok(self.cast_open_ordinary_surface_door(direction));
        };
        let idx = dungeon_cell_index(level, self.player.x, self.player.y);
        let tile = self.grid[idx];
        if tile >> 4 != 0x4 {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.grid[idx] = 0x70 | (tile & 0x0f);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Safely opened dungeon chest at ({}, {}) on {} level {level}; trap generator bypassed by An Sanct, marked visit-local open chest.",
            self.player.x,
            self.player.y,
            scene.key()
        );
        Ok(MoveOutcome::ContainerOpened)
    }

    pub fn cast_open_ordinary_surface_door(&mut self, direction: Option<Direction>) -> MoveOutcome {
        let Some(direction) = direction else {
            self.message = "Direction? Use C1AS8/C1AS6/C1AS2/C1AS4.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Open requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }

        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        let Some(idx) = (match self.area {
            Area::World { .. } => {
                let tx = tx.rem_euclid(WORLD_SIDE as isize) as usize;
                let ty = ty.rem_euclid(WORLD_SIDE as isize) as usize;
                Some(world_cell_index(tx, ty))
            }
            Area::Town { .. } => {
                if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
                    None
                } else {
                    Some(ty as usize * 32 + tx as usize)
                }
            }
            Area::Dungeon { .. } => None,
        }) else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        };

        self.grid[idx] = match self.grid[idx] {
            0x97 => 0xb8,
            0x98 => 0xba,
            _ => {
                self.advance_turn();
                self.message = "Failed!".to_string();
                return MoveOutcome::Blocked;
            }
        };
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = "Opened!".to_string();
        MoveOutcome::DoorOpened
    }

    pub fn cast_dungeon_level_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        delta: i8,
        label: &str,
    ) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, spell_index, DUNGEON_LEVEL_SPELL_COST)
        {
            return outcome;
        }

        let next_level = level as i8 + delta;
        if !(0..=7).contains(&next_level) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let next_level = next_level as u8;
        self.area = Area::Dungeon {
            scene,
            level: next_level,
        };
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "{label}! Changed to {} ({}) level {next_level}.",
            scene.key(),
            scene.name()
        );
        MoveOutcome::Transition(AreaTransition::ChangedDungeonLevel {
            scene,
            level: next_level,
        })
    }

    pub fn cast_dungeon_field_spell(
        &mut self,
        caster_index: usize,
        spell_index: usize,
        mana_cost: u8,
        direction: Option<Direction>,
        base_field: u8,
        marker_field: u8,
        label: &str,
    ) -> MoveOutcome {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        let Some(direction) = direction else {
            self.message = "Direction? Use C1FGI6/C1GIN6/C1GIZ6/C1GIS6.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Field placement requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) = self.cast_spell_resource_gate(caster_index, spell_index, mana_cost) {
            return outcome;
        }

        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&tx) || !(0..DUNGEON_SIDE as isize).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let idx = dungeon_cell_index(level, tx as usize, ty as usize);
        self.grid[idx] = match self.grid[idx] {
            0x00 => base_field,
            0x08 => marker_field,
            _ => {
                self.advance_turn();
                self.message = "Failed!".to_string();
                return MoveOutcome::Blocked;
            }
        };
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "{label} placed {} at ({}, {}) on {} level {level}.",
            direction.name(),
            tx,
            ty,
            scene.key()
        );
        MoveOutcome::Cast
    }

    pub fn cast_dispel_field(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
    ) -> MoveOutcome {
        if self.combat_active {
            return self.cast_combat_dispel_field(caster_index, direction);
        }
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not here!".to_string();
            return MoveOutcome::Blocked;
        };
        let Some(direction) = direction else {
            self.message = "Direction? Use C1AG6.".to_string();
            return MoveOutcome::Blocked;
        };
        if !direction.is_cardinal() {
            self.message = "Dispel Field requires a cardinal direction.".to_string();
            return MoveOutcome::Blocked;
        }
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, DISPEL_FIELD_SPELL_INDEX, DISPEL_FIELD_COST)
        {
            return outcome;
        }

        let (dx, dy) = direction.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..DUNGEON_SIDE as isize).contains(&tx) || !(0..DUNGEON_SIDE as isize).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        }

        let idx = dungeon_cell_index(level, tx as usize, ty as usize);
        let cell = self.grid[idx];
        let Some(field) = dungeon_field_effect(cell) else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return MoveOutcome::Blocked;
        };
        self.grid[idx] = cell & 0x08;
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Dispelled {} at ({}, {}) on {} level {level}.",
            field.label(),
            tx,
            ty,
            scene.key()
        );
        MoveOutcome::Cast
    }

    pub fn cast_time_stop(&mut self, caster_index: usize) -> MoveOutcome {
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, TIME_STOP_SPELL_INDEX, TIME_STOP_COST)
        {
            return outcome;
        }

        self.advance_turn();
        self.active_effect_tag = Some(NEGATE_TIME_ACTIVE_EFFECT_TAG);
        self.active_effect_counter = TIME_STOP_DURATION;
        self.message = "Negate time!".to_string();
        MoveOutcome::Cast
    }

    pub fn cast_blink(
        &mut self,
        caster_index: usize,
        direction: Option<Direction>,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let Some(direction) = direction else {
            self.message = "Direction? Use C1IP6.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let Some(entry) = self.blink_target_at(game_dir, direction)? else {
            self.message = "No Blink target.".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, BLINK_SPELL_INDEX, BLINK_COST)
        {
            return Ok(outcome);
        }

        if !self.blink_source_matches(entry) || !self.blink_destination_legal(game_dir, entry)? {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Blinked {} to ({}, {}) in {}.",
            direction.name(),
            entry.to_x,
            entry.to_y,
            self.current_area_label()
        );
        Ok(MoveOutcome::Cast)
    }

    pub fn blink_target_at(
        &self,
        game_dir: &Path,
        direction: Direction,
    ) -> io::Result<Option<BlinkTargetEntry>> {
        let (target, floor, x, y) = self.current_blink_context();
        Ok(load_blink_target_entries(game_dir)?.and_then(|entries| {
            entries.into_iter().find(|entry| {
                entry.target == target
                    && entry.floor == floor
                    && entry.from_x == x
                    && entry.from_y == y
                    && entry.direction == direction
            })
        }))
    }

    pub fn current_blink_context(&self) -> (PlayTarget, i8, usize, usize) {
        match self.area {
            Area::World { plane } => (
                PlayTarget::World(plane),
                plane.save_floor(),
                self.player.x,
                self.player.y,
            ),
            Area::Town { scene, floor } => {
                (PlayTarget::Town(scene), floor, self.player.x, self.player.y)
            }
            Area::Dungeon { scene, level } => (
                PlayTarget::Dungeon(scene),
                level as i8,
                self.player.x,
                self.player.y,
            ),
        }
    }

    pub fn current_area_label(&self) -> String {
        match self.area {
            Area::World { plane } => plane.key().to_string(),
            Area::Town { scene, floor } => format!("{} floor {floor}", scene.key()),
            Area::Dungeon { scene, level } => format!("{} level {level}", scene.key()),
        }
    }

    pub fn blink_source_matches(&self, entry: BlinkTargetEntry) -> bool {
        entry.expected_from_tile.map_or(true, |expected| {
            expected == self.current_area_tile(entry.from_x, entry.from_y)
        })
    }

    pub fn blink_destination_legal(
        &self,
        game_dir: &Path,
        entry: BlinkTargetEntry,
    ) -> io::Result<bool> {
        if entry.expected_to_tile.map_or(false, |expected| {
            expected != self.current_area_tile(entry.to_x, entry.to_y)
        }) {
            return Ok(false);
        }
        match self.area {
            Area::World { .. } | Area::Town { .. } => {
                self.player_can_land_on_foot(Some(game_dir), entry.to_x, entry.to_y)
            }
            Area::Dungeon { scene, level } => {
                let cell = self.dungeon_cell(level, entry.to_x, entry.to_y);
                if self.dungeon_closed_door_at(
                    Some(game_dir),
                    scene,
                    level,
                    entry.to_x,
                    entry.to_y,
                    cell,
                )? {
                    return Ok(false);
                }
                Ok(is_dungeon_walkable(cell)
                    || self.dungeon_open_door_at(
                        Some(game_dir),
                        scene,
                        level,
                        entry.to_x,
                        entry.to_y,
                        cell,
                    )?)
            }
        }
    }

    pub fn current_area_tile(&self, x: usize, y: usize) -> u8 {
        match self.area {
            Area::World { .. } => self.grid[world_cell_index(x, y)],
            Area::Town { .. } => self.grid[y * 32 + x],
            Area::Dungeon { level, .. } => self.dungeon_cell(level, x, y),
        }
    }

    pub fn cast_magic_lock(
        &mut self,
        caster_index: usize,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, MAGIC_LOCK_SPELL_INDEX, MAGIC_LOCK_COST)
        {
            return Ok(outcome);
        }

        let (dx, dy) = self.player.facing.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        let Some(entry) = self.town_magic_lock_target_at(game_dir, scene, floor, tx, ty, tile)?
        else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        self.grid[idx] = entry.locked_tile;
        self.forget_open_town_door(scene, floor, tx, ty);
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = "Magic lock!".to_string();
        Ok(MoveOutcome::Cast)
    }

    pub fn cast_unlock_magic(
        &mut self,
        caster_index: usize,
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if let Some(outcome) =
            self.cast_spell_resource_gate(caster_index, UNLOCK_MAGIC_SPELL_INDEX, UNLOCK_MAGIC_COST)
        {
            return Ok(outcome);
        }

        let (dx, dy) = self.player.facing.delta();
        let tx = self.player.x as isize + dx;
        let ty = self.player.y as isize + dy;
        if !(0..32).contains(&tx) || !(0..32).contains(&ty) {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let tx = tx as usize;
        let ty = ty as usize;
        let idx = ty * 32 + tx;
        let tile = self.grid[idx];
        let Some(entry) = self.town_lock_at(Some(game_dir), scene, floor, tx, ty, tile)? else {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        if entry.kind != TownLockKind::Magic {
            self.advance_turn();
            self.message = "Failed!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.grid[idx] = entry.unlocked_tile;
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = "Unlocked!".to_string();
        Ok(MoveOutcome::Cast)
    }
}

fn append_inventory_rows(lines: &mut Vec<String>, rows: Vec<String>) {
    if rows.is_empty() {
        lines.push("None.".to_string());
        return;
    }
    let mut shown = 0;
    let total = rows.len();
    for row in rows.into_iter().take(Z_STATS_INVENTORY_PANEL_ROWS) {
        shown += 1;
        lines.push(row);
    }
    if total > shown {
        lines.push(format!("... {} more", total - shown));
    }
}

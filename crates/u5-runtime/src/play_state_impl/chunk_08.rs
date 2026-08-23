use std::io;
use std::path::Path;

use crate::rest_camp::{
    COMPLETED_LONG_CAMP_HP_GAIN_MAX, COMPLETED_LONG_CAMP_HP_GAIN_MIN, COMPLETED_LONG_CAMP_MIN_HOURS,
};
use crate::*;

impl PlayState {
    pub fn start_rest_prompt(&mut self) -> MoveOutcome {
        self.active_rest = Some(RestSession::new());
        self.message = self.render_active_rest();
        MoveOutcome::Observed
    }

    pub fn render_active_rest(&self) -> String {
        self.active_rest
            .as_ref()
            .map(|session| self.render_rest_session(session))
            .unwrap_or_else(|| "Rest?".to_string())
    }

    fn render_rest_session(&self, session: &RestSession) -> String {
        match session.phase {
            RestPhase::Hours => {
                let label = if matches!(self.area, Area::Town { .. }) {
                    "Hole up"
                } else {
                    "Rest"
                };
                // cleak/u5-spec#81: the hours prompt literal is unpublished; the
                // invented `Choose 1-9; Space/0 cancels.` line is removed.
                format!("{label}- how many hours? _")
            }
            RestPhase::WatchYesNo => "Set watch? (Y/N)".to_string(),
            RestPhase::WatchSlot => {
                let last = self.party.len().min(6);
                format!(
                    "Who keeps watch? _\nChoose party member 1-{last}; Space/0 leaves no watch."
                )
            }
        }
    }

    pub fn step_active_rest(
        &mut self,
        key: char,
        suffix: &str,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(mut session) = self.active_rest.take() else {
            return Ok(None);
        };
        for ch in std::iter::once(key).chain(suffix.chars()) {
            match session.phase {
                RestPhase::Hours => {
                    if ch == '\u{1b}' {
                        self.message = "None!".to_string();
                        return Ok(None);
                    }
                    let duration_input = if ch.is_ascii() {
                        rest_duration_input(ch as u8)
                    } else {
                        RestDurationInput::Discard
                    };
                    match duration_input {
                        RestDurationInput::Hours(hours) => {
                            if !matches!(self.area, Area::Town { .. })
                                && self.rest_watch_prompt_needed()
                            {
                                session.hours = Some(hours);
                                session.phase = RestPhase::WatchYesNo;
                                self.active_rest = Some(session);
                                self.message = self.render_active_rest();
                                return Ok(None);
                            }
                            return self.finish_active_rest(hours, None, game_dir);
                        }
                        RestDurationInput::Cancel => {
                            self.message = "None!".to_string();
                            return Ok(None);
                        }
                        RestDurationInput::Discard => {}
                    }
                }
                RestPhase::WatchYesNo => match ch.to_ascii_uppercase() {
                    'Y' => {
                        session.phase = RestPhase::WatchSlot;
                        self.active_rest = Some(session);
                        self.message = self.render_active_rest();
                        return Ok(None);
                    }
                    'N' | '\u{1b}' | ' ' | '0' | '\r' | '\n' => {
                        let hours = session.hours.unwrap_or(1);
                        return self.finish_active_rest(hours, None, game_dir);
                    }
                    _ => {}
                },
                RestPhase::WatchSlot => {
                    if matches!(ch, '\u{1b}' | ' ' | '0' | '\r' | '\n') {
                        let hours = session.hours.unwrap_or(1);
                        return self.finish_active_rest(hours, None, game_dir);
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
                    let hours = session.hours.unwrap_or(1);
                    let watcher = self.rest_prompt_watcher(digit - 1);
                    return self.finish_active_rest(hours, watcher, game_dir);
                }
            }
        }
        self.active_rest = Some(session);
        self.message = self.render_active_rest();
        Ok(None)
    }

    fn finish_active_rest(
        &mut self,
        hours: u8,
        watcher: Option<usize>,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let request = InlineRestRequest {
            hours: Some(hours),
            watcher,
        };
        self.hole_up_command(game_dir, request).map(Some)
    }

    fn rest_watch_prompt_needed(&self) -> bool {
        self.rest_watch_eligible_count() > 1
    }

    fn rest_watch_eligible_count(&self) -> usize {
        self.party
            .iter()
            .filter(|member| {
                character_status_for_byte(member.status).is_some_and(rest_with_watch_participates)
                    && member.living()
            })
            .count()
    }

    fn rest_prompt_watcher(&self, watcher: usize) -> Option<usize> {
        if !self.rest_watch_prompt_needed() {
            return None;
        }
        let member = self.party.get(watcher)?;
        (member.status == b'G' && member.living()).then_some(watcher)
    }

    pub fn rest_with_watch(
        &mut self,
        request: InlineRestRequest,
        game_dir: Option<&Path>,
    ) -> io::Result<MoveOutcome> {
        let Some(hours) = request.hours else {
            return Ok(self.start_rest_prompt());
        };
        if !(1..=9).contains(&hours) {
            self.message = "Rest hours must be in 1..9.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let watch_note = self.rest_watch_note(request.watcher);
        let rest_entry_statuses = self
            .party
            .iter()
            .map(|member| member.status)
            .collect::<Vec<_>>();
        let asleep_at_start = self
            .party
            .iter()
            .filter(|member| member.status == b'S')
            .map(|member| member.slot)
            .collect::<Vec<_>>();
        let mut recovered_hp = 0;
        let mut recovered_mana = 0;
        let mut world_damage_ticks = 0;
        let mut last_world_damage = None;
        let mut world_status_ticks = 0;
        let mut last_world_status = None;
        let mut interrupted = false;
        let mut ambush_monster = None;
        let mut rest_ticks = 0u64;
        'resting: for _ in 0..hours {
            for _ in 0..REST_WATCH_TICKS_PER_HOUR {
                self.advance_turn_with_minutes(REST_WATCH_MINUTES_PER_TICK);
                rest_ticks += 1;
                let (hp, mana) = self.apply_rest_with_watch_recovery_tick();
                recovered_hp += hp;
                recovered_mana += mana;
                if let (Some(game_dir), Area::World { plane }) = (game_dir, self.area) {
                    if let Some(report) =
                        self.apply_world_underfoot_damage(Some(game_dir), plane)?
                    {
                        world_damage_ticks += 1;
                        last_world_damage = Some(report);
                    }
                    if let Some(report) = self.apply_world_underfoot_status_tick(plane) {
                        world_status_ticks += 1;
                        last_world_status = Some(report);
                    }
                }
                if self.dangerous_rest_interrupted() {
                    interrupted = true;
                    ambush_monster = sleep_ambush_monster(self.sleep_ambush_monster_row());
                    break 'resting;
                }
            }
        }
        let woke = if interrupted {
            self.restore_sleep_ambush_party_statuses(&rest_entry_statuses)
        } else {
            self.wake_initial_rest_sleepers(&asleep_at_start)
        };
        if !interrupted {
            let (hp, mana) = self.apply_completed_long_camp_recovery(
                hours,
                request.watcher,
                &rest_entry_statuses,
            );
            recovered_hp += hp;
            recovered_mana += mana;
        }
        let completed_hours = rest_ticks / u64::from(REST_WATCH_TICKS_PER_HOUR);
        let completed_minutes = (rest_ticks % u64::from(REST_WATCH_TICKS_PER_HOUR))
            * u64::from(REST_WATCH_MINUTES_PER_TICK);
        let duration = if completed_minutes == 0 {
            format!(
                "{completed_hours} hour{}",
                if completed_hours == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "{completed_hours} hour{} {completed_minutes} minute{}",
                if completed_hours == 1 { "" } else { "s" },
                if completed_minutes == 1 { "" } else { "s" }
            )
        };
        self.message = format!(
            "Party rested {duration}; {watch_note}; recovered {recovered_hp} HP and {recovered_mana} MP; woke {woke} asleep member(s).",
        );
        if let Some(monster) = ambush_monster {
            let z = match self.area {
                Area::World { plane } => plane.save_floor(),
                Area::Dungeon { level, .. } => level as i8,
                Area::Town { floor, .. } => floor,
            };
            let note = self.enter_sleep_ambush_combat(monster, z)?;
            self.message.push_str(&format!(" Ambushed! {note}."));
        }
        if let Some(report) = last_world_damage {
            self.message.push_str(&format!(
                " Underfoot world damage triggered {world_damage_ticks} tick(s); last {report}."
            ));
        }
        if let Some(report) = last_world_status {
            self.message.push_str(&format!(
                " Underfoot world status triggered {world_status_ticks} tick(s); last {report}."
            ));
        }
        self.append_pending_hourly_status_message();
        if !interrupted
            && matches!(self.area, Area::World { .. })
            && lord_british_camp_event_triggered(self.lord_british_camp_event_roll())
        {
            let event_message = self.resolve_lord_british_camp_event(game_dir)?;
            self.message.push(' ');
            self.message.push_str(&event_message);
        }
        Ok(MoveOutcome::Rested)
    }

    pub fn rest_watch_note(&self, watcher: Option<usize>) -> String {
        let eligible_count = self
            .party
            .iter()
            .filter(|member| {
                character_status_for_byte(member.status).is_some_and(rest_with_watch_participates)
                    && member.living()
            })
            .count();
        let Some(watcher) = watcher else {
            return if eligible_count > 1 {
                "no watch set".to_string()
            } else {
                "no watch needed".to_string()
            };
        };
        let Some(member) = self.party.get(watcher) else {
            return "no valid watch set".to_string();
        };
        if eligible_count <= 1 || member.status != b'G' || !member.living() {
            return "no valid watch set".to_string();
        }
        format!("party slot {} keeps watch", watcher + 1)
    }

    pub fn lord_british_camp_event_roll(&mut self) -> u8 {
        self.random_range_u8(0, 99)
    }

    pub fn resolve_lord_british_camp_event(
        &mut self,
        game_dir: Option<&Path>,
    ) -> io::Result<String> {
        self.normalize_party_progress_vectors();
        let mut notes = vec!["Lord British-in-disguise camp event.".to_string()];
        let mut level_changes = 0;
        for index in 0..self.party.len() {
            if !self.party[index].living() {
                continue;
            }
            let experience = self.party_experience[index];
            let level = level_for_experience(u32::from(experience));
            if self.party[index].level == level {
                continue;
            }
            self.party[index].level = level;
            let hp = lord_british_camp_event_hp_for_level(level);
            self.party[index].hp = hp;
            self.party[index].max_hp = hp;
            let reward = self.apply_lord_british_camp_stat_reward(index);
            self.refresh_party_member_class_mana(index);
            level_changes += 1;
            notes.push(format!(
                "P{} reached level {level} from {experience} XP and received {reward}.",
                index + 1
            ));
        }
        if level_changes == 0 {
            notes.push("No living party member was ready for a new level.".to_string());
        }
        notes.push(self.lord_british_camp_verdict_message(game_dir)?);
        self.mark_visibility_dirty();
        Ok(notes.join(" "))
    }

    pub fn normalize_party_progress_vectors(&mut self) {
        let len = self.party.len();
        self.party_names.resize(len, [0; SAVE_CHARACTER_NAME_LEN]);
        self.party_experience.resize(len, 0);
        self.party_stay_counters.resize(len, 0);
        self.party_strengths.resize(len, self.avatar_stats.strength);
        self.party_intelligence
            .resize(len, self.avatar_stats.intelligence);
        if len > 0 {
            self.party_strengths[0] = self.avatar_stats.strength;
            self.party_intelligence[0] = self.avatar_stats.intelligence;
        }
    }

    pub fn apply_lord_british_camp_stat_reward(&mut self, member_index: usize) -> &'static str {
        match lord_british_camp_stat_reward(self.lord_british_camp_stat_roll(member_index)) {
            Some(LordBritishCampStatReward::Strength) => {
                if member_index == 0 {
                    self.avatar_stats.increase_strength();
                    self.party_strengths[0] = self.avatar_stats.strength;
                } else if let Some(strength) = self.party_strengths.get_mut(member_index) {
                    increase_capped_stat(strength);
                }
                "Strength reward"
            }
            Some(LordBritishCampStatReward::Dexterity) => {
                if member_index == 0 {
                    self.avatar_stats.increase_dexterity();
                    if let Some(member) = self.party.get_mut(0) {
                        member.climb_stat = self.avatar_stats.dexterity;
                    }
                } else if let Some(member) = self.party.get_mut(member_index) {
                    increase_capped_stat(&mut member.climb_stat);
                }
                "Dexterity reward"
            }
            Some(LordBritishCampStatReward::Intelligence) => {
                if member_index == 0 {
                    self.avatar_stats.increase_intelligence();
                    self.party_intelligence[0] = self.avatar_stats.intelligence;
                } else if let Some(intelligence) = self.party_intelligence.get_mut(member_index) {
                    increase_capped_stat(intelligence);
                }
                "Intelligence reward"
            }
            None => unreachable!("Lord British camp stat roll is constrained to 1..=3"),
        }
    }

    pub fn lord_british_camp_stat_roll(&mut self, _member_index: usize) -> u8 {
        self.random_range_u8(1, 3)
    }

    pub fn refresh_party_member_class_mana(&mut self, member_index: usize) {
        let Some(member) = self.party.get(member_index).copied() else {
            return;
        };
        let intelligence = if member_index == 0 {
            self.avatar_stats.intelligence
        } else {
            self.party_intelligence
                .get(member_index)
                .copied()
                .unwrap_or(self.avatar_stats.intelligence)
        };
        if let Some(mana) = lord_british_camp_refreshed_mana(member.class_byte, intelligence) {
            if let Some(member) = self.party.get_mut(member_index) {
                member.mana = mana;
            }
        }
    }

    pub fn lord_british_camp_verdict_message(&self, game_dir: Option<&Path>) -> io::Result<String> {
        let record_index = lord_british_camp_karma_record_index(self.moral_standing);
        let Some(game_dir) = game_dir else {
            return Ok(format!(
                "KARMA.DAT verdict record {record_index} unavailable."
            ));
        };
        let Some(records) = load_karma_records(game_dir)? else {
            return Ok(format!(
                "KARMA.DAT verdict record {record_index} unavailable."
            ));
        };
        Ok(records
            .get(record_index)
            .map(|record| format!("Verdict: {record}"))
            .unwrap_or_else(|| format!("KARMA.DAT verdict record {record_index} unavailable.")))
    }

    pub fn dangerous_rest_interrupted(&mut self) -> bool {
        matches!(self.area, Area::World { .. } | Area::Dungeon { .. })
            && sleep_ambush_rest_interrupted(self.dangerous_rest_interrupt_roll())
    }

    pub fn dangerous_rest_interrupt_roll(&mut self) -> u8 {
        self.random_range_u8(0, 63)
    }

    pub fn sleep_ambush_monster_row(&mut self) -> u8 {
        self.random_range_u8(0, 7)
    }

    pub fn restore_sleep_ambush_party_statuses(&mut self, entry_statuses: &[u8]) -> usize {
        let mut restored = 0;
        for (index, member) in self.party.iter_mut().enumerate() {
            if !member.living() {
                continue;
            }
            let Some(entry_status) = entry_statuses
                .get(index)
                .and_then(|status| character_status_for_byte(*status))
            else {
                continue;
            };
            let restored_status = sleep_ambush_restored_status(entry_status).save_byte();
            if member.status != restored_status {
                member.status = restored_status;
                restored += 1;
            }
        }
        restored
    }

    pub fn apply_rest_recovery_tick(&mut self) -> (u16, u16) {
        // `cleak/u5-spec#47`: ordinary rest advances time only. The
        // hourly clock path has already applied provisions, poison,
        // starvation, schedules, and ambient effects.
        (0, 0)
    }

    pub fn apply_rest_with_watch_recovery_tick(&mut self) -> (u16, u16) {
        // Watch participation affects prompts, interruption setup, and
        // cleanup status transitions, not direct HP/MP restoration.
        (0, 0)
    }

    pub fn apply_completed_long_camp_recovery(
        &mut self,
        accepted_hours: u8,
        watcher: Option<usize>,
        entry_statuses: &[u8],
    ) -> (u16, u16) {
        if accepted_hours < COMPLETED_LONG_CAMP_MIN_HOURS {
            return (0, 0);
        }

        let mut recovered_hp = 0u16;
        let mut recovered_mana = 0u16;
        for index in 0..self.party.len() {
            if watcher == Some(index) || entry_statuses.get(index).copied() == Some(b'P') {
                continue;
            }
            let member = self.party[index];
            if !member.living() {
                continue;
            }

            let hp_gain = u16::from(self.random_range_u8(
                COMPLETED_LONG_CAMP_HP_GAIN_MIN,
                COMPLETED_LONG_CAMP_HP_GAIN_MAX,
            ));
            let hp_target = member.hp.saturating_add(hp_gain).min(member.max_hp);
            recovered_hp = recovered_hp.saturating_add(hp_target.saturating_sub(member.hp));
            self.party[index].hp = hp_target;

            let intelligence = if index == 0 {
                self.avatar_stats.intelligence
            } else {
                self.party_intelligence
                    .get(index)
                    .copied()
                    .unwrap_or(self.avatar_stats.intelligence)
            };
            if let Some(mana_target) =
                lord_british_camp_refreshed_mana(member.class_byte, intelligence)
            {
                recovered_mana = recovered_mana
                    .saturating_add(u16::from(mana_target.saturating_sub(member.mana)));
                self.party[index].mana = mana_target;
            }
        }
        (recovered_hp, recovered_mana)
    }

    pub fn apply_inn_rest_night_recovery(&mut self) -> (u16, u16, usize) {
        let mut recovered_hp = 0u16;
        let mut recovered_mana = 0u16;
        let mut status_changes = 0usize;
        for (index, member) in self.party.iter_mut().enumerate() {
            if member.status == b'D' {
                continue;
            }
            if member.status == b'P' {
                member.status = b'D';
                member.hp = 0;
                status_changes += 1;
                continue;
            }
            if member.status == b'S' {
                member.status = b'G';
                status_changes += 1;
            }

            let hp_target = inn_rest_hp_target(member.class_byte, member.max_hp);
            if member.hp < hp_target {
                recovered_hp += hp_target - member.hp;
                member.hp = hp_target;
            }

            let intelligence = self.party_intelligence.get(index).copied().unwrap_or(0);
            if let Some(mana_target) = inn_rest_mana_target(member.class_byte, intelligence) {
                if member.mana < mana_target {
                    recovered_mana += u16::from(mana_target - member.mana);
                    member.mana = mana_target;
                }
            }
        }
        (recovered_hp, recovered_mana, status_changes)
    }

    pub fn mark_town_rest_sleepers(&mut self) -> usize {
        let mut marked = 0;
        for member in &mut self.party {
            let Some(status) = character_status_for_byte(member.status) else {
                continue;
            };
            if town_rest_temp_sleep_marked(status) && member.living() {
                member.status = b'S';
                marked += 1;
            }
        }
        marked
    }

    pub fn wake_town_rest_sleepers(&mut self) -> usize {
        let mut woke = 0;
        for member in &mut self.party {
            let Some(status) = character_status_for_byte(member.status) else {
                continue;
            };
            if rest_cleanup_transitions_to_good(status) && member.hp > 0 {
                member.status = b'G';
                woke += 1;
            }
        }
        woke
    }

    pub fn wake_initial_rest_sleepers(&mut self, asleep_at_start: &[u8]) -> usize {
        let mut woke = 0;
        for member in &mut self.party {
            let Some(status) = character_status_for_byte(member.status) else {
                continue;
            };
            if rest_cleanup_transitions_to_good(status)
                && member.hp > 0
                && asleep_at_start.contains(&member.slot)
            {
                member.status = b'G';
                woke += 1;
            }
        }
        woke
    }

    pub fn idle_tick(&mut self) -> MoveOutcome {
        self.advance_visual_tick();
        if let Some(wind) = self.idle_wind_drift() {
            // weather.md §2: on the underworld plane the wind state still
            // updates, but the helper uses its non-surface presentation
            // branch instead of printing the ordinary cardinal wind label.
            let on_surface = matches!(
                self.area,
                Area::World {
                    plane: WorldPlane::Britannia
                }
            );
            self.message = if on_surface {
                format!("Idle animation tick. {}", wind.status_message())
            } else {
                "Idle animation tick. The air shifts.".to_string()
            };
        } else {
            self.message = "Idle animation tick.".to_string();
        }
        MoveOutcome::IdleTick
    }

    pub fn idle_wind_drift(&mut self) -> Option<WindState> {
        if !matches!(self.area, Area::World { .. })
            || !WindState::autonomous_drift_outer_accepted(self.idle_wind_roll(0))
        {
            return None;
        }
        for attempt in 0..=u8::MAX {
            let candidate = self.idle_wind_candidate(attempt);
            if candidate != WindState::Calm
                || self.idle_wind_roll(attempt.wrapping_add(2)) >= WIND_DRIFT_CALM_ACCEPT_MIN
            {
                self.apply_wind_state(candidate);
                return Some(candidate);
            }
        }
        None
    }

    pub fn idle_wind_candidate(&self, attempt: u8) -> WindState {
        match self.idle_wind_roll(attempt.wrapping_add(1)) % WIND_DRIFT_CANDIDATE_MODULUS {
            0 => WindState::Calm,
            1 => WindState::North,
            2 => WindState::South,
            3 => WindState::East,
            _ => WindState::West,
        }
    }

    pub fn idle_wind_roll(&self, attempt: u8) -> u8 {
        self.turn as u8
            ^ self.clock.hour.wrapping_mul(3)
            ^ self.clock.minute.wrapping_mul(5)
            ^ (self.player.x as u8).wrapping_mul(7)
            ^ (self.player.y as u8).wrapping_mul(11)
            ^ self.animation.frame.wrapping_mul(13)
            ^ attempt.wrapping_mul(17)
    }

    pub fn ignite_torch(&mut self) -> MoveOutcome {
        if self.torches == 0 {
            self.message = "No torches!".to_string();
            return MoveOutcome::Blocked;
        }

        self.torches = self.torches.saturating_sub(1);
        self.advance_turn();
        match self.area {
            Area::Dungeon { .. } => {
                let added = self.dungeon_torch_duration_roll();
                self.torch_counter = self.torch_counter.saturating_add(added);
                self.recompute_daylight();
                self.message = format!(
                    "Ignited a torch; dungeon light counter is {} and {} torch(es) remain.",
                    self.torch_counter, self.torches
                );
            }
            _ => {
                self.torch_counter = SURFACE_TORCH_DURATION;
                self.recompute_daylight();
                self.message = format!(
                    "Ignited a torch; light counter is {} and {} torch(es) remain.",
                    self.torch_counter, self.torches
                );
            }
        }
        MoveOutcome::Ignited
    }

    pub fn klimb_command(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        match self.area {
            Area::Town { scene, floor } => {
                let tile = self.grid[self.player.y * 32 + self.player.x];
                let choices = self.connected_town_climb_choices(
                    game_dir,
                    scene,
                    floor,
                    self.player.x,
                    self.player.y,
                    tile,
                )?;
                match choices.as_slice() {
                    [] => {
                        self.message = "Not climbable!".to_string();
                        Ok(MoveOutcome::Blocked)
                    }
                    [intent] => self.climb(game_dir, *intent),
                    _ => Ok(self.start_klimb_direction_prompt()),
                }
            }
            Area::Dungeon { level, .. } => {
                let tile = self.dungeon_cell(level, self.player.x, self.player.y);
                if tile == 0x60 {
                    self.climb(game_dir, ClimbIntent::Up)
                } else {
                    match tile >> 4 {
                        0x1 => self.climb(game_dir, ClimbIntent::Up),
                        0x2 => self.climb(game_dir, ClimbIntent::Down),
                        0x3 => Ok(self.start_klimb_direction_prompt()),
                        _ => {
                            self.message = "Not climbable!".to_string();
                            Ok(MoveOutcome::Blocked)
                        }
                    }
                }
            }
            Area::World { plane } => self.climb_outdoors(game_dir, plane),
        }
    }

    pub fn connected_town_climb_choices(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Vec<ClimbIntent>> {
        if let Some(entry) = self.town_stair_at(game_dir, scene, floor, x, y, tile)? {
            return self.available_town_climb_choices(game_dir, scene, floor, entry.kind.intents());
        }
        if !is_town_stair_tile(tile) && !(80..=87).contains(&tile) {
            return Ok(Vec::new());
        }
        self.available_town_climb_choices(
            game_dir,
            scene,
            floor,
            &[ClimbIntent::Up, ClimbIntent::Down],
        )
    }

    pub fn available_town_climb_choices(
        &self,
        game_dir: &Path,
        scene: Scene,
        floor: i8,
        candidates: &[ClimbIntent],
    ) -> io::Result<Vec<ClimbIntent>> {
        let mut choices = Vec::new();
        for intent in candidates {
            let delta = town_climb_delta(*intent);
            match load_floor(game_dir, scene, floor.saturating_add(delta)) {
                Ok(_) => choices.push(*intent),
                Err(err) if err.kind() == io::ErrorKind::InvalidInput => {}
                Err(err) => return Err(err),
            }
        }
        Ok(choices)
    }

    pub fn climb(&mut self, game_dir: &Path, intent: ClimbIntent) -> io::Result<MoveOutcome> {
        let Area::Town { scene, floor } = self.area else {
            return self.climb_dungeon(game_dir, intent);
        };
        let tile = self.grid[self.player.y * 32 + self.player.x];
        let delta = if let Some(entry) =
            self.town_stair_at(game_dir, scene, floor, self.player.x, self.player.y, tile)?
        {
            if !entry.kind.allows(intent) {
                self.message = "Not climbable!".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            town_climb_delta(intent)
        } else if let Some(delta) = stair_delta(tile, intent) {
            delta
        } else if is_town_stair_tile(tile) {
            town_climb_delta(intent)
        } else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let next_floor = floor.saturating_add(delta);
        self.change_town_floor(game_dir, scene, next_floor)
    }

    pub fn change_town_floor(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        next_floor: i8,
    ) -> io::Result<MoveOutcome> {
        let next_grid = match load_town_runtime_floor(game_dir, scene, next_floor, self.clock.hour)
        {
            Ok(grid) => grid,
            Err(err) if err.kind() == io::ErrorKind::InvalidInput => {
                self.message = "No connected floor in this slice.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            Err(err) => return Err(err),
        };
        self.grid = next_grid;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town {
            scene,
            floor: next_floor,
        };
        self.clear_town_floor_reload_door_state();
        self.restore_revealed_town_secret_doors_for_floor(game_dir, scene, next_floor)?;
        self.relink_npc_objects();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!("Changed to {} floor {}.", scene.key(), next_floor);
        Ok(MoveOutcome::Transition(AreaTransition::ChangedFloor {
            scene,
            floor: next_floor,
        }))
    }

    pub fn climb_dungeon(
        &mut self,
        game_dir: &Path,
        intent: ClimbIntent,
    ) -> io::Result<MoveOutcome> {
        let Area::Dungeon { scene, level } = self.area else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let tile = self.dungeon_cell(level, self.player.x, self.player.y);
        // `dungeon-mode.md` §13.1: the pit family `0x6?` is an ordinary
        // K-Klimb *descent* case handled by `dungeon_ladder_delta`, not a
        // shortcut into the exit contract. The earlier claim that exact
        // `0x60` invoked the surface-reset helper directly is withdrawn,
        // and is contradicted by the shipped `DUNGEON.DAT`: Deceit level
        // zero carries `0x60` at (1, 3) and Destard level zero at (7, 3)
        // and (1, 7), where klimbing descends to level one.
        let Some(delta) = dungeon_ladder_delta(tile, intent) else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let next_level = level as i8 + delta;
        if next_level > (DUNGEON_SIDE - 1) as i8 {
            if let Some(entry) = self.dungeon_deeper_transition_at(
                game_dir,
                scene,
                level,
                self.player.x,
                self.player.y,
            )? {
                self.advance_turn();
                self.apply_dungeon_deeper_transition(game_dir, entry)?;
                return Ok(MoveOutcome::Transition(
                    AreaTransition::ExitedDungeonToWorldPlane {
                        scene,
                        plane: entry.to_plane,
                    },
                ));
            }
        }
        if next_level < 0 {
            // `dungeon-mode.md` §13 up-ladder arm: K-Klimb moves Z to Z-1,
            // "or leaves the dungeon when the current level is already
            // zero". Hitting a level edge goes through the one shared exit
            // contract of §13.2 rather than refusing the climb.
            return self.resolve_dungeon_surface_reset(
                game_dir,
                scene,
                level,
                format!("Klimbed out of {} ({})", scene.key(), scene.name()),
            );
        }
        if next_level > 7 {
            self.message = "Blocked!".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        // `dungeon-mode.md` §13.1: a climb **never inspects the cell it
        // lands on** - the ladder or pit underfoot is proof enough that
        // the destination is reachable, so a climb cannot be blocked by
        // what sits on the level above or below. The destination test
        // belongs to the level-change spells
        // ([`dungeon_level_change_spell_destination_allowed`]); the
        // earlier claim that the climb route ran it too is withdrawn.
        let next_level = next_level as u8;
        self.area = Area::Dungeon {
            scene,
            level: next_level,
        };
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message = format!(
            "Changed to {} ({}) level {}.",
            scene.key(),
            scene.name(),
            dungeon_display_level(next_level)
        );
        Ok(MoveOutcome::Transition(
            AreaTransition::ChangedDungeonLevel {
                scene,
                level: next_level,
            },
        ))
    }

    pub fn resolve_dungeon_surface_reset(
        &mut self,
        game_dir: &Path,
        scene: DungeonScene,
        level: u8,
        event: String,
    ) -> io::Result<MoveOutcome> {
        if self.restore_return_world() {
            self.advance_turn();
            self.message = format!(
                "Exited {} ({}) to overworld debug return point.",
                scene.key(),
                scene.name()
            );
        } else if self.restore_world_for_target(game_dir, PlayTarget::Dungeon(scene))? {
            self.advance_turn();
            self.message = format!(
                "Exited {} ({}) to world-location table return point.",
                scene.key(),
                scene.name()
            );
        } else {
            return Ok(self.block_missing_dungeon_return(scene, level, event));
        }
        self.mark_visibility_dirty();
        Ok(MoveOutcome::Transition(AreaTransition::ExitedDungeon(
            scene,
        )))
    }

    pub fn dungeon_deeper_transition_at(
        &self,
        game_dir: &Path,
        scene: DungeonScene,
        level: u8,
        x: usize,
        y: usize,
    ) -> io::Result<Option<DungeonDeeperTransitionEntry>> {
        Ok(
            load_dungeon_deeper_transition_entries(game_dir)?.and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| {
                        entry.scene == scene && entry.level == level && entry.x == x && entry.y == y
                    })
                    .copied()
            }),
        )
    }

    pub fn apply_dungeon_deeper_transition(
        &mut self,
        game_dir: &Path,
        entry: DungeonDeeperTransitionEntry,
    ) -> io::Result<()> {
        self.area = Area::World {
            plane: entry.to_plane,
        };
        self.player.x = entry.to_x;
        self.player.y = entry.to_y;
        self.force_foot_transport();
        self.grid = load_world_map(game_dir, entry.to_plane)?;
        self.rebuild_world_live_chunks_from_grid(entry.to_plane)?;
        self.natural_moongate_live_cells.clear();
        self.npcs.clear();
        self.replace_world_active_objects(game_dir, entry.to_plane, entry.to_x, entry.to_y)?;
        self.clear_town_visit_state();
        self.return_world = None;
        self.pending_moongate = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        self.message = format!(
            "Descended from {} ({}) through a scripted deeper transition to {} at ({}, {}). {}.",
            entry.scene.key(),
            entry.scene.name(),
            entry.to_plane.key(),
            entry.to_x,
            entry.to_y,
            self.wind_status_message()
        );
        Ok(())
    }

    pub fn enter_current_location(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        let Area::World { plane } = self.area else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        if let Some(entry) = self.moongate_at(plane, self.player.x, self.player.y) {
            return self.enter_moongate(game_dir, plane, entry);
        }

        let (entries, has_sidecar) =
            effective_world_location_entries_with_sidecar_status(game_dir)?;
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        if let Some(entry) = entries.iter().find(|entry| {
            entry.plane == plane
                && entry.x == self.player.x
                && entry.y == self.player.y
                && match entry.expected_tile {
                    Some(expected) => expected == tile,
                    None => true,
                }
        }) {
            return self.enter_world_target(
                game_dir,
                plane,
                entry.target,
                entry.town_entry_y,
                false,
            );
        }

        if has_sidecar {
            self.message = format!(
                "No entry in {WORLD_LOCATION_TABLE_FILE} for {} at ({}, {}).",
                plane.key(),
                self.player.x,
                self.player.y
            );
            return Ok(MoveOutcome::Blocked);
        }

        let Some(target) = self.debug_enter else {
            self.message = format!(
                "No entry in {WORLD_LOCATION_TABLE_FILE} for {} at ({}, {}).",
                plane.key(),
                self.player.x,
                self.player.y
            );
            return Ok(MoveOutcome::Blocked);
        };
        self.enter_world_target(game_dir, plane, target, None, true)
    }

    pub fn enter_moongate(
        &mut self,
        game_dir: &Path,
        from_plane: WorldPlane,
        entry: MoongateEntry,
    ) -> io::Result<MoveOutcome> {
        self.advance_turn();
        self.apply_moongate(game_dir, from_plane, entry)
    }

    pub fn apply_moongate(
        &mut self,
        game_dir: &Path,
        from_plane: WorldPlane,
        entry: MoongateEntry,
    ) -> io::Result<MoveOutcome> {
        self.pending_moongate = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        if entry.is_single_ended() {
            self.message = "Entered moongate, but it has no destination.".to_string();
            return Ok(MoveOutcome::Observed);
        }
        let to_plane = entry.destination_plane;
        if to_plane != from_plane {
            self.cache_current_world_overlay();
            self.area = Area::World { plane: to_plane };
            self.force_foot_transport();
            self.grid = load_world_map(game_dir, to_plane)?;
            self.natural_moongate_live_cells.clear();
            self.npcs.clear();
            self.replace_world_active_objects(
                game_dir,
                to_plane,
                entry.destination_x,
                entry.destination_y,
            )?;
            self.clear_open_town_door_state();
            self.return_world = None;
            self.pending_moongate = None;
            self.pending_town_arrest = None;
            self.active_blackthorn = None;
        }
        self.player.x = entry.destination_x;
        self.player.y = entry.destination_y;
        self.rebuild_world_live_chunks_from_grid(to_plane)?;
        self.sync_player_object();
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        self.message = format!(
            "Entered moongate to {} at ({}, {}). {}.",
            to_plane.key(),
            entry.destination_x,
            entry.destination_y,
            self.wind_status_message()
        );
        Ok(MoveOutcome::Transition(
            AreaTransition::MoongateTeleported {
                from: from_plane,
                to: to_plane,
            },
        ))
    }

    pub fn resolve_moongate_prompt(
        &mut self,
        key: char,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(entry) = self.pending_moongate else {
            return Ok(None);
        };
        match key {
            'y' | 'Y' => {
                let Area::World { plane } = self.area else {
                    self.pending_moongate = None;
                    self.active_blackthorn = None;
                    self.message = "Moongate prompt cancelled outside the overworld.".to_string();
                    return Ok(Some(MoveOutcome::Blocked));
                };
                self.apply_moongate(game_dir, plane, entry).map(Some)
            }
            'n' | 'N' => {
                self.pending_moongate = None;
                self.active_blackthorn = None;
                self.message = "Moongate ignored.".to_string();
                Ok(Some(MoveOutcome::PromptDeclined))
            }
            _ => {
                self.message = "Enter moongate? (Y/N).".to_string();
                Ok(Some(MoveOutcome::Blocked))
            }
        }
    }

    pub fn refresh_cached_moon_glyphs(&mut self) {
        self.cached_moon_glyph_bytes = cached_moon_glyph_bytes_for_hour(self.clock.hour);
    }

    pub fn set_cached_moon_glyph_bytes(&mut self, trammel: u8, felucca: u8) {
        self.cached_moon_glyph_bytes = [trammel, felucca];
    }

    pub fn set_cached_moon_glyph_slots(
        &mut self,
        trammel_slot: Option<usize>,
        felucca_slot: Option<usize>,
    ) {
        let encode = |slot: Option<usize>, sentinel: u8| {
            slot.filter(|slot| *slot < MOONSTONE_SLOT_COUNT)
                .map(|slot| b'0' + slot as u8)
                .unwrap_or(sentinel)
        };
        self.cached_moon_glyph_bytes = [
            encode(trammel_slot, TRAMMEL_OFF_HORIZON_SENTINEL),
            encode(felucca_slot, FELUCCA_OFF_HORIZON_SENTINEL),
        ];
    }

    pub fn resolve_natural_moongate_entry(
        &mut self,
        game_dir: &Path,
    ) -> io::Result<Option<MoveOutcome>> {
        let Some(idx) = self.current_live_natural_moongate_cell_index() else {
            return Ok(None);
        };
        if self.grid.get(idx).copied() != Some(NATURAL_MOONGATE_TERRAIN_TILE) {
            return Ok(None);
        }

        // `overworld.md §9.2` (spec HEAD c00bf63) stage B: the transit
        // drives the *shared* gate-presence counter from 15 down to 1 and
        // "the countdown ends with the counter at zero". Because the
        // counter is shared, a gate that was mid-rise elsewhere in view is
        // driven to zero by this unrelated transit and rises again from
        // zero on subsequent turns. §9.1 calls that out explicitly as "the
        // original's behaviour, not a defect to design around", so it is
        // reproduced here rather than worked around.
        self.natural_moongate_counter = 0;
        self.grid[idx] = NATURAL_MOONGATE_RESTORED_TERRAIN_TILE;
        self.refresh_world_live_chunks_for_current_area()?;
        self.natural_moongate_live_cells
            .retain(|tracked_idx| *tracked_idx != idx);
        self.mark_visibility_dirty();

        if natural_moongate_dispatches_meditate(self.clock.hour, self.clock.minute) {
            if let Some(outcome) = self.read_codex_urn_at_current_position(game_dir)? {
                return Ok(Some(outcome));
            }
            if let Some(outcome) = self.start_shrine_prompt_at_current_position(game_dir)? {
                return Ok(Some(outcome));
            }
            self.message = "Natural moongate opened the shrine meditation path.".to_string();
            return Ok(Some(MoveOutcome::Observed));
        }

        let Some(slot_index) = self.cached_natural_moongate_slot_index() else {
            self.message = "Natural moongate moon-glyph cache is unavailable.".to_string();
            return Ok(Some(MoveOutcome::Blocked));
        };

        let phase = slot_index + 1;
        let slot = self.moonstone_slots[slot_index];
        match gate_travel_destination(slot) {
            GateTravelDestination::Ready {
                target,
                floor,
                start,
            } => {
                self.apply_gate_travel(game_dir, phase, target, floor, start)?;
                Ok(Some(MoveOutcome::Transition(
                    AreaTransition::GateTraveled { target },
                )))
            }
            GateTravelDestination::Empty => {
                self.message = format!("Natural moongate phase {phase} is not set.");
                Ok(Some(MoveOutcome::Blocked))
            }
            GateTravelDestination::Invalid(reason) => {
                self.message = format!("Natural moongate phase {phase} is invalid: {reason}.");
                Ok(Some(MoveOutcome::Blocked))
            }
        }
    }

    pub fn cached_natural_moongate_slot_index(&self) -> Option<usize> {
        let moon = natural_moongate_cached_glyph_slot(self.clock.hour) as usize;
        moonstone_slot_from_glyph_byte(self.cached_moon_glyph_bytes[moon])
            .filter(|slot| *slot < MOONSTONE_SLOT_COUNT)
    }

    pub fn current_live_natural_moongate_cell_index(&self) -> Option<usize> {
        let idx = match self.area {
            Area::World { .. } => world_cell_index(self.player.x, self.player.y),
            Area::Town { .. } if self.player.x < 32 && self.player.y < 32 => {
                self.player.y * 32 + self.player.x
            }
            _ => return None,
        };
        (idx < self.grid.len()).then_some(idx)
    }

    pub fn enter_world_target(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        target: PlayTarget,
        town_entry_y: Option<usize>,
        debug: bool,
    ) -> io::Result<MoveOutcome> {
        if !debug
            && matches!(target, PlayTarget::Dungeon(scene) if scene.record == 7)
            && !self.all_shadowlords_vanquished()
        {
            self.message = "Doom is sealed until all Shadowlords are vanquished.".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        self.restore_tracked_natural_moongates();
        self.cache_current_world_overlay();
        let return_world = WorldReturn {
            plane,
            x: self.player.x,
            y: self.player.y,
            transport: self.player.transport,
            timing_status: self.timing_status,
            sail_cadence: self.sail_cadence,
            sail_stall_pending: self.sail_stall_pending,
            grid: self.grid.clone(),
            active_objects: self.active_objects.clone(),
            pending_vehicle: None,
        };
        let mut options = PlayOptions {
            target,
            floor: 0,
            start: None,
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
            party_roster: self.synced_party_roster(),
            equipment_stock: self.equipment_stock,
            spell_charges: self.spell_charges,
            scroll_stock: self.scroll_stock,
            potion_stock: self.potion_stock,
            reagents: self.reagents,
            rare_reagent_harvest_days: self.rare_reagent_harvest_days,
            fixed_hidden_treasure_found: self.fixed_hidden_treasure_found,
            fixed_hidden_treasure_daily_day: self.fixed_hidden_treasure_daily_day,
            fixed_hidden_treasure_single_use_cookie: self.fixed_hidden_treasure_single_use_cookie,
            dungeon_room_clear_bitmap: self.dungeon_room_clear_bitmap,
            saved_dungeon_working_buffer: None,
            moonstone_slots: self.moonstone_slots,
            shadowlord_hideouts: self.shadowlord_hideouts,
            quest_progress_word: self.quest_progress_word,
            shrine_ordained_mask: self.shrine_ordained_mask,
            shrine_codex_mask: self.shrine_codex_mask,
            moral_standing: self.moral_standing,
            toll_progress: self.toll_progress,
            // `overworld.md §9.1` (spec HEAD c00bf63): the
            // gate-presence counter survives scene changes.
            natural_moongate_counter: self.natural_moongate_counter,
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
            facing: None,
            pending_vehicle: None,
            inn_registry: self.inn_registry.clone(),
            blackthorn_story: self.blackthorn_story,
            initial_britannia_overlay: None,
            debug_enter: self.debug_enter,
            saved_active_objects: None,
            save_template_source: self.save_template_source,
        };
        let mut next = match target {
            PlayTarget::Town(scene) => {
                options.start = town_entry_y
                    .map(|entry_y| Ok(Some((LOCATION_DEFAULT_ENTRY_X, entry_y))))
                    .unwrap_or_else(|| {
                        load_location_entry_y(game_dir, scene)
                            .map(|entry_y| entry_y.map(|y| (LOCATION_DEFAULT_ENTRY_X, y)))
                    })?;
                Self::load_town_scene(game_dir, scene, options)?
            }
            PlayTarget::Dungeon(scene) => {
                options.floor = match plane {
                    WorldPlane::Britannia => 0,
                    WorldPlane::Underworld if scene.record == 7 => 0,
                    WorldPlane::Underworld => 7,
                };
                options.start = match plane {
                    WorldPlane::Britannia => Some((1, 1)),
                    WorldPlane::Underworld if scene.record == 7 => Some((1, 1)),
                    WorldPlane::Underworld => Some((7, 7)),
                };
                let mut dungeon = Self::load_dungeon_scene(game_dir, scene, options)?;
                if matches!(plane, WorldPlane::Underworld) && scene.record != 7 {
                    dungeon.player.facing = Direction::West;
                }
                dungeon
            }
            PlayTarget::World(_) => {
                self.message = "World enter target must be a town or dungeon scene.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
        };
        // Interior play starts on foot; the return snapshot owns outside transport.
        next.force_foot_transport();
        next.sync_player_object();
        next.turn = self.turn;
        next.return_world = Some(return_world);
        next.world_overlays = self.world_overlays.clone();
        next.message = match target {
            PlayTarget::Town(scene) if debug => {
                format!("Debug-entered {} from {}.", scene.key(), plane.key())
            }
            PlayTarget::Town(scene) => format!("Entered {} from {}.", scene.key(), plane.key()),
            PlayTarget::Dungeon(scene) if debug => {
                format!(
                    "Debug-entered {} ({}) from {}.",
                    scene.key(),
                    scene.name(),
                    plane.key()
                )
            }
            PlayTarget::Dungeon(scene) => {
                format!(
                    "Entered {} ({}) from {}.",
                    scene.key(),
                    scene.name(),
                    plane.key()
                )
            }
            PlayTarget::World(_) => unreachable!(),
        };
        next.append_stonegate_entry_presentation_message();
        *self = next;
        Ok(match target {
            PlayTarget::Town(scene) => {
                MoveOutcome::Transition(AreaTransition::EnteredLocation(scene))
            }
            PlayTarget::Dungeon(scene) => {
                MoveOutcome::Transition(AreaTransition::EnteredDungeon(scene))
            }
            PlayTarget::World(_) => unreachable!(),
        })
    }

    pub fn restore_return_world(&mut self) -> bool {
        let Some(return_world) = self.return_world.take() else {
            return false;
        };
        let plane = return_world.plane;
        self.area = Area::World { plane };
        self.player.x = return_world.x;
        self.player.y = return_world.y;
        self.player.transport = return_world.transport;
        self.timing_status = return_world.timing_status;
        self.sail_cadence = return_world.sail_cadence;
        self.sail_stall_pending = return_world.sail_stall_pending;
        self.grid = return_world.grid;
        if let Err(err) = self.rebuild_world_live_chunks_from_grid(plane) {
            self.message = err.to_string();
        }
        self.natural_moongate_live_cells.clear();
        self.npcs.clear();
        self.active_objects = return_world.active_objects;
        if let Some(pending) = return_world.pending_vehicle {
            if let Err(err) =
                place_pending_vehicle_acquisition(&mut self.active_objects, plane, pending)
            {
                self.message = err.to_string();
            }
        }
        self.sync_player_object();
        self.cache_current_world_overlay();
        self.clear_town_visit_state();
        self.pending_moongate = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        true
    }

    pub fn restore_world_for_target(
        &mut self,
        game_dir: &Path,
        target: PlayTarget,
    ) -> io::Result<bool> {
        let entries = effective_world_location_entries(game_dir)?;
        let matches: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| entry.target == target)
            .collect();
        let Some(entry) = matches.first().copied() else {
            return Ok(false);
        };
        if matches.len() > 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} has multiple return rows for {}",
                    target.key()
                ),
            ));
        }

        self.area = Area::World { plane: entry.plane };
        self.player.x = entry.x;
        self.player.y = entry.y;
        self.force_foot_transport();
        self.grid = load_world_map(game_dir, entry.plane)?;
        self.rebuild_world_live_chunks_from_grid(entry.plane)?;
        self.natural_moongate_live_cells.clear();
        self.npcs.clear();
        self.replace_world_active_objects(game_dir, entry.plane, entry.x, entry.y)?;
        self.clear_town_visit_state();
        self.pending_moongate = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        Ok(true)
    }

    pub fn moongate_at(&self, plane: WorldPlane, x: usize, y: usize) -> Option<MoongateEntry> {
        if plane != WorldPlane::Britannia {
            return None;
        }
        if !self.moongates_visible_by_light() {
            return None;
        }
        self.moongates.iter().copied().find(|entry| {
            entry.x == x
                && entry.y == y
                && entry.is_active_at(self.clock.hour)
                && self.moongate_origin_tile_matches(*entry)
        })
    }

    pub fn moongates_visible_by_light(&self) -> bool {
        self.ambient_light >= FULL_DAYLIGHT
    }

    pub fn moongate_origin_tile_matches(&self, entry: MoongateEntry) -> bool {
        entry.matches_origin_tile(self.grid[world_cell_index(entry.x, entry.y)])
    }

    pub fn visible_moongate_at(&self, plane: WorldPlane, x: usize, y: usize) -> bool {
        if plane != WorldPlane::Britannia || !self.moongates_visible_by_light() {
            return false;
        }

        self.moongates.iter().any(|entry| {
            entry.is_active_at(self.clock.hour)
                && self.moongate_origin_tile_matches(*entry)
                && ((entry.x == x && entry.y == y)
                    || (!entry.is_single_ended()
                        && entry.destination_plane == WorldPlane::Britannia
                        && entry.destination_x == x
                        && entry.destination_y == y))
        })
    }

    pub fn visible_moongate_cells(&self) -> Vec<(usize, usize)> {
        if !matches!(
            self.area,
            Area::World {
                plane: WorldPlane::Britannia
            }
        ) || !self.moongates_visible_by_light()
        {
            return Vec::new();
        }

        let mut cells = Vec::new();
        for entry in self.moongates.iter().filter(|entry| {
            entry.is_active_at(self.clock.hour) && self.moongate_origin_tile_matches(**entry)
        }) {
            let origin = (entry.x, entry.y);
            if !cells.contains(&origin) {
                cells.push(origin);
            }
            if !entry.is_single_ended() && entry.destination_plane == WorldPlane::Britannia {
                let destination = (entry.destination_x, entry.destination_y);
                if !cells.contains(&destination) {
                    cells.push(destination);
                }
            }
        }
        cells
    }

    pub fn world_plane_transition_at(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<Option<WorldPlaneTransitionEntry>> {
        if plane == WorldPlane::Britannia
            && x == usize::from(SURFACE_CHASM_X)
            && y == usize::from(SURFACE_CHASM_Y)
        {
            return Ok(Some(WorldPlaneTransitionEntry {
                from_plane: WorldPlane::Britannia,
                x,
                y,
                to_plane: WorldPlane::Underworld,
                to_x: x,
                to_y: y,
                expected_tile: None,
            }));
        }
        let tile = self.grid[world_cell_index(x, y)];
        Ok(
            load_world_plane_transition_entries(game_dir)?.and_then(|entries| {
                entries
                    .iter()
                    .find(|entry| {
                        entry.from_plane == plane
                            && entry.x == x
                            && entry.y == y
                            && match entry.expected_tile {
                                Some(expected) => expected == tile,
                                None => true,
                            }
                    })
                    .copied()
            }),
        )
    }

    pub fn apply_world_underfoot_plane_transition(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
    ) -> io::Result<Option<AreaTransition>> {
        let Some(entry) =
            self.world_plane_transition_at(game_dir, plane, self.player.x, self.player.y)?
        else {
            return Ok(None);
        };
        let to_plane = entry.to_plane;
        self.apply_world_plane_transition(game_dir, entry)?;
        Ok(Some(AreaTransition::ChangedWorldPlane {
            from: plane,
            to: to_plane,
        }))
    }

    pub fn world_damage_tile_at(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
        tile: u8,
    ) -> io::Result<Option<WorldDamageTileEntry>> {
        let Some(entries) = load_world_damage_tile_entries(game_dir)? else {
            return Ok(intrinsic_world_damage_tile_entry(plane, x, y, tile));
        };
        Ok(world_damage_tile_entry_at(&entries, plane, x, y, tile))
    }

    pub fn append_world_damage_tile_message(
        &mut self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
    ) -> io::Result<()> {
        if let Some(report) = self.apply_world_underfoot_damage(game_dir, plane)? {
            self.message.push_str(&format!(" {report}."));
        }
        Ok(())
    }

    pub fn append_world_status_tile_message(&mut self, plane: WorldPlane) {
        if let Some(report) = self.apply_world_underfoot_status_tick(plane) {
            self.message.push_str(&format!(" {report}."));
        }
    }

    pub fn apply_world_underfoot_status_tick(&mut self, _plane: WorldPlane) -> Option<String> {
        if !self.player.transport.is_foot() {
            return None;
        }
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        if tile != BRIT_SWAMP_TILE {
            return None;
        }

        let mut poisoned = Vec::new();
        let mut checked = 0;
        for member in &mut self.party {
            if !member.living() {
                continue;
            }
            checked += 1;
            if member.status == b'P' {
                continue;
            }
            member.status = b'P';
            poisoned.push(member.slot);
        }

        if poisoned.is_empty() {
            Some(format!(
                "swamp poison skipped for {checked} living member(s)"
            ))
        } else {
            Some(format!(
                "swamp poison: set party slot{} {} to poisoned",
                if poisoned.len() == 1 { "" } else { "s" },
                poisoned
                    .iter()
                    .map(|slot| slot.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        }
    }

    pub fn apply_world_underfoot_damage(
        &mut self,
        game_dir: Option<&Path>,
        plane: WorldPlane,
    ) -> io::Result<Option<String>> {
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        let entry = if let Some(game_dir) = game_dir {
            self.world_damage_tile_at(game_dir, plane, self.player.x, self.player.y, tile)?
        } else {
            intrinsic_world_damage_tile_entry(plane, self.player.x, self.player.y, tile)
        };
        let Some(entry) = entry else { return Ok(None) };
        if entry.effect.damages_transport(self.player.transport) {
            Ok(Some(self.apply_world_damage_tile(entry)))
        } else {
            Ok(None)
        }
    }

    pub fn apply_fixed_narrative_gate_branch(&mut self, plane: WorldPlane) -> bool {
        if plane != WorldPlane::Britannia
            || self.player.x != NARRATIVE_GATE_X as usize
            || self.player.y != NARRATIVE_GATE_Y as usize
        {
            return false;
        }

        let narrative = if self.shrine_ordained_mask != 0 {
            "A fixed narrative gate opens. Ordained shrine progress blocks entry."
        } else {
            self.player.y = (self.player.y + 1) % WORLD_SIDE;
            self.sync_player_object();
            let _ = self.rebuild_world_live_chunks_from_grid(plane);
            self.mark_visibility_dirty();
            "A fixed narrative gate opens. The party enters and steps south."
        };
        if self.message.is_empty() {
            self.message = narrative.to_string();
        } else {
            self.message.push(' ');
            self.message.push_str(narrative);
        }
        true
    }
}

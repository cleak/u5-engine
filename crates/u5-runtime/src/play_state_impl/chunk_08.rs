use std::io;
use std::path::Path;

use crate::rest_camp::{
    COMPLETED_LONG_CAMP_COOLDOWN_HOURS, COMPLETED_LONG_CAMP_HP_GAIN_MAX,
    COMPLETED_LONG_CAMP_HP_GAIN_MIN, COMPLETED_LONG_CAMP_MIN_HOURS, camp_cooldown_blocks_recovery,
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
            RestPhase::Hours => REST_HOURS_PROMPT.to_string(),
            RestPhase::WatchYesNo => REST_WATCH_PROMPT.to_string(),
            RestPhase::WatchSlot => REST_WATCH_MEMBER_PROMPT.to_string(),
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
                        self.emit_message_line(REST_WATCH_YES_LITERAL);
                        session.phase = RestPhase::WatchSlot;
                        self.active_rest = Some(session);
                        self.message = self.render_active_rest();
                        return Ok(None);
                    }
                    'N' | '\u{1b}' | ' ' | '0' | '\r' | '\n' => {
                        self.emit_message_line(REST_WATCH_NO_LITERAL);
                        let hours = session.hours.unwrap_or(1);
                        return self.finish_active_rest(hours, None, game_dir);
                    }
                    _ => {}
                },
                RestPhase::WatchSlot => {
                    if matches!(ch, '\u{1b}' | ' ' | '0' | '\r' | '\n') {
                        self.emit_message_line(REST_NO_WATCH_LITERAL);
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
        game_dir: &Path,
    ) -> io::Result<MoveOutcome> {
        let Some(hours) = request.hours else {
            return Ok(self.start_rest_prompt());
        };
        if !(1..=9).contains(&hours) {
            self.message = "Rest hours must be in 1..9.".to_string();
            return Ok(MoveOutcome::Blocked);
        }
        let camp_messages = load_camp_result_messages(game_dir)?;
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
        let mut world_damage_ticks = 0;
        let mut last_world_damage = None;
        let mut interrupted = false;
        let mut ambush_monster = None;
        let wilderness_camp = matches!(self.area, Area::World { .. });
        let ticks_per_hour = if wilderness_camp {
            WILDERNESS_CAMP_TICKS_PER_HOUR
        } else {
            REST_WATCH_TICKS_PER_HOUR
        };
        let minutes_per_tick = if wilderness_camp {
            WILDERNESS_CAMP_MINUTES_PER_TICK
        } else {
            REST_WATCH_MINUTES_PER_TICK
        };
        'resting: for _ in 0..hours {
            for _ in 0..ticks_per_hour {
                let hour_before_tick = self.clock.hour;
                // `rest-and-camp.md §5`: "That loop advances the clock [...]
                // and never enters the shared party status/provision pass, so
                // while a camp is elapsing no poison damage is taken, no
                // provisions are spent, and no starvation damage is applied,
                // regardless of how many hours the camp covers. Only the
                // town-bed loop runs that pass." `time.md §10`'s caller census
                // agrees: the pass has exactly four call sites, and the only
                // rest one is "the town-bed rest loop's ten-minute step".
                // Dungeon rest-with-watch is not a town bed - it is a camp -
                // so it takes the camp tick too. Only the step size differs
                // between the two camp surfaces.
                self.advance_wilderness_camp_tick(minutes_per_tick);
                let _ = self.apply_rest_with_watch_recovery_tick();
                if let Area::World { plane } = self.area {
                    if let Some(report) =
                        self.apply_world_underfoot_damage(Some(game_dir), plane)?
                    {
                        world_damage_ticks += 1;
                        last_world_damage = Some(report);
                    }
                }
                let interruption_probe_due = match self.area {
                    Area::World { .. } => self.clock.hour != hour_before_tick,
                    Area::Dungeon { .. } => true,
                    Area::Town { .. } => false,
                };
                let ambush_row =
                    if matches!(self.area, Area::World { .. }) && interruption_probe_due {
                        self.wilderness_camp_hour_change_ambush_row(host_clock_prng_seed_now())
                    } else if matches!(self.area, Area::Dungeon { .. })
                        && self.dangerous_rest_interrupted()
                    {
                        Some(self.sleep_ambush_monster_row())
                    } else {
                        None
                    };
                if let Some(row) = ambush_row {
                    interrupted = true;
                    ambush_monster = sleep_ambush_monster(row);
                    break 'resting;
                }
            }
        }
        if interrupted {
            self.restore_sleep_ambush_party_statuses(&rest_entry_statuses)
        } else {
            self.wake_initial_rest_sleepers(&asleep_at_start)
        };
        // `rest-and-camp.md §5` (answer to cleak/u5-spec#95): the
        // cooldown gate is read only after the camp's time has elapsed,
        // so rollovers during this attempt can make recovery eligible.
        let long_camp = hours >= COMPLETED_LONG_CAMP_MIN_HOURS;
        let cooldown_blocked =
            !interrupted && long_camp && camp_cooldown_blocks_recovery(self.camp_cooldown);
        if !interrupted {
            let _ = self.apply_completed_long_camp_recovery(
                hours,
                request.watcher,
                &rest_entry_statuses,
            );
        }
        self.message = if cooldown_blocked {
            camp_messages.no_effect
        } else {
            camp_messages.success
        };
        if let Some(monster) = ambush_monster {
            let z = match self.area {
                Area::World { plane } => plane.save_floor(),
                Area::Dungeon { level, .. } => level as i8,
                Area::Town { floor, .. } => floor,
            };
            let note = self.enter_sleep_ambush_combat(monster, z, game_dir)?;
            self.message.push_str(&format!(" Ambushed! {note}."));
        }
        if let Some(report) = last_world_damage {
            self.message.push_str(&format!(
                " Underfoot world damage triggered {world_damage_ticks} tick(s); last {report}."
            ));
        }
        self.append_pending_hourly_status_message();
        if !interrupted
            && !cooldown_blocked
            && long_camp
            && matches!(self.area, Area::World { .. })
            && lord_british_camp_event_triggered(self.lord_british_camp_event_roll())
        {
            // `rest-and-camp.md §5`: this is the same single draw an
            // earlier spec revision misidentified as a marker-stamp
            // branch. Nothing is stamped. The successful draw copies
            // the current month into the persisted, write-only cookie.
            self.camp_month_cookie = self.clock.month;
            // `rest-and-camp.md §5` (cleak/u5-spec#96): the live caller
            // condition is the dungeon-rest selector. The Area gate
            // above implements its suppression before the PRNG draw.
            // The second condition is reserved and no shipped public
            // caller sets it; town-bed rest uses a separate handler.
            let event_message = self.resolve_lord_british_camp_event(Some(game_dir))?;
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
                // `rest-and-camp.md §7`: dead members skip healing,
                // narration, level recomputation and rewards, but still
                // reach the class-keyed MP refresh branch.
                self.refresh_party_member_class_mana(index);
                continue;
            }
            // Every living member is silently full-healed and cured
            // before the level check, even when the stored level already
            // agrees with experience and no reward follows.
            self.party[index].hp = self.party[index].max_hp;
            self.party[index].status = b'G';
            let experience = self.party_experience[index];
            let level = level_for_experience(u32::from(experience));
            if self.party[index].level != level {
                self.party[index].level = level;
                let hp = lord_british_camp_event_hp_for_level(level);
                self.party[index].hp = hp;
                self.party[index].max_hp = hp;
                let reward = self.apply_lord_british_camp_stat_reward(index);
                level_changes += 1;
                notes.push(format!(
                    "P{} reached level {level} from {experience} XP and received {reward}.",
                    index + 1
                ));
            }
            // The class refresh belongs after the level/reward branch,
            // so an Intelligence reward affects this write immediately.
            self.refresh_party_member_class_mana(index);
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

    // `encounters.md §6`: "the rest handler rolls the shared integer PRNG
    // across sixty-four outcomes. The zero outcome interrupts". Anchored
    // to the constant so the interval's *size* (64) and its inclusive
    // high bound (63) cannot drift apart.
    pub fn dangerous_rest_interrupt_roll(&mut self) -> u8 {
        self.random_mod_u8(SLEEP_AMBUSH_INTERRUPT_DENOMINATOR)
    }

    pub fn sleep_ambush_monster_row(&mut self) -> u8 {
        self.random_range_u8(0, 7)
    }

    pub fn sleep_ambush_monster_row_after_host_seed(&mut self, host_seed: u16) -> u8 {
        self.prng_state = host_seed;
        self.sleep_ambush_monster_row()
    }

    /// `prng.md §3` / `rest-and-camp.md §5`: a wilderness camp probes
    /// for interruption only when the game hour changes. The existing stream
    /// supplies the `0..63` probe; only its zero result installs a fresh host
    /// seed, immediately followed by the `0..7` encounter-row draw.
    pub fn wilderness_camp_hour_change_ambush_row(&mut self, host_seed: u16) -> Option<u8> {
        if !sleep_ambush_rest_interrupted(self.dangerous_rest_interrupt_roll()) {
            return None;
        }
        Some(self.sleep_ambush_monster_row_after_host_seed(host_seed))
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

    /// `rest-and-camp.md §5` completed-long-camp recovery walk. The
    /// published guards implemented here are: accepted duration
    /// greater than five hours, the member was not Poisoned in the
    /// rest-local entry snapshot, the member is not Dead, and the
    /// member is not the selected watcher. Each survivor gains a
    /// uniform random 1..63 HP capped at maximum HP, then the class
    /// rows that publish an MP write have their current MP assigned
    /// (not added) to the published target.
    ///
    /// The block-level guards — the cooldown counter and the duration —
    /// short-circuit before the walk; the remaining three are per-member
    /// and only skip that member. §5 states the ordering as "the handler
    /// can walk active party records and apply recovery if all of these
    /// guards pass", then "for each member that passes those guards".
    ///
    /// After the walk the cooldown is armed at
    /// [`COMPLETED_LONG_CAMP_COOLDOWN_HOURS`] whether or not any member
    /// recovered and whether or not the single apparition draw runs.
    /// A refused camp does not re-arm it. The earlier spec's separate
    /// marker-stamp interpretation was withdrawn by cleak/u5-spec#95:
    /// there is no marker tile and no world-map write.
    pub fn apply_completed_long_camp_recovery(
        &mut self,
        accepted_hours: u8,
        watcher: Option<usize>,
        entry_statuses: &[u8],
    ) -> (u16, u16) {
        // `rest-and-camp.md §5`: "A second camp begun inside fourteen
        // game hours of the previous one therefore prints the no-effect
        // line and recovers nothing."
        if camp_cooldown_blocks_recovery(self.camp_cooldown) {
            return (0, 0);
        }
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
        // `rest-and-camp.md §5`: "After the recovery walk, the handler
        // arms the cooldown counter at 14 ... The cooldown is armed
        // whether or not the marker is stamped, and whether or not any
        // member actually recovered."
        self.camp_cooldown = COMPLETED_LONG_CAMP_COOLDOWN_HOURS;
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
        // The wind check belongs to the world step itself
        // (`animation.md §13.2`), so it runs inside `advance_visual_tick`
        // and this shell only narrates the result.
        if let Some(wind) = self.advance_visual_tick() {
            // `weather.md §2.1`: the wind banner "is not drawn in
            // combat-class scenes, in the underworld scene, or on
            // below-surface map levels", so those are the cases that take
            // the non-cardinal presentation branch. A surface town floor is
            // not one of them - the paired Britain captures show the
            // original naming the direction inside a town.
            let on_surface = match self.area {
                Area::World { plane } => plane == WorldPlane::Britannia,
                Area::Town { floor, .. } => floor >= 0,
                Area::Dungeon { .. } => false,
            } && !self.combat_active;
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

    /// `weather.md §2` "Autonomous Wind Drift" / `animation.md §13.2`.
    ///
    /// One world step runs "one wind-change check - a single **1-in-64**
    /// check, not a drift step. When it fires a new prevailing direction is
    /// drawn, with 'calm' accepted only behind a further roughly 1-in-4
    /// confirmation, so a fired event always installs some direction."
    ///
    /// Three things this used to get wrong:
    ///
    /// * **Scene gate.** It only ran on the world plane. `weather.md §2`:
    ///   "The store happens before any scene test, so the state is always
    ///   updated; only the banner repaint is conditional." The original
    ///   changes wind while the party is inside a town, which is exactly
    ///   what the paired Britain captures show.
    /// * **Roll source.** The rolls come off the shared gameplay PRNG.
    ///   `prng.md §4` names "**The per-pass wind check**, which draws
    ///   **once** and returns in the common case - sixty-three invocations
    ///   in sixty-four. On the uncommon result it enters a retry loop taking
    ///   **one further draw at a time**, so its count per invocation is one,
    ///   two, three, and so on upward, with each extra iteration continuing
    ///   at roughly `0.15`" as the second of the **three** per-pass
    ///   consumers behind "Rendering and idling perturb the gameplay
    ///   stream". A private hash of the clock and position is not that
    ///   stream. (The earlier "draws in pairs until it settles" reading is
    ///   withdrawn by that section's own retraction paragraph, and this
    ///   loop draws one at a time accordingly: the outer roll, then one
    ///   candidate draw per iteration plus one confirmation draw only when
    ///   the candidate is Calm.)
    /// * **Cadence.** It only ran from the TUI `.` key, so no shell running
    ///   the ordinary idle redraw ever changed the wind. It now hangs off
    ///   the world step in [`Self::advance_visual_tick`].
    pub fn idle_wind_drift(&mut self) -> Option<WindState> {
        if !WindState::autonomous_drift_outer_accepted(
            self.wind_drift_roll(WIND_DRIFT_OUTER_ROLL_MAX),
        ) {
            return None;
        }
        for _ in 0..=u8::MAX {
            let candidate = self.idle_wind_candidate();
            if candidate != WindState::Calm
                || self.wind_drift_roll(u8::MAX) >= WIND_DRIFT_CALM_ACCEPT_MIN
            {
                // `audio.md §7.3` is titled "Accepted wind change / Rel Hur"
                // and every row of its table describes a transition the
                // player accepted behind the direction prompt. An unprompted
                // weather drift is not a published trigger, so it commits the
                // identical state change without the shared variant — the
                // alternative is a one-to-two-second blocking sequence firing
                // on an idle tick at sea.
                self.apply_wind_state_without_sound(candidate);
                return Some(candidate);
            }
        }
        None
    }

    /// `weather.md §2`: "On a zero roll, it chooses a candidate wind in
    /// `0..4`."
    pub fn idle_wind_candidate(&mut self) -> WindState {
        match self.wind_drift_roll(WIND_DRIFT_CANDIDATE_MODULUS - 1) {
            0 => WindState::Calm,
            1 => WindState::North,
            2 => WindState::South,
            3 => WindState::East,
            _ => WindState::West,
        }
    }

    /// One gameplay-PRNG draw over `0..=high` for the wind selector
    /// (`prng.md §4`).
    fn wind_drift_roll(&mut self, high: u8) -> u8 {
        self.random_range_u8(0, high)
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
                if self.player.transport.is_horse() {
                    self.message = "-On foot!".to_string();
                    return Ok(MoveOutcome::Blocked);
                }
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
                    [] => Ok(self.start_klimb_direction_prompt()),
                    [intent] => self.climb(game_dir, *intent),
                    _ => Ok(self.start_klimb_direction_prompt()),
                }
            }
            // `dungeon-mode.md §8`: "the dispatcher masks the underfoot cell to
            // its high nibble before any comparison, so the whole `0x6?` family
            // - not just the exact byte `0x60`, and including the marked/fired
            // variants - enables the down arm. That arm calls the same
            // level-step helper a down ladder uses, so the party simply
            // descends one level". §13.2 adds: "An earlier revision said exact
            // byte `0x60` bypassed the level-step helper and invoked the exit
            // contract directly; that is withdrawn."
            Area::Dungeon { level, .. } => {
                let tile = self.dungeon_cell(level, self.player.x, self.player.y);
                match dungeon_klimb_apply(tile) {
                    DungeonKlimbApply::UpLadder => self.climb(game_dir, ClimbIntent::Up),
                    DungeonKlimbApply::DownLadder | DungeonKlimbApply::PitDescent => {
                        self.climb(game_dir, ClimbIntent::Down)
                    }
                    DungeonKlimbApply::TwoWayPrompt => Ok(self.start_klimb_direction_prompt()),
                    DungeonKlimbApply::NoLevelChange => {
                        // `dungeon-mode.md` Section 8.1 Klimb prompts:
                        // `Klimb-what?` "when neither [direction] is [available]
                        // and the cell has no climbable feature at all". There
                        // is no "you are at the top/bottom level" refusal.
                        self.message = DUNGEON_KLIMB_WHAT_REFUSAL.to_string();
                        Ok(MoveOutcome::Blocked)
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
        Ok(town_klimb_underfoot_intent(tile).into_iter().collect())
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
        } else if town_klimb_underfoot_intent(tile) == Some(intent) {
            town_climb_delta(intent)
        } else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let next_floor = floor.saturating_add(delta);
        self.change_town_floor(game_dir, scene, next_floor)
    }

    pub fn klimb_over_town_target(&mut self, direction: Direction) -> MoveOutcome {
        let Area::Town { .. } = self.area else {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        };
        let Some((x, y)) = self.adjacent_position(direction) else {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        };
        if !town_klimb_over_target(self.grid[y * TOWN_GRID_SIDE + x]) {
            self.message = "What?".to_string();
            return MoveOutcome::Blocked;
        }

        self.player.facing = direction;
        self.player.x = x;
        self.player.y = y;
        self.sync_player_object();
        self.mark_visibility_dirty();
        self.advance_turn();
        self.message.clear();
        MoveOutcome::Moved
    }

    pub fn change_town_floor(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        next_floor: i8,
    ) -> io::Result<MoveOutcome> {
        let Area::Town { floor, .. } = self.area else {
            self.message = "Not climbable!".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let announcement = if next_floor > floor { "Up!" } else { "Down!" };
        self.message = announcement.to_string();
        match self.reload_town_floor(game_dir, scene, next_floor) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::InvalidInput => {
                self.message = "No connected floor in this slice.".to_string();
                return Ok(MoveOutcome::Blocked);
            }
            Err(err) => return Err(err),
        }
        self.advance_turn();
        Ok(MoveOutcome::Transition(AreaTransition::ChangedFloor {
            scene,
            floor: next_floor,
        }))
    }

    pub fn reload_town_floor(
        &mut self,
        game_dir: &Path,
        scene: Scene,
        next_floor: i8,
    ) -> io::Result<()> {
        let (next_grid, beacon_sources) = load_town_runtime_floor_with_beacon_sources(
            game_dir,
            scene,
            next_floor,
            self.clock.hour,
        )?;
        self.grid = next_grid;
        // `visibility.md §12.6`: a new location floor is fresh map setup —
        // clear both beacon positions and re-record up to two bright-light
        // hits. Harvested from the RAW floor, because the runtime
        // normalisation pass scrubs the marker byte the beacon looks for.
        self.light_beacon.sources = beacon_sources;
        self.natural_moongate_live_cells.clear();
        self.area = Area::Town {
            scene,
            floor: next_floor,
        };
        let _ = self.apply_resident_shadowlord_blight_with_seed(host_clock_prng_seed_now());
        self.clear_town_floor_reload_door_state();
        self.restore_revealed_town_secret_doors_for_floor(game_dir, scene, next_floor)?;
        self.relink_npc_objects();
        self.mark_visibility_dirty();
        Ok(())
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
        // `dungeon-mode.md` Section 8.1: "Applying a climb prints `Up!` or
        // `Down!` **first**, before any test."
        self.emit_message_line(match intent {
            ClimbIntent::Up => DUNGEON_KLIMB_UP,
            ClimbIntent::Down => DUNGEON_KLIMB_DOWN,
        });
        let Some(delta) = dungeon_ladder_delta(tile, intent) else {
            // "an impassable destination then adds `Failed!` with the short
            // **rising** sweep ... the same recipe the spell-failure tail
            // uses" - not the falls-style descent.
            //
            // **Engine assignment, not a published pairing** - marked rather
            // than hidden, the same way `dungeon_search_outcome_line` marks
            // its stalactite/caved-in flavour pairing. Section 8.1 attaches
            // `Failed!` to "an impassable **destination**", while §13.1 and
            // `doors-and-z-transitions.md §9` state a climb "never tests the
            // cell it lands on". No published sentence joins the line to the
            // predicate used here, which is the *underfoot* cell not offering
            // the requested direction - the residual case the prompt-form
            // dispatcher (which answers `Klimb-what?` when the cell offers
            // neither direction) does not already cover. It is kept because
            // it is the nearest published refusal for a climb that cannot
            // apply; it is not evidence about the original's condition.
            self.message = DUNGEON_KLIMB_FAILED.to_string();
            self.emit_sound_effect(SoundEffect::CastFailure);
            return Ok(MoveOutcome::Blocked);
        };
        let next_level = level as i8 + delta;
        if !(0..DUNGEON_SIDE as i8).contains(&next_level) {
            // `dungeon-mode.md` §13 up-ladder arm: K-Klimb moves Z to Z-1,
            // "or leaves the dungeon when the current level is already
            // zero". The down-ladder arm is symmetric at level seven.
            // Hitting either edge goes through the one shared surface-reset
            // contract of §13.2; there is no per-dungeon transition table.
            return self.resolve_dungeon_surface_reset(
                game_dir,
                scene,
                level,
                format!("Klimbed out of {} ({})", scene.key(), scene.name()),
            );
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
        self.setup_dungeon_active_monster_fresh();
        self.mark_visibility_dirty();
        self.advance_turn();
        // Section 8.1: the direction word above is the whole of an accepted
        // climb's narration; there is no level or coordinate line.
        self.message = match intent {
            ClimbIntent::Up => DUNGEON_KLIMB_UP,
            ClimbIntent::Down => DUNGEON_KLIMB_DOWN,
        }
        .to_string();
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
        let entries = effective_world_location_entries(game_dir)?;
        let matches: Vec<_> = entries
            .iter()
            .copied()
            .filter(|entry| entry.target == PlayTarget::Dungeon(scene))
            .collect();
        let Some(entry) = matches.first().copied() else {
            return Ok(self.block_missing_dungeon_return(scene, level, event));
        };
        if matches.len() > 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "{WORLD_LOCATION_TABLE_FILE} has multiple return rows for {}",
                    PlayTarget::Dungeon(scene).key()
                ),
            ));
        }

        // `dungeon-mode.md` §13.2: both exit arms use the dungeon's one
        // published exterior coordinate. The level at which the edge was
        // reached, not the entry row's plane and not the cached entry plane,
        // selects the destination world.
        let plane = if level == 0 {
            WorldPlane::Britannia
        } else {
            WorldPlane::Underworld
        };
        // The committed command belongs to dungeon mode, whose ordinary
        // minute increment is one. Advance before replacing the active area
        // so the world-mode two-minute cadence is not applied retroactively.
        self.advance_turn();
        self.restore_world_at(game_dir, plane, entry.x, entry.y)?;
        self.message = match plane {
            WorldPlane::Britannia => DUNGEON_EXIT_TO_BRITANNIA_NARRATION,
            WorldPlane::Underworld => DUNGEON_EXIT_TO_UNDERWORLD_NARRATION,
        }
        .to_string();
        Ok(MoveOutcome::Transition(
            AreaTransition::ExitedDungeonToWorldPlane { scene, plane },
        ))
    }

    pub fn enter_current_location(&mut self, game_dir: &Path) -> io::Result<MoveOutcome> {
        let Area::World { plane } = self.area else {
            self.message = "Not here!".to_string();
            return Ok(MoveOutcome::Blocked);
        };

        let (entries, _has_sidecar) =
            effective_world_location_entries_with_sidecar_status(game_dir)?;
        let tile = self.grid[world_cell_index(self.player.x, self.player.y)];
        let Some(live_class) = WorldEntryNarrationClass::from_live_tile(tile) else {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        };
        let helper = live_class.helper();
        let coordinate_matches = |entry: &&WorldLocationEntry| {
            (entry.plane == plane || entry.accepts_both_world_planes)
                && entry.x == self.player.x
                && entry.y == self.player.y
        };

        // An extension row at this exact coordinate is deliberately unusable
        // until it publishes a narration class. Never infer one from its key.
        if entries
            .iter()
            .filter(coordinate_matches)
            .any(|entry| entry.narration_class.is_none())
        {
            self.message = "What?".to_string();
            return Ok(MoveOutcome::Blocked);
        }

        let entry = entries.iter().filter(coordinate_matches).find(|entry| {
            let row_helper = match entry.target {
                PlayTarget::Town(_) => WorldEntryHelper::Town,
                PlayTarget::Dungeon(_) => WorldEntryHelper::Dungeon,
                PlayTarget::World(_) => return false,
            };
            row_helper == helper
        });
        let Some(entry) = entry else {
            self.advance_turn();
            self.message = format!(
                "{}\n{}",
                live_class.text(),
                match helper {
                    WorldEntryHelper::Town => "What town?",
                    WorldEntryHelper::Dungeon => "What dungeon?",
                }
            );
            return Ok(MoveOutcome::Blocked);
        };

        if helper == WorldEntryHelper::Dungeon
            && !matches!(self.player.transport, TransportState::Foot)
        {
            self.advance_turn();
            self.message = format!("{}\nOn foot!", live_class.text());
            return Ok(MoveOutcome::Blocked);
        }

        if matches!(entry.target, PlayTarget::Dungeon(scene) if scene.record == 7)
            && !self.all_shadowlords_vanquished()
        {
            self.advance_turn();
            self.message = format!("{}\nAttacked at entrance!", live_class.text());
            // The public contract identifies this as an entrance ambush object,
            // not an immediate scene transition. Reuse the native outdoor
            // encounter placer so it obeys the live object table and terrain.
            let _ = self.spawn_native_world_encounter(plane);
            return Ok(MoveOutcome::Blocked);
        }

        self.emit_world_entry_narration(live_class, entry.proper_name);
        self.enter_world_target(game_dir, plane, entry.target, false)
    }

    fn emit_world_entry_narration(
        &mut self,
        live_class: WorldEntryNarrationClass,
        proper_name: Option<&'static str>,
    ) {
        let class_text = live_class.text();
        self.message = class_text.to_string();
        if !self.commit_command_echo() {
            self.push_message_entry(class_text, false);
        }
        if let Some(name) = proper_name {
            self.push_explicit_blank_message_entry();
            self.push_centered_message_entry(name);
            self.message = format!("{class_text}\n\n{name}\n");
        } else {
            self.message = format!("{class_text}\n");
        }
        self.message_flushed = self.message.clone();
    }

    /// `moons.md §3` / `time.md §5`: the hour-change hook that refreshes
    /// the sky strip runs only when the active scene is in the
    /// surface/town-family range *and* the party is not below the
    /// surface. `moons.md §3`: "below the surface, nothing is drawn,
    /// nothing is erased, and **nothing is cached**, exactly as for
    /// combat and dungeon scenes."
    ///
    /// The below-surface test is the party's saved Z with its high bit
    /// set — the Underworld plane outdoors, or a below-entry floor
    /// inside a town-family location. Ordinary dungeon levels count up
    /// from zero and never set that bit, so they are excluded by the
    /// scene-range half of the gate instead.
    pub fn sky_strip_hour_refresh_runs(&self) -> bool {
        if self.current_floor().is_none_or(|floor| floor < 0) {
            return false;
        }
        match self.area {
            Area::World { plane } => sky_strip_renders(0, matches!(plane, WorldPlane::Underworld)),
            Area::Town { scene, .. } => sky_strip_renders(scene.byte, false),
            Area::Dungeon { .. } => false,
        }
    }

    /// `moons.md §5`: "Each refresh caches the two glyph bytes for the
    /// current day *before* it tests whether either marker is on the
    /// visible horizon", so a reached refresh always writes the cache
    /// even when neither marker is drawn. A refresh that is never
    /// reached — below the surface, in a dungeon, in combat — writes
    /// nothing at all.
    pub fn refresh_cached_moon_glyphs(&mut self) {
        if !self.sky_strip_hour_refresh_runs() {
            return;
        }
        self.cached_moon_glyph_bytes =
            cached_moon_glyph_bytes_for_day(self.clock.day).unwrap_or(MOON_GLYPH_CACHE_NO_GATE);
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

        // `overworld.md §9.2` (spec HEAD c00bf63): on `0xDC` the hook runs
        // a **blocking** transition to completion "before the party is
        // relocated and before any key is read", and the player cannot
        // skip it. Stage A swallows the party into the gate cell over 256
        // dispatch steps; stage B drives the *shared* gate-presence
        // counter from 15 down to 1 and "the countdown ends with the
        // counter at zero".
        //
        // Because the counter is shared, a gate that was mid-rise
        // elsewhere in view is driven to zero by this unrelated transit
        // and rises again from zero on subsequent turns. §9.1 calls that
        // out explicitly as "the original's behaviour, not a defect to
        // design around", so it is reproduced here rather than worked
        // around.
        let sound_serial_before_transit = self.sound_effect_serial;
        let playback = self.play_natural_moongate_transit()?;
        let outcome = self.resolve_natural_moongate_entry_after_transit(game_dir, idx);
        // The warp below rebuilds the state from `PlayOptions`, which does
        // not carry presentation bookkeeping; the transit it dropped had
        // already finished, so restore its record afterwards rather than
        // letting the destination scene claim none played.
        self.last_natural_moongate_transit = Some(playback);
        // The rebuild used to discard the recorded sound history too, which
        // silenced the `audio.md §8.3` transit envelope on exactly the path
        // that has a destination handoff. `enter_world_target` now carries the
        // outgoing history across instead of clearing it, so the envelope
        // survives the warp on its own. This assertion is what keeps that
        // true: it is the only place in the engine where a cue is emitted
        // before a scene rebuild and read after one.
        debug_assert_eq!(
            self.sound_effects_after(sound_serial_before_transit)
                .first(),
            Some(&SoundEffect::MoongateTransit),
            "the transit envelope must survive the destination rebuild",
        );
        outcome
    }

    /// `overworld.md §9.2`: everything the live-gate entry hook does once
    /// the blocking transition has run to completion - step 4's cell
    /// rewrite, then the hook's two outcomes (the midnight kneel overlay,
    /// or a cached-glyph destination through the shared saved-slot warp).
    fn resolve_natural_moongate_entry_after_transit(
        &mut self,
        game_dir: &Path,
        idx: usize,
    ) -> io::Result<Option<MoveOutcome>> {
        // §9.2 step 4: "The gate's live cell is rewritten to terrain `5`,
        // the viewport is marked dirty, and the cell is repainted."
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

    /// `overworld.md §9.2` (spec HEAD c00bf63): play the blocking transit
    /// transition at the gate cell.
    ///
    /// The whole sequence runs here, in one call. `§9.2` makes it blocking
    /// and unskippable - "the abort poll that some other presentation
    /// effects offer is disabled in overworld scenes" - so this leaves no
    /// resumable position behind, only a record of what it spent.
    ///
    /// Stage A is "paced by a world tick every eight steps rather than by
    /// a fixed wait, so it also advances ambient animation while it runs":
    /// each of those ticks runs the `animation.md §6` static-tile pass,
    /// which is the ambient animation this engine has. The gate-presence
    /// counter is deliberately *not* advanced by that pass - `§9.1` states
    /// it "is not advanced by the animation tick" - so the two remain
    /// independent while both move during the stage.
    ///
    /// Stage B drives the shared counter from 15 down to 1 and ends it at
    /// zero. The pixel-level presentation of both stages lives in
    /// [`crate::moongate_transit::run_moongate_transit_presentation`],
    /// which composes each stage-B frame through the `§9.1` scratch slot.
    pub fn play_natural_moongate_transit(&mut self) -> io::Result<MoongateTransitPlayback> {
        // `audio.md §8.3`: "During an accepted transit, run
        // `(2, 2000, 30000, 1, 5900)`. No destination handoff means no
        // transit envelope." The envelope belongs to the transit itself, so
        // it is recorded on entry, before the blocking stage-A/stage-B
        // presentation this call runs to completion. A movement that hands
        // no live gate cell to the entry hook returns from
        // `resolve_natural_moongate_entry` without reaching here and stays
        // silent.
        self.emit_sound_effect(SoundEffect::MoongateTransit);
        let animation = &mut self.animation;
        let playback =
            run_moongate_transit(&mut self.natural_moongate_counter, &mut |step, _phase| {
                for _ in 0..step.world_ticks() {
                    animation.tick_static_tiles();
                }
                Ok(())
            })?;
        self.last_natural_moongate_transit = Some(playback);
        Ok(playback)
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
        debug: bool,
    ) -> io::Result<MoveOutcome> {
        let prior_sound_serial = self.sound_effect_serial;
        self.restore_tracked_natural_moongates();
        self.cache_current_world_overlay();
        let entering_transport = self.player.transport;
        let entering_player_object = self.active_objects.first().copied();
        let return_world = matches!(target, PlayTarget::Dungeon(_)).then(|| WorldReturn {
            plane,
            x: self.player.x,
            y: self.player.y,
            transport: self.player.transport,
            sail_cadence: self.sail_cadence,
            sail_stall_pending: self.sail_stall_pending,
            grid: self.grid.clone(),
            active_objects: self.active_objects.clone(),
            pending_vehicle: self.pending_vehicle_save.acquisition(),
        });
        // Entry persists the complete current outdoor object table before
        // either town-family setup or dungeon-record loading.
        let bytes = encode_active_object_table(&self.active_objects)?;
        let file_name = match plane {
            WorldPlane::Britannia => BRIT_OOL_FILENAME,
            WorldPlane::Underworld => UNDER_OOL_FILENAME,
        };
        write_disk_file(&game_dir.join(file_name), bytes)?;
        let entry_transcript = self.message_transcript.clone();
        let entry_transcript_revision = self.message_transcript_revision;
        let entry_message = self.message.clone();
        let entry_message_flushed = self.message_flushed.clone();
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
            transport: entering_transport,
            facing: None,
            door_tracker: None,
            pending_vehicle: None,
            pending_vehicle_save: self.pending_vehicle_save,
            inn_registry: self.inn_registry.clone(),
            initial_britannia_overlay: None,
            debug_enter: self.debug_enter,
            saved_active_objects: None,
            town_npc_mutations: self.town_npc_mutations.clone(),
            save_template_source: self.save_template_source,
        };
        let mut next = match target {
            PlayTarget::Town(scene) => {
                // `town-mode.md §5` / public #94: overworld entry always
                // writes (15, 30, floor 0). Per-scene entry-row tables were
                // a withdrawn conflation with the Shadowlord helper.
                options.start = Some((LOCATION_DEFAULT_ENTRY_X, LOCATION_DEFAULT_ENTRY_Y));
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
                dungeon.prng_state = self.prng_state;
                dungeon.setup_dungeon_active_monster_fresh();
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
        match target {
            PlayTarget::Town(_) => {
                // Fresh town setup owns slots 1..31 but preserves the outdoor
                // player record. Install its seven auxiliary bytes, then only
                // synchronize the interior coordinate and transport frame.
                if let Some(player_object) = entering_player_object {
                    next.active_objects[0] = player_object;
                }
                let floor = next.current_floor().unwrap_or(0);
                let player_object = &mut next.active_objects[0];
                let marker = entering_transport.save_marker();
                player_object.type_byte = marker;
                player_object.tile = marker;
                player_object.x = next.player.x;
                player_object.y = next.player.y;
                player_object.z = floor;
            }
            PlayTarget::Dungeon(_) => {
                // Dungeon entry retains its independently specified on-foot
                // interior and return-snapshot behavior.
                next.force_foot_transport();
                next.sync_player_object();
            }
            PlayTarget::World(_) => unreachable!(),
        }
        next.turn = self.turn;
        next.return_world = return_world;
        next.world_overlays = self.world_overlays.clone();
        next.message = if debug {
            match target {
                PlayTarget::Town(scene) if debug => {
                    format!("Debug-entered {} from {}.", scene.key(), plane.key())
                }
                PlayTarget::Dungeon(scene) if debug => {
                    format!(
                        "Debug-entered {} ({}) from {}.",
                        scene.key(),
                        scene.name(),
                        plane.key()
                    )
                }
                PlayTarget::Town(_) | PlayTarget::Dungeon(_) => unreachable!(),
                PlayTarget::World(_) => unreachable!(),
            }
        } else {
            entry_message
        };
        if !debug {
            next.message_transcript = entry_transcript;
            next.message_transcript_revision = entry_transcript_revision;
            next.message_flushed = entry_message_flushed;
            next.pending_command_echo = None;
        }
        if debug {
            next.append_stonegate_entry_presentation_message();
        }
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
            match place_pending_vehicle_acquisition(&mut self.active_objects, plane, pending) {
                Ok(_) => {
                    self.pending_vehicle_save =
                        PendingVehicleSaveState::from_acquisition(pending).clear_class();
                }
                Err(err) => {
                    self.pending_vehicle_save = PendingVehicleSaveState::from_acquisition(pending);
                    self.message = err.to_string();
                }
            }
        }
        self.sync_player_object();
        self.cache_current_world_overlay();
        self.clear_town_visit_state();
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

        self.restore_world_at(game_dir, entry.plane, entry.x, entry.y)?;
        Ok(true)
    }

    /// Installs one outdoor world plane at an exact coordinate.
    ///
    /// Normal dungeon exits use this with the coordinate from the dungeon's
    /// location row and a plane selected from the exited level. Keeping those
    /// two inputs separate is important: seven dungeon rows are published on
    /// Britannia even though their bottom exit lands on the Underworld map.
    pub fn restore_world_at(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<()> {
        self.area = Area::World { plane };
        self.player.x = x;
        self.player.y = y;
        self.force_foot_transport();
        self.grid = load_world_map(game_dir, plane)?;
        apply_world_quest_tile_substitutions(
            &mut self.grid,
            &self.word_of_power_seal_flags,
            &self.shrine_ruin_flags,
        );
        self.rebuild_world_live_chunks_from_grid(plane)?;
        self.natural_moongate_live_cells.clear();
        self.npcs.clear();
        self.replace_world_active_objects(game_dir, plane, x, y)?;
        self.clear_town_visit_state();
        self.return_world = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        Ok(())
    }

    /// Install the world side of a town boundary exit from the destination
    /// plane's complete canonical `.OOL` mirror.
    ///
    /// `town-mode.md §15.1`: no pre-entry snapshot participates.  The
    /// current marker survives, the fixed exterior coordinate is installed,
    /// all 32 records are replaced, slot zero keeps the mirror's auxiliary
    /// bytes while receiving the marker frame and coordinate, and only then
    /// is a queued shipwright acquisition materialized.
    pub fn restore_world_from_town_mirror(
        &mut self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<()> {
        self.area = Area::World { plane };
        self.player.x = x;
        self.player.y = y;
        self.grid = load_world_map(game_dir, plane)?;
        apply_world_quest_tile_substitutions(
            &mut self.grid,
            &self.word_of_power_seal_flags,
            &self.shrine_ruin_flags,
        );
        self.rebuild_world_live_chunks_from_grid(plane)?;
        self.natural_moongate_live_cells.clear();
        self.npcs.clear();
        self.active_objects = load_world_active_object_mirror_table(game_dir, plane)?;

        let player_object = &mut self.active_objects[0];
        let marker = self.player.transport.save_marker();
        player_object.type_byte = marker;
        player_object.tile = marker;
        player_object.x = x;
        player_object.y = y;
        player_object.z = plane.save_floor();

        if let Some(pending) = self.pending_vehicle_save.acquisition() {
            place_pending_vehicle_acquisition(&mut self.active_objects, plane, pending)?;
            self.pending_vehicle_save = self.pending_vehicle_save.clear_class();
        }
        self.cache_current_world_overlay();
        self.clear_town_visit_state();
        self.return_world = None;
        self.pending_town_arrest = None;
        self.active_blackthorn = None;
        self.mode_zero_cleanup();
        self.mark_visibility_dirty();
        Ok(())
    }

    pub fn world_plane_transition_at(
        &self,
        game_dir: &Path,
        plane: WorldPlane,
        x: usize,
        y: usize,
    ) -> io::Result<Option<WorldPlaneTransitionEntry>> {
        // `RETRACTIONS.md` R320: the surface chasm is **not** a coordinate
        // trigger. `(54, 138)` is the cell the falls handler tests after its
        // two forced southward steps, and the whole chain - banner, steps,
        // sweep, damage pass and only then the plane write - belongs to
        // `PlayState::apply_world_falls_chain`. Keying an underfoot plane
        // transition on the coordinate fired at none of the real brink cells
        // and skipped the forced steps everywhere, so that arm is gone.
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
            self.append_result_sentence(&format!("{report}."));
        }
        Ok(())
    }

    pub fn append_world_status_tile_message(&mut self, plane: WorldPlane) {
        if let Some(report) = self.apply_world_underfoot_status_tick(plane) {
            self.append_result_sentence(&format!("{report}."));
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
            "\nPass, Seeker!\n"
        } else {
            self.player.y = (self.player.y + 1) % WORLD_SIDE;
            self.sync_player_object();
            let _ = self.rebuild_world_live_chunks_from_grid(plane);
            self.mark_visibility_dirty();
            "\nThou art not upon a Sacred Quest!\nPassage denied!\n"
        };
        // The accepted step no longer leaves a line in the slot, so the
        // narrative block must not open with the separator newline it used
        // to need.
        if self.message.is_empty() {
            self.message = narrative.trim_start_matches('\n').to_string();
        } else {
            self.message.push_str(narrative);
        }
        true
    }
}
